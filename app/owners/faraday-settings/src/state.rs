use core::borrow::Borrow;

use faraday_kernel::AtLock;
use faraday_kernel::settings::Settings;

use crate::file;
use crate::kept::ModeAsked;

/// Where the boot stick's settings file stands: read once per process, from the boot stick
/// alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Look {
    /// No boot stick has been listed yet.
    Waiting,
    /// The file is being read.
    Asked,
    /// What the boot stick holds is known: read, failed, absent, or carried from the last build.
    Known,
}

/// The person's choices as the owner holds them.
///
/// Nothing here is secret: the choices are kept across a lock, as `main`'s `Settings` were.
#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    settings: Settings,
    #[lock(public)]
    look: Look,
    /// The body the boot stick holds, or what it would give had it held a file; `None` until
    /// known.
    #[lock(public)]
    on_stick: Option<String>,
    /// The mode asked for, once chosen: `main`'s `ModesArea::next_mode`.
    #[lock(public)]
    asked: Option<ModeAsked>,
    /// The mode this build of the scope runs in; a lock keeps it unless a change was begun.
    #[lock(public)]
    in_use: ModeAsked,
    /// The Lock sheet has been asked for the mode in `asked`: the only lock that applies it.
    #[lock(public)]
    pending: bool,
}

impl State {
    /// A start: the defaults until the settings file is read, or what the last build carried.
    pub(crate) fn new(
        settings: Settings,
        look: Look,
        on_stick: Option<String>,
        in_use: ModeAsked,
    ) -> State {
        State {
            settings,
            look,
            on_stick,
            asked: None,
            in_use,
            pending: false,
        }
    }

    pub(crate) fn settings(&self) -> &Settings {
        &self.settings
    }

    pub(crate) fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub(crate) fn look(&self) -> Look {
        self.look
    }

    pub(crate) fn set_look(&mut self, look: Look) {
        self.look = look;
    }

    /// What the boot stick holds is the settings now (read from it, or the settings it would
    /// have given had it held a file).
    pub(crate) fn on_stick_known(&mut self) {
        self.on_stick = Some(file::body(&self.settings));
        self.look = Look::Known;
    }

    pub(crate) fn on_stick(&self) -> Option<&str> {
        self.on_stick.as_deref()
    }

    pub(crate) fn ask_mode(&mut self, m: Option<ModeAsked>) {
        self.asked = m;
    }

    /// This build of the scope runs in `mode` (the kernel's `Mode::index`). An owner that
    /// outlives the scope (the Starter's) sees a change here: the asked mode took effect, so it
    /// and its Lock sheet are done with.
    pub(crate) fn refresh(&mut self, index: u8) {
        let now = ModeAsked::of_index(index);
        if now != self.in_use {
            self.in_use = now;
            self.asked = None;
            self.pending = false;
        }
    }

    pub(crate) fn set_pending(&mut self, on: bool) {
        self.pending = on;
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending
    }

    /// The mode the next build starts in: the one asked for once the Lock sheet was asked for
    /// it, else the one in use (`main`'s `next_mode.unwrap_or(in_use)`).
    pub(crate) fn mode_after_lock(&self) -> ModeAsked {
        match self.asked {
            Some(m) if self.pending => m,
            _ => self.in_use,
        }
    }

    /// The mode asked for, if one is.
    pub fn mode_asked(&self) -> Option<ModeAsked> {
        self.asked
    }

    /// Whether a setting differs from what the boot stick holds, or from the defaults when it has
    /// no settings file: the visit's Settings row is offered, and written, only then.
    pub fn changed(&self) -> bool {
        let now = file::body(&self.settings);
        match &self.on_stick {
            Some(on) => *on != now,
            None => now != file::body(&Settings::default()),
        }
    }

    /// The settings file's whole text, to write to a stick.
    pub fn file_text(&self) -> String {
        file::file_text(&self.settings)
    }
}

impl Borrow<Settings> for State {
    fn borrow(&self) -> &Settings {
        &self.settings
    }
}
