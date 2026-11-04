//! What the core hands the shell: the commands, the storage commands,
//! whether the screen needs drawing, and the toast on show.

use std::collections::VecDeque;

use faraday_shell_api::Command;

use crate::storage_api::StorageCommand;

/// What the core hands the shell (`docs/STRUCTURE-PASS.md` §7.4.4, step 1):
/// the commands, the storage commands, whether the screen needs drawing,
/// and the toast on show. Areas are given `&mut Out` in place of these.
pub(crate) struct Out {
    pub(crate) commands: VecDeque<Command>,
    pub(crate) storage_out: VecDeque<StorageCommand>,
    dirty: bool,
    /// Only the active region's offset or stretch changed
    /// (`docs/FRAMES.md` §3.4).
    moved: bool,
    /// What the moved frame keeps between frames (`moved.rs`).
    pub(crate) kept: crate::moved::State,
    /// The toast on show, and the tick it ends at.
    pub(crate) toast: Option<(String, u64)>,
    /// The tick the toast on show first came up at.
    pub(crate) toast_at: Option<u64>,
}

impl Out {
    pub(crate) fn new() -> Self {
        Self {
            commands: VecDeque::new(),
            storage_out: VecDeque::new(),
            dirty: true,
            moved: false,
            kept: crate::moved::State::default(),
            toast: None,
            toast_at: None,
        }
    }

    /// The next frame is drawn whole.
    pub(crate) fn redraw(&mut self) {
        self.dirty = true;
    }

    /// Only the active region's offset or stretch changed: the next
    /// frame may move the rows already drawn, unless something asked for
    /// it whole ([`Out::redraw`]). Called by the motion code alone.
    pub(crate) fn moved(&mut self) {
        self.moved = true;
    }

    /// The next frame is drawn whole when `yes`.
    pub(crate) fn redraw_if(&mut self, yes: bool) {
        self.dirty |= yes;
    }

    /// A frame is wanted, whole or moved.
    pub(crate) fn wanted(&self) -> bool {
        self.dirty || self.moved
    }

    /// A frame is wanted and only the region's motion asked for it.
    pub(crate) fn only_moved(&self) -> bool {
        self.moved && !self.dirty
    }

    /// Takes the want for a frame, and whether it was only the region's
    /// motion.
    pub(crate) fn take_wanted(&mut self) -> Option<bool> {
        let want = self.wanted().then(|| self.only_moved());
        self.dirty = false;
        self.moved = false;
        want
    }

    /// Shows `text` as the toast from `now_ms`, for
    /// [`TOAST_SHOW_MS`](faraday_draw::tokens::TOAST_SHOW_MS).
    pub(crate) fn toast(&mut self, text: &str, now_ms: u64) {
        let until = now_ms + faraday_draw::tokens::TOAST_SHOW_MS;
        self.toast = Some((text.to_string(), until));
        self.toast_at = None;
    }

    /// How much of the toast shows at `now_ms`, 0 to 1, and how far below
    /// its place it is, in units: it rises in on Material's decelerating
    /// curve and fades on its accelerating one as its time runs out;
    /// with reduce motion (`still`) it does not rise.
    pub(crate) fn toast_shown(&self, now_ms: u64, still: bool) -> (f32, f32) {
        use faraday_draw::tokens::{EASE_STANDARD_ACCELERATE, EASE_STANDARD_DECELERATE};
        let Some((_, until)) = &self.toast else {
            return (0.0, 0.0);
        };
        let came = crate::motion::progress(self.toast_at, now_ms, crate::motion::TOAST_MS);
        let rise = crate::motion::ease(EASE_STANDARD_DECELERATE, came);
        let left = until.saturating_sub(now_ms) as f32 / crate::motion::TOAST_MS as f32;
        let fade = 1.0 - crate::motion::ease(EASE_STANDARD_ACCELERATE, 1.0 - left.min(1.0));
        let lift = if still { 0.0 } else { 10.0 * (1.0 - rise) };
        (rise.min(fade), lift)
    }

    /// Queues `command` for the shell.
    pub(crate) fn command(&mut self, command: Command) {
        self.commands.push_back(command);
    }
}
