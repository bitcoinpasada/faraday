//! Where Back lands (F77): exactly the screen seen before the one being
//! left.
//!
//! Every input that takes the person from one screen to another puts the
//! screen it left on the trail, as it was seen: which wallet's card, the
//! chart or the card, how far each list was scrolled, which vault and
//! kind, the Learn sheet over it. Back, a flow's Cancel and the Done of a
//! flow that returns to where it was opened from take the last screen on
//! the trail that can still be shown and put it back as it was. A screen
//! reached again by any other way cuts the trail where it was, so the
//! trail never holds the screen on show; Home empties it.
//!
//! A press on the way back a screen draws is Back: its own action runs
//! first, for what it lets go, then the trail decides where it lands.
//! What the way back is called is the name of where it lands, worked out
//! from the trail with lookups only.
//!
//! Within a flow with steps, Back and Forward move between its steps
//! (F104): Back opens the step before the open one, with its choices as
//! they were, and Forward opens again the step Back left. Each flow plugs
//! in through [`StepFlow`]: its steps in order, the one open, the first
//! Back may open once something is made, whether the open step is
//! running, the press that opens a step and the step's name. The first
//! step's Back leaves the flow, as above. A choice that decides a step
//! moves the flow on through the same plug (F103).

use crate::strings::EN;
use crate::{Action, Faraday, Screen, Sheet, vault::VaultAction};

/// The most screens the trail keeps; the oldest goes first.
const MAX: usize = 32;

/// The most screens kept for the forward button.
const FORWARD_MAX: usize = 8;

/// The most steps kept for the forward button within a flow.
const STEPS_MAX: usize = 16;

/// The trail and the mouse's forward button: the screens left for
/// another, the way back the last frame drew, and the screens the back
/// button left. Holds no other area's state; whether a screen can be
/// shown is asked of the glue on [`Faraday`], which is given here as a
/// closure.
#[derive(Default)]
pub(crate) struct Trail {
    /// The screens left for another, each as it was seen, the last left
    /// last: where Back lands.
    trail: Vec<Spot>,
    /// The screen as the input being handled found it, until the input
    /// is done.
    entered: Option<Spot>,
    /// The input being handled went back: the screen it left is not
    /// put on the trail.
    backing: bool,
    /// Screens left by the mouse's back button, the last left last
    /// (`docs/history/NEW-WALLET.md` §13.1).
    forward: Vec<Screen>,
    /// The screen the last back or forward button landed on.
    landed: Option<Screen>,
    /// The screen's back link, as the last frame drew it, with the
    /// screen it was drawn on: what [`Action::Back`] does first.
    back_link: Option<(Screen, Action)>,
    /// Where in `hits` the last frame drew the back link's press.
    back_hit: Option<usize>,
    /// Steps Back left within the flow on show, the last left last:
    /// where Forward goes (F104).
    step_forward: Vec<u8>,
    /// The flow's screen and the step Back or Forward last opened in it:
    /// Forward goes on only while that step is still the one open.
    step_landed: Option<(Screen, u8)>,
}

impl Trail {
    /// Notes `here` as the screen the input being handled found, once
    /// per input.
    fn note(&mut self, here: impl FnOnce() -> Spot) {
        if self.entered.is_none() {
            self.entered = Some(here());
        }
    }

    /// Forgets the screen noted, as a new process starts over.
    pub(crate) fn forget_entered(&mut self) {
        self.entered = None;
    }

    /// After an input that ended on `screen`: the screen it left goes on
    /// the trail, unless it went back. A screen reached again cuts the
    /// trail where it was; Home empties it.
    fn note_done(&mut self, screen: Screen) {
        let backing = std::mem::take(&mut self.backing);
        let Some(before) = self.entered.take() else {
            return;
        };
        if backing || before.screen == screen {
            return;
        }
        if screen == Screen::Home {
            self.trail.clear();
            return;
        }
        if let Some(i) = self.trail.iter().position(|s| s.screen == screen) {
            self.trail.truncate(i);
            return;
        }
        if self.trail.len() == MAX {
            self.trail.remove(0);
        }
        self.trail.push(before);
    }

    /// Keeps the way back the frame just drawn on `screen`, and where
    /// its press is.
    pub(crate) fn set_link(&mut self, screen: Screen, link: Option<Action>, hit: Option<usize>) {
        self.back_link = link.map(|a| (screen, a));
        self.back_hit = hit;
    }

    /// The way back drawn on `screen`, by the press it offers.
    fn link_on(&self, screen: Screen) -> Option<Action> {
        self.back_link.filter(|(s, _)| *s == screen).map(|(_, a)| a)
    }

    /// Where in `hits` the way back's press is.
    fn hit(&self) -> Option<usize> {
        self.back_hit
    }

    /// Whether `a` is the way back drawn on `screen`.
    fn is_link(&self, screen: Screen, a: Action) -> bool {
        self.back_link == Some((screen, a))
    }

    /// Takes the last screen off the trail.
    fn pop(&mut self) -> Option<Spot> {
        self.trail.pop()
    }

    /// The last screen on the trail that `shows` accepts.
    fn last_shown(&self, shows: impl Fn(&Spot) -> bool) -> Option<&Spot> {
        self.trail.iter().rev().find(|s| shows(s))
    }

    /// Which screens on the trail `shows` accepts.
    fn shown(&self, shows: impl Fn(&Spot) -> bool) -> Vec<bool> {
        self.trail.iter().map(shows).collect()
    }

    /// Cuts the trail at the first screen that was shown (`was`) and is
    /// not now (`now`); the input is then a way back.
    fn cut_unshown(&mut self, was: &[bool], now: &[bool]) {
        let first = was.iter().zip(now).position(|(&was, &now)| was && !now);
        if let Some(i) = first {
            self.trail.truncate(i);
        }
        self.backing = true;
    }

    /// Marks the input being handled as a way back.
    fn set_backing(&mut self) {
        self.backing = true;
    }

    /// Takes `screen` off the trail, where it is a flow just ended.
    pub(crate) fn drop_screen(&mut self, screen: Screen) {
        self.trail.retain(|s| s.screen() != screen);
    }

    /// The mouse's back button is pressed on `before`: the forward
    /// screens go unless the last landing was here.
    fn begin_mouse_back(&mut self, before: Screen) {
        if self.landed != Some(before) {
            self.forward.clear();
        }
    }

    /// The mouse's back button left `before` for `now`: it can be opened
    /// again with the forward button.
    fn end_mouse_back(&mut self, before: Screen, now: Screen) {
        if now != before {
            if self.forward.len() == FORWARD_MAX {
                self.forward.remove(0);
            }
            self.forward.push(before);
            self.landed = Some(now);
        }
    }

    /// The screen the forward button goes to from `screen`, while
    /// nothing else has been opened since (`sheet_open`).
    fn take_forward(&mut self, screen: Screen, sheet_open: bool) -> Option<Screen> {
        if self.landed != Some(screen) || sheet_open {
            self.forward.clear();
            return None;
        }
        self.forward.pop()
    }

    /// Drops the screens the forward button would go to.
    fn clear_forward(&mut self) {
        self.forward.clear();
    }

    /// The forward button landed on `screen`.
    fn landed_on(&mut self, screen: Screen) {
        self.landed = Some(screen);
    }

    /// Back left step `from` of the flow on `screen` for step `to`:
    /// Forward can open `from` again. Steps kept from another flow, or
    /// from before the person moved on by themselves, go.
    fn step_left(&mut self, screen: Screen, from: u8, to: u8) {
        if self.step_landed != Some((screen, from)) {
            self.step_forward.clear();
        }
        if self.step_forward.len() == STEPS_MAX {
            self.step_forward.remove(0);
        }
        self.step_forward.push(from);
        self.step_landed = Some((screen, to));
    }

    /// The step Forward opens in the flow on `screen`, whose open step
    /// is `open`: the one Back last left, while the step Back opened is
    /// still the one open.
    fn take_step_forward(&mut self, screen: Screen, open: Option<u8>) -> Option<u8> {
        let here = open.map(|k| (screen, k));
        if here.is_none() || self.step_landed != here {
            self.clear_steps();
            return None;
        }
        self.step_forward.pop()
    }

    /// Forward opened step `k` of the flow on `screen`.
    fn step_landed_on(&mut self, screen: Screen, k: u8) {
        self.step_landed = Some((screen, k));
    }

    /// Drops the steps Forward would open.
    fn clear_steps(&mut self) {
        self.step_forward.clear();
        self.step_landed = None;
    }
}

/// A flow's steps as Back and Forward walk them (F104), as the flow
/// stands now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Steps {
    /// The steps shown, in order, by the flow's own numbers.
    pub(crate) order: Vec<u8>,
    /// The step open, if any.
    pub(crate) open: Option<u8>,
    /// The first step Back may open: the one after a step that made
    /// something the flow cannot take back (a wallet made from its
    /// seeds, a transaction signed). Back from it leaves the flow, as
    /// from the first step.
    pub(crate) first: Option<u8>,
    /// The open step is running (a vault being made, a search): it takes
    /// no Back.
    pub(crate) running: bool,
}

impl Steps {
    /// The flow's steps in `order`, `open` open, nothing made and
    /// nothing running.
    pub(crate) fn new(order: Vec<u8>, open: Option<u8>) -> Steps {
        Steps {
            order,
            open,
            first: None,
            running: false,
        }
    }

    /// The step Back opens from the one open: the step before it, while
    /// the open step is past the first step and past the first Back may
    /// open. `None` on the first step, with no step open, or with the
    /// open step not shown: Back then leaves the flow.
    pub(crate) fn before(&self) -> Option<u8> {
        let at = self.order.iter().position(|&k| Some(k) == self.open)?;
        let floor = self
            .first
            .and_then(|f| self.order.iter().position(|&k| k == f))
            .unwrap_or(0);
        (at > floor).then(|| self.order[at - 1])
    }
}

/// A flow with steps, plugged into Back, Forward and the choices that
/// move on (F103, F104). One unit type per flow implements it, beside the
/// flow's own state; [`Faraday::step_flow`] names the one on show.
pub(crate) trait StepFlow: Sync {
    /// The flow's steps as it stands, while it is open. Only lookups.
    fn steps(&self, app: &Faraday) -> Option<Steps>;
    /// The press that opens step `k`, every choice in it as it was.
    fn open(&self, app: &Faraday, k: u8) -> Action;
    /// What step `k` is called: the arrow names where Back lands.
    fn name(&self, app: &Faraday, k: u8) -> &'static str;
    /// After `chosen` was pressed: the press that closes its step and
    /// opens the next, when `chosen` is a single choice that decides its
    /// step and the flow took it (F103). Worked out from the state as the
    /// press left it.
    fn moves_on(&self, _app: &Faraday, _chosen: Action) -> Option<Action> {
        None
    }
}

/// Whether `a` is only ever a way out of its screen, wherever it is
/// pressed: the backup flow's Leave, which lands where Back does.
fn leaves(a: Action) -> bool {
    matches!(a, Action::Flow(crate::backup_flow::steps::FlowAction::Back))
}

/// A screen as it was seen: what a way back to it puts back.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spot {
    /// The screen.
    screen: Screen,
    /// The Learn sheet was open over it, the one sheet that is put back:
    /// what it shows is kept while it is closed.
    learn: bool,
    /// The wallet whose card was shown.
    wallet: usize,
    /// The card was shown as its chart.
    glance: bool,
    /// How far each list was scrolled: the page's, the card's, the
    /// Outbox's, a vault item's and the board's.
    list_offset: f32,
    card_offset: f32,
    files_out_offset: f32,
    vault_item_offset: f32,
    board_list_offset: f32,
    /// The wallet Backups showed, a page each on a small panel.
    backups_at: usize,
    /// The open vault, the kind shown and whether it was chosen, the
    /// item shown in each kind and whether as its page.
    vault_current: usize,
    vault_category: usize,
    vault_category_chosen: bool,
    vault_item: [usize; 7],
    vault_item_open: bool,
}

/// Whether a way back that presses `a` stays on its screen: the chart's
/// page back to the card, a vault item's page back to the vault's list,
/// a Settings section's page back to the list of sections. Such a way
/// back is the screen's own step, not a way out of it.
pub(crate) fn stays(a: Action) -> bool {
    matches!(
        a,
        Action::Glance(false)
            | Action::Vault(VaultAction::ItemBack)
            | Action::SettingsSection(None)
    )
}

impl Spot {
    /// The screen.
    pub(crate) fn screen(&self) -> Screen {
        self.screen
    }
}

impl Faraday {
    /// The screen on show as it is seen now.
    pub(crate) fn spot(&self) -> Spot {
        Spot {
            screen: self.screen,
            learn: self.sheet == Some(Sheet::Learn),
            wallet: self.wallets.wallet(),
            glance: self.glance.open(),
            list_offset: self.list_offset,
            card_offset: self.wallets.card_offset(),
            files_out_offset: *self.visit.files_out_offset(),
            vault_item_offset: self.vaults.item_offset(),
            board_list_offset: self.flows.board_list_offset(),
            backups_at: self.backups_area.at(),
            vault_current: self.vaults.current,
            vault_category: self.vaults.category,
            vault_category_chosen: self.vaults.category_chosen,
            vault_item: self.vaults.item,
            vault_item_open: self.vaults.item_open,
        }
    }

    /// Whether `s` can still be shown as it was seen: the flow it is a
    /// step of is not ended, the wallet or vault it showed is still
    /// here. Only lookups.
    fn shows(&self, s: &Spot) -> bool {
        self.can_show(s.screen)
            && match s.screen {
                Screen::Wallets => s.wallet < self.session.wallets.len(),
                Screen::VaultContents => s.vault_current < self.vaults.open.len(),
                _ => true,
            }
    }

    /// Notes the screen as the input now being handled finds it, once
    /// per input: what goes on the trail if the input leaves it.
    pub(crate) fn note_here(&mut self) {
        let here = self.spot();
        self.trail.note(|| here);
    }

    /// After an input: the screen it left goes on the trail, unless it
    /// went back. A screen reached again cuts the trail where it was;
    /// Home empties it.
    pub(crate) fn trail_note(&mut self) {
        self.trail.note_done(self.screen);
    }

    /// The way back the last frame drew on the screen on show, by the
    /// press it offers: for tests that tap it.
    pub fn back_link(&self) -> Option<Action> {
        self.trail.link_on(self.screen)
    }

    /// Where the last frame drew the way back on the screen on show, in
    /// pixels: the middle of its press, for tests that tap it.
    pub fn back_offered(&self) -> Option<(u16, u16)> {
        let (r, a) = self.pointer.hits().get(self.trail.hit()?)?;
        (Some(*a) == self.back_link()).then(|| ((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// A tap of `a` in `at`: [`Self::as_back`] when `at` is where the way
    /// back was drawn, `a` when it is another control of the same action.
    pub(crate) fn as_back_at(&self, a: Action, at: Option<faraday_draw::Rect>) -> Action {
        let on_link = self
            .trail
            .hit()
            .and_then(|i| self.pointer.hits().get(i))
            .is_some_and(|(r, _)| Some(*r) == at);
        if on_link || leaves(a) {
            self.as_back(a)
        } else {
            a
        }
    }

    /// A person's press of `a`: [`Action::Back`] when `a` is the way out
    /// of the screen the last frame drew, else `a`. Only lookups.
    pub(crate) fn as_back(&self, a: Action) -> Action {
        let link = self.trail.is_link(self.screen, a);
        // A held app's way home goes to its holder, whatever was seen
        // before.
        let held_home = self.held && a == Action::Nav(Screen::Home);
        if link && self.sheet.is_none() && !stays(a) && !held_home {
            Action::Back
        } else {
            a
        }
    }

    /// **Back**: the screen's own way back first, for what it lets go,
    /// then the last screen on the trail that can still be shown, as it
    /// was seen. A way back that opened a sheet or stayed on its screen
    /// is all there is to it.
    pub(crate) fn back(&mut self) {
        if self.step_back() {
            return;
        }
        let own = self
            .trail
            .link_on(self.screen)
            .filter(|a| self.sheet.is_none() && *a != Action::Back);
        let from = self.screen;
        if let Some(a) = own {
            self.act_inner(a);
            if self.screen == from {
                return;
            }
        }
        self.trail.set_backing();
        self.land_back();
    }

    /// Lands on the last screen on the trail that can still be shown,
    /// as it was seen; the screens past it go. Returns whether there was
    /// one.
    pub(crate) fn land_back(&mut self) -> bool {
        while let Some(s) = self.trail.pop() {
            if self.shows(&s) {
                self.land(s);
                self.trail.set_backing();
                return true;
            }
        }
        false
    }

    /// A flow that goes on to `to` on its Done: `to` as it was seen when
    /// it is the screen before, else `to` (Set the time opened on the way
    /// into the codes goes on to them).
    pub(crate) fn return_back(&mut self, to: Screen) {
        let before = self.trail.last_shown(|s| self.shows(s));
        if before.is_some_and(|s| s.screen == to) {
            self.land_back();
        } else {
            self.screen = to;
        }
        self.trail.set_backing();
    }

    /// Which screens on the trail can be shown now, for [`Self::cancel_landing`].
    pub(crate) fn trail_shown(&self) -> Vec<bool> {
        self.trail.shown(|s| self.shows(s))
    }

    /// A flow ended by its Cancel: back to the screen before its first
    /// step on the trail, the first that could be shown before (`was`)
    /// and cannot now; before the screen on show when none is.
    pub(crate) fn cancel_landing(&mut self, was: &[bool]) {
        let now = self.trail_shown();
        self.trail.cut_unshown(was, &now);
        self.land_back();
    }

    /// Puts `s` back on show as it was seen.
    fn land(&mut self, s: Spot) {
        if self.screen != s.screen && self.sheet != Some(Sheet::Learn) {
            // A sheet the way back opened belongs to where it went.
            self.sheet = None;
        }
        self.screen = s.screen;
        if s.learn {
            self.sheet = Some(Sheet::Learn);
        }
        self.osk_leave();
        self.wallets.set_renaming(None);
        self.glance.set_focus(None);
        self.list_offset = s.list_offset;
        self.wallets.set_card_offset(s.card_offset);
        *self.visit.files_out_offset_mut() = s.files_out_offset;
        self.vaults.set_item_offset(s.vault_item_offset);
        self.flows.set_board_list_offset(s.board_list_offset);
        match s.screen {
            Screen::Wallets => {
                self.wallets.set_wallet(s.wallet);
                self.glance.set_open(s.glance);
            }
            Screen::Backups => self.backups_area.set_at(s.backups_at),
            Screen::VaultContents => {
                self.vaults.restore_view(
                    s.vault_current,
                    s.vault_category,
                    s.vault_category_chosen,
                    s.vault_item,
                    s.vault_item_open,
                );
            }
            _ => {}
        }
        if s.screen != Screen::Wallets {
            self.glance.set_open(false);
        }
    }

    /// The name of where Back lands from here: the last screen on the
    /// trail that can still be shown, a wallet's card by the wallet's
    /// name. Only lookups.
    pub(crate) fn trail_name(&self) -> Option<&str> {
        let s = self.trail.last_shown(|s| self.shows(s))?;
        Some(match s.screen {
            Screen::Wallets => self
                .session
                .wallets
                .get(s.wallet)
                .map_or(EN.compact_wallets, |w| w.name.as_str()),
            other => self.back_name(other),
        })
    }

    /// The way back a screen draws, its own label and press: named by
    /// where it lands when it leaves the screen, by its own label when it
    /// is a step within it or the trail holds nothing. Only lookups.
    pub(crate) fn named_back<'a>(&'a self, back: (&'a str, Action)) -> (&'a str, Action) {
        let (label, own) = back;
        if stays(own) {
            return back;
        }
        if let Some(step) = self.step_back_name() {
            return (step, own);
        }
        (self.trail_name().unwrap_or(label), own)
    }

    /// The mouse's back button: the screen's back link, or Escape; a
    /// screen it leaves can be opened again with the forward button.
    pub(crate) fn mouse_back(&mut self) {
        let before = self.screen;
        self.trail.begin_mouse_back(before);
        let link = self.trail.link_on(self.screen);
        match (self.sheet, link) {
            // A sheet's own way out, as Escape takes it.
            (Some(Sheet::Lock), _) => {}
            (Some(_), _) => self.act(Action::Cancel),
            // Learn's supporting pane goes first, as Escape takes it.
            (None, _) if self.learn_pane_shown() => self.learn_close(),
            // Out of the screen, where the link lands (F77); a step
            // within it, as the link takes it.
            (None, Some(a)) => self.act(self.as_back(a)),
            (None, None) => self.key(faraday_shell_api::Key::Escape),
        }
        self.trail.end_mouse_back(before, self.screen);
    }

    /// The mouse's forward button: the screen the back button last left,
    /// while nothing else has been opened since and it can still be
    /// shown as it was left.
    pub(crate) fn mouse_forward(&mut self) {
        if self.step_forward() {
            return;
        }
        let Some(s) = self.trail.take_forward(self.screen, self.sheet.is_some()) else {
            return;
        };
        if !self.can_show(s) {
            self.trail.clear_forward();
            return;
        }
        self.act(Action::Nav(s));
        self.trail.landed_on(self.screen);
    }

    /// The flow with steps on show, as Back, Forward and the choices that
    /// move on see it (F103, F104). Only lookups.
    fn step_flow(&self) -> Option<&'static dyn StepFlow> {
        Some(match self.screen {
            Screen::Restore => &crate::restore::RESTORE_STEPS,
            Screen::Spend => &crate::wallet::SPEND_STEPS,
            Screen::Family => &crate::family::FAMILY_STEPS,
            Screen::Message => &crate::screens::MESSAGE_STEPS,
            Screen::CreateVault => &crate::vault::CREATE_VAULT_STEPS,
            Screen::Bip85 => &crate::bip85::BIP85_STEPS,
            Screen::Silent => &crate::silent::SILENT_STEPS,
            Screen::Vanity => &crate::vanity::VANITY_STEPS,
            Screen::KeyGen => &crate::keygen::steps::KEYGEN_STEPS,
            _ => return None,
        })
    }

    /// The flow on show and its steps, with no sheet over it. Only
    /// lookups.
    fn steps_on_show(&self) -> Option<(&'static dyn StepFlow, Steps)> {
        if self.sheet.is_some() {
            return None;
        }
        let flow = self.step_flow()?;
        Some((flow, flow.steps(self)?))
    }

    /// The name of the step Back opens from here, while it opens one.
    /// Only lookups.
    fn step_back_name(&self) -> Option<&'static str> {
        let (flow, steps) = self.steps_on_show()?;
        Some(flow.name(self, steps.before()?))
    }

    /// **Back** within the flow on show (F104): the step before the open
    /// one opens, its choices as they were, and Forward can open the one
    /// left again. A running step takes no Back. Returns whether Back was
    /// taken here; on the first step it is not, and Back leaves the flow.
    fn step_back(&mut self) -> bool {
        let Some((flow, steps)) = self.steps_on_show() else {
            return false;
        };
        if steps.running {
            return true;
        }
        let (Some(from), Some(to)) = (steps.open, steps.before()) else {
            return false;
        };
        self.act_inner(flow.open(self, to));
        if flow.steps(self).and_then(|s| s.open) == Some(to) {
            self.trail.step_left(self.screen, from, to);
        } else {
            self.trail.clear_steps();
        }
        true
    }

    /// **Forward** within the flow on show (F104): the step Back last
    /// left opens again, as it was, while the step Back opened is still
    /// the one open. Returns whether one opened.
    fn step_forward(&mut self) -> bool {
        let Some((flow, steps)) = self.steps_on_show() else {
            self.trail.clear_steps();
            return false;
        };
        if steps.running {
            return false;
        }
        let Some(to) = self.trail.take_step_forward(self.screen, steps.open) else {
            return false;
        };
        self.act(flow.open(self, to));
        if flow.steps(self).and_then(|s| s.open) == Some(to) {
            self.trail.step_landed_on(self.screen, to);
        } else {
            self.trail.clear_steps();
        }
        true
    }

    /// A press, then, where it was a single choice that decides its step,
    /// the press that moves the flow on (F103). A press that left the
    /// flow's screen or opened a sheet moves nothing on.
    pub(crate) fn act_moving_on(&mut self, action: Action) {
        let screen = self.screen;
        self.act_inner(action);
        if self.screen != screen || self.sheet.is_some() {
            return;
        }
        let Some(next) = self.step_flow().and_then(|f| f.moves_on(self, action)) else {
            return;
        };
        self.act_inner(next);
    }
}
