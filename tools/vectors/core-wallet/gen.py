"""Bitcoin Core's descriptor wallets, made by Bitcoin Core (F229).

Runs the pinned bitcoind of `tools/crosscheck/core_node.py` on regtest with
no network, makes nine descriptor wallets (a migrated one, below, and plain, encrypted with the
passphrase `faraday`, watch-only, blank, with labels, with an imported
descriptor, with an imported uncompressed key, with an imported multisig and a miniscript
descriptor of master keys with hardened steps), and writes for each, beside this file:

- `<name>.json`: the wallet's SQLite application id, every (key, value) of
  its table `main` as hex, Core's `listdescriptors` (with private keys
  where the wallet has them), `listdescriptors_public` (without them, the
  descriptors with their origins and the account keys) and `getwalletinfo`;
- `<name>.dat`: the wallet.dat itself, for the plain, the encrypted, the
  multisig, the watch-only and the migrated wallet (the SQLite reader, the Read a Bitcoin Core wallet tests and the
  Practice sample use them).

The migrated wallet is made in two steps: the older bitcoind of
`tools/reference/core/legacy.pin` (fetched once with `fetch_core.py --legacy`)
makes a legacy Berkeley DB wallet with keys of each kind, and the pinned
bitcoind's `migratewallet` turns it into a descriptor wallet, which holds
`combo()` descriptors for the imported keys. Only the migrated wallet's files
(`migrated.json`, `migrated.dat`) come of it.

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
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))
sys.path.insert(0, str(ROOT / "tools" / "reference" / "core"))

import core_node  # noqa: E402
from area_core_wallet import multisig_descriptors  # noqa: E402

PASSPHRASE = "faraday"
KEEP_DAT = ("plain", "encrypted", "watchonly", "multisig", "migrated")
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


def legacy_wallet(into):
    """Costs a node start and some blocks: a legacy Berkeley DB wallet,
    made by the older release, copied into `into`/migrated/. It has an HD
    chain, an address of each kind the legacy wallet makes, a change
    address, and an imported private key of each of two kinds."""
    release = core_node.pin(core_node.LEGACY_PIN)
    with core_node.Node(wallet=True, release=release,
                        args=["-deprecatedrpc=create_bdb", "-keypool=10"]) as node:
        os.environ.update(node.environ())
        core_node.forget_rpc()
        rpc = core_node.rpc()
        walletdir = pathlib.Path(json.loads(os.environ[core_node.ENV_RPC])["walletdir"])
        rpc.createwallet("migrated", False, False, "", False, False, False)
        client = core_node._wallet_client("migrated")
        addr = client.getnewaddress("", "bech32")
        for kind in ("legacy", "p2sh-segwit", "bech32"):
            client.getnewaddress("", kind)
        client.getrawchangeaddress()
        rpc.generatetoaddress(1, addr)
        # An imported key, and the key of an address of another wallet's:
        # the legacy wallet keeps these outside its HD chain.
        rpc.createwallet("other", False, False, "", False, False, False)
        donor = core_node._wallet_client("other")
        for kind in ("legacy", "bech32"):
            client.importprivkey(donor.dumpprivkey(donor.getnewaddress("", kind)))
        client.importprivkey(wif_uncompressed(os.urandom(32)))
        rpc.unloadwallet("migrated")
        rpc.unloadwallet("other")
        shutil.copytree(walletdir / "migrated", into / "migrated")
    core_node.forget_rpc()


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
    only = sys.argv[1:]
    legacy = pathlib.Path(tempfile.mkdtemp(prefix="faraday-legacy-"))
    try:
        if not only or "migrated" in only:
            legacy_wallet(legacy)
        run(legacy, only)
    finally:
        shutil.rmtree(legacy, ignore_errors=True)


def run(legacy, only):
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
        multisig = make("multisig", False, True, "", False, True, False)

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

        # A multisig and a miniscript over the master key of a source wallet,
        # whose keys carry an origin with further hardened steps, and none.
        source = make("source", False, False, "", False, True, False)
        done = multisig.importdescriptors([
            {"desc": t, "timestamp": "now", "active": False, "range": [0, 20]}
            for t in multisig_descriptors(rpc, source)])
        assert all(r["success"] for r in done), done

        # The legacy wallet, migrated by this release: its own folder in the
        # wallet directory, then `migratewallet`.
        if (legacy / "migrated").is_dir():
            shutil.copytree(legacy / "migrated", pathlib.Path(walletdir) / "migrated")
            moved = rpc.migratewallet("migrated")
            print("migrated:", moved["wallet_name"], moved.get("watchonly_name"),
                  moved.get("solvables_name"))
            wallets["migrated"] = core_node._wallet_client("migrated")

        infos = {}
        for name, client in wallets.items():
            if name == "encrypted":
                client.walletpassphrase(PASSPHRASE, 600)
            lst = client.listdescriptors(name not in ("watchonly",))
            infos[name] = {"listdescriptors": lst,
                           "listdescriptors_public": client.listdescriptors(False),
                           "getwalletinfo": client.getwalletinfo()}
            if name == "encrypted":
                client.walletlock()
        for name in wallets:
            rpc.unloadwallet(name)
        for name in wallets:
            if (only and name not in only) or name == "source":
                continue
            dump(walletdir, name, infos[name])
            print(name, "ok")


if __name__ == "__main__":
    main()
