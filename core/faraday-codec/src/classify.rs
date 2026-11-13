//! What a scanned or loaded payload is.
//!
//! [`classify`] looks at the bytes only; it parses nothing it does not
//! need to. Order matters where formats overlap: a 48-digit string is a
//! SeedQR before it is "text", `ur:` wins over everything, and the raw
//! 16/32-byte CompactSeedQR check comes after the text kinds because
//! random bytes match nothing else. A Krux encrypted code (KEF) has no magic number, so it is
//! the last arm of all, asked only when nothing stricter claimed the
//! payload and raw 16 or 32 bytes were not taken as a CompactSeedQR.

use alloc::string::String;
use alloc::vec::Vec;
use core::str::FromStr;

use bitcoin::address::{Address, NetworkUnchecked};
use faraday_bip::bip39::Language;
use faraday_bip::descriptor::verify_checksum;
use faraday_bip::slip132;
use faraday_bip::xkey;

/// The kinds a payload can be routed as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadKind {
    /// 48 or 96 decimal digits: word indices.
    SeedQr {
        /// 12 or 24.
        words: u8,
    },
    /// 16 or 32 raw bytes: entropy.
    CompactSeedQr {
        /// 12 or 24.
        words: u8,
    },
    /// A PSBT, binary (`psbt\xff`) or base64.
    Psbt,
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
    /// Check an address and shown there as what it is.
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
    /// one share of a split of one.
    Codex32,
    /// 12–24 words all from one BIP-39 list, whitespace-separated.
    Bip39Words {
        /// Word count.
        words: u8,
        /// The list every word was found in (first match in
        /// `Language::ALL` order).
        language: Language,
    },
    /// Krux's encrypted code (`docs/KEF.md` §8): an envelope that
    /// parses, in one of the forms Krux reads. The weakest test here,
    /// so it is the last arm; it names the kind only, and the card gets
    /// the envelope from [`crate::kef::recognize`].
    Kef,
    /// Nothing above.
    Unknown,
}

/// The kind a text payload names, if any text arm takes it.
fn text_kind(text: &str) -> Option<PayloadKind> {
    if let Some(ur) = ur_kind(text) {
        return Some(ur);
    }
    // A codex32 string names itself with `ms1` and proves itself
    // with its checksum; nothing else this function reads begins
    // that way.
    if is_codex32(text) {
        return Some(PayloadKind::Codex32);
    }
    // A silent payment address is read before an ordinary one: it
    // is bech32m under a part of its own, which no address parser
    // takes, and the BIP-321 URI that carries one is text nothing
    // else here reads.
    if faraday_bip::silent::address_of(text).is_some() {
        return Some(PayloadKind::SilentAddress);
    }
    if Address::<NetworkUnchecked>::from_str(text).is_ok() {
        return Some(PayloadKind::Address);
    }
    if xkey::decode_xpub(text).is_ok() || slip132::decode_xpub(text).is_ok() {
        return Some(PayloadKind::Xpub);
    }
    // Both BSMS records say what they are on their first line; the
    // second line tells the two rounds apart, and neither is read
    // as the descriptor or the wallet it carries.
    if faraday_bip::bsms::looks_like_record(text) {
        return Some(if faraday_bip::bsms::is_signer_record(text) {
            PayloadKind::BsmsSigner
        } else {
            PayloadKind::BsmsDescriptor
        });
    }
    if let Some(d) = descriptor_kind(text) {
        return Some(d);
    }
    if faraday_bip::coldcard::looks_like_export(text) {
        return Some(PayloadKind::ColdcardXpubs);
    }
    if faraday_bip::multisig_config::looks_like_config(text) {
        return Some(PayloadKind::MultisigConfig);
    }
    if let Some(w) = words_kind(text) {
        return Some(w);
    }
    None
}

/// Whether text looks like entries for a vault: an `otpauth://` line, or
/// a `title:` line, on any line, after Unicode whitespace is trimmed.
pub fn looks_like_entries(text: &str) -> bool {
    text.lines().map(str::trim).any(|l| {
        ["otpauth://", "title:"]
            .iter()
            .any(|p| l.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p)))
    })
}

/// Whether a 16 or 32 byte payload is a CompactSeedQR (SeedSigner's
/// `docs/seed_qr/README.md`): entropy bytes in a byte-mode code. Both
/// SeedSigner (`decode_qr.py`, `detect_segment_type`) and Krux
/// (`mnemonic_loader.py`, `load_key_from_qr_code`) read text first and
/// take 16 or 32 bytes that no text kind took as entropy, whether or not
/// the bytes happen to be valid UTF-8. So does this: bytes that are not
/// text are one, and so is text that no arm of [`classify`] and no
/// setup-code or entry-list prefix claims. The one rule the classifier and
/// the device's seed readers share.
pub fn is_compact_seedqr(bytes: &[u8]) -> bool {
    if !matches!(bytes.len(), 16 | 32) {
        return false;
    }
    let head = bytes.trim_ascii_start();
    if head.starts_with(b"psbt\xff") || head.starts_with(b"cHNidP8") {
        return false;
    }
    let Ok(text) = core::str::from_utf8(bytes) else {
        return true;
    };
    let text = text.trim_ascii();
    let starts = |p: &str| {
        text.get(..p.len())
            .is_some_and(|h| h.eq_ignore_ascii_case(p))
    };
    if starts("otpauth-migration://") || looks_like_entries(text) {
        return false;
    }
    text_kind(text).is_none()
}

/// Costs what its parsers cost, the last arm as much as
/// [`crate::kef::recognize`] (base43 text of up to
/// [`crate::kef::BASE43_MAX`] characters, a time that grows with the
/// square of the text): what kind of payload `bytes` is. It runs nothing
/// beyond parsing.
pub fn classify(bytes: &[u8]) -> PayloadKind {
    let trimmed = bytes.trim_ascii();
    if matches!(trimmed.len(), 48 | 96) && trimmed.iter().all(u8::is_ascii_digit) {
        return PayloadKind::SeedQr {
            words: (trimmed.len() / 4) as u8,
        };
    }
    if trimmed.starts_with(b"psbt\xff") || trimmed.starts_with(b"cHNidP8") {
        return PayloadKind::Psbt;
    }
    if let Ok(text) = core::str::from_utf8(trimmed)
        && let Some(kind) = text_kind(text)
    {
        return kind;
    }
    if is_compact_seedqr(bytes) {
        return PayloadKind::CompactSeedQr {
            words: (bytes.len() * 3 / 4) as u8,
        };
    }
    if crate::kef::recognize(bytes).is_some() {
        return PayloadKind::Kef;
    }
    PayloadKind::Unknown
}

/// Whether `text` is a codex32 string: the `ms1` prefix in either case,
/// and a string the codec takes whole.
fn is_codex32(text: &str) -> bool {
    text.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("ms1"))
        && faraday_bip::codex32::Codex32::parse(text).is_ok()
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

/// Costs the whole list, twice for a Latin word not in its stored form:
/// the index of `word` in `lang`, in the stored (published or NFKD) form, or,
/// for the Latin-script lists, an accent-folded match so that text with
/// precomposed accents (`ábaco` as one code point, the usual form in a
/// QR or a file) is found without Unicode tables. Words are matched
/// whole, and every entry is compared in full, so the time does not
/// follow the word's place in the list; the Load wizard's checksum step
/// catches a wrong one.
pub fn word_index(lang: Language, word: &str) -> Option<u16> {
    if let Some(i) = lang.index_of(word) {
        return Some(i);
    }
    if !lang.is_latin() {
        return None;
    }
    let mut folded = faraday_bip::bip39::Typed::new();
    if !fold_precomposed(word, &mut folded) {
        return None;
    }
    lang.index_of_typed(&folded)
}

/// Lower-case ASCII of a Latin word whose accents may be precomposed:
/// the Latin-1 and Latin Extended-A letters the five accented wordlists
/// use (Spanish, French, Italian, Czech, Portuguese) map to their base
/// letter; combining marks are dropped. Written into `out`, which wipes
/// itself; false, with `out` cleared, for anything else or a word too
/// long for it.
fn fold_precomposed(word: &str, out: &mut faraday_bip::bip39::Typed) -> bool {
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
    out.clear();
    for c in word.chars() {
        let c = c.to_lowercase().next().unwrap_or(c);
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            continue;
        }
        let base = if c.is_ascii_alphabetic() {
            Some(c.to_ascii_lowercase())
        } else {
            BASES
                .iter()
                .find(|(set, _)| set.contains(c))
                .map(|(_, base)| char::from(*base))
        };
        if !base.is_some_and(|b| out.push(b)) {
            out.clear();
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_crypto::work::Compute;

    const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";

    /// A word from a QR or a file, in capitals or with its accents as one
    /// code point each, is found at its place on the published list,
    /// wherever on the list it is.
    #[test]
    fn latin_words_in_capitals_or_precomposed_are_found() {
        const COMPOSE: [(&str, &str); 6] = [
            ("a\u{301}", "á"),
            ("e\u{301}", "é"),
            ("i\u{301}", "í"),
            ("o\u{301}", "ó"),
            ("u\u{301}", "ú"),
            ("n\u{303}", "ñ"),
        ];
        for lang in Language::ALL.into_iter().filter(|l| l.is_latin()) {
            for i in (0..2048u16).step_by(101).chain([2047]) {
                let upper = lang.word(i).to_uppercase();
                assert_eq!(word_index(lang, &upper), Some(i), "{lang:?} {upper}");
            }
        }
        let mut composed = 0;
        for i in 0..2048u16 {
            let nfkd = Language::Spanish.word_nfkd(i);
            if nfkd.is_ascii() {
                continue;
            }
            let word = COMPOSE
                .iter()
                .fold(nfkd.to_string(), |w, (d, c)| w.replace(d, c));
            assert!(word.chars().all(|c| !('\u{300}'..='\u{36f}').contains(&c)));
            assert_eq!(word_index(Language::Spanish, &word), Some(i), "{word}");
            assert_eq!(
                word_index(Language::Spanish, &word.to_uppercase()),
                Some(i),
                "{word}"
            );
            composed += 1;
        }
        assert!(composed > 0);
        assert_eq!(word_index(Language::Spanish, "abacoo"), None);
        assert_eq!(word_index(Language::Spanish, "ábac0"), None);
    }

    #[test]
    fn routing_table() {
        let c = Compute::unchecked();
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
        let m = faraday_bip::bip39::Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
        let master = faraday_bip::keys::MasterKey::from_seed(
            &c,
            &m.to_seed(&c, b"").unwrap(),
            faraday_bip::keys::Network::Signet,
        );
        let signet = master
            .account_xpub(&c, faraday_bip::keys::ScriptType::NativeSegwit, 0)
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
        assert_eq!(classify(b"hello world"), PayloadKind::Unknown);
        assert_eq!(classify(b"nothing(here"), PayloadKind::Unknown);
        assert_eq!(classify(b""), PayloadKind::Unknown);
        assert_eq!(classify(b"ur:"), PayloadKind::Unknown);
    }

    /// Krux's own codes, from `tools/vectors/kef/`.
    const KRUX_CTR: &str = "07746573742049440f0000014f52a1936c3e3271209e9d64de442732247c37cb455d1b5410b25744dad4ac6fe4f2f4cf5c4de77bfc829682938298f0";
    const KRUX_GCM: &str =
        "07746573742049441400000a4f52a1936c3e3271209e9d64bfb7766f5d5d8a4f908e86e738344c025d8fed54";
    const KRUX_GRIND: &str = "08613165366337653405000001e3c3b2a72967a5711e54d8734bf1fdd02ddc80";

    #[test]
    fn a_krux_encrypted_code_is_kef_in_every_form() {
        use crate::kef::{Form, encode};
        for hex in [KRUX_CTR, KRUX_GCM, KRUX_GRIND] {
            let raw = crate::encodings::from_hex(hex).unwrap();
            // The grind code is exactly 32 raw bytes: KEF section 8's
            // case, which stays a CompactSeedQR.
            let expect = if raw.len() == 32 {
                PayloadKind::CompactSeedQr { words: 24 }
            } else {
                PayloadKind::Kef
            };
            assert_eq!(classify(&raw), expect, "{hex}");
            for form in [Form::Hex, Form::Base32, Form::Base43, Form::Base64] {
                let text = encode(form, &raw).unwrap();
                assert_eq!(classify(text.as_bytes()), PayloadKind::Kef, "{form:?}");
            }
            assert_eq!(classify(hex.as_bytes()), PayloadKind::Kef, "lower-case hex");
        }
    }

    #[test]
    fn raw_16_and_32_bytes_stay_a_compact_seedqr_even_when_they_parse_as_kef() {
        // Version 5 with an 8-byte id and a payload that fills 32 bytes.
        let mut found = 0;
        for len in [16usize, 32] {
            for lead in 0..=255u8 {
                for v in [5u8, 6, 7, 10, 11, 12, 15, 16, 20, 21] {
                    for tail in 0..=255u8 {
                        let mut b = alloc::vec![tail; len];
                        b[0] = lead;
                        b[1 + usize::from(lead) % (len - 1)] = v;
                        if crate::kef::recognize(&b).is_some() {
                            found += 1;
                            assert_eq!(
                                classify(&b),
                                PayloadKind::CompactSeedQr {
                                    words: (len * 3 / 4) as u8
                                }
                            );
                        }
                    }
                }
            }
        }
        assert!(found > 0, "no sample parsed as KEF, so nothing was checked");
    }

    #[test]
    fn a_code_that_is_not_an_envelope_is_not_kef() {
        use crate::kef::{Form, encode};
        let mut over = alloc::vec![253u8];
        over.extend_from_slice(&[b'x'; 253]);
        over.extend_from_slice(&[5, 0, 0, 1]);
        over.extend_from_slice(&[0u8; 40]);
        let mut unknown = crate::encodings::from_hex(KRUX_GCM).unwrap();
        let at = 1 + usize::from(unknown[0]);
        unknown[at] = 99;
        let mut zero = crate::encodings::from_hex(KRUX_GCM).unwrap();
        zero[at + 1..at + 4].copy_from_slice(&[0, 0, 0]);
        for bytes in [over, unknown, zero] {
            assert_eq!(classify(&bytes), PayloadKind::Unknown);
            let text = encode(Form::Base64, &bytes).unwrap();
            assert_eq!(classify(text.as_bytes()), PayloadKind::Unknown);
        }
    }

    #[test]
    fn other_kinds_vectors_are_not_taken_as_kef() {
        let v = include_str!("../../../tools/vectors/bip93/vectors.json");
        for s in v.split('"').filter(|s| s.starts_with("ms1")) {
            assert_ne!(classify(s.as_bytes()), PayloadKind::Kef, "{s}");
        }
        for file in [
            &include_bytes!("../../../tools/vectors/bbqr/1in2out.psbt")[..],
            include_bytes!("../../../tools/vectors/bbqr/devils-txn.txn"),
            include_bytes!("../../../tools/vectors/bbqr/finalized-by-ckcc.txn"),
            include_bytes!("../../../tools/vectors/bbqr/last.txn"),
            include_bytes!("../../../tools/vectors/bbqr/nfc-result.txn"),
            include_bytes!("../../../tools/vectors/bbqr/real-scan.txt"),
            include_bytes!("../../../tools/vectors/bbqr/signed.txn"),
        ] {
            assert_ne!(classify(file), PayloadKind::Kef);
        }
    }

    // Entropy bytes that happen to be ASCII text are still a
    // CompactSeedQR, as SeedSigner and Krux read them; text that is
    // a setup code is not.
    #[test]
    fn compact_seedqr_whose_bytes_are_text() {
        assert_eq!(
            classify(b"abcdefghijklmnop"),
            PayloadKind::CompactSeedQr { words: 12 }
        );
        assert_eq!(
            classify(b"abcdefghijklmnopqrstuvwxyzabcdef"),
            PayloadKind::CompactSeedQr { words: 24 }
        );
        assert!(!is_compact_seedqr(b"otpauth://totp/a?secret=ABCDEFGH"));
    }

    #[test]
    fn psbt_and_entries_text_of_16_or_32_bytes_is_not_a_seed() {
        let b64 = b"cHNidP8BAHECAAAAAQAAAAAAAAAAAAAA";
        let binary = b"psbt\xffabcdefghijk";
        assert_eq!((b64.len(), binary.len()), (32, 16));
        assert!(!is_compact_seedqr(b64));
        assert!(!is_compact_seedqr(binary));
        assert_eq!(classify(b64), PayloadKind::Psbt);
        assert_eq!(classify(binary), PayloadKind::Psbt);
        let later = b"ab\ntitle: abcdef";
        let nbsp = "\u{a0}title: abcdefg".as_bytes();
        assert_eq!((later.len(), nbsp.len()), (16, 16));
        assert!(!is_compact_seedqr(later));
        assert!(!is_compact_seedqr(nbsp));
    }

    #[test]
    fn a_value_that_is_text_only_inside_non_ascii_whitespace_is_classified_one_way() {
        // An address between no-break spaces (U+00A0, 2 bytes each): a
        // text kind only if Unicode whitespace is trimmed, which neither
        // reader does, so both call it a CompactSeedQR.
        let address = "1111111111111111111114oLvT2";
        assert_eq!(classify(address.as_bytes()), PayloadKind::Address);
        let edged = format!("\u{a0} {address}\u{a0}");
        assert_eq!(edged.len(), 32);
        assert_eq!(
            classify(edged.as_bytes()),
            PayloadKind::CompactSeedQr { words: 24 }
        );
        assert!(is_compact_seedqr(edged.as_bytes()));
    }
}
