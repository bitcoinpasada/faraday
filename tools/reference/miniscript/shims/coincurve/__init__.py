"""Written here, not upstream: python-bip380 imports `coincurve` (a
package over libsecp256k1) for two key types, read and written as bytes,
and one tweak. This stands in for it over the BIPs repository's
`secp256k1lab` (`tools/reference/bip352/`), which the cross-check
already uses for the curve: only the calls python-bip380 makes.

- `PublicKey(bytes)`: a compressed key, refused when off the curve;
  `format()` gives its 33 bytes.
- `PublicKeyXOnly(bytes)`: an x-only key (BIP-340), refused when no point
  has that x; `format()` gives its 32 bytes, `parity` whether the point
  it stands for has an odd y, and `tweak_add(t)` replaces it with
  `P + t·G`, `P` the even-y point, as libsecp256k1's
  `secp256k1_xonly_pubkey_tweak_add` and coincurve's `tweak_add` do.
"""

from secp256k1lab.secp256k1 import FAST_G, GE

ORDER = GE.ORDER


class PublicKey:
    def __init__(self, data):
        if not isinstance(data, bytes) or len(data) != 33 or data[0] not in (2, 3):
            raise ValueError("not a compressed public key")
        try:
            self.point = GE.from_bytes_compressed(data)
        except (ValueError, AssertionError) as e:
            raise ValueError("a public key off the curve") from e

    def format(self, compressed=True):
        if not compressed:
            return self.point.to_bytes_uncompressed()
        return self.point.to_bytes_compressed()


class PublicKeyXOnly:
    def __init__(self, data, parity=False):
        if not isinstance(data, bytes) or len(data) != 32:
            raise ValueError("not an x-only public key")
        try:
            self.point = GE.from_bytes_xonly(data)
        except (ValueError, AssertionError) as e:
            raise ValueError("no point has this x") from e
        self.parity = parity

    def format(self):
        return self.point.to_bytes_xonly()

    def tweak_add(self, scalar):
        t = int.from_bytes(scalar, "big")
        if t >= ORDER:
            raise ValueError("a tweak not below the group order")
        q = self.point + FAST_G.mul(t)
        if q.infinity:
            raise ValueError("the tweak gives the point at infinity")
        self.parity = not q.has_even_y()
        self.point = GE.from_bytes_xonly(q.to_bytes_xonly())
