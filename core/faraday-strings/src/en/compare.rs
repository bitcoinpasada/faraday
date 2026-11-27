//! Faraday's English words for the compare area.

macro_rules! words_compare {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- Compare two PSBTs and the fee bump (docs/COMPARE.md), faraday-core/src/decode -----
    /// Text in `words::difference`. `{}`: the two versions.
    compare_version: "Version: {} sent, {} returned",
    /// Text in `words::difference`. `{}`: the two locktimes.
    compare_locktime: "Locktime: {} sent, {} returned",
    /// Text in `words::difference`. `{}`: the two PSBT versions.
    compare_psbt_version: "PSBT version: {} sent, {} returned",
    /// Text in `words::difference`. `{}`: the input's outpoint.
    compare_input_sent_only: "Input {}: in the sent only",
    /// Text in `words::difference`. `{}`: the input's outpoint.
    compare_input_returned_only: "Input {}: in the returned only",
    /// Text in `words::difference`. `{}`: the outpoint, the two sequences.
    compare_sequence: "Input {}: sequence {} sent, {} returned",
    /// Text in `words::difference`. `{}`: the outpoint, the two amounts.
    compare_spends: "Input {}: spends {} sent, {} returned",
    /// Text in `words::difference`. `{}`: the outpoint.
    compare_amount_sent_only: "Input {}: amount stated in the sent only",
    /// Text in `words::difference`. `{}`: the outpoint.
    compare_amount_returned_only: "Input {}: amount stated in the returned only",
    /// Text in `words::difference`. `{}`: the outpoint, the two counts.
    compare_signatures: "Input {}: {} signatures sent, {} returned",
    /// Text in `words::difference`. `{}`: the outpoint.
    compare_final_sent_only: "Input {}: final in the sent only",
    /// Text in `words::difference`. `{}`: the outpoint.
    compare_final_returned_only: "Input {}: final in the returned only",
    /// Text in `words::difference`. `{}`: its amount; the output's address is under it.
    compare_output_sent_only: "Output of {}, in the sent only, to",
    /// Text in `words::difference`. `{}`: its amount; the output's address is under it.
    compare_output_returned_only: "Output of {}, in the returned only, to",
    /// Text in `words::difference`. `{}`: the two amounts; the output's address is under it.
    compare_output_amount: "Output of {} sent and {} returned, to",
    /// Text in `words::difference`. `{}`: the outpoint.
    compare_spent_twice_sent: "Input {}: spent twice in the sent",
    /// Text in `words::difference`. `{}`: the outpoint.
    compare_spent_twice_returned: "Input {}: spent twice in the returned",
    /// Text in `words::difference`; the output's address is under it.
    compare_yours_sent_only: "Output yours, in the sent only, to",
    /// Text in `words::difference`; the output's address is under it.
    compare_yours_returned_only: "Output yours, in the returned only, to",
    /// Text in `words::output`, an output with no address. `{}`: the script's length.
    compare_script_bytes: "script {} bytes",
    /// Text in `words::differences`, where there are none.
    compare_no_differences: "No differences",
    /// Text in `words::result`.
    bump_only_fee: "Only the fee is raised",
    /// Text in `words::result`.
    bump_more_changed: "More than the fee changed",
    /// Text in `words::result`.
    bump_cannot_tell: "Cannot tell whether only the fee is raised",
    /// Text in `words::result_from`. `{}`: the original's file name.
    bump_only_fee_from: "Only the fee is raised from {}",
    /// Text in `words::result_from`. `{}`: the original's file name.
    bump_more_changed_from: "More than the fee changed from {}",
    /// Text in `words::result_from`. `{}`: the original's file name.
    bump_cannot_tell_from: "Cannot tell whether only the fee is raised from {}",
    /// Text in `words::line`. `{}`: the outpoint.
    bump_input_dropped: "Input {}: in the original, not in the replacement",
    /// Text in `words::line`. `{}`: the outpoint.
    bump_input_added: "Input {}: added, and not yours",
    /// Text in `words::line`. `{}`: the outpoint, the two sequences.
    bump_sequence: "Input {}: sequence {} in the original, {} in the replacement",
    /// Text in `words::line`. `{}`: the outpoint, the two amounts.
    bump_spends: "Input {}: spends {} in the original, {} in the replacement",
    /// Text in `words::line`. `{}`: the outpoint.
    bump_spent_twice_original: "Input {}: spent twice in the original",
    /// Text in `words::line`. `{}`: the outpoint.
    bump_spent_twice_replacement: "Input {}: spent twice in the replacement",
    /// Text in `words::line`. `{}`: the two versions.
    bump_version: "Version: {} in the original, {} in the replacement",
    /// Text in `words::line`. `{}`: the two locktimes.
    bump_locktime: "Locktime: {} in the original, {} in the replacement",
    /// Text in `words::line`. `{}`: the replacement's amount; the payee is under it.
    bump_pays_other: "Pays {} to",
    /// Text in `words::line`, under the payee. `{}`: the original's amount.
    bump_pays_other_original: "The original pays {}",
    /// Text in `words::line`. `{}`: the original's amount; the payee is under it.
    bump_no_longer_pays: "No longer pays {} to",
    /// Text in `words::line`. `{}`: the replacement's amount; the payee is under it.
    bump_pays_new: "Pays {}, which the original does not, to",
    /// Text in `words::line`. `{}`: the two fees in sat.
    bump_fee: "Fee: {} sat in the original, {} sat in the replacement",
    /// Text in `words::line`. `{}`: the two fee rates in sat/vB.
    bump_fee_rate: "Fee rate: {} sat/vB in the original, {} sat/vB in the replacement",
    /// Text in `words::line`. `{}`: the outpoint.
    bump_amount_not_stated: "Input {}: amount not stated",
    /// Text in `words::line`. `{}`: the key's fingerprint; the output's address is under it.
    bump_change_not_loaded: "Output of change for {}, which is not loaded, to",
    /// Text in `words::line`. `{}`: the outpoint, the key's fingerprint.
    bump_input_not_loaded: "Input {}: added, of {}, which is not loaded",
    /// Text in `words::line`; the output's address is under it.
    bump_not_stated_yours: "Output in the original only, and not stated to be yours, to",
    /// Text in `words::compare_facts`, a fact's label.
    compare_fact_transaction: "Transaction",
    /// Text in `words::compare_facts` and `words::bump_facts`, a fact's label.
    compare_fact_fee: "Fee",
    /// Text in `words::compare_facts`, a fact's label.
    compare_fact_inputs: "Inputs",
    /// Text in `words::compare_facts`, a fact's label.
    compare_fact_outputs: "Outputs",
    /// Text in `words::compare_facts`. `{}`: how many.
    compare_fact_in_both: "{} in both",
    /// Text in `words::compare_facts`. `{}`: how many.
    compare_fact_sent_only: "{} in the sent only",
    /// Text in `words::compare_facts`. `{}`: how many.
    compare_fact_returned_only: "{} in the returned only",
    /// Text in `words::fee_value`, a fee that cannot be worked out.
    compare_fee_not_known: "Not known",
    /// Text in `words::fee_value`. `{}`: the fee in sat, its rate in sat/vB.
    compare_fee_value: "{} sat · {} sat/vB",
    /// Text in `words::bump_facts`, a value's label.
    bump_fact_original: "Original",
    /// Text in `words::bump_facts`, a value's label.
    bump_fact_replacement: "Replacement",
    /// Text in `words::bump_facts`, a fact's label.
    bump_fact_inputs_added: "Inputs added",
    /// Text in `words::bump_facts`, a fact's label.
    bump_fact_rbf: "Replace-by-fee",
    /// Text in `words::bump_facts`.
    bump_signalled: "Signalled by the original",
    /// Text in `words::bump_facts`.
    bump_not_signalled: "Not signalled by the original",
    /// Compare's card with no PSBT or finished transaction in Files.
    compare_no_files: "No PSBT or transaction in Files",
    /// What a scan and a card say of bytes that are neither a PSBT nor a finished transaction.
    compare_not_a_transaction: "Not a PSBT or a transaction",
    /// A finished transaction's line in Compare's list. `{}`: its id in short form.
    compare_row_finished: "Finished · {}",
    /// Compare's Result: the section label of the fee bump result.
    compare_fee_bump_label: "Fee bump",
    /// The review's fee bump: the button that compares the PSBT under
    /// review with the other transaction.
    compare_from_review: "Compare",
    /// Compare's Result: the section label of the differences.
    compare_differences_label: "Differences",
        }
    };
}
