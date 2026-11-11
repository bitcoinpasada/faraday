//! Make's screens, drawn from `&State`: Material 3 Tabs, Lists with a
//! Radio button, Segmented-style buttons, outlined Text fields, a Card
//! and a Progress indicator (`docs/DESIGN.md` §4). Drawing reads what
//! the worker sent back and works nothing out.

use aqrv::{Size, Step};
use faraday_draw::Color;
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, FIELD_PAD, GAP, GAP_SMALL, LABEL_L, PAD, TITLE_M, TITLE_S, TOUCH,
};
use faraday_draw::widgets::Icon;
use faraday_strings::{EN, fill};
use faraday_ui::tabs::{Tab as TabItem, TabKind};
use faraday_ui::{Glyph, Role, Style, Ui, W};

use crate::samples::SAMPLES;
use crate::state::{Action, Dialog, Field, OpenStage, OwnKind, Page, SIZES, Source, State, Tab};
use crate::worker::{Label, Value};

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
        Tab::Open => open(ui, state, (x, top - state.offset, cw), phase),
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
    match state.dialog {
        Some(Dialog::About) => about(ui, (w, h)),
        Some(Dialog::Working) => working_dialog(ui, state, (w, h)),
        None => {}
    }
    if let Some(text) = state.snack.as_deref() {
        ui.snackbar_closable((0.0, w, h), text, (1.0, 0.0), Action::CloseSnack);
    }
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
        Size::Tiny => EN.aqrv_size_25,
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
        Some(t) => fill(EN.aqrv_seconds, &[&seconds(t)]),
        None => EN.aqrv_timing.to_owned(),
    }
}

/// A time in seconds, rounded up: one decimal under 10 s, else whole.
fn seconds(t: f32) -> String {
    if t < 10.0 {
        format!("{:.1}", (t * 10.0).ceil().max(1.0) / 10.0)
    } else {
        format!("{}", t.ceil())
    }
}

/// Make, the settings (Segmented-style size buttons, Radio button list,
/// Text fields).
fn settings(ui: U, st: &State, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, EN.aqrv_settings_size);
    y += ui.line_as(TITLE_S) + GAP;
    // A size the item does not fit does nothing when pressed.
    let options: Vec<(&str, Action)> = SIZES
        .iter()
        .map(|&s| (size_label(s), Action::Size(s)))
        .collect();
    let chosen = SIZES.iter().position(|&s| s == st.size);
    let off: Vec<bool> = SIZES.iter().map(|&s| !st.fits(s)).collect();
    y += ui.choices_with((x, y, w, 40.0), LABEL_L.size, &options, chosen, &off) + GAP;
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
    if let Some(text) = &made.text {
        ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, EN.aqrv_text);
        y += ui.line_as(TITLE_S) + GAP_SMALL;
        let chars: Vec<char> = text.chars().collect();
        for chunk in chars.chunks(16) {
            let line: String = chunk.iter().collect();
            ui.code_box(x, y, BODY_M.size, &line);
            y += ui.code_height(BODY_M.size) + GAP_SMALL;
        }
        y += GAP;
    }
    y += button(
        ui,
        (x, y, w),
        EN.aqrv_save_picture,
        Style::Primary,
        Action::SavePicture,
    );
    y += button(
        ui,
        (x, y, w),
        EN.aqrv_save_file,
        Style::Secondary,
        Action::SaveFile,
    );
    y += button(
        ui,
        (x, y, w),
        EN.aqrv_show_working,
        Style::Secondary,
        Action::ShowWorking,
    );
    y += button(
        ui,
        (x, y, w),
        EN.aqrv_make_again,
        Style::Secondary,
        Action::Again,
    );
    y += button(ui, (x, y, w), EN.aqrv_about, Style::Ghost, Action::About);
    y - GAP
}

fn item_label(l: Label) -> &'static str {
    match l {
        Label::Words => EN.aqrv_item_words,
        Label::Fingerprint => EN.aqrv_item_fingerprint,
        Label::String => EN.aqrv_item_string,
        Label::Threshold => EN.aqrv_item_threshold,
        Label::Identifier => EN.aqrv_item_identifier,
        Label::Index => EN.aqrv_item_index,
        Label::Secret => EN.aqrv_item_secret,
        Label::Share => EN.aqrv_item_share,
        Label::Text => EN.aqrv_item_text,
        Label::Key => EN.aqrv_item_key,
    }
}

/// A muted label with its value under it. Returns the height taken.
fn pair(ui: U, (x, y, w): (f32, f32, f32), label: &str, value: &str) -> f32 {
    let mut h = ui.line_as(TITLE_S);
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, label);
    if !value.is_empty() {
        h += GAP_SMALL;
        h += ui.wrap_as((x, y + h, w), BODY_L, Role::OnSurface, value);
    }
    h + GAP
}

/// Open (Material 3 Text field, Buttons, Lists): the path, then what
/// came of the file, the passphrase and the item.
fn open(ui: U, st: &State, (x, y0, w): (f32, f32, f32), phase: Option<f32>) -> f32 {
    let o = &st.open;
    let mut y = y0;
    y += field(
        ui,
        (x, y, w),
        EN.aqrv_open_path,
        &o.path,
        false,
        st.focus == Some(Field::Path),
        Action::Field(Field::Path),
    );
    let style = if o.path.trim().is_empty() {
        Style::PrimaryOff
    } else {
        Style::Primary
    };
    y += button(ui, (x, y, w), EN.aqrv_open_file, style, Action::ReadPath);
    let error = |ui: U, y: f32, text: &str| ui.wrap_as((x, y, w), BODY_M, Role::Error, text) + GAP;
    match o.stage {
        OpenStage::Empty => {}
        OpenStage::Reading => {
            y += ui.wrap_as((x, y, w), BODY_L, Role::OnSurface, EN.aqrv_reading) + GAP;
        }
        OpenStage::Unreadable => y += error(ui, y, EN.aqrv_file_unread),
        OpenStage::NotACode => y += error(ui, y, EN.aqrv_not_a_code),
        _ => {
            if let Some(h) = o.header {
                y += ui.wrap_as((x, y, w), BODY_L, Role::OnSurface, size_label(h.size)) + GAP_SMALL;
                let step = fill(EN.aqrv_code_step, &[memory_label(h.step)]);
                y += ui.wrap_as((x, y, w), BODY_L, Role::OnSurface, &step) + GAP;
            }
            match o.stage {
                OpenStage::Working => {
                    let step = o.header.map_or("", |h| memory_label(h.step));
                    let line = fill(EN.aqrv_opening, &[step]);
                    y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &line) + GAP;
                    ui.progress((x, y, w), None, phase);
                    y += 4.0 + 2.0 * GAP;
                    y += button(
                        ui,
                        (x, y, w),
                        EN.aqrv_cancel,
                        Style::Secondary,
                        Action::Cancel,
                    );
                }
                OpenStage::Opened => {
                    if let Some(shown) = o.shown.as_ref() {
                        for (label, value) in &shown.lines {
                            y += pair(ui, (x, y, w), item_label(*label), value);
                        }
                    }
                    y += button(
                        ui,
                        (x, y, w),
                        EN.aqrv_open_another,
                        Style::Secondary,
                        Action::OpenAnother,
                    );
                }
                stage => {
                    y += field(
                        ui,
                        (x, y, w),
                        EN.aqrv_passphrase,
                        &o.pass,
                        !st.shown,
                        st.focus == Some(Field::OpenPass),
                        Action::Field(Field::OpenPass),
                    );
                    match stage {
                        OpenStage::DoesNotOpen => y += error(ui, y, EN.aqrv_does_not_open),
                        OpenStage::NotEnoughMemory => y += error(ui, y, EN.aqrv_not_enough_memory),
                        _ => {}
                    }
                    let eye = if st.shown { EN.aqrv_hide } else { EN.aqrv_show };
                    y += button(ui, (x, y, w), eye, Style::Ghost, Action::Eye);
                    let style = if o.pass.is_empty() {
                        Style::PrimaryOff
                    } else {
                        Style::Primary
                    };
                    y += button(ui, (x, y, w), EN.aqrv_unlock, style, Action::Unlock);
                }
            }
        }
    }
    y += button(ui, (x, y, w), EN.aqrv_about, Style::Ghost, Action::About);
    y - GAP
}

fn value_label(v: Value) -> String {
    match v {
        Value::Pw => EN.aqrv_w_pw.to_owned(),
        Value::Prk => EN.aqrv_w_prk.to_owned(),
        Value::KEnc => EN.aqrv_w_k_enc.to_owned(),
        Value::KMac => EN.aqrv_w_k_mac.to_owned(),
        Value::Keystream => EN.aqrv_w_keystream.to_owned(),
        Value::P => EN.aqrv_w_p.to_owned(),
        Value::C => EN.aqrv_w_c.to_owned(),
        Value::Message => EN.aqrv_w_message.to_owned(),
        Value::Tag(k) => fill(EN.aqrv_w_tag, &[&format!("{k:02x}")]),
    }
}

/// About (Material 3 Dialog).
fn about(ui: U, (w, h): (f32, f32)) {
    let head = (
        Glyph::from(Icon::Info),
        Role::OnSurface.paint(),
        EN.aqrv_about_title,
    );
    faraday_ui::sheet::dialog(
        ui,
        (w, h, COLUMN),
        head,
        &mut |ui, x, y, iw| {
            ui.wrap_as(
                (x, y, iw),
                BODY_M,
                Role::OnSurfaceVariant,
                EN.aqrv_about_text,
            )
        },
        &[(EN.aqrv_close, Style::Primary, Action::CloseDialog)],
    );
}

/// The working of the code made (Material 3 Dialog): each value of
/// `aqrv::workings`, as the worker sent it back.
fn working_dialog(ui: U, st: &State, (w, h): (f32, f32)) {
    let head = (
        Glyph::from(Icon::Info),
        Role::OnSurface.paint(),
        EN.aqrv_working_title,
    );
    ui.offset = st.dialog_offset;
    faraday_ui::sheet::dialog(
        ui,
        (w, h, COLUMN),
        head,
        &mut |ui, x, y, iw| match st.working.as_ref() {
            None => ui.wrap_as(
                (x, y, iw),
                BODY_M,
                Role::OnSurfaceVariant,
                EN.aqrv_working_wait,
            ),
            Some(values) => {
                let mut cy = y;
                for (v, hex) in values {
                    cy += pair(ui, (x, cy, iw), &value_label(*v), hex);
                }
                cy - y
            }
        },
        &[(EN.aqrv_close, Style::Primary, Action::CloseDialog)],
    );
}
