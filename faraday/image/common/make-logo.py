#!/usr/bin/env python3
"""Faraday: upstream's common/make-logo.py at c418768, drawing Faraday's
mark (faraday-mark.txt, beside this script) alone, over its name, in the
middle of the logo. Upstream's build.sh still passes --mark (its own
mark.txt); it is accepted and not drawn.

The kernel draws its logo at the top left of the framebuffer and does
not scale it, so the logo is made the size build.sh asks for: the whole
panel on a Pi, a 320 x 320 square on a PC. The mark is a pixel grid of
'#' (the stroke), 'o' (its lit edge) and '.' (the field), scaled by a whole
number to about a quarter of the logo's width. The name is a 5x7 pixel
font. Three colours, well inside the 224 a CLUT224 logo may use.

faraday-mark.txt is Faraday's mark as the app draws it (faraday-ui
`Ui::mark`, F140) at 17 x 23 squares, set by hand: the edge one square
wide all round and a stair of one square a row to the tip, as the even
band reads at this size, and the keyhole's neck one square, so its head
stands apart from its slot.

The output is a plain (ASCII, P3) PPM: the kernel's pnmtologo refuses
binary PNM ("Binary PNM is not supported").

  make-logo.py --width 480 --height 640 --out logo.ppm
"""

import argparse
import os
import sys

BLACK = (0, 0, 0)
# The mark as the app's `Ui::mark` draws it in the default theme on its
# Light page (faraday-ui's pal/schemes.rs, Cobalt Light at Standard
# contrast): the field Cobalt Light's surface (#FAF8FF), the shield in
# primary (#00327D), its edge primary mixed half with white, the name in
# primary (owner, 2026-10-27).
COLOURS = {"field": (0xFA, 0xF8, 0xFF), "#": (0x00, 0x32, 0x7D), "o": (0x80, 0x99, 0xBE), "name": (0x00, 0x32, 0x7D)}

FARADAY_MARK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "faraday-mark.txt")

FONT = {
    "F": ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
    "A": [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    "R": ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    "D": ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
    "Y": ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
}
GLYPH_W, GLYPH_H, GLYPH_GAP = 5, 7, 1
PHI = (1 + 5 ** 0.5) / 2


def read_mark(path):
    """The grid as equal-length rows of '#', 'o' and '.'."""
    with open(path, encoding="ascii") as f:
        rows = [line.rstrip("\n") for line in f if line.strip()]
    if not rows:
        raise SystemExit(f"{path}: no rows")
    width = len(rows[0])
    for i, row in enumerate(rows):
        if len(row) != width:
            raise SystemExit(f"{path}: row {i + 1} is {len(row)} wide, not {width}")
        bad = set(row) - set("#o.")
        if bad:
            raise SystemExit(f"{path}: row {i + 1} has {bad.pop()!r}, not one of #o.")
    return rows


def bounds(rows):
    """The columns and rows the mark uses: (left, right, top, bottom)."""
    cols = [x for row in rows for x, c in enumerate(row) if c != "."]
    used = [y for y, row in enumerate(rows) if row.strip(".")]
    return min(cols), max(cols) + 1, min(used), max(used) + 1


def blit(canvas, rows, ink, left, top, scale):
    height, width = len(canvas), len(canvas[0])
    for y, row in enumerate(rows):
        for x, c in enumerate(row):
            colour = ink.get(c)
            if colour is None:
                continue
            for dy in range(scale):
                cy = top + y * scale + dy
                if 0 <= cy < height:
                    line = canvas[cy]
                    for dx in range(scale):
                        cx = left + x * scale + dx
                        if 0 <= cx < width:
                            line[cx] = colour


def word_width(word, scale):
    return (len(word) * (GLYPH_W + GLYPH_GAP) - GLYPH_GAP) * scale


def draw_word(canvas, word, centre, top, scale, colour):
    left = centre - word_width(word, scale) // 2
    for i, letter in enumerate(word):
        blit(canvas, FONT[letter], {"#": colour}, left + i * (GLYPH_W + GLYPH_GAP) * scale, top, scale)


def build(faraday, width, height, golden=False):
    """Faraday's mark over FARADAY, the pair centred on the logo. The name
    is set at the smallest whole scale that makes it wider than the mark.
    With golden, the logo is a square whose side is the pair's longer side
    times the golden ratio, the pair drawn at the size `width` gives it."""
    left, right, used_top, used_bottom = bounds(faraday)
    scale = max(1, round(width / 4 / (right - left)))
    text_scale = (right - left) * scale // word_width("FARADAY", 1) + 1
    while word_width("FARADAY", text_scale) > width - 8 and text_scale > 1:
        text_scale -= 1
    gap = 2 * scale
    block = (used_bottom - used_top) * scale + gap + GLYPH_H * text_scale
    if golden:
        block_w = max((right - left) * scale, word_width("FARADAY", text_scale))
        width = height = round(max(block, block_w) * PHI)
    top = max(0, (height - block) // 2)
    baseline = top + (used_bottom - used_top) * scale
    centre = width // 2
    colours = COLOURS
    canvas = [[colours["field"]] * width for _ in range(height)]
    x = centre - (left + right) * scale // 2
    y = baseline - used_bottom * scale
    blit(canvas, faraday, {"#": colours["#"], "o": colours["o"]}, x, y, scale)
    draw_word(canvas, "FARADAY", centre, baseline + gap, text_scale, colours["name"])
    return canvas


def write_ppm(canvas, path):
    with open(path, "w", encoding="ascii") as f:
        f.write(f"P3\n{len(canvas[0])} {len(canvas)}\n255\n")
        for row in canvas:
            f.write(" ".join(f"{r} {g} {b}" for r, g, b in row))
            f.write("\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--mark", help="upstream's pixel grid; accepted, not drawn")
    ap.add_argument("--faraday-mark", default=FARADAY_MARK, help="Faraday's pixel grid")
    ap.add_argument("--width", type=int, required=True, help="logo width")
    ap.add_argument("--height", type=int, required=True, help="logo height")
    ap.add_argument("--out", required=True, help="the PPM to write")
    ap.add_argument("--golden", action="store_true",
                    help="a square the golden ratio around the mark and name (a display not known yet)")
    args = ap.parse_args()
    if args.width <= 0 or args.height <= 0:
        raise SystemExit("the panel has no pixels")
    canvas = build(read_mark(args.faraday_mark), args.width, args.height, args.golden)
    write_ppm(canvas, args.out)
    colours = {c for row in canvas for c in row}
    if len(colours) > 224:
        raise SystemExit(f"{len(colours)} colours; a CLUT224 logo may have 224")
    print(f"logo {len(canvas[0])}x{len(canvas)}, {len(colours)} colours -> {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
