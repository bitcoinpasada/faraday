#!/usr/bin/env python3
"""Tests of the cross-check harness itself, not of Faraday: the
shortcuts it takes must give the answers the references give, and the
number of workers, the Python they run under and the progress it writes
must not change a case, an answer or the report.

    python3 -I -B tools/crosscheck/test_harness.py      (`just crosscheck-test`)

Standard library and the vendored references only; no Rust, no network:
the pool's tests run areas of their own against a stand-in command
written in Python. `crosscheck-identity.sh` runs the real areas with the
Rust command under one and several workers and compares their reports.
"""

import io
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

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
    """A taproot tree or a miniscript takes part, and is signed, only
    through a registered wallet whose script it is (F135, F136, F141), as
    Faraday's inspection counts it; another wallet registered beside it
    does not. These are the area's own rules (`area_sign.policy`), which
    need no Bitcoin Core."""

    def test_a_wallet_beside_a_tree_it_does_not_give_takes_no_part(self):
        import area_sign as A
        import bip32_ref
        master = bip32_ref.master(bytes.fromhex(SEED_806), test=True)
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
                A.policy(raw, masters, wallets)
            self.assertEqual(refused.exception.kind, "NoParticipatingKey")

    def leaf_spend(self, key_path):
        """A held master's `pk(K)` leaf under BIP 341's unspendable
        internal key, `K` at m/86'/1'/0'/0/1, spent alone or after a
        BIP-86 key-path input of the same master at m/86'/1'/0'/0/0: the
        master, the PSBT, and the leaf's wallet."""
        import area_sign as A
        import bip32_ref
        H = bip32_ref.HARDENED
        master = bip32_ref.master(bytes(range(32)), test=True)
        fp = master.fingerprint()
        account = [86 + H, 1 + H, 0 + H]
        ins, tx = [], A.CTransaction()
        tx.version = 2
        if key_path:
            x = master.derive(account + [0, 0]).pub[1:]
            ins.append([(b"\x01", A.CTxOut(100_000, A.p2tr(x, None)[0]).serialize()),
                        (b"\x17", x),
                        (b"\x16" + x, A.ser_compact(0) + A.ser_origin(fp, account + [0, 0]))])
        x = master.derive(account + [0, 1]).pub[1:]
        nums = bytes.fromhex(A.NUMS_HEX)
        script = A.SCRIPTS.add(f"pk({x.hex()})", True)
        info = A.taproot_construct_safe(nums, [("l0", script)])
        leaf = info.leaves["l0"]
        control = bytes([A.TAPSCRIPT | info.negflag]) + nums + leaf.merklebranch
        ins.append([(b"\x01", A.CTxOut(100_000, bytes(info.scriptPubKey)).serialize()),
                    (b"\x17", nums), (b"\x18", info.merkle_root),
                    (b"\x15" + control, script + bytes([A.TAPSCRIPT])),
                    (b"\x16" + x, A.ser_compact(1) + leaf.leaf_hash
                     + A.ser_origin(fp, account + [0, 1]))])
        for j in range(len(ins)):
            tx.vin.append(A.CTxIn(A.COutPoint(j + 1, 0), b"", 0xFFFFFFFF))
        tx.vout = [A.CTxOut(100_000 * len(ins) - 2_000, A.p2wpkh(bytes(20)))]
        raw = A.write([(b"\x00", tx.serialize_without_witness())], ins, [[]])
        xpub = master.derive(account).neuter().serialize()
        wallet = f"tr({A.NUMS_HEX},pk([{fp.hex()}/86h/1h/0h]{xpub}/<0;1>/*))"
        return {fp: master}, raw, wallet

    def test_a_leaf_without_its_wallet_is_left_unsigned_beside_a_signed_input(self):
        import area_sign as A
        masters, raw, wallet = self.leaf_spend(True)
        self.assertEqual(A.policy(raw, masters, [])[2], {1})
        self.assertEqual(A.policy(raw, masters, [wallet])[2], set())

    def test_a_leaf_without_its_wallet_alone_takes_no_part(self):
        import area_sign as A
        masters, raw, wallet = self.leaf_spend(False)
        with self.assertRaises(A.Known) as refused:
            A.policy(raw, masters, [])
        self.assertEqual(refused.exception.kind, "NoParticipatingKey")
        self.assertEqual(A.policy(raw, masters, [wallet])[2], set())


class Bip39Words(unittest.TestCase):
    """`area_bip39` hands python-mnemonic its wordlists as `Words`: the
    same words, and `index` and `in` answer as the plain list's do."""

    def test_words_answer_as_the_list_does(self):
        import area_bip39
        for lang in range(len(area_bip39.LANGUAGES)):
            words = area_bip39.mnemonic(lang).wordlist
            plain = list(words)
            self.assertIsInstance(words, area_bip39.Words)
            self.assertEqual(len(plain), 2048)
            for w in plain + ["", "zzzz", plain[0] + "x", plain[-1].upper()]:
                self.assertEqual(w in words, w in plain)
                if w in plain:
                    self.assertEqual(words.index(w), plain.index(w))
                else:
                    with self.assertRaises(ValueError):
                        words.index(w)


class MusigKept(unittest.TestCase):
    """`area_musig` keeps the reference's point multiplications: a kept
    answer is the one the reference computes again."""

    def test_kept_answers_are_the_references(self):
        import area_musig
        R = area_musig.R
        rng = random.Random("test-harness-musig")
        points = [None, R.G] + [area_musig._point_mul(R.G, rng.randrange(1, R.n)) for _ in range(3)]
        for _ in range(12):
            P, k = rng.choice(points), rng.choice([0, 1, R.n - 1, R.n, rng.randrange(2**256)])
            first = R.point_mul(P, k)
            self.assertEqual(first, area_musig._point_mul(P, k))
            self.assertEqual(R.point_mul(P, k), first)
        self.assertIs(R.point_mul, area_musig.point_mul)


class TaprootKept(unittest.TestCase):
    """`area_taproot` keeps the reference's key tweaks: a kept answer is
    the one the reference computes again, refusals included."""

    def test_kept_answers_are_the_references(self):
        import area_taproot as T
        rng = random.Random("test-harness-taproot")
        keys = [T.K.compute_xonly_pubkey(rng.randbytes(32))[0] for _ in range(3)]
        keys.append(bytes(32))  # no point
        tweaks = [rng.randbytes(32) for _ in range(3)] + [b"\xff" * 32]  # past the order
        for _ in range(3):
            for k in keys:
                for t in tweaks:
                    self.assertEqual(T.tweak_add_pubkey(k, t), T._tweak_add_pubkey(k, t))
        self.assertIs(T.S.tweak_add_pubkey, T.tweak_add_pubkey)
        with self.assertRaises(AssertionError):
            T.tweak_add_pubkey(b"short", tweaks[0])


STAND_IN = r'''#!/usr/bin/env python3
"""The stand-in for the Rust command: sha256 and length, one request a
line; told to sabotage an operation, it answers that one wrongly."""
import hashlib, json, os, sys
bad = os.environ.get("FARADAY_CROSSCHECK_SABOTAGE")
for line in sys.stdin:
    req = json.loads(line)
    data = bytes.fromhex(req.get("data", ""))
    if req["op"] == "fake.hash":
        out = {"ok": {"h": hashlib.sha256(data).hexdigest()}}
    elif req["op"] == "fake.len":
        out = {"ok": {"n": len(data)}} if data[:1] != b"\xff" else {"err": "refused"}
    else:
        out = {"fatal": "unknown op"}
    if bad == req["op"] and "ok" in out:
        out = {"ok": {"wrong": True}}
    out["id"] = req["id"]
    print(json.dumps(out), flush=True)
'''

AREA_MODULE = r'''
"""Areas of the harness's own tests: one that agrees, one whose answers
lead to its next draws, one that disagrees at a case of its corpus."""
import hashlib
import pathlib
import sys
sys.path.insert(0, str(pathlib.Path(__file__).parent / "a"))
import clash
from common import Area

assert clash.WHO == "a", "another area's clash"


def h(data):
    return {"h": hashlib.sha256(data).hexdigest()}


def n(data):
    if data[:1] == b"\xff":
        raise ValueError("refused")
    return {"n": len(data)}


class Alpha(Area):
    name = "alpha"
    LOOPS = (("corpus", 300), ("chain", 40))

    def prelude(self):
        for d in (b"", b"abc", bytes(64)):
            self.compare("fake.hash", {"data": d.hex()}, lambda: h(d))
        self.compare("fake.len", {"data": "ff"}, lambda: n(b"\xff"))

    def case_corpus(self):
        d = self.randbytes(self.rng.randint(0, 40))
        self.compare("fake.hash", {"data": d.hex()}, lambda: h(d))
        if self.rng.random() < 0.3:
            self.compare("fake.len", {"data": d.hex()}, lambda: n(d))
            self.known += d[:1] == b"\xff"

    def case_chain(self):
        d = self.randbytes(8)
        for _ in range(self.rng.randint(1, 5)):
            got = self.need(self.compare("fake.hash", {"data": d.hex()}, lambda: h(d)), "hash")
            d = bytes.fromhex(got["h"])[: 1 + self.rng.randrange(32)]

    def coda(self):
        self.compare("fake.len", {"data": self.randbytes(5).hex()}, lambda: {"n": 5})


class Beta(Area):
    name = "beta"
    LOOPS = (("corpus", 400),)

    def prelude(self):
        self.compare("fake.len", {"data": "00"}, lambda: n(b"\x00"))

    def case_corpus(self):
        d = self.randbytes(self.rng.randint(1, 20))
        wrong = self.rng.randrange(150) == 0
        self.compare("fake.len", {"data": d.hex()},
                     lambda: {"n": len(d) + 1} if wrong else n(d))
'''

GAMMA_MODULE = r'''
"""An area whose reference is a module named like another area's, as
two areas' references are (`reference`, `bip32`): it must run in a
process of its own."""
import pathlib
import sys
sys.path.insert(0, str(pathlib.Path(__file__).parent / "g"))
import clash
from common import Area

assert clash.WHO == "g", "another area's clash"


class Gamma(Area):
    name = "gamma"
    LOOPS = (("corpus", 30),)

    def prelude(self):
        self.compare("fake.len", {"data": "00"}, lambda: {"n": 1})

    def case_corpus(self):
        d = self.randbytes(3)
        self.compare("fake.len", {"data": d.hex()}, lambda: {"n": len(d)})
'''

AREAS_JSON = {
    "alpha": ["fake_areas", "Alpha", "fake.hash", 2],
    "beta": ["fake_areas", "Beta", "fake.len", 1],
    "gamma": ["fake_gamma", "Gamma", "fake.len", 1],
}

TIMES = [(re.compile(r"\(\d+(\.\d+)? s\)"), "(T s)"), (re.compile(r"after \d+(\.\d+)? s"), "after T s")]


def untimed(text):
    for pattern, repl in TIMES:
        text = pattern.sub(repl, text)
    return text


class Pool(unittest.TestCase):
    """The pool against areas of its own: the report reads the same for
    any number of workers and under PyPy, and the progress follows it."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = pathlib.Path(tempfile.mkdtemp(prefix="crosscheck-test-"))
        (cls.tmp / "fake_areas.py").write_text(AREA_MODULE)
        (cls.tmp / "areas.json").write_text(json.dumps(AREAS_JSON))
        cls.rust = cls.tmp / "stand-in"
        cls.rust.write_text(STAND_IN)
        cls.rust.chmod(0o755)
        (cls.tmp / "fake_gamma.py").write_text(GAMMA_MODULE)
        for who in ("a", "g"):
            (cls.tmp / who).mkdir()
            (cls.tmp / who / "clash.py").write_text(f"WHO = {who!r}\n")

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp)

    def run_crosscheck(self, *args, progress=None, python=sys.executable):
        cmd = [python, "-I", "-B", str(HERE / "crosscheck.py"), "--rust", str(self.rust),
               "--test-areas", str(self.tmp), "--allow-network", "--seed", "test-seed",
               "--progress", str(progress or ""), *args]
        return subprocess.run(cmd, capture_output=True, text=True, timeout=600)

    def test_the_report_is_the_same_for_any_number_of_workers(self):
        runs = {j: self.run_crosscheck("--jobs", str(j), "--python", "cpython") for j in (1, 4, 7)}
        one = untimed(runs[1].stdout)
        self.assertIn("self-test alpha: caught a wrong fake.hash", one)
        self.assertIn("self-test beta: caught a wrong fake.len", one)
        self.assertIn("alpha: ", one)
        self.assertIn("BETA" if False else "beta: MISMATCH", one)
        self.assertRegex(one, r"at case corpus:\d+ ")
        for j, r in runs.items():
            self.assertEqual(r.returncode, 1, r.stderr)
            self.assertEqual(untimed(r.stdout), one, f"--jobs {j}")

    def test_a_case_alone_gives_its_disagreement(self):
        r = self.run_crosscheck("--jobs", "3", "--python", "cpython")
        case = re.search(r"at case (corpus:\d+) ", r.stdout).group(1)
        report = re.search(r"DISAGREEMENT in beta.*?reference: [^\n]*", r.stdout, re.S).group(0)
        alone = self.run_crosscheck("--area", "beta", "--case", case, "--no-self-test",
                                    "--python", "cpython")
        self.assertEqual(alone.returncode, 1)
        self.assertIn(report, alone.stdout)

    def test_the_progress_follows_the_run(self):
        folder = self.tmp / "progress"
        r = self.run_crosscheck("--jobs", "2", "--python", "cpython", "--log-every", "0.05",
                                progress=folder)
        log = (folder / "progress.log").read_text()
        # Between events, a line on the whole run at the interval asked.
        self.assertRegex(log, r"run: \d+ of 770 cases, [\d.]+ a second, \d of 2 workers busy")
        for line in ("crosscheck: seed test-seed", "self-test: 3 areas",
                     "self-test alpha: caught a wrong fake.hash", "self-test: every area caught",
                     "run: 3 areas", "gamma: done, 30 of 30 cases", "alpha: started, 340 cases", "beta: started, 400 cases",
                     "alpha: done, 340 of 340 cases", "beta: DISAGREEMENT at case corpus:",
                     "beta: MISMATCH at case corpus:", "crosscheck: FAILED"):
            self.assertIn(line, log)
        p = json.loads((folder / "progress.json").read_text())
        self.assertEqual(p["phase"], "failed")
        self.assertEqual(p["areas"]["alpha"]["state"], "done")
        self.assertEqual(p["areas"]["alpha"]["cases_done"], 340)
        self.assertEqual(p["areas"]["beta"]["state"], "failed")
        self.assertEqual((folder / "report.txt").read_text(), r.stdout)
        status = subprocess.run([sys.executable, "-I", str(HERE / "status.py"), "--dir",
                                 str(folder)], capture_output=True, text=True)
        self.assertIn("crosscheck: failed; seed test-seed", status.stdout)
        self.assertIn("DISAGREEMENT", status.stdout)
        # Writing the progress changes nothing in the report.
        quiet = self.run_crosscheck("--jobs", "2", "--python", "cpython")
        self.assertEqual(untimed(quiet.stdout), untimed(r.stdout))

    def test_a_fresh_run_writes_the_release_record(self):
        folder = self.tmp / "record" / "crosscheck"
        # A fresh disagreement is kept beside the harness; this test's is not.
        self.addCleanup(lambda: (HERE / "found" / "beta-test-seed.json").unlink(missing_ok=True))
        r = self.run_crosscheck("--fresh", "--scale", "1", "--jobs", "2", "--python", "cpython",
                                progress=folder)
        record = (folder.parent / "crosscheck-record.txt").read_text()
        self.assertEqual(r.returncode, 1)
        for line in ("commit ", "seed test-seed", "scale 1.0", " UTC", "alpha: pass, 340 cases",
                     "beta: mismatch", "tools/reference/bbqr  commit ",
                     "result: first disagreement in beta at case corpus:"):
            self.assertIn(line, record)
        self.assertRegex(record, r"tools/reference/core  commit .* sha256 [0-9a-f]{64}")
        # No path of this machine, and the same record twice but for the date.
        self.assertNotIn(str(HERE.parents[1]), record)
        self.assertNotIn(str(self.tmp), record)
        self.run_crosscheck("--fresh", "--scale", "1", "--jobs", "2", "--python", "cpython",
                                    progress=folder)
        dated = re.compile(r"^date .*$", re.M)
        self.assertEqual(dated.sub("", (folder.parent / "crosscheck-record.txt").read_text())
                         , dated.sub("", record))
        # A run of one area is not a release's.
        (folder.parent / "crosscheck-record.txt").unlink()
        self.run_crosscheck("--fresh", "--area", "alpha", "--jobs", "2", "--python", "cpython",
                            progress=folder)
        self.assertFalse((folder.parent / "crosscheck-record.txt").exists())

    @unittest.skipUnless(shutil.which("pypy3"), "no pypy3 on the PATH")
    def test_pypy_gives_the_same_report(self):
        cpython = self.run_crosscheck("--jobs", "3", "--python", "cpython")
        pypy = self.run_crosscheck("--jobs", "3", "--python", "pypy")
        self.assertEqual(untimed(pypy.stdout), untimed(cpython.stdout))


DELTA_MODULE = r'''
"""An area that asks Bitcoin Core."""
from common import Area


class Delta(Area):
    name = "delta"
    CORE = True
    LOOPS = (("corpus", 3),)

    def case_corpus(self):
        self.compare("fake.len", {"data": "00"}, lambda: {"n": 1})
'''


class CoreRequired(unittest.TestCase):
    """An area that asks Bitcoin Core does not run without the pinned
    release: the run refuses before the self-test and says what is
    missing and which area needs it."""

    def test_a_run_without_core_refuses_and_names_it(self):
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="crosscheck-test-core-"))
        try:
            (tmp / "fake_delta.py").write_text(DELTA_MODULE)
            (tmp / "areas.json").write_text(json.dumps(
                {"delta": ["fake_delta", "Delta", "fake.len", 1]}))
            rust = tmp / "stand-in"
            rust.write_text(STAND_IN)
            rust.chmod(0o755)
            empty = tmp / "no-core"
            empty.mkdir()
            env = dict(os.environ, FARADAY_BITCOIN_CORE=str(empty))
            r = subprocess.run([sys.executable, "-I", "-B", str(HERE / "crosscheck.py"),
                                "--rust", str(rust), "--test-areas", str(tmp), "--allow-network",
                                "--progress", ""], capture_output=True, text=True, timeout=120,
                               env=env)
            self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
            self.assertIn("Bitcoin Core 31.1 is missing", r.stdout)
            self.assertIn("fetch_core.py", r.stdout)
            self.assertIn("(needed by delta)", r.stdout)
            self.assertNotIn("self-test", r.stdout)
        finally:
            shutil.rmtree(tmp)


if __name__ == "__main__":
    unittest.main()
