#!/usr/bin/env python3
"""Faraday: upstream's common/make-logo.py at c418768, without the mark or
the word OPENSIGNER. The kernel's boot logo is a plain black canvas the
size of the panel: boot still shows FRAMEBUFFER_CONSOLE is alive, which is
what `kernel.required` asks it for (a black display means the firmware
and not a silent working device), without naming OpenSigner on a Faraday
device.

  make-logo.py --mark mark.txt --width 480 --height 640 --out logo.ppm

--mark is accepted and ignored, so build.sh's call needs no change.
"""

import argparse
import sys


def write_ppm(width, height, path):
    with open(path, "w", encoding="ascii") as f:
        f.write(f"P3\n{width} {height}\n255\n")
        row = ("0 0 0 " * width).strip() + "\n"
        for _ in range(height):
            f.write(row)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--mark", help="ignored; kept so build.sh's call needs no change")
    ap.add_argument("--width", type=int, required=True, help="panel width")
    ap.add_argument("--height", type=int, required=True, help="panel height")
    ap.add_argument("--out", required=True, help="the PPM to write")
    args = ap.parse_args()
    if args.width <= 0 or args.height <= 0:
        raise SystemExit("the panel has no pixels")
    write_ppm(args.width, args.height, args.out)
    print(f"logo {args.width}x{args.height}, blank -> {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
