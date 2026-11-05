//! The Sticks and vaults card (`docs/STICK-SETUP.md` §2), drawn: the two
//! steppers, the vaults each with New or Already on a stick, the sticks
//! with the vaults on them, the checks, and Continue. The chips and
//! stick boxes are the backup board's (`board/screen.rs`). What a press
//! does is `layout.rs`.

use crate::Action;
use crate::backup_flow::layout::SetupLayout;
use crate::board::screen as board_screen;
use crate::board::{BoardPress, Chip, ChipView, Thing, Zone};
use crate::drawing::View;
use crate::screens::stepper;
use crate::strings::{EN as FEN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{self, BODY_M, BUTTON_H, LABEL_L, SEGMENTED_H};
use faraday_draw::widgets::Icon;

/// A vault's chip, in the list or on a stick.
fn chip_of(l: &SetupLayout, v: usize, on: Option<usize>) -> ChipView {
    ChipView {
        chip: match on {
            Some(s) => Chip::Placed(Zone::Card(s), Thing::Vault(v)),
            None => Chip::List(Thing::Vault(v)),
        },
        name: SetupLayout::vault_name(v),
        marks: String::new(),
        forms: Vec::new(),
        warn: false,
        link: on.is_some() && l.sticks_of(v).len() > 1,
        fixed: false,
        faint: false,
        off: on.is_some(),
        menu: true,
    }
}

/// The two counts' action makers: Vaults, then Sticks.
type Counts = (fn(u16) -> Action, fn(u16) -> Action);

/// The card for the layout of `owner`, drawn from (x, y) in `w`.
/// `locked`: nothing changes any more. Returns its height.
pub(crate) fn card(
    app: &View,
    ui: &mut Ui,
    owner: u64,
    (x, y, w): (f32, f32, f32),
    locked: bool,
    (counts, next): (Counts, Action),
) -> f32 {
    let Some(l) = app.flows.layout_of(owner) else {
        return 0.0;
    };
    let mut cy = y + steppers(ui, l, (x, y, w), locked, counts);
    let b = l.board();
    let mut fr = board_screen::card_frame(app, ui, b.held, b.drag, b.focus);
    cy += vault_list(ui, &mut fr, l, (x, cy, w), locked);
    cy += stick_boxes(ui, &mut fr, l, (x, cy, w), locked);
    if let (Some(held), Some(_)) = (b.held, b.drag)
        && let Thing::Vault(v) = held.thing()
    {
        let on = match held {
            Chip::Placed(Zone::Card(s), _) => Some(s),
            _ => None,
        };
        board_screen::card_lifted(ui, &mut fr, &chip_of(l, v, on));
    }
    cy += check_lines(ui, l, (x, cy + tokens::GAP, w)) + tokens::GAP;
    if !locked {
        let label = FEN.create_continue;
        let lw = ui.measure_as(LABEL_L, label) + tokens::SETUP_CONTINUE_PAD;
        let style = if l.blocked() {
            Style::Disabled
        } else {
            Style::Primary
        };
        ui.button(
            x + w - lw,
            cy + tokens::GAP,
            Some(lw),
            BUTTON_H,
            label,
            style,
            next,
        );
        cy += tokens::TOUCH + tokens::GAP;
    }
    cy - y
}

/// The Vaults and Sticks steppers, side by side where the width allows.
fn steppers(
    ui: &mut Ui,
    l: &SetupLayout,
    (x, y, w): (f32, f32, f32),
    locked: bool,
    (set_vaults, set_sticks): Counts,
) -> f32 {
    let step = |n: usize, set: fn(u16) -> Action| {
        let n = n.min(usize::from(u16::MAX)) as u16;
        (set(n.saturating_sub(1).max(1)), set(n.saturating_add(1)))
    };
    let side = w >= tokens::SETUP_STEPPER_PAIR_W;
    let sw = if side { (w - tokens::GAP) / 2.0 } else { w };
    stepper(
        ui,
        x,
        y,
        sw,
        FEN.backup_flow_vault_count_label,
        l.vaults(),
        step(l.vaults(), set_vaults).0,
        step(l.vaults(), set_vaults).1,
        !locked,
    );
    let (sx, sy) = if side {
        (x + sw + tokens::GAP, y)
    } else {
        (x, y + tokens::SETUP_STEPPER_H)
    };
    stepper(
        ui,
        sx,
        sy,
        sw,
        &fill(FEN.backup_flow_stick_count_label, &[l.medium().caps()]),
        l.sticks(),
        step(l.sticks(), set_sticks).0,
        step(l.sticks(), set_sticks).1,
        !locked,
    );
    (if side {
        tokens::SETUP_STEPPER_H
    } else {
        tokens::SETUP_STEPPER_H + tokens::SETUP_STEPPER_H
    }) + tokens::GAP
}

/// The vaults, each a chip with New or Already on a stick; the list is
/// where a vault dropped is taken off its stick. Returns its height.
fn vault_list(
    ui: &mut Ui,
    fr: &mut board_screen::CardFrame,
    l: &SetupLayout,
    (x, y, w): (f32, f32, f32),
    locked: bool,
) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as(
        (x, cy, w),
        LABEL_L,
        Role::OnSurfaceVariant,
        FEN.backup_flow_vault_count_label,
    ) + tokens::GAP_SMALL;
    let existing = fill(FEN.backup_flow_layout_existing, &[l.medium().a()]);
    let mut labels = [FEN.backup_flow_layout_new, existing.as_str()];
    let mut seg_w = 2.0 * ui.segment_width(LABEL_L.size, &labels);
    if seg_w > w {
        labels[1] = FEN.backup_flow_layout_existing_short;
        seg_w = 2.0 * ui.segment_width(LABEL_L.size, &labels);
    }
    let rows: Vec<(ChipView, bool)> = (0..l.vaults())
        .map(|v| {
            let cv = chip_of(l, v, None);
            let cw = board_screen::card_chip_w(ui, &cv, w);
            (cv, cw + tokens::GAP + seg_w <= w)
        })
        .collect();
    // A line of a row is a press target tall: the chip's, and the
    // segmented buttons' (40 dp drawn, Material's 48 dp target).
    let row_h = |beside: bool| {
        if beside {
            tokens::TOUCH
        } else {
            2.0 * tokens::TOUCH
        }
    };
    let list_h: f32 = rows.iter().map(|r| row_h(r.1)).sum();
    if !locked {
        ui.press_box(
            x,
            cy,
            w,
            list_h,
            Action::Board(BoardPress::Zone(Zone::List)),
        );
    }
    let off = (tokens::TOUCH - tokens::BOARD_CHIP) / 2.0;
    let seg_off = (tokens::TOUCH - SEGMENTED_H) / 2.0;
    let top = cy;
    for (v, (cv, beside)) in rows.iter().enumerate() {
        board_screen::card_chips(ui, fr, std::slice::from_ref(cv), (x, cy + off, w));
        let (sx, sy) = if *beside {
            (x + w - seg_w, cy + seg_off)
        } else {
            (x, cy + tokens::TOUCH + seg_off)
        };
        let form = |k: u8| Action::Board(BoardPress::Form(Zone::List, Thing::Vault(v), k));
        let options = [(labels[0], form(0)), (labels[1], form(1))];
        let chosen = usize::from(l.existing(v));
        ui.segmented(
            (sx, sy, SEGMENTED_H),
            LABEL_L.size,
            &options,
            Some(chosen),
            None,
        );
        cy += row_h(*beside);
    }
    top + list_h + tokens::GAP_SMALL - y
}

/// The sticks, each a box with the vaults on it. Returns their height.
fn stick_boxes(
    ui: &mut Ui,
    fr: &mut board_screen::CardFrame,
    l: &SetupLayout,
    (x, y, w): (f32, f32, f32),
    locked: bool,
) -> f32 {
    let sp = tokens::BOARD_STICK_PAD;
    let hh = tokens::BOARD_STICK_HEAD;
    let mut cy = y;
    for s in 0..l.sticks() {
        let chips: Vec<ChipView> = l.on(s).iter().map(|&v| chip_of(l, v, Some(s))).collect();
        let inner = board_screen::card_chips_h(ui, &chips, w - 2.0 * sp);
        let h = hh + inner.max(tokens::BOARD_CHIP) + sp;
        let zone = Zone::Card(s);
        board_screen::card_stick(
            ui,
            fr,
            &l.stick_name(s),
            (x, cy, w, h),
            Action::Board(BoardPress::Zone(zone)),
            !locked && l.held_takes(zone),
        );
        board_screen::card_chips(ui, fr, &chips, (x + sp, cy + hh, w - 2.0 * sp));
        cy += h + tokens::GAP_SMALL;
    }
    cy - y
}

/// The checks, a line each (§2.5). Returns their height.
fn check_lines(ui: &mut Ui, l: &SetupLayout, (x, y, w): (f32, f32, f32)) -> f32 {
    let mark = tokens::BOARD_MARK;
    let mut cy = y;
    for check in l.checks() {
        let (role, icon) = if check.blocks {
            (Role::Error, Icon::Error)
        } else {
            (Role::Warning, Icon::Warning)
        };
        ui.icon(x, cy + tokens::GAP_SMALL, mark, icon, mark, role);
        cy += ui.wrap_as(
            (x + mark + tokens::GAP, cy, w - mark - tokens::GAP),
            BODY_M,
            role,
            &check.line,
        ) + tokens::GAP_SMALL;
    }
    cy - y
}
