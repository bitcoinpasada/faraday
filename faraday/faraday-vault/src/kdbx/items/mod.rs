//! Faraday's items in a KDBX document (`docs/VAULT.md` §7): what the app
//! reads of a [`Document`], and the changes it makes applied back to it.
//!
//! **The shape.** The [`Document`] stays the source of truth: [`read`] gives
//! a view of it, and every change goes back into it ([`put`], [`remove`],
//! [`put_plan`], [`put_signed`], [`remove_custom`], [`stamp`]), entry by
//! UUID, so what the app does not change is written back as read (the
//! document's own carry-through). The view gives the app its records
//! ([`Record`], by kind number), each with the entry's UUID and its group
//! path beside it: a record is turned into an entry's fields by its
//! published text form, and back. One kind is not a record: a GPG key
//! ([`Body::Gpg`]), whose record holds the keys' seeds where the vault
//! holds the armored secret key, and turning one into the other needs an
//! OpenPGP secret-key reader not written yet; its fields are given as
//! text.
//!
//! **Nothing about Faraday's items refuses the file** (owner, 2026-11-03).
//! An item that does not read (an unknown `faraday.kind`, a field its kind
//! does not name, a required field missing, a value not in its published
//! form, a `faraday.codex32` or `faraday.child` that is not its TOML) is an
//! [`Unreadable`], naming the entry by title and UUID and the field or key
//! and fault; it is never made a record, and its entry is written back as
//! read. So is an entry whose UUID another entry shares, or one with two
//! fields of one key. Database custom data that does not read
//! (`faraday.plan.{n}`, `faraday.signed.{tx}`, an unknown `faraday.` key) is
//! kept unchanged and named the same way, with no UUID.
//!
//! **The vault's own values.** `faraday.format` missing is a KeePass file
//! not made by Faraday ([`Format::Foreign`]): its entries open, its seal
//! count reads 0 and the next seal writes 1. Another value than `1`
//! ([`Format::Newer`]) makes every Faraday item Unreadable and the vault
//! read-only, as does a `Meta` or database custom data that cannot be told
//! apart from what it was read as (two `faraday.format` items among them):
//! it counts as a newer format. A malformed `faraday.seals` is named and
//! reads 0.
//!
//! **A person's entry** (no `faraday.kind`) is an entry record (type 6):
//! `Title`, `UserName`, `Password`, `URL`, `Notes` and `otp`. Its TOTP is
//! read by [`totp::from_fields`] over all its fields ([`Item::totp`]): an
//! `otp` that does not read, or HOTP, is a badge, and the entry reads. Its
//! other fields stay in the document and are listed by name and protection
//! ([`Item::extra`]). The entry is never Unreadable for a field's length:
//! the record holds a field up to 65,535 bytes, and a standard field longer
//! than that (up to the vault's largest size) is left out of the record and
//! listed as an extra, kept as it is; a change to the entry leaves it
//! unless the record gives that field a value.
//!
//! The values the app checks are read by the parsers it already has:
//! BIP-39 words by `faraday-bip`, a policy and a descriptor by
//! `faraday-bip`'s wallet policy and descriptor readers, a codex32 string
//! by its reader, the TOML values by `contents`, PEM by `pem-rfc7468`.

mod forms;
mod text;
mod write;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use faraday_bip::bip39::Language;
use faraday_bip::descriptor::reads_as_descriptor;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use super::xml::{Document, Entry, Group};
use crate::records::{self, BACKUP_PENDING, NO_LOAD_AT_UNLOCK, Record, field, kind};
use crate::totp::{self, NotTotp, Totp};
use crate::{child, codex32, contents};

pub use text::{
    Custom, Damage, Extra, Fault, Format, GpgKey, Item, Read, ReadOnly, Related, Unreadable,
    utc_time,
};
use text::{Name, Path, Sealed, damage, title_of};
pub use write::{
    Place, Unwritable, check, put, put_known, put_nonces, put_plan, put_signed, remove,
    remove_custom, remove_known, stamp,
};

/// The `faraday.format` this build reads and writes.
pub const FORMAT: &str = "1";

/// What an entry holds for the app.
pub enum Body {
    /// A record of its kind (§7's table): a key, a wallet, a note, a
    /// sheet, Secure Boot keys, a wallet share, a passphrase, a codex32
    /// share, or a person's entry (type 6).
    Record(Record),
    /// A GPG key.
    Gpg(GpgKey),
}

impl Body {
    /// The record type this is.
    pub fn kind(&self) -> u8 {
        match self {
            Body::Record(r) => r.kind,
            Body::Gpg(_) => kind::GPG,
        }
    }
}

// ------------------------------------------------------------- the kinds

/// `faraday.kind` of each record type that is an entry.
const KINDS: [(&str, u8); 9] = [
    ("seed", kind::KEY),
    ("wallet", kind::WALLET),
    ("note", kind::NOTE),
    ("sheet", kind::SHEET),
    ("gpg", kind::GPG),
    ("secure_boot", kind::SECURE_BOOT),
    ("wallet_share", kind::WALLET_SHARE),
    ("passphrase", kind::PASSPHRASE),
    ("share", kind::CODEX32_SHARE),
];

/// The fields each kind names; Title is every kind's.
fn named(k: u8) -> &'static [&'static str] {
    match k {
        kind::KEY => &["Title", "Password", "hex", "list", "passphrase"],
        kind::WALLET => &["Title", "policy", "descriptor"],
        kind::NOTE => &["Title", "Notes"],
        kind::SHEET => &["Title", "descriptor", "Notes"],
        kind::GPG => &["Title", "key", "public", "revocation", "revoked"],
        kind::SECURE_BOOT => &[
            "Title", "owner", "pk_key", "kek_key", "db_key", "pk_cert", "kek_cert", "db_cert",
        ],
        kind::WALLET_SHARE => &["Title", "Password", "index", "of", "wallet"],
        kind::PASSPHRASE | kind::CODEX32_SHARE => &["Title", "Password", "for"],
        _ => &STANDARD,
    }
}

/// The entry custom data keys each kind names, beside `faraday.kind`.
fn named_custom(k: u8) -> &'static [&'static str] {
    match k {
        kind::KEY => &[
            "faraday.load",
            "faraday.held",
            "faraday.codex32",
            "faraday.child",
        ],
        kind::WALLET => &["faraday.load"],
        _ => &[],
    }
}

/// A person's entry's fields that Faraday reads.
const STANDARD: [&str; 6] = ["Title", "UserName", "Password", "URL", "Notes", "otp"];
const ENTRY_FIELDS: [u8; 6] = [
    field::TITLE,
    field::USERNAME,
    field::PASSWORD,
    field::URL,
    field::NOTES,
    field::TOTP,
];

/// The fields KeePass shows by default: an item's must be empty where its
/// kind does not name them.
const KEEPASS: [&str; 5] = ["Title", "UserName", "Password", "URL", "Notes"];

const PEM_FIELDS: [(&str, u8, &str); 6] = [
    ("pk_key", 2, "PRIVATE KEY"),
    ("pk_cert", 3, "CERTIFICATE"),
    ("kek_key", 4, "PRIVATE KEY"),
    ("kek_cert", 5, "CERTIFICATE"),
    ("db_key", 6, "PRIVATE KEY"),
    ("db_cert", 7, "CERTIFICATE"),
];

/// The fields of one entry being read: each taken is marked, and
/// [`Fields::finish`] names the first one left that holds a value.
struct Fields<'a> {
    e: &'a Entry,
    used: Vec<&'static str>,
}

impl<'a> Fields<'a> {
    /// The value, when present and not empty.
    fn get(&mut self, name: &'static str) -> Option<&'a str> {
        self.used.push(name);
        self.e
            .field(name)
            .map(|f| f.value())
            .filter(|v| !v.is_empty())
    }

    fn need(&mut self, name: &'static str) -> Result<&'a str, Damage> {
        self.get(name).ok_or_else(|| damage(name, Fault::Missing))
    }

    fn finish(self, k: u8) -> Result<(), Damage> {
        for f in self.e.fields() {
            let named_here = named(k).contains(&f.key());
            if !self.used.contains(&f.key()) && !f.value().is_empty() && !named_here {
                return Err(damage(f.key(), Fault::UnknownField));
            }
        }
        for (key, _) in self.e.custom().items() {
            if key != "faraday.kind" && !named_custom(k).contains(&key) {
                return Err(damage(key, Fault::UnknownField));
            }
        }
        Ok(())
    }
}

/// Says which field a record's own check refused.
fn record_damage(k: u8, f: records::Fault) -> Damage {
    let n = match f {
        records::Fault::UnknownField(_, n)
        | records::Fault::Missing(_, n)
        | records::Fault::Repeated(_, n)
        | records::Fault::Value(_, n) => n,
        _ => 0,
    };
    let at = match (k, n) {
        (kind::KEY, field::KEY) => "Password",
        (kind::KEY, field::KEY_CODEX32) => "faraday.codex32",
        (kind::KEY, field::KEY_CHILD) => "faraday.child",
        (kind::KEY, field::KEY_PASSPHRASE) => "passphrase",
        (kind::SHEET, _) => "descriptor",
        (kind::CODEX32_SHARE, _) => "Password",
        (kind::ENTRY, _) => STANDARD
            .get(usize::from(n).wrapping_sub(1))
            .copied()
            .unwrap_or("Title"),
        _ => "Title",
    };
    damage(at, Fault::Value)
}

fn checked(r: Record) -> Result<Body, Damage> {
    r.check().map_err(|f| record_damage(r.kind, f))?;
    Ok(Body::Record(r))
}

/// The two values of an exclusive pair: exactly one.
fn one_of<'a>(
    f: &mut Fields<'a>,
    a: &'static str,
    b: &'static str,
) -> Result<(&'static str, &'a str), Damage> {
    match (f.get(a), f.get(b)) {
        (Some(v), None) => Ok((a, v)),
        (None, Some(v)) => Ok((b, v)),
        (None, None) => Err(damage(a, Fault::Missing)),
        (Some(_), Some(_)) => Err(damage(b, Fault::Twice)),
    }
}

/// A `true` or `false` custom data value.
fn flag(e: &Entry, key: &str) -> Result<Option<bool>, Damage> {
    match e.custom().get(key) {
        None => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) => Err(damage(key, Fault::Value)),
    }
}

/// Whether a seed's title is the one written for a key with no label.
fn is_default_title(title: &str) -> bool {
    title
        .strip_prefix("Key ")
        .is_some_and(|fp| forms::fingerprint(fp).is_some())
}

/// Costs a parse of the words: a seed item as its key record.
fn read_seed(c: &Compute, e: &Entry, f: &mut Fields<'_>) -> Result<Body, Damage> {
    let mut r = Record::new(kind::KEY);
    let (which, value) = one_of(f, "Password", "hex")?;
    let payload = if which == "Password" {
        let lang = match f.get("list") {
            None => Language::English,
            Some(name) => forms::list_of(name).ok_or_else(|| damage("list", Fault::Value))?,
        };
        forms::words_payload(c, lang, value).ok_or_else(|| damage("Password", Fault::Value))?
    } else {
        if f.get("list").is_some() {
            return Err(damage("list", Fault::UnknownField));
        }
        forms::seed_payload(value).ok_or_else(|| damage("hex", Fault::Value))?
    };
    r.push(field::KEY, &payload);
    if let Some(t) = f.get("Title").filter(|t| !is_default_title(t)) {
        r.push(field::KEY_LABEL, t.as_bytes());
    }
    let mut flags = 0u8;
    if flag(e, "faraday.load")? == Some(false) {
        flags |= NO_LOAD_AT_UNLOCK;
    }
    if flag(e, "faraday.held")? == Some(true) {
        flags |= BACKUP_PENDING;
    }
    if flags != 0 {
        r.push(field::KEY_FLAGS, &[flags]);
    }
    if let Some(p) = f.get("passphrase") {
        r.push(field::KEY_PASSPHRASE, p.as_bytes());
    }
    if let Some(v) = e.custom().get("faraday.codex32") {
        let facts = contents::read_codex32_facts(v.as_bytes())
            .map_err(|_| damage("faraday.codex32", Fault::Value))?;
        let p =
            codex32::set_payload(&facts).ok_or_else(|| damage("faraday.codex32", Fault::Value))?;
        r.push(field::KEY_CODEX32, &p);
    }
    if let Some(v) = e.custom().get("faraday.child") {
        let of = contents::read_child_facts(v.as_bytes())
            .map_err(|_| damage("faraday.child", Fault::Value))?;
        let p = child::payload(&of).ok_or_else(|| damage("faraday.child", Fault::Value))?;
        r.push(field::KEY_CHILD, &p);
    }
    checked(r)
}

fn read_gpg(f: &mut Fields<'_>) -> Result<Body, Damage> {
    let key = f.need("key")?;
    if !forms::is_armor(key, "PRIVATE KEY BLOCK") {
        return Err(damage("key", Fault::Value));
    }
    let public = f.get("public");
    if public.is_some_and(|p| !forms::is_armor(p, "PUBLIC KEY BLOCK")) {
        return Err(damage("public", Fault::Value));
    }
    let revocation = f.get("revocation");
    if revocation.is_some_and(|p| !forms::is_armor(p, "PUBLIC KEY BLOCK")) {
        return Err(damage("revocation", Fault::Value));
    }
    let revoked = f.get("revoked");
    if revoked.is_some_and(|t| !forms::is_utc_time(t)) {
        return Err(damage("revoked", Fault::Value));
    }
    Ok(Body::Gpg(GpgKey {
        title: f.get("Title").unwrap_or("").into(),
        key: Zeroizing::new(key.into()),
        public: public.map(Into::into),
        revocation: revocation.map(|r| Zeroizing::new(r.into())),
        revoked: revoked.map(Into::into),
    }))
}

fn read_secure_boot(f: &mut Fields<'_>) -> Result<Body, Damage> {
    f.get("Title");
    let owner = forms::guid(f.need("owner")?).ok_or_else(|| damage("owner", Fault::Value))?;
    let mut r = Record::new(kind::SECURE_BOOT).with(1, &owner);
    for (name, n, label) in PEM_FIELDS {
        if let Some(text) = f.get(name) {
            let der = forms::pem_der(text, label).ok_or_else(|| damage(name, Fault::Value))?;
            r.push(n, &der);
        }
    }
    r.fields.sort_by_key(|x| x.number);
    checked(r)
}

/// Costs a parse of the words, a policy or a descriptor: an item of kind
/// `k` as its body.
fn read_kind(c: &Compute, k: u8, e: &Entry) -> Result<Body, Damage> {
    let mut f = Fields {
        e,
        used: Vec::new(),
    };
    let body = match k {
        kind::KEY => read_seed(c, e, &mut f)?,
        kind::WALLET => {
            let mut r = Record::new(kind::WALLET);
            let (which, text) = one_of(&mut f, "policy", "descriptor")?;
            let reads = if which == "policy" {
                WalletPolicy::parse(c, text).is_ok()
            } else {
                reads_as_descriptor(c, text)
            };
            if !reads {
                return Err(damage(which, Fault::Value));
            }
            r.push(field::WALLET, text.as_bytes());
            if let Some(t) = f.get("Title") {
                r.push(field::WALLET_NAME, t.as_bytes());
            }
            if flag(e, "faraday.load")? == Some(false) {
                r.push(field::WALLET_FLAGS, &[NO_LOAD_AT_UNLOCK]);
            }
            checked(r)?
        }
        kind::NOTE => {
            f.get("Title");
            let text = f.get("Notes").unwrap_or("");
            checked(Record::new(kind::NOTE).with(field::NOTE, text.as_bytes()))?
        }
        kind::SHEET => {
            let d = f.need("descriptor")?;
            if !reads_as_descriptor(c, d) {
                return Err(damage("descriptor", Fault::Value));
            }
            let name = f.get("Title").unwrap_or("");
            let note = f.get("Notes").unwrap_or("");
            let p = crate::layouts::sheet_payload(d.as_bytes(), name.as_bytes(), note.as_bytes())
                .ok_or_else(|| damage("descriptor", Fault::Value))?;
            checked(Record::new(kind::SHEET).with(field::SHEET, &p))?
        }
        kind::GPG => read_gpg(&mut f)?,
        kind::SECURE_BOOT => read_secure_boot(&mut f)?,
        kind::WALLET_SHARE => {
            let mut r =
                Record::new(kind::WALLET_SHARE).with(field::SHARE, f.need("Password")?.as_bytes());
            if let Some(t) = f.get("Title") {
                r.push(field::SHARE_WALLET_NAME, t.as_bytes());
            }
            for (name, n) in [("index", field::SHARE_NUMBER), ("of", field::SHARE_OF)] {
                if let Some(v) = f.get(name) {
                    let v = forms::decimal(v, 255).ok_or_else(|| damage(name, Fault::Value))?;
                    r.push(n, &[v as u8]);
                }
            }
            if let Some(w) = f.get("wallet") {
                r.push(field::SHARE_WALLET, w.as_bytes());
            }
            checked(r)?
        }
        kind::PASSPHRASE => {
            f.get("Title");
            let p = f.need("Password")?;
            let of =
                forms::fingerprint(f.need("for")?).ok_or_else(|| damage("for", Fault::Value))?;
            checked(
                Record::new(kind::PASSPHRASE)
                    .with(field::PASSPHRASE_KEY, &of)
                    .with(field::PASSPHRASE, p.as_bytes()),
            )?
        }
        kind::CODEX32_SHARE => {
            f.get("Title");
            let s = f.need("Password")?;
            if !codex32::is_share(s.as_bytes()) {
                return Err(damage("Password", Fault::Value));
            }
            let of =
                forms::fingerprint(f.need("for")?).ok_or_else(|| damage("for", Fault::Value))?;
            checked(
                Record::new(kind::CODEX32_SHARE)
                    .with(field::CODEX32_STRING, s.as_bytes())
                    .with(field::CODEX32_KEY, &of),
            )?
        }
        _ => return Err(damage("faraday.kind", Fault::UnknownKind)),
    };
    f.finish(k)?;
    Ok(body)
}

/// Whether a standard field's value is longer than a record holds
/// (65,535 bytes): such a field is not in the record but is an [`Extra`],
/// kept as it is.
fn too_long(v: &str) -> bool {
    v.len() > usize::from(u16::MAX)
}

/// A person's entry: its record, its other fields and its TOTP. An `otp`
/// that does not read, or HOTP, is the TOTP's badge, never a damage. The
/// entry never fails for a field's length: a standard field longer than
/// the record holds is left out of the record and given as an extra.
fn read_person(e: &Entry) -> Result<Person, Damage> {
    let mut r = Record::new(kind::ENTRY);
    let mut extra = Vec::new();
    for (name, n) in STANDARD.iter().zip(ENTRY_FIELDS) {
        let field = e.field(name);
        let v = field.map(|f| f.value()).unwrap_or("");
        if too_long(v) {
            extra.push(Extra {
                name: (*name).into(),
                protected: field.is_some_and(|f| f.protected()),
            });
            if n == field::TITLE {
                r.push(n, b"");
            }
        } else if n == field::TITLE || !v.is_empty() {
            r.push(n, v.as_bytes());
        }
    }
    extra.extend(
        e.fields()
            .filter(|f| !STANDARD.contains(&f.key()))
            .map(|f| Extra {
                name: f.key().into(),
                protected: f.protected(),
            }),
    );
    let pairs: Vec<(&str, &str)> = e.fields().map(|f| (f.key(), f.value())).collect();
    Ok((checked(r)?, extra, totp::from_fields(&pairs)))
}

/// A person's entry's record, other fields and TOTP.
type Person = (Body, Vec<Extra>, Option<Result<Totp, NotTotp>>);

/// The first key two fields of the entry share, as the document model
/// marks it.
fn has_same_field(e: &Entry) -> Option<Name> {
    e.duplicate_keys().next().map(Into::into)
}

// --------------------------------------------------------------- reading

/// What marks the groups an entry is in: a group's UUID another shares,
/// or the first reason a group is ambiguous.
#[derive(Clone, Copy, Default)]
struct Marks {
    same_uuid: bool,
    ambiguous: Option<&'static str>,
}

/// Every entry with its group path and the marks of its groups.
type Walked<'a> = Vec<(Path, Marks, &'a Entry)>;

/// `names` are the groups from the root down to `g`, each stored once; the
/// entries of `g` share one [`Path`] made from them (at most `MAX_DEPTH`
/// pointers), so a group name is never copied per entry.
fn walk<'a>(g: &'a Group, names: &mut Vec<Arc<str>>, marked: Marks, out: &mut Walked<'a>) {
    let marked = Marks {
        same_uuid: marked.same_uuid || g.duplicate_uuid(),
        ambiguous: marked.ambiguous.or(g.ambiguous()),
    };
    let mut path: Option<Path> = None;
    for e in g.entries() {
        let path = path.get_or_insert_with(|| names.as_slice().into());
        out.push((Arc::clone(path), marked, e));
    }
    for sub in g.groups() {
        names.push(sub.name().into());
        walk(sub, names, marked, out);
        names.pop();
    }
}

/// The number a new backup plan takes: one past every `faraday.plan.{n}`
/// in the database's custom data, readable or not (1 when there is none).
/// A database holding `faraday.plan.4294967295` has no number past it:
/// [`Unwritable::Plan`].
pub fn next_plan(doc: &Document) -> Result<u32, Unwritable> {
    doc.custom()
        .items()
        .filter_map(|(k, _)| k.strip_prefix(PLAN))
        .filter_map(|n| n.parse::<u32>().ok())
        .max()
        .map_or(Ok(1), |n| n.checked_add(1).ok_or(Unwritable::Plan))
}

/// Costs a parse of each seed's words and of each wallet's policy or
/// descriptor and each sheet's descriptor: reads what `doc` holds for the
/// app (module doc). Nothing here refuses the document.
pub fn read(c: &Compute, doc: &Document) -> Read {
    let mut unreadable = Vec::new();
    let mut custom = Vec::new();
    let meta_ambiguous = doc.ambiguous().or_else(|| doc.custom().ambiguous());
    let (format, seals, sealed) = read_meta(doc, meta_ambiguous, &mut custom, &mut unreadable);
    let mut all = Vec::new();
    walk(doc.root(), &mut Vec::new(), Marks::default(), &mut all);
    let mut items = Vec::new();
    for (path, group, e) in all {
        let bad = |what: Damage| Unreadable {
            title: title_of(e),
            uuid: Some(e.uuid()),
            what,
        };
        if e.duplicate_uuid() {
            unreadable.push(bad(damage("UUID", Fault::SameUuid)));
            continue;
        }
        if group.same_uuid {
            unreadable.push(bad(damage("Group", Fault::SameUuid)));
            continue;
        }
        if let Some(k) = has_same_field(e) {
            unreadable.push(bad(damage(&k, Fault::SameField)));
            continue;
        }
        if let Some(why) = group.ambiguous.or(e.ambiguous()) {
            unreadable.push(bad(damage(why, Fault::Ambiguous)));
            continue;
        }
        let read = match e.custom().get("faraday.kind") {
            None => read_person(e),
            Some(_) if matches!(format, Format::Newer(_)) => {
                Err(damage("faraday.format", Fault::Format))
            }
            Some(name) => match KINDS.iter().find(|(n, _)| *n == name) {
                None => Err(damage("faraday.kind", Fault::UnknownKind)),
                Some((_, k)) => read_kind(c, *k, e).map(|b| (b, Vec::new(), None)),
            },
        };
        match read {
            Ok((body, extra, totp)) => items.push(Item {
                uuid: e.uuid(),
                path,
                body,
                extra,
                totp,
                related: None,
            }),
            Err(d) => unreadable.push(bad(d)),
        }
    }
    relate(&mut items);
    Read {
        format,
        seals,
        sealed,
        label: doc.label().into(),
        items,
        custom,
        unreadable,
        history: doc.history_entries(),
        recycle_bin: doc.recycle_bin_entries(),
        meta_ambiguous,
    }
}

/// What a codex32 share is one of: its threshold, its set's identifier, its
/// index and its key, the same for shares of one set, either case.
fn share_set_of(r: &Record) -> Option<Vec<u8>> {
    if r.kind != kind::CODEX32_SHARE {
        return None;
    }
    // A share record's string was checked as codex32 when it was saved:
    // its header ("ms1", threshold, identifier, index) is read in place.
    let head = r.field(field::CODEX32_STRING)?.get(..9)?;
    let k = head[3].checked_sub(b'0').filter(|k| *k <= 9)?;
    let key = r.field(field::CODEX32_KEY)?;
    Some(
        [
            &[k, head[8].to_ascii_lowercase()][..],
            &head[4..8].to_ascii_lowercase(),
            key,
        ]
        .concat(),
    )
}

/// Related shares across items: several codex32 shares of one threshold,
/// identifier, index and key. Each reads; each is given its place among them, in the order read
/// (a group's entries, then its groups).
fn relate(items: &mut [Item]) {
    let key_of = |it: &Item| -> Option<Vec<u8>> {
        match &it.body {
            Body::Record(r) => share_set_of(r),
            Body::Gpg(_) => None,
        }
    };
    let mut count: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut seen: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    for it in items.iter() {
        if let Some(v) = key_of(it) {
            *count.entry(v).or_default() += 1;
        }
    }
    for it in items.iter_mut() {
        if let Some(key) = key_of(it) {
            let of = count[&key];
            let place = seen.entry(key).or_default();
            *place += 1;
            if of > 1 {
                it.related = Some(Related { place: *place, of });
            }
        }
    }
}

fn find_mut<'a>(g: &'a mut Group, uuid: &[u8; 16]) -> Option<&'a mut Entry> {
    if g.entries().any(|e| e.uuid() == *uuid) {
        return g.entries_mut().find(|e| e.uuid() == *uuid);
    }
    g.groups_mut().find_map(|s| find_mut(s, uuid))
}

/// How many entries of a document have each UUID, and whether the one that
/// has it cannot be told apart from what it was read as: built by one walk
/// of the document as read, so that a change to it is checked from the one
/// entry it touches.
pub struct Index {
    entries: alloc::collections::BTreeMap<[u8; 16], (usize, bool)>,
}

impl Index {
    /// Costs a walk of the document, linear in its size.
    pub fn of(doc: &Document) -> Self {
        fn walk(g: &Group, into: &mut alloc::collections::BTreeMap<[u8; 16], (usize, bool)>) {
            for e in g.entries() {
                let bad = has_same_field(e).is_some() || e.ambiguous().is_some();
                let at = into.entry(e.uuid()).or_insert((0, bad));
                at.0 += 1;
            }
            for s in g.groups() {
                walk(s, into);
            }
        }
        let mut entries = alloc::collections::BTreeMap::new();
        walk(doc.root(), &mut entries);
        Self { entries }
    }

    /// What the index holds: a count of its entries, no copy.
    pub fn held(&self) -> usize {
        // A key, a value and the tree's own nodes, with room to spare.
        self.entries.len() * 96
    }

    /// Whether an entry has this UUID: a lookup.
    pub(crate) fn holds(&self, uuid: &[u8; 16]) -> bool {
        self.entries.contains_key(uuid)
    }

    /// How many entries have this UUID.
    fn count(&self, uuid: &[u8; 16]) -> usize {
        self.entries.get(uuid).map_or(0, |e| e.0)
    }

    /// Whether the entry with this UUID is ambiguous or has two fields of
    /// one key.
    fn bad(&self, uuid: &[u8; 16]) -> bool {
        self.entries.get(uuid).is_some_and(|e| e.1)
    }
}

fn count_uuid(g: &Group, uuid: &[u8; 16]) -> usize {
    g.entries().filter(|e| e.uuid() == *uuid).count()
        + g.groups().map(|s| count_uuid(s, uuid)).sum::<usize>()
}

const PLAN: &str = "faraday.plan.";
const SIGNED: &str = "faraday.signed.";
const NONCES: &str = "faraday.nonces.";

/// The vault's own values, and the plans and signed amounts.
fn read_meta(
    doc: &Document,
    ambiguous: Option<&'static str>,
    custom: &mut Vec<Custom>,
    unreadable: &mut Vec<Unreadable>,
) -> (Format, u64, Sealed) {
    let mut format = Format::Foreign;
    let mut seals = 0;
    let mut sealed = None;
    if ambiguous.is_some() {
        // The values cannot be told apart: each is named and kept as it
        // is, and the vault is read-only.
        for (key, _) in doc.custom().items() {
            unreadable.push(Unreadable {
                title: key.chars().take(64).collect(),
                uuid: None,
                what: damage(key, Fault::Ambiguous),
            });
        }
        return (
            Format::Newer(ambiguous.unwrap_or("Meta").into()),
            seals,
            sealed,
        );
    }
    for (key, value) in doc.custom().items() {
        let mut bad = |fault: Fault| {
            unreadable.push(Unreadable {
                title: key.chars().take(64).collect(),
                uuid: None,
                what: damage(key, fault),
            });
        };
        // No key is met twice here: custom data lists only `faraday.*`
        // keys, and the reader marks a repeat of one ambiguous, which
        // returned above.
        match key {
            "faraday.format" => {
                format = if value == FORMAT {
                    Format::Faraday
                } else {
                    Format::Newer(value.chars().take(64).collect())
                };
            }
            "faraday.seals" => match forms::decimal(value, u64::MAX) {
                Some(n) => seals = n,
                None => bad(Fault::Value),
            },
            "faraday.sealed" => {
                if forms::is_utc_time(value) {
                    sealed = Some(value.into());
                } else {
                    bad(Fault::Value);
                }
            }
            _ if key.starts_with(PLAN) || key.starts_with(SIGNED) || key.starts_with(NONCES) => {
                match custom_record(key, value) {
                    Some(record) => custom.push(Custom {
                        key: key.into(),
                        record,
                    }),
                    None => bad(Fault::Value),
                }
            }
            _ => bad(Fault::UnknownField),
        }
    }
    (format, seals, sealed)
}

/// The record a plan's or a signed amounts' key and value make, if they
/// read.
fn custom_record(key: &str, value: &str) -> Option<Record> {
    if let Some(n) = key.strip_prefix(PLAN) {
        forms::decimal(n, u64::from(u32::MAX))?;
        contents::read_plan(value.as_bytes()).ok()
    } else if let Some(tx) = key.strip_prefix(NONCES) {
        nonces_of(tx, value)
    } else {
        signed_of(key.strip_prefix(SIGNED)?, value)
    }
}

/// A nonce record from its key's `{tx}` and its value: entries joined by
/// `,`, each `{key}:{r}:{msg}` of 64 lower-case hex digits apiece.
fn nonces_of(tx: &str, value: &str) -> Option<Record> {
    let tx = forms::unhex(tx).filter(|b| b.len() == 32)?;
    let mut r = Record::new(kind::NONCES).with(field::NONCES_TX, &tx);
    for entry in value.split(',') {
        let mut entry_bytes = zeroize::Zeroizing::new(Vec::with_capacity(96));
        let mut parts = 0;
        for part in entry.split(':') {
            parts += 1;
            if part.len() != 64 {
                return None;
            }
            entry_bytes.extend_from_slice(&forms::unhex(part)?);
        }
        if parts != 3 {
            return None;
        }
        r.push(field::NONCES_ENTRY, &entry_bytes);
    }
    r.check().ok()?;
    Some(r)
}

/// A signed amounts record from its key's `{tx}` and its value.
fn signed_of(tx: &str, value: &str) -> Option<Record> {
    let tx = forms::unhex(tx).filter(|b| b.len() == 32)?;
    let mut r = Record::new(kind::AMOUNTS).with(field::AMOUNTS_TX, &tx);
    for a in value.split(',') {
        r.push(field::AMOUNTS_INPUT, &forms::amount(a)?.to_le_bytes());
    }
    r.check().ok()?;
    Some(r)
}

/// Costs a parse of every seed's words and wallet's policy, twice: reads
/// `document` as KDBX XML and, when it reads, maps it for the app,
/// writes the mapped document again and maps that: the two maps agree on
/// what reads. For the fuzzer.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn fuzz_items(c: &Compute, document: &[u8]) -> bool {
    const KEY: &[u8] = &[7; 64];
    let Ok(doc) = Document::read(document, KEY) else {
        return false;
    };
    let first = read(c, &doc);
    let again = Document::read(&doc.write(KEY), KEY).expect("a written document reads");
    let second = read(c, &again);
    assert_eq!(first.items.len(), second.items.len());
    assert_eq!(first.unreadable, second.unreadable);
    true
}
