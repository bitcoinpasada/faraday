#!/usr/bin/env python3
"""The kernel's keymap with every key K_HOLE.

The kernel's virtual terminal turns each key a keyboard reports into a
character through `drivers/tty/vt/defkeymap.c_shipped` and queues it for
whoever reads /dev/tty1. Faraday reads keys from evdev and has no use for
that queue, which would hold a passphrase after the app is gone. With every
key `K_HOLE` (0xf200 in the file) the terminal turns no key into anything.

    empty-keymap.py write FILE   rewrite FILE in place, every key K_HOLE
    empty-keymap.py check FILE   exit 1 unless every key in FILE is K_HOLE

The key maps are the `NR_KEYS` arrays; the function key table and the
accents are left as they are, since no key reaches them.
"""
import re
import sys

K_HOLE = "0xf200"
BLOCK = re.compile(r"(\n[^\n]*\b\w+_map\s*\[\s*NR_KEYS\s*\]\s*=\s*\{)(.*?)(\n\s*\};)", re.S)
ENTRY = re.compile(r"0[xX][0-9a-fA-F]+")


def main(argv):
    if len(argv) != 3 or argv[1] not in ("write", "check"):
        print(__doc__, file=sys.stderr)
        return 2
    path = argv[2]
    with open(path, encoding="utf-8") as f:
        text = f.read()
    found = list(BLOCK.finditer(text))
    if not found:
        print(f"{path}: no key map found", file=sys.stderr)
        return 1
    if argv[1] == "write":
        text = BLOCK.sub(lambda m: m.group(1) + ENTRY.sub(K_HOLE, m.group(2)) + m.group(3), text)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        return 0
    for m in found:
        if any(e.lower() != K_HOLE for e in ENTRY.findall(m.group(2))):
            print(f"{path}: a key map still maps keys", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
