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


if __name__ == "__main__":
    unittest.main()
