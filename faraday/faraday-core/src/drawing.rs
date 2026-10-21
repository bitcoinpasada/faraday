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

use crate::{Faraday, flow, plan, work};

mod allowed;
pub(crate) use allowed::Allowed;
mod kept;
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
    /// ([`crate::Faraday::board_view_now`], `docs/BACKUP-FLOW.md` §19).
    pub board: OnceCell<Option<std::rc::Rc<crate::board::BoardView>>>,
    /// What Create's Keys card offers from the vaults
    /// ([`View::create_vaults`], `docs/history/NEW-WALLET.md` §16.1).
    pub create_vaults: OnceCell<std::rc::Rc<crate::create_from::CreateVaults>>,
    /// The flows in progress ([`crate::Faraday::flows_in_progress_now`],
    /// `docs/FLOWS.md`, Flows in progress).
    pub flows: OnceCell<std::rc::Rc<Vec<crate::in_progress::Row>>>,
    /// What a lock keeps and wipes, by `power_off`
    /// ([`crate::Faraday::kept_wiped_now`]).
    pub kept_wiped: RefCell<Vec<(bool, crate::vaults::KeptWiped)>>,
    /// The sticks the wallet being backed up is written to
    /// ([`crate::Faraday::wallet_stick_rows_now`]).
    pub stick_rows: OnceCell<Vec<StickRow>>,
    /// The Load sheet an unlock shows ([`crate::Faraday::vault_load_view_now`],
    /// F62).
    pub load_sheet: OnceCell<Option<std::rc::Rc<crate::vault_load::LoadView>>>,
    /// The seeds open vaults keep without a passphrase
    /// ([`crate::Faraday::vault_passphrase_seeds_now`], F62).
    pub passphrase_seeds: OnceCell<std::rc::Rc<Vec<crate::vault_load::PassSeed>>>,
    /// Every wallet known and where its backup is
    /// ([`crate::Faraday::backups_now`]).
    pub backups: OnceCell<Vec<crate::backups::Entry>>,
    /// The keys and wallets each open vault holds, by its place among
    /// the open vaults ([`crate::Faraday::vault_rows_now`]).
    pub vault_rows: RefCell<Vec<(usize, Vec<crate::vaults::VaultRow>)>>,
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
    /// Writes what a drawing pass noted. Cheap.
    pub(crate) fn apply_after(&mut self, a: After) {
        for (column, to) in a.scrolls {
            match column {
                Scrolled::BackupFlow => {
                    if let Some(i) = self.flow_on
                        && let Some(f) = self.backup_flows.get_mut(i)
                    {
                        f.live.scroll = to;
                    }
                }
                Scrolled::Silent => {
                    if let Some(s) = self.silent.as_mut() {
                        s.scroll = to;
                    }
                }
                Scrolled::Bip85 => {
                    if let Some(b) = self.bip85.as_mut() {
                        b.scroll = to;
                    }
                }
                Scrolled::KeyGen => {
                    if let Some(k) = self.keygen.as_mut() {
                        k.scroll = to;
                    }
                }
                Scrolled::Vanity => {
                    if let Some(v) = self.vanity.as_mut() {
                        v.scroll = to;
                    }
                }
                Scrolled::CreateVault => {
                    if let Some(c) = self.vaults.create.as_mut() {
                        c.scroll = to;
                    }
                }
                Scrolled::Spend => {
                    if let Some(s) = self.spend.as_mut() {
                        s.scroll = to.y;
                        s.follow = to.follow;
                    }
                }
                Scrolled::Backup => {
                    if let Some(b) = self.backup.as_mut() {
                        b.scroll = to;
                    }
                }
                Scrolled::Message => {
                    if let Some(m) = self.message.as_mut() {
                        m.scroll = to;
                    }
                }
                Scrolled::Create => {
                    if let Some(c) = self.create.as_mut() {
                        c.scroll = to;
                    }
                }
                Scrolled::Restore => {
                    if let Some(r) = self.restore.as_mut() {
                        r.scroll = to;
                    }
                }
                Scrolled::Family => self.family.scroll = to,
            }
        }
        if let Some((max, scroll)) = a.learn {
            self.learn.max = max;
            self.learn.scroll = scroll;
        }
        if let Some((max, scroll)) = a.wordlist
            && let Some(st) = self.wordlist.as_mut()
        {
            st.max = max;
            st.scroll = scroll;
            st.follow = false;
        }
        if let Some(lift) = a.lift {
            self.osk_set_lift(lift);
        }
        if a.redraw {
            self.dirty = true;
            self.commands.push_back(faraday_shell_api::Command::Draw);
        }
    }
}
