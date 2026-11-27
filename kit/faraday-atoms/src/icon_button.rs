//! Icon buttons (09 §3.2).

use faraday_draw::Rect;
use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::hits::Shape;
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;
use faraday_draw::icons::Icon;
use faraday_tokens::dims::*;

/// Where a control's tooltip anchors this frame: the box it is drawn in, the press it is for
/// and its text (`Ui::take_tips`).
#[derive(Debug, Clone, PartialEq)]
pub struct TipAnchor<A> {
    /// The control's box, in pixels.
    pub rect: Rect,
    /// The press the control takes.
    pub press: A,
    /// The tooltip's text, which the caller gave.
    pub text: String,
    /// Which of Material's two tooltips it takes.
    pub kind: TipKind,
}

/// Which of Material's two tooltips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipKind {
    /// A few words naming a control, on the inverse surface.
    Plain,
    /// A sentence or more, on a raised surface.
    Rich,
}

/// An icon button: Material's outlined one, or its filled tonal one (04 §12.1).
pub struct IconButtonModel<'s, A> {
    pub icon: Icon,
    /// Its plain tooltip.
    pub tip: Shown<'s>,
    pub offered: Offered<A>,
    /// The filled tonal one rather than the outlined one.
    pub tonal: bool,
}

/// Draws the icon button with its top-left at (x, y), design units.
pub fn draw<A: Pressed>(m: &IconButtonModel<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32)) {
    ui.offered(&m.offered, |ui, press, off| match press {
        Some(action) if m.tonal && !off => ui.tonal_icon_button(at, m.icon, m.tip.as_str(), action),
        Some(action) => ui.outlined_icon_button(at, m.icon, m.tip.as_str(), !off, action),
        None => ui.outlined_icon_button_off(at, m.icon),
    });
}

impl<A: Pressed> Ui<'_, A> {
    /// Notes a tooltip anchor over the box (x, y, w, h), units.
    pub(crate) fn tip_anchor(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        press: &A,
        (tip, kind): (&str, TipKind),
    ) {
        let rect = self.rect(x, y, w, h).intersect(&self.seen_clip());
        if !rect.is_empty() && !tip.is_empty() {
            self.tips.push(TipAnchor {
                rect,
                press: press.clone(),
                text: tip.to_string(),
                kind,
            });
        }
    }

    /// Marks the box (x, y, w, h), units, as an icon-only control `owner` that a plain tooltip
    /// names: `label` is the action, a word or two, never an explanation.
    pub fn tip_plain(&mut self, r: (f32, f32, f32, f32), owner: &A, label: &str) {
        self.tip_anchor(r, owner, (label, TipKind::Plain));
    }

    /// Marks the box (x, y, w, h), units, as a control `owner` that a rich tooltip is about:
    /// `text` is a line or more, shown to the pointer, the keyboard and a long press.
    pub fn tip_rich(&mut self, r: (f32, f32, f32, f32), owner: &A, text: &str) {
        self.tip_anchor(r, owner, (text, TipKind::Rich));
    }

    /// The tooltip anchors this frame drew, in the order drawn.
    pub fn take_tips(&mut self) -> Vec<TipAnchor<A>> {
        std::mem::take(&mut self.tips)
    }

    /// Material's outlined icon button: an [`ICON_BUTTON`] circle at
    /// (x, y) edged in outline, `icon` at [`ICON_GLYPH`] in
    /// on-surface-variant, its state layer and focus ring in its shape,
    /// a press taken on at least the touch floor ([`Ui::touch`]) square, and `tip`, its plain
    /// tooltip. Not available (`on` false), edge and icon are on-surface
    /// at the disabled opacities and it takes no press.
    pub fn outlined_icon_button(
        &mut self,
        (x, y): (f32, f32),
        icon: Icon,
        tip: &str,
        on: bool,
        action: A,
    ) {
        let s = ICON_BUTTON;
        let r = shape_full(s);
        if on && !self.answering {
            self.stroke(x, y, s, s, r, Role::Outline);
            let ink = Role::OnSurfaceVariant.paint();
            self.press(Shape::new(x, y, s, s, r), ink, action.clone());
            self.icon(x, y, s, icon, ICON_GLYPH, Role::OnSurfaceVariant);
            self.tip_anchor((x, y, s, s), &action, (tip, TipKind::Plain));
        } else {
            // Faint in every theme: it is not available now.
            let edge = self.col(Role::OnSurface.at(DISABLED_CONTAINER));
            let (rect, line) = (self.rect(x, y, s, s), self.f.max(1.0));
            self.c.stroke_rounded_rect(rect, r * self.f, line, edge);
            let ink = Role::OnSurface.at(DISABLED_CONTENT);
            self.icon(x, y, s, icon, ICON_GLYPH, ink);
        }
    }

    /// An outlined icon button not available now and offering no press: faint, taking none.
    pub(crate) fn outlined_icon_button_off(&mut self, (x, y): (f32, f32), icon: Icon) {
        let s = ICON_BUTTON;
        let r = shape_full(s);
        let edge = self.col(Role::OnSurface.at(DISABLED_CONTAINER));
        let (rect, line) = (self.rect(x, y, s, s), self.f.max(1.0));
        self.c.stroke_rounded_rect(rect, r * self.f, line, edge);
        let ink = Role::OnSurface.at(DISABLED_CONTENT);
        self.icon(x, y, s, icon, ICON_GLYPH, ink);
    }

    /// Material's filled tonal icon button: an [`ICON_BUTTON`] circle at
    /// (x, y) in the secondary container, `icon` at [`ICON_GLYPH`] on it,
    /// its state layer and focus ring in its shape, a press taken on at
    /// least the touch floor square, and `tip`, its plain tooltip.
    pub fn tonal_icon_button(&mut self, (x, y): (f32, f32), icon: Icon, tip: &str, action: A) {
        let s = ICON_BUTTON;
        let r = shape_full(s);
        self.fill(x, y, s, s, r, Role::SecondaryContainer);
        let ink = Role::OnSecondaryContainer.paint();
        self.press(Shape::new(x, y, s, s, r), ink, action.clone());
        self.icon(x, y, s, icon, ICON_GLYPH, Role::OnSecondaryContainer);
        self.tip_anchor((x, y, s, s), &action, (tip, TipKind::Plain));
    }
}
