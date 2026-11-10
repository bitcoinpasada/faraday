"""The outside programs that read and write KDBX files for the `vault`
area: pykeepass, the library KeePass tooling in Python uses, and
`keepassxc-cli`, KeePassXC's command line. Neither knows anything of this
repository. Both are the distribution's packages (Debian and Ubuntu:
`python3-pykeepass`, `keepassxc`); an area that asks for one stops and
names it when it is missing, so a run without it fails rather than
passing without the check.

`keepassxc-cli` runs on a file in a folder of its own and a home folder of
its own, so it reads and writes nothing else, with the passphrase on standard
input: one `open` shell for the commands of a file (`Kpxc`), a process of its
own for a command the shell cannot take.
"""

import os
import re
import secrets
import selectors
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
    context manager; `path` is the file.

    Commands run in one `keepassxc-cli open` shell, started at the first
    command with its passphrase and kept until a one-shot run changes the
    file or the object is closed, so the file is opened (one Argon2 run) once
    and not for each command. What the shell does, and what is done about it:

    - It reads the passphrase and then writes its prompt, the database's name
      (the file's when it has none) and `> `, the first thing on standard
      output; an open that fails ends the process with a message on standard
      error. The prompt is not known beforehand (a Faraday vault has a name
      of its own), so the first end marker (below) is sent when the first
      byte of it has come, and the prompt is what stands before that
      marker's echo. A line sent before the passphrase is read is lost,
      which is why it waits for that byte.
    - It echoes each line it reads, writes a command's output to standard
      output and its errors to standard error: two pipes, so unordered. A
      command's end is a second line after it, `ls` of a group named by a
      random token: the shell echoes it on standard output and answers it
      with `Cannot find group <token>.` on standard error, both after
      everything the command wrote, and neither can be in what a command
      prints. No timeout and no guess finds the end.
    - A command that fails writes to standard error and one that succeeds
      does not (`-q`), so an empty standard error is a success: a shell has
      no exit status.
    - `add -p` reads the password from what follows its prompt, which it
      prints only without `-q`: the password is sent when the prompt is
      seen, and the end marker after the command has finished.
    - It splits a line at spaces and takes backslash and a double quote in
      double quotes as themselves after a backslash. A line cannot hold a
      line feed or another control character: a command with one in an
      argument (a note of several lines, a title with a carriage return) is
      a one-shot run, which first closes the shell, as it changes the file
      under it.
    - A wide terminal (`COLUMNS`) stops its line editor redrawing a long line
      in its echo."""

    PROMPT = b"vault.kdbx> "
    PASSWORD_PROMPT = b"Enter password for new entry: "

    def __init__(self, data):
        self.exe = need_keepassxc_cli()
        self.dir = tempfile.TemporaryDirectory(prefix="kpxc-")
        self.path = os.path.join(self.dir.name, "vault.kdbx")
        home = os.path.join(self.dir.name, "home")
        os.mkdir(home)
        self.env = {"PATH": os.environ.get("PATH", "/usr/bin"), "HOME": home,
                    "XDG_CONFIG_HOME": home, "XDG_CACHE_HOME": home, "XDG_DATA_HOME": home,
                    "XDG_RUNTIME_DIR": self.dir.name, "QT_QPA_PLATFORM": "offscreen", "LC_ALL": "C.UTF-8",
                    "COLUMNS": "1000000", "LINES": "1000"}
        with open(self.path, "wb") as f:
            f.write(data)
        self.error = ""
        self.shell = None  # (process, selector, {"o": bytes, "e": bytes}, passphrase) or None
        self.refused = set()  # the passphrases the file did not open with

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close_shell()
        self.dir.cleanup()

    def data(self):
        with open(self.path, "rb") as f:
            return f.read()

    def run(self, args, passphrase, more=""):
        """One `keepassxc-cli` process for one command: (exit status, standard
        output); `more` follows the passphrase on standard input (the entry's
        password, for `add -p`). What the command wrote to standard error is
        kept in `self.error`, for a message when it fails."""
        run = subprocess.run([self.exe, *args], input=(passphrase + "\n" + more).encode(),
                             capture_output=True, env=self.env, timeout=300)
        self.error = run.stderr.decode("utf-8", "replace").strip()
        return run.returncode, run.stdout.decode("utf-8", "surrogateescape")

    # The shell.

    def close_shell(self):
        if self.shell is None:
            return
        proc, sel = self.shell[0], self.shell[1]
        self.shell = None
        try:
            proc.stdin.write(b"exit\n")
        except OSError:  # it has ended already
            pass
        try:
            proc.stdin.close()
        except OSError:
            pass
        while sel.get_map():  # drained, so that it never waits on a full pipe
            for key, _ in sel.select():
                if not os.read(key.fd, 65536):
                    sel.unregister(key.fileobj)
        proc.wait()
        proc.stdout.close()
        proc.stderr.close()
        sel.close()

    def pump(self):
        """Reads what is ready, from either pipe; False when both have ended."""
        sel, buf = self.shell[1], self.shell[2]
        if not sel.get_map():
            return False
        for key, _ in sel.select():
            chunk = os.read(key.fd, 65536)
            if chunk:
                buf[key.data] += chunk
            else:
                sel.unregister(key.fileobj)
        return True

    def pump_or_fail(self):
        if not self.pump():
            raise AssertionError("keepassxc-cli's shell ended in the middle of a command")

    def open_shell(self, passphrase):
        """Opens the file in a shell: True, or False with `self.error` when it will not open."""
        proc = subprocess.Popen([self.exe, "open", self.path], stdin=subprocess.PIPE,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=self.env)
        sel = selectors.DefaultSelector()
        for pipe, name in ((proc.stdout, "o"), (proc.stderr, "e")):
            os.set_blocking(pipe.fileno(), False)
            sel.register(pipe, selectors.EVENT_READ, name)
        self.shell = (proc, sel, {"o": b"", "e": b""}, passphrase)
        buf = self.shell[2]
        try:
            proc.stdin.write(passphrase.encode() + b"\n")
            proc.stdin.flush()
        except OSError:
            pass
        self.prompt = None
        while not buf["o"] and self.pump():
            pass
        if not buf["o"]:
            self.error = buf["e"].decode("utf-8", "replace").strip()
            self.close_shell()
            return False
        self.end_of_command(proc)  # learns the prompt
        return True

    def end_of_command(self, proc):
        """Sends the end marker and reads to it: (what the command wrote to
        standard output, what it wrote to standard error), the prompt before
        the marker's echo left out. What follows the marker stays in the
        buffers."""
        buf = self.shell[2]
        token = secrets.token_hex(8)
        proc.stdin.write(f"ls {token}\n".encode())
        proc.stdin.flush()
        line = f"ls {token}\n".encode()
        refusal = f"Cannot find group {token}.\n".encode()
        while line not in buf["o"] or refusal not in buf["e"]:
            self.pump_or_fail()
        text, buf["o"] = buf["o"].split(line, 1)
        failed, buf["e"] = buf["e"].split(refusal, 1)
        if self.prompt is None:
            self.prompt, text = text, b""
        elif text.endswith(self.prompt):
            text = text[:-len(self.prompt)]
        else:
            raise AssertionError(f"keepassxc-cli's shell wrote {text[-200:]!r}, not its prompt, before the end marker")
        return text, failed

    @staticmethod
    def quote(arg):
        """`arg` as one argument of a line of the shell, or None when a line cannot hold it."""
        if any(c < " " or c == "\x7f" or "\ud800" <= c <= "\udfff" for c in arg):
            return None
        return '"' + arg.replace("\\", "\\\\").replace('"', '\\"') + '"'

    def command(self, passphrase, args, more=None):
        """`args` (the command and its arguments, the database left out) on the
        opened file: (succeeded, standard output), or None when the shell
        cannot take it (a line cannot hold an argument, or the shell was
        opened with another passphrase) and it is for the caller to run it on
        its own. `more` is the password `add -p` asks for. What a failure
        wrote is in `self.error`."""
        quoted = [self.quote(a) for a in args[1:]]
        if None in quoted or (self.shell is not None and self.shell[3] != passphrase):
            return None
        if self.shell is None:
            if passphrase in self.refused or not self.open_shell(passphrase):
                self.refused.add(passphrase)
                return False, ""
        proc, _, buf, _ = self.shell
        # What is left of the last command is the prompt of this one, whole or in part.
        prompt = self.prompt
        while len(buf["o"]) < len(prompt):
            self.pump_or_fail()
        if not buf["o"].startswith(prompt):
            raise AssertionError(f"keepassxc-cli's shell wrote {buf['o'][:200]!r} where a prompt belongs")
        buf["o"] = buf["o"][len(prompt):]
        line = args[0] + "".join(" " + q for q in quoted)
        echo = line.encode() + b"\n"
        proc.stdin.write(echo)
        proc.stdin.flush()
        if more is not None:
            # Once the password is asked for (or, the command having failed first,
            # the prompt is back), and nothing behind it before the command ends.
            while not (buf["o"].startswith(echo)
                       and buf["o"][len(echo):].endswith((self.PASSWORD_PROMPT, prompt))):
                self.pump_or_fail()
            if buf["o"].endswith(self.PASSWORD_PROMPT):
                proc.stdin.write(more.encode() + b"\n")
                proc.stdin.flush()
                while not buf["o"].endswith(prompt):
                    self.pump_or_fail()
        text, failed = self.end_of_command(proc)
        if not text.startswith(echo):
            raise AssertionError(f"keepassxc-cli's shell echoed {text[:200]!r} for {line!r}")
        self.error = failed.decode("utf-8", "replace").strip()
        return self.error == "", text[len(echo):].decode("utf-8", "surrogateescape")

    def go(self, passphrase, cmd, options, positionals, more=None, writes=False):
        """`cmd` with its options and the positional arguments after the
        database: (succeeded, standard output), in the shell when it can take
        it and else in one process of its own. `more` is the password `add -p`
        asks for. What a failure wrote is in `self.error`."""
        done = self.command(passphrase, [cmd, *options, *positionals], more)
        if done is not None:
            return done
        if writes:
            self.close_shell()
        rc, out = self.run([cmd, *options, self.path, *positionals], passphrase,
                           "" if more is None else more + "\n")
        return rc == 0, out

    def entries(self, passphrase):
        """`ls -R -f`: every group (ending in `/`) and entry by its path, sorted,
        or None when the file will not open."""
        ok, out = self.go(passphrase, "ls", ["-R", "-f", "-q"], [])
        if not ok:
            return None
        # An empty group is listed with a line `[empty]` in it, a bare one for an empty root.
        # Lines end at a line feed only: a Title may hold a carriage return.
        return sorted(line for line in out.split("\n")
                      if line and line != "[empty]" and not line.endswith("/[empty]"))

    def show(self, passphrase, entry, attrs):
        """`show -s -a …`: each attribute's value in turn, each followed by a
        line feed, or None when it fails (a file that will not open, an
        entry or an attribute it does not have)."""
        options = ["-s", "-q"]
        for a in attrs:
            options += ["-a", a]
        ok, out = self.go(passphrase, "show", options, [entry])
        return out if ok else None

    def attachment(self, passphrase, entry, name):
        """The bytes `attachment-export` writes of an attachment, or None
        (`--stdout` goes through a text stream and changes bytes that are
        not UTF-8, so the export is to a file)."""
        out = os.path.join(self.dir.name, "attachment.out")
        ok, _ = self.go(passphrase, "attachment-export", ["-q"], [entry, name, out])
        if not ok:
            return None
        with open(out, "rb") as f:
            data = f.read()
        os.unlink(out)
        return data

    def attachments(self, passphrase, entry):
        """The names of an entry's attachments (`show --show-attachments`), or None."""
        ok, out = self.go(passphrase, "show", ["-q", "--show-attachments"], [entry])
        if not ok:
            return None
        names, seen = [], False
        for line in out.split("\n"):
            if line == "Attachments:":
                seen = True
            elif seen and line.startswith("  ") and re.search(r" \([0-9.]+ [KMG]?i?B\)$", line):
                names.append(line[2:line.rindex(" (")])
        return names

    # Writing: each is one command that saves the file.

    def mkdir(self, passphrase, group):
        return self.go(passphrase, "mkdir", ["-q"], [group], writes=True)[0]

    def add(self, passphrase, entry, user, password, url):
        # Without `-q`: the shell is told by the password prompt when to send the password.
        return self.go(passphrase, "add", ["-p", "-u", user, "--url", url], [entry], password, writes=True)[0]

    def edit_notes(self, passphrase, entry, notes):
        # A note of several lines does not fit a line of the shell: one process.
        return self.go(passphrase, "edit", ["-q", "--notes", notes], [entry], writes=True)[0]

    def attach(self, passphrase, entry, name, source):
        return self.go(passphrase, "attachment-import", ["-q"], [entry, name, source], writes=True)[0]
