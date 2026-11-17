# Check a multisig setup

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `MultisigCheck`
**Key:** multisig-check

This shows what a wallet of several keys is, from its descriptor, a BIP-388 policy, a setup file or a BIP 129 record, so you can compare it with every other signer's device and the coordinator. Nothing is added to the session.

## Setup

**Key:** multisig-check.setup

Opened from Tools, the tool opens the setup in Files when exactly one waits there. With several, **Choose a setup** lists them by name. With none and no wallet of several keys loaded, a card says "No multisig setup to check" with **Scan** (filled), **Open Files** and, while a stick is attached and no key is open, a button to bring in from that stick. **Scan** reads the setup from a code. **A wallet loaded here** lists the wallets of several keys already loaded. A setup that is refused shows the reason in one line and no Result: a checksum that does not hold, a key with no fingerprint and derivation path, a private key, a wallet of one key.

## Result

**Key:** multisig-check.result

The shape of the wallet, then each cosigner's fingerprint, path and whole extended public key, in groups of four, in the descriptor's order. **Yours** marks a cosigner that a key loaded here is. The descriptor checksum says whether the setup gave one and it held, or that it was worked out here. **Quorum** shows where the shape does not state one, as seeds; **Ways to spend** lists each way a script tree or a miniscript is spent. The first receive address is the one every coordinator shows when the wallet is set up.

**Add this wallet** loads it here; the button then reads **Open the wallet**.

The result is what the setup says. It does not show that a signer holds a key or that a coordinator stored the same setup: each device's own screen says that.
