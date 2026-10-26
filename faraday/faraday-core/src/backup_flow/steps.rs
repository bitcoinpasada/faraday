//! What the backup flow does at each step (`docs/BACKUP-FLOW.md` §4):
//! editing the plan, reading a stick with nothing secret open, binding it
//! to its place, unlocking and making the vaults inside the flow, making
//! the key into every vault, and writing each stick by place. The state is
//! `backup_flow.rs`; the cards are `backup_flow/screen.rs`.

use faraday_crypto::work::Compute;
use faraday_vault as fv;
use faraday_vault::records::kind;

use crate::backup_flow::{
    BackupFlow, Bound, Confirm, Kind, Pick, Problem, Purpose, Reading, Sealed, Step, Subject,
    Target, VaultPlan, VaultRead, Writing, Written, hex_text, subject_of,
};
use crate::strings::{EN as FEN, fill};
use crate::vault::{Focus, Form, TextBox, Work};
use crate::{Action, Faraday, FileKind, Item, Screen, Sheet, StickInfo, StorageCommand, sha256_of};

/// Everything a press in the flow can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowAction {
    /// Open a card, or close it.
    Open(u8),
    /// A press on a wallet's plan cards, as Back up's
    /// (`docs/BACKUP-FLOW.md` §15).
    Plan(crate::PlanPress),
    /// Leave the flow, keeping its state.
    Back,
    /// Open the first card to change the plan.
    ChangePlan,
    /// Another place.
    AddPlace,
    /// Take a place out of the plan.
    RemovePlace(usize),
    /// The place keeps the stick of this vault (a new one when the index
    /// is the number of vaults).
    PlaceVault(usize, usize),
    /// The vault is an existing one, or a new one.
    VaultExisting(usize),
    /// Go on from the first card.
    Continue,
    /// Lock, then bring in the sticks.
    LockBring,
    /// Use the stick that is in the machine already, by its place among
    /// the sticks.
    UseBoot(usize),
    /// The vault is the file of this place in the stick's vault files.
    PickFile(usize, usize),
    /// The vault is a new one beside what the stick holds.
    PickBeside(usize),
    /// The copy of an existing vault the stick holds is the one to use
    /// (or the first copy read stays).
    UseCopy(bool),
    /// A tap: this is the stick for its place.
    Tap,
    /// A hold: this is the stick for its place, or a second stick.
    Hold,
    /// Leave this stick; ask for another.
    UseAnother,
    /// "I do not have its passphrase", for a vault.
    NoPass(usize),
    /// A new vault beside the one whose passphrase is not known.
    BesideIt(usize),
    /// Another stick for the vault.
    AnotherStick(usize),
    /// Unlock a vault of the plan.
    UnlockVault(usize),
    /// Make a vault of the plan.
    CreateVault(usize),
    /// Make the key.
    Make,
    /// The key is made: on to the lock.
    MadeContinue,
    /// Save the key into the vault added after it was made.
    CopyKey,
    /// Lock, then write.
    LockWrite,
    /// Unlock a vault of the plan once the sticks are written.
    UnlockDone(usize),
    /// The copy at this place was destroyed: it leaves the map.
    Destroyed(usize),
    /// A waiting action: open the flow of the key selected.
    BackUpFirst(Kind),
    /// Open the flow that made the key shown.
    Resume,
    /// Open flow `n`, from Home's lead.
    Show(usize),
    /// A place keeps several vaults on one stick, or on one each.
    Together(usize),
    /// Place `p` also keeps (or stops keeping) the stick of vault `v`.
    AlsoVault(usize, usize),
    /// The stick of this place carries the key's public files.
    PublicAt(usize),
    /// A wallet's answer to a question of the plan, as Back up's plan
    /// ticks it: the list and the row.
    Answer(u8, u8),
    /// The checklist's **Lock and write**: the wallet's flow takes the
    /// vaults and the files, and locks.
    WalletLock,
    /// Steps 1 to 3 are done: on to the wallet's keys.
    WalletMake,
}

/// Whether an action acts when held.
pub fn is_hold(a: FlowAction) -> bool {
    matches!(a, FlowAction::Hold)
}

/// The room a new vault is counted at before its size is chosen: the
/// default slot's file.
fn new_vault_len() -> u64 {
    fv::file_len(fv::DEFAULT_SLOT) as u64
}

impl Faraday {
    // ---- the flow on screen ----

    fn fi(&self) -> Option<usize> {
        self.flows.on.filter(|&i| i < self.flows.len())
    }

    /// The flow on screen.
    pub fn flow(&self) -> Option<&BackupFlow> {
        self.flows.get(self.fi()?)
    }

    fn flow_mut(&mut self) -> Option<&mut BackupFlow> {
        let i = self.fi()?;
        self.flows.all.get_mut(i)
    }

    /// The flow is on screen and not left.
    pub(crate) fn flow_screen(&self) -> bool {
        self.screen == Screen::BackupFlow && self.fi().is_some()
    }

    /// Starts the flow for `kind` on its first card, or resumes one
    /// that has not made its key (§5).
    pub(crate) fn flow_begin(&mut self, kind: Kind) {
        let c = self.compute();
        if let Some(i) = self.flows.resumable(kind) {
            return self.flow_show(i);
        }
        self.flows.all.push(BackupFlow::new(&c, kind));
        let i = self.flows.len() - 1;
        self.flow_show(i);
    }

    /// Opens flow `i` on its step.
    pub(crate) fn flow_show(&mut self, i: usize) {
        if i >= self.flows.len() {
            return;
        }
        self.flows.on = Some(i);
        self.screen = Screen::BackupFlow;
        self.osk_leave();
        let f = &mut self.flows.all[i];
        f.on_screen = true;
        f.live.open = Some(f.step.n());
        f.live.problem = None;
        self.flow_enter();
    }

    /// Prepares what the step on screen needs.
    pub(crate) fn flow_enter(&mut self) {
        let Some(i) = self.fi() else {
            return;
        };
        let (step, kind, made) = {
            let f = &self.flows[i];
            (f.step, f.kind, f.made)
        };
        match step {
            Step::Vaults => {
                self.vaults.set_back_to(Some(Screen::BackupFlow));
            }
            Step::Make if !made && !kind.is_wallet() => self.flow_make_form(kind),
            Step::Sticks | Step::Write => self.flow_examine(),
            _ => {}
        }
        self.flow_settle();
        self.out.dirty = true;
    }

    fn flow_goto(&mut self, i: usize, step: Step) {
        let f = &mut self.flows.all[i];
        f.step = step;
        f.live.open = Some(step.n());
        f.live.problem = None;
        f.live.no_pass = None;
        f.live.unlocking = None;
        f.live.reading = None;
        if step == Step::Done {
            self.flow_finish(i);
        }
        if self.fi() == Some(i) {
            self.flow_enter();
        }
    }

    // ---- the plan ----

    pub(crate) fn flow_open_index(&self, i: usize, v: usize) -> Option<usize> {
        let salt = self.flows[i].plan.vaults.get(v)?.salt?;
        self.vaults
            .open
            .iter()
            .position(|o| o.header().salt == salt)
    }

    /// The vaults of flow `i` that must be open for the key to be made or
    /// copied, and those that hold it.
    fn flow_needed_vaults(&self, i: usize) -> Vec<usize> {
        let f = &self.flows[i];
        let used = f.plan.used_vaults();
        if !f.adding {
            return used;
        }
        // The round for a vault added after the key: the added ones, and
        // one that holds the key to copy it from.
        let mut need: Vec<usize> = used
            .iter()
            .copied()
            .filter(|v| !f.made_in.contains(v))
            .collect();
        // A wallet's round needs, of the vaults that hold keys, those that
        // hold the seeds the added ones take.
        if let Some(wf) = f.wallet.as_ref() {
            let mut want: Vec<usize> = need
                .iter()
                .flat_map(|&v| wf.answers.vault_seeds(&wf.shape, v))
                .collect();
            want.sort_unstable();
            want.dedup();
            for &v in used.iter().filter(|v| f.made_in.contains(v)) {
                let has = wf.answers.vault_seeds(&wf.shape, v);
                if has.iter().any(|s| want.contains(s)) {
                    need.push(v);
                    want.retain(|s| !has.contains(s));
                }
            }
            return need;
        }
        let holder = used
            .iter()
            .copied()
            .find(|v| f.made_in.contains(v) && self.flow_open_index(i, *v).is_some())
            .or_else(|| used.iter().copied().find(|v| f.made_in.contains(v)));
        need.extend(holder);
        need
    }

    /// Automatic transitions: a step whose work is done goes on.
    pub(crate) fn flow_settle(&mut self) {
        let mut sticks_done = None;
        for _ in 0..8 {
            let Some(i) = self.fi() else {
                return;
            };
            let step = self.flows[i].step;
            let next = match step {
                Step::Sticks => {
                    let f = &self.flows[i];
                    let unbound = (0..f.slots.len()).any(|s| self.flows[i].needs_bind(s));
                    (!unbound && f.live.reading.is_none()).then_some(Step::Vaults)
                }
                Step::Vaults => {
                    let ready = self
                        .flow_needed_vaults(i)
                        .iter()
                        .all(|&v| self.flow_open_index(i, v).is_some());
                    ready.then_some(Step::Make)
                }
                Step::Write if self.flows[i].all_written() => Some(Step::Done),
                // Coming back with a vault open again: the flow is on Lock,
                // which seals it (§5). Written sticks stay written.
                Step::Write if !self.clean_now() && self.flows[i].live.writing.is_none() => {
                    Some(Step::Lock)
                }
                _ => None,
            };
            match next {
                Some(s) => {
                    if step == Step::Sticks && s == Step::Vaults {
                        sticks_done = Some(i);
                    }
                    self.flow_goto(i, s);
                    // A wallet's keys are made in Make a new wallet, not
                    // in a card of the flow: steps 1 to 3 done, it goes
                    // back there.
                    let f = &self.flows[i];
                    if s == Step::Make && f.kind.is_wallet() && !f.made && self.fi() == Some(i) {
                        self.wallet_flow_leave(i);
                    }
                }
                None => break,
            }
        }
        if let Some(i) = sticks_done {
            self.flow_create_first(i);
        }
    }

    /// The last stick bound: step 3 offers each vault found to open and
    /// each new one to create, and unlocks nothing by itself. Where only
    /// Create applies (no vault found on the sticks, none open in memory),
    /// Create a vault opens for the first new vault at once, as **Create
    /// {vault}** would, its Pull sheet first while a stick is in (owner,
    /// 2026-10-25, 2026-10-26).
    fn flow_create_first(&mut self, i: usize) {
        if self.fi() != Some(i) || !self.flow_screen() || !self.vaults.open.is_empty() {
            return;
        }
        let f = &self.flows[i];
        if f.step != Step::Vaults || f.live.creating.is_some() || f.live.unlocking.is_some() {
            return;
        }
        let need = self.flow_needed_vaults(i);
        let mut left: Vec<usize> = f
            .plan
            .used_vaults()
            .into_iter()
            .filter(|v| need.contains(v) && self.flow_open_index(i, *v).is_none())
            .collect();
        left.sort_unstable();
        let only_new = left.iter().all(|&v| f.plan.vaults[v].salt.is_none());
        if let Some(&v) = left.first().filter(|_| only_new) {
            self.act(Action::Flow(FlowAction::CreateVault(v)));
        }
    }

    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn flow_act(&mut self, a: FlowAction) {
        use FlowAction as F;
        match a {
            F::BackUpFirst(_) | F::Resume => return self.flow_resume_selected(),
            F::Show(i) => return self.flow_show(i),
            // The checklist's last item starts the flow itself.
            F::WalletLock => return self.wallet_lock_write(),
            _ => {}
        }
        let Some(i) = self.fi() else {
            return;
        };
        // The plan does not change under a write: nothing of it is acted on
        // until the stick is pulled or the write is done.
        if self.flows[i].live.writing.is_some()
            && matches!(
                a,
                F::ChangePlan
                    | F::AddPlace
                    | F::RemovePlace(_)
                    | F::Destroyed(_)
                    | F::PlaceVault(..)
                    | F::VaultExisting(_)
                    | F::Continue
                    | F::Together(_)
                    | F::AlsoVault(..)
                    | F::PublicAt(_)
                    | F::Answer(..)
                    | F::Plan(_)
            )
        {
            return;
        }
        match a {
            F::Open(n) => {
                let f = &mut self.flows.all[i];
                f.live.open = if f.live.open == Some(n) {
                    None
                } else {
                    Some(n)
                };
                f.live.scroll.follow = true;
            }
            // Back leaves on any card but while writing, where pulling the
            // stick is the way to stop (§5).
            F::Back if self.flows[i].live.writing.is_some() => {}
            // A wallet's flow left before its keys exist goes back to the
            // Make a new wallet or Restore it came from.
            F::Back if self.wallet_flow_back(i) => {}
            F::Back => {
                self.flows.all[i].on_screen = false;
                self.flows.on = None;
                self.vaults.leave_create();
                self.screen = Screen::Home;
            }
            F::ChangePlan => {
                let f = &mut self.flows.all[i];
                f.live.open = Some(Step::Where.n());
                f.live.scroll.follow = true;
                // A wallet's plan opens on its board (`docs/BACKUP-FLOW.md`
                // §17.21 item 11).
                if f.kind.is_wallet() {
                    f.live.plan_q = Some(crate::qstep::BOARD);
                }
            }
            F::AddPlace => self.flow_add_place(i),
            F::RemovePlace(p) => self.flow_remove_place(i, p, false),
            F::Destroyed(p) => self.flow_remove_place(i, p, true),
            F::PlaceVault(p, v) => self.flow_place_vault(i, p, v),
            F::AlsoVault(p, v) => self.flow_also_vault(i, p, v),
            F::Together(p) => self.flow_together(i, p),
            F::PublicAt(p) => self.flows.all[i].public_at(p),
            F::Answer(list, row) => self.flow_wallet_answer(i, list, row),
            F::Plan(press) => self.flow_plan_press(i, press),
            F::WalletMake => self.wallet_flow_leave(i),
            F::VaultExisting(v) => {
                let f = &mut self.flows.all[i];
                if let Some(vp) = f.plan.vaults.get_mut(v)
                    && vp.salt.is_none()
                {
                    vp.existing = !vp.existing;
                }
            }
            F::Continue => self.flow_continue(i),
            F::LockBring => self.lock_ask(true),
            // One stick, one place: the boot stick that holds a place is
            // not taken for another; at step 6 it is offered for its own.
            F::UseBoot(_)
                if (0..self.flows[i].slots.len()).any(|j| {
                    Some(j) != self.flow_slot_wanted(i) && self.flows[i].boot_holds(j)
                }) => {}
            F::UseBoot(k) => self.flow_use_present(i, k),
            F::PickFile(v, k) => self.flow_pick_file(i, v, k),
            F::PickBeside(v) => self.flow_pick_beside(i, v),
            F::UseCopy(this) => self.flow_use_copy(i, this),
            F::Tap => self.flow_confirm(i, false),
            F::Hold => self.flow_confirm(i, true),
            F::UseAnother => self.flows.all[i].use_another(),
            F::NoPass(v) => {
                let f = &mut self.flows.all[i];
                f.live.no_pass = if f.live.no_pass == Some(v) {
                    None
                } else {
                    Some(v)
                };
            }
            F::BesideIt(v) => self.flows.all[i].beside_it(v),
            F::AnotherStick(v) => self.flow_another_stick(i, v),
            F::UnlockVault(v) => self.flow_unlock_start(i, v),
            F::CreateVault(v) => self.flow_create_start(i, v),
            F::Make => self.flow_make_ask(i),
            F::MadeContinue => {
                let f = &self.flows[i];
                let next = if f.slots.is_empty() {
                    Step::Done
                } else {
                    Step::Lock
                };
                self.flow_goto(i, next);
            }
            F::CopyKey => self.flow_copy_key(i),
            F::LockWrite => self.flow_lock_write(i),
            F::UnlockDone(v) => self.flow_unlock_done(i, v),
            F::BackUpFirst(_) | F::Resume | F::Show(_) | F::WalletLock => {}
        }
        self.out.dirty = true;
    }

    fn flow_add_place(&mut self, i: usize) {
        let c = self.compute();
        if self.flows[i].kind.is_wallet() {
            return self.flow_wallet_add_place(i);
        }
        let f = &mut self.flows.all[i];
        let v = f.plan.used_vaults().first().copied().unwrap_or(0);
        if f.plan.vaults.is_empty() {
            f.plan.vaults.push(VaultPlan::default());
        }
        f.plan.places.push(vec![v]);
        f.sync_slots(&c);
    }

    /// Takes place `p` out of the plan: before its write at once, after
    /// it only as "I destroyed this copy" (§4.8). The key is never taken
    /// out of the last vault that holds it.
    fn flow_remove_place(&mut self, i: usize, p: usize, destroyed: bool) {
        let c = self.compute();
        // A wallet's places are its plan's: Back up's questions take them
        // out.
        if self.flows[i].kind.is_wallet() && !destroyed {
            return;
        }
        let f = &self.flows[i];
        let at = f.slots_at(p);
        if at.is_empty() {
            return;
        }
        let written = at.iter().any(|&k| f.slots[k].written.is_some());
        if written != destroyed {
            return;
        }
        if f.plan.stick_places().len() <= f.kind.min_places() {
            return;
        }
        let used_before = f.plan.used_vaults();
        let mut after = f.plan.clone();
        after.places.remove(p);
        if f.made && !after.used_vaults().iter().any(|v| f.made_in.contains(v)) {
            return;
        }
        let f = &mut self.flows.all[i];
        f.plan.places.remove(p);
        f.plan.together.retain(|&q| q != p);
        f.plan.place_files.retain(|&q| q != p);
        for q in f.plan.place_files.iter_mut() {
            if *q > p {
                *q -= 1;
            }
        }
        for q in f.plan.together.iter_mut() {
            if *q > p {
                *q -= 1;
            }
        }
        if f.plan.public_at == p {
            f.plan.public_at = 0;
        } else if f.plan.public_at > p {
            f.plan.public_at -= 1;
        }
        let mut orphaned: Vec<String> = Vec::new();
        f.slots.retain(|s| {
            let gone = s.target.place() == Some(p) || s.target.files_place() == Some(p);
            if gone {
                orphaned.extend(s.files.iter().cloned());
            }
            !gone
        });
        for s in &mut f.slots {
            s.target = match s.target {
                Target::Place(q) if q > p => Target::Place(q - 1),
                Target::PlaceVault(q, v) if q > p => Target::PlaceVault(q - 1, v),
                Target::PlaceFiles(q) if q > p => Target::PlaceFiles(q - 1),
                Target::Another(q, v, n) if q > p => Target::Another(q - 1, v, n),
                t => t,
            };
        }
        for vp in &mut f.plan.vaults {
            if let Some((fp, _)) = vp.from.as_mut()
                && *fp > p
            {
                *fp -= 1;
            }
        }
        // The public files go to the first stick that is left.
        orphaned.retain(|n| !f.slots.iter().any(|s| s.files.contains(n)));
        if let Some(first) = f.slots.first_mut() {
            first.files.extend(orphaned);
        }
        f.live.reading = None;
        self.flow_tidy(i, &used_before);
        self.flows.all[i].sync_slots(&c);
    }

    fn flow_place_vault(&mut self, i: usize, p: usize, v: usize) {
        let c = self.compute();
        if self.flows[i].kind.is_wallet() {
            return;
        }
        let f = &self.flows[i];
        if p >= f.plan.places.len() {
            return;
        }
        if f.slots_at(p).iter().any(|&k| f.slots[k].written.is_some()) {
            return;
        }
        let v = v.min(f.plan.vaults.len());
        let used_before = f.plan.used_vaults();
        let mut after = f.plan.clone();
        after.places[p] = vec![v];
        if f.made && !after.used_vaults().iter().any(|x| f.made_in.contains(x)) {
            return;
        }
        let f = &mut self.flows.all[i];
        if v >= f.plan.vaults.len() {
            f.plan.vaults.push(VaultPlan::default());
        }
        f.plan.places[p] = vec![v];
        f.plan.together.retain(|&q| q != p);
        f.live.reading = None;
        self.flow_tidy(i, &used_before);
        self.flows.all[i].sync_slots(&c);
    }

    /// Place `p` also keeps the stick of vault `v`, or stops (§4.2: a
    /// place may hold a stick for each vault).
    fn flow_also_vault(&mut self, i: usize, p: usize, v: usize) {
        let c = self.compute();
        if self.flows[i].kind.is_wallet() {
            return;
        }
        let f = &self.flows[i];
        if p >= f.plan.places.len() || v >= f.plan.vaults.len() {
            return;
        }
        if f.slots_at(p).iter().any(|&k| f.slots[k].written.is_some()) {
            return;
        }
        let used_before = f.plan.used_vaults();
        let mut after = f.plan.clone();
        if after.places[p].contains(&v) {
            if after.places[p].len() < 2 {
                return;
            }
            after.places[p].retain(|&x| x != v);
        } else {
            after.places[p].push(v);
        }
        if f.made && !after.used_vaults().iter().any(|x| f.made_in.contains(x)) {
            return;
        }
        let f = &mut self.flows.all[i];
        f.plan.places[p] = after.places[p].clone();
        f.live.reading = None;
        self.flow_tidy(i, &used_before);
        self.flows.all[i].sync_slots(&c);
    }

    /// **One stick for both**: a place that keeps several vaults keeps them
    /// on one stick, or on one each.
    fn flow_together(&mut self, i: usize, p: usize) {
        let c = self.compute();
        // A wallet's sticks are its map's, a vault on each
        // (`docs/BACKUP-FLOW.md` §17.4).
        if self.flows[i].kind.is_wallet() {
            return;
        }
        let f = &mut self.flows.all[i];
        if f.plan.places.get(p).is_none_or(|vs| vs.len() < 2) {
            return;
        }
        if f.slots_at(p).iter().any(|&k| f.slots[k].written.is_some()) {
            return;
        }
        if f.plan.together.contains(&p) {
            f.plan.together.retain(|&q| q != p);
        } else {
            f.plan.together.push(p);
        }
        f.live.reading = None;
        f.sync_slots(&c);
    }

    /// Drops the vaults no place keeps any more. An open vault that holds
    /// only this flow's key (and its label and the signed-amount memory) is
    /// dropped with it; any other loses the key and stays. A vault that is
    /// locked is left as it is: its file stays in Files and the plan lets go
    /// of it. The key is never taken out of the last vault that holds it.
    fn flow_tidy(&mut self, i: usize, used_before: &[usize]) {
        if self.flows[i].kind.is_wallet() {
            return;
        }
        let (dropped, kind, subject, holders_left) = {
            let f = &self.flows[i];
            let used_now = f.plan.used_vaults();
            let dropped: Vec<usize> = used_before
                .iter()
                .copied()
                .filter(|v| !used_now.contains(v))
                .collect();
            let holders_left = used_now.iter().any(|v| f.made_in.contains(v));
            (
                dropped,
                f.kind,
                f.subject.as_ref().map(|s| s.id.clone()),
                holders_left,
            )
        };
        for v in dropped {
            let vp = self.flows[i].plan.vaults[v].clone();
            let Some(salt) = vp.salt else {
                continue;
            };
            let Some(o) = self
                .vaults
                .open
                .iter()
                .position(|o| o.header().salt == salt)
            else {
                continue;
            };
            if !holders_left {
                continue;
            }
            let only_key = subject
                .as_deref()
                .is_some_and(|id| self.flow_vault_holds_only(o, kind, id));
            if only_key && !vp.existing {
                self.vaults.close_at(o);
                let same = |it: &Item| {
                    it.kind == FileKind::Vault
                        && fv::read_header(&it.bytes).is_ok_and(|h| h.salt == salt)
                };
                self.outbox.retain(|it| !same(it));
                self.inbox.retain(|it| !same(it));
                self.save_boxes();
            } else if let Some(id) = subject.as_deref() {
                self.flow_remove_subject(o, kind, id);
            }
        }
        // The vaults are renumbered: what refers to one follows it.
        let f = &mut self.flows.all[i];
        let used = f.plan.used_vaults();
        f.plan.tidy();
        let renum = |v: usize| used.iter().position(|&u| u == v);
        f.made_in = f.made_in.iter().filter_map(|&v| renum(v)).collect();
        f.sealing = f
            .sealing
            .iter()
            .filter_map(|s| {
                renum(s.vault).map(|v| Sealed {
                    vault: v,
                    ..s.clone()
                })
            })
            .collect();
        for s in &mut f.slots {
            if let Some(b) = s.bound.as_mut() {
                b.picks = b
                    .picks
                    .iter()
                    .filter_map(|(v, p)| renum(*v).map(|n| (n, p.clone())))
                    .collect();
                b.declined = b.declined.iter().filter_map(|&v| renum(v)).collect();
            }
        }
    }

    /// Whether open vault `o` holds nothing but this flow's key: its label
    /// and the signed-amount memory are not entries.
    fn flow_vault_holds_only(&self, o: usize, kind_: Kind, id: &str) -> bool {
        self.vaults.open.get(o).is_some_and(|open| {
            self.vaults.records(open.id).iter().all(|(_, r)| {
                r.kind() == kind::SLOT_LABEL
                    || r.kind() == kind::AMOUNTS
                    || (r.kind() == kind_.record_kind()
                        && subject_of(kind_, r).as_deref() == Some(id))
            })
        })
    }

    fn flow_remove_subject(&mut self, o: usize, kind_: Kind, id: &str) {
        if let Some(vid) = self.vaults.open.get(o).map(|o| o.id) {
            let gone = self.vaults.edit(vid, |s| {
                s.remove_where(vid, |r| {
                    r.kind() == kind_.record_kind() && subject_of(kind_, r).as_deref() == Some(id)
                })
            });
            if gone > 0 {
                self.vaults.changed(vid);
            }
        }
    }

    /// Continue on the first card, or after a change of plan.
    fn flow_continue(&mut self, i: usize) {
        let c = self.compute();
        if self.flows[i].kind.is_wallet()
            && self.flows[i]
                .wallet
                .as_ref()
                .is_some_and(|w| w.answers.empty_vault().is_some())
        {
            return;
        }
        let f = &mut self.flows.all[i];
        f.sync_slots(&c);
        // A wallet may keep nothing on a stick at all: its plan decides.
        if !f.kind.is_wallet() && f.plan.stick_places().len() < f.kind.min_places() {
            return;
        }
        if f.step == Step::Where {
            f.step = Step::Sticks;
            f.live.open = Some(Step::Sticks.n());
        } else {
            // A change made later: what it asks for.
            let unbound = (0..f.slots.len())
                .any(|s| f.slots[s].target.place().is_some() && f.slots[s].bound.is_none());
            if f.made {
                f.made_plan_changed(&c, true);
                f.live.open = Some(f.step.n());
            } else {
                if unbound {
                    f.step = Step::Sticks;
                }
                f.live.open = Some(f.step.n());
            }
        }
        self.flow_enter();
    }

    // ---- reading a stick ----

    /// The sticks in the machine that are not the medium it started from.
    fn flow_candidates(&self) -> Vec<usize> {
        (0..self.stick.len())
            .filter(|&k| !self.stick[k].boot)
            .collect()
    }

    /// The slot of the flow on screen whose stick is wanted now: the one
    /// choice the edged pill, the card's Insert line and the reading all
    /// follow (§18).
    pub(crate) fn flow_wanted_slot(&self) -> Option<usize> {
        self.flow_slot_wanted(self.fi()?)
    }

    fn flow_slot_wanted(&self, i: usize) -> Option<usize> {
        let f = &self.flows[i];
        match f.step {
            Step::Sticks => (0..f.slots.len()).find(|&s| self.flows[i].needs_bind(s)),
            Step::Write => (0..f.slots.len()).find(|&s| f.slots[s].written.is_none()),
            _ => None,
        }
    }

    /// Takes up the stick in the machine, if the flow is asking for one.
    pub(crate) fn flow_examine(&mut self) {
        let Some(i) = self.fi() else {
            return;
        };
        if !self.flow_screen() || self.flows[i].live.reading.is_some() {
            return;
        }
        if !matches!(self.flows[i].step, Step::Sticks | Step::Write) {
            return;
        }
        if !self.clean() {
            return;
        }
        let cands = self.flow_candidates();
        self.flows.all[i].live.crowd = cands.len() > 1;
        let [k] = cands[..] else {
            return;
        };
        let st = self.stick[k].clone();
        if self.flows[i].live.ignored.contains(&st.id) {
            return;
        }
        let Some(slot) = self.flow_slot_wanted(i) else {
            return;
        };
        let purpose =
            if self.flows[i].step == Step::Write && self.flows[i].slots[slot].bound.is_some() {
                Purpose::Check
            } else {
                Purpose::Bind
            };
        self.flow_start_reading(i, slot, purpose, &st, false);
    }

    fn flow_use_present(&mut self, i: usize, k: usize) {
        let Some(st) = self.stick.get(k).cloned() else {
            return;
        };
        let Some(slot) = self.flow_slot_wanted(i) else {
            return;
        };
        if !self.clean() || self.flows[i].live.reading.is_some() {
            return;
        }
        let purpose =
            if self.flows[i].step == Step::Write && self.flows[i].slots[slot].bound.is_some() {
                Purpose::Check
            } else {
                Purpose::Bind
            };
        // Use {label} at step 2 is the press that names the stick: once it
        // is read, it is bound with no second tap (owner, 2026-10-25).
        let chosen = self.flows[i].step == Step::Sticks;
        self.flow_start_reading(i, slot, purpose, &st, chosen);
    }

    fn flow_start_reading(
        &mut self,
        i: usize,
        slot: usize,
        purpose: Purpose,
        st: &StickInfo,
        chosen: bool,
    ) {
        let ofv: Vec<String> = st
            .files
            .iter()
            .filter(|(n, _)| n.to_ascii_lowercase().ends_with(".ofv"))
            .map(|(n, _)| n.clone())
            .collect();
        let mut r = Reading {
            id: st.id.clone(),
            slot,
            purpose,
            label: st.label.clone(),
            serial: st.serial,
            listing: st.files.clone(),
            pending: ofv.clone(),
            space_pending: false,
            free: None,
            files: Vec::new(),
            problem: None,
            picks: Vec::new(),
            bound: false,
            hold: false,
            differs: None,
            declined: Vec::new(),
            boot: st.boot,
            checked: false,
            first: false,
            chosen,
        };
        let found = self
            .stick
            .problems()
            .iter()
            .find(|(id, _)| *id == st.id)
            .map(|(_, ro)| *ro);
        if st.label.to_ascii_uppercase().starts_with("FARADAYBOOT") {
            r.problem = Some(Problem::Boot);
            r.pending.clear();
        } else if let Some(ro) = found {
            r.problem = Some(if ro {
                Problem::ReadOnly
            } else {
                Problem::NotFat
            });
            r.pending.clear();
        } else {
            r.space_pending = true;
            self.out.storage_out.push_back(StorageCommand::StickSpace {
                stick: st.id.clone(),
            });
            for name in ofv {
                self.out.storage_out.push_back(StorageCommand::Read {
                    stick: st.id.clone(),
                    name,
                });
            }
        }
        self.flows.all[i].live.reading = Some(r);
        self.flow_reading_settled();
    }

    pub(crate) fn flow_wants_read(&self, stick: &str, name: &str) -> bool {
        self.flow()
            .and_then(|f| f.live.reading.as_ref())
            .is_some_and(|r| r.id == stick && r.pending.iter().any(|n| n == name))
    }

    pub(crate) fn flow_read_answer(&mut self, name: &str, bytes: Option<Vec<u8>>) {
        let c = self.compute();
        let Some(i) = self.fi() else {
            return;
        };
        let Some(r) = self.flows.all[i].live.reading.as_mut() else {
            return;
        };
        r.pending.retain(|n| n != name);
        if let Some(bytes) = bytes
            && let Ok(h) = fv::read_header(&bytes)
        {
            r.files.push((
                VaultRead {
                    name: name.to_string(),
                    len: bytes.len(),
                    sha: sha256_of(&c, &bytes),
                    salt: h.salt,
                },
                bytes,
            ));
        }
        self.flow_reading_settled();
    }

    pub(crate) fn flow_wants_space(&self, stick: &str) -> bool {
        self.flow()
            .and_then(|f| f.live.reading.as_ref())
            .is_some_and(|r| r.id == stick && r.space_pending)
    }

    pub(crate) fn flow_space_answer(&mut self, free: u64, size: u64) {
        let Some(i) = self.fi() else {
            return;
        };
        if let Some(r) = self.flows.all[i].live.reading.as_mut() {
            r.space_pending = false;
            // A folder standing for a stick says 0 and 0: no limit known.
            r.free = (size > 0).then_some(free);
        }
        self.flow_reading_settled();
    }

    /// The room on the stick being read could not be asked for: it cannot be
    /// used, and the card offers Use another stick.
    pub(crate) fn flow_space_failed(&mut self, stick: &str) {
        let Some(i) = self.fi() else {
            return;
        };
        if let Some(r) = self.flows.all[i].live.reading.as_mut()
            && r.id == stick
            && r.space_pending
        {
            r.problem = Some(Problem::Unreadable);
            r.pending.clear();
            r.space_pending = false;
        }
    }

    /// The shell says a stick cannot be used: not FAT32, or read-only. It is
    /// kept for the sticks read after it and applied to the one being read.
    pub(crate) fn flow_stick_problem(&mut self, stick: &str, read_only: bool) {
        self.stick.note_problem(stick, read_only);
        let Some(i) = self.fi() else {
            return;
        };
        if let Some(r) = self.flows.all[i].live.reading.as_mut()
            && r.id == stick
        {
            r.problem = Some(if read_only {
                Problem::ReadOnly
            } else {
                Problem::NotFat
            });
            r.pending.clear();
            r.space_pending = false;
        }
    }

    /// The reads are all in: what the stick makes of the step.
    fn flow_reading_settled(&mut self) {
        let Some(i) = self.fi() else {
            return;
        };
        let Some(r) = self.flows[i].live.reading.as_ref() else {
            return;
        };
        if r.problem.is_some() || !r.pending.is_empty() || r.space_pending || r.checked {
            return;
        }
        match r.purpose {
            Purpose::Bind => self.flow_after_read(i),
            Purpose::Check => self.flow_after_check(i),
        }
    }

    /// Step 2: a stick read. Refused when it reads like a stick bound to
    /// another place and holds something to tell it by; the vault
    /// questions follow.
    fn flow_after_read(&mut self, i: usize) {
        let f = &self.flows[i];
        let Some(r) = f.live.reading.as_ref() else {
            return;
        };
        for (k, s) in f.slots.iter().enumerate() {
            if k == r.slot {
                continue;
            }
            if let Some(b) = &s.bound
                && b.label == r.label
                && b.serial == r.serial
                && b.listing == r.listing
                && !r.listing.is_empty()
            {
                let t = s.target;
                if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                    r.problem = Some(Problem::BoundTo(t));
                }
                return;
            }
        }
        // A stick put in at step 6 that is one this flow wrote is refused
        // as it arrives.
        if self.flows[i].step == Step::Write
            && let Some(r) = self.flows[i].live.reading.clone()
            && let Some(p) = self.flows[i].holds_written(&r)
        {
            return self.flows.all[i].refuse(p);
        }
        self.flow_decide(i);
    }

    /// Settles each vault the stick keeps that is not decided: by what the
    /// plan already knows of it, or by a question.
    fn flow_decide(&mut self, i: usize) {
        let c = self.compute();
        let slot = {
            let Some(r) = self.flows[i].live.reading.as_ref() else {
                return;
            };
            // A differing copy is asked about before anything else.
            if r.differs.is_some() {
                return;
            }
            r.slot
        };
        let vs = self.flows[i].vaults_of(slot);
        for v in vs {
            let decided = self.flows[i]
                .live
                .reading
                .as_ref()
                .is_some_and(|r| r.picks.iter().any(|(x, _)| *x == v));
            if decided {
                continue;
            }
            let Some(vp) = self.flows[i].plan.vaults.get(v).cloned() else {
                continue;
            };
            let files: Vec<VaultRead> = self.flows[i]
                .live
                .reading
                .as_ref()
                .map(|r| r.files.iter().map(|(vr, _)| vr.clone()).collect())
                .unwrap_or_default();
            if let Some(salt) = vp.salt {
                // The vault is known: its copy on this stick, if it has
                // one, is the one to write over; the other vaults stay.
                let same: Vec<&VaultRead> = files.iter().filter(|f| f.salt == salt).collect();
                let pick = match same.as_slice() {
                    [] => Pick::New,
                    [one] => Pick::Over(one.name.clone()),
                    [a, b, ..] => {
                        if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                            r.problem = Some(Problem::TwoCopies(v, a.name.clone(), b.name.clone()));
                        }
                        return;
                    }
                };
                if let Pick::Over(name) = &pick
                    && let Some(theirs) = files.iter().find(|f| &f.name == name)
                    && self.flow_files_differ(&c, &theirs.salt, &theirs.sha)
                {
                    let f = &self.flows[i];
                    if f.step == Step::Write {
                        // The key is made and the vaults sealed: no copy is
                        // chosen to unlock now. A copy that is not what the
                        // flow wrote or read is not written over without
                        // the hold (§4.6).
                        let own = f.sealed_of(v).is_some_and(|s| s.sha == theirs.sha);
                        if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                            if !own && !r.declined.contains(&v) {
                                r.declined.push(v);
                            }
                            r.picks.push((v, pick));
                        }
                        continue;
                    }
                    // Same vault, other bytes: the person picks the copy to
                    // unlock (§4.2).
                    if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                        r.differs = Some((v, name.clone()));
                    }
                    return;
                }
                if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                    r.picks.push((v, pick));
                }
                continue;
            }
            // Not known yet.
            if files.is_empty() {
                if vp.existing {
                    if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                        r.problem = Some(Problem::NoVault(v));
                    }
                    return;
                }
                if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                    r.picks.push((v, Pick::New));
                }
                continue;
            }
            // A question: the card shows the files.
            return;
        }
        self.flow_picks_done(i);
    }

    /// Costs a hash of a whole file (through `hash`). Whether Files holds a
    /// copy of the vault of this salt whose bytes are not these (§4.2's
    /// copy that differs).
    fn flow_files_differ(&self, c: &Compute, salt: &[u8; 32], sha: &[u8; 32]) -> bool {
        self.inbox.iter().chain(self.outbox.iter()).any(|it| {
            it.kind == FileKind::Vault
                && fv::read_header(&it.bytes).is_ok_and(|h| &h.salt == salt)
                && &it.hash(c) != sha
        })
    }

    /// Costs a hash of a whole file (through `hash`). How the copy of the
    /// vault in Files is called for the question: the stick it came from,
    /// else its file name.
    pub(crate) fn flow_files_copy_label(&self, c: &Compute, salt: &[u8; 32]) -> String {
        let Some(it) = self.inbox.iter().chain(self.outbox.iter()).find(|it| {
            it.kind == FileKind::Vault && fv::read_header(&it.bytes).is_ok_and(|h| &h.salt == salt)
        }) else {
            return String::new();
        };
        let hash = it.hash(c);
        self.visit
            .vault_from_find(|(n, h, _)| n == &it.name || *h == hash)
            .map_or_else(|| it.name.clone(), |(_, _, label)| label.clone())
    }

    /// Which copy of a vault that differs from the one in Files to unlock:
    /// this stick's, which replaces the one in Files, or the one in Files,
    /// in which case this stick's copy is written over only after a hold.
    fn flow_use_copy(&mut self, i: usize, this: bool) {
        let c = self.compute();
        // The copy to unlock is chosen at step 2 and step 3 only.
        if self.flows[i].step == Step::Write {
            return;
        }
        let Some((v, name)) = self.flows.all[i]
            .live
            .reading
            .as_mut()
            .and_then(|r| r.differs.take())
        else {
            return;
        };
        if this {
            let found = self.flows[i].live.reading.as_ref().and_then(|r| {
                r.files
                    .iter()
                    .find(|(f, _)| f.name == name)
                    .map(|(_, b)| (b.clone(), r.label.clone()))
            });
            if let Some((bytes, label)) = found {
                // The sticks already bound with another copy of this vault
                // are declined: each is written over only after a hold.
                if let Ok(h) = fv::read_header(&bytes) {
                    let sha = sha256_of(&c, &bytes);
                    for s in &mut self.flows.all[i].slots {
                        if let Some(b) = s.bound.as_mut()
                            && b.vaults.iter().any(|r| r.salt == h.salt && r.sha != sha)
                            && !b.declined.contains(&v)
                        {
                            b.declined.push(v);
                        }
                    }
                }
                self.flow_take_copy(i, v, &name, bytes, &label);
            }
        }
        if let Some(r) = self.flows.all[i].live.reading.as_mut() {
            r.picks.retain(|(x, _)| *x != v);
            r.picks.push((v, Pick::Over(name)));
            if !this {
                r.declined.push(v);
            }
        }
        self.flow_decide(i);
    }

    fn flow_pick_file(&mut self, i: usize, v: usize, k: usize) {
        let c = self.compute();
        let Some(r) = self.flows[i].live.reading.as_ref() else {
            return;
        };
        let Some((vr, _)) = r.files.get(k) else {
            return;
        };
        let (name, salt, sha) = (vr.name.clone(), vr.salt, vr.sha);
        // Two files of one vault: that stick cannot serve that vault.
        if let Some((other, _)) = r
            .files
            .iter()
            .find(|(o, _)| o.salt == salt && o.name != name)
        {
            let b = other.name.clone();
            if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                r.problem = Some(Problem::TwoCopies(v, name, b));
            }
            return;
        }
        // The plan's vault is that existing one.
        if let Some(vp) = self.flows.all[i].plan.vaults.get_mut(v) {
            vp.existing = true;
        }
        // The same vault in Files with other bytes: which copy is asked.
        if self.flow_files_differ(&c, &salt, &sha) {
            if let Some(r) = self.flows.all[i].live.reading.as_mut() {
                r.picks.retain(|(x, _)| *x != v);
                r.differs = Some((v, name));
            }
            return;
        }
        if let Some(r) = self.flows.all[i].live.reading.as_mut() {
            r.picks.retain(|(x, _)| *x != v);
            r.picks.push((v, Pick::Over(name)));
        }
        self.flow_decide(i);
    }

    fn flow_pick_beside(&mut self, i: usize, v: usize) {
        if let Some(r) = self.flows.all[i].live.reading.as_mut() {
            r.picks.retain(|(x, _)| *x != v);
            r.picks.push((v, Pick::New));
        }
        self.flows.all[i].plan.vaults[v].existing = false;
        self.flow_decide(i);
    }

    /// Every vault the stick keeps is decided: its room, then its place.
    fn flow_picks_done(&mut self, i: usize) {
        let needed = self.flow_room_needed(i);
        if let Some(r) = self.flows.all[i].live.reading.as_mut()
            && let Some(free) = r.free
            && needed > free
        {
            r.problem = Some(Problem::NoRoom(needed, free));
        }
        // A stick named by its own press is bound now, where nothing is
        // left to tell apart: a tap is asked only where it reads like a
        // stick bound to another place.
        let chosen = self.flows[i].live.reading.as_ref().is_some_and(|r| {
            r.chosen
                && r.purpose == Purpose::Bind
                && r.problem.is_none()
                && r.differs.is_none()
                && !r.bound
        });
        if chosen && self.flows[i].step == Step::Sticks && self.flow_confirm_kind() == Confirm::Tap
        {
            self.flow_confirm(i, false);
        }
    }

    /// The room the vaults the stick keeps take: the vault's length, twice
    /// where a file of the same vault is there to be replaced
    /// (`docs/VAULT.md` §6), and the other files known at that place.
    fn flow_room_needed(&self, i: usize) -> u64 {
        let f = &self.flows[i];
        let Some(r) = f.live.reading.as_ref() else {
            return 0;
        };
        let mut need = 0u64;
        for (v, pick) in &r.picks {
            let len = f.plan.vaults.get(*v).map_or(0, |vp| vp.len as u64);
            let len = if len == 0 {
                r.files
                    .iter()
                    .find(|(vr, _)| matches!(pick, Pick::Over(n) if *n == vr.name))
                    .map_or_else(new_vault_len, |(vr, _)| vr.len as u64)
            } else {
                len
            };
            need += if matches!(pick, Pick::Over(_)) {
                2 * len
            } else {
                len
            };
        }
        let s = &f.slots[r.slot];
        for name in &s.files {
            if let Some(it) = self
                .outbox
                .iter()
                .chain(self.inbox.iter())
                .find(|it| &it.name == name)
            {
                need += it.bytes.len() as u64;
            }
        }
        need
    }

    /// What confirming the stick's place takes: a tap, or a hold when it
    /// reads like a stick bound to another place and has nothing to tell
    /// it by.
    pub(crate) fn flow_confirm_kind(&self) -> Confirm {
        let Some(f) = self.flow() else {
            return Confirm::Tap;
        };
        let Some(r) = f.live.reading.as_ref() else {
            return Confirm::Tap;
        };
        let alike = f.slots.iter().enumerate().any(|(k, s)| {
            k != r.slot
                && s.bound.as_ref().is_some_and(|b| {
                    b.label == r.label && b.serial == r.serial && b.listing == r.listing
                })
        });
        if alike { Confirm::Hold } else { Confirm::Tap }
    }

    /// Tap or hold: the stick is for its place. Only a process that is
    /// clean reads or writes a stick.
    fn flow_confirm(&mut self, i: usize, hold: bool) {
        if !self.clean() {
            return;
        }
        let Some(r) = self.flows[i].live.reading.clone() else {
            return;
        };
        if r.problem.is_some() || r.bound || r.differs.is_some() {
            return;
        }
        let at_write = self.flows[i].step == Step::Write;
        match r.purpose {
            Purpose::Bind => {
                let kind_ = self.flow_confirm_kind();
                if (kind_ == Confirm::Hold) != hold {
                    return;
                }
                let vs = self.flows[i].vaults_of(r.slot);
                if vs.iter().any(|v| !r.picks.iter().any(|(x, _)| x == v)) {
                    return;
                }
                self.flow_bind(i, kind_);
                if at_write {
                    self.flow_write_check_bound(i);
                }
            }
            Purpose::Check => {
                // The second stick's hold.
                if !at_write || !r.hold || !hold {
                    return;
                }
                self.flow_write_go(i);
            }
        }
    }

    /// The stick is the one for its place: what it holds of the vaults is
    /// taken into Files, and what was read is kept.
    fn flow_bind(&mut self, i: usize, confirm: Confirm) {
        let Some(r) = self.flows[i].live.reading.clone() else {
            return;
        };
        let now = self.clock_now();
        let slot = r.slot;
        let place = self.flows[i]
            .slots
            .get(slot)
            .and_then(|s| s.target.place())
            .unwrap_or(0);
        for (v, pick) in &r.picks {
            if let Pick::Over(name) = pick
                && self.flows[i]
                    .plan
                    .vaults
                    .get(*v)
                    .is_some_and(|vp| vp.salt.is_none())
                && let Some((vr, bytes)) = r.files.iter().find(|(f, _)| &f.name == name)
            {
                let item_name = self.flow_take_copy(i, *v, name, bytes.clone(), &r.label);
                let vp = &mut self.flows.all[i].plan.vaults[*v];
                vp.existing = true;
                vp.file = Some(item_name);
                vp.salt = Some(vr.salt);
                vp.len = vr.len;
                vp.from = Some((place, name.clone()));
            }
        }
        let bound = Bound {
            label: r.label.clone(),
            serial: r.serial,
            listing: r.listing.clone(),
            vaults: r.files.iter().map(|(f, _)| f.clone()).collect(),
            free: r.free,
            at: now,
            confirm,
            picks: r.picks.clone(),
            declined: r.declined.clone(),
            boot: r.boot,
        };
        let at_write = self.flows[i].step == Step::Write;
        let f = &mut self.flows.all[i];
        if let Some(s) = f.slots.get_mut(slot) {
            s.bound = Some(bound);
        }
        // The medium the machine started from is never pulled: its reading
        // ends at the bind, or goes on to the write.
        if r.boot && !at_write {
            f.live.reading = None;
        } else if let Some(r) = f.live.reading.as_mut() {
            // The boot stick is Stick 1 already: nothing to mark (§18).
            r.first = !r.boot;
            // At step 6 the stick goes on to be checked and written.
            if at_write {
                r.purpose = Purpose::Check;
            } else {
                r.bound = true;
            }
        }
        self.flow_settle();
    }

    /// Takes a vault file from a stick into Files, as Files holds one
    /// copied in today. Returns the name it has there.
    fn flow_take_copy(
        &mut self,
        _i: usize,
        _v: usize,
        name: &str,
        bytes: Vec<u8>,
        label: &str,
    ) -> String {
        let c = self.compute();
        let salt = fv::read_header(&bytes).map(|h| h.salt).ok();
        // A different vault already under that name keeps its name.
        let clash = self.inbox.iter().any(|it| {
            it.name == name
                && !(it.kind == FileKind::Vault
                    && fv::read_header(&it.bytes).ok().map(|h| h.salt) == salt)
        });
        let own = if clash {
            self.free_vault_name(&crate::stem(name))
        } else {
            name.to_string()
        };
        self.vault_read_from(&own, &bytes, label);
        self.inbox.retain(|it| {
            it.name != own
                && !(it.kind == FileKind::Vault
                    && fv::read_header(&it.bytes).ok().map(|h| h.salt) == salt)
        });
        self.inbox.push(Item::new(&c, &own, bytes));
        self.save_boxes();
        own
    }

    // ---- the stick arriving and going ----

    /// A stick arrived while the flow is on screen: the flow's, never the
    /// visit's (§5). Returns whether the flow took it.
    pub(crate) fn flow_stick_arrived(&mut self) -> bool {
        if !self.flow_screen() {
            return false;
        }
        let Some(i) = self.fi() else {
            return false;
        };
        match self.flows[i].step {
            Step::Vaults | Step::Make => {
                // Unlock and Make wait for the stick to be pulled.
                if self.sheet.is_none() {
                    *self.visit.pull_mut() = None;
                    self.sheet = Some(Sheet::Pull);
                }
            }
            Step::Sticks | Step::Write => self.flow_examine(),
            _ => {}
        }
        true
    }

    /// The set of sticks changed while a flow is kept.
    pub(crate) fn flow_sticks_changed(&mut self) {
        let ids: Vec<String> = self.stick.iter().map(|s| s.id.clone()).collect();
        for f in &mut self.flows.all {
            f.live.ignored.retain(|id| ids.contains(id));
            let gone = f
                .live
                .reading
                .as_ref()
                .is_some_and(|r| !ids.contains(&r.id));
            if gone {
                let writing = f.live.writing.take();
                if let Some(r) = f.live.reading.take()
                    && writing.is_some()
                    && !r.bound
                {
                    f.live.problem = Some(Problem::Pulled);
                }
                if let Some(w) = writing {
                    let _ = w;
                    f.live.problem = Some(Problem::Pulled);
                }
            }
            // A stick that was refused as unusable is read again with each
            // listing: it may have been handed out since, or have been put
            // right. A problem that is still there is told again.
            if f.live.reading.as_ref().is_some_and(|r| {
                matches!(
                    r.problem,
                    Some(Problem::NotFat | Problem::ReadOnly | Problem::Unreadable)
                ) && !r.bound
            }) {
                f.live.reading = None;
            }
            f.live.crowd = self.stick.iter().filter(|s| !s.boot).count() > 1;
        }
        if self.flow_screen() {
            self.flow_settle();
            self.flow_examine();
            self.flow_settle();
        }
    }

    // ---- step 3: the vaults ----

    fn flow_unlock_start(&mut self, i: usize, v: usize) {
        let Some(salt) = self.flows[i].plan.vaults.get(v).and_then(|vp| vp.salt) else {
            return;
        };
        let Some(at) = self
            .vault_files()
            .iter()
            .position(|f| f.header.salt == salt && f.open.is_none())
        else {
            return;
        };
        let focus = self.may_load_keys().then_some(Focus::Passphrase);
        self.vaults.pick_unlock(at, Some(Screen::BackupFlow), focus);
        self.flows.all[i].live.unlocking = Some(v);
        self.vaults.measure();
    }

    fn flow_create_start(&mut self, i: usize, v: usize) {
        let mut form = crate::vault::CreateForm::default();
        form.name.set(&BackupFlow::vault_name(v));
        let focus = self.may_load_keys().then_some(Focus::Phrase(0, false));
        self.vaults.begin_create(form, Screen::BackupFlow, focus);
        self.flows.all[i].live.creating = Some(v);
        self.vaults.measure();
    }

    /// Create a vault inside the flow made a vault: it is the plan's, and
    /// is unlocked now with the passphrase just typed twice. Returns
    /// whether the flow took it.
    pub(crate) fn flow_vault_made(&mut self, name: &str, passphrase: &str) -> bool {
        let Some(i) = self.fi() else {
            return false;
        };
        let Some(v) = self.flows[i].live.creating else {
            return false;
        };
        let Some(it) = self.outbox.iter().find(|it| it.name == name) else {
            return false;
        };
        let Ok(h) = fv::read_header(&it.bytes) else {
            return false;
        };
        let len = it.bytes.len();
        {
            let vp = &mut self.flows.all[i].plan.vaults[v];
            vp.existing = false;
            vp.file = Some(name.to_string());
            vp.salt = Some(h.salt);
            vp.len = len;
        }
        self.flows.all[i].live.creating = None;
        self.vaults.drop_create();
        self.screen = Screen::BackupFlow;
        let Some(at) = self
            .vault_files()
            .iter()
            .position(|f| f.header.salt == h.salt)
        else {
            return true;
        };
        // Nothing is typed or unlocked with a stick in: the vault is made,
        // and its Unlock waits for the stick to go.
        if !self.may_load_keys() {
            return true;
        }
        self.vaults.unlock_with(at, passphrase, Screen::BackupFlow);
        true
    }

    /// Create a vault was left from the flow.
    pub(crate) fn flow_create_left(&mut self) {
        if let Some(f) = self.flow_mut() {
            f.live.creating = None;
        }
    }

    /// A vault was unlocked: the flow goes on.
    pub(crate) fn flow_vault_opened(&mut self) {
        let Some(i) = self.fi() else {
            return;
        };
        self.flows.all[i].live.unlocking = None;
        if self.flows[i].live.no_pass.is_some() {
            self.flows.all[i].live.no_pass = None;
        }
        // The page the person asked for once the sticks were written.
        if self.flows[i].step == Step::Done && self.flows[i].live.open == Some(u8::MAX) {
            if self.flows[i].kind.is_wallet() {
                self.wallet_flow_land(i);
            } else {
                self.flow_select_subject(i);
            }
            return;
        }
        if self.flows[i].step == Step::Done {
            return;
        }
        if self.screen != Screen::BackupFlow {
            self.screen = Screen::BackupFlow;
        }
        self.flow_settle();
    }

    /// "Use another stick": back to step 2 for that vault's places, which
    /// locks first when a vault is open.
    fn flow_another_stick(&mut self, i: usize, v: usize) {
        let f = &mut self.flows.all[i];
        *f.plan
            .vaults
            .get_mut(v)
            .unwrap_or(&mut VaultPlan::default()) = VaultPlan {
            existing: true,
            ..VaultPlan::default()
        };
        for s in &mut f.slots {
            if let Some(p) = s.target.place()
                && f.plan.places.get(p).is_some_and(|vs| vs.contains(&v))
            {
                s.bound = None;
            }
        }
        f.live.no_pass = None;
        f.live.reading = None;
        f.step = Step::Sticks;
        f.live.open = Some(Step::Sticks.n());
        self.flow_enter();
    }

    // ---- step 4: the key ----

    /// The kind's own form, in the flow's card.
    fn flow_make_form(&mut self, kind_: Kind) {
        let want = kind_.record_kind();
        if self.vaults.form.as_ref().is_some_and(|f| f.kind == want) {
            return;
        }
        let mk = |n: u8, label: &'static str, text: &str| {
            let mut b = TextBox::default();
            b.set(text);
            (n, label, b, false)
        };
        let fields = match kind_ {
            Kind::Gpg => {
                self.vaults.set_gpg_years(crate::gpg::DEFAULT_YEARS);
                vec![
                    mk(1, crate::strings::EN.vaults_name, ""),
                    mk(2, crate::strings::EN.vaults_email, ""),
                ]
            }
            Kind::SecureBoot => vec![mk(1, crate::strings::EN.vaults_owner_name, "")],
            // A wallet's keys are made in Make a new wallet.
            Kind::Wallet => Vec::new(),
        };
        self.vaults.open_form(Form {
            kind: want,
            record: None,
            fields,
        });
    }

    fn flow_make_ask(&mut self, i: usize) {
        let (kind_, made) = {
            let f = &self.flows[i];
            (f.kind, f.made)
        };
        // In the demo no GPG or Secure Boot key is made, however
        // Make is reached (`docs/DEMO.md` §4).
        if made || kind_.is_wallet() || !self.makes_secrets() {
            return;
        }
        let ready = self
            .flow_needed_vaults(i)
            .iter()
            .all(|&v| self.flow_open_index(i, v).is_some());
        if !ready {
            return self.toast(FEN.backup_flow_unlock_first);
        }
        let Some(form) = self.vaults.form.as_ref() else {
            return;
        };
        let texts: Vec<String> = form.fields.iter().map(|f| f.2.text.to_string()).collect();
        // The time first, as every key that keeps it asks.
        match kind_ {
            Kind::Gpg => {
                if texts[0].trim().is_empty() && texts[1].trim().is_empty() {
                    return self.toast(FEN.gpg_a_key_needs_a_name);
                }
            }
            Kind::SecureBoot | Kind::Wallet => {}
        }
        if self.time_asked() {
            return;
        }
        if self.clock_now().is_none() {
            return self.toast(FEN.secureboot_this_computer);
        }
        // An RSA key takes a while: "Making the key" is drawn first. A GPG
        // key is made at once.
        if kind_ == Kind::Gpg {
            return self.flow_make_work();
        }
        self.flows.all[i].live.making = true;
        self.vaults.start_work(Work::FlowMake);
    }

    /// Makes the key once and saves it into every vault of the plan, in
    /// one action (§4.4).
    pub(crate) fn flow_make_work(&mut self) {
        let c = self.compute();
        let Some(i) = self.fi() else {
            return;
        };
        self.flows.all[i].live.making = false;
        if !self.may_load_keys() {
            return self.toast(FEN.backup_flow_pull_to_unlock_toast);
        }
        let kind_ = self.flows[i].kind;
        if kind_.is_wallet() {
            return;
        }
        let texts: Vec<String> = self
            .vaults
            .form
            .as_ref()
            .map(|f| f.fields.iter().map(|x| x.2.text.to_string()).collect())
            .unwrap_or_default();
        if texts.is_empty() {
            return;
        }
        let built = match kind_ {
            Kind::Gpg => self.gpg_build(&texts[0], &texts[1]),
            Kind::SecureBoot => self.sb_build(&texts[0]),
            Kind::Wallet => return,
        };
        let built = match built {
            Ok(b) => b,
            Err(e) => return self.toast(&e),
        };
        let targets: Vec<usize> = self.flows[i].plan.used_vaults();
        let opens: Vec<usize> = targets
            .iter()
            .filter_map(|&v| self.flow_open_index(i, v))
            .collect();
        // Every vault takes it, or none does.
        for &o in &opens {
            let vid = self.vaults.open[o].id;
            if !self.vaults.store().fits(vid, &[], &[built.record.size()]) {
                return self.toast(FEN.vaults_slot_too_small);
            }
        }
        for &o in &opens {
            let vid = self.vaults.open[o].id;
            self.vaults.edit(vid, |s| s.insert(vid, &built.record));
            self.vaults.changed(vid);
        }
        // The public files made with the key go For the stick, and are
        // written in the flow to the stick of the place the plan names.
        let mut public: Vec<String> = Vec::new();
        for (name, bytes) in built.outbox {
            let sha = sha256_of(&c, &bytes);
            if kind_ == Kind::Gpg {
                self.gpg_put_public(&name, bytes);
            } else {
                self.put_outbox(&name, bytes);
            }
            if self
                .outbox
                .iter()
                .any(|it| it.name == name && sha256_of(&c, &it.bytes) == sha)
            {
                public.push(name);
            }
        }
        let f = &mut self.flows.all[i];
        if f.kind != Kind::SecureBoot
            && let Some(k) = f.public_slot()
        {
            for n in &public {
                if !f.slots[k].files.contains(n) {
                    f.slots[k].files.push(n.clone());
                }
            }
        }
        f.public = public;
        f.made = true;
        f.made_in = targets;
        f.subject = Some(Subject {
            id: hex_text(&built.subject),
            name: built.name,
        });
        self.vaults.close_form();
        // The result's Show as QR and PNG point at the key just made.
        self.flow_point_at_subject(i);
        if kind_ == Kind::Gpg {
            self.toast_out(FEN.vaults_made_the_certificate);
        }
        self.out.dirty = true;
    }

    /// The extra round: the key is copied from a vault that holds it into
    /// the vault added after it (§4.8).
    fn flow_copy_key(&mut self, i: usize) {
        if self.flows[i].kind.is_wallet() {
            return self.flow_wallet_copy(i);
        }
        let (kind_, id) = {
            let f = &self.flows[i];
            let Some(s) = f.subject.as_ref() else {
                return;
            };
            (f.kind, s.id.clone())
        };
        let used = self.flows[i].plan.used_vaults();
        let holder = used
            .iter()
            .filter(|v| self.flows[i].made_in.contains(v))
            .filter_map(|&v| self.flow_open_index(i, v))
            .find_map(|o| super::record_in(&self.vaults, o, kind_, &id).map(|at| (o, at)));
        let Some((holder_open, holder_at)) = holder else {
            return self.toast(FEN.backup_flow_unlock_first);
        };
        let from = self.vaults.open[holder_open].id;
        let Some(size) = self.vaults.records(from).get(holder_at).map(|r| r.size()) else {
            return self.toast(FEN.backup_flow_unlock_first);
        };
        let added: Vec<usize> = used
            .iter()
            .copied()
            .filter(|v| !self.flows[i].made_in.contains(v))
            .collect();
        for &v in &added {
            let Some(o) = self.flow_open_index(i, v) else {
                return self.toast(FEN.backup_flow_unlock_first);
            };
            if !self
                .vaults
                .store()
                .fits(self.vaults.open[o].id, &[], &[size])
            {
                return self.toast(FEN.vaults_slot_too_small);
            }
        }
        for &v in &added {
            if let Some(o) = self.flow_open_index(i, v) {
                let to = self.vaults.open[o].id;
                self.vaults
                    .edit(to, |s| s.copy_record(from, holder_at, to, None));
                self.vaults.changed(to);
            }
        }
        let f = &mut self.flows.all[i];
        f.made_in.extend(added);
        f.adding = false;
    }

    // ---- step 5 and the lock ----

    fn flow_lock_write(&mut self, i: usize) {
        if self.flows[i].step != Step::Lock {
            return;
        }
        self.idle.set_lock_for_flow(true);
        self.lock();
    }

    /// A lock begins: the flows keep where they are, resuming on the
    /// card their step leaves them at (§9). A key made resumes on the
    /// writes unless the round for an added vault is under way, which
    /// resumes on its vaults; Set the time over the flow's Make counts as
    /// the flow.
    pub(crate) fn flow_before_lock(&mut self) {
        self.flow_keep_makers();
        let on = self.screen == Screen::BackupFlow
            || (self.screen == Screen::SetTime && self.set_time.back_to() == Screen::BackupFlow);
        let cur = self.fi();
        for (k, f) in self.flows.all.iter_mut().enumerate() {
            f.on_screen = on && Some(k) == cur;
            let was_write = f.step == Step::Write;
            f.step = match f.step {
                Step::Lock if f.adding => Step::Vaults,
                Step::Lock => Step::Write,
                Step::Make if f.made && !f.adding => Step::Write,
                Step::Make => Step::Vaults,
                s => s,
            };
            // A plan changed after the sealing, with no Continue: a vault of
            // it has no sealed copy. The key goes back to the round that
            // makes one, so nothing invalid is kept.
            if f.made
                && was_write
                && !f
                    .plan
                    .used_vaults()
                    .into_iter()
                    .all(|v| f.sealed_of(v).is_some())
            {
                f.step = Step::Vaults;
                f.adding = true;
            }
            if f.step == Step::Write && f.slots.is_empty() {
                f.step = Step::Done;
            }
        }
    }

    /// A sealing of an open vault was made: the flows whose key it holds.
    /// A vault that does not hold the key is not recorded: a copy of it is
    /// not a backup of the key.
    pub(crate) fn flow_sealed(&mut self, o: usize, name: &str, bytes: &[u8]) {
        let c = self.compute();
        let Some(salt) = self.vaults.open.get(o).map(|v| v.header().salt) else {
            return;
        };
        let sha = sha256_of(&c, bytes);
        let holds: Vec<bool> = self
            .flows
            .iter()
            .map(|f| {
                // A wallet's vaults hold what its plan puts in them: the
                // checklist saw to it before the lock.
                f.kind.is_wallet()
                    || f.subject
                        .as_ref()
                        .is_some_and(|s| super::record_in(&self.vaults, o, f.kind, &s.id).is_some())
            })
            .collect();
        for (f, holds) in self.flows.all.iter_mut().zip(holds) {
            if !f.made || f.step == Step::Done || !holds {
                continue;
            }
            for v in f.plan.used_vaults() {
                if f.plan.vaults[v].salt == Some(salt) && f.made_in.contains(&v) {
                    f.plan.vaults[v].len = bytes.len();
                    if !f.sealing.iter().any(|s| s.vault == v && s.sha == sha) {
                        f.sealing.push(Sealed {
                            vault: v,
                            name: name.to_string(),
                            len: bytes.len(),
                            sha,
                        });
                    }
                }
            }
        }
    }

    // ---- step 6: writing ----

    /// Step 6: a stick bound at step 2 is in. A stick that is another one
    /// written for this flow is refused first; then it must read as it did,
    /// the flow's own sealed bytes and part file aside (a write that was
    /// pulled half way, or whose answer was lost, leaves them); a stick that
    /// already holds the whole of the slot's own copy is counted.
    fn flow_after_check(&mut self, i: usize) {
        let Some(r) = self.flows[i].live.reading.clone() else {
            return;
        };
        if let Some(p) = self.flows[i].holds_written(&r) {
            return self.flows.all[i].refuse(p);
        }
        let f = &self.flows[i];
        let Some(slot) = f.slots.get(r.slot) else {
            return;
        };
        let Some(b) = &slot.bound else {
            return;
        };
        let own: Vec<[u8; 32]> = f
            .vaults_of(r.slot)
            .iter()
            .filter_map(|&v| f.sealed_of(v).map(|s| s.sha))
            .collect();
        let over: Vec<&str> = b
            .picks
            .iter()
            .filter_map(|(_, p)| match p {
                Pick::Over(n) => Some(n.as_str()),
                Pick::New => None,
            })
            .collect();
        // A file this write replaces: the old bytes, the new, or none yet.
        let over_ok = over
            .iter()
            .all(|n| match r.files.iter().find(|(vr, _)| vr.name == *n) {
                None => true,
                Some((vr, _)) => {
                    own.contains(&vr.sha)
                        || b.vaults.iter().any(|w| w.name == *n && w.sha == vr.sha)
                }
            });
        let part = |n: &str| n.starts_with('.') && n.ends_with(".part");
        let own_name = |n: &str| {
            slot.files.iter().any(|x| {
                x == n
                    || n.strip_prefix(x.rsplit_once('.').map_or(x.as_str(), |(s, _)| s))
                        .is_some_and(|t| t.starts_with('-'))
            })
        };
        let skip = |n: &str| over.contains(&n) || part(n);
        let own_files: Vec<&str> = r
            .files
            .iter()
            .filter(|(vr, _)| own.contains(&vr.sha))
            .map(|(vr, _)| vr.name.as_str())
            .collect();
        let mut want: Vec<VaultRead> = b
            .vaults
            .iter()
            .filter(|v| !skip(&v.name))
            .cloned()
            .collect();
        let mut got: Vec<VaultRead> = r
            .files
            .iter()
            .map(|(v, _)| v)
            .filter(|v| !own.contains(&v.sha) && !skip(&v.name))
            .cloned()
            .collect();
        want.sort_by(|a, b| a.name.cmp(&b.name));
        got.sort_by(|a, b| a.name.cmp(&b.name));
        let mut l1: Vec<(String, u64)> = b
            .listing
            .iter()
            .filter(|(n, _)| !skip(n))
            .cloned()
            .collect();
        let mut l2: Vec<(String, u64)> = r
            .listing
            .iter()
            .filter(|(n, _)| !skip(n) && !own_files.contains(&n.as_str()) && !own_name(n))
            .cloned()
            .collect();
        l1.sort();
        l2.sort();
        if !over_ok || b.label != r.label || b.serial != r.serial || l1 != l2 || want != got {
            let t = slot.target;
            return self.flows.all[i].refuse(Problem::NotRead(t));
        }
        // The write landed and its answer was lost: the flow's own sealed
        // bytes are all there.
        let vs = f.vaults_of(r.slot);
        let landed: Vec<(String, [u8; 32])> = vs
            .iter()
            .filter_map(|&v| f.sealed_of(v))
            .filter_map(|s| {
                r.files
                    .iter()
                    .find(|(vr, _)| vr.sha == s.sha)
                    .map(|(vr, _)| (vr.name.clone(), s.sha))
            })
            .collect();
        if slot.files.is_empty() && !vs.is_empty() && landed.len() == vs.len() {
            let (id, label, serial) = (r.id.clone(), r.label.clone(), r.serial);
            self.flow_written_done(i, r.slot, id, label, serial, landed);
            return;
        }
        self.flow_write_check_copies(i);
    }

    /// The stick named after step 2 is bound (tapped): its copies.
    fn flow_write_check_bound(&mut self, i: usize) {
        self.flow_write_check_copies(i);
    }

    fn flow_write_check_copies(&mut self, i: usize) {
        let Some(r) = self.flows[i].live.reading.clone() else {
            return;
        };
        if let Some(p) = self.flows[i].holds_written(&r) {
            return self.flows.all[i].refuse(p);
        }
        let f = &self.flows[i];
        // A later copy, or a copy of a vault the person declined in favour of
        // another, is written only after a hold.
        let declined = f
            .slots
            .get(r.slot)
            .and_then(|s| s.bound.as_ref())
            .is_some_and(|b| !b.declined.is_empty());
        let later = declined
            || f.vaults_of(r.slot)
                .iter()
                .any(|&v| f.copy_number(v, r.slot) > 0);
        // Room for what is to be written.
        let need = self.flow_write_bytes(i, r.slot);
        let free = r.free;
        if let Some(r2) = self.flows.all[i].live.reading.as_mut() {
            r2.checked = true;
            r2.hold = later;
            if let Some(free) = free
                && need > free
            {
                r2.problem = Some(Problem::NoRoom(need, free));
                r2.checked = false;
            }
        }
        if !later {
            self.flow_write_go(i);
        }
    }

    /// The bytes step 6 writes to the stick of `slot`, counting a file
    /// replaced twice.
    fn flow_write_bytes(&self, i: usize, slot: usize) -> u64 {
        let f = &self.flows[i];
        let mut need = 0u64;
        for v in f.vaults_of(slot) {
            if let Some(s) = f.sealed_of(v) {
                let over = f.slots[slot]
                    .bound
                    .as_ref()
                    .and_then(|b| b.picks.iter().find(|(x, _)| *x == v))
                    .is_some_and(|(_, p)| matches!(p, Pick::Over(_)));
                need += s.len as u64 * if over { 2 } else { 1 };
            }
        }
        let mut names = f.slots[slot].files.clone();
        names.extend(f.last_write_files(slot));
        for name in &names {
            if let Some(it) = self.outbox.iter().find(|it| &it.name == name) {
                need += it.bytes.len() as u64;
            }
        }
        need
    }

    /// Writes the stick of the slot being read. Only a clean process at step
    /// 6 writes; every vault of the slot must have its sealing and its
    /// Outbox copy, and nothing secret goes to a stick.
    fn flow_write_go(&mut self, i: usize) {
        let c = self.compute();
        if !self.clean() || self.flows[i].step != Step::Write {
            return;
        }
        let Some(r) = self.flows[i].live.reading.clone() else {
            return;
        };
        let f = &self.flows[i];
        let slot = r.slot;
        let mut sends: Vec<(String, Vec<u8>, bool)> = Vec::new();
        let mut refused = false;
        for v in f.vaults_of(slot) {
            // The Outbox, or Files once the flow was done: found by name and
            // SHA-256, so a copy deleted later can be written again.
            let item = f.sealed_of(v).and_then(|s| {
                self.outbox
                    .iter()
                    .chain(self.inbox.iter())
                    .find(|it| it.name == s.name && sha256_of(&c, &it.bytes) == s.sha)
            });
            match item {
                Some(it) if it.exposure() != crate::secrets::Exposure::Secret => {
                    sends.push((it.name.clone(), it.bytes.clone(), true));
                }
                _ => refused = true,
            }
        }
        let mut names: Vec<String> = f
            .slots
            .get(slot)
            .map(|s| s.files.clone())
            .unwrap_or_default();
        names.extend(f.last_write_files(slot));
        // The seeds' files a wallet's plan keeps, let out unprotected,
        // go to the stick the plan keeps them on and to no other: the stick
        // of files, or a stick at a place, beside its vault or not
        // (`docs/BACKUP-FLOW.md` §17.4).
        let files_stick = f
            .slots
            .get(slot)
            .is_some_and(|s| s.target != Target::Public);
        for name in names {
            match self
                .outbox
                .iter()
                .chain(self.inbox.iter())
                .find(|it| it.name == name)
            {
                Some(it) if it.exposure() != crate::secrets::Exposure::Secret => {
                    sends.push((it.name.clone(), it.bytes.clone(), false));
                }
                Some(it)
                    if files_stick
                        && f.public_slot() != Some(slot)
                        && it.secret
                        && f.kind.is_wallet()
                        && f.seed_file_ok_for(&c, slot, &it.name) =>
                {
                    sends.push((it.name.clone(), it.bytes.clone(), false));
                }
                _ => refused = true,
            }
        }
        if refused || sends.is_empty() {
            return self.flows.all[i].refuse(Problem::NotChecked);
        }
        let pending: Vec<(String, [u8; 32])> = sends
            .iter()
            .map(|(n, b, _)| (n.clone(), sha256_of(&c, b)))
            .collect();
        let plain: Vec<String> = sends
            .iter()
            .filter(|(_, _, vault)| !vault)
            .map(|(n, _, _)| n.clone())
            .collect();
        for (name, bytes, vault) in sends {
            // A vault goes over the copy of its salt and nothing else, or
            // beside what the stick holds; any other file beside under a
            // free name (`faraday-files`).
            self.out.storage_out.push_back(StorageCommand::Write {
                stick: r.id.clone(),
                name,
                bytes,
                replace: vault,
                renamed: false,
            });
        }
        let f = &mut self.flows.all[i];
        if let Some(r) = f.live.reading.as_mut() {
            r.hold = false;
        }
        let target = f.slots[slot].target;
        f.live.writing = Some(Writing {
            id: r.id,
            slot,
            target,
            pending,
            done: Vec::new(),
            plain,
            retried: Vec::new(),
            aliases: Vec::new(),
            qr: Vec::new(),
            label: r.label,
            serial: r.serial,
        });
    }

    pub(crate) fn flow_wants_written(&self, stick: &str, name: &str) -> bool {
        self.flow()
            .and_then(|f| f.live.writing.as_ref())
            .is_some_and(|w| w.id == stick && w.pending.iter().any(|(n, _)| n == name))
    }

    /// A write was answered; `Ok` with the name it was written as, or the
    /// reason it was not. A picture written is read back for its codes
    /// (`docs/BACKUP-FLOW.md` §17.12 item 10); a file that did not read
    /// back is written again once, and a second time refuses the stick.
    pub(crate) fn flow_write_answer(&mut self, name: &str, answer: Result<String, String>) {
        let Some(i) = self.fi() else {
            return;
        };
        let Some(w) = self.flows.all[i].live.writing.as_mut() else {
            return;
        };
        let Some(k) = w.pending.iter().position(|(n, _)| n == name) else {
            return;
        };
        let (_, sha) = w.pending.remove(k);
        let meant = w
            .aliases
            .iter()
            .find(|(sent, _)| sent == name)
            .map_or_else(|| name.to_string(), |(_, m)| m.clone());
        let plain = w.plain.contains(&meant);
        let stick = w.id.clone();
        let picture = self.flows[i]
            .wallet
            .as_ref()
            .is_some_and(|wf| wf.readback.iter().any(|(n, _)| *n == meant));
        match answer {
            Ok(wrote_as) if picture => {
                if let Some(w) = self.flows.all[i].live.writing.as_mut() {
                    w.qr.push((wrote_as.clone(), meant, sha));
                }
                self.out.storage_out.push_back(StorageCommand::ReadQr {
                    stick,
                    name: wrote_as,
                });
            }
            Ok(wrote_as) => {
                if let Some(w) = self.flows.all[i].live.writing.as_mut() {
                    w.done.push((wrote_as, sha));
                }
            }
            Err(_) if plain => return self.flow_write_again(i, &meant, name, false),
            Err(_) => {
                self.flows.all[i].live.writing = None;
                self.flows.all[i].refuse(Problem::NotChecked);
                return;
            }
        }
        self.flow_write_settle(i);
    }

    /// Whether the flow on screen waits for picture `name`'s codes, read
    /// back after its write.
    pub(crate) fn flow_wants_qr(&self, name: &str) -> bool {
        self.flow()
            .and_then(|f| f.live.writing.as_ref())
            .is_some_and(|w| w.qr.iter().any(|(n, _, _)| n == name))
    }

    /// A picture read back from the stick it was just written to (its bytes
    /// and its codes), or `None` where it could not be read: it counts when
    /// its codes hold what it was made to hold, else it is written again
    /// once. A public picture is checked against the SHA-256 kept at the
    /// lock; a SeedQR against the Outbox's picture byte for byte and, while
    /// its key is loaded, against the digits of the key's words, which are
    /// written out for the comparison and dropped.
    pub(crate) fn flow_qr_answer(&mut self, name: &str, read: Option<(Vec<u8>, Vec<Vec<u8>>)>) {
        let c = self.compute();
        use crate::backup_flow::ReadBack;
        let Some(i) = self.fi() else {
            return;
        };
        let Some(w) = self.flows.all[i].live.writing.as_mut() else {
            return;
        };
        let Some(k) = w.qr.iter().position(|(n, _, _)| n == name) else {
            return;
        };
        let (wrote_as, meant, sha) = w.qr.remove(k);
        let want = self.flows[i]
            .wallet
            .as_ref()
            .and_then(|wf| wf.readback.iter().find(|(n, _)| *n == meant))
            .map(|(_, b)| b.clone());
        let good = match (want, read) {
            (Some(ReadBack::Codes(h)), Some((_, codes))) => sha256_of(&c, &codes.concat()) == h,
            (Some(ReadBack::Seed(fp)), Some((bytes, codes))) => {
                self.secret_picture_reads(&c, &meant, (&bytes, &codes), &fp, None)
            }
            (Some(ReadBack::Codex32(fp, j)), Some((bytes, codes))) => {
                self.secret_picture_reads(&c, &meant, (&bytes, &codes), &fp, Some(j))
            }
            _ => false,
        };
        if good {
            if let Some(w) = self.flows.all[i].live.writing.as_mut() {
                w.done.push((wrote_as, sha));
            }
            return self.flow_write_settle(i);
        }
        self.flow_write_again(i, &meant, &wrote_as, true);
    }

    /// File `meant` (sent as `sent`) did not read back: written again once,
    /// over what was written when `over`; a second time, the stick's write
    /// does not count and the card says so.
    fn flow_write_again(&mut self, i: usize, meant: &str, sent: &str, over: bool) {
        let c = self.compute();
        let Some(w) = self.flows[i].live.writing.as_ref() else {
            return;
        };
        let item = self
            .outbox
            .iter()
            .chain(self.inbox.iter())
            .find(|it| it.name == meant)
            .map(|it| it.bytes.clone());
        let (Some(bytes), false) = (item, w.retried.iter().any(|n| n == meant)) else {
            self.flows.all[i].live.writing = None;
            return self.flows.all[i].refuse(Problem::NoReadBack(meant.to_string()));
        };
        let stick = w.id.clone();
        let sha = sha256_of(&c, &bytes);
        if let Some(w) = self.flows.all[i].live.writing.as_mut() {
            w.retried.push(meant.to_string());
            w.pending.push((sent.to_string(), sha));
            if sent != meant {
                w.aliases.push((sent.to_string(), meant.to_string()));
            }
        }
        self.out.storage_out.push_back(StorageCommand::Write {
            stick,
            name: sent.to_string(),
            bytes,
            replace: over,
            renamed: false,
        });
    }

    /// Every file of the write under way is answered and read back: the
    /// stick counts.
    fn flow_write_settle(&mut self, i: usize) {
        let ready = self.flows[i]
            .live
            .writing
            .as_ref()
            .is_some_and(|w| w.pending.is_empty() && w.qr.is_empty());
        if ready && let Some(w) = self.flows.all[i].live.writing.take() {
            // The write counts for the stick it was for, by target: the
            // slots may have moved since it started.
            let Some(slot) = self.flows[i]
                .slots
                .iter()
                .position(|s| s.target == w.target)
            else {
                return;
            };
            self.flow_written_done(i, slot, w.id, w.label, w.serial, w.done);
        }
    }

    /// A stick's files are all written, read back and matched (or are found
    /// on it as the flow wrote them): it counts.
    fn flow_written_done(
        &mut self,
        i: usize,
        slot: usize,
        id: String,
        label: String,
        serial: Option<u32>,
        done: Vec<(String, [u8; 32])>,
    ) {
        let c = self.compute();
        let time = self.clock_now();
        let done: Vec<(String, [u8; 32])> = done
            .into_iter()
            .map(|(n, h)| (n, self.hash_to_keep(&c, h)))
            .collect();
        let Some(s) = self.flows.all[i].slots.get_mut(slot) else {
            return;
        };
        s.written = Some(Written {
            stick: id.clone(),
            label: label.clone(),
            serial,
            time,
            files: done.clone(),
        });
        *self.visit.receipt_mut() = Some(crate::Receipt {
            by_flow: true,
            label,
            stick: id,
            time,
            files: done
                .iter()
                .map(|(n, h)| crate::ReceiptFile {
                    name: n.clone(),
                    hash: *h,
                    verified: true,
                    failure: None,
                })
                .collect(),
        });
        self.save_boxes();
        // The medium the machine started from is never pulled.
        let f = &mut self.flows.all[i];
        if f.live.reading.as_ref().is_some_and(|r| r.boot) {
            f.live.reading = None;
            self.flow_settle();
        } else if let Some(r) = f.live.reading.as_mut() {
            r.bound = true;
        }
    }

    // ---- done ----

    /// Every stick is written: the sealed vaults leave For the stick and
    /// are in Files as the sticks now hold them (the visit does the same).
    fn flow_finish(&mut self, i: usize) {
        let c = self.compute();
        let used = self.flows[i].plan.used_vaults();
        let seals: Vec<Sealed> = self.flows[i]
            .sealing
            .iter()
            .filter(|s| used.contains(&s.vault))
            .cloned()
            .collect();
        for s in seals {
            if let Some(k) = self
                .outbox
                .iter()
                .position(|it| it.name == s.name && sha256_of(&c, &it.bytes) == s.sha)
            {
                let item = self.outbox.remove(k);
                if let Ok(h) = fv::read_header(&item.bytes) {
                    self.inbox.retain(|it| {
                        it.name != item.name
                            && !(it.kind == FileKind::Vault
                                && fv::read_header(&it.bytes).is_ok_and(|o| o.salt == h.salt))
                    });
                }
                self.inbox
                    .push(Item::new(&c, &item.name, item.bytes.clone()));
            }
        }
        // Nothing the flow made waits For the stick: the public files it
        // wrote move to Files, where a copy deleted later is written again
        // from.
        let names: Vec<String> = self.flows[i]
            .slots
            .iter()
            .filter(|s| s.written.is_some())
            .flat_map(|s| s.files.iter().cloned())
            .chain(self.flows[i].public.iter().cloned())
            .collect();
        let moved: Vec<Item> = self
            .outbox
            .iter()
            .filter(|it| names.contains(&it.name))
            .cloned()
            .collect();
        self.outbox.retain(|it| !names.contains(&it.name));
        for it in moved {
            self.inbox.retain(|x| x.name != it.name);
            self.inbox.push(it);
        }
        // The enrolment files of a Secure Boot key are written with the
        // last stick, so a stick written again later can carry them again.
        if self.flows[i].kind != Kind::SecureBoot {
            self.flows.all[i].public.clear();
        }
        self.flows.all[i].live.reading = None;
        self.save_boxes();
    }

    /// Unlock a vault of the plan from the Done card; lands on the key's
    /// page.
    fn flow_unlock_done(&mut self, i: usize, v: usize) {
        let Some(salt) = self.flows[i].plan.vaults.get(v).and_then(|vp| vp.salt) else {
            return;
        };
        let kind_ = self.flows[i].kind;
        let Some(at) = self
            .vault_files()
            .iter()
            .position(|f| f.header.salt == salt && f.open.is_none())
        else {
            // Open already: its key's page, or the wallet's checklist.
            if kind_.is_wallet() {
                return self.wallet_flow_land(i);
            }
            return self.flow_select_subject(i);
        };
        self.vaults.choose_category(kind_.category());
        self.flows.all[i].live.open = Some(u8::MAX);
        self.act(Action::Vault(crate::vault::VaultAction::OpenFrom(
            at,
            Screen::VaultContents,
        )));
    }

    /// Points Vault contents at the key the flow made, wherever it is
    /// open, without leaving the flow. Returns whether it was found.
    pub(crate) fn flow_point_at_subject(&mut self, i: usize) -> bool {
        let (kind_, id) = {
            let f = &self.flows[i];
            let Some(s) = f.subject.as_ref() else {
                return false;
            };
            (f.kind, s.id.clone())
        };
        let cat = kind_.category();
        let kinds = crate::vault::CATEGORIES[cat].1;
        let found = self.vaults.open.iter().enumerate().find_map(|(o, open)| {
            let mut n = 0;
            for (_, r) in self.vaults.records(open.id).iter() {
                if !kinds.contains(&r.kind()) {
                    continue;
                }
                if r.kind() == kind_.record_kind()
                    && subject_of(kind_, r).as_deref() == Some(id.as_str())
                {
                    return Some((o, n));
                }
                n += 1;
            }
            None
        });
        let Some((o, n)) = found else {
            return false;
        };
        self.vaults.point_at(o, cat, n);
        true
    }

    /// The key's page in Vault contents.
    pub(crate) fn flow_select_subject(&mut self, i: usize) {
        if !self.flow_point_at_subject(i) {
            return;
        }
        self.vaults.open_item();
        self.flows.all[i].live.open = Some(Step::Done.n());
        self.vaults.set_item_offset(0.0);
        self.screen = Screen::VaultContents;
    }

    /// A waiting action, **Continue the backup**: the flow of the key
    /// shown opens on its step.
    pub(crate) fn flow_resume_selected(&mut self) {
        let Some((kind_, id)) = self.flow_selected_subject() else {
            return;
        };
        if let Some(i) = self
            .flows
            .iter()
            .position(|f| f.kind == kind_ && f.subject.as_ref().is_some_and(|s| s.id == id))
        {
            self.flow_show(i);
        }
    }

    /// The kind and public value of the key selected in Vault contents.
    pub(crate) fn flow_selected_subject(&self) -> Option<(Kind, String)> {
        let at = self.vault_selected_index()?;
        let r = self
            .vaults
            .records(self.vaults.open.get(self.vaults.current)?.id)
            .get(at)?;
        let kind_ = match r.kind() {
            kind::GPG => Kind::Gpg,
            kind::SECURE_BOOT => Kind::SecureBoot,
            _ => return None,
        };
        Some((kind_, subject_of(kind_, r)?))
    }

    /// Whether the key selected waits for its backup: its written count
    /// and the number of places.
    pub(crate) fn selected_waits(&self) -> Option<(usize, usize)> {
        let (k, id) = self.flow_selected_subject()?;
        self.flows
            .iter()
            .find(|f| f.kind == k && f.subject.as_ref().is_some_and(|s| s.id == id))
            .filter(|f| f.waiting())
            .map(|f| f.written_of())
    }

    /// A stick's file was deleted or the stick was wiped: if it is one the
    /// flow wrote, its card opens again (§9). The stick is known by its
    /// label and serial and the files written to it; one that cannot be
    /// found among the sticks in the machine, or that reads otherwise,
    /// changes nothing. `name` is the file deleted, or `None` for a wipe.
    pub(crate) fn flow_copy_gone(&mut self, stick: &str, name: Option<&str>) {
        let Some((label, serial)) = self
            .stick
            .iter()
            .find(|s| s.id == stick)
            .map(|s| (s.label.clone(), s.serial))
        else {
            return;
        };
        self.flow_copy_gone_for(stick, &label, serial, name);
    }

    /// [`Faraday::flow_copy_gone`] for a stick known by its label and
    /// serial, which may be out of the machine by now.
    pub(crate) fn flow_copy_gone_for(
        &mut self,
        stick: &str,
        label: &str,
        serial: Option<u32>,
        name: Option<&str>,
    ) {
        let mut reset = false;
        for f in &mut self.flows.all {
            let hits: Vec<usize> = (0..f.slots.len())
                .filter(|&k| {
                    f.slots[k].written.as_ref().is_some_and(|w| {
                        w.label == label
                            && w.serial == serial
                            && w.files
                                .iter()
                                .any(|(n, _)| name.is_none_or(|x| x.eq_ignore_ascii_case(n)))
                    })
                })
                .collect();
            // Where the stick written is the one in the machine, only it.
            let exact: Vec<usize> = hits
                .iter()
                .copied()
                .filter(|&k| {
                    f.slots[k]
                        .written
                        .as_ref()
                        .is_some_and(|w| w.stick == stick)
                })
                .collect();
            let hits = if exact.is_empty() { hits } else { exact };
            for k in hits {
                f.slots[k].written = None;
                reset = true;
                if f.step == Step::Done {
                    f.step = Step::Write;
                    f.live.open = Some(Step::Write.n());
                }
            }
        }
        if reset {
            let text = FEN.backup_flow_first_gone_toast;
            self.toast(text);
        }
    }

    /// The text of Problem `p`.
    pub fn flow_problem_text(&self, p: &Problem) -> String {
        let m = self.medium.noun();
        let f = self.flow();
        let target = |t: &Target| -> String { f.map_or_else(String::new, |f| f.target_name(t, m)) };
        match p {
            Problem::Boot => fill(FEN.backup_flow_boot_part, &[m]),
            Problem::NotFat => fill(FEN.backup_flow_not_fat, &[m, m]),
            Problem::Unreadable => fill(FEN.backup_flow_unreadable, &[m, m]),
            Problem::ReadOnly => fill(FEN.backup_flow_read_only, &[m]),
            Problem::NoRoom(need, free) => fill(
                FEN.backup_flow_no_room,
                &[
                    &crate::vault::file_text(*need as usize),
                    &crate::vault::file_text(*free as usize),
                ],
            ),
            Problem::NoVault(v) => fill(FEN.backup_flow_no_vault, &[&BackupFlow::vault_name(*v)]),
            Problem::TwoCopies(v, a, b) => {
                let label = f
                    .and_then(|f| f.live.reading.as_ref())
                    .map(|r| r.label.clone())
                    .unwrap_or_default();
                fill(
                    FEN.backup_flow_two_copies,
                    &[&label, &BackupFlow::vault_name(*v), a, b],
                )
            }
            Problem::BoundTo(t) => fill(FEN.backup_flow_bound_place, &[&target(t)]),
            Problem::NotRead(t) => {
                // On a device, by the number of the stick it should be.
                let n = f.and_then(|f| f.slots.iter().position(|s| s.target == *t));
                match (n.and_then(|k| self.flow_slot_name(k)), f) {
                    (Some(name), Some(f)) => {
                        fill(FEN.backup_flow_not_read_n, &[&name, &f.target_what(t, m)])
                    }
                    _ => fill(
                        FEN.backup_flow_not_read,
                        &[m, &f.map_or_else(String::new, |f| f.target_short(t, m))],
                    ),
                }
            }
            Problem::HasFirst => fill(FEN.backup_flow_has_first, &[m]),
            Problem::HasFilesOf(t) => {
                let place = match t.place() {
                    Some(p) => BackupFlow::place_name(p),
                    None => target(t),
                };
                fill(FEN.backup_flow_other_files, &[m, &place])
            }
            Problem::NotChecked => {
                let label = f
                    .and_then(|f| f.live.reading.as_ref())
                    .map(|r| r.label.clone())
                    .unwrap_or_default();
                fill(FEN.backup_flow_not_checked, &[&label])
            }
            Problem::Pulled => FEN.backup_flow_pulled.to_string(),
            Problem::NoReadBack(file) => {
                let label = f
                    .and_then(|f| f.live.reading.as_ref())
                    .map(|r| r.label.clone())
                    .unwrap_or_default();
                fill(FEN.backup_flow_no_read_back, &[file, &label])
            }
        }
    }
}

/// What a kind's own build gives back (§4.4).
pub struct Built {
    /// The record the key is kept as, held: each vault takes a copy.
    pub record: faraday_keys::vault::MadeKey,
    /// The public value it is known by.
    pub subject: Vec<u8>,
    /// The name it is shown under.
    pub name: String,
    /// Files For the stick: the certificate or the public key.
    pub outbox: Vec<(String, Vec<u8>)>,
}

impl Faraday {
    /// The backup flows begun, at most one on screen.
    pub fn backup_flows(&self) -> &[BackupFlow] {
        &self.flows
    }

    /// The writes the key `subject` waits for ([`BackupFlowArea::waits`]).
    pub fn backup_waits(&self, kind: Kind, subject: &[u8]) -> Option<(usize, usize)> {
        self.flows.waits(kind, subject)
    }

    /// The key page's Backup row ([`BackupFlowArea::row`]).
    pub fn backup_row(&self, kind: Kind, subject: &[u8]) -> Option<super::BackupRow> {
        self.flows.row(kind, subject)
    }

    /// The flow Home leads with and the Power sheet lists
    /// ([`BackupFlowArea::pending`]).
    pub fn backup_pending(&self) -> Option<(usize, &BackupFlow)> {
        self.flows.pending()
    }

    /// Copies and public files kept in Files for the flow's writes are
    /// listed on the visit as In the backup
    /// ([`BackupFlowArea::in_the_backup`]).
    pub fn in_the_backup(&self, name: &str) -> bool {
        self.flows.in_the_backup(name)
    }

    /// Reads back what the previous process kept. A flow that was on screen
    /// opens on its step (`docs/BACKUP-FLOW.md` §5).
    pub(crate) fn flow_restore(&mut self, bytes: &[u8]) {
        let c = self.compute();
        let (flows, dropped) = super::decode(&c, bytes);
        self.flows.all = flows;
        // The Make a new wallet or Restore a flow came from is gone with
        // the lock: it keeps an id no screen has.
        for k in 0..self.flows.len() {
            let id = self.fresh_owner();
            if let Some(w) = self.flows.all[k].wallet.as_mut() {
                w.owner = id;
            }
        }
        if dropped > 0 {
            self.toast(crate::strings::EN.backup_flow_dropped);
        }
        self.flow_makers_return();
        let on = self.flows.iter().position(|f| f.on_screen);
        self.flows.on = None;
        if let Some(i) = on {
            self.flow_show(i);
        }
    }

    /// Costs what `BackupFlowArea::gate_restore` costs. Reads the keys
    /// that wait.
    pub(crate) fn flow_gate_restore(&mut self, bytes: &[u8]) {
        let c = self.compute();
        self.flows.gate_restore(&c, bytes);
    }
}

impl Faraday {
    /// The vaults of the flow on screen that must be open now.
    pub(crate) fn flow_needed_list(&self) -> Vec<usize> {
        self.fi()
            .map_or_else(Vec::new, |i| self.flow_needed_vaults(i))
    }

    /// What was seen of the vault with this salt: "Main · seen 14:02".
    pub(crate) fn vault_summary_for_salt(&self, salt: &[u8; 32]) -> Option<String> {
        let s = self.vaults.summaries.iter().find(|s| &s.salt == salt)?;
        Some(match s.seen {
            Some(t) => fill(
                FEN.backup_flow_seen,
                &[
                    &s.name,
                    &crate::vault::time_of_day(crate::vault::local_secs(t, self.tz_min())),
                ],
            ),
            None => s.name.clone(),
        })
    }
}
