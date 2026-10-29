#!/usr/bin/env python3
"""Collects the guide's pictures from the tours' renders (`just guide-pictures`).

    guide-pictures.py OUT RENDER_DIR...
    guide-pictures.py --check IMAGE...
    guide-pictures.py --strip IMAGE...

The pictures the guide shows are the PNGs its pages link
(`docs/GUIDE.md`, `docs/guide/**/*.md`) and the ones a page names as still
to come ("*Picture to come: `<name>.png`, ...*"). Each render directory
holds shots named as their pictures, with `<name>.mark.json` beside a
picture whose next control is marked. Every picture shown is copied into
OUT with its mark; a picture in OUT that no page shows is removed. A shown
picture no tour drew is listed, as is one not at 1280 x 800 and a mark
asked for where the frame offered no such control. A picture
two render directories both drew, or one carrying text, EXIF, ICC or time
chunks, fails the run. The standard library only.
"""

import pathlib
import re
import shutil
import struct
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SIZE = (1280, 800)
# Chunks that carry text, camera or colour-profile data, or a time.
FORBIDDEN = {b"tEXt", b"zTXt", b"iTXt", b"eXIf", b"iCCP", b"tIME"}
LINK = re.compile(r"!\[[^\]]*\]\(([^)\s]+\.png)\)")
TO_COME = re.compile(r"\*Picture to come: `([^`]+\.png)`")


def shown():
    """The picture names the guide's pages show or name, without `.png`."""
    pages = [ROOT / "docs/GUIDE.md", *sorted((ROOT / "docs/guide").rglob("*.md"))]
    names = set()
    for page in pages:
        text = page.read_text(encoding="utf-8")
        for path in LINK.findall(text) + TO_COME.findall(text):
            names.add(pathlib.PurePosixPath(path).stem)
    return names


def chunks(path):
    """The PNG's chunk types and its width and height. Reads the file."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    at, kinds, size = 8, [], None
    while at < len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        kind = data[at + 4 : at + 8]
        if kind == b"IHDR":
            size = struct.unpack(">II", data[at + 8 : at + 16])
        kinds.append(kind)
        at += 12 + length
    return kinds, size


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
    if len(argv) >= 2 and argv[1] in ("--check", "--strip"):
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
    if len(argv) < 3:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    out = pathlib.Path(argv[1])
    renders = [pathlib.Path(d) for d in argv[2:]]
    want = shown()
    out.mkdir(parents=True, exist_ok=True)
    found, failed = {}, []
    for d in renders:
        for png in sorted(d.glob("*.png")):
            name = png.stem
            if name not in want:
                continue
            if name in found:
                failed.append(f"{name}: drawn in both {found[name].parent} and {d}")
                continue
            found[name] = png
    off_size = []
    for name, png in sorted(found.items()):
        kinds, size = chunks(png)
        bad = sorted(k.decode() for k in set(kinds) & FORBIDDEN)
        if bad:
            failed.append(f"{name}: carries {', '.join(bad)}")
            continue
        if size != SIZE:
            off_size.append(f"{name}: {size[0]} x {size[1]}")
        shutil.copyfile(png, out / f"{name}.png")
        mark = png.with_name(f"{name}.mark.json")
        if mark.exists():
            shutil.copyfile(mark, out / mark.name)
        elif (out / mark.name).exists():
            (out / mark.name).unlink()
    removed = 0
    for f in sorted(out.iterdir()):
        stem = f.name.removesuffix(".mark.json").removesuffix(".png")
        if f.suffix in (".png", ".json") and stem not in want:
            f.unlink()
            removed += 1
    # A mark a tour asked for where the frame offered no such control.
    unmarked = []
    for d in renders:
        notes = d / "unmarked.txt"
        if notes.exists():
            for line in notes.read_text(encoding="utf-8").splitlines():
                if line.split(":", 1)[0] in want:
                    unmarked.append(line)
    missing = sorted(want - found.keys())
    marks = sum(1 for n in found if (out / f"{n}.mark.json").exists())
    print(f"{len(found)} pictures in {out}, {marks} with a mark; {removed} removed")
    for line in off_size:
        print(f"not at {SIZE[0]} x {SIZE[1]}: {line}")
    for line in unmarked:
        print(f"no mark: {line}")
    if missing:
        print(f"{len(missing)} shown but drawn by no tour:")
        for name in missing:
            print(f"  {name}")
    for line in failed:
        print(f"error: {line}", file=sys.stderr)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
