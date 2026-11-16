"""First receive address of multisig wallets, made by Bitcoin Core.

Runs the pinned bitcoind of `tools/crosscheck/core_node.py` on testnet4 with
no network. Three master keys are made with the standard library from the
seeds of BIP 32's test vectors 1, 2 and 3 (HMAC-SHA512 "Bitcoin seed": no
Faraday code); each is imported into a blank Bitcoin Core wallet as
descriptors, and Core lists the account keys with their origins. Of those
keys the script writes one descriptor per kind the Check a multisig setup
flow reads, takes its checksum from Core (`getdescriptorinfo`) and the
first receive address from Core (`deriveaddresses <descriptor> [0,0]`), into
`vectors.json`, beside this file.

The node is a testnet4 one, so Core writes tpub keys and tb1/2 addresses, which
the check shows for tpub keys; nothing is changed after Core.

Usage: python3 -I -B tools/vectors/multisig-check/gen.py
Standard library only.
"""

import hashlib
import hmac
import json
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))

import core_node  # noqa: E402

# BIP 32 test vectors 1, 2 and 3: the seeds.
SEEDS = [
    "000102030405060708090a0b0c0d0e0f",
    "fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542",
    "4b381541583be4423346c643850da4b320e46a87ae3d2a4e6da11eba819cd4acba45d239319ac14f863b8d5ab5a0d0c64d2e8a1e7d1457df2e5a3c51c73235be",
]
# The account paths: BIP 48 script types 2 and 1, BIP 45, BIP 87.
PATHS = {
    "wsh": "48h/0h/0h/2h",
    "shwsh": "48h/0h/0h/1h",
    "sh": "45h",
    "tr": "87h/0h/0h",
}
# BIP 341's NUMS point H, the x-only internal key of a script-only output.
NUMS = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"

B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
TPRV = bytes.fromhex("04358394")


def b58encode(raw):
    n = int.from_bytes(raw, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return "1" * (len(raw) - len(raw.lstrip(b"\0"))) + out


def b58check_encode(payload):
    return b58encode(payload + hashlib.sha256(hashlib.sha256(payload).digest()).digest()[:4])


def b58check_decode(text):
    n = 0
    for ch in text:
        n = n * 58 + B58.index(ch)
    raw = n.to_bytes((n.bit_length() + 7) // 8, "big")
    raw = b"\0" * (len(text) - len(text.lstrip("1"))) + raw
    payload, check = raw[:-4], raw[-4:]
    assert hashlib.sha256(hashlib.sha256(payload).digest()).digest()[:4] == check
    return payload


def master_tprv(seed_hex):
    i = hmac.new(b"Bitcoin seed", bytes.fromhex(seed_hex), hashlib.sha512).digest()
    return b58check_encode(TPRV + b"\0" * 9 + i[32:] + b"\0" + i[:32])


def with_checksum(rpc, text):
    return text + "#" + rpc.getdescriptorinfo(text)["checksum"]


def account_keys(rpc):
    """For each seed, each kind's `[origin]tpub` as Core lists it."""
    keys = []
    for n, seed in enumerate(SEEDS):
        name = f"k{n}"
        rpc.createwallet(name, False, True, "", False, True, False)
        w = core_node._wallet_client(name)
        prv = master_tprv(seed)
        done = w.importdescriptors([
            {"desc": with_checksum(rpc, f"pkh({prv}/{p})"),
             "timestamp": "now", "active": False}
            for p in PATHS.values()])
        assert all(r["success"] for r in done), done
        got = {}
        for d in w.listdescriptors(False)["descriptors"]:
            inner = d["desc"].split("#")[0][len("pkh("):-1]
            origin, tpub = inner[1:].split("]", 1)
            for kind, p in PATHS.items():
                if origin.split("/", 1)[1] == p:
                    got[kind] = f"[{origin}]{tpub}"
        assert set(got) == set(PATHS), got
        keys.append(got)
    return keys


def main():
    with core_node.Node(wallet=True, chain="testnet4") as node:
        os.environ.update(node.environ())
        rpc = core_node.rpc()
        keys = account_keys(rpc)

        def list_of(kind):
            return ",".join(k[kind] + "/0/*" for k in keys)

        wsh, shwsh, sh, tr = (list_of(k) for k in ("wsh", "shwsh", "sh", "tr"))
        a, b = keys[0]["wsh"] + "/0/*", keys[1]["wsh"] + "/0/*"
        cases = [
            ("wsh sortedmulti 2 of 3", f"wsh(sortedmulti(2,{wsh}))"),
            ("wsh multi 2 of 3", f"wsh(multi(2,{wsh}))"),
            ("sh wsh sortedmulti 2 of 3", f"sh(wsh(sortedmulti(2,{shwsh})))"),
            ("sh wsh multi 2 of 3", f"sh(wsh(multi(2,{shwsh})))"),
            ("sh sortedmulti 2 of 3", f"sh(sortedmulti(2,{sh}))"),
            ("sh multi 2 of 3", f"sh(multi(2,{sh}))"),
            ("tr sortedmulti_a 2 of 3", f"tr({NUMS},sortedmulti_a(2,{tr}))"),
            ("tr multi_a 2 of 3", f"tr({NUMS},multi_a(2,{tr}))"),
            ("wsh miniscript", f"wsh(or_d(pk({a}),and_v(v:pk({b}),older(144))))"),
        ]
        out = []
        for name, text in cases:
            desc = with_checksum(rpc, text)
            addr = rpc.deriveaddresses(desc, [0, 0])[0]
            out.append({"name": name, "descriptor": desc,
                        "address": addr})
        (HERE / "vectors.json").write_text(json.dumps(out, indent=1) + "\n")
        print("ok")


if __name__ == "__main__":
    main()
