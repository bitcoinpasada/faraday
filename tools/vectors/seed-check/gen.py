"""BIP-44 and BIP-48 accounts of the test phrase, made by Bitcoin Core.

Runs the pinned bitcoind of `tools/crosscheck/core_node.py` on regtest with
no network. The test phrase ("abandon ... about", no passphrase) is turned
into a master key with the standard library (PBKDF2-HMAC-SHA512 and the
BIP-32 master step: no Faraday code), the master key is imported into a
blank Bitcoin Core wallet as descriptors, and Core's own answers are written
to `vectors.json`, beside this file:

- `bip44`: the account's extended public key (m/44'/0'/0') and the first
  receive and change addresses of `pkh()`;
- `bip48`: the cosigner extended public key at m/48'/0'/0'/2'.

The node is a regtest one, so Core writes tpub keys and regtest addresses;
the script changes only the version bytes of what Core wrote to the
mainnet ones (xpub, and the P2PKH prefix 0x00), the key and hash bytes
are Core's.

Usage: python3 -I -B tools/vectors/seed-check/gen.py
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

PHRASE = ("abandon abandon abandon abandon abandon abandon abandon abandon "
          "abandon abandon abandon about")
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
TPRV = bytes.fromhex("04358394")
TPUB = bytes.fromhex("043587cf")
XPUB = bytes.fromhex("0488b21e")


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


def master_tprv(phrase):
    seed = hashlib.pbkdf2_hmac("sha512", phrase.encode(), b"mnemonic", 2048)
    i = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    return b58check_encode(TPRV + b"\0" * 9 + i[32:] + b"\0" + i[:32])


def with_checksum(rpc, text):
    return text + "#" + rpc.getdescriptorinfo(text)["checksum"]


def mainnet_xpub(tpub):
    payload = b58check_decode(tpub)
    assert payload[:4] == TPUB
    return b58check_encode(XPUB + payload[4:])


def mainnet_p2pkh(address):
    payload = b58check_decode(address)
    assert payload[0] == 0x6F
    return b58check_encode(b"\0" + payload[1:])


def origin_key(desc, wrap):
    """The `[origin]key` of `pkh([origin]key/...)` as (origin, key)."""
    inner = desc[len(wrap):].split("#")[0]
    origin, rest = inner[1:].split("]", 1)
    return origin, rest.split("/")[0].rstrip(")")


def main():
    with core_node.Node(wallet=True) as node:
        os.environ.update(node.environ())
        rpc = core_node.rpc()
        rpc.createwallet("seed", False, True, "", False, True, False)
        w = core_node._wallet_client("seed")
        prv = master_tprv(PHRASE)
        descs = [
            {"desc": with_checksum(rpc, f"pkh({prv}/44h/0h/0h/0/*)"),
             "timestamp": "now", "active": True, "internal": False, "range": [0, 1]},
            {"desc": with_checksum(rpc, f"pkh({prv}/44h/0h/0h/1/*)"),
             "timestamp": "now", "active": True, "internal": True, "range": [0, 1]},
            {"desc": with_checksum(rpc, f"pkh({prv}/48h/0h/0h/2h)"),
             "timestamp": "now", "active": False},
        ]
        done = w.importdescriptors(descs)
        assert all(r["success"] for r in done), done
        out = {"phrase": PHRASE}
        listed = w.listdescriptors(False)["descriptors"]
        for d in listed:
            text = d["desc"]
            origin = text.split("[", 1)[1].split("]", 1)[0]
            tpub = text.split("#")[0].split("]", 1)[1].split("/")[0].rstrip(")")
            if origin.endswith("/44h/0h/0h"):
                kind = "change" if d["internal"] else "receive"
                addr = rpc.deriveaddresses(with_checksum(rpc, text.split("#")[0]), [0, 0])[0]
                out.setdefault("bip44", {"origin": origin.split("/", 1)[1],
                                         "xpub": mainnet_xpub(tpub)})[kind] = mainnet_p2pkh(addr)
            elif origin.endswith("/48h/0h/0h/2h"):
                out["bip48"] = {"origin": origin.split("/", 1)[1], "xpub": mainnet_xpub(tpub)}
        assert "bip44" in out and "bip48" in out, listed
        (HERE / "vectors.json").write_text(json.dumps(out, indent=1) + "\n")
        print("ok")


if __name__ == "__main__":
    main()
