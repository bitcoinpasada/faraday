"""Stands in for the QR encoder krux's firmware has as a C module
(`import qrcode`). Written for the cross-check, not krux's.

`qr.to_qr_codes` calls `qrcode.encode(part)` on each part's text and
yields the picture; the cross-check wants the text, so this returns it.
"""


def encode(data):
    return data
