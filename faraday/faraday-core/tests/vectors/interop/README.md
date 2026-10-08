# Other wallets' files

Files other wallets write, taken from those wallets' own test suites on
2026-10-07, for `tests/interop.rs`. Public keys and addresses only; the
seeds behind them are the projects' published test seeds.

| Folder | From | Licence |
|---|---|---|
| `sparrow/` | `sparrowwallet/sparrow`, `src/test/resources/com/sparrowwallet/sparrow/io/` (Coldcard, Caravan, Cobo, Electrum, Jade, Keystone, Specter DIY and Specter Desktop files, and BIP 129 records) | Apache-2.0 |
| `bluewallet/` | `BlueWallet/BlueWallet`, `tests/unit/fixtures/` and the strings in `tests/unit/multisig-hd-wallet.test.js` (Coldcard Mk4 and Q, Nunchuk, Unchained, Caravan, Casa, Cobo, Electrum, and Sparrow's exports) | MIT |

Renamed on the way in: BlueWallet's `coldcardmk4/descriptor.txt`,
`new-wasabi.json` and `sparrow-export.json` are `coldcardmk4-descriptor.txt`,
`coldcardmk4-wasabi.json` and `coldcardmk4-generic.json` (the last is
Coldcard's generic export, which Sparrow also writes).
`casa-descriptor.txt`, `sparrow-crypto-output.txt` and
`cobo-crypto-account.txt` are strings from BlueWallet's test file.

Not every file here is one Faraday reads: `docs/INTEROP.md` lists which
are refused and why.
