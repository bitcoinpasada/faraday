"""What the Compare button of "My seed makes my wallet" is tested with.

Runs outside code only: BIP-39's seed step with the standard library
(PBKDF2-HMAC-SHA512), BIP-32 by btclib (`tools/crosscheck/bip32_ref.py`)
and the bech32 address by Bitcoin Core's functional-test framework
(`tools/reference/core/test_framework/segwit_addr.py`). It writes
`compare.json`, beside this file:

- `bip84_change_7`: the address at m/84'/0'/0'/1/7 of the test phrase
  ("abandon ... about", no passphrase), a chain-1 index past 0;
- `cosigners`: three BIP-39 test phrases (BIP-39's vectors, no passphrase),
  each with its fingerprint and its BIP-48 cosigner key at m/48'/0'/0'/2';
- `wallet`: the 2-of-3 `wsh(sortedmulti())` descriptor of the three, in
  that order, which the test phrase of the second is key 2 of.

Usage: python3 -I -B tools/vectors/seed-check/compare_gen.py
"""

import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "tools" / "crosscheck"))

import bip32_ref  # noqa: E402
from bip32_ref import HARDENED  # noqa: E402
from common import use_reference  # noqa: E402

use_reference("core")
from test_framework import segwit_addr  # noqa: E402

ABOUT = ("abandon abandon abandon abandon abandon abandon abandon abandon "
         "abandon abandon abandon about")
LEGAL = "legal winner thank year wave sausage worth useful legal winner thank yellow"
LETTER = "letter advice cage absurd amount doctor acoustic avoid letter advice cage above"


def master(phrase):
    seed = hashlib.pbkdf2_hmac("sha512", phrase.encode(), b"mnemonic", 2048)
    return bip32_ref.master(seed)


def hash160(data):
    return hashlib.new("ripemd160", hashlib.sha256(data).digest()).digest()


def cosigner(phrase):
    m = master(phrase)
    path = [48 + HARDENED, HARDENED, HARDENED, 2 + HARDENED]
    xpub = m.derive(path).neuter().serialize()
    return {"phrase": phrase, "fingerprint": m.fingerprint().hex(), "xpub": xpub}


def main():
    m = master(ABOUT)
    key = m.derive([84 + HARDENED, HARDENED, HARDENED, 1, 7]).neuter()
    address = segwit_addr.encode_segwit_address("bc", 0, hash160(key.pub))
    keys = [cosigner(p) for p in (LEGAL, ABOUT, LETTER)]
    parts = ",".join(
        f"[{k['fingerprint']}/48h/0h/0h/2h]{k['xpub']}/<0;1>/*" for k in keys)
    out = {
        "phrase": ABOUT,
        "bip84_change_7": address,
        "cosigners": keys,
        "wallet": f"wsh(sortedmulti(2,{parts}))",
    }
    (HERE / "compare.json").write_text(json.dumps(out, indent=1) + "\n")


main()
