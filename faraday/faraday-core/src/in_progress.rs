//! Flows in progress (`docs/FLOWS.md`, Flows in progress; F41).
//!
//! A flow whose state is kept when it is left is in progress from its
//! first step done until it is finished, started over or let go, or its
//! state is wiped. Each has a row in the sidebar under the tab it is
//! opened from, and a tile at the head of the small Home: its name and
//! its open step, never a word, a passphrase or a value it holds. A press
//! opens it on that step, as it was left.
//!
//! Which flows are in progress, and what their rows say, is worked out
//! once a frame through the drawing memo (`docs/SPEED.md` §3.3): a
//! finished backup is a walk over its checklist. A press decides from the
//! state as it is. The order the flows were begun in, and the screen of
//! its own each was last on, are noted after each press.

use std::rc::Rc;

use crate::strings::fill;
use faraday_crypto::work::Compute;

use crate::backup_flow::{Kind, Step};
use crate::drawing::View;
use crate::family::{CardId, Open};
use crate::glance::{Now, Reads};
use crate::plan::At;
use crate::strings::EN;
use crate::{BStage, Faraday, Screen, bstep};
use faraday_draw::tokens::BODY_M;

/// A flow that can be in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Going {
    /// Make a new wallet, with the vaults set up before its keys and the
    /// Back up it ends in: one row.
    Create,
    /// Restore a wallet, with the vaults set up before its seeds.
    Restore,
    /// Back up a wallet, opened on its own.
    Backup,
    /// A spend: the Spend tab, or a transaction's own steps.
    Spend,
    /// Sign a message.
    Message,
    /// Create a vault, opened on its own.
    Vault,
    /// A backup flow, by its place in [`Faraday::backup_flows`]: a
    /// GPG or Secure Boot key's, or a wallet's no other flow holds.
    Flow(usize),
}

/// One flow in progress, as its row and its tile show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Which flow.
    pub going: Going,
    /// The sidebar entry it is opened from, which it is drawn under.
    pub tab: Screen,
    /// Its name.
    pub name: String,
    /// Its open step.
    pub step: String,
}

/// The flows in progress, in the order they were begun, each with the
/// screen of its own it was last on. What the sidebar's rows and the small
/// Home's tiles list is worked out from the state; this keeps the order.
#[derive(Debug, Default)]
pub(crate) struct InProgress {
    begun: Vec<(Going, Screen)>,
}

impl InProgress {
    /// Where flow `g` stands in the order begun; `usize::MAX` for one not
    /// noted yet (a flow kept across a lock, before the first press).
    pub(crate) fn order_of(&self, g: &Going) -> usize {
        self.begun
            .iter()
            .position(|(n, _)| n == g)
            .unwrap_or(usize::MAX)
    }

    /// Notes the flows now begun, in `now`, from a press that left the
    /// person on `screen`, on `on_screen`'s flow if any: a flow started
    /// over is begun again, last.
    pub(crate) fn note(&mut self, now: Vec<Going>, screen: Screen, on_screen: Option<Going>) {
        self.begun.retain(|(g, _)| now.contains(g));
        for g in now {
            if !self.begun.iter().any(|(n, _)| *n == g) {
                self.begun.push((g, screen));
            }
        }
        if let Some(g) = on_screen
            && let Some(e) = self.begun.iter_mut().find(|(n, _)| *n == g)
        {
            e.1 = screen;
        }
    }

    /// The screen of its own flow `g` was last on.
    pub(crate) fn left_on(&self, g: Going) -> Option<Screen> {
        self.begun.iter().find(|(n, _)| *n == g).map(|(_, s)| *s)
    }

    /// Backup flow `i` is gone: its row goes and the flows after it are
    /// numbered one lower.
    pub(crate) fn flow_removed(&mut self, i: usize) {
        self.begun.retain(|(g, _)| *g != Going::Flow(i));
        for (g, _) in &mut self.begun {
            if let Going::Flow(k) = g
                && *k > i
            {
                *k -= 1;
            }
        }
    }
}

impl Faraday {
    /// Costs what `flows_in_progress_uncached` costs. The flows in
    /// progress, walking a backup's checklist (kept for the frame while one
    /// is drawn): in the order they were begun, what the sidebar's rows and
    /// the small Home's tiles show.
    ///
    /// Works out [`crate::drawing::View::flows_in_progress`] from the state as it is.
    pub fn flows_in_progress_now(&self, c: &Compute) -> Rc<Vec<Row>> {
        Rc::new(self.flows_in_progress_uncached(c))
    }

    /// Costs what `flow_finished` and `flow_row` cost. Works out
    /// [`Faraday::flows_in_progress_now`].
    fn flows_in_progress_uncached(&self, c: &Compute) -> Vec<Row> {
        let mut begun = self.flows_begun_now();
        // In the order noted; one not noted yet (a flow kept across a
        // lock, before the first press) after them.
        begun.sort_by_key(|g| self.in_progress.order_of(g));
        begun
            .into_iter()
            .filter(|&g| !self.flow_finished(c, g))
            .filter_map(|g| self.flow_row(c, g))
            .collect()
    }

    /// The flows begun and not owned by another, in a fixed order: only
    /// lookups.
    fn flows_begun_now(&self) -> Vec<Going> {
        let mut v = Vec::new();
        if self
            .create
            .state()
            .is_some_and(|c| c.built.is_some() || c.done.iter().any(|d| *d))
        {
            v.push(Going::Create);
        }
        if self
            .restore
            .state()
            .is_some_and(|r| r.done.iter().any(|d| *d))
        {
            v.push(Going::Restore);
        }
        if self.backup_going() == Some(Going::Backup) && self.backup_begun() {
            v.push(Going::Backup);
        }
        if self.wallets.spend().is_some() || self.family.state().route.is_some() {
            v.push(Going::Spend);
        }
        if self
            .message
            .state()
            .is_some_and(|m| m.done.iter().any(|d| *d))
        {
            v.push(Going::Message);
        }
        // Create a vault opens with its size done for the person: its
        // first step done is a card of Customise's.
        if self.vaults.back_to.is_none()
            && self.vaults.create.as_ref().is_some_and(|c| {
                c.done
                    .iter()
                    .enumerate()
                    .any(|(k, d)| *d && k != usize::from(crate::vault::vstep::PRESET))
            })
        {
            v.push(Going::Vault);
        }
        for (i, f) in self.flows.iter().enumerate() {
            if (f.step > Step::Where || f.made) && self.flow_held_by(i).is_none() {
                v.push(Going::Flow(i));
            }
        }
        v
    }

    /// Back up's plan is not the empty board it opens on (a preset
    /// chosen, a plan saved before, something placed), or its checklist
    /// is made.
    fn backup_begun(&self) -> bool {
        self.backup.state().is_some_and(|b| {
            b.stage == BStage::Checklist
                || b.answers.map.as_ref().is_none_or(|m| {
                    m.iter()
                        .any(|s| !matches!(s.at, At::Away) && !s.holds.is_empty())
                })
        })
    }

    /// Which row Back up is part of: Make a new wallet's when it backs up
    /// the wallet just made there, else its own.
    fn backup_going(&self) -> Option<Going> {
        let b = self.backup.state()?;
        if self.create.state().and_then(|c| c.built) == Some(b.wallet) {
            Some(Going::Create)
        } else {
            Some(Going::Backup)
        }
    }

    /// The flow that holds backup flow `i` as a part of its own: Make a
    /// new wallet's or Restore's setup of the vaults, or the wallet flow
    /// of the Back up under way.
    fn flow_held_by(&self, i: usize) -> Option<Going> {
        let f = self.flows.get(i)?;
        let owner = f.wallet.as_ref().map_or(0, |w| w.owner);
        if owner != 0 && self.create.state().is_some_and(|c| c.owner == owner) {
            return Some(Going::Create);
        }
        if owner != 0 && self.restore.state().is_some_and(|r| r.owner == owner) {
            return Some(Going::Restore);
        }
        if f.kind.is_wallet() && self.wallet_flow_of_backup() == Some(i) {
            return self.backup_going();
        }
        None
    }

    /// Costs what `flow_finished_on` costs: whether flow `g` is finished.
    fn flow_finished(&self, c: &Compute, g: Going) -> bool {
        flow_finished_on(&Now { app: self, c }, g)
    }

    /// The flow now on screen, when it is one that can be in progress.
    pub(crate) fn flow_on_screen(&self) -> Option<Going> {
        match self.screen {
            Screen::Create if self.create.state().is_some() => Some(Going::Create),
            Screen::Restore if self.restore.state().is_some() => Some(Going::Restore),
            Screen::Backup => self.backup_going(),
            Screen::Spend | Screen::Family => Some(Going::Spend),
            Screen::Message if self.message.state().is_some() => Some(Going::Message),
            Screen::CreateVault if self.vaults.create.is_some() => match self.vaults.back_to {
                None => Some(Going::Vault),
                Some(Screen::BackupFlow) => self.flows.on().map(|i| self.flow_going(i)),
                Some(_) => None,
            },
            Screen::BackupFlow => self.flows.on().map(|i| self.flow_going(i)),
            _ => None,
        }
    }

    /// The row backup flow `i` is shown in.
    pub(crate) fn flow_going(&self, i: usize) -> Going {
        self.flow_held_by(i).unwrap_or(Going::Flow(i))
    }

    /// Notes, after a press, the order the flows were begun in and the
    /// screen of its own each is on: a flow started over is begun again,
    /// last. Only lookups.
    pub(crate) fn flows_note(&mut self) {
        let now = self.flows_begun_now();
        let on_screen = self.flow_on_screen();
        self.in_progress.note(now, self.screen, on_screen);
    }

    /// The screen of its own flow `g` was last on.
    fn flow_left_on(&self, g: Going) -> Option<Screen> {
        self.in_progress.left_on(g)
    }

    /// Opens flow `g` on its open step, as it was left: what its row and
    /// its tile do. Decides from the state as it is: a flow no longer
    /// kept opens nothing.
    pub(crate) fn flow_resume(&mut self, g: Going) {
        let left = self.flow_left_on(g);
        let setup = |app: &Faraday, owner: u64| {
            app.wallet_flow_of_owner(owner)
                .filter(|&i| app.flow_held_by(i) == Some(g))
        };
        match g {
            Going::Create => {
                let Some(owner) = self.create.state().map(|c| c.owner) else {
                    return;
                };
                match left {
                    Some(Screen::Backup) if self.backup_going() == Some(Going::Create) => {
                        self.go(Screen::Backup)
                    }
                    Some(Screen::BackupFlow) if let Some(i) = setup(self, owner) => {
                        self.flow_show(i)
                    }
                    _ => self.go(Screen::Create),
                }
            }
            Going::Restore => {
                let Some(owner) = self.restore.state().map(|r| r.owner) else {
                    return;
                };
                match left {
                    Some(Screen::BackupFlow) if let Some(i) = setup(self, owner) => {
                        self.flow_show(i)
                    }
                    _ => self.go(Screen::Restore),
                }
            }
            Going::Backup if self.backup.state().is_some() => self.go(Screen::Backup),
            Going::Spend => match left {
                Some(Screen::Spend) if self.wallets.spend().is_some() => self.go(Screen::Spend),
                _ => self.go(Screen::Family),
            },
            Going::Message if self.message.state().is_some() => self.go(Screen::Message),
            Going::Vault if self.vaults.create.is_some() => self.go(Screen::CreateVault),
            Going::Flow(i) if i < self.flows.len() => self.flow_show(i),
            _ => {}
        }
        // Opened again from where the person is now: Back goes there, not
        // to where the flow was first opened from (F77).
        let here = self.screen;
        self.trail.drop_screen(here);
    }

    /// To a screen as the sidebar goes there.
    fn go(&mut self, s: Screen) {
        self.act_inner(crate::Action::Nav(s));
    }

    /// The flow on screen when it offers **Cancel**: one that can be in
    /// progress, unless a backup flow whose key is made is on screen or
    /// is the flow. Such a flow ends only by finishing, so its writes'
    /// hold (F6, F31) is never let go by a Cancel. Only lookups.
    pub(crate) fn cancel_offered(&self) -> Option<Going> {
        let g = self.flow_on_screen()?;
        let made = |i: usize| self.flows.get(i).is_some_and(|f| f.made);
        let shown = self.screen == Screen::BackupFlow
            || (self.screen == Screen::CreateVault
                && self.vaults.back_to == Some(Screen::BackupFlow));
        if shown && self.flows.on().is_some_and(made) {
            return None;
        }
        if let Going::Flow(i) = g
            && made(i)
        {
            return None;
        }
        Some(g)
    }

    /// **Cancel** at the top of the flow on screen (F51): asks first when
    /// ending it loses something the person would notice, else ends it.
    /// Decides from the state as it is.
    pub(crate) fn flow_cancel(&mut self) {
        if self.key_cancel_offered() {
            if self.key_cancel_loses() {
                self.sheet = Some(crate::Sheet::CancelKey);
            } else {
                self.key_cancel_end();
            }
            return;
        }
        let Some(g) = self.cancel_offered() else {
            return;
        };
        if self.cancel_loses(g).is_some() {
            self.sheet = Some(crate::Sheet::CancelFlow(g));
        } else {
            self.flow_end(g);
        }
    }

    /// **Cancel the flow** on the sheet: the flow it names ends.
    pub(crate) fn flow_cancel_now(&mut self) {
        match self.sheet {
            Some(crate::Sheet::CancelFlow(g)) => {
                self.sheet = None;
                self.flow_end(g);
            }
            Some(crate::Sheet::CancelKey) => {
                self.sheet = None;
                self.key_cancel_end();
            }
            _ => {}
        }
    }

    /// Whether the page offers **Cancel** at its top: a flow that can be
    /// in progress ([`Self::cancel_offered`]), or a key flow opened from
    /// Make a new wallet ([`Self::key_cancel_offered`]). Only lookups.
    pub(crate) fn cancel_shown(&self) -> bool {
        self.cancel_offered().is_some() || self.key_cancel_offered()
    }

    /// Whether the key flow on screen, New key or Make codex32 shares
    /// opened for a slot of Make a new wallet, offers **Cancel**: until
    /// its key is locked in. Its Cancel returns to Make a new wallet, which
    /// goes on (`docs/FLOWS.md`, Cancel). Only lookups.
    pub(crate) fn key_cancel_offered(&self) -> bool {
        self.screen == Screen::KeyGen
            && self.keygen.state().is_some_and(|k| {
                k.slot.is_some()
                    && k.back == Screen::Create
                    && !k.locked
                    && !k.codex32.as_ref().is_some_and(|m| m.locked())
            })
            && self.create.state().is_some_and(|c| c.built.is_none())
    }

    /// Whether the key flow's Cancel discards something entered: rolls,
    /// characters, words or a passphrase. Only lookups.
    pub(crate) fn key_cancel_loses(&self) -> bool {
        self.keygen
            .state()
            .is_some_and(|k| k.entered_any(self.entry.state()))
    }

    /// The key flow's Cancel: what it held is discarded, and Make a new
    /// wallet is shown at its Keys card, the slot as it was; where Keys was
    /// passed over for this flow, at the card before Keys. Decides from
    /// the state as it is.
    pub(crate) fn key_cancel_end(&mut self) {
        let Some(k) = self.keygen_drop() else {
            return;
        };
        self.return_back(k.back);
        let c = self.compute();
        let open = if self.create_keys_one_way(&c) {
            self.create_card_before_keys()
        } else {
            Some(crate::cstep::KEYS)
        };
        if let Some(cs) = self.create.state_mut() {
            cs.open = open;
            cs.scroll.follow = true;
        }
    }

    /// The Cancel sheet's title over flow `g`.
    pub(crate) fn cancel_title(&self, g: Going) -> &'static str {
        match g {
            Going::Create => EN.flow_cancel_create,
            Going::Restore => EN.flow_cancel_restore,
            Going::Backup => EN.flow_cancel_backup,
            Going::Spend => EN.flow_cancel_spend,
            Going::Message => EN.flow_cancel_message,
            Going::Vault => EN.flow_cancel_vault,
            Going::Flow(i) => match self.flows.get(i).map(|f| f.kind) {
                Some(Kind::SecureBoot) => EN.flow_cancel_keys,
                Some(Kind::Gpg) => EN.flow_cancel_key,
                _ => EN.flow_cancel_backup,
            },
        }
    }

    /// What ending flow `g` loses that the person would notice, as the
    /// line the Cancel sheet says it in: `None` when nothing past its
    /// first card is done. Keys, vaults and the Outbox are never lost by
    /// it.
    pub(crate) fn cancel_loses(&self, g: Going) -> Option<&'static str> {
        // A card done other than the first, by the cards in order.
        let past_first =
            |cards: &[u8], done: &dyn Fn(u8) -> bool| cards.iter().skip(1).any(|&k| done(k));
        match g {
            Going::Create => {
                let c = self.create.state()?;
                if self.backup_going() == Some(Going::Create) && self.backup_begun() {
                    return Some(EN.flow_lost_plan);
                }
                if self.wallet_flow_of_owner(c.owner).is_some() {
                    return Some(EN.flow_lost_setup);
                }
                let cards = self.create_cards();
                (c.built.is_some() || past_first(&cards, &|k| c.done[k as usize]))
                    .then_some(EN.flow_lost_answers)
            }
            Going::Restore => {
                let r = self.restore.state()?;
                if self.wallet_flow_of_owner(r.owner).is_some() {
                    return Some(EN.flow_lost_setup);
                }
                past_first(&r.steps(), &|k| r.done[k as usize]).then_some(EN.flow_lost_answers)
            }
            Going::Backup => self.backup_begun().then_some(EN.flow_lost_plan),
            Going::Spend => self
                .wallets
                .spend()
                .is_some_and(|s| s.done.iter().any(|d| *d))
                .then_some(EN.flow_lost_spend),
            Going::Message => {
                let m = self.message.state()?;
                m.done[1..]
                    .iter()
                    .any(|d| *d)
                    .then_some(EN.flow_lost_answers)
            }
            Going::Vault => {
                let c = self.vaults.create.as_ref()?;
                let preset = crate::vault::vstep::PRESET;
                c.steps()
                    .iter()
                    .skip(1)
                    .any(|&k| k != preset && c.done[k as usize])
                    .then_some(EN.flow_lost_answers)
            }
            Going::Flow(i) => {
                let f = self.flows.get(i)?;
                (f.step > Step::Where).then_some(EN.flow_lost_plan)
            }
        }
    }

    /// Ends flow `g` for good: its own state as if never begun (its
    /// answers, its plan, its board), and the screen it was opened from
    /// shown. The keys made or loaded, the wallet made, the vaults and
    /// the Outbox stay. A backup flow it holds goes with it unless that
    /// flow's key is made: its writes still wait, under a row of its own.
    pub(crate) fn flow_end(&mut self, g: Going) {
        use crate::Action;
        let was = self.trail_shown();
        let to = match g {
            Going::Create => {
                let Some(home) = self.create_drop() else {
                    return;
                };
                if home {
                    Action::Nav(Screen::Home)
                } else {
                    Action::Nav(Screen::WalletsStart)
                }
            }
            Going::Restore => {
                let Some(r) = self.restore.take() else {
                    return;
                };
                self.flow_drop_unmade(self.wallet_flow_of_owner(r.owner));
                if r.from_home {
                    Action::Nav(Screen::Home)
                } else {
                    Action::Nav(Screen::WalletsStart)
                }
            }
            Going::Backup => {
                let i = self.wallet_flow_of_backup();
                let Some(b) = self.backup.take() else {
                    return;
                };
                self.flow_drop_unmade(i);
                if b.from_vault {
                    Action::Nav(Screen::VaultContents)
                } else {
                    Action::OpenWallet(b.wallet)
                }
            }
            Going::Spend => {
                let to = match self
                    .family_wallet()
                    .filter(|_| self.family.state().from_card)
                {
                    Some(w) => Action::OpenWallet(w),
                    None if self.family.state().from == Some(Screen::Home) => {
                        Action::Nav(Screen::Home)
                    }
                    None => Action::Nav(Screen::WalletsStart),
                };
                self.wallets.clear_spend();
                self.family_act(crate::family::FamilyAction::StartOver);
                to
            }
            Going::Message => {
                self.message.clear_state();
                Action::Nav(Screen::WalletsStart)
            }
            Going::Vault => {
                let home = self.vaults.from_home;
                self.vaults.drop_create();
                if home {
                    Action::Nav(Screen::Home)
                } else {
                    Action::Nav(Screen::Vaults)
                }
            }
            Going::Flow(i) => {
                let Some(f) = self.flows.get(i) else {
                    return;
                };
                let tab = if f.kind.is_wallet() {
                    Screen::WalletsStart
                } else {
                    Screen::Catalog
                };
                self.flow_drop(i);
                Action::Nav(tab)
            }
        };
        self.act_inner(to);
        // Where it was opened from, as it was seen (F77); `to` when the
        // trail does not hold it.
        self.cancel_landing(&was);
    }

    /// [`Self::flow_drop`] of backup flow `i`, unless its key is made.
    fn flow_drop_unmade(&mut self, i: Option<usize>) {
        if let Some(i) = i
            && self.flows.get(i).is_some_and(|f| !f.made)
        {
            self.flow_drop(i);
        }
    }

    /// Make a new wallet ended: its state, its Back up and the backup
    /// flows that were part of it and made nothing. The wallet made
    /// stays. Returns whether it was opened from Home; `None` when none
    /// was under way.
    pub(crate) fn create_drop(&mut self) -> Option<bool> {
        let c = self.create.state()?;
        let (owner, home) = (c.owner, c.from_home);
        // Its Back up goes with it.
        let backup = (self.backup_going() == Some(Going::Create))
            .then(|| self.wallet_flow_of_backup())
            .flatten();
        if self.backup_going() == Some(Going::Create) {
            self.backup.close();
        }
        self.create.clear();
        let setup = self.wallet_flow_of_owner(owner);
        // The later index first, so the earlier stays where it is.
        let mut drop: Vec<usize> = setup.into_iter().chain(backup).collect();
        drop.sort_unstable();
        drop.dedup();
        for i in drop.into_iter().rev() {
            self.flow_drop_unmade(Some(i));
        }
        Some(home)
    }

    /// Costs what the checklist's answers cost. Make a new wallet is under
    /// way and its keys are kept nowhere yet: begun, its backup not done,
    /// and no backup flow of it has made its vaults (`docs/history/NEW-WALLET.md`
    /// §18). The idle lock waits, a lock or a power-off ends it, and the
    /// sidebar says so.
    pub fn lock_held(&self, c: &Compute) -> bool {
        lock_held_on(&Now { app: self, c })
    }

    /// Takes backup flow `i` out of the list, and with it the vault form
    /// it had open; what points into the list past it follows.
    fn flow_drop(&mut self, i: usize) {
        if i >= self.flows.len() {
            return;
        }
        if self.flows.drop_flow(i) && self.vaults.back_to == Some(Screen::BackupFlow) {
            self.vaults.leave_create();
        }
        self.in_progress.flow_removed(i);
    }

    /// Costs what `backup_step` costs. Flow `g`'s row: its tab, its name
    /// and its open step.
    fn flow_row(&self, c: &Compute, g: Going) -> Option<Row> {
        let wallets = Screen::WalletsStart;
        let (tab, name, step) = match g {
            Going::Create => {
                let step = match self.backup_going() {
                    Some(Going::Create) if self.backup_begun() => self.backup_step(c)?,
                    _ => self.create_step()?,
                };
                (wallets, EN.sidebar_flow_create.to_string(), step)
            }
            Going::Restore => {
                let r = self.restore.state()?;
                let steps = r.steps();
                let k = r
                    .open
                    .or_else(|| steps.iter().copied().find(|&k| !r.done[k as usize]))
                    .or_else(|| steps.last().copied())?;
                let step = crate::screens::RSTEPS[k as usize].to_string();
                (wallets, EN.sidebar_flow_restore.to_string(), step)
            }
            Going::Backup => (
                wallets,
                EN.sidebar_flow_backup.to_string(),
                self.backup_step(c)?,
            ),
            Going::Spend => (
                wallets,
                EN.sidebar_flow_spend.to_string(),
                self.spend_step(),
            ),
            Going::Message => {
                let m = self.message.state()?;
                let k = m
                    .open
                    .or_else(|| (0..4u8).find(|&k| !m.done[k as usize]))
                    .unwrap_or(crate::mstep::SIGN);
                let step = crate::screens::MSTEPS[k as usize].to_string();
                (wallets, EN.sidebar_flow_message.to_string(), step)
            }
            Going::Vault => {
                let c = self.vaults.create.as_ref()?;
                let k = c
                    .open
                    .or_else(|| (0..c.done.len()).find(|&k| !c.done[k]).map(|k| k as u8))
                    .unwrap_or(crate::vault::vstep::PHRASES);
                let step = crate::vault::screen::VSTEPS[k as usize].to_string();
                (Screen::Vaults, EN.sidebar_flow_vault.to_string(), step)
            }
            Going::Flow(i) => {
                let f = self.flows.get(i)?;
                let s = f.live.open.and_then(Step::of).unwrap_or(f.step);
                let step = crate::backup_flow::screen::card_title(self, f, s);
                let tab = if f.kind == Kind::Wallet {
                    wallets
                } else {
                    Screen::Catalog
                };
                (tab, f.title(), step)
            }
        };
        Some(Row {
            going: g,
            tab,
            name,
            step,
        })
    }

    /// Make a new wallet's open card, or the first not done.
    fn create_step(&self) -> Option<String> {
        let c = self.create.state()?;
        let cards = self.create_cards();
        let k = c
            .open
            .or_else(|| cards.iter().copied().find(|&k| !self.create_card_done(k)))
            .or_else(|| cards.last().copied())?;
        Some(if k == crate::cstep::STICKS {
            fill(EN.backup_flow_card_sticks, &[self.medium.nouns()])
        } else {
            crate::screens::CSTEPS[k as usize].to_string()
        })
    }

    /// Costs what `backup_items_now`, `backup_item_done_now` and
    /// `item_title_now` cost. Back up's open card: a card of the plan, or
    /// an item of the checklist, else the first item not done.
    fn backup_step(&self, c: &Compute) -> Option<String> {
        let b = self.backup.state()?;
        if b.stage == BStage::Plan {
            return Some(crate::plan::cards::q_title(
                b.q.unwrap_or(crate::qstep::PLAN),
            ));
        }
        let n = b.open.or_else(|| {
            self.backup_items_now(c)
                .into_iter()
                .find(|&n| bstep::item(n).is_some_and(|it| !self.backup_item_done_now(c, it)))
        });
        Some(match n {
            Some(n) => crate::screens::item_title_now(c, self, n).0,
            None => EN.app_done.to_string(),
        })
    }

    /// The spend's open card: a page of the Spend tab or a step of the
    /// transaction.
    fn spend_step(&self) -> String {
        let step_of = |n: u8| crate::family::text::title(self, CardId::Step(n));
        let spend_open = || {
            let s = self.wallets.spend()?;
            s.open
                .or_else(|| s.steps.iter().copied().find(|&n| !s.done[n as usize]))
                .map(step_of)
        };
        if self.flow_left_on(Going::Spend) == Some(Screen::Spend) && self.wallets.spend().is_some()
        {
            return spend_open().unwrap_or_else(|| EN.sidebar_flow_spend.to_string());
        }
        match self.family.state().open {
            Some(Open::Page(p)) => crate::family::text::title(self, CardId::Page(p)),
            Some(Open::Spend) => spend_open().unwrap_or_else(|| EN.sidebar_flow_spend.to_string()),
            None => spend_open().unwrap_or_else(|| {
                crate::family::text::title(self, CardId::Page(crate::family::page::HOLDING))
            }),
        }
    }
}

/// The Cancel sheet over flow `g` (F51): its question, the one line of
/// what is lost, then **Cancel the flow** and **Keep going**. Reads only.
pub(crate) fn cancel_sheet(app: &View, ui: &mut crate::ui::Ui, g: Going, w: f32, h: f32) {
    use crate::Action;
    use crate::compact::{buttons, scroll_sheet};
    use crate::ui::Style;
    use crate::ui::pal::*;
    use faraday_draw::widgets::Icon;
    let title = app.cancel_title(g);
    let line = app.cancel_loses(g).unwrap_or_default();
    let place = if ui.compact {
        (8.0, w - 16.0)
    } else {
        let sw = (w - 48.0).min(520.0);
        ((w - sw) / 2.0, sw)
    };
    let margin = if ui.compact { 8.0 } else { 32.0 };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Warning, Role::Warning, title);
        } else {
            ui.icon(x, cy, 30.0, Icon::Warning, 18.0, Role::Warning);
            let t = ui.fit_as(faraday_draw::tokens::sheet_head(false), title, iw - 42.0);
            ui.text_mid_as(
                (x + 42.0, cy, 30.0),
                faraday_draw::tokens::sheet_head(false),
                Role::OnSurface,
                &t,
            );
            cy += 46.0;
        }
        cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, line) + 18.0;
        cy += buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (EN.flow_keep_going, Style::Secondary, Action::Cancel),
                (
                    EN.flow_cancel_the_flow,
                    Style::Primary,
                    Action::CancelFlowNow,
                ),
            ],
        );
        cy - y
    });
}

/// The Cancel dialog over a key flow opened from Make a new wallet
/// (`faraday_ui::sheet::dialog`): the key so far is discarded and Make a
/// new wallet shown again; **Keep going** and **Discard**. Reads only.
pub(crate) fn cancel_key_sheet(_app: &View, ui: &mut crate::ui::Ui, w: f32, h: f32) {
    use crate::Action;
    use crate::ui::Style;
    use crate::ui::pal::*;
    use faraday_draw::widgets::Icon;
    let head = (
        Icon::Warning.into(),
        Role::Warning.paint(),
        EN.flow_cancel_key_title,
    );
    let actions = [
        (EN.flow_keep_going, Style::Secondary, Action::Cancel),
        (
            EN.flow_cancel_key_discard,
            Style::Primary,
            Action::CancelFlowNow,
        ),
    ];
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            // Material's supporting text, Body Medium (14).
            ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, EN.flow_cancel_key_line)
                + faraday_draw::tokens::GAP
        },
        &actions,
    );
}

/// [`Faraday::lock_held`] read through `r` ([`Reads`]).
pub(crate) fn lock_held_on(r: &impl Reads) -> bool {
    let app = r.app();
    let Some(cs) = app.create.state() else {
        return false;
    };
    let begun = cs.built.is_some() || cs.done.iter().any(|d| *d);
    let kept = app.flows.iter().any(|f| {
        f.kind == Kind::Wallet && f.made && f.wallet.as_ref().is_some_and(|w| w.owner == cs.owner)
    });
    begun && !kept && !flow_finished_on(r, Going::Create)
}

/// Whether flow `g` is finished, read through `r` ([`Reads`]).
fn flow_finished_on(r: &impl Reads, g: Going) -> bool {
    let app = r.app();
    match g {
        Going::Create => {
            app.create.state().is_some_and(|c| c.built.is_some())
                && app.backup_going() == Some(Going::Create)
                && backup_finished_on(r)
        }
        Going::Restore => app
            .restore
            .state()
            .is_some_and(|x| x.done[crate::rstep::CHECK as usize]),
        Going::Backup => backup_finished_on(r),
        Going::Spend => app
            .wallets
            .spend()
            .is_some_and(|s| s.out_tx || s.out_signed),
        Going::Message => app.message.state().is_some_and(|m| m.signed.is_some()),
        Going::Vault => false,
        Going::Flow(i) => app.flows.get(i).is_none_or(|f| f.step == Step::Done),
    }
}

/// Back up's checklist is made and every item on it done, read through
/// `r` ([`Reads`]).
fn backup_finished_on(r: &impl Reads) -> bool {
    r.app().backup().is_some_and(|b| {
        b.stage == BStage::Checklist
            && r.backup_items()
                .into_iter()
                .filter_map(bstep::item)
                .all(|it| r.backup_item_done(it))
    })
}
