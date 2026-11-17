//! Faraday's English words for the scan area.

macro_rules! words_scan {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- A scanned code refused (faraday-core/src/qr_view.rs, faraday-qr's kinds) -----
    /// Under the scanner: a code too long to read.
    scan_refused_too_long: "A code over 4,096 characters",
    /// Under the scanner. `{}`: what is wrong, one of the `scan_bbqr_malformed_` strings.
    scan_refused_bbqr_malformed: "Not a BBQr part: {}",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_short: "too short",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_encoding: "an encoding BBQr does not have",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_count: "a part count that is not base36",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_number: "a part number past the count",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_data: "its data is not in its encoding",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_uneven: "its parts are of uneven length",
    /// What is wrong with a BBQr part.
    scan_bbqr_malformed_inflate: "its compressed data does not inflate within 360 KiB",
    /// Under the scanner. `{}`: the BBQr file type, a letter.
    scan_refused_bbqr_type: "A BBQr file of type {}, which Faraday does not read",
    /// Under the scanner.
    scan_refused_bbqr_other: "A part of another BBQr transfer. Start again to read that one",
    /// Under the scanner.
    scan_refused_bbqr_conflict: "A BBQr part arrived twice with different contents",
    /// Under the scanner.
    scan_refused_bbqr_large: "The BBQr transfer is over 360 KiB",
    /// Under the scanner.
    scan_refused_numbered_other: "A numbered part of another set. Start again to read that one",
    /// Under the scanner.
    scan_refused_numbered_conflict: "A numbered part arrived twice with different text",
    /// Under the scanner.
    scan_refused_numbered_large: "The numbered parts come to more than 360 KiB",
    /// Under the scanner.
    scan_refused_bbqr_text: "A BBQr text that is not UTF-8",
    /// Under the scanner.
    scan_refused_bbqr_json: "BBQr JSON that is not UTF-8",
    /// Under the scanner: a UR registry type refused. `{}`: the UR type, or PSBT.
    ur_registry_shape: "The {} in that code is not in the registry's shape",
    /// Under the scanner: a UR registry type refused.
    ur_registry_seed: "That is a seed (ur:crypto-seed). Faraday takes a seed as its words or a SeedQR, in Add a key",
    /// Under the scanner: a UR registry type refused.
    ur_registry_path: "A key path in that code is not in the registry's shape",
    /// Under the scanner: a UR registry type refused.
    ur_registry_key: "The key in that code is not in the registry's shape",
    /// Under the scanner: a UR registry type refused.
    ur_registry_private_key: "That is a private key. Faraday takes keys as their words or a SeedQR, in Add a key",
    /// Under the scanner: a UR registry type refused.
    ur_registry_no_chain_code: "The key in that code has no chain code, so no addresses come from it",
    /// Under the scanner: a UR registry type refused.
    ur_registry_wallet: "A wallet in that code is not in the registry's shape",
    /// Under the scanner: a UR registry type refused.
    ur_registry_not_extended: "A key in that wallet is not an extended key, which Faraday reads",
    /// Under the scanner: a UR registry type refused.
    ur_registry_no_threshold: "A multisig in that code has no threshold",
    /// Under the scanner: a UR registry type refused.
    ur_registry_no_multisig_keys: "A multisig in that code has no keys",
    /// Under the scanner: a UR registry type refused. `{}`: the script's tag number.
    ur_registry_script: "A wallet script (tag {}) Faraday does not read",
    /// Under the scanner: a UR registry type refused.
    ur_registry_no_fingerprint: "The account in that code has no master fingerprint",
    /// Under the scanner: a UR registry type refused.
    ur_registry_no_account_keys: "The account in that code has no keys",
    /// Under the scanner: a UR registry type refused.
    ur_registry_no_key_read: "The account in that code holds no key Faraday reads",
    /// A Faraday file envelope refused, scanned or to be sent.
    envelope_size: "A file from 1 byte to 256 KiB",
    /// A Faraday file envelope refused.
    envelope_kind: "An envelope kind Faraday OS does not write",
    /// A Faraday file envelope refused.
    envelope_name: "A name of 1 to 128 letters, digits, dots, hyphens or underscores",
    /// A Faraday file envelope refused.
    envelope_too_large: "The envelope is over 360 KiB",
    /// A Faraday file envelope refused.
    envelope_not: "Not a Faraday file envelope",
    /// A Faraday file envelope refused.
    envelope_no_name: "The envelope has no name",
    /// A Faraday file envelope refused.
    envelope_no_kind: "The envelope has no kind",
    /// A Faraday file envelope refused.
    envelope_no_data: "The envelope has no data",
    /// A Faraday file envelope refused.
    envelope_not_base64: "The envelope's data is not base64",
    /// A Faraday file envelope refused.
    envelope_mismatch: "The envelope's size, hash or fields do not match its file",
    // ----- What a code read is (docs/SCANNING.md §7.2), faraday-core/src/scan_read.rs -----
    /// A code read: a seed's words.
    read_seed_words: "A seed's words",
    /// A code read: a SeedQR.
    read_seedqr: "A SeedQR",
    /// A code read: a CompactSeedQR.
    read_compact_seedqr: "A CompactSeedQR",
    /// A code read: a codex32 string with share index s.
    read_codex32_secret: "A codex32 secret",
    /// A code read: a codex32 string with another share index.
    read_codex32_share: "A codex32 share",
    /// A code read: a SLIP-39 share.
    read_slip39_share: "A SLIP-39 share",
    /// A code read: words of a BIP-39 list whose checksum fails.
    read_bad_checksum: "Words that fail their checksum",
    /// A code read: 24 words that are an LND aezeed by its own checksum.
    read_aezeed: "An LND aezeed",
    /// A code read: a WIF private key.
    read_private_key: "A private key",
    /// A code read: an extended private key.
    read_xprv: "An extended private key",
    /// A code read: a Nostr secret key.
    read_nostr_secret: "A Nostr secret key",
    /// The read card's kind: BIP-392's silent payments spend key.
    read_spend_key: "A silent payments spend key",
    /// A code read: ur:crypto-seed.
    read_ur_seed: "A seed (ur:crypto-seed)",
    /// A code read: a GPG key's paperkey.
    read_paperkey: "A paperkey",
    /// A code read: a TOTP setup code.
    read_totp: "A TOTP setup code",
    /// A code read: vault entries.
    read_entries: "Vault entries",
    /// A code read: a PSBT.
    read_psbt: "A transaction to sign",
    /// A code read: a finished transaction.
    read_transaction: "A finished transaction",
    /// A code read: a descriptor.
    read_wallet: "A descriptor",
    /// A code read: a multisig config with keys left off.
    read_split_sheets: "Split sheets of a wallet",
    /// A code read: an account key.
    read_account_key: "An account key",
    /// A code read: an address.
    read_address: "An address",
    /// A code read: a bitcoin: URI.
    read_payment_request: "A payment request",
    /// A code read: a silent payment address.
    read_silent_address: "A silent payment address",
    /// A code read: a signed message.
    read_message: "A signed message",
    /// A code read: a GPG public key.
    read_gpg: "A GPG public key",
    /// A code read: a file kept as it came (an envelope, a vault, a PDF).
    read_file: "A file",
    /// A code read: BIP-329 labels.
    read_labels: "Labels (BIP-329)",
    /// A code read: a Lightning invoice.
    read_invoice: "A Lightning invoice or offer",
    /// A code read: a web address.
    read_web: "A web address",
    /// A code read: text that is none of the kinds above.
    read_text: "Text",
    /// A code read: bytes that are none of the kinds above.
    read_unreadable: "Nothing Faraday reads",
    /// A scanner's line for a kind it does not take. `{}`: the kind, one of the `read_*` above.
    read_not_here: "{} · not used here",
    // ----- The read card (docs/SCANNING.md §5, §7.2, §7.3), faraday-core/src/scan_key.rs -----
    /// The read card's first fact's label: what the code read is.
    read_label: "Read",
    /// The read card's fact for a seed. `{}`: its key's fingerprint without a passphrase.
    read_key_fact: "Key {}",
    /// The read card's fact for a share. `{}`: its index, its set's identifier, how many make the key.
    read_share_fact: "Share {} of {} · {} needed",
    /// The read card's fact for a PSBT. `{}`: `read_psbt_inputs` or `read_psbt_input`, the loaded wallet it spends from.
    read_psbt_fact: "{} · {}",
    /// A PSBT's inputs, on the read card. `{}`: how many, more than one.
    read_psbt_inputs: "{} inputs",
    /// A PSBT's one input, on the read card.
    read_psbt_input: "1 input",
    /// The read card's fact for a TOTP setup code. `{}`: its issuer, its account.
    read_totp_fact: "{} · {}",
    /// The read card's way over for a seed: the seed sheet.
    read_go_key: "Add this key",
    /// The read card's way over for a share: its form in Add a key.
    read_go_share: "Add this share",
    /// The read card's way over for a TOTP code. `{}`: the open vault.
    read_go_vault: "Add to {}",
    /// The read card's way over for a TOTP code with a vault in Files but none open. `{}`: the vault.
    read_go_unlock: "Unlock {}",
    /// The read card's way over for a TOTP code with no vault at all.
    read_go_make_vault: "Make a new vault",
    /// The read card's line under Make a vault: the code read is not carried on.
    read_not_kept: "Not kept · scan it again once the vault is open",
    /// The read card's way over for a PSBT.
    read_go_sign: "Sign it",
    /// The read card's way over for a PSBT of the spend under way.
    read_go_collect: "Add to the signatures",
    /// The read card's way over for a finished transaction.
    read_go_decode: "Decode it",
    /// The read card's way over for a descriptor.
    read_go_wallet: "Load the wallet",
    /// The read card's way over for split sheets.
    read_go_restore: "Restore the wallet",
    /// The read card's way over for a signed message.
    read_go_check: "Check the signature",
    /// The read card's way over for text: a note in the open vault.
    read_go_note: "Save as a note",
    /// The read card's way over for text: Sign a message with it.
    read_go_message: "Sign this message",
    /// The address check's way over for a find in a single-key wallet, or in a loaded key's account: Sign a message at that address.
    read_go_sign_address: "Sign a message with this address",
    /// The read card's way over for a public kind: filed in Files.
    read_keep: "Keep in Files",
    /// The read card's kind for a Krux encrypted code.
    read_kef: "An encrypted code",
    /// A code read: a QR vault.
    read_qr_vault: "A QR vault",
    /// The read card's way over for a QR vault: the QR vault tool.
    read_go_qr_vault: "Open in QR vault",
    /// A file in Files that is a QR vault: its button.
    files_open_qr_vault: "Open in QR vault",
    /// A file in Files that is a QR vault: its action in the file menu.
    inbox_actions_open_qr_vault: "Open in QR vault",
    /// The card's version line: the version number and its name.
    kef_version_fact: "Version {} · {}",
    /// The card's id line, the id as text or as 0x hex.
    kef_id_fact: "Id {}",
    /// The card's count line: rounds of PBKDF2 and the time here.
    kef_count_fact: "{} rounds · {}",
    /// The card's count line before the time here is known.
    kef_count_only: "{} rounds",
    /// The card's time here, under a second.
    kef_under_a_second: "under a second",
    /// The card's time here in seconds.
    kef_seconds: "about {} s",
    /// The card's time here in minutes.
    kef_minutes: "about {} min",
    /// The card's line for a count above what Krux itself writes.
    kef_count_high: "More rounds than Krux writes",
    /// The second press of Open on a code above what Krux writes.
    kef_open_anyway: "Open anyway",
    /// The card's action: open the code.
    kef_open: "Open",
    /// The card's action: the code is something else.
    kef_not_encrypted: "Not encrypted",
    /// The seed sheet's action for a CompactSeedQR that also reads as an encrypted code.
    kef_open_as: "Open as an encrypted code",
    /// The passphrase sheet's title.
    kef_sheet_title: "Encrypted code",
    /// The passphrase field's label.
    kef_passphrase: "Passphrase",
    /// The label of the passphrase typed a second time.
    kef_passphrase_again: "Passphrase again",
    /// The hex field's label.
    kef_hex: "Passphrase bytes in hex",
    /// The action that switches to the hex field.
    kef_enter_hex: "Enter as hex",
    /// The action that switches to the typed field.
    kef_enter_text: "Enter as text",
    /// The one message for every failure to open.
    kef_failed: "Could not open. Wrong passphrase or damaged code.",
    /// A hex field that is not whole bytes.
    kef_hex_bad: "Not whole bytes of hex.",
    /// The working line while the key is worked out.
    kef_working: "Opening the code",
    /// Stops the opening.
    kef_cancel: "Cancel",
    /// The sheet's title for contents that are a note.
    kef_note_title: "Decrypted text",
    /// The sheet's title for contents that are bytes.
    kef_bytes_title: "Decrypted bytes",
    /// The note's action: carry the text to the next seed's passphrase.
    kef_use_passphrase: "Use as a passphrase",
    /// The note's action: its 16 or 32 bytes are read as a seed's entropy.
    kef_read_as_seed: "Read as a seed",
    /// The sheet's close action.
    kef_done: "Done",
    /// A note the opening left: the check passed only after NULs were put back.
    kef_retried: "The writer left out NUL padding.",
    /// The seed sheet's line for a code id of 8 hex characters: it matches the key.
    kef_id_matches: "Code id {} matches this key",
    /// The seed sheet's line for a code id of 8 hex characters: it differs from the key.
    kef_id_differs: "Code id {} differs from this key",
    /// The seed sheet's line for an id that is not a fingerprint.
    kef_id_line: "Code id {}",
    /// The read card's Scan again: the code wiped, the camera back on.
    read_again: "Scan again",
    /// The seed sheet's text button for 12 or 24 words: Add a key's Seed XOR form, the words in.
    read_go_xor: "It is a Seed XOR part",
    /// The read card's way over for an LND aezeed: Lightning, the aezeed typed in.
    read_go_lightning: "Use in Lightning",
    /// A copy check's line: the SeedQR scanned is a whole seed, but another one.
    read_another_seed: "This is another seed's SeedQR",
    /// A scanner's title (`docs/SCANNING.md` §7.1): Restore's Scan it and Scan descriptor.
    scan_title_description: "Scan the descriptor",
    /// A scanner's title: Create's key slot. `{}`: the key's number.
    scan_title_xpub: "Scan Key {}'s xpub",
    /// A scanner's title: Restore's seeds card.
    scan_title_cosigner: "Scan a cosigner's key",
    /// A scanner's title: Spend's Scan a copy, the Spend tab's Scan it and Scan the PSBT, Tools' Sign a transaction.
    scan_title_psbt: "Scan the PSBT",
    /// A scanner's title: Tools' Decode a transaction.
    scan_title_transaction: "Scan a transaction",
    /// Title in `scan_screen`.
    scan_title_invoice: "Scan an invoice",
    /// A scanner's title: Tools' Check an address.
    scan_title_address: "Scan an address",
    /// A scanner's title: an Add a key form's Scan.
    scan_title_part: "Scan a share",
    /// A scanner's title: a vault's Scan a setup code.
    scan_title_totp: "Scan a setup code",
    /// The line under a way over that loads a key, dimmed while a stick is in.
    read_need_no_stick: "Remove the stick first",
    // ----- paste (faraday-core/src/scan_key.rs) -----
    /// Note in `pasted`: the clipboard held a seed's words.
    pasted_words: "Those are a seed's words. No seed is taken here",
    /// Note in `pasted`: the clipboard held a SeedQR's digits.
    pasted_seedqr: "That is a SeedQR. No seed is taken here",
    /// Note in `pasted`: the clipboard held a private key.
    pasted_private_key: "That is a private key. A private key is never pasted",
    /// Note in `pasted`: the clipboard held a codex32 string or a SLIP-39
    /// share.
    pasted_share: "That is a share of a key. No seed is taken here",
    /// Line in `scan_sheet` for two seconds after Paste found no text.
    pasted_nothing: "Nothing to paste",
    // ----- scan_key (faraday-core/src/scan_key.rs) -----
    /// Text in `not_a_wallet_key`. `{}`: `fp_text(fp)`, `name`.
    scan_key_is_not_one: "{} is not one of {}'s keys",
    /// Text in `wallet_key_lines`. `{}`: `n + 1`, `fp_text(fp)`.
    scan_key_key: "Key {} · {}",
    /// Note in `wallet_key_scanned`.
    scan_key_not_a_seedqr: "Not a SeedQR or a CompactSeedQR",
    /// Note in `wallet_key_scanned`. `{}`: `n + 1`.
    scan_key_key_already: "Key {} · already here",
    /// Toast in `wallet_key_said`. `{}`: `n + 1`, `short`.
    scan_key_key_added_more: "Key {} added · {} more",
    /// Toast in `wallet_key_said`. `{}`: `n + 1`.
    scan_key_key_added: "Key {} added · Can spend",
        }
    };
}
