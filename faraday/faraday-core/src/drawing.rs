//! What a frame works out once.
//!
//! A frame asks the backup's checklist the same questions many times: the
//! items, whether each is done, the plan's shape, the seeds, the held keys
//! and the public file names, a stick's among them, what a lock keeps and
//! wipes, and the sticks to write; and for every screen, what the Inbox
//! gives, every wallet known with its backup, and each open vault's rows.
//! While [`crate::Faraday::render`] runs the answers are kept here;
//! nothing is kept outside it. Only drawing reads
//! them: a decision (a press gated, a hold released) computes from the
//! state as it is (`docs/SPEED.md` §2 rule 3).
//!
//! What these answers are put together from is kept on the state, read
//! once from what it describes (a plan's map, a vault's wallet records,
//! an Inbox file's wallet, an address): a fill here puts parts together
//! and parses nothing again. Costly work reaches a frame only through
//! the view's token, in a memo fill or a kept value's first working out.
//!
//! Drawing reads the app through a [`View`] (`docs/COMPUTE.md` §4): the
//! memo's fills, the kept values and the owner's exceptions are its
//! accessors in `drawing/` (`memo.rs`, `kept.rs`, `allowed.rs`), and what
//! a pass writes is noted in its [`After`] for `render` to apply. The
//! view holds the one token drawing can reach, private, and its
//! accessors alone hand it to the work they do.
//! Decisions use each memo answer's `_now` form on [`Faraday`].

use std::cell::{OnceCell, RefCell};
use std::ops::Deref;

use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;

use crate::{Faraday, Screen, flow, plan, work};

mod allowed;
pub(crate) use allowed::Allowed;
mod kept;
pub(crate) use kept::Kept;
mod memo;

/// A wallet's seeds: a name each and the session key that holds it.
pub(crate) type SeedList = Vec<(String, Option<usize>)>;

/// A stick the wallet being backed up is written to: its place, the files
/// it will hold, and the stick's label once written
/// ([`crate::Faraday::wallet_stick_rows_now`]).
pub type StickRow = (String, Vec<String>, Option<String>);

/// The answers kept for one frame.
#[derive(Default)]
pub(crate) struct Drawing {
    /// What `backup_items` returned.
    pub items: OnceCell<Vec<u8>>,
    /// What `backup_item_done` returned for each item asked.
    pub done: RefCell<Vec<(plan::Item, bool)>>,
    /// `plan_shape` per wallet.
    pub shapes: RefCell<Vec<(usize, plan::Shape)>>,
    /// `backup_seed_list` per wallet.
    pub seed_lists: RefCell<Vec<(usize, SeedList)>>,
    /// `held_keys`.
    pub held: OnceCell<Vec<Fingerprint>>,
    /// `backup_public_names_for` per wallet, for the backup's own answers.
    pub public_names: RefCell<Vec<(usize, Vec<String>)>>,
    /// The backup board as the frame draws it
    /// ([`crate::Faraday::board_view_now`], `docs/BACKUP-FLOW.md` §17).
    pub board: OnceCell<Option<std::rc::Rc<crate::board::BoardView>>>,
    /// What Create's Keys card offers from the vaults
    /// ([`View::create_vaults`], `docs/history/NEW-WALLET.md` §16.1).
    pub create_vaults: OnceCell<std::rc::Rc<crate::create_from::CreateVaults>>,
    /// The flows in progress ([`crate::Faraday::flows_in_progress_now`],
    /// `docs/FLOWS.md`, Flows in progress).
    pub flows: OnceCell<std::rc::Rc<Vec<crate::in_progress::Row>>>,
    /// What a lock keeps and wipes, by `power_off`
    /// ([`crate::Faraday::kept_wiped_now`]).
    pub kept_wiped: RefCell<Vec<(bool, crate::vault::KeptWiped)>>,
    /// The sticks the wallet being backed up is written to
    /// ([`crate::Faraday::wallet_stick_rows_now`]).
    pub stick_rows: OnceCell<Vec<StickRow>>,
    /// The Load sheet an unlock shows ([`crate::Faraday::vault_load_view_now`],
    /// F62).
    pub load_sheet: OnceCell<Option<std::rc::Rc<crate::vault::load::LoadView>>>,
    /// The seeds open vaults keep without a passphrase
    /// ([`crate::Faraday::vault_passphrase_seeds_now`], F62).
    pub passphrase_seeds: OnceCell<std::rc::Rc<Vec<crate::vault::load::PassSeed>>>,
    /// Every wallet known and where its backup is
    /// ([`crate::Faraday::backups_now`]).
    pub backups: OnceCell<Vec<crate::backups::Entry>>,
    /// The keys and wallets each open vault holds, by its place among
    /// the open vaults ([`crate::Faraday::vault_rows_now`]).
    pub vault_rows: RefCell<Vec<(usize, Vec<crate::vault::VaultRow>)>>,
    /// Everything the Inbox gives ([`crate::Faraday::inbox_found_now`]).
    pub inbox_found: OnceCell<crate::inbox::Found>,
    /// The session's keys and wallets no open vault holds
    /// ([`crate::Faraday::unsaved_now`]).
    pub unsaved: OnceCell<(Vec<String>, Vec<String>)>,
    /// What the boot stick brought, counted
    /// ([`crate::Faraday::import_count_now`]).
    pub import_count: OnceCell<Option<crate::boot_import::ImportCount>>,
}

impl Drawing {
    /// Empties every cell. Cheap.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// What drawing sees of the app: the state, read only, and what a frame
/// works out, through the named accessors in `drawing/`: the memo's
/// fills (`memo.rs`), values kept on the state (`kept.rs`) and the
/// owner's `Allowed` exceptions (`allowed.rs`) (`docs/COMPUTE.md` §4).
/// It derefs to [`Faraday`], never to `&mut Faraday`: what a pass writes
/// (a scroll clamped, another frame asked for) it notes in its [`After`],
/// which `render` applies once the pass is drawn.
///
/// What the types hold (`docs/COMPUTE.md` §9). Each example that must
/// not compile sits beside one that must, differing in the one thing
/// refused: rustdoc here does not check the error code a `compile_fail`
/// example names, so the compiling twin is what shows the refusal is the
/// one meant and not a typo.
///
/// Event code holding the app to change it mints a token and does
/// costly work with it:
///
/// ```
/// use faraday_core::Faraday;
/// use faraday_bip::policy::WalletPolicy;
/// fn event(app: &mut Faraday, text: &str) {
///     let c = app.compute();
///     let _ = WalletPolicy::parse(&c, text);
/// }
/// ```
///
/// Drawing holds the view, which mints nothing:
///
/// ```compile_fail,E0596
/// fn draw(v: &faraday_core::View<'_>) {
///     let c = v.compute();
/// }
/// ```
///
/// nor does the app read only:
///
/// ```compile_fail,E0596
/// fn draw(app: &faraday_core::Faraday) {
///     let c = app.compute();
/// }
/// ```
///
/// Costly work takes a token, so a call without one does not compile:
///
/// ```compile_fail,E0061
/// fn draw(_v: &faraday_core::View<'_>, text: &str) {
///     let _ = faraday_bip::policy::WalletPolicy::parse(text);
/// }
/// ```
///
/// The view's own token is private to its accessors in `drawing/`,
///
/// ```compile_fail,E0616
/// fn draw(v: &faraday_core::View<'_>) {
///     let c = &v.fill;
/// }
/// ```
///
/// while the state it derefs to is read as it is:
///
/// ```
/// fn draw(v: &faraday_core::View<'_>) -> faraday_core::Screen {
///     v.screen
/// }
/// ```
#[doc(hidden)]
pub struct View<'a> {
    app: &'a Faraday,
    /// The token the accessors in `drawing/` hand to the work they fill,
    /// keep or are allowed; private, so no screen can name it.
    fill: Compute,
    after: RefCell<After>,
}

impl Deref for View<'_> {
    type Target = Faraday;

    fn deref(&self) -> &Faraday {
        self.app
    }
}

impl<'a> View<'a> {
    /// The view one drawing pass reads, with its token. Cheap.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: View's token for its memo fills, kept values and Allowed entries (docs/COMPUTE.md §3)"
    )]
    pub(crate) fn new(app: &'a Faraday) -> Self {
        View {
            app,
            fill: Compute::new(work::core_count),
            after: RefCell::new(After::default()),
        }
    }

    /// A memo fill (`memo.rs`): `f` given the token. Costs what `f`
    /// costs.
    fn filled<T>(&self, f: impl FnOnce(&Compute) -> T) -> T {
        f(&self.fill)
    }

    /// A kept value reached while drawing (`kept.rs`): `f` given the
    /// token. Costs what `f` costs.
    fn kept<T>(&self, f: impl FnOnce(&Compute) -> T) -> T {
        f(&self.fill)
    }

    /// One of the owner's exceptions (`allowed.rs`): `f` given the
    /// token; `why` names the entry in the owner's list. Costs what `f`
    /// costs.
    fn allowed<T>(&self, why: Allowed, f: impl FnOnce(&Compute) -> T) -> T {
        let _ = why;
        f(&self.fill)
    }

    /// What the pass noted for after it. Cheap.
    pub(crate) fn into_after(self) -> After {
        self.after.into_inner()
    }

    /// A step column's scroll as the pass leaves it: set after the pass,
    /// with another frame asked for when `again`.
    pub(crate) fn scroll(&self, column: Scrolled, to: flow::Scroll, again: bool) {
        let mut a = self.after.borrow_mut();
        a.scrolls.push((column, to));
        a.redraw |= again;
    }

    /// The Learn sheet's end and its scroll as drawn, set after the pass.
    pub(crate) fn learn_scroll(&self, max: f32, scroll: f32) {
        self.after.borrow_mut().learn = Some((max, scroll));
    }

    /// The word list's end and its scroll as drawn, set after the pass,
    /// its follow done.
    pub(crate) fn wordlist_scroll(&self, max: f32, scroll: f32) {
        self.after.borrow_mut().wordlist = Some((max, scroll));
    }

    /// How far the wide layout draws the page up under its keyboard
    /// ([`Faraday::osk_lift_now`]), kept after the pass for the caret.
    /// Cheap.
    pub(crate) fn osk_lift(&self, visible: f32, f: f32) -> f32 {
        let lift = self.app.osk_lift_now(visible, f);
        self.after.borrow_mut().lift = Some(lift);
        lift
    }
}

/// A step column whose scroll a drawing pass sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scrolled {
    /// The backup flow open.
    BackupFlow,
    /// Silent payments.
    Silent,
    /// BIP-85.
    Bip85,
    /// New key.
    KeyGen,
    /// Vanity.
    Vanity,
    /// Create a vault.
    CreateVault,
    /// A spend.
    Spend,
    /// A wallet's backup.
    Backup,
    /// Sign a message.
    Message,
    /// Create a wallet.
    Create,
    /// Restore a wallet.
    Restore,
    /// The Spend tab.
    Family,
}

/// What a drawing pass writes, applied by `render` after the pass
/// ([`Faraday::apply_after`]).
#[derive(Default)]
pub(crate) struct After {
    /// Another frame wanted.
    pub redraw: bool,
    /// Step columns' scrolls, in the order drawn.
    pub scrolls: Vec<(Scrolled, flow::Scroll)>,
    /// The Learn sheet's end and scroll.
    pub learn: Option<(f32, f32)>,
    /// The word list's end and scroll.
    pub wordlist: Option<(f32, f32)>,
    /// How far the wide layout drew the page up under its keyboard.
    pub lift: Option<f32>,
}

impl Faraday {
    /// Whether the pages are drawn one column at a time: every window
    /// but one beside the expanded rail (`docs/LAYOUT.md` §3).
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    /// Whether the window as it is now takes one-column pages: every
    /// navigation but the expanded rail; held inside the Starter, a
    /// window under the wide layout's 1280 dp. A lookup.
    pub(crate) fn one_column(&self) -> bool {
        let window = self.window();
        if self.held {
            return window.width < faraday_draw::tokens::WINDOW_HELD_WIDE;
        }
        crate::nav::Nav::of(&window, false, false) != crate::nav::Nav::ExpandedRail
    }

    /// The navigation the window gets (`docs/LAYOUT.md` §3): the menu,
    /// the bar, the rail or the expanded rail, by its class and height.
    /// A lookup.
    pub fn nav(&self) -> crate::nav::Nav {
        crate::nav::Nav::of(&self.window(), self.held, self.compact)
    }

    /// The step on screen must not be left, so the rail and the bar are
    /// not drawn for it (`docs/LAYOUT.md` §3, as F51's Cancel rule): a
    /// stick being written by the upgrade or a backup flow, or a backup
    /// flow's key being made. A lookup.
    pub fn nav_hidden(&self) -> bool {
        let upgrade =
            self.screen == Screen::Upgrade && self.upgrade().is_some_and(|u| u.writing.is_some());
        let flow = self.screen == Screen::BackupFlow
            && self
                .flows
                .on()
                .and_then(|i| self.flows.get(i))
                .is_some_and(|f| f.live.writing.is_some() || f.live.making);
        upgrade || flow
    }

    /// The window as `docs/LAYOUT.md` §2 classes it: its size in dp at
    /// the screen's own density times the person's Display scale, its
    /// class by width, and whether it is short or low. Read from the last
    /// display the shell gave; before any, a 1280 × 800 window at 160
    /// dpi. With the held rule, it decides `is_compact` and `nav`. A
    /// lookup.
    pub fn window(&self) -> faraday_draw::geom::Window {
        self.last_display.map_or(
            faraday_draw::geom::Window {
                width: faraday_draw::tokens::WINDOW_DEFAULT_W,
                height: faraday_draw::tokens::WINDOW_DEFAULT_H,
            },
            |d| {
                let f = self.pixels_per_unit_of(&d);
                faraday_draw::geom::Window {
                    width: f32::from(d.width) / f,
                    height: f32::from(d.height) / f,
                }
            },
        )
    }

    /// Pixels per design unit on display `d`: its own density (160 dpi
    /// is one pixel a unit) times the person's Display scale, never
    /// scaled to fit a page (`docs/LAYOUT.md` §7, F101). A lookup.
    pub(crate) fn pixels_per_unit_of(&self, d: &faraday_shell_api::DisplayInfo) -> f32 {
        let density = f32::from(d.dpi.max(1)) / 160.0;
        (density * f32::from(self.scale_pct()) / 100.0).max(0.5)
    }

    /// Writes what a drawing pass noted. Cheap.
    pub(crate) fn apply_after(&mut self, a: After) {
        for (column, to) in a.scrolls {
            match column {
                Scrolled::BackupFlow => {
                    self.flows.set_scroll(to);
                }
                Scrolled::Silent => {
                    self.silent.set_scroll(to);
                }
                Scrolled::Bip85 => {
                    self.bip85.set_scroll(to);
                }
                Scrolled::KeyGen => {
                    self.keygen.set_scroll(to);
                }
                Scrolled::Vanity => {
                    self.vanity.set_scroll(to);
                }
                Scrolled::CreateVault => {
                    self.vaults.set_create_scroll(to);
                }
                Scrolled::Spend => {
                    if let Some(s) = self.wallets.spend_mut() {
                        s.scroll = to.y;
                        s.follow = to.follow;
                    }
                }
                Scrolled::Backup => {
                    self.backup.set_scroll(to);
                }
                Scrolled::Message => {
                    self.message.set_scroll(to);
                }
                Scrolled::Create => {
                    if let Some(c) = self.create.state_mut() {
                        c.scroll = to;
                    }
                }
                Scrolled::Restore => {
                    if let Some(r) = self.restore.state_mut() {
                        r.scroll = to;
                    }
                }
                Scrolled::Family => self.family.set_scroll(to),
            }
        }
        if let Some((max, scroll)) = a.learn {
            self.learn.set_scroll(max, scroll);
        }
        if let Some((max, scroll)) = a.wordlist {
            self.wordlist.set_scroll(max, scroll);
        }
        if let Some(lift) = a.lift {
            self.osk_set_lift(lift);
        }
        if a.redraw {
            self.out.dirty = true;
            self.out
                .commands
                .push_back(faraday_shell_api::Command::Draw);
        }
    }

    /// The strings the current frame draws, for a test to check what a
    /// title or a label actually reads.
    pub fn drawn_texts(&mut self) -> Vec<String> {
        self.drawn_text_boxes()
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    /// The strings the current frame draws, each with the box of its ink
    /// in pixels (x, y, w, h): for a test of where a line is put, such as
    /// whether a row of labels fits on one line.
    pub fn drawn_text_boxes(&mut self) -> Vec<(String, (i32, i32, i32, i32))> {
        self.drawn_ink()
            .into_iter()
            .filter_map(|i| match i.kind {
                faraday_draw::canvas::InkKind::Text { text, .. } => {
                    Some((text, (i.rect.x, i.rect.y, i.rect.w, i.rect.h)))
                }
                _ => None,
            })
            .collect()
    }

    /// Everything the current frame draws that a person reads as one
    /// thing (a line of text, an icon, a progress indicator, a badge's
    /// dot), each with the box of its ink in pixels: for a test of what
    /// a screen shows.
    pub fn drawn_ink(&mut self) -> Vec<faraday_draw::canvas::Ink> {
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(true);
        }
        self.before_frame();
        self.render();
        let ink = self
            .canvas
            .as_ref()
            .map(|c| c.ink().to_vec())
            .unwrap_or_default();
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(false);
        }
        self.out.dirty = true;
        ink
    }
}
