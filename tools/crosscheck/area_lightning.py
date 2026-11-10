"""Lightning node keys and the aezeed (`faraday_bip::aezeed`) against
LND's own code (the vendored Go tree of `tools/reference/lnd/`, one
process for the run: `main.go` there) and ldk's published answers
(`tools/vectors/aezeed/`), with ldk's derivation written here from the
sources `tools/reference/aezeed/README.md` quotes, over btclib's BIP-32
(`bip32_ref.py`, btclib's own curve) and python-mnemonic (`bip39/`).

- LND (README §3), answered by LND's code: the aezeed read by LND's
  `aezeed` package (its wordlist, version byte, CRC-32C, scrypt and the AEZ
  of Yawning Angel's `aez`, unchanged), and the node key at
  `m/1017'/coin'/6'/0/0` of the aezeed's sixteen bytes of entropy taken as
  the BIP-32 seed over btcd's `hdkeychain`, coin 0 on mainnet and 1
  elsewhere. The path's numbers are `keychain/derivation.go`'s, written in
  `main.go`; LND derives them through btcwallet, which is not vendored.
- ldk-node (README §6), still ours as a composition: a master key from
  the 64-byte BIP-39 seed, its private key taken as LDK's 32-byte seed, a
  second master key from that, and `m/0'` of the second; the network
  changes nothing. Each master key, the derivation and the public key are
  btclib's.

- Core Lightning (`tools/reference/cln/`, F228), answered by its code: its
  `hsm_secret` (32 bytes plain, or 73 bytes encrypted under a passphrase,
  which `get_encryption_key` and `decrypt_hsm_secret` of `common/hsm_secret.c`
  open over libsodium) and the node key `hsmd` derives from it (ccan's
  HKDF-SHA256, the info `nodeid`, a 4-byte salt from 0 until the 32 bytes are
  a private key; the public key is btclib's). The encrypted files are made by
  libsodium through Core Lightning's `encrypt_legacy_hsm_secret`. Core
  Lightning's node from BIP-39 words is the first 32 bytes of their seed
  (python-mnemonic's) through the same derivation.
  Its mnemonic form (a file over 32 bytes and not 73: a 32-byte hash, zero
  without a passphrase and else the SHA-256 of the 64-byte seed, then the
  words as text) is read by `detect_hsm_secret_type` and
  `extract_mnemonic_secret` as `main.c` writes them out, with the hash
  compared by ccan's SHA-256; the one part not Core Lightning's code, which
  uses libwally for it, is BIP-39 itself (the words' validity and the
  seed), which python-mnemonic (`bip39/`) gives, an implementation Faraday
  did not write.

- Every vector of `tools/vectors/aezeed/`, and Core Lightning's published node
  ids for BIP-39 words, and a file libsodium sealed under Core Lightning's
  test passphrase, made for this check (its node id is the one Core
  Lightning's test of a plain 0x01 secret gives).
- Random BIP-39 seeds on every network (ldk) and random entropies on
  every network (LND).
- Random 32-byte secrets plain, random secrets sealed by libsodium under
  random passphrases (the sealed file opened by both), random BIP-39 words
  under random passphrases, mnemonic-form files of random words with and
  without a passphrase (the same node id from both), and files shorter than
  32 bytes; a sealed file under another passphrase, or with one byte
  changed, a mnemonic-form file under a wrong passphrase, under a passphrase
  when it has none, or with its words broken (the checksum failing): each
  refused by both. A NUL in the words is not compared: Core Lightning reads
  the text up to it and Faraday refuses the file, as it refuses any text
  Core Lightning might read differently.
- Random word lists (the checksum fails), lists with the checksum put
  right and a version other than 0, and the published seeds under a
  changed passphrase, with an enciphered byte or the salt changed and the
  checksum put right, and with a word changed: each of them LND refuses.
- Every aezeed case above is asked twice. The second time Faraday's command
  is the one built with the aes crate's software round (`--cfg
  aes_backend="soft"`, the fixsliced round the Pi runs) and LND's program
  the one built without assembly (tags `noasm`, `purego`: Yawning Angel's AEZ
  then runs its own bitsliced round), both made by `tools/crosscheck/run.sh`
  and checked by `soft.py` before the run; on a processor with AES-NI the
  first pass compares the processor's round on both sides, so the software
  round is compared only here.
"""

import hashlib
import json
import os
import re
import subprocess

import bip32_ref
import bolt11_enc
import bolt12_enc
from common import ROOT, Area, HarnessError, Mismatch, Rust, reference, use_reference, use_wheels

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

use_wheels("bolt11", [
    "bech32-1.2.0-py3-none-any.whl",
    "base58-2.1.1-py3-none-any.whl",
    "bitstring-3.1.9-py3-none-any.whl",
    "bolt11-2.2.0-py3-none-any.whl",
])
use_reference("bolt11", "shims")
import bolt11  # noqa: E402
from bolt11.bit_utils import scid_to_int  # noqa: E402
from bolt11.models.features import FeatureState  # noqa: E402

H = bip32_ref.HARDENED
NETWORKS = ["mainnet", "testnet", "signet", "regtest"]
ENGLISH = Mnemonic("english")


def crc32c(data):
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ (0x82F63B78 if crc & 1 else 0)
    return crc ^ 0xFFFFFFFF


def to_bytes(words):
    n = 0
    for w in words:
        n = (n << 11) | w
    return n.to_bytes(33, "big")


def to_words(data):
    n = int.from_bytes(data, "big")
    return [(n >> (11 * (23 - i))) & 2047 for i in range(24)]


def with_checksum(data):
    return data[:29] + crc32c(data[:29]).to_bytes(4, "big")


def ldk(seed):
    """Costs an elliptic-curve multiplication or two, in btclib."""
    outer = bip32_ref.master(seed)
    inner = bip32_ref.master(outer.k.to_bytes(32, "big"), True)
    child = inner.derive([H])
    return {"secret": child.k.to_bytes(32, "big").hex(), "node_id": child.pub.hex()}


def cln_exe():
    """Core Lightning's answers as a program: built by `tools/crosscheck/run.sh`
    from `tools/reference/cln/` (`FARADAY_CLN_REF` names another place)."""
    path = os.environ.get("FARADAY_CLN_REF") or str(ROOT / "out/crosscheck/cln-ref")
    if not os.access(path, os.X_OK):
        raise HarnessError(
            f"the lightning area needs Core Lightning's answer program, {path}, which "
            "tools/crosscheck/run.sh builds from tools/reference/cln/ with a C compiler and "
            "libsodium (docs/DEVELOPING.md)")
    return path


# secp256k1's group order: a 32-byte HKDF output is a private key below it and above 0.
CURVE_ORDER = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141


class Cln:
    """The C program answering with Core Lightning's functions, a request a
    line: `ok HEX` or `err`."""

    def __init__(self):
        self.proc = subprocess.Popen([cln_exe()], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     text=True, bufsize=1)

    def call(self, *words):
        self.proc.stdin.write(" ".join(words) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise HarnessError(f"{words[0]}: Core Lightning's answer program exited")
        kind, _, rest = line.strip().partition(" ")
        if kind == "ok":
            return rest
        raise ValueError(f"{words[0]}: refused")

    def node(self, secret):
        """Costs an elliptic-curve multiplication in btclib. The node key
        `hsmd` derives from `secret` (`node_key` in `hsmd/libhsmd.c`):
        ccan's HKDF-SHA256 with the info `nodeid` and the salt 0, 1, ... until
        the output is a private key."""
        salt = 0
        while True:
            okm = bytes.fromhex(self.call("hkdf", secret.hex(), str(salt)))
            k = int.from_bytes(okm, "big")
            if 0 < k < CURVE_ORDER:
                return {"secret": okm.hex(), "node_id": bip32_ref.point(k).hex()}
            salt += 1

    def file(self, data, passphrase):
        """Core Lightning's reading of an `hsm_secret` file, told apart by
        length as `detect_hsm_secret_type` does: 32 bytes plain, 73
        encrypted, every other length over 32 the mnemonic form, a shorter
        file invalid."""
        if len(data) == 32:
            return self.node(data)
        if len(data) == 73:
            return self.node(bytes.fromhex(
                self.call("open", data.hex(), passphrase.encode().hex() or "-")))
        if len(data) < 32:
            raise ValueError("invalid hsm_secret format")
        return self.mnemonic(data, passphrase)

    def mnemonic(self, data, passphrase):
        """A mnemonic-form file. libwally's two answers, whether the words
        validate (English list) and their seed, are python-mnemonic's (12 to
        24 words, NFKD where libwally has none: only ASCII cases are asked); the
        text is read up to a NUL, as `tal_strndup` does. An empty passphrase
        is none (NULL) when the hash is zero, and given as "" otherwise."""
        text = data[32:].split(b"\0")[0]
        zero = not any(data[:32])
        given = None if zero and passphrase == "" else passphrase
        try:
            words = text.decode()
        except UnicodeDecodeError:
            words = None
        valid = words is not None and ENGLISH.check(words)
        seed = ENGLISH.to_seed(words, given or "") if words is not None else bytes(64)
        pass_arg = "none" if given is None else (given.encode().hex() or "-")
        return self.node(bytes.fromhex(self.call(
            "mnemonic", data.hex(), pass_arg, "1" if valid else "0", seed.hex())))

    def seal(self, secret, passphrase):
        return bytes.fromhex(self.call("seal", secret.hex(), passphrase.encode().hex() or "-"))

    def version(self):
        return self.call("version")

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


def lnd_exe():
    """LND's answers as a program: built by `tools/crosscheck/run.sh` from
    the vendored Go tree (`FARADAY_LND_REF` names another place)."""
    path = os.environ.get("FARADAY_LND_REF") or str(ROOT / "out/crosscheck/lnd-ref")
    if not os.access(path, os.X_OK):
        raise HarnessError(
            f"the lightning area needs LND's answer program, {path}, which "
            "tools/crosscheck/run.sh builds from tools/reference/lnd/ with the Go toolchain "
            "(docs/DEVELOPING.md: the version go.mod names, installed from go.dev/dl)")
    return path


def soft_exes():
    """The software-round pass's two programs, which `run.sh` builds and
    names: Faraday's command with the aes crate's software backend and LND's
    program without assembly."""
    rust, lnd = os.environ.get("FARADAY_SOFT_RUST"), os.environ.get("FARADAY_LND_SOFT_REF")
    for path in (rust, lnd):
        if not path or not os.access(path, os.X_OK):
            raise HarnessError(
                "the lightning area's software-round pass needs the two programs "
                "tools/crosscheck/run.sh builds (FARADAY_SOFT_RUST, FARADAY_LND_SOFT_REF); "
                f"{path or 'one'} is missing")
    return rust, lnd


# Faraday's refusals of what the `bolt11` package accepts. BOLT #11 makes a
# wrong-length p, h, s or n, a repeated p or s, and both or neither of d and
# h a refusal too, as do mixed case, an amount finer than a millisatoshi
# and a leading zero value in x, c or 9 (the package lags the specification
# there); it leaves a repeated x, c, m, 9, d, h or n, a text over 1023
# characters and an amount with a leading zero open, which Faraday refuses
# (docs/STANDARDS.md, BOLT 11).
STRICTER = {"TooLong", "MixedCase", "Amount", "Field", "Duplicate", "UnknownRequiredFeature",
            "Description"}
NETWORK_OF = {"bc": "bc", "tb": "tb", "tbs": "tbs", "bcrt": "bcrt"}


def bolt11_ref(text):
    """What the `bolt11` package reads of an invoice, in the shape of
    Faraday's answer (the fields it reads: not the fallback address, which
    it writes in bech32 for every witness version, nor unknown fields)."""
    inv = bolt11.decode(text)
    first_f = inv.tags.get(bolt11.models.tags.TagChar.fallback)
    fallback = None
    if first_f is not None and first_f.data is not None:
        data = first_f.data.data
        program = data[5:]
        fallback = {"version": data[0:5].uint,
                    "data": program[: len(program) - len(program) % 8].tobytes().hex()}
    features = []
    if inv.features is not None:
        for feature, state in inv.features.feature_list.items():
            features.append(feature.value * 2 + (1 if state == FeatureState.supported else 0))
    return {
        "network": inv.currency,
        "amount_msat": None if inv.amount_msat is None else int(inv.amount_msat),
        "timestamp": inv.date,
        "payment_hash": inv.payment_hash,
        "description": inv.description,
        "description_hash": inv.description_hash,
        "payee": inv.payee,
        "expiry": inv.expiry,
        "min_final_cltv_expiry": inv.min_final_cltv_expiry,
        "fallback": fallback,
        "route_hints": [[{
            "node_id": r.public_key, "short_channel_id": scid_to_int(r.short_channel_id),
            "fee_base_msat": r.base_fee % 2**32,
            "fee_proportional_millionths": r.ppm_fee % 2**32,
            "cltv_expiry_delta": r.cltv_expiry_delta % 2**16} for r in hint.routes]
            for hint in inv.route_hints],
        "features": sorted(features),
        "payment_secret": inv.payment_secret,
        "metadata": inv.metadata,
    }


def bolt11_view(got):
    """Faraday's answer cut to what the package reads: its first fallback
    (the others are compared with the writer's list, `invoice`)."""
    out = {k: v for k, v in got.items()
           if k not in ("fallbacks", "unknown", "payee_from_field")}
    first = got["fallbacks"][0] if got["fallbacks"] else None
    out["fallback"] = first
    return out


def bolt12_exe():
    """Core Lightning's offer decoder, `devtools/bolt12-cli`: built by
    `tools/crosscheck/run.sh` from the pinned release
    (`tools/reference/bolt12/`; `FARADAY_BOLT12_CLI` names another place)."""
    path = os.environ.get("FARADAY_BOLT12_CLI") or str(ROOT / "out/crosscheck/bolt12-cli")
    if not os.access(path, os.X_OK):
        raise HarnessError(
            f"the lightning area needs Core Lightning's bolt12-cli, {path}, which "
            "tools/crosscheck/run.sh builds from the release tools/reference/bolt12/release.pin "
            "pins (fetched once with tools/crosscheck/fetch_cln.py; docs/DEVELOPING.md)")
    return path


def cln_offer(text):
    """Core Lightning's reading of an offer, `bolt12-cli decode`, in the shape
    of Faraday's answer: ("ok", fields) when it exits 0, else ("err", why).
    Faraday reads a `lightning:` prefix; the decoder does not, so the area
    takes it off first."""
    if text[:10].lower() == "lightning:":
        text = text[10:]
    # The decoder reads one Bech32 string: BOLT #12's `+` between two Bech32
    # characters, and the white space after it (any, as Python's \s), are taken
    # out first; a `+` anywhere else stays and the decoder refuses it.
    text = re.sub(r"(?<=[ac-hj-np-z0-9])\+\s*(?=[ac-hj-np-z0-9])", "", text, flags=re.I)
    if "\0" in text:
        # An argument cannot hold a NUL byte: no string to give the command.
        return ("err", "NUL")
    run = subprocess.run([bolt12_exe(), "decode", text], capture_output=True, timeout=60)
    out = run.stdout.decode("utf-8", "replace")
    if run.returncode == 0:
        return ("ok", cln_view(out))
    # Exit 1 is a decoder's refusal, 3 a text that is not Bech32 (a `+` join
    # is for the callers of the decoder: `joined` below).
    if run.returncode in (1, 3):
        return ("err", (run.stderr.decode("utf-8", "replace").strip().splitlines() or [""])[-1])
    raise HarnessError(f"bolt12-cli decode {text[:60]}... ended with {run.returncode}: "
                       f"{run.stderr.decode('utf-8', 'replace')[-200:]}")


def scid_dir(text):
    """Core Lightning's `103x1x0/1` as a short channel id and a direction."""
    scid, _, direction = text.partition("/")
    block, tx, out = (int(x) for x in scid.split("x"))
    return block << 40 | tx << 16 | out, int(direction)


def cln_view(out):
    """What `bolt12-cli decode` prints of an offer, as the area's comparison
    shape (hashes are printed as block hashes, reversed)."""
    v = {"chains": [], "metadata": None, "amount": None, "description": None, "features": [],
         "expiry": None, "paths": [], "issuer": None, "quantity_max": None, "issuer_id": None,
         "unknown": []}
    for line in out.split("\n"):
        name, _, rest = line.partition(": ")
        if name == "offer_chains":
            v["chains"] = [bytes.fromhex(h)[::-1].hex() for h in rest.split()]
        elif name == "offer_metadata":
            v["metadata"] = rest
        elif name == "offer_amount":
            m = re.fullmatch(r"(\d+)(?:\.(\d+))?(.*)", rest)
            whole, frac, unit = m.group(1), m.group(2) or "", m.group(3)
            if unit and unit.islower():
                v["amount"] = {"msat": int(whole + frac)}
            else:
                v["amount"] = {"currency": unit, "exponent": len(frac), "units": int(whole + frac)}
        elif name == "offer_description":
            v["description"] = rest
        elif name == "offer_features":
            v["features"] = [int(b) for b in rest.split()]
        elif name == "offer_absolute_expiry":
            v["expiry"] = int(rest.split()[0])
        elif name.startswith("offer_paths "):
            m = re.fullmatch(r"first_path_key \w+ first_node_id (\S+) path (.*)", rest)
            first, hops = m.group(1), m.group(2).split()
            if "x" in first:
                scid, direction = scid_dir(first)
                v["paths"].append({"scid": scid, "direction": direction, "hops": len(hops)})
            else:
                v["paths"].append({"node": first, "hops": len(hops)})
        elif name == "offer_issuer":
            v["issuer"] = rest
        elif name == "offer_quantity_max":
            v["quantity_max"] = int(rest)
        elif name == "offer_issuer_id":
            v["issuer_id"] = rest
        elif name.startswith("Unknown field "):
            v["unknown"].append({"type": int(name.split()[2]), "data": rest})
    return v


# Faraday's refusals of what Core Lightning's decoder accepts (F232's list of
# what BOLT #12 leaves open), by the reason Faraday names, and the writer's
# kind that makes each. A refusal Core Lightning shares is no difference.
OFFER_STRICTER = {
    "too_long": {"TooLong"},
    "padding_nonzero": {"Padding"},
    "plus_whitespace": {"Join"},
    "plus_not_bech32": {"Join"},
    "chain_twice": {"ChainRepeated"},
    "chains_unknown": {"ChainUnknown"},
    "currency": {"Currency", "Field"},
    "amount_over_21m": {"AmountTooLarge"},
    "expiry_late": {"ExpiryTooLate"},
    "first_node_byte": {"Path"},
    "unassigned_char": {"Text"},
}

# Where Faraday refuses what Core Lightning's command reads, and the refusal is
# BOLT #12's or BOLT #1's own (not F232's stricter list), by the reason Faraday
# names: findings about Core Lightning, kept as known differences.
#   UnknownRequiredFeature: `bolt12-cli` hands the decoder no feature set, so an
#     offer that sets an even feature bit is read.
#   FeaturesNotMinimal: the decoder does not check that the feature bytes have
#     no leading zero byte.
#   Text: the decoder checks that a description, issuer or currency is UTF-8,
#     not that it holds none of BOLT #1's refused characters.
OFFER_CLN_READS = {"UnknownRequiredFeature", "FeaturesNotMinimal", "Text"}


def cln_iso4217():
    """The currency codes of Core Lightning's own table (`common/iso4217.c` of
    the unpacked release, ISO 4217 as of 2018)."""
    for path in sorted((ROOT / "out/crosscheck").glob("cln-*/common/iso4217.c")):
        return set(re.findall(r'\{ "([A-Z]{3})", \d+ \}', path.read_text()))
    raise HarnessError("no unpacked Core Lightning release in out/crosscheck "
                       "(tools/crosscheck/build_bolt12_cli.sh)")


class Lnd:
    """The Go program answering with LND's code, a request a line."""

    def __init__(self, exe=None):
        self.proc = subprocess.Popen([exe or lnd_exe()], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     text=True, bufsize=1)
        self.next_id = 0

    def call(self, op, **args):
        self.next_id += 1
        self.proc.stdin.write(json.dumps(dict(args, id=self.next_id, op=op)) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise HarnessError(f"{op}: LND's answer program exited")
        out = json.loads(line)
        if out.get("id") != self.next_id:
            raise HarnessError(f"{op}: LND's answer out of step: {line.strip()}")
        if "ok" in out:
            return out["ok"]
        raise ValueError(out["err"])

    def close(self):
        self.proc.stdin.close()
        self.proc.wait()


class Lightning(Area):
    # The published vectors, which corpus cases also draw from.
    aezeed_vectors = json.loads((ROOT / "tools/vectors/aezeed/vectors.json").read_text())
    name = "lightning"
    faraday = (
        "`faraday_bip::aezeed` (ldk-node's and LND's node keys, the aezeed read), "
        "`faraday_codec::bolt11::decode` (a BOLT #11 invoice read), "
        "`faraday_codec::bolt12::decode` (a BOLT #12 offer read), "
        "`faraday_keys::hsm_secret` (Core Lightning's node key from its hsm_secret, plain "
        "encrypted or the mnemonic form, and from BIP-39 words)"
    )
    reference = (
        "LND's own code, vendored in Go (`tools/reference/lnd/`: its `aezeed` package "
        "over Yawning Angel's `aez`, and the node key over btcd's `hdkeychain`), one "
        "process answering the run's requests (the aezeed also asked of a second build "
        "of each side, Faraday's with the aes crate's software round and LND's without "
        "assembly); ldk-node's derivation as "
        "`tools/reference/aezeed/README.md` quotes it, written out in `area_lightning.py` "
        "over btclib's BIP-32 (`btclib/`, on btclib's own curve); for invoices, the "
        "`bolt11` package by lnbits (2.2.0, `tools/reference/bolt11/`: its wheels over "
        "the bech32, base58 and bitstring packages, with a stand-in for its libsecp256k1 "
        "binding `coincurve` written over btclib's curve), and invoices written in "
        "`bolt11_enc.py` from BOLT #11 and signed with btclib; for offers, Core "
        "Lightning's own offer decoder, `devtools/bolt12-cli` built from its source "
        "release by its own configure and make (`tools/reference/bolt12/`), and offers "
        "written in `bolt12_enc.py` from BOLT #12; "
        "Core Lightning's "
        "code (`tools/reference/cln/`: ccan's HKDF-SHA256 compiled as it is, and its "
        "`common/hsm_secret.c` functions written out over the system libsodium, the BIP-39 words and seed of "
        "the mnemonic form from python-mnemonic), one process answering the run's requests"
    )
    reference_checked = (
        "LND's code reads the aezeed (wordlist, version, CRC-32C, scrypt, AEZ) and "
        "derives the node key, and its published answers (`tools/vectors/aezeed/`) are "
        "required of it as well as of Faraday; the Go program is built offline from the "
        "vendored tree (`GOFLAGS=-mod=vendor`, `GOTOOLCHAIN=local`) and its version is "
        "written into the stamp; the area stops when the program is missing; "
        "the software-round pass runs only on programs `soft.py` has checked to be soft "
        "(the tags `noasm` and `purego`; no AES-NI instruction in Faraday's command) and "
        "the command says it was built soft; "
        "ldk-node's derivation is written in the area from the sources the README "
        "quotes, and ldk's published answers are required of it as well as of Faraday; "
        "the `bolt11` package's wheels and its stand-in are hashed against "
        "`references.sha256` before any run, and each of BOLT #11's 31 published "
        "examples is required to be read by Faraday as the BOLT says (a valid one read, "
        "an invalid one refused), the package reading the valid ones to the same fields; "
        "Core Lightning's HKDF is its compiled code, its salt, limits and calls are those "
        "of `hsm_secret.c`, libsodium itself seals and opens the file, and Core "
        "Lightning's two published node ids for words are required of it as well as of "
        "Faraday; the area stops when the program is missing and the stamp records "
        "libsodium's version; Core Lightning's release archive is hashed against "
        "`tools/reference/bolt12/release.pin` by the fetch and again by every build, the "
        "area stops when `bolt12-cli` is missing, and each of the BOLT #12 vectors is "
        "required to be read by Faraday as the vector says (a valid one read, an invalid "
        "one refused) and read by Core Lightning's decoder to the same fields where "
        "Faraday reads it"
    )
    vectors = (
        "LND's five cipher seeds, LND's two node keys, ldk's five node ids "
        '(`tools/vectors/aezeed/`); BOLT #11\'s 31 examples, valid and invalid '
        '(`tools/vectors/lightning-bolt11/`); BOLT #12\'s 53 offer vectors and 12 format '
        'strings, valid and invalid (`tools/vectors/lightning-bolt12/`)'
    )
    corpus = (
        'random BIP-39 seeds and entropies on every network; every aezeed case again with '
        'the aes crate\'s software round against LND\'s AEZ without assembly; random words, a wrong '
        'version with a right checksum, published seeds under a changed passphrase, a '
        'changed enciphered byte or salt with the checksum put right, a changed word, '
        "each refused by LND's code; 150 invoices written with every kind of field "
        "(amounts in every multiplier, description or its hash, payee key, expiry, "
        "final cltv, fallbacks of every version, route hints, features, metadata, "
        "fields BOLT #11 does not define) and signed, read by Faraday and by the "
        "package field by field, then each with one Bech32 character changed, with one "
        "letter in upper case, and with two of these spoiled: a repeated p or x, a "
        "wrong-length p, s, h or n, no p, no s, both or neither of d and h, an "
        "unknown required feature, a leading zero value in a longer x, c or 9, a leading zero or a fraction of a millisatoshi in "
        "the amount, a bad multiplier, an unknown network, a high-S signature, a "
        "field cut short, a text over 1023 characters, a description that is not UTF-8; "
        "for Core Lightning, random plain and encrypted hsm_secrets under random "
        "passphrases (opened by Faraday and by libsodium, node ids compared), a secret "
        "sealed by libsodium and opened by Faraday and by Core Lightning's functions, a changed byte, a wrong "
        "passphrase and a wrong length each refused by both, random mnemonic-form files "
        "with and without a passphrase (node ids compared; a wrong passphrase, a "
        "passphrase the file does not have and broken words each refused by both), and "
        "random BIP-39 words with a passphrase; 250 rounds of BOLT #12 offers, each an "
        "offer written with every kind of field (chains, metadata, a currency or an "
        "amount in millisatoshis, description, features, expiry, blinded paths, issuer, "
        "quantity, issuer id, unknown odd fields; `+` joins, upper case and a "
        "`lightning:` prefix) read by Faraday and by Core Lightning's decoder field by "
        "field and by the writer's own record, then one with a Bech32 character "
        "changed, one in mixed case, and three of thirty-eight spoils: types out of "
        "order or repeated, a field cut short or longer than the stream, a BigSize, "
        "truncated integer or feature field not minimal, a type outside the offer's "
        "ranges or an unknown even one, a text that is not BOLT #1's, an amount without "
        "a description, of zero, or a currency without an amount, no issuer id and no "
        "path, a path of no hops or a bad point, chains empty or of a wrong length, "
        "padding of more than four bits, a bad `+`, a wrong prefix, characters outside "
        "Bech32, and F232's stricter cases (a text over 4,296 characters, a non-zero "
        "padding bit, a chain twice or none known, a currency not in ISO 4217, an amount "
        "above 21 million bitcoin, an expiry past the year 9999, a first node byte that "
        "is no key, an unassigned character)"
    )
    differences = [
        (
            "BOLT #11 invoices: the package skips a p, h, s or n field of the wrong "
            "length, reads the first of a repeated p or s, and accepts both or neither "
            "of a description and its hash, mixed case, an amount finer than a "
            "millisatoshi and a leading zero value in x, c or 9, all of which BOLT #11 "
            "makes a refusal or says \"should\" of and Faraday refuses; it also accepts "
            "what BOLT #11 leaves open, which Faraday is stricter on: a repeated x, c, m, "
            "9, d, h or n, a text over 1023 characters, an amount with a leading zero, and a "
            "required feature bit Faraday does not know; the area counts each "
            "refusal as known, naming its reason"
        ),
        (
            "BOLT #11 route hints: the package reads a hop's fees and delta as signed "
            "integers (a fee above 2^31 reads negative); the area reads them modulo "
            "2^32 and 2^16 before comparing, and keeps a hint to three hops, as the "
            "package drops the last hop of a hint of a multiple of five"
        ),
        (
            "BOLT #11 fallbacks: the package reads only the first fallback address and "
            "writes the others in bech32 for every witness version, no bech32m; the "
            "area compares Faraday's whole list of fallbacks with the writer's f fields "
            "of versions 0 to 18, in order, and its fields of an unknown version by "
            "the writer's list"
        ),
        (
            "BOLT #11 features: the package keeps one state per feature, so a feature "
            "with both its required and its supported bit set reads as supported only; "
            "the writer sets one bit of each pair"
        ),
        (
            "BOLT #12 offers: Core Lightning's `bolt12-cli` passes the decoder no feature "
            "set, so it reads an offer with an even feature bit that Faraday refuses (BOLT "
            "#9 defines no offer feature); it also reads feature bytes with a leading zero "
            "byte, which BOLT #9 makes not minimal, and a description, issuer or currency "
            "holding a character of Unicode's Cc, Cf, Co, Zl or Zp categories, which BOLT "
            "#1 refuses and Faraday refuses; and it refuses what Faraday reads when the "
            "currency is missing from its ISO 4217 table (the list of 2018) or an amount in "
            "bitcoin has a first chain Faraday does not know (it names the unit by the "
            "first chain alone), which Faraday reads as BOLT #12 says; the area counts "
            "each as known, naming the reason, and every other difference fails the run"
        ),
        (
            "BOLT #12 offers, F232's stricter cases: Faraday refuses what Core Lightning "
            "reads (a text over 4,296 characters, a non-zero padding bit, whitespace "
            "after a `+` that is not a space, tab or line break, a chain twice, no known "
            "chain, a currency not in the ISO 4217 list Faraday holds, an amount over 21 "
            "million bitcoin, an expiry past the year 9999, a first node that is no key, "
            "an unassigned character); the area requires Faraday's refusal and counts it "
            "as known"
        ),
    ]
    not_compared = (
        "ldk-node's node key is compared with a derivation written here, not with "
        "ldk's code (Rust, not vendored): only ldk's five published answers are outside "
        "it; LND's key path numbers are written in the Go program from "
        "`keychain/derivation.go`, as LND derives through btcwallet, which is not vendored; "
        "the `bolt11` package is not asked the fields it does not read (fields BOLT #11 "
        "does not define, a fallback of a witness version above 16, a second or later "
        "fallback), which are compared with the writer's own list; its signature check runs on "
        "btclib's curve, not libsecp256k1's, and refuses a high-S signature as "
        "libsecp256k1 does; "
        "`hsm_secret.c` itself is not compiled (it needs tal and libwally): its functions "
        "are written out in `main.c`, and for the mnemonic form libwally's two answers, "
        "whether the words validate and their BIP-39 seed, are python-mnemonic's, which "
        "takes 12 to 24 words and normalises the words and passphrase (NFKD) where "
        "libwally takes 27 and 30 words too and normalises nothing, so the area judges "
        "only English 12 to 24-word ASCII cases and Faraday refuses 27 and 30 words "
        "(F228); a NUL in the mnemonic form's words is not compared (Core Lightning reads "
        "the text up to it, Faraday refuses the file); the words' derivation "
        "(`lightning-hsmtool generatehsm`) is BIP-39's seed, whose first 32 bytes are "
        "taken by both sides here; Core Lightning's `bolt12-cli` reads one Bech32 "
        "string, so the area takes out a `+` between two Bech32 characters and the "
        "white space after it, and a `lightning:` prefix, before asking it, and a "
        "text with a NUL byte cannot be given to it (Faraday refuses it); its output "
        "is read as text (a short channel id as `BxTxO/direction`, chain hashes "
        "reversed)"
    )

    lnd_proc = None
    cln_proc = None
    soft_rust = None
    soft_lnd = None

    def __init__(self, exe, seed, scale, sabotage=None):
        # The self-test makes the software pass's Faraday wrong on its own
        # (`lightning.aezeed-soft`), the first pass's with `lightning.aezeed`.
        soft_sabotage = None
        if sabotage == "lightning.aezeed-soft":
            sabotage, soft_sabotage = None, "lightning.aezeed"
        super().__init__(exe, seed, scale, sabotage)
        rust_exe, lnd_exe_soft = soft_exes()
        self.soft_rust = Rust(rust_exe, self.name, soft_sabotage)
        self.soft_lnd = Lnd(lnd_exe_soft)
        backend = self.soft_rust.call("lightning.backend")
        if backend != ("ok", {"aes": "soft"}):
            raise HarnessError(
                f"the software-round pass's Faraday command is not built with the software "
                f"AES round: {backend}")

    def lnd_ref(self):
        if self.lnd_proc is None:
            self.lnd_proc = Lnd()
        return self.lnd_proc

    def cln_ref(self):
        if self.cln_proc is None:
            self.cln_proc = Cln()
        return self.cln_proc

    def close(self):
        if self.lnd_proc is not None:
            self.lnd_proc.close()
        if self.cln_proc is not None:
            self.cln_proc.close()
        if self.soft_lnd is not None:
            self.soft_lnd.close()
        if self.soft_rust is not None:
            self.soft_rust.close()
        super().close()

    def ldk(self, seed, network, expected=None):
        return self.compare("lightning.ldk", {"seed": seed.hex(), "network": network},
                            lambda: ldk(seed), expected)

    def lnd(self, entropy, network, expected=None):
        return self.compare("lightning.lnd", {"entropy": entropy.hex(), "network": network},
                            lambda: self.lnd_ref().call("node", entropy=entropy.hex(),
                                                        network=network), expected)

    def hsm_file(self, data, passphrase, expected=None):
        """Faraday's reading of an `hsm_secret` file against Core Lightning's."""
        return self.compare("lightning.hsm_file", {"file": data.hex(), "passphrase": passphrase},
                            lambda: self.cln_ref().file(data, passphrase), expected)

    def hsm_words(self, words, passphrase, expected=None):
        def ref():
            # hsmd/libhsmd.c node_key: "Use first 32 bytes for node key derivation"
            return self.cln_ref().node(ENGLISH.to_seed(words, passphrase)[:32])
        return self.compare("lightning.hsm_words", {"words": words, "passphrase": passphrase},
                            ref, expected)

    def aezeed(self, words, passphrase, cost, expected):
        """Faraday's reading against LND's, and the expected answer: a
        published seed's, or a refusal."""
        def ref():
            return self.lnd_ref().call("aezeed", words=[ENGLISH.wordlist[w] for w in words],
                                       passphrase=passphrase.hex(), cost=cost)
        inputs = {"words": words, "passphrase": passphrase.hex(), "cost": cost}
        got = self.compare("lightning.aezeed", inputs, ref, expected)
        self.aezeed_soft(inputs, expected)
        return got

    def aezeed_soft(self, inputs, expected):
        """The same case with the aes crate's software round against LND's
        AEZ without assembly."""
        rust = self.soft_rust.call("lightning.aezeed", **inputs)
        lnd = reference(lambda: self.soft_lnd.call(
            "aezeed", words=[ENGLISH.wordlist[w] for w in inputs["words"]],
            passphrase=inputs["passphrase"], cost=inputs["cost"]))
        self.compared += 1
        op = "lightning.aezeed-soft"
        if rust[0] != lnd[0] or (rust[0] == "ok" and rust[1] != lnd[1]):
            raise Mismatch(self.name, op, inputs, rust, lnd)
        if expected is not None and (expected[0] != lnd[0]
                                     or (expected[0] == "ok" and expected[1] != lnd[1])):
            raise Mismatch(self.name, op, inputs, rust, lnd,
                           f"both differ from the published vector {expected}")

    def invoice(self, text, expect, tagged=None):
        """A BOLT #11 invoice: Faraday's reading against the `bolt11`
        package's, field by field where both accept. `expect` is what
        Faraday must do ("ok" or "err": the invoice was built valid, or
        spoiled, or is a published vector). The package is laxer, so
        where Faraday refuses and the package accepts, the refusal must be
        one of Faraday's stricter rules (`STRICTER`) and counts as a known
        difference; Faraday accepting what the package refuses is a
        disagreement. `tagged` is the writer's list of fields, which Faraday's
        whole list of fallbacks (versions 0 to 18) and its unknown fields are
        also compared with."""
        inputs = {"invoice": text}
        rust = self.rust.call("lightning.bolt11", **inputs)
        ref = reference(lambda: bolt11_ref(text))
        self.compared += 1
        if rust[0] != expect:
            raise Mismatch(self.name, "lightning.bolt11", inputs, rust, ref,
                           f"Faraday must answer {expect}")
        if rust[0] == "err":
            if ref[0] == "ok":
                reason = re.match(r"\w+", rust[1]).group(0)
                if reason not in STRICTER:
                    raise Mismatch(self.name, "lightning.bolt11", inputs, rust, ref,
                                   "Faraday refuses what the package accepts, not by a stricter rule")
                self.known += 1
            return
        if ref[0] != "ok":
            raise Mismatch(self.name, "lightning.bolt11", inputs, rust, ref,
                           "Faraday accepts what the package refuses")
        got, want = bolt11_view(rust[1]), ref[1]
        if want["fallback"] is not None and want["fallback"]["version"] > 18:
            got["fallback"] = want["fallback"] = None
        if got != want:
            raise Mismatch(self.name, "lightning.bolt11", inputs, got, want)
        if tagged is not None:
            unknown = [{"tag": t, "data": v} for t, v in tagged
                       if t in bolt11_enc.UNDEFINED_TAGS or (t == "f" and v[0] > 18)]
            known = [{"version": v[0], "data": bolt11_enc.from5_bytes(v[1:]).hex()}
                     for t, v in tagged if t == "f" and v[0] <= 18]
            if rust[1]["fallbacks"] != known:
                raise Mismatch(self.name, "lightning.bolt11", inputs, rust[1]["fallbacks"], known,
                               "the fallbacks, as the writer put them")
            if rust[1]["unknown"] != unknown:
                raise Mismatch(self.name, "lightning.bolt11", inputs, rust[1]["unknown"], unknown,
                               "the fields BOLT #11 does not define, as the writer put them")

    def invoices_corpus(self):
        rng = self.rng
        inv = bolt11_enc.random_invoice(rng)
        text = bolt11_enc.text_of(inv)
        # Faraday refuses a text over 1023 characters (BOLT #11 says to read it).
        self.invoice(text, "ok" if len(text) <= 1023 else "err", inv["tagged"])
        self.invoice(bolt11_enc.damaged(text, rng), "err")
        self.invoice(bolt11_enc.mixed_case(text, rng), "err")
        for kind in rng.sample(bolt11_enc.KINDS, 2):
            bad = bolt11_enc.spoil(inv, kind, rng)
            # A high-S signature fails the check against an `n` field; the key
            # recovered from one is the same key.
            high_s_ok = kind == "high_s" and not any(t == "n" for t, _ in inv["tagged"])
            bad_text = bolt11_enc.text_of(bad)
            self.invoice(bad_text, "ok" if high_s_ok and len(bad_text) <= 1023 else "err")

    def cln_refuses(self, fields):
        """Whether Core Lightning's command is known to refuse an offer Faraday
        reads, by what the command does with the amount: a currency code
        missing from its 2018 table, or a bitcoin amount whose first chain is
        none of the five Faraday knows (the command names the amount's unit by
        the first chain alone and exits 1 for one it does not know)."""
        amount = fields["amount"]
        if amount is None:
            return False
        if "currency" in amount:
            return amount["currency"] not in cln_iso4217()
        known = {c.hex() for c in bolt12_enc.KNOWN_CHAINS}
        return bool(fields["chains"]) and fields["chains"][0] not in known

    def offer(self, text, expect, kind=None, written=None):
        """A BOLT #12 offer: Faraday's reading against Core Lightning's
        `bolt12-cli decode`, field by field where both read it. `expect` is
        what Faraday must do ("ok", "err", or "any" for a text damaged by a
        change that may leave a valid offer). A kind of `OFFER_STRICTER` is a
        case of F232's stricter list: Faraday refuses it with one of the
        listed reasons and Core Lightning may read it (a known difference),
        or refuses it too. Any other refusal of Faraday must be Core
        Lightning's too, and a read of Faraday's must be Core Lightning's.
        `written` is the writer's own record of the offer, which both are
        required to read as it says."""
        inputs = {"offer": text}
        rust = self.rust.call("lightning.bolt12", **inputs)
        ref = cln_offer(text)
        self.compared += 1
        if expect != "any" and rust[0] != expect:
            raise Mismatch(self.name, "lightning.bolt12", inputs, rust, ref,
                           f"Faraday must answer {expect} (kind {kind})")
        if rust[0] == "err":
            if ref[0] == "ok":
                reason = re.match(r"\w+", rust[1]).group(0)
                if reason not in OFFER_STRICTER.get(kind, set()) | OFFER_CLN_READS:
                    raise Mismatch(self.name, "lightning.bolt12", inputs, rust, ref,
                                   "Faraday refuses what Core Lightning reads, not by a "
                                   "stricter rule of F232's list")
                self.known += 1
            return
        if ref[0] != "ok":
            if self.cln_refuses(rust[1]):
                self.known += 1
                return
            raise Mismatch(self.name, "lightning.bolt12", inputs, rust, ref,
                           "Faraday reads what Core Lightning refuses")
        if rust[1] != ref[1]:
            raise Mismatch(self.name, "lightning.bolt12", inputs, rust[1], ref[1],
                           "the offer's fields, Faraday and Core Lightning")
        if written is not None and rust[1] != bolt12_enc.expected_view(written):
            raise Mismatch(self.name, "lightning.bolt12", inputs, rust[1],
                           bolt12_enc.expected_view(written), "the fields, as the writer put them")

    def offers_corpus(self):
        rng = self.rng
        o = bolt12_enc.random_offer(rng)
        text = bolt12_enc.text_of(bolt12_enc.stream(bolt12_enc.tlvs_of(o)), rng)
        if rng.random() < 0.2:
            text = rng.choice(["lightning:", "LIGHTNING:", "Lightning:"]) + text
        # Faraday refuses a text over 4,296 characters, which the reader of
        # BOLT #12 reads.
        self.offer(text, "ok" if len(text) <= 4296 else "err",
                   "too_long" if len(text) > 4296 else None, o)
        plain = bolt12_enc.text_of(bolt12_enc.stream(bolt12_enc.tlvs_of(o)))
        self.offer(bolt12_enc.damaged(plain, rng), "any")
        self.offer(bolt12_enc.mixed_case(plain, rng), "err")
        for kind in rng.sample(sorted(bolt12_enc.KINDS), 3):
            fn, who = bolt12_enc.KINDS[kind]
            self.offer(fn(rng), "err", kind)

    def published_offers(self):
        base = ROOT / "tools/vectors/lightning-bolt12"
        for v in json.loads((base / "offers-test.json").read_text()):
            self.offer(v["bolt12"], "ok" if v["valid"] else "err")
        for v in json.loads((base / "format-string-test.json").read_text()):
            self.offer(v["string"], "ok" if v["valid"] else "err")

    def published(self):
        for v in json.loads((ROOT / "tools/vectors/lightning-bolt11/invoice-test.json").read_text()):
            self.invoice(v["invoice"], "ok" if v["valid"] else "err")
        self.published_aezeed()

    def published_aezeed(self):
        v = self.aezeed_vectors
        entropy = bytes.fromhex(v["entropy"])
        for a in v["aezeed"]:
            words = [ENGLISH.wordlist.index(w) for w in a["words"].split()]
            self.aezeed(words, a["passphrase"].encode(), a["cost"], ("ok", {
                "internal_version": a["internal_version"], "birthday": a["birthday"],
                "entropy": v["entropy"]}))
        for n in v["lnd_node"]:
            want = {"secret": n["secret"], "node_id": n["node_id"]}
            self.lnd(entropy, n["network"], ("ok", want))
        for n in v["ldk_node"]:
            seed = ENGLISH.to_seed(n["words"], n["passphrase"])
            for network in NETWORKS:
                self.ldk(seed, network, ("ok", {"secret": n["secret"], "node_id": n["node_id"]}))

    # Core Lightning's own test suite (tests/test_wallet.py) names these two
    # node ids; the file is `\x01` thirty-two times, as its encrypt test
    # writes it, sealed by libsodium under `test_password`.
    CLN_WORDS = (
        ("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon "
         "abandon about", "",
         "03653e90c1ce4660fd8505dd6d643356e93cfe202af109d382787639dd5890e87d"),
        ("ritual idle hat sunny universe pluck key alpha wing cake have wedding",
         "test_passphrase",
         "039020371fb803cd4ce1e9a909b502d7b0a9e0f10cccc35c3e9be959c52d3ba6bd"),
    )
    CLN_SEALED = bytes.fromhex(
        "2c9ac8c7092b7c78712ceb9178be048d98c4a5871ae4eea739d8aa473e0fd750242028ee9acd86ecc6131"
        "d72cbc725c6efda4fb84ff06ba6c9b8eba2bfad6001a618eab8d79b30b911")
    CLN_ONES_ID = "026f61d7ee82f937f9697f4f3e44bfaaa25849cc4f526b3a57326130eba6346002"

    def published_cln(self):
        for words, passphrase, node_id in self.CLN_WORDS:
            self.hsm_words(words, passphrase, ("ok", {
                "secret": self.cln_ref().node(ENGLISH.to_seed(words, passphrase)[:32])["secret"],
                "node_id": node_id}))
        want = ("ok", {"secret": self.cln_ref().node(b"\x01" * 32)["secret"],
                       "node_id": self.CLN_ONES_ID})
        self.hsm_file(b"\x01" * 32, "", want)
        self.hsm_file(self.CLN_SEALED, "test_password", want)
        self.hsm_file(self.CLN_SEALED, "test_passwore", ("err", None))
        # The same words as a mnemonic-form file, as Core Lightning writes it.
        for words, passphrase, node_id in self.CLN_WORDS:
            seed = ENGLISH.to_seed(words, passphrase)
            hashed = hashlib.sha256(seed).digest() if passphrase else bytes(32)
            self.hsm_file(hashed + words.encode(), passphrase, ("ok", {
                "secret": self.cln_ref().node(seed[:32])["secret"], "node_id": node_id}))

    def passphrase(self):
        rng = self.rng
        return "".join(rng.choice("abcXYZ019 !~#_-") for _ in range(rng.choice([0, 1, 7, 20])))

    def hsm_case(self):
        """A plain secret, BIP-39 words, and a file with no hsm_secret in it."""
        rng = self.rng
        self.hsm_file(self.randbytes(32), "")
        words = ENGLISH.to_mnemonic(self.randbytes(rng.choice([16, 24, 32])))
        self.hsm_words(words, self.passphrase())
        # A file shorter than 32 bytes, or longer with no words in it.
        length = rng.choice([n for n in range(0, 140) if n not in (32, 73)])
        self.hsm_file(self.randbytes(length), self.passphrase(), ("err", None))

    def mnemonic_case(self):
        """A mnemonic-form file of random words: read under its passphrase,
        then under a wrong one, with none given, with a passphrase it does
        not have, and with a word changed so the checksum fails."""
        rng = self.rng
        words = ENGLISH.to_mnemonic(self.randbytes(rng.choice([16, 20, 24, 28, 32])))
        passphrase = self.passphrase()
        hashed = hashlib.sha256(ENGLISH.to_seed(words, passphrase)).digest() if passphrase \
            else bytes(32)
        file = hashed + words.encode()
        self.hsm_file(file, passphrase)
        r = rng.random()
        if r < 0.3:
            # A passphrase the file does not have, or another one.
            self.hsm_file(file, passphrase + rng.choice("x! "), ("err", None))
        elif r < 0.45 and passphrase:
            self.hsm_file(file, "", ("err", None))
        elif r < 0.9:
            for _ in range(100):
                broken = words.split()
                broken[rng.randrange(len(broken))] = rng.choice(ENGLISH.wordlist)
                broken = " ".join(broken)
                if not ENGLISH.check(broken):
                    self.hsm_file(hashed + broken.encode(), passphrase, ("err", None))
                    break

    def sealed_case(self):
        """A secret sealed by libsodium: opened under its passphrase, under
        another, and with one byte changed."""
        rng = self.rng
        secret, passphrase = self.randbytes(32), self.passphrase()
        sealed = self.cln_ref().seal(secret, passphrase)
        want = ("ok", self.cln_ref().node(secret))
        self.hsm_file(sealed, passphrase, want)
        r = rng.random()
        if r < 0.5:
            self.hsm_file(sealed, passphrase + rng.choice("x! "), ("err", None))
        else:
            changed = bytearray(sealed)
            changed[rng.randrange(73)] ^= 1 << rng.randrange(8)
            self.hsm_file(bytes(changed), passphrase, ("err", None))

    def corpus_case(self):
        rng = self.rng
        network = rng.choice(NETWORKS)
        words = ENGLISH.to_mnemonic(self.randbytes(rng.choice([16, 20, 24, 28, 32])))
        passphrase = "".join(rng.choice("abcXYZ019 !~") for _ in range(rng.choice([0, 0, 8])))
        self.ldk(ENGLISH.to_seed(words, passphrase), network)
        self.lnd(self.randbytes(16), network)
        # The outer layer: random words, a version other than 0 with the
        # checksum right.
        self.aezeed([rng.randrange(2048) for _ in range(24)], b"", 16, ("err", None))
        data = bytearray(self.randbytes(33))
        data[0] = rng.randint(1, 255)
        self.aezeed(to_words(with_checksum(bytes(data))), b"", 16, ("err", None))
        # A published seed (at the low cost) changed: the passphrase, an
        # enciphered or salt byte with the checksum put right, a word.
        a = rng.choice([x for x in self.aezeed_vectors["aezeed"] if x["cost"] == 16])
        good = [ENGLISH.wordlist.index(w) for w in a["words"].split()]
        r = rng.random()
        if r < 0.4:
            self.aezeed(good, (a["passphrase"] + rng.choice(["x", "!", " "])).encode(), 16,
                        ("err", None))
        elif r < 0.8:
            data = bytearray(to_bytes(good))
            data[rng.randint(1, 28)] ^= 1 << rng.randrange(8)
            self.aezeed(to_words(with_checksum(bytes(data))), a["passphrase"].encode(), 16,
                        ("err", None))
        else:
            bad = good[:]
            i = rng.randrange(24)
            bad[i] = (bad[i] + rng.randint(1, 2047)) % 2048
            self.aezeed(bad, a["passphrase"].encode(), 16, ("err", None))

    # The sealed files stretch 256 MiB three times each (libsodium's seal,
    # Core Lightning's open and Faraday's), so there are few of them.
    LOOPS = (("corpus", 400), ("invoices", 150), ("hsm", 400), ("mnemonic", 200), ("sealed", 6), ("offers", 250))

    def prelude(self):
        self.published()
        self.published_cln()
        self.published_offers()

    def case_corpus(self):
        self.corpus_case()

    def case_invoices(self):
        self.invoices_corpus()

    def case_hsm(self):
        self.hsm_case()

    def case_mnemonic(self):
        self.mnemonic_case()

    def case_offers(self):
        self.offers_corpus()

    def case_sealed(self):
        self.sealed_case()
