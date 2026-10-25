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
    /// The sticks present that the stick visit has been on screen for
    /// since they arrived, by id.
    visited: Vec<String>,
    /// A stick present has not been visited since it arrived: the stick
    /// visit's badge. Invariant: true exactly when a stick in `sticks`
    /// is not in `visited`, kept by [`StickArea::set_sticks`] and
    /// [`StickArea::note_visit`], the only ways either changes.
    waits: bool,
    /// The Sticks menu of Faraday Desktop's top bar
    /// ([`crate::Sheet::SticksMenu`]): where its pill was, in pixels, when
    /// it opened, so the menu stands under it.
    menu_anchor: Option<(i32, i32, i32, i32)>,
    /// The menu opening (`true`) or closing, and the tick its motion
    /// started on (`None` until the next tick); `None` at rest.
    menu_motion: Option<(bool, Option<u64>)>,
    /// Where the page's top bar drew its stick pills in the last frame
    /// that drew the page: their right end and top in design units, and
    /// the room they had. Read to draw them again over a sheet, where
    /// they still take a press, as the shell's own; set by the frame
    /// each time it draws the page (`stick::pills::bar`).
    bar_spot: std::cell::Cell<Option<(f32, f32, f32)>>,
}

impl std::ops::Deref for StickArea {
    type Target = Vec<StickInfo>;
    fn deref(&self) -> &Vec<StickInfo> {
        &self.sticks
    }
}

impl StickArea {
    /// Where the Sticks menu's pill was when it opened, in pixels.
    pub(crate) fn menu_anchor(&self) -> Option<(i32, i32, i32, i32)> {
        self.menu_anchor
    }

    /// The Sticks menu's motion: opening or closing, and since when.
    pub(crate) fn menu_motion(&self) -> Option<(bool, Option<u64>)> {
        self.menu_motion
    }

    /// The Sticks menu opens under its pill at `anchor`, moving in as
    /// `moving` says.
    pub(crate) fn set_menu(&mut self, anchor: Option<(i32, i32, i32, i32)>, moving: bool) {
        self.menu_anchor = anchor;
        self.menu_motion = moving.then_some((true, None));
    }

    /// The Sticks menu's motion ends, or a closing one starts as `moving`
    /// says.
    pub(crate) fn close_menu(&mut self, moving: bool) {
        self.menu_motion = moving.then_some((false, None));
    }

    /// The menu's motion at tick `now_ms`, which lasts `ms`: started on the
    /// first tick, ended once over. Returns whether it still moves.
    pub(crate) fn menu_tick(&mut self, now_ms: u64, ms: impl Fn(bool) -> u64) -> bool {
        match self.menu_motion {
            Some((open, None)) => self.menu_motion = Some((open, Some(now_ms))),
            Some((open, Some(at))) if now_ms >= at + ms(open) + crate::motion::FIRST_STEP_MS => {
                self.menu_motion = None;
            }
            _ => {}
        }
        self.menu_motion.is_some()
    }

    /// Where the top bar's stick pills were last drawn.
    pub(crate) fn bar_spot(&self) -> Option<(f32, f32, f32)> {
        self.bar_spot.get()
    }

    /// Notes where the top bar's stick pills are drawn, or none.
    pub(crate) fn note_bar(&self, spot: Option<(f32, f32, f32)>) {
        self.bar_spot.set(spot);
    }

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
