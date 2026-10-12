//! Faraday's English wording: every string Faraday's screens, the stick's
//! device check and the hub app show that OpenSigner does not. OpenSigner's
//! own wording stays in `opensigner/opensigner-core/src/strings/en.rs`;
//! Faraday never adds to that file. Same house style as upstream's.

/// Every string Faraday shows that OpenSigner's [`opensigner_core::strings::Strings`]
/// does not carry, grouped by screen.
pub struct Strings {
    /// Power-off sheet on a stick that started with Secure Boot off.
    pub power_secure_boot_off: &'static str,
    // ----- The stick's device check (docs/HUB.md §3.1a), faraday-core -----
    /// Title of the sheet that names what is attached.
    pub devices_title: &'static str,
    /// What an IP-KVM or KVM can do, after its name and a colon.
    pub devices_kvm: &'static str,
    /// What a wireless keyboard or mouse receiver lets happen.
    pub devices_receiver: &'static str,
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
    /// The desktop app's app menu entry's name.
    pub hub_menu_desktop_name: &'static str,
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
    /// A wireless keyboard or mouse receiver (HUB §3.1a), by name.
    pub hub_concern_receiver: &'static str,
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
}

/// The English wording.
pub static EN: Strings = Strings {
    power_secure_boot_off: "Secure Boot is off. Turn it back on before starting Windows.",
    devices_title: "Attached to this computer",
    devices_kvm: "a KVM can show this screen and type on this computer from elsewhere",
    devices_receiver: "a wireless keyboard's keys travel over the air",
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
    hub_item_make: "Make a Faraday stick",
    hub_item_bitlocker: "BitLocker",
    hub_item_start: "Start this computer from the stick",
    hub_item_secure_boot: "Turn Secure Boot back on",
    hub_item_resume: "Resume BitLocker",
    hub_state_no_stick: "No Faraday stick attached",
    hub_state_stick: "Faraday stick attached · {}",
    hub_state_protected: "Protected",
    hub_state_suspended: "Suspended by Faraday",
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
    hub_no_stick_image: "This Faraday app carries no stick image",
    hub_no_pi_image: "This Faraday app carries no Pi image",
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
    hub_dev_image: "Not the Faraday app's own image",
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
    hub_restart: "Restart into firmware settings",
    hub_restart_stick: "Restart from the stick",
    hub_restart_save: "This restarts the computer now. Save your work first.",
    hub_stick_restart_failed: "Restart from the stick did not happen: {}",
    hub_entry_kept: "The firmware's Faraday stick entry remains: {}",
    hub_restart_now: "Restart",
    hub_cancel: "Cancel",
    hub_secure_boot_turn_off: "Turn Secure Boot off",
    hub_secure_boot_already_off: "Secure Boot is already off",
    hub_fast_boot: "Turn Fast Boot off where it hides USB",
    hub_boot_from_stick: "Start from the stick",
    hub_boot_how: "The boot menu key",
    hub_boot_how_windows: "The boot menu key, or Use a device in Advanced startup",
    hub_afterwards: "Turn Secure Boot back on",
    hub_afterwards_windows: "Turn Secure Boot back on before starting Windows",
    hub_on_a_phone: "On a phone",
    hub_install_desktop: "Also install the desktop app, for testing with test seeds only",
    hub_install_now: "Install",
    hub_installing: "Installing",
    hub_installed_in: "Installed in {}",
    hub_installed_version: "Installed version",
    hub_this_version: "This version",
    hub_installed_app: "Faraday",
    hub_installed_desktop: "Desktop app",
    hub_installed_menu: "Menu entry",
    hub_installed_desktop_menu: "Desktop app's menu entry",
    hub_installed_icon: "Icon",
    hub_installed_entry: "Installed apps entry",
    hub_installed_reminder: "Logon reminder",
    hub_not_installed: "Not installed",
    hub_uninstall: "Uninstall",
    hub_uninstall_ask: "Uninstall Faraday?",
    hub_uninstall_keeps: "Removes the files listed. Received files and settings stay.",
    hub_uninstall_suspended: "BitLocker stays suspended until it is resumed.",
    hub_why_not_carried: "This Faraday app does not carry what was confirmed. Nothing was written.",
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
    hub_why_no_desktop: "This Faraday app carries no desktop app",
    hub_why_cache_sum: "The desktop app does not match the manifest: its SHA-256 is {}",
    hub_why_file: "{}: {}",
    hub_why_in_the_way: "{} is another program's, and is in the way",
    hub_why_start: "Starting {} failed: {}",
    hub_why_restart: "Restarting into firmware settings failed: {}",
    hub_why_declined: "Administrator permission was declined. Nothing was changed.",
    hub_why_unsupported: "Not on this computer",
    hub_why_missing: "{} is not installed",
    hub_why_boot_entry: "efibootmgr failed: {}",
    hub_why_no_stick: "No Faraday stick is attached",
    hub_menu_name: "Faraday",
    hub_menu_desktop_name: "Faraday desktop (testing)",
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
    hub_no_payload: "This Faraday app carries no images",
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
    hub_concern_receiver: "{} is a wireless receiver. What is typed travels over the air: use a wired keyboard.",
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
};
