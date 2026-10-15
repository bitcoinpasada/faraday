#!/usr/bin/env python3
"""Byte-level edits of the APK fixtures (docs/APK-SIGNING.md §8), used by
make.sh. Standard library only. Each command writes files; none signs.

    variants.py add-library IN OUT   IN plus a stored lib/arm64-v8a/libfixture.so
    variants.py add-asset IN OUT     IN plus a stored 2.5 MiB asset of fixed bytes
    variants.py unaligned IN OUT     IN with its entries moved off their boundaries
    variants.py variants IN DIR      IN once per rule of "One reading only"
"""

from __future__ import annotations

import hashlib
import struct
import sys
import zlib
from pathlib import Path

LIBRARY = "lib/arm64-v8a/libfixture.so"
ASSET = "assets/fixed.bin"


class Entry:
    """One ZIP entry as stored: its data is the compressed bytes."""

    def __init__(self, name: bytes, flags: int, method: int, crc: int, usize: int, data: bytes):
        self.name, self.flags, self.method, self.crc = name, flags, method, crc
        self.usize, self.data = usize, data
        self.local_extra = b""
        # Edits that apply to the local header alone.
        self.local_flags: int | None = None
        self.local_method: int | None = None
        self.local_usize: int | None = None

    def copy(self) -> "Entry":
        return Entry(self.name, self.flags, self.method, self.crc, self.usize, self.data)


def fixed_bytes(label: bytes, n: int) -> bytes:
    out, block = b"", label
    while len(out) < n:
        block = hashlib.sha256(block).digest()
        out += block
    return out[:n]


def read_zip(path: Path) -> list[Entry]:
    b = path.read_bytes()
    eocd = b.rindex(b"PK\x05\x06")
    count, _, cd_off = struct.unpack_from("<HII", b, eocd + 10)
    at, out = cd_off, []
    for _ in range(count):
        (_, _, _, flags, method, _, _, crc, csize, usize, n, x, c, _, _, _, lho) = struct.unpack_from("<4sHHHHHHIIIHHHHHII", b, at)
        name = b[at + 46 : at + 46 + n]
        ln, lx = struct.unpack_from("<HH", b, lho + 26)
        data_at = lho + 30 + ln + lx
        out.append(Entry(name, flags, method, crc, usize, b[data_at : data_at + csize]))
        at += 46 + n + x + c
    return out


def stored(name: str, content: bytes) -> Entry:
    return Entry(name.encode(), 0, 0, zlib.crc32(content), len(content), content)


def deflated(name: bytes, content: bytes) -> Entry:
    z = zlib.compressobj(9, zlib.DEFLATED, -15)
    packed = z.compress(content) + z.flush()
    return Entry(name, 0, 8, zlib.crc32(content), len(content), packed)


def pad_for(offset: int, n: int, want: int) -> bytes:
    """An extra field that puts the data at a multiple of `want`."""
    pad = (-(offset + 30 + n)) % want
    if pad == 0:
        return b""
    if pad < 4:
        pad += want
    return struct.pack("<HH", 0xD935, pad - 4) + b"\0" * (pad - 4)


def build(
    entries: list[Entry],
    *,
    align: bool = True,
    prefix: bytes = b"",
    gap_after: dict[int, bytes] | None = None,
    swap_cd: tuple[int, int] | None = None,
) -> bytes:
    """A ZIP of the entries, offsets and all, with the edits asked for."""
    out = bytearray(prefix)
    offsets = []
    for i, e in enumerate(entries):
        offsets.append(len(out))
        extra = e.local_extra
        if align and e.method == 0 and not e.name.endswith(b"/"):
            want = 16384 if e.name.lower().endswith(b".so") else 4
            extra = pad_for(len(out), len(e.name), want)
        flags = e.flags if e.local_flags is None else e.local_flags
        method = e.method if e.local_method is None else e.local_method
        usize = e.usize if e.local_usize is None else e.local_usize
        out += struct.pack("<4sHHHHHIIIHH", b"PK\x03\x04", 20, flags, method, 0, 0x21, e.crc, len(e.data), usize, len(e.name), len(extra))
        out += e.name + extra + e.data
        if gap_after and i in gap_after:
            out += gap_after[i]
    cd_off = len(out)
    cds = [
        struct.pack("<4sHHHHHHIIIHHHHHII", b"PK\x01\x02", 20, 20, e.flags, e.method, 0, 0x21, e.crc, len(e.data), e.usize, len(e.name), 0, 0, 0, 0, 0, off) + e.name
        for e, off in zip(entries, offsets)
    ]
    if swap_cd:
        a, b = swap_cd
        cds[a], cds[b] = cds[b], cds[a]
    cd = b"".join(cds)
    out += cd
    out += struct.pack("<4sHHHHIIH", b"PK\x05\x06", 0, 0, len(entries), len(entries), len(cd), cd_off, 0)
    return bytes(out)


# --- Android binary XML ----------------------------------------------------


class Axml:
    """A binary manifest: its strings, and its other chunks as
    [type, header after the first 8 bytes, body]."""

    def __init__(self, data: bytes):
        assert struct.unpack_from("<HHI", data, 0) == (3, 8, len(data))
        at, self.chunks = 8, []
        while at < len(data):
            t, h, size = struct.unpack_from("<HHI", data, at)
            whole = data[at : at + size]
            if t == 1:
                count, styles, flags, strings_at, _ = struct.unpack_from("<IIIII", whole, 8)
                assert styles == 0
                self.utf8 = bool(flags & 0x100)
                self.strings = [self._string(whole, strings_at, struct.unpack_from("<I", whole, 0x1C + 4 * i)[0]) for i in range(count)]
            else:
                self.chunks.append([t, whole[8:h], whole[h:]])
            at += size

    def _string(self, whole: bytes, base: int, off: int) -> str:
        p = base + off
        if self.utf8:
            def length(p):
                b0 = whole[p]
                return (((b0 & 0x7F) << 8) | whole[p + 1], p + 2) if b0 & 0x80 else (b0, p + 1)
            _, p = length(p)
            n, p = length(p)
            return whole[p : p + n].decode("utf-8")
        n = struct.unpack_from("<H", whole, p)[0]
        return whole[p + 2 : p + 2 + 2 * n].decode("utf-16-le")

    def encode(self) -> bytes:
        offs, blob = [], b""
        for s in self.strings:
            offs.append(len(blob))
            if self.utf8:
                e = s.encode("utf-8")
                length = lambda n: bytes([n]) if n < 0x80 else bytes([0x80 | n >> 8, n & 0xFF])
                blob += length(len(s)) + length(len(e)) + e + b"\0"
            else:
                blob += struct.pack("<H", len(s)) + s.encode("utf-16-le") + b"\0\0"
        blob += b"\0" * (-len(blob) % 4)
        strings_at = 0x1C + 4 * len(self.strings)
        pool = struct.pack("<HHIIIIII", 1, 0x1C, strings_at + len(blob), len(self.strings), 0, 0x100 if self.utf8 else 0, strings_at, 0)
        pool += b"".join(struct.pack("<I", o) for o in offs) + blob
        out = pool
        for t, h, body in self.chunks:
            out += struct.pack("<HHI", t, 8 + len(h), 8 + len(h) + len(body)) + h + body
        return struct.pack("<HHI", 3, 8, 8 + len(out)) + out

    def resource_ids(self) -> list[int]:
        for c in self.chunks:
            if c[0] == 0x0180:
                return list(struct.unpack("<%dI" % (len(c[2]) // 4), c[2]))
        raise SystemExit("no resource map")

    def set_resource_ids(self, ids: list[int]) -> None:
        for c in self.chunks:
            if c[0] == 0x0180:
                c[2] = struct.pack("<%dI" % len(ids), *ids)

    def element(self, name: str):
        for c in self.chunks:
            if c[0] == 0x0102 and self.strings[struct.unpack_from("<i", c[2], 4)[0]] == name:
                return c
        raise SystemExit(f"no element {name}")

    @staticmethod
    def attrs(chunk) -> list[list[int]]:
        (count,) = struct.unpack_from("<H", chunk[2], 12)
        return [list(struct.unpack_from("<iiiHBBI", chunk[2], 20 + i * 20)) for i in range(count)]

    @staticmethod
    def set_attrs(chunk, attrs) -> None:
        body = bytearray(chunk[2][:20])
        struct.pack_into("<H", body, 12, len(attrs))
        for a in attrs:
            body += struct.pack("<iiiHBBI", *a)
        chunk[2] = bytes(body)


def edit_manifest(xml: bytes, how: str) -> bytes:
    m = Axml(xml)
    ids = m.resource_ids()
    manifest, sdk = m.element("manifest"), m.element("uses-sdk")

    def named(attrs, rid):
        return next(a for a in attrs if a[1] < len(ids) and ids[a[1]] == rid)

    if how == "fake-id":
        # An attribute whose name is the text "versionCode" but whose
        # resource ID is another: Android ignores it.
        m.strings.append("versionCode")
        new = len(m.strings) - 1
        ids += [0] * (new - len(ids)) + [0x0101FFFF]
        m.set_resource_ids(ids)
        attrs = m.attrs(manifest)
        attrs.append([named(attrs, 0x0101021B)[0], new, -1, 8, 0, 0x10, 99])
        m.set_attrs(manifest, attrs)
    elif how == "version-name-reference":
        attrs = m.attrs(manifest)
        a = named(attrs, 0x0101021C)
        a[2], a[4], a[5], a[6] = -1, 0, 0x01, 0x7F010000
        m.set_attrs(manifest, attrs)
    elif how == "codename-sdk":
        m.strings.append("VanillaIceCream")
        s = len(m.strings) - 1
        attrs = m.attrs(sdk)
        a = named(attrs, 0x0101020C)
        a[2], a[4], a[5], a[6] = s, 0, 0x03, s
        m.set_attrs(sdk, attrs)
    else:
        raise SystemExit(how)
    return m.encode()


def with_manifest(entries: list[Entry], how: str) -> list[Entry]:
    out = []
    for e in entries:
        if e.name == b"AndroidManifest.xml":
            xml = e.data if e.method == 0 else zlib.decompress(e.data, -15)
            out.append(deflated(e.name, edit_manifest(xml, how)))
        else:
            out.append(e)
    return out


def main() -> None:
    cmd = sys.argv[1]
    src = Path(sys.argv[2])
    if cmd == "add-library":
        entries = read_zip(src) + [stored(LIBRARY, fixed_bytes(b"libfixture", 6000))]
        Path(sys.argv[3]).write_bytes(build(entries, align=False))
    elif cmd == "add-asset":
        entries = read_zip(src) + [stored(ASSET, fixed_bytes(b"asset", 2_621_440))]
        Path(sys.argv[3]).write_bytes(build(entries, align=False))
    elif cmd == "unaligned":
        entries = read_zip(src)
        # Five bytes of well-formed extra field in front of the first
        # entry's data move every stored entry after it off its boundary.
        entries[0].local_extra = struct.pack("<HHB", 0x7777, 1, 0)
        Path(sys.argv[3]).write_bytes(build(entries, align=False))
    elif cmd == "variants":
        out = Path(sys.argv[3])
        entries = read_zip(src)
        manifest = next(i for i, e in enumerate(entries) if e.name == b"AndroidManifest.xml")
        library = next(i for i, e in enumerate(entries) if e.name == LIBRARY.encode())

        def write(name: str, data: bytes) -> None:
            (out / f"{name}-unsigned.apk").write_bytes(data)

        write("prefix", build(entries, prefix=b"FARADAY-PREFIX!!"))
        write("gap", build(entries, gap_after={0: b"\0" * 8}))
        write("order", build(entries, swap_cd=(0, 1)))
        for label, attr, value in (("local-method", "local_method", 8), ("local-sizes", "local_usize", entries[library].usize + 1), ("local-flags", "local_flags", 0x0800)):
            e = [x.copy() for x in entries]
            setattr(e[library], attr, value)
            write(label, build(e))
        e = [x.copy() for x in entries]
        for x in e:
            x.flags |= 0x0008
        write("descriptor", build(e))
        write("duplicate-manifest", build(entries + [entries[manifest].copy()]))
        write("fake-id", build(with_manifest(entries, "fake-id")))
        write("version-name-reference", build(with_manifest(entries, "version-name-reference")))
        write("codename-sdk", build(with_manifest(entries, "codename-sdk")))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
