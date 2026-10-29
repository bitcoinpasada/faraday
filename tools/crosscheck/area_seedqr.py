"""SeedQR and CompactSeedQR (`faraday_codec::seedqr`) against SeedSigner's
own encoder and decoder (`encode_qr.py` and `decode_qr.py`, vendored
unchanged in `seedsigner/`, with its BIP-39 pieces from embit in
`embit/`), and the QR codes against Project Nayuki's `qrcodegen.py`
(`qrcodegen/`), the Python library Faraday's encoder was ported from,
module for module.

What SeedSigner's code does: `SeedQrEncoder` writes every word's
wordlist index as four zero-padded decimal digits, `CompactSeedQrEncoder`
the entropy behind the words without the checksum bits; `DecodeQR`,
SeedSigner's front door for a scanned code, tells the two apart and
`SeedQrDecoder` reads them. A SeedQR is a numeric-mode QR code at level
L, a CompactSeedQR a byte-mode one; the code is the smallest version
that holds the payload, at the level asked for (never raised), with the
mask Nayuki's encoder picks.

SeedSigner reads more than Faraday does, so Faraday's own rule stays in
the area, in front of SeedSigner's reader: only 12 and 24 words are
defined, so exactly 48 or 96 digits (SeedSigner's search for a run of 48
to 96 digits takes 49 as 12 words and drops the tail, and `int()` takes a sign
or a space inside a group of four) and 16 or 32 bytes (embit takes 20,
24 and 28 too) are read. SeedSigner does not check BIP-39's checksum
when it reads digits; the area takes the entropy from embit's
`mnemonic_to_bytes`, which does, and Faraday must refuse what that
refuses. SeedSigner has the English wordlist only, and the indices of a
key are the same in every language, so the area runs it in English and
compares in indices and entropy; Faraday's language path is still what
is asked.

- SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`): the digits
  and the bytes made from the words and read back to them, and their
  codes.
- Random keys of every length and language: both codes, module for
  module, and the words read back.
- Digits and bytes read that are malformed: the wrong length, a
  character that is not a digit, an index of 2048 or more, a broken
  checksum.
- Random payloads in each mode at each level and version limit, codes
  module for module, with payloads too long and characters outside the
  mode refused.

Codex32QR/v1-48 (`docs/CODEX32.md` §10.2, the profile pinned in
`tools/vectors/codex32qr/README.md`): a 48-character codex32 string in
upper case, alphanumeric mode, version 3, level L, the mask Nayuki's
encoder picks (Faraday fixes none). Any other length is not a
Codex32QR.

- Every string of the profile's vectors and of its BIP 93 pictures, and
  random 48-character strings in either case (and strings of other
  lengths, refused): Faraday's code module for module against
  `qrcodegen.py`, and the picture Faraday writes of it on a stick
  (`codex32qr-*.png`) read back, module for module, to the same code.
- The profile's own pictures, read here from their pixels: each must be
  Faraday's code of its string module for module. Where the profile's
  encoder chose another mask than Faraday and Nayuki do, Faraday's code
  is taken with its mask swapped for the picture's by `qrcodegen.py`'s
  own masking (its mask removed, the picture's applied, the format bits
  written again), and that must be the picture: the same data and error
  correction under the picture's mask. Those pictures are counted as a
  known difference, the mask choice, which a reader ignores.
"""

import json
import struct
import zlib

from common import ROOT, Area, Mismatch, use_reference

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

use_reference("seedsigner", "shims")
use_reference("seedsigner")
use_reference("embit")
import outside_imports  # noqa: E402

outside_imports.install()
from embit import bip39 as embit_bip39  # noqa: E402
from seedsigner.models.decode_qr import DecodeQR, DecodeQRStatus  # noqa: E402
from seedsigner.models.encode_qr import CompactSeedQrEncoder, SeedQrEncoder  # noqa: E402
from seedsigner.models.qr_type import QRType  # noqa: E402

use_reference("qrcodegen")
from qrcodegen import QrCode, QrSegment  # noqa: E402

LANGUAGES = ["english", "japanese", "korean", "spanish", "chinese_simplified",
             "chinese_traditional", "french", "italian", "czech", "portuguese"]
LENGTHS = [16, 20, 24, 28, 32]
ECC = [QrCode.Ecc.LOW, QrCode.Ecc.MEDIUM, QrCode.Ecc.QUARTILE, QrCode.Ecc.HIGH]
ALNUM = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"
_MNEMONICS = {}


def mnemonic(lang):
    if lang not in _MNEMONICS:
        _MNEMONICS[lang] = Mnemonic(LANGUAGES[lang])
    return _MNEMONICS[lang]


def indices(lang, entropy):
    m = mnemonic(lang)
    return [m.wordlist.index(w) for w in m.to_mnemonic(entropy).split(m.delimiter)]


def code(segment, ecc, max_version=40):
    q = QrCode.encode_segments([segment], ECC[ecc], 1, max_version, -1, False)
    size = q.get_size()
    return {"version": q.get_version(),
            "rows": ["".join("1" if q.get_module(x, y) else "0" for x in range(size))
                     for y in range(size)]}


def encode(lang, entropy):
    """SeedSigner's two encoders over the words embit makes of the entropy
    (English: the indices, and so the digits, are the same in every
    language). `CompactSeedQrEncoder` strips the checksum bits of 12 and
    24 words only, so for the other lengths the compact form is the
    entropy, as Faraday writes it."""
    words = embit_bip39.mnemonic_from_bytes(entropy).split()
    digits = SeedQrEncoder(mnemonic=words).next_part()
    if len(words) in (12, 24):
        compact = CompactSeedQrEncoder(mnemonic=words).next_part()
    else:
        compact = bytes(entropy)
    return {"digits": digits, "compact": compact.hex(),
            "seedqr": code(QrSegment.make_numeric(digits), 0),
            "compact_qr": code(QrSegment.make_bytes(compact), 0)}


def seedsigner_reads(data, qr_type):
    """What SeedSigner's `DecodeQR` makes of a scanned payload: the words
    it names, as wordlist indices, with the entropy embit's own BIP-39
    reader gets from them (which checks the checksum)."""
    d = DecodeQR()
    status = d.add_data(data)
    if status != DecodeQRStatus.COMPLETE or d.qr_type != qr_type:
        raise ValueError(f"SeedSigner does not read it as {qr_type}: {status}, {d.qr_type}")
    words = d.get_seed_phrase()
    entropy = embit_bip39.mnemonic_to_bytes(" ".join(words))
    return {"indices": [embit_bip39.WORDLIST.index(w) for w in words], "entropy": entropy.hex()}


def read_digits(lang, digits):
    # Faraday's own rule, which SeedSigner is looser than (see above).
    if len(digits) not in (48, 96):
        raise ValueError("SeedQR is 48 or 96 digits")
    if not all(c in "0123456789" for c in digits):
        raise ValueError("a SeedQR is digits")
    return seedsigner_reads(digits, QRType.SEED__SEEDQR)


def read_compact(lang, data):
    if len(data) not in (16, 32):
        raise ValueError("CompactSeedQR is 16 or 32 bytes")
    got = seedsigner_reads(data, QRType.SEED__COMPACTSEEDQR)
    if got["entropy"] != data.hex():
        raise AssertionError("SeedSigner reads other entropy from a CompactSeedQR")
    return got


CODEX32 = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
C32_DIR = ROOT / "tools/vectors/codex32qr"


def png_luma(data):
    """A greyscale PNG (bit depth 1 or 8, not interlaced) as rows of
    luma, 0 to 255: the chunks, zlib and the five row filters of the PNG
    specification, over the standard library."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, head = 8, b"", None
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b"IHDR":
            head = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    w, h, depth, colour, _, _, interlace = head
    if colour != 0 or depth not in (1, 8) or interlace:
        raise ValueError(f"a PNG this reader does not take: {head}")
    raw = zlib.decompress(idat)
    stride = (w * depth + 7) // 8
    bpp = max(1, depth // 8)
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        f, line = raw[i], bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b, c = prev[x], prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if pa <= pb and pa <= pc else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 255
        prev = line
        if depth == 8:
            rows.append(bytes(line))
        else:
            rows.append(bytes(255 if line[x // 8] >> (7 - x % 8) & 1 else 0 for x in range(w)))
    return rows


def png_code(data):
    """The QR code drawn in a picture, as rows of `1` and `0`: the first
    dark pixel is the top-left corner of the top-left finder, whose width
    is seven modules; the last dark pixel of that row ends the top-right
    finder; every module is read at its centre."""
    rows = png_luma(data)
    y0 = next(y for y, r in enumerate(rows) if min(r) < 128)
    row = rows[y0]
    x0 = next(x for x, v in enumerate(row) if v < 128)
    run = next(x for x in range(x0, len(row)) if row[x] >= 128) - x0
    unit = run / 7
    x1 = max(x for x, v in enumerate(row) if v < 128)
    size = round((x1 - x0 + 1) / unit)
    return ["".join("1" if rows[int(y0 + (y + 0.5) * unit)][int(x0 + (x + 0.5) * unit)] < 128
                    else "0" for x in range(size)) for y in range(size)]


def codex32qr_qr(string, mask=-1):
    if len(string) != 48:
        raise ValueError("a Codex32QR is 48 characters")
    seg = QrSegment.make_alphanumeric(string.upper())
    return QrCode.encode_segments([seg], QrCode.Ecc.LOW, 1, 3, mask, False)


def rows_of(q):
    size = q.get_size()
    return ["".join("1" if q._modules[y][x] else "0" for x in range(size)) for y in range(size)]


def codex32qr_code(string, mask=-1):
    q = codex32qr_qr(string, mask)
    return {"version": q.get_version(), "rows": rows_of(q)}


def remask(string, rows, to_mask):
    """`rows`, a Codex32QR code of `string` under Nayuki's own mask
    choice, with that mask swapped for `to_mask` by qrcodegen's masking."""
    q = codex32qr_qr(string)
    size = q.get_size()
    # The function modules, which no mask touches: qrcodegen drops its
    # map once a code is made, so it is drawn again on a blank grid.
    q._modules = [[False] * size for _ in range(size)]
    q._isfunction = [[False] * size for _ in range(size)]
    q._draw_function_patterns()
    q._modules = [[c == "1" for c in r] for r in rows]
    q._apply_mask(q.get_mask())
    q._apply_mask(to_mask)
    q._draw_format_bits(to_mask)
    return rows_of(q)


def picture_read(answer):
    """Faraday's answer with the picture it wrote read back to its code."""
    if answer[0] == "ok" and isinstance(answer[1].get("png"), str):
        v = dict(answer[1])
        v["png"] = png_code(bytes.fromhex(v["png"]))
        return ("ok", v)
    return answer


def profile_pictures():
    """The profile's pictures, each with the string it holds, from its
    vectors and from the BIP 93 list beside its pictures."""
    out = []
    v = json.loads((C32_DIR / "vectors.json").read_text())
    for case in v["vectors"]:
        for k, path in case["pictures"].items():
            out.append((path, case["secret"] if k == "s" else case["shares"][k]))
    b = json.loads((C32_DIR / "test_vectors/bip93_128bit_vectors.json").read_text())
    for case in b["cases"]:
        for k, path in case["qrs"].items():
            out.append(("test_vectors/" + path, case["secret"] if k == "s" else case["shares"][k]))
    return out


class SeedQR(Area):
    name = "seedqr"
    faraday = (
        '`faraday_codec::seedqr` (SeedQR, CompactSeedQR), `faraday_codec::qr`, the '
        'encoder that draws them, and `faraday_codec::qr::codex32qr` with the picture '
        '`faraday_core::picture` writes of it'
    )
    reference = (
        "SeedSigner's own encoders and `DecodeQR` (`seedsigner/`, unchanged, with "
        "embit's BIP-39 code in `embit/`); Project Nayuki's `qrcodegen.py` "
        '(`qrcodegen/`), module for module'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; SeedSigner's reader is looser than Faraday (it takes 49 digits, a "
        'sign inside a group, 20 to 28 bytes, and does not check the checksum of '
        "digits), so Faraday's own rule, the lengths and the digits, stands in front of "
        "it in `area_seedqr.py` and the checksum is embit's; SeedSigner runs in English "
        'for every language, the indices being the same; the profile\'s own pictures '
        "are read from their pixels and must be Faraday's code module for module"
    )
    vectors = (
        "SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`); every string and "
        'picture of the Codex32QR profile (`tools/vectors/codex32qr/`)'
    )
    corpus = (
        'keys of every length and language, both codes and the words read back; digits '
        'and bytes malformed; payloads in each mode at each level and version limit; '
        '48-character codex32 strings in either case as Codex32QR codes and the '
        'pictures written of them read back, other lengths refused'
    )
    differences = [
        (
            'SeedQR digits: SeedSigner reads a string with 1 to 3 digits more than 48 or '
            '96 (it drops the tail) and one with a sign or a space inside a group of four '
            '(`int()` takes them); Faraday refuses both (5 cases, each checked on both '
            'sides and counted). SeedSigner takes no CompactSeedQR of 20, 24 or 28 bytes '
            'either, and strips checksum bits for 12 and 24 words only; the corpus keeps '
            "Faraday's 15, 18 and 21 word compact form on the entropy itself."
        ),
        (
            "Codex32QR: Faraday picks the mask Nayuki's encoder picks; the profile's "
            'encoder chose another for 6 of its 22 pictures. For those the area swaps the '
            "mask of Faraday's code for the picture's with `qrcodegen.py`'s masking, "
            'requires the picture, and counts it.'
        ),
    ]
    not_compared = (
        'none'
    )

    def __init__(self, *a):
        super().__init__(*a)
        self.known = 0

    def codex32qr(self, string):
        def ref():
            code = codex32qr_code(string)
            return {"code": code, "png": code["rows"]}
        return self.compare("seedqr.codex32qr", {"string": string, "title": "Share A of L0VE",
                                                 "line": "codex32 share"},
                            ref, normalize=picture_read)

    def codex32qr_profile(self):
        pictures = profile_pictures()
        for path, string in pictures:
            drawn = png_code((C32_DIR / path).read_bytes())
            faraday = self.need(self.codex32qr(string), "codex32qr")["code"]
            if faraday["version"] != 3:
                raise AssertionError(f"{path}: Faraday's code is not version 3")
            if faraday["rows"] == drawn:
                continue
            masks = [m for m in range(8) if codex32qr_code(string, m)["rows"] == drawn]
            if len(masks) != 1 or remask(string, faraday["rows"], masks[0]) != drawn:
                raise Mismatch(self.name, "codex32qr.picture", {"picture": path, "string": string},
                               ("ok", faraday["rows"]), ("ok", drawn),
                               "the profile's picture is not Faraday's code under any mask")
            self.known += 1
        if len(pictures) != 22:
            raise AssertionError(f"the profile has 22 pictures, not {len(pictures)}")

    def codex32qr_case(self):
        rng = self.rng
        n = 48 if rng.random() < 0.9 else rng.choice([0, 47, 49, 74, 127])
        body = "".join(rng.choice(CODEX32) for _ in range(max(0, n - 3)))
        string = ("ms1" + body)[:n]
        r = rng.random()
        if r < 0.4:
            string = string.upper()
        elif r < 0.5:
            string = "".join(c.upper() if rng.random() < 0.5 else c for c in string)
        if string and rng.random() < 0.03:
            # A character alphanumeric mode has not.
            i = rng.randrange(len(string))
            string = string[:i] + "#" + string[i + 1:]
        self.codex32qr(string)

    def encode(self, lang, entropy, expected=None):
        return self.compare("seedqr.encode", {"language": lang, "entropy": entropy.hex()},
                            lambda: encode(lang, entropy), expected)

    def digits(self, lang, digits, expected=None):
        return self.compare("seedqr.digits", {"language": lang, "digits": digits},
                            lambda: read_digits(lang, digits), expected)

    def compact(self, lang, data, expected=None):
        return self.compare("seedqr.compact", {"language": lang, "bytes": data.hex()},
                            lambda: read_compact(lang, data), expected)

    def qr(self, mode, data, ecc, max_version):
        def ref():
            if mode == "numeric":
                seg = QrSegment.make_numeric(data.decode("latin-1"))
            elif mode == "alphanumeric":
                seg = QrSegment.make_alphanumeric(data.decode("latin-1"))
            else:
                seg = QrSegment.make_bytes(data)
            return code(seg, ecc, max_version)
        return self.compare("seedqr.qr", {"mode": mode, "data": data.hex(), "ecc": ecc,
                                          "max_version": max_version}, ref)

    def published(self):
        v = json.loads((ROOT / "tools/vectors/seedqr/vectors.json").read_text())
        m = mnemonic(0)
        for case in v["vectors"]:
            entropy = bytes(m.to_entropy(case["words"].split()))
            if entropy.hex() != case["compact"]:
                raise AssertionError(f"python-mnemonic and SeedSigner differ on {case['words']}")
            ss = encode(0, entropy)
            if ss["digits"] != case["digits"] or ss["compact"] != case["compact"]:
                raise AssertionError(f"SeedSigner's own encoders differ from its vector: {case}")
            got = self.need(self.encode(0, entropy), "encode")
            if got["digits"] != case["digits"] or got["compact"] != case["compact"]:
                raise AssertionError(f"SeedSigner's vector differs: {case} {got}")
            want = ("ok", {"indices": [int(case["digits"][i:i + 4])
                                       for i in range(0, len(case["digits"]), 4)],
                           "entropy": case["compact"]})
            self.digits(0, case["digits"], want)
            self.compact(0, entropy, want)

    def corpus_case(self):
        rng = self.rng
        lang = rng.randrange(len(LANGUAGES))
        entropy = self.randbytes(rng.choice(LENGTHS))
        self.encode(lang, entropy)
        digits = "".join(f"{i:04d}" for i in indices(lang, entropy))
        self.digits(lang, digits)
        self.compact(lang, entropy)
        # Malformed: a digit changed (often a checksum failure), an
        # index past 2047, a letter, a digit too many or too few, bytes
        # of another length.
        i = rng.randrange(len(digits))
        self.digits(lang, digits[:i] + rng.choice("0123456789") + digits[i + 1:])
        w = rng.randrange(len(digits) // 4)
        self.digits(lang, digits[:4 * w] + rng.choice(["2048", "9999", str(rng.randint(2048, 9999))])
                    + digits[4 * w + 4:])
        self.digits(lang, digits[:i] + rng.choice("aZ -") + digits[i + 1:])
        self.digits(lang, digits[:-1] if rng.random() < 0.5 else digits + "0")
        self.compact(lang, self.randbytes(rng.choice([0, 15, 17, 20, 24, 31, 33])))
        # Any payload.
        mode = rng.choice(["numeric", "alphanumeric", "bytes"])
        n = rng.choice([0, 1, rng.randint(1, 60), rng.randint(1, 400), rng.randint(1000, 3000)])
        if mode == "numeric":
            data = "".join(rng.choice("0123456789") for _ in range(n)).encode()
        elif mode == "alphanumeric":
            data = "".join(rng.choice(ALNUM) for _ in range(n)).encode()
        else:
            data = self.randbytes(n)
        if mode != "bytes" and data and rng.random() < 0.05:
            j = rng.randrange(len(data))
            data = data[:j] + rng.choice([b"a", b"#", b"\x00"]) + data[j + 1:]
        self.qr(mode, data, rng.randrange(4), rng.choice([40, 40, rng.randint(1, 40)]))

    LOOPS = (("corpus", 800),)

    def seedsigner_known(self):
        """Digits SeedSigner reads and Faraday refuses: a digit more than 48
        or 96 (the tail is dropped), and a sign or a space inside a group
        (`int()` takes both). Each must still hold on both sides."""
        base = "".join(f"{i:04d}" for i in indices(0, bytes(range(16))))
        long = "".join(f"{i:04d}" for i in indices(0, bytes(range(32))))
        cases = [base + "0", base + "123", long + "9",
                 long[:48] + "+" + long[49:], long[:48] + " " + long[49:]]
        for digits in cases:
            d = DecodeQR()
            if d.add_data(digits) != DecodeQRStatus.COMPLETE:
                raise AssertionError(f"SeedSigner no longer reads {digits!r}")
            got = self.rust.call("seedqr.digits", language=0, digits=digits)
            self.compared += 1
            if got[0] != "err":
                raise AssertionError(f"Faraday now reads {digits!r}, which it refused: {got}")
            self.known += 1

    def prelude(self):
        self.published()
        self.seedsigner_known()
        self.codex32qr_profile()

    def case_corpus(self):
        self.corpus_case()
        if self.rng.random() < 0.25:
            self.codex32qr_case()
