"""SLIP-39 against python-shamir-mnemonic, Trezor's reference.

All 45 of the reference's own vectors, then random backups: the same
random bytes are handed to both splits, so the shares must match word
for word; then recovery from random qualifying subsets, from subsets
that do not qualify, and from a set with one word changed; and the
passphrase encryption on its own. SLIP-39's limits (16 groups, all
needed, 16 members each) are split every run; the random tape is sized
to what a split draws, which reaches 8 KiB there.

One difference is known and kept out of the random corpus: Faraday
refuses a group threshold of 1 over several groups, which the reference
accepts (`KNOWN_DIFFERENCES` below checks that it still holds).
"""

import string

from common import Area, Mismatch, use_reference, vectors

use_reference("slip39")
import shamir_mnemonic as S  # noqa: E402
from shamir_mnemonic import cipher, shamir  # noqa: E402

PRINTABLE = string.ascii_letters + string.digits + string.punctuation + " "


class Tape:
    """The random bytes a split draws, in order."""

    def __init__(self, data):
        self.data = data
        self.at = 0

    def __call__(self, n):
        if self.at + n > len(self.data):
            raise AssertionError("the random tape ran out")
        out = self.data[self.at:self.at + n]
        self.at += n
        return out


def tape_length(secret_len, gt, groups):
    """The random bytes a split draws (SLIP-39's _split_secret): for a
    threshold t above 1, t - 2 random shares and the digest share's random
    part, at each level. The tape is cut to this plus a little, so a split
    that draws more or fewer shows as a different `random_used`."""
    def drawn(t):
        return 0 if t == 1 else (t - 2) * secret_len + secret_len - 4
    return drawn(gt) + sum(drawn(t) for t, _ in groups) + 64


def ref_split(secret, passphrase, identifier, extendable, e, gt, groups, random):
    tape = Tape(random)
    saved = shamir.RANDOM_BYTES
    shamir.RANDOM_BYTES = tape
    try:
        ems = S.EncryptedMasterSecret.from_master_secret(secret, passphrase, identifier, extendable, e)
        grouped = S.split_ems(gt, groups, ems)
    finally:
        shamir.RANDOM_BYTES = saved
    return {"mnemonics": [s.mnemonic() for g in grouped for s in g], "random_used": tape.at}


class Slip39(Area):
    name = "slip39"

    def split(self, secret, passphrase, identifier, extendable, e, gt, groups, random):
        return self.compare("slip39.split", {
            "secret": secret.hex(), "passphrase": passphrase, "identifier": identifier,
            "extendable": extendable, "exponent": e, "group_threshold": gt,
            "groups": [list(g) for g in groups], "random": random.hex()},
            lambda: ref_split(secret, passphrase.encode(), identifier, extendable, e, gt,
                              groups, random))

    def recover(self, mnemonics, passphrase, expected=None):
        return self.compare("slip39.recover", {"mnemonics": mnemonics, "passphrase": passphrase},
                            lambda: {"secret": S.combine_mnemonics(mnemonics, passphrase.encode()).hex()},
                            expected)

    def cipher(self, op, value, passphrase, identifier, extendable, e):
        fn = cipher.encrypt if op == "encrypt" else cipher.decrypt
        return self.compare(f"slip39.{op}", {
            "value": value.hex(), "passphrase": passphrase, "identifier": identifier,
            "extendable": extendable, "exponent": e},
            lambda: {"value": fn(value, passphrase.encode(), e, identifier, extendable).hex()})

    def published(self):
        for desc, mnemonics, secret, _xprv in vectors("slip39", "vectors.json"):
            want = ("ok", {"secret": secret}) if secret else ("err", None)
            try:
                self.recover(mnemonics, "TREZOR", want)
            except Mismatch as m:
                m.note = (m.note + "; " if m.note else "") + f"vector: {desc}"
                raise

    def known_differences(self):
        """Faraday refuses a 1-of-n group threshold over several groups;
        the reference makes the shares. Both halves must still hold."""
        args = {"secret": bytes(16).hex(), "passphrase": "", "identifier": 1, "extendable": True,
                "exponent": 0, "group_threshold": 1, "groups": [[1, 1], [2, 3]],
                "random": bytes(64).hex()}
        rust = self.rust.call("slip39.split", **args)
        ref = ref_split(bytes(16), b"", 1, True, 0, 1, [(1, 1), (2, 3)], bytes(64))
        if rust[0] != "err" or not ref["mnemonics"]:
            raise Mismatch(self.name, "slip39.split", args, rust, ("ok", ref),
                           "the known difference (1-of-n groups) no longer holds; update the area")

    def limits(self):
        """SLIP-39's limits, every run: 16 groups, all 16 needed, each 16
        of 16 members, over a 32-byte secret, and one past each refused."""
        secret = self.randbytes(32)
        full = [(16, 16)] * 16
        for gt, groups in [(16, full), (16, full + [(2, 2)]), (17, full),
                           (2, [(2, 17), (2, 2)]), (2, [(17, 16), (2, 2)])]:
            self.split(secret, "", 27924, True, 0, gt, groups,
                       self.randbytes(tape_length(32, gt, groups)))

    def random_groups(self):
        rng = self.rng
        count = rng.choice([1, 1, 2, 3, rng.randint(1, 16)])
        groups = []
        for _ in range(count):
            n = rng.choice([1, 2, 3, 5, rng.randint(1, 16)])
            t = 1 if n == 1 else rng.randint(2, n)
            groups.append((t, n))
        gt = 1 if count == 1 else rng.randint(2, count)
        return gt, groups

    def corpus_case(self):
        rng = self.rng
        secret = self.randbytes(rng.choice([16, 32, 2 * rng.randint(8, 16)]))
        passphrase = "".join(rng.choice(PRINTABLE) for _ in range(rng.choice([0, 0, 6, 20])))
        identifier, extendable, e = rng.randrange(1 << 15), rng.random() < 0.5, rng.choice([0, 0, 1])
        gt, groups = self.random_groups()
        ans = self.split(secret, passphrase, identifier, extendable, e, gt, groups,
                         self.randbytes(tape_length(len(secret), gt, groups)))
        shares = ans[1]["mnemonics"]
        by_group, at = [], 0
        for t, n in groups:
            by_group.append((t, shares[at:at + n]))
            at += n
        chosen = rng.sample(range(len(groups)), gt)
        subset = [m for gi in chosen for m in rng.sample(by_group[gi][1], by_group[gi][0])]
        rng.shuffle(subset)
        got = self.recover(subset, passphrase)
        if got[1]["secret"] != secret.hex():
            raise AssertionError("the shares do not recover the secret they split")
        # A set short of one share; a set with one word changed.
        if len(subset) > 1:
            self.recover(subset[:-1], passphrase)
        words = subset[0].split()
        words[rng.randrange(len(words))] = rng.choice(S.share.wordlist.WORDLIST)
        self.recover([" ".join(words)] + subset[1:], passphrase)
        if rng.random() < 0.3:
            op = rng.choice(["encrypt", "decrypt"])
            self.cipher(op, secret, passphrase, identifier, extendable, e)

    LOOPS = (("corpus", 1500),)

    def prelude(self):
        self.published()
        self.known_differences()
        self.limits()

    def case_corpus(self):
        self.corpus_case()
