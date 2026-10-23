//! APK signing keys in a vault (`docs/APK-SIGNING.md`): making one, what
//! its page shows, and signing an APK copied in with it. The key,
//! the certificate, the reader and the signer are `faraday-apk`'s; the key
//! lives in an open vault as a type-12 record (`docs/VAULT.md` §7) and only
//! the certificate and signed APKs reach the Outbox.
//!
//! A key made in the backup flow signs nothing until every place of its
//! plan is written (`docs/BACKUP-FLOW.md`, §3 here): the gate is
//! [`Faraday::backup_waits`]. Nothing gates on a stick's FAT serial.
//!
//! Its presses and what they change are the vault area's glue, in
//! `vault/mod.rs`.

use std::cell::RefCell;

use crate::strings::EN;
use crate::strings::fill;
use faraday_apk::{Info, Refusal, key};
use faraday_crypto::work::Compute;
use faraday_keys::vault::RecordView;
use faraday_vault::records::field;

use crate::vault::TextBox;

/// The most APKs kept read ([`ApkState`]'s reads); the next one drops
/// the oldest.
pub(crate) const APK_READS_KEPT: usize = 32;

/// The longest a name For the stick is, in bytes of UTF-8
/// (`docs/LARGE-FILES.md` §9).
pub const NAME_MAX: usize = 230;
/// FAT32's largest file.
pub const FAT32_MAX: u64 = 4_294_967_295;
/// The fewest hex digits of the second build's SHA-256 that are compared.
pub const SECOND_MIN: usize = 32;
/// What the screens keep of APK signing.
#[derive(Default)]
pub struct ApkState {
    /// Sign an APK is open on the selected key.
    pub signing: bool,
    /// An APK copied in is being offered a key: "Which key?".
    pub which: Option<String>,
    /// The review of the APK to sign.
    pub review: Option<Review>,
    /// The second build's SHA-256, as typed.
    pub second: TextBox,
    /// What an APK read as, by its SHA-256, at most [`APK_READS_KEPT`].
    pub(crate) listed: RefCell<Vec<Read>>,
}

/// What one APK read as, by its whole SHA-256.
pub(crate) struct Read {
    pub(crate) sha: [u8; 32],
    pub(crate) result: Result<Info, Refusal>,
}

impl ApkState {
    /// Leaves whatever APK signing had on the contents: the result, the
    /// list of APKs, the review.
    pub fn leave(&mut self) {
        self.signing = false;
        self.which = None;
        self.review = None;
        self.second.clear();
    }
}

/// The APK being reviewed before it is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    /// Its name in the Inbox.
    pub name: String,
    /// The key that signs it, by its certificate's SHA-256.
    pub key: [u8; 32],
    /// SHA-256 of the file as copied in.
    pub sha: [u8; 32],
}

/// What the typed second build says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Second {
    /// Nothing typed.
    Empty,
    /// Fewer than 32 hex digits.
    TooShort,
    /// Not the start of the file's SHA-256.
    Differs,
    /// The start of it, with this many digits compared.
    Matches(usize),
}

/// Why an APK in Files cannot be signed with the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// Not an APK, or one read two ways.
    NotApk,
    /// Already carries a signature.
    AlreadySigned,
    /// Not aligned.
    NotAligned,
    /// Supports Android before 7.0.
    OldSdk,
    /// The manifest's package is not the key's.
    Other(String),
    /// The signed file would pass FAT32's largest.
    TooBig,
    /// The name it would be written under passes 230 bytes.
    NameTooLong,
}

impl Why {
    /// The words the row says it in.
    pub fn text(&self) -> String {
        match self {
            Why::NotApk => EN.apk_not_an_apk.to_string(),
            Why::AlreadySigned => EN.apk_already_signed.to_string(),
            Why::NotAligned => EN.apk_not_aligned.to_string(),
            Why::OldSdk => EN.apk_old_android_row.to_string(),
            Why::Other(p) => fill(EN.apk_other_package, &[p]),
            Why::TooBig => EN.apk_over_4_gib.to_string(),
            Why::NameTooLong => EN.apk_name_too_long.to_string(),
        }
    }
}

pub(crate) fn hex_array(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// What an APK's row in Files offers.
pub enum Row {
    /// **Sign with APK key**.
    Sign,
    /// The key waits for its backup: **Back up first**.
    Back,
    /// Why it cannot be signed.
    Note(String),
    /// Nothing.
    None,
}

/// An APK copied in, as Sign an APK lists it.
#[derive(Debug, Clone)]
pub struct Listed {
    /// Its place in the Inbox.
    pub inbox: usize,
    /// Its name.
    pub name: String,
    /// Its size.
    pub size: usize,
    /// What it states of itself, when it reads.
    pub info: Option<Info>,
    /// Why it cannot be signed, when it cannot.
    pub why: Option<Why>,
}

/// What a type-12 record shows.
pub struct Shown {
    /// The certificate's name.
    pub name: String,
    /// The package it signs for.
    pub package: String,
    /// SHA-256 of the certificate, in groups of four.
    pub fingerprint: String,
    /// The same, whole, lower case.
    pub hex: String,
    /// The day it was made, local.
    pub made: String,
}

/// The certificate's SHA-256, from a type-12 record.
pub fn fingerprint_of(r: RecordView<'_>) -> Option<[u8; 32]> {
    r.public(field::APK_CERT).map(key::fingerprint)
}

/// What a type-12 record shows. The date is local (`docs/TOTP.md`
/// §2.3).
pub fn shown(r: RecordView<'_>, tz_min: i16) -> Option<Shown> {
    let cert = r.public(field::APK_CERT)?;
    let info = key::describe(cert)?;
    Some(Shown {
        name: info.name,
        package: r.public_text(field::APK_PACKAGE)?.to_string(),
        fingerprint: key::grouped(&info.fingerprint),
        hex: faraday_apk::hex(&info.fingerprint),
        made: crate::gpg::date(crate::vault::local_secs(info.made, tz_min)),
    })
}

/// A name for the key's files: the certificate's name lower-cased, every
/// other character a hyphen, and the first 8 hex digits of its
/// fingerprint.
pub fn file_stem(name: &str, fp: &[u8; 32]) -> String {
    let name: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let tail = &faraday_apk::hex(fp)[..8];
    format!("{}-{tail}", name.trim_matches('-'))
        .trim_start_matches('-')
        .to_string()
}

/// What `name` is written as once signed: `-unsigned` removed before
/// `.apk`; a name without `-unsigned` gains `-signed` before a final
/// `.apk`, or `-signed.apk` where it has no `.apk` (both matched ignoring
/// ASCII case, the rest of the name kept as it is).
pub fn output_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    // `-unsigned` goes only where it stands before a final `.apk`; any
    // other name, `x-unsigned` included, gains `-signed`.
    if lower.ends_with("-unsigned.apk") {
        let at = name.len() - "-unsigned.apk".len();
        return format!("{}{}", &name[..at], &name[at + "-unsigned".len()..]);
    }
    if lower.ends_with(".apk") {
        let stem = &name[..name.len() - 4];
        format!("{stem}-signed{}", &name[name.len() - 4..])
    } else {
        format!("{name}-signed.apk")
    }
}

/// Android's release for an API level, as Sign an APK says "8.0 and
/// later".
pub fn android_release(api: u32) -> String {
    match api {
        24 => "7.0".into(),
        25 => "7.1".into(),
        26 => "8.0".into(),
        27 => "8.1".into(),
        28 => "9".into(),
        29..=31 => (api - 19).to_string(),
        32 => "12L".into(),
        33..=36 => (api - 20).to_string(),
        n => fill(EN.apk_api_level, &[&n.to_string()]),
    }
}

/// "0.1.3 (4)": `versionName` and `versionCode`; "No name (4)" without a
/// name; "(1:4)" where `versionCodeMajor` is not 0.
pub fn version_text(i: &Info) -> String {
    let name = i
        .version_name
        .clone()
        .unwrap_or_else(|| EN.apk_no_name.to_string());
    if i.version_code_major == 0 {
        format!("{name} ({})", i.version_code)
    } else {
        format!("{name} ({}:{})", i.version_code_major, i.version_code)
    }
}

/// The typed second build against the file's SHA-256: hex, case and
/// spaces ignored, at least 32 digits of the start.
pub fn compare_second(typed: &str, sha: &[u8; 32]) -> Second {
    let digits: String = typed
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if digits.is_empty() {
        return Second::Empty;
    }
    let full = faraday_apk::hex(sha);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) || digits.len() > full.len() {
        return Second::Differs;
    }
    if digits.len() < SECOND_MIN {
        return Second::TooShort;
    }
    if full.starts_with(&digits) {
        Second::Matches(digits.len())
    } else {
        Second::Differs
    }
}

/// Costs a SHA-256 pass over the whole file: the APK's SHA-256.
pub(crate) fn sha_of(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    faraday_crypto::sha256_whole(c, bytes)
}

impl ApkState {
    /// How many APKs are kept read, for the test that bounds the caches.
    #[cfg(test)]
    pub(crate) fn kept(&self) -> usize {
        self.listed.borrow().len()
    }
}
