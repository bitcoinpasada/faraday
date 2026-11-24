//! Faraday's English words for the wallet area.

macro_rules! words_wallet {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- out_bytes (faraday-core/src/wallet/v2.rs) -----
    /// Toast where a spend's version 2 PSBT cannot take its signatures
    /// back (`Spend::out_bytes`).
    spend_v2_not_written: "Not written: the signatures do not go into the version 2 PSBT",
    // ----- wallet (faraday-core/src/wallet.rs) -----
    /// Text in `network_name`.
    wallet_mainnet: "Mainnet",
    /// Text in `network_name`.
    wallet_testnet: "Testnet",
    /// Text in `network_name`.
    wallet_signet: "Signet",
    /// Text in `network_name`.
    wallet_regtest: "Regtest",
    /// Text in `text`. `{}`: `e`.
    wallet_not_a_valid: "Not a valid seed: {}",
    /// Text in `text`. `{}`: `fp`.
    wallet_is_already_loaded: "{} is already loaded",
    /// Text in `text`. `{}`: `got`, `wanted`.
    wallet_these_words_give: "These words give {}, not {}",
    /// Text in `text`. `{}`: `got`, `wallet`.
    wallet_these_words: "These words give {}, not a key of {}",
    /// Text in `text`. `{}`: `e`.
    wallet_not_a_wallet: "Not a wallet: {}",
    /// Text in `text`. `{}`: `network_name(*offered)`, `network_name(*loaded)`.
    wallet_is_not_the_network: "{} is not {}, the network of the loaded wallets",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the application id in hex.
    core_wallet_application: "This wallet file is for a network Faraday does not have (application id {})",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the record types and counts.
    core_wallet_legacy: "This is a legacy wallet (records {}); Bitcoin Core's migratewallet turns it into a descriptor wallet",
    /// Refusal in `core_wallet::Refusal::message`.
    core_wallet_not_descriptors: "This wallet is not a descriptor wallet",
    /// Text in `text`: a wallet read with a key that is a bare extended
    /// public key.
    wallet_key_no_origin: "A key of this wallet has no fingerprint and derivation path",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the record.
    core_wallet_no_origin: "A descriptor of this wallet has a key with no origin ({})",
    /// Refusal in `core_wallet::Refusal::message`.
    core_wallet_not_sqlite: "This file is not a SQLite database, so not a descriptor wallet's wallet.dat",
    /// Refusal in `core_wallet::Refusal::message`.
    core_wallet_wal: "This wallet file is in the write-ahead-log form, which Faraday does not read",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the table's name.
    core_wallet_other_table: "This database holds records in a table other than main ({})",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the table's name.
    core_wallet_no_table: "This database has no table {}",
    /// Refusal in `core_wallet::Refusal::message`.
    core_wallet_file_damaged: "This wallet file is damaged or not as Bitcoin Core writes it",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the other file's name.
    core_wallet_beside: "The file {} is beside this wallet, so it is in use or holds data not yet in it",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the version.
    core_wallet_schema_version: "This wallet file has database version {}, not 0 as Bitcoin Core writes it",
    /// Refusal in `core_wallet::Refusal::message`. `{}`: the most rows.
    core_wallet_too_many_rows: "This wallet file holds more than {} records, more than a Bitcoin Core wallet has",
    /// Refusal in `core_wallet::Refusal::message`.
    core_wallet_berkeley: "This is a legacy Berkeley DB wallet; Bitcoin Core's migratewallet turns it into a descriptor wallet",
    /// Text in `text`. `{}`: `fp`.
    wallet_is_not_a_key: "{} is not a sample: Practice takes only samples",
    /// Text in `text`: a wallet refused because one of its keys has a
    /// loaded key's fingerprint and is not what that key gives at its
    /// path. `{}`: the key's fingerprint.
    wallet_not_your_key: "Key {} in this wallet is not this device's key at its path",
    /// Text in `text`: a wallet refused because one of its keys is a key
    /// made here, not backed up yet, under another fingerprint. `{}`: the
    /// fingerprint the wallet gives the key.
    wallet_not_backed_up: "Key {} in this wallet is this device's key under another fingerprint, and it is not backed up yet; the wallet loads once it is backed up",
    /// Text in `text`: a silent payments wallet refused because a loaded
    /// key with its fingerprint gives another scan key or spend key. `{}`:
    /// the key's fingerprint.
    wallet_silent_differs: "This wallet's record does not match key {}: its scan key or spend key is not that key's",
    /// Text in `text`: a wallet of several keys, or one a key not of its
    /// own spends alone, not added until its sheet is confirmed (F254).
    wallet_confirm_unconfirmed: "Not added yet: waiting for its confirmation",
    /// A `tr()` wallet's key path line, the NUMS point (F132).
    wallet_key_path_none: "Key path: none (unspendable)",
    /// A `tr()` wallet's key path line, a key of the wallet. `{}`: the key's fingerprint and label.
    wallet_key_path_alone: "Key path: {} alone",
    /// The key path's holder when its fingerprint is unknown. `{}`: its number from 1.
    wallet_key_n: "key {}",
    /// A `tr()` wallet's danger line, a key that is not one of the wallet's (F132). `{}`: the key, in groups of four.
    wallet_key_path_other: "A key that is not one of this wallet's keys can spend everything alone, without the other signatures: {}",
    /// The wallet confirmation sheet's title (F254).
    wallet_confirm_head: "Add this wallet",
    /// The title when several wallets wait one after another (F254). `{}`: which one, from 1; `{}`: how many.
    wallet_confirm_head_of: "Add this wallet · {} of {}",
    /// The wallet confirmation sheet's name row (F254).
    wallet_confirm_name: "Name",
    /// The line under the keys of the wallet confirmation sheet (F254).
    wallet_confirm_compare: "Compare every key with the one on its own device",
    /// The wallet confirmation sheet's hold (F254).
    wallet_confirm_hold: "Hold to add this wallet",
    /// The wallet confirmation sheet's dismissive action.
    wallet_confirm_cancel: "Cancel",
    /// Toast once the wallet is added from the wallet confirmation sheet. `{}`: its name.
    wallet_confirm_added: "{} added",
    /// The published-words sheet's title: a key whose words are published, on Mainnet.
    published_head: "Published words",
    /// The published-words sheet's line.
    published_line: "These words are published. Anyone can spend from a wallet made of them.",
    /// The published-words sheet's title for a key a vault says was written on an online computer.
    published_online_head: "Made on an online computer",
    /// The published-words sheet's line for such a key.
    published_online_line: "This key was written by Faraday Demo on an online computer. Anyone who had access to that computer may have its seed.",
    /// The hold that keeps such a key.
    published_online_hold: "Hold to keep this key",
    /// The published-words sheet's title for one of Faraday's public samples.
    published_sample_head: "Public sample seed",
    /// The published-words sheet's line for one.
    published_sample_line: "This seed is a public sample. Its coins can be taken by anyone, so it is not kept on Mainnet.",
    /// Text in the Lightning screen and Convert: a node key of a public
    /// sample, refused on Mainnet.
    published_sample_node: "This Lightning key is a public sample. Its coins can be taken by anyone, so it is not used on Mainnet.",
    /// The published-words sheet's primary action: the key is dropped.
    published_remove: "Remove",
    /// The published-words sheet's other action: the key stays, for testing.
    published_keep: "Keep for testing",
    // ----- Which account of a Coldcard export loads (docs/INTEROP.md §5, BIP-392 sp()), faraday-core -----
    /// Text in `text`: a Coldcard export holding a single-sig account and
    /// a silent payments account, not added until one is chosen.
    wallet_account_unchosen: "Not added yet: choose its account first",
    /// A MuSig2 wallet two of whose keys come from one seed, refused.
    wallet_musig_one_seed: "Two keys of this wallet come from one seed. This device cannot sign for both.",
    /// The account dialog's title.
    wallet_account_head: "Which account to load?",
    /// The account dialog's line. `{}`: the wallet's name.
    wallet_account_line: "{} holds a single-sig account and a silent payments account",
    /// The account dialog's single-sig choice. `{}`: its BIP, "BIP-84".
    wallet_account_single: "Single-sig · {}",
    /// The account dialog's silent payments choice.
    wallet_account_silent: "Silent payments · BIP-352",
    /// The account dialog's dismissive action.
    wallet_account_cancel: "Cancel",
    // ----- Register this wallet, from a spend (docs/SPEND-FROM-BACKUP.md §8), faraday-core -----
    /// The Spend review's button under the line that holds a spend from
    /// a wallet of several keys not registered here, where the PSBT
    /// carries the cosigners' keys; and the register sheet's title.
    register_head: "Register this wallet",
    /// The Spend review's line under that block where the PSBT does not
    /// carry the cosigners' keys.
    register_load_descriptor: "Load this wallet's descriptor to sign",
    /// The register sheet's shape line. `{}`: the threshold, the number
    /// of keys, the script (`register_p2wsh` and the like), then
    /// `register_sorted` or `register_in_order`.
    register_shape: "{} of {}, {}, {}",
    /// The script in `register_shape`.
    register_p2wsh: "P2WSH",
    /// The script in `register_shape`.
    register_p2sh: "P2SH",
    /// The script in `register_shape`.
    register_p2sh_p2wsh: "P2SH-P2WSH",
    /// The key order in `register_shape`: `sortedmulti`.
    register_sorted: "sorted",
    /// The key order in `register_shape`: `multi`, the keys in the order listed.
    register_in_order: "keys in this order",
    /// The register sheet's name field's label.
    register_name: "Name",
    /// The register sheet's hold.
    register_hold: "Hold to register",
    /// The register sheet's dismissive action.
    register_cancel: "Cancel",
    /// Toast once the wallet is registered. `{}`: its name.
    register_added: "{} registered",
    /// The register sheet's refusal: the descriptor built from the
    /// PSBT's keys does not read as a wallet.
    register_refused_descriptor: "These keys do not make a wallet",
    /// The register sheet's refusal. `{}`: the input's number.
    register_refused_input: "These keys do not give input {}'s address",
    /// The register sheet's refusal. `{}`: the output's number.
    register_refused_output: "Output {} names this wallet but is not its address",
    /// The register sheet's refusal: no key of the wallet is one loaded here.
    register_refused_not_yours: "None of these keys is yours",
    /// The register sheet's refusal: a key under a loaded key's
    /// fingerprint that the loaded key does not give. `{}`: the
    /// fingerprint, then the path.
    register_refused_not_your_key: "Key {} at {} has your fingerprint but is not your key",
    /// The register sheet's line over the two readings, where the spend allows both.
    register_reading_line: "Most coordinators sort the keys",
    /// The register sheet's choice: `sortedmulti`.
    register_reading_sorted: "Sorted",
    /// The register sheet's choice: `multi`, the keys in the order listed.
    register_reading_in_order: "In this order",
    /// Text in `source_tag`. `{}`: `source`.
    wallet_source_tag: "From {}",
    /// Text in `source_tag`. `{}`: `source`.
    wallet_from_inline: "from {}",
    /// Text in `add_seed`.
    wallet_a_master_seed: "a master seed is 16 to 64 bytes",
    /// Text in `shape`.
    wallet_legacy: "legacy",
    /// Text in `shape`.
    wallet_nested_segwit: "nested SegWit",
    /// Text in `shape`.
    wallet_native_segwit: "native SegWit",
    /// Text in `shape`.
    wallet_taproot: "Taproot",
    /// Text in `shape`. `{}`: `script(t)`.
    wallet_single_key: "Single key · {}",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_legacy_multisig: "{} of {} · legacy multisig",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_nested_segwit: "{} of {} · nested SegWit multisig",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_native_segwit: "{} of {} · native SegWit multisig",
    /// Text in `shape`. `{}`: `m`, `n`.
    wallet_of_taproot_multisig: "{} of {} · Taproot multisig",
    /// Text in `shape`. `{}`: `wallet.policy.keys().len()`.
    wallet_keys_miniscript: "{} keys · miniscript with conditions",
    /// Text in `shape`. `{}`: `wallet.policy.keys().len()`.
    wallet_keys_taproot: "{} keys · Taproot key and leaves",
    /// Text in `shape`. `{}`: `n`, `n`.
    wallet_of_musig2: "{} of {} · MuSig2",
    /// Text in `shape`.
    wallet_1_key_silent: "1 key · silent payments",
    /// Text in `shape`.
    wallet_wallet: "Wallet",
    /// Text in `address`. `{}`: `e`.
    wallet_no_address: "no address: {}",
    /// Error in `sign_with`.
    wallet_no_signing_key: "No key in this session signs this transaction",
    /// Text in `read_wallet`.
    wallet_a_json_file: "a .json file with no descriptor",
    /// Text in `pick_best`.
    wallet_several: "a file with more than one wallet",
    /// Text in `read_wallet`. `{}`: `e`.
    wallet_bsms: "BSMS: {}",
    /// Text in `coldcard_export`. `{}`: `why`.
    wallet_coldcard_export: "Coldcard export: {}",
    /// Text in `coldcard_export`.
    wallet_no_xfp: "no xfp",
    /// Error in `coldcard_export`. `{}`: `field`, `first`.
    wallet_its_first_address: "its first {} address is not {}",
    /// Error in `coldcard_export`.
    wallet_no_single_sig: "no single-sig account",
    /// Error in `silent_refusal`: BIP-392's `sp()` with no key origin.
    wallet_sp_no_origin: "This sp() descriptor names no key origin",
    /// Error in `silent_refusal`. `{}`: why.
    wallet_sp: "sp() descriptor: {}",
    /// Error in `silent_refusal`.
    wallet_sp_origin: "its key origin is not BIP-352's path on its network",
    /// Error in `silent_refusal`.
    wallet_sp_key: "its key is not a spscan key",
    /// Error in `silent_refusal`.
    wallet_sp_checksum: "its checksum does not match",
    /// Error in `silent_refusal`.
    wallet_sp_shape: "it is not sp() over one spscan key",
    /// Error in `wallet::mismatch_text`. `{}`: the key, shortened; the
    /// script type its version names; the script type around it.
    wallet_slip132_mismatch: "{} is a {} key; this wallet is {}",
    /// Text in `wallet::script_name`.
    wallet_slip132_native_multisig: "native SegWit multisig",
    /// Text in `wallet::script_name`.
    wallet_slip132_nested_multisig: "nested SegWit multisig",
    /// Text in `wallet::script_name`.
    wallet_slip132_legacy_multisig: "legacy multisig",
    /// Text in `wallet::script_name`.
    wallet_slip132_silent_payments: "silent payments",
    /// Text in `wallet::script_name`: a descriptor of no type a
    /// SLIP-132 version names.
    wallet_slip132_other: "another script type",
    /// Error in `single_key::read`. `{}`: why.
    wallet_single_key_file: "Single-key file: {}",
    /// Text in `single_key::read`.
    wallet_single_key_fingerprint: "MasterFingerprint is not 8 hex digits",
    /// Text in `single_key::read`.
    wallet_single_key_xpub: "ExtPubKey is not an extended public key",
    /// Text in `single_key::read`.
    wallet_single_key_path: "AccountKeyPath is not a derivation path",
    /// Text in `read_wallet`, of a single-key file.
    wallet_single_key_no_type: "its key is not a single-key wallet's",
    /// Error in `caravan::descriptor`. `{}`: why.
    wallet_caravan: "Caravan config: {}",
    /// Error in `read_wallet`: Specter DIY's `addwallet` with no descriptor after its `&`.
    wallet_specter_diy_no_descriptor: "Specter DIY addwallet: no descriptor after the &",
    /// Text in `caravan::descriptor`.
    wallet_caravan_address_type: "addressType is not P2WSH, P2SH-P2WSH or P2SH",
    /// Text in `caravan::descriptor`.
    wallet_caravan_network: "network is not mainnet, testnet, signet or regtest",
    /// Text in `caravan::descriptor`.
    wallet_caravan_starting_index: "startingAddressIndex is not 0",
    /// Text in `caravan::descriptor`.
    wallet_caravan_keys: "a key in extendedPublicKeys is not an extended public key",
    /// Text in `caravan::descriptor`.
    wallet_caravan_quorum: "the quorum does not match the keys listed",
    /// Text in `caravan::descriptor`.
    wallet_caravan_xfp: "an xfp is not 8 hex digits",
    /// Text in `caravan::descriptor`. `{}`: the key, shortened.
    wallet_caravan_no_origin: "{} has no bip32Path or xfp; every key of a multisig wallet needs both",
    /// Text in `caravan::descriptor`. `{}`: the key, shortened; the
    /// network it is on; the network the file names.
    wallet_caravan_key_network: "{} is a {} key; the file says {}",
    /// Error in `electrum::descriptor`. `{}`: why.
    wallet_electrum: "Electrum file: {}",
    /// Error in `read_wallet`: an Electrum file encrypted, whole or its
    /// keystores.
    wallet_electrum_encrypted: "This Electrum file is encrypted",
    /// Text in `electrum::descriptor`.
    wallet_electrum_fingerprint: "a root_fingerprint or ckcc_xfp is not a fingerprint",
    /// Text in `electrum::descriptor`. `{}`: the keystore's `type`.
    wallet_electrum_keystore_type: "keystore type {} is not read",
    /// Text in `electrum::descriptor`.
    wallet_electrum_no_key: "a keystore has no xpub",
    /// Text in `electrum::descriptor`.
    wallet_electrum_derivation: "a derivation is not a derivation path",
    /// Text in `electrum::descriptor`. `{}`: the script type a key's
    /// version names.
    wallet_electrum_key_type: "a key is a {} key, which this wallet_type does not hold",
    /// Text in `electrum::descriptor`. `{}`: the file's `wallet_type`.
    wallet_electrum_wallet_type: "wallet_type {} is not read",
    /// Text in `electrum::descriptor`.
    wallet_electrum_quorum: "wallet_type does not match the keystores",
    /// Text in `electrum::descriptor`. `{}`: the key, shortened.
    wallet_electrum_no_origin: "{} has no root_fingerprint and derivation; every key of a multisig wallet needs both",
    /// Text in `electrum::descriptor`.
    wallet_electrum_mixed: "its keys name more than one script type",
    /// Error in `read_wallet`. `{}`: `json::MAX_DEPTH`.
    wallet_json_too_deep: "JSON nested more than {} levels deep",
    /// Error in `read_wallet`.
    wallet_json_too_large: "JSON longer than 1 MiB",
    /// Text in `collect_line`. `{}`: `missing`, `if collected == 0 { "" } else { " mor…`, `if missing == 1 { EN.wallet_signature…`.
    wallet_collect: "Collect {}{} {}",
    /// Text in `collect_line`, after a space.
    wallet_more: "more",
    /// Text in `collect_line`.
    wallet_signature: "signature",
    /// Text in `collect_line`.
    wallet_signatures: "signatures",
    /// Error in `collect`.
    wallet_not_a_psbt: "Not a PSBT",
    /// Error in `collect`. `{}`: `source`.
    wallet_psbt_other_transaction: "{} is a PSBT for another transaction",
    /// Text in `collect`. `{}`: `e`.
    wallet_cannot_combine: "Cannot combine: {}",
    /// Error in `collect`. `{}`: `source`.
    wallet_a_signature: "A signature in {} does not verify",
    /// Collect's refusal of a cosigner's PSBT that asks, or signs with, a
    /// sighash other than the transaction's. `{}`: the file's name.
    wallet_sighash_other: "A signature in {} uses a different sighash from this transaction's",
    /// Collect's refusal of a PSBT whose inputs ask another sighash than
    /// this transaction's, signed or not. `{}`: the file's name.
    wallet_sighash_field_other: "{} asks for a different sighash on an input than this transaction does",
    /// Error in `finish`.
    wallet_not_enough_signatures: "Not enough signatures yet",
    /// Text in `finish_reason`.
    wallet_script_unsatisfied: "The script is not satisfied yet: another signature, or a timelock not yet reached",
    /// Error in `sign_message`.
    wallet_no_such_wallet: "no such wallet",
    /// Error in `sign_message`.
    wallet_messages_are: "Messages are signed with single-key wallets",
    /// Error in `sign_message`.
    wallet_the_wallet_has: "the wallet has no key",
    /// Error in `sign_message`.
    wallet_the_wallet_s_xpub: "the wallet's xpub has no origin",
    /// Error in `sign_message`.
    wallet_the_wallet_s_key: "the wallet's key is not loaded here",
    /// Error in `sign_message`.
    wallet_the_wallet_s: "the wallet's xpub has no path",
    /// Text in `paths`. `{}`: `n`, `n / 144`.
    wallet_blocks_after: "{} blocks after the coins arrived (about {} days)",
    /// Text in `paths`. `{}`: `u64::from(*n) * 512 / 86_400`.
    wallet_days_after_arrival: "{} days after the coins arrived",
    /// Text in `paths`. `{}`: `h`.
    wallet_after_block: "after block {}",
    /// Text in `paths`. `{}`: `t`.
    wallet_after_time: "after time {}",
    /// Text in `paths`.
    wallet_a_secret_preimage: "a secret preimage",
    // ----- spend paths (faraday-core/src/wallet/paths.rs) -----
    /// Value of the small panel's Spend paths row. `{}`: how many paths.
    wallet_paths_count: "{} ways to spend",
    /// Heading in the wallet card, the small panel's row and the chart's wallet sheet.
    wallet_paths_heading: "Spend paths",
    /// Text in `when_text`: a path with no wait.
    wallet_paths_any_time: "Any time",
    /// Text in `keys_text`. `{}`: the key's number.
    wallet_paths_key: "Key {}",
    /// Text in `keys_text`. `{}`: the numbers as a list, how many keys, how many it wants.
    wallet_paths_keys: "Keys {}, {} of {}",
    /// Text in `list`. `{}`: the numbers before the last, the last.
    wallet_paths_list_and: "{} and {}",
    /// Text in `lock_text`. `{}`: the wait in blocks, in about so many days.
    wallet_paths_blocks: "{} blocks after each coin arrives · about {} days",
    /// Text in `lock_text`. `{}`: the wait in days.
    wallet_paths_intervals: "{} days after each coin arrives",
    /// Text in `lock_text`. `{}`: the block height.
    wallet_paths_after_block: "After block {}",
    /// Text in `lock_text`. `{}`: the date.
    wallet_paths_after_date: "After {}",
    /// Text in `lock_text`.
    wallet_paths_preimage: "A secret preimage",
    /// Label in `rows`: the margin of the shortest relative wait.
    wallet_paths_move_within: "Move each coin within",
    /// Value in `margin_text`, a wait in blocks. `{}`: whole days.
    wallet_paths_within_about: "about {} days of arriving",
    /// Value in `margin_text`, a wait in 512-second units. `{}`: whole days.
    wallet_paths_within: "{} days of arriving",
    /// Label in `moved_row`: when the person last moved every coin.
    wallet_coins_last_moved: "Coins last moved",
    /// Value in `moved_row`, a wait in blocks. `{}`: the age, then whole days.
    wallet_coins_moved_about: "{} · the wait ends at about {} days",
    /// Value in `moved_row`, a wait in 512-second units. `{}`: the age, then whole days.
    wallet_coins_moved: "{} · the wait ends at {} days",
    /// Action on the wallet's card and its chart sheet: every coin moved today.
    wallet_coins_i_moved: "I moved every coin",
    /// Toast in `chart_moved`.
    chart_edit_moved: "Coins moved today",
        }
    };
}
