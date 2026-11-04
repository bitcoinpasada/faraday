"""SLIP-39 against python-shamir-mnemonic, Trezor's reference, and
bip39_slip39 (the btclib developers', `bip39-slip39/`), which answers each
SLIP-39 split, recovery and cipher call beside it.

All 45 of the reference's own vectors, then random backups: the same
random bytes are handed to both splits, so the shares must match word
for word; then recovery from random qualifying subsets, from subsets
that do not qualify, and from a set with one word changed; and the
passphrase encryption on its own. A second loop (`sets`) reads a set's
encrypted master secret, splits it again and makes a member at an
index, as rotating a set, adding a share and replacing a lost one do.
SLIP-39's limits (16 groups, all
needed, 16 members each) are split every run; the random tape is sized
to what a split draws, which reaches 8 KiB there.

A third loop (`convert`) runs the fixed 1 of 1 against both libraries and
the codex32 conversions: Faraday's codex32 read by python-codex32, and a
SLIP-39 set with a passphrase recovered by python-shamir-mnemonic, written
as codex32 by python-codex32 and read by Faraday; fingerprints are btclib's.
The lengths the 1 of 1 refuses are Faraday's own rule, which no second judge
covers.

One difference is known and kept out of the random corpus: Faraday
refuses a group threshold of 1 over several groups, which the reference
accepts (`KNOWN_DIFFERENCES` below checks that it still holds).
"""

import string

from common import Area, Mismatch, use_reference, use_wheels, vectors

use_reference("slip39")
import shamir_mnemonic as S  # noqa: E402
from shamir_mnemonic import cipher, shamir  # noqa: E402

use_reference("codex32", "shims")
use_reference("codex32")
from codex32 import Codex32String  # noqa: E402
from codex32.bech32 import CHARSET  # noqa: E402
from codex32.bip93 import encode as codex32_encode  # noqa: E402

import bip32_ref  # noqa: E402

use_wheels("bip39-slip39", ["bip39_slip39-2026.10.24-py3-none-any.whl"])
from bip39_slip39 import slip39 as B  # noqa: E402

PRINTABLE = string.ascii_letters + string.digits + string.punctuation + " "


def fingerprint(seed):
    """The master fingerprint of a seed, btclib's (`bip32_ref.py`), never
    Faraday's own."""
    return bytes(bip32_ref.master(seed).fingerprint())


def one_of_one_identifier(seed):
    """SLIP-39's 15-bit identifier of a 1 of 1: the first 15 bits of the
    master fingerprint (§4.4 of `docs/SEED-CONVERSION.md`)."""
    return int.from_bytes(fingerprint(seed), "big") >> 17


def codex32_identifier(seed):
    """codex32's identifier of a seed: the first 20 bits of its master
    fingerprint as four bech32 characters (`docs/CODEX32.md` §6.4.5)."""
    bits = int.from_bytes(fingerprint(seed), "big")
    return "".join(CHARSET[(bits >> (27 - 5 * i)) & 31] for i in range(4))


def ref_one_of_one(seed, judge):
    """The 1 of 1 of §4.4 through a SLIP-39 library: 16 or 32 bytes (the
    lengths hardware wallets read, Faraday's rule), identifier from the
    fingerprint, extendable, iteration exponent 1, no passphrase."""
    if len(seed) not in (16, 32):
        raise ValueError("a 1 of 1 is made of 16 or 32 bytes")
    args = (seed, b"" if judge is ref_split else "", one_of_one_identifier(seed), True, 1, 1,
            [(1, 1)], b"")
    made = judge(*args)
    if made["random_used"]:
        raise AssertionError("a 1 of 1 drew random bytes")
    return {"mnemonic": made["mnemonics"][0]}


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


def btl_split(secret, passphrase, identifier, extendable, e, gt, groups, random):
    """bip39_slip39's split. It draws the identifier's two bytes first, from
    the entropy source, where the reference takes the identifier as given:
    the tape is the identifier and then the same random bytes."""
    tape = Tape(identifier.to_bytes(2, "big") + random)
    shares = B.mnemonics_from_master_secret(
        secret, [tuple(g) for g in groups], gt, passphrase, e, extendable, tape)
    return {"mnemonics": [m for g in shares for m in g], "random_used": tape.at - 2}


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


def ems_answer(identifier, extendable, e, ciphertext):
    return {"ciphertext": ciphertext.hex(), "identifier": identifier,
            "extendable": extendable, "exponent": e}


def ref_recover_ems(mnemonics):
    ems = shamir.recover_ems(shamir.decode_mnemonics(mnemonics))
    return ems_answer(ems.identifier, ems.extendable, ems.iteration_exponent, ems.ciphertext)


def btl_recover_ems(mnemonics):
    ems = B.encrypted_master_secret_from_mnemonics(mnemonics)
    return ems_answer(ems.identifier, ems.extendable, ems.iteration_exponent, ems.value)


def ref_split_ems(ciphertext, identifier, extendable, e, gt, groups, random):
    tape = Tape(random)
    saved = shamir.RANDOM_BYTES
    shamir.RANDOM_BYTES = tape
    try:
        ems = shamir.EncryptedMasterSecret(identifier, extendable, e, ciphertext)
        grouped = S.split_ems(gt, groups, ems)
    finally:
        shamir.RANDOM_BYTES = saved
    return {"mnemonics": [s.mnemonic() for g in grouped for s in g], "random_used": tape.at}


def btl_split_ems(ciphertext, identifier, extendable, e, gt, groups, random):
    tape = Tape(random)
    ems = B.EncryptedMasterSecret(identifier=identifier, extendable=extendable,
                                  iteration_exponent=e, value=ciphertext)
    grouped = B.mnemonics_from_encrypted_master_secret(
        ems, [tuple(g) for g in groups], gt, tape)
    return {"mnemonics": [m for g in grouped for m in g], "random_used": tape.at}


def derive_refused(group_indices, thresholds, indices, index):
    """Faraday's own refusals of a new member, before the arithmetic,
    which neither reference has a function for: shares of more than one
    group, an index outside 0 to 15, a group of threshold 1 (SLIP-39
    allows it one share), a count other than the threshold, a repeated
    index, and an index among the shares given (that share exists)."""
    if len(set(group_indices)) != 1:
        return "shares of more than one group"
    if not 0 <= index < 16:
        return "member index outside 0 to 15"
    if len(set(thresholds)) != 1:
        return "member thresholds differ"
    t = thresholds[0]
    if len(set(indices)) != len(indices) or len(indices) != t:
        return "a repeated index or a count other than the threshold"
    if t == 1:
        return "a group of threshold 1"
    if index in indices:
        return "the index is among the shares given"
    return None


def ref_derive(mnemonics, index):
    """A new member through the reference: its own decoding and set
    checks, the digest checked by `_recover_secret`, and `_interpolate`
    at the index."""
    groups = shamir.decode_mnemonics(mnemonics)
    shares = [s for g in groups.values() for s in g]
    why = derive_refused([s.group_index for s in shares], [s.member_threshold for s in shares],
                         [s.index for s in shares], index)
    if why or len(shares) != len(mnemonics):
        raise ValueError(why or "a share given twice")
    first, t = shares[0], shares[0].member_threshold
    raw = [shamir.RawShare(s.index, s.value) for s in shares]
    shamir._recover_secret(t, raw)
    value = shamir._interpolate(raw, index)
    new = S.share.Share(first.identifier, first.extendable, first.iteration_exponent,
                        first.group_index, first.group_threshold, first.group_count,
                        index, t, value)
    return {"mnemonic": new.mnemonic()}


def btl_derive(mnemonics, index):
    shares = [B.share_from_mnemonic(m) for m in mnemonics]
    B._common_field(shares)
    why = derive_refused([s.group_index for s in shares], [s.member_threshold for s in shares],
                         [s.member_index for s in shares], index)
    if why:
        raise ValueError(why)
    first, t = shares[0], shares[0].member_threshold
    points = [(s.member_index, s.value) for s in shares]
    B._recover_secret(t, points)
    new = B.Share(identifier=first.identifier, extendable=first.extendable,
                  iteration_exponent=first.iteration_exponent, group_index=first.group_index,
                  group_threshold=first.group_threshold, group_count=first.group_count,
                  member_index=index, member_threshold=t, value=B._interpolate(points, index))
    return {"mnemonic": B.mnemonic_from_share(new)}


class Slip39(Area):
    name = "slip39"
    JUDGE = "bip39_slip39"
    faraday = (
        '`faraday_bip::slip39`'
    )
    reference = (
        'python-shamir-mnemonic (`slip39/`), its `recover_ems`, `split_ems` and '
        '`_interpolate` for a set\'s EMS, a new split of it and a member at an index; '
        'bip39_slip39 (`bip39-slip39/`) answering each SLIP-39 operation beside it (not the '
        'two codex32 operations); python-codex32 (`codex32/`) for the codex32 side of the '
        'conversions and btclib (`bip32_ref.py`) for the fingerprint; the refusals '
        'of a member at an index before the arithmetic are ours (`derive_refused`), and so '
        'are the lengths the fixed 1 of 1 refuses, which no second judge covers'
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        'Faraday, and of bip39_slip39 too; all 45 of its own vectors are run'
    )
    vectors = (
        'all 45 of `tools/vectors/slip39/`, each again through its EMS and its decryption'
    )
    corpus = (
        'splits fed the same random bytes, which must match word for word; recovery '
        'from qualifying and short subsets and a changed word; the passphrase cipher; '
        'a set\'s EMS from a quorum, refused from a short or changed set, split again on '
        'the same random bytes (some under a new identifier where extendable) and a member '
        'added or a lost one made again at an index, each recovering the first set\'s '
        'secret, and a member refused at an index given, past 15, from a short quorum and '
        'across two groups; random seeds of 16 and 32 bytes through the fixed 1 of 1 against '
        'both libraries under the fingerprint\'s first 15 bits, recovered through both, and '
        'the lengths it refuses, against Faraday\'s own rule; the seed written as codex32 by '
        'Faraday and read by python-codex32, and a SLIP-39 set with a passphrase recovered '
        'by Faraday and by python-shamir-mnemonic, written as codex32 by Faraday and by '
        'python-codex32, the second read by Faraday, '
        'the same bytes both ways, the identifier\'s 20 bits written and the fingerprint '
        '(btclib\'s) compared on both sides'
    )
    differences = [
        (
            'SLIP-39: Faraday refuses a group threshold of 1 over several groups; '
            'python-shamir-mnemonic makes the shares. The area checks that this still '
            'holds, so a change either way is seen.'
        ),
    ]
    not_compared = (
        'none'
    )

    def split(self, secret, passphrase, identifier, extendable, e, gt, groups, random):
        return self.compare("slip39.split", {
            "secret": secret.hex(), "passphrase": passphrase, "identifier": identifier,
            "extendable": extendable, "exponent": e, "group_threshold": gt,
            "groups": [list(g) for g in groups], "random": random.hex()},
            lambda: ref_split(secret, passphrase.encode(), identifier, extendable, e, gt,
                              groups, random),
            also=lambda: btl_split(secret, passphrase, identifier, extendable, e, gt,
                                   groups, random))

    def recover(self, mnemonics, passphrase, expected=None):
        return self.compare("slip39.recover", {"mnemonics": mnemonics, "passphrase": passphrase},
                            lambda: {"secret": S.combine_mnemonics(mnemonics, passphrase.encode()).hex()},
                            expected,
                            also=lambda: {"secret": B.master_secret_from_mnemonics(
                                mnemonics, passphrase).hex()})

    def cipher(self, op, value, passphrase, identifier, extendable, e):
        fn = cipher.encrypt if op == "encrypt" else cipher.decrypt

        def also():
            if op == "encrypt":
                return {"value": B._feistel(value, passphrase, e, identifier, extendable,
                                            decrypt=False).hex()}
            return {"value": B.EncryptedMasterSecret(identifier, extendable, e, value)
                    .master_secret(passphrase).hex()}
        return self.compare(f"slip39.{op}", {
            "value": value.hex(), "passphrase": passphrase, "identifier": identifier,
            "extendable": extendable, "exponent": e},
            lambda: {"value": fn(value, passphrase.encode(), e, identifier, extendable).hex()},
            also=also)

    def recover_ems(self, mnemonics, expected=None):
        return self.compare("slip39.recover_ems", {"mnemonics": mnemonics},
                            lambda: ref_recover_ems(mnemonics), expected,
                            also=lambda: btl_recover_ems(mnemonics))

    def split_ems(self, ems, gt, groups, random):
        args = (bytes.fromhex(ems["ciphertext"]), ems["identifier"], ems["extendable"],
                ems["exponent"], gt, groups, random)
        return self.compare("slip39.split_ems", {
            **ems, "group_threshold": gt, "groups": [list(g) for g in groups],
            "random": random.hex()},
            lambda: ref_split_ems(*args), also=lambda: btl_split_ems(*args))

    def derive_member(self, mnemonics, index):
        return self.compare("slip39.derive_member", {"mnemonics": mnemonics, "index": index},
                            lambda: ref_derive(mnemonics, index),
                            also=lambda: btl_derive(mnemonics, index))

    def published(self):
        for desc, mnemonics, secret, _xprv in vectors("slip39", "vectors.json"):
            want = ("ok", {"secret": secret}) if secret else ("err", None)
            try:
                self.recover(mnemonics, "TREZOR", want)
                # The EMS, then its decryption, give the vector's secret;
                # every refused vector is refused before decryption.
                ems = self.recover_ems(mnemonics, None if secret else ("err", None))
                if secret:
                    ems = ems[1]
                    got = self.cipher("decrypt", bytes.fromhex(ems["ciphertext"]), "TREZOR",
                                      ems["identifier"], ems["extendable"], ems["exponent"])
                    if got[1]["value"] != secret:
                        raise AssertionError("the EMS does not decrypt to the vector's secret")
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
        # bip39_slip39 makes the same shares as the reference.
        alt = btl_split(bytes(16), "", 1, True, 0, 1, [(1, 1), (2, 3)], bytes(64))
        if alt != ref:
            raise Mismatch(self.name, "slip39.split", args, ("ok", alt), ("ok", ref),
                           "bip39_slip39 differs from the reference where Faraday refuses")
        self.second["judged"] += 1

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

    def quorum(self, by_group, gt, groups):
        """A qualifying subset: gt groups, each its threshold of members."""
        rng = self.rng
        chosen = rng.sample(range(len(groups)), gt)
        return {gi: rng.sample(by_group[gi], groups[gi][0]) for gi in chosen}

    def sets_case(self):
        """A set's EMS, a new set split from it, and a member added to a
        group or a lost one made again: each against the references, and
        each recovering the secret the first set was split from."""
        rng = self.rng
        secret = self.randbytes(rng.choice([16, 32, 2 * rng.randint(8, 16)]))
        passphrase = "".join(rng.choice(PRINTABLE) for _ in range(rng.choice([0, 6])))
        identifier, extendable, e = rng.randrange(1 << 15), rng.random() < 0.5, rng.choice([0, 1])
        gt, groups = self.random_groups()
        shares = self.split(secret, passphrase, identifier, extendable, e, gt, groups,
                            self.randbytes(tape_length(len(secret), gt, groups)))[1]["mnemonics"]
        by_group, at = [], 0
        for _t, n in groups:
            by_group.append(shares[at:at + n])
            at += n

        # The EMS from a quorum, and refused from a short or changed set.
        quorum = self.quorum(by_group, gt, groups)
        subset = [m for ms in quorum.values() for m in ms]
        rng.shuffle(subset)
        ems = self.need(self.recover_ems(subset), "the EMS of a quorum")
        if len(subset) > 1:
            self.recover_ems(subset[:-1])
        words = subset[0].split()
        words[rng.randrange(len(words))] = rng.choice(S.share.wordlist.WORDLIST)
        self.recover_ems([" ".join(words)] + subset[1:])

        # Rotate: a new split of the EMS opens the same wallet; an
        # extendable set may take a new identifier.
        new_gt, new_groups = self.random_groups()
        rotated = dict(ems)
        if extendable and rng.random() < 0.5:
            rotated["identifier"] = rng.randrange(1 << 15)
        new = self.need(self.split_ems(
            rotated, new_gt, new_groups,
            self.randbytes(tape_length(len(secret), new_gt, new_groups))), "a rotated set")
        new_by_group, at = [], 0
        for _t, n in new_groups:
            new_by_group.append(new["mnemonics"][at:at + n])
            at += n
        again = [m for ms in self.quorum(new_by_group, new_gt, new_groups).values() for m in ms]
        if self.recover(again, passphrase)[1]["secret"] != secret.hex():
            raise AssertionError("the rotated set does not recover the secret")

        # A member at an index: of a group whose threshold is above 1.
        wide = [gi for gi, (t, _n) in enumerate(groups) if t > 1]
        if not wide:
            gi = rng.randrange(len(groups))
            self.derive_member(rng.sample(by_group[gi], groups[gi][0]), rng.randrange(16))
            return
        gi = rng.choice(wide)
        t, n = groups[gi]
        members = rng.sample(range(n), t)
        given = [by_group[gi][mi] for mi in members]
        free = [i for i in range(16) if i not in members]
        index = rng.choice(free)
        made = self.need(self.derive_member(given, index), "a new member")["mnemonic"]
        if index < n and made != by_group[gi][index]:
            raise AssertionError("a lost share made again differs from the original")
        others = {g: ms for g, ms in self.quorum(by_group, gt, groups).items() if g != gi}
        if len(others) == gt:
            others.pop(next(iter(others)))
        with_new = [made] + given[1:] + [m for ms in others.values() for m in ms]
        if self.recover(with_new, passphrase)[1]["secret"] != secret.hex():
            raise AssertionError("a new member does not recover the secret")
        # Refused: an index given, one past 15, a short quorum, two groups.
        self.derive_member(given, rng.choice(members))
        self.derive_member(given, 16)
        self.derive_member(given[:-1], index)
        if len(groups) > 1:
            other = rng.choice([g for g in range(len(groups)) if g != gi])
            self.derive_member(given[:-1] + [by_group[other][0]], index)
        # Refused at the digest: a quorum drawn from two splits of the
        # secret under one identifier.
        twin = self.split(secret, passphrase, identifier, extendable, e, gt, groups,
                          self.randbytes(tape_length(len(secret), gt, groups)))[1]["mnemonics"]
        twin_at = sum(n for _t, n in groups[:gi])
        self.derive_member(given[:-1] + [twin[twin_at + members[-1]]], index)

    def one_of_one(self, seed):
        """A length Faraday refuses (not 16 or 32) is its own rule, which
        both libraries would accept: no `also`, so `compare` counts it as
        not covered by bip39_slip39, once."""
        also = (lambda: ref_one_of_one(seed, btl_split)) if len(seed) in (16, 32) else None
        return self.compare("slip39.one_of_one", {"secret": seed.hex()},
                            lambda: ref_one_of_one(seed, ref_split), also=also)

    def to_codex32(self, seed):
        """Faraday's codex32 string of a seed, which the codex32 reference
        reads: the same bytes and identifier, and the fingerprint btclib
        works out of the bytes the reference read equals the seed's."""
        ident = codex32_identifier(seed)
        made = self.compare("slip39.to_codex32", {"seed": seed.hex()},
                            lambda: {"string": Codex32String(codex32_encode(
                                "ms", f"0{ident}s", seed, pad_val=0)).s.lower()})
        string = self.need(made, "a seed written as codex32")["string"]
        read = Codex32String.from_string("ms", string)
        if read.data != seed or read.ident.lower() != ident or read.share_idx.lower() != "s":
            raise AssertionError("the codex32 reference does not read the seed back")
        if fingerprint(bytes(read.data)) != fingerprint(seed):
            raise AssertionError("the bytes the codex32 reference read have another fingerprint")
        return string

    def from_codex32(self, string, seed):
        """Faraday's reading of a codex32 string the reference wrote: the
        seed and its fingerprint, which btclib works out."""
        return self.compare("slip39.from_codex32", {"string": string},
                            lambda: {"seed": seed.hex(), "fingerprint": fingerprint(seed).hex()})

    def convert_case(self):
        rng = self.rng
        seed = self.randbytes(rng.choice([16, 32]))
        # The fixed 1 of 1: the words match both libraries word for word,
        # and read back through both to the seed.
        words = self.need(self.one_of_one(seed), "a 1 of 1")["mnemonic"]
        if self.recover([words], "")[1]["secret"] != seed.hex():
            raise AssertionError("a 1 of 1 does not recover its seed")
        for bad in (rng.choice([20, 24, 28]), 64, rng.choice([0, 15, 17, 33])):
            self.one_of_one(self.randbytes(bad))
        # Faraday's codex32 of the seed, read by the codex32 reference.
        self.to_codex32(seed)
        # A SLIP-39 set with a passphrase, recovered by python-shamir-mnemonic,
        # written as codex32 by the codex32 reference, read by Faraday.
        passphrase = "".join(rng.choice(PRINTABLE) for _ in range(rng.choice([6, 20])))
        gt, groups = rng.choice([(1, [(1, 1)]), (1, [(2, 3)]), (2, [(1, 1), (2, 3)])])
        gt = 1 if len(groups) == 1 else gt
        shares = self.need(self.split(
            seed, passphrase, rng.randrange(1 << 15), rng.random() < 0.5, 0, gt, groups,
            self.randbytes(tape_length(len(seed), gt, groups))), "a set")["mnemonics"]
        subset, at = [], 0
        for t, n in groups:
            subset += rng.sample(shares[at:at + n], t)
            at += n
        got = S.combine_mnemonics(subset, passphrase.encode())
        if got != seed:
            raise AssertionError("python-shamir-mnemonic does not recover the seed")
        ours = self.need(self.recover(subset, passphrase), "a set with a passphrase")
        if bytes.fromhex(ours["secret"]) != seed:
            raise AssertionError("Faraday does not recover the seed")
        self.to_codex32(bytes.fromhex(ours["secret"]))
        wallet = codex32_encode("ms", f"0{codex32_identifier(got)}s", got, pad_val=0)
        self.from_codex32(wallet, got)

    LOOPS = (("corpus", 1500), ("sets", 500), ("convert", 100))

    def prelude(self):
        self.published()
        self.known_differences()
        self.limits()

    def case_corpus(self):
        self.corpus_case()

    def case_sets(self):
        self.sets_case()

    def case_convert(self):
        self.convert_case()
