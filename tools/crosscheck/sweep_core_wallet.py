"""Every changed file the core_wallet area's mutation loop can make, of
every shape, against Faraday and Bitcoin Core at once (`area_core_wallet.py`,
`case_mutation`).

For each shape (and, where a block is mined, with and without one) Core
makes the wallet on a node of its own, as the area does. Each record type
Faraday reads is changed in each way the loop changes a file:

- a bit flipped where the loop flips it (the first copy of the record's key
  or value in the file): every bit of every byte of a piece of up to 40
  bytes, and of a longer piece every bit of its first ten and last six
  bytes (its length prefix, the fields at its ends) and bits 0 and 7 of
  every byte between (a field's first or last byte, a character made
  another or not ASCII);
- the record dropped;
- the file cut after each whole page.

Of a type with more than twelve records (the `walletdescriptorcache` keys,
the transactions), the first two and the last are changed. Each file is
read by Faraday and loaded by Core (`CoreWallet.loads` on a node each
worker keeps, at its genesis block as a fresh one is), and judged as the
loop judges it (`CoreWallet.verdict`). Every pair the loop would fail on is
printed as one JSON line, then a count of the outcomes by record and kind.

    just crosscheck --sweep-core-wallet [--jobs N] [--shapes plain,blank] [--limit N]

run.sh builds the Rust command and runs this in a network namespace of its
own, where the cross-check runs (the pinned Core is fetched the same way).
A load Core does not answer in time is judged as the loop judges it, and
the worker's node is started again.
"""

import argparse
import collections
import hashlib
import json
import multiprocessing
import multiprocessing.util
import os
import pathlib
import sqlite3
import sys
import time

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import area_core_wallet as cw  # noqa: E402
import core_node  # noqa: E402

SAMPLED = 12  # a type with more records than this has three of them changed


def pieces(con):
    """Each record to change: (key, value), all of a type with up to
    SAMPLED records, else its first two and its last, in rowid order."""
    rows = [(bytes(k), bytes(v)) for (k, v) in
            con.execute("SELECT key, value FROM main ORDER BY rowid")]
    by_type = collections.defaultdict(list)
    for k, v in rows:
        by_type[cw.record_type(k.hex())].append((k, v))
    out = []
    for kind in sorted(by_type):
        some = by_type[kind]
        out += some if len(some) <= SAMPLED else some[:2] + some[-1:]
    return out, rows


def flips(length):
    """The (offset, bit) pairs flipped in a piece of `length` bytes."""
    for at in range(length):
        edge = length <= 40 or at < 10 or at >= length - 6
        for bit in (range(8) if edge else (0, 7)):
            yield at, bit


def mutations(facts):
    """Every changed file of one wallet Core made: (changed, desc, where)."""
    data = facts["file"]
    page = int.from_bytes(data[16:18], "big") or 65536
    page = 65536 if page == 1 else page
    con = sqlite3.connect(":memory:", isolation_level=None)
    con.deserialize(data)
    chosen, _ = pieces(con)
    seen = set()

    def fresh(changed):
        h = hashlib.sha256(changed).digest()
        if h in seen:
            return False
        seen.add(h)
        return True

    for key, value in chosen:
        for piece, where in ((key, "key"), (value, "value")):
            at = data.find(piece) if piece else -1
            if at < 0:
                continue
            for off, bit in flips(len(piece)):
                changed = bytearray(data)
                changed[at + off] ^= 1 << bit
                changed = bytes(changed)
                if fresh(changed):
                    yield changed, {"kind": "flip", "record": key.hex(), "in": where}, \
                        {"offset": off, "bit": bit, "length": len(piece)}
        db = sqlite3.connect(":memory:", isolation_level=None)
        db.deserialize(data)
        db.execute("DELETE FROM main WHERE key = ?", (key,))
        changed = db.serialize()
        db.close()
        if fresh(changed):
            yield changed, {"kind": "drop", "record": key.hex()}, {}
    for keep in range(1, len(data) // page):
        yield data[: keep * page], {"kind": "cut", "pages_kept": keep}, {}
    con.close()


# ----- The workers: each its own Rust command and Core node -------------------

_W = {}


def _start(rust, keypool):
    area = cw.CoreWallet(rust, "sweep", 1.0)
    node = core_node.Node(rpc_threads=4, wallet=True, args=[f"-keypool={keypool}"])
    node.start()
    _W.update(area=area, node=node, n=0)
    multiprocessing.util.Finalize(None, _stop, exitpriority=10)


def _stop():
    if _W.get("node"):
        _W["node"].stop()
    if _W.get("area"):
        _W["area"].rust.proc.kill()


def _restart(node):
    """The worker's node stopped and started again, tried three times."""
    node.stop()
    for attempt in range(3):
        try:
            node.start()
            return
        except core_node.CoreError:
            node.stop()
            if attempt == 2:
                raise


def _judge(job):
    try:
        return _judge_one(job)
    except core_node.CoreError as e:
        # A node that did not start: the file is named, not judged.
        _restart(_W["node"])
        facts, _, desc, where = job
        return {"desc": desc, "where": where, "faraday": None, "core": None,
                "outcome": "diff", "detail": ["not judged", str(e)[:200], "node"]}


def _judge_one(job):
    facts, changed, desc, where = job
    area, node = _W["area"], _W["node"]
    _W["n"] += 1
    name = f"s{os.getpid()}-{_W['n']}"
    got = area.read_changed(facts, changed)
    core = area.loads(changed, name, book=cw.wants_book(got), node=node,
                      timeout=cw.load_wait(got))
    if core == ("err", "stopped") or (core[0] == "err" and cw.TIMED_OUT in str(core[1])):
        # Core may still be deriving: the node is started again.
        _restart(node)
    outcome = area.verdict(facts, changed, name, desc, got, core, node)
    said = cw.faraday_said(got) if not (got[0] == "ok" and got[1]["whole"]) else "whole"
    core_said = core[0] if core[0] == "ok" else ("err", str(core[1]).replace(name, "W")[-300:])
    return {"desc": desc, "where": where, "faraday": said, "core": core_said,
            "outcome": outcome[0], "detail": outcome[1:] if outcome[0] != "diff"
            else [str(outcome[1])[:200], str(outcome[2])[:200], outcome[3]]}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--rust", required=True)
    ap.add_argument("--jobs", type=int, default=os.cpu_count())
    ap.add_argument("--shapes", default=",".join(cw.SHAPES))
    ap.add_argument("--limit", type=int, default=0, help="at most N files a wallet (timing)")
    ap.add_argument("--keypool", type=int, default=5,
                    help="Core's -keypool on both nodes (the loop's wallets have 1000)")
    args = ap.parse_args()
    core_node.loopback_up()
    started = time.monotonic()
    node = core_node.Node(rpc_threads=4, wallet=True, args=[f"-keypool={args.keypool}"])
    node.start()
    try:
        os.environ.update(node.environ())
        maker = cw.CoreWallet(args.rust, "sweep", 1.0)
        jobs = []
        for shape in args.shapes.split(","):
            for mine in ((False, True) if shape in ("plain", "labels", "encrypted") else (False,)):
                facts = maker.make(shape, f"w-{shape}-{int(mine)}", mine)
                small = {"shape": shape, "info": facts["info"]}
                made = list(mutations(facts))
                if args.limit:
                    made = made[: args.limit]
                jobs += [(small, c, d, dict(w, shape=shape, mined=mine)) for c, d, w in made]
        maker.rust.proc.kill()
    finally:
        node.stop()
    made_at = time.monotonic()
    print(json.dumps({"files": len(jobs), "made_s": round(made_at - started, 1),
                      "by_shape": collections.Counter(w["shape"] for *_, w in jobs)}), flush=True)
    counts = collections.Counter()
    with multiprocessing.get_context("fork").Pool(args.jobs, _start, (args.rust, args.keypool)) as pool:
        for r in pool.imap_unordered(_judge, jobs, chunksize=8):
            rec = cw.record_type(r["desc"]["record"]) if "record" in r["desc"] else "file"
            counts[(r["outcome"], rec, r["desc"]["kind"],
                    r["detail"][0] if r["outcome"] == "known" else "")] += 1
            if r["outcome"] == "diff":
                print(json.dumps(r), flush=True)
        pool.close()
        pool.join()
    took = time.monotonic() - made_at
    for (outcome, rec, kind, why), n in sorted(counts.items()):
        print(json.dumps({"count": n, "outcome": outcome, "record": rec, "kind": kind,
                          "why": why}))
    print(json.dumps({"files": len(jobs), "judged_s": round(took, 1),
                      "per_s": round(len(jobs) / took, 1) if took else None,
                      "diffs": sum(n for (o, *_), n in counts.items() if o == "diff")}))
    print("DONE", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
