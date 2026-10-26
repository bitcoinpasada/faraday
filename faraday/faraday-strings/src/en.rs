//! Faraday's English wording: every string Faraday's screens, the stick's
//! device check and the Starter show, in one file (F68).

/// The struct of every string and the English static, from one list:
/// each key's doc, then its value, so the two are read together. A key
/// whose doc is followed by `#[cfg(…)]` is in that build only: the strings
/// of what is compiled out of the app (the `unreleased` feature, F109).
macro_rules! strings {
    ($( $(#[doc = $doc:expr])* $(#[cfg($cfg:meta)])? $name:ident: $value:expr, )*) => {
        /// Every string Faraday shows, grouped by screen.
        pub struct Strings {
            $( $(#[doc = $doc])* $(#[cfg($cfg)])? pub $name: &'static str, )*
        }

        /// The English wording.
        pub static EN: Strings = Strings {
            $( $(#[cfg($cfg)])? $name: $value, )*
        };
    };
}

strings! {
    /// Power-off sheet's first line on a PC stick that started with Secure Boot off or unread.
    power_secure_boot_off: "Turn Secure Boot back on in the BIOS before starting Windows.",
    // ----- A vault's key and wallet, shown and saved (docs/BACKUP-FLOW.md §17.7), faraday-core -----
    /// A key record's SeedQR, shown only while held.
    vault_show_seedqr: "Show SeedQR",
    /// A vault record's button and the popup's title: its files to the
    /// Outbox, for the stick.
    vault_save_to_a_stick: "Save to a stick",
    /// The popup's line for a key record's files, before anything is made.
    vault_anyone_who_finds: "Anyone who finds this stick has this seed",
    // ----- Your own randomness (docs/OWN-RANDOMNESS.md §2), faraday-core -----
    /// The status line's first part. `{}`: the source the shell named, or `EN.create_device_os`.
    own_system: "System: {}",
    /// The status line. `{}`: `own_system`, then what was added or `own_none`.
    own_status: "{} · your own: {}",
    /// Nothing added yet, in the status line.
    own_none: "none",
    /// One count in the status line. `{}`: the number, then its noun.
    own_count: "{} {}",
    /// Status line noun, one.
    own_camera_frame: "camera frame",
    /// Status line noun, more than one.
    own_camera_frames: "camera frames",
    /// Status line noun, one.
    own_dice_roll: "dice roll",
    /// Status line noun, more than one.
    own_dice_rolls: "dice rolls",
    /// Status line noun, one.
    own_coin_flip: "coin flip",
    /// Status line noun, more than one.
    own_coin_flips: "coin flips",
    /// Status line noun, one.
    own_card: "card",
    /// Status line noun, more than one.
    own_cards: "cards",
    /// Status line noun, one.
    own_pointer_key: "pointer move or key",
    /// Status line noun, more than one.
    own_pointer_keys: "pointer moves and keys",
    /// Settings' row's title.
    own_randomness: "Randomness",
    /// The page's title, and the button on Settings and New key that opens it.
    own_add_randomness: "Add randomness",
    /// Source choice.
    own_source_camera: "Camera",
    /// Source choice.
    own_source_dice: "Dice",
    /// Source choice.
    own_source_coins: "Coins",
    /// Source choice.
    own_source_cards: "Cards",
    /// The batch not yet added. `{}`: how many, then `own_dice_rolls` or its kin.
    own_batch: "In this batch: {} {}",
    /// A dice batch of 99 rolls or more.
    own_enough_256: "Enough for 256 bits on its own",
    /// The button that adds the batch.
    own_add: "Add",
    /// Keyboard hint under the dice.
    own_dice_keys: "Type 1 to 6 · Backspace takes one back · Enter adds",
    /// Keyboard hint under the coins.
    own_coin_keys: "Type H or T · Backspace takes one back · Enter adds",
    /// Keyboard hint under the camera.
    own_camera_keys: "Space takes a picture",
    // ----- The stick's device check (docs/STARTER.md §3.1a), faraday-core -----
    /// Title of the sheet that names what is attached.
    devices_title: "Attached to this computer",
    /// What an IP-KVM or KVM can do, after its name and a colon.
    devices_kvm: "a KVM can show this screen and type on this computer from elsewhere",
    /// A wireless receiver attached (STARTER §3.1a): one line that holds
    /// nothing, on the Starter's Home and, on the stick, among the
    /// reminders before a secret.
    devices_receiver_notice: "Wireless receiver attached. A wireless keyboard's typing can be picked up nearby: use a wired keyboard or the on-screen keyboard.",
    /// A Logitech Bolt receiver attached: its keyboards' typing is
    /// encrypted. In place of the receiver line.
    devices_bolt_notice: "Logitech Bolt receiver attached. A Bolt keyboard's typing is encrypted.",
    /// What a virtual machine's host can do.
    devices_virtual_machine: "a virtual machine's host sees and can change everything in it",
    /// The name a virtual machine goes by when its maker is not known.
    devices_virtual_machine_unknown: "This computer",
    /// The line under a virtual machine: there is no way on.
    devices_real_computer: "Faraday must start on a real computer.",
    /// The name of a screen whose EDID gives no name.
    devices_this_screen: "This screen",
    /// What a TV can do.
    devices_tv: "a TV can record what it shows",
    /// A USB device that gives no name: `{}` is its vendor:product ID.
    devices_unnamed: "USB device {}",
    /// The button that reads what is attached again.
    devices_check_again: "Check again",
    /// The button that goes on with them attached.
    devices_continue: "Continue anyway",
    /// The line Continue anyway asks with.
    devices_continue_ask: "What is listed stays possible while they are attached.",
    /// The button that answers it.
    devices_continue_yes: "Yes, continue",
    /// Toast: Check again found them still attached.
    devices_still: "Still attached",
    /// The sheet's Power off, for a virtual machine.
    devices_power_off: "Power off",
    /// The reminder before a secret, on the stick, when the screen's
    /// EDID did not settle whether it may be a TV.
    devices_tv_line: "On a TV? Stop: a smart TV can record its screen.",
    /// Title of the sheet before a secret when it lists only reminders.
    devices_before: "Before a secret is shown",
    /// The button that goes on from the reminders.
    devices_understood: "I understand",
    /// The button that goes back from them.
    devices_cancel: "Cancel",
    /// Home, while the system has given no randomness well after the
    /// start: nothing is sealed and no keyboard gets a code.
    seed_missing: "No randomness from the system. Faraday has to restart",
    // ----- The Starter (docs/STARTER.md), faraday-starter -----
    /// Title of the Starter's Home, where the way back goes.
    starter_home: "Home",
    /// Label of Home's This computer card.
    starter_computer: "This computer",
    /// The card's name when the maker and model could not be read.
    starter_computer_unknown: "This computer",
    /// Label above Home's checklist.
    starter_checklist: "Checklist",
    /// Secure Boot state.
    starter_secure_boot_enabled: "Secure Boot: on",
    /// Secure Boot state.
    starter_secure_boot_off: "Secure Boot: off",
    /// Secure Boot state that could not be read.
    starter_secure_boot_unknown: "Secure Boot: not read",
    /// An ARM computer.
    starter_cannot_start_arm: "This computer cannot start a Faraday stick",
    /// A computer started in legacy BIOS mode.
    starter_cannot_start_bios: "Started in legacy BIOS mode: UEFI has to be on",
    /// A Mac with 32-bit EFI firmware (docs/MAC.md §1).
    starter_cannot_start_mac_firmware: "This Mac's firmware cannot start Faraday",
    /// A Mac with Apple silicon, until its image exists (docs/MAC.md §4).
    starter_cannot_start_mac_yet: "This Mac cannot start Faraday yet",
    /// Card line: a Mac with the T2 chip.
    starter_mac_t2: "Apple T2 Security Chip",
    /// Checklist item, card button and page, on an Intel Mac.
    starter_item_start_mac: "Start this Mac from the stick",
    /// Start this Mac's state: a Mac without the T2 chip.
    starter_state_mac_option: "Option key at power-on",
    /// Start this Mac's state: a Mac with the T2 chip.
    starter_state_mac_t2: "Startup Security, then the Option key",
    /// Start this Mac's state: a Mac the table does not know.
    starter_state_mac_unknown: "Option key at power-on · Startup Security on a T2 Mac",
    /// Over the T2 steps, for a Mac the table does not know.
    starter_mac_t2_only: "Only on a Mac with the T2 chip",
    /// Step.
    starter_mac_lower: "Lower Startup Security",
    /// Under it.
    starter_mac_recovery: "macOS Recovery: hold Command-R at power-on",
    /// Under it.
    starter_mac_utility_lower: "Utilities, Startup Security Utility: No Security, and Allow booting from external or removable media",
    /// Under Start from the stick, on a Mac.
    starter_mac_option: "Shut down. Hold Option at power-on and choose EFI Boot",
    /// Under it.
    starter_mac_firmware_password: "A firmware password, when one is set, is asked for first",
    /// Step.
    starter_mac_afterwards: "Afterwards: Startup Security back to Full",
    /// Under it.
    starter_mac_utility_full: "macOS Recovery, Startup Security Utility: Full Security, and Disallow booting from external or removable media",
    /// Guide title.
    starter_learn_mac_intel: "Starting an Intel Mac from the stick",
    /// Make a Faraday stick's first choice, once the payload carries both stick images (docs/MAC.md §6).
    starter_which_title: "For which computer?",
    /// Row.
    starter_which_pc: "A PC or an Intel Mac",
    /// Row.
    starter_which_apple: "A Mac with M1 or M2",
    /// Checklist item and its page.
    starter_item_make: "Make a Faraday stick",
    /// Checklist item and its page.
    starter_item_start: "Start this computer from the stick",
    /// Checklist item.
    starter_item_secure_boot: "Turn Secure Boot back on",
    /// Make a Faraday stick's state.
    starter_state_no_stick: "No Faraday stick attached",
    /// Make a Faraday stick's state: `{}` the version.
    starter_state_stick: "Faraday stick attached · {}",
    /// Home row.
    starter_row_pi: "Make a Pi card",
    /// Home row.
    starter_row_transfer: "Transfer",
    /// Home row.
    starter_row_desktop: "Open Faraday Demo",
    /// Home row and page.
    starter_row_install: "Install",
    /// Home row and page.
    starter_row_installed: "Installed",
    /// Home row and page, when the running copy is newer.
    starter_row_update: "Update the installed Faraday",
    /// Home row and page.
    starter_row_about: "About",
    /// The ⋮ menu's row and its sheet's title: Theme, Mode and Contrast
    /// (`docs/THEMES.md` §14).
    starter_row_appearance: "Appearance",
    /// Home row.
    starter_row_learn: "Learn",
    /// Rail and bar destination, and the title of its page.
    starter_nav_make: "Make",
    /// Rail and bar destination, and the title of its page.
    starter_nav_tools: "Tools",
    /// Top app bar button while the running copy is newer than the installed one.
    starter_bar_update: "Update",
    /// Make's first card.
    starter_card_stick: "Faraday stick",
    /// Make's second card.
    starter_card_pi: "Pi card",
    /// Make's Pi card, without the image in this Starter.
    starter_card_pi_none: "Not in this Starter",
    /// A card's line naming a version: `{}` the version.
    starter_card_version: "Version {}",
    /// Home's card for Faraday Demo.
    starter_card_desktop: "Faraday Demo",
    /// Home's update card's line: `{}` the installed version, then the running one.
    starter_card_update_line: "Installed {} · this copy {}",
    /// Home's card for the firmware's Faraday stick entry.
    starter_card_entry: "Boot entry",
    /// Tools' Transfer card's line.
    starter_card_transfer_line: "Send and Receive",
    /// Tools' Transfer card's button.
    starter_open_transfer: "Open Transfer",
    /// Tools' Blank templates card's line.
    starter_card_templates_line: "Words, SeedQR and codex32",
    /// Tools' Blank templates card's button.
    starter_open_templates: "Open blank templates",
    /// Title: which disk becomes the stick.
    starter_disks_stick: "Which stick?",
    /// Title: which card becomes the Pi card.
    starter_disks_card: "Which card?",
    /// The Pi card page: which Pi and screen.
    starter_pi_supported_boards: "For a Raspberry Pi 3B, 3B+ or Zero 2 W with the Waveshare 2.8 inch screen",
    /// The Pi card page: which cameras.
    starter_pi_cameras: "Cameras: a USB webcam, or Camera Module v1 or v2",
    /// A Starter with no stick image.
    starter_no_stick_image: "Faraday Starter carries no stick image",
    /// A Starter with no Pi image.
    starter_no_pi_image: "Faraday Starter carries no Pi image",
    /// A computer that cannot write a disk from here.
    starter_no_writer: "This computer cannot write a stick from here",
    /// Which stick? with none attached.
    starter_no_disks_stick: "No USB stick attached · 256 MB or larger",
    /// Which card? with none attached.
    starter_no_disks_card: "No SD card or USB stick attached · 256 MB or larger",
    /// A disk with no maker or model.
    starter_disk_unnamed: "Disk",
    /// What is on a disk: `{}` the version.
    starter_on_faraday: "Faraday stick · {}",
    /// A Faraday stick's version when it was not read: Windows lets
    /// only an administrator read the boot partition.
    starter_version_not_read: "version not read",
    /// What is on a disk.
    starter_on_empty: "Empty",
    /// One volume with no name.
    starter_on_volume: "1 volume",
    /// Volumes with no names: `{}` how many.
    starter_on_volumes: "{} volumes",
    /// After the volume's name.
    starter_volume_has_files: "files on it",
    /// After the volumes' names.
    starter_on_files_them: "files on them",
    /// After the volumes' names.
    starter_on_no_files: "no files",
    /// A disk the computer uses: `{}` the mount point.
    starter_refused_mounted: "Used by this computer: mounted at {}",
    /// A disk the computer uses.
    starter_refused_swap: "Used by this computer: swap",
    /// A disk the computer uses.
    starter_refused_system: "Holds this computer's system",
    /// A disk too small: `{}` the size needed.
    starter_refused_small: "Smaller than the image: needs {}",
    /// A size: `{}` the number.
    starter_size_gb: "{} GB",
    /// A size: `{}` the number.
    starter_size_mb: "{} MB",
    /// Title: what to write.
    starter_mode_title: "What to write?",
    /// Option.
    starter_mode_whole_stick: "New Faraday stick",
    /// Option.
    starter_mode_whole_card: "New Faraday card",
    /// Its second line.
    starter_mode_whole_sub: "Everything on it is lost",
    /// Option.
    starter_mode_keep: "Replace Faraday, keep data",
    /// Its second line.
    starter_mode_keep_sub: "Only the boot partition is written",
    /// The fact under it (docs/STARTER.md §4.4).
    starter_mode_keep_fact: "This computer can read and change the data partition while the stick is attached.",
    /// Keep data refused.
    starter_mode_same: "Already this build",
    /// Keep data ticked on a stick whose version was not read: the
    /// elevated copy reading it.
    starter_mode_reading: "Reading the stick's version",
    /// The whole stick refused for a boot partition image.
    starter_mode_boot_only: "The image is a boot partition alone",
    /// Button.
    starter_continue: "Continue",
    /// Title.
    starter_confirm_title: "Confirm",
    /// Confirm row.
    starter_stick: "Stick",
    /// Confirm row.
    starter_card: "Card",
    /// Confirm row.
    starter_size: "Size",
    /// Confirm row.
    starter_device: "Device",
    /// Confirm row.
    starter_serial: "Serial",
    /// A disk with no serial.
    starter_serial_none: "None",
    /// Confirm row.
    starter_disk_contents_row: "On it",
    /// Confirm row.
    starter_writes: "Writes",
    /// Confirm and About row.
    starter_build: "Build",
    /// An image with no build ID.
    starter_build_unknown: "Not known",
    /// Confirm row.
    starter_img_sha: "SHA-256 of the image",
    /// Confirm and About row for a developer's image.
    starter_image_sha: "SHA-256 of this image",
    /// About row for a developer's image.
    starter_image_file: "Image file",
    /// A developer's image (--image).
    starter_dev_image: "Not Faraday Starter's own image",
    /// Keep data over a newer stick: `{}` its version.
    starter_keep_newer: "This stick holds a newer Faraday: {}",
    /// Keep data writing a dev build.
    starter_keep_dev: "Writes a dev build, with a serial console",
    /// Confirm refused.
    starter_not_confirmed_stick: "Not the stick confirmed: it was removed or changed",
    /// Confirm refused.
    starter_not_confirmed_card: "Not the card confirmed: it was removed or changed",
    /// Button on Confirm when the disk is no longer the one confirmed.
    starter_choose_again: "Choose again",
    /// Button.
    starter_erase_and_write: "Erase and write",
    /// Button.
    starter_write: "Write",
    /// Title.
    starter_writing_title: "Writing",
    /// Progress: `{}` done, `{}` in all.
    starter_progress_bytes: "{} of {}",
    /// Progress while writing.
    starter_progress_writing: "Written {}",
    /// Progress while reading back.
    starter_reading_back: "Reading back · {}",
    /// Result title.
    starter_done_title: "Written",
    /// Result title.
    starter_failed_title: "Not written",
    /// Result line.
    starter_pull_stick: "Keep the stick in to start this computer from it. To start another computer, pull it out.",
    /// Result line.
    starter_pull_card: "The card can be pulled out.",
    /// Button.
    starter_done: "Done",
    /// Button.
    starter_try_again: "Try again",
    /// A guide whose Learn page is not there: `{}` its title.
    starter_guide_placeholder: "Learn: {}",
    /// Guide title.
    starter_learn_start_windows: "Starting a Windows computer from the stick",
    /// Guide title.
    starter_learn_start_linux: "Starting a Linux computer from the stick",
    /// Guide title.
    starter_learn_no_start: "When the stick does not start",
    /// Guide title.
    starter_learn_keep_data: "Replacing Faraday on a stick",
    /// Linux with a TPM-unlocked disk (docs/STARTER.md §6.4).
    starter_tpm_line: "A disk here is unlocked by the TPM: have its passphrase ready",
    /// Step and button.
    starter_restart: "Restart into the BIOS",
    /// Card button and the sheet's head (docs/STARTER.md §3.1).
    starter_restart_stick: "Restart from the stick",
    /// Button under the Start page's Start from the stick step, on
    /// Windows with Secure Boot off.
    starter_restart_choose_device: "Restart to choose a device",
    /// The restart sheets: the stick stays in for the start that follows.
    starter_restart_keep_stick: "Keep the stick in.",
    /// Restart from the stick's sheet on Windows: what Windows shows.
    starter_restart_windows_menu: "Windows shows Choose an option: Use a device, then the stick.",
    /// The restart's confirmation.
    starter_restart_save: "This restarts the computer now. Save your work first.",
    /// Card line, when the restart from the stick did not happen. `{}`
    /// the reason.
    starter_stick_restart_failed: "Restart from the stick did not happen: {}",
    /// Installed, after an uninstall that left the firmware's boot entry.
    /// `{}` the reason.
    starter_entry_kept: "The BIOS's Faraday stick entry remains: {}",
    /// Button.
    starter_restart_now: "Restart",
    /// Button.
    starter_cancel: "Cancel",
    /// Step.
    starter_secure_boot_turn_off: "Turn Secure Boot off",
    /// Step, when it is.
    starter_secure_boot_already_off: "Secure Boot is already off",
    /// Under it.
    starter_fast_boot: "Turn Fast Boot off where it hides USB",
    /// Step.
    starter_boot_from_stick: "Start from the stick",
    /// Under it.
    starter_boot_how: "The boot menu key",
    /// Under it, on Windows.
    starter_boot_how_windows: "The boot menu key, or Use a device in Advanced startup",
    /// Label (Done, and the Start page's boot-from-stick step): the
    /// expected boot menu entry's row.
    starter_boot_menu_label: "In the boot menu",
    /// Value for `starter_boot_menu_label`: `{}` the expected entry.
    starter_boot_menu_value: "something like {}",
    /// The boot menu entry's pattern for most makers (`decide::
    /// boot_menu_entry`): `{}` the stick's vendor and model.
    starter_boot_menu_default: "UEFI: {}, Partition 1",
    /// The boot menu entry's pattern for Lenovo's IdeaPad family and
    /// Framework: `{}` the stick's vendor and model.
    starter_boot_menu_efi_usb_device: "EFI USB Device ({})",
    /// The boot menu entry's pattern for HP: `{}` the stick's vendor and
    /// model.
    starter_boot_menu_hp: "USB Hard Drive - {}",
    /// Step.
    starter_afterwards: "Turn Secure Boot back on",
    /// Step, on Windows.
    starter_afterwards_windows: "Turn Secure Boot back on before starting Windows",
    /// Label above the guide's code.
    starter_on_a_phone: "On a phone",
    /// A key the maker table does not give, with the keys to try: `{}`
    /// the keys.
    starter_notes_try: "Not known: try {}",
    /// Label above the Start page's three things to write down (§3.4, F47).
    starter_notes_label: "Write down",
    /// Row: the maker's boot menu key.
    starter_notes_boot_key: "Boot menu key",
    /// Row: the maker's BIOS key.
    starter_notes_bios_key: "Usual BIOS key",
    /// Row: where the Secure Boot setting is in the maker's BIOS.
    starter_notes_setting: "Secure Boot setting",
    /// Row on an Intel Mac with the T2 chip: the key for macOS Recovery.
    starter_notes_mac_recovery: "macOS Recovery key",
    /// Row on an Intel Mac with the T2 chip: where Startup Security is.
    starter_notes_mac_setting: "Startup Security",
    /// The Mac's boot menu key.
    starter_notes_mac_boot_value: "Option, held",
    /// The Mac's key for macOS Recovery.
    starter_notes_mac_recovery_value: "Command-R, held",
    /// Where Startup Security is on a Mac.
    starter_notes_mac_setting_value: "macOS Recovery → Utilities → Startup Security Utility",
    /// A row's value for a maker without a page.
    starter_notes_not_known: "Not known",
    /// A row only a T2 Mac has, on a Mac the table does not know: `{}`
    /// the row's label.
    starter_notes_t2_only: "{}, only on a Mac with the T2 chip",
    /// Button: the sheet as a PDF, opened in the system's viewer.
    starter_notes_print: "Print",
    /// Tick box beside Print, on Windows with BitLocker on the system
    /// drive (§3.4, §6, F49): the sheet then carries the drive's recovery
    /// key. Unticked by default.
    starter_notes_include_key: "Include the BitLocker recovery key",
    /// The sheet's row for the recovery key: `{}` the drive (`C:`).
    starter_sheet_recovery_key: "BitLocker recovery key, drive {}",
    /// The sheet's line to circle the BIOS key that worked (F47).
    starter_sheet_bios_key_used: "BIOS key used (circle one)",
    /// The sheet's line to circle the boot menu key that worked (F47).
    starter_sheet_boot_key_used: "Boot menu key used (circle one)",
    /// The last choice of a circle line, with a line to write on.
    starter_sheet_other_key: "Other: ________",
    /// The Escape key's name as the maker table writes it: first on the
    /// circle lines, before the function keys.
    starter_sheet_key_esc: "Esc",
    /// The Start page's BitLocker row (§6, F49): `{}` the drive (`C:`).
    starter_bitlocker_row: "BitLocker, drive {}",
    /// Its value: protection on.
    starter_bitlocker_enabled: "On",
    /// Its value: suspended, with the times Windows starts before it turns
    /// back on: `{}` the number.
    starter_bitlocker_suspended_times: "Suspended · turns back on by itself after Windows starts {} times",
    /// The same, with one start left.
    starter_bitlocker_suspended_once: "Suspended · turns back on by itself after Windows starts once",
    /// Its value: suspended with no count (`DisableCount` 0).
    starter_bitlocker_suspended_until: "Suspended until resumed",
    /// Its value: suspended, the count not read (read without
    /// elevation).
    starter_bitlocker_suspended: "Suspended",
    /// Button: suspend BitLocker for the next starts of Windows.
    starter_bitlocker_suspend: "Temporarily suspend BitLocker",
    /// Under the button: `{}` the number of starts.
    starter_bitlocker_suspend_note: "For the next {} times Windows starts; then it turns back on by itself",
    /// Button: turn BitLocker back on now.
    starter_bitlocker_resume: "Resume BitLocker",
    /// While the elevation prompt is open.
    starter_waiting_permission: "Waiting for administrator permission",
    /// Under the Secure Boot step, and on the sheet, on Windows.
    starter_not_found_windows: "If the Secure Boot setting cannot be found: exit the BIOS without saving changes. Windows starts as before.",
    /// The same, on Linux.
    starter_not_found: "If the Secure Boot setting cannot be found: exit the BIOS without saving changes. The computer starts as before.",
    /// On the sheet, under Turn Secure Boot off.
    starter_sheet_off_how: "In the BIOS: the Secure Boot setting to Disabled, then save and exit",
    /// On the sheet, under Start from the stick, on Windows.
    starter_sheet_boot_how_windows: "The boot menu key, on the restart after saving, before Windows starts",
    /// The same, on Linux.
    starter_sheet_boot_how: "The boot menu key, on the restart after saving",
    /// On the sheet, under Turn Secure Boot back on.
    starter_sheet_back_how: "The BIOS key at power-on: the Secure Boot setting to Enabled, then save and exit",
    /// Button.
    starter_install_now: "Install",
    /// Button while installing.
    starter_installing: "Installing",
    /// Install done: `{}` where.
    starter_install_done_path: "Installed in {}",
    /// Row.
    starter_installed_version: "Installed version",
    /// Row.
    starter_this_version: "This version",
    /// Installed row.
    starter_installed_app: "Faraday Starter",
    /// Installed row.
    starter_installed_desktop: "Faraday Demo",
    /// Installed row.
    starter_installed_menu: "Menu entry",
    /// Installed row.
    starter_installed_desktop_menu: "Faraday Demo's menu entry",
    /// Installed row.
    starter_installed_icon: "Icon",
    /// Installed row.
    starter_installed_entry: "Installed apps entry",
    /// Installed with nothing installed.
    starter_not_installed: "Not installed",
    /// Button.
    starter_uninstall: "Uninstall",
    /// The uninstall's question.
    starter_uninstall_ask: "Uninstall Faraday Starter?",
    /// Under it.
    starter_uninstall_keeps: "Removes the files listed. Received files and settings stay.",
    /// Reason.
    starter_why_not_carried: "Faraday Starter does not carry what was confirmed. Nothing was written.",
    /// Reason. `{}` device, `{}` serial, `{}` size.
    starter_why_changed: "{} is no longer the disk that was confirmed (serial {}, {}). Nothing was written.",
    /// Reason. The serial of a disk that reports none.
    starter_why_no_serial: "none",
    /// Reason. A size in bytes.
    starter_why_bytes: "{} bytes",
    /// Reason. `{}` mount point.
    starter_why_mounted: "A partition on it is mounted at {}. Nothing was written.",
    /// Reason.
    starter_why_swap: "A partition on it is in use as swap. Nothing was written.",
    /// Reason.
    starter_why_system: "It holds this computer's system. Nothing was written.",
    /// Reason.
    starter_why_boot_only: "A boot partition alone is written only over a Faraday stick, keeping its data.",
    /// Reason. `{}` disk size, `{}` image size.
    starter_why_too_small: "The disk ({}) is smaller than the image ({}). Nothing was written.",
    /// Reason.
    starter_why_payload_sum: "The payload file does not match the manifest. Nothing was written.",
    /// Reason. `{}` file, `{}` sum.
    starter_why_image_changed: "{} has changed since the app started: its SHA-256 is now {}. Nothing was written.",
    /// Reason.
    starter_why_gone: "The disk is no longer attached. Nothing was written.",
    /// Reason. `{}` the system's words.
    starter_why_bus: "udisks2 did not answer: {}",
    /// Reason. `{}` the system's words.
    starter_why_unmounting: "Unmounting it failed: {}. Nothing was written.",
    /// Reason. `{}` the system's words.
    starter_why_open_write: "Opening it to write failed: {}. Nothing was written.",
    /// Reason. `{}` the system's words.
    starter_why_open_read: "Opening it to read back failed: {}",
    /// Reason. `{}` device, `{}` the system's words.
    starter_why_device: "{}: {}",
    /// Reason. `{}` device, `{}` the system's words.
    starter_why_flush: "Flushing {} failed: {}",
    /// Reason. `{}` device, `{}` the fault.
    starter_why_not_stick: "{} is not a Faraday stick: {}. Nothing was written.",
    /// Reason. `{}` the fault.
    starter_why_not_stick_image: "The image is not a Faraday stick image: {}",
    /// Reason. `{}` sizes.
    starter_why_boot_larger: "The image's boot partition ({}) is larger than the stick's ({})",
    /// Reason.
    starter_why_boot_label: "The image's boot partition is not a FAT32 filesystem labelled FARADAYBOOT",
    /// Reason.
    starter_why_boot_short: "The boot partition image is shorter than one sector",
    /// Reason. `{}` device.
    starter_why_table_changed: "The partition table on {} changed",
    /// Reason. `{}` the system's words.
    starter_why_reading_table: "Reading the partition table failed: {}",
    /// Reason. `{}` device, `{}` byte.
    starter_why_back_ends: "Reading back {}: the disk ends at byte {}",
    /// Reason. `{}` device, `{}` byte.
    starter_why_back_differs: "Reading back {}: what was read differs from what was written at byte {}",
    /// Reason. `{}` device, `{}` the system's words.
    starter_why_back_failed: "Reading back {} failed: {}",
    /// Reason. `{}` the system's words.
    starter_why_source_read: "Reading the source failed: {}",
    /// Reason. `{}` the system's words.
    starter_why_write_failed: "Writing failed: {}",
    /// Reason.
    starter_why_payload_changed: "The payload file changed while it was read",
    /// Reason. `{}` the decoder's words.
    starter_why_damaged: "The .xz file is damaged: {}",
    /// Reason. `{}` bytes.
    starter_why_longer: "The decompressed image is longer than {} bytes",
    /// Reason. `{}` bytes.
    starter_why_shorter: "The decompressed image is {} bytes, not {}",
    /// Reason. `{}` sum.
    starter_why_wrong_sum: "The decompressed image's SHA-256 is {}, not the expected one",
    /// Reason.
    starter_why_no_home: "The home folder is not known",
    /// Reason.
    starter_why_no_desktop: "Faraday Starter carries no Faraday Demo for this computer",
    /// Reason. `{}` sum.
    starter_why_cache_sum: "Faraday Demo does not match the manifest: its SHA-256 is {}",
    /// Reason. `{}` path, `{}` the system's words.
    starter_why_file: "{}: {}",
    /// Reason. `{}` path of a file that is another program's.
    starter_why_in_the_way: "{} is another program's, and is in the way",
    /// Reason. `{}` program, `{}` the system's words.
    starter_why_start: "Starting {} failed: {}",
    /// Reason. `{}` the system's words.
    starter_why_restart: "Restarting into the BIOS failed: {}",
    /// Reason: the administrator permission was declined.
    starter_why_declined: "Administrator permission was declined. Nothing was changed.",
    /// Reason. `{}` a program's name.
    starter_why_missing: "{} is not installed",
    /// Reason. `{}` efibootmgr's words.
    starter_why_boot_entry: "efibootmgr failed: {}",
    /// Reason.
    starter_why_no_stick: "No Faraday stick is attached",
    /// The elevated writer found this build already on the stick.
    starter_why_same_build: "The stick already holds this build. Nothing was written.",
    /// Reason: declined on an account that is not an administrator's.
    starter_why_no_admin: "An administrator of this computer has to do this step.",
    /// Reason: BitLocker refused to suspend: `{}` its return value.
    starter_why_managed: "BitLocker was not suspended ({}). This computer's BitLocker is managed: its administrator has to suspend it.",
    /// Reason: a BitLocker call failed: `{}` the call, `{}` its code.
    starter_why_bitlocker: "BitLocker refused {}: {}",
    /// Reason: the system drive is not encrypted.
    starter_why_not_encrypted: "The drive is not encrypted.",
    /// Reason: BitLocker on the system drive could not be read.
    starter_why_not_read: "BitLocker on the system drive could not be read.",
    /// Reason: the system drive has no recovery key (no numerical
    /// password protector).
    starter_why_no_recovery_key: "The drive has no BitLocker recovery key.",
    /// Reason.
    starter_why_unsupported: "Not on this computer",
    /// The app menu entry's name, in the desktop entry file install writes.
    starter_menu_name: "Faraday Starter",
    /// The desktop app's menu entry's name.
    starter_menu_desktop_name: "Faraday Demo",
    /// The desktop app's menu entry's description.
    starter_menu_desktop_comment: "Practice, no Mainnet",
    /// The publisher in the Starter's installed-apps entry and the `.exe`'s properties.
    starter_publisher: "Faraday",
    /// Fault. `{}` the system's words.
    starter_fault_reading: "reading the partition table failed: {}",
    /// Fault.
    starter_fault_no_header: "it has no GPT header",
    /// Fault.
    starter_fault_not_gpt: "its partition table is not GPT",
    /// Fault.
    starter_fault_header_damaged: "its GPT header is damaged",
    /// Fault.
    starter_fault_header_checksum: "its GPT header's checksum is wrong",
    /// Fault.
    starter_fault_entries_damaged: "its GPT entries are damaged",
    /// Fault.
    starter_fault_entries_past_end: "its GPT entries lie past the end of the disk",
    /// Fault.
    starter_fault_entries_checksum: "its GPT entries' checksum is wrong",
    /// Fault.
    starter_fault_disk_uuid: "its disk UUID is not a Faraday stick's",
    /// Fault. `{}` count.
    starter_fault_partitions: "it has {} partitions, not two",
    /// Fault.
    starter_fault_boot_name: "its first partition is not named FARADAYBOOT",
    /// Fault.
    starter_fault_boot_type: "its first partition is not a Faraday EFI system partition",
    /// Fault.
    starter_fault_data_type: "its second partition is not a Faraday FARADAYDATA partition",
    /// Fault.
    starter_fault_boot_damaged: "its FARADAYBOOT partition is damaged",
    /// Fault. `{}` sizes.
    starter_fault_boot_small: "its FARADAYBOOT partition ({}) is smaller than the image ({})",
    /// Fault.
    starter_fault_overlap: "its partitions overlap",
    /// Fault.
    starter_fault_boot_past_end: "its boot partition runs past its end",
    /// About row.
    starter_version: "Version",
    /// About row.
    starter_release: "Release",
    /// About row.
    starter_commit: "Commit",
    /// About label.
    starter_payload: "Carried in this app",
    /// About, with no payload.
    starter_no_payload: "Faraday Starter carries no images",
    /// About row.
    starter_sha_xz: "SHA-256",
    /// About row: `{}` the file inside.
    starter_sha_inner: "SHA-256 of {}",
    /// About label.
    starter_release_key: "Release key",
    /// About label.
    starter_check_title: "Checking this file",
    /// About.
    starter_check_text: "Check this file against SHA256SUMS and its signature, SHA256SUMS.asc, from the release, with the release key above.",
    /// About: a command.
    starter_check_cmd_import: "gpg --fingerprint F1536BE07E234CFD",
    /// About: a command.
    starter_check_cmd_verify: "gpg --verify SHA256SUMS.asc SHA256SUMS",
    /// About: a command.
    starter_check_cmd_sums: "sha256sum -c --ignore-missing SHA256SUMS",
    /// About, on Windows.
    starter_check_windows: "On Windows: Get-FileHash -Algorithm SHA256 for the sum, Gpg4win for the signature.",
    /// About label, on Windows.
    starter_unsigned_title: "Not code-signed",
    /// About, on Windows.
    starter_unsigned_text: "A code-signing certificate names a verified person or company, and Faraday is published under a pseudonym. Windows warns on first run; More info, then Run anyway, opens it. The sums and the signature are the check.",
    /// Home row and page title.
    starter_row_templates: "Blank templates",
    /// Row and Choice title.
    starter_tpl_words: "Words",
    /// Row and Choice title.
    starter_tpl_copy: "Copy",
    /// Row and Choice title.
    starter_tpl_places: "Places",
    /// Copy choice.
    starter_tpl_seedqr: "SeedQR",
    /// Copy choice.
    starter_tpl_both: "Both",
    /// Primary button.
    starter_tpl_print: "Print",
    /// Secondary button.
    starter_tpl_save: "Save the PDF",
    /// After a save.
    starter_tpl_open: "Open it",
    /// Where the PDF was saved.
    starter_tpl_saved: "Saved {}",
    /// Places stepper.
    starter_tpl_fewer: "Fewer",
    /// Places stepper.
    starter_tpl_more: "More",
    /// Blank templates' Form row (codex32 batch).
    starter_tpl_form: "Form",
    /// Form: BIP-39 words.
    starter_tpl_form_words: "BIP-39 words",
    /// Form: codex32.
    starter_tpl_form_codex32: "codex32",
    /// Codex32 length row.
    starter_tpl_length: "Length",
    /// A codex32 length. `{}`: 48, 74 or 127.
    starter_tpl_chars: "{} characters",
    /// Codex32 copy: the string alone.
    starter_tpl_string: "String",
    /// Codex32 copy: the Codex32QR grid.
    starter_tpl_codex32qr: "Codex32QR",
    /// Codex32 copy: a page of four smaller Codex32QR grids.
    starter_tpl_grids: "Four Codex32QR grids",
    /// Codex32 worksheets row.
    starter_tpl_worksheets: "Worksheets",
    /// Worksheets: none.
    starter_tpl_ws_none: "None",
    /// Worksheets: dice, checksum and translation.
    starter_tpl_ws_all: "Dice, checksum and translation",
    /// Another encryption product's driver found, by name (STARTER §6.4).
    starter_other_found: "An encrypted drive Faraday does not manage: {}",
    /// The person has that product's rescue disk or password.
    starter_other_confirm: "I have its rescue disk or password",
    /// A screen read as a TV (STARTER §3.1a), by name.
    starter_concern_tv: "{} is a TV. A TV can record what it shows: use another screen or computer.",
    /// A KVM switch or IP-KVM attached (STARTER §3.1a), by name.
    starter_concern_kvm: "{} is a KVM switch. It can capture the screen and type: connect the screen and keyboard directly.",
    /// The Starter runs in a virtual machine (STARTER §3.1a), by its name.
    starter_concern_vm: "{} is a virtual machine. Faraday must start on the real computer, and this app cannot see the host's screens and devices.",
    /// The name of a screen whose EDID has none.
    starter_concern_screen: "The screen",
    /// The name of a USB device with no strings.
    starter_concern_device: "A USB device",
    /// The name of a virtual machine whose firmware names none.
    starter_concern_computer: "This computer",
    /// Reads what is attached again (STARTER §3.1a).
    starter_check_again: "Check again",
    /// Lets one item of concern through for the session (STARTER §3.1a).
    starter_continue_anyway: "Continue anyway",
    /// Continue anyway's sheet, by the item's name.
    starter_let_through_ask: "Continue with {} attached?",
    /// Continue anyway's sheet: how long it lasts.
    starter_let_through_until: "Until this app is closed.",
    /// The link to the Learn page of that name (STARTER §3.1b).
    starter_learn_secure_boot_bitlocker: "What Secure Boot and BitLocker are",
    /// The link to the Learn page on screens that record (STARTER §3.1a).
    starter_learn_screens: "Screens and devices that can record",
    /// The Learn page on the management processors under the OS.
    starter_learn_what_else_runs: "What else runs on this computer",
    /// The link on Restart from the stick's sheet (STARTER §3.1c).
    starter_learn_boot_entry: "About the boot entry",
    /// Home's row removing the firmware's Faraday stick entry (STARTER §3.1c).
    starter_remove_entry: "Remove the Faraday boot entry",
    /// That row's second line.
    starter_remove_entry_sub: "In this computer's boot list",
    /// That row while the removal runs.
    starter_removing_entry: "Removing",
    // ----- backup (faraday-core/src/wallet_backup.rs) -----
    /// Text in `copy_scan_line`. `{}`: `k`.
    backup_your_copy_scans_but: "Your copy scans but word {} differs",
    /// Text in `copy_scan_line`.
    backup_your_copy_scans: "Your copy scans and matches the seed",
    /// Text in `envelope`. `{}`: `i + 1`, `p.len()`.
    backup_envelope_share_count: "share {} of {}",
    /// Text in `envelope`.
    backup_the_backup_sheet: "the backup sheet",
    /// Text in `envelope`. `{}`: `i + 1`, `i + 1`, `fp`, `sheet`.
    backup_envelope_line: "Envelope {}: the seed of key {} {}, with {}",
    // ----- backups (faraday-core/src/backups/mod.rs) -----
    /// Text in `backups`.
    backups_wallet: "Wallet",
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
    /// Text in `backup_line`. `{}`: `parts.join(" · ")`.
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
    // ----- bip85 (faraday-core/src/bip85/mod.rs) -----
    /// Toast in `bip85_open`.
    bip85_load_a_key_first: "Load a key first",
    /// Text in `bip85_title`. `{}`: `app_name(b.app)`, `fp`, `b.index().unwrap_or(0)`.
    bip85_bip_85_index: "BIP-85 {} · {} · index {}",
    /// Toast in `bip85_to_vault`.
    bip85_no_vault_is_open: "No vault is open",
    /// Text in `bip85_to_vault`. `{}`: `title`.
    bip85_is_in_the_vault: "{} is in the vault",
    /// Toast in `bip85_load`. `{}`: `self.medium.noun()`.
    bip85_remove_the_first: "Remove the {} first",
    /// Toast in `bip85_load`. `{}`: `fp_text(fp)`.
    bip85_key_added: "Key {} added",
    /// Text in `bip85_out`.
    bip85_a_child_seed: "A child seed's words",
    /// Text in `bip85_out`.
    bip85_a_private_key: "A private key",
    /// Text in `bip85_out`.
    bip85_secret_bytes: "Secret bytes",
    /// Text in `bip85_out`.
    bip85_a_password: "A password",
    /// Text in `bip85_out`.
    bip85_secret_warning: "Whoever has it has what it protects or spends",
    // ----- bip85 (faraday-core/src/bip85/screen.rs) -----
    /// Text in `TITLES`.
    bip85_key: "Key",
    /// Text in `TITLES`.
    bip85_application: "Application",
    /// Text in `TITLES`.
    bip85_index: "Index",
    /// Text in `TITLES`.
    bip85_value: "Value",
    /// Text in `draw`.
    bip85_wallets: "Wallets",
    /// Text in `draw`.
    bip85_derive_with_bip: "Derive with BIP-85",
    /// Text in `length_label`. `{}`: `b.length`.
    bip85_words: "{} words",
    /// Text in `length_label`. `{}`: `b.length`.
    bip85_bytes: "{} bytes",
    /// Text in `length_label`. `{}`: `b.length`.
    bip85_characters: "{} characters",
    /// Text in `summary`.
    bip85_not_chosen: "Not chosen",
    /// Text in `guide`.
    bip85_bip_85_turns: "Choose the key to derive from. Each value comes back from the same key, application and index, so this key's backup covers them all.",
    /// Text in `guide`.
    bip85_guide_child_seed: "A child seed is a key of its own, for another wallet or another device. It reveals nothing about this key.",
    /// Text in `guide`.
    bip85_guide_password: "A password for a website or a program, as long as it allows. Keep it in a vault entry; the index makes the next one.",
    /// Text in `guide`.
    bip85_the_value_is_made: "The value is made the way BIP-85 specifies, so any implementation gives the same one from the same key.",
    /// Text in `guide`.
    bip85_guide_index: "Index 0 is the first of its kind; each number gives another. Write down the index with what you use it for: it is all that is needed to make the value again.",
    /// Text in `guide`.
    bip85_guide_value: "The value is a secret. It goes into the open vault, sealed under its passphrase; out of the vault it goes only after the warning.",
    /// Text in `card`. `{}`: `lo`, `hi`.
    bip85_index_range: "{} to {}",
    /// Line in `card`.
    bip85_index_limits: "An index is 0 to 2147483647",
    /// Line in `card`.
    bip85_type_the_digits: "Type the digits · Enter continues",
    /// Button in `card`.
    bip85_continue: "Continue",
    /// Line in `card`.
    bip85_path: "Path",
    /// Text in `card`.
    bip85_hide: "Hide",
    /// Text in `card`.
    bip85_show: "Show",
    /// Text in `card`. `{}`: `v.name`.
    bip85_save_into: "Save into {}",
    /// Text in `card`.
    bip85_load_as_a_key: "Load as a key",
    /// Text in `card`.
    bip85_out_unprotected: "Out unprotected…",
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
    boot_import_no_files: "No files read",
    /// Note in `import_codes`.
    boot_import_not_a_psbt: "Not a PSBT, a descriptor, an xpub or a signed message",
    /// Text in `import_codes`.
    boot_import_no_qr_code: "No QR code found",
    /// Text in `import_codes`.
    boot_import_its_codes: "Its codes are not complete",
    /// Text in `import_codes`. `{}`: `n`, `if n == 1 { EN.boot_import_code } els…`, `kinds.join(", ")`.
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
    /// Text in `import_go`. `{}`: `count(wallets, EN.boot_import_wallet_inline,…`, `count(keys, EN.boot_import_key, EN.bo…`, `count(moved, EN.boot_import_file, EN.…`.
    boot_import_imported: "Imported {} · {} · {}",
    /// Text in `import_go`.
    boot_import_wallet_inline: "wallet",
    /// Text in `import_go`.
    boot_import_wallets_inline: "wallets",
    /// Text in `import_go`.
    boot_import_key: "key",
    /// Text in `import_go`.
    boot_import_keys: "keys",
    /// Text in `import_go`.
    boot_import_file: "file",
    /// Text in `import_go`.
    boot_import_files_inline: "files",
    // ----- boot_import (faraday-core/src/boot_import/screen.rs) -----
    /// Text in `body`. `{}`: `label`.
    boot_import_read_source: "Read from {}",
    /// Text in `body`. `{}`: `app.medium.noun()`.
    boot_import_pull_medium: "Pull the {} to continue",
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
    // ----- catalog (faraday-core/src/catalog.rs) -----
    /// Text in `TILES`.
    catalog_add_a_key: "Add a key",
    /// Text in `TILES`.
    catalog_make_or_type: "Make one from dice, coins, cards, hex, camera or a mix, or type its words",
    /// Text in `TILES`.
    catalog_scan_a_seedqr: "Scan a SeedQR",
    /// Text in `TILES`.
    catalog_scan_seedqr_line: "Standard or compact",
    /// Text in `TILES`.
    catalog_slip_39_shares: "SLIP-39 shares",
    /// Text in `TILES`.
    catalog_a_new_key_dealt: "A new key dealt as m of n shares",
    /// Text in `TILES`.
    catalog_restore_slip39: "Restore from SLIP-39",
    /// Text in `TILES`.
    catalog_shares_typed: "Shares typed or scanned back into a key",
    /// Text in `TILES`.
    catalog_codex32_line: "A secret or its shares typed into a key",
    /// Text in `TILES`.
    catalog_seed_xor: "Seed XOR",
    /// Text in `TILES`.
    catalog_parts_combined: "Parts combined into a key",
    /// Text in `TILES`.
    catalog_explore_a_key: "Explore a key",
    /// Text in `TILES`.
    catalog_explore_key_line: "Its xpubs and addresses at any path",
    /// Text in `TILES`.
    catalog_child_seeds: "Child seeds and passwords",
    /// Text in `TILES`.
    catalog_child_seeds_line: "Derived from a key here",
    /// Text in `TILES`.
    catalog_vanity_address: "Vanity address",
    /// Text in `TILES`.
    catalog_a_first_address: "A first address that begins your way",
    /// Text in `TILES`.
    catalog_silent_payments: "Silent payments",
    /// Text in `TILES`.
    catalog_one_address: "One address to give out",
    /// Text in `TILES`.
    catalog_keys_and_seeds: "Keys and seeds",
    /// Text in `TILES`.
    catalog_lightning_node: "Lightning node key",
    /// Text in `TILES`.
    catalog_from_a_key: "From a key here, or an LND aezeed",
    /// Text in `TILES`.
    catalog_single_key: "Single-key wallet",
    /// Text in `TILES`.
    catalog_native_segwit: "Native SegWit, Taproot, nested or legacy",
    /// Text in `TILES`.
    catalog_multisig: "Multisig",
    /// Text in `TILES`.
    catalog_k_of_n_keys: "k of n keys, a quorum signs",
    /// Text in `TILES`.
    catalog_taproot_multisig: "Taproot multisig",
    /// Text in `TILES`.
    catalog_k_of_n_in_one: "k of n in one script leaf",
    /// Text in `TILES`.
    catalog_all_keys_one: "All keys, one signature on chain",
    /// Text in `TILES`.
    #[cfg(feature = "unreleased")]
    catalog_frost_threshold: "FROST threshold",
    /// Text in `TILES`.
    #[cfg(feature = "unreleased")]
    catalog_any_m_of_n: "Any m of n shares, one signature on chain",
    /// Text in `TILES`.
    catalog_miniscript_inheritance: "Miniscript and inheritance",
    /// Text in `TILES`.
    catalog_miniscript_line: "Load one from its descriptor",
    /// Text in `TILES`.
    catalog_restore_wallet: "Restore an existing wallet",
    /// Text in `TILES`.
    catalog_descriptor_wallet: "Descriptor, wallet file, BIP 129 record or split sheets",
    /// Text in `TILES`.
    catalog_wallets: "Wallets",
    /// Text in `TILES`.
    catalog_back_up_a_wallet: "Back up a wallet",
    /// Text in `TILES`.
    catalog_seeds_by_hand: "Seeds by hand, sheets, envelopes, codex32 shares",
    /// Text in `TILES`.
    catalog_sign_a_transaction: "Sign a transaction",
    /// Text in `TILES`.
    catalog_sign_transaction_line: "A PSBT from Files or a QR code",
    /// Text in `TILES`.
    catalog_nonce_check: "Nonce check",
    /// Text in `TILES`.
    catalog_every_signature: "Every signature recomputed and checked",
    /// Text in `TILES`.
    catalog_decode_a_transaction: "Decode a transaction",
    /// Text in `TILES`.
    catalog_what_it_spends: "What it spends and pays, field by field",
    /// Text in `TILES`.
    catalog_transactions: "Transactions",
    /// Text in `TILES`.
    catalog_check_an_address: "Check an address",
    /// Text in `TILES`.
    catalog_check_address_line: "Scan it; the loaded wallets are searched",
    /// Text in `TILES`.
    catalog_sign_a_message: "Sign a message",
    /// Text in `TILES`.
    catalog_with_a_single: "With a single-key wallet's key here",
    /// Text in `TILES`.
    catalog_messages: "Messages",
    /// Text in `TILES`.
    catalog_check_a_signed: "Check a signed message",
    /// Text in `TILES`.
    catalog_check_signed_line: "Against its address",
    /// Text in `TILES`.
    catalog_vaults: "Vaults",
    /// Text in `TILES`.
    catalog_keys_wallets: "Keys, wallets, entries and notes under passphrases",
    /// Text in `TILES`.
    catalog_totp_codes: "TOTP codes",
    /// Text in `TILES`.
    catalog_totp_codes_line: "Codes from the entries in open vaults",
    /// Text in `TILES`.
    catalog_diceware_passphrase: "Diceware passphrase",
    /// Text in `TILES`.
    catalog_diceware_line: "Rolled with dice in Create a vault",
    /// Text in `TILES`.
    catalog_gpg_key: "GPG key",
    /// Text in `TILES`.
    catalog_make_sign_revoke: "Make, sign, revoke, paperkey",
    /// Text in `TILES`.
    catalog_secure_boot: "Secure Boot keys",
    /// Text in `TILES`.
    catalog_secure_boot_line: "PK, KEK and db; sign an EFI image",
    /// Text in `TILES`.
    catalog_keepass_export: "KeePass export",
    /// Text in `TILES`.
    catalog_a_vault_entry: "A vault entry as a sealed database",
    /// Text in `TILES`.
    catalog_vaults_group: "Vaults and other keys",
    /// Text in `TILES`.
    catalog_hashes: "Hashes",
    /// Text in `TILES`.
    catalog_sha_256_sha: "SHA-256, SHA-256d and HASH160",
    /// Text in `TILES`.
    catalog_encodings: "Encodings",
    /// Text in `TILES`.
    catalog_what_a_string: "What a string is encoded in, and its value",
    /// Text in `TILES`.
    catalog_descriptor: "Descriptor checksum",
    /// Text in `TILES`.
    catalog_a_descriptor: "A descriptor's checksum, and whether it holds",
    /// Text in `TILES`.
    catalog_convert_xpub: "Convert an xpub",
    /// Text in `TILES`.
    catalog_every_spelling: "Every spelling of one extended key",
    /// Text in `TILES`.
    catalog_units: "Units",
    /// Text in `TILES`.
    catalog_one_amount: "One amount in all four units",
    /// Text in `TILES`.
    catalog_calculators: "Calculators",
    /// Text in `TILES`.
    catalog_miniscript: "Miniscript compiler",
    /// Text in `TILES`.
    catalog_a_policy_compiled: "A policy compiled into a descriptor",
    /// Text in `TILES`.
    catalog_bip_39_words: "BIP-39 words",
    /// Text in `TILES`.
    catalog_2048_words: "2048 words, each with its number and its 11 bits",
    /// Text in `TILES`.
    catalog_eff_large_list: "EFF large list",
    /// Text in `TILES`.
    catalog_7776_words: "7776 words, each with its number and its five dice",
    /// Text in `TILES`.
    catalog_eff_short_list_1: "EFF short list 1",
    /// Text in `TILES`.
    catalog_1296_words_each: "1296 words, each with its number and its four dice",
    /// Text in `TILES`.
    catalog_word_lists: "Word lists",
    /// Text in `TILES`.
    catalog_eff_short_list_2: "EFF short list 2",
    /// Text in `TILES`.
    catalog_1296_words: "1296 words with unique three-letter starts, each with its number and its four dice",
    /// Text in `TILES`.
    catalog_qr: "QR",
    /// Text in `TILES`.
    catalog_scan_a_qr_code: "Scan a QR code",
    /// Text in `TILES`.
    catalog_single_or_animated: "Single or animated, into Files",
    /// Text in `tile_need`.
    catalog_load_a_key: "Load a key first",
    /// Text in `tile_need`.
    catalog_at_the_end: "At the end of signing",
    /// Text in `tile_need`.
    catalog_load_a_wallet: "Load a wallet first",
    /// Text in `tile_need`.
    catalog_needs_a_single: "Needs a single-key wallet with its key here",
    /// Text in `tile_need`.
    catalog_copy_a_signed: "Copy a signed message in first",
    /// Text in `tile_need`.
    catalog_make_a_vault: "Make a vault first",
    /// Text in `tile_need`.
    catalog_unlock_a_vault: "Unlock a vault first",
    // ----- chart_edit (faraday-core/src/chart_edit.rs) -----
    /// Toast in `chart_notes_change`.
    chart_edit_that_does: "That does not fit in this vault's slot size",
    /// Text in `edit_title`.
    chart_edit_a_new_place: "a new place",
    /// Text in `edit_title`. `{}`: `p + 1`.
    chart_edit_place: "Place {}",
    /// Text in `edit_title`. `{}`: `v + 1`.
    chart_edit_vault: "Vault {}",
    /// Text in `edit_title`. `{}`: `self.medium.cap()`.
    chart_edit_of_files: "{} of files",
    /// Text in `edit_title`.
    chart_edit_watch_only: "Watch-only software",
    /// Text in `edit_title`. `{}`: `glance::thing_label(g.as_ref(), what)`, `spot(at)`.
    chart_edit_save_into: "Save {} into {}",
    /// Text in `edit_title`. `{}`: `glance::thing_label(g.as_ref(), what)`.
    chart_edit_as_a_file: "{} as a file",
    /// Text in `edit_title`. `{}`: `glance::thing_label(g.as_ref(), what)`, `spot(at)`.
    chart_edit_place_title: "{} at {}",
    /// Text in `edit_title`. `{}`: `label(from, what)`, `spot(to)`.
    chart_edit_move_title: "Move {} to {}",
    /// Text in `edit_title`. `{}`: `place(p)`.
    chart_edit_remove: "Remove {}",
    /// Text in `edit_title`. `{}`: `spot(at)`.
    chart_edit_off_plan: "{} off the plan",
    /// Toast in `chart_key_checked`.
    chart_edit_checked: "Checked today",
    /// Text in `envelopes_out`. `{}`: `v + 1`.
    chart_edit_vault_stick: "Vault {} stick",
    // ----- codes (faraday-core/src/codes/mod.rs) -----
    /// Text in `codes_rows`.
    codes_entry: "Entry",
    // ----- codes (faraday-core/src/codes/screen.rs) -----
    /// Text in `now_row`. `{}`: `crate::vault::tz_offset_text(app.tz_…`.
    codes_now_utc: "Now · UTC{}",
    /// Text in `now_row`.
    codes_not_known: "Not known",
    /// Button in `now_row`.
    codes_set_the_time: "Set the time",
    /// Text in `code_row`.
    codes_unreadable: "Unreadable",
    /// Title in `draw`.
    codes_totp_codes: "TOTP codes",
    /// Line in `compact`.
    codes_no_totp_entries: "No entries with a TOTP secret",
    /// Button in `compact`.
    codes_scan_a_setup: "Scan a setup code",
    /// Button in `compact`.
    codes_add_an_entry: "Add an entry",
    // ----- compact (faraday-core/src/compact/mod.rs) -----
    /// Text in `screen_name`.
    compact_spend: "Spend",
    /// Text in `screen_name`.
    compact_add_a_key: "Add a key",
    /// Text in `screen_name`.
    compact_back_up: "Back up",
    /// Text in `screen_name`.
    compact_sign_a_message: "Sign a message",
    /// Text in `screen_name`.
    compact_check_a_message: "Check a message",
    /// Text in `screen_name`.
    compact_make_a_new: "Make a new wallet",
    /// Text in `screen_name`.
    compact_restore_wallet: "Restore an existing wallet",
    /// Text in `screen_name`.
    compact_set_the_time: "Set the time",
    /// Text in `screen_name`.
    compact_totp_codes: "TOTP codes",
    /// Text in `screen_name`.
    compact_create_a_vault: "Create a vault",
    /// Text in `screen_name`.
    compact_new_key: "New key",
    /// Text in `screen_name`.
    compact_bip_85: "BIP-85",
    /// Text in `screen_name`.
    compact_silent_payments: "Silent payments",
    /// Text in `screen_name`.
    compact_explore_a_key: "Explore a key",
    /// Text in `screen_name`.
    compact_lightning: "Lightning",
    /// Text in `screen_name`.
    compact_vanity_address: "Vanity address",
    /// Text in `screen_name`.
    compact_transaction: "Transaction",
    /// Text in `page`.
    compact_at_a_glance: "At a glance",
    /// Text in `page`.
    compact_vault_contents: "Vault contents",
    /// Text in `page`.
    compact_home: "Home",
    /// Text in `home`.
    compact_nothing_loaded: "Nothing loaded",
    /// Text in `home`.
    compact_key_inline: "key",
    /// Text in `home`.
    compact_keys: "keys",
    /// Text in `home`.
    compact_wallet: "wallet",
    /// Text in `home`.
    compact_wallets_inline: "wallets",
    /// Text in `home`. `{}`: `v`, `if v == 1 { EN.compact_vault } else {…`.
    compact_vaults_open: "{} {} open",
    /// Text in `home`.
    compact_vault: "vault",
    /// Text in `home`.
    compact_vaults_inline: "vaults",
    /// Text in `home`.
    compact_bring_in_button: "Bring in",
    /// Text in `home`.
    compact_open_button: "Open",
    /// Text in `home`.
    compact_work: "Work",
    /// Text in `home`.
    compact_write_out: "Write out",
    /// Button in `home`.
    compact_lock: "Lock",
    /// Button in `home`.
    compact_scan: "Scan",
    /// Text in `tiles`.
    compact_wallets: "Wallets",
    /// Text in `tiles`.
    compact_vaults: "Vaults",
    /// Text in `tiles`. `{}`: `n`.
    compact_locked: "{} locked",
    /// Text in `tiles`. `{}`: `o`.
    compact_open: "{} open",
    /// Text in `tiles`.
    compact_files: "Files",
    /// Text in `tiles`.
    compact_learn: "Learn",
    /// Text in `tiles`.
    compact_backups: "Backups",
    /// Text in `tiles`. `{}`: `without`.
    compact_not_backed: "{} not backed up",
    /// Text in `tiles`.
    compact_tools: "Tools",
    /// Text in `tiles`.
    compact_transfer: "Transfer",
    /// Text in `tiles`.
    compact_settings: "Settings",
    /// Text in `keyboard`.
    compact_done: "Done",
    /// Text in `files_count`.
    compact_none: "none",
    /// Text in `home_files_line`. `{}`: `self.medium.noun()`.
    compact_no_medium: "no {}",
    // ----- compact (faraday-core/src/compact/screen.rs) -----
    /// Line in `wallet_card`.
    compact_no_wallet_loaded: "No wallet loaded",
    /// Text in `wallet_card`. `{}`: `s.spend.source`.
    compact_continue_signing: "Continue signing {}",
    /// Text in `wallet_card`. `{}`: `app.inbox[i].name`.
    compact_review: "Review {}",
    /// Button in `wallet_card`.
    compact_rename: "Rename",
    /// Text in `wallet_card`. `{}`: `here`, `m`.
    compact_of_signatures: "{} of {} signatures here",
    /// Chip in `wallet_card`.
    compact_can_spend: "Can spend",
    /// Button in `wallet_card`.
    compact_scan_a_key: "Scan a key",
    /// Text in `wallet_card`. `{}`: `nodes.len()`, `if nodes.len() == 1 { EN.compact_plac…`.
    compact_backup: "{} backup {}",
    /// Text in `wallet_card`.
    compact_place: "place",
    /// Text in `wallet_card`.
    compact_places: "places",
    /// Text in `wallet_card`. `{}`: `vault`.
    compact_plan_in_locked: "plan in {}, locked",
    /// Text in `wallet_card`.
    compact_no_backup_plan: "no backup plan",
    /// Heading in `wallet_card`.
    compact_first_receive: "First receive address · 0/0",
    /// Heading in `silent_card`.
    compact_key: "Key",
    /// Text in `silent_card`.
    compact_here: "Here",
    /// Text in `silent_card`.
    compact_not_here: "Not here",
    /// Text in `silent_card`.
    compact_load_from_vault: "Load from vault",
    /// Text in `silent_card`.
    compact_add_its_key: "Add its key",
    /// Heading in `silent_card`.
    compact_address: "Address",
    /// Line in `silent_card`.
    compact_labels_handed: "Labels handed out",
    /// Text in `silent_card`.
    compact_address_labels: "Address, labels and payments",
    /// Text in `silent_card`.
    compact_show_wallet: "Show wallet QR",
    /// Text in `silent_card`.
    compact_remove_from_session: "Remove from session",
    /// Text in `about`.
    compact_about_page: "About this page",
    /// Text in `entry`. `{}`: `wl.name`.
    compact_entry_for_wallet: "For {}",
    /// Text in `entry`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    compact_for_key: "For key {}",
    /// Text in `entry`.
    compact_other_languages: "Other languages",
    /// Text in `entry`. `{}`: `crate::forms::language_name(lang)`.
    compact_other_lists: "{} · other lists",
    /// Text in `entry`.
    compact_other_forms: "Other forms",
    /// Button in `entry`.
    compact_scan_a_seedqr: "Scan a SeedQR",
    /// Text in `entry`. `{}`: `taken.len() + 1`.
    compact_word: "Word {}",
    /// Heading in `entry`.
    compact_words: "Words",
    /// Label in `entry`.
    compact_bip_39_passphrase: "BIP-39 passphrase",
    /// Text in `entry`.
    compact_add_key: "Add key",
    /// Text in `entry`.
    compact_make_a_new_key: "Make a new key",
    /// Text in `entry`.
    compact_clear: "Clear",
    /// Text in `entry_form`.
    compact_words_of_one: "Words of one share",
    /// Text in `entry_form`.
    compact_nothing_in_yet: "Nothing in yet",
    /// Text in `entry_form`.
    compact_count_one: "1 in",
    /// Text in `entry_form`. `{}`: `n`.
    compact_count_many: "{} in",
    /// Label in `entry_form`.
    compact_passphrase: "Passphrase",
    /// Text in `entry_form`.
    compact_add_this_one: "Add this one",
    /// Text in `entry_form`.
    compact_scan_a_qr_code: "Scan a QR code",
    /// Text in `entry_form`.
    compact_make_the_key: "Make the key",
    // ----- create (faraday-core/src/create.rs) -----
    /// Text in `name`.
    create_single_key_native: "Single key · native SegWit",
    /// Text in `name`.
    create_single_key_taproot: "Single key · Taproot",
    /// Text in `name`.
    create_single_key_nested: "Single key · nested SegWit",
    /// Text in `name`.
    create_single_key_legacy: "Single key · legacy",
    /// Text in `name`.
    create_multisig_native: "Multisig · native SegWit",
    /// Text in `name`.
    create_multisig_nested: "Multisig · nested SegWit",
    /// Text in `name`.
    create_multisig_legacy: "Multisig · legacy",
    /// Text in `name`.
    create_multisig_taproot: "Multisig · Taproot",
    /// Text in `name`.
    create_threshold_frost: "Threshold · FROST",
    /// Text in `name`.
    create_all_keys_musig2: "All keys · MuSig2",
    /// Text in `line`.
    create_bc1q_addresses: "bc1q addresses; what most wallets make",
    /// Text in `line`.
    create_bc1p_addresses: "bc1p addresses; the smallest signatures",
    /// Text in `line`.
    create_3_addresses: "3… addresses, for services that cannot pay bc1",
    /// Text in `line`.
    create_1_addresses: "1… addresses, for old software only",
    /// Text in `line`.
    create_several_keys: "Several keys, a quorum signs; Sparrow, Coldcard, SeedSigner",
    /// Text in `line`.
    create_multisig_nested_line: "Multisig with 3… addresses",
    /// Text in `line`.
    create_multisig_legacy_line: "Multisig with 3… addresses, old style; avoid unless required",
    /// Text in `line`.
    create_multisig_taproot_line: "Multisig on Taproot, one script leaf; Sparrow and Nunchuk",
    /// Text in `line`.
    create_any_m_of_n_shares: "Any m of n shares sign one Taproot key; single-key on chain",
    /// Text in `line`.
    create_every_key_signs: "Every key signs together, one Taproot key; single-key on chain",
    /// Error in `spec`.
    create_a_share_has: "A share has no account xpub",
    /// Text in `path_spec`. `{}`: `path`.
    create_not_a_derivation: "Not a derivation path: {}",
    // ----- decode (faraday-core/src/decode.rs) -----
    /// Text in `script_kind`.
    decode_other_script: "Other script",
    /// Text in `sequence_text`.
    decode_final: "final",
    /// Text in `sequence_text`.
    decode_locktime_no_rbf: "locktime on, no replace-by-fee",
    /// Text in `sequence_text`.
    decode_relative_time: "relative time lock, replace-by-fee",
    /// Text in `sequence_text`.
    decode_relative_block: "relative block lock, replace-by-fee",
    /// Text in `locktime_text`.
    decode_0_none: "0 · none",
    /// Text in `locktime_text`. `{}`: `n`.
    decode_block_height: "{} · block height",
    /// Text in `locktime_text`. `{}`: `n`.
    decode_unix_time: "{} · Unix time",
    /// Text in `btc`. `{}`: `Amount::from_sat(sats).to_btc()`.
    decode_btc: "{} BTC",
    // ----- explore (faraday-core/src/explore/mod.rs) -----
    /// Toast in `explore_open`.
    explore_load_a_key: "Load a key first",
    // ----- explore (faraday-core/src/explore/screen.rs) -----
    /// Text in `draw`.
    explore_the_public: "The public side of a loaded key at any path: the extended public key, the public key and the address there. Compare them with what another wallet shows for the same path. Nothing private is shown.",
    /// Line in `draw`.
    explore_path: "Path",
    /// Text in `draw`.
    explore_index_prev: "Index -1",
    /// Text in `draw`.
    explore_index_next: "Index +1",
    /// Text in `draw`.
    explore_native_segwit: "Native SegWit",
    /// Text in `draw`.
    explore_taproot: "Taproot",
    /// Text in `draw`.
    explore_nested_segwit: "Nested SegWit",
    /// Text in `draw`.
    explore_legacy: "Legacy",
    /// Line in `draw`.
    explore_script: "Script",
    /// Text in `draw`.
    explore_address: "Address",
    /// Text in `draw`.
    explore_public_key: "Public key",
    /// Text in `draw`.
    explore_extended_key: "Extended key",
    /// Line in `header`.
    explore_wallets: "Wallets",
    /// Title in `header`.
    explore_explore: "Explore",
    // ----- family (faraday-core/src/family/mod.rs) -----
    /// Text in `WORDS_SOURCE`.
    family_typed_words: "Typed words",
    /// Text in `spend_button`.
    family_finish_the_backup: "Finish the backup first",
    /// Text in `spend_button`. `{}`: `s.signers.len()`, `self.spend_needed()`.
    family_carry_on_spend: "Carry on the spend · Signatures {} of {}",
    /// Text in `spend_button`. `{}`: `self.medium.noun()`.
    family_sign_the_psbt: "Sign the PSBT from the {}",
    /// Text in `spend_button`.
    family_spend_from_wallet: "Spend from this wallet",
    /// Text in `family_act`.
    family_finished_transaction: "Finished transaction",
    /// Note in `family_intake_seed`.
    family_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    /// Text in `family_intake_seed`. `{}`: `n + 1`.
    family_key_already: "Key {} · already here",
    /// Text in `family_intake_file`.
    family_wallet_description_read: "Wallet description read · a wallet is open already",
    /// Text in `family_intake_file`.
    family_wallet_description: "Wallet description read",
    /// Text in `family_intake_file`. `{}`: `k`.
    family_part_read_rebuilt: "Part {} read · the wallet is rebuilt",
    /// Text in `family_intake_file`. `{}`: `k`, `m.have.len()`, `m.n`.
    family_part_read_progress: "Part {} read · {} of {} keys in hand",
    /// Text in `family_intake_file`.
    family_a_public_key: "A public key: it shows the wallet but cannot sign",
    /// Text in `family_intake_file`.
    family_a_transaction: "A transaction to sign · kept for later",
    /// Text in `family_intake_file`. `{}`: `name`.
    family_is_in_files: "{} is in Files",
    /// Note in `family_intake_again`. `{}`: `fp_text(fp)`.
    family_seed_read: "Seed {} read",
    /// Text in `family_no_money`. `{}`: `label`.
    family_no_money: "{} · no money",
    /// Text in `family_seed_with`.
    family_passphrase_inline: "passphrase",
    /// Text in `family_seed_with`. `{}`: `l`.
    family_passphrase_n: "{} · passphrase",
    /// Text in `family_words_kind_now`. `{}`: `kind_name(*k)`.
    family_words: "Words · {}",
    /// Text in `family_rebuild`.
    family_restored_wallet: "Restored wallet",
    /// Text in `family_rebuild`.
    family_its_shares: "its shares",
    /// Error in `family_rebuild`. `{}`: `m.have.len()`, `m.n`.
    family_of_keys_in_hand: "{} of {} keys in hand",
    /// Text in `kind_name`.
    family_native_segwit_kind: "native SegWit",
    /// Text in `kind_name`.
    family_taproot: "Taproot",
    /// Text in `kind_name`.
    family_nested_segwit_kind: "nested SegWit",
    /// Text in `kind_name`.
    family_legacy: "legacy",
    /// Text in `kind_label`.
    family_native_segwit: "native SegWit multisig",
    /// Text in `kind_label`.
    family_taproot_multisig: "Taproot multisig",
    /// Text in `kind_label`.
    family_nested_segwit: "nested SegWit multisig",
    /// Text in `kind_label`.
    family_legacy_multisig: "legacy multisig",
    // ----- family (faraday-core/src/family/screen.rs) -----
    /// Text in `draw`. `{}`: `app.session.wallets[i].name`.
    family_spend_from_n: "Spend from {}",
    /// Text in `draw`.
    family_wallets: "Wallets",
    /// Text in `draw`.
    family_this_page_takes: "This page takes someone spending for the first time through every step. With a wallet or a seed already loaded, Spend opens on a shorter page instead: what is loaded and what each wallet needs next.",
    /// Text in `loaded_compact`.
    family_walk_through: "Walk through from the start",
    /// Text in `loaded_compact`.
    family_no_wallet_loaded: "no wallet loaded",
    /// Text in `loaded_compact`. `{}`: `s.spend.source`, `name`.
    family_signing_wallet: "Signing {} · {}",
    /// Button in `loaded_compact`.
    family_continue_signing: "Continue signing",
    /// Section label in `loaded_compact`.
    family_wallets_loaded: "Wallets loaded",
    /// Line in `loaded_compact`.
    family_none_loaded: "None loaded",
    /// Text in `loaded_compact`. `{}`: `here.min(need)`, `need`.
    family_can_spend_count: "Can spend · {} of {}",
    /// Text in `loaded_compact`. `{}`: `here`, `if here == 1 { EN.family_key } else {…`, `need`.
    family_here_needed: "{} {} here · {} needed",
    /// Text in `loaded_compact`.
    family_key: "key",
    /// Text in `loaded_compact`.
    family_keys: "keys",
    /// Text in `loaded_compact`.
    family_no_key_here: "No key here",
    /// Line in `loaded_compact`.
    family_an_open_vault: "An open vault holds a missing key · Open it to load it",
    /// Text in `loaded_compact`.
    family_spend_from_button: "Spend from it",
    /// Text in `loaded_compact`.
    family_open_button: "Open it",
    /// Section label in `loaded_compact`.
    family_keys_without_wallet: "Keys with no wallet loaded",
    /// Text in `loaded_compact`.
    family_restore_wallet: "Restore its wallet",
    /// Text in `loaded_compact`.
    family_spend_as_a_single: "Spend as a single-key wallet",
    /// Text in `loaded_compact`.
    family_home: "Home",
    /// Text in `loaded_compact`.
    family_spend: "Spend",
    /// Text in `summary`.
    family_six_stops: "Six stops",
    /// Text in `summary`.
    family_desktop_copy: "Desktop copy",
    /// Text in `summary`. `{}`: `app.medium.noun()`.
    family_running_from_medium: "Running from the {}",
    /// Text in `summary`.
    family_nothing_in_yet: "Nothing in yet",
    /// Text in `summary`.
    family_things_one: "1 thing in",
    /// Text in `summary`. `{}`: `n`.
    family_things_count: "{} things in",
    /// Text in `summary`.
    family_loaded_in_sparrow: "Loaded in Sparrow",
    /// Text in `summary`.
    family_money_shows: "Money shows",
    /// Text in `summary`.
    family_done_in_sparrow: "Done in Sparrow",
    /// Text in `summary`. `{}`: `s.spend.source`.
    family_signing: "Signing {}",
    /// Text in `summary`.
    family_not_here_yet: "Not here yet",
    /// Text in `summary`. `{}`: `app.medium.for_the()`.
    family_nothing_waits: "Nothing waits {}",
    /// Text in `summary`. `{}`: `app.medium.for_the()`.
    family_1_file: "1 file {}",
    /// Text in `summary`. `{}`: `n`, `app.medium.for_the()`.
    family_files: "{} files {}",
    /// Text in `summary`.
    family_after_the_transaction: "After the transaction",
    /// Button in `body`.
    family_have_psbt_button: "I have the PSBT",
    /// Text in `body`.
    family_show_the_finished: "Show the finished transaction as QR",
    /// Text in `more`.
    family_less_about: "Less about this",
    /// Text in `more`.
    family_more_about: "More about this",
    /// Button in `map`.
    family_start: "Start",
    /// Line in `holding`.
    family_bring_in_paper: "Or bring in what is on paper",
    /// Text in `holding`.
    family_scan_the_qr: "Scan the QR codes you have",
    /// Text in `holding`.
    family_type_the_words: "Type the words you have",
    /// Text in `holding`. `{}`: `i.name`.
    family_open: "Open {}",
    /// Text in `holding`. `{}`: `app.medium.a()`.
    family_copy_files_button: "Copy files in from {}",
    /// Line in `holding`.
    family_came_in_heading: "What came in",
    /// Text in `holding`.
    family_rebuild_wallet: "Rebuild the wallet",
    /// Text in `holding`. `{}`: `fp_text(fp)`, `name`.
    family_seed_not_wallets: "Seed {} is not one of {}'s keys",
    /// Text in `holding`.
    family_add_its_passphrase: "Add its passphrase",
    /// Text in `came_rows`. `{}`: `t`.
    family_seed_row: "Seed · {}",
    /// Text in `came_rows`. `{}`: `t`, `label`.
    family_seed: "Seed · {} · {}",
    /// Text in `came_rows`. `{}`: `wl.name`, `Session::shape(wl)`.
    family_description_row_shape: "Wallet description · {} · {}",
    /// Text in `came_rows`. `{}`: `name`.
    family_description_row: "Wallet description · {}",
    /// Text in `came_rows`. `{}`: `sheets`, `m.have.len()`, `m.n`.
    family_part_of_keys: "Part {} · {} of {} keys in hand",
    /// Error in `came_rows`. `{}`: `sheets`.
    family_part: "Part {}",
    /// Text in `came_rows`. `{}`: `fp`.
    family_key_xpub_n: "Key · {} · xpub",
    /// Text in `came_rows`.
    family_key_xpub: "Key · xpub",
    /// Text in `came_rows`.
    family_transaction: "Transaction · kept for later",
    /// Line in `question`.
    family_anything_else: "Anything else in the envelope?",
    /// Text in `question`.
    family_type_more_words: "Type more words",
    /// Text in `question`.
    family_that_s_everything: "That's everything",
    /// Line in `question`.
    family_these_seeds: "These seeds are one wallet with several keys.",
    /// Text in `question`.
    family_scan_the_description: "Scan the description",
    /// Text in `question`.
    family_i_can_t_find: "I can't find one",
    /// Text in `question`. `{}`: `s.n`.
    family_without_description: "Without the description Faraday needs all {} keys: their seeds or xpubs",
    /// Text in `vault`. `{}`: `app.medium.noun()`.
    family_remove_the_then: "Remove the {}, then type the passphrase",
    /// Text in `vault`. `{}`: `app.medium.for_the()`.
    family_sealed_here: "Sealed here, {}",
    /// Text in `vault`. `{}`: `app.medium.a()`.
    family_copied_from_medium: "Copied in from {}",
    /// Text in `vault`. `{}`: `app.vaults.time_text(mem, f.header.co…`.
    family_unlock_time: "Unlocks in {}",
    /// Line in `vault`.
    family_passphrase: "Passphrase",
    /// Line in `vault`.
    family_unlocking: "Unlocking",
    /// Text in `vault`. `{}`: `mib_text(m)`.
    family_this_computer: "This computer: {} free",
    /// Button in `vault`.
    family_unlock: "Unlock",
    /// Text in `stick_row`. `{}`: `app.medium.noun()`.
    family_plug_medium: "Plug the {} in",
    /// Text in `stick_row`. `{}`: `app.medium.a()`.
    family_faraday_asks: "Faraday asks to lock first when {} goes in",
    /// Line in `opened`.
    family_spend_from_label: "Spend from",
    /// Text in `opened`. `{}`: `Session::shape(wl)`, `here`, `slots.len()`.
    family_of_keys_here: "{} · {} of {} keys here",
    /// Line in `shortcut`.
    family_have_psbt_line: "I have the PSBT from Sparrow ready",
    /// Text in `shortcut`.
    family_scan_button: "Scan it",
    /// Text in `SHORTCUT`.
    family_i_have_the_psbt: "I have the PSBT from Sparrow ready: Scan it",
    /// Text in `loaded_in_sparrow`.
    family_show_the_wallet_qr: "Show the wallet QR",
    /// Line in `loaded_in_sparrow`.
    family_receive_address: "Receive address 0",
    /// Button in `loaded_in_sparrow`.
    family_it_is_loaded: "It is loaded",
    /// Line in `check`.
    family_none_of_these: "None of these show money.",
    /// Line in `check`.
    family_the_words_may: "The words may belong to a wallet with several keys, or need a different passphrase.",
    /// Text in `check`.
    family_scan_more: "Scan more",
    /// Text in `check`.
    family_type_a_passphrase: "Type a passphrase",
    /// Line in `check`.
    family_is_there_a_word: "Is there a word or sentence kept apart from the list?",
    /// Text in `check`.
    family_yes_type_button: "Yes, type it",
    /// Text in `check`.
    family_no_button: "No",
    /// Text in `check`. `{}`: `crate::family::kind_label(kind)`, `if pass { EN.family_with_a_passphrase…`, `g.at + 1`, `Faraday::guess_total(g)`.
    family_trying_progress: "Trying: {}, {} · {} of {}",
    /// Text in `check`.
    family_with_a_passphrase: "with a passphrase",
    /// Text in `check`.
    family_no_passphrase: "no passphrase",
    /// Text in `check`.
    family_show_the_wallet: "Show the wallet as a QR code",
    /// Text in `check`.
    family_open_online_wallet: "Open this in the wallet on the online computer",
    /// Text in `check`.
    family_open_this_one: "Open this one on the online computer too",
    /// Text in `check`.
    family_receive: "Receive",
    /// Text in `check`.
    family_change: "Change",
    /// Text in `check`.
    family_show_more_addresses: "Show more addresses",
    /// Text in `check`.
    family_no_money_shows: "No money shows",
    /// Text in `check`.
    family_yes_my_money: "Yes, my money shows",
    /// Text in `bring`. `{}`: `name`.
    family_every_coin_to_wallet: "Every coin to {}",
    /// Line in `bring`.
    family_amount_max_hint: "Amount: Max, in Sparrow's Send tab",
    /// Text in `bring`.
    family_scan_the_psbt: "Scan the PSBT",
    /// Text in `bring`.
    family_scan: "Scan",
    /// Text in `bring`. `{}`: `app.medium.a()`.
    family_on_medium: "On {}",
    /// Text in `bring`. `{}`: `app.medium.a()`.
    family_copy_psbt_button: "Copy it in from {}",
    /// Line in `bring`.
    family_in_files: "In Files",
    /// Line in `bring`.
    family_signing_this_one: "Signing this one",
    /// Text in `bring`.
    family_use_this_one: "Use this one",
    /// Text in `bring`. `{}`: `wl.name`, `Session::shape(wl)`.
    family_spends_from_wallet: "Spends from {} · {}",
    /// Line in `bring`.
    family_no_wallet_open: "No wallet open here matches this transaction. Change cannot be verified until its wallet is open: check that this is the transaction for this wallet.",
    /// Button in `bring`.
    family_read_button: "Read it",
    /// Text in `signers`.
    family_finish_backup_button: "Finish the backup",
    /// Text in `signers`.
    family_load_from_vault: "Load from vault",
    /// Text in `signers`.
    family_signs_here: "Signs here",
    /// Text in `signers`.
    family_not_here: "Not here",
    /// Text in `signers`. `{}`: `f.name`.
    family_unlock_n: "Unlock {}",
    /// Text in `signers`.
    family_scan_a_key: "Scan a key",
    /// Text in `signers`.
    family_collect_a_signature: "Collect a signature",
    /// Text in `signers`. `{}`: `app.medium.noun()`.
    family_remove_the_first: "Remove the {} first",
    /// Button in `signers`.
    family_continue: "Continue",
    /// Button in `away`.
    family_lock: "Lock",
    /// Button in `away`.
    family_power_off: "Power off",
    /// Text in `away`.
    family_start_over: "Start over",
    // ----- family (faraday-core/src/family/text.rs) -----
    /// Text in `map`.
    family_paper: "Paper",
    /// Text in `map`.
    family_what_you_have: "What you have",
    /// Text. `{}`: `m.noun()`.
    family_the_faraday: "The Faraday {} and its passphrase, or words written down, and often a sheet describing the wallet.",
    /// Text in `map`.
    family_open_the_wallet_here: "Open the wallet here",
    /// Text. `{}`: `m.noun()`.
    family_start_a_computer: "Start a computer from the Faraday {} and open the wallet.",
    /// Text in `map`.
    family_check_the_balance: "Check the balance",
    /// Text in `map`.
    family_put_the_wallet: "Put the wallet's description into Sparrow on an online computer.",
    /// Text in `map`.
    family_make_the_transaction: "Make the transaction in Sparrow. It gives you a PSBT.",
    /// Text in `map`.
    family_offline: "Offline",
    /// Text in `map`.
    family_sign_it_here: "Sign it here",
    /// Text in `map`.
    family_bring_the_psbt: "Bring the PSBT to Faraday, read what it does, and sign it.",
    /// Text in `map`.
    family_online: "Online",
    /// Text in `map`.
    family_take_the_signed: "Take the signed transaction back to Sparrow and broadcast it.",
    /// Text in `WORDS_ARE_MONEY`.
    family_the_words_are: "The words are the money. Never photograph them, and never type them into a phone, a website or a password manager. The same goes for the vault's passphrase.",
    /// Text in `UNSURE`.
    family_not_sure_what: "Not sure what you have?",
    /// Text in `ANYTHING_HINT`.
    family_another_list: "Another list of words, another QR code, a line starting wsh( or tr(, or a word kept apart from the list.",
    /// Text in `DESCRIPTION_HINT`.
    family_its_description: "Its description says how they fit together: a QR code labelled wallet, descriptor, setup or multisig; a long line starting wsh( or tr(; or sheets with lines like 73C5DA0A: xpub…",
    /// Text in `looks`.
    family_a_square_black: "A square black-and-white pattern",
    /// Text in `looks`.
    family_looks_qr_code: "A QR code: the same information in a form a camera can read. Several numbered 1 of 5 and so on are one thing in parts: scan them one after another. Never scan a QR code that sits beside seed words into anything but Faraday.",
    /// Text in `looks`.
    family_words_numbered: "Words numbered 1 to 12, 18 or 24",
    /// Text in `looks`.
    family_the_seed_phrase: "The seed phrase. Often headed recovery phrase, seed words or backup, or stamped into steel.",
    /// Text in `looks`.
    family_one_long_line: "One long line starting wpkh(, wsh(, tr(, xpub, zpub or tpub",
    /// Text in `looks`.
    family_a_descriptor: "A descriptor or an xpub. It shows the addresses and the balance but cannot spend. Do not post it publicly: it shows the wallet's whole history.",
    /// Text in `looks`.
    family_several_sheets: "Several sheets each holding a few lines, like 73C5DA0A: xpub…",
    /// Text in `looks`.
    family_one_description: "One description split up on purpose, often under a heading such as this sheet carries 2 of 3 wallet descriptor parts. Enough of the sheets, in any order, add up to the whole. A line such as Policy: 2 of 3 says how many signatures the wallet needs.",
    /// Text in `looks`.
    family_a_word_or_sentence: "A word or sentence kept apart from the list, called a passphrase or a 25th word",
    /// Text in `looks`.
    family_if_one_exists: "If one exists, the words alone open a wallet that is real but empty, and the money is in the wallet the words and the passphrase open. This is not the vault's passphrase: the vault's passphrase opens the vault file, and this one changes which wallet the words open.",
    /// Text in `looks`.
    family_a_short_line: "A short line starting bc1, 3 or 1",
    /// Text in `looks`.
    family_one_address: "One address, not a backup.",
    /// Text in `looks`.
    family_a_small_device: "A small device with a screen and buttons",
    /// Text in `looks`.
    family_a_hardware_wallet: "A hardware wallet. It is a way of using the words, not a replacement for them.",
    /// Text. `{}`: `m.a_full_cap()`.
    family_labelled_faraday: "{} labelled Faraday, and a passphrase on a separate card",
    /// Text. `{}`: `m.noun()`.
    family_looks_medium: "The {} starts the computer and carries the vault, a locked file named like vault.ofv. The passphrase opens it. The vault holds the wallet and the keys that sign for it.",
    /// Text in `title`.
    family_how_to_spend: "How to spend bitcoin",
    /// Text in `title`. `{}`: `app.medium.noun()`.
    family_title_start: "Start from the {}",
    /// Text in `title`.
    family_what_do_you: "What do you have?",
    /// Text in `title`.
    family_open_the_wallet: "Open the wallet",
    /// Text in `title`.
    family_load_the_wallet: "Load the wallet in Sparrow",
    /// Text in `title`.
    family_title_check_money: "Check the money is there, on the online computer",
    /// Text in `title`.
    family_title_write_payment: "Write the payment in Sparrow",
    /// Text in `title`.
    family_check_the_money: "Check the money is really there",
    /// Text in `title`.
    family_write_the_payment: "Write the payment",
    /// Text in `title`.
    family_bring_the_transaction: "Bring the transaction here",
    /// Text in `title`.
    family_put_everything: "Put everything away",
    /// Text in `title`.
    family_read_the_transaction: "Read the transaction",
    /// Text in `title`.
    family_the_transaction: "The transaction id",
    /// Text in `title`.
    family_title_which_way: "Which way it spends",
    /// Text in `title`.
    family_nonces: "Nonces",
    /// Text in `title`.
    family_add_the_seeds: "Add the seeds that can sign",
    /// Text in `title`.
    family_who_signs: "Who signs",
    /// Text in `title`.
    family_title_sign: "Sign it",
    /// Text in `title`.
    family_the_other_signatures: "The other signatures",
    /// Text in `title`.
    family_title_send: "Send it",
    /// Text in `lead`.
    family_six_stops_three: "Six stops, three on this computer and three on an online one. Nothing moves until the last.",
    /// Text in `lead`. `{}`: `app.medium.noun()`.
    family_lead_desktop_copy: "This is the desktop copy of Faraday, which runs on an ordinary computer that has been online. It is for testing. For real money, start a computer from the Faraday {}.",
    /// Text in `lead`. `{}`: `app.medium.noun()`.
    family_lead_running_offline: "This computer is running Faraday from the {}. It has no network, and it forgets everything when it is switched off.",
    /// Text in `lead`.
    family_scan_or_type: "Scan or type everything in the envelope, one thing at a time, in any order. Faraday works out the wallet from it.",
    /// Text in `lead`.
    family_in_sparrow_file_new: "In Sparrow: File, New Wallet, then scan the wallet QR. Its first address is receive address 0 here.",
    /// Text in `lead`.
    family_in_sparrow_s: "In Sparrow's Send tab: the address, the amount and a fee. Then Create Transaction, Finalize Transaction for Signing and Show QR.",
    /// Text in `lead`.
    family_words_alone: "Words alone do not say which kind of wallet they made. Faraday made the most common kind from them. Open it in the wallet on the online computer, such as Sparrow, and look at its balance. If no money shows, Faraday makes the next kind.",
    /// Text in `lead`.
    family_load_this_same: "Load this same wallet in Sparrow on your online computer to check the balance. The code below is the wallet's description: enough for Sparrow to find every address the wallet uses and add up what is on all of them. It cannot spend. Then check that the first address Sparrow shows is character for character the same as receive address 0 below. If it matches, two separate programs have read the same backup and agreed. If it does not match, stop: something was typed or scanned wrongly.",
    /// Text in `lead`.
    family_this_part_happens: "This part happens on the online computer. In Sparrow, open the wallet's Send tab, enter the address to pay and the amount, and choose a fee. Press Create Transaction, then Finalize Transaction for Signing, then Show QR: an animated code, which is what Faraday reads. Sparrow can also save it as a .psbt file.",
    /// Text in `lead`. `{}`: `app.medium.a()`, `app.medium.a()`, `app.medium.noun()`.
    family_two_ways_scanning: "Two ways. Scanning is the simpler: nothing has to lock. Press Scan and hold Sparrow's QR code up to this computer's camera; an animation is read as it loops. Or save the .psbt onto {} and copy it in. Faraday reads {} only while no vault or key is open, so with one open it locks first: it seals the vault, forgets the keys and starts again. Copy the PSBT in, remove the {}, and unlock with the same passphrase. This tab comes back on this page.",
    /// Text in `lead`. `{}`: `app.medium.noun()`, `app.medium.a()`, `app.medium.noun()`, `app.medium.noun()`.
    family_lock_faraday: "Lock Faraday. The vault is sealed again with the keys inside it, and none is left in memory. If a file you still need waits for the {}, plug {} in and the {} visit writes it. Then switch off, take the {} out, and put it back with the papers.",
    /// Text in `lead`.
    family_this_opens_once: "This opens once the transaction is here.",
    /// Text in `lead`.
    family_read_the_summary: "Read what leaves, what comes back and the fee before signing.",
    /// Text in `lead`.
    family_lead_write_down_id: "Write down the first and last few characters of this id. Sparrow shows the same id for the transaction it wrote; after sending, the id is how you follow it.",
    /// Text in `lead`.
    family_lead_every_key: "Every key that signs is listed. A key not here yet comes from its own place: Scan a key reads its SeedQR, or its words typed instead, and a vault that holds it unlocks. Faraday works out which key it is. Each is checked against the wallet, so a key from another wallet is refused. A key held by someone else does not have to come here at all: they sign the same PSBT on their own device, and their signed copy is collected after this device signs.",
    /// Text in `lead`.
    family_lead_one_seed: "One seed at a time: put its words in, add it, and the count goes up. Order does not matter and the same seed twice does no harm. Each list is checked against the wallet's own keys, so one belonging to a different wallet is refused. A seed kept as a QR code beside the words does not have to be typed: scan it. If that backup has a separate passphrase, type it in the passphrase box before adding.",
    /// Text in `lead`.
    family_signing_adds: "Signing adds the signature of every key loaded here. For a wallet that needs signatures from elsewhere, the next card lists who still has to sign and takes their signed copies; each is checked against its key and this transaction before it counts.",
    /// Text in `lead`. `{}`: `app.medium.a()`.
    family_the_finished: "The finished transaction is what the network accepts. Show the signed PSBT as a QR code; in Sparrow, on the same transaction, press Scan QR and hold it up to the webcam, then Broadcast Transaction. Or carry the finished transaction, which waits in Files, on {}. It is safe to hand to anyone: nobody can change where the money goes without breaking the signatures, and a broken transaction is rejected, not redirected.",
    /// Text in `more`.
    family_anyone_who_reads: "Anyone who reads the words can spend the money from anywhere in the world, and it cannot be reversed. Where a wallet needs several lists, anyone with enough of them can. Treat every list as if it were the only one. No legitimate bitcoin company will ever ask for them: not a support line, not someone helping you, not an AI.",
    /// Text in `more`.
    family_nothing_done: "Nothing done here is irreversible until the last step, and that step does not happen in Faraday. Opening the vault, typing words, looking at addresses and reading a transaction move nothing and notify no one. You can stop and come back.",
    /// Text in `more`.
    family_more_no_deadline: "There is no deadline; bitcoin does not expire. Rushing is how people get robbed, by scammers and by helpers acting in their own interest. Take your time, and find someone you trust.",
    /// Text in `more`. `{}`: `m.noun()`.
    family_more_stop_if_unsure: "If any of this stops making sense, stop. Put the {} and the paper somewhere safe and get help from someone you would trust with the money itself. Waiting costs nothing.",
    /// Text in `more`.
    family_shut_the_computer: "Shut the computer down completely, not sleep or restart. Unplug any network cable.",
    /// Text in `more`. `{}`: `m.noun()`.
    family_plug_the_faraday: "Plug the Faraday {} in, switch the computer on and press the boot menu key straight away, repeatedly. It is usually F12, F9, Esc or F2. The first screen often names the key for a second or two.",
    /// Text in `more`. `{}`: `m.full()`.
    family_more_pick_boot_device: "Pick the {} from the list that appears. If no list appears, the key was wrong or came too late: shut down and try another one.",
    /// Text in `more`. `{}`: `m.noun()`, `m.a()`.
    family_if_the_is_missing: "If the {} is missing, or someone else has opened the envelope, do not use it. Do not use {} you are unsure about: the words are the money, and taking longer costs nothing.",
    /// Text in `more`.
    family_a_vault_unlocks: "A vault unlocks in the time shown on purpose: it makes guessing the passphrase slow. A wrong passphrase opens nothing and says so.",
    /// Text in `more`.
    family_more_words_order: "Words go in in the order they are written. Faraday checks them against the list of 2048 words and the backup's own checksum, so a mistyped or missing word is caught.",
    /// Text in `more`.
    family_a_wallet_description: "A wallet description opens the wallet read-only: enough to see every address and to check the balance. The words that sign come later, after the transaction is read.",
    /// Text in `more`.
    family_split_sheets: "Split sheets go in one at a time, in any order; the wallet opens when enough of them are in.",
    /// Text in `more`.
    family_in_sparrow_file: "In Sparrow: File, New Wallet, give it a name, then choose the way in that scans a QR code or imports a descriptor, and hold this screen up to the webcam. Menu names change between versions; look for an import that takes a descriptor or a scan. If the code is animated, let it loop once.",
    /// Text in `more`.
    family_more_trusted_software: "Only load it into wallet software you chose and trust.",
    /// Text in `more`.
    family_faraday_can: "Faraday can work out every address the wallet owns, but not which were used. Money is often spread over several addresses, including change addresses from old payments, so the first few can be empty while the money is further down. Sparrow adds them all up.",
    /// Text in `more`.
    family_on_a_phone_you: "On a phone you can also look an address up on a block explorer such as mempool.space. That moves nothing; it does tell the website you looked.",
    /// Text in `more`.
    family_paying_into: "Paying into an exchange: the exchange gives a deposit address for bitcoin in your account. Copy it exactly, and compare the first and last few characters after pasting.",
    /// Text in `more`.
    family_faraday_locks: "Faraday locks after a few minutes without input. That is expected. Unlock again with the same passphrase and this tab opens where you left it.",
    /// Text in `more`.
    family_if_the_envelope: "If the envelope held a hardware wallet that came with the papers and looks untouched, you can sign the same PSBT on it too and compare. Follow the device's own guide to restore from the same words. Compare what both screens say before approving anything: the amount, the fee and the address. They must match.",
    /// Text in `more`.
    family_more_faraday_enough: "Faraday on its own is enough to do the whole job.",
    /// Text in `more`.
    family_any_of_these: "Any of these sends it: Sparrow, which opens a transaction from a QR code, a file or pasted text and broadcasts it; a block explorer's broadcast page, such as mempool.space or blockstream.info; or someone you trust.",
    /// Text in `more`.
    family_more_check_id: "Then check the id. Whatever sends it should report the id Faraday showed. If the id you see anywhere differs, what was sent is not what you signed here: stop and find out why.",
    /// Text in `more`.
    family_the_first_confirmation: "The first confirmation usually takes about ten minutes but can take hours if the fee was low. Exchanges usually want one to six confirmations before the money shows in the account.",
    /// Text in `more`. `{}`: `m.noun()`.
    family_when_faraday: "When Faraday signs with a vault open, it records the amounts it signed inside that vault, so locking seals a changed copy of the vault to write back. Writing it back to the {} keeps that record; leaving it out changes nothing else.",
    // ----- flow (faraday-core/src/flow.rs) -----
    /// Text in `column_foot`.
    flow_guided: "Guided",
    /// Text in `column_foot`.
    flow_steps_only: "Steps only",
    /// Line in `paged`.
    flow_change: "Change",
    /// Text in `paged`. `{}`: `i + 1`, `cards.len()`.
    flow_step_count: "Step {} of {}",
    /// Text in `paged`.
    flow_about_this_step: "About this step",
    /// Text in `paged`.
    flow_home: "Home",
    /// Button in `column_foot` and `paged`: leaves the flow for good (F51).
    flow_cancel: "Cancel",
    /// Button on the Cancel sheet (`in_progress::cancel_sheet`).
    flow_cancel_the_flow: "Cancel the flow",
    /// Button on the Cancel sheet (`in_progress::cancel_sheet`).
    flow_keep_going: "Keep going",
    /// Title of the Cancel dialog over a key flow opened from Make a new
    /// wallet (`in_progress::cancel_key_sheet`).
    flow_cancel_key_title: "Discard this key?",
    /// Line of the Cancel dialog over a key flow opened from Make a new
    /// wallet.
    flow_cancel_key_line: "What is entered for this key so far is discarded, and you return to Make a new wallet.",
    /// Button on the Cancel dialog over a key flow opened from Make a new
    /// wallet: discards the key so far.
    flow_cancel_key_discard: "Discard",
    /// Title of the Cancel sheet over Make a new wallet.
    flow_cancel_create: "Cancel making this wallet?",
    /// Title of the Cancel sheet over Restore a wallet.
    flow_cancel_restore: "Cancel restoring this wallet?",
    /// Title of the Cancel sheet over a wallet's backup.
    flow_cancel_backup: "Cancel this backup?",
    /// Title of the Cancel sheet over a spend.
    flow_cancel_spend: "Cancel this spend?",
    /// Title of the Cancel sheet over Sign a message.
    flow_cancel_message: "Cancel signing this message?",
    /// Title of the Cancel sheet over Create a vault.
    flow_cancel_vault: "Cancel making this vault?",
    /// Title of the Cancel sheet over a GPG key's flow.
    flow_cancel_key: "Cancel making this key?",
    /// Title of the Cancel sheet over the Secure Boot keys' flow.
    flow_cancel_keys: "Cancel making these keys?",
    /// Line on the Cancel sheet: what it loses.
    flow_lost_plan: "The plan and its places are lost.",
    /// Line on the Cancel sheet: what it loses.
    flow_lost_answers: "Its answers are lost.",
    /// Line on the Cancel sheet: what it loses.
    flow_lost_setup: "Its answers and the setup of its vaults are lost.",
    /// Line on the Cancel sheet: what it loses.
    flow_lost_spend: "Its steps and any signatures made are lost.",
    // ----- forms (faraday-core/src/forms.rs) -----
    /// Text in `name`.
    forms_bip_39_words: "BIP-39 words",
    /// Text in `name`.
    forms_slip_39_shares: "SLIP-39 shares",
    /// Text in `name`.
    forms_codex32: "codex32",
    /// Text in `name`.
    forms_seed_xor_parts: "Seed XOR parts",
    /// Text in `lines`. `{}`: `s.group_index() + 1`, `s.group_count()`, `s.member_index() + 1`, `s.member_threshold()`, `s.identifier()`.
    forms_group_of_member: "Group {} of {}, member {} · {} of the group needed · id {}",
    /// Text in `lines`. `{}`: `id`.
    forms_the_secret_id: "The secret · id {}",
    /// Text in `lines`. `{}`: `char::from(c.share_index())`, `id`, `c.threshold()`.
    forms_share_id_needed: "Share {} · id {} · {} needed",
    /// Text in `lines`. `{}`: `i`, `words`.
    forms_part_words: "Part {} · {} words",
    /// Text in `form_add_part`. `{}`: `e`.
    forms_not_slip39_share: "Not a SLIP-39 share: {}",
    /// Text in `form_add_part`. `{}`: `e`.
    forms_not_a_codex32: "Not a codex32 string: {}",
    /// Error in `form_add_part`.
    forms_other_seed_string: "That string is of another seed: its id differs",
    /// Error in `form_add_part`.
    forms_share_duplicate: "That share is in already",
    /// Text in `form_add_part`. `{}`: `e`.
    forms_not_bip_39_words: "Not BIP-39 words: {}",
    /// Error in `form_add_part`.
    forms_every_part_has: "Every part has the same number of words",
    /// Text in `form_recover`.
    forms_slip_39_key: "SLIP-39 key",
    /// Text in `form_recover`.
    forms_codex32_key: "codex32 key",
    /// Text in `form_recover`.
    forms_seed_xor_key: "Seed XOR key",
    /// Text in `form_recover`. `{}`: `e`.
    forms_the_shares_do: "The shares do not make a key: {}",
    /// Text in `form_recover`. `{}`: `e`.
    forms_the_strings_do: "The strings do not make a key: {}",
    /// Error in `form_recover`.
    forms_seed_xor_needs: "Seed XOR needs two parts or more",
    /// Toast in `form_recover`. `{}`: `fp_text(fp)`.
    forms_key_added: "Key {} added",
    /// Text in `language_name`.
    forms_english: "English",
    /// Text in `language_name`.
    forms_spanish: "Spanish",
    /// Text in `language_name`.
    forms_french: "French",
    /// Text in `language_name`.
    forms_italian: "Italian",
    /// Text in `language_name`.
    forms_czech: "Czech",
    /// Text in `language_name`.
    forms_portuguese: "Portuguese",
    /// Text in `language_name`.
    forms_japanese: "Japanese",
    /// Text in `language_name`.
    forms_korean: "Korean",
    /// Text in `language_name`.
    forms_chinese_simplified: "Chinese (Simplified)",
    /// Text in `language_name`.
    forms_chinese_traditional: "Chinese (Traditional)",
    /// Text in `typed_mnemonic`. `{}`: `k + 1`, `language_name(lang)`.
    forms_word_not_in_list: "Word {} is not on the {} list",
    // ----- glance (faraday-core/src/glance/mod.rs) -----
    /// Text in `RECEIVE_WARNING`.
    glance_back_up_before: "Back up before you receive",
    /// Text in `WAITING`.
    glance_waiting_backup: "Waiting for its backup",
    /// Text in `build`.
    glance_no_origin: "no origin",
    /// Text in `build`.
    glance_can_sign_passphrase: "Can sign · passphrase",
    /// Text in `build`.
    glance_can_sign_here: "Can sign here",
    /// Text in `build`.
    glance_in_this_vault_passphrase: "In this vault · passphrase",
    /// Text in `build`.
    glance_in_this_vault: "In this vault",
    /// Text in `build`.
    glance_cosigner_xpub: "Cosigner · xpub only",
    /// Text in `build`.
    glance_load_from_vault: "Load from vault",
    /// Text in `backup_nodes`. `{}`: `v + 1`.
    glance_vault: "Vault {}",
    /// Text in `backup_nodes`. `{}`: `p + 1`.
    glance_place: "Place {}",
    /// Text in `backup_nodes`. `{}`: `j + 1`, `shape.keys`.
    glance_share_count: "Share {} of {}",
    /// Text in `backup_nodes`. `{}`: `vault_name(v)`.
    glance_stick: "{} stick",
    /// Text in `backup_nodes`.
    glance_description: "Description",
    /// Text in `backup_nodes`. `{}`: `number(i)`.
    glance_key_file: "Key {} file",
    /// Text in `backup_nodes`. `{}`: `inside.join(", ")`.
    glance_in_the_vault: "{} in the vault",
    /// Text in `backup_nodes`. `{}`: `vault_name(v)`.
    glance_copy: "{} · copy",
    /// Text in `backup_nodes`. `{}`: `app.medium.cap()`.
    glance_of_files: "{} of files",
    /// Text in `backup_nodes`.
    glance_watch_only_software: "Watch-only software",
    /// Text in `backup_nodes`.
    glance_on_its_own_device: "On its own device",
    /// Text in `backup_nodes`.
    glance_its_own_stick: "Its own stick",
    /// Text in `backup_nodes`. `{}`: `at.join(", ")`.
    glance_stick_places: "Stick at {}",
    /// Text in `backup_nodes`.
    glance_not_here: "Not here",
    /// Text in `checked_text`.
    glance_checked: "Checked",
    /// Text in `checked_text`. `{}`: `d`.
    glance_checked_n: "Checked {}",
    /// Text in `alone_color`.
    glance_nothing: "Nothing",
    /// Text in `alone_color`.
    glance_sealed: "Sealed",
    /// Text. `{}`: `k.number`, `k.fingerprint`.
    glance_key_fingerprint: "Key {} · {}",
    /// Text. `{}`: `vault`.
    glance_plan_in_locked: "Plan in {}, locked",
    /// Text in `rows`.
    glance_unlock: "Unlock",
    /// Text in `rows`.
    glance_no_backup_plan_short: "No backup plan",
    /// Text in `rows`.
    glance_back_up: "Back up",
    /// Text in `rows`.
    glance_no_backup_plan: "No backup plan in this vault",
    /// Text in `draw_column_with`.
    glance_backup: "Backup",
    /// Text in `draw_column_with`.
    glance_keys: "Keys",
    /// Text in `draw_column_with`.
    glance_wallet: "Wallet",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_words: "Key {} words",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_seedqr: "Key {} SeedQR",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_passphrase: "Passphrase {}",
    /// Text in `thing_label`.
    glance_wallet_sheet: "Wallet sheet",
    /// Text in `thing_label`. `{}`: `j + 1`.
    glance_share: "Share {}",
    /// Text in `thing_label`. `{}`: `v + 1`.
    glance_vault_copy: "Vault {} · copy",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_seed: "Key {} seed",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key_seed_passphrase: "Key {} seed + passphrase",
    /// Text in `thing_label`.
    glance_the_description: "The description",
    /// Text in `thing_label`. `{}`: `number(i)`.
    glance_key: "Key {}",
    // ----- glance_sheet (faraday-core/src/glance/sheet.rs) -----
    /// Row in `view`.
    glance_sheet_open_vault: "Open the vault",
    /// Row in `view`.
    glance_sheet_open_wallets: "Open in Wallets",
    /// Row in `view`.
    glance_sheet_check_addresses: "Check addresses",
    /// Row in `view`.
    glance_sheet_the_first: "The first three",
    /// Row in `view`.
    glance_sheet_public_files: "Public files",
    /// Row in `view`.
    glance_sheet_for_the_software: "For the software",
    /// Text in `view`.
    glance_sheet_reprint_wallet_sheet: "Print the wallet sheet again",
    /// Row in `view`.
    glance_sheet_rename_wallet: "Rename the wallet",
    /// Text in `view`.
    glance_sheet_shape: "Shape",
    /// Text in `view`.
    glance_sheet_checksum: "Checksum",
    /// Text in `view`.
    glance_sheet_where_label: "Where",
    /// Text in `view`.
    glance_sheet_label: "Label",
    /// Text in `view`.
    glance_sheet_last_checked_label: "Last checked",
    /// Text in `view`.
    glance_sheet_its_seed: "Its seed is at",
    /// Row in `view`.
    glance_sheet_show_xpub: "Show xpub QR",
    /// Row in `view`.
    glance_sheet_xpub_file: "Xpub file",
    /// Row in `view`.
    glance_sheet_explore: "Explore this key",
    /// Row in `view`.
    glance_sheet_where: "Where it is",
    /// Row in `view`.
    glance_sheet_words: "Words, by hand",
    /// Row in `view`.
    glance_sheet_seedqr: "SeedQR, by hand",
    /// Row in `view`.
    glance_sheet_seed_xor: "Seed XOR or codex32",
    /// Row in `view`.
    glance_sheet_by_hand: "By hand",
    /// Text in `view`. `{}`: `n.name`.
    glance_sheet_into: "Into {}",
    /// Row in `view`.
    glance_sheet_as_a_file: "As a file",
    /// Text in `view`.
    glance_sheet_make_another_backup: "Make another backup of this seed",
    /// Text in `view`.
    glance_sheet_check_seed_copy: "Check a copy of this seed",
    /// Text in `view`.
    glance_sheet_its_passphrase: "Its passphrase",
    /// Row in `view`.
    glance_sheet_show_key_xpub: "Show its xpub",
    /// Text in `view`.
    glance_sheet_name_holder: "Name its holder",
    /// Text in `view`.
    glance_sheet_stick: "Stick",
    /// Text in `view`.
    glance_sheet_found: "Found alone",
    /// Text in `view`.
    glance_sheet_not_yet: "Not yet",
    /// Row in `view`.
    glance_sheet_open_vault_to_keep: "Open a vault to keep this",
    /// Text in `view`.
    glance_sheet_mark_checked: "Mark checked",
    /// Row in `view`.
    glance_sheet_what_goes: "What goes in its envelope",
    /// Row in `view`.
    glance_sheet_the_envelope: "The envelope list",
    /// Row in `view`.
    glance_sheet_envelope_pdf: "PDF, for the stick",
    /// Row in `view`.
    glance_sheet_add_here: "Add here",
    /// Text in `view`.
    glance_sheet_mark_lost: "Mark lost",
    /// Text in `view`.
    glance_sheet_mark_exposed: "Mark exposed",
    /// Row in `view`.
    glance_sheet_remove_place: "Remove the place",
    /// Text in `view`.
    glance_sheet_rename: "Rename",
    /// Row in `view`.
    glance_sheet_save_seeds: "Save its seeds into it",
    /// Row in `view`.
    glance_sheet_copy_to_stick: "Copy to another stick",
    /// Row in `view`.
    glance_sheet_write_the_seed: "Write the seed files again",
    /// Row in `view`.
    glance_sheet_rewrite_public: "Write the public files again",
    /// Row in `view`.
    glance_sheet_destroy_copies: "Destroy its copies by hand",
    /// Row in `view`.
    glance_sheet_reshow_descriptor_qr: "Show the descriptor QR again",
    /// Row in `view`.
    glance_sheet_public: "Public files for this software",
    /// Text in `view`.
    glance_sheet_contents_group: "In it",
    /// Text in `view`. `{}`: `key.number`.
    glance_sheet_key_seed: "Key {} seed",
    /// Text in `view`.
    glance_sheet_with_passphrase: "With its passphrase",
    /// Row in `view`.
    glance_sheet_the_description: "The description",
    /// Text in `view`.
    glance_sheet_save_more: "Save more into it",
    /// Text in `view`.
    glance_sheet_kept_at_label: "Kept at",
    /// Text in `view`.
    glance_sheet_seen_here: "Seen here",
    /// Text in `view`.
    glance_sheet_last_checked: "Last checked here",
    /// Text in `view`.
    glance_sheet_marked: "Marked",
    /// Row in `view`.
    glance_sheet_load_this_key: "Load this key",
    /// Row in `view`.
    glance_sheet_scan_my: "Scan my copy",
    /// Row in `view`.
    glance_sheet_type_my: "Type my copy",
    /// Row in `view`.
    glance_sheet_check_copy: "Check this copy",
    /// Row in `view`.
    glance_sheet_make_another: "Make another copy",
    /// Row in `view`.
    glance_sheet_rewrite: "Write it again",
    /// Row in `view`.
    glance_sheet_stick_detail: "For the stick",
    /// Row in `view`.
    glance_sheet_check_row: "Check it",
    /// Row in `view`.
    glance_sheet_by_derived_key: "By the key it makes",
    /// Row in `view`.
    glance_sheet_reprint: "Print it again",
    /// Row in `view`.
    glance_sheet_as_a_qr: "As a QR picture",
    /// Row in `view`.
    glance_sheet_as_text: "As text",
    /// Row in `view`.
    glance_sheet_another_copy: "Another copy",
    /// Row in `view`.
    glance_sheet_elsewhere: "Elsewhere",
    /// Row in `view`.
    glance_sheet_print: "Print the shares again",
    /// Row in `view`.
    glance_sheet_pdfs_text: "PDFs, text and pictures",
    /// Row in `view`.
    glance_sheet_keys_left: "Keys left off each share",
    /// Row in `view`.
    glance_sheet_copy_vault: "Copy the vault to another stick",
    /// Row in `view`.
    glance_sheet_then_place: "Then a place",
    /// Row in `view`.
    glance_sheet_write: "Write it out again",
    /// Row in `view`.
    glance_sheet_changed: "Changed since written",
    /// Row in `view`.
    glance_sheet_show_wallet: "Show wallet QR",
    /// Row in `view`.
    glance_sheet_the_descriptor: "The descriptor",
    /// Row in `view`.
    glance_sheet_move: "Move",
    /// Row in `view`.
    glance_sheet_to_another: "To another place",
    /// Row in `view`.
    glance_sheet_remove_from_plan: "Remove from the plan",
    /// Row in `view`.
    glance_sheet_i_destroyed: "I destroyed this copy",
    /// Text in `view`.
    glance_sheet_lost: "Lost",
    /// Text in `view`.
    glance_sheet_exposed: "Exposed",
    /// A line's mark on its sheet: a SeedQR dropped for the words.
    glance_sheet_drawn_in_part: "Drawn in part",
    /// Text in `view`. `{}`: `label`.
    glance_sheet_move_title: "Move {} to",
    /// Row in `view`.
    glance_sheet_a_new: "A new place",
    /// Text in `view`. `{}`: `glance::thing_label(Some(&g), what)`.
    glance_sheet_another: "Another copy of {}",
    /// Text in `view`. `{}`: `key.number`.
    glance_sheet_key_words: "Key {} words",
    /// Text in `view`. `{}`: `key.number`.
    glance_sheet_key_seedqr: "Key {} SeedQR",
    /// Text in `view`. `{}`: `j + 1`.
    glance_sheet_share: "Share {}",
    /// Row in `view`.
    glance_sheet_wallet: "Wallet sheet",
    /// Text in `view`. `{}`: `x.name`.
    glance_sheet_stick_n: "{} stick",
    /// Row in `view`.
    glance_sheet_a_copy: "A copy of the vault",
    /// Text in `view`. `{}`: `node.name`.
    glance_sheet_add_title: "Add at {}",
    /// Text in `view`.
    glance_sheet_destroy: "Destroy what goes off the plan by hand",
    /// Text in `view`.
    glance_sheet_confirm: "Confirm",
    /// Text in `view`.
    glance_sheet_someone_can: "Someone can now spend",
    /// Text in `view`.
    glance_sheet_someone_could: "Someone could spend with a vault's passphrase",
    /// Row in `view`.
    glance_sheet_move_money: "Move the money",
    /// Row in `view`.
    glance_sheet_new_wallet: "New wallet",
    /// Text in `view`.
    glance_sheet_the_wallet: "The wallet can no longer be rebuilt from what is left",
    /// Row in `view`.
    glance_sheet_back_up: "Back up again now",
    /// Text in `view`.
    glance_sheet_someone: "Someone can see the balance",
    /// Text in `view`.
    glance_sheet_nothing_lost: "Nothing lost that is not elsewhere",
    /// Row in `view`.
    glance_sheet_make_replacement: "Make a replacement copy",
    /// Text in `view`.
    glance_sheet_nothing: "Nothing it holds spends or shows the balance",
    /// Row in `view`.
    glance_sheet_nowhere: "Nowhere",
    /// Text in `view`.
    glance_sheet_chosen: "Chosen",
    /// Text in `view`.
    glance_sheet_destroyed: "Destroyed",
    /// Text in `view`. `{}`: `label`.
    glance_sheet_goes_to_group: "{} goes to",
    /// Text in `view`. `{}`: `node.name`.
    glance_sheet_remove: "Remove {}",
    /// Text in `view`. `{}`: `key.number`, `key.fingerprint`.
    glance_sheet_key: "Key {} · {}",
    /// Text in `view`.
    glance_sheet_holder: "Holder",
    /// Text in `view`.
    glance_sheet_name: "Name",
    /// Text in `view`.
    glance_sheet_keep: "Keep",
    /// Text in `view`.
    glance_sheet_type_passphrase: "Type it",
    /// Text in `view`.
    glance_sheet_passphrase: "Passphrase",
    /// Text in `view`. `{}`: `key.fingerprint`.
    glance_sheet_matches: "Matches: it makes key {}",
    /// Text in `view`.
    glance_sheet_mismatch: "Does not match",
    /// Text in `view`.
    glance_sheet_check_button: "Check",
    /// Text in `view`. `{}`: `key.number`.
    glance_sheet_passphrase_n: "Passphrase {}",
    /// Row in `view`.
    glance_sheet_show_row: "Show it",
    /// Row in `view`.
    glance_sheet_to_copy: "To copy by hand",
    /// Text in `view`.
    glance_sheet_load_with_passphrase: "Load this key with its passphrase",
    /// Text in `view`.
    glance_sheet_check: "Check my copy",
    /// Text in `view`.
    glance_sheet_change: "Change the plan",
    /// Text in `draw`.
    glance_sheet_back: "Back",
    /// Text in `draw`.
    glance_sheet_close: "Close",
    // ----- gpg (faraday-core/src/gpg.rs) -----
    /// Error in `clock`.
    gpg_this_computer_s: "This computer's clock is not known",
    /// Error in `gpg_make`.
    gpg_a_key_needs_a_name: "A key needs a name or an email",
    /// Error in `gpg_make`.
    gpg_no_randomness: "No randomness from the system yet. Try again",
    /// Text in `gpg_code`. `{}`: `faraday_pgp::fingerprint_text(&key.fi…`.
    gpg_fingerprint: "Fingerprint {}",
    /// Error in `gpg_code`.
    gpg_no_gpg_key_is_selected: "No GPG key is selected",
    /// Text in `gpg_code`.
    gpg_gpg_public_key: "GPG public key",
    /// Text in `gpg_code`.
    gpg_public_checks_signatures: "Public: checks signatures, signs nothing",
    /// Text in `gpg_code`.
    gpg_gpg_revocation: "GPG revocation certificate",
    /// Text in `gpg_code`.
    gpg_revocation_warning: "Whoever has it can revoke the key",
    /// Error in `gpg_code`.
    gpg_file_gone: "That file is no longer in Files",
    /// Text in `gpg_code`. `{}`: `item.name`.
    gpg_gpg_signature: "GPG signature · {}",
    /// Text in `gpg_code`. `{}`: `item.name`.
    gpg_signs: "Signs {}",
    /// Text in `gpg_code`. `{}`: `who`.
    gpg_signer_line: "By {}",
    /// Error in `gpg_code`.
    gpg_not_a_gpg_file: "Not a GPG file",
    /// Toast in `gpg_renew`. `{}`: `name`, `self.medium.for_the()`.
    gpg_renewed_waits: "Renewed · {} waits {}",
    /// Text in `gpg_sign_ticked`. `{}`: `self.medium.for_the()`.
    gpg_1_signature_waits: "1 signature waits {}",
    /// Text in `gpg_sign_ticked`. `{}`: `n`, `self.medium.for_the()`.
    gpg_signatures_wait: "{} signatures wait {}",
    /// Text in `shown`.
    gpg_never: "Never",
    /// A span of one second, as GnuPG says how far in the future a key was made.
    gpg_second: "{} second",
    /// A span of seconds, as GnuPG says it.
    gpg_seconds: "{} seconds",
    /// A span of one day.
    gpg_day: "{} day",
    /// A span of days.
    gpg_days: "{} days",
    /// Error in `usable`. `{}`: the date the key expired.
    gpg_expired: "Unusable secret key: it expired on {}",
    /// Error in `usable`. `{}`: the date.
    gpg_revoked: "Unusable secret key: it was revoked on {}",
    /// Error in `usable`, GnuPG's words. `{}`: the key's ID, the span.
    gpg_future: "Key {} was created {} in the future (time warp or clock problem)",
    /// Error in `gpg_sign_ticked`, `gpg_export`. `{}`: the file's name.
    gpg_exists: "File exists: {}",
    /// Error in `gpg_sign_ticked`. `{}`: the file's name.
    gpg_changed: "{} changed since it was ticked. It was not signed",
    /// Error in `gpg_renew`.
    gpg_choose_the_term: "Choose the new term first",
    /// Error in `gpg_renew`, after GnuPG's.
    gpg_time_conflict: "Time conflict: the clock is before the key's last signature",
    /// Error in `gpg_export_revocation`.
    gpg_no_revocation_kept: "No revocation certificate is kept with this key yet. Set the time and open the key again",
    /// Toast in `gpg_mark_revoked`.
    gpg_marked_revoked: "The key is marked revoked",
    /// Button in `years_row`.
    vault_3_years: "3 years",
    /// Button and heading on a GPG key's page.
    vault_make_a_revocation: "Make a revocation",
    /// Button on a GPG key's page.
    vault_export_revocation: "Export the kept revocation",
    /// Label of the reason buttons.
    vault_reason: "Reason",
    /// GnuPG's menu entry 0.
    vault_reason_0: "No reason specified",
    /// GnuPG's menu entry 1.
    vault_reason_1: "Key has been compromised",
    /// GnuPG's menu entry 2.
    vault_reason_2: "Key is superseded",
    /// GnuPG's menu entry 3.
    vault_reason_3: "Key is no longer used",
    /// Button of the revocation form.
    vault_make_the_revocation: "Make the revocation",
    /// Label of the revocation form's field.
    vaults_description: "Description (optional)",
    /// Label on a GPG key's page.
    vault_status: "Status",
    /// Row on a GPG key's page. `{}`: the date.
    vault_revoked_date: "Revoked on {}",
    /// Row on a GPG key's page. `{}`: the date.
    vault_kept_with_the_key: "Kept with the key · made {}",
    /// Row on a GPG key's page.
    vault_none_kept: "None kept yet",
    /// Line on a GPG key's page.
    vault_term_never: "Term now: does not expire",
    /// Line on a GPG key's page. `{}`: the date.
    vault_term_until: "Term now: expires {}",
    /// Hold button on a GPG key's page.
    vault_hold_to_mark_revoked: "Hold to mark the key revoked",
    /// Line on the signing page. `{}`: the IDs, the date.
    vault_signing_key_ids: "Signing subkey ID {} · main key ID {} · created {}",
    /// Hold button on Sign with {key}.
    vault_hold_to_sign_1_file: "Hold to sign 1 file",
    /// Hold button on Sign with {key}. `{}`: the count.
    vault_hold_to_sign_files: "Hold to sign {} files",
    /// Title of the sheet Sign opens on the signing page (F63). `{}`:
    /// the key's user ID.
    vault_sign_sheet_title: "Sign with {}",
    /// Row on Sign with {key}: the time the signatures will carry.
    vault_signature_time: "Signature time",
    // ----- guide (faraday-core/src/guide.rs) -----
    /// A Guided text's link to its Learn page. `{}`: the page's title.
    guide_learn_link: "Learn: {}",
    /// Text in `spend`.
    guide_this_is_the_wallet: "This is the wallet the transaction spends from, matched from its inputs. If it is not the wallet you meant, stop here.",
    /// Text in `spend`.
    guide_no_wallet_loaded: "Load the wallet first if you have its description. Continue goes on without it, and no output can be shown as change.",
    /// Text in `spend`.
    guide_open_the_same: "Compare these addresses with the ones the software that made this transaction shows. If they match, both read the same wallet and the change comes back to you.",
    /// Text in `spend`.
    guide_read_each_line: "Check each line against what you meant to send: the amount, the address, the change and the fee. Nothing is signed yet; if anything differs, stop.",
    /// Text in `spend`.
    guide_musig2_signs: "Every key adds a one-time nonce before anyone signs. Add the others' copies with their nonces, or go first and share this device's. Keep this session unlocked until the signatures return: locking forgets the nonce.",
    /// Text in `spend`.
    guide_this_wallet_can: "Each path lists the keys it needs and how long it waits. The line under it says whether this transaction's sequence allows that path; only the network knows whether the coins are old enough.",
    /// Text in `spend`. `{}`: `needed`, `if needed == 2 { EN.guide_one_other.t…`.
    #[cfg(feature = "unreleased")]
    guide_any_of_these: "Any {} shares sign, one after another, each at its own place. Pick {} to follow this one; after that no other share can join.",
    /// Text in `spend`.
    #[cfg(feature = "unreleased")]
    guide_one_other: "one other",
    /// Text in `spend`. `{}`: `needed - 1`.
    #[cfg(feature = "unreleased")]
    guide_others: "{} others",
    /// Text in `spend`.
    guide_this_wallet_has: "Each key below can sign. Which keys are enough depends on the path this transaction takes; some paths wait for a timelock.",
    /// Text in `spend`. `{}`: `needed`, `if needed == 1 { EN.guide_of_these } …`.
    guide_keys_must_sign: "{} {} keys must sign. Keys marked Signs here are loaded on this device. The others are on other devices or with other people; each of them signs the same PSBT and hands it back.",
    /// Text in `spend`.
    guide_of_these: "of these",
    /// Text in `spend`.
    guide_keys_must_sign_plural: "of the",
    /// Text in `spend`.
    guide_signed_each_signature: "Signed. Each signature was checked against its key before it was kept.",
    /// Text in `spend`, Signers, with no wallet loaded.
    guide_keys_signs_here: "Keys marked Signs here are loaded on this device. The others sign the same PSBT elsewhere.",
    /// Text in `spend`. `{}`: `m.noun()`.
    #[cfg(feature = "unreleased")]
    guide_spend_share_nonce: "Signing adds this share's part and writes a carry file for the next share. The carry file is secret until that share signs: keep the {} with you and sign the same day.",
    /// Text in `spend`.
    guide_spend_round_two: "Round two: this device adds its partial signature over everyone's nonces. The others do the same on their copies; when theirs come back, Faraday combines all of them into one signature.",
    /// Text in `spend`. `{}`: `needed`, `if needed == 1 { EN.guide_signature }…`.
    guide_spend_adds_signatures: "This adds the signature of every key loaded here. On its own it moves nothing: the transaction needs {} {} before it can be broadcast.",
    /// Text in `spend`.
    guide_signature: "signature",
    /// Text in `spend`.
    guide_signatures: "signatures",
    /// Text in `spend`. `{}`: `m.a()`, "a stick".
    guide_signing_uses: "Signing uses the key loaded here and checks the signature before keeping it. Nothing leaves the device until you write it to {} or show it as a QR code.",
    /// Text in `spend`. `{}`: `m.a()`, "a stick".
    guide_get_the_other: "Each cosigner signs the same PSBT on their own device. Hand it over as a QR code or on {}; scan or copy in each signed copy that comes back. A copy of another transaction is refused.",
    /// Text in `spend`.
    guide_there_are_two: "There are two results. The signed PSBT goes back to the wallet that made the transaction, which can broadcast it. The finished transaction can be broadcast as it is, from any node you trust. Either one is enough.",
    /// Text in `spend`. `{}`: `m.a()`.
    #[cfg(feature = "unreleased")]
    guide_keep_the_carry: "Keep the carry file in the vault, or write it to {}. At the next share's place, copy it into Faraday with that share loaded, open it from Files and sign. The share that signs last gets the finished transaction.",
    /// Text in `spend`.
    guide_the_transaction: "The transaction is finished when enough keys have signed. Until then it cannot be broadcast.",
    /// Text in `backup`.
    guide_print_the_blank: "Print the blank templates from Faraday Starter on the online computer: they hold no secret. Done marks them printed; you copy each seed onto one by hand next.",
    /// Text in `backup`.
    guide_backup_none_typed: "None of this wallet's seeds were typed into this session, so there is nothing to copy here. Each seed is backed up on the device that holds it.",
    /// Text in `backup`.
    guide_copy_words: "Copy each seed onto its own template by hand, the words in order. Then type your copy's words back; the first word that differs is named.",
    /// Text in `backup`, a passphrase on paper.
    guide_passphrase_on_paper: "Write the passphrase on paper by hand, exactly as it is, capitals and spaces included. Then type your copy back; it is compared whole.",
    /// Text in `backup`, a copy of the SeedQR.
    guide_copy_seedqr: "Copy each seed's SeedQR onto its own template by hand, square by square; the grey squares are printed already. Then scan your copy; with no camera, type its words back.",
    /// Text in `backup`, a copy of both.
    guide_copy_both: "Copy each seed onto its own template by hand: the words, then the SeedQR square by square. Type the words back and scan the grid; a mistake is named by its word.",
    /// Text in `backup`. `{}`: the Copy card's lead.
    guide_backup_paper: "{} A seed never goes to a printer.",
    /// Text in `backup`. `{}`: `medium.for_the()`.
    guide_backup_xpub_half: "These files hold xpubs only: they cannot spend, but only they put the keys back together into this wallet. Make the file for each program you use; it waits {}.",
    /// Text in `backup`, when the plan splits. `{}`: `medium.for_the()`.
    guide_instead_of_one: "Each signer gets a share of the wallet description with some keys left off: one share alone cannot watch the wallet, enough together rebuild it. Make them; each waits {} to print.",
    /// Text in `backup`.
    guide_one_envelope: "Put each place's papers in their own envelope and store the envelopes apart. A copy in a vault is one more copy, not a replacement for paper.",
    /// Text in `backup`, Write to sticks. `{}`: `medium.noun()`.
    guide_backup_sticks: "Lock and write locks this session, then each {} of the plan is written in turn, by its place.",
    /// Text in `backup`, a vault's card. `{}`: `medium.noun()`.
    guide_backup_vault: "This vault keeps a further copy of the seed, sealed under its passphrase. Save into it; Lock and write, the last item, seals it and writes it to its {}.",
    /// Text in `backup`, the wallet sheet of a plan that does not split. `{}`: `medium.for_the()`.
    guide_backup_sheet: "One sheet with the wallet's name, every xpub and the checksum: it cannot spend, but it rebuilds the wallet. Make it; the PDF waits {} to print.",
    /// Text in `backup`, the descriptor shown to the software.
    guide_backup_show: "Show this QR code to the watch-only software to load the wallet. It holds xpubs only: it can watch the wallet, not spend.",
    /// Text in `backup`, the seeds as files. `{}`: `medium.for_the()`, `medium.noun()`.
    guide_backup_files: "Each seed as a file {}, unprotected: anyone who copies the {} can spend with it. Make it only if your plan needs it; a file never holds the passphrase.",
    /// Text in `message`.
    guide_a_signed_message: "Pick the wallet and the receive address someone asked you to prove you control. Only single-key wallets whose key is loaded here are listed.",
    /// Text in `message`.
    guide_type_the_message: "Type the message exactly as the other side gave it. A single changed character, a space or a line break, makes a different message and the signature will not check.",
    /// Text in `message`.
    guide_message_formats: "Use the format the other side asked for. A format this address cannot use is greyed out.",
    /// Text in `message`.
    guide_signing_moves: "Signing moves no money and spends nothing. The result is three lines: the address, the signature and the message. Carry it out as a file or show it as a QR code.",
    /// Text in `create`. `{}`: `m`, `keys`.
    #[cfg(feature = "unreleased")]
    guide_any_of_shares: "Any {} of {} shares sign; on the chain the wallet is one Taproot key. Keep the shares in different places, or the threshold protects nothing. Up to five shares are dealt.",
    /// Text in `create`. `{}`: `m`, `keys - m`.
    #[cfg(feature = "unreleased")]
    guide_choose_loaded: "Choose {} loaded keys of 24 words as the first shares. Their words become shares as they are, so a person who already keeps one of these seeds keeps only that. The deal computes the other {} from them.",
    /// Text in `create`.
    guide_create_kinds: "Choose how the wallet locks its coins. One key is simplest; a multisig needs several keys, so one lost or stolen key does not lose the money. Pick a kind your other software supports.",
    /// Text in `create`. `{}`: `m`, `keys`, `m`.
    guide_of_every_spend: "{} of {}: every spend needs {} signatures. Keep the keys apart, on different devices or with different people, or the quorum protects nothing.",
    /// Text in `create`.
    guide_fill_each_slot: "Fill each slot with a key loaded here, a New key, or a cosigner's xpub, scanned or from a file; Someone else adds it later keeps a slot open. Make the wallet, once every slot is filled, makes it.",
    /// Text in `create`.
    guide_choose_the_key: "Choose the key: one loaded here, or a new one made here from fresh randomness. A new key exists only in this session until its words are written down in the backup step.",
    /// Text in `create`.
    guide_load_the_wallet: "Load the wallet into the software you will receive with, by the QR code or the descriptor file, and compare these addresses. They must match before you send anything to it.",
    /// Text in `create`, making the vaults.
    guide_create_vaults: "Make or unlock each vault now, before the keys exist, so each new seed can be saved into it.",
    /// Text in `create`, bringing in the sticks. `{}`: `medium.noun()` twice.
    guide_create_sticks: "Insert each vault's {} in turn. Keys are made only after the {} is removed.",
    /// Text in `create` and `restore`, Number of vaults. `{}`: `medium.noun()`.
    guide_create_vaults_how_many: "How many vaults the backup uses. Each is made on its own {}.",
    /// Text in `create` and `restore`, Use vaults.
    guide_create_vaults_where: "Choose whether vaults are part of the backup. A vault keeps a further copy of the seeds, sealed under a passphrase, beside the paper.",
    /// Text in `restore`.
    guide_the_kind_of_wallet: "Choose the kind written on the backup. A wallet description, scanned or in Files, sets the kind, the quorum and every key for you.",
    /// Text in `restore`.
    guide_how_many_keys: "How many keys the wallet has and how many sign, as written on the backup, often as 2 of 3.",
    /// Text in `restore`.
    guide_the_descriptor: "The descriptor, wallet file, BIP 129 record or split sheets. Without one, the wallet is made from the seeds and the cosigners' xpubs at the kind's standard path.",
    /// Text in `restore`.
    guide_one_slot_per: "One slot per key: its seed and passphrase, if it has one, or a cosigner's xpub. A seed that is not one of the description's keys is refused, so a wrong word or passphrase is caught here.",
    /// Text in `restore`.
    guide_compare_these: "Compare these addresses with the backup sheet or the wallet software that holds the history. If they match, the wallet is back as it was.",
    /// Text in `restore`.
    guide_restore_session: "The wallet is in this session until you lock.",
    /// Text in `keygen`.
    guide_a_slip_39_key: "Choose the length: shares of 20 words for 128 bits or 33 words for 256. A SLIP-39 key has no BIP-39 words.",
    /// Text in `keygen`.
    guide_write_each_share: "Write each share down on its own paper, in order, and keep the papers apart: in different places or with different people. Any of them up to the number needed are the key; fewer are nothing.",
    /// Text in `keygen`.
    guide_answer_from_your_papers: "Answer from your papers, not from memory: each share is asked in turn, every word once in a random order. A wrong answer means that share's paper needs correcting.",
    /// Text in `keygen`.
    guide_12_words_carry: "12 words carry 128 bits and 24 words 256. Either is beyond guessing; most wallets and signers read every length here.",
    /// Text in `keygen`, the Start card.
    guide_keygen_start: "Choose how the key comes in: made here from randomness, typed from words you hold, or scanned from a SeedQR. SLIP-39 shares, codex32 and Seed XOR are here too.",
    /// Text in `keygen`.
    guide_randomness_you_make: "Dice, coins or cards make a secret that does not depend on this device's software. The device's generator is quicker, but trusts this device. A mix is as good as its best source.",
    /// Text in `keygen`.
    guide_randomness_you: "Dice, coins or cards do not depend on this device's software. In the first group each word is your entries' own bits; in the second they are hashed. The device's generator trusts this device.",
    /// Text in `keygen`.
    guide_roll_one_die_at_a_time: "Roll one die at a time and enter the face: 1 to 3 counts as tails, 4 to 6 as heads.",
    /// Text in `keygen`.
    guide_keygen_dice_flips: "Roll one die at a time and enter the face: 1 to 3 is 0, 4 to 6 is 1. Every 11 rolls make one word, shown as its 11th roll comes in.",
    /// Text in `keygen`.
    guide_keygen_dice_words: "Roll one die at a time and enter the face; six rolls make a word, as the line below shows. A 5 or 6 in the first five is rolled again. The last word's checksum is worked out for you.",
    /// Text in `keygen`.
    guide_flip_a_coin_and_enter: "Flip a coin and enter each result: heads is 1, tails 0. Every 11 flips make one word, shown as its 11th flip comes in. Never choose the side.",
    /// Text in `keygen`.
    guide_roll_one_die: "Roll one die at a time and press the face that came up. Use a real die on a flat surface, not the same throw twice, and do not pick numbers in your head.",
    /// Text in `keygen`.
    guide_keygen_coins: "Flip a coin and press what came up, once per flip. Catching the coin in the air is fine; choosing the side is not.",
    /// Text in `keygen`.
    guide_shuffle_a_full: "Shuffle a full deck well, then turn cards over one at a time and enter each: its rank, then its suit. A card cannot come up twice from one deck; past 52, shuffle the deck again and keep going.",
    /// Text in `keygen`.
    guide_type_the_hex: "Type the hex digits you made elsewhere. They become the key as they are, so they must already be random.",
    /// Text in `keygen`.
    guide_point_the_camera: "Point the camera at something that moves or is lit unevenly and take each picture. The pictures are hashed and never kept.",
    /// Text in `keygen`.
    guide_this_device_asks: "This device asks the operating system's generator for 32 fresh bytes when this card opens. The key is those bytes, hashed. Nothing on the screen can show whether they were random.",
    /// Text in `keygen`.
    guide_keygen_lock_step: "Lock in adds the key these words make to its slot. A passphrase makes a different key: then both words and passphrase are needed to spend. Leave it empty for none.",
    /// Text in `keygen`.
    guide_lock_in_adds: "Lock in adds the key these words make; its fingerprint shows at once. Words and Quiz follow, to write the words down and check the copy.",
    /// Text in `keygen`.
    guide_these_counts: "These counts catch a biased die or entries typed instead of rolled. A caution is not a refusal; if one appears and you are not sure why, roll again.",
    /// Text in `keygen`.
    guide_write_the_words: "Write the words in order on paper before any money goes to this key. Anyone with the words can spend; without them, a lost device loses the money.",
    /// Text in `keygen`.
    guide_answer_from_your: "Answer from your paper, not from memory: each word is asked once, in a random order. A wrong answer means the paper needs correcting.",
    // ----- held (faraday-core/src/held.rs) -----
    /// Toast in `backup_settle`. `{}`: `names.join(", ")`.
    held_signs_from_now: "{} signs from now on",
    /// Text in `backup_extras`. `{}`: `name`, `seed`.
    held_holds_seed: "{} holds seed {}",
    // ----- inbox (faraday-core/src/inbox.rs) -----
    /// Text in `name`. `{}`: `ks.len()`.
    inbox_slip39_share_count: "{} SLIP-39 shares",
    /// Text in `name`. `{}`: `ks.len()`.
    inbox_codex32_shares: "{} codex32 shares",
    /// Text in `name`.
    inbox_loaded: "Loaded",
    /// Text in `found_in`.
    inbox_slip_39_seed: "SLIP-39 seed",
    /// Text in `found_in`. `{}`: `ks.len()`, `need`.
    inbox_slip_39_shares: "SLIP-39 shares · {} of {} in Files",
    /// Text in `found_in`.
    inbox_codex32_seed: "codex32 seed",
    /// Text in `found_in`. `{}`: `ks.len()`, `need`.
    inbox_codex32_shares_inline: "codex32 shares · {} of {} in Files",
    /// Text in `found_in`.
    inbox_restored_wallet: "Restored wallet",
    /// Text in `found_in`. `{}`: `m.have.len()`, `m.n`.
    inbox_split_sheets_progress: "Split sheets · {} of {} keys in hand",
    /// Text in `found_in`. `{}`: `e`.
    inbox_split_sheets: "Split sheets · {}",
    /// Text in `inbox_load_named`.
    inbox_its_shares: "its shares",
    /// Text in `xpub_make`. `{}`: `fp.map(fp_text).unwrap_or_else(|| EN.…`, `crate::family::kind_name(kind)`.
    inbox_watch_only: "{} · {} · watch-only",
    /// Text in `xpub_make`.
    inbox_xpub: "Xpub",
    /// Text in `xpub_make`.
    inbox_an_xpub_in_files: "an xpub in Files",
    /// Error in `potential_make`. `{}`: `self.medium.noun()`.
    inbox_remove_the_first: "Remove the {} first",
    /// Text in `potential_make`. `{}`: `seed.label`.
    inbox_with_passphrase: "{} with passphrase",
    /// Text in `potential_make`.
    inbox_this_seed_was: "This seed was loaded without its words; a passphrase cannot be added",
    /// Toast in `potential_make`. `{}`: `fp_text(fp)`.
    inbox_loaded_with_wallet: "{} loaded with its wallet",
    /// Text in `potential_make`.
    inbox_a_seed_in_files: "a seed in Files",
    /// Toast in `potential_make`. `{}`: `name`.
    inbox_is_a_wallet: "{} is a wallet",
    // ----- inputs (faraday-core/src/inputs.rs) -----
    /// Toast in `input_believe`. `{}`: `d.name`.
    inputs_is_in_use: "{} is in use",
    /// Toast in `input_typed`. `{}`: `name`.
    inputs_wrong_code: "{} typed the wrong code. Unplug it to try again.",
    /// Toast in `input_typed` and `input_new`.
    inputs_new_keyboards_ignored: "New keyboards are ignored until Faraday starts again.",
    // ----- keygen (faraday-core/src/keygen/mod.rs) -----
    /// Text in `source_name`.
    keygen_seed_xor: "Seed XOR",
    /// Text in `procedure_name`.
    keygen_words_chosen: "Words chosen by the dice",
    /// Error in `deal`.
    keygen_no_secret_yet: "No secret yet",
    /// Text in `deal`. `{}`: `e`.
    keygen_the_shares_could: "The shares could not be made: {}",
    /// Text in `caution_lines`.
    keygen_caution_long_run: "A long run of one result",
    /// Text in `caution_lines`.
    keygen_far_from_even: "Far from even: one result came up much more than the others",
    /// Text in `caution_lines`.
    keygen_a_long_run_counting: "A long run counting up or down",
    /// Text in `roll`.
    keygen_reroll_rule: "A 5 or a 6 is rolled again in a word's first five rolls",
    /// Note in `die_flip`.
    keygen_a_die_has_six: "A die has six faces",
    /// Note in `type_char`.
    keygen_no_more_are: "No more are needed",
    /// Text in `card`.
    keygen_card_already_drawn: "That card is already out of this deck",
    /// Note in `take_frame`.
    keygen_no_camera_picture: "No picture from the camera yet",
    /// Note in `take_frame`.
    keygen_too_little_variation: "Too little variation in that picture",
    /// Note in `finish_entry`. `{}`: `have`, `need`.
    keygen_entry_count: "{} of {}",
    /// Toast in `keygen_open`. `{}`: `self.medium.noun()`.
    keygen_remove_the_first: "Remove the {} first",
    /// Text in `keygen_act`.
    keygen_choose_two_sources: "Choose two sources or more",
    /// Text in `keygen_act`.
    keygen_choose_a_source: "Choose a source",
    /// Note in `keygen_act`.
    keygen_no_randomness: "No randomness from the system yet. Try again",
    /// Note in `keygen_lock`.
    keygen_the_two_passphrases: "The two passphrases differ",
    /// Text in `keygen_lock`. `{}`: `self.new_keys + 1`.
    keygen_new_key_n: "New key {}",
    /// Text in `keygen_done`. `{}`: `crate::wallet::fp_text(faraday_bip::keys:…`, `if k.slip39 { FEN.keygen_shares_inline } els…`.
    keygen_key_added_back: "Key {} added; back up its {} before you rely on it",
    /// Text in `keygen_done`.
    keygen_shares_inline: "shares",
    /// Text in `keygen_done`.
    keygen_words_inline: "words",
    // ----- keygen (faraday-core/src/keygen/screen.rs) -----
    /// Text in `TITLES`.
    keygen_check: "Check",
    /// Text in `TITLES`.
    keygen_words: "Words",
    /// Text in `draw`.
    keygen_make_a_new_wallet: "Make a new wallet",
    /// Text in `draw`.
    keygen_add_a_key: "Add a key",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_start: "Start",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_make_a_new_key: "Make a new key",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_type_the_words: "Type the words",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_scan_a_seedqr: "Scan a SeedQR",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_make_slip_39_shares: "Make SLIP-39 shares",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_type_slip_39_shares: "Type SLIP-39 shares",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_type_a_codex32: "Type codex32 strings",
    /// New key's Start card (`docs/history/NEW-WALLET.md` §15.1).
    keygen_type_seed_xor: "Type Seed XOR parts",
    /// Text in `draw`.
    keygen_tools: "Tools",
    /// Text in `draw`.
    keygen_back: "Back",
    /// Text in `draw`.
    keygen_new_key: "New key",
    /// Text in `entries_title`.
    keygen_flips: "Flips",
    /// Text in `entries_title`.
    keygen_cards: "Cards",
    /// Text in `entries_title`.
    keygen_pictures: "Pictures",
    /// Text in `entries_title`.
    keygen_generator: "Generator",
    /// Text in `entries_title`.
    keygen_entries: "Entries",
    /// Text in `entries_title`. `{}`: `what`, `(k.mix_at + 1).min(n)`, `n`.
    keygen_source_count: "{} · source {} of {}",
    /// Text in `asks`. `{}`: `k.dice_needed()`.
    keygen_rolls_of_a_six: "{} rolls of a six-sided die",
    /// Text in `asks`. `{}`: `CoinFlips::needed(st)`.
    keygen_flips_or_rolls: "{} flips, or rolls of a die read as flips",
    /// Text in `asks`. `{}`: `CardDraws::needed(st)`.
    keygen_cards_source: "{} cards from a shuffled deck",
    /// Text in `asks`. `{}`: `st.hex_digits()`.
    keygen_digits_made: "{} digits made elsewhere",
    /// Text in `asks`. `{}`: `CameraNoise::needed(st)`.
    keygen_pictures_n: "{} pictures",
    /// Text in `asks`.
    keygen_two_or_more: "Two or more of these, combined",
    /// This device's randomness, in `asks` and `device_rows`. `{}`: `EN.create_device_os`, the source the shell named (`/dev/random`, `getrandom(2)`, …).
    keygen_device_source: "{} · {}",
    /// This device's randomness once the person has added their own,
    /// which it mixes in (OWN-RANDOMNESS §2.4). `{}`: the line above.
    keygen_device_with_own: "{} · with your own",
    /// This device's foot button while the shell's answer is not in
    /// (F102).
    keygen_waiting_for_randomness: "Waiting for randomness",
    /// Text in `summary`. `{}`: `k.slip_m`, `k.slip_n`, `k.words`.
    keygen_slip_39_of_shares: "SLIP-39 · {} of {} shares · {} words each",
    /// Text in `summary`. `{}`: `source_name(Source::Coins)`.
    keygen_coins_from_die: "{} · read from a die",
    /// Text in `summary`.
    keygen_not_chosen: "Not chosen",
    /// Text in `summary`.
    keygen_complete: "Complete",
    /// Text in `summary`.
    keygen_not_locked: "Not locked in",
    /// Text in `summary`. `{}`: `fp`.
    keygen_locked_summary: "{} · locked in",
    /// Text in `summary`. `{}`: `fp`.
    keygen_passphrase_set: "{} · passphrase set · locked in",
    /// Text in `summary`. `{}`: `fp`.
    keygen_locked_no_passphrase: "{} · passphrase not set · locked in",
    /// Text in `summary`. `{}`: `k.mixed.len()`.
    keygen_sources_combined: "{} sources combined",
    /// Text in `summary`.
    keygen_no_cautions: "No cautions",
    /// Text in `summary`.
    keygen_1_caution: "1 caution",
    /// Text in `summary`. `{}`: `n`.
    keygen_cautions: "{} cautions",
    /// Text in `summary`. `{}`: `n`.
    keygen_shares_n: "{} shares",
    /// Text in `summary`.
    keygen_not_yet: "Not yet",
    /// Label in `read_only`.
    keygen_length: "Length",
    /// Label in `read_only`.
    keygen_randomness: "Randomness",
    /// Line in `verdict`.
    keygen_fair_no_cautions: "Fair: no cautions",
    /// Label in `key_card`.
    keygen_passphrase_again: "Passphrase again",
    /// Text in `key_card`.
    keygen_lock_button: "Lock in",
    /// Line in `key_card`. `{}`: the sample's name (`sample_words`), its
    /// fingerprint.
    keygen_lock_in_adds: "Lock in adds sample {} ({}) in place of these words",
    /// Line in `handed_line`. `{}`: the sample's name (`sample_words`), its
    /// fingerprint.
    keygen_added_sample: "Added sample {} ({}) in place of the key made",
    /// Text in `key_card` and `handed_line`. `{}`: the word, how many times
    /// it is repeated.
    sample_words: "{} ×{}",
    /// Line in `key_card`.
    keygen_enter_locks: "Enter locks in",
    /// Label in `key_card`.
    keygen_passphrase: "Passphrase",
    /// Text in `key_card`.
    keygen_set: "Set",
    /// Text in `key_card`.
    keygen_not_set: "Not set",
    /// Label in `key_card`.
    keygen_first_address: "First address",
    /// Label in `key_card`.
    keygen_key: "Key",
    /// Text in `SLIP39_FACTS`.
    keygen_fewer_shares: "Fewer shares than the threshold say nothing about the seed: with them, every seed stays as likely as every other. No entropy is lost to a holder of too few shares.",
    /// Text in `SLIP39_FACTS`.
    keygen_each_share_states: "Each share states in the clear its backup's identifier, the shares needed, how many were dealt and its own number: one share tells its holder how many others to look for.",
    /// Text in `SLIP39_FACTS`.
    keygen_the_threshold: "The threshold of shares rebuilds the whole seed on one device, and that device holds all of it while it signs.",
    /// Text in `SLIP39_FACTS`.
    keygen_the_split_s: "The split's own randomness comes from the device that deals it, here this device's system generator. A dealer with weak randomness gives the seed away in its shares.",
    /// Text in `SLIP39_FACTS`.
    keygen_a_share_cannot: "A share cannot be checked on its own against the seed; a wrong or mixed-up share shows only when the threshold is put together, by a 4-byte digest.",
    /// Text in `SLIP39_FACTS`.
    keygen_shares_restore: "Shares restore only in software that reads SLIP-39, such as Trezor's; a wallet that reads only BIP-39 words cannot.",
    /// Text in `SLIP39_FACTS`.
    keygen_slip39_share_length: "A share is 20 words for a 128-bit seed, the strength of 12 BIP-39 words, or 33 words for 256 bits.",
    /// Text in `length`.
    keygen_bip_39_words: "BIP-39 words",
    /// Text in `length`.
    keygen_slip_39_shares: "SLIP-39 shares",
    /// Text in `length`.
    keygen_shares_needed: "Shares needed",
    /// Text in `length`.
    keygen_shares_dealt: "Shares dealt",
    /// Text in `length`. `{}`: `k.slip_m`, `k.slip_n`.
    keygen_any_of_the_shares: "Any {} of the {} shares restore the key",
    /// Line in `length`.
    keygen_what_shares: "What shares reveal",
    /// Text in `length_counts`. `{}`: `n`.
    keygen_words_n: "{} words",
    /// Text in `way_name`. `{}`: `source_name(Source::Dice)`.
    keygen_flip_mode: "{} · Flip mode",
    /// Text in `way_line`. `{}`: `CoinFlips::needed(st)`.
    keygen_rolls_read_as: "{} rolls, read as flips",
    /// Text in `group_head`.
    keygen_your_own_entropy: "Your own entropy, words verifiable by hand",
    /// Text in `group_head`.
    keygen_your_entropy: "Your entropy, computer generates words",
    /// Text in `group_head`.
    keygen_device_group: "Made by this device",
    /// Chip in `group_head`.
    keygen_recommended: "Recommended",
    /// Text in `entries`.
    keygen_rolls_inline: "rolls",
    /// Text in `entries`.
    keygen_flips_inline: "flips",
    /// Text in `entries`.
    keygen_cards_inline: "cards",
    /// Text in `entries`.
    keygen_digits_inline: "digits",
    /// Text in `entries`.
    keygen_pictures_inline: "pictures",
    /// Text in `entries`. `{}`: `have`, `need`, `unit`.
    keygen_entry_count_unit: "{} of {} {}",
    /// Text in `entries`. `{}`: `last`.
    keygen_last: "Last: {}",
    /// Text in `entries`. `{}`: `k.cards.deck()`, `k.cards.in_deck()`.
    keygen_deck_status: "Deck {} · {} drawn from it",
    /// Text in `entries`. `{}`: `st.distinct`.
    keygen_luma_values: "{} luma values in the last picture",
    /// Text in `entries`.
    keygen_camera_active: "Camera on",
    /// Text in `entries`.
    keygen_waiting_camera: "Waiting for the camera",
    /// Line in `entries`.
    keygen_no_need_to_write: "No need to write the words down yet: Back up shows them again.",
    /// Section label in `entries`.
    keygen_randomness_check: "Randomness check",
    /// Text in `entries`.
    keygen_next_source: "Next source",
    /// Text in `entries`.
    keygen_make_the_words: "Make the words",
    /// Text in `entries`.
    keygen_undo: "Undo",
    /// Text in `entries`.
    keygen_clear: "Clear",
    /// Text in `entries`.
    keygen_dice_keys_hint: "Type 1 to 6 · Backspace takes one back · Enter continues",
    /// Text in `entries`.
    keygen_type_h_or_t: "Type H or T · Backspace takes one back · Enter continues",
    /// Text in `entries`.
    keygen_type_the_rank: "Type the rank (A 2–9 T J Q K), then the suit (S H D C)",
    /// Text in `entries`.
    keygen_hex_keys_hint: "Type 0–9 and a–f · Backspace takes one back · Enter continues",
    /// Text in `entries`.
    keygen_space_takes: "Space takes a picture · Enter continues",
    /// Text in `entries`.
    keygen_enter_continues: "Enter continues",
    /// Text in `entry_style`.
    keygen_coin: "Coin",
    /// Text in `entry_style`.
    keygen_die: "Die",
    /// Text in `entry_style`.
    keygen_typed: "Typed",
    /// Text in `entry_style`.
    keygen_buttons: "Buttons",
    /// Text in `entry_style`.
    keygen_dice_flip_rule: "Each roll is one flip: 1, 2, 3 are 0 (tails); 4, 5, 6 are 1 (heads). The key is the one these flips make",
    /// Text in `entry_style`.
    keygen_heads_is_1_tails: "Heads is 1, tails is 0",
    /// Text in `typed_box`.
    keygen_h_or_1_for_heads: "H or 1 for heads, T or 0 for tails",
    /// Text in `typed_box`.
    keygen_dice_faces_caption: "Faces 1 to 6, as rolled",
    /// Text in `reveal`. `{}`: `part`.
    keygen_of_11_bits: "{} of 11 bits",
    /// Row in `reveal`.
    keygen_from_the_checksum: "From the checksum, once the rest is in",
    /// Text in `reveal`. `{}`: `checksum`, `last + 1`.
    keygen_the_last_bits: "The last {} bits of word {} are the checksum",
    /// Section label in `rolled_words`.
    keygen_words_so_far: "Words so far",
    /// Line in `rolled_words`.
    keygen_dice_word_rule: "Rolls 1 to 5 of a word: 1 = 00, 2 = 01, 3 = 10, 4 = 11, 5 or 6 rolled again · Roll 6: 1 to 3 = 0, 4 to 6 = 1 · The 11 bits are the word's number in the list, from 0",
    /// Line in `rolled_words`.
    keygen_checksum: "checksum",
    /// Text in `rolled_words`. `{}`: `i`, `i + 1`.
    keygen_word: "= {} · word {}",
    /// Text in `rolled_words`. `{}`: `i`, `i + 1`.
    keygen_word_number: "= {} · word {} of the list",
    /// Note in `rolled_words`. `{}`: `got.len()`, `per`.
    keygen_of_rolls: "{} of {} rolls",
    /// Row in `rolled_words`.
    keygen_the_checksum: "The checksum, from all the other bits, once every roll is in",
    /// Text in `device_rows`.
    keygen_result: "Result",
    /// Text in `check`.
    keygen_continue: "Continue",
    /// Text in `again_label`.
    keygen_start_again: "Start again",
    /// Label in `check_rows`.
    keygen_rolls: "Rolls",
    /// Label in `check_rows`.
    keygen_faces: "Faces",
    /// Label in `check_rows`.
    keygen_suits: "Suits",
    /// Label in `check_rows`.
    keygen_digits: "Digits",
    /// Text in `check_rows`. `{}`: `hex`.
    keygen_sha_256: "SHA-256 {}…",
    /// Label in `share_words`.
    keygen_share: "Share",
    /// Text in `share_words`. `{}`: `k.share_at + 1`, `k.shares.len()`, `k.slip_m`.
    keygen_of_needed: "{} of {} · {} needed",
    /// Button in `share_words`.
    keygen_previous_share: "Previous share",
    /// Button in `share_words`.
    keygen_next_share: "Next share",
    /// Button in `share_words`.
    keygen_i_wrote_them_all: "I wrote them all down",
    /// Line in `words`.
    keygen_not_made_yet: "Not made yet",
    /// Label in `words`.
    keygen_source: "Source",
    /// Text in `words`.
    keygen_hide_words: "Hide words",
    /// Text in `words`.
    keygen_show_words: "Show words",
    /// Text in `words`.
    keygen_i_wrote_them: "I wrote them down",
    /// Line in `quiz`.
    keygen_show_the_words_first: "Show the words first",
    /// Label in `quiz`.
    keygen_fingerprint: "Fingerprint",
    /// Text in `quiz`. `{}`: `k.shares.len()`.
    keygen_all_shares_right: "All {} shares right",
    /// Text in `quiz`. `{}`: `n`.
    keygen_all_words_right: "All {} words right",
    /// Text in `quiz`.
    keygen_quiz_skipped: "Skipped: the copy is not checked",
    /// Label in `quiz`.
    keygen_quiz: "Quiz",
    /// Text in `quiz`.
    keygen_done: "Done",
    /// Text in `quiz`.
    keygen_use_this_key: "Use this key",
    /// Text in `quiz`.
    keygen_add_key: "Add key",
    /// Text in `quiz`. `{}`: `k.quiz_share + 1`, `k.shares.len()`, `done`, `n`.
    keygen_quiz_progress: "Share {} of {} · {} of {} right",
    /// Text in `quiz`. `{}`: `done`, `n`.
    keygen_of_right: "{} of {} right",
    /// Text in `quiz`. `{}`: `q.position() + 1`.
    keygen_which_is_word: "Which is word {}?",
    /// Line in `quiz`.
    keygen_not_that_one: "Not that one. Check your copy against the words",
    /// Button in `quiz`.
    keygen_ask_it_again: "Ask it again",
    /// Button in `quiz`.
    keygen_show_the_words: "Show the words",
    /// Line in `quiz`.
    keygen_quiz_skip_warning: "Without the quiz, a word copied wrongly is found only when the backup is needed",
    /// Text in `quiz`.
    keygen_skip_and_add: "Skip and add the key",
    /// Button in `quiz`.
    keygen_skip_the_quiz: "Skip the quiz",
    /// Text in `quiz`.
    keygen_quiz_keys_add: "Type 1 to 4 to pick · Enter adds the key once it passes",
    /// Text in `quiz`.
    keygen_quiz_keys_done: "Type 1 to 4 to pick · Enter is Done once it passes",
    // ----- name (faraday-core/src/lib.rs) -----
    /// Text in `name`.
    name_faraday_starter_app: "Faraday Starter",
    /// The Starter's name in `--version` before F16, still recognised.
    name_faraday_starter_app_old: "Faraday hub app",
    /// The Starter's name in `--version` as "Faraday Hub" (the code's
    /// working name before the F16 rename), still recognised.
    name_faraday_starter_app_old2: "Faraday Hub",
    /// Text in `name`.
    name_faraday_desktop: "Faraday Demo",
    /// The desktop app's name in `--version` before F16, still recognised.
    name_faraday_desktop_old: "Faraday desktop app",
    /// The desktop app's name in `--version` before it became Faraday Demo, still recognised.
    name_faraday_desktop_old2: "Faraday Desktop",
    // ----- version_label (faraday-core/src/lib.rs) -----
    /// Text in `version_label`. `{}`: `VERSION`.
    version_label_local: "{} (local build)",
    // ----- stick_kind (faraday-core/src/lib.rs) -----
    /// Text in `stick_kind`.
    stick_kind_psbt: "PSBT",
    /// Text in `stick_kind`.
    stick_kind_text: "Text",
    /// Text in `stick_kind`.
    stick_kind_transaction: "Transaction",
    /// Text in `stick_kind`.
    stick_kind_print_sheet: "Sheet for printing",
    /// Text in `stick_kind`.
    stick_kind_vault: "Vault",
    /// Text in `stick_kind`.
    stick_kind_efi_image: "EFI image",
    /// Text in `stick_kind`.
    #[cfg(feature = "unreleased")]
    stick_kind_threshold: "Threshold spend, part-signed",
    /// Text in `stick_kind`.
    stick_kind_qr_codes: "QR codes in a PNG",
    /// Text in `stick_kind`.
    stick_kind_file: "File",
    // ----- stick_keys (faraday-core/src/lib.rs) -----
    /// Text in `stick_keys`. `{}`: `m.noun()`.
    stick_keys_keys_load: "Keys load only with no {} attached",
    // ----- of (faraday-core/src/lib.rs) -----
    /// Text in `of`.
    of_ur_crypto_psbt_one: "ur:crypto-psbt · one code",
    /// Text in `of`. `{}`: `frames.len()`.
    of_ur_crypto_psbt_frames: "ur:crypto-psbt · {} frames, shown in turn",
    /// Text in `of`.
    of_text_one_code: "Text · one code",
    /// Text in `of`.
    of_ur_crypto_account: "ur:crypto-account · one code",
    /// Text in `of`.
    of_key_one_code: "Key · one code",
    /// Text in `of`.
    of_file_ur_bytes: "File · ur:bytes",
    // ----- parts_frames (faraday-core/src/lib.rs) -----
    /// Text in `parts_frames`.
    parts_frames_text_one: "Text · one code",
    /// Text in `parts_frames`. `{}`: `frames.len()`.
    parts_frames_numbered: "Numbered parts · {} frames, shown in turn",
    // ----- bbqr_frames (faraday-core/src/lib.rs) -----
    /// Error in `bbqr_frames`.
    bbqr_frames_too_large: "Too large for BBQr",
    /// Text in `bbqr_frames`. `{}`: `t`.
    bbqr_frames_bbqr_one: "BBQr {} · one code",
    /// Text in `bbqr_frames`. `{}`: `t`, `frames.len()`.
    bbqr_frames_bbqr_frames: "BBQr {} · {} frames, shown in turn",
    // ----- key_label (faraday-core/src/lib.rs) -----
    /// Text in `key_label`. `{}`: `fp`, `kind`, `p`.
    key_label_key_kind_path: "Key {} · {} · {}",
    /// Text in `key_label`. `{}`: `fp`, `kind`.
    key_label_key: "Key {} · {}",
    // ----- ur_bytes_frames (faraday-core/src/lib.rs) -----
    /// Text in `ur_bytes_frames`. `{}`: `what`.
    ur_bytes_frames_one: "{} · one code",
    /// Text in `ur_bytes_frames`. `{}`: `what`, `frames.len()`.
    ur_bytes_frames_frames: "{} · {} frames, shown in turn",
    // ----- part_note (faraday-core/src/lib.rs) -----
    /// Text in `part_note`. `{}`: `what`, `have`, `total`, `list.join(", ")`, `more`.
    part_note_of_missing: "{}: {} of {} · missing {}{}",
    // ----- secret_text (faraday-core/src/lib.rs) -----
    /// Text in `secret_text`.
    secret_text_those_are: "Those are a seed's words. Seeds go in through Add a key, not Files",
    /// Text in `secret_text`.
    secret_text_private_key: "That is a private key. Faraday takes keys as words or a SeedQR, in Add a key",
    // ----- what_instead_of_a_seed (faraday-core/src/lib.rs) -----
    /// Text in `what_instead_of_a_seed`.
    what_instead_of_a_seed_transaction: "That is a transaction, not a seed. Scan it from Files",
    /// Text in `what_instead_of_a_seed`.
    what_instead_of_a_seed_files_code: "That is a code for Files, not a seed",
    /// Text in `what_instead_of_a_seed`.
    what_instead_of_a_seed_public: "That is a public wallet or key, not a seed: it cannot sign",
    // ----- app (faraday-core/src/lib.rs) -----
    /// Text in `NOT_A_KEY`.
    app_not_wallet_key: "Not a key of this wallet: check the words or the passphrase",
    // ----- heading (faraday-core/src/lib.rs) -----
    /// Text in `heading`. `{}`: `self.label`, `vault::time_of_day(vault::local_sec…`.
    heading_written_to_time: "Written to {} at {}",
    /// Text in `heading`. `{}`: `self.label`.
    heading_written_to_label: "Written to {}",
    // ----- storage_event (faraday-core/src/lib.rs) -----
    /// Text in `storage_event`. `{}`: `name`, `self.medium.noun()`.
    storage_event_copied_words: "Copied {}: a key's words, loaded when the {} is removed",
    /// Text in `storage_event`. `{}`: `name`.
    storage_event_wrote_read: "Wrote {}, read back and matched",
    /// Text in `storage_event`. `{}`: `name`, `wrote_as`.
    storage_event_wrote: "Wrote {} as {}, read back and matched",
    /// Text in `storage_event`.
    storage_event_it_stays: "It stays in Files",
    /// Text in `storage_event`. `{}`: `self.medium.for_the()`.
    storage_event_it_still: "It still waits {}",
    /// Text in `storage_event`. `{}`: `name`, `reason`, `stays`.
    storage_event_not_written: "{} not written: {}. {}",
    /// Toast in `storage_event`. `{}`: `path`.
    storage_event_saved: "Saved {}",
    /// Text in `storage_event`.
    storage_event_too_fine: "Too fine for this camera: show it as an animated code with smaller parts, bring it closer, or carry it as a file",
    /// Toast in `storage_event`. `{}`: `reason`.
    storage_event_not_printed: "Not printed: {}",
    // ----- sticks_changed (faraday-core/src/lib.rs) -----
    /// Text in `sticks_changed`. `{}`: `self.medium.cap()`.
    sticks_changed_removed_only: "{} removed",
    /// Text in `sticks_changed`. `{}`: `self.medium.cap()`, `loaded`, `if loaded == 1 { EN.sticks_changed_ke…`.
    sticks_changed_removed: "{} removed · {} {} loaded",
    /// Text in `sticks_changed`.
    sticks_changed_key: "key",
    /// Text in `sticks_changed`.
    sticks_changed_keys: "keys",
    // ----- toast_out (faraday-core/src/lib.rs) -----
    /// Text in `toast_out`. `{}`: `what`, `self.medium.for_the()`.
    toast_out_waits: "{} waits {}",
    // ----- pull_what (faraday-core/src/lib.rs) -----
    /// Text in `pull_what`.
    pull_what_load_wallet: "load the wallet",
    /// Text in `pull_what`.
    pull_what_unlock: "unlock",
    /// Text in `pull_what`.
    pull_what_make_a_key: "make a key",
    /// Text in `pull_what`: a backup flow's Create {vault}, the key made
    /// after it.
    pull_what_create_vault: "create the vault, then your key",
    /// Text in `pull_what`.
    pull_what_add_a_key: "add a key",
    /// Text in `pull_what`.
    pull_what_scan: "scan",
    /// Text in `pull_what`.
    pull_what_load_samples: "load the samples",
    /// The Lock sheet's last section when the lock is a mode change.
    lock_mode_keeps_nothing: "A mode change keeps nothing but the mode: Files, For the stick, keys and wallets are emptied.",
    // ----- visit_then (faraday-core/src/lib.rs) -----
    /// Text in `visit_then`. `{}`: `self.back_name(s)`.
    visit_then_then: "Then: {}",
    /// Text in `visit_then`.
    visit_then_then_unlock: "Then: Unlock",
    /// Text in `visit_then`.
    visit_then_then_home: "Then: Home",
    // ----- start_spend (faraday-core/src/lib.rs) -----
    /// Toast in `start_spend`.
    #[cfg(feature = "unreleased")]
    start_spend_the_carry: "The carry file holds no PSBT",
    /// Toast in `start_spend`.
    start_spend_not_a_psbt: "Not a PSBT",
    // ----- session_stage (faraday-core/src/lib.rs) -----
    /// Text in `session_stage`.
    session_stage_bring: "Bring in",
    /// Text in `session_stage`.
    session_stage_write: "Write out",
    /// Text in `session_stage`.
    session_stage_work: "Work",
    /// Text in `session_stage`.
    session_stage_open: "Open",
    // ----- act (faraday-core/src/lib.rs) -----
    /// Text in `act`. `{}`: `w`, `if w == 1 { EN.act_wallet } else { EN…`, `k`, `if k == 1 { EN.act_seed } else { EN.a…`.
    act_and_loaded: "{} {} and {} {} loaded",
    /// Text in `act`.
    act_wallet: "wallet",
    /// Text in `act`.
    act_wallets: "wallets",
    /// Text in `act`.
    act_seed: "seed",
    /// Text in `act`.
    act_seeds: "seeds",
    /// Toast in `act`. `{}`: `self.medium.noun()`.
    act_remove_the_first: "Remove the {} first",
    /// Text in `act`. `{}`: `s.spend.source`.
    act_finished_from_source: "Finished from {}",
    /// Text in `act`.
    act_transaction: "Transaction",
    /// Text in `act`. `{}`: `self.medium.a()`.
    act_plug_in_the_visit: "Plug in {}: the visit writes what waits for it",
    /// Toast in `act`.
    act_bip_129_covers: "BIP 129 covers wsh and sh(wsh) multisig keys",
    /// Note in `act`.
    act_carrying_on_parts: "Carrying on with the parts read before",
    /// Error in `act`. `{}`: `m.have.len()`, `m.n`.
    act_of_keys_in_hand: "{} of {} keys in hand",
    /// Text in `act`.
    act_restored_wallet: "Restored wallet",
    /// Text in `act`.
    act_its_shares: "its shares",
    /// Toast in `act`. `{}`: `name`.
    act_restored_from_shares: "{} restored from its shares",
    /// Toast in `act`.
    act_keys_missing: "Not every key is in hand yet",
    /// Text in `act`.
    act_signed_psbt: "Signed PSBT",
    /// Text in `act`.
    act_psbt_for_the_next: "PSBT for the next signer",
    /// Text in `act`.
    #[cfg(feature = "unreleased")]
    act_the_secret_nonce: "The secret nonce the next share signs this transaction with",
    /// Text in `act`.
    #[cfg(feature = "unreleased")]
    act_nonce_warning: "Whoever has it and a share's signature can work out that share",
    // ----- backup_kept (faraday-core/src/lib.rs) -----
    /// Text in `backup_kept`.
    backup_kept_not_checked: "Not checked: no vault unlocked",
    /// Text in `backup_kept`. `{}`: `self.vaults.open[v].name`.
    backup_kept_wallet_vault: "Wallet in {}",
    /// Text in `backup_kept`.
    backup_kept_wallet: "Wallet not in a vault",
    /// Text in `backup_kept`. `{}`: `self.vaults.open[v].name`, `if with { " with its passphrase" } el…`.
    backup_kept_vault: "In {}{}",
    /// Text in `backup_kept`, after a space.
    backup_with_its_passphrase: "with its passphrase",
    /// Text in `backup_kept`.
    backup_kept_no_vault: "Not in a vault",
    /// Text in `backup_kept`.
    backup_kept_paper_copy: "Paper copy checked",
    /// Text in `backup_kept`.
    backup_kept_paper_unchecked: "Paper: not checked",
    /// Text in `backup_kept`. `{}`: `self.medium.for_the()`.
    backup_kept_file_unprotected: "File {}, unprotected",
    /// Text in `backup_kept`.
    backup_kept_not_here: "Not here: backed up on its own device",
    /// Text in `backup_kept`. `{}`: `id + 1`.
    backup_kept_share: "Share {}",
    // ----- backup_seed_list_of (faraday-core/src/lib.rs) -----
    /// Text in `backup_seed_list_of`. `{}`: `id + 1`.
    backup_seed_list_of_share: "Share {}",
    // ----- place_name (faraday-core/src/lib.rs) -----
    /// Text in `place_name`. `{}`: `p + 1`.
    place_name_place: "Place {}",
    // ----- plan_store (faraday-core/src/lib.rs) -----
    /// Text in `plan_store`. `{}`: `p + 1`.
    plan_store_place: "Place {}",
    /// Text in `plan_store`. `{}`: `self.medium.cap()`.
    plan_store_of_files: "{} of files",
    /// Text in `plan_store`.
    plan_store_watch_only: "Watch-only software",
    /// Text in `plan_store`.
    plan_store_on_its_own: "On its own device",
    /// Text in `plan_store`. `{}`: `name`.
    plan_store_the_plan: "The plan is in {}",
    // ----- public_pictures (faraday-core/src/lib.rs) -----
    /// Text in `public_pictures`. `{}`: `j + 1`, `k`.
    public_pictures_part: "Part {} of {}",
    /// Text in `public_pictures`.
    public_pictures_key: "Key",
    /// Text in `public_pictures`.
    public_pictures_keys: "Keys",
    /// Text in `public_pictures`. `{}`: `sum`.
    public_pictures_descriptor: "Descriptor checksum {}",
    /// Text in `public_pictures`.
    public_pictures_public_watch: "Public: watch only, spends nothing",
    /// Text in `public_pictures`. `{}`: `i + 1`, `plan.len()`, `m`.
    public_pictures_share: "Share {} of {} · any {} rebuild the wallet",
    /// Text in `public_pictures`. `{}`: `held.join(" · ")`.
    public_pictures_holds: "Holds {}",
    /// Text in `public_pictures`. `{}`: `off.join(" · ")`.
    public_pictures_leaves: "Leaves off {}",
    /// Text in `public_pictures`.
    public_pictures_not_wallet: "Not a wallet on its own",
    /// Text in `public_pictures`.
    public_pictures_public: "Public: spends nothing",
    // ----- public_out (faraday-core/src/lib.rs) -----
    /// Toast in `public_out`. `{}`: `k`.
    public_out_files: "{} files",
    // ----- backup_files_make (faraday-core/src/lib.rs) -----
    /// Toast in `backup_files_make`. `{}`: `k`.
    backup_files_make_files: "{} files",
    // ----- public_made (faraday-core/src/lib.rs) -----
    /// Toast in `public_made`. `{}`: `e`.
    public_made_no_pdf: "No PDF: {}",
    // ----- wallet_key (faraday-core/src/lib.rs) -----
    /// Error in `wallet_key`.
    wallet_key_that_wallet: "That wallet is no longer loaded",
    /// Error in `wallet_key`.
    wallet_key_this_wallet_s: "This wallet's keys are not account keys",
    /// Error in `wallet_key`.
    wallet_key_this_wallet: "This wallet has no such key",
    /// Error in `wallet_key`.
    wallet_key_that_key_names: "That key names no master",
    /// Error in `wallet_key`.
    wallet_key_that_key: "That key is not held here",
    // ----- wallet_key_record (faraday-core/src/lib.rs) -----
    /// Error in `wallet_key_record`.
    wallet_key_record_bip: "BIP 129 covers wsh and sh(wsh) multisig keys",
    // ----- create_key_text (faraday-core/src/lib.rs) -----
    /// Error in `create_key_text`.
    create_key_text_nothing: "nothing being created",
    /// Error in `create_key_text`.
    create_key_text_not_held: "That key is not held here",
    /// Error in `create_key_text`.
    create_key_text_a_chosen: "a chosen key is no longer loaded",
    // ----- create_keys (faraday-core/src/lib.rs) -----
    /// Error in `create_keys`.
    create_keys_nothing: "nothing being created",
    /// Error in `create_keys`. `{}`: `i + 1`.
    create_keys_key_has: "Key {} has no key yet",
    /// Text in `create_keys`. `{}`: `i + 1`, `self.medium.a()`, `i + 1`.
    create_keys_key_waits: "Key {} waits for its cosigner's xpub file. Copy it in on {} visit, then choose it for Key {} under Keys",
    /// Error in `create_keys`.
    create_keys_a_chosen: "a chosen key is no longer loaded",
    /// Error in `create_keys`.
    create_keys_the_same: "The same key is in two slots",
    // ----- restore_refusal (faraday-core/src/lib.rs) -----
    /// Text in `restore_refusal`.
    restore_refusal_every: "Every slot is filled",
    // ----- restore_described_make (faraday-core/src/lib.rs) -----
    /// Text in `restore_described_make`. `{}`: `open`, `if open == 1 { EN.restore_described_m…`.
    restore_described_make_still: "{} {} still empty",
    /// Text in `restore_described_make`.
    restore_described_make_slot: "slot",
    /// Text in `restore_described_make`.
    restore_described_make_slots: "slots",
    // ----- restore_from_vault (faraday-core/src/lib.rs) -----
    /// Text in `restore_from_vault`.
    restore_from_vault_wallet: "Wallet",
    /// Text in `restore_from_vault`. `{}`: `keys`, `if keys == 1 { EN.restore_from_vault_…`, `source`.
    restore_from_vault_count: "{} {} from {}",
    /// Text in `restore_from_vault`.
    restore_from_vault_key: "key",
    /// Text in `restore_from_vault`.
    restore_from_vault_keys: "keys",
    // ----- create_pass_lock (faraday-core/src/lib.rs) -----
    /// Note in `create_pass_lock`.
    create_pass_lock_mismatch: "The two passphrases differ",
    /// Text in `create_pass_lock`. `{}`: `key.label`.
    create_pass_lock_passphrase: "{} · passphrase",
    // ----- create_make (faraday-core/src/lib.rs) -----
    /// Text in `create_make`. `{}`: `self.session.wallets.len() + 1`.
    create_make_new_wallet: "New wallet {}",
    /// Text in `create_make`.
    create_make_made_here: "Made here",
    // ----- make_pdf (faraday-core/src/lib.rs) -----
    /// Text in `make_pdf`. `{}`: `stem(name)`.
    make_pdf_pdf: "{}.pdf",
    /// Toast in `make_pdf`. `{}`: `e`.
    make_pdf_not_printed: "Not printed: {}",
    // ----- scanned (faraday-core/src/lib.rs) -----
    /// Text in `scanned`.
    scanned_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    /// Text in `scanned`.
    scanned_not_a_psbt: "Not a PSBT, a descriptor, an xpub or a signed message",
    /// Toast in `scanned`. `{}`: `name`.
    scanned_is_a_cosigner: "{} is a cosigner's key",
    /// Toast in `scanned`. `{}`: `name`, `slot + 1`.
    scanned_is_key: "{} is Key {}",
    /// Toast in `scanned`. `{}`: `name`.
    scanned_is_in_files: "{} is in Files",
    /// Toast in `family_arrived`. `{}`: the file's name.
    family_psbt_not_this_spend: "{} is for another transaction. Cancel this spend to sign it; the file is in Files",
    // ----- copy_scanned (faraday-core/src/lib.rs) -----
    /// Note in `copy_scanned`.
    copy_scanned_not_seedqr: "Not a SeedQR",
    // ----- read_code (faraday-core/src/lib.rs) -----
    /// Text in `read_code`.
    read_code_is_seedqr: "That is a SeedQR. Seeds go in through Add a key, not Files",
    /// Text in `read_code`.
    read_code_that_may: "That may be a CompactSeedQR. Seeds go in through Add a key, not Files",
    /// Text in `read_code`. `{}`: `ur_type`.
    read_code_ur_unknown: "ur:{} is not a kind this build reads",
    // ----- ur_feed (faraday-core/src/lib.rs) -----
    /// Note in `ur_feed`: a part of another UR message started it again.
    ur_feed_new_qr: "A new QR started; reading it from the start",
    /// Note in `ur_feed`: a part no decoder reads.
    ur_feed_not_a_part: "That QR is not a readable UR part",
    // ----- paste (faraday-core/src/scan_key.rs) -----
    /// Note in `pasted`: the clipboard held a seed's words.
    pasted_words: "Those are a seed's words. No seed is taken here",
    /// Note in `pasted`: the clipboard held a SeedQR's digits.
    pasted_seedqr: "That is a SeedQR. No seed is taken here",
    /// Note in `pasted`: the clipboard held a private key.
    pasted_private_key: "That is a private key. A private key is never pasted",
    /// Note in `pasted`: the clipboard held a codex32 string or a SLIP-39
    /// share.
    pasted_share: "That is a share of a key. No seed is taken here",
    /// Line in `scan_sheet` for two seconds after Paste found no text.
    pasted_nothing: "Nothing to paste",
    // ----- address_note (faraday-core/src/lib.rs) -----
    /// Text in `address_note`.
    address_note_that_address_fails: "That address fails its checksum: a character in it is wrong",
    /// Text in `address_note`. `{}`: `wallet::network_name(net)`.
    address_note_wrong_network: "That address is not on {}",
    /// Text in `address_note`.
    address_note_change: "change",
    /// Text in `address_note`.
    address_note_receive: "receive",
    /// Text in `address_note`. `{}`: `w.name`, `chain`, `index`.
    address_note_found: "That address is {}'s {} address {}",
    /// Text in `address_note`.
    address_note_no_wallet: "That is an address. Load a wallet to check whether it is yours",
    /// Text in `address_note`.
    address_note_that_address: "That address is not among the first 100 of any loaded wallet",
    // ----- open_qr (faraday-core/src/lib.rs) -----
    /// Toast in `open_qr`. `{}`: `e`.
    open_qr_cannot_make: "Cannot make a code: {}",
    // ----- code_png (faraday-core/src/lib.rs) -----
    /// Toast in `code_png`. `{}`: `e`.
    code_png_cannot_make: "Cannot make a code: {}",
    /// Toast in `code_png`. `{}`: `v.title`.
    code_png_is_more_than: "{} is more than one code",
    // ----- code_view (faraday-core/src/lib.rs) -----
    /// Text in `code_view`. `{}`: `w.name`.
    code_view_threshold: "{} · threshold record",
    /// Text in `code_view`. `{}`: `sum`.
    code_view_descriptor: "Descriptor checksum {}",
    /// Text in `code_view`. `{}`: `w.name`.
    code_view_wallet_descriptor: "{} · wallet descriptor",
    /// Error in `code_view`.
    code_view_this_wallet_has: "This wallet has no such file",
    /// Text in `code_view`. `{}`: `w.name`.
    code_view_multisig: "{} · multisig config",
    /// Text in `code_view`. `{}`: `w.name`.
    code_view_bsms_descriptor: "{} · BSMS descriptor record",
    /// Text in `code_view`.
    code_view_public_watch: "Public: watch only, spends nothing",
    /// Error in `code_view`.
    code_view_bip_129_covers: "BIP 129 covers wsh and sh(wsh) multisig keys",
    /// Text in `code_view`. `{}`: `k.fp`, `k.kind`.
    code_view_xpub_kind: "Xpub {} · {}",
    /// Error in `code_view`.
    code_view_that_wallet: "That wallet is no longer loaded",
    /// Error in `code_view`.
    code_view_this_wallet_s: "This wallet's keys are not account keys",
    /// Error in `code_view`.
    code_view_this_wallet: "This wallet has no such key",
    /// Text in `code_view`.
    code_view_no_origin: "no origin",
    /// Text in `code_view`. `{}`: `fp`, `Session::shape(w)`.
    code_view_key: "Key {} · {}",
    /// Text in `code_view`. `{}`: `fp`.
    code_view_xpub: "Xpub {}",
    /// Text in `code_view`.
    code_view_bip_129_key: "BIP 129 key record, signed by the key",
    /// Text in `code_view`.
    code_view_public_spends: "Public: spends nothing",
    /// Text in `code_view`. `{}`: `k.fp`.
    code_view_key_bsms: "Key {} · BSMS record",
    /// Error in `code_view`.
    code_view_no_message: "No message is signed",
    /// Error in `code_view`.
    code_view_the_message: "The message is not signed yet",
    /// Text in `code_view`. `{}`: `sig.address`.
    code_view_signed_by_address: "Signed by {}",
    /// Text in `code_view`.
    code_view_public_proves: "Public: proves who signed, spends nothing",
    /// Text in `code_view`.
    code_view_signed_message: "Signed message",
    // ----- wallet_label (faraday-core/src/lib.rs) -----
    /// Text in `wallet_label`.
    wallet_label_key: "Key",
    /// Text in `wallet_label`.
    wallet_label_keys: "Keys",
    // ----- entry_add (faraday-core/src/lib.rs) -----
    /// Toast in `entry_add`. `{}`: `fp_text(fp)`.
    entry_add_key_added: "Key {} added",
    // ----- load_key (faraday-core/src/lib.rs) -----
    /// Toast in `load_key`. `{}`: `fp_text(fp)`.
    load_key_key_added: "Key {} added",
    // ----- qr_read (faraday-core/src/lib.rs) -----
    /// Text in `qr_read`. `{}`: `name`.
    qr_read_copied_no_qr: "Copied {}: no QR code read",
    /// Text in `qr_read`. `{}`: `name`, `got`, `self.medium.noun()`.
    qr_read_a_seedqr_copied: "{}: a SeedQR, copied in as {}; its key loads when the {} is removed",
    /// Toast in `qr_read`. `{}`: `got`.
    qr_read_is_in_files: "{} is in Files",
    /// Text in `qr_read`. `{}`: `name`, `got`.
    qr_read_read_into_files: "{} read into Files as {}",
    /// Text in `qr_read`.
    qr_read_read: "read",
    // ----- visit_inbox (faraday-core/src/lib.rs) -----
    /// Text in `visit_inbox`.
    visit_inbox_png_no_qr: "A PNG with no QR code read",
    /// Text in `visit_inbox`. `{}`: `screens::kind_name(k).to_lowercase()`.
    visit_inbox_png_kind: "A PNG of {}",
    /// Text in `visit_inbox`.
    visit_inbox_a_file: "A file Faraday does not read",
    /// Text in `visit_inbox`.
    visit_inbox_whoever_has_it_holds: "Whoever has it holds the key: spends its coins, or signs as it",
    /// Text in `visit_inbox`.
    visit_inbox_whoever_has_enough: "Whoever has enough parts holds the key",
    /// Text in `visit_inbox`.
    visit_inbox_vault_warning: "Whoever has it reads every password and code in it",
    /// Text in `visit_inbox`.
    #[cfg(feature = "unreleased")]
    visit_inbox_whoever_has: "Whoever has it and a share's signature can work out that share",
    /// Text in `visit_inbox`.
    visit_inbox_whoever: "Whoever has it reads it",
    // ----- sign_here (faraday-core/src/lib.rs) -----
    /// Text in `sign_here`. `{}`: `waiting.join(", ")`.
    sign_here_backup_first: "{} waits for its backup: finish the backup first",
    /// Error in `sign_here`.
    sign_here_no_randomness: "No randomness from the system yet. Try again",
    /// Text in `sign_here`.
    sign_here_this_transaction: "This transaction was signed here before with other amounts for its inputs. It is refused.",
    /// Text in `sign_here`. `{}`: `want`, `if want == 1 { EN.sign_here_share } e…`.
    #[cfg(feature = "unreleased")]
    sign_here_choose_other: "Choose {} other {} under Signers",
    /// Text in `sign_here`.
    #[cfg(feature = "unreleased")]
    sign_here_share: "share",
    /// Text in `sign_here`.
    #[cfg(feature = "unreleased")]
    sign_here_shares: "shares",
    /// Toast in `sign_here`.
    sign_here_this_device: "This device's nonce is on the PSBT: carry it to the other signers",
    // ----- collect (faraday-core/src/lib.rs) -----
    /// Error in `collect`. `{}`: `name`.
    collect_has_nothing: "{} has nothing this copy lacks",
    /// Error in `collect`.
    collect_no_randomness: "No randomness from the system yet. Try again",
    // ----- primary (faraday-core/src/lib.rs) -----
    /// Error in `primary`. `{}`: `pos + 1`.
    primary_finish_step: "Finish step {} first",
    // ----- event (faraday-core/src/lib.rs) -----
    /// Line in `scan_sheet` after `CameraUnavailable`.
    event_no_camera: "No camera, or the camera was refused",
    // ----- lightning (faraday-core/src/lightning/mod.rs) -----
    /// Error in `lightning_resolve`.
    lightning_this_key: "This key has no BIP-39 seed",
    /// Error in `lightning_resolve`. `{}`: `words.len()`.
    lightning_an_aezeed: "An aezeed is 24 words; {} are typed",
    /// Error in `lightning_resolve`. `{}`: `k + 1`.
    lightning_word_not_in_list: "Word {} is not on the list",
    /// Text in `lightning_resolve`. `{}`: `e`.
    lightning_not_an_aezeed: "Not an aezeed for this passphrase: {}",
    /// Toast in `lightning_act`.
    lightning_no_vault: "No vault is open",
    /// Text in `lightning_act`. `{}`: `id`, `private.as_str()`.
    lightning_lightning: "Lightning node key\nnode id {}\nprivate key {}",
    /// Toast in `lightning_act`.
    lightning_the_node: "The node key is in the vault",
    /// Text in `lightning_act`.
    lightning_a_lightning: "A Lightning node's private key",
    /// Text in `lightning_act`.
    lightning_whoever_has: "Whoever has it is the node: its channels and their money",
    // ----- lightning (faraday-core/src/lightning/screen.rs) -----
    /// Text in `draw`.
    lightning_a_lightning_node: "A Lightning node's key, made the way ldk-node or LND makes it. Its public key is the node id, safe to share; its private key is the node, and goes into a vault.",
    /// Line in `header`.
    lightning_wallets: "Wallets",
    /// Title in `header`.
    lightning_lightning_node: "Lightning node key",
    /// Text in `body`. `{}`: `crate::wallet::key_line(fp, &k.label)`.
    lightning_ldk_node: "ldk-node · {}",
    /// Text in `body`.
    lightning_lnd_type: "LND · type an aezeed",
    /// Text in `body`.
    lightning_the_24_words: "The 24 words",
    /// Text in `body`. `{}`: `n`.
    lightning_word_count: "{} of 24",
    /// Line in `body`.
    lightning_passphrase: "Passphrase",
    /// Text in `body`.
    lightning_aezeed_lnd: "aezeed (LND's default)",
    /// Button in `body`.
    lightning_read_button: "Read it",
    /// Line in `body`.
    lightning_node_id: "Node id",
    /// Line in `body`.
    lightning_made: "Made",
    /// Line in `body`.
    lightning_private_key: "Private key",
    /// Text in `body`.
    lightning_hide: "Hide",
    /// Text in `body`.
    lightning_show: "Show",
    /// Text in `body`. `{}`: `v.name`.
    lightning_save_into: "Save into {}",
    /// Text in `body`.
    lightning_out_unprotected: "Out unprotected…",
    // ----- medium (faraday-core/src/medium.rs) -----
    /// Text in `noun`.
    medium_stick_inline: "stick",
    /// Text in `nouns`.
    medium_sticks_inline: "sticks",
    /// Text in `a`.
    medium_a_stick_inline: "a stick",
    /// Text in `a`.
    medium_an_sd_card_inline: "an SD card",
    /// Text in `cap`.
    medium_stick: "Stick",
    /// Text in `caps`.
    medium_sticks: "Sticks",
    /// Text in `caps`.
    medium_sd_cards: "SD cards",
    /// Text in `a_cap`.
    medium_a_stick: "A stick",
    /// Text in `full`.
    medium_usb_stick: "USB stick",
    /// Text in `full`.
    medium_sd_card: "SD card",
    /// Text in `a_full_cap`.
    medium_a_usb_stick: "A USB stick",
    /// Text in `a_full_cap`.
    medium_an_sd_card: "An SD card",
    /// Text in `from_box`.
    medium_from_the_stick: "From the stick",
    /// Text in `from_box`.
    medium_from_the_sd_card: "From the SD card",
    /// Text in `for_box`.
    medium_for_the_stick: "For the stick",
    /// Text in `for_box`.
    medium_for_the_sd_card: "For the SD card",
    /// Text in `on_box`.
    medium_on_box_stick: "On the stick",
    /// Text in `on_box`.
    medium_on_box_sd_card: "On the SD card",
    /// Text in `on_this_box`.
    medium_on_this_stick: "On this stick",
    /// Text in `on_this_box`.
    medium_on_this_sd_card: "On this SD card",
    /// Text in `from_the`.
    medium_from_the_stick_inline: "from the stick",
    /// Text in `from_the`.
    medium_from_the_sd: "from the SD card",
    /// Text in `on_the`.
    medium_on_the_stick: "on the stick",
    /// Text in `on_the`.
    medium_on_the_sd_card: "on the SD card",
    /// Text in `for_the`.
    medium_for_the_stick_inline: "for the stick",
    /// Text in `for_the`.
    medium_for_the_sd_card_inline: "for the SD card",
    /// Text in `visit`.
    medium_stick_visit: "Stick visit",
    /// Text in `visit`.
    medium_sd_card_visit: "SD card visit",
    /// Text in `upgrade`.
    medium_upgrade_a_faraday_stick: "Upgrade a Faraday stick",
    /// Text in `upgrade`.
    medium_upgrade_a_faraday: "Upgrade a Faraday SD card",
    /// Text in `boot`.
    medium_boot_stick: "Boot stick",
    /// Text in `boot`.
    medium_boot_sd_card: "Boot SD card",
    // ----- paper (faraday-core/src/paper.rs) -----
    /// Toast in `paper_xor`.
    paper_no_words_to_split: "This key has no words to split",
    /// Toast in `paper_codex32`.
    paper_no_randomness: "No randomness from the system yet. Try again",
    /// Toast in `paper_codex32`. `{}`: `e`.
    paper_codex32: "codex32: {}",
    // ----- pdf (faraday-core/src/pdf.rs) -----
    /// Error in `fields`.
    pdf_not_a_faraday_sheet: "not a Faraday sheet",
    /// Text in `sheet`. `{}`: `other`.
    pdf_a_sheet_of_kind: "a sheet of kind {} is not one this build prints",
    /// Text in `wallet_sheet`. `{}`: `name`.
    pdf_wallet_backup: "{} · wallet backup",
    /// Line in `wallet_sheet`.
    pdf_xpubs_only_note: "Xpubs only. This sheet spends nothing; with the seeds it rebuilds the wallet.",
    /// Line in `wallet_sheet`.
    pdf_descriptor: "Descriptor",
    /// Line in `wallet_sheet`.
    pdf_xpubs: "Xpubs",
    /// Line in `wallet_sheet`.
    pdf_first_addresses: "First addresses",
    /// Text in `wallet_sheet`.
    pdf_written_on_label: "Written on",
    /// Text in `wallet_sheet`.
    pdf_kept_at_label: "Kept at",
    /// Text in `wallet_sheet`.
    pdf_goes_with_seed: "Goes with seed of key",
    /// Line in `wallet_sheet`.
    pdf_made_by_faraday_check: "Made by Faraday. Check the first address against your wallet software before relying on this sheet.",
    /// Text in `envelopes_sheet`. `{}`: `name`.
    pdf_envelopes: "{} · envelopes",
    /// Text in `share_sheet`.
    pdf_wallet: "Wallet",
    /// Text in `share_sheet`. `{}`: `name`, `part`.
    pdf_split_backup_sheet: "{} · split backup, sheet {}",
    /// Text in `share_sheet`. `{}`: `quorum`.
    pdf_part_of_the_wallet: "Part of the wallet's descriptor: xpubs only, it spends nothing. Sheets enough to hold every xpub ({} wallet) rebuild it.",
    /// Line in `share_sheet`.
    pdf_made_by_faraday: "Made by Faraday. Scan the code into Faraday, or type the lines, one sheet at a time.",
    /// Error in `blank_sheet`.
    pdf_template_word_count: "the template is for 12 or 24 words",
    /// Text in `blank_sheet`. `{}`: `words`.
    pdf_seed_sheet_words: "Seed sheet · {} words",
    /// Line in `blank_sheet`.
    pdf_written_by_hand: "Written by hand. Never photographed or scanned by any other device once filled in.",
    /// Text in `blank_sheet`. `{}`: `get(f, "network").unwrap_or("mainnet")`.
    pdf_network: "Network: {}",
    /// Text in `blank_sheet`.
    pdf_fingerprint: "Fingerprint",
    /// Text in `blank_sheet`.
    pdf_first_address: "First address",
    /// Line in `blank_sheet`.
    pdf_there_is_no_line: "There is no line for a passphrase. Written beside the words, it would stop being a second factor.",
    /// The blank template's footer, on every page.
    pdf_print_actual_size: "Print at 100% (actual size).",
    /// Line in `blank_sheet`.
    pdf_places: "Places",
    /// Line in `blank_sheet`.
    pdf_place: "Place",
    /// Line in `blank_sheet`.
    pdf_holds: "holds",
    // ----- codex32 blank templates (faraday-core/src/pdf/codex32.rs) -----
    /// Error in `sheet`: a codex32 template of another length.
    pdf_c32_length: "the codex32 template is for 48, 74 or 127 characters",
    /// Title of the share card. `{}`: the length in characters.
    pdf_c32_card: "Codex32 share · {} characters",
    /// Line on the share card.
    pdf_c32_hand: "Write one character in each box. Write 0, 7, S and Z with a slash.",
    /// Label under box 4 of the share card.
    pdf_c32_threshold: "Threshold",
    /// Label under boxes 5 to 8 of the share card.
    pdf_c32_identifier: "Identifier",
    /// Label under box 9 of the share card.
    pdf_c32_index: "Share index",
    /// Label over the alphabet at the foot of the share card.
    pdf_c32_alphabet: "The 32 characters",
    /// Caption of the Codex32QR grid on the share card.
    pdf_c32_grid: "Codex32QR · the black squares are printed already",
    /// Title of the Codex32QR sheet.
    pdf_c32qr_title: "Codex32QR · 48 characters",
    /// Line on the Codex32QR sheet.
    pdf_c32qr_hand: "Fill each square the device shows black, block by block: columns 1 to 6, rows A to F.",
    /// Title of the page of four Codex32QR grids.
    pdf_c32qr_grids_title: "Codex32QR · four grids",
    /// Title of the dice worksheet, the booklet's (two pages). `{}`: the
    /// page, the pages.
    pdf_c32_dice: "Dice de-biasing worksheet · page {} of {}",
    /// The printed dice worksheet's credit, at the foot of each page
    /// (owner, 2026-10-24; never on a screen).
    pdf_c32_dice_credit: "Dice worksheet after the codex32 booklet, © 2022 Blockstream, MIT.",
    /// Title of the checksum worksheet. `{}`: the length in characters.
    pdf_c32_checksum: "Checksum worksheet · {} characters",
    /// A worksheet title with its page. `{}`: the title, the page, the pages.
    pdf_c32_page: "{} · page {} of {}",
    /// Key of the checksum worksheet: the bold boxes.
    pdf_c32_key_given: "Given",
    /// Key of the checksum worksheet: the pink boxes.
    pdf_c32_key_checksum: "Checksum",
    /// Key of the checksum worksheet: the grey rows.
    pdf_c32_key_lookup: "Row of the checksum table",
    /// Title of a translation worksheet. `{}`: k, the length in characters.
    pdf_c32_translation: "Translation worksheet · k = {} · {} characters",
    /// Label of an input string on the translation worksheet. `{}`: its index.
    pdf_c32_share: "Share {}",
    /// Label over the translation symbol's box.
    pdf_c32_symbol: "Symbol",
    /// Label of the sum row on the translation worksheet.
    pdf_c32_new_share: "New share",
    /// Title of a derivation table. `{}`: k.
    pdf_c32_derivation: "Derivation table · k = {}",
    /// Title of the page of the principal tables.
    pdf_c32_tables: "Tables",
    /// Name of the Addition table.
    pdf_c32_addition: "Addition",
    /// Name of the Translation table.
    pdf_c32_translation_table: "Translation",
    /// Name of the Recovery table.
    pdf_c32_recovery: "Recovery",
    /// Name of the Fusion table.
    pdf_c32_fusion: "Fusion",
    /// Title of a page of the checksum table. `{}`: the page, the pages.
    pdf_c32_checksum_table: "Checksum table · {} of {}",
    // ----- plan (faraday-core/src/plan/mod.rs) -----
    /// Text in `vault_name`. `{}`: `v + 1`, `names.join(", ")`.
    plan_vault_names: "Vault {} · {}",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_words: "Seed {}: words",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_seedqr: "Seed {}: SeedQR",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_passphrase_label: "Passphrase of {}",
    /// Text in `label`.
    plan_wallet_sheet: "Wallet sheet",
    /// Text in `label`. `{}`: `j + 1`, `shape.keys`.
    plan_share_count: "Share {} of {}",
    /// Text in `label`. `{}`: `medium.cap()`, `v + 1`.
    plan_with_vault: "{} with Vault {}",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed: "Seed {}",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_with_passphrase: "Seed {} with its passphrase",
    /// Text in `label`.
    plan_wallet_description: "Wallet description",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seed_file: "Seed {}: file",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_seedqr_file: "Seed {}: SeedQR file",
    /// Text in `label`. `{}`: `seed(i)`.
    plan_passphrase_file: "Passphrase of {}: file",
    /// Text in `label`. `{}`: the form, `form_name`.
    plan_wallet_file: "Wallet description · {}",
    /// Text in `label`. `{}`: the share, how many, the form.
    plan_share_file: "Share {} of {} · {}",
    /// Text in `label`.
    plan_software_file: "Watch-only software file",
    /// A public file's form, in `form_name`.
    plan_form_text_file: "Text file",
    /// A public file's form, in `form_name`.
    plan_form_qr_picture: "QR picture",
    /// A node's thing on the chart. `{}`: the key's number.
    glance_key_seedqr_file: "Key {} SeedQR file",
    /// A node's thing on the chart. `{}`: the key's number.
    glance_passphrase_file: "Passphrase {} file",
    /// Text in `text`.
    plan_yes_with_the_vault: "Yes, with the vault's passphrase",
    /// Text in `text`.
    plan_yes: "Yes",
    /// Text in `text`.
    plan_only_with_passphrase: "Only with the vault's passphrase",
    /// Text in `text`.
    plan_no: "No",
    /// Text in `CHECK_LINES`.
    plan_any_one_place: "Any one place lost: the rest rebuild the wallet",
    /// Text in `CHECK_LINES`.
    plan_one_place_found_can: "One place found: can spend",
    /// Text in `CHECK_LINES`.
    plan_one_place_found: "One place found: sees the balance",
    /// Text in `text`.
    plan_can_spend: "Can spend",
    /// Text in `text`.
    plan_sees_the_balance_can: "Sees the balance · can spend with the vault's passphrase",
    /// Text in `text`.
    plan_can_spend_passphrase: "Can spend with the vault's passphrase",
    /// Text in `text`.
    plan_sees_balance: "Sees the balance",
    /// Text in `text`.
    plan_sees_the_balance: "Sees the balance with the vault's passphrase",
    /// Text in `text`.
    plan_nothing: "Cannot see balance",
    /// Text in `lost_why`.
    plan_at_no_place: "at no place",
    /// Text in `lost_why`. `{}`: `v + 1`, `noun`, `stick_text(*v)`.
    plan_lost_vault_medium: "Vault {}, whose {} is {}",
    /// Text in `lost_why`. `{}`: `w`.
    plan_lost_in: "in {}",
    /// Text in `lost_why`. `{}`: `w`.
    plan_lost_on: "on {}",
    /// Text in `lost_why`. `{}`: `w`.
    plan_lost_at: "at {}",
    /// Text in `lost_why`. `{}`: `name_at(gone, name, noun)`.
    plan_lost_with: "with {}",
    /// Text in `lost_why`. `{}`: `seed`, `line`.
    plan_seed_is_kept_only: "Seed {} is kept only {}",
    /// Text in `lost_why`. `{}`: `seed`, `name_at(gone, name, noun)`.
    plan_lost_passphrase_only: "The passphrase of {} is kept only with {}",
    /// Text in `lost_why`. `{}`: `name_at(gone, name, noun)`.
    plan_without_the_rest: "Without {}, the rest do not rebuild the wallet",
    /// Text in `name_at`. `{}`: `v + 1`.
    plan_vault: "Vault {}",
    /// Text in `name_at`. `{}`: `noun`.
    plan_the_of_files: "the {} of files",
    /// Text in `name_at`.
    plan_the_software: "the software",
    /// Text in `name_at`.
    plan_the_devices: "the devices",
    /// Text in `kept_nowhere`. `{}`: `seed.name`.
    plan_seed_is_kept_nowhere: "Seed {} is kept nowhere",
    /// Text in `kept_nowhere`. `{}`: `seed.name`.
    plan_the_passphrase: "The passphrase of {} is kept nowhere",
    /// A thing's tag on the map, as `Tag::Secret` shows.
    plan_tag_secret: "secret",
    /// A thing's tag on the map, as `Tag::Sealed` shows.
    plan_tag_sealed: "sealed",
    /// A thing's tag on the map, as `Tag::Public` shows.
    plan_tag_public: "public",
    /// A thing's mark on the chart.
    plan_mark_lost: "lost",
    /// A thing's mark on the chart.
    plan_mark_exposed: "exposed",
    /// A thing's mark on the chart: a SeedQR dropped for the words.
    plan_mark_drawn_in_part: "drawn in part",
    // ----- restore (faraday-core/src/restore.rs) -----
    /// Error in `merge`.
    restore_not_a_multisig: "not a multisig config",
    /// Error in `merge`.
    restore_these_shares: "these shares are of different wallets",
    /// Error in `merge`.
    restore_no_shares: "no shares",
    // ----- scan_key (faraday-core/src/scan_key.rs) -----
    /// Text in `not_a_wallet_key`. `{}`: `fp_text(fp)`, `name`.
    scan_key_is_not_one: "{} is not one of {}'s keys",
    /// Text in `wallet_key_lines`. `{}`: `n + 1`, `fp_text(fp)`.
    scan_key_key: "Key {} · {}",
    /// Note in `wallet_key_scanned`.
    scan_key_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    /// Note in `wallet_key_scanned`. `{}`: `n + 1`.
    scan_key_key_already: "Key {} · already here",
    /// Toast in `wallet_key_said`. `{}`: `n + 1`, `short`.
    scan_key_key_added_more: "Key {} added · {} more",
    /// Toast in `wallet_key_said`. `{}`: `n + 1`.
    scan_key_key_added: "Key {} added · Can spend",
    // ----- notices (faraday-core/src/screens/mod.rs) -----
    /// Text in `notices`. `{}`: `app.medium.noun()`.
    notices_remove_medium: "Remove the {} to keep working",
    // ----- nav (faraday-core/src/nav/screen.rs) -----
    /// The rail's foot and the ⋮ menu's row: power off.
    nav_power_off: "Power off",
    /// Title of the top app bar's ⋮ menu on a compact window.
    nav_more: "More",
    /// The ⋮ menu's way out.
    nav_close: "Close",
    /// A destination's badge past 999 things waiting (Material's most).
    nav_badge_many: "999+",
    // ----- sidebar (faraday-core/src/screens/sidebar.rs) -----
    /// Text in `sidebar`.
    sidebar_home: "Home",
    /// Text in `sidebar`.
    sidebar_vaults: "Vaults",
    /// Text in `sidebar`.
    sidebar_tools: "Tools",
    /// Text in `sidebar`.
    sidebar_transfer: "Transfer",
    /// Text in `sidebar`.
    sidebar_settings: "Settings",
    /// Text in `sidebar`. `{}`: `v`.
    sidebar_open_n: "{} open",
    /// Row in `sidebar`.
    sidebar_learn: "Learn",
    /// Text in `sidebar`.
    sidebar_bring_in_button: "Bring in",
    /// Text in `sidebar`.
    sidebar_open: "Open",
    /// Text in `sidebar`.
    sidebar_work: "Work",
    /// Text in `sidebar`.
    sidebar_write_out: "Write out",
    /// Text in `sidebar`.
    sidebar_passphrase: "passphrase",
    /// Text in `sidebar`. `{}`: `total - shown`.
    sidebar_more: "+ {} more",
    /// Text in `sidebar`.
    sidebar_keys: "Keys",
    /// Text in `sidebar`. `{}`: `m`, `n`.
    sidebar_count: "{} of {}",
    /// Text in `sidebar`.
    sidebar_single_key: "single key",
    /// Text in `sidebar`.
    sidebar_wallets: "Wallets",
    /// Line in `sidebar`.
    sidebar_files: "Files",
    /// Text in `sidebar`.
    sidebar_import_waiting: "import waiting",
    /// Text in `sidebar`.
    sidebar_none: "none",
    /// Text in `sidebar`. `{}`: `k`.
    sidebar_attached: "{} attached",
    /// Button in `sidebar`.
    sidebar_lock: "Lock",
    /// A flow in progress's name, in `sidebar` and the small Home
    /// (`in_progress`).
    sidebar_flow_create: "Make a new wallet",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_restore: "Restore a wallet",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_backup: "Back up a wallet",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_spend: "Spend",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_message: "Sign a message",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_vault: "Create a vault",
    /// A wallet's backup flow in progress (`in_progress`). `{}`: the
    /// wallet's name.
    sidebar_flow_backup_wallet: "Backup of {}",
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
    /// Text in `home_ways`.
    home_ways_create_vault: "Create a vault",
    /// Text in `home_ways`.
    home_ways_none_yet: "None yet",
    /// Text in `home_ways`.
    home_ways_vaults: "Vaults",
    /// Text in `home_ways`. `{}`: `vault_files`.
    home_ways_locked: "{} locked",
    /// Text in `home_ways`. `{}`: `open`.
    home_ways_open: "{} open",
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
    // ----- start (faraday-core/src/screens/wallets_start.rs) -----
    /// Title in `start`.
    start_wallets: "Wallets",
    /// Text in `start`. `{}`: `app.medium.a()`.
    start_no_wallet_loaded: "No wallet is loaded. Create one with new keys or keys you load here, or restore one from its backups: {}, a vault or paper.",
    /// Text in `start`.
    start_start_from_what: "Start from what you have in front of you. Every way in ends at the same place: the wallet, with each of its keys marked as here, elsewhere or missing, and the one thing still to do.",
    /// Section label in `start`.
    start_your_wallets: "Your wallets",
    /// Section label in `start`.
    start_keys_without: "Keys without a wallet",
    /// Text in `start`.
    start_make_a_new_wallet: "Make a new wallet",
    /// Text in `start`.
    start_create_line: "Single key or multisig",
    /// Text in `start`.
    start_restore_an_existing: "Restore an existing wallet",
    /// Text in `start`.
    start_from_its_backups: "From its backups",
    /// Text in `start`.
    start_spend_backup: "Spend from a backup, step by step",
    /// Text in `start`.
    start_a_stick_words: "A stick, words, or paper",
    /// Text in `start`.
    start_add_a_key: "Add a key",
    /// Text in `start`.
    start_keys_first_wallets: "Keys first, wallets later",
    /// Text in `start`. `{}`: `files[i].name`.
    start_is_in_files_locked: "{} is in Files, locked",
    /// Button in `start`.
    start_unlock: "Unlock",
    // ----- nonce_check (faraday-core/src/screens/spend.rs) -----
    /// Text in `nonce_check`. `{}`: `check.rows.len()`, `if check.rows.len() == 1 { EN.nonce_c…`, `here`.
    nonce_check_nonce_check_valid: "Nonce check · {} {} valid · {} recomputed here",
    /// Text in `nonce_check`.
    nonce_check_signature: "signature",
    /// Text in `nonce_check`.
    nonce_check_signatures: "signatures",
    /// Text in `nonce_check`.
    nonce_check_nonce_check: "Nonce check · a signature is wrong",
    /// Text in `nonce_check`. `{}`: `reuse.name()`, `inputs.join(", ")`.
    nonce_check_one_nonce: "{}: one nonce on inputs {} · this gives the key away",
    /// Text in `nonce_check`. `{}`: `r.input`.
    nonce_check_input: "Input {}",
    /// Text in `nonce_check`.
    nonce_check_invalid: "Invalid: not this key's over this transaction",
    /// Text in `nonce_check`. `{}`: `why`.
    nonce_check_not_checked: "Not checked · {}",
    /// Text in `nonce_check`. `{}`: `mode`.
    nonce_check_valid_nonce: "Valid · nonce recomputed, {}",
    /// Text in `nonce_check`.
    nonce_check_wrong_nonce: "Valid · not the nonce this key makes: do not broadcast",
    /// Text in `nonce_check`.
    nonce_check_valid_signed: "Valid · signed elsewhere, its nonce needs that device's key",
    /// Text in `nonce_check`. `{}`: `rest.len()`, `if ok { EN.nonce_check_none_wrong } e…`.
    nonce_check_more: "{} more {}",
    /// Text in `nonce_check`.
    nonce_check_none_wrong: "· none wrong",
    /// Text in `nonce_check`.
    nonce_check_one_wrong: "· one is wrong",
    /// Headline of `nonce_check` when some signatures could not be checked. `{}`: how many were not checked, how many there are, `nonce_check_signature(s)`.
    nonce_unchecked_head: "Nonce check · {} of {} {} not checked",
    // ----- inbox_panel (faraday-core/src/screens/files.rs) -----
    /// Line in `inbox_panel`.
    inbox_panel_wallets: "Wallets",
    /// Text in `inbox_panel`. `{}`: `fw.shape`, `fw.seeds_here`, `n`, `if n == 1 { EN.inbox_panel_key } else…`, `if fw.files.len() > 1 { format!(" · {…`.
    inbox_panel_of_here: "{} · {} of {} {} here{}",
    /// Text in `inbox_panel`, after a space. `{}`: `fw.files.len()`.
    inbox_panel_files_agree: "· {} files say it",
    /// Text in `inbox_panel`, after a space.
    inbox_panel_from_split_sheets: "· from split sheets",
    /// Text in `inbox_panel`.
    inbox_panel_key: "key",
    /// Text in `inbox_panel`.
    inbox_panel_keys: "keys",
    /// Text in `inbox_panel`.
    inbox_panel_ready_to_sign: "Ready to sign once loaded",
    /// Text in `inbox_panel`. `{}`: `wallets.len() - ROWS`.
    inbox_panel_more_loaded: "+ {} more, loaded with these",
    /// Text in `inbox_panel`.
    inbox_panel_nothing: "Nothing chosen",
    /// Text in `inbox_panel`.
    inbox_panel_load_the_wallet: "Load the wallet with its seeds",
    /// Text in `inbox_panel`.
    inbox_panel_load_wallet: "Load the wallet",
    /// Text in `inbox_panel`. `{}`: `n`.
    inbox_panel_load_wallets_seeds: "Load {} wallets with their seeds",
    /// Text in `inbox_panel`. `{}`: `n`.
    inbox_panel_load_wallets: "Load {} wallets",
    /// Text in `inbox_panel`. `{}`: `app.medium.noun()`.
    inbox_panel_keys_load: "Keys load once the {} is out",
    /// Line in `inbox_panel`.
    inbox_panel_loose_keys: "Keys with no wallet",
    /// Text in `inbox_panel`.
    inbox_panel_make_wallet: "Make it a wallet",
    /// Text in `inbox_panel`. `{}`: `potential.len() - ROWS`.
    inbox_panel_more: "+ {} more",
    /// Line in `inbox_panel`.
    inbox_panel_loose_xpubs: "Xpubs with no wallet",
    /// Text in `inbox_panel`.
    inbox_panel_no_origin: "no origin",
    /// Text in `inbox_panel`.
    inbox_panel_make_watch_only: "Make it a watch-only wallet",
    /// Line in `inbox_panel`.
    inbox_panel_not_complete: "Not complete or sealed",
    // ----- potential (faraday-core/src/screens/sheets.rs) -----
    /// Text in `potential_sheet`. `{}`: `item.name`.
    potential_open: "Open {}",
    /// Text in `potential_sheet`. `{}`: `fp_text(fp)`.
    potential_a_watch_only_wallet: "A watch-only wallet from {}",
    /// Text in `potential_sheet`.
    potential_a_watch_only: "A watch-only wallet from an xpub",
    /// Text in `potential_sheet`. `{}`: `fp_text(fp)`.
    potential_a_wallet: "A wallet from {}",
    /// Text in `potential_sheet`. `{}`: `from.join(" · ")`.
    potential_sources: "From {}",
    /// Text in `potential_sheet`.
    potential_slip_39_passphrase: "SLIP-39 passphrase · none if left empty",
    /// Text in `potential_sheet`.
    potential_bip_39_passphrase: "BIP-39 passphrase · none if left empty",
    /// Text in `potential_sheet`. `{}`: `crate::family::kind_name(p.kind)`.
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
    // ----- catalog (faraday-core/src/screens/catalog.rs) -----
    /// Title in `catalog_screen`.
    catalog_tools: "Tools",
    /// Text in `catalog_screen`.
    catalog_find_a_tool: "Find a tool: a name, a BIP number",
    /// Line in `catalog_screen`.
    catalog_no_tool_matches: "No tool matches",
    // ----- decode_back (faraday-core/src/screens/decode.rs) -----
    /// Text in `decode_back`.
    decode_back_sign_transaction: "Sign a transaction",
    /// Text in `decode_back`.
    decode_back_spend: "Spend",
    /// Text in `decode_back`.
    decode_back_files: "Files",
    /// Text in `decode_back`.
    decode_back_wallets: "Wallets",
    /// Text in `decode_back`.
    decode_back_tools: "Tools",
    /// Text in `decode_back`.
    decode_back_back: "Back",
    // ----- decode (faraday-core/src/screens/decode.rs) -----
    /// Title in `decode_screen`.
    decode_decode_a_transaction: "Decode a transaction",
    /// Text in `decode_screen`. `{}`: `f`, `f as f64 / t.size.1.max(1) as f64`.
    decode_sat_sat_vb: "{} sat · {} sat/vB",
    /// Text in `decode_screen`.
    decode_fee_not_stated: "Not stated: the amounts going in are not here",
    /// Text in `decode_screen`.
    decode_transaction: "Transaction id",
    /// Text in `decode_screen`.
    decode_witness_id: "Witness id",
    /// Text in `decode_screen`.
    decode_version: "Version",
    /// Text in `decode_screen`.
    decode_locktime: "Locktime",
    /// Text in `decode_screen`.
    decode_size: "Size",
    /// Text in `decode_screen`. `{}`: `t.size.0`, `t.size.1`, `t.size.2`.
    decode_bytes_vb_wu: "{} bytes · {} vB · {} WU",
    /// Text in `decode_screen`.
    decode_fee: "Fee",
    /// Text in `decode_screen`. `{}`: `t.inputs.len()`.
    decode_inputs: "Inputs · {}",
    /// Text in `decode_screen`.
    decode_no_witness: "No witness",
    /// Text in `decode_screen`. `{}`: `input.witness.len()`, `if input.witness.len() == 1 { EN.deco…`, `sizes.join(", ")`.
    decode_witness_bytes: "Witness · {} {} · {} bytes",
    /// Text in `decode_screen`.
    decode_item: "item",
    /// Text in `decode_screen`.
    decode_items: "items",
    /// Text in `decode_screen`. `{}`: `input.sequence`, `wit`, `if input.script_sig > 0 { format!(" ·…`.
    decode_sequence: "Sequence {} · {}{}",
    /// Text in `decode_screen`, after a space. `{}`: `input.script_sig`.
    decode_script_sig_bytes: "· scriptSig {} bytes",
    /// Text in `decode_screen`. `{}`: `t.outputs.len()`.
    decode_outputs: "Outputs · {}",
    /// Text in `decode_screen`. `{}`: `name`, `n`.
    decode_change_back: "Change · back to {} · 1/{}",
    /// Text in `decode_screen`. `{}`: `name`, `n`.
    decode_back_to_receive: "Back to {} · receive 0/{}",
    /// Text in `decode_screen`.
    decode_pays_from_loaded: "Pays out of the loaded wallets",
    /// Text in `decode_screen`.
    decode_hide_the_hex: "Hide the hex",
    /// Text in `decode_screen`.
    decode_show_the_hex: "Show the hex",
    /// Text in `decode_screen`.
    decode_show_as_qr: "Show as QR",
    // ----- missing_keys_line (faraday-core/src/screens/wallets.rs) -----
    /// Text in `missing_keys_line`. `{}`: `files[i].name`.
    missing_keys_line_locked: "{} is locked · it may hold them",
    /// Text in `missing_keys_line`. `{}`: `open_names.join(", ")`.
    missing_keys_line_not_open: "Not in {} · perhaps another vault, a paper backup or a SeedQR",
    /// Text in `missing_keys_line`. `{}`: `app.medium.a()`.
    missing_keys_line_elsewhere: "A vault on {}, a paper backup or a SeedQR may hold them",
    /// Button in `missing_keys_line`.
    missing_keys_line_unlock: "Unlock",
    // ----- wallet_rows (faraday-core/src/screens/wallets_start.rs) -----
    /// Line in `wallet_rows`.
    wallet_rows_none_loaded: "None loaded",
    /// Text in `wallet_rows`. `{}`: `here`, `m`.
    wallet_rows_can_spend: "Can spend · {} of {}",
    /// Text in `wallet_rows`. `{}`: `here`, `m`.
    wallet_rows_of_here: "{} of {} here",
    /// Text in `wallet_rows`.
    wallet_rows_watch_only: "Watch only",
    /// Text in `wallet_rows`. `{}`: `total - shown`.
    wallet_rows_more: "+ {} more",
    // ----- app (faraday-core/src/screens/) -----
    /// Text in `FROM_KEY`.
    app_wallet_from_key: "Make a wallet from this key",
    /// Text in `STEPS`.
    app_wallet: "Wallet",
    /// Text in `STEPS`.
    app_transaction: "Transaction",
    /// Text in `STEPS`.
    app_transaction_id: "Transaction id",
    /// Text in `STEPS`.
    app_signers: "Signers",
    /// Text in `STEPS`.
    app_sign: "Sign",
    /// Text in `STEPS`.
    app_step_signatures: "Signatures for this transaction",
    /// Text in `STEPS`.
    app_finish: "Finish",
    /// Text in `STEPS`.
    app_path: "Path",
    /// Text in `STEPS`.
    app_nonces: "Nonces",
    /// Text in `SOFTWARE_ROWS`.
    app_sparrow: "Sparrow",
    /// Text in `SOFTWARE_ROWS`.
    app_coldcard_keystone: "Coldcard, Keystone, Passport",
    /// Text in `SOFTWARE_ROWS`.
    app_nunchuk: "Nunchuk",
    /// Text in `SOFTWARE_ROWS`.
    app_bitcoin_core: "Bitcoin Core",
    /// Text in `SOFTWARE_ROWS`.
    app_not_sure: "Not sure",
    /// Text in `MSTEPS`.
    app_address: "Address",
    /// Text in `MSTEPS`.
    app_message: "Message",
    /// Text in `MSTEPS`.
    app_format: "Format",
    /// Text in `MSTEPS`.
    app_signature: "Signature",
    /// Text in `CSTEPS`.
    app_back_up: "Back up",
    /// Text in `RSTEPS`.
    app_kind: "Kind",
    /// Text in `RSTEPS`.
    app_quorum: "Quorum",
    /// Text in `RSTEPS`.
    app_description: "Description",
    /// Text in `RSTEPS`.
    app_keys: "Keys",
    /// Text in `RSTEPS`.
    app_check: "Check",
    /// Text in `RSTEPS`.
    app_done: "Done",
    /// Text in `PANIC_KEY`.
    app_panic_key_hint: "Super + S held 2 s: power off at once",
    // ----- loose_card (faraday-core/src/screens/wallets.rs) -----
    /// Text in `loose_card`. `{}`: `fp_text(*only)`.
    loose_card_key_is_loaded: "Key {} is loaded. No wallet uses it.",
    /// Text in `loose_card`. `{}`: `loose.len()`.
    loose_card_keys_are: "{} keys are loaded. No wallet uses them.",
    /// Text in `loose_card`.
    loose_card_add_another: "Add another key",
    /// Text in `loose_card`.
    loose_card_restore: "Restore an existing wallet",
    // ----- wallets (faraday-core/src/screens/wallets.rs) -----
    /// Text in `wallets`.
    wallets_signing: "Signing",
    /// Text in `wallets`.
    wallets_continue: "Continue",
    /// Text in `wallets`.
    wallets_in_files: "in Files",
    /// Text in `wallets`.
    wallets_review: "Review",
    /// Section label in `wallets`.
    wallets_wallets: "Wallets",
    /// Text in `wallets`. `{}`: `shape`.
    wallets_watch_only: "{} · watch only",
    /// Text in `wallets`. `{}`: `shape`, `here`, `m`.
    wallets_of_here: "{} · {} of {} here",
    /// Section label in `wallets`.
    wallets_keys_without: "Keys without a wallet",
    /// Button in `wallets`.
    wallets_restore_wallet: "Restore an existing wallet",
    /// Text in `wallets`. `{}`: `wl.name`.
    wallets_scan_key_title: "Scan a key for {}",
    /// Text in `wallets`.
    wallets_add_a_key: "Add a key",
    /// Text in `wallets`.
    wallets_no_wallet_loaded: "No wallet loaded",
    /// Line in `wallets`.
    wallets_enter_keeps: "Enter keeps it · Esc leaves it",
    /// Button in `wallets`.
    wallets_rename: "Rename",
    /// Text in `wallets`.
    wallets_show_wallet: "Show wallet QR",
    /// Text in `wallets`.
    wallets_back_up: "Back up",
    /// Text in `wallets`.
    wallets_sign_a_message: "Sign a message",
    /// Text in `wallets`.
    wallets_remove_from_session: "Remove from session",
    /// Text in `wallets`. `{}`: `here`, `m`.
    wallets_of_signatures: "{} of {} signatures possible here",
    /// Text in `wallets`.
    wallets_can_spend: "Can spend",
    /// Text in `wallets`.
    wallets_scan_a_key: "Scan a key",
    /// Line in `wallets`.
    wallets_at_a_glance: "At a glance",
    /// Line in `wallets`.
    wallets_first_receive: "First receive address · 0/0",
    // ----- silent_wallet_card (faraday-core/src/screens/wallets.rs) -----
    /// Line in `silent_wallet_card`.
    silent_wallet_card_key: "Key",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_here: "Here",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_not_here: "Not here",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_load: "Load from vault",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_add: "Add its key",
    /// Line in `silent_wallet_card`.
    silent_wallet_card_address_label: "Address",
    /// Line in `silent_wallet_card`.
    silent_wallet_card_labels: "Labels handed out",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_address: "Address, labels and payments",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_show: "Show wallet QR",
    /// Text in `silent_wallet_card`.
    silent_wallet_card_remove: "Remove from session",
    // ----- spend (faraday-core/src/screens/spend.rs) -----
    /// Line in `spend`.
    spend_no_transaction: "No transaction loaded",
    /// Button in `spend`.
    spend_open_files: "Open Files",
    /// Text in `spend`.
    spend_wallets: "Wallets",
    /// Text in `spend`.
    spend_sign_a_transaction: "Sign a transaction",
    // ----- spend_status (faraday-core/src/screens/spend.rs) -----
    /// Text in `spend_status`. `{}`: `id + 1`.
    #[cfg(feature = "unreleased")]
    spend_status_share: "share {}",
    /// Text in `spend_status`.
    #[cfg(feature = "unreleased")]
    spend_status_carry: "Carry file kept · the next share signs with it",
    /// Text in `spend_status`.
    #[cfg(feature = "unreleased")]
    spend_status_the_next: "The next share signs with the carry file",
    /// Text in `spend_status`. `{}`: `app.medium.for_the()`.
    spend_status_both_wait: "Both wait {}",
    /// Text in `spend_status`.
    spend_status_complete: "Complete",
    /// Text in `spend_status`. `{}`: `missing - 1`, `if missing == 2 { EN.spend_status_its…`.
    #[cfg(feature = "unreleased")]
    spend_status_sign_here_more: "Sign here · {} more at {} own place",
    /// Text in `spend_status`.
    #[cfg(feature = "unreleased")]
    spend_status_its_one: "its",
    /// Text in `spend_status`.
    #[cfg(feature = "unreleased")]
    spend_status_their: "their",
    /// Text in `spend_status`. `{}`: `missing - 1`.
    spend_status_sign_and_collect: "Sign here and collect {} more",
    /// Text in `spend_status`.
    spend_status_sign_here: "Sign here",
    /// Text in `spend_status`.
    #[cfg(feature = "unreleased")]
    spend_status_keep_carry: "Keep the carry file",
    /// Text in `spend_status`. `{}`: `app.medium.a()`.
    spend_status_insert: "Insert {} to write it",
    /// Text in `spend_status`.
    spend_status_open_files: "Open Files",
    /// Text in `spend_status`.
    spend_status_finish: "Finish",
    /// Text in `spend_status`. `{}`: `can_sign_here.join(", ")`.
    spend_status_sign_with_keys: "Sign with {}",
    // ----- spend_bar (faraday-core/src/screens/spend.rs) -----
    /// Text in `spend_bar`. `{}`: `count`, `needed`.
    spend_bar_count: "{} of {}",
    // ----- step_summary (faraday-core/src/screens/spend.rs) -----
    /// Text in `step_summary`.
    step_summary_no_loaded: "No loaded wallet matches",
    /// Text in `step_summary`.
    step_summary_addresses: "Addresses · Compare",
    /// Text in `step_summary`. `{}`: `btc(i.amount_to_others.to_sat())`, `i.fee_rate_sat_vb .map(|r| format!("{…`.
    step_summary_sends: "Sends {} BTC · fee {} sat/vB",
    /// Text in `step_summary`. `{}`: `here`, `needed`.
    step_summary_here_needed: "{} here · {} needed",
    /// Text in `step_summary`.
    step_summary_not_signed: "Not signed here yet",
    /// Text in `step_summary`. `{}`: `fps.join(", ")`.
    step_summary_signed_keys: "Signed with {}",
    /// Text in `step_summary`. `{}`: `s.signers.len()`, `needed`.
    step_summary_count: "{} of {}",
    /// Text in `step_summary`. `{}`: `have`, `rows.len()`.
    step_summary_of_nonces: "{} of {} nonces",
    /// Text in `step_summary`.
    step_summary_way_one: "way to spend",
    /// Text in `step_summary`.
    step_summary_ways_many: "ways to spend",
    /// Text in `step_summary`.
    step_summary_signed: "Signed PSBT and finished transaction",
    /// Text in `step_summary`.
    step_summary_not_finished: "Not finished yet",
    /// Text in `step_summary`.
    step_summary_not_reached: "Not reached yet",
    // ----- step (faraday-core/src/screens/spend.rs) -----
    /// Text in `step_body`. `{}`: `Session::shape(wl)`, `wl.source`.
    step_wallet_source: "{} · from {}",
    /// Line in `step_body`.
    step_no_loaded_wallet: "No loaded wallet matches this transaction. Change cannot be verified until its wallet is loaded.",
    /// Text in `step_body`.
    step_receive_0_0: "Receive 0/0",
    /// Text in `step_body`.
    step_receive_0_1: "Receive 0/1",
    /// Text in `step_body`.
    step_change_1_0: "Change 1/0",
    /// Line in `step_body`.
    step_no_wallet_to_show: "No wallet to show addresses from",
    /// Text in `step_body`.
    step_sends: "Sends",
    /// Text in `step_body`. `{}`: `if change { 1 } else { 0 }`, `index`.
    step_to_path: "to {}/{}",
    /// Text in `step_body`.
    step_change: "Change",
    /// Text in `step_body`. `{}`: `short(&o.address)`.
    step_to_address: "to {}",
    /// Text in `step_body`.
    step_unverified: "Unverified",
    /// Text in `step_body`.
    step_fee: "Fee",
    /// Text in `step_body`. `{}`: `thousands(i.fee.to_sat())`, `i.fee_rate_sat_vb .map(|r| format!("{…`.
    step_sat_sat_vb: "{} sat · {} sat/vB",
    /// Text in `step_body`.
    step_spends: "Spends",
    /// Text in `step_body`.
    step_amount_unchecked: "Amount unchecked",
    /// Text in `step_body`.
    step_unchanged_by_signing: "Unchanged by signing",
    /// Text in `step_body`.
    step_changes_when_signed: "Changes when signed",
    /// Text in `step_body`.
    step_txid: "Txid",
    /// Text in `step_body`.
    step_not_known_until: "Not known until signed",
    /// Chip in `step_body`.
    step_no_warnings: "No warnings",
    /// Line in `step_body`.
    step_input: "Input",
    /// Line in `step_body`.
    step_output: "Output",
    /// Text in `step_body`.
    step_hide_the_table: "Hide the table",
    /// Text in `step_body`.
    step_show_the_table: "Show the table",
    /// Text in `step_body`. `{}`: `k + 1`.
    #[cfg(feature = "unreleased")]
    step_share_n: "Share {}",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_signed: "Signed",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_signs_at_its_own: "Signs at its own place",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_not_signing: "Not signing",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_signs_next: "Signs next",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_choose: "Choose",
    /// Text in `step_body`. `{}`: `needed`, `t.ours.len()`, `s.others.len()`.
    #[cfg(feature = "unreleased")]
    step_needs_here_chosen: "Needs {} · {} here · {} chosen",
    /// Text in `step_body`. `{}`: `needed`, `t.signed.len()`.
    #[cfg(feature = "unreleased")]
    step_needs_signed: "Needs {} · {} signed",
    /// Text in `step_body`.
    step_load_from_vault: "Load from vault",
    /// Text in `step_body`.
    step_signs_here: "Signs here",
    /// Text in `step_body`.
    step_not_here: "Not here",
    /// Text in `step_body`. `{}`: `f.name`.
    step_unlock: "Unlock {}",
    /// Text in `step_body`.
    step_scan_a_key: "Scan a key",
    /// Text in `step_body`.
    step_collect_a_signature: "Collect a signature",
    /// Text in `step_body`. `{}`: `app.medium.noun()`.
    step_remove_the_first: "Remove the {} first",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_no_share_loaded: "No share of this wallet is loaded here",
    /// Text in `step_body`. `{}`: `want`, `if want == 1 { EN.step_share_inline } else…`.
    #[cfg(feature = "unreleased")]
    step_choose_other_under: "Choose {} other {} under Signers",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_share_inline: "share",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_shares: "shares",
    /// Text in `step_body`. `{}`: `names.join(", ")`.
    #[cfg(feature = "unreleased")]
    step_sign_as: "Sign as {}",
    /// Text in `step_body`. `{}`: `fps.join(", ")`.
    step_signed_keys: "Signed with {}",
    /// Line in `step_body`.
    step_no_signing_key: "No key in this session signs this transaction",
    /// Text in `step_body`. `{}`: `mine.join(", ")`.
    step_sign_with_keys: "Sign with {}",
    /// The Sign button in `step_body` where `step_sign_with_keys` is too
    /// wide for it and stands as a line above it.
    step_sign: "Sign",
    /// Text in `step_body`.
    step_input_inline: "input",
    /// Text in `step_body`.
    step_inputs: "inputs",
    /// Text in `step_body`. `{}`: `src`.
    step_from_inline: "from {}",
    /// Text in `step_body`.
    step_signed_here: "Signed here",
    /// Text in `step_body`.
    step_verified: "Verified",
    /// Text in `step_body`.
    step_not_needed: "Not needed",
    /// Line in `step_body`.
    step_no_cosigner_s: "No cosigner's copy in Files",
    /// Text in `step_body`.
    step_scan_a_copy: "Scan a copy",
    /// Text in `step_body`. `{}`: `id + 1`.
    #[cfg(feature = "unreleased")]
    step_share_n_inline: "share {}",
    /// Line in `step_body`.
    #[cfg(feature = "unreleased")]
    step_carry_file: "Carry file",
    /// Text in `step_body`. `{}`: `if next_ids.is_empty() { EN.step_the_…`.
    #[cfg(feature = "unreleased")]
    step_carry_holds_nonce: "For {} · holds its secret nonce until it signs",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_the_next_share: "the next share",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_kept: "Kept",
    /// Text in `step_body`.
    #[cfg(feature = "unreleased")]
    step_keep_button: "Keep it",
    /// Text in `step_body`. `{}`: `missing`, `if s.signers.is_empty() { "" } else {…`, `if missing == 1 { EN.step_signature }…`.
    step_needs: "Needs {}{} {}",
    /// Text in `step_body`, after a space.
    step_more: "more",
    /// Text in `step_body`.
    step_signature: "signature",
    /// Text in `step_body`.
    step_signatures: "signatures",
    /// Text in `step_body`.
    step_signed_psbt: "Signed PSBT",
    /// Text in `step_body`.
    step_for_the_wallet: "For the wallet that wrote it · not finalised",
    /// Text in `step_body`.
    step_finished_transaction: "Finished transaction",
    /// Text in `step_body`.
    step_ready_to_broadcast: "Ready to broadcast",
    /// Text in `step_body`.
    step_remove: "Remove",
    /// Text in `step_body`.
    step_make_again: "Make again",
    /// Text in `step_body`. `{}`: `short(&tx.compute_txid().to_string())`, `tx.vsize()`.
    step_vb: "{} · {} vB",
    /// Text in `step_body`.
    step_decode_button: "Decode it",
    /// Text in `step_body`.
    step_hide_the_raw_hex: "Hide the raw hex",
    /// Text in `step_body`.
    step_show_the_raw_hex: "Show the raw hex",
    /// Line in `step_body`.
    step_this_device: "This device",
    /// Text in `step_body`.
    step_nonce_here: "Nonce here",
    /// Text in `step_body`.
    step_waiting: "Waiting",
    /// Text in `step_body`. `{}`: `name`.
    step_add_source: "Add from {}",
    /// Text in `step_body`.
    step_show_as_qr: "Show as QR",
    /// Button in `step_body`.
    step_go_first_share: "Go first: share this device's nonce",
    /// Text in `step_body`. `{}`: `k + 1`.
    step_path: "Path {}",
    /// Text in `step_body`. `{}`: `slot`, `fp`.
    step_key: "key {} {}",
    /// Line in `step_body`.
    step_no_wait: "No wait",
    /// Text in `step_body`. `{}`: `l`.
    step_waits: "Waits {}",
    /// Text in `step_body`.
    step_this_transaction_s: "This transaction's sequence allows this path",
    /// Text in `step_body`.
    step_this_transaction: "This transaction's sequence does not allow this path",
    /// Text in `step_body`.
    step_continue: "Continue",
    // ----- spend_panel (faraday-core/src/screens/spend.rs) -----
    /// Line in `spend_panel`.
    spend_panel_this_transaction: "This transaction",
    /// Text in `spend_panel`.
    spend_panel_verified: "Verified",
    /// Text in `spend_panel`.
    spend_panel_unverified: "Unverified",
    /// Text in `spend_panel`.
    spend_panel_none: "None",
    /// Text in `spend_panel`.
    spend_panel_wallet: "Wallet",
    /// Text in `spend_panel`.
    spend_panel_not_loaded: "Not loaded",
    /// Text in `spend_panel`.
    spend_panel_sends: "Sends",
    /// Text in `spend_panel`. `{}`: `btc(i.amount_to_others.to_sat())`.
    spend_panel_btc: "{} BTC",
    /// Text in `spend_panel`.
    spend_panel_to_label: "To",
    /// Text in `spend_panel`.
    spend_panel_fee: "Fee",
    /// Text in `spend_panel`. `{}`: `thousands(i.fee.to_sat())`, `i.fee_rate_sat_vb .map(|r| format!("{…`.
    spend_panel_sat_sat: "{} sat · {} sat/vB",
    /// Text in `spend_panel`.
    spend_panel_change: "Change",
    /// Text in `spend_panel`.
    spend_panel_txid: "Txid",
    /// Line in `spend_panel`.
    spend_panel_signatures: "Signatures",
    /// Text in `spend_panel`. `{}`: `count`, `needed`.
    spend_panel_count: "{} of {}",
    // ----- question_title (faraday-core/src/screens/backup.rs) -----
    /// Text in `question_title`.
    question_title_software: "Software",
    // ----- ticked (faraday-core/src/screens/parts.rs) -----
    /// Text in `ticked`.
    ticked_none: "None",
    // ----- item_title (faraday-core/src/screens/backup.rs) -----
    /// Text in `item_title`. `{}`: `k`, `if k == 1 { EN.item_title_template_inline …`.
    item_title_print_blank: "Print {} blank {}",
    /// Text in `item_title`.
    item_title_template_inline: "template",
    /// Text in `item_title`.
    item_title_templates: "templates",
    /// Text in `item_title`. `{}`: `b.words`.
    item_title_template_n: "Template for {} words",
    /// Text in `item_title`. `{}`: `shape.seeds.get(i).map_or("", |s| s.n…`.
    item_title_copy_seed: "Copy seed {} by hand",
    /// Text in `item_title`. `{}`: the seed's name.
    item_title_passphrase: "Write the passphrase of seed {} on paper",
    /// Text in `item_title`.
    item_title_checked: "Checked",
    /// Text in `item_title`.
    item_title_not_checked: "Not checked",
    /// Text in `item_title`. `{}`: `v + 1`.
    item_title_vault_number: "Vault {}",
    /// Text in `item_title`. `{}`: `v + 1`, `names.join(", ")`.
    item_title_vault: "Vault {} · {}",
    /// Text in `item_title`. `{}`: `name`.
    item_title_in_vault: "In {}",
    /// Text in `item_title`.
    item_title_not_saved: "Not saved",
    /// Text in `item_title`.
    item_title_the_seeds: "The seeds as files",
    /// Text in `item_title`.
    item_title_not_made: "Not made",
    /// Text in `item_title`.
    item_title_the_shares: "The shares",
    /// Text in `item_title`. `{}`: `keys_n`, `keys_n - a.omit.min(shape.m.saturatin…`.
    item_title_shares_keys: "{} shares, {} keys each",
    /// Text in `item_title`.
    item_title_the_wallet: "The wallet sheet",
    /// Text in `item_title`.
    item_title_print_pdf: "A PDF to print",
    /// Text in `item_title`.
    item_title_the_public: "The public files",
    /// Text in `item_title`.
    item_title_show_descriptor: "Show the descriptor to the software",
    /// Text in `item_title`.
    item_title_shown: "Shown",
    /// Text in `item_title`.
    item_title_not_shown: "Not shown",
    /// Text in `item_title`.
    item_title_one_envelope: "One envelope per place",
    /// Text in `item_title`.
    item_title_envelope: "envelope",
    /// Text in `item_title`.
    item_title_envelopes: "envelopes",
    // ----- question_summary (faraday-core/src/screens/backup.rs) -----
    /// Text in `question_summary`. `{}`: `c.lost.text()`.
    question_summary_any: "Any one place lost: {}",
    // ----- backup (faraday-core/src/screens/backup.rs) -----
    /// Text in `backup_screen`. `{}`: `t`.
    backup_after: "After: {}",
    /// Text in `backup_screen`.
    backup_back_up_a_wallet: "Back up a wallet",
    /// Text in `backup_screen`. `{}`: `wallet.name`.
    backup_back_up: "Back up {}",
    /// Text in `backup_screen`.
    backup_vault_contents: "Vault contents",
    /// Text in `backup_screen`.
    backup_wallets: "Wallets",
    /// Text in `backup_screen`.
    backup_then_wallets: "Then: Wallets",
    /// Text in `backup_screen`.
    backup_change_the_plan: "Change the plan",
    /// Line in `backup_body`.
    backup_seed_words: "Words on the seed",
    /// Line in `backup_body`.
    backup_no_typed_key: "No key of this wallet was loaded from words in this session",
    /// Text in `backup_body`.
    backup_hide_the_seed: "Hide the seed",
    /// Text in `backup_body`.
    backup_show_the_seed: "Show the seed",
    /// Text in `backup_body`, a passphrase item.
    backup_hide_the_passphrase: "Hide the passphrase",
    /// Text in `backup_body`, a passphrase item.
    backup_show_the_passphrase: "Show the passphrase",
    /// Text in `backup_body`, a passphrase item. `{}`: the key's line.
    backup_passphrase_item: "Passphrase of {}",
    /// Line in `backup_body`: the key keeps no passphrase to show.
    backup_no_passphrase_kept: "This key was loaded without its passphrase",
    /// Text in `backup_body`. `{}`: `key_line(key.master.fingerprint(), &k…`, `list.len()`.
    backup_words: "{} · {} words",
    /// Line in `backup_body`.
    backup_seedqr_english_only: "SeedQR is for English words only: copy these words by hand",
    /// Text in `backup_body`.
    backup_standard_seedqr: "Standard SeedQR",
    /// Text in `backup_body`.
    backup_compact_seedqr: "Compact SeedQR",
    /// Text in `backup_body`.
    backup_shown: "Shown",
    /// Text in `backup_body`.
    backup_not_shown: "Not shown",
    /// Text in `backup_body`.
    backup_make_button: "Make it",
    /// Row in `backup_body`.
    backup_backup_sheet: "Backup sheet",
    /// Row in `backup_body`.
    backup_a_pdf_to_print: "A PDF to print",
    /// Text in `backup_body`.
    backup_make_them: "Make them",
    /// Row in `backup_body`.
    backup_the_shares: "The shares",
    /// Row in `backup_body`.
    backup_sheets_to_print: "Sheets to print, text files and pictures",
    /// Text in `backup_body`.
    backup_nothing: "Nothing",
    /// Text in `backup_body`.
    backup_back_to_wallet: "Back to the wallet",
    /// Text in `backup_body`.
    backup_done: "Done",
    // ----- plan (faraday-core/src/screens/backup.rs) -----
    /// Line in `plan_body`.
    plan_a_key_made_here: "A key made here signs only once this backup is done.",
    /// Text in `plan_body`.
    plan_what_is_a_share: "What is a share?",
    /// Section label in `plan_body`.
    plan_form: "Form",
    /// Text in `plan_body`.
    plan_qr_picture: "QR picture",
    /// Text in `plan_body`.
    plan_text: "Text",
    /// Text in `plan_body`. `{}`: `v + 1`, `app.medium.noun()`.
    plan_vault_s_needs: "Vault {}'s {} needs a place",
    /// Button in `plan_body`.
    plan_make_the_checklist: "Make the checklist",
    /// Button in `plan_body`.
    plan_continue: "Continue",
    // ----- place_names (faraday-core/src/screens/backup.rs) -----
    // ----- map_rows (faraday-core/src/screens/backup.rs) -----
    /// Text in `map_rows`. `{}`: `app.medium.cap()`.
    map_rows_of_files: "{} of files",
    /// Text in `map_rows`.
    map_rows_watch_only: "Watch-only software",
    /// Text in `map_rows`.
    map_rows_on_its_own: "On its own device",
    /// Line in `map_rows`.
    map_rows_nothing: "Nothing",
    // ----- backup_done (faraday-core/src/screens/backup.rs) -----
    /// Line in `backup_done`.
    backup_done_backup: "Backup done",
    /// Text in `backup_done`.
    backup_done_open_wallet: "Open the wallet",
    /// Text in `backup_done`.
    backup_done_change: "Change the plan",
    // ----- signers_count (faraday-core/src/screens/spend.rs) -----
    /// Text in `signers_count`. `{}`: `here`, `needed`.
    signers_count_of_signatures_here: "{} of {} signatures here",
    /// Text in `signers_count`. `{}`: `here`, `needed`, `needed - here`.
    signers_count_of_signatures: "{} of {} signatures here · {} more needed",
    /// Text in `signers_count`.
    signers_count_can_spend: "Can spend",
    /// Line in `signers_count`.
    signers_count_continue: "Continue",
    // ----- shares_card (faraday-core/src/screens/backup.rs) -----
    /// Text in `shares_card`.
    shares_card_keys_left: "Keys left off each share",
    /// Text in `shares_card`. `{}`: `k + 1`.
    shares_card_share: "Share {}",
    /// Text in `shares_card`. `{}`: `m`.
    shares_card_any_shares: "Any {} shares rebuild the wallet",
    /// Text in `shares_card`. `{}`: `m`.
    shares_card_some_shares: "Some {} shares do not rebuild the wallet",
    /// Text in `shares_card`. `{}`: `a.smallest_rebuild`.
    shares_card_the_fewest: "The fewest shares that rebuild it: {}",
    /// Text in `shares_card`. `{}`: `a.copies_per_key`.
    shares_card_each_key: "Each key is on {} shares",
    /// Text in `shares_card`.
    shares_card_no_single: "No single share holds every key",
    /// Text in `shares_card`.
    shares_card_each_share: "Each share holds every key",
    // ----- copy_check (faraday-core/src/screens/backup.rs) -----
    /// Label of the typed words in `copy_check`.
    copy_check_your_copys_words: "Your copy's words",
    /// Text in `copy_check`. `{}`: the word being typed, the seed's word count.
    copy_check_word_count: "Word {} of {}",
    /// Text in `copy_check`.
    copy_check_your_copy: "Your copy matches the seed",
    /// Text in `copy_check`. `{}`: `k`.
    copy_check_word_differs: "Word {} differs from the seed",
    /// Button in `copy_check`.
    copy_check_clear: "Clear",
    /// Label of the typed passphrase in `copy_check`.
    copy_check_your_copys_passphrase: "Your copy's passphrase",
    /// Text in `copy_check`.
    copy_check_passphrase_matches: "Your copy matches the passphrase",
    /// Text in `copy_check`.
    copy_check_passphrase_differs: "Your copy differs from the passphrase",
    /// Button on a SeedQR copy's item: the copy dropped for the words.
    copy_check_keep_the_words: "Keep the words only",
    /// Text on a SeedQR copy's item. `{}`: the count.
    copy_check_scans_without: "{} scans without a match",
    /// Text on a copy's item. `{}`: the seed's fingerprint.
    copy_check_drawn_in_part: "Seed {}'s SeedQR, drawn in part, stays on the map until destroyed",
    // ----- item_action (faraday-core/src/screens/backup.rs) -----
    /// Text in `item_action`.
    item_action_continue: "Continue",
    /// Text in `item_action`.
    item_action_check_my: "Check my copy",
    /// Text in `item_action`: a passphrase typed back, compared.
    item_action_compare: "Compare",
    /// Text in `item_action`.
    item_action_lock_new_vault: "Lock it and make a new vault",
    /// Text in `item_action`. `{}`: `open.name`.
    item_action_save_into: "Save into {}",
    /// Text in `item_action`.
    item_action_make: "Make it",
    /// Text in `item_action`.
    item_action_make_them: "Make them",
    /// Text in `item_action`.
    item_action_show_as: "Show as QR",
    // ----- vault_item (faraday-core/src/screens/backup.rs) -----
    /// Text in `vault_item`. `{}`: `o.name`.
    vault_item_holds_another: "{} holds another vault's seed",
    /// Text in `vault_item`. `{}`: `o.name`.
    vault_item_into: "Into {}",
    /// Text in `vault_item`.
    vault_item_into_vault: "Into the vault",
    /// Text in `vault_item`. `{}`: `name`.
    vault_item_seed_passphrase: "Seed {} with its passphrase",
    /// Text in `vault_item`. `{}`: `name`.
    vault_item_seed: "Seed {}",
    /// Text in `vault_item`.
    vault_item_wallet_description: "Wallet description",
    // ----- seed_files (faraday-core/src/screens/backup.rs) -----
    /// Text in `seed_files`.
    seed_files_not_made: "Not made",
    // ----- paper_section (faraday-core/src/screens/backup.rs) -----
    /// Section label in `paper_section`.
    paper_section_other: "Other paper forms",
    /// Text in `paper_section`.
    paper_section_seed_xor: "Seed XOR, 2 parts",
    /// Text in `paper_section`.
    paper_section_seed: "Seed XOR, 3 parts",
    /// Text in `paper_section`. `{}`: `parts.len()`.
    paper_section_parts: "{} parts · every one is needed",
    /// Button in `paper_section`.
    paper_section_put_them: "Put them away",
    // ----- seed_grid (faraday-core/src/screens/backup.rs) -----
    /// Text in `seed_grid`. `{}`: `pin`.
    seed_grid_row: "row {}",
    // ----- backup_panel (faraday-core/src/screens/backup.rs) -----
    /// Line in `backup_panel`.
    backup_panel_map: "Map",
    // ----- kept_rows (faraday-core/src/screens/backup.rs) -----
    /// Line in `kept_rows`.
    kept_rows_keys: "Keys",
    // ----- message (faraday-core/src/screens/message.rs) -----
    /// Text in `message_screen`.
    message_none_chosen: "None chosen",
    /// Text in `message_screen`.
    message_not_written: "Not written yet",
    /// Text in `message_screen`.
    message_signed: "Signed",
    /// Text in `message_screen`.
    message_not_signed: "Not signed yet",
    /// Text in `message_screen`.
    message_wallets: "Wallets",
    /// Text in `message_screen`.
    message_sign_a_message: "Sign a message",
    /// Text in `message_body`. `{}`: `m.index`.
    message_receive_address: "Receive address 0/{}",
    /// Text in `message_body`.
    message_type_the_message: "Type the message",
    /// Text in `message_body`.
    message_press_type: "Press Type to write the message",
    /// Text in `message_body`.
    message_stop_typing: "Stop typing",
    /// Text in `message_body`.
    message_type: "Type",
    /// Text in `message_body`. `{}`: `m.text.chars().count()`.
    message_characters: "{} characters",
    /// Button in `message_body`.
    message_continue: "Continue",
    /// Button in `message_body`.
    message_sign_the_message: "Sign the message",
    /// Line in `message_body`.
    message_signature: "Signature",
    /// Text in `message_body`.
    message_make_the_file: "Make the file",
    /// Text in `message_body`. `{}`: `app.medium.for_the()`.
    message_png: "PNG {}",
    /// Row in `message_body`.
    message_signed_message: "Signed message",
    /// Text in `message_body`.
    message_show_as_qr: "Show as QR",
    // ----- check (faraday-core/src/screens/check_message.rs) -----
    /// Line in `check_screen`.
    check_wallets: "Wallets",
    /// Title in `check_screen`.
    check_check_a_signed: "Check a signed message",
    /// Text in `check_screen`.
    check_a_valid_signature: "A valid signature proves that whoever holds this address's key signed exactly this text. It says nothing about who is showing it to you, or when it was signed.",
    /// Line in `check_screen`.
    check_no_signed_message: "No signed message chosen",
    /// Text in `check_screen`. `{}`: `app.session.network().name()`.
    check_not_a_signed: "Not a signed message for a {} address",
    /// Text in `check_screen`. `{}`: `if *f == faraday_psbt::message::Format::B…`.
    check_signature_valid: "The signature is valid ({})",
    /// Error in `check_screen`. `{}`: `e`.
    check_the_signature: "The signature does not verify: {}",
    /// Text in `check_screen`.
    check_file: "File",
    /// Text in `check_screen`.
    check_address: "Address",
    /// Text in `check_screen`.
    check_wallet: "Wallet",
    /// Text in `check_screen`.
    check_change: "change",
    /// Text in `check_screen`.
    check_receive: "receive",
    /// Text in `check_screen`.
    check_not_an_address: "Not an address of a loaded wallet",
    /// Line in `check_screen`.
    check_message: "Message",
    // ----- create (faraday-core/src/screens/create.rs) -----
    /// Text in `create_screen`. `{}`: `c.m`, `c.n`.
    create_threshold: "{} of {}",
    /// Text in `create_screen`. `{}`: `filled`, `c.slots.len()`.
    create_of_chosen: "{} of {} chosen",
    /// Text in `create_screen`. `{}`: `filled - k`, `c.slots.len()`, `k`.
    create_slots_progress: "{} of {} chosen · {} to come",
    /// Text in `create_screen`. `{}`: `w.name`.
    create_first_addresses: "{} · first addresses",
    /// Text in `create_screen`.
    create_not_made_yet: "Not made yet",
    /// Text in `create_screen`.
    create_home: "Home",
    /// Text in `create_screen`.
    create_wallets: "Wallets",
    /// Text in `create_screen`.
    create_make_a_new_wallet: "Make a new wallet",
    /// Text in `create_body`.
    create_start_over: "Start over",
    /// Text in `create_body`.
    create_signatures_needed: "Signatures needed",
    /// Text in `create_body`.
    create_keys: "Keys",
    /// Text in `create_body`. `{}`: `c.n`.
    create_all_keys_sign: "All {} keys sign together, as one signature on chain. Losing any one key loses the money: there is no spare.",
    /// Text in `create_body`. `{}`: `c.m`, `c.n`, `c.n - c.m`, `if c.n - c.m == 1 { EN.create_key_inline }…`, `c.m`.
    create_any_of_the_keys: "Any {} of the {} keys can spend. Losing {} {} still leaves the money spendable; {} keys together can take it.",
    /// Text in `create_body`.
    create_key_inline: "key",
    /// Text in `create_body`.
    create_keys_inline: "keys",
    /// Line in `create_body`.
    create_locked_line: "Locked in",
    /// Text in `create_body`.
    create_not_chosen: "Not chosen",
    /// Text in `create_body`. `{}`: `fpt`, `name`.
    create_loaded_here: "{} · {} · loaded here",
    /// Text in `create_body`. `{}`: `&t[1..9.min(t.len())]`.
    create_a_cosigner_s: "{} · a cosigner's xpub",
    /// Text in `create_body`.
    create_a_cosigner_adds: "A cosigner adds it later · waiting for its xpub file",
    /// Text in `create_body`.
    create_new_key: "New key",
    /// Text in `create_body`. `{}`: `name`.
    create_slot_source: "From {}",
    /// Text in `create_body`.
    create_scan_its_xpub: "Scan its xpub",
    /// Text in `create_body`.
    create_someone_else: "Someone else adds it later",
    // NEW-WALLET §16 (F45): keys from a vault in Create; a new wallet
    // from one held.
    /// Text in `create_body`: a slot left for a cosigner's xpub at
    /// another account. `{}`: the slot's number, the account.
    create_needs_xpub_account: "Needs Key {}'s xpub at account {}",
    /// Label in `account_row`: the Account row and its stepper, at the
    /// foot of Quorum or the head of Keys (F98).
    create_account: "Account",
    /// Button in `account_row`: opens the Account row's stepper (F98).
    create_account_change: "Change",
    /// The closed line of the card that carries the Account row, and the
    /// proposed name of a wallet made from another. `{}`: the card's
    /// line, the kind's or the wallet's name; the account.
    create_at_account: "{} · account {}",
    /// Label in `create_body`: Check's row of the made wallet's
    /// derivation paths (`docs/history/NEW-WALLET.md` §17).
    create_path: "Path",
    /// Button in `create_body`: Check's Path row, back to the Account
    /// row (F98).
    create_path_change: "Change",
    /// Text in Check's Path row: a key that came with no origin.
    create_path_not_given: "no path given",
    /// Button on a wallet's card.
    wallets_new_wallet_button: "New wallet from this one",
    /// Text in `create_body`.
    create_clear: "Clear",
    /// Text in `create_body`.
    create_show_xpub_qr: "Show xpub QR",
    /// Text in `create_body`. `{}`: `app.medium.for_the()`.
    create_xpub_png: "Xpub PNG {}",
    /// Text in `create_body`. `{}`: `app.medium.for_the()`.
    create_xpub_file: "Xpub file {}",
    /// Text in `create_body`.
    create_add_a_passphrase: "Add a passphrase",
    /// Label in `create_body`.
    create_passphrase: "Passphrase",
    /// Label in `create_body`.
    create_passphrase_again: "Passphrase again",
    /// Button in `create_body`.
    create_lock_button: "Lock in",
    /// Text in `create_body`.
    create_key: "Key",
    /// Text in `create_body`.
    create_continue: "Continue",
    /// Text in `create_body`. `{}`: `app.session.wallets[old].name`.
    create_move_the_money: "Move the money from {}",
    /// Check's foot once the wallet is checked: Back up on the wallet just
    /// made (`docs/BACKUP-FLOW.md` §17.21 item 2).
    create_back_up_button: "Back up wallet",
    /// Line in `create_body`.
    create_make_the_wallet: "Make the wallet first",
    // ----- kind_rows (faraday-core/src/screens/parts.rs) -----
    /// Button in `kind_rows`.
    kind_rows_more_kinds: "More kinds",
    // ----- first_addresses (faraday-core/src/screens/parts.rs) -----
    /// Section label in `first_addresses`.
    first_addresses_heading: "The wallet's first addresses",
    /// Text in `first_addresses`.
    first_addresses_receive_0_0: "Receive 0/0",
    /// Text in `first_addresses`.
    first_addresses_receive_0_1: "Receive 0/1",
    /// Text in `first_addresses`.
    first_addresses_change: "Change 1/0",
    // ----- made_place (faraday-core/src/screens/parts.rs) -----
    /// Text in `made_place`. `{}`: `r.label`.
    made_place_written: "Written to {}",
    // ----- made_line (faraday-core/src/screens/spend.rs) -----
    /// Button in `made_line`.
    made_line_remove: "Remove",
    // ----- made_state (faraday-core/src/screens/parts.rs) -----
    /// Text in `made_state`.
    made_state_remove: "Remove",
    /// Text in `made_state`.
    made_state_make_again: "Make again",
    // ----- locked_tag (faraday-core/src/screens/parts.rs) -----
    /// Text in `locked_tag`.
    locked_tag_text: "Locked in",
    // ----- descriptor_row (faraday-core/src/screens/parts.rs) -----
    /// Line in `descriptor_row`.
    descriptor_row_descriptor: "Descriptor",
    // ----- public_files (faraday-core/src/screens/backup.rs) -----
    /// Text in `public_files`.
    public_files_show_as: "Show as QR",
    /// Text in `public_files`.
    public_files_descriptor: "Descriptor",
    /// Text in `public_files`.
    public_files_any_wallet: "Any wallet software",
    /// Text in `public_files`.
    public_files_wallet: "Wallet file",
    /// Text in `public_files`.
    public_files_sparrow: "Sparrow, Specter",
    /// Text in `public_files`.
    public_files_multisig: "Multisig config",
    /// Text in `public_files`.
    public_files_coldcard: "Coldcard, Keystone, Passport",
    /// Text in `public_files`.
    public_files_backup: "Backup sheet",
    /// Text in `public_files`.
    public_files_a_pdf: "A PDF to print",
    /// Text in `public_files`.
    public_files_bsms_descriptor: "BSMS descriptor record",
    /// Text in `public_files`.
    public_files_bip_129: "BIP 129: Coldcard, Sparrow, Nunchuk",
    /// Text in `public_files`.
    public_files_bitcoin: "Core 29 or later",
    /// Text in `public_files`. `{}`: `fp`.
    public_files_key: "Key {}",
    /// Text in `public_files`.
    public_files_cosigners: "For the cosigners",
    /// Text in `public_files`. `{}`: `fp`.
    public_files_key_bsms: "Key {}, BSMS",
    /// Text in `public_files`.
    public_files_signed: "Signed xpub record, BIP 129",
    /// Text in `public_files`: the Caravan config's row.
    public_files_caravan: "Caravan config",
    /// Text in `public_files`: who reads the Caravan config.
    public_files_caravan_unchained: "Caravan, Unchained",
    /// Text in `public_files`: the Wasabi file's row.
    public_files_wasabi: "Wasabi file",
    /// Text in `public_files`: who reads the Wasabi file.
    public_files_wasabi_wallet: "Wasabi Wallet",
    /// Text in `public_files`: the row of Bitcoin Core's import file
    /// for Core before 29.
    public_files_core_before_29: "Core before 29",
    /// Text in `public_files`: who reads either of Bitcoin Core's import
    /// files.
    public_files_bitcoin_core: "Bitcoin Core, watch-only",
    /// Text in `public_files`: the Electrum wallet file's row.
    public_files_electrum: "Electrum wallet file",
    /// Text in `public_files`: who reads the Electrum wallet file.
    public_files_electrum_watch_only: "Electrum, watch-only",
    // ----- public_rows_for (faraday-core/src/screens/backup.rs) -----
    /// Text in `public_rows_for`.
    public_rows_for_make: "Make the file",
    /// Text in `public_rows_for`: the ghost button the files few people
    /// need are under (F64).
    public_rows_for_more_options: "More options",
    // ----- restore (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_screen`. `{}`: `s.m`, `s.n`.
    restore_threshold: "{} of {}",
    /// Text in `restore_screen`.
    restore_none: "None",
    /// Text in `restore_screen`. `{}`: `here`, `all`.
    restore_of_filled: "{} of {} filled",
    /// Text in `restore_screen`.
    restore_home: "Home",
    /// Text in `restore_screen`.
    restore_title: "Restore an existing wallet",
    /// Text in `restore_body`.
    restore_have_description: "I have the wallet description",
    /// Text in `restore_body`.
    restore_scan_button: "Scan it",
    /// Text in `restore_body`.
    restore_from_files: "From Files",
    /// Text in `restore_body`.
    restore_continue: "Continue",
    /// Text in `restore_body`.
    restore_it_matches: "It matches",
    /// Line in `restore_body`.
    restore_make_the_wallet: "Make the wallet first",
    /// Text in `restore_body`.
    restore_back_up_button: "Back up this wallet",
    /// Text in `restore_body`.
    restore_wallets: "Wallets",
    /// Text in `restore_body`: New wallet from this one, on Done.
    restore_another_wallet: "Another wallet from these seeds",
    // ----- restore_description (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_description`. `{}`: `n`.
    restore_description_source: "From {}",
    /// Text in `restore_description`.
    restore_description_i_have: "I have the seed",
    /// Text in `restore_description`.
    restore_description_scan: "Scan descriptor",
    /// Text in `restore_description`.
    restore_description_i: "I do not have it",
    /// Line in `restore_description`.
    restore_description_shares: "Shares of a split backup",
    /// Text in `restore_description`. `{}`: `m.have.len()`, `m.n`, `m.have.join(" ")`.
    restore_description_progress: "{} of {} keys in hand · {}",
    /// Button in `restore_description`.
    restore_description_rebuild: "Rebuild the wallet",
    // ----- restore_seeds (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_seeds`. `{}`: `k + 1`.
    restore_seeds_key_number: "Key {} of the wallet",
    /// Button in `restore_seeds`.
    restore_seeds_change: "Change",
    /// Button in `restore_seeds`.
    restore_seeds_not_here: "Not here",
    /// Text in `restore_seeds`.
    restore_seeds_here: "Here",
    /// Text in `restore_seeds`. `{}`: `label`.
    restore_seeds_here_n: "Here · {}",
    /// Row in `restore_seeds`.
    restore_seeds_empty: "Empty",
    /// Text in `restore_seeds`.
    restore_seeds_its_xpub: "Its xpub",
    /// Text in `restore_seeds`.
    restore_seeds_make: "Make the wallet",
    // ----- restore_open_slot (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_open_slot`.
    restore_open_slot_type: "Type the words",
    /// Text in `restore_open_slot`.
    restore_open_slot_scan: "Scan a SeedQR",
    // ----- restore_sources (faraday-core/src/screens/restore.rs) -----
    /// Text in `restore_sources_compact`.
    restore_sources_copy_files: "Copy files in",
    /// Text in `restore_sources_compact`.
    restore_sources_key_loaded: "Not while a key is loaded",
    /// Text in `restore_sources_compact`. `{}`: `app.medium.noun()`.
    restore_sources_insert: "Insert the {}",
    /// Line in `restore_sources_compact`.
    restore_sources_vault: "Vault",
    /// Text in `restore_sources_compact`. `{}`: `f.name`.
    restore_sources_unlock: "Unlock {}",
    /// Text in `restore_sources_compact`.
    restore_sources_wallet: "Wallet",
    /// Text in `restore_sources_compact`. `{}`: `app.medium.noun()`.
    restore_sources_copy: "Copy the vault in from its {}",
    // ----- kind_line (faraday-core/src/screens/parts.rs) -----
    /// Text in `kind_line`. `{}`: `bytes as f32 / 1e6`.
    kind_line_mb: "{} MB",
    /// Text in `kind_line`. `{}`: `bytes as f32 / 1e3`.
    kind_line_kb: "{} KB",
    // ----- kind_name (faraday-core/src/screens/parts.rs) -----
    /// Text in `kind_name`.
    kind_name_wallet_descriptor: "Wallet descriptor",
    /// Text in `kind_name`.
    kind_name_finished: "Finished transaction",
    /// Text in `kind_name`.
    kind_name_print_sheet: "Sheet for printing",
    /// Text in `kind_name`.
    kind_name_signed_message: "Signed message",
    /// Text in `kind_name`.
    kind_name_account_xpub: "Account xpub",
    /// Text in `kind_name`.
    kind_name_seed_words: "Seed words",
    /// Text in `kind_name`.
    kind_name_split_share: "Share of a split backup",
    /// Text in `kind_name`.
    #[cfg(feature = "unreleased")]
    kind_name_threshold: "Threshold spend, part-signed",
    /// Text in `kind_name`.
    kind_name_vault_locked: "Vault, locked",
    /// Text in `kind_name`.
    kind_name_vault_entries: "Entries for a vault",
    /// Text in `kind_name`.
    kind_name_keepass_database: "KeePass database, encrypted",
    /// Text in `kind_name`.
    kind_name_pdf_to_print: "PDF to print",
    /// Text in `kind_name`.
    kind_name_efi_image: "EFI image",
    /// Text in `kind_name`.
    kind_name_seed_part: "Part of a seed: SLIP-39 or codex32",
    /// Text in `kind_name`.
    kind_name_text: "Text",
    /// Text in `kind_name`.
    kind_name_gpg: "GPG",
    /// Text in `kind_name`: another wallet's file that holds a private
    /// key or a seed beside its public keys.
    kind_name_secret_wallet: "Wallet file with a private key",
    /// Text in `kind_name`.
    kind_name_file: "File",
    // ----- add_to_vault (faraday-core/src/screens/files.rs) -----
    /// Text in `add_to_vault`.
    add_to_vault_button: "Add to vault",
    // ----- restore_shares_button (faraday-core/src/screens/files.rs) -----
    /// Text in `restore_shares_button`. `{}`: `texts.len()`.
    restore_shares_button_restore: "Restore the wallet · {} shares",
    /// Text in `restore_shares_button`. `{}`: `m.have.len()`, `m.n`.
    restore_shares_button_progress: "{} of {} keys in hand",
    // ----- files (faraday-core/src/screens/files.rs) -----
    /// Title in `files`.
    files_files: "Files",
    /// Button in `files`.
    files_add_to_signatures: "Add to signatures",
    /// Text in `files`.
    files_import_into_vault: "Import into the vault",
    /// Text in `files`.
    files_contents: "Contents",
    /// Text in `files`.
    files_unlock: "Unlock",
    /// Button in `files`.
    files_sign: "Sign",
    /// Button in `files`.
    files_load_the_wallet: "Load the wallet",
    /// Button in `files`.
    files_decode: "Decode",
    /// Button in `files`.
    files_load_this_key: "Load this key",
    /// Button in `files`.
    files_check_the_signature: "Check the signature",
    /// Text in `files_compact`. `{}`: `app.medium.a()`.
    files_waiting_medium: "Waiting for {}",
    /// Text in `files_compact`. `{}`: `app.medium.cap()`.
    files_attached_lock: "{} attached · lock to use it",
    /// Text in `files_compact`. `{}`: `app.sticks[app.visit.stick.min(app.st…`.
    files_visit: "Visit {}",
    /// Button in `files_compact`.
    files_scan_a_qr_code: "Scan a QR code",
    /// Line in `files_compact`.
    files_in_memory_emptied: "In memory · emptied at power-off",
    /// Text in `files_compact`.
    files_locked: "locked",
    /// Text in `files_compact`.
    files_open: "open",
    /// Text in `files_compact`. `{}`: `app.medium.a()`.
    files_empty_files_are: "Empty · files are copied in on {} visit",
    /// Text in `files_compact`.
    files_public: "Public",
    /// Text in `files_compact`.
    files_anyone_may_read: "Anyone may read these",
    /// Text in `files_compact`.
    files_written_sealed: "Written sealed",
    /// Text in `files_compact`. `{}`: `app.medium.a()`.
    files_sealed_under: "Sealed under their passphrases before they reach {}",
    /// Text in `files_compact`.
    files_unprotected_secrets: "Unprotected secrets",
    /// Text in `files_compact`. `{}`: `app.medium.noun()`.
    files_anyone_who_has: "Anyone who has the {} can use these",
    /// Text in `files_compact`. `{}`: `v.changes`, `if v.changes == 1 { EN.files_change }…`.
    files_vault_open_unsaved: "Vault, open · {} unsaved {} · sealed and locked at Lock",
    /// Text in `files_compact`.
    files_change: "change",
    /// Text in `files_compact`.
    files_changes: "changes",
    /// Text in `files_compact`.
    files_vault_open_sealed: "Vault, open · sealed and locked at Lock",
    /// Text in `files_compact`.
    files_unprotected: "Unprotected",
    /// Text in `files_compact`.
    files_written_after: "Written after Lock",
    /// Text in `files_compact`.
    files_ready: "Ready",
    /// Text in `files_compact`.
    files_make_pdf: "Make PDF",
    /// Text in `files_compact`.
    files_show_as_qr: "Show as QR",
    /// Text in `files_compact`.
    files_remove: "Remove",
    /// Line in `files_compact`.
    files_empty: "Empty",
    /// Text in `files_compact`. `{}`: `app.medium.for_the()`.
    files_power_off_asks: "Power off asks first while files wait {}",
    /// Text in `files_compact`. `{}`: `app.medium.from_box()`, `app.medium.nouns()`, `app.medium.for_box()`.
    files_holds_what_was: "{} holds what was copied in; {} holds what waits to be written. Both are emptied at power-off. A secret goes out unprotected only after a warning.",
    // ----- receipt_rows (faraday-core/src/screens/files.rs) -----
    /// Text in `receipt_rows`.
    receipt_rows_verified: "verified",
    // ----- vault_panel (faraday-core/src/screens/files.rs) -----
    /// Text in `vault_panel`. `{}`: `open.name`, `open.label()`.
    vault_panel_open: "{} · {} · open",
    /// Text in `vault_panel`.
    vault_panel_select: "Select all",
    /// Line in `vault_panel`.
    vault_panel_seeded_wallets: "Wallets with their seeds",
    /// Text in `vault_panel`. `{}`: `here`, `wt.total`, `if wt.total == 1 { EN.vault_panel_key…`.
    vault_panel_keys_here: "{} of {} {} in this vault",
    /// Text in `vault_panel`.
    vault_panel_ready_to_sign: "Ready to sign once loaded",
    /// Text in `vault_panel`. `{}`: `more`.
    vault_panel_more_listed: "+ {} more · listed under Each seed and wallet",
    /// Text in `vault_panel`.
    vault_panel_load_wallet: "Load wallet with keys",
    /// Text in `vault_panel`.
    vault_panel_load: "Load 1 wallet with keys",
    /// Text in `vault_panel`. `{}`: `n`.
    vault_panel_load_wallets: "Load {} wallets with keys",
    /// Text in `vault_panel`.
    vault_panel_hide_each: "Hide each seed and wallet",
    /// Text in `vault_panel`. `{}`: `rows.len()`.
    vault_panel_each_seed: "Each seed and wallet · {}",
    /// Text in `vault_panel`.
    vault_panel_loaded: "Loaded",
    /// Text in `vault_panel`.
    vault_panel_everything: "Everything is loaded",
    /// Text in `vault_panel`.
    vault_panel_nothing: "Nothing chosen",
    /// Text in `vault_panel`. `{}`: `count(wl, EN.vault_panel_wallet, EN.v…`.
    vault_panel_load_n: "Load {}",
    /// Text in `vault_panel`. `{}`: `count(s, EN.vault_panel_key, EN.vault…`, `count(wl, EN.vault_panel_wallet, EN.v…`.
    vault_panel_load_keys_wallets: "Load {} and {}",
    /// Text in `vault_panel`.
    vault_panel_key: "key",
    /// Text in `vault_panel`.
    vault_panel_keys: "keys",
    /// Text in `vault_panel`.
    vault_panel_wallet: "wallet",
    /// Text in `vault_panel`.
    vault_panel_wallets: "wallets",
    /// Button in `vault_panel`.
    vault_panel_go_to_wallets: "Go to Wallets",
    // ----- inbox (faraday-core/src/screens/files.rs) -----
    /// Line in `inbox_panel_compact`.
    inbox_wallets: "Wallets",
    /// Text in `inbox_panel_compact`. `{}`: `fw.shape`, `fw.seeds_here`, `n`, `if n == 1 { EN.inbox_key } else { EN.…`, `if fw.files.len() > 1 { format!(" · {…`.
    inbox_of_here: "{} · {} of {} {} here{}",
    /// Text in `inbox_panel_compact`.
    inbox_key: "key",
    /// Text in `inbox_panel_compact`.
    inbox_keys: "keys",
    /// Line in `inbox_panel_compact`.
    inbox_ready_to_sign: "Ready to sign once loaded",
    /// Text in `inbox_panel_compact`. `{}`: `wallets.len() - ROWS`.
    inbox_more_loaded: "+ {} more, loaded with these",
    /// Text in `inbox_panel_compact`.
    inbox_nothing_chosen: "Nothing chosen",
    /// Text in `inbox_panel_compact`.
    inbox_load_wallet_seeds: "Load the wallet with its seeds",
    /// Text in `inbox_panel_compact`.
    inbox_load_the_wallet: "Load the wallet",
    /// Text in `inbox_panel_compact`. `{}`: `n`.
    inbox_load_wallets_seeds: "Load {} wallets with their seeds",
    /// Text in `inbox_panel_compact`. `{}`: `n`.
    inbox_load_wallets: "Load {} wallets",
    /// Text in `inbox_panel_compact`. `{}`: `app.medium.noun()`.
    inbox_keys_load_once: "Keys load once the {} is out",
    /// Line in `inbox_panel_compact`.
    inbox_loose_keys: "Keys with no wallet",
    /// Text in `inbox_panel_compact`.
    inbox_make_it_a_wallet: "Make it a wallet",
    /// Text in `inbox_panel_compact`. `{}`: `potential.len() - ROWS`.
    inbox_more: "+ {} more",
    /// Line in `inbox_panel_compact`.
    inbox_loose_xpubs: "Xpubs with no wallet",
    /// Text in `inbox_panel_compact`.
    inbox_no_origin: "no origin",
    /// Text in `inbox_panel_compact`.
    inbox_make_it_a_watch: "Make it a watch-only wallet",
    /// Line in `inbox_panel_compact`.
    inbox_not_complete: "Not complete or sealed",
    // ----- vault (faraday-core/src/screens/files.rs) -----
    /// Text in `vault_panel_compact`. `{}`: `open.name`, `open.label()`.
    vault_open: "{} · {} · open",
    /// Line in `vault_panel_compact`.
    vault_seeded_wallets: "Wallets with their seeds",
    /// Text in `vault_panel_compact`. `{}`: `here`, `wt.total`, `if wt.total == 1 { EN.vault_key_inline } els…`.
    vault_of_in_this_vault: "{} of {} {} in this vault",
    /// Line in `vault_panel_compact`.
    vault_ready_to_sign: "Ready to sign once loaded",
    /// Text in `vault_panel_compact`. `{}`: `more`.
    vault_more_listed_under: "+ {} more · listed under Each seed and wallet",
    /// Text in `vault_panel_compact`.
    vault_load_wallet_keys: "Load wallet with keys",
    /// Text in `vault_panel_compact`.
    vault_load_1_wallet: "Load 1 wallet with keys",
    /// Text in `vault_panel_compact`. `{}`: `n`.
    vault_load_wallets: "Load {} wallets with keys",
    /// Text in `vault_panel_compact`.
    vault_hide_each_seed: "Hide each seed and wallet",
    /// Text in `vault_panel_compact`. `{}`: `rows.len()`.
    vault_each_seed_wallet: "Each seed and wallet · {}",
    /// Line in `vault_panel_compact`.
    vault_select_all: "Select all",
    /// Text in `vault_panel_compact`.
    vault_loaded: "Loaded",
    /// Text in `vault_panel_compact`.
    vault_everything_loaded: "Everything is loaded",
    /// Text in `vault_panel_compact`.
    vault_nothing_chosen: "Nothing chosen",
    /// Text in `vault_panel_compact`. `{}`: `count(wl, EN.vault_wallet_inline, EN.vault_w…`.
    vault_load_n: "Load {}",
    /// Text in `vault_panel_compact`. `{}`: `count(s, EN.vault_key_inline, EN.vault_keys)`, `count(wl, EN.vault_wallet_inline, EN.vault_w…`.
    vault_load_keys_wallets: "Load {} and {}",
    /// Text in `vault_panel_compact`.
    vault_key_inline: "key",
    /// Text in `vault_panel_compact`.
    vault_keys: "keys",
    /// Text in `vault_panel_compact`.
    vault_wallet_inline: "wallet",
    /// Text in `vault_panel_compact`.
    vault_wallets: "wallets",
    /// Text in `vault_panel_compact`.
    vault_go_to_wallets: "Go to Wallets",
    // ----- inbox_actions (faraday-core/src/screens/files.rs) -----
    /// Text in `inbox_actions`.
    inbox_actions_add_to_vault: "Add to vault",
    /// Text in `inbox_actions`.
    inbox_actions_add_signatures: "Add to signatures",
    /// Text in `inbox_actions`.
    inbox_actions_import: "Import into the vault",
    /// Text in `inbox_actions`.
    inbox_actions_contents: "Contents",
    /// Text in `inbox_actions`.
    inbox_actions_unlock: "Unlock",
    /// Text in `inbox_actions`.
    inbox_actions_sign: "Sign",
    /// Text in `inbox_actions`.
    inbox_actions_load_wallet: "Load the wallet",
    /// Text in `inbox_actions`.
    inbox_actions_decode: "Decode",
    /// Text in `inbox_actions`. `{}`: `texts.len()`.
    inbox_actions_restore: "Restore the wallet · {} shares",
    /// Note in `inbox_actions`. `{}`: `m.have.len()`, `m.n`.
    inbox_actions_of_keys: "{} of {} keys in hand",
    /// Text in `inbox_actions`.
    inbox_actions_load: "Load this key",
    /// Text in `inbox_actions`.
    inbox_actions_check: "Check the signature",
    /// Text in `inbox_actions`.
    inbox_actions_make: "Make PDF",
    // ----- visit (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_compact`.
    visit_unselect_all: "Unselect all",
    /// Line in `visit_compact`.
    visit_select_all: "Select all",
    /// Text in `visit`. `{}`: `app.medium.noun()`, `app.medium.noun()`, `app.medium.noun()`.
    visit_write_what_waits: "Write what waits for the {} and copy in what you need from it. Each file written is read back and compared before it counts. Remove the {} when you are done: keys load only with no {} attached.",
    /// Text in `visit`. `{}`: `app.medium.noun()`.
    visit_no_attached: "No {} attached",
    /// Text in `visit`.
    visit_in_faraday: "In Faraday",
    /// Text in `visit`.
    visit_public: "Public",
    /// Text in `visit`.
    visit_sealed: "Sealed",
    /// Text in `visit`.
    visit_unprotected_secret: "Unprotected secret",
    /// Text in `visit`. `{}`: `app.medium.for_the()`.
    visit_nothing_waits: "Nothing waits {}",
    /// Text in `visit`. `{}`: `nout`, `if nout == 1 { EN.visit_file } else {…`.
    visit_write: "Write {} {}",
    /// Text in `visit`.
    visit_file_too_large: "Larger than 18 MB: not read",
    /// Text in `visit`.
    visit_settings: "Settings",
    /// Text in `visit`: a file asked for and not yet read. `{}`: its size.
    visit_being_read: "Being read · {}",
    /// A toast on the frame drawn before a press's pictures and PDFs are
    /// made (`docs/SPEED.md` §9).
    working_files: "Making the files",
    /// A toast on the frame drawn before an aezeed is stretched for its
    /// node key.
    working_node_key: "Working out the node key",
    /// Text in `visit`. `{}`: `app.medium.noun()`.
    visit_no_files: "No files on this {}",
    /// Button in `visit`.
    visit_delete_files: "Delete files",
    /// Text in `visit`.
    visit_import_and_load: "Import and load",
    /// Text in `visit`. `{}`: `nin`, `if nin == 1 { EN.visit_file } else { …`.
    visit_import: "Import {} {}",
    /// Text in `visit`.
    visit_file: "file",
    /// Text in `visit`.
    visit_files: "files",
    /// Text in `visit`.
    visit_open_files: "Open Files",
    // ----- visit_inbox_line (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_inbox_line`. `{}`: `kind_line(FileKind::Other, item.bytes…`.
    visit_inbox_line_png_no_qr: "PNG, no QR code read · {}",
    /// Text in `visit_inbox_line`. `{}`: `kind_line(k, item.bytes.len())`.
    visit_inbox_line_png: "PNG · {}",
    // ----- visit_inbox_tag (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_sealed: "Sealed",
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_public: "Public",
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_may: "May be a secret",
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_secret: "Secret",
    // ----- visit_select_all (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_select_all`.
    visit_select_all_select: "Select all",
    // ----- visit_unselect_all (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_unselect_all`.
    visit_unselect_all_unselect: "Unselect all",
    // ----- visit_list_bar (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_list_bar`. `{}`: `(below / ROW).ceil() as usize`.
    visit_list_bar_more: "{} more · scroll to see them",
    // ----- pull_line (faraday-core/src/screens/parts.rs) -----
    /// Text in `pull_line`. `{}`: `noun`, `v`.
    pull_line_remove_to_unlock: "Remove the {} to unlock {}",
    /// Text in `pull_line`. `{}`: `noun`.
    pull_line_remove_when_done: "Remove the {} when you are done",
    /// Text in `pull_line`. `{}`: `noun`.
    pull_line_1_key_copied: "1 key copied in · remove the {} to load it",
    /// Text in `pull_line`. `{}`: `n`, `noun`.
    pull_line_keys_copied: "{} keys copied in · remove the {} to load them",
    // ----- entry (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry`.
    entry_spend: "Spend",
    /// Text in `entry`.
    entry_wallets: "Wallets",
    /// Text in `entry`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    entry_add_key: "Add key {}",
    /// Text in `entry`. `{}`: `name`.
    entry_add_key_title: "Add a key for {}",
    /// Text in `entry`. `{}`: `app.entry.form.name()`.
    entry_add_a_key_n: "Add a key · {}",
    /// Text in `entry`.
    entry_add_a_key: "Add a key",
    /// Text in `entry`.
    entry_other_languages: "Other languages",
    /// Text in `entry`.
    entry_other_forms: "Other forms",
    /// Button in `entry`.
    entry_scan_a_seedqr: "Scan a SeedQR",
    /// Text in `entry`.
    entry_type_the_words: "Type the words in order; Tab finishes a word once only one fits. The last word carries a checksum, so a mistyped word is caught when the count is complete.",
    /// Text in `entry`.
    entry_no_word_starts: "No word starts with that",
    /// Text in `entry`, after the candidates and three spaces.
    entry_tab_completes: "· Tab completes a single match",
    // ----- word_count_row (faraday-core/src/screens/entry.rs) -----
    /// Line in `word_count_row`.
    word_count_row_words: "Words",
    // ----- keyed_words (faraday-core/src/screens/entry.rs) -----
    /// Text in `keyed_words`.
    keyed_words_type_each_word_s: "Type each word's pinyin, then its tone 1 to 5, and press the character among those offered. The last word's checksum is checked when the count is complete.",
    /// Text in `keyed_words`.
    keyed_words_type_each_word: "Type each word's 注音 and then its tone on the keys below, and press the character among those offered. The last word's checksum is checked when the count is complete.",
    /// Text in `keyed_words`.
    keyed_words_type_each: "Type each word on the keys below and press it among the words it can be. Keys that lead to no word are off. The last word carries a checksum, checked when the count is complete.",
    /// Text in `keyed_words`.
    keyed_words_type_next: "Type the next word",
    /// Text in `keyed_words`.
    keyed_words_tone_prompt: "Now its tone",
    /// Text in `keyed_words`.
    keyed_words_no_word: "No word starts with that",
    /// Line in `keyed_words`.
    keyed_words_press_hint: "Press a word to take it",
    // ----- word_keys (faraday-core/src/screens/parts.rs) -----
    /// Button in `word_keys`.
    word_keys_shift: "Shift",
    // ----- keyed_status (faraday-core/src/screens/entry.rs) -----
    /// Text in `keyed_status`. `{}`: `n`.
    keyed_status_word_count: "{} words",
    /// Text in `keyed_status`. `{}`: `n`, `fp_text(fp)`.
    keyed_status_words: "{} words · checksum correct · {}",
    // ----- entry_status (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry_status`. `{}`: `n`, `fp_text(fp)`.
    entry_status_words_checksum: "{} words · checksum correct · {}",
    /// Text in `entry_status`. `{}`: `n`.
    entry_status_words: "{} words",
    // ----- entry_finish (faraday-core/src/screens/entry.rs) -----
    /// Line in `entry_finish`.
    entry_finish_bip39_passphrase: "BIP-39 passphrase",
    /// Button in `entry_finish`.
    entry_finish_add_key: "Add key",
    /// Button in `entry_finish`.
    entry_finish_clear: "Clear",
    /// Button in `entry_finish`.
    entry_finish_new_key: "Make a new key",
    /// Line in `entry_finish`.
    entry_finish_keys_hint: "Type on the keyboard · Enter adds · Esc goes back",
    // ----- entry_stick (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry_stick`. `{}`: `app.medium.a()`.
    entry_stick_from_plug: "From {} · plug it in",
    /// Button in `entry_stick`.
    entry_stick_copy_files: "Copy files in",
    // ----- entry_form (faraday-core/src/screens/entry.rs) -----
    /// Text in `entry_form`.
    entry_form_type_one_share: "Type one share at a time, 20 or 33 words, and press Enter, in any order. Then the passphrase, if the backup has one, and Make the key.",
    /// Text in `entry_form`.
    entry_form_type_one_codex32: "Type one codex32 string at a time, ms1 and all, and press Enter. The secret string is the key at once; shares need as many as the digit after ms1 says.",
    /// Text in `entry_form`.
    entry_form_type_one: "Type one part at a time, its BIP-39 words, and press Enter. Every part is needed, and they combine into the words of the key.",
    /// Text in `entry_form`.
    entry_form_one_share_words: "Words of one share",
    /// Text in `entry_form`.
    entry_form_nothing: "Nothing in yet",
    /// Text in `entry_form`.
    entry_form_count_one: "1 in",
    /// Text in `entry_form`. `{}`: `n`.
    entry_form_count_many: "{} in",
    /// Line in `entry_form`.
    entry_form_passphrase: "Passphrase",
    /// Button in `entry_form`.
    entry_form_add_share: "Add this one",
    /// Button in `entry_form`.
    entry_form_scan_a_qr: "Scan a QR code",
    /// Button in `entry_form`.
    entry_form_make_key: "Make the key",
    /// Line in `entry_form`.
    entry_form_enter_adds: "Enter adds what is typed · Enter on an empty line makes the key",
    // ----- tz_row (faraday-core/src/screens/parts.rs) -----
    /// Line in `tz_row`.
    tz_row_time_zone: "Time zone",
    // ----- set_time (faraday-core/src/screens/set_time.rs) -----
    /// Title in `set_time_screen`.
    set_time_set_the_time: "Set the time",
    /// Text in `set_time_compact`.
    set_time_now: "Now",
    /// Section label in `set_time_compact`.
    set_time_date: "Date",
    /// Section label in `set_time_compact`.
    set_time_time: "Time",
    /// Text in `set_time_compact`.
    set_time_set: "Set",
    /// Text in `set_time_compact`.
    set_time_keep_button: "Keep this time",
    /// Text in `set_time_compact`.
    set_time_back: "Back",
    // ----- settings (faraday-core/src/screens/settings.rs) -----
    /// Text in `settings_compact`.
    settings_time: "Time",
    /// Line in `settings_compact`.
    settings_now: "Now",
    /// Button in `settings_compact`.
    settings_set_the_time: "Set the time",
    /// Title in `settings`.
    settings_settings: "Settings",
    /// Mode's explainer on Settings, shown with Guided on.
    settings_guide_mode: "The network that keys, wallets and addresses are made for.",
    /// Session's explainer on Settings, shown with Guided on.
    settings_guide_session: "Lock and power-off end the session: every key and wallet loaded is gone. Remove every key unloads the keys and keeps the wallets; the timers lock or power off after that long with no input.",
    /// Time's explainer on Settings, shown with Guided on.
    settings_guide_time: "TOTP codes, GPG and Secure Boot signatures, and backup check dates read this clock.",
    /// Randomness' explainer on Settings, shown with Guided on.
    settings_guide_randomness: "Randomness you add, from the camera, dice, coins or cards, is hashed together with the system's into the session's randomness: what comes out is at least as strong as the stronger of the two.",
    /// Appearance's explainer on Settings, shown with Guided on.
    settings_guide_appearance: "Theme, mode and contrast set the colours. Reduce motion shows changes at once, with no slide or fade.",
    /// Display scale's explainer on Settings, shown with Guided on.
    settings_guide_scale: "The size of text and controls. Auto is the size for this screen.",
    /// Signed amounts' explainer on Settings, shown with Guided on.
    settings_guide_signed: "The amounts each signed input stated are remembered, and the same transaction stated with other amounts is refused. Sealed into an open vault, they last past power-off.",
    /// The stick's explainer on Settings, shown with Guided on. `{}`: the medium's noun.
    settings_guide_stick: "Upgrade copies the system this device started from onto another Faraday {}.",
    /// About's explainer on Settings, shown with Guided on.
    settings_guide_about: "The self-test runs at every start, before any key is accepted. Run again repeats it.",
    /// Line in `settings`.
    settings_session: "Session",
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_key } else { …`.
    settings_loaded: "{} {} loaded",
    /// Text in `settings`.
    settings_key: "key",
    /// Text in `settings`.
    settings_keys: "keys",
    /// Button in `settings`.
    settings_lock: "Lock",
    /// Button in `settings`.
    settings_remove_every: "Remove every key",
    /// Button in `settings`.
    settings_power_off_button: "Power off",
    /// Button in `settings` in the desktop app, where there is no power to turn off.
    settings_quit: "Quit",
    /// Text in `settings`.
    settings_lock_after: "Lock after",
    /// Text in `settings`.
    settings_power_off: "Power off after",
    /// Text in `settings`.
    settings_never: "Never",
    /// Text in `settings`. `{}`: `m`.
    settings_min: "{} min",
    /// Text in `settings`. `{}`: `names.join(", ")`.
    settings_ignored_until: "Ignored until unplugged: {}",
    /// Row in `settings`, and the mode sheet's title row there.
    settings_mode: "Mode",
    /// Line in `settings`.
    settings_appearance: "Appearance",
    /// Settings' switch row: nothing slides, glides, coasts or fades
    /// (`docs/MOTION.md`).
    settings_reduce_motion: "Reduce motion",
    /// Line in `settings`.
    settings_display_scale: "Display scale",
    /// Text in `settings`.
    settings_auto: "Auto",
    /// Preview size's choice: the window's own size.
    settings_preview_this_window: "This window",
    /// Preview size's choice: the Pi's 2.8-inch panel, 268 × 358 dp.
    settings_preview_pi: "Pi panel",
    /// Preview size's choice: a narrow window, 400 × 800 dp.
    settings_preview_narrow: "Narrow window",
    /// Preview size's choice: half a screen, 760 × 1000 dp.
    settings_preview_half: "Half screen",
    /// Preview size's choice: a 7-inch panel, 960 × 580 dp.
    settings_preview_7in: "7-inch panel",
    /// Preview size's choice: a 10-inch panel, 1370 × 860 dp.
    settings_preview_10in: "10-inch panel",
    /// Preview size's choice: the desktop app's window, 1280 × 800 dp.
    settings_preview_desktop: "Desktop window",
    /// Preview size's choice: a full monitor, 1920 × 1080 dp.
    settings_preview_monitor: "Full monitor",
    /// Preview size's value under the choices. `{}`: width, height in dp.
    settings_preview_dp: "{} × {} dp",
    /// Faraday Demo's host strip (docs/DESKTOP-HOST.md): the preview
    /// size's button. `{}`: the size shown.
    host_size: "Size · {}",
    /// Faraday Demo's host strip: the preview size's button where the
    /// strip is narrow.
    host_size_short: "Size",
    /// Faraday Demo's host strip: the sticks' menu where the strip is
    /// narrowest.
    host_sticks_short: "Sticks",
    /// Faraday Demo's host strip: the computer's die's Rolls per click,
    /// opening its choices. `{}`: the one chosen, or the number typed.
    host_rolls_per_click: "Rolls per click · {}",
    /// Faraday Demo's host strip: the head of the screen's practice aids.
    host_demo: "Practice",
    /// Faraday Demo's host strip: the practice aids that do not fit, in a
    /// menu.
    host_more: "More",
    /// Faraday Demo's host strip: Play's options, how far it runs and
    /// how fast, in a menu.
    host_play_options: "Play options",
    /// Line in `settings`.
    settings_signed_amounts: "Signed amounts",
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_transaction }…`.
    settings_remembered: "{} {} remembered",
    /// Text in `settings`.
    settings_transaction: "transaction",
    /// Text in `settings`.
    settings_transactions: "transactions",
    /// Text in `settings`.
    settings_sealed_into: "Sealed into an open vault",
    /// Text in `settings`.
    settings_kept_until: "Kept until power-off only",
    /// Line in `settings`.
    settings_about: "About",
    /// Text in `settings`. `{}`: `crate::version_label()`.
    settings_faraday: "Faraday {}",
    /// Text in `settings`.
    settings_keys_are_typed: "Keys are typed, scanned as SeedQR or loaded from a vault, and kept for the session only",
    /// Text in `settings`. `{}`: `app.medium.noun()`, `app.medium.a()`.
    settings_vaults_files: "Vaults · files move by {} · QR by camera and from PNG files on {}",
    /// Text in `settings`. `{}`: `app.medium.caps()`.
    settings_are_folders: "{} are folders in ~/faraday-sticks",
    /// Text in `settings`. `{}`: `app.medium.caps()`.
    settings_medium_reader_note: "{} are read by an unprivileged disk process; the kernel mounts none",
    /// Text in `settings`.
    settings_not_run: "not run",
    // ----- institute_note (faraday-core/src/screens/settings.rs) -----
    /// Text in `institute_note`. `{}`: `SITE`.
    institute_note_visit: "Visit {} to learn about Bitcoin’s history, economics, and technology. Not affiliated.",
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
    /// Text in `lock_sheet`.
    lock_fresh_start_upgrade: "A fresh start, and the upgrade",
    /// Text in `lock_sheet`. `{}`: `app.medium.noun()`.
    lock_a_fresh_start: "A fresh start, and the {} visit",
    /// Text in `lock_sheet`.
    lock_lock_and_upgrade: "Lock and upgrade",
    /// Text in `lock_sheet`.
    lock_the_upgrade_runs: "The upgrade runs in a fresh session",
    /// Text in `lock_sheet`. `{}`: `app.medium.a_cap()`.
    lock_is_attached: "{} is attached",
    /// Text in `lock_sheet`. `{}`: `app.medium.noun()`.
    lock_lock_and_use: "Lock and use {}",
    /// Text in `lock_sheet`. `{}`: `label`.
    lock_nothing_has_been: "{} · nothing has been read from it",
    /// Text in `lock_sheet`.
    lock_not_now: "Not now",
    // ----- write_out (faraday-core/src/screens/sheets.rs) -----
    /// Text in `write_out_sheet`. `{}`: `app.medium.a()`.
    write_out_lock_plug: "Lock, plug in {}, and the visit writes them",
    /// Text in `write_out_sheet`, with the stick already in.
    write_out_lock_here: "Lock, and the visit writes them",
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
    /// Dell's BIOS key (`docs/learn/faraday/boot/maker-dell.md`).
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
    /// Dell's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_dell: "F12",
    /// HP's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_hp: "Esc, then F9",
    /// Lenovo ThinkPad's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_lenovo_thinkpad: "F12",
    /// Lenovo IdeaPad, Yoga and Legion's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_lenovo_ideapad: "F12 (or Fn+F12)",
    /// ASUS's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_asus: "Esc, held",
    /// Acer's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_acer: "F12",
    /// MSI's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_msi: "F11",
    /// Framework's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
    boot_key_framework: "F12",
    /// Surface's boot menu key (`docs/learn/faraday/boot/maker-*.md`).
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
    // ----- learn (faraday-core/src/screens/sheets.rs) -----
    /// Button in `learn_sheet`.
    learn_done: "Done",
    /// Text in `learn_sheet`.
    learn_spending_step: "Spending, step by step",
    // ----- secret_out (faraday-core/src/screens/sheets.rs) -----
    /// Text in `secret_out_sheet`.
    secret_out_this_may: "This may be a secret",
    /// Text in `secret_out_sheet`.
    secret_out_title: "This is a secret",
    /// Text in `secret_out_sheet`. `{}`: `app.medium.for_the()`.
    secret_out_put_it_unprotected: "Put it {} unprotected",
    /// Text in `secret_out_sheet`. `{}`: `app.medium.noun()`.
    secret_out_unprotected_medium: "Unprotected, on the {}",
    /// Text in `secret_out_sheet`.
    secret_out_tick_hint: "Tick it to write",
    /// Text in `secret_out_sheet`. `{}`: `app.medium.for_the()`.
    secret_out_unprotected: "Unprotected, {}",
    /// Text in `secret_out_sheet`.
    secret_out_file: "File",
    /// Text in `secret_out_sheet`.
    secret_out_what_label: "What it is",
    /// Text in `secret_out_sheet`.
    secret_out_who_can: "Who can use it",
    /// Line in `secret_out_sheet`.
    secret_out_sealed: "Sealed in a vault",
    /// Text in `secret_out_sheet`. `{}`: `name`.
    secret_out_in_vault: "In {}",
    /// Text in `secret_out_sheet`. `{}`: `name`.
    secret_out_save_into: "Save into {}",
    /// Button in `secret_out_sheet`.
    secret_out_cancel: "Cancel",
    // ----- secrets (faraday-core/src/secrets.rs) -----
    /// Text in `text`. `{}`: `noun`.
    secrets_anyone_who: "Anyone who copies the {} or sees the code can read it",
    /// Text in `text`. `{}`: `noun`.
    secrets_i_understand: "I understand: anyone who copies the {} or sees this file can spend these coins",
    /// Text in `text`. `{}`: `noun`.
    secrets_faraday_cannot: "Faraday cannot tell whether this is a secret: anyone who copies the {} can read it",
    /// Text in `offer_seed`.
    secrets_words: "Words",
    /// Text in `offer_seed`. `{}`: `fps`.
    secrets_seed: "Seed {}",
    /// Text in `offer_seed`. `{}`: `m.word_count()`.
    secrets_words_secret: "{} words · secret: spends",
    /// Text in `offer_seed`.
    secrets_compact_seedqr: "Compact SeedQR PNG",
    /// Text in `offer_seed`.
    secrets_seedqr_png: "SeedQR PNG",
    /// Text in `offer_seed`.
    secrets_seed_warning: "Whoever has it spends this wallet's coins",
    /// Text in `offer_seed`. `{}`: `fps`, `more`, `if more == 1 { EN.secrets_key } else …`.
    secrets_whoever_has: "Whoever has it signs as key {}; with {} more {}, spends",
    /// Text in `offer_seed`.
    secrets_key: "key",
    /// Text in `offer_seed`.
    secrets_keys: "keys",
    /// Text in `offer_seed`.
    secrets_a_wallet_s: "A wallet's seed",
    /// Toast in `secret_to_vault`.
    secrets_no_vault_open: "No vault is open",
    /// Text in `secret_to_vault`. `{}`: `vault`, `r.psbt_name`, `self.medium.for_the()`.
    #[cfg(feature = "unreleased")]
    secrets_nonce_saved: "The nonce is in {}; {} waits {}",
    /// Toast in `secret_to_vault`. `{}`: `out.name`, `vault`.
    secrets_saved_to_vault: "{} is in {}",
    /// Text in `secret_unprotected`. `{}`: `name`, `self.medium.for_the()`.
    secrets_waits_unprotected: "{} waits {}, unprotected",
    // ----- secureboot (faraday-core/src/secure_boot/mod.rs) -----
    /// Toast in `sb_make`.
    secureboot_no_randomness: "No randomness from the system yet. Try again",
    /// Toast in `sb_enrol`.
    secureboot_this_computer: "This computer's clock is not known",
    /// Toast in `sb_enrol`. `{}`: `n`.
    secureboot_enrolment: "{} enrolment files",
    /// Toast in `sb_sign`. `{}`: `name`.
    secureboot_signed: "{}, signed,",
    /// Text in `sb_check`.
    secureboot_signed_by_vault: "Signed by this vault's db key",
    /// Text in `sb_check`.
    secureboot_signed_by_other: "Signed by another key",
    /// Text in `sb_check`.
    secureboot_changed: "Changed since it was signed",
    /// Text in `sb_check`.
    secureboot_not_signed: "Not signed",
    /// Text in `sb_check`.
    secureboot_its_signature: "Its signature could not be read",
    // ----- seed_read (faraday-core/src/seeds/read.rs) -----
    /// Error in `seed_read_act`.
    seed_read_type_passphrase: "Type the passphrase",
    /// Text in `seed_read_wallet`. `{}`: `n + 1`.
    seed_read_key_already: "Key {} · already here",
    /// Text in `seed_read_take`. `{}`: `k.label`.
    seed_read_key_passphrase: "{} · passphrase",
    /// Text in `sheet`.
    seed_read_not_this_one: "Not this one",
    /// Text in `sheet`.
    seed_read_cancel: "Cancel",
    /// Text in `sheet`.
    seed_read_use_passphrase: "Use with this passphrase",
    /// Text in `sheet`.
    seed_read_use_without: "Use without a passphrase",
    /// Text in `sheet`.
    seed_read_add_a_passphrase: "Add a passphrase",
    /// Label in `sheet`.
    seed_read_no_passphrase: "Without a passphrase",
    /// Label in `sheet`.
    seed_read_passphrase_label: "Passphrase",
    /// Label in `sheet`.
    seed_read_passphrase: "Passphrase again",
    /// Label in `sheet`.
    seed_read_with_passphrase: "With this passphrase",
    /// Line in `sheet`.
    seed_read_the_two_passphrases: "The two passphrases differ",
    /// Text in `sheet`.
    seed_read_seedqr_read: "SeedQR read",
    // ----- seeds (faraday-core/src/seeds/mod.rs) -----
    /// Error in `seeds_take_file`.
    seeds_no_account_key: "No account key in that file",
    /// Error in `seeds_descriptor`.
    seeds_type_a_derivation: "Type a derivation path, like m/48'/0'/0'/2'",
    /// Error in `seeds_descriptor`.
    seeds_seed_unloaded: "A seed is no longer loaded",
    /// Error in `seeds_descriptor`.
    seeds_no_seed_yet: "No seed yet",
    /// Text in `seeds_descriptor`. `{}`: `at`, `crate::wallet::network_name(kn)`, `crate::wallet::network_name(net)`.
    seeds_key_is_a_key: "Key {} is a {} key; this session is on {}",
    /// Error in `seeds_descriptor`. `{}`: `at`.
    seeds_key_has_no_account: "Key {} has no account key yet",
    /// Error in `seeds_descriptor`. `{}`: `at`.
    seeds_not_account_key: "Key {} is not an account key",
    /// Error in `seeds_descriptor`. `{}`: `keys.len()`, `s.n`.
    seeds_of_keys_in_hand: "{} of {} keys in hand",
    /// Toast in `seeds_from_vault`. `{}`: `want`, `self.vaults.open[v].name`.
    seeds_loaded_from_vault: "{} loaded from {}",
    /// Text in `seeds_make`.
    seeds_typed_seed: "Typed seed",
    /// Text in `seeds_make`.
    seeds_typed_seeds: "Typed seeds",
    // ----- seeds (faraday-core/src/seeds/screen.rs) -----
    /// Text in `more_ways`. `{}`: `fp`, `name`.
    seeds_load_from_vault: "Load {} from {}",
    /// Text in `quorum`. `{}`: `s.m.max(1)`, `n`.
    seeds_quorum: "{} of {}",
    /// Text in `quorum`.
    seeds_one_key_signs: "one key signs",
    /// Text in `quorum`. `{}`: `s.m`, `if s.m == 1 { EN.seeds_signature } el…`, `n`.
    seeds_of_keys: "{} {} of {} keys",
    /// Text in `quorum`.
    seeds_signature: "signature",
    /// Text in `quorum`.
    seeds_signatures_inline: "signatures",
    /// Text in `quorum`.
    seeds_signatures: "Signatures",
    /// Text in `quorum`.
    seeds_keys: "Keys",
    /// Button in `cosigner`.
    seeds_change: "Change",
    /// Label in `cosigner`.
    seeds_type_or_paste: "Type or paste an xpub",
    /// Line in `cosigner`.
    seeds_not_an_account: "Not an account key",
    /// Text in `cosigner`. `{}`: `n`.
    seeds_use: "Use {}",
    /// Text in `cosigner`.
    seeds_scan: "Scan",
    /// Line in `shape`.
    seeds_cosigners_account: "Cosigners' account keys (xpub)",
    /// Line in `shape`.
    seeds_kind: "Kind",
    /// Line in `shape`.
    seeds_derivation_path: "Derivation path",
    /// Line in `shape`.
    seeds_standard: "Standard",
    /// Text in `shape`.
    seeds_account: "Account",
    /// Line in `shape`.
    seeds_custom_path: "Custom path",
    /// Line in `shape`.
    seeds_first_receive: "First receive address · 0/0",
    /// Text in `shape`.
    seeds_back_to_the_seeds: "Back to the seeds",
    /// Text in `shape`.
    seeds_make_the_wallet: "Make the wallet",
    // ----- set_time (faraday-core/src/set_time.rs) -----
    /// Text in `now_local_text`.
    set_time_not_known: "Not known",
    /// Toast in `set_time_set`. `{}`: `h`, `mi`.
    set_time_done_toast: "Time set to {}:{}",
    // ----- silent (faraday-core/src/silent/mod.rs) -----
    /// Toast in `silent_open`.
    silent_load_a_key_first: "Load a key first",
    /// Error in `silent_code`.
    silent_not_open: "Silent payments is not open",
    /// Error in `silent_code`.
    silent_choose_a_key: "Choose a key first",
    /// Text in `silent_code`. `{}`: `fp`.
    silent_silent_payment: "Silent payment address · {}",
    /// Error in `silent_code`.
    silent_no_address: "No address for this key",
    /// Text in `silent_code`. `{}`: `s.label`.
    silent_label: "Label {}",
    /// Text in `silent_code`.
    silent_public_receives: "Public: receives payments, spends nothing",
    /// Text in `silent_code`.
    silent_silent_payments_address: "Silent payments address",
    /// Error in `silent_code`.
    silent_no_record: "No record for this key",
    /// Text in `silent_code`.
    silent_public_watch: "Public: watch only, spends nothing",
    /// Text in `silent_code`.
    silent_silent_payments_record: "Silent payments record",
    /// Toast in `silent_act`.
    silent_no_vault_open: "No vault is open",
    /// Text in `silent_act`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    silent_silent_payments_scan: "Silent payments scan key · {}",
    /// Text in `silent_act`. `{}`: `title`.
    silent_is_in_the_vault: "{} is in the vault",
    /// Text in `silent_act`.
    silent_a_silent_payments: "A silent payments scan key",
    /// Text in `silent_act`.
    silent_whoever_has: "Whoever has it sees every payment to this address; it cannot spend them",
    /// Toast in `silent_add_wallet`.
    silent_load_the_key: "Load the key first",
    /// Text in `silent_add_wallet`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    silent_silent_payments_n: "Silent payments {}",
    /// Text in `silent_add_wallet`.
    silent_silent_payments: "Silent payments",
    /// Toast in `silent_add_wallet`.
    silent_added_to_your: "Added to your wallets",
    /// Toast in `silent_check`.
    silent_not_a_transaction: "Not a transaction",
    // ----- silent (faraday-core/src/silent/screen.rs) -----
    /// Text in `TITLES`.
    silent_key: "Key",
    /// Text in `TITLES`.
    silent_address: "Address",
    /// Text in `TITLES`.
    silent_scan_key: "Scan key",
    /// Text in `TITLES`.
    silent_check_a_payment: "Check a payment",
    /// Text in `draw`.
    silent_wallets: "Wallets",
    /// Text in `summary`.
    silent_not_chosen: "Not chosen",
    /// Text in `summary`.
    silent_a_secret_shows: "A secret: shows every payment",
    /// Text in `summary`.
    silent_needs_a_previous: "Needs a previous transaction",
    /// Text in `summary`.
    silent_a_transaction: "A transaction from Files",
    /// Text in `guide`.
    silent_a_silent_payments_address: "A silent payments address is one address you can give out many times: each payment to it lands at an address nobody else can link to it or to the others.",
    /// Text in `guide`.
    silent_give_this_address: "Give this address to whoever pays you. A label makes another address of the same wallet, so you can tell payers apart; the record lists the labels for the software that watches.",
    /// Text in `guide`.
    silent_finding_payments: "Finding payments needs the scan key: it shows every payment to this address, though it cannot spend them. It goes into the vault, or to a scanning wallet you run yourself.",
    /// Text in `guide`.
    silent_guide_payment: "Copy the signed transaction into Files with the transactions it spends from. This works out whether any output pays this wallet, and to which label.",
    /// Line in `card`.
    silent_its_key_is_not_loaded: "Its key is not loaded: labels, the scan key and a check need it",
    /// Text in `card`.
    silent_add_its_key: "Add its key",
    /// Text in `card`.
    silent_no_label: "No label",
    /// Text in `card`.
    silent_show_as_qr: "Show as QR",
    /// Text in `card`.
    silent_as_a_bitcoin: "As a bitcoin: link",
    /// Text in `card`. `{}`: `app.medium.for_the()`.
    silent_record: "Record {}",
    /// Text in `card`.
    silent_record_as_qr: "Record as QR",
    /// Text in `card`.
    silent_record_as_png: "Record as PNG",
    /// Text in `card`.
    silent_add_as_a_wallet: "Add as a wallet",
    /// Button in `card`.
    silent_continue: "Continue",
    /// Chip in `card`.
    silent_secret_shows: "Secret · shows every payment, cannot spend",
    /// Text in `card`. `{}`: `v.name`.
    silent_save_into: "Save into {}",
    /// Text in `card`.
    silent_out_unprotected: "Out unprotected…",
    /// Text in `checked_text`. `{}`: `n`, `if n == 1 { EN.silent_output } else {…`.
    silent_pays_this_wallet: "Pays this wallet: {} {}",
    /// Text in `checked_text`.
    silent_output: "output",
    /// Text in `checked_text`.
    silent_outputs: "outputs",
    /// Text in `checked_text`.
    silent_pays_nothing: "Pays nothing to this wallet",
    /// Text in `checked_text`.
    silent_spends_nothing: "Spends nothing a silent payment is made from",
    /// Text in `checked_text`.
    silent_input_key_missing: "Not signed: an input's key is not in it yet",
    /// Text in `checked_text`.
    silent_key_not_loaded: "Its key is not loaded",
    /// Text in `check_card`.
    silent_the_address: "the address",
    /// Text in `check_card`.
    silent_change: "change",
    /// Text in `check_card`. `{}`: `m`.
    silent_label_inline: "label {}",
    /// Text in `check_card`. `{}`: `p.vout`, `crate::ui::btc(p.amount.to_sat())`, `to`.
    silent_output_row: "Output {} · {} · to {}",
    /// Text in `check_card`. `{}`: `txid`.
    silent_needs_the_transaction: "Needs the transaction it spends from, {}: copy it into Files and check again",
    /// Text in `check_card`. `{}`: `it.name`.
    silent_check: "Check {}",
    /// Line in `check_card`.
    silent_no_transaction: "No transaction in Files",
    // ----- stick_settings (faraday-core/src/stick/settings.rs) -----
    /// Text in `settings_event`. `{}`: `wrote_as`.
    stick_settings_wrote: "Wrote {}, read back and matched",
    /// Text in `settings_event`. `{}`: `name`, `reason`.
    stick_settings_not_written: "{} not written: {}",
    // ----- stick_wipe (faraday-core/src/stick/wipe.rs) -----
    /// Text in `LIMIT`.
    stick_wipe_no_zeroing: "No zeroing is a certain erase. A USB stick or SD card can keep old copies of a file's blocks where no write reaches: its controller moves data around inside the chip. Zeros stop recovery tools; a lab that reads the chip itself may still find them. Only destroying the stick is certain.",
    /// Text in `REACH`.
    stick_wipe_faraday: "Faraday only ever writes into this partition, so zeroing it reaches everything Faraday ever wrote to the stick.",
    /// Text in `SECRET`.
    stick_wipe_if_a_seed: "If a seed or other secret was ever saved to this stick unencrypted, the only sure way to remove it is to destroy the stick.",
    /// Text in `WITHIN`.
    stick_wipe_within: "Only this stick's files partition is written. Its other partitions, its partition table and any space outside a partition are not touched.",
    /// Text in `BOUNDARY`.
    stick_wipe_boundary_why: "Why: the process that reads sticks takes in whatever a stick brings, so it is given the files partition and nothing else. It can never write a boot partition or a partition table, where a bad stick could leave code for the next computer to boot.",
    /// Text in `size_text`. `{}`: `b as f64 / f64::from(1u32 << 30)`.
    stick_wipe_gb: "{} GB",
    /// Text in `size_text`. `{}`: `b >> 20`.
    stick_wipe_mb: "{} MB",
    /// Text in `size_text`. `{}`: `(b >> 10).max(1)`.
    stick_wipe_kb: "{} KB",
    /// Text in `minutes_text`.
    stick_wipe_about_minute: "about a minute",
    /// Text in `minutes_text`. `{}`: `m`.
    stick_wipe_about_minutes: "about {} minutes",
    /// Text in `minutes_text`. `{}`: `m as f64 / 60.0`.
    stick_wipe_about_hours: "about {} hours",
    /// Text in `wiped`. `{}`: `name`.
    stick_wipe_deleted: "{} deleted, zeros over it first",
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
    stick_wipe_zeros_over: "Zeros over each, then it is removed with its name",
    /// Text in `delete_sheet`.
    stick_wipe_the_free: "The free space",
    /// Text in `delete_sheet`.
    stick_wipe_the_files: "The files stay; what earlier deletes left is zeroed",
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
    // ----- tools (faraday-core/src/tools/screen.rs) -----
    /// Text in `GUIDE`.
    tools_calculators_each: "Calculators: each takes what you type, works one thing out and keeps nothing. None of them touches a key; an extended private key is refused.",
    /// Line in `header`.
    tools_wallets: "Wallets",
    /// Title in `header`.
    tools_tools: "Tools",
    /// Text in `body`.
    tools_read_as_automatic: "Read as: automatic",
    /// Text in `body`.
    tools_text: "Text",
    /// Text in `body`.
    tools_hex: "Hex",
    /// Text in `body`. `{}`: `n`.
    tools_unit_option: "In {}",
    /// Text in `body`.
    tools_text_or_hex: "Text or hex",
    /// Text in `body`.
    tools_base58_bech32: "Base58, bech32 or hex",
    /// Text in `body`.
    tools_a_descriptor: "A descriptor",
    /// Text in `body`.
    tools_an_xpub_ypub: "An xpub, ypub, zpub, tpub…",
    /// Text in `body`.
    tools_an_amount: "An amount",
    /// Text in `body`.
    tools_a_policy_e_g: "A policy, e.g. or(pk(9a6a2580),and(pk(…),older(1000)))",
    /// Button in `body`.
    tools_clear: "Clear",
    /// Text in `body`.
    tools_encoding: "Encoding",
    /// Text in `body`.
    tools_prefix: "Prefix",
    /// Text in `body`.
    tools_version: "Version",
    /// Text in `body`.
    tools_bytes: "Bytes",
    /// Note in `body`.
    tools_not_base58_bech32: "Not Base58, bech32, bech32m or hex",
    /// Text in `body`.
    tools_checksum: "Checksum",
    /// Text in `body`.
    tools_given: "Given",
    /// Text in `body`.
    tools_holds: "holds",
    /// Text in `body`.
    tools_does_not_hold: "does not hold",
    /// Text in `body`.
    tools_with_checksum_label: "With it",
    /// Text in `body`.
    tools_not_loadable: "not one this device loads",
    /// Note in `body`.
    tools_not_a_descriptor: "Not a descriptor",
    /// Text in `body`.
    tools_legacy: "Legacy",
    /// Text in `body`.
    tools_nested_segwit: "Nested SegWit",
    /// Text in `body`.
    tools_native_segwit: "Native SegWit",
    /// Text in `body`.
    tools_taproot: "Taproot",
    /// Text in `body`.
    tools_depth: "Depth",
    /// Text in `body`.
    tools_fingerprint: "Fingerprint",
    /// Text in `body`.
    tools_child: "Child",
    /// Text in `body`.
    tools_that_is_an_extended: "That is an extended private key: this tool takes public keys only",
    /// Note in `body`.
    tools_not_an_extended: "Not an extended key",
    /// Text in `body`.
    tools_descriptor: "Descriptor",
    /// Text in `body`.
    tools_spend_paths: "Spend paths",
    /// Text in `body`.
    tools_wallet: "Wallet",
    /// Text in `body`.
    tools_one_this_device: "one this device loads",
    /// Text in `body`.
    tools_keys_need_origins: "keys need origins and both chains to load",
    // ----- transfer (faraday-core/src/transfer/mod.rs) -----
    /// Text in `transfer_send`. `{}`: `self.medium.a()`.
    transfer_larger_than: "Larger than 256 KiB: carry it on {}",
    /// Text in `transfer_send`. `{}`: `base_name(&path)`.
    transfer_is_empty: "{} is empty",
    /// Note in `transfer_scanned`. `{}`: `name`.
    transfer_saving: "Saving {}",
    /// Text in `transfer_saved`. `{}`: `path`.
    transfer_saved_secret: "Saved {}: this file holds a secret",
    /// Text in `transfer_saved`. `{}`: `path`.
    transfer_saved: "Saved {}",
    /// Error in `transfer_saved`. `{}`: `reason`.
    transfer_not_saved: "Not saved: {}",
    // ----- transfer (faraday-core/src/transfer/screen.rs) -----
    /// Text in `size_text`. `{}`: `bytes as f32 / 1e6`.
    transfer_mb: "{} MB",
    /// Text in `size_text`. `{}`: `bytes as f32 / 1e3`.
    transfer_kb: "{} KB",
    /// Title in `draw`.
    transfer_transfer: "Transfer",
    /// Section label in `send`.
    transfer_send: "Send",
    /// Text in `send`. `{}`: `name`.
    transfer_reading: "Reading {}",
    /// Line in `send`.
    transfer_no_files: "No files",
    /// Text in `send`.
    transfer_over_256_kib: "Over 256 KiB",
    /// Section label in `receive`.
    transfer_receive: "Receive",
    /// Button in `receive`.
    transfer_scan_camera: "Scan with the camera",
    /// Button in `receive`.
    transfer_open_folder: "Open folder",
    // ----- ui (faraday-core/src/ui.rs) -----
    /// Settings' Appearance row of the ten themes (`docs/THEMES.md` §12).
    ui_theme_row: "Theme",
    /// Settings' Appearance row of Light and Dark.
    ui_mode_row: "Mode",
    /// Settings' Appearance row of the contrast levels.
    ui_contrast_row: "Contrast",
    /// A mode, in Mode's segmented button.
    ui_light: "Light",
    /// A mode, in Mode's segmented button.
    ui_dark: "Dark",
    /// A contrast level, in Contrast's segmented button.
    ui_contrast_standard: "Standard",
    /// A contrast level, in Contrast's segmented button.
    ui_contrast_medium: "Medium",
    /// A contrast level, in Contrast's segmented button.
    ui_contrast_high: "High",
    /// A theme's name, on its chip.
    ui_cobalt: "Cobalt",
    /// A theme's name, on its chip.
    ui_bitcoin_orange: "Bitcoin Orange",
    /// A theme's name, on its chip.
    ui_lamplight: "Lamplight",
    /// A theme's name, on its chip.
    ui_mars: "Mars",
    /// A theme's name, on its chip.
    ui_forest: "Forest",
    /// A theme's name, on its chip.
    ui_frost: "Frost",
    /// A theme's name, on its chip.
    ui_dusk: "Dusk",
    /// A theme's name, on its chip.
    ui_lavender: "Lavender",
    /// A theme's name, on its chip.
    ui_rose: "Rose",
    /// A theme's name, on its chip.
    ui_monochrome: "Monochrome",
    /// Mark on the chip of the theme a first start is in.
    ui_default_theme: "Default",
    // ----- upgrade (faraday-core/src/upgrade/mod.rs) -----
    /// Text in `version`.
    upgrade_0_1_0_or_earlier: "0.1.0 or earlier",
    /// Text in `describe`. `{}`: `number`, `commit`.
    upgrade_test_release_short: "{} test release ({})",
    /// Text in `describe`. `{}`: `number`, `commit`, `fp`.
    upgrade_test_release: "{} test release ({}, changes {})",
    /// Text in `describe`. `{}`: `number`, `commit`.
    upgrade_dev: "{} dev ({})",
    /// Text in `describe`. `{}`: `number`, `commit`, `fp`.
    upgrade_dev_changes: "{} dev ({}, changes {})",
    // ----- upgrade (faraday-core/src/upgrade/screen.rs) -----
    /// Text in `source`. `{}`: `app.medium.noun()`.
    upgrade_insert_medium: "Insert the {} Faraday started from",
    /// Label in `source`.
    upgrade_read: "Read",
    /// Label in `source`.
    upgrade_reading: "Reading",
    /// Text in `target`. `{}`: `app.medium.noun()`.
    upgrade_remove_it_insert: "Remove it. Insert the {} to upgrade",
    /// Text in `target`.
    upgrade_file: "file",
    /// Text in `target`.
    upgrade_files: "files",
    /// Label in `target`.
    upgrade_its_data_partition: "Its data partition",
    /// Label in `target`.
    upgrade_target_contents: "On it",
    /// Label in `target`.
    upgrade_to_be_written: "To be written",
    /// Text in `target`.
    upgrade_newer_than: "Newer than this Faraday",
    /// Text in `target`.
    upgrade_writes_a_dev: "Writes a dev build, with a serial console",
    /// Text in `target`.
    upgrade_same_version: "Already this version",
    /// Text in `target`. `{}`: `has`, `needs`.
    upgrade_boot_partition: "Boot partition {} MB · needs {} MB",
    /// Text in `target`.
    upgrade_removed_during: "Removed during the write: it does not start until upgraded again · its data partition was not written",
    /// Text in `target`. `{}`: `why`.
    upgrade_not_written: "Not written: {}",
    /// Text in `target`.
    upgrade_writing: "Writing",
    /// Text in `target`.
    upgrade_upgrade: "Upgrade",
    /// Label in `done`.
    upgrade_written_read: "Written, read back and matched",
    /// Text in `done`. `{}`: `app.medium.noun()`.
    upgrade_remove_medium: "Remove the {}",
    /// Text in `done`. `{}`: `app.medium.noun()`.
    upgrade_upgrade_another: "Upgrade another {}",
    /// Text in `done`.
    upgrade_done: "Done",
    // ----- upgrade (faraday-boot/src/lib.rs) -----
    /// Text in `read_source`. `{}`: how many boot partitions hold it.
    upgrade_several_hold: "{} sticks in carry this version · Remove all but the one Faraday started from",
    /// Text in `read_source`. `{}`: its MB, this Faraday's MB.
    upgrade_source_size: "The stick carrying this version has a {} MB boot partition; this Faraday's is {} MB",
    // ----- vanity (faraday-core/src/vanity/mod.rs) -----
    /// Text in `script_name`.
    vanity_legacy: "Legacy",
    /// Text in `script_name`.
    vanity_nested_segwit: "Nested SegWit",
    /// Text in `script_name`.
    vanity_native_segwit: "Native SegWit",
    /// Text in `script_name`.
    vanity_taproot: "Taproot",
    /// Text in `script_name`. `{}`: `name`.
    vanity_m_or_n: "{} · m… or n…",
    /// Text in `dial_name`.
    vanity_account_number: "Account number",
    /// Text in `dial_name`.
    vanity_the_same_words: "The same words and passphrase, a different account: fast. The wallet is the key's own, at the account found.",
    /// Text in `dial_name`.
    vanity_passphrase: "Passphrase",
    /// Text in `dial_name`.
    vanity_a_few_characters: "A few characters added to the key's BIP-39 passphrase: slow, one PBKDF2 a try. The words with that passphrase are another wallet, which needs the passphrase kept with them.",
    /// Toast in `vanity_open`.
    vanity_load_a_key_first: "Load a key first",
    /// Error in `vanity_act`.
    vanity_type_at_least: "Type at least one character after the fixed part",
    /// Text in `vanity_act`.
    vanity_this_key_was: "This key was not loaded from BIP-39 words: only its account turns",
    /// Error in `vanity_tick`.
    vanity_every_counter: "Every counter was tried: no address begins that way",
    /// Text in `vanity_use`. `{}`: `label`.
    vanity_vanity_passphrase: "{} · vanity passphrase",
    /// Text in `vanity_use`.
    vanity_vanity_search: "Vanity search",
    /// Toast in `vanity_use`. `{}`: `name`.
    vanity_is_open: "{} is open",
    /// Error in `vanity_key_in`. `{}`: `c`.
    vanity_cannot_stand: "{} cannot stand there in this kind of address",
    // ----- vanity (faraday-core/src/vanity/screen.rs) -----
    /// Text in `TITLES`.
    vanity_key: "Key",
    /// Text in `TITLES`.
    vanity_what_turns: "What turns",
    /// Text in `TITLES`.
    vanity_kind_of_address: "Kind of address",
    /// Text in `TITLES`.
    vanity_characters: "Characters",
    /// Text in `TITLES`.
    vanity_search: "Search",
    /// Text in `draw`.
    vanity_wallets: "Wallets",
    /// Text in `draw`.
    vanity_vanity_address: "Vanity address",
    /// Text in `guide`.
    vanity_the_key_whose: "The key whose address is searched. Nothing about the key changes.",
    /// Text in `guide`.
    vanity_a_search_tries: "A search tries one account number, or one passphrase, after another until the first receive address begins with the characters chosen. The same key and the same number give the same address on any device.",
    /// Text in `guide`.
    vanity_guide_kind: "The kind of address decides the characters it starts with.",
    /// Text in `guide`.
    vanity_type_the_characters: "Type the characters the address should begin with, after the part every address of this kind shares. Each character more takes 32 times as long to find for a bech32 address, and 58 times for a legacy one.",
    /// Text in `guide`.
    vanity_the_search_runs: "The search runs while this screen is open and stops at the first match. The time is an expectation, not a promise: a find can come much sooner or later.",
    /// Text in `summary`.
    vanity_not_chosen: "Not chosen",
    /// Text in `summary`. `{}`: `thousands(v.grind.tested)`.
    vanity_tried_count: "{} tried",
    /// Text in `card`. `{}`: `thousands(expected)`.
    vanity_about_tries: "About {} tries",
    /// Button in `card`.
    vanity_continue: "Continue",
    /// Line in `card`.
    vanity_found: "Found",
    /// Text in `card`. `{}`: `f.account`, `thousands(v.grind.tested)`.
    vanity_found_account: "Account {} of this key, after {} tries",
    /// Line in `card`.
    vanity_added_to_passphrase: "Added to the passphrase:",
    /// Text in `card`.
    vanity_hide: "Hide",
    /// Text in `card`.
    vanity_show: "Show",
    /// Line in `card`.
    vanity_these_characters: "These characters are part of the passphrase: write them down with it. The words with any other passphrase open another wallet.",
    /// Button in `card`.
    vanity_open_this_wallet: "Open this wallet",
    /// Button in `card`.
    vanity_search_again: "Search again",
    /// Text in `card`. `{}`: `thousands(r as u64)`.
    vanity_a_second: "{} a second",
    /// Text in `card`, after a space. `{}`: `seconds_text(s)`.
    vanity_about_expected: "· about {} expected",
    /// Text in `card`.
    vanity_measuring: "Measuring",
    /// Text in `card`. `{}`: `v.grind.prefix.as_str()`.
    vanity_searching_prefix: "Searching for {}",
    /// Text in `card`. `{}`: `thousands(v.grind.tested)`, `rate`, `left`.
    vanity_tried: "{} tried · {}{}",
    /// Button in `card`.
    vanity_stop: "Stop",
    /// Text in `card`. `{}`: `v.grind.prefix.as_str()`.
    vanity_search_prefix: "Search for {}",
    /// Text in `seconds_text`. `{}`: `s / 60`.
    vanity_min: "{} min",
    /// Text in `seconds_text`. `{}`: `s / 86_400`.
    vanity_days: "{} days",
    // ----- vault (faraday-core/src/vault/screen.rs) -----
    /// Text in `gpg_file_size`. `{}`: `bytes as f32 / 1e6`.
    vault_mb: "{} MB",
    /// Text in `gpg_file_size`. `{}`: `bytes as f32 / 1e3`.
    vault_kb: "{} KB",
    /// Text in `stick_field`. `{}`: `medium.noun()`.
    vault_remove_the_first: "Remove the {} first",
    /// Text in `secret`.
    vault_release_to_hide: "Release to hide",
    /// Text in `secret`.
    vault_hold_to_show: "Hold to show",
    /// Button in `hold_delete`.
    vault_hold_to_delete: "Hold to delete from vault",
    /// Text in `unlock`. `{}`: `app.vaults.time_text(mem, f.header.co…`.
    vault_unlock_time_here: "{} here",
    /// Text in `list_line`.
    vault_unlock_to_see: "Unlock to see what it holds",
    /// Text in `ONCE_MORE`.
    vault_type_the_passphrase: "Type the passphrase once more to open it",
    /// Text in `unlock_back`.
    vault_import: "Import",
    /// Line in `list_compact`.
    vault_no_vault_in_files: "No vault in Files",
    /// Text in `list_compact`. `{}`: `app.medium.noun()`.
    vault_remove_to_unlock: "Remove the {} to unlock",
    /// Text in `list_compact`.
    vault_new_vault: "New vault",
    /// Text in `list_compact`.
    vault_locked_not_written: "Locked · not written yet",
    /// Text in `list_compact`.
    vault_locked_vault: "Locked vault",
    /// Text in `list_compact`.
    vault_locked: "Locked",
    /// Text in `list_compact`.
    vault_view_contents: "View contents",
    /// Text in `list_compact`. `{}`: `app.medium.a()`, `app.medium.for_the()`.
    vault_a_vault_holds: "A vault holds keys, wallets, entries and notes under one to four passphrases. It is copied in on {} visit and sealed {} again when the session locks with changes.",
    /// Text in `unlock_compact`. `{}`: `app.medium.noun()`.
    vault_remove_the_then_type: "Remove the {}, then type the passphrase",
    /// Line in `unlock_compact`.
    vault_no_locked_vault: "No locked vault in Files",
    /// Text in `unlock_compact`. `{}`: `t`.
    vault_unlocking: "Unlocking · {}",
    /// Button in `unlock_compact`.
    vault_unlock: "Unlock",
    /// Text in `unlock_compact`. `{}`: `mib_text(m)`.
    vault_this_computer_free: "This computer: {} free",
    /// Text in `unlock_compact`.
    vault_this_computer_memory: "This computer: memory not reported",
    /// Text in `unlock_compact`.
    vault_choose_the_vault: "Choose the vault and type a passphrase. Each passphrase opens its own contents, and a vault does not say how many it has. The time is the Argon2id cost the vault was made with, measured on this computer.",
    /// Text in `VSTEPS`.
    vault_where_will_you: "Where will you open it?",
    /// Text in `create`. `{}`: `app.back_name(s)`.
    vault_then: "Then: {}",
    /// Text in `create`.
    vault_home: "Home",
    /// Text in `create`.
    vault_create_a_vault: "Create a vault",
    /// Text in `create_guide`.
    vault_a_vault_opens_only: "A vault opens only on a machine with the memory its cost asks for. Choose every kind of machine it will ever be opened on; the cost suggested next fits the one with the least memory.",
    /// Text in `create_guide`.
    vault_guide_guess_cost: "Each guess at a passphrase costs this much memory and time; a machine without the memory cannot open the vault at all. The passphrase counts for more than the cost. The cost cannot be changed later.",
    /// Text in `create_guide`.
    vault_every_passphrase: "Every passphrase gets the same space, fixed for the life of the vault. 256 KiB holds keys, wallets, notes and about a hundred entries.",
    /// Text in `create_guide`.
    vault_a_vault_opens: "512 MiB opens on PCs; 64 MiB on a Raspberry Pi as well. The cost cannot be changed later. Customise sets the machines, the cost and the space one by one.",
    /// Text in `create_guide`.
    vault_each_passphrase: "Each passphrase opens its own contents, and the file does not say how many there are. They are fixed now: a change later means a new vault. Six EFF words are out of reach at any cost.",
    /// Text in `create_summaries`.
    vault_none_chosen: "None chosen",
    /// Text. `{}`: `name`, `mib_text(mem)`, `passes`.
    vault_cost_row: "{} · {} · {} passes",
    /// Text. `{}`: `size_text(c.slot)`, `file_text(file_len(c.slot))`.
    vault_file_n: "{} · file {}",
    /// Text. `{}`: `vault::vault_stem(&c.name.text)`, `n`, `if n == 1 { FEN.vault_passphrase_inline } …`, `if ok { FEN.vault_match } else { FEN.…`.
    vault_ofv: "{}.ofv · {} {} · {}",
    /// Text in `create_summaries`.
    vault_passphrase_inline: "passphrase",
    /// Text in `create_summaries`.
    vault_passphrases: "passphrases",
    /// Text in `create_summaries`.
    vault_match: "match",
    /// Text in `create_summaries`.
    vault_not_finished: "not finished",
    /// Text in `create_body`. `{}`: `mib_text(app.vaults.here_limit())`, `mib_text(free)`.
    vault_free: "{} ({} free)",
    /// Text in `create_body`.
    vault_memory_not_reported: "Memory not reported",
    /// Line in `create_body`.
    vault_this_computer: "This computer",
    /// Text in `create_body`.
    vault_choose_memory: "Choose memory and passes",
    /// Text in `create_body`.
    vault_suggested: "Suggested",
    /// Text in `create_body`.
    vault_fits: "Fits",
    /// Text in `create_body`. `{}`: `weakest`.
    vault_too_large: "Too large for {}",
    /// Text in `create_body`. `{}`: `mib_text(m)`, `p`, `vault::cost_bits(m, p)`.
    vault_passes_bits: "{} · {} passes · ≈{} bits",
    /// Line in `create_body`.
    vault_memory: "Memory",
    /// Line in `create_body`.
    vault_passes_label: "Passes",
    /// Text in `create_body`. `{}`: `mib_text(mem)`.
    vault_unlock_memory_n: "Unlock memory {}",
    /// Text in `create_body`. `{}`: `weakest`, `mib_text(limit)`.
    vault_memory_limit: "{}: up to {}",
    /// Text in `create_body`. `{}`: `vault::cost_bits(mem, passes)`.
    vault_cost_bits: "Adds ≈{} bits to every passphrase",
    /// Text in `create_body`. `{}`: `mib_text(mem)`, `passes`.
    vault_passes: "{} · {} passes",
    /// Text in `create_body`. `{}`: `app.vaults.time_text(mem, passes)`.
    vault_unlocks_in_here: "Unlocks in {} here",
    /// Text in `create_body`.
    vault_unlock_time_unmeasured: "Unlock time not measured yet",
    /// Text in `create_body`. `{}`: `cost`, `app.vaults.time_text(mem, passes)`.
    vault_here: "{} · {} here",
    /// Text in `create_body`. `{}`: `mib_text(f)`.
    vault_this_computer_has: "This computer has {} free",
    /// Button in `create_body`.
    vault_customise: "Customise",
    /// Text in `create_body`. `{}`: `file_text(file_len(*s))`.
    vault_file_n_inline: "file {}",
    /// Button in `create_body`.
    vault_continue: "Continue",
    /// Line in `create_body`.
    vault_name: "Name",
    /// Text in `create_body`. `{}`: `app.medium.noun()`.
    vault_what_it_holds: "What it holds, for anyone who sees the {} · leave empty for vault.ofv",
    /// Text in `create_body`. `{}`: `vault::vault_stem(&c.name.text)`.
    vault_file_ofv: "File {}.ofv",
    /// Text in `create_body`. `{}`: `app.medium.noun()`.
    vault_remove_the_then: "Remove the {}, then type the passphrases",
    /// Text in `create_body`.
    vault_hide_passphrases: "Hide passphrases",
    /// Text in `create_body`.
    vault_show_passphrases: "Show passphrases",
    /// Text in `create_body`. `{}`: `i + 1`.
    vault_passphrase_n: "Passphrase {}",
    /// Button in `create_body`.
    vault_remove: "Remove",
    /// Button in `create_body`.
    vault_generate_dice: "Generate with dice",
    /// Line in `create_body`.
    vault_or_type_your: "or type your own below",
    /// Text in `create_body`.
    vault_matches: "Matches",
    /// Text in `create_body`.
    vault_does_not_match: "Does not match yet",
    /// Line in `create_body`, in Practice, once the dice made a
    /// passphrase (`docs/DEMO.md` §5.2).
    vault_demo_uses_public: "Practice puts faraday in place of these words",
    /// Text in `create_body`.
    vault_type_it_then: "Type it, then type it again",
    /// Text in `create_body`. `{}`: `own`, `cost`, `own + cost`.
    vault_bits_from_dice: "{} bits from dice + ≈{} from the unlock cost ≈ {} bits",
    /// Text in `create_body`. `{}`: `cost`.
    vault_typed_its_own: "Typed: its own bits are not measured · the unlock cost adds ≈{}",
    /// Button in `create_body`.
    vault_add_another_passphrase: "Add another passphrase",
    /// Text in `dice_panel`. `{}`: `list.dice_per_word()`.
    vault_dice_per_word: "Roll {} dice for each word and type the numbers",
    /// Line in `dice_panel`.
    vault_no_word_yet: "No word yet",
    /// Text in `dice_panel`. `{}`: `words.len()`, `if words.len() == 1 { FEN.vault_word …`, `own`, `own + vault::cost_bits(mem, passes)`, `if left > 0 { format!(" · {left} of {…`.
    vault_dice_strength: "{} {} · {} bits · ≈{} with the unlock cost{}",
    /// Text in `dice_panel`, after a space. `{}`: `left`, `per`.
    vault_dice_for_the_next_word: "· {} of {} dice for the next word",
    /// Text in `dice_panel`.
    vault_word: "word",
    /// Text in `dice_panel`.
    vault_words_inline: "words",
    /// Button in `dice_panel`.
    vault_use_these_words: "Use these words",
    /// Line in `create_panel`.
    vault_your_choices: "Your choices",
    /// Text in `create_panel`.
    vault_opens_on_label: "Opens on",
    /// Text in `create_panel`.
    vault_unlock_cost: "Unlock cost",
    /// Text in `create_panel`.
    vault_space_per_passphrase: "Space per passphrase",
    /// Text in `create_panel`.
    vault_name_and_passphrases: "Name and passphrases",
    /// Text in `create_panel`.
    vault_not_chosen_yet: "Not chosen yet",
    /// Text in `create_summary`.
    vault_custom: "Custom",
    /// Text in `create_summary`. `{}`: `mib_text(mem)`.
    vault_this_computer_n: "This computer cannot allocate {}. A vault it could not open is never written.",
    /// Text in `create_summary`. `{}`: `name`, `mib_text(mem)`, `weakest`, `mib_text(limit)`.
    vault_memory_mismatch: "{} needs {}. {} gives a vault {} at most.",
    /// Text in `create_summary`. `{}`: `app.vaults.time_text(mem, passes)`.
    vault_unlocking_takes: "Unlocking takes {} here.",
    /// Text in `create_summary`.
    vault_raspberry_pi: "Raspberry Pi unlock times are not measured yet.",
    /// Line in `create_summary`.
    vault_will_create: "Will create",
    /// Text in `create_summary`.
    vault_no_passphrase: "No passphrase yet",
    /// Text in `create_summary`.
    vault_not_measured: "Not measured",
    /// Text in `create_summary`.
    vault_file: "File",
    /// Text in `create_summary`.
    vault_size: "Size",
    /// Text in `create_summary`.
    vault_unlock_memory: "Unlock memory",
    /// Text in `create_summary`.
    vault_unlock_here: "Unlock here",
    /// Text in `create_summary`.
    vault_unlock_cost_adds: "Unlock cost adds",
    /// Text in `create_summary`. `{}`: `cost`.
    vault_bits: "≈{} bits",
    /// Text in `create_summary`.
    vault_weakest_passphrase: "Weakest passphrase",
    /// Text in `create_summary`.
    vault_strength: "Strength",
    /// Text in `create_summary`.
    vault_goes_to_label: "Goes to",
    /// Text in `create_button`. `{}`: `app.vaults.time_text(mem, passes)`.
    vault_creating: "Creating · {}",
    /// Button in `create_button`.
    vault_create_vault: "Create vault",
    /// Text in `item_heading`.
    vault_key: "Key",
    /// Text in `item_heading`. `{}`: `m.word_count()`.
    vault_words_n: "{} words",
    /// Text in `item_heading`.
    vault_master_seed: "Master seed",
    /// Text in `item_heading`.
    vault_not_loaded_at_unlock_inline: "not loaded at unlock",
    /// Text in `item_heading`.
    vault_totp: "TOTP",
    /// Text in `item_heading`. `{}`: `t.chars().count()`.
    vault_note_characters: "Note · {} characters",
    /// Text in `item_heading`.
    vault_recovery_sheet: "Recovery sheet",
    /// Text in `item_heading`. `{}`: `name`.
    vault_recovery_sheet_n: "Recovery sheet · {}",
    /// Text in `item_heading`.
    vault_ed25519_does: "Ed25519 · does not expire",
    /// Text in `item_heading`. `{}`: `g.expires`.
    vault_ed25519_expires: "Ed25519 · expires {}",
    /// Text in `item_heading`.
    vault_owner_keys: "Owner keys",
    /// Text in `item_heading`: the Secure Boot keys a record holds.
    vault_pk_kek_db: "PK, KEK, db",
    /// Text in `item_heading`.
    vault_record: "Record",
    /// Text in `contents`.
    vault_vaults: "Vaults",
    /// Title in `contents`.
    vault_vault_contents: "Vault contents",
    /// Text in `NOTHING_YET`.
    vault_empty_line: "Nothing in it yet",
    /// Text in `add_label`.
    vault_make_keys: "Make keys",
    /// Text in `empty_buttons`. `{}`: `app.medium.a()`.
    vault_write_to_medium: "Write it to {}",
    /// Text in `add_full`.
    vault_save_a_bitcoin: "Save a bitcoin key",
    /// Text in `add_full`.
    vault_make_a_gpg_key: "Make a GPG key",
    /// Text in `add_short`.
    vault_bitcoin_key: "Bitcoin key",
    /// Text in `add_short`.
    vault_wallet: "Wallet",
    /// Text in `add_short`.
    vault_entry: "Entry",
    /// Text in `add_short`.
    vault_gpg_key: "GPG key",
    /// Text in `add_short`.
    vault_secure_boot: "Secure Boot",
    /// Line in `contents_compact`.
    vault_no_vault_is_open: "No vault is open",
    /// Text in `contents_compact`. `{}`: `v.changes`, `if v.changes == 1 { FEN.vault_change …`.
    vault_open_unsaved: "Open · {} unsaved {}",
    /// Text in `contents_compact`.
    vault_change: "change",
    /// Text in `contents_compact`.
    vault_changes: "changes",
    /// Button in `contents_compact`.
    vault_rename: "Rename",
    /// Text in `contents_compact`.
    vault_scan_a_code: "Scan a code",
    /// Text in `contents_compact`. `{}`: `entries_files`.
    vault_import_from_files: "Import from Files ({})",
    /// Text in `contents_compact`.
    vault_totp_codes: "TOTP codes",
    /// Line in `contents_compact`.
    vault_none_here: "None in this vault",
    /// Text in `contents_compact`. `{}`: `app.medium.noun()`.
    vault_secret_values: "Secret values show only while held. Changes stay in this session until it locks; locking seals them, and the next {} visit writes the vault back over its own file.",
    /// Text in `detail`.
    vault_rename_the_vault: "Rename the vault",
    /// Text in `detail`.
    vault_add_a_note: "Add a note",
    /// Text in `detail`.
    vault_edit_the_note: "Edit the note",
    /// Text in `detail`.
    vault_make_a_key: "Make a key",
    /// Text in `detail`.
    vault_make_secure_boot: "Make Secure Boot keys",
    /// Text in `detail`.
    vault_add_an_entry: "Add an entry",
    /// Text in `detail`.
    vault_edit_the_entry: "Edit the entry",
    /// Line in `detail`.
    vault_expires: "Expires",
    /// Text in `detail`.
    vault_make_the_key: "Make the key",
    /// Text in `detail`.
    vault_make_the_keys: "Make the keys",
    /// Text in `detail`.
    vault_load_with_a_bip: "Load with a BIP-39 passphrase",
    /// Line in `detail`.
    vault_passphrase: "Passphrase",
    /// Line in `detail`.
    vault_sealing: "Sealing",
    /// Text in `detail`.
    vault_open_button: "Open",
    /// Text in `detail`.
    vault_seal_and_export: "Seal and export",
    /// Text in `detail`.
    vault_load: "Load",
    /// Button in `detail`.
    vault_cancel: "Cancel",
    /// Text in `detail`. `{}`: `app.medium.from_the()`.
    vault_no_file: "No file {}",
    /// Chip in `detail`.
    vault_signed: "Signed",
    /// Button in `detail`.
    vault_back_to_the_key: "Back to the key",
    /// Label in `detail`.
    vault_words: "Words",
    /// Label in `detail`.
    vault_bip_39_passphrase: "BIP-39 passphrase",
    /// Text in `detail`.
    vault_stored: "Stored",
    /// Text in `detail`.
    vault_not_stored: "Not stored",
    /// Label in `detail`, and the action that turns it on (F62).
    vault_load_at_unlock: "Load at unlock",
    /// Text in `detail`: a key that loads with a wallet of its vault.
    /// `{}`: the wallet's name.
    vault_loads_with_wallet: "Loads with {}",
    /// Text in `detail`.
    vault_yes: "Yes",
    /// Text in `detail`.
    vault_no: "No",
    /// Label in `detail`.
    vault_in_this_session: "In this session",
    /// Text in `detail`.
    vault_not_loaded: "Not loaded",
    /// Text in `detail`.
    vault_load_into_session: "Load into session",
    /// Text in `detail`.
    vault_load_with_a_passphrase: "Load with a passphrase",
    /// Text in `detail`: the action that turns Load at unlock off.
    vault_dont_load_at_unlock: "Don't load at unlock",
    /// Title of the Load sheet an unlock shows (F62). `{}`: the vault.
    load_sheet_title: "Load from {}",
    /// The Load sheet's count: one wallet.
    load_sheet_wallet: "wallet",
    /// The Load sheet's count: wallets.
    load_sheet_wallets: "wallets",
    /// The Load sheet's count: one key.
    load_sheet_key: "key",
    /// The Load sheet's count: keys.
    load_sheet_keys: "keys",
    /// A wallet card's line when the vault holds none of its seeds.
    load_sheet_watch_only: "Watch-only here",
    /// A wallet card's line. `{}`: seeds not in the vault, then the
    /// wallet's seeds.
    load_sheet_kept_elsewhere: "{} of {} seeds kept elsewhere",
    /// A seed in one other wallet of the vault too. `{}`: that wallet.
    load_sheet_also_in_wallet: "also in {}",
    /// A seed in two other wallets of the vault. `{}`: each wallet.
    load_sheet_also_in_two: "also in {} and {}",
    /// A seed in three or more other wallets. `{}`: how many.
    load_sheet_also_in_n: "also in {} other wallets",
    /// What the session holds already, on the Load sheet.
    load_sheet_loaded: "Loaded",
    /// Title of the Load sheet's card of keys in no wallet.
    load_sheet_keys_card: "Keys",
    /// The Load sheet's button that loads nothing.
    load_sheet_not_now: "Not now",
    /// The Load sheet's button that loads what is ticked.
    load_sheet_load_ticked: "Load ticked",
    /// Title of the sheet of seeds an open vault keeps with no
    /// passphrase, from a wallet short of a key (F62).
    pass_seeds_title: "Seeds with no passphrase stored",
    /// Label of that sheet's passphrase field.
    pass_seeds_passphrase: "BIP-39 passphrase",
    /// The typed passphrase made another key. `{}`: its fingerprint, then
    /// the wallet's name.
    pass_seeds_not_a_key: "This passphrase gives {}, not a key of {}",
    /// That sheet's way back from a seed's field to the list.
    pass_seeds_other_seed: "Another seed",
    /// That sheet's button that loads the seed with the passphrase.
    pass_seeds_load: "Load",
    /// That sheet's button that closes it.
    pass_seeds_cancel: "Cancel",
    /// Text in `detail`.
    vault_open_in_wallets: "Open in Wallets",
    /// Label in `detail`.
    vault_username: "Username",
    /// Label in `detail`.
    vault_password: "Password",
    /// Label in `detail`.
    vault_url: "URL",
    /// Line in `detail`.
    vault_code: "Code",
    /// Button in `detail`.
    vault_set_the_time: "Set the time",
    /// Text in `detail`.
    vault_unreadable: "Unreadable",
    /// Label in `detail`.
    vault_totp_secret: "TOTP secret",
    /// Label in `detail`.
    vault_notes: "Notes",
    /// Text in `detail`.
    vault_export_for_keepass: "Export for KeePass",
    /// Label in `detail`.
    vault_text: "Text",
    /// Text in `detail`.
    vault_edit: "Edit",
    /// Label in `detail`.
    vault_descriptor: "Descriptor",
    /// Label in `detail`.
    vault_note: "Note",
    /// Label in `detail`.
    vault_fingerprint: "Fingerprint",
    /// Label in `detail`.
    vault_signing_subkey: "Signing subkey",
    /// Label in `detail`.
    vault_user_id: "User ID",
    /// Text in `detail`. `{}`: `g.created`.
    vault_gpg_no_expiry: "From {} · does not expire",
    /// Text in `detail`. `{}`: `g.created`, `g.expires`.
    vault_gpg_validity: "{} to {}",
    /// Label in `detail`.
    vault_valid: "Valid",
    /// Label in `detail`.
    vault_paperkey: "Paperkey",
    /// Line in `detail`.
    vault_renew_label: "Renew for",
    /// Button in `detail`.
    vault_renew: "Renew",
    /// Text in `detail`.
    vault_export_public: "Export public key",
    /// Text in `detail`.
    vault_revocation_certificate: "Revocation certificate",
    /// Text in `detail`.
    vault_show_as_qr: "Show as QR",
    /// Text in `detail`.
    vault_sign_a_file: "Sign a file",
    /// Label in `detail`.
    vault_owner_guid: "Owner GUID",
    /// Line in `detail`.
    vault_enrolment_adds: "Enrolment adds",
    /// Text in `detail`.
    vault_windows_compatible: "Windows-compatible",
    /// Text in `detail`.
    vault_own_keys_only: "Own keys only",
    /// Text in `detail`.
    vault_own_keys_only_warning: "Your PK, KEK and db only. Windows will not start, nor any card or controller whose firmware Microsoft signed",
    /// Text in `detail`.
    vault_yours_with_microsoft: "Yours, with Microsoft's KEK CAs and Windows CAs. Not Microsoft's third-party CA: a card or controller whose firmware it signed may not start",
    /// Text in `detail`.
    vault_enrolment_files: "Enrolment files",
    /// Text in `save_panel`.
    vault_save_a_key: "Save a key",
    /// Text in `save_panel`.
    vault_save_a_wallet: "Save a wallet",
    /// Text in `save_panel`. `{}`: `key.label`.
    vault_bip_39_passphrase_n: "{} · BIP-39 passphrase",
    /// Button in `save_panel`.
    vault_save_passphrase_button: "Save with its passphrase",
    /// Line in `save_panel`.
    vault_every_key_saved: "Every key in this session is in the vault",
    /// Button in `save_panel`, and beside each key or wallet in no vault
    /// on `lock_ask_sheet`.
    vault_save: "Save",
    /// Line in `save_panel`.
    vault_every_wallet: "Every wallet in this session is in the vault",
    /// Text in `idle_warn_sheet`, in place of Sealed while the system has
    /// given no randomness.
    vault_not_sealed: "Not sealed",
    /// Button in `lock_ask_sheet`, in place of Lock while the system has given
    /// no randomness: the changes are lost.
    vault_lock_without_sealing: "Lock without sealing",
    /// Button in `lock_ask_sheet`.
    vault_back_to_the_vault: "Back to the vault",
    /// Title and button in `lock_ask_sheet`.
    vault_lock: "Lock",
    /// The Then line in `lock_ask_sheet`.
    lock_ask_then: "Faraday restarts on Unlock",
    /// Text in `years_row`.
    vault_1_year: "1 year",
    /// Text in `years_row`.
    vault_2_years: "2 years",
    /// Text in `years_row`.
    vault_5_years: "5 years",
    /// Text in `years_row`.
    vault_never: "Never",
    /// Text in `images_panel`. `{}`: `item.name`.
    vault_sign_n: "Sign {}",
    /// Line in `images_panel`.
    vault_compare_build_record: "Compare it with the build record whose fingerprint you checked yourself",
    /// Button in `images_panel`.
    vault_hold_to_sign: "Hold to sign",
    /// Button in `images_panel`.
    vault_back: "Back",
    /// Line in `images_panel`.
    vault_sign_or_check: "Sign or check an image",
    /// Text in `images_panel`. `{}`: `app.medium.from_the()`.
    vault_no_efi_image: "No EFI image {}",
    /// Button in `images_panel`, and on the GPG signing page.
    vault_sign: "Sign",
    /// Button in `images_panel`.
    vault_check: "Check",
    /// Button in `images_panel`.
    vault_back_to_the_keys: "Back to the keys",
    // ----- vaults (faraday-core/src/vault/mod.rs) -----
    /// Text in `PRESETS`.
    vaults_light: "Light",
    /// Text in `PRESETS`.
    vaults_standard: "Standard",
    /// Text in `PRESETS`.
    vaults_medium: "Medium",
    /// Text in `PRESETS`.
    vaults_strong: "Strong",
    /// Text in `PRESETS`.
    vaults_maximum: "Maximum",
    /// Text in `SIZES`.
    vaults_pcs_only: "PCs only",
    /// Text in `SIZES`.
    vaults_pcs_and_a_raspberry: "PCs and a Raspberry Pi",
    /// Text in `MACHINES`.
    vaults_other_pcs: "Other PCs",
    /// Text in `MACHINES`.
    vaults_other_pcs_inline: "other PCs",
    /// A machine a vault may be opened on.
    vaults_raspberry_pi_5: "Raspberry Pi 5",
    /// Its short name.
    vaults_pi_5: "Pi 5",
    /// A machine a vault may be opened on.
    vaults_raspberry_pi_4: "Raspberry Pi 4",
    /// Its short name.
    vaults_pi_4: "Pi 4",
    /// A machine a vault may be opened on.
    vaults_raspberry_pi_3: "Raspberry Pi 3",
    /// Its short name.
    vaults_pi_3: "Pi 3",
    /// A machine a vault may be opened on, and its short name.
    vaults_pi_zero_2_w: "Pi Zero 2 W",
    /// Text in `CATEGORIES`.
    vaults_bitcoin_keys: "Bitcoin keys",
    /// Text in `CATEGORIES`.
    vaults_wallets: "Wallets",
    /// Text in `CATEGORIES`.
    vaults_entries: "Entries",
    /// Text in `CATEGORIES`.
    vaults_gpg_keys: "GPG keys",
    /// Text in `CATEGORIES`.
    vaults_secure_boot: "Secure Boot",
    /// Text in `dice_aim`. `{}`: `n`, `STRONG_BITS`.
    vaults_aim_for_words: "Aim for {} words or more on this list: the count turns green at {} bits, which this vault counts as strong. The lists and how to roll for them: eff.org/dice",
    /// Text in `mib_text`. `{}`: `f64::from(mib) / 1024.0`.
    vaults_gib: "{} GiB",
    /// Text in `size_text`. `{}`: `slot / 1_048_576`.
    vaults_mib: "{} MiB",
    /// Text in `size_text`. `{}`: `slot / 1024`.
    vaults_kib: "{} KiB",
    /// Text in `file_text`. `{}`: `len as f64 / 1e6`.
    vaults_mb: "{} MB",
    /// Text in `file_text`. `{}`: `len.div_ceil(1000)`.
    vaults_kb: "{} KB",
    /// Text in `cost_text`.
    vaults_pass: "pass",
    /// Text in `cost_text`.
    vaults_passes: "passes",
    /// Text in `time_text`. `{}`: `s.round() as u64`.
    vaults_about_s: "about {} s",
    /// Text in `time_text`.
    vaults_not_measured: "not measured yet",
    /// Text in `weakest`.
    vaults_this_computer_name: "This computer",
    /// Text in `kept_wiped`.
    vaults_file: "file",
    /// Text in `kept_wiped`.
    vaults_files: "files",
    /// Text in `kept_wiped`. `{}`: `v`.
    vaults_in_copy_checked: "in {} · copy checked",
    /// Text in `kept_wiped`.
    vaults_copy_checked: "copy checked",
    /// Text in `kept_wiped`. `{}`: `fp`.
    vaults_seed_n: "Seed {}",
    /// Text in `kept_wiped`. `{}`: `v`.
    vaults_kept_in_vault: "in {}",
    /// Text in `kept_wiped`.
    vaults_in_files: "in Files",
    /// Text in `kept_wiped`.
    vaults_nowhere_else: "nowhere else",
    /// Text in `vault_way`.
    vaults_make_a_vault: "Make a vault",
    /// Text in `vault_way`. `{}`: `files[k].name`.
    vaults_unlock: "Unlock {}",
    /// Text in `vault_way`.
    vaults_unlock_way: "Unlock a vault",
    /// Text in `back_name`.
    vaults_make_a_new_wallet: "Make a new wallet",
    /// Text in `back_name`.
    vaults_restore_an_existing: "Restore an existing wallet",
    /// Text in `back_name`.
    vaults_spend: "Spend",
    /// Text in `back_name`.
    vaults_back_up_a_wallet: "Back up a wallet",
    /// Text in `back_name`.
    vaults_child_seeds: "Child seeds and passwords",
    /// Text in `back_name`.
    vaults_lightning_node: "Lightning node key",
    /// Text in `back_name`.
    vaults_import: "Import",
    /// Text in `back_link_name`.
    vaults_tools: "Tools",
    /// Text in `vault_act`. `{}`: `keys`, `if keys == 1 { EN.vaults_seed_inline } else …`, `wallets`, `if wallets == 1 { EN.vaults_wallet_inline …`.
    vaults_and_loaded: "{} {} and {} {} loaded",
    /// Text in `vault_act`.
    vaults_wallet_inline: "wallet",
    /// Text in `vault_act`.
    vaults_wallets_inline: "wallets",
    /// Toast in `vault_import_entries`.
    vaults_those_entries: "Those entries do not fit in this vault's slot size",
    /// Text in `vault_scanned_entry`. `{}`: `n`, `if n == 1 { EN.vaults_entry_inline } else …`.
    vaults_added: "{} {} added",
    /// Text in `vault_scanned_entry`.
    vaults_entry_inline: "entry",
    /// Text in `vault_scanned_entry`.
    vaults_entries_inline: "entries",
    /// Error in `vault_scanned_entry`.
    vaults_not_a_totp_setup: "Not a TOTP setup code",
    /// Error in `vault_prompt_go`.
    vaults_a_passphrase: "A passphrase of 8 characters or more",
    /// Text in `vault_unlock_check`. `{}`: `self.medium.noun()`.
    vaults_remove_to_unlock: "Remove the {} to unlock",
    /// Text in `vault_unlock_check`.
    vaults_type_the_passphrase: "Type the passphrase",
    /// Text in `vault_unlock_check`. `{}`: `mib_text(need)`, `mib_text(free)`.
    vaults_needs_this_computer: "Needs {}. This computer has {} free.",
    /// Toast in `vault_unlock`. `{}`: `self.vaults.open[v].name`.
    vaults_unlocked: "{} unlocked",
    /// Toast in `unlock_landed`. `{}`: `what`, `from`.
    vaults_loaded_note: "{} loaded from {}",
    /// Text in `vault_rows`.
    vaults_seed: "Seed",
    /// Text in `vault_load_wallet`. `{}`: `keys`, `if keys == 1 { EN.vaults_seed_inline } else …`.
    vaults_loaded: "{} {} loaded",
    /// Text in `vault_load_wallet`.
    vaults_seed_inline: "seed",
    /// Text in `vault_load_wallet`.
    vaults_seeds: "seeds",
    /// Text in `vault_kdbx`.
    vaults_entry: "Entry",
    /// Toast in `vault_kdbx`. `{}`: `name`, `self.medium.for_the()`.
    vaults_waits_sealed: "{} waits {}, sealed",
    /// Error in `vault_kdbx`.
    vaults_entry_too_large_kdbx: "That entry is too large for a KeePass export",
    /// Toast in `vault_delete`.
    vaults_deleted_toast: "Deleted from the vault",
    /// Toast in `vault_load_selected`. `{}`: `crate::wallet::fp_text(fp)`.
    vaults_key_is_loaded: "Key {} is loaded",
    /// Text in `vault_load_selected`.
    vaults_vault: "Vault",
    /// Text in `vault_form`.
    vaults_title: "Title",
    /// Text in `vault_form`.
    vaults_username: "Username",
    /// Text in `vault_form`.
    vaults_password: "Password",
    /// Text in `vault_form`.
    vaults_url: "URL",
    /// Text in `vault_form`.
    vaults_notes: "Notes",
    /// Text in `vault_form`.
    vaults_totp_secret: "TOTP secret",
    /// Text in `vault_form`.
    vaults_text: "Text",
    /// Text in `vault_form`.
    vaults_name: "Name",
    /// Text in `vault_form`.
    vaults_email: "Email",
    /// Text in `vault_form`.
    vaults_owner_name: "Owner name",
    /// Toast in `vault_form_save`.
    vaults_made_the_certificate: "Made · the certificate. The revocation certificate is kept with the key",
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
    vaults_is_in_the_vault_as: "{} is in the vault as a wallet",
    /// Toast in `key_record`.
    vaults_this_key_has: "This key has no words to keep",
    /// Text in `vault_save_wallet`. `{}`: `name`.
    vaults_is_in_the_vault: "{} is in the vault",
    /// Toast in `vault_push_in`.
    vaults_slot_too_small: "That does not fit in this vault's slot size",
    /// Text in `vault_create_check`. `{}`: `medium.noun()`.
    vaults_remove_the_first: "Remove the {} first",
    /// Text in `vault_create_check`. `{}`: `n + 1`.
    vaults_finish_step: "Finish step {} first",
    /// Text in `vault_create_check`.
    vaults_every_passphrase: "Every passphrase needs a value",
    /// Text in `vault_create_check`.
    vaults_each_passphrase: "Each passphrase must be typed the same twice",
    /// Text in `vault_create_check`.
    vaults_two_passphrases: "Two passphrases are the same. Each must differ",
    /// Text in `vault_create_check`.
    vaults_choose_a_cost: "Choose a cost this computer can allocate",
    /// Error in `vault_create`.
    vaults_no_randomness: "No randomness from the system yet. Try again",
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
    /// nowhere yet (`docs/history/NEW-WALLET.md` §18).
    lock_held_pill: "Auto-lock off until backup complete",
    /// The lock and power sheets' wiped line then: the flow ends.
    lock_held_ends: "Make a new wallet · ends",
    /// Text in `line`.
    vaults_unchanged: "Unchanged",
    /// Text in `line`. `{}`: `self.wallets.len()`.
    vaults_wallets_n: "{} wallets",
    /// Text in `line`. `{}`: `self.keys[0]`.
    vaults_key: "key {}",
    /// Text in `line`. `{}`: `self.keys.join(", ")`.
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
    // ----- visit_taken (faraday-core/src/visit_taken.rs) -----
    /// Text in `line`. `{}`: `self.name`, `keep_as`.
    visit_taken_keep_both_n: "{} · Keep both: {}",
    /// Text in `line`. `{}`: `self.name`.
    visit_taken_the_same: "{} · the same vault, updated here",
    /// Text in `line`. `{}`: `self.name`, `keep_as`.
    visit_taken_another: "{} · another vault · kept, this one as {}",
    /// Text in `taken_sheet`. `{}`: `app.medium.on_the()`.
    visit_taken_already: "Already {}",
    /// Text in `taken_sheet`.
    visit_taken_replace: "Replace",
    /// Text in `taken_sheet`.
    visit_taken_keep_both: "Keep both",
    /// Text in `taken_sheet`.
    visit_taken_cancel: "Cancel",
    // ----- modes (faraday-core/src/modes/mod.rs) -----
    /// Text in `Mode::name`.
    mode_demo: "Practice",
    /// The pill in Practice: the mode and its network.
    mode_demo_pill: "Practice · Testnet",
    /// Title of the mode sheet.
    mode_sheet_title: "Mode",
    /// Button in the mode sheet.
    mode_not_now: "Not now",
    /// The line under every field where a seed or a secret is typed, on an
    /// online shell.
    never_real_seed: "Never type a seed that holds real bitcoin here.",
    /// Button in the "Demo only" card that puts a sample in the empty
    /// field of the page where a key or an input is typed (DEMO §14).
    use_a_sample: "Use a sample",
    /// Button in the "Demo only" card of Check my copy, its grid empty:
    /// it types the sample's words.
    fill_words: "Fill words",
    /// Button in Faraday Demo's strip, Practice section, while Add a key's
    /// words form is on screen: the clipboard's words go into it whole.
    paste_words: "Paste words",
    /// Refusal on the words form: the words pasted are not as many as a
    /// key has. `{}`: how many were pasted.
    paste_words_count: "{} words pasted. A key has 12, 15, 18, 21 or 24",
    /// The scanner's tile, and the step's button, that hands over the
    /// sample transaction.
    scan_sample_psbt: "Load sample PSBT",
    /// The scanner's tile that hands over a sample wallet.
    scan_sample_wallet: "Load sample wallet",
    /// The scanner's tile that hands over a sample seed.
    scan_sample_seed: "Load sample seed",
    /// The scanner's tile that hands over a sample share or part.
    scan_sample_share: "Load sample share",
    /// The scanner's tile that hands over a sample key.
    scan_sample_key: "Load sample key",
    /// The scanner's tile that hands over a sample SeedQR.
    scan_sample_seedqr: "Load sample SeedQR",
    /// The scanner's tile that hands over a sample code.
    scan_sample_code: "Load sample code",
    /// The scanner's tile that hands over a sample address.
    scan_sample_address: "Load sample address",
    // ----- samples (faraday-core/src/samples/) -----
    /// The Load samples pill, on the compact bar and Home.
    samples_pill_short: "Samples",
    /// Title of the Load samples sheet.
    samples_title: "Load samples",
    /// Heading of the sample wallets in Load samples.
    samples_wallets: "Wallets",
    /// Heading of the sample keys in Load samples.
    samples_keys: "Keys",
    /// Link in Load samples while none is ticked.
    samples_select_all: "Select all",
    /// Link in Load samples while any is ticked.
    samples_unselect_all: "Unselect all",
    /// Button in Load samples.
    samples_not_now: "Not now",
    /// Button in Load samples.
    samples_load: "Load ticked",
    /// Line in Load samples, followed by the passphrase in a code box.
    samples_password: "In Practice, the sample passphrase is",
    /// Toast after Load ticked. `{}`: what loaded, made of `samples_n_*`.
    samples_loaded: "Loaded {}",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_wallet: "{} wallet",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_wallets: "{} wallets",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_key: "{} key",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_keys: "{} keys",
    /// Text in `loaded_text`. `{}`: the wallets, the keys.
    samples_loaded_pair: "{} and {}",
    /// Where a sample wallet says it came from.
    samples_source: "Sample",
    /// Key row in Load samples.
    sample_abandon: "abandon … about",
    /// Key row in Load samples.
    sample_japanese: "Japanese words",
    /// Key row in Load samples.
    sample_slip39_one: "SLIP-39, one share",
    /// Key row in Load samples.
    sample_slip39_two_of_three: "SLIP-39, 2 of 3",
    /// Key row in Load samples.
    sample_slip39_groups: "SLIP-39, groups",
    /// Key row in Load samples.
    sample_codex32_secret: "codex32 secret",
    /// Key row in Load samples.
    sample_codex32_shares: "codex32, 2 shares",
    /// Key row in Load samples.
    sample_xor: "Seed XOR, 2 parts",
    /// Name of the LND cipher seed sample.
    sample_aezeed: "LND cipher seed",
    /// File name of the sample vault, which Use a sample puts in the Inbox.
    sample_vault_file: "sample.ofv",
    /// File name of the sample signed message, which Use a sample puts in
    /// the Inbox.
    sample_message_file: "sample-message.txt",
    /// Wallet row in Load samples.
    sample_savings: "Savings",
    /// Wallet row in Load samples.
    sample_spending: "Spending",
    /// Wallet row in Load samples.
    sample_taproot: "Taproot",
    /// Wallet row in Load samples.
    sample_nested: "Nested SegWit",
    /// Wallet row in Load samples.
    sample_legacy: "Legacy",
    /// Wallet row in Load samples.
    sample_nested_multisig: "Nested multisig",
    /// Wallet row in Load samples.
    sample_legacy_multisig: "Legacy multisig",
    /// Wallet row in Load samples.
    sample_taproot_multisig: "Taproot multisig",
    /// Wallet row in Load samples.
    sample_inheritance: "Inheritance",
    /// Wallet row in Load samples.
    sample_tree: "Taproot tree",
    /// Wallet row in Load samples.
    sample_musig: "MuSig2",
    /// Wallet row in Load samples.
    #[cfg(feature = "unreleased")]
    sample_frost: "FROST 2 of 3",
    /// Wallet row in Load samples.
    sample_three_of_five: "3 of 5",
    /// What a Load samples row is: its kind and its script, or its form and its size. `{}`: the two parts.
    sample_type: "{} · {}",
    /// Kind of a sample wallet with one key.
    sample_kind_single: "single-sig",
    /// Kind of a sample multisig wallet. `{}`: signatures required, keys.
    sample_kind_multisig: "{}-of-{} multisig",
    /// Kind of a sample MuSig2 wallet. `{}`: keys.
    sample_kind_musig: "{}-key MuSig2",
    /// Kind of a sample FROST wallet. `{}`: shares that sign, shares.
    #[cfg(feature = "unreleased")]
    sample_kind_frost: "{}-of-{} FROST",
    /// Kind of a sample wallet with a key and a second that spends after a delay.
    sample_kind_timelock: "timelock recovery",
    /// Script of a sample Taproot wallet with a tree of scripts.
    sample_script_tree: "Taproot, script tree",
    /// Form of a sample key made of BIP-39 words.
    sample_form_bip39: "BIP-39",
    /// Form of a sample key made of SLIP-39 shares.
    sample_form_slip39: "SLIP-39",
    /// Form of a sample key made of codex32 strings.
    sample_form_codex32: "codex32",
    /// Form of a sample key made of Seed XOR parts.
    sample_form_xor: "Seed XOR",
    /// Form of a sample key that is a master seed.
    sample_form_seed: "Master seed",
    /// Form of the sample LND cipher seed.
    sample_form_aezeed: "LND cipher seed",
    /// Size of a sample key of BIP-39 words. `{}`: how many.
    sample_size_words: "{} words",
    /// Size of a sample key of Japanese BIP-39 words. `{}`: how many.
    sample_size_words_japanese: "{} words, Japanese",
    /// Size of a sample key of one share.
    sample_size_share: "1 share",
    /// Size of a sample key of shares. `{}`: how many.
    sample_size_shares: "{} shares",
    /// Size of a sample SLIP-39 set. `{}`: shares that recover the key, shares dealt.
    sample_size_threshold: "{} of {} shares",
    /// Size of a sample SLIP-39 set in groups. `{}`: shares.
    sample_size_groups: "groups, {} shares",
    /// Size of the sample codex32 secret.
    sample_size_secret: "secret",
    /// Size of a sample Seed XOR set. `{}`: parts.
    sample_size_parts: "{} parts",
    /// Size of a sample master seed. `{}`: bytes.
    sample_size_bytes: "{} bytes",
    // ----- wallet (faraday-core/src/wallet.rs) -----
    /// Text in `network_name`.
    wallet_mainnet: "Mainnet",
    /// Text in `network_name`.
    wallet_testnet: "Testnet",
    /// Text in `network_name`.
    wallet_signet: "Signet",
    /// Text in `network_name`.
    wallet_regtest: "Regtest",
    /// Text in `text`. `{}`: `e`.
    wallet_not_a_valid: "Not a valid seed: {}",
    /// Text in `text`. `{}`: `fp`.
    wallet_is_already_loaded: "{} is already loaded",
    /// Text in `text`. `{}`: `got`, `wanted`.
    wallet_these_words_give: "These words give {}, not {}",
    /// Text in `text`. `{}`: `got`, `wallet`.
    wallet_these_words: "These words give {}, not a key of {}",
    /// Text in `text`. `{}`: `e`.
    wallet_not_a_wallet: "Not a wallet: {}",
    /// Text in `text`. `{}`: `network_name(*offered)`, `network_name(*loaded)`.
    wallet_is_not_the_network: "{} is not {}, the network of the loaded wallets",
    /// Text in `text`. `{}`: `fp`.
    wallet_is_not_a_key: "{} is not a sample: Practice takes only samples",
    /// Text in `text`. `{}`: `network_name(*n)`.
    wallet_runs_on_only: "This app runs on {} only",
    /// Text in `source_tag`. `{}`: `source`.
    wallet_source_tag: "From {}",
    /// Text in `source_tag`. `{}`: `source`.
    wallet_from_inline: "from {}",
    /// Text in `add_seed`.
    wallet_a_master_seed: "a master seed is 16 to 64 bytes",
    /// Text in `shape`.
    wallet_legacy: "legacy",
    /// Text in `shape`.
    wallet_nested_segwit: "nested SegWit",
    /// Text in `shape`.
    wallet_native_segwit: "native SegWit",
    /// Text in `shape`.
    wallet_taproot: "Taproot",
    /// Text in `shape`. `{}`: `script(t)`.
    wallet_single_key: "Single key · {}",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_legacy_multisig: "{} of {} · legacy multisig",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_nested_segwit: "{} of {} · nested SegWit multisig",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_native_segwit: "{} of {} · native SegWit multisig",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_taproot_multisig: "{} of {} · Taproot multisig",
    /// Text in `shape`. `{}`: `wallet.policy.keys().len()`.
    wallet_keys_miniscript: "{} keys · miniscript with conditions",
    /// Text in `shape`. `{}`: `wallet.policy.keys().len()`.
    wallet_keys_taproot: "{} keys · Taproot key and leaves",
    /// Text in `shape`. `{}`: `n`, `n`.
    wallet_of_musig2: "{} of {} · MuSig2",
    /// Text in `shape`. `{}`: `m`, `n`.
    #[cfg(feature = "unreleased")]
    wallet_of_frost: "{} of {} · FROST",
    /// Why a FROST threshold record is not read: FROST is compiled out of
    /// the app (F109).
    wallet_frost_not_read: "FROST threshold wallets are not in this version of Faraday",
    /// Text in `shape`.
    wallet_1_key_silent: "1 key · silent payments",
    /// Text in `shape`.
    wallet_wallet: "Wallet",
    /// Text in `address`. `{}`: `e`.
    wallet_no_address: "no address: {}",
    /// Error in `sign_with`.
    wallet_no_signing_key: "No key in this session signs this transaction",
    /// Text in `read_wallet`.
    wallet_a_json_file: "a .json file with no descriptor",
    /// Text in `read_wallet`. `{}`: `e`.
    wallet_bsms: "BSMS: {}",
    /// Text in `coldcard_export`. `{}`: `why`.
    wallet_coldcard_export: "Coldcard export: {}",
    /// Text in `coldcard_export`.
    wallet_no_xfp: "no xfp",
    /// Error in `coldcard_export`. `{}`: `field`, `first`.
    wallet_its_first_address: "its first {} address is not {}",
    /// Error in `coldcard_export`.
    wallet_no_single_sig: "no single-sig account",
    /// Error in `wallet::mismatch_text`. `{}`: the key, shortened; the
    /// script type its version names; the script type around it.
    wallet_slip132_mismatch: "{} is a {} key; this wallet is {}",
    /// Text in `wallet::script_name`.
    wallet_slip132_native_multisig: "native SegWit multisig",
    /// Text in `wallet::script_name`.
    wallet_slip132_nested_multisig: "nested SegWit multisig",
    /// Text in `wallet::script_name`.
    wallet_slip132_legacy_multisig: "legacy multisig",
    /// Text in `wallet::script_name`.
    wallet_slip132_silent_payments: "silent payments",
    /// Text in `wallet::script_name`: a descriptor of no type a
    /// SLIP-132 version names.
    wallet_slip132_other: "another script type",
    /// Error in `single_key::read`. `{}`: why.
    wallet_single_key_file: "Single-key file: {}",
    /// Text in `single_key::read`.
    wallet_single_key_fingerprint: "MasterFingerprint is not 8 hex digits",
    /// Text in `single_key::read`.
    wallet_single_key_xpub: "ExtPubKey is not an extended public key",
    /// Text in `single_key::read`.
    wallet_single_key_path: "AccountKeyPath is not a derivation path",
    /// Text in `read_wallet`, of a single-key file.
    wallet_single_key_no_type: "its key is not a single-key wallet's",
    /// Error in `caravan::descriptor`. `{}`: why.
    wallet_caravan: "Caravan config: {}",
    /// Text in `caravan::descriptor`.
    wallet_caravan_address_type: "addressType is not P2WSH, P2SH-P2WSH or P2SH",
    /// Text in `caravan::descriptor`.
    wallet_caravan_network: "network is not mainnet, testnet, signet or regtest",
    /// Text in `caravan::descriptor`.
    wallet_caravan_starting_index: "startingAddressIndex is not 0",
    /// Text in `caravan::descriptor`.
    wallet_caravan_keys: "a key in extendedPublicKeys is not an extended public key",
    /// Text in `caravan::descriptor`.
    wallet_caravan_quorum: "the quorum does not match the keys listed",
    /// Text in `caravan::descriptor`.
    wallet_caravan_xfp: "an xfp is not 8 hex digits",
    /// Text in `caravan::descriptor`. `{}`: the key, shortened.
    wallet_caravan_no_origin: "{} has no bip32Path or xfp; every key of a multisig wallet needs both",
    /// Text in `caravan::descriptor`. `{}`: the key, shortened; the
    /// network it is on; the network the file names.
    wallet_caravan_key_network: "{} is a {} key; the file says {}",
    /// Error in `electrum::descriptor`. `{}`: why.
    wallet_electrum: "Electrum file: {}",
    /// Error in `read_wallet`: an Electrum file encrypted, whole or its
    /// keystores.
    wallet_electrum_encrypted: "This Electrum file is encrypted",
    /// Text in `electrum::descriptor`.
    wallet_electrum_fingerprint: "a root_fingerprint or ckcc_xfp is not a fingerprint",
    /// Text in `electrum::descriptor`. `{}`: the keystore's `type`.
    wallet_electrum_keystore_type: "keystore type {} is not read",
    /// Text in `electrum::descriptor`.
    wallet_electrum_no_key: "a keystore has no xpub",
    /// Text in `electrum::descriptor`.
    wallet_electrum_derivation: "a derivation is not a derivation path",
    /// Text in `electrum::descriptor`. `{}`: the script type a key's
    /// version names.
    wallet_electrum_key_type: "a key is a {} key, which this wallet_type does not hold",
    /// Text in `electrum::descriptor`. `{}`: the file's `wallet_type`.
    wallet_electrum_wallet_type: "wallet_type {} is not read",
    /// Text in `electrum::descriptor`.
    wallet_electrum_quorum: "wallet_type does not match the keystores",
    /// Text in `electrum::descriptor`. `{}`: the key, shortened.
    wallet_electrum_no_origin: "{} has no root_fingerprint and derivation; every key of a multisig wallet needs both",
    /// Text in `electrum::descriptor`.
    wallet_electrum_mixed: "its keys name more than one script type",
    /// Error in `read_wallet`. `{}`: `json::MAX_DEPTH`.
    wallet_json_too_deep: "JSON nested more than {} levels deep",
    /// Error in `read_wallet`.
    wallet_json_too_large: "JSON longer than 1 MiB",
    /// Text in `collect_line`. `{}`: `missing`, `if collected == 0 { "" } else { " mor…`, `if missing == 1 { EN.wallet_signature…`.
    wallet_collect: "Collect {}{} {}",
    /// Text in `collect_line`, after a space.
    wallet_more: "more",
    /// Text in `collect_line`.
    wallet_signature: "signature",
    /// Text in `collect_line`.
    wallet_signatures: "signatures",
    /// Error in `collect`.
    wallet_not_a_psbt: "Not a PSBT",
    /// Error in `collect`. `{}`: `source`.
    wallet_psbt_other_transaction: "{} is a PSBT for another transaction",
    /// Text in `collect`. `{}`: `e`.
    wallet_cannot_combine: "Cannot combine: {}",
    /// Error in `collect`. `{}`: `source`.
    wallet_a_signature: "A signature in {} does not verify",
    /// Error in `finish`.
    wallet_not_enough_signatures: "Not enough signatures yet",
    /// Text in `finish_reason`.
    wallet_script_unsatisfied: "The script is not satisfied yet: another signature, or a timelock not yet reached",
    /// Error in `sign_message`.
    wallet_no_such_wallet: "no such wallet",
    /// Error in `sign_message`.
    wallet_messages_are: "Messages are signed with single-key wallets",
    /// Error in `sign_message`.
    wallet_the_wallet_has: "the wallet has no key",
    /// Error in `sign_message`.
    wallet_the_wallet_s_xpub: "the wallet's xpub has no origin",
    /// Error in `sign_message`.
    wallet_the_wallet_s_key: "the wallet's key is not loaded here",
    /// Error in `sign_message`.
    wallet_the_wallet_s: "the wallet's xpub has no path",
    /// Text in `paths`. `{}`: `n`, `n / 144`.
    wallet_blocks_after: "{} blocks after the coins arrived (about {} days)",
    /// Text in `paths`. `{}`: `u64::from(*n) * 512 / 86_400`.
    wallet_days_after_arrival: "{} days after the coins arrived",
    /// Text in `paths`. `{}`: `h`.
    wallet_after_block: "after block {}",
    /// Text in `paths`. `{}`: `t`.
    wallet_after_time: "after time {}",
    /// Text in `paths`.
    wallet_a_secret_preimage: "a secret preimage",
    // ----- wordlist (faraday-core/src/wordlist/mod.rs) -----
    /// Text in `PRESS_A_WORD`.
    wordlist_press_a_word: "Press a word to see it in the list",
    /// Text in `name`.
    wordlist_bip_39_english: "BIP-39 English",
    /// Text in `name`.
    wordlist_eff_large: "EFF large",
    /// Text in `name`.
    wordlist_eff_short_1: "EFF short 1",
    /// Text in `name`.
    wordlist_eff_short_2: "EFF short 2",
    /// Text in `code_heading`.
    wordlist_bits: "Bits",
    /// Text in `code_heading`.
    wordlist_dice: "Dice",
    // ----- wordlist (faraday-core/src/wordlist/screen.rs) -----
    /// Button in `draw`.
    wordlist_back: "Back",
    /// Text in `draw`. `{}`: `words.len()`, `m + 1`, `words.len()`.
    wordlist_words_word: "{} words · word {} of {} marked",
    /// Text in `draw`. `{}`: `words.len()`.
    wordlist_words: "{} words",
    /// Text in `draw`.
    wordlist_type_to_find: "Type to find a word",
    /// Text in `draw`. `{}`: `st.find`.
    wordlist_find: "Find: {}",
    /// Line in `draw`.
    wordlist_number_column: "No.",
    /// Line in `draw`.
    wordlist_word: "Word",
    // ----- The shells: window titles and the labels they give (faraday/shells) -----
    /// The desktop app's window title.
    desktop_title: "Faraday",
    /// The desktop app's window title at a panel size: `{}` the diagonal in
    /// inches.
    desktop_title_panel: "Faraday · {} in panel",
    /// The pill of the boot stick, Stick 1, across the top of the window.
    stick_pill_boot: "Boot stick",
    /// A pill's state: the stick is in.
    stick_pill_inserted: "in",
    /// A pill's state: the stick is out.
    stick_pill_out: "out",
    /// A pill's label and its state: `{}` the label, then the state.
    stick_pill_state: "{} · {}",
    /// Faraday Demo's top bar: the pill the sticks that do not fit
    /// fold into, which opens the Sticks menu. `{}` how many it holds.
    stick_pill_more: "Sticks · {}",
    /// The Starter's window title.
    starter_window_title: "Faraday Starter",
    /// A Windows disk as Disk Management names it: `{}` its number.
    starter_disk_number: "Disk {}",
    /// The label of the firmware boot entry the Starter makes on Linux; it
    /// also finds the entry again.
    starter_boot_entry_label: "Faraday stick",
    /// A USB stick mounted with no volume label: `{}` its device name.
    stick_usb_stick: "USB stick ({})",
    // ----- Virtual machines by name, as the stick's device check and the Starter show them -----
    /// A hypervisor's name.
    vm_qemu: "QEMU",
    /// A hypervisor's name.
    vm_kvm: "KVM",
    /// A hypervisor's name.
    vm_vmware: "VMware",
    /// A hypervisor's name.
    vm_virtualbox: "VirtualBox",
    /// A hypervisor's name.
    vm_hyper_v: "Hyper-V",
    /// A hypervisor's name.
    vm_xen: "Xen",
    /// A hypervisor's name.
    vm_parallels: "Parallels",
    /// A hypervisor's name.
    vm_bochs: "Bochs",
    /// The backup flow's name when no key is named.
    backup_flow_heading: "Backup",
    /// A GPG key, as the flow names what it makes.
    backup_flow_kind_gpg: "GPG key",
    /// Secure Boot keys, as the flow names what it makes.
    backup_flow_kind_sb: "Secure Boot keys",
    /// The way out of the flow, which keeps its state.
    backup_flow_leave: "Home",
    /// A vault of the plan, by number.
    backup_flow_vault_n: "Vault {}",
    /// A place of the plan, by number.
    backup_flow_place_n: "Place {}",
    /// Step 1's card.
    backup_flow_card_where: "Where it will be kept",
    /// Step 2's card; the plural of the medium.
    backup_flow_card_sticks: "Bring in the {}",
    /// Guided: Bring in the sticks in the setup of a wallet's vaults
    /// (`docs/BACKUP-FLOW.md` §10). `{}`: the medium, its plural, the
    /// medium.
    backup_flow_guide_sticks: "Each vault's {} is read here, before any key exists: whether it can be written, how much room it has, and which vaults it holds already, so this vault is one of them or a new one beside them. Nothing is written to it now: the vaults go onto their {} after the keys are saved into them and memory is locked. With a key in memory, a {} is read only after that lock, which wipes the key.",
    /// Step 3's card.
    backup_flow_card_vaults: "Vaults",
    /// Step 4's card.
    backup_flow_card_make: "Make the key",
    /// Step 5's card.
    backup_flow_card_lock: "Lock",
    /// Step 6's card; the plural of the medium.
    backup_flow_card_write: "Write the {}",
    /// Step 7's card.
    backup_flow_card_done: "Done",
    /// Button on Where: another place.
    backup_flow_add_place: "Add a place",
    /// A place's line: what it keeps, the medium first.
    backup_flow_place_keeps: "A {} with",
    /// A new vault, as a place's choice and a vault's state.
    backup_flow_new_vault: "A new vault",
    /// An existing vault, as a vault's state.
    backup_flow_existing_vault: "An existing vault",
    /// The stick for a vault at a place: the medium, vaults, place.
    backup_flow_vault_stick: "the {} for {} ({})",
    /// The stick of files.
    backup_flow_files_stick: "the {} of files",
    /// The stick of public files.
    backup_flow_public_stick: "the {} of public files",
    /// A place's stick of files (§17.2): the place.
    backup_flow_place_files: "Files ({})",
    /// A place's card's sheet: the row, the place.
    backup_flow_pick_title: "{} · {}",
    /// The plan's last card, the map approved.
    backup_flow_the_plan: "The plan",
    /// The plan held by a vault that holds nothing: the vault.
    backup_flow_vault_holds_nothing: "Vault {} holds nothing",
    /// A wallet share kept in a vault: the wallet, the share, of.
    backup_flow_vault_share: "{} · Share {} of {}",
    /// An edit after the lock refused: the vault held this before the
    /// flow. The vault.
    backup_flow_held_before: "Vault {} held this before; take it out from the vault's contents",
    /// An edit after the lock refused: another wallet in the vault uses
    /// this key. The vault.
    backup_flow_other_wallet_key: "Another wallet in Vault {} uses this key; take it out from the vault's contents",
    /// An edit after the lock refused: it changes files or paper.
    backup_flow_files_after_lock: "Files and paper stay as made after the lock; finish the writes, then Back up again to change them",
    /// The checklist or Lock and write of a backup past its lock and not
    /// done: refused.
    backup_flow_past_lock: "This backup is past its lock; finish the writes, then Back up again",
    /// Foot of step 2 when a secret is open.
    backup_flow_lock_bring: "Lock and bring in the {}",
    /// Asks for a stick.
    backup_flow_insert: "Insert {}",
    /// A stick in the machine already, with when it was read.
    backup_flow_use_read: "Use {}, read at {}",
    /// Two sticks are in.
    backup_flow_one_at_a_time: "One {} at a time",
    /// A stick that is not FAT32.
    backup_flow_not_fat: "Not a FAT32 {}. Format it as FAT32, or use another {}",
    /// An FARADAYBOOT partition.
    backup_flow_boot_part: "This is a boot partition. Use another {}",
    /// A stick that is read-only.
    backup_flow_read_only: "Cannot be written. Use another {}",
    /// A stick with too little room.
    backup_flow_no_room: "Not enough room: {} needed, {} free",
    /// The plan gives an existing vault and the stick holds none.
    backup_flow_no_vault: "No vault for {} on this stick",
    /// Choice of the vault on a stick.
    backup_flow_which_vault: "Which vault is {}?",
    /// A new vault written beside the vaults a stick holds.
    backup_flow_beside: "A new vault beside them",
    /// Two files of one vault on one stick.
    backup_flow_two_copies: "{} has two copies of {}: {} and {}",
    /// The same vault, other bytes.
    backup_flow_copies_differ: "{}'s copy of {} differs from {}'s",
    /// Which copy of a vault to unlock.
    backup_flow_use_copy: "Use {}'s copy",
    /// Tap that confirms a stick's place.
    backup_flow_confirm_place: "This is {}",
    /// Hold that confirms a stick's place.
    backup_flow_hold_confirm_place: "Hold: this is {}",
    /// A stick that reads as one bound to another place.
    backup_flow_bound_place: "This is {}",
    /// Tap for a stick that keeps two vaults.
    backup_flow_both: "One {} for both",
    /// Asks for the stick to be pulled.
    backup_flow_pull: "Pull the {}",
    /// Under an existing vault's Unlock.
    backup_flow_no_passphrase: "I do not have its passphrase",
    /// After no passphrase: a new vault beside that file.
    backup_flow_new_vault_beside: "A new vault beside it",
    /// Leaves the stick for another.
    backup_flow_use_another: "Use another {}",
    /// On a device, a row naming one of the flow's sticks: its pill's
    /// name, then what it is for ("Stick 2 · Vault 1 (Place 1)").
    backup_flow_slot_row: "{} · {}",
    /// On a device, asks for a stick by its pill's name ("Insert Stick 3:
    /// Vault 2 (Place 2)").
    backup_flow_insert_n: "Insert {}: {}",
    /// On a device, a stick unlike the one read for this pill at step 2
    /// ("Not Stick 3, read for Vault 2 (Place 2)").
    backup_flow_not_read_n: "Not {}, read for {}",
    /// On a device, above the hold: a stick not yet settled that reads as
    /// another of the flow's sticks ("This reads as Stick 2").
    backup_flow_reads_as: "This reads as {} {}",
    /// On a device, what the stick of files is for, after its number.
    backup_flow_for_files: "Files",
    /// On a device, what the stick of public files is for, after its
    /// number.
    backup_flow_for_public: "Public files",
    /// A backup stick's pill: the medium's word and its number, from 2
    /// ("Stick 2", "Card 2"; `docs/BACKUP-FLOW.md` §16).
    backup_flow_pill: "{} {}",
    /// The boot pill on the Pi, Card 1.
    backup_flow_pill_boot_card: "Boot card",
    /// A device's pill word on a USB stick.
    backup_flow_pill_stick: "Stick",
    /// A device's pill word on an SD card.
    backup_flow_pill_card: "Card",
    /// Under a stick's row once it is first taken for its place: the
    /// pill's word and number.
    backup_flow_mark_as: "Mark it as {} {}",
    /// A stick refused that reads as another of the flow's sticks: the
    /// pill's word and number.
    backup_flow_this_is_n: "This is {} {}",
    /// A stick refused that reads as none of the flow's sticks.
    backup_flow_not_ours: "Not one of this backup's {}",
    /// A row of the result: the vault the key is in.
    backup_flow_in_vault: "In {}",
    /// Foot of step 5.
    backup_flow_lock_and_write: "Lock and write",
    /// A stick unlike the one read at step 2.
    backup_flow_not_read: "Not the {} read for {}",
    /// A second copy refused on the stick holding the first.
    backup_flow_has_first: "This {} has the first copy",
    /// Hold for a vault's second copy.
    backup_flow_hold_second: "Hold: this is a second {}",
    /// Hold for a vault's third and later copy.
    backup_flow_hold_another: "Hold: this is another {}",
    /// A stick that holds files written for another.
    backup_flow_other_files: "This {} has {}'s files",
    /// A write under way.
    backup_flow_writing: "Writing {}",
    /// A stick written, read back and matched.
    backup_flow_written: "Written and checked on {}",
    /// A stick pulled while writing.
    backup_flow_pulled: "Pulled before the copy was checked. Nothing counted",
    /// A write that did not read back.
    backup_flow_not_checked: "The copy did not check on {}. Nothing counted",
    /// A file written to a stick did not read back, twice
    /// (`docs/BACKUP-FLOW.md` §17.12 item 10): the file, the stick.
    backup_flow_no_read_back: "{} did not read back from {}; try another stick",
    /// A stick's written copy was deleted or the stick wiped.
    backup_flow_first_gone_toast: "A written copy was deleted",
    /// Done's result.
    backup_flow_done: "Written",
    /// The key page's row.
    backup_flow_row: "Backup",
    /// The Backup row before any write.
    backup_flow_row_none: "Not written",
    /// The Backup row part way.
    backup_flow_row_some: "Written to {} of {} places",
    /// The Backup row when done.
    backup_flow_row_all: "Written to every place",
    /// Opens the flow on its step.
    backup_flow_continue: "Continue the backup",
    /// Goes on from a card.
    backup_flow_continue_make: "Continue",
    /// A waiting action, until every place is written.
    backup_flow_first: "Back up first",
    /// Home's lead.
    backup_flow_home_lead: "Continue the backup of {}",
    /// The Power sheet's line.
    backup_flow_power: "Backup of {}: written to {} of {} places",
    /// Tag of a file the flow will write, on the stick visit.
    backup_flow_in_the_backup: "In the backup",
    /// The fewest places a key is planned at.
    backup_flow_at_least: "At least {} places",
    /// Under every card.
    backup_flow_change_plan: "Change the plan",
    /// Step 4 of the round for a vault added after the key.
    backup_flow_copy_across: "The new vault takes the key from one that holds it",
    /// Button of that round.
    backup_flow_copy_key: "Copy the key",
    /// Makes a vault of the plan.
    backup_flow_create_vault: "Create {}",
    /// Opens a vault of the plan found on its stick: its passphrase next.
    backup_flow_open_vault: "Open {}",
    /// Takes a written place off the plan.
    backup_flow_destroyed: "I destroyed this copy",
    /// Memory this computer has free.
    backup_flow_free: "{} free",
    /// Lock card: what is kept.
    backup_flow_kept: "Kept",
    /// Lock card: what is wiped.
    backup_flow_wiped: "Wiped from memory",
    /// Summary of a done Lock card.
    backup_flow_locked: "Locked",
    /// Summary of the Lock card before the lock.
    backup_flow_not_locked: "Not locked",
    /// Result of making the key.
    backup_flow_made: "{} made",
    /// Button that makes the key.
    backup_flow_make_the_key: "Make the key",
    /// While the key is made.
    backup_flow_making: "Making the key",
    /// Result row.
    backup_flow_name: "Name",
    /// Summary of Make before the key.
    backup_flow_not_made: "Not made",
    /// Summary of Where.
    backup_flow_places_vaults: "Places {} · Vaults {}",
    /// Banner while a stick is in at step 3 or 4.
    backup_flow_pull_to_unlock: "Pull the {} to unlock",
    /// While a stick is read.
    backup_flow_reading: "Reading {}",
    /// Summary of step 2.
    backup_flow_read_count: "{} of {} read",
    /// Takes a place off the plan.
    backup_flow_remove: "Remove",
    /// A vault on a stick, seen open this session.
    backup_flow_seen: "{} · seen {}",
    /// A vault of the plan, open.
    backup_flow_unlocked: "Unlocked",
    /// Summary of step 3.
    backup_flow_unlocked_count: "{} of {} unlocked",
    /// Toast: the key is made into open vaults.
    backup_flow_unlock_first: "Unlock every vault first",
    /// Toast: the vault set up for a plan vault holds a key only another is to hold.
    backup_flow_set_up_holds_other: "{} was set up for this vault and holds a key the plan puts in another. Change the plan",
    /// Toast: the setup's question or count while one of its vaults is open.
    create_vaults_locked: "A vault of the setup is open. Lock to change this answer",
    /// While a vault unlocks.
    backup_flow_unlocking: "Unlocking",
    /// Unlocks a vault of the plan.
    backup_flow_unlock_vault: "Unlock {}",
    /// Summary of step 6.
    backup_flow_written_count: "{} of {} written",
    /// Summary of a done step 3: how many vaults.
    backup_flow_vaults_n: "{} vaults",
    /// A vault of the plan, new: its name first.
    backup_flow_vault_new: "{} · new",
    /// A vault of the plan that exists already: its name first.
    backup_flow_vault_existing: "{} · an existing vault",
    /// Toast: a flow in the kept state was corrupt or of another version.
    backup_flow_dropped: "A kept backup did not read and was dropped",
    /// A stick whose room cannot be asked for: it cannot be read.
    backup_flow_unreadable: "This {} cannot be read. Use another {}",
    /// Toast: a lock was refused because a waiting backup could not be kept.
    toast_lock_not_kept: "A backup waits and could not be kept. Sealed, not restarted",
    /// Button on Where: the place also keeps this vault's stick.
    backup_flow_also: "Also {}",
    /// Button on Where: this place's stick carries the key's public files.
    backup_flow_public_here: "Public files here",
    /// Toast: a key is not made with a stick in.
    backup_flow_pull_to_unlock_toast: "Pull the stick first",
    /// Fact line under the plan's checks: every secret of the wallet is at one place. The place's name.
    plan_all_at_one_place: "Every secret of this wallet is at {}. Separate places are strongly recommended.",
    /// Fact line under the plan's checks, for a single key with no passphrase.
    plan_passphrase_apart: "A passphrase kept at another place than the seed is recommended.",
    /// Button beside that line.
    plan_add_passphrase: "Add a passphrase",
    /// Templates item of the checklist: print them from Faraday Starter. The count, the words, the copy form.
    item_templates_starter: "Print {} from Faraday Starter: Blank templates, {} words, {}",
    /// Templates item: the copy is the words alone.
    item_templates_words: "Words",
    /// Templates item: the copy is the SeedQR alone.
    item_templates_seedqr: "SeedQR",
    /// Templates item: the copy is the words and the SeedQR.
    item_templates_both: "Both",
    /// Foot of the Templates item, a press.
    item_action_done: "Done",
    /// Checklist item: write to the sticks, by place. The medium, plural.
    backup_flow_item_sticks: "Write to {}",
    /// Heading of a Bitcoin wallet's flow.
    backup_flow_kind_wallet: "Bitcoin wallet",
    /// A key of a wallet to be made, before it has a fingerprint.
    backup_flow_key_n: "Key {}",
    /// Toast: a key kept only on paper is not here to be copied into a vault.
    backup_flow_key_on_paper: "Its key is kept only on paper. Type its words in first",
    /// Checklist item whose files were written: where.
    backup_flow_written_place: "Written to {}",
    /// Make card of a wallet's flow.
    backup_flow_card_make_wallet: "Make the keys",
    /// Make a new wallet's and Restore's card before the keys: whether
    /// vaults are part of the backup.
    backup_flow_card_use_vaults: "Use vaults",
    /// The question on that card.
    backup_flow_use_vaults_question: "Will you use vaults as part of this backup?",
    /// The muted line under that question (`docs/BACKUP-FLOW.md` §17.21
    /// item 5).
    backup_flow_vaults_on_sticks: "Vaults already on sticks are brought in here, not later.",
    /// The card that asks how many vaults.
    backup_flow_card_vault_count: "Number of vaults",
    /// The line over the count of vaults.
    backup_flow_how_many_question: "How many vaults will be part of this backup?",
    /// The count's label.
    backup_flow_vault_count_label: "Vaults",
    /// A stick named by its vault before the keys exist: "the stick for
    /// Vault 1".
    backup_flow_stick_for_vault: "the {} for {}",
    /// The flow's first card for a wallet whose keys exist: the plan.
    backup_flow_card_plan: "The plan",
    /// Opens the flow on the card to change what it decided.
    backup_flow_change: "Change",
    /// Button of a wallet's Make card.
    backup_flow_make_the_keys: "Make the keys",
    /// Make card of a round for an added vault: the seeds kept only on paper.
    backup_flow_on_paper_only: "{} is kept only on paper",
    /// Button: type the seed back from its copy.
    backup_flow_add_from_copy: "Add the key from its copy",
    /// Toast: a change would take a written stick off the plan.
    backup_flow_written_stays: "A stick already written stays on the plan",
    /// Toast: the round has no copy of the wallet description to put in.
    backup_flow_no_description: "No copy of the wallet description is here to put in",
    /// Toast: a change would leave a key held back kept on no stick.
    backup_flow_held_stays: "A key not yet written stays kept on a stick",
    /// The backup board's card (`docs/BACKUP-FLOW.md` §17): its title.
    board_title: "Places",
    /// The board's card closed: how many places. `{}`: the count.
    board_places: "{} places",
    /// A list chip's count, or the card closed with one place.
    board_one_place: "1 place",
    /// A place's box that makes a stick of what is dropped on it.
    board_add_stick: "+ stick",
    /// A stick zone's label. `{}`: its number at the place, from 1.
    board_stick: "Stick {}",
    /// A stick zone's label where the medium is an SD card (the Pi).
    /// `{}`: its number at the place, from 1.
    board_sd_card: "SD card {}",
    /// The files box's header inside a stick box: the files kept on the
    /// stick outside any vault (`docs/BACKUP-FLOW.md` §17.17).
    board_files_not_sealed: "Files · not sealed",
    /// A place card's paper zone.
    board_paper: "Paper",
    /// The software place's zone: the description shown for it to scan.
    board_scanned_zone: "Scanned in",
    /// The list's entry and the software place's title when no software
    /// is chosen.
    board_watch_only: "Watch-only software",
    /// The list's group of seeds.
    board_seeds: "Seeds",
    /// The list's group of passphrases.
    board_passphrases: "Passphrases",
    /// The list's group of the description and its shares.
    board_wallet_description: "Wallet description",
    /// The whole description's chip.
    board_whole: "Whole",
    /// A share's chip. `{}`: its number.
    board_share: "Share {}",
    /// The list's group of vaults.
    board_vaults: "Vaults",
    /// The list's button that adds a vault, always a new one, made in the
    /// checklist (`docs/BACKUP-FLOW.md` §17.21 item 4).
    board_add_vault: "+ new vault",
    /// A cosigner's seed in the list: its key is not here.
    board_not_here: "Not here",
    /// A form: a seed's words on paper.
    board_words: "Words",
    /// A form: a seed's SeedQR on paper.
    board_seedqr: "SeedQR",
    /// A form: a seed's words as a text file.
    board_words_file: "Words file",
    /// A form: a seed's SeedQR as a picture.
    board_seedqr_file: "SeedQR file",
    /// A form: a passphrase as a text file.
    board_passphrase_file: "Passphrase file",
    /// A form: the description or a share as text.
    board_text_file: "Text file",
    /// A form: the description or a share as a QR picture.
    board_qr_picture: "QR picture",
    /// A form: the software's own file. `{}`: the software.
    board_software_file: "{} file",
    /// A check: a place keeps a passphrase beside its seed's words. The
    /// place, the seed.
    board_pass_with_words: "{} keeps {}'s words with its passphrase",
    /// A check: one place alone spends the wallet. The place.
    board_alone_spends: "{} alone can spend the wallet",
    /// Under a vault's chip, before it is taken off: what it holds. The
    /// vault, what it holds.
    board_vault_holds: "{} holds {}",
    /// Takes a vault that holds something off the plan.
    board_take_off: "Take off",
    /// The narrow column's button that opens the board's list as a sheet
    /// (`docs/BACKUP-FLOW.md` §17.12 item 3).
    board_add: "Add",
    /// The board's row after the last place that adds a place
    /// (`docs/BACKUP-FLOW.md` §17.15).
    board_add_place: "+ place",
    /// A vault box's header band. `{}`: the vault, "Vault 1".
    board_sealed: "{} · sealed",
    /// A vault box's header band while the vault is not made yet: it is
    /// made in the checklist (`docs/BACKUP-FLOW.md` §17.21 item 6). `{}`:
    /// the vault.
    board_sealed_made: "{} · sealed · made in the checklist",
    /// The same, where that does not fit the band. `{}`: the vault.
    board_sealed_new: "{} · new",
    /// The ⋮ menu of a chip in the list, and its title: where it can be
    /// placed (`docs/BACKUP-FLOW.md` §17.20).
    board_menu_add: "Add to…",
    /// A placed chip's ⋮ menu: the item moved to another box.
    board_menu_move: "Move to…",
    /// A placed chip's ⋮ menu: the item also kept in another box.
    board_menu_copy: "Copy to…",
    /// A box in the ⋮ menu: a stick's files. `{}`: the stick, "Stick 1".
    board_menu_files: "{} · files",
    /// A box in the ⋮ menu: a vault on a stick. `{}`: the stick, the vault.
    board_menu_vault: "{} · {}",
    /// A box in the ⋮ menu: a new stick at the place, as **+ stick** makes.
    board_menu_new_stick: "New stick",
    /// The ⋮ menu's way out on the small panel, where it is a sheet.
    board_menu_cancel: "Cancel",
    /// The board's right-hand column, and the narrow column's button in
    /// place of Add, while the board is read-only: opens it for changes
    /// (`docs/BACKUP-FLOW.md` §17.6).
    board_customize: "Customize backup",
    /// The read-only board's button that adds the software place, with the
    /// description Scanned in (`docs/BACKUP-FLOW.md` §17.21 item 3).
    board_add_watch_only: "Add watch-only software",
    /// The same button while the software place is on the board: takes it
    /// off.
    board_take_off_watch: "Take off watch-only software",
    /// A place's spend tag (`docs/BACKUP-FLOW.md` §17.22): its seeds alone sign.
    board_spend_alone: "Can spend alone",
    /// Its seeds sign alone, with the vault's passphrase.
    board_spend_alone_vault: "Can spend alone with the vault's passphrase",
    /// One more place's seeds and this place's sign.
    board_spend_more_one: "Spends with 1 more place",
    /// `{}`: how many more places' seeds sign with this place's.
    board_spend_more: "Spends with {} more places",
    /// As `board_spend_more_one`, with the vault's passphrase.
    board_spend_vault_more_one: "Spends with the vault's passphrase and 1 more place",
    /// As `board_spend_more`, with the vault's passphrase. `{}`: places.
    board_spend_vault_more: "Spends with the vault's passphrase and {} more places",
    /// The sheet over a drop that brings a place nearer a spend (§19.22). `{}`: the place.
    board_spend_title_alone: "Anyone with access to {} can spend",
    /// As `board_spend_title_alone`, with the vault's passphrase. `{}`: the place.
    board_spend_title_alone_vault: "Anyone with access to {} and the vault's passphrase can spend",
    /// `{}`: the place; one more place spends.
    board_spend_title_one: "Anyone with access to {} needs only 1 more place to spend",
    /// `{}`: the place, then how many more places.
    board_spend_title_more: "Anyone with access to {} needs only {} more places to spend",
    /// As `board_spend_title_one`, with the vault's passphrase.
    board_spend_title_vault_one: "Anyone with access to {} and the vault's passphrase needs only 1 more place to spend",
    /// As `board_spend_title_more`, with the vault's passphrase.
    board_spend_title_vault_more: "Anyone with access to {} and the vault's passphrase needs only {} more places to spend",
    /// The sheet's line. `{}`: the place, its seeds, the seeds a spend needs.
    board_spend_line: "{} keeps {} of the {} seeds a spend needs.",
    /// As `board_spend_line`, counting the seeds in its vaults.
    board_spend_line_vault: "{} keeps {} of the {} seeds a spend needs, counting its vaults.",
    /// The sheet over a drop that lets a place see the balance
    /// (`docs/BACKUP-FLOW.md` §17.5). `{}`: the place.
    board_sees_title: "{} can see the wallet's balance",
    /// Its line: the place keeps the whole description.
    board_sees_whole: "The whole description is kept there.",
    /// Its line: what at the place together makes the description. `{}`:
    /// "Share 1 and seed 9a6a2580".
    board_sees_together: "{} together make the whole description.",
    /// Its line: one share alone makes the description. `{}`: "Share 1".
    board_sees_alone: "{} makes the whole description.",
    /// One share in its line. `{}`: its number.
    board_sees_share: "Share {}",
    /// Shares in its line. `{}`: "1, 2", the last.
    board_sees_shares: "Shares {} and {}",
    /// One seed in its line. `{}`: its fingerprint.
    board_sees_seed: "seed {}",
    /// Seeds in its line. `{}`: the fingerprints but the last, the last.
    board_sees_seeds: "seeds {} and {}",
    /// The shares and the seeds in its line. `{}`: the shares, the seeds.
    board_sees_both: "{} and {}",
    /// Keeps the drop.
    board_keep_button: "Keep it",
    /// Undoes the drop.
    board_take_it_off: "Take it off",
    /// Under the list's Wallet description: hands a share to every place
    /// (`docs/BACKUP-FLOW.md` §17.19 item 1).
    board_partials_add: "Add partial descriptions to this backup",
    /// The same button while they are on: takes every share off.
    board_partials_remove: "Remove partial descriptions",
    /// The sheet after adding them: its title.
    board_partials_each: "Each place now holds a part of the description.",
    /// Its line where the whole description is still kept. `{}`: the
    /// places, "Place 1, Place 2 and Place 3".
    board_partials_whole: "The whole description is still kept at {}, so the balance can be seen there. Take it off any place where the balance should stay hidden.",
    /// Takes the whole description off every place.
    board_take_it_off_every: "Take it off every place",
    /// A warning: nothing of the description is kept.
    board_desc_nothing: "The description cannot be put back together: add the whole description",
    /// A warning: what is kept does not put it back together. `{}`: what
    /// is kept ("Share 1 twice and Share 2"), what would complete it
    /// ("Share 3").
    board_desc_missing: "The description cannot be put back together: the places hold {}; add {} or the whole description",
    /// The same, every share kept somewhere. `{}`: what is kept.
    board_desc_kept_add: "The description cannot be put back together: the places hold {}; add the whole description",
    /// A share kept twice, in those warnings. `{}`: its number.
    board_share_twice: "Share {} twice",
    /// A share kept more often. `{}`: its number, how often.
    board_share_times: "Share {} {} times",
    /// A warning: a place sees the balance from its share and seeds.
    /// `{}`: the place, what there makes the description.
    board_sees_warn: "{} can see the balance. {}",
    /// A warning: a place keeps no seed. `{}`: the place.
    board_no_seed: "{} holds no seed",
    /// The backup's last card, in Guided. `{}`: `medium.noun()`.
    backup_extra_copies: "More copies can be made once the wallet is made: a vault copied to another {}, words copied by hand. They are not part of the plan.",
    /// The Learn link under it.
    backup_extra_copies_link: "Extra copies",
    /// "Any one place lost" with a single place (§19.19).
    plan_no_single_place: "No: a single place",
    /// Why "Any one place lost" reads No when the description goes. `{}`:
    /// the place.
    plan_without_no_description: "Without {}, the rest cannot put the description back together",
    // ----- A ⋮ on every file; Rename; Details (docs/FLOWS.md decision 61), faraday-core -----
    /// A file's ⋮ menu: the popup to rename it.
    file_menu_rename: "Rename",
    /// A file's ⋮ menu: the sheet of its name, kind, size and SHA-256.
    file_menu_details: "Details",
    /// A file on the stick visited: its ⋮ menu's way to the Delete files
    /// sheet with it alone chosen. `{}`: "stick", "SD card".
    file_menu_delete_from_medium: "Delete from the {}",
    /// The ⋮ menu's way out on the small panel, where it is a sheet.
    file_menu_cancel: "Cancel",
    /// The Rename popup's title. `{}`: the file's name.
    rename_title: "Rename {}",
    /// The Rename popup's button.
    rename_button: "Rename",
    /// The Rename popup's way out.
    rename_cancel: "Cancel",
    /// Rename refused: nothing typed.
    rename_empty: "Type a name",
    /// Rename refused: the name starts with a dot.
    rename_dot: "A name cannot start with a dot",
    /// Rename refused: a character a stick's names cannot hold.
    rename_character: "A name cannot hold / \\ : * ? \" < > | or a control character",
    /// Rename refused: the name with its extension is over 255 characters.
    rename_long: "A name is at most 255 characters with its extension",
    /// Rename refused: another file For the stick has the name, ignoring
    /// case. `{}`: "for the stick", the name.
    rename_taken_outbox: "Another file {} is named {}",
    /// The visit's line once a file on the stick is renamed. `{}`: the
    /// old name, the new.
    rename_done: "{} renamed {}",
    /// The visit's line: refused, the name is on the stick already. `{}`:
    /// the name, "stick".
    rename_taken_on_medium: "{} is already on this {}",
    /// The visit's line: refused, the file of that name is the same vault.
    /// `{}`: the name, "stick".
    rename_same_vault: "{} on this {} is this same vault",
    /// The visit's line: the stick refused the rename. `{}`: the name, why.
    rename_not_done: "{} not renamed: {}",
    /// The Details sheet's labels.
    details_name: "Name",
    /// The Details sheet's label for the row's kind line.
    details_kind: "Kind",
    /// The Details sheet's label for the size.
    details_size: "Size",
    /// The size in bytes. `{}`: the number.
    details_bytes: "{} bytes",
    /// The Details sheet's label for the hash.
    details_sha: "SHA-256",
    /// The hash while the stick is read for it.
    details_reading: "Reading…",
    /// The hash not read. `{}`: why.
    details_not_read: "Not read: {}",
    /// The Details sheet's one button.
    details_close: "Close",
    /// The Delete files sheet's button when a vault is chosen: a hold
    /// (`docs/FLOWS.md` decision 4). `{}`: the count, "file" or "files".
    stick_wipe_hold_delete: "Hold to delete {} {} with zeros",
    // ----- New key's sources, checks and dice lists, the self-test, BIP-85's applications, Tools' calculators, Explore's path -----
    /// Row that declines a passphrase.
    passphrase_none: "No passphrase",
    /// Button in the "Demo only" card that puts Practice's passphrase in
    /// the empty passphrase fields of the page (DEMO §5.4).
    demo_use_faraday: "Use faraday",
    /// Button that discards the entries.
    create_sanity_again: "Roll again",
    /// Source: a six-sided die.
    create_source_dice: "Dice",
    /// Source: coin flips.
    create_source_coins: "Coin flips",
    /// Source: typed hex.
    create_source_hex: "Hex entropy",
    /// Source: a shuffled deck.
    create_source_cards: "Playing cards",
    /// Source: camera noise.
    create_source_camera: "Camera noise",
    /// Source: several sources mixed.
    create_source_mix: "Mixed sources",
    /// Source: the device's own generator.
    create_source_device: "This device",
    /// Label: the chi-square statistic of the rolls.
    create_chi_square: "Chi-square",
    /// Label: the longest run of one outcome.
    create_longest_run: "Longest run",
    /// The run's value on a dice check: `{}` runs of the face named by
    /// `create_face_1` to `create_face_6`, as `8 sixes`.
    create_run_face: "{} {}",
    /// Plural name of dice face 1, for that value.
    create_face_1: "ones",
    /// Plural name of dice face 2.
    create_face_2: "twos",
    /// Plural name of dice face 3.
    create_face_3: "threes",
    /// Plural name of dice face 4.
    create_face_4: "fours",
    /// Plural name of dice face 5.
    create_face_5: "fives",
    /// Plural name of dice face 6.
    create_face_6: "sixes",
    /// Verdict beside a statistic: `normal for {} rolls`.
    create_stat_normal: "normal for {} rolls",
    /// Verdict beside a statistic that is out of the usual range.
    create_stat_high: "high for {} rolls",
    /// Label: heads.
    create_heads: "Heads",
    /// Label: tails.
    create_tails: "Tails",
    /// That Choice: the rolls hashed as ASCII digits.
    dice_procedure_hashed: "Hashed (Coldcard, SeedSigner)",
    /// That Choice: every 6 written as a 0, then hashed.
    dice_procedure_six_as_zero: "Six as zero, hashed (Keystone)",
    /// The second line of a procedure's row: `{} rolls`.
    dice_procedure_rolls: "{} rolls",
    /// Label: how many cards were drawn.
    create_cards_row: "Cards",
    /// The shutter under it.
    create_camera_shutter: "Take a picture",
    /// Label: how many frames were taken.
    create_frames_row: "Frames",
    /// Label: how many of the 256 luma values the last frame used.
    create_distinct_row: "Luma values",
    /// The coloured line on that step.
    create_device_result: "Not checkable",
    /// Label: what produced the bytes.
    create_device_row: "Source",
    /// The generator on a device with no secure element.
    create_device_os: "operating system",
    /// The caution this source carries on the source list.
    create_trusts_device: "trusts this device",
    /// Label on the device-randomness result: what the bytes rest on.
    create_trust_row: "Trust",
    /// That row's value, in the caution tone.
    create_trust_device: "this device's software",
    /// "Draw again" on the sanity step, for cards.
    create_sanity_again_draw: "Draw again",
    /// "Take again" on the sanity step, for the camera.
    create_sanity_again_take: "Take again",
    /// Application 39': a child key of words.
    bip85_app_words: "Words",
    /// Application 2': an HD-seed WIF.
    bip85_app_wif: "WIF",
    /// Application 32': an extended private key.
    bip85_app_xprv: "Extended private key",
    /// Application 128169': raw bytes.
    bip85_app_hex: "Hex",
    /// Application 707764': a password in base64.
    bip85_app_base64: "Password (base64)",
    /// Application 707785': a password in base85.
    bip85_app_base85: "Password (base85)",
    /// Row: the hash functions.
    tools_hashes: "Hashes",
    /// Row: base58, bech32, hex, binary.
    tools_encodings: "Encodings",
    /// Row: the descriptor checksum.
    tools_descriptor_checksum: "Descriptor checksum",
    /// Row: xpub to SLIP-132 and back.
    tools_convert_key: "Convert key",
    /// Row: the unit converter.
    tools_units: "Units",
    /// Tools row: the miniscript policy compiler.
    tools_miniscript: "Miniscript",
    /// Title of the Choice that picks the list.
    dice_list_title: "Which list?",
    /// Row: the 7776-word list.
    dice_list_large: "Long list",
    /// What that row costs to roll.
    dice_list_large_detail: "7776 words · five dice",
    /// Row: the first 1296-word list.
    dice_list_short1: "Short list",
    /// What that row costs to roll.
    dice_list_short1_detail: "1296 words · four dice",
    /// Row: the second 1296-word list.
    dice_list_short2: "Short list 2",
    /// What that row costs to roll, and what its words are chosen for.
    dice_list_short2_detail: "1296 words · 3 edits apart",
    /// A path with two slashes in a row.
    explore_path_empty: "Two slashes in a row.",
    /// A path that ends with a slash.
    explore_path_unfinished: "Type an index after the slash.",
    /// A level that is not a number.
    explore_path_not_a_number: "A level is a number, with h for hardened.",
    /// An index of two to the thirty-first or more.
    explore_path_too_large: "The largest index is 2147483647.",
    /// More than ten levels.
    explore_path_too_deep: "At most ten levels.",
    /// Button and title: the account's addresses, a page at a time.
    explore_addresses: "Addresses",
    /// Button and title: look for an address among the account's.
    explore_find: "Find an address",
    /// Label: the account the addresses are of.
    explore_account: "Account",
    /// The account: `{}` its path without `m/`, `{}` its script.
    explore_account_path: "m/{} · {}",
    /// Button: the receive chain.
    explore_receive: "Receive",
    /// Button: the change chain.
    explore_change: "Change",
    /// An address's name: `{}` its index on the receive chain.
    explore_receive_n: "Receive {}",
    /// An address's name: `{}` its index on the change chain.
    explore_change_n: "Change {}",
    /// Label: the list's first index, typed.
    explore_from_index: "From index",
    /// Button: the page before; `{}` how many addresses a page holds.
    explore_previous_n: "Previous {}",
    /// Button: the page after; `{}` how many addresses a page holds.
    explore_next_n: "Next {}",
    /// Label: how many indices of each chain the search looks at.
    explore_look_through: "Look through",
    /// Button: a search's depth; `{}` the indices of each chain.
    explore_first_n: "First {}",
    /// Button: start the search.
    explore_search: "Search",
    /// Button: stop the search where it is.
    explore_stop: "Stop",
    /// A search under way: `{}` the indices looked at, `{}` how many it looks at, on each chain.
    explore_searched: "Searched {} of {} on each chain",
    /// A search that found nothing: `{}` the indices looked at on each chain.
    explore_not_in_the_first: "Not in the first {} receive and change addresses",
    /// A search's matches: `{}` how many.
    explore_found_n: "{} found",
    /// Matches past the ones listed: `{}` how many.
    explore_more_n: "{} more not listed",
    /// A paste with a character no address has, or too long for one.
    explore_not_an_address: "Not an address",
    /// Label: the self-test result.
    settings_selftest: "Self-test",
    /// The self-test passed: `{}` is the check count.
    settings_selftest_passed: "passed · {} checks",
    /// The self-test failed: `{}` is the check.
    settings_selftest_failed: "failed · {}",
    /// Action that runs the self-test again.
    settings_selftest_run: "Run again",
    /// Failure title.
    selftest_failed_title: "Self-test failed",
    /// Row label: the check that did not reproduce.
    selftest_check_row: "Check",
    /// The only button.
    selftest_exit: "Exit",
    // ----- Codex32 error correction (docs/CODEX32.md §8.4), faraday-core -----
    /// The tick on a codex32 form, off whenever the form opens.
    codex32_correct_suggest: "Suggest corrections",
    /// Title over a proposal's pairs.
    codex32_correct_possible: "Possible correction",
    /// One box a proposal changes: `{}` the box, counted from 1 at `m`, what it was, what it would be.
    codex32_correct_pair: "Box {} · {} → {}",
    /// The page of pairs on show: `{}` the page, `{}` how many.
    codex32_correct_page: "Page {} of {}",
    /// Button: the proposal's previous page.
    codex32_correct_previous: "Previous",
    /// Button: the proposal's next page.
    codex32_correct_next: "Next",
    /// Button: the paper is right, the typing was wrong.
    codex32_correct_mistyped: "I mistyped",
    /// Button: the paper itself is wrong.
    codex32_correct_damaged: "The paper is damaged",
    /// Button: the boxes marked `?` cannot be read on the paper.
    codex32_correct_unreadable: "Unreadable on the paper",
    /// Button: back to the string as typed, to edit it.
    codex32_correct_check_boxes: "Check the boxes",
    /// Button: on past a caution or the repairs.
    codex32_correct_continue: "Continue",
    /// The caution before a proposal that fills a long run of unreadable boxes.
    codex32_correct_assumes: "This assumes every other box is right",
    /// Title over the pairs as repairs to write on a damaged paper.
    codex32_correct_repairs: "Repairs to write on the paper",
    /// Title over the re-entry of the changed boxes.
    codex32_correct_type_again: "Type the changed boxes from the paper",
    /// The re-entry's count: `{}` boxes typed, `{}` to type.
    codex32_correct_typed_of: "{} of {} boxes typed",
    /// The re-entry differed from the proposal: nothing taken.
    codex32_correct_differs: "The boxes typed differ from the correction",
    /// The caution before a proposal for unreadable boxes is taken as it is.
    codex32_correct_from_checksum: "These characters come from the checksum, not from the paper",
    /// Button: take the proposal as it is.
    codex32_correct_take: "Take the correction",
    /// No correction within BIP 93's bound.
    codex32_correct_not_found: "No correction found within 4 changes or 8 unreadable boxes",
    /// Too many unreadable boxes: `{}` the most, `{}` the longest run filled.
    codex32_correct_too_many_unreadable: "More unreadable boxes than can be filled: at most {}, or one run of up to {}",
    /// Boxes 4 to 8 differ from the strings already in, or are unreadable.
    codex32_correct_header_fixed: "Boxes 4 to 8 must read as in the strings already in",
    /// The text is not a codex32 string even with its unreadable boxes: `{}` why.
    codex32_correct_malformed: "Not a codex32 string: {}",
    /// The one correction is not a codex32 string: `{}` why.
    codex32_correct_not_a_string: "The correction is not a codex32 string: {}",
    /// Beside the fingerprint of a key whose strings were typed again from the paper.
    codex32_correct_entry_corrected: "Share entry corrected",
    /// Beside the fingerprint of a key whose strings were repaired or filled from the checksum.
    codex32_correct_repairs_unchecked: "Share repairs unchecked",
    /// Beside a string repaired or filled from the checksum.
    codex32_correct_repaired_not_checked: "Repaired, not checked",
    // ----- The dice roller on Faraday Demo (docs/CODEX32.md §5.6), faraday-core -----
    /// Label over the roller's die: what its values are.
    roller_by_this_computer: "Rolled by this computer",
    /// Button: roll the die as many times as one click rolls.
    roller_roll: "Roll",
    /// Button: stop a click's rolls where they are.
    roller_stop: "Stop",
    /// Choice: one roll per click.
    roller_one: "1",
    /// Choice: five rolls per click.
    roller_five: "5",
    /// Choice: ten rolls per click.
    roller_ten: "10",
    /// Choice: rolls until the entry is full.
    roller_to_the_end: "To the end",
    /// The typed number's field when it is empty.
    roller_number: "Number",
    // ----- The five-dice roller (docs/CODEX32.md §5.5), faraday-core/src/fresh/roller/ -----
    /// Heading of the codex32 dice step's roller, beside the entry for real dice.
    roller_or_let_faraday_roll: "Or let Faraday roll",
    /// Choice: the five dice once.
    roller_reach_pass: "One pass",
    /// Choice: both passes, one character.
    roller_reach_character: "One character",
    /// Choice: every character of the share being entered.
    roller_reach_share: "One share",
    /// Result's entropy row after the roller made every value.
    roller_entropy_simulated: "Entropy · simulated dice",
    /// Result's entropy row after the roller and real dice made the values.
    roller_entropy_both: "Entropy · simulated dice and your dice",
    // ----- Make codex32 shares (docs/CODEX32.md §6, §7), faraday-core/src/codex32/ -----
    /// New key's Start card: the label over the ways to add a key held.
    keygen_add_a_key_you_have: "Add a key you have",
    /// New key's Start card, under Make a new key: BIP-39 words.
    keygen_bip39_words: "BIP-39 words",
    /// New key's Start card, under Make a new key: Make codex32 shares.
    keygen_codex32: "Codex32",
    /// The Start card closed on a way under Make a new key. `{}`: the way.
    keygen_start_new_key: "Make a new key · {}",
    /// Make codex32 shares' card in New key's step list.
    codex32_codex32: "Codex32",
    /// Card title (§6.1).
    codex32_scheme: "Scheme",
    /// Card title (§6.2), and a row of Strings made.
    codex32_identifier: "Identifier",
    /// Card title (§6.3), and a row of Strings made.
    codex32_entropy: "Entropy",
    /// Card title (§6.4) once every character is in.
    codex32_enter: "Characters",
    /// Card title (§6.5).
    codex32_worksheets: "Worksheets",
    /// Card title (§6.6), a stepper's label, and the closed card's name.
    codex32_strings: "Strings",
    /// Card title (§6.7).
    codex32_strings_made: "Strings made",
    /// The entry's title. `{}`: the share ("Share A"), the character, the characters in all.
    codex32_share_character: "{} · character {} of {}",
    /// The Strings card's title. `{}`: which string, how many.
    codex32_share_n_of: "Share {} of {}",
    /// A share named by its index. `{}`: the index, upper case.
    codex32_share_x: "Share {}",
    /// The string whose index is s.
    codex32_the_secret: "Secret",
    /// The Scheme card closed. `{}`: the length, the split.
    codex32_scheme_summary: "{} characters · {}",
    /// A split. `{}`: the threshold, the strings.
    codex32_k_of_n: "{} of {}",
    /// A card closed once its work is done.
    codex32_done: "Done",
    /// The Strings card closed. `{}`: how many strings.
    codex32_strings_n: "{} written",
    /// The Scheme card's first Choice, and a row of Strings made.
    codex32_length: "Length",
    /// Length: 48 characters.
    codex32_chars_48: "48 characters",
    /// Length: 74 characters.
    codex32_chars_74: "74 characters",
    /// The Scheme card's second Choice, and a row of Strings made.
    codex32_split: "Split",
    /// Split: 2 of 3.
    codex32_two_of_three: "2 of 3",
    /// Split: 3 of 5.
    codex32_three_of_five: "3 of 5",
    /// Split: the steppers.
    codex32_other: "Other",
    /// Split: no threshold.
    codex32_split_alone: "None, the secret alone",
    /// Other's first stepper.
    codex32_threshold: "Threshold",
    /// Under the split where k is 4 or more. `{}`: k.
    codex32_no_translation_sheet: "The booklet publishes no translation sheet for k = {}; derived shares are this device's arithmetic",
    /// The identifier's Suggest button.
    codex32_suggest: "Suggest",
    /// The identifier's Continue before four characters are in.
    codex32_four_characters: "Four characters",
    /// Where an identifier is already used: the codex32 strings in Add a key.
    codex32_used_in_add_a_key: "Used by the strings typed in Add a key",
    /// The Entropy card's Choice: dice.
    codex32_roll_dice: "Roll dice",
    /// The Entropy card's Choice: characters typed.
    codex32_type_the_characters: "Type the characters",
    /// Above the entry of characters typed (§5.2, owner 2026-10-23).
    codex32_not_chosen: "Characters from dice or a worksheet, not chosen",
    /// A refusal from the codec. `{}`: its reason.
    codex32_refused: "Refused: {}",
    /// The small panel's tile: the dice sheet.
    codex32_tile_dice: "Dice",
    /// The small panel's tile and the strip's first chip: the checksum worksheet.
    codex32_tile_checksum: "Checksum",
    /// The small panel's tile: the share's string.
    codex32_tile_string: "String",
    /// The dice de-biasing sheet's head.
    codex32_dice_sheet: "Dice de-biasing",
    /// A die's track. `{}`: its number.
    codex32_die_n: "Die {}",
    /// A die's pair. `{}`: first, second, lower or higher, the bit.
    codex32_die_then: "{} then {} · {} · {}",
    /// A second value lower than the first.
    codex32_lower: "lower",
    /// A second value higher than the first.
    codex32_higher: "higher",
    /// An equal pair, struck. `{}`: the die, its first and second value.
    codex32_die_equal: "Die {} · {} then {} · equal · roll both again",
    /// A die's pair in a cell's working. `{}`: the die, first, second, the bit.
    codex32_die_row: "Die {} · {} then {} · {}",
    /// The tree's path reaching its leaf. `{}`: the bits, their value, the character.
    codex32_path_leaf: "{} = {} → {}",
    /// The checksum worksheet's head. `{}`: the share.
    codex32_checksum_of: "Checksum of {}",
    /// The review's checksum worksheet head. `{}`: the share, where its data came from.
    codex32_checksum_of_marked: "Checksum of {} · {}",
    /// The data came from dice.
    codex32_from_your_rolls: "from your rolls",
    /// The data was typed.
    codex32_as_typed: "as typed",
    /// A worksheet row: the string.
    codex32_row_string: "String",
    /// A worksheet row: the residue of MS.
    codex32_row_ms: "MS",
    /// A worksheet row: a sum. `{}`: its number.
    codex32_row_sum: "Sum {}",
    /// A worksheet row: a lookup from the checksum table. `{}`: its number.
    codex32_row_lookup: "Table {}",
    /// A translation worksheet row: an input times its symbol. `{}`: the input.
    codex32_row_times: "{} × symbol",
    /// A translation worksheet's head. `{}`: what it makes.
    codex32_translation_to: "Translation to {}",
    /// Label over a sheet's translation symbols.
    codex32_translation_symbols: "Translation symbols",
    /// Label over a recovery sheet's symbols.
    codex32_recovery_symbols: "Recovery symbols",
    /// Label over a symbol's pair symbols.
    codex32_pair_symbols: "Pair symbols",
    /// An input's symbol. `{}`: the input, the symbol's name.
    codex32_symbol_of: "{} {}",
    /// The review's Previous, on the small panel.
    codex32_previous: "Previous",
    /// The review's Next, on the small panel.
    codex32_next: "Next",
    /// A review tab: a share's dice. `{}`: the share.
    codex32_tab_dice: "Dice · {}",
    /// A review tab: a share's checksum. `{}`: the share.
    codex32_tab_checksum: "Checksum · {}",
    /// A review tab: a derived share. `{}`: its index.
    codex32_tab_share: "Share {}",
    /// A rolled character in the dice review. `{}`: the box, the rows, the bits, the character.
    codex32_box_rolled: "Box {} · {} · {} → {}",
    /// A typed character in the dice review. `{}`: the box, the character.
    codex32_box_typed: "Box {} · typed · {}",
    /// A die's two values while the sheets are masked.
    codex32_die_masked: "• then •",
    /// A character in the dice review while the sheets are masked. `{}`: the box, the mask.
    codex32_box_masked: "Box {} · {}",
    /// The Strings card's toggle: the string.
    codex32_string: "String",
    /// The Strings card's toggle: the Codex32QR code.
    codex32_codex32qr: "Codex32QR",
    /// A string typed back with a box that differs. `{}`: the box.
    codex32_box_differs: "Box {} differs",
    /// A string typed back the same.
    codex32_typed_back_same: "Typed back the same",
    /// Continue before the string is typed back.
    codex32_type_it_back: "Type the string back, or skip",
    /// The Strings card's Skip.
    codex32_skip: "Skip",
    /// After a first press on Skip (§6.6).
    codex32_skip_caution: "Without typing it back, a box copied wrongly is found only when the strings are needed. Press Skip again to go on.",
    /// A row of Strings made.
    codex32_fingerprint: "Fingerprint",
    /// Strings made's entropy: dice.
    codex32_your_dice: "your dice",
    /// Strings made's entropy: characters typed.
    codex32_your_characters: "your characters",
    /// Strings made's entropy: both.
    codex32_your_dice_and_characters: "your dice and your characters",
    /// The entry's pad for real rolls, beside the roller (CODEX32 §5.5).
    codex32_your_dice_head: "Your dice",
    /// Under the dice sheet: the dice of the character in progress
    /// whose value the roller made (CODEX32 §5.5).
    codex32_dice_rolled: "{} rolled by this computer",
    /// A die in that line.
    codex32_die_short: "Die {}",
    /// A rolled character's review line, marked as the roller's.
    codex32_box_rolled_by: "{} · rolled by this computer",
    /// The Entropy card's row of dice, by their faces (CODEX32 §5.1).
    codex32_dice: "Dice",
    /// A die on that row. `{}`: its faces.
    codex32_die_size: "d{}",
    /// The Entropy card's row of each die's colour (CODEX32 §5.1).
    codex32_die_colours: "Colours",
    /// A die's colour, a press steps to the next. `{}`: the die, the colour.
    codex32_die_colour: "Die {} · {}",
    /// A die's colour.
    codex32_colour_red: "Red",
    /// A die's colour.
    codex32_colour_orange: "Orange",
    /// A die's colour.
    codex32_colour_yellow: "Yellow",
    /// A die's colour.
    codex32_colour_green: "Green",
    /// A die's colour.
    codex32_colour_blue: "Blue",
    /// A die's colour.
    codex32_colour_purple: "Purple",
    /// A die's colour.
    codex32_colour_white: "White",
    /// A die's colour.
    codex32_colour_black: "Black",
    /// The dice sheet's place for a die's marker before its first roll
    /// (the booklet's words), on screen and printed.
    codex32_free_space: "Free space",
    /// Head over the dice sheet's five tracks (the booklet's words).
    codex32_die_tracks: "Die tracks",
    /// Head over a track's pads 1 to 6 (the booklet's words).
    codex32_die_pads: "Die pads",
    /// Head over a track's pads 7 to 20, for dice of more faces (the
    /// booklet's words).
    codex32_die_pads_large: "Die pads (d7+)",
    /// Head over each die's first roll, its marker (CODEX32 §5.5).
    codex32_first_roll: "First roll",
    /// Head over each die's second roll, the die.
    codex32_second_roll: "Second roll",
    /// What a die's pair gives. `{}`: lower or higher, the bit.
    codex32_pair_bit: "{} · {}",
    /// The note when the dice are changed in the middle of a character
    /// (CODEX32 §5.1).
    codex32_dice_changed: "Dice changed: this character's rolls are cleared",
    /// The roller's dice, in the "Demo only" box. `{}`: the die, as "d20".
    codex32_rolls_die: "Rolls {}",
    // ----- The step-through in the "Demo only" card (CODEX32 §5.5, owner 2026-10-24), faraday-core -----
    /// Where the step-through stands. `{}`: the step, the character's
    /// steps, the character's number in its share.
    codex32_walk_count: "Step {} of {} · character {}",
    // ----- The Working area of the person's own entry (CODEX32 §5.7, owner 2026-10-24), faraday-core -----
    /// The Working area's head, in the input panel beside the pad.
    codex32_track_head: "Working",
    /// Where the Working area stands before a value of the character is
    /// entered. `{}`: the character's number in its share.
    codex32_track_char: "Character {}",
    /// Where it stands in the character in progress. `{}`: the value's
    /// number among the character's, the character's number in its share.
    codex32_track_roll: "Roll {} · character {}",
    /// The same three on the small panel's Working tile, short so they
    /// fit its head row (owner, 2026-10-24). `{}`: the step, the
    /// character's steps, the character's number in its share.
    codex32_walk_count_short: "{}/{} · ch {}",
    /// `{}`: the character's number in its share.
    codex32_track_char_short: "Ch {}",
    /// `{}`: the value's number, the character's number.
    codex32_track_roll_short: "Roll {} · ch {}",
    /// What the person enters next, a die's first roll. `{}`: the die.
    codex32_track_next_first: "Next: die {}, first roll.",
    /// What the person enters next, a die's second roll. `{}`: the die.
    codex32_track_next_second: "Next: die {}, second roll.",
    /// A character typed, into its box. `{}`: the character, the box.
    codex32_track_typed: "{} goes in box {}.",
    /// What the person types next. `{}`: the box.
    codex32_track_next_box: "Next: the character for box {}.",
    /// The Working area's Previous, an icon button's tooltip.
    tip_previous_step: "Previous step",
    /// The Working area's Next, an icon button's tooltip.
    tip_next_step: "Next step",
    /// Play, which runs the steps.
    codex32_walk_play: "Play",
    /// Play running: the button that stops it.
    codex32_walk_pause: "Pause",
    /// Head over how far Play runs.
    codex32_walk_play_runs: "Play runs",
    /// Head over how fast Play runs.
    codex32_walk_speed: "Speed",
    /// A step a second.
    codex32_walk_slow: "Slow",
    /// The default speed.
    codex32_walk_normal: "Normal",
    /// Quick.
    codex32_walk_fast: "Fast",
    /// A step every frame.
    codex32_walk_fastest: "Fastest",
    /// A die's first roll. `{}`: the die, its value, the pad.
    codex32_walk_first: "Die {}, first roll: {}. Marker on pad {}.",
    /// A die's second roll, higher. `{}`: the die, the second value, the
    /// second value, the first value.
    codex32_walk_higher: "Die {}, second roll: {}. {} is higher than {}, so bit 1.",
    /// A die's second roll, lower. `{}`: as `codex32_walk_higher`.
    codex32_walk_lower: "Die {}, second roll: {}. {} is lower than {}, so bit 0.",
    /// An equal pair. `{}`: the die, its first value, its second.
    codex32_walk_equal: "Die {}: {} then {}. Equal, so roll both again.",
    /// The five bits into the box. `{}`: the bits, their sum as written
    /// ("8 + 4 + 1 = 13"), the leaf, its character, the character, the box.
    codex32_walk_bits: "Bits {} = {}. Leaf {} of the tree is {}. {} goes in box {}.",
    /// A sum cell of the checksum worksheet, as the addition wheel gives it.
    /// `{}`: the box, the row's letter, the two added, the sum.
    codex32_walk_sum: "Box {}, row {}: {} + {} = {}",
    /// A table row selected by two leading cells. `{}`: the first box,
    /// the second, their characters, the row's name, the row, the box it
    /// is written from.
    codex32_walk_table: "Boxes {}–{} read {}. Table row {} is {}, written from box {}.",
    /// A table row selected by one leading cell. `{}`: the box, its
    /// character, the row's name, the row, the box it is written from.
    codex32_walk_table_one: "Box {} reads {}. Table row {} is {}, written from box {}.",
    /// A checksum box worked from below. `{}`: the box, the cell below,
    /// the table cell between, the box's character.
    codex32_walk_below: "Box {}, from below: {} + {} = {}",
    // ----- The tour of the shares made and S, after the last checksum (CODEX32 §5.7, owner 2026-10-24), faraday-core -----
    /// The choice beside the Working area: at the end of the last
    /// initial share's checksum, Next goes through each derived share and
    /// S, a step at a time (owner's wording).
    codex32_tour_choice: "Show the other shares and S as they fill",
    /// Where the tour stands. `{}`: the step, the steps in all, the sheet
    /// ("Share E", "Secret").
    codex32_tour_count: "Step {} of {} · {}",
    /// The same on the small panel's Working tile. `{}`: the step, the
    /// steps, the sheet's letter.
    codex32_tour_count_short: "{}/{} · {}",
    /// What Next does at the end of the last checksum. `{}`: the first
    /// sheet ("Share E").
    codex32_tour_next: "Next: {}.",
    /// What Next does at the tour's last step.
    codex32_tour_next_card: "Next: the worksheets.",
    /// A derived share's translation symbol. `{}`: the initial share, the
    /// share made, the symbol, its value.
    codex32_tour_symbol: "Share {}'s translation symbol for {}: {}, value {}, from the derivation table.",
    /// S's symbol at k = 2: one recovery symbol, nothing to fuse. `{}`:
    /// the share, the symbol, its value.
    codex32_tour_recovery_one: "Share {}'s recovery symbol: {}, value {}.",
    /// S's symbol at k of 3 or more: the recovery symbols fused. `{}`:
    /// the share, its recovery symbols, the fused symbol, its value.
    codex32_tour_recovery: "Share {}'s recovery symbols {} fused: {}, value {}.",
    /// A translated cell, and the wheel. `{}`: the share, the box, its
    /// character, the symbol, the product, the symbol.
    codex32_tour_translate: "Share {}, box {}: {} × {} = {}. Translation wheel set to {}.",
    /// A sum of the translated cells. `{}`: the box, the sheet, the
    /// cells added, the sum.
    codex32_tour_sum: "Box {}, {}: {} = {}",
    /// A cell of the final row of a share's checksum check. `{}`: the
    /// box, the two added, the sum, what it should read.
    codex32_tour_final: "Final row, box {}: {} + {} = {}. It should read {}.",
    /// The check done, its final row as it should read. `{}`: the share.
    codex32_tour_holds: "{}'s final row reads SECRETSHARE32: its checksum holds.",
    /// The check done, its final row not as it should read. `{}`: the
    /// share, the first box that differs.
    codex32_tour_fails: "{}'s final row differs at box {}: its checksum fails.",
    /// The head over a derived share's checksum worksheet, worked as the
    /// booklet's verification. `{}`: the share.
    codex32_tour_check_head: "Checksum of {}, checked",
    /// Over the pad of ten-faced dice: a die numbered 0 to 9 enters its 0
    /// as 10 (CODEX32 §5.1).
    codex32_d10_zero: "0 is 10",
    /// A tens digit typed on the keyboard, waiting for its units digit,
    /// Space or Enter (dice of ten faces or more). `{}`: the digit.
    codex32_face_pending: "Face {}…",
    /// Strings made's button (§6.8).
    codex32_check_on_another: "Check on another device",
    /// Check on another device's row: the subsets to try.
    codex32_sets_to_try: "Sets to try",
    /// The cell sheet's Done.
    codex32_done_button: "Done",
    /// A cell sheet's head: a box. `{}`: the box.
    codex32_cell_box: "Box {}",
    /// A cell sheet's head: the residue of MS. `{}`: the box.
    codex32_cell_ms: "Box {} · residue of MS",
    /// A cell sheet's head: a sum. `{}`: the box, the row.
    codex32_cell_sum: "Box {} · sum row {}",
    /// A cell sheet's head: a lookup. `{}`: the box, the row.
    codex32_cell_lookup: "Box {} · table row {}",
    /// A cell named in a working. `{}`: its row, its box, its value.
    codex32_named_cell: "{}, box {}: {}",
    /// The residue of MS row's working.
    codex32_ms_residue: "The residue of MS: the same on every codex32 sheet",
    /// A cell's value in its working. `{}`: the value.
    codex32_this_is: "This cell: {}",
    /// How a sum is worked.
    codex32_addition_wheel: "Addition wheel, or the addition table",
    /// A checksum box's rule.
    codex32_worked_from_below: "Worked from the bottom row up: the cell below and the table cell added",
    /// The final row reads what it must. `{}`: the letter.
    codex32_should_read_does: "Should read {}: it does",
    /// The final row does not. `{}`: the letter.
    codex32_should_read_not: "Should read {}: it does not",
    /// A lookup's leading pair. `{}`: the row, the pair.
    codex32_sum_starts: "Sum row {} starts {}",
    /// A lookup's table row. `{}`: the pair, the character, of how many.
    codex32_table_row: "Checksum table, row {}, character {} of {} (booklet pages 15 and 16)",
    /// A header box: the threshold. `{}`: it.
    codex32_header_threshold: "Threshold · {}",
    /// A header box: the identifier. `{}`: it.
    codex32_header_identifier: "Identifier · {}",
    /// A header box: the share index. `{}`: it.
    codex32_header_index: "Share index · {}",
    /// A data cell typed.
    codex32_typed: "Typed",
    /// A translated cell. `{}`: the input, the box, its character, the symbol, the product.
    codex32_translated_is: "Share {}, box {}: {} × {} = {}",
    /// The translation wheel's setting. `{}`: the symbol.
    codex32_wheel_set_to: "Translation wheel set to {}",
    /// A translation sum. `{}`: the cells added, the sum.
    codex32_sum_is: "{} = {}",
    /// A symbol's value. `{}`: its name, its value.
    codex32_symbol_value: "{} · value {}",
    /// What a symbol is the fusion of. `{}`: the kind, the symbols.
    codex32_fused_from: "{}: {}",
    /// Guided: the Scheme card.
    guide_codex32_scheme: "Choose how long each string is and how many strings any rebuild needs. Continue goes on to the identifier.",
    /// Guided: the Identifier card.
    guide_codex32_identifier: "Type four characters that name this set on every string. Continue goes on to where the randomness comes from.",
    /// Guided: the Entropy card.
    guide_codex32_entropy: "Choose five dice rolled on the de-biasing sheet, or characters you made on paper. Continue opens the sheets.",
    /// Guided: the entry.
    guide_codex32_enter: "Enter each roll or character; the sheets fill as you go, as on paper. Continue opens the finished worksheets.",
    /// Guided: the worksheets.
    guide_codex32_sheets: "Read each worksheet against your paper, a tab each. Continue shows the strings to write.",
    /// Guided: the strings.
    guide_codex32_strings: "Write each string down, then type it back to check the copy. Continue goes to the next string.",
    /// Guided: Strings made.
    guide_codex32_made: "These facts let another device check the strings. Lock in loads the key the strings make.",
    // ----- Codex32 strings typed, checked and scanned (docs/CODEX32.md §8.1, §8.3, §8.5, §10.1), faraday-core -----
    /// The same codex32 string typed or scanned again: nothing changes.
    codex32_form_already_in: "Already in",
    /// A string at a share index already in, which differs from the one held. `{}`: the index, upper case.
    codex32_form_clash: "Share {} is in already, and differs",
    /// Label over the string held, beside the one typed.
    codex32_form_held: "Held",
    /// Label over the string just typed or scanned.
    codex32_form_new: "This one",
    /// The boxes the two strings differ in. `{}`: the boxes, counted from 1 at `m`.
    codex32_form_differ: "Boxes {} differ",
    /// Keep the string held, drop the one typed.
    codex32_form_keep: "Keep the one held",
    /// Put the string typed in place of the one held.
    codex32_form_replace: "Replace it",
    /// A share whose checksum fails. `{}`: its index, upper case.
    codex32_form_share_fails: "Share {} · checksum fails",
    /// The secret string, its checksum failing.
    codex32_form_secret_fails: "Secret · checksum fails",
    /// The codex32 form's button to Check a string (§8.3).
    codex32_form_check_a_string: "Check a string",
    /// Check a string: the form's guide line.
    codex32_form_check_guide: "One string, checked. Nothing is loaded or kept.",
    /// Check a string: its button.
    codex32_form_check_it: "Check it",
    /// Check a string: leaves for the form that adds strings.
    codex32_form_back_to_adding: "Back to adding strings",
    /// Check a string: the checksum holds.
    codex32_form_holds: "Checksum holds",
    /// Check a string: the checksum fails.
    codex32_form_fails: "Checksum fails",
    /// Check a string: the string is a share of the set typed in.
    codex32_form_of_the_set: "Of the set typed in",
    /// Check a string: the string is not of the set typed in.
    codex32_form_not_of_the_set: "Not of the set typed in",
    /// Check a string: what the string is. `{}`: the index, upper case; the identifier; the threshold.
    codex32_form_checked_share: "Share {} · id {} · {} needed",
    /// Check a string: the secret. `{}`: the identifier.
    codex32_form_checked_secret: "The secret · id {}",
    // Fewer presses (F103).
    /// Button: closes the board's Add list, a sheet on a small panel.
    board_list_close: "Close",
    // ----- Make a new wallet: Single key, backup in shares (docs/CODEX32.md §6.9), and fewer presses, faraday-core/src/create.rs and screens/create.rs -----
    /// Kind row: the wallet kind whose one key is made as codex32 shares.
    create_single_key_shares: "Single key, backup in shares (codex32)",
    /// The word in that row's name that carries the tooltip below: it is in the name as written there.
    create_shares_tip_word: "codex32",
    /// Tooltip on "codex32" in the Kind row's name: an explanation on a working screen, by the owner's decision (2026-10-24).
    create_shares_tip: "codex32 is the only method that makes a seed and its shares fully by hand, the checksum included.",
    /// The kind's line where no scheme is chosen.
    create_shares_line_plain: "One key, its backup in codex32 shares",
    /// Make a new wallet's Kind row: Single key's line.
    create_single_key_line: "One key, its words on paper",
    /// Make a new wallet's Kind row: the backup in shares' line. `{}`: the shares, how many rebuild it.
    create_shares_line: "One key, its backup in {} shares, any {} rebuild it",
    /// Make a new wallet's Kind row: Multisig's line. `{}`: the keys, the signatures needed.
    create_multisig_line: "{} keys, any {} sign",
    /// Quorum's stepper for the backup in shares: the threshold.
    create_shares_rebuild: "How many to rebuild it",
    /// Quorum's stepper for the backup in shares: the strings made.
    create_shares_count: "How many shares",
    /// Quorum's line for the backup in shares. `{}`: k, n.
    create_shares_any: "Any {} of the {} shares rebuild the key.",
    /// Keys' slot button for the backup in shares: opens Make codex32 shares with the scheme.
    create_make_codex32_shares: "Make codex32 shares",
    /// Why the backup in shares is not made from the key in its slot.
    create_shares_key_needed: "Key 1 is not a key made here as codex32 shares: clear it and press Make codex32 shares.",
    /// Check's row label for the backup in shares.
    create_backup: "Backup",
    /// Check's row value. `{}`: the identifier, k, n.
    create_shares_set: "codex32 {} · {} of {}",
    /// Keys' foot once every slot is filled: makes the wallet.
    create_make_the_wallet_button: "Make the wallet",
    /// Check's foot: the addresses match the other software.
    create_it_matches: "It matches",
    /// Use vaults, closed before it is answered.
    create_not_answered: "Not answered",
    /// Guided: the backup in shares' Quorum. `{}`: k, n.
    create_guide_shares_quorum: "Any {} of the {} codex32 shares rebuild the key, and fewer show nothing of it. Keep each share in a place of its own, so no one place holds the key.",
    /// Guided: the backup in shares' Keys.
    create_guide_shares_keys: "Make codex32 shares makes the key from your dice with this scheme, every worksheet shown as on paper. Lock in fills the slot and makes the wallet.",
    // ----- Codex32 in backups (docs/CODEX32.md §9), faraday-core -----
    /// A codex32 string on paper, as the plan names it. `{}`: the seed, the share index.
    c32_plan_string: "Seed {}: codex32 {}",
    /// A codex32 string's Codex32QR drawn on paper. `{}`: the seed, the share index.
    c32_plan_qr: "Seed {}: Codex32QR {}",
    /// A codex32 string as a text file. `{}`: the seed, the share index.
    c32_plan_file: "Seed {}: codex32 {} file",
    /// A codex32 string's Codex32QR as a picture. `{}`: the seed, the share index.
    c32_plan_qr_file: "Seed {}: Codex32QR {} picture",
    /// A codex32 string, as the chart names it. `{}`: the key's number, the share index.
    c32_glance_string: "Key {} codex32 {}",
    /// The chart's sheet on a codex32 string: its checklist item.
    c32_glance_check: "Check my copy",
    /// That row's second line.
    c32_glance_strings: "Codex32 strings",
    /// The chart's sheet on a codex32 string: the set's actions.
    c32_glance_set: "Add, replace or rotate the set",
    /// Guided: the checklist's codex32 strings item.
    c32_guide_strings_item: "Write each codex32 string at the place the plan gives it, then type it back so a wrong box is caught now. These strings go on paper only, never to a file or a stick.",
    /// A codex32 string typed back that holds. `{}`: its share index, the set's identifier.
    c32_holds: "{} · checksum holds, a string of {}",
    /// A codex32 string typed back whose checksum fails.
    c32_checksum_fails: "Checksum fails",
    /// A codex32 string typed back whose header is not the set's. `{}`: the box, the identifier.
    c32_not_of_set: "Box {} is not this set's: not a string of {}",
    /// Rotate the set, where enough old strings may be in other hands. `{}`: how many, the identifier.
    c32_rotate_not_enough: "{} strings of {} may be in other hands: rotating is not enough; move the coins to a new key",
    /// Rotate the set, every old string held. `{}`: the strings.
    c32_rotate_destroy_all: "Destroy {}. Then no string of the old set is left.",
    /// Rotate the set, some lost or exposed. `{}`: the held strings, the others, the threshold, the strings.
    c32_rotate_destroy: "Destroy {}. Then at most {} is left, and fewer than {} strings of a set of {} are worthless.",
    /// A share to add, where the set has every index.
    c32_set_full: "This set has every index",
    /// More strings to type for a share. `{}`: how many.
    c32_set_type_more: "Type {} strings of the set first",
    /// The strings typed do not rebuild this key.
    c32_set_not_this_seed: "These strings do not rebuild this key",
    /// The new set's identifier, not four characters.
    c32_set_identifier: "Type four characters for the new identifier",
    /// The new set's identifier, the old one's.
    c32_set_identifier_old: "That is the old set's identifier: type a new one",
    /// The checklist's codex32 item. `{}`: the seed.
    c32_item_title: "Codex32 strings of seed {}",
    /// A set of the secret alone.
    c32_secret_alone: "The secret alone",
    /// A split. `{}`: the threshold, the strings.
    c32_k_of_n: "{} of {}",
    /// A form's facts. `{}`: the split, the length.
    c32_facts: "{} · {} characters",
    /// A form's facts with its identifier. `{}`: the identifier, the rest.
    c32_facts_id: "Identifier {} · {}",
    /// A key of words split as codex32: what the strings hold.
    c32_holds_the_seed: "Holds · the seed, not the words. Not for checking by hand.",
    /// A string and where it is kept. `{}`: the index, the places.
    c32_string_at: "{} · {}",
    /// A string typed back the same, or skipped.
    c32_checked: "Checked",
    /// A string not yet typed back.
    c32_not_checked: "Not checked",
    /// Strings of the set whose index is not known. `{}`: how many.
    c32_more_unknown: "And {} more, their index not known",
    /// The strings typed back rebuild this key.
    c32_rebuilds: "These strings rebuild this key",
    /// The strings typed back rebuild another key.
    c32_rebuilds_not: "These strings rebuild another key: look at each again",
    /// The set's actions' heading.
    c32_the_set: "The set",
    /// Add a share.
    c32_add_a_share: "Add a share",
    /// Rotate the set.
    c32_rotate: "Rotate the set",
    /// Replace a lost share. `{}`: its index.
    c32_replace: "Replace {}",
    /// A string open. `{}`: its index, its places.
    c32_share_at: "Share {} · {}",
    /// A string of a set made before, typed from its paper.
    c32_type_from_paper: "Type it from the paper",
    /// Back to the item's strings.
    c32_all_strings: "All strings",
    /// Replace a lost share, its title. `{}`: the index, the identifier.
    c32_replacing: "Replace {} of {}",
    /// Add a share, its title. `{}`: the index, the identifier.
    c32_adding: "Add {} to {}",
    /// Strings typed in so far. `{}`: how many, how many needed, the identifier.
    c32_typed_in: "{} of {} strings of {} typed",
    /// Make the share. `{}`: its index.
    c32_make_share: "Make share {}",
    /// Rotate the set, its title. `{}`: the identifier.
    c32_rotate_title: "Rotate {}",
    /// Rotate the set: each old string marked.
    c32_rotate_mark: "Mark each string held, lost or exposed",
    /// A string held.
    c32_held: "Held",
    /// A string lost.
    c32_lost: "Lost",
    /// A string exposed.
    c32_exposed: "Exposed",
    /// A held string confirmed destroyed. `{}`: its index.
    c32_destroyed: "{} destroyed",
    /// The new set, its title. `{}`: the old identifier.
    c32_new_set_of: "A new set in place of {}",
    /// The new set's identifier.
    c32_new_identifier: "New identifier",
    /// Make the new set.
    c32_make_new_set: "Make the new set",
    /// Leave a set action.
    c32_leave: "Leave",
    /// A codex32 seed's group head on the board. `{}`: the seed, the identifier, the split.
    c32_board_head_id: "{} · codex32 {} · {}",
    /// A string's chip. `{}`: the seed, the index.
    c32_board_chip: "{} · {}",
    /// Check: a string kept nowhere. `{}`: the index, the seed.
    c32_warn_nowhere: "Share {} of {} is kept nowhere",
    /// Check: a place that keeps enough strings. `{}`: the place, how many, the set.
    c32_warn_place_seed: "{} keeps {} shares of {}: the seed",
    /// Check: a place whose loss leaves too few strings. `{}`: the place, the threshold, the set.
    c32_warn_lose_place: "Losing {} leaves fewer than {} shares of {}",
    /// Check: a vault with a seed and a string of it. `{}`: the vault, the seed.
    c32_warn_vault_both: "Vault {} holds {} and a share of it",
    /// Check: a set's seed whole in a vault. `{}`: the vault, the seed.
    c32_warn_vault_whole: "Vault {} holds {} whole: one place with its passphrase is enough",
    /// Check: a string as a file in the clear. `{}`: the set.
    c32_warn_file: "A share of {} is a file that is not encrypted; enough such files are the seed",
    /// Templates item: the codex32 sheets, after the words'. `{}`: how many, the length.
    c32_templates_clause: "{} from Blank templates, codex32, {} characters",
    /// Templates item: the codex32 sheets alone. `{}`: how many, the length.
    c32_templates_starter: "Print {} from Faraday Starter: Blank templates, codex32, {} characters",
    /// Templates item: codex32 sheets with a Codex32QR, after the words'. `{}`: how many, the length, the Starter's Copy choice ("Codex32QR", "Both").
    c32_templates_clause_copy: "{} from Blank templates, codex32, {} characters, {}",
    /// Templates item: codex32 sheets with a Codex32QR, alone. `{}`: how many, the length, the Starter's Copy choice.
    c32_templates_starter_copy: "Print {} from Faraday Starter: Blank templates, codex32, {} characters, {}",
    // ----- A string's Codex32QR drawn by hand (docs/CODEX32.md §9.5, §10.2), faraday-core -----
    /// The sheet a string's Codex32QR is drawn on.
    c32_qr_sheet: "Sheet · Faraday Starter, Blank templates, codex32, 48 characters, Codex32QR",
    // ----- A QR copied by hand, SeedQR or Codex32QR (docs/BACKUP-FLOW.md §17.24), faraday-core -----
    /// The copy grid's choice: marked block by block.
    copy_grid_blocks: "Blocks",
    /// The copy grid's choice: marked row by row.
    copy_grid_rows: "Rows",
    /// The block marked. `{}`: its name, "A1" on a Codex32QR, "5 · 10" (first row · first column) on a SeedQR.
    copy_grid_block: "Block {}",
    /// Button: the block (or row) before.
    copy_grid_prev: "Previous",
    /// Button: the next block (or row).
    copy_grid_next: "Next",
    /// Key under the copy grid: the squares the blank sheet prints.
    copy_grid_printed: "Grey · printed on the sheet",
    /// Key under the copy grid: the squares to draw.
    copy_grid_draw: "Black · draw",
    /// Button: the camera on the Codex32QR drawn.
    c32_scan_my_copy: "Scan my copy",
    /// The camera read something that is not a codex32 string of the set's length.
    c32_scanned_not_string: "Not a codex32 string",
    /// Templates item's line with only codex32 sheets. `{}`: the length.
    c32_templates_sub: "Codex32, {} characters",
    // ----- Codex32 strings written, gathered and counted (docs/CODEX32.md §9.1, §9.5), faraday-core -----
    /// A vault saved a codex32 share record. `{}`: the index, the key's fingerprint.
    c32_vault_saved: "Share {} of {} saved",
    /// A string the plan keeps in a vault or a file that this session does not hold. `{}`: the index, the identifier.
    c32_vault_missing: "Share {} of {} is not here: type it in its Codex32 strings item first",
    /// A vault's share record, its row. `{}`: the index, the key's fingerprint.
    c32_vault_row: "Share {} of {}",
    /// The share record's line. `{}`: the identifier, the threshold.
    c32_vault_row_line: "codex32 {} · threshold {}",
    /// Keys loaded at unlock from strings gathered across open vaults. `{}`: the fingerprints.
    c32_gathered: "Loaded {} from codex32 shares",
    /// A vault's line on the backup checklist: the shares of a set it is to keep. `{}`: how many, the threshold, the seed.
    c32_shares_of: "{} of {} shares of {}",
    /// A place's line: shares of a set it keeps, too few. `{}`: how many, the threshold, the seed.
    c32_place_nothing: "{} of {} shares of {} · alone: nothing",
    /// A place's line: enough shares of a set, the seed. `{}`: how many, the threshold, the seed.
    c32_place_seed: "{} of {} shares of {}: the seed",
    /// A place's line: enough shares with a vault's passphrase. `{}`: how many, the threshold, the seed.
    c32_place_vault: "{} of {} shares of {}: the seed, with a vault's passphrase",
    /// A codex32 string's picture file's title. `{}`: the index, the identifier.
    c32_picture_title: "Codex32 share {} of {}",
    /// The picture's line under its title.
    c32_picture_line: "Secret: a share of a seed",
    // ----- Codex32: Make the rest, Add a share, Check a string's worksheet, Practice (docs/CODEX32.md §8.1 to §8.4, §12), faraday-core -----
    /// The codex32 form's button: the set's other strings from the k in.
    c32r_make_the_rest: "Make the rest",
    /// The codex32 form's button: one more string of the set.
    c32r_add_a_share: "Add a share",
    /// The codex32 form: strings of the set typed in that open vaults
    /// keep, which count towards k (§9.5). `{}`: how many.
    c32r_in_vaults: "{} more of this set in open vaults",
    /// The plan's codex32 item for a set that exists (§9.4). `{}`: the
    /// identifier.
    c32r_how_many: "How many strings of {} are there?",
    /// Make codex32 shares' Entropy card: shares made by hand (§5.3).
    c32r_type_shares_by_hand: "Type shares made by hand",
    /// New key's Start card closed on strings typed in going on into
    /// Make the rest or Add a share. `{}`: "Add a key you have", the way.
    c32r_start_typed: "{} · {}",
    /// Add a share's first card, and its index row.
    c32r_index: "Index",
    /// Add a share's first card closed. `{}`: the index.
    c32r_index_value: "Index · {}",
    /// Under Add a share's index.
    c32r_give_index: "Give it an index no other string of the set has",
    /// Add a share's index is one typed in, or the secret's.
    c32r_index_taken: "A string typed in has this index",
    /// A set typed in: the strings it is made from.
    c32r_from: "From",
    /// Add a share's split: the set's threshold alone. `{}`: k.
    c32r_k_needed: "{} needed",
    /// Strings made's entropy for a set typed in.
    c32r_your_shares: "your shares",
    /// Practice: the line on every card after the entry. `{}`: the sample's
    /// identifier.
    c32r_demo_sample: "Practice puts sample {} in place of these strings",
    /// Practice: Strings made's entropy. `{}`: the sample's identifier.
    c32r_entropy_sample: "sample {}",
    /// Practice: a checksum worksheet's marking for the sample.
    c32r_the_sample: "the sample",
    /// Practice with no sample to put in place of the strings entered.
    c32r_demo_no_sample: "Practice has no sample set to show in place of these strings",
    /// Check a string: the final row's first cell that is not SECRETSHARE32's. `{}`: the box.
    c32r_final_differs: "Final row differs at box {}",
    /// Check a string: every row of its worksheet.
    c32r_show_worksheet: "Show the worksheet",
    /// Check a string: back to the final row alone.
    c32r_final_row_only: "Final row only",
    /// Wallets: beside a key loaded from corrected strings. `{}`: the
    /// fingerprint, the line.
    c32r_key_mark: "{} · {}",
    // ----- Plain tooltips on icon-only controls (docs/DESIGN.md §4.12), faraday-ui and faraday-core -----
    /// Tooltip on the way back, an icon alone.
    tip_back: "Back",
    /// Tooltip on the top app bar's ?.
    tip_learn: "Learn",
    /// Tooltip on the ⋮ that opens a menu.
    tip_more: "More",
    /// Tooltip on the Lock icon.
    tip_lock: "Lock",
    /// Tooltip on the keyboard icon.
    tip_keyboard: "Keyboard",
    /// Tooltip on the power icon.
    tip_power_off: "Power off",
    /// Tooltip on an eye that shows what is hidden.
    tip_show: "Show",
    /// Tooltip on an eye that hides what is shown.
    tip_hide: "Hide",
    /// Tooltip on a × that takes a thing off.
    tip_remove: "Remove",
    /// Tooltip on the backup board's link mark.
    tip_link: "Link",
    /// Tooltip on the backup board's warning mark.
    tip_warning: "Warning",
    /// Tooltip on a stepper's −.
    tip_decrease: "Decrease",
    /// Tooltip on a stepper's +.
    tip_increase: "Increase",
}
