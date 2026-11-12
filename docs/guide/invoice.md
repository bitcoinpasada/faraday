# Decode a Lightning invoice

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Invoice`
**Key:** invoice

Reads a Lightning invoice or an offer and shows what it asks. Nothing is paid, signed or kept.

## Invoice or offer

**Key:** invoice.field

The text goes in the **Invoice or offer** box, typed or scanned. **Decode** reads it, **Scan** takes it from a QR code, and **Clear** empties the box and the result.

An invoice starts with a Lightning network prefix and is read as BOLT #11; an offer starts with `lno1` and is read as BOLT #12. A text that is neither is refused with the reason in one line.

## An invoice

**Key:** invoice.invoice

An invoice asks for one payment. **Network** and **Amount** are shown, the amount as **Any amount** when the invoice names none. **Description** or **Description hash** says what is paid for. **Payee key** is the node's key, and **Payee key from** says whether it was **Named in the invoice** or **Recovered from the signature**; a signature that does not verify is refused.

**Payment hash** and **Payment secret** identify the payment. **Created** and **Expires** are in UTC, from the clock of the payee. **Final CLTV delta**, fallback address rows, route hint hops, **Feature bits** and **Payment metadata** follow where the invoice carries them. The payee key does not say who the node belongs to.

## An offer

**Key:** invoice.offer

An offer is one line a payee hands out to be paid more than once; it has no payment hash and no signature. **Kind** reads **Offer**. **Networks** lists the networks it is for, and **Amount** is in bitcoin, in a currency by its ISO 4217 code, or **Any amount**.

**Description**, **Issuer, in its own words**, **Issuer key** and blinded path rows say who offers it; a path is shown by its first node or channel and its number of hops. **Expires** is shown as written and not judged against the clock. **Quantity** is **Any number** or up to a count. **Feature bits**, **Metadata** and unknown fields follow. An invoice request, which starts `lnr1`, is refused.
