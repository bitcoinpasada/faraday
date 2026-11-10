"""The four names `opentimestamps/core/timestamp.py` imports from
python-bitcoinlib, which is not vendored. Only `Timestamp.str_tree`, a
printing routine the area never calls, uses them; `CTransaction` here
reads nothing, so a message is never taken for a transaction."""

import binascii


class SerializationError(Exception):
    pass


class CTransaction:
    @staticmethod
    def deserialize(_data):
        raise SerializationError("python-bitcoinlib is not vendored")


def b2x(b):
    return binascii.hexlify(b).decode("ascii")


def b2lx(b):
    return binascii.hexlify(b[::-1]).decode("ascii")
