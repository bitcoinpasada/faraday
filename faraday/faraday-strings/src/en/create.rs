//! Faraday's English words for the create area.

macro_rules! words_create {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- create (faraday-core/src/create.rs) -----
    /// Text in `name`.
    create_single_key_native: "Single key · native SegWit",
    /// Text in `name`.
    create_single_key_taproot: "Single key · Taproot",
    /// Text in `name`.
    create_single_key_nested: "Single key · nested SegWit",
    /// Text in `name`.
    create_single_key_legacy: "Single key · legacy",
    /// Text in `name`.
    create_multisig_native: "Multisig · native SegWit",
    /// Text in `name`.
    create_multisig_nested: "Multisig · nested SegWit",
    /// Text in `name`.
    create_multisig_legacy: "Multisig · legacy",
    /// Text in `name`.
    create_multisig_taproot: "Multisig · Taproot",
    /// Text in `name`.
    create_all_keys_musig2: "All keys · MuSig2",
    /// Text in `line`.
    create_bc1q_addresses: "bc1q addresses; what most wallets make",
    /// Text in `line`.
    create_bc1p_addresses: "bc1p addresses; the smallest signatures",
    /// Text in `line`.
    create_3_addresses: "3… addresses, for services that cannot pay bc1",
    /// Text in `line`.
    create_1_addresses: "1… addresses, for old software only",
    /// Text in `line`.
    create_several_keys: "Several keys, a quorum signs; Sparrow, Coldcard, SeedSigner",
    /// Text in `line`.
    create_multisig_nested_line: "Multisig with 3… addresses",
    /// Text in `line`.
    create_multisig_legacy_line: "Multisig with 3… addresses, old style; avoid unless required",
    /// Text in `line`.
    create_multisig_taproot_line: "Multisig on Taproot, one script leaf; Sparrow and Nunchuk",
    /// Text in `line`.
    create_every_key_signs: "Every key signs together, one Taproot key; single-key on chain",
    /// Text in `path_spec`. `{}`: `path`.
    create_not_a_derivation: "Not a derivation path: {}",
    // ----- create_scan_arrived (faraday-core/src/create.rs) -----
    /// Toast when a scanned descriptor is filed but not put in the slot
    /// waiting for a key. `{}`: the file's name, the slot's number, why.
    create_scan_not_in_slot: "{} is in Files, not in Key {}: {}",
    /// Why, in `create_scan_not_in_slot`. `{}`: how many keys.
    create_scan_keys: "it holds {} keys, not one",
    /// Why, in `create_scan_not_in_slot`.
    create_scan_no_origin: "its key has no fingerprint and derivation path; most wallets export a key with them",
    /// Why, in `create_scan_not_in_slot`. `{}`: the key's path, the kind's name.
    create_scan_not_for: "its key at m/{} is not for {}",
    /// Why, in `create_scan_not_in_slot`.
    create_scan_coin: "its path's coin type is not its key's network",
    /// Why, in `create_scan_not_in_slot`. `{}`: the kind's name.
    create_scan_no_key: "none of its keys is for {}",
    // ----- Make a new wallet: who signs, then a single key's Backup card (F240), and fewer presses, faraday-core/src/create.rs and screens/create.rs -----
    /// Backup card's row: the key backed up as its BIP-39 words.
    create_backup_words: "Words (BIP-39)",
    /// Backup card's row: Words' line.
    create_backup_words_line: "12 to 24 words on paper",
    /// Backup card's row: the key backed up as codex32 shares.
    create_backup_codex32: "Shares (codex32)",
    /// Backup card's row: Shares' line.
    create_backup_codex32_line: "The key in shares on paper, a few of them rebuild it",
    /// The word in the Shares row's name that carries the tooltip below: it is in the name as written there.
    create_backup_tip_word: "codex32",
    /// Tooltip on "codex32" in the Backup card's Shares row: an explanation on a working screen, by the owner's decision (2026-10-24, F112; moved to the Backup card by F240).
    create_backup_tip: "codex32 is the only method that makes a seed and its shares fully by hand, the checksum included.",
    /// Make a new wallet's Kind row: Single key's line.
    create_single_key_line: "One key signs; bc1q addresses, what most wallets make",
    /// Make a new wallet's Kind row: Multisig's line. `{}`: the keys, the signatures needed.
    create_multisig_line: "{} keys, any {} sign; bc1q addresses",
    /// Make a new wallet's Kind row: Single key's name.
    create_kind_single: "Single key",
    /// Make a new wallet's Kind row: Multisig's name.
    create_kind_multi: "Multisig",
    /// The Backup card's button that shows the other ways to back up the key.
    create_more_backup: "More ways",
    /// The Backup card's title, and Check's row label for a key made as codex32 shares.
    create_backup: "Backup",
    /// Check's row value. `{}`: the identifier, k, n.
    create_shares_set: "codex32 {} · {} of {}",
    /// Keys' foot once every slot is filled: makes the wallet.
    create_make_the_wallet_button: "Make the wallet",
    /// Check's foot: the addresses match the other software.
    create_it_matches: "It matches",
    /// Use vaults, closed before it is answered.
    create_not_answered: "Not answered",

        }
    };
}
