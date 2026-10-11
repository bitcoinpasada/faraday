//! What a scanned or loaded payload is (`docs/UX.md` §4 "Scan routing").
//!
//! [`classify`] looks at the bytes only; it parses nothing it does not
//! need to. Order matters where formats overlap: a 48-digit string is a
//! SeedQR before it is "text", `ur:` wins over everything, and the raw
//! 16/32-byte CompactSeedQR check comes last because random bytes match
//! nothing else.

use alloc::string::String;
use alloc::vec::Vec;
use core::str::FromStr;

use bitcoin::address::{Address, NetworkUnchecked};
use osk_bip::bip39::Language;
use osk_bip::descriptor::verify_checksum;
use osk_bip::slip132;
use osk_bip::xkey;

/// The kinds a payload can be routed as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadKind {
    /// 48 or 96 decimal digits: word indices.
    SeedQr {
        /// 12 or 24.
        words: u8,
    },
    /// An `osk-backup`: a key's words encrypted under a passphrase,
    /// which the magic `OSKB` names outright.
    EncryptedBackup,
    /// 16 or 32 raw bytes: entropy.
    CompactSeedQr {
        /// 12 or 24.
        words: u8,
    },
    /// A PSBT, binary (`psbt\xff`) or base64.
    Psbt,
    /// A partly signed threshold transaction: the `OSKC` carry file of
    /// `docs/PLANNING.md` §16.103, which is a PSBT with the secret nonce
    /// of every signer still to sign bound to it. Routed into Sign as a
    /// PSBT is.
    ThresholdCarry,
    /// A uniform resource, `ur:<type>/…`.
    Ur {
        /// The type after `ur:`, lower-cased: `crypto-psbt`, `bytes`, …
        ur_type: String,
        /// `(sequence, count)` of a multi-part UR.
        part: Option<(usize, usize)>,
    },
    /// A Bitcoin address of any network.
    Address,
    /// A silent payment address (BIP-352), on its own or as the `sp`
    /// parameter of a BIP-321 URI. It is not an address a wallet's
    /// chain of addresses can be searched for, so it is routed to
    /// Check an address and shown there as what it is
    /// (`docs/PLANNING.md` §16.113).
    SilentAddress,
    /// An output descriptor: a known function with parentheses.
    Descriptor {
        /// `Some(valid)` when a `#checksum` is present.
        checksum: Option<bool>,
    },
    /// A base58check extended public key (`xpub`, `tpub`, SLIP-132 forms).
    Xpub,
    /// Coldcard's JSON account export: `xfp` and one object per
    /// single-sig standard, each with its derivation and account key.
    ColdcardXpubs,
    /// A coordinator's multisig config: the `Name:`/`Policy:`/
    /// `Derivation:`/`Format:` text file Sparrow, Nunchuk, Specter and
    /// Coldcard exchange a wallet in.
    MultisigConfig,
    /// BIP 129's key record: round 1 of a Bitcoin Secure Multisig
    /// Setup, one signer's key with its signature.
    BsmsSigner,
    /// BIP 129's descriptor record: round 2, the wallet itself with the
    /// derivation paths and its first address.
    BsmsDescriptor,
    /// A codex32 string (BIP 93): `ms1`, a threshold, an identifier, a
    /// share index, a payload and a checksum, which is a master seed or
    /// one share of a split of one (`docs/PLANNING.md` §16.109 rule 3).
    Codex32,
    /// 12–24 words all from one BIP-39 list, whitespace-separated.
    Bip39Words {
        /// Word count.
        words: u8,
        /// The list every word was found in (first match in
        /// `Language::ALL` order).
        language: Language,
    },
    /// Nothing above.
    Unknown,
}

/// The first four bytes of an encrypted backup
/// (`osk_backup::oskb`). Routing reads the magic and nothing else;
/// whether the rest of the header is one this build reads is
/// `osk-backup`'s answer, since it is that crate that holds the
/// cryptography.
pub const BACKUP_MAGIC: &[u8; 4] = b"OSKB";

/// The first four bytes of a threshold carry file
/// (`osk_psbt::threshold`). As with a backup, the magic is read and
/// nothing else: the app holds the format.
pub const CARRY_MAGIC: &[u8; 4] = b"OSKC";

/// Classifies `bytes`.
pub fn classify(bytes: &[u8]) -> PayloadKind {
    // The magic comes first and is not trimmed: a backup is binary, and
    // a header the app cannot read is still a backup, refused with a
    // reason rather than routed as something else.
    if bytes.starts_with(BACKUP_MAGIC) {
        return PayloadKind::EncryptedBackup;
    }
    if bytes.starts_with(CARRY_MAGIC) {
        return PayloadKind::ThresholdCarry;
    }
    let trimmed = bytes.trim_ascii();
    if matches!(trimmed.len(), 48 | 96) && trimmed.iter().all(u8::is_ascii_digit) {
        return PayloadKind::SeedQr {
            words: (trimmed.len() / 4) as u8,
        };
    }
    if trimmed.starts_with(b"psbt\xff") || trimmed.starts_with(b"cHNidP8") {
        return PayloadKind::Psbt;
    }
    if let Ok(text) = core::str::from_utf8(trimmed) {
        if let Some(ur) = ur_kind(text) {
            return ur;
        }
        // A codex32 string names itself with `ms1` and proves itself
        // with its checksum; nothing else this function reads begins
        // that way.
        if is_codex32(text) {
            return PayloadKind::Codex32;
        }
        // A silent payment address is read before an ordinary one: it
        // is bech32m under a part of its own, which no address parser
        // takes, and the BIP-321 URI that carries one is text nothing
        // else here reads.
        if osk_bip::silent::address_of(text).is_some() {
            return PayloadKind::SilentAddress;
        }
        if Address::<NetworkUnchecked>::from_str(text).is_ok() {
            return PayloadKind::Address;
        }
        if xkey::decode_xpub(text).is_ok() || slip132::decode_xpub(text).is_ok() {
            return PayloadKind::Xpub;
        }
        // Both BSMS records say what they are on their first line; the
        // second line tells the two rounds apart, and neither is read
        // as the descriptor or the wallet it carries.
        if osk_bip::bsms::looks_like_record(text) {
            return if osk_bip::bsms::is_signer_record(text) {
                PayloadKind::BsmsSigner
            } else {
                PayloadKind::BsmsDescriptor
            };
        }
        // A threshold wallet's group record is a wallet arriving, and
        // is routed as one: its own last line is the descriptor, and
        // `WalletPolicy::parse_any` reads the whole file.
        if osk_bip::threshold::ThresholdRecord::looks_like_record(text) {
            return PayloadKind::Descriptor { checksum: None };
        }
        if let Some(d) = descriptor_kind(text) {
            return d;
        }
        if osk_bip::coldcard::looks_like_export(text) {
            return PayloadKind::ColdcardXpubs;
        }
        if osk_bip::multisig_config::looks_like_config(text) {
            return PayloadKind::MultisigConfig;
        }
        if let Some(w) = words_kind(text) {
            return w;
        }
    }
    if matches!(bytes.len(), 16 | 32) {
        return PayloadKind::CompactSeedQr {
            words: (bytes.len() * 3 / 4) as u8,
        };
    }
    PayloadKind::Unknown
}

/// Whether `text` is a codex32 string: the `ms1` prefix in either case,
/// and a string the codec takes whole.
fn is_codex32(text: &str) -> bool {
    text.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("ms1"))
        && osk_bip::codex32::Codex32::parse(text).is_ok()
}

/// `ur:` prefix, case-insensitive; type and part indices from the path.
fn ur_kind(text: &str) -> Option<PayloadKind> {
    let rest = text
        .get(..3)?
        .eq_ignore_ascii_case("ur:")
        .then(|| &text[3..])?;
    let mut segments = rest.split('/');
    let ur_type = segments.next()?.to_ascii_lowercase();
    if ur_type.is_empty() {
        return None;
    }
    let second = segments.next();
    let third = segments.next();
    let part = match (second, third) {
        (Some(seq), Some(_)) => {
            let (a, b) = seq.split_once('-')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        }
        _ => None,
    };
    Some(PayloadKind::Ur { ur_type, part })
}

/// Descriptor functions BIP-380–386 and BIP-390 define.
const DESCRIPTOR_FUNCTIONS: &[&str] = &[
    "pk",
    "pkh",
    "sh",
    "wsh",
    "wpkh",
    "tr",
    "multi",
    "sortedmulti",
    "multi_a",
    "sortedmulti_a",
    "combo",
    "addr",
    "raw",
    "rawtr",
    "musig",
];

fn descriptor_kind(text: &str) -> Option<PayloadKind> {
    let open = text.find('(')?;
    if !text.contains(')') || !DESCRIPTOR_FUNCTIONS.contains(&&text[..open]) {
        return None;
    }
    let checksum = text.contains('#').then(|| verify_checksum(text));
    Some(PayloadKind::Descriptor { checksum })
}

fn words_kind(text: &str) -> Option<PayloadKind> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if !matches!(words.len(), 12 | 15 | 18 | 21 | 24) {
        return None;
    }
    let language = Language::ALL
        .into_iter()
        .find(|lang| words.iter().all(|w| word_index(*lang, w).is_some()))?;
    Some(PayloadKind::Bip39Words {
        words: words.len() as u8,
        language,
    })
}

/// Index of `word` in `lang`: the stored (published or NFKD) form, or,
/// for the Latin-script lists, an accent-folded match so that text with
/// precomposed accents (`ábaco` as one code point, the usual form in a
/// QR or a file) is found without Unicode tables. Words are matched
/// whole; the Load wizard's checksum step catches a wrong one.
pub fn word_index(lang: Language, word: &str) -> Option<u16> {
    if let Some(i) = lang.index_of(word) {
        return Some(i);
    }
    if !lang.is_latin() {
        return None;
    }
    let folded = fold_precomposed(word)?;
    lang.candidates_typed(folded.as_chars())
        .find(|&i| lang.typed(i).is_some_and(|t| t == folded))
}

/// Lower-case ASCII of a Latin word whose accents may be precomposed:
/// the Latin-1 and Latin Extended-A letters the five accented wordlists
/// use (Spanish, French, Italian, Czech, Portuguese) map to their base
/// letter; combining marks are dropped. `None` for anything else.
fn fold_precomposed(word: &str) -> Option<osk_bip::bip39::Typed> {
    const BASES: &[(&str, u8)] = &[
        ("àáâãäå", b'a'),
        ("ç", b'c'),
        ("èéêë", b'e'),
        ("ìíîï", b'i'),
        ("ñ", b'n'),
        ("òóôõö", b'o'),
        ("ùúûü", b'u'),
        ("ýÿ", b'y'),
        ("čć", b'c'),
        ("ď", b'd'),
        ("ě", b'e'),
        ("ň", b'n'),
        ("ř", b'r'),
        ("š", b's'),
        ("ť", b't'),
        ("ů", b'u'),
        ("ž", b'z'),
    ];
    let mut ascii = String::with_capacity(word.len());
    for c in word.chars() {
        let c = c.to_lowercase().next().unwrap_or(c);
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            continue;
        }
        if c.is_ascii() {
            ascii.push(c);
        } else if let Some((_, base)) = BASES.iter().find(|(set, _)| set.contains(c)) {
            ascii.push(char::from(*base));
        } else {
            return None;
        }
    }
    osk_bip::bip39::fold(&ascii)
}

#[cfg(test)]
mod tests {
    use super::*;

    const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";

    #[test]
    fn routing_table() {
        let digits48 = [b'0'; 48];
        assert_eq!(classify(&digits48), PayloadKind::SeedQr { words: 12 });
        let digits96 = [b'1'; 96];
        assert_eq!(classify(&digits96), PayloadKind::SeedQr { words: 24 });
        assert_eq!(
            classify(&[b'1'; 60]),
            PayloadKind::Unknown,
            "15 words is not a SeedQR"
        );
        assert_eq!(
            classify(&[0x5b; 16]),
            PayloadKind::CompactSeedQr { words: 12 }
        );
        assert_eq!(
            classify(&[0x0e; 32]),
            PayloadKind::CompactSeedQr { words: 24 }
        );
        assert_eq!(classify(b"psbt\xff\x01\x00"), PayloadKind::Psbt);
        assert_eq!(classify(b"cHNidP8BAHECAAAA\n"), PayloadKind::Psbt);
        assert_eq!(
            classify(b"UR:CRYPTO-PSBT/HDCX"),
            PayloadKind::Ur {
                ur_type: String::from("crypto-psbt"),
                part: None
            }
        );
        assert_eq!(
            classify(b"ur:crypto-psbt/2-7/lpao"),
            PayloadKind::Ur {
                ur_type: String::from("crypto-psbt"),
                part: Some((2, 7))
            }
        );
        assert_eq!(
            classify(b"bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"),
            PayloadKind::Address
        );
        assert_eq!(
            classify(b"BC1QCR8TE4KR609GCAWUTMRZA0J4XV80JY8Z306FYU"),
            PayloadKind::Address,
            "upper-case bech32 from an alphanumeric QR"
        );
        // A signet address derived from the test seed, so the checksum
        // is real.
        let m = osk_bip::bip39::Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
        let master = osk_bip::keys::MasterKey::from_seed(
            &m.to_seed(b"").unwrap(),
            osk_bip::keys::Network::Signet,
        );
        let signet = master
            .account_xpub(osk_bip::keys::ScriptType::NativeSegwit, 0)
            .unwrap()
            .address(false, 0)
            .unwrap();
        let signet = alloc::format!("{signet}");
        assert!(signet.starts_with("tb1q"), "{signet}");
        assert_eq!(classify(signet.as_bytes()), PayloadKind::Address);
        assert_eq!(
            classify(b"1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2"),
            PayloadKind::Address
        );
        assert_eq!(classify(XPUB.as_bytes()), PayloadKind::Xpub);
        assert_eq!(
            classify(b"zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs"),
            PayloadKind::Xpub
        );
        assert_eq!(
            classify(b"raw(deadbeef)#89f8spxm"),
            PayloadKind::Descriptor {
                checksum: Some(true)
            }
        );
        assert_eq!(
            classify(b"raw(deadbeef)#89f8spxn"),
            PayloadKind::Descriptor {
                checksum: Some(false)
            }
        );
        assert_eq!(
            classify(alloc::format!("wpkh([73c5da0a/84h/0h/0h]{XPUB}/<0;1>/*)").as_bytes()),
            PayloadKind::Descriptor { checksum: None }
        );
        // A MuSig2 wallet reaches the device either as the descriptor
        // or as the BIP-388 policy, and both are routed as a descriptor.
        assert_eq!(
            classify(
                alloc::format!(
                    "tr(musig([73c5da0a/86h/0h/0h]{XPUB},[b2b1f0cf/86h/0h/0h]{XPUB})/<0;1>/*)"
                )
                .as_bytes()
            ),
            PayloadKind::Descriptor { checksum: None }
        );
        assert_eq!(
            classify(
                alloc::format!(
                    "tr(musig(@0,@1)/**)\n[73c5da0a/86h/0h/0h]{XPUB}\n[b2b1f0cf/86h/0h/0h]{XPUB}"
                )
                .as_bytes()
            ),
            PayloadKind::Descriptor { checksum: None }
        );
        // The group record of a threshold wallet, which is a wallet
        // this device reads and reviews like any other.
        assert_eq!(
            classify(b"osk-threshold 1\nthreshold 2\ngroup 02\ntr(tpub)#aaaaaaaa"),
            PayloadKind::Descriptor { checksum: None }
        );
        assert_eq!(
            classify(b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"),
            PayloadKind::Bip39Words {
                words: 12,
                language: Language::English
            }
        );
        assert_eq!(
            classify(
                "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto"
                    .as_bytes()
            ),
            PayloadKind::Bip39Words {
                words: 12,
                language: Language::Spanish
            }
        );
        assert_eq!(
            classify(
                "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto"
                    .as_bytes()
            ),
            PayloadKind::Bip39Words {
                words: 12,
                language: Language::Spanish
            },
            "precomposed accents"
        );
        assert_eq!(word_index(Language::Spanish, "ábaco"), Some(0));
        assert_eq!(word_index(Language::Czech, "abdikace"), Some(0));
        assert_eq!(word_index(Language::French, "abaisser"), Some(0));
        assert_eq!(word_index(Language::English, "zoo"), Some(2047));
        assert_eq!(word_index(Language::English, "zoos"), None);
        assert_eq!(
            classify(b"Name: My wallet\nPolicy: 2 of 3\nFormat: P2WSH\n"),
            PayloadKind::MultisigConfig
        );
        assert_eq!(
            classify(
                alloc::format!(
                    "BSMS 1.0\n00\n[0f056943/48'/0'/0'/2']{XPUB}\nSigner 1 key\nH6DXgqkC"
                )
                .as_bytes()
            ),
            PayloadKind::BsmsSigner
        );
        assert_eq!(
            classify(
                alloc::format!(
                    "BSMS 1.0\nwsh(sortedmulti(1,[0f056943/48'/0'/0'/2']{XPUB}/**))\n/0/*,/1/*\nbc1q"
                )
                .as_bytes()
            ),
            PayloadKind::BsmsDescriptor
        );
        assert_eq!(
            classify(
                alloc::format!(
                    "{{\"xfp\": \"0F056943\", \"bip84\": {{\"deriv\": \"m/84'/0'/0'\", \"xpub\": \"{XPUB}\"}}}}"
                )
                .as_bytes()
            ),
            PayloadKind::ColdcardXpubs
        );
        assert_eq!(
            classify(b"OSKB\x01rest of a header"),
            PayloadKind::EncryptedBackup
        );
        assert_eq!(classify(b"hello world"), PayloadKind::Unknown);
        assert_eq!(classify(b"nothing(here"), PayloadKind::Unknown);
        assert_eq!(classify(b""), PayloadKind::Unknown);
        assert_eq!(classify(b"ur:"), PayloadKind::Unknown);
    }
}
