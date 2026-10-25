//! Make codex32 shares' cards, drawn as cards of New key's column
//! (`docs/CODEX32.md` §6, §7): the scheme, the identifier, the entropy,
//! the entry with the booklet's sheets filling live, the worksheets read
//! back, the strings, and Strings made. Every value is read from the
//! flow's state and the work set it holds; nothing here computes.
//!
//! The entry lays out by the window's class (§6.4.7, `docs/LAYOUT.md`
//! §2): medium and up, the share's string over the dice sheet (drawn as
//! the booklet's page on expanded and up, `dice_screen.rs`) and the
//! worksheet the strip chose; compact, one context panel at a time,
//! chosen by a tile row over the pad.

use faraday_bip::codex32::SYMBOL_GLYPHS;
use faraday_draw::geom::{SizeClass, WindowClass};
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_keys::{Reveal, WorkSet};
use faraday_ui::keyboard::Keys as OskKeys;
use faraday_ui::parts::wrap_buttons;

use super::dice_screen::{dice_rows, dice_sheet, rolled_line};
use super::worksheet_screen as sheet;
use super::{C32, CARDS, Card, Cell, Codex32Make, Given, SPLITS, Split, Tab, WAYS, Way, char_of};
use crate::drawing::View;
use crate::keygen::{KeyGen, kstep};
use crate::screens::next_button;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Slot, Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, flow};

/// Boxes 1 to 3 of every string.
const PREFIX: &[u8] = b"ms1";

/// The identifier's boxes.
const ID_LEN: u8 = 4;

/// Symbol `v` of the booklet's second alphabet, drawn as its glyph
/// (§7.2).
pub(super) fn glyph(v: u8) -> String {
    SYMBOL_GLYPHS[usize::from(v & 31)].to_string()
}

/// A press of this flow.
pub(super) fn a(p: C32) -> Action {
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
    /// The "Demo only" card is a strip pinned at the window's foot: the
    /// entry leaves room under its last row for it.
    strip: bool,
    /// Where the input panel stands (`docs/LAYOUT.md` §16.3): out of the
    /// page, the page draws neither the pad nor the rolls, and leaves
    /// room under its last row for the panel at the foot.
    panel: super::input_panel::PanelAt,
    /// A press on a box opens its working (§7.4): not on Check a
    /// string's sheet, which is no flow's.
    press: bool,
    /// The work width from `x` (`docs/LAYOUT.md` §14): wide content, the
    /// string's boxes and the sheets, is drawn in it.
    wide: f32,
    /// The dice sheet stands in a column at the right, not in the page
    /// (`docs/LAYOUT.md` §16.1).
    column: bool,
}

/// What a masked character is drawn as.
pub(super) const MASK: &str = "\u{2022}";

impl At {
    /// The same place moved to (x, y), `w` wide.
    fn to(self, x: f32, y: f32, w: f32) -> At {
        At { x, y, w, ..self }
    }
}

/// The flow's cards for New key's column, each with the card it is.
pub(crate) fn cards(k: &KeyGen, m: &Codex32Make) -> Vec<(Card, flow::Card)> {
    let open = |card: Card| k.open == Some(kstep::CODEX32) && m.open() == Some(card);
    // A set typed in has its identifier, entropy and entry in the strings
    // (§8.1, §8.2): its cards are the first one, then the review on.
    let shown = |card: &&Card| {
        m.given_kind().is_none() || !matches!(card, Card::Identifier | Card::Entropy | Card::Enter)
    };
    CARDS
        .iter()
        .filter(shown)
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
        Card::Scheme if m.given_kind() == Some(Given::Add) => EN.c32r_index.to_string(),
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
            &[&share_name(w, s), &(n + 1).to_string(), &data.to_string()],
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
    // A string made from strings typed in is named by its index: the
    // others of the set are on paper already (§8.1, §8.2).
    if m.open() == Some(Card::Strings) && m.given_kind().is_some() {
        let index = m
            .strings()
            .and_then(|s| s.get(m.at()))
            .and_then(|s| s.as_bytes().get(8).copied())
            .unwrap_or(b'?');
        return fill(
            EN.codex32_share_x,
            &[&char::from(index.to_ascii_uppercase()).to_string()],
        );
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

/// "Share A", or "Secret" with no threshold or for the secret: input `s`.
pub(super) fn share_name(work: &WorkSet, s: usize) -> String {
    let i = work.inputs().get(s).copied().unwrap_or(b'a');
    if work.scheme().threshold() == 0 || i == b's' {
        return EN.codex32_the_secret.to_string();
    }
    fill(
        EN.codex32_share_x,
        &[&char::from(i.to_ascii_uppercase()).to_string()],
    )
}

/// The split's words: "2 of 3", or "None, the secret alone".
fn split_text(m: &Codex32Make) -> String {
    match m.k_n() {
        (0, _) => EN.codex32_split_alone.to_string(),
        // Add a share does not know how many strings the set has.
        (k, _) if m.given_kind() == Some(Given::Add) => fill(EN.c32r_k_needed, &[&k.to_string()]),
        (k, n) => fill(EN.codex32_k_of_n, &[&k.to_string(), &n.to_string()]),
    }
}

fn summary(m: &Codex32Make, card: Card) -> String {
    if !m.done(card) {
        return EN.keygen_not_yet.to_string();
    }
    match card {
        Card::Scheme if m.given_kind() == Some(Given::Add) => fill(
            EN.c32r_index_value,
            &[&char::from(m.add_at().to_ascii_uppercase()).to_string()],
        ),
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
        Way::Shares => EN.c32r_type_shares_by_hand,
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
pub(super) fn choice(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, bool, Action)]) -> f32 {
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

/// Secondary tabs, [`t::CODEX32_TAB`] tall, an indicator in `primary`
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
    ui.rule(x, y + t::CODEX32_TAB - 1.0, w, Role::OutlineVariant);
    let mut tx = x;
    for (i, item) in items.iter().enumerate().skip(first) {
        let tw = if fixed { equal } else { widths[i] };
        if tx + tw > x + w + 0.5 {
            break;
        }
        if ui.is_pressed(item.action) {
            ui.wash(
                tx,
                y,
                tw,
                t::CODEX32_TAB,
                0.0,
                Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
            );
        }
        let colour = if item.on {
            Role::Primary
        } else {
            Role::OnSurfaceVariant
        };
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
                Role::Ok,
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
                Role::Primary,
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
    (card, pane, wide, panel, column): (
        Card,
        Option<super::pane_screen::PaneAt>,
        f32,
        super::input_panel::PanelAt,
        bool,
    ),
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
        strip: pane == Some(super::pane_screen::PaneAt::Strip),
        panel,
        press: true,
        // One-column pages lay a step out in the page's own width.
        wide: if app.is_compact() { w } else { wide.max(w) },
        column,
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

/// The note under a card's controls, in `error`, if any. Returns the
/// height.
fn note(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match m.note() {
        Some(n) => ui.wrap(x, y, w, t::LABEL, W::R, Role::Error, n) + t::GAP,
        None => 0.0,
    }
}

/// Continue at the card's foot, under its note. Returns the height.
fn foot(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32, label: &str) -> f32 {
    let nh = note(m, ui, x, y, w);
    let drawn = next_button(ui, x, y + nh, w, label, a(C32::Next));
    nh + if drawn { t::TOUCH } else { t::GAP }
}

/// The first card of a set typed in (§8.1, §8.2): the strings it is made
/// from, then how many strings in all (Make the rest) or the index (Add a
/// share, typed on the codex32 keyboard over the one offered).
fn given_first(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let from: Vec<String> = m
        .input_indices()
        .iter()
        .map(|&i| {
            if i == b's' {
                EN.codex32_the_secret.to_string()
            } else {
                char::from(i.to_ascii_uppercase()).to_string()
            }
        })
        .collect();
    cy += fact(ui, x, cy, w, EN.c32r_from, &from.join(", "));
    cy += fact(
        ui,
        x,
        cy,
        w,
        EN.codex32_length,
        &m.length().chars().to_string(),
    );
    let open = m.work().is_none();
    if m.given_kind() == Some(Given::Add) {
        let at = char::from(m.add_at().to_ascii_uppercase()).to_string();
        cy += fact(ui, x, cy, w, EN.c32r_index, &at);
        let (line, tone) = if m.add_at_taken() {
            (EN.c32r_index_taken, Role::Error)
        } else {
            (EN.c32r_give_index, Role::OnSurfaceVariant)
        };
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, tone, line) + t::GAP;
        if open {
            cy += keyboard(ui, x, cy, w, KeyboardKind::Codex32);
        }
    } else {
        let (k, n) = m.k_n();
        cy += fact(ui, x, cy, w, EN.codex32_threshold, &k.to_string());
        crate::screens::stepper(
            ui,
            x,
            cy,
            w,
            EN.codex32_strings,
            usize::from(n),
            a(C32::N(-1)),
            a(C32::N(1)),
            open,
        );
        cy += t::STEPPER_ROW;
    }
    cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    cy - y
}

/// §6.1: length and split, Other's steppers, the line for k of 4 or more.
fn scheme(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if m.given_kind().is_some() {
        return given_first(m, ui, x, y, w);
    }
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
            Role::OnSurfaceVariant,
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
        ui.fill(bx, cy, b, b, t::RADIUS_SMALL, Role::Surface);
        ui.stroke(
            bx,
            cy,
            b,
            b,
            t::RADIUS_SMALL,
            if on {
                Role::Primary
            } else {
                Role::OutlineVariant
            },
        );
        if let Some(c) = id.chars().nth(i) {
            let s = c.to_string();
            let cw = ui.measure(t::TITLE, W::M, &s);
            ui.text_mid(
                bx + (b - cw) / 2.0,
                cy,
                b,
                t::TITLE,
                W::M,
                Role::OnSurface,
                &s,
            );
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
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, Role::Error, r) + t::GAP;
    }
    if m.work().is_none() {
        cy += keyboard(ui, x, cy, w, KeyboardKind::Codex32);
    }
    cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    cy - y
}

/// §6.3: the way. The dice's size and colours, chosen here at every size;
/// the entry's input panel follows the size.
fn entropy(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let items: Vec<(&str, bool, Action)> = WAYS
        .iter()
        .enumerate()
        .map(|(i, &wy)| (way_name(wy), m.way() == wy, a(C32::Way(i as u8))))
        .collect();
    cy += choice(ui, x, cy, w, &items);
    if m.way() == Way::Typed {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            Role::OnSurfaceVariant,
            EN.codex32_not_chosen,
        ) + t::GAP;
    }
    // The dice's size is chosen here at every size (owner, 2026-10-25).
    if m.way() == Way::Dice {
        cy += super::dice_screen::chooser(m, ui, (x, cy, w));
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
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            Role::OnSurfaceVariant,
            EN.codex32_not_chosen,
        ) + t::GAP;
    }
    // The computer's dice roller and Demo's sample are not the person's
    // entry: the "Demo only" card holds its place beside the page, or at
    // the window's foot (`pane_screen.rs`; owner, 2026-10-24; §5.5).
    // The share being entered, or the one whose character the
    // step-through shows in its box.
    let (share, front) = m
        .entry_share()
        .unwrap_or((work.inputs().len().saturating_sub(1), None));
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
                // On the small panel the tour's sheet is the Checksum
                // tile's while the tour shows a step.
                1 => match m.tour_shown() {
                    Some((_, s)) => tour_sheets(m, work, s.sheet(), None, ui, at.to(x, cy, w)),
                    None => sheet::entry_sheet(m, work, share, front, ui, (x, cy, w, cls, at.hide)),
                },
                3 => {
                    let h = super::tracker_screen::draw(m, ui, (x, cy, w));
                    h + super::tour_screen::choice(m, ui, (x, cy + h, w)) + t::GAP
                }
                _ => string_boxes(work, share, front, ui, at.to(x, cy, w)),
            };
            cy += pad(m, ui, at.to(x, cy, w));
        }
        _ => {
            cy += string_boxes(work, share, front, ui, at.to(x, cy, w));
            // The tabs over the sheets span the work width, as they do.
            cy += view_strip(m, work, ui, x, cy, at.wide);
            cy += panels(m, work, share, front, ui, at.to(x, cy, w));
            // Out of the page, the input panel holds the Working area, the
            // pad, the rolls and Continue.
            if !at.panel.out_of_page() {
                let ww = w.min(t::COLUMN_MAX);
                cy += super::tracker_screen::draw(m, ui, (x, cy, ww));
                cy += super::tour_screen::choice(m, ui, (x, cy, ww)) + t::GAP;
                cy += pad(m, ui, at.to(x, cy, w));
            }
        }
    }
    // Continue is there once the sheets are complete: until then it would
    // only take the room the panel and the pad need. Out of the page, it
    // stands in the input panel, in the pad's place.
    if m.strings().is_some() && !at.panel.out_of_page() {
        cy += foot(m, ui, x, cy, w, EN.keygen_continue);
    } else {
        cy += note(m, ui, x, cy, w);
    }
    // Room under the last row for the strip at the window's foot, so the
    // page scrolls clear of it.
    if at.strip {
        cy += super::pane_screen::strip_h(ui, m, w, at.roller);
    }
    cy += at.panel.foot_h();
    cy - y
}

/// The small panel's tile row: Dice · Checksum · String (§6.4.7).
fn panel_tiles(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let names = [
        EN.codex32_tile_dice,
        EN.codex32_tile_checksum,
        EN.codex32_tile_string,
        EN.codex32_track_head,
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
pub(super) fn target_name(tg: u8) -> String {
    if tg == b's' {
        EN.codex32_the_secret.to_string()
    } else {
        fill(
            EN.codex32_share_x,
            &[&char::from(tg.to_ascii_uppercase()).to_string()],
        )
    }
}

/// The dice sheet over the worksheet the strip chose. The dice sheet
/// takes the card's width, as the booklet's page does: on expanded and
/// up its tree beside each die's two rolls and its tracks under them;
/// narrower, the dice as rows of words over the tree.
fn panels(
    m: &Codex32Make,
    work: &WorkSet,
    share: usize,
    front: Option<usize>,
    ui: &mut Ui,
    at: At,
) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.wide, at.cls);
    let dice = |ui: &mut Ui, y: f32| {
        if m.dice_sheet_shown() && !at.column {
            let rolls = !at.panel.out_of_page();
            dice_sheet(m, ui, (x, y, w), cls < WindowClass::Expanded, rolls)
        } else {
            0.0
        }
    };
    let sheet = |ui: &mut Ui, y: f32| match m.view() {
        0 => sheet::entry_sheet(m, work, share, front, ui, (x, y, w, cls, at.hide)),
        v => tour_sheets(
            m,
            work,
            usize::from(v) - 1,
            front.map(|f| (share, f)),
            ui,
            at.to(x, y, w),
        ),
    };
    // The sheet of the work in hand first, so the page scrolls the least
    // to it: the dice sheet while a character is rolled, the worksheet
    // while the step-through works the checksum (owner, 2026-10-24).
    if m.walk_checksum() {
        let sh = sheet(ui, y);
        sh + dice(ui, y + sh)
    } else {
        let dh = dice(ui, y);
        dh + sheet(ui, y + dh)
    }
}

/// A share made's (or S's) tab on the entry: its translation worksheet,
/// the tour's step marked on it; while the tour works the share's
/// checksum check, the check first, as the work in hand (§5.7).
fn tour_sheets(
    m: &Codex32Make,
    work: &WorkSet,
    n: usize,
    front: Option<(usize, usize)>,
    ui: &mut Ui,
    at: At,
) -> f32 {
    let (x, y, w) = (at.x, at.y, at.w);
    let mut cy = y;
    if m.tour_on_check() == Some(n) {
        let hide = at.hide;
        cy += super::tour_screen::check_sheet(m, n, ui, (x, cy, w, at.cls, hide));
    }
    let marks = m.tour_cells(n);
    cy += translation(work, n, front, &marks, ui, at.to(x, cy, w));
    cy - y
}

/// The pad: the dice pad's six faces and backspace, or a key per face of
/// other dice, or the codex32 keyboard for characters typed. On Faraday
/// Desktop the dice pad is headed "Your dice"; the computer's roller is
/// in the "Demo only" box (§5.5).
fn pad(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let (x, y, w) = (at.x, at.y, at.w);
    if m.front().is_none() {
        return 0.0;
    }
    if m.way() == Way::Typed {
        return keyboard(ui, x, y, w, KeyboardKind::Codex32);
    }
    // Six-faced dice keep the six-key pad; other dice a key per face.
    let six = m.dice_choice().faces == 6;
    let dice_pad = |ui: &mut Ui, x: f32, y: f32, w: f32| {
        if six {
            keyboard(ui, x, y, w, KeyboardKind::Dice)
        } else {
            super::dice_screen::face_pad(m, ui, x, y, w)
        }
    };
    // A small panel keeps its room for the sheet and the pad. Elsewhere
    // the pad is the input panel's; the dice are chosen on the Entropy
    // card at every size (`docs/LAYOUT.md` §16.3).
    if at.cls == WindowClass::Compact {
        return dice_pad(ui, x, y, w);
    }
    super::input_panel::page_pad(m, ui, (x, y, w)) + t::GAP
}

// ---------------------------------------------------------------------
// The string's boxes
// ---------------------------------------------------------------------

/// Where box `b` is drawn when the boxes from `from` start at `x`, each
/// `side` dp: the box pitch, and a group gap before each group of four
/// (groups start at box 1).
fn box_x(x: f32, from: usize, b: usize, side: f32) -> f32 {
    let pitch = side + t::CODEX32_BOX_GAP;
    let groups = (b - 1) / 4 - (from - 1) / 4;
    x + (b - from) as f32 * pitch + groups as f32 * (t::CODEX32_GROUP_GAP - t::CODEX32_BOX_GAP)
}

/// How many boxes `side` dp from `from` fit in `w`.
fn boxes_in(w: f32, from: usize, side: f32) -> usize {
    let mut n = 0;
    while box_x(0.0, from, from + n, side) + side <= w {
        n += 1;
    }
    n.max(1)
}

/// How a string's boxes are laid out (`docs/LAYOUT.md` §16.2): a box's
/// side, its character's size and its number's, and the boxes a row.
#[derive(Clone, Copy)]
struct Boxes {
    side: f32,
    ch: f32,
    num: f32,
    per: usize,
}

/// The string's boxes in `w` on a window of class `cls`: the first of
/// 24, 16, 12, 8 or 4 a row that fits at `CODEX32_STRING_BOX_MIN`, each
/// box as large as the row's room gives up to `CODEX32_STRING_BOX_MAX`;
/// on compact windows, as many whole groups of four as fit at the
/// class's box (`docs/LAYOUT.md` §16.2). Arithmetic.
fn string_layout(w: f32, cls: WindowClass) -> Boxes {
    if cls != WindowClass::Compact {
        for &per in &t::CODEX32_STRING_ROWS {
            // The row's gaps: between boxes, and wider between groups.
            let gaps = box_x(0.0, 1, per, 0.0);
            let side = ((w - gaps) / per as f32)
                .floor()
                .min(t::CODEX32_STRING_BOX_MAX);
            if side >= t::CODEX32_STRING_BOX_MIN {
                return Boxes {
                    side,
                    ch: side * t::CODEX32_STRING_CHAR_PART,
                    num: (side * t::CODEX32_STRING_NUMBER_PART).max(t::TEXT_MIN),
                    per,
                };
            }
        }
    }
    let side = t::codex32_box(cls);
    Boxes {
        side,
        ch: t::codex32_char(cls),
        num: t::CODEX32_STRING_NUMBER,
        per: (boxes_in(w, 1, side) / 4 * 4).max(4),
    }
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
fn one_box(
    ui: &mut Ui,
    at: At,
    (s, size): (f32, f32),
    v: Option<u8>,
    look: Look,
    front: bool,
    cell: Option<Cell>,
) {
    let (bx, y) = (at.x, at.y);
    let fill_c = match look {
        Look::Table => Role::SurfaceContainerHighest,
        Look::Checksum => Role::ChecksumContainer,
        Look::Given | Look::Fixed => Role::Surface,
    };
    ui.fill(bx, y, s, s, t::CODEX32_RADIUS, fill_c);
    let edge = match look {
        Look::Given | Look::Checksum => Role::OnSurface,
        _ => Role::OutlineVariant,
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
            Role::Primary,
        );
        ui.stroke(
            bx - o / 2.0,
            y - o / 2.0,
            s + o,
            s + o,
            t::CODEX32_RADIUS,
            Role::Primary,
        );
    }
    if let Some(v) = v {
        let ch = if at.hide {
            MASK.to_string()
        } else {
            char_of(v).to_string()
        };
        let weight = if matches!(look, Look::Checksum | Look::Fixed) {
            W::S
        } else {
            W::M
        };
        let cw = ui.measure(size, weight, &ch);
        let ink = if look == Look::Checksum {
            Role::OnChecksumContainer
        } else {
            Role::OnSurface
        };
        ui.text_mid(bx + (s - cw) / 2.0, y, s, size, weight, ink, &ch);
        // The press on a box opens the working of the nearest one: each
        // box's hit takes the gaps round it, so the boxes tile the sheet
        // (§7.4). A masked sheet opens nothing.
        if let Some(cell) = cell.filter(|_| !at.hide && at.press) {
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

/// The share's string in boxes, each numbered, groups of four, `ms1` first,
/// wrapping to the width: the header outlined, the checksum pink, the
/// front box in the accent (§6.4.1).
fn string_boxes(work: &WorkSet, share: usize, front: Option<usize>, ui: &mut Ui, at: At) -> f32 {
    let (x, y, cls) = (at.x, at.y, at.cls);
    let Some(sheet) = work.worksheet(share, Reveal::PaperForm) else {
        return 0.0;
    };
    let chars = sheet.chars();
    let last_data = sheet.last_data_box();
    // Wide content: as many a row, and as large, as the work width gives.
    let bx = string_layout(at.wide, cls);
    let (s, per) = (bx.side, bx.per);
    let size = (s, bx.ch);
    let number_row = ui.line(bx.num, W::R).max(t::CODEX32_NUMBER_ROW);
    let mut cy = y;
    let mut b = 1;
    while b <= chars {
        let to = (b + per - 1).min(chars);
        // Every box carries its number, centred over it, smaller than
        // the characters so the row of numbers stays quiet.
        for n in b..=to {
            let label = n.to_string();
            let nw = ui.measure(bx.num, W::R, &label);
            let nx = box_x(x, b, n, s) + (s - nw) / 2.0;
            ui.text(nx, cy, bx.num, W::R, Role::OnSurfaceVariant, &label);
        }
        let ry = cy + number_row;
        for n in b..=to {
            let bx = box_x(x, b, n, s);
            if n <= 3 {
                one_box(ui, at.to(bx, ry, 0.0), size, None, Look::Fixed, false, None);
                let ch = char::from(PREFIX[n - 1]).to_string();
                let cw = ui.measure(size.1, W::S, &ch);
                ui.text_mid(
                    bx + (s - cw) / 2.0,
                    ry,
                    s,
                    size.1,
                    W::S,
                    Role::OnSurface,
                    &ch,
                );
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
            one_box(
                ui,
                at.to(bx, ry, 0.0),
                size,
                v,
                look,
                front == Some(n),
                cell,
            );
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
    /// Which cell each is, for the tour's marks: an initial share's
    /// character is that share's box.
    id: fn(u8, usize, usize) -> Cell,
    /// The row's own number (the step), handed to `cell`.
    j: usize,
}

/// The rows of a worksheet from box `from` to `to`, their names in the
/// left column, the box numbers over the first. The cells in `marks`
/// are outlined in the accent, the one worked out more heavily, and
/// where they stand is said for the page to follow (`Slot::DiceWork`).
/// Returns the height.
fn grid(
    rows: &[Row],
    sheet: u8,
    (from, to): (usize, usize),
    marks: &[(Cell, bool)],
    ui: &mut Ui,
    at: At,
) -> f32 {
    let mut spot: Option<(f32, f32, f32, f32)> = None;
    let (x, y, cls) = (at.x, at.y, at.cls);
    let s = t::codex32_box(cls);
    let gx = x + t::CODEX32_LABEL_W;
    for n in (from..=to).filter(|n| (n - 1) % 4 == 0 || *n == from) {
        ui.text(
            box_x(gx, from, n, s),
            y,
            t::CODEX32_NUMBER,
            W::R,
            Role::OnSurfaceVariant,
            &n.to_string(),
        );
    }
    let mut cy = y + t::CODEX32_NUMBER_ROW;
    for (r, row) in rows.iter().enumerate() {
        if r + 1 == rows.len() && row.look == Look::Given {
            ui.rule(
                gx,
                cy - t::CODEX32_ROW_GAP / 2.0,
                box_x(gx, from, to, s) + s - gx,
                Role::OutlineVariant,
            );
        }
        let name = ui.fit(t::CAPTION, W::R, &row.name, t::CODEX32_LABEL_W - t::GAP);
        ui.text_mid(x, cy, s, t::CAPTION, W::R, Role::OnSurfaceVariant, &name);
        for (i, v) in row.cells.iter().enumerate() {
            let b = row.start + i;
            if b < from || b > to {
                continue;
            }
            let look = row.look;
            let cell = (row.cell)(sheet, row.j, i);
            let bx = box_x(gx, from, b, s);
            let mark = marks.iter().find(|m| m.0 == (row.id)(sheet, row.j, i));
            one_box(
                ui,
                at.to(bx, cy, 0.0),
                (s, t::codex32_char(cls)),
                *v,
                look,
                mark.is_some_and(|m| m.1),
                Some(cell),
            );
            if let Some(&(_, main)) = mark {
                if !main {
                    let o = t::CODEX32_FRONT_STROKE / 2.0;
                    let (w, r) = (s + 2.0 * o, t::CODEX32_RADIUS);
                    ui.stroke(bx - o, cy - o, w, w, r, Role::Primary);
                }
                let here = (bx, cy, s, s);
                spot = Some(spot.map_or(here, |sp| sheet::union(sp, here)));
            }
        }
        cy += s + t::CODEX32_ROW_GAP;
    }
    if let Some((sx, sy, sw, sh)) = spot {
        let r = ui.rect(sx, sy, sw, sh);
        ui.report_scroll_in(Slot::DiceWork, r, 0.0, 0.0);
    }
    cy - y
}

/// The window of boxes a sheet shows in `w`: all of them where they fit,
/// else a window round `front` (on `small`, two boxes either side).
fn window(chars: usize, front: Option<usize>, cls: WindowClass, w: f32) -> (usize, usize) {
    let fits = boxes_in(w - t::CODEX32_LABEL_W, 4, t::codex32_box(cls));
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

// ---------------------------------------------------------------------
// Translation worksheets (§6.4.4, §7.2, §7.3)
// ---------------------------------------------------------------------

/// The `n`-th translation worksheet: its symbols, each input and its
/// translated row, the sum; in the window round `front` (the input
/// being entered and its box) where it does not fit. The tour's step
/// (`tour.rs`) marks its cells, and the window stands round them.
fn translation(
    work: &WorkSet,
    n: usize,
    front: Option<(usize, usize)>,
    marks: &[(Cell, bool)],
    ui: &mut Ui,
    at: At,
) -> f32 {
    let (x, y, w, cls) = (at.x, at.y, at.w, at.cls);
    let Some(ts) = work.live_translation(n, Reveal::PaperForm) else {
        return 0.0;
    };
    let tg = ts.target_index();
    let head = fill(EN.codex32_translation_to, &[&target_name(tg)]);
    let mut cy = y + ui.wrap(x, y, w, t::LABEL, W::S, Role::OnSurface, &head) + t::GAP_SMALL;
    let k = ts.threshold();
    let label = if ts.is_recovery() {
        EN.codex32_recovery_symbols
    } else {
        EN.codex32_translation_symbols
    };
    cy += symbols_line(
        ui,
        (x, cy, w),
        label,
        n as u8,
        marks,
        |i| {
            let input = char::from(ts.input_index(i).to_ascii_uppercase());
            fill(
                EN.codex32_symbol_of,
                &[&input.to_string(), &glyph(ts.symbol(i))],
            )
        },
        k,
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
            id: |_, j, i| Cell::Given {
                sheet: j as u8,
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
            id: |s, j, i| Cell::Translated {
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
        id: |s, _, i| Cell::TSum {
            sheet: s,
            b: (i + 4) as u8,
        },
        j: 0,
    });
    // The window stands round the box the tour's step is on, else the
    // entry's front.
    let marked = marks.iter().find_map(|(c, _)| match *c {
        Cell::Translated { b, .. } | Cell::TSum { b, .. } => Some(usize::from(b)),
        // A symbol: the sheet from its first box.
        Cell::Symbol { .. } => Some(4),
        _ => None,
    });
    let win = window(chars, marked.or(front.map(|f| f.1)), cls, w);
    cy += grid(&rows, n as u8, win, marks, ui, at.to(x, cy, w));
    cy - y + t::GAP
}

/// The symbols' line of translation worksheet `sheet`: its label, then
/// each initial share's symbol ("A Δ"), wrapping in `w`; the symbol the
/// tour's step marks outlined in the accent and said for the page to
/// follow. Returns the height.
fn symbols_line(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    label: &str,
    sheet: u8,
    marks: &[(Cell, bool)],
    each: impl Fn(usize) -> String,
    k: usize,
) -> f32 {
    let (size, ink) = (t::CAPTION, Role::OnSurfaceVariant);
    let line = ui.line(size, W::R);
    let head = format!("{label}:");
    ui.text(x, y, size, W::R, ink, &head);
    let (mut cx, mut cy) = (x + ui.measure(size, W::R, &head) + t::GAP, y);
    for i in 0..k {
        let text = each(i);
        let tw = ui.measure(size, W::S, &text);
        if cx > x && cx + tw > x + w {
            cx = x;
            cy += line + t::GAP_SMALL;
        }
        let on = marks.contains(&(Cell::Symbol { sheet, i: i as u8 }, true));
        let role = if on { Role::Primary } else { Role::OnSurface };
        ui.text(cx, cy, size, W::S, role, &text);
        if on {
            let o = t::GAP_SMALL;
            let (bx, by, bw, bh) = (cx - o, cy - o / 2.0, tw + 2.0 * o, line + o);
            ui.stroke(bx, by, bw, bh, t::CODEX32_RADIUS, Role::Primary);
            let r = ui.rect(bx, by, bw, bh);
            ui.report_scroll_in(Slot::DiceWork, r, 0.0, 0.0);
        }
        cx += tw + t::SPACER;
    }
    cy + line - y
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
    // The sheets are wide content: the work width (LAYOUT §14).
    let ww = at.wide;
    cy += match tab_list.get(usize::from(on)).map(|t| t.1) {
        Some(Tab::Dice(s)) => dice_review(m, work, s, ui, x, cy, ww),
        Some(Tab::Checksum(s)) => sheet::review_sheet(m, work, s, ui, (x, cy, ww, at.cls, at.hide)),
        Some(Tab::Translation(n)) => translation(work, n, None, &[], ui, at.to(x, cy, ww)),
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
            cy += ui.wrap(x, cy, w, t::CAPTION, W::M, Role::OnSurface, &line) + t::GAP_SMALL;
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
        cy += ui.wrap(x, cy, w, t::CAPTION, W::M, Role::OnSurface, &line) + t::GAP_SMALL;
    }
    cy - y + t::GAP
}

// ---------------------------------------------------------------------
// The strings (§6.6) and Strings made (§6.7, §6.8)
// ---------------------------------------------------------------------

/// The field the string is typed back in, and the box that differs.
/// Returns its height.
fn typed_back(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let ink = Role::OnSurface;
    let hide = m.masked();
    let (typed, differs) = m.typed_back();
    let mut whole = String::from_utf8_lossy(PREFIX).into_owned();
    if hide {
        whole.push_str(&MASK.repeat(typed.len()));
    } else {
        whole.push_str(typed);
    }
    let shown = faraday_ui::grouped(&whole);
    ui.fill(x, y, w, t::FIELD, t::RADIUS_SMALL, Role::Surface);
    ui.stroke(
        x,
        y,
        w,
        t::FIELD,
        t::RADIUS_SMALL,
        if differs.is_some() {
            Role::Error
        } else {
            Role::OutlineVariant
        },
    );
    let fit = ui.fit(t::LABEL, W::M, &shown, w - 2.0 * t::GAP);
    ui.text_mid(x + t::GAP, y, t::FIELD, t::LABEL, W::M, ink, &fit);
    let mut cy = y + t::FIELD + t::GAP;
    if let Some(b) = differs {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            Role::Error,
            &fill(EN.codex32_box_differs, &[&b.to_string()]),
        ) + t::GAP;
    }
    cy - y
}

/// §6.6: the string in groups of four at the largest mono size that
/// fits, or its Codex32QR code; the typing back on the codex32 keyboard;
/// Continue and Skip.
fn strings(m: &Codex32Make, ui: &mut Ui, at: At) -> f32 {
    let ink = Role::OnSurface;
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
            cy += ui.wrap(x, cy, w, size, W::M, ink, &g) + t::GAP;
        }
    }
    cy += typed_back(m, ui, x, cy, w);
    let (checked, ask) = m.checked();
    if checked.get(m.at()).copied().unwrap_or(false) {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            Role::Ok,
            EN.codex32_typed_back_same,
        ) + t::GAP;
    } else {
        cy += keyboard(ui, x, cy, w, KeyboardKind::Codex32);
    }
    if ask {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            Role::Warning,
            EN.codex32_skip_caution,
        ) + t::GAP;
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
    // The computer's dice are Demo's alone, and Demo puts its sample in
    // place of what they made: no set made here is from them.
    let entropy = match (rolled, typed) {
        (true, true) => EN.codex32_your_dice_and_characters,
        (false, true) => EN.codex32_your_characters,
        _ => EN.codex32_your_dice,
    };
    let entropy = if m.sample() {
        fill(EN.c32r_entropy_sample, &[&m.identifier().to_uppercase()])
    } else if m.given_kind().is_some() {
        EN.c32r_your_shares.to_string()
    } else {
        entropy.to_string()
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
        (EN.codex32_entropy, entropy),
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
    ui.text(x, y, t::CAPTION, W::R, Role::OnSurfaceVariant, label);
    let ly = ui.line(t::CAPTION, W::R) + t::GAP_SMALL;
    let vh = ui.wrap(x, y + ly, w, t::LABEL, W::M, Role::OnSurface, value);
    ui.rule(x, y + ly + vh + t::GAP_SMALL, w, Role::OutlineVariant);
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

/// The open cell's working: a dialog (`faraday_ui::sheet::dialog_in`)
/// [`t::CHART_SHEET_WIDTH`] wide on a wide window, over Material's scrim
/// right of the sidebar, Previous, Next and Done its actions; a page of its own
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
    // Material's scrim over the page right of the sidebar; nothing under
    // the dialog is within reach, and a press beside it closes it.
    let scrim = Role::Scrim.with_alpha(t::SCRIM_ALPHA);
    ui.wash(x0, 0.0, cw, h, 0.0, scrim);
    ui.clear_hits();
    let outside = ui.outside.replace(a(C32::CellClose));
    let actions = [
        (EN.codex32_previous, Style::Secondary, a(C32::CellStep(-1))),
        (EN.codex32_next, Style::Secondary, a(C32::CellStep(1))),
        (EN.codex32_done_button, Style::Primary, a(C32::CellClose)),
    ];
    faraday_ui::sheet::dialog_in(
        ui,
        (x0, cw, h, t::CHART_SHEET_WIDTH),
        (Icon::Info.into(), Role::Primary.paint(), &head),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for l in &lines {
                cy += ui.wrap(x, cy, iw, t::LABEL, W::R, Role::OnSurface, l) + t::GAP_SMALL;
            }
            cy - y
        },
        &actions,
    );
    ui.outside = outside;
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
    ui.fill(x0, 0.0, cw, h, 0.0, Role::Surface);
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
        cy += ui.wrap(M, cy, iw, t::LABEL, W::R, Role::OnSurface, l) + t::GAP;
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
