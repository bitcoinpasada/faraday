//! The row opened on a menu page (`docs/WORKSPACE.md` §7 point 1): a press
//! on a key's, a wallet's or a held file's row of the session pane on Tools,
//! Check, Wallets or the wide layout's Home, and a file of In Faraday on the
//! wide layout's Files, opens the item in the page's second pane instead of
//! leaving the page. This is the one state of it: which page, which item.
//! The item is kept by what it is (a key by its fingerprint, a wallet by its
//! identity, a file by its name and size), never by a place that shifts when
//! another goes. It holds no key material. Drawing is `screens::row_details`.

use crate::{Action, Faraday, Screen};
use faraday_bip::policy::WalletId;

/// The pages that open a row in their second pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    /// Home, on the wide layout where the search stands in the top bar.
    Home,
    Tools,
    Check,
    Wallets,
    /// Files, on the wide layout: a file of In Faraday opens in the stick's
    /// place.
    Files,
}

impl Page {
    /// The page a screen is, where it has the pane.
    pub(crate) fn of(screen: Screen) -> Option<Page> {
        match screen {
            Screen::Home => Some(Page::Home),
            Screen::Catalog => Some(Page::Tools),
            Screen::Check => Some(Page::Check),
            Screen::Wallets | Screen::WalletsStart => Some(Page::Wallets),
            Screen::Files => Some(Page::Files),
            _ => None,
        }
    }
}

/// Whether `page` opens a row in this window: not on the phone's layout,
/// Home and Files only on the expanded rail (the rail's windows keep them
/// as they are), and not while Home's search is typed (its results take
/// the tile's place).
fn takes_rows(app: &Faraday, page: Page) -> bool {
    !app.is_compact()
        && match page {
            Page::Home => {
                app.nav() == crate::nav::Nav::ExpandedRail && app.tools.home_find().is_empty()
            }
            Page::Files => app.nav() == crate::nav::Nav::ExpandedRail,
            _ => true,
        }
}

/// Whether Files draws the rows of In Faraday as ones that open details.
pub(crate) fn files_opens(app: &Faraday) -> bool {
    app.screen == Screen::Files && takes_rows(app, Page::Files)
}

/// Whether a file's details are open on Files, which is on show.
pub(crate) fn files_open(app: &Faraday) -> bool {
    app.screen == Screen::Files
        && matches!(area(app).open, Some((Page::Files, _)))
        && opened(app).is_some()
}

/// Whether Home's card draws its rows as ones that open details.
pub(crate) fn home_opens(app: &Faraday) -> bool {
    app.screen == Screen::Home && !app.is_compact() && app.nav() == crate::nav::Nav::ExpandedRail
}

/// A row as pressed: a wallet or a file by its place in this frame's list,
/// which the press turns into what it is at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// A key, by its fingerprint.
    Key([u8; 4]),
    /// A wallet, by its place in the list.
    Wallet(usize),
    /// A held file, by its place in the Inbox.
    File(usize),
}

/// The item as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowItem {
    Key([u8; 4]),
    Wallet(WalletId),
    File { name: String, len: usize },
}

/// The item found now, by its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opened {
    Key([u8; 4]),
    Wallet(usize),
    File(usize),
}

/// What is open and where the page was scrolled before it opened.
#[derive(Default)]
pub(crate) struct RowOpenArea {
    open: Option<(Page, RowItem)>,
    tile_offset: f32,
}

impl RowOpenArea {
    /// Lock: nothing stays open.
    pub(crate) fn lock(&mut self) {
        self.open = None;
        self.tile_offset = 0.0;
    }

    /// Opens `item` on `page`; where none was open, `scrolled` is what
    /// the page was scrolled by.
    pub(crate) fn set(&mut self, page: Page, item: RowItem, scrolled: f32) {
        if self.open.is_none() {
            self.tile_offset = scrolled;
        }
        self.open = Some((page, item));
    }

    /// Closes it: the scroll the page had, where one was open.
    pub(crate) fn take(&mut self) -> Option<f32> {
        self.open
            .take()
            .map(|_| std::mem::take(&mut self.tile_offset))
    }
}

fn area(app: &Faraday) -> &RowOpenArea {
    app.wallets.row_open()
}

/// The item as it is now, where the page on show is the one it was opened
/// on and the item is still there. A lookup.
pub(crate) fn opened(app: &Faraday) -> Option<Opened> {
    let (page, item) = area(app).open.as_ref()?;
    if !takes_rows(app, *page) || Page::of(app.screen) != Some(*page) {
        return None;
    }
    match item {
        RowItem::Key(fp) => app
            .session
            .keys
            .all()
            .any(|k| k.fingerprint.0 == *fp)
            .then_some(Opened::Key(*fp)),
        RowItem::Wallet(id) => app
            .session
            .wallets
            .iter()
            .position(|w| w.id() == *id)
            .map(Opened::Wallet),
        RowItem::File { name, len } => app
            .inbox
            .iter()
            .position(|it| it.name.as_str() == name && it.bytes.len() == *len)
            .map(Opened::File),
    }
}

/// Opens `row` on the page on show, or swaps the details in place.
pub(crate) fn act(app: &mut Faraday, row: Row) {
    let Some(page) = Page::of(app.screen) else {
        return;
    };
    let item = match row {
        Row::Key(fp) => Some(RowItem::Key(fp)),
        Row::Wallet(i) => app.session.wallets.get(i).map(|w| RowItem::Wallet(w.id())),
        Row::File(k) => app.inbox.get(k).map(|it| RowItem::File {
            name: it.name.to_string(),
            len: it.bytes.len(),
        }),
    };
    let Some(item) = item else {
        return;
    };
    // The same row again leaves it open.
    if area(app).open.as_ref() == Some(&(page, item.clone())) {
        return;
    }
    let scrolled = app.list_offset;
    app.list_offset = 0.0;
    app.wallets.set_card_offset(0.0);
    app.wallets.row_open_set(page, item, scrolled);
    sync(app, false);
}

/// Back, Escape and a press on the page's tab: the page again, at the
/// scroll it had.
pub(crate) fn close(app: &mut Faraday) {
    if let Some(at) = app.wallets.row_open_take() {
        app.list_offset = at;
    }
}

/// After every press: the page returns where the item went away, a
/// wallet's page follows the wallet opened, and the page left clears it,
/// unless it was left into a flow begun from the details (a press that is
/// not a tab's): leaving that flow returns to the details. `tab`: the
/// press was a tab's.
pub(crate) fn sync(app: &mut Faraday, tab: bool) {
    let Some((page, _)) = area(app).open.as_ref() else {
        return;
    };
    let page = *page;
    if app.is_compact()
        || (matches!(page, Page::Home | Page::Files) && app.nav() != crate::nav::Nav::ExpandedRail)
    {
        app.wallets.row_open_lock();
        return;
    }
    if Page::of(app.screen) != Some(page) {
        if tab || Page::of(app.screen).is_some() {
            app.wallets.row_open_lock();
        }
        return;
    }
    match opened(app) {
        None => close(app),
        Some(Opened::Wallet(i)) => app.wallets.set_wallet(i),
        Some(_) => {}
    }
}

/// The press of the page's own tab while a row is open.
pub(crate) fn after(app: &mut Faraday, action: Action) {
    let tab = matches!(action, Action::Nav(_));
    if let (Action::Nav(s), Some((page, _))) = (action, area(app).open.as_ref())
        && Page::of(s) == Some(*page)
    {
        close(app);
        return;
    }
    sync(app, tab);
}
