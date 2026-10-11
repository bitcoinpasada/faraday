# chacha20

**Purpose.** The stream cipher the KDBX 4 writer needs
(`core/osk-backup/src/kdbx.rs`, `docs/PLANNING.md` §16.112
pass E2). KDBX encrypts its body and its inner random stream with plain
ChaCha20 (RFC 8439), not with an AEAD: the authentication is KeePass's
own HMAC-SHA-256 over the header and over each block of ciphertext.
`chacha20poly1305`, which the tree already uses, cannot be asked for the
bare cipher.

**Why this crate.** It is the crate `chacha20poly1305` is built on — the
same code, the same maintainers, the same `cipher` 0.5 generation — and
it is already in the graph as that crate's dependency, at the version
the lock file already pins. It is not re-exported, so it has to be
named. Writing ChaCha20 by hand instead would be ~100 lines of
constant-time review to avoid a crate that is already compiled into
every build.

**Cost.** Version 0.10 with `default-features = false` and the `cipher`
and `zeroize` features: no new crate in the lock file, one new edge from
`opensigner-core`. No `alloc`, no RNG feature: the nonce and the key
come from the file's header and the session key. No `unsafe` outside the
optional SIMD paths, which are gated on target features.

**Reopen when** KDBX is dropped, or if `chacha20poly1305` ever exposes
its inner cipher, in which case the edge goes away and nothing else
changes.
