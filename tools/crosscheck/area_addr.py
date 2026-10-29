"""Addresses, descriptor checksums and single-key accounts against
Bitcoin Core's functional-test framework (`core/test_framework/`):
`segwit_addr.py` (BIP-173 and BIP-350's reference code as Core keeps
it), `address.py`'s Base58Check and `descriptors.py`'s BIP-380 checksum,
with BIP-32 from `btc.py` and BIP-39 from python-mnemonic. Every
descriptor checksum is also asked of BIP 380's own code, as the BIP
prints it (`tools/reference/bip380/descsum.py`).

- Every address of `tools/vectors/addresses/`: BIP-350's and BIP-173's
  valid addresses read to the script each pays and written back from
  it; their invalid addresses refused on every network.
- BIP-380's checksum cases, and BIP-49's, BIP-84's and BIP-86's test
  accounts (the account's extended public key, SLIP-132 where the BIP
  writes it, and each address).
- Random output scripts of every kind an address has (P2PKH, P2SH,
  segwit version 0 of 20 and 32 bytes, version 1 of 32 bytes, versions
  1 to 16 of 2 to 40 bytes) written as addresses on each network and read
  back, and scripts no address has refused; the addresses changed a
  character, in case, in their prefix, in their checksum's kind, read
  on the wrong network, cut or lengthened.
- Random single-key accounts of each type: the account xpub, its
  SLIP-132 form, its descriptor with key origin and checksum, and an
  address at a random index (BIP-86's tweak included), with indices of
  2³¹ and above refused.
- Random descriptors over BIP-380's character set and outside it: the
  checksum made, and a checksum checked after a character is changed.
- Bitcoin Core v31.1 itself (bitcoind on regtest, offline;
  `core_node.py`), for every regtest account above and a fixed set in
  the prelude: Core derives the address from the seed's master key,
  written as a `tprv` descriptor with the account's whole path
  (`deriveaddresses`), with no `btc.py` in the way; and Core reads
  Faraday's own account descriptor, checksum included
  (`getdescriptorinfo`), and derives the same address from its key.
  Indices of 2³¹ and above are refused by both.

An address is read as Faraday reads one: spaces at its ends ignored, a
refusal named `Invalid` when no network reads it and `WrongNetwork`
when another network does.
"""

import hashlib
import hmac
import json

import btc
import core_node
from btc import HARDENED
from common import ROOT, Area, Mismatch, as_verify, use_reference

use_reference("core")
from test_framework import descriptors  # noqa: E402
from test_framework import segwit_addr  # noqa: E402
from test_framework.address import base58_to_byte, byte_to_base58  # noqa: E402
from test_framework.authproxy import JSONRPCException  # noqa: E402

use_reference("bip380")
import descsum as bip380  # noqa: E402

use_reference("bip39")
from mnemonic import Mnemonic  # noqa: E402

NETWORKS = ["mainnet", "testnet", "signet", "regtest"]
HRP = {"mainnet": "bc", "testnet": "tb", "signet": "tb", "regtest": "bcrt"}
P2PKH = {"mainnet": 0, "testnet": 111, "signet": 111, "regtest": 111}
P2SH = {"mainnet": 5, "testnet": 196, "signet": 196, "regtest": 196}
# SLIP-132's versions for an account's public key: nested and native
# segwit, mainnet and test.
SLIP132 = {(1, False): "049d7cb2", (1, True): "044a5262", (2, False): "04b24746",
           (2, True): "045f1cf6"}
PURPOSE = [44, 49, 84, 86]


class Refused(Exception):
    pass


def witness_script(version, program):
    return bytes([0x50 + version if version else 0, len(program)]) + bytes(program)


def read_on(text, network):
    """The script `text` pays on `network`, or None."""
    version, program = segwit_addr.decode_segwit_address(HRP[network], text)
    if version is not None:
        return witness_script(version, program)
    try:
        payload, v = base58_to_byte(text)
    except Exception:  # noqa: BLE001 - not base58check
        return None
    if len(payload) != 20:
        return None
    if v == P2PKH[network]:
        return b"\x76\xa9\x14" + payload + b"\x88\xac"
    if v == P2SH[network]:
        return b"\xa9\x14" + payload + b"\x87"
    return None


def ref_parse(text, network):
    text = text.strip()
    script = read_on(text, network)
    if script is not None:
        return {"script": script.hex()}
    if any(read_on(text, n) is not None for n in NETWORKS):
        raise Refused("WrongNetwork")
    raise Refused("Invalid")


def ref_from_script(script, network):
    """The address of an output script, where it has one."""
    if len(script) == 25 and script[:3] == b"\x76\xa9\x14" and script[23:] == b"\x88\xac":
        return byte_to_base58(script[3:23], P2PKH[network])
    if len(script) == 23 and script[:2] == b"\xa9\x14" and script[22] == 0x87:
        return byte_to_base58(script[2:22], P2SH[network])
    if len(script) >= 4 and (script[0] == 0 or 0x51 <= script[0] <= 0x60) \
            and script[1] == len(script) - 2 and 2 <= script[1] <= 40:
        version = script[0] - 0x50 if script[0] else 0
        address = segwit_addr.encode_segwit_address(HRP[network], version, script[2:])
        if address:
            return address
    raise Refused("not an address")


def tagged(tag, msg):
    t = hashlib.sha256(tag.encode()).digest()
    return hashlib.sha256(t + t + msg).digest()


def p2tr_bip86(pub):
    """BIP-86: the output key of an internal key with no script path."""
    p = btc.GE.from_bytes_compressed(pub)
    if not p.has_even_y():
        p = -p
    xonly = p.to_bytes_xonly()
    t = int.from_bytes(tagged("TapTweak", xonly), "big")
    if t >= btc.N:
        raise Refused("tweak")
    return (p + btc.FAST_G.mul(t)).to_bytes_xonly()


def ref_account(seed, network, script, account, change, index):
    test = network != "mainnet"
    if account >= HARDENED or index >= HARDENED:
        raise Refused("index out of range")
    m = btc.master(seed, test)
    path = [PURPOSE[script] + HARDENED, (1 if test else 0) + HARDENED, account + HARDENED]
    acct = m.derive(path)
    xpub = acct.neuter().serialize()
    slip = xpub
    if (script, test) in SLIP132:
        raw = btc.b58check_decode(xpub)
        slip = btc.b58check_encode(bytes.fromhex(SLIP132[(script, test)]) + raw[4:])
    origin = "/".join([m.fingerprint().hex()] + [f"{i - HARDENED}h" for i in path])
    key = f"[{origin}]{xpub}/<0;1>/*"
    body = ["pkh({})", "sh(wpkh({}))", "wpkh({})", "tr({})"][script].format(key)
    pub = acct.neuter().derive([int(change), index]).pub
    h = btc.hash160(pub)
    if script == 0:
        address = byte_to_base58(h, P2PKH[network])
    elif script == 1:
        address = byte_to_base58(btc.hash160(b"\x00\x14" + h), P2SH[network])
    elif script == 2:
        address = segwit_addr.encode_segwit_address(HRP[network], 0, h)
    else:
        address = segwit_addr.encode_segwit_address(HRP[network], 1, p2tr_bip86(pub))
    return {"xpub": xpub, "slip132": slip, "descriptor": descriptors.descsum_create(body),
            "address": address}


def core_account(seed, script, account, change, index):
    """Bitcoin Core's address for a regtest account, derived by Core from
    the seed's master key alone: ("ok", {"address": ...}), or ("err", why)
    when Core refuses. Costs two RPCs (a hardened derivation of four or
    five steps each, in Core). A node that does not answer is the
    harness's failure, never a refusal."""
    i = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    raw = bytes.fromhex("04358394") + bytes(9) + i[32:] + b"\0" + i[:32]
    tprv = byte_to_base58(raw[1:], raw[0])
    path = f"{PURPOSE[script]}h/1h/{account}h/{int(change)}/*"
    body = ["pkh({})", "sh(wpkh({}))", "wpkh({})", "tr({})"][script].format(f"{tprv}/{path}")
    node = core_node.rpc()
    try:
        desc = f"{body}#{node.getdescriptorinfo(body)['checksum']}"
        return ("ok", {"address": node.deriveaddresses(desc, [index, index])[0]})
    except JSONRPCException as e:
        return ("err", f"Core: {e.error.get('message')}")


def core_reads(descriptor, change, index):
    """The address Core derives from Faraday's own account descriptor
    (`<0;1>/*`, with its key origin and checksum), or why Core refuses it.
    Costs two RPCs."""
    node = core_node.rpc()
    try:
        node.getdescriptorinfo(descriptor)
        return ("ok", node.deriveaddresses(descriptor, [index, index])[int(change)][0])
    except JSONRPCException as e:
        return ("err", f"Core: {e.error.get('message')}")


def refusal(answer):
    if answer[0] == "ok":
        return answer
    reason = answer[1].removeprefix("Refused: ")
    return ("ok", {"refused": reason if reason in ("Invalid", "WrongNetwork", "Silent")
                   else "refused"})


class Addr(Area):
    name = "addr"
    CORE = True
    faraday = (
        "`faraday_bip::address::parse`, `bitcoin`'s `Address::from_script`, "
        '`faraday_bip::account` (account xpub, SLIP-132, descriptor, addresses), '
        "`faraday_bip::descriptor`'s checksum"
    )
    reference = (
        "Bitcoin Core's functional-test framework (`core/`): `segwit_addr.py`, "
        "Base58Check, `descriptors.py`; BIP 380's own checksum code as the BIP prints "
        "it (`bip380/`); `btc.py`; BIP-86's tweak written in the area; Bitcoin Core "
        "v31.1's `bitcoind` on regtest for regtest accounts (`deriveaddresses`, "
        "`getdescriptorinfo`)"
    )
    reference_checked = (
        'its vendored files are hashed against `references.sha256` before any run, and '
        "a published vector's answer is required of the reference as well as of "
        "Faraday; every descriptor checksum is also asked of BIP 380's own code, as the "
        "BIP prints it; Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version is "
        "asked"
    )
    vectors = (
        "BIP-350's and BIP-173's addresses, BIP-380's checksum cases, BIP-49's, 84's "
        "and 86's accounts (`tools/vectors/addresses/`)"
    )
    corpus = (
        'scripts of every address kind on every network, written and read back, then '
        "changed; accounts of every type at random indices; random descriptors' "
        'checksums'
    )
    differences = [
        (
            'none'
        ),
    ]
    not_compared = (
        'none'
    )

    def parse(self, text, network, expected=None):
        return self.compare("addr.parse", {"text": text, "network": network},
                            lambda: ref_parse(text, network), expected, normalize=refusal)

    def from_script(self, script, network):
        return self.compare("addr.from_script", {"script": script.hex(), "network": network},
                            lambda: {"address": ref_from_script(script, network)})

    def account(self, seed, network, script, account, change, index, expected=None):
        return self.compare(
            "addr.account", {"seed": seed.hex(), "network": network, "script": script,
                             "account": account, "change": change, "index": index},
            lambda: ref_account(seed, network, script, account, change, index), expected)

    def core(self, seed, script, account, change, index):
        """A regtest account asked of Bitcoin Core: the address Core derives
        from the seed must be Faraday's, and Core must read Faraday's
        descriptor to the same address."""
        inputs = {"seed": seed.hex(), "network": "regtest", "script": script,
                  "account": account, "change": change, "index": index}
        ref = core_account(seed, script, account, change, index)

        def by_core():
            if ref[0] == "err":
                raise core_node.CoreError(ref[1])
            return ref[1]

        def address(answer):
            return answer if answer[0] == "err" else ("ok", {"address": answer[1]["address"]})
        got = self.compare("addr.account", inputs, by_core, normalize=address)
        if got[0] != "ok":
            return
        read = core_reads(got[1]["descriptor"], change, index)
        self.compared += 1
        if read != ("ok", got[1]["address"]):
            raise Mismatch(self.name, "addr.account", inputs,
                           {"descriptor": got[1]["descriptor"], "address": got[1]["address"]},
                           read, "Core reads Faraday's descriptor to another address")

    def core_fixed(self):
        """Each account type, both chains, the first and a high index, and
        both refusals, at a seed of each length."""
        for n, length in enumerate([16, 32, 64]):
            seed = bytes((n * 37 + i) % 256 for i in range(length))
            for script in range(4):
                for change, index in [(False, 0), (True, 1), (False, HARDENED - 1)]:
                    self.core(seed, script, n, change, index)
        self.core(bytes(32), 2, 0, False, HARDENED)
        self.core(bytes(32), 3, HARDENED, False, 0)

    def descsum(self, text):
        """The checksum made, asked of BIP 380's code and of Core's
        `descriptors.py`."""
        def ref(create):
            return lambda: {"checksum": create(text)[-8:]}
        self.compare("addr.descsum", {"text": text}, ref(bip380.descsum_create))
        return self.compare("addr.descsum", {"text": text}, ref(descriptors.descsum_create))

    def descsum_verify(self, text, expected=None):
        """The checksum checked, asked of both; BIP 380's code raises on a
        text too short to hold one, which counts as a refusal."""
        self.compare("addr.descsum_verify", {"text": text},
                     lambda: {"valid": bip380.descsum_check(text)}, expected, normalize=as_verify)
        return self.compare("addr.descsum_verify", {"text": text},
                            lambda: {"valid": descriptors.descsum_check(text, True)}, expected,
                            normalize=as_verify)

    def published(self):
        v = json.loads((ROOT / "tools/vectors/addresses/vectors.json").read_text())
        for case in v["segwit_valid"]:
            a = case["address"]
            network = "mainnet" if a.lower().startswith("bc1") else "testnet"
            self.parse(a, network, ("ok", {"script": case["script"]}))
            self.from_script(bytes.fromhex(case["script"]), network)
        for case in v["segwit_invalid"]:
            for network in NETWORKS:
                self.parse(case["address"], network, ("ok", {"refused": "Invalid"}))
        for case in v["descriptor_checksums"]:
            valid = case["name"] == "Valid checksum"
            self.descsum_verify(case["text"], ("ok", {"valid": valid}))
            self.descsum(case["text"].split("#")[0])
        for acct in v["accounts"]:
            seed = Mnemonic.to_seed(acct["mnemonic"], "")
            script = PURPOSE.index(acct["purpose"])
            for a in acct["addresses"]:
                got = self.need(self.account(seed, acct["network"], script, 0, bool(a["change"]),
                                             a["index"]), "account")
                if got["address"] != a["address"]:
                    raise AssertionError(f"BIP-{acct['purpose']} address {a}")
                if acct["xpub"] not in (got["xpub"], got["slip132"]):
                    raise AssertionError(f"BIP-{acct['purpose']} account key")

    def random_script(self):
        rng = self.rng
        kind = rng.randrange(7)
        if kind == 0:
            return b"\x76\xa9\x14" + self.randbytes(20) + b"\x88\xac"
        if kind == 1:
            return b"\xa9\x14" + self.randbytes(20) + b"\x87"
        if kind == 2:
            return witness_script(0, self.randbytes(rng.choice([20, 32])))
        if kind == 3:
            return witness_script(1, self.randbytes(32))
        if kind == 4:
            return witness_script(rng.randint(1, 16), self.randbytes(rng.randint(2, 40)))
        if kind == 5:
            # Scripts no address has: a version 0 program of another
            # length, a program too short or too long, a push that does
            # not match its length, a bare key, an OP_RETURN.
            return rng.choice([
                witness_script(0, self.randbytes(rng.choice([0, 1, 19, 21, 31, 33, 40]))),
                witness_script(rng.randint(1, 16), self.randbytes(rng.choice([0, 1, 41]))),
                b"\x00\x15" + self.randbytes(20),
                b"\x21" + self.randbytes(33) + b"\xac",
                b"\x6a" + self.randbytes(rng.randint(0, 40)),
                b"",
            ])
        return self.randbytes(rng.randint(0, 50))

    def mutate(self, address, network):
        rng = self.rng
        how = rng.randrange(8)
        if how == 0 and address:
            i = rng.randrange(len(address))
            return address[:i] + rng.choice("qpzry9x8gf2tvdw0s3jn54khce6mua7lQ1bio") + address[i + 1:]
        if how == 1:
            return address.upper() if rng.random() < 0.5 else address[:5] + address[5:].upper()
        if how == 2:
            return rng.choice(["bc1", "tb1", "bcrt1", "BC1", "tc1", "b1"]) + address.split("1", 1)[-1]
        if how == 3:
            # The other checksum constant, the data kept.
            enc, hrp, data = segwit_addr.bech32_decode(address)
            if enc is None:
                return address
            other = (segwit_addr.Encoding.BECH32M if enc == segwit_addr.Encoding.BECH32
                     else segwit_addr.Encoding.BECH32)
            return segwit_addr.bech32_encode(other, hrp, data)
        if how == 4:
            return address[:-rng.randint(1, 6)]
        if how == 5:
            return address + rng.choice(["q", "qqqqqq", "0"])
        if how == 6:
            return rng.choice([" ", "\t", "\n "]) + address + rng.choice(["", " ", "\n"])
        return "x" * rng.randint(0, 100)

    def corpus_case(self):
        rng = self.rng
        network = rng.choice(NETWORKS)
        script = self.random_script()
        got = self.from_script(script, network)
        if got[0] == "ok":
            address = got[1]["address"]
            self.parse(address, network)
            self.parse(address, rng.choice(NETWORKS))
            self.parse(self.mutate(address, network), rng.choice([network] + NETWORKS))
        if rng.random() < 0.15:
            account = rng.choice([0, 1, rng.randrange(HARDENED), HARDENED - 1, HARDENED])
            index = rng.choice([0, 1, rng.randrange(1000), rng.randrange(HARDENED), HARDENED])
            seed = self.randbytes(rng.choice([16, 32, 64]))
            script, change = rng.randrange(4), rng.random() < 0.3
            self.account(seed, network, script, account, change, index)
            if network == "regtest":
                self.core(seed, script, account, change, index)
        if rng.random() < 0.3:
            charset = descriptors.INPUT_CHARSET
            text = "".join(rng.choice(charset) for _ in range(rng.randint(0, 120)))
            if rng.random() < 0.1:
                i = rng.randint(0, len(text))
                text = text[:i] + rng.choice(["Ü", "\n", "\t", "€", "\x7f"]) + text[i:]
            got = self.descsum(text)
            if got[0] == "ok":
                full = f"{text}#{got[1]['checksum']}"
                self.descsum_verify(full)
                i = rng.randrange(len(full))
                self.descsum_verify(full[:i] + rng.choice(charset) + full[i + 1:])

    LOOPS = (("corpus", 4000),)

    def prelude(self):
        self.published()
        self.core_fixed()

    def case_corpus(self):
        self.corpus_case()
