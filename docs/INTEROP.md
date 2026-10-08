# Interoperability with other wallets

**Status:** v0.2 · 2026-10-07 · what was tested, what passed, and what
Faraday does not read or write yet.

Faraday is a signer. Other wallets hand it a wallet description, a
cosigner's key or a PSBT, and take back a signed PSBT, a finished
transaction, a wallet description or an account key. This file lists
those files and codes for the major desktop, mobile and hardware wallets.

## 1. How it is tested

- **Bitcoin Core, end to end** (`faraday/tools/core-check.py`, not part of
  `just`; it starts `bitcoind` on regtest). For each of the ten test
  wallets (pkh, sh-wpkh, wpkh, tr, wsh, sh-wsh and sh multisig, Taproot
  `sortedmulti_a`, a wsh miniscript with a timelock, a Taproot tree):
  Core imports Faraday's `<wallet>-bitcoin-core.json`; Core's checksum of
  `<wallet>-descriptor.txt` equals Faraday's; the first three receive and
  change addresses agree; Core funds the wallet and builds a spend with
  `walletcreatefundedpsbt`; Faraday's own flow (load the wallet, add the
  seeds, open the PSBT, Sign) finishes it; Core accepts and mines it and
  sees the change as the wallet's. For each multisig, two Faraday
  sessions sign alone and Core combines the two signed PSBTs into a
  transaction it accepts. Core's own wallet export (`listdescriptors`)
  reads in Faraday as its BIP-84 wallet, with Core's first address.
  **Passed against Bitcoin Core 31.1.0, 2026-10-07: every check.**
- **Other wallets' files** (`faraday-core/tests/interop.rs`, in `just`):
  files from Sparrow's and BlueWallet's own test suites
  (`tests/vectors/interop/README.md`). Each must load as the wallet the
  other wallet holds; where the file states an address (Coldcard's
  `first`, a BIP 129 record's first address), Faraday's must be the same.

Nothing here is tested against Sparrow, Electrum, Nunchuk or BlueWallet
running; only against files they wrote. Hardware wallets are covered by
the files and codes they write.

## 2. What works

| From or to | Format | Faraday reads | Faraday writes | Tested |
|---|---|---|---|---|
| Bitcoin Core ≥ 29 | `importdescriptors` file, multipath | — | yes | Core, end to end |
| Bitcoin Core | PSBT in, finished transaction or signed PSBT out | yes | yes | Core, end to end |
| Sparrow, Liana, Nunchuk, BlueWallet, Keeper | Descriptor with `<0;1>` | yes | yes | Sparrow's and Coldcard Mk4's files |
| Coldcard, Sparrow, Electrum, Nunchuk, BlueWallet, Keystone, Jade, Keeper | Coldcard multisig setup `.txt` (one derivation, or one per key) | yes | yes | Coldcard's, Sparrow's and Electrum's files |
| Nunchuk, Sparrow, Coldcard, Keeper | BIP 129 descriptor record | yes, first address checked | yes | Nunchuk's and Sparrow's records |
| Nunchuk, Coldcard | BIP 129 signer record (a cosigner's key) | yes, signature checked | yes | `tests/bsms.rs` |
| Coldcard, Passport, Nunchuk, Sparrow | Coldcard's generic account export `.json`, as a single-sig wallet | yes (BIP-84, else 86, 49, 44), `first` checked | — | Coldcard Mk4, Coldcard Q, Nunchuk, Sparrow |
| Sparrow, Specter | Specter Desktop wallet `.json` with a `<0;1>` descriptor | yes | yes | Faraday's own read back; not against Specter |
| Sparrow, Keystone, SeedSigner, Jade, Passport | `ur:crypto-output`, `ur:crypto-account`, `ur:crypto-hdkey` | yes | `crypto-account` for a wsh cosigner key | Sparrow's multisig code, a Keystone/Cobo account code, SeedSigner's vector |
| Every coordinator | PSBT: binary, base64, `ur:crypto-psbt`, `ur:psbt`, BBQr `P` | yes | binary, `crypto-psbt`, BBQr `P` | BlueWallet's sample, Core |
| Coldcard, Sparrow | Finished transaction as hex (`-final.txn`) | yes | yes | Core |
| SeedSigner, Krux, Jade, Coldcard Q | SeedQR, CompactSeedQR | yes | yes (backup grid) | SeedQR's specification vectors (`osk-codec`), `tests/import.rs` |
| Trezor, Keystone | SLIP-39 shares | yes | yes | `tests/keygen_slip39.rs` |
| Coldcard | Seed XOR, BIP-85 | yes | yes | `tests/forms.rs`, `tests/bip85.rs` |
| Ledger, Trezor, BitBox02 | BIP-39 words | yes | — | — |
| Coldcard, Passport, Unchained | Cosigner key file (`ccxp-….json`, flat `p2wsh`/`p2sh_p2wsh`/`p2sh`, SLIP-132 keys) and the generic export's `bip48_1`, `bip48_2`, `bip45` accounts, in Create's key slots | yes, the account the kind needs | — | Coldcard's key file against its own setup file; Unchained's file against the same Coldcard's export |
| Sparrow, Bitcoin Core | A receive and a change descriptor, one per line or in `listdescriptors` | yes, as `<0;1>` | — | Sparrow's files; Core's `listdescriptors`, first address against Core's |
| Jade, Keystone | A receive descriptor alone (`/0/*`) | yes, as `<0;1>` | — | Jade's file, BIP-84's published address |
| Specter Desktop, Sparrow | Wallet `.json` whose keys carry no derivation | yes, as `<0;1>` | — | Specter's files; Sparrow's Specter and Coldcard exports of one wallet read as one |

Ledger, Trezor and BitBox02 sign over USB through a coordinator; they
reach Faraday as keys inside the wallet files above, which work.

## 3. Fixed while testing

- **Coldcard's account export read with the wrong fingerprint.** Each
  account object in the file carries an `xfp` that is the account key's
  own fingerprint; the master's is the file's top-level `xfp`
  (Coldcard's `docs/generic-wallet-export.md`). `osk_bip::coldcard::parse`
  roots the key at the inner one, so the wallet never matched the seed
  that made it. Before that, Faraday's reader took the file's first
  `desc`, which is BIP-44's, and loaded a legacy wallet. Faraday now reads
  the file itself (`wallet::coldcard_export`): BIP-84 first, rooted at the
  master, and refused if its first address is not the one the file
  states. The upstream function still has the fault; it is an upstream
  ask.

## 4. Not read or not written yet

Built 2026-10-07 from the first list: Coldcard-style cosigner key files,
and descriptors that are not multipath (§2's last four rows).

In the order they would matter to a person bringing files from those
wallets.

1. **SLIP-132 keys** (`zpub`, `Zpub`, `vpub`, `Ypub`) in a plain key
   file or inside a descriptor: Specter DIY's and BlueWallet's cosigner
   exports, Casa's descriptor, Keystone's export, Electrum's keys.
   Coldcard's JSON key files, which write them, are read.
2. **Caravan and Unchained wallet configs** (`extendedPublicKeys`,
   `quorum`, `addressType`): neither read nor written.
3. **Electrum.** Electrum takes no descriptor, BIP 129 record or Specter
   file, and treats a plain `xpub` as legacy. Faraday writes nothing
   Electrum can make a SegWit watch-only wallet from; an Electrum wallet
   file (what Sparrow exports for Electrum) would. Faraday also reads no
   Electrum wallet file. Whether Faraday signs a PSBT Electrum builds from
   a key without its master fingerprint is untested.
4. **Single-key JSON from Cobo/Keystone and Coldcard's Wasabi file**
   (`ExtPubKey`, `MasterFingerprint`, `AccountKeyPath`).
5. **Electrum's own seeds** (not BIP-39): an Electrum wallet's words do not
   load.
6. **Bitcoin Core before 29** takes no multipath descriptor; Faraday's
   import file is one multipath request.
7. **BIP 129 records** with an upper-case address or "No path
   restrictions" are refused by `osk_bip::bsms` (BIP 129's own examples
   include both).
8. Not read or written: Specter DIY's `addwallet name&descriptor` code,
   BIP-392 `sp()` silent payment descriptors (Sparrow, Keystone), BIP-329
   label files, PSBT version 2 (BIP-370; the Sign tile names BIP-370,
   which nothing here reads), and Coldcard's encrypted backup file.
