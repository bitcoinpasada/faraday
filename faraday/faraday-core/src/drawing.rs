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
use crate::{Sheet, drawing, motion, screens, ui};
use faraday_draw::Canvas;
use faraday_shell_api::Command;
use faraday_shell_api::DisplayInfo;
pub(crate) use kept::{CardSplit, Kept, SplitKey};
mod memo;

/// A wallet's seeds: a name each and the session key that holds it.
pub(crate) type SeedList = Vec<(faraday_text::Text, Option<usize>)>;

/// A stick the wallet being backed up is written to: its place, the files
/// it will hold, and the stick's label once written
/// ([`crate::Faraday::wallet_stick_rows_now`]).
pub type StickRow = (
    faraday_text::Text,
    Vec<faraday_text::Text>,
    Option<faraday_text::Text>,
);

/// The answers kept for one frame.
#[derive(Default)]
pub(crate) struct Drawing {
    /// What `backup_items` returned.
    pub items: OnceCell<Vec<crate::plan::Item>>,
    /// What `backup_item_done` returned for each item asked.
    pub done: RefCell<Vec<(plan::Item, bool)>>,
    /// `plan_shape` per wallet.
    pub shapes: RefCell<Vec<(usize, plan::Shape)>>,
    /// `backup_seed_list` per wallet.
    pub seed_lists: RefCell<Vec<(usize, SeedList)>>,
    /// `wallets_needing`, once a frame for the sidebar's badge and Home's box.
    pub needing: OnceCell<std::rc::Rc<Vec<usize>>>,
    /// `held_keys`.
    pub held: OnceCell<Vec<Fingerprint>>,
    /// `backup_public_names_for` per wallet, for the backup's own answers.
    pub public_names: RefCell<Vec<(usize, Vec<faraday_text::Text>)>>,
    /// The backup board as the frame draws it
    /// ([`crate::Faraday::board_view_now`], `docs/BACKUP-FLOW.md` §17).
    pub board: OnceCell<Option<std::rc::Rc<crate::board::BoardView>>>,
    /// What Create's Keys card offers from the vaults
    /// ([`View::create_vaults`], `docs/WALLETS.md` §14).
    pub create_vaults: OnceCell<std::rc::Rc<crate::create_from::CreateVaults>>,
    /// The Inbox files Create's Keys card lists, offered or refused
    /// ([`crate::Faraday::create_key_files_now`]).
    pub create_key_files: OnceCell<std::rc::Rc<Vec<crate::create::KeyFile>>>,
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
    pub backups: OnceCell<std::rc::Rc<Vec<crate::backups::Entry>>>,
    /// How many wallets are known and how many have no map
    /// ([`crate::Faraday::backup_counts_now`]).
    pub backup_counts: OnceCell<(usize, usize)>,
    /// The keys and wallets each open vault holds, by its place among
    /// the open vaults ([`crate::Faraday::vault_rows_now`]).
    pub vault_rows: RefCell<Vec<(usize, Vec<crate::vault::VaultRow>)>>,
    /// Everything the Inbox gives ([`crate::Faraday::inbox_found_now`]).
    pub inbox_found: OnceCell<crate::inbox::Found>,
    /// Each Inbox file's row button, by its place ([`View::inbox_buttons`]).
    pub inbox_buttons: OnceCell<std::rc::Rc<Vec<crate::items::RowButton>>>,
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

    /// The page stood side by side, noted for after the pass.
    pub(crate) fn stood_apart(&self) {
        self.after.borrow_mut().apart = true;
    }

    /// The guide sheet's end and its scroll as drawn, set after the pass.
    pub(crate) fn guide_scroll(&self, max: f32, scroll: f32, section: Option<f32>) {
        self.after.borrow_mut().guide = Some((max, scroll, section));
    }

    /// The word list's end and its scroll as drawn, set after the pass,
    /// its follow done.
    pub(crate) fn wordlist_scroll(&self, max: f32, scroll: f32) {
        self.after.borrow_mut().wordlist = Some((max, scroll));
    }

    /// The sheet of pointer `id` drew its line about Enter, noted after
    /// the pass.
    pub(crate) fn enter_line(&self, id: u32) {
        self.after.borrow_mut().enter_line = Some(id);
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
    /// Check a signer.
    SignerCheck,
    /// Convert a seed.
    Convert,
    /// The Krux code tool.
    KefTool,
    OtsCheck,
    Tools,
    /// Decode a Lightning invoice's card, where it is drawn as a flow.
    Invoice,
    SeedCheck,
    MultisigCheck,
    CoreWallet,
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
    SpendBackup,
}

/// What a drawing pass writes, applied by `render` after the pass
/// ([`Faraday::apply_after`]).
#[derive(Default)]
pub(crate) struct After {
    /// Another frame wanted.
    pub redraw: bool,
    /// Step columns' scrolls, in the order drawn.
    pub scrolls: Vec<(Scrolled, flow::Scroll)>,
    /// The guide sheet's end and scroll, and where the section it was to
    /// scroll to was found.
    pub guide: Option<(f32, f32, Option<f32>)>,
    /// The word list's end and scroll.
    pub wordlist: Option<(f32, f32)>,
    /// How far the wide layout drew the page up under its keyboard.
    pub lift: Option<f32>,
    /// The pointer whose line about Enter this pass drew.
    pub enter_line: Option<u32>,
    /// The page drew its panes side by side, not stacked.
    pub apart: bool,
}

/// A string a frame draws and the box of its ink in pixels (x, y, w, h).
pub type DrawnText = (faraday_text::Text, (i32, i32, i32, i32));

impl Faraday {
    /// Whether (x, y), in pixels, is empty room in the top strip: in the
    /// top band of the window (the pills' strip when it is taller, else
    /// the top app bar's height) with no control the last frame drew
    /// under it or within its touch floor. A press there and a drag move
    /// the window, a double-click there maximises it
    /// (`docs/DESIGN.md` §5). A lookup in the hits already recorded;
    /// false under a sheet and on the failed self-test.
    pub fn is_strip_room(&self, x: i32, y: i32) -> bool {
        if self.sheet.is_some() || x < 0 || y < 0 {
            return false;
        }
        let f = self.f;
        let pills = self
            .pointer
            .hits()
            .iter()
            .filter(|(_, a)| matches!(a, crate::Action::StickPill(_)))
            .map(|(r, _)| r.y + r.h)
            .max()
            .unwrap_or(0);
        let band = (faraday_draw::tokens::TOP_BAND * f).round() as i32;
        y < band.max(pills + (faraday_draw::tokens::GAP_SMALL * f) as i32)
            && self.pointer.hit_at(x, y).is_none()
    }

    /// Whether the pages are drawn one column at a time: every window
    /// but one beside the expanded rail (`docs/LAYOUT.md` §3).
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    /// Whether the window as it is now takes one-column pages: every
    /// navigation but the expanded rail; held inside the Starter, a
    /// window under the wide layout's 1280 dp. Both counted at the
    /// person's text size ([`Faraday::layout_window`]). A lookup.
    pub(crate) fn one_column(&self) -> bool {
        let window = self.layout_window();
        if self.held {
            return window.width < faraday_draw::tokens::WINDOW_HELD_WIDE;
        }
        crate::nav::Nav::of(&window, false, false) != crate::nav::Nav::ExpandedRail
    }

    /// The navigation the window gets (`docs/LAYOUT.md` §3): the menu,
    /// the bar, the rail or the expanded rail, by its class and height
    /// at the person's text size. A lookup.
    pub fn nav(&self) -> crate::nav::Nav {
        crate::nav::Nav::of(&self.layout_window(), self.held, self.compact)
    }

    /// Whether a page's search, and Home's first actions, stand in the top
    /// app bar ([`crate::nav::Nav::search_in_bar`]). A lookup.
    pub(crate) fn search_in_bar(&self) -> bool {
        self.nav().search_in_bar(&self.layout_window())
    }

    /// The window the layout is chosen for (`docs/TEXT-SIZE.md` §4): its
    /// size in dp, with the Display scale counted ([`Faraday::window`]),
    /// divided by the text size's factor, so larger text takes the
    /// layout of a smaller window. A lookup.
    pub(crate) fn layout_window(&self) -> faraday_draw::geom::Window {
        let w = self.window();
        let t = self.text_size().factor();
        faraday_draw::geom::Window {
            width: w.width / t,
            height: w.height / t,
        }
    }

    /// The step on screen must not be left, so the rail and the bar are
    /// not drawn for it (`docs/LAYOUT.md` §3, as F51's Cancel rule): a
    /// stick being written by a backup flow, or a backup flow's key being
    /// made. A lookup.
    pub(crate) fn nav_hidden(&self) -> bool {
        self.screen == Screen::BackupFlow
            && self
                .flows
                .on()
                .and_then(|i| self.flows.get(i))
                .is_some_and(|f| f.live.writing.is_some() || f.live.making)
    }

    /// Whether the phone layout's bottom bar is left out: where
    /// [`Self::nav_hidden`] says so, and on every page the flow container
    /// draws as a step (a flow, a tool or check on it, the backup board),
    /// which is left only by its Back (`docs/LAYOUT.md` §3). The top bar
    /// keeps its ⋮ menu, Lock and Power there: this is the bottom bar's
    /// alone. A lookup.
    pub(crate) fn nav_bar_hidden(&self) -> bool {
        use crate::Screen as S;
        self.nav_hidden()
            || match self.screen {
                S::Backup => self.backup().is_some(),
                S::KeyGen => self.keygen.state().is_some(),
                S::Create
                | S::Restore
                | S::Message
                | S::Spend
                | S::Bip85
                | S::Convert
                | S::KefTool
                | S::OtsCheck
                | S::SeedCheck
                | S::MultisigCheck
                | S::CoreWallet
                | S::Silent
                | S::Vanity
                | S::CreateVault
                | S::BackupFlow
                | S::SpendBackup
                | S::SignerCheck => true,
                _ => false,
            }
    }

    /// The side of a press's touch target in units: 44 on the compact
    /// and short layout (the 2.8-inch), 48 on every other (F188); 48
    /// held inside the Starter. A lookup.
    pub(crate) fn touch_side(&self) -> f32 {
        if self.held {
            return faraday_draw::tokens::TOUCH;
        }
        faraday_draw::tokens::touch(self.window())
    }

    /// Whether notes are set one type step up: on a low window wider than
    /// the compact class, the 7-inch panel (`docs/LAYOUT.md` §8); never
    /// inside the held rule. A lookup.
    pub(crate) fn note_up(&self) -> bool {
        let w = self.window();
        !self.held && w.low() && w.class() != faraday_draw::geom::WindowClass::Compact
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
        self.input_drawn(a.enter_line);
        self.trail.drew_apart(self.screen, a.apart);
        for (column, to) in a.scrolls {
            match column {
                Scrolled::BackupFlow => {
                    self.flows.set_scroll(to);
                }
                Scrolled::Silent => {
                    self.silent.set_scroll(to);
                }
                Scrolled::SignerCheck => {
                    self.decode.set_check_scroll(to);
                }
                Scrolled::Bip85 => {
                    self.bip85.set_scroll(to);
                }
                Scrolled::Convert => {
                    self.tools.convert.set_scroll(to);
                }
                Scrolled::KefTool => {
                    self.tools.kef_tool.set_scroll(to);
                }
                Scrolled::OtsCheck => {
                    self.tools.ots.set_scroll(to);
                }
                Scrolled::Tools => {
                    self.tools.set_scroll(to);
                }
                Scrolled::Invoice => {
                    self.tools.invoice.set_scroll(to);
                }
                Scrolled::SeedCheck => {
                    self.tools.seed_check.set_scroll(to);
                }
                Scrolled::MultisigCheck => {
                    self.tools.mscheck.set_scroll(to);
                }
                Scrolled::CoreWallet => {
                    self.tools.core_wallet.set_scroll(to);
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
                    self.wallets.scroll_spend_to(to.y, to.follow);
                }
                Scrolled::Backup => {
                    self.backup.set_scroll(to);
                }
                Scrolled::Message => {
                    self.message.set_scroll(to);
                }
                Scrolled::Create => {
                    self.create.set_scroll(to);
                }
                Scrolled::Restore => {
                    self.restore.set_scroll(to);
                }
                Scrolled::SpendBackup => self.spend_backup.set_scroll(to),
            }
        }
        if let Some((max, scroll, section)) = a.guide {
            // The guide scrolled to its section: another frame shows it.
            if self.guide.set_scroll(max, scroll, section) {
                // Opened at its section, the sheet has not been scrolled
                // by anyone: the overlay scrollbar stays hidden.
                self.guide_bar_synced();
                self.out.redraw();
                self.out
                    .commands
                    .push_back(faraday_shell_api::Command::Draw);
            }
        }
        if let Some((max, scroll)) = a.wordlist {
            self.wordlist.set_scroll(max, scroll);
        }
        if let Some(lift) = a.lift {
            self.osk_set_lift(lift);
        }
        if a.redraw {
            self.out.redraw();
            self.out
                .commands
                .push_back(faraday_shell_api::Command::Draw);
        }
    }

    /// Draws the frame. Costs a full draw of the screen; the answers the
    /// backup's cards share are worked out once and kept only until it
    /// returns, by whatever way it returns. Each drawing pass reads a
    /// [`drawing::View`]; what it noted for after it is written once it
    /// is drawn ([`drawing::After`]). A whole frame empties the charts and
    /// the backup list kept across moved frames ([`Kept::charts_frame`]).
    pub(crate) fn render(&mut self, only_moved: bool) {
        self.drawing.clear();
        self.kept.charts_frame(!only_moved, &self.today());
        #[cfg(feature = "testkit")]
        let recording = crate::leaks::begin(self.canvas.as_mut());
        self.render_frame(only_moved);
        #[cfg(feature = "testkit")]
        {
            let forms = crate::leaks::end(self.canvas.as_mut(), recording);
            self.fade_marks(only_moved, forms);
        }
        self.kept.charts_done();
        self.drawing.clear();
    }

    /// The strings the current frame draws, for a test to check what a
    /// title or a label actually reads.
    pub fn drawn_texts(&mut self) -> Vec<faraday_text::Text> {
        self.drawn_text_boxes()
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    /// The strings the current frame draws, each with the box of its ink
    /// in pixels (x, y, w, h): for a test of where a line is put, such as
    /// whether a row of labels fits on one line.
    pub fn drawn_text_boxes(&mut self) -> Vec<DrawnText> {
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

    /// What the current frame draws cut off at the sides: each control
    /// and each string that runs past the left or right side of its clip
    /// (the panel's, a card's, a sheet's). What runs past the foot of a
    /// page that scrolls is not counted. A control reads as its action and
    /// its box's left and right, pixels; a string as `text "…"`. For a
    /// test that nothing is cut off at a panel's width.
    pub fn drawn_past_sides(&mut self) -> Vec<faraday_text::Text> {
        let _ = self.drawn_ink();
        self.past_sides()
    }

    /// [`Self::drawn_past_sides`] and [`Self::drawn_text_boxes`] from one
    /// frame drawn: for a test that walks a page and checks it at each
    /// place it scrolls to.
    #[cfg(feature = "testkit")]
    pub fn drawn_past_sides_and_text(&mut self) -> (Vec<faraday_text::Text>, Vec<DrawnText>) {
        let texts = self
            .drawn_ink()
            .into_iter()
            .filter_map(|i| match i.kind {
                faraday_draw::canvas::InkKind::Text { text, .. } => {
                    Some((text, (i.rect.x, i.rect.y, i.rect.w, i.rect.h)))
                }
                _ => None,
            })
            .collect();
        (self.past_sides(), texts)
    }

    /// The actions the last frame drawn offers to a press, once each, in
    /// drawing order: for a test that walks what a page offers. A lookup.
    #[cfg(feature = "testkit")]
    pub fn drawn_actions(&self) -> Vec<crate::Action> {
        let mut out = Vec::new();
        for (_, a) in self.pointer.hits() {
            if !out.contains(a) {
                out.push(*a);
            }
        }
        out
    }

    /// Each press the last frame offers, in drawing order, as (action,
    /// target, drawn, cut, grown), pixels: the pointer's targets; a hit with no
    /// target left (trimmed to a pane) as its box, marked cut. For the
    /// touch-target test over the snapshot tours (`docs/M3-CHECKS.md`
    /// §2). A lookup.
    pub fn drawn_targets(
        &self,
    ) -> Vec<(
        crate::Action,
        faraday_draw::geom::Rect,
        faraday_draw::geom::Rect,
        bool,
        faraday_draw::geom::Rect,
    )> {
        let reach = self.pointer.reach();
        let mut out: Vec<_> = reach
            .iter()
            .map(|t| (t.action, t.target, t.drawn, t.cut, t.grown))
            .collect();
        for (r, a) in self.pointer.hits() {
            if !reach.iter().any(|t| t.drawn == *r && t.action == *a) {
                out.push((*a, *r, *r, true, *r));
            }
        }
        out
    }

    /// What the frame last drawn ran past the sides of its clips.
    fn past_sides(&self) -> Vec<faraday_text::Text> {
        let Some(c) = self.canvas.as_ref() else {
            return Vec::new();
        };
        let mut past: Vec<faraday_text::Text> = c
            .cut()
            .iter()
            .map(|s| faraday_text::text!("text {:?}", s.as_str())) // not shown: a test line
            .collect();
        past.extend(
            self.pointer
                .cut()
                .iter()
                .map(|(r, a)| faraday_text::text!("{a:?} {:?}", (r.x, r.right()))), // not shown: a test line
        );
        past
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
        self.render(false);
        let ink = self
            .canvas
            .as_ref()
            .map(|c| c.ink().to_vec())
            .unwrap_or_default();
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(false);
        }
        self.out.redraw();
        ink
    }
}

impl Faraday {
    /// The frame last painted, if any.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn canvas(&self) -> Option<&Canvas> {
        self.canvas.as_ref()
    }

    /// The frame last painted, to draw on.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn canvas_mut(&mut self) -> Option<&mut Canvas> {
        self.canvas.as_mut()
    }

    /// Costs one HMAC for each shown TOTP code not held for its period
    /// ([`Self::vault_code_refresh`], [`Self::vault_codes_refresh`]), and
    /// for each vault changed since the last frame what [`Vaults::warm`]
    /// costs: a read of the free memory and a copy of the document and a
    /// count of it where it may be near the ceiling or past what is free:
    /// what drawing looks up, in step (`docs/SPEED.md` §3.3).
    pub(crate) fn before_frame(&mut self) {
        let c = self.compute();
        for o in &self.vaults.open {
            self.vaults.warm(&c, o.id);
        }
        self.backup_plan_warm(&c);
        // The Lock sheet on show names what its Lock without saving lets go.
        if self.sheet == Some(Sheet::LockAsk) {
            self.vaults.name_lock_seen();
        }
        self.visit_sign_refresh();
        self.vault_code_refresh();
        self.vault_codes_refresh();
        self.codex32_refit();
        self.guide_place();
    }

    fn render_frame(&mut self, only_moved: bool) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        // A change of screen or sheet fades in from the frame before it.
        self.fade_begin(&mut canvas);
        // The keyboard's place, while the last frame offered it: read
        // before its hits are taken for this one.
        let focused = self.pointer.focused();
        let mut hits = self.pointer.take_hits();
        hits.clear();
        // Offsets in whole pixels, whoever set them last, so a scrolled
        // page moves as one.
        let f = self.f.max(0.1);
        let whole = self.whole_offset();
        if let Some(offset) = self.scroll_offset() {
            self.set_scroll_offset(whole(offset));
        }
        let stretch = (self.scroll.stretch() * f).round() as i32;
        // A scroll step may move the rows already drawn
        // (`docs/FRAMES.md` §3.3); drawn whole when the first pass does
        // not suit.
        let mut plan = self.moved_plan(&mut canvas, only_moved, stretch);
        let mut layer = ui::Layer::default();
        // The overlay scrollbar shows while the offset moves and fades
        // once it has rested a moment.
        let key = self.region_key();
        let slot = self.slot();
        let offset = self.scroll_offset().unwrap_or(0.0);
        let page_offset = self.screen_offset().unwrap_or(0.0);
        self.sheet_offset = self.page_offset().unwrap_or(0.0);
        let (seen_key, seen_offset, moved_at) = self.scroll.bar_seen();
        let moved_at = if seen_key != (key, slot) {
            0
        } else if seen_offset != offset || stretch != 0 {
            self.now_ms.max(1)
        } else {
            moved_at
        };
        // Held, or with the pointer over it, it stays in full.
        let hot = self.pointer.bar_held().is_some()
            || self
                .pointer
                .hover()
                .is_some_and(|(x, y)| self.bar_at(x, y).is_some());
        let moved_at = if hot { self.now_ms.max(1) } else { moved_at };
        self.scroll.set_bar_seen((key, slot), offset, moved_at);
        let bar = motion::bar_alpha(moved_at, self.now_ms);
        let mut follow_to = None;
        let (mut scrolled, mut caret, mut reach) = (Vec::new(), None, Vec::new());
        let mut tips = Vec::new();
        let mut spoiled: bool;
        // A field newly focused brings the keyboard up, worked out before
        // drawing, which reads the keyboard as it now is; on a small panel
        // the page scrolls the field into view above it (`osk_follow`).
        self.osk_track();
        for _ in 0..3 {
            hits.clear();
            let focus = {
                let mut ui = ui::Ui::new(
                    &mut canvas,
                    self.f,
                    &mut hits,
                    self.pointer.pressed().map(|p| p.0),
                );
                ui.set_text_size(self.text_size());
                ui.edit = self.edit_now();
                ui.theme = self.theme();
                ui.hovered = self.pointer.hovered();
                ui.focused = focused;
                ui.tips.shown = self.pointer.tip_shown();
                ui.dimmed = self.demo_dimmed();
                ui.dimmed_answer = ui.dimmed.clone();
                ui.keep_live = self.keep_live();
                ui.keep_content = true;
                ui.moved = plan.take();
                ui.stretch = stretch;
                ui.bar = (bar > 0).then_some((bar, offset));
                ui.stretch_in_sheet = self.sheet.is_some_and(Sheet::scrolls_with_bar)
                    || (self.compact && self.sheet.is_some());
                let frost_key = ((self.screen, self.sheet), self.theme());
                ui.frost = self.fade.take_frost(&frost_key);
                ui.about_open = self.panel.about_on(self.screen, self.sheet.is_none());
                ui.offset = self.screen_offset().unwrap_or(page_offset);
                ui.active = slot;
                (ui.compact, ui.touch) = (self.compact, self.touch_side());
                ui.note_up = self.note_up();
                ui.caret_on = self.caret_on();
                let view = drawing::View::new(self);
                screens::draw(&view, &mut ui);
                ui.draw_tip();
                tips = std::mem::take(&mut ui.tips.anchors);
                self.apply_after(view.into_after());
                caret = ui.caret_drawn.then_some(ui.caret_on);
                self.osk_caret(ui.caret_foot);
                self.typing.set_fields(std::mem::take(&mut ui.fields));
                self.trail.set_link(self.screen, ui.back_link, ui.back_hit);
                scrolled = std::mem::take(&mut ui.scrolled);
                reach = ui.take_presses();
                self.pointer.set_cut(std::mem::take(&mut ui.cut));
                follow_to = follow_to.or(ui.follow_to);
                self.fade.keep_frost(frost_key, ui.frost.take());
                // The card's place, for a step's fade through the surface.
                self.fade.set_card(ui.flow_card.take().map(|(x, y, w, h)| {
                    faraday_draw::Rect::new(ui.px(x), ui.px(y), ui.px(w), ui.px(h))
                }));
                self.fade.set_pane(ui.pane.take().map(|(x, y, w, h)| {
                    faraday_draw::Rect::new(ui.px(x), ui.px(y), ui.px(w), ui.px(h))
                }));
                layer = ui.content;
                spoiled = ui.spoiled;
                (ui.focus_box, ui.focus_action)
            };
            if !self.osk_follow(focus.0, focus.1, &scrolled) && !spoiled {
                break;
            }
        }
        if let Some(want) = follow_to {
            // A follow moves the page, whatever region the pointer was on.
            self.scroll.went_to(ui::Slot::Page);
            let now = self.scroll_offset().unwrap_or(want);
            if self.reduce_motion() {
                self.with_region(|m, r| m.jump(r, want - now));
            } else {
                self.scroll.glide_to(want - now);
            }
            self.out.redraw();
            self.out.commands.push_back(Command::Draw);
        }
        let drew = self.region_key();
        self.scroll.drew(drew, scrolled);
        self.follow_home_find();
        self.typing.set_caret_drawn(caret);
        self.vaults.set_drawn();
        self.worker.set_drawn(true);
        self.pointer.set_hits(hits, reach);
        self.pointer.set_tip_anchors(tips);
        self.fade_end(&mut canvas);
        self.canvas = Some(canvas);
        self.moved_drew(&layer, stretch, caret.is_some());
        // The frame moved things under a still pointer: the next frame
        // shows what it is over now.
        if self.find_hovered() {
            self.out.redraw();
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Whether the text caret shows now: it blinks, on for a half period
    /// and off for one, starting on at the last key or press. Steady with
    /// reduce motion.
    pub(crate) fn caret_on(&self) -> bool {
        self.typing.caret_on(self.now_ms, self.reduce_motion())
    }

    /// The width and height of the layout, in design units.
    pub fn size(&self) -> (f32, f32) {
        (self.w, self.h)
    }

    pub(crate) fn display(&mut self, d: DisplayInfo) {
        let c = self.compute();
        self.last_display = Some(d);
        // The self-test runs once, at start, before any input: a display
        // comes before anything can be pressed.
        if self.selftest.is_none() {
            self.selftest = Some(faraday_selftest::run(&c));
        }
        self.fade.display_changed();
        // Laid out at the screen's own density, never scaled to fit a page
        // (`docs/LAYOUT.md` §7, F101); the person's Display scale
        // multiplies it.
        let f = self.pixels_per_unit_of(&d);
        // One-column pages but beside the expanded rail (LAYOUT §3); held
        // inside the Starter, under the wide layout's 1280 dp.
        self.compact = self.one_column();
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.w = f32::from(d.width) / f;
        self.h = f32::from(d.height) / f;
        let mut canvas = Canvas::new(&scaled);
        // The canvas notes its pixel work, a blur or a whole copy, with
        // this crate's counter (`docs/COMPUTE.md` §2.2).
        canvas.set_counter(work::core_count);
        self.canvas = Some(canvas);
        self.fresh_ask();
    }
}

// The frame's worked-out values: always filled, so nothing to count; a lock empties them.
impl crate::at_lock::AtLock for Drawing {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::NOTHING
    }
    fn lock(&mut self) {
        self.clear();
    }
}
