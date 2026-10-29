//! What codex32 error correction draws (`docs/CODEX32.md` §8.4), from the
//! [`Correction`] a form holds: the tick's row, the panel that stands in
//! place of the form's field while a proposal or a refusal is up, and the
//! line beside a key's fingerprint. Built from DESIGN §4's components: a
//! checkbox (§4.2), section labels, lines in their tone (§4.11), a row of
//! buttons (§4.13), and the string's boxes in groups of four (§4.5).
//!
//! Each function draws in design units at (x, y), `width` wide, and
//! returns the height it took. They are generic over what a press does,
//! so the form that holds the correction names its own action for a
//! [`Press`] (`to`). Drawing reads the state and works nothing out.

use faraday_draw::tokens::{BODY_L, BODY_M, LABEL_S, TITLE_S};
use faraday_draw::tokens::{
    CORRECT_BOX, CORRECT_BOX_GAP, CORRECT_BOX_RADIUS, CORRECT_GAP, CORRECT_GROUP,
    CORRECT_GROUP_GAP, CORRECT_ICON, CORRECT_ICON_SIZE, CORRECT_NUMBER_ROW, CORRECT_PAIR_ROW,
    CORRECT_ROW_GAP, CORRECT_TICK, CORRECT_TICK_LABEL, CORRECT_TICK_ROW,
};
use faraday_draw::widgets::Icon;
use faraday_ui::pal::*;
use faraday_ui::sheet::buttons;
use faraday_ui::{Style, Ui};

use super::{BoxShown, Correction, Mark, Press, Step};
use crate::strings::{EN, fill};

/// The tick's row: **Suggest corrections**, ticked or not. A press
/// anywhere on the row is [`Press::Tick`].
pub fn tick_row<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    width: f32,
    c: &Correction,
    to: impl Fn(Press) -> A,
) -> f32 {
    ui.checkbox(x, y + (CORRECT_TICK_ROW - CORRECT_TICK) / 2.0, c.on(), true);
    ui.text_mid_as(
        (x + CORRECT_TICK_LABEL, y, CORRECT_TICK_ROW),
        BODY_M,
        Role::OnSurface,
        EN.codex32_correct_suggest,
    );
    ui.press_box(x, y, width, CORRECT_TICK_ROW, to(Press::Tick));
    CORRECT_TICK_ROW
}

/// The line beside a key's fingerprint: "Share entry corrected", or
/// "Share repairs unchecked" in the caution tone.
pub fn fingerprint_line<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    width: f32,
    mark: Mark,
) -> f32 {
    let (icon, tone) = match mark {
        Mark::Corrected => (Icon::Info, Role::OnSurfaceVariant.paint()),
        Mark::Repaired => (Icon::Warning, Role::Warning.paint()),
    };
    line(ui, x, y, width, icon, tone, mark.fingerprint_line())
}

/// A line with its icon in its tone (§4.11), wrapped. Returns its
/// height.
pub fn line<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    width: f32,
    icon: Icon,
    tone: crate::ui::Paint,
    text: &str,
) -> f32 {
    ui.icon(x, y, CORRECT_ICON, icon, CORRECT_ICON_SIZE, tone);
    let h = ui.wrap_as(
        (
            x + CORRECT_ICON + CORRECT_BOX_GAP,
            y,
            width - CORRECT_ICON - CORRECT_BOX_GAP,
        ),
        BODY_M,
        tone,
        text,
    );
    h.max(CORRECT_ICON)
}

/// The panel in place of the form's field while the correction's screen
/// is up; nothing while it is not.
pub fn panel<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    width: f32,
    c: &Correction,
    to: impl Fn(Press) -> A,
) -> f32 {
    let check = (
        EN.codex32_correct_check_boxes,
        Style::Ghost,
        to(Press::CheckBoxes),
    );
    if let Some(text) = c.refusal_line() {
        let h = line(ui, x, y, width, Icon::Error, Role::Error.paint(), &text) + CORRECT_GAP;
        return h + buttons(ui, x, y + h, width, &[(check.0, Style::Secondary, check.2)]);
    }
    match c.step() {
        None => 0.0,
        Some(Step::Caution) => caution(
            ui,
            (x, y, width),
            EN.codex32_correct_assumes,
            (EN.codex32_correct_continue, to(Press::Continue)),
            check,
        ),
        Some(Step::FromChecksum) => caution(
            ui,
            (x, y, width),
            EN.codex32_correct_from_checksum,
            (EN.codex32_correct_take, to(Press::Take)),
            check,
        ),
        Some(step @ (Step::Pairs | Step::Repairs)) => {
            proposal(ui, (x, y, width), c, step == Step::Pairs, &to)
        }
        Some(Step::Retype) => retype(ui, (x, y, width), c, check),
    }
}

/// A caution in its tone, and the way on under it beside **Check the
/// boxes**.
fn caution<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, width): (f32, f32, f32),
    text: &str,
    (label, on): (&str, A),
    check: (&str, Style, A),
) -> f32 {
    let h = line(ui, x, y, width, Icon::Warning, Role::Warning.paint(), text) + CORRECT_GAP;
    h + buttons(ui, x, y + h, width, &[(label, Style::Primary, on), check])
}

/// "Possible correction" (`pairs`) or the repairs to write on the paper:
/// the page of pairs, and the ways on, dimmed until every page was seen.
fn proposal<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, width): (f32, f32, f32),
    c: &Correction,
    pairs_step: bool,
    to: &impl Fn(Press) -> A,
) -> f32 {
    let title = if pairs_step {
        EN.codex32_correct_possible
    } else {
        EN.codex32_correct_repairs
    };
    let mut h = heading(ui, x, y, width, title);
    if c.assumes_the_rest() && pairs_step {
        h += line(
            ui,
            x,
            y + h,
            width,
            Icon::Warning,
            Role::Warning.paint(),
            EN.codex32_correct_assumes,
        );
        h += CORRECT_GAP;
    }
    h += pairs(ui, x, y + h, width, c, to);
    h += CORRECT_GAP;
    let seen = c.all_seen();
    let gate = |s: Style| if seen { s } else { Style::Disabled };
    let mut ways = Vec::with_capacity(4);
    if pairs_step {
        ways.push((
            EN.codex32_correct_mistyped,
            gate(Style::Primary),
            to(Press::Mistyped),
        ));
        ways.push((
            EN.codex32_correct_damaged,
            gate(Style::Secondary),
            to(Press::Damaged),
        ));
        if c.offers_unreadable() {
            ways.push((
                EN.codex32_correct_unreadable,
                gate(Style::Secondary),
                to(Press::Unreadable),
            ));
        }
    } else {
        ways.push((
            EN.codex32_correct_continue,
            gate(Style::Primary),
            to(Press::Continue),
        ));
    }
    ways.push((
        EN.codex32_correct_check_boxes,
        Style::Ghost,
        to(Press::CheckBoxes),
    ));
    h + buttons(ui, x, y + h, width, &ways)
}

/// The re-entry: the boxes, the count typed, the line where the last
/// re-entry differed, and **Check the boxes**.
fn retype<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, width): (f32, f32, f32),
    c: &Correction,
    check: (&str, Style, A),
) -> f32 {
    let mut h = heading(ui, x, y, width, EN.codex32_correct_type_again);
    h += boxes(ui, x, y + h, width, c);
    h += CORRECT_GAP;
    let (typed, total) = c.retyped();
    h += ui.wrap_as(
        (x, y + h, width),
        BODY_M,
        Role::OnSurfaceVariant,
        &fill(
            EN.codex32_correct_typed_of,
            &[&typed.to_string(), &total.to_string()],
        ),
    );
    if c.differs() {
        h += CORRECT_GAP;
        h += line(
            ui,
            x,
            y + h,
            width,
            Icon::Error,
            Role::Error.paint(),
            EN.codex32_correct_differs,
        );
    }
    h += CORRECT_GAP;
    h + buttons(ui, x, y + h, width, &[(check.0, Style::Secondary, check.2)])
}

/// A section label (DESIGN §4.12), wrapped where the panel is narrow,
/// and the gap under it.
fn heading<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, width: f32, title: &str) -> f32 {
    ui.wrap_as((x, y, width), TITLE_S, Role::OnSurfaceVariant, title) + CORRECT_GAP
}

/// The page of pairs on show, "Box 17 · q → p", and under them the page
/// and **Previous · Next** where there is more than one.
fn pairs<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    width: f32,
    c: &Correction,
    to: &impl Fn(Press) -> A,
) -> f32 {
    let mut h = 0.0;
    for e in c.page_edits() {
        let mut was = [0u8; 4];
        let mut now = [0u8; 4];
        let pair = fill(
            EN.codex32_correct_pair,
            &[
                &e.position().to_string(),
                char::from(e.was()).encode_utf8(&mut was),
                char::from(e.now()).encode_utf8(&mut now),
            ],
        );
        ui.text_mid_as(
            (x, y + h, CORRECT_PAIR_ROW),
            BODY_L.mono(),
            Role::OnSurface,
            &pair,
        );
        h += CORRECT_PAIR_ROW;
    }
    let Some((page, pages)) = c.page() else {
        return h;
    };
    if pages > 1 {
        h += CORRECT_ROW_GAP;
        ui.text_mid_as(
            (x, y + h, CORRECT_PAIR_ROW),
            BODY_M,
            Role::OnSurfaceVariant,
            &fill(
                EN.codex32_correct_page,
                &[&(page + 1).to_string(), &pages.to_string()],
            ),
        );
        h += CORRECT_PAIR_ROW;
        let first = page == 0;
        let last = page + 1 >= pages;
        h += buttons(
            ui,
            x,
            y + h,
            width,
            &[
                (
                    EN.codex32_correct_previous,
                    if first {
                        Style::Disabled
                    } else {
                        Style::Secondary
                    },
                    to(Press::Previous),
                ),
                (
                    EN.codex32_correct_next,
                    Style::Primary.when(!last),
                    to(Press::Next),
                ),
            ],
        );
    }
    h
}

/// The string's boxes in groups of four, wrapped at the width: each box
/// not changed locked with its character, each changed one open (the one
/// typed into next edged in the accent) or as typed again, with its
/// number under it. On a narrow panel, only the groups with a box to
/// type again.
fn boxes<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, width: f32, c: &Correction) -> f32 {
    let (ink, muted) = (Role::OnSurface, Role::OnSurfaceVariant);
    let group = CORRECT_GROUP;
    let per_row = ((width + CORRECT_GROUP_GAP) / (group + CORRECT_GROUP_GAP))
        .floor()
        .max(1.0) as usize;
    let row_h = CORRECT_BOX + CORRECT_NUMBER_ROW + CORRECT_ROW_GAP;
    let len = c.len();
    // The whole string where it fits in a few rows; on a narrow panel only
    // the groups with a box to type again, so they stay above the
    // keyboard.
    let all = len.div_ceil(4) <= per_row * WHOLE_ROWS;
    let drawn: Vec<usize> = (0..len.div_ceil(4))
        .filter(|&g| {
            all || (g * 4 + 1..=(g * 4 + 4).min(len))
                .any(|n| !matches!(c.box_at(n), Some(BoxShown::Locked(_))))
        })
        .collect();
    for n in 1..=len {
        let Some(shown) = c.box_at(n) else {
            break;
        };
        let i = n - 1;
        let k = i % 4;
        let Some(slot) = drawn.iter().position(|&g| g == i / 4) else {
            continue;
        };
        let (row, col) = (slot / per_row, slot % per_row);
        let bx = x
            + col as f32 * (group + CORRECT_GROUP_GAP)
            + k as f32 * (CORRECT_BOX + CORRECT_BOX_GAP);
        let by = y + row as f32 * row_h;
        let side = CORRECT_BOX;
        ui.fill(
            bx,
            by,
            side,
            side,
            CORRECT_BOX_RADIUS,
            Role::SurfaceContainerHighest,
        );
        let ch = match shown {
            BoxShown::Locked(ch) => Some((ch, muted)),
            BoxShown::Open { next } => {
                ui.stroke(
                    bx,
                    by,
                    CORRECT_BOX,
                    CORRECT_BOX,
                    CORRECT_BOX_RADIUS,
                    if next {
                        Role::Primary
                    } else {
                        Role::OutlineVariant
                    },
                );
                None
            }
            BoxShown::Typed(ch) => {
                ui.stroke(
                    bx,
                    by,
                    CORRECT_BOX,
                    CORRECT_BOX,
                    CORRECT_BOX_RADIUS,
                    Role::Primary,
                );
                Some((ch, ink))
            }
        };
        if let Some((ch, tone)) = ch {
            let mut buf = [0u8; 4];
            let s = char::from(ch).encode_utf8(&mut buf);
            let tw = ui.measure_as(BODY_L.mono(), s);
            ui.text_mid_as(
                (bx + (CORRECT_BOX - tw) / 2.0, by, CORRECT_BOX),
                BODY_L.mono(),
                tone,
                s,
            );
            zeroize::Zeroize::zeroize(&mut buf);
        }
        if !matches!(shown, BoxShown::Locked(_)) {
            let num = n.to_string();
            let tw = ui.measure_as(LABEL_S.regular(), &num);
            ui.text_mid_as(
                (
                    bx + (CORRECT_BOX - tw) / 2.0,
                    by + CORRECT_BOX,
                    CORRECT_NUMBER_ROW,
                ),
                LABEL_S.regular(),
                muted,
                &num,
            );
        }
    }
    let rows = drawn.len().div_ceil(per_row).max(1);
    rows as f32 * row_h - CORRECT_ROW_GAP
}

/// The most rows of boxes the re-entry draws the whole string in; past
/// it, only the groups with a box to type again.
const WHOLE_ROWS: usize = 3;

/// A whole string in boxes, groups of four wrapped at the width, as the
/// re-entry draws its locked boxes; the boxes in `marked`, counted from
/// one, edged in the error tone with their number under them. For a
/// string shown beside another it differs from (`docs/CODEX32.md`
/// §8.5). Returns the height taken.
pub fn string_boxes<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    width: f32,
    text: &str,
    marked: &[usize],
) -> f32 {
    let per_row = ((width + CORRECT_GROUP_GAP) / (CORRECT_GROUP + CORRECT_GROUP_GAP))
        .floor()
        .max(1.0) as usize;
    let row_h = CORRECT_BOX + CORRECT_NUMBER_ROW + CORRECT_ROW_GAP;
    for (i, ch) in text.bytes().enumerate() {
        let (g, k) = (i / 4, i % 4);
        let (row, col) = (g / per_row, g % per_row);
        let bx = x
            + col as f32 * (CORRECT_GROUP + CORRECT_GROUP_GAP)
            + k as f32 * (CORRECT_BOX + CORRECT_BOX_GAP);
        let by = y + row as f32 * row_h;
        let differs = marked.contains(&(i + 1));
        ui.fill(
            bx,
            by,
            CORRECT_BOX,
            CORRECT_BOX,
            CORRECT_BOX_RADIUS,
            Role::SurfaceContainerHighest,
        );
        if differs {
            ui.stroke(
                bx,
                by,
                CORRECT_BOX,
                CORRECT_BOX,
                CORRECT_BOX_RADIUS,
                Role::Error,
            );
        }
        let mut buf = [0u8; 4];
        let s = char::from(ch).encode_utf8(&mut buf);
        let tw = ui.measure_as(BODY_L.mono(), s);
        ui.text_mid_as(
            (bx + (CORRECT_BOX - tw) / 2.0, by, CORRECT_BOX),
            BODY_L.mono(),
            if differs {
                Role::Error
            } else {
                Role::OnSurface
            },
            s,
        );
        zeroize::Zeroize::zeroize(&mut buf);
        if differs {
            let num = (i + 1).to_string();
            let tw = ui.measure_as(LABEL_S.regular(), &num);
            ui.text_mid_as(
                (
                    bx + (CORRECT_BOX - tw) / 2.0,
                    by + CORRECT_BOX,
                    CORRECT_NUMBER_ROW,
                ),
                LABEL_S.regular(),
                Role::Error,
                &num,
            );
        }
    }
    let rows = text.len().div_ceil(4).div_ceil(per_row).max(1);
    rows as f32 * row_h - CORRECT_ROW_GAP
}

/// What the snapshot tour draws of the correction alone, before a form
/// holds it: the tick's row, the panel, and a key's line for `mark`, on
/// the page's background, `width` × `height` pixels at `dpi`. The frame's
/// pixels, RGBA.
#[cfg(feature = "testkit")]
pub fn preview(c: &Correction, mark: Option<Mark>, width: u16, height: u16, dpi: u16) -> Vec<u8> {
    use faraday_draw::tokens::PAD;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
    let d = DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    };
    let mut canvas = faraday_draw::Canvas::new(&d);
    let f = canvas.scale().factor();
    let mut hits: Vec<(faraday_draw::Rect, Press)> = Vec::new();
    let (w, h) = (f32::from(width) / f, f32::from(height) / f);
    {
        let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
        ui.fill(0.0, 0.0, w, h, 0.0, Role::Surface);
        let width = w - 2.0 * PAD;
        let mut y = PAD;
        y += tick_row(&mut ui, PAD, y, width, c, |p| p) + CORRECT_GAP;
        y += panel(&mut ui, PAD, y, width, c, |p| p) + CORRECT_GAP;
        if let Some(m) = mark {
            fingerprint_line(&mut ui, PAD, y, width, m);
        }
    }
    canvas.frame().rgba.to_vec()
}
