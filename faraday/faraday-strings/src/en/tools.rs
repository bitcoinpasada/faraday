//! Faraday's English words for the tools area.

macro_rules! words_tools {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- tools (faraday-core/src/tools/screen.rs) -----
    /// Text in `GUIDE`.
    tools_calculators_each: "Calculators: each takes what you type, works one thing out and keeps nothing. None of them touches a key; an extended private key is refused.",
    /// Title in `header`.
    tools_tools: "Tools",
    /// Title of the input card of a tool drawn as a flow (`tools/flow.rs`).
    tools_card_input: "Input",
    /// Button at the foot of Units' left pane: adds an amount (a tab).
    tools_add_amount: "Add an amount",
    /// A Units tab: the amount typed, then its unit. `{}`: `amount`, `unit`.
    tools_amount_tab: "{} {}",
    /// A Units tab with nothing typed yet. `{}`: `unit`.
    tools_amount_tab_empty: "0 {}",
    /// Title of the result card of a tool drawn as a flow (`tools/flow.rs`).
    tools_card_result: "Result",
    /// Button under the input card of a tool drawn as a flow: opens the result.
    tools_next: "Next",
    /// Text in `body`.
    tools_read_as_automatic: "Read as: automatic",
    /// Text in `body`.
    tools_text: "Text",
    /// Text in `body`.
    tools_hex: "Hex",
    /// Text in `body`. `{}`: `n`.
    tools_unit_option: "In {}",
    /// Text in `body`.
    tools_text_or_hex: "Text or hex",
    /// Text in `body`.
    tools_base58_bech32: "Base58, bech32 or hex",
    /// Text in `body`.
    tools_a_descriptor: "A descriptor",
    /// Text in `body`.
    tools_an_xpub_ypub: "An xpub, ypub, zpub, tpub…",
    /// Text in `body`.
    tools_an_amount: "An amount",
    /// Text in `body`.
    tools_a_policy_e_g: "A policy, e.g. or(pk(9a6a2580),and(pk(…),older(1000)))",
    /// Button in `body`.
    tools_clear: "Clear",
    /// Button under the field of Convert an xpub, Descriptor checksum and Miniscript: reads a code into it.
    tools_scan: "Scan",
    /// The pick's way in that types into the field.
    tools_way_type: "Type it",
    /// The same tools' button with no file in Files to read.
    tools_from_files: "From Files",
    /// The same tools' button, one file of Files.
    tools_from_file: "From Files · {}",
    /// Title of the scanner those tools open.
    tools_scan_title: "Scan the text",
    /// A file or code that does not fit in the field.
    tools_too_long: "Too long for the field",
    /// Text in `body`.
    tools_encoding: "Encoding",
    /// Encodings' Encoding value: Base58 with its check.
    tools_encoding_base58check: "Base58Check",
    /// Encodings' Encoding value: Base58 with nothing appended.
    tools_encoding_base58: "Base58",
    /// Encodings' Encoding value: BIP-173.
    tools_encoding_bech32: "bech32",
    /// Encodings' Encoding value: BIP-350.
    tools_encoding_bech32m: "bech32m",
    /// Encodings' Encoding value: hex digits.
    tools_encoding_hex: "Hex",
    /// Text in `body`.
    tools_prefix: "Prefix",
    /// Text in `body`.
    tools_version: "Version",
    /// Text in `body`.
    tools_bytes: "Bytes",
    /// Note in `body`.
    tools_not_base58_bech32: "Not Base58, bech32, bech32m or hex",
    /// Text in `body`.
    tools_checksum: "Checksum",
    /// Text in `body`.
    tools_given: "Given",
    /// Text in `body`.
    tools_holds: "holds",
    /// Text in `body`.
    tools_does_not_hold: "does not hold",
    /// Text in `body`.
    tools_with_checksum_label: "With it",
    /// A calculator's descriptor result: puts the descriptor in Wallets.
    tools_take_to_wallets: "Take to Wallets",
    /// A wallet's source line when it came from a calculator.
    tools_typed_in_a_calculator: "Typed in a calculator",
    /// Text in `body`.
    tools_not_loadable: "not one this device loads",
    /// Note in `body`.
    tools_not_a_descriptor: "Not a descriptor",
    /// Text in `body`.
    tools_legacy: "Legacy",
    /// Text in `body`.
    tools_nested_segwit: "Nested SegWit",
    /// Text in `body`.
    tools_native_segwit: "Native SegWit",
    /// Text in `body`.
    tools_taproot: "Taproot",
    /// Text in `body`.
    tools_depth: "Depth",
    /// Text in `body`.
    tools_fingerprint: "Fingerprint",
    /// Text in `body`.
    tools_child: "Child",
    /// Text in `body`.
    tools_that_is_an_extended: "That is an extended private key: this tool takes public keys only",
    /// Note in `body`.
    tools_not_an_extended: "Not an extended key",
    /// Note in `body`.
    tools_not_hex: "Not hex",
    /// Text in `body`.
    tools_descriptor: "Descriptor",
    /// Text in `body`.
    tools_spend_paths: "Spend paths",
    /// Text in `body`.
    tools_wallet: "Wallet",
    /// Label in `body`: the keys a descriptor uses twice.
    tools_keys: "Keys",
    /// Text in `repeated_key_line`. `{}`: the later key's number, the
    /// earlier key's number.
    tools_key_same_as: "Key {} is the same as key {}",
    /// Text in `repeated_key_line`. `{}`: the key's number.
    tools_key_used_twice: "Key {} is used twice",
    /// Text in `body`.
    tools_one_this_device: "one this device loads",
    /// Text in `body`.
    tools_keys_need_origins: "keys need origins and both chains to load",
    /// The Miniscript compiler refuses what is typed. `{}`: the
    /// compiler's own message.
    tools_miniscript_not_policy: "Not a policy: {}",
    /// The Miniscript compiler refuses what is typed. `{}`: the
    /// compiler's own message.
    tools_miniscript_no_script: "No script compiles from this policy: {}",
    /// The Miniscript compiler is given one key alone under Taproot: the
    /// key is the internal key and no script is left.
    tools_miniscript_lone_key: "One key alone has no script under Taproot",
        }
    };
}
