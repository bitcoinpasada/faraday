//! What the backup flow does at each step (`docs/BACKUP-FLOW.md` §4):
//! editing the plan, reading a stick with nothing secret open, binding it
//! to its place, unlocking and making the vaults inside the flow, making
//! the key into every vault, and writing each stick by place. The state is
//! `backup_flow.rs`; the cards are `backup_flow_screen.rs`.

use faraday_vault as fv;
use faraday_vault::Record;
use faraday_vault::records::kind;

use crate::backup_flow::{
    BackupFlow, Bound, Confirm, Kind, Pick, Problem, Purpose, Reading, Sealed, Step, Subject,
    Target, VaultPlan, VaultRead, Writing, Written, hex_text, subject_of,
};
use crate::strings::{EN as FEN, fill};
use crate::vaults::{Focus, Form, TextBox, Work};
use crate::{Action, Faraday, FileKind, Item, Screen, Sheet, StickInfo, StorageCommand, sha256_of};

/// Everything a press in the flow can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowAction {
    /// Open a card, or close it.
    Open(u8),
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
        self.flow_on.filter(|&i| i < self.backup_flows.len())
    }

    /// The flow on screen.
    pub fn flow(&self) -> Option<&BackupFlow> {
        self.backup_flows.get(self.fi()?)
    }

    fn flow_mut(&mut self) -> Option<&mut BackupFlow> {
        let i = self.fi()?;
        self.backup_flows.get_mut(i)
    }

    /// The flow is on screen and not left.
    pub(crate) fn flow_screen(&self) -> bool {
        self.screen == Screen::BackupFlow && self.fi().is_some()
    }

    /// Starts the flow for `kind` on its first card, or resumes one
    /// that has not made its key (§5).
    pub(crate) fn flow_begin(&mut self, kind: Kind) {
        if let Some(i) = self.flow_resumable(kind) {
            return self.flow_show(i);
        }
        self.backup_flows.push(BackupFlow::new(kind));
        let i = self.backup_flows.len() - 1;
        self.flow_show(i);
    }

    /// Opens flow `i` on its step.
    pub(crate) fn flow_show(&mut self, i: usize) {
        if i >= self.backup_flows.len() {
            return;
        }
        self.flow_on = Some(i);
        self.screen = Screen::BackupFlow;
        self.osk_leave();
        let f = &mut self.backup_flows[i];
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
            let f = &self.backup_flows[i];
            (f.step, f.kind, f.made)
        };
        match step {
            Step::Vaults => {
                self.vaults.back_to = Some(Screen::BackupFlow);
            }
            Step::Make if !made && !kind.is_wallet() => self.flow_make_form(kind),
            Step::Sticks | Step::Write => self.flow_examine(),
            _ => {}
        }
        self.flow_settle();
        self.dirty = true;
    }

    fn flow_goto(&mut self, i: usize, step: Step) {
        let f = &mut self.backup_flows[i];
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
        let salt = self.backup_flows[i].plan.vaults.get(v)?.salt?;
        self.vaults
            .open
            .iter()
            .position(|o| o.header().salt == salt)
    }

    /// The vaults of flow `i` that must be open for the key to be made or
    /// copied, and those that hold it.
    fn flow_needed_vaults(&self, i: usize) -> Vec<usize> {
        let f = &self.backup_flows[i];
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

    fn flow_needs_bind(&self, i: usize, slot: usize) -> bool {
        let f = &self.backup_flows[i];
        let Some(s) = f.slots.get(slot) else {
            return false;
        };
        if s.bound.is_some() {
            return false;
        }
        match s.target {
            Target::Place(_) | Target::PlaceVault(..) => {
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

    /// Automatic transitions: a step whose work is done goes on.
    pub(crate) fn flow_settle(&mut self) {
        for _ in 0..8 {
            let Some(i) = self.fi() else {
                return;
            };
            let step = self.backup_flows[i].step;
            if step == Step::Vaults && self.backup_flows[i].live.creating.is_none() {
                self.flow_vaults_auto(i);
            }
            let next = match step {
                Step::Sticks => {
                    let f = &self.backup_flows[i];
                    let unbound = (0..f.slots.len()).any(|s| self.flow_needs_bind(i, s));
                    (!unbound && f.live.reading.is_none()).then_some(Step::Vaults)
                }
                Step::Vaults => {
                    let ready = self
                        .flow_needed_vaults(i)
                        .iter()
                        .all(|&v| self.flow_open_index(i, v).is_some());
                    ready.then_some(Step::Make)
                }
                Step::Write if self.backup_flows[i].all_written() => Some(Step::Done),
                // Coming back with a vault open again: the flow is on Lock,
                // which seals it (§5). Written sticks stay written.
                Step::Write if !self.clean_now() && self.backup_flows[i].live.writing.is_none() => {
                    Some(Step::Lock)
                }
                _ => None,
            };
            match next {
                Some(s) => {
                    self.flow_goto(i, s);
                    // A wallet's keys are made in Make a new wallet, not
                    // in a card of the flow: steps 1 to 3 done, it goes
                    // back there.
                    let f = &self.backup_flows[i];
                    if s == Step::Make && f.kind.is_wallet() && !f.made && self.fi() == Some(i) {
                        self.wallet_flow_leave(i);
                    }
                }
                None => break,
            }
        }
    }

    /// Step 3 asks for each existing vault's passphrase in turn: the first
    /// still locked is picked, and takes the typing.
    fn flow_vaults_auto(&mut self, i: usize) {
        let mut order = self.backup_flows[i].plan.used_vaults();
        order.sort_by_key(|&v| (!self.backup_flows[i].plan.vaults[v].existing, v));
        let need = self.flow_needed_vaults(i);
        let active = order
            .into_iter()
            .filter(|v| need.contains(v))
            .find(|&v| self.flow_open_index(i, v).is_none());
        let Some(v) = active else {
            return;
        };
        if self.backup_flows[i].plan.vaults[v].salt.is_some()
            && self.backup_flows[i].live.unlocking != Some(v)
        {
            self.flow_unlock_start(i, v);
        }
    }

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
        if self.backup_flows[i].live.writing.is_some()
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
            )
        {
            return;
        }
        match a {
            F::Open(n) => {
                let f = &mut self.backup_flows[i];
                f.live.open = if f.live.open == Some(n) {
                    None
                } else {
                    Some(n)
                };
                f.live.scroll.follow = true;
            }
            // Back leaves on any card but while writing, where pulling the
            // stick is the way to stop (§5).
            F::Back if self.backup_flows[i].live.writing.is_some() => {}
            // A wallet's flow left before its keys exist goes back to the
            // Make a new wallet or Restore it came from.
            F::Back if self.wallet_flow_back(i) => {}
            F::Back => {
                self.backup_flows[i].on_screen = false;
                self.flow_on = None;
                self.vaults.back_to = None;
                self.vaults.create = None;
                self.vaults.focus = None;
                self.screen = Screen::Home;
            }
            F::ChangePlan => {
                let f = &mut self.backup_flows[i];
                f.live.open = Some(Step::Where.n());
                f.live.scroll.follow = true;
            }
            F::AddPlace => self.flow_add_place(i),
            F::RemovePlace(p) => self.flow_remove_place(i, p, false),
            F::Destroyed(p) => self.flow_remove_place(i, p, true),
            F::PlaceVault(p, v) => self.flow_place_vault(i, p, v),
            F::AlsoVault(p, v) => self.flow_also_vault(i, p, v),
            F::Together(p) => self.flow_together(i, p),
            F::PublicAt(p) => self.flow_public_at(i, p),
            F::Answer(list, row) => self.flow_wallet_answer(i, list, row),
            F::WalletMake => self.wallet_flow_leave(i),
            F::VaultExisting(v) => {
                let f = &mut self.backup_flows[i];
                if let Some(vp) = f.plan.vaults.get_mut(v)
                    && vp.salt.is_none()
                {
                    vp.existing = !vp.existing;
                }
            }
            F::Continue => self.flow_continue(i),
            F::LockBring => self.act(Action::LockAsk),
            F::UseBoot(k) => self.flow_use_present(i, k),
            F::PickFile(v, k) => self.flow_pick_file(i, v, k),
            F::PickBeside(v) => self.flow_pick_beside(i, v),
            F::UseCopy(this) => self.flow_use_copy(i, this),
            F::Tap => self.flow_confirm(i, false),
            F::Hold => self.flow_confirm(i, true),
            F::UseAnother => self.flow_use_another(i),
            F::NoPass(v) => {
                let f = &mut self.backup_flows[i];
                f.live.no_pass = if f.live.no_pass == Some(v) {
                    None
                } else {
                    Some(v)
                };
            }
            F::BesideIt(v) => self.flow_beside_it(i, v),
            F::AnotherStick(v) => self.flow_another_stick(i, v),
            F::UnlockVault(v) => self.flow_unlock_start(i, v),
            F::CreateVault(v) => self.flow_create_start(i, v),
            F::Make => self.flow_make_ask(i),
            F::MadeContinue => {
                let f = &self.backup_flows[i];
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
        self.dirty = true;
    }

    fn flow_add_place(&mut self, i: usize) {
        if self.backup_flows[i].kind.is_wallet() {
            return self.flow_wallet_add_place(i);
        }
        let f = &mut self.backup_flows[i];
        let v = f.plan.used_vaults().first().copied().unwrap_or(0);
        if f.plan.vaults.is_empty() {
            f.plan.vaults.push(VaultPlan::default());
        }
        f.plan.places.push(vec![v]);
        f.sync_slots();
    }

    /// Takes place `p` out of the plan: before its write at once, after
    /// it only as "I destroyed this copy" (§4.8). The key is never taken
    /// out of the last vault that holds it.
    fn flow_remove_place(&mut self, i: usize, p: usize, destroyed: bool) {
        // A wallet's places are its plan's: Back up's questions take them
        // out.
        if self.backup_flows[i].kind.is_wallet() && !destroyed {
            return;
        }
        let f = &self.backup_flows[i];
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
        let f = &mut self.backup_flows[i];
        f.plan.places.remove(p);
        f.plan.together.retain(|&q| q != p);
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
            let gone = s.target.place() == Some(p);
            if gone {
                orphaned.extend(s.files.iter().cloned());
            }
            !gone
        });
        for s in &mut f.slots {
            s.target = match s.target {
                Target::Place(q) if q > p => Target::Place(q - 1),
                Target::PlaceVault(q, v) if q > p => Target::PlaceVault(q - 1, v),
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
        self.backup_flows[i].sync_slots();
    }

    fn flow_place_vault(&mut self, i: usize, p: usize, v: usize) {
        if self.backup_flows[i].kind.is_wallet() {
            return;
        }
        let f = &self.backup_flows[i];
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
        let f = &mut self.backup_flows[i];
        if v >= f.plan.vaults.len() {
            f.plan.vaults.push(VaultPlan::default());
        }
        f.plan.places[p] = vec![v];
        f.plan.together.retain(|&q| q != p);
        f.live.reading = None;
        self.flow_tidy(i, &used_before);
        self.backup_flows[i].sync_slots();
    }

    /// Place `p` also keeps the stick of vault `v`, or stops (§4.2: a
    /// place may hold a stick for each vault).
    fn flow_also_vault(&mut self, i: usize, p: usize, v: usize) {
        if self.backup_flows[i].kind.is_wallet() {
            return;
        }
        let f = &self.backup_flows[i];
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
        let f = &mut self.backup_flows[i];
        f.plan.places[p] = after.places[p].clone();
        f.live.reading = None;
        self.flow_tidy(i, &used_before);
        self.backup_flows[i].sync_slots();
    }

    /// **One stick for both**: a place that keeps several vaults keeps them
    /// on one stick, or on one each.
    fn flow_together(&mut self, i: usize, p: usize) {
        let f = &mut self.backup_flows[i];
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
        f.sync_slots();
    }

    /// The stick of place `p` carries the key's public files.
    fn flow_public_at(&mut self, i: usize, p: usize) {
        if self.backup_flows[i].kind.is_wallet() {
            return;
        }
        let f = &mut self.backup_flows[i];
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

    /// Drops the vaults no place keeps any more. An open vault that holds
    /// only this flow's key (and its label and the signed-amount memory) is
    /// dropped with it; any other loses the key and stays. A vault that is
    /// locked is left as it is: its file stays in Files and the plan lets go
    /// of it. The key is never taken out of the last vault that holds it.
    fn flow_tidy(&mut self, i: usize, used_before: &[usize]) {
        if self.backup_flows[i].kind.is_wallet() {
            return;
        }
        let (dropped, kind, subject, holders_left) = {
            let f = &self.backup_flows[i];
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
            let vp = self.backup_flows[i].plan.vaults[v].clone();
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
                self.vaults.open.remove(o);
                self.vaults.current = 0;
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
        let f = &mut self.backup_flows[i];
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
            open.contents.records.iter().all(|r| {
                r.kind == kind::SLOT_LABEL
                    || r.kind == kind::AMOUNTS
                    || (r.kind == kind_.record_kind()
                        && subject_of(kind_, r).as_deref() == Some(id))
            })
        })
    }

    fn flow_remove_subject(&mut self, o: usize, kind_: Kind, id: &str) {
        if let Some(open) = self.vaults.open.get_mut(o) {
            let before = open.contents.records.len();
            open.contents.records.retain(|r| {
                !(r.kind == kind_.record_kind() && subject_of(kind_, r).as_deref() == Some(id))
            });
            if open.contents.records.len() != before {
                open.changes += 1;
            }
        }
    }

    /// Continue on the first card, or after a change of plan.
    fn flow_continue(&mut self, i: usize) {
        if self.backup_flows[i].kind.is_wallet() && self.flow_wallet_stickless(i) {
            return;
        }
        let f = &mut self.backup_flows[i];
        f.sync_slots();
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
                f.made_plan_changed(true);
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
        (0..self.sticks.len())
            .filter(|&k| !self.sticks[k].boot)
            .collect()
    }

    fn flow_slot_wanted(&self, i: usize) -> Option<usize> {
        let f = &self.backup_flows[i];
        match f.step {
            Step::Sticks => (0..f.slots.len()).find(|&s| self.flow_needs_bind(i, s)),
            Step::Write => (0..f.slots.len()).find(|&s| f.slots[s].written.is_none()),
            _ => None,
        }
    }

    /// Takes up the stick in the machine, if the flow is asking for one.
    pub(crate) fn flow_examine(&mut self) {
        let Some(i) = self.fi() else {
            return;
        };
        if !self.flow_screen() || self.backup_flows[i].live.reading.is_some() {
            return;
        }
        if !matches!(self.backup_flows[i].step, Step::Sticks | Step::Write) {
            return;
        }
        if !self.clean() {
            return;
        }
        let cands = self.flow_candidates();
        self.backup_flows[i].live.crowd = cands.len() > 1;
        let [k] = cands[..] else {
            return;
        };
        let st = self.sticks[k].clone();
        if self.backup_flows[i].live.ignored.contains(&st.id) {
            return;
        }
        let Some(slot) = self.flow_slot_wanted(i) else {
            return;
        };
        let purpose = if self.backup_flows[i].step == Step::Write
            && self.backup_flows[i].slots[slot].bound.is_some()
        {
            Purpose::Check
        } else {
            Purpose::Bind
        };
        self.flow_start_reading(i, slot, purpose, &st);
    }

    fn flow_use_present(&mut self, i: usize, k: usize) {
        let Some(st) = self.sticks.get(k).cloned() else {
            return;
        };
        let Some(slot) = self.flow_slot_wanted(i) else {
            return;
        };
        if !self.clean() || self.backup_flows[i].live.reading.is_some() {
            return;
        }
        let purpose = if self.backup_flows[i].step == Step::Write
            && self.backup_flows[i].slots[slot].bound.is_some()
        {
            Purpose::Check
        } else {
            Purpose::Bind
        };
        self.flow_start_reading(i, slot, purpose, &st);
    }

    fn flow_start_reading(&mut self, i: usize, slot: usize, purpose: Purpose, st: &StickInfo) {
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
        };
        let found = self
            .stick_problems
            .iter()
            .find(|(id, _)| *id == st.id)
            .map(|(_, ro)| *ro);
        if st.label.to_ascii_uppercase().starts_with("OSKBOOT") {
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
            self.storage_out.push_back(StorageCommand::StickSpace {
                stick: st.id.clone(),
            });
            for name in ofv {
                self.storage_out.push_back(StorageCommand::Read {
                    stick: st.id.clone(),
                    name,
                });
            }
        }
        self.backup_flows[i].live.reading = Some(r);
        self.flow_reading_settled();
    }

    pub(crate) fn flow_wants_read(&self, stick: &str, name: &str) -> bool {
        self.flow()
            .and_then(|f| f.live.reading.as_ref())
            .is_some_and(|r| r.id == stick && r.pending.iter().any(|n| n == name))
    }

    pub(crate) fn flow_read_answer(&mut self, name: &str, bytes: Option<Vec<u8>>) {
        let Some(i) = self.fi() else {
            return;
        };
        let Some(r) = self.backup_flows[i].live.reading.as_mut() else {
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
                    sha: sha256_of(&bytes),
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
        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
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
        if let Some(r) = self.backup_flows[i].live.reading.as_mut()
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
        self.stick_problems.retain(|(id, _)| id != stick);
        self.stick_problems.push((stick.to_string(), read_only));
        let Some(i) = self.fi() else {
            return;
        };
        if let Some(r) = self.backup_flows[i].live.reading.as_mut()
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
        let Some(r) = self.backup_flows[i].live.reading.as_ref() else {
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
        let f = &self.backup_flows[i];
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
                if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                    r.problem = Some(Problem::BoundTo(t));
                }
                return;
            }
        }
        // A stick put in at step 6 that is one this flow wrote is refused
        // as it arrives.
        if self.backup_flows[i].step == Step::Write
            && let Some(r) = self.backup_flows[i].live.reading.clone()
            && let Some(p) = self.flow_holds_written(i, &r)
        {
            return self.flow_refuse(i, p);
        }
        self.flow_decide(i);
    }

    /// Settles each vault the stick keeps that is not decided: by what the
    /// plan already knows of it, or by a question.
    fn flow_decide(&mut self, i: usize) {
        let slot = {
            let Some(r) = self.backup_flows[i].live.reading.as_ref() else {
                return;
            };
            // A differing copy is asked about before anything else.
            if r.differs.is_some() {
                return;
            }
            r.slot
        };
        let vs = self.backup_flows[i].vaults_of(slot);
        for v in vs {
            let decided = self.backup_flows[i]
                .live
                .reading
                .as_ref()
                .is_some_and(|r| r.picks.iter().any(|(x, _)| *x == v));
            if decided {
                continue;
            }
            let Some(vp) = self.backup_flows[i].plan.vaults.get(v).cloned() else {
                continue;
            };
            let files: Vec<VaultRead> = self.backup_flows[i]
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
                        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                            r.problem = Some(Problem::TwoCopies(v, a.name.clone(), b.name.clone()));
                        }
                        return;
                    }
                };
                if let Pick::Over(name) = &pick
                    && let Some(theirs) = files.iter().find(|f| &f.name == name)
                    && self.flow_files_differ(&theirs.salt, &theirs.sha)
                {
                    let f = &self.backup_flows[i];
                    if f.step == Step::Write {
                        // The key is made and the vaults sealed: no copy is
                        // chosen to unlock now. A copy that is not what the
                        // flow wrote or read is not written over without
                        // the hold (§4.6).
                        let own = f.sealed_of(v).is_some_and(|s| s.sha == theirs.sha);
                        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                            if !own && !r.declined.contains(&v) {
                                r.declined.push(v);
                            }
                            r.picks.push((v, pick));
                        }
                        continue;
                    }
                    // Same vault, other bytes: the person picks the copy to
                    // unlock (§4.2).
                    if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                        r.differs = Some((v, name.clone()));
                    }
                    return;
                }
                if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                    r.picks.push((v, pick));
                }
                continue;
            }
            // Not known yet.
            if files.is_empty() {
                if vp.existing {
                    if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                        r.problem = Some(Problem::NoVault(v));
                    }
                    return;
                }
                if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                    r.picks.push((v, Pick::New));
                }
                continue;
            }
            // A question: the card shows the files.
            return;
        }
        self.flow_picks_done(i);
    }

    /// Whether Files holds a copy of the vault of this salt whose bytes are
    /// not these (§4.2's copy that differs).
    fn flow_files_differ(&self, salt: &[u8; 32], sha: &[u8; 32]) -> bool {
        self.inbox.iter().chain(self.outbox.iter()).any(|it| {
            it.kind == FileKind::Vault
                && fv::read_header(&it.bytes).is_ok_and(|h| &h.salt == salt)
                && &sha256_of(&it.bytes) != sha
        })
    }

    /// How the copy of the vault in Files is called for the question:
    /// the stick it came from, else its file name.
    pub(crate) fn flow_files_copy_label(&self, salt: &[u8; 32]) -> String {
        let Some(it) = self.inbox.iter().chain(self.outbox.iter()).find(|it| {
            it.kind == FileKind::Vault && fv::read_header(&it.bytes).is_ok_and(|h| &h.salt == salt)
        }) else {
            return String::new();
        };
        let hash = sha256_of(&it.bytes);
        self.vault_from
            .iter()
            .find(|(n, h, _)| n == &it.name || *h == hash)
            .map_or_else(|| it.name.clone(), |(_, _, label)| label.clone())
    }

    /// Which copy of a vault that differs from the one in Files to unlock:
    /// this stick's, which replaces the one in Files, or the one in Files,
    /// in which case this stick's copy is written over only after a hold.
    fn flow_use_copy(&mut self, i: usize, this: bool) {
        // The copy to unlock is chosen at step 2 and step 3 only.
        if self.backup_flows[i].step == Step::Write {
            return;
        }
        let Some((v, name)) = self.backup_flows[i]
            .live
            .reading
            .as_mut()
            .and_then(|r| r.differs.take())
        else {
            return;
        };
        if this {
            let found = self.backup_flows[i].live.reading.as_ref().and_then(|r| {
                r.files
                    .iter()
                    .find(|(f, _)| f.name == name)
                    .map(|(_, b)| (b.clone(), r.label.clone()))
            });
            if let Some((bytes, label)) = found {
                // The sticks already bound with another copy of this vault
                // are declined: each is written over only after a hold.
                if let Ok(h) = fv::read_header(&bytes) {
                    let sha = sha256_of(&bytes);
                    for s in &mut self.backup_flows[i].slots {
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
        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
            r.picks.retain(|(x, _)| *x != v);
            r.picks.push((v, Pick::Over(name)));
            if !this {
                r.declined.push(v);
            }
        }
        self.flow_decide(i);
    }

    fn flow_pick_file(&mut self, i: usize, v: usize, k: usize) {
        let Some(r) = self.backup_flows[i].live.reading.as_ref() else {
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
            if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                r.problem = Some(Problem::TwoCopies(v, name, b));
            }
            return;
        }
        // The plan's vault is that existing one.
        if let Some(vp) = self.backup_flows[i].plan.vaults.get_mut(v) {
            vp.existing = true;
        }
        // The same vault in Files with other bytes: which copy is asked.
        if self.flow_files_differ(&salt, &sha) {
            if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
                r.picks.retain(|(x, _)| *x != v);
                r.differs = Some((v, name));
            }
            return;
        }
        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
            r.picks.retain(|(x, _)| *x != v);
            r.picks.push((v, Pick::Over(name)));
        }
        self.flow_decide(i);
    }

    fn flow_pick_beside(&mut self, i: usize, v: usize) {
        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
            r.picks.retain(|(x, _)| *x != v);
            r.picks.push((v, Pick::New));
        }
        self.backup_flows[i].plan.vaults[v].existing = false;
        self.flow_decide(i);
    }

    /// Every vault the stick keeps is decided: its room, then its place.
    fn flow_picks_done(&mut self, i: usize) {
        let needed = self.flow_room_needed(i);
        if let Some(r) = self.backup_flows[i].live.reading.as_mut()
            && let Some(free) = r.free
            && needed > free
        {
            r.problem = Some(Problem::NoRoom(needed, free));
        }
    }

    /// The room the vaults the stick keeps take: the vault's length, twice
    /// where a file of the same vault is there to be replaced
    /// (`docs/VAULT.md` §6), and the other files known at that place.
    fn flow_room_needed(&self, i: usize) -> u64 {
        let f = &self.backup_flows[i];
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
        let Some(r) = self.backup_flows[i].live.reading.clone() else {
            return;
        };
        if r.problem.is_some() || r.bound || r.differs.is_some() {
            return;
        }
        let at_write = self.backup_flows[i].step == Step::Write;
        match r.purpose {
            Purpose::Bind => {
                let kind_ = self.flow_confirm_kind();
                if (kind_ == Confirm::Hold) != hold {
                    return;
                }
                let vs = self.backup_flows[i].vaults_of(r.slot);
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
        let Some(r) = self.backup_flows[i].live.reading.clone() else {
            return;
        };
        let now = self.clock_now();
        let slot = r.slot;
        let place = self.backup_flows[i]
            .slots
            .get(slot)
            .and_then(|s| s.target.place())
            .unwrap_or(0);
        for (v, pick) in &r.picks {
            if let Pick::Over(name) = pick
                && self.backup_flows[i]
                    .plan
                    .vaults
                    .get(*v)
                    .is_some_and(|vp| vp.salt.is_none())
                && let Some((vr, bytes)) = r.files.iter().find(|(f, _)| &f.name == name)
            {
                let item_name = self.flow_take_copy(i, *v, name, bytes.clone(), &r.label);
                let vp = &mut self.backup_flows[i].plan.vaults[*v];
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
        };
        let at_write = self.backup_flows[i].step == Step::Write;
        let f = &mut self.backup_flows[i];
        if let Some(s) = f.slots.get_mut(slot) {
            s.bound = Some(bound);
        }
        // The medium the machine started from is never pulled: its reading
        // ends at the bind, or goes on to the write.
        if r.boot && !at_write {
            f.live.reading = None;
        } else if let Some(r) = f.live.reading.as_mut() {
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
        self.inbox.push(Item::new(&own, bytes));
        self.save_boxes();
        own
    }

    /// Leave this stick for another. A slot that is not written loses its
    /// binding, at step 6 too (a stick lost or broken, a read-only one whose
    /// write failed): the next stick takes the path of a stick named after
    /// step 2.
    fn flow_use_another(&mut self, i: usize) {
        let f = &mut self.backup_flows[i];
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
        match self.backup_flows[i].step {
            Step::Vaults | Step::Make => {
                // Unlock and Make wait for the stick to be pulled.
                if self.sheet.is_none() {
                    self.pull = None;
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
        let ids: Vec<String> = self.sticks.iter().map(|s| s.id.clone()).collect();
        for f in &mut self.backup_flows {
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
            f.live.crowd = self.sticks.iter().filter(|s| !s.boot).count() > 1;
        }
        if self.flow_screen() {
            self.flow_settle();
            self.flow_examine();
            self.flow_settle();
        }
    }

    // ---- step 3: the vaults ----

    fn flow_unlock_start(&mut self, i: usize, v: usize) {
        let Some(salt) = self.backup_flows[i]
            .plan
            .vaults
            .get(v)
            .and_then(|vp| vp.salt)
        else {
            return;
        };
        let Some(at) = self
            .vault_files()
            .iter()
            .position(|f| f.header.salt == salt && f.open.is_none())
        else {
            return;
        };
        self.vaults.pick = at;
        self.vaults.unlock_error = None;
        self.vaults.back_to = Some(Screen::BackupFlow);
        self.vaults.focus = self.may_load_keys().then_some(Focus::Passphrase);
        self.backup_flows[i].live.unlocking = Some(v);
        self.vault_measure();
    }

    fn flow_create_start(&mut self, i: usize, v: usize) {
        let mut form = crate::vaults::CreateForm::default();
        form.name.set(&BackupFlow::vault_name(v));
        self.vaults.create = Some(form);
        self.vaults.back_to = Some(Screen::BackupFlow);
        self.vaults.just_made = None;
        self.vaults.focus = self.may_load_keys().then_some(Focus::Phrase(0, false));
        self.backup_flows[i].live.creating = Some(v);
        self.vault_measure();
    }

    /// Create a vault inside the flow made a vault: it is the plan's, and
    /// is unlocked now with the passphrase just typed twice. Returns
    /// whether the flow took it.
    pub(crate) fn flow_vault_made(&mut self, name: &str, passphrase: &str) -> bool {
        let Some(i) = self.fi() else {
            return false;
        };
        let Some(v) = self.backup_flows[i].live.creating else {
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
            let vp = &mut self.backup_flows[i].plan.vaults[v];
            vp.existing = false;
            vp.file = Some(name.to_string());
            vp.salt = Some(h.salt);
            vp.len = len;
        }
        self.backup_flows[i].live.creating = None;
        self.vaults.create = None;
        self.vaults.focus = None;
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
        self.vaults.pick = at;
        self.vaults.passphrase.set(passphrase);
        self.vaults.unlock_error = None;
        self.vaults.back_to = Some(Screen::BackupFlow);
        self.vaults.working = Some(Work::Unlock);
        self.vaults.drawn = false;
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
        self.backup_flows[i].live.unlocking = None;
        if self.backup_flows[i].live.no_pass.is_some() {
            self.backup_flows[i].live.no_pass = None;
        }
        // The page the person asked for once the sticks were written.
        if self.backup_flows[i].step == Step::Done
            && self.backup_flows[i].live.open == Some(u8::MAX)
        {
            if self.backup_flows[i].kind.is_wallet() {
                self.wallet_flow_land(i);
            } else {
                self.flow_select_subject(i);
            }
            return;
        }
        if self.backup_flows[i].step == Step::Done {
            return;
        }
        if self.screen != Screen::BackupFlow {
            self.screen = Screen::BackupFlow;
        }
        self.flow_settle();
    }

    /// "I do not have its passphrase": a new vault beside the one on the
    /// stick; the plan's vault becomes new and is written beside that
    /// file at step 6.
    fn flow_beside_it(&mut self, i: usize, v: usize) {
        let f = &mut self.backup_flows[i];
        let Some(vp) = f.plan.vaults.get_mut(v) else {
            return;
        };
        // The file stays in Files; it is no longer the plan's vault.
        *vp = VaultPlan::default();
        f.live.no_pass = None;
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

    /// "Use another stick": back to step 2 for that vault's places, which
    /// locks first when a vault is open.
    fn flow_another_stick(&mut self, i: usize, v: usize) {
        let f = &mut self.backup_flows[i];
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
            Kind::Apk => vec![
                mk(
                    1,
                    crate::strings::EN.apk_name_on_certificate,
                    faraday_apk::key::DEFAULT_NAME,
                ),
                mk(
                    2,
                    crate::strings::EN.apk_package,
                    faraday_apk::key::DEFAULT_PACKAGE,
                ),
            ],
            Kind::Gpg => {
                self.vaults.gpg_years = crate::gpg::DEFAULT_YEARS;
                vec![
                    mk(1, crate::strings::EN.vaults_name, ""),
                    mk(2, crate::strings::EN.vaults_email, ""),
                ]
            }
            Kind::SecureBoot => vec![mk(1, crate::strings::EN.vaults_owner_name, "")],
            // A wallet's keys are made in Make a new wallet.
            Kind::Wallet => Vec::new(),
        };
        self.vaults.form = Some(Form {
            kind: want,
            record: None,
            fields,
        });
        self.vaults.saving = false;
        self.vaults.focus = Some(Focus::Field(0));
    }

    fn flow_make_ask(&mut self, i: usize) {
        let (kind_, made) = {
            let f = &self.backup_flows[i];
            (f.kind, f.made)
        };
        if made || kind_.is_wallet() {
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
            Kind::Apk => {
                if let Err(e) = Self::apk_check_form(&texts[0], &texts[1]) {
                    return self.toast(e);
                }
            }
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
        self.backup_flows[i].live.making = true;
        self.vaults.working = Some(Work::FlowMake);
        self.vaults.drawn = false;
    }

    /// Makes the key once and saves it into every vault of the plan, in
    /// one action (§4.4).
    pub(crate) fn flow_make_work(&mut self) {
        let Some(i) = self.fi() else {
            return;
        };
        self.backup_flows[i].live.making = false;
        if !self.may_load_keys() {
            return self.toast(FEN.backup_flow_pull_to_unlock_toast);
        }
        let kind_ = self.backup_flows[i].kind;
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
            Kind::Apk => self.apk_build(&texts[0], &texts[1]),
            Kind::Gpg => self.gpg_build(&texts[0], &texts[1]),
            Kind::SecureBoot => self.sb_build(&texts[0]),
            Kind::Wallet => return,
        };
        let built = match built {
            Ok(b) => b,
            Err(e) => return self.toast(&e),
        };
        let targets: Vec<usize> = self.backup_flows[i].plan.used_vaults();
        let opens: Vec<usize> = targets
            .iter()
            .filter_map(|&v| self.flow_open_index(i, v))
            .collect();
        // Every vault takes it, or none does.
        for &o in &opens {
            let open = &self.vaults.open[o];
            let mut trial = open.contents.clone();
            trial.records.push(built.record.clone());
            if trial.used() > open.header().slot_len as usize {
                return self.toast(FEN.vaults_that_does_not);
            }
        }
        for &o in &opens {
            let open = &mut self.vaults.open[o];
            open.contents.records.push(built.record.clone());
            open.changes += 1;
        }
        // The public files made with the key go For the stick, and are
        // written in the flow to the stick of the place the plan names.
        let mut public: Vec<String> = Vec::new();
        for (name, bytes) in built.outbox {
            let sha = sha256_of(&bytes);
            if kind_ == Kind::Gpg {
                self.gpg_put_public(&name, bytes);
            } else {
                self.put_outbox(&name, bytes);
            }
            if self
                .outbox
                .iter()
                .any(|it| it.name == name && sha256_of(&it.bytes) == sha)
            {
                public.push(name);
            }
        }
        let f = &mut self.backup_flows[i];
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
        self.vaults.form = None;
        self.vaults.focus = None;
        // The result's Show as QR and PNG point at the key just made.
        self.flow_point_at_subject(i);
        if kind_ == Kind::Gpg {
            self.toast_out(FEN.vaults_made_the_certificate);
        }
        self.dirty = true;
    }

    /// The extra round: the key is copied from a vault that holds it into
    /// the vault added after it (§4.8).
    fn flow_copy_key(&mut self, i: usize) {
        if self.backup_flows[i].kind.is_wallet() {
            return self.flow_wallet_copy(i);
        }
        let (kind_, id) = {
            let f = &self.backup_flows[i];
            let Some(s) = f.subject.as_ref() else {
                return;
            };
            (f.kind, s.id.clone())
        };
        let used = self.backup_flows[i].plan.used_vaults();
        let holder = used
            .iter()
            .filter(|v| self.backup_flows[i].made_in.contains(v))
            .filter_map(|&v| self.flow_open_index(i, v))
            .find_map(|o| self.flow_record_in(o, kind_, &id));
        let Some(record) = holder else {
            return self.toast(FEN.backup_flow_unlock_first);
        };
        let added: Vec<usize> = used
            .iter()
            .copied()
            .filter(|v| !self.backup_flows[i].made_in.contains(v))
            .collect();
        for &v in &added {
            let Some(o) = self.flow_open_index(i, v) else {
                return self.toast(FEN.backup_flow_unlock_first);
            };
            let open = &self.vaults.open[o];
            let mut trial = open.contents.clone();
            trial.records.push(record.clone());
            if trial.used() > open.header().slot_len as usize {
                return self.toast(FEN.vaults_that_does_not);
            }
        }
        for &v in &added {
            if let Some(o) = self.flow_open_index(i, v) {
                let open = &mut self.vaults.open[o];
                open.contents.records.push(record.clone());
                open.changes += 1;
            }
        }
        let f = &mut self.backup_flows[i];
        f.made_in.extend(added);
        f.adding = false;
    }

    // ---- step 5 and the lock ----

    fn flow_lock_write(&mut self, i: usize) {
        if self.backup_flows[i].step != Step::Lock {
            return;
        }
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
            || (self.screen == Screen::SetTime && self.set_time.back_to == Screen::BackupFlow);
        let cur = self.fi();
        for (k, f) in self.backup_flows.iter_mut().enumerate() {
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
        let Some(salt) = self.vaults.open.get(o).map(|v| v.header().salt) else {
            return;
        };
        let sha = sha256_of(bytes);
        let holds: Vec<bool> = self
            .backup_flows
            .iter()
            .map(|f| {
                // A wallet's vaults hold what its plan puts in them: the
                // checklist saw to it before the lock.
                f.kind.is_wallet()
                    || f.subject
                        .as_ref()
                        .is_some_and(|s| self.flow_record_in(o, f.kind, &s.id).is_some())
            })
            .collect();
        for (f, holds) in self.backup_flows.iter_mut().zip(holds) {
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
        let Some(r) = self.backup_flows[i].live.reading.clone() else {
            return;
        };
        if let Some(p) = self.flow_holds_written(i, &r) {
            return self.flow_refuse(i, p);
        }
        let f = &self.backup_flows[i];
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
            return self.flow_refuse(i, Problem::NotRead(t));
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

    /// Whether the stick read is one this flow wrote: it holds a file the
    /// flow wrote for another stick (by SHA-256) and reads as that stick
    /// (its label and serial): the first copy of the same vault, or another
    /// place's files. A stick that reads otherwise is compared with what
    /// was read for its own place.
    fn flow_holds_written(&self, i: usize, r: &Reading) -> Option<Problem> {
        let f = &self.backup_flows[i];
        for (k, s) in f.slots.iter().enumerate() {
            if k == r.slot {
                continue;
            }
            let Some(w) = &s.written else {
                continue;
            };
            let holds = r
                .files
                .iter()
                .any(|(vr, _)| w.files.iter().any(|(_, sha)| *sha == vr.sha));
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

    /// The stick named after step 2 is bound (tapped): its copies.
    fn flow_write_check_bound(&mut self, i: usize) {
        self.flow_write_check_copies(i);
    }

    fn flow_refuse(&mut self, i: usize, p: Problem) {
        if let Some(r) = self.backup_flows[i].live.reading.as_mut() {
            r.problem = Some(p);
        }
    }

    fn flow_write_check_copies(&mut self, i: usize) {
        let Some(r) = self.backup_flows[i].live.reading.clone() else {
            return;
        };
        if let Some(p) = self.flow_holds_written(i, &r) {
            return self.flow_refuse(i, p);
        }
        let f = &self.backup_flows[i];
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
        if let Some(r2) = self.backup_flows[i].live.reading.as_mut() {
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
        let f = &self.backup_flows[i];
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
        if !self.clean() || self.backup_flows[i].step != Step::Write {
            return;
        }
        let Some(r) = self.backup_flows[i].live.reading.clone() else {
            return;
        };
        let f = &self.backup_flows[i];
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
                    .find(|it| it.name == s.name && sha256_of(&it.bytes) == s.sha)
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
        // The seeds a wallet's plan keeps as files, which the person let
        // out unprotected on the secret sheet, go to the stick of files and
        // to no other.
        let files_stick = f.slots.get(slot).is_some_and(|s| s.target == Target::Files);
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
                        && f.seed_file_ok(&it.name) =>
                {
                    sends.push((it.name.clone(), it.bytes.clone(), false));
                }
                _ => refused = true,
            }
        }
        if refused || sends.is_empty() {
            return self.flow_refuse(i, Problem::NotChecked);
        }
        let pending: Vec<(String, [u8; 32])> = sends
            .iter()
            .map(|(n, b, _)| (n.clone(), sha256_of(b)))
            .collect();
        for (name, bytes, vault) in sends {
            // A vault goes over the copy of its salt and nothing else, or
            // beside what the stick holds; any other file beside under a
            // free name (`faraday-files`).
            self.storage_out.push_back(StorageCommand::Write {
                stick: r.id.clone(),
                name,
                bytes,
                replace: vault,
            });
        }
        let f = &mut self.backup_flows[i];
        if let Some(r) = f.live.reading.as_mut() {
            r.hold = false;
        }
        f.live.writing = Some(Writing {
            id: r.id,
            slot,
            pending,
            done: Vec::new(),
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
    /// reason it was not.
    pub(crate) fn flow_write_answer(&mut self, name: &str, answer: Result<String, String>) {
        let Some(i) = self.fi() else {
            return;
        };
        let Some(w) = self.backup_flows[i].live.writing.as_mut() else {
            return;
        };
        let Some(k) = w.pending.iter().position(|(n, _)| n == name) else {
            return;
        };
        let (_, sha) = w.pending.remove(k);
        match answer {
            Ok(wrote_as) => w.done.push((wrote_as, sha)),
            Err(_) => {
                self.backup_flows[i].live.writing = None;
                self.flow_refuse(i, Problem::NotChecked);
                return;
            }
        }
        if w.pending.is_empty()
            && let Some(w) = self.backup_flows[i].live.writing.take()
        {
            self.flow_written_done(i, w.slot, w.id, w.label, w.serial, w.done);
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
        let time = self.clock_now();
        let Some(s) = self.backup_flows[i].slots.get_mut(slot) else {
            return;
        };
        s.written = Some(Written {
            stick: id.clone(),
            label: label.clone(),
            serial,
            time,
            files: done.clone(),
        });
        self.receipt = Some(crate::Receipt {
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
        let f = &mut self.backup_flows[i];
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
        let used = self.backup_flows[i].plan.used_vaults();
        let seals: Vec<Sealed> = self.backup_flows[i]
            .sealing
            .iter()
            .filter(|s| used.contains(&s.vault))
            .cloned()
            .collect();
        for s in seals {
            if let Some(k) = self
                .outbox
                .iter()
                .position(|it| it.name == s.name && sha256_of(&it.bytes) == s.sha)
            {
                let item = self.outbox.remove(k);
                if let Ok(h) = fv::read_header(&item.bytes) {
                    self.inbox.retain(|it| {
                        it.name != item.name
                            && !(it.kind == FileKind::Vault
                                && fv::read_header(&it.bytes).is_ok_and(|o| o.salt == h.salt))
                    });
                }
                self.inbox.push(Item::new(&item.name, item.bytes.clone()));
            }
        }
        // Nothing the flow made waits For the stick: the public files it
        // wrote move to Files, where a copy deleted later is written again
        // from.
        let names: Vec<String> = self.backup_flows[i]
            .slots
            .iter()
            .filter(|s| s.written.is_some())
            .flat_map(|s| s.files.iter().cloned())
            .chain(self.backup_flows[i].public.iter().cloned())
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
        if self.backup_flows[i].kind != Kind::SecureBoot {
            self.backup_flows[i].public.clear();
        }
        self.backup_flows[i].live.reading = None;
        self.save_boxes();
    }

    /// Unlock a vault of the plan from the Done card; lands on the key's
    /// page.
    fn flow_unlock_done(&mut self, i: usize, v: usize) {
        let Some(salt) = self.backup_flows[i]
            .plan
            .vaults
            .get(v)
            .and_then(|vp| vp.salt)
        else {
            return;
        };
        let kind_ = self.backup_flows[i].kind;
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
        self.vaults.category = kind_.category();
        self.vaults.category_chosen = true;
        self.backup_flows[i].live.open = Some(u8::MAX);
        self.act(Action::Vault(crate::vaults::VaultAction::OpenFrom(
            at,
            Screen::VaultContents,
        )));
    }

    /// Points Vault contents at the key the flow made, wherever it is
    /// open, without leaving the flow. Returns whether it was found.
    pub(crate) fn flow_point_at_subject(&mut self, i: usize) -> bool {
        let (kind_, id) = {
            let f = &self.backup_flows[i];
            let Some(s) = f.subject.as_ref() else {
                return false;
            };
            (f.kind, s.id.clone())
        };
        let cat = kind_.category();
        let kinds = crate::vaults::CATEGORIES[cat].1;
        for (o, open) in self.vaults.open.iter().enumerate() {
            let mut n = 0;
            for r in &open.contents.records {
                if !kinds.contains(&r.kind) {
                    continue;
                }
                if r.kind == kind_.record_kind()
                    && subject_of(kind_, r).as_deref() == Some(id.as_str())
                {
                    self.vaults.current = o;
                    self.vaults.category = cat;
                    self.vaults.category_chosen = true;
                    self.vaults.item[cat] = n;
                    return true;
                }
                n += 1;
            }
        }
        false
    }

    /// The key's page in Vault contents.
    pub(crate) fn flow_select_subject(&mut self, i: usize) {
        if !self.flow_point_at_subject(i) {
            return;
        }
        self.vaults.item_open = true;
        self.vaults.form = None;
        self.vaults.saving = false;
        self.vaults.prompt = None;
        self.vaults.back_to = None;
        self.backup_flows[i].live.open = Some(Step::Done.n());
        self.vault_item_offset = 0.0;
        self.screen = Screen::VaultContents;
    }

    /// A waiting action, **Continue the backup**: the flow of the key
    /// shown opens on its step.
    pub(crate) fn flow_resume_selected(&mut self) {
        let Some((kind_, id)) = self.flow_selected_subject() else {
            return;
        };
        if let Some(i) = self
            .backup_flows
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
            .open
            .get(self.vaults.current)?
            .contents
            .records
            .get(at)?;
        let kind_ = match r.kind {
            kind::APK_KEY => Kind::Apk,
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
        self.backup_flows
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
            .sticks
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
        for f in &mut self.backup_flows {
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

    /// Copies and public files kept in Files for the flow's writes are
    /// listed on the visit as In the backup, not tickable.
    pub fn in_the_backup(&self, name: &str) -> bool {
        self.backup_flows.iter().any(|f| {
            f.made
                && f.step != Step::Done
                && (f.sealing.iter().any(|s| s.name == name)
                    // A Secure Boot key's enrolment files, written last.
                    || f.public.iter().any(|n| n == name)
                    || f.slots.iter().any(|s| s.files.iter().any(|n| n == name)))
        })
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
                    &crate::vaults::file_text(*need as usize),
                    &crate::vaults::file_text(*free as usize),
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
            Problem::BoundTo(t) => fill(FEN.backup_flow_bound_to, &[&target(t)]),
            Problem::NotRead(t) => fill(
                FEN.backup_flow_not_read,
                &[m, &f.map_or_else(String::new, |f| f.target_short(t, m))],
            ),
            Problem::HasFirst => fill(FEN.backup_flow_has_first, &[m]),
            Problem::HasFilesOf(t) => {
                let place = match t.place() {
                    Some(p) => BackupFlow::place_name(p),
                    None => target(t),
                };
                fill(FEN.backup_flow_has_files_of, &[m, &place])
            }
            Problem::NotChecked => {
                let label = f
                    .and_then(|f| f.live.reading.as_ref())
                    .map(|r| r.label.clone())
                    .unwrap_or_default();
                fill(FEN.backup_flow_not_checked, &[&label])
            }
            Problem::Pulled => FEN.backup_flow_pulled.to_string(),
        }
    }
}

impl BackupFlow {
    /// "Vault 1 (Place 1)", "the stick of files".
    pub fn target_short(&self, t: &Target, medium: &str) -> String {
        if self.is_setup() {
            return self.target_name(t, medium);
        }
        match t {
            Target::Place(p) => {
                let vs = self.plan.places.get(*p).cloned().unwrap_or_default();
                let names: Vec<String> = vs.iter().map(|&v| BackupFlow::vault_name(v)).collect();
                format!("{} ({})", names.join(", "), BackupFlow::place_name(*p))
            }
            Target::PlaceVault(p, v) => format!(
                "{} ({})",
                BackupFlow::vault_name(*v),
                BackupFlow::place_name(*p)
            ),
            t => self.target_name(t, medium),
        }
    }

    /// "the stick for Vault 1 (Place 1)", "the stick of files".
    pub fn target_name(&self, t: &Target, medium: &str) -> String {
        match t {
            // Before the keys, a stick is a vault's: no place is asked.
            Target::Place(_) | Target::PlaceVault(..) if self.is_setup() => fill(
                FEN.backup_flow_stick_for_vault,
                &[medium, &self.target_vaults(t)],
            ),
            Target::Place(_) | Target::PlaceVault(..) => fill(
                FEN.backup_flow_stick_for,
                &[medium, &self.target_vaults(t), &self.target_place(t)],
            ),
            Target::Files => fill(FEN.backup_flow_files_stick, &[medium]),
            Target::Public => fill(FEN.backup_flow_public_stick, &[medium]),
        }
    }

    fn target_vaults(&self, t: &Target) -> String {
        match t {
            Target::Place(p) => self
                .plan
                .places
                .get(*p)
                .map(|vs| {
                    vs.iter()
                        .map(|&v| BackupFlow::vault_name(v))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            Target::PlaceVault(_, v) => BackupFlow::vault_name(*v),
            _ => String::new(),
        }
    }

    fn target_place(&self, t: &Target) -> String {
        t.place().map(BackupFlow::place_name).unwrap_or_default()
    }
}

/// What a kind's own build gives back (§4.4).
pub struct Built {
    /// The record the key is kept as.
    pub record: Record,
    /// The public value it is known by.
    pub subject: Vec<u8>,
    /// The name it is shown under.
    pub name: String,
    /// Files For the stick: the certificate or the public key.
    pub outbox: Vec<(String, Vec<u8>)>,
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
                    &crate::vaults::time_of_day(crate::vaults::local_secs(t, self.tz_min)),
                ],
            ),
            None => s.name.clone(),
        })
    }
}
