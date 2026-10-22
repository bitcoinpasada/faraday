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
