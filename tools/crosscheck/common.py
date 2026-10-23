"""What every area of the cross-check shares: the Rust process, the
comparison, the seeded randomness and the vendored references.

Standard library only. Every reference is imported from
`tools/reference/`, after `references.sha256` has been checked.
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

    def __init__(self, exe, area):
        self.area = area
        self.next_id = 0
        self.calls = 0
        self.proc = subprocess.Popen(
            [exe], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1,
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
    """One area's run: its Rust process, its randomness, its counts."""

    name = ""

    def __init__(self, exe, seed, scale):
        self.rust = Rust(exe, self.name)
        self.rng = random.Random(f"{seed}:{self.name}")
        self.scale = scale
        self.compared = 0

    def cases(self, n):
        return max(1, int(n * self.scale))

    def randbytes(self, n):
        return self.rng.randbytes(n)

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

    def run(self):
        raise NotImplementedError

    def close(self):
        self.rust.close()


def as_verify(answer):
    """A refusal to verify and a failed verification are the same answer."""
    if answer[0] == "err":
        return ("ok", {"valid": False})
    return answer
