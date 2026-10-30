//! What a code read is (`docs/SCANNING.md` §2, §4.6): one classifier that
//! every scanner, Paste and a stick's pictures go through, so the camera
//! and Paste name and take the same kinds. A secret is told apart on the
//! one code read ([`secret_kind`]); anything else once a transfer in
//! parts is whole, from what Files would file it as ([`public_kind`]).

use crate::strings::EN;
use crate::wallet::FileKind;
use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;

/// What a code is: §2's kinds, in the words the scanner names them with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// 12 to 24 English BIP-39 words whose checksum holds.
    SeedWords,
    /// A SeedQR's 48 or 96 digits.
    SeedQr,
    /// A CompactSeedQR's 16 or 32 bytes.
    CompactSeedQr,
    /// A codex32 string with share index `s`.
    Codex32Secret,
    /// A codex32 string with another share index.
    Codex32Share,
    /// A SLIP-39 share.
    Slip39Share,
    /// Words of a BIP-39 list that are no English BIP-39 seed.
    BadChecksum,
    /// A WIF private key.
    Wif,
    /// An extended private key.
    Xprv,
    /// `ur:crypto-seed`.
    UrSeed,
    /// A Nostr secret key, `nsec1…`.
    Nostr,
    /// A GPG key's paperkey, in either of its forms.
    Paperkey,
    /// One `otpauth://` URI.
    Totp,
    /// Vault entries: `field: value` lines, several `otpauth://` URIs.
    Entries,
    /// A PSBT.
    Psbt,
    /// A finished transaction.
    Transaction,
    /// A wallet description.
    Wallet,
    /// A multisig config with keys left off.
    SplitSheets,
    /// An account key.
    AccountKey,
    /// An address.
    Address,
    /// A `bitcoin:` URI (BIP-21).
    PaymentRequest,
    /// A silent payment address.
    SilentAddress,
    /// A signed message.
    Message,
    /// An armoured GPG public key.
    Gpg,
    /// A file Faraday keeps as it came: a Faraday file envelope, a vault,
    /// a PDF, a sheet.
    File,
    /// BIP-329 labels.
    Labels,
    /// A Lightning invoice.
    Invoice,
    /// A web address.
    Web,
    /// Printable text that is none of these.
    Text,
    /// Bytes that are none of these.
    Unreadable,
}

impl Kind {
    /// Whether the kind is a whole seed a scanner that loads one takes.
    pub fn seed(self) -> bool {
        matches!(
            self,
            Kind::SeedWords | Kind::SeedQr | Kind::CompactSeedQr | Kind::Codex32Secret
        )
    }

    /// What the kind is called (`docs/SCANNING.md` §7.2).
    pub fn label(self) -> &'static str {
        match self {
            Kind::SeedWords => EN.read_seed_words,
            Kind::SeedQr => EN.read_seedqr,
            Kind::CompactSeedQr => EN.read_compact_seedqr,
            Kind::Codex32Secret => EN.read_codex32_secret,
            Kind::Codex32Share => EN.read_codex32_share,
            Kind::Slip39Share => EN.read_slip39_share,
            Kind::BadChecksum => EN.read_bad_checksum,
            Kind::Wif => EN.read_private_key,
            Kind::Xprv => EN.read_xprv,
            Kind::UrSeed => EN.read_ur_seed,
            Kind::Nostr => EN.read_nostr_secret,
            Kind::Paperkey => EN.read_paperkey,
            Kind::Totp => EN.read_totp,
            Kind::Entries => EN.read_entries,
            Kind::Psbt => EN.read_psbt,
            Kind::Transaction => EN.read_transaction,
            Kind::Wallet => EN.read_wallet,
            Kind::SplitSheets => EN.read_split_sheets,
            Kind::AccountKey => EN.read_account_key,
            Kind::Address => EN.read_address,
            Kind::PaymentRequest => EN.read_payment_request,
            Kind::SilentAddress => EN.read_silent_address,
            Kind::Message => EN.read_message,
            Kind::Gpg => EN.read_gpg,
            Kind::File => EN.read_file,
            Kind::Labels => EN.read_labels,
            Kind::Invoice => EN.read_invoice,
            Kind::Web => EN.read_web,
            Kind::Text => EN.read_text,
            Kind::Unreadable => EN.read_unreadable,
        }
    }

    /// The scanner's line for a kind it does not take: "A transaction to
    /// sign · not used here"; what is nothing Faraday reads is said so.
    pub fn not_here(self) -> String {
        if self == Kind::Unreadable {
            return EN.read_unreadable.to_string();
        }
        crate::strings::fill(EN.read_not_here, &[self.label()])
    }
}

/// Costs a mnemonic's checksum, a codex32 and a SLIP-39 parse, a WIF's
/// checksum and a CRC-24 over a paperkey's lines. The secret kind one
/// code read is, if it is one: a seed in any of its forms, a share of
/// one, words of a BIP-39 list that are no seed, a private key, a
/// paperkey, a TOTP setup code or vault entries.
pub(crate) fn secret_kind(c: &Compute, bytes: &[u8]) -> Option<Kind> {
    if faraday_keys::gpg::is_paperkey(bytes) {
        return Some(Kind::Paperkey);
    }
    // A CompactSeedQR is 16 or 32 bytes of entropy, not text.
    if matches!(bytes.len(), 16 | 32) && std::str::from_utf8(bytes).is_err() {
        return Some(Kind::CompactSeedQr);
    }
    let text = zeroize::Zeroizing::new(String::from_utf8_lossy(bytes).trim().to_string());
    secret_text_kind(c, &text)
}

/// Costs what [`secret_kind`] costs but the paperkey's CRC. The secret
/// kind a code's text is, if it is one.
pub(crate) fn secret_text_kind(c: &Compute, text: &str) -> Option<Kind> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if matches!(t.len(), 48 | 96) && t.bytes().all(|b| b.is_ascii_digit()) {
        return Some(Kind::SeedQr);
    }
    if faraday_keys::identity::is_wif(t) {
        return Some(Kind::Wif);
    }
    if t.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("nsec1")) {
        return Some(Kind::Nostr);
    }
    match faraday_intake::secret_text(c, t) {
        Some(faraday_intake::TextSecret::Words) => return Some(Kind::SeedWords),
        Some(faraday_intake::TextSecret::ExtendedPrivateKey) => return Some(Kind::Xprv),
        None => {}
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    if matches!(words.len(), 12 | 15 | 18 | 21 | 24) && on_a_word_list(&words) {
        return Some(Kind::BadChecksum);
    }
    if let Ok(s) = faraday_bip::codex32::Codex32::parse(t) {
        return Some(if s.is_secret() {
            Kind::Codex32Secret
        } else {
            Kind::Codex32Share
        });
    }
    let lower = zeroize::Zeroizing::new(t.to_lowercase());
    if faraday_bip::slip39::Share::parse(&lower).is_ok() {
        return Some(Kind::Slip39Share);
    }
    if lower.starts_with("ur:crypto-seed/") {
        return Some(Kind::UrSeed);
    }
    if faraday_intake::looks_like_entries(t) {
        let one = t.lines().filter(|l| !l.trim().is_empty()).count() == 1;
        return Some(if one && t.starts_with("otpauth://") {
            Kind::Totp
        } else {
            Kind::Entries
        });
    }
    None
}

/// Whether every word is on one of the BIP-39 lists Faraday reads, in any
/// case. A lookup per word.
pub(crate) fn on_a_word_list(words: &[&str]) -> bool {
    Language::ALL.iter().any(|lang| {
        words.iter().all(|w| {
            lang.index_of(&zeroize::Zeroizing::new(w.to_lowercase()))
                .is_some()
        })
    })
}

/// The kind of a whole code that is no secret, from what Files files it
/// as (`kind`) and its text: an address and a payment request are told
/// apart here, since nothing files them. A lookup.
pub(crate) fn public_kind(kind: FileKind, enveloped: bool, text: &str) -> Kind {
    if enveloped {
        return Kind::File;
    }
    match kind {
        FileKind::Psbt => Kind::Psbt,
        FileKind::Wallet | FileKind::SecretWallet => Kind::Wallet,
        FileKind::Transaction => Kind::Transaction,
        FileKind::Message => Kind::Message,
        FileKind::Key => Kind::AccountKey,
        FileKind::Words => Kind::SeedWords,
        FileKind::Share => Kind::SplitSheets,
        FileKind::Entries => Kind::Entries,
        FileKind::SeedPart => Kind::Codex32Share,
        FileKind::Gpg => Kind::Gpg,
        FileKind::Sheet | FileKind::Vault | FileKind::EfiImage | FileKind::Kdbx | FileKind::Pdf => {
            Kind::File
        }
        FileKind::Text | FileKind::Other => text_kind(text, kind == FileKind::Text),
    }
}

/// What text Files keeps as a note, or reads as nothing, is: an
/// address, a payment request, labels, an invoice, a web address, or
/// text. A lookup.
fn text_kind(text: &str, printable: bool) -> Kind {
    let t = text.trim();
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("bitcoin:") {
        return Kind::PaymentRequest;
    }
    if (lower.starts_with("sp1") || lower.starts_with("tsp1")) && !t.contains(char::is_whitespace) {
        return Kind::SilentAddress;
    }
    // An address's characters, as many as one has: the scanner checks it
    // against the loaded wallets before it is named (`address_note`).
    let address = t.len() >= 26
        && !t.contains(char::is_whitespace)
        && faraday_bip::keys::Network::ALL
            .iter()
            .any(|&n| faraday_bip::address::is_address_prefix(t, n));
    if address {
        return Kind::Address;
    }
    if lower.starts_with("lightning:") || lower.starts_with("lnbc") || lower.starts_with("lntb") {
        return Kind::Invoice;
    }
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return Kind::Web;
    }
    let labels = t.lines().filter(|l| !l.trim().is_empty()).all(|l| {
        let l = l.trim();
        l.starts_with('{') && l.contains("\"type\"") && l.contains("\"ref\"")
    });
    if !t.is_empty() && labels {
        return Kind::Labels;
    }
    if printable {
        Kind::Text
    } else {
        Kind::Unreadable
    }
}
