//! The Starter's Appearance (`docs/THEMES.md` §14): the scheme its pages
//! are drawn in, chosen on the Appearance page from the ⋮ menu, and the
//! held desktop app and Faraday Demo drawn in the same; a theme chosen in
//! the Demo is the Starter's too (F126, `crate::demo`). The Starter keeps
//! no settings file (`docs/STARTER.md` §5.1): it opens in Cobalt at
//! Standard contrast, Light or Dark as this computer's own setting says
//! (the shell reads it at start), and a choice lasts until it closes.

use crate::Starter;

impl Starter {
    /// Draws the pages, and the held desktop app's Transfer and Learn, in
    /// `theme` from the next frame on: the Appearance page's choice, and
    /// what the snapshots and the colour tests set.
    pub fn set_theme(&mut self, theme: faraday_ui::Theme) {
        self.theme = theme;
        self.held.draw_in(theme);
        self.frost = None;
        self.dirty = true;
        self.commands.push_back(crate::Command::Draw);
    }

    /// The scheme the pages are drawn in.
    pub fn theme(&self) -> faraday_ui::Theme {
        self.theme
    }

    /// A theme, mode or contrast pressed on the Appearance page: drawn
    /// at once.
    pub(crate) fn choose_theme(&mut self, theme: faraday_ui::Theme) {
        if self.page == crate::Page::Appearance {
            self.set_theme(theme);
        }
    }
}
