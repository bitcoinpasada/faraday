# bitcoin

**Cryptography:** computes

**Purpose.** The Bitcoin data types and algorithms Faraday does not write
itself: BIP-32 key derivation and extended keys (`bitcoin::bip32`), PSBT
(BIP-174/370) parsing and serialisation (`bitcoin::psbt`), scripts,
opcodes, addresses and Base58/Bech32 encodings, taproot, sighash
computation, BIP-137 message signatures (`bitcoin::sign_message`), and
through it `secp256k1` (libsecp256k1 bindings) and `bitcoin_hashes`.
Direct dependants: `core/faraday-bip` (keys, accounts, addresses,
descriptors), `core/faraday-psbt` (reading, reviewing and signing a PSBT)
and `core/faraday-codec` (payload classification). `faraday-keys`
(`faraday/faraday-keys`) and `faraday-core` reach it through those crates.

**Where it ships.** It is in the signing core, so in every build that
hosts `faraday-core`: the stick and the Pi image (`faraday/shells/stick`),
the desktop app (`faraday/shells/desktop`) and the Starter
(`faraday/shells/starter`, through `faraday/faraday-starter`). It is not a
build-time tool.

**Why this crate.** BIP-32, PSBT, sighash and address encoding are
specifications where a mistake costs money: a wrong address shown, a wrong
digest signed. `rust-bitcoin` is the most widely used Rust implementation,
maintained by the same project as `rust-miniscript`, used in BDK and
other wallets, and tested against Bitcoin Core's own vectors; Bitcoin
Core, Liana, Sparrow and Nunchuk read what it writes. It is `no_std` with
`alloc`, which the stick and the Pi shells need, and it is the crate
`miniscript` (`docs/deps/miniscript.md`) is built on. Faraday still writes
the formats that are short and have their own vectors in-house (BC-UR, the
BIP-388 policy parser, the finalizer). Key material is held in
`faraday-keys` and leaves it only through its named reveals
(`docs/KEYS.md`); `bitcoin` does the arithmetic on it.

**Version and features.** `bitcoin = { version = "0.32",
default-features = false, features = ["secp-recovery"] }`.

- `default-features = false` drops `std` and `rand-std`: the crate is
  `no_std` + `alloc`.
- `secp-recovery` turns on libsecp256k1's public-key recovery and the
  `sign_message` signature type, which BIP-137 needs to check a message
  signature against an address (`faraday_psbt::message`). It is a feature
  of the same crate, not another dependency.
- The 0.32 line pairs with `miniscript` 12.x. The tree moves to 0.33 and
  `miniscript` 13 together.

**Review.** Trusted by publisher (apoelstra) under F71 (`docs/decisions/F71.md`), until 2027-10-01: `supply-chain/audits.toml`. Not read line by line here.

**Reopen when** the tree moves to `bitcoin` 0.33 (with `miniscript` 13),
when a cryptographic path in it is replaced by `core/` code, or when the
publisher trust entries of F71 are written and the exemptions go.
