//! Reading a vault's records and editing them.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use super::*;

impl OpenVaults {
    // ---- reading ----

    /// A lookup. Open vault `id`'s records, for reading. Empty for an id
    /// this store has not made or has closed.
    pub fn records(&self, id: VaultId) -> Records<'_> {
        let s = self.slot(id);
        Records {
            id,
            contents: s.map_or(&[][..], |s| &s.records),
            slot: s.map(|s| (s, &self.names)),
        }
    }

    /// A lookup. How many edits open vault `id`'s records have had since
    /// it was unlocked: what a caller's record of the vault is kept in
    /// step by. Zero for a vault this store does not hold.
    pub fn version(&self, id: VaultId) -> u64 {
        self.slot(id).map_or(0, |s| s.version)
    }

    /// A lookup. What open vault `id`'s file does not read: items and
    /// keys with where and what, and GPG keys not in Faraday's shape.
    /// Written back as read. No secrets.
    pub fn unreadable(&self, id: VaultId) -> &[Unreadable] {
        self.slot(id).map_or(&[][..], |s| &s.unreadable)
    }

    /// A lookup. Why open vault `id` may not be changed, if it may not:
    /// every edit refuses while this is `Some`.
    pub fn read_only(&self, id: VaultId) -> Option<fv::file::ReadOnly> {
        self.slot(id)?.opened.read_only()
    }

    /// A lookup. What the clear header of open vault `id`'s file states.
    pub fn facts(&self, id: VaultId) -> Option<&fv::file::Facts> {
        Some(self.slot(id)?.opened.facts())
    }

    /// Costs a walk of every record changed since the vault opened (no
    /// document is read). The document's length and the attachments' together in
    /// bytes, for a seal of open vault `id`: the attachments as the vault
    /// opened with them, and the document as the larger of its opened length
    /// and what [`Slot::bound`] gives for the plaintext now, less the
    /// attachments, so what was added since it opened is counted. A seal
    /// cannot write the file as it is (a bound of `usize::MAX`): the opened
    /// length.
    pub fn held(&self, id: VaultId) -> Option<(usize, usize)> {
        let s = self.slot(id)?;
        let (document, attachments) = s.opened.held();
        let bound = s.bound(&self.names, &Overlay::NONE);
        if bound == usize::MAX {
            return Some((document, attachments));
        }
        Some((document.max(bound.saturating_sub(attachments)), attachments))
    }

    /// A lookup. Tells the store the longest document a copy may be made
    /// of, in bytes, from what this computer has free; `None` when it does
    /// not say. Where a count would copy a longer one it is not made
    /// ([`Room::Short`]).
    pub fn set_copy_room(&self, bytes: Option<usize>) {
        self.copy_room.set(bytes);
    }

    /// Tells the store how to read the room a copy may be made of now, so
    /// that a count that copies a document reads it first, never takes it
    /// from an earlier reading.
    pub fn set_copy_reader(&self, read: impl Fn() -> Option<usize> + Send + Sync + 'static) {
        if let Ok(mut r) = self.copy_room.reader.lock() {
            *r = Some(Box::new(read));
        }
    }

    /// Costs what [`Slot::used_with`] costs: a read of the free memory, a
    /// copy of the document, a count of it and a read of each item put.
    /// [`OpenVaults::held`] with the document counted exactly rather than
    /// bounded, for the seal's last word when the bound's figure is more
    /// than is free. `None` when memory is short for that copy (or no vault
    /// `id` is held): the caller keeps the bound's figure
    /// ([`OpenVaults::held`]) and refuses on it, never lets the seal go. For
    /// a vault that cannot be written as it is, the bound's figure.
    pub fn held_exact(&self, c: &Compute, id: VaultId) -> Option<(usize, usize)> {
        let s = self.slot(id)?;
        let (document, attachments) = s.opened.held();
        match s.used_with(c, &self.names, &Overlay::NONE) {
            Ok(n) if n != usize::MAX => {
                Some((document.max(n.saturating_sub(attachments)), attachments))
            }
            Err(NoRoom::Short(_)) => None,
            _ => self.held(id),
        }
    }

    /// A lookup. Open vault `id`'s identifier: the root group's UUID in
    /// hex.
    pub fn vault_id(&self, id: VaultId) -> Option<[u8; 32]> {
        Some(self.slot(id)?.opened.vault_id())
    }

    /// A lookup. The names of the groups record `i` of open vault `id`
    /// was read from, under the root; empty for the root, a new record or
    /// one not an entry.
    pub fn path(&self, id: VaultId, i: usize) -> &[Arc<str>] {
        match self.slot(id).and_then(|s| s.origin.get(i)) {
            Some(Origin::Entry { path, .. }) => path,
            _ => &[],
        }
    }

    /// A lookup. Record `i`'s place among the records of open vault `id`
    /// that hold a codex32 share of one threshold, identifier, index and
    /// key, in record order; `None` for one that is alone or holds none.
    pub fn related(&self, id: VaultId, i: usize) -> Option<Related> {
        let s = self.slot(id)?;
        let key = share_set_of(s.records.get(i)?)?;
        let (mut of, mut place) = (0, 0);
        for (j, r) in s.records.iter().enumerate() {
            if share_set_of(r).is_some_and(|v| v == key) {
                of += 1;
                if j <= i {
                    place += 1;
                }
            }
        }
        (of > 1).then_some(Related { place, of })
    }

    /// Costs what [`Slot::room`] costs: a check of each record put, and
    /// a read of the free memory and a count of the document only where the
    /// vault may be near its ceiling. Whether open vault `id` still has room after records
    /// `remove` (by place) are taken out and the records `add` are put
    /// in: the plaintext the seal would take, against the ceiling. False
    /// for a vault that is read-only or not held.
    pub fn fits<R: Fit>(&self, c: &Compute, id: VaultId, remove: &[usize], add: &[&R]) -> bool {
        self.fit(c, id, remove, add) == Room::Fits
    }

    /// Costs what [`OpenVaults::fits`] costs. Whether open vault `id`
    /// has room after records `remove` (by place) are taken out and the
    /// records `add` are put in, past the ceiling only on an exact count:
    /// where that count needs a copy of the document that what is free does
    /// not hold, it is [`Room::Short`], never full.
    pub fn fit<R: Fit>(&self, c: &Compute, id: VaultId, remove: &[usize], add: &[&R]) -> Room {
        let Some(s) = self.slot(id) else {
            return Room::Full;
        };
        let add: Vec<&Record> = add.iter().map(|r| r.record_of(Inside(()))).collect();
        let add = &add[..];
        let ov = Overlay {
            remove,
            add,
            edit: &[],
        };
        match s.room(c, &self.names, &ov) {
            Ok(()) => Room::Fits,
            Err(NoRoom::Short(len)) => Room::Short(len),
            Err(NoRoom::File(_)) => Room::Full,
        }
    }

    /// Costs a document write and a read of each item put. Whether a vault
    /// about to be made, named `label` and `size` bytes of plaintext, has
    /// room for the records `add` at its first seal after it is made: the
    /// exact plaintext the seal would take. False for a record the seal
    /// cannot write.
    pub fn fits_new<R: Fit>(&self, c: &Compute, label: &str, size: usize, add: &[&R]) -> bool {
        let mut ops = Vec::new();
        let mut plan = None;
        for r in add {
            if fresh_op(r.record_of(Inside(())), &mut plan, &mut ops).is_err() {
                return false;
            }
        }
        // Each new entry and group takes a UUID of its own, as the seal's.
        let mut n = 0u64;
        let mut next = || {
            n += 1;
            let mut u = [0u8; 16];
            u[..8].copy_from_slice(&n.to_le_bytes());
            u
        };
        let mut groups: Vec<(u8, [u8; 16])> = Vec::new();
        for op in &mut ops {
            if let Op::Put {
                uuid,
                group_uuid,
                new: true,
                record,
            } = op
            {
                *uuid = next();
                if record.kind != kind::ENTRY {
                    *group_uuid = match groups.iter().find(|(k, _)| *k == record.kind) {
                        Some((_, u)) => *u,
                        None => {
                            let u = next();
                            groups.push((record.kind, u));
                            u
                        }
                    };
                }
            } else {
                return false;
            }
        }
        with_changes(c, &self.names, &ops, false, Some(0), |changes| {
            fv::file::used_new(c, label, changes)
        })
        .is_ok_and(|used| used <= size)
    }

    // ---- edits ----

    /// Puts `record` last in open vault `id`; its place. `None` for a
    /// vault this store does not hold or that is read-only.
    pub fn insert(&mut self, id: VaultId, record: impl IntoRecord) -> Option<usize> {
        let s = self.slot_mut(id)?;
        s.records.push(record.into_record());
        s.origin.push(Origin::New);
        s.dirty.push(true);
        s.facts.push(OnceCell::new());
        s.unchecked.push(Cell::new(true));
        s.changed();
        Some(s.records.len() - 1)
    }

    /// Puts `record` at place `at` in open vault `id`, the records from
    /// there on moving up.
    pub fn insert_at(&mut self, id: VaultId, at: usize, record: impl IntoRecord) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let at = at.min(s.records.len());
        s.records.insert(at, record.into_record());
        s.origin.insert(at, Origin::New);
        s.dirty.insert(at, true);
        s.facts.insert(at, OnceCell::new());
        s.unchecked.insert(at, Cell::new(true));
        s.changed();
        true
    }

    /// Puts `record` in place of record `i` of open vault `id`.
    pub fn replace(&mut self, id: VaultId, i: usize, record: impl IntoRecord) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        *r = record.into_record();
        s.touch(i);
        true
    }

    /// Takes the codex32 set (`field::KEY_CODEX32`) or the BIP-85 origin
    /// (`field::KEY_CHILD`) off key record `i` of open vault `id`, and
    /// nothing else of it; it is gone from the file at the next seal.
    /// Does nothing, and says so with `false`, for another field, a record
    /// that is not a key or lacks the field, a vault that may not be
    /// changed, or one whose [`OpenVaults::version`] is not `version`
    /// (something changed since the person was asked).
    pub fn drop_custom(&mut self, id: VaultId, i: usize, number: u8, version: u64) -> bool {
        if number != field::KEY_CODEX32 && number != field::KEY_CHILD {
            return false;
        }
        let Some(s) = self.slot_mut(id).filter(|s| s.version == version) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if r.kind != kind::KEY || r.field(number).is_none() {
            return false;
        }
        r.set(number, &[]);
        s.touch(i);
        true
    }

    /// Takes record `i` out of open vault `id`.
    pub fn remove(&mut self, id: VaultId, i: usize) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        if i >= s.records.len() {
            return false;
        }
        s.records.remove(i);
        s.dirty.remove(i);
        s.facts.remove(i);
        s.unchecked.remove(i);
        s.removed.push(s.origin.remove(i));
        s.changed();
        true
    }

    /// Takes out of open vault `id` every record `gone` says goes; how
    /// many went. `gone` is asked once per record, in order; the `home` of
    /// the view it is given is the record's place now, shifted down by the
    /// records that went before it, not its place before the call (count
    /// the calls for that).
    pub fn remove_where(
        &mut self,
        id: VaultId,
        mut gone: impl FnMut(RecordView<'_>) -> bool,
    ) -> usize {
        let Some(s) = self.slot_mut(id) else {
            return 0;
        };
        let mut n = 0;
        let mut at = 0;
        while at < s.records.len() {
            let view = RecordView {
                record: &s.records[at],
                home: Some((id, at)),
            };
            if gone(view) {
                s.records.remove(at);
                s.dirty.remove(at);
                s.facts.remove(at);
                s.unchecked.remove(at);
                s.removed.push(s.origin.remove(at));
                n += 1;
            } else {
                at += 1;
            }
        }
        if n > 0 {
            s.changed();
        }
        n
    }

    /// Sets public field `number` of record `i` of open vault `id`; an
    /// empty `bytes` takes the field out. `false` for a secret field
    /// ([`is_secret`]), which only [`OpenVaults::set_secret`] writes, or
    /// no such record. No check of the vault's size: for what a seal or
    /// the plan's own bookkeeping writes; a person's edit is
    /// [`OpenVaults::set_publics_in`].
    pub fn set_public(&mut self, id: VaultId, i: usize, number: u8, bytes: &[u8]) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if is_secret(r.kind, number) {
            return false;
        }
        set_field(r, number, bytes);
        s.touch(i);
        true
    }

    /// Sets a repeatable public field of record `i` of open vault `id` to
    /// `values`, in order, the values it had gone. `false` for a secret
    /// field or no such record. No check of the vault's size: a person's
    /// edit is [`OpenVaults::set_public_all_in`].
    pub fn set_public_all(&mut self, id: VaultId, i: usize, number: u8, values: &[&[u8]]) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if is_secret(r.kind, number) {
            return false;
        }
        r.fields.retain(|f| f.number != number);
        for v in values {
            r.push(number, v);
        }
        s.touch(i);
        true
    }

    /// Costs what [`OpenVaults::fits`] costs. Sets each of `fields` (a
    /// public field's number and bytes; empty takes it out) of record `i`
    /// of open vault `id` together, as [`OpenVaults::set_public`] does; or,
    /// when the vault would then not fit its size, none of them ([`NotSet`]).
    /// A secret field, no such record or a vault read-only is refused the
    /// same way.
    pub fn set_publics_in(
        &mut self,
        c: &Compute,
        id: VaultId,
        i: usize,
        fields: &[(u8, &[u8])],
    ) -> Result<(), NotSet> {
        self.edit_record_in(c, id, i, |r| {
            for &(number, bytes) in fields {
                if is_secret(r.kind, number) {
                    return Err(NotSet::Full);
                }
                set_field(r, number, bytes);
            }
            Ok(())
        })
    }

    /// Costs what [`OpenVaults::fits`] costs. Record `i` of open vault
    /// `id` as `edit` changes it, kept only when the vault then fits its
    /// size; otherwise nothing changes ([`NotSet`]).
    pub(super) fn edit_record_in(
        &mut self,
        c: &Compute,
        id: VaultId,
        i: usize,
        edit: impl FnOnce(&mut Record) -> Result<(), NotSet>,
    ) -> Result<(), NotSet> {
        let s = self.slot(id).filter(|s| !s.locked()).ok_or(NotSet::Full)?;
        let mut r = s.records.get(i).ok_or(NotSet::Full)?.duplicate();
        edit(&mut r)?;
        let edit = [(i, &r)];
        let ov = Overlay {
            remove: &[],
            add: &[],
            edit: &edit,
        };
        s.room(c, &self.names, &ov).map_err(NotSet::from)?;
        let s = self.slot_mut(id).ok_or(NotSet::Full)?;
        s.records[i] = r;
        s.touch_checked(i);
        Ok(())
    }

    /// Costs what [`OpenVaults::fits`] costs. Sets a repeatable public
    /// field of every record in `at` of open vault `id` to `values`, as
    /// [`OpenVaults::set_public_all`] does; or, when the vault would then
    /// not fit its size, none of them ([`NotSet`]).
    pub fn set_public_all_in(
        &mut self,
        c: &Compute,
        id: VaultId,
        at: &[usize],
        number: u8,
        values: &[&[u8]],
    ) -> Result<(), NotSet> {
        let s = self.slot(id).filter(|s| !s.locked()).ok_or(NotSet::Full)?;
        let mut grown: Vec<(usize, Record)> = Vec::new();
        for &i in at {
            if grown.iter().any(|(j, _)| *j == i) {
                continue;
            }
            let mut r = s.records.get(i).ok_or(NotSet::Full)?.duplicate();
            if is_secret(r.kind, number) {
                return Err(NotSet::Full);
            }
            r.fields.retain(|f| f.number != number);
            for v in values {
                r.push(number, v);
            }
            grown.push((i, r));
        }
        let edit: Vec<(usize, &Record)> = grown.iter().map(|(i, r)| (*i, r)).collect();
        let ov = Overlay {
            remove: &[],
            add: &[],
            edit: &edit,
        };
        s.room(c, &self.names, &ov).map_err(NotSet::from)?;
        let s = self.slot_mut(id).ok_or(NotSet::Full)?;
        for (i, r) in grown {
            s.records[i] = r;
            s.touch_checked(i);
        }
        Ok(())
    }

    /// Sets secret field `number` of record `i` of open vault `id` to
    /// input the caller holds (a typed password), which the caller wipes
    /// when it drops; an empty `bytes` takes the field out. `false` for a
    /// public field or no such record. No check of the vault's size: a
    /// person's edit is [`OpenVaults::set_fields`].
    pub fn set_secret(
        &mut self,
        id: VaultId,
        i: usize,
        number: u8,
        bytes: &Zeroizing<Vec<u8>>,
    ) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if !is_secret(r.kind, number) {
            return false;
        }
        set_field(r, number, bytes);
        s.touch(i);
        true
    }

    /// Sets whether key or wallet record `i` of open vault `id` loads at
    /// unlock, keeping its other flags.
    pub fn set_loads_at_unlock(&mut self, id: VaultId, i: usize, loads: bool) -> bool {
        let Some(s) = self.slot_mut(id) else {
            return false;
        };
        let Some(r) = s.records.get_mut(i) else {
            return false;
        };
        if !matches!(r.kind, kind::KEY | kind::WALLET) {
            return false;
        }
        records::set_loads_at_unlock(r, loads);
        s.touch(i);
        true
    }

    /// Costs what [`OpenVaults::fits`] costs. Writes a form into open
    /// vault `id`: record `at` (or, with `None` or a place with no
    /// record, a new record of type `kind`) gets each of `fields`, a field
    /// with no bytes taken out, as the caller typed them. A record that
    /// does not fit afterwards is left as it was ([`NotSet`]). Gives the
    /// record's place.
    pub fn set_fields(
        &mut self,
        c: &Compute,
        id: VaultId,
        at: Option<usize>,
        k: u8,
        fields: &[(u8, &[u8])],
    ) -> Result<usize, NotSet> {
        let s = self
            .slot(id)
            .filter(|s| !s.locked())
            .ok_or(NotSet::Unwritable)?;
        let place = at.filter(|&i| i < s.records.len());
        let mut record = match place {
            Some(i) => s.records[i].duplicate(),
            None => Record::new(k),
        };
        for (n, bytes) in fields {
            record.set(*n, bytes);
        }
        let emptied: Vec<u8> = fields
            .iter()
            .filter(|(_, b)| b.is_empty())
            .map(|(n, _)| *n)
            .collect();
        keep_required(&mut record, &emptied);
        let ov = match place {
            Some(i) => Overlay {
                remove: &[],
                add: &[],
                edit: &[(i, &record)],
            },
            None => Overlay {
                remove: &[],
                add: &[&record],
                edit: &[],
            },
        };
        s.room(c, &self.names, &ov).map_err(NotSet::from)?;
        let s = self.slot_mut(id).ok_or(NotSet::Unwritable)?;
        let i = match place {
            Some(i) => {
                s.records[i] = record;
                s.touch_checked(i);
                i
            }
            None => {
                s.records.push(record);
                s.origin.push(Origin::New);
                s.dirty.push(true);
                s.facts.push(OnceCell::new());
                s.unchecked.push(Cell::new(false));
                s.changed();
                s.records.len() - 1
            }
        };
        Ok(i)
    }

    /// Copies record `i` of open vault `from` into open vault `to`, last;
    /// its place there. With `without`, the copy lacks those fields (a
    /// key's stored passphrase, its codex32 set or BIP-85 origin). The secrets are copied here, and are
    /// never in the caller's hands.
    pub fn copy_record(
        &mut self,
        from: VaultId,
        i: usize,
        to: VaultId,
        without: &[u8],
    ) -> Option<usize> {
        let held = self.copy_out(from, i, without)?;
        self.insert(to, held)
    }

    /// A copy of record `i` of open vault `id`, held until it is dropped:
    /// readable as a [`RecordView`], and put into a vault with
    /// [`OpenVaults::insert`]. With `without`, the copy lacks those fields.
    pub fn copy_out(&self, id: VaultId, i: usize, without: &[u8]) -> Option<Held<Record>> {
        let mut r = self.slot(id)?.records.get(i)?.duplicate();
        r.fields.retain(|f| !without.contains(&f.number));
        Some(Held::of(r))
    }

    /// A copy of record `i` of open vault `id`, staged to go into a vault
    /// ([`OpenVaults::insert`]); with `without`, lacking those fields.
    pub fn stage(&self, id: VaultId, i: usize, without: &[u8]) -> Option<Staged> {
        self.copy_out(id, i, without).map(Staged::Held)
    }

    /// A passphrase record (type 14) for the key whose fingerprint is `fp`,
    /// made from the passphrase key record `i` of open vault `id` stores,
    /// staged to go into a vault. `None` for a record that stores none.
    pub fn stage_passphrase(&self, id: VaultId, i: usize, fp: &[u8; 4]) -> Option<Staged> {
        let r = self.slot(id)?.records.get(i)?;
        if r.kind != kind::KEY {
            return None;
        }
        let p = r.field(field::KEY_PASSPHRASE)?;
        Some(Staged::Held(Held::of(
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, fp)
                .with(field::PASSPHRASE, p),
        )))
    }

    // ---- for tests ----

    /// Open vault `id`'s records as they are held, for tests (feature
    /// `master-for-tests`, which no shipped binary has).
    #[cfg(feature = "master-for-tests")]
    pub fn contents_for_tests(&self, id: VaultId) -> &[Record] {
        match self.slot(id) {
            Some(s) => &s.records,
            None => &[],
        }
    }

    /// Open vault `id`'s records, to change, for tests (feature
    /// `master-for-tests`). The vault's version moves, as for any edit.
    #[cfg(feature = "master-for-tests")]
    pub fn contents_for_tests_mut(&mut self, id: VaultId) -> &mut Vec<Record> {
        let s = self
            .slots
            .iter_mut()
            .find(|s| s.id == id)
            .expect("an open vault");
        s.changed();
        s.dirty.iter_mut().for_each(|d| *d = true);
        s.facts.iter_mut().for_each(|f| *f = OnceCell::new());
        s.unchecked.iter().for_each(|u| u.set(true));
        &mut s.records
    }
}
