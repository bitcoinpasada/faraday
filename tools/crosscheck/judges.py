#!/usr/bin/env python3
"""The outside programs `just crosscheck` needs (docs/ESTABLISHED.md §9.2, rule 4).

    judges.py check      exit 1 and name every missing tool; print nothing else
    judges.py versions   the stamp's lines, `<tool> <version>` each

The judged test files (tests that skip without these tools under plain
`just`) run with FARADAY_JUDGES_REQUIRED=1 under `just crosscheck`, and the
OVMF Secure Boot run (faraday/tools/sb-ovmf-check.py) needs QEMU and the
firmware. The tools are looked for as the tests look: FARADAY_FATTOOLS,
/usr/sbin, /sbin, then the PATH.
"""

import os
import pathlib
import shutil
import subprocess
import sys

# (tool, the package that has it on Debian or Ubuntu, the version command;
# None where the tool has none and the package's version stands for it)
TOOLS = [
    ("mkfs.fat", "dosfstools", ["--version"]),
    ("fsck.fat", "dosfstools", ["-V"]),
    ("fatlabel", "dosfstools", ["--version"]),
    ("mcopy", "mtools", ["-V"]),
    ("mmd", "mtools", ["-V"]),
    ("mdir", "mtools", ["-V"]),
    ("sfdisk", "util-linux", ["--version"]),
    ("sgdisk", "gdisk", ["--version"]),
    ("xz", "xz-utils", ["--version"]),
    ("qemu-system-x86_64", "qemu-system-x86", ["--version"]),
    ("certtool", "gnutls-bin", ["--version"]),
    ("pesign", "pesign", None),
    ("pesigcheck", "pesign", None),
    ("zbarimg", "zbar-tools", ["--version"]),
    ("sq", "sq", ["version"]),
]
# OVMF's Secure Boot firmware, as faraday/tools/sb-ovmf-check.py looks.
OVMF_DIRS = ["/usr/share/edk2/x64", "/usr/share/edk2-ovmf/x64", "/usr/share/OVMF"]
OVMF_PAIRS = [
    ("OVMF_CODE.secboot.4m.fd", "OVMF_VARS.4m.fd"),
    ("OVMF_CODE_4M.secboot.fd", "OVMF_VARS_4M.fd"),
]
# The image the OVMF run signs and boots.
SYSTEMD_BOOT = "/usr/lib/systemd/boot/efi/systemd-bootx64.efi"


def find(name):
    d = os.environ.get("FARADAY_FATTOOLS")
    if d:
        p = pathlib.Path(d) / name
        if p.is_file():
            return str(p)
    for d in ("/usr/sbin", "/sbin"):
        p = pathlib.Path(d) / name
        if p.is_file():
            return str(p)
    return shutil.which(name)


def sevenzip():
    """7-Zip's own program, `7zz` or `7z` (some systems name it so), or None.

    Accepted only when its banner reads `7-Zip <version> ... Igor Pavlov`;
    p7zip, the old fork (`7-Zip [64]`, `p7zip Version`), is not 7-Zip here.
    """
    for name in ("7zz", "7z"):
        path = find(name)
        if not path:
            continue
        run = subprocess.run([path], capture_output=True, text=True)
        text = (run.stdout + run.stderr).strip()
        if (text.startswith("7-Zip ") and not text.startswith("7-Zip [")
                and "Igor Pavlov" in text and "p7zip" not in text):
            return path
    return None


def ovmf():
    for d in OVMF_DIRS:
        for code, vars_ in OVMF_PAIRS:
            if os.path.exists(os.path.join(d, code)) and os.path.exists(os.path.join(d, vars_)):
                return os.path.join(d, code)
    return None


def first_line(path, args, package=None):
    if args is None:
        run = subprocess.run(["dpkg-query", "-W", "-f", "${Version}", package],
                             capture_output=True, text=True)
        return f"{package} {run.stdout.strip()}" if run.returncode == 0 else "unknown"
    run = subprocess.run([path, *args], capture_output=True, text=True)
    text = (run.stdout + run.stderr).strip().splitlines()
    return text[0].strip() if text else "unknown"


def sq_backend():
    """The line of `sq version` that names Sequoia's cryptographic backend
    ("with cryptographic backend Nettle 3.10 ..."), or None when `sq` is
    absent or says no such line. The GPG tests judge Faraday with `sq`
    built over Nettle (ESTABLISHED §9.7, batch X24), not over Sequoia's
    own pure-Rust backend."""
    path = find("sq")
    if not path:
        return None
    run = subprocess.run([path, "version"], capture_output=True, text=True)
    for line in (run.stdout + run.stderr).splitlines():
        if "cryptographic backend" in line:
            return line.strip()
    return None


def pefile_version():
    """python3-pefile's version, or None when Python cannot import it."""
    run = subprocess.run(
        [sys.executable, "-I", "-c", "import pefile; print(pefile.__version__)"],
        capture_output=True, text=True)
    return run.stdout.strip() if run.returncode == 0 else None


def zxingcpp_version():
    """python3-zxing-cpp's package version, or None when Python cannot
    import it (the module has no version of its own)."""
    run = subprocess.run([sys.executable, "-I", "-c", "import zxingcpp"], capture_output=True)
    if run.returncode != 0:
        return None
    got = subprocess.run(["dpkg-query", "-W", "-f", "${Version}", "python3-zxing-cpp"],
                         capture_output=True, text=True)
    return got.stdout.strip() if got.returncode == 0 and got.stdout.strip() else "unknown"


def cbor2_version():
    """python3-cbor2's package version, or None when Python cannot import it."""
    run = subprocess.run([sys.executable, "-I", "-c", "import cbor2"], capture_output=True)
    if run.returncode != 0:
        return None
    got = subprocess.run(["dpkg-query", "-W", "-f", "${Version}", "python3-cbor2"],
                         capture_output=True, text=True)
    return got.stdout.strip() if got.returncode == 0 and got.stdout.strip() else "unknown"


def missing():
    out = [f"{tool} (package {pkg})" for tool, pkg, _ in TOOLS if not find(tool)]
    if find("sq") and "cryptographic backend Nettle" not in (sq_backend() or ""):
        out.append("sq built over Nettle (`sq version` must say \"cryptographic backend "
                   "Nettle\"; package sq on Debian or Ubuntu)")
    if not sevenzip():
        out.append("7-Zip (`7zz` or `7z`; package 7zip, not p7zip)")
    if not pefile_version():
        out.append("pefile for Python (package python3-pefile)")
    if not zxingcpp_version():
        out.append("zxingcpp for Python (package python3-zxing-cpp)")
    if not cbor2_version():
        out.append("cbor2 for Python (package python3-cbor2)")
    if not ovmf():
        out.append("OVMF's Secure Boot firmware, OVMF_CODE.secboot.4m.fd or "
                   "OVMF_CODE_4M.secboot.fd and its empty variable store "
                   "(package ovmf, or edk2-ovmf)")
    if not os.path.exists(SYSTEMD_BOOT):
        out.append(f"systemd-boot's {SYSTEMD_BOOT} (package systemd-boot-efi)")
    return out


def versions():
    lines = []
    for tool, pkg, args in TOOLS:
        path = find(tool)
        lines.append(f"{tool} {first_line(path, args, pkg) if path else 'missing'}")
    if find("sq"):
        lines.append(f"sq backend {sq_backend() or 'unknown'}")
    seven = sevenzip()
    lines.append(f"7-Zip (7zz or 7z) {first_line(seven, []) if seven else 'missing'}")
    lines.append(f"python3-pefile {pefile_version() or 'missing'}")
    lines.append(f"python3-zxing-cpp {zxingcpp_version() or 'missing'}")
    lines.append(f"python3-cbor2 {cbor2_version() or 'missing'}")
    code = ovmf()
    if code:
        digest = subprocess.run(["sha256sum", code],
                                capture_output=True, text=True).stdout.split()[0]
        lines.append(f"ovmf {os.path.basename(code)} {digest}")
    else:
        lines.append("ovmf missing")
    return "".join(line + "\n" for line in lines)


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    if cmd == "versions":
        sys.stdout.write(versions())
        return 0
    gone = missing()
    for g in gone:
        print(f"crosscheck: missing judge: {g}", file=sys.stderr)
    if gone:
        print("(docs/DEVELOPING.md §1, prerequisites; docs/ESTABLISHED.md §9.2 rule 4)",
              file=sys.stderr)
    return 1 if gone else 0


if __name__ == "__main__":
    sys.exit(main())
