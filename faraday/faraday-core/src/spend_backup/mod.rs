//! The Spend tab (`docs/SPEND-FROM-BACKUP.md`): one column that
//! takes someone from what is in the envelope to a sent transaction.
//!
//! It is a wording layer over controls that exist elsewhere: Unlock, word
//! entry, Restore's sources, the spend's own steps and the QR sheet, all
//! acting on the same session, vaults and spend as every other screen, so
//! the same paper gives the same addresses and signatures here as in
//! Wallets. What the tab keeps of its own is the route what came in set,
//! the card open, the pages the person closed, the wallet chosen and the
//! files that came in on the intake card; none of it is secret, and it
//! is kept across a lock, so a lock for a stick or for idleness comes
//! back to the same page.
//!
//! Its third page, What do you have?, asks
//! for data, not a classification: whatever is scanned or typed comes in,
//! and the wallet is worked out from it, with only the next question
//! asked. A wallet worked out from seeds alone is a guess, and Check the
//! money walks through the other kinds until the money shows (§5).

pub(crate) mod screen;
pub(crate) mod text;

use crate::strings::EN;
use crate::wallet::KeyTexts;
use faraday_bip::keys::Fingerprint;
use faraday_bip::policy::WalletId;
use faraday_crypto::work::Compute;
use faraday_shell_api::Command;
use faraday_text::Text;

use crate::create::NewKind;
use crate::scan_key::{Found, key_of};
use crate::vault::VaultAction;
use crate::wallet::{FileKind, fp_text, step};
use crate::{Action, Faraday, QrView, ScanPurpose, ScanState, Screen, Sheet, flow};

/// How the wallet was opened, set from what came in: a vault unlocked
/// here, a description, or seeds alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// A vault unlocked here.
    Vault,
    /// Seeds alone.
    Words,
    /// A wallet description came in.
    Paper,
}

impl Route {
    /// Every route.
    pub const ALL: [Route; 3] = [Route::Vault, Route::Words, Route::Paper];

    fn code(self) -> &'static str {
        match self {
            Route::Vault => "vault",
            Route::Words => "words",
            Route::Paper => "paper",
        }
    }

    fn from_code(s: &str) -> Option<Route> {
        Route::ALL.into_iter().find(|r| r.code() == s)
    }
}

/// The tab's own pages, in order. The spend's steps go between `BRING`
/// and `AWAY`.
pub mod page {
    /// How to spend bitcoin: the map.
    pub const MAP: u8 = 0;
    /// Starting from the stick.
    pub const SAFE: u8 = 1;
    /// What do you have?: everything in the envelope comes in here.
    pub const HOLDING: u8 = 2;
    /// Open the wallet: one card with What do you have?
    /// It is no longer in the column. A page kept
    /// open across a lock from before maps to [`HOLDING`].
    pub const OPEN: u8 = 3;
    /// Check the money is really there.
    pub const CHECK: u8 = 4;
    /// Write the payment, online.
    pub const WRITE: u8 = 5;
    /// Bring the transaction here.
    pub const BRING: u8 = 6;
    /// Put everything away.
    pub(crate) const AWAY: u8 = 7;
    /// How many.
    pub const COUNT: u8 = 8;
}

/// The card open: one of the tab's pages, or whichever of the spend's
/// steps the spend has open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Open {
    /// A page of the tab.
    Page(u8),
    /// The spend's open step.
    Spend,
}

/// A card of the column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardId {
    /// A page of the tab.
    Page(u8),
    /// A step of the spend loaded (`wallet::step`).
    Step(u8),
    /// A step named before a transaction is here.
    Later(u8),
}

/// Everything the tab can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendBackupAction {
    /// Open or close a page.
    Card(u8),
    /// Done with a page: the next one opens.
    Next(u8),
    /// Not sure what you have?: the list of what each thing looks like.
    Unsure,
    /// Scan the QR codes you have: the intake scanner.
    Scan,
    /// That's everything: one seed opens as a single-key wallet.
    Everything,
    /// I can't find one: the seeds' wallet is shaped by hand.
    NoDescription,
    /// Add its passphrase, for a seed that is not one of the described
    /// wallet's keys.
    StrayPass([u8; 4]),
    /// No money shows: the next kind of wallet the seeds may be.
    NoMoney,
    /// The wallet of several keys made again as way `n` of the guess:
    /// run again by the hold of its confirmation sheet.
    GuessWay(usize),
    /// The passphrase question's answer: Yes, type it, or No.
    Passphrase(bool),
    /// After every kind: back to What do you have?, with the camera
    /// (`true`) or the seed's passphrase sheet.
    Again(bool),
    /// Show or hide a card's More about this, by `more_bit`.
    More(u8),
    /// Choose the vault to unlock, by its place in the vault list.
    Pick(usize),
    /// Unlock the chosen vault.
    Unlock,
    /// Type the words.
    TypeWords,
    /// Open the wallet in this Inbox file.
    UseWallet(usize),
    /// Open the flow on the wallet file at this Inbox place, from Files:
    /// the tab, then [`SpendBackupAction::UseWallet`].
    FromFile(usize),
    /// [`SpendBackupAction::UseWallet`] for the file `FromFile` started
    /// on: the flow starts again once the wallet is taken.
    TakeFile(usize),
    /// Rebuild the wallet from the split sheets in the Inbox.
    Rebuild,
    /// Spend from this wallet of the session.
    Choose(usize),
    /// Sign the PSBT in this Inbox file.
    UsePsbt(usize),
    /// Five more addresses on each chain.
    MoreAddresses,
    /// The finished transaction as a QR code.
    QrTx,
    /// Forget the route and the pages closed, and start again. A spend
    /// with steps done asks as Cancel does.
    StartOver,
    /// The last page's Done: the spend ends where it was opened from, as
    /// Cancel's end does, with nothing to ask.
    Done,
    /// From the Loaded view: spend from this wallet of the session, at
    /// the first page it has not done.
    SpendFrom(usize),
    /// From the Loaded view: the whole walk-through, from the map.
    Walkthrough,
    /// From the Loaded view: open the last seed loaded as a single-key
    /// wallet.
    SeedWallet,
}

/// One thing brought in on What do you have?, in the order it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Came {
    /// A seed, by its key's fingerprint.
    Seed([u8; 4]),
    /// A file in Files, by name.
    File(Text),
}

/// A wallet worked out from seeds alone, and the kinds tried on Check
/// the money.
#[derive(Debug, Clone, Default)]
pub struct Guess {
    /// Several keys: the multisig kinds, else the single-key ways.
    pub multi: bool,
    /// Several keys: the place in the multisig kinds of the one the seeds
    /// were shaped as, tried first.
    pub first: usize,
    /// The way tried now, by its place among the wallet's ways.
    pub at: usize,
    /// A passphrase was typed for the seed.
    pub pass: bool,
    /// Is there a word or sentence kept apart from the list? is asked.
    pub asking: bool,
    /// The ways tried that showed no money, as listed.
    pub tried: Vec<Text>,
    /// Every way was tried.
    pub none: bool,
}

/// The multisig kinds a guessed wallet of several keys walks through,
/// the most common first.
pub(crate) const MULTI_ORDER: [NewKind; 4] = [
    NewKind::Multi,
    NewKind::TapMulti,
    NewKind::MultiNested,
    NewKind::MultiLegacy,
];

/// The single-key ways: native SegWit without and with a passphrase,
/// then the other kinds.
const SINGLE_WAYS: usize = 5;

/// The question What do you have? asks next, from what came in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Nothing to ask.
    None,
    /// One seed: anything else in the envelope?
    Anything,
    /// Seeds and no description: scan it, or say it cannot be found.
    Description,
    /// Seeds and no description to be had: the wallet's shape by hand.
    Shape,
}

/// The tab's state.
#[derive(Debug, Default)]
pub struct SpendBackupState {
    /// How the wallet was opened, set from what came in.
    pub route: Option<Route>,
    /// The card open.
    pub open: Option<Open>,
    /// Pages the person closed as done: the map, the stick, the check
    /// and the payment, which no state of the app records.
    pub closed: [bool; page::COUNT as usize],
    /// The list of what each thing in the envelope looks like is open.
    pub help: bool,
    /// Cards whose More about this is open, by `more_bit`.
    pub more: u32,
    /// Addresses shown on each chain beyond the first three.
    pub addresses: u32,
    /// The wallet chosen to spend from, by its id (a hash of its whole
    /// descriptor): it survives a lock, places in the session do not,
    /// and names repeat.
    pub wallet: Option<WalletId>,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The last refusal.
    pub error: Option<Text>,
    /// The walk-through is shown although something is loaded already.
    pub walkthrough: bool,
    /// The sheet on screen was opened by Start over, not by Cancel: its
    /// confirm starts the tab again instead of leaving it.
    pub start_over_asked: bool,
    /// The seeds typed on the words route, and the wallet of several
    /// keys they make (`seeds.rs`).
    pub seeds: crate::seeds::SeedsState,
    /// Entered on a wallet already open, by Spend from this wallet:
    /// the column starts at Load the
    /// wallet in Sparrow, and Write the payment brings the PSBT itself.
    pub from_card: bool,
    /// Open from Home or from Wallets: Back goes there.
    pub from: Option<Screen>,
    /// What came in on What do you have?, newest last.
    pub came: Vec<Came>,
    /// Each key made with a passphrase here, with the plain seed it
    /// replaced on What do you have?.
    pub plain_of: Vec<([u8; 4], [u8; 4])>,
    /// That's everything was pressed on one seed.
    pub everything: bool,
    /// I can't find one was pressed: no description to be had.
    pub no_description: bool,
    /// The wallet was worked out from seeds alone.
    pub guess: Option<Guess>,
}

/// The Spend tab's area: its state, private outside this folder.
#[derive(Debug, Default)]
pub(crate) struct SpendBackupArea {
    state: SpendBackupState,
}

impl SpendBackupArea {
    /// The tab's state, to read.
    pub(crate) fn state(&self) -> &SpendBackupState {
        &self.state
    }

    /// Whether the tab opens on what is loaded rather than on its
    /// walk-through: something is loaded (`loaded`), and no route has been
    /// taken.
    pub(crate) fn overview(&self, loaded: bool) -> bool {
        !self.state.walkthrough && self.state.route.is_none() && loaded
    }

    /// The tab's pages before the spend's steps. From a wallet's card
    /// the wallet is open already: Load the wallet in Sparrow, then Write
    /// the payment, which brings the PSBT too. Starting from the stick
    /// heads What do you have? and is no page of its own (F103).
    pub(crate) fn pages(&self) -> Vec<u8> {
        if self.state.from_card {
            vec![page::CHECK, page::WRITE]
        } else {
            (0..=page::BRING)
                .filter(|&p| p != page::OPEN && p != page::SAFE)
                .collect()
        }
    }

    /// The page that brings the transaction here.
    pub(crate) fn bring_page(&self) -> u8 {
        if self.state.from_card {
            page::WRITE
        } else {
            page::BRING
        }
    }

    /// Sets the column's scroll, as the drawing pass notes it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        self.state.scroll = to;
    }

    /// How far the column is scrolled, units.
    pub(crate) fn scroll_y(&self) -> f32 {
        self.state.scroll.y
    }

    /// The column scrolled to `y` units.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        self.state.scroll.y = y;
    }

    /// Notes the screen the tab is opened from, for Back: Home or
    /// Wallets, else none.
    pub(crate) fn opened_from(&mut self, from: Screen) {
        self.state.from = matches!(from, Screen::Home | Screen::WalletsStart).then_some(from);
    }

    /// The screen the tab was opened from, when Home or Wallets.
    pub(crate) fn from(&self) -> Option<Screen> {
        self.state.from
    }

    /// The words route's seeds could not be read: the error they show.
    pub(crate) fn set_seeds_error(&mut self, error: Text) {
        self.state.seeds.error = Some(error);
    }

    /// The seeds typed on the words route, to change them.
    pub(crate) fn seeds_mut(&mut self) -> &mut crate::seeds::SeedsState {
        &mut self.state.seeds
    }

    /// A seed that came in unprotected is now the key `plain` was read as:
    /// what came in is renamed by its fingerprint.
    pub(crate) fn seed_became(&mut self, plain: [u8; 4], fp: [u8; 4]) {
        for c in self.state.came.iter_mut() {
            if *c == Came::Seed(plain) {
                *c = Came::Seed(fp);
            }
        }
    }
}

/// The single-key wallets typed words may open, the most common first.
pub const SINGLE: [NewKind; 4] = [
    NewKind::NativeSegwit,
    NewKind::Taproot,
    NewKind::NestedSegwit,
    NewKind::Legacy,
];

/// The source a wallet opened from typed words is loaded under.
pub const WORDS_SOURCE: &str = EN.spend_backup_typed_words;

/// The bit of `SpendBackupState::more` a card's More about this uses.
pub(crate) fn more_bit(id: CardId) -> u8 {
    match id {
        CardId::Page(p) => p,
        CardId::Step(n) | CardId::Later(n) => page::COUNT + n,
    }
}

impl Faraday {
    /// The sheet on screen was opened by Start over.
    pub(crate) fn start_over_asked(&self) -> bool {
        self.spend_backup.state.start_over_asked
    }

    /// Cancel, not Start over, opens the sheet from here on.
    pub(crate) fn spend_backup_not_start_over(&mut self) {
        self.spend_backup.state.start_over_asked = false;
    }

    /// The Spend tab's state.
    pub fn spend_backup(&self) -> &SpendBackupState {
        &self.spend_backup.state
    }

    /// Whether the tab opens on what is loaded rather than on its
    /// walk-through: something is loaded, and no route has been taken.
    pub fn spend_backup_overview(&self) -> bool {
        self.spend_backup
            .overview(!self.session.keys.is_empty() || !self.session.wallets.is_empty())
    }

    /// Costs what `spends_from` costs. The PSBT in Files that spends from
    /// wallet `w`: of several, the one with the fewest signatures, so a
    /// cosigner's signed copy never stands in for the transaction. Reads
    /// what each file said when it came in ([`crate::Item::psbt`]) and the
    /// scripts each wallet keeps: lookups once kept.
    pub fn psbt_for(&self, c: &Compute, w: usize) -> Option<usize> {
        self.inbox
            .iter()
            .enumerate()
            .filter_map(|(k, it)| {
                let seen = it.psbt()?;
                (self.session.spends_from(c, seen) == Some(w)).then_some((seen.sigs, k))
            })
            .min()
            .map(|(_, k)| k)
    }

    /// A lookup unless the files or the wallets changed since it was last
    /// worked out: then [`Faraday::psbt_for`] of every wallet, kept on the
    /// session for [`View::psbt_for`] to read, so no frame works it out.
    pub(crate) fn sync_psbt_wallets_if_changed(&mut self) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for w in &self.session.wallets {
            w.id().hash(&mut h);
        }
        for (k, it) in self.inbox.iter().enumerate() {
            if let Some(seen) = it.psbt() {
                (k, seen.txid, seen.sigs).hash(&mut h);
            }
        }
        let sign = h.finish();
        if self.session.psbt_wallets_inputs() == Some(sign) {
            return;
        }
        let c = self.compute();
        let found = (0..self.session.wallets.len())
            .map(|w| self.psbt_for(&c, w))
            .collect();
        self.session.set_psbt_wallets(found, sign);
    }

    /// The spend under way, resumed where it was: on the Spend tab when it
    /// went through it, else on Wallets' Sign.
    pub(crate) fn carry_on(&self) -> Action {
        let on_tab = self.wallets.spend().and_then(|s| s.wallet).filter(|&w| {
            self.spend_backup.state.route.is_some() && self.spend_backup_wallet() == Some(w)
        });
        match on_tab {
            Some(w) => Action::SpendBackup(SpendBackupAction::SpendFrom(w)),
            None => Action::Nav(Screen::Spend),
        }
    }

    /// Costs what `wallet_held_on` and the Inbox's PSBTs cost (through
    /// `spend_button_on`). The wallet card's first button:
    /// finish the backup first while a key
    /// made here waits for it (§14.3), else carry on this wallet's spend,
    /// sign its PSBT in Files, or spend from it.
    pub fn spend_button(&self, c: &Compute, w: usize) -> (Text, Action) {
        spend_button_on(&crate::glance::Now { app: self, c }, w)
    }
}

/// [`Faraday::spend_button`] read through `r` ([`crate::glance::Reads`]).
pub(crate) fn spend_button_on(r: &impl crate::glance::Reads, w: usize) -> (Text, Action) {
    let app = r.app();
    // A key made here signs once its backup is done
    // (`docs/WALLETS.md` §13).
    if !crate::held::wallet_held_on(r, w).is_empty() {
        return (
            Text::of(EN.spend_backup_finish_the_backup),
            Action::BackupFirst(w),
        );
    }
    if let Some(s) = app.wallets.spend().filter(|s| s.wallet == Some(w)) {
        return (
            Text::fill(
                EN.spend_backup_carry_on_spend,
                &[
                    &Text::num(s.signers.len() as u64),
                    &Text::num(app.spend_needed() as u64),
                ],
            ),
            app.carry_on(),
        );
    }
    let label = match r.psbt_for(w) {
        Some(_) => Text::fill(EN.spend_backup_sign_the_psbt, &[app.medium.noun()]),
        None => Text::of(EN.spend_backup_spend_from_wallet),
    };
    (label, Action::SpendBackup(SpendBackupAction::SpendFrom(w)))
}

impl Faraday {
    /// The wallet chosen to spend from, by its place in the session.
    pub fn spend_backup_wallet(&self) -> Option<usize> {
        let id = self.spend_backup.state.wallet?;
        self.session.wallets.iter().position(|w| w.id() == id)
    }

    /// A wallet is open and chosen.
    pub fn spend_backup_ready(&self) -> bool {
        self.spend_backup_wallet().is_some()
    }

    /// Whether a page is done.
    pub fn spend_backup_page_done(&self, p: u8) -> bool {
        if self.spend_backup.state.from_card {
            match p {
                // Loaded in Sparrow: said so here, or the wallet checked
                // this power-on.
                page::CHECK => {
                    return self.spend_backup_ready()
                        && (self.spend_backup.state.closed[p as usize]
                            || self
                                .spend_backup_wallet()
                                .is_some_and(|i| self.is_checked(self.session.wallets[i].id())));
                }
                // Write the payment ends with the PSBT here.
                page::WRITE => return self.wallets.spend().is_some(),
                _ => {}
            }
        }
        match p {
            page::HOLDING | page::OPEN => self.spend_backup_ready(),
            page::CHECK | page::WRITE => {
                self.spend_backup.state.closed[p as usize] && self.spend_backup_ready()
            }
            page::BRING => self.wallets.spend().is_some(),
            page::AWAY => false,
            _ => self
                .spend_backup
                .state
                .closed
                .get(p as usize)
                .copied()
                .unwrap_or(false),
        }
    }

    /// The cards, in order: the pages up to bringing the transaction,
    /// the spend's own steps (named ahead of time until there is one),
    /// then putting everything away.
    pub fn spend_backup_cards(&self) -> Vec<CardId> {
        let mut v: Vec<CardId> = self
            .spend_backup_pages()
            .into_iter()
            .map(CardId::Page)
            .collect();
        match self.wallets.spend() {
            Some(s) => v.extend(
                s.steps
                    .iter()
                    .copied()
                    .filter(|&n| n != step::WALLET && n != step::CHECK)
                    .map(CardId::Step),
            ),
            None => v.extend([step::TRANSACTION, step::SIGN, step::FINISH].map(CardId::Later)),
        }
        v.push(CardId::Page(page::AWAY));
        v
    }

    /// The tab's pages before the spend's steps. From a wallet's card
    /// the wallet is open already: Load the wallet in Sparrow, then Write
    /// the payment, which brings the PSBT too.
    pub(crate) fn spend_backup_pages(&self) -> Vec<u8> {
        self.spend_backup.pages()
    }

    /// The page that brings the transaction here.
    pub(crate) fn spend_backup_bring_page(&self) -> u8 {
        self.spend_backup.bring_page()
    }

    /// The card open now.
    pub fn spend_backup_open_card(&self) -> Option<CardId> {
        match self.spend_backup.state.open? {
            Open::Page(p) => Some(CardId::Page(p)),
            Open::Spend => self.wallets.spend()?.open.map(CardId::Step),
        }
    }

    /// The first thing not done after page `from`: a page, the spend's
    /// next step, or the last page. A page skipped before it stays where
    /// it is.
    fn spend_backup_first_open(&mut self, from: Option<u8>) -> Open {
        let pages = self.spend_backup_pages();
        if let Some(p) = pages
            .into_iter()
            .find(|&p| from.is_none_or(|f| p > f) && !self.spend_backup_page_done(p))
        {
            return Open::Page(p);
        }
        let next = self
            .wallets
            .spend()
            .map(|s| s.steps.iter().copied().find(|&n| !s.done[n as usize]));
        match next {
            // The spend's Check is the tab's Check the money.
            Some(Some(step::CHECK)) => Open::Page(page::CHECK),
            Some(Some(n)) => {
                self.wallets.open_step(n);
                Open::Spend
            }
            Some(None) | None => Open::Page(page::AWAY),
        }
    }

    fn spend_backup_advance(&mut self, from: u8) {
        self.spend_backup.state.open = Some(self.spend_backup_first_open(Some(from)));
        self.spend_backup.state.scroll.follow = true;
    }

    /// Keeps the open card one that can be reached: no page past What do
    /// you have? without a wallet, and no spend step without a spend.
    pub(crate) fn spend_backup_settle(&mut self) {
        self.spend_backup_auto_wallet();
        self.spend_backup_settle_cards();
    }

    /// [`Faraday::spend_backup_settle`] without opening the only wallet
    /// file by itself, for Remove every key: a removal brings no wallet,
    /// so one removed by hand stays removed.
    pub(crate) fn spend_backup_settle_cards(&mut self) {
        let ready = self.spend_backup_ready();
        // The wallet the card opened on is gone: the whole column again.
        if self.spend_backup.state.from_card && !ready {
            self.spend_backup.state.from_card = false;
        }
        // A guess is of the wallet it made: gone with it.
        if self.spend_backup.state.guess.is_some() && !self.spend_backup_guessed_wallet() {
            self.spend_backup.state.guess = None;
        }
        let open = match self.spend_backup.state.open {
            Some(Open::Page(page::OPEN | page::SAFE)) => Some(Open::Page(page::HOLDING)),
            Some(Open::Page(p)) if p > page::HOLDING && p != page::AWAY && !ready => {
                Some(Open::Page(page::HOLDING))
            }
            Some(Open::Spend) if self.wallets.spend().is_none() => Some(Open::Page(if ready {
                self.spend_backup_bring_page()
            } else {
                page::HOLDING
            })),
            o => o,
        };
        // A step is always open: with none, the first not done.
        let open = open.or_else(|| Some(self.spend_backup_first_open(None)));
        if open != self.spend_backup.state.open {
            self.spend_backup.state.open = open;
            self.spend_backup.state.scroll.follow = true;
        }
    }

    /// Runs one of the tab's actions.
    pub(crate) fn spend_backup_act(&mut self, a: SpendBackupAction) {
        let c = self.compute();
        use SpendBackupAction as F;
        if !matches!(a, F::More(_) | F::Unsure | F::MoreAddresses | F::Scan) {
            self.spend_backup.state.error = None;
        }
        match a {
            F::Card(p) => {
                // A step is always open: the press on the open one does
                // nothing.
                if self.spend_backup.state.open != Some(Open::Page(p)) {
                    self.spend_backup.state.open = Some(Open::Page(p));
                    self.spend_backup.state.scroll.follow = true;
                }
            }
            F::Next(p) => {
                if let Some(c) = self.spend_backup.state.closed.get_mut(p as usize) {
                    *c = true;
                }
                // "Yes, my money shows": this wallet is checked this
                // power-on (`docs/WALLETS.md` §4). "It is loaded" says
                // less.
                if p == page::CHECK
                    && !self.spend_backup.state.from_card
                    && let Some(w) = self.spend_backup_wallet()
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    self.mark_checked(wallet.id());
                }
                // The spend's own Check is this page, done by the person.
                // Only when the spend is the checked wallet's.
                if p == page::CHECK
                    && self.spend_backup_page_done(page::CHECK)
                    && self.spend_backup_wallet().is_some()
                    && self.wallets.spend().is_some_and(|s| {
                        s.wallet.is_none() || s.wallet == self.spend_backup_wallet()
                    })
                {
                    self.wallets.mark_step_done(step::CHECK);
                    self.wallets.open_next_undone();
                }
                self.spend_backup_advance(p);
            }
            F::Unsure => self.spend_backup.state.help = !self.spend_backup.state.help,
            F::Scan => self.spend_backup_scan(None),
            F::Everything => {
                self.spend_backup.state.everything = true;
                self.spend_backup_words_kind(NewKind::NativeSegwit);
                self.spend_backup_guess_start(false);
                self.refresh_spend();
                self.spend_backup_opened();
            }
            F::NoDescription => {
                self.spend_backup.state.no_description = true;
                let s = &mut self.spend_backup.state.seeds;
                if !s.shaping {
                    s.start_shape();
                }
            }
            F::StrayPass(fp) => {
                if let Some(w) = self.spend_backup_described() {
                    let reason = self.not_a_wallet_key(w, Fingerprint(fp));
                    self.seed_read_stray(Fingerprint(fp), w, reason);
                }
            }
            F::NoMoney => self.spend_backup_no_money(),
            F::GuessWay(at) => self.spend_backup_guess_step(at),
            F::Passphrase(yes) => {
                if yes {
                    if let Some(fp) = self
                        .spend_backup_words_key()
                        .map(|k| self.session.keys[k].fingerprint)
                    {
                        self.seed_read_spend_backup(fp, crate::seeds::read::ForSpendBackup::Guess);
                    }
                } else {
                    self.spend_backup_guess_step(2);
                }
            }
            F::Again(scan) => {
                self.spend_backup_drop_guess();
                if scan {
                    self.spend_backup_scan(None);
                } else if let Some(fp) = self.spend_backup.state.seeds.keys.first().copied() {
                    self.seed_read_spend_backup(
                        Fingerprint(fp),
                        crate::seeds::read::ForSpendBackup::Replace,
                    );
                }
            }
            F::More(k) => self.spend_backup.state.more ^= 1 << u32::from(k),
            F::Pick(i) => {
                self.vault_act(VaultAction::Pick(i));
                self.vaults.measure();
            }
            F::Unlock => {
                self.vaults.set_back_to(None);
                self.vaults.measure();
                self.vault_act(VaultAction::Unlock);
            }
            F::TypeWords => {
                if self.may_load_keys() {
                    // From the intake scanner's Type the words instead.
                    if self.camera.take().is_some() {
                        self.out.commands.push_back(Command::CameraOff);
                    }
                    if self.sheet == Some(Sheet::Scan) {
                        self.sheet = None;
                    }
                    self.entry
                        .begin(None, Some(Screen::SpendBackup), self.compact);
                    self.screen = Screen::Entry;
                }
            }
            F::UseWallet(i) => self.spend_backup_take(i, false),
            F::TakeFile(i) => self.spend_backup_take(i, true),
            F::FromFile(i) => self.spend_backup_from_file(i),
            F::Rebuild => {
                let names: Vec<Text> = self
                    .inbox
                    .iter()
                    .filter(|i| i.kind == FileKind::Share)
                    .map(|i| Text::of(&i.name))
                    .collect();
                for n in names {
                    self.spend_backup_came(Came::File(n));
                }
                self.spend_backup_rebuild(true);
            }
            F::Choose(i) => {
                if i < self.session.wallets.len() {
                    self.spend_backup_set_wallet(i);
                    if self.spend_backup.state.open == Some(Open::Page(page::HOLDING)) {
                        self.spend_backup_advance(page::HOLDING);
                    }
                }
            }
            F::UsePsbt(i) => self.spend_backup_use_psbt(i),
            F::MoreAddresses => {
                self.spend_backup.state.addresses = (self.spend_backup.state.addresses + 5).min(47)
            }
            F::QrTx => {
                if let Some(hex) = self.wallets.spend().and_then(|s| s.spend.finished_hex()) {
                    self.open_qr(QrView::text(&c, EN.spend_backup_finished_transaction, &hex));
                }
            }
            F::SpendFrom(i) => {
                if i >= self.session.wallets.len() {
                    return;
                }
                // Another wallet's spend gives way, as a new Sign does;
                // its PSBT stays in Files.
                if self.wallets.spend().is_some_and(|s| s.wallet != Some(i)) {
                    self.wallets.clear_spend();
                }
                self.spend_backup_set_wallet(i);
                let w = &self.session.wallets[i];
                self.spend_backup.state.from_card = true;
                self.spend_backup.state.help = false;
                self.screen = Screen::SpendBackup;
                // A wallet made here with its keys still here came from its
                // seeds, not from paper: it has no seeds to add.
                let keys_here = self.session.held_toward_quorum(w) >= crate::wallet::needed(w);
                self.spend_backup.state.route = Some(match w.origin {
                    crate::wallet::Origin::TypedWords | crate::wallet::Origin::TypedSeeds => {
                        Route::Words
                    }
                    crate::wallet::Origin::Statement if keys_here => Route::Words,
                    crate::wallet::Origin::Vault => Route::Vault,
                    crate::wallet::Origin::Statement | crate::wallet::Origin::Named => Route::Paper,
                });
                // Someone who loaded a wallet already knows the map and
                // the stick.
                self.spend_backup.state.closed[page::MAP as usize] = true;
                self.spend_backup.state.closed[page::SAFE as usize] = true;
                self.spend_backup.state.walkthrough = true;
                let psbt = self.psbt_for(&c, i);
                match (self.wallets.spend().map(|s| s.wallet), psbt) {
                    (Some(Some(w)), _) if w == i => {
                        self.spend_backup.state.open = Some(Open::Spend)
                    }
                    (_, Some(k)) if self.wallets.spend().is_none() => self.spend_backup_use_psbt(k),
                    _ => self.spend_backup_advance(page::HOLDING),
                }
                self.spend_backup.state.scroll.follow = true;
            }
            F::Walkthrough => {
                self.spend_backup.state.walkthrough = true;
                self.spend_backup.state.from_card = false;
                self.spend_backup.state.open = Some(Open::Page(page::MAP));
                self.spend_backup.state.scroll = Default::default();
            }
            F::SeedWallet => {
                self.spend_backup.state.from_card = false;
                self.spend_backup.state.route = Some(Route::Words);
                self.spend_backup.state.walkthrough = true;
                self.spend_backup.state.closed[page::MAP as usize] = true;
                self.spend_backup.state.closed[page::SAFE as usize] = true;
                self.spend_backup.state.everything = true;
                self.spend_backup_words_kind(NewKind::NativeSegwit);
                self.spend_backup_guess_start(false);
                self.refresh_spend();
                self.spend_backup_advance(page::HOLDING);
            }
            F::Done => self.flow_end(crate::in_progress::Going::Spend),
            F::StartOver => {
                if self
                    .cancel_loses(crate::in_progress::Going::Spend)
                    .is_some()
                {
                    self.sheet = Some(Sheet::CancelFlow(crate::in_progress::Going::Spend));
                    self.spend_backup.state.start_over_asked = true;
                    return;
                }
                self.spend_backup.state = SpendBackupState {
                    open: Some(Open::Page(page::HOLDING)),
                    closed: {
                        let mut c = [false; page::COUNT as usize];
                        c[page::MAP as usize] = self.spend_backup.state.closed[page::MAP as usize];
                        c[page::SAFE as usize] =
                            self.spend_backup.state.closed[page::SAFE as usize];
                        c
                    },
                    walkthrough: true,
                    ..SpendBackupState::default()
                };
                self.spend_backup.state.scroll.follow = true;
            }
        }
        self.spend_backup_settle();
    }

    /// With no wallet open yet, the description in Files opens by itself
    /// when there is just one: one wallet file, or split sheets that add
    /// up. After a lock, of the files that came in on What do you have?;
    /// on the vault route, when the open vaults hold seeds but no wallet,
    /// of every file in Files. Several wallet files wait for the person
    /// to choose.
    fn spend_backup_auto_wallet(&mut self) {
        let c = self.compute();
        if !self.session.wallets.is_empty() {
            return;
        }
        let all = match self.spend_backup.state.route {
            Some(Route::Paper) => false,
            Some(Route::Vault) => {
                if self.vaults.open.is_empty()
                    || !(0..self.vaults.open.len())
                        .all(|v| self.vault_rows_now(&c, v).iter().all(|r| r.seed))
                {
                    return;
                }
                true
            }
            _ => return,
        };
        // Files that describe one wallet in several forms (descriptor,
        // wallet .json, BIP 129 record, Core's import) count once.
        let mut wallets: Vec<(usize, Text)> = Vec::new();
        for (k, it) in self.inbox.iter().enumerate() {
            if crate::create::wallet_flow_kind(it) != FileKind::Wallet
                || !(all
                    || self
                        .spend_backup
                        .state
                        .came
                        .contains(&Came::File(Text::of(&it.name))))
            {
                continue;
            }
            if let Some(p) = it.wallet() {
                let d = crate::wallet::same_wallet(p);
                if !wallets.iter().any(|(_, x)| *x == d) {
                    wallets.push((k, d));
                }
            }
        }
        match wallets.as_slice() {
            [(one, _)] => self.spend_backup_use_wallet(*one),
            [] => self.spend_backup_rebuild(false),
            _ => {}
        }
    }

    /// The wallet to spend from, when the session says which: the only
    /// one, or the one a PSBT waiting in Files spends from.
    fn spend_backup_choose_only(&mut self) {
        let c = self.compute();
        if self.spend_backup_wallet().is_some() {
            return;
        }
        let only = (self.session.wallets.len() == 1).then_some(0);
        let by_psbt = || {
            self.lead_psbt()
                .and_then(|i| self.inbox[i].psbt())
                .and_then(|p| self.session.spends_from(&c, p))
        };
        if let Some(i) = only.or_else(by_psbt) {
            self.spend_backup_set_wallet(i);
        }
    }

    /// Drops the wallet Spend from a backup's own typed words made. Only
    /// that one: a wallet guessed from Seeds' typed seeds
    /// (`Origin::TypedSeeds`) is left alone, as the two screens keep their
    /// own guesses on their own terms.
    fn spend_backup_drop_typed_words(&mut self) {
        // One at a time, each as the list is then: what keeps a place
        // follows ([`Faraday::wallet_left`]).
        while let Some(i) = self
            .session
            .wallets
            .iter()
            .position(|w| w.origin == crate::wallet::Origin::TypedWords)
        {
            self.remove_wallet(i);
        }
    }

    /// Chooses the wallet at `i` of the session. Another wallet's pages
    /// said nothing of this one: choosing a different wallet opens the
    /// money check and the write page again.
    pub(crate) fn spend_backup_set_wallet(&mut self, i: usize) {
        let chosen = self.session.wallets[i].id();
        if self.spend_backup.state.wallet != Some(chosen) {
            self.spend_backup.state.closed[page::CHECK as usize] = false;
            self.spend_backup.state.closed[page::WRITE as usize] = false;
            self.spend_backup.state.addresses = 0;
        }
        self.spend_backup.state.wallet = Some(chosen);
        self.wallets.set_wallet(i);
    }

    /// After a wallet arrives: chosen when it is the only one, and the
    /// next page opens once there is one, unless a seed that came in is
    /// none of its keys, which What do you have? says.
    pub(crate) fn spend_backup_opened(&mut self) {
        self.spend_backup_choose_only();
        if let Some(i) = self.spend_backup_wallet() {
            self.wallets.set_wallet(i);
        }
        if self.spend_backup_ready()
            && matches!(
                self.spend_backup.state.open,
                Some(Open::Page(page::HOLDING | page::OPEN))
            )
            && self.spend_backup_strays().is_empty()
        {
            self.spend_backup_advance(page::HOLDING);
        }
    }

    /// A vault unlocked from this tab: everything in it is chosen to
    /// load, since spending needs both the wallet and its seeds, and the
    /// person loads it or less from the list. The wallet is opened from
    /// the vault from now on.
    pub(crate) fn spend_backup_unlocked(&mut self, v: usize) {
        self.vaults.load_all(v);
        self.spend_backup.state.route = Some(Route::Vault);
        self.spend_backup.state.walkthrough = true;
    }

    /// A vault's seeds loaded on What do you have? with no wallet: they
    /// came in as seeds, and the next question asks for the rest.
    pub(crate) fn spend_backup_vault_loaded(&mut self) {
        if !self.session.wallets.is_empty() {
            return;
        }
        let fps: Vec<[u8; 4]> = self.session.keys.all().map(|k| k.fingerprint.0).collect();
        for fp in fps {
            self.spend_backup.state.seeds.take(fp);
            self.spend_backup_came(Came::Seed(fp));
        }
        if self.spend_backup.state.seeds.keys.len() >= 2 {
            self.spend_backup_several_seeds();
        }
    }

    /// A key typed or scanned from this tab: it fills the described
    /// wallet's slot; with no description it waits for the next
    /// question.
    pub(crate) fn spend_backup_key_added(&mut self) {
        self.refresh_spend();
        if self.spend_backup_described().is_some() {
            self.spend_backup_opened();
        }
        self.spend_backup_settle();
    }

    /// Something came in: listed once, newest last.
    pub(crate) fn spend_backup_came(&mut self, c: Came) {
        self.spend_backup.state.walkthrough = true;
        if !self.spend_backup.state.came.contains(&c) {
            self.spend_backup.state.came.push(c);
        }
    }

    /// A key removed from the session: the seed is no longer one this
    /// flow holds or came with.
    /// The plain seed a passphrase key replaced, still held, takes its
    /// place again.
    pub(crate) fn spend_backup_key_gone(&mut self, fp: Fingerprint) {
        self.spend_backup_forget_key(fp);
        self.spend_backup_settle();
    }

    /// [`Faraday::spend_backup_key_gone`] without its settle, for several
    /// keys gone in one press. A guess whose wallet has this key in a
    /// slot goes too: its other kinds would be made from whichever key
    /// is loaded, or from no seed at all.
    pub(crate) fn spend_backup_forget_key(&mut self, fp: Fingerprint) {
        if self.spend_backup.state.guess.is_some()
            && let Some(w) = self.spend_backup_wallet()
            && self
                .session
                .slots(&self.session.wallets[w])
                .iter()
                .any(|s| s.fingerprint == Some(fp))
        {
            self.spend_backup.state.guess = None;
        }
        let plain = self
            .spend_backup
            .state
            .plain_of
            .iter()
            .find(|(k, _)| *k == fp.0)
            .map(|(_, p)| *p)
            .filter(|p| self.session.keys.key_label(Fingerprint(*p)).is_some());
        self.spend_backup.state.plain_of.retain(|(k, _)| *k != fp.0);
        if let Some(p) = plain {
            for k in self.spend_backup.state.seeds.keys.iter_mut() {
                if *k == fp.0 {
                    *k = p;
                }
            }
            for c in self.spend_backup.state.came.iter_mut() {
                if *c == Came::Seed(fp.0) {
                    *c = Came::Seed(p);
                }
            }
        }
        self.spend_backup.state.seeds.drop_key(fp.0);
        self.spend_backup
            .state
            .came
            .retain(|c| *c != Came::Seed(fp.0));
    }

    /// The wallet chosen, when it came from a description (a file, the
    /// split sheets, a vault) rather than from seeds alone.
    pub(crate) fn spend_backup_described(&self) -> Option<usize> {
        let w = self.spend_backup_wallet()?;
        let origin = self.session.wallets[w].origin;
        (!matches!(
            origin,
            crate::wallet::Origin::TypedWords | crate::wallet::Origin::TypedSeeds
        ))
        .then_some(w)
    }

    /// The seeds that came in that are none of the described wallet's
    /// keys.
    pub(crate) fn spend_backup_strays(&self) -> Vec<Fingerprint> {
        let Some(w) = self.spend_backup_described() else {
            return Vec::new();
        };
        self.spend_backup
            .state
            .came
            .iter()
            .filter_map(|c| match c {
                Came::Seed(fp) => Some(Fingerprint(*fp)),
                Came::File(_) => None,
            })
            .filter(|fp| self.session.keys.all().any(|k| k.fingerprint == *fp))
            .filter(|fp| matches!(self.wallet_key_found(w, *fp), Found::None))
            .collect()
    }

    /// The question What do you have? asks next, from what came in.
    pub fn spend_backup_question(&self) -> Question {
        if self.spend_backup_ready() || self.spend_backup_described().is_some() {
            return Question::None;
        }
        // Several wallets loaded, from a vault say: the choice is asked.
        if !self.session.wallets.is_empty() && self.spend_backup.state.route == Some(Route::Vault) {
            return Question::None;
        }
        match self.seeds_here(&self.spend_backup.state.seeds).len() {
            0 => Question::None,
            1 if self.spend_backup.state.everything => Question::None,
            1 => Question::Anything,
            _ if self.spend_backup.state.no_description => Question::Shape,
            _ => Question::Description,
        }
    }

    /// The hint the intake scanner's sheet gives for the question asked.
    pub(crate) fn spend_backup_hint(&self) -> Option<&'static str> {
        match self.spend_backup_question() {
            Question::Anything => Some(crate::spend_backup::text::ANYTHING_HINT),
            Question::Description => Some(crate::spend_backup::text::DESCRIPTION_HINT),
            _ => None,
        }
    }

    /// The intake scanner: anything, one code after another, until Done.
    pub(crate) fn spend_backup_scan(&mut self, note: Option<Text>) {
        let mut scan = ScanState::default();
        if let Some((decoder, assembler)) = self.camera.take_parts() {
            scan.decoder = decoder;
            scan.assembler = assembler;
        }
        scan.purpose = ScanPurpose::Intake;
        scan.note = note.map(|n| faraday_text::Text::of(&n));
        self.camera.open(scan);
        self.sheet = Some(Sheet::Scan);
        self.out.commands.push_back(Command::CameraOn);
        if !self.spend_backup_ready() {
            self.spend_backup.state.open = Some(Open::Page(page::HOLDING));
        }
    }

    /// The intake scanner read a seed's words. With a description in,
    /// a seed whose key with no passphrase is one of its missing keys
    /// loads at once; one that is none of its keys asks for its
    /// passphrase. Else the passphrase sheet asks how it loads, and the
    /// camera comes back on after.
    pub(crate) fn spend_backup_intake_seed(&mut self, words: &str) {
        let c = self.compute();
        let Some(fp) = key_of(&c, words, "") else {
            self.camera.say(EN.spend_backup_not_a_seedqr);
            return;
        };
        let Some(w) = self.spend_backup_described() else {
            self.seed_read_open(words);
            self.seed_read.for_spend_backup_intake();
            return;
        };
        let note = match self.wallet_key_found(w, fp) {
            Found::Missing(_) => {
                let label = self.wallet_key_label(&c, w, fp);
                match self
                    .session
                    .keys
                    .add_words_with(&c, words, "", &label, None)
                {
                    Ok(_) => {
                        self.spend_backup_came(Came::Seed(fp.0));
                        self.refresh_spend();
                        Text::fill(EN.spend_backup_seed_read, &[&fp_text(fp)])
                    }
                    Err(e) => e.text(),
                }
            }
            Found::Here(n) => {
                Text::fill(EN.spend_backup_key_already, &[&Text::num((n + 1) as u64)])
            }
            Found::None => {
                let reason = self.not_a_wallet_key(w, fp);
                self.seed_read_for_wallet(words, w, reason);
                self.seed_read.for_spend_backup_intake();
                return;
            }
        };
        self.camera.say(&note);
    }

    /// A SLIP-39 share or a codex32 string read by the intake scanner or
    /// Home's Scan: Add a key opens on its form with it in, and returns to
    /// the Spend tab once the key is put together. Returns whether it was
    /// one.
    pub(crate) fn intake_key_part(&mut self, bytes: &[u8]) -> bool {
        let given = Text::of(Text::from_utf8_lossy(bytes).trim());
        let lower: Text = given.chars().flat_map(char::to_lowercase).collect();
        // A codex32 string as given: BIP 93 refuses mixed case.
        let (form, text) = if faraday_bip::slip39::Share::parse(&lower).is_ok() {
            (crate::forms::Form::Slip39, lower)
        } else if faraday_bip::codex32::Codex32::parse(&given).is_ok() {
            (crate::forms::Form::Codex32, given)
        } else {
            return false;
        };
        // My seed makes my wallet takes the part into its own form.
        if crate::seed_check::take_part(self, form, &text) {
            self.camera.close();
            self.sheet = None;
            self.out.commands.push_back(Command::CameraOff);
            return true;
        }
        if !self.may_load_keys() {
            self.camera.say(crate::stick::keys_load(self.medium));
            return true;
        }
        self.camera.close();
        self.sheet = None;
        self.out.commands.push_back(Command::CameraOff);
        self.act(Action::Entry(None));
        if let Some(i) = crate::forms::Form::ALL.iter().position(|f| *f == form) {
            self.act(Action::EntryForm(i as u8));
        }
        self.entry.set_typed(&text);
        self.form_add_part();
        true
    }

    /// A file the intake scanner put in Files: listed, and taken as what
    /// it is. The camera stays on.
    pub(crate) fn spend_backup_intake_file(&mut self, index: usize) {
        let Some((kind, name)) = self
            .inbox
            .get(index)
            .map(|i| (crate::create::wallet_flow_kind(i), i.name.clone()))
        else {
            return;
        };
        self.spend_backup_came(Came::File(Text::of(&name)));
        self.spend_backup.state.error = None;
        let note = match kind {
            FileKind::Wallet => {
                if self.spend_backup_ready() && self.spend_backup_described().is_some() {
                    Text::of(EN.spend_backup_wallet_description_read)
                } else {
                    self.spend_backup_use_wallet(index);
                    Text::of(EN.spend_backup_wallet_description)
                }
            }
            FileKind::Share => {
                let sheets = self.spend_backup_sheets();
                let k = sheets.len();
                if !self.spend_backup_ready() {
                    self.spend_backup_rebuild(false);
                }
                match crate::restore::merge(&sheets) {
                    Ok(m) if m.whole.is_some() && self.spend_backup_ready() => {
                        Text::fill(EN.spend_backup_part_read_rebuilt, &[&Text::num(k as u64)])
                    }
                    Ok(m) => Text::fill(
                        EN.spend_backup_part_read_progress,
                        &[
                            &Text::num(k as u64),
                            &Text::num(m.have.len() as u64),
                            &Text::num(m.n as u64),
                        ],
                    ),
                    Err(e) => Text::of(&e),
                }
            }
            FileKind::Key | FileKind::KeyWithSecret => {
                self.seeds_take_file(index);
                Text::of(EN.spend_backup_a_public_key)
            }
            FileKind::Psbt => {
                self.spend_backup_psbt_kinds(index);
                Text::of(EN.spend_backup_a_transaction)
            }
            _ => Text::fill(EN.spend_backup_is_in_files, &[&name]),
        };
        let note = self.spend_backup.state.error.clone().unwrap_or(note);
        self.camera.say(&note);
    }

    /// The split sheets that came in on What do you have?, as text.
    fn spend_backup_sheets(&self) -> Vec<Text> {
        self.inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .filter(|i| {
                self.spend_backup
                    .state
                    .came
                    .contains(&Came::File(Text::of(&i.name)))
            })
            .map(|i| Text::from_utf8_lossy(&i.bytes))
            .collect()
    }

    /// A transaction that came in with one seed and nothing else: the
    /// single-key wallet of the kind it spends from opens, its kind
    /// shown by the transaction, not guessed. It does not start the
    /// spend; Bring the transaction here finds it in Files.
    fn spend_backup_psbt_kinds(&mut self, index: usize) {
        let c = self.compute();
        if self.spend_backup_ready() || self.spend_backup_question() != Question::Anything {
            return;
        }
        let Some(p) = self.inbox.get(index).and_then(|i| i.psbt()).cloned() else {
            return;
        };
        let hit = self.spend_backup_words_search(&SINGLE, |s| {
            s.spend_backup_wallet().is_some()
                && s.session.spends_from(&c, &p) == s.spend_backup_wallet()
        });
        if hit {
            self.spend_backup.state.everything = true;
            self.spend_backup.state.guess = None;
            self.refresh_spend();
            self.spend_backup_opened();
            return;
        }
        self.spend_backup_drop_typed_words();
        self.spend_backup.state.wallet = None;
        let last = self.session.wallets.len().saturating_sub(1);
        self.wallets.set_wallet(self.wallets.wallet().min(last));
    }

    /// A seed whose sheet loaded it from the intake scanner: the camera
    /// comes back on for the next code.
    pub(crate) fn spend_backup_intake_again(&mut self, fp: Option<Fingerprint>) {
        let note = fp.map(|fp| Text::fill(EN.spend_backup_seed_read, &[&fp_text(fp)]));
        self.spend_backup_scan(note);
    }

    /// Whether the wallet chosen is the one guessed from seeds alone.
    fn spend_backup_guessed_wallet(&self) -> bool {
        self.spend_backup_wallet().is_some_and(|w| {
            matches!(
                self.session.wallets[w].origin,
                crate::wallet::Origin::TypedWords | crate::wallet::Origin::TypedSeeds
            )
        })
    }

    /// The wallet just made from seeds alone is a guess: Check the money
    /// tries the other kinds.
    pub(crate) fn spend_backup_guess_start(&mut self, multi: bool) {
        let first = if multi {
            MULTI_ORDER
                .iter()
                .position(|k| *k == self.spend_backup.state.seeds.kind)
                .unwrap_or(0)
        } else {
            0
        };
        self.spend_backup.state.guess = Some(Guess {
            multi,
            first,
            ..Guess::default()
        });
        self.spend_backup.state.closed[page::CHECK as usize] = false;
    }

    /// The guessed wallet's ways, and the one tried now.
    pub fn spend_backup_guess(&self) -> Option<&Guess> {
        self.spend_backup
            .state
            .guess
            .as_ref()
            .filter(|_| self.spend_backup_guessed_wallet())
    }

    /// The way `at` of a guess, as listed: its kind, and for one key
    /// whether a passphrase goes with it.
    pub(crate) fn guess_way(g: &Guess, at: usize) -> (NewKind, bool) {
        if g.multi {
            // The kind the seeds were shaped as, then the others in order.
            let at = at.min(MULTI_ORDER.len() - 1);
            let place = match at {
                0 => g.first,
                _ if at > g.first => at,
                _ => at - 1,
            };
            (MULTI_ORDER[place], false)
        } else {
            let kind = match at {
                0 | 1 => NewKind::NativeSegwit,
                2 => NewKind::Taproot,
                3 => NewKind::NestedSegwit,
                _ => NewKind::Legacy,
            };
            (kind, at == 1 || (at > 1 && g.pass))
        }
    }

    /// How many ways a guess has.
    pub fn guess_total(g: &Guess) -> usize {
        if g.multi {
            MULTI_ORDER.len()
        } else {
            SINGLE_WAYS
        }
    }

    /// No money shows: the way tried is listed as such, and the next one
    /// replaces the wallet; for one key, first the passphrase question.
    fn spend_backup_no_money(&mut self) {
        let Some(g) = self.spend_backup.state.guess.as_mut() else {
            return;
        };
        let (kind, pass) = Self::guess_way(g, g.at);
        let mut label = Text::of(kind_label(kind));
        if pass {
            label.push(' ');
            label.push_str(EN.spend_backup_with_a_passphrase);
        }
        // A way answered again, after the next one was cancelled while
        // held for its sheet, is listed once.
        let line = Text::fill(EN.spend_backup_no_money, &[&label]);
        if !g.tried.contains(&line) {
            g.tried.push(line);
        }
        if !g.multi && g.at == 0 {
            g.asking = true;
            return;
        }
        let next = g.at + 1;
        self.spend_backup_guess_step(next);
    }

    /// The guess goes on to way `at`: the wallet made again of that
    /// kind, or None of these show money after the last.
    pub(crate) fn spend_backup_guess_step(&mut self, at: usize) {
        let Some(g) = self.spend_backup.state.guess.as_mut() else {
            return;
        };
        g.asking = false;
        if at >= Self::guess_total(g) {
            g.none = true;
            return;
        }
        let before = g.at;
        g.at = at;
        let (kind, _) = Self::guess_way(g, at);
        let multi = g.multi;
        self.spend_backup.state.addresses = 0;
        self.spend_backup.state.closed[page::CHECK as usize] = false;
        if multi {
            // Held for its sheet, the wallet is still the way before:
            // the way shown stays that one until the hold.
            if self.spend_backup_seeds_kind(kind)
                && let Some(g) = self.spend_backup.state.guess.as_mut()
            {
                g.at = before;
            }
        } else {
            self.spend_backup_words_kind(kind);
        }
        self.refresh_spend();
    }

    /// The wallet of several keys made again as `kind`, in place of the
    /// one made before, under the same name. True when a sheet holds the
    /// new wallet back and the one before stays.
    fn spend_backup_seeds_kind(&mut self, kind: NewKind) -> bool {
        let c = self.compute();
        let Some(w) = self.spend_backup_wallet() else {
            return false;
        };
        let before = self.spend_backup.state.seeds.kind;
        self.spend_backup.state.seeds.kind = kind;
        let s = self.spend_backup.state.seeds.clone();
        let d = match self.seeds_descriptor(&c, &s) {
            Ok(d) => d,
            Err(e) => {
                self.spend_backup.state.error = Some(Text::of(&e));
                return false;
            }
        };
        // Out of the list without telling the places kept: where the
        // wallet stays out ([`Faraday::wallet_left`], below) they move, and
        // where it is put back nothing moved.
        let kept = self.session.wallets.remove(w);
        let name = kept.name.clone();
        match self.session.add_wallet(
            &c,
            &name,
            &d,
            crate::seeds::SOURCE,
            crate::wallet::Origin::TypedSeeds,
        ) {
            // The same wallet again: nothing ends, it stands at its place.
            Ok(i) if self.session.wallets[i].id() == kept.id() => {
                self.session.wallets.remove(i);
                self.session.wallets.insert(w, kept);
                self.spend_backup_set_wallet(w);
            }
            Ok(i) => {
                self.wallet_left(w, kept.checksum(), kept.id());
                // `i` was the end of the list with the old wallet out: the
                // same place now.
                self.spend_backup_set_wallet(i);
            }
            // A cosigner's key no loaded key gives holds this guess back
            // for its sheet: the guess before stays until the hold, which
            // makes this way again.
            Err(crate::wallet::Refusal::Unconfirmed) => {
                self.session.wallets.insert(w, kept);
                self.spend_backup.state.seeds.kind = before;
                let way = self.spend_backup.state.guess.as_ref().map(|g| g.at);
                if let Some(at) = way {
                    self.session.wallet_replay(
                        crate::Action::SpendBackup(SpendBackupAction::GuessWay(at)),
                        self.inbox_print(),
                    );
                }
                return true;
            }
            Err(e) => {
                self.wallet_left(w, kept.checksum(), kept.id());
                self.spend_backup.state.error = Some(e.text());
            }
        }
        false
    }

    /// The seed's passphrase was typed for the guess: the seed is its
    /// key with the passphrase from now on, and native SegWit with it is
    /// tried. With none typed, the guess goes on without one.
    pub(crate) fn spend_backup_guess_pass(
        &mut self,
        plain: Fingerprint,
        source: &crate::seeds::read::Source,
        pass: &str,
    ) {
        if pass.is_empty() {
            self.spend_backup_guess_step(2);
            return;
        }
        if self.spend_backup_seed_with(plain, source, pass).is_none() {
            return;
        }
        if let Some(g) = self.spend_backup.state.guess.as_mut() {
            g.pass = true;
        }
        self.spend_backup_guess_step(1);
    }

    /// Seed `plain` replaced by its key with `pass`, on What do you
    /// have? and in its list. Returns the new key.
    pub(crate) fn spend_backup_seed_with(
        &mut self,
        plain: Fingerprint,
        source: &crate::seeds::read::Source,
        pass: &str,
    ) -> Option<Fingerprint> {
        let c = self.compute();
        let label = self
            .session
            .keys
            .key_label(plain)
            .map(|l| {
                if l.is_empty() || l.eq_ignore_ascii_case(&fp_text(plain)) {
                    Text::of(EN.spend_backup_passphrase_inline)
                } else {
                    Text::fill(EN.spend_backup_passphrase_n, &[l])
                }
            })
            .unwrap_or_default();
        let fp = match source.add(&c, &mut self.session.keys, pass, &label) {
            Ok(fp) => fp,
            Err(crate::wallet::Refusal::Duplicate(_)) => {
                source.fingerprint(&c, &self.session.keys, pass)?
            }
            Err(e) => {
                self.toast(&e.text());
                return None;
            }
        };
        self.spend_backup.state.plain_of.push((fp.0, plain.0));
        for k in self.spend_backup.state.seeds.keys.iter_mut() {
            if *k == plain.0 {
                *k = fp.0;
            }
        }
        if self.spend_backup.state.seeds.keys.is_empty() {
            self.spend_backup.state.seeds.keys.push(fp.0);
        }
        for c in self.spend_backup.state.came.iter_mut() {
            if *c == Came::Seed(plain.0) {
                *c = Came::Seed(fp.0);
            }
        }
        Some(fp)
    }

    /// Back to What do you have? after every kind was tried: the guessed
    /// wallet goes, and the questions are asked again.
    fn spend_backup_drop_guess(&mut self) {
        if let Some(w) = self
            .spend_backup_wallet()
            .filter(|_| self.spend_backup_guessed_wallet())
        {
            self.remove_wallet(w);
        }
        self.spend_backup_drop_typed_words();
        self.spend_backup.state.wallet = None;
        let last = self.session.wallets.len().saturating_sub(1);
        self.wallets.set_wallet(self.wallets.wallet().min(last));
        self.spend_backup.state.guess = None;
        self.spend_backup.state.everything = false;
        self.spend_backup.state.no_description = false;
        self.spend_backup.state.seeds.shaping = false;
        self.spend_backup.state.closed[page::CHECK as usize] = false;
        self.spend_backup.state.addresses = 0;
        self.refresh_spend();
        self.spend_backup.state.open = Some(Open::Page(page::HOLDING));
        self.spend_backup.state.scroll.follow = true;
    }

    /// The key the words route's single-key wallet is opened from: the
    /// first seed typed on the route, else the last loaded.
    pub(crate) fn spend_backup_words_key(&self) -> Option<usize> {
        self.spend_backup
            .state
            .seeds
            .keys
            .first()
            .and_then(|fp| self.session.keys.position_of(*fp))
            .or(self.session.keys.len().checked_sub(1))
    }

    /// The wallet the last key typed opens as `kind`, in place of the one
    /// it opened before.
    fn spend_backup_words_kind(&mut self, kind: NewKind) {
        self.spend_backup_words_search(&[kind], |_| true);
    }

    /// The wallet the last key typed opens as each of `kinds` in turn,
    /// until `hit` says the one standing is the one wanted (then true; when
    /// none is, the wallet before stands). What the wallet before had open
    /// (its Back up, its restore) ends only when it is not the wallet that
    /// stands at the end: the same wallet put back in one press moves and
    /// ends nothing.
    fn spend_backup_words_search(
        &mut self,
        kinds: &[NewKind],
        mut hit: impl FnMut(&mut Self) -> bool,
    ) -> bool {
        let keep = self.spend_backup_words_take();
        let mut found = false;
        let mut chosen = None;
        for kind in kinds {
            let stood = self.spend_backup_words_trial(*kind);
            if hit(self) {
                found = true;
                // Only a wallet the trial put in the list is the one
                // chosen: a refused trial leaves the wallet before.
                chosen = self.spend_backup.state.wallet.filter(|_| stood);
                break;
            }
        }
        self.spend_backup_words_settle(keep, found.then_some(chosen).flatten(), found);
        found
    }

    /// The typed-words wallet standing before a press, out of the list
    /// with its place and nothing told ([`Self::spend_backup_words_settle`]
    /// does that): any other is a wallet really gone.
    fn spend_backup_words_take(&mut self) -> Option<(usize, crate::wallet::Wallet)> {
        let typed = |s: &Self| {
            s.session
                .wallets
                .iter()
                .position(|w| w.origin == crate::wallet::Origin::TypedWords)
        };
        let first = typed(self)?;
        // The others go first, really, while the first still holds its
        // place: their places are right then.
        let rest: Vec<usize> = (first + 1..self.session.wallets.len())
            .filter(|i| self.session.wallets[*i].origin == crate::wallet::Origin::TypedWords)
            .collect();
        for i in rest.into_iter().rev() {
            self.remove_wallet(i);
        }
        let keep = self.session.wallets.remove(first);
        Some((first, keep))
    }

    /// The wallet at the end of the press: `found` says the last one tried
    /// stands (`chosen`: its id, which is a wallet already in the list when
    /// the kind added nothing), otherwise the one taken before does, and
    /// where there was none the last kind tried stands.
    fn spend_backup_words_settle(
        &mut self,
        keep: Option<(usize, crate::wallet::Wallet)>,
        chosen: Option<WalletId>,
        found: bool,
    ) {
        let trial = self
            .session
            .wallets
            .iter()
            .rposition(|w| w.origin == crate::wallet::Origin::TypedWords)
            .map(|i| self.session.wallets.remove(i));
        self.session
            .wallets
            .retain(|w| w.origin != crate::wallet::Origin::TypedWords);
        let trial = trial.filter(|_| found || keep.is_none());
        let at = match (keep, trial) {
            (None, Some(t)) => {
                self.session.wallets.push(t);
                self.session.wallets.len() - 1
            }
            (None, None) => return,
            (Some((p, k)), Some(t)) if k.id() == t.id() => {
                self.session.wallets.insert(p, k);
                p
            }
            (Some((p, k)), Some(t)) => {
                // The wallet before really leaves: the list as it was,
                // then the one that took its place.
                self.session.wallets.insert(p, k);
                self.remove_wallet(p);
                self.session.wallets.push(t);
                self.session.wallets.len() - 1
            }
            (Some((p, k)), None) => {
                // A kind that hit a wallet already in the list (another
                // origin) adds nothing: that wallet is the one chosen and
                // the typed one before really leaves.
                let other = chosen.filter(|id| *id != k.id());
                self.session.wallets.insert(p, k);
                match other.filter(|id| self.session.wallets.iter().any(|w| w.id() == *id)) {
                    Some(id) => {
                        self.remove_wallet(p);
                        match self.session.wallets.iter().position(|w| w.id() == id) {
                            Some(i) => i,
                            None => return,
                        }
                    }
                    None => p,
                }
            }
        };
        self.spend_backup_set_wallet(at);
    }

    /// The wallet the last key typed opens as `kind`, put in the list in
    /// place of any typed-words wallet there.
    fn spend_backup_words_trial(&mut self, kind: NewKind) -> bool {
        let c = self.compute();
        let Some(key) = self.spend_backup_words_key() else {
            return false;
        };
        let text = match self.session.keys.account_text(&c, key, kind, 0) {
            Ok(t) => t,
            Err(e) => {
                self.spend_backup.state.error = Some(Text::of(&e));
                return false;
            }
        };
        let descriptor = kind.descriptor(1, &[faraday_text::Text::of(&text)]);
        self.session
            .wallets
            .retain(|w| w.origin != crate::wallet::Origin::TypedWords);
        let name = Text::fill(EN.spend_backup_words, &[kind_name(kind)]);
        match self.session.add_wallet(
            &c,
            &name,
            &descriptor,
            WORDS_SOURCE,
            crate::wallet::Origin::TypedWords,
        ) {
            Ok(i) => {
                self.spend_backup_set_wallet(i);
                true
            }
            Err(e) => {
                self.spend_backup.state.error = Some(e.text());
                false
            }
        }
    }

    /// The single-key kind the words route's wallet is, if it is one.
    pub fn spend_backup_words_kind_now(&self) -> Option<NewKind> {
        // Spend from a backup's own typed-words wallet only: Seeds' typed-seeds one
        // (`Origin::TypedSeeds`) is a different guess, not this one.
        let w = self
            .session
            .wallets
            .iter()
            .find(|w| w.origin == crate::wallet::Origin::TypedWords)?;
        SINGLE
            .into_iter()
            .find(|k| w.name == *Text::fill(EN.spend_backup_words, &[kind_name(*k)]))
    }

    /// The wallet file at Inbox place `i` is taken; `fresh` when the
    /// flow was started on it from Files.
    fn spend_backup_take(&mut self, i: usize, fresh: bool) {
        if let Some(name) = self.inbox.get(i).map(|it| Text::of(&it.name)) {
            self.spend_backup_came(Came::File(name));
        }
        self.spend_backup_use_wallet_in(i, fresh);
    }

    fn spend_backup_use_wallet(&mut self, index: usize) {
        self.spend_backup_use_wallet_in(index, false);
    }

    fn spend_backup_use_wallet_in(&mut self, index: usize, fresh: bool) {
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let text = Text::from_utf8_lossy(&item.bytes);
        let name = faraday_bip::multisig_config::parse_named(&c, &text)
            .ok()
            .and_then(|(_, n)| n)
            .or_else(|| crate::wallet::carried_name(&text))
            .unwrap_or_else(|| Text::of(&crate::file_stem(&item.name)));
        let source = item.name.clone();
        let asked = self.session.wallet_asked().is_some();
        match self
            .session
            .add_wallet(&c, &name, &text, &source, crate::wallet::Origin::Named)
        {
            Ok(i) => {
                if fresh {
                    self.spend_backup_fresh_for(i, &source);
                }
                self.spend_backup_set_wallet(i);
                self.spend_backup_described_in();
                self.refresh_spend();
                self.spend_backup_opened();
            }
            // Held for its sheet: the hold runs this again.
            Err(crate::wallet::Refusal::Unconfirmed) => {
                if !asked {
                    self.session.wallet_replay(
                        crate::Action::SpendBackup(if fresh {
                            SpendBackupAction::TakeFile(index)
                        } else {
                            SpendBackupAction::UseWallet(index)
                        }),
                        self.inbox_print(),
                    );
                }
            }
            Err(e) => self.spend_backup.state.error = Some(e.text()),
        }
    }

    /// Spend from backup on Inbox file `i` (its menu): the file's wallet
    /// is taken as What do you have? takes it, and the flow starts again
    /// for it when another wallet's was under way. Another wallet's spend
    /// under way gives way as `SpendFrom` has it, asked first where
    /// ending it loses something (Start over's sheet; the file is taken
    /// when pressed again).
    fn spend_backup_from_file(&mut self, i: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(i) else {
            return;
        };
        let file = item.wallet().map(crate::wallet::same_wallet);
        // A Coldcard export of two accounts may give the other one: a
        // spend under way counts as another wallet's.
        let text = Text::from_utf8_lossy(&item.bytes);
        let two_accounts =
            self.wallets.spend().is_some() && { self.session.account_may_differ(&c, &text) };
        let spend_other = self.wallets.spend().is_some_and(|s| {
            two_accounts
                || !file.as_ref().is_some_and(|f| {
                    s.wallet
                        .and_then(|w| self.session.wallets.get(w))
                        .is_some_and(|w| *crate::wallet::same_wallet(w.policy()) == **f)
                })
        });
        // The spend stays until the file is accepted: a refusal or a
        // cancelled sheet leaves it as it was (`spend_backup_fresh_for`
        // ends one that is not the taken wallet's).
        if spend_other
            && self
                .cancel_loses(crate::in_progress::Going::Spend)
                .is_some()
        {
            self.sheet = Some(Sheet::CancelFlow(crate::in_progress::Going::Spend));
            self.spend_backup.state.start_over_asked = true;
            return;
        }
        self.act(Action::Nav(Screen::SpendBackup));
        // A refusal is said on What do you have?, in front of the person.
        self.spend_backup.state.open = Some(Open::Page(page::HOLDING));
        self.spend_backup.state.walkthrough = true;
        self.spend_backup.state.scroll.follow = true;
        self.act(Action::SpendBackup(SpendBackupAction::TakeFile(i)));
    }

    /// The flow starts again for the wallet taken from Inbox file `i`
    /// when it is not the one chosen: its own Check, route, pages and
    /// way in. Another wallet's spend does not stay beside it.
    fn spend_backup_fresh_for(&mut self, taken: usize, file: &str) {
        let id = self.session.wallets[taken].id();
        if self.spend_backup.state.wallet.is_some() && self.spend_backup.state.wallet != Some(id) {
            let old = std::mem::take(&mut self.spend_backup.state);
            let closed = old.closed;
            self.spend_backup.state = SpendBackupState {
                closed: {
                    let mut c = [false; page::COUNT as usize];
                    c[page::MAP as usize] = closed[page::MAP as usize];
                    c[page::SAFE as usize] = closed[page::SAFE as usize];
                    c
                },
                from: old.from,
                // Only the file just taken came in for this wallet.
                came: vec![Came::File(Text::of(file))],
                open: Some(Open::Page(page::HOLDING)),
                walkthrough: true,
                ..SpendBackupState::default()
            };
            self.spend_backup.state.scroll.follow = true;
        }
        if self
            .wallets
            .spend()
            .is_some_and(|s| s.wallet != Some(taken))
        {
            self.wallets.clear_spend();
        }
    }

    /// A description came in: the wallet is opened from it, and a guess
    /// made from seeds before it goes.
    fn spend_backup_described_in(&mut self) {
        if self.spend_backup.state.route != Some(Route::Vault) {
            self.spend_backup.state.route = Some(Route::Paper);
        }
        self.spend_backup.state.walkthrough = true;
        self.spend_backup.state.guess = None;
        self.spend_backup_drop_typed_words();
        if let Some(i) = self.spend_backup_wallet() {
            self.wallets.set_wallet(i);
        }
    }

    /// The split sheets in the Inbox, put together. Says what is still
    /// missing only when asked (`loud`).
    fn spend_backup_rebuild(&mut self, loud: bool) {
        let c = self.compute();
        let texts: Vec<Text> = self
            .inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .map(|i| Text::from_utf8_lossy(&i.bytes))
            .collect();
        if texts.is_empty() {
            return;
        }
        match crate::restore::merge(&texts) {
            Ok(m) => match m.whole {
                Some(whole) => {
                    let in_text = faraday_bip::multisig_config::parse_named(&c, &whole)
                        .ok()
                        .and_then(|(_, n)| n);
                    let name = crate::restore::name_or_sheets(
                        in_text,
                        m.name,
                        EN.spend_backup_restored_wallet,
                    );
                    let asked = self.session.wallet_asked().is_some();
                    match self.session.add_wallet(
                        &c,
                        &name,
                        &whole,
                        EN.spend_backup_its_shares,
                        crate::wallet::Origin::Named,
                    ) {
                        Ok(i) => {
                            self.spend_backup_set_wallet(i);
                            self.spend_backup_described_in();
                            self.refresh_spend();
                            self.spend_backup_opened();
                        }
                        Err(crate::wallet::Refusal::Unconfirmed) => {
                            if !asked {
                                self.session.wallet_replay(
                                    crate::Action::SpendBackup(SpendBackupAction::Rebuild),
                                    self.inbox_print(),
                                );
                            }
                        }
                        Err(e) => self.spend_backup.state.error = Some(e.text()),
                    }
                }
                None if loud => {
                    self.spend_backup.state.error = Some(Text::fill(
                        EN.spend_backup_of_keys_in_hand,
                        &[&Text::num(m.have.len() as u64), &Text::num(m.n as u64)],
                    ));
                }
                None => {}
            },
            Err(e) => {
                if loud {
                    self.spend_backup.state.error = Some(Text::of(&e));
                }
            }
        }
    }

    /// Starts signing an Inbox PSBT without leaving the tab. On the words
    /// route a transaction that names another kind of single-key wallet
    /// opens that kind: the transaction's own key paths say which.
    fn spend_backup_use_psbt(&mut self, index: usize) {
        if self.inbox.get(index).is_none() {
            self.screen = Screen::SpendBackup;
            return;
        }
        crate::signing::read::start_spend(self, index, crate::signing::read::Then::SpendBackup);
    }

    /// The rest of [`Self::spend_backup_use_psbt`], once the read has answered:
    /// `opened` is whether the spend opened.
    pub(crate) fn spend_backup_used_psbt(&mut self, opened: bool) {
        self.screen = Screen::SpendBackup;
        let Some(s) = self.wallets.spend().filter(|_| opened) else {
            return;
        };
        if s.wallet.is_none()
            && self.spend_backup.state.route == Some(Route::Words)
            && self.spend_backup.state.seeds.keys.len() <= 1
        {
            let found = self.spend_backup_words_search(&SINGLE, |s| {
                s.refresh_spend();
                s.wallets.spend().is_some_and(|s| s.wallet.is_some())
            });
            if !found {
                self.refresh_spend();
            }
        }
        // The wallet the transaction spends from is the one chosen.
        let w = self.wallets.spend().and_then(|s| s.wallet);
        if let Some(w) = w
            && w < self.session.wallets.len()
        {
            self.spend_backup_set_wallet(w);
        }
        // The addresses are checked on the tab's own page, Check the
        // money: the spend's Check is done only once the person did it.
        let checked = self.spend_backup_page_done(page::CHECK);
        self.wallets.mark_step_done(step::WALLET);
        if checked {
            self.wallets.mark_step_done(step::CHECK);
        }
        self.wallets.open_next_undone();
        self.spend_backup_step_moved(false);
        if self.spend_backup.state.open.is_none() {
            self.spend_backup.state.open = Some(Open::Spend);
        }
        self.spend_backup.state.scroll.follow = true;
    }

    /// Costs a PSBT read and a txid, and what `collect` costs. A file that
    /// arrived by camera while the tab is open: a wallet or the last
    /// split sheet opens the wallet, a PSBT starts the spend. While a
    /// spend is under way a PSBT is checked by its unsigned transaction's
    /// txid: the spend's own is collected into it (its signatures
    /// merged), another is kept in Files and never replaces the spend.
    /// Returns whether the tab took it (on Home, only a PSBT for a spend
    /// under way).
    pub(crate) fn spend_backup_arrived(&mut self, index: usize) -> bool {
        let Some(kind) = self.inbox.get(index).map(crate::create::wallet_flow_kind) else {
            return false;
        };
        // Home's scan takes a PSBT only for a spend under way.
        if self.screen == Screen::Home {
            return kind == FileKind::Psbt && self.spend_backup_psbt_for_open_spend(index);
        }
        if self.screen != Screen::SpendBackup {
            return false;
        }
        match kind {
            FileKind::Wallet if !self.spend_backup_ready() => {
                self.spend_backup_use_wallet(index);
                true
            }
            FileKind::Share if !self.spend_backup_ready() => {
                self.spend_backup_rebuild(false);
                true
            }
            FileKind::Psbt
                if self.spend_backup_ready() || self.spend_backup.state.route.is_some() =>
            {
                if self.spend_backup_psbt_for_open_spend(index) {
                    return true;
                }
                self.spend_backup_use_psbt(index);
                true
            }
            _ => false,
        }
    }

    /// Costs a PSBT read and a txid, and what `collect` costs. A PSBT
    /// arriving while a spend is under way: for the spend's own
    /// transaction it is collected into it; for another it is refused
    /// with a toast while the spend is not finished and has a step card
    /// open or a signature. Returns whether it was handled here; false
    /// leaves a new spend to start. Home's scan comes here too.
    pub(crate) fn spend_backup_psbt_for_open_spend(&mut self, index: usize) -> bool {
        let Some(txid) = self.wallets.spend().map(|s| s.spend.txid) else {
            return false;
        };
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return false;
        };
        let name = item.name.clone();
        let Some(other) = crate::wallet::read_psbt(&c, &item.bytes) else {
            return false;
        };
        if other.unsigned_tx().compute_txid() == txid {
            // Scanned from Home, the spend is where it moves on.
            self.screen = Screen::SpendBackup;
            crate::signing::collect::start_collect_on_spend_backup(self, index);
            self.spend_backup.state.open = Some(Open::Spend);
            self.spend_backup_step_moved(false);
            return true;
        }
        let under_way = self.wallets.spend().is_some_and(|s| {
            s.spend.finished.is_none()
                && (s.open.is_some()
                    || !s.spend.signed_here.is_empty()
                    || !s.spend.collected.is_empty())
        });
        if under_way {
            self.toast(&Text::fill(EN.spend_backup_psbt_not_this_spend, &[&name]));
            return true;
        }
        false
    }

    /// After the spend's own step cards change on this tab: the card open
    /// follows the spend's, and the last page opens after the last step.
    /// The spend's Check is no card of the tab: its open is Check the
    /// money, the tab's own page.
    pub(crate) fn spend_backup_step_moved(&mut self, closed_as_done: bool) {
        let open = self.wallets.spend().and_then(|s| s.open);
        self.spend_backup.state.open = match (open, closed_as_done) {
            (Some(step::CHECK), _) => Some(Open::Page(page::CHECK)),
            (Some(_), _) => Some(Open::Spend),
            (None, true) => Some(Open::Page(page::AWAY)),
            (None, false) => Some(self.spend_backup_first_open(None)),
        };
        self.spend_backup.state.scroll.follow = true;
    }

    /// What the tab keeps across a lock: nothing secret.
    pub(crate) fn spend_backup_kept(&self) -> Option<Vec<u8>> {
        let active = self.screen == Screen::SpendBackup
            || (self.screen == Screen::Entry
                && self.entry.state().back == Some(Screen::SpendBackup));
        if self.spend_backup.state.route.is_none()
            && self.spend_backup.state.came.is_empty()
            && !active
        {
            return None;
        }
        let open = match self.spend_backup.state.open {
            Some(Open::Page(p)) => Text::of(&Text::num(u64::from(p))),
            Some(Open::Spend) => Text::of("spend"),
            None => Text::new(),
        };
        let closed: Vec<Text> = (0..page::COUNT)
            .filter(|&p| self.spend_backup.state.closed[p as usize])
            .map(|p| Text::of(&Text::num(u64::from(p))))
            .collect();
        let mut text = faraday_text::text!(
            "active={}\nroute={}\nopen={}\nclosed=",
            u8::from(active),
            self.spend_backup.state.route.map(Route::code).unwrap_or(""),
            &*open
        );
        for (k, p) in closed.iter().enumerate() {
            if k > 0 {
                text.push(',');
            }
            text.push_str(p);
        }
        text.push('\n');
        if let Some(w) = self.spend_backup.state.wallet {
            text.push_str(&faraday_text::text!("wallet={}\n", w.to_hex().as_str()));
        }
        if self.spend_backup.state.from_card {
            text.push_str("card=1\n");
        }
        // The files that came in; seeds come back by the vault or not
        // at all.
        for c in &self.spend_backup.state.came {
            if let Came::File(name) = c {
                text.push_str(&faraday_text::text!(
                    "came={}\n",
                    faraday_text::Text::replaced(name, "\n", " ").as_str()
                ));
            }
        }
        Some(text.as_bytes().to_vec())
    }

    /// Reads back what the previous process kept.
    pub(crate) fn spend_backup_restore(&mut self, bytes: &[u8]) {
        let mut f = SpendBackupState::default();
        let mut active = false;
        for line in Text::from_utf8_lossy(bytes).lines() {
            match line.split_once('=') {
                Some(("active", v)) => active = v == "1",
                Some(("route", v)) => f.route = Route::from_code(v),
                Some(("open", "spend")) => f.open = Some(Open::Spend),
                Some(("open", v)) => {
                    // Open the wallet is one card with What do you have?
                    // now.
                    f.open = v
                        .parse::<u8>()
                        .ok()
                        .filter(|&p| p < page::COUNT)
                        .map(|p| if p == page::OPEN { page::HOLDING } else { p })
                        .map(Open::Page);
                }
                Some(("closed", v)) => {
                    for p in v.split(',').filter_map(|p| p.parse::<u8>().ok()) {
                        if let Some(c) = f.closed.get_mut(p as usize) {
                            *c = true;
                        }
                    }
                }
                // A kept form from before, an 8-character checksum, is no id
                // and names no wallet.
                Some(("wallet", v)) => f.wallet = WalletId::from_hex(v),
                Some(("card", v)) => f.from_card = v == "1",
                Some(("came", v)) if !v.is_empty() => f.came.push(Came::File(Text::of(v))),
                _ => {}
            }
        }
        f.scroll.follow = true;
        self.spend_backup.state = f;
        if active {
            self.screen = Screen::SpendBackup;
        }
        self.spend_backup_settle();
    }
}

impl Faraday {
    /// A seed typed on What do you have? with no wallet open yet: the
    /// seeds the wallet is worked out from.
    pub(crate) fn spend_backup_seed_took(&mut self, fp: Fingerprint) {
        self.spend_backup_came(Came::Seed(fp.0));
        if self.spend_backup.state.route.is_none() {
            self.spend_backup.state.route = Some(Route::Words);
        }
        // A seed typed before this flow kept its list, as the wallet the
        // words route opened from it: it is the first.
        if self.spend_backup.state.seeds.keys.is_empty()
            && let Some(w) = self
                .session
                .wallets
                .iter()
                .find(|w| w.origin == crate::wallet::Origin::TypedWords)
            && let Some(first) = self.session.slots(w).first().and_then(|s| s.fingerprint)
            && first != fp
            && self.session.keys.position_of(first.0).is_some()
        {
            self.spend_backup.state.seeds.keys.push(first.0);
        }
        self.spend_backup.state.seeds.take(fp.0);
        if self.spend_backup.state.seeds.keys.len() >= 2 {
            self.spend_backup_several_seeds();
        }
    }

    /// Two seeds or more and no description: the single-key wallet the
    /// first opened goes, and once I can't find one is pressed, the
    /// wallet's shape comes up.
    pub(crate) fn spend_backup_several_seeds(&mut self) {
        let before = self.session.wallets.len();
        self.spend_backup_drop_typed_words();
        if self.session.wallets.len() != before {
            self.wallets.set_list_changed();
            if self
                .spend_backup
                .state
                .wallet
                .is_some_and(|id| !self.session.wallets.iter().any(|w| w.id() == id))
            {
                self.spend_backup.state.wallet = None;
            }
            let last = self.session.wallets.len().saturating_sub(1);
            self.wallets.set_wallet(self.wallets.wallet().min(last));
            // A spend panel open names wallets: it must not name one that is gone.
            self.refresh_spend();
        }
        self.spend_backup.state.everything = false;
        let s = &mut self.spend_backup.state.seeds;
        if s.shaping {
            s.fit();
        } else if self.spend_backup.state.no_description {
            s.start_shape();
        }
    }
}

/// A single-key kind's name, as the words route shows it.
pub fn kind_name(kind: NewKind) -> &'static str {
    match kind {
        NewKind::NativeSegwit => EN.spend_backup_native_segwit_kind,
        NewKind::Taproot => EN.spend_backup_taproot,
        NewKind::NestedSegwit => EN.spend_backup_nested_segwit_kind,
        NewKind::Legacy => EN.spend_backup_legacy,
        _ => "",
    }
}

/// A kind's name as Check the money lists the ways tried: the
/// single-key names, and the multisig kinds named as such.
pub fn kind_label(kind: NewKind) -> &'static str {
    match kind {
        NewKind::Multi => EN.spend_backup_native_segwit,
        NewKind::TapMulti => EN.spend_backup_taproot_multisig,
        NewKind::MultiNested => EN.spend_backup_nested_segwit,
        NewKind::MultiLegacy => EN.spend_backup_legacy_multisig,
        k => kind_name(k),
    }
}

/// The Spend tab's cards, plugged into Back and Forward (F104). A card
/// goes by one number: a page by its own, a step of the spend from
/// [`STEP_AT`], a step named before the spend from [`LATER_AT`].
pub(crate) struct SpendBackupSteps;

/// The one [`SpendBackupSteps`].
pub(crate) static SPEND_BACKUP_STEPS: SpendBackupSteps = SpendBackupSteps;

/// Where the spend's steps start among the cards' numbers.
const STEP_AT: u8 = 16;

/// Where the steps named before the spend start.
const LATER_AT: u8 = 32;

/// A card's number.
fn card_code(id: CardId) -> u8 {
    match id {
        CardId::Page(p) => p,
        CardId::Step(n) => STEP_AT + n,
        CardId::Later(n) => LATER_AT + n,
    }
}

/// The card a number stands for.
pub(crate) fn card_of(k: u8) -> CardId {
    match k {
        k if k >= LATER_AT => CardId::Later(k - LATER_AT),
        k if k >= STEP_AT => CardId::Step(k - STEP_AT),
        p => CardId::Page(p),
    }
}

impl crate::trail::StepFlow for SpendBackupSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        // What is loaded, shown in place of the walk-through, has no steps.
        if app.spend_backup_overview() {
            return None;
        }
        let cards = app.spend_backup_cards();
        let order = cards.iter().copied().map(card_code).collect();
        let mut s = crate::trail::Steps::new(order, app.spend_backup_open_card().map(card_code));
        s.answered = cards
            .iter()
            .copied()
            .filter(|&id| screen::done(app, id))
            .map(card_code)
            .collect();
        s.first = app
            .wallets
            .spend()
            .and_then(crate::wallet::SpendSteps::first)
            .map(|k| card_code(CardId::Step(k)));
        Some(s)
    }

    fn open(&self, app: &Faraday, k: u8) -> Action {
        toggle(app, card_of(k))
    }

    fn name(&self, app: &Faraday, k: u8) -> &'static str {
        text::name(app, card_of(k))
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }
}

impl crate::flow::Contained for SpendBackupSteps {
    fn rows(&self, app: &Faraday) -> Vec<crate::flow::Row> {
        let open = app.spend_backup_open_card();
        app.spend_backup_cards()
            .into_iter()
            .map(|id| crate::flow::Row {
                label: faraday_text::Text::of(text::name(app, id)),
                value: Some(screen::summary(app, id)).filter(|t| !t.is_empty()),
                mono: matches!(id, CardId::Step(step::TXID)),
                opens: card_code(id),
                current: open == Some(id),
                mark: None,
                address: false,
            })
            .collect()
    }

    fn hide_shown(&self, app: &mut crate::Faraday) {
        app.vaults.hide_typed();
        app.typing.pass_eye.close();
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        false
    }
}

/// The action a card's header press takes.
pub fn toggle(app: &Faraday, id: CardId) -> Action {
    match id {
        CardId::Page(p) => Action::SpendBackup(SpendBackupAction::Card(p)),
        CardId::Step(n) => Action::Step(n),
        CardId::Later(_) => {
            Action::SpendBackup(SpendBackupAction::Card(app.spend_backup_bring_page()))
        }
    }
}

// The seeds typed for Spend from a backup.
impl crate::at_lock::AtLock for SpendBackupArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::typed(self.state.seeds.cosigner_typed())
    }
    fn lock(&mut self) {
        self.state.seeds = crate::seeds::SeedsState::default();
    }
}
