"""A Bitcoin Core descriptor wallet's wallet.dat, read whole by Faraday
(`faraday_core::core_wallet::read_file`, F229), against Bitcoin Core
itself: the pinned `bitcoind` on regtest, offline (`core_node.py`).

- The three wallet.dat files Core made for the vectors
  (`tools/vectors/core-wallet/`, plain, encrypted with the passphrase
  `faraday`, and migrated by `migratewallet` from a legacy wallet) are read
  whole and compared with what Core said of them when it made them
  (`listdescriptors` with private keys, `getwalletinfo`); the migrated one's
  `combo()` descriptors are listed as kept, one for each Core lists.
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
`KNOWN` and `KNOWN_SHOWN` below; each is made in `directed()` and asked of Core.
"""

import http.client
import json
import os
import pathlib
import shutil
import sqlite3

import core_node
from common import ROOT, Area, Mismatch, reference

VECTORS = ROOT / "tools" / "vectors" / "core-wallet"
PASSPHRASE = "faraday"
SHAPES = ("plain", "encrypted", "watchonly", "blank", "labels", "imported", "uncompressed",
          "multisig")
DISABLE_PRIVATE_KEYS = 1 << 32
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

# Where Core loads a file and Faraday does not call it whole, the record's
# type and the fault Faraday names, with why Core does not see it. Each is
# made in `directed()` and asked of Core.
KNOWN = {
    ("flags", "UnknownFlags"): "Core loads a wallet whose flags hold a bit set beyond the ones "
                               "it names; Faraday does not call such a wallet whole",
    ("file", "NotDescriptors"): "Core loads a wallet with no flags record and no descriptors "
                                "(a blank one whose flags record is gone) as an empty wallet; "
                                "Faraday refuses a file whose flags lack descriptors",
    ("walletdescriptor", "CacheMissing"): "Core derives the account key from the private key it "
                                          "holds where its cache record is gone; Faraday states "
                                          "only what the file holds",
    ("walletdescriptorlhcache", "CacheKey"): "Core lists the cached account key it loads "
                                             "without deriving it from the master key; "
                                             "Faraday checks it",
    ("walletdescriptor", "CacheKey"): "Core lists the cached account key it loads without "
                                      "comparing its depth, child number or parent fingerprint "
                                      "with the path beside it; Faraday refuses the descriptor",
    ("walletdescriptorlhcache", "CacheUnchecked"): "Core loads a private-key wallet whose "
                                                   "descriptor lacks its master key's record; "
                                                   "Faraday has no key to check the cached "
                                                   "account key with and does not call it whole",
    ("walletdescriptorkey", "NotHeld"): "Core loads a file with a private key record whose "
                                        "descriptor id no descriptor record has (a descriptor "
                                        "record gone) and leaves the key unused; Faraday does "
                                        "not call whole a file holding a key it cannot tie to "
                                        "a descriptor to check it against",
    ("walletdescriptorlhcache", "NotHeld"): "Core loads a file with a cached account key whose "
                                            "descriptor id no descriptor record has and leaves "
                                            "it unused; Faraday refuses a record naming a "
                                            "descriptor the file does not hold",
    ("walletdescriptorlhcache", "Layout"): "Core loads a cache record whose public key does not "
                                           "start with 02 or 03, taking the key as invalid; "
                                           "Faraday refuses a record that is not the 74-byte "
                                           "vector of a compressed key that it writes",
}
# Known only where Core's own reading shows it, as `shown_by_core` asks.
KNOWN_SHOWN = {
    ("purpose", "Layout"): "Core loads a purpose record whose value is not one whole UTF-8 "
                           "string (a byte that is not UTF-8, bytes left past the string), "
                           "warns of a nonstandard purpose and keeps the address with none; "
                           "Faraday refuses a record that is not the string Core writes. "
                           "Taken only where Core lists the record's address with the "
                           "purpose \"unknown\"",
}
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

# A byte changed in the last hash of a block locator: Core, as it attaches
# the wallet to its node's chain, refuses a wallet whose locator does not
# end in that chain's genesis block (a wallet of another chain; its
# -walletcrosschain lets it load). That is a check of the node's chain, not
# of the file: Faraday reads no chain, counts the locator's hashes and does
# not read them.
LOCATORS = ("bestblock_nomerkle", "bestblock")
CROSS_CHAIN = "Wallet files should not be reused across chains"


def record_type(key_hex):
    """The type name a record's key starts with."""
    key = bytes.fromhex(key_hex)
    return key[1:1 + key[0]].decode("latin-1") if key else ""


def purpose_address(failure):
    """The address a purpose record's failure names (its key's tail, a
    length byte and the address), or None if the tail is not all shown."""
    _, _, tail = failure.partition(" ")
    if tail.endswith("..."):
        return None
    try:
        raw = bytes.fromhex(tail)
    except ValueError:
        return None
    if not raw or raw[0] != len(raw) - 1:
        return None
    return raw[1:].decode("latin-1")


def shown_by_core(known, got, core):
    """Whether Core's own reading of the file shows the condition of each of
    Faraday's failures of the KNOWN_SHOWN entry `known`: for a purpose
    record, Core lists its address with the purpose "unknown"."""
    if known == ("purpose", "Layout"):
        book = core[2] if len(core) > 2 else {}
        addresses = [purpose_address(f[0]) for f in got[1]["failures"]
                     if (f[0].split(" ")[0], f[1]) == known]
        return bool(addresses) and all(a is not None and book.get(a) == "unknown"
                                       for a in addresses)
    return False


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


def multisig_descriptors(rpc, source):
    """Costs a few RPC calls: the descriptors of the `multisig` shape, made
    from `source`'s master key (a wallet of Core's with private keys):
    a 2-of-2 whose keys are one `[fingerprint/48h/1h]tprv.../0h/2h/0/*`, an
    origin and further hardened steps, and one bare master key with hardened
    steps; and a miniscript with a hash before a key. Each with its checksum."""
    import re
    master = source.gethdkeys({"private": True})[0]["xprv"]
    fp = re.search(r"\[([0-9a-f]{8})/", source.listdescriptors()["descriptors"][0]["desc"])[1]
    a = f"[{fp}/48h/1h]{master}/0h/2h/0/*"
    b = f"{master}/48h/1h/0h/2h/0/*"
    h = "ab" * 32
    texts = [f"wsh(multi(2,{a},{b}))", f"wsh(and_v(v:sha256({h}),pk({b})))"]
    return [t + "#" + rpc.getdescriptorinfo(t)["checksum"] for t in texts]


def core_listing(entries):
    """Core's `listdescriptors` as the view both sides are compared in:
    the text with its checksum, the time, which is active and of which
    chain, the last index of the range and the next, for a ranged one."""
    out = []
    for d in entries:
        ranged = "range" in d
        active = bool(d.get("active"))
        out.append((d["desc"], d["timestamp"], active,
                    d.get("internal") if active else None,
                    d["range"][1] if ranged else None,
                    d.get("next_index", d.get("next")) if ranged else None))
    return sorted(out, key=repr)


def faraday_listing(descriptors):
    out = []
    for d in descriptors:
        ranged = d["range"] is not None
        out.append((d["listed"], d["timestamp"], d["active"],
                    d["internal"] if d["active"] else None,
                    d["range"][1] if ranged else None,
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
        "the three wallet.dat files Core made (`tools/vectors/core-wallet/`), plain, "
        "encrypted and migrated from a legacy wallet by `migratewallet` (made with the older "
        "release of `tools/reference/core/legacy.pin`), with what Core said of them"
    )
    corpus = (
        "fresh wallets of eight shapes (plain, encrypted, watch-only, blank, with labels, "
        "with an imported descriptor, with an imported uncompressed key, with imported "
        "multisig and miniscript descriptors of keys with an origin and further hardened "
        "steps, a bare master key with hardened steps, and a hash before a key), with and without "
        "a transaction, each read with the right passphrase, a wrong one and none; and the "
        "same files with a byte flipped in a record, a record dropped or pages cut off"
    )
    differences = [
        (
            "Faraday gives each descriptor as Core lists it (the account key from the "
            "`walletdescriptorlhcache` records, the master key's fingerprint and the hardened "
            "path as the origin, checksum included), and the area compares that whole text, "
            "with Core's `listdescriptors` without private keys, for every shape."
        ),
        (
            "Faraday checks what Core does not look at when it loads a wallet: a private "
            "key's stored hash and public key, a descriptor's id and checksum, and flags and "
            "records Core's loader ignores. A changed file Core loads and Faraday does not "
            "call whole is counted, and must name a record and fault of the listed set; "
            "nothing is asked of Core there beyond that it loads, except for a purpose record "
            "Faraday cannot read, where Core must list the address's purpose as unknown."
        ),
        (
            "Core runs SQLite's integrity check when it loads a wallet and refuses a file "
            "in which a record's key was changed (the unique index beside the table still "
            "holds the old key), and refuses one whose `walletdescriptorcache` record was "
            "dropped; Faraday reads the table only, so it reads such a file whole when the "
            "changed key is still a record it accepts. Core also refuses a wallet whose block "
            "locator ends in another chain's genesis block, as it attaches the wallet to its "
            "node's chain; Faraday reads no chain and does not read the locator's hashes. "
            "Each is asked of Core in the prelude."
        ),
        (
            "Faraday refuses a wallet of another network than its mode and a file with a "
            "journal, log or lock beside it, which Core opens (and recovers); the area reads "
            "regtest wallets with nothing beside them, so neither arises."
        ),
    ]
    not_compared = (
        "labels and the block locator (counted by Faraday; Core's address book is asked "
        "only for a purpose record Faraday refuses), the records Faraday counts and does "
        "not read, and the transactions themselves"
    )

    # ----- Core's side ------------------------------------------------------

    def folder(self):
        return pathlib.Path(json.loads(os.environ[core_node.ENV_RPC])["walletdir"])

    def make(self, shape, name, mine):
        """Costs a wallet on Core's node: the wallet of `shape`, unloaded,
        and what Core says of it. Returns (path of wallet.dat, facts)."""
        rpc = core_node.rpc()
        disable = shape == "watchonly"
        blank = shape in ("blank", "watchonly", "multisig")
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
        if shape == "multisig":
            src = name + "-src"
            rpc.createwallet(src, False, False, "", False, True, False)
            texts = multisig_descriptors(rpc, core_node._wallet_client(src))
            done = w.importdescriptors([
                {"desc": t, "timestamp": "now", "active": False, "range": [0, 10]}
                for t in texts])
            assert all(r["success"] for r in done), done
            core_node.drop_wallet(src)
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

    def loads(self, data, name, book=False):
        """Costs a bitcoind start and a wallet load: whether `loadwallet`
        opens `data` as a wallet, and what it lists then. The node is one
        of its own for each file (Core can stop on a damaged file, and the
        run's node is shared); one that stops is the answer "stopped".
        With `book`, a third item: each labelled address's purpose as
        `getaddressesbylabel` gives it ("unknown" where Core holds none)."""
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
            if not book:
                return ("ok", listed)
            w = client(node, f"/wallet/{name}")
            purposes = {}
            try:
                for label in w.listlabels():
                    for address, entry in w.getaddressesbylabel(label).items():
                        purposes[address] = entry["purpose"]
            except Exception:  # noqa: BLE001 - nothing listed shows no condition
                purposes = {}
            return ("ok", listed, purposes)

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

        def listing(v):
            return faraday_listing(v["descriptors"])

        want_list = core_listing(facts["pub"])
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
        # The third was migrated by Core from a legacy wallet: its `combo()`
        # descriptors are listed as kept by Faraday, one for each Core lists,
        # and the others are the descriptors read.
        for name in ("plain", "encrypted", "migrated"):
            data = (VECTORS / f"{name}.dat").read_bytes()
            core = json.loads((VECTORS / f"{name}.json").read_text())
            encrypted = "unlocked_until" in core["getwalletinfo"]
            got = self.rust.call("core_wallet.read", db=data.hex(),
                                 **({"passphrase": PASSPHRASE} if encrypted else {}))
            self.compared += 1
            entries = core["listdescriptors_public"]["descriptors"]
            combos = [d for d in entries if d["desc"].startswith("combo(")]
            want_list = core_listing([d for d in entries if d not in combos])
            inputs = {"vector": name}
            if got[0] != "ok":
                raise Mismatch(self.name, "core_wallet.read", inputs, got, want_list)
            v = got[1]
            info = core["getwalletinfo"]
            if sorted(v["kept"]) != sorted(d["desc"] for d in combos):
                raise Mismatch(self.name, "core_wallet.read", inputs, v["kept"],
                               [d["desc"] for d in combos],
                               "the combo descriptors kept differ from Core's")
            if (faraday_listing(v["descriptors"]) != want_list or not v["whole"]
                    or v["encrypted"] != encrypted or v["tx_count"] != info["txcount"]
                    or v["with_private_keys"] != len(want_list)
                    or v["flags"] & DISABLE_PRIVATE_KEYS):
                raise Mismatch(self.name, "core_wallet.read", inputs,
                               faraday_listing(v["descriptors"]), want_list,
                               "the vector wallet differs from what Core said of it")
        self.directed()

    def directed(self):
        """Each listed difference, made and asked of Core, still holds."""
        self.directed_index()
        self.directed_known()
        self.directed_outside_records()
        self.directed_cross_chain()
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

    def directed_known(self):
        """Each entry of KNOWN, made in a plain wallet Core made and asked
        of Core: Core loads the file and Faraday names the record and the
        fault and does not call it whole."""
        facts = self.make("plain", "k-known", mine=False)
        con = sqlite3.connect(":memory:", isolation_level=None)
        con.deserialize(facts["file"])

        def keys(record):
            return [bytes(k) for (k,) in con.execute("SELECT key FROM main ORDER BY rowid")
                    if record_type(k.hex()) == record]

        def value(key):
            return bytes(con.execute("SELECT value FROM main WHERE key = ?", (key,)).fetchone()[0])

        def edit(key, new):
            con.execute("UPDATE main SET value = ? WHERE key = ?", (new, key))

        def flipped(raw, at):
            raw = bytearray(raw)
            raw[at] ^= 1
            return bytes(raw)

        def stray(key):
            # The id follows the record's name: one length byte, then the name.
            at = 1 + key[0]
            con.execute("INSERT INTO main (key, value) VALUES (?, ?)",
                        (flipped(key, at), value(key)))

        original = con.serialize()
        flags = keys("flags")[0]
        cache = keys("walletdescriptorlhcache")[0]
        cases = [
            (("flags", "UnknownFlags"), lambda: edit(
                flags, (int.from_bytes(value(flags), "little") | 1 << 5).to_bytes(8, "little"))),
            (("walletdescriptor", "CacheMissing"), lambda: con.execute(
                "DELETE FROM main WHERE key = ?", (cache,))),
            # The value is a compact size, then the 74 bytes: the chain code
            # starts at 10, the public key's prefix at 42, the depth at 1.
            (("walletdescriptorlhcache", "CacheKey"), lambda: edit(
                cache, flipped(value(cache), 20))),
            (("walletdescriptor", "CacheKey"), lambda: edit(
                cache, flipped(value(cache), 1))),
            (("walletdescriptorlhcache", "Layout"), lambda: edit(
                cache, value(cache)[:42] + b"\x05" + value(cache)[43:])),
            (("walletdescriptorlhcache", "CacheUnchecked"), lambda: [
                con.execute("DELETE FROM main WHERE key = ?", (k,))
                for k in keys("walletdescriptorkey")]),
            # A copy of the key's and the cache's records under an id no
            # descriptor has (the descriptor's own record gone, as with a
            # descriptor that is not active).
            (("walletdescriptorkey", "NotHeld"), lambda: stray(keys("walletdescriptorkey")[0])),
            (("walletdescriptorlhcache", "NotHeld"), lambda: stray(cache)),
        ]
        blank = self.make("blank", "k-known-blank", mine=False)["file"]
        cases.append((("file", "NotDescriptors"), lambda: (
            con.deserialize(blank),
            con.execute("DELETE FROM main WHERE key = ?", (keys("flags")[0],)))))
        for known, make_change in cases:
            con.deserialize(original)
            make_change()
            changed = con.serialize()
            got = self.rust.call("core_wallet.read", db=changed.hex())
            core = self.loads(changed, f"k-known-{known[1]}-{known[0]}")
            said = ({(f[0].split(" ")[0], f[1]) for f in got[1]["failures"]}
                    if got[0] == "ok" else {("file", got[1].split("(")[0])})
            self.known += 1
            if not ((got[0] == "err" or not got[1]["whole"]) and known in said
                    and core[0] == "ok"):
                raise Mismatch(self.name, "core_wallet.read", {"changed": known},
                               [got[0], sorted(said)], core[0],
                               "the listed difference no longer holds: Core loads the file "
                               "and Faraday names the record and fault in KNOWN")
        # A purpose record ("receive") changed in a wallet with labels: a
        # byte that is not UTF-8, and the length one short of the string.
        con.deserialize(self.make("labels", "k-known-labels", mine=False)["file"])
        labelled = con.serialize()
        purpose = keys("purpose")[0]
        text = value(purpose)
        for what, new in (("not UTF-8", text[:1] + bytes([text[1] | 0x80]) + text[2:]),
                          ("bytes left over", bytes([text[0] - 1]) + text[1:])):
            con.deserialize(labelled)
            edit(purpose, new)
            changed = con.serialize()
            got = self.rust.call("core_wallet.read", db=changed.hex())
            core = self.loads(changed, f"k-known-purpose-{what.split()[0]}", book=True)
            said = ({(f[0].split(" ")[0], f[1]) for f in got[1]["failures"]}
                    if got[0] == "ok" else set())
            self.known += 1
            if not (got[0] == "ok" and not got[1]["whole"] and ("purpose", "Layout") in said
                    and core[0] == "ok" and shown_by_core(("purpose", "Layout"), got, core)):
                raise Mismatch(self.name, "core_wallet.read", {"changed": "purpose " + what},
                               [got[0], sorted(said)], [core[0], core[2:]],
                               "the listed difference no longer holds: Core loads a purpose "
                               "record that is not one whole UTF-8 string and lists its "
                               "address with the purpose \"unknown\", and Faraday names it "
                               "as a Layout fault")
        con.close()

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

    def directed_cross_chain(self):
        """A byte changed in the genesis hash that ends a wallet's block
        locator: Faraday reads the file whole, and Core refuses it as a
        wallet of another chain (`CROSS_CHAIN`)."""
        facts = self.make("plain", "k-cross-chain", mine=False)
        con = sqlite3.connect(":memory:", isolation_level=None)
        con.deserialize(facts["file"])
        rows = [(bytes(k), bytes(v)) for (k, v) in con.execute("SELECT key, value FROM main")
                if record_type(bytes(k).hex()) == LOCATORS[0]]
        # The version, the count, then the hashes: the last 32 bytes are
        # the genesis block's.
        if not rows or len(rows[0][1]) < 4 + 1 + 32:
            con.close()
            raise Mismatch(self.name, "core_wallet.read", {}, None, None,
                           "no block locator with a hash to change")
        key, value = rows[0]
        con.execute("UPDATE main SET value = ? WHERE key = ?",
                    (value[:-1] + bytes([value[-1] ^ 1]), key))
        changed = con.serialize()
        con.close()
        got = self.rust.call("core_wallet.read", db=changed.hex())
        core = self.loads(changed, "k-cross-chain-d")
        self.known += 1
        if not (got[0] == "ok" and got[1]["whole"] and core[0] == "err"
                and CROSS_CHAIN in core[1]):
            raise Mismatch(self.name, "core_wallet.read", {"changed": "locator's genesis hash"},
                           got[0], core, "the listed difference no longer holds: Core refuses "
                           "a wallet whose locator ends in another genesis block, and Faraday "
                           "does not read the locator's hashes")

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
        book = got[0] == "ok" and any((f[0].split(" ")[0], f[1]) in KNOWN_SHOWN
                                      for f in got[1]["failures"])
        core = self.loads(changed, name, book=book)
        inputs = {"shape": facts["shape"], "db": changed.hex(), **desc}
        whole = got[0] == "ok" and got[1]["whole"]
        if whole:
            if (core[0] != "ok" and desc.get("kind") in ("drop", "flip")
                    and record_type(desc["record"]) in LENIENT):
                self.known += 1
                return
            if core[0] == "err" and desc.get("kind") == "flip" and INDEX in core[1]:
                # The flipped bytes are a copy of a key (an id also sits in
                # other records' values), found in the table, not the index.
                self.known += 1
                return
            if (core[0] == "err" and desc.get("kind") == "flip"
                    and record_type(desc["record"]) in LOCATORS and CROSS_CHAIN in core[1]):
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

            if (loose(faraday_listing(got[1]["descriptors"]))
                    != loose(core_listing(core[1]))):
                raise Mismatch(self.name, "core_wallet.read", inputs,
                               faraday_listing(got[1]["descriptors"]),
                               core_listing(core[1]),
                               "Faraday and Core list different descriptors")
        elif core[0] == "ok":
            # Core loads it and Faraday does not call it whole.
            if got[0] == "ok":
                said = sorted({(f[0].split(" ")[0], f[1]) for f in got[1]["failures"]})
                if (got[1]["passphrase"] == "wrong"):
                    said = [("mkey", "Passphrase")]
                if got[1].get("failures_more", 0) > 0:
                    # Failures past the list are a difference of their own.
                    said.append(("more", "failures past the list"))
            else:
                said = [("file", got[1].split("(")[0])]
            for one in said:
                if one in KNOWN_SHOWN and shown_by_core(one, got, core):
                    continue
                if one not in KNOWN:
                    raise Mismatch(self.name, "core_wallet.read", inputs, one, core[0],
                                   "Core loads a file Faraday does not call whole, and the "
                                   "difference is not listed in KNOWN")
            self.known += 1
