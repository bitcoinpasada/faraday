# faraday-crypto

The bottom crate. It holds secrets in memory and is the one place the other
crates call for hashes and key stretching. It has no signing, no curve
operations and no random number generator: entropy is handed in by the shell.

## What it computes

- `sha256`, `hmac_sha256`, `hmac_sha512`, `pbkdf2_hmac_sha256`,
  `pbkdf2_hmac_sha512` (`hash.rs`), `blake2b` and `scrypt`.
- `Sealed` and `SessionKey` (`sealed.rs`): a secret encrypted in memory with
  ChaCha20-Poly1305 under a per-session key; plaintext exists only inside
  `Sealed::with`.
- `work.rs`: the `Compute` token and the kinds of costly work
  (`docs/COMPUTE.md`).

## Written in this crate

- `blake2b.rs`: BLAKE2b (RFC 7693). One caller: AEZ's `Extract` in
  `faraday-bip`.
- `scrypt.rs`: scrypt (RFC 7914) over the PBKDF2 below it. One caller:
  aezeed's passphrase stretch.

SHA-256, HMAC, PBKDF2 and ChaCha20-Poly1305 come from RustCrypto crates.

## Checked against

- `blake2b.rs`, inline tests: RFC 7693 appendix A (`abc`) and the empty
  message.
- `scrypt.rs`, inline tests: the first two RFC 7914 §12 vectors. The aezeed
  tests in `core/faraday-bip/tests/all/aez.rs` run scrypt at aezeed's own
  `N = 32768`.
- `tests/primitives.rs`: SHA-256 known answers, RFC 4231 cases 1 and 2 for
  HMAC-SHA-256 and HMAC-SHA-512, PBKDF2-HMAC-SHA-256 against RFC 7914 §11,
  PBKDF2-HMAC-SHA-512, and `Secret` and `Pinned` behaviour.

## `unsafe`

Only `src/pinned.rs`; the crate sets `unsafe_code = "deny"` and that file
allows it for itself (the rest of the workspace is `forbid`). The sites:

- the page-aligned allocation in `hold` and its release in
  `release`;
- the two dereferences of the pointer in `Pinned::expose` and
  `expose_mut`;
- `mlock`/`munlock` through `libc` on Unix, behind the `pin-pages` feature;
- the `VirtualLock`/`VirtualUnlock` declarations and calls on Windows.

The crate's doc comment in `lib.rs` counts five blocks; the file has seven
`unsafe` sites because of the Windows pair.

## Secrets and wiping

- `Secret<T>`: owns a value, no `Debug`, `Clone` or serialisation, wiped on
  drop, compared in constant time.
- `Pinned<T>`: a secret on pages of its own, `mlock`ed, zeroized then
  unlocked on drop.
- `SessionKey`, `Sealed<T>`, `SeedBytes`, `MnemonicBytes`: `Zeroize` and
  `ZeroizeOnDrop`; the stack buffer in `Sealed::with` is zeroized when the
  closure returns.
- `blake2b` wipes its state and message block; `scrypt` zeroizes `ROMix`'s
  `V` (32 MiB at aezeed's parameters), its scratch and its block before
  they are freed.
