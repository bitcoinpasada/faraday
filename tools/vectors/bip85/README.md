# BIP-85 deterministic entropy vectors

BIP-85 ships no vector files: its vectors are INPUT/OUTPUT bullet lists
under each application's heading in the BIP text. The text is here
verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0085.mediawiki`,
and `extract.py` pulls the vectors out of it into `vectors.json`:

    cd tools/vectors/bip85 && python3 extract.py > vectors.json

To refresh, download the text again, re-run the script and check each
digest changed for a reason.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `bip-0085.mediawiki` | 2026-09-19 | `dcbeb97709eba4751f4af7dc582c35dff37dc831abb5adc5e2e975d708d1cf7e` | `extract.py`, people |
| `extract.py` | written here | `dda5e2e94178f345b15a560564f15d3bc246b61d965b1550bb527adbdf7aaf37` | people |
| `vectors.json` | 2026-09-19 | `018e89f952067120373fe029bbb24be768ac1973bcc6106c5510ba647948ff94` | `core/osk-bip/tests/bip85.rs` |

`vectors.json` has three keys:

- `master` — the one BIP-32 root key every vector in the BIP derives
  from. The script fails if any application's heading states a different
  one.
- `entropy` — the two raw-entropy cases of the Specification section,
  each with its path, derived key and 64 bytes of derived entropy, and
  the DRNG case with its 80 bytes. Nothing reads these: BIP85-DRNG is
  not implemented here, and the entropy is checked through the
  applications that use it.
- `applications` — one list per application this tree derives, keyed by
  the name used in `core/osk-bip/src/bip85.rs`: `bip39`, `wif`, `xprv`,
  `hex`, `base64` and `base85`. Every entry carries the BIP's heading,
  its path, the parameters read back out of that path, the derived
  entropy and the application's own output.

The applications the BIP defines that this tree does not derive — RSA,
RSA GPG, DICE and Nostr — are skipped by the script.

`core/osk-bip/tests/bip85.rs` reads the file whole and derives every
entry from the master key: the three English mnemonics, the HD-seed WIF,
the extended private key, the 64 bytes of hex and the two passwords.

## What the BIP does not say

BIP-85's application 707785' says only "Base85 encode all 64 bytes of
entropy" and names no alphabet. Its published password, `` _s`{TW89)i4` ``,
is in RFC 1924's alphabet and in no other one in use: Ascii85's runs
from `!` to `u` and holds neither `` ` `` nor `{`. The alphabet is
written out in `core/osk-bip/src/bip85.rs` with that reasoning beside it.
