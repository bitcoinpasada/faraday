#!/usr/bin/env python3
"""Reads a KDBX file with pykeepass and prints what it holds, for
faraday/faraday-vault/tests/kdbx_xml.rs: one line per entry,
`group/title<TAB>user<TAB>password<TAB>url<TAB>history count<TAB>attachments`,
then `bin: yes` or `bin: no` for a recycle bin group. Usage:

    python3 -I -B tools/vectors/kdbx/reread.py FILE PASSPHRASE
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
