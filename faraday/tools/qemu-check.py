#!/usr/bin/env python3
"""Boot the Faraday stick image headless and photograph it.

    faraday/tools/qemu-check.py [--no-kvm] IMAGE OUT_DIR TESTKIT_DIR

QEMU boots IMAGE the way faraday/tools/stick-qemu.py boots it (OVMF, the image
as a USB disk on xHCI), and the screen is saved as a PNG at each stage:
the first screen, with the boot stick in; after the boot stick is pulled;
with a test stick holding TESTKIT_DIR's files plugged in; and after that
stick is pulled. The QMP helpers are upstream's, from faraday/tools/stick-qemu.py.
The test stick is made with mtools: Buildroot's own, from the stick
build's host directory (out/pi/x86_64-uefi-efi-framebuffer[-dev]),
or the system's.

--no-kvm emulates the CPU instead of using /dev/kvm, for a machine without
it; every wait is then four times as long.

The serial port is read as well (the release image's kernel has no console on
it, so no kernel line may arrive), and the app is sent Ctrl-Z and Ctrl-C on
it, after which it must still answer: plugging the test stick in changes its
screen. The check fails (exit 1) when either does not hold.
"""

import argparse
import glob

import importlib.util
import os
import re
import shutil
import socket
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


qemu = load("stick_qemu", os.path.join(ROOT, "faraday/tools/stick-qemu.py"))
# The QMP helpers, which are upstream's, from faraday/tools/stick-qemu.py.
st = qemu

BG = (0x11, 0x16, 0x1C)
# Buildroot's host tools, from the first stick build tree that has them.
MTOOLS = sorted(glob.glob(os.path.join(
    ROOT, "out/pi/x86_64-uefi-efi-framebuffer-*/output/host/bin")))
# How much longer a wait is under software emulation.
SLOW = 1


def tool(name):
    for directory in MTOOLS:
        local = os.path.join(directory, name)
        if os.path.exists(local):
            return local
    return shutil.which(name)


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


def listen(port):
    """What the serial port has said since the last look."""
    got = b""
    while True:
        try:
            chunk = port.recv(65536)
        except socket.timeout:
            return got
        if not chunk:
            return got
        got += chunk


def settle(monitor, scratch, seconds):
    """The screen once it shows Faraday's background and stops changing."""
    last = None
    deadline = time.monotonic() + seconds * SLOW
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
    global SLOW
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-kvm", action="store_true",
                        help="emulate rather than use /dev/kvm (4 times the waits)")
    parser.add_argument("image")
    parser.add_argument("out")
    parser.add_argument("kit")
    args = parser.parse_args()
    image, out, kit = args.image, args.out, args.kit
    if args.no_kvm:
        SLOW = 4
    os.makedirs(out, exist_ok=True)
    work = os.path.join(out, "work")
    os.makedirs(work, exist_ok=True)
    boot = os.path.join(work, "boot.img")
    shutil.copyfile(image, boot)
    stick = test_stick(os.path.join(work, "teststick.img"),
                       sorted(os.path.join(kit, n) for n in os.listdir(kit)))
    sock = os.path.join(work, "qmp.sock")
    serial = os.path.join(work, "serial.sock")
    if os.path.exists(serial):
        os.remove(serial)
    argv = qemu.command(boot, vars_fd=qemu.vars_copy(work),
                        serial=f"unix:{serial},server=on,wait=off", qmp=sock, vnc=19, kvm=not args.no_kvm)
    proc = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    scratch = os.path.join(work, "screen.ppm")
    failures = []
    try:
        for _ in range(100):
            if os.path.exists(sock) and os.path.exists(serial):
                break
            time.sleep(0.1)
        monitor = st.Qmp(sock)
        port = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        port.connect(serial)
        port.settimeout(0.2)
        heard = b""
        settle(monitor, scratch, 90)
        heard += listen(port)
        shot(monitor, scratch, out, "01-booted.png")
        st.pull_boot_stick(monitor)
        time.sleep(4 * SLOW)
        _, _, before = settle(monitor, scratch, 15)
        shot(monitor, scratch, out, "02-boot-stick-pulled.png")
        # A byte on the serial pins is no key for the app: Ctrl-Z and Ctrl-C
        # leave it running and drawing.
        port.sendall(b"\x1a\x03")
        time.sleep(2 * SLOW)
        heard += listen(port)
        st.plug_in(monitor, stick)
        time.sleep(6 * SLOW)
        _, _, after = settle(monitor, scratch, 15)
        heard += listen(port)
        if st.differs(before, after) == 0:
            failures.append("the screen did not change when the test stick was "
                            "plugged in after Ctrl-Z and Ctrl-C on the serial port")
        shot(monitor, scratch, out, "03-test-stick-in.png")
        st.unplug(monitor)
        time.sleep(4 * SLOW)
        settle(monitor, scratch, 15)
        shot(monitor, scratch, out, "04-test-stick-pulled.png")
        heard += listen(port)
        # OVMF prints its own lines before the kernel starts; a kernel line
        # carries a [seconds] stamp, and the app's own output is not on this port.
        kernel = [l for l in heard.decode("utf-8", "replace").splitlines()
                  if re.match(r"\s*\[\s*\d+\.\d+\]", l)]
        if kernel:
            failures.append("kernel lines on the serial port: " + repr(kernel[:3]))
    finally:
        proc.terminate()
        proc.wait(timeout=10)
    for f in failures:
        print("FAIL:", f)
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
