"""Stands in for the firmware's `uhashlib_hw` module, which krux's `kef.py`
uses for `sha256(data).digest()` and `pbkdf2_hmac_sha256(key, salt,
iterations)` (32 bytes). Written for the cross-check, not krux's, over the
standard library's `hashlib` (OpenSSL's SHA-256 and PBKDF2)."""

import hashlib


def sha256(data=b""):
    return hashlib.sha256(data)


def pbkdf2_hmac_sha256(key, salt, iterations):
    return hashlib.pbkdf2_hmac("sha256", key, salt, iterations)
