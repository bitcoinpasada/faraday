//! Check a timestamp's screen: the step-card column over
//! [`crate::ots_check`].

use super::{Field, OtsAction as O, OtsState, Verdict, is_proof, ostep, state};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::strings::EN;
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
    let cards: Vec<flow::Card> = (0..ostep::COUNT as u8)
        .map(|step| flow::Card {
            title: TITLES[usize::from(step)].to_string(),
            summary: summary(s, step),
            mono: false,
            done: s.done(step),
            open: s.open == Some(step),
            default: false,
            toggle: Action::OtsCheck(O::Step(step)),
            guide: None,
            learn: (step == ostep::PROOF).then_some(crate::learn::Topic::Timestamps),
        })
        .collect();
    let back_name = if s.back == crate::Screen::Catalog {
        EN.ots_tools
    } else {
        app.back_name(s.back)
    };
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + tokens::FLOW_INSET,
        w: (cw - tokens::FLOW_INSET - tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        h,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.ots_title,
        guided: app.guided(),
        switch: true,
        wrap_closed: false,
        note: None,
        chip: None,
        chip_tap: None,
        moves: crate::ui::Moves::No,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match state(app_ref) {
                Some(s) => card(app_ref, s, ui, i as u8, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::OtsCheck, next, again);
}

fn summary(s: &OtsState, step: u8) -> String {
    match step {
        ostep::PROOF => s.proof.as_ref().map(|g| g.name.clone()),
        ostep::FILE => s.file.as_ref().map(|g| g.name.clone()),
        ostep::HEADER => s.header.as_ref().map(|g| g.name.clone()),
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
                s.proof.as_ref().is_some_and(|g| g.name == item.name),
            ),
            1 => (
                !is_proof(&item.bytes),
                O::File(i),
                s.file
                    .as_ref()
                    .is_some_and(|g| !g.digest_only && g.name == item.name),
            ),
            _ => (
                super::header_bytes(&item.bytes, true).is_some(),
                O::Header(i),
                s.header.as_ref().is_some_and(|g| g.name == item.name),
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
    cy - GAP
}
