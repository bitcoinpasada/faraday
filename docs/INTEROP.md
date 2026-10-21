# Interoperability with other wallets

**Status:** v0.3 · 2026-10-20 · what was tested, what passed, what
Faraday does not read or write yet, and the batches that build §4 (§5,
F64).

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
| Nunchuk, Sparrow, Coldcard, Keeper | BIP 129 descriptor record, also with an upper-case bech32 address or `No path restrictions` (§5 I3) | yes, first address checked | yes | Nunchuk's and Sparrow's records |
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
| Keystone | A change descriptor alone (`/1/*`) | yes, as `<0;1>` | — | Keystone's file |
| Casa, Specter DIY, BlueWallet, Keystone, Electrum | SLIP-132 keys (`ypub`, `zpub`, `Ypub`, `Zpub`, `upub`, `vpub`, `Upub`, `Vpub`) in a descriptor, on a key line or bare, checked against the script or path (§5 I1) | yes; a `zpub`/`vpub` or `ypub`/`upub` alone as its single-key wallet | — | Casa's descriptor, Specter DIY's keystore, Keystone's, BIP-84's account as a zpub at its published first address |
| Cobo, Keystone | Single-key `.json` (`ExtPubKey`, `MasterFingerprint`, `AccountKeyPath`), the version checked against the path (§5 I2) | yes, also in Create's key slots | — | Sparrow's Cobo file at BIP-84's first address; BlueWallet's Cobo file as Keystone's export of the same key |
| Coldcard, Wasabi | Coldcard's Wasabi file (`ColdCardFirmwareVersion`), as BIP-84 account 0 (§5 I2) | yes | `{stem}-wasabi.json`, BIP-84 account 0 only | Coldcard Mk4's file against its generic export's `first`; Faraday's read back |
| Caravan, Unchained, Sparrow, BlueWallet | Caravan wallet config, `sortedmulti` (§5 I5) | yes; a key with no `bip32Path` or `xfp` refused | `{stem}-caravan.json`, `sortedmulti` with every origin | Sparrow's exports (its two refused ones refused); BlueWallet's at its first address once its third key's path is given; Faraday's read back |

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

1. ~~**SLIP-132 keys**~~ Read since 2026-10-20 (§5 I1): in a
   descriptor, on a key line and bare, each checked against its script
   or path.
2. ~~**Caravan and Unchained wallet configs**~~ Read and written since
   2026-10-20 (§5 I5). A key whose `bip32Path` is `Unknown` or which has
   no `xfp` is refused: upstream's `WalletPolicy` holds no multisig key
   without an origin (BIP-388), so BlueWallet's Caravan file, whose third
   key has no path, does not load as written.
3. **Electrum.** Electrum takes no descriptor, BIP 129 record or Specter
   file, and treats a plain `xpub` as legacy. Faraday writes nothing
   Electrum can make a SegWit watch-only wallet from; an Electrum wallet
   file (what Sparrow exports for Electrum) would. Faraday also reads no
   Electrum wallet file. Whether Faraday signs a PSBT Electrum builds from
   a key without its master fingerprint is untested.
4. ~~**Single-key JSON from Cobo/Keystone and Coldcard's Wasabi file**~~
   Read since 2026-10-20, and the Wasabi file written (§5 I2).
5. **Electrum's own seeds** (not BIP-39): an Electrum wallet's words do not
   load.
6. **Bitcoin Core before 29** takes no multipath descriptor; Faraday's
   import file is one multipath request.
7. ~~**BIP 129 records**~~ Read since 2026-10-20 (§5 I3), by Faraday
   before `osk_bip::bsms`, which still refuses them: records with an
   upper-case address or "No path restrictions". Neither is in BIP 129's
   own examples (its one "No path restrictions" example, Vector A, is
   over plain public keys); both are Sparrow's test records
   `multisig-4.bsms` and `multisig-6.bsms`, which Sparrow accepts.
8. Not read or written: Specter DIY's `addwallet name&descriptor` code,
   BIP-392 `sp()` silent payment descriptors (Sparrow, Keystone), BIP-329
   label files, PSBT version 2 (BIP-370; the Sign tile names BIP-370,
   which nothing here reads), and Coldcard's encrypted backup file.

## 5. The batches that build §4 (F64, owner 2026-10-20)

Every reader goes in `faraday-intake`, after the pattern of
`wallet::coldcard_export` (§3). Faraday reads the format itself and never
edits an upstream crate (`core/`, `opensigner/`); what upstream should
change is noted at the end, for the owner to send. New strings go in
`faraday/faraday-strings/src/en.rs`. New vectors go under
`faraday/faraday-core/tests/vectors/interop/<source>/`, with their source
and licence in that folder's README. Reading needs no choice: every format
is recognised by its content wherever a wallet, a key or a PSBT is read
today. Choices few people need sit under **More options** (F64).

### I1. SLIP-132 keys everywhere (§4 item 1; tracker #43)

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

Where: `faraday-intake/src/slip132.rs`, called from
`read_wallet_as_written` before `parse_any`, `create::read_keys` and
`coldcard_keys`. Tests (`tests/interop.rs`): Casa's descriptor loads as
`sh(wsh(sortedmulti(3,…)))`; Specter DIY's keystore fills a native SegWit
slot; Keystone's singlesig keystore (upper-case fingerprint, `/1/*` only)
reads as `<0;1>`; `wpkh(Zpub…)` and `wsh(multi(…Ypub…))` are refused; the
`abandon…about` BIP-84 vector given as a zpub has BIP-84's published first
address.

### I2. A JSON reader; single-key JSON from Cobo/Keystone and Wasabi (§4 item 4; #46)

`faraday-intake/src/json.rs`: a value parser (depth ≤ 32, size ≤ 1 MiB,
full string escapes including `\uXXXX`), no new crate, and a fuzz target
`faraday/fuzz/fuzz_targets/json.rs`. Every new reader uses it; the
existing `json_string`/`json_object` stay for the files they read now.

Reads Cobo/Keystone `{"ExtPubKey","MasterFingerprint","AccountKeyPath"}`
(`AccountKeyPath` may lack `m/`; the version is checked against the path
as in I1) as the single-key wallet `[fp/path]xpub/<0;1>/*`, also offered as
a key in Create's slots; and Coldcard's Wasabi file
`{"MasterFingerprint","ExtPubKey","ColdCardFirmwareVersion"}`, which
names no path: Wasabi assumes BIP-84 account 0, so it reads as
`wpkh([fp/84h/{0h|1h}/0h]…)`, coin type from the key's network.

Writes `{stem}-wasabi.json` under More options, only for a single-key
`wpkh` at exactly `84h/0h/0h` or `84h/1h/0h`. Check against Wasabi's reader
what `ColdCardFirmwareVersion` must hold; if it needs a real version,
copy BlueWallet's vector. The Cobo/Keystone file is not written: it is a
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

Before `DescriptorRecord::parse`, in `faraday-intake/src/wallet.rs`: (a) an
address line that is all upper-case bech32/bech32m (`BC1…`, `TB1…`,
`BCRT1…`) is lower-cased, and a base58 line is never touched; (b) a `/**`
template whose third line is exactly `No path restrictions` reads as
`/0/*,/1/*`. The first-address check runs unchanged on the result. When
upstream fixes the parser this becomes a no-op. Not read: Sparrow's
fixed-path records (one address, not a wallet), a blank paths line
(cannot be told from a cut-off file), encrypted records. Tests:
`multisig-4.bsms` and `multisig-6.bsms` load with Sparrow's stated first
address; Sparrow's refused records (`-5`, `-7`, `-9`, `-11`, `-13`, `-14`,
`-15`, `-17`) stay refused; a base58 address with its case changed is
refused.

### I4. More options on Public files; Bitcoin Core before 29 (§4 item 6; #48)

`screens.rs::public_files` splits its rows into primary and more;
`public_rows_for` draws the primary rows, a ghost **More options**
(`Action::PublicMore(i)`), and the rest once pressed; the open state lives
on the backup flow and the chart sheet and resets when the screen opens.
Primary: Descriptor; Wallet file (Sparrow, Specter); Multisig config;
Backup sheet; BSMS record; Bitcoin Core import, now "Bitcoin Core 29 or
later"; the cosigners' keys. More options: Bitcoin Core before 29,
Electrum wallet file, Caravan config, Specter DIY, Wasabi file (numbers 9
onward in `backup_public_for`/`public_out`).

Writes `{stem}-bitcoin-core-before-29.json`: two `importdescriptors`
requests, receive (`"internal":false`) and change (`"internal":true`),
each from `Descriptor::into_single_descriptors()` with its own checksum,
`"active":true`, `"range":[0,999]`, `"timestamp":0`. Not for MuSig2, FROST
or silent payments. Tests: a fresh Public files list shows no row under
More options until it is pressed; the file's two descriptors read back as
the wallet; each checksum is Core's (shape of `core/listdescriptors.json`);
`core-check.py` (not in `just`) gains a run against Core 28.x.

### I5. Caravan and Unchained configs (§4 item 2; #44)

Reads and writes
`{"name","addressType":"P2WSH"|"P2SH-P2WSH"|"P2SH","network","client":{"type":"public"},"quorum":{"requiredSigners","totalSigners"},"extendedPublicKeys":[{"name","bip32Path","xpub","xfp","method"?}],"startingAddressIndex":0}`
(Unchained's config is this format; extra fields are ignored). Read as
`sortedmulti` (BIP 67), written only for `sortedmulti` `wsh`/`sh(wsh)`/`sh`
wallets whose keys all have origins. A key with `"bip32Path":"Unknown"` or
no `xfp` is read with no origin (watch-only here, its slot Not here); a key
whose network disagrees with `network` is refused; `startingAddressIndex`
other than 0 is refused. Writes `{stem}-caravan.json` under More options.
Tests: Sparrow's three Caravan exports load and equal its own exports or
the descriptor written by hand; BlueWallet's Caravan file loads with its
third key origin-less; Faraday's file reads back with the same first
address.

### I6. Electrum wallet files (§4 item 3; #45)

Reads an unencrypted Electrum 4.x file: standard
`{"keystore":{"type":"bip32"|"hardware","xpub","derivation","root_fingerprint"|"ckcc_xfp"},"wallet_type":"standard"}`
and multisig `{"wallet_type":"2of3","x1/":{…},…}`. Script type from each
key's version (I1): single key `xpub`→`pkh`, `ypub`→`sh(wpkh)`,
`zpub`→`wpkh`; multisig `xpub`→`sh`, `Ypub`→`sh(wsh)`, `Zpub`→`wsh`, always
`sortedmulti`, every key the same type. Fingerprint from `root_fingerprint`
(hex) or `ckcc_xfp`, a decimal u32 read little-endian (1130956047 →
`0f056943`). A keystore without fingerprint and derivation is read
origin-less. `"use_encryption": true` or an encrypted blob is refused
("This Electrum file is encrypted"). A file holding `xprv` or `seed` is a
secret: `classify` gives it the secret sheet and only its public keys are
read.

Writes `{stem}-electrum.json` under More options as Sparrow writes for
Electrum (`"seed_version": 17`, `"use_encryption": false`, keystores
`{"type":"bip32","xpub":<SLIP-132 for the type>,"xprv":null,"derivation","root_fingerprint","label":""}`),
for `pkh`, `sh(wpkh)`, `wpkh` single keys with an origin and `sortedmulti`
`sh`/`sh(wsh)`/`wsh`; not for Taproot, miniscript, unsorted `multi`,
MuSig2, FROST or silent payments. A PSBT Electrum builds from a bare zpub
carries the account key's own fingerprint; Faraday's matching is
unchanged and a test states the refusal (Faraday's own Electrum file
carries `root_fingerprint`, so Electrum's PSBTs from it are complete).
Tests: Sparrow's Electrum singlesig (`sh(wpkh)` at 49′, ab543c67) and
multisig (2-of-4 `sh(wsh)`, `ckcc_xfp` fingerprints) files; BlueWallet's
Electrum skeletons; `fromsparrow-electrum.json` (holds an `xprv`) is
secret and its wallet equals `fromsparrow-specter.json`'s; write then read
is the same wallet; the bare-zpub PSBT is refused. Optional, not in
`just`: `faraday/tools/electrum-check.py` compares addresses with
`electrum --offline`.

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
CompactSeedQR, Seed XOR, SLIP-39 or a FROST share; in a vault it is a
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
More options, for every wallet with a descriptor but MuSig2, FROST and
silent payments. Tests: Sparrow's `specter-diy-export.txt` loads, named
"Sparrow Multisig"; Faraday's code for it is that text; scanned parts
reassemble to it.

### I9. BIP-392 `sp()` (§4 item 8; #49)

Reads `sp([fp/352h/{coin}h/{acct}h]spscan1q…)` and `tspscan` (Keystone,
Sparrow), a bare `[origin]spscan1q…` (Specter DIY), and Coldcard's export
`bip352` object rooted at its top-level `xfp`, each as an `osk-silent 1`
record. The scan private key is never kept; a file without an origin is
refused ("This sp() descriptor names no key origin"); with the key
loaded, a record whose keys are not the ones it derives is refused.
Writes the descriptor under a silent payments wallet's Export › More
options, through the secret sheet. Where: a public `spscan` decoder in
`faraday-intake` (upstream's `decode_v0` is private). Tests: Sparrow's
Keystone, Specter DIY and Coldcard sp keystores give one wallet rooted at
0f056943; BIP-392's and BIP-352's keys round-trip.

### I10. PSBT version 2 (§4 item 8; #49)

`osk_psbt` and rust-bitcoin 0.32 read version 0 only, so Faraday converts.
`faraday-intake/src/psbt_v2.rs`, on raw BIP-174 maps: v2 → v0 builds the
unsigned transaction (globals `0x02` version, `0x03` fallback locktime,
`0x04`/`0x05` counts; inputs `0x0e` txid, `0x0f` index, `0x10` sequence,
absent = `0xffffffff`; outputs `0x03` amount, `0x04` script), the locktime
by BIP-370's determination from `0x11`/`0x12` (an incompatible set is
refused), drops the v2-only keys and `0xfb`, keeps every other key. Back
to v2: the original bytes plus only the per-input fields signing added
(`0x02`, `0x13`/`0x14`, MuSig2 nonces and partial signatures, `0x07`/`0x08`
when finalised). `wallet::read_psbt` keeps the original on `Spend`
(`v2: Option<Vec<u8>>`); every outward PSBT goes through
`Spend::out_bytes()`; combining another signer's PSBT reads v2 too. Tests:
BIP-370's valid vectors convert and invalid ones are refused; its locktime
cases; a testkit PSBT to v2 and back is byte-identical; v2 and v0 of one
transaction sign identically; the v2 output without the signature fields
equals the input; `psbt_parse` fuzzing gains v2.

### I11. A single private key: sweep it (owner, 2026-10-20)

Reads a WIF private key (`dumpprivkey`; `K…`/`L…` compressed, `5…`
uncompressed; `c…`/`9…` testnet), typed under Add a key › Other forms ›
**Type a private key** or scanned, and a descriptor with a private key
inside (`listdescriptors true`: `wpkh(L1…)`, `wpkh(xprv…/84h/0h/0h/0/*)`),
which `classify` sends to the secret sheet. A Tools tile, **Sweep a
private key** (found by "private key", "WIF", "sweep", "paper wallet"),
opens Add a key on that form.

A WIF loads as one key and a single-address wallet. A WIF names no
script, so the key page shows its addresses — legacy, nested SegWit,
native SegWit, Taproot; legacy only for an uncompressed key — and the
wallet uses the one chosen there (native SegWit first). It offers the
key's public description for an online wallet: the chosen script's
descriptor (`wpkh(02…)`), and for Bitcoin Core `combo(02…)`, which covers
legacy, nested and native SegWit at once; written as files and QR, never
the private key. An xprv inside a descriptor loads as a key with its
paths, as an xprv does today.

The sweep: the online wallet imports the public description watch-only,
finds the funds and builds a PSBT sending everything to a receive address
of the person's seed wallet (Faraday shows one to copy or scan); Faraday's
Spend review shows the inputs from the key's address and the output as
that wallet's own address, worked out here; the person holds to sign. A
key with no BIP-32 origin is matched in the PSBT by its public key (the
PSBT carries no master fingerprint for it). A legacy input needs its full
previous transaction in the PSBT, as Core and Sparrow put it.

The key is never offered as words, SeedQR, shares or a FROST share; it
may be kept in a vault as a single-key record, and is wiped at a lock
otherwise. Strings: `keygen_type_private_key` "Type a private key",
`catalog_sweep_key` "Sweep a private key", `key_single` "A single key",
`wif_uncompressed` "Uncompressed key: legacy addresses only",
`wif_not_a_key` "This is not a private key". Tests: BIP-32's and Core's
published WIF ↔ key ↔ address vectors (compressed and uncompressed,
main and test); the four addresses of a known key; `combo()`'s checksum
as Core gives it; a Core PSBT spending a `wpkh` and a `pkh` output of the
key (built from a test vector, previous transactions included) signs and
finalises, and the review names the destination as the wallet's own; a
`wpkh(L1…)` descriptor file goes to the secret sheet and loads the key;
the key's Backup menu offers no SeedQR.

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

### Notes for upstream (`local/for-upstream/interop.md`, the owner sends them)

`osk_bip::slip132` multisig versions and a version/script check;
`osk_bip::policy` holding a multisig key with no origin, watch-only (a
Caravan config's `"bip32Path":"Unknown"`, §5 I5);
`osk_bip::bsms` (#50); `osk_bip::coldcard::parse` rooting at the top-level
`xfp`; `osk-psbt` reading and writing BIP-370; `osk_bip::silent` public
`spscan`/`sp()` decoding; `osk_bip::core_import` before 29; optionally
`osk-psbt` matching a key by its account xpub's own fingerprint.
