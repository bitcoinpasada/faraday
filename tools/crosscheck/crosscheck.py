#!/usr/bin/env python3
"""The crypto cross-check (F79, docs/CROSSCHECK.md §10).

Faraday's own Rust answers requests (`tools/crosscheck/src/`), and each
area here recomputes every answer with the vendored references in
`tools/reference/`, over every published vector and a corpus of random
inputs drawn from a fixed seed. The first disagreement stops the area
and is printed with its inputs and both answers.

Run it through `tools/crosscheck/run.sh` (`just crosscheck`), which
builds the Rust command offline and runs this with no network.

    crosscheck.py --rust EXE              the stamp run, fixed seed
    crosscheck.py --rust EXE --fresh      a new seed, ten times the cases
    crosscheck.py --rust EXE --area musig one area only (writes no stamp)
    crosscheck.py --rust EXE --jobs 4     four workers (default: one a core)

Each area is cut into batches: its prelude (the published vectors), its
corpus loops by case index, and its coda. Every corpus case draws from a
random stream of its own (`common.Area`), so a batch gives the same
answers in any worker, and a pool of workers takes the batches from a
queue, the slowest areas' first. The answers are put back together in
case order: the report, the counts and the verdict are those of one
worker running every case in turn, whatever the number of workers or the
Python they run under.

Before the run proper, the self-test runs each area against a Rust
command told to answer one operation wrongly, and fails unless every
area catches it: a cross-check that cannot fail is worse than none.

While it runs, `out/crosscheck/progress.json` and `progress.log` say how
far it has got (`status.py`, `just crosscheck-status`).
"""

import argparse
import importlib
import json
import os
import pathlib
import platform
import secrets
import selectors
import shutil
import signal
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import common  # noqa: E402
import core_node  # noqa: E402

DEFAULT_SEED = "faraday-crosscheck-1"
SELF_TEST_SCALE = 0.05
# A corpus loop is cut into at most this many batches.
PIECES = 50

# Each area: its module, its class, the operation (or operations, one self-test round each) the self-test makes the
# Rust command answer wrongly, and its weight: about how long it takes,
# relative to the others (seconds at scale 10, 2026-10-27), which orders
# the queue (the slowest first) and nothing else.
AREAS = {
    "musig": ("area_musig", "Musig", "musig.sign", 230),
    "sp": ("area_silent", "Silent", "sp.sender", 360),
    "sps": ("area_silent_send", "SilentSend", "sps.shares", 480),
    "bip85": ("area_bip85", "Bip85", "bip85.wif", 180),
    "slip39": ("area_slip39", "Slip39", "slip39.recover", 310),
    "codex32": ("area_codex32", "Codex32", "codex32.recover", 730),
    "codex32-correct": ("area_codex32_correct", "Codex32Correct", "codex32.correct", 120),
    "xor": ("area_xor", "Xor", "xor.combine", 350),
    "vault": ("area_vault", "Vault", "vault.open", 320),
    "bip39": ("area_bip39", "Bip39", "bip39.seed", 740),
    "bip32": ("area_bip32", "Bip32", "bip32.derive", 340),
    "addr": ("area_addr", "Addr", "addr.parse", 230),
    "tx": ("area_tx", "Tx", "tx.sighash", 55),
    "psbt": ("area_psbt", "Psbt", "psbt.v0", 460),
    "sign": ("area_sign", "Sign", "sign.sign", 700),
    "psbt-review": ("area_review", "Review", "review.inspect", 120),
    "taproot": ("area_taproot", "Taproot", "taproot.tree", 900),
    "miniscript": ("area_miniscript", "Miniscript", "miniscript.script", 460),
    "msg": ("area_msg", "Msg", ("msg.sign", "msg.verify", "msg.bip322"), 150),
    "own-randomness": ("area_own", "Own", "own.session", 80),
    "entropy": ("area_entropy", "Entropy", "entropy.cards", 140),
    "roller": ("area_roller", "Roller", "roller.five", 45),
    "seedqr": ("area_seedqr", "SeedQR", "seedqr.encode", 700),
    "bbqr": ("area_bbqr", "Bbqr", "bbqr.join", 300),
    "ur": ("area_ur", "Ur", "ur.encode", 410),
    "vanity": ("area_vanity", "Vanity", "vanity.grind", 460),
    "kdbx": ("area_kdbx", "Kdbx", "kdbx.verify", 30),
    "lightning": ("area_lightning", "Lightning",
                  ("lightning.ldk", "lightning.aezeed", "lightning.lnd"), 240),
}


TEST_AREAS = None


def use_test_areas(folder):
    """The harness's own tests (`test_harness.py`): areas of their own,
    from `areas.json` in `folder`, in place of the real ones."""
    global TEST_AREAS
    TEST_AREAS = folder
    sys.path.insert(0, folder)
    AREAS.clear()
    AREAS.update({k: tuple(v) for k, v in json.loads(
        (pathlib.Path(folder) / "areas.json").read_text()).items()})


# The areas that run faster under PyPy than under CPython (measured
# 2026-10-27): with `--python auto`, the only ones given to PyPy. Most
# of the rest spend their time in `pow` on big integers, which CPython
# does faster.
PYPY_FASTER = {"codex32", "seedqr"}
# The areas PyPy is given, when it is found: None for every area that
# imports under it (`--python pypy`).
ALTERNATIVE_FOR = None


def area_class(area):
    module, cls = AREAS[area][:2]
    return getattr(importlib.import_module(module), cls)


# ----- The worker ------------------------------------------------------------

def run_batch(req, alternative):
    """One batch in this process: its result, with its counts."""
    started = time.monotonic()
    result = {"id": req["id"], "area": req["area"], "cases": 0}
    if req["part"] == "unreadable":
        result.update(status="error", seconds=0.0,
                      report=f"the area's loops could not be read: {req['why']}")
        return result
    try:
        cls = area_class(req["area"])
    except Exception as e:  # noqa: BLE001 - reported, and the run fails
        if alternative:
            result.update(status="unavailable", report=f"{type(e).__name__}: {e}")
            return result
        cls = e
    area = None
    try:
        if isinstance(cls, Exception):
            raise cls
        area = cls(req["rust"], req["seed"], req["scale"], req.get("sabotage"))
        area.fund_coins = req.get("coins")
        try:
            area.run_part(req["part"], req["lo"], req["hi"])
        finally:
            area.close()
        result.update(status="pass", compared=area.compared, known=area.known)
        if req["part"] not in ("prelude", "coda"):
            result["cases"] = req["hi"] - req["lo"]
        if req["part"].startswith("fund:"):
            result["funding"] = area.funding_txs
    except common.Mismatch as m:
        result.update(status="mismatch", report=m.report(), op=m.op, inputs=m.inputs,
                      rust=m.rust, reference=m.reference)
    except Exception as e:  # noqa: BLE001 - reported, and the run fails
        import traceback
        where = "".join(traceback.format_exception(e)[-3:])
        result.update(status="error", report=f"{type(e).__name__}: {e}\n{where}")
    if result["status"] in ("mismatch", "error") and area is not None and area.case_at:
        loop, i = area.case_at
        result["case"] = loop if i is None else f"{loop}:{i}"
    result["seconds"] = time.monotonic() - started
    result["memory_mb"] = peak_memory_mb()
    return result


def peak_memory_mb():
    """This process's largest resident size so far, in MB (output only)."""
    try:
        import resource
        kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        return round(kb / 1024) if sys.platform != "darwin" else round(kb / 1024 / 1024)
    except (ImportError, OSError):
        return None


def worker(alternative):
    """A pool worker: one batch request a line on stdin, one result a
    line on the standard output it started with. Whatever an area prints
    goes to standard error."""
    out = os.fdopen(os.dup(1), "w", buffering=1)
    os.dup2(2, 1)
    try:
        common.check_references()
        refused = None
    except common.HarnessError as e:
        refused = f"the references are not the vendored ones: {e}"
    served = None
    for line in sys.stdin:
        req = json.loads(line)
        served = served or req["area"]
        if req["area"] != served:
            # Two areas' references may import modules of one name.
            result = {"id": req["id"], "area": req["area"], "cases": 0, "status": "error",
                      "report": f"a worker of the {served} area was given a batch of another",
                      "seconds": 0}
        elif refused:
            result = {"id": req["id"], "area": req["area"], "cases": 0, "status": "error",
                      "report": refused, "seconds": 0}
        else:
            result = run_batch(req, alternative)
        out.write(json.dumps(result) + "\n")
    return 0


# ----- The plan --------------------------------------------------------------

def shape(area):
    """An area's corpus loops (at scale 1) and whether it has a coda, read
    in a process of its own (`--shape`), as the areas' references cannot
    all be imported into one."""
    cls = area_class(area)
    return {"loops": [list(x) for x in cls.LOOPS], "coda": cls.coda is not common.Area.coda,
            "core": bool(getattr(cls, "CORE", False)), "fund": dict(getattr(cls, "FUND", {})),
            "fund_depth": int(getattr(cls, "FUND_DEPTH", 0))}


def shapes(areas):
    """Every area's shape, each read by a process of its own, side by
    side; an area whose shape cannot be read has its reason instead."""
    procs = {}
    for area in areas:
        cmd = [sys.executable, "-I", "-B", str(HERE / "crosscheck.py"), "--shape", area]
        if TEST_AREAS:
            cmd += ["--test-areas", TEST_AREAS]
        procs[area] = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                       text=True)
    out = {}
    for area, p in procs.items():
        stdout, stderr = p.communicate()
        try:
            out[area] = json.loads(stdout.strip().splitlines()[-1])
        except (IndexError, json.JSONDecodeError):
            out[area] = {"error": (stderr.strip().splitlines() or ["no answer"])[-1]}
    return out


def plan(areas, seed, scale, rust, shaped, sabotage=None, only=None, coins=None, fund=False):
    """Every batch of these areas, in each area's case order, and the
    order they are queued in: the heaviest areas' first, each area's
    prelude and coda before the corpus batches, which are the ones that
    even the load at the end. An area whose shape could not be read is
    one batch that fails with the reason: never a run with cases left
    out. With `fund`, the batches of the funded loops' funding
    (`fund:<loop>`) instead; `coins` gives each area its coins."""
    batches = []
    for area in areas:
        sh = shaped[area]
        if "error" in sh:
            parts = [("unreadable", 0, 0)]
        else:
            parts = [] if fund else [("prelude", 0, 0)]
            for loop, n in sh["loops"]:
                if fund and loop not in sh.get("fund", {}):
                    continue
                n = max(1, int(n * scale))
                k = min(n, PIECES)
                name = f"fund:{loop}" if fund else loop
                parts += [(name, j * n // k, (j + 1) * n // k) for j in range(k)]
            if sh["coda"] and not fund:
                parts.append(("coda", 0, 0))
        if only:
            loop, i = only
            if fund and loop not in sh.get("fund", {}):
                continue
            parts = [(f"fund:{loop}" if fund else loop, i, i + 1)]
        for index, (part, lo, hi) in enumerate(parts):
            batches.append({
                "area": area, "index": index, "part": part, "lo": lo, "hi": hi,
                "seed": seed, "scale": scale, "rust": rust,
                "sabotage": sabotage(area) if sabotage else None,
                "why": sh.get("error"),
                "coins": (coins or {}).get(area),
            })
    order = sorted(range(len(batches)), key=lambda b: (
        batches[b]["part"] not in ("prelude", "coda"),
        -AREAS[batches[b]["area"]][3], batches[b]["area"], batches[b]["index"]))
    for n, b in enumerate(batches):
        b["id"] = n
    return batches, [batches[i] for i in order]


def cases_total(batches, area):
    return sum(b["hi"] - b["lo"] for b in batches
               if b["area"] == area and b["part"] not in ("prelude", "coda"))


def merge(batches, results, area):
    """An area's result from its batches, in case order: the first batch
    that did not pass decides it, as the first disagreement stops one
    worker running the area's cases in turn."""
    out = {"area": area, "status": "pass", "compared": 0, "known": 0, "cases": 0, "seconds": 0.0}
    for b in sorted((b for b in batches if b["area"] == area), key=lambda b: b["index"]):
        r = results.get(b["id"])
        if r is None:
            continue
        out["seconds"] += r.get("seconds", 0)
        if r["status"] == "pass":
            out["compared"] += r["compared"]
            out["known"] += r["known"]
            out["cases"] += r.get("cases", 0)
            continue
        for key in ("status", "report", "op", "inputs", "rust", "reference", "case"):
            if key in r:
                out[key] = r[key]
        break
    out["seconds"] = round(out["seconds"], 1)
    planned = cases_total(batches, area)
    if out["status"] == "pass" and out["cases"] != planned:
        out.update(status="error", report=f"{out['cases']} of its {planned} corpus cases ran")
    return out


# ----- The pool --------------------------------------------------------------

class Slot:
    """One worker's place. A worker process serves one area only: the
    references of different areas import modules of the same name (two
    `reference`s, two `bip32`s), which one process cannot hold side by
    side. A slot keeps the processes of the last two areas it ran and
    ends older ones; only one of them runs a batch at a time."""

    KEEP = 2

    def __init__(self, n, interpreters):
        self.n = n
        self.interpreters = interpreters
        self.procs = {}
        self.area = None
        self.batch = None
        self.started = 0

    def proc(self, area, which):
        key = (area, which)
        p = self.procs.pop(key, None)
        if p is None or p.poll() is not None:
            cmd = [self.interpreters[which], "-I", "-B", str(HERE / "crosscheck.py"), "--worker"]
            if which == "alternative":
                cmd.append("--alternative")
            if TEST_AREAS:
                cmd += ["--test-areas", TEST_AREAS]
            p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                 bufsize=1)
        self.procs[key] = p  # the most recent last
        while len(self.procs) > self.KEEP:
            old = next(iter(self.procs))
            self.end(self.procs.pop(old))
        return p

    @staticmethod
    def end(p):
        try:
            p.stdin.close()
        except OSError:
            pass
        try:
            p.wait(timeout=10)
        except subprocess.TimeoutExpired:
            p.kill()
            p.wait()

    def close(self):
        for p in self.procs.values():
            self.end(p)
        self.procs = {}


def run_pool(batches, queue, jobs, interpreters, progress):
    """Runs the batches on `jobs` workers; their results by batch id.

    A freed worker keeps to its area while the area has the most work
    left for each worker on it, or near it; otherwise it moves to the
    area that has, so the heaviest areas are spread over the most
    workers and every worker stays busy to the end. Once a batch of an
    area fails, the area's later batches still queued are not run: they
    cannot change its result."""
    results = {}
    failed_at = {}
    alt_off = set()
    pending = {}
    for b in queue:
        pending.setdefault(b["area"], []).append(b)
    for area in pending:
        pending[area].sort(key=lambda b: b["index"])
    total = {a: len(bs) for a, bs in pending.items()}
    slots = [Slot(n, interpreters) for n in range(jobs)]
    sel = selectors.DefaultSelector()

    def work_left(area):
        return AREAS[area][3] * len(pending[area]) / total[area]

    def on(area):
        return sum(1 for s in slots if s.batch is not None and s.batch["area"] == area)

    def pick(slot):
        """The area a freed worker takes its next batch from."""
        live = [a for a in pending if pending[a]]
        if not live:
            return None
        score = {a: work_left(a) / (1 + on(a)) for a in live}
        best = max(live, key=lambda a: (score[a], -list(AREAS).index(a)))
        if slot.area in score and score[slot.area] * 2 >= score[best]:
            return slot.area
        return best

    def next_batch(slot):
        while True:
            area = pick(slot)
            if area is None:
                return None
            b = pending[area].pop(0)
            if b["index"] > failed_at.get(area, 1 << 30):
                if progress.skipped(b):
                    progress.area_end(area, merge(batches, results, area))
                continue
            return b

    def give(slot):
        b = next_batch(slot)
        slot.batch = b
        if b is None:
            return
        which = "alternative" if "alternative" in interpreters and b["area"] not in alt_off \
            and (ALTERNATIVE_FOR is None or b["area"] in ALTERNATIVE_FOR) else "main"
        b["which"] = which
        slot.area = b["area"]
        p = slot.proc(b["area"], which)
        try:
            sel.get_key(p.stdout)
        except KeyError:
            sel.register(p.stdout, selectors.EVENT_READ, (slot, p))
        slot.started = time.monotonic()
        progress.started(b)
        try:
            p.stdin.write(json.dumps(b) + "\n")
            p.stdin.flush()
        except BrokenPipeError:
            pass

    for s in slots:
        give(s)
    try:
        while any(s.batch for s in slots):
            events = sel.select(timeout=min(1.0, progress.every))
            progress.tick()
            for key, _ in events:
                slot, p = key.data
                line = key.fileobj.readline()
                b = slot.batch
                if not line:
                    sel.unregister(key.fileobj)
                    p.wait()
                    if b is None or slot.procs.get((b["area"], b["which"])) is not p:
                        continue
                    r = {"id": b["id"], "area": b["area"], "cases": 0, "status": "error",
                         "seconds": time.monotonic() - slot.started,
                         "report": "the worker exited without an answer"}
                else:
                    r = json.loads(line)
                if r["status"] == "unavailable":
                    alt_off.add(b["area"])
                    progress.requeued(b)
                    progress.note(f"{b['area']} runs under {interpreters['main']}: under "
                                  f"{interpreters['alternative']} {r['report']}")
                    pending[b["area"]].insert(0, b)
                    give(slot)
                    continue
                results[b["id"]] = r
                if r["status"] != "pass":
                    failed_at[b["area"]] = min(failed_at.get(b["area"], 1 << 30), b["index"])
                if progress.finished(b, r):
                    progress.area_end(b["area"], merge(batches, results, b["area"]))
                give(slot)
    finally:
        for s in slots:
            s.close()
    return results


# ----- Progress --------------------------------------------------------------

def case_words(case):
    """' at case corpus:12', ' in its prelude', or nothing."""
    if not case:
        return ""
    return f" at case {case}" if ":" in case else f" in its {case}"


def say_time(seconds):
    seconds = int(seconds)
    if seconds < 60:
        return f"{seconds} s"
    if seconds < 3600:
        return f"{seconds // 60} min {seconds % 60} s"
    return f"{seconds // 3600} h {seconds // 60 % 60} min"


class Progress:
    """`progress.json`, rewritten whole (written aside and renamed) at
    most once a second and at each change of phase, and `progress.log`,
    a line at each event and at least every ten seconds. Output only:
    nothing here touches a case or an answer."""

    def __init__(self, folder, info, every=10.0):
        self.every = every
        self.dir = pathlib.Path(folder) if folder else None
        self.info = dict(info, pid=os.getpid())
        self.state = {}
        self.last_write = 0.0
        self.last_line = 0.0
        if self.dir:
            self.dir.mkdir(parents=True, exist_ok=True)

    def log(self, text):
        line = time.strftime("%H:%M:%S ") + text
        print(line, file=sys.stderr, flush=True)
        if self.dir:
            with open(self.dir / "progress.log", "a") as f:
                f.write(line + "\n")
        self.last_line = time.monotonic()

    def note(self, text):
        self.log(text)

    def phase(self, name, batches, jobs):
        self.name = name
        self.batches = batches
        self.jobs = jobs
        self.t0 = time.monotonic()
        self.busy = 0
        self.areas = {}
        for b in batches:
            a = self.areas.setdefault(b["area"], {
                "state": "queued", "cases_done": 0, "cases_total": 0, "answers": 0, "known": 0,
                "disagreements": 0, "errors": 0, "seconds": 0.0, "batches_left": 0,
                "case_seconds": 0.0})
            a["batches_left"] += 1
            if b["part"] not in ("prelude", "coda"):
                a["cases_total"] += b["hi"] - b["lo"]
        self.disagreements = []
        self.memory_mb = getattr(self, "memory_mb", 0)
        self.write(force=True)

    def started(self, b):
        self.busy += 1
        a = self.areas[b["area"]]
        if a["state"] == "queued":
            a["state"] = "running"
            if self.name == "run":
                self.log(f"{b['area']}: started, {a['cases_total']} cases")
        self.write()

    def finished(self, b, r):
        self.busy -= 1
        a = self.areas[b["area"]]
        a["batches_left"] -= 1
        a["seconds"] += r.get("seconds", 0)
        if r.get("memory_mb"):
            self.memory_mb = max(self.memory_mb, r["memory_mb"])
        a["cases_done"] += r.get("cases", 0)
        if r.get("cases"):
            a["case_seconds"] += r.get("seconds", 0)
        a["answers"] += r.get("compared", 0)
        a["known"] += r.get("known", 0)
        if r["status"] in ("mismatch", "error") and self.name == "run":
            key = "disagreements" if r["status"] == "mismatch" else "errors"
            a[key] += 1
            a["state"] = "failed"
            where = case_words(r.get("case"))
            what = r.get("op") or r.get("report", "").splitlines()[0]
            self.disagreements.append({"area": b["area"], "status": r["status"],
                                       "case": r.get("case"), "op": what})
            self.log(f"{b['area']}: {'DISAGREEMENT' if key == 'disagreements' else 'ERROR'}"
                     f"{where}: {what}")
        self.write()
        return a["batches_left"] == 0

    def requeued(self, b):
        self.busy -= 1

    def skipped(self, b):
        a = self.areas[b["area"]]
        a["batches_left"] -= 1
        return a["batches_left"] == 0

    def area_end(self, area, r):
        """An area's last batch is in: its result, as the report will
        give it."""
        a = self.areas[area]
        a["state"] = "done" if r["status"] == "pass" else "failed"
        work = say_time(a["seconds"])
        if self.name == "self-test":
            op = SABOTAGED[area]
            caught = r["status"] == "mismatch" and r.get("op") == op
            a["state"] = "done" if caught else "failed"
            self.log(f"self-test {area}: " + (f"caught a wrong {op} ({work} of work)" if caught
                                               else f"FAILED to catch a wrong {op}: {r['status']}"))
        elif self.name == "funding":
            pass
        elif r["status"] == "pass":
            self.log(f"{area}: done, {a['cases_done']} of {a['cases_total']} cases, "
                     f"{r['compared']} answers agree"
                     + (f", {r['known']} of them a known difference" if r["known"] else "")
                     + f" ({work} of work)")
        else:
            self.log(f"{area}: {r['status'].upper()}{case_words(r.get('case'))}, stopped there "
                     f"({work} of work)")
        self.write(force=True)

    def estimate(self):
        """Cases a second over the phase, and the time left: each area's
        work left at its own rate so far, shared by the workers."""
        elapsed = time.monotonic() - self.t0
        done = sum(a["cases_done"] for a in self.areas.values())
        rate = done / elapsed if elapsed > 0 else 0.0
        # An area not yet seen is reckoned from its weight, at the median
        # ratio of the seen areas' whole work to their weights.
        ratios = sorted(a["case_seconds"] / a["cases_done"] * a["cases_total"] / AREAS[name][3]
                        for name, a in self.areas.items() if a["cases_done"])
        if not ratios:
            return rate, None
        per_weight = ratios[len(ratios) // 2]
        left = 0.0
        for name, a in self.areas.items():
            if a["state"] in ("done", "failed") or not a["cases_total"]:
                continue
            if a["cases_done"]:
                left += a["case_seconds"] / a["cases_done"] * (a["cases_total"] - a["cases_done"])
            else:
                left += per_weight * AREAS[name][3]
        return rate, left / max(1, self.jobs)

    def snapshot(self):
        rate, left = self.estimate()
        return dict(self.info, phase=self.name, updated=time.time(),
                    elapsed=round(time.monotonic() - self.t0, 1), jobs=self.jobs,
                    workers_busy=self.busy, cases_per_second=round(rate, 1),
                    seconds_left=None if left is None else round(left),
                    cases_done=sum(a["cases_done"] for a in self.areas.values()),
                    cases_total=sum(a["cases_total"] for a in self.areas.values()),
                    areas={k: {x: (round(y, 1) if isinstance(y, float) else y)
                               for x, y in v.items() if x not in ("batches_left", "case_seconds")}
                           for k, v in self.areas.items()},
                    disagreements=self.disagreements, largest_worker_mb=self.memory_mb)

    def write(self, force=False, final=None):
        now = time.monotonic()
        if not self.dir or (not force and now - self.last_write < 1.0):
            return
        self.last_write = now
        snap = self.snapshot()
        if final:
            snap["phase"] = final
        tmp = self.dir / "progress.json.tmp"
        tmp.write_text(json.dumps(snap, indent=1) + "\n")
        os.replace(tmp, self.dir / "progress.json")

    def tick(self):
        self.write()
        if time.monotonic() - self.last_line >= self.every:
            snap = self.snapshot()
            left = "" if snap["seconds_left"] is None else \
                f", about {say_time(snap['seconds_left'])} left"
            running = ", ".join(f"{k} {v['cases_done']}/{v['cases_total']}"
                                for k, v in self.areas.items() if v["state"] == "running")
            done = sum(v["state"] == "done" for v in self.areas.values())
            self.log(f"{self.name}: {snap['cases_done']} of {snap['cases_total']} cases, "
                     f"{snap['cases_per_second']} a second, {snap['workers_busy']} of {self.jobs} "
                     f"workers busy{left}; {done} of {len(self.areas)} areas done"
                     + (f"; running: {running}" if running else ""))


# ----- The run ---------------------------------------------------------------

def default_jobs():
    try:
        return len(os.sched_getaffinity(0))
    except AttributeError:
        return os.cpu_count() or 1


def interpreters_for(args):
    """The main interpreter (this one) and, when it is found, PyPy for
    the areas that can import under it."""
    out = {"main": sys.executable}
    if args.python == "cpython":
        return out
    pypy = args.python if args.python not in (None, "auto", "pypy") else shutil.which("pypy3")
    if pypy and args.python != "cpython":
        try:
            ok = subprocess.run([pypy, "-I", "-c", "import sys; assert sys.version_info >= (3, 11)"],
                                capture_output=True, timeout=30).returncode == 0
        except (OSError, subprocess.TimeoutExpired):
            ok = False
        if ok:
            out["alternative"] = pypy
        elif args.python not in (None, "auto"):
            raise SystemExit(f"crosscheck: {pypy} is not a Python 3.11 or later")
    elif args.python == "pypy":
        raise SystemExit("crosscheck: --python pypy, but no pypy3 is on the PATH")
    return out


# The operation each area's Rust command is told to answer wrongly in the
# self-test round being run (SABOTAGED[area]).
SABOTAGED = {}


def sabotaged_ops(area):
    """The operations the self-test makes wrong for an area, one a round:
    the area's third entry, a name or a tuple of names."""
    op = AREAS[area][2]
    return (op,) if isinstance(op, str) else tuple(op)


def coin_layout(areas, shaped, scale):
    """Which coins each funded area's loops spend, in the run and in the
    self-test: ({area: {loop: base}} for each, the coins in all). The
    run's come first, so case i of an area's first funded loop spends
    the same coins whatever the scale or the cases picked."""
    out, k = ({}, {}), 0
    for which, s in ((0, scale), (1, SELF_TEST_SCALE)):
        for area in areas:
            sh = shaped[area]
            for loop, n in sh.get("loops", []):
                per = sh.get("fund", {}).get(loop)
                if per:
                    out[which].setdefault(area, {})[loop] = k
                    k += max(1, int(n * s)) * per
    return out[0], out[1], k


def fund(args, areas, jobs, interpreters, progress, seed, scale, only):
    """The coins of the areas that spend on Core's node, and their
    funding sent and buried, for the self-test and the run: each area's
    coins for the two, or a report of what failed."""
    funded = [a for a in areas if args.shaped[a].get("fund")]
    run_base, self_base, total = coin_layout(funded, args.shaped, scale)
    spec = core_node.make_coins(total)
    coins = ({a: dict(spec, base=run_base[a]) for a in funded},
             {a: dict(spec, base=self_base[a]) for a in funded})
    batches, queue = [], []
    phases = [(seed, scale, only, coins[0])]
    if not args.no_self_test:
        phases.insert(0, (DEFAULT_SEED, SELF_TEST_SCALE, None, coins[1]))
    for s_seed, s_scale, s_only, s_coins in phases:
        b, q = plan(funded, s_seed, s_scale, args.rust, args.shaped, only=s_only,
                    coins=s_coins, fund=True)
        for x in b:
            x["id"] += len(batches)
        batches += b
        queue += q
    progress.phase("funding", batches, jobs)
    results = run_pool(batches, queue, jobs, interpreters, progress)
    sent = []
    for b in batches:
        r = results.get(b["id"], {"status": "error", "report": "not run"})
        if r["status"] != "pass":
            return None, (f"crosscheck: FAILED: the funding of {b['area']}'s cases failed"
                          f"{case_words(r.get('case'))}: {r.get('report', r['status'])}")
        sent += r.get("funding", [])
    core_node.submit(sent)
    height = core_node.bury(max(args.shaped[a].get("fund_depth", 0) for a in funded))
    progress.log(f"funding: {total} coins, {len(sent)} funding transactions for "
                 f"{', '.join(funded)}, buried at height {height}")
    return coins, None


def self_test(args, areas, jobs, interpreters, progress, coins=None):
    """Every area must catch each operation answered wrongly that it
    names, one round for each operation an area names."""
    ok = True
    lines = []
    for k in range(max(len(sabotaged_ops(a)) for a in areas)):
        round_areas = [a for a in areas if len(sabotaged_ops(a)) > k]
        SABOTAGED.clear()
        SABOTAGED.update({a: sabotaged_ops(a)[k] for a in round_areas})
        batches, queue = plan(round_areas, DEFAULT_SEED, SELF_TEST_SCALE, args.rust, args.shaped,
                              sabotage=lambda a: SABOTAGED[a], coins=coins)
        progress.phase("self-test", batches, jobs)
        progress.log(f"self-test: {len(round_areas)} areas, {len(batches)} batches")
        results = run_pool(batches, queue, jobs, interpreters, progress)
        for area in round_areas:
            r = merge(batches, results, area)
            op = SABOTAGED[area]
            if r["status"] == "mismatch" and r["op"] == op:
                lines.append(f"self-test {area}: caught a wrong {op} ({r['seconds']} s)")
            else:
                ok = False
                lines.append(f"self-test {area}: FAILED to catch a wrong {op}: "
                             f"{r['status']} {r.get('report', '')}")
    progress.log("self-test: " + ("every area caught its wrong answer" if ok else "FAILED"))
    return ok, lines


def network_is_off():
    """Whether this process's network namespace has no interface but
    loopback, read from /proc/net/dev (which is per namespace)."""
    names = [line.split(":")[0].strip() for line in pathlib.Path("/proc/net/dev").read_text().splitlines()[2:]]
    return all(n == "lo" for n in names)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust", help="the faraday-crosscheck executable")
    ap.add_argument("--fresh", action="store_true", help="a new seed and ten times the cases")
    ap.add_argument("--seed", default=None)
    ap.add_argument("--scale", type=float, default=None)
    ap.add_argument("--area", action="append", metavar="AREA",
                    help="run only these: " + ", ".join(AREAS))
    ap.add_argument("--case", help="with one --area: only this corpus case, LOOP:INDEX")
    ap.add_argument("--jobs", type=int, default=None, help="workers (default: one a core)")
    ap.add_argument("--python", default="auto",
                    help="auto (PyPy for the areas it can run, when pypy3 is on the PATH), "
                         "cpython, pypy, or a path to PyPy")
    ap.add_argument("--progress", default=str(common.ROOT / "out" / "crosscheck"),
                    help="the folder for progress.json, progress.log and report.txt "
                         "('' for none)")
    ap.add_argument("--no-self-test", action="store_true", help=argparse.SUPPRESS)
    ap.add_argument("--log-every", type=float, default=10.0, help=argparse.SUPPRESS)
    ap.add_argument("--allow-network", action="store_true",
                    help="run although a network interface is up (no stamp is written)")
    ap.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    ap.add_argument("--shape", help=argparse.SUPPRESS)
    ap.add_argument("--alternative", action="store_true", help=argparse.SUPPRESS)
    ap.add_argument("--test-areas", help=argparse.SUPPRESS)
    args = ap.parse_args()
    if args.test_areas:
        use_test_areas(args.test_areas)
    if args.worker:
        return worker(args.alternative)
    if args.shape:
        print(json.dumps(shape(args.shape)))
        return 0
    if not args.rust:
        ap.error("--rust is required")

    if not network_is_off() and not args.allow_network:
        return refuse(args, "crosscheck: a network interface is up; run it through "
                      "tools/crosscheck/run.sh, which gives it none")
    try:
        common.check_references()
    except common.HarnessError as e:
        return refuse(args, f"crosscheck: the references are not the vendored ones: {e}")
    seed = args.seed or (secrets.token_hex(16) if args.fresh else DEFAULT_SEED)
    scale = args.scale or (10.0 if args.fresh else 1.0)
    areas = args.area or list(AREAS)
    for a in areas:
        if a not in AREAS:
            ap.error(f"--area {a}: no such area; the areas are {', '.join(AREAS)}")
    only = None
    if args.case:
        if len(areas) != 1:
            ap.error("--case needs exactly one --area")
        loop, _, i = args.case.partition(":")
        only = (loop, int(i))
    jobs = max(1, args.jobs or default_jobs())
    args.shaped = shapes(areas)
    needs = [a for a in areas if args.shaped.get(a, {}).get("core")]
    if not needs:
        return run(args, areas, jobs, only, seed, scale, None)
    # Bitcoin Core is required by these areas: no node, no run.
    for sig in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, _exit_on)
    node = core_node.Node(rpc_threads=jobs)
    try:
        core_node.loopback_up()
        node.start()
    except (core_node.CoreError, OSError, subprocess.SubprocessError) as e:
        node.stop()
        return refuse(args, f"crosscheck: {e} (needed by {', '.join(needs)})")
    try:
        os.environ.update(node.environ())
        return run(args, areas, jobs, only, seed, scale, node.info)
    finally:
        node.stop()


def _exit_on(sig, frame):
    """A stop asked for from outside ends the run through its cleanup."""
    raise SystemExit(128 + sig)


def run(args, areas, jobs, only, seed, scale, core):
    """The self-test and the run, with Bitcoin Core's node already up when
    an area needs it (`core` is what it is, for the stamp)."""
    interpreters = interpreters_for(args)
    if args.python == "auto":
        global ALTERNATIVE_FOR
        ALTERNATIVE_FOR = PYPY_FASTER
    started = time.monotonic()
    progress = Progress(args.progress or None, {
        "seed": seed, "scale": scale, "areas_run": areas,
        "python": {k: v for k, v in interpreters.items()}}, every=args.log_every)
    progress.log(f"crosscheck: seed {seed}, scale {scale}, {jobs} workers, "
                 + " and ".join(f"{k} {v}" for k, v in interpreters.items()))
    if core:
        progress.log(f"crosscheck: Bitcoin Core {core['version']} on regtest, offline "
                     f"({core['tarball']}, SHA-256 {core['sha256']})")

    report = []
    coins = ({}, {})
    if core and any(args.shaped[a].get("fund") for a in areas):
        coins, failed = fund(args, areas, jobs, interpreters, progress, seed, scale, only)
        if failed:
            report.append(failed)
            return finish(progress, report, "failed", 1)
    if not args.no_self_test:
        ok, lines = self_test(args, areas, jobs, interpreters, progress, coins[1])
        report += lines
        if not ok:
            report.append("crosscheck: the self-test failed; the cross-check cannot be trusted")
            return finish(progress, report, "failed", 1)

    report.append(f"crosscheck: seed {seed}, scale {scale}, areas {', '.join(areas)}")
    batches, queue = plan(areas, seed, scale, args.rust, args.shaped, only=only, coins=coins[0])
    progress.phase("run", batches, jobs)
    progress.log(f"run: {len(areas)} areas, {len(batches)} batches, "
                 f"{sum(cases_total(batches, a) for a in areas)} corpus cases")
    results = run_pool(batches, queue, jobs, interpreters, progress)
    failed = False
    cases = {}
    for area in areas:
        r = merge(batches, results, area)
        cases[area] = r["cases"]
        if r["status"] == "pass":
            known = f", {r['known']} of them a known difference" if r.get("known") else ""
            report.append(f"  {area}: {r['compared']} answers agree{known} ({r['seconds']} s)")
        else:
            failed = True
            report.append(f"  {area}: {r['status'].upper()} after {r['seconds']} s")
            if r.get("case"):
                report.append(f"    at case {r['case']} (alone: --seed {seed} --scale {scale}"
                              f" --area {area} --case {r['case']})"
                              if ":" in r["case"] else f"    in its {r['case']}")
            report.append("    " + r.get("report", "").replace("\n", "\n    "))
            if r["status"] == "mismatch" and args.fresh:
                report.append(keep(area, seed, scale, r))
    total = round(time.monotonic() - started, 1)
    if failed:
        report.append(f"crosscheck: FAILED ({total} s); rerun with --seed {seed} to reproduce")
        return finish(progress, report, "failed", 1)
    report.append(f"crosscheck: every answer agrees ({total} s)")
    if args.area or args.case or args.allow_network or args.no_self_test \
            or args.seed and not args.fresh and args.seed != DEFAULT_SEED:
        report.append("crosscheck: no stamp written for a partial or non-default run")
        return finish(progress, report, "passed", 0)
    import stamp
    stamp.write(seed, scale, total, jobs, list(interpreters.values()), cases, core)
    report.append(f"crosscheck: stamp written to {stamp.STAMP.relative_to(common.ROOT)}")
    return finish(progress, report, "passed", 0)


def refuse(args, text):
    """A run refused before it starts: said, and the end of the log."""
    print(text)
    if args.progress:
        Progress(args.progress, {}).log(text.replace("crosscheck: ", "crosscheck: FAILED: ", 1))
    return 2


def finish(progress, report, verdict, code):
    """Prints the report, keeps it beside the progress, and ends the
    progress with the verdict."""
    text = "\n".join(report) + "\n"
    sys.stdout.write(text)
    sys.stdout.flush()
    if progress.dir:
        (progress.dir / "report.txt").write_text(text)
    said = next(line.removeprefix("crosscheck: ") for line in reversed(report)
                if line.startswith("crosscheck: ") and ("FAILED" in line or "agrees" in line
                                                        or "self-test" in line))
    if getattr(progress, "memory_mb", 0):
        progress.log(f"crosscheck: the largest worker took {progress.memory_mb} MB")
    progress.log("crosscheck: " + (said if said.startswith("FAILED") else
                                   f"{verdict.upper()}: {said}"))
    progress.write(force=True, final=verdict)
    return code


def keep(area, seed, scale, r):
    """A disagreement found with a fresh seed is kept as a new vector."""
    d = HERE / "found"
    d.mkdir(exist_ok=True)
    path = d / f"{area}-{seed}.json"
    path.write_text(json.dumps({"area": area, "seed": seed, "scale": scale, "case": r.get("case"),
                                "op": r["op"], "inputs": r["inputs"], "faraday": r["rust"],
                                "reference": r["reference"]}, indent=1) + "\n")
    return f"    kept as {path.relative_to(common.ROOT)}"


if __name__ == "__main__":
    sys.exit(main())
