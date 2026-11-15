//! Faraday's English words for the lightning area.

macro_rules! words_lightning {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- lightning (faraday-core/src/lightning/mod.rs) -----
    /// Error in `lightning_resolve`.
    lightning_this_key: "This key has no BIP-39 seed",
    /// Error in `lightning_resolve`. `{}`: `words.len()`.
    lightning_an_aezeed: "An aezeed is 24 words; {} are typed",
    /// Error in `lightning_resolve`. `{}`: `k + 1`.
    lightning_word_not_in_list: "Word {} is not on the list",
    /// Text in `lightning_resolve`. `{}`: `e`.
    lightning_not_an_aezeed: "Not an aezeed for this passphrase: {}",
    /// Toast in `lightning_act`.
    lightning_no_vault: "No vault is open",
    /// Text in `lightning_act`. `{}`: `id`, `private.as_str()`.
    lightning_lightning: "Lightning node key\nnode id {}\nprivate key {}",
    /// Toast in `lightning_act`.
    lightning_the_node: "The node key is in the vault",
    /// Text in `lightning_act`.
    lightning_a_lightning: "A Lightning node's private key",
    /// Text in `lightning_act`.
    lightning_whoever_has: "Whoever has it is the node: its channels and their money",
    // ----- lightning (faraday-core/src/lightning/screen.rs) -----
    /// Text in `draw`.
    lightning_a_lightning_node: "A Lightning node's key, made the way ldk-node or LND makes it. Its public key is the node id, safe to share; its private key is the node, and goes into a vault.",
    /// Line in `header`.
    lightning_wallets: "Wallets",
    /// Title in `header`.
    lightning_lightning_node: "Lightning node key",
    /// Text in `body`. `{}`: `crate::wallet::key_line(fp, &k.label)`.
    lightning_ldk_node: "ldk-node · {}",
    /// Text in `body`. `{}`: `crate::wallet::key_line(fp, &k.label)`.
    lightning_cln_node: "Core Lightning · {}",
    /// Text in `body`.
    lightning_lnd_type: "LND · type an aezeed",
    /// Text in `body`.
    lightning_the_24_words: "The 24 words",
    /// Text in `body`. `{}`: `n`.
    lightning_word_count: "{} of 24",
    /// Line in `body`.
    lightning_passphrase: "Passphrase",
    /// Text in `body`.
    lightning_aezeed_lnd: "aezeed (LND's default)",
    /// Button in `body`.
    lightning_read_button: "Read it",
    /// Line in `body`.
    lightning_node_id: "Node id",
    /// Line in `body`.
    lightning_made: "Made",
    /// Line in `body`.
    lightning_private_key: "Private key",
    /// Text in `body`.
    lightning_hide: "Hide",
    /// Text in `body`.
    lightning_show: "Show",
    /// Text in `body`. `{}`: `v.name`.
    lightning_save_into: "Save into {}",
    /// Text in `body`.
    lightning_out_unprotected: "Out unprotected…",
    /// Text in `body`.
    lightning_cln_type: "Core Lightning · hsm_secret",
    /// Text in `body`.
    lightning_the_words: "The 12 to 24 words",
    /// Text in `body`. `{}`: `n`.
    lightning_words_typed: "{} words",
    /// Text in `body`.
    lightning_no_passphrase: "none",
    /// Button in `body`. `{}`: the file's name.
    lightning_use_file: "Use {}",
    /// Line in `body`.
    lightning_no_hsm_file: "No hsm_secret in Files",
    /// Lightning node key, Core Lightning: the file chosen is gone from Files; its name.
    lightning_hsm_file_gone: "{} is no longer in Files",
    /// Error in `read_done`: the mnemonic form's words.
    lightning_hsm_words: "The words in the file are not English BIP-39 words in Core Lightning's form",
    /// Error in `read_done`.
    lightning_hsm_passphrase: "Wrong passphrase",
    /// Error in `read_done`.
    lightning_hsm_passphrase_long: "The passphrase is longer than 256 bytes",
    /// Error in `read_done`, and when the file chosen changed in Files.
    lightning_hsm_opening: "Wrong passphrase or a changed file",
    /// Error in `read_done`.
    lightning_hsm_memory: "Not enough memory to open the file",
    /// Error in `resolve` and `read_words`.
    lightning_hsm_not_english: "Core Lightning takes English words only",
    /// Sample file in Files, plain.
    sample_hsm_plain: "hsm_secret",
    /// Sample file in Files, encrypted under the public passphrase.
    sample_hsm_encrypted: "hsm_secret-encrypted",
    /// Sample file in Files, the words form.
    sample_hsm_words: "hsm_secret-words",
        }
    };
}
