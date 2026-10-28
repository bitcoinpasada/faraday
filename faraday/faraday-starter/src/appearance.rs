//! The Starter's Appearance (`docs/THEMES.md` §14): the scheme its pages
//! are drawn in, chosen on the Appearance page from the ⋮ menu, and the
//! held desktop app and Faraday Demo drawn in the same; a theme chosen in
//! the Demo is the Starter's too (F126, `crate::demo`). The theme is not
//! kept (`docs/STARTER.md` §5.1): it opens in Cobalt at Standard
//! contrast, Light or Dark as this computer's own setting says (the shell
//! reads it at start), and a choice lasts until it closes.
//!
//! Text size (F85, `docs/TEXT-SIZE.md` §6) is chosen on About: one
//! choice for the pages, the held app's Transfer and Learn and the Demo.
//! An installed Starter keeps it in its settings file, which the shell
//! reads at start ([`StarterEvent::TextKept`]) and writes when asked
//! ([`StarterCommand::KeepTextSize`]); a Starter with nothing installed
//! keeps it for the run.
//!
//! [`StarterEvent::TextKept`]: crate::api::StarterEvent::TextKept
//! [`StarterCommand::KeepTextSize`]: crate::api::StarterCommand::KeepTextSize

use crate::api::{Installed, StarterCommand};
use crate::{Class, Starter};
use faraday_ui::TextSize;

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

    /// How large text is drawn (`docs/TEXT-SIZE.md` §6). A lookup.
    pub fn text_size(&self) -> TextSize {
        self.text
    }

    /// Draws text at `text` from the next frame on, the pages laid out
    /// again for it (a larger text steps the layout down a class, as a
    /// narrower window would), the held app and the Demo at the same
    /// size; while a Starter is installed, its settings file is asked to
    /// keep it.
    pub fn set_text_size(&mut self, text: TextSize) {
        self.text = text;
        self.class = Class::of(self.w, self.h, text.factor());
        self.held.draw_text_at(text);
        self.demo_text_at(text);
        self.fit_held();
        self.keep_text();
        self.frost = None;
        self.dirty = true;
        self.commands.push_back(crate::Command::Draw);
    }

    /// A text size pressed on About: drawn at once.
    pub(crate) fn choose_text_size(&mut self, text: TextSize) {
        if self.page == crate::Page::About {
            self.set_text_size(text);
        }
    }

    /// The installed Starter's settings file holds `text`: the Starter
    /// opens at it.
    pub(crate) fn text_kept_read(&mut self, text: TextSize) {
        self.text_kept = text;
        self.set_text_size(text);
    }

    /// What is installed, as the shell found it: once a Starter is
    /// installed, the size chosen is kept; once none is, its file is
    /// gone with it.
    pub(crate) fn installed_now(&mut self, installed: Option<Installed>) {
        self.installed = installed;
        if self.installed.is_none() {
            self.text_kept = TextSize::Standard;
        }
        self.keep_text();
    }

    /// Asks the shell to keep the size chosen, while a Starter is
    /// installed and its file holds another.
    fn keep_text(&mut self) {
        if self.installed.is_some() && self.text != self.text_kept {
            self.text_kept = self.text;
            self.out.push_back(StarterCommand::KeepTextSize(self.text));
        }
    }
}
