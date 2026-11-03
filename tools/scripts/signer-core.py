"""The Bitcoin Core round trip behind the signer check's two Core fixtures
(docs/SIGNER-CHECK.md §9.1).

    python3 -I -B tools/scripts/signer-core.py [<out directory>]

Starts Bitcoin Core (the pinned release, regtest, no network, scratch
datadir: `tools/crosscheck/core_node.py`), makes two wallets that hold a
test key each, and has `walletprocesspsbt` sign the committed PSBTs:

- `signer-core-signed.psbt`: `wallet-2of3.psbt` signed by a wallet that
  holds the "zoo … wrong" key (one of the 2-of-3), not finalized;
- `signer-core-final.psbt`: `demo-regtest.psbt` signed and finalized by a
  wallet that holds the "abandon … about" key.

Core is given no coins and no chain: it reads the amounts and scripts the
PSBTs carry. Both keys are the repository's public test seeds. The two
files are inputs of `core/faraday-psbt/examples/signer.rs`, not rebuilt
by it; a rerun can differ from the committed files only where Core's
signing does (RFC 6979 nonces with low R, so it does not).
"""

import hashlib
import hmac
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))
import core_node  # noqa: E402

VECTORS = ROOT / "tools" / "vectors" / "psbt"
ABANDON = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
ZOO = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
TPRV = bytes.fromhex("04358394")
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b58check(payload):
    data = payload + hashlib.sha256(hashlib.sha256(payload).digest()).digest()[:4]
    n = int.from_bytes(data, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return "1" * (len(data) - len(data.lstrip(b"\0"))) + out


def master_tprv(words):
    """The regtest master private key of a BIP-39 seed with no passphrase."""
    seed = hashlib.pbkdf2_hmac("sha512", words.encode(), b"mnemonic", 2048)
    i = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    return b58check(TPRV + bytes(9) + i[32:] + b"\0" + i[:32])


def wallet(node, name, descriptors):
    node.createwallet(name, False, True, "", False, True, False)
    client = core_node._wallet_client(name)
    for desc in descriptors:
        checksum = desc + "#" + node.getdescriptorinfo(desc)["checksum"]
        done = client.importdescriptors(
            [{"desc": checksum, "timestamp": "now", "active": True, "range": [0, 20]}])
        if not done[0].get("success"):
            sys.exit(f"Core refused {desc}: {done}")
    return client


def main():
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    out.mkdir(parents=True, exist_ok=True)
    abandon, zoo = master_tprv(ABANDON), master_tprv(ZOO)
    policy = (VECTORS / "wallet-2of3.policy").read_text().split("\n")
    keys = [line.split("]", 1)[1] for line in policy[1:4]]
    with core_node.Node(wallet=True) as node:
        import os
        os.environ.update(node.environ())
        core = core_node.rpc()
        print("Core", node.info["version"], node.info["sha256"])
        # The 2-of-3 with the zoo key private, the other two as the policy's xpubs.
        multi = (f"wsh(sortedmulti(2,[73c5da0a/48h/1h/0h/2h]{keys[0]}/<0;1>/*,"
                 f"{zoo}/48h/1h/0h/2h/<0;1>/*,[b8688df1/48h/1h/0h/2h]{keys[2]}/<0;1>/*))")
        holder = wallet(core, "zoo", [multi])
        sent = (VECTORS / "wallet-2of3.psbt").read_text().strip()
        got = holder.walletprocesspsbt(sent, True, None, True, False)
        (out / "signer-core-signed.psbt").write_text(got["psbt"])
        print("signer-core-signed: complete", got["complete"])
        # The "abandon" key for the single-signature spend, every script type.
        wallet(core, "abandon", [f"{kind}({abandon}/{path}h/1h/0h/<0;1>/*)"
                                 for kind, path in (("wpkh", 84), ("tr", 86), ("pkh", 44))])
        demo = (VECTORS / "demo-regtest.psbt").read_text().strip()
        got = core_node._wallet_client("abandon").walletprocesspsbt(demo, True, None, True, True)
        (out / "signer-core-final.psbt").write_text(got["psbt"])
        print("signer-core-final: complete", got["complete"])
        print(json.dumps(core.decodepsbt(got["psbt"])["inputs"][0].get("final_scriptwitness")))


main()
