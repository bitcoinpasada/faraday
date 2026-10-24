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
    pub(crate) dirty: bool,
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
            toast: None,
            toast_at: None,
        }
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
