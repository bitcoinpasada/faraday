//! APK signing keys in a vault (`docs/APK-SIGNING.md`): making one, what
//! its page shows, and signing an APK copied in with it. The key,
//! the certificate, the reader and the signer are `faraday-apk`'s; the key
//! lives in an open vault as a type-12 record (`docs/VAULT.md` §7) and only
//! the certificate and signed APKs reach the Outbox.
//!
//! A key made in the backup flow signs nothing until every place of its
//! plan is written (`docs/BACKUP-FLOW.md`, §3 here): the gate is
//! [`Faraday::backup_waits`]. Nothing gates on a stick's FAT serial.

use std::cell::RefCell;

use crate::strings::EN;
use crate::strings::fill;
use faraday_apk::{Apk, Info, Refusal, Slice, key, pem, sign};
use faraday_crypto::work::Compute;
use faraday_keys::vault::RecordView;
use faraday_vault::records::{field, kind};

use crate::Faraday;
use crate::backup_flow::Kind;
use crate::backup_flow::steps::Built;
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
    listed: RefCell<Vec<Read>>,
}

/// What one APK read as, by its whole SHA-256.
struct Read {
    sha: [u8; 32],
    result: Result<Info, Refusal>,
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
fn sha_of(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    faraday_crypto::sha256_whole(c, bytes)
}

impl Faraday {
    /// Costs a hash of a whole file (through `sha_of`). What APK `item`
    /// reads as, remembered while it stays as it is.
    fn apk_read(&self, c: &Compute, item: &crate::Item) -> Result<Info, Refusal> {
        // Read once per file, by the SHA-256 taken when it came in.
        let sha = item.sha.unwrap_or_else(|| sha_of(c, &item.bytes));
        let mut cache = self.vaults.apk.listed.borrow_mut();
        if let Some(e) = cache.iter().find(|e| e.sha == sha) {
            return e.result.clone();
        }
        let result = Apk::read(&mut Slice(&item.bytes)).map(|a| a.info);
        if cache.len() >= APK_READS_KEPT {
            cache.remove(0);
        }
        cache.push(Read {
            sha,
            result: result.clone(),
        });
        result
    }

    /// Costs a hash of a whole file (through `apk_read`). What the Inbox
    /// APK `name` states of itself, when it reads.
    pub fn apk_info(&self, c: &Compute, name: &str) -> Option<Info> {
        let item = self.inbox.iter().find(|i| i.name == name)?;
        self.apk_read(c, item).ok()
    }

    /// Costs a hash of a whole file (through `apk_read`). The APKs in
    /// Files, newest first, each with why it cannot be signed with a key
    /// for `package` (`docs/APK-SIGNING.md` §4.4).
    pub fn apk_listed(&self, c: &Compute, package: &str) -> Vec<Listed> {
        let mut out = Vec::new();
        for (k, item) in self.inbox.iter().enumerate().rev() {
            if item.kind != crate::FileKind::Apk {
                continue;
            }
            let (info, why) = match self.apk_read(c, item) {
                Ok(info) => {
                    let why = if info.package != package {
                        Some(Why::Other(info.package.clone()))
                    } else if output_name(&item.name).len() > NAME_MAX {
                        Some(Why::NameTooLong)
                    } else if item.bytes.len() as u64 + 2 * sign::PAGE + 8192 > FAT32_MAX {
                        Some(Why::TooBig)
                    } else {
                        None
                    };
                    (Some(info), why)
                }
                Err(Refusal::AlreadySigned) => (None, Some(Why::AlreadySigned)),
                Err(Refusal::NotAligned) => (None, Some(Why::NotAligned)),
                Err(Refusal::OldSdk) => (None, Some(Why::OldSdk)),
                Err(_) => (None, Some(Why::NotApk)),
            };
            out.push(Listed {
                inbox: k,
                name: item.name.clone(),
                size: item.bytes.len(),
                info,
                why,
            });
        }
        out
    }

    /// The selected type-12 record, with its vault and place.
    pub(crate) fn apk_selected(&self) -> Option<(usize, RecordView<'_>)> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .records(self.vaults.open.get(self.vaults.current)?.id)
            .get(at)?;
        (r.kind() == kind::APK_KEY).then_some((at, r))
    }

    /// Every APK signing key in the open vaults whose package is
    /// `package`: vault, record place, name and fingerprint.
    pub fn apk_keys_for(&self, package: &str) -> Vec<(usize, usize, String, [u8; 32])> {
        let mut out = Vec::new();
        for (v, open) in self.vaults.open.iter().enumerate() {
            for (at, r) in self.vaults.records(open.id).iter() {
                if r.kind() != kind::APK_KEY || r.public_text(field::APK_PACKAGE) != Some(package) {
                    continue;
                }
                if let Some(s) = shown(r, self.tz_min()) {
                    out.push((v, at, s.name, fingerprint_of(r).unwrap_or_default()));
                }
            }
        }
        out
    }

    /// The type-12 record whose certificate has this SHA-256, with its
    /// vault and place.
    pub(crate) fn apk_find(&self, fp: &[u8; 32]) -> Option<(usize, usize)> {
        for (v, open) in self.vaults.open.iter().enumerate() {
            for (at, r) in self.vaults.records(open.id).iter() {
                if r.kind() == kind::APK_KEY && fingerprint_of(r).as_ref() == Some(fp) {
                    return Some((v, at));
                }
            }
        }
        None
    }

    /// Costs a hash of a whole file (through `sha_of`). A vault in Files
    /// that this session unlocked or wrote: its salt is an open vault's, or
    /// its bytes are what the last write of this session put on a stick. A
    /// copy of any other vault never goes over a vault of the same size on
    /// a stick.
    pub(crate) fn vault_is_this_sessions(&self, c: &Compute, bytes: &[u8]) -> bool {
        let Ok(h) = faraday_vault::read_header(bytes) else {
            return false;
        };
        if self.vaults.open.iter().any(|v| v.header().salt == h.salt) {
            return true;
        }
        // Only a write of this session's own visit counts: the flow's
        // sealings do not, so that a later ordinary visit never writes the
        // flow's bytes over a newer copy on another stick.
        let hash = sha_of(c, bytes);
        self.receipt
            .as_ref()
            .is_some_and(|r| !r.by_flow && r.holds_hash(&hash))
    }

    /// Costs a hash of a whole file (through `apk_read`). What an APK's row
    /// in Files offers (`docs/APK-SIGNING.md` §4.1): to sign, when an open
    /// vault holds a key for its package; a note of why not otherwise; and
    /// Unlock or Make a vault, where none is open.
    pub fn apk_files_row(&self, c: &Compute, k: usize) -> Row {
        if !self.apk_signing {
            return Row::None;
        }
        let Some(item) = self.inbox.get(k) else {
            return Row::None;
        };
        let info = match self.apk_read(c, item) {
            Ok(i) => i,
            Err(Refusal::AlreadySigned) => return Row::Note(Why::AlreadySigned.text()),
            Err(Refusal::NotAligned) => return Row::Note(Why::NotAligned.text()),
            Err(Refusal::OldSdk) => return Row::Note(Why::OldSdk.text()),
            Err(_) => return Row::Note(Why::NotApk.text()),
        };
        if self.vaults.open.is_empty() {
            return Row::Sign;
        }
        if self.apk_keys_for(&info.package).is_empty() {
            return Row::Note(Why::Other(info.package).text());
        }
        if output_name(&item.name).len() > NAME_MAX {
            return Row::Note(Why::NameTooLong.text());
        }
        let keys = self.apk_keys_for(&info.package);
        if keys
            .iter()
            .all(|k| self.backup_waits(Kind::Apk, &k.3).is_some())
        {
            return Row::Back;
        }
        Row::Sign
    }

    /// Selects a key by its place for the contents to show.
    fn apk_select(&mut self, v: usize, at: usize) {
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let n = self
            .vaults
            .records(open.id)
            .iter()
            .take(at)
            .filter(|(_, r)| r.kind() == kind::APK_KEY)
            .count();
        self.vaults.current = v;
        self.vaults.category = 6;
        self.vaults.category_chosen = true;
        self.vaults.item[6] = n;
        self.vaults.item_open = true;
        self.vaults.form = None;
        self.vaults.saving = false;
        self.vaults.prompt = None;
        self.vaults.signing = false;
        self.vaults.sb_images = false;
        self.vault_item_offset = 0.0;
        self.screen = crate::Screen::VaultContents;
    }

    /// Files' Sign with APK key: with one key of an open vault for the
    /// APK's package, its review; with several, which key.
    pub(crate) fn apk_from_files(&mut self, k: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let name = item.name.clone();
        let Ok(info) = self.apk_read(&c, item) else {
            return;
        };
        let keys = self.apk_keys_for(&info.package);
        let Some(&(v, at, _, fp)) = keys.first() else {
            return;
        };
        if keys.len() == 1 && self.backup_waits(Kind::Apk, &fp).is_some() {
            // The key waits for its backup: the flow opens on its step.
            return self.backup_continue(Kind::Apk, &fp);
        }
        self.apk_select(v, at);
        self.vaults.apk.leave();
        self.vaults.apk.signing = true;
        if keys.len() == 1 {
            self.apk_open_review(&name, fp);
        } else {
            self.vaults.apk.which = Some(name);
        }
    }

    /// "Which key?" was answered.
    pub(crate) fn apk_key_picked(&mut self, v: usize, at: usize) {
        let Some(name) = self.vaults.apk.which.clone() else {
            return;
        };
        let Some(fp) = self
            .vaults
            .open
            .get(v)
            .and_then(|o| self.vaults.records(o.id).get(at))
            .and_then(crate::apk::fingerprint_of)
        else {
            return;
        };
        if self.backup_waits(Kind::Apk, &fp).is_some() {
            return self.backup_continue(Kind::Apk, &fp);
        }
        self.apk_select(v, at);
        self.vaults.apk.leave();
        self.vaults.apk.signing = true;
        self.apk_open_review(&name, fp);
    }

    /// Makes the key the flow's form described, once: the record every
    /// vault of the plan takes, and the certificate For the stick.
    pub(crate) fn apk_build(&mut self, name: &str, package: &str) -> Result<Built, String> {
        let (name, package) = Self::apk_check_form(name, package).map_err(str::to_string)?;
        let now = self
            .clock_now()
            .ok_or_else(|| EN.apk_this_computer.to_string())?;
        let seed = self
            .fresh(b"apk key")
            .map(zeroize::Zeroizing::new)
            .ok_or_else(|| EN.apk_no_randomness.to_string())?;
        let made = faraday_keys::apk::create(&seed, &name, &package, now)
            .map_err(|e| e.reason().to_string())?;
        let fp = key::fingerprint(&made.cert);
        let stem = file_stem(&name, &fp);
        Ok(Built {
            record: made.key,
            subject: fp.to_vec(),
            name,
            outbox: vec![(
                format!("{stem}-certificate.pem"),
                pem::encode(&made.cert).into_bytes(),
            )],
        })
    }

    /// The Make form's checks: the certificate's name and the package, in
    /// the words they are refused in.
    pub(crate) fn apk_check_form(
        name: &str,
        package: &str,
    ) -> Result<(String, String), &'static str> {
        let name = key::check_name(name).map_err(|_| EN.apk_name_the_certificate)?;
        let package = package.trim();
        if !key::valid_package(package) {
            return Err(EN.apk_package_invalid);
        }
        Ok((name, package.to_string()))
    }

    /// The selected key's certificate to the Outbox.
    pub(crate) fn apk_export(&mut self) {
        let Some((_, r)) = self.apk_selected() else {
            return;
        };
        let (Some(cert), Some(fp)) = (r.public(field::APK_CERT), fingerprint_of(r)) else {
            return;
        };
        let Some(s) = shown(r, self.tz_min()) else {
            return;
        };
        let name = format!("{}-certificate.pem", file_stem(&s.name, &fp));
        self.put_outbox(&name, pem::encode(cert).into_bytes());
        self.toast_out(&name);
    }

    /// Costs what `text` costs. The QR view of the selected key's
    /// certificate, with the name and label of its picture.
    pub(crate) fn apk_code(&self, c: &Compute) -> Result<crate::QrView, String> {
        let (_, r) = self
            .apk_selected()
            .ok_or_else(|| EN.apk_no_key_is_selected.to_string())?;
        let cert = r
            .public(field::APK_CERT)
            .ok_or_else(|| EN.apk_no_key_is_selected.to_string())?;
        let fp = fingerprint_of(r).unwrap_or_default();
        let s = shown(r, self.tz_min()).ok_or_else(|| EN.apk_no_key_is_selected.to_string())?;
        let stem = file_stem(&s.name, &fp);
        let lines = vec![
            s.name.clone(),
            fill(EN.apk_cert_fingerprint_line, &[&s.fingerprint]),
            EN.apk_public_checks.to_string(),
        ];
        Ok(
            crate::QrView::text(c, EN.apk_certificate, &pem::encode(cert))?
                .public(&format!("{stem}.png"), lines),
        )
    }

    /// Opens the review of Inbox APK `name` for the key `fp`.
    pub(crate) fn apk_open_review(&mut self, name: &str, fp: [u8; 32]) {
        let c = self.compute();
        let Some(item) = self.inbox.iter().find(|i| i.name == name) else {
            return;
        };
        let sha = sha_of(&c, &item.bytes);
        self.vaults.apk.review = Some(Review {
            name: name.to_string(),
            key: fp,
            sha,
        });
        self.vaults.apk.second.clear();
        self.vaults.apk.which = None;
        self.vaults.focus = Some(crate::vault::Focus::Second);
    }

    /// The second build typed so far against the file under review.
    pub fn apk_second(&self) -> Second {
        match &self.vaults.apk.review {
            Some(r) => compare_second(&self.vaults.apk.second.text, &r.sha),
            None => Second::Empty,
        }
    }

    /// Signs the APK under review: the hold completed. Everything is
    /// checked again here, and nothing is written when anything fails.
    pub(crate) fn apk_sign(&mut self) {
        let c = self.compute();
        let Some(review) = self.vaults.apk.review.clone() else {
            return;
        };
        if !matches!(self.apk_second(), Second::Matches(_)) {
            return;
        }
        if self.backup_waits(Kind::Apk, &review.key).is_some() {
            return;
        }
        let Some((v, at)) = self.apk_find(&review.key) else {
            return;
        };
        let id = self.vaults.open[v].id;
        let Some(package) = self
            .vaults
            .records(id)
            .get(at)
            .and_then(|r| r.public_text(field::APK_PACKAGE))
            .map(str::to_string)
        else {
            return;
        };
        let Some(seed) = self.fresh(b"apk signing").map(zeroize::Zeroizing::new) else {
            return self.toast(EN.apk_no_randomness);
        };
        let Some(item) = self.inbox.iter().find(|i| i.name == review.name) else {
            return;
        };
        // The bytes signed are the bytes reviewed.
        if sha_of(&c, &item.bytes) != review.sha {
            self.toast(EN.apk_changed_since_review);
            self.vaults.apk.review = None;
            return;
        }
        let mut src = Slice(&item.bytes);
        let apk = match Apk::read(&mut src) {
            Ok(a) if a.info.package == package => a,
            _ => return,
        };
        let out_name = output_name(&item.name);
        if out_name.len() > NAME_MAX {
            return;
        }
        let signed = match self.vaults.store().apk_sign(id, at, &seed, &mut src, &apk) {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => return self.toast(e.reason()),
            None => return,
        };
        self.put_outbox(&out_name, signed);
        self.vaults.apk.review = None;
        self.vaults.apk.second.clear();
        self.vaults.focus = None;
        self.vaults.apk.signing = true;
        self.toast_out(&out_name);
    }
}

impl ApkState {
    /// How many APKs are kept read, for the test that bounds the caches.
    #[cfg(test)]
    pub(crate) fn kept(&self) -> usize {
        self.listed.borrow().len()
    }
}
