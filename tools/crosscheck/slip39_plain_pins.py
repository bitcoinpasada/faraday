"""The master fingerprints of the SLIP-39 vectors that recover, with no
passphrase, worked out by python-shamir-mnemonic (Trezor's reference) and
btclib, neither written by Faraday. They are the pins `SLIP39_PLAIN` in
`faraday/faraday-samples/tests/all/published.rs` holds, in the vectors'
order. Run: `python3 -B tools/crosscheck/slip39_plain_pins.py` from the
repository root; it prints the pins as Rust, to compare by hand with the
pins the Rust test holds."""

from common import use_reference, vectors

use_reference("slip39")
from shamir_mnemonic import combine_mnemonics  # noqa: E402

import bip32_ref  # noqa: E402

for case in vectors("slip39", "vectors.json"):
    if not case[2]:
        continue
    secret = combine_mnemonics(case[1], b"")
    fp = bytes(bip32_ref.master(secret).fingerprint())
    print("    [" + ", ".join(f"0x{b:02x}" for b in fp) + "],  // " + case[0][:40])
