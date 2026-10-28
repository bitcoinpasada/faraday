//! The rail, the bar, the top app bar, the status chips and the ⋮ menu
//! as Faraday draws them (`docs/LAYOUT.md` §3, DESIGN §4.1), from
//! `faraday_ui::nav`. Each reads the state; none works anything out.

use super::{MenuPress, Nav, destination_of};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen, Sheet};
use faraday_draw::tokens::{
    APP_BAR_H, CAPTION, GAP, GAP_SMALL, GRID, ICON_SMALL, MARGIN_COMPACT, MODE_PILL_HEIGHT,
    STATUS_CHIP_H, TOUCH,
};
use faraday_draw::widgets::Icon;
use faraday_ui::nav::{AppBar, Badge, Chip, Dest, Rail};

/// The most chips a group of the status strip shows before "+ N more".
const CHIPS_PER_GROUP: usize = 4;

/// A destination for screen `s` (or `action`), lit while it is the one
/// shown.
fn dest<'s>(
    app: &View,
    icon: impl Into<Glyph>,
    label: &'s str,
    action: Action,
) -> Dest<'s, Action> {
    let current = match action {
        Action::Nav(s) => destination_of(app.screen) == s && app.sheet != Some(Sheet::Learn),
        Action::Learn => app.sheet == Some(Sheet::Learn),
        _ => false,
    };
    Dest {
        icon: icon.into(),
        label,
        action,
        current,
        badge: match action {
            Action::Nav(s) => badge_of(app, s),
            _ => Badge::None,
        },
    }
}

/// What waits at a destination, as its badge (`docs/DESIGN.md` §4.1):
/// on Files, how many files the Inbox holds; on the stick visit, a dot
/// while a stick attached has not been visited. Each reads the state.
pub(crate) fn badge_of(app: &View, s: Screen) -> Badge {
    match s {
        Screen::Files => Badge::Count(app.inbox.len()),
        Screen::Visit if app.stick.waits() => Badge::Dot,
        _ => Badge::None,
    }
}

/// The five destinations, in order, then Stick visit and Transfer
/// while they apply.
fn destinations<'s>(app: &'s View) -> (Vec<Dest<'s, Action>>, Vec<Dest<'s, Action>>) {
    let five = vec![
        dest(app, Icon::House, EN.sidebar_home, Action::Nav(Screen::Home)),
        dest(
            app,
            Icon::Wallet,
            EN.sidebar_wallets,
            Action::Nav(Screen::WalletsStart),
        ),
        dest(
            app,
            Icon::File,
            EN.sidebar_files,
            Action::Nav(Screen::Files),
        ),
        dest(
            app,
            Icon::Lock,
            EN.sidebar_vaults,
            Action::Nav(Screen::Vaults),
        ),
        dest(
            app,
            Icon::Tools,
            EN.sidebar_tools,
            Action::Nav(Screen::Catalog),
        ),
    ];
    let mut more = Vec::new();
    if !app.stick.is_empty() && !app.holds_secret() {
        more.push(dest(
            app,
            app.medium.icon(),
            app.medium.visit(),
            Action::Nav(Screen::Visit),
        ));
    }
    if app.online {
        more.push(dest(
            app,
            Icon::Qr,
            EN.sidebar_transfer,
            Action::Nav(Screen::Transfer),
        ));
    }
    (five, more)
}

/// The navigation rail down the window's left, `h` tall: the
/// destinations, Learn and Settings under a divider, Lock (while
/// something is held) and Power off at its foot.
pub(crate) fn rail(app: &View, ui: &mut Ui, h: f32) {
    let (mut top, more) = destinations(app);
    top.extend(more);
    let under = [
        dest(app, Icon::Learn, EN.sidebar_learn, Action::Learn),
        dest(
            app,
            Icon::Settings,
            EN.sidebar_settings,
            Action::Nav(Screen::Settings),
        ),
    ];
    let mut foot = Vec::new();
    if app.holds_secret() {
        foot.push(dest(app, Icon::Lock, EN.sidebar_lock, Action::LockAsk));
    }
    foot.push(dest(app, Icon::Power, EN.nav_power_off, Action::PowerAsk));
    faraday_ui::nav::rail(
        ui,
        h,
        &Rail {
            top: &top,
            under: &under,
            foot: &foot,
        },
    );
}

/// The navigation bar across the window's foot at `y`, `w` wide: the
/// five destinations, Stick visit or Transfer in place of Tools while it
/// applies (Tools stays on Home).
pub(crate) fn bar(app: &View, ui: &mut Ui, y: f32, w: f32) {
    let (mut five, more) = destinations(app);
    if let Some(d) = more.into_iter().next() {
        five[4] = d;
    }
    faraday_ui::nav::bar(ui, y, w, &five);
}

/// The top app bar across the page, `w` wide: the way back and the
/// page's name (on Home, the wordmark), and at its right end the ⋮ menu
/// and Lock on the bar's windows, the ?, the mode pill and the keyboard;
/// on the wide layout (`Nav::ExpandedRail`), drawn after the page that
/// scrolls under it, the name and way back the page handed over, and
/// Files' stick chip left of the actions. Returns its height.
pub(crate) fn app_bar(app: &View, ui: &mut Ui, w: f32, nav: Nav) -> f32 {
    let mut right = w - GAP_SMALL;
    // On a step that must not be left, nothing here leads away.
    let hidden = app.nav_hidden();
    // Drawn after the bar itself, which fills under them: measured first.
    let actions = |ui: &mut Ui, right: &mut f32| {
        if nav == Nav::Bar && !hidden {
            *right = faraday_ui::nav::more(
                ui,
                *right,
                Action::NavMenu(MenuPress::Open(None)),
                EN.tip_more,
            );
            if app.holds_secret() {
                *right = faraday_ui::nav::icon_action(
                    ui,
                    *right,
                    Icon::Lock,
                    Action::LockAsk,
                    false,
                    EN.tip_lock,
                );
            }
        }
        if !app.learn_pages().is_empty() {
            *right = faraday_ui::nav::help(ui, *right, Action::Learn, EN.tip_learn);
        }
        if crate::modes::screen::shows_pill(app) {
            let pw = crate::modes::screen::pill_width(app, ui);
            let px = *right - GAP - pw;
            crate::modes::screen::pill(app, ui, px, (APP_BAR_H - MODE_PILL_HEIGHT) / 2.0);
            *right = px - GAP;
        }
        if let Some((action, shown)) = crate::compact::keyboard_action(app) {
            *right = faraday_ui::nav::icon_action(
                ui,
                *right,
                Icon::Keyboard,
                action,
                shown,
                EN.tip_keyboard,
            );
        }
        if nav == Nav::ExpandedRail {
            // A step flow's: the codex32 sheets' eye and Cancel, as the
            // one-column layouts' bar has them (`compact::flow_bar_tools`).
            *right = crate::compact::bar_eye(app, ui, *right);
            *right = crate::compact::bar_cancel(app, ui, *right);
            if app.screen == Screen::Files {
                *right = crate::screens::files_stick_chip(app, ui, *right);
            }
            *right = crate::stick::pills::bar(app, ui, *right);
        }
    };
    let ends = crate::compact::screen::measured(ui, |ui| {
        let mut r = right;
        actions(ui, &mut r);
        r
    });
    if nav == Nav::ExpandedRail {
        // The wide page handed over its name and its way back as it was
        // drawn, under this bar (`crate::screens::bar_head`); one that
        // handed over none is named as its compact bar names it.
        let head = ui.head.take();
        let (title, back) = match &head {
            Some((name, back)) => (name.as_str(), back.as_ref().map(|(l, a)| (l.as_str(), *a))),
            None => (crate::compact::screen_name(app.screen, app.medium), None),
        };
        let bar = AppBar {
            back,
            title,
            back_tip: EN.tip_back,
        };
        faraday_ui::nav::app_bar(ui, w, &bar, ends);
    } else if app.screen == Screen::Home {
        faraday_ui::nav::app_bar(
            ui,
            w,
            &AppBar {
                back: None,
                title: "",
                back_tip: EN.tip_back,
            },
            ends,
        );
        // The mark and the name, as the sidebar's head draws them.
        let (mw, mh) = (ICON_SMALL, ICON_SMALL + GAP_SMALL);
        ui.mark(MARGIN_COMPACT, (APP_BAR_H - mh) / 2.0, mw, mh);
        let suffix = crate::screens::name_suffix(app);
        let nx = MARGIN_COMPACT + mw + GRID;
        let ny = (APP_BAR_H - CAPTION) / 2.0;
        // The name where it ends before the bar's actions, as a title is
        // fitted; Faraday Demo's word under FARADAY where the two do not
        // fit side by side; the mark alone where they leave no room for it.
        ui.wordmark_fitted((nx, ny, ends - GAP - nx), CAPTION, Role::OnSurface, suffix);
    } else {
        let (back, name) = crate::compact::page_head(app);
        let back = app.named_back(back);
        faraday_ui::nav::app_bar(
            ui,
            w,
            &AppBar {
                back: Some(back),
                title: name,
                back_tip: EN.tip_back,
            },
            ends,
        );
    }
    actions(ui, &mut right);
    APP_BAR_H
}

/// A flow in progress's chip: its name and its open step; it opens the
/// flow as it was left. Returns its width.
fn flow_chip(ui: &mut Ui, x: f32, y: f32, max: f32, r: &crate::in_progress::Row) -> f32 {
    let icon = match r.tab {
        Screen::Vaults => Icon::Lock,
        Screen::Catalog => Icon::Tools,
        _ => Icon::Wallet,
    };
    let c = Chip {
        icon: Some(icon.into()),
        head: (&r.name, W::S),
        tail: &r.step,
        action: Some(Action::Resume(r.going)),
    };
    faraday_ui::nav::status_chip(ui, (x, y, max), &c)
}

/// Chips laid in rows from (x, y) in `w`, a touch target apart.
struct Flowing {
    x0: f32,
    w: f32,
    x: f32,
    y: f32,
    any: bool,
}

impl Flowing {
    fn new(x: f32, y: f32, w: f32) -> Self {
        Flowing {
            x0: x,
            w,
            x,
            y,
            any: false,
        }
    }

    /// Places one chip, measured first, on this row or the next.
    fn put(&mut self, ui: &mut Ui, draw: &mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32) {
        let cw = crate::compact::screen::measured(ui, |ui| draw(ui, 0.0, 0.0, self.w));
        if self.x > self.x0 && self.x + cw > self.x0 + self.w {
            self.x = self.x0;
            self.y += TOUCH;
        }
        let used = draw(ui, self.x, self.y, self.x0 + self.w - self.x);
        self.x += used + GRID;
        self.any = true;
    }

    /// A status chip: its icon, its head, its tail and its press.
    fn chip(
        &mut self,
        ui: &mut Ui,
        icon: Option<Glyph>,
        head: (&str, W),
        tail: &str,
        action: Option<Action>,
    ) {
        let c = Chip {
            icon,
            head,
            tail,
            action,
        };
        self.put(ui, &mut |ui, x, y, max| {
            faraday_ui::nav::status_chip(ui, (x, y, max), &c)
        });
    }

    /// At row `i` of a group of `n`: past the most a group shows, the
    /// "+ N more" chip that opens the whole list, and true.
    fn more(&mut self, ui: &mut Ui, i: usize, n: usize, all: Action) -> bool {
        if i < CHIPS_PER_GROUP || n <= CHIPS_PER_GROUP + 1 {
            return false;
        }
        let more = fill(EN.sidebar_more, &[&(n - i).to_string()]);
        self.chip(ui, None, (&more, W::S), "", Some(all));
        true
    }

    /// The height the rows take, from where they began at `top`.
    fn height(&self, top: f32) -> f32 {
        if self.any {
            self.y - top + STATUS_CHIP_H + GAP
        } else {
            0.0
        }
    }
}

/// Over a page on the rail's and the bar's windows: a chip for each flow
/// in progress, so it is one press away (`docs/LAYOUT.md` §3). Drawn at
/// (x, y) in `w`; returns the height it takes.
pub(crate) fn flow_chips(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let flows = app.flows_in_progress();
    let on = app.flow_on_screen();
    let top = y + GAP;
    let mut f = Flowing::new(x, top, w);
    for r in flows.iter().filter(|r| Some(r.going) != on) {
        f.put(ui, &mut |ui, x, y, max| flow_chip(ui, x, y, max, r));
    }
    f.height(y)
}

/// Home's status strip where there is no expanded rail: the sidebar's
/// lists as chips (the flows in progress, the keys, the wallets, the
/// files and the sticks), each opening what its sidebar row opens.
/// Drawn at (x, y) in `w`; returns the height it takes.
pub(crate) fn status_strip(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut f = Flowing::new(x, y, w);
    for r in app.flows_in_progress().iter() {
        f.put(ui, &mut |ui, x, y, max| flow_chip(ui, x, y, max, r));
    }
    let keys: Vec<_> = app.session.keys.all().collect();
    for (i, k) in keys.iter().enumerate() {
        if f.more(ui, i, keys.len(), Action::Explore) {
            break;
        }
        let fp = fp_text(k.fingerprint);
        // A key named by its fingerprint is not named twice.
        let label = if k.label.eq_ignore_ascii_case(&fp) {
            ""
        } else {
            k.label.as_str()
        };
        let open = Action::ExploreKey(k.fingerprint.0);
        f.chip(ui, Some(Icon::Keys.into()), (&fp, W::M), label, Some(open));
    }
    let wallets = &app.session.wallets;
    for (i, wl) in wallets.iter().enumerate() {
        if f.more(ui, i, wallets.len(), Action::Nav(Screen::Wallets)) {
            break;
        }
        let open = Some(Action::OpenWallet(i));
        f.chip(ui, Some(Icon::Wallet.into()), (&wl.name, W::S), "", open);
    }
    let files = app.files_count();
    let open = Some(Action::Nav(Screen::Files));
    f.chip(
        ui,
        Some(Icon::File.into()),
        (EN.sidebar_files, W::S),
        &files,
        open,
    );
    // The sticks attached; with none, an import the boot stick brought
    // waits behind this chip.
    let waiting = app.import.state().is_some() && app.stick.is_empty();
    let sticks = match app.stick.len() {
        0 if waiting => Some(EN.sidebar_import_waiting.to_string()),
        0 => None,
        1 => Some(app.stick[0].label.clone()),
        k => Some(fill(EN.sidebar_attached, &[&k.to_string()])),
    };
    if let Some(s) = sticks {
        let open = if waiting {
            Some(crate::boot_import::OPEN)
        } else {
            (!app.holds_secret()).then_some(Action::Nav(Screen::Visit))
        };
        f.chip(
            ui,
            Some(app.medium.icon()),
            (app.medium.caps(), W::S),
            &s,
            open,
        );
    }
    f.height(y)
}

/// The ⋮ menu of the top app bar on the bar's windows: Settings and
/// Power off, as a sheet.
pub(crate) fn menu_sheet(ui: &mut Ui, w: f32, h: f32) {
    let lines = [
        crate::menu::Line::Item(
            EN.sidebar_settings.to_string(),
            Action::NavMenu(MenuPress::Go(Screen::Settings)),
        ),
        crate::menu::Line::Item(EN.nav_power_off.to_string(), Action::PowerAsk),
    ];
    crate::menu::draw(
        ui,
        w,
        h,
        &crate::menu::View {
            title: EN.nav_more,
            icon: Icon::Ellipsis,
            lines: &lines,
            at: 0,
            keys: false,
            anchor: None,
            cancel: (EN.nav_close, Action::Cancel),
            shown: 1.0,
        },
    );
}
