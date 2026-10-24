#!/usr/bin/env python3
"""The crypto cross-check (F79, docs/MULTIVENDOR.md §10).

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

Before the run proper, the self-test runs each area against a Rust
command told to answer one operation wrongly, and fails unless every
area catches it: a cross-check that cannot fail is worse than none.
"""

import argparse
import json
import os
import pathlib
import secrets
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import common  # noqa: E402

DEFAULT_SEED = "faraday-crosscheck-1"

# Each area, its module, and the operation the self-test makes the Rust
# command answer wrongly.
AREAS = {
    "musig": ("area_musig", "Musig", "musig.sign"),
    "frost": ("area_frost", "Frost", "frost.sign"),
    "sp": ("area_silent", "Silent", "sp.sender"),
    "bip85": ("area_bip85", "Bip85", "bip85.wif"),
    "slip39": ("area_slip39", "Slip39", "slip39.recover"),
    "codex32": ("area_codex32", "Codex32", "codex32.recover"),
    "codex32-correct": ("area_codex32_correct", "Codex32Correct", "codex32.correct"),
    "xor": ("area_xor", "Xor", "xor.combine"),
    "vault": ("area_vault", "Vault", "vault.open"),
    "bip39": ("area_bip39", "Bip39", "bip39.seed"),
    "bip32": ("area_bip32", "Bip32", "bip32.derive"),
    "addr": ("area_addr", "Addr", "addr.parse"),
    "tx": ("area_tx", "Tx", "tx.sighash"),
    "psbt": ("area_psbt", "Psbt", "psbt.v0"),
    "msg": ("area_msg", "Msg", "msg.sign"),
    "own-randomness": ("area_own", "Own", "own.session"),
    "entropy": ("area_entropy", "Entropy", "entropy.cards"),
    "roller": ("area_roller", "Roller", "roller.five"),
    "seedqr": ("area_seedqr", "SeedQR", "seedqr.encode"),
    "bbqr": ("area_bbqr", "Bbqr", "bbqr.join"),
    "ur": ("area_ur", "Ur", "ur.encode"),
    "vanity": ("area_vanity", "Vanity", "vanity.grind"),
    "kdbx": ("area_kdbx", "Kdbx", "kdbx.verify"),
    "lightning": ("area_lightning", "Lightning", "lightning.ldk"),
}


def worker(args):
    """One area in this process. The last line printed is its result."""
    module, cls, _ = AREAS[args.worker]
    started = time.monotonic()
    result = {"area": args.worker}
    try:
        common.check_references()
        area = getattr(__import__(module), cls)(args.rust, args.seed, args.scale)
        try:
            area.run()
        finally:
            area.close()
        result.update(status="pass", compared=area.compared, known=getattr(area, "known", 0))
    except common.Mismatch as m:
        result.update(status="mismatch", report=m.report(), op=m.op, inputs=m.inputs,
                      rust=m.rust, reference=m.reference)
    except Exception as e:  # noqa: BLE001 - reported, and the run fails
        import traceback
        where = "".join(traceback.format_exception(e)[-3:])
        result.update(status="error", report=f"{type(e).__name__}: {e}\n{where}")
    result["seconds"] = round(time.monotonic() - started, 1)
    print(json.dumps(result), flush=True)


def spawn(area, rust, seed, scale, sabotage=None):
    env = dict(os.environ)
    env.pop("FARADAY_CROSSCHECK_SABOTAGE", None)
    if sabotage:
        env["FARADAY_CROSSCHECK_SABOTAGE"] = sabotage
    cmd = [sys.executable, "-I", "-B", str(HERE / "crosscheck.py"), "--worker", area,
           "--rust", rust, "--seed", seed, "--scale", str(scale)]
    return subprocess.Popen(cmd, env=env, stdout=subprocess.PIPE, text=True)


def collect(procs):
    out = {}
    for area, p in procs.items():
        lines = p.stdout.read().strip().splitlines()
        p.wait()
        try:
            out[area] = json.loads(lines[-1])
        except (IndexError, json.JSONDecodeError):
            out[area] = {"area": area, "status": "error", "seconds": 0,
                         "report": "the worker printed no result"}
    return out


def self_test(rust, areas):
    """Every area must catch an operation answered wrongly."""
    procs = {a: spawn(a, rust, DEFAULT_SEED, 0.05, sabotage=AREAS[a][2]) for a in areas}
    ok = True
    for area, r in collect(procs).items():
        if r["status"] == "mismatch" and r["op"] == AREAS[area][2]:
            print(f"self-test {area}: caught a wrong {AREAS[area][2]} ({r['seconds']} s)")
        else:
            ok = False
            print(f"self-test {area}: FAILED to catch a wrong {AREAS[area][2]}: {r['status']}"
                  f" {r.get('report', '')}")
    return ok


def network_is_off():
    """Whether this process's network namespace has no interface but
    loopback, read from /proc/net/dev (which is per namespace)."""
    names = [line.split(":")[0].strip() for line in pathlib.Path("/proc/net/dev").read_text().splitlines()[2:]]
    return all(n == "lo" for n in names)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust", required=True, help="the faraday-crosscheck executable")
    ap.add_argument("--fresh", action="store_true", help="a new seed and ten times the cases")
    ap.add_argument("--seed", default=None)
    ap.add_argument("--scale", type=float, default=None)
    ap.add_argument("--area", action="append", choices=sorted(AREAS), help="run only these")
    ap.add_argument("--allow-network", action="store_true",
                    help="run although a network interface is up (no stamp is written)")
    ap.add_argument("--worker", choices=sorted(AREAS), help=argparse.SUPPRESS)
    args = ap.parse_args()
    if args.worker:
        args.seed = args.seed or DEFAULT_SEED
        args.scale = args.scale or 1.0
        return worker(args)

    if not network_is_off() and not args.allow_network:
        print("crosscheck: a network interface is up; run it through tools/crosscheck/run.sh,"
              " which gives it none")
        return 2
    try:
        common.check_references()
    except common.HarnessError as e:
        print(f"crosscheck: the references are not the vendored ones: {e}")
        return 2
    seed = args.seed or (secrets.token_hex(16) if args.fresh else DEFAULT_SEED)
    scale = args.scale or (10.0 if args.fresh else 1.0)
    areas = args.area or list(AREAS)
    started = time.monotonic()

    if not self_test(args.rust, areas):
        print("crosscheck: the self-test failed; the cross-check cannot be trusted")
        return 1

    print(f"crosscheck: seed {seed}, scale {scale}, areas {', '.join(areas)}")
    results = collect({a: spawn(a, args.rust, seed, scale) for a in areas})
    failed = False
    for area in areas:
        r = results[area]
        if r["status"] == "pass":
            known = f", {r['known']} of them a known difference" if r.get("known") else ""
            print(f"  {area}: {r['compared']} answers agree{known} ({r['seconds']} s)")
        else:
            failed = True
            print(f"  {area}: {r['status'].upper()} after {r['seconds']} s")
            print("    " + r.get("report", "").replace("\n", "\n    "))
            if r["status"] == "mismatch" and args.fresh:
                keep(area, seed, scale, r)
    total = round(time.monotonic() - started, 1)
    if failed:
        print(f"crosscheck: FAILED ({total} s); rerun with --seed {seed} to reproduce")
        return 1
    print(f"crosscheck: every answer agrees ({total} s)")
    if args.area or args.allow_network or args.seed and not args.fresh and args.seed != DEFAULT_SEED:
        print("crosscheck: no stamp written for a partial or non-default run")
        return 0
    import stamp
    stamp.write(seed, scale, total)
    print(f"crosscheck: stamp written to {stamp.STAMP.relative_to(common.ROOT)}")
    return 0


def keep(area, seed, scale, r):
    """A disagreement found with a fresh seed is kept as a new vector."""
    d = HERE / "found"
    d.mkdir(exist_ok=True)
    path = d / f"{area}-{seed}.json"
    path.write_text(json.dumps({"area": area, "seed": seed, "scale": scale, "op": r["op"],
                                "inputs": r["inputs"], "faraday": r["rust"],
                                "reference": r["reference"]}, indent=1) + "\n")
    print(f"    kept as {path.relative_to(common.ROOT)}")


if __name__ == "__main__":
    sys.exit(main())
