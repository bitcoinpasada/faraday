"""Signed messages against BIP-137 and BIP-322 "simple" written here from
the BIPs' text, over Bitcoin Core's functional-test framework
(`core/test_framework/`) for BIP-322's virtual transactions and their
signature hashes, `secp256k1lab`'s BIP-340 for taproot, and ECDSA with
RFC 6979 written here as libsecp256k1 states it (the message hash
reduced mod n, 32 bytes of extra data appended for Bitcoin Core's
low-R grinding).

- Published: BIP-322's message hashes and virtual txids, its simple
  signatures (verified, and made again where the BIP's key and nonce
  allow), its error cases; Trezor's and Electrum's BIP-137 signatures
  (`tools/vectors/message/`), verified, and made again from their keys.
- Random keys, every address form, mainnet and test, random messages
  (empty, long, any Unicode): the signature made by both byte for byte
  under both nonce rules (RFC 6979's first nonce, and Core's low-R
  grind) and, for taproot, zero or fresh auxiliary randomness; then
  checked by both against its address, a changed message, another
  key's address, a byte of it changed and its header moved.

BIP-322's own rule is kept: a signature uses SIGHASH_ALL, or for
taproot SIGHASH_DEFAULT. Faraday's verifier is more lenient than
BIP-137 in two ways it documents (`faraday_psbt::message`): it reads the
address form from the address, not from the header's range, and tries a
P2PKH address in both key encodings. Cases where that changes the
answer are compared with that reading and counted as known differences.
"""

import base64
import hashlib
import hmac
import json

import btc
from area_addr import HRP, P2PKH, P2SH, p2tr_bip86, tagged
from common import ROOT, Area, Mismatch, use_reference

use_reference("core")
import test_framework.script as S  # noqa: E402
from test_framework.address import base58_to_byte, byte_to_base58  # noqa: E402
from test_framework import segwit_addr  # noqa: E402
from test_framework.messages import COutPoint, CTransaction, CTxIn, CTxOut  # noqa: E402

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

use_reference("bip352", "secp256k1lab", "src")
from secp256k1lab.bip340 import schnorr_sign, schnorr_verify  # noqa: E402

N = btc.N
P_FIELD = 2**256 - 2**32 - 977
SCRIPTS = ["legacy", "nested-segwit", "native-segwit", "taproot"]
BASE = {0: 31, 1: 35, 2: 39}


class Refused(Exception):
    pass


# ----- ECDSA as libsecp256k1 makes it ---------------------------------------


def rfc6979(d, h, extra=None):
    """RFC 6979's nonce for key `d` and hash `h` (reduced mod n), with 32
    bytes of extra data appended to the key and hash, as libsecp256k1's
    nonce function does."""
    seed = d.to_bytes(32, "big") + (int.from_bytes(h, "big") % N).to_bytes(32, "big")
    if extra is not None:
        seed += extra
    v, k = b"\x01" * 32, b"\x00" * 32
    k = hmac.new(k, v + b"\x00" + seed, hashlib.sha256).digest()
    v = hmac.new(k, v, hashlib.sha256).digest()
    k = hmac.new(k, v + b"\x01" + seed, hashlib.sha256).digest()
    v = hmac.new(k, v, hashlib.sha256).digest()
    while True:
        v = hmac.new(k, v, hashlib.sha256).digest()
        t = int.from_bytes(v, "big")
        if 0 < t < N:
            return t
        k = hmac.new(k, v + b"\x00", hashlib.sha256).digest()
        v = hmac.new(k, v, hashlib.sha256).digest()


def ecdsa_sign(d, h, extra=None):
    """(r, s, recovery id), s low."""
    k = rfc6979(d, h, extra)
    R = btc.FAST_G.mul(k)
    r = int(R.x) % N
    s = pow(k, -1, N) * (int.from_bytes(h, "big") + r * d) % N
    recid = (1 if not R.has_even_y() else 0) | (2 if int(R.x) >= N else 0)
    if s > N // 2:
        s, recid = N - s, recid ^ 1
    return r, s, recid


def sign_ground(d, h, low_r):
    sig = ecdsa_sign(d, h)
    counter = 0
    while low_r and sig[0] >= 1 << 255:
        counter += 1
        sig = ecdsa_sign(d, h, counter.to_bytes(4, "little") + bytes(28))
    return sig


def recover(h, r, s, recid):
    """The public key a recoverable signature was made by."""
    if not (0 < r < N and 0 < s < N):
        raise Refused("r or s out of range")
    x = r + (N if recid & 2 else 0)
    if x >= P_FIELD:
        raise Refused("no point")
    try:
        R = btc.GE.lift_x(x)
    except (ValueError, AssertionError) as e:
        raise Refused("no point") from e
    if int(R.x) != x:
        raise Refused("no point")
    if (recid & 1) != (0 if R.has_even_y() else 1):
        R = -R
    rinv = pow(r, -1, N)
    e = int.from_bytes(h, "big") % N
    Q = btc.FAST_G.mul((-e * rinv) % N) + _mul(R, s * rinv % N)
    if Q.infinity:
        raise Refused("infinity")
    return Q


def _mul(P, k):
    out = btc.GE()
    for bit in bin(k)[2:]:
        out = out + out
        if bit == "1":
            out = out + P
    return out


def ecdsa_verify(h, r, s, Q):
    """ECDSA verification as libsecp256k1 does it: an s above n/2 is
    refused, so that a signature has one form."""
    if not (0 < r < N and 0 < s <= N // 2):
        return False
    w = pow(s, -1, N)
    e = int.from_bytes(h, "big") % N
    R = btc.FAST_G.mul(e * w % N) + _mul(Q, r * w % N)
    return not R.infinity and int(R.x) % N == r


def der(r, s):
    def num(x):
        b = x.to_bytes((x.bit_length() + 7) // 8, "big")
        return b"\0" + b if b[0] & 0x80 else b
    body = b"\x02" + bytes([len(num(r))]) + num(r) + b"\x02" + bytes([len(num(s))]) + num(s)
    return b"\x30" + bytes([len(body)]) + body


def parse_der(sig):
    """Strict DER, r and s: BIP-66."""
    if len(sig) < 8 or len(sig) > 72 or sig[0] != 0x30 or sig[1] != len(sig) - 2:
        raise Refused("not DER")
    at, nums = 2, []
    for _ in range(2):
        if at + 2 > len(sig) or sig[at] != 0x02:
            raise Refused("not DER")
        n = sig[at + 1]
        v = sig[at + 2:at + 2 + n]
        if n == 0 or len(v) != n or v[0] & 0x80 or (n > 1 and v[0] == 0 and not v[1] & 0x80):
            raise Refused("not DER")
        nums.append(int.from_bytes(v, "big"))
        at += 2 + n
    if at != len(sig):
        raise Refused("not DER")
    return nums


# ----- addresses --------------------------------------------------------------


def address_of(pub, script, network):
    h = btc.hash160(pub)
    if script == 0:
        return byte_to_base58(h, P2PKH[network])
    if script == 1:
        return byte_to_base58(btc.hash160(b"\x00\x14" + h), P2SH[network])
    if script == 2:
        return segwit_addr.encode_segwit_address(HRP[network], 0, h)
    return segwit_addr.encode_segwit_address(HRP[network], 1, p2tr_bip86(pub))


def address_script(address):
    """The output script of a mainnet or test address."""
    for hrp in ("bc", "tb", "bcrt"):
        v, prog = segwit_addr.decode_segwit_address(hrp, address)
        if v is not None:
            return bytes([0x50 + v if v else 0, len(prog)]) + bytes(prog)
    payload, version = base58_to_byte(address)
    if version in (0, 111):
        return b"\x76\xa9\x14" + payload + b"\x88\xac"
    if version in (5, 196):
        return b"\xa9\x14" + payload + b"\x87"
    raise Refused("not an address")


# ----- BIP-137 ------------------------------------------------------------------


def varstr(b):
    return bytes([len(b)]) + b if len(b) < 0xfd else b"\xfd" + len(b).to_bytes(2, "little") + b


def bip137_hash(message):
    m = message.encode()
    prefix = b"\x18Bitcoin Signed Message:\n"
    if len(m) < 0xfd:
        size = bytes([len(m)])
    elif len(m) <= 0xFFFF:
        size = b"\xfd" + len(m).to_bytes(2, "little")
    else:
        size = b"\xfe" + len(m).to_bytes(4, "little")
    return btc.hash256(prefix + size + m)


def bip137_sign(d, script, message, low_r):
    r, s, recid = sign_ground(d, bip137_hash(message), low_r)
    return base64.b64encode(bytes([BASE[script] + recid]) + r.to_bytes(32, "big")
                            + s.to_bytes(32, "big")).decode()


def bip137_verify(address, message, sig, lenient):
    """BIP-137: the header's range names the address form (27-30 P2PKH
    of an uncompressed key, 31-34 compressed, 35-38 P2SH-P2WPKH, 39-42
    P2WPKH). `lenient` reads it as Faraday does: the form from the
    address, a P2PKH address in both key encodings."""
    if len(sig) != 65 or not 27 <= sig[0] <= 42:
        raise Refused("Malformed")
    header = sig[0]
    if header < 35:
        recid, form, compressed = (header - 27) & 3, 0, header >= 31
    elif header < 39:
        recid, form, compressed = header - 35, 1, True
    else:
        recid, form, compressed = header - 39, 2, True
    r, s = int.from_bytes(sig[1:33], "big"), int.from_bytes(sig[33:], "big")
    try:
        Q = recover(bip137_hash(message), r, s, recid)
    except Refused as e:
        raise Refused("Malformed") from e
    spk = address_script(address)
    if lenient:
        if spk[:3] == b"\x76\xa9\x14":
            form = 0
            keys = [Q.to_bytes_compressed(), Q.to_bytes_uncompressed()]
        elif spk[:2] == b"\xa9\x14":
            form, keys = 1, [Q.to_bytes_compressed()]
        elif spk[:2] == b"\x00\x14":
            form, keys = 2, [Q.to_bytes_compressed()]
        else:
            raise Refused("UnsupportedAddress")
    else:
        if not (spk[:3] == b"\x76\xa9\x14" or spk[:2] in (b"\xa9\x14", b"\x00\x14")):
            raise Refused("UnsupportedAddress")
        keys = [Q.to_bytes_compressed() if compressed else Q.to_bytes_uncompressed()]
    for key in keys:
        h = btc.hash160(key)
        want = [b"\x76\xa9\x14" + h + b"\x88\xac",
                b"\xa9\x14" + btc.hash160(b"\x00\x14" + h) + b"\x87",
                b"\x00\x14" + h][form]
        if want == spk:
            return {"format": "bip137", "script": SCRIPTS[form]}
    raise Refused("Invalid")


# ----- BIP-322 simple -----------------------------------------------------------


def message_hash(message):
    return tagged("BIP0322-signed-message", message.encode())


def to_spend(spk, message):
    tx = CTransaction()
    tx.version = 0
    tx.vin.append(CTxIn(COutPoint(0, 0xFFFFFFFF), b"\x00\x20" + message_hash(message), 0))
    tx.vout.append(CTxOut(0, spk))
    return tx


def to_sign(spend, witness=None):
    tx = CTransaction()
    tx.version = 0
    tx.vin.append(CTxIn(COutPoint(spend.txid_int, 0), b"", 0))
    tx.vout.append(CTxOut(0, b"\x6a"))
    return tx


def ser_witness(stack):
    return bytes([len(stack)]) + b"".join(varstr(x) for x in stack)


def read_witness(raw):
    if not raw:
        raise Refused("Malformed")
    n, at, out = raw[0], 1, []
    if n >= 0xfd:
        raise Refused("Malformed")
    for _ in range(n):
        if at >= len(raw):
            raise Refused("Malformed")
        m = raw[at]
        if m >= 0xfd:
            if raw[at] != 0xfd or at + 3 > len(raw):
                raise Refused("Malformed")
            m, at = int.from_bytes(raw[at + 1:at + 3], "little"), at + 3
            if m < 0xfd:
                raise Refused("Malformed")
        else:
            at += 1
        if at + m > len(raw):
            raise Refused("Malformed")
        out.append(raw[at:at + m])
        at += m
    if at != len(raw):
        raise Refused("Malformed")
    return out


def tweaked_secret(d):
    P = btc.FAST_G.mul(d)
    if not P.has_even_y():
        d = N - d
    t = int.from_bytes(tagged("TapTweak", P.to_bytes_xonly()), "big")
    return (d + t) % N


def bip322_sign(d, address, message, low_r, aux):
    spk = address_script(address)
    spend = to_spend(spk, message)
    sign = to_sign(spend)
    if spk[:2] == b"\x00\x14":
        h = S.SegwitV0SignatureHash(S.CScript(b"\x76\xa9\x14" + spk[2:] + b"\x88\xac"), sign, 0,
                                    S.SIGHASH_ALL, 0)
        r, s, _ = sign_ground(d, h, low_r)
        stack = [der(r, s) + b"\x01", btc.point(d)]
    elif spk[:2] == b"\x51\x20":
        h = S.TaprootSignatureHash(sign, [spend.vout[0]], 0, 0)
        stack = [schnorr_sign(h, tweaked_secret(d).to_bytes(32, "big"), aux)]
    else:
        raise Refused("UnsupportedAddress")
    return base64.b64encode(ser_witness(stack)).decode()


def bip322_verify(address, message, raw):
    stack = read_witness(raw)
    spk = address_script(address)
    spend = to_spend(spk, message)
    sign = to_sign(spend)
    if spk[:2] == b"\x00\x14":
        if len(stack) != 2:
            raise Refused("Malformed")
        sig, key = stack
        try:
            Q = btc.GE.from_bytes_compressed(key) if len(key) == 33 else None
        except (ValueError, AssertionError):
            Q = None
        if Q is None:
            raise Refused("Malformed")
        if b"\x00\x14" + btc.hash160(key) != spk:
            raise Refused("Invalid")
        if not sig:
            raise Refused("Malformed")
        try:
            r, s = parse_der(sig[:-1])
        except Refused as e:
            raise Refused("Malformed") from e
        if sig[-1] not in (1, 2, 3, 0x81, 0x82, 0x83):
            raise Refused("Malformed")
        if sig[-1] != S.SIGHASH_ALL:
            raise Refused("Invalid")
        h = S.SegwitV0SignatureHash(S.CScript(b"\x76\xa9\x14" + spk[2:] + b"\x88\xac"), sign, 0,
                                    S.SIGHASH_ALL, 0)
        if not ecdsa_verify(h, r, s, Q):
            raise Refused("Invalid")
        return {"format": "bip322", "script": "native-segwit"}
    if spk[:2] == b"\x51\x20":
        if len(stack) != 1:
            raise Refused("Malformed")
        sig = stack[0]
        if len(sig) == 65 and sig[64] in (1, 2, 3, 0x81, 0x82, 0x83):
            raise Refused("Invalid")
        if len(sig) != 64:
            raise Refused("Malformed")
        try:
            btc.GE.from_bytes_xonly(spk[2:])
        except (ValueError, AssertionError) as e:
            raise Refused("UnsupportedAddress") from e
        h = S.TaprootSignatureHash(sign, [spend.vout[0]], 0, 0)
        if not schnorr_verify(h, spk[2:], sig):
            raise Refused("Invalid")
        return {"format": "bip322", "script": "taproot"}
    raise Refused("UnsupportedAddress")


def ref_verify(address, message, signature, lenient=False):
    text = signature.strip()
    prefixed = text[:3] == "smp"
    if text[:3] in ("ful", "pof"):
        raise Refused("UnsupportedFormat")
    try:
        raw = base64.b64decode(text[3:] if prefixed else text, validate=True)
    except Exception as e:  # noqa: BLE001 - not base64
        raise Refused("Malformed") from e
    if not prefixed and len(raw) == 65:
        return bip137_verify(address, message, raw, lenient)
    return bip322_verify(address, message, raw)


def refusal(answer):
    """A refusal as its kind: `Invalid`, `Malformed`, `UnsupportedAddress`
    or `UnsupportedFormat`."""
    if answer[0] == "ok":
        return answer
    reason = answer[1].removeprefix("Refused: ")
    return ("ok", {"refused": reason.split(":")[0].split("(")[0].strip()})


def aux_for(fresh):
    """The 32 bytes BIP-340 receives: zero, or HKDF-Expand (RFC 5869)
    from the call's bytes for input 0, the first block, written out over
    Python's `hmac` (`faraday_psbt::sign::Aux::for_input(0)`)."""
    if fresh is None:
        return bytes(32)
    info = b"faraday-schnorr-aux" + (0).to_bytes(8, "little")
    return hmac.new(fresh, info + b"\x01", hashlib.sha256).digest()


def wif_secret(wif):
    raw = btc.b58check_decode(wif)
    return int.from_bytes(raw[1:33], "big")


class Msg(Area):
    name = "msg"
    faraday = (
        '`faraday_psbt::message` (BIP-137, BIP-322 simple: sign, verify, virtual txids)'
    )
    reference = (
        'BIP-137 and BIP-322 written out in `area_msg.py`: ECDSA with RFC 6979 as '
        "libsecp256k1 states it, Core's `script.py` sighashes, `secp256k1lab`'s BIP-340"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; BIP-322's hashes, txids, simple signatures and error cases and "
        "Trezor's and Electrum's BIP-137 signatures are required of it"
    )
    vectors = (
        "BIP-322's hashes, txids, simple signatures and error cases; Trezor's and "
        "Electrum's BIP-137 signatures (`tools/vectors/message/`), made again from "
        "Trezor's keys"
    )
    corpus = (
        'random keys, address forms, networks and messages: signatures made byte for '
        'byte under both nonce rules and both aux choices, checked, then checked '
        'against a changed message, another address, a changed byte and a moved header'
    )
    differences = [
        (
            'BIP-137: Faraday reads the address form from the address rather than the '
            "header's range and tries a P2PKH address in both key encodings "
            '(`faraday_psbt::message`), as Electrum and Sparrow do; BIP-137 read strictly '
            'refuses Trezor\'s and Electrum\'s "no script type" signatures.'
        ),
    ]
    not_compared = (
        'none'
    )

    def sign(self, d, script, network, message, fmt, nonce, fresh=None, expected=None):
        def ref():
            pub = btc.point(d)
            address = address_of(pub, script, network)
            if fmt == "bip137":
                if script == 3:
                    raise Refused("UnsupportedFormat")
                sig = bip137_sign(d, script, message, nonce == "lowr")
            else:
                if script in (0, 1):
                    raise Refused("UnsupportedFormat")
                sig = bip322_sign(d, address, message, nonce == "lowr", aux_for(fresh))
            return {"address": address, "signature": sig}
        return self.compare("msg.sign", {"secret": d.to_bytes(32, "big").hex(), "script": script,
                                         "network": network, "message": message, "format": fmt,
                                         "nonce": nonce,
                                         "aux": fresh.hex() if fresh is not None else None},
                            ref, expected)

    def verify(self, address, message, signature, expected=None):
        inputs = {"address": address, "message": message, "signature": signature}
        strict = refusal(_try(lambda: ref_verify(address, message, signature)))
        lenient = refusal(_try(lambda: ref_verify(address, message, signature, True)))
        if strict == lenient:
            return self.compare("msg.verify", inputs, lambda: ref_verify(address, message, signature),
                                expected, normalize=refusal)
        rust = refusal(self.rust.call("msg.verify", **inputs))
        self.compared += 1
        self.known += 1
        if rust != lenient:
            raise Mismatch(self.name, "msg.verify", inputs, rust, strict,
                           f"the known difference (BIP-137 read leniently) no longer holds; "
                           f"the lenient reading is {lenient}")
        return rust

    def bip322_ids(self, address, message, expected=None):
        def ref():
            spend = to_spend(address_script(address), message)
            return {"message_hash": message_hash(message).hex(), "to_spend": spend.txid_hex,
                    "to_sign": to_sign(spend).txid_hex}
        return self.compare("msg.bip322", {"address": address, "message": message}, ref,
                            expected)

    def published(self):
        v = json.loads((ROOT / "tools/vectors/message/bip322-basic.json").read_text())
        for case in v["tx_hashes"]:
            self.bip322_ids(case["address"], case["message"],
                            ("ok", {"message_hash": case["message_hash"],
                                    "to_spend": case["to_spend_tx_hash"],
                                    "to_sign": case["to_sign_tx_hash"]}))
        for case in v["simple"]:
            for sig in case["bip322_signatures"]:
                want = {"p2wpkh": ("ok", {"format": "bip322", "script": "native-segwit"}),
                        "p2tr": ("ok", {"format": "bip322", "script": "taproot"})}.get(
                            case["type"], ("ok", {"refused": "UnsupportedAddress"}))
                self.verify(case["address"], case["message"], sig, want)
        for case in v["error"]:
            got = self.verify(case["address"], case["message"], case["signature"])
            if got[0] == "ok" and "refused" not in got[1]:
                raise AssertionError(f"BIP-322's error case read as valid: {case['description']}")
        self.bip137_published()

    def bip137_published(self):
        text = (ROOT / "tools/vectors/message/bip137.txt").read_text()
        for block in text.split("\n\n"):
            rec = dict(l.split(": ", 1) for l in block.splitlines()
                       if ": " in l and not l.startswith("#"))
            if "signature-hex" in rec:
                sig = base64.b64encode(bytes.fromhex(rec["signature-hex"])).decode()
            elif "signature-base64" in rec:
                sig = rec["signature-base64"]
            else:
                continue
            self.verify(rec["address"], rec["message"], sig)
            # Trezor signs with RFC 6979's first nonce: the same key signs
            # the same bytes on both sides.
            script = {"legacy": 0, "nested-segwit": 1, "native-segwit": 2}.get(rec.get("script"))
            if "words" in rec and rec.get("no-script-type") != "yes" and script is not None:
                seed = Mnemonic.to_seed(rec["words"], "")
                key = btc.master(seed).derive(btc.parse_path(rec["path"]))
                network = "mainnet" if rec["address"][0] in "13b" else "testnet"
                self.sign(key.k, script, network, rec["message"], "bip137", "first",
                          expected=("ok", {"address": rec["address"], "signature": sig}))

    def corpus_case(self):
        rng = self.rng
        d = rng.randrange(1, N)
        script = rng.randrange(4)
        network = rng.choice(["mainnet", "testnet", "signet", "regtest"])
        message = rng.choice(["", "Hello World", "x" * rng.randint(0, 300),
                              "".join(rng.choice("aé日🙂\n ") for _ in range(rng.randint(1, 40)))])
        fmt = "bip322" if script == 3 else ("bip137" if script < 2 else rng.choice(["bip137", "bip322"]))
        if rng.random() < 0.05:
            fmt = "bip322" if fmt == "bip137" else "bip137"
        nonce = rng.choice(["lowr", "first"])
        fresh = self.randbytes(32) if script == 3 and rng.random() < 0.5 else None
        got = self.sign(d, script, network, message, fmt, nonce, fresh)
        if got[0] != "ok":
            return
        address, sig = got[1]["address"], got[1]["signature"]
        self.verify(address, message, sig)
        self.verify(address, message + rng.choice(["!", " ", "\n"]), sig)
        other = address_of(btc.point(rng.randrange(1, N)), rng.randrange(4), network)
        self.verify(other, message, sig)
        raw = bytearray(base64.b64decode(sig))
        i = rng.randrange(len(raw))
        raw[i] ^= 1 << rng.randrange(8)
        self.verify(address, message, base64.b64encode(bytes(raw)).decode())
        if fmt == "bip137":
            raw = bytearray(base64.b64decode(sig))
            raw[0] = rng.choice([27, 28, 31, 32, 35, 36, 39, 40, 43, 26, raw[0] ^ 4])
            self.verify(address, message, base64.b64encode(bytes(raw)).decode())
        else:
            self.verify(address, message, "smp" + sig)

    def found(self):
        """Seed 17af375c…'s disagreement, every run: a P2WPKH witness whose
        `s` is written as a negative integer is not BIP-66 DER, so it is
        malformed (libsecp256k1's parser reads it as zero); the same
        numbers written as DER are well formed and invalid."""
        address = "bcrt1qup7h6dnv9nxpu2tzkr3cekdzqa0lkt5sfsa3pq"
        for sig, want in [
            ("AkcwRAIgTgR9Hb+lhM3ejBtyKxwGvimMnWBuSR5GbFjdacdJMjkCINvePNT/c/qRKUGZnJP1qPNFrbz+"
             "ZayRuNK46Fzltgl6ASEDNR2yiaxyEmvCy3aI/aT8NE1xJBMH6I/41yd1GUs3s80=", "Malformed"),
            ("AkgwRQIgTgR9Hb+lhM3ejBtyKxwGvimMnWBuSR5GbFjdacdJMjkCIQDb3jzU/3P6kSlBmZyT9ajzRa28/"
             "mWskbjSuOhc5bYJegEhAzUdsomschJrwst2iP2k/DRNcSQTB+iP+NcndRlLN7PN", "Invalid"),
        ]:
            self.verify(address, "Hello World", sig, ("ok", {"refused": want}))

    LOOPS = (("corpus", 600),)

    def prelude(self):
        self.published()
        self.found()

    def case_corpus(self):
        self.corpus_case()


def _try(fn):
    try:
        return ("ok", fn())
    except Exception as e:  # noqa: BLE001 - every refusal is a refusal
        return ("err", f"{type(e).__name__}: {e}")
