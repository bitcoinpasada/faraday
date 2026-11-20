//! Faraday's English words for the vm names area.

macro_rules! words_vm_names {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
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
    /// The way out of a flow Tools starts (a GPG key, Secure Boot keys),
    /// where nothing else names it: back to Tools.
    backup_flow_leave_tools: "Tools",
    /// A vault of the plan, by number.
    backup_flow_vault_n: "Vault {}",
    /// A place of the plan, by number.
    backup_flow_place_n: "Place {}",
    /// Step 1's card.
    backup_flow_card_where: "Where it will be kept",
    /// Step 3's card.
    backup_flow_card_vaults: "Vaults",
    /// Step 4's card.
    backup_flow_card_make: "Make the key",
    /// Step 5's card.
    backup_flow_card_lock: "Lock",
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
    /// In place of Make the checklist: a seed here past the checklist's
    /// numbers. `{}`: the seed's place in the wallet, then how many seeds
    /// the checklist takes.
    backup_seed_past_checklist: "Seed {} is past the checklist's {} seeds",
    /// The checklist or Lock and write of a backup past its lock and not
    /// done: refused.
    backup_flow_past_lock: "This backup is past its lock; finish the writes, then Back up again",
    /// Lock and write of a plan that keeps a secret file on the stick that
    /// carries the public files: refused. The file, then the stick.
    backup_flow_public_secret: "This plan keeps {} on {}, the stick that carries the public files",
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
    /// Step 3, under a vault's row: the file tried is another vault.
    backup_flow_not_this_vault: "{} is not {}",
    /// Above the tap: the vault is written beside the files a stick holds
    /// that no vault of the plan is known to be.
    backup_flow_goes_beside: "{} goes beside these",
    /// A button under it: this file is the stick's copy of the vault.
    backup_flow_use_file: "Use {} for {}",
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
    /// A device's pill word on a card or stick.
    backup_flow_pill_card_or_stick: "Card or stick",
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
    backup_flow_not_checked: "The copy on {} failed its check. Nothing was counted",
    /// A file written to a stick did not read back, twice
    /// (`docs/BACKUP-FLOW.md` §17.12 item 10): the file, the stick.
    backup_flow_no_read_back: "{} did not read back from {}; try another stick",
    /// Problem in the backup flow: the name-taken sheet was answered with
    /// Cancel. `{}`: the vault's file name.
    backup_flow_name_taken: "{} was not written. Its name is taken",
    /// Title of the sheet for a vault whose name a file on the stick
    /// has. `{}`: the vault's file name, `app.medium.on_the()` ("on the stick").
    vault_taken_title: "{} is already {}",
    /// Why a vault was not written when the stick was pulled with its name
    /// still asked about. `{}`: `self.medium.noun()`.
    visit_taken_pulled: "the {} was pulled",
    /// Body of that sheet: the file's vault is not known.
    vault_taken_unknown: "Faraday cannot tell which vault this file is. Overwriting destroys it, even if it is another vault.",
    /// Body of that sheet: the file is a copy of a vault the session
    /// knows. `{}`: that vault's name.
    vault_taken_other: "This file is {}, another vault. Overwriting destroys it.",
    /// Body of that sheet: the file could not be read.
    vault_taken_unreadable: "This file could not be read. It stays as it is.",
    /// Hold in that sheet. `{}`: the file's name.
    vault_taken_overwrite: "Hold: overwrite {}",
    /// Button in that sheet. `{}`: the free name.
    vault_taken_beside: "Save as {}",
    /// A stick's written copy was deleted or the stick wiped.
    backup_flow_first_gone_toast: "A written copy was deleted",
    /// Done's result.
    backup_flow_done: "Written",
    /// The button that leaves the flow once its files are written.
    backup_flow_done_button: "Done",
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
    backup_flow_create_vault: "Make {}",
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
    /// A stick of Bring in the sticks with nothing in yet.
    backup_flow_figure_waiting: "Waiting",
    /// A stick of Bring in the sticks being read.
    backup_flow_figure_reading: "Reading",
    /// A stick of Bring in the sticks read and usable.
    backup_flow_figure_checked: "Checked",
    /// The vaults a stick of Bring in the sticks will hold. `{}`: their
    /// names.
    backup_flow_figure_holds: "Holds {}",
    /// The file chosen for a vault already on a stick. `{}`: the vault,
    /// then the file.
    backup_flow_figure_file: "{}: {}",
    /// Two parts of the line under a checked figure's name (the stick's
    /// maker and model, its size, its name), the first then the next.
    backup_flow_figure_about: "{} · {}",
    /// A stick of Bring in the sticks put off to be checked at the write.
    backup_flow_figure_later: "Checked when written",
    /// A checked stick's sheet: the label over the maker and model.
    backup_flow_sheet_make: "Maker and model",
    /// A checked stick's sheet: the label over the size.
    backup_flow_sheet_size: "Size",
    /// A checked stick's sheet: the label over the stick's name.
    backup_flow_sheet_name: "Name",
    /// A checked stick's sheet: the label over the FAT volume serial.
    backup_flow_sheet_serial: "Volume serial",
    /// A checked stick's sheet: the label over the free room.
    backup_flow_sheet_free: "Free room",
    /// A checked stick's sheet: the label over its files.
    backup_flow_sheet_files: "Files",
    /// A checked stick's sheet: the label over its vaults.
    backup_flow_sheet_vaults: "Vaults",
    /// A checked stick's sheet: its one button.
    backup_flow_sheet_close: "Close",
    /// A stick's button, and its ⋮ row, to check it at the write.
    backup_flow_later_button: "Check it later",
    /// The dialog's title. `{}`: the stick's name.
    backup_flow_later_title: "Check {} later?",
    /// The dialog's first statement. `{}`: the medium's noun, twice.
    backup_flow_later_when: "A {} is not read while a key or a vault is open, so a {} not read now is read at the write step, after the lock.",
    /// The dialog's second statement. `{}`: the medium's noun.
    backup_flow_later_sealed: "The vaults are made now either way. Until the {} is written, each is kept sealed in this device's memory only.",
    /// The dialog's third statement. `{}`: the medium's noun, then the
    /// room the vaults need.
    backup_flow_later_checks: "At the write step the {} must be FAT32, writable, not a boot partition, and have room for its vaults ({}).",
    /// The dialog's fourth statement. `{}`: the medium's noun.
    backup_flow_later_fails: "If it fails then, another {} that passes can take its place. With none at hand, turning this device off or restarting it loses those vaults, and any key kept only in them.",
    /// The dialog's way out.
    backup_flow_later_cancel: "Cancel",
    /// Summary of step 2.
    backup_flow_read_count: "{} of {} read",
    /// Bring in the sticks closed with none read: each is read on Write
    /// the sticks, as it is written.
    backup_flow_read_at_write: "Read as written",
    /// Bring in the sticks closed with some read: the others are read on
    /// Write the sticks. `{}`: the sticks read, then all of them.
    backup_flow_read_rest_at_write: "{} of {} read, the rest as written",
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
    /// Toast: the vault keeps the wallet's shares plan for another key order that the copy would drop.
    backup_flow_copy_plan_kept: "The vault keeps a shares plan for the wallet with its keys in another order, or its answers are damaged or missing, and the copy would drop its shares. Copy into another vault",
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
    /// Summary of a done step 3: one vault.
    backup_flow_vaults_one: "1 vault",
    /// A vault of the plan, new: its name first.
    backup_flow_vault_new: "{} · new",
    /// A vault of the plan that exists already: its name first.
    backup_flow_vault_existing: "{} · an existing vault",
    /// Toast: a flow in the kept state was corrupt or of another version.
    backup_flow_dropped: "A kept backup did not read and was dropped",
    /// A stick whose room cannot be asked for: it cannot be read.
    backup_flow_unreadable: "This {} cannot be read. Use another {}",
    /// Toast: a lock's save failed with a backup waiting, so the process did
    /// not restart.
    toast_lock_unrestarted: "Locked without a restart. No stick can be written until the next lock",
    /// Button on Where: the place also keeps this vault's stick.
    backup_flow_also: "Also {}",
    /// Button on Where: this place's stick carries the key's public files.
    backup_flow_public_here: "Public files here",
    /// Toast: a key is not made with a stick in.
    backup_flow_pull_to_unlock_toast: "Pull the stick first",
    /// Fact line under the plan's checks: every secret of the wallet is at one place. The place's name.
    plan_all_at_one_place: "Every secret of this wallet is at {}. Separate places are strongly recommended.",
    /// Fact line under the plan's checks, for a single key with no passphrase.
    plan_passphrase_apart: "Keeping the passphrase in a different place from the seed is recommended.",
    /// Button beside that line.
    plan_add_passphrase: "Add a passphrase",
    /// Templates item of the checklist: print them from Faraday Starter. The count, the words, the copy form.
    item_templates_starter: "Print {} from Faraday Starter: Blank templates, {} words, {}",
    /// Templates item: the copy is the words alone.
    item_templates_words: "words",
    /// Templates item: the copy is the SeedQR alone.
    item_templates_seedqr: "SeedQR",
    /// Templates item: the copy is the words and the SeedQR.
    item_templates_both: "both",
    /// Foot of the Templates item, a press.
    item_action_done: "Done",
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
    /// A vault chip's menu title and its first item, naming the sticks.
    backup_flow_layout_put_on: "Put on…",
    /// A stick named by its vault before the keys exist: "the stick for
    /// Vault 1".
    backup_flow_stick_for_vault: "the {} for {}",
    /// The flow's first card for a wallet whose keys exist: the plan.
    backup_flow_card_plan: "The plan",
    /// Button of a wallet's Make card.
    backup_flow_make_the_keys: "Make the keys",
    /// Make card of a round for an added vault: the seeds kept only on paper.
    backup_flow_on_paper_only: "{} is kept only on paper",
    /// Button: type the seed back from its copy.
    backup_flow_add_from_copy: "Add the key from its copy",
    /// Toast: a change would take a written stick off the plan.
    backup_flow_written_stays: "A stick already written stays on the plan",
    /// Toast: the round has no copy of the descriptor to put in.
    backup_flow_no_description: "No copy of the descriptor is here to put in",
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
    /// A stick zone's label. `{}`: its pill's number, one numbering
    /// across the places, from 2 (the boot stick is 1).
    board_stick: "Stick {}",
    /// A stick zone's label where the medium is a card or stick (the Pi).
    /// `{}`: its pill's number, as `board_stick`.
    board_card_or_stick: "Card or stick {}",
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
    /// The list's group of the descriptor and its shares.
    board_wallet_description: "Descriptor",
    /// The whole descriptor's chip.
    board_whole: "Whole",
    /// A share's chip. `{}`: its number.
    board_share: "Share {}",
    /// The list's group of vaults.
    board_vaults: "Vaults",
    /// The list's button that adds a vault, always a new one, made in the
    /// checklist (`docs/BACKUP-FLOW.md` §17.21 item 4).
    board_add_vault: "+ Make a new vault",
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
    /// Under a place's header, before it is taken off: what it holds. The
    /// place, what it holds.
    board_place_holds: "{} holds {}",
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
    /// A box in the ⋮ menu: a stick's files. `{}`: the stick, "Stick 2".
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
    /// A place's spend tag when the search over the other places stopped
    /// at its bound.
    board_spend_not_worked_out: "Spend: not worked out, too many places to search",
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
    /// The sheet over a drop that brings a place nearer a spend (§17.22). `{}`: the place.
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
    /// Its line: the place keeps the whole descriptor.
    board_sees_whole: "The whole descriptor is kept there.",
    /// Its line: what at the place together makes the descriptor. `{}`:
    /// "Share 1 and seed 9a6a2580".
    board_sees_together: "{} together make the whole descriptor.",
    /// Its line: one share alone makes the descriptor. `{}`: "Share 1".
    board_sees_alone: "{} makes the whole descriptor.",
    /// One share in its line. `{}`: its number.
    board_sees_share: "Share {}",
    /// Shares in its line. `{}`: "1, 2", the last.
    board_sees_shares: "Shares {} and {}",
    /// Two lists joined. `{}`: the first, the last.
    board_sees_both: "{} and {}",
    /// Keeps the drop.
    board_keep_button: "Keep it",
    /// Undoes the drop.
    board_take_it_off: "Take it off",
    /// Under the list's Descriptor: hands a share to every place
    /// (`docs/BACKUP-FLOW.md` §17.19 item 1).
    board_partials_add: "Add partial descriptors to this backup",
    /// The same button while they are on: takes every share off.
    board_partials_remove: "Remove partial descriptors",
    /// In the button's place, for a wallet of more than one key whose
    /// split sheets would not rebuild it (`docs/WALLETS.md` §5).
    board_partials_not_offered: "Partial descriptors: only for a sortedmulti or tr(NUMS, sortedmulti_a) wallet",
    /// The same, for a multisig two of whose keys come from one seed.
    board_partials_one_seed: "Partial descriptors: not for a wallet with two keys of one seed",
    /// The sheet after adding them: its title.
    board_partials_each: "Each place now holds a part of the descriptor.",
    /// Its line where the whole descriptor is still kept. `{}`: the
    /// places, "Place 1, Place 2 and Place 3".
    board_partials_whole: "The whole descriptor is still kept at {}, so the balance can be seen there. Take it off any place where the balance should stay hidden.",
    /// Takes the whole descriptor off every place.
    board_take_it_off_every: "Take it off every place",
    /// A warning: nothing of the descriptor is kept.
    board_desc_nothing: "The descriptor cannot be put back together: add the whole descriptor",
    /// A warning: what is kept does not put it back together. `{}`: what
    /// is kept ("Share 1 twice and Share 2"), what would complete it
    /// ("Share 3").
    board_desc_missing: "The descriptor cannot be put back together: the places hold {}; add {} or the whole descriptor",
    /// The same, every share kept somewhere. `{}`: what is kept.
    board_desc_kept_add: "The descriptor cannot be put back together: the places hold {}; add the whole descriptor",
    /// A share kept twice, in those warnings. `{}`: its number.
    board_share_twice: "Share {} twice",
    /// A share kept more often. `{}`: its number, how often.
    board_share_times: "Share {} {} times",
    /// A warning: a place sees the balance from its share and seeds.
    /// `{}`: the place, what there makes the descriptor.
    board_sees_warn: "{} can see the balance. {}",
    /// A warning: a place keeps no seed. `{}`: the place.
    board_no_seed: "{} holds no seed",
    /// "Any one place lost" with a single place (§17.19).
    plan_no_single_place: "No: a single place",
    /// Why "Any one place lost" reads No when the descriptor goes. `{}`:
    /// the place.
    plan_without_no_description: "Without {}, the rest cannot put the descriptor back together",
        }
    };
}
