#!/usr/bin/env python3
"""Faraday: upstream's common/make-logo.py at c418768, drawing two marks
side by side: Faraday's on the left (faraday-mark.txt, beside this
script) and OpenSigner's on the right (--mark, upstream's mark.txt), each
with its name under it.

The kernel draws its logo at the top left of the framebuffer and does
not scale it, so the logo is made the size build.sh asks for: the whole
panel on a Pi, a 320 x 320 square on a PC. Each mark is a pixel grid of
'#' (the stroke), 'o' (its lit edge) and '.' (black), scaled by a whole
number to about a fifth of the logo's width, with the bottoms of the two
marks on one line. The names are a 5x7 pixel font. Five colours, well
inside the 224 a CLUT224 logo may use.

The same faraday-mark.txt is Faraday's mark in the app's sidebar
(faraday-core `Ui::mark`).

The output is a plain (ASCII, P3) PPM: the kernel's pnmtologo refuses
binary PNM ("Binary PNM is not supported").

  make-logo.py --mark mark.txt --width 480 --height 640 --out logo.ppm
"""

import argparse
import os
import sys

BLACK = (0, 0, 0)
# OpenSigner's orange and its lit edge, as upstream draws them.
OPENSIGNER = {"#": (255, 140, 0), "o": (255, 200, 90)}
# Faraday's: cyan, the app's accent, and its lit edge.
FARADAY = {"#": (0, 170, 230), "o": (140, 225, 255)}

FARADAY_MARK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "faraday-mark.txt")

FONT = {
    "O": [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "P": ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    "E": ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
    "N": ["#...#", "##..#", "#.#.#", "#.#.#", "#..##", "#...#", "#...#"],
    "S": [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    "I": ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####"],
    "G": [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###."],
    "R": ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    "F": ["#####", "#....", "#....", "####.", "#....", "#....", "#...."],
    "A": [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"],
    "D": ["####.", "#...#", "#...#", "#...#", "#...#", "#...#", "####."],
    "Y": ["#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#.."],
}
GLYPH_W, GLYPH_H, GLYPH_GAP = 5, 7, 1


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


def build(faraday, opensigner, width, height):
    """Faraday's mark over FARADAY at a quarter of the width, OpenSigner's
    over OPENSIGNER at three quarters, the pair centred in height."""
    marks = [(faraday, FARADAY, "FARADAY"), (opensigner, OPENSIGNER, "OPENSIGNER")]
    tallest = max(bounds(rows)[3] - bounds(rows)[2] for rows, _, _ in marks)
    widest = max(bounds(rows)[1] - bounds(rows)[0] for rows, _, _ in marks)
    scale = max(1, round(width / 5 / widest))
    text_scale = max(1, scale // 3)
    while word_width("OPENSIGNER", text_scale) > width // 2 - 4 and text_scale > 1:
        text_scale -= 1
    gap = 2 * scale
    block = tallest * scale + gap + GLYPH_H * text_scale
    top = max(0, (height - block) // 2)
    baseline = top + tallest * scale
    canvas = [[BLACK] * width for _ in range(height)]
    for i, (rows, ink, word) in enumerate(marks):
        centre = width * (1 + 2 * i) // 4
        left, right, used_top, used_bottom = bounds(rows)
        x = centre - (left + right) * scale // 2
        y = baseline - used_bottom * scale
        blit(canvas, rows, ink, x, y, scale)
        draw_word(canvas, word, centre, baseline + gap, text_scale, ink["#"])
    return canvas


def write_ppm(canvas, path):
    with open(path, "w", encoding="ascii") as f:
        f.write(f"P3\n{len(canvas[0])} {len(canvas)}\n255\n")
        for row in canvas:
            f.write(" ".join(f"{r} {g} {b}" for r, g, b in row))
            f.write("\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--mark", required=True, help="OpenSigner's pixel grid")
    ap.add_argument("--faraday-mark", default=FARADAY_MARK, help="Faraday's pixel grid")
    ap.add_argument("--width", type=int, required=True, help="logo width")
    ap.add_argument("--height", type=int, required=True, help="logo height")
    ap.add_argument("--out", required=True, help="the PPM to write")
    args = ap.parse_args()
    if args.width <= 0 or args.height <= 0:
        raise SystemExit("the panel has no pixels")
    canvas = build(read_mark(args.faraday_mark), read_mark(args.mark), args.width, args.height)
    write_ppm(canvas, args.out)
    colours = {c for row in canvas for c in row}
    if len(colours) > 224:
        raise SystemExit(f"{len(colours)} colours; a CLUT224 logo may have 224")
    print(f"logo {args.width}x{args.height}, {len(colours)} colours -> {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
