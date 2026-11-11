//! Make's screens, drawn from `&State`: Material 3 Tabs, Lists with a
//! Radio button, Segmented-style buttons, outlined Text fields, a Card
//! and a Progress indicator (`docs/DESIGN.md` §4). Drawing reads what
//! the worker sent back and works nothing out.

use aqrv::{Size, Step};
use faraday_draw::Color;
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, FIELD_PAD, GAP, GAP_SMALL, PAD, TITLE_M, TITLE_S, TOUCH,
};
use faraday_strings::{EN, fill};
use faraday_ui::tabs::{Tab as TabItem, TabKind};
use faraday_ui::{Role, Style, Ui, W};

use crate::samples::SAMPLES;
use crate::state::{Action, Field, OwnKind, Page, SIZES, Source, State, Tab};

/// The widest a column of content grows, in design units.
const COLUMN: f32 = 480.0;
/// The tallest the QR code is drawn, in design units.
const QR_MAX: f32 = 300.0;
/// A radio row's height.
const ROW: f32 = 56.0;
/// A radio mark's outer and inner radius.
const RADIO: f32 = 10.0;

type U<'a, 'b> = &'a mut Ui<'b, Action>;

/// Draws the whole window `w` by `h` units, scrolled by `state.offset`,
/// the indeterminate progress `phase` of the way along. Returns how far
/// the content scrolls.
pub fn window(ui: U, state: &State, (w, h): (f32, f32), phase: Option<f32>) -> f32 {
    ui.clear(Role::Surface);
    let cw = (w - 2.0 * PAD).min(COLUMN);
    let x = (w - cw) / 2.0;
    let top = TOUCH + PAD;
    let end = match state.tab {
        Tab::Make => match state.page {
            Page::Item => item(ui, state, (x, top - state.offset, cw)),
            Page::Settings => settings(ui, state, (x, top - state.offset, cw)),
            Page::Working => working(ui, state, (x, top - state.offset, cw), phase),
            Page::Code => code(ui, state, (x, top - state.offset, cw)),
        },
        Tab::Open => top - state.offset,
    };
    let content = end + state.offset + PAD;
    // The tabs stand over the content as it scrolls under them.
    ui.fill(0.0, 0.0, w, TOUCH, 0.0, Role::Surface);
    let tabs = [
        TabItem {
            label: EN.aqrv_tab_make,
            on: state.tab == Tab::Make,
            done: false,
            action: Action::Tab(Tab::Make),
        },
        TabItem {
            label: EN.aqrv_tab_open,
            on: state.tab == Tab::Open,
            done: false,
            action: Action::Tab(Tab::Open),
        },
    ];
    ui.tabs((0.0, 0.0, w, TOUCH), TabKind::Primary, &tabs);
    (content - h).max(0.0)
}

fn button(ui: U, (x, y, w): (f32, f32, f32), label: &str, style: Style, a: Action) -> f32 {
    ui.button(x, y, Some(w), 40.0, label, style, a);
    40.0 + GAP
}

/// A radio row: the mark, a name and, under it, a muted line.
fn radio(ui: U, (x, y, w): (f32, f32, f32), on: bool, name: &str, line: &str, a: Action) -> f32 {
    let (cx, cy) = (x + PAD + RADIO, y + ROW / 2.0);
    let ink = if on {
        Role::Primary
    } else {
        Role::OnSurfaceVariant
    };
    ui.dot(cx, cy, RADIO, ink);
    ui.dot(cx, cy, RADIO - 2.0, Role::Surface);
    if on {
        ui.dot(cx, cy, RADIO - 5.0, Role::Primary);
    }
    let tx = x + PAD + 2.0 * RADIO + PAD;
    let name = ui.fit_as(TITLE_M, name, w - (tx - x) - PAD);
    if line.is_empty() {
        ui.text_mid_as((tx, y, ROW), TITLE_M, Role::OnSurface, &name);
    } else {
        ui.text_as(tx, y + 8.0, TITLE_M, Role::OnSurface, &name);
        ui.text_as(tx, y + 8.0 + 24.0, BODY_M, Role::OnSurfaceVariant, line);
    }
    ui.press_box(x, y, w, ROW, a);
    ROW
}

/// An outlined text field with its label above, what is typed in it
/// (dots while `masked`). Returns its height with the gap under it.
fn field(
    ui: U,
    (x, y, w): (f32, f32, f32),
    label: &str,
    text: &str,
    masked: bool,
    focused: bool,
    a: Action,
) -> f32 {
    let by = y + ui.field_label(x, y, label);
    let fh = ui.field_h();
    let shown: String = if masked {
        text.chars().map(|_| '•').collect()
    } else {
        text.to_owned()
    };
    ui.field_box((x, by, w), focused, None);
    ui.typed(
        x + FIELD_PAD,
        by,
        fh,
        BODY_L.size,
        W::R,
        Role::OnSurface,
        &shown,
        w - 2.0 * FIELD_PAD,
        focused,
        a,
    );
    ui.press_box(x, by, w, fh, a);
    by + fh + GAP - y
}

fn kind_label(k: OwnKind) -> &'static str {
    match k {
        OwnKind::Entropy => EN.aqrv_kind_entropy,
        OwnKind::Codex32 => EN.aqrv_kind_codex32,
        OwnKind::Passphrase => EN.aqrv_kind_passphrase,
        OwnKind::Note => EN.aqrv_kind_note,
        OwnKind::RawKey => EN.aqrv_kind_raw_key,
    }
}

/// Make, the item (Material 3 Lists, Radio button, Text fields, Cards).
fn item(ui: U, st: &State, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    for (i, s) in SAMPLES.iter().enumerate() {
        let on = st.source == Source::Sample(i);
        y += radio(
            ui,
            (x, y, w),
            on,
            (s.name)(&EN),
            (s.kind)(&EN),
            Action::Sample(i),
        );
    }
    let own = st.source == Source::Own;
    y += radio(ui, (x, y, w), own, EN.aqrv_type_own, "", Action::Own);
    if own {
        y += GAP;
        let labels: Vec<(&str, Action)> = OwnKind::ALL
            .iter()
            .map(|k| (kind_label(*k), Action::Kind(*k)))
            .collect();
        let at = OwnKind::ALL.iter().position(|k| *k == st.own_kind);
        y += ui.choices((x, y, w, 40.0), TITLE_S.size, &labels, at) + GAP;
        let bad = !st.own_text.is_empty() && st.item().is_none();
        y += field(
            ui,
            (x, y, w),
            EN.aqrv_own_field,
            &st.own_text,
            false,
            st.focus == Some(Field::Own),
            Action::Field(Field::Own),
        );
        if bad {
            y += ui.wrap_as((x, y, w), BODY_S, Role::Error, EN.aqrv_not_an_item) + GAP;
        }
    }
    y += GAP;
    let inner = w - 2.0 * PAD;
    let lines = ui.wrap_height_as(BODY_M, EN.aqrv_test_data_line, inner);
    let ch = PAD + ui.line_as(TITLE_S) + GAP_SMALL + lines + PAD;
    ui.card(x, y, w, ch, Role::OutlineVariant);
    ui.text_as(
        x + PAD,
        y + PAD,
        TITLE_S,
        Role::OnSurface,
        EN.aqrv_test_data_title,
    );
    let ty = y + PAD + ui.line_as(TITLE_S) + GAP_SMALL;
    ui.wrap_as(
        (x + PAD, ty, inner),
        BODY_M,
        Role::OnSurfaceVariant,
        EN.aqrv_test_data_line,
    );
    y += ch + PAD;
    let style = if st.item().is_some() {
        Style::Primary
    } else {
        Style::PrimaryOff
    };
    y + button(ui, (x, y, w), EN.aqrv_next, style, Action::Next) - GAP
}

fn size_label(s: Size) -> &'static str {
    match s {
        Size::Small => EN.aqrv_size_29,
        Size::Large => EN.aqrv_size_33,
    }
}

fn memory_label(s: Step) -> &'static str {
    match s {
        Step::Mib64 => EN.aqrv_memory_64,
        Step::Mib128 => EN.aqrv_memory_128,
        Step::Mib256 => EN.aqrv_memory_256,
        Step::Mib512 => EN.aqrv_memory_512,
        Step::Gib1 => EN.aqrv_memory_1g,
    }
}

fn time_label(st: &State, s: Step) -> String {
    match st.seconds(s) {
        Some(t) => fill(EN.aqrv_seconds, &[&format!("{}", t.ceil().max(1.0))]),
        None => EN.aqrv_timing.to_owned(),
    }
}

/// Make, the settings (Segmented-style size buttons, Radio button list,
/// Text fields).
fn settings(ui: U, st: &State, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, EN.aqrv_settings_size);
    y += ui.line_as(TITLE_S) + GAP;
    for s in SIZES {
        let style = match (st.size == s, st.fits(s)) {
            (true, _) => Style::Selected,
            (false, true) => Style::Secondary,
            (false, false) => Style::Disabled,
        };
        y += button(ui, (x, y, w), size_label(s), style, Action::Size(s));
    }
    y += GAP;
    ui.text_as(
        x,
        y,
        TITLE_S,
        Role::OnSurfaceVariant,
        EN.aqrv_settings_memory,
    );
    y += ui.line_as(TITLE_S) + GAP_SMALL;
    for s in Step::ALL {
        let name = fill(EN.aqrv_memory_row, &[memory_label(s), &time_label(st, s)]);
        let line = if s == Step::DEFAULT {
            EN.aqrv_memory_suggested
        } else {
            ""
        };
        y += radio(ui, (x, y, w), st.step == s, &name, line, Action::Step(s));
    }
    y += GAP;
    let masked = !st.shown;
    y += field(
        ui,
        (x, y, w),
        EN.aqrv_passphrase,
        &st.pass,
        masked,
        st.focus == Some(Field::Pass),
        Action::Field(Field::Pass),
    );
    y += field(
        ui,
        (x, y, w),
        EN.aqrv_passphrase_again,
        &st.again,
        masked,
        st.focus == Some(Field::Again),
        Action::Field(Field::Again),
    );
    if !st.again.is_empty() && st.pass != st.again {
        y += ui.wrap_as((x, y, w), BODY_S, Role::Error, EN.aqrv_passphrase_differ) + GAP;
    }
    y += ui.wrap_as(
        (x, y, w),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.aqrv_passphrase_strength,
    ) + GAP;
    let eye = if st.shown { EN.aqrv_hide } else { EN.aqrv_show };
    y += button(ui, (x, y, w), eye, Style::Ghost, Action::Eye);
    if let Some(f) = st.failed {
        let line = match f {
            crate::worker::Fail::NotEnoughMemory => EN.aqrv_not_enough_memory,
            crate::worker::Fail::Refused => EN.aqrv_not_made,
        };
        y += ui.wrap_as((x, y, w), BODY_M, Role::Error, line) + GAP;
    }
    let ready = st.passphrase_ready() && st.fits(st.size);
    let style = if ready {
        Style::Primary
    } else {
        Style::PrimaryOff
    };
    y += button(ui, (x, y, w), EN.aqrv_make, style, Action::Make);
    y += button(ui, (x, y, w), EN.aqrv_back, Style::Secondary, Action::Back);
    y - GAP
}

/// Make, working: an indeterminate Progress indicator, since Argon2id
/// reports no progress.
fn working(ui: U, st: &State, (x, y0, w): (f32, f32, f32), phase: Option<f32>) -> f32 {
    let mut y = y0;
    let line = fill(EN.aqrv_working, &[memory_label(st.step)]);
    y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &line) + GAP;
    ui.progress((x, y, w), None, phase);
    y += 4.0 + 2.0 * GAP;
    y + button(
        ui,
        (x, y, w),
        EN.aqrv_cancel,
        Style::Secondary,
        Action::Cancel,
    ) - GAP
}

/// Make, the code: the QR drawn from the kept matrix, then its size, its
/// step and its bytes in hex.
fn code(ui: U, st: &State, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    let Some(made) = st.made.as_ref() else {
        return y;
    };
    let n = made.matrix.size();
    let side = w.min(QR_MAX);
    let m = (side / (n + 8) as f32).floor().max(1.0);
    let full = m * (n + 8) as f32;
    let qx = x + (w - full) / 2.0;
    ui.fill(qx, y, full, full, 0.0, Color::WHITE);
    for row in 0..n {
        for col in 0..n {
            if made.matrix.module(col, row) {
                let mx = qx + (col + 4) as f32 * m;
                ui.fill(mx, y + (row + 4) as f32 * m, m, m, 0.0, Color::BLACK);
            }
        }
    }
    y += full + GAP;
    let dims = fill(EN.aqrv_code_size, &[&n.to_string(), &n.to_string()]);
    y += ui.wrap_as((x, y, w), BODY_L, Role::OnSurface, &dims) + GAP_SMALL;
    let step = fill(EN.aqrv_code_step, &[memory_label(made.step)]);
    y += ui.wrap_as((x, y, w), BODY_L, Role::OnSurface, &step) + GAP;
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, EN.aqrv_code_bytes);
    y += ui.line_as(TITLE_S) + GAP_SMALL;
    for chunk in made.bytes.chunks(8) {
        let hex: String = chunk.iter().map(|b| format!("{b:02x}")).collect();
        y += ui.code_height(BODY_M.size) + GAP_SMALL;
        ui.code_box(
            x,
            y - ui.code_height(BODY_M.size) - GAP_SMALL,
            BODY_M.size,
            &hex,
        );
    }
    y += GAP;
    y += button(
        ui,
        (x, y, w),
        EN.aqrv_make_again,
        Style::Primary,
        Action::Again,
    );
    y - GAP
}
