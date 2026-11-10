"""RIPEMD-160 standing in for `Cryptodome.Hash.RIPEMD160.new()`, which
python-opentimestamps' `OpRIPEMD160` falls back to when `hashlib` has no
`ripemd160` (OpenSSL 3 without its legacy provider). The hash is the
pure-Python one in Bitcoin Core's test framework, vendored under
`tools/reference/core/`, which the area puts on the import path first."""

from test_framework.crypto.ripemd160 import ripemd160


class _Hash:
    def __init__(self):
        self._data = b""

    def update(self, data):
        self._data += bytes(data)
        return self

    def digest(self):
        return ripemd160(self._data)


def new(data=b""):
    return _Hash().update(data)
