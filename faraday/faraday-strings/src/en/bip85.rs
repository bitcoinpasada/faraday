//! Faraday's English words for the bip85 area.

macro_rules! words_bip85 {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- bip85 (faraday-core/src/bip85/mod.rs) -----
    /// Convert a seed's heading and its name on a small panel.
    convert_title: "Convert a seed",
    /// Convert a seed's way back.
    convert_tools: "Tools",
    /// A Convert a seed card.
    convert_from_title: "From",
    /// A Convert a seed card.
    convert_to_title: "To",
    /// A Convert a seed card.
    convert_result_title: "Result",
    /// A card's summary with nothing chosen.
    convert_not_chosen: "Not chosen",
    /// From's way to load another key.
    convert_add_a_key: "Add a key",
    /// A key loaded from words, by how many.
    convert_from_words: "BIP-39 words, {}",
    /// A key loaded from codex32, by the string's length.
    convert_from_codex32: "codex32, {} characters",
    /// A key loaded from a master seed in another form.
    convert_from_seed: "Master seed",
    /// From's way to read an LND aezeed, and the name of the source once read.
    convert_lnd_aezeed: "An LND aezeed",
    /// What the aezeed read for Convert was read from, under From.
    convert_from_aezeed: "LND aezeed, 24 words",
    /// A fact's label on Result for an aezeed.
    convert_birthday: "Birthday",
    /// A fact's label on Result for an aezeed.
    convert_channels: "Channels",
    /// What explains the aezeed's entry opened from Convert.
    convert_aezeed_guide: "An LND aezeed's 24 words and its passphrase. They are read for this tool and held only until it is left.",
    /// The channels fact for an aezeed.
    convert_aezeed_onchain: "On-chain coins only: channels come back through LND's channel backup",
    /// A line of the Paths fact for an aezeed. `{}` three times: the coin type, 0 or 1.
    convert_aezeed_paths: "BIP-84 m/84'/{}'/0'; BIP-49 m/49'/{}'/0' with LND's own change addresses; BIP-86 m/86'/{}'/0'; key families under m/1017'",
    /// A target under To.
    convert_seed: "Master seed, hex",
    /// The standard of `convert_seed`, and its Standard fact.
    convert_seed_line: "BIP-32 seed",
    /// A target under To.
    convert_xprv: "xprv",
    /// `convert_xprv` on a test network.
    convert_tprv: "tprv",
    /// The standard of `convert_xprv`, and its Standard fact.
    convert_xprv_line: "BIP-32 master extended private key",
    /// A dimmed row under To.
    convert_no_words: "BIP-39 words",
    /// Its line.
    convert_no_words_line: "BIP-39 words cannot be made from a seed",
    /// A dimmed row under To.
    convert_no_shares: "SLIP-39 or codex32",
    /// Its line.
    convert_no_shares_line: "Another wallet: these bytes as SLIP-39 or codex32 open a different, empty wallet",
    /// A dimmed row under To.
    convert_no_list: "BIP-39 words in another language",
    /// Its line.
    convert_no_list_line: "Another wallet: the seed is made from the words' text",
    /// A dimmed row under To.
    convert_no_shamir39: "Shamir39",
    /// Its line.
    convert_no_shamir39_line: "Not offered",
    /// In place of a value that read back as another wallet.
    convert_not_same_wallet: "Not shown: it did not read back as the same wallet",
    /// In place of a value not given for any other reason.
    convert_not_given: "Not shown: this key cannot be written in this form",
    /// A fact's label on Result.
    convert_standard: "Standard",
    /// A fact's label on Result.
    convert_restores_in: "Restores in",
    /// Where a master seed in hex restores.
    convert_seed_restores: "any BIP-32 library; Faraday; no common wallet takes a raw seed",
    /// Where an xprv restores.
    convert_xprv_restores: "Sparrow (master private key), Bitcoin Core (in a descriptor with the wallet's path), Faraday; Electrum takes an xprv as the account, not the master, so its addresses differ",
    /// A fact's label on Result.
    convert_fingerprint: "Fingerprint",
    /// The fingerprint fact.
    convert_same_as_source: "{} · same as the source",
    /// A fact's label on Result.
    convert_paths: "Paths",
    /// A line of the Paths fact: a wallet's name and a path.
    convert_wallet_path: "Wallet '{}' · {}",
    /// The Paths fact where no loaded wallet holds the key.
    convert_no_wallet: "No loaded wallet holds this key",
    /// A fact's label on Result.
    convert_passphrase: "Passphrase",
    /// The passphrase fact.
    convert_holds_passphrase: "Holds the passphrase's wallet: restore it with no passphrase",
    /// A vault's key under From. `{}`: the vault's name.
    convert_from_vault: "In vault {}",
    /// Result's button.
    convert_check: "Check my copy",
    /// Note on the scanner of Check my copy for a CompactSeedQR, when a
    /// SeedQR is read.
    convert_scan_is_standard: "This is a SeedQR, not a CompactSeedQR",
    /// Note on the scanner of Check my copy for a SeedQR, when a
    /// CompactSeedQR is read.
    convert_scan_is_compact: "This is a CompactSeedQR, not a SeedQR",
    /// Result's button.
    convert_save: "Save as a file…",
    /// The label over the copy typed back. `{}`: how many characters.
    convert_your_copy: "Your copy, {} characters",
    /// Check my copy's button.
    convert_compare: "Compare",
    /// Check my copy's button.
    convert_clear: "Clear",
    /// Check my copy's finding.
    convert_copy_matches: "Your copy matches",
    /// Check my copy's finding.
    convert_copy_differs: "Your copy differs",
    /// What the secret sheet calls a master seed in hex.
    convert_secret_seed: "A master seed",
    /// What the secret sheet calls an xprv.
    convert_secret_xprv: "A master private key",
    /// What the secret sheet says of either.
    convert_secret_gives: "Whoever has it can spend from every wallet of this key",
    /// What the secret sheet says of a form that leaves out the key's
    /// passphrase.
    convert_secret_gives_apart: "Whoever has it can spend from the wallet of these words with no passphrase",
    /// Result's button.
    convert_done: "Done",
    /// A target under To: the key's words as they are.
    convert_words: "The words",
    /// The standard of `convert_words`, and its Standard fact.
    convert_words_line: "BIP-39",
    /// The Restores in fact of `convert_words`.
    convert_words_restores: "every BIP-39 wallet",
    /// A target under To.
    convert_seedqr: "SeedQR",
    /// The standard of `convert_seedqr`, and its Standard fact.
    convert_seedqr_line: "SeedQR (SeedSigner's standard)",
    /// The Restores in fact of `convert_seedqr`.
    convert_seedqr_restores: "SeedSigner, Krux, Faraday",
    /// A target under To.
    convert_compact: "CompactSeedQR",
    /// The standard of `convert_compact`, and its Standard fact.
    convert_compact_line: "CompactSeedQR (SeedSigner's)",
    /// The Restores in fact of `convert_compact`.
    convert_compact_restores: "SeedSigner, Krux, Faraday",
    /// A target under To.
    convert_entropy: "Hex entropy",
    /// The standard of `convert_entropy`, and its Standard fact.
    convert_entropy_line: "BIP-39 entropy, hex",
    /// The Restores in fact of `convert_entropy`.
    convert_entropy_restores: "Ian Coleman's BIP-39 page (its entropy field), Faraday's New key (hex); no wallet takes entropy as a seed",
    /// A target under To.
    convert_numbers: "Word numbers",
    /// The standard of `convert_numbers`, and its Standard fact.
    convert_numbers_line: "BIP-39, each word's index (1 to 2048, and 0 to 2047 shown beside)",
    /// The Restores in fact of `convert_numbers`.
    convert_numbers_restores: "any BIP-39 wallet, once turned back into words by the list",
    /// A target under To.
    convert_starts: "4-letter starts",
    /// The standard of `convert_starts`, and its Standard fact.
    convert_starts_line: "BIP-39 (the English list is unique in its first four letters)",
    /// The Restores in fact of `convert_starts`.
    convert_starts_restores: "any BIP-39 wallet that completes a word from its start (Coldcard), and any, written out in full",
    /// What the secret sheet calls the words.
    convert_secret_words: "A seed's words",
    /// What the secret sheet calls hex entropy.
    convert_secret_entropy: "A seed's entropy",
    /// What the secret sheet calls word numbers.
    convert_secret_numbers: "A seed's word numbers",
    /// What the secret sheet calls 4-letter starts.
    convert_secret_starts: "A seed's 4-letter starts",
    /// What the secret sheet calls a SeedQR or CompactSeedQR picture.
    convert_secret_code: "A seed's code",
    /// Result's masked code, in place of the code.
    convert_show_code: "Show the code",
    /// A fact's label on Result: the list a key's numbers or entropy are
    /// on, for a key not on the English list.
    convert_word_list: "Word list",
    /// The Passphrase fact of a form that leaves the passphrase out.
    convert_keeps_passphrase: "Not in this form: restore it with the passphrase",
    /// Title of Convert's Settings card, between To and Result.
    convert_settings_title: "Settings",
    /// The button on Settings that makes the result.
    convert_make: "Make",
    /// The Seed XOR row under To, and its Standard fact with the parts' count.
    convert_xor: "Seed XOR parts",
    /// The line of `convert_xor`.
    convert_xor_line: "Seed XOR (Coldcard's), 2, 3 or 4 parts",
    /// The Standard fact of Seed XOR parts. `{}`: how many parts.
    convert_xor_standard: "Seed XOR (Coldcard's), {} parts",
    /// The Restores in fact of Seed XOR parts.
    convert_xor_restores: "Coldcard, Faraday; each part is itself valid BIP-39 words",
    /// Name of Seed XOR parts on the secret sheet.
    convert_secret_xor: "A seed's Seed XOR parts",
    /// The codex32 row under To.
    convert_codex32: "codex32",
    /// The line of `convert_codex32`.
    convert_codex32_line: "BIP-93 codex32, the secret or a set of shares",
    /// The Standard fact of the codex32 secret.
    convert_codex32_secret: "BIP-93 codex32, the secret",
    /// The Standard fact of a codex32 set. `{}`: shares needed, shares made.
    convert_codex32_set: "BIP-93 codex32, {} of {} shares",
    /// The Restores in fact of codex32.
    convert_codex32_restores: "Faraday, and any program that reads BIP-93 codex32",
    /// Name of codex32 strings on the secret sheet.
    convert_secret_codex32: "A seed's codex32 strings",
    /// The SLIP-39 1 of 1 row under To.
    convert_slip39: "SLIP-39, 1 of 1",
    /// The line of `convert_slip39`.
    convert_slip39_line: "SLIP-39, one group of one share, 20 or 33 words",
    /// The Standard fact of SLIP-39 1 of 1. `{}`: how many words.
    convert_slip39_standard: "SLIP-39, 1 of 1, {} words",
    /// The Restores in fact of SLIP-39 1 of 1.
    convert_slip39_restores: "Trezor (Model T, Safe 3, Safe 5), Faraday",
    /// Name of a SLIP-39 backup on the secret sheet.
    convert_secret_slip39: "A seed's SLIP-39 words",
    /// The SLIP-39 k of n row under To.
    convert_slip39_set: "SLIP-39, k of n",
    /// The line of `convert_slip39_set`.
    convert_slip39_set_line: "SLIP-39, shares in one group or in several, 20 or 33 words each",
    /// The Standard fact of one SLIP-39 group. `{}`: shares needed, then shares made.
    convert_slip39_set_standard: "SLIP-39, {} of {} shares",
    /// The Standard fact of several SLIP-39 groups. `{}`: groups needed, how many groups, then each group's `convert_slip39_shape`.
    convert_slip39_groups_standard: "SLIP-39, {} of {} groups: {}",
    /// One group's shape in `convert_slip39_groups_standard`. `{}`: shares needed, then shares made.
    convert_slip39_shape: "{} of {}",
    /// Between the groups' shapes in `convert_slip39_groups_standard`, followed by a space.
    convert_slip39_shape_sep: ",",
    /// Settings: how many SLIP-39 groups, on a stepper.
    convert_slip39_groups: "Groups",
    /// Settings: how many groups are needed, on a stepper.
    convert_slip39_groups_needed: "Groups needed",
    /// Settings: shares needed in the only group, on a stepper.
    convert_slip39_needed: "Shares needed",
    /// Settings: shares made in the only group, on a stepper.
    convert_slip39_made: "Shares made",
    /// Settings: shares needed in a group, on a stepper. `{}`: the group's number.
    convert_slip39_group_needed: "Group {} needs",
    /// Settings: shares made in a group, on a stepper. `{}`: the group's number.
    convert_slip39_group_made: "Group {} shares",
    /// A share's number in a result of several groups, before a space and the share. `{}`: its group, then its place in the group.
    convert_group_share: "Group {}, share {}:",
    /// Check my copy found the copy is a share of a result of several groups. `{}`: its group, then its place in the group.
    convert_copy_matches_group_share: "Your copy matches group {}, share {}",
    /// The dimmed SLIP-39 row for a seed of 64 bytes.
    convert_no_slip39_64: "SLIP-39",
    /// The line of `convert_no_slip39_64`.
    convert_no_slip39_64_line: "SLIP-39 backups of 64 bytes are read by no wallet",
    /// Settings: how many Seed XOR parts, on a stepper.
    convert_parts: "Parts",
    /// Settings: a codex32 set's threshold, on a stepper; 0 is the secret alone.
    convert_codex32_k: "Shares needed (0: the secret alone)",
    /// Settings: how many codex32 shares are made, on a stepper.
    convert_codex32_n: "Shares made",
    /// A fact on Settings where the target draws randomness: its label.
    convert_fresh: "Fresh randomness",
    /// The value of `convert_fresh`.
    convert_fresh_value: "Faraday's",
    /// What follows a part's number, before a space and the part, in a result of several.
    convert_part_sep: ":",
    /// From's button for the set source.
    convert_set_source: "A SLIP-39 set",
    /// From's summary with a set held. `{}`: its identifier.
    convert_set_summary: "SLIP-39 set {}",
    /// Under From's set button in Practice, and the refusal there.
    convert_set_practice: "Not offered in Practice",
    /// Label of the set entry's field.
    convert_set_field: "Type one share",
    /// Button that holds the share typed.
    convert_set_hold: "Use this share",
    /// Button that forgets the set held.
    convert_set_again: "Start again",
    /// A share refused. `{}`: the reason.
    convert_set_not_added: "Not added: {}",
    /// Fact: the set held.
    convert_set_label: "Set",
    /// The set held. `{}`: identifier, groups needed, groups.
    convert_set_id_value: "{}, groups needed {} of {}",
    /// Fact label of a group held. `{}`: its number.
    convert_set_group: "Group {}",
    /// A group held. `{}`: shares needed, the numbers typed, whether enough.
    convert_set_group_value: "Needs {}. Typed: {}. {}",
    /// A group's quorum is met.
    convert_set_enough: "Enough",
    /// A group's quorum is not met.
    convert_set_not_enough: "Not enough yet",
    /// No share typed.
    convert_set_none: "none",
    /// Button to the next card with a set held.
    convert_set_continue: "Continue",
    /// To row: a share of a group.
    convert_set_add: "Add a share",
    /// Line of Add a share.
    convert_set_add_line: "A new share of one group, at a number you choose",
    /// To row: the lost share.
    convert_set_replace: "Replace a lost share",
    /// Line of Replace a lost share.
    convert_set_replace_line: "The share that was lost, again, at its number",
    /// To row: a new set.
    convert_set_rotate: "Rotate the set",
    /// Line of Rotate the set.
    convert_set_rotate_line: "A new set of the same secret; the old set is no longer to be used",
    /// Line of a row not yet offered. `{}`: the count.
    convert_set_needs_shares: "Shares still needed in one group: {}",
    /// Line of Rotate not yet offered. `{}`: the count.
    convert_set_needs_groups: "Groups still needed: {}",
    /// Stepper: the group a share is made for.
    convert_set_group_step: "Group",
    /// Stepper: the member number.
    convert_set_member: "Share number",
    /// Under Add a share's Settings.
    convert_set_add_note: "The same number gives the same share again",
    /// Stepper of Rotate's count. `{}`: the group.
    convert_set_lost: "Group {} lost",
    /// Rotate's count reaches the limit.
    convert_set_other_hands: "The set may be in other hands. Rotating is not enough: move the coins to a new key",
    /// A typed share to press once destroyed.
    convert_set_destroyed: "Destroyed: group {}, share {}",
    /// One press for the shares not typed.
    convert_set_destroyed_others: "Destroyed: all other shares",
    /// Standard fact of a share made.
    convert_set_share_standard: "SLIP-39, group {} share {} of set {}",
    /// Passphrase fact of a share or set made.
    convert_set_passphrase: "The set's passphrase still opens the same wallet",
    /// Fact label on a rotation's result.
    convert_set_old: "Old set",
    /// Fact value on a rotation's result.
    convert_set_old_value: "No longer to be used",
    /// A share or set that did not read back.
    convert_not_same_set: "Not shown: it did not read back as the same set",
    /// Check my copy found the copy is one part of a result of several. `{}`: its place.
    convert_copy_matches_part: "Your copy matches part {}",
    /// The way in from a key's chart sheet and from Explore.
    convert_way_in: "Write it in another form",
    /// Toast in `bip85_open`.
    bip85_load_a_key_first: "Load a key first",
    /// Text in `bip85_title`, and a key recorded as a child seed where its
    /// name is shown (`wallet::key_name`). `{}`: `app_name(b.app)`, the
    /// parent's `fp`, the index.
    bip85_bip_85_index: "BIP-85 {} · {} · index {}",
    /// Head of `bip85::screen::record_dialog`, before child words go into
    /// a vault or load as a key.
    bip85_record_head: "Record it as a child seed?",
    /// Text in `record_dialog`, for the vault. `{}`: the index, the parent's fingerprint.
    bip85_record_vault: "Recorded, the vault keeps that these words are BIP-85 index {} of key {}. Not recorded, it holds them as an ordinary seed.",
    /// Text in `record_dialog`, for Load as a key. `{}`: the index, the parent's fingerprint.
    bip85_record_load: "Recorded, Faraday keeps that these words are BIP-85 index {} of key {}, and so does any vault they are saved into. Not recorded, they are an ordinary seed.",
    /// Action in `record_dialog`: the words go in as an ordinary seed.
    bip85_record_no: "Don't record",
    /// Action in `record_dialog`: the words go in recorded as a child seed.
    bip85_record_yes: "Record",
    /// Toast in `bip85_to_vault`.
    bip85_no_vault_is_open: "No vault is open",
    /// Text in `bip85_to_vault`. `{}`: `title`.
    bip85_is_in_the_vault: "{} is in the vault",
    /// Toast in `bip85_load`. `{}`: `self.medium.noun()`.
    bip85_remove_the_first: "Remove the {} first",
    /// Toast in `bip85_load`. `{}`: `fp_text(fp)`.
    bip85_key_added: "Key {} added",
    /// Text in `bip85_out`.
    bip85_a_child_seed: "A child seed's words",
    /// Text in `bip85_out`.
    bip85_a_private_key: "A private key",
    /// Text in `bip85_out`.
    bip85_secret_bytes: "Secret bytes",
    /// Text in `bip85_out`.
    bip85_a_password: "A password",
    /// The title and the file name a password out of BIP-85 goes under,
    /// in a KeePass export (`bip85::out_title`) and as a file
    /// (`bip85_out`): nothing of the key it came from or its index (F172).
    bip85_password_out: "Password",
    /// The file name a KeePass export goes under, whatever the entry's
    /// title: the stick's listing shows no entry's title (`bip85::kdbx_out`).
    kdbx_export_file: "keepass-export",
    /// Text in `bip85_out`.
    bip85_secret_warning: "Whoever has it has what it protects or spends",
    // ----- bip85 (faraday-core/src/bip85/screen.rs) -----
    /// Text in `TITLES`.
    bip85_key: "Key",
    /// Text in `TITLES`.
    bip85_application: "Application",
    /// Text in `TITLES`.
    bip85_index: "Index",
    /// Text in `TITLES`.
    bip85_value: "Value",
    /// Text in `draw`.
    bip85_wallets: "Wallets",
    /// Text in `draw`.
    bip85_derive_with_bip: "Derive with BIP-85",
    /// Text in `length_label`. `{}`: `b.length`.
    bip85_words: "{} words",
    /// Text in `length_label`. `{}`: `b.length`.
    bip85_bytes: "{} bytes",
    /// Text in `length_label`. `{}`: `b.length`.
    bip85_characters: "{} characters",
    /// Text in `summary`.
    bip85_not_chosen: "Not chosen",
    /// Text in `card`. `{}`: `lo`, `hi`.
    bip85_index_range: "{} to {}",
    /// Line in `card`.
    bip85_index_limits: "An index is 0 to 2147483647",
    /// Line in `card`.
    bip85_type_the_digits: "Type the digits · Enter continues",
    /// Button in `card`.
    bip85_continue: "Continue",
    /// Line in `card`.
    bip85_path: "Path",
    /// Text in `card`.
    bip85_hide: "Hide",
    /// Text in `card`.
    bip85_show: "Show",
    /// Text in `card`. `{}`: `v.name`.
    bip85_save_into: "Save into {}",
    /// Text in `card`.
    bip85_load_as_a_key: "Load as a key",
    /// Text in `card`.
    bip85_out_unprotected: "Out unprotected…",
        }
    };
}
