"""Bitcoin Core's descriptor wallets, made by Bitcoin Core (F229).

Runs the pinned bitcoind of `tools/crosscheck/core_node.py` on regtest with
no network, makes seven descriptor wallets (plain, encrypted with the
passphrase `faraday`, watch-only, blank, with labels, with an imported
descriptor, with an imported uncompressed key), and writes for each, beside this file:

- `<name>.json`: the wallet's SQLite application id, every (key, value) of
  its table `main` as hex, Core's `listdescriptors` (with private keys
  where the wallet has them) and `getwalletinfo`;
- `<name>.dat`: the wallet.dat itself, for the plain and the encrypted
  wallet (the SQLite reader and the Practice sample use them).

Usage: python3 -I -B tools/vectors/core-wallet/gen.py [name ...]
With names, only those wallets' files are written (the others are made and
dropped), so one wallet can be added without changing the rest.
Standard library only. The keys are fresh each run, so a run changes every
file; the tests read what is committed.
"""

import json
import os
import pathlib
import shutil
import sqlite3
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))
sys.path.insert(0, str(ROOT / "tools" / "reference" / "core"))

import core_node  # noqa: E402

PASSPHRASE = "faraday"
KEEP_DAT = ("plain", "encrypted")
DAT_LIMIT = 200 * 1024


B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def wif_uncompressed(key):
    """A regtest WIF of `key` (32 bytes) for an uncompressed public key."""
    import hashlib
    raw = b"\xef" + key
    raw += hashlib.sha256(hashlib.sha256(raw).digest()).digest()[:4]
    n = int.from_bytes(raw, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = B58[r] + out
    return "1" * (len(raw) - len(raw.lstrip(b"\0"))) + out


def dump(walletdir, name, extra):
    path = pathlib.Path(walletdir) / name / "wallet.dat"
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    app_id = con.execute("PRAGMA application_id").fetchone()[0] & 0xFFFFFFFF
    rows = con.execute("SELECT key, value FROM main ORDER BY key").fetchall()
    con.close()
    doc = {"application_id": app_id,
           "records": [[k.hex(), v.hex()] for k, v in rows], **extra}
    (HERE / f"{name}.json").write_text(json.dumps(doc, indent=1) + "\n")
    if name in KEEP_DAT and path.stat().st_size < DAT_LIMIT:
        shutil.copyfile(path, HERE / f"{name}.dat")


def main():
    with core_node.Node(wallet=True) as node:
        os.environ.update(node.environ())
        walletdir = json.loads(os.environ[core_node.ENV_RPC])["walletdir"]
        rpc = core_node.rpc()
        wallets = {}

        def make(name, *args):
            rpc.createwallet(name, *args)
            client = core_node._wallet_client(name)
            wallets[name] = client
            return client

        # createwallet(name, disable_private_keys, blank, passphrase,
        # avoid_reuse, descriptors, load_on_startup)
        plain = make("plain", False, False, "", False, True, False)
        enc = make("encrypted", False, False, PASSPHRASE, False, True, False)
        blank = make("blank", False, True, "", False, True, False)
        watch = make("watchonly", True, True, "", False, True, False)
        labels = make("labels", False, False, "", False, True, False)
        imported = make("imported", False, False, "", False, True, False)
        uncompressed = make("uncompressed", False, False, "", False, True, False)

        # A coinbase in the plain wallet gives it a transaction record and a
        # block locator.
        addr = plain.getnewaddress()
        rpc.generatetoaddress(1, addr)
        for who, label in (("labels", "savings"), ("labels", "rent")):
            labels.getnewaddress(label)
        labels.getnewaddress("", "legacy")
        labels.getnewaddress("", "p2sh-segwit")
        labels.getnewaddress("", "bech32m")

        # A descriptor with its origin, from the plain wallet's own keys,
        # imported into another wallet: its private keys travel with it.
        pdescs = plain.listdescriptors(True)["descriptors"]
        ext = next(d for d in pdescs if d["desc"].startswith("wpkh(") and not d["internal"])
        done = imported.importdescriptors([
            {"desc": ext["desc"], "timestamp": "now", "active": False, "range": [0, 20]}])
        assert done[0]["success"], done
        # The public descriptors of the plain wallet, imported watch-only.
        pubs = plain.listdescriptors()["descriptors"]
        done = watch.importdescriptors([
            {"desc": d["desc"], "timestamp": "now", "active": True,
             "internal": d["internal"], "range": [0, 20]}
            for d in pubs if d["desc"].startswith("wpkh(")])
        assert all(r["success"] for r in done), done

        # A single uncompressed key (65-byte public key, 279-byte DER private
        # key), imported with an origin as a pkh() descriptor.
        wif = wif_uncompressed(os.urandom(32))
        text = f"pkh([d34db33f/44h/1h/0h]{wif})"
        text += "#" + rpc.getdescriptorinfo(text)["checksum"]
        done = uncompressed.importdescriptors([
            {"desc": text, "timestamp": "now", "active": False}])
        assert done[0]["success"], done

        infos = {}
        for name, client in wallets.items():
            if name == "encrypted":
                client.walletpassphrase(PASSPHRASE, 600)
            lst = client.listdescriptors(name not in ("watchonly",))
            infos[name] = {"listdescriptors": lst, "getwalletinfo": client.getwalletinfo()}
            if name == "encrypted":
                client.walletlock()
        for name in wallets:
            rpc.unloadwallet(name)
        only = sys.argv[1:]
        for name in wallets:
            if only and name not in only:
                continue
            dump(walletdir, name, infos[name])
            print(name, "ok")


if __name__ == "__main__":
    main()
