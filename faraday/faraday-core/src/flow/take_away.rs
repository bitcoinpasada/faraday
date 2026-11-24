//! The take-away: what a flow's last step gives to take away, drawn in
//! the pane at the left in place of Your choices where the pair stands
//! (`docs/WORKSPACE.md` §15, `docs/FLOW-CONTAINER.md` §7). Each file is a
//! tick row (the visit's chosen set, `VisitState.out`), each code a row
//! that shows it when asked, and at the foot one filled button writes the
//! ticked files. Public things only: names, sizes and actions.

use super::{Foot, TakeAway};
use crate::screens::visit::{ListRow, row};
use crate::strings::EN;
use crate::ui::{Region, Slot, Style, Ui, pal::*};
use crate::{Action, Faraday};
use faraday_draw::tokens::{self, BUTTON_H, CHOICES_INSET, LABEL_L};
use faraday_text::Text;

/// The pane for a take-away: where it stands, as [`super::choices::Pane`].
pub(crate) struct Pane<'a> {
    /// The app, for the outbox, the sticks and the receipt.
    pub(crate) app: &'a Faraday,
    /// What the flow gives.
    pub(crate) take: &'a TakeAway,
    /// The pane's left edge, its width and the window's height.
    pub(crate) place: (f32, f32, f32),
    /// Where its heading starts.
    pub(crate) top: f32,
    /// How far it is scrolled within itself.
    pub(crate) offset: f32,
}

/// What the button says and does.
enum Write {
    /// Nothing is ticked.
    Off,
    /// No stick is in.
    Waiting,
    /// A stick is in and a file is ticked.
    Ready,
    /// Every file went and was read back.
    Done,
}

impl Pane<'_> {
    /// Draws the heading, the rows and the button. Returns the height the
    /// content needs from the pane's top, so the pair can end level.
    pub(crate) fn draw(&self, ui: &mut Ui, foot: Foot) -> f32 {
        let (px, pw, h) = self.place;
        let top = self.top;
        let clip = ui.rect(px, top, pw, h - top);
        let open = ui.region_open(Region::scrolled(Slot::Choices, clip, self.offset));
        let mark = ui.presses_mark();
        let (x, w) = (px + CHOICES_INSET, pw - 2.0 * CHOICES_INSET);
        let mut y = top;
        ui.text_as(x, y, LABEL_L, Role::OnSurfaceVariant, EN.take_away_head);
        y += ui.line_as(LABEL_L) + tokens::GRID;
        let (rows_h, state) = self.rows(ui, (px, pw, y));
        y += rows_h;
        for code in &self.take.codes {
            y += row(
                ui,
                (px, pw, y),
                ListRow {
                    name: code.name,
                    line: EN.take_away_code,
                    tag: ("", Role::OnSurfaceVariant.paint()),
                    ticked: None,
                    greyed: false,
                    same: false,
                    action: None,
                    menu: None,
                    main: Some((Text::of(EN.show_as_qr), Style::Secondary, code.show)),
                    wrap: false,
                },
            );
        }
        y += tokens::GRID;
        self.button(ui, (x, y, w), state);
        y += BUTTON_H;
        y += foot(ui, x, y, w);
        ui.clip_presses_since(mark, clip);
        ui.region_close(open, (y + CHOICES_INSET - h).max(0.0));
        y + CHOICES_INSET - top
    }

    /// The files' rows, and the state the button is in. A file waiting in
    /// the outbox is a tick row; one that went is green, from the visit's
    /// receipt.
    fn rows(&self, ui: &mut Ui, (px, pw, top): (f32, f32, f32)) -> (f32, Write) {
        let app = self.app;
        let stick = !app.stick.is_empty();
        let mut y = top;
        let (mut waiting, mut ticked, mut done) = (0usize, 0usize, 0usize);
        for file in &self.take.files {
            let name = &file.name;
            if let Some((k, item)) = app.outbox.iter().enumerate().find(|(_, i)| i.name == *name) {
                let on = app.visit.out.contains(&**name);
                waiting += 1;
                ticked += usize::from(on);
                let line = crate::screens::parts::kind_line(item.kind, item.bytes.len());
                y += row(
                    ui,
                    (px, pw, y),
                    ListRow {
                        name: &item.name,
                        line: &line,
                        tag: ("", Role::OnSurfaceVariant.paint()),
                        ticked: Some(on),
                        greyed: false,
                        same: false,
                        action: Some(Action::VisitOut(k)),
                        menu: None,
                        main: None,
                        wrap: false,
                    },
                );
                continue;
            }
            let receipt = app.visit.receipt();
            // Written and read back: the same name and the same bytes.
            if receipt.as_ref().is_some_and(|r| r.holds(name, &file.sha)) {
                done += 1;
                y += row(
                    ui,
                    (px, pw, y),
                    ListRow {
                        name,
                        line: EN.pull_line_written_checked,
                        tag: ("", Role::Ok.paint()),
                        ticked: None,
                        greyed: false,
                        same: true,
                        action: None,
                        menu: None,
                        main: None,
                        wrap: false,
                    },
                );
                continue;
            }
            // Not in the outbox under that name (removed, renamed in Files)
            // and not written as it was: the row makes it again, under the
            // name a new file would have.
            let label = if receipt.as_ref().is_some_and(|r| r.wrote(name)) {
                EN.made_state_make_again
            } else {
                file.make_label
            };
            y += row(
                ui,
                (px, pw, y),
                ListRow {
                    name: &file.again,
                    line: EN.message_signed_message,
                    tag: ("", Role::OnSurfaceVariant.paint()),
                    ticked: None,
                    greyed: false,
                    same: false,
                    action: None,
                    menu: None,
                    main: Some((Text::of(label), Style::Secondary, file.make)),
                    wrap: false,
                },
            );
        }
        let state = if waiting == 0 && done > 0 {
            Write::Done
        } else if waiting == 0 {
            Write::Off
        } else if !stick {
            Write::Waiting
        } else if ticked == 0 {
            Write::Off
        } else {
            Write::Ready
        };
        (y - top, state)
    }

    /// The one filled button at the pane's foot, in its three states.
    fn button(&self, ui: &mut Ui, (x, y, w): (f32, f32, f32), state: Write) {
        let medium = self.app.medium;
        let (label, on) = match state {
            Write::Off => (Text::fill(EN.take_away_write, &[medium.noun()]), false),
            Write::Waiting => (Text::fill(EN.files_waiting_medium, &[medium.a()]), false),
            Write::Ready => (Text::fill(EN.take_away_write, &[medium.noun()]), true),
            Write::Done => (Text::of(EN.pull_line_written_checked), false),
        };
        ui.button(
            x,
            y,
            Some(w),
            BUTTON_H,
            &label,
            Style::Primary.when(on),
            Action::VisitWrite,
        );
    }
}
