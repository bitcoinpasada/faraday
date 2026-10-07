#!/usr/bin/env python3
"""Generate the kernel's boot logo at a panel's exact size.

The kernel draws its logo at the top left of the framebuffer and does not
scale it, so the way to centre a mark on a panel is to make the logo the
whole panel: a black canvas the size of the display with the OpenSigner
mark and the word OPENSIGNER in the middle of it.

The mark is a pixel grid in mark.txt, scaled by a whole number so it comes
out about a third of the panel wide; the word is a 5x7 pixel font held
below. Both are drawn from three colours, which is well inside the 224 a
CLUT224 logo may use.

The output is a plain (ASCII, P3) PPM. The kernel's pnmtologo, which turns
the file into the C array the logo is compiled from, refuses binary PNM
outright ("Binary PNM is not supported"), so P3 is the only form that
builds. The file is large and short-lived: it exists between build.sh and
the kernel's own build step and goes no further.

  make-logo.py --mark mark.txt --width 480 --height 640 --out logo.ppm
"""

import argparse
import sys

# The mark's three colours: the orange it is drawn in, the lighter edge
# that gives it its lit look, and the black everything sits on.
ORANGE = (255, 140, 0)
LIT = (255, 200, 90)
BLACK = (0, 0, 0)

INK = {"#": ORANGE, "o": LIT, ".": BLACK}

# The word under the mark, in the plain orange. Nine letters are enough
# for OPENSIGNER; each is five wide and seven tall.
FONT = {
    "O": [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    "P": ["####.", "#...#", "#...#", "####.", "#....", "#....", "#...."],
    "E": ["#####", "#....", "#....", "####.", "#....", "#....", "#####"],
    "N": ["#...#", "##..#", "#.#.#", "#.#.#", "#..##", "#...#", "#...#"],
    "S": [".####", "#....", "#....", ".###.", "....#", "....#", "####."],
    "I": ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####"],
    "G": [".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###."],
    "R": ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
}

WORD = "OPENSIGNER"

# The font's cell, and the blank column between two letters.
GLYPH_W = 5
GLYPH_H = 7
GLYPH_GAP = 1


def read_mark(path):
    """The mark as a list of equal-length rows of '#', 'o' and '.'."""
    with open(path, encoding="ascii") as f:
        rows = [line.rstrip("\n") for line in f if line.strip()]
    if not rows:
        raise SystemExit(f"{path}: no rows")
    width = len(rows[0])
    for i, row in enumerate(rows):
        if len(row) != width:
            raise SystemExit(f"{path}: row {i + 1} is {len(row)} wide, not {width}")
        for c in row:
            if c not in INK:
                raise SystemExit(f"{path}: row {i + 1} has {c!r}, not one of #o.")
    return rows


def blit(canvas, rows, ink, left, top, scale):
    """Draw a grid of characters, each pixel a scale x scale block."""
    height = len(canvas)
    width = len(canvas[0])
    for y, row in enumerate(rows):
        for x, c in enumerate(row):
            colour = ink(c)
            if colour is None:
                continue
            for dy in range(scale):
                cy = top + y * scale + dy
                if not 0 <= cy < height:
                    continue
                line = canvas[cy]
                for dx in range(scale):
                    cx = left + x * scale + dx
                    if 0 <= cx < width:
                        line[cx] = colour


def word_width(word, scale):
    return (len(word) * (GLYPH_W + GLYPH_GAP) - GLYPH_GAP) * scale


def draw_word(canvas, word, left, top, scale):
    for i, letter in enumerate(word):
        rows = FONT.get(letter)
        if rows is None:
            raise SystemExit(f"no glyph for {letter!r}")
        blit(
            canvas,
            rows,
            lambda c: ORANGE if c == "#" else None,
            left + i * (GLYPH_W + GLYPH_GAP) * scale,
            top,
            scale,
        )


def build(mark, width, height):
    """The whole canvas: the mark over the word, centred on black."""
    mark_w = len(mark[0])
    mark_h = len(mark)

    # About a third of the panel wide, and never smaller than the grid.
    scale = max(1, round(width / 3 / mark_w))
    # The word is small enough to read as a caption, not a second mark.
    text_scale = max(1, scale // 3)
    while word_width(WORD, text_scale) > width and text_scale > 1:
        text_scale -= 1

    gap = 2 * scale
    block_h = mark_h * scale + gap + GLYPH_H * text_scale
    top = max(0, (height - block_h) // 2)

    canvas = [[BLACK] * width for _ in range(height)]
    blit(canvas, mark, INK.get, (width - mark_w * scale) // 2, top, scale)
    draw_word(
        canvas,
        WORD,
        (width - word_width(WORD, text_scale)) // 2,
        top + mark_h * scale + gap,
        text_scale,
    )
    return canvas


def write_ppm(canvas, path):
    width = len(canvas[0])
    height = len(canvas)
    with open(path, "w", encoding="ascii") as f:
        f.write(f"P3\n{width} {height}\n255\n")
        for row in canvas:
            f.write(" ".join(f"{r} {g} {b}" for r, g, b in row))
            f.write("\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--mark", required=True, help="the pixel grid to draw")
    ap.add_argument("--width", type=int, required=True, help="panel width")
    ap.add_argument("--height", type=int, required=True, help="panel height")
    ap.add_argument("--out", required=True, help="the PPM to write")
    args = ap.parse_args()
    if args.width <= 0 or args.height <= 0:
        raise SystemExit("the panel has no pixels")

    canvas = build(read_mark(args.mark), args.width, args.height)
    write_ppm(canvas, args.out)
    colours = {c for row in canvas for c in row}
    if len(colours) > 224:
        raise SystemExit(f"{len(colours)} colours; a CLUT224 logo may have 224")
    print(
        f"logo {args.width}x{args.height}, {len(colours)} colours -> {args.out}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
