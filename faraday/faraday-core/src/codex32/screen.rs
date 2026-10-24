//! Make codex32 shares' cards, drawn as cards of New key's column
//! (`docs/CODEX32.md` §6, §7): the scheme, the identifier, the entropy,
//! the entry with the booklet's sheets filling live, the worksheets read
//! back, the strings, and Strings made. Every value is read from the
//! flow's state and the work set it holds; nothing here computes.
//!
//! The entry lays out by the window's class (§6.4.7, `docs/LAYOUT.md`
//! §2): expanded and up, the share's string over two panels side by
//! side, the dice sheet and the worksheet the strip chose; medium, the
//! same stacked; compact, one context panel at a time, chosen by a tile
//! row over the pad.

use faraday_bip::codex32::{SYMBOL_GLYPHS, Scheme};
use faraday_draw::geom::{SizeClass, WindowClass};
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_entropy::booklet::{BOOKLET_DICE, TREE};
use faraday_keys::{Reveal, WorkSet};
use faraday_ui::keyboard::Keys as OskKeys;
use faraday_ui::parts::wrap_buttons;

use super::{C32, CARDS, Card, Cell, Codex32Make, SPLITS, Split, Tab, WAYS, Way, char_of};
use crate::drawing::View;
use crate::keygen::{KeyGen, kstep};
use crate::screens::next_button;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, flow};

/// Boxes 1 to 3 of every string.
const PREFIX: &[u8] = b"ms1";

/// The identifier's boxes.
const ID_LEN: u8 = 4;

/// Symbol `v` of the booklet's second alphabet, drawn as its glyph
/// (§7.2).
fn glyph(v: u8) -> String {
    SYMBOL_GLYPHS[usize::from(v & 31)].to_string()
}

/// A press of this flow.
fn a(p: C32) -> Action {
    Action::Codex32(p)
}

/// Where a part of a card is drawn: its left edge, its top, its width,
/// and the window's class, which lays the entry out (§6.4.7): compact one
/// panel at a time, medium stacked, expanded and up side by side.
#[derive(Clone, Copy)]
struct At {
    x: f32,
    y: f32,
    w: f32,
    cls: WindowClass,
    /// The sheets are masked (§6.4.6): characters are drawn as dots.
    hide: bool,
    /// The five-dice roller stands beside the pad (Faraday Desktop,
    /// §5.5).
    roller: bool,
}

/// What a masked character is drawn as.
const MASK: &str = "\u{2022}";

impl At {
    /// The same place moved to (x, y), `w` wide.
    fn to(self, x: f32, y: f32, w: f32) -> At {
        At { x, y, w, ..self }
    }
}

/// The flow's cards for New key's column, each with the card it is.
pub(crate) fn cards(k: &KeyGen, m: &Codex32Make) -> Vec<(Card, flow::Card)> {
    let open = |card: Card| k.open == Some(kstep::CODEX32) && m.open() == Some(card);
    CARDS
        .iter()
        .map(|&card| {
            (
                card,
                flow::Card {
                    title: title(m, card),
                    summary: summary(m, card),
                    mono: matches!(card, Card::Identifier | Card::Made) && m.done(card),
                    done: m.done(card),
                    open: open(card),
                    default: false,
                    toggle: a(C32::Card(card.index())),
                    // Built only for the open card.
                    guide: open(card).then(|| guide(card).to_string()),
                    learn: open(card).then(|| learn(card)),
                },
            )
        })
        .collect()
}

/// The Learn page a card's walk-through links to.
fn learn(card: Card) -> crate::learn::Topic {
    match card {
        Card::Entropy | Card::Enter => crate::learn::Topic::Randomness,
        _ => crate::learn::Topic::OtherBackups,
    }
}

/// Guided mode's walk-through of a card: what it does and what its
/// button does.
fn guide(card: Card) -> &'static str {
    match card {
        Card::Scheme => EN.guide_codex32_scheme,
        Card::Identifier => EN.guide_codex32_identifier,
        Card::Entropy => EN.guide_codex32_entropy,
        Card::Enter => EN.guide_codex32_enter,
        Card::Sheets => EN.guide_codex32_sheets,
        Card::Strings => EN.guide_codex32_strings,
        Card::Made => EN.guide_codex32_made,
    }
}

fn title(m: &Codex32Make, card: Card) -> String {
    match card {
        Card::Scheme => EN.codex32_scheme.to_string(),
        Card::Identifier => EN.codex32_identifier.to_string(),
        Card::Entropy => EN.codex32_entropy.to_string(),
        Card::Enter => enter_title(m),
        Card::Sheets => EN.codex32_worksheets.to_string(),
        Card::Strings => strings_title(m),
        Card::Made => EN.codex32_strings_made.to_string(),
    }
}

/// "Share A · character 7 of 26", or "Secret · …" with no threshold.
fn enter_title(m: &Codex32Make) -> String {
    let Some(w) = m.work() else {
        return EN.codex32_enter.to_string();
    };
    let data = w.scheme().length().data_len();
    match m.front() {
        Some((s, n)) => fill(
            EN.codex32_share_character,
            &[
                &share_name(w.scheme(), s),
                &(n + 1).to_string(),
                &data.to_string(),
            ],
        ),
        None => EN.codex32_enter.to_string(),
    }
}

/// "Share 2 of 5", or "Codex32" for the secret alone.
fn strings_title(m: &Codex32Make) -> String {
    let (k, n) = m.k_n();
    if k == 0 {
        return EN.codex32_codex32.to_string();
    }
    if m.open() == Some(Card::Strings) {
        fill(
            EN.codex32_share_n_of,
            &[&(m.at() + 1).to_string(), &n.to_string()],
        )
    } else {
        EN.codex32_strings.to_string()
    }
}

/// "Share A", or "Secret" with no threshold: initial share `s`.
fn share_name(scheme: &Scheme, s: usize) -> String {
    if scheme.threshold() == 0 {
        return EN.codex32_the_secret.to_string();
    }
    let i = scheme.initial_indices().get(s).copied().unwrap_or(b'a');
    fill(
        EN.codex32_share_x,
        &[&char::from(i.to_ascii_uppercase()).to_string()],
    )
}

/// The split's words: "2 of 3", or "None, the secret alone".
fn split_text(m: &Codex32Make) -> String {
    match m.k_n() {
        (0, _) => EN.codex32_split_alone.to_string(),
        (k, n) => fill(EN.codex32_k_of_n, &[&k.to_string(), &n.to_string()]),
    }
}

fn summary(m: &Codex32Make, card: Card) -> String {
    if !m.done(card) {
        return EN.keygen_not_yet.to_string();
    }
    match card {
        Card::Scheme => fill(
            EN.codex32_scheme_summary,
            &[&m.length().chars().to_string(), &split_text(m)],
        ),
        Card::Identifier => m.identifier(),
        Card::Entropy => way_name(m.way()).to_string(),
        Card::Enter | Card::Sheets => EN.codex32_done.to_string(),
        Card::Strings => fill(
            EN.codex32_strings_n,
            &[&m.strings().map_or(0, <[_]>::len).to_string()],
        ),
        Card::Made => m
            .fingerprint()
            .map(|f| fp_text(faraday_bip::keys::Fingerprint(f)).to_string())
            .unwrap_or_default(),
    }
}

fn way_name(w: Way) -> &'static str {
    match w {
        Way::Dice => EN.codex32_roll_dice,
        Way::Typed => EN.codex32_type_the_characters,
    }
}

fn split_name(s: Split) -> &'static str {
    match s {
        Split::TwoOfThree => EN.codex32_two_of_three,
        Split::ThreeOfFive => EN.codex32_three_of_five,
        Split::Other => EN.codex32_other,
        Split::Alone => EN.codex32_split_alone,
    }
}

/// A choice of one: Material's segmented button, or on a small panel
/// option buttons, the chosen one Selected. Returns the height.
fn choice(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, bool, Action)]) -> f32 {
    // Material's segmented button, 40 dp in a 48 dp row; buttons wrapping
    // onto rows where it does not fit (F108).
    // A small panel's buttons are drawn at its own size, so they wrap
    // whole, the chosen one Selected.
    if ui.compact {
        let rows: Vec<(&str, Style, Action)> = items
            .iter()
            .map(|&(l, on, act)| (l, Style::chosen(on), act))
            .collect();
        return wrap_buttons(ui, x, y, w, t::TOUCH, &rows);
    }
    let options: Vec<(&str, Action)> = items.iter().map(|&(l, _, act)| (l, act)).collect();
    let chosen = items.iter().position(|&(_, on, _)| on);
    let inset = (t::TOUCH - t::CODEX32_CHOICE_H) / 2.0;
    let h = ui.choices(
        (x, y + inset, w, t::CODEX32_CHOICE_H),
        t::LABEL,
        &options,
        chosen,
    );
    h + 2.0 * inset + t::GAP
}

/// One secondary tab: its label, whether it is the one shown, whether it
/// was seen (a check icon), and what a press does.
struct TabItem {
    label: String,
    on: bool,
    seen: bool,
    action: Action,
}

/// Secondary tabs, [`t::CODEX32_TAB`] tall, an indicator in `ACCENT`
/// under the one shown, a rule under the row. Where they do not all fit
/// the row shows the run that ends at the one shown (it scrolls with
/// it). Returns the height.
fn tabs(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[TabItem]) -> f32 {
    let icon = t::CODEX32_TAB_ICON;
    let widths: Vec<f32> = items
        .iter()
        .map(|i| {
            ui.measure(t::LABEL, W::S, &i.label)
                + 2.0 * t::PAD
                + if i.seen { icon + t::GAP_SMALL } else { 0.0 }
        })
        .collect();
    let total: f32 = widths.iter().sum();
    let on = items.iter().position(|i| i.on).unwrap_or(0);
    let equal = w / items.len().max(1) as f32;
    // Fixed tabs share the width where every label fits its share.
    let fixed = total <= w && widths.iter().all(|&wd| wd <= equal);
    let mut first = 0;
    if total > w {
        let mut used = widths[on];
        first = on;
        while first > 0 && used + widths[first - 1] <= w {
            first -= 1;
            used += widths[first];
        }
    }
    ui.rule(x, y + t::CODEX32_TAB - 1.0, w, INNER);
    let mut tx = x;
    for (i, item) in items.iter().enumerate().skip(first) {
        let tw = if fixed { equal } else { widths[i] };
        if tx + tw > x + w + 0.5 {
            break;
        }
        if ui.is_pressed(item.action) {
            ui.fill(tx, y, tw, t::CODEX32_TAB, 0.0, INNER);
        }
        let colour = if item.on { ACCENT } else { MUTED };
        let lw = ui.measure(t::LABEL, W::S, &item.label);
        let seen = if item.seen { icon + t::GAP_SMALL } else { 0.0 };
        let lx = tx + (tw - lw - seen) / 2.0;
        if item.seen {
            ui.icon(
                lx,
                y + (t::CODEX32_TAB - icon) / 2.0,
                icon,
                Icon::Done,
                t::CODEX32_TAB_GLYPH,
                OK,
            );
        }
        ui.text_mid(
            lx + seen,
            y,
            t::CODEX32_TAB,
            t::LABEL,
            W::S,
            colour,
            &item.label,
        );
        if item.on {
            ui.fill(
                tx,
                y + t::CODEX32_TAB - t::CODEX32_INDICATOR,
                tw,
                t::CODEX32_INDICATOR,
                0.0,
                ACCENT,
            );
        }
        ui.hit(tx, y, tw, t::CODEX32_TAB, item.action);
        tx += tw;
    }
    t::CODEX32_TAB + t::GAP
}

/// A card's body. Returns its height.
pub(crate) fn body(
    app: &View,
    m: &Codex32Make,
    card: Card,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let at = At {
        x,
        y,
        w,
        cls: app.window().class(),
        hide: m.masked(),
        roller: app.roller_offered(),
    };
    let h = match card {
        Card::Scheme => scheme(m, ui, x, y, w),
        Card::Identifier => identifier(m, ui, at),
        Card::Entropy => entropy(m, ui, x, y, w),
        Card::Enter => enter(m, ui, at),
        Card::Sheets => sheets(m, ui, at),
        Card::Strings => strings(m, ui, at),
        Card::Made => made(m, ui, x, y, w),
    };
    h + t::GAP
}

/// The note under a card's controls, in `ERR`, if any. Returns the
/// height.
fn note(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match m.note() {
        Some(n) => ui.wrap(x, y, w, t::LABEL, W::R, ERR, n) + t::GAP,
        None => 0.0,
    }
}

/// Continue at the card's foot, under its note. Returns the height.
fn foot(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32, label: &str) -> f32 {
    let nh = note(m, ui, x, y, w);
    let drawn = next_button(ui, x, y + nh, w, label, a(C32::Next));
    nh + if drawn { t::TOUCH } else { t::GAP }
}

/// §6.1: length and split, Other's steppers, the line for k of 4 or more.
fn scheme(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let fixed = m.fixed() || m.work().is_some();
    crate::screens::section_label(ui, x, cy, EN.codex32_length);
    cy += t::PAD + t::GAP_SMALL;
    let len = m.length().chars();
    cy += choice(
        ui,
        x,
        cy,
        w,
        &[
            (EN.codex32_chars_48, len == 48, a(C32::Length(0))),
            (EN.codex32_chars_74, len == 74, a(C32::Length(1))),
        ],
    );
    crate::screens::section_label(ui, x, cy, EN.codex32_split);
    cy += t::PAD + t::GAP_SMALL;
    let items: Vec<(&str, bool, Action)> = SPLITS
        .iter()
        .enumerate()
        .map(|(i, &s)| (split_name(s), m.split() == s, a(C32::Split(i as u8))))
        .collect();
    cy += choice(ui, x, cy, w, &items);
    let (k, n) = m.k_n();
    if m.split() == Split::Other {
        crate::screens::stepper(
            ui,
            x,
            cy,
            w,
            EN.codex32_threshold,
            usize::from(k),
            a(C32::K(-1)),
            a(C32::K(1)),
            !fixed,
        );
        cy += t::STEPPER_ROW;
        crate::screens::stepper(
            ui,
            x,
            cy,
            w,
            EN.codex32_strings,
            usize::from(n),
            a(C32::N(-1)),
            a(C32::N(1)),
            !fixed,
        );
        cy += t::STEPPER_ROW;
    }
    if k >= 4 {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            MUTED,
            &fill(EN.codex32_no_translation_sheet, &[&k.to_string()]),
        ) + t::GAP;
    }
    cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    cy - y
}

/// The codex32 keyboard, inside the card, its keys this flow's. Returns
/// its height.
fn keyboard(ui: &mut Ui, x: f32, y: f32, w: f32, kind: KeyboardKind) -> f32 {
    faraday_ui::keyboard::keyboard(ui, x, w, y, kind, OskKeys::default(), ui.compact, |p| {
        a(C32::Key(p))
    });
    faraday_ui::keyboard::height(kind) + t::GAP
}

/// §6.2: four boxes typed on the codex32 keyboard, and Suggest.
fn identifier(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let mut cy = y;
    let id = m.identifier();
    let b = t::codex32_box(cls) * 2.0;
    for i in 0..usize::from(ID_LEN) {
        let bx = x + i as f32 * (b + t::GAP);
        let on = i == id.len();
        ui.fill(bx, cy, b, b, t::RADIUS_SMALL, SURFACE);
        ui.stroke(
            bx,
            cy,
            b,
            b,
            t::RADIUS_SMALL,
            if on { ACCENT } else { BORDER },
        );
        if let Some(c) = id.chars().nth(i) {
            let s = c.to_string();
            let cw = ui.measure(t::TITLE, W::M, &s);
            ui.text_mid(bx + (b - cw) / 2.0, cy, b, t::TITLE, W::M, TEXT, &s);
        }
    }
    cy += b + t::GAP;
    if m.work().is_none() {
        // Beside the boxes where it fits, else under them.
        let boxes = f32::from(ID_LEN) * (b + t::GAP);
        let bw = ui.measure(t::LABEL, W::S, EN.codex32_suggest) + 2.0 * t::PAD;
        let (sx, sy) = if boxes + bw <= w {
            (x + boxes, cy - t::GAP - (b + t::TOUCH) / 2.0)
        } else {
            cy += t::TOUCH + t::GAP;
            (x, cy - t::TOUCH - t::GAP)
        };
        let s = EN.codex32_suggest;
        ui.button(
            sx,
            sy,
            Some(bw),
            t::TOUCH,
            s,
            Style::Secondary,
            a(C32::Suggest),
        );
    }
    if let Some(r) = m.id_refused() {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, ERR, r) + t::GAP;
    }
    if m.work().is_none() {
        cy += keyboard(ui, x, cy, w, KeyboardKind::Codex32);
    }
    cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    cy - y
}

/// §6.3: the way.
fn entropy(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let items: Vec<(&str, bool, Action)> = WAYS
        .iter()
        .enumerate()
        .map(|(i, &wy)| (way_name(wy), m.way() == wy, a(C32::Way(i as u8))))
        .collect();
    cy += choice(ui, x, cy, w, &items);
    if m.way() == Way::Typed {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, EN.codex32_not_chosen) + t::GAP;
    }
    cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    cy - y
}

// ---------------------------------------------------------------------
// The entry (§6.4)
// ---------------------------------------------------------------------

/// §6.4: the sheets over the pad, laid out by class.
fn enter(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(work) = m.work() else {
        return 0.0;
    };
    let mut cy = y;
    if m.way() == Way::Typed {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, EN.codex32_not_chosen) + t::GAP;
    }
    let (share, front) = match m.front() {
        Some((s, n)) => (s, Some(10 + n)),
        None => (work.scheme().initial_indices().len() - 1, None),
    };
    match cls {
        WindowClass::Compact => {
            // The tabs, the context panel one at a time, and the pad
            // under it, all on the page at once: the panel is what the
            // person reads while pressing, so it sits above the pad and
            // never scrolls under it (§6.4.7).
            cy += panel_tiles(m, ui, x, cy, w);
            cy += match m.panel() {
                0 if m.way() == Way::Dice => {
                    let h = dice_rows(m, ui, x, cy, w, true);
                    h + rolled_line(m, ui, x, cy + h, w)
                }
                1 => checksum_band(work, share, front, ui, at.to(x, cy, w)),
                _ => string_boxes(work, share, front, ui, at.to(x, cy, w)),
            };
            cy += pad(m, ui, at.to(x, cy, w));
        }
        _ => {
            cy += string_boxes(work, share, front, ui, at.to(x, cy, w));
            cy += view_strip(m, work, ui, x, cy, w);
            cy += panels(m, work, share, front, ui, at.to(x, cy, w));
            cy += pad(m, ui, at.to(x, cy, w));
        }
    }
    // Continue is there once the sheets are complete: until then it would
    // only take the room the panel and the pad need.
    if m.strings().is_some() {
        cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    } else {
        cy += note(m, ui, x, cy, w);
    }
    cy - y
}

/// The small panel's tile row: Dice · Checksum · String (§6.4.7).
fn panel_tiles(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let names = [
        EN.codex32_tile_dice,
        EN.codex32_tile_checksum,
        EN.codex32_tile_string,
    ];
    let items: Vec<TabItem> = names
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0 || m.way() == Way::Dice)
        .map(|(i, &n)| TabItem {
            label: n.to_string(),
            on: usize::from(m.panel()) == i,
            seen: false,
            action: a(C32::Panel(i as u8)),
        })
        .collect();
    tabs(ui, x, y, w, &items)
}

/// The strip of worksheets under the string: the share's checksum
/// worksheet, then each translation worksheet (§6.4.7, `wide`).
fn view_strip(m: &Codex32Make, work: &WorkSet, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let targets = work.targets();
    if targets.is_empty() {
        return 0.0;
    }
    let mut items = vec![TabItem {
        label: EN.codex32_tile_checksum.to_string(),
        on: m.view() == 0,
        seen: false,
        action: a(C32::View(0)),
    }];
    for (n, &tg) in targets.iter().enumerate() {
        items.push(TabItem {
            label: target_name(tg),
            on: usize::from(m.view()) == n + 1,
            seen: false,
            action: a(C32::View(n as u8 + 1)),
        });
    }
    tabs(ui, x, y, w, &items)
}

/// "Share D", or "Secret" for `s`.
fn target_name(tg: u8) -> String {
    if tg == b's' {
        EN.codex32_the_secret.to_string()
    } else {
        fill(
            EN.codex32_share_x,
            &[&char::from(tg.to_ascii_uppercase()).to_string()],
        )
    }
}

/// The dice sheet and the worksheet the strip chose: side by side on
/// `wide`, stacked on `mobile`.
fn panels(
    m: &Codex32Make,
    work: &WorkSet,
    share: usize,
    front: Option<usize>,
    ui: &mut Ui,
    at: At,
) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let dice = m.way() == Way::Dice;
    let side = cls >= WindowClass::Expanded && dice;
    let (dw, sx, sw) = if side {
        let dw = t::CODEX32_DICE_W;
        (
            dw,
            x + dw + t::CODEX32_PANEL_GAP,
            w - dw - t::CODEX32_PANEL_GAP,
        )
    } else {
        (w, x, w)
    };
    let dh = if dice {
        dice_sheet(m, ui, x, y, dw, cls == WindowClass::Medium)
    } else {
        0.0
    };
    let sy = if side { y } else { y + dh };
    let sh = match m.view() {
        0 => checksum_band(work, share, front, ui, at.to(sx, sy, sw)),
        v => translation(
            work,
            usize::from(v) - 1,
            front.map(|f| (share, f)),
            ui,
            at.to(sx, sy, sw),
        ),
    };
    if side { dh.max(sh) } else { dh + sh }
}

/// The pad: the dice pad's six faces and backspace, or the codex32
/// keyboard for characters typed. On Faraday Desktop the dice pad is
/// "Your dice" and the five-dice roller stands beside it, side by side
/// where the card is wide enough and under it where not (§5.5).
fn pad(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w) = (at.x, at.y, at.w);
    if m.front().is_none() {
        return 0.0;
    }
    if m.way() == Way::Typed {
        return keyboard(ui, x, y, w, KeyboardKind::Codex32);
    }
    if !at.roller {
        return keyboard(ui, x, y, w, KeyboardKind::Dice);
    }
    let side = w >= 2.0 * t::CODEX32_ROLLER_W + t::CODEX32_PANEL_GAP;
    let pw = if side {
        w - t::CODEX32_ROLLER_W - t::CODEX32_PANEL_GAP
    } else {
        w
    };
    let mut cy = y;
    crate::screens::section_label(ui, x, cy, EN.codex32_your_dice_head);
    cy += t::PAD + t::GAP_SMALL;
    let ph = keyboard(ui, x, cy, pw, KeyboardKind::Dice);
    let (rx, ry, rw) = if side {
        (x + pw + t::CODEX32_PANEL_GAP, y, t::CODEX32_ROLLER_W)
    } else {
        (x, cy + ph, w)
    };
    let rh = crate::fresh::roller::five_screen::draw(
        ui,
        (rx, ry, rw),
        m.roller(),
        m.roller_can_roll(),
        |r| a(C32::Roller(r)),
    ) + t::GAP;
    if side {
        (cy - y + ph).max(rh)
    } else {
        cy - y + ph + rh
    }
}

// ---------------------------------------------------------------------
// The string's boxes
// ---------------------------------------------------------------------

/// Where box `b` is drawn when the boxes from `from` start at `x`: the
/// box pitch, and a group gap before each group of four (groups start at
/// box 1).
fn box_x(x: f32, from: usize, b: usize, cls: WindowClass) -> f32 {
    let pitch = t::codex32_box(cls) + t::CODEX32_BOX_GAP;
    let groups = (b - 1) / 4 - (from - 1) / 4;
    x + (b - from) as f32 * pitch + groups as f32 * (t::CODEX32_GROUP_GAP - t::CODEX32_BOX_GAP)
}

/// How many boxes from `from` fit in `w`.
fn boxes_in(w: f32, from: usize, cls: WindowClass) -> usize {
    let mut n = 0;
    while box_x(0.0, from, from + n, cls) + t::codex32_box(cls) <= w {
        n += 1;
    }
    n.max(1)
}

/// How a box is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Look {
    /// A cell worked out, on the table rows' fill.
    Table,
    /// A given box: the person's data or the header, outlined heavier.
    Given,
    /// A checksum square: pink and bold.
    Checksum,
    /// A fixed cell: the residue of `ms`, the final row's letters.
    Fixed,
}

/// One box at (bx, y) holding `v`, drawn as `look`; `front` outlines it
/// in the accent. A press opens `cell`.
fn one_box(ui: &mut Ui, at: At, v: Option<u8>, look: Look, front: bool, cell: Option<Cell>) {
    let (bx, y, cls) = (at.x, at.y, at.cls);
    let s = t::codex32_box(cls);
    let fill_c = match look {
        Look::Table => CODEX32_LOOKUP_FILL,
        Look::Checksum => CODEX32_CHECKSUM_FILL,
        Look::Given | Look::Fixed => SURFACE,
    };
    ui.fill(bx, y, s, s, t::CODEX32_RADIUS, fill_c);
    let edge = match look {
        Look::Given | Look::Checksum => TEXT,
        _ => BORDER,
    };
    ui.stroke(bx, y, s, s, t::CODEX32_RADIUS, edge);
    if matches!(look, Look::Given | Look::Checksum) {
        let i = t::CODEX32_GIVEN_STROKE / 2.0;
        ui.stroke(
            bx + i,
            y + i,
            s - 2.0 * i,
            s - 2.0 * i,
            t::CODEX32_RADIUS,
            edge,
        );
    }
    if front {
        let o = t::CODEX32_FRONT_STROKE;
        ui.stroke(
            bx - o,
            y - o,
            s + 2.0 * o,
            s + 2.0 * o,
            t::CODEX32_RADIUS,
            ACCENT,
        );
        ui.stroke(
            bx - o / 2.0,
            y - o / 2.0,
            s + o,
            s + o,
            t::CODEX32_RADIUS,
            ACCENT,
        );
    }
    if let Some(v) = v {
        let ch = if at.hide {
            MASK.to_string()
        } else {
            char_of(v).to_string()
        };
        let size = t::codex32_char(cls);
        let weight = if matches!(look, Look::Checksum | Look::Fixed) {
            W::S
        } else {
            W::M
        };
        let cw = ui.measure(size, weight, &ch);
        ui.text_mid(bx + (s - cw) / 2.0, y, s, size, weight, TEXT, &ch);
        // The press on a box opens the working of the nearest one: each
        // box's hit takes the gaps round it, so the boxes tile the sheet
        // (§7.4). A masked sheet opens nothing.
        if let Some(cell) = cell.filter(|_| !at.hide) {
            let (gx, gy) = (t::CODEX32_BOX_GAP, t::CODEX32_ROW_GAP);
            ui.hit_quiet(
                bx - gx / 2.0,
                y - gy / 2.0,
                s + gx,
                s + gy,
                a(C32::Cell(cell)),
            );
        }
    }
}

/// The share's string in numbered boxes, groups of four, `ms1` first,
/// wrapping to the width: the header outlined, the checksum pink, the
/// front box in the accent (§6.4.1).
fn string_boxes(work: &WorkSet, share: usize, front: Option<usize>, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(sheet) = work.worksheet(share, Reveal::PaperForm) else {
        return 0.0;
    };
    let chars = sheet.chars();
    let last_data = sheet.last_data_box();
    let s = t::codex32_box(cls);
    let per = (boxes_in(w, 1, cls) / 4 * 4).max(4);
    let mut cy = y;
    let mut b = 1;
    while b <= chars {
        let to = (b + per - 1).min(chars);
        for n in (b..=to).step_by(4) {
            ui.text(
                box_x(x, b, n, cls),
                cy,
                t::CODEX32_NUMBER,
                W::R,
                MUTED,
                &n.to_string(),
            );
        }
        let ry = cy + t::CODEX32_NUMBER_ROW;
        for n in b..=to {
            let bx = box_x(x, b, n, cls);
            if n <= 3 {
                one_box(ui, at.to(bx, ry, 0.0), None, Look::Fixed, false, None);
                let ch = char::from(PREFIX[n - 1]).to_string();
                let size = t::codex32_char(cls);
                let cw = ui.measure(size, W::S, &ch);
                ui.text_mid(bx + (s - cw) / 2.0, ry, s, size, W::S, TEXT, &ch);
                continue;
            }
            let (v, look) = match n {
                _ if n > last_data => (sheet.symbol(n), Look::Checksum),
                _ => (sheet.symbol(n), Look::Given),
            };
            let cell = (n >= 4).then_some(Cell::Given {
                sheet: share as u8,
                b: n as u8,
            });
            one_box(ui, at.to(bx, ry, 0.0), v, look, front == Some(n), cell);
        }
        cy = ry + s + t::CODEX32_ROW_GAP;
        b = to + 1;
    }
    cy - y + t::GAP
}

// ---------------------------------------------------------------------
// The checksum worksheet (§6.4.3, §7.1)
// ---------------------------------------------------------------------

/// One row of a worksheet as drawn: its name, its first box, its cells,
/// how they look and what a press on each opens.
struct Row {
    name: String,
    start: usize,
    cells: Vec<Option<u8>>,
    look: Look,
    cell: fn(u8, usize, usize) -> Cell,
    /// The row's own number (the step), handed to `cell`.
    j: usize,
}

/// Every row of share `share`'s checksum worksheet, in the booklet's
/// order: the string, the residue of `ms`, each step's sum and lookup,
/// the final row and what it must read.
fn checksum_rows(work: &WorkSet, share: usize) -> Vec<Row> {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return Vec::new();
    };
    let len = sh.checksum_len();
    let mut rows = vec![
        Row {
            name: EN.codex32_row_string.to_string(),
            start: 4,
            cells: (4..=sh.chars()).map(|b| sh.symbol(b)).collect(),
            look: Look::Given,
            cell: |s, _, i| Cell::Given {
                sheet: s,
                b: (i + 4) as u8,
            },
            j: 0,
        },
        Row {
            name: EN.codex32_row_ms.to_string(),
            start: 4,
            cells: sh.hrp_row().iter().map(|&v| Some(v)).collect(),
            look: Look::Fixed,
            cell: |s, _, i| Cell::Hrp {
                sheet: s,
                i: i as u8,
            },
            j: 0,
        },
    ];
    for j in 0..sh.steps() {
        rows.push(Row {
            name: fill(EN.codex32_row_sum, &[&(j + 1).to_string()]),
            start: sh.sum_box(j),
            cells: (0..len).map(|i| sh.sum_cell(j, i)).collect(),
            look: Look::Table,
            cell: |s, j, i| Cell::Sum {
                sheet: s,
                j: j as u8,
                i: i as u8,
            },
            j,
        });
        rows.push(Row {
            name: fill(EN.codex32_row_lookup, &[&(j + 1).to_string()]),
            start: sh.lookup_box(j),
            cells: (0..len).map(|i| sh.lookup_cell(j, i)).collect(),
            look: Look::Table,
            cell: |s, j, i| Cell::Lookup {
                sheet: s,
                j: j as u8,
                i: i as u8,
            },
            j,
        });
    }
    let fin = sh.steps();
    rows.push(Row {
        name: EN.codex32_row_final.to_string(),
        start: sh.sum_box(fin),
        cells: (0..len).map(|i| sh.final_cell(i)).collect(),
        look: Look::Given,
        cell: |s, j, i| Cell::Sum {
            sheet: s,
            j: j as u8,
            i: i as u8,
        },
        j: fin,
    });
    rows.push(Row {
        name: EN.codex32_row_must_read.to_string(),
        start: sh.sum_box(fin),
        cells: sh.target().iter().map(|&v| Some(v)).collect(),
        look: Look::Fixed,
        cell: |s, j, i| Cell::Sum {
            sheet: s,
            j: j as u8,
            i: i as u8,
        },
        j: fin,
    });
    rows
}

/// The rows of a worksheet from box `from` to `to`, their names in the
/// left column, the box numbers over the first. `front` outlines that
/// box of the first row. Returns the height.
fn grid(
    rows: &[Row],
    sheet: u8,
    (from, to): (usize, usize),
    front: Option<usize>,
    ui: &mut Ui,
    at: At,
) -> f32 {
    let (x, y, cls) = (at.x, at.y, at.cls);
    let s = t::codex32_box(cls);
    let gx = x + t::CODEX32_LABEL_W;
    for n in (from..=to).filter(|n| (n - 1) % 4 == 0 || *n == from) {
        ui.text(
            box_x(gx, from, n, cls),
            y,
            t::CODEX32_NUMBER,
            W::R,
            MUTED,
            &n.to_string(),
        );
    }
    let mut cy = y + t::CODEX32_NUMBER_ROW;
    for (r, row) in rows.iter().enumerate() {
        if (r + 1 == rows.len() && row.look == Look::Given) || row.name == EN.codex32_row_final {
            ui.rule(
                gx,
                cy - t::CODEX32_ROW_GAP / 2.0,
                box_x(gx, from, to, cls) + s - gx,
                BORDER,
            );
        }
        let name = ui.fit(t::CAPTION, W::R, &row.name, t::CODEX32_LABEL_W - t::GAP);
        ui.text_mid(x, cy, s, t::CAPTION, W::R, MUTED, &name);
        for (i, v) in row.cells.iter().enumerate() {
            let b = row.start + i;
            if b < from || b > to {
                continue;
            }
            let look = row.look;
            let cell = (row.cell)(sheet, row.j, i);
            one_box(
                ui,
                at.to(box_x(gx, from, b, cls), cy, 0.0),
                *v,
                look,
                r == 0 && front == Some(b),
                Some(cell),
            );
        }
        cy += s + t::CODEX32_ROW_GAP;
    }
    cy - y
}

/// The window of boxes a sheet shows in `w`: all of them where they fit,
/// else a window round `front` (on `small`, two boxes either side).
fn window(chars: usize, front: Option<usize>, cls: WindowClass, w: f32) -> (usize, usize) {
    let fits = boxes_in(w - t::CODEX32_LABEL_W, 4, cls);
    let n = if cls == WindowClass::Compact {
        (2 * t::CODEX32_BAND_SIDE + 1).min(fits)
    } else {
        fits
    };
    let total = chars - 3;
    if n >= total {
        return (4, chars);
    }
    let at = front.unwrap_or(chars);
    let from = at.saturating_sub(n / 2).clamp(4, chars + 1 - n);
    (from, from + n - 1)
}

/// The checksum worksheet of share `share` during the entry: the rows
/// that hold the front column, at most [`t::CODEX32_BAND_ROWS`], the
/// string's row first, in the window round it (§6.4.3, §6.4.7).
fn checksum_band(work: &WorkSet, share: usize, front: Option<usize>, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(chars) = work.worksheet(share, Reveal::PaperForm).map(|s| s.chars()) else {
        return 0.0;
    };
    let mut rows = checksum_rows(work, share);
    let col = front.unwrap_or(chars);
    let mut band: Vec<Row> = Vec::new();
    for (r, row) in rows.drain(..).enumerate() {
        let holds = (row.start..row.start + row.cells.len()).contains(&col);
        if r == 0 || (holds && band.len() < t::CODEX32_BAND_ROWS) {
            band.push(row);
        }
    }
    let head = fill(EN.codex32_checksum_of, &[&share_name(work.scheme(), share)]);
    let hh = ui.wrap(x, y, w, t::LABEL, W::S, TEXT, &head) + t::GAP_SMALL;
    let win = window(chars, Some(col), cls, w);
    hh + grid(&band, share as u8, win, front, ui, at.to(x, y + hh, w)) + t::GAP
}

/// The whole checksum worksheet of share `share`, for the review (§6.5),
/// in the window `from`; its marking: "from your rolls" or "as typed".
fn checksum_whole(m: &Codex32Make, work: &WorkSet, share: usize, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(chars) = work.worksheet(share, Reveal::PaperForm).map(|s| s.chars()) else {
        return 0.0;
    };
    let rows = checksum_rows(work, share);
    let typed = m.made_of(share, 0).is_some_and(super::Made::typed);
    let head = fill(
        EN.codex32_checksum_of_marked,
        &[
            &share_name(work.scheme(), share),
            if typed {
                EN.codex32_as_typed
            } else {
                EN.codex32_from_your_rolls
            },
        ],
    );
    let hh = ui.wrap(x, y, w, t::LABEL, W::S, TEXT, &head) + t::GAP_SMALL;
    let mut cy = y + hh;
    // A sheet wider than the card is drawn in parts, one under another.
    let (mut from, mut parts) = (4, 0);
    while from <= chars && parts < 4 {
        let n = boxes_in(w - t::CODEX32_LABEL_W, from, cls);
        let to = (from + n - 1).min(chars);
        cy += grid(&rows, share as u8, (from, to), None, ui, at.to(x, cy, w)) + t::GAP;
        from = to + 1;
        parts += 1;
    }
    cy - y
}

// ---------------------------------------------------------------------
// Translation worksheets (§6.4.4, §7.2, §7.3)
// ---------------------------------------------------------------------

/// The `n`-th translation worksheet: its symbols, each input and its
/// translated row, the sum; in the window round `front` (the input
/// being entered and its box) where it does not fit.
fn translation(
    work: &WorkSet,
    n: usize,
    front: Option<(usize, usize)>,
    ui: &mut Ui,
    at: At,
) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(ts) = work.live_translation(n, Reveal::PaperForm) else {
        return 0.0;
    };
    let tg = ts.target_index();
    let head = fill(EN.codex32_translation_to, &[&target_name(tg)]);
    let mut cy = y + ui.wrap(x, y, w, t::LABEL, W::S, TEXT, &head) + t::GAP_SMALL;
    let k = ts.threshold();
    let mut syms = Vec::new();
    for i in 0..k {
        let input = char::from(ts.input_index(i).to_ascii_uppercase());
        syms.push(fill(
            EN.codex32_symbol_of,
            &[&input.to_string(), &glyph(ts.symbol(i))],
        ));
    }
    let label = if ts.is_recovery() {
        EN.codex32_recovery_symbols
    } else {
        EN.codex32_translation_symbols
    };
    cy += ui.wrap(
        x,
        cy,
        w,
        t::CAPTION,
        W::R,
        MUTED,
        &format!("{label}: {}", syms.join(" · ")),
    );
    cy += t::GAP_SMALL;
    let chars = work.scheme().length().chars();
    let mut rows = Vec::new();
    for i in 0..k {
        let input = char::from(ts.input_index(i).to_ascii_uppercase()).to_string();
        rows.push(Row {
            name: input.clone(),
            start: 4,
            cells: (4..=chars).map(|b| ts.given_cell(i, b)).collect(),
            look: Look::Given,
            cell: |s, j, i| Cell::Translated {
                sheet: s,
                i: j as u8,
                b: (i + 4) as u8,
            },
            j: i,
        });
        rows.push(Row {
            name: fill(EN.codex32_row_times, &[&input]),
            start: 4,
            cells: (4..=chars).map(|b| ts.translated_cell(i, b)).collect(),
            look: Look::Table,
            cell: |s, j, i| Cell::Translated {
                sheet: s,
                i: j as u8,
                b: (i + 4) as u8,
            },
            j: i,
        });
    }
    rows.push(Row {
        name: target_name(tg),
        start: 4,
        cells: (4..=chars).map(|b| ts.sum_cell(b)).collect(),
        look: Look::Given,
        cell: |s, _, i| Cell::TSum {
            sheet: s,
            b: (i + 4) as u8,
        },
        j: 0,
    });
    let win = window(chars, front.map(|f| f.1), cls, w);
    cy += grid(&rows, n as u8, win, None, ui, at.to(x, cy, w));
    cy - y + t::GAP
}

// ---------------------------------------------------------------------
// The dice de-biasing sheet (§6.4.2)
// ---------------------------------------------------------------------

/// The booklet's dice sheet for the character being rolled: five
/// tracks, each a free space and pads 1 to 6, the first value's marker
/// and the second value drawn on their pads, the row's words and its
/// bit; then the tree, its path drawn as far as the bits from die 1 run.
fn dice_sheet(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32, rows_only: bool) -> f32 {
    let mut cy = y;
    let head = EN.codex32_dice_sheet;
    cy += ui.wrap(x, cy, w, t::LABEL, W::S, TEXT, head) + t::GAP_SMALL;
    if rows_only {
        cy += dice_rows(m, ui, x, cy, w, false);
    } else {
        cy += tracks(m, ui, x, cy, w);
    }
    cy += rolled_line(m, ui, x, cy, w);
    cy += tree(m, ui, x, cy, w);
    cy - y + t::GAP
}

/// "Die 1, Die 3 rolled by this computer": the dice of the character in
/// progress whose value the roller made (§5.5), or nothing.
fn rolled_line(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let dice: Vec<String> = (0..BOOKLET_DICE)
        .filter(|&d| m.die_rolled(d))
        .map(|d| fill(EN.codex32_die_short, &[&(d + 1).to_string()]))
        .collect();
    if dice.is_empty() {
        return 0.0;
    }
    let line = fill(EN.codex32_dice_rolled, &[&dice.join(", ")]);
    ui.wrap(x, y, w, t::CAPTION, W::R, MUTED, &line) + t::GAP_SMALL
}

/// The line a die's row reads: "4 then 2 · lower · 0", or the first value
/// alone, or nothing.
fn die_line(m: &Codex32Make, die: usize) -> String {
    let d = m.dice();
    if m.masked() {
        return match (d.first(die), d.second(die)) {
            (Some(_), Some(_)) => EN.codex32_die_masked.to_string(),
            (Some(_), None) => MASK.to_string(),
            _ => String::new(),
        };
    }
    match (d.first(die), d.second(die)) {
        (Some(f), Some(s)) => {
            let bit = faraday_entropy::booklet::bit(f, s);
            fill(
                EN.codex32_die_then,
                &[
                    &f.to_string(),
                    &s.to_string(),
                    if bit == Some(0) {
                        EN.codex32_lower
                    } else {
                        EN.codex32_higher
                    },
                    &bit.unwrap_or(0).to_string(),
                ],
            )
        }
        (Some(f), None) => f.to_string(),
        _ => String::new(),
    }
}

/// The five dice as rows of words, "Die 3 · 4 then 2 · lower · 0", the
/// struck pairs under them (`mobile` and `small`). With `path`, the bits
/// as a line, "0 1 1 0 ·".
fn dice_rows(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32, path: bool) -> f32 {
    let mut cy = y;
    for die in 0..BOOKLET_DICE {
        let line = die_line(m, die);
        let on = m.dice().first(die).is_none() || m.dice().second(die).is_none();
        let text = fill(EN.codex32_die_n, &[&(die + 1).to_string()]);
        ui.text(x, cy, t::LABEL, W::S, if on { TEXT } else { MUTED }, &text);
        ui.text(x + t::CODEX32_LABEL_W, cy, t::LABEL, W::M, TEXT, &line);
        cy += ui.line(t::LABEL, W::R) + t::GAP_SMALL;
    }
    cy += struck(m, ui, x, cy, w);
    if path {
        let bits = known_bits(m, false);
        let s: Vec<String> = (0..BOOKLET_DICE)
            .map(|d| match bits[d] {
                Some(_) if m.masked() => MASK.to_string(),
                Some(b) => b.to_string(),
                None => "·".to_string(),
            })
            .collect();
        ui.text(x, cy, t::BODY, W::M, ACCENT, &s.join(" "));
        cy += ui.line(t::BODY, W::M) + t::GAP;
    }
    cy - y
}

/// The struck pairs of the character in progress: "Die 3 · 3 then 3 ·
/// roll both again", struck through.
fn struck(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for row in m.rows().iter().filter(|r| r.bit().is_none()) {
        let (f, sc) = if m.masked() {
            (MASK.to_string(), MASK.to_string())
        } else {
            (row.first.to_string(), row.second.to_string())
        };
        let s = fill(
            EN.codex32_die_equal,
            &[&(usize::from(row.die) + 1).to_string(), &f, &sc],
        );
        let tw = ui.text(x, cy, t::CAPTION, W::R, WARN, &s);
        let lh = ui.line(t::CAPTION, W::R);
        ui.fill(x, cy + lh / 2.0, tw.min(w), 1.0, 0.0, WARN);
        cy += lh + t::GAP_SMALL;
    }
    cy - y
}

/// Each die's bit, once its pair is in and not equal.
/// Where the sheets are masked, with `shown` the bits are none: the
/// tree draws no path.
fn known_bits(m: &Codex32Make, shown: bool) -> [Option<u8>; BOOKLET_DICE] {
    let mut out = [None; BOOKLET_DICE];
    if shown && m.masked() {
        return out;
    }
    for (d, slot) in out.iter_mut().enumerate() {
        *slot = m.dice().die_bit(d).flatten();
    }
    out
}

/// The five tracks as the booklet draws them: the die's number, the free
/// space and the pads 1 to 6; the first value's marker outlined, the
/// second filled; the row's words beside.
fn tracks(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let p = t::CODEX32_PAD;
    let mut cy = y;
    for die in 0..BOOKLET_DICE {
        let text = fill(EN.codex32_die_n, &[&(die + 1).to_string()]);
        ui.text_mid(x, cy, p, t::CAPTION, W::S, MUTED, &text);
        let tx = x + t::CODEX32_LABEL_W - t::GAP;
        let (f, s) = if m.masked() {
            (None, None)
        } else {
            (m.dice().first(die), m.dice().second(die))
        };
        for pad in 0..PADS {
            let px = tx + f32::from(pad) * p;
            ui.stroke(
                px,
                cy,
                p - t::CODEX32_BOX_GAP,
                p - t::CODEX32_BOX_GAP,
                t::CODEX32_RADIUS,
                INNER,
            );
            let label = if pad == 0 {
                String::new()
            } else {
                pad.to_string()
            };
            let lw = ui.measure(t::CAPTION, W::M, &label);
            ui.text_mid(
                px + (p - lw) / 2.0 - 1.0,
                cy,
                p,
                t::CAPTION,
                W::M,
                DIM,
                &label,
            );
            if f == Some(pad) || (pad == 0 && f.is_none()) {
                ui.stroke(
                    px,
                    cy,
                    p - t::CODEX32_BOX_GAP,
                    p - t::CODEX32_BOX_GAP,
                    t::CODEX32_RADIUS,
                    TEXT,
                );
            }
            if s == Some(pad) && pad > 0 {
                ui.fill(
                    px,
                    cy,
                    p - t::CODEX32_BOX_GAP,
                    p - t::CODEX32_BOX_GAP,
                    t::CODEX32_RADIUS,
                    ACCENT,
                );
                ui.text_mid(
                    px + (p - lw) / 2.0 - 1.0,
                    cy,
                    p,
                    t::CAPTION,
                    W::M,
                    ON_ACCENT,
                    &label,
                );
            }
        }
        let bit = m.dice().die_bit(die).flatten();
        if let Some(b) = bit {
            let text = if m.masked() {
                MASK.to_string()
            } else {
                b.to_string()
            };
            ui.text_mid(
                tx + f32::from(PADS) * p + t::GAP,
                cy,
                p,
                t::LABEL,
                W::M,
                ACCENT,
                &text,
            );
        }
        cy += p + t::GAP_SMALL;
    }
    cy += struck(m, ui, x, cy, w);
    cy - y
}

/// A track's pads: the free space and the values 1 to 6.
const PADS: u8 = 7;

/// The tree of 31 nodes over the 32 leaves, the leaves in the booklet's
/// order; the path from the root drawn in the accent for each bit known
/// from die 1 on, and the leaf reached named once die 5's bit is in.
fn tree(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let bits = known_bits(m, true);
    let lw = w / TREE.len() as f32;
    let lv = t::CODEX32_TREE_LEVEL;
    let node = |level: usize, idx: usize| -> (f32, f32) {
        let span = 32usize >> level;
        (
            x + (idx * span) as f32 * lw + span as f32 * lw / 2.0,
            y + level as f32 * lv,
        )
    };
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let faint = ui.col(INNER);
    let accent = ui.col(ACCENT);
    let mut at = 0usize;
    let mut path = true;
    for (level, bit) in bits.iter().enumerate() {
        for idx in 0..(1usize << level) {
            let (px, py) = node(level, idx);
            for c in 0..2 {
                let (cx, cyy) = node(level + 1, idx * 2 + c);
                ui.c.line(
                    (px + ox) * f,
                    (py + oy) * f,
                    (cx + ox) * f,
                    (cyy + oy) * f,
                    f,
                    faint,
                );
            }
        }
        match *bit {
            Some(b) if path => {
                let (px, py) = node(level, at);
                at = at * 2 + usize::from(b);
                let (cx, cyy) = node(level + 1, at);
                let sw = t::CODEX32_PATH_STROKE * f;
                ui.c.line(
                    (px + ox) * f,
                    (py + oy) * f,
                    (cx + ox) * f,
                    (cyy + oy) * f,
                    sw,
                    accent,
                );
            }
            _ => path = false,
        }
    }
    for level in 0..BOOKLET_DICE {
        for idx in 0..(1usize << level) {
            let (px, py) = node(level, idx);
            ui.dot(px, py, t::CODEX32_TREE_NODE, MUTED);
        }
    }
    let ly = y + BOOKLET_DICE as f32 * lv;
    for (i, &leaf) in TREE.iter().enumerate() {
        let s = char::from(leaf.to_ascii_uppercase()).to_string();
        let reached = path && i == at;
        let cw = ui.measure(t::CAPTION, W::M, &s);
        ui.text(
            x + i as f32 * lw + (lw - cw) / 2.0,
            ly,
            t::CAPTION,
            W::M,
            if reached { ACCENT } else { TEXT },
            &s,
        );
    }
    let mut h = BOOKLET_DICE as f32 * lv + ui.line(t::CAPTION, W::M) + t::GAP_SMALL;
    if path && bits.iter().all(Option::is_some) {
        let b: String = bits.iter().map(|b| b.unwrap_or(0).to_string()).collect();
        let s = fill(
            EN.codex32_path_leaf,
            &[&b, &at.to_string(), &char::from(TREE[at]).to_string()],
        );
        h += ui.wrap(x, y + h, w, t::LABEL, W::M, ACCENT, &s) + t::GAP_SMALL;
    }
    h
}

// ---------------------------------------------------------------------
// The review (§6.5)
// ---------------------------------------------------------------------

/// §6.5: the tab row (a Choice with Previous and Next on `small`), and
/// the worksheet of the tab shown.
fn sheets(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w) = (at.x, at.y, at.w);
    let Some(work) = m.work() else {
        return 0.0;
    };
    let tab_list = m.tabs();
    let (on, seen) = m.tab();
    let mut cy = y;
    let items: Vec<TabItem> = tab_list
        .iter()
        .enumerate()
        .map(|(i, (n, _))| TabItem {
            label: n.clone(),
            on: i == usize::from(on),
            seen: seen.get(i).copied().unwrap_or(false),
            action: a(C32::Tab(i as u8)),
        })
        .collect();
    cy += self::tabs(ui, x, cy, w, &items);
    cy += match tab_list.get(usize::from(on)).map(|t| t.1) {
        Some(Tab::Dice(s)) => dice_review(m, work, s, ui, x, cy, w),
        Some(Tab::Checksum(s)) => checksum_whole(m, work, s, ui, at.to(x, cy, w)),
        Some(Tab::Translation(n)) => translation(work, n, None, ui, at.to(x, cy, w)),
        None => 0.0,
    };
    cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    cy - y
}

/// The dice rows of every character of initial share `s`, struck pairs
/// included, each character's bits and its box.
fn dice_review(
    m: &Codex32Make,
    work: &WorkSet,
    s: usize,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    let data = work.scheme().length().data_len();
    for n in 0..data {
        let Some(made) = m.made_of(s, n) else {
            break;
        };
        if m.masked() {
            let line = fill(EN.codex32_box_masked, &[&(n + 10).to_string(), MASK]);
            cy += ui.wrap(x, cy, w, t::CAPTION, W::M, TEXT, &line) + t::GAP_SMALL;
            continue;
        }
        let rows: Vec<String> = made
            .rows()
            .iter()
            .map(|r| {
                let mark = match r.bit() {
                    Some(b) => b.to_string(),
                    None => "×".to_string(),
                };
                format!("{}:{}{} {mark}", r.die + 1, r.first, r.second)
            })
            .collect();
        let bits: String = made.bits().iter().map(u8::to_string).collect();
        let line = if made.typed() {
            fill(
                EN.codex32_box_typed,
                &[&(n + 10).to_string(), &char_of(made.value()).to_string()],
            )
        } else {
            fill(
                EN.codex32_box_rolled,
                &[
                    &(n + 10).to_string(),
                    &rows.join("  "),
                    &bits,
                    &char_of(made.value()).to_string(),
                ],
            )
        };
        let line = if m.char_rolled(s, n) {
            fill(EN.codex32_box_rolled_by, &[&line])
        } else {
            line
        };
        cy += ui.wrap(x, cy, w, t::CAPTION, W::M, TEXT, &line) + t::GAP_SMALL;
    }
    cy - y + t::GAP
}

// ---------------------------------------------------------------------
// The strings (§6.6) and Strings made (§6.7, §6.8)
// ---------------------------------------------------------------------

/// §6.6: the string in groups of four at the largest mono size that
/// fits, or its Codex32QR code; the typing back on the codex32 keyboard;
/// Continue and Skip.
fn strings(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(s) = m.strings().and_then(|s| s.get(m.at())) else {
        return 0.0;
    };
    let mut cy = y;
    let hide = m.masked();
    if s.len() == 48 && !hide {
        cy += choice(
            ui,
            x,
            cy,
            w,
            &[
                (EN.codex32_string, m.qr().is_none(), a(C32::Qr)),
                (EN.codex32_codex32qr, m.qr().is_some(), a(C32::Qr)),
            ],
        );
    }
    match m.qr().filter(|_| !hide) {
        Some(q) => {
            let side = t::qr_side(if cls == WindowClass::Compact {
                SizeClass::Small
            } else {
                SizeClass::Wide
            })
            .min(w);
            faraday_ui::parts::qr_code(ui, q, x, cy, side);
            cy += side + t::GAP;
        }
        None => {
            let g = if hide {
                faraday_ui::grouped(&MASK.repeat(s.len()))
            } else {
                faraday_ui::grouped(s)
            };
            let mut size = t::TITLE;
            while size > t::CAPTION && ui.wrap_lines(size, W::M, &g, w) > 3 {
                size -= 1.0;
            }
            cy += ui.wrap(x, cy, w, size, W::M, TEXT, &g) + t::GAP;
        }
    }
    let (typed, differs) = m.typed_back();
    let mut whole = String::from_utf8_lossy(PREFIX).into_owned();
    if hide {
        whole.push_str(&MASK.repeat(typed.len()));
    } else {
        whole.push_str(typed);
    }
    let shown = faraday_ui::grouped(&whole);
    ui.fill(x, cy, w, t::FIELD, t::RADIUS_SMALL, SURFACE);
    ui.stroke(
        x,
        cy,
        w,
        t::FIELD,
        t::RADIUS_SMALL,
        if differs.is_some() { ERR } else { BORDER },
    );
    let fit = ui.fit(t::LABEL, W::M, &shown, w - 2.0 * t::GAP);
    ui.text_mid(x + t::GAP, cy, t::FIELD, t::LABEL, W::M, TEXT, &fit);
    cy += t::FIELD + t::GAP;
    if let Some(b) = differs {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            ERR,
            &fill(EN.codex32_box_differs, &[&b.to_string()]),
        ) + t::GAP;
    }
    let (checked, ask) = m.checked();
    if checked.get(m.at()).copied().unwrap_or(false) {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, OK, EN.codex32_typed_back_same) + t::GAP;
    } else {
        cy += keyboard(ui, x, cy, w, KeyboardKind::Codex32);
    }
    if ask {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, WARN, EN.codex32_skip_caution) + t::GAP;
    }
    let nh = note(m, ui, x, cy, w);
    cy += nh;
    cy += crate::screens::buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[(EN.codex32_skip, Style::Ghost, a(C32::Skip))],
        Some((EN.keygen_continue, a(C32::Next))),
    );
    cy - y
}

/// §6.7: the Result's rows, Check on another device (§6.8), and Lock in.
fn made(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let (rolled, typed) = m.sources();
    // Faces the roller made say so (§5.5).
    let entropy = match (m.simulated(), rolled, typed) {
        (crate::fresh::roller::Sim::All, _, false) => EN.codex32_simulated_dice,
        (crate::fresh::roller::Sim::Some | crate::fresh::roller::Sim::All, _, _) => {
            EN.codex32_simulated_and_your_dice
        }
        (_, true, true) => EN.codex32_your_dice_and_characters,
        (_, false, true) => EN.codex32_your_characters,
        _ => EN.codex32_your_dice,
    };
    let fp = m
        .fingerprint()
        .map(|f| fp_text(faraday_bip::keys::Fingerprint(f)).to_string())
        .unwrap_or_default();
    let rows = [
        (EN.codex32_split, split_text(m)),
        (EN.codex32_length, m.length().chars().to_string()),
        (EN.codex32_identifier, m.identifier()),
        (EN.codex32_fingerprint, fp.clone()),
        (EN.codex32_entropy, entropy.to_string()),
    ];
    for (k, v) in rows {
        cy += fact(ui, x, cy, w, k, &v);
    }
    cy += wrap_buttons(
        ui,
        x,
        cy,
        w,
        t::TOUCH,
        &[(EN.codex32_check_on_another, Style::Secondary, a(C32::Check))],
    );
    if m.check_open() {
        cy += check_record(m, &fp, ui, x, cy, w);
    }
    let nh = note(m, ui, x, cy, w);
    cy += nh;
    // Lock in goes back to Create by itself for a slot; on its own the
    // flow ends on Done.
    let label = if m.locked() {
        EN.keygen_done
    } else {
        EN.keygen_lock_button
    };
    let act = if m.locked() {
        a(C32::Next)
    } else {
        a(C32::Lock)
    };
    let drawn = next_button(ui, x, cy, w, label, act);
    cy += if drawn { t::TOUCH } else { t::GAP };
    cy - y
}

/// A fact: its label over its value, the value in the mono face (every
/// value here is read character by character), a rule under it. Returns
/// the height.
fn fact(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, value: &str) -> f32 {
    ui.text(x, y, t::CAPTION, W::R, MUTED, label);
    let ly = ui.line(t::CAPTION, W::R) + t::GAP_SMALL;
    let vh = ui.wrap(x, y + ly, w, t::LABEL, W::M, TEXT, value);
    ui.rule(x, y + ly + vh + t::GAP_SMALL, w, INNER);
    ly + vh + t::GAP
}

/// §6.8: what any implementation must show for these strings, and the
/// sets to try.
fn check_record(m: &Codex32Make, fp: &str, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += fact(ui, x, cy, w, EN.codex32_fingerprint, fp);
    cy += fact(ui, x, cy, w, EN.codex32_identifier, &m.identifier());
    let (k, _) = m.k_n();
    if k == 0 {
        let s = m
            .strings()
            .and_then(|s| s.first())
            .map(|s| {
                if m.masked() {
                    MASK.repeat(s.len().min(12))
                } else {
                    faraday_ui::short(s.as_str())
                }
            })
            .unwrap_or_default();
        cy += fact(ui, x, cy, w, EN.codex32_the_secret, &s);
    } else {
        cy += fact(ui, x, cy, w, EN.codex32_split, &split_text(m));
        cy += fact(
            ui,
            x,
            cy,
            w,
            EN.codex32_sets_to_try,
            &m.sets_to_try().join(" · "),
        );
    }
    cy - y
}

// ---------------------------------------------------------------------
// The cell sheet (§7.4)
// ---------------------------------------------------------------------

/// The open cell's working: a dialog [`t::CHART_SHEET_WIDTH`] wide on a
/// wide window (the shared sheet look, over a scrim), a page of its own
/// with its own Back on a compact one. The Previous and Next buttons step
/// to the neighbouring cell of the row (§7.4). Every value read from the
/// kept worksheet.
pub(crate) fn cell_sheet(app: &View, m: &Codex32Make, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (Some(cell), Some(work)) = (m.cell(), m.work()) else {
        return;
    };
    let lines = working(m, work, cell);
    let head = cell_name(work, cell);
    if ui.compact {
        cell_page(app, ui, (x0, cw, h), &head, &lines);
        return;
    }
    let sw = t::CHART_SHEET_WIDTH.min(cw - 2.0 * t::PAD);
    let pad = t::PAD;
    let iw = sw - 2.0 * pad;
    let mut body_h = ui.wrap_lines(t::BODY, W::S, &head, iw - t::CODEX32_SHEET_ICON) as f32
        * ui.line(t::BODY, W::S)
        + t::GAP;
    for l in &lines {
        body_h +=
            ui.wrap_lines(t::LABEL, W::R, l, iw) as f32 * ui.line(t::LABEL, W::R) + t::GAP_SMALL;
    }
    let sh = (body_h + t::TOUCH + t::GAP + 2.0 * pad).min(h - 2.0 * t::GAP);
    ui.fill(x0, 0.0, cw, h, 0.0, BG.with_alpha(150));
    // A dialog: nothing under it is within reach, and a press beside it
    // closes it.
    ui.clear_hits();
    ui.hit_around(
        x0 + (cw - sw) / 2.0,
        (h - sh) / 2.0,
        sw,
        sh,
        a(C32::CellClose),
    );
    // The sheet over the page right of the sidebar: its box and what it
    // holds in one place.
    let (sx, sy) = faraday_ui::parts::sheet_box_in(ui, (x0, cw, h), sw, sh);
    let mut cy = sy + pad;
    cy += faraday_ui::sheet::sheet_head(ui, sx + pad, cy, iw, Icon::Info, ACCENT, &head);
    for l in &lines {
        cy += ui.wrap(sx + pad, cy, iw, t::LABEL, W::R, TEXT, l) + t::GAP_SMALL;
    }
    let by = sy + sh - pad - t::TOUCH;
    let bw = ui.measure(t::LABEL, W::S, EN.codex32_done_button) + 2.0 * t::PAD;
    ui.button(
        sx + sw - pad - bw,
        by,
        Some(bw),
        t::TOUCH,
        EN.codex32_done_button,
        Style::Primary,
        a(C32::CellClose),
    );
    step_buttons(ui, sx + pad, by, iw - bw - t::GAP);
}

/// Previous and Next, side by side from `x`, in `w` if they fit.
fn step_buttons(ui: &mut Ui, x: f32, y: f32, w: f32) {
    let items = [
        (EN.codex32_previous, a(C32::CellStep(-1))),
        (EN.codex32_next, a(C32::CellStep(1))),
    ];
    let mut bx = x;
    for (label, act) in items {
        let bw = ui.measure(t::LABEL, W::S, label) + 2.0 * t::PAD;
        if bx + bw > x + w + 0.5 {
            break;
        }
        bx += ui.button(bx, y, Some(bw), t::TOUCH, label, Style::Secondary, act) + t::GAP_SMALL;
    }
}

/// The working as a page of its own on a compact window: a bar with Back
/// to the worksheets, the cell's name, the working, and the steps to its
/// neighbours.
fn cell_page(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), head: &str, lines: &[String]) {
    use crate::compact::{BAR_H, M};
    // The page covers the flow: nothing under it is within reach, and its
    // pinned Continue goes.
    ui.clear_hits();
    ui.pin = None;
    ui.fill(x0, 0.0, cw, h, 0.0, SURFACE);
    crate::compact::bar(
        app,
        ui,
        cw,
        (EN.codex32_worksheets, a(C32::CellClose)),
        head,
    );
    let iw = cw - 2.0 * M;
    let mut cy = BAR_H + t::PAD;
    for l in lines {
        cy += ui.wrap(M, cy, iw, t::LABEL, W::R, TEXT, l) + t::GAP;
    }
    step_buttons(ui, M, (cy + t::GAP).min(h - t::TOUCH - t::GAP), iw);
}

/// The sheet's head: "Box 23 · sum row 7".
fn cell_name(work: &WorkSet, cell: Cell) -> String {
    let at = |sheet: u8, j: usize, i: u8, sum: bool| {
        work.worksheet(usize::from(sheet), Reveal::PaperForm)
            .map_or(
                0,
                |s| if sum { s.sum_box(j) } else { s.lookup_box(j) } + usize::from(i),
            )
    };
    match cell {
        Cell::Given { b, .. } => fill(EN.codex32_cell_box, &[&b.to_string()]),
        Cell::Hrp { i, .. } => fill(EN.codex32_cell_ms, &[&(usize::from(i) + 4).to_string()]),
        Cell::Sum { sheet, j, i } => fill(
            EN.codex32_cell_sum,
            &[
                &at(sheet, usize::from(j), i, true).to_string(),
                &(usize::from(j) + 1).to_string(),
            ],
        ),
        Cell::Lookup { sheet, j, i } => fill(
            EN.codex32_cell_lookup,
            &[
                &at(sheet, usize::from(j), i, false).to_string(),
                &(usize::from(j) + 1).to_string(),
            ],
        ),
        Cell::Symbol { .. } => EN.codex32_translation_symbols.to_string(),
        Cell::Translated { b, .. } | Cell::TSum { b, .. } => {
            fill(EN.codex32_cell_box, &[&b.to_string()])
        }
    }
}

/// "Sum row 4, box 38: 3": a cell named so it can be found on paper.
fn named(row: &str, b: usize, v: Option<u8>) -> String {
    fill(
        EN.codex32_named_cell,
        &[
            row,
            &b.to_string(),
            &v.map_or("·".to_string(), |v| char_of(v).to_string()),
        ],
    )
}

/// The working of `cell` as lines: its inputs, each naming its own
/// cell, and the rule (§7.4). Read from the kept worksheet: opening it
/// computes nothing.
fn working(m: &Codex32Make, work: &WorkSet, cell: Cell) -> Vec<String> {
    match cell {
        Cell::Given { sheet, b } => given_working(m, work, usize::from(sheet), usize::from(b)),
        Cell::Hrp { sheet, .. } => {
            let row: String = work
                .worksheet(usize::from(sheet), Reveal::PaperForm)
                .map(|s| s.hrp_row().iter().map(|&v| char_of(v)).collect())
                .unwrap_or_default();
            vec![EN.codex32_ms_residue.to_string(), row]
        }
        Cell::Sum { sheet, j, i } => {
            sum_working(work, usize::from(sheet), usize::from(j), usize::from(i))
        }
        Cell::Lookup { sheet, j, i } => {
            lookup_working(work, usize::from(sheet), usize::from(j), usize::from(i))
        }
        Cell::Symbol { sheet, i } => symbol_working(work, usize::from(sheet), usize::from(i)),
        Cell::Translated { sheet, i, b } => {
            let Some(ts) = work.live_translation(usize::from(sheet), Reveal::PaperForm) else {
                return Vec::new();
            };
            let (i, b) = (usize::from(i), usize::from(b));
            let sym = &glyph(ts.symbol(i));
            let input = char::from(ts.input_index(i).to_ascii_uppercase()).to_string();
            let show = |v: Option<u8>| v.map_or("·".to_string(), |v| char_of(v).to_string());
            let mut out = vec![fill(
                EN.codex32_translated_is,
                &[
                    &input,
                    &b.to_string(),
                    &show(ts.given_cell(i, b)),
                    sym,
                    &show(ts.translated_cell(i, b)),
                ],
            )];
            out.push(fill(EN.codex32_wheel_set_to, &[sym]));
            out.extend(symbol_working(work, usize::from(sheet), i));
            out
        }
        Cell::TSum { sheet, b } => {
            let Some(ts) = work.live_translation(usize::from(sheet), Reveal::PaperForm) else {
                return Vec::new();
            };
            let b = usize::from(b);
            let parts: Vec<String> = (0..ts.threshold())
                .map(|i| {
                    ts.translated_cell(i, b)
                        .map_or("·".to_string(), |v| char_of(v).to_string())
                })
                .collect();
            let sum = ts
                .sum_cell(b)
                .map_or("·".to_string(), |v| char_of(v).to_string());
            vec![
                fill(EN.codex32_sum_is, &[&parts.join(" + "), &sum]),
                EN.codex32_addition_wheel.to_string(),
            ]
        }
    }
}

/// A given box's working: the header as chosen, a data character's dice
/// rows and path (or "Typed"), a checksum character worked from below.
fn given_working(m: &Codex32Make, work: &WorkSet, share: usize, b: usize) -> Vec<String> {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return Vec::new();
    };
    let v = sh
        .symbol(b)
        .map_or("·".to_string(), |v| char_of(v).to_string());
    match b {
        4 => vec![fill(EN.codex32_header_threshold, &[&v])],
        5..=8 => vec![fill(EN.codex32_header_identifier, &[&m.identifier()])],
        9 => vec![fill(EN.codex32_header_index, &[&v])],
        _ if b <= sh.last_data_box() => match m.made_of(share, b - 10) {
            Some(made) if made.typed() => vec![EN.codex32_typed.to_string()],
            Some(made) => {
                let mut out: Vec<String> = made
                    .rows()
                    .iter()
                    .map(|r| {
                        let die = (usize::from(r.die) + 1).to_string();
                        match r.bit() {
                            Some(bit) => fill(
                                EN.codex32_die_row,
                                &[
                                    &die,
                                    &r.first.to_string(),
                                    &r.second.to_string(),
                                    &bit.to_string(),
                                ],
                            ),
                            None => fill(
                                EN.codex32_die_equal,
                                &[&die, &r.first.to_string(), &r.second.to_string()],
                            ),
                        }
                    })
                    .collect();
                let bits: String = made.bits().iter().map(u8::to_string).collect();
                out.push(fill(
                    EN.codex32_path_leaf,
                    &[
                        &bits,
                        &made.value().to_string(),
                        &char_of(made.value()).to_string(),
                    ],
                ));
                if m.char_rolled(share, b - 10) {
                    out.push(EN.roller_by_this_computer.to_string());
                }
                out
            }
            None => Vec::new(),
        },
        _ => {
            // Worked from below: the first sum row that holds this column
            // takes the character, with the lookup row above it.
            let steps = sh.steps();
            let len = sh.checksum_len();
            let Some(j) = (1..=steps).find(|&j| (sh.sum_box(j)..sh.sum_box(j) + len).contains(&b))
            else {
                return Vec::new();
            };
            let below = sh.sum_cell(j, b - sh.sum_box(j));
            let look = sh.lookup_cell(j - 1, b - sh.lookup_box(j - 1));
            let row = |n: usize| fill(EN.codex32_row_sum, &[&(n + 1).to_string()]);
            let lrow = fill(EN.codex32_row_lookup, &[&j.to_string()]);
            vec![
                named(&row(j), b, below),
                named(&lrow, b, look),
                fill(EN.codex32_this_is, &[&v]),
                EN.codex32_worked_from_below.to_string(),
            ]
        }
    }
}

/// A sum cell's working: the cell above (or the string's character) and
/// the lookup cell, added; the final row against what it must read.
fn sum_working(work: &WorkSet, share: usize, j: usize, i: usize) -> Vec<String> {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return Vec::new();
    };
    let b = sh.sum_box(j) + i;
    let this = sh.sum_cell(j, i);
    let mut out = Vec::new();
    if j == 0 {
        out.push(named(EN.codex32_row_string, b, sh.symbol(b)));
        out.push(named(EN.codex32_row_ms, b, sh.hrp_row().get(i).copied()));
    } else {
        let w = sh.step_width(j - 1);
        let above = if i + w < sh.checksum_len() {
            named(
                &fill(EN.codex32_row_sum, &[&j.to_string()]),
                b,
                sh.sum_cell(j - 1, i + w),
            )
        } else {
            named(EN.codex32_row_string, b, sh.symbol(b))
        };
        out.push(above);
        out.push(named(
            &fill(EN.codex32_row_lookup, &[&j.to_string()]),
            b,
            sh.lookup_cell(j - 1, i),
        ));
    }
    out.push(fill(
        EN.codex32_this_is,
        &[&this.map_or("·".to_string(), |v| char_of(v).to_string())],
    ));
    out.push(EN.codex32_addition_wheel.to_string());
    if j == sh.steps() {
        let want = char_of(sh.target()[i]).to_string();
        let holds = this == Some(sh.target()[i]);
        out.push(fill(
            if holds {
                EN.codex32_should_read_does
            } else {
                EN.codex32_should_read_not
            },
            &[&want.to_uppercase()],
        ));
    }
    out
}

/// A lookup cell's working: the leading pair of the sum row above that
/// selects the checksum table's row, and that row, this cell marked.
fn lookup_working(work: &WorkSet, share: usize, j: usize, i: usize) -> Vec<String> {
    let Some(sh) = work.worksheet(share, Reveal::PaperForm) else {
        return Vec::new();
    };
    let w = sh.step_width(j);
    let lead: Vec<String> = (0..w)
        .map(|n| {
            sh.sum_cell(j, n)
                .map_or("·".to_string(), |v| char_of(v).to_uppercase().to_string())
        })
        .collect();
    let lead = lead.join(" ");
    let row: Vec<String> = (0..sh.checksum_len())
        .map(|n| {
            let c = sh
                .lookup_cell(j, n)
                .map_or("·".to_string(), |v| char_of(v).to_string());
            if n == i { format!("[{c}]") } else { c }
        })
        .collect();
    vec![
        fill(EN.codex32_sum_starts, &[&(j + 1).to_string(), &lead]),
        fill(
            EN.codex32_table_row,
            &[&lead, &(i + 1).to_string(), &sh.checksum_len().to_string()],
        ),
        row.join(" "),
    ]
}

/// A translation symbol: its value and name, and the pair symbols it is
/// the fusion of (§7.3).
fn symbol_working(work: &WorkSet, sheet: usize, i: usize) -> Vec<String> {
    let Some(ts) = work.live_translation(sheet, Reveal::PaperForm) else {
        return Vec::new();
    };
    let v = ts.symbol(i);
    let mut out = vec![fill(EN.codex32_symbol_value, &[&glyph(v), &v.to_string()])];
    let pairs: Vec<String> = (0..ts.threshold())
        .filter(|&j| j != i)
        .map(|j| glyph(ts.pair(i, j)))
        .collect();
    let kind = if ts.is_recovery() {
        EN.codex32_recovery_symbols
    } else {
        EN.codex32_pair_symbols
    };
    out.push(fill(EN.codex32_fused_from, &[kind, &pairs.join(" × ")]));
    out
}
