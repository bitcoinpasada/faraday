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
