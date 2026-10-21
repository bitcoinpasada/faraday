//! The backup comes before spending (`docs/NEW-WALLET.md` §14,
//! `docs/DECISIONS.md` F6): a key made here signs nothing until a backup
//! checklist that keeps its seed, and its passphrase, is complete.
//!
//! Which keys are held is known by fingerprint and never by a secret: in
//! memory while the key is here, in a vault as a bit of the key record's
//! flags beside its seed, and in the backup plan's record as a `held`
//! line. A key loaded from a vault that says so is held again.

use crate::strings::EN;
use crate::strings::fill;
use faraday_vault::records::{self, Record, field, kind};
use osk_bip::keys::Fingerprint;
use osk_crypto::work::Compute;

use crate::wallet::{self, fp_text};
use crate::{Action, BStage, Faraday, Screen, bstep, plan};

/// Whether vault key record `r` holds the words of session key `key`,
/// from whichever list: told by the SHA-256 of each one's entropy
/// ([`wallet::WordsId`]), all 32 bytes, both kept once worked out, so
/// nothing parses the words again and two seeds never pass for one.
fn same_words(c: &Compute, app: &Faraday, r: &Record, key: &wallet::Key) -> bool {
    key.words_id(c)
        .is_some_and(|id| app.record_words_id(c, r).is_some_and(|x| *x == *id))
}

/// Whether vault key record `r` says its key's backup is pending.
fn pending(r: &Record) -> bool {
    r.field(field::KEY_FLAGS)
        .and_then(|f| f.first())
        .is_some_and(|b| b & records::BACKUP_PENDING != 0)
}

impl Faraday {
    /// Whether open vault `o` holds session key `k`'s seed: a key record
    /// of its words, and with `with_pass`, with its passphrase too.
    pub(crate) fn vault_holds_seed(
        &self,
        c: &Compute,
        o: usize,
        k: usize,
        with_pass: bool,
    ) -> bool {
        let (Some(open), Some(key)) = (self.vaults.open.get(o), self.session.keys.get(k)) else {
            return false;
        };
        open.contents.of(kind::KEY).any(|(_, r)| {
            same_words(c, self, r, key)
                && (!with_pass
                    || r.text(field::KEY_PASSPHRASE)
                        .is_some_and(|p| key.passphrase.as_ref().is_some_and(|q| q.as_str() == p)))
        })
    }

    /// Marks key `fp` as made here: held until its backup is done.
    pub(crate) fn mark_made_here(&mut self, fp: Fingerprint) {
        if !self.made_here.contains(&fp) {
            self.made_here.push(fp);
        }
    }

    /// The keys here that are held: made here, or loaded from a vault that
    /// says so, or kept back by a wallet's backup flow until its last
    /// stick is written (`docs/BACKUP-FLOW.md` §10.2), with their backup
    /// not done.
    pub fn held_keys(&self, c: &Compute) -> Vec<Fingerprint> {
        if self.drawing.is_on() {
            return crate::drawing::View::new(self).held_keys();
        }
        self.held_keys_now(c)
    }

    /// Works out [`Faraday::held_keys`] from the state as it is.
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
        let mut out: Vec<Fingerprint> = Vec::new();
        for (i, f) in self.backup_flows.iter().enumerate() {
            let Some(w) = f.wallet.as_ref().filter(|_| f.made) else {
                continue;
            };
            for k in &self.session.keys {
                let fp = k.fingerprint();
                if w.held.iter().any(|h| h.eq_ignore_ascii_case(&fp_text(fp)))
                    && !out.contains(&fp)
                    && !self.wallet_seed_released(i, fp)
                {
                    out.push(fp);
                }
            }
        }
        out
    }

    /// [`Faraday::held_keys`] without the keys a flow holds back: what a
    /// vault's key record and a plan's `held` lines say. The flow's own
    /// hold is in its state, not in the copies on the sticks.
    pub(crate) fn held_keys_stored(&self, c: &Compute) -> Vec<Fingerprint> {
        self.held_keys_stored_with(c, &self.flow_held())
    }

    /// [`Faraday::held_keys_stored`] given what [`Faraday::flow_held`]
    /// returns, so a caller that has it does not work it out again.
    fn held_keys_stored_with(&self, c: &Compute, flow: &[Fingerprint]) -> Vec<Fingerprint> {
        let mut out: Vec<Fingerprint> = self
            .made_here
            .iter()
            .copied()
            .filter(|f| !flow.contains(f))
            .collect();
        let mut add = |fp: Fingerprint| {
            if !out.contains(&fp) && !flow.contains(&fp) {
                out.push(fp);
            }
        };
        for open in &self.vaults.open {
            for (_, r) in open.contents.of(kind::PLAN) {
                let Some(text) = r.text(field::PLAN_ANSWERS) else {
                    continue;
                };
                for held in plan::held_in(text) {
                    if let Some(k) = self
                        .session
                        .keys
                        .iter()
                        .find(|k| fp_text(k.fingerprint()).eq_ignore_ascii_case(&held))
                    {
                        add(k.fingerprint());
                    }
                }
            }
            for (_, r) in open.contents.of(kind::KEY).filter(|(_, r)| pending(r)) {
                for k in self
                    .session
                    .keys
                    .iter()
                    .filter(|k| same_words(c, self, r, k))
                {
                    add(k.fingerprint());
                }
            }
        }
        out
    }

    /// Whether key `fp` is held.
    pub fn key_held(&self, c: &Compute, fp: Fingerprint) -> bool {
        self.held_keys(c).contains(&fp)
    }

    /// Whether key `fp` is held and a copy of it should say so: the
    /// hold a flow keeps is not written to a stick.
    pub(crate) fn key_held_stored(&self, c: &Compute, fp: Fingerprint) -> bool {
        self.held_keys_stored(c).contains(&fp)
    }

    /// The keys of loaded wallet `w` here that are held.
    pub fn wallet_held(&self, c: &Compute, w: usize) -> Vec<Fingerprint> {
        self.wallet_held_in(w, self.held_keys(c))
    }

    /// Whether a key of `wl` made here still waits for its backup: none of
    /// the wallet's receive addresses is shown, nor any code of one
    /// (`docs/DECISIONS.md` F31).
    pub fn receive_held(&self, c: &Compute, wl: &wallet::Wallet) -> bool {
        let held = self.held_keys(c);
        if held.is_empty() {
            return false;
        }
        let cs = wl.checksum();
        self.session
            .wallets
            .iter()
            .position(|w| w.checksum() == cs)
            .is_some_and(|i| !self.wallet_held_in(i, held).is_empty())
    }

    /// An address of `wl` as every screen shows it, in groups, and whether
    /// it is held back: then the text is "Back up before you receive"
    /// and is drawn in `WARN`. This is the one way a receive address
    /// reaches the screen or a code.
    pub fn receive_line(
        &self,
        c: &Compute,
        wl: &wallet::Wallet,
        change: bool,
        index: u32,
    ) -> (String, bool) {
        if self.receive_held(c, wl) {
            return (crate::glance::RECEIVE_WARNING.to_string(), true);
        }
        (
            crate::ui::grouped(&self.session.address_shown(c, wl, change, index)),
            false,
        )
    }

    /// [`Faraday::wallet_held`] as a copy of the plan says it: without the
    /// keys a flow holds back.
    pub(crate) fn wallet_held_stored(&self, c: &Compute, w: usize) -> Vec<Fingerprint> {
        self.wallet_held_in(w, self.held_keys_stored(c))
    }

    fn wallet_held_in(&self, w: usize, held: Vec<Fingerprint>) -> Vec<Fingerprint> {
        if held.is_empty() {
            return Vec::new();
        }
        self.backup_keys(w)
            .into_iter()
            .filter_map(|k| self.session.keys.get(k))
            .map(|k| k.fingerprint())
            .filter(|fp| held.contains(fp))
            .collect()
    }

    /// The seeds of the wallet being backed up that are held, by place
    /// among its seeds.
    pub(crate) fn backup_held_seeds(&self, c: &Compute) -> Vec<usize> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let held = self.held_keys(c);
        self.backup_seed_list(b.wallet)
            .iter()
            .enumerate()
            .filter_map(|(i, (_, k))| {
                let key = self.session.keys.get((*k)?)?;
                held.contains(&key.fingerprint()).then_some(i)
            })
            .collect()
    }

    /// The line said in place of Make the checklist, when the plan keeps
    /// no copy of a key made here, or of its passphrase.
    pub fn backup_kept_nowhere(&self, c: &Compute) -> Option<String> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape(b.wallet);
        plan::kept_nowhere_on(
            &shape,
            &self.plan_mapped(c, &shape, &b.answers),
            &self.backup_held_seeds(c),
        )
    }

    /// Before an open vault is sealed: every held key's record in it says
    /// so, and the held keys are remembered here once the vault is gone.
    pub(crate) fn held_sweep(&mut self) {
        let c = self.compute();
        let held = self.held_keys_stored(&c);
        self.made_here = held.clone();
        let keys: Vec<usize> = (0..self.session.keys.len())
            .filter(|&k| held.contains(&self.session.keys[k].fingerprint()))
            .collect();
        for v in 0..self.vaults.open.len() {
            let mut changed = false;
            for &k in &keys {
                let key = &self.session.keys[k];
                let marks: Vec<usize> = self.vaults.open[v]
                    .contents
                    .of(kind::KEY)
                    .filter(|(_, r)| same_words(&c, self, r, key) && !pending(r))
                    .map(|(i, _)| i)
                    .collect();
                for i in marks {
                    let r = &mut self.vaults.open[v].contents.records[i];
                    let flags = r
                        .field(field::KEY_FLAGS)
                        .and_then(|f| f.first().copied())
                        .unwrap_or(0);
                    r.set(field::KEY_FLAGS, &[flags | records::BACKUP_PENDING]);
                    changed = true;
                }
            }
            if changed {
                self.vaults.open[v].changes += 1;
            }
        }
    }

    /// After each press on the backup: once every checklist item is done,
    /// the held keys whose seed and passphrase the plan keeps are held no
    /// more, here and in every open vault.
    pub(crate) fn backup_settle(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
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
        let mapped = self.plan_mapped(&c, &shape, &b.answers);
        // A seed kept on any stick, a vault's or a file's, waits for it.
        let in_vault = |i: usize| mapped.seed_on_stick(i);
        let released: Vec<(Fingerprint, usize)> = self
            .backup_held_seeds(&c)
            .into_iter()
            .filter(|&i| !(writes && in_vault(i)))
            .filter(|&i| plan::kept_nowhere_on(&shape, &mapped, &[i]).is_none())
            .filter_map(|i| {
                let k = list.get(i)?.1?;
                Some((self.session.keys.get(k)?.fingerprint(), k))
            })
            .collect();
        if released.is_empty() {
            return;
        }
        for &(fp, k) in &released {
            // The same words without that passphrase are kept too.
            let words = self.session.keys[k].words.clone();
            let same: Vec<Fingerprint> = self
                .session
                .keys
                .iter()
                .filter(|o| o.words.is_some() && o.words == words)
                .map(|o| o.fingerprint())
                .collect();
            self.made_here.retain(|f| *f != fp && !same.contains(f));
            self.held_clear_marks(fp, k);
        }
        let names: Vec<String> = released.iter().map(|(fp, _)| fp_text(*fp)).collect();
        self.toast(&fill(
            EN.held_signs_from_now,
            &[&names.join(", ").to_string()],
        ));
    }

    /// Takes the mark "backup pending" off session key `k`'s record in every
    /// open vault, and key `fp`'s `held` line off every plan they keep.
    pub(crate) fn held_clear_marks(&mut self, fp: Fingerprint, k: usize) {
        let c = self.compute();
        let text = fp_text(fp);
        for v in 0..self.vaults.open.len() {
            let mut changed = false;
            let Some(key) = self.session.keys.get(k) else {
                return;
            };
            let marks: Vec<usize> = self.vaults.open[v]
                .contents
                .of(kind::KEY)
                .filter(|(_, r)| same_words(&c, self, r, key) && pending(r))
                .map(|(i, _)| i)
                .collect();
            for i in marks {
                let r = &mut self.vaults.open[v].contents.records[i];
                let flags = r
                    .field(field::KEY_FLAGS)
                    .and_then(|f| f.first().copied())
                    .unwrap_or(0);
                r.set(field::KEY_FLAGS, &[flags & !records::BACKUP_PENDING]);
                changed = true;
            }
            // The plans that name the key held: only those change.
            let plans: Vec<usize> = self.vaults.open[v]
                .contents
                .of(kind::PLAN)
                .filter(|(_, r)| {
                    r.text(field::PLAN_ANSWERS).is_some_and(|a| {
                        plan::held_in(a)
                            .iter()
                            .any(|h| h.eq_ignore_ascii_case(&text))
                    })
                })
                .map(|(i, _)| i)
                .collect();
            for i in plans {
                let r = &mut self.vaults.open[v].contents.records[i];
                if r.kind == kind::PLAN
                    && let Some(answers) = r.text(field::PLAN_ANSWERS)
                    && plan::held_in(answers)
                        .iter()
                        .any(|h| h.eq_ignore_ascii_case(&text))
                {
                    let kept: String = answers
                        .lines()
                        .filter(|l| {
                            !l.strip_prefix("held ")
                                .is_some_and(|h| h.trim().eq_ignore_ascii_case(&text))
                        })
                        .map(|l| format!("{l}\n"))
                        .collect();
                    r.set(field::PLAN_ANSWERS, kept.as_bytes());
                    changed = true;
                }
            }
            if changed {
                self.vaults.open[v].changes += 1;
            }
        }
    }

    /// Finish the backup first: the wallet's checklist, or its plan when
    /// none is made.
    pub(crate) fn backup_first(&mut self, w: usize) {
        if let Some(b) = self.backup.as_ref()
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

    /// The first item not done, which the later items wait for.
    pub fn backup_waiting(&self, c: &Compute) -> Option<u8> {
        self.backup_items(c)
            .into_iter()
            .find(|&i| bstep::item(i).is_some_and(|it| !self.backup_item_done(c, it)))
    }

    /// The first vault the plan makes whose stick no place keeps.
    pub fn backup_stickless(&self) -> Option<usize> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape(b.wallet);
        b.answers.stickless(&shape)
    }

    /// The vault item's **Load at unlock** (F62): turned off or on for
    /// plan vault `v`, and set on the wallet's record there when the vault
    /// holds it already.
    pub(crate) fn backup_vault_load_toggle(&mut self, v: usize) {
        let Some(b) = self.backup.as_mut() else {
            return;
        };
        if !b.answers.wallet_in_vault(v) {
            return;
        }
        if let Some(k) = b.no_load.iter().position(|&x| x == v) {
            b.no_load.remove(k);
        } else {
            b.no_load.push(v);
        }
        self.backup_vault_load_set(v);
    }

    /// Sets the wallet's record in the vault plan vault `v` is saved into
    /// to load at unlock or not, as the vault item says. Costs a parse of
    /// the vault's wallet records.
    fn backup_vault_load_set(&mut self, v: usize) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
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
        let Some(open) = self.vaults.open.get_mut(cur) else {
            return;
        };
        // This wallet's records whose mark is not as wanted yet.
        let to_set: Vec<usize> = open
            .contents
            .described_of(&c, kind::WALLET)
            .filter(|(_, r, d)| {
                d.same == want && faraday_vault::records::loads_at_unlock(r) != loads
            })
            .map(|(i, _, _)| i)
            .collect();
        for &i in &to_set {
            faraday_vault::records::set_loads_at_unlock(&mut open.contents.records[i], loads);
        }
        if !to_set.is_empty() {
            open.changes += 1;
        }
    }

    /// Save into the open vault everything the plan puts in vault `v`, in
    /// one press: each of its seeds, with its passphrase where the plan
    /// keeps it there, and the wallet where the plan puts it there; then
    /// the plan beside them.
    pub(crate) fn backup_vault_save(&mut self, v: usize) {
        let c = self.compute();
        use crate::vault_screens::vault_has_wallet;
        use crate::vaults::VaultAction as V;
        let Some(cur) = self.backup_vault_target(v) else {
            return;
        };
        if self.backup_vault_taken(&c, v, cur) {
            return;
        }
        self.vaults.current = cur;
        // What the vault held before this backup first saved into it.
        let salt = self.vaults.open[cur].header().salt;
        if self
            .backup
            .as_ref()
            .is_some_and(|b| !b.before.iter().any(|(s, _)| *s == salt))
        {
            let held = self.vault_tokens(&c, cur);
            if let Some(b) = self.backup.as_mut() {
                b.before.push((salt, held));
            }
        }
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let w = b.wallet;
        let wallet_too = b.answers.wallet_in_vault(v);
        let (own, _) = self.backup_vault_seeds(v);
        for (k, with) in own {
            if !self.vault_holds_seed(&c, cur, k, with) {
                self.vault_act(if with {
                    V::SaveKeyWithPassphrase(k)
                } else {
                    V::SaveKey(k)
                });
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
        // Each share the plan keeps in the vault, as a type 13 record
        // (`docs/BACKUP-FLOW.md` §17.6): once per wallet and share number,
        // an older text of that share replaced.
        // Each passphrase the plan keeps alone in the vault, as a
        // passphrase record (`docs/BACKUP-FLOW.md` §19.12 item 4): one per
        // key, an older one of that key replaced.
        for k in self.backup_vault_passphrases(&c, v) {
            if self.vault_holds_passphrase(cur, k) {
                continue;
            }
            let Some(key) = self.session.keys.get(k) else {
                continue;
            };
            let fp = key.fingerprint();
            let Some(pass) = key.passphrase.clone() else {
                continue;
            };
            if let Some(open) = self.vaults.open.get_mut(cur) {
                open.contents.records.retain(|r| {
                    !(r.kind == kind::PASSPHRASE
                        && r.field(field::PASSPHRASE_KEY) == Some(&fp.0[..]))
                });
            }
            let record = Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &fp.0)
                .with(field::PASSPHRASE, pass.as_bytes());
            let said = fill(EN.plan_passphrase_of, &[&fp_text(fp)]);
            self.vault_push_in(cur, record, &said);
        }
        let Some((name, cs, desc)) = self.session.wallets.get(w).map(|wl| {
            (
                wl.name.clone(),
                wl.checksum().to_string(),
                Faraday::wallet_text(wl),
            )
        }) else {
            return;
        };
        for (j, text, of) in self.backup_vault_shares(&c, v) {
            if self.vault_holds_share(&c, cur, &cs, j, &text) {
                continue;
            }
            let number = u8::try_from(j + 1).unwrap_or(u8::MAX);
            // This wallet's share of that number, told by the wallet and
            // never by its name, is replaced.
            if let Some(open) = self.vaults.open.get_mut(cur) {
                open.contents
                    .records
                    .retain(|r| !crate::share_record_is(&c, r, &cs, Some(j)));
            }
            let record = Record::new(kind::WALLET_SHARE)
                .with(field::SHARE, text.as_bytes())
                .with(field::SHARE_WALLET_NAME, name.as_bytes())
                .with(field::SHARE_NUMBER, &[number])
                .with(field::SHARE_OF, &[u8::try_from(of).unwrap_or(u8::MAX)])
                .with(field::SHARE_WALLET, desc.as_bytes());
            let said = fill(
                EN.backup_flow_vault_share,
                &[&name, &(j + 1).to_string(), &of.to_string()],
            );
            self.vault_push_in(cur, record, &said);
        }
        self.plan_save();
    }

    /// On Change the plan: the vaults the checklist has filled, by name,
    /// with the seeds each holds.
    pub(crate) fn backup_prior(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
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
                .map(|i| shape.seeds[i].name.clone())
                .collect();
            prior.push((name, seeds));
        }
        if let Some(b) = self.backup.as_mut() {
            b.prior = prior;
        }
    }

    /// On Make the checklist: what the last checklist put in a vault that
    /// the new plan has no vault for stays on the map as what it is.
    pub(crate) fn backup_extras(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let shape = self.plan_shape_now(b.wallet);
        let kept: Vec<String> = b
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
                let line = fill(EN.held_holds_seed, &[&name.to_string(), &seed.to_string()]);
                if !extras.contains(&line) {
                    extras.push(line);
                }
            }
        }
        if let Some(b) = self.backup.as_mut() {
            b.extras = extras;
            b.prior.clear();
        }
    }
}
