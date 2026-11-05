"""The QR vault (`faraday_vault::qr`, `docs/VAULT.md` §1.2 and §2.1), against
three programs that know nothing of this repository, each judging its own
part: Sequoia's `sq` (the distribution's package, run as a program, over
Nettle) judges the primitives except Argon2, which Sequoia takes from the
RustCrypto `argon2` crate on every backend, the app's lineage; rPGP
(`tools/crosscheck/rpgp/`, a program of its own outside the workspace) runs
RustCrypto, the app's lineage, and judges the format only (F193 point 1);
PyCA cryptography over OpenSSL (the area, below) judges the primitives,
Argon2id, HKDF-SHA-256 and AES-256-OCB, among them Argon2 over each
message's own S2K fields.

- Faraday seals an item of each kind of §2.1; `sq decrypt`, rPGP and the
  area's PyCA judge open the message to the item's QR TOML, padded to 226
  bytes, which the area writes here from §2.1 (`qr_toml`), and CPython's
  `tomllib` reads each opened item beside the comparison. The PyCA judge
  reads a message strictly (SKESK v6 and SEIPD v2, AES-256, OCB, Argon2 S2K,
  396 bytes, chunk size octet 10, 3 passes, 4 lanes, §1.2) and refuses any
  other, so it also checks that the seal is a QR vault and not something `sq`
  and rPGP merely read; a byte flipped in each field of a good vector is
  refused by it. The framing is the area's own Python, written here
  from §1.2 and RFC 9580 by Faraday's authors; only the primitives under it
  are another program's.
- rPGP (SKESK v6 and SEIPD v2, AES-256, OCB, Argon2 at 3 passes and 4
  lanes, the memory this area chooses) and the PyCA judge seal the area's
  QR TOML; Faraday opens each message to the same item, which gives the open
  path a primitive judge. `sq` 1.3.1 cannot seal one: for a password it
  writes a SKESK v4 and a SEIPD v1 under either `--profile`, and the area
  asserts that Faraday and the PyCA judge refuse what it writes.
- Faraday's writer is deterministic given its random bytes, and the PyCA
  judge's writer splits the same bytes the same way: the two messages are
  compared byte for byte, which catches a constant session key or a reused
  salt that no reader would notice.
- Each memory step of §3 (exponents 16 to 20) is sealed in a run at scale 1;
  a shorter run seals the first steps. A passphrase in NFD is sealed by
  Faraday and opened under its NFC form, and sealed under NFC and opened by
  Faraday under NFD (§3).
- The repository's own vault vectors (`tools/vectors/vault/qr/`) opened by
  `sq`, rPGP and the PyCA judge to the items `tools/vectors/vault/qr/README.md`
  names; items at the size limits; a 15-word seed and a document of two keys
  sealed by rPGP, which Faraday refuses.

OpenPGP.js, which `docs/VAULT.md` §10 names, is not run here.
"""

import json
import os
import subprocess
import tempfile
import tomllib
import unicodedata

import judges
from common import ROOT, Area, HarnessError, Mismatch, use_reference

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

use_reference("codex32", "shims")
use_reference("codex32")
from codex32.bip93 import encode as codex32_encode  # noqa: E402

try:
    import cryptography
    from cryptography.exceptions import InvalidTag
    from cryptography.hazmat.backends.openssl import backend as openssl_backend
    from cryptography.hazmat.primitives import hashes
    from cryptography.hazmat.primitives.ciphers.aead import AESOCB3
    from cryptography.hazmat.primitives.kdf.argon2 import Argon2id
    from cryptography.hazmat.primitives.kdf.hkdf import HKDF
except ImportError as e:  # pragma: no cover - the machine's packages
    raise HarnessError(
        "the qrvault area needs the distribution's PyCA cryptography, 44 or later, over OpenSSL"
        " 3.2 or later (Debian and Ubuntu: python3-cryptography; Fedora: python3-cryptography;"
        f" Arch: python-cryptography): {e}") from e

QR_LEN = 226
VECTORS = ROOT / "tools/vectors/vault/qr"
PHRASE = "correct horse battery staple"
TWELVE = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
TWENTY_FOUR = " ".join(["zoo"] * 23 + ["vote"])
SHARE = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn"
BECH32 = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
SQ_TIMEOUT = 120

# The vault vectors and what each holds (their README).
PUBLISHED = [
    ("qr-seed-12.pgp", PHRASE, {"kind": "words", "text": TWELVE}),
    ("qr-seed-24.pgp", PHRASE, {"kind": "words", "text": TWENTY_FOUR}),
    ("qr-codex32.pgp", PHRASE, {"kind": "codex32", "text": SHARE, "for": "fab6868a"}),
    ("qr-passphrase.pgp", PHRASE, {"kind": "passphrase", "text": "TREZOR", "for": "73c5da0a"}),
    ("qr-note.pgp", PHRASE, {"kind": "note", "text": "The safe's code is in the drawer."}),
    ("qr-empty.pgp", PHRASE, {"kind": "note", "text": ""}),
    ("qr-nfd.pgp", "café", {"kind": "note", "text": "café"}),
]
REFUSED = ["refused-skesk4-seipd1.pgp", "refused-second-skesk.pgp",
           "refused-compressed.pgp", "refused-trailing.pgp"]


def toml_text(s):
    """A TOML basic string as §2.1 writes it."""
    out = ['"']
    for ch in s:
        if ch in '"\\':
            out.append("\\" + ch)
        elif ch in "\n\t\r\b\f":
            out.append({"\n": "\\n", "\t": "\\t", "\r": "\\r", "\b": "\\b", "\f": "\\f"}[ch])
        elif ord(ch) < 0x20 or ord(ch) == 0x7F:
            out.append("\\u%04x" % ord(ch))
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def qr_toml(item):
    """The 226 bytes inside the message (§2.1): the keys, then line feeds;
    None where the item does not fit."""
    kind = item["kind"]
    if kind == "words":
        text = "words = %s\n" % toml_text(item["text"])
    elif kind == "codex32":
        text = "codex32 = %s\nfor = %s\n" % (toml_text(item["text"]), toml_text(item["for"]))
    elif kind == "passphrase":
        text = "passphrase = %s\nfor = %s\n" % (toml_text(item["text"]), toml_text(item["for"]))
    elif kind == "note":
        text = "note = %s\n" % toml_text(item["text"]) if item["text"] else ""
    else:
        raise ValueError(kind)
    return padded(text)


def padded(text):
    """The text as bytes, padded with line feeds to 226; None where it is longer."""
    data = text.encode()
    if len(data) > QR_LEN:
        return None
    return data + b"\n" * (QR_LEN - len(data))


# -- the PyCA judge: OpenSSL's Argon2id, HKDF-SHA-256 and AES-256-OCB under
# the OpenPGP framing of §1.2, written here (the framing is not another
# program's; the primitives are).

SKESK_HEAD = b"\xc3\x58"          # new format, tag 3, 88 bytes
SEIPD_HEAD = b"\xd2\xc0\x6f"      # new format, tag 18, 303 bytes
LITERAL_HEAD = b"\xcb\xc0\x28" + b"b\x00\x00\x00\x00\x00"   # tag 11, 232 bytes: 'b', no name, date 0
SKESK_INFO = b"\xc3\x06\x09\x02"
SEIPD_INFO = b"\xd2\x02\x09\x02\x0a"
RANDOM_LEN = 16 + 15 + 32 + 32     # the S2K salt, the IV, the session key, the SEIPD salt
EXPONENTS = range(16, 21)          # 64 MiB to 1 GiB (§3's writer steps)
VECTOR_EXPONENT = 16               # what tools/vectors/vault/qr/README.md states
PYCA_CHECKED = False
PYCA_WHY = ""


class Refused(Exception):
    pass


def pyca_argon2id(password, salt, exponent, passes=3, lanes=4):
    return Argon2id(salt=bytes(salt), length=32, iterations=passes, lanes=lanes,
                    memory_cost=1 << exponent).derive(bytes(password))


def pyca_hkdf(ikm, salt, info, length):
    return HKDF(algorithm=hashes.SHA256(), length=length, salt=salt, info=info).derive(ikm)


def check_pyca():
    """Once in each process: PyCA 44 or later over OpenSSL 3.2 or later, and each primitive
    right on a published answer (the repository's Argon2id known answer, RFC 5869 test case
    1, RFC 7253's first vector)."""
    global PYCA_CHECKED
    if PYCA_CHECKED:
        return
    ssl = openssl_backend.openssl_version_number()
    if int(cryptography.__version__.split(".")[0]) < 44 or ssl < 0x30200000:
        raise HarnessError(
            "the qrvault area needs PyCA cryptography 44 or later over OpenSSL 3.2 or later, "
            f"found {cryptography.__version__} over {openssl_backend.openssl_version_text()}")
    a = json.loads((ROOT / "tools/vectors/vault-primitives/vectors.json").read_text())["argon2id"]
    if Argon2id(salt=bytes.fromhex(a["salt"]), length=32, iterations=a["passes"], lanes=a["lanes"],
                memory_cost=a["memory_kib"]).derive(bytes.fromhex(a["password"])).hex() != a["key"]:
        raise HarnessError("PyCA's Argon2id fails the known Argon2id answer")
    okm = pyca_hkdf(bytes([0x0b] * 22), bytes(range(13)), bytes(range(0xf0, 0xfa)), 42)
    if okm.hex() != ("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf"
                     "34007208d5b887185865"):
        raise HarnessError("PyCA's HKDF-SHA-256 fails RFC 5869 test case 1")
    tag = AESOCB3(bytes(range(16))).encrypt(bytes.fromhex("bbaa99887766554433221100"), b"", b"")
    if tag.hex() != "785407bfffc8ad9edcc5520ac9111ee6":
        raise HarnessError("PyCA's AES-OCB fails RFC 7253's first vector")
    PYCA_CHECKED = True


def pyca_parse(data):
    """The fields of a QR vault message, or Refused: exactly SKESK v6 and SEIPD v2, AES-256,
    OCB, the Argon2 S2K at 3 passes and 4 lanes, 396 bytes, chunk size octet 10 (§1.2)."""
    if len(data) != 396:
        raise Refused("not 396 bytes")
    if data[:2] != SKESK_HEAD:
        raise Refused("not a SKESK v6 packet header")
    b = data[2:90]
    if b[:6] != bytes([6, 38, 9, 2, 20, 4]):
        raise Refused("not version 6, AES-256, OCB and the Argon2 S2K")
    salt, passes, lanes, exponent = b[6:22], b[22], b[23], b[24]
    if (passes, lanes) != (3, 4) or exponent not in EXPONENTS:
        raise Refused(f"cost {passes} passes, {lanes} lanes, exponent {exponent}")
    if data[90:93] != SEIPD_HEAD:
        raise Refused("not a SEIPD packet header of 303 bytes")
    body = data[93:]
    if body[:4] != bytes([2, 9, 2, 10]):
        raise Refused("not SEIPD v2, AES-256, OCB and chunk size octet 10")
    return {"salt": salt, "exponent": exponent, "iv": b[25:40], "wrapped": b[40:88],
            "seipd_salt": body[4:36], "chunk": body[36:287], "final": body[287:303]}


def pyca_open_checked(passphrase, data):
    """The QR TOML of a message under the passphrase, or Refused or InvalidTag."""
    check_pyca()
    m = pyca_parse(data)
    kek = pyca_hkdf(pyca_argon2id(passphrase.encode(), m["salt"], m["exponent"]), None,
                    SKESK_INFO, 32)
    session = AESOCB3(kek).decrypt(m["iv"], m["wrapped"], SKESK_INFO)
    okm = pyca_hkdf(session, m["seipd_salt"], SEIPD_INFO, 39)
    aead, prefix = AESOCB3(okm[:32]), okm[32:]
    plain = aead.decrypt(prefix + bytes(8), m["chunk"], SEIPD_INFO)
    aead.decrypt(prefix + (1).to_bytes(8, "big"), m["final"],
                 SEIPD_INFO + len(plain).to_bytes(8, "big"))
    if len(plain) != 235 or plain[:9] != LITERAL_HEAD:
        raise Refused("not one literal data packet of format b, no name, date 0, 226 bytes")
    return plain[9:], m["exponent"]


def pyca_open(passphrase, data):
    """(the QR TOML, the S2K's exponent) the PyCA judge finds, or None (the reason in
    PYCA_WHY)."""
    global PYCA_WHY
    try:
        return pyca_open_checked(passphrase, data)
    except Refused as e:
        PYCA_WHY = str(e)
    except InvalidTag:
        PYCA_WHY = "an authentication tag fails"
    return None


def pyca_seal(passphrase, exponent, plain, random, literal_head=LITERAL_HEAD):
    """The 396-byte message of a 226-byte QR TOML, from RANDOM_LEN random bytes."""
    check_pyca()
    assert len(plain) == QR_LEN and len(random) == RANDOM_LEN
    salt, iv, session, seipd_salt = (random[:16], random[16:31], random[31:63], random[63:])
    kek = pyca_hkdf(pyca_argon2id(passphrase.encode(), salt, exponent), None, SKESK_INFO, 32)
    wrapped = AESOCB3(kek).encrypt(iv, session, SKESK_INFO)
    okm = pyca_hkdf(session, seipd_salt, SEIPD_INFO, 39)
    aead, prefix = AESOCB3(okm[:32]), okm[32:]
    literal = literal_head + plain
    chunk = aead.encrypt(prefix + bytes(8), literal, SEIPD_INFO)
    final = aead.encrypt(prefix + (1).to_bytes(8, "big"), b"",
                         SEIPD_INFO + len(literal).to_bytes(8, "big"))
    return (SKESK_HEAD + bytes([6, 38, 9, 2, 20, 4]) + salt + bytes([3, 4, exponent]) + iv
            + wrapped + SEIPD_HEAD + bytes([2, 9, 2, 10]) + seipd_salt + chunk + final)


def packet_heads(data):
    """(tag, first body octet) of each packet from the front of `data`, as far as the
    lengths are not partial; None where the framing does not parse."""
    out, i = [], 0
    while i < len(data):
        head = data[i]
        i += 1
        if not head & 0x80:
            return None
        try:
            if head & 0x40:
                tag, first = head & 0x3F, data[i]
                if first < 192:
                    size, used = first, 1
                elif first < 224:
                    size, used = ((first - 192) << 8) + data[i + 1] + 192, 2
                elif first == 255:
                    size, used = int.from_bytes(data[i + 1:i + 5], "big"), 5
                else:
                    out.append((tag, data[i + 1]))
                    return out
            else:
                tag, kind = (head >> 2) & 0xF, head & 3
                used = (1, 2, 4, 0)[kind]
                size = int.from_bytes(data[i:i + used], "big") if used else len(data) - i
            out.append((tag, data[i + used]))
        except IndexError:
            return None
        i += used + size
    return out


def toml_of(item):
    """What a TOML reader finds in an item's QR TOML."""
    k = item["kind"]
    if k == "words":
        return {"words": item["text"]}
    if k == "codex32":
        return {"codex32": item["text"], "for": item["for"]}
    if k == "passphrase":
        return {"passphrase": item["text"], "for": item["for"]}
    return {"note": item["text"]} if item["text"] else {}


def tool_path(env, name):
    return os.environ.get(env) or str(ROOT / "out/crosscheck" / name)


def run(argv, stdin=b"", cwd=None):
    """A program that must never wait on a terminal: input from a pipe, a
    session of its own (Debian's `sq` asks /dev/tty for a password it was
    not given), a time limit."""
    try:
        p = subprocess.run(argv, input=stdin, capture_output=True, cwd=cwd,
                           start_new_session=True, timeout=SQ_TIMEOUT, check=False)
    except FileNotFoundError as e:
        raise HarnessError(f"the qrvault area needs {argv[0]}: {e}") from e
    return p.returncode, p.stdout, p.stderr


def sq(home, *args, stdin=b""):
    exe = judges.find("sq")
    if not exe:
        raise HarnessError("the qrvault area needs sq (package sq on Debian or Ubuntu)")
    return run([exe, "--home", home, "--cert-store=none", "--key-store=none", *args], stdin)


def sq_open(passphrase, data):
    """What `sq decrypt` finds under the passphrase, or None."""
    with tempfile.TemporaryDirectory() as d:
        for name, content in (("pw", passphrase.encode()), ("m.pgp", data)):
            with open(os.path.join(d, name), "wb") as f:
                f.write(content)
        code, out, _ = sq(os.path.join(d, "h"), "decrypt", "--password-file",
                          os.path.join(d, "pw"), "--signatures", "0", os.path.join(d, "m.pgp"))
    return out if code == 0 else None


def sq_seal(passphrase, plain, profile="rfc9580"):
    """What `sq encrypt` makes, or None."""
    with tempfile.TemporaryDirectory() as d:
        for name, content in (("pw", passphrase.encode()), ("p", plain)):
            with open(os.path.join(d, name), "wb") as f:
                f.write(content)
        extra = ["--profile", profile] if profile else []
        code, out, _ = sq(os.path.join(d, "h"), "encrypt", *extra, "--without-signature",
                          "--with-password-file", os.path.join(d, "pw"), "--binary",
                          os.path.join(d, "p"))
    return out if code == 0 else None


def rpgp(*args):
    code, out, _ = run([tool_path("FARADAY_RPGP_REF", "rpgp-ref"), *args])
    line = out.decode().strip()
    return bytes.fromhex(line) if code == 0 else None


def rpgp_open(passphrase, data):
    return rpgp("open", passphrase.encode().hex(), data.hex())


def rpgp_seal(passphrase, exponent, plain, chunk=10):
    """rPGP's message; `chunk` is the SEIPD's chunk size octet (10 is 64 KiB, 0 is 64 bytes)."""
    return rpgp("seal", passphrase.encode().hex(), str(exponent), str(chunk), plain.hex())


class QrVault(Area):
    name = "qrvault"
    faraday = "`faraday_vault::qr` (the QR vault sealed and opened, with `faraday_pgp::message`)"
    reference = (
        "Sequoia's `sq`, the distribution's package, over Nettle (judges the primitives except "
        "Argon2, which it shares with the app's lineage); rPGP (`tools/crosscheck/rpgp/`, a "
        "program outside the workspace, on RustCrypto like the app: judges the format only); "
        "PyCA cryptography's Argon2id, HKDF-SHA-256 and AES-256-OCB (OpenSSL's), which judge "
        "the primitives, under the OpenPGP framing written in the area from RFC 9580 and "
        "`docs/VAULT.md` §1.2 (Faraday's second version, not another program's); the QR TOML "
        "`qr_toml` written in the area from §2.1, each opened item also read by CPython's "
        "`tomllib`"
    )
    reference_checked = (
        "`sq` is the distribution's package, built over Nettle (the run stops otherwise, "
        "`judges.py`), its version written into the stamp; rPGP is built from the crates.io "
        "release `tools/crosscheck/rpgp/Cargo.lock` names; PyCA must be 44 or later over "
        "OpenSSL 3.2 or later or the area stops, and its Argon2id, HKDF-SHA-256 and AES-OCB "
        "are checked against the repository's Argon2id known answer, RFC 5869 test case 1 and "
        "RFC 7253's first vector first; `sq` and rPGP open the repository's vault vectors, "
        "and so does the PyCA judge, which reads strictly and refuses anything but a QR vault"
    )
    vectors = (
        "the seven vaults of `tools/vectors/vault/qr/` opened by `sq`, rPGP and the PyCA judge "
        "to the item each holds and by Faraday; the four refused messages refused by Faraday "
        "and by the PyCA judge; the PyCA judge refuses a vector with a byte changed in each "
        "field"
    )
    corpus = (
        "an item of every kind of §2.1 (12 and 24 words, a codex32 share, a passphrase, "
        "a note with markup, control characters and several scripts, the empty vault) "
        "sealed by Faraday and opened by `sq`, rPGP, the PyCA judge and Faraday, and sealed by "
        "rPGP and by the PyCA judge (memory 2^13 to 2^16 KiB) and opened by Faraday, each "
        "opened item read by `tomllib`; each memory step of §3 (2^16 to 2^20 KiB) sealed by "
        "Faraday at scale 1, each message equal byte for byte to the PyCA judge's seal of the "
        "same random bytes; items at the size limits (a note of 216 and 217 bytes, a "
        "passphrase of 193 and 194, a 24-word seed of the longest words); a passphrase in NFD "
        "or NFC either way; a wrong passphrase; a 15-word seed and a document of two keys "
        "sealed by rPGP, refused by Faraday; the messages `sq` writes, refused by Faraday and "
        "the PyCA judge; two items sealed by rPGP in chunks of 64 bytes (chunk size octet 0), "
        "opened by Faraday"
    )
    differences = [
        (
            "QR vault: `sq` 1.3.1 writes a SKESK v4 and a SEIPD v1 for a password under "
            "either `--profile`, never the SKESK v6 and SEIPD v2 of RFC 9580, so `sq` "
            "cannot seal a QR vault; rPGP and the PyCA judge seal the other direction, and "
            "the area asserts that Faraday and the PyCA judge refuse what `sq` writes. The "
            "PyCA judge's framing is the area's own Python; OpenPGP.js is not run."
        ),
    ]
    not_compared = (
        "OpenPGP.js, which `docs/VAULT.md` §10 names, is not run; `sq` does not seal a QR "
        "vault (below); rPGP judges the format only. §1.2's wider reading, which only "
        "Faraday's reader is held to, is not compared with a judge: any literal date, padding "
        "packets after the SEIPD packet and the literal data, and every length form RFC 9580 "
        "allows; of the chunk size octets only 0 (rPGP seals several chunks, Faraday opens "
        "them) and 10 are run"
    )

    def check_tools(self):
        for exe in ("rpgp-ref",):
            if not os.path.exists(tool_path("FARADAY_RPGP_REF", exe)):
                raise HarnessError("the qrvault area needs rPGP's program, out/crosscheck/"
                                   "rpgp-ref (run.sh builds it from tools/crosscheck/rpgp)")

    # -- one direction and the other

    def faraday_seal(self, passphrase, item, random, exponent=16):
        got = self.rust.call("qrvault.seal", passphrase=passphrase.encode().hex(),
                             exponent=exponent,
                             random=random.hex(), item=item)
        self.compared += 1
        want = qr_toml(item)
        if want is None:
            if got[0] == "ok":
                raise Mismatch(self.name, "qrvault.seal", item, got, ("err", None),
                               "Faraday sealed an item that does not fit")
            return None
        if got[0] != "ok":
            raise Mismatch(self.name, "qrvault.seal", item, got, ("ok", "a message"),
                           "Faraday refused an item that fits")
        return bytes.fromhex(got[1]["file"])

    def faraday_open(self, passphrase, data):
        got = self.rust.call("qrvault.open", passphrase=passphrase.encode().hex(), file=data.hex())
        self.compared += 1
        return got

    def read_by_outside(self, who, passphrase, data, want, inputs, item=None, exponent=None):
        """`who` opens `data` to the QR TOML `want`; `tomllib` reads what was opened
        and finds `item`; the PyCA judge also finds the S2K's `exponent`."""
        if who == "pyca":
            opened = pyca_open(passphrase, data)
            found = opened and opened[0]
            if opened and exponent is not None and opened[1] != exponent:
                raise Mismatch(self.name, "qrvault.seal", inputs, opened[1], exponent,
                               "the S2K's memory exponent is not the one asked for")
        else:
            found = (sq_open if who == "sq" else rpgp_open)(passphrase, data)
        self.compared += 1
        if found != want:
            raise Mismatch(self.name, "qrvault.seal", inputs,
                           found and found.hex(), want and want.hex(),
                           f"{who} does not read what Faraday wrote"
                           + (f" ({PYCA_WHY})" if who == "pyca" and found is None else ""))
        if item is not None:
            self.compared += 1
            if tomllib.loads(found.decode()) != toml_of(item):
                raise Mismatch(self.name, "qrvault.seal", inputs,
                               tomllib.loads(found.decode()), toml_of(item),
                               "tomllib does not read the item from the opened text")

    def written_by_outside(self, who, message, open_as, item, inputs):
        """Faraday opens a message `who` wrote, under `open_as`."""
        got = self.faraday_open(open_as, message)
        if got != ("ok", {"item": item}):
            raise Mismatch(self.name, "qrvault.open", inputs, got,
                           ("ok", {"item": item}), f"Faraday does not read what {who} wrote")

    def round(self, item, passphrase, as_nfc=None, exponent=16, wrong=True):
        """Faraday's seal read by the three judges and by Faraday; the messages rPGP and
        the PyCA judge seal read by Faraday. `exponent` is Faraday's memory step; a round
        that does not name one has the others seal at a random small step."""
        rng = self.rng
        nfc = as_nfc if as_nfc is not None else passphrase
        want = qr_toml(item)
        inputs = {"item": item, "passphrase": passphrase, "exponent": exponent}
        random = self.randbytes(RANDOM_LEN)
        msg = self.faraday_seal(passphrase, item, random, exponent)
        if msg is None:
            return
        # The writer, byte for byte: the same random bytes, split the same way (§1.2).
        self.compared += 1
        again = pyca_seal(nfc, exponent, want, random)
        if msg != again:
            raise Mismatch(self.name, "qrvault.seal", inputs, msg.hex(), again.hex(),
                           "Faraday's seal is not the PyCA judge's seal of the same random bytes")
        for who in ("sq", "rpgp", "pyca"):
            self.read_by_outside(who, nfc, msg, want, inputs, item, exponent)
        self.written_by_outside("faraday", msg, passphrase, item, inputs)
        if wrong:
            wrong_pass = nfc + "x"
            for who, opener in (("sq", sq_open), ("rpgp", rpgp_open), ("pyca", pyca_open)):
                self.compared += 1
                if opener(wrong_pass, msg) is not None:
                    raise Mismatch(self.name, "qrvault.seal", inputs, "opened", None,
                                   f"{who} opened it under a wrong passphrase")
            self.compared += 1
            if self.faraday_open(wrong_pass, msg)[0] != "err":
                raise Mismatch(self.name, "qrvault.open", inputs, "opened", ("err", None),
                               "Faraday opened it under a wrong passphrase")
        # rPGP's and PyCA's seals, opened by Faraday (`sq` 1.3.1 cannot write one: below).
        other = exponent if exponent != 16 else rng.choice([13, 14, 15, 16])
        sealed = rpgp_seal(nfc, other, want)
        if sealed is None:
            raise Mismatch(self.name, "rpgp seal", inputs, None, "a message",
                           "rPGP refused to seal")
        self.written_by_outside("rpgp", sealed, passphrase, item, inputs)
        self.written_by_outside("pyca", pyca_seal(nfc, other, want, self.randbytes(RANDOM_LEN)),
                                passphrase, item, inputs)

    # -- items

    def words(self):
        return Mnemonic("english").to_mnemonic(self.randbytes(self.rng.choice([16, 32])))

    def text(self, n):
        pool = ("abcxyz ABC 0189 <>&\"' \\ \t\n\r \x01\x1f\x7f é ß ñ 日本語"
                " ключ \U0001f511 abandon about zoo [] = # ")
        return "".join(self.rng.choice(pool) for _ in range(n))

    def share(self):
        rng = self.rng
        header = str(rng.randint(2, 9)) + "".join(rng.choice(BECH32) for _ in range(4)) \
            + rng.choice(BECH32.replace("s", ""))
        return codex32_encode("ms", header, self.randbytes(rng.choice([16, 32])))

    def item(self):
        rng = self.rng
        kind = rng.choice(["words", "words", "codex32", "passphrase", "note", "note", "empty"])
        fp = self.randbytes(4).hex()
        if kind == "words":
            return {"kind": "words", "text": self.words()}
        if kind == "codex32":
            return {"kind": "codex32", "text": self.share(), "for": fp}
        if kind == "passphrase":
            return {"kind": "passphrase", "text": self.text(rng.randint(0, 40)), "for": fp}
        if kind == "note":
            return {"kind": "note", "text": self.text(rng.randint(1, 60))}
        return {"kind": "note", "text": ""}

    def passphrase(self):
        return self.text(self.rng.randint(1, 20)).replace("\x00", "") or "p"

    # -- parts

    def long_seed(self):
        """A 24-word seed of the longest words (8 letters): its line is the 226 bytes."""
        m = Mnemonic("english")
        eight = [w for w in m.wordlist if len(w) == 8]
        while True:
            head = [self.rng.choice(eight) for _ in range(23)]
            for last in eight:
                words = " ".join(head + [last])
                if m.check(words):
                    return words

    def refused_after_rpgp(self, why, text):
        """A document rPGP seals and Faraday must refuse to open as a QR vault."""
        inputs = {"document": why}
        sealed = rpgp_seal(PHRASE, 13, padded(text))
        if sealed is None:
            raise Mismatch(self.name, "rpgp seal", inputs, None, "a message", "rPGP refused to seal")
        got = self.faraday_open(PHRASE, sealed)
        if got[0] != "err":
            raise Mismatch(self.name, "qrvault.open", inputs, got, ("err", None),
                           f"Faraday opened {why}")

    def multi_chunk(self):
        """rPGP seals with chunk size octet 0 (64 bytes: the 235 bytes of the literal packet
        are four chunks and a final tag), and Faraday opens it (§1.2 reads 0 to 16)."""
        for item in (PUBLISHED[0][2], {"kind": "note", "text": "n" * 216}):
            sealed = rpgp_seal(PHRASE, 13, qr_toml(item), chunk=0)
            if sealed is None:
                raise Mismatch(self.name, "rpgp seal", {"item": item}, None, "a message",
                               "rPGP refused to seal in several chunks")
            if len(sealed) <= 396:
                raise Mismatch(self.name, "rpgp seal", {"item": item}, len(sealed), "> 396",
                               "rPGP's chunk size octet 0 made one chunk")
            self.written_by_outside("rpgp, in several chunks", sealed, PHRASE, item,
                                    {"item": item, "chunk": 0})

    def flipped(self):
        """The PyCA judge refuses a good vector with one byte changed in each field: the packet
        headers, the S2K's cost and the algorithm octets, the chunk size octet, the ciphertext
        (the salts, the IV, the wrapped key, the chunk, the final tag), and the literal data
        packet's header (sealed again, as it is inside the encryption)."""
        good = (VECTORS / "qr-note.pgp").read_bytes()
        offsets = {"packet headers": [0, 1, 90, 91, 92],
                   "algorithm octets": [2, 3, 4, 5, 6, 7, 93, 94, 95],
                   "S2K cost": [24, 25, 26], "chunk octet": [96],
                   "ciphertext": [8, 27, 42, 97, 129, 380, 395]}
        for field, at in offsets.items():
            for i in at:
                bad = bytearray(good)
                bad[i] ^= 1
                self.compared += 1
                if pyca_open(PHRASE, bytes(bad)) is not None:
                    raise Mismatch(self.name, "qrvault.open", {"field": field, "byte": i},
                                   "opened", None,
                                   f"the PyCA judge opened a vector with byte {i} ({field}) changed")
        plain = qr_toml({"kind": "note", "text": "The safe's code is in the drawer."})
        for i in range(len(LITERAL_HEAD)):
            head = bytearray(LITERAL_HEAD)
            head[i] ^= 1
            bad = pyca_seal(PHRASE, VECTOR_EXPONENT, plain, self.randbytes(RANDOM_LEN), bytes(head))
            self.compared += 1
            if pyca_open(PHRASE, bad) is not None:
                raise Mismatch(self.name, "qrvault.open", {"field": "literal header", "byte": i},
                               "opened", None,
                               f"the PyCA judge opened a literal header with byte {i} changed")

    def prelude(self):
        self.check_tools()
        check_pyca()
        if "cryptographic backend Nettle" not in (judges.sq_backend() or ""):
            raise HarnessError("the qrvault area needs sq built over Nettle (`sq version`)")
        for name, passphrase, item in PUBLISHED:
            data = (VECTORS / name).read_bytes()
            want = qr_toml(item)
            inputs = {"vector": name}
            for who in ("sq", "rpgp", "pyca"):
                self.read_by_outside(who, passphrase, data, want, inputs, item, VECTOR_EXPONENT)
            got = self.faraday_open(passphrase, data)
            if got != ("ok", {"item": item}):
                raise Mismatch(self.name, "qrvault.open", inputs, got, ("ok", {"item": item}),
                               "Faraday does not open the repository's vector")
            if passphrase != unicodedata.normalize("NFD", passphrase):
                got = self.faraday_open(unicodedata.normalize("NFD", passphrase), data)
                if got != ("ok", {"item": item}):
                    raise Mismatch(self.name, "qrvault.open", inputs, got, ("ok", {"item": item}),
                                   "Faraday does not open the vector under NFD")
        for name in REFUSED:
            data = (VECTORS / name).read_bytes()
            got = self.faraday_open(PHRASE, data)
            if got[0] != "err":
                raise Mismatch(self.name, "qrvault.open", {"vector": name}, got, ("err", None),
                               "Faraday opened a message §1.2 refuses")
            self.compared += 1
            if pyca_open(PHRASE, data) is not None:
                raise Mismatch(self.name, "qrvault.open", {"vector": name}, "opened", None,
                               "the PyCA judge opened a message §1.2 refuses")
        # A note too long for the code is not sealed.
        self.faraday_seal(PHRASE, {"kind": "note", "text": "n" * 300}, bytes(95))
        # `sq` 1.3.1 writes a SKESK v4 and a SEIPD v1 for a password whatever
        # `--profile` says (the profile only steers messages for certificates):
        # Faraday and the PyCA judge refuse what it writes, with or without the profile.
        for profile in ("", "rfc9580"):
            old = sq_seal(PHRASE, qr_toml({"kind": "note", "text": "n"}), profile=profile)
            if old is None:
                raise Mismatch(self.name, "sq encrypt", {}, None, "a message",
                               "sq refused to seal")
            self.compared += 1
            if (packet_heads(old) or [])[:2] != [(3, 4), (18, 1)]:
                raise Mismatch(self.name, "sq encrypt", {"profile": profile},
                               packet_heads(old), [(3, 4), (18, 1)],
                               "sq no longer writes SKESK v4 and SEIPD v1 for a password; "
                               "F211 and §10 name it, so sq can now seal and the area's "
                               "direction for it changes")
            got = self.faraday_open(PHRASE, old)
            if got[0] != "err":
                raise Mismatch(self.name, "qrvault.open", {"profile": profile}, got,
                               ("err", None), "Faraday opened an sq message of SKESK v4")
            self.compared += 1
            if pyca_open(PHRASE, old) is not None:
                raise Mismatch(self.name, "qrvault.open", {"profile": profile}, "opened", None,
                               "the PyCA judge opened an sq message of SKESK v4")
        # One deterministic round for each kind of item.
        for _, _, item in PUBLISHED:
            self.round(item, PHRASE)
        # The size limits (§2.1): a note of 216 and 217 bytes, a passphrase of 193 and 194,
        # the seed line of 24 words of 8 letters; the longer ones are not sealed.
        for item in (
            {"kind": "note", "text": "n" * 216}, {"kind": "note", "text": "n" * 217},
            {"kind": "passphrase", "text": "p" * 193, "for": "73c5da0a"},
            {"kind": "passphrase", "text": "p" * 194, "for": "73c5da0a"},
            {"kind": "words", "text": self.long_seed()},
        ):
            self.round(item, PHRASE)
        # A seed of 15 words, and a document of two keys, sealed by rPGP.
        fifteen = Mnemonic("english").to_mnemonic(self.randbytes(20))
        self.refused_after_rpgp("a 15-word seed", 'words = "%s"\n' % fifteen)
        self.refused_after_rpgp("a document of two keys",
                                'words = "%s"\nnote = "n"\n' % TWELVE)
        # The NFD passphrase of §3, both directions and all the judges.
        nfc = "caf\u00e9"
        nfd = unicodedata.normalize("NFD", nfc)
        assert nfd == "cafe\u0301" and nfd != nfc
        self.round({"kind": "note", "text": nfc}, nfd, as_nfc=nfc)
        self.multi_chunk()
        self.flipped()

    def case_steps(self):
        """Faraday's seal at one memory step of §3 (2^16 to 2^20 KiB): a run at scale 1 has
        each, a shorter one the first."""
        step = 16 + self.case_at[1] % 5
        self.round(self.item(), self.passphrase(), exponent=step, wrong=False)

    def case_corpus(self):
        item = self.item()
        passphrase = self.passphrase()
        if self.rng.random() < 0.2:
            nfd = unicodedata.normalize("NFD", passphrase + "é")
            self.round(item, nfd, as_nfc=unicodedata.normalize("NFC", nfd))
        else:
            self.round(item, unicodedata.normalize("NFC", passphrase))

    LOOPS = (("steps", 5), ("corpus", 40))
