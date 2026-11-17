//! Faraday's English words for the silent area.

macro_rules! words_silent {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- silent (faraday-core/src/silent/mod.rs) -----
    /// Toast in `silent_open`.
    silent_load_a_key_first: "Load a key first",
    /// Error in `silent_code`.
    silent_not_open: "Silent payments is not open",
    /// Error in `silent_code`.
    silent_choose_a_key: "Choose a key first",
    /// Text in `silent_code`. `{}`: `fp`.
    silent_silent_payment: "Silent payment address · {}",
    /// Error in `silent_code`.
    silent_no_address: "No address for this key",
    /// Text in `silent_code`. `{}`: `s.label`.
    silent_label: "Label {}",
    /// Text in `silent_code`.
    silent_public_receives: "Public: receives payments, spends nothing",
    /// Text in `silent_code`.
    silent_silent_payments_address: "Silent payments address",
    /// Error in `silent_code`.
    silent_no_record: "No record for this key",
    /// Text in `silent_code`.
    silent_public_watch: "Public: watch only, spends nothing",
    /// Text in `silent_code`.
    silent_silent_payments_record: "Silent payments record",
    /// Title in `silent_code`.
    silent_silent_payments_dns_record: "Silent payments DNS record",
    /// Error in `silent_code`.
    silent_no_dns_record: "Type a name and a domain first",
    /// Toast in `silent_act`.
    silent_no_vault_open: "No vault is open",
    /// Text in `silent_act`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    silent_silent_payments_scan: "Silent payments scan key · {}",
    /// Text in `silent_act`. `{}`: `title`.
    silent_is_in_the_vault: "{} is in the vault",
    /// Text in `silent_act`.
    silent_a_silent_payments: "A silent payments scan key",
    /// Text in `silent_act`.
    silent_whoever_has: "Whoever has it sees every payment to this address; it cannot spend them",
    /// Title of the secret sheet for a silent payments wallet's `sp()` descriptor.
    silent_a_silent_payments_descriptor: "A silent payments sp() descriptor, with its scan key",
    /// Toast in `silent_descriptor_out`. `{}`: the record's fingerprint.
    silent_descriptor_needs_key: "Load key {} to write its sp() descriptor",
    /// Toast in `silent_add_wallet`.
    silent_load_the_key: "Load the key first",
    /// Text in `silent_add_wallet`. `{}`: `fp_text(faraday_bip::keys::Fingerprint(fp))`.
    silent_silent_payments_n: "Silent payments {}",
    /// Text in `silent_add_wallet`.
    silent_silent_payments: "Silent payments",
    /// Toast in `silent_add_wallet`.
    silent_added_to_your: "Added to your wallets",
    /// Toast in `silent_check`.
    silent_not_a_transaction: "Not a transaction",
    /// Caution under a silent payments wallet's address while no key with
    /// its fingerprint is loaded, and a line of its QR picture. `{}`: the
    /// key's fingerprint.
    silent_not_checked: "Not checked: load key {} to check this address",
    /// Danger line in place of a silent payments wallet's address where a
    /// loaded key with its fingerprint gives another scan key or spend
    /// key, and the error of its QR code. `{}`: the key's fingerprint.
    silent_record_differs: "This wallet's record does not match key {}: its scan key or spend key is not that key's. Do not use its address",
    // ----- silent (faraday-core/src/silent/screen.rs) -----
    /// Text in `TITLES`.
    silent_key: "Key",
    /// Text in `TITLES`.
    silent_address: "Address",
    /// Text in `TITLES`.
    silent_scan_key: "Scan key",
    /// Text in `TITLES`.
    silent_check_a_payment: "Check a payment",
    /// Text in `draw`.
    silent_wallets: "Wallets",
    /// Text in `summary`.
    silent_not_chosen: "Not chosen",
    /// Text in `summary`.
    silent_a_secret_shows: "A secret: shows every payment",
    /// Text in `summary`.
    silent_needs_a_previous: "Needs a previous transaction",
    /// Text in `summary`.
    silent_a_transaction: "A transaction from Files",
    /// Line in `card`.
    silent_its_key_is_not_loaded: "Its key is not loaded: labels, the scan key and a check need it",
    /// Text in `card`.
    silent_add_its_key: "Add its key",
    /// Text in `card`.
    silent_no_label: "No label",
    /// Text in `card`.
    silent_show_as_qr: "Show as QR",
    /// Text in `card`.
    silent_as_a_bitcoin: "As a bitcoin: link",
    /// Text in `card`. `{}`: `app.medium.for_the()`.
    silent_record: "Record {}",
    /// Text in `card`.
    silent_record_as_qr: "Record as QR",
    /// Heading in `dns_card`.
    silent_dns_record: "DNS record",
    /// Field label in `dns_card`.
    silent_dns_name: "Name",
    /// Field label in `dns_card`.
    silent_dns_domain: "Domain",
    /// Error in `dns_card`.
    silent_dns_invalid: "Not a valid DNS name",
    /// Button in `dns_card`.
    silent_dns_record_as_qr: "DNS record as QR",
    /// Text in `card`.
    silent_record_as_png: "Record as PNG",
    /// Text in `card`.
    silent_add_as_a_wallet: "Add as a wallet",
    /// Button in `ending`: starts the tool again.
    silent_another: "Another",
    /// Chip in `card`.
    silent_secret_shows: "Secret · shows every payment, cannot spend",
    /// Text in `card`. `{}`: `v.name`.
    silent_save_into: "Save into {}",
    /// Text in `card`.
    silent_out_unprotected: "Out unprotected…",
    /// Text in `checked_text`. `{}`: `n`, `if n == 1 { EN.silent_output } else {…`.
    silent_pays_this_wallet: "Pays this wallet: {} {}",
    /// Text in `checked_text`.
    silent_output: "output",
    /// Text in `checked_text`.
    silent_outputs: "outputs",
    /// Text in `checked_text`.
    silent_pays_nothing: "Pays nothing to this wallet",
    /// Text in `checked_text`.
    silent_spends_nothing: "Spends nothing a silent payment is made from",
    /// Text in `checked_text`.
    silent_input_key_missing: "Not signed: an input's key is not in it yet",
    /// Text in `checked_text`.
    silent_key_not_loaded: "Its key is not loaded",
    /// Text in `check_card`.
    silent_the_address: "the address",
    /// Text in `check_card`.
    silent_change: "change",
    /// Text in `check_card`. `{}`: `m`.
    silent_label_inline: "label {}",
    /// Text in `check_card`. `{}`: `p.vout`, `crate::ui::btc(p.amount.to_sat())`, `to`.
    silent_output_row: "Output {} · {} · to {}",
    /// Text in `check_card`. `{}`: `txid`.
    silent_needs_the_transaction: "Needs the transaction it spends from, {}: copy it into Files and check again",
    /// Text in `check_card`. `{}`: `it.name`.
    silent_check: "Check {}",
    /// Line in `check_card`.
    silent_no_transaction: "No transaction in Files",
        }
    };
}
