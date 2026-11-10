//! The form that makes a Krux code, drawn (`docs/KEF.md` §15.3): the
//! first step and the Options sheet. What a press does is `mod.rs`.

use super::{Focus, IdKind, KefAct, KefForm, Stage};
use crate::Action;
use crate::dice_entry::Press;
use crate::dice_entry::screen::{Ask as DiceAsk, dice_panel};
use crate::drawing::View;
use crate::screens::pass_pair::pass_pair;
use crate::screens::{dice_pad, randomness};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::screen::{secret_box, text_box};
use crate::worker::kef_make::{self, IdChoice};
use faraday_codec::kef::{Mode, spec_of};
use faraday_draw::tokens::{BODY_S, BUTTON_H, FIELD_H, GAP, LABEL_L};
use faraday_draw::widgets::Icon;

/// What the owner gives the drawing: how a press is wrapped as an
/// `Action`, and the action that puts the keys on the dice rolls.
#[derive(Clone, Copy)]
pub(crate) struct Ask {
    pub wrap: fn(KefAct) -> Action,
    pub wrap_dice: fn(Press) -> Action,
    pub on_rolls: fn(usize) -> Action,
    /// The press that marks block (row) `n` of the grid on Paper.
    pub pin: fn(usize) -> Action,
    /// The press that marks the grid by rows (`true`) or blocks.
    pub rows: fn(bool) -> Action,
}

/// The first step, at (x, y), `w` wide. Returns its height. Drawn in every stage.
pub(crate) fn draw(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    f: &KefForm,
    ask: Ask,
) -> f32 {
    let mut cy = y;
    match f.stage {
        Stage::Made => return code_card(app, ui, (x, cy, w), f, ask),
        Stage::Check => return check_card(app, ui, (x, cy, w), f, ask),
        Stage::Making | Stage::Comparing => {
            let line = if f.stage == Stage::Making {
                EN.kef_making
            } else {
                EN.kef_comparing
            };
            ui.text_as(x, cy, LABEL_L, Role::OnSurface, line);
            cy += ui.advance_as(LABEL_L, faraday_draw::tokens::KEF_HEAD_ADVANCE);
            if let Some(b) = app.busy() {
                crate::screens::busy_bar(app, ui, (x, cy, w), b);
                cy += faraday_draw::tokens::PROGRESS_H + GAP;
            }
            ui.button(
                x,
                cy,
                None,
                BUTTON_H,
                EN.kef_cancel,
                Style::Secondary,
                Action::Cancel,
            );
            return cy + BUTTON_H - y;
        }
        Stage::Entry => {}
    }
    if f.dice.open().is_some() {
        return dice_view(app, ui, (x, cy, w), f, ask);
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.kef_form_dice,
        Style::Primary,
        (ask.wrap)(KefAct::Dice),
    );
    cy += BUTTON_H + GAP;
    cy += pass_pair(
        ui,
        (x, cy, w),
        (f.pass.text.as_str(), f.pass2.text.as_str()),
        |ui, n, fy| {
            let (label, text, focus, press) = if n == 0 {
                (EN.kef_passphrase, &f.pass, Focus::Pass, KefAct::Field)
            } else {
                (
                    EN.kef_passphrase_again,
                    &f.pass2,
                    Focus::PassAgain,
                    KefAct::FieldAgain,
                )
            };
            let show = app.pass_shows(f.shown, text.text.as_str());
            let mut cy = fy;
            cy += ui.field_label(x, cy, label);
            secret_box(
                ui,
                x,
                cy,
                w,
                text,
                show,
                f.focus == Some(focus),
                (ask.wrap)(press),
                (ask.wrap)(KefAct::Eye),
            );
            cy + FIELD_H + GAP - fy
        },
    );
    let options = (ask.wrap)(KefAct::Options);
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.kef_form_options,
        Style::Secondary,
        options,
    );
    cy += BUTTON_H + GAP;
    cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &f.summary()) + GAP;
    cy += randomness::row(app, ui, (x, cy, w));
    if let Some(e) = &f.error {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, e) + GAP;
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.kef_form_make,
        Style::Primary.when(f.passphrases_agree()),
        (ask.wrap)(KefAct::Make),
    );
    cy + BUTTON_H - y
}

/// The Code card: the code's picture, Checked once it is, **Check my
/// code** and **Make another**. On Paper, in place of the picture of a
/// code that fits a blank template, **Size**, **By hand** and the grid to
/// copy it from (`docs/KEF.md` §15.1). In the tool, **Size**, **By hand**
/// and **Copy by hand**, which swaps the picture for that grid (§15.2).
/// Returns its height.
fn code_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), f: &KefForm, ask: Ask) -> f32 {
    use faraday_draw::tokens::KEF_BY_HAND_MAX_SIDE as MAX;
    let mut cy = y;
    if let Some(m) = &f.made {
        match &m.view {
            Ok(v) => {
                let code = v.frames.get(v.frame);
                let side = code.map(|q| q.size());
                let offer = f.by_hand_offer && !f.paper;
                let grid = code
                    .filter(|q| q.size() <= MAX)
                    .filter(|_| f.paper || (offer && f.by_hand));
                if let Some(s) = side.filter(|_| f.paper || offer) {
                    cy += by_hand_part(ui, (x, cy, w), s, f, ask);
                }
                if let Some(q) = grid {
                    let g = crate::screens::CopyGrid {
                        m: q,
                        printed: crate::wallet_backup::structural_fixed,
                        lettered: true,
                        rows: f.rows,
                        pin: f.pin,
                        pick: ask.pin,
                        mode: ask.rows,
                    };
                    cy += crate::screens::copy_grid(ui, x, cy, w, &g) + GAP;
                } else if let Some(q) = code {
                    let size = if ui.compact {
                        faraday_draw::geom::SizeClass::Small
                    } else {
                        faraday_draw::geom::SizeClass::Wide
                    };
                    let side = faraday_draw::tokens::qr_side(size).min(w);
                    faraday_ui::parts::qr_code(ui, q, x + (w - side) / 2.0, cy, side);
                    cy += side + GAP;
                }
                for line in &v.label {
                    cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, line);
                }
                cy += GAP;
            }
            Err(e) => cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, e) + GAP,
        }
    }
    // On Paper the code counts once its drawn copy was scanned back, or
    // with no camera to scan it with, once it opened (§15.1).
    let no_camera = app.cameras().is_empty();
    if f.paper && !f.scanned {
        let line = format!("{}: {}", EN.kef_drawn_copy, EN.kef_not_scanned);
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &line) + GAP;
    }
    if f.checked {
        ui.text_as(x, cy, LABEL_L, Role::Ok, EN.kef_checked);
        cy += ui.advance_as(LABEL_L, faraday_draw::tokens::KEF_HEAD_ADVANCE);
    } else if !(f.paper && !no_camera) {
        ui.button(
            x,
            cy,
            None,
            BUTTON_H,
            EN.kef_check_my_code,
            Style::Primary,
            (ask.wrap)(KefAct::Check),
        );
        cy += BUTTON_H + GAP;
    }
    if f.scan_offered() && !no_camera {
        ui.button(
            x,
            cy,
            None,
            BUTTON_H,
            EN.kef_scan_my_copy,
            Style::Primary,
            (ask.wrap)(KefAct::ScanCopy),
        );
        cy += BUTTON_H + GAP;
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.kef_make_another,
        Style::Secondary,
        (ask.wrap)(KefAct::Another),
    );
    cy + BUTTON_H - y
}

/// The Code card's **Size** and **By hand** lines for a code of side `s`,
/// and in the tool **Copy by hand** or **Show the code**. Returns its height.
fn by_hand_part(ui: &mut Ui, (x, y, w): (f32, f32, f32), s: usize, f: &KefForm, ask: Ask) -> f32 {
    use faraday_draw::tokens::KEF_BY_HAND_MAX_SIDE as MAX;
    let mut cy = y;
    let s2 = [s.to_string(), s.to_string()];
    let by_hand = if s <= MAX {
        fill(EN.kef_by_hand_sheet, &[&s2[0], &s2[1]])
    } else {
        EN.kef_opt_by_hand_no.to_string()
    };
    let size = fill(EN.kef_opt_side, &[&s2[0], &s2[1]]);
    for line in [
        format!("{}: {size}", EN.kef_opt_size),
        format!("{}: {by_hand}", EN.kef_opt_by_hand),
    ] {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &line);
    }
    if s > MAX {
        let big = fill(EN.kef_opt_side, &[&MAX.to_string(), &MAX.to_string()]);
        let line = format!("{}: {big}", EN.kef_largest);
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &line);
    }
    cy += GAP;
    if f.by_hand_offer && !f.paper && s <= MAX {
        let label = if f.by_hand {
            EN.kef_show_code
        } else {
            EN.kef_copy_by_hand
        };
        let act = (ask.wrap)(KefAct::ByHand);
        ui.button(x, cy, None, BUTTON_H, label, Style::Secondary, act);
        cy += BUTTON_H + GAP;
    }
    cy - y
}

/// Check my code: one masked field and **Compare**. Returns its height.
fn check_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), f: &KefForm, ask: Ask) -> f32 {
    let mut cy = y;
    cy += ui.field_label(x, cy, EN.kef_check_pass);
    let show = app.pass_shows(f.check_shown, f.check.text.as_str());
    secret_box(
        ui,
        x,
        cy,
        w,
        &f.check,
        show,
        f.focus == Some(Focus::Check),
        (ask.wrap)(KefAct::CheckField),
        (ask.wrap)(KefAct::CheckEye),
    );
    cy += FIELD_H + GAP;
    if let Some(e) = &f.error {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, e) + GAP;
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.kef_compare,
        Style::Primary,
        (ask.wrap)(KefAct::Compare),
    );
    cy += BUTTON_H + GAP;
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.kef_done,
        Style::Secondary,
        (ask.wrap)(KefAct::CheckBack),
    );
    cy + BUTTON_H - y
}

/// The dice entry, open, with its pad. Returns its height.
fn dice_view(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), f: &KefForm, ask: Ask) -> f32 {
    let mut cy = y;
    let focus = match f.focus {
        Some(Focus::Dice(p)) => Some(p),
        _ => None,
    };
    let dask = DiceAsk {
        entry: &f.dice,
        focus,
        wrap: ask.wrap_dice,
        on_rolls: ask.on_rolls,
        cost: (64, 3),
    };
    cy += dice_panel(app, ui, (x, cy, w), (super::DICE_TAG, dask), false) + GAP;
    let key = faraday_draw::tokens::DICE_PAD_KEY_H;
    let pad_w = w - key - GAP;
    dice_pad(ui, (x, cy, pad_w), &|_| Style::Secondary, |face| {
        (ask.wrap_dice)(Press::Face(face))
    });
    ui.button(
        x + w - key,
        cy,
        Some(key),
        key,
        "",
        Style::Secondary,
        (ask.wrap_dice)(Press::Face(0)),
    );
    ui.icon(
        x + w - key + (key - faraday_draw::tokens::ICON_SMALL) / 2.0,
        cy + (key - faraday_draw::tokens::ICON_SMALL) / 2.0,
        faraday_draw::tokens::ICON_SMALL,
        Icon::Delete,
        faraday_draw::tokens::CODEX32_TAB_GLYPH,
        Role::OnSurface,
    );
    cy + key + GAP - y
}

/// Options, as a dialog over a page `w` by `h`.
pub(crate) fn sheet(app: &View, ui: &mut Ui, (w, h): (f32, f32), f: &KefForm, ask: Ask) {
    let items = [(EN.kef_done, Style::Primary, (ask.wrap)(KefAct::Back))];
    let head = (
        Icon::Keys.into(),
        Role::Primary.paint(),
        EN.kef_form_options,
    );
    let body =
        |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 { options_body(app, ui, (x, y, iw), f, ask) };
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| body(ui, x, y, iw),
        &items,
    );
}

fn choice(on: bool) -> Style {
    if on {
        Style::Selected
    } else {
        Style::Secondary
    }
}

fn heading(ui: &mut Ui, (x, y): (f32, f32), text: &str) -> f32 {
    ui.text_as(x, y, LABEL_L, Role::OnSurface, text);
    ui.advance_as(LABEL_L, faraday_draw::tokens::KEF_HEAD_ADVANCE)
}

fn options_body(
    _app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    f: &KefForm,
    ask: Ask,
) -> f32 {
    let mut cy = y;
    let o = &f.options;
    cy += size_part(ui, (x, cy, w), f, ask);
    cy += heading(ui, (x, cy), EN.kef_opt_version);
    for mode in kef_make::MODES {
        let name = match mode {
            Mode::Gcm => EN.kef_mode_gcm,
            Mode::Ctr => EN.kef_mode_ctr,
            Mode::Cbc => EN.kef_mode_cbc,
            Mode::Ecb => EN.kef_mode_ecb,
        };
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, name);
        cy += ui.advance_as(BODY_S, faraday_draw::tokens::KEF_CAPTION_ADVANCE);
        cy += version_buttons(ui, (x, cy, w), &kef_make::versions_in(mode), f, ask);
        if mode == Mode::Ecb {
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                kef_make::ecb_line(),
            ) + GAP;
        }
    }
    let legacy = kef_make::legacy_versions();
    let open = f.legacy || legacy.contains(&o.version);
    let items = [(
        EN.kef_opt_legacy.to_string(),
        choice(open),
        (ask.wrap)(KefAct::Legacy),
    )];
    cy += faraday_ui::parts::button_rows(ui, x, cy, w, &items);
    if open {
        cy += version_buttons(ui, (x, cy, w), &legacy, f, ask);
    }
    if o.low_offered() {
        cy += ui.switch_row((x, cy, w), EN.kef_opt_low, o.low, (ask.wrap)(KefAct::Low)) + GAP;
    }
    cy += heading(ui, (x, cy), EN.kef_opt_rounds);
    text_box(
        ui,
        x,
        cy,
        w,
        &f.rounds,
        false,
        f.focus == Some(Focus::Rounds),
        (ask.wrap)(KefAct::Rounds),
    );
    cy += FIELD_H + GAP;
    if o.problem() == Some(EN.kef_refuse_count) {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, EN.kef_refuse_count) + GAP;
    }
    cy += id_part(ui, (x, cy, w), f, ask);
    cy += paper_part(ui, (x, cy), f);
    cy - y
}

fn version_buttons(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    versions: &[u8],
    f: &KefForm,
    ask: Ask,
) -> f32 {
    let items: Vec<_> = versions
        .iter()
        .map(|&v| {
            let label = if spec_of(v).is_some_and(|s| s.compressed) {
                fill(EN.kef_opt_compressed, &[&v.to_string()])
            } else {
                v.to_string()
            };
            (
                label,
                choice(f.options.version == v),
                (ask.wrap)(KefAct::Version(v)),
            )
        })
        .collect();
    faraday_ui::parts::button_rows(ui, x, y, w, &items)
}

/// Options: the id and its field.
fn id_part(ui: &mut Ui, (x, y, w): (f32, f32, f32), f: &KefForm, ask: Ask) -> f32 {
    let mut cy = y;
    let o = &f.options;
    cy += heading(ui, (x, cy), EN.kef_opt_id);
    let mut ids = vec![
        (
            EN.kef_opt_id_random8,
            IdKind::Random8,
            matches!(o.id, IdChoice::Random(n) if n != faraday_keys::kef::LONG_ID_LEN),
        ),
        (
            EN.kef_opt_id_random16,
            IdKind::Random16,
            matches!(o.id, IdChoice::Random(n) if n == faraday_keys::kef::LONG_ID_LEN),
        ),
        (
            EN.kef_opt_id_label,
            IdKind::Label,
            matches!(o.id, IdChoice::Label(_)),
        ),
    ];
    if f.fingerprint().is_some() {
        ids.push((
            EN.kef_opt_id_fingerprint,
            IdKind::Fingerprint,
            matches!(o.id, IdChoice::Fingerprint(_)),
        ));
    }
    let items: Vec<_> = ids
        .into_iter()
        .map(|(l, k, on)| (l.to_string(), choice(on), (ask.wrap)(KefAct::Id(k))))
        .collect();
    cy += faraday_ui::parts::button_rows(ui, x, cy, w, &items);
    if matches!(o.id, IdChoice::Label(_)) {
        cy += ui.field_label(x, cy, EN.kef_opt_label);
        text_box(
            ui,
            x,
            cy,
            w,
            &f.label,
            false,
            f.focus == Some(Focus::Label),
            (ask.wrap)(KefAct::Label),
        );
        cy += FIELD_H + GAP;
    }
    if matches!(o.id, IdChoice::Label(_) | IdChoice::Fingerprint(_)) {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            kef_make::public_id_line(),
        ) + GAP;
    }
    cy - y
}

/// Options: the Size group, one item for each sheet size the contents
/// can take, the one the options give selected; where the contents take
/// none, the one line that says so (`docs/KEF.md` §15.3).
fn size_part(ui: &mut Ui, (x, y, w): (f32, f32, f32), f: &KefForm, ask: Ask) -> f32 {
    use faraday_draw::tokens::KEF_BY_HAND_MAX_SIDE as MAX;
    let mut cy = y;
    cy += heading(ui, (x, cy), EN.kef_opt_size);
    let sizes = f.sizes();
    if sizes.is_empty() {
        let big = fill(EN.kef_opt_side, &[&MAX.to_string(), &MAX.to_string()]);
        let line = format!(
            "{}: {}, {}: {big}",
            EN.kef_opt_by_hand, EN.kef_opt_by_hand_no, EN.kef_largest
        );
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &line) + GAP;
        return cy - y;
    }
    let now = f.side();
    for &(side, _) in sizes {
        let n = [side.to_string(), side.to_string()];
        let head = fill(EN.kef_opt_side, &[&n[0], &n[1]]);
        let sheet = fill(EN.kef_by_hand_sheet, &[&n[0], &n[1]]);
        let it = faraday_ui::roles::ListItem {
            headline: &head,
            supporting: &sheet,
            trailing: None,
            action: (ask.wrap)(KefAct::Size(side)),
        };
        cy += ui.list_item((x, cy, w), &it, now == Some(side));
    }
    cy + GAP - y
}

/// Options: the size and whether it is copied by hand.
fn paper_part(ui: &mut Ui, (x, y): (f32, f32), f: &KefForm) -> f32 {
    let mut cy = y;
    let (size, by_hand) = match f.side() {
        Some(s) => (
            fill(EN.kef_opt_side, &[&s.to_string(), &s.to_string()]),
            if s <= faraday_draw::tokens::KEF_BY_HAND_MAX_SIDE {
                EN.kef_opt_by_hand_yes
            } else {
                EN.kef_opt_by_hand_no
            },
        ),
        None if spec_of(f.options.version).is_some_and(|s| s.compressed) => {
            (EN.kef_opt_size_after.to_string(), EN.kef_opt_size_after)
        }
        None => (EN.kef_opt_size_after.to_string(), EN.kef_opt_by_hand_no),
    };
    for line in [
        format!("{}: {size}", EN.kef_opt_size),
        format!("{}: {by_hand}", EN.kef_opt_by_hand),
    ] {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, &line);
        cy += ui.advance_as(BODY_S, faraday_draw::tokens::KEF_CAPTION_ADVANCE);
    }
    cy - y
}
