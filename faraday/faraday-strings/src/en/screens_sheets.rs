//! Faraday's English words for the screens sheets area.

macro_rules! words_screens_sheets {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- potential (faraday-core/src/screens/sheets.rs) -----
    /// Text in `potential_sheet`. `{}`: `fp_text(fp)`.
    potential_a_watch_only_wallet: "A watch-only wallet from {}",
    /// Text in `potential_sheet`.
    potential_a_watch_only: "A watch-only wallet from an xpub",
    /// Text in `potential_sheet`. `{}`: `fp_text(fp)`.
    potential_a_wallet: "A wallet from {}",
    /// Text in `potential_sheet`. `{}`: `from` joined by " · ".
    potential_sources: "From {}",
    /// Text in `potential_sheet`.
    potential_slip_39_passphrase: "SLIP-39 passphrase · none if left empty",
    /// Text in `potential_sheet`.
    potential_bip_39_passphrase: "BIP-39 passphrase · none if left empty",
    /// Text in `potential_sheet`. `{}`: `crate::spend_backup::kind_name(p.kind)`.
    potential_as_its_path: "{} · as its path says",
    /// Line in `potential_sheet`.
    potential_fingerprint: "Fingerprint",
    /// Text in `potential_sheet`. `{}`: `name`.
    potential_key_of_wallet: "A key of {}: it loads with that wallet",
    /// Line in `potential_sheet`.
    potential_single_key: "Single-key wallet",
    /// Button in `potential_sheet`.
    potential_cancel: "Cancel",
    /// Text in `potential_sheet`. `{}`: `name`.
    potential_load: "Load {}",
    /// Text in `potential_sheet`.
    potential_make_wallet: "Make the wallet",
    // ----- pull (faraday-core/src/screens/sheets.rs) -----
    /// Text in `pull_sheet`.
    pull_default_reason: "go on",
    /// Text in `pull_sheet`. `{}`: `app.medium.noun()`, `what`.
    pull_medium_to_reason: "Pull the {} to {}",
    /// Text in `pull_sheet`. `{}`: `app.sticks.first().map_or("", |s| s.l…`.
    pull_medium_attached: "{} · attached",
    /// Text in `pull_sheet`. `{}`: `k`, `app.medium.nouns()`.
    pull_attached: "{} {} attached",
    /// Button in `pull_sheet`.
    pull_cancel: "Cancel",
    // ----- kept_sections (faraday-core/src/screens/sheets.rs) -----
    /// Text in `kept_sections`.
    kept_sections_kept: "Kept",
    /// Text in `kept_sections`.
    kept_sections_wiped: "Wiped from memory",
    /// Text in `kept_sections`.
    kept_sections_then: "Then",
    // ----- lock (faraday-core/src/screens/sheets.rs) -----
    /// Text in `lock_sheet`. `{}`: `app.medium.noun()`.
    lock_a_fresh_start: "A fresh start, and the {} visit",
    /// Text in `lock_sheet`. `{}`: `app.medium.a_cap()`.
    lock_is_attached: "{} is attached",
    /// Text in `lock_sheet`. `{}`: `app.medium.noun()`.
    lock_lock_and_use: "Lock and use {}",
    /// Button in `lock_sheet` with one key loaded. `{}`: `app.medium.noun()`.
    lock_lock_and_use_one_key: "Lock and use {} · 1 key leaves memory",
    /// Button in `lock_sheet`. `{}`: `app.medium.noun()`, the number of keys.
    lock_lock_and_use_keys: "Lock and use {} · {} keys leave memory",
    /// Button in `lock_sheet` where a camera is present.
    lock_scan_instead: "Scan it as a QR code instead",
    /// Text in `lock_sheet`. `{}`: `label`.
    lock_nothing_has_been: "{} · nothing has been read from it",
    /// Text in `lock_sheet`.
    lock_not_now: "Not now",
    // ----- write_out (faraday-core/src/screens/sheets.rs) -----
    /// Text in `write_out_sheet`. `{}`: `app.medium.a()`.
    write_out_lock_plug: "Lock, plug in {}, and the visit writes them",
    /// Text in `write_out_sheet`, with the stick already in.
    write_out_lock_here: "Lock, and the visit writes them",
    /// Text in `write_out_sheet`, for a take-away's write. First `{}`: the
    /// files' names; second: the stick's label.
    write_out_lock_names: "Lock, then write {} to {}",
    /// Text in `write_out_sheet`. `{}`: `app.medium.a()`.
    write_out_target: "Write to {}",
    /// Text in `write_out_sheet`.
    write_out_not_now: "Not now",
    /// Text in `write_out_sheet`.
    write_out_lock: "Lock",
    /// Heading in `write_out_sheet`: the files from the stick a lock drops.
    write_out_drops: "Dropped from Files",
    /// Text in `in_own_words`: why the disk process refused a request (a
    /// read, a write, a delete), the process being no longer clean.
    storage_event_not_open: "not open to Faraday until it locks",
    // ----- scan (faraday-core/src/screens/sheets.rs) -----
    /// Text in `scan_sheet`. `{}`: `wl.name`.
    scan_key_for_wallet: "Scan a key for {}",
    /// Text in `scan_sheet`.
    scan_scan_what_you: "Scan what you have",
    /// Text in `scan_sheet`.
    scan_scan_a_seedqr: "Scan a SeedQR",
    /// Text in `scan_sheet`.
    scan_scan_your_copy: "Scan your copy",
    /// Text in `scan_sheet`.
    scan_receive_into_downloads: "Receive into Downloads",
    /// Text in `scan_sheet`.
    scan_scan_a_qr_code: "Scan a QR code",
    /// Text in `scan_sheet`.
    scan_hold_the_code: "Hold the QR in front of the camera",
    /// Line in `scan_sheet`.
    scan_starting_the_camera: "Starting the camera",
    /// Text in `scan_sheet`.
    scan_type_the_words: "Type the words instead",
    /// Text in `scan_sheet`.
    scan_done: "Done",
    /// Text in `scan_sheet`.
    scan_cancel: "Cancel",
    /// Button in `scan_sheet`: the clipboard's text, read as a QR.
    scan_paste: "Paste",
    /// Line in `scan_sheet`: no frame came in the time allowed.
    scan_no_picture: "The camera sends no picture",
    /// Line in `scan_sheet`: `CameraFailure::Refused`.
    scan_camera_refused: "The camera was refused",
    /// Line in `scan_sheet`: `CameraFailure::InUse`.
    scan_camera_in_use: "The camera is in use by another app",
    /// Line in `scan_sheet`: `CameraFailure::NotFound`.
    scan_no_camera_found: "No camera found",
    /// Note in `storage_event`: a QR seen large enough has not read.
    scan_hold_square_hint: "Hold it square-on, in focus, with its white margin in the frame",
    // ----- qr (faraday-core/src/screens/sheets.rs) -----
    /// Text in `qr_sheet`. `{}`: `q.subtitle`, `q.frame + 1`, `q.frames.len()`.
    qr_frame_count: "{} · frame {} of {}",
    /// Text in `qr_sheet`.
    qr_secret_whoever_scans: "Secret · whoever scans this can use it",
    /// Text in `qr_sheet`.
    qr_public_safe_to_scan: "Public · safe to scan",
    /// Text in `qr_sheet`.
    qr_ur: "UR",
    /// Text in `qr_sheet`.
    qr_bbqr: "BBQr",
    /// Text in `qr_sheet`.
    qr_parts: "Parts",
    /// Text in `qr_sheet`. `{}`: `app.medium.for_the()`.
    qr_png: "PNG {}",
    /// Text in `qr_sheet`.
    qr_small: "Small",
    /// Text in `qr_sheet`.
    qr_medium: "Medium",
    /// Text in `qr_sheet`.
    qr_large: "Large",
    /// Button in `qr_sheet`.
    qr_done: "Done",
    // ----- qr_sheet (faraday-core/src/screens/sheets.rs) -----
    /// Text in `qr_sheet_compact`.
    qr_sheet_ur: "UR",
    /// Text in `qr_sheet_compact`.
    qr_sheet_bbqr: "BBQr",
    /// Text in `qr_sheet_compact`.
    qr_sheet_parts: "Parts",
    /// Text in `qr_sheet_compact`.
    qr_sheet_small: "Small",
    /// Text in `qr_sheet_compact`.
    qr_sheet_medium: "Medium",
    /// Text in `qr_sheet_compact`.
    qr_sheet_large: "Large",
    /// Text in `qr_sheet_compact`. `{}`: `q.title`, `q.frame + 1`, `q.frames.len()`.
    qr_sheet_frame_count: "{} · {} of {}",
    /// Text in `qr_sheet_compact`.
    qr_sheet_secret_whoever: "Secret · whoever scans this can use it",
    /// Text in `qr_sheet_compact`.
    qr_sheet_public_safe: "Public · safe to scan",
    /// Text in `qr_sheet_compact`.
    qr_sheet_done: "Done",
    // ----- new_input (faraday-core/src/screens/sheets.rs) -----
    /// Text in `new_input_sheet`.
    new_input_title: "A new keyboard and pointer",
    /// Text in `new_input_sheet`.
    new_input_a_new_keyboard: "A new keyboard",
    /// Text in `new_input_sheet`.
    new_input_a_new_pointer: "A new pointer",
    /// Line in `new_input_sheet`.
    new_input_the_name: "The name is what the device says about itself",
    /// Line in `new_input_sheet`.
    new_input_type_code: "Type this code on it",
    /// Line in `new_input_sheet`, in place of the code while the system
    /// has given no randomness.
    new_input_no_code: "This keyboard cannot be let in until Faraday restarts",
    /// Button in `new_input_sheet`.
    new_input_ignore_button: "Ignore this device",
    /// Button in `new_input_sheet`.
    new_input_use_pointer: "Use this pointer",
    /// Title in `new_input_target`.
    new_input_a_new_touchscreen: "A new touchscreen",
    /// Label in `new_input_target`, over the target.
    new_input_tap_target: "Tap the target",
    /// Label in `new_input_target`, over a pointer's target.
    new_input_press_target: "Press the target",
    /// Line in `new_input_target` of a pointer, shown only with a trusted
    /// keyboard.
    new_input_enter_also: "Enter on the keyboard also lets the pointer in.",
    /// Line in `new_input_target`, which target of how many. `{}`: the
    /// one showing, from 1; how many.
    new_input_target_step: "Target {} of {}",
    /// Line in `new_input_target`, in place of the target while the
    /// system has no randomness or no number of targets would do, for a
    /// pointer.
    new_input_no_target_pointer: "This pointer cannot be let in until Faraday restarts",
    /// Value in `new_input_target`, once a tap missed. `{}`: tries left.
    new_input_tries_left: "Tries left: {}",
    /// Line in `new_input_target`, in place of the target while the
    /// system has given no randomness.
    new_input_no_target: "This touchscreen cannot be let in until Faraday restarts",
    // ----- idle_warn (faraday-core/src/screens/sheets.rs) -----
    /// Text in `idle_warn_sheet`.
    idle_warn_wiped: "Wiped",
    /// Text in `idle_warn_sheet`. `{}`: `keys`, `if keys == 1 { EN.idle_warn_key } els…`, `wallets`, `if wallets == 1 { EN.idle_warn_wallet…`, `app.vaults.open.len()`, `if app.vaults.open.len() == 1 { EN.id…`.
    idle_warn_open: "{} {} · {} {} · {} open {}",
    /// Text in `idle_warn_sheet`.
    idle_warn_key: "key",
    /// Text in `idle_warn_sheet`.
    idle_warn_keys: "keys",
    /// Text in `idle_warn_sheet`.
    idle_warn_wallet: "wallet",
    /// Text in `idle_warn_sheet`.
    idle_warn_wallets: "wallets",
    /// Text in `idle_warn_sheet`.
    idle_warn_vault: "vault",
    /// Text in `idle_warn_sheet`.
    idle_warn_vaults: "vaults",
    /// Text in `idle_warn_sheet`.
    idle_warn_sealed: "Sealed",
    /// Text in `idle_warn_sheet`.
    idle_warn_kept: "Kept",
    /// Text in `idle_warn_sheet`.
    idle_warn_nowhere: "Kept nowhere",
    /// Text in `idle_warn_sheet`.
    idle_warn_no_power: "No power-off",
    /// Text in `idle_warn_sheet`. `{}`: `m`, `app.medium.for_the()`.
    idle_warn_power_off: "Power off at {} minutes without input, unless files wait {}",
    /// Text in `idle_warn_sheet`.
    idle_warn_then: "Then",
    /// Text in `idle_warn_sheet`.
    idle_warn_minute: "minute",
    /// Text in `idle_warn_sheet`.
    idle_warn_minutes: "minutes",
    /// Text in `idle_warn_sheet`. `{}`: `minutes(app.idle_warn_min)`, `minutes(u64::from(app.idle_lock_min))`.
    idle_warn_no_input: "No input for {} · locks at {} without input · any key or touch keeps it",
    /// Line in `idle_warn_sheet`.
    idle_warn_locking_soon: "Locking soon",
    /// Button in `idle_warn_sheet`.
    idle_warn_keep_working: "Keep working",
    // ----- locked (faraday-core/src/screens/sheets.rs) -----
    /// Line in `locked_sheet`.
    locked_locked: "Locked",
    /// Line in `locked_sheet`.
    locked_locked_after: "Locked after a time without input. Every key and vault was wiped.",
    /// Title of `locked_sheet` when nothing was locked: the power-off time
    /// came with Files, For the stick, not empty.
    locked_not_locked: "Not locked",
    /// Line in `locked_sheet`, error tone, when a waiting file holds a secret
    /// in the clear. `{}`: how many.
    locked_secret_waiting: "Waiting files with a secret in the clear: {}. A lock does not wipe them.",
    /// Line in `locked_sheet`, after a vault's changes were lost.
    locked_changes_lost: "Changes to a vault were not saved. They did not fit in the vault, or the vault could not be written.",
    /// Line in `locked_sheet`, after a vault was sealed without its new memory step.
    locked_step_dropped: "A vault was sealed at its earlier memory setting. This computer did not have the memory for the new one.",
    /// Text in `locked_sheet`. `{}`: `clock_text(off.saturating_sub(app.idl…`.
    locked_power_off_countdown: "Powers off in {} without input",
    /// Text in `locked_sheet`. `{}`: `app.outbox.len()`, `app.medium.for_the()`.
    locked_no_power_off: "{} {} · no power-off until written",
    /// Button in `locked_sheet`.
    locked_continue: "Continue",
    // ----- power (faraday-core/src/screens/sheets.rs) -----
    /// Text in `power_sheet`.
    power_the_computer: "The computer powers off",
    /// Text in `power_sheet`.
    power_cancel: "Cancel",
    /// Text in `power_sheet`.
    power_power_off: "Power off",
    /// Title and button of `power_sheet` in the desktop app.
    power_quit: "Quit",
    /// Text in `power_sheet` in the desktop app.
    power_app_closes: "Faraday closes",
    /// Title of `power_sheet` where it offers Restart.
    power_title: "Power",
    /// Button in `power_sheet`.
    power_restart: "Restart",
    /// Button in `power_sheet` while files wait or a vault has unsaved
    /// changes, held to act.
    power_hold_restart: "Hold to restart",
    /// Button in `power_sheet`, as `power_hold_restart`.
    power_hold_power_off: "Hold to power off",
    /// Button in `power_sheet` in the desktop app, as `power_hold_restart`.
    power_hold_quit: "Hold to quit",
    /// Text in `power_sheet` where it offers Restart.
    power_the_computer_restarts: "The computer powers off, or restarts",
    /// Label of the row in `power_sheet` that gives this computer's BIOS key.
    power_bios_key: "Usual BIOS key",
    /// Dell's BIOS key.
    bios_key_dell: "F2",
    /// HP's BIOS key.
    bios_key_hp: "Esc, then F10",
    /// Lenovo ThinkPad's BIOS key.
    bios_key_lenovo_thinkpad: "F1",
    /// Lenovo IdeaPad, Yoga and Legion's BIOS key.
    bios_key_lenovo_ideapad: "F2 (or Fn+F2)",
    /// ASUS's BIOS key.
    bios_key_asus: "F2, held",
    /// Acer's BIOS key.
    bios_key_acer: "F2",
    /// MSI's BIOS key.
    bios_key_msi: "Delete",
    /// Framework's BIOS key.
    bios_key_framework: "F2",
    /// Surface's BIOS key.
    bios_key_surface: "Volume Up, held",
    /// Dell's boot menu key.
    boot_key_dell: "F12",
    /// HP's boot menu key.
    boot_key_hp: "Esc, then F9",
    /// Lenovo ThinkPad's boot menu key.
    boot_key_lenovo_thinkpad: "F12",
    /// Lenovo IdeaPad, Yoga and Legion's boot menu key.
    boot_key_lenovo_ideapad: "F12 (or Fn+F12)",
    /// ASUS's boot menu key.
    boot_key_asus: "Esc, held",
    /// Acer's boot menu key.
    boot_key_acer: "F12",
    /// MSI's boot menu key.
    boot_key_msi: "F11",
    /// Framework's boot menu key.
    boot_key_framework: "F12",
    /// Surface's boot menu key.
    boot_key_surface: "Volume Down, held",
    /// Where the Secure Boot setting is in Dell's BIOS.
    secure_boot_dell: "Boot Configuration → Secure Boot",
    /// Where the Secure Boot setting is in HP's BIOS.
    secure_boot_hp: "Boot Options → Secure Boot; business: Security → Secure Boot Configuration",
    /// Where the Secure Boot setting is in Lenovo ThinkPad's BIOS.
    secure_boot_lenovo_thinkpad: "Security → Secure Boot",
    /// Where the Secure Boot setting is in Lenovo IdeaPad, Yoga and Legion's BIOS.
    secure_boot_lenovo_ideapad: "Security → Secure Boot",
    /// Where the Secure Boot setting is in ASUS's BIOS.
    secure_boot_asus: "Advanced Mode → Security → Secure Boot → Secure Boot Control; desktops: Boot → Secure Boot → OS Type",
    /// Where the Secure Boot setting is in Acer's BIOS.
    secure_boot_acer: "Boot → Secure Boot",
    /// Where the Secure Boot setting is in MSI's BIOS.
    secure_boot_msi: "Security → Secure Boot → Secure Boot Support",
    /// Where the Secure Boot setting is in Framework's BIOS.
    secure_boot_framework: "Administer Secure Boot → Enforce Secure Boot",
    /// Where the Secure Boot setting is in Surface's BIOS.
    secure_boot_surface: "Security → Secure Boot",
    // ----- guide (faraday-core/src/screens/sheets.rs) -----
    /// Button in `guide_sheet`.
    guide_done: "Done",
    /// Text button at the foot of the guide's sheet and pane: opens the
    /// contents.
    guide_contents: "Contents",
    /// Hint of the contents' search field.
    guide_find: "Find in the guide",
    /// Line of the contents when no page matches what is typed.
    guide_no_page_matches: "No page matches",
    /// Button under it: empties the field.
    guide_clear_find: "Clear",
    /// Heading of the contents' first group, the place the guide was
    /// opened from.
    guide_here: "Here",
    /// Heading of the list of glossary words a guide page uses, under its
    /// lead.
    guide_words_here: "Words on this page",
    /// Heading of the glossary words Find a tool and Home's search find,
    /// after the settings.
    guide_words_found: "Words",
    /// A heading of the guide's contents.
    guide_group_get: "Get Faraday",
    /// A heading of the guide's contents.
    guide_group_first: "First start",
    /// A heading of the guide's contents.
    guide_group_keys: "Keys",
    /// A heading of the guide's contents.
    guide_group_backups: "Backups",
    /// A heading of the guide's contents.
    guide_group_wallets: "Wallets",
    /// A heading of the guide's contents.
    guide_group_spending: "Spending",
    /// A heading of the guide's contents.
    guide_group_vaults: "Vaults",
    /// A heading of the guide's contents.
    guide_group_tools: "Tools",
    /// A heading of the guide's contents.
    guide_group_settings: "Settings",
    /// A heading of the guide's contents.
    guide_group_starter: "The Starter",
    /// The guide's own tab, beside Cryptography, where a place offers it.
    guide_tab_guide: "Guide",
    /// The close button's label in the guide's supporting pane
    /// (`screens/guide_pane.rs`).
    guide_close: "Close",
    /// The first tab of the guide sheet and of the supporting pane (§8.2).
    guide_tab_how: "Cryptography",
    /// The head of the Starter's sheet of blocks (§8.4).
    starter_how_title: "Cryptography",
    /// The foot of the Starter's sheet of blocks.
    starter_how_close: "Close",
    /// Guide title in the Starter's guide list.
    /// A block's field (`docs/CRYPTO-LEARN.md` §1).
    how_field_action: "Action",
    /// A block's field: the specifications the action follows (F179).
    how_field_standards: "Specification",
    /// A block's field: its libraries and Faraday's code (F179).
    how_field_implementation: "Code",
    /// A block's field.
    how_field_checked: "Checked against",
    /// A block's field.
    how_field_crosscheck: "Cross-check",
    /// A block's field: the release, file and function of each entry (F179).
    how_field_code: "Source",
    /// A block's field.
    how_field_secret: "Secret handling",
    /// A block's field.
    how_field_limits: "Limits",
    /// A block's field.
    how_field_own_calls: "Faraday's own calls",
    /// A part of a block's Cross-check (§8.5).
    how_cc_area: "Area",
    /// A part of a block's Cross-check.
    how_cc_code: "Faraday's code",
    /// A part of a block's Cross-check.
    how_cc_second: "Compared with",
    /// A part of a block's Cross-check.
    how_cc_vectors: "Vectors",
    /// A part of a block's Cross-check.
    how_cc_random: "Random cases",
    /// A part of a block's Cross-check.
    how_cc_differences: "Differences",
    /// A part of a block's Cross-check.
    how_cc_when: "When it runs",
    /// A part of a block's Cross-check.
    how_cc_last_run: "Last passing run",
    /// The Last passing run's value. `{}`: the seed, then the area's case count.
    how_cc_seed_cases: "Seed {} · {} cases",
    /// One Code entry of a block. `{}`: the release, the file, the function.
    how_code_entry: "{} · {} · {}",
    /// The Code field's last entry.
    how_repository: "github.com/bitcoinpasada/faraday",
    // ----- secret_out (faraday-core/src/screens/sheets.rs) -----
    /// Text in `secret_out_sheet`.
    secret_out_this_may: "This may be a secret",
    /// Text in `secret_out_sheet`.
    secret_out_title: "This is a secret",
    /// Text in `secret_out_sheet`. `{}`: `app.medium.for_the()`.
    secret_out_put_it_unprotected: "Put it {} unprotected",
    /// Text in `secret_out_sheet`. `{}`: `app.medium.noun()`.
    secret_out_unprotected_medium: "Unprotected, on the {}",
    /// Text in `secret_out_sheet`. `{}`: `app.medium.for_the()`.
    secret_out_unprotected: "Unprotected, {}",
    /// Text in `secret_out_sheet`.
    secret_out_file: "File",
    /// Text in `secret_out_sheet`.
    secret_out_what_label: "What it is",
    /// Text in `secret_out_sheet`.
    secret_out_who_can: "Who can use it",
    /// Text in `secret_out_sheet`.
    secret_out_stays_label: "Stays",
    /// Text in `secret_out_sheet`.
    secret_out_stays: "Until it is sent or removed, or until power-off",
    /// Line in `secret_out_sheet`.
    secret_out_sealed: "Sealed in a vault",
    /// Text in `secret_out_sheet`. `{}`: `name`.
    secret_out_in_vault: "In {}",
    /// Text in `secret_out_sheet`. `{}`: `name`.
    secret_out_save_into: "Save into {}",
    /// Button in `secret_out_sheet`.
    secret_out_cancel: "Cancel",
    // ----- plain (faraday-core/src/screens/sheets.rs, `plain_sheet`; F129) -----
    /// Title of `plain_sheet`: a secret about to be saved as a plain file.
    plain_title: "Save without encryption?",
    /// Line in `plain_sheet`. `{}`: the file's name, where it goes, the
    /// medium's noun.
    plain_goes_on: "{} goes on the {} as a plain file, outside any vault. Anyone who has the {} can read it.",
    /// Line in `plain_sheet`, Transfer's Receive. `{}`: the file's name,
    /// the folder.
    plain_goes_into: "{} goes into {} as a plain file, outside any vault. Anyone who has this computer can read it.",
    /// Line in `plain_sheet`. `{}`: the medium's noun, or
    /// `plain_computer`.
    plain_as_safe: "That is as safe as writing it on paper: it is as safe as the place you keep the {}.",
    /// Line in `plain_sheet`, a stick. `{}`: the medium's noun, twice.
    plain_stick_exposed: "Any computer this {} is plugged into can read it. Keep this {} away from an online computer.",
    /// What Transfer's Receive saves into, in `plain_as_safe`.
    plain_computer: "computer",
    /// Where a plan's file goes, in `plain_goes_on`. `{}`: the medium's
    /// noun, the place.
    plain_stick_at: "{} at {}",
    /// The hold in `plain_sheet`.
    plain_hold: "Hold to save without encryption",
    /// Button in `plain_sheet`, with a vault open: the secret into it.
    plain_into_vault: "Put it in a vault",
        }
    };
}
