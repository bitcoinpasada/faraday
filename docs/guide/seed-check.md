# My seed makes my wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `SeedCheck`
**Key:** seed-check
**Words:** seed, fingerprint, xpub, multisig

This page shows what a seed makes, so you can compare it with what a wallet app and a signer show: the fingerprints, an account's extended public key and its first receive and change addresses. It opens from Tools, or from **Check a backup** on a seed words file or a SLIP-39 or codex32 share file in Files, which it takes on the Seed card, and returns where it was opened; nothing is needed first, and it loads no key unless you press **Keep this key**.

## Seed

**Key:** seed-check.seed

**Type the words** takes the words of a BIP-39 seed, in any word list Faraday reads; **Scan a SeedQR** reads a standard or compact code. A key already loaded appears as a row with its fingerprint, and choosing it skips the passphrase. The card closes on the number of words once their checksum holds. **SLIP-39 shares**, **codex32** and **Seed XOR parts** are typed or scanned here as in Add a key. **Krux code** reads an encrypted code with **Scan a Krux code** or **From Files** and asks its passphrase; the words it holds go on to the Passphrase card as typed words do. A SLIP-39 or codex32 backup has no words: only SLIP-39 asks a passphrase. Typing the seed puts it on a second device, which is safe within Faraday's design (offline, powered off after use); Check says why.

## Passphrase

**Key:** seed-check.passphrase

Choose **No passphrase**, or **Add a passphrase**, which asks for it twice, each time behind an eye, and takes it when you press **Use this passphrase**. Hardware keyboard keys are read as US; once a hardware keyboard has typed one, **Use this passphrase** waits until the eye has been opened. The card closes on the key's fingerprint, with **with a passphrase** where one was given. **Change** on Seed or Passphrase lets you enter the key again.

## Account

**Key:** seed-check.account

Choose the account your wallet app uses: **Native SegWit**, **Taproot**, **Nested SegWit**, **Legacy** or **Multisig**, each with its path at account 0 and the start its addresses have. **Another account or path** takes any account path and a script. A wrong choice shows at once, as addresses that do not match.

## Result

**Key:** seed-check.result

**Fingerprint**, **Without the passphrase** where one was given, **Path**, **Script**, **Xpub** with its origin, and the first receive and change addresses, each whole. Where the script has a SLIP-132 spelling, a button over the xpub shows it or the plain xpub. The xpub and the receive address open their code. A multisig account has no addresses: they belong to the whole wallet.

**Another account** and **Another seed** let you choose again. **Done** drops the seed. **Keep this key** loads it as **Add a key** does, and is not offered while a stick is in or for a key already loaded. Cancel, leaving the screen, the idle lock and a lock drop the seed too.

The result shows what the seed makes, not what a wallet app or signer holds: a match means both made the same keys from the same seed.
