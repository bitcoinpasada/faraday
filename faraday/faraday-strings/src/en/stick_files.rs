//! Faraday's English words for the stick files area.

macro_rules! words_stick_files {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- A stick's FAT refused (faraday-files' `fat_text`, faraday-fat's kinds) -----
    /// Why a stick's file could not be read or written.
    fat_io: "the stick could not be read or written",
    /// Why a stick's file could not be read or written.
    fat_12: "a FAT12 volume, which Faraday does not read",
    /// Why a stick's file could not be read or written.
    fat_not_found: "no such file",
    /// Why a stick's file could not be read or written.
    fat_exists: "a file by that name is already there",
    /// Why a stick's file could not be read or written.
    fat_full: "the stick is full",
    /// Why a stick's file could not be read or written.
    fat_dir_full: "the stick's top folder is full",
    /// Why a stick's file could not be read or written.
    fat_too_large: "the file is too large",
    /// A partition is not FAT: its boot sector.
    fat_boot_signature: "no boot sector signature",
    /// A partition is not FAT: its boot sector.
    fat_boot_sector_size: "a sector size FAT does not have",
    /// A partition is not FAT: its boot sector.
    fat_boot_cluster_size: "a cluster size FAT does not have",
    /// A partition is not FAT: its boot sector.
    fat_boot_no_fat: "no reserved sectors or no FAT",
    /// A partition is not FAT: its boot sector.
    fat_boot_empty_fat: "a FAT of no size",
    /// A partition is not FAT: its boot sector.
    fat_boot_fats_larger: "the FATs are larger than the volume",
    /// A partition is not FAT: its boot sector.
    fat_boot_too_many_clusters: "too many clusters",
    /// A partition is not FAT: its boot sector.
    fat_boot_fat32_root: "a FAT32 volume with a FAT16 root directory",
    /// A partition is not FAT: its boot sector.
    fat_boot_fat16_clusters: "more clusters than FAT16 holds",
    /// A partition is not FAT: its boot sector.
    fat_boot_fat16_no_root: "a FAT16 volume with no root directory",
    /// A partition is not FAT: its boot sector.
    fat_boot_volume_larger: "a volume larger than its partition",
    /// A partition is not FAT: its boot sector.
    fat_boot_fat_small: "a FAT too small for its clusters",
    /// A partition is not FAT: its boot sector.
    fat_boot_root_outside: "a root directory outside the volume",
    /// A stick's filesystem contradicts itself.
    fat_fault_cluster_number: "a cluster number FAT16 cannot hold",
    /// A stick's filesystem contradicts itself.
    fat_fault_cluster_outside: "a cluster outside the volume",
    /// A stick's filesystem contradicts itself.
    fat_fault_loop: "a cluster chain that loops",
    /// A stick's filesystem contradicts itself.
    fat_fault_top_folder_long: "a top folder longer than FAT allows",
    /// A stick's filesystem contradicts itself.
    fat_fault_folder_long: "a folder longer than FAT allows",
    /// A stick's filesystem contradicts itself.
    fat_fault_file_longer: "a file longer than its clusters",
    /// A stick's filesystem contradicts itself.
    fat_fault_empty_root: "an empty root",
    /// A stick's filesystem contradicts itself: a write did not take.
    fat_fault_unwritten: "a block kept its old bytes",
    /// A stick's filesystem contradicts itself: two files share a cluster.
    fat_fault_shared_cluster: "two files share a cluster",
    /// A stick's filesystem contradicts itself: a file's last cluster is free.
    fat_fault_free_end: "a file's last cluster is marked free",
    /// A stick's filesystem hides entries: some follow the end of the folder.
    fat_fault_hidden_entries: "entries follow the end of the top folder; Zero the free space removes them",
    /// A file name FAT cannot hold.
    fat_name_empty: "an empty name",
    /// A file name FAT cannot hold.
    fat_name_long: "a name longer than 255 characters",
    /// A file name FAT cannot hold.
    fat_name_ending: "a name ending in a space or a dot",
    /// A file name FAT cannot hold.
    fat_name_character: "a name with a character FAT does not allow",
    // ----- A stick's file refused (faraday-files, the disk process) -----
    /// Why a file was not read: it is over the largest Faraday reads.
    /// `{}`: that size in MB.
    files_larger_than: "larger than {} MB",
    /// Why a rename was refused. `{}`: the new name.
    files_already_there: "{} is already there",
    /// Why a name was refused. `{}`: the name.
    files_not_a_name: "\"{}\" is not a file name",
    /// Why a file was not written: every numbered name is taken.
    files_no_free_name: "no free name",
    /// Why a file was not written: what was read back is not what was
    /// written.
    files_read_back: "read back different from what was written",
    /// Why a file was not written: another file took its name meanwhile.
    /// `{}`: the name.
    files_appeared: "{} appeared while writing",
    /// Why a vault was not written: the bytes are not a vault.
    files_not_a_vault: "not a vault",
    /// Why a vault was not written over its file. `{}`: the file.
    files_vault_not_ours: "{} is this vault, and this session did not unlock or write it: not replaced",
    /// Why a vault was not overwritten: the file is not the one the person
    /// was shown. `{}`: the file.
    files_taken_changed: "{} changed since it was shown: not overwritten",
    /// Why a vault was not written: a hidden file holds another copy of it.
    /// `{}`: that file.
    files_newer_waiting: "{} holds another copy of this vault",
    /// Why a vault was not written: two files hold it. `{}`: the files.
    files_vault_twice: "{} and {} are the same vault; remove one",
    /// Why a vault was not written: the stick holds too many files of a
    /// vault's length to check them all.
    files_too_many_vault_len: "The stick holds too many files the size of a vault to check them all",
    /// Why a rename was refused. `{}`: the file.
    files_not_on_stick: "{} is not on the stick",
    /// Why a picture was not read for codes: it is over the largest.
    files_picture_large: "the image is too large",
    /// Why a picture was refused: its codes and the picture together are
    /// more than the disk process can pass on.
    files_picture_codes_many: "the picture holds too many codes to read",
    /// Why a picture was not read for codes: it is not a PNG Faraday
    /// reads.
    files_picture_unread: "the picture could not be read",
    /// Why a file in a folder standing for a stick was refused: the
    /// computer does not allow it.
    files_denied: "the computer does not allow it",
    /// Why a file in a folder standing for a stick was refused: it is
    /// read-only.
    files_read_only: "the stick is read-only",
    /// Why the disk process does not open a partition: it is the one the
    /// machine started from.
    disk_boot: "the boot partition is never opened",
    /// Why the disk process could not answer: the stick was taken out.
    disk_gone: "the stick is gone",
    /// Why the disk process could not read a picture's codes now.
    disk_pictures_busy: "the pictures cannot be read now",
    /// Why the disk process could not answer: the request did not read.
    disk_malformed: "malformed request",
    /// Toast in `vault_form_save`. `{}`: `self.medium.for_the()`.
    vaults_made_the_certificate: "The certificate waits {}. The revocation certificate is kept with the key",
    /// Text in `vault_form_save`.
    vaults_note_empty: "The note is empty",
    /// Text in `vault_form_save`.
    vaults_an_entry_needs: "An entry needs a title",
    /// Toast in `vault_form_save`.
    vaults_not_a_totp_key: "Not a TOTP key",
    /// Toast in `vault_add_file`.
    vaults_unlock_a_vault: "Unlock a vault first",
    /// Text in `vault_add_file`. `{}`: `it.name`.
    vaults_is_in_the_vault_as_a_note: "{} is in the vault as a note",
    /// Text in `vault_add_file`. `{}`: `it.name`.
    vaults_file_added_as_key: "{} is in the vault as a key",
    /// Text in `vault_add_file`. `{}`: `it.name`.
    vaults_file_not_a_seed: "{} is not a valid seed in any list",
    /// Text in `vault_add_file`. `{}`: `it.name`.
    vaults_is_in_the_vault_as: "{} is in the vault as a wallet",
    /// Toast in `key_record`.
    vaults_this_key_has: "This key has no words to keep",
    /// Text in `vault_save_wallet`. `{}`: `name`.
    vaults_is_in_the_vault: "{} is in the vault",
    /// Toast in `vault_push_in`.
    vaults_slot_too_small: "More than a vault can hold",
    /// Toast in `seal_stop` and `vault_form_save`: the store refuses to write the vault.
    vaults_cannot_write: "This vault cannot be written as it is",
    /// Text in `vault_create_check`. `{}`: `medium.noun()`.
    vaults_remove_the_first: "Remove the {} first",
    /// Text in `vault_create_check`. `{}`: `n + 1`.
    vaults_finish_step: "Finish step {} first",
    /// Text in `vault_create_check`.
    vaults_every_passphrase: "Every passphrase needs a value",
    /// Text in `vault_create_check`.
    vaults_each_passphrase: "Each passphrase must be typed the same twice",
    /// Text in `vault_create_check`.
    vaults_choose_a_cost: "Choose a cost this computer can allocate",
    /// Text in `vault_create_check`, free memory not reported. `{}`:
    /// Light's memory.
    vaults_memory_unknown_create: "This computer does not say how much memory it has free. Choose Light, {}, which opens anywhere.",
    /// Toast in `vault_create`.
    vaults_vault_created: "Vault created",
    /// Text in `line`.
    vaults_never_written: "Never written",
    /// Text in `line`. `{}`: `label`.
    vaults_on_current: "On {} · current",
    /// Text in `line`.
    vaults_changed_since: "Changed since written",
    /// A vault's state in what a lock wipes, while the system has given
    /// no randomness.
    vaults_changes_not_sealed: "Changes not sealed",
    /// The pill under Lock while Make a new wallet's keys are kept
    /// nowhere yet.
    lock_held_pill: "Auto-lock off until backup complete",
    /// The lock and power sheets' wiped line then: the flow ends.
    lock_held_ends: "Make a new wallet · ends",
    /// Text in `line`.
    vaults_unchanged: "Unchanged",
    /// Text in `line`. `{}`: `self.wallets.len()`.
    vaults_wallets_n: "{} wallets",
    /// Text in `line`. `{}`: `self.keys[0]`.
    vaults_key: "key {}",
    /// Text in `line`. `{}`: `self.keys` joined by ", ".
    vaults_keys_inline: "keys {}",
    /// Text in `line`. `{}`: `n`.
    vaults_keys: "{} keys",
    /// Text in `line`.
    vaults_1_entry: "1 entry",
    /// Text in `line`. `{}`: `n`.
    vaults_entries_n: "{} entries",
    /// Text in `line`. `{}`: `time_of_day(local_secs(t, tz_min))`.
    vaults_seen: "seen {}",
    /// Text in `line`.
    vaults_empty_line: "Nothing in it",
    /// Text in `redact_place_name`. `{}`: `p + 1`, `rest`.
    vaults_place: "Place {}:{}",
    /// Text in `vault_summaries_refresh`.
    vaults_wallet: "Wallet",
        }
    };
}
