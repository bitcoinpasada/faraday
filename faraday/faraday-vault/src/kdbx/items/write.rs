//! Changes the app makes, applied to the document (`docs/VAULT.md` §7):
//! an item's fields and custom data written from its record by its
//! published text form, entry by UUID, and the database's own values.

use alloc::vec::Vec;

use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use super::{
    Body, ENTRY_FIELDS, FORMAT, KEEPASS, KINDS, NONCES, PEM_FIELDS, PLAN, Read, SIGNED, STANDARD,
    count_uuid, custom_record, find_mut, forms, has_same_field, named, named_custom, text,
};
use crate::kdbx::xml::{Document, Entry, Group};
use crate::records::{self, BACKUP_PENDING, NO_LOAD_AT_UNLOCK, Record, field, kind};
use crate::{child, codex32, contents};

/// Why a change was not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unwritable {
    #[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
    /// The vault is read-only: its format is not one this build writes, or
    /// its `Meta` or database custom data cannot be told apart from what it
    /// was read as.
    ReadOnly,
    /// A record of a type that is not an entry, or that does not pass its
    /// own check.
    Record(u8),
    /// A value that has no published form here (a payload that is not
    /// words or a seed, a GUID or a DER block of the wrong length).
    Value(&'static str),
    /// The entry with this UUID is one of two, has two fields of one key,
    /// does not read (`read` lists it as unreadable) or is otherwise
    /// ambiguous, or so is its group: it is not changed.
    Unreadable,
    /// A plan's number of 0, or a value that is not its TOML.
    Plan,
}

/// Where an item goes and what it is called (§7): the entry's UUID (an
/// existing one, or fresh bytes for a new one), its kind's group under the
/// root, named in the app's language, with the UUID a new group takes, the
/// title written where the record has no title of its own ("Key
/// {fingerprint}", "Note", "Passphrase for {for}", "Codex32 share for
/// {for}", "Secure Boot keys"), and the time to write as its last
/// modification when this computer's clock is set.
pub struct Place<'a> {
    /// The entry's UUID.
    pub uuid: [u8; 16],
    /// The kind's group, for a new entry; a person's entry goes in the root.
    pub group: &'a str,
    /// The UUID a new group takes.
    pub group_uuid: [u8; 16],
    /// The title, where the record has none.
    pub title: &'a str,
    /// Unix seconds, when the clock is set.
    pub now: Option<i64>,
}

fn text_of(r: &Record, n: u8) -> &str {
    r.text(n).unwrap_or("")
}

/// The fields and custom data an entry of this body holds: each a key, a
/// value and whether it is protected.
type Pairs = Vec<(&'static str, Zeroizing<Vec<u8>>, bool)>;

fn z(s: &str) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(s.as_bytes().to_vec())
}

/// A seed's fields and custom data.
fn seed_fields(r: &Record, title: &str, fl: &mut Pairs, cd: &mut Pairs) -> Result<(), Unwritable> {
    let title_or = |n: u8| r.text(n).filter(|t| !t.is_empty()).unwrap_or(title);
    fl.push(("Title", z(title_or(field::KEY_LABEL)), false));
    let payload = r.field(field::KEY).unwrap_or(&[]);
    if let Some((words, lang)) = forms::words_text(payload) {
        fl.push(("Password", words, true));
        if let Some(name) = forms::list_name(lang) {
            fl.push(("list", z(name), false));
        }
    } else {
        let hex = forms::seed_hex(payload).ok_or(Unwritable::Value("hex"))?;
        fl.push(("hex", hex, true));
    }
    if let Some(p) = r.text(field::KEY_PASSPHRASE) {
        fl.push(("passphrase", z(p), true));
    }
    let flags = r
        .field(field::KEY_FLAGS)
        .and_then(|b| b.first().copied())
        .unwrap_or(0);
    if flags & NO_LOAD_AT_UNLOCK != 0 {
        cd.push(("faraday.load", z("false"), false));
    }
    if flags & BACKUP_PENDING != 0 {
        cd.push(("faraday.held", z("true"), false));
    }
    if let Some(b) = r.field(field::KEY_CODEX32) {
        let facts = codex32::set_of(b).ok_or(Unwritable::Value("faraday.codex32"))?;
        let v = contents::write_codex32_facts(&facts)
            .map_err(|_| Unwritable::Value("faraday.codex32"))?;
        cd.push(("faraday.codex32", v, false));
    }
    if let Some(b) = r.field(field::KEY_CHILD) {
        let of = child::child_of(b).ok_or(Unwritable::Value("faraday.child"))?;
        let v = contents::write_child_facts(&of).map_err(|_| Unwritable::Value("faraday.child"))?;
        cd.push(("faraday.child", v, false));
    }
    Ok(())
}

/// The fields of a GPG key's entry.
fn gpg_fields(g: &super::GpgKey) -> Pairs {
    let mut fl: Pairs = Vec::new();
    fl.push(("Title", z(&g.title), false));
    fl.push(("key", z(&g.key), true));
    if let Some(p) = &g.public {
        fl.push(("public", z(p), false));
    }
    if let Some(p) = &g.revocation {
        fl.push(("revocation", z(p), true));
    }
    if let Some(p) = &g.revoked {
        fl.push(("revoked", z(p), false));
    }
    fl
}

/// Costs a parse of a wallet's text: an entry's fields and custom data for
/// a body.
fn fields_for(c: &Compute, body: &Body, title: &str) -> Result<(Pairs, Pairs), Unwritable> {
    let mut fl: Pairs = Vec::new();
    let mut cd: Pairs = Vec::new();
    let r = match body {
        Body::Gpg(g) => return Ok((gpg_fields(g), cd)),
        Body::Record(r) => r,
    };
    r.check().map_err(|_| Unwritable::Record(r.kind))?;
    match r.kind {
        kind::ENTRY => {
            for (name, n) in STANDARD.iter().zip(ENTRY_FIELDS) {
                let v = text_of(r, n);
                let protected = matches!(n, field::PASSWORD | field::TOTP);
                if n != field::TOTP || !v.is_empty() {
                    fl.push((name, z(v), protected));
                }
            }
        }
        kind::KEY => seed_fields(r, title, &mut fl, &mut cd)?,
        kind::WALLET => {
            fl.push(("Title", z(text_of(r, field::WALLET_NAME)), false));
            let text = text_of(r, field::WALLET);
            let name = if WalletPolicy::parse(c, text).is_ok() {
                "policy"
            } else {
                "descriptor"
            };
            fl.push((name, z(text), false));
            if !records::loads_at_unlock(r) {
                cd.push(("faraday.load", z("false"), false));
            }
        }
        kind::NOTE => {
            fl.push(("Title", z(title), false));
            fl.push(("Notes", z(text_of(r, field::NOTE)), true));
        }
        kind::SHEET => {
            let p = r.field(field::SHEET).unwrap_or(&[]);
            let Some(crate::layouts::Opened::Sheet(s)) =
                crate::layouts::payload_of(crate::layouts::KIND_SHEET, p)
            else {
                return Err(Unwritable::Value("descriptor"));
            };
            let s8 = |b: &[u8]| {
                core::str::from_utf8(b)
                    .map(z)
                    .map_err(|_| Unwritable::Value("descriptor"))
            };
            fl.push(("Title", s8(&s.name)?, false));
            fl.push(("descriptor", s8(&s.descriptor)?, false));
            fl.push(("Notes", s8(&s.note)?, false));
        }
        kind::SECURE_BOOT => {
            fl.push(("Title", z(title), false));
            let owner =
                forms::guid_text(r.field(1).unwrap_or(&[])).ok_or(Unwritable::Value("owner"))?;
            fl.push(("owner", owner, false));
            for (name, n, label) in PEM_FIELDS {
                if let Some(der) = r.field(n) {
                    let pem = forms::der_pem(der, label).ok_or(Unwritable::Value(name))?;
                    fl.push((name, pem, label == "PRIVATE KEY"));
                }
            }
        }
        kind::WALLET_SHARE => {
            fl.push(("Title", z(text_of(r, field::SHARE_WALLET_NAME)), false));
            fl.push(("Password", z(text_of(r, field::SHARE)), true));
            for (name, n) in [("index", field::SHARE_NUMBER), ("of", field::SHARE_OF)] {
                if let Some(&[v]) = r.field(n) {
                    fl.push((name, z(&text::number(u64::from(v))), false));
                }
            }
            if let Some(w) = r.text(field::SHARE_WALLET) {
                fl.push(("wallet", z(w), false));
            }
        }
        kind::PASSPHRASE => {
            fl.push(("Title", z(title), false));
            fl.push(("Password", z(text_of(r, field::PASSPHRASE)), true));
            fl.push((
                "for",
                forms::hex(r.field(field::PASSPHRASE_KEY).unwrap_or(&[])),
                false,
            ));
        }
        kind::CODEX32_SHARE => {
            fl.push(("Title", z(title), false));
            fl.push(("Password", z(text_of(r, field::CODEX32_STRING)), true));
            fl.push((
                "for",
                forms::hex(r.field(field::CODEX32_KEY).unwrap_or(&[])),
                false,
            ));
        }
        k => return Err(Unwritable::Record(k)),
    }
    Ok((fl, cd))
}

/// Whether the database's `Meta` or its custom data is ambiguous.
fn meta_ambiguous(doc: &Document) -> bool {
    doc.ambiguous().is_some() || doc.custom().ambiguous().is_some()
}

fn is_newer(doc: &Document) -> bool {
    doc.custom()
        .get("faraday.format")
        .is_some_and(|f| f != FORMAT)
}

/// Why nothing may be changed: a newer format, or a `Meta` or custom data
/// that cannot be told apart (it counts as a newer format): both are
/// [`Unwritable::ReadOnly`].
fn locked(doc: &Document) -> Result<(), Unwritable> {
    if is_newer(doc) {
        return Err(Unwritable::ReadOnly);
    }
    if meta_ambiguous(doc) {
        return Err(Unwritable::ReadOnly);
    }
    Ok(())
}

/// Costs [`super::read`] unless `known` is given: the entry with this UUID
/// may be changed: it is not two, and `read` does not list it as
/// unreadable. Gives how many entries have the UUID. `known` is what `read`
/// gave for the document before the changes of one batch, for an entry no
/// change of the batch touched before: its answer is the same, without
/// reading every other entry again.
fn changeable(
    c: &Compute,
    doc: &Document,
    uuid: &[u8; 16],
    known: Option<&Read>,
) -> Result<usize, Unwritable> {
    let n = count_uuid(doc.root(), uuid);
    if n > 1 {
        return Err(Unwritable::Unreadable);
    }
    // An entry that is not there cannot be listed.
    if n == 0 {
        return Ok(n);
    }
    match known {
        Some(read) => changeable_n(read, n, uuid),
        None => changeable_n(&super::read(c, doc), n, uuid),
    }
}

/// [`changeable`] from the number of entries with the UUID and the `read`
/// of the document.
fn changeable_n(read: &Read, n: usize, uuid: &[u8; 16]) -> Result<usize, Unwritable> {
    if n > 1 {
        return Err(Unwritable::Unreadable);
    }
    if read
        .unreadable
        .iter()
        .any(|u| u.uuid.as_ref() == Some(uuid))
    {
        return Err(Unwritable::Unreadable);
    }
    Ok(n)
}

/// Why a plan's or signed amounts' key may not be changed: it holds a
/// value that does not read (`read` lists it as unreadable).
fn custom_readable(doc: &Document, key: &str) -> Result<(), Unwritable> {
    if !(key.starts_with(PLAN) || key.starts_with(SIGNED) || key.starts_with(NONCES)) {
        return Ok(());
    }
    match doc.custom().get(key) {
        Some(value) if custom_record(key, value).is_none() => Err(Unwritable::Unreadable),
        _ => Ok(()),
    }
}

/// Costs a parse of every seed's words and wallet's policy (it runs
/// [`super::read`], unless [`put_known`] is given what it read) and of a
/// wallet's text: writes `body` into the entry with
/// `place.uuid`, or into a new entry with that UUID in `place.group` (a
/// person's entry in the root group). An existing entry keeps everything
/// but the fields and the custom data its kind names, and its last
/// modification when `place.now` is given; a field the kind names that the
/// body leaves out is removed, except that a person's entry keeps a
/// standard field longer than a record holds (which `read` gives as an
/// [`Extra`](super::Extra)) unless the body gives that field a value.
/// Refused when the entry is unreadable. A passphrase for a key another
/// passphrase is kept for, or a codex32 share of an index and key another is
/// kept of, is taken: `read` gives each its place among them.
pub fn put(
    c: &Compute,
    doc: &mut Document,
    place: &Place<'_>,
    body: &Body,
) -> Result<(), Unwritable> {
    put_known(c, doc, place, body, None)
}

/// Costs what [`put`] costs, less the read of the document when `known` is
/// given: [`put`], reading the document for the entry's readability only when
/// `known` is not given: the `read` of the document before the batch of
/// changes this is one of, valid for an entry no earlier change of the
/// batch touched.
pub fn put_known(
    c: &Compute,
    doc: &mut Document,
    place: &Place<'_>,
    body: &Body,
    known: Option<&Read>,
) -> Result<(), Unwritable> {
    locked(doc)?;
    if doc.root().ambiguous().is_some() {
        return Err(Unwritable::Unreadable);
    }
    changeable(c, doc, &place.uuid, known)?;
    let (fl, cd) = fields_for(c, body, place.title)?;
    let k = body.kind();
    let name = KINDS.iter().find(|(_, n)| *n == k).map(|(n, _)| *n);
    let root = doc.root_mut();
    match count_uuid(root, &place.uuid) {
        0 => {
            let entry = Entry::new(place.uuid);
            if name.is_none() {
                root.push_entry(entry).map_err(|_| Unwritable::Unreadable)?;
            } else {
                if !root.groups().any(|g| g.name() == place.group) {
                    root.push_group(Group::new(place.group_uuid, place.group))
                        .map_err(|_| Unwritable::Unreadable)?;
                }
                match root.groups_mut().find(|g| g.name() == place.group) {
                    Some(g) if g.duplicate_uuid() || g.ambiguous().is_some() => {
                        return Err(Unwritable::Unreadable);
                    }
                    Some(g) => g.push_entry(entry).map_err(|_| Unwritable::Unreadable)?,
                    None => {}
                }
            }
        }
        1 => {}
        _ => return Err(Unwritable::Unreadable),
    }
    let e = find_mut(root, &place.uuid).ok_or(Unwritable::Unreadable)?;
    if has_same_field(e).is_some() || e.ambiguous().is_some() {
        return Err(Unwritable::Unreadable);
    }
    let names: &[&str] = if name.is_some() { named(k) } else { &STANDARD };
    // A person's entry's field longer than a record holds is not in the
    // body: the body's empty value does not blank it.
    let kept_long = |e: &Entry, key: &str| {
        name.is_none() && e.field(key).is_some_and(|f| super::too_long(f.value()))
    };
    for n in names {
        if !fl.iter().any(|(f, _, _)| f == n) && !kept_long(e, n) {
            e.remove_field(n).map_err(|_| Unwritable::Unreadable)?;
        }
    }
    if name.is_some() {
        for n in KEEPASS.iter().filter(|n| !names.contains(n)) {
            e.remove_field(n).map_err(|_| Unwritable::Unreadable)?;
        }
    }
    for (key, value, protected) in &fl {
        let v = core::str::from_utf8(value).map_err(|_| Unwritable::Value(key))?;
        if v.is_empty() && kept_long(e, key) {
            continue;
        }
        // A person's entry keeps each field's protection as a KeePass app
        // set it; an item's protection is §7's.
        let protected = match (name, e.field(key)) {
            (None, Some(f)) => f.protected(),
            _ => *protected,
        };
        e.set_field(key, v, protected)
            .map_err(|_| Unwritable::Unreadable)?;
    }
    let bad = |_| Unwritable::Unreadable;
    let custom = e.custom_mut();
    match name {
        Some(n) => {
            custom.set("faraday.kind", n).map_err(bad)?;
        }
        None => {
            custom.remove("faraday.kind").map_err(bad)?;
        }
    }
    for key in named_custom(k) {
        if !cd.iter().any(|(c, _, _)| c == key) {
            custom.remove(key).map_err(bad)?;
        }
    }
    for (key, value, _) in &cd {
        let v = core::str::from_utf8(value).map_err(|_| Unwritable::Value(key))?;
        custom.set(key, v).map_err(bad)?;
    }
    if let Some(t) = place.now {
        e.set_modified(t).map_err(|_| Unwritable::Unreadable)?;
    }
    Ok(())
}

/// Costs a parse of every seed's words and wallet's policy: removes the
/// entry with this UUID, wherever it is; whether it was there. Refused on
/// a read-only vault, when the UUID is in the document more than once and
/// when `read` lists the entry as unreadable.
pub fn remove(c: &Compute, doc: &mut Document, uuid: &[u8; 16]) -> Result<bool, Unwritable> {
    remove_known(c, doc, uuid, None)
}

/// Costs what [`remove`] costs, less the read of the document when `known`
/// is given. [`remove`], with `known` as for [`put_known`].
pub fn remove_known(
    c: &Compute,
    doc: &mut Document,
    uuid: &[u8; 16],
    known: Option<&Read>,
) -> Result<bool, Unwritable> {
    fn go(g: &mut Group, uuid: &[u8; 16]) -> bool {
        g.remove_entry(uuid).unwrap_or(false) || g.groups_mut().any(|s| go(s, uuid))
    }
    locked(doc)?;
    if changeable(c, doc, uuid, known)? == 0 {
        return Ok(false);
    }
    Ok(go(doc.root_mut(), uuid))
}

/// Writes a backup plan record as `faraday.plan.{n}`, `n` from 1. A new
/// plan takes a number past every existing `faraday.plan.{n}`, readable or
/// not (`read` names the ones that do not read, and they are kept as they
/// are); writing to a number whose value does not read is refused.
pub fn put_plan(doc: &mut Document, n: u32, record: &Record) -> Result<(), Unwritable> {
    locked(doc)?;
    if n == 0 {
        return Err(Unwritable::Plan);
    }
    let v = contents::write_plan(record).map_err(|_| Unwritable::Plan)?;
    let v = core::str::from_utf8(&v).map_err(|_| Unwritable::Plan)?;
    let key = text::plan_key(n);
    custom_readable(doc, &key)?;
    doc.custom_mut().set(&key, v).map_err(|_| Unwritable::Plan)
}

/// Writes a signed amounts record as `faraday.signed.{tx}`; refused when
/// the value already under that key does not read.
pub fn put_signed(doc: &mut Document, record: &Record) -> Result<(), Unwritable> {
    locked(doc)?;
    if record.kind != kind::AMOUNTS || record.check().is_err() {
        return Err(Unwritable::Record(record.kind));
    }
    let key = text::signed_key(&forms::hex(record.field(field::AMOUNTS_TX).unwrap_or(&[])));
    let value = text::signed_value(
        record
            .fields
            .iter()
            .filter(|f| f.number == field::AMOUNTS_INPUT)
            .map(|f| {
                let mut a = [0u8; 8];
                a.copy_from_slice(&f.bytes);
                u64::from_le_bytes(a)
            }),
    );
    custom_readable(doc, &key)?;
    doc.custom_mut()
        .set(&key, &value)
        .map_err(|_| Unwritable::Record(record.kind))
}

/// Writes a nonce record as `faraday.nonces.{tx}`; refused when the value
/// already under that key does not read.
pub fn put_nonces(doc: &mut Document, record: &Record) -> Result<(), Unwritable> {
    locked(doc)?;
    if record.kind != kind::NONCES || record.check().is_err() {
        return Err(Unwritable::Record(record.kind));
    }
    let key = text::nonces_key(&forms::hex(record.field(field::NONCES_TX).unwrap_or(&[])));
    let value = text::nonces_value(
        record
            .fields
            .iter()
            .filter(|f| f.number == field::NONCES_ENTRY)
            .map(|f| &f.bytes[..]),
    );
    custom_readable(doc, &key)?;
    doc.custom_mut()
        .set(&key, &value)
        .map_err(|_| Unwritable::Record(record.kind))
}

/// Removes a plan, a transaction's signed amounts or its nonce record by its key; the
/// vault's own values (`faraday.format`, `faraday.seals`, `faraday.sealed`)
/// are not removed. Whether it was there. Refused on a read-only vault and
/// for a key whose value does not read.
pub fn remove_custom(doc: &mut Document, key: &str) -> Result<bool, Unwritable> {
    locked(doc)?;
    custom_readable(doc, key)?;
    Ok(
        (key.starts_with(PLAN) || key.starts_with(SIGNED) || key.starts_with(NONCES))
            && doc.custom_mut().remove(key).unwrap_or(false),
    )
}

/// Writes the vault's own values for a seal: `faraday.format` 1, the seal
/// count, and the time of the seal when the clock is set (removed when it
/// is not). For `faraday-vault`'s seal alone.
pub fn stamp(doc: &mut Document, seals: u64, sealed: Option<&str>) -> Result<(), Unwritable> {
    locked(doc)?;
    let bad = |_| Unwritable::Unreadable;
    let custom = doc.custom_mut();
    custom.set("faraday.format", FORMAT).map_err(bad)?;
    custom
        .set("faraday.seals", &text::number(seals))
        .map_err(bad)?;
    match sealed {
        Some(t) if forms::is_utc_time(t) => {
            custom.set("faraday.sealed", t).map_err(bad)?;
        }
        Some(_) => return Err(Unwritable::Value("faraday.sealed")),
        None => {
            custom.remove("faraday.sealed").map_err(bad)?;
        }
    }
    Ok(())
}

/// Costs a parse of a wallet's text: whether `change` would be refused as
/// [`Unwritable`] if applied to `doc`, found against `read` (the document
/// as it was read) and without a copy of the document or a write of it.
/// The refusals of [`put`], [`remove`], [`put_plan`],
/// [`put_signed`], [`remove_custom`] and [`stamp`] that depend on the
/// change and the document as read; the seal applies the changes one after
/// another and refuses what only that order shows.
pub fn check(
    c: &Compute,
    doc: &Document,
    read: &Read,
    index: &super::Index,
    change: &crate::file::Change<'_>,
) -> Result<(), Unwritable> {
    use crate::file::Change;
    locked(doc)?;
    match change {
        Change::Put { place, body } => {
            if doc.root().ambiguous().is_some() {
                return Err(Unwritable::Unreadable);
            }
            let n = changeable_n(read, index.count(&place.uuid), &place.uuid)?;
            let (fl, cd) = fields_for(c, body, place.title)?;
            for (key, value, _) in fl.iter().chain(cd.iter()) {
                core::str::from_utf8(value).map_err(|_| Unwritable::Value(key))?;
            }
            let k = body.kind();
            let named_kind = KINDS.iter().any(|(_, n)| *n == k);
            match n {
                0 if named_kind => {
                    let group = doc.root().groups().find(|g| g.name() == place.group);
                    if group.is_some_and(|g| g.duplicate_uuid() || g.ambiguous().is_some()) {
                        return Err(Unwritable::Unreadable);
                    }
                }
                0 => {}
                1 => {
                    if index.bad(&place.uuid) {
                        return Err(Unwritable::Unreadable);
                    }
                }
                _ => return Err(Unwritable::Unreadable),
            }
        }
        Change::Remove(uuid) => {
            changeable_n(read, index.count(uuid), uuid)?;
        }
        Change::Plan { n, record } => {
            if *n == 0 {
                return Err(Unwritable::Plan);
            }
            let v = contents::write_plan(record).map_err(|_| Unwritable::Plan)?;
            core::str::from_utf8(&v).map_err(|_| Unwritable::Plan)?;
            custom_readable(doc, &text::plan_key(*n))?;
        }
        Change::Signed(record) => {
            if record.kind != kind::AMOUNTS || record.check().is_err() {
                return Err(Unwritable::Record(record.kind));
            }
            let key = text::signed_key(&forms::hex(record.field(field::AMOUNTS_TX).unwrap_or(&[])));
            custom_readable(doc, &key)?;
        }
        Change::Nonces(record) => {
            if record.kind != kind::NONCES || record.check().is_err() {
                return Err(Unwritable::Record(record.kind));
            }
            let key = text::nonces_key(&forms::hex(record.field(field::NONCES_TX).unwrap_or(&[])));
            custom_readable(doc, &key)?;
        }
        Change::RemoveCustom(key) => custom_readable(doc, key)?,
        Change::Label(_) => {}
    }
    Ok(())
}
