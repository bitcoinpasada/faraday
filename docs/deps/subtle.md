# subtle

**Purpose.** Constant-time equality for `Secret<T>` (`docs/PLANNING.md`
§5.1), which is how the session PIN hash and the two PIN entries are
compared without a timing side channel.

**Why this crate.** `ConstantTimeEq` over byte slices with the same
volatile-and-fence technique `zeroize` uses to keep the optimiser from
short-circuiting the comparison. It is `no_std`, dependency-free, and
the comparison primitive `secp256k1`, `curve25519-dalek` and RustCrypto
all rely on.

**Cost.** One crate, no dependencies, ~600 lines, most of it `Choice`
plumbing.

**Reopen when** never, realistically; a hand-written `ct_eq` would be a
copy of the same eight lines without the review history.
