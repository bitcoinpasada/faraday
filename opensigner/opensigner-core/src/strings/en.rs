//! English wording. One struct, one static; a second language is one more
//! file like this one.
//!
//! House style: labels of one to three words, sentences of at most twelve
//! words, no metaphors, no reassurance, no hint under a field that
//! explains itself. A warning states the fact and the consequence in two
//! short sentences.

/// Every string the application can show, grouped by screen.
pub struct Strings {
    // ----- Actions and words that appear on many screens -----
    /// Move to the next step.
    pub action_continue: &'static str,
    /// Leave a screen that has nothing to confirm.
    pub action_done: &'static str,
    /// Discard what was entered and begin again.
    pub action_start_over: &'static str,
    /// Try the same thing again.
    pub action_again: &'static str,
    /// Title of the screen after wipe and exit.
    pub settings_ended_title: &'static str,
    /// Row value on that screen: the keys are gone.
    pub settings_ended_keys: &'static str,
    /// Caption after one Back where nothing is behind (Home, the lock
    /// screen, the stored key's pad): what a second Back does.
    pub leave_again: &'static str,
    /// Title of the terminal screen after leaving by Back.
    pub leave_title: &'static str,
    /// Row value on that screen: the keys left memory, and a key kept on
    /// the device is still there.
    pub leave_ended_keys: &'static str,
    /// Caption inside a masked secret panel.
    pub secret_reveal: &'static str,
    /// Caption inside the same panel while it is held open. The line
    /// stays so the panel keeps its height on reveal.
    pub secret_held: &'static str,
    /// Caption while the app bar's eye is showing a transcription
    /// panel: `{}` is the seconds left before it masks again.
    pub secret_showing: &'static str,
    /// Save data to a file.
    pub action_save_file: &'static str,
    /// Show something as a QR code.
    pub action_show_qr: &'static str,
    /// Row: extend a list that has no end of its own.
    pub action_more: &'static str,
    /// Placeholder where a value is not known.
    pub value_none: &'static str,
    /// Duration in seconds: `{} s`.
    pub duration_seconds: &'static str,
    /// Duration in minutes: `{} min`.
    pub duration_minutes: &'static str,
    /// Script type: pay to public key hash.
    pub script_legacy: &'static str,
    /// Script type: SegWit wrapped in P2SH.
    pub script_nested: &'static str,
    /// Script type: native SegWit.
    pub script_segwit: &'static str,
    /// Script type: Taproot.
    pub script_taproot: &'static str,
    /// Script type: Taproot spent through its script tree.
    pub script_taproot_path: &'static str,
    /// Script type: a `wsh` or `sh(wsh)` script whose spend paths the
    /// script itself states.
    pub script_miniscript: &'static str,
    /// Script type: a multisig, `Multisig {} of {}`.
    pub script_multisig: &'static str,
    /// Script type: one this build does not recognise.
    pub script_unknown: &'static str,
    /// Yes.
    pub value_yes: &'static str,
    /// No.
    pub value_no: &'static str,
    /// Neither: the device does not say.
    pub value_unknown: &'static str,
    /// Why a dimmed row is dimmed: there is no camera to read noise from.
    pub reason_needs_camera: &'static str,
    /// Why a dimmed row is dimmed: it needs a secure element.
    pub reason_needs_tier_b: &'static str,
    /// Why a dimmed row is dimmed: it works on a loaded key only.
    pub reason_needs_key: &'static str,
    /// Row label: a derivation path.
    pub row_path: &'static str,
    /// Row label: an address.
    pub row_address: &'static str,
    /// Row label: what a camera read, on the Result behind a QR that could
    /// not be used.
    pub row_qr: &'static str,
    /// Row label: a value in hexadecimal.
    pub row_hex: &'static str,
    /// Row label: how many bytes something is.
    pub row_bytes: &'static str,
    /// Row label: whether a key is one of the loaded ones.
    pub row_mine: &'static str,
    /// Row label: how many keys something acts on.
    pub row_keys: &'static str,
    /// Value row: the script type a screen derives with.
    pub script_type_row: &'static str,
    /// Title of the script-type choice.
    pub script_type_title: &'static str,

    // ----- Home (UX.md §4) -----
    /// Title of the hub.
    pub home_title: &'static str,
    /// The `wide` sidebar's entry for the list of wallets.
    pub home_name: &'static str,
    /// Action: open the scanner.
    pub home_scan: &'static str,
    /// Status badge: this device holds a MuSig2 secret nonce (§4.1).
    pub status_session_open: &'static str,
    /// The action that leaves the Explore workspace's word entry.
    pub home_explore: &'static str,
    /// Tile: the standalone calculators.
    pub home_tools: &'static str,
    /// Tile: settings.
    pub home_settings: &'static str,
    /// Empty state: load a key.
    pub home_load_key: &'static str,
    /// Empty state: create a key.
    pub home_create_key: &'static str,
    /// Tile: the reading section, which does not exist yet.
    pub home_learn: &'static str,

    // ----- Keys and Wallets (`docs/PLANNING.md` §16.104) -----
    /// Tile and title: the wallets registered on this device.
    pub home_wallets: &'static str,
    /// Tile and title: the keys loaded on this device.
    pub home_keys: &'static str,
    /// Keys: the bottom action, and the title of the menu it opens.
    pub keys_add: &'static str,
    /// Wallets: the bottom action, the empty state's first row, and the
    /// title of the menu it opens.
    pub wallets_add: &'static str,
    /// The wizard's kind Choice: a wallet over one key.
    pub add_single: &'static str,
    /// The same Choice: a wallet of several keys and a quorum.
    pub add_multisig: &'static str,
    /// The same Choice: BIP 387's tapscript multisig.
    pub add_taproot_multisig: &'static str,
    /// The same Choice: a wallet whose second path opens after a wait.
    pub add_recovery: &'static str,
    /// A key's page: the row that says what the key is made of.
    pub key_made_of: &'static str,
    /// Its value for a key of words alone: `{} words`.
    pub key_made_words: &'static str,
    /// Its value for a key of words and a passphrase:
    /// `{} words and a passphrase`.
    pub key_made_passphrase: &'static str,
    /// Its value for a BIP-85 child.
    pub key_made_child: &'static str,
    /// Its value for a key read from SLIP-39 shares.
    pub key_made_slip39: &'static str,
    /// Its value for a key read from SLIP-39 shares under a passphrase.
    pub key_made_slip39_passphrase: &'static str,
    /// Its value for a key read from a codex32 string.
    pub key_made_codex32: &'static str,
    /// A wallet's Keys review: the dimmed reason on a member this
    /// device has no key for.
    pub key_not_loaded: &'static str,

    // ----- Load wizard (UX.md §7.2) -----
    /// Title of the source step.
    pub load_source_title: &'static str,
    /// Source: type the words.
    pub load_source_type: &'static str,
    /// Source: scan a QR.
    pub load_source_scan: &'static str,
    /// Source: word numbers.
    pub load_source_numbers: &'static str,
    /// Source: hex entropy.
    pub load_source_hex: &'static str,
    /// Source: the secure element.
    pub load_source_element: &'static str,
    /// Source: an encrypted backup, which is read rather than typed.
    pub load_source_backup: &'static str,
    /// Source: the shares of a SLIP-39 backup.
    pub load_source_slip39: &'static str,
    /// Source: a codex32 string, or the shares of one.
    pub load_source_codex32: &'static str,
    /// Title of the codex32 entry, for a secret or the first string.
    pub load_codex32_title: &'static str,
    /// Its title for the strings after the first: `Share {}`.
    pub load_codex32_share_title: &'static str,
    /// Record label on the accepted result: the strings of the set.
    pub load_codex32_shares: &'static str,
    /// Its value: `{} of {} needed`.
    pub load_codex32_shares_value: &'static str,
    /// The name of the share being typed, which the word step's title
    /// carries before the word number: `Share {}`.
    pub load_share_word_title: &'static str,
    /// Title of the result after a share that belongs with the set.
    pub load_share_accepted_title: &'static str,
    /// Title of the result after a share that does not.
    pub load_share_refused_title: &'static str,
    /// The result line once the shares in hand open the key.
    pub load_share_enough: &'static str,
    /// The result line while they do not.
    pub load_share_more: &'static str,
    /// Why a share was refused when the set is already as large as this
    /// device holds.
    pub load_share_too_many: &'static str,
    /// Record label: the groups of the backup.
    pub load_share_groups: &'static str,
    /// Its value: `{} of {} needed`.
    pub load_share_groups_value: &'static str,
    /// Record label for one group: `Group {}`.
    pub load_share_group: &'static str,
    /// Its value: `{} of {} shares`.
    pub load_share_group_value: &'static str,
    /// Title of the word-count step.
    pub load_count_title: &'static str,
    /// Title of the language step.
    pub load_language_title: &'static str,
    /// Record label: which wordlist the words come from.
    pub load_language_row: &'static str,
    /// Why a source is dimmed: the build runs in a browser, whose
    /// generator this device cannot vouch for at all.
    pub reason_browser: &'static str,
    /// Title of the word step: `Word {} of {}`.
    pub load_word_title: &'static str,
    /// Inline error when no word matches the letters typed.
    pub load_word_no_match: &'static str,
    /// Record label: how many words the key has.
    pub load_words_row: &'static str,
    /// Title of the checksum step.
    pub load_checksum_title: &'static str,
    /// The checksum holds.
    pub load_checksum_valid: &'static str,
    /// The checksum fails.
    pub load_checksum_failed: &'static str,
    /// Record label on a failed checksum: the word most likely mistyped.
    pub load_suspect_label: &'static str,
    /// The suspect word as a value: `word {}`.
    pub load_suspect_word: &'static str,
    /// Row and button that jumps to a word: `Fix word {}`.
    pub load_fix_word: &'static str,

    // ----- Passphrase, fingerprint confirmation and session PIN -----
    /// Title of the passphrase offer.
    pub passphrase_offer_title: &'static str,
    /// Row that declines a passphrase.
    pub passphrase_none: &'static str,
    /// Row that starts passphrase entry.
    pub passphrase_add: &'static str,
    /// Title of the passphrase entry step.
    pub passphrase_title: &'static str,
    /// Title of the with/without comparison.
    pub passphrase_which_title: &'static str,
    /// Row label: the key without the passphrase.
    pub passphrase_without: &'static str,
    /// Row label: the key with the passphrase.
    pub passphrase_with: &'static str,
    /// Title of the confirmation step.
    pub confirm_title: &'static str,
    /// Record label: the key's fingerprint.
    pub confirm_key: &'static str,
    /// Record label: the chain the key is for.
    pub confirm_network: &'static str,
    /// The button that adds the key.
    pub confirm_add: &'static str,
    /// Title of the set-PIN step.
    pub pin_set_title: &'static str,
    /// Inline error under the field after the repeat did not match.
    pub pin_mismatch: &'static str,
    /// Title of the repeat-PIN step.
    pub pin_repeat_title: &'static str,

    // ----- Key detail and its screens (UX.md §7.2) -----
    /// Title: `Key {}`, the fingerprint.
    pub detail_title: &'static str,
    /// Row: the address explorer.
    pub detail_addresses: &'static str,
    /// Row: the coordinator export.
    pub detail_export: &'static str,
    /// Row: the backup menu.
    pub detail_backup: &'static str,
    /// Row and title: the passphrase over this key's words.
    pub detail_passphrase: &'static str,
    /// Row and title: open the passphrase key of this key's words.
    pub detail_open_passphrase: &'static str,
    /// Row and title: open a BIP-85 child seed of this key.
    pub detail_open_child: &'static str,
    /// Title of the child's index entry.
    pub open_child_index: &'static str,
    /// Caption under the index field: the indices there are.
    pub open_child_index_range: &'static str,
    /// On the passphrase step, when the key it opens is not the member
    /// the wallet's row named: `{}` is the fingerprint this passphrase
    /// gives and `{}` the one the wallet asked for.
    pub open_wrong_passphrase: &'static str,
    /// The same on the child's index step.
    pub open_wrong_child: &'static str,
    /// Title of the Result after a passphrase key is opened.
    pub opened_title: &'static str,
    /// Title of the same Result after a BIP-85 child seed is opened.
    pub opened_child_title: &'static str,
    /// Result: the opened key is loaded.
    pub opened_result: &'static str,
    /// Action on that Result: go to the opened key's wallet page.
    pub opened_open: &'static str,
    /// Title of the Result after the first key a person creates.
    pub created_title: &'static str,
    /// That Result: the key is loaded.
    pub created_result: &'static str,
    /// Badge: the backup quiz has been passed.
    pub detail_backup_verified: &'static str,
    /// Badge: the backup quiz has not been passed.
    pub detail_backup_unverified: &'static str,
    /// Title.
    pub addresses_title: &'static str,
    /// Segment: the receive chain.
    pub addresses_receive: &'static str,
    /// Segment: the change chain.
    pub addresses_change: &'static str,
    /// The receive chain, inside a sentence.
    pub addresses_receive_low: &'static str,
    /// The change chain, inside a sentence.
    pub addresses_change_low: &'static str,
    /// Label over one address: the script type, the chain and the
    /// index, as Verify already words it.
    pub addresses_index: &'static str,
    /// One row of the address list: the chain and the index.
    pub addresses_row: &'static str,
    /// Shown while the account is being derived.
    pub addresses_deriving: &'static str,
    /// Title.
    pub export_title: &'static str,
    /// Value row: which form the key is exported in.
    pub export_format_row: &'static str,
    /// Title of the format choice.
    pub export_format_title: &'static str,
    /// Format: the output descriptor.
    pub export_descriptor: &'static str,
    /// Format: the account extended public key.
    pub export_xpub: &'static str,
    /// Format: the SLIP-132 form of the account key.
    pub export_slip132: &'static str,
    /// The string does not fit in one QR.
    pub export_too_long: &'static str,
    /// Forget: the row for the key's own wallet, which goes with it.
    pub forget_wallet_row: &'static str,
    /// Forget: the wallet's state afterwards.
    pub forget_wallet: &'static str,
    /// Forget: the state of a policy this key is a cosigner of, as the
    /// value beside that wallet's own label.
    pub forget_policy: &'static str,
    /// Title.
    pub forget_title: &'static str,
    /// The hold-to-confirm button.
    pub forget_hold: &'static str,

    // ----- Backup: words, numbers, codes and the quiz -----
    /// Title of the backup menu.
    pub backup_title: &'static str,
    /// Row: the words themselves.
    pub backup_words: &'static str,
    /// Row: the words as a QR code.
    pub backup_seedqr: &'static str,
    /// Row: the entropy as a QR code.
    pub backup_compact: &'static str,
    /// Row: the quiz.
    pub backup_verify: &'static str,
    /// Row: the shares of a SLIP-39 backup, the one backup a key with
    /// no words has.
    pub backup_slip39: &'static str,
    /// Row: the key written as a codex32 string, or as shares of one.
    pub backup_codex32: &'static str,
    /// Dimmed reason on a backup row when the key no longer holds its
    /// words.
    pub backup_no_words: &'static str,
    /// Row, title of the flow that makes one, and title of its QR: the
    /// key's words encrypted under a passphrase.
    pub backup_encrypted: &'static str,
    /// The result line of the screen that made one, under its own
    /// title: what happened, not the title again (§2.1).
    pub backup_encrypted_result: &'static str,
    /// Entry title: the passphrase the backup is encrypted under.
    pub backup_pass_title: &'static str,
    /// Entry title of the second typing.
    pub backup_pass_repeat: &'static str,
    /// Inline error under the field when the two typings differed.
    pub backup_pass_mismatch: &'static str,
    /// Inline error under the field when the passphrase did not open a
    /// scanned backup.
    pub backup_wrong_pass: &'static str,
    /// Row on the backup result: what the bytes hold.
    pub backup_inside_row: &'static str,
    /// Its value on a key loaded from its own words.
    pub backup_inside_words: &'static str,
    /// Its value on a passphrase key, whose backup is the words the
    /// passphrase is applied to.
    pub backup_inside_parent: &'static str,
    /// Inline error and scan reason: the file asks for more Argon2id
    /// memory than this device can allocate. `{}` is the MiB.
    pub backup_needs_memory: &'static str,
    /// Row on the backup result and on a sealed note: the Argon2id
    /// memory the file was made at. `{}` is the MiB.
    pub backup_memory_row: &'static str,
    /// Its value, and the value of every memory row: `{}` MiB.
    pub backup_memory_value: &'static str,
    /// Its value on a key with no words, whose backup is its master
    /// seed.
    pub backup_inside_seed: &'static str,
    /// Row and Choice title: the Argon2id memory an encrypted backup is
    /// made at.
    pub settings_backup_memory: &'static str,
    /// Second line of the cheapest cost: it opens on any device.
    pub settings_backup_memory_anywhere: &'static str,
    /// Second line of the cost this device recommends.
    pub settings_backup_memory_recommended: &'static str,
    /// Second line of the dearest cost.
    pub settings_backup_memory_slowest: &'static str,
    /// Tools row, and title of every screen of the Lightning node key
    /// tool.
    pub tools_lightning: &'static str,
    /// That tool's Choice title.
    pub lightning_from_title: &'static str,
    /// Its first row: twenty-four words are typed.
    pub lightning_from_aezeed: &'static str,
    /// Its second row: a key already loaded.
    pub lightning_from_key: &'static str,
    /// Why that row is dimmed with no such key loaded.
    pub lightning_no_words: &'static str,
    /// Result row: the node's public key.
    pub lightning_node_key: &'static str,
    /// Result row: the cipher seed's own version.
    pub lightning_version: &'static str,
    /// Result row: the birthday.
    pub lightning_birthday: &'static str,
    /// The birthday's value: the day, and the count the seed holds.
    pub lightning_birthday_value: &'static str,
    /// Result row: the key the node key came from.
    pub lightning_key_row: &'static str,
    /// Result row: which software derives a node key this way.
    pub lightning_software_row: &'static str,
    /// Its value for a cipher seed.
    pub lightning_software_lnd: &'static str,
    /// Its value for a loaded key.
    pub lightning_software_ldk: &'static str,
    /// The Result headline for a cipher seed that opened.
    pub lightning_read: &'static str,
    /// The Result headline for a loaded key.
    pub lightning_derived: &'static str,
    /// The Result headline before anything has been worked out.
    pub lightning_no_answer: &'static str,
    /// The Result headline for a wrong passphrase.
    pub lightning_bad_passphrase: &'static str,
    /// The Result headline for words that do not check out.
    pub lightning_bad_checksum: &'static str,
    /// The Result headline for a version this device does not read.
    pub lightning_bad_version: &'static str,
    /// The Result action that opens the Secret screen.
    pub lightning_show_secret: &'static str,
    /// The Secret screen's title over the node private key.
    pub lightning_secret_title: &'static str,
    /// Its title over the cipher seed's entropy.
    pub lightning_entropy_title: &'static str,
    /// The key page's row that grinds a vanity address.
    pub key_vanity_row: &'static str,
    /// The Choice that asks which dial the counter turns.
    pub vanity_how_title: &'static str,
    /// Its first row: the passphrase, with a counter appended.
    pub vanity_how_passphrase: &'static str,
    /// Its second row: the account index.
    pub vanity_how_account: &'static str,
    /// Second line of the passphrase row: what one candidate costs.
    pub vanity_how_passphrase_under: &'static str,
    /// Second line of the account row.
    pub vanity_how_account_under: &'static str,
    /// What both rows carry on a device whose hardware makes a grind
    /// slow.
    pub vanity_slow: &'static str,
    /// Why the passphrase row is dimmed on a key with no words.
    pub vanity_no_words: &'static str,
    /// Why it is dimmed on a key that already carries a passphrase.
    pub vanity_has_passphrase: &'static str,
    /// The Entry that takes the prefix.
    pub vanity_prefix_title: &'static str,
    /// The screen the grind runs on.
    pub vanity_running_title: &'static str,
    /// Its row for the candidates tested so far.
    pub vanity_tried_row: &'static str,
    /// Its row for the measured rate.
    pub vanity_rate_row: &'static str,
    /// That rate, in candidates a second.
    pub vanity_rate_value: &'static str,
    /// Its row for how long the prefix is expected to take.
    pub vanity_expected_row: &'static str,
    /// The expected time in seconds.
    pub vanity_seconds: &'static str,
    /// In minutes.
    pub vanity_minutes: &'static str,
    /// In hours.
    pub vanity_hours: &'static str,
    /// In days.
    pub vanity_days: &'static str,
    /// Longer than a year, which is no longer a number worth stating.
    pub vanity_over_a_year: &'static str,
    /// Stop, which ends the run.
    pub vanity_stop: &'static str,
    /// The Result headline for an address that was found.
    pub vanity_found_title: &'static str,
    /// The headline where the counter ran out before anything matched.
    pub vanity_exhausted_title: &'static str,
    /// The Result row for the account a find is at.
    pub vanity_account_row: &'static str,
    /// The Result row for the counter a find is at.
    pub vanity_counter_row: &'static str,
    /// The Result action that takes the find.
    pub vanity_use: &'static str,
    /// The action beside it, which shows the counter.
    pub vanity_show: &'static str,
    /// The Secret screen's title over that counter.
    pub vanity_secret_title: &'static str,
    /// Tools row, and title of the Menu of the notes in hand.
    pub tools_notes: &'static str,
    /// Notes row: type one.
    pub notes_new: &'static str,
    /// Notes: what the Menu says while no note has been made.
    pub notes_none: &'static str,
    /// Entry title and Document title: one note.
    pub note_title: &'static str,
    /// Row on the note document: how many characters it holds.
    pub note_length_row: &'static str,
    /// Action on the note document: forget it.
    pub note_forget: &'static str,
    /// Scan reason: the file is not text a note can hold.
    pub note_not_text: &'static str,
    /// The name a saved note is offered under, plain.
    pub note_file_name: &'static str,
    /// The name a saved note or sheet is offered under, encrypted.
    pub seal_file_name: &'static str,
    /// Wallet row and Document title: the wallet's recovery sheet.
    pub wallet_sheet_row: &'static str,
    /// Row on the sheet: the wallet's descriptor.
    pub sheet_descriptor_row: &'static str,
    /// Row on the sheet: the note, while none is written.
    pub sheet_note_empty: &'static str,
    /// Row on the sheet: write or change the note.
    pub sheet_write_note: &'static str,
    /// Action on an opened sheet: register the wallet its descriptor
    /// names.
    pub sheet_add_wallet: &'static str,
    /// The name a saved sheet is offered under, plain.
    pub sheet_file_name: &'static str,
    /// Notes: why the keep toggle is dead — the record is full.
    pub notes_keep_full: &'static str,
    /// The heading of the descriptor section of a plain sheet file.
    pub sheet_plain_descriptor: &'static str,
    /// The heading of its name section.
    pub sheet_plain_name: &'static str,
    /// The heading of its note section.
    pub sheet_plain_note: &'static str,
    /// Choice title: which form an export leaves the device in.
    pub export_form_title: &'static str,
    /// Its rows.
    pub export_form_oskb: &'static str,
    /// See [`Strings::export_form_oskb`].
    pub export_form_kdbx: &'static str,
    /// See [`Strings::export_form_oskb`].
    pub export_form_plain: &'static str,
    /// Each row's second line, and the "Read with" value on the Result:
    /// the programs that open the form.
    pub export_form_oskb_opens: &'static str,
    /// See [`Strings::export_form_oskb_opens`].
    pub export_form_kdbx_opens: &'static str,
    /// See [`Strings::export_form_oskb_opens`].
    pub export_form_plain_opens: &'static str,
    /// Result row: the file's format and its version.
    pub sealed_format_row: &'static str,
    /// Its value for an `osk-backup` file, the version filled in.
    pub sealed_format_oskb: &'static str,
    /// Result row: the cipher the file is encrypted with.
    pub sealed_cipher_row: &'static str,
    /// Its value for an `osk-backup` file.
    pub sealed_cipher_oskb: &'static str,
    /// Its value for a KDBX 4 file.
    pub sealed_cipher_kdbx: &'static str,
    /// Result row: how the passphrase becomes the key.
    pub sealed_kdf_row: &'static str,
    /// Its value: the function, then the passes and the lanes.
    pub sealed_kdf_value: &'static str,
    /// The passes in that value, more than one.
    pub sealed_kdf_passes: &'static str,
    /// The same, one.
    pub sealed_kdf_one_pass: &'static str,
    /// The lanes in that value, more than one.
    pub sealed_kdf_lanes: &'static str,
    /// The same, one.
    pub sealed_kdf_one_lane: &'static str,
    /// Result row: the programs that open the file.
    pub sealed_read_row: &'static str,
    /// Row on a QR screen whose code is public: save the code as a
    /// picture.
    pub action_save_png: &'static str,
    /// The name that picture is offered under.
    pub png_file_name: &'static str,
    /// What the QR screen says once the shell has saved it.
    pub png_saved: &'static str,
    /// What it says where the shell could not.
    pub png_not_saved: &'static str,
    /// The Notes of a KDBX entry that holds a key's words: how many
    /// words and which wordlist.
    pub kdbx_words_notes: &'static str,
    /// The Notes of one that holds a master seed, which is written as
    /// hex.
    pub kdbx_seed_notes: &'static str,
    /// What a KDBX entry for a note is titled where the note's first
    /// line is blank.
    pub kdbx_note_title: &'static str,
    /// The name a KDBX export is offered under where nothing names it.
    pub kdbx_file_name: &'static str,
    /// Result title: the file the passphrase made.
    pub seal_title: &'static str,
    /// Its result line.
    pub seal_result: &'static str,
    /// The name a saved backup is offered under. It is the same for
    /// every backup: a file's name says nothing about the key.
    pub backup_file_name: &'static str,
    /// Title over a page of the word grid: `Words {} to {}`.
    pub words_range_title: &'static str,
    /// Title over the word grid: `Words {} of {}`.
    pub words_title: &'static str,
    /// Secondary action that puts each word's wordlist number beside
    /// it, and takes it away again.
    pub words_numbers_show: &'static str,
    /// The same action on a 268 dp panel, where §4.13 shortens a label
    /// that does not fit half the width rather than shrinking it.
    pub words_numbers_short: &'static str,
    /// Pager label: `{} of {}`.
    pub words_page: &'static str,
    /// Row and title: the code drawn as a grid to copy by hand.
    pub backup_grid: &'static str,
    /// Choice question over the two seed codes a grid can draw.
    pub backup_grid_title: &'static str,
    /// The four quadrants a `small` panel pages the grid by, as the
    /// pager's label.
    pub grid_top_left: &'static str,
    /// The top right quadrant.
    pub grid_top_right: &'static str,
    /// The bottom left quadrant.
    pub grid_bottom_left: &'static str,
    /// The bottom right quadrant.
    pub grid_bottom_right: &'static str,
    /// Row and title: the words as a numbered steel plate takes them.
    pub backup_steel: &'static str,
    /// Action on `wide`: write a blank numbered table to a file.
    pub backup_print_template: &'static str,
    /// The name the steel template is offered under.
    pub steel_template_file: &'static str,
    /// The heading line of that file.
    pub steel_template_heading: &'static str,
    /// Row and title: splitting the key into Seed XOR parts.
    pub xor_split: &'static str,
    /// Choice question over the number of parts.
    pub xor_count_title: &'static str,
    /// Choice question over where the split's random parts come from.
    pub xor_source_title: &'static str,
    /// Split result row: which source the random parts came from.
    pub xor_source_row: &'static str,
    /// Title of one part's words: `Part {} of {}`.
    pub xor_part_of: &'static str,
    /// Title of one part while it is being typed, and its row on a
    /// result: `Part {}`.
    pub xor_part: &'static str,
    /// The result line after a key is split.
    pub xor_split_result: &'static str,
    /// The result line after one part has been typed.
    pub xor_part_result: &'static str,
    /// Row on Add a key, and title of the flow it opens: a key whose
    /// only written form is SLIP-39 shares (`docs/PLANNING.md` §16.107
    /// rule 4).
    pub add_create_slip39: &'static str,
    /// Row on Add a key, and title of the flow it opens: a key whose
    /// only written form is codex32 strings (`docs/PLANNING.md`
    /// §16.109 rule 4).
    pub add_create_codex32: &'static str,
    /// Choice question over how long a seed a codex32 key is made from.
    pub codex32_length_title: &'static str,
    /// One of that Choice's rows: `{} bits`.
    pub codex32_length_row: &'static str,
    /// Choice question over whether the seed is split into shares.
    pub codex32_split_title: &'static str,
    /// That Choice's rows.
    pub codex32_split_no: &'static str,
    /// The other row of that Choice.
    pub codex32_split_yes: &'static str,
    /// Choice question over the number of shares a split has.
    pub codex32_count_title: &'static str,
    /// Choice question over how many of them must be present.
    pub codex32_threshold_title: &'static str,
    /// Choice question over where the split's random shares come from.
    pub codex32_source_title: &'static str,
    /// Title of the one string a seed that is not split is written as.
    pub codex32_string_title: &'static str,
    /// Title of one share's string: `Share {} of {}`.
    pub codex32_share_title: &'static str,
    /// Title of the entry a string is typed back into, in front of the
    /// string's own title: `Type it back`.
    pub codex32_type_back: &'static str,
    /// The error line when what was typed is not the string shown.
    pub codex32_not_same: &'static str,
    /// Action: go on without typing the string back.
    pub codex32_skip: &'static str,
    /// Title of the result after a key's strings are made.
    pub codex32_result_title: &'static str,
    /// The result line under it.
    pub codex32_result: &'static str,
    /// Result row: the plan the strings were written under.
    pub codex32_split_row: &'static str,
    /// Its value where the seed was split: `{} of {}`.
    pub codex32_split_value: &'static str,
    /// Its value where it was not.
    pub codex32_split_none: &'static str,
    /// Result row: what a codex32 backup of a key with words holds.
    pub codex32_holds_row: &'static str,
    /// Its value.
    pub codex32_holds_seed: &'static str,
    /// Choice question over the number of groups a backup has.
    pub share_groups_title: &'static str,
    /// Choice question over how many of them must be present.
    pub share_quorum_title: &'static str,
    /// Choice question over the number of shares a group has.
    pub share_count_title: &'static str,
    /// Choice question over how many of them must be present.
    pub share_threshold_title: &'static str,
    /// The group a plan Choice is about, in front of its question:
    /// `Group {} of {}`.
    pub share_group_of: &'static str,
    /// Choice question over where the split's random shares come from.
    pub share_source_title: &'static str,
    /// Result row: which source the random shares came from.
    pub share_source_row: &'static str,
    /// The random value being gathered, in front of the source's own
    /// title: `Random {} of {}`.
    pub share_random_of: &'static str,
    /// The group a share or a random value belongs to, in front of what
    /// it is: `Group {}`.
    pub share_group: &'static str,
    /// Title of one share's words: `Share {} of {}`.
    pub share_words_title: &'static str,
    /// Title of the result after a backup's shares are made.
    pub slip39_result_title: &'static str,
    /// The result line under it.
    pub slip39_result: &'static str,
    /// Result row value: groups present out of groups, `{} of {}`.
    pub share_groups_value: &'static str,
    /// Action: type one more part.
    pub xor_add_another: &'static str,
    /// Action: XOR the parts typed so far into a key.
    pub xor_combine: &'static str,
    /// Load source row: the key comes back from Seed XOR parts.
    pub load_source_xor: &'static str,
    /// SeedQR title: `SeedQR of {}`.
    pub seedqr_title: &'static str,
    /// CompactSeedQR title: `CompactSeedQR of {}`.
    pub compact_title: &'static str,
    /// Title of the quiz introduction.
    pub quiz_start_title: &'static str,
    /// Toggle: someone else is holding the device.
    pub quiz_helper_toggle: &'static str,
    /// Button that starts the quiz.
    pub quiz_start: &'static str,
    /// Question: `Which is word {}?`.
    pub quiz_question: &'static str,
    /// The same question where the title also carries the run's
    /// progress, which is what a 268 dp panel has the room for:
    /// `Word {} · {}`.
    pub quiz_question_short: &'static str,
    /// Title over a wrong answer.
    pub quiz_wrong_title: &'static str,
    /// Button that opens the words from a wrong answer.
    pub quiz_show_words: &'static str,
    /// Button after a wrong answer.
    pub quiz_retry: &'static str,
    /// Button that leaves the quiz.
    pub quiz_skip: &'static str,
    /// Button that skips anyway.
    pub quiz_skip_confirm: &'static str,
    /// Button that stays in the quiz.
    pub quiz_skip_cancel: &'static str,
    /// Record label on the skip caution.
    pub quiz_row: &'static str,
    /// Record value on the skip caution.
    pub quiz_skipped: &'static str,
    /// Record label: which question of the run.
    pub quiz_question_row: &'static str,
    /// Record label: how many questions were answered.
    pub quiz_questions_row: &'static str,
    /// Button that runs the quiz again.
    pub quiz_again: &'static str,
    /// Title while a helper is holding the device: `{} · helper`.
    pub quiz_helper_title: &'static str,
    /// Title after a pass.
    pub quiz_passed_title: &'static str,
    /// Title after an abandoned quiz.
    pub quiz_failed_title: &'static str,

    // ----- Create wizard (UX.md §7.2) -----
    /// Title of the source step.
    pub create_source_title: &'static str,
    /// Progress line under a dice or coin pad: `{} of {} · {} bits`.
    pub create_entry_count: &'static str,
    /// Inline error when ✓ comes before the digits the count needs.
    pub create_too_short: &'static str,
    /// Sanity result when nothing looks wrong.
    pub create_sanity_random: &'static str,
    /// Sanity result when a caution is set.
    pub create_sanity_uneven: &'static str,
    /// Sanity result for typed entropy, which no statistic checks.
    pub create_sanity_unchecked: &'static str,
    /// Record label: how many rolls.
    pub create_rolls_row: &'static str,
    /// Record label: how many flips.
    pub create_flips_row: &'static str,
    /// Record label: how many hex digits.
    pub create_digits_row: &'static str,
    /// Record label: how many bits they carry.
    pub create_bits_row: &'static str,
    /// Record label: whether the entries count rather than vary.
    pub create_order_row: &'static str,
    /// Record value: the entries count up or down.
    pub create_order_counting: &'static str,
    /// Record label on the math: the checksum.
    pub create_math_checksum: &'static str,
    /// Record value on the math: `{} bits`.
    pub create_math_bits: &'static str,
    /// Record label on the math: `Word {}`.
    pub create_math_word: &'static str,
    /// Title of the word-count step.
    pub create_count_title: &'static str,
    /// Title of the language step.
    pub create_language_title: &'static str,
    /// Title of the dice step: `Roll {} of {}`.
    pub create_dice_title: &'static str,
    /// Title of the coin step: `Flip {} of {}`.
    pub create_coin_title: &'static str,
    /// Title of the hex step.
    pub create_hex_title: &'static str,
    /// Title of the sanity step.
    pub create_sanity_title: &'static str,
    /// Button that discards the entries.
    pub create_sanity_again: &'static str,
    /// Row that opens the derivation screen.
    pub create_math_row: &'static str,
    /// Title of the derivation screen.
    pub create_math_title: &'static str,
    /// Label: the hash of the entries.
    pub create_math_entropy: &'static str,
    /// Source: a six-sided die.
    pub create_source_dice: &'static str,
    /// Source: coin flips.
    pub create_source_coins: &'static str,
    /// Source: typed hex.
    pub create_source_hex: &'static str,
    /// Source: a shuffled deck.
    pub create_source_cards: &'static str,
    /// Source: camera noise.
    pub create_source_camera: &'static str,
    /// Source: several sources mixed.
    pub create_source_mix: &'static str,
    /// Source: the device's own generator.
    pub create_source_device: &'static str,
    /// Label: the chi-square statistic of the rolls.
    pub create_chi_square: &'static str,
    /// Label: the longest run of one outcome.
    pub create_longest_run: &'static str,
    /// The run's value on a dice check: `{}` runs of the face named by
    /// [`Strings::create_face`], as `8 sixes`.
    pub create_run_face: &'static str,
    /// Plural name of dice face 1, for that value.
    pub create_face_1: &'static str,
    /// Plural name of dice face 2.
    pub create_face_2: &'static str,
    /// Plural name of dice face 3.
    pub create_face_3: &'static str,
    /// Plural name of dice face 4.
    pub create_face_4: &'static str,
    /// Plural name of dice face 5.
    pub create_face_5: &'static str,
    /// Plural name of dice face 6.
    pub create_face_6: &'static str,
    /// Verdict beside a statistic: `normal for {} rolls`.
    pub create_stat_normal: &'static str,
    /// Verdict beside a statistic that is out of the usual range.
    pub create_stat_high: &'static str,
    /// The same verdict where the row has one line to say it in: the
    /// count is already the row above.
    pub create_stat_normal_short: &'static str,
    /// The short form of the verdict for a statistic out of range.
    pub create_stat_high_short: &'static str,
    /// Label: heads.
    pub create_heads: &'static str,
    /// Label: tails.
    pub create_tails: &'static str,
    /// Title of the card pad: `{}` is the draw, `{}` the count needed.
    pub create_cards_title: &'static str,
    /// Which deck the cards are being drawn from: `{}` is 1 or 2.
    pub create_card_deck: &'static str,
    /// Caption under the card strip: the deck tracker refused a card
    /// that is already out.
    pub create_already_drawn: &'static str,
    /// Title of the dice-procedure Choice (`docs/PLANNING.md` §16.115).
    pub dice_procedure_title: &'static str,
    /// That Choice: the rolls hashed as ASCII digits.
    pub dice_procedure_hashed: &'static str,
    /// That Choice: every 6 written as a 0, then hashed.
    pub dice_procedure_six_as_zero: &'static str,
    /// That Choice: the rolls naming words directly.
    pub dice_procedure_words: &'static str,
    /// The second line of a procedure's row: `{} rolls`.
    pub dice_procedure_rolls: &'static str,
    /// Why direct selection is dimmed where the key has no BIP-39 words.
    pub reason_no_words: &'static str,
    /// Caption under the dice pad: the face was rerolled.
    pub create_rerolled: &'static str,
    /// Record label on the math: which dice procedure made the words.
    pub dice_procedure_row: &'static str,
    /// Record label: how many words the rolls named.
    pub create_words_row: &'static str,
    /// Label: how many cards were drawn.
    pub create_cards_row: &'static str,
    /// Title of the camera-noise step.
    pub create_camera_title: &'static str,
    /// The shutter under it.
    pub create_camera_shutter: &'static str,
    /// The state inside the viewfinder: `{}` is the frame, `{}` the
    /// count needed.
    pub create_camera_state: &'static str,
    /// Caption when a frame carries too few distinct luma values: the
    /// lens is covered, or the picture is saturated.
    pub create_too_little_variation: &'static str,
    /// Label: how many frames were taken.
    pub create_frames_row: &'static str,
    /// Label: how many pixels the frames the shell delivered carry.
    pub create_frame_size_row: &'static str,
    /// Label: how many of the 256 luma values the last frame used.
    pub create_distinct_row: &'static str,
    /// Label: the last frame's mean luma.
    pub create_mean_row: &'static str,
    /// Label: the last frame's variance.
    pub create_variance_row: &'static str,
    /// Title of the device-randomness step.
    pub create_device_title: &'static str,
    /// The coloured line on that step.
    pub create_device_result: &'static str,
    /// Label: what produced the bytes.
    pub create_device_row: &'static str,
    /// The generator on a device with no secure element.
    pub create_device_os: &'static str,
    /// The generator inside a trusted execution environment.
    pub create_device_tee: &'static str,
    /// The generator inside a separate security chip.
    pub create_device_strongbox: &'static str,
    /// The caution this source carries on the source list.
    pub create_trusts_device: &'static str,
    /// Label on the device-randomness result: what the bytes rest on.
    pub create_trust_row: &'static str,
    /// That row's value, in the caution tone.
    pub create_trust_device: &'static str,
    /// Title of the multi-select that chooses what a mix combines.
    pub create_mix_title: &'static str,
    /// Title of the step that lists a mix's commitments.
    pub create_mixed_title: &'static str,
    /// The coloured line on it.
    pub create_mixed_result: &'static str,
    /// "Draw again" on the sanity step, for cards.
    pub create_sanity_again_draw: &'static str,
    /// "Take again" on the sanity step, for the camera.
    pub create_sanity_again_take: &'static str,

    // ----- Sign (UX.md §7.3) -----
    /// Title of the entry, parse-error and result screens.
    pub sign_title: &'static str,
    /// Review label: what leaves the wallet.
    pub sign_amount: &'static str,
    /// Review label when every output is our own.
    pub sign_move: &'static str,
    /// Review label: the recipient.
    pub sign_to: &'static str,
    /// Review call to action.
    pub sign_review: &'static str,
    /// Title of an output screen: `Output {} of {}`.
    pub sign_output_title: &'static str,
    /// One output by name, `Output {}`, which is the instance line over
    /// its address on the Compare screen (§5 Compare).
    pub sign_output_name: &'static str,
    /// Title of the warnings screen.
    pub sign_warnings_title: &'static str,
    /// Row value while a file request is out.
    pub sign_waiting: &'static str,
    /// Title of the summary step.
    pub sign_summary_title: &'static str,
    /// Label: the fee.
    pub sign_fee: &'static str,
    /// Label: the fee rate.
    pub sign_rate: &'static str,
    /// Label on an output that returns to this wallet.
    pub sign_change: &'static str,
    /// Said of change no loaded key derives.
    pub sign_change_unverified: &'static str,
    /// Title of the inputs step.
    pub sign_inputs_title: &'static str,
    /// Label: the inputs.
    pub sign_inputs: &'static str,
    /// How many inputs are ours: `{} yours`.
    pub sign_inputs_ours: &'static str,
    /// Row label of one input: `#{}`.
    pub sign_input_row: &'static str,
    /// Record label: what the inputs add up to.
    pub sign_total_in: &'static str,
    /// Record label for the multisig policy.
    pub sign_multisig: &'static str,
    /// Record label for the cosigner fingerprints.
    pub sign_cosigners: &'static str,
    /// Label: replace-by-fee.
    pub sign_rbf: &'static str,
    /// Label: the locktime.
    pub sign_locktime: &'static str,
    /// Title of the confirmation step.
    pub sign_confirm_title: &'static str,
    /// Toggle over a danger warning.
    pub sign_acknowledge: &'static str,
    /// The hold-to-sign button.
    pub sign_hold: &'static str,
    /// The hold when the pass ends in a MuSig2 nonce and no signature.
    pub sign_hold_share: &'static str,
    /// Every input was signed.
    pub sign_result_complete: &'static str,
    /// Some inputs are still unsigned.
    pub sign_result_partial: &'static str,
    /// The pass ended in round 1 of MuSig2: a nonce was shared and
    /// nothing was signed.
    pub sign_result_nonce: &'static str,
    /// Row: the MuSig2 session this device now holds.
    pub sign_session_row: &'static str,
    /// Its value while it is open.
    pub sign_session_open: &'static str,
    /// Row: what ends the session.
    pub sign_session_ends_row: &'static str,
    /// What ends it.
    pub sign_session_ends: &'static str,
    /// Row and title: the signatures written to the transaction.
    pub sign_signatures: &'static str,
    /// Row: the file the result was written to.
    pub sign_file_row: &'static str,
    /// Which file: `saved as {}`.
    pub sign_saved_as: &'static str,
    /// The file row when the shell wrote nothing: no card, no room, a
    /// picker the person cancelled.
    pub sign_not_saved: &'static str,
    /// Title of the QR screen.
    pub sign_qr_title: &'static str,
    /// Label under the transaction QR.
    pub sign_qr_label: &'static str,
    /// Toggle: the animated parts.
    pub sign_qr_animated: &'static str,
    /// Why one QR is not offered on this display.
    pub sign_qr_dense: &'static str,
    /// Why the review cannot go on: nothing is loaded to sign with.
    pub sign_no_key: &'static str,
    /// Why the review cannot go on: the loaded keys sign nothing here.
    pub sign_keys_not_in: &'static str,
    /// Label of the key-context row.
    pub sign_key_row: &'static str,
    /// Row label when several keys signed.
    pub sign_keys_row: &'static str,
    /// Title when the bytes are not a transaction.
    pub sign_parse_error_title: &'static str,
    /// Row label on that screen: what was read.
    pub sign_read_row: &'static str,
    /// What it was not.
    pub sign_not_psbt: &'static str,
    /// The quorum: `{} of {} · {} signed`.
    pub sign_multisig_value: &'static str,
    /// Label over the key chips on the hold screen.
    pub sign_sign_with: &'static str,
    /// Label over the second chip row on the hold screen: the other
    /// shares of a threshold wallet that will sign at a later location.
    pub sign_then_with: &'static str,
    /// The signing attempt failed.
    pub sign_failed: &'static str,
    /// Label over the transaction id.
    pub sign_txid: &'static str,
    /// The kind one signature is, which titles its Compare screen
    /// (§5 Compare).
    pub sign_signature: &'static str,
    /// Title of the page that lists the signatures a transaction
    /// carries (`docs/PLANNING.md` §16.111).
    pub sign_signatures_title: &'static str,
    /// A signature that is the key's, over this transaction.
    pub sign_sig_valid: &'static str,
    /// A signature the key did not make over this transaction.
    pub sign_sig_invalid: &'static str,
    /// A signature nothing was checked about: `not checked · {}`.
    pub sign_sig_unchecked: &'static str,
    /// Why: the input carries no previous output.
    pub sign_sig_no_prevout: &'static str,
    /// Why: the script is one this device does not take apart.
    pub sign_sig_unsupported: &'static str,
    /// Why: the script names no key to check against.
    pub sign_sig_unknown_key: &'static str,
    /// The nonce rule that reproduced a signature: `deterministic · {}`.
    pub sign_sig_deterministic: &'static str,
    /// No nonce rule reproduced it.
    pub sign_sig_not_deterministic: &'static str,
    /// The BIP-340 nonce rule, beside Settings' two RFC 6979 ones.
    pub sign_nonce_bip340: &'static str,
    /// Warning value: a signature that is not this transaction's,
    /// `{} on input {}`.
    pub warn_signature_invalid: &'static str,
    /// Warning value: one nonce under one key, twice.
    pub warn_nonce_reuse: &'static str,
    /// Warning value: a signature of ours matching no nonce rule.
    pub warn_nondeterministic: &'static str,
    /// Warning value: a signature present and not checked.
    pub warn_signature_unchecked: &'static str,
    /// Warning label: an inscription envelope.
    pub warn_inscription_label: &'static str,
    /// Warning value: `input {} · {}`.
    pub warn_inscription: &'static str,
    /// The envelope states no content type.
    pub warn_inscription_unknown: &'static str,
    /// Tools row and title: two transactions compared.
    pub tools_compare: &'static str,
    /// Row: the first transaction read.
    pub compare_tx_first: &'static str,
    /// Row: the second.
    pub compare_tx_second: &'static str,
    /// The way to the second transaction.
    pub compare_tx_read_second: &'static str,
    /// Result: the two are one transaction.
    pub compare_tx_same: &'static str,
    /// Result: they are not.
    pub compare_tx_different: &'static str,
    /// Row label for a difference in the transaction itself.
    pub compare_tx_transaction: &'static str,
    /// Row label for one input, `Input {}`.
    pub compare_tx_input: &'static str,
    /// Row label for one output, `Output {}`.
    pub compare_tx_output: &'static str,
    /// Row label for the signing state.
    pub compare_tx_signing: &'static str,
    /// Value: a key that signed only the first, `{} signed only here`.
    pub compare_tx_signed_first: &'static str,
    /// Value: a key that signed only the second.
    pub compare_tx_signed_second: &'static str,
    /// Value: final only in the first.
    pub compare_tx_final_first: &'static str,
    /// Value: final only in the second.
    pub compare_tx_final_second: &'static str,
    /// Value: `{} \u{2192} {}`, the two values of a changed field.
    pub compare_tx_changed: &'static str,
    /// Value: the two values are too long to show side by side.
    pub compare_tx_differs: &'static str,
    /// Field: the transaction version.
    pub cmp_version: &'static str,
    /// Field: the absolute locktime.
    pub cmp_locktime: &'static str,
    /// Field: how many inputs.
    pub cmp_inputs: &'static str,
    /// Field: how many outputs.
    pub cmp_outputs: &'static str,
    /// Field: the outpoint an input spends.
    pub cmp_spends: &'static str,
    /// Field: an input's sequence.
    pub cmp_sequence: &'static str,
    /// Field: an output's amount.
    pub cmp_amount: &'static str,
    /// Field: an output's script.
    pub cmp_script: &'static str,
    /// Field: the transaction an input spends.
    pub cmp_previous_tx: &'static str,
    /// Field: the output an input spends.
    pub cmp_spent_output: &'static str,
    /// Field: a redeem script.
    pub cmp_redeem_script: &'static str,
    /// Field: a witness script.
    pub cmp_witness_script: &'static str,
    /// Field: the sighash an input asks for.
    pub cmp_sighash: &'static str,
    /// Field: key origins.
    pub cmp_key_origins: &'static str,
    /// Field: taproot key origins.
    pub cmp_tap_key_origins: &'static str,
    /// Field: a taproot internal key.
    pub cmp_internal_key: &'static str,
    /// Field: a taproot merkle root.
    pub cmp_merkle_root: &'static str,
    /// Field: leaf scripts and their control blocks.
    pub cmp_leaf_scripts: &'static str,
    /// Field: key-value pairs the device does not know.
    pub cmp_unknown: &'static str,
    /// Field: proprietary key-value pairs.
    pub cmp_proprietary: &'static str,
    /// Field: the global extended public keys.
    pub cmp_xpubs: &'static str,
    /// Warning label: one output.
    pub warn_output: &'static str,
    /// Warning label: how much of the transaction a signature covers.
    pub warn_sighash_label: &'static str,
    /// Warning value: change whose script the key does not produce.
    pub warn_change_spoof: &'static str,
    /// Warning value: the fee as a share of the amount, `{} %`.
    pub warn_high_fee: &'static str,
    /// Warning value: an output too small to be worth spending.
    pub warn_dust: &'static str,
    /// Warning value: a signature that does not cover the whole
    /// transaction.
    pub warn_sighash: &'static str,
    /// Warning value: the inputs spend more than one script type.
    pub warn_mixed_scripts: &'static str,
    /// Warning value: a key belongs to another chain.
    pub warn_network: &'static str,
    /// Warning value: no loaded key signs any input.
    pub warn_no_key: &'static str,
    /// Warning value: a path outside every loaded account.
    pub warn_unknown_path: &'static str,
    /// Warning value: a fee rate far above anything ordinary.
    pub warn_absurd_rate: &'static str,
    /// Warning value: the transaction carries a locktime.
    pub warn_locktime: &'static str,
    /// Warning value: an output pays a script with no address.
    pub warn_non_standard: &'static str,
    /// Warning value: two outputs share one script.
    pub warn_address_reuse: &'static str,
    /// Warning value: the input's own data disagrees with itself.
    pub warn_utxo_mismatch: &'static str,
    /// Warning value: an input carries no amount.
    pub warn_missing_utxo: &'static str,
    /// Warning value: an input spends a script this build cannot sign.
    pub warn_unsupported: &'static str,
    /// Warning value: a MuSig2 input's participants are not those of any
    /// registered wallet.
    pub warn_musig_not_registered: &'static str,
    /// Warning value: a MuSig2 input carries a nonce of this device's
    /// that no open session holds, so this pass replaces it.
    pub warn_musig_nonce_replaced: &'static str,
    /// Warning value: a threshold input names a group no registered
    /// wallet has.
    pub warn_threshold_not_registered: &'static str,
    /// Warning value: the file read is bound to another transaction.
    pub warn_threshold_other_tx: &'static str,
    /// Warning value: the file read holds no nonce for a key this
    /// device has.
    pub warn_threshold_other_share: &'static str,
    /// Warning value: the file names a different set of signers from the
    /// transaction.
    pub warn_threshold_signer_set: &'static str,
    /// Warning value: a stored nonce is not the one the transaction
    /// carries for that share.
    pub warn_threshold_nonce: &'static str,
    /// Warning value: a partial signature on the transaction does not
    /// verify.
    pub warn_threshold_partial_sig: &'static str,
    /// Warning value: the transaction carries nonces and no file was
    /// read.
    pub warn_threshold_no_file: &'static str,
    /// Warning value: a file was read for a transaction with no nonces
    /// on it.
    pub warn_threshold_no_nonces: &'static str,
    /// Warning value: this device's key is not one of the signers.
    pub warn_threshold_not_a_signer: &'static str,
    /// Warning value: the keys chosen to sign are not the number that
    /// must sign.
    pub warn_threshold_chosen: &'static str,
    /// Warning label on every card that blocks signing, in place of the
    /// field the warning is about.
    pub warn_blocked: &'static str,
    /// Warning value: a SegWit input carries no previous transaction, so
    /// nothing checks the amount it states.
    pub warn_amount_unverified: &'static str,
    /// Warning value: the spend path an input takes still waits on a
    /// timelock, `Timelock not met: {}`.
    pub warn_timelock_not_met: &'static str,

    // ----- Scanner (UX.md §4) -----
    /// Title when the scanner takes anything.
    pub scan_title: &'static str,
    /// Title when a seed QR is expected.
    pub scan_title_seed: &'static str,
    /// Title when an encrypted backup is expected.
    pub scan_title_backup: &'static str,
    /// Title when a transaction is expected.
    pub scan_title_psbt: &'static str,
    /// Title when an address is expected.
    pub scan_title_address: &'static str,
    /// State inside the viewfinder while the camera is live.
    pub scan_looking: &'static str,
    /// State inside the viewfinder while a file is being read.
    pub scan_waiting: &'static str,
    /// State inside the viewfinder when the shell has no camera.
    pub scan_no_camera: &'static str,
    /// Row that reads a file instead of a QR.
    pub scan_from_file: &'static str,
    /// Row that takes the clipboard instead of a QR.
    pub scan_paste: &'static str,
    /// Row that opens the keyboard the flow has.
    pub scan_type: &'static str,
    /// State inside the viewfinder after a Paste that found no text.
    pub scan_nothing_to_paste: &'static str,
    /// Reason a clipboard row is dead.
    pub reason_no_clipboard: &'static str,
    /// Row action that puts a public string on the clipboard.
    pub action_copy: &'static str,
    /// Caption after a Copy that worked.
    pub copy_done: &'static str,
    /// Caption after a Copy the shell could not do.
    pub copy_none: &'static str,
    /// Reason a payload was refused because it is a secret.
    pub scan_reason_secret: &'static str,
    /// Title of the Result a refused secret lands on.
    pub scan_refused_title: &'static str,
    /// Its coloured line.
    pub scan_refused_result: &'static str,
    /// Label of the row that says where the refused payload came from,
    /// when it came off the clipboard.
    pub row_clipboard: &'static str,
    /// Title of the Entry the builder's "Type" row opens.
    pub build_type_title: &'static str,
    /// Fact row on the Policy entry: the fingerprints a policy may name.
    pub tool_policy_keys: &'static str,
    /// Row on Convert key's scanner: take a key this device holds.
    pub tool_use_loaded_key: &'static str,
    /// Row on the Unknown QR menu: hash the bytes.
    pub scan_as_hashes: &'static str,
    /// Row on the Unknown QR menu: read the bytes as an encoding.
    pub scan_as_encodings: &'static str,

    /// Title of the Files screen: the list a shell that can list its
    /// files answers a file request with (`docs/DESIGN.md` §5).
    pub files_title: &'static str,
    /// The Files screen with nothing on the list.
    pub files_none: &'static str,
    /// A file's size where the shell has no clock to date it by:
    /// `{} bytes`.
    pub files_bytes: &'static str,
    /// The same in kilobytes: `{} KB`.
    pub files_kb: &'static str,
    /// Parts of a multi-part QR: `Part {} of {}`.
    pub scan_ur_progress: &'static str,
    /// Result title when a QR could not be used.
    pub scan_error_title: &'static str,
    /// Reason: the digits are not a SeedQR.
    pub scan_reason_seedqr: &'static str,
    /// Reason: the QR names a type this build does not read.
    pub scan_reason_type: &'static str,
    /// Reason: the bytes carry the backup magic and a header this build
    /// does not read.
    pub scan_reason_backup: &'static str,
    /// A wallet policy this device cannot read or check.
    pub scan_reason_policy: &'static str,
    /// Title of the menu behind an unrecognised QR.
    pub scan_unknown_title: &'static str,
    /// Row: read the QR as a transaction.
    pub scan_as_psbt: &'static str,
    /// Row: read the QR as text.
    pub scan_as_text: &'static str,
    /// Result title for words a camera has just read.
    pub scan_words_title: &'static str,
    /// The action that loads words read in the clear.
    pub scan_words_load: &'static str,

    // ----- Signed messages (UX.md F7 and G4) -----
    /// Scanner: the title while a message to sign is expected.
    pub scan_title_message: &'static str,
    /// Scanner: the title while a signed message is expected.
    pub scan_title_signed: &'static str,
    /// Tools row and result title: check a signed message.
    pub msg_check_row: &'static str,
    /// Title of the message screen and of the message a check shows.
    pub msg_title: &'static str,
    /// Row label: which signature format.
    pub msg_format_row: &'static str,
    /// The legacy compact signature.
    pub msg_bip137: &'static str,
    /// The BIP-322 "simple" signature.
    pub msg_bip322: &'static str,
    /// Why BIP-322 is dimmed for a nested-segwit address.
    pub msg_bip322_nested: &'static str,
    /// Title of the QR that carries the signature.
    pub msg_qr_title: &'static str,
    /// What the QR under the square is.
    pub msg_qr_label: &'static str,
    /// Why the Animated row is dimmed: the signature fits one code.
    pub msg_qr_one_code: &'static str,
    /// The result of signing a message.
    pub msg_signed_title: &'static str,
    /// The signature holds for the address.
    pub msg_valid_title: &'static str,
    /// It does not.
    pub msg_invalid_title: &'static str,
    /// Row label: the address form the check read.
    pub msg_checked_row: &'static str,
    /// Row label and value row: the message a check was over.
    pub msg_message_row: &'static str,
    /// Scanner: the text is not a signed message.
    pub msg_not_signed: &'static str,

    // ----- Verify › address (UX.md §7.4) -----
    /// Title of the entry menu and the result.
    pub verify_entry_title: &'static str,
    /// Title of the typing screen.
    pub verify_title: &'static str,
    /// Row: the wallet page's way into the address check.
    pub wallet_check: &'static str,
    /// The address belongs to a loaded key.
    pub verify_yours_title: &'static str,
    /// The address was not found.
    pub verify_not_found_title: &'static str,
    /// Row label: how far the search went.
    pub verify_searched: &'static str,
    /// How far: `{} addresses per wallet`.
    pub verify_search_depth: &'static str,
    /// The text is not an address.
    pub verify_invalid_title: &'static str,
    /// The address is for another network.
    pub verify_network_title: &'static str,
    /// The action that clears the result and asks for another address.
    pub verify_check_another: &'static str,

    // ----- A scanned descriptor, key or text -----
    /// Title for a descriptor.
    pub inspect_descriptor: &'static str,
    /// Title for an extended public key.
    pub inspect_xpub: &'static str,
    /// Title for plain text.
    pub inspect_text: &'static str,
    /// Nothing was captured.
    pub inspect_empty: &'static str,
    /// Row label: how long the text is.
    pub inspect_length_row: &'static str,
    /// Length value: `{} characters`.
    pub inspect_length: &'static str,
    /// Row label: the descriptor checksum.
    pub inspect_checksum: &'static str,
    /// The descriptor checksum holds.
    pub inspect_checksum_ok: &'static str,
    /// The descriptor checksum fails.
    pub inspect_checksum_bad: &'static str,
    /// Card title: this wallet is one key away from a wallet in use.
    pub inspect_swap: &'static str,
    /// Which key changed: `Key {} replaced by {}`.
    pub inspect_swap_replaced: &'static str,
    /// The key that replaced it states no origin: `Key {} replaced by a
    /// key with no origin`.
    pub inspect_swap_no_origin: &'static str,
    /// The replacement keeps the fingerprint: `Key {}, same fingerprint,
    /// different key`.
    pub inspect_swap_same: &'static str,
    /// The one key names a master this device holds and is not the key
    /// that master derives: `Claims to be {} and is not`.
    pub inspect_swap_claims: &'static str,
    /// Title and row label for a wallet policy.
    pub wallet_title: &'static str,
    /// The wallet's Name row, and the title of the field it opens.
    pub wallet_name: &'static str,
    /// The menu row an unnamed wallet carries in place of that row.
    pub wallet_set_name: &'static str,
    /// What a wallet over one key is, in its row.
    pub wallet_single: &'static str,
    /// What a wallet of several keys and a quorum is, in its row.
    pub wallet_kind_multisig: &'static str,
    /// What a wallet whose script says how it can be spent is, in its
    /// row.
    pub wallet_kind_miniscript: &'static str,
    /// What a `tr(key, tree)` wallet is, in its row.
    pub wallet_kind_tree: &'static str,
    /// What a BIP 387 tapscript multisig is, in its row.
    pub wallet_kind_taproot_multisig: &'static str,
    /// What a wallet with a timelocked second path is, in its row.
    pub wallet_kind_recovery: &'static str,
    /// The Kind row's value for such a wallet.
    pub wallet_recovery: &'static str,
    /// Its primary path's row label.
    pub wallet_recovery_now: &'static str,
    /// A recovery path's row label: `After {} days`.
    pub wallet_recovery_after: &'static str,
    /// What a recovery path row states: `{} of {} \u{00b7} {} blocks`.
    pub wallet_recovery_value: &'static str,
    /// Row label for a taproot wallet's key path.
    pub wallet_key_path: &'static str,
    /// Its value when the internal key is BIP 341's NUMS point.
    pub wallet_unspendable: &'static str,
    /// The quorum of a multisig wallet: `{} of {}`.
    pub wallet_quorum: &'static str,
    /// What a MuSig2 wallet is, as a row label and in its name.
    pub wallet_musig: &'static str,
    /// What a threshold wallet is, as a row label and in its name.
    pub wallet_threshold: &'static str,
    /// The same in a wallet's row on Home, where the kind is lower case.
    pub wallet_kind_threshold: &'static str,
    /// How many keys aggregate into a MuSig2 wallet: `{} keys`.
    pub wallet_musig_keys: &'static str,
    /// Row label for the kind of wallet a miniscript or taproot tree is.
    pub wallet_kind_row: &'static str,
    /// A `wsh(...)` or `sh(wsh(...))` wallet whose script says how it
    /// can be spent.
    pub wallet_miniscript: &'static str,
    /// A `tr(key, tree)` wallet: a key path and one or more leaves.
    pub wallet_tree: &'static str,
    /// Row label for one way the wallet's coins can be spent, numbered
    /// in the order the script writes them: `Path {}`.
    pub wallet_spend_path: &'static str,
    /// The same, where one path is named on its own and there is no
    /// number to give it: the warning card over an unmet timelock.
    pub wallet_spend_path_row: &'static str,
    /// A spend path that wants one key: `Key {}`.
    pub wallet_path_key: &'static str,
    /// A spend path that wants several: `Keys {}`.
    pub wallet_path_keys: &'static str,
    /// Two names in a list before the last: `{}, {}`.
    pub wallet_path_list: &'static str,
    /// The last two names of a list: `{} and {}`.
    pub wallet_path_last: &'static str,
    /// Keys and the wait before they can spend: `{} after {}`.
    pub wallet_path_after: &'static str,
    /// A wait with no key behind it: `After {}`.
    pub wallet_path_after_only: &'static str,
    /// A spend path with neither a key nor a wait.
    pub wallet_path_anyone: &'static str,
    /// The taproot key path, as the way an input spends its wallet.
    pub wallet_path_keypath: &'static str,
    /// A relative timelock in blocks, and how long that is:
    /// `{} blocks \u{00b7} {}`.
    pub wallet_lock_blocks: &'static str,
    /// The same in 512-second units: `{} intervals \u{00b7} {}`.
    pub wallet_lock_seconds: &'static str,
    /// An absolute timelock the chain reaches by height: `block {}`.
    pub wallet_lock_height: &'static str,
    /// A hash the spender must produce the preimage of.
    pub wallet_lock_preimage: &'static str,
    /// One day, as an approximate duration.
    pub wallet_about_day: &'static str,
    /// More than one: `about {} days`.
    pub wallet_about_days: &'static str,
    /// One year.
    pub wallet_about_year: &'static str,
    /// More than one: `about {} years`.
    pub wallet_about_years: &'static str,
    /// The action that adds the wallet to the ones in use.
    pub wallet_use: &'static str,
    /// Row: sign a transaction with this wallet.
    pub wallet_sign: &'static str,
    /// Row: sign a message with this wallet.
    pub wallet_sign_message: &'static str,
    /// The state of a wallet registered for the session.
    pub wallet_in_use: &'static str,
    /// The value of a wallet's key row where a master fingerprint would
    /// stand, for a key that arrived with no origin.
    pub wallet_origin_unknown: &'static str,
    /// The action that drops it again.
    pub wallet_forget: &'static str,
    /// The Add row and start row that opens the wallet scanner.
    pub wallet_load: &'static str,
    /// Title when a wallet is expected at the scanner.
    pub scan_title_wallet: &'static str,
    /// The wallet menu's toggle that keeps this wallet on the device.
    pub wallet_keep_row: &'static str,
    /// Title of the Choice that asks it when the wallet is added.
    pub wallet_keep_title: &'static str,
    /// Its rows.
    pub wallet_keep_no: &'static str,
    /// The row that keeps it.
    pub wallet_keep_yes: &'static str,
    /// The wallet menu's row that opens the keys the wallet is made of.
    pub wallet_keys: &'static str,
    /// The same row on a wallet over one key, which has one.
    pub wallet_key: &'static str,
    /// One member of a FROST wallet, counted from one on screen:
    /// `Key {} of {}`.
    pub wallet_member: &'static str,

    // ----- FROST wallets (`crate::threshold`) -----
    /// "How many keys?"
    pub threshold_count_title: &'static str,
    /// The QR screen of the group record.
    pub threshold_record_title: &'static str,
    /// The name the record is saved under: `frost-{}.txt`.
    pub threshold_file_name: &'static str,
    /// The FROST review's title.
    pub threshold_confirm_title: &'static str,
    /// Its row for the wallet.
    pub threshold_confirm_wallet: &'static str,
    /// The wallet menu's row that exports it to a coordinator.
    pub wallet_export: &'static str,
    /// The export format that is the BIP-388 policy: the template and
    /// its keys.
    pub export_policy: &'static str,
    /// The same, for BIP 129's descriptor record.
    pub export_bsms: &'static str,
    /// The same, for BIP 129's key record, which one signer writes
    /// from one of its own multisig accounts.
    pub export_bsms_signer: &'static str,
    /// The key page's row that exports one of the key's public
    /// accounts (`docs/PLANNING.md` §16.110 rule 1).
    pub key_account_row: &'static str,
    /// Title of the Choice that row opens.
    pub account_which_title: &'static str,
    /// Account: BIP-44, one key per address.
    pub account_legacy: &'static str,
    /// Account: BIP-49.
    pub account_nested: &'static str,
    /// Account: BIP-84.
    pub account_segwit: &'static str,
    /// Account: BIP-86, which a MuSig2 wallet also takes.
    pub account_taproot: &'static str,
    /// Account: BIP-48 script type 1.
    pub account_multisig_nested: &'static str,
    /// Account: BIP-48 script type 2, which a taproot multisig also
    /// takes.
    pub account_multisig_segwit: &'static str,
    /// Title of a key's account export, which is not a wallet's.
    pub export_key_title: &'static str,
    /// Entry title: the session token of a BSMS setup.
    pub bsms_token_title: &'static str,
    /// The preset row above that field, which writes `00`.
    pub bsms_token_none: &'static str,
    /// Entry title: the description the record carries.
    pub bsms_description_title: &'static str,
    /// Error line: BIP 129 allows eighty characters.
    pub bsms_description_long: &'static str,

    // BIP-85's other applications, and the Bitcoin Core import file
    // (`docs/PLANNING.md` §16.114).
    /// The key page's row that derives one of BIP-85's applications.
    pub key_bip85_row: &'static str,
    /// Title of the Choice that row opens.
    pub bip85_which_title: &'static str,
    /// Application 39': a child key of words.
    pub bip85_app_words: &'static str,
    /// Application 2': an HD-seed WIF.
    pub bip85_app_wif: &'static str,
    /// Application 32': an extended private key.
    pub bip85_app_xprv: &'static str,
    /// Application 128169': raw bytes.
    pub bip85_app_hex: &'static str,
    /// Application 707764': a password in base64.
    pub bip85_app_base64: &'static str,
    /// Application 707785': a password in base85.
    pub bip85_app_base85: &'static str,
    /// Title of the Choice that asks the hex application's length.
    pub bip85_bytes_title: &'static str,
    /// One of its rows: a byte count.
    pub bip85_bytes_row: &'static str,
    /// Pad title: a password's length.
    pub bip85_length_title: &'static str,
    /// The caption under that field, which states the bounds.
    pub bip85_length_range: &'static str,
    /// Secret screen title: the application and the index.
    pub bip85_value_title: &'static str,
    /// The export format that is Bitcoin Core's import file.
    pub export_core_import: &'static str,
    /// Title of the Choice before it.
    pub core_rescan_title: &'static str,
    /// Its first row: from the genesis block.
    pub core_rescan_start: &'static str,
    /// What that costs and finds.
    pub core_rescan_start_value: &'static str,
    /// Its second row: from now.
    pub core_rescan_now: &'static str,
    /// What that suits.
    pub core_rescan_now_value: &'static str,
    /// The name the import file is offered under.
    pub core_import_file_name: &'static str,
    /// Review row: the chains a BIP 129 record restricts the wallet to.
    pub inspect_bsms_paths: &'static str,
    /// Review row: the wallet's first receive address.
    pub inspect_bsms_first: &'static str,
    /// Review row: which key of the list is this device's.
    pub inspect_bsms_ours: &'static str,
    /// Its value where this device holds none of them.
    pub inspect_bsms_not_ours: &'static str,
    /// Refusal: round 1 of a setup, which is a key and not a wallet.
    pub scan_reason_bsms_signer: &'static str,
    /// Refusal: a record this device cannot read.
    pub scan_reason_bsms: &'static str,
    /// Refusal: an encrypted record, which this build does not open.
    pub scan_reason_bsms_encrypted: &'static str,

    // ----- The wallet builder (`crate::build`) -----
    /// The wizard's own title, and the Add a wallet row that starts it.
    pub build_new: &'static str,
    /// Title of the kind Choice.
    pub build_kind_title: &'static str,
    /// Title of the script-type Choice.
    pub build_script_title: &'static str,
    /// The dimmed row for a Taproot multisig, which this build has no
    /// template for. The word is the script type, as on the other rows.
    pub build_script_taproot: &'static str,
    /// Title of the Keys step, with the count so far: `Keys \u{00b7} {}`.
    pub build_keys_title: &'static str,
    /// Title of the Choice that says which key to add.
    pub build_which_title: &'static str,
    /// That Choice's last row, which opens the scanner.
    pub build_scan_key: &'static str,
    /// Title when a cosigner's key is expected at the scanner.
    pub build_scan_title: &'static str,
    /// Title of the threshold Choice.
    pub build_threshold_title: &'static str,
    /// Title of the primary path's threshold Choice.
    pub build_threshold_now_title: &'static str,
    /// Title of the recovery path's threshold Choice.
    pub build_threshold_later_title: &'static str,
    /// Title of the Keys step for a recovery wallet's primary path,
    /// with the count so far: `Now \u{00b7} {}`.
    pub build_now_title: &'static str,
    /// The same for its recovery path: `Later \u{00b7} {}`.
    pub build_later_title: &'static str,
    /// The same where the wallet has more than one recovery path, with
    /// the path's number first: `Later {} \u{00b7} {}`.
    pub build_later_n_title: &'static str,
    /// Title of the wait Choice.
    pub build_delay_title: &'static str,
    /// One of its rows: `{} days`.
    pub build_delay_days: &'static str,
    /// The block count under that row: `{} blocks`.
    pub build_delay_blocks: &'static str,
    /// Its last row, which opens the pad a number of days is typed on.
    pub build_delay_type: &'static str,
    /// Reason: the wait is not longer than the path before it, whose own
    /// wait fills it: `not after {} days`.
    pub build_delay_too_short: &'static str,
    /// Title of the pad that number is typed on.
    pub build_days_title: &'static str,
    /// The error under its field: `at most {} days`.
    pub build_days_too_many: &'static str,
    /// Title of the Choice that offers a further recovery path.
    pub build_another_path_title: &'static str,
    /// That Choice's first row.
    pub build_another_no: &'static str,
    /// Its second.
    pub build_another_yes: &'static str,
    /// Title of the Choice a wallet started from a key's page opens on.
    pub build_passphrase_title: &'static str,
    /// That Choice's first row: the wallet is over the loaded key.
    pub build_passphrase_no: &'static str,
    /// Its second: the wallet is over a passphrase key of its words.
    pub build_passphrase_yes: &'static str,
    /// Title of the passphrase entry that Yes opens, which adds a key.
    pub build_passphrase_key_title: &'static str,
    /// Reason: the key already spends on the wallet's other path.
    pub build_reason_other_path: &'static str,
    /// The confirm that guards the keys on the way out.
    pub build_discard_title: &'static str,
    /// Reason: the key states no master it descends from.
    pub build_reason_origin: &'static str,
    /// Reason: a mainnet key on a device on a test network.
    pub build_reason_mainnet: &'static str,
    /// Reason: a test-network key on a mainnet device.
    pub build_reason_testnet: &'static str,
    /// Reason: the wallet already holds this key.
    pub build_reason_added: &'static str,
    /// Reason: the wallet holds the most keys a multisig script takes.
    pub build_reason_full: &'static str,
    /// §4.11 reason: the wallet holds every key its kind takes, so this
    /// one cannot join. §16.128 rule 1.
    pub build_reason_group_full: &'static str,
    /// Reason: the text is a whole wallet rather than one key.
    pub build_reason_wallet: &'static str,
    /// Reason a key cannot be a member of a FROST group: it is not 24
    /// words. `{}` is the count it is.
    pub build_reason_words: &'static str,
    /// Reason a 24-word key cannot be one either: this device does not
    /// hold its words, or they state no scalar.
    pub build_reason_not_usable: &'static str,
    /// The dimmed row that states how many keys a FROST group of this
    /// shape needs: `Needs {} keys`.
    pub build_needs_keys: &'static str,

    // ----- Tools (UX.md §7.5) -----
    /// Title of the area, and of the menu it opens on.
    pub tools_title: &'static str,
    /// Row: the key explorer.
    pub tools_explorer: &'static str,
    /// Row: browse and search a BIP-39 word list.
    pub tools_word_list: &'static str,
    /// Row: an EFF diceware passphrase from dice.
    pub tools_dice_passphrase: &'static str,
    /// Row: the hash functions.
    pub tools_hashes: &'static str,
    /// Row: base58, bech32, hex, binary.
    pub tools_encodings: &'static str,
    /// Row: the descriptor checksum.
    pub tools_descriptor_checksum: &'static str,
    /// Row: xpub to SLIP-132 and back.
    pub tools_convert_key: &'static str,
    /// Row: the unit converter.
    pub tools_units: &'static str,
    /// Tools row: the miniscript policy compiler.
    pub tools_miniscript: &'static str,
    /// Row: read a transaction without signing it.
    pub tools_decode: &'static str,
    /// Change row of a transaction being read on a device with no key
    /// and no wallet: there was nothing to recognise change against.
    pub decode_no_key: &'static str,

    // ----- Tools > the calculators -----
    /// Title of the Hashes field.
    pub tool_hashes_title: &'static str,
    /// The mode row above that field.
    pub tool_read_as: &'static str,
    /// Read the field as hex where it is hex, as text where it is not.
    pub tool_read_auto: &'static str,
    /// Read the field's own characters.
    pub tool_read_text: &'static str,
    /// Read the field as hex digits.
    pub tool_read_hex: &'static str,
    /// Record row: how many bytes the input was.
    pub tool_length_row: &'static str,
    /// The unit of that row.
    pub tool_bytes: &'static str,
    /// Record row: SHA-256.
    pub tool_sha256_row: &'static str,
    /// Record row: SHA-256 applied twice.
    pub tool_sha256d_row: &'static str,
    /// Record row: RIPEMD-160 of SHA-256.
    pub tool_hash160_row: &'static str,
    /// Title of the Encodings field.
    pub tool_encoding_title: &'static str,
    /// Record row: which encoding the string is.
    pub tool_encoding_row: &'static str,
    /// Base58 with a four-byte check.
    pub tool_base58check: &'static str,
    /// Base58 with nothing appended.
    pub tool_base58: &'static str,
    /// BIP-173 bech32.
    pub tool_bech32: &'static str,
    /// BIP-350 bech32m.
    pub tool_bech32m: &'static str,
    /// Hex digits, two per byte.
    pub tool_hex: &'static str,
    /// Caption line: the string is none of the encodings.
    pub tool_not_an_encoding: &'static str,
    /// Record row: the Base58Check version byte.
    pub tool_version_row: &'static str,
    /// Record row: the bech32 human-readable part.
    pub tool_hrp_row: &'static str,
    /// Record row: the verdict on the checksum the text arrived with.
    pub tool_checksum_row: &'static str,
    /// That row's value where the checksum given is the one this device
    /// computes.
    pub tool_checksum_valid: &'static str,
    /// That row's value where it is not.
    pub tool_checksum_wrong: &'static str,
    /// That row's value where the text arrived with no checksum.
    pub tool_checksum_missing: &'static str,
    /// Title of the Descriptor field.
    pub tool_descriptor_title: &'static str,
    /// Record row: the descriptor with its checksum.
    pub tool_descriptor_row: &'static str,
    /// Record row: the checksum this device computed.
    pub tool_computed_checksum_row: &'static str,
    /// Title of the Extended key field.
    pub tool_key_title: &'static str,
    /// Record row: the BIP-32 spelling.
    pub tool_bip32_row: &'static str,
    /// Record row: which chain the key is for.
    pub tool_network_row: &'static str,
    /// Record row: how far below the master the key is.
    pub tool_depth_row: &'static str,
    /// Record row: the child number it was derived at.
    pub tool_child_row: &'static str,
    /// Caption line: an extended private key, which this tool refuses.
    pub tool_public_only: &'static str,
    /// Caption line: not an extended key at all.
    pub tool_not_a_key: &'static str,
    /// Title of the Units field.
    pub tool_units_title: &'static str,
    /// Title of the miniscript compiler's field.
    pub tool_policy_title: &'static str,
    /// Its mode row: what the compiled policy is wrapped in.
    pub tool_policy_script: &'static str,
    /// That mode's two values.
    pub tool_policy_segwit: &'static str,
    /// The other.
    pub tool_policy_taproot: &'static str,
    /// The action on a compiled policy that is a wallet.
    pub tool_policy_load: &'static str,
    /// The mode row above that field.
    pub tool_from: &'static str,
    /// Satoshi.
    pub tool_sats: &'static str,
    /// Whole bitcoin.
    pub tool_btc: &'static str,
    /// Millibitcoin.
    pub tool_mbtc: &'static str,
    /// Bits, which are microbitcoin.
    pub tool_bits: &'static str,

    // ----- Tools > Word list (UX.md §7.5) -----
    /// Title of the Choice that picks the notation the field is searched
    /// by.
    pub wordlist_by_title: &'static str,
    /// The mode row above the field, whose value is the notation.
    pub wordlist_by: &'static str,
    /// Search by the word itself, on the language's own keyboard.
    pub wordlist_by_word: &'static str,
    /// Search by the 1-based number.
    pub wordlist_by_number: &'static str,
    /// Search by the eleven bits.
    pub wordlist_by_binary: &'static str,
    /// Search by the three hex digits.
    pub wordlist_by_hex: &'static str,
    /// Title of the search screen.
    pub wordlist_search_title: &'static str,
    /// Row of that Choice: open the list at its first word and walk it
    /// with the pager.
    pub wordlist_browse: &'static str,
    /// Caption line: the number typed is outside 1 to 2048.
    pub wordlist_out_of_range: &'static str,
    /// Caption line: the hex typed is above 7ff.
    pub wordlist_hex_out_of_range: &'static str,
    /// Caption line: nothing in this list starts like that.
    pub wordlist_no_match: &'static str,
    /// Record row: the word's 1-based place in the list.
    pub word_number_row: &'static str,
    /// Record row: the word's 0-based index, which is what BIP-39 and
    /// every other tool counts by.
    pub word_index_row: &'static str,
    /// Record row: the eleven bits.
    pub word_binary_row: &'static str,
    /// Record row: the three hex digits.
    pub word_hex_row: &'static str,
    /// Record row: which wordlist the word is from.
    pub word_language_row: &'static str,

    // ----- Tools > Dice passphrase (UX.md §7.5) -----
    /// Title of the Choice that picks the list.
    pub dice_list_title: &'static str,
    /// Row: the 7776-word list.
    pub dice_list_large: &'static str,
    /// What that row costs to roll.
    pub dice_list_large_detail: &'static str,
    /// Row: the first 1296-word list.
    pub dice_list_short1: &'static str,
    /// What that row costs to roll.
    pub dice_list_short1_detail: &'static str,
    /// Row: the second 1296-word list.
    pub dice_list_short2: &'static str,
    /// What that row costs to roll, and what its words are chosen for.
    pub dice_list_short2_detail: &'static str,
    /// Title of the Choice that picks the length.
    pub dice_words_title: &'static str,
    /// A length and what it is worth: `{} words · {} bits`.
    pub dice_words_bits: &'static str,
    /// Title of the pad: `Roll {} of {}`.
    pub dice_roll_title: &'static str,
    /// The count under the pad: `{} of {} · {} bits`.
    pub dice_roll_count: &'static str,
    /// Title of the passphrase itself.
    pub dice_result_title: &'static str,
    /// Record row: what the passphrase is worth.
    pub dice_entropy_row: &'static str,
    /// That row's value: `{} bits`.
    pub dice_bits: &'static str,
    /// Title of the confirm that guards the rolls.
    pub dice_discard_title: &'static str,

    // ----- Explore (UX.md §7.5) -----
    /// Title of the area, and of the menu it opens on.
    pub explore_title: &'static str,
    /// Title of the Choice that picks what to explore.
    pub explore_using_title: &'static str,
    /// The chooser row that stands for typed words.
    pub explore_typed: &'static str,
    /// Row that opens word entry.
    pub explore_type_words: &'static str,
    /// Row and title: the words and what they encode.
    pub explore_section_words: &'static str,
    /// Heading: encodings of the account key.
    pub explore_section_encodings: &'static str,
    /// Value of the passphrase row when one is set.
    pub explore_passphrase_set: &'static str,
    /// Label: the fingerprint at the applied path.
    pub explore_fingerprint: &'static str,
    /// The confirm before typed words are discarded.
    pub explore_discard_title: &'static str,
    /// Button that keeps the typed words.
    pub explore_keep: &'static str,
    /// Button that discards them.
    pub explore_discard: &'static str,
    /// Label: the entropy behind the words.
    pub explore_entropy: &'static str,
    /// Label: the checksum bits.
    pub explore_checksum: &'static str,
    /// Label: the 64-byte seed.
    pub explore_seed: &'static str,
    /// Label: the master private key.
    pub explore_master_xprv: &'static str,
    /// Label and title: the derivation path.
    pub explore_path: &'static str,
    /// A path with two slashes in a row.
    pub explore_path_empty: &'static str,
    /// A path that ends with a slash.
    pub explore_path_unfinished: &'static str,
    /// A level that is not a number.
    pub explore_path_not_a_number: &'static str,
    /// An index of two to the thirty-first or more.
    pub explore_path_too_large: &'static str,
    /// More than ten levels.
    pub explore_path_too_deep: &'static str,
    /// Label: the account key.
    pub explore_account_xpub: &'static str,
    /// Label: the SLIP-132 form of the account key.
    pub explore_slip132: &'static str,
    /// Row into the address screen of this key.
    pub explore_addresses: &'static str,
    /// Label: the extended public key at the applied path.
    pub explore_level_xpub: &'static str,
    /// Label: the extended private key at the applied path.
    pub explore_level_xprv: &'static str,
    /// Group label: the path editor's four purpose presets (§4.6).
    pub explore_purpose_group: &'static str,
    /// Group label: the path editor's Receive | Change pair (§4.6).
    pub explore_chain_group: &'static str,
    /// Why this purpose has no SLIP-132 form, in §4.6's vocabulary.
    pub explore_no_slip132: &'static str,

    // ----- Settings and the lock screen (UX.md §7.7) -----
    /// Title.
    pub settings_title: &'static str,
    /// Row and Choice title: the network.
    pub settings_network: &'static str,
    /// Row and Choice title: the unit amounts are shown in.
    pub settings_unit: &'static str,
    /// Unit: satoshis.
    pub settings_unit_sat: &'static str,
    /// Unit: whole bitcoin.
    pub settings_unit_btc: &'static str,
    /// Row that locks now.
    pub settings_lock_now: &'static str,
    /// Row and Choice title: the auto-lock timer.
    pub settings_lock_after: &'static str,
    /// Row and Choice title: the auto-wipe timer.
    pub settings_wipe_after: &'static str,
    /// Timer value: no auto-wipe.
    pub settings_never: &'static str,
    /// Row and Choice title: how far a camera frame is turned before it
    /// is read.
    pub settings_camera_rotation: &'static str,
    /// Camera rotation: no turn.
    pub settings_camera_rotation_0: &'static str,
    /// Camera rotation: a quarter turn clockwise.
    pub settings_camera_rotation_90: &'static str,
    /// Camera rotation: a half turn.
    pub settings_camera_rotation_180: &'static str,
    /// Camera rotation: a quarter turn anticlockwise.
    pub settings_camera_rotation_270: &'static str,
    /// Toggle: shuffle the PIN pad.
    pub settings_scramble: &'static str,
    /// Row and Choice title: which RFC 6979 nonce a signature uses.
    pub settings_nonce: &'static str,
    /// Nonce: retry until the signature is its shortest form.
    pub settings_nonce_low_r: &'static str,
    /// Nonce: the first nonce RFC 6979 gives.
    pub settings_nonce_first: &'static str,
    /// Row and Choice title: where a Schnorr signature's auxiliary
    /// randomness comes from.
    pub settings_schnorr: &'static str,
    /// Schnorr: no auxiliary randomness, so the bytes are reproducible.
    pub settings_schnorr_deterministic: &'static str,
    /// Schnorr: 32 fresh bytes for every signature.
    pub settings_schnorr_fresh: &'static str,
    /// Label of the in-memory encryption key's state.
    pub settings_memory_key: &'static str,
    /// That key came from the shell's own entropy.
    pub settings_memory_key_ok: &'static str,
    /// The shell gave no entropy.
    pub settings_memory_key_weak: &'static str,
    /// Row and title of the About screen.
    pub settings_about: &'static str,
    /// Label: the version.
    pub settings_version: &'static str,
    /// Label: the assurance tier.
    pub settings_tier: &'static str,
    /// Label: what the platform says about this device's boot. Also the
    /// caution card's label before a key is kept.
    pub settings_boot: &'static str,
    /// Label: the hash of the core.
    pub settings_core_hash: &'static str,
    /// No hash was recorded.
    pub settings_core_hash_none: &'static str,
    /// Label: the self-test result.
    pub settings_selftest: &'static str,
    /// The self-test passed: `{}` is the check count.
    pub settings_selftest_passed: &'static str,
    /// The self-test failed: `{}` is the check.
    pub settings_selftest_failed: &'static str,
    /// Action that runs the self-test again.
    pub settings_selftest_run: &'static str,
    /// Row and title of the wipe screen.
    pub settings_wipe_row: &'static str,
    /// The hold-to-wipe button.
    pub settings_wipe_hold: &'static str,
    /// Value of the stored-key row on both wipe screens, where a key is
    /// kept on the device: the wipe removes it too. `keep_stored_row`
    /// labels it.
    pub settings_wipe_stored: &'static str,
    /// Result title once every key is gone.
    pub settings_wiped_title: &'static str,
    /// Row that opens the wipe-and-exit screen.
    pub settings_exit_row: &'static str,
    /// Title of the wipe-and-exit screen.
    pub settings_exit_title: &'static str,
    /// The hold-to-exit button.
    pub settings_exit_hold: &'static str,
    /// Title.
    pub lock_title: &'static str,
    /// After a wrong PIN: `{}` is the attempts left.
    pub lock_wrong: &'static str,

    // ----- A key kept on the device (`keep.rs`, PLANNING §15 item 32) -----
    /// Key menu row that keeps this key on the device.
    pub keep_row: &'static str,
    /// The same row once the key is kept; its value names the hardware.
    pub keep_row_kept: &'static str,
    /// Title of the Hold that keeps it.
    pub keep_title: &'static str,
    /// The hold-to-keep button.
    pub keep_hold: &'static str,
    /// The plain action beside it on the offer that follows adding a
    /// key, which declines it.
    pub keep_not_now: &'static str,
    /// Row on that Hold: what unlocks the stored key.
    pub keep_unlocked_by: &'static str,
    /// Its value: the session PIN and the secure element together.
    pub keep_unlocked_by_value: &'static str,
    /// After a wrong PIN on the stored key's pad, or on the lock screen
    /// of a device that keeps a key: `{}` is the attempts left before
    /// it is removed.
    pub keep_wrong: &'static str,
    /// The stored key, as the Wipe screen names it among what goes. Its
    /// pad carries `lock_title`: the two are one pad (§16.63).
    pub keep_stored_row: &'static str,
    /// Result title once the stored key is gone.
    pub keep_removed_title: &'static str,
    /// Settings row that sets a duress PIN, and the pad's title.
    pub keep_duress_row: &'static str,
    /// About row: what backs the secure element.
    pub keep_secure_row: &'static str,
    /// Its value where there is a separate chip.
    pub keep_secure_strongbox: &'static str,
    /// Its value where a trusted execution environment backs it.
    pub keep_secure_tee: &'static str,

    // ----- Self-test (UX.md §7.1) -----
    /// Title.
    pub selftest_title: &'static str,
    /// Failure title.
    pub selftest_failed_title: &'static str,
    /// Row label: the check that did not reproduce.
    pub selftest_check_row: &'static str,
    /// The only button.
    pub selftest_exit: &'static str,

    // ----- The refusal on a device whose boot was not verified -----
    /// Result title: the platform did not verify the running system.
    pub boot_refused_title: &'static str,
    /// Row label: the system that is running.
    pub boot_refused_system: &'static str,
    /// Its value.
    pub boot_refused_system_value: &'static str,
    /// Row label: what makes this device a signer again.
    pub boot_refused_fix: &'static str,
    /// Its value.
    pub boot_refused_fix_value: &'static str,
    /// The only button.
    pub boot_refused_exit: &'static str,

    // ----- Assurance tiers (`docs/PLANNING.md` §3) -----
    /// Tier A as the badge §4.8 names: the Home status line and the
    /// About row, wherever the line has the width for it.
    pub tier_a_badge: &'static str,
    /// Tier B as the same badge.
    pub tier_b_badge: &'static str,
    /// Tier C as the same badge.
    pub tier_c_badge: &'static str,
    /// Tier D as the same badge.
    pub tier_d_badge: &'static str,
    /// Tier A on a 268 dp status line, where §4.8 drops the word "Tier"
    /// so the badge fits beside the network badge and two buttons.
    pub tier_a_badge_short: &'static str,
    /// Tier B on that line.
    pub tier_b_badge_short: &'static str,
    /// Tier C on that line.
    pub tier_c_badge_short: &'static str,
    /// Tier D on that line.
    pub tier_d_badge_short: &'static str,
    /// The tier badge's letter alone, which is what the 268 dp status
    /// line has room for while a session badge shares it (§4.8).
    pub tier_a_badge_letter: &'static str,
    /// The same for Tier B.
    pub tier_b_badge_letter: &'static str,
    /// The same for Tier C.
    pub tier_c_badge_letter: &'static str,
    /// The same for Tier D.
    pub tier_d_badge_letter: &'static str,
    /// Assurance tier A.
    pub tier_a: &'static str,
    /// Assurance tier B.
    pub tier_b: &'static str,
    /// Assurance tier C.
    pub tier_c: &'static str,
    /// Assurance tier D.
    pub tier_d: &'static str,
    /// What tier A can and cannot see.
    pub tier_a_statement: &'static str,
    /// What tier B can and cannot see.
    pub tier_b_statement: &'static str,
    /// What tier C can and cannot see.
    pub tier_c_statement: &'static str,
    /// What tier D can and cannot see.
    pub tier_d_statement: &'static str,
    /// Title of the tier explanation screen.
    pub tier_title: &'static str,
    /// Heading over this device's own tier: `{}` is the tier's name.
    pub tier_this_device: &'static str,
    /// Title of the screen a Tier A device shows once at start: the
    /// board has no secure boot, so the card is the whole of the
    /// software's provenance (security review 2026-09-11, M6).
    pub no_secure_boot_title: &'static str,
    /// The coloured line on that screen.
    pub no_secure_boot_result: &'static str,
    /// Row label: the board's check of what it boots.
    pub no_secure_boot_device_row: &'static str,
    /// Row value: whether this board has one.
    pub no_secure_boot_device: &'static str,
    /// The one way on, to Home.
    pub no_secure_boot_continue: &'static str,

    // ----- Silent payments (`docs/PLANNING.md` §16.113) -----
    /// The kind on "What kind of wallet?", and the name a silent
    /// payments wallet goes by.
    pub wallet_silent: &'static str,
    /// The same kind on a Wallets row, beside the network.
    pub wallet_kind_silent: &'static str,
    /// A silent payments wallet's "Address" row, and the title of the
    /// screen it opens.
    pub silent_address_row: &'static str,
    /// The label under that screen's code.
    pub silent_address_label: &'static str,
    /// The form the address is shown in: the Choice's title.
    pub silent_form_title: &'static str,
    /// That Choice's first row, and the form the screen opens on.
    pub silent_form_address: &'static str,
    /// Its second row: BIP-321's URI.
    pub silent_form_uri: &'static str,
    /// The wallet's "Labels" row, and the title of the list it opens.
    pub silent_labels_row: &'static str,
    /// One row of that list, and the title of the address it opens.
    pub silent_label: &'static str,
    /// The bottom row that hands out the next label.
    pub silent_add_label: &'static str,
    /// §4.11 reason: the wallet has all the labels it holds.
    pub silent_labels_full: &'static str,
    /// The wallet's "Check a payment" row, and the title of the flow.
    pub silent_check_row: &'static str,
    /// The row that reads the transaction.
    pub silent_read: &'static str,
    /// The row that reads the previous transaction an input needs, with
    /// the transaction it is waiting for under it.
    pub silent_previous_row: &'static str,
    /// The coloured line of a check that found a payment.
    pub silent_paid: &'static str,
    /// The coloured line of a check that found none.
    pub silent_not_paid: &'static str,
    /// The coloured line for a transaction with no input a shared
    /// secret comes from.
    pub silent_no_inputs: &'static str,
    /// The coloured line for a transaction whose inputs carry no
    /// signatures yet.
    pub silent_unsigned: &'static str,
    /// The coloured line for a wallet whose key is not loaded.
    pub silent_key_not_loaded: &'static str,
    /// Row label on that Result: which output pays, `{}` its index.
    pub silent_output_row: &'static str,
    /// Row value: the label an output was paid to, `{}` its number.
    pub silent_output_label: &'static str,
    /// Row value: an output paid to the address itself.
    pub silent_output_plain: &'static str,
    /// Export format: BIP-392's `sp(spscan…)`, which carries the scan
    /// private key.
    pub export_silent_scan: &'static str,
    /// Export format: BIP-321's URI.
    pub export_silent_uri: &'static str,
    /// Export format: BIP-353's TXT record.
    pub export_silent_dns: &'static str,
    /// The Entry that asks for the name before the domain.
    pub silent_user_title: &'static str,
    /// The Entry that asks for the domain.
    pub silent_domain_title: &'static str,
    /// Row label: the wallet's address, on the review and the export.
    pub silent_address_value_row: &'static str,
    /// The coloured line of Check an address for a silent payment
    /// address, which no wallet's chain of addresses holds.
    pub verify_silent_title: &'static str,

    // ----- Learn (UX.md §3, the L jobs) -----
    /// The Learn pages themselves (`osk_learn`).
    pub learn: &'static osk_learn::Learn,
    /// The last row of Learn › "Randomness".
    pub learn_try_dice: &'static str,
    /// The last row of Learn › "Backups".
    pub learn_try_verify: &'static str,
    /// The last row of Learn › "Encrypted backups".
    pub learn_try_encrypted: &'static str,
    /// The last row of Learn › "Passphrases".
    pub learn_try_passphrase: &'static str,
    /// The last row of Learn › "Spend paths and timelocks".
    pub learn_try_miniscript: &'static str,
}

/// The English wording.
pub static EN: Strings = Strings {
    // Actions and words that appear on many screens.
    action_continue: "Continue",
    action_done: "Done",
    action_start_over: "Start over",
    action_again: "Try again",
    settings_ended_title: "Session ended",
    leave_again: "Back again to exit and clear memory",
    leave_title: "Exit",
    leave_ended_keys: "cleared from memory",
    settings_ended_keys: "removed",
    secret_reveal: "Touch and hold to show",
    secret_held: "Release to hide",
    secret_showing: "Showing for {} s",
    action_save_file: "Save to file",
    action_show_qr: "Show as QR",
    action_more: "More",
    value_none: "none",
    duration_seconds: "{} s",
    duration_minutes: "{} min",
    script_legacy: "Legacy",
    script_nested: "Nested",
    script_segwit: "SegWit",
    script_taproot: "Taproot",
    script_taproot_path: "Taproot script",
    script_miniscript: "Miniscript",
    script_multisig: "Multisig {} of {}",
    script_unknown: "Unknown script",
    value_yes: "yes",
    value_no: "no",
    value_unknown: "unknown",
    reason_needs_camera: "no camera",
    reason_needs_tier_b: "needs Tier B",
    reason_needs_key: "no key",
    row_path: "Path",
    row_address: "Address",
    row_qr: "QR",
    row_hex: "Hex",
    row_bytes: "Bytes",
    row_mine: "Mine",
    row_keys: "Keys",
    script_type_row: "Script type",
    script_type_title: "Which script type?",
    reason_browser: "browser",

    // Home (UX.md §4).
    home_title: "OpenSigner",
    home_name: "Home",
    home_scan: "Scan",
    status_session_open: "Session open",
    home_explore: "Explore",
    home_tools: "Tools",
    home_settings: "Settings",
    home_learn: "Learn",
    home_wallets: "Wallets",
    home_keys: "Keys",
    keys_add: "Add a key",
    wallets_add: "Add a wallet",
    add_single: "Single-sig",
    add_multisig: "Multisig",
    add_taproot_multisig: "Taproot multisig",
    add_recovery: "Recovery",
    key_made_of: "Key",
    key_made_words: "{} words",
    key_made_passphrase: "{} words and a passphrase",
    key_made_child: "BIP-85 child",
    key_made_slip39: "SLIP-39 shares",
    key_made_slip39_passphrase: "SLIP-39 shares and a passphrase",
    key_made_codex32: "Codex32",
    key_not_loaded: "not loaded",
    home_load_key: "Load a key",
    home_create_key: "Create a key",

    // Keys list (UX.md §7.2).

    // Load wizard (UX.md §7.2).
    load_source_title: "Load from?",
    load_source_type: "Type the words",
    load_source_scan: "Scan a SeedQR",
    load_source_numbers: "Word numbers",
    load_source_hex: "Hex entropy",
    load_source_element: "Secure element",
    load_source_backup: "Read an encrypted backup",
    load_source_slip39: "SLIP-39 shares",
    load_source_codex32: "Codex32",
    load_codex32_title: "Codex32",
    load_codex32_share_title: "Share {}",
    load_codex32_shares: "Shares",
    load_codex32_shares_value: "{} of {} needed",
    load_share_word_title: "Share {}",
    load_share_accepted_title: "Share accepted",
    load_share_refused_title: "Share refused",
    load_share_enough: "Enough shares",
    load_share_more: "More shares needed",
    load_share_too_many: "more shares than this device holds",
    load_share_groups: "Groups",
    load_share_groups_value: "{} of {} needed",
    load_share_group: "Group {}",
    load_share_group_value: "{} of {} shares",
    load_count_title: "How many words?",
    load_language_title: "Which language?",
    load_language_row: "Language",
    load_word_title: "Word {} of {}",
    load_word_no_match: "No word starts like that",
    load_words_row: "Words",
    load_checksum_title: "Checksum",
    load_checksum_valid: "Valid",
    load_checksum_failed: "Checksum failed",
    load_suspect_label: "Likely wrong",
    load_suspect_word: "word {}",
    load_fix_word: "Fix word {}",

    // Passphrase, fingerprint confirmation and session PIN.
    passphrase_offer_title: "Add a passphrase?",
    passphrase_none: "No passphrase",
    passphrase_add: "Add a passphrase",
    passphrase_title: "Passphrase",
    passphrase_which_title: "Which key?",
    passphrase_without: "no passphrase",
    passphrase_with: "with passphrase",
    confirm_title: "Add this key?",
    confirm_key: "Key",
    confirm_network: "Network",
    confirm_add: "Add key",
    pin_set_title: "Set a PIN",
    pin_mismatch: "PINs differed",
    pin_repeat_title: "Repeat the PIN",

    // Key detail and its screens (UX.md §7.2).
    detail_title: "Key {}",
    detail_addresses: "Addresses",
    detail_export: "Wallet export",
    detail_backup: "Backup",
    detail_passphrase: "Passphrase",
    detail_open_passphrase: "Open with passphrase",
    detail_open_child: "Open BIP-85 child",
    open_child_index: "Index",
    open_child_index_range: "0 to 2147483647",
    open_wrong_passphrase: "Opens {}, not {}",
    open_wrong_child: "This child is {}, not {}",
    opened_title: "Passphrase key",
    opened_child_title: "BIP-85 child",
    opened_result: "Added",
    opened_open: "Open wallet",
    created_title: "Key created",
    created_result: "Key added",
    detail_backup_verified: "verified",
    detail_backup_unverified: "not verified",
    addresses_title: "Addresses",
    addresses_receive: "Receive",
    addresses_change: "Change",
    addresses_receive_low: "receive",
    addresses_change_low: "change",
    addresses_index: "{} {} {}",
    addresses_row: "{} {}",
    addresses_deriving: "Deriving the account…",
    export_title: "Wallet export",
    export_format_row: "Format",
    export_format_title: "Which format?",
    export_descriptor: "Descriptor",
    export_xpub: "Account key",
    export_slip132: "SLIP-132",
    export_too_long: "Too long for one QR.",
    forget_wallet_row: "Its wallet",
    forget_wallet: "removed with it",
    forget_policy: "stays, can no longer sign",
    forget_title: "Forget key",
    forget_hold: "Hold to forget",

    // Backup: words, numbers, codes and the quiz.
    backup_title: "Backup",
    backup_words: "Show words",
    backup_seedqr: "SeedQR",
    backup_compact: "CompactSeedQR",
    backup_verify: "Verify backup",
    backup_slip39: "SLIP-39 shares",
    backup_codex32: "Codex32",
    backup_no_words: "no words",
    backup_encrypted: "Encrypted backup",
    backup_encrypted_result: "Encrypted",
    backup_pass_title: "Backup passphrase",
    backup_pass_repeat: "Repeat it",
    backup_pass_mismatch: "Passphrases differed",
    backup_wrong_pass: "Wrong passphrase",
    backup_inside_row: "Inside",
    backup_inside_words: "this key's words",
    backup_inside_parent: "the parent key's words",
    backup_needs_memory: "Needs {} MiB",
    backup_memory_row: "Memory",
    backup_memory_value: "{} MiB",
    backup_inside_seed: "this key's master seed",
    settings_backup_memory: "Backup memory",
    settings_backup_memory_anywhere: "opens anywhere",
    settings_backup_memory_recommended: "recommended",
    settings_backup_memory_slowest: "slowest to open",
    tools_lightning: "Lightning node key",
    lightning_from_title: "From?",
    lightning_from_aezeed: "An LND cipher seed",
    lightning_from_key: "A loaded key",
    lightning_no_words: "no key with words",
    lightning_node_key: "Node public key",
    lightning_version: "Cipher seed version",
    lightning_birthday: "Birthday",
    lightning_birthday_value: "{} \u{b7} {} days",
    lightning_key_row: "Key",
    lightning_software_row: "Derived as",
    lightning_software_lnd: "LND",
    lightning_software_ldk: "ldk-node",
    lightning_read: "Cipher seed read",
    lightning_derived: "Node key derived",
    lightning_no_answer: "No node key",
    lightning_bad_passphrase: "Wrong passphrase",
    lightning_bad_checksum: "The words do not check out",
    lightning_bad_version: "Not a version 0 cipher seed",
    lightning_show_secret: "Show secret",
    lightning_secret_title: "Node private key",
    lightning_entropy_title: "Entropy",
    key_vanity_row: "Vanity address",
    vanity_how_title: "How?",
    vanity_how_passphrase: "Passphrase counter",
    vanity_how_account: "Account index",
    vanity_how_passphrase_under: "a key derived each try",
    vanity_how_account_under: "an account derived each try",
    vanity_slow: "slow here",
    vanity_no_words: "no words",
    vanity_has_passphrase: "has a passphrase",
    vanity_prefix_title: "Address prefix",
    vanity_running_title: "Grinding",
    vanity_tried_row: "Tried",
    vanity_rate_row: "Rate",
    vanity_rate_value: "{} a second",
    vanity_expected_row: "Expected",
    vanity_seconds: "{} seconds",
    vanity_minutes: "{} minutes",
    vanity_hours: "{} hours",
    vanity_days: "{} days",
    vanity_over_a_year: "over a year",
    vanity_stop: "Stop",
    vanity_found_title: "Address found",
    vanity_exhausted_title: "No address found",
    vanity_account_row: "Account",
    vanity_counter_row: "Counter",
    vanity_use: "Use it",
    vanity_show: "Show it",
    vanity_secret_title: "Passphrase counter",
    tools_notes: "Notes",
    notes_new: "New note",
    notes_none: "No notes",
    note_title: "Note",
    note_length_row: "Characters",
    note_forget: "Forget",
    note_not_text: "not text",
    note_file_name: "opensigner-note.txt",
    seal_file_name: "opensigner-sealed.oskb",
    wallet_sheet_row: "Recovery sheet",
    sheet_descriptor_row: "Descriptor",
    sheet_note_empty: "none",
    sheet_write_note: "Write the note",
    sheet_add_wallet: "Add this wallet",
    sheet_file_name: "opensigner-sheet.txt",
    notes_keep_full: "8 kept",
    sheet_plain_descriptor: "Descriptor",
    sheet_plain_name: "Name",
    sheet_plain_note: "Note",
    export_form_title: "Which form?",
    export_form_oskb: "Encrypted backup",
    export_form_kdbx: "KDBX 4",
    export_form_plain: "Plain text",
    export_form_oskb_opens: "OpenSigner \u{00b7} decrypt.py",
    export_form_kdbx_opens: "a KeePass app",
    export_form_plain_opens: "any text editor \u{00b7} not encrypted",
    sealed_format_row: "Format",
    sealed_format_oskb: "OSKB {}",
    sealed_cipher_row: "Cipher",
    sealed_cipher_oskb: "XChaCha20-Poly1305",
    sealed_cipher_kdbx: "ChaCha20",
    sealed_kdf_row: "Key derivation",
    sealed_kdf_value: "Argon2id \u{00b7} {} \u{00b7} {}",
    sealed_kdf_passes: "{} passes",
    sealed_kdf_one_pass: "1 pass",
    sealed_kdf_lanes: "{} lanes",
    sealed_kdf_one_lane: "1 lane",
    sealed_read_row: "Read with",
    action_save_png: "Save as PNG",
    png_file_name: "opensigner-qr.png",
    png_saved: "Saved as {}",
    png_not_saved: "Not saved",
    kdbx_words_notes: "{} words, {}",
    kdbx_seed_notes: "Master seed, {} bytes, as hex",
    kdbx_note_title: "Note",
    kdbx_file_name: "opensigner.kdbx",
    seal_title: "Encrypted",
    seal_result: "Sealed",
    backup_file_name: "opensigner-backup.oskb",
    words_range_title: "Words {} to {}",
    words_title: "Words",
    words_numbers_show: "Show numbers",
    words_numbers_short: "Numbers",
    words_page: "{} of {}",
    backup_grid: "Draw a SeedQR",
    backup_grid_title: "Which code?",
    grid_top_left: "Top left",
    grid_top_right: "Top right",
    grid_bottom_left: "Bottom left",
    grid_bottom_right: "Bottom right",
    backup_steel: "Numbers for steel",
    backup_print_template: "Print template",
    steel_template_file: "opensigner-steel-template.txt",
    steel_template_heading: "BIP-39 words",
    xor_split: "Split with Seed XOR",
    xor_count_title: "How many parts?",
    xor_source_title: "Random parts from?",
    xor_source_row: "Random parts",
    xor_part_of: "Part {} of {}",
    xor_part: "Part {}",
    xor_split_result: "Split",
    xor_part_result: "Part read",
    add_create_slip39: "Create SLIP-39 shares",
    add_create_codex32: "Create Codex32 shares",
    codex32_length_title: "How long a seed?",
    codex32_length_row: "{} bits",
    codex32_split_title: "Split into shares?",
    codex32_split_no: "No, one string",
    codex32_split_yes: "Yes",
    codex32_count_title: "How many shares?",
    codex32_threshold_title: "How many must be present?",
    codex32_source_title: "Random shares from?",
    codex32_string_title: "Codex32",
    codex32_share_title: "Share {} of {}",
    codex32_type_back: "Type it back",
    codex32_not_same: "Not the same string",
    codex32_skip: "Skip",
    codex32_result_title: "Strings made",
    codex32_result: "The backup is these strings",
    codex32_split_row: "Split",
    codex32_split_value: "{} of {}",
    codex32_split_none: "none",
    codex32_holds_row: "Holds",
    codex32_holds_seed: "the seed, not the words",
    share_groups_title: "How many groups?",
    share_quorum_title: "How many groups must be present?",
    share_count_title: "How many shares?",
    share_threshold_title: "How many must be present?",
    share_group_of: "Group {} of {}",
    share_source_title: "Random shares from?",
    share_source_row: "Random shares",
    share_random_of: "Random {} of {}",
    share_group: "Group {}",
    share_words_title: "Share {} of {}",
    slip39_result_title: "Shares made",
    slip39_result: "The backup is these shares",
    share_groups_value: "{} of {}",
    xor_add_another: "Add another part",
    xor_combine: "Combine",
    load_source_xor: "Seed XOR parts",
    seedqr_title: "SeedQR",
    compact_title: "CompactSeedQR",
    quiz_start_title: "Verify backup",
    quiz_helper_toggle: "Held by a helper",
    quiz_start: "Start quiz",
    quiz_question: "Which is word {}?",
    quiz_question_short: "Word {} · {}",
    quiz_wrong_title: "Not a match",
    quiz_show_words: "Show the words",
    quiz_retry: "Try again",
    quiz_skip: "Skip quiz",
    quiz_skip_confirm: "Skip anyway",
    quiz_skip_cancel: "Keep going",
    quiz_row: "Quiz",
    quiz_skipped: "skipped",
    quiz_question_row: "Question",
    quiz_questions_row: "Questions",
    quiz_again: "Run again",
    quiz_helper_title: "{} · helper",
    quiz_passed_title: "Backup verified",
    quiz_failed_title: "Not verified",

    // Create wizard (UX.md §7.2).
    create_source_title: "Create from?",
    create_entry_count: "{} of {} · {} bits",
    create_too_short: "Too short",
    create_sanity_random: "Looks random",
    create_sanity_uneven: "Looks uneven",
    create_sanity_unchecked: "Not checked",
    create_rolls_row: "Rolls",
    create_flips_row: "Flips",
    create_digits_row: "Digits",
    create_bits_row: "Bits",
    create_order_row: "Order",
    create_order_counting: "counting",
    create_math_checksum: "Checksum",
    create_math_bits: "{} bits",
    create_math_word: "Word {}",
    create_count_title: "How many words?",
    create_language_title: "Which language?",
    create_dice_title: "Roll {} of {}",
    create_coin_title: "Flip {} of {}",
    create_hex_title: "Hex entropy",
    create_sanity_title: "Sanity check",
    create_sanity_again: "Roll again",
    create_math_row: "The math",
    create_math_title: "The math",
    create_math_entropy: "Entropy",
    create_source_dice: "Dice",
    create_source_coins: "Coin flips",
    create_source_hex: "Hex entropy",
    create_source_cards: "Playing cards",
    create_source_camera: "Camera noise",
    create_source_mix: "Mixed sources",
    create_source_device: "This device",
    create_chi_square: "Chi-square",
    create_longest_run: "Longest run",
    create_run_face: "{} {}",
    create_face_1: "ones",
    create_face_2: "twos",
    create_face_3: "threes",
    create_face_4: "fours",
    create_face_5: "fives",
    create_face_6: "sixes",
    create_stat_normal: "normal for {} rolls",
    create_stat_high: "high for {} rolls",
    create_stat_normal_short: "normal",
    create_stat_high_short: "high",
    create_heads: "Heads",
    create_tails: "Tails",
    create_cards_title: "Card {} of {}",
    create_card_deck: "Deck {}",
    create_already_drawn: "Already drawn",
    dice_procedure_title: "Which procedure?",
    dice_procedure_hashed: "Hashed (Coldcard, SeedSigner)",
    dice_procedure_six_as_zero: "Six as zero, hashed (Keystone)",
    dice_procedure_words: "Words chosen by the dice (BitBox)",
    dice_procedure_rolls: "{} rolls",
    reason_no_words: "no words",
    create_rerolled: "5 and 6 are rolled again",
    dice_procedure_row: "Procedure",
    create_words_row: "Words rolled",
    create_cards_row: "Cards",
    create_camera_title: "Camera noise",
    create_camera_shutter: "Take a picture",
    create_camera_state: "Frame {} of {}",
    create_too_little_variation: "Too little variation",
    create_frames_row: "Frames",
    create_frame_size_row: "Pixels",
    create_distinct_row: "Luma values",
    create_mean_row: "Mean",
    create_variance_row: "Variance",
    create_device_title: "Device randomness",
    create_device_result: "Not checkable",
    create_device_row: "Source",
    create_device_os: "operating system",
    create_device_tee: "trusted execution environment",
    create_device_strongbox: "separate security chip",
    create_trusts_device: "trusts this device",
    create_trust_row: "Trust",
    create_trust_device: "this device's software",
    create_mix_title: "Mix which sources?",
    create_mixed_title: "Mixed",
    create_mixed_result: "Combined",
    create_sanity_again_draw: "Draw again",
    create_sanity_again_take: "Take again",

    // Sign (UX.md §7.3).
    sign_title: "Sign",
    sign_amount: "Amount",
    sign_move: "Moved",
    sign_to: "To",
    sign_review: "Review details",
    sign_output_title: "Output {} of {}",
    sign_output_name: "Output {}",
    sign_warnings_title: "Warnings",
    sign_waiting: "waiting…",
    sign_summary_title: "Review",
    sign_fee: "Fee",
    sign_rate: "Rate",
    sign_change: "Change",
    sign_change_unverified: "not verified",
    sign_inputs_title: "Inputs",
    sign_inputs: "Inputs",
    sign_inputs_ours: "{} yours",
    sign_input_row: "#{}",
    sign_total_in: "Total in",
    sign_multisig: "Multisig",
    sign_cosigners: "Cosigners",
    sign_rbf: "Replaceable",
    sign_locktime: "Locktime",
    sign_confirm_title: "Confirm",
    sign_acknowledge: "Acknowledged",
    sign_hold: "Hold to sign",
    sign_hold_share: "Hold to share nonce",
    sign_result_complete: "Signed",
    sign_result_partial: "Partly signed",
    sign_result_nonce: "Nonce shared",
    sign_session_row: "Session",
    sign_session_open: "open",
    sign_session_ends_row: "Ends",
    sign_session_ends: "power off, lock, or signing",
    sign_signatures: "Signatures",
    sign_file_row: "File",
    sign_saved_as: "saved as {}",
    sign_not_saved: "not saved",
    sign_qr_title: "Transaction",
    sign_qr_label: "signed transaction",
    sign_qr_animated: "Animated",
    sign_qr_dense: "too dense",
    sign_no_key: "No key loaded",
    sign_keys_not_in: "Not for these keys",
    sign_key_row: "Key",
    sign_keys_row: "Keys",
    sign_parse_error_title: "Not a transaction",
    sign_read_row: "Read",
    sign_not_psbt: "not a PSBT",
    sign_multisig_value: "{} of {} · {} signed",
    sign_sign_with: "Sign with",
    sign_then_with: "Then with",
    sign_failed: "Signing failed",
    sign_txid: "Transaction id",
    sign_signature: "Signature",
    sign_signatures_title: "Signatures",
    sign_sig_valid: "valid",
    sign_sig_invalid: "invalid",
    sign_sig_unchecked: "not checked \u{b7} {}",
    sign_sig_no_prevout: "no previous output",
    sign_sig_unsupported: "unsupported script",
    sign_sig_unknown_key: "no key in the script",
    sign_sig_deterministic: "deterministic \u{b7} {}",
    sign_sig_not_deterministic: "not deterministic",
    sign_nonce_bip340: "BIP-340",
    warn_signature_invalid: "{} did not sign this transaction, on input {}",
    warn_nonce_reuse: "two signatures of {} share one nonce; the key follows from them",
    warn_nondeterministic: "{} signed input {} with a nonce this device cannot reproduce",
    warn_signature_unchecked: "a signature on input {} was not checked \u{b7} {}",
    warn_inscription_label: "Inscription",
    warn_inscription: "input {} \u{b7} {}",
    warn_inscription_unknown: "no content type",
    tools_compare: "Compare transactions",
    compare_tx_first: "First",
    compare_tx_second: "Second",
    compare_tx_read_second: "Read the second transaction",
    compare_tx_same: "Same transaction",
    compare_tx_different: "Different",
    compare_tx_transaction: "Transaction",
    compare_tx_input: "Input {}",
    compare_tx_output: "Output {}",
    compare_tx_signing: "Signing",
    compare_tx_signed_first: "{} signed only the first",
    compare_tx_signed_second: "{} signed only the second",
    compare_tx_final_first: "final only in the first",
    compare_tx_final_second: "final only in the second",
    compare_tx_changed: "{} \u{2192} {}",
    compare_tx_differs: "differs",
    cmp_version: "Version",
    cmp_locktime: "Locktime",
    cmp_inputs: "Inputs",
    cmp_outputs: "Outputs",
    cmp_spends: "Spends",
    cmp_sequence: "Sequence",
    cmp_amount: "Amount",
    cmp_script: "Script",
    cmp_previous_tx: "Previous transaction",
    cmp_spent_output: "Spent output",
    cmp_redeem_script: "Redeem script",
    cmp_witness_script: "Witness script",
    cmp_sighash: "Sighash",
    cmp_key_origins: "Key origins",
    cmp_tap_key_origins: "Taproot key origins",
    cmp_internal_key: "Internal key",
    cmp_merkle_root: "Merkle root",
    cmp_leaf_scripts: "Leaf scripts",
    cmp_unknown: "Unknown fields",
    cmp_proprietary: "Proprietary fields",
    cmp_xpubs: "Extended public keys",
    warn_output: "Output",
    warn_sighash_label: "Signature",
    warn_change_spoof: "does not match the key",
    warn_high_fee: "{} % of the amount",
    warn_dust: "below the dust limit",
    warn_sighash: "not the whole transaction",
    warn_mixed_scripts: "mixed script types",
    warn_network: "another chain",
    warn_no_key: "none can sign",
    warn_unknown_path: "not a loaded account",
    warn_absurd_rate: "very high",
    warn_locktime: "set",
    warn_non_standard: "non-standard script",
    warn_address_reuse: "used twice",
    warn_utxo_mismatch: "inconsistent",
    warn_missing_utxo: "no amount",
    warn_unsupported: "cannot be signed",
    warn_musig_not_registered: "MuSig2 wallet not registered",
    warn_threshold_not_registered: "Threshold wallet not registered",
    warn_threshold_other_tx: "The file is for another transaction",
    warn_threshold_other_share: "The file is for another key",
    warn_threshold_signer_set: "The file names another set of keys",
    warn_threshold_nonce: "A nonce does not match its record",
    warn_threshold_partial_sig: "A signature does not verify",
    warn_threshold_no_file: "No file was read for this transaction",
    warn_threshold_no_nonces: "The file is for another transaction",
    warn_threshold_not_a_signer: "This key is not one of the signers",
    warn_threshold_chosen: "The keys chosen are not the number that must sign",
    warn_musig_nonce_replaced: "Nonce replaced \u{00b7} earlier signatures dropped",
    warn_blocked: "Blocked",
    warn_amount_unverified: "amount not checked",
    warn_timelock_not_met: "not met: {}",

    // Scanner (UX.md §4).
    scan_title: "Scan",
    scan_title_seed: "Scan a seed",
    scan_title_backup: "Scan a backup",
    scan_title_psbt: "Scan a transaction",
    scan_title_address: "Scan an address",
    scan_looking: "Looking for a QR",
    scan_waiting: "Waiting for the file",
    scan_no_camera: "No camera",
    scan_from_file: "Read a file",
    scan_paste: "Paste",
    scan_type: "Type",
    scan_nothing_to_paste: "Nothing to paste",
    reason_no_clipboard: "no clipboard",
    action_copy: "Copy",
    copy_done: "Copied",
    copy_none: "No clipboard",
    scan_reason_secret: "a secret",
    scan_refused_title: "Refused",
    scan_refused_result: "Not pasted",
    row_clipboard: "Clipboard",
    build_type_title: "Cosigner key",
    tool_policy_keys: "Keys",
    tool_use_loaded_key: "Use a loaded key",
    scan_as_hashes: "Hash it",
    scan_as_encodings: "Show encodings",

    // Files (§5 Menu).
    files_title: "Files",
    files_none: "No files",
    files_bytes: "{} bytes",
    files_kb: "{} KB",
    scan_ur_progress: "Part {} of {}",
    scan_error_title: "Not usable",
    scan_reason_seedqr: "not a SeedQR",
    scan_reason_type: "unsupported type",
    scan_reason_backup: "unsupported backup",
    scan_reason_policy: "not a wallet policy",
    scan_unknown_title: "Unknown QR",
    scan_as_psbt: "Read as a transaction",
    scan_as_text: "Read as text",
    scan_words_title: "Words as plain text",
    scan_words_load: "Load",

    // Signed messages (UX.md F7 and G4).
    scan_title_message: "Scan a message",
    scan_title_signed: "Scan a signed message",
    msg_check_row: "Verify a message",
    msg_title: "Message",
    msg_format_row: "Format",
    msg_bip137: "BIP-137",
    msg_bip322: "BIP-322",
    msg_bip322_nested: "native segwit only",
    msg_qr_title: "Signature",
    msg_qr_label: "signature",
    msg_qr_one_code: "one code",
    msg_signed_title: "Signed",
    msg_valid_title: "Valid",
    msg_invalid_title: "Invalid",
    msg_checked_row: "Checked as",
    msg_message_row: "Message",
    msg_not_signed: "not a signed message",

    // Verify › address (UX.md §7.4).
    verify_entry_title: "Check an address",
    verify_title: "Address",
    wallet_check: "Check an address",
    verify_yours_title: "Yours",
    verify_not_found_title: "Not found",
    verify_searched: "Searched",
    verify_search_depth: "{} addresses per wallet",
    verify_invalid_title: "Not an address",
    verify_network_title: "Wrong network",
    verify_check_another: "Check another",

    // A scanned descriptor, key or text.
    inspect_descriptor: "Descriptor",
    inspect_xpub: "Public key",
    inspect_text: "Text",
    inspect_empty: "Nothing to show",
    inspect_length_row: "Length",
    inspect_length: "{} characters",
    inspect_checksum: "Checksum",
    inspect_checksum_ok: "valid",
    inspect_checksum_bad: "invalid",
    inspect_swap: "Differs from the wallet in use",
    inspect_swap_replaced: "Key {} replaced by {}",
    inspect_swap_no_origin: "Key {} replaced by a key with no origin",
    inspect_swap_same: "Key {}, same fingerprint, different key",
    inspect_swap_claims: "Claims to be {} and is not",
    wallet_title: "Wallet",
    wallet_name: "Name",
    wallet_set_name: "Set a name",
    wallet_single: "single-sig",
    wallet_kind_multisig: "multisig",
    wallet_kind_miniscript: "miniscript",
    wallet_kind_tree: "taproot tree",
    wallet_kind_taproot_multisig: "taproot multisig",
    wallet_kind_recovery: "recovery",
    wallet_recovery: "Recovery",
    wallet_recovery_now: "Now",
    wallet_recovery_after: "After {} days",
    wallet_recovery_value: "{} of {} · {} blocks",
    wallet_key_path: "Key path",
    wallet_unspendable: "unspendable",
    wallet_quorum: "{} of {}",
    wallet_musig: "MuSig2",
    wallet_threshold: "FROST",
    wallet_kind_threshold: "FROST",
    wallet_musig_keys: "{} keys",
    wallet_kind_row: "Kind",
    wallet_miniscript: "Miniscript",
    wallet_tree: "Taproot tree",
    wallet_spend_path: "Path {}",
    wallet_spend_path_row: "Spend path",
    wallet_path_key: "Key {}",
    wallet_path_keys: "Keys {}",
    wallet_path_list: "{}, {}",
    wallet_path_last: "{} and {}",
    wallet_path_after: "{} after {}",
    wallet_path_after_only: "After {}",
    wallet_path_anyone: "Anyone",
    wallet_path_keypath: "Key path",
    wallet_lock_blocks: "{} blocks \u{00b7} {}",
    wallet_lock_seconds: "{} intervals of 512 seconds \u{00b7} {}",
    wallet_lock_height: "block {}",
    wallet_lock_preimage: "a preimage",
    wallet_about_day: "about 1 day",
    wallet_about_days: "about {} days",
    wallet_about_year: "about 1 year",
    wallet_about_years: "about {} years",
    wallet_use: "Add this wallet",
    wallet_sign: "Sign a transaction",
    wallet_sign_message: "Sign a message",
    wallet_in_use: "In use",
    wallet_origin_unknown: "origin unknown",
    wallet_forget: "Forget",
    wallet_load: "Scan a wallet",
    scan_title_wallet: "Scan a wallet",
    wallet_keep_row: "Kept on this device",
    wallet_keep_title: "Keep this wallet on the device?",
    wallet_keep_no: "No",
    wallet_keep_yes: "Yes",
    wallet_keys: "Keys",
    wallet_key: "Key",
    wallet_member: "Key {} of {}",

    // FROST wallets (`crate::threshold`).
    threshold_count_title: "How many keys?",
    threshold_record_title: "Group record",
    threshold_file_name: "frost-{}.txt",
    threshold_confirm_title: "New wallet",
    threshold_confirm_wallet: "Wallet",
    wallet_export: "Export",
    export_policy: "Policy",
    export_bsms: "BSMS record",
    export_bsms_signer: "BSMS key record",
    key_account_row: "Account key",
    account_which_title: "Which account?",
    account_legacy: "Legacy",
    account_nested: "Nested SegWit",
    account_segwit: "SegWit",
    account_taproot: "Taproot",
    account_multisig_nested: "Nested multisig",
    account_multisig_segwit: "SegWit multisig",
    export_key_title: "Key export",
    bsms_token_title: "Session token",
    bsms_token_none: "None",
    bsms_description_title: "Description",
    bsms_description_long: "at most 80 characters",

    key_bip85_row: "BIP-85",
    bip85_which_title: "Which application?",
    bip85_app_words: "Words",
    bip85_app_wif: "WIF",
    bip85_app_xprv: "Extended private key",
    bip85_app_hex: "Hex",
    bip85_app_base64: "Password (base64)",
    bip85_app_base85: "Password (base85)",
    bip85_bytes_title: "How many bytes?",
    bip85_bytes_row: "{} bytes",
    bip85_length_title: "Length",
    bip85_length_range: "{} to {}",
    bip85_value_title: "{} · {}",
    export_core_import: "Bitcoin Core import",
    core_rescan_title: "Rescan from?",
    core_rescan_start: "The start",
    core_rescan_start_value: "finds old coins, slow",
    core_rescan_now: "Now",
    core_rescan_now_value: "a new wallet",
    core_import_file_name: "{}-core-import.json",
    inspect_bsms_paths: "Paths",
    inspect_bsms_first: "First address",
    inspect_bsms_ours: "This device",
    inspect_bsms_not_ours: "none of these keys",
    scan_reason_bsms_signer: "a signer's key record, not a wallet",
    scan_reason_bsms: "not a record this device reads",
    scan_reason_bsms_encrypted: "an encrypted record",

    // The wallet builder (`crate::build`).
    build_new: "New wallet",
    build_kind_title: "What kind of wallet?",
    build_script_title: "Which script type?",
    build_script_taproot: "Taproot",
    build_keys_title: "Keys · {}",
    build_which_title: "Which key?",
    build_scan_key: "Scan a key",
    build_scan_title: "Scan a key",
    build_threshold_title: "How many must sign?",
    build_threshold_now_title: "How many must sign now?",
    build_threshold_later_title: "How many must sign later?",
    build_now_title: "Now · {}",
    build_later_title: "Later · {}",
    build_later_n_title: "Later {} · {}",
    build_delay_title: "After how long?",
    build_delay_days: "{} days",
    build_delay_blocks: "{} blocks",
    build_delay_type: "Type a number of days",
    build_delay_too_short: "not after {} days",
    build_days_title: "Days",
    build_days_too_many: "at most {} days",
    build_another_path_title: "Another path later?",
    build_another_no: "No",
    build_another_yes: "Yes",
    build_passphrase_title: "Passphrase?",
    build_passphrase_no: "No",
    build_passphrase_yes: "Yes",
    build_passphrase_key_title: "New key with passphrase",
    build_reason_other_path: "on the other path",
    build_discard_title: "Discard this wallet?",
    build_reason_origin: "no origin",
    build_reason_mainnet: "mainnet key",
    build_reason_testnet: "testnet key",
    build_reason_added: "already added",
    build_reason_full: "15 keys",
    build_reason_group_full: "full",
    build_reason_wallet: "a wallet, not a key",
    build_reason_words: "{} words",
    build_reason_not_usable: "not usable",
    build_needs_keys: "Needs {} keys",

    // Tools (UX.md §7.5).
    tools_title: "Tools",
    tools_explorer: "Key explorer",
    tools_word_list: "Word list",
    tools_dice_passphrase: "Dice passphrase",
    tools_hashes: "Hashes",
    tools_encodings: "Encodings",
    tools_descriptor_checksum: "Descriptor checksum",
    tools_convert_key: "Convert key",
    tools_units: "Units",
    tools_miniscript: "Miniscript",
    tools_decode: "Decode a transaction",
    decode_no_key: "no key loaded",
    tool_hashes_title: "Text or hex",
    tool_read_as: "Read as",
    tool_read_auto: "Automatic",
    tool_read_text: "Text",
    tool_read_hex: "Hex",
    tool_length_row: "Length",
    tool_bytes: "{} bytes",
    tool_sha256_row: "SHA-256",
    tool_sha256d_row: "SHA-256d",
    tool_hash160_row: "HASH160",
    tool_encoding_title: "Value",
    tool_encoding_row: "Encoding",
    tool_base58check: "Base58Check",
    tool_base58: "Base58",
    tool_bech32: "Bech32",
    tool_bech32m: "Bech32m",
    tool_hex: "Hex",
    tool_not_an_encoding: "Not an encoding",
    tool_version_row: "Version",
    tool_hrp_row: "Prefix",
    tool_checksum_row: "Checksum",
    tool_checksum_valid: "valid",
    tool_checksum_wrong: "wrong",
    tool_checksum_missing: "missing",
    tool_descriptor_title: "Descriptor",
    tool_descriptor_row: "Descriptor",
    tool_computed_checksum_row: "Computed",
    tool_key_title: "Extended key",
    tool_bip32_row: "BIP-32",
    tool_network_row: "Network",
    tool_depth_row: "Depth",
    tool_child_row: "Child number",
    tool_public_only: "Public keys only",
    tool_not_a_key: "Not an extended key",
    tool_units_title: "Units",
    tool_policy_title: "Policy",
    tool_policy_script: "Script",
    tool_policy_segwit: "SegWit",
    tool_policy_taproot: "Taproot",
    tool_policy_load: "Load as wallet",
    tool_from: "From",
    tool_sats: "sats",
    tool_btc: "BTC",
    tool_mbtc: "mBTC",
    tool_bits: "bits",

    // Tools > Word list (UX.md §7.5).
    wordlist_by_title: "Find a word by?",
    wordlist_by: "Search by",
    wordlist_by_word: "Word",
    wordlist_by_number: "Number",
    wordlist_by_binary: "Binary",
    wordlist_by_hex: "Hex",
    wordlist_search_title: "Word list",
    wordlist_browse: "Browse the list",
    wordlist_out_of_range: "The list has 2048 words",
    wordlist_hex_out_of_range: "The last word is 7ff",
    wordlist_no_match: "No word starts like that",
    word_number_row: "Number",
    word_index_row: "Index",
    word_binary_row: "Binary",
    word_hex_row: "Hex",
    word_language_row: "Language",

    // Tools > Dice passphrase (UX.md §7.5).
    dice_list_title: "Which list?",
    dice_list_large: "Long list",
    dice_list_large_detail: "7776 words · five dice",
    dice_list_short1: "Short list",
    dice_list_short1_detail: "1296 words · four dice",
    dice_list_short2: "Short list 2",
    dice_list_short2_detail: "1296 words · 3 edits apart",
    dice_words_title: "How many words?",
    dice_words_bits: "{} words · {} bits",
    dice_roll_title: "Roll {} of {}",
    dice_roll_count: "{} of {} · {} bits",
    dice_result_title: "Passphrase",
    dice_entropy_row: "Entropy",
    dice_bits: "{} bits",
    dice_discard_title: "Discard the rolls?",

    // Explore (UX.md §7.5).
    explore_title: "Explore",
    explore_using_title: "Which key?",
    explore_typed: "Typed words",
    explore_type_words: "Words",
    explore_section_words: "Words and bits",
    explore_section_encodings: "Encodings",
    explore_passphrase_set: "set",
    explore_fingerprint: "Fingerprint",
    explore_discard_title: "Discard words?",
    explore_keep: "Keep",
    explore_discard: "Discard",
    explore_entropy: "Entropy",
    explore_checksum: "Checksum bits",
    explore_seed: "Seed",
    explore_master_xprv: "Master private key",
    explore_path: "Path",
    explore_path_empty: "Two slashes in a row.",
    explore_path_unfinished: "Type an index after the slash.",
    explore_path_not_a_number: "A level is a number, with h for hardened.",
    explore_path_too_large: "The largest index is 2147483647.",
    explore_path_too_deep: "At most ten levels.",
    explore_account_xpub: "Account key",
    explore_slip132: "SLIP-132",
    explore_addresses: "Addresses",
    explore_level_xpub: "Extended public key",
    explore_level_xprv: "Extended private key",
    explore_purpose_group: "Purpose",
    explore_chain_group: "Chain",
    explore_no_slip132: "only for Nested and SegWit",

    // Settings and the lock screen (UX.md §7.7).
    settings_title: "Settings",
    settings_network: "Network",
    settings_unit: "Unit",
    settings_unit_sat: "sats",
    settings_unit_btc: "BTC",
    settings_lock_now: "Lock now",
    settings_lock_after: "Auto-lock",
    settings_wipe_after: "Auto-wipe",
    settings_never: "never",
    settings_camera_rotation: "Camera rotation",
    settings_camera_rotation_0: "0°",
    settings_camera_rotation_90: "90°",
    settings_camera_rotation_180: "180°",
    settings_camera_rotation_270: "270°",
    settings_scramble: "Shuffle PIN pad",
    settings_nonce: "Nonce",
    settings_nonce_low_r: "Low R",
    settings_nonce_first: "First",
    settings_schnorr: "Schnorr",
    settings_schnorr_deterministic: "Deterministic",
    settings_schnorr_fresh: "Fresh randomness",
    settings_memory_key: "Memory key",
    settings_memory_key_ok: "ok",
    settings_memory_key_weak: "weak",
    settings_about: "About",
    settings_version: "Version",
    settings_tier: "Tier",
    settings_boot: "Verified boot",
    settings_core_hash: "Core hash",
    settings_core_hash_none: "not recorded",
    settings_selftest: "Self-test",
    settings_selftest_passed: "passed · {} checks",
    settings_selftest_failed: "failed · {}",
    settings_selftest_run: "Run again",
    settings_wipe_row: "Wipe all keys",
    settings_wipe_hold: "Hold to wipe",
    settings_wipe_stored: "removed from this device",
    settings_wiped_title: "Keys removed",
    settings_exit_row: "Wipe and exit",
    settings_exit_title: "Wipe and exit",
    settings_exit_hold: "Hold to wipe and exit",
    lock_title: "Enter your PIN",
    lock_wrong: "Wrong PIN · {} left before wipe",

    // A key kept on the device (`keep.rs`).
    keep_row: "Keep on this device",
    keep_row_kept: "Kept on this device",
    keep_title: "Keep on this device",
    keep_hold: "Hold to keep",
    keep_not_now: "Not now",
    keep_unlocked_by: "Unlocked by",
    keep_unlocked_by_value: "PIN and secure hardware",
    keep_wrong: "Wrong PIN · {} left before removal",
    keep_stored_row: "Stored key",
    keep_removed_title: "Stored key removed",
    keep_duress_row: "Duress PIN",
    keep_secure_row: "Secure hardware",
    keep_secure_strongbox: "StrongBox",
    keep_secure_tee: "TEE",

    // Self-test (UX.md §7.1).
    selftest_title: "Self-test",
    selftest_failed_title: "Self-test failed",
    selftest_check_row: "Check",
    selftest_exit: "Exit",
    boot_refused_title: "Boot not verified",
    boot_refused_system: "System",
    boot_refused_system_value: "not the one the manufacturer signed",
    boot_refused_fix: "To run OpenSigner",
    boot_refused_fix_value: "lock the bootloader and reinstall",
    boot_refused_exit: "Exit",

    // Assurance tiers (`docs/PLANNING.md` §3).
    tier_a_badge: "Tier A · airgapped",
    tier_b_badge: "Tier B · phone",
    tier_c_badge: "Tier C · desktop",
    tier_d_badge: "Tier D · browser",
    tier_a_badge_short: "A · airgapped",
    tier_b_badge_short: "B · phone",
    tier_c_badge_short: "C · desktop",
    tier_d_badge_short: "D · browser",
    tier_a_badge_letter: "A",
    tier_b_badge_letter: "B",
    tier_c_badge_letter: "C",
    tier_d_badge_letter: "D",
    tier_a: "Tier A · airgapped signer",
    tier_b: "Tier B · secure phone",
    tier_c: "Tier C · desktop",
    tier_d: "Tier D · browser",
    tier_a_statement: "Runs OpenSigner and no other software. No network connection. The key exists in memory only and is gone when the device shuts down. The board does not support secure boot, so the software is only what the card you flashed holds.",
    tier_b_statement: "The key is wrapped by a secure element, and every attempt to open it costs the device lock or a biometric plus a PIN. A copy of the phone's storage is useless without the element. This holds while the phone's operating system is intact: a locked bootloader, not rooted.",
    tier_c_statement: "The operating system and any other software on it can read this app's memory, log what you type, and record the screen. Nothing is stored between runs.",
    tier_d_statement: "The page host and the operating system can read what you enter and what the page holds in memory. Memory may reach disk. Nothing is stored.",
    tier_title: "Tiers",
    tier_this_device: "This device · {}",
    no_secure_boot_title: "This device",
    no_secure_boot_result: "No secure boot",
    no_secure_boot_device_row: "Secure boot",
    no_secure_boot_device: "Not supported by this device",
    no_secure_boot_continue: "Continue",
    // Silent payments (`docs/PLANNING.md` §16.113).
    wallet_silent: "Silent payments",
    wallet_kind_silent: "silent",
    silent_address_row: "Address",
    silent_address_label: "Silent payment",
    silent_form_title: "Which form?",
    silent_form_address: "Address",
    silent_form_uri: "Payment URI",
    silent_labels_row: "Labels",
    silent_label: "Label {}",
    silent_add_label: "Add a label",
    silent_labels_full: "20 labels",
    silent_check_row: "Check a payment",
    silent_read: "Read a transaction",
    silent_previous_row: "Previous transaction",
    silent_paid: "Paid",
    silent_not_paid: "Not paid here",
    silent_no_inputs: "No input this device can read a key from",
    silent_unsigned: "This transaction is not signed yet",
    silent_key_not_loaded: "This wallet's key is not loaded",
    silent_output_row: "Output {}",
    silent_output_label: "Label {}",
    silent_output_plain: "Address",
    export_silent_scan: "Scan descriptor",
    export_silent_uri: "Payment URI",
    export_silent_dns: "DNS record",
    silent_user_title: "User name",
    silent_domain_title: "Domain",
    silent_address_value_row: "Address",
    verify_silent_title: "A silent payment address; it can be shown, not checked",
    // Learn (UX.md §3, the L jobs).
    learn_try_dice: "Roll dice for a key",
    learn_try_verify: "Verify a backup",
    learn_try_encrypted: "Make an encrypted backup",
    learn_try_passphrase: "Open a passphrase",
    learn_try_miniscript: "Miniscript tool",
    learn: &osk_learn::EN,
};
