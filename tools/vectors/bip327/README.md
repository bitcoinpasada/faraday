# BIP-327 MuSig2 vectors

All eight of BIP-327's vector files, verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0327/vectors/`.
To refresh one, download it again and check the digest changed for a
reason.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `key_agg_vectors.json` | 2026-09-12 | `03c02a97e4ef3f2edfbc8e6013c127496dfcfd5889cfca60ddf009a4e9091cab` | `core/osk-bip/tests/musig.rs` |
| `key_sort_vectors.json` | 2026-09-12 | `2389fa0c146cfd7455c643ca240ec32835dcfc916f430f50dd94d0b49c9ea16c` | `core/osk-bip/tests/musig.rs` |
| `nonce_gen_vectors.json` | 2026-09-17 | `2e823580fc072427f0db0f000212cc9124ad2b9dca2b58357eb65088aee4358d` | `core/osk-bip/tests/musig.rs` |
| `nonce_agg_vectors.json` | 2026-09-17 | `8409e87b81ea769759598ad3ce53b277a78afffb3a490a86ce02c4d69984524b` | `core/osk-bip/tests/musig.rs` |
| `sign_verify_vectors.json` | 2026-09-17 | `692eecc101f3e515c29137f05031935e1210d2a01bab91e674eb0234f095c15c` | `core/osk-bip/tests/musig.rs` |
| `sig_agg_vectors.json` | 2026-09-17 | `15f14c034fb2a5739d7ce638be94c5b37ea675a2e01159092dd93b59d69c3439` | `core/osk-bip/tests/musig.rs` |
| `tweak_vectors.json` | 2026-09-17 | `80ce6385ce062644ad1f4edcb9d4797f70ddb0b74769e4099f51b3c9e6ab4aff` | `core/osk-bip/tests/musig.rs` |
| `det_sign_vectors.json` | 2026-09-17 | `3d4fdb64b24e31762f20830036dc0c59d39fa896649131b54b87906ffdc6e9e8` | `core/osk-bip/tests/musig.rs` |

`core/osk-bip/tests/musig.rs` reads all eight. `key_agg`: the four valid
cases must aggregate to the key the file states and the five error cases
must be refused. `key_sort`: the order the file states. `nonce_gen`: the
secret and public nonce each fixed `rand'` draws. `nonce_agg`: both
sums, including the one that is the point at infinity and is written as
33 zero bytes, and the three malformed nonces. `sign_verify`: six
partial signatures, six refusals, three verification failures and two
malformed contributions. `tweak`: five tweak orders and a tweak at the
curve order. `sig_agg`: four aggregate signatures, each also checked as a
BIP-340 signature of the message under the tweaked aggregate key, and a
partial signature above the curve order. `det_sign`: four
`DeterministicSign` results with and without the optional `rand`, and
five refusals.

The `key_agg` file's tweaks appear only in error cases — one tweak at the
curve order and one whose result is the point at infinity — so
`ApplyTweak` is covered there for its refusals, against BIP-390's
descriptors for the taproot tweak it computes, and by `tweak_vectors`
for the rest.
