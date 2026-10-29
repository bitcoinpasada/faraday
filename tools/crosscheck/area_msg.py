"""Signed messages against code Faraday did not write (F160): Bitcoin
Core and btclib for BIP-137, btclib-wallet and btcd for BIP-322.

- BIP-137. Bitcoin Core v31.1 itself: `signmessagewithprivkey` makes every
  signature Faraday makes with RFC 6979's first nonce again, byte for
  byte (its r, s and recovery id; Core writes the P2PKH header), and
  `verifymessage` judges every P2PKH case, asked in one RPC batch per
  batch of cases. btclib's `bms` (BIP-137 with Electrum's header ranges)
  signs and verifies every address form; Faraday's low-R signature is
  btclib's own low-R signature (`dsa.sign`, Core's grind) of the same
  hash, with the header btclib's verifier finds.
- BIP-322. btclib-wallet's `bip322` (Python, over btclib's own curve, not
  libsecp256k1) and btcd's `bip322` from pull request 2521 (Go, the
  implementation the BIP names, which generated its vectors), each
  asked every question: the message hash and the virtual txids, every
  signature Faraday makes (made again byte for byte where the nonce is
  RFC 6979's first, as both make it; Core's `signrawtransactionwithkey`
  makes the low-R P2WPKH witness again), and every signature Faraday
  checks. Their own signatures, btcd's taproot ones included, are put
  to Faraday.
- Published: BIP-322's basic and generated vectors and btcd's negative
  vectors (`tools/vectors/message/`), required of Faraday and of both
  references; Trezor's and Electrum's BIP-137 signatures, verified, and
  Trezor's made again from its keys (btclib-wallet's BIP-32).
- Random keys, every address form, mainnet and test, random messages:
  each signature then checked against a changed message, another key's
  address, a byte of it changed, and its header moved or a prefix added.

The verdict is compared: valid, with the format and the address form
Faraday names, or refused. The kind of a refusal is Faraday's own word
for it and is not compared. What Faraday does not support (BIP-322's full
form and proof of funds, script addresses) and its lenient BIP-137 reading
are counted as known differences, each with the references' answer
required.
"""

import base64
import json
import os
import subprocess

import core_node
from common import ROOT, Area, HarnessError, Mismatch, use_reference, use_wheels

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

import lineage

use_wheels("btclib", lineage.BTCLIB_WHEELS)
from ecc import curves  # noqa: E402

# btclib's own curve arithmetic, never libsecp256k1's: that is what makes
# it a second lineage (F160 item 7).
curves.set_libsecp256k1_serving(serving=False)
if curves.is_libsecp256k1_serving():
    raise HarnessError("btclib would sign through libsecp256k1")

from btclib import b32, b58  # noqa: E402
from btclib.ecc import bms  # noqa: E402
from btclib.exceptions import InconclusiveError  # noqa: E402
from btclib.hashes import magic_message  # noqa: E402
from btclib.key import PrvKeyData, PubKeyData  # noqa: E402
from btclib.script.script_pub_key import ScriptPubKey, type_and_payload  # noqa: E402
from btclib.script.taproot import output_pubkey  # noqa: E402
from btclib.script.witness import Witness  # noqa: E402
from btclib.tx import Tx  # noqa: E402
from btclib_wallet import bip32, bip322  # noqa: E402
from ecc.ecc import dsa  # noqa: E402

N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
SCRIPTS = ["legacy", "nested-segwit", "native-segwit", "taproot"]
# The address form btclib reads off an address, as Faraday names it.
FORM = {"p2pkh": "legacy", "p2sh": "nested-segwit", "p2wpkh": "native-segwit",
        "p2tr": "taproot"}
KIND = ["p2pkh", "p2sh-p2wpkh", "p2wpkh", "p2tr"]
BASE = [31, 35, 39]
PREFIXES = ("smp", "ful", "pof")
# What btclib-wallet's verifier raises for an invalid signature.
INVALID = bip322._INVALID


# ----- kept for area_sign.py ---------------------------------------------------
#
# `area_sign.py` still takes its ECDSA from here; this area does not use
# these. They are Faraday's own (RFC 6979 as libsecp256k1 states it, the
# low-R grind) and go when `area_sign.py` takes outside code instead.

import hashlib  # noqa: E402
import hmac  # noqa: E402

import btc  # noqa: E402


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


def der(r, s):
    def num(x):
        b = x.to_bytes((x.bit_length() + 7) // 8, "big")
        return b"\0" + b if b[0] & 0x80 else b
    body = b"\x02" + bytes([len(num(r))]) + num(r) + b"\x02" + bytes([len(num(s))]) + num(s)
    return b"\x30" + bytes([len(body)]) + body


# ----- btcd -----------------------------------------------------------------------


def btcd_exe():
    """btcd's answer program, which `run.sh` builds from
    `tools/reference/btcd-bip322/` (vendored Go, offline)."""
    path = os.environ.get("FARADAY_BTCD_BIP322_REF") or str(ROOT / "out/crosscheck/btcd-bip322-ref")
    if not os.path.isfile(path):
        raise HarnessError(
            f"btcd's BIP-322 program is not built ({path}): tools/crosscheck/run.sh builds it "
            "from tools/reference/btcd-bip322/ with the Go toolchain")
    return path


class Btcd:
    """One process for the area's batches: a JSON array of requests a
    line, the answers' array back."""

    def __init__(self):
        self.proc = subprocess.Popen([btcd_exe()], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     text=True, bufsize=1)

    def ask(self, *requests):
        self.proc.stdin.write(json.dumps(list(requests)) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise HarnessError("btcd's BIP-322 program exited")
        return json.loads(line)

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


# ----- what the references say -------------------------------------------------


def key_of(d, network):
    return PrvKeyData(d, network, True)


def address_of(pub, script):
    """btclib's address of a public key, of each form."""
    if script == 0:
        return b58.p2pkh(pub)
    if script == 1:
        return b58.p2wpkh_p2sh(pub)
    if script == 2:
        return b32.p2wpkh(pub)
    return b32.p2tr(output_pubkey(PubKeyData(pub.sec, check_validity=False))[0], pub.network)


def kind_of(address):
    """The address's type as btclib reads it (`p2pkh`, `p2wsh`, …), or
    None for one it does not read."""
    try:
        return type_and_payload(ScriptPubKey.from_address(address).script)[0]
    except ValueError:
        return None


def regtest_p2pkh(address):
    """The P2PKH address of the same key hash on regtest, the network the
    run's Core node is on: what `verifymessage` checks is the hash."""
    return b58.address_from_h160("p2pkh", type_and_payload(
        ScriptPubKey.from_address(address).script)[1], "regtest")


def b64(raw):
    return base64.b64encode(raw).decode()


def bms_shaped(text):
    """Whether Faraday and btclib read the text as BIP-137: no BIP-322
    prefix and 65 bytes of base64 (btclib-wallet's own test)."""
    return text[:3] not in PREFIXES and bip322._is_bms(text)


def bms_valid(message, address, sig):
    try:
        return bms.verify(message.encode(), address, sig)
    except (ValueError, RuntimeError):
        return False


def btclib_state(message, address, sig):
    """btclib-wallet's BIP-322 verdict: valid, invalid or inconclusive."""
    try:
        bip322.assert_as_valid(message.encode(), address, sig)
    except InconclusiveError:
        return "inconclusive"
    except INVALID:
        return "invalid"
    return "valid"


def rust_verdict(answer):
    """Faraday's answer to msg.verify: ("valid", format, script) or
    ("refused", kind)."""
    if answer[0] == "ok":
        return ("valid", answer[1].get("format"), answer[1].get("script"))
    return ("refused", answer[1].removeprefix("Refused: ").split(":")[0].split("(")[0].strip())


class Msg(Area):
    name = "msg"
    CORE = True
    faraday = (
        '`faraday_psbt::message` (BIP-137, BIP-322 simple: sign, verify, virtual txids)'
    )
    reference = (
        "Bitcoin Core v31.1 and btclib for BIP-137, btclib-wallet and btcd for BIP-322: "
        "Core's `signmessagewithprivkey`, `verifymessage` and `signrawtransactionwithkey` "
        "(the low-R P2WPKH witness); btclib's `bms` and btclib-wallet's `bip322` "
        "(`btclib/`, btclib's own curve); python-ecdsa (the distribution's package) for the "
        "first-nonce ECDSA signature of BIP-137; btcd's `bip322` from pull request 2521 "
        "(`btcd-bip322/`, Go)"
    )
    reference_checked = (
        "the wheels and the Go tree are hashed against `references.sha256` before any run, "
        "Core's release against its pin; BIP-322's basic and generated vectors and btcd's "
        "negative vectors are required of both BIP-322 references, each in the state the "
        "vector names, and Trezor's and Electrum's BIP-137 signatures of btclib"
    )
    vectors = (
        "BIP-322's basic and generated vectors (hashes, txids, simple, full and proof-of-funds "
        "signatures, errors) and btcd's negative vectors; Trezor's and Electrum's BIP-137 "
        "signatures (`tools/vectors/message/`), Trezor's made again from its keys"
    )
    corpus = (
        'random keys, address forms, networks and messages: virtual txids; signatures '
        'made byte for byte under both nonce rules and checked by every reference, then '
        'checked against a changed message, another address, a changed byte and a moved '
        "header; btcd's own simple signatures checked by Faraday"
    )
    differences = [
        (
            'BIP-137: Faraday reads the address form from the address rather than the '
            "header's range and tries a P2PKH address in both key encodings "
            '(`faraday_psbt::message`), as Electrum and Sparrow do; btclib holds the ranges '
            "(with Electrum's for SegWit) and Core reads the key encoding from the header. "
            "Counted where btclib accepts the signature with the header the address's form "
            'names, or Core with the other key encoding.'
        ),
        (
            "BIP-322: Faraday refuses the full form and proof of funds, which both "
            'references make and accept (for P2PKH and P2SH-P2WPKH addresses they sign in '
            'the full form); counted where both accept and Faraday refuses the format.'
        ),
        (
            'BIP-322: Faraday checks simple signatures for P2WPKH and P2TR addresses only; a '
            'P2WSH script both references accept is refused as an unsupported address, and '
            'counted.'
        ),
        (
            "Between the references: btclib's interpreter holds witness v0 scripts alone to "
            "MINIMALIF, so it accepts btcd's two negative vectors of a legacy P2SH script, in "
            'the full form, whose `OP_IF` takes a non-minimal true; btcd must call them invalid '
            'and Faraday, which reads no full form, refuse them, and each is counted.'
        ),
    ]
    not_compared = (
        "a taproot signature's bytes (BIP-340's auxiliary randomness is the signer's: "
        "Faraday's is verified by both BIP-322 references, and btcd's by Faraday); the kind "
        "of a refusal (Malformed, Invalid, Unsupported…), Faraday's own words, checked by "
        "its tests; Core's reading of a BIP-137 header outside 27 to 42"
    )

    btcd_proc = None
    pending = None

    def btcd(self, *requests):
        if self.btcd_proc is None:
            self.btcd_proc = Btcd()
        return self.btcd_proc.ask(*requests)

    def close(self):
        if self.btcd_proc is not None:
            self.btcd_proc.close()
        super().close()

    # ----- Core, in one batch ----------------------------------------------

    def core(self, op, inputs, rust, calls, judge):
        """Asks Core later, with the rest of the batch: `calls` are
        (method, params), `judge(results)` returns None when Core agrees,
        else what Core said. A case's Core checks come after its own."""
        self.pending.append((self.case_at, op, inputs, rust, calls, judge))

    def flush(self, before=None):
        """Sends the batch's Core calls in one RPC batch and judges them in
        case order; with `before`, only those of earlier cases."""
        pending, self.pending = self.pending, []
        if before is not None:
            pending = [p for p in pending if p[0] != before]
        if not pending:
            return
        node = core_node.rpc()
        requests = [getattr(node, m).get_request(*params) for p in pending for m, params in p[4]]
        by_id = {r["id"]: r for r in node.batch(requests)}
        at = 0
        for case_at, op, inputs, rust, calls, judge in pending:
            results = []
            for _ in calls:
                r = by_id[requests[at]["id"]]
                at += 1
                results.append(("err", r["error"].get("message")) if r.get("error")
                               else ("ok", r["result"]))
            self.compared += 1
            said = judge(results)
            if said is not None:
                self.case_at = case_at
                raise Mismatch(self.name, op, inputs, rust, {"bitcoin-core": said})

    def run_part(self, part, lo=0, hi=0):
        self.pending = []
        try:
            super().run_part(part, lo, hi)
        except Exception:
            at = self.case_at
            self.flush(before=at)
            self.case_at = at
            raise
        self.flush()

    # ----- msg.sign ----------------------------------------------------------

    def sign(self, d, script, network, message, fmt, nonce, fresh=None, expected=None):
        inputs = {"secret": d.to_bytes(32, "big").hex(), "script": script, "network": network,
                  "message": message, "format": fmt, "nonce": nonce,
                  "aux": fresh.hex() if fresh is not None else None}
        rust = self.rust.call("msg.sign", **inputs)
        self.compared += 1
        key = key_of(d, network)
        address = address_of(key.pub, script)
        m = message.encode()

        def fail(ref, note=""):
            raise Mismatch(self.name, "msg.sign", inputs, rust, ref, note)

        def want(signature):
            ref = ("ok", {"address": address, "signature": signature})
            if rust != ref:
                fail(ref)
            if expected is not None and expected != ref:
                fail(ref, f"the references differ from the published vector {expected}")

        if fmt == "bip137" and script == 3:
            # BIP-137 has no taproot form: btclib refuses it too.
            try:
                bms.sign(m, key, address)
            except ValueError:
                if rust[0] != "err":
                    fail(("err", "btclib: no BIP-137 form for a taproot address"))
                return rust
            raise AssertionError("btclib signed BIP-137 for a taproot address")
        if fmt == "bip137":
            first = bms.sign(m, key, address)
            if nonce == "first":
                want(first.b64encode())
                # python-ecdsa, plain RFC 6979: the signature body is its own.
                digest = hashlib.sha256(magic_message(m)).digest()
                body = base64.b64decode(rust[1]["signature"])[1:]
                lineage.check_ecdsa(d, None, digest, lineage.der_of(body), "first")
            else:
                # Core's low-R grind on the plain signer, by btclib; the
                # recoverable form's header is the one btclib's verifier
                # accepts in the address form's range.
                plain = dsa.sign(magic_message(m), d)
                base = 27 + (first.rf - 27) // 4 * 4
                rs = plain.r.to_bytes(32, "big") + plain.s.to_bytes(32, "big")
                found = [b64(bytes([h]) + rs) for h in range(base, base + 4)
                         if bms_valid(message, address, b64(bytes([h]) + rs))]
                if len(found) != 1:
                    raise AssertionError(f"btclib verifies {len(found)} headers of its own "
                                         "low-R signature")
                want(found[0])
            raw = base64.b64decode(rust[1]["signature"])
            recid = (raw[0] - 27) & 3
            wif = b58.wif_from_prv_key(d, "regtest", True)
            if nonce == "first":
                self.core("msg.sign", inputs, rust, [("signmessagewithprivkey", (wif, message))],
                          lambda r, raw=raw, recid=recid: None if r[0] == (
                              "ok", b64(bytes([31 + recid]) + raw[1:])) else r[0])
            self.core("msg.sign", inputs, rust, [("verifymessage", (
                regtest_p2pkh(b58.p2pkh(key.pub)), b64(bytes([31 + recid]) + raw[1:]), message))],
                lambda r: None if r[0] == ("ok", True) else r[0])
            return rust
        if script in (0, 1):
            self.full_form(inputs, rust, key, address, message, KIND[script])
            return rust
        # BIP-322 simple: P2WPKH and P2TR.
        btcd = self.btcd({"op": "sign", "kind": KIND[script], "secret": inputs["secret"],
                          "message": message})[0]
        if rust[0] != "ok" or rust[1]["address"] != address:
            fail(("ok", {"address": address, "signature": "(any the references accept)"}))
        sig = rust[1]["signature"]
        states = {"btclib-wallet": btclib_state(message, address, sig),
                  "btcd": self.btcd({"op": "verify", "address": address, "message": message,
                                     "signature": sig})[0]["state"]}
        if set(states.values()) != {"valid"}:
            fail(states, "a reference does not accept Faraday's signature")
        if script == 2 and nonce == "first":
            made = bip322.sign(m, key, address).b64encode()
            if btcd.get("signature") != made:
                raise Mismatch(self.name, "msg.sign", inputs, rust,
                               {"btclib-wallet": made, "btcd": btcd},
                               "the two references sign differently")
            want(made.removeprefix("smp"))
        elif script == 2:
            self.core_p2wpkh(inputs, rust, d, address, message)
        return rust

    def full_form(self, inputs, rust, key, address, message, kind):
        """BIP-322 for a P2PKH or P2SH-P2WPKH address: both references
        sign it in the full form, each accepted by the other; Faraday
        refuses the format."""
        made = bip322.sign(message.encode(), key, address).b64encode()
        btcd = self.btcd({"op": "sign", "kind": kind, "secret": inputs["secret"],
                          "message": message})[0].get("signature", "")
        checks = {"btcd's by btclib-wallet": btclib_state(message, address, btcd),
                  "btclib-wallet's by btcd": self.btcd({"op": "verify", "address": address,
                                                        "message": message,
                                                        "signature": made})[0]["state"]}
        if not (made.startswith("ful") and btcd.startswith("ful")) \
                or set(checks.values()) != {"valid"}:
            raise Mismatch(self.name, "msg.sign", inputs, rust, checks,
                           "the references do not both sign and accept the full form")
        if rust_verdict(rust) != ("refused", "UnsupportedFormat"):
            raise Mismatch(self.name, "msg.sign", inputs, rust, {"full form": made},
                           "Faraday makes BIP-322's simple form only")
        self.known += 1

    def core_p2wpkh(self, inputs, rust, d, address, message):
        """Core's own low-R signature of BIP-322's `to_sign` (btclib-wallet
        builds it), whose witness must be Faraday's."""
        spk = ScriptPubKey.from_address(address).script
        spend = bip322.to_spend(message.encode(), spk)
        tx = bip322.to_sign(spend).serialize(include_witness=False).hex()
        prev = [{"txid": spend.id.hex(), "vout": 0, "scriptPubKey": spk.hex(), "amount": 0}]
        stack = [x.hex() for x in Witness.parse(base64.b64decode(rust[1]["signature"])).stack]

        def judge(r):
            if r[0][0] != "ok" or not r[0][1].get("complete"):
                return r[0]
            got = [x.hex() for x in Tx.parse(bytes.fromhex(r[0][1]["hex"])).vin[0]
                   .script_witness.stack]
            return None if got == stack else {"witness": got}
        self.core("msg.sign", inputs, rust, [("signrawtransactionwithkey", (
            tx, [b58.wif_from_prv_key(d, "regtest", True)], prev))], judge)

    # ----- msg.verify ----------------------------------------------------------

    def verify(self, address, message, signature, expected=None, state=None):
        """Faraday's verdict against the references'. `expected` is a
        published vector's: True (valid) or False; `state` the state it
        names for each BIP-322 reference."""
        inputs = {"address": address, "message": message, "signature": signature}
        rust = self.rust.call("msg.verify", **inputs)
        self.compared += 1
        got = rust_verdict(rust)
        kind = kind_of(address)
        text = signature.strip()
        if bms_shaped(text):
            ref, known = self.verify_bip137(inputs, rust, got, kind, text)
        else:
            ref, known = self.verify_bip322(inputs, rust, got, kind, text, state)
        if expected is not None and (ref[0] == "valid") != expected:
            raise Mismatch(self.name, "msg.verify", inputs, rust, ref,
                           f"the references differ from the published vector ({expected})")
        if got == ref or (ref[0] == "refused" and got[0] == "refused"):
            return rust
        if known is not None and known(got):
            self.known += 1
            return rust
        raise Mismatch(self.name, "msg.verify", inputs, rust, ref)

    def verify_bip137(self, inputs, rust, got, kind, text):
        address, message = inputs["address"], inputs["message"]
        raw = base64.b64decode(text)
        header = raw[0]
        strict = bms_valid(message, address, text)
        if kind == "p2pkh":
            legacy = btclib_state(message, address, text) == "valid"
            if legacy != strict:
                raise Mismatch(self.name, "msg.verify", inputs, rust,
                               {"btclib bms": strict, "btclib-wallet bip322 legacy": legacy},
                               "the references disagree")
        ref = ("valid", "bip137", FORM[kind]) if strict else ("refused", None)
        if kind == "p2pkh" and 27 <= header <= 42:
            # Core reads the key encoding from the header's bit 2; Faraday
            # tries both, so it accepts where Core accepts either.
            at = regtest_p2pkh(address)
            self.core("msg.verify", inputs, rust, [
                ("verifymessage", (at, b64(raw), message)),
                ("verifymessage", (at, b64(bytes([27 + ((header - 27) ^ 4)]) + raw[1:]), message))],
                lambda r: None if (("ok", True) in r) == (got[0] == "valid") else r)
        if strict or kind not in ("p2pkh", "p2sh", "p2wpkh") or not 27 <= header <= 42:
            return ref, None
        recid = (header - 27) & 3
        bases = {"p2pkh": [27, 31], "p2sh": [35], "p2wpkh": [39]}[kind]
        lenient = any(bms_valid(message, address, b64(bytes([b + recid]) + raw[1:]))
                      for b in bases)
        return ref, (lambda g: lenient and g == ("valid", "bip137", FORM[kind]))

    def verify_bip322(self, inputs, rust, got, kind, text, state):
        address, message = inputs["address"], inputs["message"]
        states = {"btclib-wallet": btclib_state(message, address, inputs["signature"]),
                  "btcd": self.btcd({"op": "verify", **inputs})[0]["state"]}
        if state is not None and set(states.values()) != {state}:
            raise Mismatch(self.name, "msg.verify", inputs, rust, states,
                           f"a reference differs from the published vector ({state})")
        valid = [s == "valid" for s in states.values()]
        if any(valid) and not all(valid):
            raise Mismatch(self.name, "msg.verify", inputs, rust, states,
                           "the references disagree")
        if not all(valid):
            return ("refused", None), None
        if text[:3] in ("ful", "pof"):
            return ("valid", "bip322", FORM.get(kind)), (
                lambda g: g == ("refused", "UnsupportedFormat"))
        if kind not in ("p2wpkh", "p2tr"):
            return ("valid", "bip322", FORM.get(kind)), (
                lambda g: g == ("refused", "UnsupportedAddress"))
        return ("valid", "bip322", FORM[kind]), None

    # ----- msg.bip322 ----------------------------------------------------------

    def bip322_ids(self, address, message, expected=None):
        m = message.encode()
        spend = bip322.to_spend(m, ScriptPubKey.from_address(address).script)
        ref = {"message_hash": bip322.message_hash(m).hex(), "to_spend": spend.id.hex(),
               "to_sign": bip322.to_sign(spend).id.hex()}
        btcd = self.btcd({"op": "ids", "address": address, "message": message})[0]
        if btcd != {"to_spend": ref["to_spend"], "to_sign": ref["to_sign"]}:
            raise Mismatch(self.name, "msg.bip322", {"address": address, "message": message},
                           None, {"btclib-wallet": ref, "btcd": btcd}, "the references disagree")
        return self.compare("msg.bip322", {"address": address, "message": message},
                            lambda: ref, expected)

    # ----- the published vectors -------------------------------------------------

    def published(self):
        v = json.loads((ROOT / "tools/vectors/message/bip322-basic.json").read_text())
        for case in v["tx_hashes"]:
            self.bip322_ids(case["address"], case["message"],
                            ("ok", {"message_hash": case["message_hash"],
                                    "to_spend": case["to_spend_tx_hash"],
                                    "to_sign": case["to_sign_tx_hash"]}))
        for case in v["simple"]:
            for sig in case["bip322_signatures"]:
                self.verify(case["address"], case["message"], sig, True)
        for case in v["error"]:
            self.verify(case["address"], case["message"], case["signature"], False)
        g = json.loads((ROOT / "tools/vectors/message/bip322-generated.json").read_text())
        for variant in ("simple", "full", "proof_of_funds"):
            for case in g[variant]:
                for sig in case["bip322_signatures"]:
                    self.verify(case["address"], case["message"], sig, True, "valid")
                if variant == "simple" and case["type"] in ("p2wpkh", "p2tr"):
                    self.sign_vector(case)
        for case in g["error"]:
            self.verify(case["address"], case["message"], case["signature"], False)
        n = json.loads((ROOT / "tools/vectors/message/bip322-btcd-negative.json").read_text())
        for case in n["error"]:
            if case["error_substr"] == "ErrMinimalIf" and case["signature"][:3] == "ful" \
                    and kind_of(case["address"]) == "p2sh":
                self.minimal_if_gap(case)
                continue
            self.verify(case["address"], case["message"], case["signature"], False,
                        case["expected_result"])
        self.bip137_published()

    def minimal_if_gap(self, case):
        """btcd's two legacy P2SH scripts whose OP_IF takes a non-minimal
        true, in the full form: invalid by BIP-322's MINIMALIF. btclib's
        engine applies MINIMALIF to witness v0 scripts only and accepts
        them, a gap its `bip322` module names; btcd refuses them, and so
        does Faraday, which reads no full form."""
        inputs = {k: case[k] for k in ("address", "message", "signature")}
        rust = self.rust.call("msg.verify", **inputs)
        self.compared += 1
        states = {"btclib-wallet": btclib_state(case["message"], case["address"],
                                                case["signature"]),
                  "btcd": self.btcd({"op": "verify", **inputs})[0]["state"]}
        if states != {"btclib-wallet": "valid", "btcd": "invalid"} \
                or rust_verdict(rust)[0] != "refused":
            raise Mismatch(self.name, "msg.verify", inputs, rust, states,
                           "btcd's MINIMALIF vector: btcd invalid, btclib's known gap valid, "
                           "Faraday refused")
        self.known += 1

    def sign_vector(self, case):
        """A generated single-key simple vector, signed by Faraday with the
        vector's key: P2WPKH byte for byte (RFC 6979's first nonce, as the
        BIP's generator signs), P2TR checked by both references."""
        d = b58.prv_key_data_from_wif(case["private_keys"][0]).q
        script = 2 if case["type"] == "p2wpkh" else 3
        expected = None
        if script == 2:
            expected = ("ok", {"address": case["address"],
                               "signature": case["bip322_signatures"][0].removeprefix("smp")})
        got = self.sign(d, script, "mainnet", case["message"], "bip322", "first",
                        expected=expected)
        if got[0] != "ok" or got[1]["address"] != case["address"]:
            raise AssertionError(f"the generated vector's key signs for {got}")

    def bip137_published(self):
        text = (ROOT / "tools/vectors/message/bip137.txt").read_text()
        for block in text.split("\n\n"):
            rec = dict(line.split(": ", 1) for line in block.splitlines()
                       if ": " in line and not line.startswith("#"))
            if "signature-hex" in rec:
                sig = b64(bytes.fromhex(rec["signature-hex"]))
            elif "signature-base64" in rec:
                sig = rec["signature-base64"]
            else:
                continue
            self.verify(rec["address"], rec["message"], sig, True)
            # Trezor signs with RFC 6979's first nonce: the same key signs
            # the same bytes on every side.
            script = {"legacy": 0, "nested-segwit": 1, "native-segwit": 2}.get(rec.get("script"))
            if "words" in rec and rec.get("no-script-type") != "yes" and script is not None:
                seed = Mnemonic.to_seed(rec["words"], "")
                xprv = bip32.derive(bip32.rootxprv_from_seed(seed), rec["path"])
                d = int.from_bytes(bip32.BIP32KeyData.b58decode(xprv).key[1:], "big")
                network = "mainnet" if rec["address"][0] in "13b" else "testnet"
                self.sign(d, script, network, rec["message"], "bip137", "first",
                          expected=("ok", {"address": rec["address"], "signature": sig}))

    def found(self):
        """Seed 17af375c…'s disagreement, every run: a P2WPKH witness whose
        `s` is written as a negative integer is not BIP-66 DER (libsecp256k1's
        parser reads it as zero), and the same numbers written as DER do not
        verify: refused either way."""
        address = "bcrt1qup7h6dnv9nxpu2tzkr3cekdzqa0lkt5sfsa3pq"
        for sig in [
            "AkcwRAIgTgR9Hb+lhM3ejBtyKxwGvimMnWBuSR5GbFjdacdJMjkCINvePNT/c/qRKUGZnJP1qPNFrbz+"
            "ZayRuNK46Fzltgl6ASEDNR2yiaxyEmvCy3aI/aT8NE1xJBMH6I/41yd1GUs3s80=",
            "AkgwRQIgTgR9Hb+lhM3ejBtyKxwGvimMnWBuSR5GbFjdacdJMjkCIQDb3jzU/3P6kSlBmZyT9ajzRa28/"
            "mWskbjSuOhc5bYJegEhAzUdsomschJrwst2iP2k/DRNcSQTB+iP+NcndRlLN7PN",
        ]:
            self.verify(address, "Hello World", sig, False)

    # ----- the corpus ------------------------------------------------------------

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
        address = address_of(key_of(d, network).pub, script)
        self.bip322_ids(address, message)
        if script >= 2:
            # btcd's own simple signature, which Faraday must accept.
            theirs = self.btcd({"op": "sign", "kind": KIND[script], "secret": f"{d:064x}",
                                "message": message})[0]["signature"]
            self.verify(address, message, theirs, True)
        if got[0] != "ok":
            return
        sig = got[1]["signature"]
        self.verify(address, message, sig, True)
        self.verify(address, message + rng.choice(["!", " ", "\n"]), sig)
        other = address_of(key_of(rng.randrange(1, N), network).pub, rng.randrange(4))
        self.verify(other, message, sig)
        raw = bytearray(base64.b64decode(sig))
        i = rng.randrange(len(raw))
        raw[i] ^= 1 << rng.randrange(8)
        self.verify(address, message, b64(bytes(raw)))
        if fmt == "bip137":
            raw = bytearray(base64.b64decode(sig))
            raw[0] = rng.choice([27, 28, 31, 32, 35, 36, 39, 40, 43, 26, raw[0] ^ 4])
            self.verify(address, message, b64(bytes(raw)))
        else:
            self.verify(address, message, "smp" + sig, True)

    LOOPS = (("corpus", 600),)

    def prelude(self):
        self.published()
        self.found()

    def case_corpus(self):
        self.corpus_case()

