//! Faraday's English words for the stick area.

macro_rules! words_stick {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- keys_load (faraday-core/src/stick/mod.rs) -----
    /// Text in `keys_load`, and the refusal of a key added with a stick
    /// attached. `{}`: `m.noun()`.
    stick_keys_keys_load: "Keys load only with no {} attached",
    /// Said when a vault entry's or note's field is pressed, or the entry
    /// opened to edit, with a stick attached. `{}`: `m.noun()`.
    vaults_fields_with_stick: "Fields of a vault entry take no typing while a {} is attached",
    // ----- stick_settings (faraday-core/src/stick/settings.rs) -----
    /// Text in `settings_event`. `{}`: `wrote_as`.
    stick_settings_wrote: "Wrote {}, read back and matched",
    /// Text in `settings_event`. `{}`: `name`, `reason`.
    stick_settings_not_written: "{} not written: {}",
    // ----- stick_wipe (faraday-core/src/stick/wipe.rs) -----
    /// Text in `LIMIT`.
    /// Text in `REACH`.
    stick_wipe_faraday: "Faraday only ever writes into this partition, so zeroing it reaches everything Faraday ever wrote to the stick.",
    /// Text in `SECRET`.
    stick_wipe_if_a_seed: "If a seed or other secret was ever saved to this stick unencrypted, the only sure way to remove it is to destroy the stick.",
    /// Text in `WITHIN`.
    stick_wipe_within: "Only this stick's files partition is written. Its other partitions, its partition table and any space outside a partition are not touched.",
    /// Text in `BOUNDARY`.
    /// Text in `minutes_text`.
    stick_wipe_about_minute: "about a minute",
    /// Text in `minutes_text`. `{}`: `m`.
    stick_wipe_about_minutes: "about {} minutes",
    /// Text in `minutes_text`. `{}`: `m as f64 / 60.0`.
    stick_wipe_about_hours: "about {} hours",
    /// Text in `wiped`. `{}`: `name`.
    stick_wipe_deleted: "{} deleted, zeros over it first",
    /// Text in `wiped`. `{}`: `name`.
    stick_wipe_name_only: "{} removed by name, its bytes left",
    /// Text in `wiped`. `{}`: `name`, `why`.
    stick_wipe_not_deleted: "{} not deleted: {}",
    /// Text in `zero_step`. `{}`: `share`.
    stick_wipe_stopped_percent: "Stopped at {}%",
    /// Text in `zero_step`. `{}`: `z.label`.
    stick_wipe_free_space: "Free space on {} zeroed",
    /// Text in `zero_step`. `{}`: `z.label`.
    stick_wipe_s_partition_zeroed: "{}'s partition zeroed",
    /// Text in `zero_step`. `{}`: `z.label`, `n`.
    stick_wipe_s_partition: "{}'s partition zeroed, {} passes",
    /// Text in `zero_failed`. `{}`: `share`, `why`.
    stick_wipe_stopped: "Stopped at {}%: {}",
    /// Text in `zero_sticks_changed`. `{}`: `self.medium.noun()`.
    stick_wipe_the_was: "the {} was pulled",
    /// Text in `delete_sheet`. `{}`: `d.label`.
    stick_wipe_delete_files: "Delete files from {}",
    /// Text in `delete_sheet`. `{}`: `n`, `if n == 1 { EN.stick_wipe_file } else…`.
    stick_wipe_delete_zeros: "Delete {} {} with zeros",
    /// Text in `delete_sheet`.
    stick_wipe_file: "file",
    /// Text in `delete_sheet`.
    stick_wipe_files: "files",
    /// Text in `delete_sheet`.
    stick_wipe_zero_the_free: "Zero the free space",
    /// Text in `delete_sheet`.
    stick_wipe_zero_partition: "Zero the whole partition",
    /// Text in `delete_sheet`.
    stick_wipe_nothing: "Nothing to write: a folder standing for a stick",
    /// Text in `delete_sheet`. `{}`: `size_text(shown)`, `minutes_text(ms)`.
    stick_wipe_to_write: "{} to write · {} on a typical stick",
    /// Text in `delete_sheet`.
    stick_wipe_reading: "Reading the stick's space",
    /// Line in `delete_sheet`.
    stick_wipe_danger_zone: "Danger zone: what is deleted here cannot be undone",
    /// Text in `delete_sheet`.
    stick_wipe_chosen_files: "Chosen files",
    /// Text in `delete_sheet`.
    stick_wipe_zeros_over: "Each is overwritten with zeros, then removed along with its name",
    /// Text in `delete_sheet`.
    stick_wipe_the_free: "The free space",
    /// Text in `delete_sheet`.
    stick_wipe_the_files: "The files stay; what earlier deletions left behind is zeroed",
    /// Text in `delete_sheet`.
    stick_wipe_the_whole: "The whole partition",
    /// Text in `delete_sheet`.
    stick_wipe_every_file: "Every file is deleted, then all of it is zeroed",
    /// Text in `delete_sheet`. `{}`: `app.medium.noun()`.
    stick_wipe_no_files: "No files on this {}",
    /// Text in `delete_sheet`.
    stick_wipe_one_pass: "One pass",
    /// Text in `delete_sheet`.
    stick_wipe_two_passes_label: "Two passes",
    /// Line in `delete_sheet`.
    stick_wipe_two_passes: "Two passes write it all twice, which makes a stick replace more of its old blocks.",
    /// Text in `delete_sheet`.
    stick_wipe_cancel: "Cancel",
    /// Text in `zeroing_sheet`. `{}`: `z.label`.
    stick_wipe_zeroing_title: "Zeroing the whole partition on {}",
    /// Text in `zeroing_sheet`. `{}`: `z.label`.
    stick_wipe_zeroing: "Zeroing the free space on {}",
    /// Text in `zeroing_sheet`. `{}`: `(share * 100.0) as u32`, `minutes_text(ms)`.
    stick_wipe_left: "{}% · {} left",
    /// Text in `zeroing_sheet`. `{}`: `z.pass`, `z.passes`.
    stick_wipe_pass_count: "Pass {} of {}",
    /// Text in `zeroing_sheet`. `{}`: `app.medium.noun()`.
    stick_wipe_keep_medium: "Keep the {} in until it is done",
    /// Text in `zeroing_sheet`.
    stick_wipe_stopping: "Stopping",
    /// Text in `zeroing_sheet`.
    stick_wipe_stop: "Stop",
        }
    };
}
