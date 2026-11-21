//! Faraday's English words for the psbt refusal area.

macro_rules! words_psbt_refusal {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- A PSBT refused or warned of (faraday-psbt's kinds, worded in `review.rs`) -----
    /// Why an input is not signed or finished: the previous transaction is not the one the input spends.
    psbt_reason_prev_tx_not_spent: "non_witness_utxo is not the transaction the input spends",
    /// Why an input is not signed or finished: the previous transaction has no output at the index spent.
    psbt_reason_prev_tx_no_output: "non_witness_utxo has no output at the spent index",
    /// Why an input is not signed or finished: the stated output and the previous transaction's disagree.
    psbt_reason_utxos_disagree: "witness_utxo and non_witness_utxo disagree",
    /// Why an input is not signed or finished: a stated output alone for an input that is not SegWit.
    psbt_reason_witness_utxo_legacy: "witness_utxo given for a non-witness input",
    /// Why an input is not signed or finished: a P2SH input with no redeem script.
    psbt_reason_no_redeem_script: "p2sh input without redeem_script",
    /// Why an input is not signed or finished: the redeem script is not the one paid to.
    psbt_reason_redeem_mismatch: "redeem_script does not match the scriptPubKey",
    /// Why an input is not signed or finished: a P2WSH input with no witness script.
    psbt_reason_no_witness_script: "p2wsh input without witness_script",
    /// Why an input is not signed or finished: the witness script is not the one committed to.
    psbt_reason_witness_mismatch: "witness_script does not match the witness program",
    /// Why an input is not signed or finished: a SegWit v0 input with no previous transaction.
    psbt_reason_no_prev_tx: "no previous transaction for a SegWit input",
    /// Why an input is not signed or finished: no path of a taproot input is this device's.
    psbt_reason_no_taproot_path_ours: "no leaf or key path of this taproot input is one of ours",
    /// Why an input is not signed or finished: a taproot input names nothing of its tree.
    psbt_reason_no_taproot_leaf_or_key: "the taproot input names no leaf script and no internal key of this tree",
    /// Why an input is not signed or finished: a script Faraday does not sign or finish.
    psbt_reason_unknown_script: "unknown script type",
    /// Why an input is not signed or finished: a taproot input states no internal key.
    psbt_reason_no_internal_key: "taproot input without tap_internal_key",
    /// Why an input is not signed or finished: a p2wsh multisig input that states a redeem_script.
    psbt_reason_stray_redeem_script: "p2wsh input with a redeem_script",
    /// Why an input is not signed or finished: a p2sh multisig input that states a witness_script.
    psbt_reason_stray_witness_script: "p2sh input with a witness_script",
    /// Why an input is not signed or finished: a redeem script too long to push.
    psbt_reason_redeem_too_long: "redeem script too long",
    /// Why an input is not signed or finished: a MuSig2 participants field that does not read.
    psbt_reason_musig_participants: "the MuSig2 participants field is malformed",
    /// Why an input is not signed or finished: a MuSig2 nonce or partial signature keyed to no participant.
    psbt_reason_musig_entry_key: "a MuSig2 nonce or partial signature names no participant",
    /// Why an input is not signed or finished: a MuSig2 public nonce that does not read.
    psbt_reason_musig_nonce: "a MuSig2 public nonce is malformed",
    /// Why an input is not signed or finished: a MuSig2 partial signature that does not read.
    psbt_reason_musig_signature: "a MuSig2 partial signature is malformed",
    /// Why an input is not signed or finished: a MuSig2 field of a script path.
    psbt_reason_musig_script_path: "the MuSig2 field names a script path",
    /// Why an input is not signed or finished: a MuSig2 field that does not read.
    psbt_reason_musig_malformed: "a MuSig2 field is malformed",
    /// Why an input is not signed or finished: a MuSig2 input asks for another sighash.
    psbt_reason_musig_sighash: "a MuSig2 input signs under SIGHASH_DEFAULT only",
    /// Why an input is not signed or finished: no MuSig2 nonce could be drawn.
    psbt_reason_musig_nonce_not_drawn: "a MuSig2 nonce could not be drawn for this input",
    /// Why an input is not signed or finished: another participant's MuSig2 nonce is missing.
    psbt_reason_musig_nonce_missing: "another participant's MuSig2 nonce is missing",
    /// Why an input is not signed or finished: the other participants' MuSig2 nonces do not aggregate.
    psbt_reason_musig_other_nonces: "the other participants' MuSig2 nonces do not aggregate",
    /// Why an input is not signed or finished: the input's MuSig2 nonces do not aggregate.
    psbt_reason_musig_nonces: "the MuSig2 nonces on this input do not aggregate",
    /// Why an input is not signed or finished: no registered MuSig2 wallet has the participants.
    psbt_reason_musig_no_wallet: "no registered MuSig2 wallet has these participants",
    /// Why an input is not signed or finished: the MuSig2 participants do not aggregate.
    psbt_reason_musig_no_aggregate: "the MuSig2 participants do not aggregate",
    /// Why an input is not signed or finished: the MuSig2 participants are not their field's key.
    psbt_reason_musig_not_field_key: "the MuSig2 participants are not the key the field names",
    /// Why an input is not signed or finished: a MuSig2 input states no internal key.
    psbt_reason_musig_no_internal_key: "the MuSig2 input states no internal key",
    /// Why an input is not signed or finished: a MuSig2 input states no chain and index.
    psbt_reason_musig_no_chain_index: "the MuSig2 input states no chain and index below the aggregate key",
    /// Why an input is not signed or finished: the aggregate does not derive at the chain and index.
    psbt_reason_musig_not_derived: "the MuSig2 aggregate key does not derive at the stated chain and index",
    /// Why an input is not signed or finished: the internal key is not the aggregate there.
    psbt_reason_musig_internal: "the MuSig2 internal key is not the aggregate at the stated chain and index",
    /// Why an input is not signed or finished: the aggregate does not tweak to the output key.
    psbt_reason_musig_no_tweak: "the MuSig2 aggregate key does not tweak to the output key",
    /// Why an input is not signed or finished: the input does not pay to the registered wallet.
    psbt_reason_musig_not_wallet: "the MuSig2 input does not pay to the registered wallet's address",
    /// Why signing or finishing refused: the text is not base64.
    psbt_error_base64: "invalid base64",
    /// The same: the bytes are not a PSBT.
    psbt_error_not_psbt: "not a PSBT (missing magic bytes)",
    /// The same: the PSBT does not read.
    psbt_error_parse: "invalid PSBT",
    /// The same. `{}`: the version.
    psbt_error_version: "unsupported PSBT version {}",
    /// The same, an input's. `{}`: the input, why (a `psbt_reason_*`).
    psbt_error_input: "input {}: {}",
    /// The same. `{}`: the input.
    psbt_error_no_utxo: "input {}: no UTXO information",
    /// The same. `{}`: the input.
    psbt_error_key_mismatch: "input {}: derived key does not match the PSBT's key origin",
    /// The same. `{}`: the key's fingerprint.
    psbt_error_no_such_key: "no loaded key with fingerprint {}",
    /// The same. `{}`: how many danger warnings.
    psbt_error_danger: "refused: {} danger warning(s)",
    /// The same: a warning passed as held that no hold takes.
    psbt_error_not_holdable: "refused: a warning no hold accepts",
    /// The same. `{}`: how many blocking warnings.
    psbt_error_blocked: "refused: {} blocking warning(s)",
    /// The same. `{}`: the input.
    psbt_error_sighash: "input {}: cannot compute signature hash",
    /// The same. `{}`: the input.
    psbt_error_verify: "input {}: produced signature failed verification",
    /// The same. `{}`: the input.
    psbt_error_nondeterministic: "input {}: signing twice gave different signatures",
    /// The same. `{}`: the input, the participant's key.
    psbt_error_musig_partial: "input {}: the MuSig2 partial signature of {} does not verify",
    /// The same. `{}`: the bitcoin library's own message.
    psbt_error_extract: "cannot extract transaction: {}",
    /// A warning on Review. `{}`: the input.
    psbt_warn_amount_unverified: "input {} carries no previous transaction; its amount cannot be checked",
    /// A warning on Review. `{}`: the input.
    psbt_warn_missing_utxo: "input {} has no UTXO data; its value and the fee cannot be verified",
    /// A warning on Review. `{}`: the input, why (a `psbt_reason_*`).
    psbt_warn_cannot_sign: "input {} cannot be signed here: {}",
    /// A warning on Review. `{}`: the input, the key, the path.
    psbt_warn_key_not_under_account: "input {} names your key {} at {}, which is not under a loaded account; ownership is checked only when signing",
    /// A warning on Review. `{}`: the input, the key, the path.
    psbt_warn_key_other_script: "input {} names your key {} at {}, in a script other than that account's",
    /// A warning on Review. `{}`: the input, the key, the path.
    psbt_warn_key_not_input: "input {} names your key {} at {}, but the derived key does not match the input",
    /// A warning on Review. `{}`: the input, its m and n, the first inputs' m and n.
    psbt_warn_multisig_shapes: "input {} is {}-of-{} multisig; earlier inputs are {}-of-{}",
    /// A warning on Review. `{}`: the input.
    psbt_warn_musig_nonce_replaced: "input {} carries a nonce of this device's that no open session holds; it is replaced and the partial signatures made against it are dropped",
    /// A warning on Review. `{}`: the input.
    psbt_warn_wallet_not_registered: "input {} spends from a wallet of several keys that is not registered here; register the wallet to sign",
    /// A warning on Review. `{}`: the input, the wait (a `psbt_wait_*`).
    psbt_warn_timelock: "input {} spends a path that waits {}, which this transaction's sequence and locktime do not meet; it can never be valid",
    /// A warning on Review. A wait. `{}`: blocks.
    psbt_wait_blocks: "{} blocks",
    /// A warning on Review. A wait. `{}`: intervals.
    psbt_wait_intervals: "{} intervals of 512 seconds",
    /// A warning on Review. A wait. `{}`: the block height.
    psbt_wait_height: "for block {}",
    /// A warning on Review. A wait. `{}`: the Unix time.
    psbt_wait_time: "until Unix time {}",
    /// A warning on Review. A wait for a secret.
    psbt_wait_preimage: "for a preimage",
    /// A warning on Review. `{}`: the output.
    psbt_warn_data_output: "output {} is a data (OP_RETURN) output",
    /// A warning on Review. `{}`: the output.
    psbt_warn_nonstandard_output: "output {} pays a non-standard script",
    /// A warning on Review. `{}`: the output, its amount in sat.
    psbt_warn_dust: "output {} ({} sat) is below the dust limit",
    /// A warning on Review. `{}`: the two outputs.
    psbt_warn_same_address: "outputs {} and {} pay the same address",
    /// A warning on Review. `{}`: the output, the input.
    psbt_warn_pays_to_input: "output {} pays back to the address of input {}",
    /// A warning on Review. The outputs are more than the inputs.
    psbt_warn_outputs_exceed: "outputs exceed inputs; the UTXO data is wrong",
    /// A warning on Review: an output, or the outputs' total, is over
    /// Bitcoin's money limit, which Bitcoin Core refuses.
    psbt_warn_outputs_over_max_money: "an output, or the outputs' total, is over 21,000,000 BTC",
    /// A warning on Review: an amount an input spends, or their total, is
    /// over Bitcoin's money limit.
    psbt_warn_inputs_over_max_money: "an input's amount, or the inputs' total, is over 21,000,000 BTC",
    /// A warning on Review: the transaction is a BIP-322 signed-message
    /// request, which Sign a message handles.
    psbt_warn_message_request: "this looks like a signed-message request, not a payment; use Sign a message",
    /// A warning on Review. `{}`: the kinds, joined by commas.
    psbt_warn_mixed_scripts: "inputs mix script types: {}",
    /// A warning on Review. A taproot script-path input.
    psbt_kind_tr_script: "p2tr script path",
    /// A warning on Review. A miniscript input. `{}`: its wrapper.
    psbt_kind_miniscript: "{} miniscript",
    /// A warning on Review. A multisig input. `{}`: its wrapper, m, n.
    psbt_kind_multisig: "{} {}-of-{}",
    /// A warning on Review. An input of a script Faraday does not take apart.
    psbt_kind_unknown: "unknown script",
    /// A warning on Review. `{}`: the locktime (a `psbt_locktime_*`).
    psbt_warn_locktime: "cannot be mined before {}; this signer has no clock to check it",
    /// A warning on Review. A locktime. `{}`: the block height.
    psbt_locktime_block: "block {}",
    /// A warning on Review. A locktime. `{}`: the date and time.
    psbt_locktime_time: "{} UTC",
    /// A warning on Review. A relative lock. `{}`: the input, the wait
    /// (a `psbt_wait_*`).
    psbt_warn_relative_lock: "input {} cannot be mined until {} after its coin confirmed",
    /// A warning on Review. None of the loaded keys can sign.
    psbt_warn_no_key: "none of your keys can sign this transaction",
    /// A warning on Review. `{}`: the key.
    psbt_warn_nonce_reuse: "one nonce is used twice under {}; a private key can follow from it",
    /// A warning on Review. `{}`: how many fields; two or more.
    psbt_warn_unknown_fields: "{} unknown or proprietary fields: not read",
    /// A warning on Review. One field.
    psbt_warn_unknown_field: "1 unknown or proprietary field: not read",
    /// A warning on Review. `{}`: the input.
    psbt_warn_inscription: "input {} carries an inscription envelope",
    /// A warning on Review. `{}`: the output.
    psbt_warn_musig_change_spoof: "output {} claims to be change of a MuSig2 wallet you are using, but the script is not that address",
    /// A warning on Review. `{}`: the output.
    psbt_warn_wallet_change_spoof: "output {} claims to be change of a wallet you are using, but the script is not that address",
    /// A warning on Review. `{}`: the output.
    psbt_warn_transfer: "output {} pays another of your wallets, not the one these coins are spent from; it counts as sent",
    /// A warning on Review. `{}`: the output, the key.
    psbt_warn_change_not_loaded: "output {} claims to belong to key {}, which is not loaded; treat it as a payment to others",
    /// A warning on Review. `{}`: the output, the key, the path.
    psbt_warn_change_spoof: "output {} claims to be change of your key {} at {}, but the script is not that address",
    /// A warning on Review. `{}`: the output, the key, the path.
    psbt_warn_change_not_registered: "output {} claims to be change of your key {} at {}; no wallet registered on this device holds that script, so it is counted as a payment to others. Register the wallet to check its change.",
    /// A warning on Review. `{}`: the output, the key, the path.
    psbt_warn_change_not_under_account: "output {} claims to be change of your key {} at {}, which is not under a loaded account; treat it as a payment to others",
    /// A warning on Review. `{}`: the output, the address index.
    psbt_warn_change_far_index: "output {} is change at address index {}; a wallet that scans with a gap limit does not find coins at an index that high",
    /// A warning on Review. `{}`: the key, the network it is loaded for, the one checked for.
    psbt_warn_key_network: "key {} is loaded for {} but the transaction is being checked for {}",
    /// A warning on Review. `{}`: the network checked for.
    psbt_warn_xpub_network: "a global xpub is for a different network than {}",
    /// A warning on Review. `{}`: the input, the path, its coin type, the network, its coin type.
    psbt_warn_coin_input: "input {} key path {} is for coin type {}; {} uses {}",
    /// A warning on Review. `{}`: the output, the path, its coin type, the network, its coin type.
    psbt_warn_coin_output: "output {} key path {} is for coin type {}; {} uses {}",
    /// A warning on Review. A sighash no algorithm accepts. `{}`: its value in hex.
    psbt_sighash_nonstandard: "non-standard {}",
    /// An output's script that has no address form, where Review names
    /// its address. `{}`: the script in hex.
    psbt_nonstandard_script: "non-standard script {}",
    /// A warning on Review. `{}`: the input, the sighash.
    psbt_warn_sighash_none: "input {} signs with {}: outputs are not committed to, anyone can redirect the funds",
    /// A warning on Review. `{}`: the input, the sighash.
    psbt_warn_sighash_nonstandard: "input {} requests a {} sighash type",
    /// A warning on Review. `{}`: the input, twice.
    psbt_warn_single_no_output: "input {} asks to sign with SIGHASH_SINGLE but the transaction has no output {}; such a signature would commit to nothing",
    /// A warning on Review. `{}`: the input, twice.
    psbt_warn_single: "input {} signs with SINGLE: only output {} is committed to; the others, change included, can be changed after you sign",
    /// A warning on Review. `{}`: the input, the sighash.
    psbt_warn_sighash_partial: "input {} signs with {}: not the whole transaction is committed to",
    /// A warning on Review. `{}`: the input.
    psbt_warn_signature_unkeyed: "input {}: a signature in it is not a signature of this transaction",
    /// A warning on Review. `{}`: the input, the key.
    psbt_warn_signature_invalid: "input {}: the signature under {} is not a signature of this transaction",
    /// A warning on Review, a line of the nonce check card and why a spend
    /// is not finished. `{}`: the input.
    psbt_warn_witness_mismatch: "input {}: its signing data does not match the output it spends",
    /// A warning on Review, a line of the nonce check card and why a spend
    /// is not finished: the interpreter refuses the final fields. `{}`: the input.
    psbt_final_signature_refused: "input {}: its final fields do not satisfy the script it spends",
    /// Why a spend is not finished (the Send it step's line, `Spend::finish`, a refused `collect`). `{}`: the input; its `witness_utxo` and previous transaction state different outputs, final or not.
    psbt_final_prev_disagree: "input {}: its witness_utxo and its previous transaction state different outputs",
    /// Why a spend is not finished (the Send it step's line, `Spend::finish`, a refused `collect`). `{}`: the input; it is final and the output it spends cannot be read.
    psbt_final_prev_unread: "input {}: it is final but the output it spends cannot be read",
    /// Why a spend is not finished (the Send it step's line, `Spend::finish`, a refused `collect`). `{}`: the input; it is not final and its previous transaction is not the one it spends or lacks the output.
    psbt_prev_false: "input {}: its previous transaction is not the one it spends, or has no such output",
    /// Why a file is not collected (`Spend::collect`). `{}`: the file, the input; the file brings a previous transaction or a `witness_utxo` that states another output than what the spend holds.
    wallet_file_states_false: "{}: input {}: it states an output its previous transaction does not",
    /// Why a file is not collected (`Spend::collect`). `{}`: the file, the input; the spend and the file state different outputs for it by a bare `witness_utxo` each, and no previous transaction proves either.
    wallet_outputs_differ_unproven: "{}: input {}: the spend and this file state different outputs for it, and neither is proven",
    /// Why a file is not collected (`Spend::collect`). `{}`: the file, the input; the spend and the file hold different final witnesses for it and the device cannot check either.
    wallet_finals_differ: "{}: input {}: the spend and this file hold different final witnesses for it, and neither can be checked",
    /// Why a file is not collected (`Spend::collect`). `{}`: the file, the input; the file holds a signature the spend lacks or holds differently, and no output is stated to check it.
    wallet_signature_unchecked: "{}: input {}: it holds a signature and no output is stated to check it",
    /// Why a file is not collected (`Spend::collect`). `{}`: the file, the input; the spend and the file hold different scripts for one control block and no output is stated to check them.
    wallet_scripts_differ: "{}: input {}: the spend and this file hold different scripts for one control block, and no output is stated to check them",
    /// A warning on Review. `{}`: the input, why (a `nonce_unchecked_*`).
    psbt_warn_signature_unchecked: "input {}: a signature was not checked: {}",
    /// A warning on Review. `{}`: the input; the element is a hash preimage, not a signature.
    psbt_warn_preimage_unchecked: "input {}: a hash preimage was not checked as a signature",
    /// A warning on Review. `{}`: the input, the key, the sighash signed with, the one asked for.
    psbt_warn_signed_none: "input {}: the signature under {} signs with {}, not the {} asked for: outputs are not committed to, anyone can redirect the funds",
    /// A warning on Review. `{}`: the input, the key, the sighash signed with, the one asked for.
    psbt_warn_signed_stricter: "input {}: the signature under {} signs with {}, stricter than the {} asked for: the transaction cannot be changed as the request allows",
    /// A warning on Review. `{}`: the input, the key, the sighash signed with, the one asked for.
    psbt_warn_signed_other: "input {}: the signature under {} signs with {}, not the {} asked for",
    /// Review's line for an input of the person's key in a miniscript
    /// that no registered wallet gives: it is left unsigned and the rest
    /// is signed (F141). `{}`: the input's index.
    step_input_no_wallet_script: "Input {} is left unsigned: no registered wallet covers this script. Register its wallet to sign it.",
    /// Review's line when Continue is pressed before a fee's hold (F136).
    step_review_hold_first: "Hold to pay this fee first.",
    /// Review's facts above the fee's hold (F136). `{}`: the fee in sat,
    /// its rate in sat/vB, its share of what is sent in percent.
    step_fee_hold_facts: "Fee {} sat · {} sat/vB · {}% of what it sends",
    /// Review's line naming the cases a fee this high is for (F136).
    step_fee_hold_cases: "A fee this high is paid on purpose for a child paying for its parent, a large batch or a consolidation.",
    /// Review's hold past a high fee or an absurd fee rate (F136).
    step_fee_hold: "Hold to pay this fee",
    /// Text in `step_body`. `{}`: `thousands(i.fee.to_sat())`, `i.fee_rate_sat_vb .map(|r| text!("{…`.
    step_sat_sat_vb: "{} sat · {} sat/vB",
    /// Text in `step_body`.
    step_spends: "Spends",
    /// Text in `step_body`.
    step_amount_unchecked: "Amount unchecked",
    /// Text in `step_body`.
    step_unchanged_by_signing: "Unchanged by signing",
    /// Text in `step_body`.
    step_changes_when_signed: "Changes when signed",
    /// Text in `step_body`.
    step_txid: "Txid",
    /// Text in `step_body`.
    step_not_known_until: "Not known until signed",
    /// Chip in `step_body`.
    step_no_warnings: "No warnings",
    /// Line in `step_body`.
    step_input: "Input",
    /// Line in `step_body`.
    step_output: "Output",
    /// Text in `step_body`.
    step_hide_the_table: "Hide the table",
    /// Text in `step_body`.
    step_show_the_table: "Show the table",
    /// Text in `step_body`.
    step_load_from_vault: "Load from vault",
    /// Text in `step_body`.
    step_signs_here: "Signs here",
    /// Text in `step_body`.
    step_not_here: "Not here",
    /// Text in `step_body`. `{}`: `f.name`.
    step_unlock: "Unlock {}",
    /// Text in `step_body`.
    step_scan_a_key: "Scan a key",
    /// Button in `step_body`: the Sign card of a single-key spend whose key is not loaded.
    step_scan_a_seedqr: "Scan a SeedQR",
    /// Button in `step_body`: a vault file in Files, opened to load the key. `{}`: its name.
    step_open_vault: "Open {}",
    /// Button in `step_body`: no vault file in Files.
    step_open_a_vault: "Open a vault",
    /// The foot's button on a single-key spend whose key is not loaded.
    step_load_the_key: "Load the key",
    /// Text in `step_body`.
    step_collect_a_signature: "Collect a signature",
    /// Text in `step_body`. `{}`: `app.medium.noun()`.
    step_remove_the_first: "Remove the {} first",
    /// Text in `step_body`. `{}`: `fps` joined by ", ".
    step_signed_keys: "Signed with {}",
    /// Line in `step_body`.
    step_no_signing_key: "No key in this session signs this transaction",
    /// Text in `step_body`. `{}`: `mine` joined by ", ".
    step_sign_with_keys: "Sign with {}",
    /// The Sign button in `step_body` where `step_sign_with_keys` is too
    /// wide for it and stands as a line above it.
    step_sign: "Sign",
    /// Text in `step_body`.
    step_input_inline: "input",
    /// Text in `step_body`.
    step_inputs: "inputs",
    /// Text in `step_body`. `{}`: `src`.
    step_from_inline: "from {}",
    /// Text in `step_body`.
    step_signed_here: "Signed here",
    /// Text in `step_body`.
    step_verified: "Verified",
    /// Text in `step_body`.
    step_not_needed: "Not needed",
    /// Text in `step_body`: a partial signature that verifies and that
    /// this device did not write, so cannot confirm.
    step_in_file_unconfirmed: "In file, unconfirmed",
    /// Text in `step_body`: a partial signature that does not verify.
    step_does_not_verify: "Does not verify",
    /// Line in `step_body`.
    step_no_cosigner_s: "No cosigner's copy in Files",
    /// Text in `step_body`.
    step_scan_a_copy: "Scan a copy",
    /// Text in `step_body`. `{}`: `missing`, `if s.signers.is_empty() { "" } else {…`, `if missing == 1 { EN.step_signature }…`.
    step_needs: "Needs {}{} {}",
    /// Text in `step_body`, after a space.
    step_more: "more",
    /// Text in `step_body`.
    step_signature: "signature",
    /// Text in `step_body`.
    step_signatures: "signatures",
    /// Text in `step_body`.
    step_signed_psbt: "Signed PSBT",
    /// Text in `step_body`.
    step_for_the_wallet: "For the wallet that wrote it · not finalised",
    /// Text in `step_body`.
    step_finished_transaction: "Finished transaction",
    /// Text in `step_body`.
    step_ready_to_broadcast: "Ready to broadcast",
    /// Text in `step_body`.
    step_remove: "Remove",
    /// Text in `step_body`.
    step_make_again: "Make again",
    /// Text in `step_body`. `{}`: `short(&text!("{}", tx.compute_txid()))`, `tx.vsize()`.
    step_vb: "{} · {} vB",
    /// Text in `step_body`.
    step_decode_button: "Decode it",
    /// Text in `step_body`.
    step_hide_the_raw_hex: "Hide the raw hex",
    /// Text in `step_body`.
    step_show_the_raw_hex: "Show the raw hex",
    /// Line in `step_body`.
    step_this_device: "This device",
    /// Text in `step_body`.
    step_nonce_here: "Nonce here",
    /// Text in `step_body`.
    step_waiting: "Waiting",
    /// Text in `step_body`. `{}`: `name`.
    step_add_source: "Add from {}",
    /// Button in `step_body`.
    step_go_first_share: "Go first: share this device's nonce",
    /// Text in `step_body`. `{}`: `k + 1`.
    step_path: "Path {}",
    /// Text in `step_body`. `{}`: `slot`, `fp`.
    step_key: "key {} {}",
    /// Line in `step_body`.
    step_no_wait: "No wait",
    /// Text in `step_body`. `{}`: `l`.
    step_waits: "Waits {}",
    /// Text in `step_body`.
    step_this_transaction_s: "This transaction's sequence allows this path",
    /// Text in `step_body`.
    step_this_transaction: "This transaction's sequence does not allow this path",
    /// Text in `step_body`.
    step_continue: "Continue",
    /// Button in `step_body`: the foot's button on the Review until the end of the card is in view.
    step_scroll_to_end: "Scroll to the end",
        }
    };
}
