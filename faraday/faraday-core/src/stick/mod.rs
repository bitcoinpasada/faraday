//! The stick area: the sticks present and what is chosen about them.

pub(crate) mod pills;
pub mod settings;
pub mod wipe;

use crate::medium::Medium;
use crate::strings::{EN, fill};
use crate::vault::Focus;
use crate::{Faraday, StickInfo};

/// Said where what would load a key, or take a secret typed, waits for no
/// stick attached: on the camera, and in a vault's fields.
pub(crate) fn keys_load(m: Medium) -> String {
    fill(EN.stick_keys_keys_load, &[m.noun()])
}

/// The one check of the stick rule for a vault (`docs/SECURITY-MODEL.md`):
/// while a stick is attached a passphrase takes no typing and no key is
/// loaded, and no field of a vault entry or note (`entry`) takes typing or
/// is filled. A passphrase or a key also waits for a passed self-test
/// ([`Faraday::may_load_keys`]); an entry's fields wait for the stick
/// alone. Returns whether it refused; typing then goes nowhere and, with a
/// stick attached, the screen says why.
pub(crate) fn refuses(app: &mut Faraday, entry: bool) -> bool {
    let attached = !app.stick.is_empty();
    if !(attached || !entry && !app.may_load_keys()) {
        return false;
    }
    app.vaults.set_focus(None);
    if attached {
        let line = if entry {
            fill(EN.vaults_fields_with_stick, &[app.medium.noun()])
        } else {
            keys_load(app.medium)
        };
        app.toast(&line);
    }
    true
}

/// Whether typing at `focus` is a secret the stick rule refuses
/// ([`refuses`]); an entry's field (editing an entry, too) is one.
pub(crate) fn refuses_focus(app: &mut Faraday, focus: Focus) -> bool {
    app.vaults.focus_is_secret(focus) && refuses(app, matches!(focus, Focus::Field(_)))
}

/// Typing goes to `focus`, unless the stick rule refuses it.
pub(crate) fn focus(app: &mut Faraday, focus: Focus) {
    if !refuses_focus(app, focus) {
        app.vaults.set_focus(Some(focus));
    }
}

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
    /// The sticks present that the stick visit has been on screen for
    /// since they arrived, by id.
    visited: Vec<String>,
    /// A stick present has not been visited since it arrived: the stick
    /// visit's badge. Invariant: true exactly when a stick in `sticks`
    /// is not in `visited`, kept by [`StickArea::set_sticks`] and
    /// [`StickArea::note_visit`], the only ways either changes.
    waits: bool,
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
        let present = &self.sticks;
        self.visited
            .retain(|id| present.iter().any(|s| &s.id == id));
        self.note_waits();
    }

    /// Notes the stick visit on screen (`on`): every stick present is
    /// visited.
    pub(crate) fn note_visit(&mut self, on: bool) {
        if !on || !self.waits {
            return;
        }
        for s in &self.sticks {
            if !self.visited.contains(&s.id) {
                self.visited.push(s.id.clone());
            }
        }
        self.note_waits();
    }

    fn note_waits(&mut self) {
        self.waits = self.sticks.iter().any(|s| !self.visited.contains(&s.id));
    }

    /// Whether a stick present has not been visited since it arrived.
    pub(crate) fn waits(&self) -> bool {
        self.waits
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
