//! Tags (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::shown::Shown;
use crate::ui::Ui;

/// What a status tag says (`docs/DESIGN.md` §3 Status,
/// `docs/THEMES.md` §6): done, needs attention, refused, or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Done, verified, can sign: ok-container.
    Ok,
    /// Waiting, unsaved, needs attention: warning-container.
    Warning,
    /// Refused: error-container.
    Error,
    /// Neither: surface-container-highest.
    Neutral,
}

impl Status {
    /// Its container and the colour on it.
    pub const fn roles(self) -> (Role, Role) {
        match self {
            Status::Ok => (Role::OkContainer, Role::OnOkContainer),
            Status::Warning => (Role::WarningContainer, Role::OnWarningContainer),
            Status::Error => (Role::ErrorContainer, Role::OnErrorContainer),
            Status::Neutral => (Role::SurfaceContainerHighest, Role::OnSurfaceVariant),
        }
    }
}

/// A status tag: a rounded label with a dot.
pub struct TagModel<'s> {
    pub label: Shown<'s>,
    pub status: Status,
}

/// Draws the tag with its top-left at (x, y), design units. Returns its width.
pub fn draw<A: Pressed>(m: &TagModel<'_>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) -> f32 {
    ui.showing(&m.label, |ui| ui.tag(x, y, m.label.as_str(), m.status))
}

impl<A: Pressed> Ui<'_, A> {
    /// A status tag (`docs/DESIGN.md` §3 Status): a rounded label with
    /// a dot in `status`'s container colour, its dot and text in its
    /// on-container colour. Returns its width.
    pub fn tag(&mut self, x: f32, y: f32, label: &str, status: Status) -> f32 {
        let (bg, fg) = status.roles();
        self.chip(x, y, label, fg, bg)
    }
}
