# Check a multisig setup

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `MultisigCheck`
**Key:** multisig-check
**Words:** multisig, quorum, descriptor, fingerprint

This shows what a wallet of several keys is, from its descriptor, a BIP-388 policy, a setup file or a BIP 129 record, so you can compare it with every other signer's device and the coordinator. Nothing is added to the session.

## Setup

**Key:** multisig-check.setup

Opened from Tools, the tool opens the setup in Files when exactly one waits there. With several, **Choose a setup** lists them by name. With none and no wallet of several keys loaded, a card says "No multisig setup to check" with **Scan** (filled), **Open Files** and, while a stick is attached and no key is open, a button to bring in from that stick. **Scan** reads the setup from a code. **A wallet loaded here** lists the wallets of several keys already loaded. A setup that is refused shows the reason in one line and no Result: a checksum that does not hold, a key with no fingerprint and derivation path, a private key, a wallet of one key.

## Result

**Key:** multisig-check.result

The shape of the wallet, then each cosigner's fingerprint, path and whole extended public key, in groups of four, in the descriptor's order. **Yours, can sign** marks a cosigner that a key loaded here is. The descriptor checksum says whether the setup gave one and it held, or that it was worked out here. **Quorum** shows where the shape does not state one, as seeds; **Ways to spend** lists each way a script tree or a miniscript is spent. The first receive address is the one every coordinator shows when the wallet is set up.

**Add this wallet** loads it here; the button then reads **Open the wallet**.

The result is what the setup says. It does not show that a signer holds a key or that a coordinator stored the same setup: each device's own screen says that.

## Marks

**Key:** multisig-check.marks

Each key has a mark. A green check with **Yours, can sign** is a key this device worked out from a seed loaded here; a key made here whose backup is not done reads **Yours, signs after its backup**, as it signs nothing until the backup is done. A green check with **Checked by you** is a key you ticked. A grey circle with **Not checked** is a key nothing here vouches for. A red cross is a key that claims the fingerprint of a key loaded here and is not that key: do not use the wallet. Under the keys a line says how many are not checked, or **Every key checked**.

## Ticking a key

**Key:** multisig-check.tick

To tick a key, compare it, in full, with what its own device shows for that signer, then press it; press again to untick. Ticking is optional and the wallet can be added with keys not checked, but the wallet then shows that for the rest of the session, on its chart node, beside its receive addresses and in the review of a spend. Lock ends it unless a vault holds the wallet. Keys ticked here go with the wallet when you add it, each on the same key.

## The checksum

**Key:** multisig-check.checksum

The descriptor checksum is a typing check: it catches a slip when you copy a descriptor. It is not proof, because a different wallet with the same eight characters can be made. Compare the keys, not only the checksum.
