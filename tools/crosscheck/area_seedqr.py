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

segno, an encoder of another lineage (not a port of Nayuki's), judges every
code the area compares with Nayuki's: SeedQR and CompactSeedQR, the payloads
in each mode, and the Codex32QR strings, all with the version, level and mode
Faraday's code has. With its mask forced to the one Faraday's code is under,
segno's code is Faraday's module for module and reports that mask; with another
mask forced, it is Nayuki's code under that mask; at its own mask choice it is
Faraday's code, or the same data and error correction under the other mask it
reports (Nayuki's masking swapped to it), counted as a known difference, the
mask choice. One more difference is named: segno 1.6 adds eight zero bits
where ISO/IEC 18004 7.4.10 adds none when the terminated data ends on a codeword
boundary with room left, so a zero codeword stands where the pad codewords begin
(`write_padding_bits`); for those payloads (about one in eight) the area compares
segno with Nayuki's code made from the same codewords, and counts it. A reader
stops at the terminator and does not see it. rqrr, the app's reader, reads Faraday's code and segno's own back to
the payload (`seedqr.read`, `decode_luma` over the code drawn at four pixels a
module with a quiet zone). segno takes no empty payload, which is not compared.

Every PNG Faraday writes in the area (the Codex32QR pictures, a greyscale
picture of random size, a code's picture at a scale) is read by two outside
readers, Pillow and CPython's zlib: each chunk's CRC-32 is zlib's, the zlib
stream's Adler-32 is `zlib.adler32` of the rows, and Pillow reads the same
pixels as this area's own reader and as the reference's.

Two outside decoders, zbar and zxing-cpp (`qr_decode.py`), read the QR
code in each picture Faraday writes on a stick, and the payload each
returns, as bytes, must be the one the picture holds: the Codex32QR pictures
(the string in upper case), the profile's own 22 pictures, and the labelled
pictures of a key's SeedQR (its digits) and CompactSeedQR (its entropy
bytes). Faraday's reader is then given the decoded digits and bytes and must
give the key's words. A picture either cannot read, or reads as other bytes,
fails the run naming the picture.

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
import qr_decode

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

import segno  # noqa: E402

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


def png_read_outside(data):
    """A picture Faraday wrote, read by two readers that are not its
    own and checked against this area's own: Pillow (the decoder checks
    every chunk's CRC-32 and the zlib stream's) and CPython's zlib
    (`decompress` refuses a wrong Adler-32; the trailer is also compared
    with `zlib.adler32` of the rows, and every chunk's CRC-32 with
    `zlib.crc32`). Returns the rows of luma, which must be the ones
    `png_luma` reads."""
    pos, idat = 8, b""
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        crc = struct.unpack(">I", data[pos + 8 + n:pos + 12 + n])[0]
        if zlib.crc32(kind + body) != crc:
            raise AssertionError(f"a {kind!r} chunk's CRC-32 is not zlib's")
        if kind == b"IDAT":
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    if idat[-4:] != struct.pack(">I", zlib.adler32(raw)):
        raise AssertionError("the zlib stream's Adler-32 is not zlib's")
    pixels, (w, h) = qr_decode.pixels(data)
    rows = [pixels[y * w:(y + 1) * w] for y in range(h)]
    if rows != png_luma(data):
        raise AssertionError("Pillow reads other pixels than this area's reader")
    return rows


def png_code(data):
    """The QR code drawn in a picture, as rows of `1` and `0`: the first
    dark pixel is the top-left corner of the top-left finder, whose width
    is seven modules; the last dark pixel of that row ends the top-right
    finder; every module is read at its centre."""
    rows = png_read_outside(data)
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


SEGNO_MODES = {"numeric": "numeric", "alphanumeric": "alphanumeric", "bytes": "byte"}


def segment(mode, data):
    if mode == "numeric":
        return QrSegment.make_numeric(data.decode("latin-1"))
    if mode == "alphanumeric":
        return QrSegment.make_alphanumeric(data.decode("latin-1"))
    return QrSegment.make_bytes(data)


def segno_pads_a_zero(mode, data, ecc, version):
    """Whether segno 1.6 writes this payload's data codewords other than
    ISO/IEC 18004 7.4.10 does, and where: the terminated bit stream ends on
    a codeword boundary with room left, and segno's `write_padding_bits`
    adds 8 zero bits where none are due, so a zero codeword stands where the
    pad codewords begin and they follow it, one fewer. Returns the number of
    codewords before the zero one, or None."""
    bits = QrSegment.get_total_bits([segment(mode, data)], version)
    cap = QrCode._get_num_data_codewords(version, ECC[ecc]) * 8
    stream = bits + min(4, cap - bits)
    return stream // 8 if stream % 8 == 0 and stream < cap else None


def nayuki_code(mode, data, ecc, max_version, mask=-1, zero_at=None):
    """Nayuki's code of a payload, the object (its mask is known); with
    `zero_at`, the data codewords as segno writes them (see
    `segno_pads_a_zero`)."""
    seg = segment(mode, data)
    if zero_at is None:
        return QrCode.encode_segments([seg], ECC[ecc], 1, max_version, mask, False)
    original = QrCode._add_ecc_and_interleave

    def quirk(self, codewords):
        words = list(codewords)
        return original(self, bytearray(words[:zero_at] + [0] + words[zero_at:-1]))
    QrCode._add_ecc_and_interleave = quirk
    try:
        return QrCode.encode_segments([seg], ECC[ecc], 1, max_version, mask, False)
    finally:
        QrCode._add_ecc_and_interleave = original


def segno_make(mode, data, ecc, version, mask, fault=None):
    """segno's code of a payload at this version, level and mode, with
    its mask forced (or its own choice when `mask` is None): its rows and
    the mask it reports. `fault` makes it wrong, for the self-test: one
    module flipped, or the mask reported another."""
    content = data if mode == "bytes" else data.decode("latin-1")
    q = segno.make(content, error="LMQH"[ecc], version=version, mode=SEGNO_MODES[mode],
                   mask=mask, boost_error=False, micro=False)
    rows = ["".join("1" if v else "0" for v in r) for r in q.matrix]
    reported = q.mask
    if fault == "module":
        last = rows[-1]
        rows[-1] = last[:-1] + ("0" if last[-1] == "1" else "1")
    elif fault == "mask":
        reported = (reported + 1) % 8
    return rows, reported


def luma_of(rows, scale=4, quiet=4):
    """The picture of a code as 8-bit luma, dark modules 0, with a quiet
    zone: (width, height, bytes)."""
    size = len(rows)
    side = (size + 2 * quiet) * scale
    blank = b"\xff" * side
    pad = b"\xff" * (quiet * scale)
    lines = [blank] * (quiet * scale)
    for r in rows:
        line = pad + b"".join((b"\x00" if c == "1" else b"\xff") * scale for c in r) + pad
        lines += [line] * scale
    lines += [blank] * (quiet * scale)
    return side, side, b"".join(lines)


def nayuki_remask(q, to_mask):
    """`q`, a code under Nayuki's own mask choice, with that mask swapped
    for `to_mask` by qrcodegen's masking: its rows."""
    rows = rows_of(q)
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


def codex32qr_code(string, mask=-1):
    q = codex32qr_qr(string, mask)
    return {"version": q.get_version(), "rows": rows_of(q)}


def remask(string, rows, to_mask):
    """`rows`, a Codex32QR code of `string` under Nayuki's own mask
    choice, with that mask swapped for `to_mask` by qrcodegen's masking."""
    return nayuki_remask(codex32qr_qr(string), to_mask)


def picture_read(answer):
    """Faraday's answer with the picture it wrote read back to its code."""
    if answer[0] == "ok" and isinstance(answer[1].get("png"), str):
        v = dict(answer[1])
        v["png"] = png_code(bytes.fromhex(v["png"]))
        return ("ok", v)
    return answer


def png_rows(answer):
    """Faraday's answer with the picture it wrote read to its rows of
    luma, as hex."""
    if answer[0] == "ok" and isinstance(answer[1].get("png"), str):
        rows = png_read_outside(bytes.fromhex(answer[1]["png"]))
        return ("ok", {"png": [r.hex() for r in rows]})
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
        "(`qrcodegen/`), module for module; segno 1.6 (`python3-segno`), an encoder of "
        "another lineage, at Faraday's mask and at its own; Pillow and CPython's zlib, "
        "which read every PNG Faraday writes; zbar (`zbarimg`) and zxing-cpp, which "
        'decode the QR code in every picture and return its payload as bytes '
        '(`qr_decode.py`)'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; SeedSigner's reader is looser than Faraday (it takes 49 digits, a "
        'sign inside a group, 20 to 28 bytes, and does not check the checksum of '
        "digits), so Faraday's own rule, the lengths and the digits, stands in front of "
        "it in `area_seedqr.py` and the checksum is embit's; SeedSigner runs in English "
        'for every language, the indices being the same; the profile\'s own pictures '
        "are read from their pixels and must be Faraday's code module for module; "
        "segno is asked for Faraday's version, level and mode and its mask forced, "
        "and its mask is read back from it, its padding quirk being modelled in Nayuki's "
        "code; rqrr reads Faraday's and segno's codes; the self-test makes each "
        "decoder's answer wrong (a payload's last byte changed) and must catch it"
    )
    vectors = (
        "SeedSigner's nine test SeedQRs (`tools/vectors/seedqr/`); every string and "
        'picture of the Codex32QR profile (`tools/vectors/codex32qr/`), each decoded by zbar '
        'and zxing-cpp'
    )
    corpus = (
        'keys of every length and language, both codes and the words read back; digits '
        'and bytes malformed; payloads in each mode at each level and version limit; '
        '48-character codex32 strings in either case as Codex32QR codes and the '
        'pictures written of them read back and decoded by zbar and zxing-cpp, other lengths '
        'refused; the pictures of a key\'s SeedQR and CompactSeedQR decoded by both and '
        'read by Faraday to the key\'s words; greyscale pictures '
        "and QR pictures of random size and scale read by Pillow and zlib"
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
        (
            "segno 1.6 writes a zero codeword where the pad codewords begin when the data, "
            "terminator included, ends on a codeword boundary with room left (its "
            "`write_padding_bits` adds 8 zero bits where none are due; ISO/IEC 18004 7.4.10 "
            "adds none): its code is Nayuki's with that codeword changed, which rqrr reads, "
            "counted as a known difference, the padding; Faraday's is the standard's."
        ),
        (
            "segno picks a mask by its own penalty rule, and for some payloads another than "
            "Nayuki's and Faraday's; the area requires that code to be Faraday's under the "
            "mask segno reports (Nayuki's masking swapped to it) and rqrr to read it, and "
            "counts each as a known difference, the mask choice, which a reader ignores."
        ),
    ]
    not_compared = (
        "an empty payload, which segno refuses"
    )

    def __init__(self, exe, seed, scale, sabotage=None):
        # The self-test makes segno wrong on its own: `seedqr.segno-module`
        # flips a module of its code, `seedqr.segno-mask` has it report
        # another mask than it used.
        self.segno_fault = None
        if sabotage in ("seedqr.segno-module", "seedqr.segno-mask"):
            self.segno_fault = sabotage.split("-")[1]
            sabotage = None
        super().__init__(exe, seed, scale, sabotage)
        self.sabotage = sabotage
        self.known = 0

    def segno_check(self, mode, data, ecc, max_version, faraday_rows):
        """segno's code of the payload beside Faraday's (`faraday_rows`,
        which equals Nayuki's): at the mask Faraday's code is under, module
        for module and with segno reporting that mask; at another mask,
        against Nayuki's code under it; and at segno's own choice, the same
        code, or the same data and error correction under the mask segno
        reports (Nayuki's masking swapped to it: a known difference, the
        mask choice). Then rqrr, through Faraday's reader, reads Faraday's
        code and segno's own back to the payload. segno takes no empty
        payload."""
        if not data:
            return
        q = nayuki_code(mode, data, ecc, max_version)
        version, mask = q.get_version(), q.get_mask()
        zero_at = segno_pads_a_zero(mode, data, ecc, version)
        inputs = {"mode": mode, "data": data.hex(), "ecc": ecc, "version": version}
        fault = self.segno_fault

        def fail(op, got, want, note):
            raise Mismatch(self.name, op, dict(inputs, mask=mask), ("ok", got), ("ok", want), note)

        if rows_of(q) != faraday_rows:
            fail("seedqr.segno-module", rows_of(q), faraday_rows,
                 "Nayuki's code is not Faraday's")
        # Where segno writes a zero codeword in front of the pad codewords
        # its code is Nayuki's with the same codewords (a known difference,
        # counted); everywhere else it is Faraday's.
        base = faraday_rows
        if zero_at is not None:
            self.known += 1
            base = rows_of(nayuki_code(mode, data, ecc, max_version, mask, zero_at))
        rows, reported = segno_make(mode, data, ecc, version, mask, fault)
        self.compared += 1
        if rows != base:
            fail("seedqr.segno-module", rows, base,
                 "segno's code at Faraday's mask is not Faraday's"
                 + (" with segno's padding" if zero_at is not None else ""))
        if reported != mask:
            fail("seedqr.segno-mask", reported, mask, "segno reports another mask than it was given")
        other = (mask + 1 + len(data) % 7) % 8
        rows, reported = segno_make(mode, data, ecc, version, other, fault)
        self.compared += 1
        want = rows_of(nayuki_code(mode, data, ecc, max_version, other, zero_at))
        if rows != want:
            fail("seedqr.segno-module", rows, want, f"segno's code at mask {other} is not Nayuki's")
        if reported != other:
            fail("seedqr.segno-mask", reported, other, "segno reports another mask than it was given")
        own, reported = segno_make(mode, data, ecc, version, None, fault)
        self.compared += 1
        if own != base:
            if reported == mask:
                fail("seedqr.segno-module", own, base,
                     "segno's own code has Faraday's mask and is not Faraday's code")
            want = nayuki_remask(nayuki_code(mode, data, ecc, max_version, -1, zero_at), reported)
            if own != want:
                fail("seedqr.segno-module", own, want,
                     f"segno's own code is not Faraday's under its mask {reported}")
            self.known += 1
        elif reported != mask:
            fail("seedqr.segno-mask", reported, mask, "segno's code is Faraday's under another mask")
        for who, picture in (("Faraday's code", faraday_rows), ("segno's own code", own)):
            w, h, luma = luma_of(picture)
            got = self.rust.call("seedqr.read", width=w, height=h, luma=luma.hex())
            self.compared += 1
            if got != ("ok", {"payloads": [data.hex()]}):
                fail("seedqr.qr-read", got, {"payloads": [data.hex()]},
                     f"rqrr does not read {who} back to the payload")

    def codex32qr(self, string):
        def ref():
            code = codex32qr_code(string)
            return {"code": code, "png": code["rows"]}
        raw = self.compare("seedqr.codex32qr", {"string": string, "title": "Share A of L0VE",
                                                "line": "codex32 share"},
                           ref, normalize=picture_read)
        if raw[0] == "ok":
            self.segno_check("alphanumeric", string.upper().encode(), 0, 3, raw[1]["code"]["rows"])
            self.decoded(f"codex32qr {string}", bytes.fromhex(raw[1]["png"]), string.upper().encode())
        return raw

    def decoded(self, picture, png, payload):
        """zbar and zxing-cpp read the picture Faraday wrote as its payload."""
        qr_decode.check(self.name, picture, png, payload, self.sabotage)
        self.compared += 2

    def key_pictures(self, lang, entropy, digits):
        """The pictures Faraday writes of a key's SeedQR and CompactSeedQR
        on a stick, decoded by both readers: the digits and the entropy
        bytes, which Faraday reads back (as far as it reads them) to the
        key's words."""
        got = self.rust.call("seedqr.pictures", language=lang, entropy=entropy.hex())
        self.compared += 1
        if got[0] != "ok":
            return
        self.decoded(f"SeedQR of {entropy.hex()}", bytes.fromhex(got[1]["seedqr"]), digits.encode())
        self.decoded(f"CompactSeedQR of {entropy.hex()}", bytes.fromhex(got[1]["compact_qr"]), entropy)
        want = ("ok", {"indices": [int(digits[i:i + 4]) for i in range(0, len(digits), 4)],
                       "entropy": entropy.hex()})
        if len(digits) in (48, 96):
            self.digits(lang, digits, want)
        if len(entropy) in (16, 32):
            self.compact(lang, entropy, want)

    def png_grey(self):
        rng = self.rng
        w, h = rng.randint(1, 300), rng.randint(1, 120)
        n = rng.choice([w * h, w * h, rng.randint(0, w * h), w * h + rng.randint(1, 50)])
        pixels = self.randbytes(n)
        # A picture long enough to run past one stored deflate block.
        if rng.random() < 0.05:
            w, h = 301, 257
            pixels = self.randbytes(w * h)
        full = (pixels[:w * h] + b"\xff" * (w * h))[:w * h]
        self.compare("seedqr.png", {"kind": "grey", "width": w, "height": h,
                                    "pixels": pixels.hex()},
                     lambda: {"png": [full[y * w:(y + 1) * w].hex() for y in range(h)]},
                     normalize=png_rows)

    def png_qr(self):
        rng = self.rng
        data = "".join(rng.choice("0123456789") for _ in range(rng.randint(1, 200))).encode()
        ecc, scale = rng.randrange(4), rng.randint(1, 8)

        def ref():
            rows = code(QrSegment.make_numeric(data.decode()), ecc)["rows"]
            size = len(rows)
            row = lambda r: b"".join((b"\x00" if c == "1" else b"\xff") * scale for c in r)
            blank = b"\xff" * ((size + 8) * scale)
            lines = [blank] * (4 * scale)
            for r in rows:
                lines += [b"\xff" * (4 * scale) + row(r) + b"\xff" * (4 * scale)] * scale
            lines += [blank] * (4 * scale)
            return {"png": [x.hex() for x in lines]}
        self.compare("seedqr.png", {"kind": "qr", "data": data.hex(), "ecc": ecc,
                                    "scale": scale}, ref, normalize=png_rows)

    def codex32qr_profile(self):
        pictures = profile_pictures()
        for path, string in pictures:
            picture = (C32_DIR / path).read_bytes()
            self.decoded(f"{path}", picture, string.upper().encode())
            drawn = png_code(picture)
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
        raw = self.compare("seedqr.encode", {"language": lang, "entropy": entropy.hex()},
                           lambda: encode(lang, entropy), expected)
        if raw[0] == "ok":
            self.segno_check("numeric", raw[1]["digits"].encode(), 0, 40, raw[1]["seedqr"]["rows"])
            self.segno_check("bytes", bytes.fromhex(raw[1]["compact"]), 0, 40,
                             raw[1]["compact_qr"]["rows"])
        return raw

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
        raw = self.compare("seedqr.qr", {"mode": mode, "data": data.hex(), "ecc": ecc,
                                         "max_version": max_version}, ref)
        if raw[0] == "ok":
            self.segno_check(mode, data, ecc, max_version, raw[1]["rows"])
        return raw

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
            self.key_pictures(0, entropy, case["digits"])

    def corpus_case(self):
        rng = self.rng
        lang = rng.randrange(len(LANGUAGES))
        entropy = self.randbytes(rng.choice(LENGTHS))
        self.encode(lang, entropy)
        digits = "".join(f"{i:04d}" for i in indices(lang, entropy))
        self.digits(lang, digits)
        self.compact(lang, entropy)
        if self.rng.random() < 0.25:
            self.key_pictures(lang, entropy, digits)
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
        if self.rng.random() < 0.25:
            self.png_grey()
            self.png_qr()
