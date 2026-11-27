//! The navigation and the top app bar (09 §3.3): Material 3's navigation bar and small top app
//! bar, drawn in Faraday's look. Each draws what it is given and names the press each region
//! takes; which destinations there are, and which is current, is the app's.
//!
//! The roles are Material's (`docs/THEMES.md` §7): the current destination's pill is
//! secondary-container, its icon on-secondary-container and its label on-surface; the others are
//! on-surface-variant; hover and press are state layers, and the focus ring follows the shape the
//! state layer takes. The bars carry no rule along their edge.
//!
//! Not built here: the rail and the expanded rail's destination row (the wide window's, which the
//! device does not draw), the close button and the bar's actions' `under_bar` flag.

use faraday_atoms::button::{self, ButtonModel, Look};
use faraday_atoms::hits::Shape;
use faraday_atoms::{Anchor, Glyph, Icon, Offered, Pressed, Shown, Ui, W};
use faraday_tokens::dims::{
    APP_BAR_H, BADGE_LABEL, BADGE_LARGE, BADGE_PAD, BADGE_SMALL, BODY_L, CARET_MIN, GAP_SMALL,
    HOME_SEARCH_MIN, LABEL_M, MARGIN_COMPACT, MENU_GAP, MODE_PILL_HEIGHT, NAV_BAR_H, NAV_ICON,
    NAV_LABEL_GAP, NAV_PILL_H, NAV_PILL_W, SEARCH_BAR_H, SEARCH_GLYPH, SEARCH_ICON, SEARCH_INSET,
    TITLE_L, TITLE_M, TITLE_S, TOUCH,
};
use faraday_tokens::elevation::ELEVATION_SHEET;
use faraday_tokens::palette::{BADGE, ON_BADGE, Paint, Role};
use faraday_tokens::type_roles::TypeRole;

/// Material's badge on a destination's icon: a small dot where something waits, or a large badge
/// with a count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Badge {
    /// None.
    #[default]
    None,
    /// The small badge: a dot.
    Dot,
    /// The large badge with a count, "999+" past 999 (Material's most characters, four).
    Count(usize),
}

/// One destination: its icon, its label, what a press on it does, whether it is the one shown
/// now, and its badge.
pub struct Dest<'s, A> {
    pub icon: Glyph,
    pub label: Shown<'s>,
    pub offered: Offered<A>,
    /// It is the destination shown now.
    pub current: bool,
    /// What waits there, on its icon.
    pub badge: Badge,
    /// How many flows in progress stand there, on its icon too.
    pub flows: usize,
    /// The bar draws it as well as the rail: the bar keeps five destinations, the rail up to
    /// seven (`main`'s rule).
    pub bar: bool,
}

/// The destinations, the Scan button, the power button, the app's name and the lock tag: one
/// model, drawn as the rail, the bar or the sidebar as the screen template places it (04 V10).
/// Only the destinations and the count's words are filled: the rail's other parts are not built.
pub struct NavModel<'s, A> {
    pub dests: Vec<Dest<'s, A>>,
    /// A count past 999 as it is written ("999+").
    pub many: Shown<'s>,
}

/// The way back from a page: the name of where it lands, the phrase "Back to" it, its tooltip and
/// its press.
pub struct BackLink<'s, A> {
    /// Where Back goes, by name.
    pub name: Shown<'s>,
    /// The whole phrase, "Back to" the name.
    pub phrase: Shown<'s>,
    pub tip: Shown<'s>,
    pub press: A,
}

/// The title, the way back and the pills.
pub struct TopBarModel<'s, A> {
    pub back: Option<BackLink<'s, A>>,
    /// The page's name.
    pub title: Shown<'s>,
    /// The mode pill's width at the first line's end; nothing on mainnet.
    pub pill_w: f32,
    /// The title names an item, which nothing else on the bar names: cut inside the word where
    /// it does not fit.
    pub item_name: bool,
    /// Drawn over a sheet's scrim: the bar does not tint by the page's scroll.
    pub in_sheet: bool,
    /// Keys the scroll of the page under the bar, which tints it.
    pub under: Option<Anchor>,
    /// Where the actions drawn after the bar begin: the title is kept left of it.
    pub right: f32,
    /// The power button at the bar's right end: what it presses and its tooltip.
    pub power: Option<(A, Shown<'s>)>,
}

/// A large badge's count as it is written.
fn count_label(n: usize, many: Shown<'_>) -> String {
    if n > 999 {
        many.as_str().to_string()
    } else {
        n.to_string()
    }
}

/// A large badge's width for `label`.
fn count_width<A: Pressed>(ui: &Ui<'_, A>, label: &str) -> f32 {
    (ui.measure(BADGE_LABEL, W::S, label) + 2.0 * BADGE_PAD).max(BADGE_LARGE)
}

/// A large badge's pill at `(x, y)`, `label` centred in it.
fn count_pill<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    label: &str,
    fill: impl Into<Paint>,
    ink: impl Into<Paint>,
) {
    let h = BADGE_LARGE;
    let w = count_width(ui, label);
    ui.fill(x, y, w, h, h / 2.0, fill.into());
    let tw = ui.measure(BADGE_LABEL, W::S, label);
    ui.text_mid(
        x + (w - tw) / 2.0,
        y,
        h,
        BADGE_LABEL,
        W::S,
        ink.into(),
        label,
    );
}

/// Draws `badge` on an icon glyph whose box is `glyph` (x, y, side): the dot on its top right
/// corner, the large badge from its middle across its top right, as Material places them, in
/// Bitcoin orange with a white count, in every theme.
pub fn badge<A: Pressed>(
    ui: &mut Ui<'_, A>,
    glyph: (f32, f32, f32),
    badge: Badge,
    many: Shown<'_>,
) {
    let (gx, gy, gs) = glyph;
    match badge {
        Badge::None | Badge::Count(0) => {}
        Badge::Dot => {
            let d = BADGE_SMALL;
            ui.dot(gx + gs, gy, d / 2.0, BADGE);
        }
        Badge::Count(n) => {
            let label = count_label(n, many);
            let at = (gx + gs / 2.0, gy - BADGE_LARGE / 2.0);
            count_pill(ui, at, &label, BADGE, ON_BADGE);
        }
    }
}

/// The mark of `flows` in progress on an icon: a pill in the primary colour at the corner, beside
/// the destination's own badge `own` where it has one.
fn flow_pill<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (gx, gy, gs): (f32, f32, f32),
    own: Badge,
    flows: usize,
    many: Shown<'_>,
) {
    if flows == 0 {
        return;
    }
    let corner = gx + gs / 2.0;
    let x = match own {
        Badge::None | Badge::Count(0) => corner,
        Badge::Dot => gx + gs + BADGE_SMALL / 2.0 + GAP_SMALL,
        Badge::Count(n) => corner + count_width(ui, &count_label(n, many)) + GAP_SMALL,
    };
    let at = (x, gy - BADGE_LARGE / 2.0);
    count_pill(
        ui,
        at,
        &count_label(flows, many),
        Role::Primary,
        Role::OnPrimary,
    );
}

/// The height a destination's pill and label take together, its label's line at the text size.
fn item_content<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    NAV_PILL_H + NAV_LABEL_GAP + ui.line_as(LABEL_M)
}

/// A destination as the bar draws it: its icon on a pill over its label, centred in the box at
/// (x, y), `w` by `h`, the whole box pressable.
fn item<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    d: &Dest<'_, A>,
    many: Shown<'_>,
) {
    let top = y + ((h - item_content(ui)) / 2.0).max(0.0);
    let px = x + (w - NAV_PILL_W) / 2.0;
    if d.current {
        ui.fill(
            px,
            top,
            NAV_PILL_W,
            NAV_PILL_H,
            NAV_PILL_H / 2.0,
            Role::SecondaryContainer,
        );
    }
    let (icon_fg, fg) = if d.current {
        (Role::OnSecondaryContainer, Role::OnSurface)
    } else {
        (Role::OnSurfaceVariant, Role::OnSurfaceVariant)
    };
    // Ringed round its pill, the shape its state layer takes.
    let shape = Shape::new(px, top, NAV_PILL_W, NAV_PILL_H, NAV_PILL_H / 2.0);
    ui.offered(&d.offered, |ui, press, dimmed| {
        if let Some(a) = press {
            ui.press_to(shape, (x, y, w, h), icon_fg.paint(), a);
        }
        let _ = dimmed;
    });
    let ix = x + (w - NAV_PILL_H) / 2.0;
    ui.icon(ix, top, NAV_PILL_H, d.icon, NAV_ICON, icon_fg);
    let inset = (NAV_PILL_H - NAV_ICON) / 2.0;
    let glyph = (ix + inset, top + inset, NAV_ICON);
    badge(ui, glyph, d.badge, many);
    flow_pill(ui, glyph, d.badge, d.flows, many);
    // Material's label medium, the current destination's heavier.
    let role = if d.current {
        LABEL_M
    } else {
        LABEL_M.regular()
    };
    // A one-line slot: fitted down to Standard (`docs/TEXT-SIZE.md` §3).
    let room = w - 2.0 * GAP_SMALL;
    let label = d.label.as_str();
    let lw = ui.text_fit_width(role, label, room);
    let ly = top + NAV_PILL_H + NAV_LABEL_GAP;
    // The band its own line tall, so the label's top is at `ly`.
    let band = (
        x + (w - lw) / 2.0,
        ly,
        ui.line(role.size, role.face.into()),
        room,
    );
    ui.showing(d.label, |ui| {
        ui.text_fit_as(band, role, fg, label);
    });
}

/// Draws the navigation as the rail. Not built: the wide window's rail is not on the device's
/// path.
pub fn draw<A: Pressed>(_m: &NavModel<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}

/// Draws the navigation as the bar across the window at `y`, `w` wide: surface-container, with
/// no rule along its top (Material's navigation bar), its destinations evenly spaced.
pub fn draw_bar<A: Pressed>(m: &NavModel<'_, A>, ui: &mut Ui<'_, A>, (y, w): (f32, f32)) {
    ui.fill(0.0, y, w, NAV_BAR_H, 0.0, Role::SurfaceContainer);
    let on_bar = || m.dests.iter().filter(|d| d.bar);
    let iw = w / on_bar().count().max(1) as f32;
    for (i, d) in on_bar().enumerate() {
        item(ui, (i as f32 * iw, y, iw, NAV_BAR_H), d, m.many);
    }
}

/// The bar's own fill over part of it, from `x`, `w` wide and `h` tall, as the page stands under
/// it now: [`draw_top`]'s, and behind what is drawn over its end after it.
pub fn fill_behind<A: Pressed>(
    m: &TopBarModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, w, h): (f32, f32, f32),
) {
    let offset = match m.under {
        Some(id) if !m.in_sheet => ui.scroll_of(id),
        _ => 0.0,
    };
    if ui.theme.is_high_contrast() {
        let fill = if offset > 0.0 {
            Role::SurfaceContainer
        } else {
            Role::Surface
        };
        ui.fill(x, 0.0, w, h, 0.0, fill);
    } else {
        // The scrolled fill comes in with the page's first units under the bar, and the page's
        // top edge fades into it.
        let tone = ui.bar_tone(offset);
        ui.wash(x, 0.0, w, h, 0.0, Paint::Fixed(tone));
    }
}

/// How the bar sets the page's name.
enum Name {
    /// One line in this role.
    One(TypeRole, String),
    /// Two lines, wrapped at a word, in this role.
    Two(TypeRole, String, String),
}

/// The page's name as the bar shows it (`docs/DESIGN.md` §4.1), `first` wide on the first line
/// (the pill stands at its end) and `second` on the second: whole in title large, Material's role
/// for the bar; else whole in title medium; else wrapped at a word onto the second line in title
/// medium, or the next smaller role where two lines of it would not fit the bar; else, where one
/// word does not fit a line, in title medium shortened at a word with an ellipsis; else left off,
/// where no whole word would show; an item's name is cut inside the word instead, since nothing
/// else on the bar names it. Cheap: a few widths measured.
fn bar_name<A: Pressed>(
    ui: &Ui<'_, A>,
    title: &str,
    item_name: bool,
    first: f32,
    second: f32,
) -> Name {
    for role in [TITLE_L, TITLE_M] {
        if ui.measure_as(role, title) <= first {
            return Name::One(role, title.to_string());
        }
    }
    let words: Vec<&str> = title.split_whitespace().collect();
    for role in [TITLE_M, TITLE_S] {
        if 2.0 * ui.line_as(role) > APP_BAR_H {
            continue;
        }
        // The most words the first line holds; the rest must fit the second.
        let Some(k) = (1..words.len())
            .rev()
            .find(|&k| ui.measure_as(role, &words[..k].join(" ")) <= first)
        else {
            continue;
        };
        let rest = words[k..].join(" ");
        if ui.measure_as(role, &rest) <= second {
            return Name::Two(role, words[..k].join(" "), rest);
        }
    }
    for k in (1..words.len()).rev() {
        let short = format!("{}…", words[..k].join(" "));
        if ui.measure_as(TITLE_M, &short) <= first {
            return Name::One(TITLE_M, short);
        }
    }
    let cut = if item_name {
        ui.fit_as(TITLE_M, title, first).to_string()
    } else {
        String::new()
    };
    Name::One(TITLE_M, cut)
}

/// The top of the mode pill when its centre is at `mid`, kept in the bar.
fn pill_top_at(mid: f32) -> f32 {
    (mid - MODE_PILL_HEIGHT / 2.0).clamp(0.0, APP_BAR_H - MODE_PILL_HEIGHT)
}

/// A name wrapped onto both lines of the bar, centred in it. Returns the pill's top, level with
/// the first.
fn two_lines<A: Pressed>(
    ui: &mut Ui<'_, A>,
    x: f32,
    role: TypeRole,
    (one, two): (&str, &str),
) -> f32 {
    let th = ui.line_as(role);
    let top = (APP_BAR_H - 2.0 * th) / 2.0;
    ui.text_mid_as((x, top, th), role, Role::OnSurface, one);
    ui.text_mid_as((x, top + th, th), role, Role::OnSurface, two);
    pill_top_at(top + th / 2.0)
}

/// Whether the bar of a page with a way back shows its title whole before its actions: on one
/// line, or wrapped onto two. Cheap: a few widths measured.
pub fn title_whole<A: Pressed>(m: &TopBarModel<'_, A>, ui: &Ui<'_, A>) -> bool {
    let room = (m.right - GAP_SMALL - TOUCH).max(0.0);
    let first = (room - m.pill_w).max(0.0);
    let title = m.title.as_str();
    match bar_name(ui, title, m.item_name, first, room) {
        Name::One(_, name) => name == title,
        Name::Two(..) => true,
    }
}

/// Draws the top app bar, `w` wide: Material 3's small top app bar, [`APP_BAR_H`] across the
/// page, the way back where the page has one, the page's name in title large with where Back goes
/// under it in label medium ("Back to Size", or "Size" alone where that would be cut), the name
/// wrapping onto the second line, Back's label left off, where it does not fit the first. Surface,
/// and surface-container once the page is scrolled under it; no rule under it. Returns the top of
/// the mode pill, at the first line's end, centred on the page's name's line.
pub fn draw_top<A: Pressed>(m: &TopBarModel<'_, A>, ui: &mut Ui<'_, A>, w: f32) -> f32 {
    fill_behind(m, ui, (0.0, w, APP_BAR_H));
    let title = m.title.as_str();
    let mut x = MARGIN_COMPACT;
    let limit = match &m.power {
        Some((action, tip)) => icon_action(
            ui,
            (w - GAP_SMALL, APP_BAR_H / 2.0),
            Icon::Power,
            (action.clone(), false, *tip),
        ),
        None => w,
    }
    .min(m.right);
    let room = |x: f32| (limit - x).max(0.0);
    let Some(back) = &m.back else {
        let first = (room(x) - m.pill_w).max(0.0);
        return match bar_name(ui, title, m.item_name, first, room(x)) {
            Name::One(role, name) => {
                ui.text_mid_as((x, 0.0, APP_BAR_H), role, Role::OnSurface, &name);
                pill_top_at(APP_BAR_H / 2.0)
            }
            Name::Two(role, one, two) => two_lines(ui, x, role, (&one, &two)),
        };
    };
    let (bx, by) = (GAP_SMALL, (APP_BAR_H - TOUCH) / 2.0);
    let round = Shape::new(bx, by, TOUCH, TOUCH, TOUCH / 2.0);
    ui.press_to(
        round,
        (bx, by, TOUCH, TOUCH),
        Role::OnSurfaceVariant.paint(),
        back.press.clone(),
    );
    ui.icon(
        bx,
        by,
        TOUCH,
        Icon::ChevronLeft,
        NAV_ICON,
        Role::OnSurfaceVariant,
    );
    ui.tip_plain((bx, by, TOUCH, TOUCH), &back.press, back.tip.as_str());
    ui.back(back.press.clone());
    x = bx + TOUCH;
    let first = (room(x) - m.pill_w).max(0.0);
    let name = bar_name(ui, title, m.item_name, first, room(x));
    match name {
        // "Back to Kind"; the name alone where that would be cut. Where the name wraps it takes
        // both lines and Back's label is left off.
        Name::One(role, name) => {
            let (phrase, bare) = (back.phrase.as_str(), back.name.as_str());
            let line = room(x);
            let label = if ui.measure_as(LABEL_M, phrase) <= line {
                phrase.to_string()
            } else if bare
                .split_whitespace()
                .next()
                .is_some_and(|word| ui.measure_as(LABEL_M, word) > line)
            {
                // Not even its first word shows whole: left off, never "T…".
                String::new()
            } else {
                ui.fit_as(LABEL_M, bare, line).to_string()
            };
            let (th, lh) = (ui.line_as(role), ui.line_as(LABEL_M));
            let top = (APP_BAR_H - th - lh) / 2.0;
            ui.text_mid_as((x, top, th), role, Role::OnSurface, &name);
            ui.text_mid_as((x, top + th, lh), LABEL_M, Role::OnSurfaceVariant, &label);
            pill_top_at(top + th / 2.0)
        }
        Name::Two(role, one, two) => two_lines(ui, x, role, (&one, &two)),
    }
}

/// An action at the bar's right end, its right edge at `right`, centred at `mid`: an icon in a
/// touch target, lit while `on`, named `label` by its tooltip. Returns where the next action ends.
pub fn icon_action<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (right, mid): (f32, f32),
    icon: impl Into<Glyph>,
    (action, on, label): (A, bool, Shown<'_>),
) -> f32 {
    let (x, y) = (right - TOUCH, (mid - TOUCH / 2.0).max(0.0));
    // Lit: a toggled icon button's selected state, its container secondary-container and its
    // icon on it.
    if on {
        ui.fill(x, y, TOUCH, TOUCH, TOUCH / 2.0, Role::SecondaryContainer);
    }
    let fg = if on {
        Role::OnSecondaryContainer
    } else {
        Role::OnSurfaceVariant
    };
    let round = Shape::new(x, y, TOUCH, TOUCH, TOUCH / 2.0);
    ui.press_to(round, (x, y, TOUCH, TOUCH), fg.paint(), action.clone());
    ui.icon(x, y, TOUCH, icon, NAV_ICON, fg);
    ui.tip_plain((x, y, TOUCH, TOUCH), &action, label.as_str());
    x
}

/// The help button that opens what explains the page, its right edge at `right`, named `label`
/// by its tooltip: Material's standard icon button with the help symbol. Returns where the next
/// action ends.
pub fn help<A: Pressed>(ui: &mut Ui<'_, A>, right: f32, action: A, label: Shown<'_>) -> f32 {
    icon_action(
        ui,
        (right, APP_BAR_H / 2.0),
        Icon::Help,
        (action, false, label),
    )
}

/// The ⋮ that opens the page's menu, its right edge at `right`, named `label` by its tooltip.
/// Returns where the next action ends.
pub fn more<A: Pressed>(ui: &mut Ui<'_, A>, right: f32, action: A, label: Shown<'_>) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    let round = Shape::new(x, y, TOUCH, TOUCH, TOUCH / 2.0);
    ui.press_to(
        round,
        (x, y, TOUCH, TOUCH),
        Role::OnSurfaceVariant.paint(),
        action.clone(),
    );
    let (cx, cy) = (x + TOUCH / 2.0, y + TOUCH / 2.0);
    let r = GAP_SMALL / 2.0;
    for k in [-1.0f32, 0.0, 1.0] {
        ui.dot(
            cx,
            cy + k * (GAP_SMALL + r * 2.0),
            r,
            Role::OnSurfaceVariant,
        );
    }
    ui.tip_plain((x, y, TOUCH, TOUCH), &action, label.as_str());
    x
}

/// How Home's first actions stand beside the search field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ways {
    /// Three text pills.
    Pills,
    /// One Start button that opens a menu of the three.
    Start,
}

/// Where the group (field, then the actions) starts, how wide the field is and how the actions
/// stand. `(from, to)` is the room (the bar's, between the name and the actions; the page row's
/// whole width), `centre` the line the group is centred on where it fits there, `pills` the three
/// pills' width with their gaps (none where only the one button may stand), `start` the one
/// button's width with its gap, `want` the field's width where nothing is short. The field
/// shrinks to [`HOME_SEARCH_MIN`] before the pills give way to the one button, and the field
/// takes what the button leaves after that. A page with no actions passes `start` 0. The one
/// rule of the bar and of the page's row.
pub fn place(
    (from, to): (f32, f32),
    centre: f32,
    (pills, start): (Option<f32>, f32),
    want: f32,
) -> (f32, f32, Ways) {
    let room = (to - from).max(0.0);
    let (ways, extra) = match pills {
        Some(p) if room - p >= HOME_SEARCH_MIN => (Ways::Pills, p),
        _ => (Ways::Start, start),
    };
    let sw = want.min((room - extra).max(0.0));
    let group = sw + extra;
    let at = (centre - group / 2.0).clamp(from, (to - group).max(from));
    (at, sw, ways)
}

/// How a search field or a pill is lifted off the bar: level 1 on a menu page, none in a flow
/// (at High contrast, which casts no shadow, an outline either way).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raise {
    /// Level 1.
    Level,
    /// Flat.
    Flat,
}

/// The bar's search field: Material's search bar, a full-round surface-container-high field, the
/// magnifying glass leading, the hint until something is typed, a trailing x that clears while it
/// holds text. Typing lands in it with no press first, so at rest it draws no caret; a press shows
/// one until something is typed.
pub struct SearchBarModel<'s, A> {
    /// What is typed.
    pub typed: Shown<'s>,
    pub hint: Shown<'s>,
    /// The typing goes to it.
    pub focused: bool,
    /// The press that focuses it.
    pub here: A,
    /// The press that clears it, and its tooltip.
    pub clear: (A, Shown<'s>),
    pub raise: Option<Raise>,
}

/// Home's first actions beside the field: the three pills (text-only) and the one Start button
/// that stands for them where they do not fit. A pill not offered is drawn dimmed.
pub struct WaysModel<'s, A> {
    pub pills: Vec<ButtonModel<'s, A>>,
    pub start: ButtonModel<'s, A>,
    /// Whether the three pills may stand here at all (not in the page's row).
    pub pills_ok: bool,
    pub raise: Option<Raise>,
}

/// Gives the box (x, y, w, h) its height: level 1 and, at High contrast, an outline.
fn raise<A: Pressed>(how: Option<Raise>, ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32)) {
    let Some(how) = how else { return };
    if how == Raise::Level {
        ui.shadow(x, y, w, h, h / 2.0, ELEVATION_SHEET);
    }
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, h / 2.0, Role::Outline);
    }
}

/// Draws the search field at (x, y), `w` wide. Returns the height it takes, the gap under it
/// counted.
pub fn draw_search<A: Pressed>(
    m: &SearchBarModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let h = SEARCH_BAR_H;
    raise(m.raise, ui, (x, y, w, h));
    ui.fill(x, y, w, h, h / 2.0, Role::SurfaceContainerHigh);
    // The state layer takes the pill's own shape, not the default corner.
    ui.press_to(
        Shape::new(x, y, w, h, h / 2.0),
        (x, y, w, h),
        Role::OnSurface.paint(),
        m.here.clone(),
    );
    ui.icon(
        x + SEARCH_INSET,
        y + (h - SEARCH_ICON) / 2.0,
        SEARCH_ICON,
        Icon::SEARCH,
        SEARCH_GLYPH,
        Role::OnSurfaceVariant,
    );
    let tx = x + 2.0 * SEARCH_INSET + SEARCH_ICON;
    let typed = m.typed.as_str();
    let clear = if typed.is_empty() { 0.0 } else { TOUCH };
    let room = (x + w - tx - SEARCH_INSET - clear).max(0.0);
    ui.showing(m.typed, |ui| {
        if typed.is_empty() {
            let shown = ui.fit_as(BODY_L, m.hint.as_str(), room);
            ui.text_mid_as((tx, y, h), BODY_L, Role::OnSurfaceVariant, &shown);
            if m.focused {
                ui.caret(tx, y + (h - CARET_MIN) / 2.0, CARET_MIN);
            }
        } else {
            ui.typed(
                tx,
                y,
                h,
                BODY_L.size,
                BODY_L.face.into(),
                Role::OnSurface,
                typed,
                room,
                true,
                m.here.clone(),
            );
        }
    });
    if !typed.is_empty() {
        let cx = x + w - GAP_SMALL - TOUCH;
        let cy = y + (h - TOUCH) / 2.0;
        let cross = "\u{d7}";
        ui.text_mid_as(
            (
                cx + (TOUCH - ui.measure_as(TITLE_M, cross)) / 2.0,
                cy,
                TOUCH,
            ),
            TITLE_M,
            Role::OnSurfaceVariant,
            cross,
        );
        ui.hit_drawn(cx, cy, TOUCH, TOUCH, m.clear.0.clone());
        ui.tip_plain((cx, cy, TOUCH, TOUCH), &m.clear.0, m.clear.1.as_str());
    }
    h + faraday_tokens::dims::GAP
}

/// The three pills' widths, each with its gap before it.
fn way_widths<A: Pressed>(w: &WaysModel<'_, A>, ui: &Ui<'_, A>) -> Vec<f32> {
    w.pills
        .iter()
        .map(|p| MENU_GAP + ui.button_width(p.label.as_str(), Look::Tonal))
        .collect()
}

/// The one Start button's width with its gap.
fn start_width<A: Pressed>(w: &WaysModel<'_, A>, ui: &Ui<'_, A>) -> f32 {
    MENU_GAP + ui.button_icon_width(w.start.label.as_str(), Look::Tonal)
}

/// The search and Home's first actions from `from` to `to`, at height `y`: the one place that
/// draws them, in the top bar and in the page's row. `centre` is the line the group centres on,
/// `want` the field's widest. `ways` is none for a page with no actions. Returns where the group
/// ends.
pub fn draw_group<A: Pressed>(
    search: &SearchBarModel<'_, A>,
    ways: Option<&WaysModel<'_, A>>,
    ui: &mut Ui<'_, A>,
    (range, centre): ((f32, f32), f32),
    (y, want): (f32, f32),
) -> f32 {
    let widths = ways.map(|w| way_widths(w, ui)).unwrap_or_default();
    let start_w = ways.map_or(0.0, |w| start_width(w, ui));
    let pills = ways
        .filter(|w| w.pills_ok)
        .map(|_| widths.iter().sum::<f32>());
    let (start, sw, how) = place(range, centre, (pills, start_w), want);
    draw_search(search, ui, (start, y, sw));
    let mut x = start + sw;
    let Some(ways) = ways else { return x };
    match how {
        Ways::Start => {
            raise(
                ways.raise,
                ui,
                (x + MENU_GAP, y, start_w - MENU_GAP, SEARCH_BAR_H),
            );
            let b = &ways.start;
            ui.offered(&b.offered, |ui, press, _| {
                if let Some(a) = press {
                    ui.button_icon_after(
                        (x + MENU_GAP, y, Some(start_w - MENU_GAP), SEARCH_BAR_H),
                        Icon::ChevronDown,
                        b.label.as_str(),
                        Look::Tonal,
                        a,
                    );
                }
            });
            x += start_w;
        }
        Ways::Pills => {
            for (pill, w) in ways.pills.iter().zip(widths) {
                raise(
                    ways.raise,
                    ui,
                    (x + MENU_GAP, y, w - MENU_GAP, SEARCH_BAR_H),
                );
                button::draw(
                    pill,
                    ui,
                    (x + MENU_GAP, y, Some(w - MENU_GAP), SEARCH_BAR_H),
                );
                x += w;
            }
        }
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, pixel};
    use faraday_atoms::{ComponentId, Motion, MotionStore, Theme};

    /// The page's scroll, keyed by this anchor.
    const PAGE: Anchor = Anchor {
        scope: "page",
        n: 1,
    };

    /// Invariant: The top app bar looks like the page while the page is at its top and looks different once the page is scrolled under it.
    /// Regression: The bar keeps one fill however far the page is scrolled under it, so a page running under it is not told apart from it.
    /// Owner: kit
    #[test]
    fn the_top_app_bar_looks_different_once_the_page_scrolls_under_it() {
        let mut bars = Vec::new();
        for offset in [0.0, 30.0] {
            let mut c = canvas(160);
            let mut store = MotionStore::new();
            store.set(
                ComponentId(Anchor { scope: "", n: 0 }, PAGE),
                Motion::Region {
                    offset,
                    velocity: 0.0,
                    stretch: 0.0,
                    extent: 0.0,
                    bar_shown_ms: 0,
                },
            );
            let pointer = at_rest(None);
            let m: TopBarModel<'_, P> = TopBarModel {
                back: None,
                title: Shown::plain("Home"),
                pill_w: 0.0,
                item_name: false,
                in_sheet: false,
                under: Some(PAGE),
                right: 640.0,
                power: None,
            };
            let mut ui = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
            draw_top(&m, &mut ui, 640.0);
            ui.finish();
            bars.push(pixel(&c, 600, 4));
        }
        assert_ne!(bars[0], bars[1]);
    }
}
