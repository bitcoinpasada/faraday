//! Faraday's English words for the screens spend area.

macro_rules! words_screens_spend {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- nonce_check (faraday-core/src/screens/spend.rs) -----
    /// Text in `nonce_check`. `{}`: `check.rows.len()`, `if check.rows.len() == 1 { EN.nonce_c…`, `here`.
    nonce_check_nonce_check_valid: "Nonce check · {} {} valid · {} recomputed here",
    /// Text in `nonce_check`.
    nonce_check_signature: "signature",
    /// Text in `nonce_check`.
    nonce_check_signatures: "signatures",
    /// Text in `nonce_check`.
    nonce_check_nonce_check: "Nonce check · a signature is wrong",
    /// The nonce check's headline when only the final fields of inputs fail the script they spend.
    nonce_check_fields_refused: "Nonce check · final fields fail the script",
    /// The nonce check's headline when only signing data (a `scriptSig` or witness) does not match its output.
    nonce_check_witness_wrong: "Nonce check · signing data does not match its output",
    /// Text in `nonce_check`. `{}`: `reuse.name()`, `inputs` joined by ", ".
    nonce_check_one_nonce: "{}: one nonce on inputs {} · this gives the key away",
    /// Line in `nonce_card` when the nonce history was compared and nothing was used before.
    nonce_history_clean: "Not seen before in this vault",
    /// Line in `nonce_card` when the nonce history of several open vaults was compared. `{}`: how many.
    nonce_history_clean_vaults: "Not seen before in {} open vaults",
    /// Line in `nonce_card` when nonce records of a vault do not read. `{}`: how many.
    nonce_history_unread: "Not checked against this vault: {} nonce records do not read",
    /// Line in `nonce_card` for a key whose nonce an open vault holds over another message. `{}`: the key's name, the inputs.
    nonce_history_used: "{}: nonce used before, on another transaction, on inputs {}",
    /// Text in `nonce_check`. `{}`: `r.input`.
    nonce_check_input: "Input {}",
    /// Text in `nonce_check`.
    nonce_check_invalid: "Invalid: not this key's over this transaction",
    /// Text in `nonce_check`. `{}`: `why`.
    nonce_check_not_checked: "Not checked · {}",
    /// Text in `nonce_check`. `{}`: `mode`.
    nonce_check_valid_nonce: "Valid · nonce recomputed, {}",
    /// How the nonce was recomputed, in `nonce_check_valid_nonce`: RFC 6979 ground to a low R.
    nonce_mode_low_r: "low R",
    /// The same: RFC 6979's first nonce.
    nonce_mode_first: "first",
    /// The same: BIP-340 with zero auxiliary randomness.
    nonce_mode_bip340: "BIP-340",
    /// Text in `nonce_check`: the row is a hash preimage on a script leaf, not a signature.
    nonce_check_preimage: "Hash preimage · not a signature",
    /// Why a signature was not checked, in `nonce_check_not_checked`: the input has no previous output.
    nonce_unchecked_no_prevout: "no previous output",
    /// The same: a script Faraday does not take apart.
    nonce_unchecked_script: "unsupported script",
    /// The same: the script names no key.
    nonce_unchecked_no_key: "the script names no key",
    /// Text in `nonce_check`.
    nonce_check_wrong_nonce: "Valid · not the nonce this key makes: do not broadcast",
    /// Text in `nonce_check`.
    nonce_check_valid_signed: "Valid · signed elsewhere, its nonce needs that device's key",
    /// Text in `nonce_check`. `{}`: `rest.len()`, `if ok { EN.nonce_check_none_wrong } e…`.
    nonce_check_more: "{} more {}",
    /// Text in `nonce_check`.
    nonce_check_none_wrong: "· none wrong",
    /// Text in `nonce_check`.
    nonce_check_one_wrong: "· one is wrong",
    /// Text in `nonce_check`: a valid signature whose key no key origin names, so its nonce was not recomputed.
    nonce_check_no_origin: "Valid · no key origin: its nonce is not recomputed",
    /// Text in `nonce_check`: a valid signature under a key held here, read from a final witness or scriptSig.
    nonce_check_finalized_here: "Valid · finalized: its nonce is not recomputed",
    /// Text in `nonce_check`: a valid signature on a script that was not run, whose key is not counted.
    nonce_check_script_not_run_key: "Valid · key not counted: its script was not run",
    /// Headline of `nonce_check` for a raw transaction whose inputs state no output they spend. `{}`: how many inputs.
    nonce_unread_head: "Nonce check · {} not checked: the outputs they spend are not here",
    /// Headline of `nonce_check` when inputs, final or not, state an output their previous transaction does not. `{}`: how many inputs.
    nonce_disagree_head: "Nonce check · {} input(s) state an output their previous transaction does not",
    /// Headline of `nonce_check` when a final input spends a script that is not run.
    nonce_check_script_not_run: "Nonce check · a script was not run",
    /// Headline of `nonce_check` when the transaction carries no signature.
    nonce_check_no_signatures: "Nonce check · no signatures",
    // ----- spend (faraday-core/src/screens/spend.rs) -----
    /// Line in `spend`.
    spend_no_transaction: "No transaction loaded",
    /// Button in `spend`.
    spend_open_files: "Open Files",
    /// Text in `spend`.
    spend_wallets: "Wallets",
    /// Text in `spend`.
    spend_sign_a_transaction: "Sign a transaction",
    // ----- spend_status (faraday-core/src/screens/spend.rs) -----
    /// Text in `spend_status`. `{}`: `app.medium.for_the()`.
    spend_status_both_wait: "Both wait {}",
    /// Text in `spend_status`.
    spend_status_complete: "Complete",
    /// Text in `spend_status`. `{}`: `missing - 1`.
    spend_status_sign_and_collect: "Sign here and collect {} more",
    /// Text in `spend_status`.
    spend_status_sign_here: "Sign here",
    /// Text in `spend_status`. `{}`: `app.medium.a()`.
    spend_status_insert: "Insert {} to write it",
    /// Text in `spend_status`.
    spend_status_open_files: "Open Files",
    /// Text in `spend_status`.
    spend_status_finish: "Finish",
    /// Text in `spend_status`. `{}`: `can_sign_here` joined by ", ".
    spend_status_sign_with_keys: "Sign with {}",
    /// Text in `spend_status` where `spend_status_sign_with_keys` with
    /// two keys or more is too wide for the button. `{}`: how many keys.
    spend_status_sign_with_count: "Sign with {} keys",
    /// Text in `spend_status` while a check stops signing here. `{}`: the
    /// step that says why, `EN.app_transaction`.
    spend_status_stopped: "Not signed here: see {}",
    // ----- spend_bar (faraday-core/src/screens/spend.rs) -----
    /// Text in `spend_bar`. `{}`: `count`, `needed`.
    spend_bar_count: "{} of {}",
    // ----- step_summary (faraday-core/src/screens/spend.rs) -----
    /// Text in `step_summary`.
    step_summary_no_loaded: "No loaded wallet matches",
    /// Text in `step_summary`.
    step_summary_addresses: "Addresses · Compare",
    /// Text in `step_summary`. `{}`: `btc(i.amount_to_others.to_sat())`, `i.fee_rate_sat_vb .map(|r| text!("{…`.
    step_summary_sends: "Sends {} BTC · fee {} sat/vB",
    /// Text in `step_summary` when the fee cannot be worked out (F136).
    /// `{}`: what it sends.
    step_summary_sends_fee_not_known: "Sends {} BTC · fee not known",
    /// Text in `step_summary`. `{}`: `here`, `needed`.
    step_summary_here_needed: "{} here · {} needed",
    /// Text in `step_summary`.
    step_summary_not_signed: "Not signed here yet",
    /// Text in `step_summary`. `{}`: `fps` joined by ", ".
    step_summary_signed_keys: "Signed with {}",
    /// Text in `step_summary`. `{}`: `s.signers.len()`, `needed`.
    step_summary_count: "{} of {}",
    /// Text in `step_summary`. `{}`: `have`, `rows.len()`.
    step_summary_of_nonces: "{} of {} nonces",
    /// Text in `step_summary`.
    step_summary_way_one: "way to spend",
    /// Text in `step_summary`.
    step_summary_ways_many: "ways to spend",
    /// Text in `step_summary`.
    step_summary_signed: "Signed PSBT and finished transaction",
    /// Text in `step_summary`.
    step_summary_not_finished: "Not finished yet",
    /// Text in `step_summary`.
    step_summary_not_reached: "Not reached yet",
    // ----- step (faraday-core/src/screens/spend.rs) -----
    /// Text in `step_body`. `{}`: `Session::shape(wl)`, `wl.source`.
    step_wallet_source: "{} · from {}",
    /// Line in `step_body`.
    step_no_loaded_wallet: "No loaded wallet matches this transaction. Change cannot be verified until its wallet is loaded.",
    /// Text in `step_body`.
    step_receive_0_0: "Receive 0/0",
    /// Text in `step_body`.
    step_receive_0_1: "Receive 0/1",
    /// Text in `step_body`.
    step_change_1_0: "Change 1/0",
    /// Line in `step_body`.
    step_no_wallet_to_show: "No wallet to show addresses from",
    /// Text in `step_body`.
    step_sends: "Sends",
    /// Text in `step_body`: the first row of the Review on a small panel.
    step_to_others: "To others",
    /// Text in `step_body`.
    step_recipient_inline: "recipient",
    /// Text in `step_body`.
    step_recipients: "recipients",
    /// Text in `step_body`. `{}`: `if change { 1 } else { 0 }`, `index`.
    step_to_path: "to {}/{}",
    /// Text in `step_body`.
    step_change: "Change",
    /// Text in `step_body`: an output on the receive chain of one of the person's wallets.
    step_receive: "Receive",
    /// Text in `step_body`: a payment into another of the person's wallets. `{}`: its name.
    step_to_wallet: "To {}",
    /// Text in `step_body`.
    step_fee: "Fee",
    /// The fee on Review and in the spend panel when an input's amount is
    /// not known, so the fee cannot be worked out (F136).
    step_fee_not_known: "Not known",
    /// A row on Review: the block height before which the transaction
    /// cannot be mined.
    step_locked_until: "Locked until",
    /// A row on Review: whether the transaction can be replaced (BIP 125).
    step_replacement: "Replacement",
    /// The value of Review's replacement row: an input signals it.
    step_replaceable: "Replaceable",
    /// The value of Review's replacement row: no input signals it and the version is not 3.
    step_not_signalled: "Not signalled",
    /// Review's line when Faraday signs nothing of the transaction: a
    /// blocked or a danger warning with no hold (F136).
    step_review_refused: "Faraday does not sign this transaction. Make it again in the coordinator without what is marked above.",
    /// Review's line when an input's spend path waits longer than the
    /// transaction's sequence and locktime allow (F136).
    step_review_timelock: "This transaction can never be valid. In the coordinator, set the input's sequence or the transaction's locktime to the path's wait, then make it again.",
    /// Review's line when only a wallet not registered stops it (F135,
    /// F136): the register offer above it is the way on.
    step_review_register: "Faraday signs this transaction once its wallet is registered.",
    /// Review's line for an input of the person's key in a taproot tree
    /// that no registered wallet gives: it is left unsigned and the rest
    /// is signed (F141). `{}`: the input's index.
    step_input_no_wallet_tree: "Input {} is left unsigned: no registered wallet covers this taproot tree. Register its wallet to sign it.",
    /// Review's warning of a fee that is a large share of what is sent.
    /// `{}`: the fee in sat, the share in percent.
    step_review_fee_share: "fee {} is {}% of the amount sent",
    /// Review's warning of a fee rate far above any network's. `{}`: the
    /// rate in sat/vB, whole.
    step_review_fee_rate: "fee rate is about {} sat/vB",
    // ----- spend_panel (faraday-core/src/screens/spend.rs) -----
    /// Line in `spend_panel`.
    spend_panel_this_transaction: "This transaction",
    /// Text in `spend_panel`.
    spend_panel_verified: "Verified",
    /// Text in `spend_panel`.
    spend_panel_none: "None",
    /// Text in `spend_panel`.
    spend_panel_wallet: "Wallet",
    /// Text in `spend_panel`.
    spend_panel_not_loaded: "Not loaded",
    /// Text in `spend_panel`.
    spend_panel_sends: "Sends",
    /// Text in `spend_panel`. `{}`: `btc(i.amount_to_others.to_sat())`.
    spend_panel_btc: "{} BTC",
    /// Text in `spend_panel`.
    spend_panel_to_label: "To",
    /// Text in `spend_panel`.
    spend_panel_fee: "Fee",
    /// Text in `spend_panel`. `{}`: `thousands(i.fee.to_sat())`, `i.fee_rate_sat_vb .map(|r| text!("{…`.
    spend_panel_sat_sat: "{} sat · {} sat/vB",
    /// Text in `spend_panel`.
    spend_panel_change: "Change",
    /// Text in `spend_panel`.
    spend_panel_txid: "Txid",
    /// Line in `spend_panel`.
    spend_panel_signatures: "Signatures",
    /// Text in `spend_panel`. `{}`: `count`, `needed`.
    spend_panel_count: "{} of {}",
    // ----- signers_count (faraday-core/src/screens/spend.rs) -----
    /// Text in `signers_count`. `{}`: `here`, `needed`.
    signers_count_of_signatures_here: "{} of {} signatures here",
    /// Text in `signers_count`. `{}`: `here`, `needed`, `needed - here`.
    signers_count_of_signatures: "{} of {} signatures here · {} more needed",
    /// Text in `signers_count`. `{}`: the signatures in the file, `needed`,
    /// how many more are needed.
    signers_count_in_the_file: "{} of {} in the file · {} more needed",
    /// Text in `signers_count`.
    signers_count_can_spend: "Can spend",
    /// Line in `signers_count`.
    signers_count_continue: "Continue",
    // ----- made_line (faraday-core/src/screens/spend.rs) -----
    /// Button in `made_line`.
    made_line_remove: "Remove",
        }
    };
}
