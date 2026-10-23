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

use crate::strings::EN;
use crate::{Action, Faraday, Screen, Sheet, vault::VaultAction};

/// The most screens the trail keeps; the oldest goes first.
const MAX: usize = 32;

/// The most screens kept for the forward button.
const FORWARD_MAX: usize = 8;

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
/// page back to the card, a vault item's page back to the vault's list.
/// Such a way back is the screen's own step, not a way out of it.
pub(crate) fn stays(a: Action) -> bool {
    matches!(
        a,
        Action::Glance(false) | Action::Vault(VaultAction::ItemBack)
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
            wallet: self.wallet,
            glance: self.glance,
            list_offset: self.list_offset,
            card_offset: self.card_offset,
            files_out_offset: self.files_out_offset,
            vault_item_offset: self.vault_item_offset,
            board_list_offset: self.board_list_offset,
            backups_at: self.backups_at,
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
        let (r, a) = self.hits.get(self.trail.hit()?)?;
        (Some(*a) == self.back_link()).then(|| ((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// A tap of `a` in `at`: [`Self::as_back`] when `at` is where the way
    /// back was drawn, `a` when it is another control of the same action.
    pub(crate) fn as_back_at(&self, a: Action, at: Option<faraday_draw::Rect>) -> Action {
        let on_link = self
            .trail
            .hit()
            .and_then(|i| self.hits.get(i))
            .is_some_and(|(r, _)| Some(*r) == at);
        if on_link { self.as_back(a) } else { a }
    }

    /// A person's press of `a`: [`Action::Back`] when `a` is the way out
    /// of the screen the last frame drew, else `a`. Only lookups.
    pub(crate) fn as_back(&self, a: Action) -> Action {
        let link = self.trail.is_link(self.screen, a);
        // A held app's strip goes to its holder's Home, whatever was seen
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
        self.renaming = None;
        self.chart_focus = None;
        self.list_offset = s.list_offset;
        self.card_offset = s.card_offset;
        self.files_out_offset = s.files_out_offset;
        self.vault_item_offset = s.vault_item_offset;
        self.board_list_offset = s.board_list_offset;
        match s.screen {
            Screen::Wallets => {
                self.wallet = s.wallet;
                self.glance = s.glance;
            }
            Screen::Backups => self.backups_at = s.backups_at,
            Screen::VaultContents => {
                self.vaults.current = s.vault_current;
                self.vaults.category = s.vault_category;
                self.vaults.category_chosen = s.vault_category_chosen;
                self.vaults.item = s.vault_item;
                self.vaults.item_open = s.vault_item_open;
            }
            _ => {}
        }
        if s.screen != Screen::Wallets {
            self.glance = false;
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
}
