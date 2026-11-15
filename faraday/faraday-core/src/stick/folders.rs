//! The folders the shell keeps for each stick: the boot stick's and a backup
//! stick's.

use faraday_text::{Text, text};

/// The boot stick's folder.
pub const BOOT_FOLDER: &str = "BOOTSTICK";

/// The number of the first backup stick: the boot stick is Stick 1.
pub const FIRST_BACKUP: usize = 2;

/// The folder of backup Stick `n` (2 and on: Stick 1 is the boot stick,
/// whose folder is [`BOOT_FOLDER`]).
pub fn backup_folder(n: usize) -> Text {
    text!("STICK-{n}")
}

/// The number a backup stick's folder is for, when it is one.
pub(super) fn folder_number(folder: &str) -> Option<usize> {
    folder
        .strip_prefix("STICK-")?
        .parse()
        .ok()
        .filter(|&n| n >= FIRST_BACKUP)
}
