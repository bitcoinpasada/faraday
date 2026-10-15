//! A Bitcoin wallet's part of the backup flow (`docs/BACKUP-FLOW.md`
//! §10, §11): the flow driven by the wallet's plan, the vaults set up
//! before the keys exist (whether there are any, how many, their sticks and
//! their passphrases, and nothing about places or which key goes where),
//! the checklist's **Write to sticks**, the hold F6 puts on a key moved to
//! the flow at the lock, and the lines §10.3 adds.
//!
//! The steps, the cards and the checks are `backup_flow_steps.rs` and
//! `backup_flow_screen.rs`, shared with the other kinds. This file is what
//! only a wallet needs.

use crate::backup_flow::{BackupFlow, Kind, PublicOn, Step, Subject, Target};
use crate::plan;
use crate::strings::{EN, fill};
use crate::vaults::VaultAction as V;
use crate::wallet::fp_text;
use crate::{BStage, Faraday, Screen, bstep, cstep, file_stem};
use osk_bip::keys::Fingerprint;

impl Faraday {
    /// A number for a Make a new wallet or Restore, and for the flow tied
    /// to it. Never 0, which no flow is tied to.
    pub(crate) fn fresh_owner(&mut self) -> u64 {
        self.owner_seq += 1;
        self.owner_seq
    }

    /// The wallet flow of the wallet whose descriptor has this checksum: the
    /// one under way, else the one that is done (the checklist reads what
    /// it wrote).
    pub(crate) fn wallet_flow_at(&self, checksum: &str) -> Option<usize> {
        self.wallet_flow_active(checksum).or_else(|| {
            self.backup_flows.iter().position(|f| {
                f.kind == Kind::Wallet
                    && f.wallet
                        .as_ref()
                        .is_some_and(|w| w.checksum.as_deref() == Some(checksum))
            })
        })
    }

    /// The wallet flow of that wallet that is not done: a new backup of a
    /// wallet whose last one is done starts a flow of its own.
    pub(crate) fn wallet_flow_active(&self, checksum: &str) -> Option<usize> {
        self.backup_flows.iter().position(|f| {
            f.kind == Kind::Wallet
                && f.step != Step::Done
                && f.wallet
                    .as_ref()
                    .is_some_and(|w| w.checksum.as_deref() == Some(checksum))
        })
    }

    /// The place of this flow among the flows (a flow drawn is a reference
    /// into the list).
    pub(crate) fn flow_index_of(&self, f: &BackupFlow) -> usize {
        self.backup_flows
            .iter()
            .position(|x| std::ptr::eq(x, f))
            .unwrap_or(0)
    }

    /// The flow of Make a new wallet or Restore number `owner`, before and
    /// after its wallet is made, while it is not done.
    pub(crate) fn wallet_flow_of_owner(&self, owner: u64) -> Option<usize> {
        self.backup_flows.iter().position(|f| {
            f.kind == Kind::Wallet
                && f.step != Step::Done
                && f.wallet
                    .as_ref()
                    .is_some_and(|w| w.owner != 0 && w.owner == owner)
        })
    }

    /// The wallet flow of Make a new wallet or Restore number `owner`, while
    /// its wallet is not made.
    pub(crate) fn wallet_flow_pre_of(&self, owner: u64) -> Option<usize> {
        self.backup_flows.iter().position(|f| {
            f.kind == Kind::Wallet
                && !f.made
                && f.wallet
                    .as_ref()
                    .is_some_and(|w| w.checksum.is_none() && w.owner != 0 && w.owner == owner)
        })
    }

    /// The flow of the wallet being backed up, while its writes are not
    /// done and the keys are not locked away: the checklist's.
    pub(crate) fn wallet_flow_of_backup(&self) -> Option<usize> {
        let b = self.backup.as_ref()?;
        let cs = self.session.wallets.get(b.wallet)?.policy.checksum();
        self.wallet_flow_at(&cs)
    }

    /// The writes wallet `checksum` waits for: how many sticks are written
    /// and how many there are. `None` once every one is.
    pub fn wallet_backup_waits(&self, checksum: &str) -> Option<(usize, usize)> {
        let i = self.wallet_flow_at(checksum)?;
        let f = &self.backup_flows[i];
        f.waiting().then(|| f.written_of())
    }

    // ---- Make a new wallet and Restore: the vaults, before the keys ----

    /// The shape a wallet to be made has before its keys exist: a seed
    /// for each key, all to be made here. `owner` is the Make a new wallet
    /// or Restore it is for.
    pub(crate) fn wallet_placeholder(&self, owner: u64) -> Option<plan::Shape> {
        use crate::create::NewKind as K;
        let (kind, m, n_keys) = match (
            self.create.as_ref().filter(|c| c.owner == owner),
            self.restore.as_ref().filter(|r| r.owner == owner),
        ) {
            (Some(c), _) => (c.kind, c.m, c.n),
            (_, Some(r)) => {
                let s = r.seeds.as_ref()?;
                (s.kind, s.m, s.n)
            }
            _ => return None,
        };
        let n = if kind.threshold() {
            m.max(1)
        } else if kind.multi() {
            n_keys.max(1)
        } else {
            1
        };
        let seeds = (0..n)
            .map(|i| plan::Seed {
                name: fill(EN.backup_flow_key_n, &[&(i + 1).to_string()]),
                here: true,
                passphrase: false,
            })
            .collect();
        Some(plan::Shape {
            m: if n > 1 { m.min(n) } else { 1 },
            keys: n,
            seeds,
            splits: matches!(
                kind,
                K::Multi | K::MultiNested | K::MultiLegacy | K::TapMulti
            ),
            vaults: 0,
        })
    }

    /// How many vaults Make a new wallet or Restore number `owner` set
    /// out to use, when the person answered yes.
    fn wallet_vault_count(&self, owner: u64) -> Option<usize> {
        let ask = match (
            self.create.as_ref().filter(|c| c.owner == owner),
            self.restore.as_ref().filter(|r| r.owner == owner),
        ) {
            (Some(c), _) => c.vaults,
            (_, Some(r)) => r.vaults,
            _ => return None,
        };
        (ask.answer == Some(true)).then_some(ask.count.max(1))
    }

    /// The answer to the vault question of Make a new wallet or Restore
    /// number `owner` changed to no, or the count changed: the setup under
    /// way follows. No vaults leaves nothing to set up; another count
    /// adds or drops the last vaults and keeps the rest as they are.
    pub(crate) fn wallet_vaults_changed(&mut self, owner: u64) {
        let Some(i) = self.wallet_flow_pre_of(owner) else {
            return;
        };
        match self.wallet_vault_count(owner) {
            None => {
                self.backup_flows.remove(i);
                self.flow_on = None;
            }
            Some(count) => {
                if self.backup_flows[i]
                    .wallet
                    .as_ref()
                    .is_some_and(|w| w.shape.vaults != count)
                {
                    self.backup_flows[i].resize_setup(count);
                }
            }
        }
    }

    /// Make a new wallet or Restore number `owner` has its count of
    /// vaults: the setup opens on its first card, or the one under way
    /// resumes. Nothing about places, paper or which key goes where is
    /// asked: that is Back up's plan, after the keys.
    pub(crate) fn wallet_flow_begin(&mut self, owner: u64) {
        let Some(count) = self.wallet_vault_count(owner) else {
            return;
        };
        if let Some(i) = self.wallet_flow_pre_of(owner) {
            self.wallet_vaults_changed(owner);
            return self.flow_show(i);
        }
        let Some(shape) = self.wallet_placeholder(owner) else {
            return;
        };
        let mut with_vaults = shape.clone();
        with_vaults.vaults = count;
        let answers = plan::Answers::defaults(&with_vaults);
        let mut f = BackupFlow::new_setup(shape, answers, count);
        if let Some(w) = f.wallet.as_mut() {
            w.owner = owner;
        }
        self.backup_flows.push(f);
        let i = self.backup_flows.len() - 1;
        self.flow_show(i);
    }

    /// The wallet just made in Create (or read in Restore) belongs to the
    /// flow that set up its vaults: the checksum and the real shape are
    /// known now, and the plan is Back up's to make.
    pub(crate) fn wallet_flow_attach(&mut self, w: usize, owner: u64) {
        let Some(i) = self.wallet_flow_pre_of(owner) else {
            return;
        };
        let Some(wl) = self.session.wallets.get(w) else {
            return;
        };
        let (cs, name) = (wl.policy.checksum(), wl.name.clone());
        let setup = self.backup_flows[i]
            .wallet
            .as_ref()
            .map_or(0, |wf| wf.shape.vaults);
        let mut shape = self.plan_shape(w);
        shape.vaults = setup;
        // The keys-to-be were numbered by slot, the seeds are numbered as
        // the description lists them (FROST's by share): where a slot's key
        // turned out to be.
        let slots = self.wallet_owner_slots(owner);
        let list = self.backup_seed_list(w);
        let map = seed_map(&slots, &list, &self.session, shape.seeds.len());
        let base = self.wallet_seed_bases(w);
        let f = &mut self.backup_flows[i];
        // The sticks read for the vaults wait for the plan to say where
        // each vault's stick goes.
        f.park_setup();
        let keep = match f.wallet.as_mut() {
            Some(wf) => {
                wf.checksum = Some(cs.clone());
                wf.name = name.clone();
                wf.setup = false;
                wf.answers.remap_seeds(&map, shape.seeds.len());
                let keep = wf.answers.refit(&shape);
                wf.shape = shape;
                wf.base = base;
                keep
            }
            None => Vec::new(),
        };
        f.subject = Some(Subject { id: cs, name });
        // The vaults the flow knows are numbered as the answers number
        // them: they follow the rows that were kept.
        f.remap_vaults(&keep);
        f.wallet_replan();
    }

    /// The keys chosen for the wallet being made by Make a new wallet or
    /// Restore number `owner`, by slot.
    fn wallet_owner_slots(&self, owner: u64) -> Vec<crate::create::Source> {
        match (self.create.as_ref(), self.restore.as_ref()) {
            (Some(c), _) if c.owner == owner => c.slots.clone(),
            (_, Some(r)) if r.owner == owner => r
                .seeds
                .as_ref()
                .map(|s| {
                    s.keys
                        .iter()
                        .map(|fp| crate::create::Source::Here(*fp))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Each seed of wallet `w` as text, without its passphrase: the
    /// fingerprint of the same words alone.
    pub(crate) fn wallet_seed_bases(&self, w: usize) -> Vec<String> {
        use faraday_vault::Record;
        use faraday_vault::records::{self, field, kind as rkind};
        self.backup_seed_list(w)
            .into_iter()
            .map(|(name, k)| {
                let Some(key) = k.and_then(|k| self.session.keys.get(k)) else {
                    return name;
                };
                let Some(words) = key.words.as_ref() else {
                    return name;
                };
                let Ok(m) = osk_bip::bip39::Mnemonic::parse(key.language, words) else {
                    return name;
                };
                let rec = Record::new(rkind::KEY).with(field::KEY, &records::words_payload(&m));
                crate::vault_screens::key_fingerprint(self, &rec).unwrap_or(name)
            })
            .collect()
    }

    /// The vaults are set up: Make a new wallet or Restore goes on to its
    /// keys.
    pub(crate) fn wallet_flow_leave(&mut self, i: usize) {
        let owner = self
            .backup_flows
            .get(i)
            .and_then(|f| f.wallet.as_ref())
            .map_or(0, |w| w.owner);
        if let Some(f) = self.backup_flows.get_mut(i) {
            f.on_screen = false;
        }
        if self.flow_on == Some(i) {
            self.flow_on = None;
        }
        self.vaults.back_to = None;
        self.vaults.create = None;
        self.vaults.focus = None;
        if self.restore.as_ref().is_some_and(|r| r.owner == owner) {
            // A wallet read from its description exists already: it
            // belongs to the setup now.
            if let Some(w) = self.restore.as_ref().and_then(|r| r.wallet) {
                self.wallet_flow_attach(w, owner);
            }
            if let Some(r) = self.restore.as_mut() {
                r.open = Some(crate::rstep::SEEDS);
                r.done[crate::rstep::WHERE as usize] = true;
                r.done[crate::rstep::HOW_MANY as usize] = true;
                r.scroll.follow = true;
            }
            self.screen = Screen::Restore;
            self.dirty = true;
            return;
        }
        let creating = self
            .create
            .as_ref()
            .is_some_and(|c| c.owner == owner && c.built.is_none());
        if creating {
            if let Some(c) = self.create.as_mut() {
                c.open = Some(cstep::KEYS);
                c.scroll.follow = true;
            }
            self.screen = Screen::Create;
        } else if self.backup.is_some() {
            self.screen = Screen::Backup;
        } else {
            self.screen = Screen::Wallets;
        }
        self.dirty = true;
    }

    /// Back on a card of a wallet's flow whose wallet is being made: the
    /// Make a new wallet or Restore it belongs to, on the card it was on.
    /// Returns whether it was.
    pub(crate) fn wallet_flow_back(&mut self, i: usize) -> bool {
        let Some(w) = self.backup_flows[i].wallet.as_ref() else {
            return false;
        };
        if self.backup_flows[i].made || w.checksum.is_some() && self.backup.is_none() {
            return false;
        }
        let owner = w.owner;
        let to = if self.create.as_ref().is_some_and(|c| c.owner == owner) {
            Screen::Create
        } else if self.restore.as_ref().is_some_and(|r| r.owner == owner) {
            Screen::Restore
        } else {
            return false;
        };
        self.backup_flows[i].on_screen = false;
        if self.flow_on == Some(i) {
            self.flow_on = None;
        }
        self.vaults.back_to = None;
        self.vaults.create = None;
        self.vaults.focus = None;
        self.screen = to;
        self.dirty = true;
        true
    }

    // ---- Back up: the plan the flow follows ----

    /// The flow's answers follow the backup's, while the backup is open on
    /// a wallet whose flow has not locked yet. Called after each press.
    pub(crate) fn wallet_flow_sync(&mut self) {
        let Some(i) = self.wallet_flow_of_backup() else {
            return;
        };
        if self.backup_flows[i].made {
            return;
        }
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let shape = self.plan_shape(b.wallet);
        let answers = b.answers.clone();
        let f = &mut self.backup_flows[i];
        let Some(wf) = f.wallet.as_mut() else {
            return;
        };
        if wf.answers.to_text() == answers.to_text() && wf.shape == shape {
            return;
        }
        wf.answers = answers;
        wf.shape = shape;
        f.wallet_replan();
    }

    // ---- the checklist's Write to sticks ----

    /// The names of the files the checklist made for wallet `w`, apart:
    /// the public files and sheets, and the seed files (secrets let out
    /// unprotected). Only those in the Outbox.
    pub(crate) fn wallet_out_files(&self, w: usize) -> (Vec<String>, Vec<String>) {
        let Some(b) = self.backup.as_ref().filter(|b| b.wallet == w) else {
            return (Vec::new(), Vec::new());
        };
        let Some(wl) = self.session.wallets.get(w) else {
            return (Vec::new(), Vec::new());
        };
        let stem = file_stem(&wl.name);
        let in_outbox = |n: &str| self.outbox.iter().any(|i| i.name == n);
        let mut public: Vec<String> = self.backup_public_names_for(w, &b.answers);
        let split = b.answers.split && crate::backup::splits(wl);
        if split {
            public.extend(
                self.outbox
                    .iter()
                    .filter(|i| i.name.starts_with(&format!("{stem}-share-")))
                    .map(|i| i.name.clone()),
            );
        } else {
            public.push(format!("{stem}-backup.pdf"));
        }
        public.retain(|n| in_outbox(n));
        public.dedup();
        // Only this wallet's own seeds: `{stem}-{fp}-words.txt` and its kin
        // for a fingerprint of the wallet's seeds, exactly.
        // and only for a seed the plan keeps as a file now.
        let shape = self.plan_shape(w);
        let kept = b.answers.seeds_as_files(&shape);
        let fps: Vec<String> = self
            .backup_seed_list(w)
            .into_iter()
            .enumerate()
            .filter(|(s, (_, k))| k.is_some() && kept.contains(s))
            .map(|(_, (n, _))| n)
            .collect();
        let seed_files: Vec<String> = self
            .outbox
            .iter()
            .filter(|i| i.secret && seed_file_of(&stem, &fps, &i.name))
            .map(|i| i.name.clone())
            .collect();
        (public, seed_files)
    }

    /// Gives each stick of flow `i` the files it will hold: the public
    /// files and sheets to the stick that carries them, the seed files to
    /// the stick of files.
    fn wallet_assign_files(&mut self, i: usize, w: usize) {
        let (public, seeds) = self.wallet_out_files(w);
        let f = &mut self.backup_flows[i];
        for s in &mut f.slots {
            s.files.clear();
        }
        if let Some(k) = f.public_slot() {
            f.slots[k].files = public;
        }
        // A seed file never goes to the stick that carries the public files.
        let public = f.public_slot();
        if let Some(k) = f
            .slots
            .iter()
            .position(|s| s.target == Target::Files)
            .filter(|k| Some(*k) != public)
        {
            for n in seeds {
                if !f.slots[k].files.contains(&n) {
                    f.slots[k].files.push(n);
                }
            }
        }
    }

    /// **Lock and write**: the checklist's last item. Every other item is
    /// done; the plan is saved, the flow takes the vaults and the files,
    /// the hold on the keys moves to its state, and the lock comes. The
    /// stick visit page is never shown.
    pub(crate) fn wallet_lock_write(&mut self) {
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        if b.stage != BStage::Checklist {
            return;
        }
        let w = b.wallet;
        let items = self.backup_items();
        if !items.contains(&bstep::STICKS) {
            return;
        }
        let others_done = items
            .iter()
            .filter(|&&n| n != bstep::STICKS)
            .all(|&n| bstep::item(n).is_some_and(|it| self.backup_item_done(it)));
        if !others_done {
            return;
        }
        let Some(wl) = self.session.wallets.get(w) else {
            return;
        };
        let (checksum, name) = (wl.policy.checksum(), wl.name.clone());
        let answers = b.answers.clone();
        let shape = self.plan_shape(w);
        let used = answers.vaults_made(&shape);
        // Every vault of the plan is open, and holds what the plan puts
        // in it.
        let mut opens: Vec<(usize, usize)> = Vec::new();
        for &v in &used {
            let taken: Vec<usize> = opens.iter().map(|&(_, o)| o).collect();
            let Some(o) = self.wallet_vault_open(&checksum, v, &taken) else {
                return self.toast(EN.backup_flow_unlock_first);
            };
            opens.push((v, o));
        }
        self.plan_save();
        // The flow of this wallet's backup under way; a flow that is done
        // is replaced, and a new backup writes everything its plan names.
        let mut carry: Vec<String> = Vec::new();
        let i = match self.wallet_flow_active(&checksum) {
            Some(i) => i,
            None => {
                let done: Vec<usize> = (0..self.backup_flows.len())
                    .filter(|&k| {
                        let f = &self.backup_flows[k];
                        f.kind == Kind::Wallet
                            && f.step == Step::Done
                            && f.wallet
                                .as_ref()
                                .is_some_and(|x| x.checksum.as_deref() == Some(checksum.as_str()))
                    })
                    .collect();
                for k in done.into_iter().rev() {
                    let gone = self.backup_flows.remove(k);
                    if gone.waiting()
                        && let Some(w) = gone.wallet
                    {
                        carry.extend(w.held);
                    }
                }
                self.flow_on = None;
                let mut nf = BackupFlow::new_wallet(shape.clone(), answers.clone(), &name);
                let id = self.fresh_owner();
                if let Some(w) = nf.wallet.as_mut() {
                    w.owner = id;
                }
                self.backup_flows.push(nf);
                self.backup_flows.len() - 1
            }
        };
        {
            let f = &mut self.backup_flows[i];
            if let Some(wf) = f.wallet.as_mut() {
                wf.checksum = Some(checksum.clone());
                wf.name = name.clone();
                wf.answers = answers;
                wf.shape = shape;
            }
            f.subject = Some(Subject { id: checksum, name });
            f.wallet_replan();
        }
        // Each vault of the plan is the open vault that holds it; a new
        // seal is made of each, so that a copy for every stick is there.
        for &(v, o) in &opens {
            let (salt, vname) = {
                let open = &self.vaults.open[o];
                (open.header().salt, open.name.clone())
            };
            let f = &mut self.backup_flows[i];
            if f.plan.vaults.len() <= v {
                f.plan
                    .vaults
                    .resize(v + 1, crate::backup_flow::VaultPlan::default());
            }
            let vp = &mut f.plan.vaults[v];
            if vp.salt != Some(salt) {
                vp.existing = vp.salt.is_none();
                vp.salt = Some(salt);
                vp.file = Some(vname);
            }
            self.vaults.open[o].changes += 1;
        }
        let base = self.wallet_seed_bases(w);
        if let Some(wf) = self.backup_flows[i].wallet.as_mut() {
            wf.base = base;
        }
        self.wallet_hold_move(i);
        if let Some(wf) = self.backup_flows[i].wallet.as_mut() {
            for t in carry {
                if !wf.held.contains(&t) {
                    wf.held.push(t);
                }
            }
        }
        self.wallet_assign_files(i, w);
        let f = &mut self.backup_flows[i];
        if let Some(wf) = f.wallet.as_mut() {
            wf.sealed = used.iter().map(|&v| (v, sealed_marks(wf, v))).collect();
        }
        f.made = true;
        f.made_in = used;
        f.adding = false;
        f.step = Step::Lock;
        f.sealing.clear();
        self.flow_show(i);
        self.flow_act(crate::backup_flow_steps::FlowAction::LockWrite);
    }

    /// The open vault that holds what the plan puts in plan vault `v`: the
    /// vault the flow knows, else the one that fits.
    fn wallet_vault_open(&self, checksum: &str, v: usize, taken: &[usize]) -> Option<usize> {
        // The vault the flow made or chose for `v` stays its vault while it
        // fits; the lock never takes another open vault in its place.
        if let Some(i) = self.wallet_flow_active(checksum)
            && let Some(o) = self.flow_open_index(i, v)
        {
            return (self.backup_open_fits(v, o) && !taken.contains(&o)).then_some(o);
        }
        // Two planned vaults are never one open vault.
        (0..self.vaults.open.len()).find(|&o| !taken.contains(&o) && self.backup_open_fits(v, o))
    }

    /// F6's hold on the keys the plan keeps in a vault moves from the
    /// copies to the flow's state: the vaults sealed now say the backup is
    /// done, and the keys spend nothing until the last stick is written.
    fn wallet_hold_move(&mut self, i: usize) {
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let w = b.wallet;
        let shape = self.plan_shape(w);
        let list = self.backup_seed_list(w);
        let held = self.backup_held_seeds();
        // A seed kept on any stick, a vault's or a file's, waits for it.
        let on_stick = |s: usize| b.answers.seed_on_stick(&shape, s);
        let mut moved: Vec<(Fingerprint, usize)> = Vec::new();
        for s in held.into_iter().filter(|&s| on_stick(s)) {
            if let Some(k) = list.get(s).and_then(|(_, k)| *k)
                && let Some(key) = self.session.keys.get(k)
            {
                moved.push((key.master.fingerprint(), k));
            }
        }
        for (fp, k) in moved {
            // The same words without that passphrase are covered by the
            // same copy.
            let words = self.session.keys[k].words.clone();
            let same: Vec<Fingerprint> = self
                .session
                .keys
                .iter()
                .filter(|o| o.words.is_some() && o.words == words)
                .map(|o| o.master.fingerprint())
                .collect();
            self.made_here.retain(|f| *f != fp && !same.contains(f));
            for f in same.iter().chain(std::iter::once(&fp)) {
                self.held_clear_marks(*f, k);
            }
            if let Some(wf) = self.backup_flows[i].wallet.as_mut() {
                for f in same.iter().chain(std::iter::once(&fp)) {
                    let t = fp_text(*f);
                    if !wf.held.contains(&t) {
                        wf.held.push(t);
                    }
                }
            }
        }
    }

    /// The seed of wallet flow `i` that key `fp` is: by the fingerprint its
    /// seed is named by, or the one of its words alone.
    fn wallet_seed_of_key(&self, i: usize, fp: &str) -> Option<usize> {
        let wf = self.backup_flows[i].wallet.as_ref()?;
        (0..wf.shape.seeds.len()).find(|&s| {
            wf.shape.seeds[s].name.eq_ignore_ascii_case(fp)
                || wf.base.get(s).is_some_and(|b| b.eq_ignore_ascii_case(fp))
        })
    }

    /// Whether the key `fp` the wallet flow `i` holds back may be spent: every
    /// stick of the plan is written (NEW-WALLET §14.3), and the seed has
    /// sticks of its own among them, all written. The second is a check
    /// against a plan that shrank, not a rule of its own.
    pub(crate) fn wallet_seed_released(&self, i: usize, fp: Fingerprint) -> bool {
        let f = &self.backup_flows[i];
        if !f.all_written() {
            return false;
        }
        let Some(s) = self.wallet_seed_of_key(i, &fp_text(fp)) else {
            return true;
        };
        let spots = f.seed_slots(s);
        !spots.is_empty() && spots.iter().all(|&k| f.slots[k].written.is_some())
    }

    /// The seeds of flow `i`'s wallet whose keys it holds back.
    fn wallet_held_seeds(&self, i: usize) -> Vec<usize> {
        let Some(wf) = self.backup_flows[i].wallet.as_ref() else {
            return Vec::new();
        };
        wf.held
            .iter()
            .filter_map(|h| self.wallet_seed_of_key(i, h))
            .collect()
    }

    /// The flow's wallet, if loaded: its place among the session's wallets.
    pub(crate) fn wallet_flow_wallet(&self, i: usize) -> Option<usize> {
        let cs = self
            .backup_flows
            .get(i)?
            .wallet
            .as_ref()?
            .checksum
            .clone()?;
        self.session
            .wallets
            .iter()
            .position(|w| w.policy.checksum() == cs)
    }

    /// The sticks are written: the vault unlocked from Done shows the
    /// wallet's checklist, done, with nothing For the stick.
    pub(crate) fn wallet_flow_land(&mut self, i: usize) {
        // The vault just unlocked loads its keys and wallets, as an unlock
        // that lands on Wallets does.
        if self.wallet_flow_wallet(i).is_none() && self.may_load_keys() {
            let o = self.vaults.current;
            let _ = self.vault_load(o, true);
        }
        let Some(w) = self.wallet_flow_wallet(i) else {
            return;
        };
        self.backup_flows[i].live.open = Some(Step::Done.n());
        self.act(crate::Action::BackupChecklist(w));
    }

    /// The seeds, by name, that the vaults added in the round take and that
    /// are not here to copy: kept only on paper, since the lock wiped them.
    pub(crate) fn wallet_round_missing(&self, i: usize) -> Vec<String> {
        let f = &self.backup_flows[i];
        let Some(wf) = f.wallet.as_ref() else {
            return Vec::new();
        };
        let list = self
            .wallet_flow_wallet(i)
            .map(|w| self.backup_seed_list(w))
            .unwrap_or_default();
        let mut out: Vec<usize> = f
            .plan
            .used_vaults()
            .into_iter()
            .filter(|v| !f.made_in.contains(v))
            .flat_map(|v| wf.answers.vault_seeds(&wf.shape, v))
            .filter(|&s| list.get(s).and_then(|(_, k)| *k).is_none())
            .collect();
        out.sort_unstable();
        out.dedup();
        out.into_iter()
            .filter_map(|s| wf.shape.seeds.get(s).map(|x| x.name.clone()))
            .collect()
    }

    /// The copy of the keys a vault added after the keys were made takes
    /// from one that holds them (§4.8): each seed the plan puts in it, and
    /// the wallet description where the plan puts it.
    pub(crate) fn flow_wallet_copy(&mut self, i: usize) {
        use crate::vault_screens::key_fingerprint;
        use faraday_vault::Record;
        use faraday_vault::records::{field, kind as rkind};
        let Some(wf) = self.backup_flows[i].wallet.clone() else {
            return;
        };
        let Some(cs) = wf.checksum.clone() else {
            return;
        };
        let added: Vec<usize> = self.backup_flows[i]
            .plan
            .used_vaults()
            .into_iter()
            .filter(|v| !self.backup_flows[i].made_in.contains(v))
            .collect();
        // The vaults that hold the keys, open: what is copied is read from
        // their records, and never leaves them but into the added vaults.
        let holders: Vec<usize> = self.backup_flows[i]
            .plan
            .used_vaults()
            .into_iter()
            .filter(|v| self.backup_flows[i].made_in.contains(v))
            .filter_map(|v| self.flow_open_index(i, v))
            .collect();
        // A record of this wallet's description, by its checksum.
        let of_wallet = |t: &str| {
            crate::wallet::read_wallet(t)
                .ok()
                .is_some_and(|p| p.checksum() == cs)
        };
        // Everything is gathered first: every added vault takes what it
        // needs, or none does.
        struct Work {
            o: usize,
            add: Vec<Record>,
            typed: Vec<(usize, bool)>,
            out: Vec<usize>,
        }
        let mut work: Vec<Work> = Vec::new();
        for &v in &added {
            let Some(o) = self.flow_open_index(i, v) else {
                return self.toast(EN.backup_flow_unlock_first);
            };
            let mut w = Work {
                o,
                add: Vec::new(),
                typed: Vec::new(),
                out: Vec::new(),
            };
            for s in wf.answers.vault_seeds(&wf.shape, v) {
                let name = wf
                    .shape
                    .seeds
                    .get(s)
                    .map(|x| x.name.clone())
                    .unwrap_or_default();
                let base = wf.base.get(s).cloned().unwrap_or_else(|| name.clone());
                // The plan puts the passphrase in this vault, or keeps it
                // apart: the record is made so.
                let want_pass = wf.shape.seeds.get(s).is_some_and(|x| x.passphrase)
                    && wf.answers.pass_in_vault(s);
                let is_seed = |app: &Faraday, r: &Record| {
                    let fp = key_fingerprint(app, r);
                    fp.as_deref() == Some(name.as_str()) || fp.as_deref() == Some(base.as_str())
                };
                let find = |app: &Faraday, h: usize| -> Vec<(usize, Record)> {
                    app.vaults.open.get(h).map_or_else(Vec::new, |open| {
                        open.contents
                            .records
                            .iter()
                            .enumerate()
                            .filter(|(_, r)| r.kind == rkind::KEY && is_seed(app, r))
                            .map(|(k, r)| (k, r.clone()))
                            .collect()
                    })
                };
                let has_pass = |r: &Record| r.field(field::KEY_PASSPHRASE).is_some();
                let there = find(self, o);
                if let Some((_, r)) = there.first()
                    && has_pass(r) == want_pass
                {
                    continue;
                }
                // The record as the plan wants it: from a holder (the
                // passphrase taken off where the plan keeps it apart), or
                // from the key typed back.
                let from_holder = holders.iter().find_map(|&h| {
                    find(self, h).into_iter().find_map(|(_, r)| {
                        if has_pass(&r) == want_pass {
                            Some(r)
                        } else if !want_pass {
                            let mut r = r;
                            r.fields.retain(|f| f.number != field::KEY_PASSPHRASE);
                            Some(r)
                        } else {
                            None
                        }
                    })
                });
                match from_holder {
                    Some(r) => w.add.push(r),
                    None => {
                        let want = if want_pass { &name } else { &base };
                        let k = self.session.keys.iter().position(|k| {
                            k.words.is_some()
                                && fp_text(k.master.fingerprint()).eq_ignore_ascii_case(want)
                        });
                        match k {
                            Some(k) => w.typed.push((k, want_pass)),
                            // A seed kept only on paper: typed back from its
                            // checked copy first.
                            None => return self.toast(EN.backup_flow_key_on_paper),
                        }
                    }
                }
                w.out.extend(there.iter().map(|(k, _)| *k));
            }
            if wf.answers.wallet_in_vault(v) {
                let there: Vec<String> = self.vaults.open[o]
                    .contents
                    .of(rkind::WALLET)
                    .filter_map(|(_, r)| r.text(field::WALLET).map(str::to_string))
                    .collect();
                for h in &holders {
                    for (_, r) in self.vaults.open[*h].contents.of(rkind::WALLET) {
                        let t = r.text(field::WALLET).unwrap_or_default().to_string();
                        if of_wallet(&t)
                            && !there.contains(&t)
                            && !w.add.iter().any(|x| x.text(field::WALLET) == Some(&t))
                        {
                            w.add.push(r.clone());
                        }
                    }
                }
                // No vault of the round has it: the description file the
                // checklist wrote, kept in Files, is it.
                if there.is_empty() && !w.add.iter().any(|x| x.kind == rkind::WALLET) {
                    let found = self
                        .inbox
                        .iter()
                        .chain(self.outbox.iter())
                        .filter(|it| it.kind == crate::wallet::FileKind::Wallet)
                        .filter_map(|it| String::from_utf8(it.bytes.clone()).ok())
                        .find(|t| of_wallet(t));
                    match found {
                        Some(t) => w.add.push(
                            Record::new(rkind::WALLET)
                                .with(field::WALLET, t.trim().as_bytes())
                                .with(field::WALLET_NAME, wf.name.as_bytes()),
                        ),
                        None => return self.toast(EN.backup_flow_no_description),
                    }
                }
            }
            // The plan record goes with them, so the next backup finds it:
            // this wallet's, and nothing else of the holder's.
            let has_plan = self.vaults.open[o]
                .contents
                .of(rkind::PLAN)
                .any(|(_, r)| r.text(field::PLAN_WALLET).is_some_and(&of_wallet));
            if !has_plan
                && let Some(r) = holders.iter().find_map(|&h| {
                    self.vaults.open[h]
                        .contents
                        .of(rkind::PLAN)
                        .find(|(_, r)| r.text(field::PLAN_WALLET).is_some_and(&of_wallet))
                        .map(|(_, r)| r.clone())
                })
            {
                w.add.push(r);
            }
            let open = &self.vaults.open[o];
            let mut trial = open.contents.clone();
            let mut gone = w.out.clone();
            gone.sort_unstable();
            for k in gone.into_iter().rev() {
                if k < trial.records.len() {
                    trial.records.remove(k);
                }
            }
            trial.records.extend(w.add.iter().cloned());
            if trial.used() > open.header().slot_len as usize {
                return self.toast(EN.vaults_that_does_not);
            }
            work.push(w);
        }
        for w in work {
            let open = &mut self.vaults.open[w.o];
            let mut gone = w.out.clone();
            gone.sort_unstable();
            gone.dedup();
            for k in gone.into_iter().rev() {
                if k < open.contents.records.len() {
                    open.contents.records.remove(k);
                    open.changes += 1;
                }
            }
            if !w.add.is_empty() {
                open.contents.records.extend(w.add);
                open.changes += 1;
            }
            self.vaults.current = w.o;
            for (k, with) in w.typed {
                self.vault_act(if with {
                    V::SaveKeyWithPassphrase(k)
                } else {
                    V::SaveKey(k)
                });
            }
        }
        // The plan record of every vault of the wallet says the plan as it
        // is now.
        let text = wf.answers.to_text();
        for v in self.backup_flows[i].plan.used_vaults() {
            let Some(o) = self.flow_open_index(i, v) else {
                continue;
            };
            let open = &mut self.vaults.open[o];
            let mut changed = false;
            for r in open.contents.records.iter_mut() {
                if r.kind == rkind::PLAN
                    && r.text(field::PLAN_WALLET).is_some_and(&of_wallet)
                    && r.text(field::PLAN_ANSWERS) != Some(text.as_str())
                {
                    r.set(field::PLAN_ANSWERS, text.as_bytes());
                    changed = true;
                }
            }
            if changed {
                open.changes += 1;
            }
        }
        let f = &mut self.backup_flows[i];
        if let Some(wf) = f.wallet.as_mut() {
            for &v in &added {
                let marks = sealed_marks(wf, v);
                wf.sealed.retain(|(x, _)| *x != v);
                wf.sealed.push((v, marks));
            }
        }
        f.made_in.extend(added);
        f.adding = false;
    }

    // ---- what the checklist and the visit say of the writes ----

    /// The sticks the wallet being backed up will be written to, by place,
    /// each with the files it will hold and whether it is written: from the
    /// flow, or from the plan's own while no flow has locked yet.
    pub fn wallet_stick_rows(&self) -> Vec<(String, Vec<String>, Option<String>)> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let Some(wl) = self.session.wallets.get(b.wallet) else {
            return Vec::new();
        };
        let m = self.medium.noun();
        let (public, seeds) = self.wallet_out_files(b.wallet);
        let shape = self.plan_shape(b.wallet);
        let own = || BackupFlow::new_wallet(shape.clone(), b.answers.clone(), &wl.name);
        let mut flow = match self.wallet_flow_of_backup() {
            Some(i) if self.backup_flows[i].step != Step::Done || self.wallet_sticks_done() => {
                self.backup_flows[i].clone()
            }
            _ => own(),
        };
        if !flow.made {
            flow.wallet_replan();
            if let Some(k) = flow.public_slot() {
                flow.slots[k].files = public;
            }
            if let Some(k) = flow.slots.iter().position(|s| s.target == Target::Files) {
                for n in seeds {
                    if !flow.slots[k].files.contains(&n) {
                        flow.slots[k].files.push(n);
                    }
                }
            }
        }
        flow.slots
            .iter()
            .map(|s| {
                (
                    flow.target_short(&s.target, m),
                    s.files.clone(),
                    s.written.as_ref().map(|w| w.label.clone()),
                )
            })
            .collect()
    }

    /// Where a file of the wallet being backed up was written by its flow:
    /// the stick's label.
    pub(crate) fn wallet_written_to(&self, name: &str) -> Option<String> {
        let i = self.wallet_flow_of_backup()?;
        self.backup_flows[i]
            .slots
            .iter()
            .filter_map(|s| s.written.as_ref())
            .find(|w| w.files.iter().any(|(n, _)| same_file(n, name)))
            .map(|w| w.label.clone())
    }

    /// Whether the flow wrote a file of the wallet being backed up.
    pub(crate) fn wallet_wrote(&self, name: &str) -> bool {
        self.wallet_written_to(name).is_some()
    }

    /// "Written to {stick}" for the files of a checklist item, when its
    /// files were written by the flow.
    pub fn wallet_item_written(&self, names: &[String]) -> Option<String> {
        let first = names.first()?;
        let label = self.wallet_written_to(first)?;
        Some(fill(EN.backup_flow_written_to, &[&label]))
    }

    /// The `PublicOn` the wallet's plan carries its public files on, for
    /// the card that names the stick.
    pub fn wallet_public_on(&self) -> Option<PublicOn> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape(b.wallet);
        Some(crate::backup_flow::plan_of(&shape, &b.answers).public_on)
    }
}

/// Whether `written` is the file `want` as a stick keeps it: the same
/// name, or the name beside what the stick held, "name-2.ext".
fn same_file(written: &str, want: &str) -> bool {
    if written.eq_ignore_ascii_case(want) {
        return true;
    }
    let (ws, we) = written.rsplit_once('.').unwrap_or((written, ""));
    let (ts, te) = want.rsplit_once('.').unwrap_or((want, ""));
    we.eq_ignore_ascii_case(te)
        && ws
            .strip_prefix(ts)
            .and_then(|r| r.strip_prefix('-'))
            .is_some_and(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
}

// ---- Make a new wallet's cards before the keys, and the plan's answers ----

impl Faraday {
    /// The cards Make a new wallet shows: Kind, Quorum, the question of
    /// vaults, and, when the answer is yes, how many and the two cards of
    /// their setup (Bring in the sticks, Vaults), then Keys.
    pub fn create_cards(&self) -> Vec<u8> {
        let Some(c) = self.create.as_ref() else {
            return Vec::new();
        };
        let mut v = crate::create_steps(c.kind);
        if c.into_vaults() {
            let at = v
                .iter()
                .position(|&k| k == cstep::WHERE)
                .map_or(v.len(), |p| p + 1);
            let mut extra = vec![cstep::HOW_MANY];
            if self.wallet_flow_of_owner(c.owner).is_some() {
                extra.extend([cstep::STICKS, cstep::VAULTS]);
            }
            v.splice(at..at, extra);
        }
        v
    }

    /// Whether Make a new wallet's card `k` is done. The question and the
    /// count are done only when the person has pressed their own Continue;
    /// the two cards of the setup are, once the flow is past them.
    pub fn create_card_done(&self, k: u8) -> bool {
        let Some(c) = self.create.as_ref() else {
            return false;
        };
        let at = |step: Step| {
            self.wallet_flow_of_owner(c.owner)
                .is_some_and(|i| self.backup_flows[i].step > step)
        };
        // Once the wallet is made, the two cards of the setup are over;
        // the question and the count are done only by their own Continue.
        if c.built.is_some() && matches!(k, cstep::STICKS | cstep::VAULTS) {
            return true;
        }
        match k {
            cstep::STICKS => at(Step::Sticks),
            cstep::VAULTS => at(Step::Vaults),
            _ => c.done[k as usize],
        }
    }

    /// Opens the first card of Make a new wallet that is not done.
    pub(crate) fn create_open_next(&mut self) {
        let cards = self.create_cards();
        let pending = self
            .create
            .as_ref()
            .is_some_and(|c| self.setup_pending(c.owner, c.into_vaults(), c.built.is_some()));
        let next = cards
            .iter()
            .copied()
            .find(|&k| !self.create_card_done(k) && !(pending && k == cstep::KEYS));
        if let Some(c) = self.create.as_mut() {
            c.open = next;
            c.scroll.follow = true;
        }
    }

    /// The vault part of the wallet's plan has a vault with no place for its
    /// stick: the plan waits (`docs/NEW-WALLET.md` §14.2).
    pub(crate) fn flow_wallet_stickless(&self, i: usize) -> bool {
        self.backup_flows[i]
            .wallet
            .as_ref()
            .is_some_and(|w| w.answers.stickless(&w.shape).is_some())
    }

    /// A wallet's answer to a question of the plan, as Back up ticks it. A
    /// stick already written is not taken off the plan from here.
    pub(crate) fn flow_wallet_answer(&mut self, i: usize, list: u8, row: u8) {
        let Some(q) = crate::qrow::question(list) else {
            return;
        };
        let mut trial = self.backup_flows[i].clone();
        {
            let Some(w) = trial.wallet.as_mut() else {
                return;
            };
            let shape = w.shape.clone();
            w.answers.toggle(&shape, q, usize::from(row));
        }
        trial.wallet_replan();
        let written: Vec<Target> = self.backup_flows[i]
            .slots
            .iter()
            .filter(|s| s.written.is_some())
            .map(|s| s.target)
            .collect();
        if !written
            .iter()
            .all(|t| trial.slots.iter().any(|s| s.target == *t))
        {
            // A stick already written stays on the plan until its copy is
            // destroyed.
            return self.toast(EN.backup_flow_written_stays);
        }
        // A key held back for its sticks stays kept on one, with its
        // passphrase, and every vault has a stick to be written to.
        if self.backup_flows[i].made {
            let held = self.wallet_held_seeds(i);
            let bad = trial.wallet.as_ref().is_some_and(|wf| {
                wf.answers.stickless(&wf.shape).is_some()
                    || held.iter().any(|&s| trial.seed_slots(s).is_empty())
                    || plan::kept_nowhere(&wf.shape, &wf.answers, &held).is_some()
            });
            if bad {
                return self.toast(EN.backup_flow_held_stays);
            }
        }
        // The screen's state stays; the plan follows the answers.
        trial.live = self.backup_flows[i].live.clone();
        trial.live.reading = None;
        if trial.made {
            // What a vault must hold changed: it is as one added, now.
            let step = trial.step;
            trial.made_plan_changed(false);
            if trial.step != step {
                trial.live.open = Some(trial.step.n());
            }
        }
        self.backup_flows[i] = trial;
        // The checklist's own answers follow, where it is open.
        if let Some(wf) = self.backup_flows[i].wallet.clone()
            && let Some(w) = self.wallet_flow_wallet(i)
            && let Some(b) = self.backup.as_mut().filter(|b| b.wallet == w)
        {
            b.answers = wf.answers;
        }
    }

    /// Another place, for a wallet.
    pub(crate) fn flow_wallet_add_place(&mut self, i: usize) {
        let places = self.backup_flows[i]
            .wallet
            .as_ref()
            .map_or(1, |w| w.answers.places);
        self.flow_wallet_answer(i, crate::qrow::PLACES, (places + 1).min(255) as u8);
    }

    /// **Add a passphrase** on the plan's check lines. Create opens on
    /// Keys with the key in its slot and the passphrase fields open: the
    /// key with a passphrase is another key, and its wallet another
    /// wallet.
    pub(crate) fn plan_add_passphrase(&mut self) {
        // The wallet shown: the checklist's on Back up, else the one
        // chosen.
        let w = match self.backup.as_ref() {
            Some(b) if self.screen == Screen::Backup => b.wallet,
            _ => self.wallet,
        };
        let Some(wl) = self.session.wallets.get(w) else {
            return;
        };
        let Some(kind) = crate::create::NewKind::of(&wl.policy) else {
            return;
        };
        let Some(fp) = wl.policy.keys().first().and_then(|k| k.fingerprint()) else {
            return;
        };
        let (m, n) = crate::wallet::Session::quorum(wl);
        let owner = self.fresh_owner();
        self.create = Some(crate::CreateState {
            owner,
            open: Some(cstep::KEYS),
            m,
            n,
            kind,
            slots: vec![crate::create::Source::Here(fp.0)],
            pass_slot: Some(0),
            pass_focus: Some(0),
            ..crate::CreateState::default()
        });
        if let Some(c) = self.create.as_mut() {
            c.done[cstep::WHERE as usize] = true;
            c.vaults.answer = Some(false);
        }
        self.screen = Screen::Create;
    }
}

/// Where each key-to-be of a wallet being made turned out to be among the
/// wallet's seeds: a slot holding a key from words is the seed that key is;
/// the others take the seeds left, in order.
fn seed_map(
    slots: &[crate::create::Source],
    list: &[(String, Option<usize>)],
    session: &crate::wallet::Session,
    n: usize,
) -> Vec<usize> {
    let mut map = vec![usize::MAX; slots.len().max(n)];
    let mut used = vec![false; n];
    for (slot, src) in slots.iter().enumerate() {
        if let crate::create::Source::Here(fp) = src
            && let Some(s) = list.iter().position(|(_, k)| {
                k.and_then(|k| session.keys.get(k))
                    .is_some_and(|key| key.master.fingerprint().0 == *fp)
            })
        {
            map[slot] = s;
            used[s] = true;
        }
    }
    let mut free = (0..n).filter(|s| !used[*s]);
    for m in map.iter_mut().filter(|m| **m == usize::MAX) {
        if let Some(s) = free.next() {
            *m = s;
        }
    }
    map
}

/// A mark, among what a vault was sealed with, for the wallet description.
const WALLET_MARK: usize = 1_000_000;
/// A mark for seed `s`'s passphrase being in the vault: this plus `s`.
const PASS_MARK: usize = 2_000_000;

/// What vault `v` of the plan holds: its seeds, the wallet description where
/// the plan puts it there, and each passphrase the plan puts in with its
/// seed. A plan changed after the lock that asks for more is a round.
pub(crate) fn sealed_marks(wf: &crate::backup_flow::WalletFlow, v: usize) -> Vec<usize> {
    let seeds = wf.answers.vault_seeds(&wf.shape, v);
    let mut marks = seeds.clone();
    if wf.answers.wallet_in_vault(v) {
        marks.push(WALLET_MARK);
    }
    for s in seeds {
        if wf.shape.seeds.get(s).is_some_and(|x| x.passphrase) && wf.answers.pass_in_vault(s) {
            marks.push(PASS_MARK + s);
        }
    }
    marks
}

/// Whether `name` is a seed file of this wallet: `{stem}-{fp}-words.txt`,
/// `-seedqr.png` or `-compactseedqr.png` for one of its seeds `fps`,
/// exactly.
pub(crate) fn seed_file_of(stem: &str, fps: &[String], name: &str) -> bool {
    fps.iter().any(|fp| {
        let head = format!("{stem}-{}-", fp.to_lowercase());
        name.strip_prefix(&head)
            .is_some_and(|rest| ["words.txt", "seedqr.png", "compactseedqr.png"].contains(&rest))
    })
}

impl BackupFlow {
    /// A plan changed after the keys were made: a vault that now holds
    /// more than it was sealed with is as one added (the round puts it in
    /// and its sticks are written again), and a vault the plan has and the
    /// flow has not made starts the round. With `advance`, a flow that has
    /// not reached the writes goes on to the lock.
    pub(crate) fn made_plan_changed(&mut self, advance: bool) {
        let f = self;
        let stale: Vec<usize> = match f.wallet.as_ref() {
            Some(wf) => f
                .plan
                .used_vaults()
                .into_iter()
                .filter(|v| f.made_in.contains(v))
                .filter(|v| {
                    let had = wf
                        .sealed
                        .iter()
                        .find(|(x, _)| x == v)
                        .map_or(&[][..], |(_, s)| s.as_slice());
                    sealed_marks(wf, *v).iter().any(|s| !had.contains(s))
                })
                .collect(),
            None => Vec::new(),
        };
        if !stale.is_empty() {
            f.made_in.retain(|v| !stale.contains(v));
            f.sealing.retain(|s| !stale.contains(&s.vault));
            for k in 0..f.slots.len() {
                if f.vaults_of(k).iter().any(|v| stale.contains(v)) {
                    f.slots[k].written = None;
                }
            }
        }
        let missing: Vec<usize> = f
            .plan
            .used_vaults()
            .into_iter()
            .filter(|v| !f.made_in.contains(v))
            .collect();
        if !missing.is_empty() {
            f.adding = true;
            let bind = (0..f.slots.len()).any(|s| {
                let vs = f.vaults_of(s);
                f.slots[s].bound.is_none()
                    && vs.iter().any(|v| {
                        missing.contains(v)
                            && f.plan.vaults[*v].existing
                            && f.plan.vaults[*v].salt.is_none()
                    })
            });
            f.step = if bind { Step::Sticks } else { Step::Vaults };
        } else if advance && f.step < Step::Write && f.step != Step::Lock {
            f.step = Step::Lock;
        }
    }

    /// The slots whose write keeps seed `s` on a stick: those of the
    /// vaults that hold it, and the stick of files for its seed file.
    pub(crate) fn seed_slots(&self, s: usize) -> Vec<usize> {
        let Some(wf) = self.wallet.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<usize> = Vec::new();
        for v in wf.answers.vaults_made(&wf.shape) {
            if wf.answers.vault_seeds(&wf.shape, v).contains(&s) {
                out.extend((0..self.slots.len()).filter(|&k| self.vaults_of(k).contains(&v)));
            }
        }
        if wf.answers.seeds_as_files(&wf.shape).contains(&s) {
            out.extend((0..self.slots.len()).filter(|&k| self.slots[k].target == Target::Files));
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `name` is a seed file of the wallet this flow is for.
    pub fn seed_file_ok(&self, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let kept = w.answers.seeds_as_files(&w.shape);
        let fps: Vec<String> = w
            .shape
            .seeds
            .iter()
            .enumerate()
            .filter(|(s, x)| x.here && kept.contains(s))
            .map(|(_, s)| s.name.clone())
            .collect();
        seed_file_of(&file_stem(&w.name), &fps, name)
    }
}

impl Faraday {
    /// The checklist's writes are done: the flow wrote every stick the plan
    /// as it is now names, so a plan changed since is not done.
    pub(crate) fn wallet_sticks_done(&self) -> bool {
        let Some(i) = self.wallet_flow_of_backup() else {
            return false;
        };
        let f = &self.backup_flows[i];
        if !f.all_written() {
            return false;
        }
        let Some(b) = self.backup.as_ref() else {
            return false;
        };
        let shape = self.plan_shape(b.wallet);
        let now = BackupFlow::new_wallet(shape, b.answers.clone(), "");
        now.slots.iter().all(|n| {
            f.slots
                .iter()
                .any(|s| s.target == n.target && s.written.is_some())
        })
    }

    /// "Written to {stick}" for the wallet's seed files, once the flow wrote
    /// them.
    pub fn wallet_seeds_written(&self) -> Option<String> {
        let b = self.backup.as_ref()?;
        let stem = file_stem(&self.session.wallets.get(b.wallet)?.name);
        let fps: Vec<String> = self
            .backup_seed_list(b.wallet)
            .into_iter()
            .filter(|(_, k)| k.is_some())
            .map(|(n, _)| n)
            .collect();
        self.wallet_written_where(&|n| seed_file_of(&stem, &fps, n))
            .map(|l| fill(EN.backup_flow_written_to, &[&l]))
    }

    /// "Written to {stick}" for the wallet's shares, once the flow wrote
    /// them.
    pub fn wallet_shares_written(&self) -> Option<String> {
        let b = self.backup.as_ref()?;
        let stem = file_stem(&self.session.wallets.get(b.wallet)?.name);
        let head = format!("{stem}-share-");
        self.wallet_written_where(&|n| n.starts_with(&head))
            .map(|l| fill(EN.backup_flow_written_to, &[&l]))
    }

    /// Where a file of the wallet whose name satisfies `is` was written by
    /// its flow: the stick's label.
    pub(crate) fn wallet_written_where(&self, is: &dyn Fn(&str) -> bool) -> Option<String> {
        let i = self.wallet_flow_of_backup()?;
        self.backup_flows[i]
            .slots
            .iter()
            .filter_map(|s| s.written.as_ref())
            .find(|w| w.files.iter().any(|(n, _)| is(n)))
            .map(|w| w.label.clone())
    }
}

// ---- Make a new wallet and Restore across the lock that makes the process clean ----

impl Faraday {
    /// Before a lock: the non-secret state of the Make a new wallet or
    /// Restore that asked for a setup still under way goes into its flow,
    /// so the lock does not orphan the flow. Nothing secret is kept: the
    /// keys typed or made so far go with the lock.
    pub(crate) fn flow_keep_makers(&mut self) {
        for i in 0..self.backup_flows.len() {
            let Some(w) = self.backup_flows[i].wallet.as_ref() else {
                continue;
            };
            if !w.setup || w.checksum.is_some() || w.owner == 0 {
                continue;
            }
            let owner = w.owner;
            let kind_at = |k: crate::create::NewKind| {
                crate::create::NewKind::ALL
                    .iter()
                    .position(|x| *x == k)
                    .unwrap_or(0) as u8
            };
            let again = if let Some(c) = self
                .create
                .as_ref()
                .filter(|c| c.owner == owner && c.built.is_none())
            {
                Some(crate::backup_flow::Again {
                    restore: false,
                    kind: kind_at(c.kind),
                    m: c.m,
                    n: c.n,
                })
            } else {
                self.restore.as_ref().filter(|r| r.owner == owner).map(|r| {
                    crate::backup_flow::Again {
                        restore: true,
                        kind: kind_at(r.kind()),
                        m: r.seeds.as_ref().map_or(1, |s| s.m),
                        n: r.seeds.as_ref().map_or(1, |s| s.n),
                    }
                })
            };
            if let Some(wf) = self.backup_flows[i].wallet.as_mut() {
                wf.again = again.or(wf.again.take());
            }
        }
    }

    /// After a lock: the Make a new wallet or Restore a setup came from is
    /// there again, past the questions it was past, to take the keys when
    /// the setup is done.
    pub(crate) fn flow_makers_return(&mut self) {
        for i in 0..self.backup_flows.len() {
            let Some(w) = self.backup_flows[i].wallet.as_ref() else {
                continue;
            };
            let (Some(a), true, true) = (w.again.clone(), w.setup, w.checksum.is_none()) else {
                continue;
            };
            let (owner, count) = (w.owner, w.shape.vaults.max(1));
            let kind = crate::create::NewKind::ALL
                .get(usize::from(a.kind))
                .copied()
                .unwrap_or_default();
            let ask = crate::VaultAsk {
                answer: Some(true),
                count,
            };
            if a.restore {
                let mut r = crate::RestoreState::new();
                r.owner = owner;
                r.vaults = ask;
                if let Some(s) = r.seeds.as_mut() {
                    s.fix(kind);
                    s.set_n(a.n);
                    s.set_m(a.m);
                }
                for k in [
                    crate::rstep::KIND,
                    crate::rstep::QUORUM,
                    crate::rstep::DESCRIPTION,
                    crate::rstep::WHERE,
                    crate::rstep::HOW_MANY,
                ] {
                    r.done[k as usize] = true;
                }
                r.open = None;
                self.restore = Some(r);
            } else {
                let mut c = crate::CreateState {
                    owner,
                    kind,
                    m: a.m,
                    n: a.n,
                    vaults: ask,
                    ..crate::CreateState::default()
                };
                let slots = match kind {
                    k if k.threshold() => a.m,
                    k if k.multi() => a.n,
                    _ => 1,
                };
                c.slots = vec![crate::create::Source::Empty; slots];
                for k in [cstep::KIND, cstep::QUORUM, cstep::WHERE, cstep::HOW_MANY] {
                    c.done[k as usize] = true;
                }
                self.create = Some(c);
                self.create_open_next();
            }
            // The flow's own setup is what the person continues with.
            return;
        }
    }

    /// Whether the setup of Make a new wallet or Restore number `owner`
    /// has a vault open: its question and its count are then fixed until
    /// a lock (`docs/BACKUP-FLOW.md` §10).
    pub(crate) fn setup_vault_open(&self, owner: u64) -> bool {
        let Some(i) = self.wallet_flow_pre_of(owner) else {
            return false;
        };
        (0..self.backup_flows[i].plan.vaults.len()).any(|v| self.flow_open_index(i, v).is_some())
    }

    /// Whether Make a new wallet or Restore number `owner`, with vaults
    /// answered yes, has its setup still to do: its keys stay closed.
    pub(crate) fn setup_pending(&self, owner: u64, into_vaults: bool, made: bool) -> bool {
        into_vaults
            && !made
            && !self
                .wallet_flow_of_owner(owner)
                .is_some_and(|i| self.backup_flows[i].step > Step::Vaults)
    }
}
