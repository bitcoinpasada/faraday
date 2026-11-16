# Check a multisig setup: the first address

`vectors.json` holds, for each kind of multisig wallet the Check a multisig
setup flow reads (`Tools`, "Check a multisig setup",
`faraday/faraday-core/src/multisig_check/`), a descriptor with its checksum
and the first receive address Bitcoin Core derives from it. The flow is
tested against it (`faraday/faraday-core/tests/all/multisig_check.rs`): the
descriptor, read there, shows Core's address.

The kinds: `wsh`, `sh(wsh)` and `sh`, each with `multi` and `sortedmulti`
(2 of 3); `tr` with the BIP 341 unspendable internal key and `multi_a` and
`sortedmulti_a` (2 of 3); and a `wsh` miniscript. The keys are the account
keys of BIP 32's test vectors 1, 2 and 3 at the BIP 48, BIP 45 and BIP 87
paths, with their origins.

**Tested against** Bitcoin Core 31.1 (`tools/reference/core/release.pin`),
an implementation Faraday did not write: `gen.py` runs its pinned
`bitcoind` on testnet4 with no network, imports the three master keys (made
from the vectors' seeds with the standard library, not with Faraday's code)
into blank wallets as descriptors, lists the account keys, and takes each
descriptor's checksum from `getdescriptorinfo` and its first receive address
from `deriveaddresses <descriptor> [0,0]`. The node is a testnet4 one, so Core writes tpub keys and tb1/2 addresses, which
the check shows for tpub keys; nothing is changed after Core.

To remake it: fetch Core once (`python3 -I -B tools/crosscheck/fetch_core.py`)
and run `python3 -I -B tools/vectors/multisig-check/gen.py`. The file is the
same on every run, since nothing in it is random.
