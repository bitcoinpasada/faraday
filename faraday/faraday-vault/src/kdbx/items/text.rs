//! The values the app reads of a vault that are text, and the names its
//! keys are written under. A secret is held here only in a wiping type
//! (`GpgKey`'s secret key and revocation certificate are `Zeroizing`); the
//! other values are plain heap text, which `tools/lint-secrets.sh` refuses
//! in `mod.rs`, `write.rs` and `forms.rs` and allows in this file. The
//! secret values of an item stay in the wiping buffers of those three.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use zeroize::Zeroizing;

use super::{Body, Entry};
use crate::records::Record;
use crate::totp::{NotTotp, Totp};

/// A key or a name, as text.
pub(super) type Name = String;
/// The names of the groups an entry is in.
pub(super) type Path = Vec<String>;
/// `faraday.sealed`, when it reads.
pub(super) type Sealed = Option<String>;

pub(super) fn title_of(e: &Entry) -> String {
    e.field("Title")
        .map_or_else(String::new, |f| f.value().into())
}

/// `faraday.plan.{n}`.
pub(super) fn plan_key(n: u32) -> String {
    let mut key = String::from(super::PLAN);
    key.push_str(&n.to_string());
    key
}

/// `faraday.signed.{tx}` from the transaction's hex.
pub(super) fn signed_key(tx_hex: &[u8]) -> String {
    let mut key = String::from(super::SIGNED);
    key.push_str(core::str::from_utf8(tx_hex).unwrap_or(""));
    key
}

/// A signed amounts value: the inputs' amounts, comma separated.
pub(super) fn signed_value(amounts: impl Iterator<Item = u64>) -> String {
    let mut value = String::new();
    for a in amounts {
        if !value.is_empty() {
            value.push(',');
        }
        value.push_str(&a.to_string());
    }
    value
}

/// A number as text.
pub(super) fn number(n: u64) -> String {
    n.to_string()
}

/// What `faraday.format` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    /// `1`: a vault Faraday made.
    Faraday,
    /// Absent: a KeePass file not made by Faraday.
    Foreign,
    /// Any other value, kept as read: a newer format, or damage; or, with
    /// the reason as its text, a `Meta` or custom data that cannot be told
    /// apart from what it was read as. Faraday's items are not read and the
    /// vault is read-only.
    Newer(String),
}

/// What was wrong with an item or a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// A `faraday.kind` §7 does not name.
    UnknownKind,
    /// A field, or a `faraday.` key, the kind does not name.
    UnknownField,
    /// A required field is missing.
    Missing,
    /// A value not in its published form.
    Value,
    /// Two of what may be one: both of two exclusive fields, a key
    /// written twice.
    Twice,
    /// `faraday.format` is not one this build reads.
    Format,
    /// Another entry has the same UUID.
    SameUuid,
    /// The entry has two fields of this key.
    SameField,
    /// The document cannot be told apart from what it was read as (two
    /// UUIDs, two names, a field with two values, ...): the damage's `at`
    /// is the reason as the document model words it.
    Ambiguous,
}

/// Where an item is damaged: the field or the key, and the fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    /// The field (`Password`) or custom data key (`faraday.child`); for the
    /// database's custom data, the key.
    pub at: String,
    /// What was wrong.
    pub fault: Fault,
}

pub(super) fn damage(at: &str, fault: Fault) -> Damage {
    Damage {
        at: at.chars().take(64).collect(),
        fault,
    }
}

/// An item or a key that does not read. It is never a record, and its entry
/// or key is written back unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// The entry's title, or the database key.
    pub title: String,
    /// The entry's UUID; `None` for the database's custom data.
    pub uuid: Option<[u8; 16]>,
    /// Where, and what.
    pub what: Damage,
}

/// A GPG key's fields (§7, `gpg`), armored, as the vault holds them.
pub struct GpgKey {
    /// The primary user ID, the entry's title.
    pub title: String,
    /// The armored secret key, unprotected.
    pub key: Zeroizing<String>,
    /// The public certificate as last exported.
    pub public: Option<String>,
    /// The revocation certificate made with the key. Secret (F32).
    pub revocation: Option<Zeroizing<String>>,
    /// When the key was marked revoked, an RFC 3339 date-time.
    pub revoked: Option<String>,
}

/// A field of a person's entry beyond the six Faraday reads: kept in the
/// document, shown on the entry's page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extra {
    /// The field's key.
    pub name: String,
    /// Whether its value is protected.
    pub protected: bool,
}

/// One entry that reads.
pub struct Item {
    /// The entry's UUID.
    pub uuid: [u8; 16],
    /// The names of the groups it is in, from under the root; empty for the
    /// root group.
    pub path: Vec<String>,
    /// What it holds.
    pub body: Body,
    /// A person's entry's other fields; empty for Faraday's items.
    pub extra: Vec<Extra>,
    /// A person's entry's TOTP as [`crate::totp::from_fields`] reads it over all
    /// its fields (`otp`, KeePass 2's `TimeOtp-*`, older KeePassXC's `TOTP
    /// Seed`): `None` when it holds none; an `Err` is the badge (HOTP,
    /// unreadable), not a damage of the entry. Always `None` for Faraday's
    /// items.
    pub totp: Option<Result<Totp, NotTotp>>,
    /// Set when other items hold a passphrase for the same key, or a codex32
    /// share of the same index and key: its place among them. `None` for
    /// every other item, and for one that is alone.
    pub related: Option<Related>,
}

/// What a group of related secrets is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelatedKind {
    /// Passphrases for one key.
    Passphrases,
    /// Codex32 shares of one index and key.
    Shares,
}

/// An item's place among the items that hold a related secret (several
/// passphrases for one key, several shares of one index and key), in
/// the order read (a group's entries, then its groups): the app labels each, such as "1 of 2 passphrases for key
/// {fingerprint}".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Related {
    /// What the group is.
    pub kind: RelatedKind,
    /// This item's place in the group, from 1.
    pub place: usize,
    /// How many items the group holds, at least 2.
    pub of: usize,
}

/// One value of the database's custom data that reads: a backup plan
/// (type 11) or a transaction's signed amounts (type 10).
pub struct Custom {
    /// Its key (`faraday.plan.2`, `faraday.signed.{tx}`).
    pub key: String,
    /// The record.
    pub record: Record,
}

/// What the document holds for the app.
pub struct Read {
    /// `faraday.format`.
    pub format: Format,
    /// `faraday.seals`: 0 when absent or malformed, so the next seal writes 1.
    pub seals: u64,
    /// `faraday.sealed`, when it reads.
    pub sealed: Option<String>,
    /// `Meta/DatabaseName`.
    pub label: String,
    /// The entries that read, in document order.
    pub items: Vec<Item>,
    /// The plans and signed amounts that read.
    pub custom: Vec<Custom>,
    /// What did not read.
    pub unreadable: Vec<Unreadable>,
    /// How many entries have history, kept in the document.
    pub history: usize,
    /// How many entries were in the recycle bin, kept in the document.
    pub recycle_bin: usize,
    /// Why the database's `Meta` or its custom data cannot be told apart
    /// from what it was read as, if it cannot.
    pub meta_ambiguous: Option<&'static str>,
}

/// Why an open vault is not changed or sealed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOnly {
    /// `faraday.format` states a format newer than this build writes (the
    /// value as read).
    Newer(String),
    /// The database's `Meta` or custom data cannot be told apart from
    /// what it was read as: the reason, as the document model words it.
    Ambiguous(&'static str),
}

/// `faraday.sealed`'s form from Unix seconds: RFC 3339 in UTC
/// (`2026-10-28T14:02:00Z`); `None` for a year past 9999.
pub fn utc_time(unix: u64) -> Option<String> {
    let days = i64::try_from(unix / 86_400).ok()?;
    let rest = unix % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    if !(0..=9999).contains(&year) {
        return None;
    }
    let mut out = String::with_capacity(20);
    let two = |out: &mut String, n: i64| {
        out.push(char::from(b'0' + (n / 10) as u8));
        out.push(char::from(b'0' + (n % 10) as u8));
    };
    two(&mut out, year / 100);
    two(&mut out, year % 100);
    out.push('-');
    two(&mut out, month);
    out.push('-');
    two(&mut out, day);
    out.push('T');
    two(&mut out, (rest / 3_600) as i64);
    out.push(':');
    two(&mut out, (rest % 3_600 / 60) as i64);
    out.push(':');
    two(&mut out, (rest % 60) as i64);
    out.push('Z');
    Some(out)
}

impl Read {
    /// Why the vault may not be changed, if it may not
    /// ([`Read::read_only`]): an ambiguous `Meta` before a newer format.
    pub fn why_read_only(&self) -> Option<ReadOnly> {
        if let Some(why) = self.meta_ambiguous {
            return Some(ReadOnly::Ambiguous(why));
        }
        match &self.format {
            Format::Newer(f) => Some(ReadOnly::Newer(f.clone())),
            _ => None,
        }
    }

    /// Whether the vault may be changed: not when its format is newer or
    /// its `Meta` is ambiguous.
    pub fn read_only(&self) -> bool {
        matches!(self.format, Format::Newer(_)) || self.meta_ambiguous.is_some()
    }

    /// The seal count the next seal writes.
    pub fn next_seal(&self) -> u64 {
        self.seals.saturating_add(1)
    }
}
