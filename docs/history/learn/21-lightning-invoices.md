# Lightning invoices

**Kind:** History
**Status:** The Learn pages' text as it was edited before the in-app guide (`docs/guide/`) replaced them

## What an invoice is

A Lightning invoice is one line of text that a payee gives to a payer to ask for one payment. It starts `lnbc` on the main network, `lntb` on a test network, `lntbs` on signet and `lnbcrt` on regtest, and the amount, when it names one, follows. The format is BOLT #11, written in Bech32 like a native segwit address, so a wrong character is caught by its checksum.

It is not a Bitcoin address. Paying it needs a Lightning wallet with a channel; this device has none, and Decode a Lightning invoice only reads. Nothing is paid, signed or kept.

## What it says

The amount, in satoshis or in thousandths of one, or no amount at all, which leaves the payer to choose. A description, or the hash of a longer description kept elsewhere. The payment hash and the payment secret. The time it was made and the number of seconds it lasts, an hour when it says nothing. The number of blocks the last hop must allow. Hints for routes to the payee, and an on-chain address to pay instead where the payee gave one.

The payment hash is the hash of a secret the payee holds. The payer's wallet pays to that hash; the payee gives up the secret to take the money, and the payer keeps it afterwards as proof of payment. The payment secret ties the payment to this invoice, so that a node along the way cannot make up a payment of its own.

## Who is paid

The invoice carries a signature, and the payee's node key is either written in it or worked out from the signature. Decode a Lightning invoice shows which. A signature that does not verify is refused.

The key says which node signed the invoice. It does not say who that node belongs to: compare it with the key you were given by the payee, from a source other than the invoice. The description and the creation time are what the payee wrote, from the payee's own clock.

## Offers

An offer is one line that starts `lno1`, which a payee gives out to be paid more than once. The format is BOLT #12, written in Bech32 without a checksum. A line may be broken with a `+` and a line break, and Decode a Lightning invoice reads it joined.

An offer is not an invoice. It names no payment hash; the payer's wallet asks the payee's node for an invoice over the Lightning network, which this device cannot do. Decode a Lightning invoice reads an offer and shows what it carries: the networks, the amount in bitcoin or in a currency by its ISO 4217 code, or any amount, the description, the issuer's own words, the issuer's node key, blinded paths by their first node and number of hops, the expiry, the quantity, the feature bits, and the fields it does not know.

An offer has no signature, so it does not say who made it. Compare the issuer key and the paths with what the payee gave you from another source. The expiry is shown as written and not judged: this computer's clock is not trusted to say whether it has passed.

An invoice request, which starts `lnr1`, and an invoice that starts `lni1`, are messages between two nodes and are refused. A refused offer shows the reason in one line.

## What is refused

A text with a bad checksum, upper and lower case mixed, an amount finer than a thousandth of a satoshi, a field of the wrong length, a payment hash or secret missing or given twice, both a description and its hash or neither, a feature the invoice requires that this reader does not know, and a signature that does not verify. Where BOLT #11 leaves a case open, Faraday refuses it. A refused invoice shows the reason in one line.
