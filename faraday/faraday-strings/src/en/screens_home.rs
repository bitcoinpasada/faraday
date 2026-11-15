//! Faraday's English words for the screens home area.

macro_rules! words_screens_home {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- home_lead (faraday-core/src/screens/home.rs) -----
    /// Text in `home_lead`.
    home_lead_make_a_new: "Make a new wallet",
    /// Text in `home_lead`.
    home_lead_single_key: "Single key or multisig",
    // ----- home_job (faraday-core/src/screens/home.rs) -----
    /// Text in `home_job`.
    home_job_carry_on_spend: "Carry on the spend",
    /// Text in `home_job`. `{}`: `w.name`, `s.signers.len()`, `app.spend_needed()`.
    home_job_signatures: "{} · Signatures {} of {}",
    /// Text in `home_job`. `{}`: `s.spend.source`.
    home_job_continue_signing: "Continue signing {}",
    /// Text in `home_job`. `{}`: `s.signers.len()`, `app.spend_needed()`.
    home_job_of_signatures: "{} of {} signatures",
    /// Text in `home_job`. `{}`: `app.inbox[i].name`.
    home_job_sign: "Sign {}",
    /// Text in `home_job`.
    home_job_from_files: "From Files",
    /// Text in `home_job`. `{}`: `imp.label`.
    home_job_read_source: "Read from {}",
    /// Text in `home_job`. `{}`: `imp.label`.
    home_job_import_source: "Import from {}",
    /// Text in `home_job`. `{}`: `r.label`.
    home_job_written_target: "Written to {}",
    /// Text in `home_job`. `{}`: `f.name`.
    home_job_in_files: "{} · in Files",
    /// Text in `home_job`. `{}`: `st.label`.
    home_job_files_on: "Files on {}",
    /// Text in `home_job`. `{}`: `name`.
    home_job_unlock: "Unlock {}",
    /// Text in `home_job`. `{}`: `app.medium.noun()`.
    home_job_pull_to_unlock: "Pull the {} to unlock",
    /// Text in `home_job`. `{}`: `name`, `app.medium.a()`.
    home_job_write_target: "Write {} to {}",
    // ----- home_ways (faraday-core/src/screens/home.rs) -----
    /// Text in `home_ways`.
    home_ways_make_a_new: "Make a new wallet",
    /// Text in `home_ways`.
    home_ways_single_key: "Single key or multisig",
    /// Text in `home_ways`.
    home_ways_restore_wallet: "Restore an existing wallet",
    /// Text in `home_ways`.
    home_ways_restore_line: "From its backups",
    /// Text in `home_ways`.
    home_ways_spend_backup: "Spend from a backup, step by step",
    /// Text in `home_ways`.
    home_ways_a_stick_words: "A stick, words, or paper",
    /// Home's Check tile, always the fourth.
    home_ways_check: "Check a hardware wallet",
    /// Home's Check tile's line.
    home_ways_check_line: "Its seed, addresses and signatures",
    /// Text in `home_ways`.
    home_ways_a_stick_attached: "A stick attached",
    /// Text in `home_ways`.
    home_ways_backups: "Backups",
    /// Text in `home_ways`. `{}`: `known.len()`.
    home_ways_with_a_plan: "{} with a plan",
    /// Text in `home_ways`. `{}`: `without`.
    home_ways_not_backed: "{} not backed up",
    // ----- home (faraday-core/src/screens/home.rs) -----
    /// Title in `home`.
    home_home: "Home",
    /// Section label in `home`.
    home_start: "Start",
    /// Button in `home`.
    home_scan: "Scan",
        }
    };
}
