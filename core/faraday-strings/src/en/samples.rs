//! Faraday's English words for the samples area.

macro_rules! words_samples {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- samples (faraday-core/src/samples/) -----
    /// The Load samples pill, on the compact bar and Home.
    samples_pill_short: "Samples",
    /// Title of the Load samples sheet.
    samples_title: "Load samples",
    /// Heading of the sample wallets in Load samples.
    samples_wallets: "Wallets",
    /// Heading of the sample keys in Load samples.
    samples_keys: "Keys",
    /// Link in Load samples while none is ticked.
    samples_select_all: "Select all",
    /// Link in Load samples while any is ticked.
    samples_unselect_all: "Unselect all",
    /// Button in Load samples.
    samples_not_now: "Not now",
    /// Button in Load samples.
    samples_load: "Load ticked",
    /// Line in Load samples, followed by the passphrase in a code box.
    samples_password: "In Practice, the sample passphrase is",
    /// Toast after Load ticked. `{}`: what loaded, made of `samples_n_*`.
    samples_loaded: "Loaded {}",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_wallet: "{} wallet",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_wallets: "{} wallets",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_key: "{} key",
    /// Text in `loaded_text`. `{}`: how many.
    samples_n_keys: "{} keys",
    /// Text in `loaded_text`. `{}`: the wallets, the keys.
    samples_loaded_pair: "{} and {}",
    /// Where a sample wallet says it came from.
    samples_source: "Sample",
    /// Key row in Load samples: the key the user guide makes from its
    /// published dice rolls, and its fingerprint.
    sample_guide: "Guide key · 3dca6805",
    /// Key row in Load samples.
    sample_abandon: "abandon … about",
    /// Key row in Load samples.
    sample_japanese: "Japanese words",
    /// Key row in Load samples.
    sample_slip39_one: "SLIP-39, one share",
    /// Key row in Load samples.
    sample_slip39_two_of_three: "SLIP-39, 2 of 3",
    /// Key row in Load samples.
    sample_slip39_groups: "SLIP-39, groups",
    /// Key row in Load samples.
    sample_codex32_secret: "codex32 secret",
    /// Key row in Load samples.
    sample_codex32_shares: "codex32, 2 shares",
    /// Key row in Load samples.
    sample_xor: "Seed XOR, 2 parts",
    /// Name of the LND cipher seed sample.
    sample_aezeed: "LND cipher seed",
    /// File name of the sample vault, which Use a sample puts in the Inbox.
    sample_vault_file: "sample.kdbx",
    /// File name of the sample signed message, which Use a sample puts in
    /// the Inbox.
    sample_message_file: "sample-message.txt",
    /// Wallet row in Load samples.
    sample_savings: "Savings",
    /// Wallet row in Load samples.
    sample_spending: "Spending",
    /// Wallet row in Load samples.
    sample_taproot: "Taproot",
    /// Wallet row in Load samples.
    sample_nested: "Nested SegWit",
    /// Wallet row in Load samples.
    sample_legacy: "Legacy",
    /// Wallet row in Load samples.
    sample_nested_multisig: "Nested multisig",
    /// Wallet row in Load samples.
    sample_legacy_multisig: "Legacy multisig",
    /// Wallet row in Load samples.
    sample_taproot_multisig: "Taproot multisig",
    /// Wallet row in Load samples.
    sample_inheritance: "Inheritance",
    /// Wallet row in Load samples.
    sample_tree: "Taproot tree",
    /// Wallet row in Load samples.
    sample_musig: "MuSig2",
    /// Wallet row in Load samples.
    sample_three_of_five: "3 of 5",
    /// What a Load samples row is: its kind and its script, or its form and its size. `{}`: the two parts.
    sample_type: "{} · {}",
    /// Kind of a sample wallet with one key.
    sample_kind_single: "single-sig",
    /// Kind of a sample multisig wallet. `{}`: signatures required, keys.
    sample_kind_multisig: "{}-of-{} multisig",
    /// Kind of a sample MuSig2 wallet. `{}`: keys.
    sample_kind_musig: "{}-key MuSig2",
    /// Kind of a sample wallet with a key and a second that spends after a delay.
    sample_kind_timelock: "timelock recovery",
    /// Script of a sample Taproot wallet with a tree of scripts.
    sample_script_tree: "Taproot, script tree",
    /// Form of a sample key made of BIP-39 words.
    sample_form_bip39: "BIP-39",
    /// Form of a sample key made of SLIP-39 shares.
    sample_form_slip39: "SLIP-39",
    /// Form of a sample key made of codex32 strings.
    sample_form_codex32: "codex32",
    /// Form of a sample key made of Seed XOR parts.
    sample_form_xor: "Seed XOR",
    /// Form of a sample key that is a master seed.
    sample_form_seed: "Master seed",
    /// Form of the sample LND cipher seed.
    sample_form_aezeed: "LND cipher seed",
    /// Size of a sample key of BIP-39 words. `{}`: how many.
    sample_size_words: "{} words",
    /// Size of a sample key of Japanese BIP-39 words. `{}`: how many.
    sample_size_words_japanese: "{} words, Japanese",
    /// Size of a sample key of one share.
    sample_size_share: "1 share",
    /// Size of a sample key of shares. `{}`: how many.
    sample_size_shares: "{} shares",
    /// Size of a sample SLIP-39 set. `{}`: shares that recover the key, shares dealt.
    sample_size_threshold: "{} of {} shares",
    /// Size of a sample SLIP-39 set in groups. `{}`: shares.
    sample_size_groups: "groups, {} shares",
    /// Size of the sample codex32 secret.
    sample_size_secret: "secret",
    /// Size of a sample Seed XOR set. `{}`: parts.
    sample_size_parts: "{} parts",
    /// Size of a sample master seed. `{}`: bytes.
    sample_size_bytes: "{} bytes",
        }
    };
}
