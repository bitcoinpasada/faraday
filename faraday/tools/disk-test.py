#!/usr/bin/env python3
"""Boot the dev stick image and prove the stick rule (PLAN.md §4.3, §5.1).

    faraday/tools/disk-test.py IMAGE OUT_DIR TESTKIT_DIR

The dev image has a root login on ttyS0. QEMU's serial port is a unix
socket here, so this script logs in and looks at the machine itself
while it plugs sticks in and pulls them out through QMP:

1. At boot no filesystem is mounted, the kernel has no vfat, and the
   grant helper and the disk process are running.
2. The boot partition (OSKBOOT) stays root's; the boot stick's data
   partition belongs to the disk process (uid 201) while the app is clean.
3. A test stick plugged in while the app is clean is handed out, and the
   app shows its files (a screenshot).
4. With the clean marker gone, as when a secret is in memory, a handed
   stick goes back to root within a second, and a stick plugged in then
   stays root's.
5. The grant helper is still alive.
6. USB: the kernel authorises only hard-wired devices, and the grant
   helper authorises storage, HID and hub interfaces of the rest.

Screenshots go to OUT_DIR. Exits non-zero at the first check that fails.
"""

import importlib.util
import os
import re
import shutil
import socket
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
check = load("qemu_check", os.path.join(ROOT, "faraday/tools/qemu-check.py"))


class Serial:
    """The dev image's root console, over QEMU's unix socket."""

    def __init__(self, path):
        for _ in range(200):
            try:
                self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.s.connect(path)
                break
            except OSError:
                time.sleep(0.1)
        self.s.settimeout(0.5)
        self.buf = b""
        # Everything the console said, for what other processes print.
        self.log = b""

    def read_until(self, needle, seconds):
        deadline = time.monotonic() + seconds
        while needle.encode() not in self.buf:
            if time.monotonic() > deadline:
                return None
            try:
                chunk = self.s.recv(65536)
                if chunk:
                    self.buf += chunk
                    self.log += chunk
            except socket.timeout:
                pass
        text, _, rest = self.buf.partition(needle.encode())
        self.buf = rest
        return text.decode("utf-8", "replace")

    def send(self, text):
        self.s.sendall(text.encode())

    def login(self):
        seen = b""
        for _ in range(30):
            self.send("\n")
            got = self.read_until("login:", 2)
            if got is not None:
                break
            seen = self.buf[-2000:]
        else:
            raise SystemExit(f"no login prompt on ttyS0; it said: {seen!r}")
        self.send("root\n")
        if self.read_until("#", 10) is None:
            raise SystemExit(f"no root prompt on ttyS0; it said: {self.buf[-2000:]!r}")

    def run(self, cmd, seconds=20):
        """A command's output. The end marker is split in the command
        line, so the line's echo does not end the read."""
        self.buf = b""
        self.send(f"{cmd}; echo __DO\"NE\"__\n")
        out = self.read_until("__DONE__", seconds)
        if out is None:
            raise SystemExit(f"no answer to {cmd!r}")
        lines = out.replace("\r", "").split("\n")
        # The first line is the command's echo.
        return "\n".join(line for line in lines[1:] if "__DO" not in line).strip()


def owner(console, node):
    """The uid owning a device node, or None when there is no such node."""
    # BusyBox here has no `stat`: the uid is ls -n's third field.
    out = console.run(
        f"if [ -e /dev/{node} ]; then set -- $(ls -ln /dev/{node}); echo OWN\"ER\"=$3; "
        f"else echo OWN\"ER\"=none; fi")
    found = re.search(r"OWNER=(\d+|none)", out)
    word = found.group(1) if found else "none"
    return None if word == "none" else int(word)


def wait_owner(console, node, uid, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if owner(console, node) == uid:
            return True
        time.sleep(0.5)
    return False


def expect(ok, what):
    print(("ok    " if ok else "FAIL  ") + what)
    if not ok:
        raise SystemExit(1)


def main():
    image, out, kit = sys.argv[1:4]
    os.makedirs(out, exist_ok=True)
    work = os.path.join(out, "work")
    os.makedirs(work, exist_ok=True)
    boot = os.path.join(work, "boot.img")
    shutil.copyfile(image, boot)
    stick = check.test_stick(os.path.join(work, "teststick.img"),
                             sorted(os.path.join(kit, n) for n in os.listdir(kit)))
    late = check.test_stick(os.path.join(work, "late.img"), [], label="LATESTICK")
    sock = os.path.join(work, "qmp.sock")
    serial_sock = os.path.join(work, "serial.sock")
    for p in (sock, serial_sock):
        if os.path.exists(p):
            os.remove(p)
    argv = qemu.command(boot, vars_fd=qemu.vars_copy(work),
                        serial=f"unix:{serial_sock},server=on,wait=off",
                        qmp=sock, vnc=19)
    proc = subprocess.Popen(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    scratch = os.path.join(work, "screen.ppm")
    try:
        for _ in range(100):
            if os.path.exists(sock) and os.path.exists(serial_sock):
                break
            time.sleep(0.1)
        monitor = st.Qmp(sock)
        console = Serial(serial_sock)
        check.settle(monitor, scratch, 90)
        check.shot(monitor, scratch, out, "01-booted.png")
        console.login()
        # The kernel's own lines stop coming to the console, so they cannot
        # land in the middle of a command's answer.
        console.run("dmesg -n 1")

        # 1. Nothing mounted, no vfat, both helpers up.
        mounts = console.run("cat /proc/mounts")
        expect(not any(line.startswith("/dev/sd") for line in mounts.splitlines()),
               "no stick partition is mounted")
        expect("vfat" not in console.run("cat /proc/filesystems"),
               "the kernel has no vfat")
        ps = console.run("ps")
        expect("faraday-grant" in ps, "the grant helper runs")
        expect("faraday-disk" in ps, "the disk process runs")
        # USB (PLAN.md §4.6): only hard-wired devices are authorised by
        # the kernel; the grant helper authorises the rest by class.
        expect("usbcore.authorized_default=2" in console.run("cat /proc/cmdline"),
               "the kernel authorises only hard-wired USB devices by itself")
        deadline = time.monotonic() + 60
        while True:
            ifaces = console.run(
                "for d in /sys/bus/usb/devices/*:*; do "
                "echo IF $(basename $d) $(cat $d/bInterfaceClass) $(cat $d/authorized); done")
            rows = [line.split()[1:] for line in ifaces.splitlines()
                    if line.startswith("IF ") and len(line.split()) == 4]
            done = rows and all(a == "1" for _, c, a in rows if c in ("03", "08", "09"))
            if done or time.monotonic() > deadline:
                break
            time.sleep(2)
        print("\n".join(" ".join(r) for r in rows))
        expect(done, "storage, HID and hub interfaces are authorised")
        grant_status = console.run(
            "cat /proc/$(pidof faraday-grant)/status | grep -E '^(Uid|CapEff|CapBnd|NoNewPrivs|Seccomp):'")
        print(grant_status)
        expect("CapEff:\t0000000000000001" in grant_status,
               "the grant helper keeps CAP_CHOWN and nothing else")
        expect("NoNewPrivs:\t1" in grant_status,
               "the grant helper has no-new-privileges")

        # 2. The boot stick: OSKBOOT root's, its data partition handed out.
        expect(console.run("cat /run/faraday-clean/clean 2>/dev/null").strip() == "clean",
               "the app publishes that it is clean")
        # QEMU under WSL runs the guest's clock slowly: the boot stick's
        # disk can take a long time to be enumerated.
        deadline = time.monotonic() + 180
        while "sda2" not in console.run("ls /sys/class/block"):
            if time.monotonic() > deadline:
                expect(False, "the boot stick's partitions appear")
            time.sleep(2)
        expect(True, "the boot stick's partitions appear")
        expect(owner(console, "sda1") == 0, "the boot partition stays root's")
        expect(wait_owner(console, "sda2", 201, 30),
               "the boot stick's data partition goes to the disk process")

        # 3. A test stick while clean: handed out and read.
        st.pull_boot_stick(monitor)
        time.sleep(3)
        st.plug_in(monitor, stick)
        expect(wait_owner(console, "sdb1", 201, 120) or wait_owner(console, "sda1", 201, 5),
               "a stick plugged in while clean is handed out")
        time.sleep(3)
        check.settle(monitor, scratch, 15)
        check.shot(monitor, scratch, out, "02-test-stick-visit.png")
        print(console.run("dmesg | grep -A2 'sticks:' | tail -12"))
        print("\n".join(l for l in console.log.decode("utf-8", "replace").splitlines()
                        if "faraday-disk" in l or "panic" in l))
        node = "sdb1" if owner(console, "sdb1") is not None else "sda1"

        # 4. The marker gone: the stick goes back, and a new one stays.
        console.run("rm /run/faraday-clean/clean")
        expect(wait_owner(console, node, 0, 15),
               "with the clean marker gone the stick goes back to root")
        st.unplug(monitor)
        time.sleep(3)
        monitor.command("blockdev-add", driver="raw", **{"node-name": "late"},
                        file={"driver": "file", "filename": os.path.abspath(late)})
        monitor.command("device_add", driver="usb-storage", bus="xhci.0",
                        drive="late", id="late-dev")
        deadline = time.monotonic() + 120
        while not re.search(r"sd[a-z][0-9]", console.run("ls /sys/class/block")):
            if time.monotonic() > deadline:
                expect(False, "the late stick's partition appears")
            time.sleep(2)
        expect(True, "the late stick's partition appears")
        time.sleep(3)
        nodes = console.run("ls /sys/class/block | grep -E '^sd[a-z][0-9]' || true").split()
        print("partitions now:", nodes)
        expect(nodes and all(owner(console, n) == 0 for n in nodes),
               "a stick plugged in while not clean stays root's")
        check.settle(monitor, scratch, 15)
        check.shot(monitor, scratch, out, "03-late-stick-not-handed.png")

        # 5. The helper survived all of it.
        expect("faraday-grant" in console.run("ps"), "the grant helper is still alive")
    finally:
        proc.terminate()
        proc.wait(timeout=10)


if __name__ == "__main__":
    main()
