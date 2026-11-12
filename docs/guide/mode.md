# Mode

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Mode`, `Samples`
**Key:** mode

The sheet lists the modes the app offers, the one in use marked; **Not now** closes it. Practice is Faraday on testnet with public sample keys, on an online computer: it opens every screen and works as Faraday does, but makes no secret a person could take away.

## Samples

**Key:** mode.samples

Practice takes only its samples; any other key is refused by its fingerprint. **Load samples** opens a list of sample **Wallets** and sample **Keys**, each with a tick box. **Load ticked** loads the ticked rows as if typed or scanned: a wallet with the sample keys it uses, a set as one key. A loaded row is ticked and dimmed. **Select all** and **Unselect all** change every tick.

Each row says what the sample is: a wallet its kind and script, a key its form and size. The sample passphrase is `faraday`; **Use faraday** puts it in a passphrase field. Where a screen asks for a key, a vault, a signed message or an address, **Use a sample** puts one in. Every sample is public, so anything sent to its addresses can be taken by anyone.

## Not made in Practice

**Key:** mode.not-made

These actions are dimmed and do nothing: Seed XOR parts, New key as SLIP-39 shares, a GPG key made or restored from its paperkey, Secure Boot keys, every BIP-85 output, the vanity passphrase dial, a Lightning node's private key shown or sent out, and a silent payments scan key sent out; also the way to a vault to keep one of those. Their screens open and everything else on them works.

New key runs as on the device, but at the lock Practice adds the next sample of the length chosen that is not loaded, in place of the key made; with every sample of that length loaded the lock is dimmed. Codex32 screens put a sample's strings in place of those entered. A key made in Faraday on a stick, on a computer with no network, makes all of these.

## Test networks

**Key:** mode.networks

**Testnet**, **Signet** and **Regtest** take any seed and make every action, with coins that have no value. Faraday Demo starts in Practice, which replaces Mainnet there. Every screen that takes a seed or a secret says **Never type a seed that holds real bitcoin here.**; a seed typed on an online computer may be read by anything running there.
