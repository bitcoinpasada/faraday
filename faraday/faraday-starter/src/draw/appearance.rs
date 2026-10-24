//! The Appearance page (`docs/THEMES.md` §14): Settings' Theme, Mode and
//! Contrast rows (§12), the same components; a page, as About is, so it
//! scrolls on a small window.

use super::{Starter, StarterAction, Ui};
use faraday_draw::tokens::APPEARANCE_LABEL_W;

/// The rows from (x, y0), `w` wide: each label beside its control, or
/// over it on a compact window. Returns where they end.
pub(super) fn page(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let label = (!starter.compact()).then_some(APPEARANCE_LABEL_W);
    let pick = |t| StarterAction::Theme(t).host();
    y0 + ui.appearance((x, y0, w), label, starter.theme(), pick)
}
