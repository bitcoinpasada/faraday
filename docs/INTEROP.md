# Interoperability with other wallets

**Kind:** Spec
**Status:** Tested; §2 lists what works; §5 I1 to I6, I10 and I11 built
**Open:** §5 I7 to I9: Electrum seed words, Specter DIY `addwallet`, reading BIP-392 `sp()`.

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
- **Other wallets' files** (`faraday-core/tests/all/interop.rs`, in `just`):
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
| Bitcoin Core before 29 | `importdescriptors` file, a receive and a change request (§5 I4) | — | `{stem}-bitcoin-core-before-29.json`, under More options; not MuSig2 or silent payments | Core's own `listdescriptors` strings come back, checksums and all; not yet run against Core 28 |
| Bitcoin Core | PSBT in, finished transaction or signed PSBT out | yes | yes | Core, end to end |
| Bitcoin Core (`dumpprivkey`), paper wallets | A WIF private key, swept (§5 I11) | yes, never kept | the swept PSBT and transaction | Core's `key_io` vectors; Core signs the same sweep in the cross-check |
| Sparrow, Liana, Nunchuk, BlueWallet, Keeper | Descriptor with `<0;1>` | yes | yes | Sparrow's and Coldcard Mk4's files |
| Coldcard, Sparrow, Electrum, Nunchuk, BlueWallet, Keystone, Jade, Keeper | Coldcard multisig setup `.txt` (one derivation, or one per key) | yes | yes | Coldcard's, Sparrow's and Electrum's files |
| Nunchuk, Sparrow, Coldcard, Keeper | BIP 129 descriptor record, also with an upper-case bech32 address or `No path restrictions` (§5 I3) | yes, first address checked | yes | Nunchuk's and Sparrow's records |
| Nunchuk, Coldcard | BIP 129 signer record (a cosigner's key) | yes, signature checked | yes | `faraday-core/tests/all/bsms.rs` |
| Coldcard, Passport, Nunchuk, Sparrow | Coldcard's generic account export `.json`, as a single-sig wallet | yes (BIP-84, else 86, 49, 44), `first` checked | — | Coldcard Mk4, Coldcard Q, Nunchuk, Sparrow |
| Sparrow, Specter | Specter Desktop wallet `.json` with a `<0;1>` descriptor | yes | yes | Faraday's own read back; not against Specter |
| Sparrow, Keystone, SeedSigner, Jade, Passport | `ur:crypto-output`, `ur:crypto-account`, `ur:crypto-hdkey` | yes | `crypto-account` for a wsh cosigner key | Sparrow's multisig code, a Keystone/Cobo account code, SeedSigner's vector |
| Every coordinator | PSBT: binary, base64, `ur:crypto-psbt`, `ur:psbt`, BBQr `P`; version 0 or version 2 (BIP-370, §5 I10) | yes | binary, `crypto-psbt`, BBQr `P`, in the version read | BlueWallet's sample, Core; version 2: BIP-370's vectors and HWI |
| Coldcard, Sparrow | Finished transaction as hex (`-final.txn`) | yes | yes | Core |
| SeedSigner, Krux, Jade, Coldcard Q | SeedQR, CompactSeedQR | yes | yes (backup grid) | SeedQR's specification vectors (`faraday-codec`), `tests/all/import.rs` |
| Trezor, Keystone | SLIP-39 shares | yes: typed, the SLIP-39 form's Scan, Home's Scan, What do you have? (F167) | yes | `tests/all/keygen_slip39.rs`, `faraday/faraday-core/tests/all/home_scan.rs` |
| Coldcard | Seed XOR, BIP-85 | yes | yes | `tests/all/forms.rs`, `tests/all/bip85.rs` |
| SeedSigner (codex32 branch), any Codex32QR/v1-48 writer | Codex32QR/v1-48: one 48-character codex32 string, upper case, QR version 3, level L (`docs/CODEX32.md` §10) | yes: the codex32 form's Scan, Add a key's Scan a SeedQR, Home's Scan, a picture in Files | yes, String · Codex32QR on Make codex32 shares' strings | every picture of the profile's vectors read to its stated fingerprint (`faraday/faraday-core/tests/all/codex32_forms.rs`); BIP 93's vectors (`core/faraday-bip/tests/all/codex32.rs`) |
| Ledger, Trezor, BitBox02 | BIP-39 words | yes | — | — |
| Coldcard, Passport, Unchained | Cosigner key file (`ccxp-….json`, flat `p2wsh`/`p2sh_p2wsh`/`p2sh`, SLIP-132 keys) and the generic export's `bip48_1`, `bip48_2`, `bip45` accounts, in Create's key slots | yes, the account the kind needs | — | Coldcard's key file against its own setup file; Unchained's file against the same Coldcard's export |
| Sparrow, Bitcoin Core | A receive and a change descriptor, one per line or in `listdescriptors` | yes, as `<0;1>` | — | Sparrow's files; Core's `listdescriptors`, first address against Core's |
| Jade, Keystone | A receive descriptor alone (`/0/*`) | yes, as `<0;1>` | — | Jade's file, BIP-84's published address |
| Specter Desktop, Sparrow | Wallet `.json` whose keys carry no derivation | yes, as `<0;1>` | — | Specter's files; Sparrow's Specter and Coldcard exports of one wallet read as one |
| Keystone | A change descriptor alone (`/1/*`) | yes, as `<0;1>` | — | Keystone's file |
| Casa, Specter DIY, BlueWallet, Keystone, Electrum | SLIP-132 keys (`ypub`, `zpub`, `Ypub`, `Zpub`, `upub`, `vpub`, `Upub`, `Vpub`) in a descriptor, on a key line or bare, checked against the script or path (§5 I1) | yes; a `zpub`/`vpub` or `ypub`/`upub` alone as its single-key wallet | — | Casa's descriptor, Specter DIY's keystore, Keystone's, BIP-84's account as a zpub at its published first address |
| Cobo, Keystone | Single-key `.json` (`ExtPubKey`, `MasterFingerprint`, `AccountKeyPath`), the version checked against the path (§5 I2) | yes, also in Create's key slots | — | Sparrow's Cobo file at BIP-84's first address; BlueWallet's Cobo file as Keystone's export of the same key |
| Coldcard, Wasabi | Coldcard's Wasabi file (`ColdCardFirmwareVersion`), as BIP-84 account 0 (§5 I2) | yes | `{stem}-wasabi.json`, BIP-84 account 0 only | Coldcard Mk4's file against its generic export's `first`; Faraday's read back |
| Electrum 4.x, Sparrow | Electrum wallet file, unencrypted: one key (`xpub`/`ypub`/`zpub` → `pkh`/`sh(wpkh)`/`wpkh`) or `NofM` `sortedmulti` (`xpub`/`Ypub`/`Zpub` → `sh`/`sh(wsh)`/`wsh`), fingerprint from `root_fingerprint` or `ckcc_xfp` little-endian (§5 I6) | yes; a multisig keystore with no fingerprint and derivation refused; a file with an `xprv`, a `seed` or imported keys is a secret; an encrypted file refused | `{stem}-electrum.json` under More options, keys in their SLIP-132 version, `root_fingerprint` and derivation | Sparrow's two files (its multisig one equals the same Coldcards' setup file); BlueWallet's two; BIP-84's account at its published first address; Faraday's read back |
| Caravan, Unchained, Sparrow, BlueWallet | Caravan wallet config, `sortedmulti` (§5 I5) | yes; a key with no `bip32Path` or `xfp` refused | `{stem}-caravan.json`, `sortedmulti` with every origin | Sparrow's exports (its two refused ones refused); BlueWallet's at its first address once its third key's path is given; Faraday's read back |

Ledger, Trezor and BitBox02 sign over USB through a coordinator; they
reach Faraday as keys inside the wallet files above, which work.

## 3. Fixed while testing

Coldcard's account export is read by `faraday_bip::coldcard::read`
(`parse` uses it; `cosigner_keys` for Create's slots): each account
object's `xfp` is the account key's own fingerprint and the master's is
the file's top-level `xfp` (Coldcard's docs/generic-wallet-export.md), so
the wallet is rooted at the top-level one, BIP-84 first, and refused if
its first address is not the one the file states. `wallet::coldcard_export`
only words its refusals. The bug this fixed: moved to
`docs/history/INTEROP-LOG.md`.

## 4. Not read or not written yet

In the order they would matter to a person bringing files from those
wallets. Items 1 to 4, 6 and 7 are built (§2, §5 I1 to I6); what this
list said of them: moved to `docs/history/INTEROP-LOG.md`.

5. **Electrum's own seeds** (not BIP-39): an Electrum wallet's words do not
   load (§5 I7).
8. Not read or written: Specter DIY's `addwallet name&descriptor` code
   (§5 I8), BIP-392 `sp()` silent payment descriptors (Sparrow, Keystone;
   §5 I9), BIP-329 label files, and Coldcard's encrypted backup file.
   PSBT version 2 is read and written (§5 I10). A single WIF private key is swept, never loaded (§5 I11).

## 5. The batches that build §4 (F64, owner 2026-10-20)

Every reader goes in `faraday-intake`, after the pattern of
`caravan` and `electrum`, except where `core/`'s crates also read
the format: such a reader goes in `core/`. New strings go in
`faraday/faraday-strings/src/en.rs`. New vectors go under
`faraday/faraday-core/tests/vectors/interop/<source>/`, with their source
and licence in that folder's README. Reading needs no choice: every format
is recognised by its content wherever a wallet, a key or a PSBT is read
today. Choices few people need sit under **More options** (F64).

### I1. SLIP-132 keys everywhere (§4 item 1)

Reads extended keys with SLIP-0132's version bytes:

| Prefix | Version bytes | Network | Script type it names |
|---|---|---|---|
| `ypub` / `upub` | `049d7cb2` / `044a5262` | main / test | `sh(wpkh)` |
| `zpub` / `vpub` | `04b24746` / `045f1cf6` | main / test | `wpkh` |
| `Ypub` / `Upub` | `0295b43f` / `024289ef` | main / test | `sh(wsh(multi…))` |
| `Zpub` / `Vpub` | `02aa7ed3` / `02575483` | main / test | `wsh(multi…)` |
| `xpub` / `tpub` | — | main / test | any |

inside a descriptor (Casa's `sh(wsh(sortedmulti(3,[35282ae3]Ypub…`), on a
key line `[fp/path]vpub…` (Specter DIY), and as a bare key; each is
rewritten to `xpub`/`tpub` before `WalletPolicy` sees it.

**The rule.** The type the version names must agree with the script
around the key, and in a key file with the purpose in its path: 84′ →
`z`/`v`; 49′ → `y`/`u`; 48′/…/2′ → `Z`/`V`; 48′/…/1′ → `Y`/`U`; 44′, 45′,
86′ and 352′ → `x`/`t` only. A key that disagrees is refused
(`wallet_slip132_mismatch`, "{} is a {} key; this wallet is {}"); nothing
is converted silently. This replaced `wallet::plain_xpub`, which
converted Coldcard's flat fields unchecked until 2026-10-20. A key line or bare key with no
descriptor reads as the single-key wallet its version names; as a
cosigner key, `key_for` takes it.

Where: `faraday_bip::slip132` (`decode`, `key_line`, `plain_descriptor`,
`encode`; moved from `faraday-intake/src/slip132.rs` 2026-10-20, the
refusal's sentence staying in `faraday_intake::wallet::mismatch_text`),
called from `read_wallet_as_written` before `parse_any`,
`create::read_keys`, Coldcard's files and `faraday_bip::multisig_config`, which
since 2026-10-20 refuses a key whose version disagrees with its `Format:`
itself (so a rebuilt backup config is checked too, `restore::readable`). Tests (`tests/all/interop.rs`): Casa's descriptor loads as
`sh(wsh(sortedmulti(3,…)))`; Specter DIY's keystore fills a native SegWit
slot; Keystone's singlesig keystore (upper-case fingerprint, `/1/*` only)
reads as `<0;1>`; `wpkh(Zpub…)` and `wsh(multi(…Ypub…))` are refused; the
`abandon…about` BIP-84 vector given as a zpub has BIP-84's published first
address.

### I2. A JSON reader; single-key JSON from Cobo/Keystone and Wasabi (§4 item 4; #46)

`faraday_bip::json` (`core/faraday-bip/src/json.rs`, moved from `faraday-intake`
2026-10-20): a value parser (depth ≤ 32, size ≤ 1 MiB, full string
escapes including `\uXXXX`, a key named twice refused), no new crate,
and a fuzz target `fuzz/fuzz_targets/json.rs`. Every reader of a JSON
file uses it: since 2026-10-20 also the wallet `.json` and
`listdescriptors` reader, the `"label"` a wallet `.json` names, and
Coldcard's files, whose scanners (`json_string`, `json_object`,
`faraday_bip::coldcard`'s own) are gone.

Reads Cobo/Keystone `{"ExtPubKey","MasterFingerprint","AccountKeyPath"}`
(`AccountKeyPath` may lack `m/`; the version is checked against the path
as in I1) as the single-key wallet `[fp/path]xpub/<0;1>/*`, also offered as
a key in Create's slots; and Coldcard's Wasabi file
`{"MasterFingerprint","ExtPubKey","ColdCardFirmwareVersion"}`, which
names no path: Wasabi assumes BIP-84 account 0, so it reads as
`wpkh([fp/84h/{0h|1h}/0h]…)`, coin type from the key's network.

Writes `{stem}-wasabi.json` under More options, only for a single-key
`wpkh` at exactly `84h/0h/0h` or `84h/1h/0h`. The Cobo/Keystone file is not written: it is a
device's export of its own key, and no coordinator takes it from a third
party. Wasabi's reader (`WalletWasabi/Helpers/ImportWalletHelper.cs`,
`GetKeyManagerByColdcardJson`, checked 2026-10-20) takes a file of at
most three fields as Coldcard's, and reverses the fingerprint's bytes
when `ColdCardFirmwareVersion` is missing or `2.1.0`; any other version
must parse as a .NET `Version`. Faraday reads the fingerprint as Wasabi
does and writes `5.2.2`, Coldcard Mk4's in BlueWallet's vector. Tests: Cobo's singlesig keystore (73C5DA0A) has BIP-84's first
address for `abandon…about`; `skeleton-cobo.txt` loads; Coldcard MK4's
Wasabi file is the BIP-84 account of its generic file, with the generic
file's `first`; Faraday's Wasabi file reads back as the same wallet; JSON
past the depth limit is refused.

### I3. BIP 129 records Faraday refuses (§4 item 7; #50)

In `faraday_bip::bsms::DescriptorRecord::parse` (since 2026-10-20; before
that a tidy-up in `faraday-intake/src/wallet.rs` ran first): (a) the
stated address is compared as its encoding writes it
(`assume_checked().to_string()`), so an all upper-case bech32/bech32m
address (`BC1…`, `TB1…`, `BCRT1…`) is the same address in lower case,
and a base58 address in another case does not parse; (b) a `/**`
template whose third line is exactly `No path restrictions` reads as
`/0/*,/1/*`. The network check and the first-address check are
unchanged. Not read: Sparrow's
fixed-path records (one address, not a wallet), a blank paths line
(cannot be told from a cut-off file), encrypted records. Tests:
`multisig-4.bsms` and `multisig-6.bsms` load with Sparrow's stated first
address; Sparrow's refused records (`-5`, `-7`, `-9`, `-11`, `-13`, `-14`,
`-15`, `-17`) stay refused; a base58 address with its case changed is
refused.

### I4. More options on Public files; Bitcoin Core before 29 (§4 item 6; #48)

`screens/backup.rs::public_files` splits its rows into primary and more;
`public_rows_for` draws the primary rows, a ghost **More options**
(`Action::PublicMore(i)`), and the rest once pressed; the open state lives
on the backup flow and the chart sheet and resets when the screen opens.
Primary: Descriptor; Wallet file (Sparrow, Specter); Multisig config;
Backup sheet; BSMS record; Bitcoin Core 29 or later; the cosigners' keys. More options: Bitcoin Core before 29,
Electrum wallet file, Caravan config, Specter DIY, Wasabi file (numbers 9
onward in `backup_public_for`/`public_out`).

Writes `{stem}-bitcoin-core-before-29.json`: two `importdescriptors`
requests, receive (`"internal":false`) and change (`"internal":true`),
each from `Descriptor::into_single_descriptors()` with its own checksum,
`"active":true`, `"range":[0,999]`, `"timestamp":0`. Not for MuSig2
or silent payments. Written by `faraday_bip::core_import::import_descriptors_pair`
(moved from `faraday-intake/src/core_before_29.rs` 2026-10-20). Tests: a fresh Public files list shows no row under
More options until it is pressed; the file's two descriptors read back as
the wallet; each checksum is Core's (shape of `faraday/faraday-core/tests/vectors/interop/core/listdescriptors.json`);
`core-check.py` (not in `just`) is to gain a run against Core 28.x.

The rows read **Core 29 or later** and **Core before 29**, each "Bitcoin
Core, watch-only" under it. The More options state is
`BackupState::public_more`, open when a file under it is already For the
stick. A `sortedmulti_a` pair reads back as its wallet, as every other
pair does: `miniscript` has no `sortedmulti_a`, so each line is first read
as its `multi_a` twin, its checksum checked over the line as written
(`faraday_bip::descriptor::reads_as_descriptor`). `core-check.py` against Core
28.x is not done. The build's notes: moved to `docs/history/INTEROP-LOG.md`.

### I5. Caravan and Unchained configs (§4 item 2; #44)

Reads and writes
`{"name","addressType":"P2WSH"|"P2SH-P2WSH"|"P2SH","network","client":{"type":"public"},"quorum":{"requiredSigners","totalSigners"},"extendedPublicKeys":[{"name","bip32Path","xpub","xfp","method"?}],"startingAddressIndex":0}`
(Unchained's config is this format; extra fields are ignored). Read as
`sortedmulti` (BIP 67), written only for `sortedmulti` `wsh`/`sh(wsh)`/`sh`
wallets whose keys all have origins. A key with `"bip32Path":"Unknown"` or
no `xfp` is refused (a multisig key with no origin is not held); a key
whose network disagrees with `network` is refused; `startingAddressIndex`
other than 0 is refused. Writes `{stem}-caravan.json` under More options.
Tests: Sparrow's three Caravan exports load and equal its own exports or
the descriptor written by hand; BlueWallet's Caravan file loads at its
first address once its third key's path is given, and as written is
refused; Faraday's file reads back with the same first address.

### I6. Electrum wallet files (§4 item 3; #45)

Reads an unencrypted Electrum 4.x file: standard
`{"keystore":{"type":"bip32"|"hardware","xpub","derivation","root_fingerprint"|"ckcc_xfp"},"wallet_type":"standard"}`
and multisig `{"wallet_type":"2of3","x1/":{…},…}`. Script type from each
key's version (I1): single key `xpub`→`pkh`, `ypub`→`sh(wpkh)`,
`zpub`→`wpkh`; multisig `xpub`→`sh`, `Ypub`→`sh(wsh)`, `Zpub`→`wsh`, always
`sortedmulti`, every key the same type. Fingerprint from `root_fingerprint`
(hex) or `ckcc_xfp`, a decimal u32 read little-endian (1130956047 →
`0f056943`). A keystore without fingerprint and derivation is read
origin-less only for a single key, which the wallet policy holds; a
multisig one is refused with a line naming the key, as Caravan's is (I5).
`"use_encryption": true` or an encrypted blob is refused
("This Electrum file is encrypted"). A file holding `xprv` or `seed` is a
secret (`FileKind::SecretWallet`): `classify` gives it the secret sheet,
Files and the Inbox offer to load it like a wallet, and only its public
keys are read.

Writes `{stem}-electrum.json` under More options as Sparrow writes for
Electrum (`"seed_version": 17`, `"use_encryption": false`, keystores
`{"type":"bip32","xpub":<SLIP-132 for the type>,"xprv":null,"derivation","root_fingerprint","label":""}`),
for `pkh`, `sh(wpkh)`, `wpkh` single keys with an origin and `sortedmulti`
`sh`/`sh(wsh)`/`wsh`; not for Taproot, miniscript, unsorted `multi`,
MuSig2 or silent payments. A PSBT Electrum builds from a bare zpub
carries the account key's own fingerprint; Faraday's matching is
unchanged and a test states the refusal (Faraday's own Electrum file
carries `root_fingerprint`, so Electrum's PSBTs from it are complete).
Tests: Sparrow's Electrum singlesig (`sh(wpkh)` at 49′, ab543c67) and
multisig (2-of-4 `sh(wsh)`, `ckcc_xfp` fingerprints) files; BlueWallet's
Electrum skeletons; `fromsparrow-electrum.json` (holds an `xprv`) is
secret and its wallet equals `fromsparrow-specter.json`'s; write then read
is the same wallet; the bare-zpub PSBT is refused. Optional, not in
`just`: faraday/tools/electrum-check.py compares addresses with
`electrum --offline`.

Built in `faraday-intake/src/electrum.rs`. Accepted keystore types are
`bip32` and `hardware`; `wallet_type` `standard` or `NofM`; anything else
is refused by name. The whole-file encryption recognised is the base64 of
Electrum's `BIE1` envelope. Not done: `electrum-check.py`. The build's
notes: moved to `docs/history/INTEROP-LOG.md`.

### I7. Electrum seed words (§4 item 5; #47)

Add a key › Other forms › **Type Electrum seed words** (and the Entry
form's other forms): 12 or 13 English words, Electrum 2.0 and later.
Normalised as Electrum does (lower-case, whitespace collapsed). The seed
version is the hex of HMAC-SHA512(key "Seed version", words): prefix `01`
standard, `100` segwit; `101`/`102` (2FA) and pre-2.0 seeds are refused
with their own lines; anything else "These words are not an Electrum
seed". The seed is PBKDF2-HMAC-SHA512(words, "electrum" + extension, 2048
rounds, 64 bytes); the key loads through `Session::add_seed`. Paths:
standard `pkh([fp]xpub/<0;1>/*)` at the master; segwit `wpkh([fp/0h]…)`;
as a multisig cosigner (from an Electrum file, I6) segwit `m/1h` (`wsh`),
standard `m/` (`sh`). After Lock in, the Key card offers **Load its
Electrum wallet**. The key is never treated as BIP-39 words: `Key` gains
`electrum: Option<ElectrumType>`; its Backup menu offers the words by hand
and the vault or encrypted backup of the master seed, never SeedQR,
CompactSeedQR, Seed XOR or SLIP-39; in a vault it is a
master-seed record. Tests: Electrum's `test_mnemonic.py` English seed
cases (seed version and seed bytes, with and without an extension);
`test_wallet_vertical.py` standard and segwit (xpub/zpub, first receive
and change addresses); a BIP-39 phrase that is no Electrum seed is
refused here; an Electrum key's Backup menu has no SeedQR; an extension in
capitals gives the same key.

### I8. Specter DIY `addwallet` (§4 item 8; #49)

Reads `addwallet {name}&{descriptor}` from a scan or a file (keys without
`/<0;1>/*`, which `with_chains` reads). Writes the same text as Show as
QR, PNG (Specter's numbered parts) and `{stem}-specter-diy.txt`, under
More options, for every wallet with a descriptor but MuSig2 and
silent payments. Tests: Sparrow's `specter-diy-export.txt` loads, named
"Sparrow Multisig"; Faraday's code for it is that text; scanned parts
reassemble to it.

### I9. BIP-392 `sp()` (§4 item 8; #49)

Reads `sp([fp/352h/{coin}h/{acct}h]spscan1q…)` and `tspscan` (Keystone,
Sparrow), a bare `[origin]spscan1q…` (Specter DIY), and Coldcard's export
`bip352` object rooted at its top-level `xfp`, each as an `faraday-silent 1`
record. The scan private key is never kept; a file without an origin is
refused ("This sp() descriptor names no key origin"); with the key
loaded, a record whose keys are not the ones it derives is refused.
Writes the descriptor under a silent payments wallet's Export › More
options, through the secret sheet. Where: a public `spscan` decoder in
`faraday-intake` (`decode_v0` is private). Tests: Sparrow's
Keystone, Specter DIY and Coldcard sp keystores give one wallet rooted at
0f056943; BIP-392's and BIP-352's keys round-trip.

### I10. PSBT version 2 (§4 item 8; #49)

Built. `bitcoin` 0.32 reads version 0 only, so Faraday converts, in
`core/faraday-psbt/src/v2.rs` on raw BIP-174 maps (`faraday_psbt::v2`;
in `core/` since its signer reads them, not in `faraday-intake`).
`PsbtV2::to_v0` builds the unsigned transaction (globals `0x02` version,
`0x03` fallback locktime, `0x04`/`0x05` counts; inputs `0x0e` txid,
`0x0f` index, `0x10` sequence, absent = `0xffffffff`; outputs `0x03`
amount, `0x04` script), the locktime by BIP-370's determination from
`0x11`/`0x12` (an incompatible set is refused), drops the v2-only keys
and `0xfb`, and keeps every other key. Back to v2, `PsbtV2::merge_signed`
writes the original pairs plus only the per-input fields signing added
(`0x02`, `0x13`/`0x14`, MuSig2 nonces and partial signatures `0x1b`/`0x1c`,
`0x07`/`0x08` when finalised). `PsbtV2::from_v0` writes the version 2
PSBT of a version 0 one, as HWI's `convert_to_v2` does, which `to_v0`
turns back into the same bytes.

In the app, `wallet::read_psbt` reads version 2 wherever it reads a PSBT
(Files, Home's Scan, a pasted PSBT, Decode, combining another signer's
copy), as its version 0 form; `wallet::read_psbt_kept` keeps the
original, read, on `Spend` (`v2: Option<PsbtV2>`, `wallet/v2.rs`), and every outward PSBT
(the signed PSBT, the part for the next signer, their QR codes) goes
through `Spend::out_bytes()`, version 2 again; where `merge_signed`
refuses, nothing is written or shown (never the version 0 form) and a
toast says so. A version 2 PSBT with an
output that has no script yet (a silent payment's, BIP-375) has no
transaction to sign and is not read as one. An empty `PSBT_OUT_SCRIPT`
is the empty script, as BIP-370 has it, and turns into version 0 and
back as one; only on a silent payment output is it a script not yet
given, as BIP-375's reference reads it (`v2::script_given`).

Tests: BIP-370's valid vectors read in Files, binary and base64, and its
invalid ones are refused; its locktime cases give their locktime, and
the incompatible set is refused (`faraday-core/tests/all/psbt_v2.rs`);
every testkit PSBT to v2 and back is byte-identical, and so is every
BIP-174 version 0 vector (`faraday-psbt/tests/all/bips.rs`); v2 and v0
of one transaction sign identically, and the v2 output without the
signature fields equals the input; a signing pass brings back only the
signing fields; a spend whose version 2 PSBT is of another transaction
writes nothing. `psbt_parse` fuzzing reads v2, signs its v0 form and
makes v0 into v2; `psbt_v2` fuzzes the v2 reader alone. The cross-check (`tools/crosscheck/area_psbt.py`)
holds every version 0 PSBT's v2 form against HWI's `convert_to_v2`,
map by map, and its v0 again against the bytes read; version 2 reading
and `to_v0` were held against HWI and Core already.

### I11. A single private key: sweep it (owner, 2026-10-20; F171, 2026-10-29)

Built as F171 has it: a WIF is swept and never kept. The plan of
2026-10-20 (the key loaded as a key and a wallet, kept in a vault):
moved to `docs/history/INTEROP-LOG.md`.

- **Reading.** `faraday_bip::wif::SingleKey::decode` reads a WIF as
  Bitcoin Core's `DecodeSecret` does: base58check, a version of `0x80`
  (`K…`, `L…`, `5…`) or `0xef` (`c…`, `9…`), 32 bytes of key then
  nothing (uncompressed) or exactly `0x01` (compressed), a scalar in
  `1..n`. The key is held on a pinned page in a signing context blinded
  from the key, and again from the session's randomness; it pays P2PKH
  and, compressed, P2WPKH and P2SH-P2WPKH. The session's key store holds
  at most one (`Keys::sweep_read`, `faraday-keys`' `sweep`), refused while
  a stick is attached and for the other kind of network: Faraday Demo
  takes test-network keys and refuses mainnet ones, the device on Mainnet
  the reverse. Practice sweeps none.
- **The Sweep sheet** (`faraday-core`'s `seeds/sweep.rs`). Tools' tile
  **Sweep a private key** (found by "private key", "WIF", "sweep",
  "paper wallet") opens it on a field the key is typed into, shown as
  dots, with **Scan**; Home's Scan opens it on a WIF it reads; its own
  scanner takes the key, and on the desktop the key pasted (F75). Every
  other scanner refuses a WIF ("That is a private key. Sweep a private
  key, in Tools, takes it") and files nothing. With the key read, the
  sheet names it by its legacy address, says it is not kept, and shows
  each address it pays with its QR code (legacy, native SegWit, nested
  SegWit; legacy alone and "Uncompressed key: legacy addresses only" for
  an uncompressed key), for an online wallet to watch.
- **The sweep.** That wallet builds the PSBT; **Scan the PSBT** reads it
  (in parts too). The review, worked out once as it is read, shows the
  key's inputs (their addresses and amounts), the inputs of other keys
  (not signed), each output with the loaded wallet whose address it is
  among its first hundred on each chain, or flagged "Not a loaded
  wallet's address", the fee, and any blocked or danger warning. An
  input is the key's when the output it spends pays one of the key's
  scripts: the key has no BIP-32 origin to match. **Hold to sign**
  signs the key's inputs (`faraday_psbt::sweep::sign`): a block, or a
  danger with no hold, refuses, but for no loaded key taking part; a
  holdable one is held past by the hold (F136); every input is checked
  before the first signature; a legacy input needs its previous
  transaction. The signed PSBT (`sweep-signed.psbt`) and, every input
  signed, the finished transaction (`sweep-final.txn`) go to the Outbox,
  and **Show as QR** shows the transaction, or the PSBT where it is not
  finished. The key leaves only as
  signatures (no reveal is added, KEYS §2) and is erased at **Done**, a
  lock, or leaving the sheet (its own scanner aside).
- **`OP_CODESEPARATOR`.** Faraday refuses to sign a legacy input whose
  script code holds `OP_CODESEPARATOR`, in every legacy spend and not
  only a sweep's, before anything is signed: "A legacy input with
  OP_CODESEPARATOR is not signed" (owner, 2026-10-24). The `bitcoin`
  crate's legacy signature hash takes the script code as given and
  Bitcoin Core's removes every `OP_CODESEPARATOR` first
  (`tools/crosscheck/README.md`, Known differences). A P2PKH or P2SH
  multisig script code holds none, so no spend Faraday made before is
  refused.
- **Tests.** Core's `key_io_valid.json` and `key_io_invalid.json`
  (`tools/vectors/wif`): every private key reads to its bytes, form and
  chain, nothing else reads as one; each rule refused alone; the
  generator's published addresses; BIP-143's P2SH-P2WPKH example swept
  to its published signature and transaction; a sweep of the key's three
  output kinds beside another key's, an uncompressed key's legacy one,
  a PSBT of another key signing nothing; a legacy input with
  `OP_CODESEPARATOR` refusing the PSBT with nothing signed; and in the
  app the sheet from Tools, Home's Scan, a paste, Files' refusal, the
  other network's key, the review naming the wallet and flagging an
  output, the hold, the Outbox, and the key erased at Done and on
  leaving. The cross-check's `sign` area sweeps random WIF keys' outputs
  funded on Core's node: Core signs the same PSBT from the WIF's
  descriptors, byte for byte, finalizes it and accepts it to its mempool.
- **Not built** (F171): a WIF loaded as a key, a single-key vault record,
  a Taproot address for the key, and a descriptor with a private key
  inside (`wpkh(L…)`), which is not read as a sweep.

### Left out (owner, 2026-10-20)

- **BIP-329 label files.** A label from the coordinator beside an output
  in the signing review is text an attacker controls ("Savings" beside
  their address). Labels belong in the online wallet.
- **Coldcard's encrypted backup** (7z, AES-256-CBC): a new AES crate and a
  7z parser, to save typing a 12-word password no easier than the seed
  words that already load.
- **Electrum pre-2.0 and 2FA seeds**, Sparrow's fixed-path BSMS records,
  the Cobo/Keystone single-key writer, and a longer Back up software
  question (its `software` field is five bits).

### What these batches change in `core/`

`faraday_bip::slip132` multisig versions and a version/script check;
`faraday_bip::bsms` (#50); `faraday_bip::coldcard::parse` rooting at the top-level
`xfp`; `faraday-psbt` reading and writing BIP-370 (built, I10); `faraday_bip::silent` public
`spscan`/`sp()` decoding; `faraday_bip::core_import` before 29;
`faraday_bip::slip132` writing a key in the version that names its script
(the Electrum file, §5 I6); a receive and a change `sortedmulti_a`
descriptor read as one wallet, which `miniscript` cannot parse (§5 I4);
`faraday_bip::wif` and `faraday_psbt::sweep`, a WIF key read and its
sweep signed, and the refusal of a legacy `OP_CODESEPARATOR` (§5 I11).
`faraday-psbt` matching a key by its account xpub's own fingerprint stays
refused: BIP-174's fingerprint is the master key's.
