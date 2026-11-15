# My seed makes my wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SeedCheck`
**Key:** seed-check

Shows what a seed makes, to hold against what a wallet app and a hardware wallet show: the fingerprints, an account's extended public key and its first receive and change addresses. It opens from Tools and returns there; nothing is needed first, and no key is loaded by it unless **Keep this key** is pressed.

## Seed

**Key:** seed-check.seed

**Type the words** takes the words of a BIP-39 seed, in any word list Faraday reads; **Scan a SeedQR** reads a standard or compact code. A key already loaded is a row by its fingerprint, and it skips the passphrase. A card closes on the number of words once their checksum holds. A set that is not BIP-39 is loaded with **Add a key** first.

## Passphrase

**Key:** seed-check.passphrase

**No passphrase**, or **Add a passphrase**, which asks it twice, each behind an eye, and takes it with **Use this passphrase**. The card closes on the key's fingerprint, with **with a passphrase** where one was given. **Change** on Seed or Passphrase takes the key again.

## Account

**Key:** seed-check.account

The account the wallet app uses: **Native SegWit**, **Taproot**, **Nested SegWit**, **Legacy** or **Multisig**, each with its path at account 0 and the start its addresses have. **Another account or path** takes any account path and a script. A wrong pick shows at once, as addresses that do not match.

## Result

**Key:** seed-check.result

**Fingerprint**, **Without the passphrase** where one was given, **Path**, **Script**, **Xpub** with its origin, and the first receive and change addresses, each whole. Where the script has a SLIP-132 spelling, a button over the xpub shows it or the plain xpub. The xpub and the receive address open their code. A multisig account has no addresses: they belong to the whole wallet.

**Another account** and **Another seed** pick again. **Done** drops the seed. **Keep this key** loads it as **Add a key** does, and is not offered while a stick is in or for a key already loaded. Cancel, leaving the screen, the idle lock and a lock drop the seed too.
