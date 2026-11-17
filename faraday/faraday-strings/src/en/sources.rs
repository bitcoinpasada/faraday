//! Faraday's English words for the sources area.

macro_rules! words_sources {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- New key's sources, checks and dice lists, the self-test, BIP-85's applications, Tools' calculators, Explore's path -----
    /// Row that declines a passphrase.
    passphrase_none: "No passphrase",
    /// Button in the "Demo only" card that puts Practice's passphrase in
    /// the empty passphrase fields of the page (DEMO §5.4).
    demo_use_faraday: "Use faraday",
    /// Button that discards the entries.
    create_sanity_again: "Roll again",
    /// Source: a six-sided die.
    create_source_dice: "Dice",
    /// Source: coin flips.
    create_source_coins: "Coin flips",
    /// Source: typed hex.
    create_source_hex: "Hex entropy",
    /// Source: a shuffled deck.
    create_source_cards: "Playing cards",
    /// Source: camera noise.
    create_source_camera: "Camera noise",
    /// Source: several sources mixed.
    create_source_mix: "Mixed sources",
    /// Source: the device's own generator.
    create_source_device: "This device",
    /// Label: the chi-square statistic of the rolls.
    create_chi_square: "Chi-square",
    /// Label: the longest run of one outcome.
    create_longest_run: "Longest run",
    /// The run's value on a dice check: `{}` runs of the face named by
    /// `create_face_1` to `create_face_6`, as `8 sixes`.
    create_run_face: "{} {}",
    /// Plural name of dice face 1, for that value.
    create_face_1: "ones",
    /// Plural name of dice face 2.
    create_face_2: "twos",
    /// Plural name of dice face 3.
    create_face_3: "threes",
    /// Plural name of dice face 4.
    create_face_4: "fours",
    /// Plural name of dice face 5.
    create_face_5: "fives",
    /// Plural name of dice face 6.
    create_face_6: "sixes",
    /// Verdict beside a statistic: `normal for {} rolls`.
    create_stat_normal: "normal for {} rolls",
    /// Verdict beside a statistic that is out of the usual range.
    create_stat_high: "high for {} rolls",
    /// Label: heads.
    create_heads: "Heads",
    /// Label: tails.
    create_tails: "Tails",
    /// That Choice: the rolls hashed as ASCII digits.
    dice_procedure_hashed: "Hashed (Coldcard, SeedSigner)",
    /// That Choice: every 6 written as a 0, then hashed.
    dice_procedure_six_as_zero: "Six as zero, hashed (Keystone)",
    /// The second line of a procedure's row: `{} rolls`.
    dice_procedure_rolls: "{} rolls",
    /// Label: how many cards were drawn.
    create_cards_row: "Cards",
    /// The shutter under it.
    create_camera_shutter: "Take a picture",
    /// Label: how many frames were taken.
    create_frames_row: "Frames",
    /// Label: how many of the 256 luma values the last frame used.
    create_distinct_row: "Luma values",
    /// The coloured line on that step.
    create_device_result: "Not checkable",
    /// Label: what produced the bytes.
    create_device_row: "Source",
    /// The generator on a device with no secure element.
    create_device_os: "operating system",
    /// The caution this source carries on the source list.
    create_trusts_device: "trusts this device",
    /// Label on the device-randomness result: what the bytes rest on.
    create_trust_row: "Trust",
    /// That row's value, in the caution tone.
    create_trust_device: "this device's software",
    /// "Draw again" on the sanity step, for cards.
    create_sanity_again_draw: "Draw again",
    /// "Take again" on the sanity step, for the camera.
    create_sanity_again_take: "Take again",
    /// Application 39': a child key of words.
    bip85_app_words: "Words",
    /// Application 2': an HD-seed WIF.
    bip85_app_wif: "WIF",
    /// Application 32': an extended private key.
    bip85_app_xprv: "Extended private key",
    /// Application 128169': raw bytes.
    bip85_app_hex: "Hex",
    /// Application 707764': a password in base64.
    bip85_app_base64: "Password (base64)",
    /// Application 707785': a password in base85.
    bip85_app_base85: "Password (base85)",
    /// Row: the hash functions.
    tools_hashes: "Hashes",
    /// Row: base58, bech32, hex, binary.
    tools_encodings: "Encodings",
    /// Row: the descriptor checksum.
    tools_descriptor_checksum: "Descriptor checksum",
    /// Row: xpub to SLIP-132 and back.
    tools_convert_key: "Convert an xpub",
    /// Row: the unit converter.
    tools_units: "Units",
    /// Tools row: the miniscript policy compiler.
    tools_miniscript: "Miniscript compiler",
    /// Title of the Choice that picks the list.
    dice_list_title: "Which list?",
    /// Row: the 7776-word list.
    dice_list_large: "Long list",
    /// What that row costs to roll.
    dice_list_large_detail: "7776 words · five dice",
    /// Row: the first 1296-word list.
    dice_list_short1: "Short list",
    /// What that row costs to roll.
    dice_list_short1_detail: "1296 words · four dice",
    /// Row: the second 1296-word list.
    dice_list_short2: "Short list 2",
    /// What that row costs to roll, and what its words are chosen for.
    dice_list_short2_detail: "1296 words · 3 edits apart",
    /// A path with two slashes in a row.
    explore_path_empty: "Two slashes in a row.",
    /// A path that ends with a slash.
    explore_path_unfinished: "Type an index after the slash.",
    /// A level that is not a number.
    explore_path_not_a_number: "A level is a number, with h for hardened.",
    /// An index of two to the thirty-first or more.
    explore_path_too_large: "The largest index is 2147483647.",
    /// More than ten levels.
    explore_path_too_deep: "At most ten levels.",
    /// Button and title: the account's addresses, a page at a time.
    explore_addresses: "Addresses",
    /// Button and title: look for an address among the account's.
    explore_find: "Find an address",
    /// Button: make the wallet of the account explored.
    explore_make_wallet: "Make a wallet",
    /// Button: open the wallet already made for the account explored.
    explore_open_wallet: "Open the wallet",
    /// A wallet made at a path: `{}` the key's name, `{}` the account's path.
    made_wallet_name: "{} · {}",
    /// Under the button: the wallet's first address is not the key's; `{}` the account's path.
    made_wallet_differs: "Its first address is not the one this key gives at {}: not added",
    /// Button on the made wallet's dialog: back to the page.
    made_wallet_done: "Done",
    /// Button on the made wallet's dialog: the wallet's card.
    made_wallet_open: "Open the wallet",
    /// Label: the account the addresses are of.
    explore_account: "Account",
    /// The account: `{}` its path without `m/`, `{}` its script.
    explore_account_path: "m/{} · {}",
    /// Button: the receive chain.
    explore_receive: "Receive",
    /// Button: the change chain.
    explore_change: "Change",
    /// An address's name: `{}` its index on the receive chain.
    explore_receive_n: "Receive {}",
    /// An address's name: `{}` its index on the change chain.
    explore_change_n: "Change {}",
    /// Label: the list's first index, typed.
    explore_from_index: "From index",
    /// Button: the page before; `{}` how many addresses a page holds.
    explore_previous_n: "Previous {}",
    /// Button: the page after; `{}` how many addresses a page holds.
    explore_next_n: "Next {}",
    /// Label: how many indices of each chain the search looks at.
    explore_look_through: "Look through",
    /// Button: a search's depth; `{}` the indices of each chain.
    explore_first_n: "First {}",
    /// Button: start the search.
    explore_search: "Search",
    /// Button: stop the search where it is.
    explore_stop: "Stop",
    /// A search under way: `{}` the indices looked at, `{}` how many it looks at, on each chain.
    explore_searched: "Searched {} of {} on each chain",
    /// A search that found nothing: `{}` the indices looked at on each chain.
    explore_not_in_the_first: "Not in the first {} receive and change addresses",
    /// A search's matches: `{}` how many.
    explore_found_n: "{} found",
    /// Matches past the ones listed: `{}` how many.
    explore_more_n: "{} more not listed",
    /// A paste with a character no address has, or too long for one.
    explore_not_an_address: "Not an address",
    /// Label beside the eye on the key's own secret: its words and passphrase, or its seed.
    explore_secret: "Secret",
    /// Label: the key's seed words, numbered.
    explore_words: "Words",
    /// Label: the key's BIP-39 passphrase.
    explore_passphrase: "Passphrase",
    /// Label: a key's master seed, in hex.
    explore_master_seed: "Master seed",
    /// Label: a key's codex32 secret.
    explore_codex32_secret: "codex32 secret",
    /// Label: a codex32 share: `{}` its share index.
    explore_codex32_share: "codex32 share {}",
    /// Label beside the eye on an address's private key, and over the key shown.
    explore_wif: "Private key (WIF)",
    /// Line under an address's private key at an unhardened path.
    explore_wif_gives_account: "With the account's extended public key, this key gives every key of the account",
    /// Label beside the eye on the root's and the account's extended private keys.
    explore_xprvs: "Extended private keys",
    /// Label: the root's extended private key.
    explore_xprv_root: "Root",
    /// Label: an extended private key's path: `{}` the path without `m/`.
    explore_path_m: "m/{}",
    /// Label: the self-test result.
    settings_selftest: "Self-test",
    /// The self-test passed: `{}` is the check count.
    settings_selftest_passed: "passed · {} checks",
    /// The self-test failed: `{}` is the check.
    settings_selftest_failed: "failed · {}",
    /// Action that runs the self-test again.
    settings_selftest_run: "Run again",
    /// Snackbar after Run again passed: `{}` is the check count.
    settings_selftest_toast_passed: "Self-test passed: {} checks",
    /// Snackbar after Run again failed: `{}` is the check.
    settings_selftest_toast_failed: "Self-test failed: {}",
    /// Failure title.
    selftest_failed_title: "Self-test failed",
    /// Row label: the check that did not reproduce.
    selftest_check_row: "Check",
    /// The only button.
    selftest_exit: "Exit",
        }
    };
}
