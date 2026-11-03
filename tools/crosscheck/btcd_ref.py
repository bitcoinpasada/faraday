"""A released btcd's answer program (`tools/reference/btcd/`, vendored Go,
built by `run.sh` with the Go toolchain, offline) for the areas that ask it:
`hdkeychain` for `bip32`, `txscript`'s control-block check for `taproot`.
One process for an area's batches: a JSON array of requests a line, the
array of answers back, in order (the program's header says the requests).
"""

import json
import os
import subprocess

from common import ROOT, HarnessError


def btcd_exe():
    path = os.environ.get("FARADAY_BTCD_REF") or str(ROOT / "out/crosscheck/btcd-ref")
    if not os.path.isfile(path):
        raise HarnessError(
            f"btcd's answer program is not built ({path}): tools/crosscheck/run.sh builds it "
            "from tools/reference/btcd/ with the Go toolchain")
    return path


class Btcd:
    def __init__(self):
        self.proc = subprocess.Popen([btcd_exe()], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     text=True, bufsize=1)

    def ask(self, request):
        """The answer to one request (a dict), as the program gives it."""
        self.proc.stdin.write(json.dumps([request]) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise HarnessError("btcd's answer program exited")
        return json.loads(line)[0]

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()
