# BIP-392, BIP-321 and BIP-353: where a silent payment address travels

The three BIP texts that carry a silent payment address somewhere other
than a wallet screen, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/`, downloaded
2026-09-19.

| File | SHA-256 | Read by |
|---|---|---|
| `bip-0392.mediawiki` | `5be97bb2ebca7a047923a111854fbefea65262107d19afccd218291d00401493` | people |
| `bip-0321.mediawiki` | `6fcb782620390c0a60958d983e0ca9ffbd67eadfba37169f94e1228028eee8c4` | people |
| `bip-0353.mediawiki` | `b9239e2df8b8551ac4d5ef1aae26a2b9c74981dda0481641333ed782b85bab61` | people |

BIP-392 is a draft and its "Test Vectors" section reads "TBD"; its
examples are written as `sp(spscan1q...)` with the payload elided, so
there is no published string to check an encoding against. What
`core/osk-bip/tests/silent.rs` checks instead is the encoding the BIP
specifies — the human-readable part, the version character `q` and the
payload `ser256(b_scan) || serP(B_spend)` — and that the descriptor is
that key expression inside `sp()` with the key origin the BIP puts
before it.

BIP-321's examples carry the placeholder `sp1qsilentpayment` rather than
a real address, so the URI is checked as the form the BIP states,
`bitcoin:?sp=<address>`, over an address the BIP-352 vectors publish.

BIP-353 defines the owner name `user.user._bitcoin-payment.domain` and
the TXT record holding that URI. The DNSSEC proofs it publishes are for
resolvers; this device resolves nothing and only writes the record line
a person hands to whoever runs their zone.
