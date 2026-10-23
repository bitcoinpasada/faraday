"""Lists the strings whose checksum worksheets volvelle fills.

    cd tools/vectors/codex32-volvelle && python3 -I strings.py > strings.txt

Every valid string of BIP 93's vectors (`../bip93/vectors.json`: the
secrets, the sets' shares, derived shares and secrets, the alternates)
and of the Codex32QR profile's (`../codex32qr/vectors.json`) at 48 or 74
characters, the lengths the booklet prints checksum worksheets for, in
lower case, each once, in the order first met.
"""

import json

seen = []


def add(s):
    s = s.lower()
    if len(s) in (48, 74) and s not in seen:
        seen.append(s)


bip93 = json.load(open("../bip93/vectors.json"))
for case in bip93["secrets"]:
    add(case["string"])
for case in bip93["sets"]:
    for s in case["shares"] + list(case["derived"].values()) + [case["secret"]]:
        add(s)
for case in bip93["alternates"]:
    for s in case["strings"]:
        add(s)
for v in json.load(open("../codex32qr/vectors.json"))["vectors"]:
    for s in list(v["shares"].values()) + [v["secret"]]:
        add(s)
print("\n".join(seen))
