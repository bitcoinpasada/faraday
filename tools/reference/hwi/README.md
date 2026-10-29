# HWI's PSBT reader, vendored

The PSBT classes of HWI, the Bitcoin Core project's Hardware Wallet
Interface, and the few modules they import, copied unchanged, for the
crypto cross-check's `psbt` area (`tools/crosscheck/area_psbt.py`,
F160). The area reads every version 2 PSBT with HWI's `PSBT` class, as
published, and requires of Faraday's version 2 reader HWI's acceptance
or refusal, its locktime, its unsigned transaction and its version 0
form. Bitcoin Core itself refuses version 2 PSBTs, so it checks version
0 only. For version 0 the area also asks HWI whether a version 2 field
is allowed there, which Core reads as an unknown field.

Source: `https://github.com/bitcoin-core/HWI`, commit
`e63a0af2bf5c7c60b8cc71d99aba59068e8be0f0` on `master` (2026-08-25,
"Merge bitcoin-core/HWI#792"), folder `hwilib/` and `LICENSE`. Fetched
with `git clone`. The commit is signed; its signature was checked with
`git verify-commit` on 2026-10-29 against the key
`1528 1230 0785 C964 44D3 334D 1756 5732 E08E 5E41` (Ava Chow, HWI's
maintainer) as Bitcoin Core's `guix.sigs` repository publishes it
(`builder-keys/achow101.gpg`, at the commit `tools/reference/core/release.pin`
names): a good signature by that key. What pins the copy is the commit
hash and the digests below.

Why a commit and not the last release, 3.2.0 (2026-02-09, its tag signed
by the same key): 3.2.0's version 2 reader overwrites every version 2
PSBT's transaction version with 1 and its fallback locktime with 0
(`cache_unsigned_tx_pieces`), sets the locktime it computes on the PSBT
object instead of the transaction, requires `PSBT_IN_SEQUENCE`, and
takes BIP-370's version 2 fields in a version 0 PSBT and required
locktimes out of their range. HWI fixed each of these on `master`
between 2026-07-10 and 2026-08-06; no release carries them yet.

Licence: MIT (`LICENSE`), which allows copying.

Only the files the PSBT classes import are copied: `psbt.py`, `key.py`
(key origins), `tx.py`, `_serialize.py` and `_script.py` (transactions),
`common.py`, `errors.py` and `_base58.py`, and the package's
`__init__.py`. The device drivers, the command line and the rest of
HWI are not.

`shims/` is written here, not upstream: `_serialize.py` imports
`Protocol` from the `typing_extensions` package, for type hints only;
`shims/typing_extensions.py` gives the standard library's
`typing.Protocol` where the package is missing (the area puts it last
on the import path, so a machine's own `typing_extensions` comes first).

SHA-256 of every file (also in `tools/crosscheck/references.sha256`,
which the cross-check checks before it imports anything):

| File | SHA-256 |
|---|---|
| `LICENSE` | `fd02c0dfea382dd4c42bcf87f1d638fcc5962238a319447d3ca035074bcd07f7` |
| `hwilib/__init__.py` | `3945f7ed877a64ef367741892f67662b48194ed73fc6f953bc640897623e0fc9` |
| `hwilib/_base58.py` | `dd5d3fc11eae7c81ba2414cd39f6ef5106e9cb3894ae2c1773bf23af46468ad6` |
| `hwilib/_script.py` | `4531b3dc525b1c776398e95a776ef5f38b8a79121e0cda345827e9dc31976422` |
| `hwilib/_serialize.py` | `291c5c19c303dd86104914d7a7b0bcc32d8fb6f9a1e0a7d90500b16b5b396bb5` |
| `hwilib/common.py` | `a4d8a88cc284d9a0db9db73c9e977518675ae803bc3b476655de3fb8bd0fdf4f` |
| `hwilib/errors.py` | `08c95f1cd7a88271637680e363ef27a5032d3fc5e6e635cbfc78dc6552ef25b0` |
| `hwilib/key.py` | `0d2e04d4ebe6d18ce50133331ba80cc1d6ef2600a46e5a32875843dcb21d4b84` |
| `hwilib/psbt.py` | `837d8cb331ef24ca1c01e1468228e89b9725d6de756636c45764b71352408899` |
| `hwilib/tx.py` | `44e27afa5d0bd30f15cad5431723e077681753e7e9026d459c83936d71b56d80` |
| `shims/typing_extensions.py` | `bde5e125d26755622abd77a96977ab442b689cc7383dc11fae39c39488aa3d40` |

## What it covers

- Reading a version 2 PSBT: the global, input and output fields of
  BIP-370, BIP-174 and BIP-371, their key lengths and value sizes, a
  key twice, the fields version 2 requires and the ones it forbids, and
  the range of a required locktime.
- BIP-370's version 2 fields refused in a version 0 PSBT.
- The locktime BIP-370 determines (`compute_lock_time`), the unsigned
  transaction (`get_unsigned_tx`) and the version 0 PSBT of a version 2
  one (`convert_to_v0`, written out by `serialize`), which the area has
  Core read.

## What it does not cover

- A taproot signature of 65 bytes whose sighash byte is 0 (BIP-341),
  bytes after the last map, and a stream that ends inside a map: HWI
  reads them, which the BIPs refuse. The area keeps its own rule for
  each, labelled as Faraday's own.
- A value shorter than its length says: HWI's `deser_string` takes the
  bytes there are, and HWI writes the field out repaired. Where that
  happens the area has Core judge the version 0 PSBT of the PSBT's own
  pairs.
- Whether a key is on the curve: HWI does not check. Faraday's version 2
  reader leaves BIP-174's and BIP-371's fields to its bridge to version
  0, which checks them, and the area has Core read the bridge's output.
