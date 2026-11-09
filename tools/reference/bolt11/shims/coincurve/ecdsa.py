"""The `coincurve.ecdsa` calls `bolt11` makes (see `__init__.py`): the
recoverable signature as a tuple (r, s, recovery id), then plain, then DER."""

from ecc.ecc import dsa


def deserialize_recoverable(data):
    if len(data) != 65:
        raise ValueError("a recoverable signature is 65 bytes")
    return int.from_bytes(data[:32], "big"), int.from_bytes(data[32:64], "big"), data[64]


def recoverable_convert(cdata):
    return cdata[0], cdata[1]


def cdata_to_der(cdata):
    return dsa.Sig(cdata[0], cdata[1]).serialize()
