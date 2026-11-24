//! The rail, the bar, the top app bar, the status chips and the ⋮ menu
//! as Faraday draws them (`docs/LAYOUT.md` §3, DESIGN §4.1), from
//! `faraday_ui::nav`. Each reads the state; none works anything out.

use super::{BarTool, MenuPress, Nav, Way, lit_destination};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Shape, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Screen, Sheet};
use faraday_draw::tokens::{
    APP_BAR_H, BODY_S, GAP, GAP_SMALL, GRID, ICON_SMALL, LABEL_L, LOADED_LINE, MODE_PILL_HEIGHT,
    NAV_ICON, PAD, SHAPE_M, SHAPE_XS, STATUS_CHIP_H, TOUCH,
};
use faraday_draw::widgets::Icon;
use faraday_ui::nav::{AppBar, Badge, Chip, Dest, Rail};

/// The most keys or wallets Home's box lists before "+ N more".
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
        Action::Nav(s) => lit_destination(app) == s && app.sheet != Some(Sheet::Guide),
        Action::Guide => app.sheet == Some(Sheet::Guide),
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
/// on Files, how many files the Inbox holds, or a dot while a stick
/// attached has not been visited (F235); on Wallets, how many wallets need
/// something now. Each reads the state.
pub(crate) fn badge_of(app: &View, s: Screen) -> Badge {
    match s {
        Screen::Files if !app.inbox.is_empty() => Badge::Count(app.inbox.len()),
        Screen::Files if app.stick.waits() => Badge::Dot,
        Screen::Files => Badge::Count(0),
        // Wallets: how many need something now (`docs/HOME.md` §3).
        Screen::Vaults => Badge::Count(app.vaults.open.len()),
        Screen::WalletsStart => Badge::Count(crate::screens::wallets_needing(app).len()),
        _ => Badge::None,
    }
}

/// The five destinations, in order, then Transfer while it applies.
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
    #[cfg_attr(not(feature = "demo"), allow(unused_mut))]
    let mut more = Vec::new();
    #[cfg(feature = "demo")]
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
/// destinations, the guide and Settings under a divider; Lock and Power
/// off are the top app bar's power button's.
pub(crate) fn rail(app: &View, ui: &mut Ui, h: f32) {
    let (mut top, more) = destinations(app);
    // Check stands right under Home: a rail takes up to seven destinations,
    // the bar keeps its five (F255).
    top.insert(
        1,
        dest(
            app,
            Icon::Verified,
            EN.sidebar_check,
            Action::Nav(Screen::Check),
        ),
    );
    top.extend(more);
    let under = [dest(
        app,
        Icon::Settings,
        EN.sidebar_settings,
        Action::Nav(Screen::Settings),
    )];
    faraday_ui::nav::rail(
        ui,
        h,
        &Rail {
            fab: Some(faraday_ui::nav::RailFab {
                icon: Icon::Scan,
                label: EN.home_scan,
                action: Action::Scan,
            }),
            top: &top,
            under: &under,
            foot: &[],
            mark: true,
        },
    );
}

/// The navigation bar across the window's foot at `y`, `w` wide: the
/// five destinations, always the same: Tools is never displaced (F235).
pub(crate) fn bar(app: &View, ui: &mut Ui, y: f32, w: f32) {
    let (five, _more) = destinations(app);
    faraday_ui::nav::bar(ui, y, w, &five);
}

/// The Guide button, its right edge at `right`; on the wide
/// layout the codex32 sheets' eye stands left of it, so that it keeps its
/// place in the page's margin. Returns where the next action ends. A lookup.
fn guide_tools(app: &View, ui: &mut Ui, mut right: f32, nav: Nav) -> f32 {
    right = faraday_ui::nav::help(ui, right, Action::Guide, EN.tip_guide);
    if nav == Nav::ExpandedRail {
        right = crate::compact::bar_eye(app, ui, right);
    }
    right
}

/// The top app bar across the page, `w` wide: the way back and the
/// page's name (never the wordmark: the rail carries the mark), the way back only below a destination, and at its right end the ⋮ menu
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
        }
        // Lock, Restart and Power off, left of the ⋮ and right of the ?.
        if !hidden {
            *right = faraday_ui::nav::icon_action(
                ui,
                *right,
                Icon::Power,
                Action::NavMenu(MenuPress::Power),
                matches!(app.sheet, Some(Sheet::PowerMenu(_))),
                EN.tip_power,
            );
        }
        *right = guide_tools(app, ui, *right, nav);
        *right = crate::compact::bar_choices(app, ui, *right);
        if crate::modes::screen::shows_pill(app) {
            let pw = crate::modes::screen::pill_width(app, ui);
            let px = *right - GAP - pw;
            crate::modes::screen::pill(app, ui, px, (APP_BAR_H - MODE_PILL_HEIGHT) / 2.0);
            *right = px - GAP;
        }
        *right = crate::compact::receiver_mark(app, ui, *right);
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
            // A step flow's: Cancel, as the one-column layouts' bar has it
            // (`compact::flow_bar_tools`); the eye stands by the ?.
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
            None => (crate::compact::screen_name(app.screen), None),
        };
        let bar = AppBar {
            back: back.filter(|_| !super::top_destination(app)),
            title,
            back_tip: EN.tip_back,
        };
        faraday_ui::nav::app_bar(ui, w, &bar, ends);
        // The page's search, and Home's first actions, stand in the bar.
        crate::screens::bar_search_draw(app, ui, (w, bar.title, ends));
    } else {
        let (back, name) = crate::compact::page_head(app);
        let back = app.named_back(back);
        faraday_ui::nav::app_bar(
            ui,
            w,
            &AppBar {
                back: (!super::top_destination(app)).then_some(back),
                title: name,
                back_tip: EN.tip_back,
            },
            ends,
        );
        // The narrow rail's bar holds the page's search too.
        if app.search_in_bar() {
            crate::screens::bar_search_draw(app, ui, (w, name, ends));
        }
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

    /// The height the rows take, from where they began at `top`.
    fn height(&self, top: f32) -> f32 {
        if self.any {
            self.y - top + STATUS_CHIP_H + 2.0 * GAP
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

/// One line of Home's box of what is loaded: its icon, its text and
/// what a press opens.
struct Line {
    icon: Option<Glyph>,
    head: String,
    tail: String,
    action: Option<Action>,
}

/// The surface of a box of what is loaded: a tinted, outlined, rounded
/// container `w` by `h` at (x, y). The Pi's and the phone's box and the
/// wide Home's box of list rows are drawn on it.
pub(crate) fn loaded_surface(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32) {
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    ui.stroke(x, y, w, h, SHAPE_M, Role::OutlineVariant);
}

/// Home's box of what is loaded where there is no expanded rail: one
/// card, a plain line in it for each flow in progress, key, wallet, the
/// files and a stick's import waiting, each opening what its sidebar row
/// opens. Drawn at (x, y) in `w`; returns the height it takes.
pub(crate) fn loaded_card(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let (lines, at, row_h, h) = loaded_layout(app, ui, w);
    let room = w - 2.0 * PAD;
    loaded_surface(ui, x, y, w, h);
    for (i, l) in lines.iter().enumerate() {
        let (ox, r) = at[i];
        let line_h = row_h[r];
        let lx = x + PAD + ox;
        let ly = y + GAP_SMALL + row_h[..r].iter().sum::<f32>();
        let next = at
            .get(i + 1)
            .filter(|n| n.1 == r)
            .map_or(room, |n| n.0 - PAD);
        let lw = (next - ox).min(room - ox);
        if let Some(a) = l.action {
            ui.press(
                Shape::new(lx - GAP_SMALL, ly, lw + 2.0 * GAP_SMALL, line_h, SHAPE_XS),
                Role::OnSurface.paint(),
                a,
            );
        }
        let mut tx = lx;
        if let Some(ic) = l.icon {
            let iy = ly + (line_h - ICON_SMALL) / 2.0;
            ui.icon(
                tx,
                iy,
                ICON_SMALL,
                ic,
                NAV_ICON - GAP_SMALL,
                Role::OnSurfaceVariant,
            );
            tx += ICON_SMALL + GRID;
        }
        // A line is as wide as its text was measured; a pixel over keeps
        // rounding from cutting a name short with room left beside it.
        let avail = lx + lw - tx + 1.0;
        let head = ui.fit_as(LABEL_L, &l.head, avail);
        let used = ui.text_mid_as((tx, ly, line_h), LABEL_L, Role::OnSurface, &head);
        if !l.tail.is_empty() {
            let tail = ui.fit_as(BODY_S, &l.tail, (avail - used - GRID).max(0.0));
            ui.text_mid_as(
                (tx + used + GRID, ly, line_h),
                BODY_S,
                Role::OnSurfaceVariant,
                &tail,
            );
        }
    }
    h
}

/// The height [`loaded_card`] takes in `w`, without drawing it.
pub(crate) fn loaded_card_h(app: &View, ui: &Ui, w: f32) -> f32 {
    loaded_layout(app, ui, w).3
}

/// Whether the box has nothing to show: no vault open, flow, key or wallet, no file
/// brought in or waiting, no stick and no import waiting. Home leaves the
/// box out then, as the wide layout does.
pub(crate) fn loaded_empty(app: &View) -> bool {
    app.vaults.open.is_empty()
        && app.flows_in_progress().is_empty()
        && app.session.keys.all().next().is_none()
        && app.session.wallets.is_empty()
        && app.inbox.is_empty()
        && app.outbox.is_empty()
        && app.stick.is_empty()
        && app.import.state().is_none()
}

/// What [`loaded_layout`] works out: the lines, where each stands (its
/// offset in its row, and the row), each row's height and the box's height.
type Laid = (Vec<Line>, Vec<(f32, usize)>, Vec<f32>, f32);

/// The box's lines laid out in `w`.
fn loaded_layout(app: &View, ui: &Ui, w: f32) -> Laid {
    let mut lines: Vec<Line> = Vec::new();
    // The vaults open stand in the first line.
    if !app.vaults.open.is_empty() {
        let n = app.vaults.open.len();
        let session = if app.session.keys.len() + app.session.wallets.len() == 0 {
            Screen::WalletsStart
        } else {
            Screen::Wallets
        };
        let (head, tail) = if n == 1 {
            (
                app.vaults.open[0].name.to_string(),
                EN.home_going_open.to_string(),
            )
        } else {
            (
                fill(
                    EN.compact_vaults_open,
                    &[&n.to_string(), EN.compact_vaults_inline],
                ),
                String::new(),
            )
        };
        lines.push(Line {
            icon: Some(Icon::Lock.into()),
            head,
            tail,
            action: Some(Action::Nav(session)),
        });
    }
    for r in app.flows_in_progress().iter() {
        let icon = match r.tab {
            Screen::Vaults => Icon::Lock,
            Screen::Catalog => Icon::Tools,
            _ => Icon::Wallet,
        };
        lines.push(Line {
            icon: Some(icon.into()),
            head: r.name.to_string(),
            tail: EN.home_in_progress.to_string(),
            action: Some(Action::Resume(r.going)),
        });
    }
    let more = |n: usize, i: usize, all: Action| Line {
        icon: None,
        head: fill(EN.sidebar_more, &[&(n - i).to_string()]),
        tail: String::new(),
        action: Some(all),
    };
    let keys: Vec<_> = app.session.keys.all().collect();
    for (i, k) in keys.iter().enumerate() {
        if i >= CHIPS_PER_GROUP && keys.len() > CHIPS_PER_GROUP + 1 {
            lines.push(more(keys.len(), i, Action::Explore));
            break;
        }
        let fp = fp_text(k.fingerprint);
        // A key named by its fingerprint is not named twice.
        let name = crate::bip85::key_name(k);
        let tail = if name.eq_ignore_ascii_case(&fp) {
            String::new()
        } else {
            name.to_string()
        };
        lines.push(Line {
            icon: Some(Icon::Keys.into()),
            head: fp.to_string(),
            tail,
            action: Some(Action::ExploreKey(k.fingerprint.0)),
        });
    }
    // The wallets that need something now are named; the rest are one
    // line, "+ {n} more" after those named, else "All wallets".
    let needing = crate::screens::wallets_needing(app);
    let total = app.session.wallets.len();
    let mut named = 0;
    for &index in needing.iter().take(CHIPS_PER_GROUP) {
        named += 1;
        lines.push(Line {
            icon: Some(Icon::Wallet.into()),
            head: app.session.wallets[index].name.to_string(),
            tail: String::new(),
            action: Some(Action::OpenWallet(index)),
        });
    }
    if total > named {
        let head = if named == 0 {
            fill(EN.home_all_wallets, &[&total.to_string()])
        } else {
            fill(EN.sidebar_more, &[&(total - named).to_string()])
        };
        lines.push(Line {
            icon: (named == 0).then(|| Icon::Wallet.into()),
            head,
            tail: String::new(),
            action: Some(Action::Nav(Screen::WalletsStart)),
        });
    }
    lines.push(Line {
        icon: Some(Icon::File.into()),
        head: app.home_files_line().to_string(),
        tail: String::new(),
        action: Some(Action::Nav(Screen::Files)),
    });
    // An import the boot stick brought waits behind its own line.
    if app.import.state().is_some() && app.stick.is_empty() {
        lines.push(Line {
            icon: Some(app.medium.icon()),
            head: EN.sidebar_import_waiting.to_string(),
            tail: String::new(),
            action: Some(crate::boot_import::OPEN),
        });
    }
    let room = w - 2.0 * PAD;
    // A key's line keeps its short height here; every other line is a
    // touch target tall.
    let heights: Vec<f32> = lines
        .iter()
        .map(|l| {
            let token = if matches!(l.action, Some(Action::ExploreKey(_))) {
                LOADED_LINE
            } else {
                ui.touch
            };
            ui.row_height(token, ui.line_as(LABEL_L), GAP_SMALL)
        })
        .collect();
    // Each line's width, then the rows they fill left to right; a line
    // wider than the row has a row of its own. A row is as tall as its
    // tallest line.
    let mut at: Vec<(f32, usize)> = Vec::with_capacity(lines.len());
    let (mut cx, mut row) = (0.0_f32, 0_usize);
    for l in &lines {
        let mut lw = ui.measure_as(LABEL_L, &l.head);
        if l.icon.is_some() {
            lw += ICON_SMALL + GRID;
        }
        if !l.tail.is_empty() {
            lw += GRID + ui.measure_as(BODY_S, &l.tail);
        }
        let lw = lw.min(room);
        if cx > 0.0 && cx + lw > room {
            cx = 0.0;
            row += 1;
        }
        at.push((cx, row));
        cx += lw + PAD;
    }
    let mut row_h = vec![0.0_f32; row + 1];
    for (i, &(_, r)) in at.iter().enumerate() {
        row_h[r] = row_h[r].max(heights[i]);
    }
    let h = row_h.iter().sum::<f32>() + 2.0 * GAP_SMALL;
    (lines, at, row_h, h)
}

/// The power button's menu: Lock (while something is held), Restart
/// (where the computer offers it) and Power off, each with its icon; on
/// the desktop app Lock and Quit. A popup under the button where there is
/// room, a sheet on a narrow one. Restart and Power off open the power
/// sheet, which names both (`docs/FLOWS.md`, Power off and Restart).
pub(crate) fn power_menu_sheet(
    app: &View,
    ui: &mut Ui,
    w: f32,
    h: f32,
    anchor: Option<(i32, i32, i32, i32)>,
) {
    let item = |icon, label: &str, t| {
        crate::menu::Line::Icon(icon, label.to_string(), Action::NavMenu(MenuPress::Tool(t)))
    };
    let mut lines = Vec::new();
    if app.holds_secret() {
        lines.push(item(Icon::Lock, EN.sidebar_lock, BarTool::Lock));
    }
    if !app.online && app.offers_restart() {
        lines.push(item(Icon::Restart, EN.power_restart, BarTool::Restart));
    }
    lines.push(item(
        Icon::Power,
        crate::screens::leave_label(app),
        BarTool::Power,
    ));
    crate::menu::draw(
        ui,
        w,
        h,
        &crate::menu::View {
            title: EN.power_title,
            icon: Icon::Power,
            lines: &lines,
            at: 0,
            keys: false,
            anchor,
            cancel: (EN.nav_close, Action::Cancel),
            shown: 1.0,
        },
    );
}

/// Home's **Start** menu: New wallet, Restore and Spend from backup, a
/// popup under the button wherever the button stands, whatever the
/// window (Material's menu: no title, no icons, rows 48 tall, kept inside
/// the window).
pub(crate) fn start_menu_sheet(
    _app: &View,
    ui: &mut Ui,
    w: f32,
    h: f32,
    anchor: Option<(i32, i32, i32, i32)>,
) {
    use faraday_ui::menu::{Menu, MenuLine};
    let lines = [
        MenuLine::Item(
            EN.home_ways_new_short,
            Action::NavMenu(MenuPress::Way(Way::New)),
            false,
        ),
        MenuLine::Item(
            EN.home_ways_restore_short,
            Action::NavMenu(MenuPress::Way(Way::Restore)),
            false,
        ),
        MenuLine::Item(
            EN.home_ways_spend_short,
            Action::NavMenu(MenuPress::Way(Way::Spend)),
            false,
        ),
    ];
    let m = Menu {
        title: None,
        icon: Glyph::Icon(Icon::Wallet),
        lines: &lines,
        at: None,
        anchor,
        cancel: (EN.nav_close, Action::Cancel),
        shown: 1.0,
    };
    // The popup is the wide window's drawing; the button stands on the
    // compact ones too.
    let compact = std::mem::replace(&mut ui.compact, false);
    faraday_ui::menu::menu(ui, w, h, &m);
    ui.compact = compact;
}

/// The ⋮ menu of the top app bar on the bar's windows: Settings, as a
/// sheet.
pub(crate) fn menu_sheet(_app: &View, ui: &mut Ui, w: f32, h: f32) {
    let lines = [crate::menu::Line::Item(
        EN.sidebar_settings.to_string(),
        Action::NavMenu(MenuPress::Go(Screen::Settings)),
    )];
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
