//! The backup comes before spending (`docs/history/NEW-WALLET.md` §14,
//! `docs/DECISIONS.md` F6): a key made here signs nothing until a backup
//! checklist that keeps its seed, and its passphrase, is complete.
//!
//! Which keys are held is known by fingerprint and never by a secret: in
//! memory while the key is here, in a vault as a bit of the key record's
//! flags beside its seed, and in the backup plan's record as a `held`
//! line. A key loaded from a vault that says so is held again.

use crate::strings::EN;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_keys::vault::RecordView;
use faraday_text::Text;
use faraday_vault::records::{self, Record, field, kind};

use crate::glance::{Now, Reads};
use crate::wallet::{self, fp_text};
use crate::{Action, BStage, Faraday, Screen, bstep, plan};

/// Costs what `words_id` and `record_words_id` cost. Whether vault key
/// record `r` holds the words of session key `key`, from whichever
/// list: told by the SHA-256 of each one's entropy
/// ([`wallet::WordsId`]), all 32 bytes, both kept once worked out, so
/// nothing parses the words again and two seeds never pass for one.
fn same_words(c: &Compute, app: &Faraday, r: RecordView<'_>, key: usize) -> bool {
    app.session
        .keys
        .words_id(c, key)
        .is_some_and(|id| app.vaults.record_words_id(c, r).is_some_and(|x| *x == *id))
}

/// Whether vault key record `r` stores session key `key`'s passphrase.
fn stores_passphrase(app: &Faraday, r: RecordView<'_>, key: usize) -> bool {
    app.vaults
        .store()
        .stores_passphrase_of(r, &app.session.keys, key)
}

/// Whether vault key record `r` says its key's backup is pending.
fn pending(r: RecordView<'_>) -> bool {
    r.public(field::KEY_FLAGS)
        .and_then(|f| f.first())
        .is_some_and(|b| b & records::BACKUP_PENDING != 0)
}

impl Faraday {
    /// Key record `i` of open vault `id`'s flags, 0 where it has none.
    fn flags_of(&self, id: faraday_keys::vault::VaultId, i: usize) -> u8 {
        self.vaults
            .records(id)
            .get(i)
            .and_then(|r| r.public(field::KEY_FLAGS))
            .and_then(|f| f.first().copied())
            .unwrap_or(0)
    }

    /// Costs what `same_words` costs. Whether open vault `o` holds session
    /// key `k`'s seed: a key record of its words, and with `with_pass`,
    /// with its passphrase too.
    pub(crate) fn vault_holds_seed(
        &self,
        c: &Compute,
        o: usize,
        k: usize,
        with_pass: bool,
    ) -> bool {
        let Some(open) = self.vaults.open.get(o) else {
            return false;
        };
        let seed = self.session.keys.seed_record_id(k);
        self.vaults.records(open.id).of(kind::KEY).any(|(_, r)| {
            (same_words(c, self, r, k)
                || seed.is_some_and(|id| faraday_keys::vault::key_record_id(r) == Some(id)))
                && (!with_pass
                    || self
                        .vaults
                        .store()
                        .stores_passphrase_of(r, &self.session.keys, k))
        })
    }

    /// Marks key `fp` as made here: held until its backup is done.
    pub(crate) fn mark_made_here(&mut self, fp: Fingerprint) {
        self.flows.note_made(fp);
    }

    /// Costs what `held_keys_stored_with` costs. The keys here that are
    /// held: made here, or loaded from a vault that says so, or kept back
    /// by a wallet's backup flow until its last stick is written
    /// (`docs/BACKUP-FLOW.md` §10.2), with their backup not done.
    ///
    /// Works out [`crate::drawing::View::held_keys`] from the state as it is.
    pub fn held_keys_now(&self, c: &Compute) -> Vec<Fingerprint> {
        let flow = self.flow_held();
        let mut out = self.held_keys_stored_with(c, &flow);
        for fp in flow {
            if !out.contains(&fp) {
                out.push(fp);
            }
        }
        out
    }

    /// The keys here that a wallet's flow holds back until its writes are
    /// done.
    pub(crate) fn flow_held(&self) -> Vec<Fingerprint> {
        self.flow_keys(false)
    }

    /// The keys here whose copy is written: since the app started, or by a
    /// wallet's flow that held them back and has written its sticks. A
    /// vault's record that says its backup is pending holds none of them,
    /// as that mark may be kept for another key of its words.
    fn keys_copied(&self) -> Vec<Fingerprint> {
        let mut out = self.flow_keys(true);
        for &fp in self.flows.copied() {
            if !out.contains(&fp) {
                out.push(fp);
            }
        }
        out
    }

    /// Flow `i` goes: the keys it released, its sticks written, stay
    /// counted as copied once it is gone.
    pub(crate) fn flow_copies_note(&mut self, i: usize) {
        let Some(f) = self.flows.get(i) else {
            return;
        };
        let Some(w) = f.wallet.as_ref().filter(|_| f.made) else {
            return;
        };
        let released: Vec<Fingerprint> = self
            .session
            .keys
            .all()
            .map(|k| k.fingerprint)
            .filter(|&fp| {
                w.held.iter().any(|h| h.eq_ignore_ascii_case(&fp_text(fp)))
                    && self.wallet_seed_released(i, fp)
            })
            .collect();
        self.flows.note_copied(released);
    }

    /// The keys here a wallet's flow held back: those it still holds, or
    /// with `released` those it has released, its sticks written.
    fn flow_keys(&self, released: bool) -> Vec<Fingerprint> {
        let mut out: Vec<Fingerprint> = Vec::new();
        for (i, f) in self.flows.iter().enumerate() {
            let Some(w) = f.wallet.as_ref().filter(|_| f.made) else {
                continue;
            };
            for k in self.session.keys.all() {
                let fp = k.fingerprint;
                if w.held.iter().any(|h| h.eq_ignore_ascii_case(&fp_text(fp)))
                    && !out.contains(&fp)
                    && self.wallet_seed_released(i, fp) == released
                {
                    out.push(fp);
                }
            }
        }
        if !released {
            return out;
        }
        // A key another flow still holds is not released.
        let held = self.flow_keys(false);
        out.retain(|f| !held.contains(f));
        out
    }

    /// Costs what `held_keys_stored_with` costs. [`Faraday::held_keys_now`]
    /// without the keys a flow holds back: what a vault's key record and a
    /// plan's `held` lines say. The flow's own hold is in its state, not in
    /// the copies on the sticks.
    pub(crate) fn held_keys_stored(&self, c: &Compute) -> Vec<Fingerprint> {
        self.held_keys_stored_with(c, &self.flow_held())
    }

    /// Costs what `same_words` costs. [`Faraday::held_keys_stored`] given
    /// what [`Faraday::flow_held`] returns, so a caller that has it does
    /// not work it out again.
    fn held_keys_stored_with(&self, c: &Compute, flow: &[Fingerprint]) -> Vec<Fingerprint> {
        let mut out: Vec<Fingerprint> = self
            .flows
            .made_here()
            .iter()
            .copied()
            .filter(|f| !flow.contains(f))
            .collect();
        let mut add = |fp: Fingerprint| {
            if !out.contains(&fp) && !flow.contains(&fp) {
                out.push(fp);
            }
        };
        let copied = self.keys_copied();
        for open in &self.vaults.open {
            for (_, r) in self.vaults.records(open.id).of(kind::PLAN) {
                let Some(text) = r.public_text(field::PLAN_ANSWERS) else {
                    continue;
                };
                for held in plan::held_in(text) {
                    if let Some(k) = self
                        .session
                        .keys
                        .all()
                        .find(|k| fp_text(k.fingerprint).eq_ignore_ascii_case(&held))
                    {
                        add(k.fingerprint);
                    }
                }
            }
            for (_, r) in self
                .vaults
                .records(open.id)
                .of(kind::KEY)
                .filter(|(_, r)| pending(*r))
            {
                for (i, k) in self.session.keys.all().enumerate() {
                    // Every key of its words, as before the passphrase was
                    // told apart; a copy written since takes the key out of
                    // the hold the mark keeps for another key of them.
                    if !same_words(c, self, r, i) || copied.contains(&k.fingerprint) {
                        continue;
                    }
                    add(k.fingerprint);
                }
            }
        }
        out
    }

    /// Costs what `held_keys_now` costs. Whether key `fp` is held.
    pub fn key_held(&self, c: &Compute, fp: Fingerprint) -> bool {
        self.held_keys_now(c).contains(&fp)
    }

    /// Costs what `held_keys_stored` costs. Whether key `fp` is held and a
    /// copy of it should say so: the hold a flow keeps is not written to a
    /// stick.
    pub(crate) fn key_held_stored(&self, c: &Compute, fp: Fingerprint) -> bool {
        self.held_keys_stored(c).contains(&fp)
    }

    /// Costs what `wallet_held_on` costs. The keys of loaded wallet `w`
    /// here that are held, for a decision ([`wallet_held_on`]).
    pub fn wallet_held(&self, c: &Compute, w: usize) -> Vec<Fingerprint> {
        wallet_held_on(&Now { app: self, c }, w)
    }

    /// Costs what `receive_held_on` costs. Whether a key of `wl` made here
    /// still waits for its backup, for a decision ([`receive_held_on`]).
    pub fn receive_held(&self, c: &Compute, wl: &wallet::Wallet) -> bool {
        receive_held_on(&Now { app: self, c }, wl)
    }

    /// Costs what `receive_line_on` costs: an address derivation the first
    /// time, and the held keys. An address of `wl` as every screen shows
    /// it, and whether it is held back, for a decision
    /// ([`receive_line_on`]).
    pub fn receive_line(
        &self,
        c: &Compute,
        wl: &wallet::Wallet,
        change: bool,
        index: u32,
    ) -> (Text, bool) {
        receive_line_on(&Now { app: self, c }, wl, change, index)
    }

    /// Costs what `held_keys_stored` costs. [`Faraday::wallet_held`] as a
    /// copy of the plan says it: without the keys a flow holds back.
    pub(crate) fn wallet_held_stored(&self, c: &Compute, w: usize) -> Vec<Fingerprint> {
        self.wallet_held_in(w, self.held_keys_stored(c))
    }

    fn wallet_held_in(&self, w: usize, held: Vec<Fingerprint>) -> Vec<Fingerprint> {
        if held.is_empty() {
            return Vec::new();
        }
        self.backup_keys(w)
            .into_iter()
            .filter_map(|k| self.session.keys.info(k))
            .map(|k| k.fingerprint)
            .filter(|fp| held.contains(fp))
            .collect()
    }

    /// Costs what `backup_held_seeds_on` costs. The seeds of the wallet
    /// being backed up that are held, by place among its seeds.
    pub(crate) fn backup_held_seeds(&self, c: &Compute) -> Vec<usize> {
        backup_held_seeds_on(&Now { app: self, c })
    }

    /// Costs what `backup_kept_nowhere_on` costs. The line said in place of
    /// Make the checklist, for a decision ([`backup_kept_nowhere_on`]).
    pub fn backup_kept_nowhere(&self, c: &Compute) -> Option<Text> {
        backup_kept_nowhere_on(&Now { app: self, c })
    }

    /// Before an open vault is sealed: every key record of a held key's
    /// words in it says so, whichever passphrase it stores, as a pending
    /// record holds every key of its words; and the held keys are
    /// remembered here once the vault is gone.
    pub(crate) fn held_sweep(&mut self) {
        let c = self.compute();
        let held = self.held_keys_stored(&c);
        self.flows.set_made_here(held.clone());
        let keys: Vec<usize> = (0..self.session.keys.len())
            .filter(|&k| held.contains(&self.session.keys[k].fingerprint))
            .collect();
        for v in 0..self.vaults.open.len() {
            let (id, vault) = (self.vaults.open[v].id, self.vaults.open[v].vault_id());
            let opened = self.vaults.store().opened_digest(id);
            // Each key record of a held key's words: where it is, whether it
            // was marked already, its id and the held keys of its words.
            type Row = (usize, bool, Option<[u8; 32]>, Vec<Fingerprint>);
            let rows: Vec<Row> = self
                .vaults
                .records(id)
                .of(kind::KEY)
                .filter_map(|(i, r)| {
                    let fps: Vec<Fingerprint> = keys
                        .iter()
                        .filter(|&&k| same_words(&c, self, r, k))
                        .map(|&k| self.session.keys[k].fingerprint)
                        .collect();
                    (!fps.is_empty())
                        .then(|| (i, pending(r), faraday_keys::vault::key_record_id(r), fps))
                })
                .collect();
            let mut changed = false;
            for (i, was, record, fps) in rows {
                if let Some(record) = record {
                    self.flows
                        .note_sealed_for((vault, opened), record, &fps, !was);
                }
                if was {
                    continue;
                }
                let flags = self.flags_of(id, i);
                self.vaults.edit(id, |s| {
                    s.set_public(id, i, field::KEY_FLAGS, &[flags | records::BACKUP_PENDING])
                });
                changed = true;
            }
            if changed {
                self.vaults.bump(v);
            }
        }
    }

    /// After each press on the backup: once every checklist item is done,
    /// the held keys whose seed and passphrase the plan keeps are held no
    /// more, here and in every open vault.
    pub(crate) fn backup_settle(&mut self) {
        let c = self.compute();
        self.backup_move_on(&c);
        let Some(b) = self.backup.state() else {
            return;
        };
        if b.stage != BStage::Checklist {
            return;
        }
        let w = b.wallet;
        let items = self.backup_items_now(&c);
        // Every item but the writes: those wait for the lock, and the
        // hold on a key kept in a vault moves to the flow there.
        let all = items
            .iter()
            .filter(|&&n| n != bstep::STICKS)
            .all(|&n| bstep::item(n).is_some_and(|it| self.backup_item_done_now(&c, it)));
        if !all {
            return;
        }
        let writes = items.contains(&bstep::STICKS);
        let shape = self.plan_shape_now(w);
        let list = self.backup_seed_list_now(w);
        let mapped = self.kept.plan_mapped(&c, &shape, &b.answers);
        // A seed kept on any stick, a vault's or a file's, waits for it.
        let in_vault = |i: usize| mapped.seed_on_stick(i);
        let released: Vec<(Fingerprint, usize)> = self
            .backup_held_seeds(&c)
            .into_iter()
            .filter(|&i| !(writes && in_vault(i)))
            .filter(|&i| plan::kept_nowhere_on(&shape, &mapped, &[i]).is_none())
            .filter_map(|i| {
                let k = list.get(i)?.1?;
                Some((self.session.keys.info(k)?.fingerprint, k))
            })
            .collect();
        if released.is_empty() {
            return;
        }
        for &(fp, k) in &released {
            // The same words without that passphrase are kept too.
            let same = self.bare_words_of(&c, k);
            self.flows.release_made(fp, &same);
            self.held_clear_marks(fp, k);
        }
        // The toast names only what signs now: a key another hold keeps is
        // not among them.
        let held = self.held_keys_now(&c);
        let mut names = Text::new();
        for (fp, _) in released.iter().filter(|(fp, _)| !held.contains(fp)) {
            if !names.is_empty() {
                names.push_str(", ");
            }
            names.push_str(&fp_text(*fp));
        }
        if names.is_empty() {
            return;
        }
        self.toast(&Text::fill(EN.held_signs_from_now, &[&names]));
    }

    /// Costs what `with_same_words` costs. The fingerprints of the keys of
    /// session key `k`'s words that have no passphrase: the words alone,
    /// which a copy of the words covers. Another passphrase of the same
    /// words is not covered by it.
    pub(crate) fn bare_words_of(&self, c: &Compute, k: usize) -> Vec<Fingerprint> {
        self.session
            .keys
            .with_same_words(c, k)
            .into_iter()
            .filter(|&o| self.session.keys.info(o).is_some_and(|i| !i.has_passphrase))
            .map(|o| self.session.keys[o].fingerprint)
            .collect()
    }

    /// Costs what `with_same_words` costs. Whether another passphrase of
    /// session key `k`'s words is made here still, its copy not written. A
    /// vault's pending record holds every passphrase of its words, whichever
    /// it stores, so while one waits the record keeps its mark. A key a flow
    /// holds back is not counted: the flow keeps that hold in its own state
    /// until its sticks are written, and the copies its lock seals say the
    /// backup is done, so a mark kept for it would outlast it.
    fn other_passphrase_made_here(&self, c: &Compute, k: usize) -> bool {
        self.session
            .keys
            .with_same_words(c, k)
            .into_iter()
            .filter(|&o| o != k && self.session.keys.info(o).is_some_and(|i| i.has_passphrase))
            .map(|o| self.session.keys[o].fingerprint)
            .any(|f| self.flows.made_here().contains(&f))
    }

    /// Costs what `same_words` costs. Whether a copy of session key `k`
    /// written takes the mark "backup pending" off vault key record `r` of
    /// its words, unless `other_made` (another passphrase of them is made
    /// here still): one with no passphrase, at once; one that stores a
    /// passphrase, when `k` is that one's key or a copy of that key is
    /// written too (in `copied`).
    fn copy_clears(
        &self,
        c: &Compute,
        r: RecordView<'_>,
        k: usize,
        copied: &[Fingerprint],
        other_made: bool,
    ) -> bool {
        if other_made || !same_words(c, self, r, k) {
            return false;
        }
        !r.has(field::KEY_PASSPHRASE)
            || stores_passphrase(self, r, k)
            || self
                .session
                .keys
                .all()
                .enumerate()
                .any(|(j, o)| copied.contains(&o.fingerprint) && stores_passphrase(self, r, j))
    }

    /// Open vault `o` was sealed into the file whose SHA-256 is `written`:
    /// the marks "backup pending" its lock made are that file's
    /// ([`Self::held_unlock_clear`]).
    pub(crate) fn held_sealed(&mut self, o: usize, written: [u8; 32]) {
        let Some((id, vault)) = self.vaults.open.get(o).map(|v| (v.id, v.vault_id())) else {
            return;
        };
        let opened = self.vaults.store().opened_digest(id);
        self.flows.note_sealed_file((vault, opened), written);
    }

    /// Costs what `copy_clears` costs, for each key of each key record of
    /// open vault `o`. At `o`'s unlock from the file whose SHA-256 is `sha`:
    /// the mark "backup pending" comes off each key record of it that a lock
    /// in this session marked in that file, when the
    /// keys it marked it for and its keys here all have a copy written
    /// since the app started, none of them made here or held by a flow, as
    /// the last of those copies would have taken it off had the vault been
    /// open then. A vault locked while that copy was written keeps no hold
    /// for the next start; a mark from an earlier session stays.
    pub(crate) fn held_unlock_clear(&mut self, o: usize, sha: Option<[u8; 32]>) {
        let c = self.compute();
        let Some((id, vault)) = self.vaults.open.get(o).map(|v| (v.id, v.vault_id())) else {
            return;
        };
        let copied = self.keys_copied();
        let flow = self.flow_held();
        let done = |fp: &Fingerprint| {
            copied.contains(fp) && !self.flows.made_here().contains(fp) && !flow.contains(fp)
        };
        let marks: Vec<usize> = self
            .vaults
            .records(id)
            .of(kind::KEY)
            .filter(|(_, r)| {
                // Only a mark a lock in this session made in the file opened,
                // once every key it was made for has a copy: a mark from an
                // earlier session, in another copy too, may be kept for a key
                // that is not here.
                let sealed = faraday_keys::vault::key_record_id(*r)
                    .and_then(|record| self.flows.sealed_for(vault, record, sha));
                if !pending(*r) || !sealed.is_some_and(|fps| fps.iter().all(done)) {
                    return false;
                }
                let keys: Vec<usize> = (0..self.session.keys.len())
                    .filter(|&k| same_words(&c, self, *r, k))
                    .collect();
                !keys.is_empty()
                    && keys
                        .iter()
                        .all(|&k| done(&self.session.keys[k].fingerprint))
                    && keys
                        .iter()
                        .any(|&k| self.copy_clears(&c, *r, k, &copied, false))
            })
            .map(|(i, _)| i)
            .collect();
        if marks.is_empty() {
            return;
        }
        for i in marks {
            let flags = self.flags_of(id, i);
            self.vaults.edit(id, |s| {
                s.set_public(id, i, field::KEY_FLAGS, &[flags & !records::BACKUP_PENDING])
            });
        }
        self.vaults.bump(o);
    }

    /// Takes the mark "backup pending" off the records in every open vault
    /// that a copy of session key `k` covers ([`Faraday::copy_clears`]), and
    /// key `fp`'s `held` line off every plan they keep.
    pub(crate) fn held_clear_marks(&mut self, fp: Fingerprint, k: usize) {
        let c = self.compute();
        let text = fp_text(fp);
        let other_made = self.other_passphrase_made_here(&c, k);
        let copied = self.keys_copied();
        for v in 0..self.vaults.open.len() {
            let mut changed = false;
            if self.session.keys.info(k).is_none() {
                return;
            }
            let marks: Vec<usize> = self
                .vaults
                .records(self.vaults.open[v].id)
                .of(kind::KEY)
                .filter(|(_, r)| pending(*r) && self.copy_clears(&c, *r, k, &copied, other_made))
                .map(|(i, _)| i)
                .collect();
            let id = self.vaults.open[v].id;
            for i in marks {
                let flags = self.flags_of(id, i);
                self.vaults.edit(id, |s| {
                    s.set_public(id, i, field::KEY_FLAGS, &[flags & !records::BACKUP_PENDING])
                });
                changed = true;
            }
            // The plans that name the key held: only those change.
            let plans: Vec<usize> = self
                .vaults
                .records(self.vaults.open[v].id)
                .of(kind::PLAN)
                .filter(|(_, r)| {
                    r.public_text(field::PLAN_ANSWERS).is_some_and(|a| {
                        plan::held_in(a)
                            .iter()
                            .any(|h| h.eq_ignore_ascii_case(&text))
                    })
                })
                .map(|(i, _)| i)
                .collect();
            for i in plans {
                let Some(answers) = self
                    .vaults
                    .records(id)
                    .get(i)
                    .filter(|r| r.kind() == kind::PLAN)
                    .and_then(|r| r.public_text(field::PLAN_ANSWERS))
                else {
                    continue;
                };
                if plan::held_in(answers)
                    .iter()
                    .any(|h| h.eq_ignore_ascii_case(&text))
                {
                    let mut kept = Text::new();
                    for l in answers.lines().filter(|l| {
                        !l.strip_prefix("held ")
                            .is_some_and(|h| h.trim().eq_ignore_ascii_case(&text))
                    }) {
                        kept.push_str(l);
                        kept.push_str("\n");
                    }
                    self.vaults.edit(id, |s| {
                        s.set_public(id, i, field::PLAN_ANSWERS, kept.as_bytes())
                    });
                    changed = true;
                }
            }
            if changed {
                self.vaults.bump(v);
            }
        }
    }

    /// Finish the backup first: the wallet's checklist, or its plan when
    /// none is made.
    pub(crate) fn backup_first(&mut self, w: usize) {
        if let Some(b) = self.backup.state()
            && b.wallet == w
            && b.stage == BStage::Plan
        {
            self.screen = Screen::Backup;
            return;
        }
        self.act(Action::BackupChecklist(w));
        if self.screen != Screen::Backup {
            self.act(Action::Backup(w));
        }
    }

    /// Costs what `backup_items_now` and `backup_item_done_now` cost.
    /// Whether checklist item `n` may open: it is on the checklist and
    /// every item before it is done.
    pub(crate) fn backup_reachable(&self, c: &Compute, n: u8) -> bool {
        let items = self.backup_items_now(c);
        items.contains(&n)
            && items
                .iter()
                .take_while(|&&i| i != n)
                .all(|&i| bstep::item(i).is_some_and(|it| self.backup_item_done_now(c, it)))
    }

    /// Costs what `backup_items_now` and `backup_item_done_now` cost. The
    /// first item not done, which the later items wait for.
    pub fn backup_waiting(&self, c: &Compute) -> Option<u8> {
        self.backup_items_now(c)
            .into_iter()
            .find(|&i| bstep::item(i).is_some_and(|it| !self.backup_item_done_now(c, it)))
    }

    /// The vault item's **Load at unlock** (F62): turned off or on for
    /// plan vault `v`, and set on the wallet's record there when the vault
    /// holds it already.
    pub(crate) fn backup_vault_load_toggle(&mut self, v: usize) {
        if self.backup.load_toggle(v) {
            self.backup_vault_load_set(v);
        }
    }

    /// Sets the wallet's record in the vault plan vault `v` is saved into
    /// to load at unlock or not, as the vault item says. Costs a parse of
    /// the vault's wallet records.
    fn backup_vault_load_set(&mut self, v: usize) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        let loads = !b.no_load.contains(&v);
        let Some(want) = self
            .session
            .wallets
            .get(b.wallet)
            .map(|w| crate::wallet::same_wallet(w.policy()))
        else {
            return;
        };
        let Some(cur) = self.backup_vault_target(v) else {
            return;
        };
        if self.backup_vault_taken(&c, v, cur) {
            return;
        }
        let Some(id) = self.vaults.open.get(cur).map(|o| o.id) else {
            return;
        };
        // This wallet's records whose mark is not as wanted yet.
        let to_set: Vec<usize> = self
            .vaults
            .described_of(&c, id, kind::WALLET)
            .filter(|(_, r, d)| d.same.as_str() == &*want && r.loads_at_unlock() != loads)
            .map(|(i, _, _)| i)
            .collect();
        for &i in &to_set {
            self.vaults
                .edit(id, |s| s.set_loads_at_unlock(id, i, loads));
        }
        if !to_set.is_empty() {
            self.vaults.changed(id);
        }
    }

    /// Save into the open vault everything the plan puts in vault `v`, in
    /// one press: each of its seeds, with its passphrase where the plan
    /// keeps it there, and the wallet where the plan puts it there; then
    /// the plan beside them.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn backup_vault_save(&mut self, v: usize) {
        let c = self.compute();
        use crate::vault::VaultAction as V;
        use crate::vault::screen::vault_has_wallet;
        let Some(cur) = self.backup_vault_target(v) else {
            return;
        };
        if self.backup_vault_taken(&c, v, cur) {
            return;
        }
        // A seed about to be saved with a codex32 set or a BIP-85 origin
        // asks first; Save comes back here (`docs/VAULT.md` §7).
        let (own, _) = self.backup_vault_seeds(v);
        let unsaved: Vec<usize> = own
            .iter()
            .filter(|&&(k, with)| !self.vault_holds_seed(&c, cur, k, with))
            .map(|&(k, _)| k)
            .collect();
        let asks = crate::vault::keep::asks_for(self, &unsaved);
        if crate::vault::keep::gate(self, asks, crate::vault::keep::Then::Vault(v), &[cur]) {
            return;
        }
        self.vaults.set_current(cur);
        // What the vault held before this backup first saved into it.
        let salt = self.vaults.open[cur].vault_id();
        if self
            .backup
            .state()
            .is_some_and(|b| !b.before.iter().any(|(s, _)| *s == salt))
        {
            let held = self.vault_tokens(&c, cur);
            self.backup
                .note_before(salt, held.iter().map(|t| Text::of(t)).collect());
        }
        let Some(b) = self.backup.state() else {
            return;
        };
        let w = b.wallet;
        let wallet_too = b.answers.wallet_in_vault(v);
        for (k, with) in own {
            if !self.vault_holds_seed(&c, cur, k, with) {
                self.vault_act(V::SaveKeyKept(k, with, crate::vault::keep::Scope::Backup));
            }
        }
        let has = self
            .session
            .wallets
            .get(w)
            .is_some_and(|wl| vault_has_wallet(&c, self, cur, wl));
        if wallet_too && !has {
            self.vault_act(V::SaveWallet(w));
        }
        self.backup_vault_load_set(v);
        // Each passphrase the plan keeps alone in the vault, as a
        // passphrase record (`docs/BACKUP-FLOW.md` §17.12 item 4): one per
        // key; a passphrase the vault holds already is left as it is.
        for k in self.backup_vault_passphrases(&c, v) {
            if self.vault_holds_passphrase(cur, k) {
                continue;
            }
            let Some(key) = self.session.keys.info(k) else {
                continue;
            };
            let fp = key.fingerprint;
            if !key.has_passphrase {
                continue;
            }
            let pass = self.session.keys.reveal(
                k,
                faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Passphrase),
            );
            let record = Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &fp.0)
                .with(field::PASSPHRASE, pass.as_bytes());
            let said = Text::fill(EN.plan_passphrase_label, &[&fp_text(fp)]);
            self.vault_push_in(cur, record, &said);
        }
        // Each codex32 string the plan keeps in the vault, as a share
        // record (`docs/CODEX32.md` §9.5): once per key and index. A
        // string this session does not hold is named, to be typed in its
        // item first. A share record of a codex32 seed of the plan that the
        // plan keeps in another vault and not here goes; one the plan lists
        // in no vault stays.
        let gone = self.backup_vault_c32_strays(&c, cur, v);
        if !gone.is_empty()
            && let Some(id) = self.vaults.open.get(cur).map(|o| o.id)
        {
            // `remove_where` asks each record once, in order: the count is
            // the record's place before any went (its own place shifts).
            let mut seen = 0;
            self.vaults.edit(id, |s| {
                s.remove_where(id, |_| {
                    seen += 1;
                    gone.contains(&(seen - 1))
                })
            });
            self.vaults.changed(id);
        }
        for (k, index, place) in self.backup_vault_strings(&c, v) {
            if self.vault_holds_string(cur, k, index) {
                continue;
            }
            let fp = self.session.keys.info(k).map(|i| i.fingerprint);
            let Some(form) = self
                .backup
                .state()
                .and_then(|b| b.answers.c32(place))
                .map(|f| ((f.k, f.identifier), f.id_text()))
            else {
                continue;
            };
            let share =
                Text::of(char::to_ascii_uppercase(&char::from(index)).encode_utf8(&mut [0; 4]));
            // Only a string of the plan's set is written: a key loaded
            // with another set holds strings that are not it.
            let record = self
                .session_holds_c32(k, form.0, index)
                .then(|| {
                    self.session.keys.codex32_share_record(
                        k,
                        index,
                        faraday_keys::Reveal::VaultEntry(faraday_keys::Part::Codex32),
                    )
                })
                .flatten();
            match record {
                Some(record) => {
                    let said = Text::fill(
                        EN.c32_vault_saved,
                        &[&share, &fp.map(fp_text).unwrap_or_default()],
                    );
                    self.vault_push_in(cur, record, &said);
                }
                None => self.toast(&Text::fill(EN.c32_vault_missing, &[&share, &form.1])),
            }
        }
        let Some((name, cs, desc)) = self.session.wallets.get(w).map(|wl| {
            (
                wl.name.clone(),
                Text::of(wl.checksum()),
                Faraday::wallet_text(wl),
            )
        }) else {
            return;
        };
        // The plan's split sheets go in only where they rebuild the wallet.
        let mut shares = self.backup_vault_shares(&c, v);
        if !shares.is_empty() {
            let omit = self.backup.state().map_or(0, |b| b.answers.omit);
            let rebuilds = self.session.wallets.get(w).is_some_and(|wl| {
                let (m, n) = crate::wallet::Session::quorum_keys(wl);
                let plan = crate::wallet_backup::split_plan(n, m, omit);
                crate::restore::sheets_rebuild(&c, self.session.network(), wl, &plan)
            });
            if !rebuilds {
                self.toast(EN.backup_sheets_rebuild_another);
                shares.clear();
            }
        }
        for (j, text, of) in shares {
            if self.vault_holds_share(&c, cur, &cs, j, &text) {
                continue;
            }
            let number = u8::try_from(j + 1).unwrap_or(u8::MAX);
            // This wallet's share of that number, told by the wallet and
            // never by its name, is replaced.
            if let Some(id) = self.vaults.open.get(cur).map(|o| o.id) {
                self.vaults.edit(id, |s| {
                    s.remove_where(id, |r| crate::share_record_is(&c, r, &cs, Some(j)))
                });
            }
            let record = Record::new(kind::WALLET_SHARE)
                .with(field::SHARE, text.as_bytes())
                .with(field::SHARE_WALLET_NAME, name.as_bytes())
                .with(field::SHARE_NUMBER, &[number])
                .with(field::SHARE_OF, &[u8::try_from(of).unwrap_or(u8::MAX)])
                .with(field::SHARE_WALLET, desc.as_bytes());
            let said = Text::fill(
                EN.backup_flow_vault_share,
                &[
                    &name,
                    &Text::num(u64::try_from(j + 1).unwrap_or(u64::MAX)),
                    &Text::num(u64::try_from(of).unwrap_or(u64::MAX)),
                ],
            );
            self.vault_push_in(cur, record, &said);
        }
        self.plan_save();
        for (share, id) in self.backup_vault_c32_left(&c, cur, v) {
            self.toast(&Text::fill(
                EN.c32_vault_left,
                &[share.encode_utf8(&mut [0; 4]), &id],
            ));
        }
    }

    /// On Change the plan: the vaults the checklist has filled, by name,
    /// with the seeds each holds.
    pub(crate) fn backup_prior(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        if b.stage != BStage::Checklist {
            return;
        }
        let shape = self.plan_shape_now(b.wallet);
        let mut prior = Vec::new();
        for v in b.answers.vaults_made(&shape) {
            let Some(name) = self.backup_vault_fits(&c, v) else {
                continue;
            };
            let seeds = b
                .answers
                .vault_seeds(&shape, v)
                .into_iter()
                .map(|i| Text::of(&shape.seeds[i].name))
                .collect();
            prior.push((name, seeds));
        }
        self.backup.set_prior(prior);
    }

    /// On Make the checklist: what the last checklist put in a vault that
    /// the new plan has no vault for stays on the map as what it is.
    pub(crate) fn backup_extras(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.state() else {
            return;
        };
        let shape = self.plan_shape_now(b.wallet);
        let kept: Vec<Text> = b
            .answers
            .vaults_made(&shape)
            .into_iter()
            .filter_map(|v| self.backup_vault_fits(&c, v))
            .collect();
        let mut extras = b.extras.clone();
        for (name, seeds) in &b.prior {
            if kept.contains(name) {
                continue;
            }
            for seed in seeds {
                let line = Text::fill(EN.held_holds_seed, &[name, seed]);
                if !extras.contains(&line) {
                    extras.push(line);
                }
            }
        }
        self.backup.set_extras(extras);
    }
}

/// The keys of loaded wallet `w` here that are held, read through `r`
/// ([`Reads`]).
pub(crate) fn wallet_held_on(r: &impl Reads, w: usize) -> Vec<Fingerprint> {
    r.app().wallet_held_in(w, r.held_keys())
}

/// Whether a key of `wl` made here still waits for its backup: none of
/// the wallet's receive addresses is shown, nor any code of one
/// (`docs/DECISIONS.md` F31). Read through `r` ([`Reads`]).
pub(crate) fn receive_held_on(r: &impl Reads, wl: &wallet::Wallet) -> bool {
    let app = r.app();
    let held = r.held_keys();
    if held.is_empty() {
        return false;
    }
    let cs = wl.checksum();
    app.session
        .wallets
        .iter()
        .position(|w| w.checksum() == cs)
        .is_some_and(|i| !app.wallet_held_in(i, held).is_empty())
}

/// An address of `wl` as every screen shows it, in groups, and whether
/// it is held back: then the text is "Back up before you receive" and is
/// drawn in `warning`. This is the one way a receive address reaches the
/// screen or a code. Read through `r` ([`Reads`]).
pub(crate) fn receive_line_on(
    r: &impl Reads,
    wl: &wallet::Wallet,
    change: bool,
    index: u32,
) -> (Text, bool) {
    if receive_held_on(r, wl) {
        return (Text::of(crate::glance::RECEIVE_WARNING), true);
    }
    (
        crate::ui::grouped(&r.address_shown(wl, change, index)),
        false,
    )
}

/// The backup's seeds, by their place in the plan, whose keys here are
/// held. Read through `r` ([`Reads`]).
pub(crate) fn backup_held_seeds_on(r: &impl Reads) -> Vec<usize> {
    let app = r.app();
    let Some(b) = app.backup() else {
        return Vec::new();
    };
    let held = r.held_keys();
    r.backup_seed_list(b.wallet)
        .iter()
        .enumerate()
        .filter_map(|(i, (_, k))| {
            let key = app.session.keys.info((*k)?)?;
            held.contains(&key.fingerprint).then_some(i)
        })
        .collect()
}

/// The line said in place of Make the checklist, when the plan keeps no
/// copy of a key made here, or of its passphrase. Read through `r`
/// ([`Reads`]).
pub(crate) fn backup_kept_nowhere_on(r: &impl Reads) -> Option<Text> {
    let b = r.app().backup()?;
    let shape = r.plan_shape(b.wallet);
    plan::kept_nowhere_on(
        &shape,
        &r.plan_mapped(&shape, &b.answers),
        &backup_held_seeds_on(r),
    )
}
