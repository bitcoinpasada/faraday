//! The top app bar (`docs/LAYOUT.md` §3, DESIGN §4.1): Material 3's
//! small top app bar, `APP_BAR_H` across the page, the way back where
//! the page has one, the page's name in title large with where Back goes
//! under it in label medium, "Back to Size", or "Size" alone where that
//! would be cut (Material's `AppBarSmallTokens`), the name wrapping onto
//! the second line, Back's label left off, where it does not fit the
//! first, the mode pill at the first line's end off mainnet,
//! and its actions at the right end, each a touch target.

use crate::pal::*;
use crate::{Glyph, Shape, Ui, W};
use faraday_draw::tokens::{
    APP_BAR_H, GAP_SMALL, ICON_SMALL, LABEL, LABEL_M, MARGIN_COMPACT, MODE_PILL_HEIGHT, NAV_ICON,
    TITLE_L, TITLE_M, TITLE_S, TOUCH, TypeRole,
};
use faraday_draw::widgets::Icon;
use faraday_strings::EN;

/// What the bar shows at its left.
pub struct AppBar<'s, A> {
    /// The way back, and the name of where it lands, drawn under the
    /// page's name as "Back to" it (F77, F104).
    pub back: Option<(&'s str, A)>,
    /// The page's name.
    pub title: &'s str,
    /// What the way back's tooltip says.
    pub back_tip: &'static str,
}

/// The bar at the top of the page, `w` wide, its name kept left of
/// `right`, where the actions drawn after it begin. Surface, and
/// surface-container once the page is scrolled under it
/// ([`app_bar_fill`]); no rule under it (Material's top app bar).
pub fn app_bar<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, bar: &AppBar<A>, right: f32) {
    let fill = app_bar_fill(ui.offset > 0.0 && !ui.in_sheet);
    ui.fill(0.0, 0.0, w, APP_BAR_H, 0.0, fill);
    let mut x = MARGIN_COMPACT;
    let room = |x: f32| (right - x).max(0.0);
    if let Some((label, action)) = bar.back {
        let (bx, by) = (GAP_SMALL, (APP_BAR_H - TOUCH) / 2.0);
        let round = (bx, by, TOUCH, TOUCH, TOUCH / 2.0);
        ui.press_to(
            Shape::new(round.0, round.1, round.2, round.3, round.4),
            (bx, by, TOUCH, TOUCH),
            Role::OnSurfaceVariant.paint(),
            action,
        );
        ui.icon(
            bx,
            by,
            TOUCH,
            Icon::ChevronLeft,
            NAV_ICON,
            Role::OnSurfaceVariant,
        );
        ui.tip_plain((bx, by, TOUCH, TOUCH), action, bar.back_tip);
        ui.back(action);
        x = bx + TOUCH;
        let first = (room(x) - ui.bar_pill_w).max(0.0);
        let name = bar_name(ui, bar.title, first, room(x));
        // "Back to Kind"; the name alone where that would be cut. Where
        // the name wraps it takes both lines and Back's label is left off.
        let label = match &name {
            Name::One(..) => {
                let back_to = faraday_strings::fill(EN.bar_back_to, &[label]);
                let line = room(x);
                Some(if ui.measure_as(LABEL_M, &back_to) <= line {
                    back_to
                } else if label
                    .split_whitespace()
                    .next()
                    .is_some_and(|word| ui.measure_as(LABEL_M, word) > line)
                {
                    // Not even its first word shows whole: left off, never "T…".
                    String::new()
                } else {
                    ui.fit_as(LABEL_M, label, line)
                })
            }
            Name::Two(..) => None,
        };
        match (name, label) {
            (Name::One(role, name), Some(label)) => {
                let (th, lh) = (ui.line_as(role), ui.line_as(LABEL_M));
                let top = (APP_BAR_H - th - lh) / 2.0;
                ui.bar_pill_top = Some(pill_top_at(top + th / 2.0));
                ui.text_mid_as((x, top, th), role, Role::OnSurface, &name);
                ui.text_mid_as((x, top + th, lh), LABEL_M, Role::OnSurfaceVariant, &label);
            }
            (Name::Two(role, one, two), _) => two_lines(ui, x, role, &one, &two),
            (Name::One(..), None) => {}
        }
    } else {
        let first = (room(x) - ui.bar_pill_w).max(0.0);
        match bar_name(ui, bar.title, first, room(x)) {
            Name::One(role, name) => {
                ui.bar_pill_top = Some(pill_top_at(APP_BAR_H / 2.0));
                ui.text_mid_as((x, 0.0, APP_BAR_H), role, Role::OnSurface, &name);
            }
            Name::Two(role, one, two) => two_lines(ui, x, role, &one, &two),
        }
    }
}

/// A name wrapped onto both lines of the bar, centred in it; the pill
/// stands level with the first.
fn two_lines<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, role: TypeRole, one: &str, two: &str) {
    let th = ui.line_as(role);
    let top = (APP_BAR_H - 2.0 * th) / 2.0;
    ui.bar_pill_top = Some(pill_top_at(top + th / 2.0));
    ui.text_mid_as((x, top, th), role, Role::OnSurface, one);
    ui.text_mid_as((x, top + th, th), role, Role::OnSurface, two);
}

/// The top of the mode pill when its centre is at `mid`, kept in the bar.
fn pill_top_at(mid: f32) -> f32 {
    (mid - MODE_PILL_HEIGHT / 2.0).clamp(0.0, APP_BAR_H - MODE_PILL_HEIGHT)
}

/// Where the top of the mode pill is: at the end of the bar's first line,
/// centred on the page's name's line as the last [`app_bar`] drew it.
pub fn pill_top<A: Copy + PartialEq>(ui: &Ui<A>) -> f32 {
    ui.bar_pill_top
        .unwrap_or_else(|| pill_top_at(APP_BAR_H / 2.0))
}

/// The top app bar's container (`docs/THEMES.md` §7): surface, and
/// surface-container while the page is `scrolled` under it.
pub fn app_bar_fill(scrolled: bool) -> Role {
    if scrolled {
        Role::SurfaceContainer
    } else {
        Role::Surface
    }
}

/// Whether the bar of a page with a way back shows `title` whole before
/// its actions, which begin at `right`, as [`app_bar`] draws it: on one
/// line, or wrapped onto two. Cheap: a few widths measured.
pub fn bar_title_whole<A: Copy + PartialEq>(ui: &Ui<A>, title: &str, right: f32) -> bool {
    let room = (right - GAP_SMALL - TOUCH).max(0.0);
    let first = (room - ui.bar_pill_w).max(0.0);
    match bar_name(ui, title, first, room) {
        Name::One(_, name) => name == title,
        Name::Two(..) => true,
    }
}

/// Whether the bar shows `title` whole on its first line, as
/// [`bar_title_whole`] but not wrapped. Cheap: a width or two measured.
pub fn bar_title_one_line<A: Copy + PartialEq>(ui: &Ui<A>, title: &str, right: f32) -> bool {
    let room = (right - GAP_SMALL - TOUCH).max(0.0);
    let first = (room - ui.bar_pill_w).max(0.0);
    [TITLE_L, TITLE_M]
        .into_iter()
        .any(|role| ui.measure_as(role, title) <= first)
}

/// How the bar sets the page's name.
enum Name {
    /// One line in this role.
    One(TypeRole, String),
    /// Two lines, wrapped at a word, in this role.
    Two(TypeRole, String, String),
}

/// The page's name as the bar shows it (`docs/DESIGN.md` §4.1), `first`
/// wide on the first line (the pill stands at its end) and `second` on
/// the second: whole in title large, Material's role for the bar; else
/// whole in title medium; else wrapped at a word onto the second line in
/// title medium, or the next smaller role where two lines of it would not
/// fit the bar; else, where one word does not fit a line, in title medium
/// shortened at a word, its first word at least, with an ellipsis; else
/// left off (empty), where no whole word would show. Cheap: a few widths
/// measured.
fn bar_name<A: Copy + PartialEq>(ui: &Ui<A>, title: &str, first: f32, second: f32) -> Name {
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
    Name::One(TITLE_M, String::new())
}

/// An action at the bar's right end, its right edge at `right`: an icon
/// in a touch target, lit while `on`, named `label` by its tooltip.
/// Returns where the next action ends.
pub fn icon_action<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    icon: impl Into<Glyph>,
    action: A,
    on: bool,
    label: &'static str,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    // Lit: a toggled icon button's selected state, its container
    // secondary-container and its icon on it.
    if on {
        ui.fill(x, y, TOUCH, TOUCH, TOUCH / 2.0, Role::SecondaryContainer);
    }
    let fg = if on {
        Role::OnSecondaryContainer
    } else {
        Role::OnSurfaceVariant
    };
    let round = (x, y, TOUCH, TOUCH, TOUCH / 2.0);
    ui.press_to(
        Shape::new(round.0, round.1, round.2, round.3, round.4),
        (x, y, TOUCH, TOUCH),
        fg.paint(),
        action,
    );
    ui.icon(x, y, TOUCH, icon, NAV_ICON, fg);
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}

/// The ? that opens what explains the page, its right edge at `right`,
/// named `label` by its tooltip. Returns where the next action ends.
pub fn help<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    action: A,
    label: &'static str,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    let d = ICON_SMALL + GAP_SMALL * 2.0;
    let (cx, cy) = (x + (TOUCH - d) / 2.0, y + (TOUCH - d) / 2.0);
    let round = (cx, cy, d, d, d / 2.0);
    ui.press_to(
        Shape::new(round.0, round.1, round.2, round.3, round.4),
        (x, y, TOUCH, TOUCH),
        Role::OnSurfaceVariant.paint(),
        action,
    );
    ui.stroke(cx, cy, d, d, d / 2.0, Role::Outline);
    let tw = ui.measure(LABEL, W::S, "?");
    ui.text_mid(
        cx + (d - tw) / 2.0,
        cy,
        d,
        LABEL,
        W::S,
        Role::OnSurfaceVariant,
        "?",
    );
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}

/// The ⋮ that opens the page's menu, its right edge at `right`, named
/// `label` by its tooltip. Returns where the next action ends.
pub fn more<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    right: f32,
    action: A,
    label: &'static str,
) -> f32 {
    let (x, y) = (right - TOUCH, (APP_BAR_H - TOUCH) / 2.0);
    let round = (x, y, TOUCH, TOUCH, TOUCH / 2.0);
    ui.press_to(
        Shape::new(round.0, round.1, round.2, round.3, round.4),
        (x, y, TOUCH, TOUCH),
        Role::OnSurfaceVariant.paint(),
        action,
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
    ui.tip_plain((x, y, TOUCH, TOUCH), action, label);
    x
}
