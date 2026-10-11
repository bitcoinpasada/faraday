#!/usr/bin/env python3
"""Boot the Faraday stick image headless and photograph it.

    faraday/tools/qemu-check.py IMAGE OUT_DIR TESTKIT_DIR

QEMU boots IMAGE the way tools/stick-qemu.py boots it (OVMF, the image
as a USB disk on xHCI), and the screen is saved as a PNG at each stage:
the first screen, with the boot stick in; after the boot stick is pulled;
with a test stick holding TESTKIT_DIR's files plugged in; and after that
stick is pulled. The QMP helpers are upstream's, from tools/stick-test.py.
The test stick is made with mtools: Buildroot's own, from the stick
build's host directory, or the system's.
"""

import importlib.util
import os
import shutil
import struct
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


qemu = load("stick_qemu", os.path.join(ROOT, "tools/stick-qemu.py"))
st = load("stick_test", os.path.join(ROOT, "tools/stick-test.py"))

BG = (0x11, 0x16, 0x1C)
MTOOLS = os.path.join(ROOT, "out/pi/x86_64-uefi-efi-framebuffer/output/host/bin")


def tool(name):
    local = os.path.join(MTOOLS, name)
    return local if os.path.exists(local) else shutil.which(name)


def test_stick(path, files, label="TESTSTICK"):
    """A 32 MiB disk with one FAT partition holding `files`."""
    start, sectors = 2048, 63488
    mbr = bytearray(512)
    mbr[450] = 0x0C
    struct.pack_into("<II", mbr, 454, start, sectors)
    mbr[510:512] = b"\x55\xaa"
    with open(path, "wb") as f:
        f.truncate((start + sectors) * 512)
        f.seek(0)
        f.write(mbr)
    at = f"{path}@@{start * 512}"
    subprocess.check_call([tool("mformat"), "-i", at, "-F", "-v", label, "::"])
    for name in files:
        subprocess.check_call([tool("mcopy"), "-i", at, name, "::"])
    return path


def bg_share(pixels):
    n = len(pixels) // 3
    hits = sum(1 for i in range(0, len(pixels), 3 * 97)
               if tuple(pixels[i:i + 3]) == BG)
    return hits / max(1, n // 97)


def settle(monitor, scratch, seconds):
    """The screen once it shows Faraday's background and stops changing."""
    last = None
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        time.sleep(1.0)
        w, h, now = monitor.screendump(scratch)
        if last is not None and st.differs(last, now) == 0 and bg_share(now) > 0.3:
            return w, h, now
        last = now
    return w, h, now


def shot(monitor, scratch, out, name):
    w, h, px = monitor.screendump(scratch)
    path = os.path.join(out, name)
    st.write_png(path, w, h, px)
    print(path)


def main():
    image, out, kit = sys.argv[1:4]
    os.makedirs(out, exist_ok=True)
    work = os.path.join(out, "work")
    os.makedirs(work, exist_ok=True)
    boot = os.path.join(work, "boot.img")
    shutil.copyfile(image, boot)
    stick = test_stick(os.path.join(work, "teststick.img"),
                       sorted(os.path.join(kit, n) for n in os.listdir(kit)))
    sock = os.path.join(work, "qmp.sock")
    serial = os.path.join(work, "serial.txt")
    argv = qemu.command(boot, vars_fd=qemu.vars_copy(work),
                        serial=f"file:{serial}", qmp=sock, vnc=19)
    proc = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    scratch = os.path.join(work, "screen.ppm")
    try:
        for _ in range(100):
            if os.path.exists(sock):
                break
            time.sleep(0.1)
        monitor = st.Qmp(sock)
        settle(monitor, scratch, 90)
        shot(monitor, scratch, out, "01-booted.png")
        st.pull_boot_stick(monitor)
        time.sleep(4)
        settle(monitor, scratch, 15)
        shot(monitor, scratch, out, "02-boot-stick-pulled.png")
        st.plug_in(monitor, stick)
        time.sleep(6)
        settle(monitor, scratch, 15)
        shot(monitor, scratch, out, "03-test-stick-in.png")
        st.unplug(monitor)
        time.sleep(4)
        settle(monitor, scratch, 15)
        shot(monitor, scratch, out, "04-test-stick-pulled.png")
    finally:
        proc.terminate()
        proc.wait(timeout=10)


if __name__ == "__main__":
    main()
