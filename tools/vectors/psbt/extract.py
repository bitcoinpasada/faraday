"""Extracts the BIP-174 test vectors from bip-0174.mediawiki into
tools/vectors/psbt/bip174.txt. Run once; the output is committed."""
import re, sys
src = open(sys.argv[1], encoding="utf-8").read()
out = []
sec = src[src.index("==Test Vectors=="):src.index("==Rationale==")]
lines = sec.splitlines()

def pre(s):
    m = re.search(r"<pre>(.*?)</pre>", s)
    return m.group(1).strip()

out.append("# BIP-174 test vectors, extracted from")
out.append("# https://raw.githubusercontent.com/bitcoin/bips/master/bip-0174.mediawiki")
out.append("# by tools/vectors/psbt/extract.py. Documented in core/osk-psbt/README.md.")
out.append("#")
out.append("# Format: blank-line separated records of `key: value` lines.")
out.append("#   case: free text description")
out.append("#   expect: invalid | valid | signer-fails")
out.append("#   hex: PSBT as hex     base64: the same PSBT as base64")
out.append("")
expect = None
i = 0
role_section = False
while i < len(lines):
    l = lines[i]
    if l.startswith("The following are invalid"): expect = "invalid"
    elif l.startswith("The following are valid"): expect = "valid"
    elif l.startswith("Fails Signer checks"): expect = "signer-fails"
    elif l.startswith("The private keys in the tests below"): break
    elif l.startswith("* Case:"):
        name = l[len("* Case:"):].strip()
        h = pre(lines[i+1]); b = pre(lines[i+2])
        assert lines[i+1].startswith("** Bytes in Hex") and lines[i+2].startswith("** Base64")
        out += [f"case: {name}", f"expect: {expect}", f"hex: {h}", f"base64: {b}", ""]
        i += 2
    i += 1
open(sys.argv[2], "w").write("\n".join(out) + "\n")

# Role chain
roles = []
rest = lines[i:]
text = "\n".join(rest)
tprv = re.search(r"Extended Private Key: <pre>(.*?)</pre>", text).group(1)
wif_master = re.search(r"Seed: <pre>(.*?)</pre>", text).group(1)
def all_pre_pairs(block):
    hs = re.findall(r"Bytes in Hex: <pre>(.*?)</pre>", block)
    bs = re.findall(r"Base64 String: <pre>(.*?)</pre>", block)
    return list(zip(hs, bs))
markers = [
    ("creator", "must create this PSBT:"),
    ("updater", "Must create this PSBT:"),
    ("updater-sighash-all", "An updater which adds SIGHASH_ALL"),
    ("signer-1", "cR6SXDoyfQrcp4piaiHE97Rsgta9mNhGTen9XeonVgwsh4iSgw6d"),
    ("signer-2", "cNBc3SWUip9PPm1GjRoLEJT6T41iNzCYtD7qro84FMnM5zEqeJsE"),
    ("combiner", "a combiner must create this PSBT"),
    ("finalizer", "an input finalizer must create this PSBT"),
    ("extractor", "a transaction extractor must create this Bitcoin transaction"),
]
o = []
o.append("# BIP-174 role test vectors (creator → updater → signer → combiner → finalizer → extractor),")
o.append("# extracted from https://raw.githubusercontent.com/bitcoin/bips/master/bip-0174.mediawiki")
o.append("# by tools/vectors/psbt/extract.py. Same record format as bip174.txt; additional keys:")
o.append("#   master-tprv, master-wif, key: <wif> <path>, redeem-script, witness-script, prev-tx,")
o.append("#   pubkey: <hex> <path>, tx: final transaction hex (extractor step).")
o.append("")
o.append(f"master-tprv: {tprv}")
o.append(f"master-wif: {wif_master}")
for k, p in re.findall(r"Key: <tt>([0-9a-f]+)</tt>, Derivation Path: <tt>(.*?)</tt>", text):
    o.append(f"pubkey: {k} {p}")
for w, p in re.findall(r"\* <tt>([1-9A-HJ-NP-Za-km-z]{51,52})</tt> \(<tt>(m/[^<]*)</tt>\)", text):
    o.append(f"key: {w} {p}")
rs = re.search(r"\* Redeem Scripts:\n(.*?)\* Witness Scripts:", text, re.S).group(1)
for s in re.findall(r"<tt>([0-9a-f]+)</tt>", rs): o.append(f"redeem-script: {s}")
ws = re.search(r"\* Witness Scripts:\n(.*?)\* Previous Transactions:", text, re.S).group(1)
for s in re.findall(r"<tt>([0-9a-f]+)</tt>", ws): o.append(f"witness-script: {s}")
pt = re.search(r"\* Previous Transactions:\n(.*?)\* Public Keys", text, re.S).group(1)
for s in re.findall(r"<pre>([0-9a-f]+)</pre>", pt): o.append(f"prev-tx: {s}")
o.append("")
pos = 0
for name, marker in markers:
    idx = text.index(marker, pos)
    if name == "extractor":
        h = re.search(r"Bytes in Hex: <pre>(.*?)</pre>", text[idx:]).group(1)
        o += [f"case: role {name}", f"tx: {h}", ""]
        pos = idx + len(marker)
        continue
    h, b = all_pre_pairs(text[idx:])[0]
    o += [f"case: role {name}", "expect: valid", f"hex: {h}", f"base64: {b}", ""]
    pos = idx + len(marker)
# unknown-key combiner
u = text[text.index("Given these two PSBTs with unknown key-value pairs"):]
pairs = all_pre_pairs(u)
assert len(pairs) == 3, len(pairs)
for name, (h, b) in zip(["combiner-unknown-a", "combiner-unknown-b", "combiner-unknown-result"], pairs):
    o += [f"case: role {name}", "expect: valid", f"hex: {h}", f"base64: {b}", ""]
open(sys.argv[3], "w").write("\n".join(o) + "\n")
