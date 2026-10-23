"""Written here, not upstream: pycoin's 32-byte big-endian conversions,
which BIP-85's reference imports (see `../symbols/btc.py`)."""


def from_bytes_32(b):
    if len(b) < 32:
        raise ValueError("fewer than 32 bytes")
    return int.from_bytes(b[:32], "big")


def to_bytes_32(v):
    return v.to_bytes(32, "big")
