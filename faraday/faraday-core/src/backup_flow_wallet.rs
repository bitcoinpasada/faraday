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
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;

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
        let cs = self.session.wallets.get(b.wallet)?.checksum();
        self.wallet_flow_at(cs)
    }

    /// The writes wallet `checksum` waits for: how many sticks are written
    /// and how many there are. `None` once every one is.
    pub fn wallet_backup_waits(&self, checksum: &str) -> Option<(usize, usize)> {
        let i = self.wallet_flow_at(checksum)?;
        let f = &self.backup_flows[i];
        f.waiting().then(|| f.written_of())
    }

    /// Whether wallet `w`'s backup was left part way: its vaults set up
    /// before its keys, its plan not made yet. Back up resumes it at the
    /// plan, and the wallet's card and Backups offer it until it is done
    /// (`docs/history/NEW-WALLET.md` §12.3).
    pub fn wallet_backup_unfinished(&self, w: usize) -> bool {
        self.session
            .wallets
            .get(w)
            .and_then(|wl| self.wallet_flow_active(wl.checksum()))
            .is_some_and(|i| !self.backup_flows[i].made)
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
        let c = self.compute();
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
                    self.backup_flows[i].resize_setup(&c, count);
                }
            }
        }
    }

    /// Make a new wallet or Restore number `owner` has its count of
    /// vaults: the setup opens on its first card, or the one under way
    /// resumes. Nothing about places, paper or which key goes where is
    /// asked: that is Back up's plan, after the keys.
    pub(crate) fn wallet_flow_begin(&mut self, owner: u64) {
        let c = self.compute();
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
        // The flow's plan takes changes from the start: its vaults were
        // set up before the keys, which no preset places
        // (`docs/BACKUP-FLOW.md` §19.6).
        let mut answers = plan::Answers::defaults(&with_vaults);
        answers.custom = true;
        let mut f = BackupFlow::new_setup(&c, shape, answers, count);
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
        let c = self.compute();
        let Some(i) = self.wallet_flow_pre_of(owner) else {
            return;
        };
        let Some(wl) = self.session.wallets.get(w) else {
            return;
        };
        let (cs, name) = (wl.checksum().to_string(), wl.name.clone());
        let setup = self.backup_flows[i]
            .wallet
            .as_ref()
            .map_or(0, |wf| wf.shape.vaults);
        let mut shape = self.plan_shape_now(w);
        shape.vaults = setup;
        // The keys-to-be were numbered by slot, the seeds are numbered as
        // the description lists them (FROST's by share): where a slot's key
        // turned out to be.
        let slots = self.wallet_owner_slots(owner);
        let list = self.backup_seed_list_now(w);
        let map = seed_map(&slots, &list, &self.session, shape.seeds.len());
        let base = self.wallet_seed_bases(&c, w);
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
        f.wallet_replan(&c);
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

    /// Costs what `parse_words` and `key_fingerprint` cost. Each seed of
    /// wallet `w` as text, without its passphrase: the fingerprint of the
    /// same words alone.
    pub(crate) fn wallet_seed_bases(&self, c: &Compute, w: usize) -> Vec<String> {
        use faraday_vault::Record;
        use faraday_vault::records::{self, field, kind as rkind};
        self.backup_seed_list_now(w)
            .into_iter()
            .map(|(name, k)| {
                let Some(key) = k.and_then(|k| self.session.keys.get(k)) else {
                    return name;
                };
                let Some(words) = key.words.as_ref() else {
                    return name;
                };
                let Ok(m) = crate::wallet::parse_words(c, key.language, words) else {
                    return name;
                };
                let rec = Record::new(rkind::KEY).with(field::KEY, &records::words_payload(&m));
                crate::vault_screens::key_fingerprint(c, self, &rec).unwrap_or(name)
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
        let c = self.compute();
        let Some(i) = self.wallet_flow_of_backup() else {
            return;
        };
        if self.backup_flows[i].made {
            return;
        }
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let shape = self.plan_shape_now(b.wallet);
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
        f.wallet_replan(&c);
    }

    // ---- the checklist's Write to sticks ----

    /// Costs what `backup_public_names_for_now` and `plan_mapped` cost. The
    /// names of the files the checklist made for wallet `w`, apart: the
    /// public files and sheets, and the seed files (secrets let out
    /// unprotected). Only those in the Outbox.
    pub(crate) fn wallet_out_files(&self, c: &Compute, w: usize) -> (Vec<String>, Vec<String>) {
        let Some(b) = self.backup.as_ref().filter(|b| b.wallet == w) else {
            return (Vec::new(), Vec::new());
        };
        let Some(wl) = self.session.wallets.get(w) else {
            return (Vec::new(), Vec::new());
        };
        let stem = file_stem(&wl.name);
        let in_outbox = |n: &str| self.outbox.iter().any(|i| i.name == n);
        let mut public: Vec<String> = self.backup_public_names_for_now(c, w, &b.answers);
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
        let shape = self.plan_shape_now(w);
        let kept = self.plan_mapped(c, &shape, &b.answers).seeds_as_files();
        let fps: Vec<String> = self
            .backup_seed_list_now(w)
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

    /// Costs what `wallet_out_files` and `plan_mapped` cost. The files each
    /// stick of `flow` will hold for wallet `w`, by slot: the public files
    /// and sheets to the stick that carries them; the seed files the plan
    /// keeps on the stick of files to it; and to each stick at a place the
    /// files the plan keeps on it, beside its vault when it carries one
    /// (`docs/BACKUP-FLOW.md` §19.4), with no warning where they meet.
    fn wallet_slot_files(&self, c: &Compute, w: usize, flow: &BackupFlow) -> Vec<Vec<String>> {
        let mut out: Vec<Vec<String>> = vec![Vec::new(); flow.slots.len()];
        let (public, seeds) = self.wallet_out_files(c, w);
        let Some(b) = self.backup.as_ref().filter(|b| b.wallet == w) else {
            return out;
        };
        let shape = self.plan_shape_now(w);
        let mapped = self.plan_mapped(c, &shape, &b.answers);
        // The public files go to an online computer only where the plan
        // needs them there; files kept at a place go on its own stick.
        let public_k = flow
            .public_slot()
            .filter(|_| plan::public_stick_on(&b.answers, &mapped));
        if let Some(k) = public_k {
            out[k] = public;
        }
        let boxes = &mapped.map;
        for (k, slot) in flow.slots.iter().enumerate() {
            let mut names: Vec<String> = match slot.target {
                // A seed file never goes to the stick that carries the
                // public files.
                Target::Files if Some(k) != public_k => seeds.clone(),
                Target::Files | Target::Public => Vec::new(),
                t => target_sticks(boxes, t)
                    .into_iter()
                    .flat_map(|at| plan::holds_at(boxes, at))
                    .filter(|what| plan::is_file(*what))
                    .flat_map(|what| self.stick_file_names_now(w, &shape, &b.answers, what))
                    .collect(),
            };
            for n in names.drain(..) {
                if !out[k].contains(&n) {
                    out[k].push(n);
                }
            }
        }
        out
    }

    /// Splits a long text into its QR parts and makes a share's text to
    /// name them: the names file `what` of wallet `w` is written as on a
    /// stick under answers `a` (`docs/BACKUP-FLOW.md` §19.12 item 6), a
    /// seed's `seed-{fingerprint}.txt`, `seedqr-{fingerprint}.png`,
    /// `passphrase-{fingerprint}.txt`; the description's
    /// `{wallet}-descriptor.txt` or its pictures; a share's
    /// `{wallet}-share-{j}-of-{n}.txt` or its pictures; the software's own
    /// files. Kept for the frame while one is drawn.
    ///
    pub(crate) fn stick_file_names_now(
        &self,
        w: usize,
        shape: &plan::Shape,
        a: &plan::Answers,
        what: plan::What,
    ) -> Vec<String> {
        use plan::What as P;
        let Some(wl) = self.session.wallets.get(w) else {
            return Vec::new();
        };
        let stem = file_stem(&wl.name);
        match what {
            P::SeedFile(i) | P::SeedQrFile(i) | P::PassphraseFile(i) => shape
                .seeds
                .get(i)
                .filter(|x| x.here)
                .and_then(|x| seed_stick_name(what, &x.name))
                .into_iter()
                .collect(),
            P::WalletFile(f) if f == plan::form::TEXT => vec![format!("{stem}-descriptor.txt")],
            P::WalletFile(_) => crate::picture_names(
                &format!("{stem}-descriptor"),
                "-",
                &wl.policy().to_descriptor_checksummed(),
            ),
            P::ShareFile(j, f) => {
                let (m, n) = crate::wallet::Session::quorum(wl);
                let head = format!("{stem}-share-{}-of-{n}", j + 1);
                if f == plan::form::TEXT {
                    return vec![format!("{head}.txt")];
                }
                let plan = crate::backup::split_plan(n, m, a.omit);
                plan.get(j)
                    .and_then(|row| crate::backup::split_share(wl, row))
                    .map(|text| crate::picture_names(&head, "-part-", &text))
                    .unwrap_or_default()
            }
            P::SoftwareFile => software_files(&stem, &Self::backup_public_for(wl, a)),
            _ => Vec::new(),
        }
    }

    /// Makes a SeedQR, a QR picture or a share's file for each file the
    /// plan keeps on a stick at a place, For the stick, while the keys are
    /// loaded: at **Lock and write**, since a stick's files are written
    /// with it and are no checklist item of their own
    /// (`docs/BACKUP-FLOW.md` §19.12 item 6). A seed's files go out as
    /// secrets let out unprotected. Returns, for each picture made, its
    /// name and the SHA-256 of what its codes must read as, which the
    /// write step reads it back against (§19.12 item 10): a SeedQR by its
    /// key's fingerprint only, never anything of the seed.
    pub(crate) fn wallet_stick_files_make(
        &mut self,
        w: usize,
    ) -> Vec<(String, crate::backup_flow::ReadBack)> {
        let c = self.compute();
        use crate::backup_flow::ReadBack;
        use plan::What as P;
        let Some(b) = self.backup.as_ref().filter(|b| b.wallet == w) else {
            return Vec::new();
        };
        let shape = self.plan_shape_now(w);
        let answers = b.answers.clone();
        let boxes = plan::map(&c, &shape, &answers);
        let mut expect: Vec<(String, ReadBack)> = Vec::new();
        let mut whats: Vec<P> = Vec::new();
        for s in boxes.iter().filter(|s| matches!(s.at, plan::At::Stick(..))) {
            for (what, _) in &s.holds {
                if plan::is_file(*what) && !whats.contains(what) {
                    whats.push(*what);
                }
            }
        }
        let list = self.backup_seed_list_now(w);
        for what in whats {
            let names = self.stick_file_names_now(w, &shape, &answers, what);
            match what {
                P::SeedFile(i) | P::SeedQrFile(i) | P::PassphraseFile(i) => {
                    let Some(k) = list.get(i).and_then(|(_, k)| *k) else {
                        continue;
                    };
                    if let (P::SeedQrFile(_), Some(name)) = (what, names.first()) {
                        let fp = crate::wallet::fp_text(self.session.keys[k].fingerprint());
                        expect.push((name.clone(), ReadBack::Seed(fp)));
                    }
                    if let (Some(name), Some(bytes)) =
                        (names.first(), self.seed_stick_bytes(&c, k, what))
                    {
                        self.outbox.retain(|it| it.name != *name);
                        let mut item = crate::Item::new(&c, name, bytes.to_vec());
                        item.secret = true;
                        self.outbox.push(item);
                    }
                }
                P::WalletFile(f) => {
                    if f == plan::form::QR
                        && let Some(wl) = self.session.wallets.get(w)
                    {
                        let text = wl.policy().to_descriptor_checksummed();
                        expect.extend(pictures_expect(&c, &names, &text));
                    }
                    let made = self.public_made(w, if f == plan::form::TEXT { 1 } else { 8 });
                    self.put_made(made, &names);
                }
                P::ShareFile(j, f) => {
                    if f == plan::form::QR
                        && let Some(wl) = self.session.wallets.get(w)
                    {
                        let (m, n) = crate::wallet::Session::quorum(wl);
                        let split = crate::backup::split_plan(n, m, answers.omit);
                        if let Some(text) = split
                            .get(j)
                            .and_then(|row| crate::backup::split_share(wl, row))
                        {
                            expect.extend(pictures_expect(&c, &names, &text));
                        }
                    }
                    let made = self.public_made(w, 4);
                    self.put_made(made, &names);
                }
                P::SoftwareFile => {
                    let Some(wl) = self.session.wallets.get(w) else {
                        continue;
                    };
                    for x in Self::backup_public_for(wl, &answers) {
                        let made = self.public_made(w, x);
                        self.put_made(made, &names);
                    }
                }
                _ => {}
            }
        }
        self.save_boxes();
        expect
    }

    /// Costs what `parse_words` costs. Writes out the digits session key
    /// `k`'s SeedQR holds, to say whether `payload` is them: what its
    /// picture must read as (English words only, as the picture is made).
    /// `None` where the words are not here.
    pub(crate) fn seedqr_reads_as(&self, c: &Compute, k: usize, payload: &[u8]) -> Option<bool> {
        use faraday_bip::bip39::Language;
        let key = self.session.keys.get(k)?;
        if key.language != Language::English {
            return None;
        }
        let m = crate::wallet::parse_words(c, key.language, key.words.as_deref()?).ok()?;
        let digits = faraday_codec::seedqr::to_digits(&m);
        Some(digits.expose().as_bytes() == payload)
    }

    /// The files of `made` named in `names`, For the stick.
    fn put_made(&mut self, made: Vec<(String, Vec<u8>)>, names: &[String]) {
        for (name, bytes) in made {
            if names.contains(&name) {
                self.put_outbox(&name, bytes);
            }
        }
    }

    /// Costs what `parse_words` and `seedqr_png` cost. Makes a SeedQR and
    /// its picture for the picture: the bytes of seed form `what` of
    /// session key `k`, for its file on a stick; the words, one line; the
    /// SeedQR as a labelled picture (English words only); the passphrase,
    /// one line.
    fn seed_stick_bytes(
        &self,
        c: &Compute,
        k: usize,
        what: plan::What,
    ) -> Option<zeroize::Zeroizing<Vec<u8>>> {
        use faraday_bip::bip39::Language;
        let key = self.session.keys.get(k)?;
        let line = |t: &str| {
            let mut v = Vec::with_capacity(t.len() + 1);
            v.extend_from_slice(t.as_bytes());
            v.push(b'\n');
            zeroize::Zeroizing::new(v)
        };
        match what {
            plan::What::SeedFile(_) => Some(line(key.words.as_deref()?)),
            plan::What::PassphraseFile(_) => Some(line(key.passphrase.as_deref()?)),
            plan::What::SeedQrFile(_) => {
                if key.language != Language::English {
                    return None;
                }
                let m = crate::wallet::parse_words(c, key.language, key.words.as_deref()?).ok()?;
                seedqr_png(c, &m, &fp_text(key.fingerprint()))
            }
            _ => None,
        }
    }

    /// Costs what `plan_mapped` costs. The first seed Back up's plan keeps
    /// as a file on a stick at a place whose warning the person has not
    /// been through ([`crate::plan::Answers::seeds_on_stick_files`]): the
    /// gate **Lock and write** passes only once there is none. Computes
    /// from the state as it is.
    pub fn backup_stick_seed_unwarned(&self, c: &Compute) -> Option<usize> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape_now(b.wallet);
        self.plan_mapped(c, &shape, &b.answers)
            .seeds_on_stick_files(&shape)
            .into_iter()
            .find(|&i| {
                shape.seeds.get(i).is_some_and(|x| {
                    !b.files_warned
                        .iter()
                        .any(|f| f.eq_ignore_ascii_case(&x.name))
                })
            })
    }

    /// The secret sheet's warning for seed `i`'s files on a stick, before
    /// they are made: what the seed gives whoever finds the stick, and the
    /// line to tick. Past it the seed is let out and the lock goes on.
    fn offer_stick_seed(&mut self, i: usize) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let w = b.wallet;
        let shape = self.plan_shape_now(w);
        let Some(seed) = shape.seeds.get(i) else {
            return;
        };
        let fp = seed.name.clone();
        let names: Vec<String> = plan::map(&c, &shape, &b.answers)
            .iter()
            .filter(|s| matches!(s.at, plan::At::Stick(..)))
            .flat_map(|s| s.holds.iter().map(|(x, _)| *x))
            .filter(|x| {
                matches!(*x, plan::What::SeedFile(j) | plan::What::SeedQrFile(j)
                    | plan::What::PassphraseFile(j) if j == i)
            })
            .filter_map(|x| seed_stick_name(x, &fp))
            .fold(Vec::new(), |mut v, n| {
                if !v.contains(&n) {
                    v.push(n);
                }
                v
            });
        let need = self
            .session
            .wallets
            .get(w)
            .map_or(1, |wl| crate::wallet::Session::quorum(wl).0);
        let gives = if need <= 1 {
            EN.secrets_whoever_has_it.to_string()
        } else {
            let more = need - 1;
            fill(
                EN.secrets_whoever_has,
                &[
                    &fp,
                    &more.to_string(),
                    if more == 1 {
                        EN.secrets_key
                    } else {
                        EN.secrets_keys
                    },
                ],
            )
        };
        let mut out = crate::secrets::SecretOut::new(
            names.join(", "),
            zeroize::Zeroizing::new(Vec::new()),
            EN.secrets_a_wallet_s,
            &gives,
        );
        out.keep = crate::secrets::Keep::None;
        out.ack = crate::secrets::Ack::Seed;
        out.to_lock = Some(fp);
        self.offer_secret(out);
    }

    /// Gives each stick of flow `i` the files it will hold
    /// ([`Faraday::wallet_slot_files`]).
    fn wallet_assign_files(&mut self, i: usize, w: usize) {
        let c = self.compute();
        let files = self.wallet_slot_files(&c, w, &self.backup_flows[i]);
        for (s, f) in self.backup_flows[i].slots.iter_mut().zip(files) {
            s.files = f;
        }
    }

    /// **Lock and write**: the checklist's last item. Every other item is
    /// done; the plan is saved, the flow takes the vaults and the files,
    /// the hold on the keys moves to its state, and the lock comes. The
    /// stick visit page is never shown.
    pub(crate) fn wallet_lock_write(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        if b.stage != BStage::Checklist {
            return;
        }
        let w = b.wallet;
        let items = self.backup_items_now(&c);
        if !items.contains(&bstep::STICKS) {
            return;
        }
        let others_done = items
            .iter()
            .filter(|&&n| n != bstep::STICKS)
            .all(|&n| bstep::item(n).is_some_and(|it| self.backup_item_done_now(&c, it)));
        if !others_done {
            return;
        }
        let Some(wl) = self.session.wallets.get(w) else {
            return;
        };
        let (checksum, name) = (wl.checksum().to_string(), wl.name.clone());
        // A backup past its lock and not done keeps its plan: its writes
        // are finished first, then Back up runs again (§17.8).
        if self.backup_locked_flow().is_some() {
            return self.toast(EN.backup_flow_past_lock);
        }
        let answers = b.answers.clone();
        let shape = self.plan_shape_now(w);
        // A seed's file on a stick is made only past the warning that it
        // is not protected, as the seed files item made one: nothing is
        // made or written until the person has said so for each seed
        // (`docs/BACKUP-FLOW.md` §19.12 item 6).
        if let Some(i) = self.backup_stick_seed_unwarned(&c) {
            return self.offer_stick_seed(i);
        }
        let used = answers.vaults_made(&shape);
        // Every vault of the plan is open, and holds what the plan puts
        // in it.
        let mut opens: Vec<(usize, usize)> = Vec::new();
        for &v in &used {
            let taken: Vec<usize> = opens.iter().map(|&(_, o)| o).collect();
            let Some(o) = self.wallet_vault_open(&c, &checksum, v, &taken) else {
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
                let mut nf = BackupFlow::new_wallet(&c, shape.clone(), answers.clone(), &name);
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
            f.subject = Some(Subject {
                id: checksum.clone(),
                name,
            });
            f.wallet_replan(&c);
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
            // What the vault held before this backup put anything in it
            // (all it holds, where no save was seen), and the keys other
            // wallets' records in it name: the round takes none of them
            // out.
            let held = self
                .backup
                .as_ref()
                .and_then(|b| b.before.iter().find(|(s, _)| *s == salt))
                .map_or_else(|| self.vault_tokens(&c, o), |(_, t)| t.clone());
            let others = self.vault_others(&c, o, &checksum);
            if let Some(wf) = self.backup_flows[i].wallet.as_mut() {
                if !wf.before.iter().any(|(s, _)| *s == salt) {
                    wf.before.push((salt, held));
                }
                wf.others.retain(|(s, _)| *s != salt);
                wf.others.push((salt, others));
            }
        }
        let base = self.wallet_seed_bases(&c, w);
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
        // A stick's files are made now, with the keys still loaded, and
        // written with their stick (`docs/BACKUP-FLOW.md` §19.12 item 6).
        let readback = self.wallet_stick_files_make(w);
        if let Some(wf) = self.backup_flows[i].wallet.as_mut() {
            wf.readback = readback;
        }
        self.wallet_assign_files(i, w);
        let f = &mut self.backup_flows[i];
        if let Some(wf) = f.wallet.as_mut() {
            wf.sealed = used.iter().map(|&v| (v, sealed_marks(&c, wf, v))).collect();
            // The description on paper or as files at the lock: its split
            // is fixed from now on.
            wf.pieces = plan::map(&c, &wf.shape, &wf.answers).iter().any(|s| {
                s.holds.iter().any(|(w, _)| match s.at {
                    plan::At::Place(_) | plan::At::Stick(..) => matches!(
                        w,
                        plan::What::Sheet
                            | plan::What::Share(_)
                            | plan::What::WalletFile(_)
                            | plan::What::ShareFile(..)
                            | plan::What::SoftwareFile
                    ),
                    plan::At::Files => *w == plan::What::Wallet,
                    _ => false,
                })
            });
        }
        f.made = true;
        f.made_in = used;
        f.adding = false;
        f.step = Step::Lock;
        f.sealing.clear();
        self.flow_show(i);
        self.flow_act(crate::backup_flow_steps::FlowAction::LockWrite);
    }

    /// Costs what `backup_open_fits` costs. The open vault that holds what
    /// the plan puts in plan vault `v`: the vault the flow knows, else the
    /// one that fits.
    fn wallet_vault_open(
        &self,
        c: &Compute,
        checksum: &str,
        v: usize,
        taken: &[usize],
    ) -> Option<usize> {
        // The vault the flow made or chose for `v` stays its vault while it
        // fits; the lock never takes another open vault in its place.
        if let Some(i) = self.wallet_flow_active(checksum) {
            if let Some(o) = self.flow_open_index(i, v) {
                return (self.backup_open_fits(c, v, o) && !taken.contains(&o)).then_some(o);
            }
            // A vault the flow knows by its salt, or a flow past its lock:
            // never another open vault in its place.
            let f = &self.backup_flows[i];
            if f.made || f.plan.vaults.get(v).is_some_and(|vp| vp.salt.is_some()) {
                return None;
            }
        }
        // Two planned vaults are never one open vault.
        (0..self.vaults.open.len()).find(|&o| !taken.contains(&o) && self.backup_open_fits(c, v, o))
    }

    /// F6's hold on the keys the plan keeps in a vault moves from the
    /// copies to the flow's state: the vaults sealed now say the backup is
    /// done, and the keys spend nothing until the last stick is written.
    fn wallet_hold_move(&mut self, i: usize) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let w = b.wallet;
        let shape = self.plan_shape_now(w);
        let list = self.backup_seed_list_now(w);
        let held = self.backup_held_seeds(&c);
        // A seed kept on any stick, a vault's or a file's, waits for it.
        let on_stick = |s: usize| b.answers.seed_on_stick(&c, &shape, s);
        let mut moved: Vec<(Fingerprint, usize)> = Vec::new();
        for s in held.into_iter().filter(|&s| on_stick(s)) {
            if let Some(k) = list.get(s).and_then(|(_, k)| *k)
                && let Some(key) = self.session.keys.get(k)
            {
                moved.push((key.fingerprint(), k));
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
                .map(|o| o.fingerprint())
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
        // Each stick counted for the seed holds it as written: a stick of
        // files the seed's file, a vault's stick a vault sealed with it.
        let spots = f.seed_slots(s);
        let sealed_with = |v: usize| {
            f.wallet.as_ref().is_some_and(|w| {
                w.sealed
                    .iter()
                    .any(|(x, marks)| *x == v && marks.contains(&s))
            })
        };
        !spots.is_empty()
            && spots.iter().all(|&k| {
                let Some(wr) = f.slots[k].written.as_ref() else {
                    return false;
                };
                // A stick may carry a vault and files beside it
                // (`docs/BACKUP-FLOW.md` §19.4): either holds the seed.
                wr.files.iter().any(|(n, _)| f.is_seed_file(s, n))
                    || f.vaults_of(k).iter().any(|&v| sealed_with(v))
            })
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
        self.session.wallets.iter().position(|w| w.checksum() == cs)
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
            .map(|w| self.backup_seed_list_now(w))
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

    /// Costs what `read_wallet` and `add_wallet` cost. Share `j` of the
    /// wallet with checksum `cs` as a type 13 record (`docs/BACKUP-FLOW.md`
    /// §17.6), made from its description: the wallet loaded, or its
    /// description in an open vault or Files.
    fn wallet_share_record(
        &self,
        c: &Compute,
        wf: &crate::backup_flow::WalletFlow,
        cs: &str,
        j: usize,
    ) -> Option<faraday_vault::Record> {
        use faraday_vault::records::{field, kind as rkind};
        let of_wallet = |t: &str| {
            crate::wallet::read_wallet(c, t)
                .ok()
                .is_some_and(|p| p.checksum() == cs)
        };
        let text = |wl: &crate::wallet::Wallet| -> Option<(String, usize, String)> {
            let (m, n) = crate::wallet::Session::quorum(wl);
            let plan = crate::backup::split_plan(n, m, wf.answers.omit);
            Some((
                crate::backup::split_share(wl, plan.get(j)?)?,
                plan.len(),
                Faraday::wallet_text(wl),
            ))
        };
        let made = match self.session.wallets.iter().find(|wl| wl.checksum() == cs) {
            Some(wl) => text(wl),
            None => {
                // A wallet record, or a plan record's wallet (its field
                // 1), in any open vault; or the description file.
                let desc = self
                    .vaults
                    .open
                    .iter()
                    .flat_map(|o| {
                        o.contents
                            .records
                            .iter()
                            .filter_map(|r| match r.kind {
                                rkind::WALLET => r.text(field::WALLET),
                                rkind::PLAN => r.text(field::PLAN_WALLET),
                                _ => None,
                            })
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .chain(
                        self.inbox
                            .iter()
                            .chain(self.outbox.iter())
                            .filter(|it| it.kind == crate::wallet::FileKind::Wallet)
                            .filter_map(|it| String::from_utf8(it.bytes.clone()).ok()),
                    )
                    .find(|t| of_wallet(t))?;
                let mut s = crate::wallet::Session::on(self.session.network());
                let k = s
                    .add_wallet(c, &wf.name, desc.trim(), "", crate::wallet::Origin::Named)
                    .ok()?;
                text(&s.wallets[k])
            }
        };
        let (share, of, desc) = made?;
        Some(
            faraday_vault::Record::new(rkind::WALLET_SHARE)
                .with(field::SHARE, share.as_bytes())
                .with(field::SHARE_WALLET, desc.as_bytes())
                .with(field::SHARE_WALLET_NAME, wf.name.as_bytes())
                .with(
                    field::SHARE_NUMBER,
                    &[u8::try_from(j + 1).unwrap_or(u8::MAX)],
                )
                .with(field::SHARE_OF, &[u8::try_from(of).unwrap_or(u8::MAX)]),
        )
    }

    /// The copy of the keys a vault added after the keys were made takes
    /// from one that holds them (§4.8): each seed the plan puts in it, and
    /// the wallet description where the plan puts it.
    pub(crate) fn flow_wallet_copy(&mut self, i: usize) {
        let c = self.compute();
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
            crate::wallet::read_wallet(&c, t)
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
            // What the vault held before the flow first put anything in it,
            // and the keys other wallets' records in it name: kept.
            let salt = self.vaults.open[o].header().salt;
            let first = self.vault_tokens(&c, o);
            let others = self.vault_others(&c, o, &cs);
            if let Some(f) = self.backup_flows[i].wallet.as_mut() {
                if !f.before.iter().any(|(s, _)| *s == salt) {
                    f.before.push((salt, first));
                }
                f.others.retain(|(s, _)| *s != salt);
                f.others.push((salt, others.clone()));
            }
            let before: Vec<String> = self.backup_flows[i]
                .wallet
                .as_ref()
                .and_then(|f| f.before.iter().find(|(s, _)| *s == salt))
                .map(|(_, t)| t.clone())
                .unwrap_or_default();
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
                    let fp = key_fingerprint(&c, app, r);
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
                // This flow's own record of the key not as the plan wants
                // it goes; a record the vault held before stays.
                w.out.extend(
                    there
                        .iter()
                        .filter(|(_, r)| {
                            has_pass(r) != want_pass
                                && !self.record_tokens(&c, r).iter().any(|t| before.contains(t))
                        })
                        .map(|(k, _)| *k),
                );
                // A record as the plan wants it, among all of this key's:
                // added once.
                if there.iter().any(|(_, r)| has_pass(r) == want_pass) {
                    continue;
                }
                // The record as the plan wants it: from a holder, or this
                // vault's own (the passphrase taken off where the plan
                // keeps it apart), or from the key typed back.
                let from_holder = holders.iter().chain(std::iter::once(&o)).find_map(|&h| {
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
                            k.words.is_some() && fp_text(k.fingerprint()).eq_ignore_ascii_case(want)
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
            // Each share the plan keeps in the vault (§17.6), as a type 13
            // record: from a holder that keeps it, else made from the
            // wallet's description, loaded or kept in a vault or Files.
            // What the plan no longer keeps here of this wallet goes (§10.2):
            // a seed it does not put in this vault, the description where it
            // keeps it elsewhere. Another wallet's records are never
            // touched.
            let wanted = wf.answers.vault_seeds(&wf.shape, v);
            for (k, r) in self.vaults.open[o].contents.records.iter().enumerate() {
                let gone = match r.kind {
                    rkind::KEY => {
                        let fp = key_fingerprint(&c, self, r);
                        (0..wf.shape.seeds.len()).any(|s| {
                            let name = &wf.shape.seeds[s].name;
                            let base = wf.base.get(s).unwrap_or(name);
                            !wanted.contains(&s)
                                && (fp.as_deref() == Some(name.as_str())
                                    || fp.as_deref() == Some(base.as_str()))
                        })
                    }
                    rkind::WALLET => {
                        !wf.answers.wallet_in_vault(v)
                            && r.text(field::WALLET).is_some_and(&of_wallet)
                    }
                    _ => false,
                };
                if gone {
                    w.out.push(k);
                }
            }
            // Each share the plan keeps in the vault (§17.6), as a type 13
            // record of this wallet (told by its field 5, never by its
            // name), with the text the plan's split gives now; any other
            // share of this wallet goes.
            let kept = wf.answers.vault_shares(&c, &wf.shape, v);
            for (k, r) in self.vaults.open[o].contents.records.iter().enumerate() {
                if crate::share_record_is(&c, r, &cs, None)
                    && !kept
                        .iter()
                        .any(|&j| crate::share_record_is(&c, r, &cs, Some(j)))
                {
                    w.out.push(k);
                }
            }
            for j in kept {
                let want = self.wallet_share_record(&c, &wf, &cs, j).or_else(|| {
                    holders.iter().find_map(|&h| {
                        self.vaults.open[h]
                            .contents
                            .records
                            .iter()
                            .find(|r| crate::share_record_is(&c, r, &cs, Some(j)))
                            .cloned()
                    })
                });
                let Some(want) = want else {
                    return self.toast(EN.backup_flow_no_description);
                };
                let text = want.text(field::SHARE).map(str::to_string);
                let mut have = false;
                for (k, r) in self.vaults.open[o].contents.records.iter().enumerate() {
                    if crate::share_record_is(&c, r, &cs, Some(j)) {
                        if !have && r.text(field::SHARE).map(str::to_string) == text {
                            have = true;
                        } else {
                            w.out.push(k);
                        }
                    }
                }
                if !have
                    && !w
                        .add
                        .iter()
                        .any(|r| crate::share_record_is(&c, r, &cs, Some(j)))
                {
                    w.add.push(want);
                }
            }
            // Each passphrase the plan keeps alone in the vault, a type 14
            // record (`docs/BACKUP-FLOW.md` §19.12 item 4): from an open
            // vault's record of it, or its key's record that stores it;
            // one of this wallet's seeds' the plan no longer keeps here
            // goes.
            let alone = wf.answers.vault_passphrases(&c, &wf.shape, v);
            let fp_of = |s: usize| -> Option<[u8; 4]> {
                let name = &wf.shape.seeds.get(s)?.name;
                let mut out = [0u8; 4];
                for (k, b) in out.iter_mut().enumerate() {
                    *b = u8::from_str_radix(name.get(2 * k..2 * k + 2)?, 16).ok()?;
                }
                Some(out)
            };
            for (k, r) in self.vaults.open[o].contents.records.iter().enumerate() {
                if r.kind == rkind::PASSPHRASE
                    && (0..wf.shape.seeds.len()).any(|s| {
                        !alone.contains(&s)
                            && fp_of(s)
                                .is_some_and(|f| r.field(field::PASSPHRASE_KEY) == Some(&f[..]))
                    })
                {
                    w.out.push(k);
                }
            }
            for s in alone {
                let Some(f) = fp_of(s) else {
                    continue;
                };
                let name = wf.shape.seeds[s].name.clone();
                let here = |r: &Record| {
                    r.kind == rkind::PASSPHRASE && r.field(field::PASSPHRASE_KEY) == Some(&f[..])
                };
                if self.vaults.open[o].contents.records.iter().any(here) || w.add.iter().any(here) {
                    continue;
                }
                let found = self.vaults.open.iter().find_map(|open| {
                    open.contents.records.iter().find_map(|r| {
                        if here(r) {
                            return Some(r.clone());
                        }
                        let p = r.text(field::KEY_PASSPHRASE)?;
                        (r.kind == rkind::KEY
                            && key_fingerprint(&c, self, r).as_deref() == Some(name.as_str()))
                        .then(|| {
                            Record::new(rkind::PASSPHRASE)
                                .with(field::PASSPHRASE_KEY, &f)
                                .with(field::PASSPHRASE, p.as_bytes())
                        })
                    })
                });
                match found {
                    Some(r) => w.add.push(r),
                    None => return self.toast(EN.backup_flow_unlock_first),
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
            // Only what this flow put into the vault is taken out: never a
            // record or a field it held before, never a key another
            // wallet's record names.
            w.out.retain(|&k| {
                let Some(r) = self.vaults.open[o].contents.records.get(k) else {
                    return false;
                };
                let tokens = self.record_tokens(&c, r);
                let key = r.kind == rkind::KEY;
                !(tokens.iter().any(|t| before.contains(t)) || (key && !others.is_empty()))
            });
            let open = &self.vaults.open[o];
            let mut trial = open.contents.clone();
            let mut gone = w.out.clone();
            gone.sort_unstable();
            gone.dedup();
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
                let marks = sealed_marks(&c, wf, v);
                wf.sealed.retain(|(x, _)| *x != v);
                wf.sealed.push((v, marks));
            }
        }
        f.made_in.extend(added);
        f.adding = false;
    }

    // ---- what the checklist and the visit say of the writes ----

    /// Costs what `new_wallet_on`, `plan_mapped`, `wallet_sticks_done` and
    /// the rest it calls cost. The sticks the wallet being backed up will
    /// be written to, by place, each with the files it will hold and
    /// whether it is written: from the flow, or from the plan's own while
    /// no flow has locked yet. Works out each stick's files; kept for the
    /// frame while one is drawn (`docs/SPEED.md` §3.12).
    ///
    /// Works out each stick's files: what [`crate::drawing::View::wallet_stick_rows`]
    /// answers.
    pub fn wallet_stick_rows_now(&self, c: &Compute) -> Vec<crate::drawing::StickRow> {
        let Some(b) = self.backup.as_ref() else {
            return Vec::new();
        };
        let Some(wl) = self.session.wallets.get(b.wallet) else {
            return Vec::new();
        };
        let m = self.medium.noun();
        let shape = self.plan_shape_now(b.wallet);
        let own = || {
            BackupFlow::new_wallet_on(
                c,
                shape.clone(),
                b.answers.clone(),
                &wl.name,
                &self.plan_mapped(c, &shape, &b.answers),
            )
        };
        let mut flow = match self.wallet_flow_of_backup() {
            Some(i) if self.backup_flows[i].step != Step::Done || self.wallet_sticks_done(c) => {
                self.backup_flows[i].clone()
            }
            _ => own(),
        };
        if !flow.made {
            if let Some(w) = flow.wallet.as_ref() {
                let mapped = self.plan_mapped(c, &w.shape, &w.answers);
                flow.wallet_replan_on(c, &mapped);
            }
            let files = self.wallet_slot_files(c, b.wallet, &flow);
            for (s, f) in flow.slots.iter_mut().zip(files) {
                s.files = f;
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

    /// Costs what `plan_mapped` costs. The `PublicOn` the wallet's plan
    /// carries its public files on, for the card that names the stick.
    pub fn wallet_public_on(&self, c: &Compute) -> Option<PublicOn> {
        let b = self.backup.as_ref()?;
        let shape = self.plan_shape_now(b.wallet);
        Some(
            crate::backup_flow::plan_of_on(&b.answers, &self.plan_mapped(c, &shape, &b.answers))
                .public_on,
        )
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

    /// A wallet's answer to a question of the plan, as Back up ticks it. A
    /// stick already written is not taken off the plan from here.
    pub(crate) fn flow_wallet_answer(&mut self, i: usize, list: u8, row: u8) {
        // The plan's cards ask the rest place by place (§17); the software
        // and its form are the Software card's.
        if !matches!(list, crate::qrow::SOFTWARE | crate::qrow::FORM) {
            return;
        }
        let Some(q) = crate::qrow::question(list) else {
            return;
        };
        self.flow_wallet_edit(i, &|a, shape| a.toggle(shape, q, usize::from(row)));
    }

    /// A press on the flow's plan cards, Back up's cards drawn in the flow
    /// (`docs/BACKUP-FLOW.md` §17.7): the card open, the next card, the
    /// software, or the keys left off each share. The board's own presses
    /// are `board.rs`'s (§19.12 item 1).
    pub(crate) fn flow_plan_press(&mut self, i: usize, press: crate::PlanPress) {
        let c = self.compute();
        use crate::PlanPress as P;
        let Some(wf) = self.backup_flows[i].wallet.as_ref() else {
            return;
        };
        let (shape, answers) = (wf.shape.clone(), wf.answers.clone());
        match press {
            P::Q(q) => {
                let live = &mut self.backup_flows[i].live;
                live.plan_q = if live.plan_q == Some(q) {
                    None
                } else {
                    Some(q)
                };
            }
            P::Next(q) => {
                let cards = crate::plan_cards(&c, &shape, &answers);
                self.backup_flows[i].live.plan_q =
                    cards.iter().skip_while(|&&c| c != q).nth(1).copied();
            }
            P::Answer(list, row) => self.flow_wallet_answer(i, list, row),
            // The shares on paper and in files were made with the keys
            // left off as they are: after the lock that stays.
            P::Omit(n) => self.flow_wallet_edit(i, &|a, shape| {
                let most = shape.m.saturating_sub(1).min(shape.keys.saturating_sub(1));
                a.omit = usize::from(n).min(most);
            }),
        }
    }

    /// Costs what `record_described` and `key_fingerprint` cost. What a
    /// vault record is, as the flow keeps what a vault held before it:
    /// `k{fp}` a key's record of its words alone, `p{fp}` one with its
    /// passphrase, `a{fp}` a key's passphrase kept alone (type 14),
    /// `w{sum}` a wallet's description, `s{sum}:{n}` a wallet's share
    /// (`docs/BACKUP-FLOW.md` §17). Never a secret.
    pub(crate) fn record_tokens(&self, c: &Compute, r: &faraday_vault::Record) -> Vec<String> {
        use faraday_vault::records::{field, kind as rkind};
        // As an open vault keeps it read; a copy is read now.
        let sum = || self.record_described(c, r).map(|d| d.checksum.clone());
        match r.kind {
            rkind::KEY => {
                let Some(fp) = crate::vault_screens::key_fingerprint(c, self, r) else {
                    return Vec::new();
                };
                // A record of the words alone, and one with a passphrase,
                // are two records: told apart.
                if r.field(field::KEY_PASSPHRASE).is_some() {
                    vec![format!("p{fp}")]
                } else {
                    vec![format!("k{fp}")]
                }
            }
            rkind::WALLET => sum().map(|s| vec![format!("w{s}")]).unwrap_or_default(),
            rkind::PASSPHRASE => r
                .field(field::PASSPHRASE_KEY)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
                .map(|b| vec![format!("a{}", fp_text(faraday_bip::keys::Fingerprint(b)))])
                .unwrap_or_default(),
            rkind::WALLET_SHARE => {
                let n = r
                    .field(field::SHARE_NUMBER)
                    .and_then(|b| b.first())
                    .copied();
                match (sum(), n) {
                    (Some(s), Some(n)) => vec![format!("s{s}:{n}")],
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Costs what `record_tokens` costs. Everything open vault `o` holds,
    /// as [`Faraday::record_tokens`].
    pub(crate) fn vault_tokens(&self, c: &Compute, o: usize) -> Vec<String> {
        let mut out: Vec<String> = self
            .vaults
            .open
            .get(o)
            .map(|v| {
                v.contents
                    .records
                    .iter()
                    .flat_map(|r| self.record_tokens(c, r))
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out.dedup();
        out
    }

    /// Costs what `described` costs. The keys, by fingerprint, that a
    /// record in open vault `o` of a wallet other than the one with
    /// checksum `cs` names: its description, or its plan.
    pub(crate) fn vault_others(&self, c: &Compute, o: usize, cs: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let Some(v) = self.vaults.open.get(o) else {
            return out;
        };
        // A wallet's, a plan's or a share's description, as the vault
        // keeps it read.
        for i in 0..v.contents.records.len() {
            let Some(d) = v.contents.described(c, i) else {
                continue;
            };
            let p = &d.policy;
            if d.checksum == cs {
                continue;
            }
            // Any record of another wallet: its keys may derive from a
            // record here in ways the flow cannot see.
            out.push(OTHER_WALLET.to_string());
            out.extend(p.keys().iter().filter_map(|k| k.fingerprint()).map(fp_text));
        }
        out.sort();
        out.dedup();
        out
    }

    /// Costs what `sealed_marks` costs. Why an edit after the lock would
    /// take out of a vault what this flow did not put there
    /// (`docs/BACKUP-FLOW.md` §17): a key, a passphrase, a description or a
    /// share the vault held before the flow, or a key another wallet's
    /// record in it names. The toast's line, or `None`.
    fn flow_takes_out_foreign(&self, c: &Compute, trial: &BackupFlow) -> Option<String> {
        let wf = trial.wallet.as_ref()?;
        let cs = wf.checksum.clone().unwrap_or_default();
        for (v, had) in &wf.sealed {
            let Some(salt) = trial.plan.vaults.get(*v).and_then(|vp| vp.salt) else {
                continue;
            };
            if !trial.plan.used_vaults().contains(v) {
                continue;
            }
            let now = sealed_marks(c, wf, *v);
            let before: Vec<String> = wf
                .before
                .iter()
                .find(|(s, _)| *s == salt)
                .map(|(_, t)| t.clone())
                .unwrap_or_default();
            let others: Vec<String> = wf
                .others
                .iter()
                .find(|(s, _)| *s == salt)
                .map(|(_, t)| t.clone())
                .unwrap_or_default();
            let fps = |s: usize| -> Vec<String> {
                let name = wf
                    .shape
                    .seeds
                    .get(s)
                    .map(|x| x.name.clone())
                    .unwrap_or_default();
                let base = wf.base.get(s).cloned().filter(|b| !b.is_empty());
                std::iter::once(name).chain(base).collect()
            };
            for &m in had.iter().filter(|m| !now.contains(m)) {
                let n = (v + 1).to_string();
                // A key record, or its passphrase field, never leaves a
                // vault that holds any record of another wallet.
                if (m < WALLET_MARK || (PASS_MARK..SHARE_MARK).contains(&m)) && !others.is_empty() {
                    return Some(fill(EN.backup_flow_other_wallet_key, &[&n]));
                }
                if m < WALLET_MARK {
                    for fp in fps(m) {
                        if others.iter().any(|x| x.eq_ignore_ascii_case(&fp)) {
                            return Some(fill(EN.backup_flow_other_wallet_key, &[&n]));
                        }
                        if before.iter().any(|t| {
                            t.eq_ignore_ascii_case(&format!("k{fp}"))
                                || t.eq_ignore_ascii_case(&format!("p{fp}"))
                        }) {
                            return Some(fill(EN.backup_flow_held_before, &[&n]));
                        }
                    }
                } else if (PASS_MARK..SHARE_MARK).contains(&m) {
                    for fp in fps(m - PASS_MARK) {
                        if before
                            .iter()
                            .any(|t| t.eq_ignore_ascii_case(&format!("p{fp}")))
                        {
                            return Some(fill(EN.backup_flow_held_before, &[&n]));
                        }
                    }
                } else if m == WALLET_MARK {
                    if before.contains(&format!("w{cs}")) {
                        return Some(fill(EN.backup_flow_held_before, &[&n]));
                    }
                } else if (SHARE_MARK..OMIT_MARK).contains(&m)
                    && before.contains(&format!("s{cs}:{}", m - SHARE_MARK + 1))
                {
                    return Some(fill(EN.backup_flow_held_before, &[&n]));
                }
            }
        }
        None
    }

    /// Costs a map of the plan (through `place_map` and `map`). Whether
    /// plan `new` changes, from `old`, anything §17.8 keeps as made after
    /// the lock: words or a SeedQR by hand, a passphrase on paper, a seed's
    /// file, the sheet or a share on paper, the description's or a share's
    /// files, the software or the form, a stick's files, or Whole / In
    /// shares and the keys left off while any of the description is on
    /// paper or in a file.
    fn flow_changes_files(&self, c: &Compute, old: &BackupFlow, new: &BackupFlow) -> bool {
        let (Some(a), Some(b)) = (old.wallet.as_ref(), new.wallet.as_ref()) else {
            return false;
        };
        let pieces = |wf: &crate::backup_flow::WalletFlow, p: usize| -> Vec<plan::What> {
            let mut v: Vec<plan::What> = plan::place_map(c, &wf.shape, &wf.answers)
                .into_iter()
                .filter(|s| s.at == plan::At::Place(p))
                .flat_map(|s| s.holds.into_iter().map(|(w, _)| w))
                .filter(|w| !matches!(w, plan::What::VaultStick(_)))
                .collect();
            v.sort_by_key(|w| plan::what_code(*w));
            v
        };
        let elsewhere = |wf: &crate::backup_flow::WalletFlow| -> Vec<(String, Vec<String>)> {
            plan::map(c, &wf.shape, &wf.answers)
                .into_iter()
                .filter(|s| matches!(s.at, plan::At::Files | plan::At::Software))
                .map(|s| {
                    let mut v: Vec<String> =
                        s.holds.iter().map(|(w, _)| plan::what_code(*w)).collect();
                    v.sort();
                    (plan::at_code(s.at), v)
                })
                .collect()
        };
        // Place by place, a place added or the last one taken away
        // comparing as empty.
        let n = a.answers.places.max(b.answers.places);
        if (0..n).any(|p| pieces(a, p) != pieces(b, p)) {
            return true;
        }
        if elsewhere(a) != elsewhere(b)
            || a.answers.software != b.answers.software
            || a.answers.form != b.answers.form
        {
            return true;
        }
        if a.pieces && (a.answers.split != b.answers.split || a.answers.omit != b.answers.omit) {
            return true;
        }
        // A stick that carries files never leaves the plan.
        old.slots
            .iter()
            .any(|s| !s.files.is_empty() && !new.slots.iter().any(|t| t.target == s.target))
    }

    /// The wallet flow of the wallet Back up is open on, past its lock and
    /// not done: Back up's plan edits go through it
    /// ([`Faraday::flow_wallet_edit`]).
    pub(crate) fn backup_locked_flow(&self) -> Option<usize> {
        let i = self.wallet_flow_of_backup()?;
        let f = &self.backup_flows[i];
        (f.made && f.step != Step::Done).then_some(i)
    }

    /// An edit of flow `i`'s wallet plan, tried first: a stick already
    /// written stays on the plan, and after the lock a key held back for
    /// its sticks stays kept on one; refused, the toast says why and the
    /// plan stays as it was. A plan changed after the lock that changes
    /// what a vault holds starts the round (§10.2).
    pub(crate) fn flow_wallet_edit(
        &mut self,
        i: usize,
        edit: &dyn Fn(&mut plan::Answers, &plan::Shape),
    ) {
        let c = self.compute();
        // The plan does not change under a write, by any way in.
        if self.backup_flows[i].live.writing.is_some() {
            return;
        }
        let mut trial = self.backup_flows[i].clone();
        {
            let Some(w) = trial.wallet.as_mut() else {
                return;
            };
            let shape = w.shape.clone();
            edit(&mut w.answers, &shape);
        }
        trial.wallet_replan(&c);
        // Files and paper stay as they were made: after the lock only what
        // goes into a vault and where its sticks are kept change, until a
        // backup is done and Back up runs again (§17.8), a vault open or
        // not.
        if trial.made && self.flow_changes_files(&c, &self.backup_flows[i], &trial) {
            return self.toast(EN.backup_flow_files_after_lock);
        }
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
                    || plan::kept_nowhere(&c, &wf.shape, &wf.answers, &held).is_some()
            });
            if bad {
                return self.toast(EN.backup_flow_held_stays);
            }
            // Nothing the vault held before the flow, or another wallet
            // in it uses, is taken out.
            if let Some(line) = self.flow_takes_out_foreign(&c, &trial) {
                return self.toast(&line);
            }
        }
        // The screen's state stays; the plan follows the answers.
        trial.live = self.backup_flows[i].live.clone();
        trial.live.reading = None;
        if trial.made {
            // What a vault must hold changed: it is as one added, now.
            let step = trial.step;
            trial.made_plan_changed(&c, false);
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
        self.flow_wallet_edit(i, &|a, shape| {
            if a.places < crate::qstep::MAX_PLACES {
                a.add_place(shape);
            }
        });
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
        let Some(kind) = crate::create::NewKind::of(wl.policy()) else {
            return;
        };
        let Some(fp) = wl.policy().keys().first().and_then(|k| k.fingerprint()) else {
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
                    .is_some_and(|key| key.fingerprint().0 == *fp)
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
/// The sealed mark of share `j`: `SHARE_MARK + j`.
const SHARE_MARK: usize = 3_000_000;
/// The sealed mark of the keys left off each share, where a vault keeps
/// a share: `OMIT_MARK + omit`.
const OMIT_MARK: usize = 4_000_000;
/// The sealed mark of seed `s`'s passphrase kept alone in the vault, a
/// passphrase record (`docs/BACKUP-FLOW.md` §19.12 item 4): this plus `s`.
const PASS_ALONE_MARK: usize = 5_000_000;
/// In a vault's list of keys other wallets name: the vault holds a record
/// of another wallet at all.
const OTHER_WALLET: &str = "*";

/// Costs what `vault_shares` and `vault_passphrases` cost. What vault
/// `v` of the plan holds: its seeds, the wallet description where the
/// plan puts it there, and each passphrase the plan puts in with its
/// seed. A plan changed after the lock that asks for more is a round.
pub(crate) fn sealed_marks(
    c: &Compute,
    wf: &crate::backup_flow::WalletFlow,
    v: usize,
) -> Vec<usize> {
    let seeds = wf.answers.vault_seeds(&wf.shape, v);
    let mut marks = seeds.clone();
    if wf.answers.wallet_in_vault(v) {
        marks.push(WALLET_MARK);
    }
    // Each share the vault keeps (`docs/BACKUP-FLOW.md` §17.6): one added
    // after the lock is a change of what the vault holds (§10.2).
    let shares = wf.answers.vault_shares(c, &wf.shape, v);
    for &j in &shares {
        marks.push(SHARE_MARK + j);
    }
    // The keys left off each share make each share's text: another
    // number of them is another share (§17.7).
    if !shares.is_empty() {
        marks.push(OMIT_MARK + wf.answers.omit);
    }
    for s in seeds {
        if wf.shape.seeds.get(s).is_some_and(|x| x.passphrase) && wf.answers.pass_in_vault(s) {
            marks.push(PASS_MARK + s);
        }
    }
    for s in wf.answers.vault_passphrases(c, &wf.shape, v) {
        marks.push(PASS_ALONE_MARK + s);
    }
    marks
}

/// Costs what `encode_seedqr` and `labelled_png` cost. Makes a SeedQR
/// and draws it as a labelled picture: seed `m`'s code, titled with its
/// fingerprint `fp`, for its `seedqr-{fp}.png` on a stick. `None` for
/// words of another list than English, which SeedQR does not number.
pub(crate) fn seedqr_png(
    c: &Compute,
    m: &faraday_bip::bip39::Mnemonic,
    fp: &str,
) -> Option<zeroize::Zeroizing<Vec<u8>>> {
    if m.language() != faraday_bip::bip39::Language::English {
        return None;
    }
    let mx = faraday_codec::seedqr::encode_seedqr(c, m).ok()?;
    let title = fill(EN.secrets_seed, &[fp]);
    let lines = [fill(
        EN.secrets_words_secret,
        &[&m.word_count().to_string()],
    )];
    Some(zeroize::Zeroizing::new(crate::picture::labelled_png(
        c, &mx, &title, &lines,
    )))
}

/// The name seed form `what` of the seed with fingerprint `fp` is written
/// as on a stick (`docs/BACKUP-FLOW.md` §19.12 item 6): `seed-{fp}.txt`,
/// `seedqr-{fp}.png`, `passphrase-{fp}.txt`, the fingerprint in lower case.
pub(crate) fn seed_stick_name(what: plan::What, fp: &str) -> Option<String> {
    let fp = fp.to_lowercase();
    match what {
        plan::What::SeedFile(_) => Some(format!("seed-{fp}.txt")),
        plan::What::SeedQrFile(_) => Some(format!("seedqr-{fp}.png")),
        plan::What::PassphraseFile(_) => Some(format!("passphrase-{fp}.txt")),
        _ => None,
    }
}

/// Whether `name` is a file on a stick of one of the seeds `fps`, in any
/// form ([`seed_stick_name`]).
pub(crate) fn seed_stick_file_of(fps: &[String], name: &str) -> bool {
    fps.iter().any(|fp| {
        [
            plan::What::SeedFile(0),
            plan::What::SeedQrFile(0),
            plan::What::PassphraseFile(0),
        ]
        .into_iter()
        .any(|w| seed_stick_name(w, fp).as_deref() == Some(name))
    })
}

/// Costs a hash of a whole file (through `sha256_of`). Splits `text`
/// into its codes' parts: each of the pictures `names` (as
/// [`crate::picture_names`] names them for `text`) with the SHA-256 of
/// what its codes hold ([`crate::picture_payloads`]).
fn pictures_expect(
    c: &Compute,
    names: &[String],
    text: &str,
) -> Vec<(String, crate::backup_flow::ReadBack)> {
    names
        .iter()
        .cloned()
        .zip(crate::picture_payloads(text))
        .map(|(n, p)| {
            (
                n,
                crate::backup_flow::ReadBack::Codes(crate::sha256_of(c, &p)),
            )
        })
        .collect()
}

/// The software's own files of `stem`, by the numbers
/// [`Faraday::backup_public_for`] gives them: the descriptor where the
/// software reads no file of its own.
pub(crate) fn software_files(stem: &str, what: &[u8]) -> Vec<String> {
    what.iter()
        .map(|x| match x {
            2 => format!("{stem}-multisig-config.txt"),
            5 => format!("{stem}-wallet.json"),
            6 => format!("{stem}-bsms.txt"),
            7 => format!("{stem}-bitcoin-core.json"),
            _ => format!("{stem}-descriptor.txt"),
        })
        .collect()
}

/// The stick at a place on the map `boxes` that the flow's stick for
/// `target` is, as a list of none or one ([`crate::backup_flow::target_stick`],
/// `docs/BACKUP-FLOW.md` §19.4): each stick of the map is one stick of the
/// flow.
pub(crate) fn target_sticks(boxes: &[plan::Spot], target: Target) -> Vec<plan::At> {
    crate::backup_flow::target_stick(boxes, target)
        .into_iter()
        .collect()
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
    /// Costs what `sealed_marks` costs. A plan changed after the keys were
    /// made: a vault that now holds more than it was sealed with is as one
    /// added (the round puts it in and its sticks are written again), and a
    /// vault the plan has and the flow has not made starts the round. With
    /// `advance`, a flow that has not reached the writes goes on to the
    /// lock.
    pub(crate) fn made_plan_changed(&mut self, c: &Compute, advance: bool) {
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
                    let now = sealed_marks(c, wf, *v);
                    // Anything of this wallet added to it or taken out of
                    // it: a seed, a passphrase, the description, a share
                    // (§10.2, §17.6).
                    now.iter().any(|s| !had.contains(s)) || had.iter().any(|m| !now.contains(m))
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
        // A stick counts for the seed's files only where it is given one
        // of them (its words, its SeedQR, its passphrase), never from the
        // plan alone: a stick of files, or a vault's stick with files
        // beside the vault (`docs/BACKUP-FLOW.md` §19.4).
        out.extend(
            (0..self.slots.len())
                .filter(|&k| self.slots[k].files.iter().any(|n| self.is_seed_file(s, n))),
        );
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `name` is one of seed `s`'s own files: on the stick of
    /// files, or on a stick at a place in any form.
    pub(crate) fn is_seed_file(&self, s: usize, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let Some(seed) = w.shape.seeds.get(s) else {
            return false;
        };
        let fps = std::slice::from_ref(&seed.name);
        seed_file_of(&file_stem(&w.name), fps, name) || seed_stick_file_of(fps, name)
    }

    /// Costs a map of the plan (through `map`). Whether `name` is a seed's
    /// file the plan keeps on the stick of slot `slot`: the stick of files
    /// takes the seeds kept there; a stick at a place the seeds' files the
    /// plan keeps on that stick, beside its vault or not
    /// (`docs/BACKUP-FLOW.md` §17.2, §19.4).
    pub fn seed_file_ok_for(&self, c: &Compute, slot: usize, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let Some(target) = self.slots.get(slot).map(|s| s.target) else {
            return false;
        };
        let boxes = plan::map(c, &w.shape, &w.answers);
        if target == Target::Files {
            let fps: Vec<String> = plan::holds_at(&boxes, plan::At::Files)
                .into_iter()
                .filter_map(|what| match what {
                    plan::What::SeedFile(i) => w.shape.seeds.get(i).filter(|x| x.here),
                    _ => None,
                })
                .map(|x| x.name.clone())
                .collect();
            return seed_file_of(&file_stem(&w.name), &fps, name);
        }
        target_sticks(&boxes, target)
            .into_iter()
            .flat_map(|at| plan::holds_at(&boxes, at))
            .any(|what| {
                let seed = match what {
                    plan::What::SeedFile(i)
                    | plan::What::SeedQrFile(i)
                    | plan::What::PassphraseFile(i) => w.shape.seeds.get(i).filter(|x| x.here),
                    _ => None,
                };
                seed.is_some_and(|x| seed_stick_name(what, &x.name).as_deref() == Some(name))
            })
    }

    /// Costs what `seeds_as_files` and `map` cost. Whether `name` is a
    /// seed's file of the wallet this flow is for that the plan keeps: on
    /// the stick of files, or on a stick at a place.
    pub fn seed_file_ok(&self, c: &Compute, name: &str) -> bool {
        let Some(w) = self.wallet.as_ref() else {
            return false;
        };
        let kept = w.answers.seeds_as_files(c, &w.shape);
        let fps: Vec<String> = w
            .shape
            .seeds
            .iter()
            .enumerate()
            .filter(|(s, x)| x.here && kept.contains(s))
            .map(|(_, s)| s.name.clone())
            .collect();
        if seed_file_of(&file_stem(&w.name), &fps, name) {
            return true;
        }
        plan::map(c, &w.shape, &w.answers)
            .iter()
            .filter(|s| matches!(s.at, plan::At::Stick(..)))
            .flat_map(|s| s.holds.iter())
            .any(|(what, _)| {
                let seed = match *what {
                    plan::What::SeedFile(i)
                    | plan::What::SeedQrFile(i)
                    | plan::What::PassphraseFile(i) => w.shape.seeds.get(i).filter(|x| x.here),
                    _ => None,
                };
                seed.is_some_and(|x| seed_stick_name(*what, &x.name).as_deref() == Some(name))
            })
    }
}

impl Faraday {
    /// Costs what `plan_mapped` and `new_wallet_on` cost. The checklist's
    /// writes are done: the flow wrote every stick the plan as it is now
    /// names, so a plan changed since is not done.
    pub(crate) fn wallet_sticks_done(&self, c: &Compute) -> bool {
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
        let shape = self.plan_shape_now(b.wallet);
        let mapped = self.plan_mapped(c, &shape, &b.answers);
        let now = BackupFlow::new_wallet_on(c, shape, b.answers.clone(), "", &mapped);
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
            .backup_seed_list_now(b.wallet)
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
