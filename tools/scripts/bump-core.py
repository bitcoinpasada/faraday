"""The Bitcoin Core fee bump fixtures (docs/COMPARE.md §6.1).

    python3 -I -B tools/scripts/bump-core.py [<out directory>]

Starts Bitcoin Core (the pinned release, regtest, no network, scratch
datadir: `tools/crosscheck/core_node.py`) with a wallet that holds the
"abandon … about" key at BIP-84, and writes:

- `bump-core-original.psbt`: one payment with change, as
  `walletcreatefundedpsbt` made it (replace-by-fee on), unsigned, with the
  change output's key origin;
- `bump-core-original.tx`: that payment signed, finalized and broadcast, as
  hex;
- `bump-core-replacement.psbt`: `psbtbumpfee`'s replacement of it, at a rate
  the change can pay;
- `bump-core-added.psbt`: `psbtbumpfee` at a rate the change cannot pay, so
  Core adds an input of the wallet's.

The wallet is funded by mining to it (the block times are fixed by
`setmocktime`), the spent coin, the amount, the change position and the
fee rates are fixed, and Core signs with RFC 6979 nonces, so the original
and its transaction are the same on every run. `psbtbumpfee` does not keep
the original's locktime: it sets the anti-fee-sniping one, the chain height
(102) or, one run in ten, up to 99 blocks lower, so a replacement's bytes
differ between runs in that field only. The key is the repository's public
test seed.
"""

import hashlib
import hmac
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))
import core_node  # noqa: E402

ABANDON = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
ZOO = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
TPRV = bytes.fromhex("04358394")
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
MOCKTIME = 1_700_000_000
PAY_SAT = 4_999_990_000   # of a 50 BTC coin: the change is a little under 10 000 sat
ORIGINAL_RATE = 10        # sat/vB
REPLACEMENT_RATE = 20     # sat/vB: the change pays it
ADDED_RATE = 100          # sat/vB: the change cannot pay it


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


def describe(tx):
    return {"locktime": tx["locktime"], "sequences": [i["sequence"] for i in tx["vin"]],
            "inputs": len(tx["vin"]), "outputs": len(tx["vout"])}


def main():
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    out.mkdir(parents=True, exist_ok=True)
    with core_node.Node(wallet=True) as node:
        os.environ.update(node.environ())
        core = core_node.rpc()
        print("Core", node.info["version"], node.info["sha256"])
        core.setmocktime(MOCKTIME)
        core.createwallet("bump", False, True, "", False, True, False)
        wallet = core_node._wallet_client("bump")
        for chain in (0, 1):
            desc = f"wpkh({master_tprv(ABANDON)}/84h/1h/0h/{chain}/*)"
            desc += "#" + core.getdescriptorinfo(desc)["checksum"]
            done = wallet.importdescriptors([{"desc": desc, "timestamp": "now", "active": True,
                                              "internal": chain == 1, "range": [0, 20]}])
            if not done[0].get("success"):
                sys.exit(f"Core refused the descriptor: {done}")
        # Two coinbase coins mature: the payment spends the first, and the
        # second is what a bump adds.
        mine_to = wallet.getnewaddress("", "bech32")
        core.generatetoaddress(102, mine_to)
        coins = sorted(wallet.listunspent(1), key=lambda u: u["confirmations"], reverse=True)
        if len(coins) < 2:
            sys.exit("the wallet has fewer than two mature coins")
        first = coins[0]
        pay_to = core.deriveaddresses(
            f"wpkh({master_tprv(ZOO)}/84h/1h/0h/0/0)#"
            + core.getdescriptorinfo(f"wpkh({master_tprv(ZOO)}/84h/1h/0h/0/0)")["checksum"])[0]
        funded = wallet.walletcreatefundedpsbt(
            [{"txid": first["txid"], "vout": first["vout"]}],
            [{pay_to: round(PAY_SAT / 1e8, 8)}], 0,
            {"replaceable": True, "add_inputs": False, "changePosition": 1,
             "fee_rate": ORIGINAL_RATE}, True)
        original = funded["psbt"]
        signed = wallet.walletprocesspsbt(original, True, "ALL", True, True)
        if not signed["complete"]:
            sys.exit("Core did not finalize the payment")
        raw = core.finalizepsbt(signed["psbt"], True)["hex"]
        core.sendrawtransaction(raw)
        txid = core.decoderawtransaction(raw)["txid"]
        (out / "bump-core-original.psbt").write_text(original + "\n")
        (out / "bump-core-original.tx").write_text(raw + "\n")
        print("original:", describe(core.decoderawtransaction(raw)), "fee", funded["fee"])
        for name, rate in (("replacement", REPLACEMENT_RATE), ("added", ADDED_RATE)):
            bumped = wallet.psbtbumpfee(txid, {"fee_rate": rate})
            (out / f"bump-core-{name}.psbt").write_text(bumped["psbt"] + "\n")
            tx = core.decodepsbt(bumped["psbt"])["tx"]
            print(f"{name}:", describe(tx), "fee", bumped["origfee"], "->", bumped["fee"])


main()
