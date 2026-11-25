//! Faraday's English words for the common area.

macro_rules! words_common {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    /// Power-off sheet's first line on a PC stick that started with Secure Boot off or unread.
    power_secure_boot_off: "Turn Secure Boot back on in the BIOS before starting Windows.",
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
    stick_kind_qr_codes: "QR codes in a PNG",
    /// Text in `stick_kind`.
    stick_kind_file: "File",
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
    /// Error in `send_error`: the codec made no code of it.
    qr_send_code_failed: "No QR code could be made of it",
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
    /// Text in `part_note`. `{}`: `what`, `have`, `total`, `list` joined by ", ", `more`.
    part_note_of_missing: "{}: {} of {} · missing {}{}",
    /// `what` in `part_note` for Specter's numbered text parts.
    part_note_numbered: "numbered parts",
    // ----- secret_text (faraday-core/src/lib.rs) -----
    /// Text in `secret_text`.
    secret_text_those_are: "Those are a seed's words. Seeds go in through Add a key, not Files",
    /// Text in `secret_text`.
    secret_text_private_key: "That is a private key. Faraday takes keys as words or a SeedQR, in Add a key",
    /// Text in `secret_text`.
    secret_text_file_holds: "That file holds a seed or a private key. Those go in through Add a key, not Files",
    /// Scanner note in `secret_text`: a Nostr secret key (`nsec1…`) read.
    secret_text_nostr: "That is a Nostr secret key. Faraday does not take one",
    /// Scanner note in `secret_text`: 12 to 24 words of a BIP-39 list that are no English BIP-39 seed.
    secret_text_not_a_seed: "Those are seed words, but not an English BIP-39 seed: the checksum fails",
    /// Scanner note in `secret_text`: 24 words that are an LND aezeed by its own checksum.
    secret_text_aezeed: "That is an LND aezeed. Lightning takes it",
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
    storage_event_seed_not_copied: "{}: reads as a secret, not copied",
    /// Text in `storage_event`. `{}`: `name`.
    storage_event_wrote_read: "Wrote {}, read back and matched",
    /// Text in `storage_event`. `{}`: `name`, `wrote_as`.
    storage_event_wrote: "Wrote {} as {}, read back and matched",
    /// A visit's line: a file came in under a name Files already holds.
    /// `{}`: its name on the stick, then the name it is kept under.
    storage_event_kept_as: "{} is in Files already: this one kept as {}",
    /// Files' line when the words and passphrase files are too many to
    /// pair. `{}`: how many pairs were left, then how many were tried.
    inbox_pairs_untried: "{} words and passphrase file pairs not tried, past the first {}",
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
    /// Text in `storage_event`, where the code scanned is a GPG key's paperkey, which has no smaller parts.
    gpg_restore_too_fine: "Too fine for this camera: bring it closer, or type the paperkey",
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
    /// Text in `toast_out`: one file. `{}`: its name, `self.medium.for_the()`.
    toast_out_waits: "{} waits {}",
    /// Text in `toast_out_files`: more than one file. `{}`: how many, `self.medium.for_the()`.
    toast_out_files_wait: "{} files wait {}",
    // ----- pull_what (faraday-core/src/lib.rs) -----
    /// Text in `pull_what`.
    pull_what_load_wallet: "load the wallet",
    /// Text in `pull_what`.
    pull_what_unlock: "unlock",
    /// Text in `pull_what`.
    pull_what_make_a_key: "make a key",
    /// Text in `pull_what`: a backup flow's Create {vault}, the key made
    /// after it.
    pull_what_create_vault: "make a new vault, then your key",
    /// Text in `pull_what`.
    pull_what_add_a_key: "add a key",
    /// Text in `pull_what`.
    pull_what_scan: "scan",
    /// Text in `pull_what`.
    pull_what_load_samples: "load the samples",
    /// The Lock sheet's last section when the lock is a mode change.
    lock_mode_keeps_nothing: "A mode change keeps nothing but the mode: Files, For the stick, keys and wallets are emptied.",
    // ----- start_spend (faraday-core/src/lib.rs) -----
    /// Toast in `start_spend`.
    start_spend_not_a_psbt: "Not a PSBT",
    /// Toast in `start_spend` for a PSBT with silent payment fields.
    start_spend_silent_payments: "Not signed: this PSBT sends to silent payments, which this device does not sign",
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
    /// Text in `wallet`: the name of a wallet whose file named it a secret.
    wallet_default_name: "Wallet",
    /// Text in `act`.
    act_restored_wallet: "Restored wallet",
    /// Text in `act`.
    act_its_shares: "its shares",
    /// Text in `act`.
    act_signed_psbt: "Signed PSBT",
    /// Text in `act`.
    act_psbt_for_the_next: "PSBT for the next signer",
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
    // ----- backup_seed_list_of (faraday-core/src/lib.rs) -----
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
    /// Text in `plan_store`.
    plan_store_other_key_order: "The shares plan kept for this wallet has its keys in another order. Load the wallet with its keys in that order to change the plan.",
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
    /// Text in `public_pictures`. `{}`: `held` joined by " · ".
    public_pictures_holds: "Holds {}",
    /// Text in `public_pictures`. `{}`: `off` joined by " · ".
    public_pictures_leaves: "Leaves off {}",
    /// Text in `public_pictures`.
    public_pictures_not_wallet: "Not a wallet on its own",
    /// Text in `public_pictures`.
    public_pictures_public: "Public: spends nothing",
    // ----- public_out (faraday-core/src/lib.rs) -----
    /// Toast in `public_made` and a vault's save: the split sheets,
    /// rebuilt as Restore rebuilds them, give other addresses than the
    /// wallet's; none is written.
    backup_sheets_rebuild_another: "These split sheets rebuild another wallet: none written",
    /// Toast in `public_made`: the same for the multisig config.
    backup_config_rebuilds_another: "This multisig config rebuilds another wallet: not written",
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
    /// Text in `create_pass_lock`. `{}`: `key.label`.
    create_pass_lock_passphrase: "{} · passphrase",
    // ----- create_make (faraday-core/src/lib.rs) -----
    /// Text in `create_make`. `{}`: `self.session.wallets.len() + 1`.
    create_make_new_wallet: "New wallet {}",
    /// Text in `create_make`.
    create_make_made_here: "Made here",
    /// Head of `screens::create::child_notice`, after a wallet is made with
    /// a key recorded as a BIP-85 child seed.
    create_child_head: "A BIP-85 child seed",
    /// Text in `child_notice`. `{}`: the key's fingerprint.
    create_child_body: "Faraday records that {} is a BIP-85 child seed only inside a vault. Nothing written for its backup says where it came from. To remember that, write it down yourself.",
    /// Action in `child_notice`.
    create_child_ok: "OK",
    // ----- make_pdf (faraday-core/src/qr_view.rs) -----
    /// Text in `make_pdf`. `{}`: `stem(name)`.
    make_pdf_pdf: "{}.pdf",
    /// Toast in `make_pdf`. `{}`: `e`.
    make_pdf_not_printed: "Not printed: {}",
    // ----- scanned (faraday-core/src/lib.rs) -----
    /// Text in `scanned`.
    scanned_not_a_psbt: "Not a PSBT, a descriptor, an xpub or a signed message",
    /// Toast in `scanned`. `{}`: `name`.
    scanned_is_a_cosigner: "{} is a cosigner's key",
    /// Scanner note in `scanned_entries`: a TOTP setup code or vault entries read by the Spend tab's intake.
    scanned_totp_not_here: "That is a TOTP setup code · not used here",
    /// Toast in `scanned`. `{}`: `name`, `slot + 1`.
    scanned_is_key: "{} is Key {}",
    /// Toast in `scanned`. `{}`: `name`.
    scanned_is_in_files: "{} is in Files",
    /// Toast in `spend_backup_arrived`. `{}`: the file's name.
    spend_backup_psbt_not_this_spend: "{} is for another transaction. Cancel this spend to sign it; the file is in Files",
    // ----- copy_scanned (faraday-core/src/lib.rs) -----
    /// Note in `copy_scanned`.
    copy_scanned_not_seedqr: "Not a SeedQR",
    // ----- read_code (faraday-core/src/lib.rs) -----
    /// Text in `read_code`.
    read_code_is_seedqr: "That is a SeedQR. Seeds go in through Add a key, not Files",
    /// Text in `read_code`.
    read_code_that_may: "That may be a CompactSeedQR. Seeds go in through Add a key, not Files",
    /// Text in `read_code`: a paperkey read, filed nowhere.
    read_code_is_paperkey: "That is a paperkey. Restore a GPG key takes it at Type the paperkey, not Files",
    /// Text in `read_code`. `{}`: `ur_type`.
    read_code_ur_unknown: "ur:{} is not a kind this build reads",
    // ----- ur_feed (faraday-core/src/lib.rs) -----
    /// Note in `ur_feed`: a part of another UR message started it again.
    ur_feed_new_qr: "A new QR started; reading it from the start",
    /// Note in `ur_feed`: a part no decoder reads.
    ur_feed_not_a_part: "That QR is not a readable UR part",
    // ----- address_note (faraday-core/src/lib.rs) -----
    /// Text in `address_note`.
    address_note_that_address_fails: "That address fails its checksum: a character in it is wrong",
    /// Text in `address_note`. `{}`: the address's network, then the wallet's, each as `wallet::network_name` names it, in lower case.
    address_note_wrong_network: "A {} address; this wallet is {}",
    /// Text in `address_note`.
    address_note_change: "change",
    /// Text in `address_note`.
    address_note_receive: "receive",
    /// Text in `address_note`: a `bitcoin:` link with a `req-` parameter.
    address_note_uri_required: "That payment link needs something this device does not read",
    /// Text in `address_note`: a `bitcoin:` link that is not well formed.
    address_note_uri_malformed: "That payment link is not well formed",
    /// Title of Check an address's page.
    address_check_title: "Check an address",
    /// Label of Check an address's field.
    address_check_field: "Address",
    /// Button on Check an address: scan the address.
    address_check_scan: "Scan",
    /// Under Check an address's field: the text is no Bitcoin address.
    address_check_not_bitcoin: "Not a Bitcoin address",
    /// Under Check an address's field: the text is another kind of code.
    address_check_not_an_address: "Not an address",
    /// Result line of the address check when the address is one of a loaded wallet's. `{}`: the wallet's name.
    address_check_found: "Found in {}",
    /// Result line of the address check when the address is one of a loaded key's standard accounts. `{}`: the key's name.
    address_check_found_key: "Found in key {}",
    /// Result line of the address check when the address is not found.
    address_check_missed: "Not found",
    /// The address check's page with neither a key nor a wallet loaded.
    address_check_load_first: "Load a key or a wallet first",
    /// The address check's result: the address's label.
    address_check_address: "Address",
    /// The address check's result: the place's label.
    address_check_place: "Place",
    /// The address check's result: a receive address. `{}`: the index.
    address_check_receive: "Receive · index {}",
    /// The address check's result: a change address. `{}`: the index.
    address_check_change: "Change · index {}",
    /// The address check's result: the wallet's shape label.
    address_check_wallet: "Wallet",
    /// The address check's result: a key account's label.
    address_check_account: "Account",
    /// The address check's result: a key account. `{}`: the script's name, the account number.
    address_check_account_value: "{} · account {}",
    /// The address check's result: the path's label.
    address_check_path: "Path",
    /// The address check's result: a multisig wallet's keys' label.
    address_check_keys: "Keys",
    /// The address check's result: a key and its path to the address. `{}`: a fingerprint, a path.
    address_check_key_path: "{} · {}",
    /// The address check's result: how far a miss looked, label.
    address_check_searched: "Searched",
    /// The address check's result: how far a miss looked. `{}`: the depth, a wallet's name or a count below.
    address_check_searched_value: "The first {} receive and change addresses of {}",
    /// The address check's result: how many wallets were searched. `{}`: the count.
    address_check_wallets: "{} wallets",
    /// The address check's result: one wallet searched among key accounts.
    address_check_wallet_one: "1 wallet",
    /// The address check's result: how many key accounts were searched. `{}`: the count.
    address_check_accounts: "{} key accounts",
    /// The address check's result: one key account searched.
    address_check_account_one: "1 key account",
    /// The address check's result: wallets and key accounts searched. `{}`: the wallets, the key accounts, each as counted above.
    address_check_wallets_accounts: "{} and {}",
    /// Check an address's button that opens the menu of text files in Files, and that menu's title.
    address_check_file: "From a file",
    /// Check an address's menu in place of a file that is empty, over 64 KiB or not text.
    address_check_not_text: "Not a text file",
    /// Check an address's line for a file that holds a descriptor.
    address_check_descriptor: "A descriptor, not an address",
    /// Check an address's line for a text file that holds no address.
    address_check_none_in_file: "No address in the file",
    /// Check an address's count over the list of a file's addresses. `{}`: how many are listed, then how many the file holds.
    address_check_read_count: "{} of {} addresses read",
    /// Way over from another scanner's read card of an address to the address check.
    read_go_address: "Check the address",
    /// Button on the read card of an address not found. `{}`: the depth, in indices of each chain.
    read_look_further: "Look further: {}",
    /// Button on the read card while it looks further.
    read_look_stop: "Stop",
    /// Line on the read card while it looks further. `{}`: the index reached, the depth.
    read_look_progress: "Looking: {} of {}",
    // ----- open_qr (faraday-core/src/lib.rs) -----
    /// Toast in `open_qr`. `{}`: `e`.
    open_qr_cannot_make: "Cannot make a code: {}",
    // ----- code_png (faraday-core/src/lib.rs) -----
    /// Toast in `code_png`. `{}`: `e`.
    code_png_cannot_make: "Cannot make a code: {}",
    /// Toast in `code_png`. `{}`: `v.title`.
    code_png_is_more_than: "{} is more than one code",
    // ----- code_view (faraday-core/src/lib.rs) -----
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
    /// Text in `code_view`. `{}`: `w.name`.
    code_view_specter_diy: "{} · Specter DIY addwallet",
    /// Text in `code_view`.
    code_view_public_watch: "Public: watch only, spends nothing",
    /// Error in `code_view`.
    code_view_bip_129_covers: "BIP 129 covers wsh and sh(wsh) multisig keys",
    /// Text in `code_view`. `{}`: `k.fp`, `k.kind`.
    code_view_xpub_kind: "Xpub {} · {}",
    /// Error in `code_view`.
    code_view_that_wallet: "That wallet is no longer loaded",
    /// Title of a receive address's QR sheet. `{}`: the wallet's name, the address's index.
    code_view_receive: "{} · receive {}",
    /// Title of Explore's address's QR sheet. `{}`: the path.
    code_view_explore_address: "Address · {}",
    /// Title of Explore's public key's QR sheet. `{}`: the path.
    code_view_explore_public_key: "Public key · {}",
    /// Title of Explore's extended key's QR sheet. `{}`: the path.
    code_view_explore_xpub: "Extended key · {}",
    /// Title of the Lightning node id's QR sheet and its picture.
    code_view_node_id: "Lightning node id",
    /// The last line of the node id's picture.
    code_view_public_node: "Public: names the node, spends nothing",
    /// Title of a recovery sheet's descriptor's QR sheet. `{}`: the sheet's name.
    code_view_sheet: "{} · recovery sheet descriptor",
    /// Title of an entry's TOTP secret's QR sheet. `{}`: the entry's title.
    code_view_totp: "{} · TOTP secret",
    /// Title of a GPG key's paperkey QR sheet. `{}`: the key's first user ID.
    code_view_paperkey: "{} · Paperkey",
    /// Error in `code_view`: the wallet gives no address there.
    code_view_no_address: "No address to show",
    /// Error in `code_view`: no Lightning node key is on screen.
    code_view_no_node: "No node key on show",
    /// Error in `code_view`: no recovery sheet is selected.
    code_view_no_sheet: "No recovery sheet is selected",
    /// The Lightning node id's button: its labelled picture, for the stick.
    lightning_png: "PNG",
    /// The node's kind on its picture, for a node read from an aezeed.
    code_view_node_lnd: "LND",
    /// The label of a Core Lightning node's code and picture when its source is a file or typed words.
    code_view_node_cln: "Core Lightning",
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
    /// Receipt line in `text_seed_answer`: the picture stays a file. `{}`: its name.
    qr_read_text_left: "{}: left as a file",
    /// Title of the sheet for a picture whose code reads as a seed or as text. `{}`: the file's name.
    text_seed_title: "{} reads as a seed or as text",
    /// Action of that sheet.
    text_seed_load: "Load as a key",
    /// Action of that sheet.
    text_seed_leave: "Leave as a file",
    /// Toast in `qr_read`. `{}`: `got`.
    qr_read_is_in_files: "{} is in Files",
    /// Text in `qr_read`. `{}`: `name`, `got`.
    qr_read_read_into_files: "{} read into Files as {}",
    /// Text in `qr_read`.
    qr_read_read: "read",
    // ----- visit_inbox (faraday-core/src/lib.rs) -----
    /// Text in `visit_inbox`.
    visit_inbox_png_no_qr: "A PNG with no QR code read",
    /// Text in `visit_inbox`. `{}`: `screens::kind_name(k)` in lower case.
    visit_inbox_png_kind: "A PNG of {}",
    /// Text in `visit_inbox`.
    visit_inbox_a_file: "A file Faraday does not read",
    /// Text in `visit_inbox`.
    visit_inbox_whoever_has_it_holds: "Whoever has it holds the key: they can spend its coins or sign as it",
    /// Text in `visit_inbox`.
    visit_inbox_whoever_has_enough: "Whoever has enough parts holds the key",
    /// Text in `visit_inbox`.
    visit_inbox_vault_warning: "Whoever has it reads every password and code in it",
    /// Text in `visit_inbox`.
    visit_inbox_whoever: "Whoever has it reads it",
    // ----- sign_here (faraday-core/src/lib.rs) -----
    /// Text in `sign_here`. `{}`: `waiting` joined by ", ".
    sign_here_backup_first: "{} waits for its backup: finish the backup first",
    /// Text in `sign_here`.
    sign_here_this_transaction: "This transaction was signed here before with other amounts for its inputs. It is refused.",
    /// Toast in `sign_here`.
    sign_here_this_device: "This device's nonce is on the PSBT: carry it to the other signers",
    // ----- collect (faraday-core/src/lib.rs) -----
    /// Error in `collect`. `{}`: `name`.
    collect_has_nothing: "{} has nothing this copy lacks",
    /// Error in `collect`.
    // ----- primary (faraday-core/src/lib.rs) -----
    /// Error in `primary`. `{}`: `pos + 1`.
    primary_finish_step: "Finish step {} first",
    // ----- event (faraday-core/src/lib.rs) -----
    /// Line in `scan_sheet` after `CameraUnavailable`.
    event_no_camera: "No camera, or the camera was refused",
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
    // ----- chart_edit (faraday-core/src/chart_edit.rs) -----
    /// Toast in `chart_key_checked`.
    chart_edit_checked: "Checked today",
    /// Toast in `chart_checked`.
    chart_edit_not_current: "This is not the current copy. Open the current copy to mark it checked.",
    /// Toast in `chart_notes_change`.
    chart_edit_keys_order: "The plan cannot be changed with the keys in this order.",
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
    /// A TOTP row's badge while the clock is not known.
    codes_set_the_time_first: "Set the time first",
    /// Title in `draw`.
    codes_totp_codes: "TOTP codes",
    /// Line in `compact`.
    codes_no_totp_entries: "No entries with a TOTP secret",
    /// The codes screen's search bar's placeholder.
    codes_search: "Search codes",
    /// Line in place of the rows when no entry's title holds what the search bar holds.
    codes_search_none: "No codes match",
    /// Button in `compact`.
    codes_scan_a_setup: "Scan a setup code",
    /// Button in `compact`.
    codes_add_an_entry: "Add an entry",
    // ----- flow (faraday-core/src/flow.rs) -----
    /// Text in `flow::container`. `{}`: `i + 1`, the step count.
    flow_step_count: "Step {} of {}",
    /// Text in `flow::container` where the step count can change. `{}`: `i + 1`.
    flow_step_number: "Step {}",
    /// Heading of a flow's Your choices (`flow::choices`).
    flow_your_choices: "Your choices",
    /// A row of Your choices that has no answer yet (`flow::choices`).
    flow_not_chosen_yet: "Not chosen yet",
    /// Text in `flow::container`.
    flow_home: "Home",
    /// Button in `compact`: leaves the flow for good (F51).
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
    // ----- held (faraday-core/src/held.rs) -----
    /// Toast in `backup_settle`. `{}`: `names` joined by ", ".
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
    /// Text in `xpub_make`. `{}`: `fp.map(fp_text).unwrap_or_else(|| EN.…`, `crate::spend_backup::kind_name(kind)`.
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
    /// Toast in `input_new`, a pointer or touchscreen held until it is used. `{}`: `d.name`.
    inputs_new_device_held: "New device: {}",
    /// Notice in `keyboard_notice`, keyboards held until a key is typed.
    /// `{}`: their names, with commas.
    inputs_keyboard_attached: "Keyboard attached: {}. To use it, type any key on it.",
    /// Toast in `input_boot_usb`, the external USB devices plugged in at
    /// start. `{}`: their names, with commas.
    inputs_plugged_in_at_start: "Plugged in: {}. Only your keyboard and mouse should be plugged in. Unplug anything else.",
    /// The name of a USB device that gave none, in that toast.
    inputs_usb_device_unnamed: "USB device",
    /// Toast in `input_believe`, a keyboard. `{}`: `d.name`.
    inputs_keyboard_in_use: "Keyboard in use: {}",
    /// Toast in `input_believe`, a pointer. `{}`: `d.name`.
    inputs_pointer_in_use: "Pointer in use: {}",
    /// Toast in `input_believe`, a device that is both. `{}`: `d.name`.
    inputs_keyboard_pointer_in_use: "Keyboard and pointer in use: {}",
    /// Toast in `input_believe`, a touchscreen. `{}`: `d.name`.
    inputs_touchscreen_in_use: "Touchscreen in use: {}",
    /// Toast in `input_typed`. `{}`: `name`.
    inputs_wrong_code: "{} typed the wrong code. Unplug it to try again.",
    /// Toast in `input_tapped`. `{}`: `name`.
    inputs_wrong_taps: "{} tapped beside the target. Unplug it to try again.",
    /// Toast in `input_tapped`. `{}`: `name`.
    inputs_wrong_presses: "{} pressed beside the target. Unplug it to try again.",
    /// Toast in `toast_ignored`, with no input trusted.
    inputs_new_devices_ignored: "New keyboards, pointers and touchscreens are ignored until Faraday starts again.",
    /// Toast in `toast_ignored`.
    inputs_new_keyboards_ignored: "New keyboards and touchscreens are ignored until Faraday starts again.",
    // ----- restore (faraday-core/src/restore.rs) -----
    /// Error in `merge`.
    restore_not_a_multisig: "not a multisig config",
    /// Error in `merge`.
    restore_these_shares: "these shares are of different wallets",
    /// A split sheet that names one key twice. `{}`: its fingerprint.
    restore_key_twice: "a share names key {} twice",
    /// Error in `merge`.
    restore_no_shares: "no shares",
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
    /// The overflow menu of a compact top app bar too full for its ?:
    /// what the ? opens, named as the ?'s tooltip names it.
    bar_menu_guide: "Guide",
    /// The overflow menu's keyboard, while the keyboard is put away.
    bar_menu_show_keyboard: "Show keyboard",
    /// The overflow menu's keyboard, while the keyboard is up.
    bar_menu_hide_keyboard: "Hide keyboard",
    /// The overflow menu's eye on Make codex32 shares, while the sheets
    /// are masked.
    bar_menu_show_sheets: "Show sheets",
    /// The overflow menu's eye on Make codex32 shares, while the sheets
    /// are shown.
    bar_menu_hide_sheets: "Hide sheets",
    /// A destination's badge past 999 things waiting (Material's most).
    nav_badge_many: "999+",
    // ----- Check (faraday-core/src/check/screen.rs) -----
    /// A group on Check.
    check_group_wallet: "The wallet",
    /// A group on Check.
    check_group_signing: "Signing",
    /// A group on Check.
    check_group_setup: "Setup and backups",
    /// A group on Check.
    check_group_messages: "Messages and keys",
    /// The mark on a Check row that works from the seed.
    check_seed: "Seed",
    /// A Check row's line.
    check_seed_line: "Its words, its passphrase and the wallet app",
    /// A Check row's line.
    check_multisig_line: "The wallet's descriptor or setup file",
    /// A Check row's line.
    check_address_line: "The address and the wallet loaded here",
    /// A Check row's line.
    check_transaction_line: "The PSBT, as a file or a QR code",
    /// A Check row's line.
    check_signed_line: "The PSBT sent and the one returned",
    /// A Check row's line.
    check_signatures_line: "A signed PSBT or transaction",
    /// A Check row's line.
    check_dice_line: "The rolls and the words made from them",
    /// A Check row's line.
    check_backup_line: "Shares, parts, a SeedQR, codex32 or a Krux code",
    /// A Check row's line.
    check_message_line: "The message, its signature and the address",
    /// A Check row's line.
    check_child_line: "The parent seed and the child's index",
    /// A Check row's line.
    check_xpub_line: "An extended public key in any spelling",
    /// A form in the backup menu.
    check_menu_slip39: "SLIP-39 shares",
    /// A form in the backup menu.
    check_menu_xor: "Seed XOR parts",
    /// A form in the backup menu.
    check_menu_seedqr: "SeedQR",
    /// A form in the backup menu.
    check_menu_codex32: "codex32",
    /// A form in the backup menu.
    check_menu_krux: "Krux code",
    // ----- start (faraday-core/src/screens/wallets_start.rs) -----
    /// Title in `start`.
    start_wallets: "Wallets",
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
    // ----- nonce check of a transaction brought in (faraday-core/src/screens/nonce_check.rs) -----
    /// Title of `Screen::NonceCheck`.
    nonce_screen_title: "Nonce check",
    /// Question heading the pane of `Screen::NonceCheck` as the flow's one card (`decode/nonce_flow.rs`).
    nonce_pane_question: "Choose an input",
    /// Heading of the pane's group holding the file or scan the check opened from.
    nonce_pane_file: "Checked",
    /// Heading of the pane's group of inputs.
    nonce_pane_inputs: "Inputs",
    /// First row of the pane: every input's signatures.
    nonce_pane_all: "All inputs",
    /// Second line of a pane row. `{}`: the number of signatures on the input, `{}`: its verdict.
    nonce_pane_line: "{} · {}",
    /// Pane row's signatures: one.
    nonce_pane_one_signature: "1 signature",
    /// Pane row's signatures: not one. `{}`: how many.
    nonce_pane_signatures: "{} signatures",
    /// Second line of an input row with no signature.
    nonce_pane_no_signature: "No signature",
    /// Verdict of a pane row whose input shares a nonce with another input.
    nonce_pane_reused: "Nonce reused",
    /// Verdict of a pane row whose input's key was used before, on another transaction.
    nonce_pane_used_before: "Nonce used before",
    /// Verdict of a pane row whose input does not match the output it spends.
    nonce_pane_mismatch: "Does not match",
    /// The card's line when the chosen input has no signature.
    nonce_card_input_none: "No signature on this input",
    /// Line in `Screen::NonceCheck` for a PSBT that is not finished.
    nonce_screen_more_wanted: "Not finished: more signatures are wanted",
    /// Line in `Screen::NonceCheck` when a signature is wrong.
    nonce_screen_refused: "Not finished: a signature is wrong",
    /// Line in `Screen::NonceCheck` when a key's nonce was used before on another transaction.
    nonce_screen_refused_used: "Not finished: a nonce was used before",
    /// Why Spend has no finished transaction, nothing to save or show as a code: a signature reuses a nonce, in this transaction or held in an open vault.
    spend_nonce_held: "Not finished: a signature reuses a nonce",
    /// Line in `Screen::NonceCheck` when a PSBT has an input, final or not, whose output cannot be read or is stated falsely.
    nonce_screen_refused_unread: "Not finished: an input's output cannot be read",
    /// Line in `Screen::NonceCheck` when a PSBT has an input, final or not, whose `witness_utxo` and previous transaction state different outputs.
    nonce_screen_refused_disagree: "Not finished: an input states an output its previous transaction does not",
    /// The nonce check's refusal line when only the final fields of inputs fail the script they spend.
    nonce_screen_refused_fields: "Not finished: final fields do not satisfy the script they spend",
    /// The nonce check's refusal line when only signing data (a `scriptSig` or witness) does not match its output.
    nonce_screen_refused_witness: "Not finished: signing data does not match its output",
    /// Headline of `nonce_check` when some signatures could not be checked. `{}`: how many were not checked, how many there are, `nonce_check_signature(s)`.
    nonce_unchecked_head: "Nonce check · {} of {} {} not checked",
    // ----- catalog (faraday-core/src/screens/catalog.rs) -----
    /// Title in `catalog_screen`.
    catalog_tools: "Tools",
    /// Text in `catalog_screen`.
    /// Line in `catalog_screen`.
    catalog_no_tool_matches: "No tool matches",
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
    /// Question over the wallets in `MessageSteps::pick`.
    message_which_wallet: "Which wallet signs?",
    /// Reason of a dimmed row in `MessageSteps::pick`: a multisig wallet.
    message_single_key_only: "Single-key wallets only",
    /// Reason of a dimmed row in `MessageSteps::pick`: its key is not loaded.
    message_key_not_held: "Its key is not loaded",
    /// Text in `message_screen`.
    message_sign_a_message: "Sign a message",
    /// Text in `message_body`. `{}`: `m.index`.
    message_receive_address: "Receive address 0/{}",
    /// Text in `message_body`. `{}`: the change address's index.
    message_change_address: "Change address 1/{}",
    /// Text in `Session::sign_message_at`: the key behind a found address is
    /// no longer loaded. `{}`: the key's fingerprint.
    message_key_not_loaded: "Key {} is not loaded here",
    /// Text in `Session::sign_message_at`: the key cannot derive the found
    /// address's path. `{}`: the key's fingerprint.
    message_key_cannot_derive: "Key {} cannot derive this path",
    /// Text in `Session::sign_message_at`: the key gives another address at
    /// the path. `{}`: the key's fingerprint.
    message_key_not_this_address: "Key {} does not give this address at its path",
    /// Text in `message_body`.
    message_type_the_message: "Type the message",
    /// Text in `message_body`.
    message_press_type: "Press Type or Scan to give the message",
    /// Text in `message_body`.
    message_stop_typing: "Stop typing",
    /// Text in `message_body`.
    message_type: "Type",
    /// Button in `message_body`: a QR code of the message.
    message_scan: "Scan",
    /// Sign a message's scanner.
    message_scan_title: "Scan the message",
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
    /// The button that shows a file, a message, a code or a key as a QR code, wherever one is offered.
    show_as_qr: "Show as QR",
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
    /// Check a message's verdict, or Sign a message's error: the signature does not hold.
    message_error_invalid: "The signature does not verify for this address and this message",
    /// The same: not base64, or not a signature of its format.
    message_error_malformed: "The signature is not base64 or not a signature of its format",
    /// The same: a script address (multisig, P2WSH) or a future witness version.
    message_error_unsupported_address: "Messages for this kind of address are not signed or checked here",
    /// The same: the format cannot express this address form.
    message_error_unsupported_format: "This format cannot sign for this kind of address",
    /// The same: a BIP-322 full-variant signature.
    message_error_unsupported_variant: "Faraday does not check this kind of signature",
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
    // ----- set_time (faraday-core/src/screens/set_time.rs) -----
    /// Title in `set_time_screen`.
    set_time_set_the_time: "Set the time",
    /// Text in `set_time_compact`.
    set_time_now: "Now",
    /// Section label in `set_time_compact`.
    set_time_date: "Date",
    /// Section label in `set_time_compact`.
    set_time_time: "Local time",
    /// The name of UTC, after a time read in it.
    set_time_utc: "UTC",
    /// Value of the Time zone row while no zone is chosen.
    set_time_zone_not_set: "Not set",
    /// Label of the offset steppers under the list of zones.
    set_time_other_offset: "Other offset",
    /// The result of Set both ways. `{}`: the local date and time, the
    /// zone's offset, the date and time in UTC.
    set_time_result: "{} UTC{} is {} UTC",
    /// Text in `set_time_compact`.
    set_time_set: "Set",
    /// Text in `set_time_compact`.
    set_time_keep_button: "Keep this time",
    /// Text in `set_time_compact`.
    set_time_back: "Back",
    // ----- secrets (faraday-core/src/secrets.rs) -----
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
    secrets_whoever_has: "Whoever has it can sign as key {}, and with {} more {} can spend",
    /// Text in `offer_seed`.
    secrets_key: "key",
    /// Text in `offer_seed`.
    secrets_keys: "keys",
    /// Text in `offer_seed`.
    secrets_a_wallet_s: "A wallet's seed",
    /// Toast in `secret_to_vault`.
    secrets_no_vault_open: "No vault is open",
    /// Toast in `secret_to_vault`. `{}`: `out.name`, `vault`.
    secrets_saved_to_vault: "{} is in {}",
    /// Text in `secret_unprotected`. `{}`: `name`, `self.medium.for_the()`.
    secrets_waits_unprotected: "{} waits {}, unprotected",
    /// Text in `vault_stick_save_write`: more than one file, one of them a seed. `{}`: how many, `self.medium.for_the()`.
    secrets_wait_unprotected: "{} files wait {}, unprotected",
    // ----- secureboot (faraday-core/src/secure_boot/mod.rs) -----
    /// Toast in `sb_enrol`.
    secureboot_this_computer: "This computer's clock is not known",
    /// A Secure Boot key could not be made, read or used.
    secureboot_error_key: "The key could not be made or read",
    /// A file to sign or check is not a PE image.
    secureboot_error_image: "Not an EFI image this build reads",
    /// A file to sign is not the boot program a release's sums list.
    secureboot_error_not_release: "Not the boot program the release's sums list",
    /// Toast in `sb_enrol`. `{}`: `n`, `self.medium.for_the()`.
    secureboot_enrolment: "{} enrolment files wait {}",
    /// Toast in `sb_sign`. `{}`: `name`, `self.medium.for_the()`.
    secureboot_signed: "{}, signed, waits {}",
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
    /// Text in `sb_check`: a signature by an algorithm or with signers this check does not read.
    secureboot_signed_not_read: "Signed in a way this check does not read",
    /// Line in `images_panel` and toast in `sb_sign`: the release's `SHA256SUMS` or `SHA256SUMS.asc` is not in the Inbox (F134).
    secureboot_no_release_sums: "No signed sums from a Faraday release are beside this file",
    /// Line in `images_panel` and toast in `sb_sign`: the sums' signature does not verify with a release key (F134).
    secureboot_sums_not_signed: "The release's sums are not signed by Faraday's release key",
    /// Line in `images_panel` and toast in `sb_sign` (F134). `{}`: the release's version.
    secureboot_not_release_boot: "This file is not the boot program of release {}",
    /// Line in `images_panel`: the file is the release's own (F134). `{}`: the release's version.
    secureboot_release_boot: "The boot program of release {}, checked against its signed sums",
    /// Line in `images_panel` and toast in `sb_sign`: the file does not read as an EFI image (F134).
    secureboot_not_an_image: "This file is not an EFI image",
    /// Line in `images_panel` and toast in `sb_sign`: a genuine release older than the Faraday running (F134). `{}`: the release's version, then this Faraday's.
    secureboot_older_release: "Release {} is older than this Faraday ({}); sign a release at least as new",
    // ----- set_time (faraday-core/src/set_time.rs) -----
    /// Text in `now_local_text`.
    set_time_not_known: "Not known",
    /// Under Set the time's Now, while the shell gives no clock reading.
    set_time_no_reading: "No clock reading",
    /// Under Set the time's Now, once a time is set with no clock reading.
    set_time_counted_from_set: "Counted from Set: no clock reading",
    /// Toast in `set_time_set`. `{}`: `h`, `mi`.
    set_time_done_toast: "Time set to {}:{}",
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
    // ----- visit_taken (faraday-core/src/stick/visit_taken.rs) -----
    /// Text in `line`. `{}`: `self.name`, `keep_as`.
    visit_taken_keep_both_n: "{} · Keep both: {}",
    /// Text in `taken_sheet`. `{}`: `app.medium.on_the()`.
    visit_taken_already: "Already {}",
    /// Text in `taken_sheet`.
    visit_taken_replace: "Replace",
    /// Text in `taken_sheet`.
    visit_taken_keep_both: "Keep both",
    /// Text in `taken_sheet`.
    visit_taken_cancel: "Cancel",
    /// Why a vault was not written, in the visit's receipt: the person
    /// answered the name-taken sheet with Cancel.
    visit_taken_name_taken: "its name is taken",
    // ----- modes (faraday-core/src/modes/mod.rs) -----
    /// Text in `Mode::name`.
    mode_demo: "Practice",
    /// What a press on an action Practice refuses answers, and a tile that needs a private key there.
    practice_refused: "Not available in Practice: this is an online computer",
    /// The pill in Practice: the mode and its network.
    mode_demo_pill: "Practice · Testnet",
    /// Title of the mode sheet.
    mode_sheet_title: "Mode",
    /// Button in the mode sheet.
    mode_not_now: "Not now",
    /// Title of the sheet that asks before a mode change; the mode pressed.
    mode_change_title: "Change to {}?",
    /// The button of that sheet that locks into the mode pressed.
    mode_lock_and_use: "Lock and use {}",
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
    /// Check a signer's scanners' second tile, which hands over a sample signed by a key that is not loaded.
    scan_sample_signed_elsewhere: "Load sample PSBT, signed elsewhere",
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
    /// The scanner's tile that hands over a sample Krux code.
    scan_sample_krux: "Load sample Krux code",
    /// The scanner's tile that hands over a sample address.
    scan_sample_address: "Load sample address",
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
    /// The search field's placeholder on the BIP-39 list.
    wordlist_search_bits: "Word, number or bits",
    /// The search field's placeholder on an EFF list.
    wordlist_search_dice: "Word, number or dice",
    /// Beside the search field while it holds a search. `{}`: how many
    /// words it finds.
    wordlist_found: "{} found",
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

    // ----- A ⋮ on every file; Rename; Details (docs/FLOWS.md decision 61), faraday-core -----
    /// A file's ⋮ menu: the popup to rename it.
    file_menu_rename: "Rename",
    /// A file's ⋮ menu: the sheet of its name, kind, size and SHA-256.
    file_menu_details: "Details",
    /// A file on the stick visited: its ⋮ menu's way to the Delete files
    /// sheet with it alone chosen. `{}`: "stick", "card or stick".
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
    // ----- Rename stick: the data partition's name (docs/STICK-IDENTITY.md §6), faraday-core -----
    /// The visit's button and its dialog's title. `{}`: "stick", "card or stick".
    stick_name_button: "Rename {}",
    /// The line under the field when what is typed is written otherwise:
    /// in capitals, as FAT keeps a name. `{}`: the name as written.
    stick_name_written_as: "Written as {}",
    /// The line under the field when it is empty.
    stick_name_none: "No name",
    /// Refused: longer than FAT's 11 characters.
    stick_name_long: "At most 11 characters",
    /// Refused: a character a FAT name cannot hold.
    stick_name_character: "Letters, digits, spaces and ! # $ % & ' ( ) - @ ^ _ ` { } ~ only",
    /// Refused, here and by the disk process: a boot partition's name.
    stick_name_boot: "EFI, bootfs and FARADAYBOOT are boot partitions' names",
    /// The visit's line once named. `{}`: "stick", the name.
    stick_name_done: "This {} is named {}",
    /// The visit's line once its name is taken away. `{}`: "stick".
    stick_name_removed: "This {} has no name",
    /// The visit's line: refused. `{}`: why.
    stick_name_not_done: "Not renamed: {}",
    /// What a computer shows the stick as cannot be changed here: the
    /// desktop app's sticks are folders.
    storage_name_folder: "a folder has no volume name to change",
    /// The shell's answer was not the disk process's answer to a naming.
    storage_answer_name: "an answer that is not a naming",
    // ----- Files: the medium the device started from (docs/STICK-IDENTITY.md §5.2), faraday-core -----
    /// Files' line on a PC when two sticks of the running build were in
    /// at start: neither is the device's own, and no settings are kept.
    files_boot_two: "Two sticks of this build are in",
    /// Files' line on a PC when the stick it started from was not found.
    files_boot_not_found: "The stick Faraday started from was not found",
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
    /// The Delete files sheet's button when only a vault's key file is
    /// chosen: a hold.
    stick_wipe_hold_delete_key_file: "Hold to delete a vault's key file",
    /// The Delete sheet's hold for the free space (F131).
    stick_wipe_hold_zero_free: "Hold to zero the free space",
    /// The Delete sheet's hold for the whole partition (F131).
    stick_wipe_hold_zero_partition: "Hold to zero the whole partition",
    // ----- Plain tooltips on icon-only controls (docs/DESIGN.md §4.12), faraday-ui and faraday-core -----
    /// Tooltip on the way back, an icon alone.
    tip_back: "Back",
    /// The top app bar's line under the page's name: where its way back
    /// lands (DESIGN §4.1, F104, F146). `{}`: that page's or step's
    /// name, shown alone where this would be cut.
    bar_back_to: "Back to {}",
    /// Tooltip on the top app bar's ?.
    tip_guide: "Guide",
    /// Tooltip on the ⋮ that opens a menu.
    tip_more: "More",
    /// Tooltip on the Lock icon.
    tip_lock: "Lock",
    /// Tooltip on the top app bar's power button, and its menu's title.
    tip_power: "Power",
    /// Tooltip on the keyboard icon.
    tip_keyboard: "Keyboard",
    /// The top bar's mark while a wireless receiver is attached.
    tip_wireless_keyboard: "Wireless keyboard",
    /// Tooltip on the power icon.
    tip_power_off: "Power off",
    /// Line under a passphrase field once a key of a hardware keyboard typed
    /// into it.
    keys_read_as_us: "Keys are read as a US keyboard. Show the passphrase once to check it.",
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
    // ----- What faraday-storage refuses, worded here (docs/STRUCTURE-PASS.md §7) -----
    /// A download that found no free file name; the name.
    storage_no_free_name: "no free name for {}",
    /// Print on a device with no printing.
    storage_no_print: "this device does not print",
    /// Reading a computer file on a device with none.
    storage_no_host_read: "this device does not read this computer's files",
    /// Saving a download on a device with no downloads folder.
    storage_no_download: "this device does not save downloads",
    /// The disk process gave no answer in time.
    storage_disk_silent: "the disk process does not answer",
    /// A request made while a vault is being written or read, which the
    /// disk process would answer only after it.
    storage_stick_busy: "the stick is busy",
    /// The disk process answered with bytes that do not decode.
    storage_answer_malformed: "a malformed answer",
    /// The disk process answered a picture read with another answer.
    storage_answer_codes: "an answer that is not codes",
    /// The disk process answered a listing with another answer.
    storage_answer_listing: "an answer that is not a listing",
    /// The disk process answered a read with another answer.
    storage_answer_file: "an answer that is not a file",
    /// The disk process answered a write with another answer.
    storage_answer_write: "an answer that is not a write",
    /// The disk process answered a delete with another answer.
    storage_answer_delete: "an answer that is not a delete",
    /// The disk process answered a space query with another answer.
    storage_answer_space: "an answer that is not a stick's space",
    /// The disk process answered a zeroing step with another answer.
    storage_answer_zeroing: "an answer that is not zeroing",
    /// The disk process answered a rename with another answer.
    storage_answer_rename: "an answer that is not a rename",
        }
    };
}
