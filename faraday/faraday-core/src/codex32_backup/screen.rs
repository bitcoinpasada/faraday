//! The checklist's codex32 item as it is drawn (`docs/CODEX32.md` §9.2,
//! §9.4, §9.7): the strings and where each is kept, one string at a time
//! to write and type back, and the set's actions. Drawing reads the
//! item's state, the plan through the memo and the key's facts; it works
//! nothing out.

use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_draw::widgets::keyboard::KeyboardKind;
use faraday_ui::keyboard::Keys as OskKeys;

use super::{Found, Press, SetPress, SetStep, Strings};
use crate::Action;
use crate::drawing::View;
use crate::plan::codex32::{C32, index_char, places_of_string};
use crate::screens::buttons_and_next;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};

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
    let mut out = fill(EN.c32_facts, &[&split, &f.length.to_string()]);
    if let Some(id) = f.id_text() {
        out = fill(EN.c32_facts_id, &[&id, &out]);
    }
    out
}

/// Filter chips, one per choice: outlined, the chosen one filled with a
/// check mark (Material 3's filter chip), each in a touch target.
/// Returns the height used.
fn chips(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, bool, Action)]) -> f32 {
    let row = t::TOUCH;
    let (mut cx, mut cy) = (x, y + (row - t::C32_CHIP) / 2.0);
    for &(label, on, action) in items {
        let lead = if on { t::C32_CHECK + t::GAP_SMALL } else { 0.0 };
        let cw = ui.measure(t::LABEL, W::S, label) + 2.0 * t::PAD + lead;
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
            if on { INNER } else { BG },
        );
        ui.stroke(
            cx,
            cy,
            cw,
            t::C32_CHIP,
            t::RADIUS_SMALL,
            if on { ACCENT } else { BORDER },
        );
        let mut tx = cx + t::PAD;
        if on {
            ui.icon(
                tx - t::GAP_SMALL,
                cy + (t::C32_CHIP - t::C32_CHECK) / 2.0,
                t::C32_CHECK,
                Icon::Check,
                t::C32_CHECK,
                ACCENT,
            );
            tx += lead - t::GAP_SMALL;
        }
        ui.text_mid(tx, cy, t::C32_CHIP, t::LABEL, W::S, TEXT, label);
        ui.hit_quiet(cx, cy - (row - t::C32_CHIP) / 2.0, cw, row, action);
        cx += cw + t::GAP;
    }
    cy + t::C32_CHIP + (row - t::C32_CHIP) / 2.0 - y
}

/// The string typed so far after `ms1`, in groups of four, in a field
/// edged in `ERR` while it differs. Returns the height.
fn typed_field(ui: &mut Ui, x: f32, y: f32, w: f32, st: &Strings, prefix: &str) -> f32 {
    let mut whole = String::from(prefix);
    whole.push_str(&st.typed);
    let shown = faraday_ui::grouped(&whole);
    let wrong = matches!(st.found, Some(Found::Differs(_)));
    ui.fill(x, y, w, t::FIELD, t::RADIUS_SMALL, SURFACE);
    ui.stroke(
        x,
        y,
        w,
        t::FIELD,
        t::RADIUS_SMALL,
        if wrong { ERR } else { BORDER },
    );
    let fit = ui.fit(t::LABEL, W::M, &shown, w - 2.0 * t::GAP);
    ui.text_mid(x + t::GAP, y, t::FIELD, t::LABEL, W::M, TEXT, &fit);
    t::FIELD + t::GAP
}

/// What the string typed was found to be, in `OK` or `ERR`. Returns the
/// height.
fn found_line(ui: &mut Ui, x: f32, y: f32, w: f32, st: &Strings) -> f32 {
    match &st.found {
        Some(Found::Holds(l)) => ui.wrap(x, y, w, t::LABEL, W::R, OK, l) + t::GAP,
        Some(Found::Differs(l)) => ui.wrap(x, y, w, t::LABEL, W::R, ERR, l) + t::GAP,
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

/// The item's note, in `WARN`. Returns the height.
fn note(ui: &mut Ui, x: f32, y: f32, w: f32, st: &Strings) -> f32 {
    st.note
        .as_deref()
        .map_or(0.0, |n| ui.wrap(x, y, w, t::LABEL, W::R, WARN, n) + t::GAP)
}

/// Seed `i`'s codex32 item, under its title. `foot` says whether the
/// item's own foot (Make them, Check my copy) is drawn by the checklist
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
        return (
            one_string(ui, (x, y, w), (form, st), j, &places.join(", ")),
            false,
        );
    }
    let mut cy = y;
    cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, &facts(form)) + t::GAP;
    if form.length == 127 {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, EN.c32_holds_the_seed) + t::GAP;
    }
    let paper = crate::plan::codex32::paper_strings(&b.answers, &mapped, i);
    for &j in &paper {
        let places: Vec<String> = places_of_string(&mapped, i, j)
            .into_iter()
            .map(place)
            .collect();
        let done = b.c32_checked.contains(&(i, j));
        let line = fill(EN.c32_string_at, &[&name(j), &places.join(", ")]);
        let press = form
            .fixed()
            .then(|| a(Press::Open(u8::try_from(j).unwrap_or(0))));
        string_row(ui, (x, cy, w), &line, done, press);
        cy += t::TOUCH;
    }
    if form.unknown > 0 {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            MUTED,
            &fill(EN.c32_more_unknown, &[&form.unknown.to_string()]),
        ) + t::GAP;
    }
    match st.rebuilds {
        Some(true) => cy += ui.wrap(x, cy, w, t::LABEL, W::R, OK, EN.c32_rebuilds) + t::GAP,
        Some(false) => cy += ui.wrap(x, cy, w, t::LABEL, W::R, ERR, EN.c32_rebuilds_not) + t::GAP,
        None => {}
    }
    cy += note(ui, x, cy, w, st);
    // The set's actions, once the set exists and is split (§9.7).
    if form.fixed() && form.k >= 2 {
        cy += set_actions(ui, x, cy, w, form);
    }
    (cy - y, true)
}

/// One string open (§6.6, §9.4): the string where it was made here, in
/// groups of four or as its Codex32QR; typed back on the codex32
/// keyboard; Skip and Continue. Returns the height.
fn one_string(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (form, st): (&C32, &Strings),
    j: usize,
    places: &str,
) -> f32 {
    let mut cy = y;
    let head = fill(EN.c32_share_at, &[&name(j), places]);
    cy += ui.wrap(x, cy, w, t::BODY, W::S, TEXT, &head) + t::GAP;
    if let Some(s) = st.made(j) {
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
                let side = t::qr_side(if ui.compact {
                    faraday_draw::geom::SizeClass::Small
                } else {
                    faraday_draw::geom::SizeClass::Wide
                })
                .min(w);
                faraday_ui::parts::qr_code(ui, q, x, cy, side);
                cy += side + t::GAP;
            }
            None => {
                let g = faraday_ui::grouped(s);
                let mut size = t::TITLE;
                while size > t::CAPTION && ui.wrap_lines(size, W::M, &g, w) > 3 {
                    size -= 1.0;
                }
                cy += ui.wrap(x, cy, w, size, W::M, TEXT, &g) + t::GAP;
            }
        }
    } else {
        let line = if form.fixed() {
            EN.c32_type_from_paper
        } else {
            EN.c32_not_made
        };
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, line) + t::GAP;
    }
    cy += typed_field(ui, x, cy, w, st, "ms1");
    cy += found_line(ui, x, cy, w, st);
    if !matches!(st.found, Some(Found::Holds(_))) {
        cy += keyboard(ui, x, cy, w);
    }
    if st.skip_ask {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, WARN, EN.codex32_skip_caution) + t::GAP;
    }
    cy += note(ui, x, cy, w, st);
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        &[
            (EN.c32_all_strings, Style::Ghost, a(Press::List)),
            (EN.codex32_skip, Style::Ghost, a(Press::Skip)),
        ],
        Some((EN.item_action_continue, a(Press::Next))),
    );
    cy - y
}

/// The set action under way (§9.7). Returns the height.
fn set_step(ui: &mut Ui, x: f32, y: f32, w: f32, form: &C32, st: &Strings) -> f32 {
    let mut cy = y;
    let id = form.id_text().unwrap_or_default();
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
            cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, &count) + t::GAP;
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
        if done { OK } else { DIM },
    );
    let state = if done {
        EN.c32_checked
    } else {
        EN.c32_not_checked
    };
    let sw = ui.measure(t::LABEL, W::R, state);
    let fit = ui.fit(t::LABEL, W::S, line, w - sw - 2.0 * t::GAP - t::GAP);
    ui.text_mid(x + 2.0 * t::GAP, y, t::TOUCH, t::LABEL, W::S, TEXT, &fit);
    ui.text_right(
        x + w,
        y,
        t::TOUCH,
        t::LABEL,
        W::R,
        if done { OK } else { MUTED },
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
    ui.wrap(x, y, w, t::BODY, W::S, TEXT, text) + t::GAP
}

/// Rotate the set's marks: each old string held, lost or exposed.
/// Returns the height.
fn marking(ui: &mut Ui, x: f32, y: f32, w: f32, marks: &[(usize, u8)]) -> f32 {
    let mut cy = y + ui.wrap(x, y, w, t::LABEL, W::R, MUTED, EN.c32_rotate_mark) + t::GAP;
    for &(j, m) in marks {
        let jj = u8::try_from(j).unwrap_or(0);
        ui.text_mid(x, cy, t::TOUCH, t::LABEL, W::S, TEXT, &name(j));
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

/// The copies by hand's codex32 choice (`docs/CODEX32.md` §9.1): the
/// threshold, None (the secret alone) or 2 to 9, 2 chosen first; the
/// strings, `k` to 31, 3 first; and **Make the strings**. Returns the
/// height used.
pub(crate) fn paper_choice(ui: &mut Ui, x: f32, y: f32, w: f32, kn: Option<(u8, u8)>) -> f32 {
    let (k, n) = kn.unwrap_or((2, 3));
    let mut cy = y;
    crate::screens::section_label(ui, x, cy, EN.c32_paper_k);
    cy += t::PAD + t::GAP_SMALL;
    let labels: Vec<String> = (2..=faraday_bip::codex32::MAX_THRESHOLD)
        .map(|v| v.to_string())
        .collect();
    let mut items: Vec<(&str, bool, Action)> =
        vec![(EN.c32_paper_none, k == 0, a(Press::PaperK(0)))];
    items.extend(
        labels
            .iter()
            .zip(2..)
            .map(|(l, v)| (l.as_str(), k == v, a(Press::PaperK(v)))),
    );
    cy += chips(ui, x, cy, w, &items);
    if k >= 2 {
        crate::screens::stepper(
            ui,
            x,
            cy,
            w,
            EN.codex32_strings,
            usize::from(n),
            a(Press::PaperN(-1)),
            a(Press::PaperN(1)),
            true,
        );
        cy += t::STEPPER_ROW;
    }
    cy += faraday_ui::parts::wrap_buttons(
        ui,
        x,
        cy,
        w,
        t::TEXT_FIELD,
        &[(EN.c32_paper_make, Style::Secondary, Action::BCodex32(k, n))],
    );
    cy - y
}

/// The Result's rows over the copies by hand's codex32 strings (§9.1,
/// §6.7): the split, the length, the identifier, the key's fingerprint,
/// where the shares' randomness came from, and for a key of words what
/// the strings hold. Read from the strings shown. Returns the height.
pub(crate) fn paper_facts(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    (k, strings): (u8, &[zeroize::Zeroizing<String>]),
) -> f32 {
    let Some(first) = strings.first() else {
        return 0.0;
    };
    let id = first.get(4..8).unwrap_or("");
    let split = if k == 0 {
        EN.c32_secret_alone.to_string()
    } else {
        fill(EN.c32_k_of_n, &[&k.to_string(), &strings.len().to_string()])
    };
    let fp = app
        .backup()
        .and_then(|b| app.session.keys.info(b.key))
        .map(|key| crate::wallet::fp_text(key.fingerprint))
        .unwrap_or_default();
    let mut rows = vec![
        fill(EN.c32_made_split, &[&split]),
        fill(EN.c32_made_length, &[&first.len().to_string()]),
        fill(EN.c32_made_identifier, &[id]),
        fill(EN.c32_made_fingerprint, &[&fp]),
    ];
    if k >= 2 {
        rows.push(EN.c32_made_random.to_string());
    }
    if first.len() == 127 {
        rows.push(EN.c32_made_holds.to_string());
        rows.push(EN.c32_made_not_by_hand.to_string());
    }
    let mut cy = y;
    for r in rows {
        cy += ui.wrap(x, cy, w, t::LABEL, W::R, MUTED, &r) + t::GAP_SMALL;
    }
    cy + t::GAP - y
}
