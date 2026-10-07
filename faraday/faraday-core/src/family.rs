//! The Spend tab (`docs/FAMILY.md`): one column that
//! takes someone from what is in the envelope to a sent transaction.
//!
//! It is a wording layer over controls that exist elsewhere: Unlock, word
//! entry, Restore's sources, the spend's own steps and the QR sheet, all
//! acting on the same session, vaults and spend as every other screen, so
//! the same paper gives the same addresses and signatures here as in
//! Wallets. What the tab keeps of its own is the route chosen, the card
//! open, the pages the person closed and the wallet chosen; none of it is
//! secret, and it is kept across a lock, so a lock for a stick or for
//! idleness comes back to the same page.

use crate::create::NewKind;
use crate::vaults::VaultAction;
use crate::wallet::{FileKind, step};
use crate::{Action, Faraday, QrView, Screen, flow};

/// What the person is holding: the way the wallet is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// A Faraday stick and its passphrase: a vault.
    Vault,
    /// Words and nothing else: one key.
    Words,
    /// Words and a wallet description.
    Paper,
}

impl Route {
    /// Every route, in the order the answers are shown.
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
    /// What are you holding?
    pub const HOLDING: u8 = 2;
    /// Open the wallet.
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
    /// The answer to What are you holding?
    Holding(Route),
    /// I am not sure: the list of what each thing looks like.
    Unsure,
    /// Show or hide a card's More about this, by `more_bit`.
    More(u8),
    /// Choose the vault to unlock, by its place in the vault list.
    Pick(usize),
    /// Unlock the chosen vault.
    Unlock,
    /// Type the words.
    TypeWords,
    /// The kind of single-key wallet the typed words open, by its place
    /// in `SINGLE`.
    Kind(u8),
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

/// The tab's state.
#[derive(Debug, Default)]
pub struct FamilyState {
    /// What the person is holding.
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
}

/// The single-key wallets typed words may open, the most common first.
pub const SINGLE: [NewKind; 4] = [
    NewKind::NativeSegwit,
    NewKind::Taproot,
    NewKind::NestedSegwit,
    NewKind::Legacy,
];

/// The source a wallet opened from typed words is loaded under.
pub const WORDS_SOURCE: &str = "Typed words";

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
        match p {
            page::HOLDING => self.family.route.is_some(),
            page::OPEN => self.family_ready(),
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
        let mut v: Vec<CardId> = (0..=page::BRING).map(CardId::Page).collect();
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
        if let Some(p) = (from + 1..=page::BRING).find(|&p| !self.family_page_done(p)) {
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

    /// Keeps the open card one that can be reached: no page past What
    /// are you holding? without an answer, none past Open the wallet
    /// without a wallet, and no spend step without a spend.
    pub(crate) fn family_settle(&mut self) {
        self.family_auto_wallet();
        let ready = self.family_ready();
        let open = match self.family.open {
            Some(Open::Page(p))
                if p > page::HOLDING && p != page::AWAY && self.family.route.is_none() =>
            {
                Some(Open::Page(page::HOLDING))
            }
            Some(Open::Page(p)) if p > page::OPEN && p != page::AWAY && !ready => {
                Some(Open::Page(page::OPEN))
            }
            Some(Open::Spend) if self.spend.is_none() => {
                Some(Open::Page(if ready { page::BRING } else { page::OPEN }))
            }
            o => o,
        };
        if open != self.family.open {
            self.family.open = open;
            self.family.scroll.follow = true;
        }
    }

    /// Runs one of the tab's actions.
    pub(crate) fn family_act(&mut self, a: FamilyAction) {
        use FamilyAction as F;
        if !matches!(a, F::More(_) | F::Unsure | F::MoreAddresses) {
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
                self.family_advance(p);
            }
            F::Holding(r) => {
                self.family.route = Some(r);
                self.family.help = false;
                self.family.open = Some(Open::Page(page::OPEN));
                self.family.scroll.follow = true;
                // A wallet already open on another tab, or before a lock,
                // is the one to spend from when it is the only one.
                self.family_choose_only();
            }
            F::Unsure => self.family.help = !self.family.help,
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
                    self.entry = crate::EntryState::default();
                    self.entry.back = Some(Screen::Family);
                    self.screen = Screen::Entry;
                }
            }
            F::Kind(k) => {
                if let Some(&kind) = SINGLE.get(usize::from(k)) {
                    self.family_words_kind(kind);
                    self.refresh_spend();
                }
            }
            F::UseWallet(i) => self.family_use_wallet(i),
            F::Rebuild => self.family_rebuild(true),
            F::Choose(i) => {
                if let Some(w) = self.session.wallets.get(i) {
                    self.family.wallet = Some(w.name.clone());
                    self.wallet = i;
                }
            }
            F::UsePsbt(i) => self.family_use_psbt(i),
            F::MoreAddresses => self.family.addresses = (self.family.addresses + 5).min(47),
            F::QrTx => {
                if let Some(hex) = self.spend.as_ref().and_then(|s| s.spend.finished_hex()) {
                    self.open_qr(QrView::text("Finished transaction", &hex));
                }
            }
            F::SpendFrom(i) => {
                let Some(w) = self.session.wallets.get(i) else {
                    return;
                };
                self.family.wallet = Some(w.name.clone());
                self.wallet = i;
                self.family.route = Some(match w.source.as_str() {
                    WORDS_SOURCE => Route::Words,
                    "Vault" => Route::Vault,
                    _ => Route::Paper,
                });
                // Someone who loaded a wallet already knows the map and
                // the stick.
                self.family.closed[page::MAP as usize] = true;
                self.family.closed[page::SAFE as usize] = true;
                self.family.walkthrough = true;
                let psbt = self.inbox.iter().position(|it| {
                    it.kind == FileKind::Psbt
                        && crate::wallet::read_psbt(&it.bytes)
                            .and_then(|p| self.session.wallet_for(&p))
                            == Some(i)
                });
                match (self.spend.as_ref().map(|s| s.wallet), psbt) {
                    (Some(Some(w)), _) if w == i => self.family.open = Some(Open::Spend),
                    (_, Some(k)) if self.spend.is_none() => self.family_use_psbt(k),
                    _ => self.family_advance(page::OPEN),
                }
                self.family.scroll.follow = true;
            }
            F::Walkthrough => {
                self.family.walkthrough = true;
                self.family.open = Some(Open::Page(page::MAP));
                self.family.scroll = Default::default();
            }
            F::SeedWallet => {
                self.family.route = Some(Route::Words);
                self.family.walkthrough = true;
                self.family.closed[page::MAP as usize] = true;
                self.family.closed[page::SAFE as usize] = true;
                self.family_words_kind(NewKind::NativeSegwit);
                self.refresh_spend();
                self.family_advance(page::OPEN);
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
    /// up. On the paper route, and on the vault route when the open
    /// vaults hold seeds but no wallet. Several wallet files wait for the
    /// person to choose.
    fn family_auto_wallet(&mut self) {
        if !self.session.wallets.is_empty() {
            return;
        }
        let wanted = match self.family.route {
            Some(Route::Paper) => true,
            Some(Route::Vault) => {
                !self.vaults.open.is_empty()
                    && (0..self.vaults.open.len())
                        .all(|v| self.vault_rows(v).iter().all(|r| r.seed))
            }
            _ => false,
        };
        if !wanted {
            return;
        }
        // Files that describe one wallet in several forms (descriptor,
        // wallet .json, BIP 129 record, Core's import) count once.
        let mut wallets: Vec<(usize, String)> = Vec::new();
        for (k, it) in self.inbox.iter().enumerate() {
            if it.kind != FileKind::Wallet {
                continue;
            }
            let text = String::from_utf8_lossy(&it.bytes);
            if let Ok(p) = crate::wallet::read_wallet(&text) {
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
        if self.family_wallet().is_some() {
            return;
        }
        let only = (self.session.wallets.len() == 1).then_some(0);
        let by_psbt = || {
            self.lead_psbt()
                .and_then(|i| crate::wallet::read_psbt(&self.inbox[i].bytes))
                .and_then(|p| self.session.wallet_for(&p))
        };
        if let Some(i) = only.or_else(by_psbt) {
            self.family.wallet = Some(self.session.wallets[i].name.clone());
            self.wallet = i;
        }
    }

    /// After a wallet arrives: chosen when it is the only one, and the
    /// next page opens once there is one.
    pub(crate) fn family_opened(&mut self) {
        self.family_choose_only();
        if let Some(i) = self.family_wallet() {
            self.wallet = i;
        }
        if self.family_ready() && self.family.open == Some(Open::Page(page::OPEN)) {
            self.family_advance(page::OPEN);
        }
    }

    /// A vault unlocked from this tab: everything in it is chosen to
    /// load, since spending needs both the wallet and its seeds, and the
    /// person loads it or less from the list.
    pub(crate) fn family_unlocked(&mut self, v: usize) {
        if let Some(o) = self.vaults.open.get_mut(v) {
            o.skip.clear();
        }
    }

    /// A key typed or scanned from this tab. On the words route it opens
    /// a native SegWit wallet; the other
    /// single-key kinds are one press away.
    pub(crate) fn family_key_added(&mut self) {
        if self.family.route == Some(Route::Words) && !self.family_ready() {
            self.family_words_kind(NewKind::NativeSegwit);
        }
        self.refresh_spend();
        self.family_opened();
        self.family_settle();
    }

    /// The wallet the last key typed opens as `kind`, in place of the one
    /// it opened before.
    fn family_words_kind(&mut self, kind: NewKind) {
        let Some(key) = self.session.keys.last() else {
            return;
        };
        let text = match kind.key_text(&key.master) {
            Ok(t) => t,
            Err(e) => {
                self.family.error = Some(e);
                return;
            }
        };
        let descriptor = kind.descriptor(1, &[text]);
        self.session.wallets.retain(|w| w.source != WORDS_SOURCE);
        let name = format!("Words · {}", kind_name(kind));
        match self.session.add_wallet(&name, &descriptor, WORDS_SOURCE) {
            Ok(i) => {
                self.family.wallet = Some(name);
                self.wallet = i;
            }
            Err(e) => self.family.error = Some(e.text()),
        }
    }

    /// The single-key kind the words route's wallet is, if it is one.
    pub fn family_words_kind_now(&self) -> Option<NewKind> {
        let w = self
            .session
            .wallets
            .iter()
            .find(|w| w.source == WORDS_SOURCE)?;
        SINGLE
            .into_iter()
            .find(|k| w.name == format!("Words · {}", kind_name(*k)))
    }

    fn family_use_wallet(&mut self, index: usize) {
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let text = String::from_utf8_lossy(&item.bytes).into_owned();
        let name = osk_bip::multisig_config::parse_named(&text)
            .ok()
            .and_then(|(_, n)| n)
            .or_else(|| {
                crate::wallet::json_string(&text, "label")
                    .filter(|l| text.trim_start().starts_with('{') && !l.trim().is_empty())
            })
            .unwrap_or_else(|| crate::file_stem(&item.name));
        let source = item.name.clone();
        match self.session.add_wallet(&name, &text, &source) {
            Ok(i) => {
                self.family.wallet = Some(self.session.wallets[i].name.clone());
                self.wallet = i;
                self.refresh_spend();
                self.family_opened();
            }
            Err(e) => self.family.error = Some(e.text()),
        }
    }

    /// The split sheets in the Inbox, put together. Says what is still
    /// missing only when asked (`loud`).
    fn family_rebuild(&mut self, loud: bool) {
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
                    let name = osk_bip::multisig_config::parse_named(&whole)
                        .ok()
                        .and_then(|(_, n)| n)
                        .unwrap_or_else(|| "Restored wallet".to_string());
                    match self.session.add_wallet(&name, &whole, "its shares") {
                        Ok(i) => {
                            self.family.wallet = Some(self.session.wallets[i].name.clone());
                            self.wallet = i;
                            self.refresh_spend();
                            self.family_opened();
                        }
                        Err(e) => self.family.error = Some(e.text()),
                    }
                }
                None if loud => {
                    self.family.error = Some(format!("{} of {} keys in hand", m.have.len(), m.n));
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
        if s.wallet.is_none() && self.family.route == Some(Route::Words) {
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
        if self.family.route.is_none() && !active {
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
                    f.open = v
                        .parse::<u8>()
                        .ok()
                        .filter(|&p| p < page::COUNT)
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
        NewKind::NativeSegwit => "native SegWit",
        NewKind::Taproot => "Taproot",
        NewKind::NestedSegwit => "nested SegWit",
        NewKind::Legacy => "legacy",
        _ => "",
    }
}

/// The action a card's header press takes.
pub fn toggle(id: CardId) -> Action {
    match id {
        CardId::Page(p) => Action::Family(FamilyAction::Card(p)),
        CardId::Step(n) => Action::Step(n),
        CardId::Later(_) => Action::Family(FamilyAction::Card(page::BRING)),
    }
}
