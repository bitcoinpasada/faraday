#!/usr/bin/env python3
"""Secure Boot, end to end in real UEFI firmware (PLAN.md §8, the test).

    faraday/tools/sb-ovmf-check.py OUT_DIR [IMAGE.efi]

For each enrolment policy, Faraday's own keys alone and Windows-compatible:

1. `faraday-sb` makes test keys and their enrolment files, and signs
   IMAGE.efi with the db key (`examples/ovmf_kit.rs`). IMAGE.efi is
   systemd-boot unless another is named: it shows a menu the serial
   console carries, and it enrols keys itself.
2. QEMU boots OVMF in Setup Mode from a FAT ESP holding the signed image
   and the three `.auth` updates under `loader/keys/auto`. systemd-boot
   writes them into the firmware as authenticated variables, as a
   firmware's own setup screen does, and the firmware takes them only if
   PK is signed by PK, KEK by PK and db by KEK.
3. With Secure Boot now enforced, the ESP's image is swapped: the
   unsigned image and the signed one altered by one byte must each be
   refused ("Access Denied"); the signed one must run (its menu shows);
   and the unsigned one must still be refused after it.

Each boot's serial output is kept in OUT_DIR. Exit status 0 only if every
step held. QEMU, OVMF's Secure Boot build (`OVMF_CODE.secboot.4m.fd`) and
mtools are needed; nothing is downloaded.
"""

import os
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

OVMF_DIRS = ["/usr/share/edk2/x64", "/usr/share/edk2-ovmf/x64", "/usr/share/OVMF"]
SYSTEMD_BOOT = "/usr/lib/systemd/boot/efi/systemd-bootx64.efi"
ENROLLED = b"successfully enrolled"
DENIED = b"Access Denied"
MENU = b"Reboot Into Firmware"


def need(path_or_tool):
    found = path_or_tool if os.path.exists(path_or_tool) else shutil.which(path_or_tool)
    if not found:
        sys.exit(f"sb-ovmf-check: {path_or_tool} is not here")
    return found


def ovmf():
    for d in OVMF_DIRS:
        code = os.path.join(d, "OVMF_CODE.secboot.4m.fd")
        vars_ = os.path.join(d, "OVMF_VARS.4m.fd")
        if os.path.exists(code) and os.path.exists(vars_):
            return code, vars_
    sys.exit("sb-ovmf-check: no OVMF Secure Boot build (OVMF_CODE.secboot.4m.fd)")


def esp(path, kit):
    """A 64 MiB FAT32 ESP: the signed image, loader.conf, the updates."""
    with open(path, "wb") as f:
        f.truncate(64 * 1024 * 1024)
    subprocess.check_call([need("mformat"), "-i", path, "-F", "::"])
    for d in ["EFI", "EFI/BOOT", "loader", "loader/keys", "loader/keys/auto"]:
        subprocess.check_call([need("mmd"), "-i", path, "::" + d])
    conf = os.path.join(kit, "loader.conf")
    with open(conf, "w") as f:
        # No entry is ever chosen by itself: a choice would leave the
        # firmware set to open its own menu at the next boot.
        f.write("timeout menu-force\nsecure-boot-enroll force\n")
    put(path, conf, "loader/loader.conf")
    for name in ["PK", "KEK", "db"]:
        put(path, os.path.join(kit, "loader/keys/auto", name + ".auth"), f"loader/keys/auto/{name}.auth")
    put(path, os.path.join(kit, "signed.efi"), "EFI/BOOT/BOOTX64.EFI")


def put(image, src, dst):
    subprocess.check_call([need("mcopy"), "-o", "-i", image, src, "::" + dst])


def boot(code, vars_, image, log, seconds):
    """Boots once; the firmware's variables persist in `vars_`. A reboot
    ends QEMU; otherwise it is stopped after `seconds`."""
    cmd = [
        need("qemu-system-x86_64"),
        "-machine", "q35,smm=on",
        "-global", "driver=cfi.pflash01,property=secure,value=on",
        "-drive", f"if=pflash,format=raw,unit=0,file={code},readonly=on",
        "-drive", f"if=pflash,format=raw,unit=1,file={vars_}",
        "-drive", f"file={image},format=raw,if=virtio",
        "-display", "none",
        "-m", "512",
        "-no-reboot",
        "-serial", f"file:{log}",
    ]
    try:
        subprocess.run(cmd, timeout=seconds, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        pass
    with open(log, "rb") as f:
        return f.read()


def run(policy, out, payload, code, vars_template):
    kit = os.path.join(out, policy)
    os.makedirs(kit, exist_ok=True)
    args = ["cargo", "run", "-q", "--release", "-p", "faraday-sb", "--example", "ovmf_kit", "--", kit, payload]
    if policy == "windows":
        args.append("windows")
    subprocess.check_call(args, cwd=ROOT, stdout=subprocess.DEVNULL)
    image = os.path.join(kit, "esp.img")
    esp(image, kit)
    vars_ = os.path.join(kit, "vars.fd")
    shutil.copyfile(vars_template, vars_)
    results = []

    def step(what, ok):
        results.append(ok)
        print(f"{'ok  ' if ok else 'FAIL'} {policy}: {what}")

    said = boot(code, vars_, image, os.path.join(kit, "enrol.txt"), 120)
    step("the firmware takes PK, KEK and db from the .auth updates", ENROLLED in said)
    for which, want_run in [("unsigned", False), ("altered", False), ("signed", True), ("unsigned", False)]:
        put(image, os.path.join(kit, which + ".efi"), "EFI/BOOT/BOOTX64.EFI")
        said = boot(code, vars_, image, os.path.join(kit, f"boot-{which}.txt"), 30)
        ran, denied = MENU in said, DENIED in said
        if want_run:
            step("the image signed with the db key runs", ran and not denied)
        else:
            step(f"the {which} image is refused", denied and not ran)
    return all(results)


def main():
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__.strip().splitlines()[2].strip())
    out = os.path.abspath(sys.argv[1])
    payload = os.path.abspath(sys.argv[2]) if len(sys.argv) == 3 else need(SYSTEMD_BOOT)
    code, vars_template = ovmf()
    os.makedirs(out, exist_ok=True)
    ok = all([run(p, out, payload, code, vars_template) for p in ["own", "windows"]])
    print("PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
