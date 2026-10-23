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

    /// Shows `text` as the toast from `now_ms`, for three seconds.
    pub(crate) fn toast(&mut self, text: &str, now_ms: u64) {
        self.toast = Some((text.to_string(), now_ms + 3000));
        self.toast_at = None;
    }

    /// Queues `command` for the shell.
    pub(crate) fn command(&mut self, command: Command) {
        self.commands.push_back(command);
    }
}
