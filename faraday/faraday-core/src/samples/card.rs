//! The "Demo only" card (owner, 2026-10-24; `docs/DEMO.md` §14): every
//! page and sheet keeps what it offers only in Demo, or only on Faraday
//! Desktop as a stand-in for the person's own input, out of its own
//! column, in one outlined card headed "Demo only" (`faraday_ui::demo`).
//! On a window with the room, the card is a supporting pane at the right,
//! its top below the top-right strip and its right edge `MARGIN_WIDE`
//! from the window's (`docs/LAYOUT.md` §6, §15); narrower, a strip pinned
//! at the window's foot. Make codex32 shares' entry has its own card
//! with the same look (`codex32/pane_screen.rs`).
//!
//! What the card holds: the buttons the page hands over where it would
//! have drawn them ([`Ui::demo_offer`]: Use a sample, Use faraday, a
//! scanner's sample, Load sample PSBT), the lines that say what Demo puts
//! in place of the person's own, and the computer's dice roller of New
//! key's dice and Generate with dice. Whether a page holds the card at
//! all is decided from the state before the page is drawn, so the page is
//! laid out beside it ([`Faraday::demo_card_now`]); the lines and the
//! roller are read from the state too. Lookups throughout.

use faraday_draw::tokens as t;
use faraday_ui::demo::{self, DemoItem};

use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Ui;
use crate::{Action, Screen};

/// The computer's dice roller the card holds (`docs/CODEX32.md` §5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Roller {
    /// New key's dice.
    KeyGen,
    /// Create a vault's Generate with dice.
    Vault,
}

/// What the frame's "Demo only" card holds besides the buttons the page
/// hands it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Hosted {
    /// The lines saying what Demo puts in place of the person's own.
    pub lines: Vec<String>,
    /// The computer's dice roller.
    pub roller: Option<Roller>,
}

/// Where the card stands on this window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Place {
    /// A supporting pane at the window's right.
    Side,
    /// A strip pinned at the window's foot, this tall.
    Strip(f32),
}

/// Where the card stands beside a page area `room` wide: at the right
/// where the page keeps [`t::DEMO_PAGE_MIN`] beside it on a window that
/// takes two panes ([`t::DEMO_PAGE_PANEL_MIN`] for a page with a side
/// panel of its own, or a sheet over one), else in a strip at the foot.
/// Arithmetic on the font.
pub(crate) fn place(app: &View, ui: &mut Ui, room: f32, hosted: &Hosted) -> Place {
    if beside(app, room) {
        Place::Side
    } else {
        Place::Strip(strip_h(ui, room, hosted))
    }
}

/// Whether the card stands at the right of a page area `room` wide, not
/// in a strip at the foot ([`place`]). Arithmetic.
pub(crate) fn beside(app: &crate::Faraday, room: f32) -> bool {
    let panel = matches!(
        app.screen,
        Screen::CreateVault | Screen::Backup | Screen::Spend | Screen::BackupFlow
    ) && app.sheet.is_none();
    let least = if panel {
        t::DEMO_PAGE_PANEL_MIN
    } else {
        t::DEMO_PAGE_MIN
    };
    !app.is_compact() && room - crate::screens::support_pane_room() >= least
}

/// Where the card stands as a pane for the page area at `x0`, `room`
/// wide, in a window `h` tall: the supporting pane's place
/// (`docs/LAYOUT.md` §6), its top just under the top-right strip's pills,
/// or under the codex32 sheets' eye where it shows. Its left edge, top,
/// width and the height it shows. Arithmetic.
pub(crate) fn pane_place(app: &View, (x0, room, h): (f32, f32, f32)) -> (f32, f32, f32, f32) {
    let (x, _, w, _) = crate::screens::support_pane_place((x0, room, h));
    let top = if crate::compact::eye_shown(app) {
        t::SUPPORT_PANE_TOP
    } else {
        t::DEMO_CARD_TOP
    };
    (x, top, w, (h - top - t::MARGIN_WIDE).max(t::TOUCH))
}

/// The room the card takes from the page at its right: none for the
/// strip.
pub(crate) fn side_room(place: Option<Place>) -> f32 {
    match place {
        Some(Place::Side) => crate::screens::support_pane_room(),
        _ => 0.0,
    }
}

/// The room the strip takes at the window's foot: none for the pane.
pub(crate) fn foot_room(place: Option<Place>) -> f32 {
    match place {
        Some(Place::Strip(h)) => h,
        _ => 0.0,
    }
}

/// The room the heading takes at the start of the strip's row.
fn head_room(ui: &Ui) -> f32 {
    ui.measure(t::BODY, crate::ui::W::S, EN.demo_only) + 2.0 * t::PAD
}

/// A button's width in the strip's row, as `wrap_buttons` sizes it.
fn button_w(ui: &Ui, label: &str) -> f32 {
    ui.measure(t::LABEL, crate::ui::W::S, label) + 2.0 * t::PAD
}

/// The strip's height in a window `w` wide before the page hands it its
/// buttons: its lines, then one row of "Demo only", the roller and a
/// button, wrapping where they do not fit. Arithmetic on the font.
fn strip_h(ui: &mut Ui, w: f32, hosted: &Hosted) -> f32 {
    let guess = [DemoItem::Button(EN.demo_use_faraday, Action::UseSample)];
    strip_need(ui, w, &guess, hosted)
}

/// The strip's height with `items` in it. Arithmetic on the font.
fn strip_need(ui: &mut Ui, w: f32, items: &[DemoItem<Action>], hosted: &Hosted) -> f32 {
    let lines: Vec<DemoItem<Action>> = hosted.lines.iter().cloned().map(DemoItem::Line).collect();
    let iw = w - 2.0 * t::PAD;
    let (top, mut from) = if lines.is_empty() {
        (t::GAP, t::PAD + head_room(ui))
    } else {
        let head = t::PAD + demo::head_h(ui);
        (head + demo::items_h(ui, iw, &lines), t::PAD)
    };
    let rh = row_h(hosted);
    let mut row = top;
    if hosted.roller.is_some() {
        (from, row) = crate::fresh::roller::screen::row_end(ui, (t::PAD, top, iw), from);
    }
    for i in items {
        if let DemoItem::Button(l, _) = i {
            let bw = button_w(ui, l);
            if from + bw > t::PAD + iw {
                from = t::PAD;
                row += rh + t::GAP;
            }
            from += bw + t::GAP;
        }
    }
    row + rh + t::GAP
}

/// The strip's row height: the roller's where it holds one, else a
/// button's.
fn row_h(hosted: &Hosted) -> f32 {
    if hosted.roller.is_some() {
        crate::fresh::roller::screen::ROW_H
    } else {
        t::TOUCH
    }
}

/// Draws the card for the page area at `x0`, `room` wide, in a window `h`
/// tall, holding `hosted` and the buttons the page handed `ui`.
pub(crate) fn draw(
    app: &View,
    ui: &mut Ui,
    place: Place,
    hosted: &Hosted,
    (x0, room, h): (f32, f32, f32),
) {
    let mut items: Vec<DemoItem<Action>> =
        hosted.lines.iter().cloned().map(DemoItem::Line).collect();
    items.extend(
        ui.take_demo()
            .into_iter()
            .filter(|i| matches!(i, DemoItem::Button(..))),
    );
    if items.is_empty() && hosted.roller.is_none() {
        return;
    }
    match place {
        Place::Side => {
            let (x, top, w, view) = pane_place(app, (x0, room, h));
            let clip = ui.rect(x, top, w, view);
            ui.c.push_clip(clip);
            let ch = body(app, ui, (x, top, w), &items, hosted);
            ui.c.pop_clip();
            demo::outline(ui, (x, top, w, ch.min(view)));
        }
        Place::Strip(sh) => {
            let sh = sh.max(strip_need(ui, room, &items, hosted));
            demo::cover(ui, (x0, h - sh, room, sh));
            strip(app, ui, (x0, h - sh, room), &items, hosted);
        }
    }
}

/// The strip's content at (`x`, `top`, `w`): its lines, then "Demo
/// only", the roller and the buttons in one row, wrapping where they do
/// not fit (owner, 2026-10-24: no taller than it needs).
fn strip(
    app: &View,
    ui: &mut Ui,
    (x, top, w): (f32, f32, f32),
    items: &[DemoItem<Action>],
    hosted: &Hosted,
) {
    let (ix, iw) = (x + t::PAD, w - 2.0 * t::PAD);
    let lines: Vec<DemoItem<Action>> = items
        .iter()
        .filter(|i| matches!(i, DemoItem::Line(_)))
        .cloned()
        .collect();
    let rh = row_h(hosted);
    // "Demo only" heads the strip: over its lines where it has any, else
    // at the start of its row.
    let (cy, start) = if lines.is_empty() {
        let ink = crate::ui::pal::Role::OnSurface;
        let cy = top + t::GAP;
        ui.text_mid(ix, cy, rh, t::BODY, crate::ui::W::S, ink, EN.demo_only);
        (cy, ix + head_room(ui))
    } else {
        let mut cy = top + t::PAD;
        cy += demo::head(ui, (ix, cy), EN.demo_only);
        cy += demo::items(ui, (ix, cy, iw), &lines);
        (cy, ix)
    };
    let (mut from, mut row) = (start, cy);
    if hosted.roller.is_some() {
        (from, row) = roller_row(app, ui, (ix, cy, iw), from, hosted);
    }
    for i in items {
        if let DemoItem::Button(l, a) = i {
            let bw = button_w(ui, l);
            if from + bw > ix + iw {
                from = ix;
                row += rh + t::GAP;
            }
            let by = row + (rh - t::TOUCH) / 2.0;
            ui.button(
                from,
                by,
                Some(bw),
                t::TOUCH,
                l,
                crate::ui::Style::Secondary,
                *a,
            );
            from += bw + t::GAP;
        }
    }
}

/// The roller the strip holds, in its row from `from`. Returns where it
/// ends.
fn roller_row(
    app: &View,
    ui: &mut Ui,
    at: (f32, f32, f32),
    from: f32,
    hosted: &Hosted,
) -> (f32, f32) {
    use crate::fresh::roller::screen::{Hosting, row};
    match hosted.roller {
        Some(Roller::KeyGen) => app.keygen.state().map_or((from, at.1), |k| {
            let host = Hosting {
                room: k.roller_room(),
                typing: k.roller_typing,
            };
            row(ui, at, from, &k.roller, host, Action::KRoller)
        }),
        Some(Roller::Vault) => {
            let host = Hosting {
                room: crate::vault::dice::room(app),
                typing: app.vaults.focus == Some(crate::vault::Focus::Rolls),
            };
            let act = |a| Action::Vault(crate::vault::VaultAction::Roller(a));
            row(ui, at, from, &app.vaults.roller, host, act)
        }
        None => (from, at.1),
    }
}

/// The card's content from its heading down at (`x`, `y`, `w`): the
/// heading, the lines, the roller, the buttons. Returns its height.
fn body(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    items: &[DemoItem<Action>],
    hosted: &Hosted,
) -> f32 {
    let (ix, iw) = (x + t::PAD, w - 2.0 * t::PAD);
    let mut cy = y + t::PAD;
    cy += demo::head(ui, (ix, cy), EN.demo_only);
    let (lines, buttons): (Vec<_>, Vec<_>) = items
        .iter()
        .cloned()
        .partition(|i| matches!(i, DemoItem::Line(_)));
    cy += demo::items(ui, (ix, cy, iw), &lines);
    cy += roller(app, ui, (ix, cy, iw), hosted);
    cy += demo::items(ui, (ix, cy, iw), &buttons);
    cy - y + t::GAP
}

/// The roller the card holds, drawn at (`x`, `y`, `w`). Returns its
/// height with the gap under it.
fn roller(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), hosted: &Hosted) -> f32 {
    use crate::fresh::roller::screen::{Hosting, draw};
    match hosted.roller {
        Some(Roller::KeyGen) => app.keygen.state().map_or(0.0, |k| {
            let host = Hosting {
                room: k.roller_room(),
                typing: k.roller_typing,
            };
            draw(ui, (x, y, w), &k.roller, host, Action::KRoller) + t::ROLLER_GAP
        }),
        Some(Roller::Vault) => {
            let host = Hosting {
                room: crate::vault::dice::room(app),
                typing: app.vaults.focus == Some(crate::vault::Focus::Rolls),
            };
            let act = |a| Action::Vault(crate::vault::VaultAction::Roller(a));
            draw(ui, (x, y, w), &app.vaults.roller, host, act) + t::ROLLER_GAP
        }
        None => 0.0,
    }
}
