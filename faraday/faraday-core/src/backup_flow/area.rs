//! The backup flow area's state: the flows begun, the one on screen, the
//! keys made here that are held until their backup is done, the number
//! a wallet's flow is tied to while it is made, and the board's list
//! scroll. Reading the area is reading its flows (a read-only `Deref`);
//! [`crate::Faraday::backup_flows`] reads them from outside the crate.
//! Changed through the methods here, and by the backup flow's own glue in
//! `backup_flow/` and its children.

use faraday_bip::keys::Fingerprint;

use faraday_crypto::work::Compute;

use super::{
    BackupFlow, BackupRow, Kind, Pick, Problem, Reading, Step, Subject, VaultPlan, WalletFlow,
    hex_text,
};
use crate::board::BoardState;

/// The backup flow area.
#[derive(Default)]
pub(crate) struct BackupFlowArea {
    /// The backup flows, at most one on screen (`docs/BACKUP-FLOW.md` §7).
    pub(super) all: Vec<BackupFlow>,
    /// The flow on screen.
    pub(super) on: Option<usize>,
    /// The keys made here whose backup is not yet done, by fingerprint
    /// (`docs/history/NEW-WALLET.md` §14.3): they sign nothing until it
    /// is. Never a secret; kept in a vault beside the seed and in the plan
    /// record.
    pub(super) made_here: Vec<Fingerprint>,
    /// The last number given to a Make a new wallet or a Restore, which a
    /// wallet's backup flow is tied to while the wallet is being made.
    pub(super) owner_seq: u64,
    /// The board's list of things, design units scrolled past
    /// (`docs/BACKUP-FLOW.md` §17.14).
    board_list_offset: f32,
}

/// Reading the area is reading its flows: `self.flows[i]` reads as the
/// accessor [`crate::Faraday::backup_flows`] does. Nothing is changed
/// through it.
impl std::ops::Deref for BackupFlowArea {
    type Target = Vec<BackupFlow>;
    fn deref(&self) -> &Vec<BackupFlow> {
        &self.all
    }
}

impl BackupFlowArea {
    /// The flow on screen.
    pub(crate) fn on(&self) -> Option<usize> {
        self.on
    }

    /// The keys made here whose backup is not yet done.
    pub(crate) fn made_here(&self) -> &[Fingerprint] {
        &self.made_here
    }

    /// Key `fp` was made here: it signs nothing until its backup is done.
    pub(crate) fn note_made(&mut self, fp: Fingerprint) {
        if !self.made_here.contains(&fp) {
            self.made_here.push(fp);
        }
    }

    /// The keys made here, as the held keys stored say.
    pub(crate) fn set_made_here(&mut self, made: Vec<Fingerprint>) {
        self.made_here = made;
    }

    /// Key `fp`, and the keys `same` with its words, are released.
    pub(crate) fn release_made(&mut self, fp: Fingerprint, same: &[Fingerprint]) {
        self.made_here.retain(|f| *f != fp && !same.contains(f));
    }

    /// The board's list of things, design units scrolled past.
    pub(crate) fn board_list_offset(&self) -> f32 {
        self.board_list_offset
    }

    /// The board's list scroll, to move it.
    pub(crate) fn board_list_offset_mut(&mut self) -> &mut f32 {
        &mut self.board_list_offset
    }

    /// Sets the board's list scroll.
    pub(crate) fn set_board_list_offset(&mut self, y: f32) {
        self.board_list_offset = y;
    }

    /// Flow `i`'s board, to change it.
    pub(crate) fn board_mut(&mut self, i: usize) -> Option<&mut BoardState> {
        self.all.get_mut(i).map(|f| &mut f.live.board)
    }

    /// The boards not on screen (every flow's but `shown`'s) let go of
    /// their empty sticks, menu and focus.
    pub(crate) fn board_tidy(&mut self, shown: Option<usize>) {
        for (i, f) in self.all.iter_mut().enumerate() {
            if shown != Some(i) {
                f.live.board.empty.clear();
                f.live.board.menu = None;
                f.live.board.focus = None;
            }
        }
    }

    /// The flow on screen's scroll offset, to move it.
    pub(crate) fn scroll_y_mut(&mut self) -> Option<&mut f32> {
        let i = self.on?;
        self.all.get_mut(i).map(|f| &mut f.live.scroll.y)
    }

    /// Sets the flow on screen's scroll, as the drawing pass noted it.
    pub(crate) fn set_scroll(&mut self, scroll: crate::flow::Scroll) {
        if let Some(i) = self.on
            && let Some(f) = self.all.get_mut(i)
        {
            f.live.scroll = scroll;
        }
    }

    /// Flow `i` put back as it was.
    pub(crate) fn restore(&mut self, i: usize, was: BackupFlow) {
        self.all[i] = was;
    }

    /// Drops flow `i`; what points past it follows. Whether it was the
    /// flow on screen.
    pub(crate) fn drop_flow(&mut self, i: usize) -> bool {
        self.all.remove(i);
        match self.on {
            Some(k) if k == i => {
                self.on = None;
                true
            }
            Some(k) if k > i => {
                self.on = Some(k - 1);
                false
            }
            _ => false,
        }
    }

    /// The files written to the stick of label `label` and serial
    /// `serial` named `from` (in any case) are named `to`, as the stick's
    /// file was renamed.
    pub(crate) fn rename_written(
        &mut self,
        label: &str,
        serial: Option<u32>,
        from: &str,
        to: &str,
    ) {
        for f in &mut self.all {
            for slot in &mut f.slots {
                let Some(w) = slot.written.as_mut() else {
                    continue;
                };
                if w.label != label || w.serial != serial {
                    continue;
                }
                for (n, _) in &mut w.files {
                    if n.eq_ignore_ascii_case(from) {
                        *n = to.to_string();
                    }
                }
            }
        }
    }

    /// Sets the codex32 string flow `i`'s round stopped on, or clears it
    /// (`Live::c32_wanted`).
    pub(crate) fn set_c32_wanted(&mut self, i: usize, want: Option<crate::plan::codex32::Wanted>) {
        if let Some(f) = self.all.get_mut(i) {
            f.live.c32_wanted = want;
        }
    }

    /// The flows, for the tests' kit to set what they hold.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn all_mut(&mut self) -> &mut Vec<BackupFlow> {
        &mut self.all
    }
}

impl BackupFlowArea {
    /// The flow that made the key `subject`, when it is kept.
    pub(crate) fn of(&self, kind: Kind, subject: &[u8]) -> Option<&BackupFlow> {
        let id = hex_text(subject);
        self.all
            .iter()
            .find(|f| f.kind == kind && f.made && f.subject.as_ref().is_some_and(|s| s.id == id))
    }

    /// The writes the key `subject` waits for: how many are done and how
    /// many there are. `None` once every place the plan gives it is
    /// written, and for a key no flow made here, which signs at once
    /// (§8).
    pub(crate) fn waits(&self, kind: Kind, subject: &[u8]) -> Option<(usize, usize)> {
        self.of(kind, subject)
            .filter(|f| f.waiting())
            .map(|f| f.written_of())
    }

    /// The key page's Backup row: `None` for a key unlocked from a stick.
    pub(crate) fn row(&self, kind: Kind, subject: &[u8]) -> Option<BackupRow> {
        let f = self.of(kind, subject)?;
        let (n, m) = f.written_of();
        let en = &crate::strings::EN;
        let line = if f.all_written() {
            en.backup_flow_row_all.to_string()
        } else if n == 0 {
            en.backup_flow_row_none.to_string()
        } else {
            crate::strings::fill(en.backup_flow_row_some, &[&n.to_string(), &m.to_string()])
        };
        Some(BackupRow {
            line,
            waiting: !f.all_written(),
        })
    }

    /// Costs what `BackupFlow::new` costs, once a key whose flow did not
    /// read. Reads the keys that wait. One whose flow did not read comes
    /// back as a flow of its own at Where, made and unwritten, so it still
    /// waits.
    pub(crate) fn gate_restore(&mut self, c: &Compute, bytes: &[u8]) {
        for line in String::from_utf8_lossy(bytes).lines() {
            let mut p = line.splitn(4, '\t');
            let (Some(k), Some(id), Some(name)) = (p.next(), p.next(), p.next()) else {
                continue;
            };
            let held = p.next().unwrap_or("");
            let Some(kind) = Kind::from_code(k) else {
                continue;
            };
            let known = self.all.iter().any(|f| {
                f.kind == kind && f.made && f.subject.as_ref().is_some_and(|s| s.id == id)
            });
            if known {
                // What the gate holds back is held, whatever the flow's
                // own state says.
                if kind.is_wallet()
                    && let Some(w) = self
                        .all
                        .iter_mut()
                        .find(|f| {
                            f.kind == kind
                                && f.made
                                && f.subject.as_ref().is_some_and(|s| s.id == id)
                        })
                        .and_then(|f| f.wallet.as_mut())
                {
                    for h in held.split(',').filter(|x| !x.is_empty()) {
                        if !w.held.iter().any(|x| x.eq_ignore_ascii_case(h)) {
                            w.held.push(h.to_string());
                        }
                    }
                }
                continue;
            }
            let mut f = BackupFlow::new(c, kind);
            f.made = true;
            f.on_screen = false;
            f.subject = Some(Subject {
                id: id.to_string(),
                name: name.to_string(),
            });
            if kind.is_wallet() {
                let mut w = WalletFlow::blank();
                w.checksum = Some(id.to_string());
                w.name = name.to_string();
                w.held = held
                    .split(',')
                    .filter(|x| !x.is_empty())
                    .map(str::to_string)
                    .collect();
                f.wallet = Some(w);
            }
            self.all.push(f);
        }
    }

    /// The flow Home leads with and the Power sheet lists: made, and not
    /// every place written.
    pub(crate) fn pending(&self) -> Option<(usize, &BackupFlow)> {
        self.all.iter().enumerate().find(|(_, f)| f.waiting())
    }

    /// What the Power sheet says of each backup not finished (§9).
    pub(crate) fn power_lines(&self) -> Vec<String> {
        self.all
            .iter()
            .filter(|f| f.waiting())
            .map(|f| {
                let (n, m) = f.written_of();
                crate::strings::fill(
                    crate::strings::EN.backup_flow_power,
                    &[
                        &f.subject
                            .as_ref()
                            .map_or(f.kind.name().to_string(), |s| s.name.clone()),
                        &n.to_string(),
                        &m.to_string(),
                    ],
                )
            })
            .collect()
    }

    /// A flow not finished and not yet at the key made, of this kind: what
    /// the kind's tile resumes (**Continue**).
    pub(crate) fn resumable(&self, kind: Kind) -> Option<usize> {
        self.all
            .iter()
            .position(|f| f.kind == kind && !f.made && f.step != Step::Done)
    }

    /// Copies and public files kept in Files for the flow's writes are
    /// listed on the visit as In the backup, not tickable.
    pub(crate) fn in_the_backup(&self, name: &str) -> bool {
        self.all.iter().any(|f| {
            f.made
                && f.step != Step::Done
                && (f.sealing.iter().any(|s| s.name == name)
                    // A Secure Boot key's enrolment files, written last.
                    || f.public.iter().any(|n| n == name)
                    || f.slots.iter().any(|s| s.files.iter().any(|n| n == name)))
        })
    }
}

/// What one flow works out and changes of itself.
impl BackupFlow {
    /// Whether the stick read is one this flow wrote: it holds a file the
    /// flow wrote for another stick (by SHA-256) and reads as that stick
    /// (its label and serial): the first copy of the same vault, or another
    /// place's files. A stick that reads otherwise is compared with what
    /// was read for its own place.
    pub(crate) fn holds_written(&self, r: &Reading) -> Option<Problem> {
        let f = self;
        for (k, s) in f.slots.iter().enumerate() {
            if k == r.slot {
                continue;
            }
            let Some(w) = &s.written else {
                continue;
            };
            // A vault file by its SHA-256; a file beside it, which is not
            // read, by its name in the listing (a stick of files holds no
            // vault, `docs/BACKUP-FLOW.md` §17.4).
            let holds = r
                .files
                .iter()
                .any(|(vr, _)| w.files.iter().any(|(_, sha)| *sha == vr.sha))
                || w.files.iter().any(|(n, _)| {
                    !n.to_ascii_lowercase().ends_with(".ofv")
                        && r.listing.iter().any(|(x, _)| x == n)
                });
            if holds && w.label == r.label && w.serial == r.serial {
                let mine = f.vaults_of(r.slot);
                let theirs = f.vaults_of(k);
                let same_vault = mine.iter().any(|v| theirs.contains(v));
                return Some(if same_vault {
                    Problem::HasFirst
                } else {
                    Problem::HasFilesOf(s.target)
                });
            }
        }
        None
    }

    /// The stick of place `p` carries the key's public files.
    pub(crate) fn public_at(&mut self, p: usize) {
        if self.kind.is_wallet() {
            return;
        }
        let f = self;
        if p >= f.plan.places.len() || f.slots_at(p).is_empty() {
            return;
        }
        // Files already written stay where they are.
        if f.slots
            .iter()
            .any(|s| s.written.is_some() && s.files.iter().any(|n| f.public.contains(n)))
        {
            return;
        }
        f.plan.public_at = p;
        let names = f.public.clone();
        for s in &mut f.slots {
            s.files.retain(|n| !names.contains(n));
        }
        if f.kind != Kind::SecureBoot
            && let Some(k) = f.public_slot()
        {
            f.slots[k].files.extend(names);
        }
    }

    pub(crate) fn needs_bind(&self, slot: usize) -> bool {
        let f = self;
        let Some(s) = f.slots.get(slot) else {
            return false;
        };
        if s.bound.is_some() {
            return false;
        }
        match s.target {
            t if t.vault_stick() => {
                if !f.adding {
                    return true;
                }
                // The round reads only a stick that holds a vault the key
                // is to be copied into.
                f.vaults_of(slot)
                    .iter()
                    .any(|&v| f.plan.vaults[v].existing && f.plan.vaults[v].salt.is_none())
            }
            _ => false,
        }
    }

    /// "I do not have its passphrase": a new vault beside the one on the
    /// stick; the plan's vault becomes new and is written beside that
    /// file at step 6.
    pub(crate) fn beside_it(&mut self, v: usize) {
        let f = self;
        let Some(vp) = f.plan.vaults.get_mut(v) else {
            return;
        };
        // The file stays in Files; it is no longer the plan's vault.
        *vp = VaultPlan::default();
        f.live.no_pass = None;
        if f.live.unlocking == Some(v) {
            f.live.unlocking = None;
        }
        for s in &mut f.slots {
            if let Some(b) = s.bound.as_mut() {
                for (pv, pick) in &mut b.picks {
                    if *pv == v {
                        *pick = Pick::New;
                    }
                }
            }
        }
    }

    /// Leave this stick for another. A slot that is not written loses its
    /// binding, at step 6 too (a stick lost or broken, a read-only one whose
    /// write failed): the next stick takes the path of a stick named after
    /// step 2.
    pub(crate) fn use_another(&mut self) {
        let f = self;
        if let Some(r) = f.live.reading.take() {
            if let Some(s) = f.slots.get_mut(r.slot)
                && s.written.is_none()
            {
                s.bound = None;
            }
            f.live.ignored.push(r.id);
        }
        f.live.problem = None;
    }

    pub(crate) fn refuse(&mut self, p: Problem) {
        if let Some(r) = self.live.reading.as_mut() {
            r.problem = Some(p);
        }
    }
}
