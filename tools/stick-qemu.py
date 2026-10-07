#!/usr/bin/env python3
"""Boot the OpenSigner stick image in QEMU, the way a laptop boots it.

The point of the arrangement below is that nothing is virtualised that a
laptop would not have. OVMF is the firmware; the image is a USB disk on an
xHCI controller, so the firmware finds `EFI/BOOT/BOOTX64.EFI` on the first
FAT partition of a removable disk and runs it, which is the path the stick
takes on real hardware; the keyboard and the mouse are USB devices. There
is no virtio anything and no virtual hard disk.

  just stick-qemu           the release image, display on VNC 127.0.0.1:5909
  just dev=1 stick-qemu     the dev image, whose console is on stdio

`--usb IMAGE` attaches a second USB disk at start, which is how a person
tries the file channel with a stick that is not the boot stick. Any raw
disk image with a FAT partition on it will do.

`tools/stick-test.py` imports `command` from here so that the machine the
test proves is the machine the owner looks at.

VNC listens on the loopback address only. The way to see it from another
machine is an SSH tunnel to this one; the image README says how.
"""

import argparse
import os
import shutil
import subprocess
import sys

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
        sys.exit(f"{args.image}: no such image. Run `just stick-image` first.")
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
