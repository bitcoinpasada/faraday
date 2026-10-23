# faraday-bip

Keys, seeds, descriptors and the key-sharing schemes around them. `no_std` +
`alloc`. `bitcoin` supplies BIP-32 derivation, addresses and secp256k1;
`miniscript` supplies descriptors.

## What it computes

BIP-39 mnemonics and seeds (`bip39`), BIP-32 master and hardened keys
(`keys`), accounts and addresses (`account`, `address`), SLIP-132
(`slip132`), descriptors and policies (`descriptor`, `policy`, `compile`,
`tapmulti`, `recovery`), BIP-85 (`bip85`), SLIP-39 (`slip39`), codex32
(`codex32`), MuSig2 (`musig`), FROST (`frost`, `threshold`), silent payments
(`silent`, `silent_wallet`), LND's aezeed and the Lightning node keys
(`aezeed`), vanity grinding (`vanity`), BSMS records (`bsms`) and the
wordlists (`wordlists`, `diceware`).

## Written in this crate

| Module | What |
|---|---|
| `bip39` | BIP-39 mnemonics, checksums, seed derivation, last-word candidates |
| `slip39` | SLIP-39 Shamir backup with RS1024 |
| `codex32` | BIP 93 |
| `musig` | BIP-327 MuSig2, BIP-328 aggregate xpub, BIP-390 key expression |
| `frost` | BIP 445 FROST signing and the trusted-dealer split |
| `silent` | BIP-352 receiving side, BIP-392 key expression |
| `bip85` | BIP-85 over `HMAC-SHA512("bip-entropy-from-k", k)` |
| `aez`, `aezeed` | AEZ v5 (a port of the reference code) and LND's cipher seed; BLAKE2b and scrypt are in `faraday-crypto` |
| `vanity` | the counter that turns a key's passphrase and account until an address starts with chosen characters |
| `xkey` | strict extended-key decoding (`bitcoin` accepts seven of BIP-32 vector 5's invalid keys) |
| `base64`, `hangul`, `kana` | codec and input automata |

## Checked against

Vectors under `tools/vectors/`, tests under `tests/all/`:

- `bip39`: `bip39/vectors.json` and the bip32JP file (`vectors.rs`); every
  language and length against the `bip39` crate, a dev-dependency
  (`crosscheck.rs`).
- `slip39`: `slip39/vectors.json` (`slip39.rs`).
- `codex32`: `bip93/vectors.json` (`codex32.rs`); the worksheets and fresh
  sets against volvelle-website's cells (`codex32-volvelle/`), the codex32
  booklet's tables (`codex32-booklet/`) and the Codex32QR profile's vectors
  (`codex32qr/`) (`codex32_sheets.rs`).
- `musig`: `bip327/`, `bip328/`, `bip390/`, `bip32/`, `bip39/` (`musig.rs`).
- `frost`: `bip445/` and RFC 9591 appendix F.5 for the dealer (`frost.rs`);
  `tools/scripts/threshold-reference.py` replays a session against BIP 445's
  reference.
- `silent`: `bip352/send_and_receive_test_vectors.json` (`silent.rs`).
- `bip85`: `bip85/vectors.json` (`bip85.rs`).
- `aez`: the reference implementation's data in
  `tools/reference/aezeed/aez-testdata/` (`aez.rs`); node ids against LND and
  ldk-node in `faraday/faraday-core/tests/all/lightning.rs`.
- `vanity`: EntropyLab's output (`tools/reference/vanity/`, `vanity.rs`).
- `xkey`, `keys`: `bip32/invalid-keys.txt` (`bip32.rs`).
- `bsms`: `bip129/` (`bsms.rs`); `diceware`: `eff/` (`diceware.rs`).

## `unsafe`

None. The workspace sets `unsafe_code = "forbid"`.

## Secrets

Every module in the list below is in `tools/lint-secrets.sh`'s
`SECRET_FILES`: no printing, no `Debug` on secret types, no heap text.

- `keys.rs`: `MasterKey`, `DerivedKey`, `XprivAscii`, `WifAscii`; the secret
  key is overwritten with `non_secure_erase`. `vanity.rs`, `musig.rs`,
  `bip85.rs` and `silent.rs` (`Receiver`'s scan key) do the same.
- `bip39.rs`: `Mnemonic`, `Entropy`, `Typed`, `LastWordCandidates`.
- `slip39.rs`: `Share`, `SecretBytes`, `Points`.
- `frost.rs`: `SecShare`, `SecNonce`; `musig.rs`: `SecNonce`.
- `bip85.rs`: `Bytes`, `HexBytes`, `Password`.
- `aez.rs`, `aezeed.rs`, `codex32/`, `xkey.rs`, `base64.rs`: `Zeroizing`
  buffers for intermediate values.
