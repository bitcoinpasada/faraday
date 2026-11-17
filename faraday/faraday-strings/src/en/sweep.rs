//! Faraday's English words for the sweep area.

macro_rules! words_sweep {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- Sweep a private key (docs/INTEROP.md §5, Sweeping a private key, F171), faraday-core -----
    /// Text in `TILES`, and the Sweep sheet's title.
    catalog_sweep_key: "Sweep a private key",
    /// Text in `TILES`.
    catalog_sweep_key_line: "A paper wallet's or a WIF key's coins, to your wallet",
    /// The Sweep sheet's field label.
    sweep_field: "Private key (WIF)",
    /// The Sweep sheet's button that reads the key typed.
    sweep_use: "Use this key",
    /// The Sweep sheet's button that opens the camera, for the key or the PSBT.
    sweep_scan: "Scan",
    /// The Sweep sheet's way out, which wipes the key.
    sweep_done: "Done",
    /// Text that is no WIF private key, typed or scanned for the sweep.
    wif_not_a_key: "This is not a private key",
    /// A mainnet WIF on a test network. `{}`: the network's name.
    sweep_mainnet_key: "This is a mainnet key, and this session is on {}",
    /// A test network's WIF on Mainnet.
    sweep_test_key: "This key is for the test networks, and this session is on Mainnet",
    /// Practice's refusal: the tile's need and the scanner's note.
    sweep_practice: "Practice sweeps no private key",
    /// A private key scanned on a scanner that does not sweep.
    sweep_elsewhere: "That is a private key. Sweep a private key, in Tools, takes it",
    /// The key, named by its legacy address. `{}`: the address.
    sweep_key_of: "Key of {}",
    /// What happens to the key.
    sweep_not_kept: "Not kept: wiped at Done, a lock, or leaving this sheet",
    /// An uncompressed key's line.
    wif_uncompressed: "Uncompressed key: legacy addresses only",
    /// An address's script type on the Sweep sheet.
    sweep_legacy: "Legacy",
    /// An address's script type on the Sweep sheet.
    sweep_native: "Native SegWit",
    /// An address's script type on the Sweep sheet.
    sweep_nested: "Nested SegWit",
    /// The button that scans the sweep's PSBT.
    sweep_scan_psbt: "Scan the PSBT",
    /// The scanner's note for a code that is no PSBT, once the key is read.
    sweep_not_psbt: "Not a PSBT",
    /// The review's inputs line. `{}`: how many, then the total.
    sweep_from: "From this key: {} inputs, {}",
    /// The review's other inputs. `{}`: how many.
    sweep_others: "Other inputs, not signed: {}",
    /// The review's outputs heading.
    sweep_to: "To",
    /// An output to a loaded wallet. `{}`: the wallet, receive or change, the index.
    sweep_to_wallet: "{}, {} address {}",
    /// An output to no loaded wallet's address (its first hundred on each chain).
    sweep_to_other: "Not a loaded wallet's address",
    /// The review's fee. `{}`: the amount.
    sweep_fee: "Fee: {}",
    /// A PSBT none of whose inputs spends the key.
    sweep_no_inputs: "No input of this PSBT spends this key",
    /// The hold that signs.
    sweep_hold: "Hold to sign",
    /// After signing, finished. `{}`: how many inputs.
    sweep_signed_final: "Signed {} inputs. The transaction is in the Outbox",
    /// After signing, not finished. `{}`: how many inputs.
    sweep_signed_part: "Signed {} inputs. The PSBT is in the Outbox",
    /// The button that shows what was signed as a QR code.
    sweep_show_qr: "Show as QR",
    /// The QR view's title for the swept transaction.
    sweep_qr_transaction: "Swept transaction",
    /// The QR view's title for the swept PSBT.
    sweep_qr_psbt: "Swept PSBT",
    /// A signing refusal: a legacy input whose script code holds OP_CODESEPARATOR.
    sign_codeseparator: "A legacy input with OP_CODESEPARATOR is not signed",
    /// The Tools tile's name for comparing the PSBT sent with the one returned.
    catalog_check_a_signer: "Check a signer",
    /// The Check a signer tile's line.
    catalog_sent_and_returned: "Sent and returned PSBTs compared",
    /// Check a signer's first card: the PSBT that went to the device.
    signer_sent: "Sent",
    /// Check a signer's second card: the PSBT that came back.
    signer_returned: "Returned",
    /// Check a signer's last card: the verdict.
    signer_result: "Result",
    /// The name of a PSBT read by the scanner, on a closed card.
    signer_scanned: "Scanned",
    /// The scanner's title on the Sent card.
    signer_scan_sent: "Scan the PSBT sent",
    /// The scanner's title on the Returned card.
    signer_scan_returned: "Scan the PSBT returned",
    /// The button that opens the scanner on a card.
    signer_scan: "Scan",
    /// The Returned card's label over the PSBTs of other transactions.
    signer_other_transactions: "Other transactions",
    /// A PSBT row's line. `{}`: its signatures, its transaction's short id.
    signer_row: "{} signatures · {}",
    /// A PSBT row's line with one signature. `{}`: its transaction's short id.
    signer_row_one: "1 signature · {}",
    /// The verdict where only signatures were added.
    signer_only_signatures: "Only signatures were added",
    /// The verdict where signatures were added and inputs finalized. `{}`: how many.
    signer_only_finalized: "Only signatures were added; {} inputs finalized from them",
    /// The verdict where signatures were added and one input finalized.
    signer_only_one_finalized: "Only signatures were added; 1 input finalized from them",
    /// The verdict where a MuSig2 nonce was added with the signatures.
    signer_only_nonces: "Only nonces and signatures were added",
    /// The verdict with findings. `{}`: how many.
    signer_findings: "{} findings",
    /// The verdict with one finding.
    signer_one_finding: "1 finding",
    /// The verdict where the two PSBTs share no input.
    signer_not_the_same: "Not the same transaction",
    /// A place in a Check a signer line: the transaction itself.
    signer_at_transaction: "Transaction",
    /// A place in a Check a signer line. `{}`: the input, from 0.
    signer_at_input: "Input {}",
    /// A place in a Check a signer line. `{}`: the output, from 0.
    signer_at_output: "Output {}",
    /// A finding's level word, in its tone.
    signer_level_blocked: "Blocked",
    /// A finding's level word, in its tone.
    signer_level_danger: "Danger",
    /// A finding's level word, in its tone.
    signer_level_caution: "Caution",
    /// A line's level word where the level is only a note.
    signer_level_info: "Info",
    /// A finding: the version changed. `{}`: where, the version sent, the one returned.
    signer_t_version: "{}: version {} sent, {} returned",
    /// A finding: the locktime changed. `{}`: where, the locktime sent, the one returned.
    signer_t_locktime: "{}: locktime {} sent, {} returned",
    /// A finding: the input count changed. `{}`: where, the count sent, the one returned.
    signer_t_inputs: "{}: {} inputs sent, {} returned",
    /// A finding: the output count changed. `{}`: where, the count sent, the one returned.
    signer_t_outputs: "{}: {} outputs sent, {} returned",
    /// A finding: an input spends another outpoint. `{}`: where, the one returned, the one sent.
    signer_t_spends: "{}: spends {}, not {}",
    /// A finding: an input's sequence changed. `{}`: where, the sequence sent, the one returned.
    signer_t_sequence: "{}: sequence {} sent, {} returned",
    /// A finding: an output's amount changed. `{}`: where, the amount returned, the one sent.
    signer_t_amount: "{}: {} returned, {} sent",
    /// A finding: an output pays another script. `{}`: where, the one returned, the one sent.
    signer_t_pays: "{}: pays {}, not {}",
    /// A script that has no address. `{}`: its length in bytes.
    signer_script_bytes: "script {} bytes",
    /// A finding: an added signature does not verify. `{}`: where, the key.
    signer_s_invalid: "{}: the signature under {} is not a signature of this transaction",
    /// A finding: an added signature was not checked. `{}`: where, the key, why.
    signer_s_unchecked: "{}: the signature under {} was not checked: {}",
    /// Why a MuSig2 signature was not checked: its wallet is not registered.
    signer_unchecked_musig_wallet: "the MuSig2 wallet is not registered",
    /// Why a MuSig2 signature was not checked: a participant's nonce is missing.
    signer_unchecked_musig_nonce: "a participant's nonce is missing",
    /// A finding: a signature under a key the sent PSBT does not name. `{}`: where, the key.
    signer_s_unnamed: "{}: signature under {}, a key the sent PSBT does not name",
    /// A finding: a signature over a leaf the sent PSBT does not hold. `{}`: where, the key.
    signer_s_leaf: "{}: the signature under {} is over a leaf the sent PSBT does not hold",
    /// A finding: another sighash than asked. `{}`: where, the key, the sighash signed with, the one asked for.
    signer_s_sighash: "{}: the signature under {} signs with {}, not the {} asked for",
    /// A finding: another sighash than asked that commits to no output. `{}`: where, the key, signed with, asked for.
    signer_s_sighash_none: "{}: the signature under {} signs with {}, not the {} asked for: outputs are not committed to",
    /// A finding: a sighash that commits to more than asked. `{}`: where, the key, signed with, asked for.
    signer_s_sighash_more: "{}: the signature under {} signs with {}, which commits to more than the {} asked for",
    /// A finding: one nonce in more than one place. `{}`: the keys, the inputs.
    signer_s_nonce_reuse: "{}: one nonce on inputs {}",
    /// A finding: a nonce a loaded key would not make. `{}`: where, the key.
    signer_s_not_reproduced: "{}: the signature under {} could not be reproduced. Either the device signed with fresh randomness, or it chose the nonce",
    /// A finding: a signature that could not be recomputed. `{}`: where, the key.
    signer_s_not_recomputed: "{}: the signature under {} could not be recomputed",
    /// A finding: a signature that is gone. `{}`: where, the key.
    signer_s_removed: "{}: the signature under {} was removed",
    /// A finding: a signature with other bytes. `{}`: where, the key.
    signer_s_changed: "{}: the signature under {} changed",
    /// A finding: nothing was added.
    signer_s_none_added: "No signature was added",
    /// A finding: the final fields differ from what the signatures give. `{}`: where.
    signer_f_differs: "{}: finalized; the final fields differ from what its signatures give",
    /// A finding: the final fields cannot be compared. `{}`: where.
    signer_f_not_compared: "{}: finalized; Faraday cannot finalize this input to compare",
    /// A finding: the final fields changed. `{}`: where.
    signer_f_changed: "{}: the final fields changed",
    /// A finding: the final fields were removed. `{}`: where.
    signer_f_removed: "{}: the final fields were removed",
    /// A finding: a field was added. `{}`: where, the field.
    signer_m_added: "{}: {} added",
    /// A finding: a field changed. `{}`: where, the field.
    signer_m_changed: "{}: {} changed",
    /// A finding: a field was removed. `{}`: where, the field.
    signer_m_removed: "{}: {} removed",
    /// A finding: a difference the check names no case for. `{}`: where, the field.
    signer_unnamed: "{}: {} differs in a way the check does not name",
    /// A finding: an unknown pair added. `{}`: where, its type, its key's bytes, its value's bytes.
    signer_p_unknown_added: "{}: unknown field added · type {} · key {} bytes · value {} bytes",
    /// A finding: an unknown pair changed. `{}`: where, its type, the value's bytes before, after.
    signer_p_unknown_changed: "{}: unknown field changed · type {} · value {} → {} bytes",
    /// A finding: a proprietary pair added. `{}`: where, the prefix's bytes, the subtype, the value's bytes.
    signer_p_proprietary_added: "{}: proprietary field added · prefix {} bytes · subtype {} · value {} bytes",
    /// A finding: a proprietary pair changed. `{}`: where, the prefix's bytes, the subtype, the value's bytes before, after.
    signer_p_proprietary_changed: "{}: proprietary field changed · prefix {} bytes · subtype {} · value {} → {} bytes",
    /// A finding: an unknown pair removed. `{}`: where, its type.
    signer_p_unknown_removed: "{}: unknown field removed · type {}",
    /// A finding: a proprietary pair removed. `{}`: where, the prefix's bytes, the subtype.
    signer_p_proprietary_removed: "{}: proprietary field removed · prefix {} bytes · subtype {}",
    /// A finding: bytes in the returned file no field accounts for. `{}`: how many.
    signer_e_unaccounted: "Returned file: {} bytes its fields do not account for",
    /// A stated line: a MuSig2 public nonce was added. `{}`: where, the key.
    signer_i_nonce_added: "{}: public nonce added for {}",
    /// A stated line: an input was finalized. `{}`: where.
    signer_i_finalized: "{}: finalized",
    /// A field's name in a Check a signer line.
    signer_field_version: "version",
    /// A field's name in a Check a signer line.
    signer_field_locktime: "locktime",
    /// A field's name in a Check a signer line.
    signer_field_input_count: "input count",
    /// A field's name in a Check a signer line.
    signer_field_output_count: "output count",
    /// A field's name in a Check a signer line.
    signer_field_spends: "spent outpoint",
    /// A field's name in a Check a signer line.
    signer_field_sequence: "sequence",
    /// A field's name in a Check a signer line.
    signer_field_amount: "amount",
    /// A field's name in a Check a signer line.
    signer_field_script: "script",
    /// A field's name in a Check a signer line.
    signer_field_previous_transaction: "previous transaction",
    /// A field's name in a Check a signer line.
    signer_field_spent_output: "spent output",
    /// A field's name in a Check a signer line.
    signer_field_redeem_script: "redeem script",
    /// A field's name in a Check a signer line.
    signer_field_witness_script: "witness script",
    /// A field's name in a Check a signer line.
    signer_field_sighash: "sighash type",
    /// A field's name in a Check a signer line.
    signer_field_key_origins: "key origins",
    /// A field's name in a Check a signer line.
    signer_field_taproot_key_origins: "taproot key origins",
    /// A field's name in a Check a signer line.
    signer_field_internal_key: "taproot internal key",
    /// A field's name in a Check a signer line.
    signer_field_merkle_root: "taproot merkle root",
    /// A field's name in a Check a signer line.
    signer_field_leaf_scripts: "taproot leaf scripts",
    /// A field's name in a Check a signer line.
    signer_field_unknown: "unknown fields",
    /// A field's name in a Check a signer line.
    signer_field_proprietary: "proprietary fields",
    /// A field's name in a Check a signer line.
    signer_field_xpubs: "extended public keys",
    /// A field's name in a Check a signer line.
    signer_field_preimages: "preimages",
    /// A field's name in a Check a signer line.
    signer_field_tap_tree: "taproot tree",
    /// A field's name in a Check a signer line.
    signer_field_psbt_version: "PSBT version",
    /// A field's name in a Check a signer line.
    signer_field_v2_fields: "version 2 fields",
    /// A field's name in a Check a signer line.
    signer_field_modifiable: "modifiable flags",
    /// A field's name in a Check a signer line.
    signer_field_musig_participants: "MuSig2 participants",
    /// A field's name in a Check a signer line.
    signer_field_musig_nonce: "MuSig2 public nonce",
    /// A field's name in a Check a signer line.
    signer_field_musig_partial_sig: "MuSig2 partial signature",
    /// The Result's fact: the transaction's id where both PSBTs have it.
    signer_transaction: "Transaction",
    /// The Result's fact: how many signatures the device added.
    signer_signatures_added: "Signatures added",
    /// The Result's fact: how many inputs the device finalized.
    signer_inputs_finalized: "Inputs finalized",
    /// A closed card's line for a file replaced in Files since it was
    /// chosen. `{}`: the file's name.
    signer_file_differs: "{} · changed or no longer in Files",
    /// A finding's line: its level's word, then the finding. `{}`: the
    /// level's word; the finding.
    signer_finding_line: "{} · {}",
    /// The Result's change fact, where an output is verified change.
    signer_change_verified: "verified",
    /// The Result's change fact, where no output is.
    signer_change_not_verified: "not verified",
    /// The Result's fact: how many added signatures' nonces a loaded key reproduced.
    signer_nonces_recomputed: "Nonces recomputed",
    /// The value of that fact. `{}`: how many were recomputed, how many were added under an ordinary key.
    signer_nonces_of: "{} of {}",
    /// The value of that fact, one line per key whose seed is not loaded. `{}`: the key's fingerprint.
    signer_nonces_none: "None: {} not loaded",
    /// A line of that fact beside a count: a key whose seed is not loaded. `{}`: the key's fingerprint.
    signer_nonces_not_loaded: "{} not loaded",
    /// The Result's button that opens Add a key for a key that signed and is not loaded. `{}`: the key's fingerprint.
    signer_load: "Load {}",
    /// The Result's line under a key that signed and whose seed is not loaded.
    signer_seed_not_loaded: "Seed not loaded",
    /// The Result's button on that line, which opens the sheet of ways to load the seed.
    signer_load_the_seed: "Load the seed",
    /// That sheet's title. `{}`: the key's fingerprint.
    signer_seed_of: "Seed of {}",
    /// That sheet's way in through a vault.
    signer_open_a_vault: "Open a vault",
    /// The words or the passphrase give another key than the one asked for. `{}`: the fingerprint they give, then the one asked for.
    signer_wrong_seed: "Fingerprint {}: not {}",
    /// The Result's label over the findings.
    signer_findings_label: "Findings",
    /// The Result's line where the check found nothing.
    signer_no_findings: "No findings",
    /// The Result's label over what is stated and is no finding.
    signer_stated_label: "Stated",
    /// Check a signer's Result, where a side is a finished transaction.
    signer_needs_two_psbts: "The signer check needs two PSBTs",
    /// The Result's button that swaps the two PSBTs and checks again.
    signer_swap: "Swap",
    /// The Result's button that opens the Returned card.
    signer_choose_again: "Choose again",
    /// The working line while a Krux code's key is worked out.
    kef_making: "Making the code",
    /// The title of the Krux code's picture and of its sheet.
    kef_code_title: "Krux encrypted code",
    /// The sheet's line under the title.
    kef_code_subtitle: "One code, base43 text",
    /// A line under the code's picture: the version and the count. `{}`: the version; the count.
    kef_code_label_version: "Version {} · {} rounds",
    /// A line under the code's picture: the id, where it is text the person chose. `{}`: the id.
    kef_code_label_id: "Id {}",
    /// The file name of the code's picture. `{}`: the version; the id in hex.
    kef_code_png_name: "krux-v{}-{}.png",
    /// The file name of the code's bytes. `{}`: the version; the id in hex.
    kef_code_file_name: "krux-v{}-{}.kef",
    /// Said when the contents do not fit one code, and only the file is made.
    kef_code_too_big: "Too large for one code. The file is saved.",
    /// A refusal: two blocks of the contents are equal and the mode shows it. `{}`: a version that works.
    kef_refuse_duplicate: "Two blocks are equal and this version would show it. Version {} works.",
    /// A refusal: the end of the contents cannot be told from padding. `{}`: a version that works.
    kef_refuse_nul_end: "The contents end in a zero byte, which this version's padding cannot keep. Version {} works.",
    /// A refusal: the passphrase has a character the code's keypads do not.
    kef_refuse_passphrase: "The passphrase is 1 to 200 printable ASCII characters.",
    /// A refusal: nothing to seal.
    kef_refuse_empty: "There is nothing to put in the code.",
    /// A refusal: the count is outside what is offered.
    kef_refuse_count: "The count is from 10,000 to 500,000.",
    /// A refusal: the typed id is empty or longer than the code can hold.
    kef_refuse_id: "The id is 1 to 252 bytes.",
    /// A refusal: the version is not one of KEF's.
    kef_refuse_version: "That version is not one the format has.",
    /// Said under Krux code… when the seed cannot be put in a Krux code: only 12 and 24 words of the English list can.
    kef_not_offered: "12 or 24 English words",
    /// The line under the ECB versions in Options.
    kef_ecb_line: "ECB writes equal blocks alike, so a code with two equal blocks is refused.",
    /// The line on the passphrase step and Options about a typed id.
    kef_public_id_line: "A typed id or the fingerprint is printed on the code and is not secret.",
    /// The primary action of the Krux code form: roll dice for the passphrase.
    kef_form_dice: "Generate with dice",
    /// The button that opens the code's options.
    kef_form_options: "Options",
    /// The options button's line. `{}`: version, rounds, the id.
    kef_form_summary: "Version {} · {} rounds · {}",
    /// The options line with the code's side. `{}`: the line, the side, the side.
    kef_form_summary_side: "{} · {} × {}",
    /// Makes the code.
    kef_form_make: "Make the code",
    /// The code cannot start until the session has fresh randomness.
    kef_form_need_random: "Add randomness, then make the code.",
    /// In the options line: an 8-byte random id.
    kef_id_random8_fact: "random id",
    /// In the options line: a 16-byte random id.
    kef_id_random16_fact: "16-byte random id",
    /// In the options line: a typed id.
    kef_id_label_fact: "label id",
    /// In the options line: the fingerprint as id.
    kef_id_fingerprint_fact: "fingerprint id",
    /// Options: the versions.
    kef_opt_version: "Version",
    /// Options: the GCM versions' name.
    kef_mode_gcm: "AES-GCM",
    /// Options: the ECB versions' name.
    kef_mode_ecb: "AES-ECB",
    /// Options: low error correction, for version 5.
    kef_opt_low: "Low error correction",
    /// Options: the number of rounds.
    kef_opt_rounds: "Rounds",
    /// Options: the id.
    kef_opt_id: "Id",
    /// Options: an 8-byte random id.
    kef_opt_id_random8: "Random 8 bytes",
    /// Options: a 16-byte random id.
    kef_opt_id_random16: "Random 16 bytes",
    /// Options: a typed id.
    kef_opt_id_label: "A label",
    /// Options: the fingerprint as id.
    kef_opt_id_fingerprint: "The fingerprint",
    /// Options: the typed id's field.
    kef_opt_label: "Label",
    /// Options, on Paper: the code's side.
    kef_opt_size: "Size",
    /// Options, on Paper: whether the code can be copied by hand.
    kef_opt_by_hand: "By hand",
    /// Options, on Paper: a code that fits a blank template.
    kef_opt_by_hand_yes: "Sheet",
    /// Options, on Paper: a code too large for a blank template.
    kef_opt_by_hand_no: "No",
    /// Options, on Paper: a side in modules. `{}`: the side, twice.
    kef_opt_side: "{} × {}",
    /// The Code card, on Paper: the sheet a code that fits one is copied on. `{}`: the side, twice.
    kef_by_hand_sheet: "Sheet: Faraday Starter, Blank templates, Krux code, {} × {}",
    /// The Code card, on Paper: the line for the largest side that fits a blank template.
    kef_largest: "Largest",
    /// The Code card, on Paper: whether the drawn copy was scanned.
    kef_drawn_copy: "Drawn copy",
    /// The Code card, on Paper: the drawn copy was not scanned.
    kef_not_scanned: "Not scanned",
    /// The Code card, on Paper: the camera on the drawn copy.
    kef_scan_my_copy: "Scan my copy",
    /// The scanner for the drawn Krux code: what it read is not the code.
    kef_not_this_code: "Not this code",
    /// The Code card, on Paper: takes the code off Paper.
    kef_not_on_paper: "Not on Paper",
    /// Templates item: the Krux code sheets, after the others'. `{}`: how many, the side twice.
    kef_templates_clause: "{} from Blank templates, Krux code, {} × {}",
    /// Templates item: the Krux code sheets alone. `{}`: how many, the side twice.
    kef_templates_starter: "Print {} from Faraday Starter: Blank templates, Krux code, {} × {}",
    /// Templates item's line with only Krux code sheets. `{}`: the side twice.
    kef_templates_sub: "Krux code, {} × {}",
    /// The Code card: opens the check of the code against what it holds.
    kef_check_my_code: "Check my code",
    /// The Code card: goes back to the first step.
    kef_make_another: "Make another",
    /// The tool's Code card: shows the grid to copy the code from.
    kef_copy_by_hand: "Copy by hand",
    /// The tool's Code card: shows the code's picture again.
    kef_show_code: "Show the code",
    /// Check my code: the button that opens the code and compares.
    kef_compare: "Compare",
    /// Check my code: the field's label.
    kef_check_pass: "Passphrase of the code",
    /// Check my code: the working line while the code is opened.
    kef_comparing: "Comparing the code",
    /// Check my code: the code opened and holds what it was made from.
    kef_checked: "Checked",
    /// Check my code: the code did not open or holds something else.
    kef_check_no_match: "Not matching. Try the passphrase again.",
    /// Text in `label`: a seed's Krux code as files on a stick. `{}`: `seed(i)`.
    plan_kef_code: "Seed {}: Krux code",
    /// Text in `label`: a seed's Krux code copied by hand onto paper. `{}`: `seed(i)`.
    plan_kef_code_paper: "Seed {}: Krux code, by hand",
    /// Text in `label`: a Seed XOR part, written on paper. `{}`: `seed(i)`, the part's number.
    plan_xor_part: "Seed {}: Seed XOR part {}",
    /// Text in `label`: a SLIP-39 share, written on paper. `{}`: `seed(i)`, the group's number, the share's number in the group.
    plan_slip39_share: "Seed {}: SLIP-39 group {} share {}",
    /// A form on a chip: a seed's Krux encrypted code, on a stick or on paper.
    board_kef_code: "Krux code",
    /// The chart's name for a seed's Krux code. `{}`: the key's number.
    glance_kef_code: "Key {} Krux code",
    /// A row in a seed chip's menu: the boxes a Krux code can go to.
    board_menu_kef: "Krux code…",
    /// The title of the menu of boxes a seed's Krux code can go to.
    board_menu_kef_to: "Krux code to…",
    /// Tools: the tile that makes a Krux encrypted code.
    catalog_krux_code: "Krux code",
    /// Tools: the line under the Krux code tile.
    catalog_krux_code_line: "A passphrase, note or wallet as an encrypted code",
    /// The Krux code tool's heading.
    kef_tool_title: "Krux code",
    /// The Krux code tool: the Back button's name when opened from Tools.
    kef_tool_tools: "Tools",
    /// The Krux code tool's first card: what the code holds.
    kef_tool_contents: "Contents",
    /// The Krux code tool's second card: the form.
    kef_tool_make: "Passphrase",
    /// The Krux code tool's third card: the code made.
    kef_tool_code: "Code",
    /// Contents: the choice of typing the text.
    kef_tool_type: "Type it",
    /// Contents: the choice of a loaded wallet's descriptor.
    kef_tool_wallet: "A loaded wallet",
    /// Contents: the typed text's field label.
    kef_tool_text: "Text",
    /// Contents: goes on to the passphrase.
    kef_tool_continue: "Continue",
    /// Contents: no wallet is loaded.
    kef_tool_no_wallet: "No wallet is loaded.",
    /// Contents, closed: typed text. `{}`: its length in bytes.
    kef_tool_typed_sum: "Typed text, {} bytes",
    /// Contents, closed: a wallet's descriptor. `{}`: the wallet's name.
    kef_tool_wallet_sum: "Descriptor of {}",
    /// The Code card, closed: the code is made.
    kef_tool_code_sum: "Made",
    /// The Code card, closed: the code is made and checked.
    kef_tool_code_checked_sum: "Checked",
    /// Tools: the tile that makes and opens a QR vault.
    catalog_qr_vault: "QR vault",
    /// Tools: the line under the QR vault tile.
    catalog_qr_vault_line: "A note or a key sealed in one QR code",
    /// The QR vault tool's heading.
    qr_tool_title: "QR vault",
    /// Tools: the Check a timestamp tile.
    catalog_check_timestamp: "Check a timestamp",
    /// Tools: the line under the Check a timestamp tile.
    catalog_check_timestamp_line: "A proof against a block header",
    /// Text in `tile_need`.
    catalog_copy_a_proof: "Copy a proof in first",
    /// Check a timestamp: the sample proof's name.
    sample_ots_proof_file: "sample-timestamp.ots",
    /// Check a timestamp: the sample header's name.
    sample_ots_header_file: "sample-block-header",
    /// Check a multisig setup: the screen's title and the Files menu's entry.
    mscheck_title: "Check a multisig setup",
    /// The Tools tile's line of Check a multisig setup.
    mscheck_line: "A setup's quorum and which key is yours",
    /// Check a multisig setup: a file row's menu entry.
    files_check_setup: "Check this setup",
    /// Check a multisig setup: the first card.
    mscheck_setup: "Setup",
    /// Check a multisig setup: the second card.
    mscheck_result: "Result",
    /// Check a multisig setup: the scanner's row.
    mscheck_scan: "Scan",
    /// The scanner's title in Check a multisig setup.
    mscheck_scan_title: "Scan the wallet's setup",
    /// Check a multisig setup: the heading over the files of Files to choose from.
    mscheck_choose_setup: "Choose a setup",
    /// Check a multisig setup: the line when no setup waits.
    mscheck_nothing_waits: "No multisig setup to check",
    /// Check a multisig setup: the heading over the wallets loaded here.
    mscheck_loaded: "A wallet loaded here",
    /// Check a multisig setup: the name of a setup read from a code.
    mscheck_scanned: "Scanned",
    /// The source of a setup taken from Paste.
    mscheck_pasted: "Pasted",
    /// Faraday Demo's Paste on the Setup card.
    mscheck_paste: "Paste",
    /// Check a multisig setup: the line for a wallet of one key.
    mscheck_refused_one_key: "One key: not a multisig setup",
    /// Check a multisig setup: the line for a descriptor that holds a private key.
    mscheck_private: "A private key: not taken here",
    /// Check a multisig setup: the line for a descriptor whose checksum fails. `{}`: the checksum given, then the checksum the descriptor gives.
    mscheck_checksum_fails: "The descriptor checksum is wrong: it is given as {}, and the descriptor gives {}",
    /// Check a multisig setup: the checksum's label.
    mscheck_checksum: "Descriptor checksum",
    /// Check a multisig setup: a checksum given and held. `{}`: the checksum.
    mscheck_checksum_holds: "{} · holds",
    /// Check a multisig setup: a setup with no checksum. `{}`: the checksum worked out.
    mscheck_checksum_worked_out: "{} · worked out here, none given",
    /// Check a multisig setup: a cosigner's label. `{}`: its number.
    mscheck_key: "Key {}",
    /// Check a multisig setup: a cosigner a key loaded here is.
    mscheck_yours: "Yours",
    /// Check a multisig setup: a cosigner that is a single-key account of a key loaded here.
    mscheck_yours_single: "Yours · your single-key account",
    /// Check a multisig setup: a cosigner under a loaded key's fingerprint that is not that key. `{}`: the fingerprint, then the path.
    mscheck_wrong_key: "The fingerprint is {}'s; the key at {} is not",
    /// Check a multisig setup: the quorum's label.
    mscheck_quorum: "Quorum",
    /// Check a multisig setup: a quorum counted in seeds. `{}`: the fewest seeds, then every seed.
    mscheck_quorum_seeds: "{} of {} seeds",
    /// Check a multisig setup: the label over the ways a script is spent.
    mscheck_ways: "Ways to spend",
    /// Check a multisig setup: a script with more ways to spend than are listed.
    mscheck_ways_too_many: "More ways to spend than are listed here",
    /// Check a multisig setup: two keys of a way to spend. `{}`: the keys so far, then the next key.
    mscheck_keys_and: "{} and {}",
    /// Check a multisig setup: one way to spend, no wait.
    mscheck_way_any_time: "{} · any time",
    /// Check a multisig setup: one way to spend with a wait. `{}`: the keys, then the wait.
    mscheck_way_wait: "{} · {}",
    /// Check a multisig setup: a wait in blocks. `{}`: the blocks.
    mscheck_wait_blocks: "{} blocks after each coin arrives",
    /// Check a multisig setup: a wait in time units. `{}`: the seconds.
    mscheck_wait_seconds: "{} seconds after each coin arrives",
    /// Check a multisig setup: a wait for a block height. `{}`: the height.
    mscheck_wait_height: "block {}",
    /// Check a multisig setup: a wait for a time. `{}`: the Unix time.
    mscheck_wait_time: "Unix time {}",
    /// Check a multisig setup: a way that needs a hash's preimage.
    mscheck_wait_preimage: "a hash's preimage",
    /// Check a multisig setup: the first receive address's label.
    mscheck_first_receive: "First receive address",
    /// Check a multisig setup: the Result's button that adds the wallet.
    mscheck_add: "Add this wallet",
    /// Check a multisig setup: the Result's line once the wallet is loaded.
    mscheck_loaded_as: "Loaded here as {}",
    /// Check a multisig setup: the Result's text button.
    mscheck_another: "Another setup",
    /// Check a multisig setup: the Result's text button, which opens the
    /// seed's steps under the rows.
    mscheck_check_seed: "Check a seed against it",
    /// Check a multisig setup: the answer for a seed that is a cosigner's
    /// key, `{}` its number.
    mscheck_seed_is_key: "This seed is key {} of this wallet",
    /// Check a multisig setup: the answer for a seed that is several
    /// cosigners' key, `{}` their numbers.
    mscheck_seed_is_keys: "This seed is keys {} of this wallet",
    /// Check a multisig setup: the mark on the cosigner's row that the
    /// seed given is.
    mscheck_this_seed: "This seed",
    /// Check a multisig setup: leaves the screen.
    mscheck_done: "Done",
    /// Check a multisig setup: the Result before a setup is read.
    mscheck_result_waits: "Not checked yet",
    /// The scanner's tile that hands over a sample wallet setup.
    scan_sample_setup: "Load sample setup",
    /// The scanner's tile that hands over a sample block header.
    scan_sample_header: "Load sample block header",
    /// The scanner's tile that hands over a sample file hash.
    scan_sample_hash: "Load sample file hash",
    /// Check a timestamp's heading.
    ots_title: "Check a timestamp",
    /// Check a timestamp: the Back button's name when opened from Tools.
    ots_tools: "Tools",
    /// Check a timestamp: the scanner's title.
    ots_scan_title: "Scan a block header",
    /// Check a timestamp: the file hash scanner's title.
    ots_scan_hash_title: "Scan the file's SHA-256",
    /// Check a timestamp's first card.
    ots_card_proof: "Proof",
    /// Check a timestamp's second card.
    ots_card_file: "File",
    /// Check a timestamp's third card.
    ots_card_header: "Block header",
    /// Check a timestamp's fourth card.
    ots_card_result: "Result",
    /// Check a timestamp: a card with no file of Files that fits.
    ots_none_in_files: "No file in Files",
    /// Check a timestamp: the typed hash field's label.
    ots_hash_label: "SHA-256 of the file",
    /// Check a timestamp: the typed header field's label.
    ots_header_label: "Header in hex",
    /// Check a timestamp: takes the typed text.
    ots_use: "Use",
    /// Check a timestamp: opens the scanner.
    ots_scan: "Scan",
    /// Check a timestamp: a value that was typed, as a card's summary.
    ots_typed: "Typed",
    /// Check a timestamp: a value that was scanned, as a card's summary.
    ots_scanned: "Scanned",
    /// Check a timestamp: the Result card before the three are in.
    ots_result_waits: "Not checked yet",
    /// Check a timestamp: the verdict of a counted path.
    ots_committed: "The header commits to this file",
    /// Check a timestamp: a second or later counted path; {} is its height.
    ots_counted_also: "Bitcoin block {}: also counted",
    /// Check a timestamp: the verdict of a counted path when only a SHA-256 read from a text file was given.
    ots_committed_digest: "The header commits to this SHA-256",
    /// Check a timestamp: a card's summary for a SHA-256 read from a text file; {} is the file's name.
    ots_digest_from: "SHA-256 from {}",
    /// Check a timestamp: the verdict with no counted path.
    ots_not_committed: "No path through SHA-256 or RIPEMD-160 reaches this header",
    /// Check a timestamp: the file is not the one the proof states.
    ots_file_differs: "This is not the file the proof states",
    /// Check a timestamp: the verdict when the file's hash is not enough.
    ots_needs_file_short: "The whole file is needed",
    /// Check a timestamp: the proof starts from a hash a typed SHA-256 cannot give; {} is its name.
    ots_needs_file: "The proof starts from the file's {}: choose the file",
    /// Check a timestamp: a counted path's block hash.
    ots_block_hash: "Block hash",
    /// Check a timestamp: a counted path's block height.
    ots_height: "Height the proof states",
    /// Check a timestamp: a counted path's block time.
    ots_time: "Block time",
    /// Check a timestamp: a Bitcoin attestation the header does not match; {} is its height.
    ots_mismatch: "Bitcoin block {}: not this header",
    /// Check a timestamp: a path through a hash that is not counted; {} is the height, {} the hash.
    ots_not_counted: "Bitcoin block {}: passes through {}, not counted",
    /// Check a timestamp: a path through a hash that is not counted and reaches the header; {} is the height, {} the hash.
    ots_not_counted_reaches: "Bitcoin block {}: passes through {} and reaches the header, not counted",
    /// Check a timestamp: a pending attestation; {} is the calendar.
    ots_pending: "Pending at {}",
    /// Check a timestamp: a Litecoin attestation; {} is the height.
    ots_litecoin: "Litecoin block {}: not checked",
    /// Check a timestamp: an Ethereum attestation; {} is the height.
    ots_ethereum: "Ethereum block {}: not checked",
    /// Check a timestamp: the Result lists only some attestations; {} is how many more there are.
    ots_more: "and {} more",
    /// Check a timestamp: an attestation of a kind Faraday does not know; {} is its tag in hex.
    ots_unknown: "Unknown attestation {}",
    /// Check a timestamp: what was given is not a block header.
    ots_not_a_header: "Not an 80-byte block header or its hex",
    /// Check a timestamp: what was typed is not a SHA-256.
    ots_not_a_hash: "Not a SHA-256: 64 hex digits",
    /// The name of SHA-1 in a timestamp result.
    ots_op_sha1: "SHA-1",
    /// The name of RIPEMD-160 in a timestamp result.
    ots_op_ripemd160: "RIPEMD-160",
    /// The name of SHA-256 in a timestamp result.
    ots_op_sha256: "SHA-256",
    /// The name of Keccak-256 in a timestamp result.
    ots_op_keccak256: "Keccak-256",
    /// Check a timestamp: refusal.
    ots_err_proof_too_large: "The proof is over 64 KiB",
    /// Check a timestamp: refusal.
    ots_err_too_many_entries: "The proof has too many entries",
    /// Check a timestamp: refusal.
    ots_err_too_much_work: "The proof asks for too much work",
    /// Check a timestamp: refusal.
    ots_err_bad_magic: "Not an OpenTimestamps proof",
    /// Check a timestamp: refusal; {} is the version byte read.
    ots_err_version: "The proof's version is {}, not 1",
    /// Check a timestamp: refusal.
    ots_err_truncated: "The proof ends too soon",
    /// Check a timestamp: refusal.
    ots_err_trailing: "Bytes follow the proof",
    /// Check a timestamp: refusal.
    ots_err_file_hash_op: "The proof names a hash of the file Faraday does not know",
    /// Check a timestamp: refusal.
    ots_err_unknown_op: "The proof has an operation Faraday does not know",
    /// Check a timestamp: refusal.
    ots_err_empty_argument: "The proof has an operation with no bytes",
    /// Check a timestamp: refusal.
    ots_err_argument_long: "The proof has an operation with over 4,096 bytes",
    /// Check a timestamp: refusal.
    ots_err_result_long: "The proof makes a value over 4,096 bytes",
    /// Check a timestamp: refusal.
    ots_err_too_deep: "The proof is nested too deep",
    /// Check a timestamp: refusal.
    ots_err_number: "The proof has a number written in more than 9 bytes",
    /// Check a timestamp: refusal.
    ots_err_repeated_op: "The proof has the same operation twice at one step",
    /// Check a timestamp: refusal.
    ots_err_file_keccak: "The proof takes the file with Keccak-256, which Faraday does not accept for a file",
    /// Check a timestamp: refusal.
    ots_err_height: "The proof has a block height over 32 bits",
    /// Check a timestamp: refusal.
    ots_err_payload_long: "The proof has an attestation over 8,192 bytes",
    /// Check a timestamp: refusal.
    ots_err_payload_trailing: "The proof has an attestation with bytes after its fields",
    /// Check a timestamp: refusal.
    ots_err_uri_long: "The proof has a calendar address over 1,000 bytes",
    /// Check a timestamp: refusal.
    ots_err_uri_invalid: "The proof has a calendar address with a character that is not allowed",
    /// Check a timestamp: refusal.
    ots_err_header_length: "A block header is 80 bytes",
    /// Check a timestamp: refusal.
    ots_err_header_target: "The header's target is not valid",
    /// Check a timestamp: refusal.
    ots_err_header_easy: "The header's target is easier than the network allows",
    /// Check a timestamp: refusal.
    ots_err_header_work: "The header's hash is above its own target",
    /// Tools: the Read a Bitcoin Core wallet tile.
    catalog_read_core_wallet: "Read a Bitcoin Core wallet",
    /// Tools: the line under the Read a Bitcoin Core wallet tile.
    catalog_read_core_wallet_line: "A wallet.dat, read and checked",
    /// Text in `tile_need`.
    catalog_copy_a_wallet_dat: "Copy a wallet.dat in first",
    /// Read a Bitcoin Core wallet: the sample plain wallet's name.
    sample_core_wallet_plain_file: "sample-plain-wallet.dat",
    /// Read a Bitcoin Core wallet: the sample encrypted wallet's name.
    sample_core_wallet_encrypted_file: "sample-encrypted-wallet.dat",
    /// Read a Bitcoin Core wallet: the heading.
    core_wallet_title: "Read a Bitcoin Core wallet",
    /// Read a Bitcoin Core wallet: the Back button's name when opened from Tools.
    core_wallet_tools: "Tools",
    /// Read a Bitcoin Core wallet: the first card.
    core_wallet_card_file: "File",
    /// Read a Bitcoin Core wallet: the second card.
    core_wallet_card_result: "Result",
    /// Read a Bitcoin Core wallet: no file to choose.
    core_wallet_none_in_files: "No wallet file in Files",
    /// Read a Bitcoin Core wallet: the passphrase field's label.
    core_wallet_passphrase: "Passphrase, if the wallet is encrypted",
    /// Read a Bitcoin Core wallet: the button that reads the file.
    core_wallet_read: "Read",
    /// Read a Bitcoin Core wallet: the Result card before a read.
    core_wallet_result_waits: "Nothing read yet",
    /// Read a Bitcoin Core wallet: the result, every record and key passed.
    core_wallet_whole: "The wallet is whole",
    /// Read a Bitcoin Core wallet: the result, a record or key failed.
    core_wallet_not_whole: "The wallet is not whole",
    /// Read a Bitcoin Core wallet: the result, the passphrase failed.
    core_wallet_wrong_passphrase: "The passphrase does not open the wallet",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_network: "Network",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_version: "Written by Bitcoin Core",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_encryption: "Encryption",
    /// Read a Bitcoin Core wallet: the value for a wallet with no master key.
    core_wallet_not_encrypted: "None",
    /// Read a Bitcoin Core wallet: the value for an encrypted wallet. `{}`: the rounds.
    core_wallet_encrypted: "SHA-512 and AES-256, {} rounds",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_keys: "Private keys",
    /// Read a Bitcoin Core wallet: the value. `{}`: how many.
    core_wallet_keys_checked: "{} checked",
    /// Read a Bitcoin Core wallet: the value. `{}`: how many.
    core_wallet_keys_not_checked: "{} not checked, no passphrase given",
    /// Read a Bitcoin Core wallet: the value. `{}`: how many.
    core_wallet_keys_not_opened: "{} not checked, the passphrase is wrong",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_descriptor: "Descriptor",
    /// Read a Bitcoin Core wallet: a descriptor that is the active receive one.
    core_wallet_active_receive: "Active, receive",
    /// Read a Bitcoin Core wallet: a descriptor that is the active change one.
    core_wallet_active_change: "Active, change",
    /// Read a Bitcoin Core wallet: a descriptor's range. `{}`: first, last, next index.
    core_wallet_range: "Range {} to {}, next index {}",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_labels: "Labels",
    /// Read a Bitcoin Core wallet: a label.
    core_wallet_transactions: "Transactions",
    /// Read a Bitcoin Core wallet: a label for record types counted and not read.
    core_wallet_not_read: "Records not read",
    /// Read a Bitcoin Core wallet: a label for record types Faraday does not know.
    core_wallet_unknown: "Records not known",
    /// Read a Bitcoin Core wallet: records of unknown types past the list. `{}`: how many.
    core_wallet_unknown_more: "and {} more",
    /// Read a Bitcoin Core wallet: a record that failed. `{}`: the record, what failed.
    core_wallet_failed: "{}: {}",
    /// Read a Bitcoin Core wallet: failures past the list. `{}`: how many.
    core_wallet_failed_more: "{} more records failed",
    /// Read a Bitcoin Core wallet: the button that takes the descriptors to Wallets.
    core_wallet_take: "Take to Wallets",
    /// Read a Bitcoin Core wallet: the output type an active pair is of.
    core_wallet_type_legacy: "Legacy (pkh)",
    /// Read a Bitcoin Core wallet: the output type an active pair is of.
    core_wallet_type_p2sh_segwit: "Nested SegWit (sh-wpkh)",
    /// Read a Bitcoin Core wallet: the output type an active pair is of.
    core_wallet_type_segwit: "Native SegWit (wpkh)",
    /// Read a Bitcoin Core wallet: the output type an active pair is of.
    core_wallet_type_taproot: "Taproot (tr)",
    /// Read a Bitcoin Core wallet: the label of an active pair whose receive descriptor is a script, such as a multisig.
    core_wallet_type_script: "Script",
    /// Read a Bitcoin Core wallet: the result, a whole wallet with no active receive descriptor.
    core_wallet_no_active: "No active descriptors",
    /// Read a Bitcoin Core wallet: the label over the `combo()` descriptors Faraday lists and does not take. `{}`: how many.
    core_wallet_kept: "Kept by Bitcoin Core, not taken ({})",
    /// Read a Bitcoin Core wallet: `combo()` descriptors past the list. `{}`: how many.
    core_wallet_kept_more: "{} more kept by Bitcoin Core, not taken",
    /// Read a Bitcoin Core wallet: a failure, the record is not the shape Bitcoin Core writes.
    core_wallet_fault_layout: "not the shape Bitcoin Core writes",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_flags: "the flags hold a bit Bitcoin Core does not know",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_descriptor: "not an output descriptor of public keys",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_checksum: "the descriptor's checksum is wrong",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_id: "the descriptor's id is not the hash of its text",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_range: "the range or next index is not valid, or a descriptor with no wildcard stores a range of more than one index",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_key_network: "the keys are of another network than the file",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_not_held: "names a descriptor the file does not hold",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_repeated: "its key is in the file twice",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_key_malformed: "the private key's record is not the shape Bitcoin Core writes",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_key_hash: "the private key's stored hash is wrong",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_key_public: "the private key gives another public key than the record names",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_no_master: "encrypted keys and no master key",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_many_master: "a second master key",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_plain_key: "a plain private key in an encrypted wallet",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_master_method: "the master key's method is not one Faraday opens",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_master_rounds: "the master key's rounds are none or too many",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_cache_missing: "the account key Core cached for a key of the descriptor is missing",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_cache_key: "a cached account key is not the one its path and master key give",
    /// Read a Bitcoin Core wallet: a failure.
    core_wallet_fault_cache_unchecked: "a cached account key could not be compared with a master key",
    /// The QR vault tool: the Back button's name when opened from Tools.
    qr_tool_tools: "Tools",
    /// The QR vault tool's first card: what the code holds.
    qr_tool_contents: "Contents",
    /// The QR vault tool's second card: the passphrase and the cost.
    qr_tool_passphrase: "Passphrase",
    /// The QR vault tool's third card: the code made.
    qr_tool_code: "Code",
    /// Contents: the choice of typing a note.
    qr_tool_type_note: "Type a note",
    /// Contents: the choice of a loaded key.
    qr_tool_loaded_key: "A loaded key",
    /// Contents: the note's field label.
    qr_tool_note: "Note",
    /// Contents: the room left in the note. `{}`: the characters.
    qr_tool_room_left: "{} characters left",
    /// Contents: goes on to the passphrase.
    qr_tool_continue: "Continue",
    /// Contents: no key is loaded.
    qr_tool_no_key: "No key is loaded.",
    /// Contents: under a key that cannot be sealed.
    qr_tool_key_not_offered: "Not 12 or 24 English words",
    /// Contents, closed: a key. `{}`: its name.
    qr_tool_key_sum: "Key {}",
    /// Passphrase: the first field's label.
    qr_tool_pass: "Passphrase",
    /// Passphrase: the second field's label.
    qr_tool_pass_again: "Passphrase again",
    /// Passphrase: what the code's strength rests on.
    qr_tool_pass_warning: "The code is only as strong as its passphrase",
    /// Passphrase: the memory choice's label.
    qr_tool_memory: "Memory",
    /// Passphrase: one memory step. `{}`: the MiB.
    qr_tool_memory_mib: "{} MiB",
    /// Passphrase: the file's name field label.
    qr_tool_name: "Name",
    /// Passphrase: makes the code.
    qr_tool_make: "Make the code",
    /// Passphrase, closed. `{}`: the memory step in MiB.
    qr_tool_pass_sum: "{} MiB",
    /// The line shown while the code is sealed.
    qr_tool_sealing: "Sealing the code",
    /// Passphrase: the seal was refused. `{}`: the MiB it needs.
    qr_tool_no_memory: "This computer has no room for {} MiB. Choose a smaller step",
    /// Passphrase: the code was not made.
    qr_tool_not_made: "The code could not be made",
    /// Code: shows the code.
    qr_tool_show: "Show as QR",
    /// Code: the picture to the Outbox.
    qr_tool_png: "PNG to the Outbox",
    /// Code: the `.pgp` file to the Outbox.
    qr_tool_file: "To the Outbox",
    /// Code: starts again.
    qr_tool_another: "Another",
    /// Code and Open: back to where the tool was opened.
    qr_tool_done: "Done",
    /// A toast. `{}`: the file's name.
    qr_tool_saved: "{} is in the Outbox",
    /// The code's sheet title and the picture's first line.
    qr_tool_view_title: "QR vault",
    /// The code's sheet: under the title.
    qr_tool_view_subtitle: "OpenPGP message, one code",
    /// The Code card, closed.
    qr_tool_code_sum: "Made",
    /// The Open card: the code brought, `{}` its length in bytes.
    qr_tool_open_code: "QR vault code, {} bytes",
    /// The Open card's title and its button.
    qr_tool_open: "Open",
    /// The busy line while a code is opened.
    qr_tool_opening: "Opening the code",
    /// The Open card: the passphrase was wrong.
    qr_tool_wrong_pass: "The passphrase is wrong",
    /// The Open card: the code is an OpenPGP message that is not a QR vault.
    qr_tool_not_vault: "An OpenPGP message Faraday does not open",
    /// The Open card: the code is not an OpenPGP message.
    qr_tool_not_message: "Not an OpenPGP message",
    /// The Open card: the code states an Argon2 cost Faraday refuses.
    qr_tool_cost: "The code states a cost Faraday does not accept",
    /// The Open card: the code is damaged.
    qr_tool_damaged: "The code is damaged",
    /// The Open card: what the code holds is refused.
    qr_tool_not_opened: "The code could not be opened",
    /// The Open card: the code holds nothing.
    qr_tool_open_empty: "This QR vault is empty.",
    /// The Open card: the code holds a seed, `{}` its word count.
    qr_tool_open_seed: "A seed of {} words",
    /// The Open card: the code holds a codex32 share.
    qr_tool_open_share: "A codex32 share",
    /// The Open card: the code holds a passphrase.
    qr_tool_open_passphrase: "A passphrase",
    /// The Open card: the key a passphrase is of, `{}` its fingerprint.
    qr_tool_open_key: "Key {}",
    /// The Open card: the code holds a note.
    qr_tool_open_note: "A note",
    /// The Open card, a seed: opens the seed sheet again.
    qr_tool_load_seed: "Load as a key",
    /// The Open card, a codex32 share: opens Add a key's codex32 form with it.
    qr_tool_add_share: "Add the share",
    /// The Open card, a passphrase: adds it to the loaded key whose words make the key. `{}`: that key's fingerprint.
    qr_tool_add_to_key: "Add to key {}",
    /// The Open card, a passphrase whose key is not loaded. `{}`: the fingerprint of the key it makes.
    qr_tool_key_not_loaded: "Key {} is not loaded",
    /// The Open card, a passphrase whose key is not loaded: what to do first.
    qr_tool_load_words_first: "Load its words without a passphrase first.",
    /// The Open card, a note: saves it into the open vault.
    qr_tool_save_note: "Save as a note",
    /// The Open card, a note and no vault open: why Save as a note is dimmed.
    qr_tool_open_vault_first: "Open a vault first",
    /// The Open card, a note saved. `{}`: the vault's name.
    qr_tool_note_saved: "Note saved in {}",
    /// The Open card, a passphrase naming a loaded key that no loaded key's
    /// words make with it. `{}`: the key's fingerprint.
    qr_tool_pass_makes_no_loaded_key: "No loaded key makes key {} with this passphrase",
    /// The Open card, a note saved: Save as a note, dimmed.
    qr_tool_note_is_saved: "Saved as a note",
    /// The Open card, a passphrase added to a key. `{}`: the key's fingerprint.
    qr_tool_pass_added: "Key {} added",
    /// Contents card opened from a vault item, a seed. `{}`: the vault.
    qr_tool_item_seed: "A seed in {}",
    /// The same, a codex32 share.
    qr_tool_item_share: "A codex32 share in {}",
    /// The same, a BIP-39 passphrase.
    qr_tool_item_passphrase: "A BIP-39 passphrase in {}",
    /// The same, a note.
    qr_tool_item_note: "A note in {}",
    /// The same, a seed that keeps a passphrase.
    qr_tool_item_pass_kept: "Its passphrase is not in this code.",
    /// Make, the item's vault locked, closed or changed since the press.
    qr_tool_item_gone: "The vault was locked or changed. Nothing was sealed.",
        }
    };
}
