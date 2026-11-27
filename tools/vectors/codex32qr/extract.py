"""Reads the Codex32QR profile's test vectors out of its text into JSON.

    cd tools/vectors/codex32qr && python3 -I extract.py > vectors.json

`Codex32QR_Spec.md` §10 gives four vectors as prose: each its strings,
its fingerprint, a derivation path and the extended public key there on
testnet, and the pictures of its QR codes. This script writes them as
`vectors.json`, one entry a vector, the pictures under the names this
folder gives them (`img/split-shares/`, `img/s-shares/`; the profile's
folders are `img/Split Shares/` and `img/S Shares/`).

The profile's §11 names two more pictures of a secret without saying
which; their file names say `l0ve` and `wfsp`, and each decodes to the
secret of vector 1 and vector 2 (README.md). They are added to those
vectors here.

An extended public key is marked `complete` only when it is 111
characters long: the profile prints vector 2's cut short.
"""

import json
import re
import sys

EXTRA = {1: "img/s-shares/codex32_s_share_2_l0ve.png", 2: "img/s-shares/codex32_s_share_3_wfsp.png"}


def picture(link):
    link = link.replace("%20", " ")
    return link.replace("img/Split Shares/", "img/split-shares/").replace("img/S Shares/", "img/s-shares/")


def main():
    text = open("Codex32QR_Spec.md", encoding="utf-8").read()
    section = text.split("## 10) Reference test vectors", 1)[1].split("## 11)", 1)[0]
    out = []
    for block in re.split(r"^### Test vector ", section, flags=re.M)[1:]:
        number = int(block.split(" ", 1)[0])
        v = {"vector": number, "shares": {}, "pictures": {}}
        for label, string in re.findall(r"^(Share [A-Z]|Derived secret seed S|S Share): (\S+)$", block, re.M):
            if label.startswith("Share "):
                v["shares"][label[-1].lower()] = string
            else:
                v["secret"] = string
        strings = list(v["shares"].values()) + [v["secret"]]
        v["identifier"] = strings[0][4:8]
        v["threshold"] = int(strings[0][3])
        v["network"] = re.search(r"^- Network: (\S+)$", block, re.M).group(1)
        v["fingerprint"] = re.search(r"^- Fingerprint: `([0-9a-f]{8})`$", block, re.M).group(1)
        v["derivation"] = re.search(r"^- Derivation: `([^`]+)`$", block, re.M).group(1)
        xpub = re.search(r"^- Xpub: `([^`]+)`$", block, re.M).group(1)
        v["xpub"] = {"text": xpub, "complete": len(xpub) == 111}
        for link in re.findall(r"\]\((img/[^)]+\.png)\)", block):
            name = picture(link)
            index = "s" if "/s-shares/" in name else name.rsplit("_", 1)[1][0]
            v["pictures"][index] = name
        if number in EXTRA:
            v["pictures"]["s"] = EXTRA[number]
        out.append(v)
    if [v["vector"] for v in out] != [1, 2, 3, 4]:
        raise SystemExit("expected vectors 1 to 4")
    json.dump({"source": "Codex32QR_Spec.md §10 (README.md)", "vectors": out}, sys.stdout, indent=1)
    sys.stdout.write("\n")


main()
