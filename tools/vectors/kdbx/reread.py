#!/usr/bin/env python3
"""Reads a KDBX file with pykeepass and prints what it holds, for
faraday/faraday-vault/tests/kdbx_xml.rs: one line per entry,
`group/title<TAB>user<TAB>password<TAB>url<TAB>history count<TAB>attachments`,
then `bin: yes` or `bin: no` for a recycle bin group. Usage:

    python3 -I -B tools/vectors/kdbx/reread.py FILE PASSPHRASE [more]

With `more`, a line `data: KEY=VALUE` for each of the database's custom data
items, `entry: TITLE tags=TAGS icon=NUMBER` and `entry data: KEY=VALUE` for each
entry, and `attachment: NAME=HEX` (the bytes in hex) for each attachment follow.
"""
import sys

from pykeepass import PyKeePass

kp = PyKeePass(sys.argv[1], password=sys.argv[2])
for e in kp.entries:
    path = "/".join(e.path[:-1])
    print(
        "\t".join(
            [
                path + "/" + (e.title or ""),
                e.username or "",
                e.password or "",
                e.url or "",
                str(len(e.history or [])),
                ",".join(a.filename for a in e.attachments),
            ]
        )
    )
print("bin:", "yes" if kp.recyclebin_group is not None else "no")
if len(sys.argv) > 3 and sys.argv[3] == "more":
    for item in kp.tree.find("Meta").find("CustomData").iter("Item"):
        print("data: %s=%s" % (item.find("Key").text, item.find("Value").text))
    for e in kp.entries:
        for a in e.attachments:
            print("attachment: %s=%s" % (a.filename, a.data.hex()))
    for e in kp.entries:
        print("entry: %s tags=%s icon=%s" % (e.title, ";".join(e.tags or []), e.icon))
        for item in e._element.findall("CustomData/Item"):
            print("entry data: %s=%s" % (item.find("Key").text, item.find("Value").text))
