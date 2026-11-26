//! Check an address (`docs/DESIGN.md` §5, Check an address): a flow's
//! one card, or a page on a compact panel, with
//! a field for an address typed, pasted or scanned, searched in the
//! loaded wallets and keys' standard accounts as a scan's address is, its
//! result a card under the ways in. A `bitcoin:` link is read to its
//! address. Text that is a secret is refused and its field emptied, so
//! nothing of it is kept or drawn; the field and the refusal are wiping
//! text, and leaving the page drops them all.
//!
//! The search is [`Faraday::address_check`]'s (the first 100 receive and
//! change addresses of each loaded wallet and key account); a miss goes on
//! as **Look further** asks, a tick at a time ([`tick`]), as Explore's Find
//! an address does. Drawing reads the [`Answer`] worked out when it
//! arrived and works nothing out.

pub(crate) mod file;
pub(crate) mod flow;
pub(crate) mod pane;
pub(crate) mod screen;

use crate::address_find::{self, Answer, FoundIn, Look};
use crate::scan_read::{self, Kind, Way};
use crate::strings::EN;
use crate::{Action, Faraday, Screen};
use faraday_bip::bitcoin::Address;
use faraday_crypto::work::Compute;
use faraday_shell_api::Key;
use faraday_text::Text;

/// The most characters the field takes: more than any address or
/// payment link is read at.
const FIELD_MAX: usize = 400;

/// A press on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressPress {
    /// Scan an address.
    Scan,
    /// The clipboard's text into the field, checked at once.
    Paste,
    /// **From a file**: the menu of text files in Files.
    Files,
    /// The text file at this place of the Inbox, read for its addresses.
    File(u16),
    /// The address at this place of a file's list, its answer shown.
    Row(u8),
    /// Back from a row's answer to the list.
    Back,
    /// The way over at this place of the result's ways: **Look
    /// further**, **Stop**, **Make a wallet** or **Open the wallet**.
    Way(u8),
    /// Another: the page again, empty, to check the next address.
    Another,
}

/// What the page holds.
pub struct AddressCheckState {
    /// The text in the field.
    typed: Text,
    /// The screen its back link returns to.
    back: Screen,
    /// Why the text was not checked, under the field in the error colour.
    refusal: Option<Text>,
    /// The check under way or done: where a look further goes on.
    look: Option<Look>,
    /// What the result card says.
    answer: Option<Answer>,
    /// The ways over the result offers.
    ways: Vec<Way>,
    /// Whether the address was not searched for want of a wallet or key.
    load_first: bool,
    /// The addresses of a file that held several, listed.
    list: Option<file::Listed>,
    /// Where the button that opened the menu of files is, for the menu
    /// to stand under.
    files_at: Option<(i32, i32, i32, i32)>,
    /// The Inbox place of the file whose address or list is on show.
    from: Option<usize>,
    /// How far the flow's card is scrolled, where the page is drawn as one.
    scroll: crate::flow::Scroll,
}

impl AddressCheckState {
    fn new(back: Screen, load_first: bool) -> Self {
        AddressCheckState {
            typed: Text::new(),
            back,
            refusal: None,
            look: None,
            answer: None,
            ways: Vec::new(),
            load_first,
            list: None,
            files_at: None,
            from: None,
            scroll: crate::flow::Scroll::default(),
        }
    }

    /// The text in the field.
    pub fn typed(&self) -> &Text {
        &self.typed
    }

    /// The screen its back link returns to.
    pub fn back(&self) -> Screen {
        self.back
    }

    /// The Inbox place of the file whose address or list is on show.
    pub(crate) fn from(&self) -> Option<usize> {
        self.from
    }

    /// How far the flow's card is scrolled.
    pub(crate) fn scroll(&self) -> crate::flow::Scroll {
        self.scroll
    }

    /// Why the text was not checked.
    pub fn refusal(&self) -> Option<&Text> {
        self.refusal.as_ref()
    }

    /// Whether the page says to load a key or a wallet first.
    pub fn load_first(&self) -> bool {
        self.load_first
    }

    /// Whether the page has nothing in it yet: no text, no refusal, no
    /// result and no list.
    pub(crate) fn is_empty(&self) -> bool {
        self.typed.is_empty()
            && self.refusal.is_none()
            && self.answer.is_none()
            && self.list.is_none()
    }

    /// The ways over the result offers.
    pub fn ways(&self) -> &[Way] {
        &self.ways
    }

    /// What the result card says, a line each: the Result line, then
    /// each fact as its label, a colon and its value; empty before a
    /// check.
    pub fn result_lines(&self) -> Vec<Text> {
        let Some(a) = self.answer.as_ref() else {
            return Vec::new();
        };
        let mut out = vec![a.head.clone()];
        for row in &a.rows {
            for (i, line) in row.lines.iter().enumerate() {
                let mut one = Text::of(if i == 0 { row.label } else { "" });
                one.push_str(": ");
                one.push_str(line);
                out.push(one);
            }
        }
        out
    }

    /// The result card's answer.
    pub(crate) fn answer(&self) -> Option<&Answer> {
        self.answer.as_ref()
    }

    /// Wallet `gone` left the session list: the wallet made from the find
    /// moves up with it, and is none when it is the one gone.
    pub(crate) fn wallet_left(&mut self, gone: usize) {
        if let Some(look) = self.look.as_mut()
            && look.made.is_some()
        {
            look.made = look
                .made
                .and_then(|m| crate::wallet::place_after_removal(m, gone));
            self.refresh();
        }
    }

    /// What was checked and found is forgotten, as the text changes.
    /// The place of the wallet made from the find, once it was.
    pub(crate) fn made(&self) -> Option<usize> {
        self.look.as_ref().and_then(|l| l.made)
    }

    fn forget(&mut self) {
        self.refusal = None;
        self.look = None;
        self.answer = None;
        self.ways.clear();
        self.load_first = false;
        self.list = None;
        self.from = None;
    }

    /// The answer and the ways over, again from the look.
    fn refresh(&mut self) {
        if let Some(look) = self.look.as_ref() {
            let (answer, ways) = address_find::answer(look);
            self.answer = Some(answer);
            self.ways = ways;
        }
    }
}

/// The page's area: its state while it is open.
#[derive(Default)]
pub(crate) struct AddressCheckArea {
    state: Option<AddressCheckState>,
    /// The place of a wallet opened from the made wallet's dialog: the
    /// page stays while that wallet's card is on show, so its Back
    /// returns here.
    held_for: Option<usize>,
}

/// Holds nothing a lock wipes: an address typed, public.
impl crate::at_lock::Public for AddressCheckArea {}

impl AddressCheckArea {
    /// Keeps the page while wallet `i`'s card is on show.
    pub(crate) fn hold_for(&mut self, i: usize) {
        self.held_for = Some(i);
    }

    /// Lets the page go with the card.
    pub(crate) fn release(&mut self) {
        self.held_for = None;
    }

    /// The wallet whose card keeps the page, if any.
    pub(crate) fn held_for(&self) -> Option<usize> {
        self.held_for
    }

    /// The page's state, while it is open.
    pub(crate) fn state(&self) -> Option<&AddressCheckState> {
        self.state.as_ref()
    }

    /// How far the flow's card is scrolled, units.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|s| s.scroll.y)
    }

    /// The flow's card scrolled to `y` units.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(s) = self.state.as_mut() {
            s.scroll.y = y;
        }
    }

    /// The flow's card scrolled to `to`.
    pub(crate) fn set_scroll(&mut self, to: crate::flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// Wallet `gone` left the session list.
    pub(crate) fn wallet_left(&mut self, gone: usize) {
        self.held_for = self
            .held_for
            .and_then(|h| crate::wallet::place_after_removal(h, gone));
        if let Some(s) = self.state.as_mut() {
            s.wallet_left(gone);
        }
    }

    /// Forgets the text and what was found.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

/// Whether nothing is loaded to search: no wallet and no key.
fn nothing_to_search(app: &Faraday) -> bool {
    app.session.wallets.is_empty() && app.session.keys.is_empty()
}

/// Costs what a paste check costs. The clipboard's text asked for by
/// Paste or Ctrl+V on the page, going into the field: returns whether it
/// was for the page.
pub(crate) fn pasted(app: &mut Faraday, text: &str) -> bool {
    if app.screen != Screen::CheckAddress
        || app.sheet.is_some()
        || app.address_check_state().is_none()
    {
        return false;
    }
    let c = app.compute();
    check_text(app, &c, text);
    true
}

/// The page opened on its own, empty, to return to where it was opened
/// from.
pub(crate) fn open(app: &mut Faraday) {
    let back = back_for(app);
    let load_first = nothing_to_search(app);
    app.tools.address_check.state = Some(AddressCheckState::new(back, load_first));
    app.screen = Screen::CheckAddress;
}

/// Costs what an address check costs. The page shown again after a wallet
/// was loaded for it: the address that was in the field is checked
/// against the wallet.
pub(crate) fn reopen(app: &mut Faraday) {
    let typed = app
        .address_check_state()
        .map(|s| s.typed.as_str().to_string())
        .unwrap_or_default();
    open(app);
    if !typed.is_empty() {
        let c = app.compute();
        check_text(app, &c, &typed);
    }
}

/// Costs what reading a file costs. Tools' Check an address: the text
/// file itself when exactly one waits in Files, and the page empty
/// otherwise, which offers the files to choose from when several wait
/// and the empty part (Scan, Open Files, the stick) when none does.
pub(crate) fn open_tile(app: &mut Faraday) {
    let waiting = file::waiting(app);
    open(app);
    if let [(at, _)] = waiting[..] {
        let c = app.compute();
        file::read(app, &c, at);
    }
}

/// Where the page returns to: where it is opened from, and the same
/// when it is already open.
fn back_for(app: &Faraday) -> Screen {
    match (app.screen, app.address_check_state()) {
        (Screen::CheckAddress, Some(s)) => s.back,
        (Screen::CheckAddress, None) => Screen::Catalog,
        (screen, _) => screen,
    }
}

/// Costs what an address check costs. An address a scanner read, or
/// why it was refused, on the page: the page opens on it and the address
/// is checked at once.
pub(crate) fn show(app: &mut Faraday, c: &Compute, read: Result<Address, Text>) {
    let back = back_for(app);
    app.tools.address_check.state = Some(AddressCheckState::new(back, nothing_to_search(app)));
    app.screen = Screen::CheckAddress;
    match read {
        Err(why) => {
            if let Some(s) = app.tools.address_check.state.as_mut() {
                s.refusal = Some(why);
            }
        }
        Ok(addr) => {
            let mut shown = Text::new();
            let _ = std::fmt::Write::write_fmt(&mut shown, format_args!("{addr}"));
            if let Some(s) = app.tools.address_check.state.as_mut() {
                s.typed = shown;
            }
            check_address(app, c, &addr);
        }
    }
}

/// Costs what an address check costs. `addr` searched in the loaded
/// wallets and keys' accounts, the result kept.
fn check_address(app: &mut Faraday, c: &Compute, addr: &Address) {
    let look = app.address_check(c, addr);
    let Some(s) = app.tools.address_check.state.as_mut() else {
        return;
    };
    s.forget();
    match look {
        Some(look) => {
            s.look = Some(look);
            s.refresh();
        }
        None => s.load_first = true,
    }
}

/// Costs what the pasted-text classifier and an address check cost. The
/// text in the field or pasted, checked: a secret is refused and the
/// field emptied; an address or a `bitcoin:` link is searched; anything
/// else is named.
fn check_text(app: &mut Faraday, c: &Compute, text: &str) {
    let t = text.trim();
    if t.is_empty() {
        if let Some(s) = app.tools.address_check.state.as_mut() {
            s.forget();
            s.refusal = Some(Text::of(EN.pasted_nothing));
        }
        return;
    }
    if let Some(why) = crate::scan_key::pasted_secret(c, t, false) {
        if let Some(s) = app.tools.address_check.state.as_mut() {
            s.forget();
            s.typed.clear();
            s.refusal = Some(Text::of(why));
        }
        return;
    }
    // What was pasted is what the field shows, whatever the check says.
    if let Some(s) = app.tools.address_check.state.as_mut()
        && s.typed.as_str() != t
    {
        s.typed.clear();
        for ch in t.chars().take(FIELD_MAX) {
            s.typed.push(ch);
        }
    }
    match app.address_read(t) {
        Some(Ok(addr)) => {
            let mut shown = Text::new();
            let _ = std::fmt::Write::write_fmt(&mut shown, format_args!("{addr}"));
            if let Some(s) = app.tools.address_check.state.as_mut() {
                s.typed = shown;
            }
            check_address(app, c, &addr);
        }
        Some(Err(why)) => refuse(app, why),
        None => {
            let kind = scan_read::public_kind(
                crate::wallet::classify(c, "", t.as_bytes()),
                false,
                t.as_bytes(),
            );
            let why = match kind {
                Kind::Text | Kind::Unreadable | Kind::Address | Kind::PaymentRequest => {
                    Text::of(EN.address_check_not_bitcoin)
                }
                Kind::Invoice
                | Kind::Web
                | Kind::Kef
                | Kind::SilentAddress
                | Kind::Labels
                | Kind::File => Text::of(EN.address_check_not_an_address),
                other => other.not_here(),
            };
            refuse(app, why);
        }
    }
}

/// `why` under the field, nothing found.
fn refuse(app: &mut Faraday, why: Text) {
    if let Some(s) = app.tools.address_check.state.as_mut() {
        s.forget();
        s.refusal = Some(why);
    }
}

/// Costs what a check costs. One press on the page.
pub(crate) fn act(app: &mut Faraday, press: AddressPress) {
    match press {
        AddressPress::Scan => app.act(Action::ScanFor(crate::Ask::Address)),
        AddressPress::Paste => request_paste(app),
        AddressPress::Files => file::open_menu(app),
        AddressPress::File(i) => {
            let c = app.compute();
            file::read(app, &c, usize::from(i));
        }
        AddressPress::Row(i) => file::show_row(app, usize::from(i)),
        AddressPress::Back => {
            file::back_to_list(app);
        }
        AddressPress::Way(i) => way(app, usize::from(i)),
        AddressPress::Another => open(app),
    }
}

/// The shell is asked for the clipboard's text, where it has one.
pub(crate) fn request_paste(app: &mut Faraday) {
    if app.clipboard {
        app.out
            .command(faraday_shell_api::Command::RequestClipboard {
                kind: faraday_shell_api::FileKind::Text,
            });
    }
}

/// Costs what making a wallet costs. A press on the result's way over
/// at place `i`.
fn way(app: &mut Faraday, i: usize) {
    let Some(way) = app
        .address_check_state()
        .and_then(|s| s.ways.get(i).cloned())
    else {
        return;
    };
    let now = app.now_ms;
    match way {
        Way::Look(look_way) => {
            let wallets = &app.session.wallets;
            if let Some(s) = app.tools.address_check.state.as_mut() {
                if let Some(look) = s.look.as_mut() {
                    look.sync(wallets);
                    look.press(look_way, now);
                }
                s.refresh();
            }
        }
        Way::MakeWallet => make(app),
        Way::OpenWallet(place) => app.act(Action::OpenWallet(place)),
        Way::SignAddress => sign(app),
        _ => {}
    }
}

/// A lookup. Whether the key that would sign the address the result found
/// waits for its backup, from the keys kept as waiting.
pub(crate) fn sign_waits(app: &Faraday) -> bool {
    let look = app.address_check_state().and_then(|s| s.look.as_ref());
    let found = look.and_then(|l| l.found_in.as_ref().map(|f| (l, f)));
    match found.and_then(|(l, f)| Some((l, f.sign()?))) {
        Some((l, crate::address_find::SignAt::Wallet(w, _))) => {
            l.same_wallet(&app.session, *w) && app.wallet_waits_kept(*w)
        }
        Some((_, crate::address_find::SignAt::Key(at))) => app.message_waits(Some(at), None),
        None => false,
    }
}

/// Opens Sign a message on the address the result found, where a loaded
/// key signs it.
fn sign(app: &mut Faraday) {
    let Some(look) = app.address_check_state().and_then(|s| s.look.as_ref()) else {
        return;
    };
    let sign = look.found_in.as_ref().and_then(|f| f.sign().cloned());
    match sign {
        Some(crate::address_find::SignAt::Wallet(wallet, index))
            if look.same_wallet(&app.session, wallet) =>
        {
            let c = app.compute();
            app.message_open_at(&c, wallet, index);
        }
        Some(crate::address_find::SignAt::Key(at)) => app.message_open_path(at),
        _ => {}
    }
}

/// Costs a key derivation for each step of an account's path, a
/// descriptor parse and two address derivations. The wallet of the key's
/// account the address was found in, which the result then offers to
/// open.
fn make(app: &mut Faraday) {
    let Some(at) = app
        .address_check_state()
        .and_then(|s| s.look.as_ref())
        .and_then(|l| match l.found_in.as_ref()? {
            FoundIn::Account { at, script, .. } => Some((*at.0.as_bytes(), at.1.clone(), *script)),
            FoundIn::Wallet { .. } => None,
        })
    else {
        return;
    };
    let c = app.compute();
    match app.make_wallet_at(&c, at.0, &at.1, at.2) {
        Ok(place) => {
            if let Some(s) = app.tools.address_check.state.as_mut() {
                if let Some(look) = s.look.as_mut() {
                    look.made = Some(place);
                }
                s.refresh();
            }
            app.sheet = Some(crate::Sheet::MadeWallet);
        }
        Err(why) => app.toast(&why),
    }
}

/// Costs an address derivation for each index of the tick, on both
/// chains of every loaded wallet and key account. One tick of the look
/// further under way. Returns whether to draw.
pub(crate) fn tick(app: &mut Faraday, now_ms: u64) -> bool {
    let running = app
        .address_check_state()
        .and_then(|s| s.look.as_ref())
        .is_some_and(Look::running);
    if !running || app.screen != Screen::CheckAddress {
        return false;
    }
    let c = app.compute();
    let session = &app.session;
    let Some(s) = app.tools.address_check.state.as_mut() else {
        return false;
    };
    let Some(look) = s.look.as_mut() else {
        return false;
    };
    look.sync(&session.wallets);
    if look.running() {
        let policies: Vec<_> = session.wallets.iter().map(|w| w.policy()).collect();
        look.tick(&c, &policies, now_ms);
        look.settle(session);
    }
    s.refresh();
    true
}

/// Typing into the field: a character other than white space, Backspace,
/// and Enter, which checks. Returns whether the key was taken.
pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if app.screen != Screen::CheckAddress || app.sheet.is_some() {
        return false;
    }
    let Some(s) = app.tools.address_check.state.as_mut() else {
        return false;
    };
    match key {
        Key::Escape if s.row_shown() => {
            file::back_to_list(app);
        }
        Key::Char(c) if !c.is_control() && !c.is_whitespace() => {
            if s.typed.chars().count() < FIELD_MAX {
                s.typed.push(c);
            }
            s.forget();
        }
        Key::Backspace => {
            s.typed.pop();
            s.forget();
        }
        Key::Enter => {
            if !s.typed.is_empty() {
                let typed = s.typed.clone();
                let c = app.compute();
                check_text(app, &c, &typed);
            }
        }
        _ => return false,
    }
    true
}
