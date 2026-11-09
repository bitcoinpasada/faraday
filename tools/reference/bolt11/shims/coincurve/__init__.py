"""A stand-in for the `coincurve` package, which `bolt11` imports and which
is a C extension (libsecp256k1's binding). Written for the cross-check, not
upstream: only the calls `bolt11/models/signature.py` makes, each handed
to btclib's `ecc` on its own pure-Python curve. The area sets that up
before this is imported (`tools/crosscheck/area_lightning.py`).

Like libsecp256k1, it refuses a signature whose s is above half the order
(a high-S signature) when verifying, not when recovering.
"""

from hashlib import sha256

from ecc.curves import bytes_from_point, secp256k1
from ecc.ecc import dsa
from ecc.exceptions import EccValueError

HALF_N = secp256k1.n // 2


def _check(sig):
    if sig.s > HALF_N:
        raise ValueError("high-S signature")


class PrivateKey:
    def __init__(self, secret):
        self.secret = int.from_bytes(secret, "big")

    @classmethod
    def from_hex(cls, hexstr):
        return cls(bytes.fromhex(hexstr))

    def sign_recoverable(self, message):
        """The 65 bytes r || s || recovery id, low S, over SHA-256 of the message."""
        sig, key_id = dsa.sign_recoverable(message, self.secret, lower_s=True)
        return sig.r.to_bytes(32, "big") + sig.s.to_bytes(32, "big") + bytes([key_id])


class PublicKey:
    def __init__(self, point):
        self.point = point

    @classmethod
    def from_signature_and_message(cls, signature, message):
        if len(signature) != 65 or signature[64] > 3:
            raise ValueError("bad recoverable signature")
        sig = dsa.Sig(int.from_bytes(signature[:32], "big"), int.from_bytes(signature[32:64], "big"))
        try:
            return cls(dsa.recover_pub_key(signature[64], message, sig))
        except (EccValueError, ValueError) as exc:
            raise ValueError("no key recovers") from exc

    def format(self, compressed=True):
        return bytes_from_point(self.point, secp256k1, compressed)


def verify_signature(der, message, public_key):
    try:
        sig = dsa.Sig.parse(der)
        _check(sig)
        return dsa.verify(message, public_key, sig)
    except (EccValueError, ValueError):
        return False
