"""Stands in for the packages SeedSigner's `decode_qr.py` and
`encode_qr.py` import at the top and the SeedQR, numbered-part and
mnemonic code never calls: a camera reader (pyzbar), the UR type tables
(urtypes), the QR image library and Pillow, and embit's key, PSBT and
network modules. Written for the cross-check, not SeedSigner's.

Every name read from one of these modules is a class that refuses to be
made or called, so a path that did reach one would fail instead of
answering. embit's `bip39` and `misc` are not here: the real ones are
vendored (`tools/reference/embit/`).
"""

import sys
import types

NAMES = [
    "pyzbar", "pyzbar.pyzbar",
    "urtypes", "urtypes.crypto", "urtypes.bytes",
    "qrcode", "qrcode.constants", "qrcode.image", "qrcode.image.styledpil",
    "qrcode.image.styles", "qrcode.image.styles.moduledrawers",
    "PIL", "PIL.Image", "PIL.ImageDraw",
    "embit.bip32", "embit.bip85", "embit.networks", "embit.psbt",
    "embit.util",
]


class _Unavailable:
    def __init__(self, *a, **k):
        raise RuntimeError("an outside package the cross-check does not provide was called")


class _Stub(types.ModuleType):
    __path__ = []

    def __getattr__(self, name):
        if name.startswith("__"):
            raise AttributeError(name)
        value = _Unavailable if name[:1].isupper() else {}
        setattr(self, name, value)
        return value


def install():
    for name in NAMES:
        if name not in sys.modules:
            sys.modules[name] = _Stub(name)
