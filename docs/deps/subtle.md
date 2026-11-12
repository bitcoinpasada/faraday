# subtle

**Purpose.** Constant-time equality for `Secret<T>`, which is how the session PIN hash and the two PIN entries are
compared without a timing side channel, and the 4-byte hash check of a key file in `faraday-vault`.
It is also used by `aqrv`, `faraday-bip`, `faraday-crypto`, `faraday-core`, `faraday-kef`, `faraday-keys` and
`faraday-pgp` for constant-time compares and masked selects of secret bytes.

**Why this crate.** `ConstantTimeEq` over byte slices with the same
volatile-and-fence technique `zeroize` uses to keep the optimiser from
short-circuiting the comparison. It is `no_std`, dependency-free, and
the comparison primitive `secp256k1`, `curve25519-dalek` and RustCrypto
all rely on.

**Cost.** One crate, no dependencies, ~600 lines, most of it `Choice`
plumbing.

**Reopen when** never, realistically; a hand-written `ct_eq` would be a
copy of the same eight lines without the review history.

**Review.** Covered by audits imported from other organisations (`supply-chain/imports.lock`); not read here.