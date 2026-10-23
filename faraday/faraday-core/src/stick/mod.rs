//! The stick area: the sticks present and what is chosen about them.

pub(crate) mod pills;
pub mod settings;
pub mod wipe;

use crate::StickInfo;

/// The stick area: the sticks present, the pills pressed, the questions
/// asked of a stick being wiped, the problems the shell found. Reads of
/// the sticks go through [`std::ops::Deref`]; every change is one of the
/// methods below.
#[derive(Default)]
pub(crate) struct StickArea {
    /// The sticks present.
    sticks: Vec<StickInfo>,
    /// "Not now" was chosen for the sticks present.
    not_now: bool,
    /// A stick arrived while the camera was on: held back, and taken as
    /// arriving once the camera is off.
    held: bool,
    /// The sticks the shell found unusable (not FAT32, read-only), by id.
    problems: Vec<(String, bool)>,
    /// Pills pressed since the shell last asked: the folder names to
    /// insert or pull ([`crate::Faraday::take_stick_toggles`]).
    toggles: Vec<String>,
    /// The settings the boot stick holds, as `settings_body` writes them:
    /// read from it at boot, or written to it since; `None` until the
    /// boot stick has been looked at.
    settings: Option<String>,
    /// What [`crate::Sheet::Delete`] is choosing.
    delete_ask: Option<wipe::DeleteAsk>,
    /// Zeroing a stick's free space or whole partition, under way.
    zeroing: Option<wipe::Zeroing>,
}

impl std::ops::Deref for StickArea {
    type Target = Vec<StickInfo>;
    fn deref(&self) -> &Vec<StickInfo> {
        &self.sticks
    }
}

impl StickArea {
    /// The sticks the shell reports.
    pub(crate) fn set_sticks(&mut self, sticks: Vec<StickInfo>) {
        self.sticks = sticks;
    }

    /// Every stick, to change them.
    pub(crate) fn iter_mut(&mut self) -> std::slice::IterMut<'_, StickInfo> {
        self.sticks.iter_mut()
    }

    /// Whether "Not now" was chosen for the sticks present.
    pub(crate) fn not_now(&self) -> bool {
        self.not_now
    }

    /// Notes whether "Not now" was chosen.
    pub(crate) fn set_not_now(&mut self, on: bool) {
        self.not_now = on;
    }

    /// Whether a stick arrived while the camera was on and waits.
    pub(crate) fn held(&self) -> bool {
        self.held
    }

    /// Notes whether a stick waits for the camera to go off.
    pub(crate) fn set_held(&mut self, on: bool) {
        self.held = on;
    }

    /// The sticks the shell found unusable, by id, with whether each is
    /// read-only.
    pub(crate) fn problems(&self) -> &[(String, bool)] {
        &self.problems
    }

    /// Forgets the sticks found unusable.
    pub(crate) fn clear_problems(&mut self) {
        self.problems.clear();
    }

    /// Notes that `stick` is unusable, read-only or not FAT32.
    pub(crate) fn note_problem(&mut self, stick: &str, read_only: bool) {
        self.problems.retain(|(id, _)| id != stick);
        self.problems.push((stick.to_string(), read_only));
    }

    /// The settings the boot stick holds, once it has been looked at.
    pub(crate) fn settings(&self) -> Option<&String> {
        self.settings.as_ref()
    }

    /// Notes the settings the boot stick holds.
    pub(crate) fn set_settings(&mut self, body: Option<String>) {
        self.settings = body;
    }

    /// What the Delete sheet is choosing, to change it.
    pub(crate) fn delete_ask_mut(&mut self) -> Option<&mut wipe::DeleteAsk> {
        self.delete_ask.as_mut()
    }

    /// Forgets what the Delete sheet was choosing.
    pub(crate) fn clear_delete_ask(&mut self) {
        self.delete_ask = None;
    }
}
