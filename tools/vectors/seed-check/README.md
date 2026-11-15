# My seed makes my wallet: BIP-44 and BIP-48

`vectors.json` holds what Bitcoin Core makes of the test phrase ("abandon
... about", no passphrase), which the Check a seed flow (`Tools`, "My seed
makes my wallet", `faraday/faraday-core/src/seed_check/`) is tested against
(`faraday/faraday-core/tests/all/seed_check.rs`) where the BIPs publish no
account of their own:

- `bip44`: the account's extended public key at m/44'/0'/0' and the first
  receive and change addresses of its `pkh()`;
- `bip48`: the cosigner's extended public key at m/48'/0'/0'/2'.

**Tested against** Bitcoin Core 31.1 (`tools/reference/core/release.pin`),
an implementation Faraday did not write: `gen.py` runs its pinned
`bitcoind` on regtest with no network, imports the phrase's master key as
descriptors into a blank wallet, and writes the keys and addresses Core
lists and derives. The master key is made from the phrase with the standard
library (PBKDF2-HMAC-SHA512 and BIP-32's master step), not with Faraday's
code. The node is a regtest one, so Core writes
tpub keys and regtest addresses; `gen.py` changes only their version bytes to
the mainnet ones (xpub, and the P2PKH prefix), the key and hash bytes are
Core's.

To remake it: fetch Core once (`python3 -I -B tools/crosscheck/fetch_core.py`)
and run `python3 -I -B tools/vectors/seed-check/gen.py`. The file is the same
on every run, since nothing in it is random.
