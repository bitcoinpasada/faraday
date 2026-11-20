//! Faraday's English words for the vault area.

macro_rules! words_vault {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- A vault's key and wallet, shown and saved (docs/BACKUP-FLOW.md §17.7), faraday-core -----
    /// A key record's SeedQR, shown by its eye.
    vault_show_seedqr: "Show SeedQR",
    /// A vault record's button and the popup's title: its files to the
    /// Files, For the stick.
    vault_save_to_a_stick: "Save to a stick",
    /// The popup's line for a key record's files, before anything is made.
    vault_anyone_who_finds: "Anyone who finds this stick has this seed",
    // ----- vault (faraday-core/src/vault/screen.rs) -----
    /// Text in `stick_field`. `{}`: `medium.noun()`.
    vault_remove_the_first: "Remove the {} first",
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
    vault_a_vault_holds: "A vault holds keys, wallets, entries and notes under its passphrase. It is copied in on {} visit and sealed {} again when the session locks with changes.",
    /// Text in `unlock_compact`. `{}`: `app.medium.noun()`.
    vault_remove_the_then_type: "Remove the {}, then type the passphrase",
    /// Line in `unlock_compact`.
    vault_no_locked_vault: "No locked vault in Files",
    /// Text in `unlock_compact`. `{}`: `t`.
    vault_unlocking: "Unlocking · {}",
    /// Button in `unlock_compact`.
    vault_unlock: "Unlock",
    /// Text button under Unlock's passphrase.
    vault_add_key_file: "Add a key file",
    /// Head of the dialog that lists the files a key file is chosen from.
    vault_choose_key_file: "Choose a key file",
    /// Button of that dialog.
    vault_choose: "Choose",
    /// That dialog when Files holds no file that is not a vault.
    vault_no_key_files: "Files holds no file that is not a vault",
    /// Row of that dialog: Faraday makes the key file at Create.
    vault_make_key_file: "Make a key file",
    /// Row of that dialog: a file in Files is the key file.
    vault_choose_a_file: "Choose a file",
    /// Body of that dialog.
    vault_key_file_warning: "If this file is lost or changed, the vault cannot be opened. Keep it apart from the vault's file.",
    /// Create a vault's closed passphrase line, after the match.
    vault_with_key_file: "key file",
    /// Chip on Create a vault for a key file Faraday makes. `{}`: the vault's file name without `.kdbx`.
    vault_keyx_made_at_create: "{}.keyx · made at Create",
    /// Create a vault: the key file could not be put in Files, For the stick, so no vault was made.
    vault_key_file_not_kept: "The key file could not be put in Files, For the stick. No vault was made.",
    /// Under Unlock: the key file read as damaged. `{}`: its name.
    vault_key_file_damaged: "{} is damaged as a key file.",
    /// Under Unlock: the key file is empty. `{}`: its name.
    vault_key_file_empty: "{} is empty.",
    /// Under Unlock: the key file is a vault. `{}`: its name.
    vault_key_file_vault: "{} is a vault, not a key file.",
    /// Under Unlock: the chosen key file is no longer in Files. `{}`: its name.
    vault_key_file_gone: "{} is no longer in Files.",
    /// Text in `unlock_compact`. `{}`: `mib_text(m)`.
    vault_this_computer_free: "This computer: {} free",
    /// Text in `unlock_compact`.
    vault_this_computer_memory: "This computer: memory not reported",
    /// Text in `unlock_compact`.
    vault_choose_the_vault: "Choose the vault and type its passphrase. The time is the Argon2id cost the vault was made with, measured on this computer.",
    /// Text in `VSTEPS`.
    vault_where_will_you: "Where will you open it?",
    /// Text in `VSTEPS`.
    vault_opens_on_step: "Opens on",
    /// Text in `create`. `{}`: `app.back_name(s)`.
    vault_then: "Then: {}",
    /// Text in `create`.
    vault_home: "Home",
    /// Text in `create`.
    vault_create_a_vault: "Make a new vault",
    /// Under Create a vault's passphrase fields.
    vault_passphrase_no_reset: "A vault cannot be opened without this passphrase. Nobody can reset it.",
    /// Text in `create_summaries`.
    vault_none_chosen: "None chosen",
    /// Text. `{}`: `name`, `mib_text(mem)`, `passes`.
    vault_cost_row: "{} · {} · {} passes",
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
    vault_passes_bits: "{} passes · ≈{} bits",
    /// Line in `create_body`.
    vault_memory: "Memory",
    /// Text in `create_body`. `{}`: `mib_text(mem)`.
    vault_unlock_memory_n: "Unlock memory {}",
    /// Text in `create_body`. `{}`: `weakest`, `mib_text(limit)`.
    vault_memory_limit: "{}: up to {}",
    /// Text in `create_body`. `{}`: `vault::cost_bits(mem, passes)`.
    vault_cost_bits: "Each guess costs about as much as 2^{} plain hashes",
    /// Text in `create_body`. `{}`: `app.vaults.time_text(mem, passes)`.
    vault_unlocks_in_here: "Unlocks in {} here",
    /// Text in `create_body`. `{}`: `cost`, `app.vaults.time_text(mem, passes)`.
    vault_here: "{} · {} here",
    /// Text in `create_body`. `{}`: `mib_text(f)`.
    vault_this_computer_has: "This computer has {} free",
    /// Button in `create_body`.
    vault_customize: "Customize",
    /// Button in `create_body`.
    vault_continue: "Continue",
    /// Line in `create_body`.
    vault_name: "Name",
    /// Text in `create_body`. `{}`: `app.medium.noun()`.
    vault_what_it_holds: "What it holds, for anyone who sees the {} · leave empty for vault.kdbx",
    /// Text in `create_body`. `{}`: `app.medium.noun()`.
    vault_remove_the_then: "Remove the {}, then type the passphrase",
    /// Text in `create_body`.
    vault_hide_passphrases: "Hide passphrase",
    /// Text in `create_body`.
    vault_show_passphrases: "Show passphrase",
    /// Button in `create_body`.
    vault_generate_dice: "Generate with dice",
    /// Line in `create_body`.
    vault_or_type_your: "or type your own below",
    /// Line in `create_body`, in Practice, once the dice made a
    /// passphrase (`docs/DEMO.md` §5.2).
    vault_demo_uses_public: "Practice puts faraday in place of these words",
    /// Text in `create_body`. `{}`: `own`, `cost`, `own + cost`.
    vault_bits_from_dice: "{} bits from dice + ≈{} from the unlock cost ≈ {} bits",
    /// Text in `create_body`. `{}`: `cost`.
    vault_typed_its_own: "Typed: its own bits are not measured · each guess costs about as much as 2^{} plain hashes",
    /// Text in `dice_panel`. `{}`: `list.dice_per_word()`.
    vault_dice_per_word: "Roll {} dice for each word and type the numbers",
    /// Line in `dice_panel`.
    vault_no_word_yet: "No word yet",
    /// Text in `dice_panel`. `{}`: `words.len()`, `if words.len() == 1 { FEN.vault_word …`, `own`, `own + vault::cost_bits(mem, passes)`, `if left > 0 { text!(" · {left} of {…`.
    vault_dice_strength: "{} {} · {} bits · ≈{} with the unlock cost{}",
    /// Text in `dice_panel`, after a space. `{}`: `left`, `per`.
    vault_dice_for_the_next_word: "· {} of {} dice for the next word",
    /// Text in `dice_panel`.
    vault_word: "word",
    /// Text in `dice_panel`.
    vault_words_inline: "words",
    /// Button in `dice_panel`.
    vault_use_these_words: "Use these words",
    /// Text in the vault's rows of Your choices.
    vault_opens_on_label: "Opens on",
    /// Text in the vault's rows of Your choices.
    vault_unlock_cost: "Unlock cost",
    /// Text in the vault's rows of Your choices.
    vault_name_and_passphrases: "Name and passphrase",
    /// Text in `create_summary`.
    vault_custom: "Custom",
    /// Text in `create_summary`. `{}`: `mib_text(mem)`.
    vault_this_computer_n: "This computer cannot allocate {}. A vault it could not open is never written.",
    /// Text in `create_summary`. `{}`: `name`, `mib_text(mem)`, `weakest`, `mib_text(limit)`.
    vault_memory_mismatch: "{} needs {}. {} gives a vault {} at most.",
    /// Text in `create_summary`. `{}`: `app.vaults.time_text(mem, passes)`.
    vault_unlocking_takes: "Unlocking takes {} here.",
    /// Text in `create_summary`: the estimate for a Raspberry Pi 3B+
    /// (`docs/VAULT.md`, the reference table).
    vault_raspberry_pi: "On a Raspberry Pi 3B+, about 2.5 s at 64 MiB and 20 s at 512 MiB, 3 passes.",
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
    vault_strength: "Strength",
    /// Text in `create_summary`.
    vault_goes_to_label: "Goes to",
    /// Text in `create_button`. `{}`: `app.vaults.time_text(mem, passes)`.
    vault_creating: "Creating · {}",
    /// Button in `create_button`.
    vault_create_vault: "Make a new vault",
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
    /// The badge on a counter-based code in Codes.
    codes_hotp: "HOTP",
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
    /// The items column's search bar's placeholder, on Wallets (`vault/search_screen.rs`).
    vault_search_wallets: "Search wallets",
    /// The search bar's placeholder, on Entries.
    vault_search_entries: "Search entries",
    /// The search bar's placeholder, on GPG keys.
    vault_search_gpg_keys: "Search GPG keys",
    /// Line in place of the items when no title holds what the search bar holds.
    vault_search_none: "No titles match",
    /// Tooltip on a search bar's ×, which empties it.
    vault_search_clear: "Clear search",
    /// Text in `contents_compact`. `{}`: `app.medium.noun()`.
    vault_secret_values: "A secret value shows after a press on its eye and hides at the next press or when the page is left. Changes stay in this session until it locks; locking seals them, and the next {} visit writes the vault back over its own file.",
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
    /// Button on the open vault's page and head of its dialog.
    vault_change_passphrase: "Change passphrase",
    /// Field in Change passphrase.
    vault_new_passphrase: "New passphrase",
    /// Field in Change passphrase.
    vault_passphrase_again: "Passphrase again",
    /// Row of the open vault's Memory dialog: the step the next seal writes.
    vault_next_seal: "Next seal",
    /// Toast after Memory.
    vault_memory_next_seal: "Takes effect at the next seal. Copies sealed before keep their step.",
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
    /// The Load sheet's button once something is ticked. `{}`: what it
    /// loads, "2 wallets and 1 key".
    load_sheet_load_count: "Load {}",
    /// Two counts joined, "2 wallets and 1 key". `{}`: each.
    load_sheet_and: "{} and {}",
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
    vault_own_keys_only_warning: "Your PK, KEK and db only. Windows will not start, nor a computer whose graphics card, disk controller or network card firmware Microsoft signed. Restore Factory Keys in the BIOS undoes it",
    /// Text in `detail`.
    vault_yours_with_microsoft: "Yours, with Microsoft's KEK CAs and Windows CAs, not its third-party CA: a computer whose graphics card, disk controller or network card firmware Microsoft signed under it may not start. Restore Factory Keys in the BIOS undoes it",
    /// Button in `detail`.
    vault_hold_for_enrolment_files: "Hold for enrolment files",
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
    /// Toast in `vault_lock_one` and the Lock sheet's line. `{}`: the
    /// vault's name, the bytes its changes are over the most a vault holds.
    vault_cannot_seal: "{} cannot be sealed: its changes are {} bytes over the most a vault holds",
    /// The head, for a file Faraday sealed. `{}`: the size its length
    /// shows, which the vault holds to within 32 KiB.
    vault_holds_up_to: "Holds up to {}",
    /// Button in `lock_ask_sheet`, in place of Lock while the system has given
    /// no randomness or a vault cannot be sealed: the changes are lost.
    vault_lock_without_sealing: "Lock without sealing",
    /// Line of `lock_ask_sheet` when memory is short for vaults with changes.
    /// `{}`: their names, what memory needs (`vaults_needs_this_computer`).
    vault_lock_short: "{}: memory is short. {}",
    /// Line on the Lock sheet. `{}`: the vault's name.
    vault_lock_unwritable: "{}: cannot be written as it is",
    /// Line of `lock_ask_sheet` for a vault with an item moved to another
    /// vault the sheet names. `{}`: its name, that vault's name.
    vault_lock_moved: "{}: its item moved to {} is not saved",
    /// Button in `lock_ask_sheet` when memory is short for vaults with
    /// changes: the vaults named are closed and their changes are lost.
    vault_lock_without_saving: "Lock without saving",
    /// Button in `lock_ask_sheet`, with a vault open.
    vault_back_to_the_vault: "Back to the vault",
    /// Button in `lock_ask_sheet` with no vault open: closes the sheet.
    lock_ask_cancel: "Cancel",
    /// The QR sheet's speeds, in frames a second, slowest first.
    qr_speed_2: "2/s",
    /// The QR sheet's middle speed.
    qr_speed_3: "3/s",
    /// The QR sheet's fastest speed.
    qr_speed_5: "5/s",
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
    /// Error in Create a vault.
    vault_cost_not_written: "This cost cannot be written",
    // ----- vaults (faraday-core/src/vault/mod.rs) -----
    /// Text in `PRESETS`.
    vaults_light: "Light",
    /// Text in `PRESETS`.
    vaults_standard: "Standard",
    /// Text in `PRESETS`.
    vaults_medium: "Medium",
    /// Text in `PRESETS`.
    vaults_strong: "Strong",
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
    vaults_gib: "{} GiB",
    /// Text in `size_text`. `{}`: `slot / 1_048_576`.
    vaults_mib: "{} MiB",
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
    vaults_make_a_vault: "Make a new vault",
    /// Text in `vault_way`. `{}`: `files[k].name`.
    vaults_unlock: "Unlock {}",
    /// Text in `vault_way`.
    vaults_unlock_way: "Unlock a vault",
    /// Text in `back_name`.
    vaults_make_a_new_wallet: "Make a new wallet",
    /// Text in `back_name`.
    vaults_restore_an_existing: "Restore an existing wallet",
    /// Text in `back_name`.
    vaults_spend: "Spend from a backup",
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
    /// Toast in `vault_import_entries`, text in `vault_scanned_entry`.
    /// `{}`: the bytes the entries take, the bytes left under the ceiling.
    vaults_those_entries: "Those entries take {} bytes; this vault has {} left",
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
    /// Toast for an edit not made when memory is short and the vault is not
    /// known, so no figure is.
    vaults_memory_short: "Memory is short",
    /// Text in `vault_unlock_check`. `{}`: `mib_text(need)`, `mib_text(free)`.
    vaults_needs_this_computer: "Needs {}. This computer has {} free.",
    /// Text in `vault_unlock_check`, free memory not reported. `{}`:
    /// Light's memory, then the vault's.
    vaults_memory_unknown_unlock: "This computer does not say how much memory it has free. It unlocks vaults of {}; this one takes {}.",
    /// Toast in `vault_unlock`. `{}`: `self.vaults.open[v].name`.
    vaults_unlocked: "{} unlocked",
    /// Toast in `unlock_landed`. `{}`: `what`, `from`.
    vaults_loaded_note: "{} loaded from {}",
    /// Text in `vault_rows`.
    vaults_seed: "Seed",
    /// Toast after a vault load that left a wallet out. `{}`: the counts
    /// toast, the wallet's name, the reason.
    vaults_loaded_refused: "{}. Not loaded, {}: {}",
    /// Toast after a vault load that left a wallet out and brought in
    /// nothing else. `{}`: the wallet's name, the reason.
    vaults_refused: "Not loaded, {}: {}",
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
    /// Text. `{}`: `vault::vault_stem(&c.name.text)`, `if ok { FEN.vault_match } else { FEN.…`.
    vault_kdbx: "{}.kdbx · passphrase {}",
    /// Text beside the name field. `{}`: `vault::vault_stem(&c.name.text)`.
    vault_file_kdbx: "File {}.kdbx",
    // ----- A vault refused (faraday-core/src/vault/errors.rs) -----
    /// Under Unlock: the file is not KeePass.
    vault_error_not_kdbx: "Not a KeePass file",
    /// Under Unlock: KeePass 3 or older.
    vault_error_older: "A KeePass file of an older version",
    /// Under Unlock. `{}`: the version, as `4.2`.
    vault_error_newer: "A KeePass file of version {}, which this build does not read",
    /// Under Unlock. `{}`: the field's number.
    vault_error_unknown_field: "A header field this build does not know: {}",
    /// Under Unlock: Twofish.
    vault_error_twofish: "Saved in Twofish, which this build does not read",
    /// Under Unlock: a cipher other than AES-256, ChaCha20 and Twofish.
    vault_error_other_cipher: "Saved in a cipher this build does not read",
    /// Under Unlock: AES-KDF.
    vault_error_aes_kdf: "Saved with AES-KDF, which this build does not read",
    /// Under Unlock: a key derivation other than Argon2.
    vault_error_other_kdf: "Saved with a key derivation this build does not read",
    /// Under Unlock: a compression other than none and gzip.
    vault_error_compression: "Saved with a compression this build does not read",
    /// Under Unlock: Salsa20 inner stream.
    vault_error_salsa20: "Saved with Salsa20, which this build does not read",
    /// Under Unlock: an inner stream other than ChaCha20.
    vault_error_other_inner: "Saved with an inner stream this build does not read",
    /// Under Unlock: the unlock cost is outside what is read.
    vault_error_cost: "Its unlock cost is outside what Faraday reads",
    /// Under Unlock. `{}`: the MiB it needs.
    vault_error_memory: "Needs {} MiB, which this computer cannot spare",
    /// Under Unlock: the wrong passphrase, tried alone.
    vault_error_wrong: "Wrong passphrase, or this file also needs a key file.",
    /// Under Unlock: the wrong passphrase or key file.
    vault_error_wrong_key_file: "Wrong passphrase or key file.",
    /// Under Unlock and Create: an empty passphrase.
    vault_error_empty_passphrase: "The passphrase is empty",
    /// Under Unlock: damage found before any run.
    vault_error_damaged: "The file is damaged",
    /// Under Unlock: more than Faraday opens.
    vault_error_too_large: "The file is larger than Faraday opens",
    /// Under Unlock: the XML document holds what cannot be carried.
    vault_error_contents: "The contents cannot be read here",
    /// Under Unlock and Lock: the contents are more than the largest vault holds.
    vault_error_full: "The contents are more than a vault can hold",
    /// A toast at Lock, Create a vault and the KeePass export: the file made did not read back, so it was not given.
    vault_error_read_back: "The vault file was not made, because it did not read back. Nothing was lost.",
    /// Under Lock: a change that cannot be written.
    vault_error_unwritable: "That change cannot be written to this vault",
    /// Under Lock: a vault opened to read only.
    vault_error_read_only: "This vault is open to read only",
    // ----- Group names in a vault (faraday-core/src/vault/mod.rs) -----
    /// A KeePass group.
    vault_group_keys: "Bitcoin keys",
    /// A KeePass group.
    vault_group_wallets: "Wallets",
    /// A KeePass group.
    vault_group_notes: "Notes",
    /// A KeePass group.
    vault_group_sheets: "Recovery sheets",
    /// A KeePass group.
    vault_group_gpg: "GPG keys",
    /// A KeePass group.
    vault_group_secure_boot: "Secure Boot",
    /// A KeePass group.
    vault_group_wallet_shares: "Wallet shares",
    /// A KeePass group.
    vault_group_passphrases: "Passphrases",
    /// A KeePass group.
    vault_group_shares: "Codex32 shares",
    /// Title of a key with no label. `{}`: its fingerprint.
    vault_title_key: "Key {fingerprint}",
    /// Title of a note.
    vault_title_note: "Note",
    /// Title of a passphrase. `{for}` stands for the fingerprint it is for.
    vault_title_passphrase: "Passphrase for {for}",
    /// Title of a codex32 share. `{for}` stands for the fingerprint it is for.
    vault_title_share: "Codex32 share for {for}",
    /// Title of Secure Boot keys.
    vault_title_secure_boot: "Secure Boot keys",
    // ----- What is open to read only, kept apart, or not read -----
    /// The head: a newer Faraday made it.
    vault_read_only_newer: "Made by a newer version of Faraday: open to read only",
    /// The head: its database settings are ambiguous.
    vault_read_only_settings: "Its database settings cannot be read unambiguously: open to read only",
    /// The head: the vault was saved by another program.
    vault_saved_by_other: "Saved by another program · {}",
    /// The head. `{}`: the cipher, key derivation and compression.
    vault_other_settings: "Saved in {}",
    /// A cipher, key derivation or compression in the other-settings line.
    vault_setting_aes256: "AES-256",
    /// As above.
    vault_setting_chacha20: "ChaCha20",
    /// As above.
    vault_setting_argon2d: "Argon2d",
    /// As above.
    vault_setting_argon2id: "Argon2id",
    /// As above.
    vault_setting_gzip: "gzip",
    /// As above.
    vault_setting_uncompressed: "uncompressed",
    /// The version in the other-settings line, for a file of minor version
    /// 4.1. `{}`: the minor version.
    vault_setting_version: "KDBX 4.{}",
    /// A file's size in KiB, in the unpadded-size line. `{}`: the number.
    vault_kib: "{} KiB",
    /// The head and Unlock: a file Faraday never sealed.
    vault_not_sealed_yet: "Not sealed by Faraday yet",
    /// Unlock refused. `{}`: the vault, the file it is open from.
    vault_already_open: "{} is open already, from {}",
    /// Section title. `{}`: how many items.
    vault_not_read: "Not read here · {}",
    /// Row title of an item without one.
    vault_no_title: "No title",
    /// Toast at unlock: one item.
    vault_not_read_toast_one: "1 item in {} could not be read",
    /// Toast at unlock. `{}`: how many, the vault.
    vault_not_read_toast: "{} items in {} could not be read",
    /// Label of a passphrase choice on the Load sheet. `{}`: the fingerprint of the key it makes.
    vault_related_passphrase: "Passphrase for key {}",
    /// A passphrase choice on the Load sheet when several vaults are open. `{}`: the label, the vault.
    vault_related_in: "{} · {}",
    /// Line of a codex32 share record that another share of its threshold, identifier, index and key is held with. `{}`: the share's index.
    vault_related_share: "Also saved: another share {} of this set",
    /// Title of the dialog before a second share of a set is saved.
    vault_save_also_head: "Save another?",
    /// Line of that dialog. `{}`: the vault.
    vault_holds_share: "A share of this set is already in {}",
    /// Button of that dialog.
    vault_save_also_continue: "Continue",
    /// Button of that dialog.
    vault_save_also_cancel: "Cancel",
    /// Button on an item's page, and title of the sheet it opens.
    vault_copy_to: "Copy to another vault…",
    /// Title of that sheet.
    vault_copy_title: "Copy to another vault",
    /// Reason a locked vault on that sheet is dimmed: nothing to unlock it from.
    vault_copy_no_file: "Its file is not in Files",
    /// Row of that sheet: Create a vault, the item in it from its first seal.
    vault_copy_new: "New vault with this item",
    /// Reason a waiting copy or move is not made: the original changed.
    vault_copy_changed: "The original changed.",
    /// Reason a waiting copy or move is not made: another sheet was open
    /// when the vault opened.
    vault_copy_sheet_up: "Another sheet was open.",
    /// Toast when a copy waiting for an unlock cannot be made. `{}`: the
    /// vault, then the reason.
    vault_copy_refused: "Nothing was copied to {}. {}",
    /// Toast when a move waiting for an unlock cannot be made. `{}`: the
    /// vault, then the reason.
    vault_move_refused: "Nothing was moved to {}. {}",
    /// Reason a vault on that sheet is dimmed: it holds the item already.
    vault_copy_holds: "Already holds this item",
    /// Toast when a key is saved to a vault that holds it already. `{}`:
    /// the vault.
    vault_holds_key: "{} already holds this key",
    /// Tick on that sheet for a seed that has a codex32 set.
    vault_copy_keep_codex32: "Keep its codex32 set",
    /// Tick on that sheet for a seed that has a BIP-85 origin.
    vault_copy_keep_child: "Keep its BIP-85 origin",
    /// Button of that sheet.
    vault_copy_cancel: "Cancel",
    /// Title of the dialog before a copy of an entry with data Faraday does not read.
    vault_copy_unread_head: "Copy this entry?",
    /// Line of that dialog. `{}`: the vault the entry is copied from, where that data stays.
    vault_copy_unread: "Its attachments and other data saved by a KeePass app stay in {}.",
    /// Button of that dialog.
    vault_copy_go: "Copy",
    /// Toast after a copy. `{}`: the vault it went to.
    vault_copied_to: "Copied to {}",
    /// Button on an item's page, and title of the sheet it opens.
    vault_move_to: "Move to another vault…",
    /// Title of that sheet.
    vault_move_title: "Move to another vault",
    /// Button on a seed's page that has a codex32 set, and title of its dialog.
    vault_trim_codex32: "Remove codex32 set",
    /// Button on a seed's page that has a BIP-85 origin, and title of its dialog.
    vault_trim_child: "Remove BIP-85 origin",
    /// Line of the dialog before the codex32 set is removed from a seed.
    /// `{}`: the vault.
    vault_trim_codex32_line: "The codex32 set is removed from this seed in {} at the next seal. Its words and its other data stay. Copies of {} sealed before still hold the set.",
    /// The same for the BIP-85 origin. `{}`: the vault.
    vault_trim_child_line: "The BIP-85 origin is removed from this seed in {} at the next seal. Its words and its other data stay. Copies of {} sealed before still hold the origin.",
    /// Button of those dialogs that removes.
    vault_trim_go: "Remove",
    /// Toast after the codex32 set is removed from a seed.
    vault_trimmed_codex32: "Codex32 set removed",
    /// Toast after the BIP-85 origin is removed from a seed.
    vault_trimmed_child: "BIP-85 origin removed",
    /// Toast when Remove does nothing because the vault changed under the dialog.
    vault_trim_refused: "Nothing was removed. The vault changed.",
    /// Toast after an entry is added.
    vault_entry_added: "Entry added",
    /// Toast after an entry is changed.
    vault_entry_saved: "Entry saved",
    /// Toast after a note is added.
    vault_note_added: "Note added",
    /// Toast after a note is changed.
    vault_note_saved: "Note saved",
    /// Toast after a vault is renamed. `{}`: the new name.
    vault_renamed_to: "Renamed to {}",
    /// Toast after a vault's name is cleared.
    vault_name_removed: "Name removed",
    /// Button on a loaded seed's page: Explore on that key.
    vault_open_the_key: "Open the key",
    /// Button on a loaded seed's page: the backup of the wallet it belongs to.
    vault_back_it_up: "Back it up",
    /// Button on a wallet's page: its backup checklist.
    vault_backup_checklist: "Backup checklist",
    /// Item of the open vault's ⋮ and head of its dialog.
    vault_key_file: "Key file",
    /// Row of that dialog where the vault has a key file or one waits.
    vault_remove_key_file: "Remove key file",
    /// Line of that dialog. `{}`: `vault_next_seal_none` or `vault_next_seal_a_key_file`.
    vault_next_seal_key_file: "Next seal: {}",
    /// What the next seal writes when it writes no key file.
    vault_next_seal_none: "none",
    /// What the next seal writes when it writes a key file.
    vault_next_seal_a_key_file: "a key file",
    /// Toast after a key file is made, chosen or removed on the open vault.
    vault_key_file_next_seal: "Takes effect at the next seal. Copies sealed before keep the key file they had.",
    /// The open vault's key file could not be put in Files, For the stick, so none was made.
    vault_key_file_not_kept_open: "The key file could not be put in Files, For the stick. The vault keeps the key file it had.",
    /// Line under the open vault's name: the next seal needs a key file.
    vault_needs_key_file: "Needs a key file",
    /// Title of the dialog before a move.
    vault_move_head: "Move this item?",
    /// Line of that dialog. `{}`: the vault it goes to, the vault it is in,
    /// the same vault again.
    vault_move_line: "The item goes to {} and will be gone from {}. Copies of {} sealed before still hold it.",
    /// Second line of that dialog for an entry with data Faraday does not
    /// read. `{}`: the vault it is in.
    vault_move_unread: "Its attachments and other data saved by a KeePass app do not go with it and are gone from {} too.",
    /// Line of that dialog for a seed whose codex32 set is not kept. `{}`:
    /// the vault it is in.
    vault_move_no_codex32: "Its codex32 set does not go with it and is gone from {} too.",
    /// Line of that dialog for a seed whose BIP-85 origin is not kept. `{}`:
    /// the vault it is in.
    vault_move_no_child: "Its BIP-85 origin does not go with it and is gone from {} too.",
    /// Button of that dialog.
    vault_move_go: "Move",
    /// Reason a vault on the Move sheet is dimmed: the item would wait for
    /// a seal of this vault and this vault for the item. `{}`: the vault the
    /// item is in.
    vault_move_ring: "Lock {} first",
    /// Toast after a move. `{}`: the vault it went to.
    vault_moved_to: "Moved to {}",
    /// Toast when a vault is locked alone while an item moved out of it
    /// is not yet sealed in its target. `{}`: the target.
    vault_moved_lock_first: "Lock {} first, which holds an item moved from this vault",
    /// Fault of an item.
    vault_fault_unknown_kind: "Kind not known to this build",
    /// Fault of an item.
    vault_fault_unknown_field: "Field not known to this build",
    /// Fault of an item.
    vault_fault_missing: "A required field is missing",
    /// Fault of an item.
    vault_fault_value: "A value in a form this build does not read",
    /// Fault of an item.
    vault_fault_twice: "Written twice",
    /// Fault of an item.
    vault_fault_format: "A format this build does not read",
    /// Fault of an item.
    vault_fault_same_uuid: "Shares its UUID with another item",
    /// Fault of an item.
    vault_fault_same_field: "Two fields of one name",
    /// Fault of an item.
    vault_fault_ambiguous: "Cannot be told apart from another item",
    /// A wallet item that no wallet rule reads.
    vault_fault_wallet: "Not a wallet this build reads",
    /// The wallet fault, then the refusal's reason.
    vault_fault_wallet_why: "{}: {}",
    // ----- vault seal count (faraday-core/src/vault/seals.rs) -----
    /// Text in `times`: a count of one.
    vault_once: "once",
    /// Text in `times`. `{}`: the count.
    vault_times: "{} times",
    /// Text in `seals_line`. `{}`: `times(count)`.
    vault_seals: "Sealed {}",
    /// Text in `seals_line`. `{}`: the date and time.
    vault_sealed_last: "last {}",
    /// Text in `seals_line`.
    vault_sealed_no_time: "last time not recorded",
    /// Text in `seals_line`. `{}`: day, month name, year, time of day.
    vault_sealed_date: "{} {} {}, {}",
    /// Text in `seals_line`: the twelve months' short names, split at spaces.
    vault_months: "Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec",
    /// Text in `seals_line`. `{}`: the count, the time sealed.
    vault_seals_joined: "{} · {}",
    /// Text in `SealWarn::line`. `{}`: `times(mine)`, `times(seen)`.
    vault_seals_older: "An older copy: sealed {}. A copy sealed {} was open since power-on.",
    /// Text in `SealWarn::line`. `{}`: `times(mine)`, the plan's vault, `times(seen)`.
    vault_seals_older_plan: "An older copy: sealed {}. {}'s backup plan records a copy sealed {}.",
    /// Text in `SealWarn::line`. `{}`: `times(count)`.
    vault_seals_same_count: "Another copy was sealed {} too, with other changes.",
    /// Head of `older_copy`, the dialog an unlock of an older copy stops at.
    vault_older_head: "Open this copy?",
    /// Action in `older_copy`: the copy closes, nothing of it read.
    vault_older_back: "Keep locked",
    /// Action in `older_copy`: the unlock goes on.
    vault_older_open: "Open",
        }
    };
}
