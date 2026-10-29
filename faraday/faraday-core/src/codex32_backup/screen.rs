//! The checklist's codex32 item as it is drawn (`docs/CODEX32.md` §9.2,
//! §9.4, §9.7): the strings and where each is kept, one string at a time
//! to write and type back, and the set's actions. Drawing reads the
//! item's state, the plan through the memo and the key's facts; it works
//! nothing out.

use faraday_draw::tokens as t;
use faraday_draw::tokens::{BODY_L, BODY_S, TypeRole};
use faraday_draw::tokens::{BODY_M, TITLE_M, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_ui::keyboard::Keys as OskKeys;

use super::{Found, Press, SetPress, SetStep, Strings};
use crate::Action;
use crate::drawing::View;
use crate::plan::codex32::{C32, PaperCopy, index_char, places_of_string};
use crate::screens::buttons_and_next;
use crate::strings::{EN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};

/// A press on the item.
fn a(p: Press) -> Action {
    Action::BC32(p)
}

/// A set action's press.
fn set(p: SetPress) -> Action {
    Action::BC32(Press::Set(p))
}

/// A share index as the item names it: upper case.
fn name(j: usize) -> String {
    index_char(j).to_ascii_uppercase().to_string()
}

/// The facts line of a form: "Identifier · w0za · 2 of 3 · 48 characters".
fn facts(f: &C32) -> String {
    let split = if f.k == 0 {
        EN.c32_secret_alone.to_string()
    } else {
        fill(EN.c32_k_of_n, &[&f.k.to_string(), &f.n().to_string()])
    };
    let out = fill(EN.c32_facts, &[&split, &f.length.to_string()]);
    fill(EN.c32_facts_id, &[&f.id_text(), &out])
}

/// Filter chips, one per choice: outlined, the chosen one filled with a
/// check mark (Material 3's filter chip), each in a touch target.
/// Returns the height used.
fn chips(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, bool, Action)]) -> f32 {
    let row = t::TOUCH;
    let (mut cx, mut cy) = (x, y + (row - t::C32_CHIP) / 2.0);
    for &(label, on, action) in items {
        let lead = if on { t::C32_CHECK + t::GAP_SMALL } else { 0.0 };
        let cw = ui.measure_as(TITLE_S, label) + 2.0 * t::PAD + lead;
        if cx > x && cx + cw > x + w {
            cx = x;
            cy += row;
        }
        ui.fill(
            cx,
            cy,
            cw,
            t::C32_CHIP,
            t::RADIUS_SMALL,
            if on {
                Role::SurfaceContainerHigh
            } else {
                Role::SurfaceContainerHighest
            },
        );
        ui.stroke(
            cx,
            cy,
            cw,
            t::C32_CHIP,
            t::RADIUS_SMALL,
            if on {
                Role::Primary
            } else {
                Role::OutlineVariant
            },
        );
        // Its layer over the fill and under the label.
        ui.press_to(
            Shape::new(cx, cy, cw, t::C32_CHIP, t::RADIUS_SMALL),
            (cx, cy - (row - t::C32_CHIP) / 2.0, cw, row),
            Role::OnSurface.paint(),
            action,
        );
        let mut tx = cx + t::PAD;
        if on {
            ui.icon(
                tx - t::GAP_SMALL,
                cy + (t::C32_CHIP - t::C32_CHECK) / 2.0,
                t::C32_CHECK,
                Icon::Check,
                t::C32_CHECK,
                Role::Primary,
            );
            tx += lead - t::GAP_SMALL;
        }
        ui.text_mid_as((tx, cy, t::C32_CHIP), TITLE_S, Role::OnSurface, label);
        cx += cw + t::GAP;
    }
    cy + t::C32_CHIP + (row - t::C32_CHIP) / 2.0 - y
}

/// The string typed so far after `ms1`, in groups of four, in a field
/// edged in `error` while it differs. Returns the height.
fn typed_field(ui: &mut Ui, x: f32, y: f32, w: f32, st: &Strings, prefix: &str) -> f32 {
    let mut whole = String::from(prefix);
    whole.push_str(&st.typed);
    let shown = faraday_ui::grouped(&whole);
    let wrong = matches!(st.found, Some(Found::Differs(_)));
    ui.fill(x, y, w, t::FIELD, t::RADIUS_SMALL, Role::Surface);
    ui.stroke(
        x,
        y,
        w,
        t::FIELD,
        t::RADIUS_SMALL,
        if wrong {
            Role::Error
        } else {
            Role::OutlineVariant
        },
    );
    let fit = ui.fit_as(BODY_M.mono(), &shown, w - 2.0 * t::GAP);
    ui.text_mid_as(
        (x + t::GAP, y, t::FIELD),
        BODY_M.mono(),
        Role::OnSurface,
        &fit,
    );
    t::FIELD + t::GAP
}

/// What the string typed was found to be, in `ok` or `error`. Returns the
/// height.
fn found_line(ui: &mut Ui, x: f32, y: f32, w: f32, st: &Strings) -> f32 {
    match &st.found {
        Some(Found::Holds(l)) => ui.wrap_as((x, y, w), BODY_M, Role::Ok, l) + t::GAP,
        Some(Found::Differs(l)) => ui.wrap_as((x, y, w), BODY_M, Role::Error, l) + t::GAP,
        None => 0.0,
    }
}

/// The codex32 keyboard, its keys this item's. Returns its height.
fn keyboard(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let kind = KeyboardKind::Codex32;
    faraday_ui::keyboard::keyboard(ui, x, w, y, kind, OskKeys::default(), ui.compact, |p| {
        a(Press::Key(p))
    });
    faraday_ui::keyboard::height(kind) + t::GAP
}

/// The item's note, in `warning`. Returns the height.
fn note(ui: &mut Ui, x: f32, y: f32, w: f32, st: &Strings) -> f32 {
    st.note.as_deref().map_or(0.0, |n| {
        ui.wrap_as((x, y, w), BODY_M, Role::Warning, n) + t::GAP
    })
}

/// Seed `i`'s codex32 item, under its title. `foot` says whether the
/// item's own foot (Check my copy) is drawn by the checklist
/// after it: false while a string or a set action is open, which draw
/// their own. Returns the height used and that.
pub(crate) fn item(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, i: usize) -> (f32, bool) {
    let Some(b) = app.backup() else {
        return (0.0, true);
    };
    let Some(form) = b.answers.c32(i) else {
        return (0.0, true);
    };
    let Some(st) = b.c32.as_ref().filter(|st| st.seed == i) else {
        return (0.0, true);
    };
    let shape = app.plan_shape(b.wallet);
    let mapped = app.plan_mapped(&shape, &b.answers);
    let place = |p: usize| app.place_name(p);
    if !matches!(st.set, SetStep::None) {
        return (set_step(ui, x, y, w, form, st), false);
    }
    if let Some(j) = st.at {
        let places: Vec<String> = places_of_string(&mapped, i, j)
            .into_iter()
            .map(place)
            .collect();
        let copy = crate::plan::codex32::paper_copy(&mapped, i, j);
        return (
            one_string(ui, (x, y, w), (form, st), (j, copy), &places.join(", ")),
            false,
        );
    }
    let mut cy = y;
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurfaceVariant, &facts(form)) + t::GAP;
    if form.length == 127 {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_M,
            Role::OnSurfaceVariant,
            EN.c32_holds_the_seed,
        ) + t::GAP;
    }
    let paper = crate::plan::codex32::paper_strings(&b.answers, &mapped, i);
    for &j in &paper {
        let places: Vec<String> = places_of_string(&mapped, i, j)
            .into_iter()
            .map(place)
            .collect();
        let done = b.c32_checked.contains(&(i, j));
        let line = fill(EN.c32_string_at, &[&name(j), &places.join(", ")]);
        let press = Some(a(Press::Open(u8::try_from(j).unwrap_or(0))));
        string_row(ui, (x, cy, w), &line, done, press);
        cy += t::TOUCH;
    }
    // How many strings of a set that exists there are (§9.4).
    if form.k >= 2 {
        let id = form.id_text();
        crate::screens::stepper(
            ui,
            x,
            cy,
            w,
            &fill(EN.c32r_how_many, &[&id]),
            form.n(),
            a(Press::Exist(-1)),
            a(Press::Exist(1)),
            true,
        );
        cy += t::STEPPER_ROW;
    }
    if form.unknown > 0 {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_M,
            Role::OnSurfaceVariant,
            &fill(EN.c32_more_unknown, &[&form.unknown.to_string()]),
        ) + t::GAP;
    }
    match st.rebuilds {
        Some(true) => cy += ui.wrap_as((x, cy, w), BODY_M, Role::Ok, EN.c32_rebuilds) + t::GAP,
        Some(false) => {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, EN.c32_rebuilds_not) + t::GAP
        }
        None => {}
    }
    cy += note(ui, x, cy, w, st);
    // The set's actions, once the set is split (§9.7).
    if form.k >= 2 {
        cy += set_actions(ui, x, cy, w, form);
    }
    (cy - y, true)
}

/// One string open (§6.6, §9.4, §9.5): the string where it was made
/// here, in groups of four or as its Codex32QR to copy, square by square
/// in blocks; where the plan draws its Codex32QR by hand, the sheet it is
/// drawn on and **Scan my copy**; typed back on the codex32 keyboard
/// where it is written; Skip and Continue. Returns the height.
fn one_string(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (form, st): (&C32, &Strings),
    (j, copy): (usize, Option<PaperCopy>),
    places: &str,
) -> f32 {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let mut cy = y;
    let head = fill(EN.c32_share_at, &[&name(j), places]);
    cy += ui.wrap_as((x, cy, w), TITLE_M, ink, &head) + t::GAP;
    let drawn = form.length == 48 && copy.is_some_and(PaperCopy::drawn);
    if drawn {
        cy += ui.wrap_as((x, cy, w), BODY_M, muted, EN.c32_qr_sheet) + t::GAP;
    }
    if let Some(s) = st.known(j) {
        if s.len() == 48 {
            cy += chips(
                ui,
                x,
                cy,
                w,
                &[
                    (EN.codex32_string, st.qr.is_none(), a(Press::Qr)),
                    (EN.codex32_codex32qr, st.qr.is_some(), a(Press::Qr)),
                ],
            );
        }
        match &st.qr {
            Some(q) => {
                let g = crate::screens::CopyGrid {
                    m: q,
                    printed: |x, y, _| super::printed(x, y),
                    lettered: true,
                    rows: st.rows,
                    pin: usize::from(st.block),
                    pick: |b| a(Press::BlockAt(u8::try_from(b).unwrap_or(u8::MAX))),
                    mode: |r| a(Press::Rows(r)),
                };
                cy += crate::screens::copy_grid(ui, x, cy, w, &g);
            }
            None => {
                let g = faraday_ui::grouped(s);
                let role = [BODY_L, BODY_M]
                    .into_iter()
                    .map(TypeRole::mono)
                    .find(|r| ui.wrap_lines_as(*r, &g, w) <= 3)
                    .unwrap_or(BODY_S.mono());
                cy += ui.wrap_as((x, cy, w), role, ink, &g) + t::GAP;
            }
        }
    }
    // Written: typed back from the paper. Only drawn and made here:
    // scanned. A string of a set that exists is typed from its paper
    // first, which shows its Codex32QR to draw.
    let written = copy.is_none_or(PaperCopy::written) || !drawn || st.made(j).is_none();
    if written {
        if st.made(j).is_none() {
            cy += ui.wrap_as((x, cy, w), BODY_M, muted, EN.c32_type_from_paper) + t::GAP;
        }
        cy += typed_field(ui, x, cy, w, st, "ms1");
    }
    cy += found_line(ui, x, cy, w, st);
    if written && !matches!(st.found, Some(Found::Holds(_))) {
        cy += keyboard(ui, x, cy, w);
    }
    if st.skip_ask {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Warning, EN.codex32_skip_caution) + t::GAP;
    }
    cy += note(ui, x, cy, w, st);
    let mut rows = vec![
        (EN.c32_all_strings, Style::Ghost, a(Press::List)),
        (EN.codex32_skip, Style::Ghost, a(Press::Skip)),
    ];
    if drawn {
        rows.insert(
            0,
            (EN.c32_scan_my_copy, Style::Secondary, a(Press::ScanCopy)),
        );
    }
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &rows,
        Some((EN.item_action_continue, a(Press::Next))),
    );
    cy - y
}

/// The set action under way (§9.7). Returns the height.
fn set_step(ui: &mut Ui, x: f32, y: f32, w: f32, form: &C32, st: &Strings) -> f32 {
    let mut cy = y;
    let id = form.id_text();
    let mut next: Option<(&str, Action)> = None;
    let made_label;
    match &st.set {
        SetStep::None => {}
        SetStep::Typing { index } => {
            let title = match index {
                Some(j) if form.indices.contains(&usize::from(*j)) => {
                    fill(EN.c32_replacing, &[&name(usize::from(*j)), &id])
                }
                Some(j) => fill(EN.c32_adding, &[&name(usize::from(*j)), &id]),
                None => String::new(),
            };
            cy += heading(ui, x, cy, w, &title);
            let count = fill(
                EN.c32_typed_in,
                &[&st.held.len().to_string(), &form.need().to_string(), &id],
            );
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurfaceVariant, &count) + t::GAP;
            cy += typed_field(ui, x, cy, w, st, "ms1");
            cy += found_line(ui, x, cy, w, st);
            if st.held.len() < form.need() {
                cy += keyboard(ui, x, cy, w);
            } else if let Some(j) = index {
                made_label = fill(EN.c32_make_share, &[&name(usize::from(*j))]);
                next = Some((made_label.as_str(), set(SetPress::MakeNew)));
            }
        }
        SetStep::Marking(marks) => {
            cy += heading(ui, x, cy, w, &fill(EN.c32_rotate_title, &[&id]));
            cy += marking(ui, x, cy, w, marks);
            next = Some((EN.item_action_continue, set(SetPress::Counted)));
        }
        SetStep::Destroying { held, .. } => {
            cy += heading(ui, x, cy, w, &fill(EN.c32_rotate_title, &[&id]));
            for &(j, done) in held {
                let jj = u8::try_from(j).unwrap_or(0);
                let label = fill(EN.c32_destroyed, &[&name(j)]);
                cy += chips(
                    ui,
                    x,
                    cy,
                    w,
                    &[(label.as_str(), done, set(SetPress::Destroyed(jj)))],
                );
            }
        }
        SetStep::Scheme(k, n) => {
            cy += heading(ui, x, cy, w, &fill(EN.c32_new_set_of, &[&id]));
            cy += scheme(ui, x, cy, w, (*k, *n), st);
            next = Some((EN.c32_make_new_set, set(SetPress::MakeNew)));
        }
    }
    cy += note(ui, x, cy, w, st);
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[(EN.c32_leave, Style::Ghost, set(SetPress::Leave))],
        next,
    );
    cy - y
}

/// A string on paper in the item's list, its places, and Checked or Not
/// checked; `press` opens it.
fn string_row(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    line: &str,
    done: bool,
    press: Option<Action>,
) {
    ui.dot(
        x + t::GAP_SMALL,
        y + t::TOUCH / 2.0,
        t::GAP_SMALL,
        if done {
            Role::Ok
        } else {
            Role::OnSurfaceVariant
        },
    );
    let state = if done {
        EN.c32_checked
    } else {
        EN.c32_not_checked
    };
    let sw = ui.measure_as(BODY_M, state);
    let fit = ui.fit_as(TITLE_S, line, w - sw - 2.0 * t::GAP - t::GAP);
    ui.text_mid_as(
        (x + 2.0 * t::GAP, y, t::TOUCH),
        TITLE_S,
        Role::OnSurface,
        &fit,
    );
    ui.text_right_as(
        (x + w, y, t::TOUCH),
        BODY_M,
        if done {
            Role::Ok
        } else {
            Role::OnSurfaceVariant
        },
        state,
    );
    if let Some(p) = press {
        ui.hit(x, y, w, t::TOUCH, p);
    }
}

/// The set's actions under the item's list (§9.7): **Add a share**,
/// **Rotate the set**, and **Replace** for each string. Returns the
/// height.
fn set_actions(ui: &mut Ui, x: f32, y: f32, w: f32, form: &C32) -> f32 {
    let mut cy = y + t::GAP;
    crate::screens::section_label(ui, x, cy, EN.c32_the_set);
    cy += t::PAD + t::GAP_SMALL;
    let rows: Vec<(&str, Style, Action)> = vec![
        (EN.c32_add_a_share, Style::Secondary, set(SetPress::Add)),
        (EN.c32_rotate, Style::Secondary, set(SetPress::Rotate)),
    ];
    cy += faraday_ui::parts::wrap_buttons(ui, x, cy, w, t::TEXT_FIELD, &rows);
    let replace: Vec<String> = form
        .indices
        .iter()
        .map(|&j| fill(EN.c32_replace, &[&name(j)]))
        .collect();
    let items: Vec<(&str, bool, Action)> = form
        .indices
        .iter()
        .zip(&replace)
        .map(|(&j, l)| {
            (
                l.as_str(),
                false,
                set(SetPress::Replace(u8::try_from(j).unwrap_or(0))),
            )
        })
        .collect();
    cy += chips(ui, x, cy, w, &items);
    cy - y
}

/// A set action's heading. Returns its height.
fn heading(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, text) + t::GAP
}

/// Rotate the set's marks: each old string held, lost or exposed.
/// Returns the height.
fn marking(ui: &mut Ui, x: f32, y: f32, w: f32, marks: &[(usize, u8)]) -> f32 {
    let mut cy =
        y + ui.wrap_as(
            (x, y, w),
            BODY_M,
            Role::OnSurfaceVariant,
            EN.c32_rotate_mark,
        ) + t::GAP;
    for &(j, m) in marks {
        let jj = u8::try_from(j).unwrap_or(0);
        ui.text_mid_as((x, cy, t::TOUCH), TITLE_S, Role::OnSurface, &name(j));
        cy += chips(
            ui,
            x + t::LIST_INDENT,
            cy,
            w - t::LIST_INDENT,
            &[
                (EN.c32_held, m == 0, set(SetPress::Mark(jj, 0))),
                (EN.c32_lost, m == 1, set(SetPress::Mark(jj, 1))),
                (EN.c32_exposed, m == 2, set(SetPress::Mark(jj, 2))),
            ],
        );
    }
    cy - y
}

/// The new set's threshold and strings, and its identifier typed.
/// Returns the height.
fn scheme(ui: &mut Ui, x: f32, y: f32, w: f32, (k, n): (u8, u8), st: &Strings) -> f32 {
    let mut cy = y;
    crate::screens::stepper(
        ui,
        x,
        cy,
        w,
        EN.codex32_threshold,
        usize::from(k),
        set(SetPress::K(-1)),
        set(SetPress::K(1)),
        true,
    );
    cy += t::STEPPER_ROW;
    crate::screens::stepper(
        ui,
        x,
        cy,
        w,
        EN.codex32_strings,
        usize::from(n),
        set(SetPress::N(-1)),
        set(SetPress::N(1)),
        true,
    );
    cy += t::STEPPER_ROW;
    crate::screens::section_label(ui, x, cy, EN.c32_new_identifier);
    cy += t::PAD + t::GAP_SMALL;
    cy += typed_field(ui, x, cy, w, st, "");
    cy += keyboard(ui, x, cy, w);
    cy - y
}

/// A vault's codex32 share record (`docs/CODEX32.md` §9.5) as its list
/// names it: by its index and the key its set makes, "Share A of
/// fab6868a", and "codex32 name · threshold 2"; read from the record's
/// public fields and the string's header. The title, the line, and that
/// the title is data.
pub(crate) fn vault_row(r: faraday_keys::vault::RecordView<'_>) -> (String, String, bool) {
    use faraday_vault::records::field;
    let fp = record_fp(r, field::CODEX32_KEY);
    match faraday_keys::vault::codex32_share_header(r) {
        Some((k, id, index)) => (
            fill(
                EN.c32_vault_row,
                &[&char::from(index).to_ascii_uppercase().to_string(), &fp],
            ),
            fill(
                EN.c32_vault_row_line,
                &[&String::from_utf8_lossy(&id), &k.to_string()],
            ),
            true,
        ),
        None => (EN.vault_record.to_string(), String::new(), false),
    }
}

/// The key fingerprint a vault record's public field `f` names, as text;
/// empty where it names none.
pub(crate) fn record_fp(r: faraday_keys::vault::RecordView<'_>, f: u8) -> String {
    r.public(f)
        .and_then(|b| <[u8; 4]>::try_from(b).ok())
        .map(|b| crate::wallet::fp_text(faraday_bip::keys::Fingerprint(b)))
        .unwrap_or_default()
}
