//! Faraday's English words for the decode area.

macro_rules! words_decode {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- decode (faraday-core/src/decode.rs) -----
    /// Text in `script_kind`.
    decode_other_script: "Other script",
    /// Text in `sequence_text`.
    decode_final: "final",
    /// Text in `sequence_text`.
    decode_locktime_no_rbf: "locktime on, no replace-by-fee",
    /// Text in `sequence_text`.
    decode_relative_time: "relative time lock, replace-by-fee",
    /// Text in `sequence_text`.
    decode_relative_block: "relative block lock, replace-by-fee",
    /// Text in `locktime_text`.
    decode_0_none: "0 · none",
    /// Text in `locktime_text`. `{}`: `n`.
    decode_block_height: "{} · block height",
    /// Text in `locktime_text`. `{}`: `n`.
    decode_unix_time: "{} · Unix time",
    /// Text in `btc`. `{}`: `Amount::from_sat(sats).to_btc()`.
    decode_btc: "{} BTC",
    /// Decode's refusal, as a toast: an output, or the outputs' total, is
    /// over Bitcoin's money limit, which Bitcoin Core refuses.
    decode_outputs_over_max_money: "Not decoded: an output, or the outputs' total, is over 21,000,000 BTC",
    /// Decode's refusal, as a toast: an amount an input spends, or their
    /// total, is over Bitcoin's money limit.
    decode_inputs_over_max_money: "Not decoded: an input's amount, or the inputs' total, is over 21,000,000 BTC",
    // ----- decode_back (faraday-core/src/decode/screen.rs) -----
    /// Text in `decode_back`.
    decode_back_sign_transaction: "Sign a transaction",
    /// Text in `decode_back`.
    decode_back_spend: "Spend from a backup",
    /// Text in `decode_back`.
    decode_back_files: "Files",
    /// Text in `decode_back`.
    decode_back_wallets: "Wallets",
    /// Text in `decode_back`.
    decode_back_tools: "Tools",
    /// Text in `decode_back`.
    decode_back_back: "Back",
    /// Text in `decode_back`.
    decode_back_nonce_check: "Nonce check",
    // ----- decode (faraday-core/src/decode/screen.rs) -----
    /// Title in `decode_screen`.
    decode_decode_a_transaction: "Decode a transaction",
    /// Text in `decode_screen`. `{}`: `f`, `f as f64 / t.size.1.max(1) as f64`.
    decode_sat_sat_vb: "{} sat · {} sat/vB",
    /// Text in `decode_screen`.
    decode_fee_not_stated: "Not stated: the amounts going in are not here",
    /// Row label in `decode_screen`: where the amounts going in came from.
    decode_amounts_in: "Amounts in",
    /// Its value. `{}`: the PSBT's file name, or the spend's.
    decode_amounts_from: "From {}",
    /// Its value when no PSBT of this transaction is here.
    decode_amounts_none: "Not here",
    /// Its value when PSBTs of this transaction state different amounts.
    decode_amounts_disagree: "PSBTs of it state different amounts",
    /// Text in `decode_screen`.
    decode_transaction: "Transaction id",
    /// Text in `decode_screen`.
    decode_witness_id: "Witness id",
    /// Text in `decode_screen`.
    decode_version: "Version",
    /// Text in `decode_screen`.
    decode_locktime: "Locktime",
    /// Text in `decode_screen`.
    decode_size: "Size",
    /// Text in `decode_screen`. `{}`: `t.size.0`, `t.size.1`, `t.size.2`.
    decode_bytes_vb_wu: "{} bytes · {} vB · {} WU",
    /// Text in `decode_screen`.
    decode_fee: "Fee",
    /// Text in `decode_screen`. `{}`: `t.inputs.len()`.
    decode_inputs: "Inputs · {}",
    /// Text in `decode_screen`.
    decode_no_witness: "No witness",
    /// Text in `decode_screen`. `{}`: `input.witness.len()`, `if input.witness.len() == 1 { EN.deco…`, `sizes` joined by ", ".
    decode_witness_bytes: "Witness · {} {} · {} bytes",
    /// Text in `decode_screen`.
    decode_item: "item",
    /// Text in `decode_screen`.
    decode_items: "items",
    /// Text in `decode_screen`. `{}`: `input.sequence`, `wit`, `if input.script_sig > 0 { text!(" ·…`.
    decode_sequence: "Sequence {} · {}{}",
    /// Text in `decode_screen`, after a space. `{}`: `input.script_sig`.
    decode_script_sig_bytes: "· scriptSig {} bytes",
    /// Text in `decode_screen`. `{}`: `t.outputs.len()`.
    decode_outputs: "Outputs · {}",
    /// Text in `decode_screen`. `{}`: `name`, `n`.
    decode_change_back: "Change · back to {} · 1/{}",
    /// Text in `decode_screen`. `{}`: `name`, `n`.
    decode_back_to_receive: "Back to {} · receive 0/{}",
    /// Text in `decode_screen`.
    decode_pays_from_loaded: "Pays out of the loaded wallets",
    /// Text in `decode_screen`.
    decode_hide_the_hex: "Hide the hex",
    /// Text in `decode_screen`.
    decode_load_a_wallet: "Load a wallet",
    /// Text in `decode_screen`.
    decode_show_the_hex: "Show the hex",
    /// Text in `decode_screen`.
    decode_show_as_qr: "Show as QR",
    /// Line in `decode_landing`.
    decode_no_transaction_waits: "No transaction waits in Files",
    /// Line in `decode_landing`: the files to choose from.
    decode_choose_transaction: "Choose a transaction",
    /// Line in `landing` (check_message.rs): the files to choose from.
    decode_choose_message: "Choose a signed message",
        }
    };
}
