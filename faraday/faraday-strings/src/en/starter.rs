//! Faraday's English words for the starter area.

macro_rules! words_starter {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
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
    /// The line Continue anyway asks with: the case its hold is for
    /// (F136).
    devices_continue_ask: "Only for a screen or switch that is your own, in front of you. What is listed stays possible while it is attached.",
    /// The hold that answers it (F136).
    devices_continue_hold: "Hold to continue",
    /// Toast: Check again found them still attached.
    devices_still: "Still attached",
    /// The sheet's Power off, for a virtual machine.
    devices_power_off: "Power off",
    /// The reminder before a secret, on the stick, when the screen's
    /// EDID did not settle whether it may be a TV.
    devices_tv_line: "On a TV? Stop: a smart TV can record its screen.",
    /// Title of the sheet before a secret when it lists only reminders.
    devices_before: "Before a secret is shown",
    /// The fixed line of the sheet before a secret: what no check can detect.
    devices_look_around: "No camera or window behind you. No capture card or extender between this computer and its screen.",
    /// The button that goes on from the reminders.
    devices_understood: "I understand",
    /// The button that goes back from them.
    devices_cancel: "Cancel",
    /// A press that needs the system's randomness, a minute after the first display with none (F102): it does nothing.
    no_randomness: "The system gives no randomness",
    /// A button whose press waits for the system's randomness, and the Lock sheet's line (F102).
    waiting_for_randomness: "Waiting for randomness",
    // ----- The Starter (docs/STARTER.md), faraday-starter -----
    /// Title of the Starter's Home, where the way back goes.
    starter_home: "Home",
    /// Label of Home's This computer card.
    starter_computer: "This computer",
    /// Header of Home's right pane, which holds the cards about the Starter.
    starter_this_starter: "This Starter",
    /// Group heading of Home's Appearance and About rows.
    starter_group_settings: "Settings",
    /// Group heading, in This computer's pane, of what is attached that
    /// holds a restart and the wireless receiver's line.
    starter_group_screens: "Screens and devices",
    /// Appearance's row on Home, under its name.
    starter_row_appearance_sub: "Theme, mode and contrast",
    /// About's row on Home, under its name.
    starter_row_about_sub: "Versions, the release key, text size, remove",
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
    /// Summary above the checks on This computer: every check that
    /// decides it passes.
    starter_check_can: "This computer can start Faraday",
    /// Summary: it can start Faraday, but the stick has no camera to use.
    starter_check_can_no_camera: "This computer can start Faraday, without a camera",
    /// Summary: a check that decides it fails.
    starter_check_cannot: "This computer cannot start Faraday",
    /// Check label.
    starter_check_start_mode: "Start mode",
    /// Start mode value.
    starter_check_uefi: "UEFI",
    /// Start mode value: legacy BIOS.
    starter_check_legacy: "Legacy BIOS: not compatible",
    /// Check label.
    starter_check_secure_boot: "Secure Boot",
    /// Secure Boot and BitLocker value.
    starter_check_on: "On",
    /// Secure Boot value.
    starter_check_off: "Off",
    /// Secure Boot value that could not be read.
    starter_check_not_read: "Not read",
    /// Check label.
    starter_check_camera: "Camera",
    /// Camera value: a USB video camera with uncompressed frames.
    starter_check_camera_usable: "Should work on the stick",
    /// Camera value: no camera.
    starter_check_camera_none: "None found: files go by stick",
    /// Camera value: a camera the stick cannot use.
    starter_check_camera_not_usable: "Not usable on the stick: files go by stick",
    /// Camera value: it could not be read.
    starter_check_camera_unknown: "Could not be checked",
    /// Check label, shown only for an ARM computer.
    starter_check_processor: "Processor",
    /// Processor value.
    starter_check_arm: "ARM: not compatible",
    /// Check label, shown only on Windows with BitLocker on.
    starter_check_bitlocker: "BitLocker",
    /// An ARM computer.
    starter_cannot_start_arm: "This computer cannot start a Faraday stick",
    /// A computer started in legacy BIOS mode.
    starter_cannot_start_bios: "Started in legacy BIOS mode: UEFI has to be on",
    /// A Mac with 32-bit EFI firmware (docs/history/MAC-LOG.md §1).
    starter_cannot_start_mac_firmware: "This Mac's firmware cannot start Faraday",
    /// A Mac with Apple silicon, until its image exists (docs/history/MAC-LOG.md §4).
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
    starter_start_mac_intel: "Starting an Intel Mac from the stick",
    /// Make a Faraday stick's first choice, once the images found include both stick images (docs/history/MAC-LOG.md §6).
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
    /// Make's stick card, Windows: reads the attached stick's boot partition
    /// through the elevated copy and compares it with the release.
    starter_stick_check: "Check this stick",
    /// The attached stick's boot partition is the release's.
    starter_stick_boot_matches: "Boot partition matches this release",
    /// The attached stick's boot partition is the release's except the
    /// boot file the person signed (§4.5).
    starter_stick_boot_signed: "Boot partition matches this release, except the boot file you signed",
    /// The attached stick's boot partition is not the release's.
    starter_stick_boot_differs: "Boot partition is not this release's. Write it again with Replace Faraday, keep data",
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
    starter_row_guide: "Guide",
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
    /// Make's Pi card, with no Pi image found.
    starter_card_pi_none: "No image found",
    /// A card's line naming a version: `{}` the version.
    starter_card_version: "Version {}",
    /// Home's card for Faraday Demo.
    starter_card_desktop: "Faraday Demo",
    /// Home's update card's line: `{}` the installed version, then the running one.
    starter_card_update_line: "Installed {} · this copy {}",
    /// Home's card for the firmware's Faraday stick entry.
    starter_card_entry: "Boot entry",
    /// Tools' Transfer card's line.
    starter_card_transfer_line: "Send files and text, Receive",
    /// Tools' Transfer card's button.
    starter_open_transfer: "Open Transfer",
    /// Tools' Blank templates card's line.
    starter_card_templates_line: "Words, SeedQR, codex32 and Krux code",
    /// Tools' Blank templates card's button.
    starter_open_templates: "Open blank templates",
    /// Title: which disk becomes the stick.
    starter_disks_stick: "Which stick?",
    /// Title: which card becomes the Pi card.
    starter_disks_card: "Which card?",
    /// The Pi card page: which Pi and screen.
    starter_pi_supported_boards: "For a Raspberry Pi 3B, 3B+ or Zero 2 W with the Waveshare 2.8 inch screen or an HDMI screen",
    /// The Pi card page: which cameras.
    starter_pi_cameras: "Cameras: a USB webcam, or Camera Module v1 or v2",
    /// A Starter with no stick image.
    starter_no_stick_image: "Faraday Starter carries no stick image",
    /// A Starter with no Pi image.
    starter_no_pi_image: "Faraday Starter carries no Pi image",
    /// Make's label over the images to write from.
    starter_images: "Image",
    /// A found image's second line: `{}` the folder, `{}` the build.
    starter_image_found: "In {}, build {}",
    /// Label over the image files to download.
    starter_images_missing: "Not found",
    /// An image not found, second line.
    starter_image_download: "Download it, then put it in Downloads",
    /// A listed file whose contents are not the release's.
    starter_image_mismatch: "Not the release's file",
    /// A file of another release: `{}` the version this Starter needs.
    starter_image_other: "Needs version {}",
    /// A computer that cannot write a disk from here.
    starter_no_writer: "This computer cannot write a stick from here",
    /// Which stick? with none attached.
    starter_no_disks_stick: "No USB stick attached · 256 MB or larger",
    /// Which card? with none attached.
    starter_no_disks_card: "No SD card or USB stick attached · 256 MB or larger",
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
    /// After the volumes' names: an encrypted volume is there.
    starter_on_encrypted: "encrypted",
    /// A disk that reports itself fixed, on its row, before what is on it.
    starter_unlike_fixed: "Reports itself as a fixed disk",
    /// A disk larger than a stick: `{}` the largest size of a stick.
    starter_unlike_large: "Over {}, larger than a stick",
    /// A disk holding an encrypted volume or another system's file system.
    starter_unlike_foreign: "Holds a file system a stick does not use",
    /// Confirm's hold for a disk unlike a stick: `{}` its size.
    starter_unlike_hold: "Hold to erase this {} disk",
    /// A disk the computer uses: `{}` the mount point.
    starter_refused_mounted: "Used by this computer: mounted at {}",
    /// A disk the computer uses.
    starter_refused_swap: "Used by this computer: swap",
    /// A disk the computer uses: another device is built on it.
    starter_refused_held: "Used by this computer: another device is built on it",
    /// A disk whose use could not be checked.
    starter_refused_unchecked: "Not checked: what uses it could not be read",
    /// A disk the computer uses.
    starter_refused_system: "Holds this computer's system",
    /// A disk too small: `{}` the size needed.
    starter_refused_small: "Smaller than the image: needs {}",
    /// A size: `{}` the number.
    starter_size_gb: "{} GB",
    /// A size: `{}` the number.
    starter_size_mb: "{} MB",
    /// Title: what to write.
    starter_mode_title: "What to write?",
    /// Option.
    starter_mode_whole_stick: "New Faraday stick",
    /// Option.
    starter_mode_whole_card: "New Faraday card",
    /// Its second line.
    starter_mode_whole_sub: "Everything on it is lost",
    /// Its second line on a Faraday stick.
    starter_mode_whole_data_sub: "Everything on its data partition is lost",
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
    /// A Which stick row that reads like another's: `{}` its second line,
    /// then the disk's serial.
    starter_row_serial: "{} · Serial {}",
    /// The same for a disk with no serial: `{}` its second line, then the
    /// device.
    starter_row_device: "{} · {}",
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
    /// Confirm's line under it: the case its hold is for (F136).
    starter_keep_older_case: "Only to go back to an older release.",
    /// Confirm's Write over a Faraday stick, a hold (F136).
    starter_erase_data_hold: "Hold to erase the data partition",
    /// Confirm's Write over a newer stick, a hold (F136).
    starter_keep_older_hold: "Hold to write the older release",
    /// Keep data writing a dev build.
    starter_keep_dev: "Writes a dev build, with a serial console",
    /// What to write's section and Confirm's row (docs/STARTER.md §4.5).
    starter_boot_file: "Boot file",
    /// Boot file option: the release's BOOTX64.EFI as it is.
    starter_boot_release: "The release's (unsigned)",
    /// Boot file option: one the person signed with their own db key.
    starter_boot_signed: "One I signed",
    /// Its second line before a file is chosen.
    starter_boot_signed_sub: "BOOTX64.EFI signed with your db key",
    /// Row: the name on the signing certificate, the file's own choice.
    starter_boot_cert_name: "Certificate name",
    /// A signer's certificate with no common name.
    starter_boot_signer_unnamed: "None",
    /// Row: the signing certificate's SHA-256, what the person compares.
    starter_boot_fingerprint: "Certificate SHA-256: compare it with your db key's",
    /// Row: the chosen file.
    starter_boot_chosen_file: "File",
    /// Label over the boot files found: `{}` the folder.
    starter_boot_in: "In {}",
    /// No boot file found: `{}` the folder.
    starter_boot_none_in: "No .efi file in {}",
    /// A boot file being checked against the release.
    starter_boot_checking: "Checking against this release",
    /// Confirm refused.
    starter_not_confirmed_stick: "Not the stick that was confirmed: it was removed or changed",
    /// Confirm refused.
    starter_not_confirmed_card: "Not the card that was confirmed: it was removed or changed",
    /// Button on Confirm when the disk is no longer the one confirmed.
    starter_choose_again: "Choose again",
    /// Button.
    starter_write_faraday: "Write Faraday",
    /// Button.
    starter_write: "Write",
    /// Title.
    starter_writing_title: "Writing",
    /// Progress: `{}` done, `{}` in all.
    starter_progress_bytes: "{} of {}",
    /// Writing page, after the window was asked to close.
    starter_close_after_write: "Closes when the write ends.",
    /// Progress while writing.
    starter_progress_writing: "Written {}",
    /// Progress while reading back.
    starter_reading_back: "Reading back · {}",
    /// Progress while zeros go over the stick before it is made: `{}` is
    /// the bytes cleared of the stick's size.
    starter_clearing: "Clearing · {}",
    /// Progress under the clearing pass: the time left, at 8 MB/s, a USB 2
    /// stick's writing speed, rounded up to whole minutes. `{}` minutes.
    starter_clearing_left: "About {} min left",
    // ----- What to write's Clear the whole stick first
    // (docs/STICK-IDENTITY.md §4.2) -----
    /// The option's tick.
    starter_clear_first: "Clear the whole stick first",
    /// Under the tick: the time it adds. `{}` is the stick's size in
    /// bytes divided by 8 MB/s, a USB 2 stick's writing speed, in
    /// minutes rounded up.
    starter_clear_adds: "Adds about {} min",
    /// Confirm's row label for the option when ticked.
    starter_clear_row: "Before writing",
    /// Confirm's row value: `{}` is the same minutes as the option's.
    starter_clear_row_value: "Clears the whole stick, about {} min",
    /// Result title.
    starter_done_title: "Written",
    /// Result title.
    starter_failed_title: "Not written",
    /// Done's line after a stick is made or keep data (docs/STICK-IDENTITY.md §4.3, §4.4).
    starter_checked_boot: "The boot partition matches the release",
    /// Done's line after a stick is made or keep data with a boot file the person signed.
    starter_checked_boot_signed: "The boot partition matches the release, except the boot file you signed, as expected",
    /// Result line, for a Pi card.
    starter_checked_card: "Matches the release image, except this card's own IDs, as expected",
    /// Done's line for a Pi card written with a name (§4.3).
    starter_checked_card_named: "Matches the release image, except this card's own IDs and its name, as expected",
    // ----- What to write's Name field (docs/STICK-IDENTITY.md §6) -----
    /// The field's label, and Confirm's row.
    starter_name_label: "Name",
    /// The field's placeholder while it is empty: no name is written.
    starter_name_none: "No name",
    /// The line under the field when what is typed is written otherwise:
    /// in capitals, as FAT keeps a name. `{}`: the name as written.
    starter_name_written_as: "Written as {}",
    /// Refused: longer than FAT's 11 characters.
    starter_name_long: "At most 11 characters",
    /// Refused: a character a FAT name cannot hold.
    starter_name_character: "Letters, digits, spaces and ! # $ % & ' ( ) - @ ^ _ ` { } ~ only",
    /// Refused: a boot partition's name.
    starter_name_boot: "EFI, bootfs and FARADAYBOOT are boot partitions' names",
    /// Result line.
    starter_pull_stick: "Keep the stick in to start this computer from it. To start another computer, pull it out.",
    /// Result line.
    starter_pull_card: "The card can be pulled out.",
    /// Button.
    starter_done: "Done",
    /// Button.
    starter_try_again: "Try again",
    /// The Guide destination's heading over the Starter's own guide pages.
    starter_guide_head_starter: "Faraday Starter",
    /// The Guide destination's heading over the guides for starting a
    /// computer from the stick.
    starter_guide_head_start: "Starting a computer from the stick",
    /// A guide whose guide page is not there: `{}` its title.
    /// Guide title.
    starter_start_windows: "Starting a Windows computer from the stick",
    /// Guide title.
    starter_start_linux: "Starting a Linux computer from the stick",
    /// Guide title.
    starter_start_no_start: "When the stick does not start",
    /// Guide title.
    starter_start_keep_data: "Replacing Faraday on a stick",
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
    /// Tooltip on the Linux Starter's close button at the top app bar's end.
    starter_tip_close: "Close",
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
    /// The sheet's row for the recovery key's ID, as Windows shows it at its
    /// recovery screen.
    starter_sheet_recovery_key_id: "Recovery key ID",
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
    /// On the Start page after the suspend button's note while BitLocker
    /// is on or not read, and on the restart sheet above its hold, until
    /// the key is printed or BitLocker suspended.
    starter_bitlocker_restart_warning: "Windows may ask for the recovery key if it starts with Secure Boot off or this computer's Secure Boot keys changed.",
    /// The Start page's BitLocker row when the drive is not known.
    starter_bitlocker_row_system: "BitLocker, system drive",
    /// Its value: the state could not be read.
    starter_bitlocker_not_read: "Not read",
    /// Restart sheet's hold while BitLocker's state is not known.
    starter_restart_unread_hold: "Hold to restart, BitLocker not read",
    /// Restart sheet's hold while BitLocker is on, not suspended, no key printed.
    starter_restart_without_key_hold: "Hold to restart without the recovery key",
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
    starter_installed_menu: "Menu entry",
    /// Installed row.
    starter_installed_icon: "Icon",
    /// Installed row.
    starter_installed_entry: "Installed apps entry",
    /// Installed row: the file an installed Starter keeps its text size
    /// in.
    starter_installed_settings: "Settings",
    /// Installed with nothing installed.
    starter_not_installed: "Not installed",
    /// Button.
    starter_uninstall: "Uninstall",
    /// The uninstall's question.
    starter_uninstall_ask: "Uninstall Faraday Starter?",
    /// Under it.
    starter_uninstall_keeps: "Removes the files listed. Received files and Faraday Demo's sticks stay.",
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
    starter_why_held: "Another device is built on it or on one of its partitions. Nothing was written.",
    /// Reason.
    starter_why_unchecked: "What uses it could not be read. Nothing was written.",
    /// Reason.
    starter_why_system: "It holds this computer's system. Nothing was written.",
    /// Reason.
    starter_why_boot_only: "A boot partition alone is written only over a Faraday stick, keeping its data.",
    /// Reason. `{}` disk size, `{}` image size.
    starter_why_too_small: "The disk ({}) is smaller than the image ({}). Nothing was written.",
    /// Reason.
    starter_why_payload_sum: "The image file does not match its list. Nothing was written.",
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
    starter_why_boot_label: "The image's boot partition is not a FAT32 filesystem labelled EFI",
    /// Reason. `{}` device, `{}` its sector size in bytes.
    starter_why_sector_size: "{} has {}-byte sectors; a Faraday stick is made for 512-byte sectors. Nothing was written.",
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
    /// Reason. `{}` device, `{}` sum: what was read back, with the
    /// image's own IDs put back (docs/STICK-IDENTITY.md §4.3).
    starter_why_back_sum: "Reading back {}: with the image's own IDs put back, its SHA-256 is {}, not the image's",
    /// Reason. `{}` sum: the release image's boot partition, which is not
    /// the one the release lists (docs/STICK-IDENTITY.md §4.4).
    starter_why_boot_sum: "The image's boot partition has SHA-256 {}, not the release's. Nothing was written.",
    /// Reason. `{}` the system's words.
    starter_why_random: "This computer's randomness could not be read for the stick's own IDs: {}. Nothing was written.",
    /// Reason. `{}` the system's words.
    starter_why_source_read: "Reading the source failed: {}",
    /// Reason. `{}` the system's words.
    starter_why_write_failed: "Writing failed: {}",
    /// Reason.
    starter_why_payload_changed: "The image file changed while it was read",
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
    /// Reason: no folder that only this user can open could be made for
    /// the sheet with the recovery key. Nothing was written.
    starter_why_key_folder: "No folder that only you can open could be made for the sheet. Nothing was written.",
    /// Reason: a signed boot file of another program.
    starter_why_boot_file_other: "This is not this release's boot file",
    /// Reason: Check this stick could not compare the stick with the release.
    starter_why_not_judged: "This stick could not be checked against this release.",
    /// Reason: the release's boot file, or another, with no signature that holds.
    starter_why_boot_file_unsigned: "This boot file is not signed",
    /// Reason: a signature that does not hold over the file.
    starter_why_boot_file_broken: "This boot file has no signature that holds",
    /// Reason: a signature by an algorithm or with signers the Starter does not read.
    starter_why_boot_file_not_read: "This boot file is signed in a way this check does not read",
    /// Reason: bytes after the signature, or more than one signature.
    starter_why_boot_file_layout: "This boot file holds more than the release's program and one signature",
    /// Reason: `{}` the file.
    starter_why_boot_file_read: "{} could not be read",
    /// Reason: `{}` the file, changed after it was chosen.
    starter_why_boot_file_changed: "{} changed after it was chosen. Nothing was written.",
    /// Reason: `{}` the file.
    starter_why_boot_file_large: "{} is larger than a boot partition",
    /// Reason: `{}` why, one of the `fat_` strings or `secureboot_error_image`.
    starter_why_release_boot_file: "The release's boot file could not be read or replaced: {}",
    /// Reason.
    starter_why_unsupported: "Not on this computer",
    /// The app menu entry's name, in the desktop entry file install writes.
    starter_menu_name: "Faraday Starter",
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
    /// Fault. `{}` count.
    starter_fault_partitions: "it has {} partitions, not two",
    /// Fault.
    starter_fault_boot_type: "its first partition is not an EFI system partition",
    /// Fault.
    starter_fault_data_type: "its second partition is not a basic data partition",
    /// Fault.
    starter_fault_boot_damaged: "its boot partition is damaged",
    /// Fault. `{}` sizes.
    starter_fault_boot_small: "its boot partition ({}) is smaller than the image's ({})",
    /// Fault: the first partition is not where a Faraday stick's is
    /// (docs/STICK-IDENTITY.md §5.1).
    starter_fault_boot_place: "its first partition is not 64 MiB at 1 MiB",
    /// Fault: the second partition is not where a Faraday stick's is.
    starter_fault_data_place: "its second partition is not at least 128 MiB at 65 MiB and inside it",
    /// Fault.
    starter_fault_boot_past_end: "its boot partition runs past its end",
    /// Fault: an image that is not a Pi card's.
    starter_fault_card_layout: "it is not two FAT partitions of 64 MiB and 128 MiB in an MBR",
    /// Fault: an image whose partition has no FAT volume serial to change.
    starter_fault_no_serial: "a partition holds no FAT volume serial",
    /// Fault.
    starter_fault_backup_table: "its backup GPT is not the same as its GPT",
    /// Fault: the name asked for cannot be written to the image's data
    /// partition.
    starter_fault_name: "its data partition cannot take this name",
    /// About row.
    starter_version: "Version",
    /// About label.
    starter_release_key: "Release key",
    /// About label.
    starter_check_title: "Checking this file",
    /// About: the own-risk statement.
    starter_own_risk: "Pre-release, not reviewed by anyone outside the project. Keep a backup that does not depend on Faraday. No warranty of any kind.",
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
    /// Form: a Krux encrypted code.
    starter_tpl_form_krux: "Krux code",
    /// Krux code size row.
    starter_tpl_size: "Size",
    /// A Krux code size. `{}`: squares across, then squares down.
    starter_tpl_square: "{} × {}",
    /// What a 25 × 25 Krux code holds.
    starter_tpl_size_25: "12 words, version 5 at low error correction",
    /// What a 29 × 29 Krux code holds.
    starter_tpl_size_29: "24 words, version 5 at low error correction; 12 words, version 5",
    /// What a 33 × 33 Krux code holds.
    starter_tpl_size_33: "12 or 24 words at the default, version 20; 24 words, version 5; 12 or 24 words, version 0; 12 words, version 1",
    /// What a 37 × 37 Krux code holds.
    starter_tpl_size_37: "24 words, version 20 with a 16-byte id; 24 words, version 10 or version 1; 12 words, version 11 with a 16-byte id",
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
    /// Continue anyway's sheet: the case its hold is for (F136).
    starter_let_through_case: "Only for a screen or switch that is your own, in front of you.",
    /// Continue anyway's hold (F136).
    starter_let_through_hold: "Hold to continue",
    /// The link to the guide page of that name (STARTER §3.1b).
    starter_start_secure_boot_bitlocker: "What Secure Boot and BitLocker are",
    /// The link to the guide page on screens that record (STARTER §3.1a).
    starter_start_screens: "Screens and devices that can record",
    /// The guide page on the management processors under the OS.
    starter_start_what_else_runs: "What else runs on this computer",
    /// The link on Restart from the stick's sheet (STARTER §3.1c).
    starter_start_boot_entry: "About the boot entry",
    /// Home's row removing the firmware's Faraday stick entry (STARTER §3.1c).
    starter_remove_entry: "Remove the Faraday boot entry",
    /// That row's second line.
    starter_remove_entry_sub: "In this computer's boot list",
    /// That row while the removal runs.
    starter_removing_entry: "Removing",
        }
    };
}
