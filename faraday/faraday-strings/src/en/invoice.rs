//! Faraday's English words for the invoice area.

macro_rules! words_invoice {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- invoice (faraday-core/src/invoice/) -----
    /// Title in `invoice_screen`.
    invoice_title: "Decode a Lightning invoice",
    /// The empty field's hint.
    invoice_field: "Invoice or offer",
    /// Button: reads the text in the field.
    invoice_decode: "Decode",
    /// Button: opens the scanner for an invoice.
    invoice_scan: "Scan",
    /// Button: empties the field.
    invoice_clear: "Clear",
    /// The About page's text and the guide line above the field.
    invoice_about: "Reads an invoice or an offer and shows what it asks. Nothing is paid, signed or kept.",
    /// Row name.
    invoice_network: "Network",
    /// Row name.
    invoice_amount: "Amount",
    /// Row value: an invoice that names no amount.
    invoice_any_amount: "Any amount",
    /// An amount in whole satoshis. `{}`: the number.
    invoice_sat: "{} sat",
    /// An amount that is not whole satoshis. `{}`: millisatoshis.
    invoice_msat: "{} msat",
    /// Row name.
    invoice_description: "Description",
    /// Row name.
    invoice_description_hash: "Description hash",
    /// Row name.
    invoice_payee: "Payee key",
    /// Row name.
    invoice_payee_from: "Payee key from",
    /// Row value: the invoice's n field.
    invoice_payee_named: "Named in the invoice",
    /// Row value: no n field.
    invoice_payee_recovered: "Recovered from the signature",
    /// Row name.
    invoice_payment_hash: "Payment hash",
    /// Row name.
    invoice_payment_secret: "Payment secret",
    /// Row name.
    invoice_created: "Created",
    /// Row name.
    invoice_expires: "Expires",
    /// A time. `{}`: the date, then the time of day.
    invoice_utc: "{} {} UTC",
    /// Row name.
    invoice_final_cltv: "Final CLTV delta",
    /// Row value. `{}`: the number.
    invoice_blocks: "{} blocks",
    /// Row name.
    invoice_fallback_p2pkh: "Fallback address, P2PKH",
    /// Row name.
    invoice_fallback_p2sh: "Fallback address, P2SH",
    /// Row name.
    invoice_fallback_witness: "Fallback address, witness",
    /// Row name. `{}`: the hint's number, then the hop's.
    invoice_route_hop: "Route hint {}, hop {}",
    /// Row value. `{}`: node key, channel id, base fee msat, ppm, CLTV delta.
    invoice_hop: "{} · channel {} · base {} msat · {} ppm · delta {}",
    /// Row name.
    invoice_features: "Feature bits",
    /// Row name.
    invoice_metadata: "Payment metadata",
    /// Row name. `{}`: the field's tag character.
    invoice_unknown: "Unknown field {}",
    /// Row name: an offer's metadata.
    invoice_offer_metadata: "Metadata",
    /// Row value: an offer's metadata. `{}`: its length in bytes.
    invoice_offer_metadata_len: "{} bytes",
    /// Row name: an odd field of an offer that BOLT #12 does not define. `{}`: its type.
    invoice_offer_unknown: "Unknown field, type {}",
    /// Row value: that field. `{}`: its length in bytes.
    invoice_offer_unknown_len: "{} bytes",
    /// Refusal.
    invoice_err_too_long: "Refused: the text is over 1023 characters",
    /// Refusal.
    invoice_err_mixed_case: "Refused: the text mixes upper and lower case",
    /// Refusal.
    invoice_err_bech32: "Refused: the text is not Bech32",
    /// Refusal.
    invoice_err_checksum: "Refused: the checksum does not hold",
    /// Refusal.
    invoice_err_prefix: "Refused: the text does not start with a Lightning network prefix",
    /// Refusal.
    invoice_err_amount: "Refused: the amount is malformed",
    /// Refusal.
    invoice_err_too_short: "Refused: the text is shorter than a timestamp and a signature",
    /// Refusal.
    invoice_err_truncated: "Refused: a field runs past the end of the text",
    /// Refusal. `{}`: the field's tag.
    invoice_err_field: "Refused: field {} is malformed",
    /// Refusal. `{}`: the field's tag.
    invoice_err_duplicate: "Refused: field {} appears twice",
    /// Refusal.
    invoice_err_no_payment_hash: "Refused: no payment hash",
    /// Refusal.
    invoice_err_no_payment_secret: "Refused: no payment secret",
    /// Refusal.
    invoice_err_description: "Refused: it needs one description or description hash, not both or neither",
    /// Refusal. `{}`: the feature bit.
    invoice_err_feature: "Refused: it requires feature bit {}, which is not known here",
    /// Refusal.
    invoice_err_signature: "Refused: the signature does not verify",
    /// Row name: what the text is.
    invoice_kind: "Kind",
    /// Row value: a BOLT #12 offer.
    invoice_kind_offer: "Offer",
    /// Row name: the networks an offer is for.
    invoice_chains: "Networks",
    /// Row value: a network.
    invoice_chain_mainnet: "mainnet",
    /// Row value: a network.
    invoice_chain_testnet3: "testnet3",
    /// Row value: a network.
    invoice_chain_testnet4: "testnet4",
    /// Row value: a network.
    invoice_chain_signet: "signet",
    /// Row value: a network.
    invoice_chain_regtest: "regtest",
    /// Row value: a chain Faraday does not know. `{}`: its hash.
    invoice_chain_other: "Chain {}",
    /// Row value: an amount in a currency. `{}`: the amount, the ISO 4217 code.
    invoice_currency_amount: "{} {}",
    /// Row name: what the issuer wrote about itself.
    invoice_issuer: "Issuer, in its own words",
    /// Row name: the node an invoice is asked from.
    invoice_issuer_id: "Issuer key",
    /// Row name. `{}`: the path's number.
    invoice_path: "Blinded path {}",
    /// Row value: a path starting at a node. `{}`: the node key, the hops.
    invoice_path_node: "Starts at node {} · hops {}",
    /// Row value: a path starting at a channel. `{}`: the short channel id, the direction, the hops.
    invoice_path_channel: "Starts at channel {} · direction {} · hops {}",
    /// Row name: how many items one invoice may be for.
    invoice_quantity: "Quantity",
    /// Row value: an offer that allows any number.
    invoice_quantity_any: "Any number",
    /// Row value. `{}`: the most.
    invoice_quantity_up_to: "Up to {}",
    /// Refusal: an invoice request.
    invoice_err_request: "Refused: this is an invoice request; only offers are read",
    /// Refusal.
    invoice_err_offer_too_long: "Refused: the text is over 4296 characters",
    /// Refusal.
    invoice_err_join: "Refused: a + joins something other than two parts of the text",
    /// Refusal.
    invoice_err_offer_prefix: "Refused: the text does not start with lno1",
    /// Refusal.
    invoice_err_padding: "Refused: the text ends in more than four bits of padding, or padding that is not zero",
    /// Refusal.
    invoice_err_not_minimal: "Refused: a number in the offer is not in its shortest form",
    /// Refusal.
    invoice_err_order: "Refused: the offer's fields are not in increasing order",
    /// Refusal. `{}`: the field type.
    invoice_err_out_of_range: "Refused: field {} is not one an offer may carry",
    /// Refusal. `{}`: the field type.
    invoice_err_unknown_even: "Refused: field {} is required and not known here",
    /// Refusal. `{}`: the field type.
    invoice_err_text: "Refused: field {} is not valid text",
    /// Refusal.
    invoice_err_chains_empty: "Refused: it names networks and lists none",
    /// Refusal.
    invoice_err_chain_repeated: "Refused: a network is named twice",
    /// Refusal.
    invoice_err_chain_unknown: "Refused: none of its networks is one Bitcoin network known here",
    /// Refusal.
    invoice_err_currency: "Refused: the currency is not an ISO 4217 code known here",
    /// Refusal.
    invoice_err_amount_zero: "Refused: the amount is zero",
    /// Refusal.
    invoice_err_amount_too_large: "Refused: the amount is over 21 million bitcoin",
    /// Refusal.
    invoice_err_amount_no_description: "Refused: it has an amount and no description",
    /// Refusal.
    invoice_err_currency_no_amount: "Refused: it names a currency and no amount",
    /// Refusal.
    invoice_err_expiry_late: "Refused: the expiry is past the year 9999",
    /// Refusal.
    invoice_err_features_minimal: "Refused: the feature bits are not in their shortest form",
    /// Refusal.
    invoice_err_no_issuer: "Refused: it names neither an issuer key nor a path",
    /// Refusal.
    invoice_err_path: "Refused: a blinded path is empty or malformed",
    /// Refusal. `{}`: the field type.
    invoice_err_point: "Refused: field {} holds a key that is not a valid point",
        }
    };
}
