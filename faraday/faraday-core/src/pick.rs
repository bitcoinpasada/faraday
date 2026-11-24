//! The pick's rows (`docs/WORKSPACE.md` §3, "The pick"): what a flow's
//! first step offers to choose, as data, and the one drawer of them. A
//! row is a Material single-choice list row: the radio at the start, a
//! tonal container behind it, the chosen one in the secondary container,
//! the state layer on hover and press. A way in is the same row with no
//! radio. A row the step cannot take is dimmed, keeps its reason as its
//! line and takes no press, by pointer or keyboard. Labels only: no row
//! holds or reads a secret.

use crate::Action;
use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens::{
    BODY_L, BODY_M, DISABLED_CONTENT, GAP, GAP_SMALL, ICON_GLYPH, ICON_GLYPH_BUTTON, LABEL_L,
    LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, SHAPE_S,
};
use faraday_draw::widgets::Icon;

/// What a row is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Something loaded the flow takes: a radio, ticked when `chosen`.
    Pick { chosen: bool },
    /// A way to get what the flow takes (Add a key): no radio.
    WayIn,
}

/// One row of the pick.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PickRow {
    pub icon: Icon,
    pub head: String,
    /// The second line; the reason when the row is dimmed.
    pub line: String,
    pub kind: Kind,
    /// Dimmed: the row stays and takes no press.
    pub dimmed: bool,
    pub action: Action,
}

impl PickRow {
    /// A loaded item the flow takes.
    pub(crate) fn pick(icon: Icon, head: String, line: String, chosen: bool, a: Action) -> Self {
        PickRow {
            icon,
            head,
            line,
            kind: Kind::Pick { chosen },
            dimmed: false,
            action: a,
        }
    }

    /// A way in, which has no second line.
    pub(crate) fn way_in(icon: Icon, head: &str, action: Action) -> Self {
        PickRow {
            icon,
            head: head.to_string(),
            line: String::new(),
            kind: Kind::WayIn,
            dimmed: false,
            action,
        }
    }

    /// The row as it is when the step cannot take it, with its `reason`.
    pub(crate) fn dimmed_for(mut self, reason: &str) -> Self {
        self.dimmed = true;
        self.line = reason.to_string();
        self
    }
}

/// The ways-in group's heading: "Other ways in", or "Ways in" when
/// nothing loaded is taken.
pub(crate) fn ways_in_head(loaded: bool) -> &'static str {
    if loaded {
        EN.pick_other_ways_in
    } else {
        EN.pick_ways_in
    }
}

/// A group's heading at `y`. Returns the height used.
pub(crate) fn head(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    let h = ui.advance_as(LABEL_L, ICON_GLYPH + GAP);
    let text = ui.fit_as(LABEL_L, text, w);
    ui.text_mid_as((x, y, h), LABEL_L, Role::OnSurfaceVariant, &text);
    h
}

/// The rows from `y`, one under another. Returns the height used.
pub(crate) fn rows(ui: &mut Ui, x: f32, y: f32, w: f32, rows: &[PickRow]) -> f32 {
    let mut cy = y;
    for r in rows {
        cy += row(ui, (x, cy, w), r) + GAP_SMALL;
    }
    cy - y
}

fn row(ui: &mut Ui, (x, y, w): (f32, f32, f32), r: &PickRow) -> f32 {
    let two = !r.line.is_empty();
    let lines = if two {
        ui.line_as(BODY_L) + ui.line_as(BODY_M)
    } else {
        ui.line_as(BODY_L)
    };
    let h = ui.row_height(if two { LIST_2 } else { LIST_1 }, lines, LIST_PAD_V);
    let on = matches!(r.kind, Kind::Pick { chosen: true });
    let ink = |role: Role| {
        if r.dimmed {
            Role::OnSurface.at(DISABLED_CONTENT)
        } else {
            on_selected(on, role)
        }
    };
    ui.fill(
        x,
        y,
        w,
        h,
        SHAPE_S,
        selected_or(on, Role::SurfaceContainerHighest),
    );
    let mut tx = x + LIST_PAD;
    if let Kind::Pick { chosen } = r.kind {
        let d = ICON_GLYPH_BUTTON / 2.0;
        let (cx, cy) = (tx + d, y + h / 2.0);
        ui.dot(cx, cy, d, ink(Role::OnSurfaceVariant));
        ui.dot(
            cx,
            cy,
            d - 2.0,
            selected_or(on, Role::SurfaceContainerHighest),
        );
        if chosen {
            ui.dot(cx, cy, d - 5.0, ink(Role::OnSurface));
        }
        tx += ICON_GLYPH_BUTTON + GAP;
    }
    ui.icon(
        tx,
        y + (h - ICON_GLYPH) / 2.0,
        ICON_GLYPH,
        r.icon,
        ICON_GLYPH,
        ink(Role::OnSurfaceVariant),
    );
    tx += ICON_GLYPH + GAP;
    let room = x + w - LIST_PAD - tx;
    let head = ui.fit_as(BODY_L, &r.head, room);
    if two {
        let (lh, vh) = (ui.line_as(BODY_L), ui.line_as(BODY_M));
        let top = y + (h - lh - vh) / 2.0;
        ui.text_mid_as((tx, top, lh), BODY_L, ink(Role::OnSurface), &head);
        let line = ui.fit_as(BODY_M, &r.line, room);
        ui.text_mid_as(
            (tx, top + lh, vh),
            BODY_M,
            ink(Role::OnSurfaceVariant),
            &line,
        );
    } else {
        ui.text_mid_as((tx, y, h), BODY_L, ink(Role::OnSurface), &head);
    }
    // A dimmed row takes no press: nothing is registered for it.
    if !r.dimmed {
        ui.press_box(x, y, w, h, r.action);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

    /// The presses a drawn pick registers, for pointer and keyboard alike.
    fn presses(rows_: &[PickRow]) -> Vec<Action> {
        let d = DisplayInfo {
            width: 480,
            height: 640,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let mut canvas = faraday_draw::Canvas::new(&d);
        let f = canvas.scale().factor();
        let mut hits: Vec<(faraday_draw::Rect, Action)> = Vec::new();
        {
            let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
            rows(&mut ui, 0.0, 0.0, 300.0, rows_);
        }
        hits.into_iter().map(|(_, a)| a).collect()
    }

    #[test]
    fn a_dimmed_row_takes_no_press() {
        let row = |fp: u8| {
            PickRow::pick(
                Icon::Keys,
                "key".into(),
                "line".into(),
                false,
                Action::PKey([fp; 4]),
            )
        };
        let got = presses(&[row(1), row(2).dimmed_for("No seed held"), row(3)]);
        assert_eq!(got, [Action::PKey([1; 4]), Action::PKey([3; 4])]);
    }
}
