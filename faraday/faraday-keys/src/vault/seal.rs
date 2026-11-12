//! Opening and sealing a vault: unlock, the seal job and its keys, the seal state.

use super::*;

impl OpenVaults {
    pub(super) fn slot(&self, id: VaultId) -> Option<&Slot> {
        self.slots.iter().find(|s| s.id == id)
    }

    /// The slot to edit: none for an id not held or a read-only vault.
    pub(super) fn slot_mut(&mut self, id: VaultId) -> Option<&mut Slot> {
        self.slots
            .iter_mut()
            .find(|s| s.id == id)
            .filter(|s| !s.locked())
    }

    /// Sets the names new groups and titles take, for every vault. Set
    /// once, before the first edit.
    pub fn set_names(&mut self, names: Names) {
        self.names = names;
        for s in &mut self.slots {
            s.used = OnceCell::new();
            s.uuids = OnceCell::new();
        }
    }

    /// Costs an Argon2id run over the vault's memory, as
    /// `faraday_vault::file::open` does, and the reading of each GPG key.
    /// Opens `file` under `passphrase` and keeps it, with its records.
    /// Gives its id and the file's facts, which are public. The
    /// passphrase is lent, and the caller wipes its copy.
    pub fn unlock(
        &mut self,
        c: &Compute,
        file: Vec<u8>,
        passphrase: &str,
    ) -> Result<(VaultId, fv::file::Facts), fv::file::Error> {
        self.unlock_with(c, file, passphrase, fv::kdbx::Components::none())
    }

    /// Costs what [`OpenVaults::unlock`] costs. [`OpenVaults::unlock`] for a
    /// vault that also needs a key file's key or a YubiKey's secret
    /// (`docs/VAULT.md` §3.2, F208): the open vault keeps them for its
    /// seals, wiped as it closes, and never shows them.
    pub fn unlock_with(
        &mut self,
        c: &Compute,
        file: Vec<u8>,
        passphrase: &str,
        components: fv::kdbx::Components,
    ) -> Result<(VaultId, fv::file::Facts), fv::file::Error> {
        let mut opened = fv::file::open_with(
            c,
            &file,
            passphrase,
            components,
            &mut |_| Ok(()),
            &mut |_, _| Ok(None),
            None,
        )?;
        let built = build(&opened);
        let digest = faraday_crypto::sha256_whole(c, &file);
        // The file is not needed past its digest.
        drop(file);
        let base = base_of(&mut opened);
        Ok(self.keep(digest, base, Arc::new(opened), built))
    }

    /// Keeps the file a [`crate::job::StretchJob`] opened, on the thread
    /// that holds the store: no Argon2id run. Gives its id and the file's
    /// facts, or why the job could not open it. An answer dropped unkept
    /// is wiped.
    pub fn adopt(
        &mut self,
        stretched: crate::job::Stretched,
    ) -> Result<(VaultId, fv::file::Facts), fv::file::Error> {
        let slot = stretched.into_opened()?;
        let crate::job::OpenedSlot {
            identities: _,
            digest,
            base,
            opened,
            built,
        } = *slot;
        Ok(self.keep(digest, base, opened, built))
    }

    pub(super) fn keep(
        &mut self,
        digest: [u8; 32],
        base: usize,
        opened: Arc<fv::file::Opened>,
        built: Built,
    ) -> (VaultId, fv::file::Facts) {
        let facts = opened.facts().clone();
        let id = VaultId(self.next);
        self.next += 1;
        let Built {
            records,
            origin,
            unreadable,
        } = built;
        let records_len = records.len();
        self.slots.push(Slot {
            id,
            digest,
            base,
            opened,
            dirty: vec![false; records.len()],
            unchecked: (0..records.len()).map(|_| Cell::new(false)).collect(),
            records,
            origin,
            removed: Vec::new(),
            unreadable,
            version: 0,
            facts: (0..records_len).map(|_| OnceCell::new()).collect(),
            used: OnceCell::new(),
            uuids: OnceCell::new(),
            sealing: false,
            limit: self
                .limit
                .map_or(fv::file::CEILING, |l| l.min(fv::file::CEILING)),
            copy_room: Arc::clone(&self.copy_room),
            new_passphrase: None,
            new_step: None,
            new_key_file: None,
            key_taken: false,
            key_made: false,
            drop_yubikey: false,
        });
        (id, facts)
    }

    /// Lowers the most plaintext a seal may take, for the vaults held and
    /// the ones opened after, to `bytes` (never above the ceiling): a test
    /// reaches the limit of a vault without a gibibyte of contents. It
    /// only refuses more.
    pub fn lower_limit(&mut self, bytes: usize) {
        let bytes = bytes.min(fv::file::CEILING);
        self.limit = Some(bytes);
        for s in &mut self.slots {
            s.limit = bytes;
        }
    }

    /// A lookup. The most plaintext a seal of open vault `id` may take:
    /// the ceiling, or less where [`OpenVaults::lower_limit`] was told.
    /// `None` for a vault not held.
    pub fn limit_of(&self, id: VaultId) -> Option<usize> {
        Some(self.slot(id)?.limit())
    }

    /// Closes open vault `id`: its records are wiped.
    pub fn close(&mut self, id: VaultId) {
        self.slots.retain(|s| s.id != id);
    }

    /// Closes every open vault.
    pub fn clear(&mut self) {
        self.slots.clear();
        self.made_parts = None;
    }

    /// A lookup once read, which [`OpenVaults::warm`] does before a frame.
    /// How many bytes of `uuids` [`OpenVaults::seal_job`] takes for open
    /// vault `id`: 16 for each new entry and for each kind's group among
    /// them. `None` for an id this store does not hold. The first read after
    /// an edit works the seal's writes out, which clones each record
    /// changed (its bytes in wiping types) and counts the UUIDs they take;
    /// the answer is kept until the next edit.
    pub fn uuid_bytes(&self, id: VaultId) -> Option<usize> {
        let s = self.slot(id)?;
        Some(*s.uuids.get_or_init(|| {
            let mut n = 0;
            // A seal that refuses draws nothing: its job carries the refusal.
            let ok = s
                .ops(&Overlay::NONE, false, &mut || {
                    n += 1;
                    [0; 16]
                })
                .is_ok();
            if ok { n * 16 } else { 0 }
        }))
    }

    /// Makes the job that seals open vault `id`, which stays in the
    /// store, marked sealing (every edit refuses) until
    /// [`OpenVaults::seal_ended`]. The job's file is written with the changes: its changes written, the seal count one higher with the
    /// time `now` (Unix seconds, `None` when the clock is not set), under
    /// `random` and with the new entries' and groups' UUIDs from `uuids`
    /// ([`OpenVaults::uuid_bytes`] long). `None`, and the vault not
    /// marked, for an id this store does not hold, a read-only or already
    /// sealing vault or too few `uuids`.
    pub fn seal_job(
        &mut self,
        id: VaultId,
        random: &[u8; SEAL_RANDOM_LEN],
        uuids: &[u8],
        now: Option<u64>,
    ) -> Option<crate::job::SealJob> {
        let at = self.slots.iter().position(|s| s.id == id)?;
        let s = &self.slots[at];
        if s.locked() {
            return None;
        }
        let mut chunks = uuids.chunks_exact(16);
        let mut taken = 0;
        let mut short = false;
        let ops = s.ops(&Overlay::NONE, false, &mut || {
            taken += 1;
            match chunks.next().and_then(|u| <[u8; 16]>::try_from(u).ok()) {
                Some(u) => u,
                None => {
                    short = true;
                    [0; 16]
                }
            }
        });
        if short && ops.is_ok() {
            return None;
        }
        let rekey = (
            s.new_passphrase
                .as_ref()
                .map(|p| SecretText::of(p.as_str())),
            s.new_step,
        );
        // A key drawn and not yet taken as a document is not written: the
        // file keeps the components it had.
        let parts = (
            if s.key_pending() {
                None
            } else {
                s.new_key_file.clone()
            },
            s.drop_yubikey,
        );
        // A change the seal cannot write is its refusal, not a change left out.
        let ops = ops.map_err(fv::file::Error::from);
        let s = &mut self.slots[at];
        s.sealing = true;
        Some(crate::job::SealJob::new(
            Arc::clone(&s.opened),
            ops,
            self.names.clone(),
            rekey,
            parts,
            random,
            now,
        ))
    }

    /// Takes `passphrase` (moved in, wiped as it is replaced or the vault
    /// closes) as the one the next seal of open vault `id` writes, in place
    /// of the one the file was opened with. It counts as a change: the
    /// version moves. Refused, and the passphrase wiped, for an empty one,
    /// a vault this store does not hold and a read-only or sealing one.
    pub fn set_passphrase(&mut self, id: VaultId, passphrase: SecretText) -> bool {
        if passphrase.as_str().is_empty() {
            return false;
        }
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() {
            return false;
        }
        s.new_passphrase = Some(passphrase);
        s.changed();
        true
    }

    /// Takes `key` (moved in, wiped as it is replaced or the vault closes)
    /// as the key file's key the next seal of open vault `id` writes, to add
    /// a key file or replace the one the file has; `None` writes no key
    /// file. A key given is a chosen one: no document of it leaves
    /// ([`OpenVaults::make_key_file`] is the call for a new key). It counts
    /// as a change: the version moves. Refused (`false`, and `key` wiped)
    /// for an unknown vault, a read-only or sealing one, and for removing a
    /// key file the vault does not have. The change is the vault's once a
    /// seal that wrote it succeeds; a failed seal leaves the file's own
    /// components and this change pending.
    pub fn set_key_file(&mut self, id: VaultId, key: Option<Zeroizing<[u8; 32]>>) -> bool {
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() {
            return false;
        }
        s.key_made = false;
        s.key_taken = false;
        if key.is_none() && !s.opened.components_held().0 {
            // Nothing to take out: only a pending key file is taken back.
            if s.new_key_file.is_none() {
                return false;
            }
            s.new_key_file = None;
            s.changed();
            return true;
        }
        s.new_key_file = Some(key);
        s.changed();
        true
    }

    /// Costs a SHA-256 of the key file. Reads the key file `bytes` and
    /// takes its key as [`OpenVaults::set_key_file`] does; a refused file
    /// changes nothing.
    pub fn set_key_file_from(
        &mut self,
        id: VaultId,
        bytes: &[u8],
    ) -> Result<bool, fv::kdbx::keyfile::KeyFileError> {
        let key = fv::kdbx::keyfile::read(bytes)?;
        Ok(self.set_key_file(id, Some(key)))
    }

    /// Takes `random`, drawn by the caller for this and nothing else, as a
    /// new key file's key for the next seal of open vault `id` (add or
    /// replace), and records it as made: the only key whose KeePass
    /// document [`OpenVaults::key_file_document`] gives. Same refusals and
    /// the same change as [`OpenVaults::set_key_file`]. `random` must be
    /// fresh randomness (the store cannot check it). The key is written
    /// only once its document has been taken: until then a seal, a lock's
    /// included, writes the file under the components it had, and
    /// [`OpenVaults::rekeyed`], not the version, says whether a lock has a
    /// change to seal.
    pub fn make_key_file(&mut self, id: VaultId, random: &[u8; 32]) -> bool {
        if !self.set_key_file(id, Some(Zeroizing::new(*random))) {
            return false;
        }
        if let Some(s) = self.slots.iter_mut().find(|s| s.id == id) {
            s.key_made = true;
            s.key_taken = false;
        }
        true
    }

    /// Takes back a key file made for open vault `id` ([`OpenVaults::make_key_file`])
    /// whose document was not kept (it could not be, or its `.keyx` was
    /// taken out of the Outbox): the key is wiped and no key file is
    /// pending. `false` where the pending key file is not a made
    /// one (a chosen key or a removal stays), an unknown vault and a
    /// read-only or sealing one.
    pub fn unmake_key_file(&mut self, id: VaultId) -> bool {
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() || !s.key_made {
            return false;
        }
        s.key_made = false;
        s.key_taken = false;
        s.new_key_file = None;
        s.changed();
        true
    }

    /// Leaves the YubiKey's secret out of the next seal of open vault `id`:
    /// the file after it opens with the passphrase and any key file alone.
    /// It counts as a change. Refused for an unknown vault, a read-only or
    /// sealing one and one opened without a secret, which cannot be given
    /// one.
    pub fn remove_yubikey(&mut self, id: VaultId) -> bool {
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() || !s.opened.components_held().1 {
            return false;
        }
        s.drop_yubikey = true;
        s.changed();
        true
    }

    /// Takes back [`OpenVaults::remove_yubikey`]: the next seal of open
    /// vault `id` keeps the YubiKey's secret. It counts as a change. Same
    /// refusals, and `false` where no removal is pending.
    pub fn keep_yubikey(&mut self, id: VaultId) -> bool {
        let Some(s) = self.slots.iter_mut().find(|s| s.id == id) else {
            return false;
        };
        if s.locked() || !s.opened.components_held().1 || !s.drop_yubikey {
            return false;
        }
        s.drop_yubikey = false;
        s.changed();
        true
    }

    /// A lookup. Whether open vault `id` has a Remove YubiKey secret
    /// waiting for its seal ([`OpenVaults::remove_yubikey`]).
    pub fn yubikey_removal_pending(&self, id: VaultId) -> bool {
        self.slot(id).is_some_and(|s| s.drop_yubikey)
    }

    /// A lookup. Whether the next seal of open vault `id` writes a file
    /// that needs a key file and whether it needs a YubiKey's secret; `None`
    /// for an id this store does not hold.
    pub fn needs(&self, id: VaultId) -> Option<(bool, bool)> {
        let s = self.slot(id)?;
        let (key_file, yubikey) = s.opened.components_held();
        Some((
            if s.key_pending() {
                key_file
            } else {
                s.new_key_file.as_ref().map_or(key_file, Option::is_some)
            },
            yubikey && !s.drop_yubikey,
        ))
    }

    /// Reveal: the KeePass key file (`faraday_vault::kdbx::keyfile::write`)
    /// of the key file made for open vault `id` ([`OpenVaults::make_key_file`])
    /// that its next seal writes, for `why` (`Reveal::OutboxFile` of
    /// `Part::KeyFile`), to be written to a stick (`docs/KEYS.md` §2).
    /// Handed out once: the key is marked taken, and a second call gives
    /// `None` until another key is made. `None` for a key file chosen, one
    /// the vault was opened with, none pending, a vault being sealed, or a
    /// reason that names another part. Giving it lets the next seal write
    /// the key.
    pub fn key_file_document(
        &mut self,
        id: VaultId,
        why: Reveal,
    ) -> Option<Held<Zeroizing<Vec<u8>>>> {
        if !matches!(why, Reveal::OutboxFile(Part::KeyFile)) {
            return None;
        }
        let s = self.slots.iter_mut().find(|s| s.id == id)?;
        if s.sealing || !s.key_made || s.key_taken {
            return None;
        }
        let document = fv::kdbx::keyfile::write(s.new_key_file.as_ref()?.as_ref()?);
        s.key_taken = true;
        Some(Held::of(document))
    }

    /// Takes `memory_kib` as the memory step the next seal of open vault
    /// `id` writes, up or down (passes stay the file's or 3, over 4
    /// lanes). The step the file already has, with none pending, changes
    /// nothing, and it takes back another one pending. `Some(true)` where
    /// the pending step changed, which the caller records, as the store's
    /// calls do not move the open vault's change count a lock decides from.
    /// `None` for a step that is not one of [`fv::file::STEPS_KIB`], an
    /// unknown vault and a read-only or sealing one.
    pub fn set_step(&mut self, id: VaultId, memory_kib: u32) -> Option<bool> {
        if !fv::file::STEPS_KIB.contains(&memory_kib) {
            return None;
        }
        let s = self.slots.iter_mut().find(|s| s.id == id)?;
        if s.locked() {
            return None;
        }
        let have = s.opened.next_cost_at(None).memory_kib == memory_kib;
        let new_step = (!have).then_some(memory_kib);
        if s.new_step == new_step {
            return Some(false);
        }
        s.new_step = new_step;
        s.changed();
        Some(true)
    }

    /// A lookup. The cost the next seal of open vault `id` writes: its
    /// memory step is the one taken ([`OpenVaults::set_step`]) or else the
    /// file's own rounded up (§7.1, a file a KeePass app saved is written
    /// in Faraday's form). `None` for an id this store does not hold.
    pub fn next_cost(&self, id: VaultId) -> Option<fv::Cost> {
        let s = self.slot(id)?;
        Some(s.opened.next_cost_at(s.new_step))
    }

    /// A lookup. Whether open vault `id` has a memory step waiting for its
    /// seal ([`OpenVaults::set_step`]).
    pub fn has_step(&self, id: VaultId) -> bool {
        self.slot(id).is_some_and(|s| s.new_step.is_some())
    }

    /// Drops the memory step open vault `id` waits to seal with, so its
    /// next seal is written at the file's own: the changes stay.
    pub fn clear_step(&mut self, id: VaultId) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.id == id) {
            s.new_step = None;
        }
    }

    /// A lookup. Whether open vault `id` has a change of its key file
    /// waiting for the next seal (a key made, chosen or removed), made key
    /// or not.
    pub fn key_file_changing(&self, id: VaultId) -> bool {
        self.slot(id).is_some_and(|s| s.new_key_file.is_some())
    }

    /// A lookup. Whether open vault `id` is read-only or sealing, so no
    /// edit of it is taken: [`OpenVaults::unmake_key_file`] refuses it.
    pub fn is_locked(&self, id: VaultId) -> bool {
        self.slot(id).is_some_and(|s| s.locked())
    }

    /// A lookup. Whether the key file waiting in open vault `id` is one
    /// Faraday drew ([`OpenVaults::make_key_file`]) and not taken back.
    pub fn key_file_made(&self, id: VaultId) -> bool {
        self.slot(id).is_some_and(|s| s.key_made)
    }

    /// A lookup. Whether open vault `id` has a new passphrase, a new
    /// memory step or a change of its key file or YubiKey waiting for its
    /// seal: a change, so a lock seals it. A key file Faraday drew whose
    /// document is not yet taken is not one: no seal writes it.
    pub fn rekeyed(&self, id: VaultId) -> bool {
        self.slot(id).is_some_and(|s| {
            s.new_passphrase.is_some()
                || s.new_step.is_some()
                || (s.new_key_file.is_some() && !s.key_pending())
                || s.drop_yubikey
        })
    }

    /// A seal job of open vault `id` ended: with `ok` the vault is closed
    /// (its file was made); else it stays open with its changes and may
    /// be edited and sealed again.
    pub fn seal_ended(&mut self, id: VaultId, ok: bool) {
        if ok {
            self.slots.retain(|s| s.id != id);
        } else if let Some(s) = self.slots.iter_mut().find(|s| s.id == id) {
            s.sealing = false;
        }
    }

    /// A lookup. Open vault `id`'s identifier (the root group's UUID in
    /// hex) and its seal count with the time of the last seal, as the file
    /// was opened. `None` for an id this store does not hold.
    pub fn seal_state(&self, id: VaultId) -> Option<([u8; 32], Seals)> {
        let s = self.slot(id)?;
        let read = s.opened.read();
        Some((
            s.opened.vault_id(),
            Seals {
                count: read.seals,
                last: read.sealed.as_deref().and_then(unix_of_rfc3339),
            },
        ))
    }

    /// The digest of open vault `id`'s file as it was opened (worked out
    /// when it opened): what tells two seals of one count apart. `None`
    /// for an id this store does not hold.
    pub fn opened_digest(&self, id: VaultId) -> Option<[u8; 32]> {
        Some(self.slot(id)?.digest)
    }
}
