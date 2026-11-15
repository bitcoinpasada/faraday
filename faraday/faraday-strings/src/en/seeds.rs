//! Faraday's English words for the seeds area.

macro_rules! words_seeds {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- seed_read (faraday-core/src/seeds/read.rs) -----
    /// Error in `seed_read_act`.
    seed_read_type_passphrase: "Type the passphrase",
    /// Text in `seed_read_wallet`. `{}`: `n + 1`.
    seed_read_key_already: "Key {} · already here",
    /// Text in `seed_read_take`. `{}`: `k.label`.
    seed_read_key_passphrase: "{} · passphrase",
    /// Text in `sheet`.
    seed_read_not_this_one: "Not this one",
    /// Text in `sheet`.
    seed_read_cancel: "Cancel",
    /// Text in `sheet`.
    seed_read_use_passphrase: "Use with this passphrase",
    /// Text in `sheet`.
    seed_read_use_without: "Use without a passphrase",
    /// Text in `sheet`.
    seed_read_add_a_passphrase: "Add a passphrase",
    /// Label in `sheet`.
    seed_read_no_passphrase: "Without a passphrase",
    /// Label in `sheet`.
    seed_read_passphrase_label: "Passphrase",
    /// Label in `sheet`.
    seed_read_passphrase: "Passphrase again",
    /// Label in `sheet`.
    seed_read_with_passphrase: "With this passphrase",
    /// Text in `sheet`.
    seed_read_seedqr_read: "SeedQR read",
    /// The seed sheet's title for a seed's words read (`docs/SCANNING.md` §6).
    seed_read_words_read: "Words read",
    /// The seed sheet's title for a seed opened from a QR vault.
    seed_read_qr_vault_read: "QR vault read",
    /// The seed sheet's title for a CompactSeedQR read.
    seed_read_compact_read: "CompactSeedQR read",
    /// The seed sheet's line for a key a loaded wallet has. `{}`: its place in the wallet, the wallet.
    seed_read_key_of: "Key {} of {}",
    // ----- seeds (faraday-core/src/seeds/mod.rs) -----
    /// Error in `seeds_take_file`.
    seeds_no_account_key: "No account key in that file",
    /// Error in `seeds_descriptor`.
    seeds_type_a_derivation: "Type a derivation path, like m/48'/0'/0'/2'",
    /// Error in `seeds_descriptor`.
    seeds_seed_unloaded: "A seed is no longer loaded",
    /// Error in `seeds_descriptor`.
    seeds_no_seed_yet: "No seed yet",
    /// Text in `seeds_descriptor`. `{}`: `at`, `crate::wallet::network_name(kn)`, `crate::wallet::network_name(net)`.
    seeds_key_is_a_key: "Key {} is a {} key; this session is on {}",
    /// Error in `seeds_descriptor`. `{}`: `at`.
    seeds_key_has_no_account: "Key {} has no account key yet",
    /// Error in `seeds_descriptor`. `{}`: `at`.
    seeds_not_account_key: "Key {} is not an account key",
    /// Error in `seeds_descriptor`. `{}`: `keys.len()`, `s.n`.
    seeds_of_keys_in_hand: "{} of {} keys in hand",
    /// Toast in `seeds_from_vault`. `{}`: `want`, `self.vaults.open[v].name`.
    seeds_loaded_from_vault: "{} loaded from {}",
    /// Text in `seeds_make`.
    seeds_typed_seed: "Typed seed",
    /// Text in `seeds_make`.
    seeds_typed_seeds: "Typed seeds",
    // ----- seeds (faraday-core/src/seeds/screen.rs) -----
    /// Text in `more_ways`. `{}`: `fp`, `name`.
    seeds_load_from_vault: "Load {} from {}",
    /// Text in `quorum`. `{}`: `s.m.max(1)`, `n`.
    seeds_quorum: "{} of {}",
    /// Text in `quorum`.
    seeds_one_key_signs: "one key signs",
    /// Text in `quorum`. `{}`: `s.m`, `if s.m == 1 { EN.seeds_signature } el…`, `n`.
    seeds_of_keys: "{} {} of {} keys",
    /// Text in `quorum`.
    seeds_signature: "signature",
    /// Text in `quorum`.
    seeds_signatures_inline: "signatures",
    /// Text in `quorum`.
    seeds_signatures: "Signatures",
    /// Text in `quorum`.
    seeds_keys: "Keys",
    /// Button in `cosigner`.
    seeds_change: "Change",
    /// Label in `cosigner`.
    seeds_type_or_paste: "Type or paste an xpub",
    /// Line in `cosigner`.
    seeds_not_an_account: "Not an account key",
    /// Text in `cosigner`. `{}`: `n`.
    seeds_use: "Use {}",
    /// Text in `cosigner`.
    seeds_scan: "Scan",
    /// Line in `shape`.
    seeds_cosigners_account: "Cosigners' account keys (xpub)",
    /// Line in `shape`.
    seeds_kind: "Kind",
    /// Line in `shape`.
    seeds_derivation_path: "Derivation path",
    /// Line in `shape`.
    seeds_standard: "Standard",
    /// Text in `shape`.
    seeds_account: "Account",
    /// Line in `shape`.
    seeds_custom_path: "Custom path",
    /// Line in `shape`.
    seeds_first_receive: "First receive address · 0/0",
    /// Text in `shape`.
    seeds_back_to_the_seeds: "Back to the seeds",
    /// Text in `shape`.
    seeds_make_the_wallet: "Make the wallet",
        }
    };
}
