#!/usr/bin/env python3
"""Faraday against Bitcoin Core, on regtest.

    faraday/tools/core-check.py OUT_DIR

For every test wallet (`testkit::kits`: single-sig of each script type,
multisig of each, Taproot multisig, a miniscript and a Taproot tree):

1. Faraday's Back up step writes the wallet's public files
   (`examples/interop.rs export`). Core takes Faraday's
   `<wallet>-bitcoin-core.json` into a watch-only descriptor wallet with
   `importdescriptors`, and Core's descriptor checksum of Faraday's
   `<wallet>-descriptor.txt` must equal the one Faraday wrote.
2. Core's first three receive and change addresses must equal Faraday's.
3. Core funds the wallet's first address and builds a spend of it with
   `walletcreatefundedpsbt`, as a coordinator does. Faraday loads the
   wallet, adds the test seeds a spend needs, opens the PSBT and signs
   (`examples/interop.rs sign`). The finished transaction Faraday writes
   must pass `testmempoolaccept`, be mined, and leave the change in
   Core's view of the wallet.
4. For each multisig, two Faraday sessions each sign alone, and Core
   combines their two signed PSBTs (`combinepsbt`, `finalizepsbt`) into a
   transaction it accepts.

Last, Core's own wallet export (`listdescriptors`, a receive and a change
descriptor per kind) must read in Faraday as its BIP-84 wallet, with
Core's first receive address.

Bitcoin Core (`bitcoind`, `bitcoin-cli`, v29 or later for multipath
descriptors) must be on PATH or in BITCOIN_BIN. Nothing is downloaded.
Exit status 0 only if every check held. Not part of `just`.
"""

import json
import os
import shutil
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
EXAMPLE = os.path.join(ROOT, "target", "debug", "examples", "interop")

# The seeds a spend needs: a multisig's quorum, else test key 1.
SIGNERS = {
    "savings": "bacon,zebra",
    "vault-nested": "bacon,zebra",
    "vault-legacy": "bacon,zebra",
    "taproot-multisig": "bacon,zebra",
}
MULTISIG = sorted(SIGNERS)

failures = []


def check(ok, what):
    print(("  ok   " if ok else "  FAIL ") + what)
    if not ok:
        failures.append(what)
    return ok


def tool(name):
    d = os.environ.get("BITCOIN_BIN")
    found = os.path.join(d, name) if d else shutil.which(name)
    if not found or not os.path.exists(found):
        sys.exit(f"core-check: {name} is not here (set BITCOIN_BIN)")
    return found


class Core:
    def __init__(self, datadir):
        self.datadir = datadir
        self.cli = tool("bitcoin-cli")
        os.makedirs(datadir, exist_ok=True)
        self.proc = subprocess.Popen(
            [
                tool("bitcoind"),
                "-regtest",
                f"-datadir={datadir}",
                "-port=18911",
                "-rpcport=18912",
                "-listen=0",
                "-fallbackfee=0.0002",
                "-txindex=0",
                "-printtoconsole=0",
            ]
        )
        for _ in range(100):
            try:
                self.rpc("getblockchaininfo")
                return
            except RuntimeError:
                time.sleep(0.2)
        sys.exit("core-check: bitcoind did not start")

    def rpc(self, method, *params, wallet=None):
        cmd = [self.cli, "-regtest", f"-datadir={self.datadir}", "-rpcport=18912"]
        if wallet:
            cmd.append(f"-rpcwallet={wallet}")
        cmd += ["-named" if any(isinstance(p, tuple) for p in params) else "-stdin", method]
        named = [p for p in params if isinstance(p, tuple)]
        plain = [p for p in params if not isinstance(p, tuple)]
        if named:
            cmd += [f"{k}={v if isinstance(v, str) else json.dumps(v)}" for k, v in named]
            r = subprocess.run(cmd, capture_output=True, text=True)
        else:
            args = "\n".join(p if isinstance(p, str) else json.dumps(p) for p in plain)
            r = subprocess.run(cmd, input=args, capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError(r.stderr.strip() or r.stdout.strip())
        out = r.stdout.strip()
        try:
            return json.loads(out)
        except json.JSONDecodeError:
            return out

    def stop(self):
        try:
            self.rpc("stop")
        except RuntimeError:
            pass
        self.proc.wait(timeout=60)


def faraday(*args):
    r = subprocess.run([EXAMPLE, *args], capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr.strip())
    return r.stdout.strip().splitlines()


def sign(kit, seeds, psbt_path, out):
    name = faraday("sign", kit, seeds, psbt_path, out)[-1]
    with open(os.path.join(out, name), "rb") as f:
        return name, f.read()


def address_kind(desc):
    if desc.startswith("tr("):
        return "bech32m"
    if desc.startswith(("wpkh(", "wsh(")):
        return "bech32"
    if desc.startswith(("sh(wpkh(", "sh(wsh(")):
        return "p2sh-segwit"
    return "legacy"


def one_kit(core, kit, desc, out):
    print(f"{kit}")
    d = os.path.join(out, kit)
    files = faraday("export", kit, d)
    with open(os.path.join(d, next(f for f in files if f.endswith("-descriptor.txt")))) as f:
        written = f.read().strip()
    info = core.rpc("getdescriptorinfo", written.split("#")[0])
    check(info["checksum"] == written.split("#")[1], "Core's checksum is the one Faraday wrote")

    core.rpc("createwallet", ("wallet_name", kit), ("disable_private_keys", True), ("blank", True))
    with open(os.path.join(d, next(f for f in files if f.endswith("-bitcoin-core.json")))) as f:
        requests = json.load(f)
    result = core.rpc("importdescriptors", requests, wallet=kit)
    if not check(all(r.get("success") for r in result), "Core imports <wallet>-bitcoin-core.json"):
        print(f"       {result}")
        return

    ours = [line.split() for line in faraday("addresses", kit, "3")]
    kind = address_kind(desc)
    theirs = [("0", str(i), core.rpc("getnewaddress", ("address_type", kind), wallet=kit)) for i in range(3)]
    theirs += [("1", str(i), core.rpc("getrawchangeaddress", ("address_type", kind), wallet=kit)) for i in range(3)]
    check([tuple(a) for a in ours] == theirs, "the first three receive and change addresses agree")

    first = ours[0][2]
    core.rpc("sendtoaddress", ("address", first), ("amount", 1.0), wallet="miner")
    core.rpc("generatetoaddress", 1, core.miner_address)
    to = core.rpc("getnewaddress", wallet="miner")
    made = core.rpc(
        "walletcreatefundedpsbt",
        ("outputs", [{to: 0.3}]),
        ("options", {"add_inputs": True, "change_type": kind}),
        wallet=kit,
    )
    psbt_path = os.path.join(d, "core-unsigned.psbt")
    with open(psbt_path, "w") as f:
        f.write(made["psbt"])

    try:
        name, signed = sign(kit, SIGNERS.get(kit, "bacon"), psbt_path, os.path.join(d, "signed"))
    except RuntimeError as e:
        check(False, f"Faraday signs Core's PSBT: {e}")
        return
    if not check(name.endswith("-final.txn"), f"Faraday finishes the transaction ({name})"):
        return
    raw = signed.decode().strip()
    accept = core.rpc("testmempoolaccept", [raw])[0]
    if not check(accept["allowed"], "Core accepts Faraday's finished transaction"):
        print(f"       {accept}")
        return
    core.rpc("sendrawtransaction", raw)
    core.rpc("generatetoaddress", 1, core.miner_address)
    left = core.rpc("getbalances", wallet=kit)["mine"]["trusted"]
    check(0.69 < left < 0.7, f"the change is the wallet's in Core ({left})")

    if kit in MULTISIG:
        made = core.rpc(
            "walletcreatefundedpsbt",
            ("outputs", [{to: 0.2}]),
            ("options", {"change_type": kind}),
            wallet=kit,
        )
        with open(psbt_path, "w") as f:
            f.write(made["psbt"])
        parts = []
        for seed in ("bacon", "summer"):
            try:
                name, signed = sign(kit, seed, psbt_path, os.path.join(d, f"part-{seed}"))
            except RuntimeError as e:
                check(False, f"Faraday signs alone with {seed}: {e}")
                return
            parts.append(signed)
        import base64

        combined = core.rpc("combinepsbt", [base64.b64encode(p).decode() for p in parts])
        final = core.rpc("finalizepsbt", combined)
        if check(final.get("complete"), "Core combines two Faraday signatures into a whole spend"):
            accept = core.rpc("testmempoolaccept", [final["hex"]])[0]
            check(accept["allowed"], "Core accepts the combined spend")


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    out = os.path.abspath(sys.argv[1])
    os.makedirs(out, exist_ok=True)
    subprocess.run(
        ["cargo", "build", "-q", "-p", "faraday-core", "--example", "interop"],
        cwd=ROOT,
        check=True,
    )
    datadir = os.path.join(out, f"core-{int(time.time())}")
    core = Core(datadir)
    try:
        version = core.rpc("getnetworkinfo")["subversion"]
        print(f"Bitcoin Core {version}, regtest, {datadir}")
        core.rpc("createwallet", "miner")
        core.miner_address = core.rpc("getnewaddress", wallet="miner")
        core.rpc("generatetoaddress", 101, core.miner_address)
        for line in faraday("kits"):
            kit, desc = line.split(" ", 1)
            try:
                one_kit(core, kit, desc, out)
            except RuntimeError as e:
                check(False, f"{kit}: {e}")

        print("Core's own wallet, exported with listdescriptors, into Faraday")
        core.rpc("createwallet", "core-hot")
        listed = core.rpc("listdescriptors", wallet="core-hot")
        path = os.path.join(out, "core-listdescriptors.json")
        with open(path, "w") as f:
            json.dump(listed, f, indent=2)
        read = faraday("read", path)
        if check(len(read) == 3 and read[1].startswith("wpkh("), "Faraday reads it as its BIP-84 wallet"):
            theirs = core.rpc("getnewaddress", ("address_type", "bech32"), wallet="core-hot")
            check(read[2] == theirs, "its first receive address is Core's")
    finally:
        core.stop()
    print(f"\n{len(failures)} failed" if failures else "\nevery check held")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
