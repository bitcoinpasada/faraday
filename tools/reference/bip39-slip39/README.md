# bip39-slip39, vendored as its wheel

The cross-check's `bip39`, `slip39` and `bip85` areas ask this package,
beside their other judges, for BIP-39 (words to entropy, entropy to
words, the seed, in every language it ships), for SLIP-0039 (a share
read, a set combined, a secret split) and, run by btclib-wallet's
`bip85`, for the sentence of BIP-85's mnemonic application
(`tools/crosscheck/area_bip39.py`, `area_slip39.py`, `area_bip85.py`).
This folder holds the one wheel, byte for byte the file PyPI publishes.

| Wheel | Project | Release, commit | Licence | SHA-256 |
|---|---|---|---|---|
| `bip39_slip39-2026.10.24-py3-none-any.whl` | bip39-slip39, <https://github.com/btclib-org/bip39-slip39> | v2026.10.24, `f75580077b2046d491abec1ef2a372080031d920` | MIT | `dc0cdb66ce64c9e62b0688bcc5cd27e629c14ba377fe1f2978d85862461a7074` |

The same digest is in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything. It is the digest PyPI
lists for the file. The package's files were compared with the tagged
commit and are the same files; the licence in the wheel's
`.dist-info/licenses/` is the repository's `LICENSE` (MIT, with the
notice "Copyright (c) The btclib developers" in each file).

## What it is, and who wrote it

By the btclib developers (Ferdinando Ametrano and others), split out of
btclib in 2026. It needs Python 3.11 and the standard library alone
(`hashlib`, `unicodedata`, `secrets`, `os`); it imports nothing else and
reads its word lists from `_data/` beside its modules, so each process
unpacks it as `common.use_wheels` unpacks btclib's wheels.

It shares no code with Faraday's `faraday_bip` (Rust), with python-mnemonic
(Trezor's BIP-39, `tools/reference/bip39/`) or with python-shamir-mnemonic
(Trezor's SLIP-0039, `tools/reference/slip39/`), so it is a second lineage
for each. It ships twelve BIP-39 word lists: Faraday's ten and Russian and
Turkish, which Faraday does not have, so those two are not compared.

How it reads differently from python-mnemonic, and so what the areas must
allow for: it collapses runs of white space in a sentence, takes the
language from the words when none is given, and writes a Japanese sentence
with the ideographic space; python-mnemonic splits on single spaces. The
areas give it a language and a sentence in single-space NFKD form, as
before.

## Updating

With network, in a scratch folder: download the wheel from PyPI, check its
hash against the one the release lists there, compare its package with a
checkout of the tagged commit, copy it here, change the table above and
write the new hash into `tools/crosscheck/references.sha256`, in a
reviewed commit.
