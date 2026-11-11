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
`KNOWN`, `KNOWN_SHOWN`, `LENIENT` and `SLOW` below; each is made in
`directed()` and asked of Core.

`sweep_core_wallet.py` makes every change the mutation loop can make to
every shape's wallet (each bit of each record's key and value, by position
class for long ones, each record dropped, each page cut) and judges each as
the loop does; it is run by hand after a change to the reader or to these
lists (its docstring says how), not in the cross-check.
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
    ("name", "Layout"): "Core loads a name record (an address's label) whose value is not one "
                        "whole UTF-8 string (walletdb.cpp `LoadAddressBookRecords` reads the "
                        "string and checks neither its text nor bytes left past it) and keeps "
                        "the label as read; Faraday refuses a record that is not the string "
                        "Core writes. Taken only where Core lists the address with a label "
                        "that is not UTF-8 or is short of the bytes the record holds",
    ("purpose", "Layout"): "Core loads a purpose record whose value is not one whole UTF-8 "
                           "string (a byte that is not UTF-8, bytes left past the string), "
                           "warns of a nonstandard purpose and keeps the address with none; "
                           "Faraday refuses a record that is not the string Core writes. "
                           "Taken only where Core lists the record's address with the "
                           "purpose \"unknown\"",
    ("bestblock", "Layout"): "Core reads the old block locator `bestblock` (normally empty) and, "
                             "where the record does not read as a locator (its count claims "
                             "hashes it lacks), falls back to `bestblock_nomerkle` "
                             "(walletdb.cpp `WalletBatch::ReadBestBlock`; db.h "
                             "`DatabaseBatch::Read` returns false on a failed read); Faraday "
                             "refuses a locator whose size is not its count's. Taken only where "
                             "Core shows the fallback: the file with the genesis hash of "
                             "`bestblock_nomerkle` changed is refused as another chain's",
    ("bestblock_nomerkle", "Layout"): "Core loads a wallet whose `bestblock_nomerkle` does not "
                                      "read as a locator (a count past its hashes, or short of "
                                      "them) as one with no locator, rescanning from genesis; "
                                      "Faraday refuses a locator whose size is not its count's. "
                                      "Taken only where Core shows it read no locator from the "
                                      "record: the file with the record's last byte changed "
                                      "loads (a locator read would end in another genesis)",
    ("walletdescriptor", "Range"): "Core loads a descriptor's range and next index as written, "
                                   "unchecked (walletdb.cpp `LoadDescriptorWalletRecords`), "
                                   "derives from the range's start to its end "
                                   "(scriptpubkeyman.cpp `SetCache`) and tops the range up to "
                                   "the next index and its key pool (`TopUpWithDB`, which also "
                                   "sets the range of a descriptor that is not ranged to 0 and "
                                   "1); Faraday refuses a negative next index or start, or an "
                                   "end before the start. Taken only where Core, loading the "
                                   "file on a node with -keypool=1, lists the descriptor "
                                   "Faraday does not with no range, a negative next index or "
                                   "start, a start past its end, or an end at the next index "
                                   "(the end it read was not past the next index)",
    ("walletdescriptorckey", "NoMasterKey"): "Core loads a wallet whose encrypted keys have "
                                             "no master key record, as encrypted and locked "
                                             "(walletdb.cpp `LoadDescriptorWalletRecords` "
                                             "does not decrypt); Faraday names each key it "
                                             "cannot open. Taken only where Core's "
                                             "walletpassphrase does not open the wallet with "
                                             "the passphrase",
    ("file", "File"): "SQLite takes the header's text encoding as its low two bits, a 0 as "
                      "UTF-8 (prepare.c `sqlite3InitOne`), so Core opens a file whose text "
                      "encoding is 5, 9 or 0; Faraday refuses any value but the three the file "
                      "format names. Taken only where Faraday's refusal is of the text encoding "
                      "and SQLite reads the field as UTF-8",
}
# Where Faraday calls a file whole and Core's loadwallet refuses it, the
# record type changed (by a dropped record or a flipped byte) and why
# Faraday does not see it: Faraday counts these records and does not read
# them. Each is made in the prelude and asked of Core.
LENIENT = {
    "walletdescriptorcache": "Core keeps the derived parent keys of a descriptor here and "
                             "refuses a wallet whose cache is not the one its keys give",
    "tx": "Core reads each transaction record as it loads (walletdb.cpp `LoadTxRecords`) and "
          "refuses a wallet one of which does not read (\"Wallet corrupted\"); Faraday counts "
          "them. Taken only on Core's refusal as corrupted",
}
# The message Core refuses a wallet with where a record does not read.
CORRUPTED = "Wallet corrupted"

# Core derives every index of a descriptor's range, and tops it up to the
# next index, as it loads the wallet (scriptpubkeyman.cpp `SetCache`,
# `TopUpWithDB`), about 30 microseconds an index on an idle machine (2^20
# in 31 s, 2^21 in 64 s) and many times that on a busy one. Faraday derives
# nothing as it reads and reads such a file whole up to the limit of 1,000,000
# on the size of a range and on the next index, past which it refuses;
# Core's answer is not had.
# Known only where Faraday lists a ranged descriptor whose next index or
# last index is at least SLOW and Core did not answer in SLOW_WAIT seconds
# or stopped; such a file is waited for SLOW_WAIT seconds.
SLOW = 1 << 16

# A descriptor that is not ranged whose stored range spans more than one
# index: Core derives each index of the range as it loads, gets the same
# script twice and refuses the wallet as corrupted (scriptpubkeyman.cpp
# `SetCache`, "Already loaded script"); Faraday refuses the record too
# (`Range`), so both refuse. A range of one index or none on such a
# descriptor is ignored and reset by Core, and Faraday lists no range. A
# range of 1,000,000 indexes or more, or a next index of 1,000,000 or more,
# is refused by Faraday (Core's `CheckDescriptorRangeBounds`, "Range is too
# large", is on the size of the range); a small range that starts high is
# read whole, as Core reads it.
SLOW_WAIT = 20
LOAD_WAIT = 120
TIMED_OUT = "took longer than"


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
# The records whose fault asks Core's address book (`wants_book`).
BOOK = (("purpose", "Layout"), ("name", "Layout"))
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


def faraday_said(got):
    """The (record, fault) pairs of Faraday's reading of a file it does not
    call whole: its failures, a wrong passphrase, failures past the list,
    or the refusal of the whole file."""
    if got[0] != "ok":
        return [("file", got[1].split("(")[0])]
    said = sorted({(f[0].split(" ")[0], f[1]) for f in got[1]["failures"]})
    if got[1]["passphrase"] == "wrong":
        said = [("mkey", "Passphrase")]
    if got[1].get("failures_more", 0) > 0:
        # Failures past the list are a difference of their own.
        said.append(("more", "failures past the list"))
    return said


def locator_end_changed(data):
    """`data` with the last byte of its `bestblock_nomerkle` record's value
    changed, or None where it has no such record of a hash or more."""
    con = sqlite3.connect(":memory:", isolation_level=None)
    con.deserialize(data)
    rows = [(bytes(k), bytes(v)) for (k, v) in con.execute("SELECT key, value FROM main")
            if record_type(bytes(k).hex()) == LOCATORS[0]]
    if len(rows) != 1 or len(rows[0][1]) < 4 + 1 + 32:
        con.close()
        return None
    key, value = rows[0]
    con.execute("UPDATE main SET value = ? WHERE key = ?", (value[:-1] + bytes([value[-1] ^ 1]), key))
    out = con.serialize()
    con.close()
    return out


def wholly(got):
    """Whether Faraday's reading `got` is a whole wallet."""
    return got[0] == "ok" and got[1]["whole"]


def slow_range(got):
    """Whether Faraday's whole reading `got` lists a ranged descriptor whose
    next index or last index is at least SLOW."""
    return got[0] == "ok" and any(
        d["range"] is not None and max(d["next_index"], d["range"][1]) >= SLOW
        for d in got[1]["descriptors"])


def load_wait(got):
    """How long Core's loadwallet is waited for, given Faraday's reading."""
    return SLOW_WAIT if slow_range(got) else LOAD_WAIT


def range_shown(got, listed):
    """Whether Core's listing `listed` (on a node with -keypool=1) shows,
    for each descriptor it lists and Faraday's reading `got` does not, a
    range Faraday's rule refuses: none (Core does not read the range of a
    descriptor that is not ranged), a negative next index or start, a
    start past its end, or an end at the next index (Core topped up an end
    it read at or before the next index). False where there is none."""
    if got[0] != "ok" or isinstance(listed, tuple):
        return False
    read = {d["listed"] for d in got[1]["descriptors"]}
    others = [d for d in listed if d["desc"] not in read]
    if not others:
        return False
    for d in others:
        r, n = d.get("range"), d.get("next_index", d.get("next"))
        if r is None or n is None:
            continue
        if not (n < 0 or r[0] < 0 or r[0] > r[1] + 1 or r[1] == n):
            return False
    return True


def encoding_shown(got, data):
    """Whether Faraday's refusal `got` is of the header's text encoding
    and SQLite reads the field of `data` as UTF-8 (its low two bits 0 or
    1, prepare.c `sqlite3InitOne`)."""
    return (got[0] != "ok" and "text encoding" in got[1]
            and int.from_bytes(data[56:60], "big") & 3 in (0, 1))


def wants_book(got):
    """Whether Core's address book is asked of a file: where Faraday names
    a purpose or name record it cannot read."""
    return got[0] == "ok" and any((f[0].split(" ")[0], f[1]) in BOOK
                                  for f in got[1]["failures"])


def raw_call(url, method, *params):
    """Costs an RPC call: Core's answer to `method`, its bytes that are not
    UTF-8 kept as surrogate escapes (a label Core read as written), which
    the vendored client cannot decode."""
    import base64
    import urllib.parse
    u = urllib.parse.urlparse(url)
    body = json.dumps({"jsonrpc": "1.0", "id": 1, "method": method, "params": list(params)},
                      ensure_ascii=False).encode("utf-8", "surrogateescape")
    auth = base64.b64encode(f"{u.username}:{u.password}".encode()).decode()
    conn = http.client.HTTPConnection(u.hostname, u.port, timeout=60)
    try:
        conn.request("POST", u.path or "/", body, {"Authorization": "Basic " + auth,
                                                   "Content-Type": "application/json"})
        answer = json.loads(conn.getresponse().read().decode("utf-8", "surrogateescape"))
    finally:
        conn.close()
    if answer.get("error"):
        raise RuntimeError(answer["error"])
    return answer["result"]


def label_shown(got, core, changed):
    """Whether Core's reading shows each of Faraday's name record failures:
    Core lists the record's address with a label that is not UTF-8 or is
    shorter than the bytes the record's value holds after its length."""
    labels = core[3] if len(core) > 3 else {}
    con = sqlite3.connect(":memory:", isolation_level=None)
    con.deserialize(changed)
    values = {bytes(k): bytes(v) for (k, v) in con.execute("SELECT key, value FROM main")
              if record_type(bytes(k).hex()) == "name"}
    con.close()
    fails = [f[0] for f in got[1]["failures"] if (f[0].split(" ")[0], f[1]) == ("name", "Layout")]
    if not fails:
        return False
    for failure in fails:
        address = purpose_address(failure)
        if address is None or address not in labels:
            return False
        value = values.get(b"\x04name" + bytes([len(address)]) + address.encode("latin-1"))
        label = labels[address]
        try:
            label.decode("utf-8")
            utf8 = True
        except UnicodeDecodeError:
            utf8 = False
        if value is None or (utf8 and len(label) >= len(value) - 1):
            return False
    return True


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
            "nothing is asked of Core there beyond that it loads, except where Core's own "
            "reading must show the condition: a purpose record Faraday cannot read (Core lists "
            "the address's purpose as unknown), a block locator that does not read as one "
            "(Core shows it read no locator from the record), a descriptor's range or next "
            "index out of order (Core lists it so, loaded with -keypool=1), and a header text "
            "encoding other than the three the file format names (SQLite reads it as UTF-8)."
        ),
        (
            "Core reads each transaction record as it loads and refuses a wallet one of which "
            "does not read; Faraday counts them. Core derives every index of a descriptor's "
            "range, and tops it up to its next index, as it loads; it refuses a descriptor "
            "that is not ranged whose stored range spans more than one index (the same "
            "script twice), and Faraday refuses it too; Faraday also refuses a range of "
            "1,000,000 indexes or more and a next index of 1,000,000 or more (Core's limit on "
            "the size of a range). Past 2^16 the area does not "
            "wait for Core's answer (past 2^19 it takes minutes), and Faraday, which derives "
            "nothing as it reads, reads such a file whole up to that limit; it is counted where Core did not "
            "answer. Each is asked of Core in the prelude."
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

    def loads(self, data, name, book=False, node=None, timeout=120, unlock=False):
        """Costs a bitcoind start and a wallet load: whether `loadwallet`
        opens `data` as a wallet, and what it lists then. The node is one
        of its own for each file (Core can stop on a damaged file, and the
        run's node is shared); one that stops is the answer "stopped".
        With `book`, a third and fourth item: each labelled address's purpose
        as `getaddressesbylabel` gives it ("unknown" where Core holds none),
        and its label's bytes as Core gives them (`raw_call`).
        With `unlock`, a third item: "ok" where `walletpassphrase` opens the
        wallet with the passphrase, else Core's refusal.
        Given a `node` (a `core_node.Node` with wallets, on regtest at its
        genesis block, as a fresh one is), the file is loaded there and
        unloaded after, and no node is started."""
        if node is None:
            with core_node.Node(rpc_threads=4, wallet=True) as fresh:
                return self.loads(data, name, book, fresh, timeout, unlock)
        import sys
        folder = str(ROOT / "tools" / "reference" / "core")
        if folder not in sys.path:
            sys.path.insert(0, folder)
        from test_framework.authproxy import AuthServiceProxy

        def client(path=""):
            c = AuthServiceProxy(node.info["url"] + path, timeout=timeout)
            c.reuse_http_connections = False
            return c

        where = node.scratch / "data" / "regtest" / "wallets" / name
        where.mkdir(parents=True)
        (where / "wallet.dat").write_bytes(data)
        try:
            client().loadwallet(name)
        except (ConnectionError, OSError, http.client.HTTPException):
            return ("err", "stopped")
        except Exception as e:  # noqa: BLE001 - a refusal of Core's is the answer
            shutil.rmtree(where, ignore_errors=True)
            return ("err", str(e))
        w = client(f"/wallet/{name}")
        try:
            listed = w.listdescriptors()["descriptors"]
        except Exception as e:  # noqa: BLE001
            listed = ("err", str(e))
        out = ("ok", listed)
        if book:
            purposes, labels = {}, {}
            wallet_url = node.info["url"] + f"/wallet/{name}"
            try:
                for label in w.listlabels():
                    for address, entry in w.getaddressesbylabel(label).items():
                        purposes[address] = entry["purpose"]
            except Exception:  # noqa: BLE001 - nothing listed shows no condition
                purposes = {}
            try:
                # Every address of the book with its label, none asked by a
                # label (Core's JSON parser refuses one that is not UTF-8).
                for entry in raw_call(wallet_url, "listreceivedbyaddress", 0, True, True):
                    if "label" in entry:
                        labels[entry["address"]] = entry["label"].encode("utf-8",
                                                                         "surrogateescape")
            except Exception:  # noqa: BLE001
                labels = {}
            out = ("ok", listed, purposes, labels)
        if unlock:
            try:
                w.walletpassphrase(PASSPHRASE, 1)
                unlocked = "ok"
            except Exception as e:  # noqa: BLE001 - Core's refusal is the answer
                unlocked = str(e)
            out = ("ok", listed, unlocked)
        try:
            client().unloadwallet(name)
        except (ConnectionError, OSError, http.client.HTTPException):
            pass
        except Exception:  # noqa: BLE001 - the answer is taken; the node is the caller's
            pass
        shutil.rmtree(where, ignore_errors=True)
        return out

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
        self.directed_shown()
        self.directed_slow()
        self.directed_unranged()
        for record, why in LENIENT.items():
            if record == "tx":
                continue  # made in directed_shown
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
                    and core[0] == "ok" and self.shown_by_core(("purpose", "Layout"), got, core,
                                                              changed, "k-known-purpose")):
                raise Mismatch(self.name, "core_wallet.read", {"changed": "purpose " + what},
                               [got[0], sorted(said)], [core[0], core[2:]],
                               "the listed difference no longer holds: Core loads a purpose "
                               "record that is not one whole UTF-8 string and lists its "
                               "address with the purpose \"unknown\", and Faraday names it "
                               "as a Layout fault")
        # A name record (a label) changed the same two ways.
        name = [k for k in keys("name") if len(value(k)) > 2][0]
        text = value(name)
        for what, new in (("not UTF-8", text[:1] + bytes([text[1] | 0x80]) + text[2:]),
                          ("bytes left over", bytes([text[0] - 1]) + text[1:])):
            con.deserialize(labelled)
            edit(name, new)
            changed = con.serialize()
            got = self.rust.call("core_wallet.read", db=changed.hex())
            core = self.loads(changed, f"k-known-name-{what.split()[0]}", book=True)
            self.known += 1
            if not (got[0] == "ok" and ("name", "Layout") in faraday_said(got)
                    and core[0] == "ok" and self.shown_by_core(("name", "Layout"), got, core,
                                                              changed, "k-known-name")):
                raise Mismatch(self.name, "core_wallet.read", {"changed": "name " + what},
                               faraday_said(got), [core[0], core[3:]],
                               "the listed difference no longer holds: "
                               + KNOWN_SHOWN[("name", "Layout")])
        con.close()

    def directed_shown(self):
        """Each KNOWN_SHOWN entry of a locator, a range or the text
        encoding, made in a wallet Core made and asked of Core: Core loads
        the file, Faraday names the record and fault, and Core's reading
        shows it (`shown_by_core`); and a transaction record that does not
        read, which Core refuses as corrupted and Faraday reads whole."""
        plain = self.make("plain", "k-shown", mine=True)
        data = plain["file"]

        def changed_value(record, change, nth=0):
            con = sqlite3.connect(":memory:", isolation_level=None)
            con.deserialize(data)
            key, value = [(bytes(k), bytes(v)) for (k, v) in
                          con.execute("SELECT key, value FROM main ORDER BY rowid")
                          if record_type(bytes(k).hex()) == record][nth]
            con.execute("UPDATE main SET value = ? WHERE key = ?", (change(value), key))
            out = con.serialize()
            con.close()
            return out

        def at(raw, where, new):
            return raw[:where] + new + raw[where + len(new):]

        def tail(field, v):
            # The descriptor's value ends in the time, then the next
            # index, the range's start and its end, four bytes each.
            where = {"next": -12, "start": -8, "end": -4}[field]
            return lambda raw: at(raw, len(raw) + where, v.to_bytes(4, "little", signed=True))

        cases = [
            (("bestblock", "Layout"), "the count raised",
             changed_value("bestblock", lambda v: at(v, 4, bytes([v[4] ^ 0x40])))),
            (("bestblock_nomerkle", "Layout"), "the count raised",
             changed_value("bestblock_nomerkle", lambda v: at(v, 4, bytes([v[4] ^ 0x40])))),
            (("bestblock_nomerkle", "Layout"), "the count 0",
             changed_value("bestblock_nomerkle", lambda v: at(v, 4, b"\0"))),
            (("walletdescriptor", "Range"), "the next index -1",
             changed_value("walletdescriptor", tail("next", -1))),
            (("walletdescriptor", "Range"), "the start past the end",
             changed_value("walletdescriptor", tail("start", 1 << 20))),
            (("walletdescriptor", "Range"), "the end -1",
             changed_value("walletdescriptor", tail("end", -1))),
            (("file", "File"), "text encoding 0", at(data, 56, (0).to_bytes(4, "big"))),
            (("file", "File"), "text encoding 5", at(data, 56, (5).to_bytes(4, "big"))),
        ]
        for n, (known, what, changed) in enumerate(cases):
            got = self.rust.call("core_wallet.read", db=changed.hex())
            core = self.loads(changed, f"k-shown-{n}")
            self.known += 1
            if not (core[0] == "ok" and known in faraday_said(got)
                    and self.shown_by_core(known, got, core, changed, f"k-shown-{n}")):
                raise Mismatch(self.name, "core_wallet.read", {"changed": what},
                               faraday_said(got), core[0],
                               f"the listed difference no longer holds: {KNOWN_SHOWN[known]}")
        encrypted = self.make("encrypted", "k-shown-mkey", mine=False)["file"]
        con = sqlite3.connect(":memory:", isolation_level=None)
        con.deserialize(encrypted)
        con.execute("DELETE FROM main WHERE key IN (SELECT key FROM main WHERE "
                    "substr(key, 2, 4) = CAST('mkey' AS BLOB))")
        changed = con.serialize()
        con.close()
        known = ("walletdescriptorckey", "NoMasterKey")
        got = self.rust.call("core_wallet.read", db=changed.hex(), passphrase=PASSPHRASE)
        core = self.loads(changed, "k-shown-mkey-d")
        self.known += 1
        if not (core[0] == "ok" and known in faraday_said(got)
                and self.shown_by_core(known, got, core, changed, "k-shown-mkey-d")):
            raise Mismatch(self.name, "core_wallet.read", {"changed": "the master key dropped"},
                           faraday_said(got), core[0],
                           f"the listed difference no longer holds: {KNOWN_SHOWN[known]}")
        changed = changed_value("tx", lambda v: v[:-1])
        got = self.rust.call("core_wallet.read", db=changed.hex())
        core = self.loads(changed, "k-shown-tx")
        self.known += 1
        if not (got[0] == "ok" and got[1]["whole"] and core[0] == "err"
                and CORRUPTED in str(core[1])):
            raise Mismatch(self.name, "core_wallet.read", {"changed": "a transaction cut short"},
                           got[0], core, f"the listed difference no longer holds: {LENIENT['tx']}")

    def directed_unranged(self):
        """The range stored on the descriptor of an imported uncompressed
        key (not ranged): five indexes, which Core refuses as corrupted (it
        derives the same script twice) and Faraday refuses as `Range`, so both
        refuse; a range of one index or none, which Core ignores and resets
        and Faraday lists with no range (Core is not asked)."""
        facts = self.make("uncompressed", "k-unranged", mine=False)

        def stored(end):
            con = sqlite3.connect(":memory:", isolation_level=None)
            con.deserialize(facts["file"])
            rows = [(bytes(k), bytes(v)) for (k, v) in
                    con.execute("SELECT key, value FROM main ORDER BY rowid")
                    if record_type(bytes(k).hex()) == "walletdescriptor"
                    and b"*" not in bytes(v)[1:1 + bytes(v)[0]]]
            if not rows:
                con.close()
                raise Mismatch(self.name, "core_wallet.read", {}, None, None,
                               "no descriptor that is not ranged to change")
            key, value = rows[0]
            con.execute("UPDATE main SET value = ? WHERE key = ?",
                        (value[:-4] + end.to_bytes(4, "little"), key))
            out = con.serialize()
            con.close()
            return out

        changed = stored(5)
        got = self.rust.call("core_wallet.read", db=changed.hex())
        core = self.loads(changed, "k-unranged-d")
        self.compared += 1
        if not (not wholly(got) and "Range" in {f for (_, f) in faraday_said(got)}
                and core[0] == "err" and CORRUPTED in str(core[1])):
            raise Mismatch(self.name, "core_wallet.read", {"changed": "unranged range end 5"},
                           faraday_said(got), core, "Faraday and Core must both refuse a "
                           "descriptor that is not ranged whose range spans more than one index")
        for end in (0, 1):
            got = self.rust.call("core_wallet.read", db=stored(end).hex())
            self.compared += 1
            if not wholly(got):
                raise Mismatch(self.name, "core_wallet.read", {"changed": f"unranged end {end}"},
                               faraday_said(got), "whole", "Faraday reads a range of one "
                               "index or none on a descriptor that is not ranged, as Core "
                               "ignores it")

    def directed_slow(self):
        """Core derives each index to a descriptor's next index as it loads.
        A next index of 999,999 (just under the limit): Faraday reads the
        file whole and lists it, and Core does not answer in SLOW_WAIT
        seconds. Past the limit Faraday refuses (Core is not asked): a next
        index of 1,000,000 and a range of 1,000,000 indexes, at any start;
        a small range that starts high is read whole."""
        facts = self.make("watchonly", "k-slow", mine=False)

        def stored(next_index, start, end):
            con = sqlite3.connect(":memory:", isolation_level=None)
            con.deserialize(facts["file"])
            key, value = [(bytes(k), bytes(v)) for (k, v) in
                          con.execute("SELECT key, value FROM main ORDER BY rowid")
                          if record_type(bytes(k).hex()) == "walletdescriptor"][0]
            tail = b"".join(n.to_bytes(4, "little") for n in (next_index, start, end))
            con.execute("UPDATE main SET value = ? WHERE key = ?", (value[:-12] + tail, key))
            out = con.serialize()
            con.close()
            return out

        changed = stored(999_999, 0, 10)
        got = self.rust.call("core_wallet.read", db=changed.hex())
        core = self.loads(changed, "k-slow-d", timeout=load_wait(got))
        self.known += 1
        if not (wholly(got) and slow_range(got)
                and (core == ("err", "stopped") or TIMED_OUT in str(core[1]))):
            raise Mismatch(self.name, "core_wallet.read", {"changed": "next index 999,999"},
                           faraday_said(got), core, "the listed difference no longer holds: "
                           "Core derives each index to a descriptor's next index as it loads")
        for what, args in (("next index 1,000,000", (1_000_000, 0, 10)),
                           ("range of 1,000,000 indexes", (0, 0, 1_000_001)),
                           ("range of 1,000,000 indexes from 5", (0, 5, 1_000_006))):
            got = self.rust.call("core_wallet.read", db=stored(*args).hex())
            self.compared += 1
            if wholly(got) or "Range" not in {f for (_, f) in faraday_said(got)}:
                raise Mismatch(self.name, "core_wallet.read", {"changed": what},
                               faraday_said(got), "refused by Core's limit of 1,000,000",
                               "Faraday must refuse a range of 1,000,000 indexes or a next "
                               "index of 1,000,000 or more")
        got = self.rust.call("core_wallet.read", db=stored(0, 5_000_000, 5_000_010).hex())
        self.compared += 1
        if not wholly(got):
            raise Mismatch(self.name, "core_wallet.read", {"changed": "small range, high start"},
                           faraday_said(got), "whole", "Faraday reads a small range that "
                           "starts high, as Core does")

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

    def shown_by_core(self, known, got, core, changed, name, node=None):
        """Whether Core's own reading of the file shows the condition of each of
        Faraday's failures of the KNOWN_SHOWN entry `known`. A purpose record:
        Core lists its address with the purpose "unknown". A block locator
        record: Core reads no locator from it, which it shows by a variant
        of the file with the last byte of `bestblock_nomerkle` changed (the
        genesis hash its locator ends in): Core refuses that variant as a
        wallet of another chain where it falls back from an unread
        `bestblock` to `bestblock_nomerkle`, and loads it where it reads no
        locator from `bestblock_nomerkle` (walletdb.cpp `ReadBestBlock`,
        wallet.cpp `AttachChain`). Costs a further load for a locator."""
        if known == ("name", "Layout"):
            return label_shown(got, core, changed)
        if known == ("purpose", "Layout"):
            book = core[2] if len(core) > 2 else {}
            addresses = [purpose_address(f[0]) for f in got[1]["failures"]
                         if (f[0].split(" ")[0], f[1]) == known]
            return bool(addresses) and all(a is not None and book.get(a) == "unknown"
                                           for a in addresses)
        if known[1] == "Layout" and known[0] in LOCATORS:
            variant = locator_end_changed(changed)
            if variant is None:
                return False
            again = self.loads(variant, name + "-v", node=node)
            if known[0] == "bestblock":
                return again[0] == "err" and CROSS_CHAIN in again[1]
            return again[0] == "ok"
        if known == ("walletdescriptor", "Range"):
            with core_node.Node(rpc_threads=4, wallet=True, args=["-keypool=1"]) as one:
                again = self.loads(changed, name + "-k", node=one, timeout=load_wait(got))
            return again[0] == "ok" and range_shown(got, again[1])
        if known == ("file", "File"):
            return encoding_shown(got, changed)
        if known == ("walletdescriptorckey", "NoMasterKey"):
            again = self.loads(changed, name + "-u", node=node, unlock=True)
            return again[0] == "ok" and again[2] != "ok"
        return False

    def judge_changed(self, facts, changed, name, desc):
        got = self.read_changed(facts, changed)
        core = self.loads(changed, name, book=wants_book(got), timeout=load_wait(got))
        outcome = self.verdict(facts, changed, name, desc, got, core)
        if outcome[0] == "diff":
            _, said, core_said, note = outcome
            raise Mismatch(self.name, "core_wallet.read",
                           {"shape": facts["shape"], "db": changed.hex(), **desc},
                           said, core_said, note)
        if outcome[0] == "known":
            self.known += 1

    def read_changed(self, facts, changed):
        """Faraday's reading of a changed file, with the passphrase where
        the wallet Core made was encrypted."""
        encrypted = "unlocked_until" in facts["info"]
        got = self.rust.call("core_wallet.read", db=changed.hex(),
                             **({"passphrase": PASSPHRASE} if encrypted else {}))
        self.compared += 1
        return got

    def verdict(self, facts, changed, name, desc, got, core, node=None):
        """Faraday's reading `got` and Core's `loads` answer `core` of a
        changed file, judged: ("agree",), ("known", why) or ("diff",
        Faraday's side, Core's side, note). Costs a further load of Core's
        where a KNOWN_SHOWN entry asks Core of a variant of the file."""
        whole = got[0] == "ok" and got[1]["whole"]
        if whole:
            lenient = desc.get("kind") in ("drop", "flip") and record_type(desc["record"])
            if (core[0] != "ok" and lenient in LENIENT
                    and (lenient != "tx" or CORRUPTED in str(core[1]))):
                return ("known", lenient)
            if (core[0] != "ok" and slow_range(got)
                    and (core == ("err", "stopped") or TIMED_OUT in str(core[1]))):
                return ("known", "slow load")
            if core[0] == "err" and desc.get("kind") == "flip" and INDEX in core[1]:
                # The flipped bytes are a copy of a key (an id also sits in
                # other records' values), found in the table, not the index.
                return ("known", "index")
            if (core[0] == "err" and desc.get("kind") == "flip"
                    and record_type(desc["record"]) in LOCATORS and CROSS_CHAIN in core[1]):
                return ("known", "cross-chain")
            if core[0] != "ok":
                return ("diff", "whole", core,
                        "Faraday called a file whole that Core's loadwallet refuses")
            if isinstance(core[1], tuple):
                return ("agree",)
            # Core tops the key pool up as it loads, so the range and next
            # index are not compared here.
            def loose(rows):
                return sorted(((r[0], r[1], r[2], r[3]) for r in rows), key=repr)

            if (loose(faraday_listing(got[1]["descriptors"]))
                    != loose(core_listing(core[1]))):
                return ("diff", faraday_listing(got[1]["descriptors"]), core_listing(core[1]),
                        "Faraday and Core list different descriptors")
            return ("agree",)
        if core[0] != "ok":
            return ("agree",)
        # Core loads it and Faraday does not call it whole.
        said = faraday_said(got)
        for one in said:
            if one in KNOWN_SHOWN and self.shown_by_core(one, got, core, changed, name, node):
                continue
            if one not in KNOWN:
                return ("diff", one, core[0],
                        "Core loads a file Faraday does not call whole, and the "
                        "difference is not listed in KNOWN")
        return ("known", " ".join("/".join(one) for one in said))
