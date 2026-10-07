# zeroize

**Purpose.** Wiping secrets when they are dropped (`docs/PLANNING.md`
§5.1, §5.3): `Secret<T>`, `Sealed<T>`, `SessionKey`, `Mnemonic`, the
wizards' typed state and every `Zeroizing` buffer in the core. It is also
the single dependency of `osk-shell-api`, where `EntropyBytes` wipes the
32 bytes a shell answered a session's entropy request with (security
review 2026-09-11, L4).

**Why this crate.** Its `zeroize()` is a volatile write followed by a
compiler fence, which is what stops the optimiser from removing a store
to memory that is never read again; a plain `fill(0)` is not enough. It
is the de facto standard (used by rust-bitcoin's `secp256k1` bindings,
RustCrypto, and most Rust wallets), `no_std`, and its `derive` feature
is unused here: every `Zeroize` impl in this repository is written out,
so the audit surface is the ~400-line crate and nothing generated.

**Cost.** One crate, no dependencies with `default-features = false`;
the `alloc` feature (for `Zeroizing<Vec<u8>>`) in the crates that
allocate.

**Reopen when** the standard library grows an equivalent, or if a target
needs `mlock`-style page pinning, which is out of this crate's scope.
