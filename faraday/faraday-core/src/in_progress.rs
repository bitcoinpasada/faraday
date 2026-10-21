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
use osk_crypto::work::Compute;

use crate::backup_flow::{Kind, Step};
use crate::drawing::View;
use crate::family::{CardId, Open};
use crate::plan::At;
use crate::strings::EN;
use crate::{BStage, Faraday, Screen, bstep};

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
    /// A backup flow, by its place in [`Faraday::backup_flows`]: an APK,
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

impl Faraday {
    /// The flows in progress, walking a backup's checklist (kept for the
    /// frame while one is drawn): in the order they were begun, what the
    /// sidebar's rows and the small Home's tiles show.
    pub fn flows_in_progress(&self, c: &Compute) -> Rc<Vec<Row>> {
        if self.drawing.is_on() {
            return crate::drawing::View::new(self).flows_in_progress();
        }
        self.flows_in_progress_now(c)
    }

    /// Works out [`Faraday::flows_in_progress`] from the state as it is.
    pub fn flows_in_progress_now(&self, c: &Compute) -> Rc<Vec<Row>> {
        Rc::new(self.flows_in_progress_uncached(c))
    }

    /// Works out [`Faraday::flows_in_progress`].
    fn flows_in_progress_uncached(&self, c: &Compute) -> Vec<Row> {
        let mut begun = self.flows_begun_now();
        // In the order noted; one not noted yet (a flow kept across a
        // lock, before the first press) after them.
        let at = |g: &Going| {
            self.flows_begun
                .iter()
                .position(|(n, _)| n == g)
                .unwrap_or(usize::MAX)
        };
        begun.sort_by_key(at);
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
            .as_ref()
            .is_some_and(|c| c.built.is_some() || c.done.iter().any(|d| *d))
        {
            v.push(Going::Create);
        }
        if self
            .restore
            .as_ref()
            .is_some_and(|r| r.done.iter().any(|d| *d))
        {
            v.push(Going::Restore);
        }
        if self.backup_going() == Some(Going::Backup) && self.backup_begun() {
            v.push(Going::Backup);
        }
        if self.spend.is_some() || self.family.route.is_some() {
            v.push(Going::Spend);
        }
        if self
            .message
            .as_ref()
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
                    .any(|(k, d)| *d && k != usize::from(crate::vaults::vstep::PRESET))
            })
        {
            v.push(Going::Vault);
        }
        for (i, f) in self.backup_flows.iter().enumerate() {
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
        self.backup.as_ref().is_some_and(|b| {
            b.stage == BStage::Checklist
                || b.answers.map.as_ref().is_none_or(|m| {
                    m.iter()
                        .any(|s| !matches!(s.at, At::Away) && !s.holds.is_empty())
                })
        })
    }

    /// Back up's checklist is made and every item on it done.
    fn backup_finished(&self, c: &Compute) -> bool {
        self.backup.as_ref().is_some_and(|b| {
            b.stage == BStage::Checklist
                && self
                    .backup_items()
                    .into_iter()
                    .filter_map(bstep::item)
                    .all(|it| self.backup_item_done(c, it))
        })
    }

    /// Which row Back up is part of: Make a new wallet's when it backs up
    /// the wallet just made there, else its own.
    fn backup_going(&self) -> Option<Going> {
        let b = self.backup.as_ref()?;
        if self.create.as_ref().and_then(|c| c.built) == Some(b.wallet) {
            Some(Going::Create)
        } else {
            Some(Going::Backup)
        }
    }

    /// The flow that holds backup flow `i` as a part of its own: Make a
    /// new wallet's or Restore's setup of the vaults, or the wallet flow
    /// of the Back up under way.
    fn flow_held_by(&self, i: usize) -> Option<Going> {
        let f = self.backup_flows.get(i)?;
        let owner = f.wallet.as_ref().map_or(0, |w| w.owner);
        if owner != 0 && self.create.as_ref().is_some_and(|c| c.owner == owner) {
            return Some(Going::Create);
        }
        if owner != 0 && self.restore.as_ref().is_some_and(|r| r.owner == owner) {
            return Some(Going::Restore);
        }
        if f.kind.is_wallet() && self.wallet_flow_of_backup() == Some(i) {
            return self.backup_going();
        }
        None
    }

    fn flow_finished(&self, c: &Compute, g: Going) -> bool {
        match g {
            Going::Create => {
                self.create.as_ref().is_some_and(|c| c.built.is_some())
                    && self.backup_going() == Some(Going::Create)
                    && self.backup_finished(c)
            }
            Going::Restore => self
                .restore
                .as_ref()
                .is_some_and(|r| r.done[crate::rstep::CHECK as usize]),
            Going::Backup => self.backup_finished(c),
            Going::Spend => self
                .spend
                .as_ref()
                .is_some_and(|s| s.out_tx || s.out_signed),
            Going::Message => self.message.as_ref().is_some_and(|m| m.signed.is_some()),
            Going::Vault => false,
            Going::Flow(i) => self
                .backup_flows
                .get(i)
                .is_none_or(|f| f.step == Step::Done),
        }
    }

    /// The flow now on screen, when it is one that can be in progress.
    pub(crate) fn flow_on_screen(&self) -> Option<Going> {
        match self.screen {
            Screen::Create if self.create.is_some() => Some(Going::Create),
            Screen::Restore if self.restore.is_some() => Some(Going::Restore),
            Screen::Backup => self.backup_going(),
            Screen::Spend | Screen::Family => Some(Going::Spend),
            Screen::Message if self.message.is_some() => Some(Going::Message),
            Screen::CreateVault if self.vaults.create.is_some() => match self.vaults.back_to {
                None => Some(Going::Vault),
                Some(Screen::BackupFlow) => self.flow_on.map(|i| self.flow_going(i)),
                Some(_) => None,
            },
            Screen::BackupFlow => self.flow_on.map(|i| self.flow_going(i)),
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
        self.flows_begun.retain(|(g, _)| now.contains(g));
        let screen = self.screen;
        for g in now {
            if !self.flows_begun.iter().any(|(n, _)| *n == g) {
                self.flows_begun.push((g, screen));
            }
        }
        if let Some(g) = self.flow_on_screen()
            && let Some(e) = self.flows_begun.iter_mut().find(|(n, _)| *n == g)
        {
            e.1 = screen;
        }
    }

    /// The screen of its own flow `g` was last on.
    fn flow_left_on(&self, g: Going) -> Option<Screen> {
        self.flows_begun
            .iter()
            .find(|(n, _)| *n == g)
            .map(|(_, s)| *s)
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
                let Some(owner) = self.create.as_ref().map(|c| c.owner) else {
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
                let Some(owner) = self.restore.as_ref().map(|r| r.owner) else {
                    return;
                };
                match left {
                    Some(Screen::BackupFlow) if let Some(i) = setup(self, owner) => {
                        self.flow_show(i)
                    }
                    _ => self.go(Screen::Restore),
                }
            }
            Going::Backup if self.backup.is_some() => self.go(Screen::Backup),
            Going::Spend => match left {
                Some(Screen::Spend) if self.spend.is_some() => self.go(Screen::Spend),
                _ => self.go(Screen::Family),
            },
            Going::Message if self.message.is_some() => self.go(Screen::Message),
            Going::Vault if self.vaults.create.is_some() => self.go(Screen::CreateVault),
            Going::Flow(i) if i < self.backup_flows.len() => self.flow_show(i),
            _ => {}
        }
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
        let made = |i: usize| self.backup_flows.get(i).is_some_and(|f| f.made);
        let shown = self.screen == Screen::BackupFlow
            || (self.screen == Screen::CreateVault
                && self.vaults.back_to == Some(Screen::BackupFlow));
        if shown && self.flow_on.is_some_and(made) {
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
        if let Some(crate::Sheet::CancelFlow(g)) = self.sheet {
            self.sheet = None;
            self.flow_end(g);
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
            Going::Flow(i) => match self.backup_flows.get(i).map(|f| f.kind) {
                Some(Kind::SecureBoot) => EN.flow_cancel_keys,
                Some(Kind::Apk | Kind::Gpg) => EN.flow_cancel_key,
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
                let c = self.create.as_ref()?;
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
                let r = self.restore.as_ref()?;
                if self.wallet_flow_of_owner(r.owner).is_some() {
                    return Some(EN.flow_lost_setup);
                }
                past_first(&r.steps(), &|k| r.done[k as usize]).then_some(EN.flow_lost_answers)
            }
            Going::Backup => self.backup_begun().then_some(EN.flow_lost_plan),
            Going::Spend => self
                .spend
                .as_ref()
                .is_some_and(|s| s.done.iter().any(|d| *d))
                .then_some(EN.flow_lost_spend),
            Going::Message => {
                let m = self.message.as_ref()?;
                m.done[1..]
                    .iter()
                    .any(|d| *d)
                    .then_some(EN.flow_lost_answers)
            }
            Going::Vault => {
                let c = self.vaults.create.as_ref()?;
                let preset = crate::vaults::vstep::PRESET;
                c.steps()
                    .iter()
                    .skip(1)
                    .any(|&k| k != preset && c.done[k as usize])
                    .then_some(EN.flow_lost_answers)
            }
            Going::Flow(i) => {
                let f = self.backup_flows.get(i)?;
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
        let to = match g {
            Going::Create => {
                let Some(home) = self.create_drop() else {
                    return;
                };
                if home {
                    Action::Nav(Screen::Home)
                } else {
                    Action::Nav(Screen::Start)
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
                    Action::Nav(Screen::Start)
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
                let to = match self.family_wallet().filter(|_| self.family.from_card) {
                    Some(w) => Action::OpenWallet(w),
                    None if self.family.from == Some(Screen::Home) => Action::Nav(Screen::Home),
                    None => Action::Nav(Screen::Start),
                };
                self.spend = None;
                self.family_act(crate::family::FamilyAction::StartOver);
                to
            }
            Going::Message => {
                self.message = None;
                Action::Nav(Screen::Start)
            }
            Going::Vault => {
                let home = self.vaults.from_home;
                self.vaults.create = None;
                self.vaults.focus = None;
                if home {
                    Action::Nav(Screen::Home)
                } else {
                    Action::Nav(Screen::Vaults)
                }
            }
            Going::Flow(i) => {
                let Some(f) = self.backup_flows.get(i) else {
                    return;
                };
                let tab = if f.kind.is_wallet() {
                    Screen::Start
                } else {
                    Screen::Catalog
                };
                self.flow_drop(i);
                Action::Nav(tab)
            }
        };
        self.act_inner(to);
    }

    /// [`Self::flow_drop`] of backup flow `i`, unless its key is made.
    fn flow_drop_unmade(&mut self, i: Option<usize>) {
        if let Some(i) = i
            && self.backup_flows.get(i).is_some_and(|f| !f.made)
        {
            self.flow_drop(i);
        }
    }

    /// Make a new wallet ended: its state, its Back up and the backup
    /// flows that were part of it and made nothing. The wallet made
    /// stays. Returns whether it was opened from Home; `None` when none
    /// was under way.
    pub(crate) fn create_drop(&mut self) -> Option<bool> {
        let c = self.create.as_ref()?;
        let (owner, home) = (c.owner, c.from_home);
        // Its Back up goes with it.
        let backup = (self.backup_going() == Some(Going::Create))
            .then(|| self.wallet_flow_of_backup())
            .flatten();
        if self.backup_going() == Some(Going::Create) {
            self.backup = None;
        }
        self.create = None;
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

    /// Make a new wallet is under way and its keys are kept nowhere yet:
    /// begun, its backup not done, and no backup flow of it has made its
    /// vaults (`docs/NEW-WALLET.md` §18). The idle lock waits, a lock or
    /// a power-off ends it, and the sidebar says so. Costs what the
    /// checklist's answers cost.
    pub fn lock_held(&self, c: &Compute) -> bool {
        let Some(cs) = self.create.as_ref() else {
            return false;
        };
        let begun = cs.built.is_some() || cs.done.iter().any(|d| *d);
        let kept = self.backup_flows.iter().any(|f| {
            f.kind == Kind::Wallet
                && f.made
                && f.wallet.as_ref().is_some_and(|w| w.owner == cs.owner)
        });
        begun && !kept && !self.flow_finished(c, Going::Create)
    }

    /// Takes backup flow `i` out of the list, and with it the vault form
    /// it had open; what points into the list past it follows.
    fn flow_drop(&mut self, i: usize) {
        if i >= self.backup_flows.len() {
            return;
        }
        self.backup_flows.remove(i);
        match self.flow_on {
            Some(k) if k == i => {
                self.flow_on = None;
                if self.vaults.back_to == Some(Screen::BackupFlow) {
                    self.vaults.back_to = None;
                    self.vaults.create = None;
                    self.vaults.focus = None;
                }
            }
            Some(k) if k > i => self.flow_on = Some(k - 1),
            _ => {}
        }
        self.flows_begun.retain(|(g, _)| *g != Going::Flow(i));
        for (g, _) in &mut self.flows_begun {
            if let Going::Flow(k) = g
                && *k > i
            {
                *k -= 1;
            }
        }
    }

    /// Flow `g`'s row: its tab, its name and its open step.
    fn flow_row(&self, c: &Compute, g: Going) -> Option<Row> {
        let wallets = Screen::Start;
        let (tab, name, step) = match g {
            Going::Create => {
                let step = match self.backup_going() {
                    Some(Going::Create) if self.backup_begun() => self.backup_step(c)?,
                    _ => self.create_step()?,
                };
                (wallets, EN.sidebar_flow_create.to_string(), step)
            }
            Going::Restore => {
                let r = self.restore.as_ref()?;
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
                let m = self.message.as_ref()?;
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
                    .unwrap_or(crate::vaults::vstep::PHRASES);
                let step = crate::vault_screens::VSTEPS[k as usize].to_string();
                (Screen::Vaults, EN.sidebar_flow_vault.to_string(), step)
            }
            Going::Flow(i) => {
                let f = self.backup_flows.get(i)?;
                let s = f.live.open.and_then(Step::of).unwrap_or(f.step);
                let step = crate::backup_flow_screen::card_title(self, f, s);
                match (f.kind, f.wallet.as_ref()) {
                    (Kind::Wallet, Some(w)) if !w.name.is_empty() => {
                        (wallets, fill(EN.sidebar_flow_backup_of, &[&w.name]), step)
                    }
                    (Kind::Wallet, _) => (wallets, f.kind.name().to_string(), step),
                    (k, _) => (Screen::Catalog, k.name().to_string(), step),
                }
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
        let c = self.create.as_ref()?;
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

    /// Back up's open card: a card of the plan, or an item of the
    /// checklist, else the first item not done.
    fn backup_step(&self, c: &Compute) -> Option<String> {
        let b = self.backup.as_ref()?;
        if b.stage == BStage::Plan {
            return Some(crate::plan_cards::q_title(
                b.q.unwrap_or(crate::qstep::PLAN),
            ));
        }
        let n = b.open.or_else(|| {
            self.backup_items()
                .into_iter()
                .find(|&n| bstep::item(n).is_some_and(|it| !self.backup_item_done(c, it)))
        });
        Some(match n {
            Some(n) => crate::screens::item_title(c, self, n).0,
            None => EN.app_done.to_string(),
        })
    }

    /// The spend's open card: a page of the Spend tab or a step of the
    /// transaction.
    fn spend_step(&self) -> String {
        let step_of = |n: u8| crate::family_text::title(self, CardId::Step(n));
        let spend_open = || {
            let s = self.spend.as_ref()?;
            s.open
                .or_else(|| s.steps.iter().copied().find(|&n| !s.done[n as usize]))
                .map(step_of)
        };
        if self.flow_left_on(Going::Spend) == Some(Screen::Spend) && self.spend.is_some() {
            return spend_open().unwrap_or_else(|| EN.sidebar_flow_spend.to_string());
        }
        match self.family.open {
            Some(Open::Page(p)) => crate::family_text::title(self, CardId::Page(p)),
            Some(Open::Spend) => spend_open().unwrap_or_else(|| EN.sidebar_flow_spend.to_string()),
            None => spend_open().unwrap_or_else(|| {
                crate::family_text::title(self, CardId::Page(crate::family::page::HOLDING))
            }),
        }
    }
}

/// The Cancel sheet over flow `g` (F51): its question, the one line of
/// what is lost, then **Cancel the flow** and **Keep going**. Reads only.
pub(crate) fn cancel_sheet(app: &View, ui: &mut crate::ui::Ui, g: Going, w: f32, h: f32) {
    use crate::Action;
    use crate::compact::{buttons, scroll_sheet};
    use crate::ui::pal::*;
    use crate::ui::{Style, W};
    use osk_ui::widgets::Icon;
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
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Warning, WARN, title);
        } else {
            ui.icon(x, cy, 30.0, Icon::Warning, 18.0, WARN);
            let t = ui.fit(20.0, W::S, title, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        cy += ui.wrap(x, cy, iw, 14.0, W::R, TEXT, line) + 18.0;
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
