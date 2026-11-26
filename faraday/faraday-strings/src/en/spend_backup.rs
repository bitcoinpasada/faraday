//! Faraday's English words for the spend backup area.

macro_rules! words_spend_backup {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- spend_backup (faraday-core/src/spend_backup/mod.rs) -----
    /// Text in `WORDS_SOURCE`.
    spend_backup_typed_words: "Typed words",
    /// Text in `spend_button`.
    spend_backup_finish_the_backup: "Finish the backup first",
    /// Text in `spend_button`. `{}`: `s.signers.len()`, `self.spend_needed()`.
    spend_backup_carry_on_spend: "Carry on the spend · Signatures {} of {}",
    /// Text in `spend_button`. `{}`: `self.medium.noun()`.
    spend_backup_sign_the_psbt: "Sign the PSBT from the {}",
    /// Text in `spend_button`.
    spend_backup_spend_from_wallet: "Spend from this wallet",
    /// Text in `spend_backup_act`.
    spend_backup_finished_transaction: "Finished transaction",
    /// Note in `spend_backup_intake_seed`.
    spend_backup_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    /// Text in `spend_backup_intake_seed`. `{}`: `n + 1`.
    spend_backup_key_already: "Key {} · already here",
    /// Text in `spend_backup_intake_file`.
    spend_backup_wallet_description_read: "Descriptor read · a wallet is open already",
    /// Text in `spend_backup_intake_file`.
    spend_backup_wallet_description: "Descriptor read",
    /// Text in `spend_backup_intake_file`. `{}`: `k`.
    spend_backup_part_read_rebuilt: "Part {} read · the wallet is rebuilt",
    /// Text in `spend_backup_intake_file`. `{}`: `k`, `m.have.len()`, `m.n`.
    spend_backup_part_read_progress: "Part {} read · {} of {} keys in hand",
    /// Text in `spend_backup_intake_file`.
    spend_backup_a_public_key: "A public key: it shows the wallet but cannot sign",
    /// Text in `spend_backup_intake_file`.
    spend_backup_a_transaction: "A transaction to sign · kept for later",
    /// Text in `spend_backup_intake_file`. `{}`: `name`.
    spend_backup_is_in_files: "{} is in Files",
    /// Note in `spend_backup_intake_again`. `{}`: `fp_text(fp)`.
    spend_backup_seed_read: "Seed {} read",
    /// Text in `spend_backup_no_money`. `{}`: `label`.
    spend_backup_no_money: "{} · no money",
    /// Text in `spend_backup_seed_with`.
    spend_backup_passphrase_inline: "passphrase",
    /// Text in `spend_backup_seed_with`. `{}`: `l`.
    spend_backup_passphrase_n: "{} · passphrase",
    /// Text in `spend_backup_words_kind_now`. `{}`: `kind_name(*k)`.
    spend_backup_words: "Words · {}",
    /// Text in `spend_backup_rebuild`.
    spend_backup_restored_wallet: "Restored wallet",
    /// Text in `spend_backup_rebuild`.
    spend_backup_its_shares: "its shares",
    /// Error in `spend_backup_rebuild`. `{}`: `m.have.len()`, `m.n`.
    spend_backup_of_keys_in_hand: "{} of {} keys in hand",
    /// Text in `kind_name`.
    spend_backup_native_segwit_kind: "native SegWit",
    /// Text in `kind_name`.
    spend_backup_taproot: "Taproot",
    /// Text in `kind_name`.
    spend_backup_nested_segwit_kind: "nested SegWit",
    /// Text in `kind_name`.
    spend_backup_legacy: "legacy",
    /// Text in `kind_label`.
    spend_backup_native_segwit: "native SegWit multisig",
    /// Text in `kind_label`.
    spend_backup_taproot_multisig: "Taproot multisig",
    /// Text in `kind_label`.
    spend_backup_nested_segwit: "nested SegWit multisig",
    /// Text in `kind_label`.
    spend_backup_legacy_multisig: "legacy multisig",
    // ----- spend_backup (faraday-core/src/spend_backup/screen.rs) -----
    /// Text in `draw`. `{}`: `app.session.wallets[i].name`.
    spend_backup_spend_from_n: "Spend from {}",
    /// Text in `draw`.
    spend_backup_wallets: "Wallets",
    /// Text in `loaded_compact`.
    spend_backup_walk_through: "Walk through from the start",
    /// Text in `loaded_compact`.
    spend_backup_no_wallet_loaded: "no wallet loaded",
    /// Text in `loaded_compact`. `{}`: `s.spend.source`, `name`.
    spend_backup_signing_wallet: "Signing {} · {}",
    /// Button in `loaded_compact`.
    spend_backup_continue_signing: "Continue signing",
    /// Section label in `loaded_compact`.
    spend_backup_wallets_loaded: "Wallets loaded",
    /// Line in `loaded_compact`.
    spend_backup_none_loaded: "None loaded",
    /// Text in `loaded_compact`. `{}`: `here.min(need)`, `need`.
    spend_backup_can_spend_count: "Can spend · {} of {}",
    /// Text in `loaded_compact`. `{}`: `here`, `if here == 1 { EN.spend_backup_key } else {…`, `need`.
    spend_backup_here_needed: "{} {} here · {} needed",
    /// Text in `loaded_compact`.
    spend_backup_key: "key",
    /// Text in `loaded_compact`.
    spend_backup_keys: "keys",
    /// Text in `loaded_compact`.
    spend_backup_no_key_here: "No key here",
    /// Line in `loaded_compact`.
    spend_backup_an_open_vault: "An open vault holds a missing key · Open it to load it",
    /// Section label in `loaded_compact`.
    spend_backup_keys_without_wallet: "Keys with no wallet loaded",
    /// Text in `loaded_compact`.
    spend_backup_restore_wallet: "Restore its wallet",
    /// Text in `loaded_compact`.
    spend_backup_spend_as_a_single: "Spend as a single-key wallet",
    /// Text in `loaded_compact`.
    spend_backup_home: "Home",
    /// Text in `loaded_compact`.
    spend_backup_spend: "Spend from a backup",
    /// Text in `summary`.
    spend_backup_six_stops: "Six stops",
    /// Text in `summary`.
    spend_backup_desktop_copy: "Desktop copy",
    /// Text in `summary`. `{}`: `app.medium.noun()`.
    spend_backup_running_from_medium: "Running from the {}",
    /// Text in `summary`.
    spend_backup_nothing_in_yet: "Nothing in yet",
    /// Text in `summary`.
    spend_backup_things_one: "1 thing in",
    /// Text in `summary`. `{}`: `n`.
    spend_backup_things_count: "{} things in",
    /// Text in `summary`.
    spend_backup_loaded_in_sparrow: "Loaded in Sparrow",
    /// Text in `summary`.
    spend_backup_money_shows: "Money shows",
    /// Text in `summary`.
    spend_backup_done_in_sparrow: "Done in Sparrow",
    /// Text in `summary`. `{}`: `s.spend.source`.
    spend_backup_signing: "Signing {}",
    /// Text in `summary`.
    spend_backup_not_here_yet: "Not here yet",
    /// Text in `summary`. `{}`: `app.medium.for_the()`.
    spend_backup_nothing_waits: "Nothing waits {}",
    /// Text in `summary`. `{}`: `app.medium.for_the()`.
    spend_backup_1_file: "1 file {}",
    /// Text in `summary`. `{}`: `n`, `app.medium.for_the()`.
    spend_backup_files: "{} files {}",
    /// Text in `summary`.
    spend_backup_after_the_transaction: "After the transaction",
    /// Button in `body`.
    spend_backup_have_psbt_button: "I have the PSBT",
    /// Text in `body`.
    spend_backup_show_the_finished: "Show the finished transaction as QR",
    /// Text in `more`.
    spend_backup_less_about: "Less about this",
    /// Text in `more`.
    spend_backup_more_about: "More about this",
    /// Button in `map`.
    spend_backup_start: "Start",
    /// Line in `holding`.
    spend_backup_bring_in_paper: "Or bring in what is on paper",
    /// Text in `holding`.
    spend_backup_scan_the_qr: "Scan the QR codes you have",
    /// Text in `holding`.
    spend_backup_type_the_words: "Type the words you have",
    /// Text in `holding`. `{}`: `i.name`.
    spend_backup_open: "Open {}",
    /// Text in `holding`. `{}`: `app.medium.a()`.
    spend_backup_copy_files_button: "Copy files in from {}",
    /// Line in `holding`.
    spend_backup_came_in_heading: "What came in",
    /// Text in `holding`.
    spend_backup_rebuild_wallet: "Rebuild the wallet",
    /// Text in `holding`. `{}`: `fp_text(fp)`, `name`.
    spend_backup_seed_not_wallets: "Seed {} is not one of {}'s keys",
    /// Text in `holding`.
    spend_backup_add_its_passphrase: "Add its passphrase",
    /// Text in `came_rows`. `{}`: `t`.
    spend_backup_seed_row: "Seed · {}",
    /// Text in `came_rows`. `{}`: `t`, `label`.
    spend_backup_seed: "Seed · {} · {}",
    /// Text in `came_rows`. `{}`: `wl.name`, `Session::shape(wl)`.
    spend_backup_description_row_shape: "Descriptor · {} · {}",
    /// Text in `came_rows`. `{}`: `name`.
    spend_backup_description_row: "Descriptor · {}",
    /// Text in `came_rows`. `{}`: `sheets`, `m.have.len()`, `m.n`.
    spend_backup_part_of_keys: "Part {} · {} of {} keys in hand",
    /// Error in `came_rows`. `{}`: `sheets`.
    spend_backup_part: "Part {}",
    /// Text in `came_rows`. `{}`: `fp`.
    spend_backup_key_xpub_n: "Key · {} · xpub",
    /// Text in `came_rows`.
    spend_backup_key_xpub: "Key · xpub",
    /// Text in `came_rows`.
    spend_backup_transaction: "Transaction · kept for later",
    /// Line in `question`.
    spend_backup_anything_else: "Anything else in the envelope?",
    /// Text in `question`.
    spend_backup_type_more_words: "Type more words",
    /// Text in `question`.
    spend_backup_that_s_everything: "That's everything",
    /// Line in `question`.
    spend_backup_these_seeds: "These seeds are one wallet with several keys.",
    /// Text in `question`.
    spend_backup_scan_the_description: "Scan the descriptor",
    /// Text in `question`.
    spend_backup_i_can_t_find: "I can't find one",
    /// Text in `question`. `{}`: `s.n`.
    spend_backup_without_description: "Without the description Faraday needs all {} keys: their seeds or xpubs",
    /// Text in `vault`. `{}`: `app.medium.noun()`.
    spend_backup_remove_the_then: "Remove the {}, then type the passphrase",
    /// Text in `vault`. `{}`: `app.medium.for_the()`.
    spend_backup_sealed_here: "Sealed here, {}",
    /// Text in `vault`. `{}`: `app.medium.a()`.
    spend_backup_copied_from_medium: "Copied in from {}",
    /// Text in `vault`. `{}`: `app.vaults.time_text(mem, f.header.co…`.
    spend_backup_unlock_time: "Unlocks in {}",
    /// Line in `vault`.
    spend_backup_passphrase: "Passphrase",
    /// Line in `vault`.
    spend_backup_unlocking: "Unlocking",
    /// Text in `vault`. `{}`: `mib_text(m)`.
    spend_backup_this_computer: "This computer: {} free",
    /// Button in `vault`.
    spend_backup_unlock: "Unlock",
    /// Text in `stick_row`. `{}`: `app.medium.noun()`.
    spend_backup_plug_medium: "Plug the {} in",
    /// Text in `stick_row`. `{}`: `app.medium.a()`.
    spend_backup_faraday_asks: "Faraday asks to lock first when {} goes in",
    /// Line in `opened`.
    spend_backup_spend_from_label: "Spend from",
    /// Text in `wallet_rows`. `{}`: the keys' marker, `Session::shape(wl)`.
    spend_backup_shape_and_keys: "{} · {}",
    /// Text in `opened`. `{}`: `Session::shape(wl)`, `here`, `slots.len()`.
    spend_backup_of_keys_here: "{} · {} of {} keys here",
    /// Line in `shortcut`.
    spend_backup_have_psbt_line: "I have the PSBT from Sparrow ready",
    /// Text in `shortcut`.
    spend_backup_scan_button: "Scan it",
    /// Text in `SHORTCUT`.
    spend_backup_i_have_the_psbt: "I have the PSBT from Sparrow ready: Scan it",
    /// Text in `loaded_in_sparrow`.
    spend_backup_show_the_wallet_qr: "Show the wallet QR",
    /// Line in `loaded_in_sparrow`.
    spend_backup_receive_address: "Receive address 0",
    /// Button in `loaded_in_sparrow`.
    spend_backup_it_is_loaded: "It is loaded",
    /// Line in `check`.
    spend_backup_none_of_these: "None of these show money.",
    /// Line in `check`.
    spend_backup_the_words_may: "The words may belong to a wallet with several keys, or need a different passphrase.",
    /// Text in `check`.
    spend_backup_scan_more: "Scan more",
    /// Text in `check`.
    spend_backup_type_a_passphrase: "Type a passphrase",
    /// Line in `check`.
    spend_backup_is_there_a_word: "Is there a word or sentence kept apart from the list?",
    /// Text in `check`.
    spend_backup_yes_type_button: "Yes, type it",
    /// Text in `check`.
    spend_backup_no_button: "No",
    /// Text in `check`. `{}`: `crate::spend_backup::kind_label(kind)`, `if pass { EN.spend_backup_with_a_passphrase…`, `g.at + 1`, `Faraday::guess_total(g)`.
    spend_backup_trying_progress: "Trying: {}, {} · {} of {}",
    /// Text in `check`.
    spend_backup_with_a_passphrase: "with a passphrase",
    /// Text in `check`.
    spend_backup_no_passphrase: "no passphrase",
    /// Text in `check`.
    spend_backup_show_the_wallet: "Show the wallet as a QR code",
    /// Text in `check`.
    spend_backup_open_online_wallet: "Open this in the wallet on the online computer",
    /// Text in `check`.
    spend_backup_open_this_one: "Open this one on the online computer too",
    /// Text in `check`.
    spend_backup_receive: "Receive",
    /// Text in `check`.
    spend_backup_change: "Change",
    /// Text in `check`.
    spend_backup_show_more_addresses: "Show more addresses",
    /// Text in `check`.
    spend_backup_no_money_shows: "No money shows",
    /// Text in `check`.
    spend_backup_yes_my_money: "Yes, my money shows",
    /// Text in `bring`. `{}`: `name`.
    spend_backup_every_coin_to_wallet: "Every coin to {}",
    /// Line in `bring`.
    spend_backup_amount_max_hint: "Amount: Max, in Sparrow's Send tab",
    /// Text in `bring`.
    spend_backup_scan_the_psbt: "Scan the PSBT",
    /// Text in `bring`.
    spend_backup_scan: "Scan",
    /// Text in `bring`. `{}`: `app.medium.a()`.
    spend_backup_on_medium: "On {}",
    /// Text in `bring`. `{}`: `app.medium.a()`.
    spend_backup_copy_psbt_button: "Copy files in from {}",
    /// Line in `bring`.
    spend_backup_in_files: "In Files",
    /// Line in `bring`.
    spend_backup_signing_this_one: "Signing this one",
    /// Text in `bring`.
    spend_backup_use_this_one: "Use this one",
    /// Text in `bring`. `{}`: `wl.name`, `Session::shape(wl)`.
    spend_backup_spends_from_wallet: "Spends from {} · {}",
    /// Line in `bring`.
    spend_backup_no_wallet_open: "No wallet open here matches this transaction. Change cannot be verified until its wallet is open: check that this is the transaction for this wallet.",
    /// Button in `bring`.
    spend_backup_read_button: "Read it",
    /// Text in `signers`.
    spend_backup_finish_backup_button: "Finish the backup",
    /// Text in `signers`.
    spend_backup_load_from_vault: "Load from vault",
    /// Text in `signers`.
    spend_backup_signs_here: "Signs here",
    /// Text in `signers`.
    spend_backup_not_here: "Not here",
    /// Text in `signers`. `{}`: `f.name`.
    spend_backup_unlock_n: "Unlock {}",
    /// Text in `signers`.
    spend_backup_scan_a_key: "Scan a key",
    /// Text in `signers`.
    spend_backup_collect_a_signature: "Collect a signature",
    /// Text in `signers`. `{}`: `app.medium.noun()`.
    spend_backup_remove_the_first: "Remove the {} first",
    /// Button in `signers`.
    spend_backup_continue: "Continue",
    /// Button in `away`.
    spend_backup_lock: "Lock",
    /// Button in `away`.
    spend_backup_power_off: "Power off",
    /// Text in `away`.
    spend_backup_start_over: "Start over",
    // ----- spend_backup (faraday-core/src/spend_backup/text.rs) -----
    /// Text in `map`.
    spend_backup_paper: "Paper",
    /// Text in `map`.
    spend_backup_what_you_have: "What you have",
    /// Text. `{}`: `m.noun()`.
    spend_backup_the_faraday: "The Faraday {} and its passphrase, or words written down, and often a sheet describing the wallet.",
    /// Text in `map`.
    spend_backup_open_the_wallet_here: "Open the wallet here",
    /// Text. `{}`: `m.noun()`.
    spend_backup_start_a_computer: "Start a computer from the Faraday {} and open the wallet.",
    /// Text in `map`.
    spend_backup_check_the_balance: "Check the balance",
    /// Text in `map`.
    spend_backup_put_the_wallet: "Put the wallet's descriptor into Sparrow on an online computer.",
    /// Text in `map`.
    spend_backup_make_the_transaction: "Make the transaction in Sparrow. It gives you a PSBT.",
    /// Text in `map`.
    spend_backup_offline: "Offline",
    /// Text in `map`.
    spend_backup_sign_it_here: "Sign it here",
    /// Text in `map`.
    spend_backup_bring_the_psbt: "Bring the PSBT to Faraday, read what it does, and sign it.",
    /// Text in `map`.
    spend_backup_online: "Online",
    /// Text in `map`.
    spend_backup_take_the_signed: "Take the signed transaction back to Sparrow and broadcast it.",
    /// Text in `WORDS_ARE_MONEY`.
    spend_backup_the_words_are: "The words are the money. Never photograph them, and never type them into a phone, a website or a password manager. The same goes for the vault's passphrase.",
    /// Text in `UNSURE`.
    spend_backup_not_sure_what: "Not sure what you have?",
    /// Text in `ANYTHING_HINT`.
    spend_backup_another_list: "Another list of words, another QR code, a line starting wsh( or tr(, or a word kept apart from the list.",
    /// Text in `DESCRIPTION_HINT`.
    spend_backup_its_description: "Its descriptor says how they fit together: a QR code labelled wallet, descriptor, setup or multisig; a long line starting wsh( or tr(; or sheets with lines like 73C5DA0A: xpub…",
    /// Text in `looks`.
    spend_backup_a_square_black: "A square black-and-white pattern",
    /// Text in `looks`.
    spend_backup_looks_qr_code: "A QR code: the same information in a form a camera can read. Several numbered 1 of 5 and so on are one thing in parts: scan them one after another. Never scan a QR code that sits beside seed words into anything but Faraday.",
    /// Text in `looks`.
    spend_backup_words_numbered: "Words numbered 1 to 12, 18 or 24",
    /// Text in `looks`.
    spend_backup_the_seed_phrase: "The seed phrase. Often headed recovery phrase, seed words or backup, or stamped into steel.",
    /// Text in `looks`.
    spend_backup_one_long_line: "One long line starting wpkh(, wsh(, tr(, xpub, zpub or tpub",
    /// Text in `looks`.
    spend_backup_a_descriptor: "A descriptor or an xpub. It shows the addresses and the balance but cannot spend. Do not post it publicly: it shows the wallet's whole history.",
    /// Text in `looks`.
    spend_backup_several_sheets: "Several sheets each holding a few lines, like 73C5DA0A: xpub…",
    /// Text in `looks`.
    spend_backup_one_description: "One description split up on purpose, often under a heading such as this sheet carries 2 of 3 wallet descriptor parts. Enough of the sheets, in any order, add up to the whole. A line such as Policy: 2 of 3 says how many signatures the wallet needs.",
    /// Text in `looks`.
    spend_backup_a_word_or_sentence: "A word or sentence kept apart from the list, called a passphrase or a 25th word",
    /// Text in `looks`.
    spend_backup_if_one_exists: "If one exists, the words alone open a wallet that is real but empty, and the money is in the wallet the words and the passphrase open. This is not the vault's passphrase: the vault's passphrase opens the vault file, and this one changes which wallet the words open.",
    /// Text in `looks`.
    spend_backup_a_short_line: "A short line starting bc1, 3 or 1",
    /// Text in `looks`.
    spend_backup_one_address: "One address, not a backup.",
    /// Text in `looks`.
    spend_backup_a_small_device: "A small device with a screen and buttons",
    /// Text in `looks`.
    spend_backup_a_hardware_wallet: "A signer. It is a way of using the words, not a replacement for them.",
    /// Text. `{}`: `m.a_full_cap()`.
    spend_backup_labelled_faraday: "{} labelled Faraday, and a passphrase on a separate card",
    /// Text. `{}`: `m.noun()`.
    spend_backup_looks_medium: "The {} starts the computer and carries the vault, a locked file named like vault.kdbx. The passphrase opens it. The vault holds the wallet and the keys that sign for it.",
    /// Text in `title`.
    spend_backup_how_to_spend: "How to spend bitcoin",
    /// Text in `title`. `{}`: `app.medium.noun()`.
    spend_backup_title_start: "Start from the {}",
    /// Text in `title`.
    spend_backup_what_do_you: "What do you have?",
    /// Text in `title`.
    spend_backup_open_the_wallet: "Open the wallet",
    /// Text in `title`.
    spend_backup_load_the_wallet: "Load the wallet in Sparrow",
    /// Text in `title`.
    spend_backup_title_check_money: "Check the money is there, on the online computer",
    /// Text in `title`.
    spend_backup_title_write_payment: "Write the payment in Sparrow",
    /// Text in `title`.
    spend_backup_check_the_money: "Check the money is really there",
    /// Text in `title`.
    spend_backup_write_the_payment: "Write the payment",
    /// Text in `title`.
    spend_backup_bring_the_transaction: "Bring the transaction here",
    /// Text in `title`.
    spend_backup_put_everything: "Put everything away",
    /// Text in `title`.
    spend_backup_read_the_transaction: "Read the transaction",
    /// Text in `title`.
    spend_backup_the_transaction: "The transaction id",
    /// Text in `title`.
    spend_backup_title_which_way: "Which way it spends",
    /// Text in `title`.
    spend_backup_nonces: "Nonces",
    /// Text in `title`.
    spend_backup_add_the_seeds: "Add the seeds that can sign",
    /// Text in `title`.
    spend_backup_who_signs: "Who signs",
    /// Text in `title`.
    spend_backup_title_sign: "Sign it",
    /// Text in `title`.
    spend_backup_the_other_signatures: "The other signatures",
    /// Text in `title`.
    spend_backup_title_send: "Send it",
    /// Text in `lead`.
    spend_backup_six_stops_three: "Six stops, three on this computer and three on an online one. Nothing moves until the last.",
    /// Text in `lead`. `{}`: `app.medium.noun()`.
    spend_backup_lead_desktop_copy: "The desktop copy of Faraday is for testing. For real money, start a computer from the Faraday {}.",
    /// Text in `lead`. `{}`: `app.medium.noun()`.
    spend_backup_lead_running_offline: "This computer runs Faraday from the {}, with no network.",
    /// Text in `lead`.
    spend_backup_scan_or_type: "Scan or type everything in the envelope, in any order.",
    /// Text in `lead`.
    spend_backup_in_sparrow_file_new: "In Sparrow: File, New Wallet, then scan the wallet QR.",
    /// Text in `lead`.
    spend_backup_in_sparrow_s: "On the online computer, in Sparrow's Send tab: the address, the amount and a fee. Then Create Transaction, Finalize Transaction for Signing and Show QR.",
    /// Text in `lead`.
    spend_backup_words_alone: "Open this in Sparrow and look at its balance. If no money shows, press No money shows.",
    /// Text in `lead`.
    spend_backup_load_this_same: "Load this wallet in Sparrow, then compare its first address with receive address 0 below.",
    /// Text in `lead`. `{}`: `app.medium.a()`, `app.medium.a()`, `app.medium.noun()`.
    spend_backup_two_ways_scanning: "Two ways. Scanning is the simpler: nothing has to lock. Press Scan and hold Sparrow's QR code up to this computer's camera; an animation is read as it loops. Or save the .psbt onto {} and copy it in. Faraday reads {} only while no vault or key is open, so with one open it locks first: it seals the vault, forgets the keys and starts again. Copy the PSBT in, remove the {}, and unlock with the same passphrase. This tab comes back on this page.",
    /// Text in `lead`. `{}`: `app.medium.noun()`, `app.medium.a()`, `app.medium.noun()`, `app.medium.noun()`.
    spend_backup_lock_faraday: "Lock Faraday. The vault is sealed again with the keys inside it, and none is left in memory. If a file you still need waits for the {}, plug {} in and the {} visit writes it. Then switch off, take the {} out, and put it back with the papers.",
    /// Text in `lead`.
    /// Text in `lead`.
    spend_backup_lead_nonces: "Add the others' nonces, or go first and share this device's.",
    /// Text in `lead`.
    spend_backup_lead_path: "Each path lists the keys it needs and how long it waits.",
    /// Text in `lead`.
    spend_backup_lead_collect: "Scan or copy in each cosigner's signed copy.",
    /// Text in `more`.
    spend_backup_more_words_kind: "Words alone do not say which kind of wallet they made. Faraday made the most common kind from them. If no money shows, Faraday makes the next kind.",
    /// Text in `more`.
    spend_backup_more_description: "The code is the wallet's descriptor: enough for Sparrow to find every address the wallet uses and add up what is on all of them. It cannot spend. If the first address matches, two separate programs have read the same backup and agreed. If it does not match, stop: something was typed or scanned wrongly.",
    /// Text in `more`.
    spend_backup_more_psbt_file: "Sparrow can also save the transaction as a .psbt file.",
    /// Text in `more`.
    spend_backup_more_follow_id: "After sending, the id is how to follow the transaction.",
    /// Text in `more`.
    spend_backup_more_other_signatures: "For a wallet that needs signatures from elsewhere, the next card lists who still has to sign and takes their signed copies; each is checked against its key and this transaction before it counts.",
    /// Text in `more`.
    spend_backup_more_safe_to_hand: "The finished transaction is what the network accepts. It is safe to hand to anyone: nobody can change where the money goes without breaking the signatures, and a broken transaction is rejected, not redirected.",
    /// Text in `lead`.
    spend_backup_read_the_summary: "What leaves, what comes back and the fee",
    /// Text in `lead`.
    spend_backup_this_opens_once: "This opens once the transaction is here.",
    /// Text in `lead`.
    spend_backup_lead_write_down_id: "Write down the first and last few characters of this id; Sparrow shows the same id.",
    /// Text in `lead`.
    spend_backup_lead_every_key: "Every key that signs is listed. A key not here yet comes from its own place: Scan a key reads its SeedQR, or its words typed instead, and a vault that holds it unlocks. Faraday works out which key it is. Each is checked against the wallet, so a key from another wallet is refused. A key held by someone else does not have to come here at all: they sign the same PSBT on their own device, and their signed copy is collected after this device signs.",
    /// Text in `lead`.
    spend_backup_lead_one_seed: "One seed at a time: put its words in, add it, and the count goes up. Order does not matter and the same seed twice does no harm. Each list is checked against the wallet's own keys, so one belonging to a different wallet is refused. A seed kept as a QR code beside the words does not have to be typed: scan it. If that backup has a separate passphrase, type it in the passphrase box before adding.",
    /// Text in `lead`.
    spend_backup_signing_adds: "Signing adds the signature of every key loaded here.",
    /// Text in `lead`. `{}`: `app.medium.a()`.
    spend_backup_the_finished: "Show the signed PSBT as a QR code; in Sparrow, on the same transaction, press Scan QR, then Broadcast Transaction. Or carry the finished transaction, in Files, on {}.",
    /// Text in `more`.
    spend_backup_anyone_who_reads: "Anyone who reads the words can spend the money from anywhere in the world, and it cannot be reversed. Where a wallet needs several lists, anyone with enough of them can. Treat every list as if it were the only one. No legitimate bitcoin company will ever ask for them: not a support line, not someone helping you, not an AI.",
    /// Text in `more`.
    spend_backup_nothing_done: "Nothing done here is irreversible until the last step, and that step does not happen in Faraday. Opening the vault, typing words, looking at addresses and reading a transaction move nothing and notify no one. You can stop and come back.",
    /// Text in `more`.
    spend_backup_more_no_deadline: "There is no deadline; bitcoin does not expire. Rushing is how people get robbed, by scammers and by helpers acting in their own interest. Take your time, and find someone you trust.",
    /// Text in `more`. `{}`: `m.noun()`.
    spend_backup_more_stop_if_unsure: "If any of this stops making sense, stop. Put the {} and the paper somewhere safe and get help from someone you would trust with the money itself. Waiting costs nothing.",
    /// Text in `more`.
    spend_backup_shut_the_computer: "Shut the computer down completely, not sleep or restart. Unplug any network cable.",
    /// Text in `more`. `{}`: `m.noun()`.
    spend_backup_plug_the_faraday: "Plug the Faraday {} in, switch the computer on and press the boot menu key straight away, repeatedly. It is usually F12, F9, Esc or F2. The first screen often names the key for a second or two.",
    /// Text in `more`. `{}`: `m.full()`.
    spend_backup_more_pick_boot_device: "Pick the {} from the list that appears. If no list appears, the key was wrong or came too late: shut down and try another one.",
    /// Text in `more`. `{}`: `m.noun()`, `m.a()`.
    spend_backup_if_the_is_missing: "If the {} is missing, or someone else has opened the envelope, do not use it. Do not use {} you are unsure about: the words are the money, and taking longer costs nothing.",
    /// Text in `more`.
    spend_backup_a_vault_unlocks: "A vault unlocks in the time shown on purpose: it makes guessing the passphrase slow. A wrong passphrase opens nothing and says so.",
    /// Text in `more`.
    spend_backup_more_words_order: "Words go in in the order they are written. Faraday checks them against the list of 2048 words and the backup's own checksum, so a mistyped or missing word is caught.",
    /// Text in `more`.
    spend_backup_a_wallet_description: "A descriptor opens the wallet read-only: enough to see every address and to check the balance. The words that sign come later, after the transaction is read.",
    /// Text in `more`.
    spend_backup_split_sheets: "Split sheets go in one at a time, in any order; the wallet opens when enough of them are in.",
    /// Text in `more`.
    spend_backup_in_sparrow_file: "In Sparrow: File, New Wallet, give it a name, then choose the way in that scans a QR code or imports a descriptor, and hold this screen up to the webcam. Menu names change between versions; look for an import that takes a descriptor or a scan. If the code is animated, let it loop once.",
    /// Text in `more`.
    spend_backup_more_trusted_software: "Only load it into wallet software you chose and trust.",
    /// Text in `more`.
    spend_backup_faraday_can: "Faraday can work out every address the wallet owns, but not which were used. Money is often spread over several addresses, including change addresses from old payments, so the first few can be empty while the money is further down. Sparrow adds them all up.",
    /// Text in `more`.
    spend_backup_on_a_phone_you: "On a phone you can also look an address up on a block explorer such as mempool.space. That moves nothing; it does tell the website you looked.",
    /// Text in `more`.
    spend_backup_paying_into: "Paying into an exchange: the exchange gives a deposit address for bitcoin in your account. Copy it exactly, and compare the first and last few characters after pasting.",
    /// Text in `more`.
    spend_backup_faraday_locks: "Faraday locks after a few minutes without input. That is expected. Unlock again with the same passphrase and this tab opens where you left it.",
    /// Text in `more`.
    spend_backup_if_the_envelope: "If the envelope held a signer that came with the papers and looks untouched, you can sign the same PSBT on it too and compare. Follow the device's own guide to restore from the same words. Compare what both screens say before approving anything: the amount, the fee and the address. They must match.",
    /// Text in `more`.
    spend_backup_more_faraday_enough: "Faraday on its own is enough to do the whole job.",
    /// Text in `more`.
    spend_backup_any_of_these: "Any of these sends it: Sparrow, which opens a transaction from a QR code, a file or pasted text and broadcasts it; a block explorer's broadcast page, such as mempool.space or blockstream.info; or someone you trust.",
    /// Text in `more`.
    spend_backup_more_check_id: "Then check the id. Whatever sends it should report the id Faraday showed. If the id you see anywhere differs, what was sent is not what you signed here: stop and find out why.",
    /// Text in `more`.
    spend_backup_the_first_confirmation: "The first confirmation usually takes about ten minutes but can take hours if the fee was low. Exchanges usually want one to six confirmations before the money shows in the account.",
    /// Text in `more`. `{}`: `m.noun()`.
    spend_backup_when_faraday: "When Faraday signs with a vault open, it records the amounts it signed inside that vault, so locking seals a changed copy of the vault to write back. Writing it back to the {} keeps that record; leaving it out changes nothing else.",
        }
    };
}
