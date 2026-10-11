//! The backup comes before spending (`docs/NEW-WALLET.md` §14,
//! `docs/DECISIONS.md` F6): a key made here signs nothing until a backup
//! checklist that keeps its seed, and its passphrase, is complete.
//!
//! Which keys are held is known by fingerprint and never by a secret: in
//! memory while the key is here, in a vault as a bit of the key record's
//! flags beside its seed, and in the backup plan's record as a `held`
//! line. A key loaded from a vault that says so is held again.

use faraday_vault::records::{self, Record, field, kind};
use osk_bip::keys::Fingerprint;

use crate::wallet::{self, fp_text};
use crate::{Action, BStage, Faraday, Screen, bstep, plan};

/// Whether vault key record `r` holds the words of session key `key`.
fn same_words(r: &Record, key: &wallet::Key) -> bool {
    let Some(words) = key.words.as_ref() else {
        return false;
    };
    let Ok(m) = osk_bip::bip39::Mnemonic::parse(key.language, words.as_str()) else {
        return false;
    };
    records::words_of(r).is_some_and(|stored| stored.indices() == m.indices())
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
    pub(crate) fn vault_holds_seed(&self, o: usize, k: usize, with_pass: bool) -> bool {
        let (Some(open), Some(key)) = (self.vaults.open.get(o), self.session.keys.get(k)) else {
            return false;
        };
        open.contents.of(kind::KEY).any(|(_, r)| {
            same_words(r, key)
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
    /// says so, with their backup not done.
    pub fn held_keys(&self) -> Vec<Fingerprint> {
        let mut out = self.made_here.clone();
        let mut add = |fp: Fingerprint| {
            if !out.contains(&fp) {
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
                        .find(|k| fp_text(k.master.fingerprint()).eq_ignore_ascii_case(&held))
                    {
                        add(k.master.fingerprint());
                    }
                }
            }
            for (_, r) in open.contents.of(kind::KEY).filter(|(_, r)| pending(r)) {
                for k in self.session.keys.iter().filter(|k| same_words(r, k)) {
                    add(k.master.fingerprint());
                }
            }
        }
        out
    }

    /// Whether key `fp` is held.
    pub fn key_held(&self, fp: Fingerprint) -> bool {
        self.held_keys().contains(&fp)
    }

    /// The keys of loaded wallet `w` here that are held.
    pub fn wallet_held(&self, w: usize) -> Vec<Fingerprint> {
        let held = self.held_keys();
        if held.is_empty() {
            return Vec::new();
        }
        self.backup_keys(w)
            .into_iter()
            .filter_map(|k| self.session.keys.get(k))
            .map(|k| k.master.fingerprint())
            .filter(|fp| held.contains(fp))
            .collect()
    }

    /// The seeds of the wallet being backed up that are held, by place
    /// among its seeds.
    pub(crate) fn backup_held_seeds(&self) -> Vec<usize> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let held = self.held_keys();
        self.backup_seed_list(b.wallet)
            .iter()
            .enumerate()
            .filter_map(|(i, (_, k))| {
                let key = self.session.keys.get((*k)?)?;
                held.contains(&key.master.fingerprint()).then_some(i)
            })
            .collect()
    }

    /// The line said in place of Make the checklist, when the plan keeps
    /// no copy of a key made here, or of its passphrase.
    pub fn backup_kept_nowhere(&self) -> Option<String> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape(b.wallet);
        plan::kept_nowhere(&shape, &b.answers, &self.backup_held_seeds())
    }

    /// Before an open vault is sealed: every held key's record in it says
    /// so, and the held keys are remembered here once the vault is gone.
    pub(crate) fn held_sweep(&mut self) {
        let held = self.held_keys();
        self.made_here = held.clone();
        let keys: Vec<usize> = (0..self.session.keys.len())
            .filter(|&k| held.contains(&self.session.keys[k].master.fingerprint()))
            .collect();
        for v in 0..self.vaults.open.len() {
            let mut changed = false;
            for &k in &keys {
                let key = &self.session.keys[k];
                for r in self.vaults.open[v].contents.records.iter_mut() {
                    if r.kind == kind::KEY && same_words(r, key) && !pending(r) {
                        let flags = r
                            .field(field::KEY_FLAGS)
                            .and_then(|f| f.first().copied())
                            .unwrap_or(0);
                        r.set(field::KEY_FLAGS, &[flags | records::BACKUP_PENDING]);
                        changed = true;
                    }
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
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        if b.stage != BStage::Checklist {
            return;
        }
        let w = b.wallet;
        let items = self.backup_items();
        let all = items
            .iter()
            .all(|&n| bstep::item(n).is_some_and(|it| self.backup_item_done(it)));
        if !all {
            return;
        }
        let shape = self.plan_shape(w);
        let list = self.backup_seed_list(w);
        let released: Vec<(Fingerprint, usize)> = self
            .backup_held_seeds()
            .into_iter()
            .filter(|&i| plan::kept_nowhere(&shape, &b.answers, &[i]).is_none())
            .filter_map(|i| {
                let k = list.get(i)?.1?;
                Some((self.session.keys.get(k)?.master.fingerprint(), k))
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
                .map(|o| o.master.fingerprint())
                .collect();
            self.made_here.retain(|f| *f != fp && !same.contains(f));
            let text = fp_text(fp);
            for v in 0..self.vaults.open.len() {
                let mut changed = false;
                let key = &self.session.keys[k];
                for r in self.vaults.open[v].contents.records.iter_mut() {
                    if r.kind == kind::KEY && same_words(r, key) && pending(r) {
                        let flags = r
                            .field(field::KEY_FLAGS)
                            .and_then(|f| f.first().copied())
                            .unwrap_or(0);
                        r.set(field::KEY_FLAGS, &[flags & !records::BACKUP_PENDING]);
                        changed = true;
                    }
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
        let names: Vec<String> = released.iter().map(|(fp, _)| fp_text(*fp)).collect();
        self.toast(&format!("{} signs from now on", names.join(", ")));
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
    pub(crate) fn backup_reachable(&self, n: u8) -> bool {
        let items = self.backup_items();
        items.contains(&n)
            && items
                .iter()
                .take_while(|&&i| i != n)
                .all(|&i| bstep::item(i).is_some_and(|it| self.backup_item_done(it)))
    }

    /// The first item not done, which the later items wait for.
    pub fn backup_waiting(&self) -> Option<u8> {
        self.backup_items()
            .into_iter()
            .find(|&i| bstep::item(i).is_some_and(|it| !self.backup_item_done(it)))
    }

    /// The first vault the plan makes whose stick no place keeps.
    pub fn backup_stickless(&self) -> Option<usize> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape(b.wallet);
        b.answers.stickless(&shape)
    }

    /// Save into the open vault everything the plan puts in vault `v`, in
    /// one press: each of its seeds, with its passphrase where the plan
    /// keeps it there, and the wallet where the plan puts it there; then
    /// the plan beside them.
    pub(crate) fn backup_vault_save(&mut self, v: usize) {
        use crate::vault_screens::vault_has_wallet;
        use crate::vaults::VaultAction as V;
        let cur = self.vaults.current;
        if self.vaults.open.get(cur).is_none() || self.backup_vault_taken(v, cur) {
            return;
        }
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let w = b.wallet;
        let wallet_too = b.answers.wallet_in_vault(v);
        let (own, _) = self.backup_vault_seeds(v);
        for (k, with) in own {
            if !self.vault_holds_seed(cur, k, with) {
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
            .is_some_and(|wl| vault_has_wallet(self, cur, wl));
        if wallet_too && !has {
            self.vault_act(V::SaveWallet(w));
        }
        self.plan_save();
    }

    /// On Change the plan: the vaults the checklist has filled, by name,
    /// with the seeds each holds.
    pub(crate) fn backup_prior(&mut self) {
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        if b.stage != BStage::Checklist {
            return;
        }
        let shape = self.plan_shape(b.wallet);
        let mut prior = Vec::new();
        for v in b.answers.vaults_made(&shape) {
            let Some(name) = self.backup_vault_fits(v) else {
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
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let shape = self.plan_shape(b.wallet);
        let kept: Vec<String> = b
            .answers
            .vaults_made(&shape)
            .into_iter()
            .filter_map(|v| self.backup_vault_fits(v))
            .collect();
        let mut extras = b.extras.clone();
        for (name, seeds) in &b.prior {
            if kept.contains(name) {
                continue;
            }
            for seed in seeds {
                let line = format!("{name} holds seed {seed}");
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
