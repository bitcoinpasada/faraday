//! Words so far in New key's dice step: the words the rolls have named, a
//! pill each in its place, and each word's working on a sheet. It stands
//! in a pane at the card's right where the window holds one beside Your
//! choices, below the pad in the card elsewhere, and on the small panel
//! behind a "Words so far" button that opens a page (`docs/LAYOUT.md`
//! §16.3, `docs/FLOW-CONTAINER.md`). The Guide takes the pane's place
//! while it stands there. The word just made glides in from the strip.
//! Drawing reads the flow's state.

use faraday_bip::bip39::Language;
use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_M, BODY_S, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_text::Text;

use super::{KeyGen, dice_strip, kstep};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Shape, Style, Ui};

/// Where Words so far stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum At {
    /// In the supporting pane at the card's right.
    Pane,
    /// Below the pad, in the card.
    Below,
    /// Behind a button, on a page of its own.
    Button,
}

/// The words made, by place: the rolls' own, and the last once its
/// checksum is in. `hide` is a word still gliding in.
fn made(k: &KeyGen, hide: Option<u8>) -> Vec<Option<u16>> {
    let live = k.live_words();
    let last = k.words - 1;
    (0..k.words)
        .map(|s| {
            if hide.is_some_and(|h| usize::from(h) == s) {
                None
            } else if s < last {
                live.get(s).copied()
            } else {
                k.checksum_word()
            }
        })
        .collect()
}

/// The table's cells: a number and a pill each.
struct Grid {
    x: f32,
    y: f32,
    cols: usize,
    cell: f32,
    num_w: f32,
    rows: usize,
}

impl Grid {
    fn new(ui: &mut Ui, (x, y, w): (f32, f32, f32), words: usize) -> Grid {
        let num_w = ui.measure_as(BODY_M.mono(), "00");
        let pill = ui.measure_as(BODY_M.mono(), "mmmmmmmm") + 2.0 * t::TAG_PAD;
        let cell = num_w + t::GAP_SMALL + pill + t::GAP;
        let cols = (((w + t::GAP) / cell) as usize).clamp(1, words.max(1));
        Grid {
            x,
            y,
            cols,
            cell,
            num_w,
            rows: words.div_ceil(cols),
        }
    }

    /// A row is a touch target tall, the pill in its middle.
    fn pitch() -> f32 {
        t::TOUCH
    }

    /// Where the pill of place `s` stands.
    fn pill_at(&self, s: usize) -> (f32, f32) {
        (
            self.x + (s % self.cols) as f32 * self.cell + self.num_w + t::GAP_SMALL,
            self.y + (s / self.cols) as f32 * Self::pitch() + (t::TOUCH - t::TAG_H) / 2.0,
        )
    }

    fn height(&self) -> f32 {
        self.rows as f32 * Self::pitch()
    }
}

/// The mask's stand-in for a word.
const MASK: &str = "• • •";

/// Draws the cells: each made word a pill that opens its sheet, masked
/// when the eye is closed, and the rest a dot.
fn cells(ui: &mut Ui, g: &Grid, words: &[Option<u16>], masked: bool) {
    let muted = Role::OnSurfaceVariant;
    for (s, w) in words.iter().enumerate() {
        let (px, py) = g.pill_at(s);
        let n = Text::num(s as u64 + 1);
        let nw = ui.measure_as(BODY_M.mono(), &n);
        ui.text_mid_as(
            (px - t::GAP_SMALL - nw, py, t::TAG_H),
            BODY_M.mono(),
            muted,
            &n,
        );
        match w {
            Some(i) => {
                let word = if masked {
                    MASK
                } else {
                    Language::English.word(*i)
                };
                let pw = ui.measure_as(BODY_M.mono(), word) + 2.0 * t::TAG_PAD;
                ui.fill(
                    px,
                    py,
                    pw,
                    t::TAG_H,
                    t::SHAPE_S,
                    Role::SurfaceContainerHighest,
                );
                ui.stroke(px, py, pw, t::TAG_H, t::SHAPE_S, Role::Outline);
                ui.text_mid_as(
                    (px + t::TAG_PAD, py, t::TAG_H),
                    BODY_M.mono(),
                    Role::OnSurface,
                    word,
                );
                // The press is a touch target at least.
                let (tw, th) = (pw.max(t::TOUCH), t::TOUCH);
                ui.press(
                    Shape::new(
                        px - (tw - pw) / 2.0,
                        py - (th - t::TAG_H) / 2.0,
                        tw,
                        th,
                        t::SHAPE_S,
                    ),
                    Role::OnSurface.paint(),
                    Action::KWordOpen(s as u8),
                );
            }
            None => {
                ui.text_mid_as((px, py, t::TAG_H), BODY_M.mono(), muted, "·");
            }
        }
    }
}

/// The pill gliding from `from` to `to`, `p` of the way, with the
/// emphasis easing away behind it.
fn glide(ui: &mut Ui, (from, to): ((f32, f32), (f32, f32)), word: &str, p: f32, fades: bool) {
    let e = crate::motion::ease_move(p);
    let (x, y) = (from.0 + (to.0 - from.0) * e, from.1 + (to.1 - from.1) * e);
    let w = ui.measure_as(BODY_M.mono(), word) + 2.0 * t::TAG_PAD;
    // Out of view, the pill fades on its way to the card's lower edge.
    let keep = if fades { 1.0 - e } else { 1.0 };
    ui.fill(
        x,
        y,
        w,
        t::TAG_H,
        t::SHAPE_S,
        Role::SurfaceContainerHighest.at(keep),
    );
    ui.stroke(x, y, w, t::TAG_H, t::SHAPE_S, Role::Outline.at(keep));
    ui.text_mid_as(
        (x + t::TAG_PAD, y, t::TAG_H),
        BODY_M.mono(),
        Role::OnSurface.at(keep),
        word,
    );
    let alpha = (t::STATE_PRESS + t::PRESS_EMPHASIS) * (1.0 - e) * keep;
    ui.wash(x, y, w, t::TAG_H, t::SHAPE_S, Role::Primary.at(alpha));
}

/// The word gliding in, as the table shows it: its pill, masked as the
/// table is.
fn arriving(k: &KeyGen, slot: u8) -> Text {
    if k.strip_masked() {
        return Text::of(MASK);
    }
    let live = k.live_words();
    live.get(usize::from(slot))
        .map_or_else(|| Text::of(""), |&i| Text::of(Language::English.word(i)))
}

/// Words so far in the card, below the pad (or its button on the small
/// panel), with the word gliding in from where the strip drew it
/// (`from`). Returns its height.
pub(crate) fn inline(
    app: &View,
    k: &KeyGen,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    from: Option<(f32, f32)>,
) -> f32 {
    let arrival = app.keygen_arrival();
    let words = made(k, arrival.map(|(s, _)| s));
    let count = words.iter().flatten().count();
    let at = app.words_at();
    let mut cy = y + t::GAP_SMALL;
    let target;
    match at {
        At::Button => {
            let label = Text::fill(EN.keygen_words_so_far_n, &[&Text::num(count as u64)]);
            let h = ui.button(
                x,
                cy,
                Some(w),
                t::BUTTON_H,
                &label,
                Style::Secondary,
                Action::KWordsPage(true),
            );
            target = (x + w / 2.0, cy);
            cy += h + t::GAP;
        }
        At::Below => {
            faraday_ui::parts::section_label(ui, x, cy, EN.keygen_words_so_far);
            cy += ui.line_as(TITLE_S) + t::GAP_SMALL;
            let g = Grid::new(ui, (x, cy, w), k.words);
            cells(ui, &g, &words, k.strip_masked());
            target = arrival.map_or((x, cy), |(s, _)| g.pill_at(usize::from(s)));
            cy += g.height() + t::GAP;
        }
        // The pane draws it; the word glides to its cell there, laid out
        // with the pane (`Ui::words_pane`). A cell below the pane's view
        // is out of sight: the word heads for the pane's lower edge and
        // fades. Without the pane's place, an estimate by the column.
        At::Pane => {
            let place = arrival.map(|(s, _)| usize::from(s));
            let (to, fades) = match (ui.words_pane, place) {
                (Some((px, py, pw, view)), Some(s)) => {
                    let top = py + t::PAD + ui.line_as(TITLE_S) + t::GAP;
                    let g = Grid::new(ui, (px + t::PAD, top, pw - 2.0 * t::PAD), k.words);
                    let cell = g.pill_at(s);
                    if cell.1 + t::TAG_H > py + view {
                        ((cell.0, py + view), true)
                    } else {
                        (cell, false)
                    }
                }
                _ => {
                    let g = Grid::new(ui, (0.0, 0.0, t::SUPPORT_PANE - 2.0 * t::PAD), k.words);
                    let off = place.map_or(0.0, |s| g.pill_at(s).0);
                    let at = (
                        x + w + t::PAD + t::SPACER + t::PAD + off,
                        from.map_or(cy, |f| f.1),
                    );
                    (at, false)
                }
            };
            return glide_from(ui, k, arrival, from, (to, fades), y, cy);
        }
    }
    glide_from(ui, k, arrival, from, (target, false), y, cy)
}

fn glide_from(
    ui: &mut Ui,
    k: &KeyGen,
    arrival: Option<(u8, f32)>,
    from: Option<(f32, f32)>,
    (to, fades): ((f32, f32), bool),
    y: f32,
    cy: f32,
) -> f32 {
    if let (Some((s, p)), Some(from)) = (arrival, from) {
        glide(ui, (from, to), &arriving(k, s), p, fades);
    }
    cy - y
}

/// Where the pane stands in the page's drawing: the Guide's place at the
/// window's right (`screens::support_pane_place` over the room from the
/// navigation to the window's edge), which the flow's unit makes room for
/// as it does for the Guide's pane.
pub(crate) fn pane_place(app: &View, ui: &Ui, (x0, _, h): (f32, f32, f32)) -> (f32, f32, f32, f32) {
    let room = app.size().0 - x0;
    crate::screens::support_pane_place((x0, room - ui.ox, h))
}

/// The supporting pane at the page area's right, `(x0, cw, h)` as the
/// Guide's pane takes them (`screens::support_pane_place`).
pub(crate) fn pane(app: &View, k: &KeyGen, ui: &mut Ui, area: (f32, f32, f32)) {
    let (x, y, w, view) = pane_place(app, ui, area);
    ui.fill(x, y, w, view, t::SHAPE_L, Role::SurfaceContainer);
    let words = made(k, app.keygen_arrival().map(|(s, _)| s));
    let (ix, iw) = (x + t::PAD, w - 2.0 * t::PAD);
    let mut cy = y + t::PAD;
    faraday_ui::parts::section_label(ui, ix, cy, EN.keygen_words_so_far);
    cy += ui.line_as(TITLE_S) + t::GAP;
    let g = Grid::new(ui, (ix, cy, iw), k.words);
    cells(ui, &g, &words, k.strip_masked());
}

/// Words so far as a page of its own (the small panel's button), and a
/// word's sheet over the page or the step: drawn last in the frame.
pub(crate) fn overlays(app: &View, k: &KeyGen, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    if k.open != Some(kstep::ENTER) || k.hosted() || !dice_strip::stands(k) {
        return;
    }
    if k.words_page() && ui.compact {
        page(app, k, ui, (x0, cw, h));
    }
    if let Some(s) = k.word_sheet() {
        sheet(app, k, ui, (x0, cw, h), s);
    }
}

fn page(app: &View, k: &KeyGen, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    use crate::compact::{BAR_H, M};
    ui.clear_hits();
    ui.pin = None;
    ui.fill(x0, 0.0, cw, h, 0.0, Role::Surface);
    crate::compact::bar(
        app,
        ui,
        cw,
        (
            super::screen::card_name(k, kstep::ENTER),
            Action::KWordsPage(false),
        ),
        EN.keygen_words_so_far,
    );
    let words = made(k, None);
    let g = Grid::new(ui, (M, BAR_H + t::PAD, cw - 2.0 * M), k.words);
    cells(ui, &g, &words, k.strip_masked());
}

/// A word's working: its rolls, their bits, the number they make and
/// the word; the last word's checksum too. Read from what the step
/// holds.
/// A word's working: its rows, and the line about its checksum.
type Working = (Vec<(&'static str, Text)>, Option<Text>);

fn working(k: &KeyGen, slot: usize) -> Option<Working> {
    let per = faraday_entropy::DICE_WORD_ROLLS;
    let last = k.words - 1;
    let rolls = k.dice.rolls();
    let from = slot * per;
    let is_last = slot == last;
    let live = k.live_words();
    let index = if is_last {
        k.checksum_word()?
    } else {
        *live.get(slot)?
    };
    let got = rolls.get(from..)?;
    let got = &got[..got.len().min(if is_last { k.last_rolls().0 } else { per })];
    let mut faces = Text::with_room(2 * per);
    for (i, f) in got.iter().enumerate() {
        if i > 0 {
            faces.push(' ');
        }
        faces.push_str(&Text::num(u64::from(*f)));
    }
    let all = Text::formatted(format_args!("{index:011b}"));
    let sums = k.words / 3;
    let kept = k.last_bits();
    let mut bits = Text::with_room(32);
    if is_last {
        bits.push_str(&all.as_str()[..kept]);
        bits.push(' ');
        bits.push_str(&all.as_str()[kept..]);
    } else {
        for (p, &f) in got.iter().enumerate() {
            if p > 0 {
                bits.push(' ');
            }
            bits.push_str(dice_strip::bits_of(p, f, false));
        }
    }
    let mut rows = vec![
        (EN.keygen_rolls, faces),
        (EN.wordlist_bits, bits),
        (EN.roller_number, Text::of(&Text::num(u64::from(index)))),
        (EN.wordlist_word, Text::of(Language::English.word(index))),
    ];
    let line = is_last.then(|| {
        rows.push((
            EN.glance_sheet_checksum,
            Text::of(&all.as_str()[11 - sums..]),
        ));
        Text::fill(
            EN.keygen_the_last_bits,
            &[&Text::num(sums as u64), &Text::num(slot as u64 + 1)],
        )
    });
    Some((rows, line))
}

fn sheet(app: &View, k: &KeyGen, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), slot: u8) {
    let Some((rows, line)) = working(k, usize::from(slot)) else {
        return;
    };
    let masked = k.strip_masked();
    let head = Text::fill(EN.keygen_word_n, &[&Text::num(u64::from(slot) + 1)]);
    let draw_rows = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let muted = Role::OnSurfaceVariant;
        let label_w = rows
            .iter()
            .map(|(l, _)| ui.measure_as(BODY_S, l))
            .fold(0.0, f32::max);
        let mut cy = y;
        for (label, value) in &rows {
            let shown = if masked {
                "• • • •"
            } else {
                value.as_str()
            };
            let rh = ui.line_as(BODY_M);
            ui.text_mid_as((x, cy, rh), BODY_S, muted, label);
            ui.text_mid_as(
                (x + label_w + t::GAP, cy, rh),
                BODY_M.mono(),
                Role::OnSurface,
                shown,
            );
            cy += rh + t::GAP_SMALL;
        }
        if let (Some(l), false) = (&line, masked) {
            cy += ui.wrap_as((x, cy, iw), BODY_S, muted, l) + t::GAP_SMALL;
        }
        cy - y
    };
    if ui.compact {
        use crate::compact::{BAR_H, M};
        ui.clear_hits();
        ui.pin = None;
        ui.fill(x0, 0.0, cw, h, 0.0, Role::Surface);
        let back = if k.words_page() {
            EN.keygen_words_so_far
        } else {
            super::screen::card_name(k, kstep::ENTER)
        };
        crate::compact::bar(app, ui, cw, (back, Action::KWordClose), &head);
        draw_rows(ui, M, BAR_H + t::PAD, cw - 2.0 * M);
        return;
    }
    let scrim = Role::Scrim.with_alpha(t::SCRIM_ALPHA);
    ui.wash(x0, 0.0, cw, h, 0.0, scrim);
    ui.clear_hits();
    let outside = ui.outside.replace(Action::KWordClose);
    let mut draw_rows = draw_rows;
    faraday_ui::sheet::dialog_in(
        ui,
        (x0, cw, h, t::CHART_SHEET_WIDTH),
        (Icon::Info.into(), Role::Primary.paint(), &head),
        &mut draw_rows,
        &[(EN.app_done, Style::Primary, Action::KWordClose)],
    );
    ui.outside = outside;
}
