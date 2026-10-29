"""Bitcoin Core as a reference: bitcoind on regtest with no network
(F79, docs/CROSSCHECK.md §10.4).

The release is not vendored. `tools/reference/core/release.pin` names
it and pins each tarball's SHA-256; `fetch_core.py` fetches the one for
this machine once, after checking the signed SHA256SUMS, into the folder
`FARADAY_BITCOIN_CORE` names (default `~/.cache/faraday/bitcoin-core`,
under `XDG_CACHE_HOME` when that is set). Every run hashes the tarball
against the pin and unpacks `bitcoind` and `bitcoin-cli` from it into a
scratch folder, so what runs is what was checked. Missing or mismatched,
the cross-check fails naming it.

The main process starts one node for the run (`Node`), with a scratch
datadir, `-regtest -networkactive=0 -listen=0 -connect=0`, no seeds, no
wallet, RPC on loopback only with a cookie, and stops it and deletes the
folders on every way out; the node is also killed if the main process
dies (`PR_SET_PDEATHSIG`). The workers reach it through
`FARADAY_CROSSCHECK_CORE` (`rpc()`), with the vendored
`test_framework/authproxy.py`. Standard library only.
"""

import ctypes
import fcntl
import hashlib
import json
import os
import pathlib
import platform
import shutil
import signal
import socket
import struct
import subprocess
import tarfile
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
PIN = ROOT / "tools" / "reference" / "core" / "release.pin"
ENV_DIR = "FARADAY_BITCOIN_CORE"
ENV_RPC = "FARADAY_CROSSCHECK_CORE"
FETCH = "python3 -I -B tools/crosscheck/fetch_core.py"


class CoreError(Exception):
    """Bitcoin Core is missing, is not the pinned release, or did not
    start or answer as it should."""


def pin():
    """The pinned release: version, url, guix.sigs repository and commit,
    the signatures required, and each tarball's SHA-256."""
    out = {"sha256": {}}
    for line in PIN.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        key, *rest = line.split()
        if key == "sha256":
            out["sha256"][rest[0]] = rest[1]
        elif key == "guix-sigs":
            out["guix_repo"], out["guix_commit"] = rest
        elif key == "min-signatures":
            out["min_signatures"] = int(rest[0])
        else:
            out[key] = rest[0]
    return out


def tarball_name(p=None, machine=None):
    """The release tarball for this machine (x86_64 or aarch64 Linux)."""
    p = p or pin()
    machine = machine or platform.machine()
    arch = {"x86_64": "x86_64", "amd64": "x86_64", "aarch64": "aarch64",
            "arm64": "aarch64"}.get(machine.lower())
    if platform.system() != "Linux" or arch is None:
        raise CoreError(f"Bitcoin Core: no pinned release for {platform.system()} {machine}; "
                        "the cross-check runs on x86_64 or aarch64 Linux")
    name = f"bitcoin-{p['version']}-{arch}-linux-gnu.tar.gz"
    if name not in p["sha256"]:
        raise CoreError(f"Bitcoin Core: {name} is not in {PIN.relative_to(ROOT)}")
    return name


def folder():
    """Where the tarball is kept: `FARADAY_BITCOIN_CORE`, or the user's
    cache folder."""
    if os.environ.get(ENV_DIR):
        return pathlib.Path(os.environ[ENV_DIR]).expanduser()
    cache = os.environ.get("XDG_CACHE_HOME") or str(pathlib.Path.home() / ".cache")
    return pathlib.Path(cache) / "faraday" / "bitcoin-core"


def sha256_file(path):
    """Costs a read of the whole file (about 50 MB for a tarball)."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def checked_tarball():
    """The tarball's path, its name and its SHA-256, refused unless it is
    there and matches the pin. Costs a read of the tarball."""
    p = pin()
    name = tarball_name(p)
    path = folder() / name
    if not path.is_file():
        raise CoreError(f"Bitcoin Core {p['version']} is missing: no {path} "
                        f"(set {ENV_DIR}, or fetch it once with `{FETCH}`; "
                        "docs/DEVELOPING.md)")
    got = sha256_file(path)
    if got != p["sha256"][name]:
        raise CoreError(f"Bitcoin Core: {path} has SHA-256 {got}, pinned "
                        f"{p['sha256'][name]} in {PIN.relative_to(ROOT)}; "
                        f"fetch it again with `{FETCH}`")
    return path, name, got


def unpack(tarball, version, into):
    """`bitcoind` and `bitcoin-cli` from the tarball, into `into`. Costs
    a decompression of the tarball."""
    out = {}
    with tarfile.open(tarball) as t:
        for prog in ("bitcoind", "bitcoin-cli"):
            member = t.getmember(f"bitcoin-{version}/bin/{prog}")
            if not member.isfile():
                raise CoreError(f"Bitcoin Core: {member.name} in {tarball} is not a file")
            src = t.extractfile(member)
            dst = pathlib.Path(into) / prog
            with open(dst, "wb") as f:
                shutil.copyfileobj(src, f)
            dst.chmod(0o700)
            out[prog] = dst
    return out


SIOCGIFFLAGS = 0x8913
SIOCSIFFLAGS = 0x8914
IFF_UP = 0x1


def loopback_up():
    """Brings up `lo` in this network namespace, the one interface the
    cross-check's namespace has (run.sh), so RPC reaches bitcoind on
    loopback. Nothing else is brought up or routed."""
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
        req = struct.pack("16sH14x", b"lo", 0)
        flags = struct.unpack("16sH14x", fcntl.ioctl(s, SIOCGIFFLAGS, req))[1]
        if flags & IFF_UP:
            return
        fcntl.ioctl(s, SIOCSIFFLAGS, struct.pack("16sH14x", b"lo", flags | IFF_UP))


def _die_with_parent():
    """In the child, before exec: killed when the process that started it
    dies, however it dies."""
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl(1, signal.SIGKILL)  # PR_SET_PDEATHSIG


def _free_port():
    with socket.socket() as s:
        s.bind(("localhost", 0))
        return s.getsockname()[1]


class Node:
    """One bitcoind for the run, on regtest, offline. Use as a context
    manager: it is stopped and its folders deleted on every way out."""

    def __init__(self, rpc_threads=4):
        self.rpc_threads = max(4, rpc_threads)
        self.proc = None
        self.scratch = None
        self.info = None

    def __enter__(self):
        try:
            self.start()
        except BaseException:
            self.stop()
            raise
        return self

    def __exit__(self, *exc):
        self.stop()
        return False

    def start(self):
        """Costs a tarball read and decompression and a node start (about
        two seconds)."""
        p = pin()
        tarball, name, digest = checked_tarball()
        self.scratch = pathlib.Path(tempfile.mkdtemp(prefix="faraday-core-"))
        bins = unpack(tarball, p["version"], self.scratch)
        datadir = self.scratch / "data"
        datadir.mkdir()
        port = _free_port()
        common_args = [f"-datadir={datadir}", "-regtest", f"-rpcport={port}"]
        self.proc = subprocess.Popen(
            [str(bins["bitcoind"]), *common_args, "-networkactive=0", "-listen=0",
             "-connect=0", "-dnsseed=0", "-fixedseeds=0", "-discover=0", "-disablewallet",
             # No -rpcbind or -rpcallowip: Core then binds RPC to loopback only.
             "-server", f"-rpcthreads={self.rpc_threads}", "-rpcworkqueue=256",
             "-printtoconsole=0"],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
            stderr=open(self.scratch / "stderr", "wb"), preexec_fn=_die_with_parent,
        )
        cli = subprocess.run(
            [str(bins["bitcoin-cli"]), *common_args, "-rpcwait", "-rpcwaittimeout=60",
             "getnetworkinfo"], capture_output=True, text=True, timeout=90)
        if cli.returncode != 0:
            said = (self.scratch / "stderr").read_text(errors="replace").strip()
            raise CoreError(f"Bitcoin Core: bitcoind did not answer: {cli.stderr.strip()} "
                            f"{said}".strip())
        net = json.loads(cli.stdout)
        if net.get("networkactive") is not False or net.get("connections") != 0:
            raise CoreError(f"Bitcoin Core: the node's network is not off: {net}")
        want = "/Satoshi:" + p["version"]
        if not net.get("subversion", "").startswith(want):
            raise CoreError(f"Bitcoin Core: the node says {net.get('subversion')}, "
                            f"pinned {p['version']}")
        cookie = (datadir / "regtest" / ".cookie").read_text().strip()
        user, password = cookie.split(":", 1)
        self.info = {"url": f"http://{user}:{password}@localhost:{port}",
                     "version": net["subversion"].strip("/").split(":")[1],
                     "tarball": name, "sha256": digest}

    def stop(self):
        """Asks the node to stop, waits, kills it if it does not, and
        deletes the scratch folder."""
        if self.proc and self.proc.poll() is None:
            try:
                self.proc.send_signal(signal.SIGTERM)  # bitcoind shuts down cleanly
                self.proc.wait(timeout=30)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        self.proc = None
        if self.scratch:
            shutil.rmtree(self.scratch, ignore_errors=True)
            self.scratch = None

    def environ(self):
        """What a worker needs to reach the node."""
        return {ENV_RPC: json.dumps(self.info)}


_proxy = None


def batch(calls):
    """Costs one request to the node: Core's answers to `calls`, each
    `(method, param, …)`, sent as one JSON-RPC batch and given back in
    order. An answer that is an error raises CoreError naming the call."""
    node = rpc()
    reqs = [getattr(node, method).get_request(*params) for method, *params in calls]
    if not reqs:
        return []
    by_id = {r["id"]: r for r in node.batch(reqs)}
    out = []
    for req in reqs:
        r = by_id.get(req["id"])
        if r is None or r.get("error"):
            raise CoreError(f"Bitcoin Core: {req['method']} answered "
                            f"{None if r is None else r['error']}")
        out.append(r["result"])
    return out


def answers(calls):
    """Costs one request to the node: Core's answer to each of `calls`,
    `(method, param, …)`, sent as one JSON-RPC batch, in order, each
    ("ok", result) or ("err", Core's message)."""
    node = rpc()
    reqs = [getattr(node, method).get_request(*params) for method, *params in calls]
    if not reqs:
        return []
    by_id = {r["id"]: r for r in node.batch(reqs)}
    out = []
    for req in reqs:
        r = by_id.get(req["id"])
        if r is None:
            raise CoreError(f"Bitcoin Core: no answer to {req['method']}")
        out.append(("err", r["error"].get("message")) if r.get("error") else ("ok", r["result"]))
    return out


# ----- Coins for the areas that spend on the node (`Area.FUND`) ----------------
#
# The node's coinbases pay P2WSH(OP_TRUE), which anyone spends with the
# witness script alone, and each is fanned out into PER coins of one
# value. Coin k is output k % PER of fan-out k // PER: the same in every
# run, as a regtest coinbase's txid depends only on its height and what
# it pays. An area spends its coins in funding transactions of its own,
# which the run submits and buries before any case is compared.

OP_TRUE = b"\x51"
PER = 1000
FANOUT_FEE = 100_000


def op_true_spk():
    return b"\x00\x20" + hashlib.sha256(OP_TRUE).digest()


def _framework():
    import sys
    folder = str(ROOT / "tools" / "reference" / "core")
    if folder not in sys.path:
        sys.path.insert(0, folder)
    import test_framework.messages as messages
    return messages


def _op_true_descriptor():
    raw = f"raw({op_true_spk().hex()})"
    return rpc().getdescriptorinfo(raw)["descriptor"]


def make_coins(count):
    """Costs mining 100 blocks and one per thousand coins, and a fan-out
    each: `count` coins spendable by OP_TRUE, as the areas are told of
    them ({"fanout": txids, "per", "value"})."""
    m = _framework()
    node = rpc()
    desc = _op_true_descriptor()
    fanouts = max(1, -(-count // PER))
    if node.getblockcount() != 0:
        raise CoreError("Bitcoin Core: the run's node is not at its genesis block")
    hashes = node.generatetodescriptor(fanouts, desc)
    node.generatetodescriptor(100, desc)
    txids, value = [], None
    for h in hashes:
        coinbase = node.getblock(h)["tx"][0]
        reward = int(round(node.getrawtransaction(coinbase, True, h)["vout"][0]["value"] * 10**8))
        each = (reward - FANOUT_FEE) // PER
        value = each if value is None else min(value, each)
        txids.append(coinbase)
    out = []
    for coinbase in txids:
        tx = m.CTransaction()
        tx.version = 2
        tx.vin = [m.CTxIn(m.COutPoint(int(coinbase, 16), 0), b"", 0xFFFFFFFF)]
        tx.vout = [m.CTxOut(value, op_true_spk()) for _ in range(PER)]
        w = m.CTxInWitness()
        w.scriptWitness.stack = [OP_TRUE]
        tx.wit.vtxinwit = [w]
        out.append(tx.serialize().hex())
    fanned = submit(out)
    node.generatetodescriptor(1, desc)
    return {"fanout": fanned, "per": PER, "value": value}


def submit(hexes, chunk=500):
    """Costs a request to the node a `chunk` transactions: each sent to
    its mempool (no fee limit); their txids, in order."""
    out = []
    for at in range(0, len(hexes), chunk):
        out += batch([("sendrawtransaction", h, 0) for h in hexes[at:at + chunk]])
    return out


def bury(depth):
    """Costs mining a block for each block's worth of the mempool and
    `depth` more: every transaction sent is confirmed `depth` deep."""
    node = rpc()
    desc = _op_true_descriptor()
    while node.getmempoolinfo()["size"]:
        node.generatetodescriptor(1, desc)
    if depth:
        node.generatetodescriptor(depth, desc)
    return node.getblockcount()


def rpc():
    """This process's RPC client to the run's node, made on first use."""
    global _proxy
    if _proxy is None:
        raw = os.environ.get(ENV_RPC)
        if not raw:
            raise CoreError("Bitcoin Core: no node for this run (the area needs one, and "
                            "crosscheck.py starts it when an area says CORE)")
        import sys
        folder = str(ROOT / "tools" / "reference" / "core")
        if folder not in sys.path:
            sys.path.insert(0, folder)
        from test_framework.authproxy import AuthServiceProxy
        _proxy = AuthServiceProxy(json.loads(raw)["url"], timeout=120)
        # A connection of its own for each request: Core closes one left
        # idle for 30 seconds, and a worker can wait longer between batches.
        _proxy.reuse_http_connections = False
    return _proxy
