//! APK signing keys in a vault (`docs/APK-SIGNING.md`): making one, what
//! its page shows, and signing an APK copied in with it. The key,
//! the certificate, the reader and the signer are `faraday-apk`'s; the key
//! lives in an open vault as a type-12 record (`docs/VAULT.md` §7) and only
//! the certificate and signed APKs reach the Outbox.
//!
//! A key made in a session signs nothing until the vault holding it has
//! been written, after the key was added, to a stick and then to a second
//! one (§3): [`Wait`] keeps where it is. The two writes are two visits,
//! each started by its own button; nothing gates on a stick's FAT serial,
//! since every Faraday stick of one release has the same.

use std::cell::RefCell;

use crate::strings::EN;
use faraday_apk::{Apk, Info, Refusal, Slice, key, pem, sign};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use opensigner_core::strings::fill;
use osk_bip::bitcoin::hashes::{Hash, sha256};

use crate::Faraday;
use crate::vaults::TextBox;

/// The longest a name For the stick is, in bytes of UTF-8
/// (`docs/LARGE-FILES.md` §9).
pub const NAME_MAX: usize = 230;
/// FAT32's largest file.
pub const FAT32_MAX: u64 = 4_294_967_295;
/// The fewest hex digits of the second build's SHA-256 that are compared.
pub const SECOND_MIN: usize = 32;
/// Sealings kept for a key that waits for its copies.
const SEALINGS_KEPT: usize = 16;

/// A key made this session and not yet written to two sticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wait {
    /// The certificate's SHA-256.
    pub fp: [u8; 32],
    /// SHA-256 of each sealing of its vault that holds the key.
    pub sealings: Vec<[u8; 32]>,
    /// The first write, once it was read back and matched.
    pub first: Option<First>,
}

/// The vault file as the first stick holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct First {
    /// Its name on the stick.
    pub name: String,
    /// Its length.
    pub len: usize,
    /// SHA-256 of the bytes written.
    pub hash: [u8; 32],
}

/// What the screens keep of APK signing.
#[derive(Default)]
pub struct ApkState {
    /// The first write's file being read back from the stick that came
    /// for the second copy: the stick's id and the file's name.
    pub probe: Option<(String, String)>,
    /// The stick (by id) whose file of the first write's name is byte for
    /// byte the first write.
    pub same_on: Option<String>,
    /// The visit the next stick is for: 1 the first write, 2 the second.
    /// Set by the key page's buttons, kept across the lock they make,
    /// gone when the stick is pulled.
    pub visit_for: Option<u8>,
    /// The key whose page a lock for the stick's sake comes back to once
    /// the vault is unlocked again; kept across the lock.
    pub back: Option<[u8; 32]>,
    /// The vault the key being made goes to, when several are open.
    pub vault_pick: Option<usize>,
    /// The key just made, while its result shows.
    pub made: Option<[u8; 32]>,
    /// Sign an APK is open on the selected key.
    pub signing: bool,
    /// An APK copied in is being offered a key: "Which key?".
    pub which: Option<String>,
    /// The review of the APK to sign.
    pub review: Option<Review>,
    /// The second build's SHA-256, as typed.
    pub second: TextBox,
    /// The key being made: name, package, vault, while its frame waits.
    pub making: Option<(String, String, usize)>,
    /// Keys made in this session, waiting for their copies.
    pub waits: Vec<Wait>,
    /// What each sealing of an open vault held, by its SHA-256.
    pub sealed: Vec<([u8; 32], Vec<[u8; 32]>)>,
    /// What an APK read as, by name, size and a check of its ends.
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
        self.made = None;
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
            Why::OldSdk => EN.apk_before_7.to_string(),
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

/// The waiting keys as the kept state holds them: a line each, the
/// certificate's SHA-256, the sealings, and the first write (name, length
/// and hash), split by tabs and commas.
pub fn waits_encode(waits: &[Wait]) -> Vec<u8> {
    waits
        .iter()
        .map(|w| {
            let sealings: Vec<String> = w.sealings.iter().map(|h| faraday_apk::hex(h)).collect();
            let first = w.first.as_ref().map_or(String::new(), |f| {
                format!("{},{},{}", f.len, faraday_apk::hex(&f.hash), f.name)
            });
            format!(
                "{}\t{}\t{}\n",
                faraday_apk::hex(&w.fp),
                sealings.join(","),
                first
            )
        })
        .collect::<String>()
        .into_bytes()
}

/// Read back what [`waits_encode`] kept.
pub fn waits_decode(bytes: &[u8]) -> Vec<Wait> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| {
            let mut f = line.split('\t');
            let fp = hex_array(f.next()?)?;
            let sealings = f.next()?;
            let first = f.next().unwrap_or("");
            let first = {
                let mut p = first.splitn(3, ',');
                match (p.next(), p.next(), p.next()) {
                    (Some(len), Some(hash), Some(name)) => Some(First {
                        len: len.parse().ok()?,
                        hash: hex_array(hash)?,
                        name: name.to_string(),
                    }),
                    _ => None,
                }
            };
            Some(Wait {
                fp,
                sealings: sealings.split(',').filter_map(hex_array).collect(),
                first,
            })
        })
        .collect()
}

/// What an APK's row in Files offers.
pub enum Row {
    /// **Sign with APK key**.
    Sign,
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

/// The record a made key is kept as.
pub fn record_of(pair: &faraday_sb::Pair, package: &str) -> Record {
    Record::new(kind::APK_KEY)
        .with(field::APK_KEY, &pair.key)
        .with(field::APK_CERT, &pair.cert)
        .with(field::APK_PACKAGE, package.as_bytes())
}

/// The certificate's SHA-256, from a type-12 record.
pub fn fingerprint_of(r: &Record) -> Option<[u8; 32]> {
    r.field(field::APK_CERT).map(key::fingerprint)
}

/// What a type-12 record shows. The date is local (`docs/TOTP.md`
/// §2.3).
pub fn shown(r: &Record, tz_min: i16) -> Option<Shown> {
    let cert = r.field(field::APK_CERT)?;
    let info = key::describe(cert)?;
    Some(Shown {
        name: info.name,
        package: r.text(field::APK_PACKAGE)?.to_string(),
        fingerprint: key::grouped(&info.fingerprint),
        hex: faraday_apk::hex(&info.fingerprint),
        made: crate::gpg::date(crate::vaults::local_secs(info.made, tz_min)),
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

fn sha_of(bytes: &[u8]) -> [u8; 32] {
    sha256::Hash::hash(bytes).to_byte_array()
}

impl Faraday {
    /// What APK `item` reads as, remembered while it stays as it is.
    fn apk_read(&self, item: &crate::Item) -> Result<Info, Refusal> {
        // Read once per file, by the SHA-256 taken when it came in.
        let sha = item.sha.unwrap_or_else(|| sha_of(&item.bytes));
        let mut cache = self.vaults.apk.listed.borrow_mut();
        if let Some(e) = cache.iter().find(|e| e.sha == sha) {
            return e.result.clone();
        }
        let result = Apk::read(&mut Slice(&item.bytes)).map(|a| a.info);
        cache.push(Read {
            sha,
            result: result.clone(),
        });
        result
    }

    /// What the Inbox APK `name` states of itself, when it reads.
    pub fn apk_info(&self, name: &str) -> Option<Info> {
        let item = self.inbox.iter().find(|i| i.name == name)?;
        self.apk_read(item).ok()
    }

    /// The APKs in Files, newest first, each with why it cannot be signed
    /// with a key for `package` (`docs/APK-SIGNING.md` §4.4).
    pub fn apk_listed(&self, package: &str) -> Vec<Listed> {
        let mut out = Vec::new();
        for (k, item) in self.inbox.iter().enumerate().rev() {
            if item.kind != crate::FileKind::Apk {
                continue;
            }
            let (info, why) = match self.apk_read(item) {
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
    pub(crate) fn apk_selected(&self) -> Option<(usize, Record)> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .open
            .get(self.vaults.current)?
            .contents
            .records
            .get(at)?;
        (r.kind == kind::APK_KEY).then(|| (at, r.clone()))
    }

    /// Every APK signing key in the open vaults whose package is
    /// `package`: vault, record place, name and fingerprint.
    pub fn apk_keys_for(&self, package: &str) -> Vec<(usize, usize, String, [u8; 32])> {
        let mut out = Vec::new();
        for (v, open) in self.vaults.open.iter().enumerate() {
            for (at, r) in open.contents.records.iter().enumerate() {
                if r.kind != kind::APK_KEY || r.text(field::APK_PACKAGE) != Some(package) {
                    continue;
                }
                if let Some(s) = shown(r, self.tz_min) {
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
            for (at, r) in open.contents.records.iter().enumerate() {
                if r.kind == kind::APK_KEY && fingerprint_of(r).as_ref() == Some(fp) {
                    return Some((v, at));
                }
            }
        }
        None
    }

    /// How many writes a key made this session has had (0 or 1), when it
    /// still waits for two; `None` when it signs.
    pub fn apk_waiting(&self, fp: &[u8; 32]) -> Option<usize> {
        self.vaults
            .apk
            .waits
            .iter()
            .find(|w| w.fp == *fp)
            .map(|w| usize::from(w.first.is_some()))
    }

    /// Why a visit started for the second copy must not write now, if it
    /// must not: the stick has this vault already.
    pub(crate) fn apk_second_refusal(&self) -> Option<&'static str> {
        if self.vaults.apk.visit_for != Some(2) {
            return None;
        }
        self.vaults
            .apk
            .waits
            .iter()
            .find_map(|w| w.first.as_ref())?;
        let stick = self.sticks.get(self.visit.stick)?;
        if self
            .vaults
            .apk
            .probe
            .as_ref()
            .is_some_and(|p| p.0 == stick.id)
        {
            return Some(EN.apk_reading_stick);
        }
        if self.vaults.apk.same_on.as_deref() == Some(stick.id.as_str()) {
            return Some(EN.apk_same_stick);
        }
        None
    }

    /// A stick came for the second copy: its file of the first write's
    /// name, if it has one, is read back to be compared byte for byte
    /// (SHA-256) with what the first write wrote. An older copy of this
    /// vault under the same name is no reason to refuse.
    pub(crate) fn apk_probe_start(&mut self) {
        self.vaults.apk.probe = None;
        self.vaults.apk.same_on = None;
        if self.vaults.apk.visit_for != Some(2) {
            return;
        }
        let Some(first) = self.vaults.apk.waits.iter().find_map(|w| w.first.clone()) else {
            return;
        };
        let Some(stick) = self.sticks.get(self.visit.stick) else {
            return;
        };
        let Some((name, _)) = stick
            .files
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&first.name))
        else {
            return;
        };
        let (id, name) = (stick.id.clone(), name.clone());
        self.vaults.apk.probe = Some((id.clone(), name.clone()));
        self.storage_out
            .push_back(crate::StorageCommand::Read { stick: id, name });
    }

    /// Whether a file read back is the probe's.
    pub(crate) fn apk_probing(&self, stick: &str, name: &str) -> bool {
        self.vaults
            .apk
            .probe
            .as_ref()
            .is_some_and(|p| p.0 == stick && p.1 == name)
    }

    /// The probe's file came back (`bytes`), or could not be read (`None`).
    pub(crate) fn apk_probe_answer(&mut self, bytes: Option<&[u8]>) {
        let Some((stick, _)) = self.vaults.apk.probe.take() else {
            return;
        };
        let first = self.vaults.apk.waits.iter().find_map(|w| w.first.as_ref());
        if let (Some(b), Some(f)) = (bytes, first)
            && sha_of(b) == f.hash
        {
            self.vaults.apk.same_on = Some(stick);
        }
    }

    /// A stick had the first copy deleted or was emptied: the key goes
    /// back to its first step.
    pub(crate) fn apk_first_gone(&mut self, name: Option<&str>) {
        let mut reset = false;
        for w in &mut self.vaults.apk.waits {
            if w.first
                .as_ref()
                .is_some_and(|f| name.is_none_or(|n| n.eq_ignore_ascii_case(&f.name)))
            {
                w.first = None;
                reset = true;
            }
        }
        if reset {
            self.toast(EN.apk_back_to_first);
        }
    }

    /// A vault in Files that this session unlocked or wrote: its salt is
    /// an open vault's, or its bytes are what a write of this session or
    /// of a waiting key's sealings put on a stick. A copy of any other
    /// vault never goes over a vault of the same size on a stick.
    pub(crate) fn vault_is_this_sessions(&self, bytes: &[u8]) -> bool {
        let Ok(h) = faraday_vault::read_header(bytes) else {
            return false;
        };
        if self.vaults.open.iter().any(|v| v.header().salt == h.salt) {
            return true;
        }
        let hash = sha_of(bytes);
        self.receipt.as_ref().is_some_and(|r| r.holds_hash(&hash))
            || self.vaults.apk.waits.iter().any(|w| {
                w.sealings.contains(&hash) || w.first.as_ref().is_some_and(|f| f.hash == hash)
            })
    }

    /// A sealing of a vault was made: the keys waiting that it holds.
    pub(crate) fn apk_sealed(&mut self, v: usize, bytes: &[u8]) {
        if self.vaults.apk.waits.is_empty() {
            return;
        }
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let held: Vec<[u8; 32]> = open
            .contents
            .of(kind::APK_KEY)
            .filter_map(|(_, r)| fingerprint_of(r))
            .filter(|fp| self.vaults.apk.waits.iter().any(|w| w.fp == *fp))
            .collect();
        if held.is_empty() {
            return;
        }
        let hash = sha_of(bytes);
        for w in self
            .vaults
            .apk
            .waits
            .iter_mut()
            .filter(|w| held.contains(&w.fp))
        {
            if !w.sealings.contains(&hash) {
                w.sealings.push(hash);
                if w.sealings.len() > SEALINGS_KEPT {
                    w.sealings.remove(0);
                }
            }
        }
    }

    /// The vault file in Files that a visit started for the second copy
    /// has ticked: the one the first write put on the first stick. On no
    /// other visit is the vault ticked from Files.
    pub(crate) fn apk_inbox_ticks(&self) -> std::collections::BTreeSet<String> {
        if self.vaults.apk.visit_for != Some(2) {
            return Default::default();
        }
        let Some(first) = self.vaults.apk.waits.iter().find_map(|w| w.first.as_ref()) else {
            return Default::default();
        };
        self.inbox
            .iter()
            .filter(|i| i.kind == crate::FileKind::Vault)
            .filter(|i| !self.outbox.iter().any(|o| o.name == i.name))
            .filter(|i| sha_of(&i.bytes) == first.hash)
            .map(|i| i.name.clone())
            .collect()
    }

    /// A write was answered, read back and matched. On a visit started
    /// for the first copy, a sealing that holds a waiting key makes the
    /// first write; on one started for the second, the second, and the
    /// key signs from now on.
    pub(crate) fn apk_written(&mut self, name: &str, hash: &[u8; 32]) {
        let len = self
            .inbox
            .iter()
            .chain(&self.outbox)
            .find(|i| i.name == name && sha_of(&i.bytes) == *hash)
            .map(|i| i.bytes.len());
        let visit = self.vaults.apk.visit_for;
        let mut released = false;
        for w in &mut self.vaults.apk.waits {
            if !w.sealings.contains(hash) {
                continue;
            }
            match (visit, &w.first) {
                (Some(1), None) => {
                    w.first = Some(First {
                        name: name.to_string(),
                        len: len.unwrap_or(0),
                        hash: *hash,
                    });
                }
                (Some(2), Some(_)) => released = true,
                _ => {}
            }
        }
        if released {
            self.vaults
                .apk
                .waits
                .retain(|w| !(w.first.is_some() && w.sealings.contains(hash)));
            self.vaults.apk.visit_for = None;
            self.toast(EN.apk_signs_from_now);
        } else if visit == Some(1) && self.vaults.apk.waits.iter().any(|w| w.first.is_some()) {
            self.vaults.apk.visit_for = None;
        }
    }

    /// Makes the key the Make form asked for, into its vault. Runs a frame
    /// after the form, so "Making the key" is on the screen.
    pub(crate) fn apk_make_work(&mut self) {
        let Some((name, package, v)) = self.vaults.apk.making.take() else {
            return;
        };
        let (Some(now), Some(seed)) = (
            self.clock_now(),
            self.fresh(b"apk key").map(zeroize::Zeroizing::new),
        ) else {
            self.apk_make_failed(&name, &package, v, EN.apk_no_randomness);
            return;
        };
        let pair = match key::make(&seed, &name, now) {
            Ok(p) => p,
            Err(e) => {
                self.apk_make_failed(&name, &package, v, e.reason());
                return;
            }
        };
        let fp = key::fingerprint(&pair.cert);
        if !self.vault_push_in(v, record_of(&pair, &package), EN.apk_the_key_is_in) {
            self.apk_make_failed(&name, &package, v, EN.apk_that_does_not_fit);
            return;
        }
        self.vaults.apk.waits.push(Wait {
            fp,
            sealings: Vec::new(),
            first: None,
        });
        let stem = file_stem(&name, &fp);
        self.put_outbox(
            &format!("{stem}-certificate.pem"),
            pem::encode(&pair.cert).into_bytes(),
        );
        self.vaults.form = None;
        self.vaults.focus = None;
        self.vaults.current = v;
        self.vaults.category = 6;
        self.vaults.category_chosen = true;
        let n = self
            .vaults
            .open
            .get(v)
            .map_or(0, |o| o.contents.of(kind::APK_KEY).count());
        self.vaults.item[6] = n.saturating_sub(1);
        self.vaults.apk.leave();
        self.vaults.apk.made = Some(fp);
        self.vaults.item_open = true;
    }

    /// The form comes back with what was typed, and why nothing was made.
    fn apk_make_failed(&mut self, name: &str, package: &str, v: usize, why: &str) {
        self.vault_form_apk(name, package);
        self.vaults.apk.vault_pick = Some(v);
        self.toast(why);
    }

    /// The Make an APK key form, with these in its fields.
    pub(crate) fn vault_form_apk(&mut self, name: &str, package: &str) {
        let (mut n, mut p) = (TextBox::default(), TextBox::default());
        n.set(name);
        p.set(package);
        self.vaults.apk.leave();
        self.vaults.apk.vault_pick = None;
        self.vaults.form = Some(crate::vaults::Form {
            kind: kind::APK_KEY,
            record: None,
            fields: vec![
                (1, EN.apk_name_on_certificate, n, false),
                (2, EN.apk_package, p, false),
            ],
        });
        self.vaults.saving = false;
        self.vaults.focus = Some(crate::vaults::Focus::Field(0));
    }

    /// Make the key: the name and the package are checked, the vault is
    /// the one open or the one chosen, and the work waits for a frame
    /// that says it is under way.
    pub(crate) fn vault_form_apk_save(&mut self, form: crate::vaults::Form) {
        let name = form.fields[0].2.text.to_string();
        let package = form.fields[1].2.text.to_string();
        let checked = Self::apk_check_form(&name, &package);
        let v = match (self.vaults.open.len(), self.vaults.apk.vault_pick) {
            (0, _) => None,
            (1, _) => Some(0),
            (_, pick) => pick,
        };
        let (name, package) = match (checked, v) {
            (Err(e), _) => {
                self.vaults.form = Some(form);
                return self.toast(e);
            }
            (Ok(_), None) => {
                self.vaults.form = Some(form);
                return self.toast(EN.apk_choose_the_vault);
            }
            (Ok(c), Some(_)) => c,
        };
        if self.clock_now().is_none() {
            self.vaults.form = Some(form);
            return self.toast(EN.apk_this_computer);
        }
        self.vaults.form = Some(form);
        self.vaults.apk.making = Some((name, package, v.unwrap_or(0)));
        self.vaults.working = Some(crate::vaults::Work::ApkKey);
        self.vaults.drawn = false;
    }

    /// What an APK's row in Files offers (`docs/APK-SIGNING.md` §4.1): to
    /// sign, when an open vault holds a key for its package; a note of why
    /// not otherwise; and Unlock or Make a vault, where none is open.
    pub fn apk_files_row(&self, k: usize) -> Row {
        let Some(item) = self.inbox.get(k) else {
            return Row::None;
        };
        let info = match self.apk_read(item) {
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
        Row::Sign
    }

    /// Selects a key by its place for the contents to show.
    fn apk_select(&mut self, v: usize, at: usize) {
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let n = open.contents.records[..at.min(open.contents.records.len())]
            .iter()
            .filter(|r| r.kind == kind::APK_KEY)
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
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let name = item.name.clone();
        let Ok(info) = self.apk_read(item) else {
            return;
        };
        let keys = self.apk_keys_for(&info.package);
        let Some(&(v, at, _, fp)) = keys.first() else {
            return;
        };
        self.apk_select(v, at);
        self.vaults.apk.leave();
        self.vaults.apk.signing = true;
        if keys.len() == 1 {
            if self.apk_waiting(&fp).is_none() {
                self.apk_open_review(&name, fp);
            }
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
            .and_then(|o| o.contents.records.get(at))
            .and_then(crate::apk::fingerprint_of)
        else {
            return;
        };
        self.apk_select(v, at);
        self.vaults.apk.leave();
        self.vaults.apk.signing = true;
        if self.apk_waiting(&fp).is_none() {
            self.apk_open_review(&name, fp);
        }
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
        let (Some(cert), Some(fp)) = (r.field(field::APK_CERT), fingerprint_of(&r)) else {
            return;
        };
        let Some(s) = shown(&r, self.tz_min) else {
            return;
        };
        let name = format!("{}-certificate.pem", file_stem(&s.name, &fp));
        self.put_outbox(&name, pem::encode(cert).into_bytes());
        self.toast_out(&name);
    }

    /// The QR view of the selected key's certificate, with the name and
    /// label of its picture.
    pub(crate) fn apk_code(&self) -> Result<crate::QrView, String> {
        let (_, r) = self
            .apk_selected()
            .ok_or_else(|| EN.apk_no_key_is_selected.to_string())?;
        let cert = r
            .field(field::APK_CERT)
            .ok_or_else(|| EN.apk_no_key_is_selected.to_string())?;
        let fp = fingerprint_of(&r).unwrap_or_default();
        let s = shown(&r, self.tz_min).ok_or_else(|| EN.apk_no_key_is_selected.to_string())?;
        let stem = file_stem(&s.name, &fp);
        let lines = vec![
            s.name.clone(),
            fill(EN.apk_sha_256_of, &[&s.fingerprint]),
            EN.apk_public_checks.to_string(),
        ];
        Ok(crate::QrView::text(EN.apk_certificate, &pem::encode(cert))?
            .public(&format!("{stem}.png"), lines))
    }

    /// Opens the review of Inbox APK `name` for the key `fp`.
    pub(crate) fn apk_open_review(&mut self, name: &str, fp: [u8; 32]) {
        let Some(item) = self.inbox.iter().find(|i| i.name == name) else {
            return;
        };
        let sha = sha_of(&item.bytes);
        self.vaults.apk.review = Some(Review {
            name: name.to_string(),
            key: fp,
            sha,
        });
        self.vaults.apk.second.clear();
        self.vaults.apk.which = None;
        self.vaults.focus = Some(crate::vaults::Focus::Second);
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
        let Some(review) = self.vaults.apk.review.clone() else {
            return;
        };
        if !matches!(self.apk_second(), Second::Matches(_)) {
            return;
        }
        if self.apk_waiting(&review.key).is_some() {
            return;
        }
        let Some((v, at)) = self.apk_find(&review.key) else {
            return;
        };
        let record = self.vaults.open[v].contents.records[at].clone();
        let (Some(pkcs8), Some(cert), Some(package)) = (
            record.field(field::APK_KEY),
            record.field(field::APK_CERT),
            record.text(field::APK_PACKAGE),
        ) else {
            return;
        };
        let Some(seed) = self.fresh(b"apk signing").map(zeroize::Zeroizing::new) else {
            return self.toast(EN.apk_no_randomness);
        };
        let Some(item) = self.inbox.iter().find(|i| i.name == review.name) else {
            return;
        };
        // The bytes signed are the bytes reviewed.
        if sha_of(&item.bytes) != review.sha {
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
        let mut blind = faraday_sb::rng::Stream::new(&seed, 0);
        let signed = match sign::sign(&mut blind, &mut src, &apk, pkcs8, cert)
            .and_then(|s| sign::assemble(&mut src, &apk, &s))
        {
            Ok(bytes) => bytes,
            Err(e) => return self.toast(e.reason()),
        };
        self.put_outbox(&out_name, signed);
        self.vaults.apk.review = None;
        self.vaults.apk.second.clear();
        self.vaults.focus = None;
        self.vaults.apk.signing = true;
        self.toast_out(&out_name);
    }

    /// A vault was unlocked: the key page a lock for the stick came from,
    /// if this vault holds it.
    pub(crate) fn apk_return(&mut self) {
        let Some(fp) = self.vaults.apk.back else {
            return;
        };
        if let Some((v, at)) = self.apk_find(&fp) {
            self.vaults.apk.back = None;
            self.apk_select(v, at);
        }
    }
}
