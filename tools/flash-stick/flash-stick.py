#!/usr/bin/env python3
"""Flash a Faraday stick from this checkout: a window to pick the stick and
the build, and a confirmation that names the stick before anything is
written.

  tools/flash-stick/flash-stick.py      or   just faraday-flash-stick

or a double-click on flash-stick.desktop beside it, in a file manager
that runs launchers (Thunar, Dolphin, Nemo).

It lists the USB disks attached now, with their maker, model, size,
serial and partitions, and says which are Faraday sticks already. The
list follows sticks as they are plugged in and pulled out. Then one of:

  Stick build, whole stick         out/stick/faraday-x86_64-uefi.img
  Stick build, keep data           out/stick/faraday-x86_64-uefi-boot.vfat
  Dev build, whole stick           out/stick/faraday-x86_64-uefi-dev.img
  Dev build, keep data             out/stick/faraday-x86_64-uefi-dev-boot.vfat

"Keep data" replaces the OSKBOOT partition and leaves OSKDATA, with its
vaults and settings, as it is; it is offered only for a stick that is a
Faraday stick already. Each build shows the build ID it carries, read
from the kernel's release string in the image.

The window runs as the person who opened it. The write itself is
tools/flash-stick/write.sh, run as root through pkexec, so the system
asks for the password; it checks the stick again (USB, removable, not in
use, the serial and size that were confirmed) and reads a whole write
back. Needs Python 3 with PyGObject and GTK 4, polkit's pkexec and
util-linux.
"""

import json
import mmap
import os
import re
import subprocess
import sys
import threading
from pathlib import Path

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk, Pango  # noqa: E402

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
WRITER = HERE / "write.sh"
OUT = REPO / "out" / "stick"

# (mode, image, title, what it does)
CHOICES = [
    (
        "whole",
        "faraday-x86_64-uefi.img",
        "Stick build, whole stick",
        "Erases everything on the stick and makes it a new Faraday stick.",
    ),
    (
        "boot",
        "faraday-x86_64-uefi-boot.vfat",
        "Stick build, keep data",
        "Replaces Faraday on a Faraday stick and keeps its data partition: "
        "vaults and settings stay.",
    ),
    (
        "whole",
        "faraday-x86_64-uefi-dev.img",
        "Dev build, whole stick",
        "Erases everything. The dev build has a root login on the serial "
        "port and shows the kernel log on the screen.",
    ),
    (
        "boot",
        "faraday-x86_64-uefi-dev-boot.vfat",
        "Dev build, keep data",
        "The dev build on a Faraday stick, keeping its data partition.",
    ),
]

# Mount points that mean the disk is the computer's own.
SYSTEM = ("/", "/boot", "/boot/efi", "/efi", "/home", "/usr", "/var", "[SWAP]")

BUILD_ID = re.compile(rb"\d+\.\d+\.\d+-faraday-([0-9A-Za-z.+-]+)")
PROGRESS = re.compile(r"^(\d+) bytes")


def human(n):
    """A size the way a stick's box gives it."""
    for unit in ("bytes", "kB", "MB", "GB", "TB"):
        if n < 1000 or unit == "TB":
            return f"{n:.0f} {unit}" if unit == "bytes" else f"{n:.1f} {unit}"
        n /= 1000
    return ""


_build_ids = {}


def build_id(path):
    """The Faraday build ID in an image, from its kernel release string."""
    try:
        st = path.stat()
    except OSError:
        return None
    key = (str(path), st.st_mtime_ns, st.st_size)
    if key not in _build_ids:
        found = "unknown build"
        try:
            with open(path, "rb") as f, mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ) as m:
                hit = BUILD_ID.search(m)
                if hit:
                    found = hit.group(1).decode()
        except (OSError, ValueError):
            pass
        _build_ids[key] = found
    return _build_ids[key]


def sticks():
    """The USB disks attached now, as lsblk describes them."""
    try:
        out = subprocess.run(
            [
                "lsblk", "-J", "-b", "-o",
                "NAME,PATH,TYPE,TRAN,RM,HOTPLUG,SIZE,VENDOR,MODEL,SERIAL,"
                "LABEL,PARTLABEL,FSTYPE,MOUNTPOINTS",
            ],
            capture_output=True, text=True, check=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return []
    found = []
    for d in json.loads(out).get("blockdevices", []):
        if d.get("type") != "disk" or d.get("tran") != "usb":
            continue
        if not (truthy(d.get("rm")) or truthy(d.get("hotplug"))):
            continue
        parts = d.get("children") or []
        labels = [p.get("partlabel") for p in parts]
        mounts = [m for p in [d, *parts] for m in (p.get("mountpoints") or []) if m]
        found.append({
            "path": d["path"],
            "name": " ".join(x.strip() for x in (d.get("vendor"), d.get("model")) if x and x.strip())
            or "USB disk",
            "size": int(d.get("size") or 0),
            "serial": (d.get("serial") or "").strip(),
            "parts": parts,
            "faraday": labels[:2] == ["OSKBOOT", "OSKDATA"] and len(parts) == 2,
            "system": any(m in SYSTEM for m in mounts),
        })
    return found


def truthy(v):
    return v is True or v == "1" or v == 1


def describe_part(p):
    bits = [Path(p["path"]).name]
    if p.get("partlabel"):
        bits.append(p["partlabel"])
    if p.get("fstype"):
        bits.append(p["fstype"])
    if p.get("label") and p.get("label") != p.get("partlabel"):
        bits.append(f"“{p['label']}”")
    bits.append(human(int(p.get("size") or 0)))
    mounts = [m for m in (p.get("mountpoints") or []) if m]
    if mounts:
        bits.append("mounted at " + ", ".join(mounts))
    return " · ".join(bits)


class Window(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Flash a Faraday stick")
        self.set_default_size(660, 760)
        self.sticks = []
        self.signature = None
        self.selected = None  # (path, serial)
        self.running = False

        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        for side in ("top", "bottom", "start", "end"):
            getattr(page, f"set_margin_{side}")(18)
        scroller = Gtk.ScrolledWindow(vexpand=True)
        scroller.set_child(page)
        self.set_child(scroller)

        page.append(heading("Stick"))
        self.list = Gtk.ListBox(selection_mode=Gtk.SelectionMode.SINGLE)
        self.list.add_css_class("boxed-list")
        self.list.connect("row-selected", self.on_row)
        page.append(self.list)

        page.append(heading("What to write"))
        self.radios = []
        group = None
        for i, (mode, image, title, what) in enumerate(CHOICES):
            radio = Gtk.CheckButton()
            if group:
                radio.set_group(group)
            group = group or radio
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=2)
            box.append(label(title, bold=True))
            box.append(label(what, dim=True))
            detail = label("", dim=True, small=True)
            box.append(detail)
            radio.set_child(box)
            radio.set_active(i == 0)
            page.append(radio)
            self.radios.append((radio, detail))

        self.button = Gtk.Button(label="Write to the stick")
        self.button.add_css_class("destructive-action")
        self.button.set_halign(Gtk.Align.END)
        self.button.connect("clicked", self.on_write)
        self.reason = label("", dim=True)
        bar = Gtk.Box(spacing=12)
        self.reason.set_hexpand(True)
        bar.append(self.reason)
        bar.append(self.button)
        page.append(bar)

        self.progress = Gtk.ProgressBar(show_text=True, visible=False)
        page.append(self.progress)
        self.status = label("")
        page.append(self.status)
        self.log = Gtk.TextView(editable=False, monospace=True, cursor_visible=False)
        self.log.set_wrap_mode(Gtk.WrapMode.WORD_CHAR)
        logbox = Gtk.ScrolledWindow(min_content_height=140, visible=False)
        logbox.set_child(self.log)
        self.logbox = logbox
        page.append(logbox)

        for radio, _ in self.radios:
            radio.connect("toggled", lambda *_: self.update())
        self.refresh()
        # A list holding the focus selects the row it focuses; the stick
        # is chosen by a click, so the focus starts on the first build.
        self.set_focus(self.radios[0][0])
        GLib.timeout_add_seconds(2, self.refresh)

    # ---- the list of sticks ---------------------------------------------------

    def refresh(self):
        if self.running:
            return True
        found = sticks()
        signature = json.dumps(found, sort_keys=True)
        if signature != self.signature:
            self.signature = signature
            self.sticks = found
            self.fill()
        return True

    def fill(self):
        keep = self.selected
        self.list.remove_all()
        if not self.sticks:
            row = Gtk.ListBoxRow(activatable=False, selectable=False)
            row.set_child(padded(label("No USB stick attached. Plug one in.", dim=True)))
            self.list.append(row)
        chosen = None
        for s in self.sticks:
            row = Gtk.ListBoxRow()
            row.stick = s
            box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=3)
            box.append(label(f"{s['name']}  ·  {human(s['size'])}  ·  {s['path']}", bold=True))
            facts = [f"Serial {s['serial'] or 'none'}"]
            if s["faraday"]:
                facts.append("a Faraday stick")
            if s["system"]:
                facts.append("in use by this computer: not offered")
            box.append(label(" · ".join(facts), dim=True))
            for p in s["parts"]:
                box.append(label(describe_part(p), dim=True, small=True))
            if not s["parts"]:
                box.append(label("no partitions", dim=True, small=True))
            row.set_child(padded(box))
            row.set_selectable(not s["system"])
            self.list.append(row)
            if keep and (s["path"], s["serial"]) == keep and not s["system"]:
                chosen = row
        if chosen is None and len([s for s in self.sticks if not s["system"]]) == 1:
            chosen = next(
                r for r in self.rows() if getattr(r, "stick", None) and not r.stick["system"]
            )
        if chosen is None:
            self.list.unselect_all()
        else:
            self.list.select_row(chosen)
        self.update()

    def rows(self):
        r = self.list.get_first_child()
        while r:
            yield r
            r = r.get_next_sibling()

    def on_row(self, _list, row):
        s = getattr(row, "stick", None) if row else None
        self.selected = (s["path"], s["serial"]) if s else None
        self.update()

    def stick(self):
        row = self.list.get_selected_row()
        return getattr(row, "stick", None) if row else None

    def choice(self):
        for i, (radio, _) in enumerate(self.radios):
            if radio.get_active():
                return CHOICES[i]
        return CHOICES[0]

    # ---- what can be written --------------------------------------------------

    def update(self):
        for (radio, detail), (_, image, _, _) in zip(self.radios, CHOICES):
            path = OUT / image
            bid = build_id(path)
            if bid is None:
                detail.set_text(f"Not built: out/stick/{image}")
            else:
                detail.set_text(f"{bid}  ·  out/stick/{image}")
        mode, image, _, _ = self.choice()
        s = self.stick()
        why = ""
        if self.running:
            why = "Writing…"
        elif s is None:
            why = "Choose a stick."
        elif build_id(OUT / image) is None:
            why = f"out/stick/{image} is not built."
        elif mode == "boot" and not s["faraday"]:
            why = "Keep data needs a stick that is a Faraday stick already."
        elif (OUT / image).stat().st_size > s["size"]:
            why = "The image is larger than the stick."
        self.reason.set_text(why)
        self.button.set_sensitive(not why)

    # ---- the write -------------------------------------------------------------

    def on_write(self, _button):
        mode, image, title, _ = self.choice()
        s = self.stick()
        path = OUT / image
        name = f"{s['name']}, {human(s['size'])}, {s['path']}, serial {s['serial'] or 'none'}"
        if mode == "whole":
            message = f"Erase {s['path']} and write the {title.split(',')[0].lower()}?"
            detail = (
                f"{name}\n\nEverything on this stick is replaced by Faraday "
                f"{build_id(path)}. This cannot be undone."
            )
            verb = "Erase and write"
        else:
            message = f"Replace Faraday on {s['path']}?"
            detail = (
                f"{name}\n\nThe boot partition gets Faraday {build_id(path)}. "
                "The data partition, with its vaults and settings, is kept."
            )
            verb = "Write"
        dialog = Gtk.AlertDialog(message=message, detail=detail, modal=True)
        dialog.set_buttons(["Cancel", verb])
        dialog.set_cancel_button(0)
        dialog.set_default_button(0)
        dialog.choose(self, None, self.on_confirmed, (mode, path, dict(s)))

    def on_confirmed(self, dialog, result, args):
        try:
            if dialog.choose_finish(result) != 1:
                return
        except GLib.Error:
            return
        mode, path, s = args
        self.running = True
        self.update()
        self.log.get_buffer().set_text("")
        self.logbox.set_visible(True)
        self.progress.set_visible(True)
        self.progress.set_fraction(0)
        self.progress.set_text("Waiting for the password")
        self.status.set_text("")
        argv = [
            "pkexec", str(WRITER), mode, str(path), s["path"],
            s["serial"] or "-", str(s["size"]),
        ]
        total = path.stat().st_size
        threading.Thread(target=self.run, args=(argv, total), daemon=True).start()

    def run(self, argv, total):
        try:
            p = subprocess.Popen(
                argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL
            )
        except OSError as e:
            GLib.idle_add(self.finished, None, f"Cannot start pkexec: {e}")
            return
        pending = b""
        while True:
            chunk = p.stdout.read1(4096)
            if not chunk:
                break
            pending += chunk
            *lines, pending = re.split(rb"[\r\n]", pending)
            for line in lines:
                if line.strip():
                    GLib.idle_add(self.line, line.decode(errors="replace"), total)
        if pending.strip():
            GLib.idle_add(self.line, pending.decode(errors="replace"), total)
        GLib.idle_add(self.finished, p.wait(), None)

    def line(self, text, total):
        hit = PROGRESS.match(text)
        if hit:
            done = int(hit.group(1))
            self.progress.set_fraction(min(done / total, 1.0))
            self.progress.set_text(f"Writing  {human(done)} of {human(total)}")
            return
        if text.startswith("Reading back"):
            self.progress.set_fraction(1.0)
            self.progress.set_text("Reading back")
        elif text.startswith("Writing"):
            self.progress.set_text("Writing")
        buf = self.log.get_buffer()
        buf.insert(buf.get_end_iter(), text + "\n")
        self.log.scroll_to_iter(buf.get_end_iter(), 0, False, 0, 0)

    def finished(self, code, error):
        self.running = False
        if error:
            text = error
        elif code == 0:
            text = "Done. The stick can be pulled out."
            self.progress.set_fraction(1.0)
            self.progress.set_text("Done")
        elif code in (126, 127):
            text = "Not authorised: nothing was written."
            self.progress.set_visible(False)
        else:
            text = "The write failed: the log above says why."
            self.progress.set_text("Failed")
        self.status.set_text(text)
        self.signature = None
        self.refresh()
        return False


def heading(text):
    w = label(text, bold=True)
    w.add_css_class("title-4")
    return w


def label(text, bold=False, dim=False, small=False):
    w = Gtk.Label(label=text, xalign=0, wrap=True)
    w.set_wrap_mode(Pango.WrapMode.WORD_CHAR)
    if bold:
        w.add_css_class("heading")
    if dim:
        w.add_css_class("dim-label")
    if small:
        w.add_css_class("caption")
    return w


def padded(child):
    for side in ("top", "bottom", "start", "end"):
        getattr(child, f"set_margin_{side}")(10)
    return child


def main():
    app = Gtk.Application(application_id="org.faraday.FlashStick")
    app.connect("activate", lambda a: Window(a).present())
    return app.run(sys.argv[:1])


if __name__ == "__main__":
    sys.exit(main())
