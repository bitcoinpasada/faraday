#!/usr/bin/env python3
"""The software-round pass of the aezeed (area `lightning`) runs only on
builds that are what they say (`run.sh`):

    soft.py check RUST RUST_SOFT LND LND_SOFT

stops (exit 1, a line each) unless Faraday's soft command holds no AES-NI
instruction while the ordinary one does (x86-64; elsewhere the area asks
the command which backend it was built with), and LND's soft program was
built with the tags noasm and purego and lacks the assembly's cpuid
routine that the ordinary one holds. Prints one line when all hold.
Standard library only; run as `python3 -I -B`."""

import platform
import re
import subprocess
import sys

AESNI = re.compile(r"\s(v?aes(enc|dec)(last)?)\s")


def aesni_count(path):
    """The AES-NI instructions in an executable, as binutils' objdump
    disassembles it (a byte search finds data that only looks like one)."""
    try:
        out = subprocess.run(["objdump", "-d", "--no-show-raw-insn", path], capture_output=True,
                             text=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError) as e:
        raise SystemExit(f"soft: objdump (binutils) is needed to read {path}: {e}")
    return sum(1 for line in out.splitlines() if AESNI.search(line))


def go_tags(path):
    out = subprocess.run(["go", "version", "-m", path], capture_output=True, text=True,
                         check=True).stdout
    for line in out.splitlines():
        if "-tags=" in line:
            return line.split("-tags=")[1].strip().split(",")
    return []


def go_has_cpuid(path):
    out = subprocess.run(["go", "tool", "nm", path], capture_output=True, text=True,
                         check=True).stdout
    return "cpuidAMD64" in out


def check(rust, rust_soft, lnd, lnd_soft):
    bad = []
    tags = go_tags(lnd_soft)
    if not {"noasm", "purego"} <= set(tags):
        bad.append(f"soft: LND's soft program was not built with the tags noasm and purego ({tags})")
    if platform.machine() in ("x86_64", "AMD64"):
        soft, hard = aesni_count(rust_soft), aesni_count(rust)
        if soft:
            bad.append(f"soft: the soft Rust command holds {soft} AES-NI instructions")
        if not hard:
            bad.append("soft: the ordinary Rust command holds no AES-NI instruction, "
                       "so this check cannot tell the two apart")
        if go_has_cpuid(lnd_soft):
            bad.append("soft: LND's soft program holds the assembly's cpuid routine")
        if not go_has_cpuid(lnd):
            bad.append("soft: LND's ordinary program holds no assembly, so this check "
                       "cannot tell the two apart")
        detail = f"{hard} AES-NI instructions in the ordinary command, {soft} in the soft one"
    else:
        detail = "no AES-NI instruction check on this machine"
    return bad, detail


def main():
    if len(sys.argv) != 6 or sys.argv[1] != "check":
        raise SystemExit(__doc__)
    bad, detail = check(*sys.argv[2:])
    if bad:
        print("\n".join(bad))
        raise SystemExit(1)
    print(f"soft AES round in Faraday's command and LND without assembly ({detail})")


if __name__ == "__main__":
    main()
