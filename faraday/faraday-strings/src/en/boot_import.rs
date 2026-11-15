//! Faraday's English words for the boot import area.

macro_rules! words_boot_import {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- boot_import (faraday-core/src/boot_import/mod.rs) -----
    /// Text in `line`.
    boot_import_vault_inline: "vault",
    /// Text in `line`.
    boot_import_vaults: "vaults",
    /// Text in `line`.
    boot_import_psbt: "PSBT",
    /// Text in `line`.
    boot_import_psbts: "PSBTs",
    /// Text in `line`.
    boot_import_other_file: "other file",
    /// Text in `line`.
    boot_import_other_files: "other files",
    /// Text in `line`.
    boot_import_no_files: "No files Faraday reads",
    /// Text in `line`, before Import: the counts guessed from the files'
    /// names. `{}`: "1 vault · 2 PSBTs · 4 other files".
    boot_import_by_names: "{}, by their names",
    /// Text in `line`, while Import reads. `{}`: files read, files asked for.
    boot_import_files_read: "{} of {} files read",
    /// Text in `import_view`, a file before Import: what it is by its name
    /// alone, nothing of it read (F130). `{}`: `stick_kind(name)`, `size`.
    boot_import_by_name: "{}, by its name · {}",
    /// Note in `import_codes`.
    boot_import_not_a_psbt: "Not a PSBT, a descriptor, an xpub or a signed message",
    /// Text in `import_codes`.
    boot_import_no_qr_code: "No QR code found",
    /// Text in `import_codes`.
    boot_import_its_codes: "Its codes are not complete",
    /// Text in `import_codes`. `{}`: `n`, `if n == 1 { EN.boot_import_code } els…`, `kinds` joined by ", ".
    boot_import_qr: "{} QR {} · {}",
    /// Text in `import_codes`.
    boot_import_code: "code",
    /// Text in `import_codes`.
    boot_import_codes: "codes",
    /// Text in `import_stick_gone`. `{}`: `noun`.
    boot_import_the_was: "the {} was removed before it was read",
    /// Text in `import_view`. `{}`: `size`.
    boot_import_being_read: "Being read · {}",
    /// Text in `import_view`. `{}`: `size`.
    boot_import_not_a_kind: "Not a kind of file Faraday reads · {}",
    /// Text in `import_view`. `{}`: `why`.
    boot_import_not_read: "Not read: {}",
    /// Text in `import_view`. `{}`: `size`.
    boot_import_vault_n: "Vault · {}",
    /// Text in `import_if_read`. `{}`: `count(wallets, EN.boot_import_wallet_inline,…`, `count(keys, EN.boot_import_key, EN.bo…`, `count(moved, EN.boot_import_file, EN.…`.
    boot_import_imported: "Imported {} · {} · {}",
    /// Text in `import_toast`, after `boot_import_imported` and a " · " when a file
    /// Import asked for was not read. `{}`: how many.
    boot_import_n_not_read: "{} not read",
    /// The import's toast when a file came in beside one of the same
    /// name in Files. `{}`: the toast so far, then the new names.
    boot_import_kept_as: "{} · already in Files, kept as {}",
    /// Text in `import_if_read`.
    boot_import_wallet_inline: "wallet",
    /// Text in `import_if_read`.
    boot_import_wallets_inline: "wallets",
    /// Text in `import_if_read`.
    boot_import_key: "key",
    /// Text in `import_if_read`.
    boot_import_keys: "keys",
    /// Text in `import_if_read`.
    boot_import_file: "file",
    /// Text in `import_if_read`.
    boot_import_files_inline: "files",
    // ----- boot_import (faraday-core/src/boot_import/screen.rs) -----
    /// Text in `body`. `{}`: `label`.
    boot_import_read_source: "Read from {}",
    /// Text in `body`. `{}`: `app.medium.noun()`.
    boot_import_pull_medium: "Pull the {} to continue",
    /// Text in `picker`, the boot stick pulled before Import, which reads
    /// it. `{}`: `app.medium.noun()`.
    boot_import_put_back: "Put the {} back in to import",
    /// Text in `body`. `{}`: `crate::vault::file_text(vault.len)`.
    boot_import_locked: "Locked · {}",
    /// Text in `picker`.
    boot_import_later: "Later",
    /// Text in `unlock`.
    boot_import_cancel: "Cancel",
    /// Text in `body`.
    boot_import_import: "Import",
    /// Text in `body`.
    boot_import_unlock: "Unlock",
    /// Text in `unlocking`. `{}`: `time`.
    boot_import_unlocking: "Unlocking · {}",
        }
    };
}
