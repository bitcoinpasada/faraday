//! The values the app reads of a vault that are text, and the names its
//! keys are written under. A secret is held here only in a wiping type
//! (`GpgKey`'s secret key and revocation certificate are `Zeroizing`); the
//! other values are plain heap text, which `tools/lint-secrets.sh` refuses
//! in `mod.rs`, `write.rs` and `forms.rs` and allows in this file. The
//! secret values of an item stay in the wiping buffers of those three.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use zeroize::Zeroizing;

use super::{Body, Entry};
use crate::records::Record;
use crate::totp::{NotTotp, Totp};

/// A key or a name, as text.
pub(super) type Name = String;
/// The names of the groups an entry is in.
///
/// Shared: every entry of a group holds the group's one list, and every
/// name is stored once, however many groups below it and entries in them.
pub(super) type Path = Arc<[Arc<str>]>;
/// `faraday.sealed`, when it reads.
pub(super) type Sealed = Option<String>;

pub(super) fn title_of(e: &Entry) -> String {
    e.field("Title")
        .map_or_else(String::new, |f| f.value().into())
}

/// `faraday.plan.{n}`.
#[expect(
    clippy::disallowed_methods,
    reason = "a number made into text: no secret in it"
)]
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

/// `faraday.nonces.{tx}` from the transaction's hex.
pub(super) fn nonces_key(tx_hex: &[u8]) -> String {
    let mut key = String::from(super::NONCES);
    key.push_str(core::str::from_utf8(tx_hex).unwrap_or(""));
    key
}

/// A nonce record's value: each entry's three 32-byte parts as
/// `{key}:{r}:{msg}` in lower-case hex, entries comma separated.
pub(super) fn nonces_value<'a>(entries: impl Iterator<Item = &'a [u8]>) -> String {
    let mut value = String::new();
    for e in entries {
        if !value.is_empty() {
            value.push(',');
        }
        for (i, part) in e.chunks(32).enumerate() {
            if i > 0 {
                value.push(':');
            }
            value.push_str(core::str::from_utf8(&super::forms::hex(part)).unwrap_or(""));
        }
    }
    value
}

/// A signed amounts value: the inputs' amounts, comma separated.
#[expect(
    clippy::disallowed_methods,
    reason = "an amount made into text: no secret in it"
)]
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
#[expect(
    clippy::disallowed_methods,
    reason = "a number made into text: no secret in it"
)]
pub(super) fn number(n: u64) -> String {
    n.to_string()
}

/// What `faraday.format` says.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
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
    /// A wallet's text that no wallet rule reads, with the refusal a fresh
    /// addition of the same text would give.
    Wallet(faraday_bip::policy::Error),
}

/// Where an item is damaged: the field or the key, and the fault.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
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
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
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
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
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
    pub path: Arc<[Arc<str>]>,
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
    /// Set when other items hold a codex32 share of the same threshold,
    /// identifier, index and key: its place among them. `None` for every
    /// other item, and for one that is alone.
    pub related: Option<Related>,
}

/// A codex32 share's place among the shares of one threshold, identifier,
/// index and key, in the order read (a group's entries, then its groups).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Related {
    /// This item's place in the group, from 1.
    pub place: usize,
    /// How many items the group holds, at least 2.
    pub of: usize,
}

/// One value of the database's custom data that reads: a backup plan
/// (type 11), a transaction's signed amounts (type 10) or its nonce
/// record (type 17).
pub struct Custom {
    /// Its key (`faraday.plan.2`, `faraday.signed.{tx}`, `faraday.nonces.{tx}`).
    pub key: String,
    /// The record.
    pub record: Record,
}

/// What a record holds, its fields at their capacity: a count, no copy.
fn record_held(r: &Record) -> usize {
    size_of::<Record>()
        + r.fields.capacity() * size_of::<crate::records::Field>()
        + r.fields
            .iter()
            .map(|f| f.bytes.capacity() + 16)
            .sum::<usize>()
}

impl Read {
    /// Costs a walk of the items, no copy. An upper bound on the bytes this
    /// read holds, and the app's records built from it hold again: its
    /// items' records and titles and its custom values, each with an
    /// allocation's overhead.
    pub fn held(&self) -> usize {
        // A group's entries share one path, and every path shares the
        // names: each path and each name once.
        let mut paths: BTreeSet<*const ()> = BTreeSet::new();
        let mut names: BTreeSet<*const u8> = BTreeSet::new();
        let mut pathed = 0usize;
        for it in &self.items {
            if paths.insert(Arc::as_ptr(&it.path).cast()) {
                pathed += size_of::<Arc<[Arc<str>]>>() + 16 + size_of_val(&*it.path);
                for n in it.path.iter() {
                    if names.insert(Arc::as_ptr(n).cast()) {
                        pathed += size_of::<Arc<str>>() + n.len() + 32;
                    }
                }
            }
        }
        let items: usize = pathed
            + self
                .items
                .iter()
                .map(|it| {
                    let body = match &it.body {
                        Body::Record(r) => record_held(r),
                        Body::Gpg(g) => {
                            size_of::<GpgKey>()
                                + g.title.capacity()
                                + g.key.capacity()
                                + g.public.as_ref().map_or(0, String::capacity)
                                + g.revocation.as_ref().map_or(0, |r| r.capacity())
                                + g.revoked.as_ref().map_or(0, String::capacity)
                                + 80
                        }
                    };
                    let extra: usize = it
                        .extra
                        .iter()
                        .map(|e| size_of::<Extra>() + e.name.capacity() + 16)
                        .sum();
                    let totp = match &it.totp {
                        Some(Ok(t)) => t.held(),
                        _ => 0,
                    };
                    32 + body + extra + totp
                })
                .sum::<usize>();
        let custom: usize = self
            .custom
            .iter()
            .map(|c| c.key.capacity() + record_held(&c.record))
            .sum();
        let unreadable: usize = self
            .unreadable
            .iter()
            .map(|u| u.title.capacity() + 256)
            .sum();
        // The lists are held at their capacity, which a list that grew by
        // doubling takes beyond its length.
        let lists = self.items.capacity() * size_of::<Item>()
            + self.custom.capacity() * size_of::<Custom>()
            + self.unreadable.capacity() * size_of::<Unreadable>();
        let own = self.label.capacity() + self.sealed.as_ref().map_or(0, String::capacity);
        items + custom + unreadable + lists + own
    }
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
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
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
    let (year, month, day) = faraday_crypto::date::civil_from_days(days);
    let (month, day) = (i64::from(month), i64::from(day));
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
    #[expect(
        clippy::disallowed_methods,
        reason = "a file format version number: no secret in it"
    )]
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
