#!/usr/bin/env python3
"""BIP 445's reference implementation as the second share of the
fixture spend (docs/PLANNING.md §16.103).

The fixture writer records one signing session in
`tools/vectors/psbt/wallet-threshold-transcript.json`: the group, the
signer set, their public shares and nonces, the tweaks, the message, and
share 1's secret share and secret nonce, which are this fixture group's
and no one else's. This replays that session with the vendored reference
and asserts three things:

* the reference's `sign` gives the partial signature this tree wrote;
* every partial signature verifies under `partial_sig_verify`;
* `partial_sig_agg` gives the 64-byte signature Bitcoin Core accepted.

Standard library only; the reference and its curve library are vendored
under `tools/reference/bip445/`. Run it with `just threshold-reference`.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
REF = ROOT / "tools" / "reference" / "bip445"
sys.path.insert(0, str(REF))
sys.path.insert(0, str(REF / "secp256k1lab"))

from frost_ref.signing import (  # noqa: E402
    SessionContext,
    nonce_agg,
    partial_sig_agg,
    partial_sig_verify,
    sign,
)


def main() -> int:
    path = ROOT / "tools" / "vectors" / "psbt" / "wallet-threshold-transcript.json"
    t = json.loads(path.read_text())
    ids = t["ids"]
    pubshares = [bytes.fromhex(p) for p in t["pubshares"]]
    pubnonces = [bytes.fromhex(p) for p in t["pubnonces"]]
    tweaks = [bytes.fromhex(x["tweak"]) for x in t["tweaks"]]
    is_xonly = [bool(x["is_xonly"]) for x in t["tweaks"]]
    thresh_pk = bytes.fromhex(t["thresh_pk"])
    msg = bytes.fromhex(t["msg"])
    psigs = [bytes.fromhex(p) for p in t["psigs"]]
    my_id = t["my_id"]
    at = ids.index(my_id)

    ctx = SessionContext(
        t["n"], t["t"], ids, pubshares, thresh_pk, nonce_agg(pubnonces), tweaks,
        is_xonly, msg,
    )
    mine = sign(
        bytearray(bytes.fromhex(t["secnonce"])), bytes.fromhex(t["secshare"]), my_id, ctx
    )
    assert mine == psigs[at], (
        f"the reference signs {mine.hex()}; this tree wrote {psigs[at].hex()}"
    )
    print(f"sign as share {my_id}: {mine.hex()}")

    for i, psig in enumerate(psigs):
        ok = partial_sig_verify(
            psig, pubnonces, t["n"], t["t"], ids, pubshares, thresh_pk, tweaks,
            is_xonly, msg, i,
        )
        assert ok, f"partial signature {i} does not verify"
        print(f"partial_sig_verify share {ids[i]}: ok")

    agg = partial_sig_agg(psigs, ctx)
    expected = bytes.fromhex(t["sig"])
    assert agg == expected, (
        f"the reference aggregates {agg.hex()}; this tree wrote {expected.hex()}"
    )
    print(f"partial_sig_agg: {agg.hex()}")
    print("threshold-reference: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
