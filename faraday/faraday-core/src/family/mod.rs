//! The Spend tab (`docs/FAMILY.md`): one column that
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
//! Its third page, What do you have? (`docs/SCAN-TO-SIGN.md` §4), asks
//! for data, not a classification: whatever is scanned or typed comes in,
//! and the wallet is worked out from it, with only the next question
//! asked. A wallet worked out from seeds alone is a guess, and Check the
//! money walks through the other kinds until the money shows (§5).

pub(crate) mod screen;
pub(crate) mod text;

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::KeyTexts;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_shell_api::Command;

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
    /// Open the wallet: one card with What do you have? since
    /// `docs/SCAN-TO-SIGN.md` §4, no longer in the column. A page kept
    /// open across a lock from before maps to [`HOLDING`].
    pub const OPEN: u8 = 3;
    /// Check the money is really there.
    pub const CHECK: u8 = 4;
    /// Write the payment, online.
    pub const WRITE: u8 = 5;
    /// Bring the transaction here.
    pub const BRING: u8 = 6;
    /// Put everything away.
    pub const AWAY: u8 = 7;
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
pub enum FamilyAction {
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
    /// Forget the route and the pages closed, and start again.
    StartOver,
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
    File(String),
}

/// A wallet worked out from seeds alone, and the kinds tried on Check
/// the money (`docs/SCAN-TO-SIGN.md` §5).
#[derive(Debug, Clone, Default)]
pub struct Guess {
    /// Several keys: the multisig kinds, else the single-key ways.
    pub multi: bool,
    /// The way tried now, by its place among the wallet's ways.
    pub at: usize,
    /// A passphrase was typed for the seed.
    pub pass: bool,
    /// Is there a word or sentence kept apart from the list? is asked.
    pub asking: bool,
    /// The ways tried that showed no money, as listed.
    pub tried: Vec<String>,
    /// Every way was tried.
    pub none: bool,
}

/// The multisig kinds a guessed wallet of several keys walks through,
/// the most common first.
pub const MULTI_ORDER: [NewKind; 4] = [
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
pub struct FamilyState {
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
    /// The wallet chosen to spend from, by name: names survive a lock,
    /// places in the session do not.
    pub wallet: Option<String>,
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The last refusal.
    pub error: Option<String>,
    /// The walk-through is shown although something is loaded already.
    pub walkthrough: bool,
    /// The seeds typed on the words route, and the wallet of several
    /// keys they make (`seeds.rs`).
    pub seeds: crate::seeds::SeedsState,
    /// Entered on a wallet already open, by Spend from this wallet
    /// (`docs/history/NEW-WALLET.md` §11.2): the column starts at Load the
    /// wallet in Sparrow, and Write the payment brings the PSBT itself.
    pub from_card: bool,
    /// Open from Home or from Wallets: Back goes there.
    pub from: Option<Screen>,
    /// What came in on What do you have?, newest last.
    pub came: Vec<Came>,
    /// That's everything was pressed on one seed.
    pub everything: bool,
    /// I can't find one was pressed: no description to be had.
    pub no_description: bool,
    /// The wallet was worked out from seeds alone.
    pub guess: Option<Guess>,
}

/// The single-key wallets typed words may open, the most common first.
pub const SINGLE: [NewKind; 4] = [
    NewKind::NativeSegwit,
    NewKind::Taproot,
    NewKind::NestedSegwit,
    NewKind::Legacy,
];

/// The source a wallet opened from typed words is loaded under.
pub const WORDS_SOURCE: &str = EN.family_typed_words;

/// The bit of `FamilyState::more` a card's More about this uses.
pub fn more_bit(id: CardId) -> u8 {
    match id {
        CardId::Page(p) => p,
        CardId::Step(n) | CardId::Later(n) => page::COUNT + n,
    }
}

impl Faraday {
    /// Whether the tab opens on what is loaded rather than on its
    /// walk-through: something is loaded, and no route has been taken.
    pub fn family_overview(&self) -> bool {
        !self.family.walkthrough
            && self.family.route.is_none()
            && (!self.session.keys.is_empty() || !self.session.wallets.is_empty())
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

    /// The spend under way, resumed where it was: on the Spend tab when it
    /// went through it, else on Wallets' Sign.
    pub fn carry_on(&self) -> Action {
        let on_tab = self
            .spend
            .as_ref()
            .and_then(|s| s.wallet)
            .filter(|&w| self.family.route.is_some() && self.family_wallet() == Some(w));
        match on_tab {
            Some(w) => Action::Family(FamilyAction::SpendFrom(w)),
            None => Action::Nav(Screen::Spend),
        }
    }

    /// Costs what `wallet_held_on` and the Inbox's PSBTs cost (through
    /// `spend_button_on`). The wallet card's first button
    /// (`docs/history/NEW-WALLET.md` §11.2): finish the backup first while a key
    /// made here waits for it (§14.3), else carry on this wallet's spend,
    /// sign its PSBT in Files, or spend from it.
    pub fn spend_button(&self, c: &Compute, w: usize) -> (String, Action) {
        spend_button_on(&crate::glance::Now { app: self, c }, w)
    }
}

/// [`Faraday::spend_button`] read through `r` ([`crate::glance::Reads`]).
pub(crate) fn spend_button_on(r: &impl crate::glance::Reads, w: usize) -> (String, Action) {
    let app = r.app();
    // A key made here signs once its backup is done
    // (`docs/history/NEW-WALLET.md` §14.3).
    if !crate::held::wallet_held_on(r, w).is_empty() {
        return (
            EN.family_finish_the_backup.to_string(),
            Action::BackupFirst(w),
        );
    }
    if let Some(s) = app.spend.as_ref().filter(|s| s.wallet == Some(w)) {
        return (
            fill(
                EN.family_carry_on_the,
                &[
                    &s.signers.len().to_string(),
                    &app.spend_needed().to_string(),
                ],
            ),
            app.carry_on(),
        );
    }
    let label = match r.psbt_for(w) {
        Some(_) => fill(EN.family_sign_the_psbt, &[app.medium.noun()]),
        None => EN.family_spend_from_this.to_string(),
    };
    (label, Action::Family(FamilyAction::SpendFrom(w)))
}

impl Faraday {
    /// The wallet chosen to spend from, by its place in the session.
    pub fn family_wallet(&self) -> Option<usize> {
        let name = self.family.wallet.as_deref()?;
        self.session.wallets.iter().position(|w| w.name == name)
    }

    /// A wallet is open and chosen.
    pub fn family_ready(&self) -> bool {
        self.family_wallet().is_some()
    }

    /// Whether a page is done.
    pub fn family_page_done(&self, p: u8) -> bool {
        if self.family.from_card {
            match p {
                // Loaded in Sparrow: said so here, or the wallet checked
                // this power-on.
                page::CHECK => {
                    return self.family_ready()
                        && (self.family.closed[p as usize]
                            || self.family_wallet().is_some_and(|i| {
                                self.is_checked(self.session.wallets[i].checksum())
                            }));
                }
                // Write the payment ends with the PSBT here.
                page::WRITE => return self.spend.is_some(),
                _ => {}
            }
        }
        match p {
            page::HOLDING | page::OPEN => self.family_ready(),
            page::CHECK | page::WRITE => self.family.closed[p as usize] && self.family_ready(),
            page::BRING => self.spend.is_some(),
            page::AWAY => false,
            _ => self.family.closed.get(p as usize).copied().unwrap_or(false),
        }
    }

    /// The cards, in order: the pages up to bringing the transaction,
    /// the spend's own steps (named ahead of time until there is one),
    /// then putting everything away.
    pub fn family_cards(&self) -> Vec<CardId> {
        let mut v: Vec<CardId> = self.family_pages().into_iter().map(CardId::Page).collect();
        match self.spend.as_ref() {
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
    pub fn family_pages(&self) -> Vec<u8> {
        if self.family.from_card {
            vec![page::CHECK, page::WRITE]
        } else {
            (0..=page::BRING).filter(|&p| p != page::OPEN).collect()
        }
    }

    /// The page that brings the transaction here.
    pub fn family_bring_page(&self) -> u8 {
        if self.family.from_card {
            page::WRITE
        } else {
            page::BRING
        }
    }

    /// The card open now.
    pub fn family_open_card(&self) -> Option<CardId> {
        match self.family.open? {
            Open::Page(p) => Some(CardId::Page(p)),
            Open::Spend => self.spend.as_ref()?.open.map(CardId::Step),
        }
    }

    /// The first thing not done after page `from`: a page, the spend's
    /// next step, or the last page. A page skipped before it stays where
    /// it is.
    fn family_first_open(&mut self, from: u8) -> Open {
        let pages = self.family_pages();
        if let Some(p) = pages
            .into_iter()
            .find(|&p| p > from && !self.family_page_done(p))
        {
            return Open::Page(p);
        }
        match self.spend.as_mut() {
            Some(s) => {
                let next = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
                match next {
                    Some(n) => {
                        s.open = Some(n);
                        Open::Spend
                    }
                    None => Open::Page(page::AWAY),
                }
            }
            None => Open::Page(page::AWAY),
        }
    }

    fn family_advance(&mut self, from: u8) {
        self.family.open = Some(self.family_first_open(from));
        self.family.scroll.follow = true;
    }

    /// Keeps the open card one that can be reached: no page past What do
    /// you have? without a wallet, and no spend step without a spend.
    pub(crate) fn family_settle(&mut self) {
        self.family_auto_wallet();
        let ready = self.family_ready();
        // The wallet the card opened on is gone: the whole column again.
        if self.family.from_card && !ready {
            self.family.from_card = false;
        }
        // A guess is of the wallet it made: gone with it.
        if self.family.guess.is_some() && !self.family_guessed_wallet() {
            self.family.guess = None;
        }
        let open = match self.family.open {
            Some(Open::Page(page::OPEN)) => Some(Open::Page(page::HOLDING)),
            Some(Open::Page(p)) if p > page::HOLDING && p != page::AWAY && !ready => {
                Some(Open::Page(page::HOLDING))
            }
            Some(Open::Spend) if self.spend.is_none() => Some(Open::Page(if ready {
                self.family_bring_page()
            } else {
                page::HOLDING
            })),
            o => o,
        };
        if open != self.family.open {
            self.family.open = open;
            self.family.scroll.follow = true;
        }
    }

    /// Runs one of the tab's actions.
    pub(crate) fn family_act(&mut self, a: FamilyAction) {
        let c = self.compute();
        use FamilyAction as F;
        if !matches!(a, F::More(_) | F::Unsure | F::MoreAddresses | F::Scan) {
            self.family.error = None;
        }
        match a {
            F::Card(p) => {
                self.family.open = if self.family.open == Some(Open::Page(p)) {
                    None
                } else {
                    Some(Open::Page(p))
                };
                self.family.scroll.follow = true;
            }
            F::Next(p) => {
                if let Some(c) = self.family.closed.get_mut(p as usize) {
                    *c = true;
                }
                // "Yes, my money shows": this wallet is checked this
                // power-on (`docs/WALLETS.md` §4). "It is loaded" says
                // less.
                if p == page::CHECK
                    && !self.family.from_card
                    && let Some(w) = self.family_wallet()
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let checksum = wallet.checksum().to_string();
                    self.mark_checked(&checksum);
                }
                self.family_advance(p);
            }
            F::Unsure => self.family.help = !self.family.help,
            F::Scan => self.family_scan(None),
            F::Everything => {
                self.family.everything = true;
                self.family_words_kind(NewKind::NativeSegwit);
                self.family_guess_start(false);
                self.refresh_spend();
                self.family_opened();
            }
            F::NoDescription => {
                self.family.no_description = true;
                let s = &mut self.family.seeds;
                if !s.shaping {
                    s.start_shape();
                }
            }
            F::StrayPass(fp) => {
                if let Some(w) = self.family_described() {
                    let reason = self.not_a_wallet_key(w, Fingerprint(fp));
                    self.seed_read_stray(Fingerprint(fp), w, reason);
                }
            }
            F::NoMoney => self.family_no_money(),
            F::Passphrase(yes) => {
                if yes {
                    if let Some(fp) = self
                        .family_words_key()
                        .map(|k| self.session.keys[k].fingerprint)
                    {
                        self.seed_read_family(fp, crate::seeds::read::ForFamily::Guess);
                    }
                } else {
                    self.family_guess_step(2);
                }
            }
            F::Again(scan) => {
                self.family_drop_guess();
                if scan {
                    self.family_scan(None);
                } else if let Some(fp) = self.family.seeds.keys.first().copied() {
                    self.seed_read_family(Fingerprint(fp), crate::seeds::read::ForFamily::Replace);
                }
            }
            F::More(k) => self.family.more ^= 1 << u32::from(k),
            F::Pick(i) => {
                self.vault_act(VaultAction::Pick(i));
                self.vault_measure();
            }
            F::Unlock => {
                self.vaults.back_to = None;
                self.vault_measure();
                self.vault_act(VaultAction::Unlock);
            }
            F::TypeWords => {
                if self.may_load_keys() {
                    // From the intake scanner's Type the words instead.
                    if self.scan.take().is_some() {
                        self.commands.push_back(Command::CameraOff);
                    }
                    if self.sheet == Some(Sheet::Scan) {
                        self.sheet = None;
                    }
                    self.entry = crate::EntryState::default();
                    self.entry.back = Some(Screen::Family);
                    if self.compact {
                        self.entry.keys = Some(crate::forms::word_typer(self.entry.language()));
                    }
                    self.screen = Screen::Entry;
                }
            }
            F::UseWallet(i) => {
                if let Some(name) = self.inbox.get(i).map(|it| it.name.clone()) {
                    self.family_came(Came::File(name));
                }
                self.family_use_wallet(i);
            }
            F::Rebuild => {
                let names: Vec<String> = self
                    .inbox
                    .iter()
                    .filter(|i| i.kind == FileKind::Share)
                    .map(|i| i.name.clone())
                    .collect();
                for n in names {
                    self.family_came(Came::File(n));
                }
                self.family_rebuild(true);
            }
            F::Choose(i) => {
                if let Some(w) = self.session.wallets.get(i) {
                    self.family.wallet = Some(w.name.clone());
                    self.wallet = i;
                    if self.family.open == Some(Open::Page(page::HOLDING)) {
                        self.family_advance(page::HOLDING);
                    }
                }
            }
            F::UsePsbt(i) => self.family_use_psbt(i),
            F::MoreAddresses => self.family.addresses = (self.family.addresses + 5).min(47),
            F::QrTx => {
                if let Some(hex) = self.spend.as_ref().and_then(|s| s.spend.finished_hex()) {
                    self.open_qr(QrView::text(&c, EN.family_finished_transaction, &hex));
                }
            }
            F::SpendFrom(i) => {
                let Some(w) = self.session.wallets.get(i) else {
                    return;
                };
                // Another wallet's pages said nothing of this one.
                if self.family.wallet.as_deref() != Some(w.name.as_str()) {
                    self.family.closed[page::CHECK as usize] = false;
                    self.family.closed[page::WRITE as usize] = false;
                    self.family.addresses = 0;
                }
                // Another wallet's spend gives way, as a new Sign does;
                // its PSBT stays in Files.
                if self.spend.as_ref().is_some_and(|s| s.wallet != Some(i)) {
                    self.spend = None;
                }
                let w = &self.session.wallets[i];
                self.family.wallet = Some(w.name.clone());
                self.wallet = i;
                self.family.from_card = true;
                self.family.help = false;
                self.screen = Screen::Family;
                self.family.route = Some(match w.origin {
                    crate::wallet::Origin::TypedWords | crate::wallet::Origin::TypedSeeds => {
                        Route::Words
                    }
                    crate::wallet::Origin::Vault => Route::Vault,
                    crate::wallet::Origin::Statement | crate::wallet::Origin::Named => Route::Paper,
                });
                // Someone who loaded a wallet already knows the map and
                // the stick.
                self.family.closed[page::MAP as usize] = true;
                self.family.closed[page::SAFE as usize] = true;
                self.family.walkthrough = true;
                let psbt = self.psbt_for(&c, i);
                match (self.spend.as_ref().map(|s| s.wallet), psbt) {
                    (Some(Some(w)), _) if w == i => self.family.open = Some(Open::Spend),
                    (_, Some(k)) if self.spend.is_none() => self.family_use_psbt(k),
                    _ => self.family_advance(page::HOLDING),
                }
                self.family.scroll.follow = true;
            }
            F::Walkthrough => {
                self.family.walkthrough = true;
                self.family.from_card = false;
                self.family.open = Some(Open::Page(page::MAP));
                self.family.scroll = Default::default();
            }
            F::SeedWallet => {
                self.family.from_card = false;
                self.family.route = Some(Route::Words);
                self.family.walkthrough = true;
                self.family.closed[page::MAP as usize] = true;
                self.family.closed[page::SAFE as usize] = true;
                self.family.everything = true;
                self.family_words_kind(NewKind::NativeSegwit);
                self.family_guess_start(false);
                self.refresh_spend();
                self.family_advance(page::HOLDING);
            }
            F::StartOver => {
                self.family = FamilyState {
                    open: Some(Open::Page(page::HOLDING)),
                    closed: {
                        let mut c = [false; page::COUNT as usize];
                        c[page::MAP as usize] = self.family.closed[page::MAP as usize];
                        c[page::SAFE as usize] = self.family.closed[page::SAFE as usize];
                        c
                    },
                    walkthrough: true,
                    ..FamilyState::default()
                };
                self.family.scroll.follow = true;
            }
        }
        self.family_settle();
    }

    /// With no wallet open yet, the description in Files opens by itself
    /// when there is just one: one wallet file, or split sheets that add
    /// up. After a lock, of the files that came in on What do you have?;
    /// on the vault route, when the open vaults hold seeds but no wallet,
    /// of every file in Files. Several wallet files wait for the person
    /// to choose.
    fn family_auto_wallet(&mut self) {
        let c = self.compute();
        if !self.session.wallets.is_empty() {
            return;
        }
        let all = match self.family.route {
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
        let mut wallets: Vec<(usize, String)> = Vec::new();
        for (k, it) in self.inbox.iter().enumerate() {
            if it.kind != FileKind::Wallet
                || !(all || self.family.came.contains(&Came::File(it.name.clone())))
            {
                continue;
            }
            if let Some(p) = it.wallet() {
                let d = p.to_descriptor();
                if !wallets.iter().any(|(_, x)| *x == d) {
                    wallets.push((k, d));
                }
            }
        }
        match wallets.as_slice() {
            [(one, _)] => self.family_use_wallet(*one),
            [] => self.family_rebuild(false),
            _ => {}
        }
    }

    /// The wallet to spend from, when the session says which: the only
    /// one, or the one a PSBT waiting in Files spends from.
    fn family_choose_only(&mut self) {
        let c = self.compute();
        if self.family_wallet().is_some() {
            return;
        }
        let only = (self.session.wallets.len() == 1).then_some(0);
        let by_psbt = || {
            self.lead_psbt()
                .and_then(|i| self.inbox[i].psbt())
                .and_then(|p| self.session.spends_from(&c, p))
        };
        if let Some(i) = only.or_else(by_psbt) {
            self.family.wallet = Some(self.session.wallets[i].name.clone());
            self.wallet = i;
        }
    }

    /// After a wallet arrives: chosen when it is the only one, and the
    /// next page opens once there is one, unless a seed that came in is
    /// none of its keys, which What do you have? says.
    pub(crate) fn family_opened(&mut self) {
        self.family_choose_only();
        if let Some(i) = self.family_wallet() {
            self.wallet = i;
        }
        if self.family_ready()
            && matches!(
                self.family.open,
                Some(Open::Page(page::HOLDING | page::OPEN))
            )
            && self.family_strays().is_empty()
        {
            self.family_advance(page::HOLDING);
        }
    }

    /// A vault unlocked from this tab: everything in it is chosen to
    /// load, since spending needs both the wallet and its seeds, and the
    /// person loads it or less from the list. The wallet is opened from
    /// the vault from now on.
    pub(crate) fn family_unlocked(&mut self, v: usize) {
        if let Some(o) = self.vaults.open.get_mut(v) {
            o.skip.clear();
        }
        self.family.route = Some(Route::Vault);
        self.family.walkthrough = true;
    }

    /// A vault's seeds loaded on What do you have? with no wallet: they
    /// came in as seeds, and the next question asks for the rest.
    pub(crate) fn family_vault_loaded(&mut self) {
        if !self.session.wallets.is_empty() {
            return;
        }
        let fps: Vec<[u8; 4]> = self.session.keys.all().map(|k| k.fingerprint.0).collect();
        for fp in fps {
            self.family.seeds.take(fp);
            self.family_came(Came::Seed(fp));
        }
        if self.family.seeds.keys.len() >= 2 {
            self.family_several_seeds();
        }
    }

    /// A key typed or scanned from this tab: it fills the described
    /// wallet's slot; with no description it waits for the next
    /// question (`docs/SCAN-TO-SIGN.md` §4.3).
    pub(crate) fn family_key_added(&mut self) {
        self.refresh_spend();
        if self.family_described().is_some() {
            self.family_opened();
        }
        self.family_settle();
    }

    /// Something came in: listed once, newest last.
    pub(crate) fn family_came(&mut self, c: Came) {
        self.family.walkthrough = true;
        if !self.family.came.contains(&c) {
            self.family.came.push(c);
        }
    }

    /// The wallet chosen, when it came from a description (a file, the
    /// split sheets, a vault) rather than from seeds alone.
    pub fn family_described(&self) -> Option<usize> {
        let w = self.family_wallet()?;
        let origin = self.session.wallets[w].origin;
        (!matches!(
            origin,
            crate::wallet::Origin::TypedWords | crate::wallet::Origin::TypedSeeds
        ))
        .then_some(w)
    }

    /// The seeds that came in that are none of the described wallet's
    /// keys.
    pub fn family_strays(&self) -> Vec<Fingerprint> {
        let Some(w) = self.family_described() else {
            return Vec::new();
        };
        self.family
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
    pub fn family_question(&self) -> Question {
        if self.family_ready() || self.family_described().is_some() {
            return Question::None;
        }
        // Several wallets loaded, from a vault say: the choice is asked.
        if !self.session.wallets.is_empty() && self.family.route == Some(Route::Vault) {
            return Question::None;
        }
        match self.seeds_here(&self.family.seeds).len() {
            0 => Question::None,
            1 if self.family.everything => Question::None,
            1 => Question::Anything,
            _ if self.family.no_description => Question::Shape,
            _ => Question::Description,
        }
    }

    /// The hint the intake scanner's sheet gives for the question asked.
    pub fn family_hint(&self) -> Option<&'static str> {
        match self.family_question() {
            Question::Anything => Some(crate::family::text::ANYTHING_HINT),
            Question::Description => Some(crate::family::text::DESCRIPTION_HINT),
            _ => None,
        }
    }

    /// The intake scanner: anything, one code after another, until Done.
    pub(crate) fn family_scan(&mut self, note: Option<String>) {
        let mut scan = ScanState::default();
        if let Some((decoder, assembler)) = self.scan_parts.take() {
            scan.decoder = decoder;
            scan.assembler = assembler;
        }
        scan.purpose = ScanPurpose::Intake;
        scan.note = note;
        self.scan = Some(scan);
        self.sheet = Some(Sheet::Scan);
        self.commands.push_back(Command::CameraOn);
        if !self.family_ready() {
            self.family.open = Some(Open::Page(page::HOLDING));
        }
    }

    /// The intake scanner read a seed's words. With a description in,
    /// a seed whose key with no passphrase is one of its missing keys
    /// loads at once; one that is none of its keys asks for its
    /// passphrase. Else the passphrase sheet asks how it loads, and the
    /// camera comes back on after.
    pub(crate) fn family_intake_seed(&mut self, words: &str) {
        let c = self.compute();
        let Some(fp) = key_of(&c, words, "") else {
            if let Some(scan) = self.scan.as_mut() {
                scan.note = Some(EN.family_not_a_seedqr.to_string());
            }
            return;
        };
        let Some(w) = self.family_described() else {
            self.seed_read_open(words);
            if let Some(s) = self.seed_read.as_mut() {
                s.family = Some(crate::seeds::read::ForFamily::Intake);
            }
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
                        self.family_came(Came::Seed(fp.0));
                        self.refresh_spend();
                        fill(EN.family_seed_read, &[&fp_text(fp).to_string()])
                    }
                    Err(e) => e.text(),
                }
            }
            Found::Here(n) => fill(EN.family_key_already, &[&(n + 1).to_string()]),
            Found::None => {
                let reason = self.not_a_wallet_key(w, fp);
                self.seed_read_for_wallet(words, w, reason);
                if let Some(s) = self.seed_read.as_mut() {
                    s.family = Some(crate::seeds::read::ForFamily::Intake);
                }
                return;
            }
        };
        if let Some(scan) = self.scan.as_mut() {
            scan.note = Some(note);
        }
    }

    /// A file the intake scanner put in Files: listed, and taken as what
    /// it is. The camera stays on.
    pub(crate) fn family_intake_file(&mut self, index: usize) {
        let Some((kind, name)) = self.inbox.get(index).map(|i| (i.kind, i.name.clone())) else {
            return;
        };
        self.family_came(Came::File(name.clone()));
        self.family.error = None;
        let note = match kind {
            FileKind::Wallet => {
                if self.family_ready() && self.family_described().is_some() {
                    EN.family_wallet_description_read.to_string()
                } else {
                    self.family_use_wallet(index);
                    EN.family_wallet_description.to_string()
                }
            }
            FileKind::Share => {
                let sheets = self.family_sheets();
                let k = sheets.len();
                if !self.family_ready() {
                    self.family_rebuild(false);
                }
                match crate::restore::merge(&sheets) {
                    Ok(m) if m.whole.is_some() && self.family_ready() => {
                        fill(EN.family_part_read_the, &[&k.to_string()])
                    }
                    Ok(m) => fill(
                        EN.family_part_read_of,
                        &[&k.to_string(), &m.have.len().to_string(), &m.n.to_string()],
                    ),
                    Err(e) => e,
                }
            }
            FileKind::Key => {
                self.seeds_take_file(index);
                EN.family_a_public_key.to_string()
            }
            FileKind::Psbt => {
                self.family_psbt_kinds(index);
                EN.family_a_transaction.to_string()
            }
            _ => fill(EN.family_is_in_files, &[&name.to_string()]),
        };
        let note = self.family.error.clone().unwrap_or(note);
        if let Some(scan) = self.scan.as_mut() {
            scan.note = Some(note);
        }
    }

    /// The split sheets that came in on What do you have?, as text.
    fn family_sheets(&self) -> Vec<String> {
        self.inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .filter(|i| self.family.came.contains(&Came::File(i.name.clone())))
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect()
    }

    /// A transaction that came in with one seed and nothing else: the
    /// single-key wallet of the kind it spends from opens, its kind
    /// shown by the transaction, not guessed. It does not start the
    /// spend; Bring the transaction here finds it in Files.
    fn family_psbt_kinds(&mut self, index: usize) {
        let c = self.compute();
        if self.family_ready() || self.family_question() != Question::Anything {
            return;
        }
        let Some(p) = self.inbox.get(index).and_then(|i| i.psbt()).cloned() else {
            return;
        };
        for kind in SINGLE {
            self.family_words_kind(kind);
            if self.family_wallet().is_some()
                && self.session.spends_from(&c, &p) == self.family_wallet()
            {
                self.family.everything = true;
                self.family.guess = None;
                self.refresh_spend();
                self.family_opened();
                return;
            }
        }
        // Only a wallet guessed from Family's own typed words is dropped
        // here; one guessed from Seeds' typed seeds (`Origin::TypedSeeds`)
        // is left alone. Deliberate, not an oversight: the two screens
        // keep their own guesses on their own terms.
        self.session
            .wallets
            .retain(|w| w.origin != crate::wallet::Origin::TypedWords);
        self.family.wallet = None;
        self.wallet = self
            .wallet
            .min(self.session.wallets.len().saturating_sub(1));
    }

    /// A seed whose sheet loaded it from the intake scanner: the camera
    /// comes back on for the next code.
    pub(crate) fn family_intake_again(&mut self, fp: Option<Fingerprint>) {
        let note = fp.map(|fp| fill(EN.family_seed_read, &[&fp_text(fp).to_string()]));
        self.family_scan(note);
    }

    /// Whether the wallet chosen is the one guessed from seeds alone.
    fn family_guessed_wallet(&self) -> bool {
        self.family_wallet().is_some_and(|w| {
            matches!(
                self.session.wallets[w].origin,
                crate::wallet::Origin::TypedWords | crate::wallet::Origin::TypedSeeds
            )
        })
    }

    /// The wallet just made from seeds alone is a guess: Check the money
    /// tries the other kinds (`docs/SCAN-TO-SIGN.md` §5).
    pub(crate) fn family_guess_start(&mut self, multi: bool) {
        let at = if multi {
            MULTI_ORDER
                .iter()
                .position(|k| *k == self.family.seeds.kind)
                .unwrap_or(0)
        } else {
            0
        };
        self.family.guess = Some(Guess {
            multi,
            at,
            ..Guess::default()
        });
        self.family.closed[page::CHECK as usize] = false;
    }

    /// The guessed wallet's ways, and the one tried now.
    pub fn family_guess(&self) -> Option<&Guess> {
        self.family
            .guess
            .as_ref()
            .filter(|_| self.family_guessed_wallet())
    }

    /// The way `at` of a guess, as listed: its kind, and for one key
    /// whether a passphrase goes with it.
    pub fn guess_way(g: &Guess, at: usize) -> (NewKind, bool) {
        if g.multi {
            (MULTI_ORDER[at.min(MULTI_ORDER.len() - 1)], false)
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
    fn family_no_money(&mut self) {
        let Some(g) = self.family.guess.as_mut() else {
            return;
        };
        let (kind, pass) = Self::guess_way(g, g.at);
        let mut label = kind_label(kind).to_string();
        if pass {
            label.push(' ');
            label.push_str(EN.family_with_a_passphrase);
        }
        g.tried
            .push(fill(EN.family_no_money, &[&label.to_string()]));
        if !g.multi && g.at == 0 {
            g.asking = true;
            return;
        }
        let next = g.at + 1;
        self.family_guess_step(next);
    }

    /// The guess goes on to way `at`: the wallet made again of that
    /// kind, or None of these show money after the last.
    pub(crate) fn family_guess_step(&mut self, at: usize) {
        let Some(g) = self.family.guess.as_mut() else {
            return;
        };
        g.asking = false;
        if at >= Self::guess_total(g) {
            g.none = true;
            return;
        }
        g.at = at;
        let (kind, _) = Self::guess_way(g, at);
        let multi = g.multi;
        self.family.addresses = 0;
        self.family.closed[page::CHECK as usize] = false;
        if multi {
            self.family_seeds_kind(kind);
        } else {
            self.family_words_kind(kind);
        }
        self.refresh_spend();
    }

    /// The wallet of several keys made again as `kind`, in place of the
    /// one made before, under the same name.
    fn family_seeds_kind(&mut self, kind: NewKind) {
        let c = self.compute();
        let Some(w) = self.family_wallet() else {
            return;
        };
        self.family.seeds.kind = kind;
        let s = self.family.seeds.clone();
        let d = match self.seeds_descriptor(&c, &s) {
            Ok(d) => d,
            Err(e) => {
                self.family.error = Some(e);
                return;
            }
        };
        let name = self.session.wallets[w].name.clone();
        self.session.wallets.remove(w);
        match self.session.add_wallet(
            &c,
            &name,
            &d,
            crate::seeds::SOURCE,
            crate::wallet::Origin::TypedSeeds,
        ) {
            Ok(i) => {
                self.family.wallet = Some(self.session.wallets[i].name.clone());
                self.wallet = i;
            }
            Err(e) => self.family.error = Some(e.text()),
        }
    }

    /// The seed's passphrase was typed for the guess: the seed is its
    /// key with the passphrase from now on, and native SegWit with it is
    /// tried. With none typed, the guess goes on without one.
    pub(crate) fn family_guess_pass(
        &mut self,
        plain: Fingerprint,
        source: &crate::seeds::read::Source,
        pass: &str,
    ) {
        if pass.is_empty() {
            self.family_guess_step(2);
            return;
        }
        if self.family_seed_with(plain, source, pass).is_none() {
            return;
        }
        if let Some(g) = self.family.guess.as_mut() {
            g.pass = true;
        }
        self.family_guess_step(1);
    }

    /// Seed `plain` replaced by its key with `pass`, on What do you
    /// have? and in its list. Returns the new key.
    pub(crate) fn family_seed_with(
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
                    EN.family_passphrase_inline.to_string()
                } else {
                    fill(EN.family_passphrase_n, &[l])
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
        for k in self.family.seeds.keys.iter_mut() {
            if *k == plain.0 {
                *k = fp.0;
            }
        }
        if self.family.seeds.keys.is_empty() {
            self.family.seeds.keys.push(fp.0);
        }
        for c in self.family.came.iter_mut() {
            if *c == Came::Seed(plain.0) {
                *c = Came::Seed(fp.0);
            }
        }
        Some(fp)
    }

    /// Back to What do you have? after every kind was tried: the guessed
    /// wallet goes, and the questions are asked again.
    fn family_drop_guess(&mut self) {
        if let Some(w) = self
            .family_wallet()
            .filter(|_| self.family_guessed_wallet())
        {
            self.session.wallets.remove(w);
        }
        // Only a wallet guessed from Family's own typed words is dropped
        // here; one guessed from Seeds' typed seeds (`Origin::TypedSeeds`)
        // is left alone. Deliberate, not an oversight: the two screens
        // keep their own guesses on their own terms.
        self.session
            .wallets
            .retain(|w| w.origin != crate::wallet::Origin::TypedWords);
        self.family.wallet = None;
        self.wallet = self
            .wallet
            .min(self.session.wallets.len().saturating_sub(1));
        self.family.guess = None;
        self.family.everything = false;
        self.family.no_description = false;
        self.family.seeds.shaping = false;
        self.family.closed[page::CHECK as usize] = false;
        self.family.addresses = 0;
        self.refresh_spend();
        self.family.open = Some(Open::Page(page::HOLDING));
        self.family.scroll.follow = true;
    }

    /// The key the words route's single-key wallet is opened from: the
    /// first seed typed on the route, else the last loaded.
    pub(crate) fn family_words_key(&self) -> Option<usize> {
        self.family
            .seeds
            .keys
            .first()
            .and_then(|fp| self.session.keys.position_of(*fp))
            .or(self.session.keys.len().checked_sub(1))
    }

    /// The wallet the last key typed opens as `kind`, in place of the one
    /// it opened before.
    fn family_words_kind(&mut self, kind: NewKind) {
        let c = self.compute();
        let Some(key) = self.family_words_key() else {
            return;
        };
        let text = match self.session.keys.account_text(&c, key, kind, 0) {
            Ok(t) => t,
            Err(e) => {
                self.family.error = Some(e);
                return;
            }
        };
        let descriptor = kind.descriptor(1, &[text]);
        // Only a wallet guessed from Family's own typed words is dropped
        // here; one guessed from Seeds' typed seeds (`Origin::TypedSeeds`)
        // is left alone. Deliberate, not an oversight: the two screens
        // keep their own guesses on their own terms.
        self.session
            .wallets
            .retain(|w| w.origin != crate::wallet::Origin::TypedWords);
        let name = fill(EN.family_words, &[kind_name(kind)]);
        match self.session.add_wallet(
            &c,
            &name,
            &descriptor,
            WORDS_SOURCE,
            crate::wallet::Origin::TypedWords,
        ) {
            Ok(i) => {
                self.family.wallet = Some(name);
                self.wallet = i;
            }
            Err(e) => self.family.error = Some(e.text()),
        }
    }

    /// The single-key kind the words route's wallet is, if it is one.
    pub fn family_words_kind_now(&self) -> Option<NewKind> {
        // Family's own typed-words wallet only: Seeds' typed-seeds one
        // (`Origin::TypedSeeds`) is a different guess, not this one.
        let w = self
            .session
            .wallets
            .iter()
            .find(|w| w.origin == crate::wallet::Origin::TypedWords)?;
        SINGLE
            .into_iter()
            .find(|k| w.name == fill(EN.family_words, &[kind_name(*k)]))
    }

    fn family_use_wallet(&mut self, index: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let text = String::from_utf8_lossy(&item.bytes).into_owned();
        let name = faraday_bip::multisig_config::parse_named(&c, &text)
            .ok()
            .and_then(|(_, n)| n)
            .or_else(|| crate::wallet::json_label(&text))
            .unwrap_or_else(|| crate::file_stem(&item.name));
        let source = item.name.clone();
        match self
            .session
            .add_wallet(&c, &name, &text, &source, crate::wallet::Origin::Named)
        {
            Ok(i) => {
                self.family.wallet = Some(self.session.wallets[i].name.clone());
                self.wallet = i;
                self.family_described_in();
                self.refresh_spend();
                self.family_opened();
            }
            Err(e) => self.family.error = Some(e.text()),
        }
    }

    /// A description came in: the wallet is opened from it, and a guess
    /// made from seeds before it goes.
    fn family_described_in(&mut self) {
        if self.family.route != Some(Route::Vault) {
            self.family.route = Some(Route::Paper);
        }
        self.family.walkthrough = true;
        self.family.guess = None;
        // Only a wallet guessed from Family's own typed words is dropped
        // here; one guessed from Seeds' typed seeds (`Origin::TypedSeeds`)
        // is left alone. Deliberate, not an oversight: the two screens
        // keep their own guesses on their own terms.
        self.session
            .wallets
            .retain(|w| w.origin != crate::wallet::Origin::TypedWords);
        if let Some(i) = self.family_wallet() {
            self.wallet = i;
        }
    }

    /// The split sheets in the Inbox, put together. Says what is still
    /// missing only when asked (`loud`).
    fn family_rebuild(&mut self, loud: bool) {
        let c = self.compute();
        let texts: Vec<String> = self
            .inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect();
        if texts.is_empty() {
            return;
        }
        match crate::restore::merge(&texts) {
            Ok(m) => match m.whole {
                Some(whole) => {
                    let name = faraday_bip::multisig_config::parse_named(&c, &whole)
                        .ok()
                        .and_then(|(_, n)| n)
                        .unwrap_or_else(|| EN.family_restored_wallet.to_string());
                    match self.session.add_wallet(
                        &c,
                        &name,
                        &whole,
                        EN.family_its_shares,
                        crate::wallet::Origin::Named,
                    ) {
                        Ok(i) => {
                            self.family.wallet = Some(self.session.wallets[i].name.clone());
                            self.wallet = i;
                            self.family_described_in();
                            self.refresh_spend();
                            self.family_opened();
                        }
                        Err(e) => self.family.error = Some(e.text()),
                    }
                }
                None if loud => {
                    self.family.error = Some(fill(
                        EN.family_of_keys_in_hand,
                        &[&m.have.len().to_string(), &m.n.to_string()],
                    ));
                }
                None => {}
            },
            Err(e) => {
                if loud {
                    self.family.error = Some(e);
                }
            }
        }
    }

    /// Starts signing an Inbox PSBT without leaving the tab. On the words
    /// route a transaction that names another kind of single-key wallet
    /// opens that kind: the transaction's own key paths say which.
    fn family_use_psbt(&mut self, index: usize) {
        self.start_spend(index);
        self.screen = Screen::Family;
        let Some(s) = self.spend.as_ref() else {
            return;
        };
        if s.wallet.is_none()
            && self.family.route == Some(Route::Words)
            && self.family.seeds.keys.len() <= 1
        {
            let before = self.family_words_kind_now();
            let mut found = false;
            for kind in SINGLE {
                self.family_words_kind(kind);
                self.refresh_spend();
                if self.spend.as_ref().is_some_and(|s| s.wallet.is_some()) {
                    found = true;
                    break;
                }
            }
            if !found && let Some(k) = before {
                self.family_words_kind(k);
                self.refresh_spend();
            }
        }
        // The wallet the transaction spends from is the one chosen.
        let w = self.spend.as_ref().and_then(|s| s.wallet);
        if let Some(w) = w
            && let Some(wl) = self.session.wallets.get(w)
        {
            self.family.wallet = Some(wl.name.clone());
            self.wallet = w;
        }
        if let Some(s) = self.spend.as_mut() {
            // The addresses were checked on the tab's own page.
            s.done[step::WALLET as usize] = true;
            s.done[step::CHECK as usize] = true;
            s.open = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
        }
        self.family.open = Some(Open::Spend);
        self.family.scroll.follow = true;
    }

    /// A file that arrived by camera while the tab is open: a wallet or
    /// the last split sheet opens the wallet, a PSBT starts the spend.
    /// Returns whether the tab took it.
    pub(crate) fn family_arrived(&mut self, index: usize) -> bool {
        let Some(kind) = self.inbox.get(index).map(|i| i.kind) else {
            return false;
        };
        match kind {
            FileKind::Wallet if !self.family_ready() => {
                self.family_use_wallet(index);
                true
            }
            FileKind::Share if !self.family_ready() => {
                self.family_rebuild(false);
                true
            }
            FileKind::Psbt if self.family_ready() || self.family.route.is_some() => {
                self.family_use_psbt(index);
                true
            }
            _ => false,
        }
    }

    /// After the spend's own step cards change on this tab: the card open
    /// follows the spend's, and the last page opens after the last step.
    pub(crate) fn family_step_moved(&mut self, closed_as_done: bool) {
        let open = self.spend.as_ref().and_then(|s| s.open);
        self.family.open = match (open, closed_as_done) {
            (Some(_), _) => Some(Open::Spend),
            (None, true) => Some(Open::Page(page::AWAY)),
            (None, false) => None,
        };
        self.family.scroll.follow = true;
    }

    /// What the tab keeps across a lock: nothing secret.
    pub(crate) fn family_kept(&self) -> Option<Vec<u8>> {
        let active = self.screen == Screen::Family
            || (self.screen == Screen::Entry && self.entry.back == Some(Screen::Family));
        if self.family.route.is_none() && self.family.came.is_empty() && !active {
            return None;
        }
        let open = match self.family.open {
            Some(Open::Page(p)) => p.to_string(),
            Some(Open::Spend) => "spend".to_string(),
            None => String::new(),
        };
        let closed: Vec<String> = (0..page::COUNT)
            .filter(|&p| self.family.closed[p as usize])
            .map(|p| p.to_string())
            .collect();
        let mut text = format!(
            "active={}\nroute={}\nopen={open}\nclosed={}\n",
            u8::from(active),
            self.family.route.map(Route::code).unwrap_or(""),
            closed.join(",")
        );
        if let Some(w) = &self.family.wallet {
            text.push_str(&format!("wallet={}\n", w.replace('\n', " ")));
        }
        if self.family.from_card {
            text.push_str("card=1\n");
        }
        // The files that came in; seeds come back by the vault or not
        // at all.
        for c in &self.family.came {
            if let Came::File(name) = c {
                text.push_str(&format!("came={}\n", name.replace('\n', " ")));
            }
        }
        Some(text.into_bytes())
    }

    /// Reads back what the previous process kept.
    pub(crate) fn family_restore(&mut self, bytes: &[u8]) {
        let mut f = FamilyState::default();
        let mut active = false;
        for line in String::from_utf8_lossy(bytes).lines() {
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
                Some(("wallet", v)) if !v.is_empty() => f.wallet = Some(v.to_string()),
                Some(("card", v)) => f.from_card = v == "1",
                Some(("came", v)) if !v.is_empty() => f.came.push(Came::File(v.to_string())),
                _ => {}
            }
        }
        f.scroll.follow = true;
        self.family = f;
        if active {
            self.screen = Screen::Family;
        }
        self.family_settle();
    }
}

/// A single-key kind's name, as the words route shows it.
pub fn kind_name(kind: NewKind) -> &'static str {
    match kind {
        NewKind::NativeSegwit => EN.family_native_segwit_2,
        NewKind::Taproot => EN.family_taproot,
        NewKind::NestedSegwit => EN.family_nested_segwit_2,
        NewKind::Legacy => EN.family_legacy,
        _ => "",
    }
}

/// A kind's name as Check the money lists the ways tried: the
/// single-key names, and the multisig kinds named as such.
pub fn kind_label(kind: NewKind) -> &'static str {
    match kind {
        NewKind::Multi => EN.family_native_segwit,
        NewKind::TapMulti => EN.family_taproot_multisig,
        NewKind::MultiNested => EN.family_nested_segwit,
        NewKind::MultiLegacy => EN.family_legacy_multisig,
        k => kind_name(k),
    }
}

/// The action a card's header press takes.
pub fn toggle(app: &Faraday, id: CardId) -> Action {
    match id {
        CardId::Page(p) => Action::Family(FamilyAction::Card(p)),
        CardId::Step(n) => Action::Step(n),
        CardId::Later(_) => Action::Family(FamilyAction::Card(app.family_bring_page())),
    }
}
