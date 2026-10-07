# BIP 445 FROST signing vectors

The six vector files and their summary from BIP 445's reference
repository, verbatim, from
`https://raw.githubusercontent.com/siv2r/bip-frost-signing/master/python/vectors/`
at commit `bb5396f90d45ba5a954cbfd1af80f1b45e02b229` (2026-08-26). The
BIP is not yet published; to refresh a file, download it again and
check the digest changed for a reason.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `nonce_gen_vectors.json` | 2026-09-18 | `b9a2256b923784adec9183290454001857e2becbe50bb6956adfaa63147048c3` | `core/osk-bip/tests/frost.rs` |
| `nonce_agg_vectors.json` | 2026-09-18 | `b2f470b9ae6aed57bab3336c915d8e23e3db4c0e51b7dd181cf93216e09b7c7a` | `core/osk-bip/tests/frost.rs` |
| `sign_verify_vectors.json` | 2026-09-18 | `9c6651f34f5e714af276fc6ee243a8e69bc3a03b29805dce1526353e0213f737` | `core/osk-bip/tests/frost.rs` |
| `tweak_vectors.json` | 2026-09-18 | `236adcb702a052713d2d8cc88fff603eb81c2e9324ffcfffb11b2c9c59a445d9` | `core/osk-bip/tests/frost.rs` |
| `det_sign_vectors.json` | 2026-09-18 | `b8c2f66194736df7d279c140ae90e8b7562cf03ca731f54c4f97cde4d87c72a0` | `core/osk-bip/tests/frost.rs` |
| `sig_agg_vectors.json` | 2026-09-18 | `4ac061cf6a931ea3c044fa497338ad15d9b332e8eb190b93424ce50ba06d1c4c` | `core/osk-bip/tests/frost.rs` |
| `test_vectors_summary.md` | 2026-09-18 | `6bb8799adc83314b9f580f54697076d1b787ab679c467e74c3c404db7a64fd59` | people |

`test_vectors_summary.md` is the reference's own description of the
files' layout: every signing file carries four test groups, one per
`(t, n)` configuration (2-of-3, 1-of-3, 3-of-3, 3-of-5), each with its
own key setup and shared inputs that the cases index into. Identifiers
are 0-based; a participant with identifier `id` holds the polynomial's
value at `id + 1`, and the group secret is its value at 0.

`core/osk-bip/tests/frost.rs` reads all six. `nonce_gen`: the secret and
public nonce each fixed `rand` draws, with and without the optional
inputs. `nonce_agg`: both sums and the three malformed nonces, each
blamed on the signer the file names. `sign_verify`: every partial
signature, every refusal to sign, the three verifications that fail and
the two that are refused, in all four groups. `tweak`: seven tweak
orders per group and the four refusals. `det_sign`: every
`DeterministicSign` result, including the sole signer with no other
nonce, and every refusal. `sig_agg`: every aggregate signature, each
also checked as a BIP-340 signature of the message under the tweaked
group key, and the two refusals per group. The groups' `secshares` and
`pubshares` also serve as the dealer's check: from any `t` of a group's
secret shares the others and the group key are recovered.

The BIP ships no vectors for `ValidateThresholdInfo` and none for key
generation; the dealer in this tree is checked against the groups above
and against the RFC 9591 draft's Appendix F.5 shares that the
reference's own `trusted_dealer.py` tests carry.
