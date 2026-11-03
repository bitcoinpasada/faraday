"""Stands in for the base32 C module of krux's firmware (`import base32`),
which `bbqr.py` calls as `base32.encode(data, False)` (text, no padding)
and `base32.decode(text)` (bytes, text padded to a multiple of 8).
Written for the cross-check, not krux's; it is RFC 4648 over the
standard library, upper case only.
"""

import base64


def encode(data, padding=True):
    text = base64.b32encode(bytes(data)).decode()
    return text if padding else text.rstrip("=")


def decode(text):
    return base64.b32decode(text)
