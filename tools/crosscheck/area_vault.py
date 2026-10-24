"""The vault (`docs/VAULT.md`) against a reader and writer written here
from that document, over Argon2id from argon2-cffi and
XChaCha20-Poly1305 from PyNaCl (libsodium), the distribution's packages
(F79, `docs/MULTIVENDOR.md` §10.4), with HKDF-SHA256 (RFC 5869) and the
ChaCha20 block (RFC 8439) written here over the standard library.

Before anything else the area checks its own pieces against published
answers (`tools/vectors/vault-primitives/`): HKDF against RFC 5869's
three SHA-256 cases, ChaCha20 against RFC 8439's five block vectors,
PyNaCl against draft-irtf-cfrg-xchacha-03's AEAD vector, argon2-cffi
against `tools/vectors/kdf/`'s known answer. Then:

- Argon2id (`faraday_backup::argon2id`) at random small costs, salts and
  passwords, with the costs and salts Argon2 refuses.
- XChaCha20-Poly1305 as `faraday_vault` calls it: the published vector,
  random keys, nonces, associated data and lengths, and a changed byte of
  the ciphertext, the tag, the nonce or the associated data refused.
- The payload layouts of §7: words, seed and sheet, made and read, with
  every way a payload can be malformed.
- A slot's plaintext (§7): random records of every type, encoded by both
  and read by both, then malformed a byte or a rule at a time; the slot
  vectors of `tools/vectors/vault/slots.json`, written by hand from §7.
- Whole vaults (§1 to §5, §8): `faraday_vault::create` reproduced byte
  for byte from its seed; every passphrase opens its own slot and only
  that one on both sides; a seal under a given nonce reproduced byte for
  byte; the two published vaults (`tools/vectors/vault/`) opened; and
  files refused for each fault of §4 step 1, a wrong passphrase, a
  changed byte, two slots under one key and faulty contents.

VAULT §8 leaves how a new vault's randomness is drawn to the
implementation. `faraday_vault::create` draws it from ChaCha20 (counter
0, a zero nonce) keyed by HKDF-SHA256(seed, empty salt, "Faraday vault
create"): the salt (32 bytes), then the permutation of the slots
(Fisher-Yates from i = 3 down to 1, a byte each, a byte at or above
256 - 256 mod (i + 1) drawn again), then every byte after the header,
then each passphrase's nonce in order. `create` below draws the same
way, so the whole file is compared.

The reference is the document alone: where `docs/VAULT.md` and
`faraday_vault` once read a vault differently (the required fields of
types 7 and 9, a sheet past 16 KiB, an empty passphrase at create), the
owner settled it in the document's words (2026-10-23), and both now
agree.
"""

import hashlib
import hmac
import json
import struct

from common import ROOT, Area, HarnessError, Mismatch

try:
    import nacl.bindings as sodium
    from nacl.exceptions import CryptoError
    from argon2.low_level import Type, hash_secret_raw
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the vault area needs the distribution's Argon2 and libsodium bindings, argon2-cffi "
        "and PyNaCl (Debian and Ubuntu: python3-argon2 python3-nacl; Fedora: python3-argon2-cffi "
        f"python3-pynacl; Arch: python-argon2_cffi python-pynacl): {e}") from e

VECTORS = json.loads((ROOT / "tools/vectors/vault-primitives/vectors.json").read_text())

MAGIC = b"OFVT"
HEADER_LEN = 53
SLOT_SIZES = (65_536, 262_144, 1_048_576, 4_194_304)
SLOT_INFO = b"Faraday vault v1 slot"
CREATE_INFO = b"Faraday vault create"
LANGUAGES = 10
MAX_SHEET = 16 * 1024


class Refused(Exception):
    """A refusal, named as `faraday_vault::Error` names its kinds."""

    def __init__(self, kind, detail=""):
        super().__init__(f"{kind}: {detail}" if detail else kind)
        self.kind = kind


# ----- primitives ------------------------------------------------------------


def hkdf_sha256(ikm, salt, info, length):
    """RFC 5869 over HMAC-SHA256; an empty salt is 32 zero bytes."""
    prk = hmac.new(salt or bytes(32), ikm, hashlib.sha256).digest()
    out, t, i = b"", b"", 1
    while len(out) < length:
        t = hmac.new(prk, t + info + bytes([i]), hashlib.sha256).digest()
        out += t
        i += 1
    return prk, out[:length]


M32 = 0xFFFFFFFF


def _quarter(s, a, b, c, d):
    s[a] = (s[a] + s[b]) & M32; s[d] ^= s[a]; s[d] = ((s[d] << 16) | (s[d] >> 16)) & M32  # noqa: E702
    s[c] = (s[c] + s[d]) & M32; s[b] ^= s[c]; s[b] = ((s[b] << 12) | (s[b] >> 20)) & M32  # noqa: E702
    s[a] = (s[a] + s[b]) & M32; s[d] ^= s[a]; s[d] = ((s[d] << 8) | (s[d] >> 24)) & M32  # noqa: E702
    s[c] = (s[c] + s[d]) & M32; s[b] ^= s[c]; s[b] = ((s[b] << 7) | (s[b] >> 25)) & M32  # noqa: E702


def chacha20_block(key, counter, nonce):
    """RFC 8439 §2.3: one 64-byte block."""
    state = ([0x61707865, 0x3320646E, 0x79622D32, 0x6B206574] + list(struct.unpack("<8I", key))
             + [counter] + list(struct.unpack("<3I", nonce)))
    w = list(state)
    for _ in range(10):
        _quarter(w, 0, 4, 8, 12); _quarter(w, 1, 5, 9, 13)  # noqa: E702
        _quarter(w, 2, 6, 10, 14); _quarter(w, 3, 7, 11, 15)  # noqa: E702
        _quarter(w, 0, 5, 10, 15); _quarter(w, 1, 6, 11, 12)  # noqa: E702
        _quarter(w, 2, 7, 8, 13); _quarter(w, 3, 4, 9, 14)  # noqa: E702
    return struct.pack("<16I", *[(w[i] + state[i]) & M32 for i in range(16)])


def chacha20_stream(key, length):
    """ChaCha20 with a zero nonce from counter 0. Block 0 is computed
    here; the rest is libsodium's RFC 8439 AEAD over zeros, whose
    ciphertext is the keystream from counter 1 (checked against the
    blocks here in `check_primitives`)."""
    first = chacha20_block(key, 0, bytes(12))
    if length <= 64:
        return first[:length]
    rest = sodium.crypto_aead_chacha20poly1305_ietf_encrypt(bytes(length - 64), None, bytes(12), key)
    return first + rest[:-16]


def argon2id(password, salt, memory_kib, passes, lanes):
    return hash_secret_raw(secret=bytes(password), salt=bytes(salt), time_cost=passes,
                           memory_cost=memory_kib,
                           parallelism=lanes, hash_len=32, type=Type.ID, version=19)


def aead_seal(key, nonce, aad, plain):
    out = sodium.crypto_aead_xchacha20poly1305_ietf_encrypt(bytes(plain), bytes(aad) or None,
                                                            bytes(nonce), bytes(key))
    return out[:-16], out[-16:]


def aead_open(key, nonce, aad, ciphertext, tag):
    """The plaintext, or None when the tag fails."""
    try:
        return sodium.crypto_aead_xchacha20poly1305_ietf_decrypt(
            bytes(ciphertext) + bytes(tag), bytes(aad) or None, bytes(nonce), bytes(key))
    except CryptoError:
        return None


# ----- the slot's plaintext (§7) --------------------------------------------

TEXT = ("text", 0xFFFF)
BYTES = ("bytes",)


def text(n):
    return ("text", n)


def exact(n):
    return ("exact", n)


# Type: {field: (required, repeatable, shape)}, as §7's table states.
# "In each type the first field is required and every other field is
# optional; type 7's field 2, type 9's fields 2, 3 and 4, type 10's
# field 2 and type 14's field 2 are also required." A field a type does
# not list refuses the slot, as an unknown type does (the tables are read
# as complete).
SPEC = {
    1: {1: (True, False, text(64))},
    2: {1: (True, False, ("key",)), 2: (False, False, TEXT), 3: (False, False, exact(1)),
        4: (False, False, TEXT)},
    3: {1: (True, False, TEXT), 2: (False, False, TEXT), 3: (False, False, exact(1))},
    4: {1: (True, False, TEXT)},
    5: {1: (True, False, ("sheet",))},
    6: {n: (n == 1, False, TEXT) for n in range(1, 7)},
    7: {1: (True, False, exact(32)), 2: (True, False, exact(4)), 3: (False, False, exact(32)),
        4: (False, False, exact(4)), 5: (False, True, TEXT), 6: (False, False, exact(4)),
        7: (False, False, BYTES), 8: (False, False, BYTES), 9: (False, False, exact(4))},
    8: {1: (True, False, exact(16)), **{n: (False, False, BYTES) for n in range(2, 8)}},
    9: {1: (True, False, exact(1)), 2: (True, False, exact(32)), 3: (True, False, exact(4)),
        4: (True, False, BYTES), 5: (False, False, exact(1))},
    10: {1: (True, False, exact(32)), 2: (True, True, exact(8))},
    11: {1: (True, False, TEXT), 2: (False, False, TEXT), 3: (False, True, text(128)),
         4: (False, True, TEXT), 5: (False, True, TEXT), 6: (False, True, text(128))},
    # 12 is retired (F99, §7): it was an APK signing key; no build knows
    # it, so a slot holding one is refused like any unknown type.
    13: {1: (True, False, TEXT), 2: (False, False, TEXT), 3: (False, False, exact(1)),
         4: (False, False, exact(1)), 5: (False, False, TEXT)},
    14: {1: (True, False, exact(4)), 2: (True, False, TEXT)},
}



def words_payload_of(p):
    if len(p) != 34:
        return None
    count, lang = p[0], p[1]
    n = count * 4 // 3
    if count not in (12, 15, 18, 21, 24) or lang >= LANGUAGES or any(p[2 + n:]):
        return None
    return {"words": {"language": lang, "entropy": p[2:2 + n].hex()}}


def seed_payload_of(p):
    if len(p) != 65 or not 16 <= p[0] <= 64 or any(p[1 + p[0]:]):
        return None
    return {"seed": p[1:1 + p[0]].hex()}


def sheet_payload_of(p):
    """Three u16-length UTF-8 parts and nothing after; at most 16 KiB,
    read as written."""
    if len(p) > MAX_SHEET:
        return None
    parts, at = [], 0
    for _ in range(3):
        if at + 2 > len(p):
            return None
        n = int.from_bytes(p[at:at + 2], "little")
        at += 2
        if at + n > len(p):
            return None
        try:
            p[at:at + n].decode("utf-8")
        except UnicodeDecodeError:
            return None
        parts.append(p[at:at + n].hex())
        at += n
    return {"sheet": parts} if at == len(p) else None


def payload_of(kind, p):
    if kind == 1:
        return words_payload_of(p)
    if kind == 2:
        return seed_payload_of(p)
    if kind == 4:
        return sheet_payload_of(p)
    return None


def fits(shape, b):
    kind = shape[0]
    if kind == "text":
        if len(b) > shape[1]:
            return False
        try:
            b.decode("utf-8")
            return True
        except UnicodeDecodeError:
            return False
    if kind == "bytes":
        return len(b) <= 0xFFFF
    if kind == "exact":
        return len(b) == shape[1]
    if kind == "key":
        return (words_payload_of(b) if len(b) == 34 else seed_payload_of(b)) is not None
    if kind == "sheet":
        return payload_of(4, b) is not None
    raise AssertionError(shape)


def check_record(kind, fields):
    rules = SPEC.get(kind)
    if rules is None:
        raise Refused("Contents", f"unknown type {kind}")
    for n, b in fields:
        if n not in rules:
            raise Refused("Contents", f"type {kind} has no field {n}")
        if not fits(rules[n][2], b):
            raise Refused("Contents", f"type {kind} field {n}")
    for n, (required, repeat, _) in rules.items():
        count = sum(1 for m, _ in fields if m == n)
        if required and not count:
            raise Refused("Contents", f"type {kind} lacks field {n}")
        if count > 1 and not repeat:
            raise Refused("Contents", f"type {kind} repeats field {n}")


def check_across(records):
    """§7's rules across records: at most one slot label, one passphrase
    record (14) per key, one signed amounts record (10) per
    transaction."""
    if sum(1 for k, _ in records if k == 1) > 1:
        raise Refused("Contents", "two slot labels")
    for kind in (10, 14):
        ids = [dict(fields)[1] for k, fields in records if k == kind]
        if len(set(ids)) != len(ids):
            raise Refused("Contents", f"two type {kind} records for one "
                          + ("key" if kind == 14 else "transaction"))


def encode(records, slot_len):
    """A slot's plaintext: format 1, the records, the end record, zeros."""
    for k, fields in records:
        check_record(k, fields)
    check_across(records)
    out = bytearray([1])
    for k, fields in records:
        body = b"".join(bytes([n]) + len(b).to_bytes(2, "little") + b for n, b in fields)
        out += bytes([k]) + len(body).to_bytes(4, "little") + body
    out.append(0)
    if len(out) > slot_len:
        raise Refused("Full")
    return bytes(out) + bytes(slot_len - len(out))


def decode(plain):
    if not plain or plain[0] != 1:
        raise Refused("Contents", "format")
    at, records = 1, []
    while True:
        if at >= len(plain):
            raise Refused("Contents", "no end record")
        k = plain[at]
        at += 1
        if k == 0:
            break
        if k not in SPEC:
            raise Refused("Contents", f"unknown type {k}")
        if at + 4 > len(plain):
            raise Refused("Contents", "truncated")
        n = int.from_bytes(plain[at:at + 4], "little")
        at += 4
        if at + n > len(plain):
            raise Refused("Contents", "truncated")
        body, at = plain[at:at + n], at + n
        fields, p = [], 0
        while p < len(body):
            if p + 3 > len(body):
                raise Refused("Contents", "a field runs past its record")
            m = int.from_bytes(body[p + 1:p + 3], "little")
            if p + 3 + m > len(body):
                raise Refused("Contents", "a field runs past its record")
            fields.append((body[p], body[p + 3:p + 3 + m]))
            p += 3 + m
        check_record(k, fields)
        records.append((k, fields))
    if any(plain[at:]):
        raise Refused("Contents", "bytes after the end record")
    check_across(records)
    return records


def as_json(records):
    return [[k, [[n, b.hex()] for n, b in fields]] for k, fields in records]


def from_json(records):
    return [(k, [(n, bytes.fromhex(h)) for n, h in fields]) for k, fields in records]


# ----- the file (§1 to §5, §8) ----------------------------------------------


def file_len(slot_len):
    return HEADER_LEN + 4 * (slot_len + 40)


def header_bytes(memory, passes, lanes, slot_len, salt):
    return MAGIC + bytes([1]) + struct.pack("<IIII", memory, passes, lanes, slot_len) + salt


def check_cost(memory, passes, lanes):
    if not 8 * 1024 <= memory <= 4 * 1024 * 1024 or not 1 <= passes <= 10 or lanes != 1:
        raise Refused("Cost")


def read_header(f):
    """§4 step 1: the magic, the version, L, the length, the cost."""
    if len(f) < 4 or f[:4] != MAGIC:
        raise Refused("Magic")
    if len(f) < HEADER_LEN:
        raise Refused("Length")
    if f[4] != 1:
        raise Refused("Version")
    memory, passes, lanes, slot_len = struct.unpack("<IIII", f[5:21])
    if slot_len not in SLOT_SIZES:
        raise Refused("SlotSize")
    if len(f) != file_len(slot_len):
        raise Refused("Length")
    check_cost(memory, passes, lanes)
    return memory, passes, lanes, slot_len


def slot_key(header, passphrase):
    memory, passes, lanes = struct.unpack("<III", header[5:17])
    k = argon2id(passphrase, header[21:53], memory, passes, lanes)
    return hkdf_sha256(k, b"", SLOT_INFO, 32)[1]


def slot_at(slot_len, i):
    return HEADER_LEN + i * (slot_len + 40)


def seal_slot(f, key, i, nonce, plain):
    slot_len = len(plain)
    ct, tag = aead_seal(key, nonce, f[:HEADER_LEN] + bytes([i]), plain)
    at = slot_at(slot_len, i)
    f[at:at + 24 + slot_len + 16] = nonce + ct + tag


def open_slots(f, passphrase):
    """The slot key and every slot it opens."""
    read_header(f)
    slot_len = struct.unpack("<I", f[17:21])[0]
    key = slot_key(f[:HEADER_LEN], passphrase)
    found = []
    for i in range(4):
        at = slot_at(slot_len, i)
        plain = aead_open(key, f[at:at + 24], f[:HEADER_LEN] + bytes([i]),
                          f[at + 24:at + 24 + slot_len], f[at + 24 + slot_len:at + 40 + slot_len])
        if plain is not None:
            found.append((i, plain))
    if len(found) > 1:
        raise Refused("TwoSlots")
    if not found:
        raise Refused("Passphrase")
    return key, found[0]


def ref_open(f, passphrase):
    _, (_, plain) = open_slots(f, passphrase)
    return {"records": as_json(decode(plain))}


def ref_seal(f, passphrase, records, nonce):
    key, (i, _) = open_slots(f, passphrase)
    plain = encode(records, struct.unpack("<I", f[17:21])[0])
    out = bytearray(f)
    seal_slot(out, key, i, nonce, plain)
    return {"file": out.hex()}


def ref_create(slot_len, cost, passphrases, contents, seed):
    memory, passes, lanes = cost
    if slot_len not in SLOT_SIZES:
        raise Refused("SlotSize")
    check_cost(memory, passes, lanes)
    # §8: one to four passphrases, none empty, no two equal.
    if not 1 <= len(passphrases) <= 4 or len(passphrases) != len(contents) \
            or any(not p for p in passphrases):
        raise Refused("Passphrases")
    if len(set(passphrases)) != len(passphrases):
        raise Refused("SamePassphrase")
    plains = [encode(c, slot_len) for c in contents]
    key = hkdf_sha256(seed, b"", CREATE_INFO, 32)[1]
    stream = chacha20_stream(key, 32 + 64 + file_len(slot_len) + 24 * 4)
    at = 0

    def draw(n):
        nonlocal at
        at += n
        return stream[at - n:at]

    salt = draw(32)
    order = [0, 1, 2, 3]
    for i in (3, 2, 1):
        while True:
            b = draw(1)[0]
            if b < 256 - 256 % (i + 1):
                j = b % (i + 1)
                break
        order[i], order[j] = order[j], order[i]
    header = header_bytes(memory, passes, lanes, slot_len, salt)
    f = bytearray(header + draw(file_len(slot_len) - HEADER_LEN))
    for k, p in enumerate(passphrases):
        seal_slot(f, slot_key(header, p), order[k], draw(24), plains[k])
    return {"file": f.hex()}


def refusal(answer):
    """A refusal as its kind: `Contents(Value(2, 1))` and the reference's
    `Refused: Contents: ...` are both `Contents`."""
    if answer[0] == "ok":
        return answer
    reason = answer[1].removeprefix("Refused: ")
    return ("ok", {"refused": reason.split("(")[0].split(":")[0].strip()})


def file_summary(answer):
    """A file answer as its length, header and each slot's SHA-256, so a
    disagreement prints a few lines rather than megabytes."""
    if answer[0] != "ok":
        return refusal(answer)
    f = bytes.fromhex(answer[1]["file"])
    if len(f) < HEADER_LEN + 4 or f[:4] != MAGIC:
        return ("ok", {"len": len(f), "sha256": hashlib.sha256(f).hexdigest()})
    slot_len = struct.unpack("<I", f[17:21])[0]
    return ("ok", {"len": len(f), "header": f[:HEADER_LEN].hex(),
                   "slots": [hashlib.sha256(f[slot_at(slot_len, i):slot_at(slot_len, i + 1)])
                             .hexdigest() for i in range(4)]})


# ----- the area --------------------------------------------------------------


class Vault(Area):
    name = "vault"

    def check_primitives(self):
        """The area's own pieces against their published answers."""
        for v in VECTORS["hkdf_sha256"]:
            prk, okm = hkdf_sha256(*(bytes.fromhex(v[k]) for k in ("ikm", "salt", "info")), v["L"])
            if (prk.hex(), okm.hex()) != (v["prk"], v["okm"]):
                raise HarnessError(f"the area's HKDF fails RFC 5869: {v}")
        for v in VECTORS["chacha20_block"]:
            got = chacha20_block(bytes.fromhex(v["key"]), v["counter"], bytes.fromhex(v["nonce"]))
            if got.hex() != v["keystream"]:
                raise HarnessError(f"the area's ChaCha20 fails RFC 8439 A.1: {v}")
        key = self.randbytes(32)
        stream = chacha20_stream(key, 64 * 5 + 17)
        if stream != b"".join(chacha20_block(key, c, bytes(12)) for c in range(6))[:64 * 5 + 17]:
            raise HarnessError("libsodium's keystream is not ChaCha20's from counter 1")
        v = VECTORS["xchacha20poly1305"]
        ct, tag = aead_seal(*(bytes.fromhex(v[k]) for k in ("key", "nonce", "aad", "plaintext")))
        if (ct.hex(), tag.hex()) != (v["ciphertext"], v["tag"]):
            raise HarnessError("PyNaCl fails draft-irtf-cfrg-xchacha-03 A.3.1")
        a = VECTORS["argon2id"]
        if argon2id(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                    a["passes"], a["lanes"]).hex() != a["key"]:
            raise HarnessError("argon2-cffi fails the known Argon2id answer")

    # Each operation, both sides.

    def argon(self, password, salt, memory, passes, lanes, expected=None):
        return self.compare(
            "vault.argon2id",
            {"password": password.hex(), "salt": salt.hex(), "memory_kib": memory,
             "passes": passes, "lanes": lanes},
            lambda: {"key": argon2id(password, salt, memory, passes, lanes).hex()}, expected)

    def seal(self, key, nonce, aad, plain, expected=None):
        def ref():
            ct, tag = aead_seal(key, nonce, aad, plain)
            return {"ciphertext": ct.hex(), "tag": tag.hex()}
        return self.compare("vault.aead_seal", {"key": key.hex(), "nonce": nonce.hex(),
                                                "aad": aad.hex(), "plaintext": plain.hex()},
                            ref, expected)

    def unseal(self, key, nonce, aad, ct, tag, expected=None):
        def ref():
            plain = aead_open(key, nonce, aad, ct, tag)
            if plain is None:
                raise Refused("tag")
            return {"plaintext": plain.hex()}
        return self.compare("vault.aead_open", {"key": key.hex(), "nonce": nonce.hex(),
                                                "aad": aad.hex(), "ciphertext": ct.hex(),
                                                "tag": tag.hex()}, ref, expected)

    def encode(self, records, slot_len):
        return self.compare("vault.encode", {"contents": as_json(records), "slot_len": slot_len},
                            lambda: {"plain": encode(records, slot_len).hex()},
                            normalize=refusal)

    def decode(self, plain, expected=None):
        return self.compare("vault.decode", {"plain": plain.hex()},
                            lambda: {"records": as_json(decode(plain))}, expected,
                            normalize=_okerr)

    def payload(self, kind, p):
        def ref():
            out = payload_of(kind, p)
            if out is None:
                raise Refused("not that layout")
            return out
        return self.compare("vault.payload_of", {"kind": kind, "payload": p.hex()}, ref)

    def create(self, slot_len, cost, passphrases, contents, seed):
        return self.compare(
            "vault.create",
            {"slot_len": slot_len, "memory_kib": cost[0], "passes": cost[1], "lanes": cost[2],
             "passphrases": [p.hex() for p in passphrases],
             "contents": [as_json(c) for c in contents], "seed": seed.hex()},
            lambda: ref_create(slot_len, cost, passphrases, contents, seed),
            normalize=file_summary)

    def open(self, f, passphrase, expected=None):
        return self.compare("vault.open", {"file": f.hex(), "passphrase": passphrase.hex()},
                            lambda: ref_open(f, passphrase), expected, normalize=refusal)

    def reseal(self, f, passphrase, records, nonce):
        return self.compare("vault.seal", {"file": f.hex(), "passphrase": passphrase.hex(),
                                           "contents": as_json(records), "nonce": nonce.hex()},
                            lambda: ref_seal(f, passphrase, records, nonce),
                            normalize=file_summary)

    # Random inputs.

    def text(self, limit=40):
        rng = self.rng
        alphabet = rng.choice(["abcdefghijklmnopqrstuvwxyz ", "éüßçøñ ", "日本語漢字 ", "🙂🚀 ",
                               "abc\n\t-/"])
        out = b""
        for _ in range(rng.randint(0 if rng.random() < 0.2 or not limit else 1, limit)):
            c = rng.choice(alphabet).encode()
            if len(out) + len(c) > limit:
                break
            out += c
        return out

    def field_value(self, shape):
        rng = self.rng
        kind = shape[0]
        if kind == "text":
            return self.text(min(shape[1], rng.choice([8, 40, 200])))
        if kind == "bytes":
            return self.randbytes(rng.choice([0, 1, 16, 64, rng.randint(0, 600)]))
        if kind == "exact":
            return self.randbytes(shape[1])
        if kind == "key":
            if rng.random() < 0.5:
                count = rng.choice([12, 15, 18, 21, 24])
                n = count * 4 // 3
                return bytes([count, rng.randrange(LANGUAGES)]) + self.randbytes(n) + bytes(32 - n)
            n = rng.randint(16, 64)
            return bytes([n]) + self.randbytes(n) + bytes(64 - n)
        if kind == "sheet":
            parts = [self.text(rng.choice([0, 30, 300])) for _ in range(3)]
            return b"".join(len(p).to_bytes(2, "little") + p for p in parts)
        raise AssertionError(shape)

    def record(self, kind=None):
        """A record §7 allows (and faraday_vault reads alike)."""
        rng = self.rng
        kind = kind or rng.choice(sorted(SPEC))
        fields = []
        for n, (required, repeat, shape) in SPEC[kind].items():
            if required or rng.random() < 0.5:
                for _ in range(rng.randint(2, 3) if repeat and rng.random() < 0.3 else 1):
                    fields.append((n, self.field_value(shape)))
        rng.shuffle(fields)
        return kind, fields

    def records(self, most=6):
        rng = self.rng
        out = [self.record() for _ in range(rng.randint(0, most))]
        labels = [r for r in out if r[0] == 1]
        return [r for r in out if r[0] != 1] + labels[:1]

    def malformed(self, records):
        """The records with one thing wrong by §7's rules, and what was
        done, as a plaintext."""
        rng = self.rng
        plain = bytearray(encode(records, _used(records)))
        how = rng.randrange(10)
        if how == 9:
            # A second passphrase record for a key, or a second signed
            # amounts record for a transaction, among the others.
            kind = rng.choice([10, 14])
            k, fields = self.record(kind)
            twin = self.record(kind)
            twin = (kind, [(n, dict(fields)[1] if n == 1 else b) for n, b in twin[1]])
            records = list(records) + [(k, fields)]
            records.insert(rng.randrange(len(records) + 1), twin)
            return bytes(raw_encode(records)), "rule 9"
        if how == 0 or not records:
            plain[0] = rng.choice([0, 2, 255])
        elif how == 1:
            # A byte anywhere changed.
            plain[rng.randrange(len(plain))] ^= 1 << rng.randrange(8)
        elif how == 2:
            # Cut short, or a byte not zero after the end.
            if rng.random() < 0.5:
                del plain[rng.randrange(1, len(plain)):]
            else:
                plain += bytes(rng.randint(0, 4)) + bytes([rng.randint(1, 255)])
        else:
            records = list(records)
            i = rng.randrange(len(records))
            k, fields = records[i]
            fields = list(fields)
            if how == 3 and fields:
                # A field dropped (perhaps a required one).
                fields.pop(rng.randrange(len(fields)))
            elif how == 4 and fields:
                # A field repeated.
                fields.append(rng.choice(fields))
            elif how == 5:
                # A field the type does not list.
                fields.append((rng.choice([0, 15, 99, 255] + [n for n in range(1, 10)
                                                               if n not in SPEC[k]]), b"x"))
            elif how == 6 and fields:
                # A value one byte longer or shorter.
                j = rng.randrange(len(fields))
                n, b = fields[j]
                fields[j] = (n, b + b"\0" if rng.random() < 0.5 or not b else b[:-1])
            elif how == 7 and fields:
                # Text that is not UTF-8.
                j = rng.randrange(len(fields))
                n, b = fields[j]
                bad = rng.choice([b"\xff", b"\xc3", b"\xed\xa0\x80", b"\xf4\x90\x80\x80",
                                  b"\xc0\xaf", b"\xe2\x82"])
                at = rng.randint(0, len(b))
                fields[j] = (n, b[:at] + bad + b[at:])
            elif rng.random() < 0.5:
                # An unknown type.
                k = rng.choice([12, 15, 16, 100, 255])
            else:
                # A second slot label.
                records.append((1, [(1, b"Two")]))
            records[i] = (k, fields)
            return bytes(raw_encode(records)), f"rule {how}"
        return bytes(plain), f"bytes {how}"

    def payload_case(self):
        rng = self.rng
        kind = rng.choice([1, 2, 4])
        if kind == 1:
            lang = rng.randrange(LANGUAGES)
            count = rng.choice([12, 15, 18, 21, 24])
            entropy = self.randbytes(count * 4 // 3)
            p = self.need(self.compare(
                "vault.words_payload", {"language": lang, "entropy": entropy.hex()},
                lambda: {"payload": (bytes([count, lang]) + entropy
                                     + bytes(32 - len(entropy))).hex()}), "words")["payload"]
        elif kind == 2:
            seed = self.randbytes(rng.choice([rng.randint(16, 64), rng.randint(0, 80)]))

            def ref():
                if not 16 <= len(seed) <= 64:
                    raise Refused("not 16 to 64 bytes")
                return {"payload": (bytes([len(seed)]) + seed + bytes(64 - len(seed))).hex()}
            ans = self.compare("vault.seed_payload", {"seed": seed.hex()}, ref)
            if ans[0] != "ok":
                return
            p = ans[1]["payload"]
        else:
            parts = [self.text(rng.choice([0, 20, 400])) for _ in range(3)]
            if rng.random() < 0.05:
                parts[0] = self.randbytes(rng.choice([16380, 16384, 20000, 65536]))

            def ref():
                total = 6 + sum(len(x) for x in parts)
                if total > MAX_SHEET or any(len(x) > 0xFFFF for x in parts):
                    raise Refused("too long")
                return {"payload": b"".join(len(x).to_bytes(2, "little") + x
                                            for x in parts).hex()}
            ans = self.compare("vault.sheet_payload", {"descriptor": parts[0].hex(),
                                                       "name": parts[1].hex(),
                                                       "note": parts[2].hex()}, ref)
            if ans[0] != "ok":
                return
            p = ans[1]["payload"]
        p = bytearray(bytes.fromhex(p))
        self.payload(kind, bytes(p))
        # The same bytes read as another layout, and malformed.
        self.payload(rng.choice([k for k in (0, 1, 2, 3, 4, 5) if k != kind]), bytes(p))
        how = rng.randrange(5)
        if how == 0 and p:
            p[rng.randrange(len(p))] ^= 1 << rng.randrange(8)
        elif how == 1:
            p += bytes([rng.randrange(256)])
        elif how == 2 and p:
            del p[-1]
        elif how == 3 and kind in (1, 2):
            p[0] = rng.choice([0, 11, 13, 25, 15, 16, 64, 65, 255])
        elif how == 4 and kind == 1:
            p[1] = rng.choice([10, 11, 255])
        self.payload(kind, bytes(p))

    def big_sheet(self):
        """A sheet payload past 16 KiB, which §7 caps: refused by both, as
        a payload and inside a record."""
        p = (20000).to_bytes(2, "little") + b"d" * 20000 + bytes(4)
        self.payload(4, p)
        self.decode(bytes(raw_encode([(5, [(1, p)])])), ("err", None))

    def vault_case(self, slot_len=65536):
        rng = self.rng
        count = rng.randint(1, 4)
        passphrases = []
        while len(passphrases) < count:
            p = self.text(rng.choice([4, 30])) or b"p"
            if rng.random() < 0.2:
                p = self.randbytes(rng.randint(1, 40))
            if p not in passphrases:
                passphrases.append(p)
        contents = [self.records(4) for _ in range(count)]
        cost = (8192, rng.choice([1, 1, 2]), 1)
        seed = self.randbytes(32)
        f = bytes.fromhex(self.need(self.create(slot_len, cost, passphrases, contents, seed),
                                    "create")["file"])
        for p, c in zip(passphrases, contents):
            got = self.need(self.open(f, p), "open")
            if got != {"records": as_json(c)}:
                raise AssertionError("a passphrase did not open its own contents")
        self.open(f, rng.choice([b"", b"wrong", passphrases[0] + b" "]))
        # A seal of new contents into one slot, then both open it.
        p = rng.choice(passphrases)
        new = self.records(4)
        sealed = bytes.fromhex(self.need(self.reseal(f, p, new, self.randbytes(24)), "seal")["file"])
        self.open(sealed, p)
        self.open(sealed, rng.choice(passphrases))
        # One fault in the file.
        self.open(self.faulty(f), rng.choice(passphrases))
        # Contents §7 refuses, sealed under the right key by the reference.
        bad, _ = self.malformed(self.records(3) or [self.record()])
        bad += bytes(max(0, slot_len - len(bad)))
        if len(bad) == slot_len:
            self.open(self.with_plain(f, p, bad), p)

    def faulty(self, f):
        rng = self.rng
        f = bytearray(f)
        how = rng.randrange(9)
        if how == 0:
            f[rng.randrange(4)] ^= 0x20
        elif how == 1:
            f[4] = rng.choice([0, 2, 255])
        elif how == 2:
            f[17:21] = struct.pack("<I", rng.choice([0, 65535, 65537, 131072, 8388608]))
        elif how == 3:
            del f[rng.choice([10, 52, 53, len(f) - 1]):]
        elif how == 4:
            f += bytes(rng.randint(1, 40))
        elif how == 5:
            f[5:17] = struct.pack("<III", *rng.choice([(8191, 1, 1), (4194305, 1, 1), (8192, 0, 1),
                                                       (8192, 11, 1), (8192, 1, 2), (8192, 1, 0)]))
        elif how == 6:
            # A header byte that is not checked but is associated data.
            f[rng.randrange(21, HEADER_LEN)] ^= 1
        else:
            f[rng.randrange(HEADER_LEN, len(f))] ^= 1 << rng.randrange(8)
        return bytes(f)

    def with_plain(self, f, passphrase, plain):
        key, (i, _) = open_slots(f, passphrase)
        out = bytearray(f)
        seal_slot(out, key, i, self.randbytes(24), plain)
        return bytes(out)

    def two_slots(self):
        """A file with two slots under one key, which no writer of this
        format makes: both refuse it."""
        f = bytearray(bytes.fromhex(ref_create(65536, (8192, 1, 1), [b"a"], [[]], bytes(32))["file"]))
        key, (i, plain) = open_slots(f, b"a")
        seal_slot(f, key, (i + 1) % 4, bytes(24), plain)
        self.open(bytes(f), b"a")

    def published(self):
        self.check_primitives()
        v = VECTORS["xchacha20poly1305"]
        args = [bytes.fromhex(v[k]) for k in ("key", "nonce", "aad")]
        self.seal(*args, bytes.fromhex(v["plaintext"]), ("ok", {"ciphertext": v["ciphertext"],
                                                                "tag": v["tag"]}))
        self.unseal(*args, bytes.fromhex(v["ciphertext"]), bytes.fromhex(v["tag"]),
                    ("ok", {"plaintext": v["plaintext"]}))
        a = VECTORS["argon2id"]
        self.argon(bytes.fromhex(a["password"]), bytes.fromhex(a["salt"]), a["memory_kib"],
                   a["passes"], a["lanes"], ("ok", {"key": a["key"]}))
        for name, passphrases in [("vault-one.ofv", ["correct horse battery staple"]),
                                  ("vault-three.ofv", ["first passphrase", "second passphrase",
                                                       "third passphrase"])]:
            f = (ROOT / "tools/vectors/vault" / name).read_bytes()
            for p in passphrases:
                self.open(f, p.encode())
            self.open(f, b"fourth passphrase")

    def settled(self):
        """The rules the owner settled in §7 and §8 (2026-10-23), which
        both now hold: an empty passphrase refused at create, alone or
        among others; the retired type 12 refused, read or written; a
        GPG key without field 2 and a signing round without field 2, 3
        or 4 refused; a sheet past 16 KiB refused; two passphrase records
        for one key and two signed amounts records for one transaction
        refused; and the slot vectors of `tools/vectors/vault/slots.json`
        read as the file says."""
        for passphrases in ([b""], [b"one", b""]):
            self.create(65536, (8192, 1, 1), passphrases, [[] for _ in passphrases], bytes(32))
        apk = [(12, [(1, b"\x30\x00"), (2, b"\x30\x00"), (3, b"app.example")])]
        self.decode(bytes(raw_encode(apk)), ("err", None))
        self.encode(apk, _used(apk))
        for kind, fields in [(7, [(1, bytes(32))]), (9, [(1, b"\x01")]),
                             (9, [(1, b"\x02"), (2, bytes(32)), (3, bytes(4))])]:
            self.decode(bytes(raw_encode([(kind, fields)])), ("err", None))
            self.encode([(kind, fields)], _used([(kind, fields)]))
        self.big_sheet()
        for twins in ([(14, [(1, b"fpfp"), (2, b"a")]), (14, [(1, b"fpfp"), (2, b"b")])],
                      [(10, [(1, bytes(32)), (2, bytes(8))]),
                       (10, [(1, bytes(32)), (2, bytes(8)), (2, bytes(8))])]):
            self.decode(bytes(raw_encode(twins)), ("err", None))
            self.encode(twins, _used(twins))
        for case in json.loads((ROOT / "tools/vectors/vault/slots.json").read_text())["cases"]:
            want = ("ok", None) if case["expect"] == "ok" else ("err", None)
            got = self.decode(bytes.fromhex(case["plain"]), None)
            if _okerr(got)[0] != want[0]:
                raise Mismatch(self.name, "vault.decode", {"case": case["case"]}, got, want,
                               "both differ from tools/vectors/vault/slots.json")

    def run(self):
        self.published()
        self.settled()
        rng = self.rng
        for _ in range(self.cases(40)):
            lanes = rng.choice([1, 1, 2, 4])
            self.argon(self.randbytes(rng.randint(0, 64)),
                       self.randbytes(rng.choice([16, 32, rng.randint(0, 40)])),
                       rng.choice([8 * lanes, rng.randint(8 * lanes - 2, 300)]),
                       rng.choice([1, 2, 3, rng.randint(0, 4)]), lanes if rng.random() < 0.9 else 0)
        for _ in range(self.cases(500)):
            key, nonce = self.randbytes(32), self.randbytes(24)
            aad = self.randbytes(rng.choice([0, 13, 54, rng.randint(0, 200)]))
            plain = self.randbytes(rng.choice([0, 1, 63, 64, 65, rng.randint(0, 1000),
                                               rng.randint(0, 70000) if rng.random() < 0.02 else 7]))
            sealed = self.need(self.seal(key, nonce, aad, plain), "seal")
            ct, tag = bytes.fromhex(sealed["ciphertext"]), bytes.fromhex(sealed["tag"])
            self.unseal(key, nonce, aad, ct, tag)
            which = rng.randrange(4)
            ct, tag, nonce, aad = [bytearray(x) for x in (ct, tag, nonce, aad)]
            target = [ct, tag, nonce, aad][which]
            if target:
                target[rng.randrange(len(target))] ^= 1 << rng.randrange(8)
            else:
                aad.append(0)
            self.unseal(key, bytes(nonce), bytes(aad), bytes(ct), bytes(tag))
        for _ in range(self.cases(1500)):
            self.payload_case()
        for _ in range(self.cases(1500)):
            records = self.records()
            used = _used(records)
            slot_len = used + rng.choice([0, 0, 1, 50]) - (1 if rng.random() < 0.1 else 0)
            got = self.encode(records, slot_len)
            if got[0] == "ok" and "plain" in got[1]:
                self.decode(bytes.fromhex(got[1]["plain"]))
            plain, _ = self.malformed(records)
            self.decode(plain)
        self.two_slots()
        for size in SLOT_SIZES[1:3]:
            self.vault_case(size)
        for _ in range(self.cases(25)):
            self.vault_case()


def _used(records):
    return 2 + sum(5 + sum(3 + len(b) for _, b in fields) for _, fields in records)


def raw_encode(records):
    """Records as bytes with no rule checked, so a malformed one is
    written as asked."""
    out = bytearray([1])
    for k, fields in records:
        body = b"".join(bytes([n]) + len(b).to_bytes(2, "little") + b for n, b in fields)
        out += bytes([k]) + len(body).to_bytes(4, "little") + body
    out.append(0)
    return out


def _okerr(answer):
    """A slot's reading compared as its records, or as a refusal: which
    of §7's faults is met first is the implementation's order."""
    return answer if answer[0] == "ok" else ("err", None)
