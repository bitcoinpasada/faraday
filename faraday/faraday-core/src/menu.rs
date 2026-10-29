//! A ⋮ menu, wherever one is (`docs/BACKUP-FLOW.md` §17.20 item 5,
//! `docs/FLOWS.md` decision 61): on `wide` a popup under the ⋮ (above it
//! where it does not fit below), on the small panel a sheet with its way
//! out at its foot; its title, then its rows, each place's name over the
//! choices there. A press outside it closes it. The keyboard reaches it:
//! the arrows move, Enter or Space chooses, Escape closes. The backup
//! board's chips and the file rows share it; each works out its rows on
//! the press that opens it, and this draws what it was given.

use faraday_draw::tokens;
use faraday_draw::widgets::Icon;
use faraday_shell_api::Key as KeyIn;

use crate::Action;
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Glyph, Ui};
use faraday_ui::menu::{Menu, MenuLine};

/// One row of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Line {
    /// A place's name, over the choices there.
    Head(String),
    /// A choice: what it reads, and the press it is.
    Item(String, Action),
}

/// A menu as it is drawn: what the press that opened it worked out.
pub(crate) struct View<'a> {
    /// Its title.
    pub title: &'a str,
    /// The glyph over its title on the small panel.
    pub icon: Icon,
    /// Its rows, in order.
    pub lines: &'a [Line],
    /// The choice the arrows are on, by its place among the choices.
    pub at: usize,
    /// Open or moved by a key: the arrows' choice is drawn so.
    pub keys: bool,
    /// Where the ⋮ was drawn, in pixels (x, y, w, h): the popup goes
    /// under it.
    pub anchor: Option<(i32, i32, i32, i32)>,
    /// The small panel's way out at the sheet's foot.
    pub cancel: (&'a str, Action),
    /// How much of the popup shows, 0 to 1, from its top edge down, while
    /// it opens or closes; 1 at rest.
    pub shown: f32,
}

/// What a key does in an open menu with `n` choices, the arrows on `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// The arrows move to this choice.
    To(usize),
    /// The choice the arrows are on is chosen.
    Choose,
    /// The menu closes, back to its ⋮.
    Close,
}

/// What `key` does in an open menu (§19.20 item 5): Down and Up move,
/// round at the ends, Enter or Space chooses, Escape closes; `None` for
/// any other key, which the menu takes and does nothing with.
pub(crate) fn step(key: KeyIn, at: usize, n: usize) -> Option<Step> {
    match key {
        KeyIn::Down if n > 0 => Some(Step::To((at + 1) % n)),
        KeyIn::Up if n > 0 => Some(Step::To((at + n - 1) % n)),
        KeyIn::Enter | KeyIn::Char(' ') => Some(Step::Choose),
        KeyIn::Escape => Some(Step::Close),
        _ => None,
    }
}

/// Where a ⋮ moves among those on screen, `dots`, in the order drawn,
/// for Tab or Shift+Tab from `at`: the first or the last from none.
pub(crate) fn tab<T: PartialEq + Copy>(dots: &[T], at: Option<T>, back: bool) -> Option<T> {
    let n = dots.len();
    if n == 0 {
        return None;
    }
    let i = at.and_then(|c| dots.iter().position(|d| *d == c));
    let next = match (back, i) {
        (false, Some(i)) => (i + 1) % n,
        (false, None) => 0,
        (true, Some(i)) => (i + n - 1) % n,
        (true, None) => n - 1,
    };
    Some(dots[next])
}

/// The ⋮'s three dots, centred at (cx, cy) in `color`.
pub(crate) fn dots(ui: &mut Ui, cx: f32, cy: f32, color: crate::ui::Paint) {
    let d = tokens::BOARD_MENU_DOT;
    let step = (tokens::BOARD_MARK - d) / 2.0;
    for k in [-1.0, 0.0, 1.0] {
        ui.fill(cx - d / 2.0, cy + k * step - d / 2.0, d, d, d / 2.0, color);
    }
}

/// A ⋮ on its own, as a quiet square button `h` high at (x, y): a press
/// is `action`; edged in the accent while the keyboard is on it
/// (`focused`). Returns its width.
pub(crate) fn button(ui: &mut Ui, x: f32, y: f32, h: f32, action: Action, focused: bool) -> f32 {
    let w = h;
    let r = tokens::SHAPE_S;
    ui.press(Shape::new(x, y, w, h, r), Role::OnSurface.paint(), action);
    ui.stroke(x, y, w, h, r, Role::Outline);
    dots(ui, x + w / 2.0, y + h / 2.0, Role::OnSurfaceVariant.paint());
    if focused {
        outline(
            ui,
            (x, y, w, h),
            r,
            tokens::FOCUS_STROKE,
            Role::Primary.paint(),
        );
    }
    w
}

/// A rounded rectangle's edge, `width` thick.
pub(crate) fn outline(
    ui: &mut Ui,
    r: (f32, f32, f32, f32),
    radius: f32,
    width: f32,
    color: crate::ui::Paint,
) {
    let rect = ui.rect(r.0, r.1, r.2, r.3);
    let c = ui.col(color);
    ui.c.stroke_rounded_rect(rect, radius * ui.f, width * ui.f, c);
}

/// What `draw` would take in height, drawn out of sight with no press
/// kept.
pub(crate) fn measure(ui: &mut Ui, draw: &mut dyn FnMut(&mut Ui) -> f32) -> f32 {
    let mark = ui.presses_mark();
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.presses_rewind(mark);
    h
}

/// Costs measuring every line. The menu drawn as Material's menu
/// ([`faraday_ui::menu::menu`]): a popup under its ⋮ on `wide`, a sheet
/// on the small panel (§19.20 item 5).
pub(crate) fn draw(ui: &mut Ui, w: f32, h: f32, v: &View) {
    let lines: Vec<MenuLine<Action>> = v
        .lines
        .iter()
        .map(|l| match l {
            Line::Head(name) => MenuLine::Head(name.as_str()),
            Line::Item(label, a) => MenuLine::Item(label.as_str(), *a, false),
        })
        .collect();
    faraday_ui::menu::menu(
        ui,
        w,
        h,
        &Menu {
            title: Some(v.title),
            icon: Glyph::Icon(v.icon),
            lines: &lines,
            at: v.keys.then_some(v.at),
            anchor: v.anchor,
            cancel: v.cancel,
            shown: v.shown,
        },
    );
}
