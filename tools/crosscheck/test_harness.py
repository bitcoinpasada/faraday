#!/usr/bin/env python3
"""Tests of the cross-check harness itself, not of Faraday: the
shortcuts it takes must give the answers the references give.

    python3 -I -B tools/crosscheck/test_harness.py      (`just crosscheck-test`)

Standard library and the vendored references only; no Rust, no network.
"""

import io
import random
import sys
import time
import unittest
import pathlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import area_tx  # noqa: E402
from area_tx import M, CTransaction  # noqa: E402


def full_read(raw):
    """`area_tx.read` without its early refusal: Core's reading in full."""
    tx = CTransaction()
    f = io.BytesIO(raw)
    tx.deserialize(f)
    if f.read() or tx.serialize() != raw:
        raise area_tx.Refused(area_tx.NOT_EXACT)
    return tx


def answer(fn, raw):
    try:
        return ("ok", fn(raw).serialize())
    except Exception as e:  # noqa: BLE001 - every refusal is a refusal
        return ("err", f"{type(e).__name__}: {e}")


class Corpus(area_tx.Tx):
    """The tx area's transaction writer, without a Rust process."""

    def __init__(self, seed):
        self.rng = random.Random(seed)


def witness_from_locktime(tx, prefix):
    """A transaction without witnesses with the segwit marker and flag
    put after its version, so the witness is read from the locktime's
    bytes onward and past the end, as the tx area's corpus does
    (`raw[:4] + 00 01 + raw[4:]`); `prefix` is the locktime's bytes."""
    tx.nLockTime = int.from_bytes(prefix, "little")
    raw = tx.serialize_without_witness()
    return raw[:4] + b"\x00\x01" + raw[4:]


class TxRead(unittest.TestCase):
    def setUp(self):
        self.tx = Corpus("test-harness").random_tx()
        self.tx.wit.vtxinwit = []

    def test_a_stack_counted_in_millions_is_refused_at_once(self):
        for prefix in (b"\xfe\xff\xff\xff", b"\xff\xff\xff\xff", b"\xfe\x00\x00\x01"):
            raw = witness_from_locktime(self.tx, prefix)
            started = time.monotonic()
            self.assertEqual(answer(area_tx.read, raw), ("err", "Refused: " + area_tx.NOT_EXACT))
            self.assertLess(time.monotonic() - started, 1.0)

    def test_the_refusal_is_the_full_readings(self):
        # Counts just past the transaction's length and up to a few
        # thousand: the full reading finishes, and refuses alike.
        raw0 = witness_from_locktime(self.tx, b"\x00\x00\x00\x00")
        for count in (len(raw0) + 1, len(raw0) + 2, 300, 2999, 5000):
            raw = witness_from_locktime(self.tx, b"\xfd" + count.to_bytes(2, "little") + b"\x00")
            self.assertEqual(answer(area_tx.read, raw), answer(full_read, raw))
            self.assertEqual(answer(area_tx.read, raw), ("err", "Refused: " + area_tx.NOT_EXACT))

    def test_the_reading_is_unchanged_elsewhere(self):
        # The tx area's corpus transactions and their cut, lengthened and
        # marked forms, and counts read from a locktime under and over
        # the length, read alike with and without the early refusal.
        gen = Corpus("test-harness-corpus")
        rng = random.Random("test-harness-mutations")
        whole = M.deser_string_vector
        for _ in range(400):
            tx = gen.random_tx()
            raw = tx.serialize()
            forms = [raw, raw[:-rng.randint(1, min(10, len(raw)))], raw + b"\x00",
                     raw[:4] + b"\x00\x01" + raw[4:]]
            tx.wit.vtxinwit = []
            count = rng.randrange(0, 2 * len(raw) + 4)
            forms.append(witness_from_locktime(tx, b"\xfd" + count.to_bytes(2, "little") + b"\x00"))
            forms.append(witness_from_locktime(tx, bytes([rng.randrange(0xfd)]) + rng.randbytes(3)))
            for f in forms:
                self.assertEqual(answer(area_tx.read, f), answer(full_read, f), f.hex())
                self.assertIs(M.deser_string_vector, whole)


# The sign area's corpus case 806 of seed 4fde5465b6aeebd4007b9500d6cbd4f7 at
# scale 10 (test keys only): input 0 spends a taproot tree by a leaf whose
# key is the held master's; input 1 is a 3-of-3 `wsh(multi)` of three
# foreign keys, and that wallet is the one registered.
PSBT_806 = (
    "70736274ff0100a60200000002b5017bbab78b1076507ebdd970183409797b22fe8663e9979f5b34c200a32f"
    "cb000000000012010000483c4d9d7ed63e58feb34d8cab19e6b897e5e7ea0bfcba0dcc15d5e7bd0811750000"
    "000000ffffffff02128f100100000000225120eebed9d55988cc0366b481a7f8936a49a72821c7f9753143d1"
    "01b164713fa5947757a501000000001600149ed42e0ef69e40e07f81c3f3bf3eb1878a8c7cdf000000000001"
    "005e020000000146b57f3e3e3321877281fde1b56551e3d644b4c65f35a3f4fc4e6f962c1f09850200000000"
    "ffffffff0164ec8d01000000002251208f0777a555add2697b5eb826dad03c1c942e8ca80486988fcca5324e"
    "2716e4a20000000001012b64ec8d01000000002251208f0777a555add2697b5eb826dad03c1c942e8ca80486"
    "988fcca5324e2716e4a201172050929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803a"
    "c0011820fb2bff21e244df19174964d26003ab8621b78fea29c763d2f6063ef1a8d7c33a2215c050929b74c1"
    "a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac02720fc380b9443690a26d1f11903ef8b82"
    "6feae0afdc512328b067a5a1ed91c9763cad02f800b2c02116fc380b9443690a26d1f11903ef8b826feae0af"
    "dc512328b067a5a1ed91c9763c3901fb2bff21e244df19174964d26003ab8621b78fea29c763d2f6063ef1a8"
    "d7c33ae48f8ccd570000800100008001000080000000000a1007000001005e0200000001c6b91dfeb241843a"
    "3f4b3058a10cc93e927b6835169727a70266d0094ff5608b0100000000ffffffff01b0312801000000002200"
    "20af315513c8ed303f9b9e399164ae45ebc4e87649f85396623ce1d3308c2fdd1f0000000001012bb0312801"
    "00000000220020af315513c8ed303f9b9e399164ae45ebc4e87649f85396623ce1d3308c2fdd1f0105695321"
    "03f35e098c8adc63d01e231b35e50aab262172e0831b9bea812cbbc6b63ad7986b2103d20e79d9121f783c8d"
    "6deb9a64c76bc1325c82c3fbe5f1e10527f0b03b517b8f210282973c20f36878021940ccb5673275cce13ba3"
    "50f7c5666da9d1c5f8972b1fb553ae220603f35e098c8adc63d01e231b35e50aab262172e0831b9bea812cbb"
    "c6b63ad7986b1c9528394d3000008001000080ffa300800200008001000000f9f00b00220603d20e79d9121f"
    "783c8d6deb9a64c76bc1325c82c3fbe5f1e10527f0b03b517b8f1c86fdfbee300000800100008031ff0d8002"
    "00008001000000f9f00b0022060282973c20f36878021940ccb5673275cce13ba350f7c5666da9d1c5f8972b"
    "1fb51c1ec83cd73000008001000080fde90f800200008001000000f9f00b0001030401000000220203f35e09"
    "8c8adc63d01e231b35e50aab262172e0831b9bea812cbbc6b63ad7986b473044022076d967867a1966dbdb2d"
    "04be72cade33ebe937ddda3165bde45de1f9ff3483c3022073d61576d31c2b17290ca4ddb6896af1c54b86b2"
    "b029014ac6c641d43af902b401000000"
)
SEED_806 = '7e86f8a940e38ca66997f25243f4d06a3f1c3c0ceed9e375eafe80c06d9e9381ce263f5304c2185b4659a65b6bfd48b1cfeeff89394ebf7432ae8202ea6084fc'
WALLET_806 = (
    'wsh(multi(3,[9528394d/48h/1h/41983h/2h]tpubDF8beRG2wC9rwThwDHQjQnjb9LYN7nTjiXi4SaPWaBp2n'
    'e9j4iLSxHtZ37Nx9kZxG9ekjhmGhxttFCwheRZKHwumS3rysPCWGBek3fXBjHL/<0;1>/*,[86fdfbee/48h/1h/'
    '917297h/2h]tpubDDv9beKVA9rikKJsrbsRXHiRJxvCvtvwG5R8bRaJQm6VibiBFAMJPgiz3dNSuY8FpHi1vktqg'
    'G6L8odRGKFvh5LeaVbqU4v26hkZgYGPeb3/<0;1>/*,[1ec83cd7/48h/1h/1042941h/2h]tpubDFLDZEG1gxUt'
    'R5EaEdoGRHRznWHKUYFCWywtsFEqHbGt4qNPhepCAZ7aJDNfGQJUKVH8fuJwuTzZ6xtUeD6t1wXqKeWueAXSexY2'
    'vkJiect/<0;1>/*))'
)


class SignParticipation(unittest.TestCase):
    """A taproot tree or a miniscript takes part through a registered
    wallet whose script it is (F135, F136), as Faraday's inspection
    counts it; another wallet registered beside it does not."""

    def test_a_wallet_beside_a_tree_it_does_not_give_takes_no_part(self):
        import area_sign as A
        import btc
        master = btc.master(bytes.fromhex(SEED_806), test=True)
        masters = {master.fingerprint(): master}
        raw = bytes.fromhex("".join(PSBT_806))
        wallet = "".join(WALLET_806)
        # The case made input 0's one leaf, a key under a timelock, from
        # this miniscript (under BIP 341's unspendable internal key).
        A.SCRIPTS.add("and_v(v:pk(fc380b9443690a26d1f11903ef8b826feae0afdc512328b067a5a1ed91c9763c),"
                      "older(248))", True)
        _, ins, _, tx = A.parse(raw)
        inputs = [A.Input(m, tx, i) for i, m in enumerate(ins)]
        self.assertEqual([i.kind for i in inputs], ["tr-script", "p2wsh-multi"])
        self.assertFalse(A.several_keys(inputs[0]))
        self.assertFalse(A.wallet_gives(wallet, inputs[0], masters))
        self.assertFalse(any(A.takes_part(i, masters, [wallet]) for i in inputs))
        for wallets in ([wallet], []):
            with self.assertRaises(A.Known) as refused:
                A.ref_sign(raw, masters, "low_r", wallets)
            self.assertEqual(refused.exception.kind, "NoParticipatingKey")


if __name__ == "__main__":
    unittest.main()
