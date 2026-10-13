//! Faraday's English wording: every string Faraday's screens, the stick's
//! device check and the hub app show that OpenSigner does not. OpenSigner's
//! own wording stays in `opensigner/opensigner-core/src/strings/en.rs`;
//! Faraday never adds to that file. Same house style as upstream's.

/// Every string Faraday shows that OpenSigner's `opensigner_core::strings::Strings`
/// does not carry, grouped by screen.
pub struct Strings {
    /// Power-off sheet on a stick that started with Secure Boot off.
    pub power_secure_boot_off: &'static str,
    // ----- The stick's device check (docs/HUB.md §3.1a), faraday-core -----
    /// Title of the sheet that names what is attached.
    pub devices_title: &'static str,
    /// What an IP-KVM or KVM can do, after its name and a colon.
    pub devices_kvm: &'static str,
    /// A wireless receiver attached (HUB §3.1a): one line that holds
    /// nothing, on the stick's Home and the hub app's.
    pub devices_receiver_notice: &'static str,
    /// A Logitech Bolt receiver attached: its keyboards' typing is
    /// encrypted. In place of the receiver line.
    pub devices_bolt_notice: &'static str,
    /// What a virtual machine's host can do.
    pub devices_virtual_machine: &'static str,
    /// The name a virtual machine goes by when its maker is not known.
    pub devices_virtual_machine_unknown: &'static str,
    /// The line under a virtual machine: there is no way on.
    pub devices_real_computer: &'static str,
    /// The name of the screen the person said is a TV.
    pub devices_this_screen: &'static str,
    /// What a TV can do.
    pub devices_tv: &'static str,
    /// A USB device that gives no name: `{}` is its vendor:product ID.
    pub devices_unnamed: &'static str,
    /// The button that reads what is attached again.
    pub devices_check_again: &'static str,
    /// The button that goes on with them attached.
    pub devices_continue: &'static str,
    /// The line Continue anyway asks with.
    pub devices_continue_ask: &'static str,
    /// The button that answers it.
    pub devices_continue_yes: &'static str,
    /// Toast: Check again found them still attached.
    pub devices_still: &'static str,
    /// The sheet's Power off, for a virtual machine.
    pub devices_power_off: &'static str,
    /// The stick's first screen, always: it cannot read the screen.
    pub devices_tv_line: &'static str,
    /// Home, while the system has given no randomness well after the
    /// start: nothing is sealed and no keyboard gets a code.
    pub seed_missing: &'static str,
    /// Title of the question a desktop computer's screen gets once.
    pub devices_tv_ask: &'static str,
    /// The answer: it is a TV.
    pub devices_tv_yes: &'static str,
    /// The answer: it is not.
    pub devices_tv_no: &'static str,
    // ----- The hub app (docs/HUB.md), faraday-hub -----
    /// Title of the hub app's Home, where the way back goes.
    pub hub_home: &'static str,
    /// Label of Home's This computer card.
    pub hub_computer: &'static str,
    /// The card's name when the maker and model could not be read.
    pub hub_computer_unknown: &'static str,
    /// Label above Home's checklist.
    pub hub_checklist: &'static str,
    /// Secure Boot state.
    pub hub_secure_boot_on: &'static str,
    /// Secure Boot state.
    pub hub_secure_boot_off: &'static str,
    /// Secure Boot state that could not be read.
    pub hub_secure_boot_unknown: &'static str,
    /// The system drive: `{}` is its letter.
    pub hub_drive_encrypted: &'static str,
    /// The system drive, not encrypted.
    pub hub_drive_plain: &'static str,
    /// The system drive, suspended by the hub app.
    pub hub_drive_suspended: &'static str,
    /// The system drive, suspended by someone else.
    pub hub_drive_suspended_other: &'static str,
    /// The system drive whose protection could not be read.
    pub hub_drive_unknown: &'static str,
    /// An ARM computer.
    pub hub_cannot_start_arm: &'static str,
    /// A computer started in legacy BIOS mode.
    pub hub_cannot_start_bios: &'static str,
    /// A Mac with 32-bit EFI firmware (docs/MAC.md §1).
    pub hub_cannot_start_mac_firmware: &'static str,
    /// A Mac with Apple silicon, until its image exists (docs/MAC.md §4).
    pub hub_cannot_start_mac_yet: &'static str,
    /// Card line: a Mac with the T2 chip.
    pub hub_mac_t2: &'static str,
    /// Checklist item, card button and page, on an Intel Mac.
    pub hub_item_start_mac: &'static str,
    /// Start this Mac's state: a Mac without the T2 chip.
    pub hub_state_mac_option: &'static str,
    /// Start this Mac's state: a Mac with the T2 chip.
    pub hub_state_mac_t2: &'static str,
    /// Start this Mac's state: a Mac the table does not know.
    pub hub_state_mac_unknown: &'static str,
    /// Over the T2 steps, for a Mac the table does not know.
    pub hub_mac_t2_only: &'static str,
    /// Step.
    pub hub_mac_lower: &'static str,
    /// Under it.
    pub hub_mac_recovery: &'static str,
    /// Under it.
    pub hub_mac_utility_lower: &'static str,
    /// Under Start from the stick, on a Mac.
    pub hub_mac_option: &'static str,
    /// Under it.
    pub hub_mac_firmware_password: &'static str,
    /// Step.
    pub hub_mac_afterwards: &'static str,
    /// Under it.
    pub hub_mac_utility_full: &'static str,
    /// Guide title.
    pub hub_learn_mac_intel: &'static str,
    /// Make a Faraday stick's first choice, once the payload carries both stick images (docs/MAC.md §6).
    pub hub_which_title: &'static str,
    /// Row.
    pub hub_which_pc: &'static str,
    /// Row.
    pub hub_which_apple: &'static str,
    /// Checklist item and its page.
    pub hub_item_make: &'static str,
    /// Checklist item and its page.
    pub hub_item_bitlocker: &'static str,
    /// Checklist item and its page.
    pub hub_item_start: &'static str,
    /// Checklist item.
    pub hub_item_secure_boot: &'static str,
    /// Checklist item and its button.
    pub hub_item_resume: &'static str,
    /// Make a Faraday stick's state.
    pub hub_state_no_stick: &'static str,
    /// Make a Faraday stick's state: `{}` the version.
    pub hub_state_stick: &'static str,
    /// BitLocker's state.
    pub hub_state_protected: &'static str,
    /// BitLocker's state.
    pub hub_state_suspended: &'static str,
    /// BitLocker's state: suspended, not by the hub app.
    pub hub_state_suspended_other: &'static str,
    /// BitLocker's state.
    pub hub_state_resumed: &'static str,
    /// Why Resume is not offered.
    pub hub_resume_needs_secure_boot: &'static str,
    /// Why Resume is not offered, for a computer that had Secure Boot off.
    pub hub_resume_needs_secure_boot_off: &'static str,
    /// Why Resume is not offered in the Windows session that suspended.
    pub hub_resume_needs_restart: &'static str,
    /// Home row.
    pub hub_row_pi: &'static str,
    /// Home row.
    pub hub_row_transfer: &'static str,
    /// Home row.
    pub hub_row_desktop: &'static str,
    /// Its second line.
    pub hub_row_desktop_sub: &'static str,
    /// Home row and page.
    pub hub_row_install: &'static str,
    /// Home row and page.
    pub hub_row_installed: &'static str,
    /// Home row and page, when the running copy is newer.
    pub hub_row_update: &'static str,
    /// Home row and page.
    pub hub_row_about: &'static str,
    /// Home row.
    pub hub_row_learn: &'static str,
    /// Title: which disk becomes the stick.
    pub hub_disks_stick: &'static str,
    /// Title: which card becomes the Pi card.
    pub hub_disks_card: &'static str,
    /// The Pi card page: which Pi and screen.
    pub hub_pi_for: &'static str,
    /// The Pi card page: which cameras.
    pub hub_pi_cameras: &'static str,
    /// A hub app with no stick image.
    pub hub_no_stick_image: &'static str,
    /// A hub app with no Pi image.
    pub hub_no_pi_image: &'static str,
    /// A computer that cannot write a disk from here.
    pub hub_no_writer: &'static str,
    /// Which stick? with none attached.
    pub hub_no_disks_stick: &'static str,
    /// Which card? with none attached.
    pub hub_no_disks_card: &'static str,
    /// A disk with no maker or model.
    pub hub_disk_unnamed: &'static str,
    /// What is on a disk: `{}` the version.
    pub hub_on_faraday: &'static str,
    /// What is on a disk.
    pub hub_on_empty: &'static str,
    /// One volume with no name.
    pub hub_on_volume: &'static str,
    /// Volumes with no names: `{}` how many.
    pub hub_on_volumes: &'static str,
    /// After the volume's name.
    pub hub_on_files_it: &'static str,
    /// After the volumes' names.
    pub hub_on_files_them: &'static str,
    /// After the volumes' names.
    pub hub_on_no_files: &'static str,
    /// A disk the computer uses: `{}` the mount point.
    pub hub_refused_mounted: &'static str,
    /// A disk the computer uses.
    pub hub_refused_swap: &'static str,
    /// A disk the computer uses.
    pub hub_refused_system: &'static str,
    /// A disk too small: `{}` the size needed.
    pub hub_refused_small: &'static str,
    /// A size: `{}` the number.
    pub hub_size_gb: &'static str,
    /// A size: `{}` the number.
    pub hub_size_mb: &'static str,
    /// Title: what to write.
    pub hub_mode_title: &'static str,
    /// Option.
    pub hub_mode_whole_stick: &'static str,
    /// Option.
    pub hub_mode_whole_card: &'static str,
    /// Its second line.
    pub hub_mode_whole_sub: &'static str,
    /// Option.
    pub hub_mode_keep: &'static str,
    /// Its second line.
    pub hub_mode_keep_sub: &'static str,
    /// The fact under it (docs/HUB.md §4.4).
    pub hub_mode_keep_fact: &'static str,
    /// Keep data refused.
    pub hub_mode_same: &'static str,
    /// The whole stick refused for a boot partition image.
    pub hub_mode_boot_only: &'static str,
    /// Button.
    pub hub_continue: &'static str,
    /// Title.
    pub hub_confirm_title: &'static str,
    /// Confirm row.
    pub hub_stick: &'static str,
    /// Confirm row.
    pub hub_card: &'static str,
    /// Confirm row.
    pub hub_size: &'static str,
    /// Confirm row.
    pub hub_device: &'static str,
    /// Confirm row.
    pub hub_serial: &'static str,
    /// A disk with no serial.
    pub hub_serial_none: &'static str,
    /// Confirm row.
    pub hub_on_it: &'static str,
    /// Confirm row.
    pub hub_writes: &'static str,
    /// Confirm and About row.
    pub hub_build: &'static str,
    /// An image with no build ID.
    pub hub_build_unknown: &'static str,
    /// Confirm row.
    pub hub_img_sha: &'static str,
    /// Confirm and About row for a developer's image.
    pub hub_image_sha: &'static str,
    /// About row for a developer's image.
    pub hub_image_file: &'static str,
    /// A developer's image (--image).
    pub hub_dev_image: &'static str,
    /// Keep data over a newer stick: `{}` its version.
    pub hub_keep_newer: &'static str,
    /// Keep data writing a dev build.
    pub hub_keep_dev: &'static str,
    /// Confirm refused.
    pub hub_not_confirmed_stick: &'static str,
    /// Confirm refused.
    pub hub_not_confirmed_card: &'static str,
    /// Button.
    pub hub_erase_and_write: &'static str,
    /// Button.
    pub hub_write: &'static str,
    /// Title.
    pub hub_writing_title: &'static str,
    /// Progress: `{}` done, `{}` in all.
    pub hub_of: &'static str,
    /// Progress while writing.
    pub hub_written_of: &'static str,
    /// Progress while reading back.
    pub hub_reading_back: &'static str,
    /// Result title.
    pub hub_done_title: &'static str,
    /// Result title.
    pub hub_failed_title: &'static str,
    /// Result line.
    pub hub_pull_stick: &'static str,
    /// Result line.
    pub hub_pull_card: &'static str,
    /// Button.
    pub hub_done: &'static str,
    /// Button.
    pub hub_try_again: &'static str,
    /// A guide whose Learn page is not there: `{}` its title.
    pub hub_guide_placeholder: &'static str,
    /// Guide title.
    pub hub_learn_start_windows: &'static str,
    /// Guide title.
    pub hub_learn_start_linux: &'static str,
    /// Guide title.
    pub hub_learn_bitlocker: &'static str,
    /// Guide title.
    pub hub_learn_no_start: &'static str,
    /// Guide title.
    pub hub_learn_keep_data: &'static str,
    /// Linux with a TPM-unlocked disk (docs/HUB.md §6.4).
    pub hub_tpm_line: &'static str,
    /// Button.
    pub hub_bitlocker_step: &'static str,
    /// Step and button.
    pub hub_restart: &'static str,
    /// Card button and the sheet's head (docs/HUB.md §3.1).
    pub hub_restart_stick: &'static str,
    /// The restart's confirmation.
    pub hub_restart_save: &'static str,
    /// Card line, when the restart from the stick did not happen. `{}`
    /// the reason.
    pub hub_stick_restart_failed: &'static str,
    /// Installed, after an uninstall that left the firmware's boot entry.
    /// `{}` the reason.
    pub hub_entry_kept: &'static str,
    /// Button.
    pub hub_restart_now: &'static str,
    /// Button.
    pub hub_cancel: &'static str,
    /// Step.
    pub hub_secure_boot_turn_off: &'static str,
    /// Step, when it is.
    pub hub_secure_boot_already_off: &'static str,
    /// Under it.
    pub hub_fast_boot: &'static str,
    /// Step.
    pub hub_boot_from_stick: &'static str,
    /// Under it.
    pub hub_boot_how: &'static str,
    /// Under it, on Windows.
    pub hub_boot_how_windows: &'static str,
    /// Label (Done, and the Start page's boot-from-stick step): the
    /// expected boot menu entry's row.
    pub hub_boot_menu_label: &'static str,
    /// Value for `hub_boot_menu_label`: `{}` the expected entry.
    pub hub_boot_menu_value: &'static str,
    /// The boot menu entry's pattern for most makers (`decide::
    /// boot_menu_entry`): `{}` the stick's vendor and model.
    pub hub_boot_menu_default: &'static str,
    /// The boot menu entry's pattern for Lenovo's IdeaPad family and
    /// Framework: `{}` the stick's vendor and model.
    pub hub_boot_menu_efi_usb_device: &'static str,
    /// The boot menu entry's pattern for HP: `{}` the stick's vendor and
    /// model.
    pub hub_boot_menu_hp: &'static str,
    /// Step.
    pub hub_afterwards: &'static str,
    /// Step, on Windows.
    pub hub_afterwards_windows: &'static str,
    /// Label above the guide's code.
    pub hub_on_a_phone: &'static str,
    /// Install's check box.
    pub hub_install_desktop: &'static str,
    /// Button.
    pub hub_install_now: &'static str,
    /// Button while installing.
    pub hub_installing: &'static str,
    /// Install done: `{}` where.
    pub hub_installed_in: &'static str,
    /// Row.
    pub hub_installed_version: &'static str,
    /// Row.
    pub hub_this_version: &'static str,
    /// Installed row.
    pub hub_installed_app: &'static str,
    /// Installed row.
    pub hub_installed_desktop: &'static str,
    /// Installed row.
    pub hub_installed_menu: &'static str,
    /// Installed row.
    pub hub_installed_desktop_menu: &'static str,
    /// Installed row.
    pub hub_installed_icon: &'static str,
    /// Installed row.
    pub hub_installed_entry: &'static str,
    /// Installed row.
    pub hub_installed_reminder: &'static str,
    /// Installed with nothing installed.
    pub hub_not_installed: &'static str,
    /// Button.
    pub hub_uninstall: &'static str,
    /// The uninstall's question.
    pub hub_uninstall_ask: &'static str,
    /// Under it.
    pub hub_uninstall_keeps: &'static str,
    /// Under it, while the suspension stands.
    pub hub_uninstall_suspended: &'static str,
    /// Reason.
    pub hub_why_not_carried: &'static str,
    /// Reason. `{}` device, `{}` serial, `{}` size.
    pub hub_why_changed: &'static str,
    /// Reason. The serial of a disk that reports none.
    pub hub_why_no_serial: &'static str,
    /// Reason. A size in bytes.
    pub hub_why_bytes: &'static str,
    /// Reason. `{}` mount point.
    pub hub_why_mounted: &'static str,
    /// Reason.
    pub hub_why_swap: &'static str,
    /// Reason.
    pub hub_why_system: &'static str,
    /// Reason.
    pub hub_why_boot_only: &'static str,
    /// Reason. `{}` disk size, `{}` image size.
    pub hub_why_too_small: &'static str,
    /// Reason.
    pub hub_why_payload_sum: &'static str,
    /// Reason. `{}` file, `{}` sum.
    pub hub_why_image_changed: &'static str,
    /// Reason.
    pub hub_why_gone: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_bus: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_unmounting: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_open_write: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_open_read: &'static str,
    /// Reason. `{}` device, `{}` the system's words.
    pub hub_why_device: &'static str,
    /// Reason. `{}` device, `{}` the system's words.
    pub hub_why_flush: &'static str,
    /// Reason. `{}` device, `{}` the fault.
    pub hub_why_not_stick: &'static str,
    /// Reason. `{}` the fault.
    pub hub_why_not_stick_image: &'static str,
    /// Reason. `{}` sizes.
    pub hub_why_boot_larger: &'static str,
    /// Reason.
    pub hub_why_boot_label: &'static str,
    /// Reason.
    pub hub_why_boot_short: &'static str,
    /// Reason. `{}` device.
    pub hub_why_table_changed: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_reading_table: &'static str,
    /// Reason. `{}` device, `{}` byte.
    pub hub_why_back_ends: &'static str,
    /// Reason. `{}` device, `{}` byte.
    pub hub_why_back_differs: &'static str,
    /// Reason. `{}` device, `{}` the system's words.
    pub hub_why_back_failed: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_source_read: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_write_failed: &'static str,
    /// Reason.
    pub hub_why_payload_changed: &'static str,
    /// Reason. `{}` the decoder's words.
    pub hub_why_damaged: &'static str,
    /// Reason. `{}` bytes.
    pub hub_why_longer: &'static str,
    /// Reason. `{}` bytes.
    pub hub_why_shorter: &'static str,
    /// Reason. `{}` sum.
    pub hub_why_wrong_sum: &'static str,
    /// Reason.
    pub hub_why_no_home: &'static str,
    /// Reason.
    pub hub_why_no_desktop: &'static str,
    /// Reason. `{}` sum.
    pub hub_why_cache_sum: &'static str,
    /// Reason. `{}` path, `{}` the system's words.
    pub hub_why_file: &'static str,
    /// Reason. `{}` path of a file that is another program's.
    pub hub_why_in_the_way: &'static str,
    /// Reason. `{}` program, `{}` the system's words.
    pub hub_why_start: &'static str,
    /// Reason. `{}` the system's words.
    pub hub_why_restart: &'static str,
    /// Reason: the administrator permission was declined.
    pub hub_why_declined: &'static str,
    /// Reason. `{}` a program's name.
    pub hub_why_missing: &'static str,
    /// Reason. `{}` efibootmgr's words.
    pub hub_why_boot_entry: &'static str,
    /// Reason.
    pub hub_why_no_stick: &'static str,
    /// Reason.
    pub hub_why_unsupported: &'static str,
    /// The app menu entry's name, in the desktop entry file install writes.
    pub hub_menu_name: &'static str,
    /// The publisher in the hub app's installed-apps entry and the `.exe`'s properties.
    pub hub_publisher: &'static str,
    /// The desktop app's app menu entry's name.
    pub hub_menu_desktop_name: &'static str,
    /// The desktop app's app menu entry's description.
    pub hub_menu_desktop_comment: &'static str,
    /// Fault. `{}` the system's words.
    pub hub_fault_reading: &'static str,
    /// Fault.
    pub hub_fault_no_header: &'static str,
    /// Fault.
    pub hub_fault_not_gpt: &'static str,
    /// Fault.
    pub hub_fault_header_damaged: &'static str,
    /// Fault.
    pub hub_fault_header_checksum: &'static str,
    /// Fault.
    pub hub_fault_entries_damaged: &'static str,
    /// Fault.
    pub hub_fault_entries_past_end: &'static str,
    /// Fault.
    pub hub_fault_entries_checksum: &'static str,
    /// Fault.
    pub hub_fault_disk_uuid: &'static str,
    /// Fault. `{}` count.
    pub hub_fault_partitions: &'static str,
    /// Fault.
    pub hub_fault_boot_name: &'static str,
    /// Fault.
    pub hub_fault_boot_type: &'static str,
    /// Fault.
    pub hub_fault_data_type: &'static str,
    /// Fault.
    pub hub_fault_boot_damaged: &'static str,
    /// Fault. `{}` sizes.
    pub hub_fault_boot_small: &'static str,
    /// Fault.
    pub hub_fault_overlap: &'static str,
    /// Fault.
    pub hub_fault_boot_past_end: &'static str,
    /// About row.
    pub hub_version: &'static str,
    /// About row.
    pub hub_release: &'static str,
    /// About row.
    pub hub_commit: &'static str,
    /// About label.
    pub hub_payload: &'static str,
    /// About, with no payload.
    pub hub_no_payload: &'static str,
    /// About row.
    pub hub_sha_xz: &'static str,
    /// About row: `{}` the file inside.
    pub hub_sha_inner: &'static str,
    /// About label.
    pub hub_release_key: &'static str,
    /// About label.
    pub hub_check_title: &'static str,
    /// About.
    pub hub_check_text: &'static str,
    /// About: a command.
    pub hub_check_cmd_import: &'static str,
    /// About: a command.
    pub hub_check_cmd_verify: &'static str,
    /// About: a command.
    pub hub_check_cmd_sums: &'static str,
    /// About, on Windows.
    pub hub_check_windows: &'static str,
    /// About label, on Windows.
    pub hub_unsigned_title: &'static str,
    /// About, on Windows.
    pub hub_unsigned_text: &'static str,
    /// BitLocker row.
    pub hub_drive: &'static str,
    /// BitLocker, not read.
    pub hub_drive_unread: &'static str,
    /// BitLocker row.
    pub hub_protection: &'static str,
    /// Its value.
    pub hub_protection_none: &'static str,
    /// Its value.
    pub hub_protection_suspended: &'static str,
    /// Its value.
    pub hub_protection_unknown: &'static str,
    /// BitLocker row.
    pub hub_suspended_by_faraday: &'static str,
    /// BitLocker row.
    pub hub_secure_boot_when_suspended: &'static str,
    /// BitLocker row.
    pub hub_secure_boot_now: &'static str,
    /// Value.
    pub hub_yes: &'static str,
    /// Value.
    pub hub_no: &'static str,
    /// Why a BitLocker step did not happen: the account is not an administrator's (HUB §6.4).
    pub hub_why_no_admin: &'static str,
    /// Why BitLocker was not suspended: refused, with its code (HUB §6.4).
    pub hub_why_managed: &'static str,
    /// A BitLocker call that failed, and its code.
    pub hub_why_bitlocker: &'static str,
    /// Why there is nothing to suspend.
    pub hub_why_not_encrypted: &'static str,
    /// Why the drive's protection is not known.
    pub hub_why_not_read: &'static str,
    /// The guard's state line (HUB §6.5).
    pub hub_guard_encrypted: &'static str,
    /// The guard: suspended by Faraday, then protected again by something else (HUB §6.4).
    pub hub_guard_undone: &'static str,
    /// The guard's first action.
    pub hub_show_key: &'static str,
    /// Where the recovery key is also kept: the label (F17).
    pub hub_key_also_kept: &'static str,
    /// Where the recovery key is also kept: the value (F17).
    pub hub_key_also_kept_where: &'static str,
    /// The default BitLocker step: the person has the key (F17).
    pub hub_key_in_hand: &'static str,
    /// The extra-safe path: suspend BitLocker (F17).
    pub hub_extra_safe: &'static str,
    /// The BitLocker state once the person has the key (F17).
    pub hub_state_key_kept: &'static str,
    /// While the elevation prompt is up.
    pub hub_waiting_permission: &'static str,
    /// The key's label.
    pub hub_recovery_key: &'static str,
    /// The key protector's ID, as the recovery screen shows it.
    pub hub_key_id: &'static str,
    /// Under the key's QR code (HUB §6.2, D18).
    pub hub_key_photo: &'static str,
    /// Save the key to a disk, by its name.
    pub hub_key_save: &'static str,
    /// While the key is saved.
    pub hub_key_saving: &'static str,
    /// Where the key was saved.
    pub hub_key_saved: &'static str,
    /// After the key is kept.
    pub hub_key_kept: &'static str,
    /// The two groups asked for, by number.
    pub hub_key_type: &'static str,
    /// A group's field, by number.
    pub hub_key_group: &'static str,
    /// A group typed back that is not the key's.
    pub hub_key_wrong: &'static str,
    /// Back to the key.
    pub hub_key_show_again: &'static str,
    /// The guard's second action.
    pub hub_suspend: &'static str,
    /// While BitLocker is suspended.
    pub hub_suspending: &'static str,
    /// While BitLocker is resumed.
    pub hub_resuming: &'static str,
    /// The guard's Done (HUB §6.5).
    pub hub_guard_done: &'static str,
    /// The guard's Done, for a hub app not installed (HUB §6.5).
    pub hub_guard_open_again: &'static str,
    /// After Resume BitLocker.
    pub hub_guard_resumed: &'static str,
    /// Another encryption product's driver found, by name (HUB §6.4).
    pub hub_other_found: &'static str,
    /// The person has that product's rescue disk or password.
    pub hub_other_confirm: &'static str,
    /// The checklist's BitLocker state with another encryption product.
    pub hub_state_other: &'static str,
    /// The file Save writes, by the key ID.
    pub hub_key_file: &'static str,
    /// Uninstall while suspended and Resume not offered (HUB §5.4).
    pub hub_uninstall_suspended_where: &'static str,
    /// A screen read as a TV (HUB §3.1a), by name.
    pub hub_concern_tv: &'static str,
    /// A KVM switch or IP-KVM attached (HUB §3.1a), by name.
    pub hub_concern_kvm: &'static str,
    /// The hub app runs in a virtual machine (HUB §3.1a), by its name.
    pub hub_concern_vm: &'static str,
    /// A remote session, around the recovery key (HUB §3.1a).
    pub hub_concern_remote: &'static str,
    /// A wireless display, around the recovery key (HUB §3.1a), by name.
    pub hub_concern_wireless_display: &'static str,
    /// The name of a screen whose EDID has none.
    pub hub_concern_screen: &'static str,
    /// The name of a USB device with no strings.
    pub hub_concern_device: &'static str,
    /// The name of a virtual machine whose firmware names none.
    pub hub_concern_computer: &'static str,
    /// The name of a remote session.
    pub hub_concern_session: &'static str,
    /// Reads what is attached again (HUB §3.1a).
    pub hub_check_again: &'static str,
    /// Lets one item of concern through for the session (HUB §3.1a).
    pub hub_continue_anyway: &'static str,
    /// Continue anyway's sheet, by the item's name.
    pub hub_let_through_ask: &'static str,
    /// Continue anyway's sheet: how long it lasts.
    pub hub_let_through_until: &'static str,
    /// The link to the Learn page of that name (HUB §3.1b).
    pub hub_learn_secure_boot_bitlocker: &'static str,
    /// The link to the Learn page on screens that record (HUB §3.1a).
    pub hub_learn_screens: &'static str,
    /// The link on Restart from the stick's sheet (HUB §3.1c).
    pub hub_learn_boot_entry: &'static str,
    /// Home's row removing the firmware's Faraday stick entry (HUB §3.1c).
    pub hub_remove_entry: &'static str,
    /// That row's second line.
    pub hub_remove_entry_sub: &'static str,
    /// That row while the removal runs.
    pub hub_removing_entry: &'static str,
    // ----- backup (faraday-core/src/backup.rs) -----
    /// Text in `copy_scan_line`. `{}`: `k`.
    pub backup_your_copy_scans_but: &'static str,
    /// Text in `copy_scan_line`.
    pub backup_your_copy_scans: &'static str,
    /// Text in `envelope`. `{}`: `i + 1`, `p.len()`.
    pub backup_share_of: &'static str,
    /// Text in `envelope`.
    pub backup_the_backup_sheet: &'static str,
    /// Text in `envelope`. `{}`: `i + 1`, `i + 1`, `fp`, `sheet`.
    pub backup_envelope_the: &'static str,
    // ----- backups (faraday-core/src/backups.rs) -----
    /// Text in `backups`.
    pub backups_wallet: &'static str,
    /// Text in `backups`.
    pub backups_plan: &'static str,
    /// Text in `backups`. `{}`: `s.name`.
    pub backups_in_locked: &'static str,
    /// Text in `record_lines`. `{}`: `p + 1`.
    pub backups_place: &'static str,
    /// Text in `record_lines`.
    pub backups_vault: &'static str,
    /// Text in `record_lines`. `{}`: `v + 1`.
    pub backups_vault_n: &'static str,
    /// Text in `record_lines`. `{}`: `self.medium.cap()`.
    pub backups_of_files: &'static str,
    /// Text in `record_lines`.
    pub backups_watch_only: &'static str,
    /// Text in `record_lines`.
    pub backups_own_devices: &'static str,
    /// Text in `record_lines`.
    pub backups_checked: &'static str,
    /// Text in `record_lines`.
    pub backups_not_checked: &'static str,
    /// Text in `record_lines`.
    pub backups_not_here: &'static str,
    /// Text in `record_lines`. `{}`: `self.medium.for_box()`.
    pub backups_unprotected: &'static str,
    /// Text in `record_lines`.
    pub backups_shown: &'static str,
    /// Text in `record_lines`.
    pub backups_not_shown: &'static str,
    /// Text in `record_lines`.
    pub backups_not_seen: &'static str,
    /// Text in `record_lines`.
    pub backups_nothing: &'static str,
    /// Text in `files_state`.
    pub backups_not_made: &'static str,
    /// Text in `vault_state`.
    pub backups_a_vault_is: &'static str,
    /// Text in `vault_state`.
    pub backups_not_saved_yet: &'static str,
    /// Text in `vault_state`.
    pub backups_changed_since: &'static str,
    /// Text in `vault_state`.
    pub backups_unchanged: &'static str,
    /// Text in `backup_line`.
    pub backups_not_backed: &'static str,
    /// Text in `backup_line`. `{}`: `n`.
    pub backups_paper_checked: &'static str,
    /// Text in `backup_line`. `{}`: `k`, `n`.
    pub backups_paper_of_checked: &'static str,
    /// Text in `backup_line`. `{}`: `lower_first(&l.state)`.
    pub backups_files: &'static str,
    /// Text in `backup_line`.
    pub backups_planned: &'static str,
    /// Text in `backup_line`. `{}`: `parts.join(" · ")`.
    pub backups_backup: &'static str,
    /// Text in `on_stick`. `{}`: `label`, `crate::vaults::time_of_day(crate::vau…`.
    pub backups_on_2: &'static str,
    /// Text in `on_stick`. `{}`: `label`.
    pub backups_on: &'static str,
    // ----- backups (faraday-core/src/backups_screen.rs) -----
    /// Title in `draw`.
    pub backups_backups: &'static str,
    /// Line in `compact`.
    pub backups_no_wallet: &'static str,
    /// Line in `compact`.
    pub backups_no_backup_plan: &'static str,
    /// Button in `compact`.
    pub backups_back_up: &'static str,
    /// Text in `compact`. `{}`: `at + 1`, `n`.
    pub backups_wallet_of: &'static str,
    /// Button in `compact`.
    pub backups_previous: &'static str,
    /// Button in `compact`.
    pub backups_next: &'static str,
    // ----- bip85 (faraday-core/src/bip85.rs) -----
    /// Toast in `bip85_open`.
    pub bip85_load_a_key_first: &'static str,
    /// Text in `bip85_title`. `{}`: `app_name(b.app)`, `fp`, `b.index().unwrap_or(0)`.
    pub bip85_bip_85_index: &'static str,
    /// Toast in `bip85_to_vault`.
    pub bip85_no_vault_is_open: &'static str,
    /// Text in `bip85_to_vault`. `{}`: `title`.
    pub bip85_is_in_the_vault: &'static str,
    /// Toast in `bip85_load`. `{}`: `self.medium.noun()`.
    pub bip85_remove_the_first: &'static str,
    /// Toast in `bip85_load`. `{}`: `fp_text(fp)`.
    pub bip85_key_added: &'static str,
    /// Text in `bip85_out`.
    pub bip85_a_child_seed: &'static str,
    /// Text in `bip85_out`.
    pub bip85_a_private_key: &'static str,
    /// Text in `bip85_out`.
    pub bip85_secret_bytes: &'static str,
    /// Text in `bip85_out`.
    pub bip85_a_password: &'static str,
    /// Text in `bip85_out`.
    pub bip85_whoever_has_it: &'static str,
    // ----- bip85 (faraday-core/src/bip85_screen.rs) -----
    /// Text in `TITLES`.
    pub bip85_key: &'static str,
    /// Text in `TITLES`.
    pub bip85_application: &'static str,
    /// Text in `TITLES`.
    pub bip85_index: &'static str,
    /// Text in `TITLES`.
    pub bip85_value: &'static str,
    /// Text in `draw`.
    pub bip85_wallets: &'static str,
    /// Text in `draw`.
    pub bip85_derive_with_bip: &'static str,
    /// Text in `length_label`. `{}`: `b.length`.
    pub bip85_words: &'static str,
    /// Text in `length_label`. `{}`: `b.length`.
    pub bip85_bytes: &'static str,
    /// Text in `length_label`. `{}`: `b.length`.
    pub bip85_characters: &'static str,
    /// Text in `summary`.
    pub bip85_not_chosen: &'static str,
    /// Text in `guide`.
    pub bip85_bip_85_turns: &'static str,
    /// Text in `guide`.
    pub bip85_a_child_seed_is: &'static str,
    /// Text in `guide`.
    pub bip85_a_password_for: &'static str,
    /// Text in `guide`.
    pub bip85_the_value_is_made: &'static str,
    /// Text in `guide`.
    pub bip85_index_0_is_the: &'static str,
    /// Text in `guide`.
    pub bip85_the_value_is: &'static str,
    /// Text in `card`. `{}`: `lo`, `hi`.
    pub bip85_to: &'static str,
    /// Line in `card`.
    pub bip85_an_index_is_0: &'static str,
    /// Line in `card`.
    pub bip85_type_the_digits: &'static str,
    /// Button in `card`.
    pub bip85_continue: &'static str,
    /// Line in `card`.
    pub bip85_path: &'static str,
    /// Text in `card`.
    pub bip85_hide: &'static str,
    /// Text in `card`.
    pub bip85_show: &'static str,
    /// Text in `card`. `{}`: `v.name`.
    pub bip85_save_into: &'static str,
    /// Text in `card`.
    pub bip85_load_as_a_key: &'static str,
    /// Text in `card`.
    pub bip85_out_unprotected: &'static str,
    // ----- boot_import (faraday-core/src/boot_import.rs) -----
    /// Text in `status`.
    pub boot_import_can_sign: &'static str,
    /// Text in `status`. `{}`: `self.here`, `n`, `more`.
    pub boot_import_of_keys: &'static str,
    /// Text in `status`.
    pub boot_import_watch_only: &'static str,
    /// Text in `line`.
    pub boot_import_vault_inline: &'static str,
    /// Text in `line`.
    pub boot_import_vaults: &'static str,
    /// Text in `line`.
    pub boot_import_psbt: &'static str,
    /// Text in `line`.
    pub boot_import_psbts: &'static str,
    /// Text in `line`.
    pub boot_import_other_file: &'static str,
    /// Text in `line`.
    pub boot_import_other_files: &'static str,
    /// Text in `line`.
    pub boot_import_no_files: &'static str,
    /// Note in `import_codes`.
    pub boot_import_not_a_psbt: &'static str,
    /// Text in `import_codes`.
    pub boot_import_no_qr_code: &'static str,
    /// Text in `import_codes`.
    pub boot_import_its_codes: &'static str,
    /// Text in `import_codes`. `{}`: `n`, `if n == 1 { EN.boot_import_code } els…`, `kinds.join(", ")`.
    pub boot_import_qr: &'static str,
    /// Text in `import_codes`.
    pub boot_import_code: &'static str,
    /// Text in `import_codes`.
    pub boot_import_codes: &'static str,
    /// Text in `import_stick_gone`. `{}`: `noun`.
    pub boot_import_the_was: &'static str,
    /// Text in `import_view`.
    pub boot_import_wallet: &'static str,
    /// Text in `import_view`. `{}`: `size`.
    pub boot_import_being_read: &'static str,
    /// Text in `import_view`. `{}`: `size`.
    pub boot_import_not_a_kind: &'static str,
    /// Text in `import_view`. `{}`: `why`.
    pub boot_import_not_read: &'static str,
    /// Text in `import_view`. `{}`: `size`.
    pub boot_import_vault_open: &'static str,
    /// Text in `import_view`. `{}`: `size`.
    pub boot_import_vault_n: &'static str,
    /// Text in `import_go`. `{}`: `count(wallets, EN.boot_import_wallet_inline,…`, `count(keys, EN.boot_import_key, EN.bo…`, `count(moved, EN.boot_import_file, EN.…`.
    pub boot_import_imported: &'static str,
    /// Text in `import_go`.
    pub boot_import_wallet_inline: &'static str,
    /// Text in `import_go`.
    pub boot_import_wallets_inline: &'static str,
    /// Text in `import_go`.
    pub boot_import_key: &'static str,
    /// Text in `import_go`.
    pub boot_import_keys: &'static str,
    /// Text in `import_go`.
    pub boot_import_file: &'static str,
    /// Text in `import_go`.
    pub boot_import_files_inline: &'static str,
    // ----- boot_import (faraday-core/src/boot_import_screen.rs) -----
    /// Text in `body`. `{}`: `label`.
    pub boot_import_read_from: &'static str,
    /// Text in `body`. `{}`: `app.medium.noun()`.
    pub boot_import_pull_the: &'static str,
    /// Text in `body`. `{}`: `slot`.
    pub boot_import_open: &'static str,
    /// Text in `body`. `{}`: `crate::vaults::file_text(vault.len)`.
    pub boot_import_locked: &'static str,
    /// Text in `body`.
    pub boot_import_choose: &'static str,
    /// Text in `body`.
    pub boot_import_not_now: &'static str,
    /// Text in `body`.
    pub boot_import_import: &'static str,
    /// Text in `body`.
    pub boot_import_unlock: &'static str,
    /// Text in `unlocking`. `{}`: `time`.
    pub boot_import_unlocking: &'static str,
    /// Text in `lists`.
    pub boot_import_wallets: &'static str,
    /// Text in `lists`.
    pub boot_import_keys_with: &'static str,
    /// Text in `lists`.
    pub boot_import_files: &'static str,
    // ----- catalog (faraday-core/src/catalog.rs) -----
    /// Text in `TILES`.
    pub catalog_new_key: &'static str,
    /// Text in `TILES`.
    pub catalog_dice_coins: &'static str,
    /// Text in `TILES`.
    pub catalog_add_a_key: &'static str,
    /// Text in `TILES`.
    pub catalog_type_its_words: &'static str,
    /// Text in `TILES`.
    pub catalog_scan_a_seedqr: &'static str,
    /// Text in `TILES`.
    pub catalog_standard_or: &'static str,
    /// Text in `TILES`.
    pub catalog_slip_39_shares: &'static str,
    /// Text in `TILES`.
    pub catalog_a_new_key_dealt: &'static str,
    /// Text in `TILES`.
    pub catalog_restore_from: &'static str,
    /// Text in `TILES`.
    pub catalog_shares_typed: &'static str,
    /// Text in `TILES`.
    pub catalog_a_secret_or: &'static str,
    /// Text in `TILES`.
    pub catalog_seed_xor: &'static str,
    /// Text in `TILES`.
    pub catalog_parts_combined: &'static str,
    /// Text in `TILES`.
    pub catalog_explore_a_key: &'static str,
    /// Text in `TILES`.
    pub catalog_its_xpubs_and: &'static str,
    /// Text in `TILES`.
    pub catalog_child_seeds: &'static str,
    /// Text in `TILES`.
    pub catalog_derived_from: &'static str,
    /// Text in `TILES`.
    pub catalog_vanity_address: &'static str,
    /// Text in `TILES`.
    pub catalog_a_first_address: &'static str,
    /// Text in `TILES`.
    pub catalog_silent_payments: &'static str,
    /// Text in `TILES`.
    pub catalog_one_address: &'static str,
    /// Text in `TILES`.
    pub catalog_keys_and_seeds: &'static str,
    /// Text in `TILES`.
    pub catalog_lightning_node: &'static str,
    /// Text in `TILES`.
    pub catalog_from_a_key: &'static str,
    /// Text in `TILES`.
    pub catalog_single_key: &'static str,
    /// Text in `TILES`.
    pub catalog_native_segwit: &'static str,
    /// Text in `TILES`.
    pub catalog_multisig: &'static str,
    /// Text in `TILES`.
    pub catalog_k_of_n_keys: &'static str,
    /// Text in `TILES`.
    pub catalog_taproot_multisig: &'static str,
    /// Text in `TILES`.
    pub catalog_k_of_n_in_one: &'static str,
    /// Text in `TILES`.
    pub catalog_all_keys_one: &'static str,
    /// Text in `TILES`.
    pub catalog_frost_threshold: &'static str,
    /// Text in `TILES`.
    pub catalog_any_m_of_n: &'static str,
    /// Text in `TILES`.
    pub catalog_miniscript_and: &'static str,
    /// Text in `TILES`.
    pub catalog_load_one_from: &'static str,
    /// Text in `TILES`.
    pub catalog_restore_an: &'static str,
    /// Text in `TILES`.
    pub catalog_descriptor_wallet: &'static str,
    /// Text in `TILES`.
    pub catalog_wallets: &'static str,
    /// Text in `TILES`.
    pub catalog_back_up_a_wallet: &'static str,
    /// Text in `TILES`.
    pub catalog_seeds_by_hand: &'static str,
    /// Text in `TILES`.
    pub catalog_sign_a_transaction: &'static str,
    /// Text in `TILES`.
    pub catalog_a_psbt_from: &'static str,
    /// Text in `TILES`.
    pub catalog_nonce_check: &'static str,
    /// Text in `TILES`.
    pub catalog_every_signature: &'static str,
    /// Text in `TILES`.
    pub catalog_decode_a_transaction: &'static str,
    /// Text in `TILES`.
    pub catalog_what_it_spends: &'static str,
    /// Text in `TILES`.
    pub catalog_transactions: &'static str,
    /// Text in `TILES`.
    pub catalog_check_an_address: &'static str,
    /// Text in `TILES`.
    pub catalog_scan_it_the: &'static str,
    /// Text in `TILES`.
    pub catalog_sign_a_message: &'static str,
    /// Text in `TILES`.
    pub catalog_with_a_single: &'static str,
    /// Text in `TILES`.
    pub catalog_messages: &'static str,
    /// Text in `TILES`.
    pub catalog_check_a_signed: &'static str,
    /// Text in `TILES`.
    pub catalog_against_its: &'static str,
    /// Text in `TILES`.
    pub catalog_vaults: &'static str,
    /// Text in `TILES`.
    pub catalog_keys_wallets: &'static str,
    /// Text in `TILES`.
    pub catalog_totp_codes: &'static str,
    /// Text in `TILES`.
    pub catalog_codes_from: &'static str,
    /// Text in `TILES`.
    pub catalog_diceware_passphrase: &'static str,
    /// Text in `TILES`.
    pub catalog_rolled_with: &'static str,
    /// Text in `TILES`.
    pub catalog_gpg_key: &'static str,
    /// Text in `TILES`.
    pub catalog_make_sign_revoke: &'static str,
    /// Text in `TILES`.
    pub catalog_secure_boot: &'static str,
    /// Text in `TILES`.
    pub catalog_pk_kek_and: &'static str,
    /// Text in `TILES`.
    pub catalog_keepass_export: &'static str,
    /// Text in `TILES`.
    pub catalog_a_vault_entry: &'static str,
    /// Text in `TILES`.
    pub catalog_vaults_and: &'static str,
    /// Text in `TILES`.
    pub catalog_open_an_opensigner: &'static str,
    /// Text in `TILES`.
    pub catalog_an_oskb_with: &'static str,
    /// Text in `TILES`.
    pub catalog_hashes: &'static str,
    /// Text in `TILES`.
    pub catalog_sha_256_sha: &'static str,
    /// Text in `TILES`.
    pub catalog_encodings: &'static str,
    /// Text in `TILES`.
    pub catalog_what_a_string: &'static str,
    /// Text in `TILES`.
    pub catalog_descriptor: &'static str,
    /// Text in `TILES`.
    pub catalog_a_descriptor: &'static str,
    /// Text in `TILES`.
    pub catalog_convert_an: &'static str,
    /// Text in `TILES`.
    pub catalog_every_spelling: &'static str,
    /// Text in `TILES`.
    pub catalog_units: &'static str,
    /// Text in `TILES`.
    pub catalog_one_amount: &'static str,
    /// Text in `TILES`.
    pub catalog_calculators: &'static str,
    /// Text in `TILES`.
    pub catalog_miniscript: &'static str,
    /// Text in `TILES`.
    pub catalog_a_policy_compiled: &'static str,
    /// Text in `TILES`.
    pub catalog_bip_39_words: &'static str,
    /// Text in `TILES`.
    pub catalog_2048_words: &'static str,
    /// Text in `TILES`.
    pub catalog_eff_large_list: &'static str,
    /// Text in `TILES`.
    pub catalog_7776_words: &'static str,
    /// Text in `TILES`.
    pub catalog_eff_short_list_1: &'static str,
    /// Text in `TILES`.
    pub catalog_1296_words_each: &'static str,
    /// Text in `TILES`.
    pub catalog_word_lists: &'static str,
    /// Text in `TILES`.
    pub catalog_eff_short_list: &'static str,
    /// Text in `TILES`.
    pub catalog_1296_words: &'static str,
    /// Text in `TILES`.
    pub catalog_qr: &'static str,
    /// Text in `TILES`.
    pub catalog_scan_a_qr_code: &'static str,
    /// Text in `TILES`.
    pub catalog_single_or_animated: &'static str,
    /// Text in `tile_need`.
    pub catalog_load_a_key: &'static str,
    /// Text in `tile_need`.
    pub catalog_at_the_end: &'static str,
    /// Text in `tile_need`.
    pub catalog_load_a_wallet: &'static str,
    /// Text in `tile_need`.
    pub catalog_needs_a_single: &'static str,
    /// Text in `tile_need`.
    pub catalog_copy_a_signed: &'static str,
    /// Text in `tile_need`.
    pub catalog_make_a_vault: &'static str,
    /// Text in `tile_need`.
    pub catalog_unlock_a_vault: &'static str,
    /// Text in `tile_need`.
    pub catalog_copy_an_oskb: &'static str,
    // ----- chart_edit (faraday-core/src/chart_edit.rs) -----
    /// Toast in `chart_notes_change`.
    pub chart_edit_that_does: &'static str,
    /// Text in `edit_title`.
    pub chart_edit_a_new_place: &'static str,
    /// Text in `edit_title`. `{}`: `p + 1`.
    pub chart_edit_place: &'static str,
    /// Text in `edit_title`. `{}`: `v + 1`.
    pub chart_edit_vault: &'static str,
    /// Text in `edit_title`. `{}`: `self.medium.cap()`.
    pub chart_edit_of_files: &'static str,
    /// Text in `edit_title`.
    pub chart_edit_watch_only: &'static str,
    /// Text in `edit_title`. `{}`: `glance::thing_label(g.as_ref(), what)`, `spot(at)`.
    pub chart_edit_save_into: &'static str,
    /// Text in `edit_title`. `{}`: `glance::thing_label(g.as_ref(), what)`.
    pub chart_edit_as_a_file: &'static str,
    /// Text in `edit_title`. `{}`: `glance::thing_label(g.as_ref(), what)`, `spot(at)`.
    pub chart_edit_at: &'static str,
    /// Text in `edit_title`. `{}`: `label(from, what)`, `spot(to)`.
    pub chart_edit_move_to: &'static str,
    /// Text in `edit_title`. `{}`: `place(p)`.
    pub chart_edit_remove: &'static str,
    /// Text in `edit_title`. `{}`: `spot(at)`.
    pub chart_edit_off_the: &'static str,
    /// Toast in `chart_key_checked`.
    pub chart_edit_checked: &'static str,
    /// Text in `envelopes_out`. `{}`: `v + 1`.
    pub chart_edit_vault_stick: &'static str,
    // ----- codes (faraday-core/src/codes.rs) -----
    /// Text in `codes_rows`.
    pub codes_entry: &'static str,
    // ----- codes (faraday-core/src/codes_screen.rs) -----
    /// Text in `now_row`. `{}`: `crate::vaults::tz_offset_text(app.tz_…`.
    pub codes_now_utc: &'static str,
    /// Text in `now_row`.
    pub codes_not_known: &'static str,
    /// Button in `now_row`.
    pub codes_set_the_time: &'static str,
    /// Text in `code_row`.
    pub codes_unreadable: &'static str,
    /// Title in `draw`.
    pub codes_totp_codes: &'static str,
    /// Line in `compact`.
    pub codes_no_entries_with: &'static str,
    /// Button in `compact`.
    pub codes_scan_a_setup: &'static str,
    /// Button in `compact`.
    pub codes_add_an_entry: &'static str,
    // ----- compact (faraday-core/src/compact.rs) -----
    /// Text in `screen_name`.
    pub compact_spend: &'static str,
    /// Text in `screen_name`.
    pub compact_add_a_key: &'static str,
    /// Text in `screen_name`.
    pub compact_back_up: &'static str,
    /// Text in `screen_name`.
    pub compact_sign_a_message: &'static str,
    /// Text in `screen_name`.
    pub compact_check_a_message: &'static str,
    /// Text in `screen_name`.
    pub compact_make_a_new: &'static str,
    /// Text in `screen_name`.
    pub compact_restore_an: &'static str,
    /// Text in `screen_name`.
    pub compact_set_the_time: &'static str,
    /// Text in `screen_name`.
    pub compact_totp_codes: &'static str,
    /// Text in `screen_name`.
    pub compact_create_a_vault: &'static str,
    /// Text in `screen_name`.
    pub compact_new_key: &'static str,
    /// Text in `screen_name`.
    pub compact_bip_85: &'static str,
    /// Text in `screen_name`.
    pub compact_silent_payments: &'static str,
    /// Text in `screen_name`.
    pub compact_explore_a_key: &'static str,
    /// Text in `screen_name`.
    pub compact_lightning: &'static str,
    /// Text in `screen_name`.
    pub compact_vanity_address: &'static str,
    /// Text in `screen_name`.
    pub compact_transaction: &'static str,
    /// Text in `page`.
    pub compact_at_a_glance: &'static str,
    /// Text in `page`.
    pub compact_vault_contents: &'static str,
    /// Text in `page`.
    pub compact_home: &'static str,
    /// Text in `home`.
    pub compact_nothing_loaded: &'static str,
    /// Text in `home`.
    pub compact_key_inline: &'static str,
    /// Text in `home`.
    pub compact_keys: &'static str,
    /// Text in `home`.
    pub compact_wallet: &'static str,
    /// Text in `home`.
    pub compact_wallets_inline: &'static str,
    /// Text in `home`. `{}`: `v`, `if v == 1 { EN.compact_vault } else {…`.
    pub compact_open_3: &'static str,
    /// Text in `home`.
    pub compact_vault: &'static str,
    /// Text in `home`.
    pub compact_vaults_inline: &'static str,
    /// Text in `home`.
    pub compact_bring_in: &'static str,
    /// Text in `home`.
    pub compact_open_2: &'static str,
    /// Text in `home`.
    pub compact_work: &'static str,
    /// Text in `home`.
    pub compact_write_out: &'static str,
    /// Button in `home`.
    pub compact_lock: &'static str,
    /// Button in `home`.
    pub compact_scan: &'static str,
    /// Text in `tiles`.
    pub compact_wallets: &'static str,
    /// Text in `tiles`.
    pub compact_vaults: &'static str,
    /// Text in `tiles`. `{}`: `n`.
    pub compact_locked: &'static str,
    /// Text in `tiles`. `{}`: `o`.
    pub compact_open: &'static str,
    /// Text in `tiles`.
    pub compact_files: &'static str,
    /// Text in `tiles`.
    pub compact_learn: &'static str,
    /// Text in `tiles`.
    pub compact_backups: &'static str,
    /// Text in `tiles`. `{}`: `without`.
    pub compact_not_backed: &'static str,
    /// Text in `tiles`.
    pub compact_tools: &'static str,
    /// Text in `tiles`.
    pub compact_transfer: &'static str,
    /// Text in `tiles`.
    pub compact_settings: &'static str,
    /// Text in `keyboard`.
    pub compact_done: &'static str,
    /// Text in `files_count`.
    pub compact_none: &'static str,
    /// Text in `home_files_line`. `{}`: `self.medium.noun()`.
    pub compact_no: &'static str,
    // ----- compact (faraday-core/src/compact_screens.rs) -----
    /// Line in `wallet_card`.
    pub compact_no_wallet_loaded: &'static str,
    /// Text in `wallet_card`. `{}`: `s.spend.source`.
    pub compact_continue_signing: &'static str,
    /// Text in `wallet_card`. `{}`: `app.inbox[i].name`.
    pub compact_review: &'static str,
    /// Button in `wallet_card`.
    pub compact_rename: &'static str,
    /// Text in `wallet_card`. `{}`: `here`, `m`.
    pub compact_of_signatures: &'static str,
    /// Chip in `wallet_card`.
    pub compact_can_spend: &'static str,
    /// Button in `wallet_card`.
    pub compact_scan_a_key: &'static str,
    /// Text in `wallet_card`. `{}`: `nodes.len()`, `if nodes.len() == 1 { EN.compact_plac…`.
    pub compact_backup: &'static str,
    /// Text in `wallet_card`.
    pub compact_place: &'static str,
    /// Text in `wallet_card`.
    pub compact_places: &'static str,
    /// Text in `wallet_card`. `{}`: `vault`.
    pub compact_plan_in_locked: &'static str,
    /// Text in `wallet_card`.
    pub compact_no_backup_plan: &'static str,
    /// Heading in `wallet_card`.
    pub compact_first_receive: &'static str,
    /// Heading in `silent_card`.
    pub compact_key: &'static str,
    /// Text in `silent_card`.
    pub compact_here: &'static str,
    /// Text in `silent_card`.
    pub compact_not_here: &'static str,
    /// Text in `silent_card`.
    pub compact_load_from_vault: &'static str,
    /// Text in `silent_card`.
    pub compact_add_its_key: &'static str,
    /// Heading in `silent_card`.
    pub compact_address: &'static str,
    /// Line in `silent_card`.
    pub compact_labels_handed: &'static str,
    /// Text in `silent_card`.
    pub compact_address_labels: &'static str,
    /// Text in `silent_card`.
    pub compact_show_wallet: &'static str,
    /// Text in `silent_card`.
    pub compact_remove_from: &'static str,
    /// Text in `about`.
    pub compact_about_this: &'static str,
    /// Text in `entry`. `{}`: `wl.name`.
    pub compact_for: &'static str,
    /// Text in `entry`. `{}`: `fp_text(osk_bip::keys::Fingerprint(fp))`.
    pub compact_for_key: &'static str,
    /// Text in `entry`.
    pub compact_other_languages: &'static str,
    /// Text in `entry`. `{}`: `crate::forms::language_name(lang)`.
    pub compact_other_lists: &'static str,
    /// Text in `entry`.
    pub compact_other_forms: &'static str,
    /// Button in `entry`.
    pub compact_scan_a_seedqr: &'static str,
    /// Text in `entry`. `{}`: `taken.len() + 1`.
    pub compact_word: &'static str,
    /// Text in `entry`.
    pub compact_type_the_next: &'static str,
    /// Text in `entry`.
    pub compact_now_its_tone: &'static str,
    /// Text in `entry`.
    pub compact_no_word_starts: &'static str,
    /// Heading in `entry`.
    pub compact_words: &'static str,
    /// Label in `entry`.
    pub compact_bip_39_passphrase: &'static str,
    /// Text in `entry`.
    pub compact_add_key: &'static str,
    /// Text in `entry`.
    pub compact_make_a_new_key: &'static str,
    /// Text in `entry`.
    pub compact_clear: &'static str,
    /// Text in `entry_form`.
    pub compact_words_of_one: &'static str,
    /// Text in `entry_form`.
    pub compact_nothing_in: &'static str,
    /// Text in `entry_form`.
    pub compact_1_in: &'static str,
    /// Text in `entry_form`. `{}`: `n`.
    pub compact_in: &'static str,
    /// Label in `entry_form`.
    pub compact_passphrase: &'static str,
    /// Text in `entry_form`.
    pub compact_add_this_one: &'static str,
    /// Text in `entry_form`.
    pub compact_scan_a_qr_code: &'static str,
    /// Text in `entry_form`.
    pub compact_make_the_key: &'static str,
    // ----- create (faraday-core/src/create.rs) -----
    /// Text in `name`.
    pub create_single_key_native: &'static str,
    /// Text in `name`.
    pub create_single_key_taproot: &'static str,
    /// Text in `name`.
    pub create_single_key_nested: &'static str,
    /// Text in `name`.
    pub create_single_key_legacy: &'static str,
    /// Text in `name`.
    pub create_multisig_native: &'static str,
    /// Text in `name`.
    pub create_multisig_nested: &'static str,
    /// Text in `name`.
    pub create_multisig_legacy: &'static str,
    /// Text in `name`.
    pub create_multisig_taproot: &'static str,
    /// Text in `name`.
    pub create_threshold_frost: &'static str,
    /// Text in `name`.
    pub create_all_keys_musig2: &'static str,
    /// Text in `line`.
    pub create_bc1q_addresses: &'static str,
    /// Text in `line`.
    pub create_bc1p_addresses: &'static str,
    /// Text in `line`.
    pub create_3_addresses: &'static str,
    /// Text in `line`.
    pub create_1_addresses: &'static str,
    /// Text in `line`.
    pub create_several_keys: &'static str,
    /// Text in `line`.
    pub create_multisig_with_3: &'static str,
    /// Text in `line`.
    pub create_multisig_with: &'static str,
    /// Text in `line`.
    pub create_multisig_on: &'static str,
    /// Text in `line`.
    pub create_any_m_of_n_shares: &'static str,
    /// Text in `line`.
    pub create_every_key_signs: &'static str,
    /// Error in `key_text`.
    pub create_a_share_has: &'static str,
    /// Text in `key_text_at`. `{}`: `path`.
    pub create_not_a_derivation: &'static str,
    // ----- decode (faraday-core/src/decode.rs) -----
    /// Text in `script_kind`.
    pub decode_other_script: &'static str,
    /// Text in `sequence_text`.
    pub decode_final: &'static str,
    /// Text in `sequence_text`.
    pub decode_locktime_on: &'static str,
    /// Text in `sequence_text`.
    pub decode_relative_time: &'static str,
    /// Text in `sequence_text`.
    pub decode_relative_block: &'static str,
    /// Text in `locktime_text`.
    pub decode_0_none: &'static str,
    /// Text in `locktime_text`. `{}`: `n`.
    pub decode_block_height: &'static str,
    /// Text in `locktime_text`. `{}`: `n`.
    pub decode_unix_time: &'static str,
    /// Text in `btc`. `{}`: `Amount::from_sat(sats).to_btc()`.
    pub decode_btc: &'static str,
    // ----- explore (faraday-core/src/explore.rs) -----
    /// Toast in `explore_open`.
    pub explore_load_a_key: &'static str,
    // ----- explore (faraday-core/src/explore_screen.rs) -----
    /// Text in `draw`.
    pub explore_the_public: &'static str,
    /// Line in `draw`.
    pub explore_path: &'static str,
    /// Text in `draw`.
    pub explore_index_1_2: &'static str,
    /// Text in `draw`.
    pub explore_index_1: &'static str,
    /// Text in `draw`.
    pub explore_native_segwit: &'static str,
    /// Text in `draw`.
    pub explore_taproot: &'static str,
    /// Text in `draw`.
    pub explore_nested_segwit: &'static str,
    /// Text in `draw`.
    pub explore_legacy: &'static str,
    /// Line in `draw`.
    pub explore_script: &'static str,
    /// Text in `draw`.
    pub explore_address: &'static str,
    /// Text in `draw`.
    pub explore_public_key: &'static str,
    /// Text in `draw`.
    pub explore_extended_key: &'static str,
    /// Line in `header`.
    pub explore_wallets: &'static str,
    /// Title in `header`.
    pub explore_explore: &'static str,
    // ----- family (faraday-core/src/family.rs) -----
    /// Text in `WORDS_SOURCE`.
    pub family_typed_words: &'static str,
    /// Text in `spend_button`.
    pub family_finish_the_backup: &'static str,
    /// Text in `spend_button`. `{}`: `s.signers.len()`, `self.spend_needed()`.
    pub family_carry_on_the: &'static str,
    /// Text in `spend_button`. `{}`: `self.medium.noun()`.
    pub family_sign_the_psbt: &'static str,
    /// Text in `spend_button`.
    pub family_spend_from_this: &'static str,
    /// Text in `family_act`.
    pub family_finished_transaction: &'static str,
    /// Note in `family_intake_seed`.
    pub family_not_a_seedqr: &'static str,
    /// Text in `family_intake_seed`. `{}`: `n + 1`.
    pub family_key_already: &'static str,
    /// Text in `family_intake_file`.
    pub family_wallet_description_read: &'static str,
    /// Text in `family_intake_file`.
    pub family_wallet_description: &'static str,
    /// Text in `family_intake_file`. `{}`: `k`.
    pub family_part_read_the: &'static str,
    /// Text in `family_intake_file`. `{}`: `k`, `m.have.len()`, `m.n`.
    pub family_part_read_of: &'static str,
    /// Text in `family_intake_file`.
    pub family_a_public_key: &'static str,
    /// Text in `family_intake_file`.
    pub family_a_transaction: &'static str,
    /// Text in `family_intake_file`. `{}`: `name`.
    pub family_is_in_files: &'static str,
    /// Note in `family_intake_again`. `{}`: `fp_text(fp)`.
    pub family_seed_read: &'static str,
    /// Text in `family_no_money`. `{}`: `label`.
    pub family_no_money: &'static str,
    /// Text in `family_seed_with`.
    pub family_passphrase_inline: &'static str,
    /// Text in `family_seed_with`. `{}`: `l`.
    pub family_passphrase_n: &'static str,
    /// Text in `family_words_kind_now`. `{}`: `kind_name(*k)`.
    pub family_words: &'static str,
    /// Text in `family_rebuild`.
    pub family_restored_wallet: &'static str,
    /// Text in `family_rebuild`.
    pub family_its_shares: &'static str,
    /// Error in `family_rebuild`. `{}`: `m.have.len()`, `m.n`.
    pub family_of_keys_in_hand: &'static str,
    /// Text in `kind_name`.
    pub family_native_segwit_2: &'static str,
    /// Text in `kind_name`.
    pub family_taproot: &'static str,
    /// Text in `kind_name`.
    pub family_nested_segwit_2: &'static str,
    /// Text in `kind_name`.
    pub family_legacy: &'static str,
    /// Text in `kind_label`.
    pub family_native_segwit: &'static str,
    /// Text in `kind_label`.
    pub family_taproot_multisig: &'static str,
    /// Text in `kind_label`.
    pub family_nested_segwit: &'static str,
    /// Text in `kind_label`.
    pub family_legacy_multisig: &'static str,
    // ----- family (faraday-core/src/family_screen.rs) -----
    /// Text in `draw`. `{}`: `app.session.wallets[i].name`.
    pub family_spend_from_n: &'static str,
    /// Text in `draw`.
    pub family_wallets: &'static str,
    /// Text in `draw`.
    pub family_this_page_takes: &'static str,
    /// Text in `loaded_compact`.
    pub family_walk_through: &'static str,
    /// Text in `loaded_compact`.
    pub family_no_wallet_loaded: &'static str,
    /// Text in `loaded_compact`. `{}`: `s.spend.source`, `name`.
    pub family_signing_2: &'static str,
    /// Button in `loaded_compact`.
    pub family_continue_signing: &'static str,
    /// Section label in `loaded_compact`.
    pub family_wallets_loaded: &'static str,
    /// Line in `loaded_compact`.
    pub family_none_loaded: &'static str,
    /// Text in `loaded_compact`. `{}`: `here.min(need)`, `need`.
    pub family_can_spend_of: &'static str,
    /// Text in `loaded_compact`. `{}`: `here`, `if here == 1 { EN.family_key } else {…`, `need`.
    pub family_here_needed: &'static str,
    /// Text in `loaded_compact`.
    pub family_key: &'static str,
    /// Text in `loaded_compact`.
    pub family_keys: &'static str,
    /// Text in `loaded_compact`.
    pub family_no_key_here: &'static str,
    /// Line in `loaded_compact`.
    pub family_an_open_vault: &'static str,
    /// Text in `loaded_compact`.
    pub family_spend_from_it: &'static str,
    /// Text in `loaded_compact`.
    pub family_open_it: &'static str,
    /// Section label in `loaded_compact`.
    pub family_keys_with_no: &'static str,
    /// Text in `loaded_compact`.
    pub family_restore_its: &'static str,
    /// Text in `loaded_compact`.
    pub family_spend_as_a_single: &'static str,
    /// Text in `loaded_compact`.
    pub family_home: &'static str,
    /// Text in `loaded_compact`.
    pub family_spend: &'static str,
    /// Text in `summary`.
    pub family_six_stops: &'static str,
    /// Text in `summary`.
    pub family_desktop_copy: &'static str,
    /// Text in `summary`. `{}`: `app.medium.noun()`.
    pub family_running_from: &'static str,
    /// Text in `summary`.
    pub family_nothing_in_yet: &'static str,
    /// Text in `summary`.
    pub family_1_thing_in: &'static str,
    /// Text in `summary`. `{}`: `n`.
    pub family_things_in: &'static str,
    /// Text in `summary`.
    pub family_loaded_in_sparrow: &'static str,
    /// Text in `summary`.
    pub family_money_shows: &'static str,
    /// Text in `summary`.
    pub family_done_in_sparrow: &'static str,
    /// Text in `summary`. `{}`: `s.spend.source`.
    pub family_signing: &'static str,
    /// Text in `summary`.
    pub family_not_here_yet: &'static str,
    /// Text in `summary`. `{}`: `app.medium.for_the()`.
    pub family_nothing_waits: &'static str,
    /// Text in `summary`. `{}`: `app.medium.for_the()`.
    pub family_1_file: &'static str,
    /// Text in `summary`. `{}`: `n`, `app.medium.for_the()`.
    pub family_files: &'static str,
    /// Text in `summary`.
    pub family_after_the_transaction: &'static str,
    /// Button in `body`.
    pub family_i_have_the_psbt_2: &'static str,
    /// Text in `body`.
    pub family_show_the_finished: &'static str,
    /// Text in `more`.
    pub family_less_about_this: &'static str,
    /// Text in `more`.
    pub family_more_about_this: &'static str,
    /// Button in `map`.
    pub family_start: &'static str,
    /// Line in `holding`.
    pub family_or_bring_in: &'static str,
    /// Text in `holding`.
    pub family_scan_the_qr: &'static str,
    /// Text in `holding`.
    pub family_type_the_words: &'static str,
    /// Text in `holding`. `{}`: `i.name`.
    pub family_open: &'static str,
    /// Text in `holding`. `{}`: `app.medium.a()`.
    pub family_copy_files_in: &'static str,
    /// Line in `holding`.
    pub family_what_came_in: &'static str,
    /// Text in `holding`.
    pub family_rebuild_the: &'static str,
    /// Text in `holding`. `{}`: `fp_text(fp)`, `name`.
    pub family_seed_is_not: &'static str,
    /// Text in `holding`.
    pub family_add_its_passphrase: &'static str,
    /// Text in `came_rows`. `{}`: `t`.
    pub family_seed_2: &'static str,
    /// Text in `came_rows`. `{}`: `t`, `label`.
    pub family_seed: &'static str,
    /// Text in `came_rows`. `{}`: `wl.name`, `Session::shape(wl)`.
    pub family_wallet_description_3: &'static str,
    /// Text in `came_rows`. `{}`: `name`.
    pub family_wallet_description_2: &'static str,
    /// Text in `came_rows`. `{}`: `sheets`, `m.have.len()`, `m.n`.
    pub family_part_of_keys: &'static str,
    /// Error in `came_rows`. `{}`: `sheets`.
    pub family_part: &'static str,
    /// Text in `came_rows`. `{}`: `fp`.
    pub family_key_xpub_n: &'static str,
    /// Text in `came_rows`.
    pub family_key_xpub: &'static str,
    /// Text in `came_rows`.
    pub family_transaction: &'static str,
    /// Line in `question`.
    pub family_anything_else: &'static str,
    /// Text in `question`.
    pub family_type_more_words: &'static str,
    /// Text in `question`.
    pub family_that_s_everything: &'static str,
    /// Line in `question`.
    pub family_these_seeds: &'static str,
    /// Text in `question`.
    pub family_scan_the_description: &'static str,
    /// Text in `question`.
    pub family_i_can_t_find: &'static str,
    /// Text in `question`. `{}`: `s.n`.
    pub family_without_the: &'static str,
    /// Text in `vault`. `{}`: `app.medium.noun()`.
    pub family_remove_the_then: &'static str,
    /// Text in `vault`. `{}`: `app.medium.for_the()`.
    pub family_sealed_here: &'static str,
    /// Text in `vault`. `{}`: `app.medium.a()`.
    pub family_copied_in_from: &'static str,
    /// Text in `vault`. `{}`: `app.vaults.time_text(mem, f.header.co…`.
    pub family_unlocks_in: &'static str,
    /// Line in `vault`.
    pub family_passphrase: &'static str,
    /// Line in `vault`.
    pub family_unlocking: &'static str,
    /// Text in `vault`. `{}`: `mib_text(m)`.
    pub family_this_computer: &'static str,
    /// Button in `vault`.
    pub family_unlock: &'static str,
    /// Text in `stick_row`. `{}`: `app.medium.noun()`.
    pub family_plug_the_in: &'static str,
    /// Text in `stick_row`. `{}`: `app.medium.a()`.
    pub family_faraday_asks: &'static str,
    /// Line in `opened`.
    pub family_spend_from: &'static str,
    /// Text in `opened`. `{}`: `Session::shape(wl)`, `here`, `slots.len()`.
    pub family_of_keys_here: &'static str,
    /// Text in `opened`.
    pub family_its_key_is_here: &'static str,
    /// Text in `opened`.
    pub family_its_key_is_not: &'static str,
    /// Text in `opened`. `{}`: `needed`, `n`, `here`.
    pub family_of_keys_sign: &'static str,
    /// Line in `shortcut`.
    pub family_i_have_the_psbt_from: &'static str,
    /// Text in `shortcut`.
    pub family_scan_it: &'static str,
    /// Text in `SHORTCUT`.
    pub family_i_have_the_psbt: &'static str,
    /// Text in `loaded_in_sparrow`.
    pub family_show_the_wallet_qr: &'static str,
    /// Line in `loaded_in_sparrow`.
    pub family_receive_address: &'static str,
    /// Button in `loaded_in_sparrow`.
    pub family_it_is_loaded: &'static str,
    /// Line in `check`.
    pub family_none_of_these: &'static str,
    /// Line in `check`.
    pub family_the_words_may: &'static str,
    /// Text in `check`.
    pub family_scan_more: &'static str,
    /// Text in `check`.
    pub family_type_a_passphrase: &'static str,
    /// Line in `check`.
    pub family_is_there_a_word: &'static str,
    /// Text in `check`.
    pub family_yes_type_it: &'static str,
    /// Text in `check`.
    pub family_no: &'static str,
    /// Text in `check`. `{}`: `crate::family::kind_label(kind)`, `if pass { EN.family_with_a_passphrase…`, `g.at + 1`, `Faraday::guess_total(g)`.
    pub family_trying_of: &'static str,
    /// Text in `check`.
    pub family_with_a_passphrase: &'static str,
    /// Text in `check`.
    pub family_no_passphrase: &'static str,
    /// Text in `check`.
    pub family_show_the_wallet: &'static str,
    /// Text in `check`.
    pub family_open_this_in: &'static str,
    /// Text in `check`.
    pub family_open_this_one: &'static str,
    /// Text in `check`.
    pub family_receive: &'static str,
    /// Text in `check`.
    pub family_change: &'static str,
    /// Text in `check`.
    pub family_show_more_addresses: &'static str,
    /// Text in `check`.
    pub family_no_money_shows: &'static str,
    /// Text in `check`.
    pub family_yes_my_money: &'static str,
    /// Text in `bring`. `{}`: `name`.
    pub family_every_coin_to: &'static str,
    /// Line in `bring`.
    pub family_amount_max_in: &'static str,
    /// Text in `bring`.
    pub family_scan_the_psbt: &'static str,
    /// Text in `bring`.
    pub family_scan: &'static str,
    /// Text in `bring`. `{}`: `app.medium.a()`.
    pub family_on: &'static str,
    /// Text in `bring`. `{}`: `app.medium.a()`.
    pub family_copy_it_in_from: &'static str,
    /// Line in `bring`.
    pub family_in_files: &'static str,
    /// Line in `bring`.
    pub family_signing_this: &'static str,
    /// Text in `bring`.
    pub family_use_this_one: &'static str,
    /// Text in `bring`. `{}`: `wl.name`, `Session::shape(wl)`.
    pub family_spends_from: &'static str,
    /// Line in `bring`.
    pub family_no_wallet_open: &'static str,
    /// Button in `bring`.
    pub family_read_it: &'static str,
    /// Text in `signers`.
    pub family_finish_the_backup_2: &'static str,
    /// Text in `signers`.
    pub family_load_from_vault: &'static str,
    /// Text in `signers`.
    pub family_signs_here: &'static str,
    /// Text in `signers`.
    pub family_not_here: &'static str,
    /// Text in `signers`. `{}`: `f.name`.
    pub family_unlock_n: &'static str,
    /// Text in `signers`.
    pub family_scan_a_key: &'static str,
    /// Text in `signers`.
    pub family_collect_a_signature: &'static str,
    /// Text in `signers`. `{}`: `app.medium.noun()`.
    pub family_remove_the_first: &'static str,
    /// Button in `signers`.
    pub family_continue: &'static str,
    /// Button in `away`.
    pub family_lock: &'static str,
    /// Button in `away`.
    pub family_power_off: &'static str,
    /// Text in `away`.
    pub family_start_over: &'static str,
    // ----- family (faraday-core/src/family_text.rs) -----
    /// Text in `map`.
    pub family_paper: &'static str,
    /// Text in `map`.
    pub family_what_you_have: &'static str,
    /// Text. `{}`: `m.noun()`.
    pub family_the_faraday: &'static str,
    /// Text in `map`.
    pub family_open_the_wallet_here: &'static str,
    /// Text. `{}`: `m.noun()`.
    pub family_start_a_computer: &'static str,
    /// Text in `map`.
    pub family_check_the_balance: &'static str,
    /// Text in `map`.
    pub family_put_the_wallet: &'static str,
    /// Text in `map`.
    pub family_make_the_transaction: &'static str,
    /// Text in `map`.
    pub family_offline: &'static str,
    /// Text in `map`.
    pub family_sign_it_here: &'static str,
    /// Text in `map`.
    pub family_bring_the_psbt: &'static str,
    /// Text in `map`.
    pub family_online: &'static str,
    /// Text in `map`.
    pub family_take_the_signed: &'static str,
    /// Text in `WORDS_ARE_MONEY`.
    pub family_the_words_are: &'static str,
    /// Text in `UNSURE`.
    pub family_not_sure_what: &'static str,
    /// Text in `ANYTHING_HINT`.
    pub family_another_list: &'static str,
    /// Text in `DESCRIPTION_HINT`.
    pub family_its_description: &'static str,
    /// Text in `looks`.
    pub family_a_square_black: &'static str,
    /// Text in `looks`.
    pub family_a_qr_code_the: &'static str,
    /// Text in `looks`.
    pub family_words_numbered: &'static str,
    /// Text in `looks`.
    pub family_the_seed_phrase: &'static str,
    /// Text in `looks`.
    pub family_one_long_line: &'static str,
    /// Text in `looks`.
    pub family_a_descriptor: &'static str,
    /// Text in `looks`.
    pub family_several_sheets: &'static str,
    /// Text in `looks`.
    pub family_one_description: &'static str,
    /// Text in `looks`.
    pub family_a_word_or_sentence: &'static str,
    /// Text in `looks`.
    pub family_if_one_exists: &'static str,
    /// Text in `looks`.
    pub family_a_short_line: &'static str,
    /// Text in `looks`.
    pub family_one_address: &'static str,
    /// Text in `looks`.
    pub family_a_small_device: &'static str,
    /// Text in `looks`.
    pub family_a_hardware_wallet: &'static str,
    /// Text. `{}`: `m.a_full_cap()`.
    pub family_labelled_faraday: &'static str,
    /// Text. `{}`: `m.noun()`.
    pub family_the_starts_the: &'static str,
    /// Text in `title`.
    pub family_how_to_spend: &'static str,
    /// Text in `title`. `{}`: `app.medium.noun()`.
    pub family_start_from_the: &'static str,
    /// Text in `title`.
    pub family_what_do_you: &'static str,
    /// Text in `title`.
    pub family_open_the_wallet: &'static str,
    /// Text in `title`.
    pub family_load_the_wallet: &'static str,
    /// Text in `title`.
    pub family_check_the_money_is: &'static str,
    /// Text in `title`.
    pub family_write_the_payment_in: &'static str,
    /// Text in `title`.
    pub family_check_the_money: &'static str,
    /// Text in `title`.
    pub family_write_the_payment: &'static str,
    /// Text in `title`.
    pub family_bring_the_transaction: &'static str,
    /// Text in `title`.
    pub family_put_everything: &'static str,
    /// Text in `title`.
    pub family_read_the_transaction: &'static str,
    /// Text in `title`.
    pub family_the_transaction: &'static str,
    /// Text in `title`.
    pub family_which_way_it: &'static str,
    /// Text in `title`.
    pub family_nonces: &'static str,
    /// Text in `title`.
    pub family_add_the_seeds: &'static str,
    /// Text in `title`.
    pub family_who_signs: &'static str,
    /// Text in `title`.
    pub family_sign_it: &'static str,
    /// Text in `title`.
    pub family_the_other_signatures: &'static str,
    /// Text in `title`.
    pub family_send_it: &'static str,
    /// Text in `lead`.
    pub family_six_stops_three: &'static str,
    /// Text in `lead`. `{}`: `app.medium.noun()`.
    pub family_this_is_the: &'static str,
    /// Text in `lead`. `{}`: `app.medium.noun()`.
    pub family_this_computer_is: &'static str,
    /// Text in `lead`.
    pub family_scan_or_type: &'static str,
    /// Text in `lead`.
    pub family_in_sparrow_file_new: &'static str,
    /// Text in `lead`.
    pub family_in_sparrow_s: &'static str,
    /// Text in `lead`.
    pub family_words_alone: &'static str,
    /// Text in `lead`.
    pub family_load_this_same: &'static str,
    /// Text in `lead`.
    pub family_this_part_happens: &'static str,
    /// Text in `lead`. `{}`: `app.medium.a()`, `app.medium.a()`, `app.medium.noun()`.
    pub family_two_ways_scanning: &'static str,
    /// Text in `lead`. `{}`: `app.medium.noun()`, `app.medium.a()`, `app.medium.noun()`, `app.medium.noun()`.
    pub family_lock_faraday: &'static str,
    /// Text in `lead`.
    pub family_this_opens_once: &'static str,
    /// Text in `lead`.
    pub family_read_the_summary: &'static str,
    /// Text in `lead`.
    pub family_write_down_the: &'static str,
    /// Text in `lead`.
    pub family_every_key_that: &'static str,
    /// Text in `lead`.
    pub family_one_seed_at: &'static str,
    /// Text in `lead`.
    pub family_signing_adds: &'static str,
    /// Text in `lead`. `{}`: `app.medium.a()`.
    pub family_the_finished: &'static str,
    /// Text in `more`.
    pub family_anyone_who_reads: &'static str,
    /// Text in `more`.
    pub family_nothing_done: &'static str,
    /// Text in `more`.
    pub family_there_is_no: &'static str,
    /// Text in `more`. `{}`: `m.noun()`.
    pub family_if_any_of_this: &'static str,
    /// Text in `more`.
    pub family_shut_the_computer: &'static str,
    /// Text in `more`. `{}`: `m.noun()`.
    pub family_plug_the_faraday: &'static str,
    /// Text in `more`. `{}`: `m.full()`.
    pub family_pick_the_from: &'static str,
    /// Text in `more`. `{}`: `m.noun()`, `m.a()`.
    pub family_if_the_is_missing: &'static str,
    /// Text in `more`.
    pub family_a_vault_unlocks: &'static str,
    /// Text in `more`.
    pub family_words_go_in: &'static str,
    /// Text in `more`.
    pub family_a_wallet_description: &'static str,
    /// Text in `more`.
    pub family_split_sheets: &'static str,
    /// Text in `more`.
    pub family_in_sparrow_file: &'static str,
    /// Text in `more`.
    pub family_only_load_it: &'static str,
    /// Text in `more`.
    pub family_faraday_can: &'static str,
    /// Text in `more`.
    pub family_on_a_phone_you: &'static str,
    /// Text in `more`.
    pub family_paying_into: &'static str,
    /// Text in `more`.
    pub family_faraday_locks: &'static str,
    /// Text in `more`.
    pub family_if_the_envelope: &'static str,
    /// Text in `more`.
    pub family_faraday_on_its: &'static str,
    /// Text in `more`.
    pub family_any_of_these: &'static str,
    /// Text in `more`.
    pub family_then_check_the: &'static str,
    /// Text in `more`.
    pub family_the_first_confirmation: &'static str,
    /// Text in `more`. `{}`: `m.noun()`.
    pub family_when_faraday: &'static str,
    // ----- flow (faraday-core/src/flow.rs) -----
    /// Text in `column_foot`.
    pub flow_guided: &'static str,
    /// Text in `column_foot`.
    pub flow_steps_only: &'static str,
    /// Line in `paged`.
    pub flow_change: &'static str,
    /// Text in `paged`. `{}`: `i + 1`, `cards.len()`.
    pub flow_step_of: &'static str,
    /// Text in `paged`.
    pub flow_about_this_step: &'static str,
    /// Text in `paged`.
    pub flow_home: &'static str,
    // ----- forms (faraday-core/src/forms.rs) -----
    /// Text in `name`.
    pub forms_bip_39_words: &'static str,
    /// Text in `name`.
    pub forms_slip_39_shares: &'static str,
    /// Text in `name`.
    pub forms_codex32: &'static str,
    /// Text in `name`.
    pub forms_seed_xor_parts: &'static str,
    /// Text in `lines`. `{}`: `s.group_index() + 1`, `s.group_count()`, `s.member_index() + 1`, `s.member_threshold()`, `s.identifier()`.
    pub forms_group_of_member: &'static str,
    /// Text in `lines`. `{}`: `id`.
    pub forms_the_secret_id: &'static str,
    /// Text in `lines`. `{}`: `char::from(c.share_index())`, `id`, `c.threshold()`.
    pub forms_share_id_needed: &'static str,
    /// Text in `lines`. `{}`: `i`, `words`.
    pub forms_part_words: &'static str,
    /// Text in `form_add_part`. `{}`: `e`.
    pub forms_not_a_slip_39: &'static str,
    /// Text in `form_add_part`. `{}`: `e`.
    pub forms_not_a_codex32: &'static str,
    /// Error in `form_add_part`.
    pub forms_that_string_is: &'static str,
    /// Error in `form_add_part`.
    pub forms_that_share_is: &'static str,
    /// Text in `form_add_part`. `{}`: `e`.
    pub forms_not_bip_39_words: &'static str,
    /// Error in `form_add_part`.
    pub forms_every_part_has: &'static str,
    /// Text in `form_recover`.
    pub forms_slip_39_key: &'static str,
    /// Text in `form_recover`.
    pub forms_codex32_key: &'static str,
    /// Text in `form_recover`.
    pub forms_seed_xor_key: &'static str,
    /// Text in `form_recover`. `{}`: `e`.
    pub forms_the_shares_do: &'static str,
    /// Text in `form_recover`. `{}`: `e`.
    pub forms_the_strings_do: &'static str,
    /// Error in `form_recover`.
    pub forms_seed_xor_needs: &'static str,
    /// Toast in `form_recover`. `{}`: `fp_text(fp)`.
    pub forms_key_added: &'static str,
    /// Text in `language_name`.
    pub forms_english: &'static str,
    /// Text in `language_name`.
    pub forms_spanish: &'static str,
    /// Text in `language_name`.
    pub forms_french: &'static str,
    /// Text in `language_name`.
    pub forms_italian: &'static str,
    /// Text in `language_name`.
    pub forms_czech: &'static str,
    /// Text in `language_name`.
    pub forms_portuguese: &'static str,
    /// Text in `language_name`.
    pub forms_japanese: &'static str,
    /// Text in `language_name`.
    pub forms_korean: &'static str,
    /// Text in `language_name`.
    pub forms_chinese_simplified: &'static str,
    /// Text in `language_name`.
    pub forms_chinese_traditional: &'static str,
    /// Text in `typed_mnemonic`. `{}`: `k + 1`, `language_name(lang)`.
    pub forms_word_is_not_on: &'static str,
    // ----- glance (faraday-core/src/glance.rs) -----
    /// Text in `RECEIVE_WARNING`.
    pub glance_back_up_before: &'static str,
    /// Text in `WAITING`.
    pub glance_waiting_for: &'static str,
    /// Text in `build`.
    pub glance_no_origin: &'static str,
    /// Text in `build`.
    pub glance_can_sign_passphrase: &'static str,
    /// Text in `build`.
    pub glance_can_sign_here: &'static str,
    /// Text in `build`.
    pub glance_in_this_vault_passphrase: &'static str,
    /// Text in `build`.
    pub glance_in_this_vault: &'static str,
    /// Text in `build`.
    pub glance_cosigner_xpub: &'static str,
    /// Text in `build`.
    pub glance_load_from_vault: &'static str,
    /// Text in `backup_nodes`. `{}`: `v + 1`.
    pub glance_vault: &'static str,
    /// Text in `backup_nodes`. `{}`: `p + 1`.
    pub glance_place: &'static str,
    /// Text in `backup_nodes`. `{}`: `j + 1`, `shape.keys`.
    pub glance_share_of: &'static str,
    /// Text in `backup_nodes`. `{}`: `vault_name(v)`.
    pub glance_stick: &'static str,
    /// Text in `backup_nodes`.
    pub glance_description: &'static str,
    /// Text in `backup_nodes`. `{}`: `number(i)`.
    pub glance_key_file: &'static str,
    /// Text in `backup_nodes`. `{}`: `inside.join(", ")`.
    pub glance_in_the_vault: &'static str,
    /// Text in `backup_nodes`. `{}`: `vault_name(v)`.
    pub glance_copy: &'static str,
    /// Text in `backup_nodes`. `{}`: `app.medium.cap()`.
    pub glance_of_files: &'static str,
    /// Text in `backup_nodes`.
    pub glance_watch_only_software: &'static str,
    /// Text in `backup_nodes`.
    pub glance_on_its_own_device: &'static str,
    /// Text in `backup_nodes`.
    pub glance_its_own_stick: &'static str,
    /// Text in `backup_nodes`. `{}`: `at.join(", ")`.
    pub glance_stick_at: &'static str,
    /// Text in `backup_nodes`.
    pub glance_not_here: &'static str,
    /// Text in `checked_text`.
    pub glance_checked: &'static str,
    /// Text in `checked_text`. `{}`: `d`.
    pub glance_checked_n: &'static str,
    /// Text in `alone_color`.
    pub glance_can_spend: &'static str,
    /// Text in `alone_color`.
    pub glance_nothing: &'static str,
    /// Text in `alone_color`.
    pub glance_sealed: &'static str,
    /// Text. `{}`: `k.number`, `k.fingerprint`.
    pub glance_key_2: &'static str,
    /// Text. `{}`: `vault`.
    pub glance_plan_in_locked: &'static str,
    /// Text in `rows`.
    pub glance_unlock: &'static str,
    /// Text in `rows`.
    pub glance_no_backup_plan_2: &'static str,
    /// Text in `rows`.
    pub glance_back_up: &'static str,
    /// Text in `rows`.
    pub glance_no_backup_plan: &'static str,
    /// Text in `draw_column_with`.
    pub glance_backup: &'static str,
    /// Text in `draw_column_with`.
    pub glance_keys: &'static str,
    /// Text in `draw_column_with`.
    pub glance_wallet: &'static str,
    /// Text in `thing_label`. `{}`: `number(i)`.
    pub glance_key_words: &'static str,
    /// Text in `thing_label`. `{}`: `number(i)`.
    pub glance_key_seedqr: &'static str,
    /// Text in `thing_label`. `{}`: `number(i)`.
    pub glance_passphrase: &'static str,
    /// Text in `thing_label`.
    pub glance_wallet_sheet: &'static str,
    /// Text in `thing_label`. `{}`: `j + 1`.
    pub glance_share: &'static str,
    /// Text in `thing_label`. `{}`: `v + 1`.
    pub glance_vault_copy: &'static str,
    /// Text in `thing_label`. `{}`: `number(i)`.
    pub glance_key_seed: &'static str,
    /// Text in `thing_label`. `{}`: `number(i)`.
    pub glance_key_seed_passphrase: &'static str,
    /// Text in `thing_label`.
    pub glance_the_description: &'static str,
    /// Text in `thing_label`. `{}`: `number(i)`.
    pub glance_key: &'static str,
    // ----- glance_sheet (faraday-core/src/glance_sheet.rs) -----
    /// Row in `view`.
    pub glance_sheet_open_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_open_in: &'static str,
    /// Row in `view`.
    pub glance_sheet_check_addresses: &'static str,
    /// Row in `view`.
    pub glance_sheet_the_first: &'static str,
    /// Row in `view`.
    pub glance_sheet_public_files: &'static str,
    /// Row in `view`.
    pub glance_sheet_for_the_software: &'static str,
    /// Text in `view`.
    pub glance_sheet_print_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_rename_the: &'static str,
    /// Text in `view`.
    pub glance_sheet_shape: &'static str,
    /// Text in `view`.
    pub glance_sheet_checksum: &'static str,
    /// Text in `view`.
    pub glance_sheet_where_2: &'static str,
    /// Text in `view`.
    pub glance_sheet_label: &'static str,
    /// Text in `view`.
    pub glance_sheet_last_checked_2: &'static str,
    /// Text in `view`.
    pub glance_sheet_its_seed: &'static str,
    /// Row in `view`.
    pub glance_sheet_show_xpub: &'static str,
    /// Row in `view`.
    pub glance_sheet_xpub_file: &'static str,
    /// Row in `view`.
    pub glance_sheet_explore: &'static str,
    /// Row in `view`.
    pub glance_sheet_where: &'static str,
    /// Row in `view`.
    pub glance_sheet_words: &'static str,
    /// Row in `view`.
    pub glance_sheet_seedqr: &'static str,
    /// Row in `view`.
    pub glance_sheet_seed_xor: &'static str,
    /// Row in `view`.
    pub glance_sheet_by_hand: &'static str,
    /// Text in `view`. `{}`: `n.name`.
    pub glance_sheet_into: &'static str,
    /// Row in `view`.
    pub glance_sheet_as_a_file: &'static str,
    /// Text in `view`.
    pub glance_sheet_make_another_backup: &'static str,
    /// Text in `view`.
    pub glance_sheet_check_a: &'static str,
    /// Text in `view`.
    pub glance_sheet_its_passphrase: &'static str,
    /// Row in `view`.
    pub glance_sheet_show_its: &'static str,
    /// Text in `view`.
    pub glance_sheet_name_its: &'static str,
    /// Text in `view`.
    pub glance_sheet_stick: &'static str,
    /// Text in `view`.
    pub glance_sheet_found: &'static str,
    /// Text in `view`.
    pub glance_sheet_not_yet: &'static str,
    /// Row in `view`.
    pub glance_sheet_open_a: &'static str,
    /// Text in `view`.
    pub glance_sheet_mark_checked: &'static str,
    /// Row in `view`.
    pub glance_sheet_what_goes: &'static str,
    /// Row in `view`.
    pub glance_sheet_the_envelope: &'static str,
    /// Row in `view`.
    pub glance_sheet_pdf_for: &'static str,
    /// Row in `view`.
    pub glance_sheet_add_here: &'static str,
    /// Text in `view`.
    pub glance_sheet_mark_lost: &'static str,
    /// Text in `view`.
    pub glance_sheet_mark_exposed: &'static str,
    /// Row in `view`.
    pub glance_sheet_remove_the: &'static str,
    /// Text in `view`.
    pub glance_sheet_rename: &'static str,
    /// Row in `view`.
    pub glance_sheet_save_its: &'static str,
    /// Row in `view`.
    pub glance_sheet_copy_to: &'static str,
    /// Row in `view`.
    pub glance_sheet_write_the_seed: &'static str,
    /// Row in `view`.
    pub glance_sheet_write_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_destroy_its: &'static str,
    /// Row in `view`.
    pub glance_sheet_show_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_public: &'static str,
    /// Text in `view`.
    pub glance_sheet_in_it: &'static str,
    /// Text in `view`. `{}`: `key.number`.
    pub glance_sheet_key_seed: &'static str,
    /// Text in `view`.
    pub glance_sheet_with_its: &'static str,
    /// Row in `view`.
    pub glance_sheet_the_description: &'static str,
    /// Text in `view`.
    pub glance_sheet_save_more: &'static str,
    /// Text in `view`.
    pub glance_sheet_kept_at: &'static str,
    /// Text in `view`.
    pub glance_sheet_seen_here: &'static str,
    /// Text in `view`.
    pub glance_sheet_last_checked: &'static str,
    /// Text in `view`.
    pub glance_sheet_marked: &'static str,
    /// Row in `view`.
    pub glance_sheet_load_this_key: &'static str,
    /// Row in `view`.
    pub glance_sheet_scan_my: &'static str,
    /// Row in `view`.
    pub glance_sheet_type_it_back: &'static str,
    /// Row in `view`.
    pub glance_sheet_check_this: &'static str,
    /// Row in `view`.
    pub glance_sheet_make_another: &'static str,
    /// Row in `view`.
    pub glance_sheet_write_it: &'static str,
    /// Row in `view`.
    pub glance_sheet_for_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_check_it: &'static str,
    /// Row in `view`.
    pub glance_sheet_by_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_print_it: &'static str,
    /// Row in `view`.
    pub glance_sheet_as_a_qr: &'static str,
    /// Row in `view`.
    pub glance_sheet_as_text: &'static str,
    /// Row in `view`.
    pub glance_sheet_another_copy: &'static str,
    /// Row in `view`.
    pub glance_sheet_elsewhere: &'static str,
    /// Row in `view`.
    pub glance_sheet_print: &'static str,
    /// Row in `view`.
    pub glance_sheet_pdfs_text: &'static str,
    /// Row in `view`.
    pub glance_sheet_keys_left: &'static str,
    /// Row in `view`.
    pub glance_sheet_copy_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_then_a: &'static str,
    /// Row in `view`.
    pub glance_sheet_write: &'static str,
    /// Row in `view`.
    pub glance_sheet_changed: &'static str,
    /// Row in `view`.
    pub glance_sheet_show_wallet: &'static str,
    /// Row in `view`.
    pub glance_sheet_the_descriptor: &'static str,
    /// Row in `view`.
    pub glance_sheet_move: &'static str,
    /// Row in `view`.
    pub glance_sheet_to_another: &'static str,
    /// Row in `view`.
    pub glance_sheet_remove_from: &'static str,
    /// Row in `view`.
    pub glance_sheet_i_destroyed: &'static str,
    /// Text in `view`.
    pub glance_sheet_lost: &'static str,
    /// Text in `view`.
    pub glance_sheet_exposed: &'static str,
    /// Text in `view`. `{}`: `label`.
    pub glance_sheet_move_to: &'static str,
    /// Row in `view`.
    pub glance_sheet_a_new: &'static str,
    /// Text in `view`. `{}`: `glance::thing_label(Some(&g), what)`.
    pub glance_sheet_another: &'static str,
    /// Text in `view`. `{}`: `key.number`.
    pub glance_sheet_key_words: &'static str,
    /// Text in `view`. `{}`: `key.number`.
    pub glance_sheet_key_seedqr: &'static str,
    /// Text in `view`. `{}`: `j + 1`.
    pub glance_sheet_share: &'static str,
    /// Row in `view`.
    pub glance_sheet_wallet: &'static str,
    /// Text in `view`. `{}`: `x.name`.
    pub glance_sheet_stick_n: &'static str,
    /// Row in `view`.
    pub glance_sheet_a_copy: &'static str,
    /// Text in `view`. `{}`: `node.name`.
    pub glance_sheet_add_at: &'static str,
    /// Text in `view`.
    pub glance_sheet_destroy: &'static str,
    /// Text in `view`.
    pub glance_sheet_confirm: &'static str,
    /// Text in `view`.
    pub glance_sheet_someone_can: &'static str,
    /// Text in `view`.
    pub glance_sheet_someone_could: &'static str,
    /// Row in `view`.
    pub glance_sheet_move_the: &'static str,
    /// Row in `view`.
    pub glance_sheet_new_wallet: &'static str,
    /// Text in `view`.
    pub glance_sheet_the_wallet: &'static str,
    /// Row in `view`.
    pub glance_sheet_back_up: &'static str,
    /// Text in `view`.
    pub glance_sheet_someone: &'static str,
    /// Text in `view`.
    pub glance_sheet_nothing_lost: &'static str,
    /// Row in `view`.
    pub glance_sheet_make_a: &'static str,
    /// Text in `view`.
    pub glance_sheet_nothing: &'static str,
    /// Row in `view`.
    pub glance_sheet_nowhere: &'static str,
    /// Text in `view`.
    pub glance_sheet_chosen: &'static str,
    /// Text in `view`.
    pub glance_sheet_destroyed: &'static str,
    /// Text in `view`. `{}`: `label`.
    pub glance_sheet_goes_to: &'static str,
    /// Text in `view`. `{}`: `node.name`.
    pub glance_sheet_remove: &'static str,
    /// Text in `view`. `{}`: `key.number`, `key.fingerprint`.
    pub glance_sheet_key: &'static str,
    /// Text in `view`.
    pub glance_sheet_holder: &'static str,
    /// Text in `view`.
    pub glance_sheet_name: &'static str,
    /// Text in `view`.
    pub glance_sheet_keep: &'static str,
    /// Text in `view`.
    pub glance_sheet_type_it: &'static str,
    /// Text in `view`.
    pub glance_sheet_passphrase: &'static str,
    /// Text in `view`. `{}`: `key.fingerprint`.
    pub glance_sheet_matches: &'static str,
    /// Text in `view`.
    pub glance_sheet_does_not: &'static str,
    /// Text in `view`.
    pub glance_sheet_check_2: &'static str,
    /// Text in `view`. `{}`: `key.number`.
    pub glance_sheet_passphrase_n: &'static str,
    /// Row in `view`.
    pub glance_sheet_show_it: &'static str,
    /// Row in `view`.
    pub glance_sheet_to_copy: &'static str,
    /// Text in `view`.
    pub glance_sheet_load_this: &'static str,
    /// Text in `view`.
    pub glance_sheet_check: &'static str,
    /// Text in `view`.
    pub glance_sheet_your_changes: &'static str,
    /// Text in `view`.
    pub glance_sheet_change: &'static str,
    /// Text in `draw`.
    pub glance_sheet_back: &'static str,
    /// Text in `draw`.
    pub glance_sheet_close: &'static str,
    // ----- gpg (faraday-core/src/gpg.rs) -----
    /// Error in `clock`.
    pub gpg_this_computer_s: &'static str,
    /// Error in `gpg_make`.
    pub gpg_a_key_needs_a_name: &'static str,
    /// Error in `gpg_make`.
    pub gpg_no_randomness_from: &'static str,
    /// Toast in `gpg_make`.
    pub gpg_the_key_is_in_the: &'static str,
    /// Text in `gpg_code`. `{}`: `faraday_pgp::fingerprint_text(&key.fi…`.
    pub gpg_fingerprint: &'static str,
    /// Error in `gpg_code`.
    pub gpg_no_gpg_key_is_selected: &'static str,
    /// Text in `gpg_code`.
    pub gpg_gpg_public_key: &'static str,
    /// Text in `gpg_code`.
    pub gpg_public_checks_signatures: &'static str,
    /// Text in `gpg_code`.
    pub gpg_gpg_revocation: &'static str,
    /// Text in `gpg_code`. `{}`: `who`.
    pub gpg_revokes: &'static str,
    /// Text in `gpg_code`.
    pub gpg_whoever_has_it: &'static str,
    /// Error in `gpg_code`.
    pub gpg_that_file_is_no: &'static str,
    /// Text in `gpg_code`. `{}`: `item.name`.
    pub gpg_gpg_signature: &'static str,
    /// Text in `gpg_code`. `{}`: `item.name`.
    pub gpg_signs: &'static str,
    /// Text in `gpg_code`. `{}`: `who`.
    pub gpg_by: &'static str,
    /// Error in `gpg_code`.
    pub gpg_not_a_gpg_file: &'static str,
    /// Toast in `gpg_renew`. `{}`: `name`, `self.medium.for_the()`.
    pub gpg_renewed_waits: &'static str,
    /// Text in `gpg_sign_ticked`. `{}`: `self.medium.for_the()`.
    pub gpg_1_signature_waits: &'static str,
    /// Text in `gpg_sign_ticked`. `{}`: `n`, `self.medium.for_the()`.
    pub gpg_signatures_wait: &'static str,
    /// Text in `shown`.
    pub gpg_never: &'static str,
    // ----- guide (faraday-core/src/guide.rs) -----
    /// Text in `spend`.
    pub guide_this_is_the_wallet: &'static str,
    /// Text in `spend`.
    pub guide_no_wallet_loaded: &'static str,
    /// Text in `spend`.
    pub guide_open_the_same: &'static str,
    /// Text in `spend`.
    pub guide_read_each_line: &'static str,
    /// Text in `spend`.
    pub guide_musig2_signs: &'static str,
    /// Text in `spend`.
    pub guide_this_wallet_can: &'static str,
    /// Text in `spend`.
    pub guide_this_is_the_transaction: &'static str,
    /// Text in `spend`. `{}`: `needed`, `if needed == 2 { EN.guide_one_other.t…`.
    pub guide_any_of_these: &'static str,
    /// Text in `spend`.
    pub guide_one_other: &'static str,
    /// Text in `spend`. `{}`: `needed - 1`.
    pub guide_others: &'static str,
    /// Text in `spend`.
    pub guide_this_wallet_has: &'static str,
    /// Text in `spend`. `{}`: `needed`, `if needed == 1 { EN.guide_of_these } …`.
    pub guide_keys_must_sign: &'static str,
    /// Text in `spend`.
    pub guide_of_these: &'static str,
    /// Text in `spend`.
    pub guide_of_the: &'static str,
    /// Text in `spend`.
    pub guide_signed_each_signature: &'static str,
    /// Text in `spend`. `{}`: `m.noun()`.
    pub guide_signing_as_a: &'static str,
    /// Text in `spend`.
    pub guide_round_two_this: &'static str,
    /// Text in `spend`. `{}`: `needed`, `if needed == 1 { EN.guide_signature }…`.
    pub guide_this_adds_the: &'static str,
    /// Text in `spend`.
    pub guide_signature: &'static str,
    /// Text in `spend`.
    pub guide_signatures: &'static str,
    /// Text in `spend`. `{}`: `m.a()`.
    pub guide_signing_uses: &'static str,
    /// Text in `spend`. `{}`: `m.a()`, `m.a()`.
    pub guide_get_the_other: &'static str,
    /// Text in `spend`.
    pub guide_there_are_two: &'static str,
    /// Text in `spend`. `{}`: `m.a()`.
    pub guide_keep_the_carry: &'static str,
    /// Text in `spend`.
    pub guide_the_transaction: &'static str,
    /// Text in `backup`. `{}`: `medium.for_the()`.
    pub guide_print_the_blank: &'static str,
    /// Text in `backup`.
    pub guide_none_of_this: &'static str,
    /// Text in `backup`.
    pub guide_copy_each_seed: &'static str,
    /// Text in `backup`. `{}`: `lead`, `further`.
    pub guide_a_printer_is: &'static str,
    /// Text in `backup`, after a space. `{}`: `medium.for_the()`, `medium.noun()`.
    pub guide_paper_comes_first: &'static str,
    /// Text in `backup`. `{}`: `medium.for_the()`.
    pub guide_this_half_is: &'static str,
    /// Text in `backup`. `{}`: `keys`, `m.saturating_sub(1)`, `m`, `m`, `medium.for_the()`.
    pub guide_instead_of_one: &'static str,
    /// Text in `backup`.
    pub guide_one_envelope: &'static str,
    /// Text in `message`.
    pub guide_a_signed_message: &'static str,
    /// Text in `message`.
    pub guide_type_the_message: &'static str,
    /// Text in `message`.
    pub guide_bip_322_is_the: &'static str,
    /// Text in `message`.
    pub guide_signing_moves: &'static str,
    /// Text in `create`. `{}`: `m`, `keys`.
    pub guide_any_of_shares: &'static str,
    /// Text in `create`. `{}`: `m`, `keys - m`.
    pub guide_choose_loaded: &'static str,
    /// Text in `create`.
    pub guide_choose_how_the: &'static str,
    /// Text in `create`. `{}`: `m`, `keys`, `m`.
    pub guide_of_every_spend: &'static str,
    /// Text in `create`. `{}`: `medium.a()`.
    pub guide_fill_each_slot: &'static str,
    /// Text in `create`.
    pub guide_choose_the_key: &'static str,
    /// Text in `create`.
    pub guide_load_the_wallet: &'static str,
    /// Text in `create`.
    pub guide_the_plan_says: &'static str,
    /// Text in `restore`.
    pub guide_the_kind_of_wallet: &'static str,
    /// Text in `restore`.
    pub guide_how_many_keys: &'static str,
    /// Text in `restore`.
    pub guide_the_descriptor: &'static str,
    /// Text in `restore`.
    pub guide_one_slot_per: &'static str,
    /// Text in `restore`.
    pub guide_compare_these: &'static str,
    /// Text in `restore`.
    pub guide_the_wallet_is: &'static str,
    /// Text in `keygen`.
    pub guide_a_slip_39_key: &'static str,
    /// Text in `keygen`.
    pub guide_write_each_share: &'static str,
    /// Text in `keygen`.
    pub guide_answer_from_your_papers: &'static str,
    /// Text in `keygen`.
    pub guide_12_words_carry: &'static str,
    /// Text in `keygen`.
    pub guide_randomness_you_make: &'static str,
    /// Text in `keygen`.
    pub guide_randomness_you: &'static str,
    /// Text in `keygen`.
    pub guide_roll_one_die_at_a_time: &'static str,
    /// Text in `keygen`.
    pub guide_roll_one_die_at_a: &'static str,
    /// Text in `keygen`. `{}`: `k.last_bits()`, `k.words / 3`.
    pub guide_roll_one_die_at: &'static str,
    /// Text in `keygen`.
    pub guide_flip_a_coin_and_enter: &'static str,
    /// Text in `keygen`.
    pub guide_roll_one_die: &'static str,
    /// Text in `keygen`.
    pub guide_flip_a_coin_and: &'static str,
    /// Text in `keygen`.
    pub guide_shuffle_a_full: &'static str,
    /// Text in `keygen`.
    pub guide_type_the_hex: &'static str,
    /// Text in `keygen`.
    pub guide_point_the_camera: &'static str,
    /// Text in `keygen`.
    pub guide_this_device_asks: &'static str,
    /// Text in `keygen`.
    pub guide_lock_in_adds_the: &'static str,
    /// Text in `keygen`.
    pub guide_lock_in_adds: &'static str,
    /// Text in `keygen`.
    pub guide_these_counts: &'static str,
    /// Text in `keygen`.
    pub guide_write_the_words: &'static str,
    /// Text in `keygen`.
    pub guide_answer_from_your: &'static str,
    // ----- held (faraday-core/src/held.rs) -----
    /// Toast in `backup_settle`. `{}`: `names.join(", ")`.
    pub held_signs_from_now: &'static str,
    /// Text in `backup_extras`. `{}`: `name`, `seed`.
    pub held_holds_seed: &'static str,
    // ----- inbox (faraday-core/src/inbox.rs) -----
    /// Text in `name`. `{}`: `ks.len()`.
    pub inbox_slip_39_shares_2: &'static str,
    /// Text in `name`. `{}`: `ks.len()`.
    pub inbox_codex32_shares: &'static str,
    /// Text in `name`.
    pub inbox_loaded: &'static str,
    /// Text in `found_in`.
    pub inbox_slip_39_seed: &'static str,
    /// Text in `found_in`. `{}`: `ks.len()`, `need`.
    pub inbox_slip_39_shares: &'static str,
    /// Text in `found_in`.
    pub inbox_codex32_seed: &'static str,
    /// Text in `found_in`. `{}`: `ks.len()`, `need`.
    pub inbox_codex32_shares_inline: &'static str,
    /// Text in `found_in`. `{}`: `item.name`.
    pub inbox_opensigner_backup: &'static str,
    /// Text in `found_in`.
    pub inbox_restored_wallet: &'static str,
    /// Text in `found_in`. `{}`: `m.have.len()`, `m.n`.
    pub inbox_split_sheets_of: &'static str,
    /// Text in `found_in`. `{}`: `e`.
    pub inbox_split_sheets: &'static str,
    /// Text in `inbox_load_named`.
    pub inbox_its_shares: &'static str,
    /// Text in `xpub_make`. `{}`: `fp.map(fp_text).unwrap_or_else(|| EN.…`, `crate::family::kind_name(kind)`.
    pub inbox_watch_only: &'static str,
    /// Text in `xpub_make`.
    pub inbox_xpub: &'static str,
    /// Text in `xpub_make`.
    pub inbox_an_xpub_in_files: &'static str,
    /// Error in `potential_make`. `{}`: `self.medium.noun()`.
    pub inbox_remove_the_first: &'static str,
    /// Text in `potential_make`. `{}`: `seed.label`.
    pub inbox_with_passphrase: &'static str,
    /// Text in `potential_make`.
    pub inbox_this_seed_was: &'static str,
    /// Toast in `potential_make`. `{}`: `fp_text(fp)`.
    pub inbox_loaded_with_its: &'static str,
    /// Text in `potential_make`.
    pub inbox_a_seed_in_files: &'static str,
    /// Toast in `potential_make`. `{}`: `name`.
    pub inbox_is_a_wallet: &'static str,
    /// Text in `backup_unseal`. `{}`: `name`.
    pub inbox_loaded_n: &'static str,
    /// Error in `backup_unseal`.
    pub inbox_this_backup_holds: &'static str,
    /// Error in `backup_unseal`.
    pub inbox_not_this_backup: &'static str,
    // ----- inputs (faraday-core/src/inputs.rs) -----
    /// Toast in `input_believe`. `{}`: `d.name`.
    pub inputs_is_in_use: &'static str,
    /// Toast in `input_typed`. `{}`: `name`.
    pub inputs_wrong_code: &'static str,
    /// Toast in `input_typed` and `input_new`.
    pub inputs_new_keyboards_ignored: &'static str,
    // ----- keygen (faraday-core/src/keygen.rs) -----
    /// Text in `source_name`.
    pub keygen_seed_xor: &'static str,
    /// Text in `procedure_name`.
    pub keygen_words_chosen: &'static str,
    /// Error in `deal`.
    pub keygen_no_secret_yet: &'static str,
    /// Text in `deal`. `{}`: `e`.
    pub keygen_the_shares_could: &'static str,
    /// Text in `caution_lines`.
    pub keygen_a_long_run_of: &'static str,
    /// Text in `caution_lines`.
    pub keygen_far_from_even: &'static str,
    /// Text in `caution_lines`.
    pub keygen_a_long_run_counting: &'static str,
    /// Text in `roll`.
    pub keygen_a_5_or_a_6_is: &'static str,
    /// Note in `die_flip`.
    pub keygen_a_die_has_six: &'static str,
    /// Note in `type_char`.
    pub keygen_no_more_are: &'static str,
    /// Text in `card`.
    pub keygen_that_card_is: &'static str,
    /// Note in `take_frame`.
    pub keygen_no_picture_from: &'static str,
    /// Note in `take_frame`.
    pub keygen_too_little_variation: &'static str,
    /// Note in `finish_entry`. `{}`: `have`, `need`.
    pub keygen_of: &'static str,
    /// Toast in `keygen_open`. `{}`: `self.medium.noun()`.
    pub keygen_remove_the_first: &'static str,
    /// Text in `keygen_act`.
    pub keygen_choose_two_sources: &'static str,
    /// Text in `keygen_act`.
    pub keygen_choose_a_source: &'static str,
    /// Note in `keygen_act`.
    pub keygen_no_randomness: &'static str,
    /// Note in `keygen_lock`.
    pub keygen_the_two_passphrases: &'static str,
    /// Text in `keygen_lock`. `{}`: `self.new_keys + 1`.
    pub keygen_new_key_n: &'static str,
    /// Text in `keygen_done`. `{}`: `crate::wallet::fp_text(osk_bip::keys:…`, `if k.slip39 { FEN.keygen_shares_inline } els…`.
    pub keygen_key_added_back: &'static str,
    /// Text in `keygen_done`.
    pub keygen_shares_inline: &'static str,
    /// Text in `keygen_done`.
    pub keygen_words_inline: &'static str,
    // ----- keygen (faraday-core/src/keygen_screen.rs) -----
    /// Text in `TITLES`.
    pub keygen_check: &'static str,
    /// Text in `TITLES`.
    pub keygen_words: &'static str,
    /// Text in `draw`.
    pub keygen_make_a_new_wallet: &'static str,
    /// Text in `draw`.
    pub keygen_add_a_key: &'static str,
    /// Text in `draw`.
    pub keygen_tools: &'static str,
    /// Text in `draw`.
    pub keygen_back: &'static str,
    /// Text in `draw`.
    pub keygen_new_key: &'static str,
    /// Text in `entries_title`.
    pub keygen_flips: &'static str,
    /// Text in `entries_title`.
    pub keygen_cards: &'static str,
    /// Text in `entries_title`.
    pub keygen_pictures: &'static str,
    /// Text in `entries_title`.
    pub keygen_generator: &'static str,
    /// Text in `entries_title`.
    pub keygen_entries: &'static str,
    /// Text in `entries_title`. `{}`: `what`, `(k.mix_at + 1).min(n)`, `n`.
    pub keygen_source_of: &'static str,
    /// Text in `asks`. `{}`: `k.dice_needed()`.
    pub keygen_rolls_of_a_six: &'static str,
    /// Text in `asks`. `{}`: `CoinFlips::needed(st)`.
    pub keygen_flips_or_rolls: &'static str,
    /// Text in `asks`. `{}`: `CardDraws::needed(st)`.
    pub keygen_cards_from_a: &'static str,
    /// Text in `asks`. `{}`: `st.hex_digits()`.
    pub keygen_digits_made: &'static str,
    /// Text in `asks`. `{}`: `CameraNoise::needed(st)`.
    pub keygen_pictures_n: &'static str,
    /// Text in `asks`.
    pub keygen_two_or_more: &'static str,
    /// Text in `asks`. `{}`: `EN.create_device_os`.
    pub keygen_getrandom_2: &'static str,
    /// Text in `summary`. `{}`: `k.slip_m`, `k.slip_n`, `k.words`.
    pub keygen_slip_39_of_shares: &'static str,
    /// Text in `summary`. `{}`: `source_name(Source::Coins)`.
    pub keygen_read_from_a: &'static str,
    /// Text in `summary`.
    pub keygen_not_chosen: &'static str,
    /// Text in `summary`.
    pub keygen_complete: &'static str,
    /// Text in `summary`.
    pub keygen_not_locked_in: &'static str,
    /// Text in `summary`. `{}`: `fp`.
    pub keygen_locked_in: &'static str,
    /// Text in `summary`. `{}`: `fp`.
    pub keygen_passphrase_set: &'static str,
    /// Text in `summary`. `{}`: `fp`.
    pub keygen_passphrase_not: &'static str,
    /// Text in `summary`. `{}`: `k.mixed.len()`.
    pub keygen_sources_combined: &'static str,
    /// Text in `summary`.
    pub keygen_no_cautions: &'static str,
    /// Text in `summary`.
    pub keygen_1_caution: &'static str,
    /// Text in `summary`. `{}`: `n`.
    pub keygen_cautions: &'static str,
    /// Text in `summary`. `{}`: `n`.
    pub keygen_shares_n: &'static str,
    /// Text in `summary`.
    pub keygen_not_yet: &'static str,
    /// Label in `read_only`.
    pub keygen_length: &'static str,
    /// Label in `read_only`.
    pub keygen_randomness: &'static str,
    /// Line in `verdict`.
    pub keygen_fair_no_cautions: &'static str,
    /// Label in `key_card`.
    pub keygen_passphrase_again: &'static str,
    /// Text in `key_card`.
    pub keygen_lock_in: &'static str,
    /// Line in `key_card`.
    pub keygen_enter_locks: &'static str,
    /// Label in `key_card`.
    pub keygen_passphrase: &'static str,
    /// Text in `key_card`.
    pub keygen_set: &'static str,
    /// Text in `key_card`.
    pub keygen_not_set: &'static str,
    /// Label in `key_card`.
    pub keygen_first_address: &'static str,
    /// Label in `key_card`.
    pub keygen_key: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_fewer_shares: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_each_share_states: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_the_threshold: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_the_split_s: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_a_share_cannot: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_shares_restore: &'static str,
    /// Text in `SLIP39_FACTS`.
    pub keygen_a_share_is_20: &'static str,
    /// Text in `length`.
    pub keygen_bip_39_words: &'static str,
    /// Text in `length`.
    pub keygen_slip_39_shares: &'static str,
    /// Text in `length`.
    pub keygen_shares_needed: &'static str,
    /// Text in `length`.
    pub keygen_shares_dealt: &'static str,
    /// Text in `length`. `{}`: `k.slip_m`, `k.slip_n`.
    pub keygen_any_of_the_shares: &'static str,
    /// Line in `length`.
    pub keygen_what_shares: &'static str,
    /// Text in `length_counts`. `{}`: `n`.
    pub keygen_words_n: &'static str,
    /// Line in `length_counts`.
    pub keygen_a_threshold: &'static str,
    /// Text in `way_name`. `{}`: `source_name(Source::Dice)`.
    pub keygen_flip_mode: &'static str,
    /// Text in `way_line`. `{}`: `CoinFlips::needed(st)`.
    pub keygen_rolls_read_as: &'static str,
    /// Text in `group_head`.
    pub keygen_your_own_entropy: &'static str,
    /// Text in `group_head`.
    pub keygen_your_entropy: &'static str,
    /// Text in `group_head`.
    pub keygen_made_by_this: &'static str,
    /// Chip in `group_head`.
    pub keygen_recommended: &'static str,
    /// Text in `entries`.
    pub keygen_rolls_inline: &'static str,
    /// Text in `entries`.
    pub keygen_flips_inline: &'static str,
    /// Text in `entries`.
    pub keygen_cards_inline: &'static str,
    /// Text in `entries`.
    pub keygen_digits_inline: &'static str,
    /// Text in `entries`.
    pub keygen_pictures_inline: &'static str,
    /// Text in `entries`. `{}`: `have`, `need`, `unit`.
    pub keygen_of_2: &'static str,
    /// Text in `entries`. `{}`: `last`.
    pub keygen_last: &'static str,
    /// Text in `entries`. `{}`: `k.cards.deck()`, `k.cards.in_deck()`.
    pub keygen_deck_drawn_from: &'static str,
    /// Text in `entries`. `{}`: `st.distinct`.
    pub keygen_luma_values: &'static str,
    /// Text in `entries`.
    pub keygen_camera_on: &'static str,
    /// Text in `entries`.
    pub keygen_waiting_for: &'static str,
    /// Line in `entries`.
    pub keygen_no_need_to_write: &'static str,
    /// Section label in `entries`.
    pub keygen_randomness_check: &'static str,
    /// Text in `entries`.
    pub keygen_next_source: &'static str,
    /// Text in `entries`.
    pub keygen_make_the_words: &'static str,
    /// Text in `entries`.
    pub keygen_undo: &'static str,
    /// Text in `entries`.
    pub keygen_clear: &'static str,
    /// Text in `entries`.
    pub keygen_type_1_to_6: &'static str,
    /// Text in `entries`.
    pub keygen_type_h_or_t: &'static str,
    /// Text in `entries`.
    pub keygen_type_the_rank: &'static str,
    /// Text in `entries`.
    pub keygen_type_09_and: &'static str,
    /// Text in `entries`.
    pub keygen_space_takes: &'static str,
    /// Text in `entries`.
    pub keygen_enter_continues: &'static str,
    /// Text in `entry_style`.
    pub keygen_coin: &'static str,
    /// Text in `entry_style`.
    pub keygen_die: &'static str,
    /// Text in `entry_style`.
    pub keygen_typed: &'static str,
    /// Text in `entry_style`.
    pub keygen_buttons: &'static str,
    /// Text in `entry_style`.
    pub keygen_each_roll_is: &'static str,
    /// Text in `entry_style`.
    pub keygen_heads_is_1_tails: &'static str,
    /// Text in `typed_box`.
    pub keygen_h_or_1_for_heads: &'static str,
    /// Text in `typed_box`.
    pub keygen_faces_1_to_6: &'static str,
    /// Text in `reveal`. `{}`: `part`.
    pub keygen_of_11_bits: &'static str,
    /// Row in `reveal`.
    pub keygen_from_the_checksum: &'static str,
    /// Text in `reveal`. `{}`: `checksum`, `last + 1`.
    pub keygen_the_last_bits: &'static str,
    /// Section label in `rolled_words`.
    pub keygen_words_so_far: &'static str,
    /// Line in `rolled_words`.
    pub keygen_rolls_1_to_5: &'static str,
    /// Line in `rolled_words`.
    pub keygen_checksum: &'static str,
    /// Text in `rolled_words`. `{}`: `i`, `i + 1`.
    pub keygen_word: &'static str,
    /// Text in `rolled_words`. `{}`: `i`, `i + 1`.
    pub keygen_word_of_the: &'static str,
    /// Note in `rolled_words`. `{}`: `got.len()`, `per`.
    pub keygen_of_rolls: &'static str,
    /// Row in `rolled_words`.
    pub keygen_the_checksum: &'static str,
    /// Text in `device_rows`.
    pub keygen_result: &'static str,
    /// Text in `check`.
    pub keygen_continue: &'static str,
    /// Text in `again_label`.
    pub keygen_start_again: &'static str,
    /// Label in `check_rows`.
    pub keygen_rolls: &'static str,
    /// Label in `check_rows`.
    pub keygen_faces: &'static str,
    /// Label in `check_rows`.
    pub keygen_suits: &'static str,
    /// Label in `check_rows`.
    pub keygen_digits: &'static str,
    /// Text in `check_rows`. `{}`: `hex`.
    pub keygen_sha_256: &'static str,
    /// Label in `share_words`.
    pub keygen_share: &'static str,
    /// Text in `share_words`. `{}`: `k.share_at + 1`, `k.shares.len()`, `k.slip_m`.
    pub keygen_of_needed: &'static str,
    /// Button in `share_words`.
    pub keygen_previous_share: &'static str,
    /// Button in `share_words`.
    pub keygen_next_share: &'static str,
    /// Button in `share_words`.
    pub keygen_i_wrote_them_all: &'static str,
    /// Line in `words`.
    pub keygen_not_made_yet: &'static str,
    /// Label in `words`.
    pub keygen_source: &'static str,
    /// Text in `words`.
    pub keygen_hide_words: &'static str,
    /// Text in `words`.
    pub keygen_show_words: &'static str,
    /// Text in `words`.
    pub keygen_i_wrote_them: &'static str,
    /// Line in `quiz`.
    pub keygen_show_the_words_first: &'static str,
    /// Label in `quiz`.
    pub keygen_fingerprint: &'static str,
    /// Text in `quiz`. `{}`: `k.shares.len()`.
    pub keygen_all_shares_right: &'static str,
    /// Text in `quiz`. `{}`: `n`.
    pub keygen_all_words_right: &'static str,
    /// Text in `quiz`.
    pub keygen_skipped_the: &'static str,
    /// Label in `quiz`.
    pub keygen_quiz: &'static str,
    /// Text in `quiz`.
    pub keygen_done: &'static str,
    /// Text in `quiz`.
    pub keygen_use_this_key: &'static str,
    /// Text in `quiz`.
    pub keygen_add_key: &'static str,
    /// Text in `quiz`. `{}`: `k.quiz_share + 1`, `k.shares.len()`, `done`, `n`.
    pub keygen_share_of_of: &'static str,
    /// Text in `quiz`. `{}`: `done`, `n`.
    pub keygen_of_right: &'static str,
    /// Text in `quiz`. `{}`: `q.position() + 1`.
    pub keygen_which_is_word: &'static str,
    /// Line in `quiz`.
    pub keygen_not_that_one: &'static str,
    /// Button in `quiz`.
    pub keygen_ask_it_again: &'static str,
    /// Button in `quiz`.
    pub keygen_show_the_words: &'static str,
    /// Line in `quiz`.
    pub keygen_without_the: &'static str,
    /// Text in `quiz`.
    pub keygen_skip_and_add: &'static str,
    /// Button in `quiz`.
    pub keygen_skip_the_quiz: &'static str,
    /// Text in `quiz`.
    pub keygen_type_1_to_4_to: &'static str,
    /// Text in `quiz`.
    pub keygen_type_1_to_4: &'static str,
    // ----- name (faraday-core/src/lib.rs) -----
    /// Text in `name`.
    pub name_faraday_hub_app: &'static str,
    /// The hub app's name in `--version` before F16, still recognised.
    pub name_faraday_hub_app_old: &'static str,
    /// The hub app's name in `--version` as "Faraday Hub", still recognised.
    pub name_faraday_hub_app_old2: &'static str,
    /// Text in `name`.
    pub name_faraday_desktop: &'static str,
    /// The desktop app's name in `--version` before F16, still recognised.
    pub name_faraday_desktop_old: &'static str,
    // ----- version_label (faraday-core/src/lib.rs) -----
    /// Text in `version_label`. `{}`: `VERSION`.
    pub version_label_local: &'static str,
    // ----- stick_kind (faraday-core/src/lib.rs) -----
    /// Text in `stick_kind`.
    pub stick_kind_psbt: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_text: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_transaction: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_sheet_for: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_vault: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_efi_image: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_threshold: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_opensigner: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_qr_codes: &'static str,
    /// Text in `stick_kind`.
    pub stick_kind_file: &'static str,
    // ----- stick_keys (faraday-core/src/lib.rs) -----
    /// Text in `stick_keys`. `{}`: `m.noun()`.
    pub stick_keys_keys_load: &'static str,
    // ----- of (faraday-core/src/lib.rs) -----
    /// Text in `of`.
    pub of_ur_crypto_psbt_one: &'static str,
    /// Text in `of`. `{}`: `frames.len()`.
    pub of_ur_crypto_psbt_frames: &'static str,
    /// Text in `of`.
    pub of_text_one_code: &'static str,
    /// Text in `of`.
    pub of_ur_crypto_account: &'static str,
    /// Text in `of`.
    pub of_key_one_code: &'static str,
    /// Text in `of`.
    pub of_file_ur_bytes: &'static str,
    // ----- parts_frames (faraday-core/src/lib.rs) -----
    /// Text in `parts_frames`.
    pub parts_frames_text_one: &'static str,
    /// Text in `parts_frames`. `{}`: `frames.len()`.
    pub parts_frames_numbered: &'static str,
    // ----- bbqr_frames (faraday-core/src/lib.rs) -----
    /// Error in `bbqr_frames`.
    pub bbqr_frames_too_large: &'static str,
    /// Text in `bbqr_frames`. `{}`: `t`.
    pub bbqr_frames_bbqr_one: &'static str,
    /// Text in `bbqr_frames`. `{}`: `t`, `frames.len()`.
    pub bbqr_frames_bbqr_frames: &'static str,
    // ----- key_label (faraday-core/src/lib.rs) -----
    /// Text in `key_label`. `{}`: `fp`, `kind`, `p`.
    pub key_label_key_2: &'static str,
    /// Text in `key_label`. `{}`: `fp`, `kind`.
    pub key_label_key: &'static str,
    // ----- ur_bytes_frames (faraday-core/src/lib.rs) -----
    /// Text in `ur_bytes_frames`. `{}`: `what`.
    pub ur_bytes_frames_one: &'static str,
    /// Text in `ur_bytes_frames`. `{}`: `what`, `frames.len()`.
    pub ur_bytes_frames_frames: &'static str,
    // ----- part_note (faraday-core/src/lib.rs) -----
    /// Text in `part_note`. `{}`: `what`, `have`, `total`, `list.join(", ")`, `more`.
    pub part_note_of_missing: &'static str,
    // ----- secret_text (faraday-core/src/lib.rs) -----
    /// Text in `secret_text`.
    pub secret_text_those_are: &'static str,
    /// Text in `secret_text`.
    pub secret_text_that_is: &'static str,
    // ----- what_instead_of_a_seed (faraday-core/src/lib.rs) -----
    /// Text in `what_instead_of_a_seed`.
    pub what_instead_of_a_seed_that_is_a: &'static str,
    /// Text in `what_instead_of_a_seed`.
    pub what_instead_of_a_seed_that_is: &'static str,
    /// Text in `what_instead_of_a_seed`.
    pub what_instead_of_a_seed_that: &'static str,
    // ----- app (faraday-core/src/lib.rs) -----
    /// Text in `NOT_A_KEY`.
    pub app_not_a_key_of_this: &'static str,
    // ----- heading (faraday-core/src/lib.rs) -----
    /// Text in `heading`. `{}`: `self.label`, `vaults::time_of_day(vaults::local_sec…`.
    pub heading_written_to_at: &'static str,
    /// Text in `heading`. `{}`: `self.label`.
    pub heading_written_to: &'static str,
    // ----- storage_event (faraday-core/src/lib.rs) -----
    /// Text in `storage_event`. `{}`: `name`, `self.medium.noun()`.
    pub storage_event_copied_a: &'static str,
    /// Text in `storage_event`. `{}`: `name`.
    pub storage_event_copied: &'static str,
    /// Text in `storage_event`. `{}`: `name`.
    pub storage_event_wrote_read: &'static str,
    /// Text in `storage_event`. `{}`: `name`, `wrote_as`.
    pub storage_event_wrote: &'static str,
    /// Text in `storage_event`.
    pub storage_event_it_stays: &'static str,
    /// Text in `storage_event`. `{}`: `self.medium.for_the()`.
    pub storage_event_it_still: &'static str,
    /// Text in `storage_event`. `{}`: `name`, `reason`, `stays`.
    pub storage_event_not_written: &'static str,
    /// Toast in `storage_event`. `{}`: `path`.
    pub storage_event_saved: &'static str,
    /// Text in `storage_event`.
    pub storage_event_too_fine: &'static str,
    /// Toast in `storage_event`. `{}`: `reason`.
    pub storage_event_not_printed: &'static str,
    // ----- sticks_changed (faraday-core/src/lib.rs) -----
    /// Text in `sticks_changed`. `{}`: `self.medium.cap()`.
    pub sticks_changed_removed_2: &'static str,
    /// Text in `sticks_changed`. `{}`: `self.medium.cap()`, `loaded`, `if loaded == 1 { EN.sticks_changed_ke…`.
    pub sticks_changed_removed: &'static str,
    /// Text in `sticks_changed`.
    pub sticks_changed_key: &'static str,
    /// Text in `sticks_changed`.
    pub sticks_changed_keys: &'static str,
    // ----- toast_out (faraday-core/src/lib.rs) -----
    /// Text in `toast_out`. `{}`: `what`, `self.medium.for_the()`.
    pub toast_out_waits: &'static str,
    // ----- pull_what (faraday-core/src/lib.rs) -----
    /// Text in `pull_what`.
    pub pull_what_load_the: &'static str,
    /// Text in `pull_what`.
    pub pull_what_open_the: &'static str,
    /// Text in `pull_what`.
    pub pull_what_unlock: &'static str,
    /// Text in `pull_what`.
    pub pull_what_make_a_key: &'static str,
    /// Text in `pull_what`.
    pub pull_what_add_a_key: &'static str,
    /// Text in `pull_what`.
    pub pull_what_scan: &'static str,
    // ----- visit_then (faraday-core/src/lib.rs) -----
    /// Text in `visit_then`. `{}`: `self.back_name(s)`.
    pub visit_then_then: &'static str,
    /// Text in `visit_then`.
    pub visit_then_then_unlock: &'static str,
    /// Text in `visit_then`.
    pub visit_then_then_home: &'static str,
    // ----- start_spend (faraday-core/src/lib.rs) -----
    /// Toast in `start_spend`.
    pub start_spend_the_carry: &'static str,
    /// Toast in `start_spend`.
    pub start_spend_not_a_psbt: &'static str,
    // ----- session_stage (faraday-core/src/lib.rs) -----
    /// Text in `session_stage`.
    pub session_stage_bring: &'static str,
    /// Text in `session_stage`.
    pub session_stage_write: &'static str,
    /// Text in `session_stage`.
    pub session_stage_work: &'static str,
    /// Text in `session_stage`.
    pub session_stage_open: &'static str,
    // ----- act (faraday-core/src/lib.rs) -----
    /// Text in `act`. `{}`: `w`, `if w == 1 { EN.act_wallet } else { EN…`, `k`, `if k == 1 { EN.act_seed } else { EN.a…`.
    pub act_and_loaded: &'static str,
    /// Text in `act`.
    pub act_wallet: &'static str,
    /// Text in `act`.
    pub act_wallets: &'static str,
    /// Text in `act`.
    pub act_seed: &'static str,
    /// Text in `act`.
    pub act_seeds: &'static str,
    /// Toast in `act`. `{}`: `self.medium.noun()`.
    pub act_remove_the_first: &'static str,
    /// Text in `act`. `{}`: `s.spend.source`.
    pub act_finished_from: &'static str,
    /// Text in `act`.
    pub act_transaction: &'static str,
    /// Text in `act`. `{}`: `self.medium.a()`.
    pub act_plug_in_the_visit: &'static str,
    /// Toast in `act`.
    pub act_bip_129_covers: &'static str,
    /// Note in `act`.
    pub act_carrying_on_with: &'static str,
    /// Error in `act`. `{}`: `m.have.len()`, `m.n`.
    pub act_of_keys_in_hand: &'static str,
    /// Text in `act`.
    pub act_restored_wallet: &'static str,
    /// Text in `act`.
    pub act_its_shares: &'static str,
    /// Toast in `act`. `{}`: `name`.
    pub act_restored_from_its: &'static str,
    /// Toast in `act`.
    pub act_not_every_key_is: &'static str,
    /// Text in `act`.
    pub act_signed_psbt: &'static str,
    /// Text in `act`.
    pub act_psbt_for_the_next: &'static str,
    /// Text in `act`.
    pub act_the_secret_nonce: &'static str,
    /// Text in `act`.
    pub act_whoever_has_it: &'static str,
    // ----- backup_kept (faraday-core/src/lib.rs) -----
    /// Text in `backup_kept`.
    pub backup_kept_not_checked: &'static str,
    /// Text in `backup_kept`. `{}`: `self.vaults.open[v].name`.
    pub backup_kept_wallet_in: &'static str,
    /// Text in `backup_kept`.
    pub backup_kept_wallet: &'static str,
    /// Text in `backup_kept`. `{}`: `self.vaults.open[v].name`, `if with { " with its passphrase" } el…`.
    pub backup_kept_in: &'static str,
    /// Text in `backup_kept`, after a space.
    pub backup_with_its_passphrase: &'static str,
    /// Text in `backup_kept`.
    pub backup_kept_not_in: &'static str,
    /// Text in `backup_kept`.
    pub backup_kept_paper_copy: &'static str,
    /// Text in `backup_kept`.
    pub backup_kept_paper_not: &'static str,
    /// Text in `backup_kept`. `{}`: `self.medium.for_the()`.
    pub backup_kept_file_unprotected: &'static str,
    /// Text in `backup_kept`.
    pub backup_kept_not_here: &'static str,
    /// Text in `backup_kept`. `{}`: `id + 1`.
    pub backup_kept_share: &'static str,
    // ----- backup_seed_list_of (faraday-core/src/lib.rs) -----
    /// Text in `backup_seed_list_of`. `{}`: `id + 1`.
    pub backup_seed_list_of_share: &'static str,
    // ----- place_name (faraday-core/src/lib.rs) -----
    /// Text in `place_name`. `{}`: `p + 1`.
    pub place_name_place: &'static str,
    // ----- plan_store (faraday-core/src/lib.rs) -----
    /// Text in `plan_store`. `{}`: `p + 1`.
    pub plan_store_place: &'static str,
    /// Text in `plan_store`. `{}`: `self.medium.cap()`.
    pub plan_store_of_files: &'static str,
    /// Text in `plan_store`.
    pub plan_store_watch_only: &'static str,
    /// Text in `plan_store`.
    pub plan_store_on_its_own: &'static str,
    /// Text in `plan_store`. `{}`: `name`.
    pub plan_store_the_plan: &'static str,
    // ----- public_pictures (faraday-core/src/lib.rs) -----
    /// Text in `public_pictures`. `{}`: `j + 1`, `k`.
    pub public_pictures_part: &'static str,
    /// Text in `public_pictures`.
    pub public_pictures_key: &'static str,
    /// Text in `public_pictures`.
    pub public_pictures_keys: &'static str,
    /// Text in `public_pictures`. `{}`: `sum`.
    pub public_pictures_descriptor: &'static str,
    /// Text in `public_pictures`.
    pub public_pictures_public_watch: &'static str,
    /// Text in `public_pictures`. `{}`: `i + 1`, `plan.len()`, `m`.
    pub public_pictures_share: &'static str,
    /// Text in `public_pictures`. `{}`: `held.join(" · ")`.
    pub public_pictures_holds: &'static str,
    /// Text in `public_pictures`. `{}`: `off.join(" · ")`.
    pub public_pictures_leaves: &'static str,
    /// Text in `public_pictures`.
    pub public_pictures_not: &'static str,
    /// Text in `public_pictures`.
    pub public_pictures_public: &'static str,
    // ----- public_out (faraday-core/src/lib.rs) -----
    /// Toast in `public_out`. `{}`: `k`.
    pub public_out_files: &'static str,
    // ----- backup_files_make (faraday-core/src/lib.rs) -----
    /// Toast in `backup_files_make`. `{}`: `k`.
    pub backup_files_make_files: &'static str,
    // ----- public_made (faraday-core/src/lib.rs) -----
    /// Toast in `public_made`. `{}`: `e`.
    pub public_made_no_pdf: &'static str,
    // ----- wallet_key (faraday-core/src/lib.rs) -----
    /// Error in `wallet_key`.
    pub wallet_key_that_wallet: &'static str,
    /// Error in `wallet_key`.
    pub wallet_key_this_wallet_s: &'static str,
    /// Error in `wallet_key`.
    pub wallet_key_this_wallet: &'static str,
    /// Error in `wallet_key`.
    pub wallet_key_that_key_names: &'static str,
    /// Error in `wallet_key`.
    pub wallet_key_that_key: &'static str,
    // ----- wallet_key_record (faraday-core/src/lib.rs) -----
    /// Error in `wallet_key_record`.
    pub wallet_key_record_bip: &'static str,
    // ----- create_key_text (faraday-core/src/lib.rs) -----
    /// Error in `create_key_text`.
    pub create_key_text_nothing: &'static str,
    /// Error in `create_key_text`.
    pub create_key_text_that: &'static str,
    /// Error in `create_key_text`.
    pub create_key_text_a_chosen: &'static str,
    // ----- create_keys (faraday-core/src/lib.rs) -----
    /// Error in `create_keys`.
    pub create_keys_nothing: &'static str,
    /// Error in `create_keys`. `{}`: `i + 1`.
    pub create_keys_key_has: &'static str,
    /// Text in `create_keys`. `{}`: `i + 1`, `self.medium.a()`, `i + 1`.
    pub create_keys_key_waits: &'static str,
    /// Error in `create_keys`.
    pub create_keys_a_chosen: &'static str,
    /// Error in `create_keys`.
    pub create_keys_the_same: &'static str,
    // ----- create_shares (faraday-core/src/lib.rs) -----
    /// Error in `create_shares`.
    pub create_shares_nothing: &'static str,
    /// Error in `create_shares`. `{}`: `i + 1`.
    pub create_shares_share_has: &'static str,
    /// Error in `create_shares`.
    pub create_shares_a_chosen: &'static str,
    /// Error in `create_shares`. `{}`: `i + 1`.
    pub create_shares_share: &'static str,
    /// Error in `create_shares`.
    pub create_shares_the_same: &'static str,
    // ----- create_deal (faraday-core/src/lib.rs) -----
    /// Error in `create_deal`.
    pub create_deal_a_chosen: &'static str,
    /// Text in `create_deal`. `{}`: `self.session.wallets.len() + 1`.
    pub create_deal_new_wallet: &'static str,
    /// Text in `create_deal`.
    pub create_deal_dealt_here: &'static str,
    /// Text in `create_deal`. `{}`: `id + 1`, `name`.
    pub create_deal_share_of: &'static str,
    // ----- restore_refusal (faraday-core/src/lib.rs) -----
    /// Text in `restore_refusal`.
    pub restore_refusal_every: &'static str,
    // ----- restore_described_make (faraday-core/src/lib.rs) -----
    /// Text in `restore_described_make`. `{}`: `open`, `if open == 1 { EN.restore_described_m…`.
    pub restore_described_make_still: &'static str,
    /// Text in `restore_described_make`.
    pub restore_described_make_slot: &'static str,
    /// Text in `restore_described_make`.
    pub restore_described_make_slots: &'static str,
    // ----- restore_from_vault (faraday-core/src/lib.rs) -----
    /// Text in `restore_from_vault`.
    pub restore_from_vault_wallet: &'static str,
    /// Text in `restore_from_vault`. `{}`: `keys`, `if keys == 1 { EN.restore_from_vault_…`, `source`.
    pub restore_from_vault_from: &'static str,
    /// Text in `restore_from_vault`.
    pub restore_from_vault_key: &'static str,
    /// Text in `restore_from_vault`.
    pub restore_from_vault_keys: &'static str,
    // ----- create_pass_lock (faraday-core/src/lib.rs) -----
    /// Note in `create_pass_lock`.
    pub create_pass_lock_the: &'static str,
    /// Text in `create_pass_lock`. `{}`: `key.label`.
    pub create_pass_lock_passphrase: &'static str,
    // ----- create_make (faraday-core/src/lib.rs) -----
    /// Text in `create_make`. `{}`: `self.session.wallets.len() + 1`.
    pub create_make_new_wallet: &'static str,
    /// Text in `create_make`.
    pub create_make_made_here: &'static str,
    // ----- make_pdf (faraday-core/src/lib.rs) -----
    /// Text in `make_pdf`. `{}`: `stem(name)`.
    pub make_pdf_pdf: &'static str,
    /// Toast in `make_pdf`. `{}`: `e`.
    pub make_pdf_not_printed: &'static str,
    // ----- scanned (faraday-core/src/lib.rs) -----
    /// Text in `scanned`.
    pub scanned_not_a_seedqr: &'static str,
    /// Text in `scanned`.
    pub scanned_not_a_psbt: &'static str,
    /// Toast in `scanned`. `{}`: `name`.
    pub scanned_is_a_cosigner: &'static str,
    /// Toast in `scanned`. `{}`: `name`, `slot + 1`.
    pub scanned_is_key: &'static str,
    /// Toast in `scanned`. `{}`: `name`.
    pub scanned_is_in_files: &'static str,
    // ----- copy_scanned (faraday-core/src/lib.rs) -----
    /// Note in `copy_scanned`.
    pub copy_scanned_not_a: &'static str,
    // ----- read_code (faraday-core/src/lib.rs) -----
    /// Text in `read_code`.
    pub read_code_that_is_a: &'static str,
    /// Text in `read_code`.
    pub read_code_that_may: &'static str,
    /// Text in `read_code`. `{}`: `ur_type`.
    pub read_code_ur_is_not: &'static str,
    /// Note in `read_code`. `{}`: `have`, `of`.
    pub read_code_ur_part_of: &'static str,
    /// Note in `read_code`. `{}`: `e`.
    pub read_code_not_a_part: &'static str,
    // ----- address_note (faraday-core/src/lib.rs) -----
    /// Text in `address_note`.
    pub address_note_that_address_fails: &'static str,
    /// Text in `address_note`. `{}`: `wallet::network_name(net)`.
    pub address_note_that_address_is_not: &'static str,
    /// Text in `address_note`.
    pub address_note_change: &'static str,
    /// Text in `address_note`.
    pub address_note_receive: &'static str,
    /// Text in `address_note`. `{}`: `w.name`, `chain`, `index`.
    pub address_note_that_address_is: &'static str,
    /// Text in `address_note`.
    pub address_note_that_is: &'static str,
    /// Text in `address_note`.
    pub address_note_that_address: &'static str,
    // ----- open_qr (faraday-core/src/lib.rs) -----
    /// Toast in `open_qr`. `{}`: `e`.
    pub open_qr_cannot_make: &'static str,
    // ----- code_png (faraday-core/src/lib.rs) -----
    /// Toast in `code_png`. `{}`: `e`.
    pub code_png_cannot_make: &'static str,
    /// Toast in `code_png`. `{}`: `v.title`.
    pub code_png_is_more_than: &'static str,
    // ----- code_view (faraday-core/src/lib.rs) -----
    /// Text in `code_view`. `{}`: `w.name`.
    pub code_view_threshold: &'static str,
    /// Text in `code_view`. `{}`: `sum`.
    pub code_view_descriptor: &'static str,
    /// Text in `code_view`. `{}`: `w.name`.
    pub code_view_wallet_descriptor: &'static str,
    /// Error in `code_view`.
    pub code_view_this_wallet_has: &'static str,
    /// Text in `code_view`. `{}`: `w.name`.
    pub code_view_multisig: &'static str,
    /// Text in `code_view`. `{}`: `w.name`.
    pub code_view_bsms_descriptor: &'static str,
    /// Text in `code_view`.
    pub code_view_public_watch: &'static str,
    /// Error in `code_view`.
    pub code_view_bip_129_covers: &'static str,
    /// Text in `code_view`. `{}`: `k.fp`, `k.kind`.
    pub code_view_xpub_2: &'static str,
    /// Error in `code_view`.
    pub code_view_that_wallet: &'static str,
    /// Error in `code_view`.
    pub code_view_this_wallet_s: &'static str,
    /// Error in `code_view`.
    pub code_view_this_wallet: &'static str,
    /// Text in `code_view`.
    pub code_view_no_origin: &'static str,
    /// Text in `code_view`. `{}`: `fp`, `Session::shape(w)`.
    pub code_view_key: &'static str,
    /// Text in `code_view`. `{}`: `fp`.
    pub code_view_xpub: &'static str,
    /// Text in `code_view`.
    pub code_view_bip_129_key: &'static str,
    /// Text in `code_view`.
    pub code_view_public_spends: &'static str,
    /// Text in `code_view`. `{}`: `k.fp`.
    pub code_view_key_bsms: &'static str,
    /// Error in `code_view`.
    pub code_view_no_message: &'static str,
    /// Error in `code_view`.
    pub code_view_the_message: &'static str,
    /// Text in `code_view`. `{}`: `sig.address`.
    pub code_view_signed_by: &'static str,
    /// Text in `code_view`.
    pub code_view_public_proves: &'static str,
    /// Text in `code_view`.
    pub code_view_signed_message: &'static str,
    // ----- wallet_label (faraday-core/src/lib.rs) -----
    /// Text in `wallet_label`.
    pub wallet_label_key: &'static str,
    /// Text in `wallet_label`.
    pub wallet_label_keys: &'static str,
    // ----- entry_add (faraday-core/src/lib.rs) -----
    /// Toast in `entry_add`. `{}`: `fp_text(fp)`.
    pub entry_add_key_added: &'static str,
    // ----- load_key (faraday-core/src/lib.rs) -----
    /// Toast in `load_key`. `{}`: `fp_text(fp)`.
    pub load_key_key_added: &'static str,
    // ----- qr_read (faraday-core/src/lib.rs) -----
    /// Text in `qr_read`. `{}`: `name`.
    pub qr_read_copied_no_qr: &'static str,
    /// Text in `qr_read`. `{}`: `name`, `got`, `self.medium.noun()`.
    pub qr_read_a_seedqr_copied: &'static str,
    /// Toast in `qr_read`. `{}`: `got`.
    pub qr_read_is_in_files: &'static str,
    /// Text in `qr_read`. `{}`: `name`, `got`.
    pub qr_read_read_into_files: &'static str,
    /// Text in `qr_read`.
    pub qr_read_read: &'static str,
    // ----- visit_inbox (faraday-core/src/lib.rs) -----
    /// Text in `visit_inbox`.
    pub visit_inbox_a_png_with: &'static str,
    /// Text in `visit_inbox`. `{}`: `screens::kind_name(k).to_lowercase()`.
    pub visit_inbox_a_png_of: &'static str,
    /// Text in `visit_inbox`.
    pub visit_inbox_a_file: &'static str,
    /// Text in `visit_inbox`.
    pub visit_inbox_whoever_has_it_holds: &'static str,
    /// Text in `visit_inbox`.
    pub visit_inbox_whoever_has_enough: &'static str,
    /// Text in `visit_inbox`.
    pub visit_inbox_whoever_has_it: &'static str,
    /// Text in `visit_inbox`.
    pub visit_inbox_whoever_has: &'static str,
    /// Text in `visit_inbox`.
    pub visit_inbox_whoever: &'static str,
    // ----- sign_here (faraday-core/src/lib.rs) -----
    /// Text in `sign_here`. `{}`: `waiting.join(", ")`.
    pub sign_here_waits_for: &'static str,
    /// Error in `sign_here`.
    pub sign_here_no_randomness: &'static str,
    /// Text in `sign_here`.
    pub sign_here_this_transaction: &'static str,
    /// Text in `sign_here`. `{}`: `want`, `if want == 1 { EN.sign_here_share } e…`.
    pub sign_here_choose_other: &'static str,
    /// Text in `sign_here`.
    pub sign_here_share: &'static str,
    /// Text in `sign_here`.
    pub sign_here_shares: &'static str,
    /// Toast in `sign_here`.
    pub sign_here_this_device: &'static str,
    // ----- collect (faraday-core/src/lib.rs) -----
    /// Error in `collect`. `{}`: `name`.
    pub collect_has_nothing: &'static str,
    /// Error in `collect`.
    pub collect_no_randomness: &'static str,
    // ----- primary (faraday-core/src/lib.rs) -----
    /// Error in `primary`. `{}`: `pos + 1`.
    pub primary_finish_step: &'static str,
    // ----- event (faraday-core/src/lib.rs) -----
    /// Toast in `event`.
    pub event_no_camera_or: &'static str,
    // ----- lightning (faraday-core/src/lightning.rs) -----
    /// Error in `lightning_resolve`.
    pub lightning_this_key: &'static str,
    /// Error in `lightning_resolve`. `{}`: `words.len()`.
    pub lightning_an_aezeed: &'static str,
    /// Error in `lightning_resolve`. `{}`: `k + 1`.
    pub lightning_word_is_not: &'static str,
    /// Text in `lightning_resolve`. `{}`: `e`.
    pub lightning_not_an_aezeed: &'static str,
    /// Toast in `lightning_act`.
    pub lightning_no_vault: &'static str,
    /// Text in `lightning_act`. `{}`: `id`, `private.as_str()`.
    pub lightning_lightning: &'static str,
    /// Toast in `lightning_act`.
    pub lightning_the_node: &'static str,
    /// Text in `lightning_act`.
    pub lightning_a_lightning: &'static str,
    /// Text in `lightning_act`.
    pub lightning_whoever_has: &'static str,
    // ----- lightning (faraday-core/src/lightning_screen.rs) -----
    /// Text in `draw`.
    pub lightning_a_lightning_node: &'static str,
    /// Line in `header`.
    pub lightning_wallets: &'static str,
    /// Title in `header`.
    pub lightning_lightning_node: &'static str,
    /// Text in `body`. `{}`: `crate::wallet::key_line(fp, &k.label)`.
    pub lightning_ldk_node: &'static str,
    /// Text in `body`.
    pub lightning_lnd_type: &'static str,
    /// Text in `body`.
    pub lightning_the_24_words: &'static str,
    /// Text in `body`. `{}`: `n`.
    pub lightning_of_24: &'static str,
    /// Line in `body`.
    pub lightning_passphrase: &'static str,
    /// Text in `body`.
    pub lightning_aezeed_lnd: &'static str,
    /// Button in `body`.
    pub lightning_read_it: &'static str,
    /// Line in `body`.
    pub lightning_node_id: &'static str,
    /// Line in `body`.
    pub lightning_made: &'static str,
    /// Line in `body`.
    pub lightning_private_key: &'static str,
    /// Text in `body`.
    pub lightning_hide: &'static str,
    /// Text in `body`.
    pub lightning_show: &'static str,
    /// Text in `body`. `{}`: `v.name`.
    pub lightning_save_into: &'static str,
    /// Text in `body`.
    pub lightning_out_unprotected: &'static str,
    // ----- medium (faraday-core/src/medium.rs) -----
    /// Text in `noun`.
    pub medium_stick_inline: &'static str,
    /// Text in `nouns`.
    pub medium_sticks_inline: &'static str,
    /// Text in `a`.
    pub medium_a_stick_inline: &'static str,
    /// Text in `a`.
    pub medium_an_sd_card_inline: &'static str,
    /// Text in `cap`.
    pub medium_stick: &'static str,
    /// Text in `caps`.
    pub medium_sticks: &'static str,
    /// Text in `caps`.
    pub medium_sd_cards: &'static str,
    /// Text in `a_cap`.
    pub medium_a_stick: &'static str,
    /// Text in `full`.
    pub medium_usb_stick: &'static str,
    /// Text in `full`.
    pub medium_sd_card: &'static str,
    /// Text in `a_full_cap`.
    pub medium_a_usb_stick: &'static str,
    /// Text in `a_full_cap`.
    pub medium_an_sd_card: &'static str,
    /// Text in `from_box`.
    pub medium_from_the_stick: &'static str,
    /// Text in `from_box`.
    pub medium_from_the_sd_card: &'static str,
    /// Text in `for_box`.
    pub medium_for_the_stick: &'static str,
    /// Text in `for_box`.
    pub medium_for_the_sd_card: &'static str,
    /// Text in `from_the`.
    pub medium_from_the_stick_inline: &'static str,
    /// Text in `from_the`.
    pub medium_from_the_sd: &'static str,
    /// Text in `on_the`.
    pub medium_on_the_stick: &'static str,
    /// Text in `on_the`.
    pub medium_on_the_sd_card: &'static str,
    /// Text in `for_the`.
    pub medium_for_the_stick_inline: &'static str,
    /// Text in `for_the`.
    pub medium_for_the_sd_card_inline: &'static str,
    /// Text in `visit`.
    pub medium_stick_visit: &'static str,
    /// Text in `visit`.
    pub medium_sd_card_visit: &'static str,
    /// Text in `upgrade`.
    pub medium_upgrade_a_faraday_stick: &'static str,
    /// Text in `upgrade`.
    pub medium_upgrade_a_faraday: &'static str,
    /// Text in `boot`.
    pub medium_boot_stick: &'static str,
    /// Text in `boot`.
    pub medium_boot_sd_card: &'static str,
    // ----- paper (faraday-core/src/paper.rs) -----
    /// Toast in `paper_xor`.
    pub paper_this_key_has_no: &'static str,
    /// Toast in `paper_codex32`.
    pub paper_this_key_has: &'static str,
    /// Toast in `paper_codex32`.
    pub paper_no_randomness: &'static str,
    /// Toast in `paper_codex32`. `{}`: `e`.
    pub paper_codex32: &'static str,
    // ----- pdf (faraday-core/src/pdf.rs) -----
    /// Error in `fields`.
    pub pdf_not_a_faraday_sheet: &'static str,
    /// Text in `sheet`. `{}`: `other`.
    pub pdf_a_sheet_of_kind: &'static str,
    /// Text in `wallet_sheet`. `{}`: `name`.
    pub pdf_wallet_backup: &'static str,
    /// Line in `wallet_sheet`.
    pub pdf_xpubs_only_this: &'static str,
    /// Line in `wallet_sheet`.
    pub pdf_descriptor: &'static str,
    /// Line in `wallet_sheet`.
    pub pdf_xpubs: &'static str,
    /// Line in `wallet_sheet`.
    pub pdf_first_addresses: &'static str,
    /// Text in `wallet_sheet`.
    pub pdf_written_on: &'static str,
    /// Text in `wallet_sheet`.
    pub pdf_kept_at: &'static str,
    /// Text in `wallet_sheet`.
    pub pdf_goes_with_seed: &'static str,
    /// Line in `wallet_sheet`.
    pub pdf_made_by_faraday_check: &'static str,
    /// Text in `envelopes_sheet`. `{}`: `name`.
    pub pdf_envelopes: &'static str,
    /// Text in `share_sheet`.
    pub pdf_wallet: &'static str,
    /// Text in `share_sheet`. `{}`: `name`, `part`.
    pub pdf_split_backup_sheet: &'static str,
    /// Text in `share_sheet`. `{}`: `quorum`.
    pub pdf_part_of_the_wallet: &'static str,
    /// Line in `share_sheet`.
    pub pdf_made_by_faraday: &'static str,
    /// Error in `blank_sheet`.
    pub pdf_the_template_is: &'static str,
    /// Text in `blank_sheet`. `{}`: `words`.
    pub pdf_seed_sheet_words: &'static str,
    /// Line in `blank_sheet`.
    pub pdf_written_by_hand: &'static str,
    /// Text in `blank_sheet`. `{}`: `get(f, "network").unwrap_or("mainnet")`.
    pub pdf_network: &'static str,
    /// Text in `blank_sheet`.
    pub pdf_fingerprint: &'static str,
    /// Text in `blank_sheet`.
    pub pdf_first_address: &'static str,
    /// Line in `blank_sheet`.
    pub pdf_there_is_no_line: &'static str,
    /// Line in `blank_sheet`.
    pub pdf_places: &'static str,
    /// Line in `blank_sheet`.
    pub pdf_place: &'static str,
    /// Line in `blank_sheet`.
    pub pdf_holds: &'static str,
    // ----- plan (faraday-core/src/plan.rs) -----
    /// Text in `name`.
    pub plan_paper_only: &'static str,
    /// Text in `name`.
    pub plan_paper_and_vault: &'static str,
    /// Text in `name`.
    pub plan_paper_vault_and: &'static str,
    /// Text in `vault_name`. `{}`: `v + 1`, `names.join(", ")`.
    pub plan_vault_2: &'static str,
    /// Text in `label`. `{}`: `seed(i)`.
    pub plan_seed_words: &'static str,
    /// Text in `label`. `{}`: `seed(i)`.
    pub plan_seed_seedqr: &'static str,
    /// Text in `label`. `{}`: `seed(i)`.
    pub plan_passphrase_of: &'static str,
    /// Text in `label`.
    pub plan_wallet_sheet: &'static str,
    /// Text in `label`. `{}`: `j + 1`, `shape.keys`.
    pub plan_share_of: &'static str,
    /// Text in `label`. `{}`: `medium.cap()`, `v + 1`.
    pub plan_with_vault: &'static str,
    /// Text in `label`. `{}`: `seed(i)`.
    pub plan_seed: &'static str,
    /// Text in `label`. `{}`: `seed(i)`.
    pub plan_seed_with_its: &'static str,
    /// Text in `label`.
    pub plan_wallet_description: &'static str,
    /// Text in `label`. `{}`: `seed(i)`.
    pub plan_seed_file: &'static str,
    /// Text in `text`.
    pub plan_yes_with_the_vault: &'static str,
    /// Text in `text`.
    pub plan_yes: &'static str,
    /// Text in `text`.
    pub plan_only_with_the: &'static str,
    /// Text in `text`.
    pub plan_no: &'static str,
    /// Text in `CHECK_LINES`.
    pub plan_any_one_place: &'static str,
    /// Text in `CHECK_LINES`.
    pub plan_one_place_found_can: &'static str,
    /// Text in `CHECK_LINES`.
    pub plan_one_place_found: &'static str,
    /// Text in `text`.
    pub plan_can_spend: &'static str,
    /// Text in `text`.
    pub plan_sees_the_balance_can: &'static str,
    /// Text in `text`.
    pub plan_can_spend_with: &'static str,
    /// Text in `text`.
    pub plan_sees_the_balance_2: &'static str,
    /// Text in `text`.
    pub plan_sees_the_balance: &'static str,
    /// Text in `text`.
    pub plan_nothing: &'static str,
    /// Text in `lost_why`.
    pub plan_at_no_place: &'static str,
    /// Text in `lost_why`. `{}`: `v + 1`, `noun`, `stick_text(*v)`.
    pub plan_vault_whose_is: &'static str,
    /// Text in `lost_why`. `{}`: `w`.
    pub plan_in: &'static str,
    /// Text in `lost_why`. `{}`: `w`.
    pub plan_on: &'static str,
    /// Text in `lost_why`. `{}`: `w`.
    pub plan_at: &'static str,
    /// Text in `lost_why`. `{}`: `name_at(gone, name, noun)`.
    pub plan_with: &'static str,
    /// Text in `lost_why`. `{}`: `seed`, `line`.
    pub plan_seed_is_kept_only: &'static str,
    /// Text in `lost_why`. `{}`: `seed`, `name_at(gone, name, noun)`.
    pub plan_the_passphrase_of: &'static str,
    /// Text in `lost_why`. `{}`: `name_at(gone, name, noun)`.
    pub plan_without_the_rest: &'static str,
    /// Text in `name_at`. `{}`: `v + 1`.
    pub plan_vault: &'static str,
    /// Text in `name_at`. `{}`: `noun`.
    pub plan_the_of_files: &'static str,
    /// Text in `name_at`.
    pub plan_the_software: &'static str,
    /// Text in `name_at`.
    pub plan_the_devices: &'static str,
    /// Text in `kept_nowhere`. `{}`: `seed.name`.
    pub plan_seed_is_kept_nowhere: &'static str,
    /// Text in `kept_nowhere`. `{}`: `seed.name`.
    pub plan_the_passphrase: &'static str,
    /// A thing's tag on the map, as `Tag::Secret` shows.
    pub plan_tag_secret: &'static str,
    /// A thing's tag on the map, as `Tag::Sealed` shows.
    pub plan_tag_sealed: &'static str,
    /// A thing's tag on the map, as `Tag::Public` shows.
    pub plan_tag_public: &'static str,
    /// A thing's mark on the chart.
    pub plan_mark_lost: &'static str,
    /// A thing's mark on the chart.
    pub plan_mark_exposed: &'static str,
    // ----- restore (faraday-core/src/restore.rs) -----
    /// Error in `merge`.
    pub restore_not_a_multisig: &'static str,
    /// Error in `merge`.
    pub restore_these_shares: &'static str,
    /// Error in `merge`.
    pub restore_no_shares: &'static str,
    // ----- scan_key (faraday-core/src/scan_key.rs) -----
    /// Text in `not_a_wallet_key`. `{}`: `fp_text(fp)`, `name`.
    pub scan_key_is_not_one: &'static str,
    /// Text in `wallet_key_lines`. `{}`: `n + 1`, `fp_text(fp)`.
    pub scan_key_key: &'static str,
    /// Note in `wallet_key_scanned`.
    pub scan_key_not_a_seedqr: &'static str,
    /// Note in `wallet_key_scanned`. `{}`: `n + 1`.
    pub scan_key_key_already: &'static str,
    /// Toast in `wallet_key_said`. `{}`: `n + 1`, `short`.
    pub scan_key_key_added_more: &'static str,
    /// Toast in `wallet_key_said`. `{}`: `n + 1`.
    pub scan_key_key_added: &'static str,
    // ----- held_frame (faraday-core/src/screens.rs) -----
    /// Text in `held_frame`.
    pub held_frame_home: &'static str,
    // ----- notices (faraday-core/src/screens.rs) -----
    /// Text in `notices`. `{}`: `app.medium.noun()`.
    pub notices_remove_the: &'static str,
    // ----- sidebar (faraday-core/src/screens.rs) -----
    /// Text in `sidebar`.
    pub sidebar_home: &'static str,
    /// Text in `sidebar`.
    pub sidebar_vaults: &'static str,
    /// Text in `sidebar`.
    pub sidebar_tools: &'static str,
    /// Text in `sidebar`.
    pub sidebar_transfer: &'static str,
    /// Text in `sidebar`.
    pub sidebar_settings: &'static str,
    /// Text in `sidebar`. `{}`: `v`.
    pub sidebar_open_n: &'static str,
    /// Row in `sidebar`.
    pub sidebar_learn: &'static str,
    /// Text in `sidebar`.
    pub sidebar_bring_in: &'static str,
    /// Text in `sidebar`.
    pub sidebar_open: &'static str,
    /// Text in `sidebar`.
    pub sidebar_work: &'static str,
    /// Text in `sidebar`.
    pub sidebar_write_out: &'static str,
    /// Text in `sidebar`.
    pub sidebar_passphrase: &'static str,
    /// Text in `sidebar`. `{}`: `total - shown`.
    pub sidebar_more: &'static str,
    /// Text in `sidebar`.
    pub sidebar_keys: &'static str,
    /// Text in `sidebar`. `{}`: `m`, `n`.
    pub sidebar_of: &'static str,
    /// Text in `sidebar`.
    pub sidebar_single_key: &'static str,
    /// Text in `sidebar`.
    pub sidebar_wallets: &'static str,
    /// Line in `sidebar`.
    pub sidebar_files: &'static str,
    /// Text in `sidebar`.
    pub sidebar_import_waiting: &'static str,
    /// Text in `sidebar`.
    pub sidebar_none: &'static str,
    /// Text in `sidebar`. `{}`: `k`.
    pub sidebar_attached: &'static str,
    /// Button in `sidebar`.
    pub sidebar_lock: &'static str,
    // ----- home_lead (faraday-core/src/screens.rs) -----
    /// Text in `home_lead`.
    pub home_lead_make_a_new: &'static str,
    /// Text in `home_lead`.
    pub home_lead_single_key: &'static str,
    // ----- home_job (faraday-core/src/screens.rs) -----
    /// Text in `home_job`.
    pub home_job_carry_on_the: &'static str,
    /// Text in `home_job`. `{}`: `w.name`, `s.signers.len()`, `app.spend_needed()`.
    pub home_job_signatures: &'static str,
    /// Text in `home_job`. `{}`: `s.spend.source`.
    pub home_job_continue_signing: &'static str,
    /// Text in `home_job`. `{}`: `s.signers.len()`, `app.spend_needed()`.
    pub home_job_of_signatures: &'static str,
    /// Text in `home_job`. `{}`: `app.inbox[i].name`.
    pub home_job_sign: &'static str,
    /// Text in `home_job`.
    pub home_job_from_files: &'static str,
    /// Text in `home_job`. `{}`: `app.medium.noun()`.
    pub home_job_remove_the: &'static str,
    /// Text in `home_job`. `{}`: `imp.label`.
    pub home_job_read_from: &'static str,
    /// Text in `home_job`. `{}`: `imp.label`.
    pub home_job_import_from: &'static str,
    /// Text in `home_job`. `{}`: `r.label`.
    pub home_job_written_to: &'static str,
    /// Text in `home_job`. `{}`: `f.name`.
    pub home_job_in_files: &'static str,
    /// Text in `home_job`. `{}`: `name`.
    pub home_job_unlock: &'static str,
    /// Text in `home_job`. `{}`: `app.medium.noun()`.
    pub home_job_pull_the_to: &'static str,
    /// Text in `home_job`. `{}`: `name`, `app.medium.a()`.
    pub home_job_write_to: &'static str,
    // ----- home_ways (faraday-core/src/screens.rs) -----
    /// Text in `home_ways`.
    pub home_ways_make_a_new: &'static str,
    /// Text in `home_ways`.
    pub home_ways_single_key: &'static str,
    /// Text in `home_ways`.
    pub home_ways_restore_an: &'static str,
    /// Text in `home_ways`.
    pub home_ways_from_its: &'static str,
    /// Text in `home_ways`.
    pub home_ways_spend_from: &'static str,
    /// Text in `home_ways`.
    pub home_ways_a_stick_words: &'static str,
    /// Text in `home_ways`.
    pub home_ways_create_a: &'static str,
    /// Text in `home_ways`.
    pub home_ways_none_yet: &'static str,
    /// Text in `home_ways`.
    pub home_ways_vaults: &'static str,
    /// Text in `home_ways`. `{}`: `vault_files`.
    pub home_ways_locked: &'static str,
    /// Text in `home_ways`. `{}`: `open`.
    pub home_ways_open: &'static str,
    /// Text in `home_ways`.
    pub home_ways_a_stick_attached: &'static str,
    /// Text in `home_ways`.
    pub home_ways_backups: &'static str,
    /// Text in `home_ways`. `{}`: `known.len()`.
    pub home_ways_with_a_plan: &'static str,
    /// Text in `home_ways`. `{}`: `without`.
    pub home_ways_not_backed: &'static str,
    // ----- home (faraday-core/src/screens.rs) -----
    /// Title in `home`.
    pub home_home: &'static str,
    /// Section label in `home`.
    pub home_start: &'static str,
    /// Button in `home`.
    pub home_scan: &'static str,
    // ----- start (faraday-core/src/screens.rs) -----
    /// Title in `start`.
    pub start_wallets: &'static str,
    /// Text in `start`. `{}`: `app.medium.a()`.
    pub start_no_wallet_is: &'static str,
    /// Text in `start`.
    pub start_start_from_what: &'static str,
    /// Section label in `start`.
    pub start_your_wallets: &'static str,
    /// Section label in `start`.
    pub start_keys_without: &'static str,
    /// Text in `start`.
    pub start_make_a_new_wallet: &'static str,
    /// Text in `start`.
    pub start_single_key_or: &'static str,
    /// Text in `start`.
    pub start_restore_an_existing: &'static str,
    /// Text in `start`.
    pub start_from_its_backups: &'static str,
    /// Text in `start`.
    pub start_spend_from_a: &'static str,
    /// Text in `start`.
    pub start_a_stick_words: &'static str,
    /// Text in `start`.
    pub start_add_a_key: &'static str,
    /// Text in `start`.
    pub start_keys_first_wallets: &'static str,
    /// Text in `start`. `{}`: `files[i].name`.
    pub start_is_in_files_locked: &'static str,
    /// Button in `start`.
    pub start_unlock: &'static str,
    // ----- nonce_check (faraday-core/src/screens.rs) -----
    /// Text in `nonce_check`. `{}`: `check.rows.len()`, `if check.rows.len() == 1 { EN.nonce_c…`, `here`.
    pub nonce_check_nonce_check_valid: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_signature: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_signatures: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_nonce_check: &'static str,
    /// Text in `nonce_check`. `{}`: `reuse.name()`, `inputs.join(", ")`.
    pub nonce_check_one_nonce: &'static str,
    /// Text in `nonce_check`. `{}`: `r.input`.
    pub nonce_check_input: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_invalid: &'static str,
    /// Text in `nonce_check`. `{}`: `why`.
    pub nonce_check_not_checked: &'static str,
    /// Text in `nonce_check`. `{}`: `mode`.
    pub nonce_check_valid_nonce: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_valid_not: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_valid_signed: &'static str,
    /// Text in `nonce_check`. `{}`: `rest.len()`, `if ok { EN.nonce_check_none_wrong } e…`.
    pub nonce_check_more: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_none_wrong: &'static str,
    /// Text in `nonce_check`.
    pub nonce_check_one_is: &'static str,
    // ----- inbox_panel (faraday-core/src/screens.rs) -----
    /// Line in `inbox_panel`.
    pub inbox_panel_wallets: &'static str,
    /// Text in `inbox_panel`. `{}`: `fw.shape`, `fw.seeds_here`, `n`, `if n == 1 { EN.inbox_panel_key } else…`, `if fw.files.len() > 1 { format!(" · {…`.
    pub inbox_panel_of_here: &'static str,
    /// Text in `inbox_panel`, after a space. `{}`: `fw.files.len()`.
    pub inbox_panel_files_say_it: &'static str,
    /// Text in `inbox_panel`, after a space.
    pub inbox_panel_from_split_sheets: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_key: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_keys: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_ready_to: &'static str,
    /// Text in `inbox_panel`. `{}`: `wallets.len() - ROWS`.
    pub inbox_panel_more_loaded: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_nothing: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_load_the_wallet: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_load_the: &'static str,
    /// Text in `inbox_panel`. `{}`: `n`.
    pub inbox_panel_load_wallets_with: &'static str,
    /// Text in `inbox_panel`. `{}`: `n`.
    pub inbox_panel_load_wallets: &'static str,
    /// Text in `inbox_panel`. `{}`: `app.medium.noun()`.
    pub inbox_panel_keys_load: &'static str,
    /// Line in `inbox_panel`.
    pub inbox_panel_keys_with: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_make_it_a: &'static str,
    /// Text in `inbox_panel`. `{}`: `potential.len() - ROWS`.
    pub inbox_panel_more: &'static str,
    /// Line in `inbox_panel`.
    pub inbox_panel_xpubs_with: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_no_origin: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_make_it: &'static str,
    /// Line in `inbox_panel`.
    pub inbox_panel_not_complete: &'static str,
    /// Text in `inbox_panel`.
    pub inbox_panel_open_with: &'static str,
    // ----- potential (faraday-core/src/screens.rs) -----
    /// Text in `potential_sheet`. `{}`: `item.name`.
    pub potential_open: &'static str,
    /// Text in `potential_sheet`. `{}`: `fp_text(fp)`.
    pub potential_a_watch_only_wallet: &'static str,
    /// Text in `potential_sheet`.
    pub potential_a_watch_only: &'static str,
    /// Text in `potential_sheet`. `{}`: `fp_text(fp)`.
    pub potential_a_wallet: &'static str,
    /// Text in `potential_sheet`. `{}`: `from.join(" · ")`.
    pub potential_from: &'static str,
    /// Text in `potential_sheet`.
    pub potential_the_backup: &'static str,
    /// Text in `potential_sheet`.
    pub potential_slip_39_passphrase: &'static str,
    /// Text in `potential_sheet`.
    pub potential_bip_39_passphrase: &'static str,
    /// Text in `potential_sheet`. `{}`: `crate::family::kind_name(p.kind)`.
    pub potential_as_its_path: &'static str,
    /// Line in `potential_sheet`.
    pub potential_fingerprint: &'static str,
    /// Text in `potential_sheet`. `{}`: `name`.
    pub potential_a_key_of: &'static str,
    /// Line in `potential_sheet`.
    pub potential_single_key: &'static str,
    /// Button in `potential_sheet`.
    pub potential_cancel: &'static str,
    /// Text in `potential_sheet`.
    pub potential_open_it: &'static str,
    /// Text in `potential_sheet`. `{}`: `name`.
    pub potential_load: &'static str,
    /// Text in `potential_sheet`.
    pub potential_make_the: &'static str,
    // ----- catalog (faraday-core/src/screens.rs) -----
    /// Title in `catalog_screen`.
    pub catalog_tools: &'static str,
    /// Text in `catalog_screen`.
    pub catalog_find_a_tool: &'static str,
    /// Line in `catalog_screen`.
    pub catalog_no_tool_matches: &'static str,
    // ----- decode_back (faraday-core/src/screens.rs) -----
    /// Text in `decode_back`.
    pub decode_back_sign_a: &'static str,
    /// Text in `decode_back`.
    pub decode_back_spend: &'static str,
    /// Text in `decode_back`.
    pub decode_back_files: &'static str,
    /// Text in `decode_back`.
    pub decode_back_wallets: &'static str,
    /// Text in `decode_back`.
    pub decode_back_tools: &'static str,
    /// Text in `decode_back`.
    pub decode_back_back: &'static str,
    // ----- decode (faraday-core/src/screens.rs) -----
    /// Title in `decode_screen`.
    pub decode_decode_a_transaction: &'static str,
    /// Text in `decode_screen`. `{}`: `f`, `f as f64 / t.size.1.max(1) as f64`.
    pub decode_sat_sat_vb: &'static str,
    /// Text in `decode_screen`.
    pub decode_not_stated_the: &'static str,
    /// Text in `decode_screen`.
    pub decode_transaction: &'static str,
    /// Text in `decode_screen`.
    pub decode_witness_id: &'static str,
    /// Text in `decode_screen`.
    pub decode_version: &'static str,
    /// Text in `decode_screen`.
    pub decode_locktime: &'static str,
    /// Text in `decode_screen`.
    pub decode_size: &'static str,
    /// Text in `decode_screen`. `{}`: `t.size.0`, `t.size.1`, `t.size.2`.
    pub decode_bytes_vb_wu: &'static str,
    /// Text in `decode_screen`.
    pub decode_fee: &'static str,
    /// Text in `decode_screen`. `{}`: `t.inputs.len()`.
    pub decode_inputs: &'static str,
    /// Text in `decode_screen`.
    pub decode_no_witness: &'static str,
    /// Text in `decode_screen`. `{}`: `input.witness.len()`, `if input.witness.len() == 1 { EN.deco…`, `sizes.join(", ")`.
    pub decode_witness_bytes: &'static str,
    /// Text in `decode_screen`.
    pub decode_item: &'static str,
    /// Text in `decode_screen`.
    pub decode_items: &'static str,
    /// Text in `decode_screen`. `{}`: `input.sequence`, `wit`, `if input.script_sig > 0 { format!(" ·…`.
    pub decode_sequence: &'static str,
    /// Text in `decode_screen`, after a space. `{}`: `input.script_sig`.
    pub decode_script_sig_bytes: &'static str,
    /// Text in `decode_screen`. `{}`: `t.outputs.len()`.
    pub decode_outputs: &'static str,
    /// Text in `decode_screen`. `{}`: `name`, `n`.
    pub decode_change_back: &'static str,
    /// Text in `decode_screen`. `{}`: `name`, `n`.
    pub decode_back_to_receive: &'static str,
    /// Text in `decode_screen`.
    pub decode_pays_out_of: &'static str,
    /// Text in `decode_screen`.
    pub decode_hide_the_hex: &'static str,
    /// Text in `decode_screen`.
    pub decode_show_the_hex: &'static str,
    /// Text in `decode_screen`.
    pub decode_show_as_qr: &'static str,
    // ----- missing_keys_line (faraday-core/src/screens.rs) -----
    /// Text in `missing_keys_line`. `{}`: `files[i].name`.
    pub missing_keys_line_is: &'static str,
    /// Text in `missing_keys_line`. `{}`: `open_names.join(", ")`.
    pub missing_keys_line_not: &'static str,
    /// Text in `missing_keys_line`. `{}`: `app.medium.a()`.
    pub missing_keys_line_a: &'static str,
    /// Button in `missing_keys_line`.
    pub missing_keys_line_unlock: &'static str,
    // ----- wallet_rows (faraday-core/src/screens.rs) -----
    /// Line in `wallet_rows`.
    pub wallet_rows_none_loaded: &'static str,
    /// Text in `wallet_rows`. `{}`: `here`, `m`.
    pub wallet_rows_can_spend: &'static str,
    /// Text in `wallet_rows`. `{}`: `here`, `m`.
    pub wallet_rows_of_here: &'static str,
    /// Text in `wallet_rows`.
    pub wallet_rows_watch_only: &'static str,
    /// Text in `wallet_rows`. `{}`: `total - shown`.
    pub wallet_rows_more: &'static str,
    // ----- app (faraday-core/src/screens.rs) -----
    /// Text in `FROM_KEY`.
    pub app_make_a_wallet_from: &'static str,
    /// Text in `STEPS`.
    pub app_wallet: &'static str,
    /// Text in `STEPS`.
    pub app_transaction: &'static str,
    /// Text in `STEPS`.
    pub app_transaction_id: &'static str,
    /// Text in `STEPS`.
    pub app_signers: &'static str,
    /// Text in `STEPS`.
    pub app_sign: &'static str,
    /// Text in `STEPS`.
    pub app_signatures_for: &'static str,
    /// Text in `STEPS`.
    pub app_finish: &'static str,
    /// Text in `STEPS`.
    pub app_path: &'static str,
    /// Text in `STEPS`.
    pub app_nonces: &'static str,
    /// Text in `SEED_ROWS`.
    pub app_on_paper_words: &'static str,
    /// Text in `SEED_ROWS`.
    pub app_on_paper_seedqr: &'static str,
    /// Text in `SEED_ROWS`.
    pub app_into_vaults: &'static str,
    /// Text in `SEED_ROWS`.
    pub app_as_a_file_unprotected: &'static str,
    /// Text in `SOFTWARE_ROWS`.
    pub app_sparrow: &'static str,
    /// Text in `SOFTWARE_ROWS`.
    pub app_coldcard_keystone: &'static str,
    /// Text in `SOFTWARE_ROWS`.
    pub app_nunchuk: &'static str,
    /// Text in `SOFTWARE_ROWS`.
    pub app_bitcoin_core: &'static str,
    /// Text in `SOFTWARE_ROWS`.
    pub app_not_sure: &'static str,
    /// Text in `description_rows`.
    pub app_a_share_in_each: &'static str,
    /// Text in `description_rows`.
    pub app_a_sheet_in_each: &'static str,
    /// Text in `description_rows`.
    pub app_in_the_vault: &'static str,
    /// Text in `description_rows`.
    pub app_into_watch_only: &'static str,
    /// Text. `{}`: `medium.a()`.
    pub app_as_files_on: &'static str,
    /// Text in `MSTEPS`.
    pub app_address: &'static str,
    /// Text in `MSTEPS`.
    pub app_message: &'static str,
    /// Text in `MSTEPS`.
    pub app_format: &'static str,
    /// Text in `MSTEPS`.
    pub app_signature: &'static str,
    /// Text in `CSTEPS`.
    pub app_back_up: &'static str,
    /// Text in `RSTEPS`.
    pub app_kind: &'static str,
    /// Text in `RSTEPS`.
    pub app_quorum: &'static str,
    /// Text in `RSTEPS`.
    pub app_description: &'static str,
    /// Text in `RSTEPS`.
    pub app_keys: &'static str,
    /// Text in `RSTEPS`.
    pub app_check: &'static str,
    /// Text in `RSTEPS`.
    pub app_done: &'static str,
    /// Text in `PANIC_KEY`.
    pub app_super_s_held_2: &'static str,
    /// Text in `TIME_FOR`.
    pub app_for_totp_codes: &'static str,
    /// Text in `NOT_AIRGAPPED`.
    pub app_this_computer_is: &'static str,
    /// Text in `NOT_AIRGAPPED`.
    pub app_anything_running: &'static str,
    /// Text in `NOT_AIRGAPPED`.
    pub app_for_real_funds: &'static str,
    // ----- loose_card (faraday-core/src/screens.rs) -----
    /// Text in `loose_card`. `{}`: `fp_text(*only)`.
    pub loose_card_key_is_loaded: &'static str,
    /// Text in `loose_card`. `{}`: `loose.len()`.
    pub loose_card_keys_are: &'static str,
    /// Text in `loose_card`.
    pub loose_card_add_another: &'static str,
    /// Text in `loose_card`.
    pub loose_card_restore: &'static str,
    // ----- wallets (faraday-core/src/screens.rs) -----
    /// Text in `wallets`.
    pub wallets_signing: &'static str,
    /// Text in `wallets`.
    pub wallets_continue: &'static str,
    /// Text in `wallets`.
    pub wallets_in_files: &'static str,
    /// Text in `wallets`.
    pub wallets_review: &'static str,
    /// Section label in `wallets`.
    pub wallets_wallets: &'static str,
    /// Text in `wallets`. `{}`: `shape`.
    pub wallets_watch_only: &'static str,
    /// Text in `wallets`. `{}`: `shape`, `here`, `m`.
    pub wallets_of_here: &'static str,
    /// Section label in `wallets`.
    pub wallets_keys_without: &'static str,
    /// Button in `wallets`.
    pub wallets_restore_an: &'static str,
    /// Text in `wallets`. `{}`: `wl.name`.
    pub wallets_scan_a_key_for: &'static str,
    /// Text in `wallets`.
    pub wallets_add_a_key: &'static str,
    /// Text in `wallets`.
    pub wallets_no_wallet_loaded: &'static str,
    /// Line in `wallets`.
    pub wallets_enter_keeps: &'static str,
    /// Button in `wallets`.
    pub wallets_rename: &'static str,
    /// Text in `wallets`.
    pub wallets_show_wallet: &'static str,
    /// Text in `wallets`.
    pub wallets_back_up: &'static str,
    /// Text in `wallets`.
    pub wallets_sign_a_message: &'static str,
    /// Text in `wallets`.
    pub wallets_remove_from: &'static str,
    /// Text in `wallets`. `{}`: `here`, `m`.
    pub wallets_of_signatures: &'static str,
    /// Text in `wallets`.
    pub wallets_can_spend: &'static str,
    /// Text in `wallets`.
    pub wallets_scan_a_key: &'static str,
    /// Line in `wallets`.
    pub wallets_at_a_glance: &'static str,
    /// Line in `wallets`.
    pub wallets_first_receive: &'static str,
    // ----- silent_wallet_card (faraday-core/src/screens.rs) -----
    /// Line in `silent_wallet_card`.
    pub silent_wallet_card_key: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_here: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_not: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_load: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_add: &'static str,
    /// Line in `silent_wallet_card`.
    pub silent_wallet_card_address_2: &'static str,
    /// Line in `silent_wallet_card`.
    pub silent_wallet_card_labels: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_address: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_show: &'static str,
    /// Text in `silent_wallet_card`.
    pub silent_wallet_card_remove: &'static str,
    // ----- spend (faraday-core/src/screens.rs) -----
    /// Line in `spend`.
    pub spend_no_transaction: &'static str,
    /// Button in `spend`.
    pub spend_open_files: &'static str,
    /// Text in `spend`.
    pub spend_wallets: &'static str,
    /// Text in `spend`.
    pub spend_sign_a_transaction: &'static str,
    // ----- spend_status (faraday-core/src/screens.rs) -----
    /// Text in `spend_status`. `{}`: `id + 1`.
    pub spend_status_share: &'static str,
    /// Text in `spend_status`.
    pub spend_status_carry: &'static str,
    /// Text in `spend_status`.
    pub spend_status_the_next: &'static str,
    /// Text in `spend_status`. `{}`: `app.medium.for_the()`.
    pub spend_status_both_wait: &'static str,
    /// Text in `spend_status`.
    pub spend_status_complete: &'static str,
    /// Text in `spend_status`. `{}`: `missing - 1`, `if missing == 2 { EN.spend_status_its…`.
    pub spend_status_sign_here_more: &'static str,
    /// Text in `spend_status`.
    pub spend_status_its: &'static str,
    /// Text in `spend_status`.
    pub spend_status_their: &'static str,
    /// Text in `spend_status`. `{}`: `missing - 1`.
    pub spend_status_sign_here_and: &'static str,
    /// Text in `spend_status`.
    pub spend_status_sign_here: &'static str,
    /// Text in `spend_status`.
    pub spend_status_keep_the: &'static str,
    /// Text in `spend_status`. `{}`: `app.medium.a()`.
    pub spend_status_insert: &'static str,
    /// Text in `spend_status`.
    pub spend_status_open_files: &'static str,
    /// Text in `spend_status`.
    pub spend_status_finish: &'static str,
    /// Text in `spend_status`. `{}`: `can_sign_here.join(", ")`.
    pub spend_status_sign_with: &'static str,
    // ----- spend_bar (faraday-core/src/screens.rs) -----
    /// Text in `spend_bar`. `{}`: `count`, `needed`.
    pub spend_bar_of: &'static str,
    // ----- step_summary (faraday-core/src/screens.rs) -----
    /// Text in `step_summary`.
    pub step_summary_no_loaded: &'static str,
    /// Text in `step_summary`.
    pub step_summary_addresses: &'static str,
    /// Text in `step_summary`. `{}`: `btc(i.amount_to_others.to_sat())`, `i.fee_rate_sat_vb .map(|r| format!("{…`.
    pub step_summary_sends: &'static str,
    /// Text in `step_summary`. `{}`: `here`, `needed`.
    pub step_summary_here_needed: &'static str,
    /// Text in `step_summary`.
    pub step_summary_not_signed: &'static str,
    /// Text in `step_summary`. `{}`: `fps.join(", ")`.
    pub step_summary_signed_with: &'static str,
    /// Text in `step_summary`. `{}`: `s.signers.len()`, `needed`.
    pub step_summary_of: &'static str,
    /// Text in `step_summary`. `{}`: `have`, `rows.len()`.
    pub step_summary_of_nonces: &'static str,
    /// Text in `step_summary`.
    pub step_summary_way_to: &'static str,
    /// Text in `step_summary`.
    pub step_summary_ways_to: &'static str,
    /// Text in `step_summary`.
    pub step_summary_signed: &'static str,
    /// Text in `step_summary`.
    pub step_summary_not_finished: &'static str,
    /// Text in `step_summary`.
    pub step_summary_not_reached: &'static str,
    // ----- step (faraday-core/src/screens.rs) -----
    /// Text in `step_body`. `{}`: `Session::shape(wl)`, `wl.source`.
    pub step_from: &'static str,
    /// Line in `step_body`.
    pub step_no_loaded_wallet: &'static str,
    /// Text in `step_body`.
    pub step_receive_0_0: &'static str,
    /// Text in `step_body`.
    pub step_receive_0_1: &'static str,
    /// Text in `step_body`.
    pub step_change_1_0: &'static str,
    /// Line in `step_body`.
    pub step_no_wallet_to_show: &'static str,
    /// Text in `step_body`.
    pub step_sends: &'static str,
    /// Text in `step_body`. `{}`: `if change { 1 } else { 0 }`, `index`.
    pub step_to_2: &'static str,
    /// Text in `step_body`.
    pub step_change: &'static str,
    /// Text in `step_body`. `{}`: `short(&app.session.shown(&o.address))`.
    pub step_to: &'static str,
    /// Text in `step_body`.
    pub step_unverified: &'static str,
    /// Text in `step_body`.
    pub step_fee: &'static str,
    /// Text in `step_body`. `{}`: `thousands(i.fee.to_sat())`, `i.fee_rate_sat_vb .map(|r| format!("{…`.
    pub step_sat_sat_vb: &'static str,
    /// Text in `step_body`.
    pub step_spends: &'static str,
    /// Text in `step_body`.
    pub step_amount_unchecked: &'static str,
    /// Text in `step_body`.
    pub step_unchanged_by_signing: &'static str,
    /// Text in `step_body`.
    pub step_changes_when_signed: &'static str,
    /// Text in `step_body`.
    pub step_txid: &'static str,
    /// Text in `step_body`.
    pub step_not_known_until: &'static str,
    /// Chip in `step_body`.
    pub step_no_warnings: &'static str,
    /// Line in `step_body`.
    pub step_input: &'static str,
    /// Line in `step_body`.
    pub step_output: &'static str,
    /// Text in `step_body`.
    pub step_hide_the_table: &'static str,
    /// Text in `step_body`.
    pub step_show_the_table: &'static str,
    /// Text in `step_body`. `{}`: `k + 1`.
    pub step_share_n: &'static str,
    /// Text in `step_body`.
    pub step_signed: &'static str,
    /// Text in `step_body`.
    pub step_signs_at_its_own: &'static str,
    /// Text in `step_body`.
    pub step_not_signing: &'static str,
    /// Text in `step_body`.
    pub step_signs_next: &'static str,
    /// Text in `step_body`.
    pub step_choose: &'static str,
    /// Text in `step_body`. `{}`: `needed`, `t.ours.len()`, `s.others.len()`.
    pub step_needs_here_chosen: &'static str,
    /// Text in `step_body`. `{}`: `needed`, `t.signed.len()`.
    pub step_needs_signed: &'static str,
    /// Text in `step_body`.
    pub step_load_from_vault: &'static str,
    /// Text in `step_body`.
    pub step_signs_here: &'static str,
    /// Text in `step_body`.
    pub step_not_here: &'static str,
    /// Text in `step_body`. `{}`: `f.name`.
    pub step_unlock: &'static str,
    /// Text in `step_body`.
    pub step_scan_a_key: &'static str,
    /// Text in `step_body`.
    pub step_collect_a_signature: &'static str,
    /// Text in `step_body`. `{}`: `app.medium.noun()`.
    pub step_remove_the_first: &'static str,
    /// Text in `step_body`.
    pub step_no_share_of_this: &'static str,
    /// Text in `step_body`. `{}`: `want`, `if want == 1 { EN.step_share_inline } else…`.
    pub step_choose_other_under: &'static str,
    /// Text in `step_body`.
    pub step_share_inline: &'static str,
    /// Text in `step_body`.
    pub step_shares: &'static str,
    /// Text in `step_body`. `{}`: `names.join(", ")`.
    pub step_sign_as: &'static str,
    /// Text in `step_body`. `{}`: `fps.join(", ")`.
    pub step_signed_with: &'static str,
    /// Line in `step_body`.
    pub step_no_key_in_this: &'static str,
    /// Text in `step_body`. `{}`: `mine.join(", ")`.
    pub step_sign_with: &'static str,
    /// Text in `step_body`.
    pub step_input_inline: &'static str,
    /// Text in `step_body`.
    pub step_inputs: &'static str,
    /// Text in `step_body`. `{}`: `src`.
    pub step_from_inline: &'static str,
    /// Text in `step_body`.
    pub step_signed_here: &'static str,
    /// Text in `step_body`.
    pub step_verified: &'static str,
    /// Text in `step_body`.
    pub step_not_needed: &'static str,
    /// Line in `step_body`.
    pub step_no_cosigner_s: &'static str,
    /// Text in `step_body`.
    pub step_scan_a_copy: &'static str,
    /// Text in `step_body`. `{}`: `id + 1`.
    pub step_share_n_inline: &'static str,
    /// Line in `step_body`.
    pub step_carry_file: &'static str,
    /// Text in `step_body`. `{}`: `if next_ids.is_empty() { EN.step_the_…`.
    pub step_for_holds_its: &'static str,
    /// Text in `step_body`.
    pub step_the_next_share: &'static str,
    /// Text in `step_body`.
    pub step_kept: &'static str,
    /// Text in `step_body`.
    pub step_keep_it: &'static str,
    /// Text in `step_body`. `{}`: `missing`, `if s.signers.is_empty() { "" } else {…`, `if missing == 1 { EN.step_signature }…`.
    pub step_needs: &'static str,
    /// Text in `step_body`, after a space.
    pub step_more: &'static str,
    /// Text in `step_body`.
    pub step_signature: &'static str,
    /// Text in `step_body`.
    pub step_signatures: &'static str,
    /// Text in `step_body`.
    pub step_signed_psbt: &'static str,
    /// Text in `step_body`.
    pub step_for_the_wallet: &'static str,
    /// Text in `step_body`.
    pub step_finished_transaction: &'static str,
    /// Text in `step_body`.
    pub step_ready_to_broadcast: &'static str,
    /// Text in `step_body`.
    pub step_remove: &'static str,
    /// Text in `step_body`.
    pub step_make_again: &'static str,
    /// Text in `step_body`. `{}`: `short(&tx.compute_txid().to_string())`, `tx.vsize()`.
    pub step_vb: &'static str,
    /// Text in `step_body`.
    pub step_decode_it: &'static str,
    /// Text in `step_body`.
    pub step_hide_the_raw_hex: &'static str,
    /// Text in `step_body`.
    pub step_show_the_raw_hex: &'static str,
    /// Line in `step_body`.
    pub step_this_device: &'static str,
    /// Text in `step_body`.
    pub step_nonce_here: &'static str,
    /// Text in `step_body`.
    pub step_waiting: &'static str,
    /// Text in `step_body`. `{}`: `name`.
    pub step_add_from: &'static str,
    /// Text in `step_body`.
    pub step_show_as_qr: &'static str,
    /// Button in `step_body`.
    pub step_go_first_share: &'static str,
    /// Text in `step_body`. `{}`: `k + 1`.
    pub step_path: &'static str,
    /// Text in `step_body`. `{}`: `slot`, `fp`.
    pub step_key: &'static str,
    /// Line in `step_body`.
    pub step_no_wait: &'static str,
    /// Text in `step_body`. `{}`: `l`.
    pub step_waits: &'static str,
    /// Text in `step_body`.
    pub step_this_transaction_s: &'static str,
    /// Text in `step_body`.
    pub step_this_transaction: &'static str,
    /// Text in `step_body`.
    pub step_continue: &'static str,
    // ----- spend_panel (faraday-core/src/screens.rs) -----
    /// Line in `spend_panel`.
    pub spend_panel_this_transaction: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_verified: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_unverified: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_none: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_wallet: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_not_loaded: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_sends: &'static str,
    /// Text in `spend_panel`. `{}`: `btc(i.amount_to_others.to_sat())`.
    pub spend_panel_btc: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_to: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_fee: &'static str,
    /// Text in `spend_panel`. `{}`: `thousands(i.fee.to_sat())`, `i.fee_rate_sat_vb .map(|r| format!("{…`.
    pub spend_panel_sat_sat: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_change: &'static str,
    /// Text in `spend_panel`.
    pub spend_panel_txid: &'static str,
    /// Line in `spend_panel`.
    pub spend_panel_signatures: &'static str,
    /// Text in `spend_panel`. `{}`: `count`, `needed`.
    pub spend_panel_of: &'static str,
    // ----- question_title (faraday-core/src/screens.rs) -----
    /// Text in `question_title`.
    pub question_title_start: &'static str,
    /// Text in `question_title`.
    pub question_title_the_seeds: &'static str,
    /// Text in `question_title`.
    pub question_title_places: &'static str,
    /// Text in `question_title`.
    pub question_title_the: &'static str,
    /// Text in `question_title`.
    pub question_title_software: &'static str,
    /// Text in `question_title`.
    pub question_title_passphrases: &'static str,
    /// Text in `question_title`.
    pub question_title_map: &'static str,
    // ----- ticked (faraday-core/src/screens.rs) -----
    /// Text in `ticked`.
    pub ticked_none: &'static str,
    // ----- item_title (faraday-core/src/screens.rs) -----
    /// Text in `item_title`. `{}`: `k`, `if k == 1 { EN.item_title_template_inline …`.
    pub item_title_print_blank: &'static str,
    /// Text in `item_title`.
    pub item_title_template_inline: &'static str,
    /// Text in `item_title`.
    pub item_title_templates: &'static str,
    /// Text in `item_title`. `{}`: `b.words`.
    pub item_title_template_n: &'static str,
    /// Text in `item_title`. `{}`: `shape.seeds.get(i).map_or("", |s| s.n…`.
    pub item_title_copy_seed: &'static str,
    /// Text in `item_title`.
    pub item_title_checked: &'static str,
    /// Text in `item_title`.
    pub item_title_not_checked: &'static str,
    /// Text in `item_title`. `{}`: `v + 1`.
    pub item_title_vault_2: &'static str,
    /// Text in `item_title`. `{}`: `v + 1`, `names.join(", ")`.
    pub item_title_vault: &'static str,
    /// Text in `item_title`. `{}`: `name`.
    pub item_title_in: &'static str,
    /// Text in `item_title`.
    pub item_title_not_saved: &'static str,
    /// Text in `item_title`.
    pub item_title_the_seeds: &'static str,
    /// Text in `item_title`.
    pub item_title_not_made: &'static str,
    /// Text in `item_title`.
    pub item_title_the_shares: &'static str,
    /// Text in `item_title`. `{}`: `keys_n`, `keys_n - a.omit.min(shape.m.saturatin…`.
    pub item_title_shares_keys: &'static str,
    /// Text in `item_title`.
    pub item_title_the_wallet: &'static str,
    /// Text in `item_title`.
    pub item_title_a_pdf_to: &'static str,
    /// Text in `item_title`.
    pub item_title_the_public: &'static str,
    /// Text in `item_title`.
    pub item_title_show_the: &'static str,
    /// Text in `item_title`.
    pub item_title_shown: &'static str,
    /// Text in `item_title`.
    pub item_title_not_shown: &'static str,
    /// Text in `item_title`.
    pub item_title_one_envelope: &'static str,
    /// Text in `item_title`.
    pub item_title_envelope: &'static str,
    /// Text in `item_title`.
    pub item_title_envelopes: &'static str,
    // ----- question_summary (faraday-core/src/screens.rs) -----
    /// Text in `question_summary`.
    pub question_summary_3: &'static str,
    /// Text in `question_summary`.
    pub question_summary_place: &'static str,
    /// Text in `question_summary`.
    pub question_summary_places: &'static str,
    /// Text in `question_summary`, after a space.
    pub question_summary_a_share_each: &'static str,
    /// Text in `question_summary`, after a space.
    pub question_summary_the_whole_sheet: &'static str,
    /// Text in `question_summary`.
    pub question_summary_passphrase: &'static str,
    /// Text in `question_summary`.
    pub question_summary_passphrases: &'static str,
    /// Text in `question_summary`. `{}`: `c.lost.text()`.
    pub question_summary_any: &'static str,
    // ----- backup (faraday-core/src/screens.rs) -----
    /// Text in `backup_screen`. `{}`: `t`.
    pub backup_after: &'static str,
    /// Text in `backup_screen`.
    pub backup_back_up_a_wallet: &'static str,
    /// Text in `backup_screen`. `{}`: `wallet.name`.
    pub backup_back_up: &'static str,
    /// Text in `backup_screen`.
    pub backup_vault_contents: &'static str,
    /// Text in `backup_screen`.
    pub backup_wallets: &'static str,
    /// Text in `backup_screen`.
    pub backup_then_wallets: &'static str,
    /// Text in `backup_screen`. `{}`: `v + 1`, `app_ref.medium.noun()`.
    pub backup_vault_s_needs: &'static str,
    /// Text in `backup_screen`.
    pub backup_make_the_checklist: &'static str,
    /// Text in `backup_screen`.
    pub backup_change_the_plan: &'static str,
    /// Line in `backup_body`.
    pub backup_words_on_the: &'static str,
    /// Row in `backup_body`.
    pub backup_blank_template: &'static str,
    /// Text in `backup_body`. `{}`: `b.words`.
    pub backup_words_a_pdf: &'static str,
    /// Line in `backup_body`.
    pub backup_no_key_of_this: &'static str,
    /// Text in `backup_body`.
    pub backup_hide_the_seed: &'static str,
    /// Text in `backup_body`.
    pub backup_show_the_seed: &'static str,
    /// Text in `backup_body`. `{}`: `key_line(key.master.fingerprint(), &k…`, `list.len()`.
    pub backup_words: &'static str,
    /// Line in `backup_body`.
    pub backup_seedqr_is_for: &'static str,
    /// Text in `backup_body`.
    pub backup_standard_seedqr: &'static str,
    /// Text in `backup_body`.
    pub backup_compact_seedqr: &'static str,
    /// Text in `backup_body`.
    pub backup_shown: &'static str,
    /// Text in `backup_body`.
    pub backup_not_shown: &'static str,
    /// Text in `backup_body`.
    pub backup_make_it: &'static str,
    /// Row in `backup_body`.
    pub backup_backup_sheet: &'static str,
    /// Row in `backup_body`.
    pub backup_a_pdf_to_print: &'static str,
    /// Text in `backup_body`.
    pub backup_make_them: &'static str,
    /// Row in `backup_body`.
    pub backup_the_shares: &'static str,
    /// Row in `backup_body`.
    pub backup_sheets_to_print: &'static str,
    /// Text in `backup_body`.
    pub backup_nothing: &'static str,
    /// Text in `backup_body`.
    pub backup_back_to_the: &'static str,
    /// Text in `backup_body`.
    pub backup_done: &'static str,
    // ----- plan (faraday-core/src/screens.rs) -----
    /// Line in `plan_body`.
    pub plan_a_key_made_here: &'static str,
    /// Text in `plan_body`.
    pub plan_places_with_paper: &'static str,
    /// Section label in `plan_body`.
    pub plan_each_place_keeps: &'static str,
    /// Text in `plan_body`.
    pub plan_the_whole_wallet: &'static str,
    /// Text in `plan_body`.
    pub plan_its_own_share: &'static str,
    /// Text in `plan_body`.
    pub plan_what_is_a_share: &'static str,
    /// Text in `plan_body`. `{}`: `v + 1`, `app.medium.noun()`.
    pub plan_vault_s: &'static str,
    /// Section label in `plan_body`.
    pub plan_form: &'static str,
    /// Text in `plan_body`.
    pub plan_qr_picture: &'static str,
    /// Text in `plan_body`.
    pub plan_text: &'static str,
    /// Text in `plan_body`. `{}`: `app.place_name(p)`.
    pub plan_its_words: &'static str,
    /// Text in `plan_body`.
    pub plan_in_the_vault_with: &'static str,
    /// Text in `plan_body`. `{}`: `v + 1`, `app.medium.noun()`.
    pub plan_vault_s_needs: &'static str,
    /// Button in `plan_body`.
    pub plan_make_the_checklist: &'static str,
    /// Button in `plan_body`.
    pub plan_continue: &'static str,
    // ----- place_names (faraday-core/src/screens.rs) -----
    /// Section label in `place_names`.
    pub place_names_names: &'static str,
    /// Line in `place_names`.
    pub place_names_only_in: &'static str,
    /// Text in `place_names`. `{}`: `open.name`.
    pub place_names_names_kept: &'static str,
    /// Text in `place_names`. `{}`: `p + 1`.
    pub place_names_place: &'static str,
    /// Text in `place_names`.
    pub place_names_done: &'static str,
    /// Text in `place_names`.
    pub place_names_name: &'static str,
    // ----- map_rows (faraday-core/src/screens.rs) -----
    /// Text in `map_rows`. `{}`: `app.medium.cap()`.
    pub map_rows_of_files: &'static str,
    /// Text in `map_rows`.
    pub map_rows_watch_only: &'static str,
    /// Text in `map_rows`.
    pub map_rows_on_its_own: &'static str,
    /// Line in `map_rows`.
    pub map_rows_nothing: &'static str,
    // ----- backup_done (faraday-core/src/screens.rs) -----
    /// Line in `backup_done`.
    pub backup_done_backup: &'static str,
    /// Text in `backup_done`. `{}`: `n`, `if n == 1 { EN.backup_done_file } els…`, `app.medium.for_the()`, `b.sent.len()`.
    pub backup_done_from_this: &'static str,
    /// Text in `backup_done`.
    pub backup_done_file: &'static str,
    /// Text in `backup_done`.
    pub backup_done_files: &'static str,
    /// Text in `backup_done`. `{}`: `app.medium.a()`.
    pub backup_done_write_to: &'static str,
    /// Text in `backup_done`.
    pub backup_done_open_the: &'static str,
    /// Text in `backup_done`.
    pub backup_done_change: &'static str,
    // ----- signers_count (faraday-core/src/screens.rs) -----
    /// Text in `signers_count`. `{}`: `here`, `needed`.
    pub signers_count_of_signatures_here: &'static str,
    /// Text in `signers_count`. `{}`: `here`, `needed`, `needed - here`.
    pub signers_count_of_signatures: &'static str,
    /// Text in `signers_count`.
    pub signers_count_can_spend: &'static str,
    /// Line in `signers_count`.
    pub signers_count_continue: &'static str,
    // ----- shares_card (faraday-core/src/screens.rs) -----
    /// Text in `shares_card`.
    pub shares_card_keys_left: &'static str,
    /// Text in `shares_card`. `{}`: `k + 1`.
    pub shares_card_share: &'static str,
    /// Text in `shares_card`. `{}`: `m`.
    pub shares_card_any_shares: &'static str,
    /// Text in `shares_card`. `{}`: `m`.
    pub shares_card_some_shares: &'static str,
    /// Text in `shares_card`. `{}`: `a.smallest_rebuild`.
    pub shares_card_the_fewest: &'static str,
    /// Text in `shares_card`. `{}`: `a.copies_per_key`.
    pub shares_card_each_key: &'static str,
    /// Text in `shares_card`.
    pub shares_card_no_single: &'static str,
    /// Text in `shares_card`.
    pub shares_card_each_share: &'static str,
    // ----- copy_check (faraday-core/src/screens.rs) -----
    /// Text in `copy_check`.
    pub copy_check_type_the: &'static str,
    /// Text in `copy_check`.
    pub copy_check_type_here: &'static str,
    /// Text in `copy_check`.
    pub copy_check_your_copy: &'static str,
    /// Text in `copy_check`. `{}`: `k`.
    pub copy_check_word_differs: &'static str,
    /// Text in `copy_check`. `{}`: `typed`, `of`.
    pub copy_check_of_digits: &'static str,
    /// Button in `copy_check`.
    pub copy_check_clear: &'static str,
    // ----- item_action (faraday-core/src/screens.rs) -----
    /// Text in `item_action`.
    pub item_action_continue: &'static str,
    /// Text in `item_action`.
    pub item_action_stop_typing: &'static str,
    /// Text in `item_action`.
    pub item_action_check_my: &'static str,
    /// Text in `item_action`.
    pub item_action_lock_it: &'static str,
    /// Text in `item_action`. `{}`: `open.name`.
    pub item_action_save_into: &'static str,
    /// Text in `item_action`.
    pub item_action_make_it: &'static str,
    /// Text in `item_action`.
    pub item_action_make_them: &'static str,
    /// Text in `item_action`.
    pub item_action_show_as: &'static str,
    // ----- vault_item (faraday-core/src/screens.rs) -----
    /// Text in `vault_item`. `{}`: `o.name`.
    pub vault_item_holds_another: &'static str,
    /// Text in `vault_item`. `{}`: `o.name`.
    pub vault_item_into: &'static str,
    /// Text in `vault_item`.
    pub vault_item_into_the: &'static str,
    /// Text in `vault_item`. `{}`: `name`.
    pub vault_item_seed_with: &'static str,
    /// Text in `vault_item`. `{}`: `name`.
    pub vault_item_seed: &'static str,
    /// Text in `vault_item`.
    pub vault_item_wallet_description: &'static str,
    // ----- seed_files (faraday-core/src/screens.rs) -----
    /// Text in `seed_files`.
    pub seed_files_not_made: &'static str,
    // ----- paper_section (faraday-core/src/screens.rs) -----
    /// Section label in `paper_section`.
    pub paper_section_other: &'static str,
    /// Text in `paper_section`.
    pub paper_section_seed_xor: &'static str,
    /// Text in `paper_section`.
    pub paper_section_seed: &'static str,
    /// Text in `paper_section`.
    pub paper_section_codex32_2: &'static str,
    /// Text in `paper_section`.
    pub paper_section_codex32: &'static str,
    /// Text in `paper_section`. `{}`: `parts.len()`.
    pub paper_section_parts: &'static str,
    /// Text in `paper_section`. `{}`: `strings.len()`, `k`.
    pub paper_section_shares: &'static str,
    /// Button in `paper_section`.
    pub paper_section_put_them: &'static str,
    // ----- seed_grid (faraday-core/src/screens.rs) -----
    /// Text in `seed_grid`. `{}`: `pin`.
    pub seed_grid_row: &'static str,
    // ----- backup_panel (faraday-core/src/screens.rs) -----
    /// Line in `backup_panel`.
    pub backup_panel_map: &'static str,
    // ----- kept_rows (faraday-core/src/screens.rs) -----
    /// Line in `kept_rows`.
    pub kept_rows_keys: &'static str,
    // ----- message (faraday-core/src/screens.rs) -----
    /// Text in `message_screen`.
    pub message_none_chosen: &'static str,
    /// Text in `message_screen`.
    pub message_not_written: &'static str,
    /// Text in `message_screen`.
    pub message_signed: &'static str,
    /// Text in `message_screen`.
    pub message_not_signed: &'static str,
    /// Text in `message_screen`.
    pub message_wallets: &'static str,
    /// Text in `message_screen`.
    pub message_sign_a_message: &'static str,
    /// Text in `message_body`. `{}`: `m.index`.
    pub message_receive_address: &'static str,
    /// Text in `message_body`.
    pub message_type_the_message: &'static str,
    /// Text in `message_body`.
    pub message_press_type: &'static str,
    /// Text in `message_body`.
    pub message_stop_typing: &'static str,
    /// Text in `message_body`.
    pub message_type: &'static str,
    /// Text in `message_body`. `{}`: `m.text.chars().count()`.
    pub message_characters: &'static str,
    /// Button in `message_body`.
    pub message_continue: &'static str,
    /// Button in `message_body`.
    pub message_sign_the_message: &'static str,
    /// Line in `message_body`.
    pub message_signature: &'static str,
    /// Text in `message_body`.
    pub message_make_the_file: &'static str,
    /// Text in `message_body`. `{}`: `app.medium.for_the()`.
    pub message_png: &'static str,
    /// Row in `message_body`.
    pub message_signed_message: &'static str,
    /// Text in `message_body`.
    pub message_show_as_qr: &'static str,
    // ----- check (faraday-core/src/screens.rs) -----
    /// Line in `check_screen`.
    pub check_wallets: &'static str,
    /// Title in `check_screen`.
    pub check_check_a_signed: &'static str,
    /// Text in `check_screen`.
    pub check_a_valid_signature: &'static str,
    /// Line in `check_screen`.
    pub check_no_signed_message: &'static str,
    /// Text in `check_screen`. `{}`: `app.session.network().name()`.
    pub check_not_a_signed: &'static str,
    /// Text in `check_screen`. `{}`: `if *f == osk_psbt::message::Format::B…`.
    pub check_the_signature_is: &'static str,
    /// Error in `check_screen`. `{}`: `e`.
    pub check_the_signature: &'static str,
    /// Text in `check_screen`.
    pub check_file: &'static str,
    /// Text in `check_screen`.
    pub check_address: &'static str,
    /// Text in `check_screen`.
    pub check_wallet: &'static str,
    /// Text in `check_screen`.
    pub check_change: &'static str,
    /// Text in `check_screen`.
    pub check_receive: &'static str,
    /// Text in `check_screen`.
    pub check_not_an_address: &'static str,
    /// Line in `check_screen`.
    pub check_message: &'static str,
    // ----- create (faraday-core/src/screens.rs) -----
    /// Text in `create_screen`. `{}`: `c.m`, `c.n`.
    pub create_of: &'static str,
    /// Text in `create_screen`. `{}`: `filled`, `c.slots.len()`, `c.n - c.m`.
    pub create_of_chosen_computed: &'static str,
    /// Text in `create_screen`. `{}`: `filled`, `c.slots.len()`.
    pub create_of_chosen: &'static str,
    /// Text in `create_screen`. `{}`: `filled - k`, `c.slots.len()`, `k`.
    pub create_of_chosen_to: &'static str,
    /// Text in `create_screen`. `{}`: `w.name`.
    pub create_first_addresses: &'static str,
    /// Text in `create_screen`.
    pub create_not_made_yet: &'static str,
    /// Text in `create_screen`.
    pub create_paper_vault: &'static str,
    /// Text in `create_screen`.
    pub create_home: &'static str,
    /// Text in `create_screen`.
    pub create_wallets: &'static str,
    /// Text in `create_screen`.
    pub create_make_a_new_wallet: &'static str,
    /// Text in `create_body`.
    pub create_start_over: &'static str,
    /// Text in `create_body`.
    pub create_signatures_needed: &'static str,
    /// Text in `create_body`.
    pub create_keys: &'static str,
    /// Text in `create_body`. `{}`: `c.n`.
    pub create_all_keys_sign: &'static str,
    /// Text in `create_body`. `{}`: `c.m`, `c.n`, `c.n - c.m`, `if c.n - c.m == 1 { EN.create_key_inline }…`, `c.m`.
    pub create_any_of_the_keys: &'static str,
    /// Text in `create_body`.
    pub create_key_inline: &'static str,
    /// Text in `create_body`.
    pub create_keys_inline: &'static str,
    /// Line in `create_body`.
    pub create_locked_in: &'static str,
    /// Text in `create_body`.
    pub create_not_chosen: &'static str,
    /// Text in `create_body`. `{}`: `fpt`, `name`.
    pub create_loaded_here: &'static str,
    /// Text in `create_body`. `{}`: `&t[1..9.min(t.len())]`.
    pub create_a_cosigner_s: &'static str,
    /// Text in `create_body`.
    pub create_a_cosigner_adds: &'static str,
    /// Text in `create_body`.
    pub create_new_key: &'static str,
    /// Text in `create_body`. `{}`: `name`.
    pub create_from: &'static str,
    /// Text in `create_body`.
    pub create_scan_its_xpub: &'static str,
    /// Text in `create_body`.
    pub create_someone_else: &'static str,
    /// Text in `create_body`.
    pub create_clear: &'static str,
    /// Text in `create_body`.
    pub create_show_xpub_qr: &'static str,
    /// Text in `create_body`. `{}`: `app.medium.for_the()`.
    pub create_xpub_png: &'static str,
    /// Text in `create_body`. `{}`: `app.medium.for_the()`.
    pub create_xpub_file: &'static str,
    /// Text in `create_body`.
    pub create_add_a_passphrase: &'static str,
    /// Label in `create_body`.
    pub create_passphrase: &'static str,
    /// Label in `create_body`.
    pub create_passphrase_again: &'static str,
    /// Button in `create_body`.
    pub create_lock_in: &'static str,
    /// Text in `create_body`. `{}`: `k + 1`.
    pub create_share_n: &'static str,
    /// Line in `create_body`.
    pub create_computed_by: &'static str,
    /// Text in `create_body`. `{}`: `if c.kind.threshold() { EN.create_sha…`, `slot + 1`.
    pub create_new_key_for: &'static str,
    /// Text in `create_body`.
    pub create_share: &'static str,
    /// Text in `create_body`.
    pub create_key: &'static str,
    /// Text in `create_body`.
    pub create_continue: &'static str,
    /// Text in `create_body`. `{}`: `app.session.wallets[old].name`.
    pub create_move_the_money: &'static str,
    /// Line in `create_body`.
    pub create_make_the_wallet: &'static str,
    // ----- kind_rows (faraday-core/src/screens.rs) -----
    /// Button in `kind_rows`.
    pub kind_rows_more_kinds: &'static str,
    // ----- first_addresses (faraday-core/src/screens.rs) -----
    /// Section label in `first_addresses`.
    pub first_addresses_the: &'static str,
    /// Text in `first_addresses`.
    pub first_addresses_receive_0: &'static str,
    /// Text in `first_addresses`.
    pub first_addresses_receive: &'static str,
    /// Text in `first_addresses`.
    pub first_addresses_change: &'static str,
    // ----- made_place (faraday-core/src/screens.rs) -----
    /// Text in `made_place`. `{}`: `r.label`.
    pub made_place_written: &'static str,
    // ----- made_line (faraday-core/src/screens.rs) -----
    /// Button in `made_line`.
    pub made_line_remove: &'static str,
    // ----- made_state (faraday-core/src/screens.rs) -----
    /// Text in `made_state`.
    pub made_state_remove: &'static str,
    /// Text in `made_state`.
    pub made_state_make_again: &'static str,
    // ----- locked_tag (faraday-core/src/screens.rs) -----
    /// Text in `locked_tag`.
    pub locked_tag_locked_in: &'static str,
    // ----- descriptor_row (faraday-core/src/screens.rs) -----
    /// Line in `descriptor_row`.
    pub descriptor_row_descriptor: &'static str,
    // ----- public_files (faraday-core/src/screens.rs) -----
    /// Text in `public_files`.
    pub public_files_show_as: &'static str,
    /// Text in `public_files`.
    pub public_files_descriptor: &'static str,
    /// Text in `public_files`.
    pub public_files_any_wallet: &'static str,
    /// Text in `public_files`.
    pub public_files_wallet: &'static str,
    /// Text in `public_files`.
    pub public_files_sparrow: &'static str,
    /// Text in `public_files`.
    pub public_files_multisig: &'static str,
    /// Text in `public_files`.
    pub public_files_coldcard: &'static str,
    /// Text in `public_files`.
    pub public_files_backup: &'static str,
    /// Text in `public_files`.
    pub public_files_a_pdf: &'static str,
    /// Text in `public_files`.
    pub public_files_bsms_descriptor: &'static str,
    /// Text in `public_files`.
    pub public_files_bip_129: &'static str,
    /// Text in `public_files`.
    pub public_files_bitcoin: &'static str,
    /// Text in `public_files`.
    pub public_files_importdescriptors: &'static str,
    /// Text in `public_files`. `{}`: `fp`.
    pub public_files_key: &'static str,
    /// Text in `public_files`.
    pub public_files_for_the: &'static str,
    /// Text in `public_files`. `{}`: `fp`.
    pub public_files_key_bsms: &'static str,
    /// Text in `public_files`.
    pub public_files_signed: &'static str,
    // ----- public_rows_for (faraday-core/src/screens.rs) -----
    /// Text in `public_rows_for`.
    pub public_rows_for_make: &'static str,
    // ----- restore (faraday-core/src/screens.rs) -----
    /// Text in `restore_screen`. `{}`: `s.m`, `s.n`.
    pub restore_of: &'static str,
    /// Text in `restore_screen`.
    pub restore_none: &'static str,
    /// Text in `restore_screen`. `{}`: `here`, `all`.
    pub restore_of_filled: &'static str,
    /// Text in `restore_screen`.
    pub restore_home: &'static str,
    /// Text in `restore_screen`.
    pub restore_restore_an: &'static str,
    /// Text in `restore_body`.
    pub restore_i_have_the: &'static str,
    /// Text in `restore_body`.
    pub restore_scan_it: &'static str,
    /// Text in `restore_body`.
    pub restore_from_files: &'static str,
    /// Text in `restore_body`.
    pub restore_continue: &'static str,
    /// Text in `restore_body`.
    pub restore_it_matches: &'static str,
    /// Line in `restore_body`.
    pub restore_make_the_wallet: &'static str,
    /// Text in `restore_body`.
    pub restore_back_up_this: &'static str,
    /// Text in `restore_body`.
    pub restore_wallets: &'static str,
    // ----- restore_description (faraday-core/src/screens.rs) -----
    /// Text in `restore_description`. `{}`: `n`.
    pub restore_description_from: &'static str,
    /// Text in `restore_description`.
    pub restore_description_i_have: &'static str,
    /// Text in `restore_description`.
    pub restore_description_scan: &'static str,
    /// Text in `restore_description`.
    pub restore_description_i: &'static str,
    /// Line in `restore_description`.
    pub restore_description_shares: &'static str,
    /// Text in `restore_description`. `{}`: `m.have.len()`, `m.n`, `m.have.join(" ")`.
    pub restore_description_of: &'static str,
    /// Button in `restore_description`.
    pub restore_description_rebuild: &'static str,
    // ----- restore_seeds (faraday-core/src/screens.rs) -----
    /// Text in `restore_seeds`. `{}`: `k + 1`.
    pub restore_seeds_key_of: &'static str,
    /// Button in `restore_seeds`.
    pub restore_seeds_change: &'static str,
    /// Button in `restore_seeds`.
    pub restore_seeds_not_here: &'static str,
    /// Text in `restore_seeds`.
    pub restore_seeds_here: &'static str,
    /// Text in `restore_seeds`. `{}`: `label`.
    pub restore_seeds_here_n: &'static str,
    /// Row in `restore_seeds`.
    pub restore_seeds_empty: &'static str,
    /// Text in `restore_seeds`.
    pub restore_seeds_its_xpub: &'static str,
    /// Text in `restore_seeds`.
    pub restore_seeds_make: &'static str,
    // ----- restore_open_slot (faraday-core/src/screens.rs) -----
    /// Text in `restore_open_slot`.
    pub restore_open_slot_type: &'static str,
    /// Text in `restore_open_slot`.
    pub restore_open_slot_scan: &'static str,
    // ----- restore_sources (faraday-core/src/screens.rs) -----
    /// Text in `restore_sources_compact`.
    pub restore_sources_copy_files: &'static str,
    /// Text in `restore_sources_compact`.
    pub restore_sources_not: &'static str,
    /// Text in `restore_sources_compact`. `{}`: `app.medium.noun()`.
    pub restore_sources_insert: &'static str,
    /// Line in `restore_sources_compact`.
    pub restore_sources_vault: &'static str,
    /// Text in `restore_sources_compact`. `{}`: `f.name`.
    pub restore_sources_unlock: &'static str,
    /// Text in `restore_sources_compact`.
    pub restore_sources_wallet: &'static str,
    /// Text in `restore_sources_compact`. `{}`: `app.medium.noun()`.
    pub restore_sources_copy: &'static str,
    // ----- kind_line (faraday-core/src/screens.rs) -----
    /// Text in `kind_line`. `{}`: `bytes as f32 / 1e6`.
    pub kind_line_mb: &'static str,
    /// Text in `kind_line`. `{}`: `bytes as f32 / 1e3`.
    pub kind_line_kb: &'static str,
    // ----- kind_name (faraday-core/src/screens.rs) -----
    /// Text in `kind_name`.
    pub kind_name_wallet_descriptor: &'static str,
    /// Text in `kind_name`.
    pub kind_name_finished: &'static str,
    /// Text in `kind_name`.
    pub kind_name_sheet_for: &'static str,
    /// Text in `kind_name`.
    pub kind_name_signed_message: &'static str,
    /// Text in `kind_name`.
    pub kind_name_account_xpub: &'static str,
    /// Text in `kind_name`.
    pub kind_name_seed_words: &'static str,
    /// Text in `kind_name`.
    pub kind_name_share_of: &'static str,
    /// Text in `kind_name`.
    pub kind_name_threshold: &'static str,
    /// Text in `kind_name`.
    pub kind_name_vault_locked: &'static str,
    /// Text in `kind_name`.
    pub kind_name_entries_for: &'static str,
    /// Text in `kind_name`.
    pub kind_name_opensigner: &'static str,
    /// Text in `kind_name`.
    pub kind_name_keepass_database: &'static str,
    /// Text in `kind_name`.
    pub kind_name_pdf_to_print: &'static str,
    /// Text in `kind_name`.
    pub kind_name_efi_image: &'static str,
    /// Text in `kind_name`.
    pub kind_name_part_of_a: &'static str,
    /// Text in `kind_name`.
    pub kind_name_text: &'static str,
    /// Text in `kind_name`.
    pub kind_name_file: &'static str,
    // ----- add_to_vault (faraday-core/src/screens.rs) -----
    /// Text in `add_to_vault`.
    pub add_to_vault_add_to: &'static str,
    // ----- restore_shares_button (faraday-core/src/screens.rs) -----
    /// Text in `restore_shares_button`. `{}`: `texts.len()`.
    pub restore_shares_button_restore: &'static str,
    /// Text in `restore_shares_button`. `{}`: `m.have.len()`, `m.n`.
    pub restore_shares_button_of: &'static str,
    // ----- files (faraday-core/src/screens.rs) -----
    /// Title in `files`.
    pub files_files: &'static str,
    /// Button in `files`.
    pub files_add_to_signatures: &'static str,
    /// Text in `files`.
    pub files_import_into_the: &'static str,
    /// Text in `files`.
    pub files_contents: &'static str,
    /// Text in `files`.
    pub files_unlock: &'static str,
    /// Button in `files`.
    pub files_sign: &'static str,
    /// Button in `files`.
    pub files_load_the_wallet: &'static str,
    /// Button in `files`.
    pub files_decode: &'static str,
    /// Button in `files`.
    pub files_load_this_key: &'static str,
    /// Button in `files`.
    pub files_check_the_signature: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.a()`.
    pub files_waiting_for: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.cap()`.
    pub files_attached_lock: &'static str,
    /// Text in `files_compact`. `{}`: `app.sticks[app.visit.stick.min(app.st…`.
    pub files_visit: &'static str,
    /// Button in `files_compact`.
    pub files_scan_a_qr_code: &'static str,
    /// Line in `files_compact`.
    pub files_in_memory_emptied: &'static str,
    /// Text in `files_compact`.
    pub files_locked: &'static str,
    /// Text in `files_compact`.
    pub files_open: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.a()`.
    pub files_empty_files_are: &'static str,
    /// Text in `files_compact`.
    pub files_public: &'static str,
    /// Text in `files_compact`.
    pub files_anyone_may_read: &'static str,
    /// Text in `files_compact`.
    pub files_written_sealed: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.a()`.
    pub files_sealed_under: &'static str,
    /// Text in `files_compact`.
    pub files_unprotected_secrets: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.noun()`.
    pub files_anyone_who_has: &'static str,
    /// Text in `files_compact`. `{}`: `v.changes`, `if v.changes == 1 { EN.files_change }…`.
    pub files_vault_open_unsaved: &'static str,
    /// Text in `files_compact`.
    pub files_change: &'static str,
    /// Text in `files_compact`.
    pub files_changes: &'static str,
    /// Text in `files_compact`.
    pub files_vault_open_sealed: &'static str,
    /// Text in `files_compact`.
    pub files_unprotected: &'static str,
    /// Text in `files_compact`.
    pub files_written_after: &'static str,
    /// Text in `files_compact`.
    pub files_ready: &'static str,
    /// Text in `files_compact`.
    pub files_make_pdf: &'static str,
    /// Text in `files_compact`.
    pub files_show_as_qr: &'static str,
    /// Text in `files_compact`.
    pub files_remove: &'static str,
    /// Line in `files_compact`.
    pub files_empty: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.for_the()`.
    pub files_power_off_asks: &'static str,
    /// Text in `files_compact`. `{}`: `app.medium.from_box()`, `app.medium.nouns()`, `app.medium.for_box()`.
    pub files_holds_what_was: &'static str,
    // ----- receipt_rows (faraday-core/src/screens.rs) -----
    /// Text in `receipt_rows`.
    pub receipt_rows_verified: &'static str,
    // ----- vault_panel (faraday-core/src/screens.rs) -----
    /// Text in `vault_panel`. `{}`: `open.name`, `open.label()`.
    pub vault_panel_open: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_select: &'static str,
    /// Line in `vault_panel`.
    pub vault_panel_wallets_with: &'static str,
    /// Text in `vault_panel`. `{}`: `here`, `wt.total`, `if wt.total == 1 { EN.vault_panel_key…`.
    pub vault_panel_of_in_this: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_ready_to: &'static str,
    /// Text in `vault_panel`. `{}`: `more`.
    pub vault_panel_more_listed: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_load_wallet: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_load: &'static str,
    /// Text in `vault_panel`. `{}`: `n`.
    pub vault_panel_load_wallets: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_hide_each: &'static str,
    /// Text in `vault_panel`. `{}`: `rows.len()`.
    pub vault_panel_each_seed: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_loaded: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_everything: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_nothing: &'static str,
    /// Text in `vault_panel`. `{}`: `count(wl, EN.vault_panel_wallet, EN.v…`.
    pub vault_panel_load_n: &'static str,
    /// Text in `vault_panel`. `{}`: `count(s, EN.vault_panel_key, EN.vault…`, `count(wl, EN.vault_panel_wallet, EN.v…`.
    pub vault_panel_load_and: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_key: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_keys: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_wallet: &'static str,
    /// Text in `vault_panel`.
    pub vault_panel_wallets: &'static str,
    /// Button in `vault_panel`.
    pub vault_panel_go_to_wallets: &'static str,
    // ----- inbox (faraday-core/src/screens.rs) -----
    /// Line in `inbox_panel_compact`.
    pub inbox_wallets: &'static str,
    /// Text in `inbox_panel_compact`. `{}`: `fw.shape`, `fw.seeds_here`, `n`, `if n == 1 { EN.inbox_key } else { EN.…`, `if fw.files.len() > 1 { format!(" · {…`.
    pub inbox_of_here: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_key: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_keys: &'static str,
    /// Line in `inbox_panel_compact`.
    pub inbox_ready_to_sign: &'static str,
    /// Text in `inbox_panel_compact`. `{}`: `wallets.len() - ROWS`.
    pub inbox_more_loaded_with: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_nothing_chosen: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_load_the_wallet_with: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_load_the_wallet: &'static str,
    /// Text in `inbox_panel_compact`. `{}`: `n`.
    pub inbox_load_wallets_with: &'static str,
    /// Text in `inbox_panel_compact`. `{}`: `n`.
    pub inbox_load_wallets: &'static str,
    /// Text in `inbox_panel_compact`. `{}`: `app.medium.noun()`.
    pub inbox_keys_load_once: &'static str,
    /// Line in `inbox_panel_compact`.
    pub inbox_keys_with_no: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_make_it_a_wallet: &'static str,
    /// Text in `inbox_panel_compact`. `{}`: `potential.len() - ROWS`.
    pub inbox_more: &'static str,
    /// Line in `inbox_panel_compact`.
    pub inbox_xpubs_with_no: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_no_origin: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_make_it_a_watch: &'static str,
    /// Line in `inbox_panel_compact`.
    pub inbox_not_complete: &'static str,
    /// Text in `inbox_panel_compact`.
    pub inbox_open_with_its: &'static str,
    // ----- vault (faraday-core/src/screens.rs) -----
    /// Text in `vault_panel_compact`. `{}`: `open.name`, `open.label()`.
    pub vault_open: &'static str,
    /// Line in `vault_panel_compact`.
    pub vault_wallets_with: &'static str,
    /// Text in `vault_panel_compact`. `{}`: `here`, `wt.total`, `if wt.total == 1 { EN.vault_key_inline } els…`.
    pub vault_of_in_this_vault: &'static str,
    /// Line in `vault_panel_compact`.
    pub vault_ready_to_sign: &'static str,
    /// Text in `vault_panel_compact`. `{}`: `more`.
    pub vault_more_listed_under: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_load_wallet_with: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_load_1_wallet: &'static str,
    /// Text in `vault_panel_compact`. `{}`: `n`.
    pub vault_load_wallets: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_hide_each_seed: &'static str,
    /// Text in `vault_panel_compact`. `{}`: `rows.len()`.
    pub vault_each_seed_and: &'static str,
    /// Line in `vault_panel_compact`.
    pub vault_select_all: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_loaded: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_everything_is: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_nothing_chosen: &'static str,
    /// Text in `vault_panel_compact`. `{}`: `count(wl, EN.vault_wallet_inline, EN.vault_w…`.
    pub vault_load_n: &'static str,
    /// Text in `vault_panel_compact`. `{}`: `count(s, EN.vault_key_inline, EN.vault_keys)`, `count(wl, EN.vault_wallet_inline, EN.vault_w…`.
    pub vault_load_and: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_key_inline: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_keys: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_wallet_inline: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_wallets: &'static str,
    /// Text in `vault_panel_compact`.
    pub vault_go_to_wallets: &'static str,
    // ----- inbox_actions (faraday-core/src/screens.rs) -----
    /// Text in `inbox_actions`.
    pub inbox_actions_add_to_vault: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_add_to: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_import: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_contents: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_unlock: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_sign: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_load_the: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_decode: &'static str,
    /// Text in `inbox_actions`. `{}`: `texts.len()`.
    pub inbox_actions_restore: &'static str,
    /// Note in `inbox_actions`. `{}`: `m.have.len()`, `m.n`.
    pub inbox_actions_of_keys: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_load: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_check: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_make: &'static str,
    /// Text in `inbox_actions`.
    pub inbox_actions_remove: &'static str,
    // ----- visit (faraday-core/src/screens.rs) -----
    /// Text in `visit_compact`.
    pub visit_unselect_all: &'static str,
    /// Line in `visit_compact`.
    pub visit_select_all: &'static str,
    /// Text in `visit`. `{}`: `app.medium.noun()`, `app.medium.noun()`, `app.medium.noun()`.
    pub visit_write_what_waits: &'static str,
    /// Text in `visit`. `{}`: `app.medium.noun()`.
    pub visit_no_attached: &'static str,
    /// Text in `visit`. `{}`: `app.medium.noun()`.
    pub visit_write_to_the: &'static str,
    /// Text in `visit`.
    pub visit_public: &'static str,
    /// Text in `visit`.
    pub visit_sealed: &'static str,
    /// Text in `visit`.
    pub visit_unprotected_secret: &'static str,
    /// Text in `visit`. `{}`: `app.medium.for_the()`.
    pub visit_nothing_waits: &'static str,
    /// Text in `visit`. `{}`: `nout`, `if nout == 1 { EN.visit_file } else {…`.
    pub visit_write: &'static str,
    /// Text in `visit`. `{}`: `app.medium.from_the()`.
    pub visit_copy: &'static str,
    /// Text in `visit`.
    pub visit_larger_than_18: &'static str,
    /// Text in `visit`.
    pub visit_settings: &'static str,
    /// Text in `visit`. `{}`: `app.medium.noun()`.
    pub visit_no_files_on_this: &'static str,
    /// Button in `visit`.
    pub visit_delete_files: &'static str,
    /// Text in `visit`.
    pub visit_import_and_load: &'static str,
    /// Text in `visit`. `{}`: `nin`, `if nin == 1 { EN.visit_file } else { …`.
    pub visit_import: &'static str,
    /// Text in `visit`.
    pub visit_file: &'static str,
    /// Text in `visit`.
    pub visit_files: &'static str,
    /// Text in `visit`.
    pub visit_open_files: &'static str,
    // ----- visit_inbox_line (faraday-core/src/screens.rs) -----
    /// Text in `visit_inbox_line`. `{}`: `kind_line(FileKind::Other, item.bytes…`.
    pub visit_inbox_line_png_no: &'static str,
    /// Text in `visit_inbox_line`. `{}`: `kind_line(k, item.bytes.len())`.
    pub visit_inbox_line_png: &'static str,
    // ----- visit_inbox_tag (faraday-core/src/screens.rs) -----
    /// Text in `visit_inbox_tag`.
    pub visit_inbox_tag_sealed: &'static str,
    /// Text in `visit_inbox_tag`.
    pub visit_inbox_tag_public: &'static str,
    /// Text in `visit_inbox_tag`.
    pub visit_inbox_tag_may: &'static str,
    /// Text in `visit_inbox_tag`.
    pub visit_inbox_tag_secret: &'static str,
    // ----- visit_select_all (faraday-core/src/screens.rs) -----
    /// Text in `visit_select_all`.
    pub visit_select_all_select: &'static str,
    // ----- visit_unselect_all (faraday-core/src/screens.rs) -----
    /// Text in `visit_unselect_all`.
    pub visit_unselect_all_unselect: &'static str,
    // ----- visit_list_bar (faraday-core/src/screens.rs) -----
    /// Text in `visit_list_bar`. `{}`: `(below / ROW).ceil() as usize`.
    pub visit_list_bar_more: &'static str,
    // ----- pull_line (faraday-core/src/screens.rs) -----
    /// Text in `pull_line`. `{}`: `noun`, `v`.
    pub pull_line_remove_the_to: &'static str,
    /// Text in `pull_line`. `{}`: `noun`.
    pub pull_line_remove_the: &'static str,
    /// Text in `pull_line`. `{}`: `noun`.
    pub pull_line_1_key_copied: &'static str,
    /// Text in `pull_line`. `{}`: `n`, `noun`.
    pub pull_line_keys_copied: &'static str,
    // ----- entry (faraday-core/src/screens.rs) -----
    /// Text in `entry`.
    pub entry_spend: &'static str,
    /// Text in `entry`.
    pub entry_wallets: &'static str,
    /// Text in `entry`. `{}`: `fp_text(osk_bip::keys::Fingerprint(fp))`.
    pub entry_add_key: &'static str,
    /// Text in `entry`. `{}`: `name`.
    pub entry_add_a_key_for: &'static str,
    /// Text in `entry`. `{}`: `app.entry.form.name()`.
    pub entry_add_a_key_n: &'static str,
    /// Text in `entry`.
    pub entry_add_a_key: &'static str,
    /// Text in `entry`.
    pub entry_other_languages: &'static str,
    /// Text in `entry`.
    pub entry_other_forms: &'static str,
    /// Button in `entry`.
    pub entry_scan_a_seedqr: &'static str,
    /// Text in `entry`.
    pub entry_type_the_words: &'static str,
    /// Text in `entry`.
    pub entry_no_word_starts: &'static str,
    /// Text in `entry`, after the candidates and three spaces.
    pub entry_tab_completes: &'static str,
    // ----- word_count_row (faraday-core/src/screens.rs) -----
    /// Line in `word_count_row`.
    pub word_count_row_words: &'static str,
    // ----- keyed_words (faraday-core/src/screens.rs) -----
    /// Text in `keyed_words`.
    pub keyed_words_type_each_word_s: &'static str,
    /// Text in `keyed_words`.
    pub keyed_words_type_each_word: &'static str,
    /// Text in `keyed_words`.
    pub keyed_words_type_each: &'static str,
    /// Text in `keyed_words`.
    pub keyed_words_type_the: &'static str,
    /// Text in `keyed_words`.
    pub keyed_words_now_its: &'static str,
    /// Text in `keyed_words`.
    pub keyed_words_no_word: &'static str,
    /// Line in `keyed_words`.
    pub keyed_words_press_a: &'static str,
    // ----- word_keys (faraday-core/src/screens.rs) -----
    /// Button in `word_keys`.
    pub word_keys_shift: &'static str,
    // ----- keyed_status (faraday-core/src/screens.rs) -----
    /// Text in `keyed_status`. `{}`: `n`.
    pub keyed_status_words_2: &'static str,
    /// Text in `keyed_status`. `{}`: `n`, `fp_text(fp)`.
    pub keyed_status_words: &'static str,
    // ----- entry_status (faraday-core/src/screens.rs) -----
    /// Text in `entry_status`. `{}`: `n`, `fp_text(fp)`.
    pub entry_status_words_checksum: &'static str,
    /// Text in `entry_status`. `{}`: `n`.
    pub entry_status_words: &'static str,
    // ----- entry_finish (faraday-core/src/screens.rs) -----
    /// Line in `entry_finish`.
    pub entry_finish_bip_39: &'static str,
    /// Button in `entry_finish`.
    pub entry_finish_add_key: &'static str,
    /// Button in `entry_finish`.
    pub entry_finish_clear: &'static str,
    /// Button in `entry_finish`.
    pub entry_finish_make_a: &'static str,
    /// Line in `entry_finish`.
    pub entry_finish_type_on: &'static str,
    // ----- entry_stick (faraday-core/src/screens.rs) -----
    /// Text in `entry_stick`. `{}`: `app.medium.a()`.
    pub entry_stick_from_plug: &'static str,
    /// Button in `entry_stick`.
    pub entry_stick_copy_files: &'static str,
    // ----- entry_form (faraday-core/src/screens.rs) -----
    /// Text in `entry_form`.
    pub entry_form_type_one_share: &'static str,
    /// Text in `entry_form`.
    pub entry_form_type_one_codex32: &'static str,
    /// Text in `entry_form`.
    pub entry_form_type_one: &'static str,
    /// Text in `entry_form`.
    pub entry_form_words_of: &'static str,
    /// Text in `entry_form`.
    pub entry_form_nothing: &'static str,
    /// Text in `entry_form`.
    pub entry_form_1_in: &'static str,
    /// Text in `entry_form`. `{}`: `n`.
    pub entry_form_in: &'static str,
    /// Line in `entry_form`.
    pub entry_form_passphrase: &'static str,
    /// Button in `entry_form`.
    pub entry_form_add_this: &'static str,
    /// Button in `entry_form`.
    pub entry_form_scan_a_qr: &'static str,
    /// Button in `entry_form`.
    pub entry_form_make_the: &'static str,
    /// Line in `entry_form`.
    pub entry_form_enter_adds: &'static str,
    // ----- tz_row (faraday-core/src/screens.rs) -----
    /// Line in `tz_row`.
    pub tz_row_time_zone: &'static str,
    // ----- time_section (faraday-core/src/screens.rs) -----
    /// Line in `time_section`.
    pub time_section_time: &'static str,
    /// Line in `time_section`.
    pub time_section_now: &'static str,
    /// Button in `time_section`.
    pub time_section_set_the: &'static str,
    // ----- set_time (faraday-core/src/screens.rs) -----
    /// Title in `set_time_screen`.
    pub set_time_set_the_time: &'static str,
    /// Text in `set_time_compact`.
    pub set_time_now: &'static str,
    /// Section label in `set_time_compact`.
    pub set_time_date: &'static str,
    /// Section label in `set_time_compact`.
    pub set_time_time: &'static str,
    /// Text in `set_time_compact`.
    pub set_time_set: &'static str,
    /// Text in `set_time_compact`.
    pub set_time_keep_this: &'static str,
    /// Text in `set_time_compact`.
    pub set_time_back: &'static str,
    // ----- settings (faraday-core/src/screens.rs) -----
    /// Text in `settings_compact`.
    pub settings_time: &'static str,
    /// Line in `settings_compact`.
    pub settings_now: &'static str,
    /// Button in `settings_compact`.
    pub settings_set_the_time: &'static str,
    /// Title in `settings`.
    pub settings_settings: &'static str,
    /// Line in `settings`.
    pub settings_session: &'static str,
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_key } else { …`.
    pub settings_loaded_lost: &'static str,
    /// Text in `settings`.
    pub settings_key: &'static str,
    /// Text in `settings`.
    pub settings_keys: &'static str,
    /// Button in `settings`.
    pub settings_lock: &'static str,
    /// Button in `settings`.
    pub settings_remove_every: &'static str,
    /// Button in `settings`.
    pub settings_power_off_2: &'static str,
    /// Text in `settings`.
    pub settings_lock_after: &'static str,
    /// Text in `settings`.
    pub settings_power_off: &'static str,
    /// Text in `settings`.
    pub settings_never: &'static str,
    /// Text in `settings`. `{}`: `m`.
    pub settings_min: &'static str,
    /// Text in `settings`. `{}`: `names.join(", ")`.
    pub settings_ignored_until: &'static str,
    /// Line in `settings`.
    pub settings_network: &'static str,
    /// Line in `settings`.
    pub settings_appearance: &'static str,
    /// Line in `settings`.
    pub settings_motion: &'static str,
    /// Text in `settings`.
    pub settings_full: &'static str,
    /// Text in `settings`.
    pub settings_reduced: &'static str,
    /// Line in `settings`.
    pub settings_display_scale: &'static str,
    /// Text in `settings`.
    pub settings_auto: &'static str,
    /// Line in `settings`.
    pub settings_signed_amounts: &'static str,
    /// Text in `settings`. `{}`: `n`, `if n == 1 { EN.settings_transaction }…`.
    pub settings_remembered: &'static str,
    /// Text in `settings`.
    pub settings_transaction: &'static str,
    /// Text in `settings`.
    pub settings_transactions: &'static str,
    /// Text in `settings`.
    pub settings_sealed_into: &'static str,
    /// Text in `settings`.
    pub settings_kept_until: &'static str,
    /// Line in `settings`.
    pub settings_about: &'static str,
    /// Text in `settings`. `{}`: `crate::version_label()`.
    pub settings_faraday: &'static str,
    /// Text in `settings`.
    pub settings_keys_are_typed: &'static str,
    /// Text in `settings`. `{}`: `app.medium.noun()`, `app.medium.a()`.
    pub settings_vaults_files: &'static str,
    /// Text in `settings`. `{}`: `app.medium.caps()`.
    pub settings_are_folders: &'static str,
    /// Text in `settings`. `{}`: `app.medium.caps()`.
    pub settings_are_read_by: &'static str,
    /// Text in `settings`.
    pub settings_not_run: &'static str,
    // ----- institute_note (faraday-core/src/screens.rs) -----
    /// Text in `institute_note`. `{}`: `SITE`.
    pub institute_note_visit: &'static str,
    // ----- pull (faraday-core/src/screens.rs) -----
    /// Text in `pull_sheet`.
    pub pull_go_on: &'static str,
    /// Text in `pull_sheet`. `{}`: `app.medium.noun()`, `what`.
    pub pull_pull_the_to: &'static str,
    /// Text in `pull_sheet`. `{}`: `app.sticks.first().map_or("", |s| s.l…`.
    pub pull_attached_2: &'static str,
    /// Text in `pull_sheet`. `{}`: `k`, `app.medium.nouns()`.
    pub pull_attached: &'static str,
    /// Button in `pull_sheet`.
    pub pull_cancel: &'static str,
    // ----- kept_sections (faraday-core/src/screens.rs) -----
    /// Text in `kept_sections`.
    pub kept_sections_kept: &'static str,
    /// Text in `kept_sections`.
    pub kept_sections_wiped: &'static str,
    /// Text in `kept_sections`.
    pub kept_sections_then: &'static str,
    // ----- lock (faraday-core/src/screens.rs) -----
    /// Text in `lock_sheet`.
    pub lock_a_fresh_start_and: &'static str,
    /// Text in `lock_sheet`. `{}`: `app.medium.noun()`.
    pub lock_a_fresh_start: &'static str,
    /// Text in `lock_sheet`.
    pub lock_lock_and_upgrade: &'static str,
    /// Text in `lock_sheet`.
    pub lock_the_upgrade_runs: &'static str,
    /// Text in `lock_sheet`. `{}`: `app.medium.a_cap()`.
    pub lock_is_attached: &'static str,
    /// Text in `lock_sheet`. `{}`: `app.medium.noun()`.
    pub lock_lock_and_use: &'static str,
    /// Text in `lock_sheet`. `{}`: `label`.
    pub lock_nothing_has_been: &'static str,
    /// Text in `lock_sheet`.
    pub lock_not_now: &'static str,
    // ----- write_out (faraday-core/src/screens.rs) -----
    /// Text in `write_out_sheet`. `{}`: `app.medium.a()`.
    pub write_out_lock_plug: &'static str,
    /// Text in `write_out_sheet`. `{}`: `app.medium.a()`.
    pub write_out_write_to: &'static str,
    /// Text in `write_out_sheet`.
    pub write_out_not_now: &'static str,
    /// Text in `write_out_sheet`.
    pub write_out_lock: &'static str,
    // ----- scan (faraday-core/src/screens.rs) -----
    /// Text in `scan_sheet`. `{}`: `wl.name`.
    pub scan_scan_a_key_for: &'static str,
    /// Text in `scan_sheet`.
    pub scan_scan_what_you: &'static str,
    /// Text in `scan_sheet`.
    pub scan_scan_a_seedqr: &'static str,
    /// Text in `scan_sheet`.
    pub scan_scan_your_copy: &'static str,
    /// Text in `scan_sheet`.
    pub scan_receive_into_downloads: &'static str,
    /// Text in `scan_sheet`.
    pub scan_scan_a_qr_code: &'static str,
    /// Text in `scan_sheet`.
    pub scan_hold_the_code: &'static str,
    /// Line in `scan_sheet`.
    pub scan_starting_the_camera: &'static str,
    /// Text in `scan_sheet`.
    pub scan_type_the_words: &'static str,
    /// Text in `scan_sheet`.
    pub scan_done: &'static str,
    /// Text in `scan_sheet`.
    pub scan_cancel: &'static str,
    // ----- qr (faraday-core/src/screens.rs) -----
    /// Text in `qr_sheet`. `{}`: `q.subtitle`, `q.frame + 1`, `q.frames.len()`.
    pub qr_frame_of: &'static str,
    /// Text in `qr_sheet`.
    pub qr_secret_whoever_scans: &'static str,
    /// Text in `qr_sheet`.
    pub qr_public_safe_to_scan: &'static str,
    /// Text in `qr_sheet`.
    pub qr_ur: &'static str,
    /// Text in `qr_sheet`.
    pub qr_bbqr: &'static str,
    /// Text in `qr_sheet`.
    pub qr_parts: &'static str,
    /// Text in `qr_sheet`. `{}`: `app.medium.for_the()`.
    pub qr_png: &'static str,
    /// Text in `qr_sheet`.
    pub qr_small: &'static str,
    /// Text in `qr_sheet`.
    pub qr_medium: &'static str,
    /// Text in `qr_sheet`.
    pub qr_large: &'static str,
    /// Button in `qr_sheet`.
    pub qr_done: &'static str,
    // ----- qr_sheet (faraday-core/src/screens.rs) -----
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_ur: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_bbqr: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_parts: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_small: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_medium: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_large: &'static str,
    /// Text in `qr_sheet_compact`. `{}`: `q.title`, `q.frame + 1`, `q.frames.len()`.
    pub qr_sheet_of: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_secret_whoever: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_public_safe: &'static str,
    /// Text in `qr_sheet_compact`.
    pub qr_sheet_done: &'static str,
    // ----- new_input (faraday-core/src/screens.rs) -----
    /// Text in `new_input_sheet`.
    pub new_input_a_new_keyboard_and: &'static str,
    /// Text in `new_input_sheet`.
    pub new_input_a_new_keyboard: &'static str,
    /// Text in `new_input_sheet`.
    pub new_input_a_new_pointer: &'static str,
    /// Line in `new_input_sheet`.
    pub new_input_the_name: &'static str,
    /// Line in `new_input_sheet`.
    pub new_input_type_this: &'static str,
    /// Line in `new_input_sheet`, in place of the code while the system
    /// has given no randomness.
    pub new_input_no_code: &'static str,
    /// Button in `new_input_sheet`.
    pub new_input_ignore_this: &'static str,
    /// Button in `new_input_sheet`.
    pub new_input_use_this: &'static str,
    // ----- idle_warn (faraday-core/src/screens.rs) -----
    /// Text in `idle_warn_sheet`.
    pub idle_warn_wiped: &'static str,
    /// Text in `idle_warn_sheet`. `{}`: `keys`, `if keys == 1 { EN.idle_warn_key } els…`, `wallets`, `if wallets == 1 { EN.idle_warn_wallet…`, `app.vaults.open.len()`, `if app.vaults.open.len() == 1 { EN.id…`.
    pub idle_warn_open: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_key: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_keys: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_wallet: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_wallets: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_vault: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_vaults: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_sealed: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_kept: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_no_power: &'static str,
    /// Text in `idle_warn_sheet`. `{}`: `m`, `app.medium.for_the()`.
    pub idle_warn_power_off: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_then: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_minute: &'static str,
    /// Text in `idle_warn_sheet`.
    pub idle_warn_minutes: &'static str,
    /// Text in `idle_warn_sheet`. `{}`: `minutes(app.idle_warn_min)`, `minutes(u64::from(app.idle_lock_min))`.
    pub idle_warn_no_input: &'static str,
    /// Line in `idle_warn_sheet`.
    pub idle_warn_locking_soon: &'static str,
    /// Button in `idle_warn_sheet`.
    pub idle_warn_keep_working: &'static str,
    // ----- locked (faraday-core/src/screens.rs) -----
    /// Line in `locked_sheet`.
    pub locked_locked: &'static str,
    /// Line in `locked_sheet`.
    pub locked_locked_after: &'static str,
    /// Text in `locked_sheet`. `{}`: `clock_text(off.saturating_sub(app.idl…`.
    pub locked_powers_off_in: &'static str,
    /// Text in `locked_sheet`. `{}`: `app.outbox.len()`, `app.medium.for_the()`.
    pub locked_no_power_off: &'static str,
    /// Button in `locked_sheet`.
    pub locked_continue: &'static str,
    // ----- network_row_note (faraday-core/src/screens.rs) -----
    /// Text in `network_row_note`.
    pub network_row_note_wallets: &'static str,
    // ----- not_airgapped (faraday-core/src/screens.rs) -----
    /// Text in `not_airgapped_sheet`.
    pub not_airgapped_cancel: &'static str,
    /// Text in `not_airgapped_sheet`.
    pub not_airgapped_i_understand: &'static str,
    /// Line in `not_airgapped_sheet`.
    pub not_airgapped_not_air: &'static str,
    // ----- power (faraday-core/src/screens.rs) -----
    /// Text in `power_sheet`.
    pub power_the_computer: &'static str,
    /// Text in `power_sheet`.
    pub power_cancel: &'static str,
    /// Text in `power_sheet`.
    pub power_power_off: &'static str,
    // ----- learn (faraday-core/src/screens.rs) -----
    /// Button in `learn_sheet`.
    pub learn_done: &'static str,
    /// Text in `learn_sheet`.
    pub learn_spending_step: &'static str,
    // ----- secret_out (faraday-core/src/screens.rs) -----
    /// Text in `secret_out_sheet`.
    pub secret_out_this_may: &'static str,
    /// Text in `secret_out_sheet`.
    pub secret_out_this_is: &'static str,
    /// Text in `secret_out_sheet`. `{}`: `app.medium.for_the()`.
    pub secret_out_put_it_unprotected: &'static str,
    /// Text in `secret_out_sheet`. `{}`: `app.medium.noun()`.
    pub secret_out_unprotected_on: &'static str,
    /// Text in `secret_out_sheet`.
    pub secret_out_tick_it: &'static str,
    /// Text in `secret_out_sheet`. `{}`: `app.medium.for_the()`.
    pub secret_out_unprotected: &'static str,
    /// Text in `secret_out_sheet`.
    pub secret_out_file: &'static str,
    /// Text in `secret_out_sheet`.
    pub secret_out_what_it: &'static str,
    /// Text in `secret_out_sheet`.
    pub secret_out_who_can: &'static str,
    /// Line in `secret_out_sheet`.
    pub secret_out_sealed_in: &'static str,
    /// Text in `secret_out_sheet`. `{}`: `name`.
    pub secret_out_in: &'static str,
    /// Text in `secret_out_sheet`. `{}`: `name`.
    pub secret_out_save_into: &'static str,
    /// Button in `secret_out_sheet`.
    pub secret_out_cancel: &'static str,
    // ----- secrets (faraday-core/src/secrets.rs) -----
    /// Text in `text`. `{}`: `noun`.
    pub secrets_anyone_who: &'static str,
    /// Text in `text`. `{}`: `noun`.
    pub secrets_i_understand: &'static str,
    /// Text in `text`. `{}`: `noun`.
    pub secrets_faraday_cannot: &'static str,
    /// Text in `offer_seed`.
    pub secrets_words: &'static str,
    /// Text in `offer_seed`. `{}`: `fps`.
    pub secrets_seed: &'static str,
    /// Text in `offer_seed`. `{}`: `m.word_count()`.
    pub secrets_words_secret: &'static str,
    /// Text in `offer_seed`.
    pub secrets_compact_seedqr: &'static str,
    /// Text in `offer_seed`.
    pub secrets_seedqr_png: &'static str,
    /// Text in `offer_seed`.
    pub secrets_whoever_has_it: &'static str,
    /// Text in `offer_seed`. `{}`: `fps`, `more`, `if more == 1 { EN.secrets_key } else …`.
    pub secrets_whoever_has: &'static str,
    /// Text in `offer_seed`.
    pub secrets_key: &'static str,
    /// Text in `offer_seed`.
    pub secrets_keys: &'static str,
    /// Text in `offer_seed`.
    pub secrets_a_wallet_s: &'static str,
    /// Toast in `secret_to_vault`.
    pub secrets_no_vault_is: &'static str,
    /// Text in `secret_to_vault`. `{}`: `vault`, `r.psbt_name`, `self.medium.for_the()`.
    pub secrets_the_nonce_is: &'static str,
    /// Toast in `secret_to_vault`. `{}`: `out.name`, `vault`.
    pub secrets_is_in: &'static str,
    /// Text in `secret_unprotected`. `{}`: `name`, `self.medium.for_the()`.
    pub secrets_waits_unprotected: &'static str,
    // ----- secureboot (faraday-core/src/secureboot.rs) -----
    /// Toast in `sb_make`.
    pub secureboot_no_randomness: &'static str,
    /// Toast in `sb_make`.
    pub secureboot_pk_kek_and: &'static str,
    /// Toast in `sb_enrol`.
    pub secureboot_this_computer: &'static str,
    /// Toast in `sb_enrol`. `{}`: `n`.
    pub secureboot_enrolment: &'static str,
    /// Toast in `sb_sign`. `{}`: `name`.
    pub secureboot_signed: &'static str,
    /// Text in `sb_check`.
    pub secureboot_signed_by_this: &'static str,
    /// Text in `sb_check`.
    pub secureboot_signed_by: &'static str,
    /// Text in `sb_check`.
    pub secureboot_changed: &'static str,
    /// Text in `sb_check`.
    pub secureboot_not_signed: &'static str,
    /// Text in `sb_check`.
    pub secureboot_its_signature: &'static str,
    // ----- seed_read (faraday-core/src/seed_read.rs) -----
    /// Error in `seed_read_act`.
    pub seed_read_type_the: &'static str,
    /// Text in `seed_read_wallet`. `{}`: `n + 1`.
    pub seed_read_key_already: &'static str,
    /// Text in `seed_read_take`. `{}`: `k.label`.
    pub seed_read_passphrase_3: &'static str,
    /// Text in `sheet`.
    pub seed_read_not_this: &'static str,
    /// Text in `sheet`.
    pub seed_read_cancel: &'static str,
    /// Text in `sheet`.
    pub seed_read_use_with: &'static str,
    /// Text in `sheet`.
    pub seed_read_use_without: &'static str,
    /// Text in `sheet`.
    pub seed_read_add_a_passphrase: &'static str,
    /// Label in `sheet`.
    pub seed_read_without_a: &'static str,
    /// Label in `sheet`.
    pub seed_read_passphrase_2: &'static str,
    /// Label in `sheet`.
    pub seed_read_passphrase: &'static str,
    /// Label in `sheet`.
    pub seed_read_with_this: &'static str,
    /// Line in `sheet`.
    pub seed_read_the_two_passphrases: &'static str,
    /// Text in `sheet`.
    pub seed_read_seedqr_read: &'static str,
    // ----- seeds (faraday-core/src/seeds.rs) -----
    /// Error in `seeds_take_file`.
    pub seeds_no_account_key: &'static str,
    /// Error in `seeds_descriptor`.
    pub seeds_type_a_derivation: &'static str,
    /// Error in `seeds_descriptor`.
    pub seeds_a_seed_is_no: &'static str,
    /// Error in `seeds_descriptor`.
    pub seeds_no_seed_yet: &'static str,
    /// Text in `seeds_descriptor`. `{}`: `at`, `crate::wallet::network_name(kn)`, `crate::wallet::network_name(net)`.
    pub seeds_key_is_a_key: &'static str,
    /// Error in `seeds_descriptor`. `{}`: `at`.
    pub seeds_key_has_no_account: &'static str,
    /// Error in `seeds_descriptor`. `{}`: `at`.
    pub seeds_key_is_not_an: &'static str,
    /// Error in `seeds_descriptor`. `{}`: `keys.len()`, `s.n`.
    pub seeds_of_keys_in_hand: &'static str,
    /// Toast in `seeds_from_vault`. `{}`: `want`, `self.vaults.open[v].name`.
    pub seeds_loaded_from: &'static str,
    /// Text in `seeds_make`.
    pub seeds_typed_seed: &'static str,
    /// Text in `seeds_make`.
    pub seeds_typed_seeds: &'static str,
    // ----- seeds (faraday-core/src/seeds_screen.rs) -----
    /// Text in `more_ways`. `{}`: `fp`, `name`.
    pub seeds_load_from: &'static str,
    /// Text in `quorum`. `{}`: `s.m.max(1)`, `n`.
    pub seeds_of: &'static str,
    /// Text in `quorum`.
    pub seeds_one_key_signs: &'static str,
    /// Text in `quorum`. `{}`: `s.m`, `if s.m == 1 { EN.seeds_signature } el…`, `n`.
    pub seeds_of_keys: &'static str,
    /// Text in `quorum`.
    pub seeds_signature: &'static str,
    /// Text in `quorum`.
    pub seeds_signatures_inline: &'static str,
    /// Text in `quorum`.
    pub seeds_signatures: &'static str,
    /// Text in `quorum`.
    pub seeds_keys: &'static str,
    /// Button in `cosigner`.
    pub seeds_change: &'static str,
    /// Label in `cosigner`.
    pub seeds_type_or_paste: &'static str,
    /// Line in `cosigner`.
    pub seeds_not_an_account: &'static str,
    /// Text in `cosigner`. `{}`: `n`.
    pub seeds_use: &'static str,
    /// Text in `cosigner`.
    pub seeds_scan: &'static str,
    /// Line in `shape`.
    pub seeds_cosigners_account: &'static str,
    /// Line in `shape`.
    pub seeds_kind: &'static str,
    /// Line in `shape`.
    pub seeds_derivation_path: &'static str,
    /// Line in `shape`.
    pub seeds_standard: &'static str,
    /// Text in `shape`.
    pub seeds_account: &'static str,
    /// Line in `shape`.
    pub seeds_custom_path: &'static str,
    /// Line in `shape`.
    pub seeds_first_receive: &'static str,
    /// Text in `shape`.
    pub seeds_back_to_the_seeds: &'static str,
    /// Text in `shape`.
    pub seeds_make_the_wallet: &'static str,
    // ----- set_time (faraday-core/src/set_time.rs) -----
    /// Text in `now_local_text`.
    pub set_time_not_known: &'static str,
    /// Toast in `set_time_set`. `{}`: `h`, `mi`.
    pub set_time_time_set_to: &'static str,
    // ----- silent (faraday-core/src/silent.rs) -----
    /// Toast in `silent_open`.
    pub silent_load_a_key_first: &'static str,
    /// Error in `silent_code`.
    pub silent_silent_payments_is: &'static str,
    /// Error in `silent_code`.
    pub silent_choose_a_key: &'static str,
    /// Text in `silent_code`. `{}`: `fp`.
    pub silent_silent_payment: &'static str,
    /// Error in `silent_code`.
    pub silent_no_address_for: &'static str,
    /// Text in `silent_code`. `{}`: `s.label`.
    pub silent_label: &'static str,
    /// Text in `silent_code`.
    pub silent_public_receives: &'static str,
    /// Text in `silent_code`.
    pub silent_silent_payments_address: &'static str,
    /// Error in `silent_code`.
    pub silent_no_record_for: &'static str,
    /// Text in `silent_code`.
    pub silent_public_watch: &'static str,
    /// Text in `silent_code`.
    pub silent_silent_payments_record: &'static str,
    /// Toast in `silent_act`.
    pub silent_no_vault_is: &'static str,
    /// Text in `silent_act`. `{}`: `fp_text(osk_bip::keys::Fingerprint(fp))`.
    pub silent_silent_payments_scan: &'static str,
    /// Text in `silent_act`. `{}`: `title`.
    pub silent_is_in_the_vault: &'static str,
    /// Text in `silent_act`.
    pub silent_a_silent_payments: &'static str,
    /// Text in `silent_act`.
    pub silent_whoever_has: &'static str,
    /// Toast in `silent_add_wallet`.
    pub silent_load_the_key: &'static str,
    /// Text in `silent_add_wallet`. `{}`: `fp_text(osk_bip::keys::Fingerprint(fp))`.
    pub silent_silent_payments_n: &'static str,
    /// Text in `silent_add_wallet`.
    pub silent_silent_payments: &'static str,
    /// Toast in `silent_add_wallet`.
    pub silent_added_to_your: &'static str,
    /// Toast in `silent_check`.
    pub silent_not_a_transaction: &'static str,
    // ----- silent (faraday-core/src/silent_screen.rs) -----
    /// Text in `TITLES`.
    pub silent_key: &'static str,
    /// Text in `TITLES`.
    pub silent_address: &'static str,
    /// Text in `TITLES`.
    pub silent_scan_key: &'static str,
    /// Text in `TITLES`.
    pub silent_check_a_payment: &'static str,
    /// Text in `draw`.
    pub silent_wallets: &'static str,
    /// Text in `summary`.
    pub silent_not_chosen: &'static str,
    /// Text in `summary`.
    pub silent_a_secret_shows: &'static str,
    /// Text in `summary`.
    pub silent_needs_a_previous: &'static str,
    /// Text in `summary`.
    pub silent_a_transaction: &'static str,
    /// Text in `guide`.
    pub silent_a_silent_payments_address: &'static str,
    /// Text in `guide`.
    pub silent_give_this_address: &'static str,
    /// Text in `guide`.
    pub silent_finding_payments: &'static str,
    /// Text in `guide`.
    pub silent_a_payment_to: &'static str,
    /// Line in `card`.
    pub silent_its_key_is_not_loaded: &'static str,
    /// Text in `card`.
    pub silent_add_its_key: &'static str,
    /// Text in `card`.
    pub silent_no_label: &'static str,
    /// Text in `card`.
    pub silent_show_as_qr: &'static str,
    /// Text in `card`.
    pub silent_as_a_bitcoin: &'static str,
    /// Text in `card`. `{}`: `app.medium.for_the()`.
    pub silent_record: &'static str,
    /// Text in `card`.
    pub silent_record_as_qr: &'static str,
    /// Text in `card`.
    pub silent_record_as_png: &'static str,
    /// Text in `card`.
    pub silent_add_as_a_wallet: &'static str,
    /// Button in `card`.
    pub silent_continue: &'static str,
    /// Chip in `card`.
    pub silent_secret_shows: &'static str,
    /// Text in `card`. `{}`: `v.name`.
    pub silent_save_into: &'static str,
    /// Text in `card`.
    pub silent_out_unprotected: &'static str,
    /// Text in `checked_text`. `{}`: `n`, `if n == 1 { EN.silent_output } else {…`.
    pub silent_pays_this_wallet: &'static str,
    /// Text in `checked_text`.
    pub silent_output: &'static str,
    /// Text in `checked_text`.
    pub silent_outputs: &'static str,
    /// Text in `checked_text`.
    pub silent_pays_nothing: &'static str,
    /// Text in `checked_text`.
    pub silent_spends_nothing: &'static str,
    /// Text in `checked_text`.
    pub silent_not_signed_an: &'static str,
    /// Text in `checked_text`.
    pub silent_its_key_is_not: &'static str,
    /// Text in `check_card`.
    pub silent_the_address: &'static str,
    /// Text in `check_card`.
    pub silent_change: &'static str,
    /// Text in `check_card`. `{}`: `m`.
    pub silent_label_inline: &'static str,
    /// Text in `check_card`. `{}`: `p.vout`, `crate::ui::btc(p.amount.to_sat())`, `to`.
    pub silent_output_to: &'static str,
    /// Text in `check_card`. `{}`: `txid`.
    pub silent_needs_the_transaction: &'static str,
    /// Text in `check_card`. `{}`: `it.name`.
    pub silent_check: &'static str,
    /// Line in `check_card`.
    pub silent_no_transaction: &'static str,
    // ----- stick_settings (faraday-core/src/stick_settings.rs) -----
    /// Text in `settings_event`. `{}`: `wrote_as`.
    pub stick_settings_wrote: &'static str,
    /// Text in `settings_event`. `{}`: `name`, `reason`.
    pub stick_settings_not: &'static str,
    // ----- stick_wipe (faraday-core/src/stick_wipe.rs) -----
    /// Text in `LIMIT`.
    pub stick_wipe_no_zeroing: &'static str,
    /// Text in `REACH`.
    pub stick_wipe_faraday: &'static str,
    /// Text in `SECRET`.
    pub stick_wipe_if_a_seed: &'static str,
    /// Text in `WITHIN`.
    pub stick_wipe_only_this: &'static str,
    /// Text in `BOUNDARY`.
    pub stick_wipe_why_the: &'static str,
    /// Text in `size_text`. `{}`: `b as f64 / f64::from(1u32 << 30)`.
    pub stick_wipe_gb: &'static str,
    /// Text in `size_text`. `{}`: `b >> 20`.
    pub stick_wipe_mb: &'static str,
    /// Text in `size_text`. `{}`: `(b >> 10).max(1)`.
    pub stick_wipe_kb: &'static str,
    /// Text in `minutes_text`.
    pub stick_wipe_about_a: &'static str,
    /// Text in `minutes_text`. `{}`: `m`.
    pub stick_wipe_about_minutes: &'static str,
    /// Text in `minutes_text`. `{}`: `m as f64 / 60.0`.
    pub stick_wipe_about_hours: &'static str,
    /// Text in `wiped`. `{}`: `name`.
    pub stick_wipe_deleted: &'static str,
    /// Text in `wiped`. `{}`: `name`, `why`.
    pub stick_wipe_not_deleted: &'static str,
    /// Text in `zero_step`. `{}`: `share`.
    pub stick_wipe_stopped_at: &'static str,
    /// Text in `zero_step`. `{}`: `z.label`.
    pub stick_wipe_free_space: &'static str,
    /// Text in `zero_step`. `{}`: `z.label`.
    pub stick_wipe_s_partition_zeroed: &'static str,
    /// Text in `zero_step`. `{}`: `z.label`, `n`.
    pub stick_wipe_s_partition: &'static str,
    /// Text in `zero_failed`. `{}`: `share`, `why`.
    pub stick_wipe_stopped: &'static str,
    /// Text in `zero_sticks_changed`. `{}`: `self.medium.noun()`.
    pub stick_wipe_the_was: &'static str,
    /// Text in `delete_sheet`. `{}`: `d.label`.
    pub stick_wipe_delete_files: &'static str,
    /// Text in `delete_sheet`. `{}`: `n`, `if n == 1 { EN.stick_wipe_file } else…`.
    pub stick_wipe_delete_with: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_file: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_files: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_zero_the_free: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_zero_the: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_nothing: &'static str,
    /// Text in `delete_sheet`. `{}`: `size_text(shown)`, `minutes_text(ms)`.
    pub stick_wipe_to_write: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_reading: &'static str,
    /// Line in `delete_sheet`.
    pub stick_wipe_danger_zone: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_chosen_files: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_zeros_over: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_the_free: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_the_files: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_the_whole: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_every_file: &'static str,
    /// Text in `delete_sheet`. `{}`: `app.medium.noun()`.
    pub stick_wipe_no_files: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_one_pass: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_two_passes_2: &'static str,
    /// Line in `delete_sheet`.
    pub stick_wipe_two_passes: &'static str,
    /// Text in `delete_sheet`.
    pub stick_wipe_cancel: &'static str,
    /// Text in `zeroing_sheet`. `{}`: `z.label`.
    pub stick_wipe_zeroing_the: &'static str,
    /// Text in `zeroing_sheet`. `{}`: `z.label`.
    pub stick_wipe_zeroing: &'static str,
    /// Text in `zeroing_sheet`. `{}`: `(share * 100.0) as u32`, `minutes_text(ms)`.
    pub stick_wipe_left: &'static str,
    /// Text in `zeroing_sheet`. `{}`: `z.pass`, `z.passes`.
    pub stick_wipe_pass_of: &'static str,
    /// Text in `zeroing_sheet`. `{}`: `app.medium.noun()`.
    pub stick_wipe_keep_the: &'static str,
    /// Text in `zeroing_sheet`.
    pub stick_wipe_stopping: &'static str,
    /// Text in `zeroing_sheet`.
    pub stick_wipe_stop: &'static str,
    // ----- tools (faraday-core/src/tools_screen.rs) -----
    /// Text in `GUIDE`.
    pub tools_calculators_each: &'static str,
    /// Line in `header`.
    pub tools_wallets: &'static str,
    /// Title in `header`.
    pub tools_tools: &'static str,
    /// Text in `body`.
    pub tools_read_as_automatic: &'static str,
    /// Text in `body`.
    pub tools_text: &'static str,
    /// Text in `body`.
    pub tools_hex: &'static str,
    /// Text in `body`. `{}`: `n`.
    pub tools_in: &'static str,
    /// Text in `body`.
    pub tools_text_or_hex: &'static str,
    /// Text in `body`.
    pub tools_base58_bech32: &'static str,
    /// Text in `body`.
    pub tools_a_descriptor: &'static str,
    /// Text in `body`.
    pub tools_an_xpub_ypub: &'static str,
    /// Text in `body`.
    pub tools_an_amount: &'static str,
    /// Text in `body`.
    pub tools_a_policy_e_g: &'static str,
    /// Button in `body`.
    pub tools_clear: &'static str,
    /// Text in `body`.
    pub tools_encoding: &'static str,
    /// Text in `body`.
    pub tools_prefix: &'static str,
    /// Text in `body`.
    pub tools_version: &'static str,
    /// Text in `body`.
    pub tools_bytes: &'static str,
    /// Note in `body`.
    pub tools_not_base58_bech32: &'static str,
    /// Text in `body`.
    pub tools_checksum: &'static str,
    /// Text in `body`.
    pub tools_given: &'static str,
    /// Text in `body`.
    pub tools_holds: &'static str,
    /// Text in `body`.
    pub tools_does_not_hold: &'static str,
    /// Text in `body`.
    pub tools_with_it: &'static str,
    /// Text in `body`.
    pub tools_not_one_this: &'static str,
    /// Note in `body`.
    pub tools_not_a_descriptor: &'static str,
    /// Text in `body`.
    pub tools_legacy: &'static str,
    /// Text in `body`.
    pub tools_nested_segwit: &'static str,
    /// Text in `body`.
    pub tools_native_segwit: &'static str,
    /// Text in `body`.
    pub tools_taproot: &'static str,
    /// Text in `body`.
    pub tools_depth: &'static str,
    /// Text in `body`.
    pub tools_fingerprint: &'static str,
    /// Text in `body`.
    pub tools_child: &'static str,
    /// Text in `body`.
    pub tools_that_is_an_extended: &'static str,
    /// Note in `body`.
    pub tools_not_an_extended: &'static str,
    /// Text in `body`.
    pub tools_descriptor: &'static str,
    /// Text in `body`.
    pub tools_spend_paths: &'static str,
    /// Text in `body`.
    pub tools_wallet: &'static str,
    /// Text in `body`.
    pub tools_one_this_device: &'static str,
    /// Text in `body`.
    pub tools_keys_need_origins: &'static str,
    // ----- transfer (faraday-core/src/transfer.rs) -----
    /// Note in `read_any`. `{}`: `have`, `of`.
    pub transfer_ur_part_of: &'static str,
    /// Note in `read_any`. `{}`: `e`.
    pub transfer_not_a_part: &'static str,
    /// Text in `transfer_send`. `{}`: `self.medium.a()`.
    pub transfer_larger_than: &'static str,
    /// Text in `transfer_send`. `{}`: `base_name(&path)`.
    pub transfer_is_empty: &'static str,
    /// Note in `transfer_scanned`. `{}`: `name`.
    pub transfer_saving: &'static str,
    /// Text in `transfer_saved`. `{}`: `path`.
    pub transfer_saved_this: &'static str,
    /// Text in `transfer_saved`. `{}`: `path`.
    pub transfer_saved: &'static str,
    /// Error in `transfer_saved`. `{}`: `reason`.
    pub transfer_not_saved: &'static str,
    // ----- transfer (faraday-core/src/transfer_screen.rs) -----
    /// Text in `size_text`. `{}`: `bytes as f32 / 1e6`.
    pub transfer_mb: &'static str,
    /// Text in `size_text`. `{}`: `bytes as f32 / 1e3`.
    pub transfer_kb: &'static str,
    /// Title in `draw`.
    pub transfer_transfer: &'static str,
    /// Section label in `send`.
    pub transfer_send: &'static str,
    /// Text in `send`. `{}`: `name`.
    pub transfer_reading: &'static str,
    /// Line in `send`.
    pub transfer_no_files: &'static str,
    /// Text in `send`.
    pub transfer_over_256_kib: &'static str,
    /// Section label in `receive`.
    pub transfer_receive: &'static str,
    /// Button in `receive`.
    pub transfer_scan_with: &'static str,
    /// Button in `receive`.
    pub transfer_open_folder: &'static str,
    // ----- ui (faraday-core/src/ui.rs) -----
    /// Under a firmware sketch (`docs/HUB.md` §7.8, faraday-ui's
    /// `sketch`): it is Faraday's drawing, not the screen itself.
    pub ui_sketch_mock_up: &'static str,
    /// Text in `name`.
    pub ui_dark: &'static str,
    /// Text in `name`.
    pub ui_light: &'static str,
    /// Text in `name`.
    pub ui_cobalt: &'static str,
    /// Text in `name`.
    pub ui_nord: &'static str,
    /// Text in `name`.
    pub ui_catppuccin: &'static str,
    /// Text in `name`.
    pub ui_tokyo_night: &'static str,
    /// Text in `name`.
    pub ui_gruvbox: &'static str,
    /// Text in `name`.
    pub ui_rose_pine: &'static str,
    /// Text in `name`.
    pub ui_bitcoin_orange: &'static str,
    /// Text in `name`.
    pub ui_bitcoin_orange_light: &'static str,
    /// Text in `name`.
    pub ui_lamplight: &'static str,
    // ----- upgrade (faraday-core/src/upgrade.rs) -----
    /// Text in `version`.
    pub upgrade_0_1_0_or_earlier: &'static str,
    /// Text in `describe`. `{}`: `number`, `commit`.
    pub upgrade_test_release_2: &'static str,
    /// Text in `describe`. `{}`: `number`, `commit`, `fp`.
    pub upgrade_test_release: &'static str,
    /// Text in `describe`. `{}`: `number`, `commit`.
    pub upgrade_dev: &'static str,
    /// Text in `describe`. `{}`: `number`, `commit`, `fp`.
    pub upgrade_dev_changes: &'static str,
    // ----- upgrade (faraday-core/src/upgrade_screen.rs) -----
    /// Text in `source`. `{}`: `app.medium.noun()`.
    pub upgrade_insert_the: &'static str,
    /// Label in `source`.
    pub upgrade_read: &'static str,
    /// Label in `source`.
    pub upgrade_reading: &'static str,
    /// Text in `target`. `{}`: `app.medium.noun()`.
    pub upgrade_remove_it_insert: &'static str,
    /// Text in `target`.
    pub upgrade_file: &'static str,
    /// Text in `target`.
    pub upgrade_files: &'static str,
    /// Label in `target`.
    pub upgrade_its_data_partition: &'static str,
    /// Label in `target`.
    pub upgrade_on_it: &'static str,
    /// Label in `target`.
    pub upgrade_to_be_written: &'static str,
    /// Text in `target`.
    pub upgrade_newer_than: &'static str,
    /// Text in `target`.
    pub upgrade_writes_a_dev: &'static str,
    /// Text in `target`.
    pub upgrade_already_this: &'static str,
    /// Text in `target`. `{}`: `has`, `needs`.
    pub upgrade_boot_partition: &'static str,
    /// Text in `target`.
    pub upgrade_removed_during: &'static str,
    /// Text in `target`. `{}`: `why`.
    pub upgrade_not_written: &'static str,
    /// Text in `target`.
    pub upgrade_writing: &'static str,
    /// Text in `target`.
    pub upgrade_upgrade: &'static str,
    /// Label in `done`.
    pub upgrade_written_read: &'static str,
    /// Text in `done`. `{}`: `app.medium.noun()`.
    pub upgrade_remove_the: &'static str,
    /// Text in `done`. `{}`: `app.medium.noun()`.
    pub upgrade_upgrade_another: &'static str,
    /// Text in `done`.
    pub upgrade_done: &'static str,
    // ----- upgrade (faraday-boot/src/lib.rs) -----
    /// Text in `read_source`. `{}`: how many boot partitions hold it.
    pub upgrade_several_hold: &'static str,
    /// Text in `read_source`. `{}`: its MB, this Faraday's MB.
    pub upgrade_source_size: &'static str,
    // ----- vanity (faraday-core/src/vanity.rs) -----
    /// Text in `script_name`.
    pub vanity_legacy: &'static str,
    /// Text in `script_name`.
    pub vanity_nested_segwit: &'static str,
    /// Text in `script_name`.
    pub vanity_native_segwit: &'static str,
    /// Text in `script_name`.
    pub vanity_taproot: &'static str,
    /// Text in `script_name`. `{}`: `name`.
    pub vanity_m_or_n: &'static str,
    /// Text in `dial_name`.
    pub vanity_account_number: &'static str,
    /// Text in `dial_name`.
    pub vanity_the_same_words: &'static str,
    /// Text in `dial_name`.
    pub vanity_passphrase: &'static str,
    /// Text in `dial_name`.
    pub vanity_a_few_characters: &'static str,
    /// Toast in `vanity_open`.
    pub vanity_load_a_key_first: &'static str,
    /// Error in `vanity_act`.
    pub vanity_type_at_least: &'static str,
    /// Text in `vanity_act`.
    pub vanity_this_key_was: &'static str,
    /// Error in `vanity_tick`.
    pub vanity_every_counter: &'static str,
    /// Text in `vanity_use`. `{}`: `label`.
    pub vanity_vanity_passphrase: &'static str,
    /// Text in `vanity_use`.
    pub vanity_vanity_search: &'static str,
    /// Toast in `vanity_use`. `{}`: `name`.
    pub vanity_is_open: &'static str,
    /// Error in `vanity_key_in`. `{}`: `c`.
    pub vanity_cannot_stand: &'static str,
    // ----- vanity (faraday-core/src/vanity_screen.rs) -----
    /// Text in `TITLES`.
    pub vanity_key: &'static str,
    /// Text in `TITLES`.
    pub vanity_what_turns: &'static str,
    /// Text in `TITLES`.
    pub vanity_kind_of_address: &'static str,
    /// Text in `TITLES`.
    pub vanity_characters: &'static str,
    /// Text in `TITLES`.
    pub vanity_search: &'static str,
    /// Text in `draw`.
    pub vanity_wallets: &'static str,
    /// Text in `draw`.
    pub vanity_vanity_address: &'static str,
    /// Text in `guide`.
    pub vanity_the_key_whose: &'static str,
    /// Text in `guide`.
    pub vanity_a_search_tries: &'static str,
    /// Text in `guide`.
    pub vanity_the_kind_of: &'static str,
    /// Text in `guide`.
    pub vanity_type_the_characters: &'static str,
    /// Text in `guide`.
    pub vanity_the_search_runs: &'static str,
    /// Text in `summary`.
    pub vanity_not_chosen: &'static str,
    /// Text in `summary`. `{}`: `thousands(v.grind.tested)`.
    pub vanity_tried_2: &'static str,
    /// Text in `card`. `{}`: `thousands(expected)`.
    pub vanity_about_tries: &'static str,
    /// Button in `card`.
    pub vanity_continue: &'static str,
    /// Line in `card`.
    pub vanity_found: &'static str,
    /// Text in `card`. `{}`: `f.account`, `thousands(v.grind.tested)`.
    pub vanity_account_of_this: &'static str,
    /// Line in `card`.
    pub vanity_added_to_the: &'static str,
    /// Text in `card`.
    pub vanity_hide: &'static str,
    /// Text in `card`.
    pub vanity_show: &'static str,
    /// Line in `card`.
    pub vanity_these_characters: &'static str,
    /// Button in `card`.
    pub vanity_open_this_wallet: &'static str,
    /// Button in `card`.
    pub vanity_search_again: &'static str,
    /// Text in `card`. `{}`: `thousands(r as u64)`.
    pub vanity_a_second: &'static str,
    /// Text in `card`, after a space. `{}`: `seconds_text(s)`.
    pub vanity_about_expected: &'static str,
    /// Text in `card`.
    pub vanity_measuring: &'static str,
    /// Text in `card`. `{}`: `v.grind.prefix.as_str()`.
    pub vanity_searching_for: &'static str,
    /// Text in `card`. `{}`: `thousands(v.grind.tested)`, `rate`, `left`.
    pub vanity_tried: &'static str,
    /// Button in `card`.
    pub vanity_stop: &'static str,
    /// Text in `card`. `{}`: `v.grind.prefix.as_str()`.
    pub vanity_search_for: &'static str,
    /// Text in `seconds_text`. `{}`: `s / 60`.
    pub vanity_min: &'static str,
    /// Text in `seconds_text`. `{}`: `s / 86_400`.
    pub vanity_days: &'static str,
    // ----- vault (faraday-core/src/vault_screens.rs) -----
    /// Text in `gpg_file_size`. `{}`: `bytes as f32 / 1e6`.
    pub vault_mb: &'static str,
    /// Text in `gpg_file_size`. `{}`: `bytes as f32 / 1e3`.
    pub vault_kb: &'static str,
    /// Text in `stick_field`. `{}`: `medium.noun()`.
    pub vault_remove_the_first: &'static str,
    /// Text in `secret`.
    pub vault_release_to_hide: &'static str,
    /// Text in `secret`.
    pub vault_hold_to_show: &'static str,
    /// Button in `hold_delete`.
    pub vault_hold_to_delete: &'static str,
    /// Text in `unlock`. `{}`: `app.vaults.time_text(mem, f.header.co…`.
    pub vault_here_2: &'static str,
    /// Text in `list_line`.
    pub vault_unlock_to_see: &'static str,
    /// Text in `ONCE_MORE`.
    pub vault_type_the_passphrase: &'static str,
    /// Text in `unlock_back`.
    pub vault_import: &'static str,
    /// Line in `list_compact`.
    pub vault_no_vault_in_files: &'static str,
    /// Text in `list_compact`. `{}`: `app.medium.noun()`.
    pub vault_remove_the_to: &'static str,
    /// Text in `list_compact`.
    pub vault_new_vault: &'static str,
    /// Text in `list_compact`.
    pub vault_locked_not_written: &'static str,
    /// Text in `list_compact`.
    pub vault_locked_vault: &'static str,
    /// Text in `list_compact`.
    pub vault_locked: &'static str,
    /// Text in `list_compact`.
    pub vault_view_contents: &'static str,
    /// Text in `list_compact`. `{}`: `app.medium.a()`, `app.medium.for_the()`.
    pub vault_a_vault_holds: &'static str,
    /// Text in `unlock_compact`. `{}`: `app.medium.noun()`.
    pub vault_remove_the_then_type: &'static str,
    /// Line in `unlock_compact`.
    pub vault_no_locked_vault: &'static str,
    /// Text in `unlock_compact`. `{}`: `t`.
    pub vault_unlocking: &'static str,
    /// Button in `unlock_compact`.
    pub vault_unlock: &'static str,
    /// Text in `unlock_compact`. `{}`: `mib_text(m)`.
    pub vault_this_computer_free: &'static str,
    /// Text in `unlock_compact`.
    pub vault_this_computer_memory: &'static str,
    /// Text in `unlock_compact`.
    pub vault_choose_the_vault: &'static str,
    /// Text in `VSTEPS`.
    pub vault_where_will_you: &'static str,
    /// Text in `create`. `{}`: `app.back_name(s)`.
    pub vault_then: &'static str,
    /// Text in `create`.
    pub vault_home: &'static str,
    /// Text in `create`.
    pub vault_create_a_vault: &'static str,
    /// Text in `create_guide`.
    pub vault_a_vault_opens_only: &'static str,
    /// Text in `create_guide`.
    pub vault_each_guess_at: &'static str,
    /// Text in `create_guide`.
    pub vault_every_passphrase: &'static str,
    /// Text in `create_guide`.
    pub vault_a_vault_opens: &'static str,
    /// Text in `create_guide`.
    pub vault_each_passphrase: &'static str,
    /// Text in `create_summaries`.
    pub vault_none_chosen: &'static str,
    /// Text. `{}`: `name`, `mib_text(mem)`, `passes`.
    pub vault_passes_3: &'static str,
    /// Text. `{}`: `size_text(c.slot)`, `file_text(file_len(c.slot))`.
    pub vault_file_n: &'static str,
    /// Text. `{}`: `vaults::vault_stem(&c.name.text)`, `n`, `if n == 1 { FEN.vault_passphrase_inline } …`, `if ok { FEN.vault_match } else { FEN.…`.
    pub vault_ofv: &'static str,
    /// Text in `create_summaries`.
    pub vault_passphrase_inline: &'static str,
    /// Text in `create_summaries`.
    pub vault_passphrases: &'static str,
    /// Text in `create_summaries`.
    pub vault_match: &'static str,
    /// Text in `create_summaries`.
    pub vault_not_finished: &'static str,
    /// Text in `create_body`. `{}`: `mib_text(app.vaults.here_limit())`, `mib_text(free)`.
    pub vault_free: &'static str,
    /// Text in `create_body`.
    pub vault_memory_not_reported: &'static str,
    /// Line in `create_body`.
    pub vault_this_computer: &'static str,
    /// Text in `create_body`.
    pub vault_choose_memory: &'static str,
    /// Text in `create_body`.
    pub vault_suggested: &'static str,
    /// Text in `create_body`.
    pub vault_fits: &'static str,
    /// Text in `create_body`. `{}`: `weakest`.
    pub vault_too_large_for: &'static str,
    /// Text in `create_body`. `{}`: `mib_text(m)`, `p`, `vaults::cost_bits(m, p)`.
    pub vault_passes_bits: &'static str,
    /// Line in `create_body`.
    pub vault_memory: &'static str,
    /// Line in `create_body`.
    pub vault_passes_2: &'static str,
    /// Text in `create_body`. `{}`: `mib_text(mem)`.
    pub vault_unlock_memory_n: &'static str,
    /// Text in `create_body`. `{}`: `weakest`, `mib_text(limit)`.
    pub vault_up_to: &'static str,
    /// Text in `create_body`. `{}`: `vaults::cost_bits(mem, passes)`.
    pub vault_adds_bits_to: &'static str,
    /// Text in `create_body`. `{}`: `mib_text(mem)`, `passes`.
    pub vault_passes: &'static str,
    /// Text in `create_body`. `{}`: `app.vaults.time_text(mem, passes)`.
    pub vault_unlocks_in_here: &'static str,
    /// Text in `create_body`.
    pub vault_unlock_time_not: &'static str,
    /// Text in `create_body`. `{}`: `cost`, `app.vaults.time_text(mem, passes)`.
    pub vault_here: &'static str,
    /// Text in `create_body`. `{}`: `mib_text(f)`.
    pub vault_this_computer_has: &'static str,
    /// Button in `create_body`.
    pub vault_customise: &'static str,
    /// Text in `create_body`. `{}`: `file_text(file_len(*s))`.
    pub vault_file_n_inline: &'static str,
    /// Button in `create_body`.
    pub vault_continue: &'static str,
    /// Line in `create_body`.
    pub vault_name: &'static str,
    /// Text in `create_body`. `{}`: `app.medium.noun()`.
    pub vault_what_it_holds: &'static str,
    /// Text in `create_body`. `{}`: `vaults::vault_stem(&c.name.text)`.
    pub vault_file_ofv: &'static str,
    /// Text in `create_body`. `{}`: `app.medium.noun()`.
    pub vault_remove_the_then: &'static str,
    /// Text in `create_body`.
    pub vault_hide_passphrases: &'static str,
    /// Text in `create_body`.
    pub vault_show_passphrases: &'static str,
    /// Text in `create_body`. `{}`: `i + 1`.
    pub vault_passphrase_n: &'static str,
    /// Button in `create_body`.
    pub vault_remove: &'static str,
    /// Button in `create_body`.
    pub vault_generate_with: &'static str,
    /// Line in `create_body`.
    pub vault_or_type_your: &'static str,
    /// Text in `create_body`.
    pub vault_matches: &'static str,
    /// Text in `create_body`.
    pub vault_does_not_match: &'static str,
    /// Text in `create_body`.
    pub vault_type_it_then: &'static str,
    /// Text in `create_body`. `{}`: `own`, `cost`, `own + cost`.
    pub vault_bits_from_dice: &'static str,
    /// Text in `create_body`. `{}`: `cost`.
    pub vault_typed_its_own: &'static str,
    /// Button in `create_body`.
    pub vault_add_another_passphrase: &'static str,
    /// Text in `dice_panel`. `{}`: `list.dice_per_word()`.
    pub vault_roll_dice_for: &'static str,
    /// Line in `dice_panel`.
    pub vault_no_word_yet: &'static str,
    /// Text in `dice_panel`. `{}`: `words.len()`, `if words.len() == 1 { FEN.vault_word …`, `own`, `own + vaults::cost_bits(mem, passes)`, `if left > 0 { format!(" · {left} of {…`.
    pub vault_bits_with_the: &'static str,
    /// Text in `dice_panel`, after a space. `{}`: `left`, `per`.
    pub vault_dice_for_the_next_word: &'static str,
    /// Text in `dice_panel`.
    pub vault_word: &'static str,
    /// Text in `dice_panel`.
    pub vault_words_inline: &'static str,
    /// Button in `dice_panel`.
    pub vault_use_these_words: &'static str,
    /// Line in `create_panel`.
    pub vault_your_choices: &'static str,
    /// Text in `create_panel`.
    pub vault_opens_on: &'static str,
    /// Text in `create_panel`.
    pub vault_unlock_cost: &'static str,
    /// Text in `create_panel`.
    pub vault_space_per_passphrase: &'static str,
    /// Text in `create_panel`.
    pub vault_name_and_passphrases: &'static str,
    /// Text in `create_panel`.
    pub vault_not_chosen_yet: &'static str,
    /// Text in `create_summary`.
    pub vault_custom: &'static str,
    /// Text in `create_summary`. `{}`: `mib_text(mem)`.
    pub vault_this_computer_n: &'static str,
    /// Text in `create_summary`. `{}`: `name`, `mib_text(mem)`, `weakest`, `mib_text(limit)`.
    pub vault_needs_gives_a: &'static str,
    /// Text in `create_summary`. `{}`: `app.vaults.time_text(mem, passes)`.
    pub vault_unlocking_takes: &'static str,
    /// Text in `create_summary`.
    pub vault_raspberry_pi: &'static str,
    /// Line in `create_summary`.
    pub vault_will_create: &'static str,
    /// Text in `create_summary`.
    pub vault_no_passphrase: &'static str,
    /// Text in `create_summary`.
    pub vault_not_measured: &'static str,
    /// Text in `create_summary`.
    pub vault_file: &'static str,
    /// Text in `create_summary`.
    pub vault_size: &'static str,
    /// Text in `create_summary`.
    pub vault_unlock_memory: &'static str,
    /// Text in `create_summary`.
    pub vault_unlock_here: &'static str,
    /// Text in `create_summary`.
    pub vault_unlock_cost_adds: &'static str,
    /// Text in `create_summary`. `{}`: `cost`.
    pub vault_bits: &'static str,
    /// Text in `create_summary`.
    pub vault_weakest_passphrase: &'static str,
    /// Text in `create_summary`.
    pub vault_strength: &'static str,
    /// Text in `create_summary`.
    pub vault_goes_to: &'static str,
    /// Text in `create_button`. `{}`: `app.vaults.time_text(mem, passes)`.
    pub vault_creating: &'static str,
    /// Button in `create_button`.
    pub vault_create_vault: &'static str,
    /// Text in `item_heading`.
    pub vault_key: &'static str,
    /// Text in `item_heading`. `{}`: `m.word_count()`.
    pub vault_words_n: &'static str,
    /// Text in `item_heading`.
    pub vault_master_seed: &'static str,
    /// Text in `item_heading`.
    pub vault_chosen_at_unlock_inline: &'static str,
    /// Text in `item_heading`.
    pub vault_totp: &'static str,
    /// Text in `item_heading`. `{}`: `t.chars().count()`.
    pub vault_note_characters: &'static str,
    /// Text in `item_heading`.
    pub vault_recovery_sheet: &'static str,
    /// Text in `item_heading`. `{}`: `name`.
    pub vault_recovery_sheet_n: &'static str,
    /// Text in `item_heading`.
    pub vault_ed25519_does: &'static str,
    /// Text in `item_heading`. `{}`: `g.expires`.
    pub vault_ed25519_expires: &'static str,
    /// Text in `item_heading`.
    pub vault_owner_keys: &'static str,
    /// Text in `item_heading`: the Secure Boot keys a record holds.
    pub vault_pk_kek_db: &'static str,
    /// Text in `item_heading`.
    pub vault_record: &'static str,
    /// Text in `contents`.
    pub vault_vaults: &'static str,
    /// Title in `contents`.
    pub vault_vault_contents: &'static str,
    /// Text in `NOTHING_YET`.
    pub vault_nothing_in_it: &'static str,
    /// Text in `add_label`.
    pub vault_make_keys: &'static str,
    /// Text in `empty_buttons`. `{}`: `app.medium.a()`.
    pub vault_write_it_to: &'static str,
    /// Text in `add_full`.
    pub vault_save_a_bitcoin: &'static str,
    /// Text in `add_full`.
    pub vault_make_a_gpg_key: &'static str,
    /// Text in `add_short`.
    pub vault_bitcoin_key: &'static str,
    /// Text in `add_short`.
    pub vault_wallet: &'static str,
    /// Text in `add_short`.
    pub vault_entry: &'static str,
    /// Text in `add_short`.
    pub vault_gpg_key: &'static str,
    /// Text in `add_short`.
    pub vault_secure_boot: &'static str,
    /// Line in `contents_compact`.
    pub vault_no_vault_is_open: &'static str,
    /// Text in `contents_compact`. `{}`: `v.changes`, `if v.changes == 1 { FEN.vault_change …`.
    pub vault_open_unsaved: &'static str,
    /// Text in `contents_compact`.
    pub vault_change: &'static str,
    /// Text in `contents_compact`.
    pub vault_changes: &'static str,
    /// Button in `contents_compact`.
    pub vault_rename: &'static str,
    /// Text in `contents_compact`.
    pub vault_scan_a_code: &'static str,
    /// Text in `contents_compact`. `{}`: `entries_files`.
    pub vault_import_from_files: &'static str,
    /// Text in `contents_compact`. `{}`: `i.name`.
    pub vault_import_n: &'static str,
    /// Text in `contents_compact`.
    pub vault_totp_codes: &'static str,
    /// Line in `contents_compact`.
    pub vault_none_in_this: &'static str,
    /// Text in `contents_compact`. `{}`: `app.medium.noun()`.
    pub vault_secret_values: &'static str,
    /// Text in `detail`.
    pub vault_rename_the_vault: &'static str,
    /// Text in `detail`.
    pub vault_add_a_note: &'static str,
    /// Text in `detail`.
    pub vault_edit_the_note: &'static str,
    /// Text in `detail`.
    pub vault_make_a_key: &'static str,
    /// Text in `detail`.
    pub vault_make_secure_boot: &'static str,
    /// Text in `detail`.
    pub vault_add_an_entry: &'static str,
    /// Text in `detail`.
    pub vault_edit_the_entry: &'static str,
    /// Line in `detail`.
    pub vault_expires: &'static str,
    /// Line in `detail`.
    pub vault_making_pk_kek: &'static str,
    /// Text in `detail`.
    pub vault_make_the_key: &'static str,
    /// Text in `detail`.
    pub vault_make_the_keys: &'static str,
    /// Text in `detail`. `{}`: `app.inbox.get(k).map_or(FEN.vault_the…`.
    pub vault_open_3: &'static str,
    /// Text in `detail`.
    pub vault_the_backup: &'static str,
    /// Text in `detail`.
    pub vault_load_with_a_bip: &'static str,
    /// Line in `detail`.
    pub vault_passphrase: &'static str,
    /// Line in `detail`.
    pub vault_opening: &'static str,
    /// Line in `detail`.
    pub vault_sealing: &'static str,
    /// Text in `detail`.
    pub vault_open_2: &'static str,
    /// Text in `detail`.
    pub vault_seal_and_export: &'static str,
    /// Text in `detail`.
    pub vault_load: &'static str,
    /// Button in `detail`.
    pub vault_cancel: &'static str,
    /// Text in `detail`. `{}`: `app.medium.from_the()`.
    pub vault_no_file: &'static str,
    /// Chip in `detail`.
    pub vault_signed: &'static str,
    /// Text in `detail`.
    pub vault_sign_1_file: &'static str,
    /// Text in `detail`. `{}`: `n`.
    pub vault_sign_files: &'static str,
    /// Button in `detail`.
    pub vault_back_to_the_key: &'static str,
    /// Label in `detail`.
    pub vault_words: &'static str,
    /// Label in `detail`.
    pub vault_bip_39_passphrase: &'static str,
    /// Text in `detail`.
    pub vault_stored: &'static str,
    /// Text in `detail`.
    pub vault_not_stored: &'static str,
    /// Label in `detail`.
    pub vault_chosen_at_unlock: &'static str,
    /// Text in `detail`.
    pub vault_yes: &'static str,
    /// Text in `detail`.
    pub vault_no: &'static str,
    /// Label in `detail`.
    pub vault_in_this_session: &'static str,
    /// Text in `detail`.
    pub vault_not_loaded: &'static str,
    /// Text in `detail`.
    pub vault_load_into_session: &'static str,
    /// Text in `detail`.
    pub vault_load_with_a_passphrase: &'static str,
    /// Text in `detail`.
    pub vault_leave_unchosen: &'static str,
    /// Text in `detail`.
    pub vault_choose_at_unlock: &'static str,
    /// Text in `detail`.
    pub vault_open_in_wallets: &'static str,
    /// Label in `detail`.
    pub vault_username: &'static str,
    /// Label in `detail`.
    pub vault_password: &'static str,
    /// Label in `detail`.
    pub vault_url: &'static str,
    /// Line in `detail`.
    pub vault_code: &'static str,
    /// Button in `detail`.
    pub vault_set_the_time: &'static str,
    /// Text in `detail`.
    pub vault_unreadable: &'static str,
    /// Label in `detail`.
    pub vault_totp_secret: &'static str,
    /// Label in `detail`.
    pub vault_notes: &'static str,
    /// Text in `detail`.
    pub vault_export_for_keepass: &'static str,
    /// Label in `detail`.
    pub vault_text: &'static str,
    /// Text in `detail`.
    pub vault_edit: &'static str,
    /// Label in `detail`.
    pub vault_descriptor: &'static str,
    /// Label in `detail`.
    pub vault_note: &'static str,
    /// Label in `detail`.
    pub vault_fingerprint: &'static str,
    /// Label in `detail`.
    pub vault_signing_subkey: &'static str,
    /// Label in `detail`.
    pub vault_user_id: &'static str,
    /// Text in `detail`. `{}`: `g.created`.
    pub vault_from_does_not: &'static str,
    /// Text in `detail`. `{}`: `g.created`, `g.expires`.
    pub vault_to: &'static str,
    /// Label in `detail`.
    pub vault_valid: &'static str,
    /// Label in `detail`.
    pub vault_paperkey: &'static str,
    /// Line in `detail`.
    pub vault_renew_for: &'static str,
    /// Button in `detail`.
    pub vault_renew: &'static str,
    /// Text in `detail`.
    pub vault_export_public: &'static str,
    /// Text in `detail`.
    pub vault_revocation_certificate: &'static str,
    /// Text in `detail`.
    pub vault_show_as_qr: &'static str,
    /// Text in `detail`.
    pub vault_sign_a_file: &'static str,
    /// Label in `detail`.
    pub vault_owner_guid: &'static str,
    /// Line in `detail`.
    pub vault_enrolment_adds: &'static str,
    /// Text in `detail`.
    pub vault_windows_compatible: &'static str,
    /// Text in `detail`.
    pub vault_own_keys_only: &'static str,
    /// Text in `detail`.
    pub vault_your_pk_kek_and: &'static str,
    /// Text in `detail`.
    pub vault_yours_with_microsoft: &'static str,
    /// Text in `detail`.
    pub vault_enrolment_files: &'static str,
    /// Text in `save_panel`.
    pub vault_save_a_key: &'static str,
    /// Text in `save_panel`.
    pub vault_save_a_wallet: &'static str,
    /// Text in `save_panel`. `{}`: `key.label`.
    pub vault_bip_39_passphrase_n: &'static str,
    /// Button in `save_panel`.
    pub vault_save_with_its: &'static str,
    /// Line in `save_panel`.
    pub vault_every_key_in: &'static str,
    /// Button in `save_panel`.
    pub vault_save: &'static str,
    /// Line in `save_panel`.
    pub vault_every_wallet: &'static str,
    /// Text in `lock_ask`.
    pub vault_sealed: &'static str,
    /// Text in `lock_ask`, in place of Sealed while the system has given
    /// no randomness.
    pub vault_not_sealed: &'static str,
    /// Button in `lock_ask`, in place of Lock while the system has given
    /// no randomness: the changes are lost.
    pub vault_lock_without_sealing: &'static str,
    /// Text in `lock_ask`.
    pub vault_keys_not_saved: &'static str,
    /// Text in `lock_ask`.
    pub vault_wallets_not_saved: &'static str,
    /// Line in `lock_ask`.
    pub vault_everything_not: &'static str,
    /// Button in `lock_ask`.
    pub vault_back_to_the_vault: &'static str,
    /// Button in `lock_ask`.
    pub vault_lock: &'static str,
    /// Text in `years_row`.
    pub vault_1_year: &'static str,
    /// Text in `years_row`.
    pub vault_2_years: &'static str,
    /// Text in `years_row`.
    pub vault_5_years: &'static str,
    /// Text in `years_row`.
    pub vault_never: &'static str,
    /// Text in `images_panel`. `{}`: `item.name`.
    pub vault_sign_n: &'static str,
    /// Line in `images_panel`.
    pub vault_compare_it_with: &'static str,
    /// Button in `images_panel`.
    pub vault_hold_to_sign: &'static str,
    /// Button in `images_panel`.
    pub vault_back: &'static str,
    /// Line in `images_panel`.
    pub vault_sign_or_check: &'static str,
    /// Text in `images_panel`. `{}`: `app.medium.from_the()`.
    pub vault_no_efi_image: &'static str,
    /// Button in `images_panel`.
    pub vault_sign: &'static str,
    /// Button in `images_panel`.
    pub vault_check: &'static str,
    /// Button in `images_panel`.
    pub vault_back_to_the_keys: &'static str,
    // ----- vaults (faraday-core/src/vaults.rs) -----
    /// Text in `PRESETS`.
    pub vaults_light: &'static str,
    /// Text in `PRESETS`.
    pub vaults_standard: &'static str,
    /// Text in `PRESETS`.
    pub vaults_medium: &'static str,
    /// Text in `PRESETS`.
    pub vaults_strong: &'static str,
    /// Text in `PRESETS`.
    pub vaults_maximum: &'static str,
    /// Text in `SIZES`.
    pub vaults_pcs_only: &'static str,
    /// Text in `SIZES`.
    pub vaults_pcs_and_a_raspberry: &'static str,
    /// Text in `MACHINES`.
    pub vaults_other_pcs: &'static str,
    /// Text in `MACHINES`.
    pub vaults_other_pcs_inline: &'static str,
    /// A machine a vault may be opened on.
    pub vaults_raspberry_pi_5: &'static str,
    /// Its short name.
    pub vaults_pi_5: &'static str,
    /// A machine a vault may be opened on.
    pub vaults_raspberry_pi_4: &'static str,
    /// Its short name.
    pub vaults_pi_4: &'static str,
    /// A machine a vault may be opened on.
    pub vaults_raspberry_pi_3: &'static str,
    /// Its short name.
    pub vaults_pi_3: &'static str,
    /// A machine a vault may be opened on, and its short name.
    pub vaults_pi_zero_2_w: &'static str,
    /// Text in `CATEGORIES`.
    pub vaults_bitcoin_keys: &'static str,
    /// Text in `CATEGORIES`.
    pub vaults_wallets: &'static str,
    /// Text in `CATEGORIES`.
    pub vaults_entries: &'static str,
    /// Text in `CATEGORIES`.
    pub vaults_gpg_keys: &'static str,
    /// Text in `CATEGORIES`.
    pub vaults_secure_boot: &'static str,
    /// Text in `SLOT_LABELS`.
    pub vaults_main: &'static str,
    /// Text in `SLOT_LABELS`.
    pub vaults_second: &'static str,
    /// Text in `SLOT_LABELS`.
    pub vaults_third: &'static str,
    /// Text in `SLOT_LABELS`.
    pub vaults_fourth: &'static str,
    /// Text in `dice_aim`. `{}`: `n`, `STRONG_BITS`.
    pub vaults_aim_for_words: &'static str,
    /// Text in `mib_text`. `{}`: `f64::from(mib) / 1024.0`.
    pub vaults_gib: &'static str,
    /// Text in `size_text`. `{}`: `slot / 1_048_576`.
    pub vaults_mib: &'static str,
    /// Text in `size_text`. `{}`: `slot / 1024`.
    pub vaults_kib: &'static str,
    /// Text in `file_text`. `{}`: `len as f64 / 1e6`.
    pub vaults_mb: &'static str,
    /// Text in `file_text`. `{}`: `len.div_ceil(1000)`.
    pub vaults_kb: &'static str,
    /// Text in `cost_text`.
    pub vaults_pass: &'static str,
    /// Text in `cost_text`.
    pub vaults_passes: &'static str,
    /// Text in `time_text`. `{}`: `s.round() as u64`.
    pub vaults_about_s: &'static str,
    /// Text in `time_text`.
    pub vaults_not_measured: &'static str,
    /// Text in `weakest`.
    pub vaults_this_computer_2: &'static str,
    /// Text in `kept_wiped`.
    pub vaults_file: &'static str,
    /// Text in `kept_wiped`.
    pub vaults_files: &'static str,
    /// Text in `kept_wiped`. `{}`: `v`.
    pub vaults_in_copy_checked: &'static str,
    /// Text in `kept_wiped`.
    pub vaults_copy_checked: &'static str,
    /// Text in `kept_wiped`. `{}`: `fp`.
    pub vaults_seed_n: &'static str,
    /// Text in `kept_wiped`. `{}`: `v`.
    pub vaults_in: &'static str,
    /// Text in `kept_wiped`.
    pub vaults_in_files: &'static str,
    /// Text in `kept_wiped`.
    pub vaults_nowhere_else: &'static str,
    /// Text in `vault_way`.
    pub vaults_make_a_vault: &'static str,
    /// Text in `vault_way`. `{}`: `files[k].name`.
    pub vaults_unlock: &'static str,
    /// Text in `vault_way`.
    pub vaults_unlock_a_vault_2: &'static str,
    /// Text in `back_name`.
    pub vaults_make_a_new_wallet: &'static str,
    /// Text in `back_name`.
    pub vaults_restore_an_existing: &'static str,
    /// Text in `back_name`.
    pub vaults_spend: &'static str,
    /// Text in `back_name`.
    pub vaults_back_up_a_wallet: &'static str,
    /// Text in `back_name`.
    pub vaults_child_seeds: &'static str,
    /// Text in `back_name`.
    pub vaults_lightning_node: &'static str,
    /// Text in `back_name`.
    pub vaults_import: &'static str,
    /// Text in `back_link_name`.
    pub vaults_tools: &'static str,
    /// Text in `vault_act`. `{}`: `keys`, `if keys == 1 { EN.vaults_seed_inline } else …`, `wallets`, `if wallets == 1 { EN.vaults_wallet_inline …`.
    pub vaults_and_loaded: &'static str,
    /// Text in `vault_act`.
    pub vaults_wallet_inline: &'static str,
    /// Text in `vault_act`.
    pub vaults_wallets_inline: &'static str,
    /// Toast in `vault_import_entries`.
    pub vaults_those_entries: &'static str,
    /// Text in `vault_scanned_entry`. `{}`: `n`, `if n == 1 { EN.vaults_entry_inline } else …`.
    pub vaults_added: &'static str,
    /// Text in `vault_scanned_entry`.
    pub vaults_entry_inline: &'static str,
    /// Text in `vault_scanned_entry`.
    pub vaults_entries_inline: &'static str,
    /// Error in `vault_scanned_entry`.
    pub vaults_not_a_totp_setup: &'static str,
    /// Error in `vault_prompt_go`.
    pub vaults_type_the_backup: &'static str,
    /// Error in `vault_prompt_go`.
    pub vaults_a_passphrase: &'static str,
    /// Text in `vault_open_backup`.
    pub vaults_that_passphrase: &'static str,
    /// Text in `vault_open_backup`. `{}`: `mib`.
    pub vaults_needs_mib_which: &'static str,
    /// Text in `vault_open_backup`.
    pub vaults_not_a_backup: &'static str,
    /// Text in `vault_unlock_check`. `{}`: `self.medium.noun()`.
    pub vaults_remove_the_to: &'static str,
    /// Text in `vault_unlock_check`.
    pub vaults_type_the_passphrase: &'static str,
    /// Text in `vault_unlock_check`. `{}`: `mib_text(f.header.memory_to_open_mib())`, `mib_text(free)`.
    pub vaults_needs_this_computer: &'static str,
    /// Toast in `vault_unlock`. `{}`: `self.vaults.open[v].name`.
    pub vaults_unlocked: &'static str,
    /// Toast in `unlock_landed`. `{}`: `what`, `from`.
    pub vaults_loaded_from: &'static str,
    /// Text in `vault_rows`.
    pub vaults_seed: &'static str,
    /// Text in `vault_load_wallet`. `{}`: `keys`, `if keys == 1 { EN.vaults_seed_inline } else …`.
    pub vaults_loaded: &'static str,
    /// Text in `vault_load_wallet`.
    pub vaults_seed_inline: &'static str,
    /// Text in `vault_load_wallet`.
    pub vaults_seeds: &'static str,
    /// Text in `vault_kdbx`.
    pub vaults_entry: &'static str,
    /// Toast in `vault_kdbx`. `{}`: `name`, `self.medium.for_the()`.
    pub vaults_waits_sealed: &'static str,
    /// Error in `vault_kdbx`.
    pub vaults_that_entry_is: &'static str,
    /// Toast in `vault_delete`.
    pub vaults_deleted_from: &'static str,
    /// Toast in `vault_load_selected`. `{}`: `crate::wallet::fp_text(fp)`.
    pub vaults_key_is_loaded: &'static str,
    /// Text in `vault_load_selected`.
    pub vaults_vault: &'static str,
    /// Text in `vault_form`.
    pub vaults_title: &'static str,
    /// Text in `vault_form`.
    pub vaults_username: &'static str,
    /// Text in `vault_form`.
    pub vaults_password: &'static str,
    /// Text in `vault_form`.
    pub vaults_url: &'static str,
    /// Text in `vault_form`.
    pub vaults_notes: &'static str,
    /// Text in `vault_form`.
    pub vaults_totp_secret: &'static str,
    /// Text in `vault_form`.
    pub vaults_text: &'static str,
    /// Text in `vault_form`.
    pub vaults_name: &'static str,
    /// Text in `vault_form`.
    pub vaults_email: &'static str,
    /// Text in `vault_form`.
    pub vaults_owner_name: &'static str,
    /// Toast in `vault_form_save`.
    pub vaults_this_computer: &'static str,
    /// Toast in `vault_form_save`.
    pub vaults_made_the_certificate: &'static str,
    /// Text in `vault_form_save`.
    pub vaults_the_note_is: &'static str,
    /// Text in `vault_form_save`.
    pub vaults_an_entry_needs: &'static str,
    /// Toast in `vault_form_save`.
    pub vaults_not_a_totp_key: &'static str,
    /// Toast in `vault_add_file`.
    pub vaults_unlock_a_vault: &'static str,
    /// Text in `vault_add_file`. `{}`: `it.name`.
    pub vaults_is_in_the_vault_as_a_note: &'static str,
    /// Text in `vault_add_file`. `{}`: `it.name`.
    pub vaults_is_in_the_vault_as_a: &'static str,
    /// Text in `vault_add_file`. `{}`: `it.name`.
    pub vaults_is_in_the_vault_as: &'static str,
    /// Toast in `key_record`.
    pub vaults_this_key_has: &'static str,
    /// Text in `vault_save_wallet`. `{}`: `name`.
    pub vaults_is_in_the_vault: &'static str,
    /// Toast in `vault_push_in`.
    pub vaults_that_does_not: &'static str,
    /// Text in `vault_create_check`. `{}`: `medium.noun()`.
    pub vaults_remove_the_first: &'static str,
    /// Text in `vault_create_check`. `{}`: `n + 1`.
    pub vaults_finish_step: &'static str,
    /// Text in `vault_create_check`.
    pub vaults_every_passphrase: &'static str,
    /// Text in `vault_create_check`.
    pub vaults_each_passphrase: &'static str,
    /// Text in `vault_create_check`.
    pub vaults_two_passphrases: &'static str,
    /// Text in `vault_create_check`.
    pub vaults_choose_a_cost: &'static str,
    /// Error in `vault_create`.
    pub vaults_no_randomness: &'static str,
    /// Toast in `vault_create`.
    pub vaults_vault_created: &'static str,
    /// Text in `line`.
    pub vaults_never_written: &'static str,
    /// Text in `line`. `{}`: `label`.
    pub vaults_on_current: &'static str,
    /// Text in `line`.
    pub vaults_changed_since: &'static str,
    /// A vault's state in what a lock wipes, while the system has given
    /// no randomness.
    pub vaults_changes_not_sealed: &'static str,
    /// Text in `line`.
    pub vaults_unchanged: &'static str,
    /// Text in `line`. `{}`: `self.wallets.len()`.
    pub vaults_wallets_n: &'static str,
    /// Text in `line`. `{}`: `self.keys[0]`.
    pub vaults_key: &'static str,
    /// Text in `line`. `{}`: `self.keys.join(", ")`.
    pub vaults_keys_inline: &'static str,
    /// Text in `line`. `{}`: `n`.
    pub vaults_keys: &'static str,
    /// Text in `line`.
    pub vaults_1_entry: &'static str,
    /// Text in `line`. `{}`: `n`.
    pub vaults_entries_n: &'static str,
    /// Text in `line`. `{}`: `time_of_day(local_secs(t, tz_min))`.
    pub vaults_seen: &'static str,
    /// Text in `line`.
    pub vaults_nothing_in_it: &'static str,
    /// Text in `redact_place_name`. `{}`: `p + 1`, `rest`.
    pub vaults_place: &'static str,
    /// Text in `vault_summaries_refresh`.
    pub vaults_wallet: &'static str,
    // ----- visit_taken (faraday-core/src/visit_taken.rs) -----
    /// Text in `line`. `{}`: `self.name`, `keep_as`.
    pub visit_taken_keep_both_n: &'static str,
    /// Text in `line`. `{}`: `self.name`.
    pub visit_taken_the_same: &'static str,
    /// Text in `line`. `{}`: `self.name`, `keep_as`.
    pub visit_taken_another: &'static str,
    /// Text in `taken_sheet`. `{}`: `app.medium.on_the()`.
    pub visit_taken_already: &'static str,
    /// Text in `taken_sheet`.
    pub visit_taken_replace: &'static str,
    /// Text in `taken_sheet`.
    pub visit_taken_keep_both: &'static str,
    /// Text in `taken_sheet`.
    pub visit_taken_cancel: &'static str,
    // ----- wallet (faraday-core/src/wallet.rs) -----
    /// Text in `network_name`.
    pub wallet_mainnet: &'static str,
    /// Text in `network_name`.
    pub wallet_testnet: &'static str,
    /// Text in `network_name`.
    pub wallet_signet: &'static str,
    /// Text in `network_name`.
    pub wallet_regtest: &'static str,
    /// Text in `text`. `{}`: `e`.
    pub wallet_not_a_valid: &'static str,
    /// Text in `text`. `{}`: `fp`.
    pub wallet_is_already_loaded: &'static str,
    /// Text in `text`. `{}`: `got`, `wanted`.
    pub wallet_these_words_give: &'static str,
    /// Text in `text`. `{}`: `got`, `wallet`.
    pub wallet_these_words: &'static str,
    /// Text in `text`. `{}`: `e`.
    pub wallet_not_a_wallet: &'static str,
    /// Text in `text`. `{}`: `network_name(*offered)`, `network_name(*loaded)`.
    pub wallet_is_not_the_network: &'static str,
    /// Text in `text`. `{}`: `fp`.
    pub wallet_is_not_a_key: &'static str,
    /// Text in `source_tag`. `{}`: `source`.
    pub wallet_from: &'static str,
    /// Text in `source_tag`. `{}`: `source`.
    pub wallet_from_inline: &'static str,
    /// Text in `add_seed`.
    pub wallet_a_master_seed: &'static str,
    /// Text in `shape`.
    pub wallet_legacy: &'static str,
    /// Text in `shape`.
    pub wallet_nested_segwit: &'static str,
    /// Text in `shape`.
    pub wallet_native_segwit: &'static str,
    /// Text in `shape`.
    pub wallet_taproot: &'static str,
    /// Text in `shape`. `{}`: `script(t)`.
    pub wallet_single_key: &'static str,
    /// Text in `shape`. `{}`: `m`, `n`.
    pub wallet_of_legacy_multisig: &'static str,
    /// Text in `shape`. `{}`: `m`, `n`.
    pub wallet_of_nested_segwit: &'static str,
    /// Text in `shape`. `{}`: `m`, `n`.
    pub wallet_of_native_segwit: &'static str,
    /// Text in `shape`. `{}`: `m`, `n`.
    pub wallet_of_taproot_multisig: &'static str,
    /// Text in `shape`. `{}`: `wallet.policy.keys().len()`.
    pub wallet_keys_miniscript: &'static str,
    /// Text in `shape`. `{}`: `wallet.policy.keys().len()`.
    pub wallet_keys_taproot: &'static str,
    /// Text in `shape`. `{}`: `n`, `n`.
    pub wallet_of_musig2: &'static str,
    /// Text in `shape`. `{}`: `m`, `n`.
    pub wallet_of_frost: &'static str,
    /// Text in `shape`.
    pub wallet_1_key_silent: &'static str,
    /// Text in `shape`.
    pub wallet_wallet: &'static str,
    /// Text in `address`. `{}`: `e`.
    pub wallet_no_address: &'static str,
    /// Error in `sign_with`.
    pub wallet_no_key_in_this: &'static str,
    /// Text in `read_wallet`.
    pub wallet_a_json_file: &'static str,
    /// Text in `read_wallet`. `{}`: `e`.
    pub wallet_bsms: &'static str,
    /// Text in `coldcard_export`. `{}`: `why`.
    pub wallet_coldcard_export: &'static str,
    /// Text in `coldcard_export`.
    pub wallet_no_xfp: &'static str,
    /// Error in `coldcard_export`. `{}`: `field`, `first`.
    pub wallet_its_first_address: &'static str,
    /// Error in `coldcard_export`.
    pub wallet_no_single_sig: &'static str,
    /// Text in `collect_line`. `{}`: `missing`, `if collected == 0 { "" } else { " mor…`, `if missing == 1 { EN.wallet_signature…`.
    pub wallet_collect: &'static str,
    /// Text in `collect_line`, after a space.
    pub wallet_more: &'static str,
    /// Text in `collect_line`.
    pub wallet_signature: &'static str,
    /// Text in `collect_line`.
    pub wallet_signatures: &'static str,
    /// Error in `collect`.
    pub wallet_not_a_psbt: &'static str,
    /// Error in `collect`. `{}`: `source`.
    pub wallet_is_a_psbt_for: &'static str,
    /// Text in `collect`. `{}`: `e`.
    pub wallet_cannot_combine: &'static str,
    /// Error in `collect`. `{}`: `source`.
    pub wallet_a_signature: &'static str,
    /// Error in `finish`.
    pub wallet_not_enough_signatures: &'static str,
    /// Text in `finish_reason`.
    pub wallet_the_script_is: &'static str,
    /// Error in `sign_message`.
    pub wallet_no_such_wallet: &'static str,
    /// Error in `sign_message`.
    pub wallet_messages_are: &'static str,
    /// Error in `sign_message`.
    pub wallet_the_wallet_has: &'static str,
    /// Error in `sign_message`.
    pub wallet_the_wallet_s_xpub: &'static str,
    /// Error in `sign_message`.
    pub wallet_the_wallet_s_key: &'static str,
    /// Error in `sign_message`.
    pub wallet_the_wallet_s: &'static str,
    /// Text in `paths`. `{}`: `n`, `n / 144`.
    pub wallet_blocks_after: &'static str,
    /// Text in `paths`. `{}`: `u64::from(*n) * 512 / 86_400`.
    pub wallet_days_after_the: &'static str,
    /// Text in `paths`. `{}`: `h`.
    pub wallet_after_block: &'static str,
    /// Text in `paths`. `{}`: `t`.
    pub wallet_after_time: &'static str,
    /// Text in `paths`.
    pub wallet_a_secret_preimage: &'static str,
    // ----- wordlist (faraday-core/src/wordlist.rs) -----
    /// Text in `PRESS_A_WORD`.
    pub wordlist_press_a_word: &'static str,
    /// Text in `name`.
    pub wordlist_bip_39_english: &'static str,
    /// Text in `name`.
    pub wordlist_eff_large: &'static str,
    /// Text in `name`.
    pub wordlist_eff_short_1: &'static str,
    /// Text in `name`.
    pub wordlist_eff_short: &'static str,
    /// Text in `code_heading`.
    pub wordlist_bits: &'static str,
    /// Text in `code_heading`.
    pub wordlist_dice: &'static str,
    // ----- wordlist (faraday-core/src/wordlist_screen.rs) -----
    /// Button in `draw`.
    pub wordlist_back: &'static str,
    /// Text in `draw`. `{}`: `words.len()`, `m + 1`, `words.len()`.
    pub wordlist_words_word: &'static str,
    /// Text in `draw`. `{}`: `words.len()`.
    pub wordlist_words: &'static str,
    /// Text in `draw`.
    pub wordlist_type_to_find: &'static str,
    /// Text in `draw`. `{}`: `st.find`.
    pub wordlist_find: &'static str,
    /// Line in `draw`.
    pub wordlist_no: &'static str,
    /// Line in `draw`.
    pub wordlist_word: &'static str,
    // ----- The shells: window titles and the labels they give (faraday/shells) -----
    /// The desktop app's window title.
    pub desktop_title: &'static str,
    /// The desktop app's window title after the panel size: the keys.
    pub desktop_title_keys: &'static str,
    /// The desktop app's keys, pinned at the top of the window.
    pub desktop_keys_line: &'static str,
    /// The desktop app's window title at a panel size: `{}` the diagonal in
    /// inches, then the keys.
    pub desktop_title_panel: &'static str,
    /// The hub app's window title.
    pub hub_window_title: &'static str,
    /// A Windows disk as Disk Management names it: `{}` its number.
    pub hub_disk_number: &'static str,
    /// The label of the firmware boot entry the hub app makes on Linux; it
    /// also finds the entry again.
    pub hub_boot_entry_label: &'static str,
    /// A USB stick mounted with no volume label: `{}` its device name.
    pub stick_usb_stick: &'static str,
    // ----- Virtual machines by name, as the stick's device check and the hub app show them -----
    /// A hypervisor's name.
    pub vm_qemu: &'static str,
    /// A hypervisor's name.
    pub vm_kvm: &'static str,
    /// A hypervisor's name.
    pub vm_vmware: &'static str,
    /// A hypervisor's name.
    pub vm_virtualbox: &'static str,
    /// A hypervisor's name.
    pub vm_hyper_v: &'static str,
    /// A hypervisor's name.
    pub vm_xen: &'static str,
    /// A hypervisor's name.
    pub vm_parallels: &'static str,
    /// A hypervisor's name.
    pub vm_bochs: &'static str,
    // ----- The Companion (docs/COMPANION.md), faraday-companion -----
    /// Home's title: the app's name.
    pub companion_title: &'static str,
    /// Home row, and the scanner's title (§4.2).
    pub companion_home_receive: &'static str,
    /// Home row, and the list a file is sent from (§4.3).
    pub companion_home_send: &'static str,
    /// Home row, and the list of kept files (§4.4).
    pub companion_home_received: &'static str,
    /// Home row: the boot pages (§4.7).
    pub companion_home_start: &'static str,
    /// Home row: every Learn page.
    pub companion_home_learn: &'static str,
    /// Home row: About (§4.8).
    pub companion_home_about: &'static str,
    /// Inside the viewfinder before the first camera frame.
    pub companion_scan_waiting: &'static str,
    /// Inside the viewfinder while frames come.
    pub companion_scan_looking: &'static str,
    /// Inside the viewfinder: the shell has no camera, or access was refused.
    pub companion_scan_no_camera: &'static str,
    /// A code in parts: `{}` parts read of `{}`.
    pub companion_scan_part: &'static str,
    /// A code in parts: read, of, and the parts still missing.
    pub companion_scan_part_missing: &'static str,
    /// A code that is a part of another transfer, or damaged.
    pub companion_scan_not_read: &'static str,
    /// Paste found no text on the clipboard.
    pub companion_scan_nothing_to_paste: &'static str,
    /// The scanner's one way in besides the camera.
    pub companion_paste: &'static str,
    /// A whole file arrived and is in Received.
    pub companion_kept: &'static str,
    /// The same file arrived again: it is in Received once.
    pub companion_already_kept: &'static str,
    /// Opens the file that arrived.
    pub companion_open: &'static str,
    /// Back to the camera.
    pub companion_scan_again: &'static str,
    /// A file's name, in a record.
    pub companion_name: &'static str,
    /// A file's kind, in a record.
    pub companion_kind: &'static str,
    /// A file's size, in a record.
    pub companion_size: &'static str,
    /// When a file was kept, in a record.
    pub companion_when: &'static str,
    /// A file's SHA-256, in a record.
    pub companion_sha256: &'static str,
    /// A kind of kept file.
    pub companion_kind_psbt: &'static str,
    /// A kind of kept file.
    pub companion_kind_transaction: &'static str,
    /// A kind of kept file.
    pub companion_kind_description: &'static str,
    /// A kind of kept file.
    pub companion_kind_file: &'static str,
    /// A kind of kept file.
    pub companion_kind_text: &'static str,
    /// A size under a kibibyte.
    pub companion_bytes: &'static str,
    /// A size in kibibytes.
    pub companion_kib: &'static str,
    /// A time the phone's clock did not give.
    pub companion_time_unknown: &'static str,
    /// Refused (§5): seed words.
    pub companion_refuse_words: &'static str,
    /// Refused (§5): a SeedQR.
    pub companion_refuse_seedqr: &'static str,
    /// Refused (§5): a CompactSeedQR.
    pub companion_refuse_compact_seedqr: &'static str,
    /// Refused (§5): codex32.
    pub companion_refuse_codex32: &'static str,
    /// Refused (§5): a SLIP-39 share.
    pub companion_refuse_slip39: &'static str,
    /// Refused (§5): an extended private key.
    pub companion_refuse_xprv: &'static str,
    /// Refused (§5): a WIF private key.
    pub companion_refuse_wif: &'static str,
    /// Refused (§5): `ur:crypto-seed`.
    pub companion_refuse_ur_seed: &'static str,
    /// Refused (§5): `ur:crypto-bip39`.
    pub companion_refuse_ur_bip39: &'static str,
    /// Refused (§5): a UR key that holds a private key.
    pub companion_refuse_ur_private_key: &'static str,
    /// Refused (§5): an encrypted backup.
    pub companion_refuse_backup: &'static str,
    /// Refused (§5): a KeePass database.
    pub companion_refuse_keepass: &'static str,
    /// Refused (§5): a sealed vault.
    pub companion_refuse_vault: &'static str,
    /// Refused (§5): a threshold carry.
    pub companion_refuse_carry: &'static str,
    /// Refused (§5): `otpauth://`.
    pub companion_refuse_totp: &'static str,
    /// Refused (§5): `otpauth-migration://`.
    pub companion_refuse_totp_migration: &'static str,
    /// Refused (§5): vault entries.
    pub companion_refuse_entries: &'static str,
    /// Text that arrived, shown.
    pub companion_text_title: &'static str,
    /// Keeps the text shown in Received.
    pub companion_keep: &'static str,
    /// An address that arrived, shown.
    pub companion_address_title: &'static str,
    /// The kind of address shown.
    pub companion_address_bitcoin: &'static str,
    /// The kind of address shown.
    pub companion_address_silent: &'static str,
    /// A web address that arrived, shown and never opened.
    pub companion_web_title: &'static str,
    /// Under a web address.
    pub companion_web_not_opened: &'static str,
    /// A BitLocker recovery key that arrived, shown masked.
    pub companion_bitlocker_title: &'static str,
    /// The computer a recovery key is for.
    pub companion_bitlocker_computer: &'static str,
    /// The drive a recovery key is for.
    pub companion_bitlocker_drive: &'static str,
    /// Closes what is shown.
    pub companion_done: &'static str,
    /// Opens the system's document picker.
    pub companion_send_choose: &'static str,
    /// Send's list with nothing in Received.
    pub companion_send_nothing: &'static str,
    /// A file that cannot go as codes: the result's title.
    pub companion_send_not_sent: &'static str,
    /// A file over the envelope's 256 KiB.
    pub companion_send_too_large: &'static str,
    /// A file with nothing in it.
    pub companion_send_empty: &'static str,
    /// The shell has no document picker.
    pub companion_send_no_picker: &'static str,
    /// No code holds the file in the format asked.
    pub companion_send_no_code: &'static str,
    /// The name a file from the picker is sent under: `{}` its extension.
    pub companion_send_picked: &'static str,
    /// Why a file was not sent, in a record.
    pub companion_reason: &'static str,
    /// The QR screen's switch.
    pub companion_animated: &'static str,
    /// Why the switch is on and dimmed.
    pub companion_too_dense: &'static str,
    /// The animated code's progress: frame `{}` of `{}`.
    pub companion_frame_of: &'static str,
    /// The frame rate, which a press steps through: `{}` a number.
    pub companion_rate: &'static str,
    /// Shows the code as UR.
    pub companion_show_as_ur: &'static str,
    /// Shows the code as BBQr.
    pub companion_show_as_bbqr: &'static str,
    /// Shows the code as numbered parts.
    pub companion_show_as_parts: &'static str,
    /// Under a code.
    pub companion_made_psbt_one: &'static str,
    /// Under a code.
    pub companion_made_psbt_frames: &'static str,
    /// Under a code.
    pub companion_made_text_one: &'static str,
    /// Under a code.
    pub companion_made_key_one: &'static str,
    /// Under a code.
    pub companion_made_account: &'static str,
    /// Under a code.
    pub companion_made_bytes_one: &'static str,
    /// Under a code.
    pub companion_made_bytes_frames: &'static str,
    /// Under a code.
    pub companion_made_file_one: &'static str,
    /// Under a code.
    pub companion_made_file_frames: &'static str,
    /// Under a code: `{}` the BBQr file type.
    pub companion_made_bbqr_one: &'static str,
    /// Under a code: the BBQr file type and the frames.
    pub companion_made_bbqr_frames: &'static str,
    /// Under a code.
    pub companion_made_parts: &'static str,
    /// Received with nothing kept.
    pub companion_received_empty: &'static str,
    /// Received's action.
    pub companion_delete_all: &'static str,
    /// Received, when the kept files could not be read.
    pub companion_unreadable: &'static str,
    /// Their state: left as they are until deleted.
    pub companion_unreadable_value: &'static str,
    /// Deletes the unreadable kept files.
    pub companion_unreadable_delete: &'static str,
    /// A record's action.
    pub companion_show_as_qr: &'static str,
    /// A record's action: the system's document creator.
    pub companion_save: &'static str,
    /// A record's action.
    pub companion_delete: &'static str,
    /// The save's answer, in the record.
    pub companion_saved: &'static str,
    /// The save's answer, in the record.
    pub companion_not_saved: &'static str,
    /// Asked once before a file is deleted.
    pub companion_delete_one: &'static str,
    /// Asked once before Delete all.
    pub companion_delete_all_ask: &'static str,
    /// Asked once before the unreadable kept files are deleted.
    pub companion_delete_store_ask: &'static str,
    /// Answers no.
    pub companion_keep_it: &'static str,
    /// How many files, in a record.
    pub companion_files: &'static str,
    /// About: the app's version.
    pub companion_version: &'static str,
    /// About: the build, as the phone reports it.
    pub companion_build: &'static str,
    /// About: the APK signing certificate's SHA-256.
    pub companion_certificate: &'static str,
    /// About: what the app reaches.
    pub companion_network: &'static str,
    /// About: no network access, no permissions.
    pub companion_none: &'static str,
    /// About: the permissions the app asks for.
    pub companion_permissions: &'static str,
    /// About: a permission the system has granted.
    pub companion_granted: &'static str,
    /// About: a permission the system has not granted.
    pub companion_not_granted: &'static str,
    /// About: a fact the phone has not given.
    pub companion_not_reported: &'static str,
    /// About: how to check the APK.
    pub companion_about_check: &'static str,
    /// About: what the certificate's SHA-256 is compared with.
    pub companion_about_certificate: &'static str,
    /// The way back to Home, named in the bar.
    pub companion_home: &'static str,
    /// Home's card: what is kept on this phone.
    pub companion_on_phone: &'static str,
    /// Home's card: the file kept last.
    pub companion_newest: &'static str,
    /// Learn: Faraday's own pages after the boot and maker pages.
    pub companion_learn_faraday: &'static str,
    /// Start from the stick: the one question, asked once.
    pub companion_start_question: &'static str,
    /// Start from the stick: the answer for a Windows computer.
    pub companion_start_windows: &'static str,
    /// Start from the stick: the answer for a Linux computer.
    pub companion_start_linux: &'static str,
    /// Start from the stick: the heading over the computer chosen.
    pub companion_start_computer: &'static str,
    /// Start from the stick: asks the question again.
    pub companion_start_change: &'static str,
    /// Start from the stick, Windows: the heading over the BitLocker pages.
    pub companion_start_bitlocker: &'static str,
    /// Start from the stick: the heading over the maker chosen.
    pub companion_start_maker: &'static str,
    /// Start from the stick: opens the list of makers.
    pub companion_start_choose_maker: &'static str,
    /// Start from the stick: the heading over the pages for when it does not work.
    pub companion_start_trouble: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_dell: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_hp: &'static str,
    /// Start from the stick: the maker's name on the list, which asks
    /// ThinkPad or Other next.
    pub companion_maker_lenovo: &'static str,
    /// Start from the stick, Lenovo: a ThinkPad.
    pub companion_maker_thinkpad: &'static str,
    /// Start from the stick: any other maker, or any other Lenovo.
    pub companion_maker_other: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_asus: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_acer: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_msi: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_framework: &'static str,
    /// Start from the stick: the maker's name on the list.
    pub companion_maker_surface: &'static str,
    /// A guide page: how many of its steps are ticked, of how many.
    pub companion_steps_done: &'static str,
    /// A guide page: clears its ticks.
    pub companion_steps_again: &'static str,
    /// Learn: the pages about Bitcoin that Faraday's apps show.
    pub companion_learn_bitcoin: &'static str,
    /// About: the palettes the screens can be drawn in.
    pub companion_theme: &'static str,
    /// About: the heading over how a release is checked.
    pub companion_check_title: &'static str,
    /// Home row, and the list of kept wallet descriptions (§4.5).
    pub companion_home_descriptions: &'static str,
    /// Home row while a key is kept, and the list of kept BitLocker keys (§4.6).
    pub companion_home_bitlocker: &'static str,
    /// The phone's unlock prompt's title, opening Wallet descriptions (§7).
    pub companion_unlock_descriptions: &'static str,
    /// The unlock prompt's title, opening BitLocker keys.
    pub companion_unlock_keys: &'static str,
    /// The unlock prompt's title, keeping a description that arrived.
    pub companion_unlock_keep_description: &'static str,
    /// The unlock prompt's title, keeping a BitLocker key that arrived.
    pub companion_unlock_keep_key: &'static str,
    /// The page behind the unlock prompt.
    pub companion_unlock_waiting: &'static str,
    /// A kept description's name before it is renamed: its number.
    pub companion_description_name: &'static str,
    /// What another app shared that is nothing read whole.
    pub companion_shared_not_read: &'static str,
    /// Wallet descriptions' button, and its scanner's title.
    pub companion_descriptions_add: &'static str,
    /// Wallet descriptions with none kept.
    pub companion_descriptions_empty: &'static str,
    /// A received description's button (§4.4).
    pub companion_keep_in_descriptions: &'static str,
    /// A record's button: the system's share sheet (§6.3).
    pub companion_share: &'static str,
    /// A description's button, and the page that renames it.
    pub companion_rename: &'static str,
    /// Asked before a kept description is deleted.
    pub companion_delete_description_ask: &'static str,
    /// Back in Windows' sheet title, and a key's Delete (§4.6).
    pub companion_delete_key_ask: &'static str,
    /// Asked before a kept store that cannot be read is deleted.
    pub companion_delete_kept_store_ask: &'static str,
    /// Back in Windows' primary button (§4.6).
    pub companion_delete_the_key: &'static str,
    /// A wallet's script type.
    pub companion_script_legacy: &'static str,
    /// A wallet's script type.
    pub companion_script_nested: &'static str,
    /// A wallet's script type.
    pub companion_script_native: &'static str,
    /// A wallet's script type.
    pub companion_script_taproot: &'static str,
    /// A kind of wallet.
    pub companion_wallet_single: &'static str,
    /// A kind of wallet.
    pub companion_wallet_multisig: &'static str,
    /// A kind of wallet.
    pub companion_wallet_musig: &'static str,
    /// A kind of wallet.
    pub companion_wallet_threshold: &'static str,
    /// A kind of wallet.
    pub companion_wallet_silent: &'static str,
    /// A kind of wallet.
    pub companion_wallet_miniscript: &'static str,
    /// A kind of wallet.
    pub companion_wallet_tree: &'static str,
    /// A wallet's keys: how many sign of how many.
    pub companion_k_of_n: &'static str,
    /// A wallet's keys where it states no number that signs.
    pub companion_n_keys: &'static str,
    /// A description's record: the label over k of n.
    pub companion_keys: &'static str,
    /// A key whose fingerprint the description does not give.
    pub companion_no_fingerprint: &'static str,
    /// A description's record: the label over a key's fingerprint.
    pub companion_key_n: &'static str,
    /// A description's record: the network its keys name.
    pub companion_wallet_network: &'static str,
    /// A description's record: its first receive address.
    pub companion_first_address: &'static str,
    /// A wallet with no address at an index (silent payments).
    pub companion_no_address: &'static str,
    /// BitLocker keys with none kept.
    pub companion_keys_empty: &'static str,
    /// A kept key's card: its drive, and when it was kept.
    pub companion_drive_line: &'static str,
    /// A kept key's button: its digits show while it is held.
    pub companion_hold_to_show: &'static str,
    /// A kept key's one action (§4.6).
    pub companion_back_in_windows: &'static str,
    /// Back in Windows' first line (§4.6), word for word.
    pub companion_key_not_needed: &'static str,
    /// Back in Windows' second line (§4.6), word for word.
    pub companion_key_shown_again: &'static str,
    /// The kept store was not opened.
    pub companion_closed_title: &'static str,
    /// Why: the unlock prompt was cancelled.
    pub companion_closed_cancelled: &'static str,
    /// Why: the phone has no screen lock (§7), word for word.
    pub companion_closed_no_lock: &'static str,
    /// The way to delete a kept store that cannot be read.
    pub companion_delete_kept_store: &'static str,
    /// A description that the reader does not read.
    pub companion_not_description: &'static str,
}

/// The English wording.
pub static EN: Strings = Strings {
    power_secure_boot_off: "Secure Boot is off. Turn it back on before starting Windows.",
    devices_title: "Attached to this computer",
    devices_kvm: "a KVM can show this screen and type on this computer from elsewhere",
    devices_receiver_notice: "Wireless receiver attached. A wireless keyboard's typing can be picked up nearby: use a wired keyboard or the on-screen keyboard.",
    devices_bolt_notice: "Logitech Bolt receiver attached. A Bolt keyboard's typing is encrypted.",
    devices_virtual_machine: "a virtual machine's host sees and can change everything in it",
    devices_virtual_machine_unknown: "This computer",
    devices_real_computer: "Faraday must start on a real computer.",
    devices_this_screen: "This screen",
    devices_tv: "a TV can record what it shows",
    devices_unnamed: "USB device {}",
    devices_check_again: "Check again",
    devices_continue: "Continue anyway",
    devices_continue_ask: "What is listed stays possible while they are attached.",
    devices_continue_yes: "Yes, continue",
    devices_still: "Still attached",
    devices_power_off: "Power off",
    devices_tv_line: "On a TV? Stop: a smart TV can record its screen.",
    seed_missing: "No randomness from the system. Faraday has to restart",
    devices_tv_ask: "Is this screen a TV?",
    devices_tv_yes: "It is a TV",
    devices_tv_no: "It is not a TV",
    // The hub app (docs/HUB.md).
    hub_home: "Home",
    hub_computer: "This computer",
    hub_computer_unknown: "This computer",
    hub_checklist: "Checklist",
    hub_secure_boot_on: "Secure Boot: on",
    hub_secure_boot_off: "Secure Boot: off",
    hub_secure_boot_unknown: "Secure Boot: not read",
    hub_drive_encrypted: "Drive {} encrypted with BitLocker",
    hub_drive_plain: "Drive {} not encrypted",
    hub_drive_suspended: "Drive {} BitLocker suspended by Faraday",
    hub_drive_suspended_other: "Drive {} BitLocker suspended",
    hub_drive_unknown: "Drive {} encryption not read",
    hub_cannot_start_arm: "This computer cannot start a Faraday stick",
    hub_cannot_start_bios: "Started in legacy BIOS mode: UEFI has to be on",
    hub_cannot_start_mac_firmware: "This Mac's firmware cannot start Faraday",
    hub_cannot_start_mac_yet: "This Mac cannot start Faraday yet",
    hub_mac_t2: "Apple T2 Security Chip",
    hub_item_start_mac: "Start this Mac from the stick",
    hub_state_mac_option: "Option key at power-on",
    hub_state_mac_t2: "Startup Security, then the Option key",
    hub_state_mac_unknown: "Option key at power-on · Startup Security on a T2 Mac",
    hub_mac_t2_only: "Only on a Mac with the T2 chip",
    hub_mac_lower: "Lower Startup Security",
    hub_mac_recovery: "macOS Recovery: hold Command-R at power-on",
    hub_mac_utility_lower: "Utilities, Startup Security Utility: No Security, and Allow booting from external or removable media",
    hub_mac_option: "Shut down. Hold Option at power-on and choose EFI Boot",
    hub_mac_firmware_password: "A firmware password, when one is set, is asked for first",
    hub_mac_afterwards: "Afterwards: Startup Security back to Full",
    hub_mac_utility_full: "macOS Recovery, Startup Security Utility: Full Security, and Disallow booting from external or removable media",
    hub_learn_mac_intel: "Starting an Intel Mac from the stick",
    hub_which_title: "For which computer?",
    hub_which_pc: "A PC or an Intel Mac",
    hub_which_apple: "A Mac with M1 or M2",
    hub_item_make: "Make a Faraday stick",
    hub_item_bitlocker: "BitLocker",
    hub_item_start: "Start this computer from the stick",
    hub_item_secure_boot: "Turn Secure Boot back on",
    hub_item_resume: "Resume BitLocker",
    hub_state_no_stick: "No Faraday stick attached",
    hub_state_stick: "Faraday stick attached · {}",
    hub_state_protected: "Protected",
    hub_state_suspended: "Suspended by Faraday",
    hub_state_suspended_other: "Suspended",
    hub_state_resumed: "Resumed",
    hub_resume_needs_secure_boot: "Needs Secure Boot on",
    hub_resume_needs_secure_boot_off: "Needs Secure Boot off, as it was",
    hub_resume_needs_restart: "Needs Windows restarted first",
    hub_row_pi: "Make a Pi card",
    hub_row_transfer: "Transfer",
    hub_row_desktop: "Try the desktop app",
    hub_row_desktop_sub: "Test seeds only",
    hub_row_install: "Install",
    hub_row_installed: "Installed",
    hub_row_update: "Update the installed Faraday",
    hub_row_about: "About",
    hub_row_learn: "Learn",
    hub_disks_stick: "Which stick?",
    hub_disks_card: "Which card?",
    hub_pi_for: "For a Raspberry Pi 3B, 3B+ or Zero 2 W with the Waveshare 2.8 inch screen",
    hub_pi_cameras: "Cameras: a USB webcam, or Camera Module v1 or v2",
    hub_no_stick_image: "Faraday Starter carries no stick image",
    hub_no_pi_image: "Faraday Starter carries no Pi image",
    hub_no_writer: "This computer cannot write a stick from here",
    hub_no_disks_stick: "No USB stick attached",
    hub_no_disks_card: "No SD card or USB stick attached",
    hub_disk_unnamed: "Disk",
    hub_on_faraday: "Faraday stick · {}",
    hub_on_empty: "Empty",
    hub_on_volume: "1 volume",
    hub_on_volumes: "{} volumes",
    hub_on_files_it: "files on it",
    hub_on_files_them: "files on them",
    hub_on_no_files: "no files",
    hub_refused_mounted: "Used by this computer: mounted at {}",
    hub_refused_swap: "Used by this computer: swap",
    hub_refused_system: "Holds this computer's system",
    hub_refused_small: "Smaller than the image: needs {}",
    hub_size_gb: "{} GB",
    hub_size_mb: "{} MB",
    hub_mode_title: "What to write?",
    hub_mode_whole_stick: "New Faraday stick",
    hub_mode_whole_card: "New Faraday card",
    hub_mode_whole_sub: "Everything on it is lost",
    hub_mode_keep: "Replace Faraday, keep data",
    hub_mode_keep_sub: "Only the boot partition is written",
    hub_mode_keep_fact: "This computer can read and change the data partition while the stick is attached.",
    hub_mode_same: "Already this build",
    hub_mode_boot_only: "The image is a boot partition alone",
    hub_continue: "Continue",
    hub_confirm_title: "Confirm",
    hub_stick: "Stick",
    hub_card: "Card",
    hub_size: "Size",
    hub_device: "Device",
    hub_serial: "Serial",
    hub_serial_none: "None",
    hub_on_it: "On it",
    hub_writes: "Writes",
    hub_build: "Build",
    hub_build_unknown: "Not known",
    hub_img_sha: "SHA-256 of the image",
    hub_image_sha: "SHA-256 of this image",
    hub_image_file: "Image file",
    hub_dev_image: "Not Faraday Starter's own image",
    hub_keep_newer: "This stick holds a newer Faraday: {}",
    hub_keep_dev: "Writes a dev build, with a serial console",
    hub_not_confirmed_stick: "Not the stick confirmed: it was removed or changed",
    hub_not_confirmed_card: "Not the card confirmed: it was removed or changed",
    hub_erase_and_write: "Erase and write",
    hub_write: "Write",
    hub_writing_title: "Writing",
    hub_of: "{} of {}",
    hub_written_of: "Written {}",
    hub_reading_back: "Reading back · {}",
    hub_done_title: "Written",
    hub_failed_title: "Not written",
    hub_pull_stick: "The stick can be pulled out.",
    hub_pull_card: "The card can be pulled out.",
    hub_done: "Done",
    hub_try_again: "Try again",
    hub_guide_placeholder: "Learn: {}",
    hub_learn_start_windows: "Starting a Windows computer from the stick",
    hub_learn_start_linux: "Starting a Linux computer from the stick",
    hub_learn_bitlocker: "BitLocker and starting from a stick",
    hub_learn_no_start: "When the stick does not start",
    hub_learn_keep_data: "Replacing Faraday on a stick",
    hub_tpm_line: "A disk here is unlocked by the TPM: have its passphrase ready",
    hub_bitlocker_step: "Do the BitLocker step",
    hub_restart: "Restart into the BIOS",
    hub_restart_stick: "Restart from the stick",
    hub_restart_save: "This restarts the computer now. Save your work first.",
    hub_stick_restart_failed: "Restart from the stick did not happen: {}",
    hub_entry_kept: "The BIOS's Faraday stick entry remains: {}",
    hub_restart_now: "Restart",
    hub_cancel: "Cancel",
    hub_secure_boot_turn_off: "Turn Secure Boot off",
    hub_secure_boot_already_off: "Secure Boot is already off",
    hub_fast_boot: "Turn Fast Boot off where it hides USB",
    hub_boot_from_stick: "Start from the stick",
    hub_boot_how: "The boot menu key",
    hub_boot_how_windows: "The boot menu key, or Use a device in Advanced startup",
    hub_boot_menu_label: "In the boot menu",
    hub_boot_menu_value: "something like {}",
    hub_boot_menu_default: "UEFI: {}, Partition 1",
    hub_boot_menu_efi_usb_device: "EFI USB Device ({})",
    hub_boot_menu_hp: "USB Hard Drive - {}",
    hub_afterwards: "Turn Secure Boot back on",
    hub_afterwards_windows: "Turn Secure Boot back on before starting Windows",
    hub_on_a_phone: "On a phone",
    hub_install_desktop: "Also install the desktop app, for testing with test seeds only",
    hub_install_now: "Install",
    hub_installing: "Installing",
    hub_installed_in: "Installed in {}",
    hub_installed_version: "Installed version",
    hub_this_version: "This version",
    hub_installed_app: "Faraday Starter",
    hub_installed_desktop: "Desktop app",
    hub_installed_menu: "Menu entry",
    hub_installed_desktop_menu: "Desktop app's menu entry",
    hub_installed_icon: "Icon",
    hub_installed_entry: "Installed apps entry",
    hub_installed_reminder: "Logon reminder",
    hub_not_installed: "Not installed",
    hub_uninstall: "Uninstall",
    hub_uninstall_ask: "Uninstall Faraday Starter?",
    hub_uninstall_keeps: "Removes the files listed. Received files and settings stay.",
    hub_uninstall_suspended: "BitLocker stays suspended until it is resumed.",
    hub_why_not_carried: "Faraday Starter does not carry what was confirmed. Nothing was written.",
    hub_why_changed: "{} is no longer the disk that was confirmed (serial {}, {}). Nothing was written.",
    hub_why_no_serial: "none",
    hub_why_bytes: "{} bytes",
    hub_why_mounted: "A partition on it is mounted at {}. Nothing was written.",
    hub_why_swap: "A partition on it is in use as swap. Nothing was written.",
    hub_why_system: "It holds this computer's system. Nothing was written.",
    hub_why_boot_only: "A boot partition alone is written only over a Faraday stick, keeping its data.",
    hub_why_too_small: "The disk ({}) is smaller than the image ({}). Nothing was written.",
    hub_why_payload_sum: "The payload file does not match the manifest. Nothing was written.",
    hub_why_image_changed: "{} has changed since the app started: its SHA-256 is now {}. Nothing was written.",
    hub_why_gone: "The disk is no longer attached. Nothing was written.",
    hub_why_bus: "udisks2 did not answer: {}",
    hub_why_unmounting: "Unmounting it failed: {}. Nothing was written.",
    hub_why_open_write: "Opening it to write failed: {}. Nothing was written.",
    hub_why_open_read: "Opening it to read back failed: {}",
    hub_why_device: "{}: {}",
    hub_why_flush: "Flushing {} failed: {}",
    hub_why_not_stick: "{} is not a Faraday stick: {}. Nothing was written.",
    hub_why_not_stick_image: "The image is not a Faraday stick image: {}",
    hub_why_boot_larger: "The image's boot partition ({}) is larger than the stick's ({})",
    hub_why_boot_label: "The image's boot partition is not a FAT32 filesystem labelled OSKBOOT",
    hub_why_boot_short: "The boot partition image is shorter than one sector",
    hub_why_table_changed: "The partition table on {} changed",
    hub_why_reading_table: "Reading the partition table failed: {}",
    hub_why_back_ends: "Reading back {}: the disk ends at byte {}",
    hub_why_back_differs: "Reading back {}: what was read differs from what was written at byte {}",
    hub_why_back_failed: "Reading back {} failed: {}",
    hub_why_source_read: "Reading the source failed: {}",
    hub_why_write_failed: "Writing failed: {}",
    hub_why_payload_changed: "The payload file changed while it was read",
    hub_why_damaged: "The .xz file is damaged: {}",
    hub_why_longer: "The decompressed image is longer than {} bytes",
    hub_why_shorter: "The decompressed image is {} bytes, not {}",
    hub_why_wrong_sum: "The decompressed image's SHA-256 is {}, not the expected one",
    hub_why_no_home: "The home folder is not known",
    hub_why_no_desktop: "Faraday Starter carries no desktop app",
    hub_why_cache_sum: "The desktop app does not match the manifest: its SHA-256 is {}",
    hub_why_file: "{}: {}",
    hub_why_in_the_way: "{} is another program's, and is in the way",
    hub_why_start: "Starting {} failed: {}",
    hub_why_restart: "Restarting into the BIOS failed: {}",
    hub_why_declined: "Administrator permission was declined. Nothing was changed.",
    hub_why_unsupported: "Not on this computer",
    hub_why_missing: "{} is not installed",
    hub_why_boot_entry: "efibootmgr failed: {}",
    hub_why_no_stick: "No Faraday stick is attached",
    hub_menu_name: "Faraday Starter",
    hub_publisher: "Faraday",
    hub_menu_desktop_name: "Faraday Desktop",
    hub_menu_desktop_comment: "Test seeds only",
    hub_fault_reading: "reading the partition table failed: {}",
    hub_fault_no_header: "it has no GPT header",
    hub_fault_not_gpt: "its partition table is not GPT",
    hub_fault_header_damaged: "its GPT header is damaged",
    hub_fault_header_checksum: "its GPT header's checksum is wrong",
    hub_fault_entries_damaged: "its GPT entries are damaged",
    hub_fault_entries_past_end: "its GPT entries lie past the end of the disk",
    hub_fault_entries_checksum: "its GPT entries' checksum is wrong",
    hub_fault_disk_uuid: "its disk UUID is not a Faraday stick's",
    hub_fault_partitions: "it has {} partitions, not two",
    hub_fault_boot_name: "its first partition is not named OSKBOOT",
    hub_fault_boot_type: "its first partition is not a Faraday EFI system partition",
    hub_fault_data_type: "its second partition is not a Faraday OSKDATA partition",
    hub_fault_boot_damaged: "its OSKBOOT partition is damaged",
    hub_fault_boot_small: "its OSKBOOT partition ({}) is smaller than the image ({})",
    hub_fault_overlap: "its partitions overlap",
    hub_fault_boot_past_end: "its boot partition runs past its end",
    hub_version: "Version",
    hub_release: "Release",
    hub_commit: "Commit",
    hub_payload: "Carried in this app",
    hub_no_payload: "Faraday Starter carries no images",
    hub_sha_xz: "SHA-256",
    hub_sha_inner: "SHA-256 of {}",
    hub_release_key: "Release key",
    hub_check_title: "Checking this file",
    hub_check_text: "Check this file against SHA256SUMS and its signature, SHA256SUMS.asc, from the release, with the release key above.",
    hub_check_cmd_import: "gpg --fingerprint F1536BE07E234CFD",
    hub_check_cmd_verify: "gpg --verify SHA256SUMS.asc SHA256SUMS",
    hub_check_cmd_sums: "sha256sum -c --ignore-missing SHA256SUMS",
    hub_check_windows: "On Windows: Get-FileHash -Algorithm SHA256 for the sum, Gpg4win for the signature.",
    hub_unsigned_title: "Not code-signed",
    hub_unsigned_text: "A code-signing certificate names a verified person or company, and Faraday is published under a pseudonym. Windows warns on first run; More info, then Run anyway, opens it. The sums and the signature are the check.",
    hub_drive: "Drive",
    hub_drive_unread: "BitLocker not read",
    hub_protection: "Protection",
    hub_protection_none: "Not encrypted",
    hub_protection_suspended: "Suspended",
    hub_protection_unknown: "Not read",
    hub_suspended_by_faraday: "Suspended by Faraday",
    hub_secure_boot_when_suspended: "Secure Boot when suspended",
    hub_secure_boot_now: "Secure Boot now",
    hub_yes: "Yes",
    hub_no: "No",
    hub_why_no_admin: "An administrator of this computer has to do this step.",
    hub_why_managed: "BitLocker was not suspended ({}). This computer's BitLocker is managed: its administrator has to suspend it.",
    hub_why_bitlocker: "BitLocker refused {}: {}",
    hub_why_not_encrypted: "The drive is not encrypted. There is nothing to suspend.",
    hub_why_not_read: "BitLocker on the system drive could not be read. The drive is treated as protected.",
    hub_guard_encrypted: "Drive {} is encrypted with BitLocker",
    hub_guard_undone: "Protected again after Faraday suspended it",
    hub_show_key: "Show the recovery key",
    hub_key_also_kept: "Also kept in",
    hub_key_also_kept_where: "Microsoft account: aka.ms/myrecoverykey",
    hub_key_in_hand: "I have my recovery key",
    hub_extra_safe: "Extra safe: suspend BitLocker",
    hub_state_key_kept: "Recovery key kept",
    hub_waiting_permission: "Waiting for administrator permission",
    hub_recovery_key: "Recovery key",
    hub_key_id: "Key ID",
    hub_key_photo: "Or take a photo of this screen.",
    hub_key_save: "Save to {}",
    hub_key_saving: "Saving",
    hub_key_saved: "Saved to {}",
    hub_key_kept: "I have kept it",
    hub_key_type: "Type groups {} and {} of the key",
    hub_key_group: "Group {}",
    hub_key_wrong: "Group {} does not match the key.",
    hub_key_show_again: "Show the key again",
    hub_suspend: "Suspend BitLocker",
    hub_suspending: "Suspending",
    hub_resuming: "Resuming",
    hub_guard_done: "Windows starts without the key until BitLocker is resumed. Resume it here once Secure Boot is back on.",
    hub_guard_open_again: "Open Faraday again to do that.",
    hub_guard_resumed: "BitLocker resumed",
    hub_other_found: "An encrypted drive Faraday does not manage: {}",
    hub_other_confirm: "I have its rescue disk or password",
    hub_state_other: "Not managed by Faraday",
    hub_key_file: "BitLocker recovery key {}.txt",
    hub_uninstall_suspended_where: "BitLocker stays suspended until it is resumed: Control Panel, BitLocker Drive Encryption, Resume protection.",
    hub_concern_tv: "{} is a TV. A TV can record what it shows: use another screen or computer.",
    hub_concern_kvm: "{} is a KVM switch. It can capture the screen and type: connect the screen and keyboard directly.",
    hub_concern_vm: "{} is a virtual machine. Faraday must start on the real computer, and this app cannot see the host's screens and devices.",
    hub_concern_remote: "{} is a remote session. What it shows is sent to another computer.",
    hub_concern_wireless_display: "{} is a wireless display. What it shows goes over the air.",
    hub_concern_screen: "The screen",
    hub_concern_device: "A USB device",
    hub_concern_computer: "This computer",
    hub_concern_session: "This session",
    hub_check_again: "Check again",
    hub_continue_anyway: "Continue anyway",
    hub_let_through_ask: "Continue with {} attached?",
    hub_let_through_until: "Until this app is closed.",
    hub_learn_secure_boot_bitlocker: "What Secure Boot and BitLocker are",
    hub_learn_screens: "Screens and devices that can record",
    hub_learn_boot_entry: "About the boot entry",
    hub_remove_entry: "Remove the Faraday boot entry",
    hub_remove_entry_sub: "In this computer's boot list",
    hub_removing_entry: "Removing",
    backup_your_copy_scans_but: "Your copy scans but word {} differs",
    backup_your_copy_scans: "Your copy scans and matches the seed",
    backup_share_of: "share {} of {}",
    backup_the_backup_sheet: "the backup sheet",
    backup_envelope_the: "Envelope {}: the seed of key {} {}, with {}",
    backups_wallet: "Wallet",
    backups_plan: "Plan",
    backups_in_locked: "In {} · locked",
    backups_place: "Place {}",
    backups_vault: "Vault",
    backups_vault_n: "Vault {}",
    backups_of_files: "{} of files",
    backups_watch_only: "Watch-only software",
    backups_own_devices: "Own devices",
    backups_checked: "Checked",
    backups_not_checked: "Not checked",
    backups_not_here: "Not here",
    backups_unprotected: "{}, unprotected",
    backups_shown: "Shown",
    backups_not_shown: "Not shown",
    backups_not_seen: "Not seen",
    backups_nothing: "Nothing",
    backups_not_made: "Not made",
    backups_a_vault_is: "A vault is locked",
    backups_not_saved_yet: "Not saved yet",
    backups_changed_since: "changed since written",
    backups_unchanged: "unchanged",
    backups_not_backed: "Not backed up",
    backups_paper_checked: "paper ×{} checked",
    backups_paper_of_checked: "paper {} of {} checked",
    backups_files: "files {}",
    backups_planned: "planned",
    backups_backup: "Backup: {}",
    backups_on_2: "On {}, {}",
    backups_on: "On {}",
    backups_backups: "Backups",
    backups_no_wallet: "No wallet",
    backups_no_backup_plan: "No backup plan",
    backups_back_up: "Back up",
    backups_wallet_of: "Wallet {} of {}",
    backups_previous: "Previous",
    backups_next: "Next",
    bip85_load_a_key_first: "Load a key first",
    bip85_bip_85_index: "BIP-85 {} · {} · index {}",
    bip85_no_vault_is_open: "No vault is open",
    bip85_is_in_the_vault: "{} is in the vault",
    bip85_remove_the_first: "Remove the {} first",
    bip85_key_added: "Key {} added",
    bip85_a_child_seed: "A child seed's words",
    bip85_a_private_key: "A private key",
    bip85_secret_bytes: "Secret bytes",
    bip85_a_password: "A password",
    bip85_whoever_has_it: "Whoever has it has what it protects or spends",
    bip85_key: "Key",
    bip85_application: "Application",
    bip85_index: "Index",
    bip85_value: "Value",
    bip85_wallets: "Wallets",
    bip85_derive_with_bip: "Derive with BIP-85",
    bip85_words: "{} words",
    bip85_bytes: "{} bytes",
    bip85_characters: "{} characters",
    bip85_not_chosen: "Not chosen",
    bip85_bip_85_turns: "BIP-85 turns one key you already back up into many others: child seeds for other wallets, passwords, keys. Each comes back from the same key, application and index, so the key's backup is theirs too.",
    bip85_a_child_seed_is: "A child seed is a key of its own, for another wallet or another device. It reveals nothing about this key.",
    bip85_a_password_for: "A password for a website or a program, as long as it allows. Keep it in a vault entry; the index makes the next one.",
    bip85_the_value_is_made: "The value is made the way BIP-85 specifies, so any implementation gives the same one from the same key.",
    bip85_index_0_is_the: "Index 0 is the first of its kind; each number gives another. Write down the index with what you use it for: it is all that is needed to make the value again.",
    bip85_the_value_is: "The value is a secret. It goes into the open vault, sealed under its passphrase; out of the vault it goes only after the warning.",
    bip85_to: "{} to {}",
    bip85_an_index_is_0: "An index is 0 to 2147483647",
    bip85_type_the_digits: "Type the digits · Enter continues",
    bip85_continue: "Continue",
    bip85_path: "Path",
    bip85_hide: "Hide",
    bip85_show: "Show",
    bip85_save_into: "Save into {}",
    bip85_load_as_a_key: "Load as a key",
    bip85_out_unprotected: "Out unprotected…",
    boot_import_can_sign: "Can sign",
    boot_import_of_keys: "{} of {} keys here · {} more needed",
    boot_import_watch_only: "Watch-only",
    boot_import_vault_inline: "vault",
    boot_import_vaults: "vaults",
    boot_import_psbt: "PSBT",
    boot_import_psbts: "PSBTs",
    boot_import_other_file: "other file",
    boot_import_other_files: "other files",
    boot_import_no_files: "No files read",
    boot_import_not_a_psbt: "Not a PSBT, a descriptor, an xpub or a signed message",
    boot_import_no_qr_code: "No QR code found",
    boot_import_its_codes: "Its codes are not complete",
    boot_import_qr: "{} QR {} · {}",
    boot_import_code: "code",
    boot_import_codes: "codes",
    boot_import_the_was: "the {} was removed before it was read",
    boot_import_wallet: "Wallet",
    boot_import_being_read: "Being read · {}",
    boot_import_not_a_kind: "Not a kind of file Faraday reads · {}",
    boot_import_not_read: "Not read: {}",
    boot_import_vault_open: "Vault, open · {}",
    boot_import_vault_n: "Vault · {}",
    boot_import_imported: "Imported {} · {} · {}",
    boot_import_wallet_inline: "wallet",
    boot_import_wallets_inline: "wallets",
    boot_import_key: "key",
    boot_import_keys: "keys",
    boot_import_file: "file",
    boot_import_files_inline: "files",
    boot_import_read_from: "Read from {}",
    boot_import_pull_the: "Pull the {} to continue",
    boot_import_open: "Open · {}",
    boot_import_locked: "Locked · {}",
    boot_import_choose: "Choose what to import",
    boot_import_not_now: "Not now",
    boot_import_import: "Import",
    boot_import_unlock: "Unlock",
    boot_import_unlocking: "Unlocking · {}",
    boot_import_wallets: "Wallets",
    boot_import_keys_with: "Keys with no wallet here",
    boot_import_files: "Files",
    catalog_new_key: "New key",
    catalog_dice_coins: "Dice, coins, cards, hex, camera or a mix",
    catalog_add_a_key: "Add a key",
    catalog_type_its_words: "Type its words, any BIP-39 list, with a passphrase",
    catalog_scan_a_seedqr: "Scan a SeedQR",
    catalog_standard_or: "Standard or compact",
    catalog_slip_39_shares: "SLIP-39 shares",
    catalog_a_new_key_dealt: "A new key dealt as m of n shares",
    catalog_restore_from: "Restore from SLIP-39",
    catalog_shares_typed: "Shares typed or scanned back into a key",
    catalog_a_secret_or: "A secret or its shares typed into a key",
    catalog_seed_xor: "Seed XOR",
    catalog_parts_combined: "Parts combined into a key",
    catalog_explore_a_key: "Explore a key",
    catalog_its_xpubs_and: "Its xpubs and addresses at any path",
    catalog_child_seeds: "Child seeds and passwords",
    catalog_derived_from: "Derived from a key here",
    catalog_vanity_address: "Vanity address",
    catalog_a_first_address: "A first address that begins your way",
    catalog_silent_payments: "Silent payments",
    catalog_one_address: "One address to give out",
    catalog_keys_and_seeds: "Keys and seeds",
    catalog_lightning_node: "Lightning node key",
    catalog_from_a_key: "From a key here, or an LND aezeed",
    catalog_single_key: "Single-key wallet",
    catalog_native_segwit: "Native SegWit, Taproot, nested or legacy",
    catalog_multisig: "Multisig",
    catalog_k_of_n_keys: "k of n keys, a quorum signs",
    catalog_taproot_multisig: "Taproot multisig",
    catalog_k_of_n_in_one: "k of n in one script leaf",
    catalog_all_keys_one: "All keys, one signature on chain",
    catalog_frost_threshold: "FROST threshold",
    catalog_any_m_of_n: "Any m of n shares, one signature on chain",
    catalog_miniscript_and: "Miniscript and inheritance",
    catalog_load_one_from: "Load one from its descriptor",
    catalog_restore_an: "Restore an existing wallet",
    catalog_descriptor_wallet: "Descriptor, wallet file, BIP 129 record or split sheets",
    catalog_wallets: "Wallets",
    catalog_back_up_a_wallet: "Back up a wallet",
    catalog_seeds_by_hand: "Seeds by hand, sheets, envelopes, codex32 shares",
    catalog_sign_a_transaction: "Sign a transaction",
    catalog_a_psbt_from: "A PSBT from Files or a QR code",
    catalog_nonce_check: "Nonce check",
    catalog_every_signature: "Every signature recomputed and checked",
    catalog_decode_a_transaction: "Decode a transaction",
    catalog_what_it_spends: "What it spends and pays, field by field",
    catalog_transactions: "Transactions",
    catalog_check_an_address: "Check an address",
    catalog_scan_it_the: "Scan it; the loaded wallets are searched",
    catalog_sign_a_message: "Sign a message",
    catalog_with_a_single: "With a single-key wallet's key here",
    catalog_messages: "Messages",
    catalog_check_a_signed: "Check a signed message",
    catalog_against_its: "Against its address",
    catalog_vaults: "Vaults",
    catalog_keys_wallets: "Keys, wallets, entries and notes under passphrases",
    catalog_totp_codes: "TOTP codes",
    catalog_codes_from: "Codes from the entries in open vaults",
    catalog_diceware_passphrase: "Diceware passphrase",
    catalog_rolled_with: "Rolled with dice in Create a vault",
    catalog_gpg_key: "GPG key",
    catalog_make_sign_revoke: "Make, sign, revoke, paperkey",
    catalog_secure_boot: "Secure Boot keys",
    catalog_pk_kek_and: "PK, KEK and db; sign an EFI image",
    catalog_keepass_export: "KeePass export",
    catalog_a_vault_entry: "A vault entry as a sealed database",
    catalog_vaults_and: "Vaults and other keys",
    catalog_open_an_opensigner: "Open an OpenSigner backup",
    catalog_an_oskb_with: "An .oskb with its passphrase",
    catalog_hashes: "Hashes",
    catalog_sha_256_sha: "SHA-256, SHA-256d and HASH160",
    catalog_encodings: "Encodings",
    catalog_what_a_string: "What a string is encoded in, and its value",
    catalog_descriptor: "Descriptor checksum",
    catalog_a_descriptor: "A descriptor's checksum, and whether it holds",
    catalog_convert_an: "Convert an xpub",
    catalog_every_spelling: "Every spelling of one extended key",
    catalog_units: "Units",
    catalog_one_amount: "One amount in all four units",
    catalog_calculators: "Calculators",
    catalog_miniscript: "Miniscript compiler",
    catalog_a_policy_compiled: "A policy compiled into a descriptor",
    catalog_bip_39_words: "BIP-39 words",
    catalog_2048_words: "2048 words, each with its number and its 11 bits",
    catalog_eff_large_list: "EFF large list",
    catalog_7776_words: "7776 words, each with its number and its five dice",
    catalog_eff_short_list_1: "EFF short list 1",
    catalog_1296_words_each: "1296 words, each with its number and its four dice",
    catalog_word_lists: "Word lists",
    catalog_eff_short_list: "EFF short list 2",
    catalog_1296_words: "1296 words with unique three-letter starts, each with its number and its four dice",
    catalog_qr: "QR",
    catalog_scan_a_qr_code: "Scan a QR code",
    catalog_single_or_animated: "Single or animated, into Files",
    catalog_load_a_key: "Load a key first",
    catalog_at_the_end: "At the end of signing",
    catalog_load_a_wallet: "Load a wallet first",
    catalog_needs_a_single: "Needs a single-key wallet with its key here",
    catalog_copy_a_signed: "Copy a signed message in first",
    catalog_make_a_vault: "Make a vault first",
    catalog_unlock_a_vault: "Unlock a vault first",
    catalog_copy_an_oskb: "Copy an .oskb in first",
    chart_edit_that_does: "That does not fit in this vault's slot size",
    chart_edit_a_new_place: "a new place",
    chart_edit_place: "Place {}",
    chart_edit_vault: "Vault {}",
    chart_edit_of_files: "{} of files",
    chart_edit_watch_only: "Watch-only software",
    chart_edit_save_into: "Save {} into {}",
    chart_edit_as_a_file: "{} as a file",
    chart_edit_at: "{} at {}",
    chart_edit_move_to: "Move {} to {}",
    chart_edit_remove: "Remove {}",
    chart_edit_off_the: "{} off the plan",
    chart_edit_checked: "Checked today",
    chart_edit_vault_stick: "Vault {} stick",
    codes_entry: "Entry",
    codes_now_utc: "Now · UTC{}",
    codes_not_known: "Not known",
    codes_set_the_time: "Set the time",
    codes_unreadable: "Unreadable",
    codes_totp_codes: "TOTP codes",
    codes_no_entries_with: "No entries with a TOTP secret",
    codes_scan_a_setup: "Scan a setup code",
    codes_add_an_entry: "Add an entry",
    compact_spend: "Spend",
    compact_add_a_key: "Add a key",
    compact_back_up: "Back up",
    compact_sign_a_message: "Sign a message",
    compact_check_a_message: "Check a message",
    compact_make_a_new: "Make a new wallet",
    compact_restore_an: "Restore an existing wallet",
    compact_set_the_time: "Set the time",
    compact_totp_codes: "TOTP codes",
    compact_create_a_vault: "Create a vault",
    compact_new_key: "New key",
    compact_bip_85: "BIP-85",
    compact_silent_payments: "Silent payments",
    compact_explore_a_key: "Explore a key",
    compact_lightning: "Lightning",
    compact_vanity_address: "Vanity address",
    compact_transaction: "Transaction",
    compact_at_a_glance: "At a glance",
    compact_vault_contents: "Vault contents",
    compact_home: "Home",
    compact_nothing_loaded: "Nothing loaded",
    compact_key_inline: "key",
    compact_keys: "keys",
    compact_wallet: "wallet",
    compact_wallets_inline: "wallets",
    compact_open_3: "{} {} open",
    compact_vault: "vault",
    compact_vaults_inline: "vaults",
    compact_bring_in: "Bring in",
    compact_open_2: "Open",
    compact_work: "Work",
    compact_write_out: "Write out",
    compact_lock: "Lock",
    compact_scan: "Scan",
    compact_wallets: "Wallets",
    compact_vaults: "Vaults",
    compact_locked: "{} locked",
    compact_open: "{} open",
    compact_files: "Files",
    compact_learn: "Learn",
    compact_backups: "Backups",
    compact_not_backed: "{} not backed up",
    compact_tools: "Tools",
    compact_transfer: "Transfer",
    compact_settings: "Settings",
    compact_done: "Done",
    compact_none: "none",
    compact_no: "no {}",
    compact_no_wallet_loaded: "No wallet loaded",
    compact_continue_signing: "Continue signing {}",
    compact_review: "Review {}",
    compact_rename: "Rename",
    compact_of_signatures: "{} of {} signatures here",
    compact_can_spend: "Can spend",
    compact_scan_a_key: "Scan a key",
    compact_backup: "{} backup {}",
    compact_place: "place",
    compact_places: "places",
    compact_plan_in_locked: "plan in {}, locked",
    compact_no_backup_plan: "no backup plan",
    compact_first_receive: "First receive address · 0/0",
    compact_key: "Key",
    compact_here: "Here",
    compact_not_here: "Not here",
    compact_load_from_vault: "Load from vault",
    compact_add_its_key: "Add its key",
    compact_address: "Address",
    compact_labels_handed: "Labels handed out",
    compact_address_labels: "Address, labels and payments",
    compact_show_wallet: "Show wallet QR",
    compact_remove_from: "Remove from session",
    compact_about_this: "About this page",
    compact_for: "For {}",
    compact_for_key: "For key {}",
    compact_other_languages: "Other languages",
    compact_other_lists: "{} · other lists",
    compact_other_forms: "Other forms",
    compact_scan_a_seedqr: "Scan a SeedQR",
    compact_word: "Word {}",
    compact_type_the_next: "Type the next word",
    compact_now_its_tone: "Now its tone",
    compact_no_word_starts: "No word starts with that",
    compact_words: "Words",
    compact_bip_39_passphrase: "BIP-39 passphrase",
    compact_add_key: "Add key",
    compact_make_a_new_key: "Make a new key",
    compact_clear: "Clear",
    compact_words_of_one: "Words of one share",
    compact_nothing_in: "Nothing in yet",
    compact_1_in: "1 in",
    compact_in: "{} in",
    compact_passphrase: "Passphrase",
    compact_add_this_one: "Add this one",
    compact_scan_a_qr_code: "Scan a QR code",
    compact_make_the_key: "Make the key",
    create_single_key_native: "Single key · native SegWit",
    create_single_key_taproot: "Single key · Taproot",
    create_single_key_nested: "Single key · nested SegWit",
    create_single_key_legacy: "Single key · legacy",
    create_multisig_native: "Multisig · native SegWit",
    create_multisig_nested: "Multisig · nested SegWit",
    create_multisig_legacy: "Multisig · legacy",
    create_multisig_taproot: "Multisig · Taproot",
    create_threshold_frost: "Threshold · FROST",
    create_all_keys_musig2: "All keys · MuSig2",
    create_bc1q_addresses: "bc1q addresses; what most wallets make",
    create_bc1p_addresses: "bc1p addresses; the smallest signatures",
    create_3_addresses: "3… addresses, for services that cannot pay bc1",
    create_1_addresses: "1… addresses, for old software only",
    create_several_keys: "Several keys, a quorum signs; Sparrow, Coldcard, SeedSigner",
    create_multisig_with_3: "Multisig with 3… addresses",
    create_multisig_with: "Multisig with 3… addresses, old style; avoid unless required",
    create_multisig_on: "Multisig on Taproot, one script leaf; Sparrow and Nunchuk",
    create_any_m_of_n_shares: "Any m of n shares sign one Taproot key; single-key on chain",
    create_every_key_signs: "Every key signs together, one Taproot key; single-key on chain",
    create_a_share_has: "A share has no account xpub",
    create_not_a_derivation: "Not a derivation path: {}",
    decode_other_script: "Other script",
    decode_final: "final",
    decode_locktime_on: "locktime on, no replace-by-fee",
    decode_relative_time: "relative time lock, replace-by-fee",
    decode_relative_block: "relative block lock, replace-by-fee",
    decode_0_none: "0 · none",
    decode_block_height: "{} · block height",
    decode_unix_time: "{} · Unix time",
    decode_btc: "{} BTC",
    explore_load_a_key: "Load a key first",
    explore_the_public: "The public side of a loaded key at any path: the extended public key, the public key and the address there. Compare them with what another wallet shows for the same path. Nothing private is shown.",
    explore_path: "Path",
    explore_index_1_2: "Index -1",
    explore_index_1: "Index +1",
    explore_native_segwit: "Native SegWit",
    explore_taproot: "Taproot",
    explore_nested_segwit: "Nested SegWit",
    explore_legacy: "Legacy",
    explore_script: "Script",
    explore_address: "Address",
    explore_public_key: "Public key",
    explore_extended_key: "Extended key",
    explore_wallets: "Wallets",
    explore_explore: "Explore",
    family_typed_words: "Typed words",
    family_finish_the_backup: "Finish the backup first",
    family_carry_on_the: "Carry on the spend · Signatures {} of {}",
    family_sign_the_psbt: "Sign the PSBT from the {}",
    family_spend_from_this: "Spend from this wallet",
    family_finished_transaction: "Finished transaction",
    family_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    family_key_already: "Key {} · already here",
    family_wallet_description_read: "Wallet description read · a wallet is open already",
    family_wallet_description: "Wallet description read",
    family_part_read_the: "Part {} read · the wallet is rebuilt",
    family_part_read_of: "Part {} read · {} of {} keys in hand",
    family_a_public_key: "A public key: it shows the wallet but cannot sign",
    family_a_transaction: "A transaction to sign · kept for later",
    family_is_in_files: "{} is in Files",
    family_seed_read: "Seed {} read",
    family_no_money: "{} · no money",
    family_passphrase_inline: "passphrase",
    family_passphrase_n: "{} · passphrase",
    family_words: "Words · {}",
    family_restored_wallet: "Restored wallet",
    family_its_shares: "its shares",
    family_of_keys_in_hand: "{} of {} keys in hand",
    family_native_segwit_2: "native SegWit",
    family_taproot: "Taproot",
    family_nested_segwit_2: "nested SegWit",
    family_legacy: "legacy",
    family_native_segwit: "native SegWit multisig",
    family_taproot_multisig: "Taproot multisig",
    family_nested_segwit: "nested SegWit multisig",
    family_legacy_multisig: "legacy multisig",
    family_spend_from_n: "Spend from {}",
    family_wallets: "Wallets",
    family_this_page_takes: "This page takes someone spending for the first time through every step. With a wallet or a seed already loaded, Spend opens on a shorter page instead: what is loaded and what each wallet needs next.",
    family_walk_through: "Walk through from the start",
    family_no_wallet_loaded: "no wallet loaded",
    family_signing_2: "Signing {} · {}",
    family_continue_signing: "Continue signing",
    family_wallets_loaded: "Wallets loaded",
    family_none_loaded: "None loaded",
    family_can_spend_of: "Can spend · {} of {}",
    family_here_needed: "{} {} here · {} needed",
    family_key: "key",
    family_keys: "keys",
    family_no_key_here: "No key here",
    family_an_open_vault: "An open vault holds a missing key · Open it to load it",
    family_spend_from_it: "Spend from it",
    family_open_it: "Open it",
    family_keys_with_no: "Keys with no wallet loaded",
    family_restore_its: "Restore its wallet",
    family_spend_as_a_single: "Spend as a single-key wallet",
    family_home: "Home",
    family_spend: "Spend",
    family_six_stops: "Six stops",
    family_desktop_copy: "Desktop copy",
    family_running_from: "Running from the {}",
    family_nothing_in_yet: "Nothing in yet",
    family_1_thing_in: "1 thing in",
    family_things_in: "{} things in",
    family_loaded_in_sparrow: "Loaded in Sparrow",
    family_money_shows: "Money shows",
    family_done_in_sparrow: "Done in Sparrow",
    family_signing: "Signing {}",
    family_not_here_yet: "Not here yet",
    family_nothing_waits: "Nothing waits {}",
    family_1_file: "1 file {}",
    family_files: "{} files {}",
    family_after_the_transaction: "After the transaction",
    family_i_have_the_psbt_2: "I have the PSBT",
    family_show_the_finished: "Show the finished transaction as QR",
    family_less_about_this: "Less about this",
    family_more_about_this: "More about this",
    family_start: "Start",
    family_or_bring_in: "Or bring in what is on paper",
    family_scan_the_qr: "Scan the QR codes you have",
    family_type_the_words: "Type the words you have",
    family_open: "Open {}",
    family_copy_files_in: "Copy files in from {}",
    family_what_came_in: "What came in",
    family_rebuild_the: "Rebuild the wallet",
    family_seed_is_not: "Seed {} is not one of {}'s keys",
    family_add_its_passphrase: "Add its passphrase",
    family_seed_2: "Seed · {}",
    family_seed: "Seed · {} · {}",
    family_wallet_description_3: "Wallet description · {} · {}",
    family_wallet_description_2: "Wallet description · {}",
    family_part_of_keys: "Part {} · {} of {} keys in hand",
    family_part: "Part {}",
    family_key_xpub_n: "Key · {} · xpub",
    family_key_xpub: "Key · xpub",
    family_transaction: "Transaction · kept for later",
    family_anything_else: "Anything else in the envelope?",
    family_type_more_words: "Type more words",
    family_that_s_everything: "That's everything",
    family_these_seeds: "These seeds are one wallet with several keys.",
    family_scan_the_description: "Scan the description",
    family_i_can_t_find: "I can't find one",
    family_without_the: "Without the description Faraday needs all {} keys: their seeds or xpubs",
    family_remove_the_then: "Remove the {}, then type the passphrase",
    family_sealed_here: "Sealed here, {}",
    family_copied_in_from: "Copied in from {}",
    family_unlocks_in: "Unlocks in {}",
    family_passphrase: "Passphrase",
    family_unlocking: "Unlocking",
    family_this_computer: "This computer: {} free",
    family_unlock: "Unlock",
    family_plug_the_in: "Plug the {} in",
    family_faraday_asks: "Faraday asks to lock first when {} goes in",
    family_spend_from: "Spend from",
    family_of_keys_here: "{} · {} of {} keys here",
    family_its_key_is_here: "Its key is here: it can sign",
    family_its_key_is_not: "Its key is not here yet",
    family_of_keys_sign: "{} of {} keys sign · {} here",
    family_i_have_the_psbt_from: "I have the PSBT from Sparrow ready",
    family_scan_it: "Scan it",
    family_i_have_the_psbt: "I have the PSBT from Sparrow ready: Scan it",
    family_show_the_wallet_qr: "Show the wallet QR",
    family_receive_address: "Receive address 0",
    family_it_is_loaded: "It is loaded",
    family_none_of_these: "None of these show money.",
    family_the_words_may: "The words may belong to a wallet with several keys, or need a different passphrase.",
    family_scan_more: "Scan more",
    family_type_a_passphrase: "Type a passphrase",
    family_is_there_a_word: "Is there a word or sentence kept apart from the list?",
    family_yes_type_it: "Yes, type it",
    family_no: "No",
    family_trying_of: "Trying: {}, {} · {} of {}",
    family_with_a_passphrase: "with a passphrase",
    family_no_passphrase: "no passphrase",
    family_show_the_wallet: "Show the wallet as a QR code",
    family_open_this_in: "Open this in the wallet on the online computer",
    family_open_this_one: "Open this one on the online computer too",
    family_receive: "Receive",
    family_change: "Change",
    family_show_more_addresses: "Show more addresses",
    family_no_money_shows: "No money shows",
    family_yes_my_money: "Yes, my money shows",
    family_every_coin_to: "Every coin to {}",
    family_amount_max_in: "Amount: Max, in Sparrow's Send tab",
    family_scan_the_psbt: "Scan the PSBT",
    family_scan: "Scan",
    family_on: "On {}",
    family_copy_it_in_from: "Copy it in from {}",
    family_in_files: "In Files",
    family_signing_this: "Signing this one",
    family_use_this_one: "Use this one",
    family_spends_from: "Spends from {} · {}",
    family_no_wallet_open: "No wallet open here matches this transaction. Change cannot be verified until its wallet is open: check that this is the transaction for this wallet.",
    family_read_it: "Read it",
    family_finish_the_backup_2: "Finish the backup",
    family_load_from_vault: "Load from vault",
    family_signs_here: "Signs here",
    family_not_here: "Not here",
    family_unlock_n: "Unlock {}",
    family_scan_a_key: "Scan a key",
    family_collect_a_signature: "Collect a signature",
    family_remove_the_first: "Remove the {} first",
    family_continue: "Continue",
    family_lock: "Lock",
    family_power_off: "Power off",
    family_start_over: "Start over",
    family_paper: "Paper",
    family_what_you_have: "What you have",
    family_the_faraday: "The Faraday {} and its passphrase, or words written down, and often a sheet describing the wallet.",
    family_open_the_wallet_here: "Open the wallet here",
    family_start_a_computer: "Start a computer from the Faraday {} and open the wallet.",
    family_check_the_balance: "Check the balance",
    family_put_the_wallet: "Put the wallet's description into Sparrow on an online computer.",
    family_make_the_transaction: "Make the transaction in Sparrow. It gives you a PSBT.",
    family_offline: "Offline",
    family_sign_it_here: "Sign it here",
    family_bring_the_psbt: "Bring the PSBT to Faraday, read what it does, and sign it.",
    family_online: "Online",
    family_take_the_signed: "Take the signed transaction back to Sparrow and broadcast it.",
    family_the_words_are: "The words are the money. Never photograph them, and never type them into a phone, a website or a password manager. The same goes for the vault's passphrase.",
    family_not_sure_what: "Not sure what you have?",
    family_another_list: "Another list of words, another QR code, a line starting wsh( or tr(, or a word kept apart from the list.",
    family_its_description: "Its description says how they fit together: a QR code labelled wallet, descriptor, setup or multisig; a long line starting wsh( or tr(; or sheets with lines like 73C5DA0A: xpub…",
    family_a_square_black: "A square black-and-white pattern",
    family_a_qr_code_the: "A QR code: the same information in a form a camera can read. Several numbered 1 of 5 and so on are one thing in parts: scan them one after another. Never scan a QR code that sits beside seed words into anything but Faraday.",
    family_words_numbered: "Words numbered 1 to 12, 18 or 24",
    family_the_seed_phrase: "The seed phrase. Often headed recovery phrase, seed words or backup, or stamped into steel.",
    family_one_long_line: "One long line starting wpkh(, wsh(, tr(, xpub, zpub or tpub",
    family_a_descriptor: "A descriptor or an xpub. It shows the addresses and the balance but cannot spend. Do not post it publicly: it shows the wallet's whole history.",
    family_several_sheets: "Several sheets each holding a few lines, like 73C5DA0A: xpub…",
    family_one_description: "One description split up on purpose, often under a heading such as this sheet carries 2 of 3 wallet descriptor parts. Enough of the sheets, in any order, add up to the whole. A line such as Policy: 2 of 3 says how many signatures the wallet needs.",
    family_a_word_or_sentence: "A word or sentence kept apart from the list, called a passphrase or a 25th word",
    family_if_one_exists: "If one exists, the words alone open a wallet that is real but empty, and the money is in the wallet the words and the passphrase open. This is not the vault's passphrase: the vault's passphrase opens the vault file, and this one changes which wallet the words open.",
    family_a_short_line: "A short line starting bc1, 3 or 1",
    family_one_address: "One address, not a backup.",
    family_a_small_device: "A small device with a screen and buttons",
    family_a_hardware_wallet: "A hardware wallet. It is a way of using the words, not a replacement for them.",
    family_labelled_faraday: "{} labelled Faraday, and a passphrase on a separate card",
    family_the_starts_the: "The {} starts the computer and carries the vault, a locked file named like vault.ofv. The passphrase opens it. The vault holds the wallet and the keys that sign for it.",
    family_how_to_spend: "How to spend bitcoin",
    family_start_from_the: "Start from the {}",
    family_what_do_you: "What do you have?",
    family_open_the_wallet: "Open the wallet",
    family_load_the_wallet: "Load the wallet in Sparrow",
    family_check_the_money_is: "Check the money is there, on the online computer",
    family_write_the_payment_in: "Write the payment in Sparrow",
    family_check_the_money: "Check the money is really there",
    family_write_the_payment: "Write the payment",
    family_bring_the_transaction: "Bring the transaction here",
    family_put_everything: "Put everything away",
    family_read_the_transaction: "Read the transaction",
    family_the_transaction: "The transaction id",
    family_which_way_it: "Which way it spends",
    family_nonces: "Nonces",
    family_add_the_seeds: "Add the seeds that can sign",
    family_who_signs: "Who signs",
    family_sign_it: "Sign it",
    family_the_other_signatures: "The other signatures",
    family_send_it: "Send it",
    family_six_stops_three: "Six stops, three on this computer and three on an online one. Nothing moves until the last.",
    family_this_is_the: "This is the desktop copy of Faraday, which runs on an ordinary computer that has been online. It is for testing. For real money, start a computer from the Faraday {}.",
    family_this_computer_is: "This computer is running Faraday from the {}. It has no network, and it forgets everything when it is switched off.",
    family_scan_or_type: "Scan or type everything in the envelope, one thing at a time, in any order. Faraday works out the wallet from it.",
    family_in_sparrow_file_new: "In Sparrow: File, New Wallet, then scan the wallet QR. Its first address is receive address 0 here.",
    family_in_sparrow_s: "In Sparrow's Send tab: the address, the amount and a fee. Then Create Transaction, Finalize Transaction for Signing and Show QR.",
    family_words_alone: "Words alone do not say which kind of wallet they made. Faraday made the most common kind from them. Open it in the wallet on the online computer, such as Sparrow, and look at its balance. If no money shows, Faraday makes the next kind.",
    family_load_this_same: "Load this same wallet in Sparrow on your online computer to check the balance. The code below is the wallet's description: enough for Sparrow to find every address the wallet uses and add up what is on all of them. It cannot spend. Then check that the first address Sparrow shows is character for character the same as receive address 0 below. If it matches, two separate programs have read the same backup and agreed. If it does not match, stop: something was typed or scanned wrongly.",
    family_this_part_happens: "This part happens on the online computer. In Sparrow, open the wallet's Send tab, enter the address to pay and the amount, and choose a fee. Press Create Transaction, then Finalize Transaction for Signing, then Show QR: an animated code, which is what Faraday reads. Sparrow can also save it as a .psbt file.",
    family_two_ways_scanning: "Two ways. Scanning is the simpler: nothing has to lock. Press Scan and hold Sparrow's QR code up to this computer's camera; an animation is read as it loops. Or save the .psbt onto {} and copy it in. Faraday reads {} only while no vault or key is open, so with one open it locks first: it seals the vault, forgets the keys and starts again. Copy the PSBT in, remove the {}, and unlock with the same passphrase. This tab comes back on this page.",
    family_lock_faraday: "Lock Faraday. The vault is sealed again and the keys are forgotten. If a file you still need waits for the {}, plug {} in and the {} visit writes it. Then switch off, take the {} out, and put it back with the papers.",
    family_this_opens_once: "This opens once the transaction is here.",
    family_read_the_summary: "Read the summary before you sign, every time: what leaves, what comes back, and the fee. Change is the rest coming back to the same wallet, not money leaving. Signing is the one step that cannot be taken back. If this is not what you meant to do, a different amount, an address you do not recognise, or a warning of any kind, do not sign. Declining costs nothing.",
    family_write_down_the: "Write down the first and last few characters of this id. Sparrow shows the same id for the transaction it wrote; after sending, the id is how you follow it.",
    family_every_key_that: "Every key that signs is listed. A key not here yet comes from its own place: Scan a key reads its SeedQR, or its words typed instead, and a vault that holds it unlocks. Faraday works out which key it is. Each is checked against the wallet, so a key from another wallet is refused. A key held by someone else does not have to come here at all: they sign the same PSBT on their own device, and their signed copy is collected after this device signs.",
    family_one_seed_at: "One seed at a time: put its words in, add it, and the count goes up. Order does not matter and the same seed twice does no harm. Each list is checked against the wallet's own keys, so one belonging to a different wallet is refused. A seed kept as a QR code beside the words does not have to be typed: scan it. If that backup has a separate passphrase, type it in the passphrase box before adding.",
    family_signing_adds: "Signing adds the signature of every key loaded here. For a wallet that needs signatures from elsewhere, the next card lists who still has to sign and takes their signed copies; each is checked against its key and this transaction before it counts.",
    family_the_finished: "The finished transaction is what the network accepts. Show the signed PSBT as a QR code; in Sparrow, on the same transaction, press Scan QR and hold it up to the webcam, then Broadcast Transaction. Or carry the finished transaction, which waits in Files, on {}. It is safe to hand to anyone: nobody can change where the money goes without breaking the signatures, and a broken transaction is rejected, not redirected.",
    family_anyone_who_reads: "Anyone who reads the words can spend the money from anywhere in the world, and it cannot be reversed. Where a wallet needs several lists, anyone with enough of them can. Treat every list as if it were the only one. No legitimate bitcoin company will ever ask for them: not a support line, not someone helping you, not an AI.",
    family_nothing_done: "Nothing done here is irreversible until the last step, and that step does not happen in Faraday. Opening the vault, typing words, looking at addresses and reading a transaction move nothing and notify no one. You can stop and come back.",
    family_there_is_no: "There is no deadline; bitcoin does not expire. Rushing is how people get robbed, by scammers and by helpers acting in their own interest. Take your time, and find someone you trust.",
    family_if_any_of_this: "If any of this stops making sense, stop. Put the {} and the paper somewhere safe and get help from someone you would trust with the money itself. Waiting costs nothing.",
    family_shut_the_computer: "Shut the computer down completely, not sleep or restart. Unplug any network cable.",
    family_plug_the_faraday: "Plug the Faraday {} in, switch the computer on and press the boot menu key straight away, repeatedly. It is usually F12, F9, Esc or F2. The first screen often names the key for a second or two.",
    family_pick_the_from: "Pick the {} from the list that appears. If no list appears, the key was wrong or came too late: shut down and try another one.",
    family_if_the_is_missing: "If the {} is missing, or someone else has opened the envelope, do not use it. Do not use {} you are unsure about: the words are the money, and taking longer costs nothing.",
    family_a_vault_unlocks: "A vault unlocks in the time shown on purpose: it makes guessing the passphrase slow. A wrong passphrase opens nothing and says so.",
    family_words_go_in: "Words go in in the order they are written. Faraday checks them against the list of 2048 words and the backup's own checksum, so a mistyped or missing word is caught.",
    family_a_wallet_description: "A wallet description opens the wallet read-only: enough to see every address and to check the balance. The words that sign come later, after the transaction is read.",
    family_split_sheets: "Split sheets go in one at a time, in any order; the wallet opens when enough of them are in.",
    family_in_sparrow_file: "In Sparrow: File, New Wallet, give it a name, then choose the way in that scans a QR code or imports a descriptor, and hold this screen up to the webcam. Menu names change between versions; look for an import that takes a descriptor or a scan. If the code is animated, let it loop once.",
    family_only_load_it: "Only load it into wallet software you chose and trust.",
    family_faraday_can: "Faraday can work out every address the wallet owns, but not which were used. Money is often spread over several addresses, including change addresses from old payments, so the first few can be empty while the money is further down. Sparrow adds them all up.",
    family_on_a_phone_you: "On a phone you can also look an address up on a block explorer such as mempool.space. That moves nothing; it does tell the website you looked.",
    family_paying_into: "Paying into an exchange: the exchange gives a deposit address for bitcoin in your account. Copy it exactly, and compare the first and last few characters after pasting.",
    family_faraday_locks: "Faraday locks after a few minutes without input. That is expected. Unlock again with the same passphrase and this tab opens where you left it.",
    family_if_the_envelope: "If the envelope held a hardware wallet that came with the papers and looks untouched, you can sign the same PSBT on it too and compare. Follow the device's own guide to restore from the same words. Compare what both screens say before approving anything: the amount, the fee and the address. They must match.",
    family_faraday_on_its: "Faraday on its own is enough to do the whole job.",
    family_any_of_these: "Any of these sends it: Sparrow, which opens a transaction from a QR code, a file or pasted text and broadcasts it; a block explorer's broadcast page, such as mempool.space or blockstream.info; or someone you trust.",
    family_then_check_the: "Then check the id. Whatever sends it should report the id Faraday showed. If the id you see anywhere differs, what was sent is not what you signed here: stop and find out why.",
    family_the_first_confirmation: "The first confirmation usually takes about ten minutes but can take hours if the fee was low. Exchanges usually want one to six confirmations before the money shows in the account.",
    family_when_faraday: "When Faraday signs with a vault open, it records the amounts it signed inside that vault, so locking seals a changed copy of the vault to write back. Writing it back to the {} keeps that record; leaving it out changes nothing else.",
    flow_guided: "Guided",
    flow_steps_only: "Steps only",
    flow_change: "Change",
    flow_step_of: "Step {} of {}",
    flow_about_this_step: "About this step",
    flow_home: "Home",
    forms_bip_39_words: "BIP-39 words",
    forms_slip_39_shares: "SLIP-39 shares",
    forms_codex32: "codex32",
    forms_seed_xor_parts: "Seed XOR parts",
    forms_group_of_member: "Group {} of {}, member {} · {} of the group needed · id {}",
    forms_the_secret_id: "The secret · id {}",
    forms_share_id_needed: "Share {} · id {} · {} needed",
    forms_part_words: "Part {} · {} words",
    forms_not_a_slip_39: "Not a SLIP-39 share: {}",
    forms_not_a_codex32: "Not a codex32 string: {}",
    forms_that_string_is: "That string is of another seed: its id differs",
    forms_that_share_is: "That share is in already",
    forms_not_bip_39_words: "Not BIP-39 words: {}",
    forms_every_part_has: "Every part has the same number of words",
    forms_slip_39_key: "SLIP-39 key",
    forms_codex32_key: "codex32 key",
    forms_seed_xor_key: "Seed XOR key",
    forms_the_shares_do: "The shares do not make a key: {}",
    forms_the_strings_do: "The strings do not make a key: {}",
    forms_seed_xor_needs: "Seed XOR needs two parts or more",
    forms_key_added: "Key {} added",
    forms_english: "English",
    forms_spanish: "Spanish",
    forms_french: "French",
    forms_italian: "Italian",
    forms_czech: "Czech",
    forms_portuguese: "Portuguese",
    forms_japanese: "Japanese",
    forms_korean: "Korean",
    forms_chinese_simplified: "Chinese (Simplified)",
    forms_chinese_traditional: "Chinese (Traditional)",
    forms_word_is_not_on: "Word {} is not on the {} list",
    glance_back_up_before: "Back up before you receive",
    glance_waiting_for: "Waiting for its backup",
    glance_no_origin: "no origin",
    glance_can_sign_passphrase: "Can sign · passphrase",
    glance_can_sign_here: "Can sign here",
    glance_in_this_vault_passphrase: "In this vault · passphrase",
    glance_in_this_vault: "In this vault",
    glance_cosigner_xpub: "Cosigner · xpub only",
    glance_load_from_vault: "Load from vault",
    glance_vault: "Vault {}",
    glance_place: "Place {}",
    glance_share_of: "Share {} of {}",
    glance_stick: "{} stick",
    glance_description: "Description",
    glance_key_file: "Key {} file",
    glance_in_the_vault: "{} in the vault",
    glance_copy: "{} · copy",
    glance_of_files: "{} of files",
    glance_watch_only_software: "Watch-only software",
    glance_on_its_own_device: "On its own device",
    glance_its_own_stick: "Its own stick",
    glance_stick_at: "Stick at {}",
    glance_not_here: "Not here",
    glance_checked: "Checked",
    glance_checked_n: "Checked {}",
    glance_can_spend: "Can spend",
    glance_nothing: "Nothing",
    glance_sealed: "Sealed",
    glance_key_2: "Key {} · {}",
    glance_plan_in_locked: "Plan in {}, locked",
    glance_unlock: "Unlock",
    glance_no_backup_plan_2: "No backup plan",
    glance_back_up: "Back up",
    glance_no_backup_plan: "No backup plan in this vault",
    glance_backup: "Backup",
    glance_keys: "Keys",
    glance_wallet: "Wallet",
    glance_key_words: "Key {} words",
    glance_key_seedqr: "Key {} SeedQR",
    glance_passphrase: "Passphrase {}",
    glance_wallet_sheet: "Wallet sheet",
    glance_share: "Share {}",
    glance_vault_copy: "Vault {} · copy",
    glance_key_seed: "Key {} seed",
    glance_key_seed_passphrase: "Key {} seed + passphrase",
    glance_the_description: "The description",
    glance_key: "Key {}",
    glance_sheet_open_the: "Open the vault",
    glance_sheet_open_in: "Open in Wallets",
    glance_sheet_check_addresses: "Check addresses",
    glance_sheet_the_first: "The first three",
    glance_sheet_public_files: "Public files",
    glance_sheet_for_the_software: "For the software",
    glance_sheet_print_the: "Print the wallet sheet again",
    glance_sheet_rename_the: "Rename the wallet",
    glance_sheet_shape: "Shape",
    glance_sheet_checksum: "Checksum",
    glance_sheet_where_2: "Where",
    glance_sheet_label: "Label",
    glance_sheet_last_checked_2: "Last checked",
    glance_sheet_its_seed: "Its seed is at",
    glance_sheet_show_xpub: "Show xpub QR",
    glance_sheet_xpub_file: "Xpub file",
    glance_sheet_explore: "Explore this key",
    glance_sheet_where: "Where it is",
    glance_sheet_words: "Words, by hand",
    glance_sheet_seedqr: "SeedQR, by hand",
    glance_sheet_seed_xor: "Seed XOR or codex32",
    glance_sheet_by_hand: "By hand",
    glance_sheet_into: "Into {}",
    glance_sheet_as_a_file: "As a file",
    glance_sheet_make_another_backup: "Make another backup of this seed",
    glance_sheet_check_a: "Check a copy of this seed",
    glance_sheet_its_passphrase: "Its passphrase",
    glance_sheet_show_its: "Show its xpub",
    glance_sheet_name_its: "Name its holder",
    glance_sheet_stick: "Stick",
    glance_sheet_found: "Found alone",
    glance_sheet_not_yet: "Not yet",
    glance_sheet_open_a: "Open a vault to keep this",
    glance_sheet_mark_checked: "Mark checked",
    glance_sheet_what_goes: "What goes in its envelope",
    glance_sheet_the_envelope: "The envelope list",
    glance_sheet_pdf_for: "PDF, for the stick",
    glance_sheet_add_here: "Add here",
    glance_sheet_mark_lost: "Mark lost",
    glance_sheet_mark_exposed: "Mark exposed",
    glance_sheet_remove_the: "Remove the place",
    glance_sheet_rename: "Rename",
    glance_sheet_save_its: "Save its seeds into it",
    glance_sheet_copy_to: "Copy to another stick",
    glance_sheet_write_the_seed: "Write the seed files again",
    glance_sheet_write_the: "Write the public files again",
    glance_sheet_destroy_its: "Destroy its copies by hand",
    glance_sheet_show_the: "Show the descriptor QR again",
    glance_sheet_public: "Public files for this software",
    glance_sheet_in_it: "In it",
    glance_sheet_key_seed: "Key {} seed",
    glance_sheet_with_its: "With its passphrase",
    glance_sheet_the_description: "The description",
    glance_sheet_save_more: "Save more into it",
    glance_sheet_kept_at: "Kept at",
    glance_sheet_seen_here: "Seen here",
    glance_sheet_last_checked: "Last checked here",
    glance_sheet_marked: "Marked",
    glance_sheet_load_this_key: "Load this key",
    glance_sheet_scan_my: "Scan my copy",
    glance_sheet_type_it_back: "Type it back",
    glance_sheet_check_this: "Check this copy",
    glance_sheet_make_another: "Make another copy",
    glance_sheet_write_it: "Write it again",
    glance_sheet_for_the: "For the stick",
    glance_sheet_check_it: "Check it",
    glance_sheet_by_the: "By the key it makes",
    glance_sheet_print_it: "Print it again",
    glance_sheet_as_a_qr: "As a QR picture",
    glance_sheet_as_text: "As text",
    glance_sheet_another_copy: "Another copy",
    glance_sheet_elsewhere: "Elsewhere",
    glance_sheet_print: "Print the shares again",
    glance_sheet_pdfs_text: "PDFs, text and pictures",
    glance_sheet_keys_left: "Keys left off each share",
    glance_sheet_copy_the: "Copy the vault to another stick",
    glance_sheet_then_a: "Then a place",
    glance_sheet_write: "Write it out again",
    glance_sheet_changed: "Changed since written",
    glance_sheet_show_wallet: "Show wallet QR",
    glance_sheet_the_descriptor: "The descriptor",
    glance_sheet_move: "Move",
    glance_sheet_to_another: "To another place",
    glance_sheet_remove_from: "Remove from the plan",
    glance_sheet_i_destroyed: "I destroyed this copy",
    glance_sheet_lost: "Lost",
    glance_sheet_exposed: "Exposed",
    glance_sheet_move_to: "Move {} to",
    glance_sheet_a_new: "A new place",
    glance_sheet_another: "Another copy of {}",
    glance_sheet_key_words: "Key {} words",
    glance_sheet_key_seedqr: "Key {} SeedQR",
    glance_sheet_share: "Share {}",
    glance_sheet_wallet: "Wallet sheet",
    glance_sheet_stick_n: "{} stick",
    glance_sheet_a_copy: "A copy of the vault",
    glance_sheet_add_at: "Add at {}",
    glance_sheet_destroy: "Destroy what goes off the plan by hand",
    glance_sheet_confirm: "Confirm",
    glance_sheet_someone_can: "Someone can now spend",
    glance_sheet_someone_could: "Someone could spend with a vault's passphrase",
    glance_sheet_move_the: "Move the money",
    glance_sheet_new_wallet: "New wallet",
    glance_sheet_the_wallet: "The wallet can no longer be rebuilt from what is left",
    glance_sheet_back_up: "Back up again now",
    glance_sheet_someone: "Someone can see the balance",
    glance_sheet_nothing_lost: "Nothing lost that is not elsewhere",
    glance_sheet_make_a: "Make a replacement copy",
    glance_sheet_nothing: "Nothing it holds spends or shows the balance",
    glance_sheet_nowhere: "Nowhere",
    glance_sheet_chosen: "Chosen",
    glance_sheet_destroyed: "Destroyed",
    glance_sheet_goes_to: "{} goes to",
    glance_sheet_remove: "Remove {}",
    glance_sheet_key: "Key {} · {}",
    glance_sheet_holder: "Holder",
    glance_sheet_name: "Name",
    glance_sheet_keep: "Keep",
    glance_sheet_type_it: "Type it",
    glance_sheet_passphrase: "Passphrase",
    glance_sheet_matches: "Matches: it makes key {}",
    glance_sheet_does_not: "Does not match",
    glance_sheet_check_2: "Check",
    glance_sheet_passphrase_n: "Passphrase {}",
    glance_sheet_show_it: "Show it",
    glance_sheet_to_copy: "To copy by hand",
    glance_sheet_load_this: "Load this key with its passphrase",
    glance_sheet_check: "Check my copy",
    glance_sheet_your_changes: "Your changes on the chart are replaced",
    glance_sheet_change: "Change the plan",
    glance_sheet_back: "Back",
    glance_sheet_close: "Close",
    gpg_this_computer_s: "This computer's clock is not known",
    gpg_a_key_needs_a_name: "A key needs a name or an email",
    gpg_no_randomness_from: "No randomness from the system yet. Try again",
    gpg_the_key_is_in_the: "The key is in the vault",
    gpg_fingerprint: "Fingerprint {}",
    gpg_no_gpg_key_is_selected: "No GPG key is selected",
    gpg_gpg_public_key: "GPG public key",
    gpg_public_checks_signatures: "Public: checks signatures, signs nothing",
    gpg_gpg_revocation: "GPG revocation certificate",
    gpg_revokes: "Revokes {}",
    gpg_whoever_has_it: "Whoever has it can revoke the key",
    gpg_that_file_is_no: "That file is no longer in Files",
    gpg_gpg_signature: "GPG signature · {}",
    gpg_signs: "Signs {}",
    gpg_by: "By {}",
    gpg_not_a_gpg_file: "Not a GPG file",
    gpg_renewed_waits: "Renewed · {} waits {}",
    gpg_1_signature_waits: "1 signature waits {}",
    gpg_signatures_wait: "{} signatures wait {}",
    gpg_never: "Never",
    guide_this_is_the_wallet: "This is the wallet the transaction spends from. Faraday found it by rebuilding the wallet's address from the key paths the transaction carries, so the match is exact. If the name is not the wallet you meant to spend from, stop here.",
    guide_no_wallet_loaded: "No wallet loaded here matches this transaction. You can still sign it from the key paths it carries, but Faraday cannot tell change from a payment to someone else. Load the wallet first if you have its description.",
    guide_open_the_same: "Open the same wallet in the software that made this transaction and compare these addresses with the ones it shows. If they match, both programs are looking at the same wallet, and the change address is one you control.",
    guide_read_each_line: "Read each line against what you meant to do. Sends is the amount leaving and where it goes. Change is the rest coming back; Verified means Faraday rebuilt that address from the wallet's own keys. The fee goes to the miner. If anything here is not what you expected, stop: nothing has been signed yet.",
    guide_musig2_signs: "MuSig2 signs in two rounds and every key takes part. In the first, each signer adds a nonce: a one-time number its signature will use. The easiest order is for this device to sign last: add the copies on which the others put their nonce, then sign. Or go first, share this device's nonce and carry the PSBT to them; keep this session open until their signatures come back, because locking forgets the nonce.",
    guide_this_wallet_can: "This wallet can be spent more than one way. Each path names the keys it needs and how long it waits: a recovery path opens only after the coins have sat unspent that many blocks. Faraday reads which paths this transaction can take from its own sequence numbers; whether the coins are old enough on the chain is something only the network can say.",
    guide_this_is_the_transaction: "This is the transaction's id, fixed before anyone signs. Signatures do not change it, so you can write it down now and look for it after the transaction is broadcast.",
    guide_any_of_these: "Any {} of these shares sign together, and on the chain the result is one ordinary Taproot signature. Shares sign one after another, each at its own place. The first share to sign chooses who follows: pick {} here. After that the transaction names them, and no other share can join.",
    guide_one_other: "one other",
    guide_others: "{} others",
    guide_this_wallet_has: "This wallet has more than one way to spend. Each key below can sign; which keys are enough depends on the path this transaction takes, and some paths wait for a timelock. The last step says whether the script is satisfied.",
    guide_keys_must_sign: "{} {} keys must sign. Keys marked Signs here are loaded on this device. The others are on other devices or with other people; each of them signs the same PSBT and hands it back.",
    guide_of_these: "of these",
    guide_of_the: "of the",
    guide_signed_each_signature: "Signed. Each signature was checked against its key before it was kept.",
    guide_signing_as_a: "Signing as a share draws a fresh secret nonce for each share that signs, adds this share's part, and puts the next share's nonce in a carry file. The carry file is a secret until that share has signed: keep the {} with you and sign the same day. The last share to sign checks every part before it combines them.",
    guide_round_two_this: "Round two: this device adds its partial signature over everyone's nonces. The others do the same on their copies; when theirs come back, Faraday combines all of them into one signature.",
    guide_this_adds_the: "This adds the signature of every key loaded here. On its own it moves nothing: the transaction needs {} {} before it can be broadcast.",
    guide_signature: "signature",
    guide_signatures: "signatures",
    guide_signing_uses: "Signing uses the key loaded on this device. Faraday checks the signature before keeping it, and nothing leaves the device until {} visit writes it.",
    guide_get_the_other: "Get the other signatures. Carry the PSBT, which waits in Files, on {}, or let each cosigner sign the copy they already have. When a signed copy comes back, copy it in on {} visit and add it here. Each signature is checked against its key, and a copy of any other transaction is refused.",
    guide_there_are_two: "There are two results. The signed PSBT goes back to the wallet that made the transaction, which can broadcast it. The finished transaction can be broadcast as it is, from any node you trust. Either one is enough.",
    guide_keep_the_carry: "Keep the carry file in the vault, or write it to {}. At the next share's place, copy it into Faraday with that share loaded, open it from Files and sign. The share that signs last gets the finished transaction.",
    guide_the_transaction: "The transaction is finished when enough keys have signed. Until then it cannot be broadcast.",
    guide_print_the_blank: "Print the blank template first, anywhere: it holds no secret, only numbered lines for the words and the squares every SeedQR of this length shares. It waits {}: carry it to the desktop app, and print one copy per seed. You write on it by hand in the next step.",
    guide_none_of_this: "None of this wallet's seeds were typed into this session, so there is nothing to copy here. Each seed is backed up on the device that holds it.",
    guide_copy_each_seed: "Copy each seed onto its own template by hand: the words, and if you want a code to scan, the SeedQR square by square. The grey squares are already on the template. Click a row to keep your place, or move it with the arrow keys. Then scan your copy with the camera; a mistake is named by its word.",
    guide_a_printer_is: "{}{} A printer is a computer with memory and often a network, so a seed never goes to one. There is no line for a passphrase: written beside the words it stops being a second factor.",
    guide_paper_comes_first: "Paper comes first. Under the copy, Save into the open vault keeps a further copy sealed under the vault's passphrase. Save as a file makes the words or the SeedQR picture a file {}, unprotected, only after you tick that anyone who copies the {} can spend with it: the least safe of the three. A file never holds the passphrase; the vault keeps it only when saved with its passphrase.",
    guide_this_half_is: "This half is xpubs only. It can spend nothing, but it is the only thing that puts the keys back together into this wallet, so keep copies. Show the descriptor as a QR to load the wallet into watch-only software, or keep the QR as a picture with the wallet's name, keys and checksum under it, to print or to scan later. The files and the backup sheet wait {} from the moment the checklist is made; the sheet goes as a PDF to print. Sparrow imports the .json under Specter Desktop, and the multisig config under Coldcard Multisig.",
    guide_instead_of_one: "Instead of one sheet with every key, give each of the {} signers a share that leaves some keys off. With {} left off each, any {} shares together hold every key, so any {} signers can rebuild the wallet, and one share alone cannot watch it. This is not secret sharing: the keys are public, and the shares only decide who can see the balance. Each share waits {} as a sheet to print, a text file and a picture of its QR; Restore takes the text file or a scan of the picture.",
    guide_one_envelope: "One envelope per place, holding what the plan puts there. Store the envelopes apart. A copy kept in a vault is one more copy, not a replacement for paper.",
    guide_a_signed_message: "A signed message proves that whoever holds the key behind one address agreed to some text. Pick the wallet and the address: usually the one someone asked you to prove you control. Only single-key wallets whose key is loaded here can sign.",
    guide_type_the_message: "Type the message exactly as the other side gave it. A single changed character, a space or a line break, makes a different message and the signature will not check.",
    guide_bip_322_is_the: "BIP-322 is the newer format and the only one for Taproot addresses. BIP-137 is what older wallets and exchanges ask for, and the only one for legacy and nested SegWit addresses. Use whichever the other side asked for.",
    guide_signing_moves: "Signing moves no money and spends nothing. The result is three lines: the address, the signature and the message. Carry it out as a file or show it as a QR code.",
    guide_any_of_shares: "Any {} of {} shares sign. On the chain the wallet is one Taproot key, and a spend looks like any single-key spend. Keep the shares in different places, or the threshold protects nothing. This build deals up to five.",
    guide_choose_loaded: "Choose {} loaded keys of 24 words as the first shares. Their words become shares as they are, so a person who already keeps one of these seeds keeps only that. The deal computes the other {} from them.",
    guide_choose_how_the: "Choose how the wallet locks its coins. One key is simplest: lose the seed and the money is gone, so its backup matters more. A multisig needs several keys to sign, so one lost or stolen key is not the end of it. If another program will use this wallet, pick a kind it supports.",
    guide_of_every_spend: "{} of {}: every spend needs {} signatures. Keep the keys apart, on different devices or with different people, or the quorum protects nothing.",
    guide_fill_each_slot: "Fill each slot. A key loaded here signs on this device. New key makes a fresh seed here; write its words down in the backup step before any money goes in. A cosigner's slot takes the account xpub their device exports, copied in on {} visit; it never brings a secret. If a cosigner's xpub is not here yet, mark the slot for later and carry on; this wallet waits under Nothing yet on the Wallets page. Each key held here can give its xpub as a file or a QR code, so the cosigners can make the same wallet on their side.",
    guide_choose_the_key: "Choose the key: one loaded here, or a new one made here from fresh randomness. A new key exists only in this session until its words are written down in the backup step.",
    guide_load_the_wallet: "Load the wallet into the software you will receive with, by the QR code or the descriptor file, and compare these addresses. They must match before you send anything to it.",
    guide_the_plan_says: "The plan says what keeps this wallet: the seeds on paper, a copy in a vault, the wallet in watch-only software. Each choice opens the backup with its checklist. Go through it before any money goes in.",
    guide_the_kind_of_wallet: "The kind of wallet the backup is. A wallet description, scanned or in Files, says the kind, the quorum and every key, and the seeds are then matched to its keys.",
    guide_how_many_keys: "How many keys the wallet has and how many sign, as written on the backup, often as 2 of 3.",
    guide_the_descriptor: "The descriptor, wallet file, BIP 129 record or split sheets. Without one, the wallet is made from the seeds and the cosigners' xpubs at the kind's standard path.",
    guide_one_slot_per: "One slot per key: its seed with its passphrase, if it has one, or a cosigner's xpub. With the description known, a seed that is not one of its keys is refused, so a wrong word or passphrase is caught here.",
    guide_compare_these: "Compare these addresses with the backup sheet or the wallet software that holds the history. If they match, the wallet is back as it was.",
    guide_the_wallet_is: "The wallet is in this session until you lock.",
    guide_a_slip_39_key: "A SLIP-39 key is a master secret of 128 or 256 bits, never written whole: it is dealt as shares of 20 or 33 words, and any of them up to the number needed restore it. It has no BIP-39 words.",
    guide_write_each_share: "Write each share down on its own paper, in order, and keep the papers apart: in different places or with different people. Any of them up to the number needed are the key; fewer are nothing.",
    guide_answer_from_your_papers: "Answer from your papers, not from memory: each share is asked in turn, every word once in a random order. A wrong answer means that share's paper needs correcting.",
    guide_12_words_carry: "12 words carry 128 bits and 24 words 256. Either is beyond guessing; most wallets and signers read every length here.",
    guide_randomness_you_make: "Randomness you make yourself, with dice, coins or cards, does not depend on this device's software: you can make the same secret again elsewhere from the same rolls. The shares are worked out from it by the device. This device's generator is the operating system's; nothing on the screen can show whether it was faulty or rigged, so a key made from it trusts this device. A mix combines several sources, so the key is as good as the best of them.",
    guide_randomness_you: "Randomness you make yourself, with dice, coins or cards, does not depend on this device's software. In the first group every word is your entries' own bits: each one shows as it comes in, and you can find it in the list by hand. In the second the device hashes your entries into the words, which you can check only by making the same key again elsewhere. This device's generator is the operating system's; nothing on the screen can show whether it was faulty or rigged, so a key made from it trusts this device. A mix combines several sources, so the key is as good as the best of them.",
    guide_roll_one_die_at_a_time: "Roll one die at a time and enter the face that came up. Each roll is one flip, 1 to 3 tails and 4 to 6 heads, so the secret is the one a coin would make from the same flips.",
    guide_roll_one_die_at_a: "Roll one die at a time and enter the face that came up. Each roll is one flip, 1 to 3 tails (0) and 4 to 6 heads (1), so the key is the one a coin would make from the same flips. Every 11 flips are a word's number in the list, counted from 0, and the word shows as its 11th flip comes in.",
    guide_roll_one_die_at: "Roll one die at a time and enter the face that came up. A word takes six rolls. In its first five, 1 is 00, 2 is 01, 3 is 10 and 4 is 11; a 5 or a 6 there stands for nothing and is rolled again. The sixth roll is one bit: 1 to 3 is 0, 4 to 6 is 1. The 11 bits, in order, are the word's number in the list counted from 0: 3 1 4 2 2 5 is 10 00 11 01 01 1, which is 1131, the list's word 1132, \"miracle\". The last word is rolled the same way; the key keeps its first {} bits, and its other {} are the checksum, which the device works out from all the others and writes in their place.",
    guide_flip_a_coin_and_enter: "Flip a coin and enter what came up, once per flip. Catching the coin in the air is fine; choosing the side is not. Heads is 1 and tails 0; every 11 flips are a word's number in the list, counted from 0, and the word shows as its 11th flip comes in.",
    guide_roll_one_die: "Roll one die at a time and press the face that came up. Use a real die on a flat surface, not the same throw twice, and do not pick numbers in your head.",
    guide_flip_a_coin_and: "Flip a coin and press what came up, once per flip. Catching the coin in the air is fine; choosing the side is not.",
    guide_shuffle_a_full: "Shuffle a full deck well, then turn cards over one at a time and enter each: its rank, then its suit. A card cannot come up twice from one deck; past 52, shuffle the deck again and keep going.",
    guide_type_the_hex: "Type the hex digits you made elsewhere. They become the key as they are, so they must already be random.",
    guide_point_the_camera: "Point the camera at something that moves or is lit unevenly and take each picture. The pictures are hashed and never kept.",
    guide_this_device_asks: "This device asks the operating system's generator for 32 fresh bytes when this card opens. The key is those bytes, hashed. Nothing on the screen can show whether they were random.",
    guide_lock_in_adds_the: "Lock in adds the key these words make, with the passphrase if one is typed, and puts it in its slot. A passphrase makes another key from the same words: both the words and the passphrase are then needed to spend, and Back up asks where each is kept. Leave the fields empty for none.",
    guide_lock_in_adds: "Lock in adds the key these words make. Its fingerprint and its key show at once, so you can see the words you entered are the key that was added. Words and Quiz follow, to write the words down and check the copy.",
    guide_these_counts: "These counts catch the mistakes hands make: a die that favours one face, the same result typed over and over, or a sequence counted out instead of rolled. A caution is not a refusal; if one appears and you are not sure why, roll again.",
    guide_write_the_words: "Write the words down in order, on paper, before any money goes to this key, and keep them where only you can reach them. The words are the key: anyone who has them can spend, and without them a lost device loses the money.",
    guide_answer_from_your: "Answer from your paper, not from memory: the quiz checks the copy you will rely on. Each word is asked once, in a random order. A wrong answer means a word on the paper needs correcting; look at the words again, fix the copy, and carry on.",
    held_signs_from_now: "{} signs from now on",
    held_holds_seed: "{} holds seed {}",
    inbox_slip_39_shares_2: "{} SLIP-39 shares",
    inbox_codex32_shares: "{} codex32 shares",
    inbox_loaded: "Loaded",
    inbox_slip_39_seed: "SLIP-39 seed",
    inbox_slip_39_shares: "SLIP-39 shares · {} of {} in Files",
    inbox_codex32_seed: "codex32 seed",
    inbox_codex32_shares_inline: "codex32 shares · {} of {} in Files",
    inbox_opensigner_backup: "{} · OpenSigner backup, sealed",
    inbox_restored_wallet: "Restored wallet",
    inbox_split_sheets_of: "Split sheets · {} of {} keys in hand",
    inbox_split_sheets: "Split sheets · {}",
    inbox_its_shares: "its shares",
    inbox_watch_only: "{} · {} · watch-only",
    inbox_xpub: "Xpub",
    inbox_an_xpub_in_files: "an xpub in Files",
    inbox_remove_the_first: "Remove the {} first",
    inbox_with_passphrase: "{} with passphrase",
    inbox_this_seed_was: "This seed was loaded without its words; a passphrase cannot be added",
    inbox_loaded_with_its: "{} loaded with its wallet",
    inbox_a_seed_in_files: "a seed in Files",
    inbox_is_a_wallet: "{} is a wallet",
    inbox_loaded_n: "{} loaded",
    inbox_this_backup_holds: "This backup holds a note: import it into a vault",
    inbox_not_this_backup: "Not this backup's passphrase",
    inputs_is_in_use: "{} is in use",
    inputs_wrong_code: "{} typed the wrong code. Unplug it to try again.",
    inputs_new_keyboards_ignored: "New keyboards are ignored until Faraday starts again.",
    keygen_seed_xor: "Seed XOR",
    keygen_words_chosen: "Words chosen by the dice",
    keygen_no_secret_yet: "No secret yet",
    keygen_the_shares_could: "The shares could not be made: {}",
    keygen_a_long_run_of: "A long run of one result",
    keygen_far_from_even: "Far from even: one result came up much more than the others",
    keygen_a_long_run_counting: "A long run counting up or down",
    keygen_a_5_or_a_6_is: "A 5 or a 6 is rolled again in a word's first five rolls",
    keygen_a_die_has_six: "A die has six faces",
    keygen_no_more_are: "No more are needed",
    keygen_that_card_is: "That card is already out of this deck",
    keygen_no_picture_from: "No picture from the camera yet",
    keygen_too_little_variation: "Too little variation in that picture",
    keygen_of: "{} of {}",
    keygen_remove_the_first: "Remove the {} first",
    keygen_choose_two_sources: "Choose two sources or more",
    keygen_choose_a_source: "Choose a source",
    keygen_no_randomness: "No randomness from the system yet. Try again",
    keygen_the_two_passphrases: "The two passphrases differ",
    keygen_new_key_n: "New key {}",
    keygen_key_added_back: "Key {} added; back up its {} before you rely on it",
    keygen_shares_inline: "shares",
    keygen_words_inline: "words",
    keygen_check: "Check",
    keygen_words: "Words",
    keygen_make_a_new_wallet: "Make a new wallet",
    keygen_add_a_key: "Add a key",
    keygen_tools: "Tools",
    keygen_back: "Back",
    keygen_new_key: "New key",
    keygen_flips: "Flips",
    keygen_cards: "Cards",
    keygen_pictures: "Pictures",
    keygen_generator: "Generator",
    keygen_entries: "Entries",
    keygen_source_of: "{} · source {} of {}",
    keygen_rolls_of_a_six: "{} rolls of a six-sided die",
    keygen_flips_or_rolls: "{} flips, or rolls of a die read as flips",
    keygen_cards_from_a: "{} cards from a shuffled deck",
    keygen_digits_made: "{} digits made elsewhere",
    keygen_pictures_n: "{} pictures",
    keygen_two_or_more: "Two or more of these, combined",
    keygen_getrandom_2: "{} · getrandom(2)",
    keygen_slip_39_of_shares: "SLIP-39 · {} of {} shares · {} words each",
    keygen_read_from_a: "{} · read from a die",
    keygen_not_chosen: "Not chosen",
    keygen_complete: "Complete",
    keygen_not_locked_in: "Not locked in",
    keygen_locked_in: "{} · locked in",
    keygen_passphrase_set: "{} · passphrase set · locked in",
    keygen_passphrase_not: "{} · passphrase not set · locked in",
    keygen_sources_combined: "{} sources combined",
    keygen_no_cautions: "No cautions",
    keygen_1_caution: "1 caution",
    keygen_cautions: "{} cautions",
    keygen_shares_n: "{} shares",
    keygen_not_yet: "Not yet",
    keygen_length: "Length",
    keygen_randomness: "Randomness",
    keygen_fair_no_cautions: "Fair: no cautions",
    keygen_passphrase_again: "Passphrase again",
    keygen_lock_in: "Lock in",
    keygen_enter_locks: "Enter locks in",
    keygen_passphrase: "Passphrase",
    keygen_set: "Set",
    keygen_not_set: "Not set",
    keygen_first_address: "First address",
    keygen_key: "Key",
    keygen_fewer_shares: "Fewer shares than the threshold say nothing about the seed: with them, every seed stays as likely as every other. No entropy is lost to a holder of too few shares.",
    keygen_each_share_states: "Each share states in the clear its backup's identifier, the shares needed, how many were dealt and its own number: one share tells its holder how many others to look for.",
    keygen_the_threshold: "The threshold of shares rebuilds the whole seed on one device, and that device holds all of it while it signs.",
    keygen_the_split_s: "The split's own randomness comes from the device that deals it, here this device's system generator. A dealer with weak randomness gives the seed away in its shares.",
    keygen_a_share_cannot: "A share cannot be checked on its own against the seed; a wrong or mixed-up share shows only when the threshold is put together, by a 4-byte digest.",
    keygen_shares_restore: "Shares restore only in software that reads SLIP-39, such as Trezor's and OpenSigner's; a wallet that reads only BIP-39 words cannot.",
    keygen_a_share_is_20: "A share is 20 words for a 128-bit seed, the strength of 12 BIP-39 words, or 33 words for 256 bits.",
    keygen_bip_39_words: "BIP-39 words",
    keygen_slip_39_shares: "SLIP-39 shares",
    keygen_shares_needed: "Shares needed",
    keygen_shares_dealt: "Shares dealt",
    keygen_any_of_the_shares: "Any {} of the {} shares restore the key",
    keygen_what_shares: "What shares reveal",
    keygen_words_n: "{} words",
    keygen_a_threshold: "A threshold wallet's keys are 24 words",
    keygen_flip_mode: "{} · Flip mode",
    keygen_rolls_read_as: "{} rolls, read as flips",
    keygen_your_own_entropy: "Your own entropy, words verifiable by hand",
    keygen_your_entropy: "Your entropy, computer generates words",
    keygen_made_by_this: "Made by this device",
    keygen_recommended: "Recommended",
    keygen_rolls_inline: "rolls",
    keygen_flips_inline: "flips",
    keygen_cards_inline: "cards",
    keygen_digits_inline: "digits",
    keygen_pictures_inline: "pictures",
    keygen_of_2: "{} of {} {}",
    keygen_last: "Last: {}",
    keygen_deck_drawn_from: "Deck {} · {} drawn from it",
    keygen_luma_values: "{} luma values in the last picture",
    keygen_camera_on: "Camera on",
    keygen_waiting_for: "Waiting for the camera",
    keygen_no_need_to_write: "No need to write the words down yet: Back up shows them again.",
    keygen_randomness_check: "Randomness check",
    keygen_next_source: "Next source",
    keygen_make_the_words: "Make the words",
    keygen_undo: "Undo",
    keygen_clear: "Clear",
    keygen_type_1_to_6: "Type 1 to 6 · Backspace takes one back · Enter continues",
    keygen_type_h_or_t: "Type H or T · Backspace takes one back · Enter continues",
    keygen_type_the_rank: "Type the rank (A 2–9 T J Q K), then the suit (S H D C)",
    keygen_type_09_and: "Type 0–9 and a–f · Backspace takes one back · Enter continues",
    keygen_space_takes: "Space takes a picture · Enter continues",
    keygen_enter_continues: "Enter continues",
    keygen_coin: "Coin",
    keygen_die: "Die",
    keygen_typed: "Typed",
    keygen_buttons: "Buttons",
    keygen_each_roll_is: "Each roll is one flip: 1, 2, 3 are 0 (tails); 4, 5, 6 are 1 (heads). The key is the one these flips make",
    keygen_heads_is_1_tails: "Heads is 1, tails is 0",
    keygen_h_or_1_for_heads: "H or 1 for heads, T or 0 for tails",
    keygen_faces_1_to_6: "Faces 1 to 6, as rolled",
    keygen_of_11_bits: "{} of 11 bits",
    keygen_from_the_checksum: "From the checksum, once the rest is in",
    keygen_the_last_bits: "The last {} bits of word {} are the checksum",
    keygen_words_so_far: "Words so far",
    keygen_rolls_1_to_5: "Rolls 1 to 5 of a word: 1 = 00, 2 = 01, 3 = 10, 4 = 11 · Roll 6: 1 to 3 = 0, 4 to 6 = 1 · The 11 bits are the word's number in the list, from 0",
    keygen_checksum: "checksum",
    keygen_word: "= {} · word {}",
    keygen_word_of_the: "= {} · word {} of the list",
    keygen_of_rolls: "{} of {} rolls",
    keygen_the_checksum: "The checksum, from all the other bits, once every roll is in",
    keygen_result: "Result",
    keygen_continue: "Continue",
    keygen_start_again: "Start again",
    keygen_rolls: "Rolls",
    keygen_faces: "Faces",
    keygen_suits: "Suits",
    keygen_digits: "Digits",
    keygen_sha_256: "SHA-256 {}…",
    keygen_share: "Share",
    keygen_of_needed: "{} of {} · {} needed",
    keygen_previous_share: "Previous share",
    keygen_next_share: "Next share",
    keygen_i_wrote_them_all: "I wrote them all down",
    keygen_not_made_yet: "Not made yet",
    keygen_source: "Source",
    keygen_hide_words: "Hide words",
    keygen_show_words: "Show words",
    keygen_i_wrote_them: "I wrote them down",
    keygen_show_the_words_first: "Show the words first",
    keygen_fingerprint: "Fingerprint",
    keygen_all_shares_right: "All {} shares right",
    keygen_all_words_right: "All {} words right",
    keygen_skipped_the: "Skipped: the copy is not checked",
    keygen_quiz: "Quiz",
    keygen_done: "Done",
    keygen_use_this_key: "Use this key",
    keygen_add_key: "Add key",
    keygen_share_of_of: "Share {} of {} · {} of {} right",
    keygen_of_right: "{} of {} right",
    keygen_which_is_word: "Which is word {}?",
    keygen_not_that_one: "Not that one. Check your copy against the words",
    keygen_ask_it_again: "Ask it again",
    keygen_show_the_words: "Show the words",
    keygen_without_the: "Without the quiz, a word copied wrongly is found only when the backup is needed",
    keygen_skip_and_add: "Skip and add the key",
    keygen_skip_the_quiz: "Skip the quiz",
    keygen_type_1_to_4_to: "Type 1 to 4 to pick · Enter adds the key once it passes",
    keygen_type_1_to_4: "Type 1 to 4 to pick · Enter is Done once it passes",
    name_faraday_hub_app: "Faraday Starter",
    name_faraday_hub_app_old: "Faraday hub app",
    name_faraday_hub_app_old2: "Faraday Hub",
    name_faraday_desktop: "Faraday Desktop",
    name_faraday_desktop_old: "Faraday desktop app",
    version_label_local: "{} (local build)",
    stick_kind_psbt: "PSBT",
    stick_kind_text: "Text",
    stick_kind_transaction: "Transaction",
    stick_kind_sheet_for: "Sheet for printing",
    stick_kind_vault: "Vault",
    stick_kind_efi_image: "EFI image",
    stick_kind_threshold: "Threshold spend, part-signed",
    stick_kind_opensigner: "OpenSigner backup",
    stick_kind_qr_codes: "QR codes in a PNG",
    stick_kind_file: "File",
    stick_keys_keys_load: "Keys load only with no {} attached",
    of_ur_crypto_psbt_one: "ur:crypto-psbt · one code",
    of_ur_crypto_psbt_frames: "ur:crypto-psbt · {} frames, shown in turn",
    of_text_one_code: "Text · one code",
    of_ur_crypto_account: "ur:crypto-account · one code",
    of_key_one_code: "Key · one code",
    of_file_ur_bytes: "File · ur:bytes",
    parts_frames_text_one: "Text · one code",
    parts_frames_numbered: "Numbered parts · {} frames, shown in turn",
    bbqr_frames_too_large: "Too large for BBQr",
    bbqr_frames_bbqr_one: "BBQr {} · one code",
    bbqr_frames_bbqr_frames: "BBQr {} · {} frames, shown in turn",
    key_label_key_2: "Key {} · {} · {}",
    key_label_key: "Key {} · {}",
    ur_bytes_frames_one: "{} · one code",
    ur_bytes_frames_frames: "{} · {} frames, shown in turn",
    part_note_of_missing: "{}: {} of {} · missing {}{}",
    secret_text_those_are: "Those are a seed's words. Seeds go in through Add a key, not Files",
    secret_text_that_is: "That is a private key. Faraday takes keys as words or a SeedQR, in Add a key",
    what_instead_of_a_seed_that_is_a: "That is a transaction, not a seed. Scan it from Files",
    what_instead_of_a_seed_that_is: "That is a code for Files, not a seed",
    what_instead_of_a_seed_that: "That is a public wallet or key, not a seed: it cannot sign",
    app_not_a_key_of_this: "Not a key of this wallet: check the words or the passphrase",
    heading_written_to_at: "Written to {} at {}",
    heading_written_to: "Written to {}",
    storage_event_copied_a: "Copied {}: a key's words, loaded when the {} is removed",
    storage_event_copied: "Copied {}",
    storage_event_wrote_read: "Wrote {}, read back and matched",
    storage_event_wrote: "Wrote {} as {}, read back and matched",
    storage_event_it_stays: "It stays in Files",
    storage_event_it_still: "It still waits {}",
    storage_event_not_written: "{} not written: {}. {}",
    storage_event_saved: "Saved {}",
    storage_event_too_fine: "Too fine for this camera: show it as an animated code with smaller parts, bring it closer, or carry it as a file",
    storage_event_not_printed: "Not printed: {}",
    sticks_changed_removed_2: "{} removed",
    sticks_changed_removed: "{} removed · {} {} loaded",
    sticks_changed_key: "key",
    sticks_changed_keys: "keys",
    toast_out_waits: "{} waits {}",
    pull_what_load_the: "load the wallet",
    pull_what_open_the: "open the backup",
    pull_what_unlock: "unlock",
    pull_what_make_a_key: "make a key",
    pull_what_add_a_key: "add a key",
    pull_what_scan: "scan",
    visit_then_then: "Then: {}",
    visit_then_then_unlock: "Then: Unlock",
    visit_then_then_home: "Then: Home",
    start_spend_the_carry: "The carry file holds no PSBT",
    start_spend_not_a_psbt: "Not a PSBT",
    session_stage_bring: "Bring in",
    session_stage_write: "Write out",
    session_stage_work: "Work",
    session_stage_open: "Open",
    act_and_loaded: "{} {} and {} {} loaded",
    act_wallet: "wallet",
    act_wallets: "wallets",
    act_seed: "seed",
    act_seeds: "seeds",
    act_remove_the_first: "Remove the {} first",
    act_finished_from: "Finished from {}",
    act_transaction: "Transaction",
    act_plug_in_the_visit: "Plug in {}: the visit writes what waits for it",
    act_bip_129_covers: "BIP 129 covers wsh and sh(wsh) multisig keys",
    act_carrying_on_with: "Carrying on with the parts read before",
    act_of_keys_in_hand: "{} of {} keys in hand",
    act_restored_wallet: "Restored wallet",
    act_its_shares: "its shares",
    act_restored_from_its: "{} restored from its shares",
    act_not_every_key_is: "Not every key is in hand yet",
    act_signed_psbt: "Signed PSBT",
    act_psbt_for_the_next: "PSBT for the next signer",
    act_the_secret_nonce: "The secret nonce the next share signs this transaction with",
    act_whoever_has_it: "Whoever has it and a share's signature can work out that share",
    backup_kept_not_checked: "Not checked: no vault unlocked",
    backup_kept_wallet_in: "Wallet in {}",
    backup_kept_wallet: "Wallet not in a vault",
    backup_kept_in: "In {}{}",
    backup_with_its_passphrase: "with its passphrase",
    backup_kept_not_in: "Not in a vault",
    backup_kept_paper_copy: "Paper copy checked",
    backup_kept_paper_not: "Paper: not checked",
    backup_kept_file_unprotected: "File {}, unprotected",
    backup_kept_not_here: "Not here: backed up on its own device",
    backup_kept_share: "Share {}",
    backup_seed_list_of_share: "Share {}",
    place_name_place: "Place {}",
    plan_store_place: "Place {}",
    plan_store_of_files: "{} of files",
    plan_store_watch_only: "Watch-only software",
    plan_store_on_its_own: "On its own device",
    plan_store_the_plan: "The plan is in {}",
    public_pictures_part: "Part {} of {}",
    public_pictures_key: "Key",
    public_pictures_keys: "Keys",
    public_pictures_descriptor: "Descriptor checksum {}",
    public_pictures_public_watch: "Public: watch only, spends nothing",
    public_pictures_share: "Share {} of {} · any {} rebuild the wallet",
    public_pictures_holds: "Holds {}",
    public_pictures_leaves: "Leaves off {}",
    public_pictures_not: "Not a wallet on its own",
    public_pictures_public: "Public: spends nothing",
    public_out_files: "{} files",
    backup_files_make_files: "{} files",
    public_made_no_pdf: "No PDF: {}",
    wallet_key_that_wallet: "That wallet is no longer loaded",
    wallet_key_this_wallet_s: "This wallet's keys are not account keys",
    wallet_key_this_wallet: "This wallet has no such key",
    wallet_key_that_key_names: "That key names no master",
    wallet_key_that_key: "That key is not held here",
    wallet_key_record_bip: "BIP 129 covers wsh and sh(wsh) multisig keys",
    create_key_text_nothing: "nothing being created",
    create_key_text_that: "That key is not held here",
    create_key_text_a_chosen: "a chosen key is no longer loaded",
    create_keys_nothing: "nothing being created",
    create_keys_key_has: "Key {} has no key yet",
    create_keys_key_waits: "Key {} waits for its cosigner's xpub file. Copy it in on {} visit, then choose it for Key {} under Keys",
    create_keys_a_chosen: "a chosen key is no longer loaded",
    create_keys_the_same: "The same key is in two slots",
    create_shares_nothing: "nothing being created",
    create_shares_share_has: "Share {} has no key yet",
    create_shares_a_chosen: "a chosen key is no longer loaded",
    create_shares_share: "Share {} needs a 24-word key",
    create_shares_the_same: "The same key is in two slots",
    create_deal_a_chosen: "a chosen key is not a share",
    create_deal_new_wallet: "New wallet {}",
    create_deal_dealt_here: "Dealt here",
    create_deal_share_of: "Share {} of {}",
    restore_refusal_every: "Every slot is filled",
    restore_described_make_still: "{} {} still empty",
    restore_described_make_slot: "slot",
    restore_described_make_slots: "slots",
    restore_from_vault_wallet: "Wallet",
    restore_from_vault_from: "{} {} from {}",
    restore_from_vault_key: "key",
    restore_from_vault_keys: "keys",
    create_pass_lock_the: "The two passphrases differ",
    create_pass_lock_passphrase: "{} · passphrase",
    create_make_new_wallet: "New wallet {}",
    create_make_made_here: "Made here",
    make_pdf_pdf: "{}.pdf",
    make_pdf_not_printed: "Not printed: {}",
    scanned_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    scanned_not_a_psbt: "Not a PSBT, a descriptor, an xpub or a signed message",
    scanned_is_a_cosigner: "{} is a cosigner's key",
    scanned_is_key: "{} is Key {}",
    scanned_is_in_files: "{} is in Files",
    copy_scanned_not_a: "Not a SeedQR",
    read_code_that_is_a: "That is a SeedQR. Seeds go in through Add a key, not Files",
    read_code_that_may: "That may be a CompactSeedQR. Seeds go in through Add a key, not Files",
    read_code_ur_is_not: "ur:{} is not a kind this build reads",
    read_code_ur_part_of: "UR: part {} of {}",
    read_code_not_a_part: "Not a part of this code: {}",
    address_note_that_address_fails: "That address fails its checksum: a character in it is wrong",
    address_note_that_address_is_not: "That address is not on {}",
    address_note_change: "change",
    address_note_receive: "receive",
    address_note_that_address_is: "That address is {}'s {} address {}",
    address_note_that_is: "That is an address. Load a wallet to check whether it is yours",
    address_note_that_address: "That address is not among the first 100 of any loaded wallet",
    open_qr_cannot_make: "Cannot make a code: {}",
    code_png_cannot_make: "Cannot make a code: {}",
    code_png_is_more_than: "{} is more than one code",
    code_view_threshold: "{} · threshold record",
    code_view_descriptor: "Descriptor checksum {}",
    code_view_wallet_descriptor: "{} · wallet descriptor",
    code_view_this_wallet_has: "This wallet has no such file",
    code_view_multisig: "{} · multisig config",
    code_view_bsms_descriptor: "{} · BSMS descriptor record",
    code_view_public_watch: "Public: watch only, spends nothing",
    code_view_bip_129_covers: "BIP 129 covers wsh and sh(wsh) multisig keys",
    code_view_xpub_2: "Xpub {} · {}",
    code_view_that_wallet: "That wallet is no longer loaded",
    code_view_this_wallet_s: "This wallet's keys are not account keys",
    code_view_this_wallet: "This wallet has no such key",
    code_view_no_origin: "no origin",
    code_view_key: "Key {} · {}",
    code_view_xpub: "Xpub {}",
    code_view_bip_129_key: "BIP 129 key record, signed by the key",
    code_view_public_spends: "Public: spends nothing",
    code_view_key_bsms: "Key {} · BSMS record",
    code_view_no_message: "No message is signed",
    code_view_the_message: "The message is not signed yet",
    code_view_signed_by: "Signed by {}",
    code_view_public_proves: "Public: proves who signed, spends nothing",
    code_view_signed_message: "Signed message",
    wallet_label_key: "Key",
    wallet_label_keys: "Keys",
    entry_add_key_added: "Key {} added",
    load_key_key_added: "Key {} added",
    qr_read_copied_no_qr: "Copied {}: no QR code read",
    qr_read_a_seedqr_copied: "{}: a SeedQR, copied in as {}; its key loads when the {} is removed",
    qr_read_is_in_files: "{} is in Files",
    qr_read_read_into_files: "{} read into Files as {}",
    qr_read_read: "read",
    visit_inbox_a_png_with: "A PNG with no QR code read",
    visit_inbox_a_png_of: "A PNG of {}",
    visit_inbox_a_file: "A file Faraday does not read",
    visit_inbox_whoever_has_it_holds: "Whoever has it holds the key: spends its coins, or signs as it",
    visit_inbox_whoever_has_enough: "Whoever has enough parts holds the key",
    visit_inbox_whoever_has_it: "Whoever has it reads every password and code in it",
    visit_inbox_whoever_has: "Whoever has it and a share's signature can work out that share",
    visit_inbox_whoever: "Whoever has it reads it",
    sign_here_waits_for: "{} waits for its backup: finish the backup first",
    sign_here_no_randomness: "No randomness from the system yet. Try again",
    sign_here_this_transaction: "This transaction was signed here before with other amounts for its inputs. It is refused.",
    sign_here_choose_other: "Choose {} other {} under Signers",
    sign_here_share: "share",
    sign_here_shares: "shares",
    sign_here_this_device: "This device's nonce is on the PSBT: carry it to the other signers",
    collect_has_nothing: "{} has nothing this copy lacks",
    collect_no_randomness: "No randomness from the system yet. Try again",
    primary_finish_step: "Finish step {} first",
    event_no_camera_or: "No camera, or the camera was refused",
    lightning_this_key: "This key has no BIP-39 seed",
    lightning_an_aezeed: "An aezeed is 24 words; {} are typed",
    lightning_word_is_not: "Word {} is not on the list",
    lightning_not_an_aezeed: "Not an aezeed for this passphrase: {}",
    lightning_no_vault: "No vault is open",
    lightning_lightning: "Lightning node key\nnode id {}\nprivate key {}",
    lightning_the_node: "The node key is in the vault",
    lightning_a_lightning: "A Lightning node's private key",
    lightning_whoever_has: "Whoever has it is the node: its channels and their money",
    lightning_a_lightning_node: "A Lightning node's key, made the way its software makes it: ldk-node from a BIP-39 key, LND from its own 24-word cipher seed. Its public key is the node id, safe to share; its private key is the node, and goes into a vault.",
    lightning_wallets: "Wallets",
    lightning_lightning_node: "Lightning node key",
    lightning_ldk_node: "ldk-node · {}",
    lightning_lnd_type: "LND · type an aezeed",
    lightning_the_24_words: "The 24 words",
    lightning_of_24: "{} of 24",
    lightning_passphrase: "Passphrase",
    lightning_aezeed_lnd: "aezeed (LND's default)",
    lightning_read_it: "Read it",
    lightning_node_id: "Node id",
    lightning_made: "Made",
    lightning_private_key: "Private key",
    lightning_hide: "Hide",
    lightning_show: "Show",
    lightning_save_into: "Save into {}",
    lightning_out_unprotected: "Out unprotected…",
    medium_stick_inline: "stick",
    medium_sticks_inline: "sticks",
    medium_a_stick_inline: "a stick",
    medium_an_sd_card_inline: "an SD card",
    medium_stick: "Stick",
    medium_sticks: "Sticks",
    medium_sd_cards: "SD cards",
    medium_a_stick: "A stick",
    medium_usb_stick: "USB stick",
    medium_sd_card: "SD card",
    medium_a_usb_stick: "A USB stick",
    medium_an_sd_card: "An SD card",
    medium_from_the_stick: "From the stick",
    medium_from_the_sd_card: "From the SD card",
    medium_for_the_stick: "For the stick",
    medium_for_the_sd_card: "For the SD card",
    medium_from_the_stick_inline: "from the stick",
    medium_from_the_sd: "from the SD card",
    medium_on_the_stick: "on the stick",
    medium_on_the_sd_card: "on the SD card",
    medium_for_the_stick_inline: "for the stick",
    medium_for_the_sd_card_inline: "for the SD card",
    medium_stick_visit: "Stick visit",
    medium_sd_card_visit: "SD card visit",
    medium_upgrade_a_faraday_stick: "Upgrade a Faraday stick",
    medium_upgrade_a_faraday: "Upgrade a Faraday SD card",
    medium_boot_stick: "Boot stick",
    medium_boot_sd_card: "Boot SD card",
    paper_this_key_has_no: "This key has no words to split",
    paper_this_key_has: "This key has no words to make a seed from",
    paper_no_randomness: "No randomness from the system yet. Try again",
    paper_codex32: "codex32: {}",
    pdf_not_a_faraday_sheet: "not a Faraday sheet",
    pdf_a_sheet_of_kind: "a sheet of kind {} is not one this build prints",
    pdf_wallet_backup: "{} · wallet backup",
    pdf_xpubs_only_this: "Xpubs only. This sheet spends nothing; with the seeds it rebuilds the wallet.",
    pdf_descriptor: "Descriptor",
    pdf_xpubs: "Xpubs",
    pdf_first_addresses: "First addresses",
    pdf_written_on: "Written on",
    pdf_kept_at: "Kept at",
    pdf_goes_with_seed: "Goes with seed of key",
    pdf_made_by_faraday_check: "Made by Faraday. Check the first address against your wallet software before relying on this sheet.",
    pdf_envelopes: "{} · envelopes",
    pdf_wallet: "Wallet",
    pdf_split_backup_sheet: "{} · split backup, sheet {}",
    pdf_part_of_the_wallet: "Part of the wallet's descriptor: xpubs only, it spends nothing. Sheets enough to hold every xpub ({} wallet) rebuild it.",
    pdf_made_by_faraday: "Made by Faraday. Scan the code into Faraday, or type the lines, one sheet at a time.",
    pdf_the_template_is: "the template is for 12 or 24 words",
    pdf_seed_sheet_words: "Seed sheet · {} words",
    pdf_written_by_hand: "Written by hand. Never photographed, scanned or printed once filled in.",
    pdf_network: "Network: {}",
    pdf_fingerprint: "Fingerprint",
    pdf_first_address: "First address",
    pdf_there_is_no_line: "There is no line for a passphrase. Written beside the words, it would stop being a second factor.",
    pdf_places: "Places",
    pdf_place: "Place",
    pdf_holds: "holds",
    plan_paper_only: "Paper only",
    plan_paper_and_vault: "Paper and vault",
    plan_paper_vault_and: "Paper, vault and watch-only software",
    plan_vault_2: "Vault {} · {}",
    plan_seed_words: "Seed {}: words",
    plan_seed_seedqr: "Seed {}: SeedQR",
    plan_passphrase_of: "Passphrase of {}",
    plan_wallet_sheet: "Wallet sheet",
    plan_share_of: "Share {} of {}",
    plan_with_vault: "{} with Vault {}",
    plan_seed: "Seed {}",
    plan_seed_with_its: "Seed {} with its passphrase",
    plan_wallet_description: "Wallet description",
    plan_seed_file: "Seed {}: file",
    plan_yes_with_the_vault: "Yes, with the vault's passphrase",
    plan_yes: "Yes",
    plan_only_with_the: "Only with the vault's passphrase",
    plan_no: "No",
    plan_any_one_place: "Any one place lost: the rest rebuild the wallet",
    plan_one_place_found_can: "One place found: can spend",
    plan_one_place_found: "One place found: sees the balance",
    plan_can_spend: "Can spend",
    plan_sees_the_balance_can: "Sees the balance · can spend with the vault's passphrase",
    plan_can_spend_with: "Can spend with the vault's passphrase",
    plan_sees_the_balance_2: "Sees the balance",
    plan_sees_the_balance: "Sees the balance with the vault's passphrase",
    plan_nothing: "Nothing",
    plan_at_no_place: "at no place",
    plan_vault_whose_is: "Vault {}, whose {} is {}",
    plan_in: "in {}",
    plan_on: "on {}",
    plan_at: "at {}",
    plan_with: "with {}",
    plan_seed_is_kept_only: "Seed {} is kept only {}",
    plan_the_passphrase_of: "The passphrase of {} is kept only with {}",
    plan_without_the_rest: "Without {}, the rest do not rebuild the wallet",
    plan_vault: "Vault {}",
    plan_the_of_files: "the {} of files",
    plan_the_software: "the software",
    plan_the_devices: "the devices",
    plan_seed_is_kept_nowhere: "Seed {} is kept nowhere",
    plan_the_passphrase: "The passphrase of {} is kept nowhere",
    plan_tag_secret: "secret",
    plan_tag_sealed: "sealed",
    plan_tag_public: "public",
    plan_mark_lost: "lost",
    plan_mark_exposed: "exposed",
    restore_not_a_multisig: "not a multisig config",
    restore_these_shares: "these shares are of different wallets",
    restore_no_shares: "no shares",
    scan_key_is_not_one: "{} is not one of {}'s keys",
    scan_key_key: "Key {} · {}",
    scan_key_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    scan_key_key_already: "Key {} · already here",
    scan_key_key_added_more: "Key {} added · {} more",
    scan_key_key_added: "Key {} added · Can spend",
    held_frame_home: "Home",
    notices_remove_the: "Remove the {} to keep working",
    sidebar_home: "Home",
    sidebar_vaults: "Vaults",
    sidebar_tools: "Tools",
    sidebar_transfer: "Transfer",
    sidebar_settings: "Settings",
    sidebar_open_n: "{} open",
    sidebar_learn: "Learn",
    sidebar_bring_in: "Bring in",
    sidebar_open: "Open",
    sidebar_work: "Work",
    sidebar_write_out: "Write out",
    sidebar_passphrase: "passphrase",
    sidebar_more: "+ {} more",
    sidebar_keys: "Keys",
    sidebar_of: "{} of {}",
    sidebar_single_key: "single key",
    sidebar_wallets: "Wallets",
    sidebar_files: "Files",
    sidebar_import_waiting: "import waiting",
    sidebar_none: "none",
    sidebar_attached: "{} attached",
    sidebar_lock: "Lock",
    home_lead_make_a_new: "Make a new wallet",
    home_lead_single_key: "Single key or multisig",
    home_job_carry_on_the: "Carry on the spend",
    home_job_signatures: "{} · Signatures {} of {}",
    home_job_continue_signing: "Continue signing {}",
    home_job_of_signatures: "{} of {} signatures",
    home_job_sign: "Sign {}",
    home_job_from_files: "From Files",
    home_job_remove_the: "Remove the {} to start the import",
    home_job_read_from: "Read from {}",
    home_job_import_from: "Import from {}",
    home_job_written_to: "Written to {}",
    home_job_in_files: "{} · in Files",
    home_job_unlock: "Unlock {}",
    home_job_pull_the_to: "Pull the {} to unlock",
    home_job_write_to: "Write {} to {}",
    home_ways_make_a_new: "Make a new wallet",
    home_ways_single_key: "Single key or multisig",
    home_ways_restore_an: "Restore an existing wallet",
    home_ways_from_its: "From its backups",
    home_ways_spend_from: "Spend from a backup, step by step",
    home_ways_a_stick_words: "A stick, words, or paper",
    home_ways_create_a: "Create a vault",
    home_ways_none_yet: "None yet",
    home_ways_vaults: "Vaults",
    home_ways_locked: "{} locked",
    home_ways_open: "{} open",
    home_ways_a_stick_attached: "A stick attached",
    home_ways_backups: "Backups",
    home_ways_with_a_plan: "{} with a plan",
    home_ways_not_backed: "{} not backed up",
    home_home: "Home",
    home_start: "Start",
    home_scan: "Scan",
    start_wallets: "Wallets",
    start_no_wallet_is: "No wallet is loaded. Create one with new keys or keys you load here, or restore one from its backups: {}, a vault or paper.",
    start_start_from_what: "Start from what you have in front of you. Every way in ends at the same place: the wallet, with each of its keys marked as here, elsewhere or missing, and the one thing still to do.",
    start_your_wallets: "Your wallets",
    start_keys_without: "Keys without a wallet",
    start_make_a_new_wallet: "Make a new wallet",
    start_single_key_or: "Single key or multisig",
    start_restore_an_existing: "Restore an existing wallet",
    start_from_its_backups: "From its backups",
    start_spend_from_a: "Spend from a backup, step by step",
    start_a_stick_words: "A stick, words, or paper",
    start_add_a_key: "Add a key",
    start_keys_first_wallets: "Keys first, wallets later",
    start_is_in_files_locked: "{} is in Files, locked",
    start_unlock: "Unlock",
    nonce_check_nonce_check_valid: "Nonce check · {} {} valid · {} recomputed here",
    nonce_check_signature: "signature",
    nonce_check_signatures: "signatures",
    nonce_check_nonce_check: "Nonce check · a signature is wrong",
    nonce_check_one_nonce: "{}: one nonce on inputs {} · this gives the key away",
    nonce_check_input: "Input {}",
    nonce_check_invalid: "Invalid: not this key's over this transaction",
    nonce_check_not_checked: "Not checked · {}",
    nonce_check_valid_nonce: "Valid · nonce recomputed, {}",
    nonce_check_valid_not: "Valid · not the nonce this key makes: do not broadcast",
    nonce_check_valid_signed: "Valid · signed elsewhere, its nonce needs that device's key",
    nonce_check_more: "{} more {}",
    nonce_check_none_wrong: "· none wrong",
    nonce_check_one_is: "· one is wrong",
    inbox_panel_wallets: "Wallets",
    inbox_panel_of_here: "{} · {} of {} {} here{}",
    inbox_panel_files_say_it: "· {} files say it",
    inbox_panel_from_split_sheets: "· from split sheets",
    inbox_panel_key: "key",
    inbox_panel_keys: "keys",
    inbox_panel_ready_to: "Ready to sign once loaded",
    inbox_panel_more_loaded: "+ {} more, loaded with these",
    inbox_panel_nothing: "Nothing chosen",
    inbox_panel_load_the_wallet: "Load the wallet with its seeds",
    inbox_panel_load_the: "Load the wallet",
    inbox_panel_load_wallets_with: "Load {} wallets with their seeds",
    inbox_panel_load_wallets: "Load {} wallets",
    inbox_panel_keys_load: "Keys load once the {} is out",
    inbox_panel_keys_with: "Keys with no wallet",
    inbox_panel_make_it_a: "Make it a wallet",
    inbox_panel_more: "+ {} more",
    inbox_panel_xpubs_with: "Xpubs with no wallet",
    inbox_panel_no_origin: "no origin",
    inbox_panel_make_it: "Make it a watch-only wallet",
    inbox_panel_not_complete: "Not complete or sealed",
    inbox_panel_open_with: "Open with its passphrase",
    potential_open: "Open {}",
    potential_a_watch_only_wallet: "A watch-only wallet from {}",
    potential_a_watch_only: "A watch-only wallet from an xpub",
    potential_a_wallet: "A wallet from {}",
    potential_from: "From {}",
    potential_the_backup: "The backup's passphrase",
    potential_slip_39_passphrase: "SLIP-39 passphrase · none if left empty",
    potential_bip_39_passphrase: "BIP-39 passphrase · none if left empty",
    potential_as_its_path: "{} · as its path says",
    potential_fingerprint: "Fingerprint",
    potential_a_key_of: "A key of {}: it loads with that wallet",
    potential_single_key: "Single-key wallet",
    potential_cancel: "Cancel",
    potential_open_it: "Open it",
    potential_load: "Load {}",
    potential_make_the: "Make the wallet",
    catalog_tools: "Tools",
    catalog_find_a_tool: "Find a tool: a name, a BIP number",
    catalog_no_tool_matches: "No tool matches",
    decode_back_sign_a: "Sign a transaction",
    decode_back_spend: "Spend",
    decode_back_files: "Files",
    decode_back_wallets: "Wallets",
    decode_back_tools: "Tools",
    decode_back_back: "Back",
    decode_decode_a_transaction: "Decode a transaction",
    decode_sat_sat_vb: "{} sat · {} sat/vB",
    decode_not_stated_the: "Not stated: the amounts going in are not here",
    decode_transaction: "Transaction id",
    decode_witness_id: "Witness id",
    decode_version: "Version",
    decode_locktime: "Locktime",
    decode_size: "Size",
    decode_bytes_vb_wu: "{} bytes · {} vB · {} WU",
    decode_fee: "Fee",
    decode_inputs: "Inputs · {}",
    decode_no_witness: "No witness",
    decode_witness_bytes: "Witness · {} {} · {} bytes",
    decode_item: "item",
    decode_items: "items",
    decode_sequence: "Sequence {} · {}{}",
    decode_script_sig_bytes: "· scriptSig {} bytes",
    decode_outputs: "Outputs · {}",
    decode_change_back: "Change · back to {} · 1/{}",
    decode_back_to_receive: "Back to {} · receive 0/{}",
    decode_pays_out_of: "Pays out of the loaded wallets",
    decode_hide_the_hex: "Hide the hex",
    decode_show_the_hex: "Show the hex",
    decode_show_as_qr: "Show as QR",
    missing_keys_line_is: "{} is locked · it may hold them",
    missing_keys_line_not: "Not in {} · perhaps another vault, a paper backup or a SeedQR",
    missing_keys_line_a: "A vault on {}, a paper backup or a SeedQR may hold them",
    missing_keys_line_unlock: "Unlock",
    wallet_rows_none_loaded: "None loaded",
    wallet_rows_can_spend: "Can spend · {} of {}",
    wallet_rows_of_here: "{} of {} here",
    wallet_rows_watch_only: "Watch only",
    wallet_rows_more: "+ {} more",
    app_make_a_wallet_from: "Make a wallet from this key",
    app_wallet: "Wallet",
    app_transaction: "Transaction",
    app_transaction_id: "Transaction id",
    app_signers: "Signers",
    app_sign: "Sign",
    app_signatures_for: "Signatures for this transaction",
    app_finish: "Finish",
    app_path: "Path",
    app_nonces: "Nonces",
    app_on_paper_words: "On paper, words by hand",
    app_on_paper_seedqr: "On paper, SeedQR by hand",
    app_into_vaults: "Into vaults",
    app_as_a_file_unprotected: "As a file, unprotected",
    app_sparrow: "Sparrow",
    app_coldcard_keystone: "Coldcard, Keystone, Passport",
    app_nunchuk: "Nunchuk",
    app_bitcoin_core: "Bitcoin Core",
    app_not_sure: "Not sure",
    app_a_share_in_each: "A share in each place",
    app_a_sheet_in_each: "A sheet in each place",
    app_in_the_vault: "In the vault",
    app_into_watch_only: "Into watch-only software",
    app_as_files_on: "As files on {}",
    app_address: "Address",
    app_message: "Message",
    app_format: "Format",
    app_signature: "Signature",
    app_back_up: "Back up",
    app_kind: "Kind",
    app_quorum: "Quorum",
    app_description: "Description",
    app_keys: "Keys",
    app_check: "Check",
    app_done: "Done",
    app_super_s_held_2: "Super + S held 2 s: power off at once",
    app_for_totp_codes: "For TOTP codes, GPG and Secure Boot signatures, and backup check dates",
    app_this_computer_is: "This computer is online",
    app_anything_running: "Anything running on it can read a mainnet key typed, scanned or loaded here",
    app_for_real_funds: "For real funds, use Faraday booted from a stick on a computer with no network",
    loose_card_key_is_loaded: "Key {} is loaded. No wallet uses it.",
    loose_card_keys_are: "{} keys are loaded. No wallet uses them.",
    loose_card_add_another: "Add another key",
    loose_card_restore: "Restore an existing wallet",
    wallets_signing: "Signing",
    wallets_continue: "Continue",
    wallets_in_files: "in Files",
    wallets_review: "Review",
    wallets_wallets: "Wallets",
    wallets_watch_only: "{} · watch only",
    wallets_of_here: "{} · {} of {} here",
    wallets_keys_without: "Keys without a wallet",
    wallets_restore_an: "Restore an existing wallet",
    wallets_scan_a_key_for: "Scan a key for {}",
    wallets_add_a_key: "Add a key",
    wallets_no_wallet_loaded: "No wallet loaded",
    wallets_enter_keeps: "Enter keeps it · Esc leaves it",
    wallets_rename: "Rename",
    wallets_show_wallet: "Show wallet QR",
    wallets_back_up: "Back up",
    wallets_sign_a_message: "Sign a message",
    wallets_remove_from: "Remove from session",
    wallets_of_signatures: "{} of {} signatures possible here",
    wallets_can_spend: "Can spend",
    wallets_scan_a_key: "Scan a key",
    wallets_at_a_glance: "At a glance",
    wallets_first_receive: "First receive address · 0/0",
    silent_wallet_card_key: "Key",
    silent_wallet_card_here: "Here",
    silent_wallet_card_not: "Not here",
    silent_wallet_card_load: "Load from vault",
    silent_wallet_card_add: "Add its key",
    silent_wallet_card_address_2: "Address",
    silent_wallet_card_labels: "Labels handed out",
    silent_wallet_card_address: "Address, labels and payments",
    silent_wallet_card_show: "Show wallet QR",
    silent_wallet_card_remove: "Remove from session",
    spend_no_transaction: "No transaction loaded",
    spend_open_files: "Open Files",
    spend_wallets: "Wallets",
    spend_sign_a_transaction: "Sign a transaction",
    spend_status_share: "share {}",
    spend_status_carry: "Carry file kept · the next share signs with it",
    spend_status_the_next: "The next share signs with the carry file",
    spend_status_both_wait: "Both wait {}",
    spend_status_complete: "Complete",
    spend_status_sign_here_more: "Sign here · {} more at {} own place",
    spend_status_its: "its",
    spend_status_their: "their",
    spend_status_sign_here_and: "Sign here and collect {} more",
    spend_status_sign_here: "Sign here",
    spend_status_keep_the: "Keep the carry file",
    spend_status_insert: "Insert {} to write it",
    spend_status_open_files: "Open Files",
    spend_status_finish: "Finish",
    spend_status_sign_with: "Sign with {}",
    spend_bar_of: "{} of {}",
    step_summary_no_loaded: "No loaded wallet matches",
    step_summary_addresses: "Addresses · Compare",
    step_summary_sends: "Sends {} BTC · fee {} sat/vB",
    step_summary_here_needed: "{} here · {} needed",
    step_summary_not_signed: "Not signed here yet",
    step_summary_signed_with: "Signed with {}",
    step_summary_of: "{} of {}",
    step_summary_of_nonces: "{} of {} nonces",
    step_summary_way_to: "way to spend",
    step_summary_ways_to: "ways to spend",
    step_summary_signed: "Signed PSBT and finished transaction",
    step_summary_not_finished: "Not finished yet",
    step_summary_not_reached: "Not reached yet",
    step_from: "{} · from {}",
    step_no_loaded_wallet: "No loaded wallet matches this transaction. Change cannot be verified until its wallet is loaded.",
    step_receive_0_0: "Receive 0/0",
    step_receive_0_1: "Receive 0/1",
    step_change_1_0: "Change 1/0",
    step_no_wallet_to_show: "No wallet to show addresses from",
    step_sends: "Sends",
    step_to_2: "to {}/{}",
    step_change: "Change",
    step_to: "to {}",
    step_unverified: "Unverified",
    step_fee: "Fee",
    step_sat_sat_vb: "{} sat · {} sat/vB",
    step_spends: "Spends",
    step_amount_unchecked: "Amount unchecked",
    step_unchanged_by_signing: "Unchanged by signing",
    step_changes_when_signed: "Changes when signed",
    step_txid: "Txid",
    step_not_known_until: "Not known until signed",
    step_no_warnings: "No warnings",
    step_input: "Input",
    step_output: "Output",
    step_hide_the_table: "Hide the table",
    step_show_the_table: "Show the table",
    step_share_n: "Share {}",
    step_signed: "Signed",
    step_signs_at_its_own: "Signs at its own place",
    step_not_signing: "Not signing",
    step_signs_next: "Signs next",
    step_choose: "Choose",
    step_needs_here_chosen: "Needs {} · {} here · {} chosen",
    step_needs_signed: "Needs {} · {} signed",
    step_load_from_vault: "Load from vault",
    step_signs_here: "Signs here",
    step_not_here: "Not here",
    step_unlock: "Unlock {}",
    step_scan_a_key: "Scan a key",
    step_collect_a_signature: "Collect a signature",
    step_remove_the_first: "Remove the {} first",
    step_no_share_of_this: "No share of this wallet is loaded here",
    step_choose_other_under: "Choose {} other {} under Signers",
    step_share_inline: "share",
    step_shares: "shares",
    step_sign_as: "Sign as {}",
    step_signed_with: "Signed with {}",
    step_no_key_in_this: "No key in this session signs this transaction",
    step_sign_with: "Sign with {}",
    step_input_inline: "input",
    step_inputs: "inputs",
    step_from_inline: "from {}",
    step_signed_here: "Signed here",
    step_verified: "Verified",
    step_not_needed: "Not needed",
    step_no_cosigner_s: "No cosigner's copy in Files",
    step_scan_a_copy: "Scan a copy",
    step_share_n_inline: "share {}",
    step_carry_file: "Carry file",
    step_for_holds_its: "For {} · holds its secret nonce until it signs",
    step_the_next_share: "the next share",
    step_kept: "Kept",
    step_keep_it: "Keep it",
    step_needs: "Needs {}{} {}",
    step_more: "more",
    step_signature: "signature",
    step_signatures: "signatures",
    step_signed_psbt: "Signed PSBT",
    step_for_the_wallet: "For the wallet that wrote it · not finalised",
    step_finished_transaction: "Finished transaction",
    step_ready_to_broadcast: "Ready to broadcast",
    step_remove: "Remove",
    step_make_again: "Make again",
    step_vb: "{} · {} vB",
    step_decode_it: "Decode it",
    step_hide_the_raw_hex: "Hide the raw hex",
    step_show_the_raw_hex: "Show the raw hex",
    step_this_device: "This device",
    step_nonce_here: "Nonce here",
    step_waiting: "Waiting",
    step_add_from: "Add from {}",
    step_show_as_qr: "Show as QR",
    step_go_first_share: "Go first: share this device's nonce",
    step_path: "Path {}",
    step_key: "key {} {}",
    step_no_wait: "No wait",
    step_waits: "Waits {}",
    step_this_transaction_s: "This transaction's sequence allows this path",
    step_this_transaction: "This transaction's sequence does not allow this path",
    step_continue: "Continue",
    spend_panel_this_transaction: "This transaction",
    spend_panel_verified: "Verified",
    spend_panel_unverified: "Unverified",
    spend_panel_none: "None",
    spend_panel_wallet: "Wallet",
    spend_panel_not_loaded: "Not loaded",
    spend_panel_sends: "Sends",
    spend_panel_btc: "{} BTC",
    spend_panel_to: "To",
    spend_panel_fee: "Fee",
    spend_panel_sat_sat: "{} sat · {} sat/vB",
    spend_panel_change: "Change",
    spend_panel_txid: "Txid",
    spend_panel_signatures: "Signatures",
    spend_panel_of: "{} of {}",
    question_title_start: "Start from",
    question_title_the_seeds: "The seeds go",
    question_title_places: "Places",
    question_title_the: "The wallet description goes",
    question_title_software: "Software",
    question_title_passphrases: "Passphrases",
    question_title_map: "Map",
    ticked_none: "None",
    item_title_print_blank: "Print {} blank {}",
    item_title_template_inline: "template",
    item_title_templates: "templates",
    item_title_template_n: "Template for {} words",
    item_title_copy_seed: "Copy seed {} by hand",
    item_title_checked: "Checked",
    item_title_not_checked: "Not checked",
    item_title_vault_2: "Vault {}",
    item_title_vault: "Vault {} · {}",
    item_title_in: "In {}",
    item_title_not_saved: "Not saved",
    item_title_the_seeds: "The seeds as files",
    item_title_not_made: "Not made",
    item_title_the_shares: "The shares",
    item_title_shares_keys: "{} shares, {} keys each",
    item_title_the_wallet: "The wallet sheet",
    item_title_a_pdf_to: "A PDF to print",
    item_title_the_public: "The public files",
    item_title_show_the: "Show the descriptor to the software",
    item_title_shown: "Shown",
    item_title_not_shown: "Not shown",
    item_title_one_envelope: "One envelope per place",
    item_title_envelope: "envelope",
    item_title_envelopes: "envelopes",
    question_summary_3: "3 presets",
    question_summary_place: "place",
    question_summary_places: "places",
    question_summary_a_share_each: "· a share each",
    question_summary_the_whole_sheet: "· the whole sheet",
    question_summary_passphrase: "passphrase",
    question_summary_passphrases: "passphrases",
    question_summary_any: "Any one place lost: {}",
    backup_after: "After: {}",
    backup_back_up_a_wallet: "Back up a wallet",
    backup_back_up: "Back up {}",
    backup_vault_contents: "Vault contents",
    backup_wallets: "Wallets",
    backup_then_wallets: "Then: Wallets",
    backup_vault_s_needs: "Vault {}'s {} needs a place",
    backup_make_the_checklist: "Make the checklist",
    backup_change_the_plan: "Change the plan",
    backup_words_on_the: "Words on the seed",
    backup_blank_template: "Blank template",
    backup_words_a_pdf: "{} words · a PDF to print",
    backup_no_key_of_this: "No key of this wallet was loaded from words in this session",
    backup_hide_the_seed: "Hide the seed",
    backup_show_the_seed: "Show the seed",
    backup_words: "{} · {} words",
    backup_seedqr_is_for: "SeedQR is for English words only: copy these words by hand",
    backup_standard_seedqr: "Standard SeedQR",
    backup_compact_seedqr: "Compact SeedQR",
    backup_shown: "Shown",
    backup_not_shown: "Not shown",
    backup_make_it: "Make it",
    backup_backup_sheet: "Backup sheet",
    backup_a_pdf_to_print: "A PDF to print",
    backup_make_them: "Make them",
    backup_the_shares: "The shares",
    backup_sheets_to_print: "Sheets to print, text files and pictures",
    backup_nothing: "Nothing",
    backup_back_to_the: "Back to the wallet",
    backup_done: "Done",
    plan_a_key_made_here: "A key made here signs only once this backup is done.",
    plan_places_with_paper: "Places with paper",
    plan_each_place_keeps: "Each place keeps",
    plan_the_whole_wallet: "The whole wallet sheet",
    plan_its_own_share: "Its own share",
    plan_what_is_a_share: "What is a share?",
    plan_vault_s: "Vault {}'s {}",
    plan_form: "Form",
    plan_qr_picture: "QR picture",
    plan_text: "Text",
    plan_its_words: "{} · its words",
    plan_in_the_vault_with: "In the vault, with its seed",
    plan_vault_s_needs: "Vault {}'s {} needs a place",
    plan_make_the_checklist: "Make the checklist",
    plan_continue: "Continue",
    place_names_names: "Names",
    place_names_only_in: "Only in a vault",
    place_names_names_kept: "Names, kept in {}",
    place_names_place: "Place {}",
    place_names_done: "Done",
    place_names_name: "Name",
    map_rows_of_files: "{} of files",
    map_rows_watch_only: "Watch-only software",
    map_rows_on_its_own: "On its own device",
    map_rows_nothing: "Nothing",
    backup_done_backup: "Backup done",
    backup_done_from_this: "{} {} {} · {} from this backup",
    backup_done_file: "file",
    backup_done_files: "files",
    backup_done_write_to: "Write to {}",
    backup_done_open_the: "Open the wallet",
    backup_done_change: "Change the plan",
    signers_count_of_signatures_here: "{} of {} signatures here",
    signers_count_of_signatures: "{} of {} signatures here · {} more needed",
    signers_count_can_spend: "Can spend",
    signers_count_continue: "Continue",
    shares_card_keys_left: "Keys left off each share",
    shares_card_share: "Share {}",
    shares_card_any_shares: "Any {} shares rebuild the wallet",
    shares_card_some_shares: "Some {} shares do not rebuild the wallet",
    shares_card_the_fewest: "The fewest shares that rebuild it: {}",
    shares_card_each_key: "Each key is on {} shares",
    shares_card_no_single: "No single share holds every key",
    shares_card_each_share: "Each share holds every key",
    copy_check_type_the: "Type the four-digit number beside each word, in order",
    copy_check_type_here: "Type here",
    copy_check_your_copy: "Your copy matches the seed",
    copy_check_word_differs: "Word {} differs: check its four digits",
    copy_check_of_digits: "{} of {} digits typed · all match so far",
    copy_check_clear: "Clear",
    item_action_continue: "Continue",
    item_action_stop_typing: "Stop typing",
    item_action_check_my: "Check my copy",
    item_action_lock_it: "Lock it and make a new vault",
    item_action_save_into: "Save into {}",
    item_action_make_it: "Make it",
    item_action_make_them: "Make them",
    item_action_show_as: "Show as QR",
    vault_item_holds_another: "{} holds another vault's seed",
    vault_item_into: "Into {}",
    vault_item_into_the: "Into the vault",
    vault_item_seed_with: "Seed {} with its passphrase",
    vault_item_seed: "Seed {}",
    vault_item_wallet_description: "Wallet description",
    seed_files_not_made: "Not made",
    paper_section_other: "Other paper forms",
    paper_section_seed_xor: "Seed XOR, 2 parts",
    paper_section_seed: "Seed XOR, 3 parts",
    paper_section_codex32_2: "codex32, 2 of 3",
    paper_section_codex32: "codex32, 3 of 5",
    paper_section_parts: "{} parts · every one is needed",
    paper_section_shares: "{} shares · any {} make the seed",
    paper_section_put_them: "Put them away",
    seed_grid_row: "row {}",
    backup_panel_map: "Map",
    kept_rows_keys: "Keys",
    message_none_chosen: "None chosen",
    message_not_written: "Not written yet",
    message_signed: "Signed",
    message_not_signed: "Not signed yet",
    message_wallets: "Wallets",
    message_sign_a_message: "Sign a message",
    message_receive_address: "Receive address 0/{}",
    message_type_the_message: "Type the message",
    message_press_type: "Press Type to write the message",
    message_stop_typing: "Stop typing",
    message_type: "Type",
    message_characters: "{} characters",
    message_continue: "Continue",
    message_sign_the_message: "Sign the message",
    message_signature: "Signature",
    message_make_the_file: "Make the file",
    message_png: "PNG {}",
    message_signed_message: "Signed message",
    message_show_as_qr: "Show as QR",
    check_wallets: "Wallets",
    check_check_a_signed: "Check a signed message",
    check_a_valid_signature: "A valid signature proves that whoever holds this address's key signed exactly this text. It says nothing about who is showing it to you, or when it was signed.",
    check_no_signed_message: "No signed message chosen",
    check_not_a_signed: "Not a signed message for a {} address",
    check_the_signature_is: "The signature is valid ({})",
    check_the_signature: "The signature does not verify: {}",
    check_file: "File",
    check_address: "Address",
    check_wallet: "Wallet",
    check_change: "change",
    check_receive: "receive",
    check_not_an_address: "Not an address of a loaded wallet",
    check_message: "Message",
    create_of: "{} of {}",
    create_of_chosen_computed: "{} of {} chosen · {} computed",
    create_of_chosen: "{} of {} chosen",
    create_of_chosen_to: "{} of {} chosen · {} to come",
    create_first_addresses: "{} · first addresses",
    create_not_made_yet: "Not made yet",
    create_paper_vault: "Paper, vault, watch-only software",
    create_home: "Home",
    create_wallets: "Wallets",
    create_make_a_new_wallet: "Make a new wallet",
    create_start_over: "Start over",
    create_signatures_needed: "Signatures needed",
    create_keys: "Keys",
    create_all_keys_sign: "All {} keys sign together, as one signature on chain. Losing any one key loses the money: there is no spare.",
    create_any_of_the_keys: "Any {} of the {} keys can spend. Losing {} {} still leaves the money spendable; {} keys together can take it.",
    create_key_inline: "key",
    create_keys_inline: "keys",
    create_locked_in: "Locked in",
    create_not_chosen: "Not chosen",
    create_loaded_here: "{} · {} · loaded here",
    create_a_cosigner_s: "{} · a cosigner's xpub",
    create_a_cosigner_adds: "A cosigner adds it later · waiting for its xpub file",
    create_new_key: "New key",
    create_from: "From {}",
    create_scan_its_xpub: "Scan its xpub",
    create_someone_else: "Someone else adds it later",
    create_clear: "Clear",
    create_show_xpub_qr: "Show xpub QR",
    create_xpub_png: "Xpub PNG {}",
    create_xpub_file: "Xpub file {}",
    create_add_a_passphrase: "Add a passphrase",
    create_passphrase: "Passphrase",
    create_passphrase_again: "Passphrase again",
    create_lock_in: "Lock in",
    create_share_n: "Share {}",
    create_computed_by: "Computed by the deal",
    create_new_key_for: "New key for {} {}",
    create_share: "Share",
    create_key: "Key",
    create_continue: "Continue",
    create_move_the_money: "Move the money from {}",
    create_make_the_wallet: "Make the wallet first",
    kind_rows_more_kinds: "More kinds",
    first_addresses_the: "The wallet's first addresses",
    first_addresses_receive_0: "Receive 0/0",
    first_addresses_receive: "Receive 0/1",
    first_addresses_change: "Change 1/0",
    made_place_written: "Written to {}",
    made_line_remove: "Remove",
    made_state_remove: "Remove",
    made_state_make_again: "Make again",
    locked_tag_locked_in: "Locked in",
    descriptor_row_descriptor: "Descriptor",
    public_files_show_as: "Show as QR",
    public_files_descriptor: "Descriptor",
    public_files_any_wallet: "Any wallet software",
    public_files_wallet: "Wallet file",
    public_files_sparrow: "Sparrow, Specter",
    public_files_multisig: "Multisig config",
    public_files_coldcard: "Coldcard, Keystone, Passport",
    public_files_backup: "Backup sheet",
    public_files_a_pdf: "A PDF to print",
    public_files_bsms_descriptor: "BSMS descriptor record",
    public_files_bip_129: "BIP 129: Coldcard, Sparrow, Nunchuk",
    public_files_bitcoin: "Bitcoin Core import",
    public_files_importdescriptors: "importdescriptors, watch-only",
    public_files_key: "Key {}",
    public_files_for_the: "For the cosigners",
    public_files_key_bsms: "Key {}, BSMS",
    public_files_signed: "Signed xpub record, BIP 129",
    public_rows_for_make: "Make the file",
    restore_of: "{} of {}",
    restore_none: "None",
    restore_of_filled: "{} of {} filled",
    restore_home: "Home",
    restore_restore_an: "Restore an existing wallet",
    restore_i_have_the: "I have the wallet description",
    restore_scan_it: "Scan it",
    restore_from_files: "From Files",
    restore_continue: "Continue",
    restore_it_matches: "It matches",
    restore_make_the_wallet: "Make the wallet first",
    restore_back_up_this: "Back up this wallet",
    restore_wallets: "Wallets",
    restore_description_from: "From {}",
    restore_description_i_have: "I have the seed",
    restore_description_scan: "Scan descriptor",
    restore_description_i: "I do not have it",
    restore_description_shares: "Shares of a split backup",
    restore_description_of: "{} of {} keys in hand · {}",
    restore_description_rebuild: "Rebuild the wallet",
    restore_seeds_key_of: "Key {} of the wallet",
    restore_seeds_change: "Change",
    restore_seeds_not_here: "Not here",
    restore_seeds_here: "Here",
    restore_seeds_here_n: "Here · {}",
    restore_seeds_empty: "Empty",
    restore_seeds_its_xpub: "Its xpub",
    restore_seeds_make: "Make the wallet",
    restore_open_slot_type: "Type the words",
    restore_open_slot_scan: "Scan a SeedQR",
    restore_sources_copy_files: "Copy files in",
    restore_sources_not: "Not while a key is loaded",
    restore_sources_insert: "Insert the {}",
    restore_sources_vault: "Vault",
    restore_sources_unlock: "Unlock {}",
    restore_sources_wallet: "Wallet",
    restore_sources_copy: "Copy the vault in from its {}",
    kind_line_mb: "{} MB",
    kind_line_kb: "{} KB",
    kind_name_wallet_descriptor: "Wallet descriptor",
    kind_name_finished: "Finished transaction",
    kind_name_sheet_for: "Sheet for printing",
    kind_name_signed_message: "Signed message",
    kind_name_account_xpub: "Account xpub",
    kind_name_seed_words: "Seed words",
    kind_name_share_of: "Share of a split backup",
    kind_name_threshold: "Threshold spend, part-signed",
    kind_name_vault_locked: "Vault, locked",
    kind_name_entries_for: "Entries for a vault",
    kind_name_opensigner: "OpenSigner backup, encrypted",
    kind_name_keepass_database: "KeePass database, encrypted",
    kind_name_pdf_to_print: "PDF to print",
    kind_name_efi_image: "EFI image",
    kind_name_part_of_a: "Part of a seed: SLIP-39 or codex32",
    kind_name_text: "Text",
    kind_name_file: "File",
    add_to_vault_add_to: "Add to vault",
    restore_shares_button_restore: "Restore the wallet · {} shares",
    restore_shares_button_of: "{} of {} keys in hand",
    files_files: "Files",
    files_add_to_signatures: "Add to signatures",
    files_import_into_the: "Import into the vault",
    files_contents: "Contents",
    files_unlock: "Unlock",
    files_sign: "Sign",
    files_load_the_wallet: "Load the wallet",
    files_decode: "Decode",
    files_load_this_key: "Load this key",
    files_check_the_signature: "Check the signature",
    files_waiting_for: "Waiting for {}",
    files_attached_lock: "{} attached · lock to use it",
    files_visit: "Visit {}",
    files_scan_a_qr_code: "Scan a QR code",
    files_in_memory_emptied: "In memory · emptied at power-off",
    files_locked: "locked",
    files_open: "open",
    files_empty_files_are: "Empty · files are copied in on {} visit",
    files_public: "Public",
    files_anyone_may_read: "Anyone may read these",
    files_written_sealed: "Written sealed",
    files_sealed_under: "Sealed under their passphrases before they reach {}",
    files_unprotected_secrets: "Unprotected secrets",
    files_anyone_who_has: "Anyone who has the {} can use these",
    files_vault_open_unsaved: "Vault, open · {} unsaved {} · sealed and locked at Lock",
    files_change: "change",
    files_changes: "changes",
    files_vault_open_sealed: "Vault, open · sealed and locked at Lock",
    files_unprotected: "Unprotected",
    files_written_after: "Written after Lock",
    files_ready: "Ready",
    files_make_pdf: "Make PDF",
    files_show_as_qr: "Show as QR",
    files_remove: "Remove",
    files_empty: "Empty",
    files_power_off_asks: "Power off asks first while files wait {}",
    files_holds_what_was: "{} holds what was copied in from {}; {} holds what waits to be written to one, and what the last visit wrote. Both live in memory and are emptied at power-off. Public files are listed apart from vaults, which carry secrets sealed under a passphrase; a secret goes out unprotected only when you say so after a warning, and is listed apart.",
    receipt_rows_verified: "verified",
    vault_panel_open: "{} · {} · open",
    vault_panel_select: "Select all",
    vault_panel_wallets_with: "Wallets with their seeds",
    vault_panel_of_in_this: "{} of {} {} in this vault",
    vault_panel_ready_to: "Ready to sign once loaded",
    vault_panel_more_listed: "+ {} more · listed under Each seed and wallet",
    vault_panel_load_wallet: "Load wallet with keys",
    vault_panel_load: "Load 1 wallet with keys",
    vault_panel_load_wallets: "Load {} wallets with keys",
    vault_panel_hide_each: "Hide each seed and wallet",
    vault_panel_each_seed: "Each seed and wallet · {}",
    vault_panel_loaded: "Loaded",
    vault_panel_everything: "Everything is loaded",
    vault_panel_nothing: "Nothing chosen",
    vault_panel_load_n: "Load {}",
    vault_panel_load_and: "Load {} and {}",
    vault_panel_key: "key",
    vault_panel_keys: "keys",
    vault_panel_wallet: "wallet",
    vault_panel_wallets: "wallets",
    vault_panel_go_to_wallets: "Go to Wallets",
    inbox_wallets: "Wallets",
    inbox_of_here: "{} · {} of {} {} here{}",
    inbox_key: "key",
    inbox_keys: "keys",
    inbox_ready_to_sign: "Ready to sign once loaded",
    inbox_more_loaded_with: "+ {} more, loaded with these",
    inbox_nothing_chosen: "Nothing chosen",
    inbox_load_the_wallet_with: "Load the wallet with its seeds",
    inbox_load_the_wallet: "Load the wallet",
    inbox_load_wallets_with: "Load {} wallets with their seeds",
    inbox_load_wallets: "Load {} wallets",
    inbox_keys_load_once: "Keys load once the {} is out",
    inbox_keys_with_no: "Keys with no wallet",
    inbox_make_it_a_wallet: "Make it a wallet",
    inbox_more: "+ {} more",
    inbox_xpubs_with_no: "Xpubs with no wallet",
    inbox_no_origin: "no origin",
    inbox_make_it_a_watch: "Make it a watch-only wallet",
    inbox_not_complete: "Not complete or sealed",
    inbox_open_with_its: "Open with its passphrase",
    vault_open: "{} · {} · open",
    vault_wallets_with: "Wallets with their seeds",
    vault_of_in_this_vault: "{} of {} {} in this vault",
    vault_ready_to_sign: "Ready to sign once loaded",
    vault_more_listed_under: "+ {} more · listed under Each seed and wallet",
    vault_load_wallet_with: "Load wallet with keys",
    vault_load_1_wallet: "Load 1 wallet with keys",
    vault_load_wallets: "Load {} wallets with keys",
    vault_hide_each_seed: "Hide each seed and wallet",
    vault_each_seed_and: "Each seed and wallet · {}",
    vault_select_all: "Select all",
    vault_loaded: "Loaded",
    vault_everything_is: "Everything is loaded",
    vault_nothing_chosen: "Nothing chosen",
    vault_load_n: "Load {}",
    vault_load_and: "Load {} and {}",
    vault_key_inline: "key",
    vault_keys: "keys",
    vault_wallet_inline: "wallet",
    vault_wallets: "wallets",
    vault_go_to_wallets: "Go to Wallets",
    inbox_actions_add_to_vault: "Add to vault",
    inbox_actions_add_to: "Add to signatures",
    inbox_actions_import: "Import into the vault",
    inbox_actions_contents: "Contents",
    inbox_actions_unlock: "Unlock",
    inbox_actions_sign: "Sign",
    inbox_actions_load_the: "Load the wallet",
    inbox_actions_decode: "Decode",
    inbox_actions_restore: "Restore the wallet · {} shares",
    inbox_actions_of_keys: "{} of {} keys in hand",
    inbox_actions_load: "Load this key",
    inbox_actions_check: "Check the signature",
    inbox_actions_make: "Make PDF",
    inbox_actions_remove: "Remove",
    visit_unselect_all: "Unselect all",
    visit_select_all: "Select all",
    visit_write_what_waits: "Write what waits for the {} and copy in what you need from it. Each file written is read back and compared before it counts. Remove the {} when you are done: keys load only with no {} attached.",
    visit_no_attached: "No {} attached",
    visit_write_to_the: "Write to the {}",
    visit_public: "Public",
    visit_sealed: "Sealed",
    visit_unprotected_secret: "Unprotected secret",
    visit_nothing_waits: "Nothing waits {}",
    visit_write: "Write {} {}",
    visit_copy: "Copy {}",
    visit_larger_than_18: "Larger than 18 MB: not read",
    visit_settings: "Settings",
    visit_no_files_on_this: "No files on this {}",
    visit_delete_files: "Delete files",
    visit_import_and_load: "Import and load",
    visit_import: "Import {} {}",
    visit_file: "file",
    visit_files: "files",
    visit_open_files: "Open Files",
    visit_inbox_line_png_no: "PNG, no QR code read · {}",
    visit_inbox_line_png: "PNG · {}",
    visit_inbox_tag_sealed: "Sealed",
    visit_inbox_tag_public: "Public",
    visit_inbox_tag_may: "May be a secret",
    visit_inbox_tag_secret: "Secret",
    visit_select_all_select: "Select all",
    visit_unselect_all_unselect: "Unselect all",
    visit_list_bar_more: "{} more · scroll to see them",
    pull_line_remove_the_to: "Remove the {} to unlock {}",
    pull_line_remove_the: "Remove the {} when you are done",
    pull_line_1_key_copied: "1 key copied in · remove the {} to load it",
    pull_line_keys_copied: "{} keys copied in · remove the {} to load them",
    entry_spend: "Spend",
    entry_wallets: "Wallets",
    entry_add_key: "Add key {}",
    entry_add_a_key_for: "Add a key for {}",
    entry_add_a_key_n: "Add a key · {}",
    entry_add_a_key: "Add a key",
    entry_other_languages: "Other languages",
    entry_other_forms: "Other forms",
    entry_scan_a_seedqr: "Scan a SeedQR",
    entry_type_the_words: "Type the words in order. Each word is checked against the BIP-39 list as you type, and Tab finishes a word once only one fits. The last word carries a checksum, so a mistyped word is caught when the count is complete. The words stay in memory for this session only.",
    entry_no_word_starts: "No word starts with that",
    entry_tab_completes: "· Tab completes a single match",
    word_count_row_words: "Words",
    keyed_words_type_each_word_s: "Type each word's pinyin and then its tone, 1 to 5, on the keys below or the computer's keyboard, and press the character among those it offers. Keys that lead to no word are off. The last word carries a checksum, checked when the count is complete.",
    keyed_words_type_each_word: "Type each word's 注音 and then its tone on the keys below, and press the character among those it offers. Keys that lead to no word are off. The last word carries a checksum, checked when the count is complete.",
    keyed_words_type_each: "Type each word on the keys below and press it among the words it can be. Keys that lead to no word are off. The last word carries a checksum, checked when the count is complete.",
    keyed_words_type_the: "Type the next word",
    keyed_words_now_its: "Now its tone",
    keyed_words_no_word: "No word starts with that",
    keyed_words_press_a: "Press a word to take it",
    word_keys_shift: "Shift",
    keyed_status_words_2: "{} words",
    keyed_status_words: "{} words · checksum correct · {}",
    entry_status_words_checksum: "{} words · checksum correct · {}",
    entry_status_words: "{} words",
    entry_finish_bip_39: "BIP-39 passphrase",
    entry_finish_add_key: "Add key",
    entry_finish_clear: "Clear",
    entry_finish_make_a: "Make a new key",
    entry_finish_type_on: "Type on the keyboard · Enter adds · Esc goes back",
    entry_stick_from_plug: "From {} · plug it in",
    entry_stick_copy_files: "Copy files in",
    entry_form_type_one_share: "Type one share at a time, its 20 or 33 words, and press Enter. Shares of every group the backup needs go in, in any order; then the passphrase, if the backup has one, and Make the key. A SLIP-39 key has no BIP-39 words.",
    entry_form_type_one_codex32: "Type one codex32 string at a time, ms1 and all, and press Enter. The secret string is the key at once; shares need as many as the digit after ms1 says.",
    entry_form_type_one: "Type one part at a time, its BIP-39 words, and press Enter. Every part is needed, and they combine into the words of the key.",
    entry_form_words_of: "Words of one share",
    entry_form_nothing: "Nothing in yet",
    entry_form_1_in: "1 in",
    entry_form_in: "{} in",
    entry_form_passphrase: "Passphrase",
    entry_form_add_this: "Add this one",
    entry_form_scan_a_qr: "Scan a QR code",
    entry_form_make_the: "Make the key",
    entry_form_enter_adds: "Enter adds what is typed · Enter on an empty line makes the key",
    tz_row_time_zone: "Time zone",
    time_section_time: "Time",
    time_section_now: "Now",
    time_section_set_the: "Set the time",
    set_time_set_the_time: "Set the time",
    set_time_now: "Now",
    set_time_date: "Date",
    set_time_time: "Time",
    set_time_set: "Set",
    set_time_keep_this: "Keep this time",
    set_time_back: "Back",
    settings_time: "Time",
    settings_now: "Now",
    settings_set_the_time: "Set the time",
    settings_settings: "Settings",
    settings_session: "Session",
    settings_loaded_lost: "{} {} loaded · lost on lock or power-off",
    settings_key: "key",
    settings_keys: "keys",
    settings_lock: "Lock",
    settings_remove_every: "Remove every key",
    settings_power_off_2: "Power off",
    settings_lock_after: "Lock after",
    settings_power_off: "Power off after",
    settings_never: "Never",
    settings_min: "{} min",
    settings_ignored_until: "Ignored until unplugged: {}",
    settings_network: "Network",
    settings_appearance: "Appearance",
    settings_motion: "Motion",
    settings_full: "Full",
    settings_reduced: "Reduced",
    settings_display_scale: "Display scale",
    settings_auto: "Auto",
    settings_signed_amounts: "Signed amounts",
    settings_remembered: "{} {} remembered",
    settings_transaction: "transaction",
    settings_transactions: "transactions",
    settings_sealed_into: "Sealed into an open vault",
    settings_kept_until: "Kept until power-off only",
    settings_about: "About",
    settings_faraday: "Faraday {}",
    settings_keys_are_typed: "Keys are typed, scanned as SeedQR or loaded from a vault, and kept for the session only",
    settings_vaults_files: "Vaults · files move by {} · QR by camera and from PNG files on {}",
    settings_are_folders: "{} are folders in ~/faraday-sticks",
    settings_are_read_by: "{} are read by an unprivileged disk process; the kernel mounts none",
    settings_not_run: "not run",
    institute_note_visit: "Visit {} to learn about Bitcoin’s history, economics, and technology. Not affiliated.",
    pull_go_on: "go on",
    pull_pull_the_to: "Pull the {} to {}",
    pull_attached_2: "{} · attached",
    pull_attached: "{} {} attached",
    pull_cancel: "Cancel",
    kept_sections_kept: "Kept",
    kept_sections_wiped: "Wiped from memory",
    kept_sections_then: "Then",
    lock_a_fresh_start_and: "A fresh start, and the upgrade",
    lock_a_fresh_start: "A fresh start, and the {} visit",
    lock_lock_and_upgrade: "Lock and upgrade",
    lock_the_upgrade_runs: "The upgrade runs in a fresh session",
    lock_is_attached: "{} is attached",
    lock_lock_and_use: "Lock and use {}",
    lock_nothing_has_been: "{} · nothing has been read from it",
    lock_not_now: "Not now",
    write_out_lock_plug: "Lock, plug in {}, and the visit writes them",
    write_out_write_to: "Write to {}",
    write_out_not_now: "Not now",
    write_out_lock: "Lock",
    scan_scan_a_key_for: "Scan a key for {}",
    scan_scan_what_you: "Scan what you have",
    scan_scan_a_seedqr: "Scan a SeedQR",
    scan_scan_your_copy: "Scan your copy",
    scan_receive_into_downloads: "Receive into Downloads",
    scan_scan_a_qr_code: "Scan a QR code",
    scan_hold_the_code: "Hold the code in front of the camera",
    scan_starting_the_camera: "Starting the camera",
    scan_type_the_words: "Type the words instead",
    scan_done: "Done",
    scan_cancel: "Cancel",
    qr_frame_of: "{} · frame {} of {}",
    qr_secret_whoever_scans: "Secret · whoever scans this can use it",
    qr_public_safe_to_scan: "Public · safe to scan",
    qr_ur: "UR",
    qr_bbqr: "BBQr",
    qr_parts: "Parts",
    qr_png: "PNG {}",
    qr_small: "Small",
    qr_medium: "Medium",
    qr_large: "Large",
    qr_done: "Done",
    qr_sheet_ur: "UR",
    qr_sheet_bbqr: "BBQr",
    qr_sheet_parts: "Parts",
    qr_sheet_small: "Small",
    qr_sheet_medium: "Medium",
    qr_sheet_large: "Large",
    qr_sheet_of: "{} · {} of {}",
    qr_sheet_secret_whoever: "Secret · whoever scans this can use it",
    qr_sheet_public_safe: "Public · safe to scan",
    qr_sheet_done: "Done",
    new_input_a_new_keyboard_and: "A new keyboard and pointer",
    new_input_a_new_keyboard: "A new keyboard",
    new_input_a_new_pointer: "A new pointer",
    new_input_the_name: "The name is what the device says about itself",
    new_input_type_this: "Type this code on it",
    new_input_no_code: "This keyboard cannot be let in until Faraday restarts",
    new_input_ignore_this: "Ignore this device",
    new_input_use_this: "Use this pointer",
    idle_warn_wiped: "Wiped",
    idle_warn_open: "{} {} · {} {} · {} open {}",
    idle_warn_key: "key",
    idle_warn_keys: "keys",
    idle_warn_wallet: "wallet",
    idle_warn_wallets: "wallets",
    idle_warn_vault: "vault",
    idle_warn_vaults: "vaults",
    idle_warn_sealed: "Sealed",
    idle_warn_kept: "Kept",
    idle_warn_no_power: "No power-off",
    idle_warn_power_off: "Power off at {} minutes without input, unless files wait {}",
    idle_warn_then: "Then",
    idle_warn_minute: "minute",
    idle_warn_minutes: "minutes",
    idle_warn_no_input: "No input for {} · locks at {} without input · any key or touch keeps it",
    idle_warn_locking_soon: "Locking soon",
    idle_warn_keep_working: "Keep working",
    locked_locked: "Locked",
    locked_locked_after: "Locked after a time without input. Every key and vault was wiped.",
    locked_powers_off_in: "Powers off in {} without input",
    locked_no_power_off: "{} {} · no power-off until written",
    locked_continue: "Continue",
    network_row_note_wallets: "Wallets loaded",
    not_airgapped_cancel: "Cancel",
    not_airgapped_i_understand: "I understand",
    not_airgapped_not_air: "Not air-gapped",
    power_the_computer: "The computer powers off",
    power_cancel: "Cancel",
    power_power_off: "Power off",
    learn_done: "Done",
    learn_spending_step: "Spending, step by step",
    secret_out_this_may: "This may be a secret",
    secret_out_this_is: "This is a secret",
    secret_out_put_it_unprotected: "Put it {} unprotected",
    secret_out_unprotected_on: "Unprotected, on the {}",
    secret_out_tick_it: "Tick it to write",
    secret_out_unprotected: "Unprotected, {}",
    secret_out_file: "File",
    secret_out_what_it: "What it is",
    secret_out_who_can: "Who can use it",
    secret_out_sealed_in: "Sealed in a vault",
    secret_out_in: "In {}",
    secret_out_save_into: "Save into {}",
    secret_out_cancel: "Cancel",
    secrets_anyone_who: "Anyone who copies the {} or sees the code can read it",
    secrets_i_understand: "I understand: anyone who copies the {} or sees this file can spend these coins",
    secrets_faraday_cannot: "Faraday cannot tell whether this is a secret: anyone who copies the {} can read it",
    secrets_words: "Words",
    secrets_seed: "Seed {}",
    secrets_words_secret: "{} words · secret: spends",
    secrets_compact_seedqr: "Compact SeedQR PNG",
    secrets_seedqr_png: "SeedQR PNG",
    secrets_whoever_has_it: "Whoever has it spends this wallet's coins",
    secrets_whoever_has: "Whoever has it signs as key {}; with {} more {}, spends",
    secrets_key: "key",
    secrets_keys: "keys",
    secrets_a_wallet_s: "A wallet's seed",
    secrets_no_vault_is: "No vault is open",
    secrets_the_nonce_is: "The nonce is in {}; {} waits {}",
    secrets_is_in: "{} is in {}",
    secrets_waits_unprotected: "{} waits {}, unprotected",
    secureboot_no_randomness: "No randomness from the system yet. Try again",
    secureboot_pk_kek_and: "PK, KEK and db are in the vault",
    secureboot_this_computer: "This computer's clock is not known",
    secureboot_enrolment: "{} enrolment files",
    secureboot_signed: "{}, signed,",
    secureboot_signed_by_this: "Signed by this vault's db key",
    secureboot_signed_by: "Signed by another key",
    secureboot_changed: "Changed since it was signed",
    secureboot_not_signed: "Not signed",
    secureboot_its_signature: "Its signature could not be read",
    seed_read_type_the: "Type the passphrase",
    seed_read_key_already: "Key {} · already here",
    seed_read_passphrase_3: "{} · passphrase",
    seed_read_not_this: "Not this one",
    seed_read_cancel: "Cancel",
    seed_read_use_with: "Use with this passphrase",
    seed_read_use_without: "Use without a passphrase",
    seed_read_add_a_passphrase: "Add a passphrase",
    seed_read_without_a: "Without a passphrase",
    seed_read_passphrase_2: "Passphrase",
    seed_read_passphrase: "Passphrase again",
    seed_read_with_this: "With this passphrase",
    seed_read_the_two_passphrases: "The two passphrases differ",
    seed_read_seedqr_read: "SeedQR read",
    seeds_no_account_key: "No account key in that file",
    seeds_type_a_derivation: "Type a derivation path, like m/48'/0'/0'/2'",
    seeds_a_seed_is_no: "A seed is no longer loaded",
    seeds_no_seed_yet: "No seed yet",
    seeds_key_is_a_key: "Key {} is a {} key; this session is on {}",
    seeds_key_has_no_account: "Key {} has no account key yet",
    seeds_key_is_not_an: "Key {} is not an account key",
    seeds_of_keys_in_hand: "{} of {} keys in hand",
    seeds_loaded_from: "{} loaded from {}",
    seeds_typed_seed: "Typed seed",
    seeds_typed_seeds: "Typed seeds",
    seeds_load_from: "Load {} from {}",
    seeds_of: "{} of {}",
    seeds_one_key_signs: "one key signs",
    seeds_of_keys: "{} {} of {} keys",
    seeds_signature: "signature",
    seeds_signatures_inline: "signatures",
    seeds_signatures: "Signatures",
    seeds_keys: "Keys",
    seeds_change: "Change",
    seeds_type_or_paste: "Type or paste an xpub",
    seeds_not_an_account: "Not an account key",
    seeds_use: "Use {}",
    seeds_scan: "Scan",
    seeds_cosigners_account: "Cosigners' account keys (xpub)",
    seeds_kind: "Kind",
    seeds_derivation_path: "Derivation path",
    seeds_standard: "Standard",
    seeds_account: "Account",
    seeds_custom_path: "Custom path",
    seeds_first_receive: "First receive address · 0/0",
    seeds_back_to_the_seeds: "Back to the seeds",
    seeds_make_the_wallet: "Make the wallet",
    set_time_not_known: "Not known",
    set_time_time_set_to: "Time set to {}:{}",
    silent_load_a_key_first: "Load a key first",
    silent_silent_payments_is: "Silent payments is not open",
    silent_choose_a_key: "Choose a key first",
    silent_silent_payment: "Silent payment address · {}",
    silent_no_address_for: "No address for this key",
    silent_label: "Label {}",
    silent_public_receives: "Public: receives payments, spends nothing",
    silent_silent_payments_address: "Silent payments address",
    silent_no_record_for: "No record for this key",
    silent_public_watch: "Public: watch only, spends nothing",
    silent_silent_payments_record: "Silent payments record",
    silent_no_vault_is: "No vault is open",
    silent_silent_payments_scan: "Silent payments scan key · {}",
    silent_is_in_the_vault: "{} is in the vault",
    silent_a_silent_payments: "A silent payments scan key",
    silent_whoever_has: "Whoever has it sees every payment to this address; it cannot spend them",
    silent_load_the_key: "Load the key first",
    silent_silent_payments_n: "Silent payments {}",
    silent_silent_payments: "Silent payments",
    silent_added_to_your: "Added to your wallets",
    silent_not_a_transaction: "Not a transaction",
    silent_key: "Key",
    silent_address: "Address",
    silent_scan_key: "Scan key",
    silent_check_a_payment: "Check a payment",
    silent_wallets: "Wallets",
    silent_not_chosen: "Not chosen",
    silent_a_secret_shows: "A secret: shows every payment",
    silent_needs_a_previous: "Needs a previous transaction",
    silent_a_transaction: "A transaction from Files",
    silent_a_silent_payments_address: "A silent payments address is one address you can give out many times: each payment to it lands at an address nobody else can link to it or to the others.",
    silent_give_this_address: "Give this address to whoever pays you. A label makes another address of the same wallet, so you can tell who paid; the payer cannot tell the two apart. The record lists the labels handed out, for the wallet software that watches for payments.",
    silent_finding_payments: "Finding payments needs the scan key: it shows every payment to this address, though it cannot spend them. It goes into the vault, or to a scanning wallet you run yourself.",
    silent_a_payment_to: "A payment to a silent payments address goes to an address only this key can find. Copy the transaction into Files, signed, with the transactions it spends from; this works out whether any of its outputs pay this wallet, and to which label.",
    silent_its_key_is_not_loaded: "Its key is not loaded: labels, the scan key and a check need it",
    silent_add_its_key: "Add its key",
    silent_no_label: "No label",
    silent_show_as_qr: "Show as QR",
    silent_as_a_bitcoin: "As a bitcoin: link",
    silent_record: "Record {}",
    silent_record_as_qr: "Record as QR",
    silent_record_as_png: "Record as PNG",
    silent_add_as_a_wallet: "Add as a wallet",
    silent_continue: "Continue",
    silent_secret_shows: "Secret · shows every payment, cannot spend",
    silent_save_into: "Save into {}",
    silent_out_unprotected: "Out unprotected…",
    silent_pays_this_wallet: "Pays this wallet: {} {}",
    silent_output: "output",
    silent_outputs: "outputs",
    silent_pays_nothing: "Pays nothing to this wallet",
    silent_spends_nothing: "Spends nothing a silent payment is made from",
    silent_not_signed_an: "Not signed: an input's key is not in it yet",
    silent_its_key_is_not: "Its key is not loaded",
    silent_the_address: "the address",
    silent_change: "change",
    silent_label_inline: "label {}",
    silent_output_to: "Output {} · {} · to {}",
    silent_needs_the_transaction: "Needs the transaction it spends from, {}: copy it into Files and check again",
    silent_check: "Check {}",
    silent_no_transaction: "No transaction in Files",
    stick_settings_wrote: "Wrote {}, read back and matched",
    stick_settings_not: "{} not written: {}",
    stick_wipe_no_zeroing: "No zeroing is a certain erase. A USB stick or SD card can keep old copies of a file's blocks where no write reaches: its controller moves data around inside the chip. Zeros stop recovery tools; a lab that reads the chip itself may still find them. Only destroying the stick is certain.",
    stick_wipe_faraday: "Faraday only ever writes into this partition, so zeroing it reaches everything Faraday ever wrote to the stick.",
    stick_wipe_if_a_seed: "If a seed or other secret was ever saved to this stick unencrypted, the only sure way to remove it is to destroy the stick.",
    stick_wipe_only_this: "Only this stick's files partition is written. Its other partitions, its partition table and any space outside a partition are not touched.",
    stick_wipe_why_the: "Why: the process that reads sticks takes in whatever a stick brings, so it is given the files partition and nothing else. It can never write a boot partition or a partition table, where a bad stick could leave code for the next computer to boot.",
    stick_wipe_gb: "{} GB",
    stick_wipe_mb: "{} MB",
    stick_wipe_kb: "{} KB",
    stick_wipe_about_a: "about a minute",
    stick_wipe_about_minutes: "about {} minutes",
    stick_wipe_about_hours: "about {} hours",
    stick_wipe_deleted: "{} deleted, zeros over it first",
    stick_wipe_not_deleted: "{} not deleted: {}",
    stick_wipe_stopped_at: "Stopped at {}%",
    stick_wipe_free_space: "Free space on {} zeroed",
    stick_wipe_s_partition_zeroed: "{}'s partition zeroed",
    stick_wipe_s_partition: "{}'s partition zeroed, {} passes",
    stick_wipe_stopped: "Stopped at {}%: {}",
    stick_wipe_the_was: "the {} was pulled",
    stick_wipe_delete_files: "Delete files from {}",
    stick_wipe_delete_with: "Delete {} {} with zeros",
    stick_wipe_file: "file",
    stick_wipe_files: "files",
    stick_wipe_zero_the_free: "Zero the free space",
    stick_wipe_zero_the: "Zero the whole partition",
    stick_wipe_nothing: "Nothing to write: a folder standing for a stick",
    stick_wipe_to_write: "{} to write · {} on a typical stick",
    stick_wipe_reading: "Reading the stick's space",
    stick_wipe_danger_zone: "Danger zone: what is deleted here cannot be undone",
    stick_wipe_chosen_files: "Chosen files",
    stick_wipe_zeros_over: "Zeros over each, then it is removed with its name",
    stick_wipe_the_free: "The free space",
    stick_wipe_the_files: "The files stay; what earlier deletes left is zeroed",
    stick_wipe_the_whole: "The whole partition",
    stick_wipe_every_file: "Every file is deleted, then all of it is zeroed",
    stick_wipe_no_files: "No files on this {}",
    stick_wipe_one_pass: "One pass",
    stick_wipe_two_passes_2: "Two passes",
    stick_wipe_two_passes: "Two passes write it all twice, which makes a stick replace more of its old blocks.",
    stick_wipe_cancel: "Cancel",
    stick_wipe_zeroing_the: "Zeroing the whole partition on {}",
    stick_wipe_zeroing: "Zeroing the free space on {}",
    stick_wipe_left: "{}% · {} left",
    stick_wipe_pass_of: "Pass {} of {}",
    stick_wipe_keep_the: "Keep the {} in until it is done",
    stick_wipe_stopping: "Stopping",
    stick_wipe_stop: "Stop",
    tools_calculators_each: "Calculators: each takes what you type, works one thing out and keeps nothing. None of them touches a key; an extended private key is refused.",
    tools_wallets: "Wallets",
    tools_tools: "Tools",
    tools_read_as_automatic: "Read as: automatic",
    tools_text: "Text",
    tools_hex: "Hex",
    tools_in: "In {}",
    tools_text_or_hex: "Text or hex",
    tools_base58_bech32: "Base58, bech32 or hex",
    tools_a_descriptor: "A descriptor",
    tools_an_xpub_ypub: "An xpub, ypub, zpub, tpub…",
    tools_an_amount: "An amount",
    tools_a_policy_e_g: "A policy, e.g. or(pk(9a6a2580),and(pk(…),older(1000)))",
    tools_clear: "Clear",
    tools_encoding: "Encoding",
    tools_prefix: "Prefix",
    tools_version: "Version",
    tools_bytes: "Bytes",
    tools_not_base58_bech32: "Not Base58, bech32, bech32m or hex",
    tools_checksum: "Checksum",
    tools_given: "Given",
    tools_holds: "holds",
    tools_does_not_hold: "does not hold",
    tools_with_it: "With it",
    tools_not_one_this: "not one this device loads",
    tools_not_a_descriptor: "Not a descriptor",
    tools_legacy: "Legacy",
    tools_nested_segwit: "Nested SegWit",
    tools_native_segwit: "Native SegWit",
    tools_taproot: "Taproot",
    tools_depth: "Depth",
    tools_fingerprint: "Fingerprint",
    tools_child: "Child",
    tools_that_is_an_extended: "That is an extended private key: this tool takes public keys only",
    tools_not_an_extended: "Not an extended key",
    tools_descriptor: "Descriptor",
    tools_spend_paths: "Spend paths",
    tools_wallet: "Wallet",
    tools_one_this_device: "one this device loads",
    tools_keys_need_origins: "keys need origins and both chains to load",
    transfer_ur_part_of: "UR: part {} of {}",
    transfer_not_a_part: "Not a part of this code: {}",
    transfer_larger_than: "Larger than 256 KiB: carry it on {}",
    transfer_is_empty: "{} is empty",
    transfer_saving: "Saving {}",
    transfer_saved_this: "Saved {}: this file holds a secret",
    transfer_saved: "Saved {}",
    transfer_not_saved: "Not saved: {}",
    transfer_mb: "{} MB",
    transfer_kb: "{} KB",
    transfer_transfer: "Transfer",
    transfer_send: "Send",
    transfer_reading: "Reading {}",
    transfer_no_files: "No files",
    transfer_over_256_kib: "Over 256 KiB",
    transfer_receive: "Receive",
    transfer_scan_with: "Scan with the camera",
    transfer_open_folder: "Open folder",
    ui_sketch_mock_up: "Mock-up: the real screen may look different.",
    ui_dark: "Dark",
    ui_light: "Light",
    ui_cobalt: "Cobalt",
    ui_nord: "Nord",
    ui_catppuccin: "Catppuccin",
    ui_tokyo_night: "Tokyo Night",
    ui_gruvbox: "Gruvbox",
    ui_rose_pine: "Rosé Pine",
    ui_bitcoin_orange: "Bitcoin Orange",
    ui_bitcoin_orange_light: "Bitcoin Orange Light",
    ui_lamplight: "Lamplight",
    upgrade_0_1_0_or_earlier: "0.1.0 or earlier",
    upgrade_test_release_2: "{} test release ({})",
    upgrade_test_release: "{} test release ({}, changes {})",
    upgrade_dev: "{} dev ({})",
    upgrade_dev_changes: "{} dev ({}, changes {})",
    upgrade_insert_the: "Insert the {} Faraday started from",
    upgrade_read: "Read",
    upgrade_reading: "Reading",
    upgrade_remove_it_insert: "Remove it. Insert the {} to upgrade",
    upgrade_file: "file",
    upgrade_files: "files",
    upgrade_its_data_partition: "Its data partition",
    upgrade_on_it: "On it",
    upgrade_to_be_written: "To be written",
    upgrade_newer_than: "Newer than this Faraday",
    upgrade_writes_a_dev: "Writes a dev build, with a serial console",
    upgrade_already_this: "Already this version",
    upgrade_boot_partition: "Boot partition {} MB · needs {} MB",
    upgrade_removed_during: "Removed during the write: it does not start until upgraded again · its data partition was not written",
    upgrade_not_written: "Not written: {}",
    upgrade_writing: "Writing",
    upgrade_upgrade: "Upgrade",
    upgrade_written_read: "Written, read back and matched",
    upgrade_remove_the: "Remove the {}",
    upgrade_upgrade_another: "Upgrade another {}",
    upgrade_done: "Done",
    upgrade_several_hold: "{} sticks in carry this version · Remove all but the one Faraday started from",
    upgrade_source_size: "The stick carrying this version has a {} MB boot partition; this Faraday's is {} MB",
    vanity_legacy: "Legacy",
    vanity_nested_segwit: "Nested SegWit",
    vanity_native_segwit: "Native SegWit",
    vanity_taproot: "Taproot",
    vanity_m_or_n: "{} · m… or n…",
    vanity_account_number: "Account number",
    vanity_the_same_words: "The same words and passphrase, a different account: fast. The wallet is the key's own, at the account found.",
    vanity_passphrase: "Passphrase",
    vanity_a_few_characters: "A few characters added to the key's BIP-39 passphrase: slow, one PBKDF2 a try. The words with that passphrase are another wallet, which needs the passphrase kept with them.",
    vanity_load_a_key_first: "Load a key first",
    vanity_type_at_least: "Type at least one character after the fixed part",
    vanity_this_key_was: "This key was not loaded from BIP-39 words: only its account turns",
    vanity_every_counter: "Every counter was tried: no address begins that way",
    vanity_vanity_passphrase: "{} · vanity passphrase",
    vanity_vanity_search: "Vanity search",
    vanity_is_open: "{} is open",
    vanity_cannot_stand: "{} cannot stand there in this kind of address",
    vanity_key: "Key",
    vanity_what_turns: "What turns",
    vanity_kind_of_address: "Kind of address",
    vanity_characters: "Characters",
    vanity_search: "Search",
    vanity_wallets: "Wallets",
    vanity_vanity_address: "Vanity address",
    vanity_the_key_whose: "The key whose address is searched. Nothing about the key changes.",
    vanity_a_search_tries: "A search tries one account number, or one passphrase, after another until the first receive address begins with the characters chosen. The same key and the same number give the same address on any device.",
    vanity_the_kind_of: "The kind of address decides the characters it starts with.",
    vanity_type_the_characters: "Type the characters the address should begin with, after the part every address of this kind shares. Each character more takes 32 times as long to find for a bech32 address, and 58 times for a legacy one.",
    vanity_the_search_runs: "The search runs while this screen is open and stops at the first match. The time is an expectation, not a promise: a find can come much sooner or later.",
    vanity_not_chosen: "Not chosen",
    vanity_tried_2: "{} tried",
    vanity_about_tries: "About {} tries",
    vanity_continue: "Continue",
    vanity_found: "Found",
    vanity_account_of_this: "Account {} of this key, after {} tries",
    vanity_added_to_the: "Added to the passphrase:",
    vanity_hide: "Hide",
    vanity_show: "Show",
    vanity_these_characters: "These characters are part of the passphrase: write them down with it. The words with any other passphrase open another wallet.",
    vanity_open_this_wallet: "Open this wallet",
    vanity_search_again: "Search again",
    vanity_a_second: "{} a second",
    vanity_about_expected: "· about {} expected",
    vanity_measuring: "Measuring",
    vanity_searching_for: "Searching for {}",
    vanity_tried: "{} tried · {}{}",
    vanity_stop: "Stop",
    vanity_search_for: "Search for {}",
    vanity_min: "{} min",
    vanity_days: "{} days",
    vault_mb: "{} MB",
    vault_kb: "{} KB",
    vault_remove_the_first: "Remove the {} first",
    vault_release_to_hide: "Release to hide",
    vault_hold_to_show: "Hold to show",
    vault_hold_to_delete: "Hold to delete from vault",
    vault_here_2: "{} here",
    vault_unlock_to_see: "Unlock to see what it holds",
    vault_type_the_passphrase: "Type the passphrase once more to open it",
    vault_import: "Import",
    vault_no_vault_in_files: "No vault in Files",
    vault_remove_the_to: "Remove the {} to unlock",
    vault_new_vault: "New vault",
    vault_locked_not_written: "Locked · not written yet",
    vault_locked_vault: "Locked vault",
    vault_locked: "Locked",
    vault_view_contents: "View contents",
    vault_a_vault_holds: "A vault holds keys, wallets, entries and notes under one to four passphrases, each opening its own contents. A locked vault shows only what its file states: its size and unlock cost. A vault is copied in on {} visit, and is sealed {} again when the session locks with changes in it.",
    vault_remove_the_then_type: "Remove the {}, then type the passphrase",
    vault_no_locked_vault: "No locked vault in Files",
    vault_unlocking: "Unlocking · {}",
    vault_unlock: "Unlock",
    vault_this_computer_free: "This computer: {} free",
    vault_this_computer_memory: "This computer: memory not reported",
    vault_choose_the_vault: "Choose the vault and type a passphrase. Each passphrase opens its own contents, and a vault does not say how many it has. The time is the Argon2id cost the vault was made with, measured on this computer.",
    vault_where_will_you: "Where will you open it?",
    vault_then: "Then: {}",
    vault_home: "Home",
    vault_create_a_vault: "Create a vault",
    vault_a_vault_opens_only: "A vault opens only on a machine with the memory its cost asks for. Choose every kind of machine it will ever be opened on; the cost suggested next fits the one with the least memory.",
    vault_each_guess_at: "Each guess at a passphrase costs this much memory and time. Argon2id works through the memory 1 KiB at a time, once each pass, so a guess is memory in KiB times passes block steps, and the cost adds the base-2 logarithm of that to the passphrase's own bits: 64 MiB with 3 passes is 196,608 steps, ≈17.6 bits, and doubling the memory or the passes adds ≈1. These are estimates, marked ≈: they count work, not how much harder memory is than time for an attacker. A dice passphrase's own bits are exact. Memory is a hard limit: a machine without it cannot open the vault at any speed. The passphrase matters more: one more dice word adds 12.9 bits, and this whole range of cost adds ≈6. The cost cannot be changed later.",
    vault_every_passphrase: "Every passphrase gets the same space, fixed for the life of the vault. 256 KiB holds keys, wallets, notes and about a hundred entries.",
    vault_a_vault_opens: "A vault opens only on a machine with the memory its cost asks for, and the cost cannot be changed later. 512 MiB opens on PCs; 64 MiB opens on a Raspberry Pi as well. Customise chooses the machines, the cost and the space per passphrase one by one.",
    vault_each_passphrase: "Each passphrase opens its own contents, and the file does not say how many there are. They are set now: adding or changing one later means making a new vault and moving the contents across. The passphrase matters more than the cost: six words from the EFF long list are 77.5 bits, out of reach at any cost here, while a short or reused passphrase is not protected at any cost.",
    vault_none_chosen: "None chosen",
    vault_passes_3: "{} · {} · {} passes",
    vault_file_n: "{} · file {}",
    vault_ofv: "{}.ofv · {} {} · {}",
    vault_passphrase_inline: "passphrase",
    vault_passphrases: "passphrases",
    vault_match: "match",
    vault_not_finished: "not finished",
    vault_free: "{} ({} free)",
    vault_memory_not_reported: "Memory not reported",
    vault_this_computer: "This computer",
    vault_choose_memory: "Choose memory and passes",
    vault_suggested: "Suggested",
    vault_fits: "Fits",
    vault_too_large_for: "Too large for {}",
    vault_passes_bits: "{} · {} passes · ≈{} bits",
    vault_memory: "Memory",
    vault_passes_2: "Passes",
    vault_unlock_memory_n: "Unlock memory {}",
    vault_up_to: "{}: up to {}",
    vault_adds_bits_to: "Adds ≈{} bits to every passphrase",
    vault_passes: "{} · {} passes",
    vault_unlocks_in_here: "Unlocks in {} here",
    vault_unlock_time_not: "Unlock time not measured yet",
    vault_here: "{} · {} here",
    vault_this_computer_has: "This computer has {} free",
    vault_customise: "Customise",
    vault_file_n_inline: "file {}",
    vault_continue: "Continue",
    vault_name: "Name",
    vault_what_it_holds: "What it holds, for anyone who sees the {} · leave empty for vault.ofv",
    vault_file_ofv: "File {}.ofv",
    vault_remove_the_then: "Remove the {}, then type the passphrases",
    vault_hide_passphrases: "Hide passphrases",
    vault_show_passphrases: "Show passphrases",
    vault_passphrase_n: "Passphrase {}",
    vault_remove: "Remove",
    vault_generate_with: "Generate with dice",
    vault_or_type_your: "or type your own below",
    vault_matches: "Matches",
    vault_does_not_match: "Does not match yet",
    vault_type_it_then: "Type it, then type it again",
    vault_bits_from_dice: "{} bits from dice + ≈{} from the unlock cost ≈ {} bits",
    vault_typed_its_own: "Typed: its own bits are not measured · the unlock cost adds ≈{}",
    vault_add_another_passphrase: "Add another passphrase",
    vault_roll_dice_for: "Roll {} dice for each word and type the numbers",
    vault_no_word_yet: "No word yet",
    vault_bits_with_the: "{} {} · {} bits · ≈{} with the unlock cost{}",
    vault_dice_for_the_next_word: "· {} of {} dice for the next word",
    vault_word: "word",
    vault_words_inline: "words",
    vault_use_these_words: "Use these words",
    vault_your_choices: "Your choices",
    vault_opens_on: "Opens on",
    vault_unlock_cost: "Unlock cost",
    vault_space_per_passphrase: "Space per passphrase",
    vault_name_and_passphrases: "Name and passphrases",
    vault_not_chosen_yet: "Not chosen yet",
    vault_custom: "Custom",
    vault_this_computer_n: "This computer cannot allocate {}. A vault it could not open is never written.",
    vault_needs_gives_a: "{} needs {}. {} gives a vault {} at most.",
    vault_unlocking_takes: "Unlocking takes {} here.",
    vault_raspberry_pi: "Raspberry Pi unlock times are not measured yet.",
    vault_will_create: "Will create",
    vault_no_passphrase: "No passphrase yet",
    vault_not_measured: "Not measured",
    vault_file: "File",
    vault_size: "Size",
    vault_unlock_memory: "Unlock memory",
    vault_unlock_here: "Unlock here",
    vault_unlock_cost_adds: "Unlock cost adds",
    vault_bits: "≈{} bits",
    vault_weakest_passphrase: "Weakest passphrase",
    vault_strength: "Strength",
    vault_goes_to: "Goes to",
    vault_creating: "Creating · {}",
    vault_create_vault: "Create vault",
    vault_key: "Key",
    vault_words_n: "{} words",
    vault_master_seed: "Master seed",
    vault_chosen_at_unlock_inline: "chosen at unlock",
    vault_totp: "TOTP",
    vault_note_characters: "Note · {} characters",
    vault_recovery_sheet: "Recovery sheet",
    vault_recovery_sheet_n: "Recovery sheet · {}",
    vault_ed25519_does: "Ed25519 · does not expire",
    vault_ed25519_expires: "Ed25519 · expires {}",
    vault_owner_keys: "Owner keys",
    vault_pk_kek_db: "PK, KEK, db",
    vault_record: "Record",
    vault_vaults: "Vaults",
    vault_vault_contents: "Vault contents",
    vault_nothing_in_it: "Nothing in it yet",
    vault_make_keys: "Make keys",
    vault_write_it_to: "Write it to {}",
    vault_save_a_bitcoin: "Save a bitcoin key",
    vault_make_a_gpg_key: "Make a GPG key",
    vault_bitcoin_key: "Bitcoin key",
    vault_wallet: "Wallet",
    vault_entry: "Entry",
    vault_gpg_key: "GPG key",
    vault_secure_boot: "Secure Boot",
    vault_no_vault_is_open: "No vault is open",
    vault_open_unsaved: "Open · {} unsaved {}",
    vault_change: "change",
    vault_changes: "changes",
    vault_rename: "Rename",
    vault_scan_a_code: "Scan a code",
    vault_import_from_files: "Import from Files ({})",
    vault_import_n: "Import {}",
    vault_totp_codes: "TOTP codes",
    vault_none_in_this: "None in this vault",
    vault_secret_values: "Secret values show only while held. Changes stay in this session until it locks; locking seals them, and the next {} visit writes the vault back over its own file.",
    vault_rename_the_vault: "Rename the vault",
    vault_add_a_note: "Add a note",
    vault_edit_the_note: "Edit the note",
    vault_make_a_key: "Make a key",
    vault_make_secure_boot: "Make Secure Boot keys",
    vault_add_an_entry: "Add an entry",
    vault_edit_the_entry: "Edit the entry",
    vault_expires: "Expires",
    vault_making_pk_kek: "Making PK, KEK and db",
    vault_make_the_key: "Make the key",
    vault_make_the_keys: "Make the keys",
    vault_open_3: "Open {}",
    vault_the_backup: "the backup",
    vault_load_with_a_bip: "Load with a BIP-39 passphrase",
    vault_passphrase: "Passphrase",
    vault_opening: "Opening",
    vault_sealing: "Sealing",
    vault_open_2: "Open",
    vault_seal_and_export: "Seal and export",
    vault_load: "Load",
    vault_cancel: "Cancel",
    vault_no_file: "No file {}",
    vault_signed: "Signed",
    vault_sign_1_file: "Sign 1 file",
    vault_sign_files: "Sign {} files",
    vault_back_to_the_key: "Back to the key",
    vault_words: "Words",
    vault_bip_39_passphrase: "BIP-39 passphrase",
    vault_stored: "Stored",
    vault_not_stored: "Not stored",
    vault_chosen_at_unlock: "Chosen at unlock",
    vault_yes: "Yes",
    vault_no: "No",
    vault_in_this_session: "In this session",
    vault_not_loaded: "Not loaded",
    vault_load_into_session: "Load into session",
    vault_load_with_a_passphrase: "Load with a passphrase",
    vault_leave_unchosen: "Leave unchosen at unlock",
    vault_choose_at_unlock: "Choose at unlock",
    vault_open_in_wallets: "Open in Wallets",
    vault_username: "Username",
    vault_password: "Password",
    vault_url: "URL",
    vault_code: "Code",
    vault_set_the_time: "Set the time",
    vault_unreadable: "Unreadable",
    vault_totp_secret: "TOTP secret",
    vault_notes: "Notes",
    vault_export_for_keepass: "Export for KeePass",
    vault_text: "Text",
    vault_edit: "Edit",
    vault_descriptor: "Descriptor",
    vault_note: "Note",
    vault_fingerprint: "Fingerprint",
    vault_signing_subkey: "Signing subkey",
    vault_user_id: "User ID",
    vault_from_does_not: "From {} · does not expire",
    vault_to: "{} to {}",
    vault_valid: "Valid",
    vault_paperkey: "Paperkey",
    vault_renew_for: "Renew for",
    vault_renew: "Renew",
    vault_export_public: "Export public key",
    vault_revocation_certificate: "Revocation certificate",
    vault_show_as_qr: "Show as QR",
    vault_sign_a_file: "Sign a file",
    vault_owner_guid: "Owner GUID",
    vault_enrolment_adds: "Enrolment adds",
    vault_windows_compatible: "Windows-compatible",
    vault_own_keys_only: "Own keys only",
    vault_your_pk_kek_and: "Your PK, KEK and db only. Windows will not start, nor any card or controller whose firmware Microsoft signed",
    vault_yours_with_microsoft: "Yours, with Microsoft's KEK CAs and Windows CAs. Not Microsoft's third-party CA: a card or controller whose firmware it signed may not start",
    vault_enrolment_files: "Enrolment files",
    vault_save_a_key: "Save a key",
    vault_save_a_wallet: "Save a wallet",
    vault_bip_39_passphrase_n: "{} · BIP-39 passphrase",
    vault_save_with_its: "Save with its passphrase",
    vault_every_key_in: "Every key in this session is in the vault",
    vault_save: "Save",
    vault_every_wallet: "Every wallet in this session is in the vault",
    vault_sealed: "Sealed",
    vault_not_sealed: "Not sealed",
    vault_lock_without_sealing: "Lock without sealing",
    vault_keys_not_saved: "Keys not saved",
    vault_wallets_not_saved: "Wallets not saved",
    vault_everything_not: "Everything not in a vault is wiped with the session",
    vault_back_to_the_vault: "Back to the vault",
    vault_lock: "Lock",
    vault_1_year: "1 year",
    vault_2_years: "2 years",
    vault_5_years: "5 years",
    vault_never: "Never",
    vault_sign_n: "Sign {}",
    vault_compare_it_with: "Compare it with the build record whose fingerprint you checked yourself",
    vault_hold_to_sign: "Hold to sign",
    vault_back: "Back",
    vault_sign_or_check: "Sign or check an image",
    vault_no_efi_image: "No EFI image {}",
    vault_sign: "Sign",
    vault_check: "Check",
    vault_back_to_the_keys: "Back to the keys",
    vaults_light: "Light",
    vaults_standard: "Standard",
    vaults_medium: "Medium",
    vaults_strong: "Strong",
    vaults_maximum: "Maximum",
    vaults_pcs_only: "PCs only",
    vaults_pcs_and_a_raspberry: "PCs and a Raspberry Pi",
    vaults_other_pcs: "Other PCs",
    vaults_other_pcs_inline: "other PCs",
    vaults_raspberry_pi_5: "Raspberry Pi 5",
    vaults_pi_5: "Pi 5",
    vaults_raspberry_pi_4: "Raspberry Pi 4",
    vaults_pi_4: "Pi 4",
    vaults_raspberry_pi_3: "Raspberry Pi 3",
    vaults_pi_3: "Pi 3",
    vaults_pi_zero_2_w: "Pi Zero 2 W",
    vaults_bitcoin_keys: "Bitcoin keys",
    vaults_wallets: "Wallets",
    vaults_entries: "Entries",
    vaults_gpg_keys: "GPG keys",
    vaults_secure_boot: "Secure Boot",
    vaults_main: "Main",
    vaults_second: "Second",
    vaults_third: "Third",
    vaults_fourth: "Fourth",
    vaults_aim_for_words: "Aim for {} words or more on this list: the count turns green at {} bits, which this vault counts as strong. The lists and how to roll for them: eff.org/dice",
    vaults_gib: "{} GiB",
    vaults_mib: "{} MiB",
    vaults_kib: "{} KiB",
    vaults_mb: "{} MB",
    vaults_kb: "{} KB",
    vaults_pass: "pass",
    vaults_passes: "passes",
    vaults_about_s: "about {} s",
    vaults_not_measured: "not measured yet",
    vaults_this_computer_2: "This computer",
    vaults_file: "file",
    vaults_files: "files",
    vaults_in_copy_checked: "in {} · copy checked",
    vaults_copy_checked: "copy checked",
    vaults_seed_n: "Seed {}",
    vaults_in: "in {}",
    vaults_in_files: "in Files",
    vaults_nowhere_else: "nowhere else",
    vaults_make_a_vault: "Make a vault",
    vaults_unlock: "Unlock {}",
    vaults_unlock_a_vault_2: "Unlock a vault",
    vaults_make_a_new_wallet: "Make a new wallet",
    vaults_restore_an_existing: "Restore an existing wallet",
    vaults_spend: "Spend",
    vaults_back_up_a_wallet: "Back up a wallet",
    vaults_child_seeds: "Child seeds and passwords",
    vaults_lightning_node: "Lightning node key",
    vaults_import: "Import",
    vaults_tools: "Tools",
    vaults_and_loaded: "{} {} and {} {} loaded",
    vaults_wallet_inline: "wallet",
    vaults_wallets_inline: "wallets",
    vaults_those_entries: "Those entries do not fit in this vault's slot size",
    vaults_added: "{} {} added",
    vaults_entry_inline: "entry",
    vaults_entries_inline: "entries",
    vaults_not_a_totp_setup: "Not a TOTP setup code",
    vaults_type_the_backup: "Type the backup's passphrase",
    vaults_a_passphrase: "A passphrase of 8 characters or more",
    vaults_that_passphrase: "That passphrase does not open this backup",
    vaults_needs_mib_which: "Needs {} MiB, which this computer cannot spare",
    vaults_not_a_backup: "Not a backup this build reads",
    vaults_remove_the_to: "Remove the {} to unlock",
    vaults_type_the_passphrase: "Type the passphrase",
    vaults_needs_this_computer: "Needs {}. This computer has {} free.",
    vaults_unlocked: "{} unlocked",
    vaults_loaded_from: "{} loaded from {}",
    vaults_seed: "Seed",
    vaults_loaded: "{} {} loaded",
    vaults_seed_inline: "seed",
    vaults_seeds: "seeds",
    vaults_entry: "Entry",
    vaults_waits_sealed: "{} waits {}, sealed",
    vaults_that_entry_is: "That entry is too large for a KeePass export",
    vaults_deleted_from: "Deleted from the vault",
    vaults_key_is_loaded: "Key {} is loaded",
    vaults_vault: "Vault",
    vaults_title: "Title",
    vaults_username: "Username",
    vaults_password: "Password",
    vaults_url: "URL",
    vaults_notes: "Notes",
    vaults_totp_secret: "TOTP secret",
    vaults_text: "Text",
    vaults_name: "Name",
    vaults_email: "Email",
    vaults_owner_name: "Owner name",
    vaults_this_computer: "This computer's clock is not known",
    vaults_made_the_certificate: "Made · the certificate and a revocation certificate",
    vaults_the_note_is: "The note is empty",
    vaults_an_entry_needs: "An entry needs a title",
    vaults_not_a_totp_key: "Not a TOTP key",
    vaults_unlock_a_vault: "Unlock a vault first",
    vaults_is_in_the_vault_as_a_note: "{} is in the vault as a note",
    vaults_is_in_the_vault_as_a: "{} is in the vault as a key",
    vaults_is_in_the_vault_as: "{} is in the vault as a wallet",
    vaults_this_key_has: "This key has no words to keep",
    vaults_is_in_the_vault: "{} is in the vault",
    vaults_that_does_not: "That does not fit in this vault's slot size",
    vaults_remove_the_first: "Remove the {} first",
    vaults_finish_step: "Finish step {} first",
    vaults_every_passphrase: "Every passphrase needs a value",
    vaults_each_passphrase: "Each passphrase must be typed the same twice",
    vaults_two_passphrases: "Two passphrases are the same. Each must differ",
    vaults_choose_a_cost: "Choose a cost this computer can allocate",
    vaults_no_randomness: "No randomness from the system yet. Try again",
    vaults_vault_created: "Vault created",
    vaults_never_written: "Never written",
    vaults_on_current: "On {} · current",
    vaults_changed_since: "Changed since written",
    vaults_changes_not_sealed: "Changes not sealed",
    vaults_unchanged: "Unchanged",
    vaults_wallets_n: "{} wallets",
    vaults_key: "key {}",
    vaults_keys_inline: "keys {}",
    vaults_keys: "{} keys",
    vaults_1_entry: "1 entry",
    vaults_entries_n: "{} entries",
    vaults_seen: "seen {}",
    vaults_nothing_in_it: "Nothing in it",
    vaults_place: "Place {}:{}",
    vaults_wallet: "Wallet",
    visit_taken_keep_both_n: "{} · Keep both: {}",
    visit_taken_the_same: "{} · the same vault, updated here",
    visit_taken_another: "{} · another vault · kept, this one as {}",
    visit_taken_already: "Already {}",
    visit_taken_replace: "Replace",
    visit_taken_keep_both: "Keep both",
    visit_taken_cancel: "Cancel",
    wallet_mainnet: "Mainnet",
    wallet_testnet: "Testnet",
    wallet_signet: "Signet",
    wallet_regtest: "Regtest",
    wallet_not_a_valid: "Not a valid seed: {}",
    wallet_is_already_loaded: "{} is already loaded",
    wallet_these_words_give: "These words give {}, not {}",
    wallet_these_words: "These words give {}, not a key of {}",
    wallet_not_a_wallet: "Not a wallet: {}",
    wallet_is_not_the_network: "{} is not {}, the network of the loaded wallets",
    wallet_is_not_a_key: "{} is not a key this session takes",
    wallet_from: "From {}",
    wallet_from_inline: "from {}",
    wallet_a_master_seed: "a master seed is 16 to 64 bytes",
    wallet_legacy: "legacy",
    wallet_nested_segwit: "nested SegWit",
    wallet_native_segwit: "native SegWit",
    wallet_taproot: "Taproot",
    wallet_single_key: "Single key · {}",
    wallet_of_legacy_multisig: "{} of {} · legacy multisig",
    wallet_of_nested_segwit: "{} of {} · nested SegWit multisig",
    wallet_of_native_segwit: "{} of {} · native SegWit multisig",
    wallet_of_taproot_multisig: "{} of {} · Taproot multisig",
    wallet_keys_miniscript: "{} keys · miniscript with conditions",
    wallet_keys_taproot: "{} keys · Taproot key and leaves",
    wallet_of_musig2: "{} of {} · MuSig2",
    wallet_of_frost: "{} of {} · FROST",
    wallet_1_key_silent: "1 key · silent payments",
    wallet_wallet: "Wallet",
    wallet_no_address: "no address: {}",
    wallet_no_key_in_this: "No key in this session signs this transaction",
    wallet_a_json_file: "a .json file with no descriptor",
    wallet_bsms: "BSMS: {}",
    wallet_coldcard_export: "Coldcard export: {}",
    wallet_no_xfp: "no xfp",
    wallet_its_first_address: "its first {} address is not {}",
    wallet_no_single_sig: "no single-sig account",
    wallet_collect: "Collect {}{} {}",
    wallet_more: "more",
    wallet_signature: "signature",
    wallet_signatures: "signatures",
    wallet_not_a_psbt: "Not a PSBT",
    wallet_is_a_psbt_for: "{} is a PSBT for another transaction",
    wallet_cannot_combine: "Cannot combine: {}",
    wallet_a_signature: "A signature in {} does not verify",
    wallet_not_enough_signatures: "Not enough signatures yet",
    wallet_the_script_is: "The script is not satisfied yet: another signature, or a timelock not yet reached",
    wallet_no_such_wallet: "no such wallet",
    wallet_messages_are: "Messages are signed with single-key wallets",
    wallet_the_wallet_has: "the wallet has no key",
    wallet_the_wallet_s_xpub: "the wallet's xpub has no origin",
    wallet_the_wallet_s_key: "the wallet's key is not loaded here",
    wallet_the_wallet_s: "the wallet's xpub has no path",
    wallet_blocks_after: "{} blocks after the coins arrived (about {} days)",
    wallet_days_after_the: "{} days after the coins arrived",
    wallet_after_block: "after block {}",
    wallet_after_time: "after time {}",
    wallet_a_secret_preimage: "a secret preimage",
    wordlist_press_a_word: "Press a word to see it in the list",
    wordlist_bip_39_english: "BIP-39 English",
    wordlist_eff_large: "EFF large",
    wordlist_eff_short_1: "EFF short 1",
    wordlist_eff_short: "EFF short 2",
    wordlist_bits: "Bits",
    wordlist_dice: "Dice",
    wordlist_back: "Back",
    wordlist_words_word: "{} words · word {} of {} marked",
    wordlist_words: "{} words",
    wordlist_type_to_find: "Type to find a word",
    wordlist_find: "Find: {}",
    wordlist_no: "No.",
    wordlist_word: "Word",
    desktop_title: "Faraday · F2 boot stick · F3 stick 2 · F4 pull all",
    desktop_title_keys: "F2 boot stick · F3 stick 2 · F4 pull all",
    desktop_keys_line: "F2 boot stick · F3 stick 2 · F4 pull all · F5 every folder",
    desktop_title_panel: "Faraday · {} in panel · {}",
    hub_window_title: "Faraday Starter",
    hub_disk_number: "Disk {}",
    hub_boot_entry_label: "Faraday stick",
    stick_usb_stick: "USB stick ({})",
    vm_qemu: "QEMU",
    vm_kvm: "KVM",
    vm_vmware: "VMware",
    vm_virtualbox: "VirtualBox",
    vm_hyper_v: "Hyper-V",
    vm_xen: "Xen",
    vm_parallels: "Parallels",
    vm_bochs: "Bochs",
    companion_title: "Faraday Companion",
    companion_home_receive: "Scan a Faraday QR code",
    companion_home_send: "Send to Faraday",
    companion_home_received: "Received",
    companion_home_start: "Start from the stick",
    companion_home_learn: "Learn",
    companion_home_about: "About",
    companion_scan_waiting: "Waiting for camera",
    companion_scan_looking: "Looking for a code",
    companion_scan_no_camera: "No camera",
    companion_scan_part: "{} of {}",
    companion_scan_part_missing: "{} of {} · missing {}",
    companion_scan_not_read: "Code not read",
    companion_scan_nothing_to_paste: "Nothing to paste",
    companion_paste: "Paste",
    companion_kept: "Kept in Received",
    companion_already_kept: "Already in Received",
    companion_open: "Open",
    companion_scan_again: "Scan again",
    companion_name: "Name",
    companion_kind: "Kind",
    companion_size: "Size",
    companion_when: "Kept",
    companion_sha256: "SHA-256",
    companion_kind_psbt: "PSBT",
    companion_kind_transaction: "Transaction",
    companion_kind_description: "Wallet description",
    companion_kind_file: "File",
    companion_kind_text: "Text",
    companion_bytes: "{} B",
    companion_kib: "{} KiB",
    companion_time_unknown: "Not known",
    companion_refuse_words: "Seed words stay on the Faraday device.",
    companion_refuse_seedqr: "A SeedQR stays on the Faraday device.",
    companion_refuse_compact_seedqr: "A CompactSeedQR stays on the Faraday device.",
    companion_refuse_codex32: "A codex32 seed stays on the Faraday device.",
    companion_refuse_slip39: "A SLIP-39 share stays on the Faraday device.",
    companion_refuse_xprv: "An xprv stays on the Faraday device.",
    companion_refuse_wif: "A private key stays on the Faraday device.",
    companion_refuse_ur_seed: "A seed stays on the Faraday device.",
    companion_refuse_ur_bip39: "Seed words stay on the Faraday device.",
    companion_refuse_ur_private_key: "A private key stays on the Faraday device.",
    companion_refuse_backup: "A backup stays on the Faraday device.",
    companion_refuse_keepass: "A KeePass file stays on the Faraday device.",
    companion_refuse_vault: "A vault stays on the Faraday device.",
    companion_refuse_carry: "A threshold carry stays on the Faraday device.",
    companion_refuse_totp: "A TOTP secret stays on the Faraday device.",
    companion_refuse_totp_migration: "Authenticator codes stay on the Faraday device.",
    companion_refuse_entries: "Vault entries stay on the Faraday device.",
    companion_text_title: "Text",
    companion_keep: "Keep",
    companion_address_title: "Address",
    companion_address_bitcoin: "Bitcoin address",
    companion_address_silent: "Silent payment address",
    companion_web_title: "Web address",
    companion_web_not_opened: "Not opened",
    companion_bitlocker_title: "BitLocker recovery key",
    companion_bitlocker_computer: "Computer",
    companion_bitlocker_drive: "Drive",
    companion_done: "Done",
    companion_send_choose: "Choose a file",
    companion_send_nothing: "Nothing in Received",
    companion_send_not_sent: "Not sent",
    companion_send_too_large: "Over 256 KiB",
    companion_send_empty: "Empty",
    companion_send_no_picker: "No file picker",
    companion_send_no_code: "No code holds it",
    companion_send_picked: "picked.{}",
    companion_reason: "Reason",
    companion_animated: "Animated",
    companion_too_dense: "too dense",
    companion_frame_of: "{} of {}",
    companion_rate: "{} frames a second",
    companion_show_as_ur: "Show as UR",
    companion_show_as_bbqr: "Show as BBQr",
    companion_show_as_parts: "Show as numbered parts",
    companion_made_psbt_one: "ur:crypto-psbt · one code",
    companion_made_psbt_frames: "ur:crypto-psbt · {} frames",
    companion_made_text_one: "Text · one code",
    companion_made_key_one: "Key · one code",
    companion_made_account: "ur:crypto-account · one code",
    companion_made_bytes_one: "ur:bytes · one code",
    companion_made_bytes_frames: "ur:bytes · {} frames",
    companion_made_file_one: "File · ur:bytes · one code",
    companion_made_file_frames: "File · ur:bytes · {} frames",
    companion_made_bbqr_one: "BBQr {} · one code",
    companion_made_bbqr_frames: "BBQr {} · {} frames",
    companion_made_parts: "Numbered parts · {} frames",
    companion_received_empty: "Nothing received",
    companion_delete_all: "Delete all",
    companion_unreadable: "Kept files",
    companion_unreadable_value: "Not readable",
    companion_unreadable_delete: "Delete the kept files",
    companion_show_as_qr: "Show as QR",
    companion_save: "Save",
    companion_delete: "Delete",
    companion_saved: "Saved",
    companion_not_saved: "Not saved",
    companion_delete_one: "Delete this file?",
    companion_delete_all_ask: "Delete every received file?",
    companion_delete_store_ask: "Delete the kept files?",
    companion_keep_it: "Keep it",
    companion_files: "Files",
    companion_version: "Version",
    companion_build: "Build",
    companion_certificate: "Signing certificate SHA-256",
    companion_network: "Network access",
    companion_none: "None",
    companion_permissions: "Permissions",
    companion_granted: "Granted",
    companion_not_granted: "Not granted",
    companion_not_reported: "Not reported",
    companion_about_check: "Each release lists this app's file in SHA256SUMS, and the release's OpenPGP key signs that list. The file's SHA-256 is checked against the list, and the list's signature against the key, as the README's Verifying a release says.",
    companion_about_certificate: "The signing certificate's SHA-256 above is the one apksigner verify --print-certs prints for the release's file. The release notes state it.",
    companion_home: "Home",
    companion_on_phone: "On this phone",
    companion_newest: "Newest",
    companion_learn_faraday: "Faraday",
    companion_start_question: "Which computer are you starting?",
    companion_start_windows: "Windows",
    companion_start_linux: "Linux",
    companion_start_computer: "Computer",
    companion_start_change: "Change",
    companion_start_bitlocker: "BitLocker first",
    companion_start_maker: "Maker",
    companion_start_choose_maker: "Choose the maker",
    companion_start_trouble: "If something goes wrong",
    companion_maker_dell: "Dell",
    companion_maker_hp: "HP",
    companion_maker_lenovo: "Lenovo",
    companion_maker_thinkpad: "ThinkPad",
    companion_maker_other: "Other",
    companion_maker_asus: "ASUS",
    companion_maker_acer: "Acer",
    companion_maker_msi: "MSI",
    companion_maker_framework: "Framework",
    companion_maker_surface: "Surface",
    companion_steps_done: "{} of {} done",
    companion_steps_again: "Start again",
    companion_learn_bitcoin: "Bitcoin",
    companion_theme: "Theme",
    companion_check_title: "Checking a release",
    companion_home_descriptions: "Wallet descriptions",
    companion_home_bitlocker: "BitLocker keys",
    companion_unlock_descriptions: "Open Wallet descriptions",
    companion_unlock_keys: "Open BitLocker keys",
    companion_unlock_keep_description: "Keep a wallet description",
    companion_unlock_keep_key: "Keep a BitLocker key",
    companion_unlock_waiting: "Waiting for the phone's unlock",
    companion_description_name: "Wallet {}",
    companion_shared_not_read: "Not read",
    companion_descriptions_add: "Scan or paste a description",
    companion_descriptions_empty: "No wallet descriptions",
    companion_keep_in_descriptions: "Keep in Wallet descriptions",
    companion_share: "Share",
    companion_rename: "Rename",
    companion_delete_description_ask: "Delete this wallet description?",
    companion_delete_key_ask: "Delete this key?",
    companion_delete_kept_store_ask: "Delete the kept descriptions and keys?",
    companion_delete_the_key: "Delete the key",
    companion_script_legacy: "Legacy",
    companion_script_nested: "Nested SegWit",
    companion_script_native: "Native SegWit",
    companion_script_taproot: "Taproot",
    companion_wallet_single: "Single key",
    companion_wallet_multisig: "Multisig",
    companion_wallet_musig: "MuSig2",
    companion_wallet_threshold: "FROST threshold",
    companion_wallet_silent: "Silent payments",
    companion_wallet_miniscript: "Miniscript",
    companion_wallet_tree: "Taproot tree",
    companion_k_of_n: "{} of {}",
    companion_n_keys: "{} keys",
    companion_keys: "Keys",
    companion_no_fingerprint: "None",
    companion_key_n: "Key {}",
    companion_wallet_network: "Network",
    companion_first_address: "First receive address",
    companion_no_address: "None at an index",
    companion_keys_empty: "No BitLocker keys",
    companion_drive_line: "Drive {} · {}",
    companion_hold_to_show: "Hold to show",
    companion_back_in_windows: "Back in Windows",
    companion_key_not_needed: "Not needed if Windows started without asking for it.",
    companion_key_shown_again: "Faraday Starter shows it again the next time Faraday is set up on this computer.",
    companion_closed_title: "Not opened",
    companion_closed_cancelled: "Unlock cancelled",
    companion_closed_no_lock: "Set a screen lock on this phone to keep wallet descriptions and BitLocker keys.",
    companion_delete_kept_store: "Delete the kept descriptions and keys",
    companion_not_description: "Not a wallet description",
};
