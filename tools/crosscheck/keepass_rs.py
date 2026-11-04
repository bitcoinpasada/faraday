"""keepass-rs, the Rust KeePass library, as the vault area's judge of the file
format (F193 point 1): a program of its own, `tools/crosscheck/keepass-rs/`,
that reads a KDBX file under a passphrase and prints it as JSON, and writes
one from a JSON description. It judges the format only, both ways: it is not
counted for the primitives (it runs RustCrypto, as the app does;
`docs/ESTABLISHED.md` §9.5).
"""

import json
import os
import subprocess
import tempfile

from common import ROOT, HarnessError

# What keepass-rs 0.17 writes and reads that Faraday refuses on purpose
# (`docs/VAULT.md` §4): {name: (the description's settings, why Faraday refuses)}.
# The area makes each file with keepass-rs and expects Faraday to refuse it.
REFUSED = {
    "Twofish": ({"cipher": "twofish"}, "Twofish as the outer cipher"),
    "AES-KDF": ({"kdf": "aes", "rounds": 1000}, "AES-KDF"),
    "Salsa20": ({"inner": "salsa20"}, "Salsa20 as the inner stream"),
}


def exe():
    """The program: built by `tools/crosscheck/run.sh` (`FARADAY_KEEPASS_RS` names another)."""
    path = os.environ.get("FARADAY_KEEPASS_RS") or str(
        ROOT / "out/crosscheck/keepass-rs-target/release/keepass-rs")
    if not os.access(path, os.X_OK):
        raise HarnessError(
            f"the keepass-rs program is not built ({path}): tools/crosscheck/run.sh builds it "
            "from tools/crosscheck/keepass-rs/")
    return path


def read(data, passphrase):
    """keepass-rs's view of `data`, or None when it will not open it."""
    fd, path = tempfile.mkstemp(suffix=".kdbx")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data)
        run = subprocess.run([exe(), "read", path], input=(passphrase + "\n").encode(),
                             capture_output=True, timeout=300)
    finally:
        os.unlink(path)
    if run.returncode == 3:
        return None
    if run.returncode != 0:
        raise HarnessError(f"keepass-rs failed: {run.stderr.decode(errors='replace')[:300]}")
    return json.loads(run.stdout)


def write(description, passphrase):
    """The KDBX file keepass-rs makes of `description`, or None when it will not."""
    fd, path = tempfile.mkstemp(suffix=".kdbx")
    os.close(fd)
    try:
        run = subprocess.run([exe(), "write", path],
                             input=(passphrase + "\n" + json.dumps(description)).encode(),
                             capture_output=True, timeout=300)
        if run.returncode == 3:
            return None
        if run.returncode != 0:
            raise HarnessError(f"keepass-rs failed: {run.stderr.decode(errors='replace')[:300]}")
        with open(path, "rb") as f:
            return f.read()
    finally:
        os.unlink(path)


def _nothing(x):
    return None if x is None else str(x)


def differs(rs, v):
    """Where keepass-rs's view `rs` first differs from pykeepass's deep view `v`, as
    (what, keepass-rs, pykeepass), or None. Lists keepass-rs keeps in hash maps are
    compared sorted."""
    for k in ("label", "id"):
        if rs[k] != v[k]:
            return (k, rs[k], v[k])
    if sorted(map(tuple, rs["custom"])) != sorted(map(tuple, v["custom"])):
        return ("the vault's custom data", rs["custom"], v["custom"])
    if sorted(rs["groups"]) != sorted(v["groups"]):
        return ("groups", rs["groups"], v["groups"])
    a = {e["uuid"]: e for e in rs["entries"]}
    b = {e["uuid"]: e for e in v["entries"]}
    if a.keys() != b.keys():
        return ("entries", sorted(a), sorted(b))
    for u, e in a.items():
        w = b[u]
        if e["path"] != w["path"]:
            return (f"entry {u} path", e["path"], w["path"])
        if sorted(map(tuple, e["fields"])) != sorted(map(tuple, w["fields"])):
            return (f"entry {u} fields", e["fields"], w["fields"])
        if sorted(map(tuple, e["custom"])) != sorted(map(tuple, w["custom"])):
            return (f"entry {u} custom data", e["custom"], w["custom"])
        if sorted(e["tags"]) != sorted(w["tags"]):
            return (f"entry {u} tags", e["tags"], w["tags"])
        if sorted(map(tuple, e["attachments"])) != sorted(map(tuple, w["attachments"])):
            return (f"entry {u} attachments", e["attachments"], w["attachments"])
        if _nothing(e["icon"]) != _nothing(w["icon"]):
            return (f"entry {u} icon", e["icon"], w["icon"])
    return None
