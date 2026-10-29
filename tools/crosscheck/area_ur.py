"""Uniform Resources (`faraday_codec::ur`) against Foundation Devices'
foundation-ur-py (`tools/reference/ur/`), a Python port of Blockchain
Commons' URKit checked against URKit's own tests.

- URKit's vectors as foundation-ur-py's tests state them
  (`tools/vectors/ur/`): the 50-byte single-part `ur:bytes`, and the
  first twenty parts of a 256-byte message in fragments of at most 30
  bytes, nine fragments and eleven fountain mixes.
- Random messages as `ur:crypto-psbt` and `ur:bytes`: the single-part UR
  and the first parts of the animation (the fragments, then mixes) at
  random fragment lengths, byte for byte; then sets of those parts, some
  fragments dropped and mixes in their place, shuffled and repeated,
  put together by both decoders, which must agree on whether the
  message is whole and what it is.
- Random URs of other types written from random CBOR and read back,
  in upper case, with a character changed, cut, and with a payload that
  is not a CBOR byte string under `crypto-psbt` or `bytes`.

Where the two differ, the corpus keeps out of it:

- Faraday has no minimum fragment length (URKit's encoder takes 10 by
  default); the reference encoder is asked with a minimum of 1, which is
  Faraday's rule of ceil(length / ceil(length / max)).
- Faraday writes every part as `ur:<type>/<seq>-<count>/…`, a message of
  one fragment included; URKit's `UREncoder` writes that one as a
  single-part UR. The reference's parts are made with
  `UREncoder.encode_part` from its fountain encoder, as URKit's are
  when there is more than one.
- foundation-ur-py checks a type with `'a' <= type <= 'z'` (its
  `is_ur_type` is a test of one character), so it refuses a type
  starting with `z` and longer than one letter and passes characters
  Faraday refuses. The corpus's types are letters, digits and `-`, not
  starting with `z`.
- Faraday refuses to animate an empty message, and a part of another
  message is a refusal where the reference ignores it; neither is made.
- Faraday's decoder reduces a mix by any buffered mix whose fragments
  are a subset of its own, whichever came first; URKit's reduces in the
  order parts arrive, so Faraday sometimes finishes a set URKit does
  not. Such a set must give the message that was sent, and is counted as
  a known difference. A set URKit finishes and Faraday does not is a
  disagreement.
"""

import json

from common import ROOT, Area, Mismatch, use_reference

use_reference("ur")
from ur.cbor_lite import CBORDecoder, CBOREncoder  # noqa: E402
from ur.fountain_encoder import FountainEncoder  # noqa: E402
from ur.ur import UR  # noqa: E402
from ur.ur_decoder import URDecoder  # noqa: E402
from ur.ur_encoder import UREncoder  # noqa: E402
from ur.xoshiro256 import Xoshiro256  # noqa: E402

TYPES = {"psbt": "crypto-psbt", "bytes": "bytes"}
OTHER_TYPES = ["crypto-account", "crypto-output", "crypto-hdkey", "crypto-seed", "bytes2",
               "a", "x-1", "crypto-request"]


def cbor_bytes(data):
    e = CBOREncoder()
    e.encodeBytes(bytes(data))
    return bytes(e.get_bytes())


def message_of(ur):
    if ur.type in TYPES.values():
        d = CBORDecoder(bytes(ur.cbor))
        value, used = d.decodeBytes()
        if used != len(ur.cbor):
            raise ValueError("bytes after the CBOR byte string")
        return {"type": ur.type, "data": bytes(value).hex()}
    return {"type": ur.type, "cbor": bytes(ur.cbor).hex()}


def encode(kind, data, fragment_len, count):
    if not data:
        raise ValueError("an empty message")
    ur_type = TYPES[kind]
    cbor = cbor_bytes(data)
    fe = FountainEncoder(bytearray(cbor), fragment_len, 0, 1)
    parts = [UREncoder.encode_part(ur_type, fe.next_part()) for _ in range(count)]
    return {"single": UREncoder.encode(UR(ur_type, cbor)), "fragments": fe.seq_len(),
            "parts": parts}


def decode(parts):
    d = URDecoder()
    for p in parts:
        d.receive_part(p)
    if d.is_failure():
        raise ValueError(f"the reference failed: {d.result_error()}")
    done = bool(d.is_complete() and d.is_success())
    return {"complete": done, "message": message_of(d.result_message()) if done else None}


class Ur(Area):
    name = "ur"
    faraday = (
        "`faraday_codec::ur` (single-part URs, the animation's parts, the decoder)"
    )
    reference = (
        "Foundation Devices' foundation-ur-py (`ur/`), URKit ported"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; foundation-ur-py is itself a port of URKit checked against URKit's "
        'own tests, whose vectors the area also runs'
    )
    vectors = (
        "URKit's single-part and multipart encoder vectors (`tools/vectors/ur/`)"
    )
    corpus = (
        'messages as `crypto-psbt` and `bytes` at random fragment lengths, parts byte '
        'for byte; sets with fragments dropped, mixes added, shuffled, repeated and '
        'upper case, put together by both; URs of other types written, read, changed '
        'and cut'
    )
    differences = [
        (
            'UR: Faraday has no minimum fragment length, writes a one-fragment message as a'
            ' part like any other, and refuses to animate an empty message; '
            "foundation-ur-py checks a UR type one character's way (`'a' <= type <= 'z'`). "
            'The area asks the reference with a minimum of 1, makes parts with its fountain'
            ' encoder, and keeps its types to letters, digits and `-` not starting with '
            "`z`. Faraday's decoder reduces mixes by each other whichever arrived first, "
            "and so sometimes finishes a set URKit's does not; the area then checks the "
            'message is the one sent and counts the set.'
        ),
    ]
    not_compared = (
        'none'
    )

    def encode(self, kind, data, fragment_len, count, expected=None):
        return self.compare("ur.encode", {"kind": kind, "data": bytes(data).hex(),
                                          "fragment_len": fragment_len, "count": count},
                            lambda: encode(kind, data, fragment_len, count), expected)

    def decode(self, parts, kind=None, data=None):
        """Both decoders over one set of parts. Where the reference puts
        the message together Faraday must too, with the same message;
        Faraday may finish a set the reference does not (its decoder
        reduces mixes by each other in either order, URKit's only in the
        order they arrive), and then its message must be the one sent,
        which the area knows, and the set is counted."""
        req = {"parts": parts}
        got = self.rust.call("ur.decode", **req)
        ref = ("ok", decode(parts))
        self.compared += 1
        if got == ref:
            return got
        sent = {"type": TYPES[kind], "data": bytes(data).hex()} if kind else None
        if (got[0] == "ok" and got[1]["complete"] and not ref[1]["complete"]
                and sent is not None and got[1]["message"] == sent):
            # Counted in the run's output as a known difference.
            self.known += 1
            return got
        raise Mismatch(self.name, "ur.decode", req, got, ref)

    def single(self, text):
        return self.compare("ur.single", {"text": text},
                            lambda: message_of(URDecoder.decode(text)))

    def published(self):
        v = json.loads((ROOT / "tools/vectors/ur/vectors.json").read_text())
        s = v["single_part"]
        msg = Xoshiro256.from_string("Wolf").next_data(s["message_length"])
        got = self.need(self.encode("bytes", msg, 1000, 1), "encode")
        if got["single"] != s["ur"]:
            raise AssertionError(f"URKit's single-part vector: {got['single']}")
        self.single(s["ur"])
        m = v["multipart"]
        msg = Xoshiro256.from_string("Wolf").next_data(m["message_length"])
        got = self.need(self.encode("bytes", msg, m["max_fragment_len"], len(m["parts"])),
                        "encode")
        if got["parts"] != m["parts"]:
            raise AssertionError("URKit's multipart vector differs")
        self.decode(m["parts"])

    def corpus_case(self):
        rng = self.rng
        kind = rng.choice(["psbt", "bytes"])
        data = self.randbytes(rng.choice([1, 2, rng.randint(1, 100), rng.randint(100, 3000)]))
        frag = rng.choice([1, 5, 10, 30, rng.randint(1, 400), 5000])
        got = self.encode(kind, data, frag, 1)
        n = self.need(got, "encode")["fragments"]
        if n > 600:
            return
        extra = rng.randint(0, 2 * n + 2)
        parts = self.need(self.encode(kind, data, frag, n + extra), "encode")["parts"]
        chosen = [p for p in parts[:n] if rng.random() > rng.choice([0, 0.1, 0.3])]
        chosen += [p for p in parts[n:] if rng.random() < 0.7]
        rng.shuffle(chosen)
        if chosen and rng.random() < 0.3:
            chosen += rng.sample(chosen, k=min(3, len(chosen)))
        if rng.random() < 0.2:
            chosen = [p.upper() for p in chosen]
        if chosen:
            self.decode(chosen, kind, data)
        self.single(self.need(got, "encode")["single"])
        self.other_case()

    def other_case(self):
        rng = self.rng
        ur_type = rng.choice(OTHER_TYPES + list(TYPES.values()))
        cbor = self.randbytes(rng.randint(0, 200))
        if rng.random() < 0.3:
            cbor = cbor_bytes(cbor)
        if rng.random() < 0.2:
            cbor = cbor + self.randbytes(rng.randint(1, 3))
        got = self.compare("ur.write", {"type": ur_type, "cbor": cbor.hex()},
                           lambda: {"ur": UREncoder.encode(UR(ur_type, cbor))})
        text = self.need(got, "write")["ur"]
        self.single(text)
        r = rng.random()
        if r < 0.3:
            self.single(text.upper())
        elif r < 0.6:
            # In the payload: a type changed to start with `z` is one
            # the reference misreads (see above).
            i = rng.randrange(text.index("/") + 1, len(text))
            self.single(text[:i] + rng.choice("abcdefghijklmnopqrstuvwxyz/-") + text[i + 1:])
        elif r < 0.8:
            self.single(text[:rng.randrange(len(text))])

    LOOPS = (("corpus", 600),)

    def prelude(self):
        self.published()

    def case_corpus(self):
        self.corpus_case()
