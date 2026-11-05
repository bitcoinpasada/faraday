# Interoperability with other wallets: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/INTEROP.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/INTEROP.md)

V0.3 · 2026-10-20 · what was tested, what passed, what
Faraday does not read or write yet, and the batches that build §4 (§5,
F64).

## §3 Fixed while testing (from docs/INTEROP.md)

- **Coldcard's account export read with the wrong fingerprint.** Each
  account object in the file carries an `xfp` that is the account key's
  own fingerprint; the master's is the file's top-level `xfp`
  (Coldcard's docs/generic-wallet-export.md). `faraday_bip::coldcard::parse`
  roots the key at the inner one, so the wallet never matched the seed
  that made it. Before that, Faraday's reader took the file's first
  `desc`, which is BIP-44's, and loaded a legacy wallet. Faraday read the
  file itself (`wallet::coldcard_export`): BIP-84 first, rooted at the
  master, and refused if its first address is not the one the file
  states. Since 2026-10-20 that reader is `faraday_bip::coldcard::read`
  (`parse` uses it),
  with `cosigner_keys` for Create's slots; `wallet::coldcard_export`
  only words its refusals.

## §4 Not read or not written yet (from docs/INTEROP.md)

Built 2026-10-07 from the first list: Coldcard-style cosigner key files,
and descriptors that are not multipath (§2's last four rows).

The items built since, as they stood:

1. ~~**SLIP-132 keys**~~ Read since 2026-10-20 (§5 I1): in a
   descriptor, on a key line and bare, each checked against its script
   or path.
2. ~~**Caravan and Unchained wallet configs**~~ Read and written since
   2026-10-20 (§5 I5). A key whose `bip32Path` is `Unknown` or which has
   no `xfp` is refused: `WalletPolicy` holds no multisig key
   without an origin (BIP-388), so BlueWallet's Caravan file, whose third
   key has no path, does not load as written.
3. ~~**Electrum.**~~ Electrum wallet files read and written since
   2026-10-20 (§5 I6). A PSBT Electrum builds from a bare account key
   (no master fingerprint) is not signed: it names the account key's own
   fingerprint, which matches no seed here; Faraday's own Electrum file
   carries `root_fingerprint`, so Electrum's PSBTs from it are complete.
4. ~~**Single-key JSON from Cobo/Keystone and Coldcard's Wasabi file**~~
   Read since 2026-10-20, and the Wasabi file written (§5 I2).
6. ~~**Bitcoin Core before 29**~~ Written since 2026-10-20 (§5 I4), as
   a receive and a change request, under Public files' More options.
7. ~~**BIP 129 records**~~ Read since 2026-10-20 (§5 I3), by Faraday
   before `faraday_bip::bsms`, which still refuses them: records with an
   upper-case address or "No path restrictions". Neither is in BIP 129's
   own examples (its one "No path restrictions" example, Vector A, is
   over plain public keys); both are Sparrow's test records
   `multisig-4.bsms` and `multisig-6.bsms`, which Sparrow accepts.

## §5 I2, an instruction to the implementing agent (from docs/INTEROP.md)

Check against Wasabi's reader
what `ColdCardFirmwareVersion` must hold; if it needs a real version,
copy BlueWallet's vector.

## §5 I4, built (from docs/INTEROP.md)

Built 2026-10-20. The rows read **Core 29 or later** and **Core before
29**, each "Bitcoin Core, watch-only" under it: "Bitcoin Core before 29"
is cut short on the Pi's panel. The More options state is
`BackupState::public_more`, set when the item opens (open when a file
under it is already For the stick) and by the button; the chart sheet
opens the item the same way. The two descriptors are the multipath one
with `<0;1>` as `0` and `1`, which is what `into_single_descriptors()`
gives, written as Faraday writes the descriptor so that a wallet read
from Core's `listdescriptors` gives back Core's own strings. A
`sortedmulti_a` pair is written but does not read back here as a pair
(`miniscript` has no `sortedmulti_a`). `core-check.py` against Core 28.x
is not done.

## §5 I6, built (from docs/INTEROP.md)

Built 2026-10-20 (`faraday-intake/src/electrum.rs`). A keystore without
fingerprint and derivation is read origin-less only for one key, which
the wallet policy holds; a multisig one is refused with a line naming the
key, as Caravan's is (I5), so `fromsparrow-electrum.json` (no origins) is
refused and its three keys are checked against `fromsparrow-specter.json`'s.
A secret file is `FileKind::SecretWallet`: the secret sheet's kind, which
Files and the Inbox offer to load like a wallet. Accepted keystore types
are `bip32` and `hardware`; `wallet_type` `standard` or `NofM`; anything
else is refused by name. The whole-file encryption recognised is the
base64 of Electrum's `BIE1` envelope. Not done: `electrum-check.py`.

## §5 I11, as planned on 2026-10-20 (from docs/INTEROP.md)

Replaced on 2026-10-29 by F171 (sweep only, never kept); §5 I11 says what
was built.

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
previous transaction in the PSBT, as Core and Sparrow put it. Faraday
refuses to sign a legacy input whose script code holds
`OP_CODESEPARATOR`, in every legacy spend and not only this key's (owner,
2026-10-24): the `bitcoin` crate's legacy signature hash takes the script
code as given and Bitcoin Core's removes every `OP_CODESEPARATOR` first
(`tools/crosscheck/README.md`, Known differences), so such an input is
one whose hash cannot be checked against Core. A P2PKH or P2SH multisig
script code holds none, so no spend Faraday makes today is refused.

The key is never offered as words, SeedQR or shares; it
may be kept in a vault as a single-key record, and is wiped at a lock
otherwise. Strings: `keygen_type_private_key` "Type a private key",
`catalog_sweep_key` "Sweep a private key", `key_single` "A single key",
`wif_uncompressed` "Uncompressed key: legacy addresses only",
`wif_not_a_key` "This is not a private key", `sign_codeseparator`
"A legacy input with OP_CODESEPARATOR is not signed". Tests: BIP-32's and Core's
published WIF ↔ key ↔ address vectors (compressed and uncompressed,
main and test); the four addresses of a known key; `combo()`'s checksum
as Core gives it; a Core PSBT spending a `wpkh` and a `pkh` output of the
key (built from a test vector, previous transactions included) signs and
finalises, and the review names the destination as the wallet's own; a
`wpkh(L1…)` descriptor file goes to the secret sheet and loads the key;
the key's Backup menu offers no SeedQR; a PSBT whose legacy input's
script code holds `OP_CODESEPARATOR` is refused with that line and signs
nothing, and the cross-check's legacy sighash cases still meet the
known difference only outside what Faraday signs.

## §5 The batches that build §4, as it stood on 2026-11-05 (from docs/INTEROP.md)

The labels I1 to I11 are the ones decisions and old commits use.

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

Built in `faraday-intake/src/specter_diy.rs`, read in
`faraday-intake/src/wallet.rs::read_wallet` before the other readers.
Read as Specter DIY's own `Wallet.parse` reads it (its
`src/apps/wallets/wallet.py`): the name is everything before the last
`&`, so a name may hold one; keys none of which carries a derivation are
the two chains, keys that carry one keep it. Faraday checks a checksum
where there is one, which Specter DIY drops unread, and refuses a command
with no descriptor after its `&` and every key or wallet any descriptor
is refused for. The wallet's name, in Files, the Inbox and Family, is the
command's (`wallet::carried_name`, beside a wallet `.json`'s label).
Written for every wallet but MuSig2 and silent payments: a single-key or
multisig wallet as Sparrow writes it, its keys without their derivation
and no checksum, any other with `/<0;1>/*` (embit reads both). The row is
**Specter DIY wallet** ("Specter DIY, addwallet"), its codes in numbered
parts at the middle part size (`Code::SpecterDiy`, `public_out` 12 for the
text and 14 for the pictures, `{stem}-specter-diy-part-{j}-of-{k}.png`).
Home's Scan takes the command as text into Files, where it is a wallet
named by it. The `import` cross-check area writes it from every test
wallet, with and without derivations, checksum or not, and reads
Sparrow's export, against Core's `deriveaddresses`.

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

Built in `core/faraday-bip/src/silent.rs` (`decode_scan_key`, public:
the scan key's public key in constant time, the private key wiped) and
`core/faraday-bip/src/silent_wallet.rs` (`parse_descriptor`,
`from_scan_key`), so the decoder is in core beside `decode_v0` rather
than in `faraday-intake`, which reads the forms
(`faraday-intake/src/wallet.rs::read_wallet`, the one line `sp(…)` or
`[origin]spscan1q…`) and words the refusals. The origin must be a
fingerprint and BIP-352's path on the key's network to a hardened
account; `sp(KEY,KEY)`, `spspend` (which carries the spend private
key), a derivation after the key and any other function are refused.
`tspscan` reads as testnet. A text holding `spscan1` is a wallet file
in Files, not a secret: the scan key sees the wallet's payments and
spends none (owner, 2026-10-29). A text holding `spspend1`, the spend
private key, is a secret: a scanner names it as a private key and files
nothing (F167; `faraday_intake::secret_text`), and a file of it is a
secret in Files (`wallet::holds_secret`, as an Electrum file with an
`xprv` is). Coldcard's export is read as before, BIP-84 first, then 86,
49, 44; its `bip352` account is the wallet where the export has no
single-sig account (`faraday_bip::coldcard::silent` reads it on its
own). An export holding both asks which to load (owner, 2026-10-29):
**Which account to load?**, a dialog naming the file, with **Cancel**,
**Single-sig · BIP-84** (the account read) and **Silent payments ·
BIP-352** (`Sheet::Account`, `Action::LoadAccount`), the export waiting
on the session with both wallets read (`faraday-core`'s
`wallet/account.rs`); the choice is kept for the session and the press
that asked runs again, as the key path sheet's does (F132). The writer is
**sp() descriptor** ("Sparrow, Keystone · holds the scan key") under
Public files' More options (`Action::SDescriptorOut`), from the loaded
key whose scan and spend keys are the record's, through the secret sheet
(`faraday_keys::silent::Receiver::descriptor`, the scan key's reveal);
with no such key loaded it offers nothing out. BIP-392 publishes no
reference code or vectors (its text in `tools/vectors/bip392/`, unchanged
since it was vendored); the `sp` cross-check area writes each receiver's
scan key with BIP-352's reference bech32m and reads it with the
reference's curve, damaged one character, in another version and in
upper case, and the `sp()` around it from BIP-392's text. The fuzz target
is `fuzz/fuzz_targets/silent.rs`.

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
  their address). Labels belong in the online wallet. Checked against
  practice (owner, 2026-11-03): BIP 329 is a draft for moving labels
  between wallets that keep them; it names no signing device and lets a
  reader ignore records it does not store. Sparrow, BlueWallet and
  BitBox's app read it; a search of the source of Coldcard, Jade, Passport,
  SeedSigner, Krux, Specter DIY, Keystone, Trezor and BitBox02 on
  2026-11-03 found no mention of it. A scanner names a code of labels and
  does nothing more with it.
- **Coldcard's encrypted backup** (7z, AES-256-CBC): AES-CBC and a 7z
  parser on the way to the seed, to save typing a 12-word password no
  easier than the seed words that already load (confirmed by the owner,
  2026-11-03).
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
