#!/usr/bin/env python3
"""Boot the Faraday stick image in QEMU, the way a laptop boots it.

The point of the arrangement below is that nothing is virtualised that a
laptop would not have. OVMF is the firmware; the image is a USB disk on an
xHCI controller, so the firmware finds `EFI/BOOT/BOOTX64.EFI` on the first
FAT partition of a removable disk and runs it, which is the path the stick
takes on real hardware; the keyboard and the mouse are USB devices. There
is no virtio anything and no virtual hard disk.

  tools/stick-qemu.py --image out/stick/faraday-x86_64-uefi.img
        the image, display on VNC 127.0.0.1:5909
  tools/stick-qemu.py --image out/stick/faraday-x86_64-uefi-dev.img
        the dev image, whose console is on stdio

`--usb IMAGE` attaches a second USB disk at start, which is how a person
tries the file channel with a stick that is not the boot stick. Any raw
disk image with a FAT partition on it will do.

faraday/tools' QEMU checks (qemu-check.py, disk-test.py,
upgrade-check.py) import `command` and the QMP helpers below from here,
so that the machine they check is the machine the owner looks at.

VNC listens on the loopback address only. The way to see it from another
machine is an SSH tunnel to this one; the image README says how.
"""

import argparse
import json
import os
import shutil
import socket
import struct
import subprocess
import sys
import time
import zlib

# OVMF as the distribution installs it: the firmware code, which is
# read-only, and a variable store, which every machine needs its own
# writable copy of.
OVMF_CODE = "/usr/share/OVMF/OVMF_CODE_4M.fd"
OVMF_VARS = "/usr/share/OVMF/OVMF_VARS_4M.fd"


def vars_copy(out_dir):
    """A private copy of the OVMF variable store, made if it is missing.

    The firmware writes its boot entries and its screen mode here. A
    shared copy would mean one run's firmware settings leaking into the
    next, and the file the distribution ships is not writable anyway.
    """
    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, "OVMF_VARS.fd")
    if not os.path.exists(path):
        shutil.copyfile(OVMF_VARS, path)
    return path


def command(image, *, vars_fd, serial, qmp=None, vnc=None, kvm=True,
            usb=()):
    """The QEMU command line, as a list.

    `serial` is a `-serial` argument: "stdio" for a person, or
    "file:PATH" for a test that reads the console afterwards. `qmp` is a
    unix socket path, `vnc` a display number. `usb` is further disk
    images, each attached as its own USB mass-storage device, the way a
    second stick in a second port is.
    """
    argv = ["qemu-system-x86_64"]
    if kvm:
        argv += ["-enable-kvm"]
    argv += [
        # q35 is a PC made this decade: PCIe, no ISA bridge to speak of,
        # and the machine type OVMF is built for.
        "-machine", "q35",
        "-m", "1G",
        "-drive", f"if=pflash,format=raw,readonly=on,file={OVMF_CODE}",
        "-drive", f"if=pflash,format=raw,file={vars_fd}",
        # The image as a USB disk. `if=none` keeps QEMU from attaching it
        # to a controller of its own; the usb-storage device below is
        # what puts it on the xHCI bus.
        "-drive", f"file={image},format=raw,if=none,id=stick",
        # The controller comes first: a USB device is realised when its
        # -device argument is read, and there is no bus to put it on
        # until the controller exists. q35 has no USB controller of its
        # own.
        "-device", "qemu-xhci,id=xhci",
        "-device", "usb-storage,drive=stick,bus=xhci.0,id=stick-dev",
        "-device", "usb-kbd,bus=xhci.0",
        "-device", "usb-mouse,bus=xhci.0",
        "-display", "none",
        "-serial", serial,
    ]
    for n, disk in enumerate(usb):
        argv += [
            "-drive", f"file={disk},format=raw,if=none,id=usb{n}",
            "-device", f"usb-storage,drive=usb{n},bus=xhci.0",
        ]
    if vnc is not None:
        # Loopback only. Reaching it from another machine is an SSH
        # tunnel, not an open port.
        argv += ["-vnc", f"127.0.0.1:{vnc}"]
    if qmp is not None:
        argv += ["-qmp", f"unix:{qmp},server,nowait"]
    return argv


# ---- driving the machine over QMP, for faraday/tools' QEMU checks ----

# The largest step a USB mouse can report in one packet: QEMU's usb-mouse
# packs each axis into a signed byte, so a longer move is several packets.
MOUSE_STEP = 120


class Qmp:
    """The QEMU monitor, over its unix socket."""

    def __init__(self, path, timeout=60.0):
        deadline = time.monotonic() + timeout
        while True:
            try:
                self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.sock.connect(path)
                break
            except OSError:
                if time.monotonic() > deadline:
                    raise
                time.sleep(0.1)
        self.sock.settimeout(timeout)
        self.buf = b""
        self.read()                       # the greeting
        self.command("qmp_capabilities")

    def read(self):
        """The next JSON object, skipping the asynchronous events."""
        while True:
            while b"\n" in self.buf:
                line, self.buf = self.buf.split(b"\n", 1)
                if not line.strip():
                    continue
                message = json.loads(line)
                if "event" in message:
                    continue
                return message
            chunk = self.sock.recv(65536)
            if not chunk:
                raise EOFError("the QEMU monitor closed")
            self.buf += chunk

    def command(self, name, **arguments):
        request = {"execute": name}
        if arguments:
            request["arguments"] = arguments
        self.sock.sendall(json.dumps(request).encode() + b"\n")
        reply = self.read()
        if "error" in reply:
            raise RuntimeError(f"{name}: {reply['error']['desc']}")
        return reply.get("return")

    def send_input(self, *events):
        self.command("input-send-event", events=list(events))

    def move(self, dx, dy):
        """Move the pointer, in packets a USB mouse could have sent."""
        while dx or dy:
            step_x = max(-MOUSE_STEP, min(MOUSE_STEP, dx))
            step_y = max(-MOUSE_STEP, min(MOUSE_STEP, dy))
            events = []
            if step_x:
                events.append({"type": "rel",
                               "data": {"axis": "x", "value": step_x}})
            if step_y:
                events.append({"type": "rel",
                               "data": {"axis": "y", "value": step_y}})
            self.send_input(*events)
            dx -= step_x
            dy -= step_y
            time.sleep(0.02)

    def move_to(self, x, y, width, height):
        """Put the pointer at an absolute place.

        A USB mouse reports movement and not position, so there is no such
        thing; what there is, is a cursor that clamps at the edges of the
        display. Far enough up and left is 0,0 whatever the cursor did
        before, and the move after that starts from somewhere known.
        """
        self.move(-width, -height)
        self.move(x, y)

    def click(self):
        self.send_input({"type": "btn",
                         "data": {"down": True, "button": "left"}})
        time.sleep(0.08)
        self.send_input({"type": "btn",
                         "data": {"down": False, "button": "left"}})

    def key(self, name):
        self.send_input({"type": "key",
                         "data": {"down": True,
                                  "key": {"type": "qcode", "data": name}}})
        time.sleep(0.05)
        self.send_input({"type": "key",
                         "data": {"down": False,
                                  "key": {"type": "qcode", "data": name}}})

    def screendump(self, path):
        # The old file goes first. QEMU truncates and rewrites in place,
        # and the wait below is for a file that is there and parses — so
        # with the previous dump still in place it can be satisfied by
        # that one, and the caller compares a frame against itself.
        if os.path.exists(path):
            os.unlink(path)
        self.command("screendump", filename=os.path.abspath(path))
        # screendump returns before the file is closed on some builds.
        deadline = time.monotonic() + 10.0
        while time.monotonic() < deadline:
            if os.path.exists(path) and os.path.getsize(path) > 0:
                try:
                    return read_ppm(path)
                except ValueError:
                    pass
            time.sleep(0.1)
        raise RuntimeError(f"{path}: no screendump was written")


def read_ppm(path):
    """A binary PPM as (width, height, pixel bytes)."""
    with open(path, "rb") as f:
        data = f.read()
    fields = []
    i = 0
    while len(fields) < 4:
        while i < len(data) and data[i : i + 1].isspace():
            i += 1
        if data[i : i + 1] == b"#":
            while i < len(data) and data[i] != 0x0A:
                i += 1
            continue
        start = i
        while i < len(data) and not data[i : i + 1].isspace():
            i += 1
        if i >= len(data):
            raise ValueError("truncated PPM header")
        fields.append(data[start:i])
    if fields[0] != b"P6":
        raise ValueError(f"not a binary PPM: {fields[0]!r}")
    width, height = int(fields[1]), int(fields[2])
    pixels = data[i + 1 :]
    if len(pixels) < width * height * 3:
        raise ValueError("truncated PPM")
    return width, height, pixels[: width * height * 3]


def write_png(path, width, height, pixels):
    """The same frame as a PNG, so that it can simply be opened.

    QEMU writes PPM and nothing else. A PPM is fine to compare and
    awkward to look at, and the point of keeping these frames is that
    somebody looks at them.
    """
    raw = b"".join(b"\x00" + pixels[y * width * 3 : (y + 1) * width * 3]
                   for y in range(height))

    def chunk(kind, data):
        body = kind + data
        return (struct.pack(">I", len(data)) + body
                + struct.pack(">I", zlib.crc32(body)))

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", header))
        f.write(chunk(b"IDAT", zlib.compress(raw, 6)))
        f.write(chunk(b"IEND", b""))


def differs(a, b):
    """How many of two frames' pixels differ, as a fraction."""
    if len(a) != len(b):
        return 1.0
    changed = sum(1 for i in range(0, len(a), 3)
                  if a[i : i + 3] != b[i : i + 3])
    return changed / (len(a) / 3)


def plug_in(monitor, path):
    """Attach a disk image as a second USB stick, while the machine runs."""
    monitor.command("blockdev-add", driver="raw", **{"node-name": "plugged"},
                    file={"driver": "file", "filename": os.path.abspath(path)})
    monitor.command("device_add", driver="usb-storage", bus="xhci.0",
                    drive="plugged", id="plugged-dev")


def unplug(monitor):
    """Pull that stick out again, as a person does without warning."""
    monitor.command("device_del", id="plugged-dev")


def pull_boot_stick(monitor):
    """Pull the stick the machine booted from. The kernel and the root
    filesystem are in memory, so the app keeps running; only the exchange
    partition goes."""
    monitor.command("device_del", id="stick-dev")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True, help="the .img to boot")
    parser.add_argument("--out", default=None,
                        help="where the OVMF variable store goes "
                             "(default: beside the image)")
    parser.add_argument("--vnc", type=int, default=9,
                        help="VNC display number; 9 is port 5909")
    parser.add_argument("--serial", default="stdio",
                        help="a -serial argument (default stdio)")
    parser.add_argument("--qmp", default=None, help="a QMP unix socket path")
    parser.add_argument("--no-kvm", action="store_true",
                        help="emulate rather than use /dev/kvm")
    parser.add_argument("--usb", action="append", default=[],
                        metavar="IMAGE",
                        help="a further disk image to attach as a USB "
                             "stick; may be given more than once")
    args = parser.parse_args()

    if not os.path.exists(args.image):
        sys.exit(f"{args.image}: no such image. Run `just faraday-stick-image` first.")
    for path in args.usb:
        if not os.path.exists(path):
            sys.exit(f"{path}: no such disk image to plug in.")
    for f in (OVMF_CODE, OVMF_VARS):
        if not os.path.exists(f):
            sys.exit(f"{f}: no such file. OVMF is not installed.")

    out_dir = args.out or os.path.dirname(os.path.abspath(args.image))
    argv = command(
        args.image,
        vars_fd=vars_copy(out_dir),
        serial=args.serial,
        qmp=args.qmp,
        vnc=args.vnc,
        kvm=not args.no_kvm,
        usb=[os.path.abspath(p) for p in args.usb],
    )
    print(" ".join(argv), file=sys.stderr)
    print(f"display: VNC on 127.0.0.1:{5900 + args.vnc}", file=sys.stderr)
    return subprocess.call(argv)


if __name__ == "__main__":
    sys.exit(main())
