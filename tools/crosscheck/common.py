"""What every area of the cross-check shares: the Rust process, the
comparison, the seeded randomness and the vendored references.

Standard library only. Every vendored reference, code Faraday did not
write, is imported from `tools/reference/` after `references.sha256` has
been checked. The models Faraday's own authors wrote (`btc.py`,
`miniscript_ref.py` and the models inside the `area_*.py` files) live
here in `tools/crosscheck/`; they are not independent implementations
(F160, `docs/crypto/actions.toml`'s `independence`).
"""

import hashlib
import json
import os
import pathlib
import random
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
HERE = ROOT / "tools" / "crosscheck"
MANIFEST = HERE / "references.sha256"


class Mismatch(Exception):
    """Faraday and the reference disagree, or one of them disagrees with
    a published vector."""

    def __init__(self, area, op, inputs, rust, reference, note=""):
        super().__init__(f"{area}: {op} disagrees")
        self.area = area
        self.op = op
        self.inputs = inputs
        self.rust = rust
        self.reference = reference
        self.note = note

    def report(self):
        lines = [
            f"DISAGREEMENT in {self.area}: {self.op}",
            f"  inputs:    {json.dumps(self.inputs, sort_keys=True)}",
            f"  faraday:   {json.dumps(self.rust, sort_keys=True)}",
            f"  reference: {json.dumps(self.reference, sort_keys=True)}",
        ]
        if self.note:
            lines.append(f"  note:      {self.note}")
        return "\n".join(lines)


class HarnessError(Exception):
    """The harness itself went wrong: a malformed request, a panic, a
    reference file that does not match its hash."""


def manifest():
    """The vendored files and their SHA-256, from `references.sha256`."""
    out = {}
    for line in MANIFEST.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        digest, path = line.split(None, 1)
        out[path.strip()] = digest
    return out


def check_references():
    """Refuses to run when a vendored file does not match its recorded
    SHA-256, or when a Python file sits in a reference folder unlisted
    (it could be imported in place of a listed one)."""
    listed = manifest()
    for path, digest in listed.items():
        f = ROOT / path
        if not f.is_file():
            raise HarnessError(f"{path} is missing")
        got = hashlib.sha256(f.read_bytes()).hexdigest()
        if got != digest:
            raise HarnessError(f"{path}: SHA-256 {got}, recorded {digest}")
    dirs = {str(pathlib.Path(p).parts[2]) for p in listed if p.startswith("tools/reference/")}
    for d in sorted(dirs):
        for f in (ROOT / "tools" / "reference" / d).rglob("*.py"):
            rel = f.relative_to(ROOT).as_posix()
            if rel not in listed:
                raise HarnessError(f"{rel} is in a reference folder but not in references.sha256")


def use_reference(*parts):
    """Puts one vendored folder first on the import path."""
    sys.path.insert(0, str(ROOT.joinpath("tools", "reference", *parts)))


def vectors(*parts):
    return json.loads(ROOT.joinpath("tools", "vectors", *parts).read_text())


class Rust:
    """The Rust command, one request a line."""

    def __init__(self, exe, area, sabotage=None):
        self.area = area
        self.next_id = 0
        self.calls = 0
        env = dict(os.environ)
        env.pop("FARADAY_CROSSCHECK_SABOTAGE", None)
        if sabotage:
            env["FARADAY_CROSSCHECK_SABOTAGE"] = sabotage
        self.proc = subprocess.Popen(
            [exe], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1, env=env,
        )

    def call(self, op, **args):
        """Faraday's answer: ("ok", value) or ("err", reason)."""
        self.next_id += 1
        self.calls += 1
        req = dict(args, id=self.next_id, op=op)
        self.proc.stdin.write(json.dumps(req) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise HarnessError(f"{op}: the Rust command exited")
        out = json.loads(line)
        if out.get("id") != self.next_id:
            raise HarnessError(f"{op}: answer out of step: {line.strip()}")
        if "fatal" in out:
            raise HarnessError(f"{op}: {out['fatal']} for {json.dumps(args)}")
        if "ok" in out:
            return ("ok", out["ok"])
        return ("err", out["err"])

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


def reference(fn):
    """The reference's answer, in the same shape: ("ok", value) or
    ("err", reason) when it raises."""
    try:
        return ("ok", fn())
    except Exception as e:  # noqa: BLE001 - every refusal is a refusal
        return ("err", f"{type(e).__name__}: {e}")


class Area:
    """One area: its Rust process, its randomness, its counts.

    An area is three parts: `prelude` (the published vectors and the
    fixed cases), the corpus loops named in `LOOPS`, and `coda`. Each
    corpus case is one call of `case_<loop>()` and draws from a random
    stream of its own, `Random("<seed>:<area>:<loop>:<i>")`, so case i is
    the same whichever process runs it and whatever ran before it: the
    run cuts the loops into batches by case index and runs them on any
    number of workers (docs/CROSSCHECK.md §10.1). The prelude draws from
    `Random("<seed>:<area>")` and the coda from `"<seed>:<area>:coda"`.
    Nothing a case leaves on the area may be read by another case."""

    name = ""
    # (loop, cases at scale 1): each case is a call of `case_<loop>()`.
    LOOPS = ()

    def __init__(self, exe, seed, scale, sabotage=None):
        self.rust = Rust(exe, self.name, sabotage)
        self.run_seed = seed
        self.rng = random.Random(f"{seed}:{self.name}")
        self.scale = scale
        self.compared = 0
        self.known = 0
        self.case_at = None

    def cases(self, n):
        return max(1, int(n * self.scale))

    @classmethod
    def loops(cls, scale):
        """Each corpus loop and its number of cases at this scale."""
        return [(loop, max(1, int(n * scale))) for loop, n in cls.LOOPS]

    def randbytes(self, n):
        return self.rng.randbytes(n)

    def prelude(self):
        pass

    def coda(self):
        pass

    def case(self, loop, i):
        """Corpus case `i` of `loop`, from its own random stream."""
        self.case_at = (loop, i)
        self.rng = random.Random(f"{self.run_seed}:{self.name}:{loop}:{i}")
        getattr(self, f"case_{loop}")()

    def run_part(self, part, lo=0, hi=0):
        """One batch: the prelude, the coda, or cases lo to hi - 1 of a loop."""
        if part == "prelude":
            self.case_at = ("prelude", None)
            self.rng = random.Random(f"{self.run_seed}:{self.name}")
            self.prelude()
        elif part == "coda":
            self.case_at = ("coda", None)
            self.rng = random.Random(f"{self.run_seed}:{self.name}:coda")
            self.coda()
        else:
            for i in range(lo, hi):
                self.case(part, i)

    def run(self):
        """The whole area in one process, in order."""
        self.run_part("prelude")
        for loop, n in self.loops(self.scale):
            self.run_part(loop, 0, n)
        self.run_part("coda")

    def compare(self, op, inputs, ref_fn, expected=None, normalize=None):
        """Asks Faraday, recomputes with the reference, and stops at the
        first disagreement. `expected` is a published vector's answer:
        ("ok", value) or ("err", None) for any refusal."""
        raw = rust = self.rust.call(op, **inputs)
        ref = reference(ref_fn)
        if normalize:
            rust, ref = normalize(rust), normalize(ref)
        self.compared += 1
        if rust[0] != ref[0] or (rust[0] == "ok" and rust[1] != ref[1]):
            raise Mismatch(self.name, op, inputs, rust, ref)
        if expected is not None:
            if expected[0] != ref[0] or (expected[0] == "ok" and expected[1] != ref[1]):
                raise Mismatch(self.name, op, inputs, rust, ref,
                               f"both differ from the published vector {expected}")
        return raw

    def need(self, answer, what):
        """The value of an answer both sides gave; a refusal by both of
        an input the corpus built to be valid is a harness bug."""
        if answer[0] != "ok":
            raise AssertionError(f"{what}: both refused a valid input: {answer[1]}")
        return answer[1]

    def close(self):
        self.rust.close()


def as_verify(answer):
    """A refusal to verify and a failed verification are the same answer."""
    if answer[0] == "err":
        return ("ok", {"valid": False})
    return answer
