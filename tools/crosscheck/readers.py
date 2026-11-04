"""The outside programs that read and write KDBX files for the `vault`
area: pykeepass, the library KeePass tooling in Python uses, and
`keepassxc-cli`, KeePassXC's command line. Neither knows anything of this
repository. Both are the distribution's packages (Debian and Ubuntu:
`python3-pykeepass`, `keepassxc`); an area that asks for one stops and
names it when it is missing, so a run without it fails rather than
passing without the check.

`keepassxc-cli` is run per call, with the passphrase on standard input, on a
file in a folder of its own and a home folder of its own, so it reads and
writes nothing else.
"""

import os
import subprocess
import tempfile

from common import HarnessError

try:
    from pykeepass import PyKeePass
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the vault area needs the distribution's pykeepass (Debian and Ubuntu: "
        f"python3-pykeepass; Fedora: python3-pykeepass; Arch: python-pykeepass): {e}") from e


def open_kdbx(data, passphrase):
    """pykeepass's reading of `data`, or None when it will not open.

    pykeepass picks the header and body by the major version with a
    construct `Switch` that has no default: a major version other than
    3 or 4 parses as an empty header and no body, and `PyKeePass`
    returns without having derived a key or checked a hash or an HMAC.
    KeePass and KeePassXC refuse such a file (an unknown major version;
    the version is also inside the header's SHA-256 and HMAC), so a file
    is opened here only when pykeepass parsed a KDBX 4 body.

    pykeepass also runs the key derivation the header states before it
    checks the header's SHA-256, so a changed bit in the Argon2
    iterations (2^44 passes) holds it for good; KeePass and KeePassXC
    check the hash first and refuse at once. pykeepass is asked the same
    way: first with a key given and nothing decrypted, which checks the
    version and the header's hash and derives nothing, and only then
    with the passphrase."""
    fd, path = tempfile.mkstemp(suffix=".kdbx")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        try:
            head = PyKeePass(path, password=passphrase, transformed_key=bytes(32), decrypt=False)
            if head.version[0] != 4 or head.kdbx.body is None:
                return None
            kp = PyKeePass(path, password=passphrase)
        except Exception:  # noqa: BLE001 - every failure to open is a refusal
            return None
        if kp.version[0] != 4 or kp.kdbx.body is None:
            return None
        return kp
    finally:
        os.unlink(path)


def keepassxc_cli():
    """The path of `keepassxc-cli`, or None."""
    import judges
    return judges.find("keepassxc-cli")


def need_keepassxc_cli():
    path = keepassxc_cli()
    if not path:
        raise HarnessError(
            "the vault area needs keepassxc-cli (Debian and Ubuntu: keepassxc; Fedora: keepassxc; "
            "Arch: keepassxc; docs/DEVELOPING.md §1)")
    return path


class Kpxc:
    """One database file for `keepassxc-cli`, in a folder of its own. Use as a
    context manager; `path` is the file."""

    def __init__(self, data):
        self.exe = need_keepassxc_cli()
        self.dir = tempfile.TemporaryDirectory(prefix="kpxc-")
        self.path = os.path.join(self.dir.name, "vault.kdbx")
        home = os.path.join(self.dir.name, "home")
        os.mkdir(home)
        self.env = {"PATH": os.environ.get("PATH", "/usr/bin"), "HOME": home,
                    "XDG_CONFIG_HOME": home, "XDG_CACHE_HOME": home, "XDG_DATA_HOME": home,
                    "QT_QPA_PLATFORM": "offscreen", "LC_ALL": "C.UTF-8"}
        with open(self.path, "wb") as f:
            f.write(data)

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.dir.cleanup()

    def data(self):
        with open(self.path, "rb") as f:
            return f.read()

    def run(self, args, passphrase, more=""):
        """(exit status, standard output); `more` follows the passphrase on
        standard input (the entry's password, for `add -p`)."""
        run = subprocess.run([self.exe, *args], input=(passphrase + "\n" + more).encode(),
                             capture_output=True, env=self.env, timeout=300)
        return run.returncode, run.stdout.decode("utf-8", "surrogateescape")

    def entries(self, passphrase):
        """`ls -R -f`: every group (ending in `/`) and entry by its path, sorted,
        or None when the file will not open."""
        rc, out = self.run(["ls", "-R", "-f", "-q", self.path], passphrase)
        if rc != 0:
            return None
        # An empty group is listed with a line `[empty]` in it.
        return sorted(line for line in out.splitlines() if not line.endswith("/[empty]"))

    def show(self, passphrase, entry, attrs):
        """`show -s -a …`: each attribute's value in turn, each followed by a
        line feed, or None when it fails (a file that will not open, an
        entry or an attribute it does not have)."""
        args = ["show", "-s", "-q", self.path, entry]
        for a in attrs:
            args += ["-a", a]
        rc, out = self.run(args, passphrase)
        return out if rc == 0 else None

    def attachment(self, passphrase, entry, name):
        """The bytes `attachment-export` writes of an attachment, or None
        (`--stdout` goes through a text stream and changes bytes that are
        not UTF-8, so the export is to a file)."""
        out = os.path.join(self.dir.name, "attachment.out")
        rc, _ = self.run(["attachment-export", "-q", self.path, entry, name, out], passphrase)
        if rc != 0:
            return None
        with open(out, "rb") as f:
            data = f.read()
        os.unlink(out)
        return data

    def attachments(self, passphrase, entry):
        """The names of an entry's attachments (`show --show-attachments`), or None."""
        rc, out = self.run(["show", "-q", "--show-attachments", self.path, entry], passphrase)
        if rc != 0:
            return None
        names, seen = [], False
        for line in out.splitlines():
            if line == "Attachments:":
                seen = True
            elif seen and line.startswith("  ") and line.endswith(" B)"):
                names.append(line[2:line.rindex(" (")])
        return names

    # Writing: each is one command that saves the file.

    def mkdir(self, passphrase, group):
        return self.run(["mkdir", "-q", self.path, group], passphrase)[0] == 0

    def add(self, passphrase, entry, user, password, url):
        return self.run(["add", "-q", "-p", "-u", user, "--url", url, self.path, entry],
                        passphrase, password + "\n")[0] == 0

    def edit_notes(self, passphrase, entry, notes):
        return self.run(["edit", "-q", "--notes", notes, self.path, entry], passphrase)[0] == 0

    def attach(self, passphrase, entry, name, source):
        return self.run(["attachment-import", "-q", self.path, entry, name, source],
                        passphrase)[0] == 0
