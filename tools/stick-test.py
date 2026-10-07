#!/usr/bin/env python3
"""Boot the stick image in QEMU and prove it reaches the app and takes input.

What a person would check on a laptop, checked over QMP instead:

  1. the app draws. The screen stops being the firmware's and the boot
     logo's, settles, is not black, and carries the action bar's orange —
     so a working screen is up and not a panic or a blank framebuffer;
  2. the mouse arrives. The first screen's action bar is clicked, which
     on a first boot goes This device -> Start here, and Esc from there
     is Home; then the first row of Home is clicked and the frame
     changes, which is a pointer landing on a 48 dp row and not just
     somewhere;
  3. the keyboard arrives. Esc goes back, and the frame is Home again;
  4. a second stick is read. A disk image with one FAT partition holding
     one `test.psbt` is plugged in over QMP, the way a person plugs a
     stick into the next port, and init mounts it. The mount is what is
     checked, not the Files screen: driving the app to Sign -> Read a
     file is more than these helpers can do. Init says on the kernel log
     what it mounted and where, so the dev variant's console carries the
     line; the release image prints nothing anywhere, and there the disk
     is only attached. Then it is pulled out again, and init says it
     unmounted it.

Then the cases the watcher's rules cover which nothing had watched it do,
each making its own sticks and reading off the console what init did with
them: two sticks with the same label, a stick with no label, labels a
computer can write and a directory name cannot hold, an ext4 partition
and a FAT filesystem with no partition table, and a stick pulled out
while it is being read.

Each step writes a screendump. Their paths are printed, because a frame
that changed is not the same as a frame that changed correctly, and the
only way to know which screen it opened is to look.

  just stick-test           the release image
  just dev=1 stick-test     the dev image

The dev image prints what the shell found on ttyS0 when it starts, and
the test waits for that line and reports it. Neither variant depends on
it for timing: the app's first frame is waited for by watching the
screen, because the shell says it has started a moment before it has
anything to draw.

Nothing outside the standard library, and no dependency on the host
beyond QEMU, OVMF and /dev/kvm.
"""

import argparse
import importlib.util
import json
import os
import shutil
import socket
import struct
import subprocess
import sys
import time
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))

# The QEMU command is the one `just stick-qemu` uses, so the machine this
# test proves is the machine the owner looks at.
_spec = importlib.util.spec_from_file_location(
    "stick_qemu", os.path.join(HERE, "stick-qemu.py")
)
stick_qemu = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(stick_qemu)

# Two colours from core/osk-ui/src/color.rs, Theme::DARK: the accent, which
# the action bar's primary button is filled with, and the surface, which a
# list row is filled with. A frame with none of the first is not a working
# screen; the first band of the second, to the right of the sidebar, is the
# first row of a list.
ACCENT = (0xFF, 0x9F, 0x0A)
SURFACE = (0x1C, 0x1C, 0x1E)

# How far a channel may be from one of those and still count as it. The
# framebuffer is 32-bit and the screendump is taken from the same pixels,
# so this is nearly exact; the slack is for a depth conversion on a
# machine whose firmware picked 16-bit.
TOLERANCE = 8

# A frame counts as a different screen when this much of it changed. Well
# above the cursor, which is 12 x 19 px, and well below any screen change.
CHANGED = 0.02

# The largest step a USB mouse can report in one packet: QEMU's usb-mouse
# packs each axis into a signed byte, so a longer move is several packets.
MOUSE_STEP = 120

# The second stick: what its partition is labelled, what is on it, and how
# long init may take to notice it. The watcher looks once a second, and the
# kernel takes a moment to enumerate a mass-storage device and read its
# partition table.
PLUGGED_LABEL = "KINGSTON"
PLUGGED_FILE = "test.psbt"
PLUGGED_SECONDS = 30.0
# How long the sticks are watched for reads once everything has settled.
# The watcher polls /sys once a second, so a stick read every second
# would show a handful of reads in this; a quiet stick shows none.
QUIET_SECONDS = 8.0

# The sticks the named cases below make for themselves. 32 MiB is the
# smallest disk `mkfs.fat -F 16` will put a filesystem on at the default
# cluster size; the files are sparse, so a handful of them costs the box
# nothing. The partition starts where a computer's partitioner puts the
# first one.
CASE_SECTORS = 65536
CASE_START = 2048

# How long a case waits for a mount or an unmount. The watcher looks once
# a second; the rest is the kernel enumerating a mass-storage device and
# reading its partition table.
CASE_SECONDS = 30.0

# How long a stick that must not be mounted is watched before the case
# passes. Long enough that a mount would have happened.
IGNORED_SECONDS = 15.0

# The label `mkfs.fat` is given, and which a hostile label is written
# over afterwards: eleven characters, the whole of a FAT label field, so
# that the replacement is the same length and both copies of it — the one
# in the boot sector and the volume-label entry in the root directory —
# are found by searching the image for it.
PLACEHOLDER = b"PLACEHOLDR "

# The labels of case 3. A FAT label field is eleven bytes, so the longest
# label there can be is eleven characters; what a longer one would do
# cannot be asked of a FAT filesystem at all.
HOSTILE_LABELS = [
    ("two dots", b"..         "),
    ("a slash", b"A/B../C    "),
    ("spaces", b"  SP ACE   "),
    ("non-ASCII", b"\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9\xc3\xa9X"),
    ("eleven dots", b"..........."),
]


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


def is_colour(pixels, i, colour):
    return (abs(pixels[i] - colour[0]) <= TOLERANCE
            and abs(pixels[i + 1] - colour[1]) <= TOLERANCE
            and abs(pixels[i + 2] - colour[2]) <= TOLERANCE)


def count_colour(pixels, colour):
    return sum(1 for i in range(0, len(pixels), 3)
               if is_colour(pixels, i, colour))


def lit_pixels(pixels):
    """How many pixels are not black."""
    return sum(1 for i in range(0, len(pixels), 3)
               if pixels[i] or pixels[i + 1] or pixels[i + 2])


def differs(a, b):
    """How many of two frames' pixels differ, as a fraction."""
    if len(a) != len(b):
        return 1.0
    changed = sum(1 for i in range(0, len(a), 3)
                  if a[i : i + 3] != b[i : i + 3])
    return changed / (len(a) / 3)


def accent_center(width, pixels):
    """The middle of the action bar's primary button.

    The primary button is the only large field of the accent colour on a
    working screen, so the centre of every accent pixel is inside it.
    """
    xs = ys = n = 0
    for i in range(0, len(pixels), 3):
        if is_colour(pixels, i, ACCENT):
            p = i // 3
            xs += p % width
            ys += p // width
            n += 1
    if not n:
        return None
    return xs // n, ys // n


def first_row_center(width, height, pixels):
    """The middle of the first row of the list on screen.

    A row is a rectangle filled with the surface colour. On `wide` the
    sidebar is a column of them down the left, so the left quarter of the
    display is ignored and the topmost band of surface colour to the right
    of it is the first row of the list. Read off the frame rather than
    computed from the layout: a test that recomputed the layout would pass
    on a screen nobody could use.
    """
    left = width // 4
    in_row = []
    for y in range(height):
        base = y * width * 3
        n = sum(1 for x in range(left, width)
                if is_colour(pixels, base + x * 3, SURFACE))
        in_row.append(n > width // 8)
    try:
        top = in_row.index(True)
    except ValueError:
        return None
    bottom = top
    while bottom + 1 < height and in_row[bottom + 1]:
        bottom += 1
    xs = [x for x in range(left, width)
          if is_colour(pixels, ((top + bottom) // 2) * width * 3 + x * 3,
                       SURFACE)]
    if not xs:
        return None
    return (min(xs) + max(xs)) // 2, (top + bottom) // 2


def plugged_image(path):
    """A raw disk image with one FAT partition holding one `test.psbt`.

    What a person would have: a stick partitioned by some computer, with
    a transaction on it. `mformat` and `mcopy` make one if mtools is on
    the box; otherwise the same thing is written here, because a FAT16
    filesystem with one file in it is a few hundred bytes of header and
    a directory entry, and the test should not need a package installed.

    The partition table matters: init looks at `/sys/class/block/*/partition`,
    so a disk with no partitions on it is a disk it never probes.
    """
    sector = 512
    start = 2048                      # sectors, the usual first partition
    sectors = 20480                   # 10 MiB, comfortably FAT16
    per_cluster = 4
    reserved = 1
    fats = 2
    root_entries = 512
    root_sectors = root_entries * 32 // sector
    fat_sectors = 20                  # covers the 5101 clusters below
    data_start = reserved + fats * fat_sectors + root_sectors
    clusters = (sectors - data_start) // per_cluster
    payload = b"psbt\xff" + b"a transaction a person put on a stick\n"

    if shutil.which("mformat") and shutil.which("mcopy"):
        return mtools_image(path, start, sectors, payload)

    boot = bytearray(sector)
    boot[0:3] = b"\xeb\x3c\x90"
    boot[3:11] = b"mkfs.fat"
    struct.pack_into("<HBHBHHBHHHII", boot, 11,
                     sector, per_cluster, reserved, fats, root_entries,
                     sectors, 0xF8, fat_sectors, 32, 8, start, 0)
    boot[36] = 0x80
    boot[38] = 0x29
    boot[39:43] = b"\x4f\x53\x4b\x21"
    boot[43:54] = PLUGGED_LABEL.encode().ljust(11, b" ")
    boot[54:62] = b"FAT16   "
    boot[510:512] = b"\x55\xaa"

    used = max(1, -(-len(payload) // (sector * per_cluster)))
    fat = bytearray(fat_sectors * sector)
    struct.pack_into("<HH", fat, 0, 0xFFF8, 0xFFFF)
    for n in range(used):
        nxt = 0xFFFF if n == used - 1 else 3 + n
        struct.pack_into("<H", fat, (2 + n) * 2, nxt)
    if 2 + used > clusters:
        raise RuntimeError("the file does not fit on the image")

    root = bytearray(root_sectors * sector)
    root[0:11] = PLUGGED_LABEL.encode().ljust(11, b" ")
    root[11] = 0x08                               # the volume label
    name, ext = PLUGGED_FILE.split(".")
    root[32:43] = name.upper().ljust(8).encode() + ext.upper().ljust(3).encode()
    root[43] = 0x20                               # an ordinary file
    struct.pack_into("<HH", root, 32 + 22, 0x6000, 0x5900)   # a 2024 date
    struct.pack_into("<HI", root, 32 + 26, 2, len(payload))

    data = bytearray(used * per_cluster * sector)
    data[0:len(payload)] = payload

    mbr = bytearray(sector)
    mbr[446] = 0x00                               # not bootable
    mbr[450] = 0x0E                               # FAT16, addressed by LBA
    struct.pack_into("<II", mbr, 454, start, sectors)
    mbr[510:512] = b"\x55\xaa"

    with open(path, "wb") as f:
        f.write(mbr)
        f.write(bytes((start - 1) * sector))
        f.write(boot)
        f.write(fat * 2)
        f.write(root)
        f.write(data)
        f.truncate((start + sectors) * sector)
    return path


def mtools_image(path, start, sectors, payload):
    """The same image, made by mtools where the box has it."""
    at = f"{path}@@{start * 512}"
    mbr = bytearray(512)
    mbr[450] = 0x0E
    struct.pack_into("<II", mbr, 454, start, sectors)
    mbr[510:512] = b"\x55\xaa"
    with open(path, "wb") as f:
        f.truncate((start + sectors) * 512)
        f.seek(0)
        f.write(mbr)
    subprocess.check_call(["mformat", "-i", at, "-v", PLUGGED_LABEL, "::"])
    source = path + ".payload"
    with open(source, "wb") as f:
        f.write(payload)
    subprocess.check_call(["mcopy", "-i", at, source, f"::{PLUGGED_FILE}"])
    os.unlink(source)
    return path


def plug_in(monitor, path):
    """Attach a disk image as a second USB stick, while the machine runs."""
    monitor.command("blockdev-add", driver="raw", **{"node-name": "plugged"},
                    file={"driver": "file", "filename": os.path.abspath(path)})
    monitor.command("device_add", driver="usb-storage", bus="xhci.0",
                    drive="plugged", id="plugged-dev")


def usb_port_of(monitor, device_id):
    """The xHCI port a USB device sits on, from `info usb`, so it can be
    put back into the same port: what a person does with a stick."""
    text = monitor.command("human-monitor-command",
                           **{"command-line": "info usb"})
    for line in text.splitlines():
        if f"ID: {device_id}" in line:
            for word in line.split(","):
                word = word.strip()
                if word.startswith("Port "):
                    return word[len("Port "):]
    return None


def plug_back_in(monitor, port):
    """Put that same stick back into the same port: the block node is still
    there, so only the USB device is added again, which is what the
    machine sees."""
    args = dict(driver="usb-storage", bus="xhci.0", drive="plugged",
                id="plugged-dev")
    if port:
        args["port"] = port
    monitor.command("device_add", **args)


def unplug(monitor):
    """Pull that stick out again, as a person does without warning."""
    monitor.command("device_del", id="plugged-dev")


def wait_for_serial_text(path, needle, seconds):
    """The first console line holding `needle`, or None."""
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if os.path.exists(path):
            with open(path, "rb") as f:
                text = f.read().decode("utf-8", "replace")
            for line in text.splitlines():
                if needle in line:
                    return line.strip()
        time.sleep(0.5)
    return None


def wait_for_serial_count(path, needle, count, seconds):
    """The `count`-th console line holding `needle`, or None: for a thing
    that is expected to happen again, whose first line is already there."""
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if os.path.exists(path):
            with open(path, "rb") as f:
                text = f.read().decode("utf-8", "replace")
            hits = [line.strip() for line in text.splitlines() if needle in line]
            if len(hits) >= count:
                return hits[count - 1]
        time.sleep(0.5)
    return None


def pull_boot_stick(monitor):
    """Pull the stick the machine booted from. The kernel and the root
    filesystem are in memory, so the app keeps running; only the exchange
    partition goes."""
    monitor.command("device_del", id="stick-dev")


def replace_boot_stick(monitor, image, port):
    """Put the boot stick back into the same port, as a person does after
    copying a file onto it on another computer. The kernel sees a new
    disk. QEMU dropped the drive with the device, so the image is attached
    under a new node."""
    monitor.command("blockdev-add", driver="raw",
                    **{"node-name": "stick-back"},
                    file={"driver": "file",
                          "filename": os.path.abspath(image)})
    args = dict(driver="usb-storage", bus="xhci.0", drive="stick-back",
                id="stick-dev")
    if port:
        args["port"] = port
    monitor.command("device_add", **args)


def block_reads(monitor):
    """Read operations so far on every block device QEMU has, by name.
    What a stick's activity light shows, as a number."""
    reads = {}
    for stat in monitor.command("query-blockstats"):
        name = stat.get("device") or stat.get("node-name")
        if name:
            reads[name] = stat["stats"]["rd_operations"]
    return reads


def wait_for_serial_line(path, prefix, seconds):
    """The first console line starting with `prefix`, or None."""
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if os.path.exists(path):
            with open(path, "rb") as f:
                text = f.read().decode("utf-8", "replace")
            for line in text.splitlines():
                if line.startswith(prefix):
                    return line.strip()
        time.sleep(0.25)
    return None


def wait_for_app(monitor, scratch, seconds):
    """Wait until the app has drawn, and return that frame.

    The firmware's screen comes first, then the kernel clearing it, then
    the boot logo in a corner, and the shell says it has started a moment
    before it has anything to draw — so neither the console nor a fixed
    time says when to look. What says it is the screen: it changes from
    what the firmware left, it has something on it, it carries the action
    bar's orange, and it stops changing. The orange is in the test
    because the firmware's own logo sits still in the middle of an
    otherwise black screen for seconds at a time, which is a frame that
    changed and stopped changing and is not the app.

    A screen that is still black when the time is up is returned as it is,
    with the time it was given, so the caller fails on the frame a person
    would have been looking at rather than on an exception.
    """
    start = time.monotonic()
    width, height, before = monitor.screendump(scratch)
    last = before
    while time.monotonic() - start < seconds:
        time.sleep(1.0)
        width, height, now = monitor.screendump(scratch)
        if (differs(before, now) > CHANGED
                and differs(last, now) == 0
                and lit_pixels(now) > 0
                and count_colour(now, ACCENT) > 0):
            waited = time.monotonic() - start
            return width, height, now, f"settled after {waited:.0f}s"
        last = now
    return width, height, last, f"never settled within {seconds:.0f}s"


class Console:
    """The dev image's console, read for what init said since last asked.

    The watcher's only channel is the kernel log, which the dev variant
    prints on ttyS0, and every case below says what it expects init to
    have said about the sticks that case plugged in. So a case starts by
    forgetting everything before it: what it reads are the lines its own
    sticks produced, however many earlier cases produced before them.
    """

    def __init__(self, path):
        self.path = path
        self.seen = 0
        self.forget()

    def text(self):
        if not os.path.exists(self.path):
            return ""
        with open(self.path, "rb") as f:
            return f.read().decode("utf-8", "replace")

    def forget(self):
        """Everything printed so far is not this case's."""
        self.seen = len(self.text().splitlines())

    def since(self, needle):
        """The lines holding `needle` printed since `forget`."""
        return [line.strip()
                for line in self.text().splitlines()[self.seen:]
                if needle in line]

    def wait(self, needle, count, seconds):
        """Wait for `count` such lines, and return what there is either
        way: a case says what is wrong with too few better than a
        timeout does."""
        deadline = time.monotonic() + seconds
        while True:
            lines = self.since(needle)
            if len(lines) >= count or time.monotonic() > deadline:
                return lines
            time.sleep(0.25)


def wait_for_read(monitor, node, seconds):
    """How many reads a block node has had, as soon as it has had any.

    Which is the moment a stick is being read: the kernel is reading its
    partition table, or `blkid` its superblock. Pulling it out then is
    the worst moment there is for it, and the only way to aim at that
    moment from here is to watch the reads themselves.
    """
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        reads = block_reads(monitor).get(node, 0)
        if reads:
            return reads
        time.sleep(0.05)
    return 0


def mount_point(line):
    """Where a `mounted /dev/sdb1 at /mnt/usb/NAME` line says it went."""
    return line.split(" at ", 1)[1].strip() if " at " in line else ""


def case_disk(path, *, label=None, ext4=False, partitioned=True):
    """A raw disk image for one of the cases, as a computer would leave it.

    `label` is a FAT label of up to eleven characters, or None for a
    stick nobody named. `ext4` puts an ext4 filesystem in the partition
    instead of a FAT one. `partitioned` off writes the filesystem over
    the whole disk with no partition table, which is what a stick
    formatted by a camera or by `mkfs` on a whole device looks like.
    """
    with open(path, "wb") as f:
        f.truncate(CASE_SECTORS * 512)
    start = CASE_START if partitioned else 0
    if partitioned:
        mbr = bytearray(512)
        mbr[450] = 0x83 if ext4 else 0x0E
        struct.pack_into("<II", mbr, 454, start, CASE_SECTORS - start)
        mbr[510:512] = b"\x55\xaa"
        with open(path, "r+b") as f:
            f.write(mbr)
    blocks = (CASE_SECTORS - start) // 2          # 1024-byte blocks
    if ext4:
        subprocess.run(
            ["mkfs.ext4", "-F", "-q", "-E", f"offset={start * 512}",
             "-L", label or "", path, f"{blocks}k"],
            check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return path
    argv = ["mkfs.fat", "-F", "16", "-I"]
    if start:
        argv += ["--offset", str(start)]
    if label is not None:
        argv += ["-n", label]
    subprocess.run(argv + [path, str(blocks)],
                   check=True, stdout=subprocess.DEVNULL)
    return path


def relabel(path, label):
    """Write eleven bytes over a FAT label `mkfs.fat` would not accept.

    A FAT label lives twice: in the boot sector and as an entry in the
    root directory, and a reader may take either. `mkfs.fat` refuses a
    label with a slash or a byte over 127 in it, and a computer that
    wrote one by other means is exactly the stick this is about — so the
    filesystem is made with a placeholder and the placeholder replaced.
    """
    with open(path, "rb") as f:
        data = f.read()
    if PLACEHOLDER not in data:
        raise RuntimeError(f"{path}: the placeholder label is not on it")
    with open(path, "wb") as f:
        f.write(data.replace(PLACEHOLDER, label.ljust(11, b" ")[:11]))
    return path


def plug(monitor, path, tag):
    """Attach a disk image as a further USB stick, while the machine runs."""
    monitor.command("blockdev-add", driver="raw",
                    **{"node-name": f"node-{tag}"},
                    file={"driver": "file", "filename": os.path.abspath(path)})
    monitor.command("device_add", driver="usb-storage", bus="xhci.0",
                    drive=f"node-{tag}", id=f"dev-{tag}")
    return tag


def plug_again(monitor, tag, as_tag):
    """Put a stick that was pulled back in. QEMU still holds its image
    open under the node it was added as, so only the USB device is added
    again, which is all the machine sees."""
    monitor.command("device_add", driver="usb-storage", bus="xhci.0",
                    drive=f"node-{tag}", id=f"dev-{as_tag}")
    return as_tag


def unplug_tag(monitor, tag):
    monitor.command("device_del", id=f"dev-{tag}")


def clear_sticks(monitor, console, tags, failures, case):
    """Pull every stick a case plugged in, and see each mount go.

    A case leaves the machine as it found it, so the next one counts its
    own lines and names its own mount points with nothing of an earlier
    case's still mounted.
    """
    mounted = len(console.since("opensigner: mounted"))
    console.forget()
    for tag in tags:
        unplug_tag(monitor, tag)
    if not mounted:
        time.sleep(2.0)
        return
    gone = console.wait("opensigner: unmounted", mounted, CASE_SECONDS)
    for line in gone:
        print(f"  init says: {line}")
    if len(gone) < mounted:
        failures.append(
            f"{case}: {mounted} mounts were made and {len(gone)} taken down "
            f"when the sticks were pulled"
        )


def named_cases(monitor, console, out, width, height, failures):
    """The cases the watcher's rules cover that no test had observed.

    Each makes its own sticks, plugs them in over the monitor the way a
    person plugs one into the next port, and reads off the dev console
    what init did with them. What init says is the whole of what can be
    seen from here: the console is written to and not typed into, so
    `/mnt` is known by the mount and unmount lines that named it.
    """
    def disk(name):
        return os.path.join(out, name)

    # 1. Two sticks with the same label. Both are a place a person can
    #    read a file from, so both are mounted, and the second one's
    #    directory is not the first one's.
    print("case 1: two sticks with the same label")
    console.forget()
    same = [plug(monitor, case_disk(disk("same-1.img"), label=PLUGGED_LABEL),
                 "same1"),
            plug(monitor, case_disk(disk("same-2.img"), label=PLUGGED_LABEL),
                 "same2")]
    lines = console.wait("opensigner: mounted", 2, CASE_SECONDS)
    for line in lines:
        print(f"  init says: {line}")
    points = [mount_point(line) for line in lines]
    if len(lines) < 2:
        failures.append(
            f"case 1: two sticks labelled {PLUGGED_LABEL} were plugged in "
            f"and {len(lines)} were mounted"
        )
    elif len(set(points)) < 2:
        failures.append(f"case 1: both sticks were mounted at {points[0]}")
    for point in points:
        if not point.startswith("/mnt/usb/"):
            failures.append(f"case 1: a stick was mounted at {point}")
    clear_sticks(monitor, console, same, failures, "case 1")

    # 2. A stick nobody named. It is still a place to read a file from,
    #    so it is mounted, under whatever name the watcher gives it.
    print("case 2: a stick with no label")
    console.forget()
    bare = [plug(monitor, case_disk(disk("nameless.img")), "nolabel")]
    lines = console.wait("opensigner: mounted", 1, CASE_SECONDS)
    for line in lines:
        print(f"  init says: {line}")
    if not lines:
        failures.append("case 2: a stick with no label was not mounted")
    else:
        point = mount_point(lines[0])
        if not is_usb_name(point):
            failures.append(f"case 2: the unnamed stick was mounted at {point}")
    clear_sticks(monitor, console, bare, failures, "case 2")

    # 3. Labels a computer can write and a directory name cannot hold.
    #    Every one of them is a directory under /mnt/usb whose name is
    #    letters, digits, `_` and `-`, and none of them is /mnt itself or
    #    anywhere above it.
    print("case 3: hostile labels")
    console.forget()
    hostile = []
    for n, (what, label) in enumerate(HOSTILE_LABELS):
        path = case_disk(disk(f"hostile-{n}.img"), label=PLACEHOLDER.decode())
        relabel(path, label)
        print(f"  a stick labelled {label!r} ({what})")
        hostile.append(plug(monitor, path, f"hostile{n}"))
    lines = console.wait("opensigner: mounted", len(HOSTILE_LABELS),
                         CASE_SECONDS)
    for line in lines:
        print(f"  init says: {line}")
    if len(lines) < len(HOSTILE_LABELS):
        failures.append(
            f"case 3: {len(HOSTILE_LABELS)} sticks were plugged in and "
            f"{len(lines)} were mounted"
        )
    for line in lines:
        point = mount_point(line)
        if not is_usb_name(point):
            failures.append(
                f"case 3: a hostile label reached {point}, which is not a "
                f"plain name under /mnt/usb"
            )
    clear_sticks(monitor, console, hostile, failures, "case 3")

    # 4. What is not a FAT partition. An ext4 partition is not something
    #    this kernel can mount at all; a FAT filesystem written over a
    #    whole disk with no partition table is not a partition, and the
    #    watcher looks at partitions. Neither is mounted, and neither
    #    stops the other sticks being read.
    print("case 4: an ext4 partition, and a FAT filesystem with no "
          "partition table")
    console.forget()
    other = [
        plug(monitor, case_disk(disk("ext4.img"), label="EXTFOUR", ext4=True),
             "ext4"),
        plug(monitor,
             case_disk(disk("whole.img"), label="WHOLEDISK",
                       partitioned=False),
             "whole"),
    ]
    time.sleep(IGNORED_SECONDS)
    lines = console.since("opensigner: mounted")
    for line in lines:
        print(f"  init says: {line}")
    if lines:
        failures.append(
            f"case 4: init mounted something that is neither a FAT "
            f"partition nor a partition at all: {lines}"
        )
    else:
        print(f"  init said nothing in {IGNORED_SECONDS:.0f}s: neither is "
              f"mounted")
    clear_sticks(monitor, console, other, failures, "case 4")

    # 6. A stick pulled while it is being read. The watcher opens a new
    #    partition to find out what is on it; pulling the stick during
    #    that is the worst moment for it. The device keeps running, the
    #    mount does not survive the stick, and the same stick put back is
    #    mounted as any other is.
    print("case 6: a stick pulled while it is being read")
    console.forget()
    torn = case_disk(disk("torn.img"), label="TORNAWAY")
    plug(monitor, torn, "torn")
    reads = wait_for_read(monitor, "node-torn", CASE_SECONDS)
    unplug_tag(monitor, "torn")
    print(f"  pulled out on the first read of it, with {reads} read "
          f"{'operation' if reads == 1 else 'operations'} on it")
    if not reads:
        failures.append(
            f"case 6: nothing read the stick within {CASE_SECONDS:.0f}s, so "
            f"it was not pulled out while it was being read"
        )
    time.sleep(3.0)
    said = console.since("opensigner: ")
    for line in said:
        print(f"  init says: {line}")
    if not said:
        print("  init said nothing: the stick was gone before it was "
              "mounted")
    left = [mount_point(line) for line in console.since("opensigner: mounted")]
    gone = [mount_point(line)
            for line in console.since("opensigner: unmounted")]
    for point in left:
        if point not in gone:
            failures.append(
                f"case 6: {point} was still mounted after the stick it is on "
                f"was pulled out while it was being read"
            )
    path = os.path.join(out, "5-torn.ppm")
    _, _, frame = monitor.screendump(path)
    write_png(path.replace(".ppm", ".png"), width, height, frame)
    print(f"  the screen after it: {lit_pixels(frame)} pixels lit, "
          f"{count_colour(frame, ACCENT)} of them the action bar's orange"
          f"  {path}")
    if lit_pixels(frame) == 0 or count_colour(frame, ACCENT) == 0:
        failures.append(
            "case 6: the app is not on the screen after a stick was pulled "
            "out while it was being read"
        )
    console.forget()
    plug_again(monitor, "torn", "torn-again")
    again = console.wait("opensigner: mounted", 1, CASE_SECONDS)
    for line in again:
        print(f"  init says: {line}")
    if not again:
        failures.append(
            "case 6: the stick put back after being pulled mid-read was not "
            "mounted"
        )
    clear_sticks(monitor, console, ["torn-again"], failures, "case 6")


def is_usb_name(point):
    """Whether a mount point is one plain directory under /mnt/usb.

    Which is the whole of what a label may reach: `/mnt` itself, anything
    beside it and anything above it are not this.
    """
    if not point.startswith("/mnt/usb/"):
        return False
    name = point[len("/mnt/usb/"):]
    return bool(name) and all(
        c.isascii() and (c.isalnum() or c in "_-") for c in name
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True)
    parser.add_argument("--out", default="out/stick/test")
    parser.add_argument("--dev", action="store_true",
                        help="the image is the dev variant, which says on "
                             "ttyS0 what the shell found")
    parser.add_argument("--boot-seconds", type=float, default=90.0)
    parser.add_argument("--vnc", type=int, default=19)
    parser.add_argument("--no-kvm", action="store_true")
    args = parser.parse_args()

    if not os.path.exists(args.image):
        sys.exit(f"{args.image}: no such image. Run `just stick-image` first.")

    out = os.path.abspath(args.out)
    os.makedirs(out, exist_ok=True)
    serial_path = os.path.join(out, "serial.log")
    qmp_path = os.path.join(out, "qmp.sock")
    for path in (serial_path, qmp_path):
        if os.path.exists(path):
            os.unlink(path)

    argv = stick_qemu.command(
        os.path.abspath(args.image),
        vars_fd=stick_qemu.vars_copy(out),
        serial=f"file:{serial_path}",
        qmp=qmp_path,
        vnc=args.vnc,
        kvm=not args.no_kvm,
    )
    print(" ".join(argv))
    qemu = subprocess.Popen(argv)
    failures = []
    try:
        monitor = Qmp(qmp_path)

        if args.dev:
            line = wait_for_serial_line(serial_path, "panel ",
                                        args.boot_seconds)
            if line is None:
                failures.append("the shell never said what it found on ttyS0")
            else:
                print(f"the shell says: {line}")

        scratch = os.path.join(out, "scratch.ppm")
        width, height, first, note = wait_for_app(monitor, scratch,
                                                  args.boot_seconds)
        path = os.path.join(out, "1-first.ppm")
        os.replace(scratch, path)
        write_png(path.replace(".ppm", ".png"), width, height, first)
        lit = lit_pixels(first)
        accent = count_colour(first, ACCENT)
        print(f"{width}x{height}, {note}: {lit} pixels lit, {accent} of "
              f"them the action bar's orange  {path}")
        if lit == 0:
            failures.append("the screen is black")
        if accent == 0:
            failures.append("no pixel is the action bar's orange")

        # The first screen of a first boot is This device, whose action bar
        # opens Start here; Back from there is Home. Two steps, and both of
        # them are the pointer being read.
        button = accent_center(width, first)
        if button is None:
            failures.append("there is no action bar to click")
            home = first
        else:
            monitor.move_to(button[0], button[1], width, height)
            time.sleep(0.5)
            monitor.click()
            time.sleep(2.0)
            monitor.key("esc")
            time.sleep(2.0)
            path = os.path.join(out, "2-home.ppm")
            _, _, home = monitor.screendump(path)
            write_png(path.replace(".ppm", ".png"), width, height, home)
            print(f"after the action bar at {button[0]},{button[1]} and "
                  f"Esc: {differs(first, home):.1%} of the frame changed"
                  f"  {path}")
            if differs(first, home) < CHANGED:
                failures.append("the action bar and Esc opened nothing")

        row = first_row_center(width, height, home)
        if row is None:
            failures.append("no row was found to click")
        else:
            monitor.move_to(row[0], row[1], width, height)
            time.sleep(0.5)
            monitor.click()
            time.sleep(2.0)
            path = os.path.join(out, "3-row.ppm")
            _, _, opened = monitor.screendump(path)
            write_png(path.replace(".ppm", ".png"), width, height, opened)
            moved = differs(home, opened)
            print(f"after a click on the first row at {row[0]},{row[1]}: "
                  f"{moved:.1%} of the frame changed  {path}")
            if moved < CHANGED:
                failures.append(
                    f"the click at {row[0]},{row[1]} changed {moved:.1%} of "
                    f"the frame; nothing opened"
                )

            monitor.key("esc")
            time.sleep(2.0)
            path = os.path.join(out, "4-back.ppm")
            _, _, back = monitor.screendump(path)
            write_png(path.replace(".ppm", ".png"), width, height, back)
            undone = differs(opened, back)
            same = differs(home, back)
            print(f"after Esc: {undone:.1%} of the frame changed, and it is "
                  f"now {same:.1%} different from the screen it left"
                  f"  {path}")
            if undone < CHANGED:
                failures.append(
                    f"Esc changed {undone:.1%} of the frame; the keyboard "
                    f"did not arrive"
                )
            if same > CHANGED:
                failures.append(
                    f"Esc left a screen {same:.1%} different from the one it "
                    f"started on; it did not go back"
                )

        # A second stick, plugged in with the app already up. Init has
        # to notice the disk, read its partition table, see a vfat on it
        # and mount it under /mnt/usb; the app lists what is mounted the
        # next time a person opens Read a file.
        plugged = plugged_image(os.path.join(out, "plugged.img"))
        boot_port = usb_port_of(monitor, "stick-dev")
        plug_in(monitor, plugged)
        plugged_port = usb_port_of(monitor, "plugged-dev")
        print(f"the boot stick is in port {boot_port}, the second in "
              f"{plugged_port}")
        print(f"a second USB stick, {PLUGGED_LABEL} with one "
              f"{PLUGGED_FILE} on it: {plugged}")
        if args.dev:
            line = wait_for_serial_text(serial_path, "opensigner: mounted",
                                        PLUGGED_SECONDS)
            if line is None:
                failures.append(
                    f"init mounted nothing within {PLUGGED_SECONDS:.0f}s of "
                    f"the second stick being plugged in"
                )
            else:
                print(f"init says: {line}")
                if "/mnt/usb/" not in line:
                    failures.append(f"the stick was not mounted under "
                                    f"/mnt/usb: {line}")
                # And out again, with no warning, which is how a stick
                # leaves a machine.
                unplug(monitor)
                gone = wait_for_serial_text(serial_path,
                                            "opensigner: unmounted",
                                            PLUGGED_SECONDS)
                if gone is None:
                    failures.append(
                        f"the mount was still there {PLUGGED_SECONDS:.0f}s "
                        f"after the stick was pulled out"
                    )
                else:
                    print(f"init says: {gone}")
                # And back in: the same stick, which the kernel gives the
                # same node name and a new identity, is mounted again.
                plug_back_in(monitor, plugged_port)
                again = wait_for_serial_count(serial_path,
                                              "opensigner: mounted", 2,
                                              PLUGGED_SECONDS)
                if again is None:
                    failures.append(
                        f"the second stick put back in was not mounted "
                        f"again within {PLUGGED_SECONDS:.0f}s"
                    )
                else:
                    print(f"init says: {again}")
                    if "/mnt/usb/" not in again:
                        failures.append(f"the stick put back was not "
                                        f"mounted under /mnt/usb: {again}")
                unplug(monitor)
                wait_for_serial_count(serial_path, "opensigner: unmounted",
                                      2, PLUGGED_SECONDS)
                # The boot stick itself: out, then back with a file on it,
                # which is what a person does to hand the device a PSBT.
                pull_boot_stick(monitor)
                card_gone = wait_for_serial_text(serial_path,
                                                 "unmounted /mnt/microsd",
                                                 PLUGGED_SECONDS)
                if card_gone is None:
                    failures.append(
                        f"the boot stick's own partition was still mounted "
                        f"{PLUGGED_SECONDS:.0f}s after the stick was pulled"
                    )
                else:
                    print(f"init says: {card_gone}")
                replace_boot_stick(monitor, args.image, boot_port)
                card_back = wait_for_serial_text(serial_path,
                                                 "at /mnt/microsd",
                                                 PLUGGED_SECONDS)
                if card_back is None:
                    failures.append(
                        f"the boot stick put back in was not mounted at "
                        f"/mnt/microsd within {PLUGGED_SECONDS:.0f}s"
                    )
                else:
                    print(f"init says: {card_back}")
        else:
            # The release image prints nothing anywhere, so there is no
            # line to read. `just dev=1 stick-test` is what checks the
            # mount.
            time.sleep(PLUGGED_SECONDS / 6)
            print("the release image says nothing about what it mounted; "
                  "run `just dev=1 stick-test` for that")

        # And then nothing: with every stick where it is, init must not
        # be reading any of them. A stick's light shows every read, and
        # the first laptop run of the watcher flashed without stopping.
        time.sleep(3.0)
        before = block_reads(monitor)
        samples = []
        for _ in range(int(QUIET_SECONDS)):
            time.sleep(1.0)
            now = block_reads(monitor)
            samples.append({name: now[name] - before.get(name, 0)
                            for name in now if now[name] != before.get(name, 0)})
        after = block_reads(monitor)
        busy = {name: after[name] - before.get(name, 0)
                for name in after if after[name] != before.get(name, 0)}
        if busy:
            print(f"reads per second, cumulative: {samples}")
        if busy:
            failures.append(
                f"a stick was read while nothing was plugged or pulled: "
                f"{busy} reads in {QUIET_SECONDS:.0f}s"
            )
        else:
            print(f"no block device was read in {QUIET_SECONDS:.0f}s of "
                  f"nothing happening: {sorted(after)}")

        # The cases the watcher's rules cover and nothing had watched it
        # do: two sticks of one name, a stick of no name, labels a
        # directory name cannot hold, what is not a FAT partition, and a
        # stick pulled while it is being read. All of them are read off
        # the console, so they run on the dev image only.
        if args.dev:
            named_cases(monitor, Console(serial_path), out, width, height,
                        failures)
        else:
            print("the release image says nothing about what it mounted, so "
                  "the named cases run under `just dev=1 stick-test` only")

        monitor.command("quit")
    finally:
        try:
            qemu.wait(timeout=10)
        except subprocess.TimeoutExpired:
            qemu.kill()
            qemu.wait()
        if os.path.exists(os.path.join(out, "scratch.ppm")):
            os.unlink(os.path.join(out, "scratch.ppm"))

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1
    print("the stick boots into the app, takes a mouse and a keyboard, and "
          "init mounts a second stick plugged in while it runs, mounts it "
          "again when it comes back, takes the boot stick out and back, "
          "reads nothing while nothing happens, mounts two sticks of one "
          "name and a stick of none under plain names of its own, lets no "
          "label name a path, leaves an ext4 partition and an unpartitioned "
          "disk alone, and keeps running when a stick is pulled out mid-read")
    return 0


if __name__ == "__main__":
    sys.exit(main())
