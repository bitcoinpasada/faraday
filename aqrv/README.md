# Argon QR Vault

The Argon QR Vault (AQRV) is a proposed open format for one small secret,
such as a BIP-39 seed, encrypted under a passphrase with Argon2id and small
enough for one QR code. It is a draft for outside review: it has had no
review outside the project, and the code here is for testing it with test
data only. None of Faraday's apps uses it (F245).

## Where to find it

| What | Where |
|---|---|
| The specification: the byte layout, the keys and the tag, recognition, the `.aqrv` file | [`docs/AQRV.md`](../docs/AQRV.md) |
| How it compares with Krux's KEF and an OpenPGP QR vault | [`docs/AQRV.md` §14](../docs/AQRV.md#14-comparison-with-kef-and-with-an-openpgp-qr-vault) |
| The security analysis | [`docs/AQRV.md` §12](../docs/AQRV.md#12-security-analysis) |
| The questions for an outside cryptographer | [`docs/AQRV.md` §13](../docs/AQRV.md#13-questions-for-an-outside-cryptographer) |
| The test vectors, made with argon2-cffi and PyCA, and the script that checks them | [`tools/vectors/aqrv/`](../tools/vectors/aqrv/) |
| The library: the format and nothing else | [`aqrv/`](aqrv/) |
| The command line, `aqrv` | [`aqrv-cli/`](aqrv-cli/) |
| The desktop test app and its shell | [`aqrv-app/`](aqrv-app/), [`aqrv-desktop/`](aqrv-desktop/) |

## Checking the vectors

With Python 3 and the packages the vectors' README names:

```sh
python3 tools/vectors/aqrv/check.py
```

The library's tests run every case of `tools/vectors/aqrv/aqrv.json`:

```sh
cargo test --locked -p aqrv
```

## Comments

Comments on the format go in this repository's GitHub issues, one issue
for each point. A reply to one of the questions in §13 names its number.
