#!/usr/bin/env python3
"""Finds and removes the metadata an image carries (lint-tree's check 4).

    image-metadata.py --check IMAGE...
    image-metadata.py --strip IMAGE...

Metadata is a PNG's text, EXIF, ICC and time chunks, a JPEG's EXIF, XMP,
ICC, APP13 and comment segments, a WebP's EXIF, XMP and ICC chunks, and an
SVG's metadata element, comments, stylesheet instruction and editor
elements and attributes. `--check` lists each image that carries any and
fails; `--strip` rewrites each one without it. The standard library only.
"""

import pathlib
import re
import struct
import sys

# Chunks that carry text, camera or colour-profile data, or a time.
FORBIDDEN = {b"tEXt", b"zTXt", b"iTXt", b"eXIf", b"iCCP", b"tIME"}
JPEG_META = {0xE1, 0xE2, 0xED, 0xFE}  # APP1 EXIF/XMP, APP2 ICC, APP13, COM
WEBP_META = {b"EXIF", b"XMP ", b"ICCP"}
SVG_META = re.compile(
    r"<metadata\b.*?</metadata>|<!--.*?-->|<\?xml-stylesheet.*?\?>"
    r"|<(?:sodipodi|inkscape|rdf|cc|dc):[^>]*>|\s(?:sodipodi|inkscape):[\w-]+=\"[^\"]*\"",
    re.S)


def walk(path):
    """The file as (kind, bytes) parts, each marked as metadata or not."""
    data = path.read_bytes()
    ext = path.suffix.lower()
    if ext == ".png":
        if data[:8] != b"\x89PNG\r\n\x1a\n":
            raise ValueError("not a PNG")
        at, parts = 8, [(False, "", data[:8])]
        while at < len(data):
            (n,) = struct.unpack(">I", data[at : at + 4])
            kind = data[at + 4 : at + 8]
            parts.append((kind in FORBIDDEN, kind.decode("latin1"), data[at : at + 12 + n]))
            at += 12 + n
        return parts
    if ext in (".jpg", ".jpeg"):
        if data[:2] != b"\xff\xd8":
            raise ValueError("not a JPEG")
        at, parts = 2, [(False, "", data[:2])]
        while at + 4 <= len(data) and data[at] == 0xFF:
            marker = data[at + 1]
            if marker == 0xDA:  # the scan: the rest is image data
                break
            (n,) = struct.unpack(">H", data[at + 2 : at + 4])
            parts.append((marker in JPEG_META, f"segment {marker:02X}", data[at : at + 2 + n]))
            at += 2 + n
        parts.append((False, "", data[at:]))
        return parts
    if ext == ".webp":
        if data[:4] != b"RIFF" or data[8:12] != b"WEBP":
            raise ValueError("not a WebP")
        at, parts = 12, [(False, "", data[:12])]
        while at + 8 <= len(data):
            kind = data[at : at + 4]
            (n,) = struct.unpack("<I", data[at + 4 : at + 8])
            parts.append((kind in WEBP_META, kind.decode("latin1").strip(), data[at : at + 8 + n + n % 2]))
            at += 8 + n + n % 2
        return parts
    if ext == ".svg":
        text = data.decode("utf-8")
        parts, at = [], 0
        for m in SVG_META.finditer(text):
            parts.append((False, "", text[at : m.start()].encode()))
            parts.append((True, m.group(0)[:24].strip(), b""))
            at = m.end()
        parts.append((False, "", text[at:].encode()))
        return parts
    return []


def metadata(path):
    """The names of the metadata `path` carries. Reads the file."""
    return [name for meta, name, _ in walk(path) if meta]


def strip(path):
    """Rewrites `path` without its metadata. Reads and writes the file."""
    parts = walk(path)
    out = b"".join(b for meta, _, b in parts if not meta)
    if path.suffix.lower() == ".webp":
        out = out[:4] + struct.pack("<I", len(out) - 8) + out[8:]
    path.write_bytes(out)


def main(argv):
    if len(argv) < 2 or argv[1] not in ("--check", "--strip"):
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    bad = 0
    for name in argv[2:]:
        path = pathlib.Path(name)
        found = metadata(path)
        if not found:
            continue
        if argv[1] == "--strip":
            strip(path)
            print(f"stripped {name}: {', '.join(found)}")
        else:
            print(f"{name}: carries {', '.join(found)}")
            bad += 1
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
