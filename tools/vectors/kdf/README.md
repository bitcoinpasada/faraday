# KDF known answers

The start-up self-test (`core/osk-selftest`) checks the two KDFs the
BIP vectors do not reach. PBKDF2-HMAC-SHA512 is checked by the BIP-39
seed vectors and HMAC-SHA512 by the BIP-32 one.

## scrypt

RFC 7914 §12, the first vector: password and salt empty, N = 16, r = 1,
p = 1, 64 bytes.

```
77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442
fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906
```

Python's `hashlib.scrypt` (OpenSSL) gives the same bytes:

```
python3 -I -c "import hashlib; print(hashlib.scrypt(b'', salt=b'', n=16, r=1, p=1, dklen=64).hex())"
```

## Argon2id

RFC 9106's own vectors use a secret and associated data, which
`osk_backup::argon2id` does not take, so this answer was computed with
two implementations, which agree: argon2-cffi, which binds the
reference C implementation, and `cryptography`, which calls OpenSSL.

| Field | Value |
|---|---|
| password | `password` (ASCII) |
| salt | `somesaltsomesalt` (ASCII) |
| memory | 256 KiB |
| passes | 2 |
| lanes | 1 |
| version | 0x13 |
| length | 32 |
| key | `cab746b4621993fdc91ec50787980b414a90a692f0bc68dfe19f9c25b3cba9ec` |

```python
# uv run --no-project --with argon2-cffi --with 'cryptography>=44' python -I argon.py
from argon2.low_level import Type, hash_secret_raw
from cryptography.hazmat.primitives.kdf.argon2 import Argon2id
pw, salt = b"password", b"somesaltsomesalt"
a = hash_secret_raw(pw, salt, time_cost=2, memory_cost=256, parallelism=1, hash_len=32, type=Type.ID, version=19)
b = Argon2id(salt=salt, length=32, iterations=2, lanes=1, memory_cost=256).derive(pw)
assert a == b
print(a.hex())
```
