//! Check a timestamp's screen: the step-card column over
//! [`crate::ots_check`].

use super::{Field, OtsAction as O, OtsState, Verdict, is_proof, ostep, state};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::pal::Role;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{
    self, BODY_M, BODY_S, BUTTON_H, FIELD_H, FIELD_PAD, GAP, ICON, ICON_GLYPH_SMALL,
};
use faraday_draw::widgets::Icon;

pub(crate) const TITLES: [&str; ostep::COUNT] = [
    EN.ots_card_proof,
    EN.ots_card_file,
    EN.ots_card_header,
    EN.ots_card_result,
];

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    let back_name = if s.back == crate::Screen::Catalog {
        EN.ots_tools
    } else {
        app.back_name(s.back)
    };
    let back = Some(app.named_back((back_name, Action::Nav(s.back))));
    if s.landing {
        landing(app, ui, (x0, cw, h), back);
        return;
    }
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back,
        heading: EN.ots_title,
        chip: None,
        note: None,
        chip_tap: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&super::OTS_STEPS) else {
        return;
    };
    let scroll = s.scroll;
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            match state(app_ref) {
                Some(s) => card(app_ref, s, ui, step, (x, y, w)),
                None => 0.0,
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(Scrolled::OtsCheck, next, again);
}

/// No proof chosen: the proofs waiting to choose from, or the empty part.
fn landing(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), back: Option<(&str, Action)>) {
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if ui.compact {
        crate::compact::BAR_H + tokens::GAP
    } else {
        crate::screens::bar_head(ui, back, EN.ots_title)
    };
    let (x, w) = if ui.compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (
            x0 + tokens::FLOW_INSET,
            (cw - 2.0 * tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        )
    };
    let waiting = super::waiting(app);
    if waiting.is_empty() {
        crate::screens::parts::nothing_to_work_on(
            app,
            ui,
            (x, top, w),
            EN.ots_no_proof_waits,
            (Some(Action::Scan), crate::Screen::OtsCheck),
        );
    } else {
        let rows: Vec<(&str, Action)> = waiting
            .into_iter()
            .map(|(k, name)| (name, Action::OtsCheck(O::Proof(k))))
            .collect();
        crate::screens::parts::file_choices(ui, (x, top, w), EN.ots_choose_proof, &rows);
    }
    ui.region_close(open, 0.0);
}

pub(super) fn summary(s: &OtsState, step: u8) -> String {
    match step {
        ostep::PROOF => s.proof.as_ref().map(|g| String::from(&*g.name)),
        ostep::FILE => s.file.as_ref().map(|g| String::from(&*g.name)),
        ostep::HEADER => s.header.as_ref().map(|g| String::from(&*g.name)),
        _ => s.shown.as_ref().map(|r| {
            match r.verdict {
                Verdict::Committed => EN.ots_committed,
                Verdict::CommittedDigest => EN.ots_committed_digest,
                Verdict::NotCommitted => EN.ots_not_committed,
                Verdict::FileDiffers => EN.ots_file_differs,
                Verdict::NeedsFile => EN.ots_needs_file_short,
            }
            .to_string()
        }),
    }
    .unwrap_or_default()
}

fn card(app: &View, s: &OtsState, ui: &mut Ui, step: u8, at: (f32, f32, f32)) -> f32 {
    match step {
        ostep::PROOF => files(app, s, ui, at, 0),
        ostep::FILE => {
            let mut cy = files(app, s, ui, at, 1) + GAP;
            cy += field(s, ui, (at.0, at.1 + cy, at.2), Field::Hash);
            cy += GAP;
            ui.button(
                at.0,
                at.1 + cy,
                None,
                BUTTON_H,
                EN.ots_scan,
                Style::Secondary,
                Action::OtsCheck(O::ScanHash),
            );
            cy + BUTTON_H
        }
        ostep::HEADER => {
            let mut cy = files(app, s, ui, at, 2) + GAP;
            cy += field(s, ui, (at.0, at.1 + cy, at.2), Field::Header);
            cy += GAP;
            ui.button(
                at.0,
                at.1 + cy,
                None,
                BUTTON_H,
                EN.ots_scan,
                Style::Secondary,
                Action::OtsCheck(O::Scan),
            );
            cy + BUTTON_H
        }
        _ => result(s, ui, at),
    }
}

/// The files of Files that fit the card, one button each; a line when
/// there is none. Returns the height.
fn files(app: &View, s: &OtsState, ui: &mut Ui, (x, y, w): (f32, f32, f32), which: u8) -> f32 {
    let mut cy = 0.0;
    let mut any = false;
    for (i, item) in app.inbox.iter().enumerate() {
        let (fits, action, chosen) = match which {
            0 => (
                is_proof(&item.bytes),
                O::Proof(i),
                s.proof
                    .as_ref()
                    .is_some_and(|g| g.name == item.name.as_str()),
            ),
            1 => (
                !is_proof(&item.bytes),
                O::File(i),
                s.file
                    .as_ref()
                    .is_some_and(|g| !g.digest_only && g.name == item.name.as_str()),
            ),
            _ => (
                super::header_bytes(&item.bytes, true).is_some(),
                O::Header(i),
                s.header
                    .as_ref()
                    .is_some_and(|g| g.name == item.name.as_str()),
            ),
        };
        if !fits {
            continue;
        }
        any = true;
        ui.button(
            x,
            y + cy,
            Some(w),
            BUTTON_H,
            &item.name,
            if chosen {
                Style::Selected
            } else {
                Style::Secondary
            },
            Action::OtsCheck(action),
        );
        cy += BUTTON_H + GAP;
    }
    if !any {
        cy += ui.wrap_as(
            (x, y, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.ots_none_in_files,
        );
        cy += GAP;
    }
    if let Some(why) = &s.refused[usize::from(which)] {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, why) + GAP;
    }
    cy - GAP
}

/// A typed field with its label and a Use button. Returns the height.
fn field(s: &OtsState, ui: &mut Ui, (x, y, w): (f32, f32, f32), f: Field) -> f32 {
    let (label, text) = match f {
        Field::Hash => (EN.ots_hash_label, s.hash_text.as_str()),
        Field::Header => (EN.ots_header_label, s.header_text.as_str()),
    };
    let focus = Action::OtsCheck(O::Focus(f));
    let mut cy = ui.field_label(x, y, label);
    let focused = s.typing == Some(f);
    ui.field_box((x, y + cy, w), focused, None);
    ui.typed(
        x + FIELD_PAD,
        y + cy,
        FIELD_H,
        BODY_M.mono().size,
        W::from(BODY_M.mono().face),
        Role::OnSurface,
        text,
        w - 2.0 * FIELD_PAD,
        focused,
        focus,
    );
    ui.press_box(x, y + cy, w, FIELD_H, focus);
    cy += FIELD_H + GAP;
    let style = if text.is_empty() {
        Style::PrimaryOff
    } else {
        Style::Primary
    };
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.ots_use,
        style,
        Action::OtsCheck(O::Take(f)),
    );
    cy + BUTTON_H
}

fn result(s: &OtsState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(r) = &s.shown else {
        return ui.wrap_as(
            (x, y, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.ots_result_waits,
        );
    };
    let (line, ok) = match r.verdict {
        Verdict::Committed => (EN.ots_committed, true),
        Verdict::CommittedDigest => (EN.ots_committed_digest, true),
        Verdict::NotCommitted => (EN.ots_not_committed, false),
        Verdict::FileDiffers => (EN.ots_file_differs, false),
        Verdict::NeedsFile => (EN.ots_needs_file_short, false),
    };
    let tone = if ok { Role::Ok } else { Role::Error };
    let icon = if ok { Icon::Success } else { Icon::Error };
    ui.icon(x, y, ICON, icon, ICON_GLYPH_SMALL, tone);
    let indent = ICON + GAP;
    let mut cy = ui
        .wrap_as((x + indent, y, w - indent), BODY_M, tone, line)
        .max(ICON)
        + GAP;
    for (k, v) in &r.facts {
        cy += faraday_ui::sheet::kv(ui, x, y + cy, w, k, v, Role::OnSurface);
    }
    for l in &r.lines {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::OnSurface, l) + GAP;
    }
    cy + crate::screens::parts::ending(
        ui,
        (x, y + cy, w),
        None,
        (Action::OtsCheck(O::Another), Action::Nav(s.back)),
    )
}
