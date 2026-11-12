# Explore a key

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Explore`
**Key:** explore

Shows the public side of a loaded key at any path: the address, the public key and the extended public key, to compare with what another wallet shows for the same path. Secrets show only behind an eye.

## Path

**Key:** explore.path

One button per loaded key chooses the key. **Write it in another form** opens Convert on that key.

**Path** is typed after the fixed m/, with h for a hardened level, up to ten levels; a path that does not parse shows the reason under the field. BIP-84, BIP-86, BIP-49, BIP-44 and BIP-48 fill in the standard path of that purpose for the network, and **Index -1** and **Index +1** step the last level. **Script** is **Native SegWit**, **Taproot**, **Nested SegWit** or **Legacy**; it applies to a path whose purpose level names none, and a path of purpose 84, 86, 49 or 44 uses its own.

## Values

**Key:** explore.values

**Address**, **Public key** and **Extended key** each open as a code when pressed. The eye beside **Secret** shows the key's words, and its passphrase if it has one, or for a key with no words its master seed and codex32 forms. The eye beside **Private key (WIF)** shows the private key of the address, and the eye beside **Extended private keys** shows the root and the account's.

**Addresses** and **Find an address** are offered when the path is in a single-key account; a multisig purpose (BIP-45, BIP-48) has none.

## Addresses

**Key:** explore.addresses

The account's addresses, ten to a page, by the account path and script shown under **Account**. **Receive** and **Change** choose the chain. **From index** is typed in digits, and **Previous** and **Next** with the page size step it. Each row is **Receive** or **Change** with its index and the address, with an eye for its **Private key (WIF)**.

## Find an address

**Key:** explore.find

Looks for a typed address among the account's receive and change addresses. **Address** is typed or taken with **Paste**, where the clipboard is offered; **Not an address** shows under text that is not one.

**Look through** sets how far: First 100, 1,000 or 10,000 on each chain. **Search** starts, and **Stop** ends it while it runs. The result gives how many matches were found, each as **Receive** or **Change** with its index and the matched part marked, and how many more are not listed; or Not in the first, with the depth searched.
