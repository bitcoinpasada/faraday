"""A Bitcoin Core descriptor wallet's wallet.dat, read whole by Faraday
(`faraday_core::core_wallet::read_file`, F229), against Bitcoin Core
itself: the pinned `bitcoind` on regtest, offline (`core_node.py`).

- The two wallet.dat files Core made for the vectors
  (`tools/vectors/core-wallet/`, plain and encrypted with the passphrase
  `faraday`) are read whole and compared with what Core said of them when
  it made them (`listdescriptors` with private keys, `getwalletinfo`).
- Wallets made fresh by Core on the run's node, of each shape (plain,
  encrypted, watch-only, blank, with labels, with an imported descriptor
  and its private keys, with an imported uncompressed key, each with or
  without a transaction), are unloaded, and the wallet.dat Core left is
  read by Faraday: its descriptors (text, creation time, range, next index,
  which are active and which of the change chain), whether it is encrypted,
  whether it has private keys, how many transactions it holds, all against
  Core's `listdescriptors` and `getwalletinfo` of the wallet before it was
  unloaded; with `listdescriptors true` for the private keys against
  Faraday's key check; and for an encrypted wallet, the right passphrase
  (Core's `walletpassphrase` opens it, Faraday opens the master key and
  checks each key), a wrong one (both refuse), and none (the descriptors
  still read).
- The same files with a byte flipped in a record, a record dropped, or whole
  pages cut off, against whether Core's `loadwallet` opens them: where Faraday
  calls the file whole, Core must load it and list the same descriptors; where
  Core refuses it, Faraday must not call it whole (except where Core's
  integrity check refuses the index beside `main`, which Faraday walks for its
  pages and cell extents and does not read: a changed key, an entry dropped or
  changed, an index emptied; see `INDEX`); where Core loads it and
  Faraday does not call it whole, the record and the fault must be one of the
  listed differences. The header's database version changed, and a page no
  tree reaches, are each refused by both.

Known differences, each checked to still hold and counted: see
`KNOWN` below; each is asked of Core directly in the prelude.
"""

import http.client
import json
import os
import pathlib
import re
import shutil
import sqlite3

import core_node
from common import ROOT, Area, Mismatch, reference

VECTORS = ROOT / "tools" / "vectors" / "core-wallet"
PASSPHRASE = "faraday"
SHAPES = ("plain", "encrypted", "watchonly", "blank", "labels", "imported", "uncompressed")
DISABLE_PRIVATE_KEYS = 1 << 32
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

# Where Core loads a file and Faraday does not call it whole, the record's
# type and the fault Faraday names, with why Core does not see it. Each is
# made in the prelude and asked of Core.
KNOWN = {}
# Where Faraday calls a file whole and Core's loadwallet refuses it, the
# record type changed (by a dropped record or a flipped byte) and why
# Faraday does not see it: Faraday counts these records and does not read
# them. Each is made in the prelude and asked of Core.
LENIENT = {
    "walletdescriptorcache": "Core keeps the derived parent keys of a descriptor here and "
                             "refuses a wallet whose cache is not the one its keys give",
}


# A byte changed in a record's key: Core runs SQLite's integrity check at
# loading, which finds the key in the table and not in the unique index
# beside it; Faraday does not read the index.
INDEX = "sqlite_autoindex_main_1"


def record_type(key_hex):
    """The type name a record's key starts with."""
    key = bytes.fromhex(key_hex)
    return key[1:1 + key[0]].decode("latin-1") if key else ""


def norm(text):
    """A descriptor as text without its checksum, hardened steps as `'`."""
    return re.sub(r"(\d)h(?=[/\])])", r"\1'", text.split("#")[0])


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


def kind(text):
    """The script functions of a descriptor, outermost first: `sh,wpkh`."""
    return ",".join(re.findall(r"[a-z_]+(?=\()", text.split("#")[0]))


def shown(text, whole):
    """What is compared of a descriptor's text: the whole of it where Core
    keeps what was imported, else its script functions (Core lists a
    descriptor of its own master key with the key at the account and its
    origin, which Faraday does not rewrite)."""
    return norm(text) if whole else kind(text)


def core_listing(entries, with_text):
    """Core's `listdescriptors` as the view both sides are compared in; with
    not `with_text`, the script functions only."""
    out = []
    for d in entries:
        ranged = "range" in d
        active = bool(d.get("active"))
        out.append((shown(d["desc"], with_text), d["timestamp"], active,
                    d.get("internal") if active else None,
                    d["range"][1] if ranged else None,
                    d.get("next_index", d.get("next")) if ranged else None))
    return sorted(out, key=repr)


def faraday_listing(descriptors, with_text):
    out = []
    for d in descriptors:
        ranged = d["range_end"] > 0
        out.append((shown(d["text"], with_text), d["timestamp"], d["active"],
                    d["internal"] if d["active"] else None,
                    d["range_end"] - 1 if ranged else None,
                    d["next_index"] if ranged else None))
    return sorted(out, key=repr)


class CoreWallet(Area):
    name = "core_wallet"
    CORE = True
    WALLET = True
    LOOPS = (("fresh", 14), ("mutation", 24))
    faraday = (
        "`faraday_core::core_wallet::read_file` (a wallet.dat read whole through "
        "`faraday_sqlite`, the records read and checked, the private keys checked in "
        "`faraday-keys`)"
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest, offline: it makes each wallet "
        "(`createwallet`, `encryptwallet`, `importdescriptors`, `getnewaddress` with labels, "
        "`generatetoaddress`), says what it holds (`listdescriptors` and "
        "`listdescriptors true`, `getwalletinfo`), opens the passphrase "
        "(`walletpassphrase`) and loads or refuses a changed file (`loadwallet`)"
    )
    reference_checked = (
        "Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run and the node's version is "
        "asked; every wallet.dat is one Core wrote and every answer is one Core gave of the "
        "same wallet"
    )
    vectors = (
        "the two wallet.dat files Core made (`tools/vectors/core-wallet/`), plain and "
        "encrypted, with what Core said of them"
    )
    corpus = (
        "fresh wallets of seven shapes (plain, encrypted, watch-only, blank, with labels, "
        "with an imported descriptor, with an imported uncompressed key), with and without "
        "a transaction, each read with the right passphrase, a wrong one and none; and the "
        "same files with a byte flipped in a record, a record dropped or pages cut off"
    )
    differences = [
        (
            "Core lists a descriptor of its own master key with the key at the account and "
            "its origin; the file stores it with the master key and the path, which Faraday "
            "states as stored. The area compares the script functions of each descriptor "
            "(`sh(wpkh(...))`) and, for a watch-only wallet whose descriptors were imported "
            "as Core lists them, the whole text with hardened steps as `'`."
        ),
        (
            "Faraday checks what Core does not look at when it loads a wallet: a private "
            "key's stored hash and public key, a descriptor's id and checksum, and flags and "
            "records Core's loader ignores. A changed file Core loads and Faraday does not "
            "call whole is counted, and must name a record and fault of the listed set; "
            "nothing is asked of Core there beyond that it loads."
        ),
        (
            "Core runs SQLite's integrity check when it loads a wallet and refuses a file "
            "in which a record's key was changed (the unique index beside the table still "
            "holds the old key), and refuses one whose `walletdescriptorcache` record was "
            "dropped; Faraday reads the table only and counts the cache records without "
            "reading them, so it reads such a file whole when the changed key is still a "
            "record it accepts. Each is asked of Core in the prelude."
        ),
        (
            "Faraday refuses a wallet of another network than its mode and a file with a "
            "journal, log or lock beside it, which Core opens (and recovers); the area reads "
            "regtest wallets with nothing beside them, so neither arises."
        ),
    ]
    not_compared = (
        "labels and the block locator (counted by Faraday, listed by no Core call the area "
        "reads), the records Faraday counts and does not read, and the transactions "
        "themselves"
    )

    # ----- Core's side ------------------------------------------------------

    def folder(self):
        return pathlib.Path(json.loads(os.environ[core_node.ENV_RPC])["walletdir"])

    def make(self, shape, name, mine):
        """Costs a wallet on Core's node: the wallet of `shape`, unloaded,
        and what Core says of it. Returns (path of wallet.dat, facts)."""
        rpc = core_node.rpc()
        disable = shape == "watchonly"
        blank = shape in ("blank", "watchonly")
        rpc.createwallet(name, disable, blank, PASSPHRASE if shape == "encrypted" else "",
                         False, True, False)
        w = core_node._wallet_client(name)
        if shape in ("watchonly", "imported"):
            src = name + "-src"
            rpc.createwallet(src, False, False, "", False, True, False)
            sc = core_node._wallet_client(src)
            if shape == "imported":
                ext = next(d for d in sc.listdescriptors(True)["descriptors"]
                           if d["desc"].startswith("wpkh(") and not d["internal"])
                done = w.importdescriptors([
                    {"desc": ext["desc"], "timestamp": "now", "active": False,
                     "range": [0, 10]}])
            else:
                done = w.importdescriptors([
                    {"desc": d["desc"], "timestamp": "now", "active": True,
                     "internal": d["internal"], "range": [0, 10]}
                    for d in sc.listdescriptors()["descriptors"]
                    if d["desc"].startswith("wpkh(")])
            assert all(r["success"] for r in done), done
            core_node.drop_wallet(src)
        if shape == "uncompressed":
            text = f"pkh([d34db33f/44h/1h/0h]{wif_uncompressed(self.rng.randbytes(32))})"
            text += "#" + rpc.getdescriptorinfo(text)["checksum"]
            done = w.importdescriptors([{"desc": text, "timestamp": "now", "active": False}])
            assert done[0]["success"], done
        if shape == "labels":
            for label in ("savings", "rent"):
                w.getnewaddress(label)
            w.getnewaddress("", "bech32m")
        if mine and shape in ("plain", "labels", "encrypted"):
            if shape == "encrypted":
                w.walletpassphrase(PASSPHRASE, 600)
            rpc.generatetoaddress(1, w.getnewaddress())
            if shape == "encrypted":
                w.walletlock()
        facts = {"info": w.getwalletinfo(), "pub": w.listdescriptors()["descriptors"],
                 "shape": shape}
        if shape == "encrypted":
            facts["wrong"] = reference(lambda: w.walletpassphrase("not the passphrase", 1))
            w.walletpassphrase(PASSPHRASE, 600)
        # Unlocking tops the key pool up, so the listing is taken after it.
        facts["pub"] = w.listdescriptors()["descriptors"]
        facts["info"] = w.getwalletinfo()
        facts["priv"] = None if disable else w.listdescriptors(True)["descriptors"]
        if shape == "encrypted":
            w.walletlock()
        rpc.unloadwallet(name)
        path = self.folder() / name / "wallet.dat"
        facts["file"] = path.read_bytes()
        shutil.rmtree(path.parent, ignore_errors=True)
        return facts

    def loads(self, data, name):
        """Costs a bitcoind start and a wallet load: whether `loadwallet`
        opens `data` as a wallet, and what it lists then. The node is one
        of its own for each file (Core can stop on a damaged file, and the
        run's node is shared); one that stops is the answer "stopped"."""
        import sys
        folder = str(ROOT / "tools" / "reference" / "core")
        if folder not in sys.path:
            sys.path.insert(0, folder)
        from test_framework.authproxy import AuthServiceProxy

        def client(node, path=""):
            c = AuthServiceProxy(node.info["url"] + path, timeout=120)
            c.reuse_http_connections = False
            return c

        with core_node.Node(rpc_threads=4, wallet=True) as node:
            where = node.scratch / "data" / "regtest" / "wallets" / name
            where.mkdir(parents=True)
            (where / "wallet.dat").write_bytes(data)
            try:
                client(node).loadwallet(name)
            except (ConnectionError, OSError, http.client.HTTPException):
                return ("err", "stopped")
            except Exception as e:  # noqa: BLE001 - a refusal of Core's is the answer
                return ("err", str(e))
            try:
                listed = client(node, f"/wallet/{name}").listdescriptors()["descriptors"]
            except Exception as e:  # noqa: BLE001
                listed = ("err", str(e))
            return ("ok", listed)

    # ----- The comparison ---------------------------------------------------

    def judge(self, facts, data, label):
        """Faraday reads `data`, a wallet Core described as `facts`."""
        shape = facts["shape"]
        info = facts["info"]
        encrypted = "unlocked_until" in info
        keyed = facts["priv"] is not None
        pub_texts = {d["desc"] for d in facts["pub"]}
        with_private = (sum(1 for d in facts["priv"] if d["desc"] not in pub_texts)
                        if keyed else 0)
        inputs = {"db": data.hex(), "shape": shape, "label": label}

        def ask(passphrase):
            got = self.rust.call("core_wallet.read", db=data.hex(),
                                 **({"passphrase": passphrase} if passphrase else {}))
            self.compared += 1
            return got

        def need(got, want, note):
            if got[0] != "ok":
                raise Mismatch(self.name, "core_wallet.read", inputs, got, want, note)
            return got[1]

        facts_of = {
            "encrypted": encrypted,
            "private_keys_enabled": info["private_keys_enabled"],
            "tx_count": info["txcount"],
            "format": info["format"],
            "descriptors": info["descriptors"],
        }

        def seen(v):
            return {
                "encrypted": v["encrypted"],
                "private_keys_enabled": not v["flags"] & DISABLE_PRIVATE_KEYS,
                "tx_count": v["tx_count"],
                "format": "sqlite",
                "descriptors": True,
            }

        whole_text = shape == "watchonly"

        def listing(v):
            return faraday_listing(v["descriptors"], whole_text)

        want_list = core_listing(facts["pub"], whole_text)
        # The right passphrase (or none needed).
        right = PASSPHRASE if encrypted else None
        v = need(ask(right), facts_of, "Faraday did not read a wallet Core made")
        if seen(v) != facts_of:
            raise Mismatch(self.name, "core_wallet.read", inputs, seen(v), facts_of,
                           "getwalletinfo")
        if listing(v) != want_list:
            raise Mismatch(self.name, "core_wallet.read", inputs, listing(v), want_list,
                           "listdescriptors")
        if not v["whole"] or v["failures"] or v["with_private_keys"] != with_private:
            raise Mismatch(self.name, "core_wallet.read", inputs,
                           [v["whole"], v["failures"], v["with_private_keys"]],
                           [True, [], with_private],
                           "the wallet is whole and has the private keys Core lists")
        if v["passphrase"] != ("opened" if encrypted else "not-needed") or v["keys_unchecked"]:
            raise Mismatch(self.name, "core_wallet.read", inputs, v["passphrase"],
                           "opened" if encrypted else "not-needed", "the keys are checked")
        if encrypted:
            # Wrong: Core's walletpassphrase refuses, so does Faraday.
            if facts["wrong"][0] != "err":
                raise Mismatch(self.name, "core_wallet.walletpassphrase", inputs, None,
                               facts["wrong"], "Core opened the wallet with a wrong passphrase")
            w = need(ask("not the passphrase"), "wrong", "wrong passphrase")
            if w["passphrase"] != "wrong" or w["whole"]:
                raise Mismatch(self.name, "core_wallet.read", inputs, w["passphrase"], "wrong",
                               "Faraday took a wrong passphrase")
            # None: the descriptors still read, the encrypted keys are counted.
            n = need(ask(None), "not-given", "no passphrase")
            if (n["passphrase"] != "not-given" or not n["whole"] or not n["keys_unchecked"]
                    or listing(n) != want_list):
                raise Mismatch(self.name, "core_wallet.read", inputs,
                               [n["passphrase"], n["whole"], n["keys_unchecked"]],
                               ["not-given", True, ">0"], "no passphrase")

    def prelude(self):
        # The two files Core made for the vectors, with what it said then.
        for name in ("plain", "encrypted"):
            data = (VECTORS / f"{name}.dat").read_bytes()
            core = json.loads((VECTORS / f"{name}.json").read_text())
            encrypted = "unlocked_until" in core["getwalletinfo"]
            got = self.rust.call("core_wallet.read", db=data.hex(),
                                 **({"passphrase": PASSPHRASE} if encrypted else {}))
            self.compared += 1
            want_list = core_listing(core["listdescriptors"]["descriptors"], False)
            inputs = {"vector": name}
            if got[0] != "ok":
                raise Mismatch(self.name, "core_wallet.read", inputs, got, want_list)
            v = got[1]
            info = core["getwalletinfo"]
            if (faraday_listing(v["descriptors"], False) != want_list or not v["whole"]
                    or v["encrypted"] != encrypted or v["tx_count"] != info["txcount"]
                    or v["with_private_keys"] != len(want_list)
                    or v["flags"] & DISABLE_PRIVATE_KEYS):
                raise Mismatch(self.name, "core_wallet.read", inputs,
                               faraday_listing(v["descriptors"], False), want_list,
                               "the vector wallet differs from what Core said of it")
        self.directed()

    def directed(self):
        """Each listed difference, made and asked of Core, still holds."""
        self.directed_index()
        self.directed_outside_records()
        for record, why in LENIENT.items():
            for shape in ("plain", "watchonly"):
                facts = self.make(shape, f"k-{record}-{shape}", mine=False)
                con = sqlite3.connect(":memory:", isolation_level=None)
                con.deserialize(facts["file"])
                rows = [k for (k,) in con.execute("SELECT key FROM main")
                        if record_type(bytes(k).hex()) == record]
                if not rows:
                    con.close()
                    continue
                con.execute("DELETE FROM main WHERE key = ?", (rows[0],))
                changed = con.serialize()
                con.close()
                got = self.rust.call("core_wallet.read", db=changed.hex())
                core = self.loads(changed, f"k-{record}-{shape}-d")
                self.known += 1
                if not (got[0] == "ok" and got[1]["whole"] and core[0] == "err"):
                    raise Mismatch(self.name, "core_wallet.read", {"dropped": record,
                                   "shape": shape}, got[0], core[0],
                                   f"the listed difference no longer holds: {why}")
                break

    def directed_index(self):
        """A byte changed in a record's key that Faraday reads as another
        record is refused by Core's integrity check, on the index."""
        facts = self.make("plain", "k-index", mine=False)
        data = facts["file"]
        con = sqlite3.connect(":memory:")
        con.deserialize(data)
        keys = [bytes(k) for (k,) in con.execute("SELECT key FROM main ORDER BY rowid")]
        con.close()
        for n, key in enumerate(keys):
            at = data.find(key)
            if at < 0:
                continue
            changed = bytearray(data)
            changed[at + len(key) - 1] ^= 1
            changed = bytes(changed)
            got = self.rust.call("core_wallet.read", db=changed.hex())
            if not (got[0] == "ok" and got[1]["whole"]):
                continue
            core = self.loads(changed, f"k-index-{n}")
            self.known += 1
            if core[0] != "err" or INDEX not in core[1]:
                raise Mismatch(self.name, "core_wallet.read", {"key": key.hex()}, "whole",
                               core, "the listed difference no longer holds: Core's "
                               "integrity check finds a changed key in the index")
            return
        raise Mismatch(self.name, "core_wallet.read", {}, None, None,
                       "no changed key was read whole by Faraday to ask Core of")

    def directed_outside_records(self):
        """A file changed outside a record: the header's database version,
        and a page no tree reaches. Core refuses each (`Verify`'s schema
        version, `integrity_check`'s "never used"), and so must Faraday."""
        facts = self.make("plain", "k-outside", mine=False)
        data = facts["file"]
        page = int.from_bytes(data[16:18], "big") or 65536
        page = 65536 if page == 1 else page
        version = bytearray(data)
        version[60:64] = (1).to_bytes(4, "big")
        orphan = bytearray(data) + bytes(page)
        orphan[28:32] = (len(orphan) // page).to_bytes(4, "big")
        for what, changed in (("user_version", bytes(version)), ("orphan page", bytes(orphan))):
            got = self.rust.call("core_wallet.read", db=changed.hex())
            core = self.loads(changed, f"k-outside-{what.split()[0]}")
            self.compared += 1
            if not (got[0] == "err" and core[0] == "err"):
                raise Mismatch(self.name, "core_wallet.read", {"changed": what},
                               got[0], core[0],
                               "Faraday and Core must both refuse a file changed outside its records")

    def case_fresh(self):
        rng = self.rng
        shape = rng.choice(SHAPES)
        name = f"f{self.case_at[1]}-{rng.randrange(1 << 30)}"
        facts = self.make(shape, name, mine=rng.randrange(2) == 0)
        self.judge(facts, facts["file"], "fresh")

    def case_mutation(self):
        rng = self.rng
        shape = rng.choice(SHAPES)
        stem = f"m{self.case_at[1]}-{rng.randrange(1 << 30)}"
        facts = self.make(shape, stem, mine=rng.randrange(2) == 0)
        data = facts["file"]
        page = int.from_bytes(data[16:18], "big") or 65536
        page = 65536 if page == 1 else page
        pages = len(data) // page
        con = sqlite3.connect(":memory:")
        con.deserialize(data)
        rows = con.execute("SELECT key, value FROM main ORDER BY rowid").fetchall()
        for n in range(3):
            kind = rng.choice(["flip", "drop", "cut"])
            if kind == "flip":
                key, value = rng.choice(rows)
                piece = rng.choice([bytes(key), bytes(value)])
                at = data.find(piece) if piece else -1
                if at < 0:
                    kind = "cut"
                else:
                    at += rng.randrange(len(piece))
                    changed = bytearray(data)
                    changed[at] ^= 1 << rng.randrange(8)
                    changed = bytes(changed)
                    desc = {"kind": kind, "record": bytes(key).hex(),
                            "in": "key" if piece == bytes(key) else "value"}
            if kind == "drop":
                key, _ = rng.choice(rows)
                db = sqlite3.connect(":memory:", isolation_level=None)
                db.deserialize(data)
                db.execute("DELETE FROM main WHERE key = ?", (key,))
                changed = db.serialize()
                db.close()
                desc = {"kind": kind, "record": bytes(key).hex()}
            if kind == "cut":
                keep = rng.randrange(1, pages)
                changed = data[: keep * page]
                desc = {"kind": kind, "pages_kept": keep}
            self.judge_changed(facts, changed, f"{stem}-{n}", desc)
        con.close()

    def judge_changed(self, facts, changed, name, desc):
        encrypted = "unlocked_until" in facts["info"]
        got = self.rust.call("core_wallet.read", db=changed.hex(),
                             **({"passphrase": PASSPHRASE} if encrypted else {}))
        self.compared += 1
        core = self.loads(changed, name)
        inputs = {"shape": facts["shape"], "db": changed.hex(), **desc}
        whole = got[0] == "ok" and got[1]["whole"]
        if whole:
            if (core[0] != "ok" and desc.get("kind") in ("drop", "flip")
                    and record_type(desc["record"]) in LENIENT):
                self.known += 1
                return
            if core[0] == "err" and desc.get("in") == "key" and INDEX in core[1]:
                self.known += 1
                return
            if core[0] != "ok":
                raise Mismatch(self.name, "core_wallet.read", inputs, "whole", core,
                               "Faraday called a file whole that Core's loadwallet refuses")
            if isinstance(core[1], tuple):
                return
            # Core tops the key pool up as it loads, so the range and next
            # index are not compared here.
            def loose(rows):
                return sorted(((r[0], r[1], r[2], r[3]) for r in rows), key=repr)

            if (loose(faraday_listing(got[1]["descriptors"], False))
                    != loose(core_listing(core[1], False))):
                raise Mismatch(self.name, "core_wallet.read", inputs,
                               faraday_listing(got[1]["descriptors"], False),
                               core_listing(core[1], False),
                               "Faraday and Core list different descriptors")
        elif core[0] == "ok":
            # Core loads it and Faraday does not call it whole.
            if got[0] == "ok":
                said = sorted({(f[0].split(" ")[0], f[1]) for f in got[1]["failures"]})
                if (got[1]["passphrase"] == "wrong"):
                    said = [("mkey", "Passphrase")]
            else:
                said = [("file", got[1].split("(")[0])]
            for one in said:
                if one not in KNOWN:
                    raise Mismatch(self.name, "core_wallet.read", inputs, one, core[0],
                                   "Core loads a file Faraday does not call whole, and the "
                                   "difference is not listed in KNOWN")
            self.known += 1
