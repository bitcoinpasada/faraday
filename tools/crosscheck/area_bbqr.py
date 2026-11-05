"""BBQr (`faraday_qr::bbqr`) against Coinkite's own Python library
(`tools/reference/bbqr/`) and krux's `qr.py` (`tools/reference/krux/`), and
the numbered text parts Specter writes (`faraday_qr::specter`) against
SeedSigner's own reader, its `SpecterWalletQrDecoder` from `decode_qr.py`
(`tools/reference/seedsigner/`, vendored unchanged;
`tools/reference/specter/README.md`) and krux's "pMofN" reader.

- Coinkite's test files (`tools/vectors/bbqr/`): the 26 parts of
  `real-scan.txt`, and each of the ten PSBTs and transactions split by
  Coinkite in every encoding, read by Faraday and by both judges.
- The corpus is random files of every type and size, written by krux's
  `to_qr_codes` and `encode_bbqr` (`H`, `2` and `Z`, at the sizes of QR
  versions 1 to 20), and split by Coinkite's `split_qrs` in each encoding
  at random version limits (up to 39: see below), then read by Faraday in
  random order with repeats, a part missing, a part from another transfer,
  a repeat with other contents and a part number past the count, against
  Coinkite's `join_qrs` and krux's `QRPartParser`.
- What Faraday writes (`2` only, a fixed number of bytes a part) is krux's
  own writing of the same file at the same part size, part for part (a
  file type krux does not write is refused by both), and is read back by
  `join_qrs` and by krux.
- Numbered parts written by krux ("pMofN", at the sizes of versions 1 to
  20) are Faraday's own writing for the same text and part size, part for
  part; they are read back by SeedSigner's decoder and by krux, and random
  sets are read in any order against SeedSigner's decoder, with the same
  faults, a number of 0, and malformed headers.

Known differences, each checked to still hold and counted:

- Faraday reads the file types `P`, `T`, `U` and `J` only; BBQr also
  defines `C`, `X`, `B`, `R`, `S` and `E`, which Coinkite reads.
- Faraday reads frames of at most 4,096 characters; a version 40 part
  can carry 4,296, so the corpus keeps Coinkite to version 39 and checks
  a 4,296-character frame is refused.
- Faraday refuses a set whose parts are not all one length but the last
  (shorter or equal); Coinkite's `join_qrs` decodes each part on its own.
  The corpus does not make such sets. krux's writer can make one: when it
  rounds the size of the last part up, the last part can be longer than
  the others, which Faraday refuses (counted, below).
- SeedSigner matches `P` and `OF` in upper case; Faraday does not. A
  count over 1,000 is refused by Faraday. SeedSigner files a part number
  of 0 as the last part, fails on one past the count, keeps the first of
  a part twice with other text and takes a part with no header as the
  whole text; Faraday refuses all four, which the area keeps in front of
  SeedSigner's decoder (the numbers, the text and the putting together
  are SeedSigner's).
"""

import re

from common import ROOT, Area, reference, use_reference

use_reference("bbqr", "shims")
use_reference("bbqr")
use_reference("qrcodegen")
# krux first: its `qr.py` binds the `qrcode` shim as it is imported, and
# SeedSigner's stand-in below registers a module of the same name.
use_reference("krux", "shims")
use_reference("krux", "src")
from krux import qr as krux_qr  # noqa: E402
from krux import bbqr as krux_bbqr  # noqa: E402
from krux.bbqr import encode_bbqr  # noqa: E402

use_reference("seedsigner", "shims")
use_reference("seedsigner")
use_reference("embit")
import outside_imports  # noqa: E402

outside_imports.install()
from seedsigner.models.decode_qr import SpecterWalletQrDecoder  # noqa: E402
from bbqr import join_qrs, split_qrs  # noqa: E402
from bbqr.utils import int2base36, version_to_chars  # noqa: E402
from qrcodegen import QrCode, QrSegment  # noqa: E402

FARADAY_TYPES = "PTUJ"
ALL_TYPES = "PTJCUXBRSE"
MAX_FRAME = 4096
MAX_PARTS = 1295
KRUX_PMOFN_MAX = 99
VECTORS = ROOT / "tools" / "vectors" / "bbqr"
SPECTER_MAX = 1000


def check_capacities():
    """The shim's table against Nayuki's encoder."""
    for v in range(1, 41):
        bits = QrCode._get_num_data_codewords(v, QrCode.Ecc.LOW) * 8
        cc = QrSegment.Mode.ALPHANUMERIC.num_char_count_bits(v)
        n = version_to_chars(v)
        fits = lambda k: 4 + cc + 11 * (k // 2) + 6 * (k % 2) <= bits  # noqa: E731
        if not fits(n) or fits(n + 1):
            raise AssertionError(f"the pyqrcode shim's capacity at version {v} is wrong")


def check_krux_capacities():
    """krux's tables of how many alphanumeric characters and bytes a QR
    code of versions 1 to 20 holds at level L, against Nayuki's encoder."""
    for v in range(1, 21):
        bits = QrCode._get_num_data_codewords(v, QrCode.Ecc.LOW) * 8
        cc_alnum = QrSegment.Mode.ALPHANUMERIC.num_char_count_bits(v)
        cc_byte = QrSegment.Mode.BYTE.num_char_count_bits(v)
        tables = (
            (krux_qr.QR_CAPACITY_ALPHANUMERIC, lambda k: 4 + cc_alnum + 11 * (k // 2) + 6 * (k % 2)),
            (krux_qr.QR_CAPACITY_BYTE, lambda k: 4 + cc_byte + 8 * k),
        )
        for table, needs in tables:
            n = table[v - 1]
            if needs(n) > bits or needs(n + 1) <= bits:
                raise AssertionError(f"krux's capacity table is wrong at version {v}")


def krux_width(version):
    """The `max_width` krux's `max_qr_bytes` reads as a QR code of this
    version (17 + 4 * version modules, and its 2-module frame)."""
    return 4 * version + 19


def krux_parts(data, qr_format, version):
    """The parts krux's `to_qr_codes` writes for `data` (a `BBQrCode` or
    the text of "pMofN") at a version's size; the generator repeats, so
    the count it states is taken."""
    gen = krux_qr.to_qr_codes(data, krux_width(version), qr_format)
    first, n = next(gen)
    return [first] + [next(gen)[0] for _ in range(n - 1)]


def krux_bbqr_parts(file_type, data, encoding, version):
    """krux's BBQr parts for a file: its `encode_bbqr` (`Z` becomes `2`
    when deflate does not help) and `to_qr_codes`."""
    return krux_parts(encode_bbqr(data, encoding, file_type), krux_qr.FORMAT_BBQR, version)


def equal_parts(bodies):
    """Whether a set is written the way Faraday writes one: every body but
    the last of one length and the last of one to that many characters
    (krux can write an empty last part, or a longer one)."""
    return all(len(b) == len(bodies[0]) for b in bodies[:-1]) \
        and 1 <= len(bodies[-1]) <= len(bodies[0])


def krux_read(parts):
    """A set read by krux's `QRPartParser`, one part at a time: its
    parser, which has refused (`ValueError`) what it refuses."""
    reader = krux_qr.QRPartParser()
    for p in parts:
        reader.parse(p)
    return reader


def krux_bbqr_join(parts):
    """Where a set stands after its parts, read by krux: the answer of
    `bbqr_join`. krux reads `P`, `T`, `J` and `U` only and decodes the
    last two as UTF-8, which the caller keeps apart."""
    reader = krux_read(parts)
    if reader.format != krux_qr.FORMAT_BBQR:
        raise ValueError("krux does not take this as BBQr")
    missing = [i + 1 for i in range(reader.total) if i not in reader.parts]
    if missing:
        return {"part": {"have": reader.total - len(missing), "total": reader.total,
                         "missing": missing}}
    got = reader.result()
    return {"done": {"type": reader.bbqr.file_type,
                     "data": (got.encode() if isinstance(got, str) else bytes(got)).hex()}}


def krux_pmofn_join(parts):
    """Where a set of numbered text parts stands after its parts, read by
    krux's `QRPartParser` ("pMofN", at most 99 parts)."""
    reader = krux_read(parts)
    if reader.format != krux_qr.FORMAT_PMOFN:
        raise ValueError("krux does not take this as pMofN")
    missing = [i + 1 for i in range(reader.total) if (i + 1) not in reader.parts]
    if missing:
        return {"part": {"have": reader.total - len(missing), "total": reader.total,
                         "missing": missing}}
    return {"done": reader.result()}


def bbqr_join(parts):
    """Where a set stands after its parts, read with `join_qrs`: the
    parts present counted as BBQr's header numbers them."""
    hdr = {p[0:6] for p in parts}
    if len(hdr) != 1:
        raise ValueError("parts of more than one transfer")
    total = int(parts[0][4:6], 36)
    seen = {}
    for p in parts:
        idx = int(p[6:8], 36)
        if idx >= total:
            raise ValueError("a part past the count")
        if seen.setdefault(idx, p[8:]) != p[8:]:
            raise ValueError("a part twice with other contents")
    missing = [i + 1 for i in range(total) if i not in seen]
    if missing:
        return {"part": {"have": total - len(missing), "total": total, "missing": missing}}
    file_type, raw = join_qrs(parts)
    return {"done": {"type": file_type, "data": raw.hex()}}


SPECTER_PART = re.compile(r"^p(\d+)of(\d+) (.+$)")


def specter_parse(text):
    """A part's number, count and text as SeedSigner's
    `SpecterWalletQrDecoder` reads them, behind Faraday's own rule, which
    is stricter: a header in lower case (SeedSigner matches `P` and `OF`
    too, and takes a part without a header as the whole text), a number of
    1 to the count (SeedSigner files 0 as the last part and fails on one
    past the count), and a count of 1,000 at most."""
    if not SPECTER_PART.match(text):
        raise ValueError("not a numbered part")
    d = SpecterWalletQrDecoder()
    n, total = d.current_segment_num(text), d.total_segment_nums(text)
    if not 1 <= n <= total or total > SPECTER_MAX:
        raise ValueError("a part number of 0, past the count, or over 1,000")
    return n, total, d.parse_segment(text)


def specter_join(parts):
    """Parts put together by SeedSigner's decoder (`add`, its `segments`).
    Faraday's refusal of a part twice with other text stays here (SeedSigner
    keeps the first), and the check of the text as JSON that follows the
    last part is let to raise `TypeError` (JSON that is not an object),
    the parts being filed before it."""
    d = SpecterWalletQrDecoder()
    for p in parts:
        n, m, body = specter_parse(p)
        if len(d.segments) == m and d.segments[n - 1] not in (None, body):
            raise ValueError("a part twice with other text")
        try:
            d.add(p)
        except TypeError:
            pass
        except Exception as e:
            raise ValueError(str(e))
    total = d.total_segments
    missing = [i + 1 for i, s in enumerate(d.segments) if s is None]
    if missing:
        return {"part": {"have": total - len(missing), "total": total, "missing": missing}}
    return {"done": "".join(d.segments)}


class Bbqr(Area):
    name = "bbqr"
    faraday = (
        '`faraday_qr::bbqr`, `faraday_qr::specter` (numbered text parts)'
    )
    reference = (
        "Coinkite's BBQr library (`bbqr/`, with a one-table pyqrcode shim checked "
        "against `qrcodegen.py`); krux's `qr.py` and `bbqr.py` (`krux/`, unchanged, "
        "with three small shims for the firmware's QR, base32 and deflate modules); "
        "SeedSigner's own `SpecterWalletQrDecoder` (`seedsigner/`, unchanged)"
    )
    reference_checked = (
        "`check_capacities` and `check_krux_capacities` run first, the pyqrcode shim's "
        "table of version capacities and krux's two tables against `qrcodegen.py`'s; "
        "the vendored files are hashed against `references.sha256` before any run, and "
        "a published vector's answer is required of the reference as well as of Faraday; "
        "where Faraday refuses what SeedSigner's decoder files or misfiles (a part "
        "number of 0 or past the count, a repeat with other text, a header in upper case "
        "or missing, a count over 1,000) the refusal is written in the area, in front of "
        'the decoder'
    )
    vectors = (
        "Coinkite's BBQr test files (`tools/vectors/bbqr/`): the 26 parts of a phone's "
        "scan, which read as a `U` file with the text Coinkite's test expects, and its "
        "ten PSBTs and transactions, each split in every encoding and read back to the "
        "file's bytes"
    )
    corpus = (
        'files of every type and size written by krux (at the sizes of QR versions 1 to '
        "20) and split by Coinkite in each encoding, read in any order, with repeats, a "
        "part missing, another transfer's part, a conflicting repeat and a part past the "
        "count, against Coinkite and krux; what Faraday writes, part for part krux's "
        'own writing at the same size, read back by `join_qrs` and by krux; numbered '
        "parts written by krux, written again by Faraday, read in any order against "
        "SeedSigner's decoder and krux and refused when malformed"
    )
    differences = [
        (
            'BBQr: Faraday reads the file types `P`, `T`, `U` and `J` and frames of at most'
            " 4,096 characters; Coinkite's library reads every type BBQr defines and a "
            'version 40 part of 4,296 characters. Both refusals are checked and counted, '
            'and the corpus splits at version 39 at most. Faraday also refuses a set whose '
            'parts are not one length but the last, which Coinkite never writes.'
        ),
        (
            'Numbered parts: SeedSigner matches `P` and `OF` in upper case; Faraday does '
            'not (checked and counted). Faraday also refuses a part number of 0 or past '
            'the count, a count over 1,000, a repeat with other text and a part with no '
            "header, which SeedSigner's decoder files, fails on or takes whole; those "
            'refusals are written in the area, in front of the decoder.'
        ),
        (
            "krux (checked and counted): reads the file types `P`, `T`, `J` and `U` "
            "only, decodes `J` and `U` as UTF-8 in `2` and `Z` only, stops at 160 KiB "
            "of characters (Coinkite's `signed.txn` is over it, in `H`), and can write "
            "a last part longer than the others, which Faraday refuses; its deflate "
            "reads a 1,024-byte window on a device, which the shim reads and writes. "
            "A set with an empty last part is read by Coinkite and krux and "
            "refused by Faraday, which refuses any part with no data."
        ),
    ]
    not_compared = (
        "krux's writing of file types Faraday does not write (`C`, `X`, `B`, `R`, `S`, "
        "`E` are refused by both and compared as refusals); a set over 1,295 parts, "
        "which krux cannot number; the old cases of Faraday's writing at 1, 5, 7 and "
        "40 bytes a part, gone because writing is now sized by QR version, the size "
        "krux chooses"
    )

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0

    def join(self, parts):
        """Faraday's reading of a set against Coinkite's and krux's. krux
        decodes `J` and `U` as UTF-8 in `2` and `Z`, which Coinkite and
        Faraday do not, and stops at 160 KiB of characters (each checked
        and counted)."""
        got = self.compare("bbqr.join", {"parts": parts}, lambda: bbqr_join(parts))
        ref = reference(lambda: bbqr_join(parts))
        answer = reference(lambda: krux_bbqr_join(parts))
        if answer[0] == "err" and "codec" in answer[1]:
            # `J` and `U` as `2` or `Z` (not `H`, which krux returns as bytes).
            if ref[0] != "ok" or parts[0][2] == "H":
                raise AssertionError(f"krux and text that is not UTF-8: {answer[1]!s:.200}")
            self.compared += 1
            self.known += 1
            return got
        if answer[0] == "err" and "payload too big" in answer[1]:
            # krux stops at 160 KiB of characters; the set must be over that.
            if sum(len(p) - 8 for p in set(parts)) <= krux_bbqr.BBQR_MAX_PAYLOAD_LEN:
                raise AssertionError("krux refuses a set that is not over its payload limit")
            self.compared += 1
            self.known += 1
            return got
        self.compare("bbqr.join", {"parts": parts}, lambda: krux_bbqr_join(parts))
        return got

    def write(self, file_type, data, version):
        """Faraday's writing against krux's, part for part, at the part
        size krux's `to_qr_codes` chose for a QR code of `version`; the
        parts are read back by Coinkite's `join_qrs` and krux. A type krux
        does not write is refused by both."""
        try:
            parts = krux_bbqr_parts(file_type, data, "2", version)
        except ValueError as e:
            if file_type in FARADAY_TYPES:
                return None  # more than 1,295 parts: krux cannot number them

            def refused():
                raise e
            self.compare("bbqr.encode", {"type": file_type, "data": data.hex(), "part_bytes": 40},
                         refused)
            return None
        if not equal_parts([p[8:] for p in parts]):
            # krux's last part is empty or longer than the others; Faraday writes neither.
            self.compared += 1
            self.known += 1
            return None
        size = (len(parts[0]) - 8) if len(parts) > 1 else -(-(len(parts[0]) - 8) // 8) * 8
        got = self.compare("bbqr.encode", {"type": file_type, "data": data.hex(),
                                           "part_bytes": size // 8 * 5},
                           lambda: {"parts": parts})
        back = join_qrs(parts)
        if back != (file_type, data):
            raise AssertionError(f"join_qrs does not read the parts back: {back}")
        reader = krux_read(parts)
        if file_type in "PT" and bytes(reader.result()) != data:
            raise AssertionError("krux does not read the parts back")
        return got

    def known_difference(self, parts, why):
        got = self.rust.call("bbqr.join", parts=parts)
        self.compared += 1
        if got[0] != "err":
            raise AssertionError(f"known difference no longer holds ({why}): {got}")
        self.known += 1

    def a_file(self):
        rng = self.rng
        n = rng.choice([1, 2, rng.randint(1, 100), rng.randint(100, 3000), rng.randint(3000, 12000)])
        if rng.random() < 0.5:
            # Text compresses, so `Z` is chosen when no encoding is given.
            words = ["psbt", "cHNidP8B", "wsh(", "sortedmulti", "0", "1", "xpub", "/"]
            return "".join(rng.choice(words) for _ in range(n // 4 + 1)).encode()[:n]
        return self.randbytes(n)

    def split(self, data, file_type):
        rng = self.rng
        if rng.random() < 0.5:
            try:
                parts = krux_bbqr_parts(file_type, data, rng.choice("H2Z"), rng.randint(1, 20))
            except ValueError:
                return None
            if not equal_parts([p[8:] for p in parts]):
                self.known_difference(parts, "krux's last part")
                return None
            return parts
        encoding = rng.choice([None, "2", "H", "Z"])
        lo = rng.randint(1, 39)
        hi = rng.randint(lo, 39)
        try:
            _, parts = split_qrs(data, file_type, encoding, min_version=lo, max_version=hi)
        except (ValueError, AssertionError):
            return None
        return parts

    def corpus_case(self):
        rng = self.rng
        data = self.a_file()
        file_type = rng.choice(FARADAY_TYPES)
        parts = self.split(data, file_type)
        if parts:
            order = parts[:]
            rng.shuffle(order)
            if rng.random() < 0.3:
                order += rng.sample(parts, k=min(len(parts), rng.randint(1, 3)))
            r = rng.random()
            if r < 0.15 and len(order) > 1:
                order.remove(rng.choice(parts))
            elif r < 0.25:
                other = self.split(self.a_file(), rng.choice(FARADAY_TYPES))
                if other:
                    order.insert(rng.randrange(len(order) + 1), rng.choice(other))
            elif r < 0.3 and len(parts[0]) > 9:
                p = rng.choice(parts)
                q = p[:-1] + ("A" if p[-1] != "A" else "B")
                if p[2] != "H":
                    order.append(q)
            elif r < 0.35:
                p = rng.choice(parts)
                order.append(p[:6] + int2base36(int(p[4:6], 36) + rng.randint(0, 3)) + p[8:])
            self.join(order)
        if rng.random() < 0.05:
            t = rng.choice([c for c in ALL_TYPES if c not in FARADAY_TYPES])
            other = self.split(self.randbytes(rng.randint(1, 200)), t)
            if other:
                self.known_difference(other, f"file type {t}")
        self.write(rng.choice(FARADAY_TYPES + "C"), data[:rng.randint(1, len(data))],
                   rng.randint(1, 20))
        self.specter_case()

    def specter_case(self):
        rng = self.rng
        alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=()[],'*# "
        text = "".join(rng.choice(alphabet) for _ in range(rng.randint(1, 3000)))
        # krux writes the parts at the size of a QR code of some version, and
        # Faraday writes the text again in pieces of the size krux chose.
        krux = krux_parts(text, krux_qr.FORMAT_PMOFN, rng.randint(1, 20))
        bodies = [p.split(" ", 1)[1] for p in krux]
        if not equal_parts(bodies):
            # krux's last part is empty or longer than the others; Faraday writes neither.
            self.compared += 1
            self.known += 1
            return
        got = self.compare("specter.encode", {"text": text, "chunk": len(bodies[0])},
                           lambda: {"parts": krux})
        parts = self.need(got, "specter.encode")["parts"]
        # (A set over 1,000 parts is past what Faraday reads back.)
        back = specter_join(parts) if len(parts) <= SPECTER_MAX else {"done": text}
        if back != {"done": text}:
            raise AssertionError(f"SeedSigner's decoder does not read Faraday's parts back: {back}")
        order = parts[:]
        rng.shuffle(order)
        if len(order) > 1 and rng.random() < 0.2:
            order.pop()
        r = rng.random()
        if r >= 0.2 and len(parts) <= KRUX_PMOFN_MAX:
            self.compare("specter.join", {"parts": order}, lambda: krux_pmofn_join(order))
        if r < 0.05:
            order.append(f"p0of{len(parts)} x")
        elif r < 0.1:
            order.append(f"p{len(parts) + 1}of{len(parts)} x")
        elif r < 0.15:
            order.append(f"p1of{len(parts) + 1} x")
        elif r < 0.2 and parts[0].split(" ", 1)[1] != "x":
            order.append(parts[0].split(" ", 1)[0] + " x")
        if order and len(order) <= SPECTER_MAX:
            self.compare("specter.join", {"parts": order}, lambda: specter_join(order))
        for frame in [rng.choice(parts), f"p{rng.randint(0, 5)}of{rng.randint(0, 5)} y",
                      f"p1of2", f"p1of2  ", f"p01of002 z", f"p1 of2 z", f"q1of2 z",
                      f"p{rng.randint(1, 9)}of{rng.randint(990, 1010)} z"]:
            self.compare("specter.parse", {"text": frame}, lambda frame=frame: dict(
                zip(["n", "m", "body"], specter_parse(frame))))

    def specter_known(self):
        for frame in ["P1OF2 abc", "P1of2 abc", "p1OF2 abc"]:
            got = self.rust.call("specter.parse", text=frame)
            self.compared += 1
            if got[0] != "err":
                raise AssertionError(f"Faraday now reads {frame!r}, which it refused")
            self.known += 1

    LOOPS = (("corpus", 1500),)

    def published(self):
        """Coinkite's test files (`tools/vectors/bbqr/README.md`)."""
        lines = [ln.strip() for ln in (VECTORS / "real-scan.txt").read_text().splitlines()
                 if ln.strip()]
        for order in (lines, lines[::-1]):
            done = self.need(self.join(order), "bbqr.join")["done"]
            data = bytes.fromhex(done["data"])
            if done["type"] != "U" or b"Zlib compressed" not in data or b"PSBT" not in data:
                raise AssertionError("real-scan.txt is not the text Coinkite's test expects")
        for f in sorted(VECTORS.iterdir()):
            if f.suffix not in (".psbt", ".txn"):
                continue
            raw, file_type = f.read_bytes(), "P" if f.suffix == ".psbt" else "T"
            for encoding in (None, "2", "H", "Z"):
                for lo, hi in ((1, 39), (20, 30)):
                    _, parts = split_qrs(raw, file_type, encoding, min_version=lo, max_version=hi)
                    done = self.need(self.join(parts[::-1]), "bbqr.join")["done"]
                    if (done["type"], bytes.fromhex(done["data"])) != (file_type, raw):
                        raise AssertionError(f"{f.name}: Faraday does not read back the file")

    def empty_last(self):
        """A set whose last part has no contents (krux can write one, and
        Coinkite's reader and krux's read it): Faraday refuses a part with
        no data, which is counted as a difference; reading it would fail."""
        for encoding, body in (("2", "NBSWY3DP"), ("H", "68656C6C6F")):
            parts = [f"B${encoding}T0200{body}", f"B${encoding}T0201"]
            want = bbqr_join(parts)
            if krux_bbqr_join(parts) != want or want != {"done": {"type": "T", "data": b"hello".hex()}}:
                raise AssertionError("the judges do not agree on a set with an empty last part")
            got = self.rust.call("bbqr.join", parts=parts)
            self.compared += 1
            if got[0] != "err":
                raise AssertionError(f"Faraday no longer refuses an empty last part: {got}")
            self.known += 1

    def prelude(self):
        check_capacities()
        check_krux_capacities()
        self.published()
        self.empty_last()
        frame = "B$2P0100" + "A" * (4296 - 8)
        self.known_difference([frame], "a 4,296-character frame")
        self.specter_known()

    def case_corpus(self):
        self.corpus_case()
