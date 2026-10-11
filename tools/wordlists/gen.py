#!/usr/bin/env python3
"""Generate the embedded BIP-39 wordlists and Mandarin readings for osk-bip.

Downloads the ten official wordlists from the BIP repository, verifies them,
and writes one Rust module per language to core/osk-bip/src/wordlists/.
Each module contains:

  SHA256          hex SHA-256 of the source file bytes. The script checks
                  that the source file is exactly the 2048 words, UTF-8,
                  each followed by a single "\\n", so a Rust test can rebuild
                  the same bytes from WORDS and re-hash them.
  MAX_DISPLAY_CHARS  characters in the longest DISPLAY word (sizes the
                  words panel).
  MAX_NFKD_BYTES  length in bytes of the longest NFKD word (sizes the
                  fixed stack buffer used for seed derivation).
  WORDS           the 2048 words as published, for input matching and for
                  seed derivation.
  NFKD            the NFKD form of each word. When every word is already
                  NFKD this is a reference to WORDS.
  DISPLAY         the NFC form of each word, which is what a reader sees.
                  The published Japanese list is base kana plus combining
                  voicing marks and the published Korean list is conjoining
                  jamo; composed, they are the kana and the Hangul
                  syllables a person reads. When the published form is
                  already composed this is a reference to WORDS.

Words whose NFC form differs from the stored form are written with \\u{}
escapes so that editors which normalize on save cannot alter them.

It also downloads the Unicode Han Database and writes
core/osk-bip/src/wordlists/readings.rs: every Mandarin reading of every
character in the two Chinese lists, in both the spellings the two Chinese
keyboards type — Hanyu Pinyin and 注音 (bopomofo).

Usage (needs network; the Rust build does not):
    python3 tools/wordlists/gen.py
Run from the repository root. Python 3 standard library only.
"""

import hashlib
import io
import sys
import unicodedata
import urllib.request
import zipfile
from pathlib import Path

LANGUAGES = [
    "english",
    "japanese",
    "korean",
    "spanish",
    "chinese_simplified",
    "chinese_traditional",
    "french",
    "italian",
    "czech",
    "portuguese",
]
BASE_URL = "https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/"
OUT_DIR = Path("core/osk-bip/src/wordlists")

# The Unicode Han Database, from which every hanzi's Mandarin readings come.
# Pinned by digest: a reading that changed under us would silently change
# what the pinyin and 注音 keyboards find.
UNIHAN_URL = "https://www.unicode.org/Public/16.0.0/ucd/Unihan.zip"
UNIHAN_SHA256 = "b8f000df69de7828d21326a2ffea462b04bc7560022989f7cc704f10521ef3e0"


def fetch(url: str, sha256: str | None = None) -> bytes:
    raw = urllib.request.urlopen(url, timeout=180).read()
    digest = hashlib.sha256(raw).hexdigest()
    if sha256 is not None and digest != sha256:
        raise SystemExit(f"{url}: sha256 {digest}, expected {sha256}")
    return raw


def rust_str(word: str) -> str:
    """Render a word as a Rust string literal."""
    if word.isascii():
        return '"' + word.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if unicodedata.normalize("NFC", word) == word:
        return '"' + word + '"'
    return '"' + "".join(c if c.isascii() else f"\\u{{{ord(c):x}}}" for c in word) + '"'


def table(name: str, vis: str, words: list[str]) -> str:
    lines = ["#[rustfmt::skip]", f"{vis} static {name}: [&str; 2048] = [".lstrip()]
    lines += [f"    {rust_str(w)}," for w in words]
    lines.append("];")
    return "\n".join(lines)


def read_wordlist(name: str) -> tuple[list[str], str, str]:
    """The 2048 published words, the source URL and its SHA-256."""
    url = BASE_URL + name + ".txt"
    raw = fetch(url)
    text = raw.decode("utf-8")
    if not text.endswith("\n") or "\r" in text:
        raise SystemExit(f"{name}: unexpected line endings")
    words = text[:-1].split("\n")
    if len(words) != 2048:
        raise SystemExit(f"{name}: {len(words)} words, expected 2048")
    if len(set(words)) != 2048:
        raise SystemExit(f"{name}: duplicate words")
    for w in words:
        if not w or w != w.strip() or any(c.isspace() for c in w):
            raise SystemExit(f"{name}: malformed word {w!r}")
    if ("\n".join(words) + "\n").encode("utf-8") != raw:
        raise SystemExit(f"{name}: file is not words joined by newline")
    return words, url, hashlib.sha256(raw).hexdigest()


def generate(name: str, words: list[str], url: str, digest: str) -> None:
    nfkd = [unicodedata.normalize("NFKD", w) for w in words]
    if len(set(nfkd)) != 2048:
        raise SystemExit(f"{name}: NFKD forms collide")
    max_nfkd = max(len(w.encode("utf-8")) for w in nfkd)
    invariant = nfkd == words

    display = [unicodedata.normalize("NFC", w) for w in words]
    if len(set(display)) != 2048:
        raise SystemExit(f"{name}: NFC forms collide")
    composed = display == words
    max_display = max(len(w) for w in display)

    out = [
        "//! Generated by tools/wordlists/gen.py — do not edit.",
        f"//! Source: {url}",
        "",
        "/// SHA-256 (hex) of the source file: every word followed by `\\n`, UTF-8.",
        f'pub const SHA256: &str = "{digest}";',
        "",
        "/// Byte length of the longest word in [`NFKD`].",
        f"pub const MAX_NFKD_BYTES: usize = {max_nfkd};",
        "",
        "/// Characters in the longest word of [`DISPLAY`], which is what a",
        "/// panel of words is sized for.",
        f"pub const MAX_DISPLAY_CHARS: usize = {max_display};",
        "",
        "/// The words as published, for display and input matching.",
        table("WORDS", "pub", words),
        "",
        "/// NFKD form of each word, for seed derivation.",
    ]
    if invariant:
        out.append("pub static NFKD: &[&str; 2048] = &WORDS;")
    else:
        out.append("pub static NFKD: &[&str; 2048] = &NFKD_WORDS;")
        out.append("")
        out.append(table("NFKD_WORDS", "", nfkd))
    out.append("")
    out.append("/// NFC form of each word: what a reader sees on screen.")
    if composed:
        out.append("pub static DISPLAY: &[&str; 2048] = &WORDS;")
    else:
        out.append("pub static DISPLAY: &[&str; 2048] = &DISPLAY_WORDS;")
        out.append("")
        out.append(table("DISPLAY_WORDS", "", display))
    out.append("")
    (OUT_DIR / f"{name}.rs").write_text("\n".join(out), encoding="utf-8")
    print(
        f"{name:20} sha256={digest} max_nfkd_bytes={max_nfkd} "
        f"nfkd_invariant={invariant} nfc_invariant={composed}"
    )


# --- Mandarin readings -----------------------------------------------------
#
# Pinyin is written toneless here, with `lv` for `lü`, because that is what
# a keyboard types: the tone is a separate key. 注音 spells the same
# syllable as an optional initial, an optional medial and a final.

INITIALS = [
    ("zh", "ㄓ"),
    ("ch", "ㄔ"),
    ("sh", "ㄕ"),
    ("b", "ㄅ"),
    ("p", "ㄆ"),
    ("m", "ㄇ"),
    ("f", "ㄈ"),
    ("d", "ㄉ"),
    ("t", "ㄊ"),
    ("n", "ㄋ"),
    ("l", "ㄌ"),
    ("g", "ㄍ"),
    ("k", "ㄎ"),
    ("h", "ㄏ"),
    ("j", "ㄐ"),
    ("q", "ㄑ"),
    ("x", "ㄒ"),
    ("r", "ㄖ"),
    ("z", "ㄗ"),
    ("c", "ㄘ"),
    ("s", "ㄙ"),
]
INITIAL = dict(INITIALS)

# Finals in their full pinyin spelling: `iou`, `uei` and `uen` rather than
# the contracted `iu`, `ui` and `un`, and `v` for `ü`. The empty final is
# the one `zhi chi shi ri zi ci si` write as `i` and 注音 does not write.
FINALS = {
    "": "",
    "a": "ㄚ",
    "o": "ㄛ",
    "e": "ㄜ",
    "ai": "ㄞ",
    "ei": "ㄟ",
    "ao": "ㄠ",
    "ou": "ㄡ",
    "an": "ㄢ",
    "en": "ㄣ",
    "ang": "ㄤ",
    "eng": "ㄥ",
    "er": "ㄦ",
    "i": "ㄧ",
    "ia": "ㄧㄚ",
    "ie": "ㄧㄝ",
    "io": "ㄧㄛ",  # yo, the interjection 喲 alone
    "iao": "ㄧㄠ",
    "iou": "ㄧㄡ",
    "ian": "ㄧㄢ",
    "in": "ㄧㄣ",
    "iang": "ㄧㄤ",
    "ing": "ㄧㄥ",
    "iong": "ㄩㄥ",
    "u": "ㄨ",
    "ua": "ㄨㄚ",
    "uo": "ㄨㄛ",
    "uai": "ㄨㄞ",
    "uei": "ㄨㄟ",
    "uan": "ㄨㄢ",
    "uen": "ㄨㄣ",
    "uang": "ㄨㄤ",
    "ueng": "ㄨㄥ",
    "v": "ㄩ",
    "ve": "ㄩㄝ",
    "van": "ㄩㄢ",
    "vn": "ㄩㄣ",
}
# The seven syllables whose written `i` is no vowel: 注音 writes the
# initial alone.
EMPTY_RHYME = {"zhi", "chi", "shi", "ri", "zi", "ci", "si"}
# Contractions pinyin writes after an initial, and `ong`, which is the
# same final as the syllable-initial `weng`.
CONTRACTED = {"iu": "iou", "ui": "uei", "un": "uen", "ong": "ueng"}

TONE_MARKS = {0x304: 1, 0x301: 2, 0x30C: 3, 0x300: 4}


def toneless(reading: str) -> tuple[str, int]:
    """A Unihan reading split into its toneless spelling and its tone.

    Tones are 1 to 4 as the diacritic marks them and 5 for the neutral
    tone, which carries no mark; `ü` is written `v`.
    """
    tone = 5
    out: list[str] = []
    for c in unicodedata.normalize("NFD", reading):
        if ord(c) in TONE_MARKS:
            tone = TONE_MARKS[ord(c)]
        elif ord(c) == 0x308:  # the diaeresis of ü, on the vowel before it
            out[-1] = "v"
        else:
            out.append(c)
    return unicodedata.normalize("NFC", "".join(out)), tone


def split_syllable(p: str) -> tuple[str, str]:
    """A toneless pinyin syllable as (initial, final) in the tables above."""
    if p in EMPTY_RHYME:
        return p[:-1], ""
    if p == "er":
        return "", "er"
    if p.startswith("y"):
        rest = p[1:]
        if rest.startswith("u"):  # yu, yuan, yue, yun
            return "", "v" + rest[1:]
        if rest.startswith("i"):  # yi, yin, ying
            return "", rest
        if rest == "ong":
            return "", "iong"
        return "", "i" + rest
    if p.startswith("w"):
        rest = p[1:]
        return "", rest if rest.startswith("u") else "u" + rest
    for initial, _ in INITIALS:
        if p.startswith(initial):
            rest = p[len(initial) :]
            if initial in ("j", "q", "x") and rest.startswith("u"):
                rest = "v" + rest[1:]  # ju, juan, jue, jun are ㄩ
            if rest == "ue":
                rest = "ve"  # lüe, nüe
            return initial, CONTRACTED.get(rest, rest)
    return "", CONTRACTED.get(p, p)


def zhuyin_of(p: str) -> str:
    initial, final = split_syllable(p)
    if final not in FINALS or (initial and initial not in INITIAL):
        raise SystemExit(f"no 注音 for pinyin {p!r} ({initial!r} + {final!r})")
    return INITIAL.get(initial, "") + FINALS[final]


REV_INITIAL = {z: p for p, z in INITIALS}
REV_FINAL = {z: p for p, z in FINALS.items() if p != "ueng"}
REV_FINAL["ㄨㄥ"] = "ueng"


def pinyin_of(z: str) -> str:
    """The inverse of `zhuyin_of`, so that every syllable can be checked."""
    initial = ""
    if z and z[0] in REV_INITIAL:
        initial, z = REV_INITIAL[z[0]], z[1:]
    final = REV_FINAL[z]
    if final == "":
        return initial + "i"
    if initial == "":
        if final == "er":
            return "er"
        if final[0] == "i":
            return "y" + (final if final in ("i", "in", "ing") else final[1:])
        if final[0] == "u":
            return "w" + (final if final == "u" else final[1:])
        if final[0] == "v":
            return "yu" + final[1:]
        return final
    if final[0] == "v" and initial in ("j", "q", "x"):
        final = "u" + final[1:]
    for full, short in (("iou", "iu"), ("uei", "ui"), ("uen", "un"), ("ueng", "ong")):
        if final == full:
            final = short
    return initial + final


def unihan_readings(chars: list[str]) -> dict[str, list[tuple[str, int]]]:
    """The modern-dictionary readings of each character, head reading first.

    `kMandarin` gives the reading a modern dictionary heads the entry with;
    `kXHC1983` (Xiandai Hanyu Cidian) and `kTGHZ2013` (Tongyong Guifan
    Hanzi Zidian) give every reading those two dictionaries list, which is
    the polyphones a reader knows (行 xíng and háng, 長 zhǎng and cháng)
    without the historical readings the Hanyu Da Zidian records (王 yù,
    服 bì), which nobody types (the owner, 2026-09-10). `kMandarin` is
    first so a keyboard offers the expected character first.
    """
    want = set(chars)
    raw = zipfile.ZipFile(io.BytesIO(fetch(UNIHAN_URL, UNIHAN_SHA256)))
    fields: dict[str, dict[str, str]] = {}
    for line in raw.read("Unihan_Readings.txt").decode("utf-8").splitlines():
        if not line.startswith("U+"):
            continue
        code, field, value = line.split("\t", 2)
        if field not in ("kMandarin", "kXHC1983", "kTGHZ2013"):
            continue
        c = chr(int(code[2:], 16))
        if c in want:
            fields.setdefault(c, {})[field] = value

    out: dict[str, list[tuple[str, int]]] = {}
    for c in chars:
        found = fields.get(c, {})
        spellings: list[str] = found.get("kMandarin", "").split()
        for field in ("kXHC1983", "kTGHZ2013"):
            for entry in found.get(field, "").split():
                # `0026.011*,0027.010:xíng`: locations, then one reading.
                spellings.append(entry.split(":")[1])
        readings: list[tuple[str, int]] = []
        for spelling in spellings:
            reading = toneless(spelling)
            if reading not in readings:
                readings.append(reading)
        if not readings:
            raise SystemExit(f"U+{ord(c):04X} {c} has no Mandarin reading")
        out[c] = readings
    return out


def generate_readings(chars: list[str]) -> tuple[int, int]:
    readings = unihan_readings(chars)
    syllables = sorted({p for rs in readings.values() for p, _ in rs})
    zhuyin = {}
    for p in syllables:
        z = zhuyin_of(p)
        back = pinyin_of(z).replace("ü", "v")
        if back != p:
            raise SystemExit(f"{p} → {z} → {back} does not round-trip")
        if z in zhuyin:
            raise SystemExit(f"{p} and {zhuyin[z]} both spell {z}")
        zhuyin[z] = p
    index = {p: i for i, p in enumerate(syllables)}

    lines = [
        "//! Generated by tools/wordlists/gen.py — do not edit.",
        f"//! Source: {UNIHAN_URL} (kMandarin, kXHC1983, kTGHZ2013)",
        "//!",
        "//! Every Mandarin reading of every character in the two Chinese",
        "//! wordlists, in the two spellings their keyboards type.",
        "",
        "/// One Mandarin syllable without its tone, in both spellings.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct Syllable {",
        "    /// Toneless Hanyu Pinyin, with `lv` for `lü`.",
        "    pub pinyin: &'static str,",
        "    /// The same syllable in 注音 (bopomofo).",
        "    pub zhuyin: &'static str,",
        "}",
        "",
        "/// Every syllable the two Chinese lists are read with, sorted by",
        "/// [`pinyin`](Syllable::pinyin).",
        "#[rustfmt::skip]",
        f"pub static SYLLABLES: &[Syllable; {len(syllables)}] = &[",
    ]
    for p in syllables:
        lines.append(
            f'    Syllable {{ pinyin: "{p}", zhuyin: "{zhuyin_of(p)}" }},'
        )
    lines += [
        "];",
        "",
        "/// Each character and its readings, sorted by character: an index",
        "/// into [`SYLLABLES`] and a tone, 1 to 4 as it is marked and 5 for",
        "/// the neutral tone. The first reading is the one a dictionary",
        "/// heads the entry with.",
        "#[rustfmt::skip]",
        f"pub static READINGS: &[(char, &[(u16, u8)]); {len(chars)}] = &[",
    ]
    polyphones = 0
    for c in sorted(chars):
        rs = readings[c]
        if len(rs) > 1:
            polyphones += 1
        body = ", ".join(f"({index[p]}, {t})" for p, t in rs)
        lines.append(f"    ('\\u{{{ord(c):x}}}', &[{body}]),")
    lines += ["];", ""]
    (OUT_DIR / "readings.rs").write_text("\n".join(lines), encoding="utf-8")
    print(
        f"{'readings':20} {len(chars)} characters, {len(syllables)} syllables, "
        f"{polyphones} polyphones"
    )
    return len(syllables), polyphones


def main() -> None:
    if not OUT_DIR.is_dir():
        raise SystemExit(f"{OUT_DIR} not found; run from the repository root")
    lists = {}
    for name in LANGUAGES:
        words, url, digest = read_wordlist(name)
        lists[name] = words
        generate(name, words, url, digest)
    hanzi = sorted(
        set("".join(lists["chinese_simplified"]))
        | set("".join(lists["chinese_traditional"]))
    )
    generate_readings(hanzi)


if __name__ == "__main__":
    sys.exit(main())
