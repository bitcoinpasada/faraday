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

## Compare with the wallet app's

`compare.json` holds what the Compare button of the same flow is tested
against (`faraday/faraday-core/tests/all/seed_check.rs`): `bip84_change_7`,
the address at chain 1, index 7 of the BIP-84 account of the test phrase;
three BIP-48 cosigner keys (`cosigners`: their phrases, master fingerprints
and extended public keys at m/48'/0'/0'/2'), the test phrase being the
second; and `wallet`, a 2-of-3 `wsh(sortedmulti())` descriptor of the three.

**Tested against** implementations Faraday did not write: `compare_gen.py`
makes the seeds with the standard library's PBKDF2 (BIP-39), the keys with
btclib's BIP-32 and the address with Bitcoin Core's reference
`segwit_addr`. To remake it, run `python3 -I -B
tools/vectors/seed-check/compare_gen.py` (btclib installed); the file is the
same on every run.
