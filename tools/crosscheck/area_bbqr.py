"""BBQr (`faraday_qr::bbqr`) against Coinkite's own Python library
(`tools/reference/bbqr/`), and the numbered text parts Specter writes
(`faraday_qr::specter`) against a reader written here from SeedSigner's
(`tools/reference/specter/README.md`).

- BBQr's test files from Coinkite's repository are not vendored; the
  corpus is random files of every type and size, split by Coinkite's
  `split_qrs` in each encoding (`2`, `H`, `Z`) at random version limits
  (up to 39: see below), then read by Faraday in random order with
  repeats, a part missing, a part from another transfer, a repeat with
  other contents and a part number past the count, against Coinkite's
  `join_qrs`.
- What Faraday writes (`2` only, a fixed number of bytes a part), made
  again with Coinkite's base32 and part header and read back by
  `join_qrs`.
- Numbered parts written by Faraday and read back by the reader, and
  random sets read in any order, with the same faults, a number of 0,
  and malformed headers.

Known differences, each checked to still hold and counted:

- Faraday reads the file types `P`, `T`, `U` and `J` only; BBQr also
  defines `C`, `X`, `B`, `R`, `S` and `E`, which Coinkite reads.
- Faraday reads frames of at most 4,096 characters; a version 40 part
  can carry 4,296, so the corpus keeps Coinkite to version 39 and checks
  a 4,296-character frame is refused.
- Faraday refuses a set whose parts are not all one length but the last
  (shorter or equal); Coinkite's `join_qrs` decodes each part on its own.
  The corpus does not make such sets.
- SeedSigner matches `P` and `OF` in upper case; Faraday does not. A
  count over 1,000 is refused by Faraday.
"""

import re
from base64 import b32encode

from common import Area, use_reference

use_reference("bbqr", "shims")
use_reference("bbqr")
use_reference("qrcodegen")
from bbqr import join_qrs, split_qrs  # noqa: E402
from bbqr.utils import encode_data, int2base36, version_to_chars  # noqa: E402
from qrcodegen import QrCode, QrSegment  # noqa: E402

FARADAY_TYPES = "PTUJ"
ALL_TYPES = "PTJCUXBRSE"
MAX_FRAME = 4096
MAX_PARTS = 1295
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


def bbqr_write(file_type, data, part_bytes):
    """Faraday's way of writing, `2` with `part_bytes` a part (rounded
    down to a multiple of 5, at least 5), in Coinkite's base32 and
    header."""
    if file_type not in FARADAY_TYPES:
        raise ValueError("a type Faraday does not write")
    per = max(part_bytes // 5, 1) * 8
    encoding, encoded, _ = encode_data(data, "2")
    total = max(1, -(-len(encoded) // per))
    if total > MAX_PARTS:
        raise ValueError("over 1,295 parts")
    return {"parts": [f"B$2{file_type}" + int2base36(total) + int2base36(n)
                      + encoded[off:off + per] for n, off in enumerate(range(0, len(encoded), per))]}


SPECTER_PART = re.compile(r"^p(\d+)of(\d+) (.+$)")


def specter_parse(text):
    """SeedSigner's pattern, matched as written but case-sensitive
    (Faraday's reading; see the known differences)."""
    m = SPECTER_PART.match(text)
    if not m:
        raise ValueError("not a numbered part")
    n, total = int(m.group(1)), int(m.group(2))
    if not 1 <= n <= total or total > SPECTER_MAX:
        raise ValueError("a part number of 0, past the count, or over 1,000")
    return n, total, m.group(3)


def specter_join(parts):
    total, seen = None, {}
    for p in parts:
        n, m, body = specter_parse(p)
        if total is not None and m != total:
            raise ValueError("the count changed")
        total = m
        if seen.setdefault(n, body) != body:
            raise ValueError("a part twice with other text")
    missing = [i for i in range(1, total + 1) if i not in seen]
    if missing:
        return {"part": {"have": total - len(missing), "total": total, "missing": missing}}
    return {"done": "".join(seen[i] for i in range(1, total + 1))}


class Bbqr(Area):
    name = "bbqr"

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0

    def join(self, parts):
        return self.compare("bbqr.join", {"parts": parts}, lambda: bbqr_join(parts))

    def write(self, file_type, data, part_bytes):
        got = self.compare("bbqr.encode", {"type": file_type, "data": data.hex(),
                                           "part_bytes": part_bytes},
                           lambda: bbqr_write(file_type, data, part_bytes))
        if got[0] == "ok":
            back = join_qrs(got[1]["parts"])
            if back != (file_type, data):
                raise AssertionError(f"join_qrs does not read Faraday's parts back: {back}")
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
                   rng.choice([1, 5, 7, 40, rng.randint(1, 2000)]))
        self.specter_case()

    def specter_case(self):
        rng = self.rng
        alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=()[],'*# "
        text = "".join(rng.choice(alphabet) for _ in range(rng.randint(1, 3000)))
        chunk = rng.choice([1, 50, rng.randint(1, 500), 4000])
        got = self.compare("specter.encode", {"text": text, "chunk": chunk}, lambda: {
            "parts": [f"p{i + 1}of{m} {text[i * chunk:(i + 1) * chunk]}"
                      for m in [max(1, -(-len(text) // chunk))] for i in range(m)]})
        parts = self.need(got, "specter.encode")["parts"]
        order = parts[:]
        rng.shuffle(order)
        if len(order) > 1 and rng.random() < 0.2:
            order.pop()
        r = rng.random()
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

    def run(self):
        check_capacities()
        frame = "B$2P0100" + "A" * (4296 - 8)
        self.known_difference([frame], "a 4,296-character frame")
        self.specter_known()
        for _ in range(self.cases(1500)):
            self.corpus_case()
