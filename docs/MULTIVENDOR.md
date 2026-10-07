# Two implementations of every cryptographic step

Status: proposal, 2026-10-06. Nothing here is built. The owner will decide
the implementation later; this records the question, what is known, the
options and a recommendation.

## 1. The idea

Every cryptographic result Faraday shows or acts on comes from OpenSigner's
code. The owner's idea: compute each result a second time with an
independent implementation, written by other people, in another language
where possible, and show next to the result which implementations produced
it and whether they agree. A bug in one implementation, or a tampered
dependency, then shows as a disagreement instead of a wrong address, a
wrong key or a wrong signature.

## 2. What OpenSigner is made of

OpenSigner is partly its own Rust and partly libraries. Which part a
result comes from decides what a second implementation has to differ from.

| Part | Where it comes from |
|---|---|
| BIP-39 words to seed, BIP-85, SLIP-39, codex32 (BIP-93), Seed XOR, aezeed, MuSig2 (BIP-327, 328, 390), FROST (BIP-445), silent payments (BIP-352), the descriptor and wallet-policy layer | OpenSigner's own Rust, in `osk-bip` |
| Transactions, PSBT (BIP-174, 370), BIP-32 derivation, sighash (BIP-143, 341), addresses, script | `rust-bitcoin` 0.32 |
| Descriptors and miniscript parsing and satisfaction | `rust-miniscript` 12 |
| ECDSA and Schnorr signing and verifying, all curve arithmetic | `rust-secp256k1`, which wraps Bitcoin Core's C library `libsecp256k1` |
| SHA-2, HMAC, PBKDF2, Argon2id, ChaCha20-Poly1305 | the RustCrypto crates |

Most "other" Bitcoin libraries also call `libsecp256k1`: libwally,
python-bitcoinlib, python-bitcointx, and embit when its C binding is
present. A second implementation that uses it agrees with OpenSigner on the
curve by construction and checks only the layers above. Real independence
at the curve needs one of the implementations that do their own arithmetic.

## 3. Candidate second implementations

| Candidate | Language | Curve arithmetic | Covers | Notes |
|---|---|---|---|---|
| Bitcoin Core's functional-test framework (`test_framework/key.py`, `script.py`, `messages.py`, `descriptors.py`) | Python | its own, pure Python | ECDSA with RFC 6979, BIP-340 Schnorr, every sighash, transaction serialisation, descriptor checksums | Written by Core's developers; no dependencies; slow |
| The BIPs' own reference code (BIP-340, 327 MuSig2, 445 FROST, 352 silent payments, 93 codex32) | Python | pure Python | exactly the BIP each comes with | Written by the BIPs' authors: the strongest second opinion for those BIPs |
| `python-mnemonic` (Trezor) | Python | none needed | BIP-39 | The BIP's reference implementation |
| `python-shamir-mnemonic` (Trezor) | Python | none needed | SLIP-39 | The SLIP's reference implementation |
| embit | Python | pure Python fallback, or `libsecp256k1` | BIP-32, 39, PSBT, descriptors, miniscript, addresses | Used by SeedSigner and Specter; runs on MicroPython (not yet checked whether its pure fallback does) |
| noble-curves, scure-bip32/39, btc-signer | JavaScript | its own, audited | the same set as embit | Needs a JavaScript engine (QuickJS is about 1 MB) |
| btcd / btcec | Go | its own | the same set | A static binary; a large one |
| `argon2-cffi`, PyNaCl | Python over C | n/a | Argon2id (the PHC reference C), XChaCha20-Poly1305 (libsodium) | Independent of RustCrypto; `tools/vault/open.py` already opens a vault this way |

## 4. What can be computed twice

| Operation | Standards | Needs a secret | Second implementation |
|---|---|---|---|
| Seed from words | BIP-39 | yes | python-mnemonic |
| Master fingerprint, account xpub, child keys | BIP-32, 44, 48, 49, 84, 86 | from a seed: yes; from an xpub: no | Core's `key.py` plus a BIP-32 routine, or embit |
| Addresses of a wallet | BIP-380 to 386, 388, 389 | no | Core's `descriptors.py` and `script.py`, or embit |
| Descriptor checksum | BIP-380 | no | Core's `descriptors.py` |
| Sighash of each input | BIP-143, 341, 342 | no | Core's `script.py` |
| Signature verification | ECDSA, BIP-340 | no | Core's `key.py` |
| Signature recomputation (the nonce check, done twice) | RFC 6979, BIP-340 | yes | Core's `key.py`: Faraday signs deterministically, so the signatures must match byte for byte |
| Transaction id, finished transaction | BIP-141, 144 | no | Core's `messages.py` |
| MuSig2 aggregate key, partial signature verification | BIP-327, 328, 390 | no | the BIP's reference code |
| FROST group key, partial signature verification | BIP-445 | no | the BIP's reference code |
| Silent payment address, output detection | BIP-352 | scan key: yes | the BIP's reference code |
| Child seeds and passwords | BIP-85 | yes | its reference implementation |
| SLIP-39, codex32, Seed XOR combine | SLIP-39, BIP-93 | yes | the references above |
| Signed message | BIP-322, 137 | signing: yes; checking: no | Core's framework for BIP-322 |
| Vault cost and seal | Argon2id, HKDF, XChaCha20-Poly1305 | yes | argon2-cffi, PyNaCl |

Not worth computing twice: drawing randomness (only its conversion from
dice, coins or cards is deterministic, and that can be checked), reading QR
codes, and the screens.

## 5. Where the second implementation runs

**A. At build time, in `just`.** A differential test runs OpenSigner and
the Python references side by side over many random inputs and every
published vector: words, paths, descriptors, transactions, signatures.
It adds nothing to the stick, costs nothing at run time, and catches most
implementation bugs before a build. It does not catch a dependency changed
after the build, or a fault on the machine at hand.

**B. At run time, public data only.** A verifier process on the stick,
started by the app, with its own user, no files and no network: it
receives public inputs over a pipe and returns its results. Every check in
§4 that needs no secret runs here, which covers addresses, checksums,
sighashes, signature verification, transaction ids and the MuSig2 and
FROST public steps.

**C. At run time, secrets too.** The same process also receives seeds and
private keys, for seed to fingerprint, signature recomputation, BIP-85 and
the share schemes.

## 6. On screen

Each result that was computed twice carries a small mark beside it:
"✓ Rust · ✓ Python", in green when they agree. Pressing it lists the
implementations, their versions and what each produced. A result computed
once carries "Rust only". A disagreement is red, names both results, and
stops what depends on it: no export, no signing, no loading of the wallet.

## 7. Costs and risks

- **More code trusted.** CPython is about 20 MB in a Buildroot image;
  MicroPython well under 1 MB. Either is more code in the image and in the
  reproducible build, pinned and hashed like everything else.
- **Secrets in a second language.** Python cannot reliably wipe a seed:
  strings are immutable and copied by the runtime. The lock cycle still
  ends the process and the kernel zeroes its memory (`PLAN.md` §5.3), but
  between locks the seed would sit in a second process's heap. This is the
  cost of option C, and the reason to keep it separate from B.
- **Speed.** Pure-Python curve arithmetic takes tens of milliseconds an
  operation on a laptop and more on a Pi. Enough for the handful of
  results on one screen; too slow for a search over hundreds of addresses
  or a vanity search, which stay single.
- **Agreement is not proof.** Two implementations can share a
  misreading of a BIP; the BIPs' own reference code and published vectors
  are the defence there.

## 8. Recommendation

1. A first: build-time differential tests against the Python references.
   Cheapest, catches the most.
2. B next: the run-time verifier for public data, with the mark on screen.
3. C last, behind a setting, once the owner accepts a second process
   holding seeds.

## 9. Open questions for the owner

1. Stick image first, or the desktop app first?
2. MicroPython (small, fewer libraries) or CPython (larger, every library
   in §3)?
3. Should the second implementation ever receive seeds (option C)?
