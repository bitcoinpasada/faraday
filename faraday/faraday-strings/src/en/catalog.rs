//! Faraday's English words for the catalog area.

macro_rules! words_catalog {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- catalog (faraday-core/src/catalog.rs) -----
    /// Text in `TILES`.
    catalog_add_a_key: "Add a key",
    /// Text in `TILES`: Add a key's line.
    catalog_add_a_key_line: "Words, a SeedQR or codex32 shares you have",
    /// Text in `TILES`, and Wallets start.
    catalog_make_a_new_key: "Make a new key",
    /// Text in `TILES`.
    catalog_make_or_type: "Dice, coins, cards, hex or the camera",
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
    /// A tile's name in `TILES`: the standard's own name.
    catalog_codex32: "Restore from codex32 shares",
    /// Text in `TILES`: the group of the few tiles a person comes for most.
    catalog_common: "Common",
    /// Text in `TILES`.
    catalog_make_a_new_wallet: "Make a new wallet",
    /// Text in `TILES`.
    catalog_make_wallet_line: "Single key, multisig, taproot or MuSig2",
    /// Text in `TILES`.
    catalog_check: "Check",
    /// Text in `TILES`: Check's line.
    catalog_check_line: "Its seed, addresses and signatures",
    /// Text in `TILES`.
    catalog_make_codex32: "Make codex32 shares",
    /// Text in `TILES`.
    catalog_make_codex32_line: "A new key dealt as shares of a secret",
    /// Text in `TILES`.
    catalog_make_seed_xor: "Make Seed XOR parts",
    /// Text in `TILES`.
    catalog_make_seed_xor_line: "A key split into parts that combine back",
    /// Text in `TILES`.
    catalog_spend_backup: "Spend from a backup",
    /// Text in `TILES`.
    catalog_spend_backup_line: "From a stick, words or paper, step by step",
    /// Wallets start: the line under Make a new key.
    catalog_make_key_start_line: "A new key from randomness",
    /// Text in `TILES`.
    catalog_codex32_line: "A secret or its shares typed into a key",
    /// Text in `TILES`.
    catalog_seed_xor: "Restore from Seed XOR parts",
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
    catalog_convert_a_seed: "Convert a seed",
    /// Text in `TILES`.
    catalog_convert_a_seed_line: "The same wallet in another standard",
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
    /// A tile's name in `TILES`: the standard's own name.
    catalog_musig2: "MuSig2",
    /// Text in `TILES`.
    catalog_all_keys_one: "All keys, one signature on chain",
    /// Text in `TILES`.
    catalog_restore_wallet: "Restore an existing wallet",
    /// Text in `TILES`.
    catalog_descriptor_wallet: "Descriptor, miniscript, wallet file, BIP 129 record or split sheets",
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
    catalog_decode_a_lightning_invoice: "Decode a Lightning invoice",
    /// Text in `TILES`.
    catalog_what_it_asks: "What it asks to be paid, and by whom",
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
    /// Button in `message_body`: Sign a message again, empty.
    catalog_another_message: "Another message",
    /// Text in `TILES`.
    catalog_messages: "Messages",
    /// Text in `TILES`.
    catalog_check_a_signed: "Check a signed message",
    /// A tile's name in Check: the seed made from dice rolls, opened at New key's dice source.
    catalog_check_dice_seed: "Check a seed made from dice",
    /// A tile's name in Check, and the title of the dialog it opens: the forms of a backup.
    catalog_check_a_backup: "Check a backup in another form",
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
    catalog_diceware_passphrase: "Make a new vault",
    /// Text in `TILES`.
    catalog_diceware_line: "A new vault, its passphrase rolled with dice",
    /// Text in `TILES`.
    catalog_gpg_key: "GPG key",
    /// Text in `TILES`.
    catalog_make_sign_revoke: "Make, restore, sign, revoke, paperkey",
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
    catalog_load_a_wallet: "Load a wallet first",
    /// Text in `tile_need`.
    catalog_needs_a_single: "Needs a single-key wallet with its key here",
    /// Text in `tile_need`.
    catalog_copy_a_signed: "Copy a signed message in first",
    /// Text in `tile_need`.
    catalog_make_a_vault: "Make a new vault first",
    /// Text in `tile_need`.
    catalog_unlock_a_vault: "Unlock a vault first",
        }
    };
}
