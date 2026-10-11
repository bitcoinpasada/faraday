#!/usr/bin/env python3
"""Upgrade an older Faraday stick from a new one in QEMU, and boot it.

    faraday/tools/upgrade-check.py NEW_DEV_IMAGE OLD_IMAGE OUT_DIR

PLAN.md §5.5's test, as far as this harness reaches. NEW_DEV_IMAGE is a
dev stick image (`just dev=1 faraday-stick-image`): its serial console
has a root login, which is the only way in. OLD_IMAGE is an older stick
image, dev or release.

1. The older stick gets a vault-sized file and a settings file on its
   data partition (mtools), and its data partition's SHA-256 is taken.
2. QEMU boots the new image, the way tools/stick-qemu.py does, and the
   older stick is plugged in once it is up, as a second USB stick.
3. On the console, as root: the boot partitions are root's; root writes
   the upgrade marker the app would write (`/run/faraday-clean/upgrade`)
   beside the clean marker the app wrote; faraday-grant hands both boot
   partitions to `ofboot` and neither to `ofdisk`; then
   `faraday-boot --ask` speaks the app's own requests over the boot
   copier's pipes: list, read the source, write the older stick, list
   again. The marker is removed and the partitions go back to root.
4. The older stick's boot partition must now be the new image's, byte for
   byte, and its data partition what it was.
5. QEMU boots the upgraded stick alone; `/proc/version` must carry the
   new image's release string, and the app's first screen is saved.

What it does not exercise: the app's Upgrade screen. Every USB keyboard
and mouse in QEMU is an external device to Faraday, held back until it
types a code read off the screen (PLAN.md §4.6), so the harness cannot
press anything; root on the dev console stands in for the app's marker
and the app's requests. The QMP helpers and the machine are upstream's
(tools/stick-test.py, tools/stick-qemu.py); mtools are Buildroot's, from
the stick build's host directory, or the system's.
"""

import hashlib
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


qemu = load("stick_qemu", os.path.join(ROOT, "tools/stick-qemu.py"))
st = load("stick_test", os.path.join(ROOT, "tools/stick-test.py"))

MTOOLS = os.path.join(ROOT, "out/pi/x86_64-uefi-efi-framebuffer/output/host/bin")
PIPES = "/run/faraday-boot/requests /run/faraday-boot/responses"


def tool(name):
    local = os.path.join(MTOOLS, name)
    return local if os.path.exists(local) else shutil.which(name)


def partitions(path):
    """The GPT's partitions as {name: (offset, length)} in bytes."""
    with open(path, "rb") as f:
        f.seek(512)
        head = f.read(92)
        assert head[:8] == b"EFI PART", f"{path}: no GPT"
        entries_lba, count, size = struct.unpack_from("<QII", head, 72)
        f.seek(entries_lba * 512)
        table = f.read(count * size)
    out = {}
    for i in range(count):
        e = table[i * size:(i + 1) * size]
        if e[:16] == bytes(16):
            continue
        first, last = struct.unpack_from("<QQ", e, 32)
        name = e[56:128].decode("utf-16-le").rstrip("\0")
        out[name] = (first * 512, (last - first + 1) * 512)
    return out


def region(path, span):
    offset, length = span
    with open(path, "rb") as f:
        f.seek(offset)
        return f.read(length)


def release_in(data):
    m = re.search(rb"(\d+\.\d+[0-9A-Za-z.+_-]*-faraday-[0-9A-Za-z.+_-]+) \(", data)
    return m.group(1).decode() if m else None


class Console:
    """The serial port, as a socket: lines typed, output read back."""

    def __init__(self, path, log):
        deadline = time.monotonic() + 30
        while True:
            try:
                self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.sock.connect(path)
                break
            except OSError:
                if time.monotonic() > deadline:
                    raise
                time.sleep(0.2)
        self.sock.settimeout(0.5)
        self.buf = ""
        self.log = log
        self.n = 0

    def pump(self):
        try:
            chunk = self.sock.recv(65536)
        except socket.timeout:
            return
        if chunk:
            text = chunk.decode("utf-8", "replace")
            self.buf += text
            self.log.write(text)
            self.log.flush()

    def wait_for(self, pattern, seconds):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            m = re.search(pattern, self.buf, re.S)
            if m:
                self.buf = self.buf[m.end():]
                return m
            self.pump()
        raise RuntimeError(f"console: no {pattern!r} within {seconds}s")

    def send(self, line):
        self.sock.sendall(line.encode() + b"\r")

    def login(self, seconds):
        """Root on the dev getty, which has no password."""
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.send("")
            try:
                self.wait_for(r"login: ", 5)
                break
            except RuntimeError:
                continue
        self.send("root")
        self.wait_for(r"# ", 30)
        # Kernel messages go to this console too; they stay out of the
        # commands' output.
        self.run("dmesg -n 1")

    def run(self, command, seconds=120):
        """A command's output, between two marks the shell prints: the
        marks are arithmetic, so the echoed command line never matches."""
        self.n += 1
        a, b = 1000 + self.n * 2, 1001 + self.n * 2
        self.send(f"echo @@$(({a}-0))@@; {command}; echo @@$(({b}-0))@@")
        m = self.wait_for(rf"@@{a}@@\r?\n(.*?)@@{b}@@", seconds)
        return m.group(1).replace("\r", "").strip()


def black_share(pixels):
    """How much of a frame is the console's black."""
    n = len(pixels) // 3
    step = 3 * 97
    hits = sum(1 for i in range(0, len(pixels), step) if pixels[i:i + 3] == b"\0\0\0")
    return hits / max(1, n // 97)


def settle(monitor, scratch, seconds):
    """The screen once it has left the console's black and stopped
    changing, or as it is when the time is up."""
    last = None
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        time.sleep(1.0)
        w, h, now = monitor.screendump(scratch)
        if last is not None and st.differs(last, now) == 0 and black_share(now) < 0.2:
            break
        last = now
    return w, h, now


def owners(con, nodes):
    out = con.run("ls -ln " + " ".join(nodes))
    # BusyBox ls colours its names on a terminal.
    out = re.sub(r"\x1b\[[0-9;]*m", "", out)
    print(out)
    got = {}
    for line in out.splitlines():
        f = line.split()
        if len(f) >= 4 and f[-1].startswith("/dev/"):
            got[f[-1]] = int(f[2])
    return got


def boot(image, work, name, usb=()):
    sock = os.path.join(work, f"{name}-serial.sock")
    qmp = os.path.join(work, f"{name}-qmp.sock")
    for p in (sock, qmp):
        if os.path.exists(p):
            os.unlink(p)
    vars_dir = os.path.join(work, f"{name}-vars")
    argv = qemu.command(image, vars_fd=qemu.vars_copy(vars_dir),
                        serial=f"unix:{sock},server=on,wait=off", qmp=qmp,
                        vnc=None, usb=usb)
    proc = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return proc, sock, qmp


def main():
    new, old, out = sys.argv[1:4]
    os.makedirs(out, exist_ok=True)
    work = os.path.join(out, "work")
    os.makedirs(work, exist_ok=True)
    failures = []

    def check(ok, what):
        print(("ok   " if ok else "FAIL ") + what)
        if not ok:
            failures.append(what)

    boot_img = os.path.join(work, "new.img")
    target = os.path.join(work, "target.img")
    shutil.copyfile(new, boot_img)
    shutil.copyfile(old, target)
    new_parts = partitions(boot_img)
    old_parts = partitions(target)
    print(f"new image partitions {new_parts}")
    print(f"old image partitions {old_parts}")
    source = region(boot_img, new_parts["OSKBOOT"])
    running = release_in(source)
    print(f"new image release {running}")
    print(f"old image release {release_in(region(target, old_parts['OSKBOOT']))}")
    check(running is not None, "the new image's kernel carries a Faraday release string")

    # The older stick's data partition: a vault-sized file and settings.
    data_at = old_parts["OSKDATA"][0]
    vault = os.path.join(work, "vault.ofv")
    with open(vault, "wb") as f:
        f.write(os.urandom(4 << 20))
    settings = os.path.join(work, "faraday-settings.txt")
    with open(settings, "w") as f:
        f.write("theme=nord\n")
    for path in (vault, settings):
        subprocess.check_call([tool("mcopy"), "-o", "-i", f"{target}@@{data_at}", path, "::"])
    data_before = hashlib.sha256(region(target, old_parts["OSKDATA"])).hexdigest()
    print(f"old stick data partition sha256 {data_before}")

    log = open(os.path.join(out, "console-upgrade.txt"), "w")
    proc, sock, qmp_path = boot(boot_img, work, "upgrade")
    try:
        con = Console(sock, log)
        con.login(180)
        version = con.run("cat /proc/version")
        print(f"running: {version}")
        check(con.run("pidof opensigner-pi") != "", "the new stick runs the app")
        check(running is not None and f"Linux version {running} " in version,
              "the running kernel is the new image's release")
        monitor = st.Qmp(qmp_path)
        st.plug_in(monitor, target)
        time.sleep(8)
        print(con.run("cat /proc/partitions"))
        print(con.run("ls -l /run/faraday-clean"))
        nodes = ["/dev/sda1", "/dev/sda2", "/dev/sdb1", "/dev/sdb2"]
        before = owners(con, nodes)
        print(f"owners, clean marker only: {before}")
        check(before.get("/dev/sda1") == 0 and before.get("/dev/sdb1") == 0,
              "without the upgrade marker both boot partitions are root's")
        listed = con.run(f"faraday-boot --ask list {PIPES}")
        print(f"copier lists, no marker: {listed!r}")
        check(listed == "", "without the upgrade marker the copier has no partition")

        # Root writes the marker the app writes on the Upgrade screen.
        con.run("touch /run/faraday-clean/upgrade; sleep 2")
        during = owners(con, nodes)
        print(f"owners, upgrading: {during}")
        check(during.get("/dev/sda1") == 203 and during.get("/dev/sdb1") == 203,
              "with the upgrade marker both boot partitions are ofboot's (203)")
        check(during.get("/dev/sda2") != 203 and during.get("/dev/sdb2") != 203,
              "no data partition goes to ofboot")
        listed = con.run(f"faraday-boot --ask list {PIPES}")
        print(listed)
        ids = re.findall(r"^part (\S+) size (\d+) release (\S+)", listed, re.M)
        check(len(ids) == 2, "the copier lists both boot partitions")
        read = con.run(f"faraday-boot --ask read {PIPES}")
        print(read)
        check(read.startswith("Source") and "sda1@" in read,
              "the boot stick is read as the source")
        old_id = next((i for i, _, r in ids if i.startswith("sdb1@")), None)
        old_release = next((r for i, _, r in ids if i.startswith("sdb1@")), None)
        check(old_release == "none", "the older stick shows no release string")
        wrote = con.run(f"faraday-boot --ask write {old_id} {PIPES}", 600)
        print(wrote)
        check(wrote.startswith("Written") and running is not None and running in wrote,
              "the older stick is written, read back and matched")
        listed = con.run(f"faraday-boot --ask list {PIPES}")
        print(listed)
        check(f"release {running} source false" in listed,
              "the older stick now lists as the new release")
        con.run("rm /run/faraday-clean/upgrade; sleep 2")
        after = owners(con, nodes)
        print(f"owners, marker gone: {after}")
        check(after.get("/dev/sda1") == 0 and after.get("/dev/sdb1") == 0,
              "with the marker gone both boot partitions are root's again")
        con.send("sync; poweroff -f")
        time.sleep(3)
    finally:
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            proc.terminate()
            proc.wait(timeout=10)
        log.close()

    upgraded_boot = region(target, partitions(target)["OSKBOOT"])
    check(partitions(target) == old_parts, "the older stick's partition table is unchanged")
    check(upgraded_boot[:len(source)] == source,
          "the older stick's boot partition is the new image's, byte for byte")
    data_after = hashlib.sha256(region(target, old_parts["OSKDATA"])).hexdigest()
    print(f"old stick data partition sha256 after {data_after}")
    check(data_after == data_before, "the older stick's data partition is byte for byte what it was")

    log = open(os.path.join(out, "console-upgraded.txt"), "w")
    proc, sock, qmp_path = boot(target, work, "upgraded")
    try:
        con = Console(sock, log)
        con.login(180)
        version = con.run("cat /proc/version")
        print(f"upgraded stick runs: {version}")
        check(running is not None and f"Linux version {running} " in version,
              "the upgraded stick boots the new release")
        app = con.run("pidof opensigner-pi")
        check(app != "", "the upgraded stick runs the app")
        # The app's first screen: drawn over the console's black, and
        # still. With QEMU's keyboard plugged in it is the sheet asking
        # for that keyboard's code (PLAN.md §4.6).
        monitor = st.Qmp(qmp_path)
        scratch = os.path.join(work, "screen.ppm")
        w, h, px = settle(monitor, scratch, 90)
        check(black_share(px) < 0.2, "the upgraded stick shows the app")
        shot = os.path.join(out, "upgraded-first-screen.png")
        st.write_png(shot, w, h, px)
        print(shot)
        con.send("poweroff -f")
        time.sleep(3)
    finally:
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            proc.terminate()
            proc.wait(timeout=10)
        log.close()

    print()
    if failures:
        print(f"{len(failures)} failed")
        return 1
    print("all passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
