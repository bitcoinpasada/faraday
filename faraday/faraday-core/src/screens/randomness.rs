//! `Screen::Randomness`: Add randomness (`docs/OWN-RANDOMNESS.md` §2.2).
//! The status line, the source chosen (Camera, Dice, Coins, Cards and,
//! where the shell has a pointer and keys, Pointer and keys) and its pad,
//! New key's own, with the batch not yet added and its Add. New
//! key's Generator card draws the same sources on itself
//! ([`own_sources`]). The Randomness row ([`row`]) stands above every
//! press that draws the system's randomness and opens the page from there
//! (`docs/OWN-RANDOMNESS.md` §8, F157).

use super::parts::{buttons_and_next, camera_row, card_pad, coin_pad, dice_pad};
use crate::Action;
use crate::drawing::View;
use crate::fresh::OwnSource;
use crate::fresh::page::{Act, OwnPage};
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, LABEL_M, LIST_2, LIST_PAD_V, TITLE_S,
};
use faraday_draw::tokens::{
    COLUMN_MAX, GAP, GAP_SMALL, ICON_GAP, ICON_GLYPH_BUTTON, LABEL, MARGIN_COMPACT, MARGIN_WIDE,
    OWN_AREA, OWN_BUTTON, PAD, PRESS_EMPHASIS, SHAPE_M, SHAPE_S, STATE_PRESS,
};
use faraday_draw::widgets::Icon;

/// The sources a person picks from, in order: Camera only where the
/// shell has a camera, Pointer and keys only where it has a pointer and
/// keys.
const SOURCES: [(OwnSource, &str); 5] = [
    (OwnSource::Camera, EN.own_source_camera),
    (OwnSource::Dice, EN.own_source_dice),
    (OwnSource::Coins, EN.own_source_coins),
    (OwnSource::Cards, EN.own_source_cards),
    (OwnSource::PointerKeys, EN.own_source_pointer_keys),
];

fn own(a: Act) -> Action {
    Action::Own(a)
}

/// The Randomness row (`docs/OWN-RANDOMNESS.md` §8.2, §8.3) from (x, y),
/// `w` wide, directly above a press that draws the system's randomness:
/// Material 3's two-line list item (m3.material.io, Lists), the headline
/// Randomness, the status line as its supporting text, wrapping under the
/// headline where the column is narrow, and Material's text button **Add
/// randomness** as its trailing element. On the Pi's panel the status
/// line alone, one line in Label M; Add randomness is then the first row
/// of the bar's ⋮. A press opens the page and Back comes back to the
/// press as it was left. Nothing waits on it. Returns its height with the
/// gap under it.
pub(crate) fn row(app: &View, ui: &mut Ui, at: (f32, f32, f32)) -> f32 {
    row_in(app, ui, at, app.nav() == crate::nav::Nav::Menu)
}

/// [`row`], the status line alone where `short`; a sheet, which has no
/// bar and so no ⋮, keeps the whole row on every panel.
pub(crate) fn row_in(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), short: bool) -> f32 {
    let status = app.own_status();
    if short {
        let line = ui.line_as(LABEL_M);
        let shown = short_status(ui, &status, w);
        ui.text_mid_as((x, y, line), LABEL_M, Role::OnSurfaceVariant, &shown);
        return line + GAP;
    }
    let (label, open) = (EN.own_add_randomness, own(Act::Open));
    let bw = crate::compact::screen::measured(ui, |ui| {
        ui.button(0.0, 0.0, None, BUTTON_H, label, Style::Ghost, open)
    });
    let tw = (w - bw - GAP).max(0.0);
    let head = ui.line_as(BODY_L);
    let sup = ui.wrap_height_as(BODY_M, &status, tw);
    let h = (head + sup + 2.0 * LIST_PAD_V).max(LIST_2);
    let top = y + (h - head - sup) / 2.0;
    let name = ui.fit_as(BODY_L, EN.own_randomness, tw);
    ui.text_mid_as((x, top, head), BODY_L, Role::OnSurface, &name);
    ui.wrap_as((x, top + head, tw), BODY_M, Role::OnSurfaceVariant, &status);
    let by = y + (h - BUTTON_H) / 2.0;
    ui.button(
        x + w - bw,
        by,
        Some(bw),
        BUTTON_H,
        label,
        Style::Ghost,
        open,
    );
    h + GAP
}

/// Whether the screen on show holds a press that draws the system's
/// randomness (`docs/OWN-RANDOMNESS.md` §8.1), and so the Randomness row
/// above it: what puts Add randomness first in the Pi's ⋮. Each screen
/// draws the row where this holds. New key's Generator card offers the
/// sources on itself and the demo's dice roller none; the Lock sheet
/// draws its own ([`lock_seals_changes`]). Only lookups.
pub(crate) fn own_at_press(app: &crate::Faraday) -> bool {
    use crate::Screen as S;
    match app.screen {
        S::KeyGen => app.keygen.state().is_some_and(|k| {
            use crate::codex32::Card;
            let id = |m: &crate::codex32::Codex32Make| {
                m.open() == Some(Card::Identifier) && m.work().is_none()
            };
            !k.locked
                && ((k.slip39 && k.open == Some(crate::keygen::kstep::ENTER))
                    || k.codex32.as_ref().is_some_and(id))
        }),
        S::CreateVault => app.vaults.create.as_ref().is_some_and(|c| {
            app.vaults.working != Some(crate::vault::Work::Create)
                && (!app.compact || c.open == Some(crate::vault::vstep::PHRASES))
        }),
        S::VaultContents => at_vault_press(app),
        S::BackupFlow => app.flow().is_some_and(|f| {
            f.step == crate::backup_flow::Step::Make
                && !f.kind.is_wallet()
                && !f.made
                && !f.adding
                && app.vaults.working != Some(crate::vault::Work::FlowMake)
        }),
        S::Backup => app.backup().is_some_and(|b| {
            let rotating = b
                .c32
                .as_ref()
                .is_some_and(|st| matches!(st.set, crate::codex32_backup::SetStep::Scheme(..)));
            rotating || (b.reveal && b.check.is_none() && b.paper.is_none())
        }),
        S::Spend => app.wallets.spend().is_some_and(|s| {
            s.spend.finished.is_none()
                && matches!(s.open, Some(5 | 6 | 9))
                && crate::spend_opens_round(&s.inspection)
        }),
        _ => false,
    }
}

/// Whether a lock now seals a changed vault, drawing the seal's nonce:
/// the Lock sheet's Randomness row (`docs/OWN-RANDOMNESS.md` §8.1 row
/// 15). Only lookups.
pub(crate) fn lock_seals_changes(app: &crate::Faraday) -> bool {
    !app.lock_would_lose() && app.vaults.open.iter().any(|v| v.changes > 0)
}

/// [`own_at_press`] on an open vault's contents: Export for KeePass's
/// Seal and export, a Secure Boot record's Enrolment files, an image's
/// Hold to sign, and a vault holding changes not yet sealed, which the
/// idle lock seals with no sheet to show (§8.1 rows 6, 9, 10 and 15).
/// Only lookups.
fn at_vault_press(app: &crate::Faraday) -> bool {
    use crate::vault::{Purpose, Work};
    let v = &app.vaults;
    let Some(open) = v.open.get(v.current) else {
        return false;
    };
    if let Some(p) = &v.prompt {
        return p.purpose == Purpose::Kdbx && v.working != Some(Work::Kdbx);
    }
    if v.sb_images {
        return v.sb_sign.as_ref().is_some_and(|(_, c)| c.is_ok());
    }
    if v.form.is_some() || v.saving || v.signing {
        return false;
    }
    let secure_boot = (!app.compact || v.item_open)
        && app
            .vault_selected_index()
            .and_then(|at| v.records(open.id).get(at))
            .is_some_and(|r| r.kind() == faraday_vault::records::kind::SECURE_BOOT);
    open.changes > 0 || (secure_boot && app.selected_waits().is_none())
}

/// The status line in Label M, `w` wide, on one line: whole where it
/// fits, else cut at a count's comma with an ellipsis, never inside a
/// number (`docs/OWN-RANDOMNESS.md` §8.3); cut to fit only where even
/// its first count does not.
fn short_status(ui: &Ui, status: &str, w: f32) -> String {
    if ui.measure_as(LABEL_M, status) <= w {
        return status.to_string();
    }
    let mut end = status.len();
    while let Some(at) = status[..end].rfind(", ") {
        let cut = format!("{}…", &status[..at]);
        if ui.measure_as(LABEL_M, &cut) <= w {
            return cut;
        }
        end = at;
    }
    ui.fit_as(LABEL_M, status, w)
}

/// Add randomness, in the column at `x0`, `cw` wide.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(p) = app.own_page() else {
        return;
    };
    let (x, w) = if ui.compact {
        (x0 + MARGIN_COMPACT, cw - 2.0 * MARGIN_COMPACT)
    } else {
        let w = (cw - 4.0 * MARGIN_WIDE).min(COLUMN_MAX);
        (x0 + 2.0 * MARGIN_WIDE, w)
    };
    // The top app bar has the way back and the name.
    let top = if ui.compact {
        PAD
    } else {
        let name = app.trail_name().unwrap_or(EN.compact_settings);
        let back = app.named_back((name, Action::Back));
        super::bar_head(ui, Some(back), EN.own_add_randomness)
    } - app.list_offset;
    let mut y = top;
    y += ui.wrap_as((x, y, w), BODY_M, Role::OnSurfaceVariant, &app.own_status()) + PAD;
    y += ui.wrap_as((x, y, w), TITLE_S, Role::OnSurface, EN.own_add_to_pool) + PAD;
    y += own_sources(app, p, ui, (x, y, w), false);
    if let Some((line, shown)) = app.own_added() {
        y += PAD + added(ui, (x, y + PAD, w), line, shown);
    }
    let content = y + PAD - top;
    crate::compact::screen::finish(app, ui, x0, cw, h, content);
}

/// What was added, once anything was: a check mark and the line
/// ("Added to the pool 3 times · last: 6 dice rolls"). As a batch is
/// taken a pressed state layer in the accent, as strong as the codex32
/// pad's emphasised key at `shown` 1, eases away behind it to nothing;
/// none under reduce motion, where the line's own count is what changes.
/// Returns its height.
fn added(ui: &mut Ui, (x, y, w): (f32, f32, f32), line: &str, shown: f32) -> f32 {
    let icon = ui.line_as(BODY_M);
    let (tx, tw) = (x + icon + ICON_GAP, w - icon - ICON_GAP);
    let h = ui.wrap_height_as(BODY_M, line, tw).max(icon) + 2.0 * GAP_SMALL;
    if shown > 0.0 {
        let alpha = (STATE_PRESS + PRESS_EMPHASIS) * shown.clamp(0.0, 1.0);
        ui.wash(x, y, w, h, SHAPE_S, Role::Primary.at(alpha));
    }
    let cy = y + GAP_SMALL;
    ui.icon(x, cy, icon, Icon::Success, ICON_GLYPH_BUTTON, Role::Ok);
    ui.wrap_as((tx, cy, tw), BODY_M, Role::OnSurface, line);
    h
}

/// The sources, the chosen one's pad and its batch from (x, y), `w`
/// wide: on Add randomness, and on New key's Generator card (`card`),
/// where Add stands beside Undo and Clear, the card's own step being
/// Make the words. Returns its height.
pub(crate) fn own_sources(
    app: &View,
    p: &OwnPage,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    card: bool,
) -> f32 {
    let mut cy = y;
    let sources: Vec<(&str, Action)> = SOURCES
        .iter()
        .filter(|(s, _)| match s {
            OwnSource::Camera => !app.cameras().is_empty() || p.wants_camera(),
            OwnSource::PointerKeys => app.pointer_keys_offered(),
            _ => true,
        })
        .map(|&(s, label)| (label, own(Act::Source(s))))
        .collect();
    // On the Generator card nothing is chosen until a source is pressed.
    let on = sources
        .iter()
        .position(|&(_, a)| a == own(Act::Source(p.source())))
        .filter(|_| p.chosen());
    cy += if ui.compact {
        source_grid(ui, (x, cy, w), &sources, on)
    } else {
        ui.choices((x, cy, w, OWN_BUTTON), LABEL, &sources, on)
    } + PAD;
    if p.chosen() {
        cy += pad(p, ui, x, cy, w);
        cy += batch(p, ui, (x, cy, w), card);
    }
    cy - y
}

/// The sources on a small panel: two to a row, each half the panel, so
/// every name is whole. Returns its height.
fn source_grid(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    sources: &[(&str, Action)],
    on: Option<usize>,
) -> f32 {
    let bw = (w - GAP) / 2.0;
    let mut rows = 0.0;
    for (i, &(label, action)) in sources.iter().enumerate() {
        let (col, row) = ((i % 2) as f32, (i / 2) as f32);
        let style = Style::chosen(on == Some(i));
        let at = y + row * (OWN_BUTTON + GAP);
        ui.button(
            x + col * (bw + GAP),
            at,
            Some(bw),
            OWN_BUTTON,
            label,
            style,
            action,
        );
        rows = row + 1.0;
    }
    rows * (OWN_BUTTON + GAP) - GAP
}

/// The chosen source's pad, New key's own. Returns its height.
fn pad(p: &OwnPage, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match p.source() {
        OwnSource::Camera => camera_row(
            ui,
            (x, y, w),
            p.frame_live(),
            p.last_frame(),
            own(Act::Frame),
        ),
        OwnSource::Dice => dice_pad(ui, (x, y, w), &|_| Style::Secondary, |f| own(Act::Roll(f))),
        OwnSource::Coins => coin_pad(ui, (x, y, w), own(Act::Flip(true)), own(Act::Flip(false))),
        OwnSource::Cards => card_pad(
            ui,
            (x, y, w),
            p.rank(),
            p.cards(),
            |r| own(Act::Rank(r)),
            |s| own(Act::Suit(s)),
        ),
        OwnSource::PointerKeys => pointer_keys_area(p, ui, (x, y, w)),
    }
}

/// Pointer and keys' area, which the pointer's moves are taken over, and
/// what it has taken. Returns its height.
fn pointer_keys_area(p: &OwnPage, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    ui.fill(x, y, w, OWN_AREA, SHAPE_M, Role::SurfaceContainerHighest);
    ui.press(
        Shape::new(x, y, w, OWN_AREA, SHAPE_M),
        Role::OnSurfaceVariant.paint(),
        own(Act::Area),
    );
    let label = EN.own_pointer_keys_area;
    let lw = ui.measure_as(BODY_M, label);
    let lx = x + ((w - lw) / 2.0).max(PAD);
    ui.text_mid_as((lx, y, OWN_AREA), BODY_M, Role::OnSurfaceVariant, label);
    let mut cy = y + OWN_AREA + GAP;
    let (moves, keys) = p.pointer_keys();
    let count = |n: u32, one: &str, many: &str| {
        fill(
            EN.own_count,
            &[&n.to_string(), if n == 1 { one } else { many }],
        )
    };
    let moves = count(moves, EN.own_pointer_move, EN.own_pointer_moves);
    let line = fill(
        EN.own_pointer_keys_taken,
        &[&moves, &count(keys, EN.own_key, EN.own_keys)],
    );
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &line) + GAP;
    cy - y
}

/// What a batch holds before it is added, what the last press could not
/// do, Undo, Clear and Add (on a `card`, a button beside them, taking no
/// press while the batch is empty), and the keys on a wide layout. A
/// picture is added as it is taken, so the camera has none of these but
/// the keys. Returns its height.
fn batch(p: &OwnPage, ui: &mut Ui, (x, y, w): (f32, f32, f32), card: bool) -> f32 {
    let mut cy = y;
    let unit = match p.source() {
        OwnSource::Dice => Some((EN.own_dice_roll, EN.own_dice_rolls)),
        OwnSource::Coins => Some((EN.own_coin_flip, EN.own_coin_flips)),
        OwnSource::Cards => Some((EN.own_card, EN.own_cards)),
        OwnSource::Camera | OwnSource::PointerKeys => None,
    };
    if let Some((one, many)) = unit {
        let n = p.batch_len();
        let noun = if n == 1 { one } else { many };
        let line = fill(EN.own_batch, &[&n.to_string(), noun]);
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &line) + GAP;
        // As New key names it: 99 rolls are enough for 256 bits alone.
        if p.source() == OwnSource::Dice
            && n >= faraday_entropy::DiceRolls::needed(faraday_entropy::Strength::Bits256)
        {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::Ok, EN.own_enough_256) + GAP;
        }
    }
    if let Some(n) = p.note() {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, n) + GAP;
    }
    if unit.is_some() {
        cy += GAP_SMALL;
        let mut edits = vec![
            (EN.keygen_undo, Style::Ghost, own(Act::Undo)),
            (EN.keygen_clear, Style::Ghost, own(Act::Clear)),
        ];
        let any = p.batch_len() > 0;
        let next = (any && !card).then_some((EN.own_add, own(Act::Add)));
        if card {
            edits.push((EN.own_add, Style::Tonal.when(any), own(Act::Add)));
        }
        cy += buttons_and_next(ui, x, cy, w, &edits, next) + GAP_SMALL;
    }
    if !ui.compact {
        let hint = match p.source() {
            OwnSource::Dice => EN.own_dice_keys,
            OwnSource::Coins => EN.own_coin_keys,
            OwnSource::Cards => EN.keygen_type_the_rank,
            OwnSource::Camera => EN.own_camera_keys,
            OwnSource::PointerKeys => return cy - y,
        };
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, hint);
    }
    cy - y
}
