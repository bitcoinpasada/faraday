"""OpenTimestamps proofs (`faraday_ots`, `docs/decisions/F230.md`) against
python-opentimestamps (`tools/reference/opentimestamps/`, vendored unchanged:
the `opentimestamps/core/` modules the area imports, on three small shims
for what they import and the repository does not vendor).

- Each proof of the OpenTimestamps client's examples
  (`tools/vectors/opentimestamps/`) is read by both and walked by both
  against each of the six blocks' headers; the three invalid examples are
  refused by both, and the two that reach a block through SHA-256 only are
  committed by it in both.
- Random proofs built through the reference's own classes (`Timestamp`, every
  operation, the file hashed with SHA-256, SHA-1 or RIPEMD-160, a main path to
  a Bitcoin attestation and side branches ending in pending, Bitcoin,
  Litecoin, Ethereum and unknown attestations, some at a node that also
  has operations), merkle trees through `make_merkle_tree` with `cat_sha256`
  and `cat_sha256d`, and chains of up to 255 operations, serialised by the
  reference; both read each, and walk each against a header whose merkle
  root is the main path's result (Faraday also checks a header's proof of
  work, so the headers are mined at an easy target) and against one whose
  root is not.
- Proofs with a byte changed, inserted, deleted or set to 0x00, 0x80 or 0xff,
  cut short or with bytes after them: both read or both refuse, and read the
  same; the tree of one that both read is walked as well.
- Each limit, either side of it, built byte by byte: a chain of 255 and 256
  operations, an argument of 4,096 and 4,097 bytes, a path to 4,096 and 4,097
  bytes, a hexlify of 2,048 and 2,049 bytes, an attestation payload of 8,192
  and 8,193 bytes, an address of 1,000 and 1,001 bytes and a height of 2^32 - 1
  and 2^32.
- Block headers: 80 bytes read by Faraday and by the area's own transcription
  of Bitcoin Core's `SetCompact` and proof-of-work test, over targets that are
  easy, zero, negative, too large and short, headers that meet theirs and
  headers that do not, and lengths other than 80.

Known differences, each checked to still hold and counted:

- A Bitcoin path through SHA-1, RIPEMD-160 or Keccak-256 (or a file taken with
  SHA-1 or RIPEMD-160) is counted by the reference when it reaches the merkle
  root and named, not counted, by Faraday.
- An Ethereum attestation is verified by the reference against a block's
  transactions root and listed by Faraday, as a Litecoin one is (the reference
  raises for it).
- The reference collapses an attestation repeated at one node and refuses an
  operation repeated at one node; Faraday lists the one twice and reads the other.
- The reference reads the version as one byte and a varuint of up to nine
  bytes; Faraday reads both as a varuint, so a version written as 0x81 0x00 and
  a varuint padded past nine bytes are read by Faraday and refused by the
  reference.
- Faraday refuses a proof over 64 KiB (and one of more than 16,384 entries or
  whose operations make more than 16 MiB of values, which that size cannot
  reach in a tree the reference reads); the reference reads it.
- The reference takes the file's hash as a SHA-1, RIPEMD-160 or SHA-256
  (Keccak-256 is not one of its file hashes); Faraday also reads Keccak-256.
"""

import hashlib
import io
import json
import struct
import sys

from common import ROOT, Area, Mismatch, reference, use_reference

use_reference("core")
use_reference("opentimestamps", "shims")
use_reference("opentimestamps", "src")
from opentimestamps.core.dubious.notary import EthereumBlockHeaderAttestation  # noqa: E402
from opentimestamps.core.notary import (  # noqa: E402
    BitcoinBlockHeaderAttestation,
    LitecoinBlockHeaderAttestation,
    PendingAttestation,
    UnknownAttestation,
)
from opentimestamps.core.op import (  # noqa: E402
    OpAppend,
    OpHexlify,
    OpKECCAK256,
    OpPrepend,
    OpReverse,
    OpRIPEMD160,
    OpSHA1,
    OpSHA256,
)
from opentimestamps.core.serialize import (  # noqa: E402
    BytesDeserializationContext,
    BytesSerializationContext,
)
from opentimestamps.core.timestamp import (  # noqa: E402
    DetachedTimestampFile,
    Timestamp,
    cat_sha256,
    cat_sha256d,
    make_merkle_tree,
)

sys.setrecursionlimit(max(sys.getrecursionlimit(), 5000))

VECTORS = ROOT / "tools" / "vectors" / "opentimestamps"
MAGIC = DetachedTimestampFile.HEADER_MAGIC
TAG_PENDING = PendingAttestation.TAG
TAG_BITCOIN = BitcoinBlockHeaderAttestation.TAG
TAG_LITECOIN = LitecoinBlockHeaderAttestation.TAG
TAG_ETHEREUM = EthereumBlockHeaderAttestation.TAG
KNOWN_TAGS = (TAG_PENDING, TAG_BITCOIN, TAG_LITECOIN, TAG_ETHEREUM)
URI_CHARS = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._/:"
EASY_BITS = 0x207FFFFF
NOT_SHA256 = ("sha1", "ripemd160", "keccak256")
HASH_OPS = {"sha256": OpSHA256, "sha1": OpSHA1, "ripemd160": OpRIPEMD160}


# ----- bytes the area writes itself --------------------------------------------

def varuint(n, length=0):
    """`n` as a varuint, padded with zero bits to `length` bytes."""
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            break
    if length > len(out):
        out[-1] |= 0x80
        out += b"\x80" * (length - len(out) - 1) + b"\x00"
    return bytes(out)


def varbytes(b, length=0):
    return varuint(len(b), length) + b


def raw_attestation(tag, payload, pad=0):
    return b"\x00" + tag + varbytes(payload, pad)


def raw_proof(tree, digest=b"\x11" * 32, file_op=b"\x08", version=b"\x01"):
    return MAGIC + version + file_op + digest + tree


def raw_height(tag, height, pad=0):
    return raw_attestation(tag, varuint(height, pad))


def sha256d(b):
    return hashlib.sha256(hashlib.sha256(b).digest()).digest()


# ----- the reference's side ----------------------------------------------------

def serialize(file_op, ts):
    ctx = BytesSerializationContext()
    DetachedTimestampFile(file_op, ts).serialize(ctx)
    return ctx.getbytes()


def attestation_text(att):
    if isinstance(att, PendingAttestation):
        return f"pending {att.uri}"
    if isinstance(att, BitcoinBlockHeaderAttestation):
        return f"bitcoin {att.height}"
    if isinstance(att, LitecoinBlockHeaderAttestation):
        return f"litecoin {att.height}"
    if isinstance(att, EthereumBlockHeaderAttestation):
        return f"ethereum {att.height}"
    return f"unknown {att.TAG.hex()} {att.payload.hex()}"


def ref_walk(ts, path, bad, out):
    """Every attestation under `ts`: (path of operations, attestation,
    the value there, the first hash on the path other than SHA-256)."""
    for att in ts.attestations:
        out.append((path, att, ts.msg, bad))
    for op, child in ts.ops.items():
        b = bad
        if b is None and op.TAG_NAME in NOT_SHA256:
            b = op.TAG_NAME
        ref_walk(child, path + [str(op)], b, out)


def ref_parse(data):
    """The reference's reading of a proof: (file hash name, digest, leaves)."""
    dtf = DetachedTimestampFile.deserialize(BytesDeserializationContext(data))
    name = dtf.file_hash_op.TAG_NAME
    leaves = []
    ref_walk(dtf.timestamp, [], None if name == "sha256" else name, leaves)
    return name, dtf.file_digest, leaves


def leaf_text(path, att):
    return (" | ".join(path) + " " if path else "") + "=> " + attestation_text(att)


def ref_read(data):
    name, digest, leaves = ref_parse(data)
    return {"file_op": name, "digest": digest.hex(),
            "leaves": sorted(leaf_text(p, a) for p, a, _, _ in leaves)}


class RefHeader:
    """What `BitcoinBlockHeaderAttestation.verify_against_blockheader`
    reads of a header (python-bitcoinlib's `CBlockHeader` has these two)."""

    def __init__(self, raw):
        self.hashMerkleRoot = raw[36:68]
        self.nTime = struct.unpack("<I", raw[68:72])[0]
        self.hash = sha256d(raw)[::-1]


def ref_verify(data, digest, header):
    """The reference's verdict on every attestation of a proof against a
    header, in Faraday's words, and what the reference counts that
    Faraday does not (a Bitcoin path through SHA-1, RIPEMD-160 or
    Keccak-256 that reaches the root; an Ethereum one that verifies)."""
    _, file_digest, leaves = ref_parse(data)
    if file_digest != digest:
        return {"file_matches": False, "committed": False, "paths": []}, 0
    h = RefHeader(header)
    paths, counted = [], 0
    committed = False
    for _, att, msg, bad in leaves:
        if isinstance(att, BitcoinBlockHeaderAttestation):
            try:
                time = att.verify_against_blockheader(msg, h)
                good = True
            except Exception:  # noqa: BLE001 - a VerificationError says it does not
                time, good = None, False
            if bad:
                paths.append({"kind": "bitcoin", "height": att.height, "result": "not-sha256",
                              "op": bad, "reaches": good})
                counted += good
            elif good:
                committed = True
                paths.append({"kind": "bitcoin", "height": att.height, "result": "match",
                              "time": time, "block_hash": h.hash.hex()})
            else:
                paths.append({"kind": "bitcoin", "height": att.height, "result": "mismatch"})
        elif isinstance(att, EthereumBlockHeaderAttestation):
            block = {"transactionsRoot": "0x" + msg.hex(), "timestamp": 1}
            if len(msg) == 32 and att.verify_against_blockheader(msg, block) == 1:
                counted += 1
            paths.append({"kind": "ethereum", "height": att.height})
        elif isinstance(att, LitecoinBlockHeaderAttestation):
            paths.append({"kind": "litecoin", "height": att.height})
        elif isinstance(att, PendingAttestation):
            paths.append({"kind": "pending", "uri": att.uri})
        else:
            paths.append({"kind": "unknown", "tag": att.TAG.hex()})
    return {"file_matches": True, "committed": committed, "paths": paths}, counted


def compact(bits):
    """Bitcoin Core's `arith_uint256::SetCompact`: (target, negative,
    overflow), transcribed here."""
    size = bits >> 24
    word = bits & 0x007FFFFF
    if size <= 3:
        word >>= 8 * (3 - size)
    else:
        word <<= 8 * (size - 3)
    negative = word != 0 and bool(bits & 0x00800000)
    overflow = word >= 1 << 256
    return word, negative, overflow


def ref_header(raw):
    """What Bitcoin Core's `CheckProofOfWork` and its header read say, with
    regtest's proof-of-work limit (nBits 0x207fffff) as the highest target."""
    if len(raw) != 80:
        raise ValueError("a header is 80 bytes")
    target, negative, overflow = compact(struct.unpack("<I", raw[72:76])[0])
    if negative or overflow or target == 0 or target > compact(EASY_BITS)[0]:
        raise ValueError("target")
    block = sha256d(raw)[::-1]
    if int.from_bytes(block, "big") > target:
        raise ValueError("work")
    return {"block_hash": block.hex(), "merkle_root": raw[36:68].hex(),
            "time": struct.unpack("<I", raw[68:72])[0]}


def mine(root, rng, bits=EASY_BITS, tries=200_000):
    """A header with `root` as its merkle root that meets `bits`, or None."""
    target = compact(bits)[0]
    head = struct.pack("<I", 0x20000000) + rng.randbytes(32) + root
    time = rng.randint(1_231_006_505, 2_000_000_000)
    for nonce in range(tries):
        raw = head + struct.pack("<III", time, bits, nonce)
        if int.from_bytes(sha256d(raw)[::-1], "big") <= target:
            return raw
    return None


class Ots(Area):
    name = "ots"
    faraday = (
        "`faraday_ots` (`Proof::parse`, `Proof::verify`, `Header::parse`), asked through "
        "the three requests `ots.read`, `ots.verify` and `ots.header`"
    )
    reference = (
        "python-opentimestamps' `opentimestamps/core/` (`ots/`, unchanged), by the "
        "OpenTimestamps developers, on three small shims for what it imports and the "
        "repository does not vendor (Keccak-256, RIPEMD-160 over Bitcoin Core's pure-Python "
        "hash, and four names of python-bitcoinlib that only its printing routine uses); "
        "block headers are read by the area's own transcription of Bitcoin Core's "
        "`SetCompact` and proof-of-work test"
    )
    reference_checked = (
        "the vendored files are hashed against `references.sha256` before any run; the "
        "client's example proofs (`tools/vectors/opentimestamps/`) are read by both and the "
        "two that reach a block through SHA-256 alone are required to be committed by both; "
        "the shims' hashes are checked against known values before the corpus; each limit is "
        "built either side of its edge and both must agree"
    )
    vectors = (
        "the OpenTimestamps client's example proofs (all 22, three of them invalid) and the "
        "headers of the six blocks they name, each proof walked against each header"
    )
    corpus = (
        "proofs built through the reference's classes with every operation, the file hashed "
        "with SHA-256, SHA-1 or RIPEMD-160, side branches ending in pending, Bitcoin, "
        "Litecoin, Ethereum and unknown attestations, and chains to 255 operations, walked "
        "against a header that commits to the main path and one that does not; merkle trees "
        "of up to 40 leaves; proofs with a byte changed, inserted, deleted, cut or added "
        "to; every limit either side of its edge; headers over every kind of target and "
        "lengths other than 80"
    )
    differences = [
        (
            "OTS: a Bitcoin path through SHA-1, RIPEMD-160 or Keccak-256, or a file taken "
            "with SHA-1 or RIPEMD-160, is counted by the reference when it reaches the "
            "header's merkle root; Faraday names the first such hash and does not count it. "
            "Each is asked of the reference and counted."
        ),
        (
            "OTS: an Ethereum attestation is verified by the reference against a block's "
            "transactions root; Faraday lists it by height and does not check it. A Litecoin "
            "one is listed by both (the reference raises when asked to verify it)."
        ),
        (
            "OTS: an attestation repeated at one node is collapsed by the reference and "
            "listed twice by Faraday; an operation repeated at one node is refused by the "
            "reference and read by Faraday."
        ),
        (
            "OTS: the reference reads the version as one byte and a varuint of up to nine "
            "bytes; Faraday reads both as a varuint of any length whose value fits 64 bits, "
            "so a version written 0x81 0x00 and a varuint padded to ten bytes or more are "
            "read by Faraday and refused by the reference."
        ),
        (
            "OTS: Faraday reads a proof whose file is taken with Keccak-256; the "
            "reference takes only SHA-1, RIPEMD-160 and SHA-256 for the file and refuses it."
        ),
    ]
    not_compared = (
        "the reference's printing of a tree and its stream readers; the order of a node's "
        "entries (the walk is compared as a set of paths); whether a header is in the chain; "
        "calendars, which neither side asks"
    )

    LOOPS = (("proofs", 160), ("merkle", 60), ("mutated", 240), ("headers", 120))

    # ----- the comparisons ---------------------------------------------------

    def fail(self, op, inputs, rust, ref, note=""):
        raise Mismatch(self.name, op, inputs, rust, ref, note)

    def read_both(self, data, label=""):
        """Both read a proof alike, or both refuse it; the known differences
        are named and counted. Faraday's reading, or None when both refuse."""
        inputs = {"proof": data.hex()}
        rust = self.rust.call("ots.read", **inputs)
        ref = reference(lambda: ref_read(data))
        self.compared += 1
        self.read_agreed = rust[0] == "ok" and ref[0] == "ok"
        if rust[0] == "err" and ref[0] == "err":
            return None
        if rust[0] == "ok" and ref[0] == "ok":
            got, want = rust[1], ref[1]
            leaves = got["leaves"]
            if got["repeated_op"]:
                self.fail("ots.read", inputs, rust, ref, "an operation repeated at a node is "
                          "read by both")
            repeated = len(leaves) != len(set(leaves))
            if repeated != got["repeated_attestation"]:
                self.fail("ots.read", inputs, rust, ref, "the repeated attestation flag is wrong")
            if got["file_op"] != want["file_op"] or got["digest"] != want["digest"] \
                    or sorted(set(leaves)) != want["leaves"]:
                self.fail("ots.read", inputs, rust, ref, label)
            if repeated:
                self.known += 1
            return got
        if rust[0] == "ok" and self.explained(data, rust[1], ref[1]):
            self.known += 1
            return rust[1]
        if rust[0] == "err" and ref[0] == "ok" and "ProofTooLarge" in rust[1] \
                and len(data) > 64 * 1024:
            self.known += 1
            return None
        self.fail("ots.read", inputs, rust, ref, label or "an unlisted difference")

    def explained(self, data, got, why):
        """Faraday reads what the reference refuses, for a reason the area
        lists, and holds to it."""
        if "repeated" in why and got["repeated_op"]:
            return True
        if why.startswith("UnsupportedMajorVersion") and len(data) > 32 and data[31] & 0x80:
            value, shift = 0, 0
            for b in data[31:]:
                value |= (b & 0x7F) << shift
                shift += 7
                if not b & 0x80:
                    break
            return value == 1
        if "varuint exceeds 9 bytes" in why:
            return True
        return "Unknown unary op tag 0x67" in why and len(data) > 32 and data[32] == 0x67

    def verify_both(self, data, digest, header):
        """Both walk a proof against a header (Faraday's work check met):
        the same attestations with the same outcome, but for the counted
        paths the reference counts and Faraday does not."""
        inputs = {"proof": data.hex(), "digest": digest.hex(), "header": header.hex()}
        rust = self.rust.call("ots.verify", **inputs)
        ref = reference(lambda: ref_verify(data, digest, header))
        self.compared += 1
        if rust[0] != "ok" or ref[0] != "ok":
            self.fail("ots.verify", inputs, rust, ref, "a proof read by both is walked by both")
        got, (want, counted) = rust[1], ref[1]
        mine = sorted(json.dumps(p, sort_keys=True) for p in got["paths"])
        theirs = sorted(json.dumps(p, sort_keys=True) for p in want["paths"])
        # An attestation repeated at a node is listed twice by Faraday.
        if len(set(mine)) != len(mine) and sorted(set(mine)) == theirs:
            mine = sorted(set(mine))
        if got["file_matches"] != want["file_matches"] or got["committed"] != want["committed"] \
                or mine != theirs:
            self.fail("ots.verify", inputs, rust, ("ok", want))
        self.known += counted
        return got

    def header_both(self, raw):
        inputs = {"header": raw.hex()}
        rust = self.rust.call("ots.header", **inputs)
        ref = reference(lambda: ref_header(raw))
        self.compared += 1
        if rust[0] != ref[0] or (rust[0] == "ok" and rust[1] != ref[1]):
            self.fail("ots.header", inputs, rust, ref)
        return rust

    def walk(self, data, header, digest=None):
        """Read a proof with both and, when both do, walk it against `header`."""
        got = self.read_both(data)
        if got is None or not self.read_agreed:
            return got
        self.verify_both(data, digest if digest is not None else bytes.fromhex(got["digest"]),
                         header)
        return got

    # ----- the published vectors -------------------------------------------------

    def headers(self):
        out = {}
        for f in sorted((VECTORS / "headers").glob("*.hex")):
            raw = bytes.fromhex(f.read_text().strip())
            self.header_both(raw)
            out[int(f.stem)] = raw
        return out

    def published(self):
        headers = self.headers()
        wrong_digest = bytes(32)
        sha256_only = {"different-blockchains.txt.ots", "gdp2q25-2nd.pdf.ots"}
        for path in sorted((VECTORS / "examples").rglob("*.ots")):
            data = path.read_bytes()
            got = self.read_both(data, path.name)
            if path.parent.name == "invalid":
                if got is not None:
                    raise AssertionError(f"{path.name}: an invalid example was read")
                continue
            if got is None:
                raise AssertionError(f"{path.name}: a published example was refused")
            digest = bytes.fromhex(got["digest"])
            source = path.with_suffix("")
            if source.is_file():
                hasher = HASH_OPS[got["file_op"]]()
                with open(source, "rb") as fd:
                    own = hasher.hash_fd(fd)
                self.verify_both(data, own, next(iter(headers.values())))
            committed = False
            for raw in headers.values():
                if self.verify_both(data, digest, raw)["committed"]:
                    committed = True
            self.verify_both(data, wrong_digest, next(iter(headers.values())))
            if (path.name in sha256_only) != committed:
                raise AssertionError(f"{path.name}: committed through SHA-256 alone is {committed}")

    # ----- the shims -----------------------------------------------------------------

    def shims(self):
        """The two hashes the reference's classes take from the shims, against
        known values, so that a wrong shim cannot make both sides agree
        about a wrong hash."""
        known = [
            (OpKECCAK256, b"", "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"),
            (OpKECCAK256, b"abc",
             "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"),
            (OpRIPEMD160, b"", "9c1185a5c5e9fc54612808977ee8f548b2258d31"),
            (OpRIPEMD160, b"abc", "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"),
        ]
        for op, msg, want in known:
            if op()(msg).hex() != want:
                raise AssertionError(f"the reference's {op.TAG_NAME} of {msg!r} is wrong")
        from Cryptodome.Hash import RIPEMD160, keccak  # the shims, by name
        if RIPEMD160.new(b"abc").digest().hex() != known[3][2] \
                or keccak.new(digest_bits=256, data=b"abc").digest().hex() != known[1][2]:
            raise AssertionError("a shim's hash is wrong")

    # ----- builders -------------------------------------------------------------------

    def uri(self):
        n = self.rng.choice([1, 8, 24, 60])
        return "".join(chr(self.rng.choice(URI_CHARS)) for _ in range(n))

    def attestation(self, kind=None):
        rng = self.rng
        kind = kind or rng.choice(["pending", "bitcoin", "bitcoin", "litecoin", "ethereum",
                                   "unknown"])
        if kind == "pending":
            return PendingAttestation(self.uri())
        height = rng.choice([0, 1, 129_405, rng.randint(0, 900_000), 2 ** 32 - 1])
        if kind == "bitcoin":
            return BitcoinBlockHeaderAttestation(height)
        if kind == "litecoin":
            return LitecoinBlockHeaderAttestation(height)
        if kind == "ethereum":
            return EthereumBlockHeaderAttestation(height)
        while True:
            tag = self.randbytes(8)
            if tag not in KNOWN_TAGS:
                break
        return UnknownAttestation(tag, self.randbytes(rng.choice([0, 1, 8, 40, 300])))

    def an_op(self, msg, mixed):
        """An operation that can be applied to `msg`, or None."""
        rng = self.rng
        kinds = ["append", "prepend", "sha256", "sha256", "reverse", "hexlify"]
        if mixed:
            kinds += ["sha1", "ripemd160", "keccak256"]
        kind = rng.choice(kinds)
        if kind == "append" or kind == "prepend":
            n = rng.choice([1, 4, 32, 33, rng.randint(1, 64), rng.randint(1, 1500)])
            arg = self.randbytes(n)
            op = OpAppend(arg) if kind == "append" else OpPrepend(arg)
        elif kind == "hexlify":
            op = OpHexlify()
        elif kind == "reverse":
            op = OpReverse()
        else:
            op = {"sha256": OpSHA256, "sha1": OpSHA1, "ripemd160": OpRIPEMD160,
                  "keccak256": OpKECCAK256}[kind]()
        try:
            op(msg)
        except Exception:  # noqa: BLE001 - too long: another operation
            return None
        return op

    def extend(self, ts, steps, mixed):
        """`steps` operations down from `ts`: the last node."""
        for _ in range(steps):
            op = self.an_op(ts.msg, mixed)
            if op is not None:
                ts = ts.ops.add(op)
        return ts

    def build(self):
        """A proof (its bytes) and a header's merkle root it commits to."""
        rng = self.rng
        mixed = rng.random() < 0.35
        fop = rng.choice([OpSHA256] * 8 + [OpSHA1, OpRIPEMD160]) if mixed else OpSHA256
        digest = fop()(self.randbytes(rng.randint(0, 100)))
        root = Timestamp(digest)
        shape = rng.choice(["plain", "plain", "branches", "branches", "deep", "inner"])
        if shape == "deep":
            cur = root
            for _ in range(rng.choice([100, 200, 253, 254])):
                cur = cur.ops.add(rng.choice([OpSHA256(), OpSHA256(), OpReverse(),
                                              OpPrepend(b"\x00")]))
            tip = cur.ops.add(OpSHA256())
        else:
            final = rng.choice([OpSHA256()] * 8 + [OpKECCAK256(), OpSHA1()]) if mixed \
                else OpSHA256()
            tip = self.extend(root, rng.randint(1, 8), mixed).ops.add(final)
        tip.attestations.add(BitcoinBlockHeaderAttestation(rng.randint(0, 900_000)))
        if shape in ("branches", "inner"):
            nodes = [root]
            for _ in range(rng.randint(1, 4)):
                node = rng.choice(nodes)
                branch = self.extend(node, rng.randint(1, 4), mixed)
                branch.attestations.add(self.attestation())
                nodes.append(branch)
            if shape == "inner":
                root.attestations.add(self.attestation())
                tip.attestations.add(self.attestation())
        return serialize(fop(), root), (tip.msg if len(tip.msg) == 32 else self.randbytes(32))

    def merkle(self):
        rng = self.rng
        digest = OpSHA256()(self.randbytes(rng.randint(0, 100)))
        file_ts = Timestamp(digest)
        stamps = [Timestamp(self.randbytes(32)) for _ in range(rng.randint(0, 40))]
        stamps.insert(rng.randint(0, len(stamps)), file_ts)
        tip = make_merkle_tree(stamps, rng.choice([cat_sha256, cat_sha256d]))
        tip.attestations.add(BitcoinBlockHeaderAttestation(rng.randint(0, 900_000)))
        if rng.random() < 0.3:
            tip.attestations.add(PendingAttestation(self.uri()))
        return serialize(OpSHA256(), file_ts), tip.msg

    def header_for(self, root):
        raw = mine(root, self.rng)
        if raw is None:
            raise AssertionError("no nonce met the easy target")
        return raw

    def case_proofs(self):
        data, root = self.build()
        self.both_headers(data, root)

    def case_merkle(self):
        data, root = self.merkle()
        got = self.both_headers(data, root)
        if got is None:
            raise AssertionError("a merkle tree built by the reference was refused")

    def both_headers(self, data, root):
        digest = None
        got = self.read_both(data)
        if got is None:
            raise AssertionError("a proof built by the reference was refused")
        digest = bytes.fromhex(got["digest"])
        good = self.header_for(root)
        self.verify_both(data, digest, good)
        self.verify_both(data, digest, self.header_for(self.randbytes(32)))
        self.verify_both(data, bytes(self.randbytes(len(digest))), good)
        return got

    def case_mutated(self):
        rng = self.rng
        data, root = self.build() if rng.random() < 0.7 else self.merkle()
        header = self.header_for(root)
        for _ in range(rng.randint(2, 6)):
            data = self.mutate(data)
            self.walk(data, header)

    def mutate(self, data):
        rng = self.rng
        if not data:
            return self.randbytes(rng.randint(1, 3))
        kind = rng.choice(["flip", "set", "cut", "add", "insert", "delete", "flip"])
        i = rng.randrange(len(data))
        if kind == "flip":
            return data[:i] + bytes([data[i] ^ rng.randint(1, 255)]) + data[i + 1:]
        if kind == "set":
            return data[:i] + bytes([rng.choice([0x00, 0x80, 0xFF, 0x01, 0x08])]) + data[i + 1:]
        if kind == "cut":
            return data[:rng.randrange(len(data))]
        if kind == "add":
            return data + self.randbytes(rng.randint(1, 3))
        if kind == "insert":
            return data[:i] + bytes([rng.choice([0x00, 0x80, 0xFF, rng.randrange(256)])]) + data[i:]
        return data[:i] + data[i + 1:]

    def case_headers(self):
        rng = self.rng
        bits = rng.choice([EASY_BITS, EASY_BITS, 0x2000FFFF, 0x1D00FFFF, 0x1B0404CB, 0, 0x01003456,
                           0x02008000, 0x04800000, 0x03000000, 0x01120000, 0xFF7FFFFF, 0x20FFFFFF,
                           0x2100FFFF, 0x2200FFFF, 0x21007FFF, 0x207FFFFF - 1, 0x03123456,
                           rng.randrange(2 ** 32)])
        root = self.randbytes(32)
        raw = mine(root, rng, bits, 3000) if compact(bits)[0] >= 1 << 244 else None
        if raw is None:
            raw = (struct.pack("<I", rng.randrange(2 ** 32)) + self.randbytes(32) + root
                   + struct.pack("<III", rng.randrange(2 ** 32), bits, rng.randrange(2 ** 32)))
        self.header_both(raw)
        if rng.random() < 0.3:
            self.header_both(raw[:rng.choice([0, 1, 79])])
            self.header_both(raw + self.randbytes(rng.choice([1, 80])))

    # ----- limits and the listed differences ----------------------------------------------

    def tree_of(self, ops, tail=None):
        return b"".join(ops) + (tail or raw_height(TAG_BITCOIN, 1))

    def agreed(self, data, ok, label):
        """Both read `data`, or both refuse it, as `ok` says."""
        got = self.read_both(data, label)
        if (got is not None) != ok:
            raise AssertionError(f"{label}: read is {got is not None}, expected {ok}")
        return got

    def limits(self):
        sha = b"\x08"
        header = self.header_for(self.randbytes(32))
        for n, ok in ((254, True), (255, True), (256, False), (257, False)):
            self.agreed(raw_proof(self.tree_of([sha] * n)), ok, f"a chain of {n} operations")
        # A value of 4,096 bytes is the longest a path may reach: a prepend of
        # 4,064 bytes to the 32-byte digest, then 4,065.
        for n, ok in ((4064, True), (4065, False), (4096, False), (4097, False)):
            tree = b"\xf1" + varbytes(b"\x07" * n) + b"\x08" + raw_height(TAG_BITCOIN, 1)
            label = f"a prepend of {n} bytes to a value of 32"
            got = self.agreed(raw_proof(tree), ok, label)
            if got:
                self.walk(raw_proof(tree), header, bytes.fromhex(got["digest"]))
        self.agreed(raw_proof(b"\xf1" + varuint(0) + raw_height(TAG_BITCOIN, 1)), False,
                    "an empty argument")
        # Hexlify of a value of 2,048 and 2,049 bytes.
        for n, ok in ((2048, True), (2049, False)):
            tree = b"\xf1" + varbytes(b"\x07" * (n - 32)) + b"\xf3" + raw_height(TAG_BITCOIN, 1)
            self.agreed(raw_proof(tree), ok, f"a hexlify of {n} bytes")
        for n, ok in ((8192, True), (8193, False)):
            self.agreed(raw_proof(raw_attestation(b"\x01" * 8, b"\x02" * n)), ok,
                        f"an unknown attestation of {n} bytes")
        for n, ok in ((1000, True), (1001, False)):
            self.agreed(raw_proof(raw_attestation(TAG_PENDING, varbytes(b"a" * n))), ok,
                        f"an address of {n} bytes")
        self.agreed(raw_proof(raw_attestation(TAG_PENDING, varbytes(b"a b"))), False,
                    "an address with a space")
        self.agreed(raw_proof(raw_attestation(TAG_PENDING, varbytes(b"a") + b"\x00")), False,
                    "bytes after an address")
        for tag in (TAG_BITCOIN, TAG_LITECOIN, TAG_ETHEREUM):
            for height, ok in ((2 ** 32 - 1, True), (2 ** 32, False), (2 ** 63 - 1, False)):
                self.agreed(raw_proof(raw_height(tag, height)), ok, f"a height of {height}")
            self.agreed(raw_proof(raw_attestation(tag, varuint(1) + b"\x00")), False,
                        "bytes after a height")
            self.agreed(raw_proof(raw_attestation(tag, b"")), False, "no height")
        self.agreed(raw_proof(raw_height(TAG_BITCOIN, 5), version=b"\x02"), False, "version 2")
        self.agreed(raw_proof(raw_height(TAG_BITCOIN, 5), file_op=b"\xf3"), False, "file op")
        self.agreed(raw_proof(raw_height(TAG_BITCOIN, 5), file_op=b"\x07"), False, "file op 7")
        self.agreed(raw_proof(b"\x08"), False, "an operation and nothing after it")
        self.agreed(raw_proof(b"\xff\x00"), False, "a continuation and no entry")
        self.agreed(raw_proof(b"\x09" + raw_height(TAG_BITCOIN, 5)), False, "an unknown op")
        self.agreed((MAGIC + b"\x01\x08")[:-1], False, "a proof cut in its header")
        self.agreed(MAGIC[:-1], False, "a magic cut short")
        self.agreed(b"", False, "nothing")
        # 9 bytes of padding are read by both.
        self.agreed(raw_proof(raw_attestation(TAG_BITCOIN, varuint(5, 9), 9)), True,
                    "varuints padded to nine bytes")

    def sized(self, total):
        """A proof of `total` bytes of unknown attestations at the root, each
        with a payload of a few thousand bytes."""
        head = raw_proof(b"")
        tree_len = total - len(head)
        k = 1
        while tree_len - 12 * k + 1 > 8192 * k:
            k += 1
        payload = tree_len - 12 * k + 1
        sizes = [payload // k + (1 if i < payload % k else 0) for i in range(k)]
        entries = [raw_attestation((i + 1).to_bytes(8, "big"), b"\xaa" * n)
                   for i, n in enumerate(sizes)]
        data = head + b"".join(b"\xff" + e for e in entries[:-1]) + entries[-1]
        if len(data) != total:
            raise AssertionError("a proof of the wrong size was built")
        return data

    def known_differences(self):
        header = self.header_for(self.randbytes(32))
        base = raw_height(TAG_BITCOIN, 5)
        # A varuint of ten bytes or more, in a height, a payload's length and
        # an argument's length.
        for pad in (10, 12):
            for name, data in (
                    ("a height", raw_proof(raw_attestation(TAG_BITCOIN, varuint(5, pad)))),
                    ("a payload length", raw_proof(raw_attestation(TAG_BITCOIN, varuint(5),
                                                                   pad))),
                    ("an argument length",
                     raw_proof(b"\xf0" + varuint(2, pad) + b"\x01\x02" + base))):
                self.differs(data, "varuint exceeds 9 bytes", f"{name} padded to {pad} bytes",
                             header)
        self.differs(raw_proof(base, version=b"\x81\x00"), "UnsupportedMajorVersion",
                     "a version written in two bytes", header)
        keccak = raw_proof(base, file_op=b"\x67")
        self.differs(keccak, "Unknown unary op tag 0x67", "a file taken with Keccak-256", header)
        # An attestation twice at a node, and an operation twice.
        twice = raw_proof(b"\xff" + raw_height(TAG_BITCOIN, 5) + raw_height(TAG_BITCOIN, 5))
        got = self.read_both(twice, "an attestation repeated at a node")
        if got is None or not got["repeated_attestation"]:
            raise AssertionError("a repeated attestation is no longer read by Faraday")
        sha = raw_proof(b"\xff\x08" + base + b"\x08" + base)
        self.differs(sha, "repeated", "an operation repeated at a node", header)
        # A proof of 64 KiB is read by both, one byte more only by the reference.
        self.agreed(self.sized(64 * 1024), True, "a proof of 64 KiB")
        before = self.known
        if self.read_both(self.sized(64 * 1024 + 1), "a proof over 64 KiB") is not None \
                or self.known != before + 1:
            raise AssertionError("a proof over 64 KiB is no longer refused by Faraday alone")
        # A path through a hash other than SHA-256 that reaches the root.
        for hasher in (OpSHA1, OpRIPEMD160, OpKECCAK256):
            digest = OpSHA256()(self.randbytes(20))
            root = Timestamp(digest)
            tip = root.ops.add(OpAppend(b"\x01")).ops.add(hasher())
            if len(tip.msg) != 32:
                tip = tip.ops.add(OpSHA256())
            tip.attestations.add(BitcoinBlockHeaderAttestation(7))
            before = self.known
            got = self.verify_both(serialize(OpSHA256(), root), digest, self.header_for(tip.msg))
            if got["committed"] or self.known != before + 1:
                raise AssertionError(f"a path through {hasher.TAG_NAME} was counted by Faraday "
                                     "or not by the reference")
        # A file taken with SHA-1 or RIPEMD-160.
        for fop in (OpSHA1, OpRIPEMD160):
            digest = fop()(b"a file")
            root = Timestamp(digest)
            root.ops.add(OpSHA256()).attestations.add(BitcoinBlockHeaderAttestation(9))
            tip = root.ops[OpSHA256()]
            before = self.known
            got = self.verify_both(serialize(fop(), root), digest, self.header_for(tip.msg))
            if got["committed"] or self.known != before + 1:
                raise AssertionError("a file taken with a hash other than SHA-256 was counted")
        # Ethereum and Litecoin.
        digest = OpSHA256()(b"a file")
        root = Timestamp(digest)
        root.ops.add(OpSHA256())
        tip = root.ops[OpSHA256()]
        tip.attestations.add(EthereumBlockHeaderAttestation(5))
        tip.attestations.add(LitecoinBlockHeaderAttestation(6))
        before = self.known
        self.verify_both(serialize(OpSHA256(), root), digest, header)
        if self.known != before + 1:
            raise AssertionError("the reference no longer verifies an Ethereum attestation")

    def differs(self, data, why, label, header):
        """Faraday reads `data` and the reference refuses it for `why`."""
        inputs = {"proof": data.hex()}
        rust = self.rust.call("ots.read", **inputs)
        ref = reference(lambda: ref_read(data))
        self.compared += 1
        if rust[0] != "ok" or ref[0] != "err" or why not in ref[1] \
                or not self.explained(data, rust[1], ref[1]):
            self.fail("ots.read", inputs, rust, ref, f"the known difference no longer holds: {label}")
        self.known += 1

    def prelude(self):
        self.shims()
        self.published()
        self.limits()
        self.known_differences()
