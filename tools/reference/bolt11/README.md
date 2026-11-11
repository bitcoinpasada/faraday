# The `bolt11` package, vendored as its wheels, with a `coincurve` stand-in

The crypto cross-check's `lightning` area (`tools/crosscheck/area_lightning.py`)
asks the pure-Python `bolt11` package to read BOLT #11 invoices and compares
what it reads with Faraday's decoder (`core/faraday-codec/src/bolt11.rs`).
This folder holds the four wheels it needs, each byte for byte the file PyPI
publishes, and a stand-in for the one dependency that is not pure Python.

| Wheel | Project | Release | Licence | SHA-256 |
|---|---|---|---|---|
| `bolt11-2.2.0-py3-none-any.whl` | bolt11, <https://github.com/lnbits/bolt11> | 2.2.0 | MIT | `4684ac33cf274a2ee00f45b5a691d6743ede04ef455b4c12046648b4f20e2a81` |
| `bech32-1.2.0-py3-none-any.whl` | bech32, <https://github.com/fiatjaf/bech32> | 1.2.0 | MIT | `990dc8e5a5e4feabbdf55207b5315fdd9b73db40be294a19b3752cde9e79d981` |
| `base58-2.1.1-py3-none-any.whl` | base58, <https://github.com/keis/base58> | 2.1.1 | MIT | `11a36f4d3ce51dfc1043f3218591ac4eb1ceb172919cebe05b52a5bcc8d245c2` |
| `bitstring-3.1.9-py3-none-any.whl` | bitstring, <https://github.com/scott-griffiths/bitstring> | 3.1.9 | MIT | `0de167daa6a00c9386255a7cac931b45e6e24e0ad7ea64f1f92a64ac23ad4578` |

Each wheel is the file PyPI publishes for a release of the project's
repository, whose tag and commit are these (`bech32` has no tag: its
commit is the one that set version 1.2.0 in `setup.py`):

| Wheel | Repository | Tag | Commit |
|---|---|---|---|
| `bolt11` | <https://github.com/lnbits/bolt11> | `v2.2.0` | `51bd3b97aeed28a9c4bb0e10dbe8d9414a79d01b` |
| `bech32` | <https://github.com/fiatjaf/bech32> | none | `231e4d88b15f3dc8faf7d339f365b84f6ab5cbcc` |
| `base58` | <https://github.com/keis/base58> | `v2.1.1` | `11c293f4479fafffbc7b766fcb703a835c02dccd` |
| `bitstring` | <https://github.com/scott-griffiths/bitstring> | `bitstring-3.1.9` | `c84ad7f02818caf7102d556c6dcbdef153b0cf11` |

`bolt11` asks for `bitstring` without a version. Its 4.x and 5.x releases need
a C or Rust extension (`bitarray`, `tibs`); 3.1.9 is pure Python and is what
the cross-check runs `bolt11` over. `bolt11`'s command-line module, which
imports `click`, is not imported. The notices are in each wheel's
`.dist-info/licenses/` (or `LICENSE`).

The wheels are kept whole and unpacked by `common.use_wheels` into a scratch
folder of the process's own, after `references.sha256` has been checked.

## The `coincurve` stand-in (`shims/coincurve/`, written here, not upstream)

`bolt11` signs, verifies and recovers through `coincurve`, the binding of
libsecp256k1, a C extension. `shims/coincurve/` provides the calls
`bolt11/models/signature.py` makes (`PrivateKey.from_hex` and
`sign_recoverable`, `PublicKey.from_signature_and_message` and `format`,
`verify_signature`, and `coincurve.ecdsa`'s `deserialize_recoverable`,
`recoverable_convert` and `cdata_to_der`), each handed to btclib's `ecc`
(`tools/reference/btclib/`) on its own pure-Python curve, libsecp256k1
turned off. As libsecp256k1 does, verification refuses a signature whose s
is above half the order (a high-S signature); recovering a key does not
look at s.

SHA-256 of every file is in `tools/crosscheck/references.sha256`, which the
cross-check checks before it imports anything.
