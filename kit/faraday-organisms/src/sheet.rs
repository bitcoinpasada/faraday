//! The sheet organism: a sheet on a small panel, a dialog and a side sheet. Each is as tall as
//! what it holds, its buttons kept at its foot, and what is above them scrolled when it is
//! taller than the panel (09 §3.3).
//!
//! The sheet does not draw the frost under it: the screen template calls `frost::draw` before it
//! and clears the presses under it (the cost is one reduce-and-blur of the whole canvas a frame
//! while a sheet is up).

use faraday_atoms::button::{self, ButtonModel};
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Glyph, Pressed, Shown, Ui};
use faraday_molecules::buttons::{self, ButtonsModel};
use faraday_tokens::dims::{
    BODY_M, BODY_S, BUTTON_H, DIALOG_MAX, DIALOG_MIN, DIALOG_PAD, GAP, GAP_SMALL, ICON,
    ICON_GLYPH_SMALL, LABEL_L, PAD, SHAPE_XL, SIDE_SHEET_PAD, TOUCH,
};
use faraday_tokens::elevation::ELEVATION_SHEET;
use faraday_tokens::palette::{Paint, Role};
use faraday_tokens::type_roles::sheet_head;

use crate::content::{self, Content};

/// A sheet's title: its icon and its words.
pub struct Head<'s> {
    pub icon: Glyph,
    pub tone: Paint,
    pub title: Shown<'s>,
}

/// A sheet on a small panel (04 §12.3): its head, what it holds, its buttons at its foot, the
/// press beside it that closes it, and the scroll of what is above the foot.
pub struct SheetModel<'s, A> {
    pub head: Option<Head<'s>>,
    pub body: Content<'s, A>,
    /// Kept at the sheet's foot, outside what scrolls.
    pub foot: ButtonsModel<'s, A>,
    /// The press taken on the page beside the sheet.
    pub beside: Option<A>,
    /// Keys the scroll of what is above the foot.
    pub scroll: Anchor,
}

/// A dialog (Material's basic dialog, `docs/DESIGN.md` §4.11): a sheet whose actions are at its
/// end on a wide panel, `want` wide where that is between [`DIALOG_MIN`] and [`DIALOG_MAX`].
pub struct DialogModel<'s, A> {
    pub sheet: SheetModel<'s, A>,
    pub want: f32,
}

/// Material's standard side sheet (Material 3, Side sheets; `docs/DESIGN.md` §4.16): docked
/// against the right edge, its container the surface with a divider along its leading edge and
/// no scrim, so what lies beside it stays as it is and takes presses. Drawn at rest: `Ui` has no
/// frame time to slide it in by.
pub struct SideSheetModel<'s, A> {
    pub head: Head<'s>,
    /// At the header's end, as a text button.
    pub close: ButtonModel<'s, A>,
    pub body: Content<'s, A>,
    pub foot: ButtonsModel<'s, A>,
    pub scroll: Anchor,
}

/// Draws the sheet on a small panel `w` by `h`: the panel's width less a margin, as tall as what
/// it holds (measured first, drawing nothing), centred.
pub fn draw<A: Pressed>(m: &SheetModel<'_, A>, ui: &mut Ui<'_, A>, (w, h): (f32, f32)) {
    scroll_sheet(m, ui, (GAP, w - 2.0 * GAP), (h, GAP, PAD));
}

/// A sheet `sw` wide at `sx`, as tall as what it holds and at most `h` less `margin` above and
/// below, centred, with `pad` inside: what is above its buttons scrolls when it is taller. The
/// sheet on a small panel; a long sheet on the desktop too.
pub fn scroll_sheet<A: Pressed>(
    m: &SheetModel<'_, A>,
    ui: &mut Ui<'_, A>,
    place: (f32, f32),
    frame: (f32, f32, f32),
) {
    framed(m, ui, place, frame, (false, Role::SurfaceContainerLow));
}

/// The least room, `(width, height)` in units, a dialog leaves inside its padding on a display `w`
/// by `h`: [`DIALOG_MIN`] wide on a wide panel, the panel's width less its margins on a small one,
/// and four touches tall. A taller or wider dialog leaves more; a count that must hold in any
/// dialog is worked out from this.
pub fn dialog_room_least(w: f32, h: f32) -> (f32, f32) {
    let wide = DIALOG_MIN.min(w - 2.0 * PAD) - 2.0 * DIALOG_PAD;
    let small = w - 2.0 * GAP - 2.0 * PAD;
    let tall = (4.0 * TOUCH).min(h - 2.0 * PAD - 2.0 * DIALOG_PAD);
    (wide.min(small).max(0.0), tall.max(0.0))
}

/// Draws the dialog centred in a frame `w` by `h` whose left edge is at `x0`. On a small panel
/// (`Ui::compact`) it is the panel's width less a margin and its actions share its width;
/// elsewhere it is [`DIALOG_MIN`] to [`DIALOG_MAX`] wide with [`DIALOG_PAD`] inside and its
/// actions at its end as wide as their labels.
pub fn dialog<A: Pressed>(m: &DialogModel<'_, A>, ui: &mut Ui<'_, A>, (x0, w, h): (f32, f32, f32)) {
    let (place, frame, end) = if ui.compact {
        ((x0 + GAP, w - 2.0 * GAP), (h, GAP, PAD), false)
    } else {
        let dw = m.want.clamp(DIALOG_MIN, DIALOG_MAX).min(w - 2.0 * PAD);
        ((x0 + (w - dw) / 2.0, dw), (h, PAD, DIALOG_PAD), true)
    };
    framed(
        &m.sheet,
        ui,
        place,
        frame,
        (end, Role::SurfaceContainerHigh),
    );
}

/// The head and the body, from y, `iw` wide at `ix`. Returns the height.
fn body_of<A: Pressed>(
    head: Option<&Head<'_>>,
    body: &Content<'_, A>,
    ui: &mut Ui<'_, A>,
    (ix, y, iw): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    if let Some(head) = head {
        cy += sheet_head_at(ui, (ix, cy, iw), head);
    }
    cy += content::draw(body, ui, (ix, cy, iw));
    cy - y
}

/// The sheet, its buttons at its end when `end`, else sharing its width; filled in the container
/// role `fill` (a sheet's is surface-container-low, a dialog's surface-container-high:
/// `docs/THEMES.md` §6).
fn framed<A: Pressed>(
    m: &SheetModel<'_, A>,
    ui: &mut Ui<'_, A>,
    place: (f32, f32),
    frame: (f32, f32, f32),
    how: (bool, Role),
) {
    let body = |ui: &mut Ui<'_, A>, at| body_of(m.head.as_ref(), &m.body, ui, at);
    let held = Held {
        foot: &m.foot,
        beside: m.beside.as_ref(),
        scroll: m.scroll,
    };
    framed_by(&body, &held, ui, place, frame, how);
}

/// What draws a sheet's body from (x, y, w): its height.
pub(crate) type Body<'a, A> = dyn Fn(&mut Ui<'_, A>, (f32, f32, f32)) -> f32 + 'a;

/// What a sheet keeps outside its body: the buttons at its foot, the press beside it and the
/// scroll's key.
pub(crate) struct Held<'a, 's, A> {
    pub foot: &'a ButtonsModel<'s, A>,
    pub beside: Option<&'a A>,
    pub scroll: Anchor,
}

/// [`framed`] for a body that is drawn by `body` from (x, y, w) and returns its height: the
/// compact menu's lines are one.
pub(crate) fn framed_by<A: Pressed>(
    body: &Body<'_, A>,
    held: &Held<'_, '_, A>,
    ui: &mut Ui<'_, A>,
    (sx, sw): (f32, f32),
    (h, margin, pad): (f32, f32, f32),
    (end, fill): (bool, Role),
) {
    let (ix, iw) = (sx + pad, sw - 2.0 * pad);
    // Measured first, drawing nothing: the body's height and the buttons'.
    let ch = ui.measured(|ui| body(ui, (ix, 0.0, iw)));
    let has_foot = !held.foot.items.is_empty();
    let bh = buttons::height_as(held.foot, end, ui, iw);
    // The buttons' row, a pad above it and one below, is outside what scrolls: a control of the
    // body cut at the scroll's edge is a pad from the buttons, and leaves each button its whole
    // touch target. What scrolls is the body with a pad above it, and below it where there are
    // no buttons; unscrolled, the sheet is laid out as if the row were simply under the body.
    let foot = if has_foot { bh + 2.0 * pad } else { 0.0 };
    let content = ch + if has_foot { pad } else { 2.0 * pad };
    let sh = (content + foot).min(h - 2.0 * margin);
    let sy = ((h - sh) / 2.0).max(margin);
    if let Some(a) = held.beside {
        ui.press_plain((sx, sy, sw, sh), faraday_atoms::Plain::Beside, a.clone());
    }
    // Material's dialog and sheet: extra-large corners.
    ui.shadow(sx, sy, sw, sh, SHAPE_XL, ELEVATION_SHEET);
    ui.fill(sx, sy, sw, sh, SHAPE_XL, fill);
    // At High contrast, where it casts no shadow, its outline sets it apart
    // (`docs/THEMES.md` §9); elsewhere its shadow and scrim do.
    if ui.theme.is_high_contrast() {
        ui.stroke(sx, sy, sw, sh, SHAPE_XL, Role::Outline);
    }
    // Taller than the panel: what is above the buttons' row scrolls. Unscrolled, nothing is cut,
    // and the body's own controls keep the room down to the buttons for their touch targets.
    let view = sh - foot;
    let max = (content - view).max(0.0);
    let room = if max > 0.0 || !has_foot {
        view
    } else {
        view + pad
    };
    let clip = ui.rect(sx, sy, sw, room);
    // Its edges fade into its own container (`docs/THEMES.md` §7).
    ui.region(Region::new(held.scroll, clip).on(fill), |ui| {
        body(ui, (ix, sy + pad, iw));
        max
    });
    if has_foot {
        if max > 0.0 {
            ui.fill(
                sx + 1.0,
                sy + view,
                sw - 2.0,
                1.0,
                0.0,
                Role::OutlineVariant,
            );
        }
        buttons::draw_as(held.foot, end, ui, (ix, sy + view + pad, iw));
    }
}

/// Draws the side sheet docked against `right`, from `top` down `h`, `w` wide. Its header holds
/// the head and, at its end, the close button; what the body holds is under it, scrolling where
/// it is taller than the sheet, and the buttons are pinned at its foot.
pub fn side_sheet<A: Pressed>(
    m: &SideSheetModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (right, top, w, h): (f32, f32, f32, f32),
) {
    let pad = SIDE_SHEET_PAD;
    let sx = right - w;
    let (ix, iw) = (sx + pad, w - 2.0 * pad);
    ui.fill(sx, top, w, h, 0.0, Role::Surface);
    ui.fill(sx, top, 1.0, h, 0.0, Role::OutlineVariant);
    // The header: the title, and Close at its end in a touch target.
    let close_w = ui.measure_as(LABEL_L, m.close.label.as_str()) + 2.0 * pad;
    let th = sheet_head_at(ui, (ix, top + pad, iw - close_w - GAP), &m.head);
    let touch = ui.touch;
    button::draw(
        &m.close,
        ui,
        (
            sx + w - pad / 2.0 - close_w,
            top + pad / 2.0 + (touch - BUTTON_H) / 2.0,
            Some(close_w),
            BUTTON_H,
        ),
    );
    let body_top = top + pad + th.max(touch - pad / 2.0);
    side_body(m, ui, (sx, body_top, w, top + h - body_top));
}

/// A side sheet's body in the box (x, y, w, h): what the content draws, scrolled where it is
/// taller than the box, and the foot's buttons under a divider while it scrolls.
fn side_body<A: Pressed>(
    m: &SideSheetModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (sx, sy, sw, sh): (f32, f32, f32, f32),
) {
    let pad = SIDE_SHEET_PAD;
    let (ix, iw) = (sx + pad, sw - 2.0 * pad);
    let ch = ui.measured(|ui| content::draw(&m.body, ui, (ix, 0.0, iw)));
    let has_foot = !m.foot.items.is_empty();
    let bh = buttons::height(&m.foot, ui, iw);
    let foot = if has_foot { bh + 2.0 * pad } else { 0.0 };
    let view = (sh - foot).max(0.0);
    let max = (ch + pad - view).max(0.0);
    let clip = ui.rect(sx + 1.0, sy, sw - 1.0, view);
    ui.region(Region::new(m.scroll, clip), |ui| {
        content::draw(&m.body, ui, (ix, sy, iw));
        max
    });
    if has_foot {
        if max > 0.0 {
            ui.fill(
                sx + 1.0,
                sy + view,
                sw - 1.0,
                1.0,
                0.0,
                Role::OutlineVariant,
            );
        }
        buttons::draw(&m.foot, ui, (ix, sy + view + pad, iw));
    }
}

/// A sheet's title: its icon, and the words wrapped beside it in Material's headline small, title
/// large on the small panel. Returns its height.
pub fn sheet_head_at<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    head: &Head<'_>,
) -> f32 {
    let role = sheet_head(ui.compact);
    let line = ui.line_as(role);
    let iy = y + (ui.line(role.size, role.face.into()) - ICON) / 2.0;
    ui.icon(
        x - GAP_SMALL,
        iy,
        ICON,
        head.icon,
        ICON_GLYPH_SMALL,
        head.tone,
    );
    let at = (x + ICON, y, w - ICON);
    let th = ui.showing(&head.title, |ui| {
        ui.wrap_as(at, role, Role::OnSurface, head.title.as_str())
    });
    th.max(line) + GAP
}

/// A label over its value, each wrapped, a rule under. Returns its height.
pub fn kv<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (key, value): (Shown<'_>, Shown<'_>),
    tone: Paint,
) -> f32 {
    ui.showing(&key, |ui| {
        ui.showing(&value, |ui| {
            let vy =
                ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, key.as_str()) + GAP_SMALL;
            let vh = ui.wrap_as((x, y + vy, w), BODY_M, tone, value.as_str());
            ui.rule(x, y + vy + vh + GAP_SMALL, w, Role::OutlineVariant);
            vy + vh + GAP_SMALL + GAP
        })
    })
}

/// The dialog that says what a lock does: a title, a line, rows of what happens to what, and its
/// actions. The rows are facts in the content's own tone.
pub fn kv_sheet<'s, A: Pressed>(
    head: Head<'s>,
    sub: Option<Shown<'s>>,
    rows: Vec<(Shown<'s>, Shown<'s>)>,
    foot: ButtonsModel<'s, A>,
    scroll: Anchor,
) -> DialogModel<'s, A> {
    use crate::content::Block;
    let mut blocks = Vec::new();
    if let Some(sub) = sub.filter(|s| !s.as_str().is_empty()) {
        blocks.push(Block::Para(faraday_molecules::flow_text::FlowText {
            runs: vec![(sub, false)],
            role: BODY_M,
        }));
    }
    for (label, value) in rows {
        blocks.push(Block::Fact {
            label,
            value,
            mono: false,
        });
    }
    DialogModel {
        sheet: SheetModel {
            head: Some(head),
            body: Content(blocks),
            foot,
            beside: None,
            scroll,
        },
        want: DIALOG_MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::button::Style;
    use faraday_atoms::motion::{ComponentId, Motion, MotionStore};
    use faraday_atoms::ui::testing::{P, canvas};
    use faraday_atoms::{Offered, Pointer, Theme};

    use crate::content::Block;

    /// The sheet's scroll, keyed by this anchor.
    const SCROLL: Anchor = Anchor {
        scope: "dialog",
        n: 1,
    };

    fn button(label: &'static str, style: Style, press: u8) -> ButtonModel<'static, P> {
        ButtonModel {
            label: Shown::plain(label),
            style,
            offered: Offered::Yes(P(press)),
            chosen: false,
            icon: None,
        }
    }

    /// A dialog whose body is `lines` headings and then a button, with Cancel at its foot.
    fn dialog_of(lines: usize) -> DialogModel<'static, P> {
        let mut blocks: Vec<Block<'static, P>> = (0..lines)
            .map(|_| Block::Heading(Shown::plain("A line of what the dialog says")))
            .collect();
        blocks.push(Block::Buttons(vec![button("Hold", Style::Outlined, 2)]));
        DialogModel {
            sheet: SheetModel {
                head: Some(Head {
                    icon: Glyph::Stick,
                    tone: Role::Warning.paint(),
                    title: Shown::plain("Save it?"),
                }),
                body: Content(blocks),
                foot: ButtonsModel::row(vec![button("Cancel", Style::Text, 1)]),
                beside: None,
                scroll: SCROLL,
            },
            want: 360.0,
        }
    }

    /// Invariant: A dialog's action takes a press anywhere in its whole touch target, at any length of the dialog's body and however far the body is scrolled.
    /// Regression: A control of the body cut at the edge of what scrolls takes part of the action's target.
    /// Owner: kit
    #[test]
    fn a_dialogs_action_keeps_its_whole_touch_target_however_its_body_is_scrolled() {
        let mut short = Vec::new();
        for compact in [true, false] {
            for lines in (0..14).step_by(2) {
                for offset in (0..240).step_by(48) {
                    let mut c = canvas(160);
                    let mut store = MotionStore::new();
                    let page = Anchor { scope: "", n: 0 };
                    store.set(
                        ComponentId(page, SCROLL),
                        Motion::Region {
                            offset: offset as f32,
                            velocity: 0.0,
                            stretch: 0.0,
                            extent: 0.0,
                            bar_shown_ms: 0,
                        },
                    );
                    let pointer = Pointer {
                        at: None,
                        down: false,
                        focused: None,
                        hovered: None,
                        pressed: None,
                    };
                    let mut ui =
                        Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, false);
                    ui.compact = compact;
                    dialog(&dialog_of(lines), &mut ui, (0.0, 360.0, 320.0));
                    let floor = ui.px(ui.touch);
                    for hit in ui.finish().hits.iter().filter(|h| h.press == P(1)) {
                        if hit.target.w < floor || hit.target.h < floor {
                            short.push(format!(
                                "compact {compact}, {lines} lines, scrolled {offset}: Cancel's target {} x {}",
                                hit.target.w, hit.target.h
                            ));
                        }
                    }
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }
}
