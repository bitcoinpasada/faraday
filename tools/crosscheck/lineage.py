"""The second curve lineage (F160 item 7, docs/CROSSCHECK.md §10.4).

Core's `key.py`, `secp256k1lab` and libsecp256k1 (Faraday's own signer)
share their authors. Beside them, two implementations from elsewhere
check every signature an area makes:

- python-ecdsa (Brian Warner, Hubert Kario; the distribution's
  `python3-ecdsa`), pure Python, RFC 6979 deterministic ECDSA over
  secp256k1: the signature under the first nonce byte for byte (low S,
  DER), and the verification of every other one.
- btclib (vendored wheels, its own pure-Python curve, libsecp256k1 off):
  every BIP-340 signature verified, and made again byte for byte where
  the auxiliary randomness is known.

Both are asked in addition to the older references, never instead.
"""

import hashlib

from common import HarnessError

BTCLIB_WHEELS = [
    "typing_extensions-4.16.0-py3-none-any.whl",
    "ecc-2026.10.23-py3-none-any.whl",
    "btclib-2026.10.24-py3-none-any.whl",
    "btclib_wallet-2026.10.29-py3-none-any.whl",
]

try:
    import ecdsa
    from ecdsa.util import sigdecode_der, sigencode_der, sigencode_der_canonize
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the second-lineage checks need the distribution's python-ecdsa (Debian and Ubuntu: "
        "python3-ecdsa; Fedora: python3-ecdsa; Arch: python-ecdsa): "
        f"{e}") from e

from common import use_wheels  # noqa: E402

use_wheels("btclib", BTCLIB_WHEELS)
from ecc import curves  # noqa: E402

# btclib's own curve arithmetic, never libsecp256k1's.
curves.set_libsecp256k1_serving(serving=False)
if curves.is_libsecp256k1_serving():
    raise HarnessError("btclib would sign through libsecp256k1")

from ecc.ecc import ssa  # noqa: E402

N = ecdsa.SECP256k1.order


def der_of(compact):
    """The strict DER form of a 64-byte `r || s`."""
    return sigencode_der(int.from_bytes(compact[:32], "big"), int.from_bytes(compact[32:], "big"), N)


def ecdsa_first(d, digest):
    """Costs one elliptic-curve multiplication in pure Python: python-ecdsa's
    RFC 6979 signature of the 32-byte `digest` under the first nonce, low S,
    DER."""
    sk = ecdsa.SigningKey.from_secret_exponent(d, curve=ecdsa.SECP256k1, hashfunc=hashlib.sha256)
    return sk.sign_digest_deterministic(
        digest, hashfunc=hashlib.sha256, sigencode=sigencode_der_canonize)


def ecdsa_verifies(pub, digest, der):
    """Costs two elliptic-curve multiplications in pure Python: whether
    python-ecdsa accepts the strict DER signature `der` of `digest` under the
    compressed key `pub`, with a low S."""
    try:
        vk = ecdsa.VerifyingKey.from_string(pub, curve=ecdsa.SECP256k1, hashfunc=hashlib.sha256)
        r, s = sigdecode_der(der, N)
        if s > N // 2:
            return False
        return vk.verify_digest(der, digest, sigdecode=sigdecode_der)
    except (ecdsa.BadSignatureError, ecdsa.der.UnexpectedDER, ValueError, AssertionError):
        return False


def check_ecdsa(d, pub, digest, der, nonce):
    """Costs three elliptic-curve multiplications in pure Python: the DER
    signature `der` (no sighash byte) of `digest` by key `d`, as python-ecdsa
    has it. Under the first nonce it is python-ecdsa's own, byte for byte.
    Under the low-R rule it verifies, has a low R, and is python-ecdsa's own
    where the first nonce already gave a low R. Raises AssertionError."""
    first = ecdsa_first(d, digest)
    if nonce == "first":
        if der != first:
            raise AssertionError("python-ecdsa's RFC 6979 signature under the first nonce differs")
        return
    if not ecdsa_verifies(pub, digest, der):
        raise AssertionError("python-ecdsa does not verify the ECDSA signature")
    if der[3] >= 33:
        raise AssertionError("the ECDSA signature has a high R under the low-R rule")
    if first[3] < 33 and der != first:
        raise AssertionError("the first RFC 6979 nonce gave a low R, and not this signature")


def check_schnorr(seckey, msg, signer, sig, aux=bytes(32)):
    """Costs about three elliptic-curve multiplications in pure Python: the
    64-byte BIP-340 `sig` of `msg` under x-only key `signer`, as btclib has
    it: verified, and made again byte for byte by `seckey` with `aux`.
    Raises AssertionError."""
    if not ssa.verify_(msg, signer, sig):
        raise AssertionError("btclib does not verify the BIP-340 signature")
    if aux is None:
        return
    mine = ssa.sign_(msg, int.from_bytes(seckey, "big"), aux, verify=False).serialize()
    if mine != sig:
        raise AssertionError("btclib's BIP-340 signature with this auxiliary randomness differs")


def check_schnorr_verify(msg, signer, sig):
    """Costs two elliptic-curve multiplications in pure Python: btclib
    verifies the BIP-340 `sig`. Raises AssertionError."""
    if not ssa.verify_(msg, signer, sig):
        raise AssertionError("btclib does not verify the BIP-340 signature")
