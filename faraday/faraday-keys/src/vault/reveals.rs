//! Reveals and what is worked out of key records: words facts, the session, entries, GPG and Secure Boot keys, loading keys, Codex32.

use super::*;

impl OpenVaults {
    // ---- facts worked out of the key records' words ----

    /// Costs a parse and a hash of every key record's words in open vault
    /// `id`, and a read of each record changed (for the seal's UUIDs), the
    /// first time after a change, a lookup after. Works out what
    /// `word_count`, `words_id` and `uuid_bytes` answer.
    pub fn warm(&self, c: &Compute, id: VaultId) {
        if let Some(s) = self.slot(id) {
            s.read_facts(c);
            let _ = self.uuid_bytes(id);
        }
    }

    /// Costs what `warm` costs. How many words key
    /// record `i` has; `None` for another record, or a key record that
    /// holds a seed rather than words.
    pub fn word_count(&self, c: &Compute, id: VaultId, i: usize) -> Option<usize> {
        self.slot(id)?.fact(c, i).map(|f| f.words)
    }

    /// Costs what `word_count` costs. The word list key record `i`'s
    /// words are in; `None` as for `word_count`. Public, kept with the
    /// count.
    pub fn language(
        &self,
        c: &Compute,
        id: VaultId,
        i: usize,
    ) -> Option<faraday_bip::bip39::Language> {
        self.slot(id)?.fact(c, i).map(|f| f.language)
    }

    /// Costs what `word_count` costs. What tells key record `i`'s words
    /// from other words ([`WordsId`]); `None` as for `word_count`. A
    /// lookup once the vault's key records are read.
    pub fn words_id(&self, c: &Compute, id: VaultId, i: usize) -> Option<&[u8; 32]> {
        self.slot(id)?.fact(c, i).map(|f| &*f.id)
    }

    /// Costs what `word_count` costs for a record in an open vault, a
    /// parse of the words for any other. How many words key record `r`'s
    /// seed has.
    pub fn record_word_count(&self, c: &Compute, r: RecordView<'_>) -> Option<usize> {
        match r.home() {
            Some((id, i)) if self.slot(id).is_some() => self.word_count(c, id, i),
            _ => record_words(c, r.raw()).map(|m| m.word_count()),
        }
    }

    /// Costs what `word_count` costs for a record in an open vault, a
    /// parse of the words for any other. The word list key record `r`'s
    /// words are in.
    pub fn record_language(
        &self,
        c: &Compute,
        r: RecordView<'_>,
    ) -> Option<faraday_bip::bip39::Language> {
        match r.home() {
            Some((id, i)) if self.slot(id).is_some() => self.language(c, id, i),
            _ => record_words(c, r.raw()).map(|m| m.language()),
        }
    }

    /// Costs what `words_id` costs for a record in an open vault, a parse
    /// of the words and a hash for any other. Key record `r`'s words'
    /// [`WordsId`].
    pub fn record_words_id(&self, c: &Compute, r: RecordView<'_>) -> Option<WordsId> {
        match r.home() {
            Some((id, i)) if self.slot(id).is_some() => {
                self.words_id(c, id, i).map(|x| Zeroizing::new(*x))
            }
            _ => record_words(c, r.raw()).map(|m| words_id(c, &m)),
        }
    }

    // ---- key records and the session ----

    /// Costs what `record_words_id` and `Keys::words_id` cost. Whether key
    /// record `r` holds the words of session key `key`, from whichever
    /// list: told by the hash of each one's entropy, all 32 bytes.
    pub fn same_words(&self, c: &Compute, keys: &Keys, r: RecordView<'_>, key: usize) -> bool {
        keys.words_id(c, key)
            .is_some_and(|id| self.record_words_id(c, r).is_some_and(|x| *x == *id))
    }

    /// Whether key record `r` stores the BIP-39 passphrase session key `k`
    /// has loaded with.
    pub fn stores_passphrase_of(&self, r: RecordView<'_>, keys: &Keys, k: usize) -> bool {
        r.raw()
            .text(field::KEY_PASSPHRASE)
            .is_some_and(|p| keys.passphrase_matches(k, p))
    }

    /// Whether passphrase record `r` (type 14) holds the passphrase of
    /// session key `k`; a record with none holds that of a key with none.
    pub fn passphrase_record_is(&self, r: RecordView<'_>, keys: &Keys, k: usize) -> bool {
        match r.raw().text(field::PASSPHRASE) {
            Some(t) => keys.passphrase_matches(k, t),
            None => keys.info(k).is_some_and(|key| !key.has_passphrase),
        }
    }

    /// Costs a parse of each open vault's key records' words (see
    /// `word_count`). Whether any key record of any open vault holds the
    /// words of session key `i`, as the payload a key record keeps.
    pub fn holds_session_words(&self, c: &Compute, keys: &Keys, i: usize) -> bool {
        let held: Vec<&[u8]> = self
            .slots
            .iter()
            .flat_map(|s| s.records.iter().filter(|r| r.kind == kind::KEY))
            .filter_map(|r| r.field(field::KEY))
            .collect();
        keys.words_payload_among(c, i, &held)
    }

    /// Costs a parse of the record's words and, with `accept`, a PBKDF2
    /// and a master key for the fingerprint, then what `add_mnemonic`
    /// costs. Loads key record `at` of open vault `id` into `keys` with
    /// `passphrase`, in place of the one stored with it. With `accept`,
    /// the key it gives is loaded only when `accept` takes its
    /// fingerprint; otherwise the fingerprint is handed back
    /// ([`KeyLoad::Refused`]) and nothing loads. `None` where the record
    /// holds no words.
    pub fn load_key_with_passphrase(
        &self,
        c: &Compute,
        keys: &mut Keys,
        at: (VaultId, usize),
        passphrase: &str,
        accept: Option<&dyn Fn(Fingerprint) -> bool>,
    ) -> Option<KeyLoad> {
        let r = self.slot(at.0)?.records.get(at.1)?;
        if r.kind != kind::KEY {
            return None;
        }
        let m = record_words(c, r)?;
        if let Some(accept) = accept {
            let fp = fingerprint(c, Material::Words(&m), passphrase, keys.network())?;
            if !accept(fp) {
                return Some(KeyLoad::Refused(fp));
            }
        }
        let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
        let loaded = keys.add_mnemonic(c, &m, passphrase, &label, None);
        if loaded.is_ok() {
            keys.set_child_of(keys.len() - 1, child_of_of(r));
        }
        Some(KeyLoad::Done(loaded))
    }

    /// Costs a parse of the words. Key record `r`'s words, held until
    /// they are dropped, for the reason `why` (a SeedQR picture, a file to
    /// a stick, the words on screen). `None` for a record that holds a
    /// seed rather than words.
    pub fn words_of(&self, c: &Compute, r: RecordView<'_>, _why: Reveal) -> Option<Held<Mnemonic>> {
        record_words(c, r.raw()).map(Held::of)
    }

    /// Costs what `words_of` costs for words. Field `number` of record `i`
    /// of open vault `id`, lent while the guard lives, for the reason
    /// `why`: a key's words (a parse of them) or stored passphrase, a
    /// passphrase record's passphrase, a note's text, an entry's
    /// password, TOTP secret or notes. Empty where the field is not
    /// secret, is not there, or `why` names another part.
    pub fn reveal(
        &self,
        c: &Compute,
        id: VaultId,
        i: usize,
        number: u8,
        why: Reveal,
    ) -> Revealed<'_> {
        let Some(r) = self.slot(id).and_then(|s| s.records.get(i)) else {
            return Revealed::lent("");
        };
        if !is_secret(r.kind, number) {
            return Revealed::lent("");
        }
        let Some(part) = why.part() else {
            return Revealed::lent("");
        };
        let lent = |p: Part| -> &str {
            if part == p {
                r.text(number).unwrap_or("")
            } else {
                ""
            }
        };
        match (r.kind, number) {
            (kind::KEY, field::KEY) if matches!(part, Part::Words | Part::SeedQr) => {
                match record_words(c, r) {
                    Some(m) => Revealed::made(phrase_of(&m)),
                    None => Revealed::lent(""),
                }
            }
            (kind::KEY, field::KEY_PASSPHRASE) | (kind::PASSPHRASE, field::PASSPHRASE) => {
                Revealed::lent(lent(Part::Passphrase))
            }
            (kind::NOTE, field::NOTE) => Revealed::lent(lent(Part::Note)),
            (kind::ENTRY, field::PASSWORD) => Revealed::lent(lent(Part::Password)),
            (kind::ENTRY, field::NOTES) => Revealed::lent(lent(Part::EntryNotes)),
            (kind::ENTRY, field::TOTP) => Revealed::lent(lent(Part::Totp)),
            _ => Revealed::lent(""),
        }
    }

    // ---- entries ----

    /// Cheap: one HMAC. The code entry `i` of open vault `id` gives at
    /// `now`, and the account its TOTP secret's label names; `None` for a
    /// record with no TOTP secret. `now` is `None` while the clock is not
    /// checked: `Ok(None)` where the secret reads and there is no code to
    /// show yet. The code is revealed for `VaultEntry(Part::Totp)`.
    pub fn totp(&self, id: VaultId, i: usize, now: Option<u64>, _why: Reveal) -> Option<Totp> {
        let s = self.slot(id)?;
        let r = s.records.get(i)?;
        if r.kind != kind::ENTRY {
            return None;
        }
        let account = |t: &str| {
            fv::entries::otpauth_names(t)
                .map(|(_, a)| a)
                .filter(|a| !a.is_empty())
        };
        // An entry as read has its TOTP worked out over every field it
        // may be in; one changed here is read from its `otp` field, and
        // from the same fields as when read where it has none (Faraday
        // writes only `otp` and never rewrites a field it did not make).
        // An `otp` field taken out here leaves no code: the entry is no
        // longer read from its other fields.
        let had_otp = |item: usize| {
            s.opened.read().items.get(item).is_some_and(
                |it| matches!(&it.body, Body::Record(o) if o.field(field::TOTP).is_some()),
            )
        };
        let own = s.dirty[i]
            && (r.field(field::TOTP).is_some()
                || matches!(s.origin.get(i), Some(Origin::Entry { item, .. }) if had_otp(*item)));
        if let (false, Some(Origin::Entry { item, .. })) = (own, s.origin.get(i)) {
            let read = s.opened.read().items.get(*item)?.totp.as_ref()?;
            return Some(Totp {
                account: r.text(field::TOTP).and_then(account),
                code: match read {
                    Ok(t) => Ok(crate::totp::shown_of(t, now)),
                    Err(e) => Err(*e),
                },
            });
        }
        let t = r.text(field::TOTP)?;
        Some(Totp {
            account: account(t),
            code: crate::totp::shown(t, now),
        })
    }

    /// A vault holding one item to write on another thread, for a KeePass
    /// app (`docs/VAULT.md` §8): record `at` as it is kept, a person's
    /// entry in the root group, in a new
    /// vault `id` named `label`, under `passphrase`, moved in, at `cost`,
    /// with `random` for the seal and `uuids` for the entry and its group.
    /// A line of the notes that is a BIP-85 path is left out, so a password
    /// BIP-85 made goes without its index (F172), and the title is `title`.
    /// `None` where the record is no entry: a key, a seed or any other kind
    /// stays in the vault (F70).
    #[expect(
        clippy::too_many_arguments,
        reason = "each is a value the new file states; the caller draws them all"
    )]
    pub fn kdbx_job(
        &self,
        at: (VaultId, usize),
        title: &str,
        (id, label, now): ([u8; 16], &str, Option<u64>),
        passphrase: faraday_intake::secret_buf::SecretText,
        cost: fv::Cost,
        random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
        uuids: [u8; 32],
    ) -> Option<crate::job::KdbxJob> {
        let (vid, i) = at;
        let r = self.slot(vid)?.records.get(i)?;
        if r.kind != kind::ENTRY {
            return None;
        }
        let mut record = r.clone();
        record.set(field::TITLE, title.as_bytes());
        if let Some(n) = r.text(field::NOTES) {
            let kept: String = n
                .split_inclusive('\n')
                .filter(|l| !l.trim_start().starts_with(BIP85_PATH))
                .collect();
            record.set(field::NOTES, kept.as_bytes());
        }
        body_of(&record)?;
        let mut u = [0u8; 16];
        let mut g = [0u8; 16];
        u.copy_from_slice(&uuids[..16]);
        g.copy_from_slice(&uuids[16..]);
        Some(crate::job::KdbxJob::new(
            record,
            self.names.clone(),
            (id, label.into(), now),
            (u, g),
            passphrase,
            cost,
            random,
        ))
    }

    // ---- GPG keys ----

    /// Costs Ed25519 signatures. The public certificate (packets) of GPG
    /// record `i` of open vault `id`, signed at `at`, with its expiry
    /// `expiry` when given.
    pub fn gpg_certificate(
        &self,
        id: VaultId,
        i: usize,
        at: u32,
        expiry: Option<u32>,
    ) -> Option<Vec<u8>> {
        let mut key = gpg_key(self.slot(id)?.records.get(i)?)?;
        if let Some(e) = expiry {
            key.expiry = e;
        }
        Some(crate::gpg::certificate(&key, at))
    }

    /// Costs an Ed25519 signature and what [`OpenVaults::fits`] costs.
    /// Makes the revocation certificate GPG record `i` of open vault `id`
    /// keeps, signed at `at` with no reason, and keeps it in the record,
    /// as GnuPG keeps one at key generation. `Ok(false)` for a record that
    /// is no GPG key or keeps one already; `Err` where the vault would not
    /// fit its size with it, and nothing is kept ([`NotSet`]).
    pub fn gpg_keep_revocation(
        &mut self,
        c: &Compute,
        id: VaultId,
        i: usize,
        at: u32,
    ) -> Result<bool, NotSet> {
        let Some(r) = self.slot(id).and_then(|s| s.records.get(i)) else {
            return Ok(false);
        };
        if r.field(gpg_field::REVOCATION).is_some() {
            return Ok(false);
        }
        let Some(key) = gpg_key(r) else {
            return Ok(false);
        };
        let rev = Zeroizing::new(crate::gpg::revocation(
            &key,
            at,
            faraday_pgp::Reason::Unspecified,
            "",
        ));
        self.edit_record_in(c, id, i, |r| {
            r.set(gpg_field::REVOCATION, &rev);
            Ok(())
        })?;
        Ok(true)
    }

    /// Costs what `gpg_key` costs. The revocation certificate GPG record
    /// `i` of open vault `id` keeps, as the file GnuPG keeps it in, for `why` (`Reveal::OutboxFile` of
    /// `Part::Revocation`). `None` for a record that keeps none, or a
    /// reason that names another part.
    pub fn gpg_revocation_file(
        &self,
        id: VaultId,
        i: usize,
        why: Reveal,
    ) -> Option<Held<Zeroizing<String>>> {
        if why.part() != Some(Part::Revocation) {
            return None;
        }
        let r = self.slot(id)?.records.get(i)?;
        let rev = r.field(gpg_field::REVOCATION)?;
        let key = gpg_key(r)?;
        Some(Held::of(Zeroizing::new(crate::gpg::revocation_file(
            &key, rev,
        ))))
    }

    /// Costs an Ed25519 signature. A new revocation certificate for GPG
    /// record `i` of open vault `id`, signed at `at` for `reason` and
    /// `text` (`gpg --generate-revocation`), as the file GnuPG writes, for
    /// `why` (`Reveal::OutboxFile` of `Part::Revocation`). The kept one
    /// is not replaced. `None` for a record that is no GPG key, or a
    /// reason that names another part.
    pub fn gpg_new_revocation_file(
        &self,
        id: VaultId,
        i: usize,
        at: u32,
        reason: faraday_pgp::Reason,
        text: &str,
        why: Reveal,
    ) -> Option<Held<Zeroizing<String>>> {
        if why.part() != Some(Part::Revocation) {
            return None;
        }
        let key = gpg_key(self.slot(id)?.records.get(i)?)?;
        let rev = Zeroizing::new(crate::gpg::revocation(&key, at, reason, text));
        Some(Held::of(Zeroizing::new(crate::gpg::revocation_file(
            &key, &rev,
        ))))
    }

    /// Costs an Ed25519 signature. The signature over `data` by GPG record
    /// `i` of open vault `id`'s signing subkey, at `at`, armoured.
    pub fn gpg_sign_armored(&self, id: VaultId, i: usize, data: &[u8], at: u32) -> Option<String> {
        let key = gpg_key(self.slot(id)?.records.get(i)?)?;
        Some(crate::gpg::sign_armored(&key, data, at))
    }

    /// Costs what `gpg_key` costs and a paperkey encoding. The paperkey
    /// text of GPG record `r`: secret, made only to be shown,
    /// wiped when dropped (`Reveal::GpgPaperkey`).
    pub fn gpg_paperkey(&self, r: RecordView<'_>, _why: Reveal) -> Option<Held<Zeroizing<String>>> {
        let key = gpg_key(r.raw())?;
        Some(Held::of(crate::gpg::paperkey(&key)))
    }

    // ---- Secure Boot keys ----

    /// Costs RSA signatures. The enrolment files, by name, for Secure Boot
    /// record `i` of open vault `id` under `policy`, the signing blinded
    /// from `blind_seed`. `None` for a record that is no readable set of
    /// owner keys.
    pub fn sb_enrolment(
        &self,
        id: VaultId,
        i: usize,
        blind_seed: &[u8; 32],
        policy: faraday_sb::Policy,
        now: u64,
    ) -> Option<Result<Enrolment, faraday_sb::Error>> {
        let keys = sb_keys(self.slot(id)?.records.get(i)?)?;
        Some(crate::secure_boot::enrolment(
            blind_seed, &keys, policy, now,
        ))
    }

    /// Costs a SHA-256 of `pe` and an RSA signature. The boot program `pe`
    /// signed by the db key of Secure Boot record `i` of open vault `id`,
    /// the signing blinded from `blind_seed`, only when it is the file the
    /// checked `release` names (F134). `None` for a record that is no
    /// readable set of owner keys.
    pub fn sb_sign_image(
        &self,
        id: VaultId,
        i: usize,
        blind_seed: &[u8; 32],
        release: &faraday_sb::release::Release,
        pe: &[u8],
    ) -> Option<Result<Vec<u8>, faraday_sb::Error>> {
        let keys = sb_keys(self.slot(id)?.records.get(i)?)?;
        Some(crate::secure_boot::sign_image(
            blind_seed, pe, &keys, release,
        ))
    }

    // ---- loading keys into the session ----

    /// Costs a parse of each key record's words in `set`, and what
    /// `add_words_with` and `add_seed` cost for each. Loads the key
    /// records of open vault `id` whose places `set` names into `keys`:
    /// the words (with the passphrase stored beside them, or the one an
    /// open vault keeps alone for them that `chosen` names for the
    /// record's place, the first by default) or the master seed. Returns the
    /// fingerprints of the keys that loaded; a key the store refuses is
    /// left out.
    pub fn load_from_vault(
        &self,
        c: &Compute,
        keys: &mut Keys,
        id: VaultId,
        set: &BTreeSet<usize>,
        chosen: &std::collections::BTreeMap<usize, usize>,
    ) -> Vec<Fingerprint> {
        let Some(slot) = self.slot(id) else {
            return Vec::new();
        };
        type Set = Option<fv::codex32::SetFacts>;
        type Child = Option<fv::child::ChildOf>;
        let mut words: Vec<(Zeroizing<String>, String, Set, Child)> = Vec::new();
        let mut seeds: Vec<(Zeroizing<Vec<u8>>, String, Set)> = Vec::new();
        let mut passphrases: Vec<Zeroizing<String>> = Vec::new();
        for (i, r) in slot
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.kind == kind::KEY)
        {
            if !set.contains(&i) {
                continue;
            }
            let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
            let set = codex32_set_of(r);
            if let Some(m) = record_words(c, r) {
                // Stored with the key, or kept alone in an open vault.
                let pass = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self
                        .passphrase_kept_for(c, keys, &m, chosen.get(&i).copied().unwrap_or(0))
                        .unwrap_or_default(),
                };
                words.push((phrase_of(&m), label, set, child_of_of(r)));
                passphrases.push(pass);
            } else if let Some(seed) = seed_of(r) {
                seeds.push((seed, label, set));
            }
        }
        let mut loaded = Vec::new();
        for ((w, label, set, child), p) in words.iter().zip(&passphrases) {
            if let Ok(fp) = keys.add_words_with(c, w, p, label, None) {
                keys.set_codex32(keys.len() - 1, set.clone());
                keys.set_child_of(keys.len() - 1, *child);
                loaded.push(fp);
            }
        }
        for (seed, label, set) in &seeds {
            if let Ok(fp) = keys.add_seed(c, seed, label) {
                keys.set_codex32(keys.len() - 1, set.clone());
                loaded.push(fp);
            }
        }
        loaded
    }

    /// Costs a parse of the record's words, and what `add_mnemonic` or
    /// `add_seed` costs. Loads key record `at` of open vault `id` into
    /// `keys`. `None` where the record is no key record, or holds neither
    /// words nor a seed.
    pub fn load_key_record(
        &self,
        c: &Compute,
        keys: &mut Keys,
        id: VaultId,
        at: usize,
    ) -> Option<Result<Fingerprint, Refusal>> {
        let r = self.slot(id)?.records.get(at)?;
        if r.kind != kind::KEY {
            return None;
        }
        let label = r.text(field::KEY_LABEL).unwrap_or("").to_string();
        let loaded = match record_words(c, r) {
            Some(m) => {
                // Stored with the key, or kept alone in an open vault
                // (`docs/BACKUP-FLOW.md` §17.12 item 4).
                let p = match r.text(field::KEY_PASSPHRASE) {
                    Some(p) => Zeroizing::new(p.to_string()),
                    None => self.passphrase_kept_for(c, keys, &m, 0).unwrap_or_default(),
                };
                Some(keys.add_mnemonic(c, &m, &p, &label, None))
            }
            None => seed_of(r).map(|seed| keys.add_seed(c, &seed, &label)),
        };
        if matches!(loaded, Some(Ok(_))) {
            keys.set_codex32(keys.len() - 1, codex32_set_of(r));
            keys.set_child_of(keys.len() - 1, child_of_of(r));
        }
        loaded
    }

    /// Costs a parse of each codex32 share record's string and, for each
    /// set enough of them make, an interpolation, an HMAC-SHA512 and what
    /// `add_seed` costs. The strings the share records of every open vault
    /// keep (`docs/VAULT.md` §7 type 15), gathered towards their keys
    /// (`docs/CODEX32.md` §9.5, Getting the seed back), as the codex32 form
    /// gathers typed ones: one set, distinct indices. A key the session
    /// holds keeps the strings of its set ([`Keys::keep_codex32`]). A key
    /// it does not hold loads once `k` strings of one set are in across
    /// the open vaults and the seed they make has the fingerprint the
    /// records name; its set's facts are the strings found. Returns the
    /// fingerprints of the keys loaded.
    pub fn gather_codex32(&self, c: &Compute, keys: &mut Keys) -> Vec<Fingerprint> {
        use faraday_bip::codex32::Codex32;
        use fv::codex32::{SetFacts, State};
        // Each set, by the fingerprint the records name and its
        // identifier: its strings, one per index, and the other strings
        // of an index it has more than one of.
        type Strings = Vec<(Codex32, Zeroizing<String>)>;
        type Set = ([u8; 4], Strings, Strings);
        let mut sets: Vec<Set> = Vec::new();
        for slot in &self.slots {
            for r in slot
                .records
                .iter()
                .filter(|r| r.kind == kind::CODEX32_SHARE)
            {
                let (Some(text), Some(fp)) = (
                    r.text(field::CODEX32_STRING),
                    r.field(field::CODEX32_KEY)
                        .and_then(|b| <[u8; 4]>::try_from(b).ok()),
                ) else {
                    continue;
                };
                let Ok(share) = Codex32::parse(text) else {
                    continue;
                };
                let at = match sets.iter().position(|(f, list, _)| {
                    *f == fp
                        && list
                            .first()
                            .is_some_and(|(x, _)| x.identifier() == share.identifier())
                }) {
                    Some(at) => at,
                    None => {
                        sets.push((fp, Vec::new(), Vec::new()));
                        sets.len() - 1
                    }
                };
                let (_, list, others) = &mut sets[at];
                let text = Zeroizing::new(text.to_string());
                let same = |(x, t): &(Codex32, Zeroizing<String>)| {
                    x.share_index() == share.share_index() && t.eq_ignore_ascii_case(&text)
                };
                if list.iter().any(&same) || others.iter().any(&same) {
                    continue;
                }
                if list
                    .iter()
                    .any(|(x, _)| x.share_index() == share.share_index())
                {
                    others.push((share, text));
                } else {
                    list.push((share, text));
                }
            }
        }
        let mut loaded = Vec::new();
        for (fp, list, others) in sets {
            if let Some(k) = keys.position_of(fp) {
                let texts: Vec<&str> = list
                    .iter()
                    .chain(&others)
                    .map(|(_, t)| t.as_str())
                    .collect();
                keys.keep_codex32(c, k, &texts);
                continue;
            }
            let Some((first, _)) = list.first() else {
                continue;
            };
            let need = usize::from(first.threshold());
            if need < 2 || list.len() < need {
                continue;
            }
            let Some((seed, texts)) = codex32_pick(c, fp, need, &list, &others) else {
                continue;
            };
            let Ok(got) = keys.add_seed(c, &seed, "") else {
                continue;
            };
            let at = keys.len() - 1;
            let facts = SetFacts {
                threshold: first.threshold(),
                identifier: first.identifier(),
                length: u8::try_from(texts.first().map_or(0, |t| t.len())).unwrap_or(0),
                unknown: 0,
                strings: list
                    .iter()
                    .map(|(x, _)| (x.share_index(), State::Placed))
                    .collect(),
            };
            keys.set_codex32(at, Some(facts));
            keys.keep_codex32(c, at, &texts);
            loaded.push(got);
        }
        loaded
    }

    /// Costs a parse of each codex32 share record's string. The share strings
    /// every open vault keeps of the set `typed` is of (its identifier,
    /// threshold and length), at indices none of `typed` has, as their
    /// text (`docs/CODEX32.md` §9.5), grouped by the key each record
    /// names: one group per fingerprint, every distinct string in it
    /// (an index may have two, and the right one is found by the seed
    /// they make). Strings of two seeds may share an identifier, a
    /// threshold and a length; only one group's strings are ever
    /// combined. A record that names no key is in none. None where
    /// `typed` is empty or the secret alone.
    fn codex32_more_of(
        &self,
        typed: &[faraday_bip::codex32::Codex32],
    ) -> Vec<([u8; 4], Vec<Zeroizing<String>>)> {
        use faraday_bip::codex32::Codex32;
        let Some(first) = typed.first().filter(|f| f.threshold() >= 2) else {
            return Vec::new();
        };
        // Each key's strings, by fingerprint.
        type Group = ([u8; 4], Vec<Zeroizing<String>>);
        let mut out: Vec<Group> = Vec::new();
        for r in self.slots.iter().flat_map(|s| s.records.iter()) {
            if r.kind != kind::CODEX32_SHARE {
                continue;
            }
            let (Some(text), Some(fp)) = (
                r.text(field::CODEX32_STRING),
                r.field(field::CODEX32_KEY)
                    .and_then(|b| <[u8; 4]>::try_from(b).ok()),
            ) else {
                continue;
            };
            let Ok(s) = Codex32::parse(text) else {
                continue;
            };
            let i = s.share_index();
            let same = s.identifier() == first.identifier()
                && s.threshold() == first.threshold()
                && s.payload().len() == first.payload().len();
            if !same || s.is_secret() || typed.iter().any(|t| t.share_index() == i) {
                continue;
            }
            let at = match out.iter().position(|(f, _)| *f == fp) {
                Some(at) => at,
                None => {
                    out.push((fp, Vec::new()));
                    out.len() - 1
                }
            };
            let group = &mut out[at].1;
            if !group.iter().any(|t| t.eq_ignore_ascii_case(text)) {
                group.push(Zeroizing::new(text.to_ascii_lowercase()));
            }
        }
        out
    }

    /// Costs what `Keys::codex32_of_seed` costs, and a parse of each
    /// codex32 share record's string. Whether `string`, a share whose
    /// header is that of key `i`'s codex32 set, is of key `i`'s seed
    /// (`docs/CODEX32.md` §9.5), with the strings of its set this session
    /// holds for the key and the open vaults keep: `None` where fewer than
    /// `k` are here to rebuild it.
    pub fn codex32_of_seed(
        &self,
        c: &Compute,
        keys: &Keys,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
    ) -> Option<bool> {
        let more = self.codex32_of_key(keys, i, string)?;
        keys.codex32_of_seed(c, i, string, &more)
    }

    /// Costs what `Keys::keep_codex32` costs, and a parse of each codex32
    /// share record's string. Keeps `string`, a share whose header is
    /// that of key `i`'s codex32 set, with key `i` where it is proved of
    /// the key's seed from the strings this session holds for the key
    /// and the open vaults keep (`docs/CODEX32.md` §9.5); a string not
    /// proved is not kept. Whether it was kept.
    pub fn keep_codex32(
        &self,
        c: &Compute,
        keys: &mut Keys,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
    ) -> bool {
        let Some(more) = self.codex32_of_key(keys, i, string) else {
            return false;
        };
        let text = Zeroizing::new(string.encode().as_str().to_string());
        keys.keep_codex32_among(c, i, &[text.as_str()], &more) > 0
    }

    /// Costs a parse of each codex32 share record's string. The open vaults'
    /// strings of the set `string` is of that belong to key `i`'s own
    /// seed: strings of the same set header kept for another seed never
    /// join it. `None` for no such key.
    fn codex32_of_key(
        &self,
        keys: &Keys,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
    ) -> Option<Vec<Zeroizing<String>>> {
        let fp = keys.info(i)?.fingerprint.0;
        Some(
            self.codex32_more_of(std::slice::from_ref(string))
                .into_iter()
                .find(|(f, _)| *f == fp)
                .map(|(_, g)| g)
                .unwrap_or_default(),
        )
    }

    /// Costs a parse of each codex32 share record's string. How many strings
    /// of the set `typed` is of the open vaults add to it: at indices
    /// none of `typed` has, one per index, of the one key whose records
    /// hold the most (`docs/CODEX32.md` §9.5). Public: a count.
    pub fn codex32_count(&self, typed: &[faraday_bip::codex32::Codex32]) -> usize {
        self.codex32_more_of(typed)
            .iter()
            .map(|(_, g)| {
                let mut at: Vec<u8> = g
                    .iter()
                    .filter_map(|t| t.as_bytes().get(8).copied())
                    .collect();
                at.sort_unstable();
                at.dedup();
                at.len()
            })
            .max()
            .unwrap_or(0)
    }

    /// Costs a parse of each codex32 share record's string and, for each
    /// key whose records make up `k` with those typed, and each of at
    /// most 16 choices among its strings, an interpolation, an HMAC-SHA512
    /// and a fingerprint, and what `add_seed` costs.
    /// Strings typed into the codex32 form gathered with the open vaults'
    /// strings of their set (`docs/CODEX32.md` §9.5), one key's records
    /// at a time: once `typed` and those make `k`, and the seed they make
    /// has the fingerprint the records name, it loads as a key labelled
    /// `label`, its set's facts the indices of both and its strings kept
    /// ([`Keys::keep_codex32`]). `None` while fewer than `k` are in; a
    /// refusal where no key's records make their own key with those
    /// typed, or the key does not load.
    pub fn codex32_with_typed(
        &self,
        c: &Compute,
        keys: &mut Keys,
        typed: &[faraday_bip::codex32::Codex32],
        label: &str,
    ) -> Option<Result<Fingerprint, Refusal>> {
        use fv::codex32::{SetFacts, State};
        let first = typed.first()?;
        let need = usize::from(first.threshold());
        if need < 2 || typed.len() >= need {
            return None;
        }
        // Each key's records with those typed, until one makes the key
        // its records name; never strings of two keys together.
        let typed_texts: Vec<Zeroizing<String>> = typed
            .iter()
            .map(|s| Zeroizing::new(s.encode().as_str().to_string()))
            .collect();
        let fixed: Vec<&str> = typed_texts.iter().map(|t| t.as_str()).collect();
        let mut tried = false;
        let mut found = None;
        for (fp, more) in self.codex32_more_of(typed) {
            let pool: Vec<&str> = more.iter().map(|t| t.as_str()).collect();
            let Some(made) = codex32_find(c, fp, need, &fixed, &pool) else {
                continue;
            };
            tried = true;
            if let Some((seed, picked)) = made {
                // What made the key, then the key's other strings at
                // indices none of those has.
                let mut texts = typed_texts.clone();
                texts.extend(picked.into_iter().skip(typed.len()));
                for t in more.iter() {
                    let at = t.as_bytes().get(8);
                    if !texts.iter().any(|x| x.as_bytes().get(8) == at) {
                        texts.push(t.clone());
                    }
                }
                found = Some((texts, seed));
                break;
            }
        }
        let Some((texts, seed)) = found else {
            return tried.then(|| {
                Err(Refusal::Words(
                    faraday_strings::EN.c32_vault_other_seed.to_string(),
                ))
            });
        };
        let fp = match keys.add_seed(c, &seed, label) {
            Ok(fp) => fp,
            Err(e) => return Some(Err(e)),
        };
        let at = keys.len() - 1;
        // Those typed are on paper; those in a vault are placed there.
        let strings = texts
            .iter()
            .enumerate()
            .filter_map(|(n, t)| Some((*t.as_bytes().get(8)?, n)))
            .map(|(i, n)| {
                (
                    i,
                    if n < typed.len() {
                        State::Made
                    } else {
                        State::Placed
                    },
                )
            })
            .collect();
        keys.set_codex32(
            at,
            Some(SetFacts {
                threshold: first.threshold(),
                identifier: first.identifier(),
                length: u8::try_from(texts[0].len()).unwrap_or(0),
                unknown: 0,
                strings,
            }),
        );
        let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
        keys.keep_codex32(c, at, &refs);
        Some(Ok(fp))
    }

    /// Whether open vault `id` keeps a codex32 share record of the key
    /// with fingerprint `fp` at share index `index` (a bech32 character,
    /// lower case) of the set `set` (its threshold and identifier): a
    /// string of another set of the same key is not it. A lookup.
    pub fn holds_codex32(&self, id: VaultId, fp: [u8; 4], set: (u8, [u8; 4]), index: u8) -> bool {
        self.slot(id).is_some_and(|s| {
            s.records
                .iter()
                .any(|r| codex32_share_is(RecordView::of(r), fp, set, Some(index)))
        })
    }

    /// Costs a fingerprint for each passphrase record. The passphrase
    /// records (type 14) the open vaults keep alone for the key the words
    /// `m` make with them, found by the fingerprint each names: the vault
    /// and the place, in the order the vaults are held.
    pub fn passphrases_kept_for(
        &self,
        c: &Compute,
        keys: &Keys,
        m: &Mnemonic,
    ) -> Vec<(VaultId, usize)> {
        let mut found = Vec::new();
        for slot in &self.slots {
            for (i, r) in slot
                .records
                .iter()
                .enumerate()
                .filter(|(_, r)| r.kind == kind::PASSPHRASE)
            {
                let (Some(fp), Some(p)) =
                    (r.field(field::PASSPHRASE_KEY), r.text(field::PASSPHRASE))
                else {
                    continue;
                };
                let Some(got) = fingerprint(c, Material::Words(m), p, keys.network()) else {
                    continue;
                };
                if got.0[..] == *fp {
                    found.push((slot.id, i));
                }
            }
        }
        found
    }

    /// A lookup. What open vault `id` already holds that `r` would be
    /// related to (a codex32 share of the same threshold, identifier, index
    /// and key): false where `r` is no share, no other share of its set is
    /// held, or the same share is.
    pub fn relates(&self, id: VaultId, r: &impl Fit) -> bool {
        let Some(s) = self.slot(id) else {
            return false;
        };
        let r = r.record_of(Inside(()));
        let Some(key) = share_set_of(r) else {
            return false;
        };
        let mut held = false;
        for o in &s.records {
            if share_set_of(o).is_some_and(|v| v == key) {
                // The same share again is no second one.
                if o.field(field::CODEX32_STRING) == r.field(field::CODEX32_STRING) {
                    return false;
                }
                held = true;
            }
        }
        held
    }

    /// Costs `identity` for the source key record and for each key record
    /// of the target whose fields differ, a PBKDF2 and two elliptic-curve
    /// multiplications each unless `identity` kept them. Whether open vault
    /// `to` holds the item record `i` of open vault `from`: a record of its
    /// kind with every field the same or, for two key records, the same
    /// identity (a key whose other fields differ). Compared here, so that
    /// no secret passes the caller: `identity` is given a key record and
    /// answers what tells its key from another (its fingerprint and master
    /// id), `None` where its material cannot be read; two records with no
    /// identity are not the same.
    pub fn holds_same<I: PartialEq>(
        &self,
        from: VaultId,
        i: usize,
        to: VaultId,
        identity: impl Fn(RecordView<'_>) -> Option<I>,
    ) -> bool {
        let (Some(a), Some(t)) = (
            self.slot(from).and_then(|s| s.records.get(i)),
            self.slot(to),
        ) else {
            return false;
        };
        let fields_same = |o: &Record| {
            o.fields.len() == a.fields.len()
                && o.fields
                    .iter()
                    .zip(&a.fields)
                    .all(|(x, y)| x.number == y.number && *x.bytes == *y.bytes)
        };
        let id = |r: &Record| identity(RecordView::of(r));
        let mut mine = None;
        t.records.iter().filter(|o| o.kind == a.kind).any(|o| {
            fields_same(o)
                || (a.kind == kind::KEY && {
                    let m = mine.get_or_insert_with(|| id(a));
                    m.is_some() && id(o) == *m
                })
        })
    }

    /// A lookup. Whether record `i` of open vault `id` is an entry that
    /// carries what Faraday does not read and keeps in its place in the
    /// vault (§7.1: attachments, tags, an icon, a person's other fields and
    /// custom data), which a copy of it elsewhere does not take. False for
    /// a record made here, a plan, and a record not held.
    pub fn carries_unread(&self, id: VaultId, i: usize) -> bool {
        fn find<'a>(
            g: &'a fv::kdbx::xml::Group,
            uuid: &[u8; 16],
        ) -> Option<&'a fv::kdbx::xml::Entry> {
            g.entries()
                .find(|e| e.uuid() == *uuid)
                .or_else(|| g.groups().find_map(|s| find(s, uuid)))
        }
        let Some(s) = self.slot(id) else {
            return false;
        };
        let Some(Origin::Entry { uuid, item, .. }) = s.origin.get(i) else {
            return false;
        };
        let read = s.opened.read();
        read.items.get(*item).is_some_and(|it| !it.extra.is_empty())
            || find(s.opened.document().root(), uuid).is_some_and(|e| e.carries_unread())
    }

    /// Costs what [`Records::passphrases_kept_for`] costs, and a parse of
    /// the record's words. The passphrase records the open vaults keep
    /// alone for key record `i` of open vault `id`: none where the record
    /// holds its own passphrase or is no words.
    pub fn passphrases_of_key(
        &self,
        c: &Compute,
        keys: &Keys,
        id: VaultId,
        i: usize,
    ) -> Vec<(VaultId, usize)> {
        let Some(r) = self.slot(id).and_then(|s| s.records.get(i)) else {
            return Vec::new();
        };
        if r.kind != kind::KEY || r.text(field::KEY_PASSPHRASE).is_some() {
            return Vec::new();
        }
        record_words(c, r).map_or_else(Vec::new, |m| self.passphrases_kept_for(c, keys, &m))
    }

    /// Costs what [`Records::passphrases_kept_for`] costs. The passphrase
    /// an open vault keeps alone for the key the words `m` make: the
    /// `chosen`th of those found (the first when it names none).
    pub fn passphrase_kept_for(
        &self,
        c: &Compute,
        keys: &Keys,
        m: &Mnemonic,
        chosen: usize,
    ) -> Option<Zeroizing<String>> {
        let found = self.passphrases_kept_for(c, keys, m);
        let (id, i) = *found.get(chosen).or_else(|| found.first())?;
        let r = self.slot(id)?.records.get(i)?;
        Some(Zeroizing::new(r.text(field::PASSPHRASE)?.to_string()))
    }
}
