//! Faraday's English words for the explore area.

macro_rules! words_explore {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- explore (faraday-core/src/explore/mod.rs) -----
    /// Toast in `explore_open`.
    explore_load_a_key: "Load a key first",
    // ----- explore (faraday-core/src/explore/screen.rs) -----
    /// Text in `draw`.
    explore_the_public: "The public side of a loaded key at any path: the extended public key, the public key and the address there. Compare them with what another wallet shows for the same path. The key's secret and its private keys show only behind their eyes.",
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
    /// A line of the Result in My seed makes my wallet.
    seed_check_multisig_native: "Multisig · Native SegWit",
    /// A line of the Result in My seed makes my wallet.
    seed_check_multisig_nested: "Multisig · Nested SegWit",
    /// A line of the Account card in My seed makes my wallet. `{}`: where the addresses start.
    seed_check_starts: "Addresses start {}",
    /// A line of the Account card in My seed makes my wallet.
    seed_check_cosigner: "A cosigner's key",
    /// The screen's title and its Tools tile in My seed makes my wallet.
    seed_check_title: "My seed makes my wallet",
    /// The Tools tile's line of My seed makes my wallet.
    seed_check_line: "Seed words to its xpub and addresses",
    /// A card of My seed makes my wallet.
    seed_check_card_seed: "Seed",
    /// A card of My seed makes my wallet.
    seed_check_card_passphrase: "Passphrase",
    /// A card of My seed makes my wallet.
    seed_check_card_account: "Account",
    /// A card of My seed makes my wallet.
    seed_check_card_result: "Result",
    /// A way in on the Seed card of My seed makes my wallet.
    seed_check_scan: "Scan a SeedQR",
    /// The keys loaded, on the Seed card of My seed makes my wallet.
    seed_check_loaded: "A key loaded here",
    /// The question over the loaded keys, at the left of the Seed card of My seed makes my wallet.
    seed_check_which_key: "Which key?",
    /// A loaded key's mark on the Seed card of My seed makes my wallet.
    seed_check_with_passphrase_mark: "passphrase",
    /// The Seed card's summary of My seed makes my wallet. `{}`: how many words.
    seed_check_words_n: "{} words",
    /// The Seed card's action of My seed makes my wallet.
    seed_check_continue: "Continue",
    /// A choice on the Passphrase card of My seed makes my wallet.
    seed_check_no_passphrase: "No passphrase",
    /// A choice on the Passphrase card of My seed makes my wallet.
    seed_check_add_passphrase: "Add a passphrase",
    /// A field of the Passphrase card of My seed makes my wallet.
    seed_check_passphrase: "Passphrase",
    /// A field of the Passphrase card of My seed makes my wallet.
    seed_check_passphrase_again: "Passphrase again",
    /// An action of the Passphrase card of My seed makes my wallet.
    seed_check_use_passphrase: "Use this passphrase",
    /// The Passphrase card's summary of My seed makes my wallet. `{}`: the fingerprint.
    seed_check_fingerprint_is: "Fingerprint {}",
    /// The Passphrase card's summary of My seed makes my wallet when the passphrase is used. `{}`: the fingerprint.
    seed_check_fingerprint_is_with: "Fingerprint {} · with a passphrase",
    /// A row of the Account card of My seed makes my wallet.
    seed_check_other_account: "Another account or path",
    /// An action of the Account card of My seed makes my wallet.
    seed_check_use_path: "Use this path",
    /// What a Legacy test network address starts with, in My seed makes my wallet's Account card.
    seed_check_legacy_test_start: "m or n",
    /// A row of the Result in My seed makes my wallet.
    seed_check_fingerprint: "Fingerprint",
    /// A row of the Result in My seed makes my wallet.
    seed_check_without: "Without the passphrase",
    /// A row of the Result in My seed makes my wallet.
    seed_check_path: "Path",
    /// A row of the Result in My seed makes my wallet.
    seed_check_script: "Script",
    /// A row of the Result in My seed makes my wallet.
    seed_check_xpub: "Xpub",
    /// A row of the Result in My seed makes my wallet.
    seed_check_receive: "First receive address",
    /// A row of the Result in My seed makes my wallet.
    seed_check_change_address: "First change address",
    /// The plain spelling of the Xpub row in My seed makes my wallet.
    seed_check_plain: "xpub",
    /// An action of the Result in My seed makes my wallet.
    seed_check_another_account: "Another account",
    /// An action of the Result in My seed makes my wallet.
    seed_check_another_seed: "Another seed",
    /// An action of the Result in My seed makes my wallet.
    seed_check_keep: "Keep this key",
    /// An action of the Result in My seed makes my wallet.
    seed_check_done: "Done",
    /// An action of My seed makes my wallet.
    seed_check_cancel: "Cancel",
    /// The title of the Xpub's code sheet in My seed makes my wallet.
    seed_check_xpub_code: "Account xpub",
    /// The title of the receive address's code sheet in My seed makes my wallet.
    seed_check_receive_code: "Receive address",
    /// The Seed card, words a scanner read that are no BIP-39 seed.
    seed_check_not_seed: "Not a BIP-39 seed",
    /// The text button on My seed makes my wallet's Result that opens the comparison.
    seed_check_compare: "Compare with the wallet app's",
    /// The comparison's field, its label.
    seed_check_compare_field: "Key, address or wallet file",
    /// The comparison's Scan button.
    seed_check_compare_scan: "Scan",
    /// The comparison's From Files button, with no file in Files to take.
    seed_check_compare_files: "From Files",
    /// The comparison's From Files button, one file of Files.
    seed_check_compare_file: "From Files · {}",
    /// The comparison's action.
    seed_check_compare_go: "Compare",
    /// The comparison's answer, the value is this account's or this seed's.
    seed_check_matches: "Matches",
    /// The comparison's answer, the value is not.
    seed_check_differs: "Does not match",
    /// The comparison's line under "Does not match": the same seed with no passphrase makes it.
    seed_check_without_pass: "It matches without the passphrase",
    /// The comparison's line: an origin names this seed and its key at {path} is not the one given.
    seed_check_finger_only: "The fingerprint is this seed's; the xpub at {} is not",
    /// The comparison's line under "Does not match": the key given is at {path}, not at the account picked.
    seed_check_other_path: "The key is at {}",
    /// The comparison's line: a wallet's key this seed is, at {path}, not the account picked.
    seed_check_matches_at: "This seed's key is at {}",
    /// The comparison's line: a wallet's key this seed is, of a multisig.
    seed_check_key_of: "This seed is key {} of this wallet · {} of {}",
    /// The comparison's line: a wallet's key this seed is, of a wallet with no quorum.
    seed_check_key_of_wallet: "This seed is key {} of this wallet",
    /// The comparison's line: an address found, its chain, index and path.
    seed_check_found_receive: "Receive address {} · {}",
    /// The comparison's line: a change address found, its index and path.
    seed_check_found_change: "Change address {} · {}",
    /// The comparison's line: an address not found in the first indices of both chains.
    seed_check_not_found: "Not among the first {} addresses of each chain",
    /// The comparison's refusal: an extended private key, a seed or a private key.
    seed_check_compare_private: "A private key: not taken here",
    /// The comparison's refusal: a value over the size taken.
    seed_check_compare_too_long: "Over {}: not taken here",
    /// The comparison's refusal: a key or a wallet of another network.
    seed_check_compare_network: "That is of another network",
    /// The comparison's refusal: an address while the account is a multisig cosigner's.
    seed_check_compare_cosigner: "A cosigner's account has no addresses of its own",
    /// The comparison's refusal: a value no reader takes.
    seed_check_compare_unread: "Not a key, an address or a wallet",
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
        }
    };
}
