//! Faraday's English words for the backups area.

macro_rules! words_backups {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- backups (faraday-core/src/backups/mod.rs) -----
    /// Text in `backups`.
    backups_wallet: "Wallet",
    /// Text in `backups/screen.rs`: a loaded wallet's card.
    backups_open_wallet: "Open the wallet",
    /// Text in `backups`.
    backups_plan: "Plan",
    /// Text in `backups`. `{}`: `s.name`.
    backups_in_locked: "In {} · locked",
    /// Text in `record_lines`. `{}`: `p + 1`.
    backups_place: "Place {}",
    /// Text in `record_lines`.
    backups_vault: "Vault",
    /// Text in `record_lines`. `{}`: `v + 1`.
    backups_vault_n: "Vault {}",
    /// Text in `record_lines`. `{}`: `self.medium.cap()`.
    backups_of_files: "{} of files",
    /// Text in `record_lines`.
    backups_watch_only: "Watch-only software",
    /// Text in `record_lines`.
    backups_own_devices: "Own devices",
    /// Text in `record_lines`.
    backups_checked: "Checked",
    /// Text in `record_lines`.
    backups_not_checked: "Not checked",
    /// Text in `record_lines`. `{}`: how long ago, or the date.
    backups_checked_n: "Checked {}",
    /// Text in `record_lines`. `{}`: how long ago; due.
    backups_checked_due: "Checked {} · To check",
    /// Text in `record_lines`.
    backups_not_here: "Not here",
    /// Text in `record_lines`. `{}`: `self.medium.for_box()`.
    backups_unprotected: "{}, unprotected",
    /// Text in `record_lines`.
    backups_shown: "Shown",
    /// Text in `record_lines`.
    backups_not_shown: "Not shown",
    /// Text in `record_lines`.
    backups_not_seen: "Not seen",
    /// Text in `record_lines`.
    backups_nothing: "Nothing",
    /// Text in `files_state`.
    backups_not_made: "Not made",
    /// Text in `vault_state`.
    backups_vault_locked: "A vault is locked",
    /// Text in `vault_state`.
    backups_not_saved_yet: "Not saved yet",
    /// Text in `vault_state`.
    backups_changed_since: "changed since written",
    /// Text in `vault_state`.
    backups_unchanged: "unchanged",
    /// Text in `backup_line`.
    backups_not_backed: "Not backed up",
    /// Text in `backup_line`. `{}`: `n`.
    backups_paper_checked: "paper ×{} checked",
    /// Text in `backup_line`. `{}`: `k`, `n`.
    backups_paper_of_checked: "paper {} of {} checked",
    /// Text in `backup_line`. `{}`: `lower_first(&l.state)`.
    backups_files: "files {}",
    /// Text in `backup_line`.
    backups_planned: "planned",
    /// Text in `backup_line`. `{}`: `parts` joined by " · ".
    backups_backup: "Backup: {}",
    /// Text in `on_stick`. `{}`: `label`, `crate::vault::time_of_day(crate::vau…`.
    backups_on_stick_timed: "On {}, {}",
    /// Text in `on_stick`. `{}`: `label`.
    backups_on_stick: "On {}",
    // ----- backups (faraday-core/src/backups/screen.rs) -----
    /// Title in `draw`.
    backups_backups: "Backups",
    /// Line in `compact`.
    backups_no_wallet: "No wallet",
    /// Line in `compact`.
    backups_no_backup_plan: "No backup plan",
    /// Button in `compact`.
    backups_back_up: "Back up",
    /// Text in `compact`. `{}`: `at + 1`, `n`.
    backups_wallet_count: "Wallet {} of {}",
    /// Button in `compact`.
    backups_previous: "Previous",
    /// Button in `compact`.
    backups_next: "Next",
    /// Detail of a row in `line`.
    backups_part_check_detail: "Type its words",
    /// Line in `part_check_after`. `{}`: part number.
    backups_part_differs: "Does not match part {}",
        }
    };
}
