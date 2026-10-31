# aes

**Cryptography:** computes

**Purpose.** AES's round function (FIPS 197: SubBytes, ShiftRows,
MixColumns, AddRoundKey) for AEZ v5 (`core/faraday-bip/src/aez.rs`),
which LND's aezeed enciphers a cipher seed with. AEZ runs that round four
or ten times over keys derived from the seed's passphrase; it never runs
AES as a block cipher, so only `aes::hazmat::cipher_round` is called.

**Why this crate.** The round was written here over a 256-byte S-box
indexed by the state's bytes, which are the seed's: which cache line a
lookup reads depends on a secret. RustCrypto's `aes` runs the same round
with the processor's AES instructions (AES-NI, ARMv8's crypto extension)
where it has them and a fixsliced software round with no table where it
does not, and its documentation states every backend is designed to run
in constant time. It is the crate the RustCrypto AEADs build on, and the
round is checked here against AEZ's own vectors.

**Cost.** Version 0.9 with `default-features = false` and the `hazmat`
feature: one new crate in the lock file. It builds on `cipher` 0.5,
`cpufeatures` and `cpubits`, which the workspace's lock file already
holds (the main fuzz workspace's gains `cpubits`). Its `unsafe` is the intrinsics backends, entered only
after `cpufeatures` finds the instructions.

**Reopen when** a maintained Rust AEZ crate appears, or aezeed is
dropped.

**Review.** Trusted by publisher (github:RustCrypto/block-ciphers) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.
