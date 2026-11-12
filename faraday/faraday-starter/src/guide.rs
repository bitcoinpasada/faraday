//! The Starter's pages of the guide (`docs/GUIDE-IN-APP.md` §6): which
//! guide page and section each page opens, and the sheet that shows it.

use crate::{Page, Sheet, StartGuide, Starter};

/// The Starter's guide pages, in the order of the guide's index: the
/// Guide destination lists them so.
pub const PLACES: [&str; 8] = [
    "starter-home",
    "starter-make",
    "starter-tools",
    "starter-stick",
    "starter-start",
    "starter-install",
    "starter-templates",
    "starter-appearance",
];

/// The key of the section `page` opens in the guide
/// (`docs/GUIDE-IN-APP.md` §1.1): the section of its step where it has
/// one, else its place's lead. About is the only page with no place of
/// its own: it is read from Home's. A lookup, done when the guide is
/// opened, never while drawing.
pub fn guide_key(page: Page) -> &'static str {
    match page {
        Page::Home | Page::About => "starter-home",
        Page::Make => "starter-make",
        Page::Tools => "starter-tools",
        Page::Guide => "starter-home",
        Page::Which => "starter-stick.which",
        Page::Disks => "starter-stick.disks",
        Page::Mode => "starter-stick.mode",
        Page::Confirm => "starter-stick.confirm",
        Page::Writing => "starter-stick.writing",
        Page::Done => "starter-stick.done",
        Page::Start => "starter-start.steps",
        Page::Install => "starter-install.install",
        Page::Installed => "starter-install.installed",
        Page::Appearance => "starter-appearance.appearance",
        Page::Templates => "starter-templates.templates",
        Page::TemplateForm => "starter-templates.template-form",
        Page::TemplateWords => "starter-templates.template-words",
        Page::TemplateCopy => "starter-templates.template-copy",
        Page::TemplateLength => "starter-templates.template-length",
        Page::TemplateWorksheets => "starter-templates.template-worksheets",
        Page::TemplateSize => "starter-templates.template-size",
        Page::TemplatePlaces => "starter-templates.template-places",
    }
}

/// The section Keep data opens (`docs/GUIDE-IN-APP.md` §4.4).
pub const KEEP_DATA: &str = "starter-stick.keep-data";

impl Starter {
    /// Whether the page on show carries the Guide button in its top app
    /// bar: the pages of a task, the held app and Faraday Demo not. A
    /// lookup.
    pub fn guide_offered(&self) -> bool {
        !self.showing
            && !self.showing_demo()
            && matches!(
                self.page,
                Page::Which
                    | Page::Disks
                    | Page::Mode
                    | Page::Confirm
                    | Page::Writing
                    | Page::Done
                    | Page::Start
            )
    }

    /// Notes where the Guide sheet's drawing found its section.
    pub(crate) fn set_guide_top(&self, top: f32) {
        self.guide_top.set(top);
    }

    /// The section the open Guide sheet is at. A lookup.
    pub(crate) fn guide_at(&self) -> &'static str {
        self.guide_at
    }

    /// The start guide the open sheet shows. A lookup.
    pub(crate) fn start_shown(&self) -> Option<StartGuide> {
        self.start_shown
    }

    /// Opens the Guide sheet at `key`; it scrolls to that section when
    /// first drawn (`Starter::render`).
    pub(crate) fn guide_open(&mut self, key: &'static str) {
        if self.sheet.is_none() {
            self.guide_at = key;
            self.guide_top.set(0.0);
            self.scroller.guide_pending = true;
            self.sheet = Some(Sheet::Guide);
        }
    }

    /// The Guide button pressed: the sheet opens at the page's section;
    /// pressed again it closes.
    pub(crate) fn guide_press(&mut self) {
        if self.sheet == Some(Sheet::Guide) {
            self.sheet = None;
        } else if self.guide_offered() {
            self.guide_open(guide_key(self.page));
        }
    }
}
