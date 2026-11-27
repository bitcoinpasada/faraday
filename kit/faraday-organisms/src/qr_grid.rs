//! The QR grid organism (09 §3.3): a QR to copy by hand, square by square. **Blocks** and
//! **Rows**; the whole code as a ruled grid, heavier every five squares from the top left, its
//! blocks named at its edges; the block or row marked tinted and edged in the accent, each block
//! (each row) a press whose hover covers just it; in Blocks the marked block large beside the
//! grid (under it on a narrow column) with a square of its neighbours faded round it; the squares
//! the blank sheet prints grey, and a key saying so; the marked block's or row's name and
//! **Previous** and **Next**. Drawing only: the code was made when it was shown.

use faraday_atoms::button::{ButtonModel, Style};
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_codec::qr::QrMatrix;
use faraday_molecules::buttons;
use faraday_tokens::dims::{
    BODY_S, C32_GRID_LABEL, C32_GRID_MAX, C32_ZOOM_MAX, C32_ZOOM_MIN_CELL, GAP, GAP_SMALL, PAD,
    TITLE_M,
};
use faraday_tokens::palette::{Color, Role};

/// The paper's colours on a QR copied by hand, whatever the theme: the page, a square to draw, a
/// square the blank sheet prints, the rule between squares and the heavier one between blocks.
const PAPER: Color = Color::WHITE;
const INK: Color = Color::BLACK;
const PRINTED_INK: Color = Color::rgb(0x8a, 0x94, 0x9e);
const RULE: Color = Color::rgb(0xc8, 0xd0, 0xd6);
const BLOCK_RULE: Color = Color::rgb(0x6f, 0x82, 0x91);

/// The squares along a side of a block of a QR copied by hand: blocks counted from the top left,
/// as the blank sheets' heavier lines are, so the last block of a row or column is narrower where
/// the side is not a multiple of five (25 x 25: five whole blocks; 29 x 29: five and one of
/// four; 21 x 21: four and one of one).
pub const COPY_BLOCK: usize = 5;

/// Blocks along a side of a QR of side `n`.
pub fn copy_blocks(n: usize) -> usize {
    n.div_ceil(COPY_BLOCK)
}

/// The squares of block `b` of a QR of side `n`, in reading order: its first column and row, and
/// its width and height in squares.
pub fn copy_block_rect(b: usize, n: usize) -> (usize, usize, usize, usize) {
    let per = copy_blocks(n).max(1);
    let (c0, r0) = ((b % per) * COPY_BLOCK, (b / per) * COPY_BLOCK);
    (
        c0,
        r0,
        COPY_BLOCK.min(n.saturating_sub(c0)),
        COPY_BLOCK.min(n.saturating_sub(r0)),
    )
}

/// Block `b` of a code of side `n` as a grid names it: "A1" where the grid is lettered, else its
/// first row's and column's index, "5 · 10".
pub fn copy_block_name(b: usize, n: usize, lettered: bool) -> String {
    let per = copy_blocks(n).max(1);
    if lettered {
        let row = char::from(b'A' + u8::try_from(b / per).unwrap_or(0));
        return format!("{row}{}", b % per + 1);
    }
    let (c0, r0, _, _) = copy_block_rect(b, n);
    format!("{r0} · {c0}")
}

/// What a QR copied by hand shows and does: the SeedQR on the copy item, a codex32 string's
/// Codex32QR on its item. The code is a secret's: it is read module by module as it is drawn.
pub struct QrGridModel<'s, A> {
    /// The code.
    pub code: &'s QrMatrix,
    /// Whether square (x, y) of a code of side n is printed on the blank sheet: drawn grey.
    pub printed: fn(usize, usize, usize) -> bool,
    /// The blocks named as the Codex32QR sheet names them, rows A to F and columns 1 to 6; else
    /// by the index of their first square, as the SeedQR sheet numbers its lines, 0, 5, 10.
    pub lettered: bool,
    /// Marked row by row rather than block by block.
    pub rows: bool,
    /// The block, or the row, marked.
    pub pin: usize,
    /// The press that marks block or row `i`.
    pub pick: fn(usize) -> A,
    /// The press that chooses rows (`true`) or blocks.
    pub mode: fn(bool) -> A,
    /// The words of the two modes, of the key to the squares, and of the two steps.
    pub words: GridWords<'s>,
    /// The name of the marked block or row as the line under the grid says it ("Block A1").
    pub name: Shown<'s>,
}

/// The words the grid carries.
pub struct GridWords<'s> {
    pub blocks: Shown<'s>,
    pub rows: Shown<'s>,
    /// The key for the grey squares.
    pub printed: Shown<'s>,
    /// The key for the squares to draw.
    pub draw: Shown<'s>,
    pub previous: Shown<'s>,
    pub next: Shown<'s>,
}

fn button<'s, A>(label: Shown<'s>, style: Style, chosen: bool, press: A) -> ButtonModel<'s, A> {
    ButtonModel {
        label,
        style,
        offered: Offered::Yes(press),
        chosen,
        icon: None,
    }
}

/// Draws the grid from (x, y), `w` wide. Returns the height it takes.
pub fn draw<A: Pressed>(
    m: &QrGridModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let n = m.code.size();
    let mut cy = y;
    let modes = vec![
        button(
            m.words.blocks.clone(),
            Style::Outlined,
            !m.rows,
            (m.mode)(false),
        ),
        button(
            m.words.rows.clone(),
            Style::Outlined,
            m.rows,
            (m.mode)(true),
        ),
    ];
    cy += buttons::draw_items(&modes, &[], false, ui, (x, cy, w)) + GAP;
    let last = if m.rows {
        n - 1
    } else {
        copy_blocks(n).pow(2) - 1
    };
    let pin = m.pin.min(last);
    let lab = C32_GRID_LABEL;
    let ring = (COPY_BLOCK + 2) as f32;
    let zoom_min = lab + ring * C32_ZOOM_MIN_CELL;
    // The grid's largest side: the token, or more where a block of `COPY_BLOCK` cells would be
    // under the touch floor in it.
    let grid_max = C32_GRID_MAX.max((ui.touch / COPY_BLOCK as f32).ceil() * n as f32);
    let beside = !m.rows && w >= lab + grid_max + GAP + zoom_min;
    // Beside: the whole grid at its largest, the block large in the rest.
    let whole_w = if beside { lab + grid_max } else { w };
    let cell = ((whole_w - lab).min(grid_max) / n as f32).floor();
    let whole_h = whole(m, ui, (x, cy), cell, pin);
    let mut below = cy + whole_h + GAP;
    if !m.rows {
        let (zx, zy, zw) = if beside {
            let zx = x + whole_h + GAP;
            (zx, cy, x + w - zx)
        } else {
            (x, below, w)
        };
        let zcell = ((zw - lab).min(C32_ZOOM_MAX) / ring).floor();
        let zh = zoomed(m, ui, (zx, zy), zcell, pin);
        below = if beside {
            cy + whole_h.max(zh)
        } else {
            zy + zh
        } + GAP;
    }
    cy = below;
    for (ink, line) in [(PRINTED_INK, &m.words.printed), (INK, &m.words.draw)] {
        ui.fill(x, cy + GAP_SMALL, GAP, GAP, 0.0, ink);
        ui.text_as(
            x + 2.0 * GAP,
            cy,
            BODY_S,
            Role::OnSurfaceVariant,
            line.as_str(),
        );
        cy += PAD + GAP_SMALL;
    }
    cy += ui.showing(&m.name, |ui| {
        ui.wrap_as((x, cy, w), TITLE_M, Role::Primary, m.name.as_str())
    }) + GAP_SMALL;
    let steps = vec![
        button(
            m.words.previous.clone(),
            Style::Text,
            false,
            (m.pick)(pin.saturating_sub(1)),
        ),
        button(
            m.words.next.clone(),
            Style::Outlined,
            false,
            (m.pick)((pin + 1).min(last)),
        ),
    ];
    cy += buttons::draw_items(&steps, &[], false, ui, (x, cy, w));
    cy + GAP - y
}

/// The ink of square (col, row): grey where the blank sheet prints it.
fn ink_of<A>(m: &QrGridModel<'_, A>, col: usize, row: usize, n: usize) -> Color {
    if (m.printed)(col, row, n) {
        PRINTED_INK
    } else {
        INK
    }
}

/// The whole code at `cell` a square, its blocks' names in the room above and beside it from
/// `(x, y)`; `pin` marked. Returns the side drawn, names included.
fn whole<A: Pressed>(
    m: &QrGridModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    cell: f32,
    pin: usize,
) -> f32 {
    let n = m.code.size();
    let per = copy_blocks(n);
    let lab = C32_GRID_LABEL;
    let (gx, gy) = (x + lab, y + lab);
    let side = cell * n as f32;
    ui.fill(gx, gy, side, side, 0.0, PAPER);
    // Each block (each row) a press, its layer on the paper under the squares.
    let marks = if m.rows { n } else { per * per };
    let mark_at = |i: usize| {
        if m.rows {
            (gx, gy + i as f32 * cell, side, cell)
        } else {
            let (c0, r0, bw, bh) = copy_block_rect(i, n);
            let at = |v: usize| v as f32 * cell;
            (gx + at(c0), gy + at(r0), at(bw), at(bh))
        }
    };
    for i in 0..marks {
        let (mx, my, mw, mh) = mark_at(i);
        // A block at the right or bottom edge is short where the grid does not divide by the
        // block: its press reaches out to the touch floor into the room beside or under the
        // grid.
        let (mw, mh) = if m.rows {
            (mw, mh)
        } else {
            let (ew, eh) = (gx + side - (mx + mw), gy + side - (my + mh));
            (
                if ew < 0.5 { mw.max(ui.touch) } else { mw },
                if eh < 0.5 { mh.max(ui.touch) } else { mh },
            )
        };
        ui.press_box(mx, my, mw, mh, (m.pick)(i));
    }
    for row in 0..n {
        for col in 0..n {
            if m.code.module(col, row) {
                let (cx, cy) = (gx + col as f32 * cell, gy + row as f32 * cell);
                ui.fill(cx, cy, cell, cell, 0.0, ink_of(m, col, row, n));
            }
        }
    }
    for i in 0..=n {
        let rule = if i % COPY_BLOCK == 0 || i == n {
            BLOCK_RULE
        } else {
            RULE
        };
        let o = i as f32 * cell;
        ui.fill(gx + o, gy, 1.0, side, 0.0, rule);
        ui.fill(gx, gy + o, side, 1.0, 0.0, rule);
    }
    let span = COPY_BLOCK as f32 * cell;
    for b in 0..per {
        let o = b as f32 * span;
        let (col, row) = if m.lettered {
            let letter = char::from(b'A' + u8::try_from(b).unwrap_or(0));
            ((b + 1).to_string(), letter.to_string())
        } else {
            let at = (b * COPY_BLOCK).to_string();
            (at.clone(), at)
        };
        // Each name at the middle of its block.
        let span_here = span.min(side - o);
        let cw = ui.measure_as(BODY_S.mono(), &col);
        let col_x = gx + o + (span_here - cw) / 2.0;
        ui.text_mid_as((col_x, y, lab), BODY_S.mono(), Role::OnSurfaceVariant, &col);
        ui.text_mid_as(
            (x, gy + o, span_here),
            BODY_S.mono(),
            Role::OnSurfaceVariant,
            &row,
        );
    }
    if pin < marks {
        let (mx, my, mw, mh) = mark_at(pin);
        ui.fill(mx, my, mw, mh, 0.0, Role::Primary.with_alpha(70));
        ui.stroke(mx, my, mw, mh, 0.0, Role::Primary);
    }
    lab + side
}

/// Block `pin` large at `cell` a square from `(x, y)`: its squares, and one square of its
/// neighbours faded round it, its row and column named in the accent. Returns the height drawn.
fn zoomed<A: Pressed>(
    m: &QrGridModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    cell: f32,
    pin: usize,
) -> f32 {
    let n = m.code.size();
    let (c0, r0, bw, bh) = copy_block_rect(pin, n);
    let lab = C32_GRID_LABEL;
    let (gx, gy) = (x + lab, y + lab);
    let ring = COPY_BLOCK + 2;
    let side = ring as f32 * cell;
    ui.fill(gx, gy, side, side, 0.0, PAPER);
    for dy in 0..ring {
        for dx in 0..ring {
            let (Some(col), Some(row)) = ((c0 + dx).checked_sub(1), (r0 + dy).checked_sub(1))
            else {
                continue;
            };
            if col >= n || row >= n {
                continue;
            }
            let inside = (c0..c0 + bw).contains(&col) && (r0..r0 + bh).contains(&row);
            let (cx, cy) = (gx + dx as f32 * cell, gy + dy as f32 * cell);
            if m.code.module(col, row) {
                let ink = ink_of(m, col, row, n);
                let ink = if inside { ink } else { ink.with_alpha(60) };
                ui.fill(cx, cy, cell, cell, 0.0, ink);
            }
            ui.stroke(cx, cy, cell, cell, 0.0, RULE);
        }
    }
    let (bw, bh) = (bw as f32 * cell, bh as f32 * cell);
    ui.stroke(gx + cell, gy + cell, bw, bh, 0.0, Role::Primary);
    let name = copy_block_name(pin, n, m.lettered);
    let (r0t, c0t) = (r0.to_string(), c0.to_string());
    let (row, col): (&str, &str) = if m.lettered {
        name.split_at(1)
    } else {
        (&r0t, &c0t)
    };
    let cw = ui.measure_as(BODY_S.mono(), col);
    let col_x = gx + cell + (bw - cw) / 2.0;
    ui.text_mid_as((col_x, y, lab), BODY_S.mono(), Role::Primary, col);
    ui.text_mid_as((x, gy + cell, bh), BODY_S.mono(), Role::Primary, row);
    lab + side
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: Blocks are counted from the top left in fives, the last of a row or column narrower where the side is not a multiple of five, and a grid names its blocks "A1" or by their first square.
    /// Regression: A block past the edge of the code, or a name that points at the wrong block.
    /// Owner: kit
    #[test]
    fn blocks_are_fives_from_the_top_left_and_the_last_is_short() {
        assert_eq!(copy_blocks(25), 5);
        assert_eq!(copy_blocks(29), 6);
        assert_eq!(copy_blocks(21), 5);
        // 29 x 29: the block at the right of the first row is four wide.
        assert_eq!(copy_block_rect(5, 29), (25, 0, 4, 5));
        // 21 x 21: the last block of a side is one square.
        assert_eq!(copy_block_rect(4, 21), (20, 0, 1, 5));
        assert_eq!(copy_block_name(7, 25, true), "B3");
        assert_eq!(copy_block_name(7, 25, false), "5 · 10");
    }
}
