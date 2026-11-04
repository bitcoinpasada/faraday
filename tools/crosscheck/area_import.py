"""A wallet imported from another wallet's file, and BIP 129's key
records, against code Faraday did not write.

Faraday's side is `faraday_intake::wallet::read_wallet`, which every screen
that takes a wallet's text calls, and `faraday_bip::bsms`'s key record
read, verified and written as the Create screen and `faraday-keys` do
(`src/import.rs`).

- The wallet a file is. Its keys are made with btclib-wallet's BIP-32
  (`btclib_wallet.bip32`, btclib's own curve) from random seeds, and
  spelled in btclib's SLIP-132 versions; the addresses of the wallet the
  file states are Bitcoin Core v31.1's (`deriveaddresses` on regtest, each
  key given to Core in its `tpub` spelling, made by btclib), compared as
  scripts, so a file for mainnet is judged by the same node; descriptor
  checksums are Core's (`descriptors.py`, BIP-380's own code). Faraday must
  read the keys with their origins as the file states them and derive
  Core's first receive and change scripts. A stated first address (a
  Coldcard export's `first`, a BSMS record's fourth line) that is not
  Core's, and a checksum Core's code calls wrong, must be refused.
- BIP 129's key records. A record signed by btclib's `bms` (the account
  node's key, BIP-137's compact form, RFC 6979) must verify in Faraday,
  and one damaged must not, as btclib's verifier says; the record Faraday
  writes for a seed's BIP-48 account must be btclib's record for the same
  account byte for byte (btclib derives the account and signs).

Every real file the wallets' test suites publish is read too
(`faraday/faraday-core/tests/vectors/interop/` and `tools/vectors/wallet-files/`,
each with its repository, commit, licence and SHA-256 in its README), by
Faraday and by `read_real`, the area's reading of the file's format, with
the wallet judged by Core (batch X30).

The files the area makes are written here, from each format's documentation
(Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's and
Keystone's, Coldcard's file for Wasabi, Specter Desktop's, Specter DIY's
`addwallet` command, read as its `Wallet.parse` reads it, Core's
`listdescriptors`, BIP 129's descriptor record): what a format's fields
mean is the area's reading of it, and the prelude adds the files other
wallets wrote (BIP 129's examples, Sparrow's and Nunchuk's BSMS records,
Coldcard's exports, a Core `listdescriptors`) read the same way. What
Faraday refuses where no outside code has a rule (F155's SLIP-132 checks,
the BSMS paths line, a description over eighty characters, a token not
BIP 129's) is expected here from that rule, named in the facts.
"""

import base64
import json
import math
import re

import core_node
from common import ROOT, Area, HarnessError, use_reference, use_wheels

use_reference("core")
from test_framework.address import address_to_scriptpubkey  # noqa: E402
from test_framework.descriptors import descsum_check, descsum_create  # noqa: E402

use_wheels("btclib", [
    "typing_extensions-4.16.0-py3-none-any.whl",
    "ecc-2026.10.23-py3-none-any.whl",
    "btclib-2026.10.24-py3-none-any.whl",
    "btclib_wallet-2026.10.29-py3-none-any.whl",
])
from ecc import curves  # noqa: E402

curves.set_libsecp256k1_serving(serving=False)
if curves.is_libsecp256k1_serving():
    raise HarnessError("btclib would sign through libsecp256k1")

from btclib import b32, b58  # noqa: E402
from btclib.ecc import bms  # noqa: E402
from btclib.key import PrvKeyData, PubKeyData  # noqa: E402
from btclib.network import NETWORKS  # noqa: E402
from btclib.script.script_pub_key import ScriptPubKey  # noqa: E402
from btclib_wallet import bip32  # noqa: E402
from ecc.curves.sec_point import bytes_from_point, point_from_octets_var  # noqa: E402

N = 3  # receive and change scripts compared, from index 0
NUMS = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"
INTEROP = ROOT / "faraday" / "faraday-core" / "tests" / "vectors" / "interop"
WALLET_FILES = ROOT / "tools" / "vectors" / "wallet-files"
BIP129 = ROOT / "tools" / "vectors" / "bip129" / "bip-0129.mediawiki"
KEY = re.compile(r"(?:\[([0-9a-fA-F]{8})((?:/[0-9]+['hH]?)*)\])?"
                 r"([xtyzuvYZUV]pub[1-9A-HJ-NP-Za-km-z]{100,})")

# Single-key wallets: (purpose, template, Coldcard's field, Electrum's version).
SINGLE = {
    "pkh": ("44h", "pkh(@0)", "bip44", "bip32_pub"),
    "sh-wpkh": ("49h", "sh(wpkh(@0))", "bip49", "slip132_p2wpkh_p2sh_pub"),
    "wpkh": ("84h", "wpkh(@0)", "bip84", "slip132_p2wpkh_pub"),
    "tr": ("86h", "tr(@0)", "bip86", None),
}
COLDCARD_ORDER = ["bip84", "bip86", "bip49", "bip44"]  # Coldcard's export: the one read
# Multisig wallets: (path after purpose 48 or 45, wrapper, Format line, version).
MULTI = {
    "wsh": ("48h/{coin}h/{acct}h/2h", "wsh({})", "P2WSH", "slip132_p2wsh_pub"),
    "sh-wsh": ("48h/{coin}h/{acct}h/1h", "sh(wsh({}))", "P2SH-P2WSH", "slip132_p2wsh_p2sh_pub"),
    "sh": ("45h", "sh({})", "P2SH", "bip32_pub"),
}
# Wallets only a descriptor states: (path, template, keys).
MINI = {
    "miniscript": ("48h/{coin}h/{acct}h/2h", "wsh(or_d(pk(@0),and_v(v:pkh(@1),older(52560))))", 2),
    "tree": ("86h/{coin}h/{acct}h", "tr(@0,{and_v(v:pk(@1),older(4320)),pk(@2)})", 3),
    "tapmulti": ("48h/{coin}h/{acct}h/2h", "tr(" + NUMS + ",sortedmulti_a(2,@0,@1,@2))", 3),
}
DESCRIPTOR_FORMATS = ["multipath", "bare", "pair", "labelled", "receive", "core", "specter",
                      "specter-diy"]
NAME_CHARS = "abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.&é"
FORMATS = {
    "single": DESCRIPTOR_FORMATS + ["coldcard", "cobo", "electrum", "keyline", "wasabi"],
    "multi": DESCRIPTOR_FORMATS + ["config", "config-each", "caravan", "electrum", "bsms"],
    "mini": DESCRIPTOR_FORMATS,
}


class Ours(Exception):
    """A refusal Faraday makes by a rule no outside code here has; the
    facts name each."""


# ----- keys, by btclib-wallet ------------------------------------------------------------

JSON_MAX_DEPTH = 32
JSON_MAX_SIZE = 1 << 20


def json_reference(text):
    """CPython's `json` reading a wallet file the way Faraday's reader is
    to: a byte order mark and white space around it, a key named twice
    refused, nesting deeper than 32 refused, a text over 1 MiB refused,
    and no NaN, Infinity, number out of f64's range or lone surrogate. Numbers are compared as the
    floats they stand for, since Faraday keeps the decimal text of the
    value (`1e3` is `1000`)."""
    if len(text.encode()) > JSON_MAX_SIZE:
        raise ValueError("over 1 MiB")

    def members(pairs):
        d = {}
        for k, v in pairs:
            if k in d:
                raise ValueError(f"key {k!r} named twice")
            d[k] = v
        return d

    def refuse(name):
        raise ValueError(name)

    def finite(t):
        f = float(t)
        if not math.isfinite(f):
            raise ValueError(f"{t} is out of the f64 range")
        return f

    value = json.loads(text.removeprefix("\ufeff"), object_pairs_hook=members,
                       parse_constant=refuse, parse_float=finite)

    def walk(v, depth):
        if isinstance(v, bool) or v is None:
            return v
        if isinstance(v, (int, float)):
            try:
                return finite(v)
            except OverflowError:
                raise ValueError("out of the f64 range") from None
        if isinstance(v, str):
            v.encode()  # a lone surrogate does not encode
            return v
        if depth + 1 > JSON_MAX_DEPTH:
            raise ValueError("deeper than 32")
        if isinstance(v, list):
            return [walk(x, depth + 1) for x in v]
        return {walk(k, depth + 1): walk(x, depth + 1) for k, x in v.items()}
    return walk(value, 0)


def spelled(key, net, version):
    """The same extended key in another version (btclib's SLIP-132 table)."""
    return bip32.derive(key, "m", getattr(NETWORKS[net], version))


def plain(key):
    """A key in BIP-32's own version of its network: what Faraday keeps."""
    data = bip32.BIP32KeyData.b58decode(key)
    net = "mainnet" if data.version in {bytes.fromhex(v) for v in (
        "0488b21e", "049d7cb2", "04b24746", "0295b43f", "02aa7ed3")} else "testnet"
    return spelled(key, net, "bip32_pub")


def for_core(key):
    """The key as Core on regtest reads it: its `tpub` spelling."""
    return spelled(key, "testnet", "bip32_pub")


class Account:
    """An account key below a fresh master, all by btclib-wallet."""

    def __init__(self, area, net, path, seed=None):
        self.net = net
        self.seed = seed or area.randbytes(area.rng.choice([16, 32, 64]))
        root = bip32.rootxprv_from_seed(self.seed, NETWORKS[net].bip32_prv)
        self.fp = bip32.fingerprint(root).hex()
        self.path = path
        self.xprv = bip32.derive(root, "m/" + path)
        self.pub = bip32.xpub_from_xprv(self.xprv)

    def origin(self, mark="h"):
        return f"[{self.fp}/{self.path.replace('h', mark)}]"

    def text(self):
        """The key as Faraday states it, `[fingerprint/path]xpub`."""
        return self.origin() + self.pub

    def core(self):
        return self.origin() + for_core(self.pub)

    def secret(self):
        data = bip32.BIP32KeyData.b58decode(self.xprv)
        return PrvKeyData(int.from_bytes(data.key[1:], "big"), self.net, True)


def fill(template, keys):
    return re.sub(r"@(\d+)", lambda m: keys[int(m.group(1))], template)


# ----- Bitcoin Core --------------------------------------------------------------------

def core_scripts(receive, change):
    """Core's first N receive and change scripts of two descriptors over
    `tpub` keys; Core's refusal raises."""
    got = core_node.batch([("deriveaddresses", descsum_create(d), [0, N - 1])
                           for d in (receive, change)])
    return [[address_to_scriptpubkey(a).hex() for a in chain] for chain in got]


def to_core(descriptor):
    """A descriptor with every extended key in its `tpub` spelling."""
    return KEY.sub(lambda m: m.group(0)[:m.start(3) - m.start(0)] + for_core(m.group(3)),
                   descriptor)


def keys_stated(descriptor):
    """The keys a descriptor states, as Faraday states a key."""
    out = []
    for fp, path, key in KEY.findall(descriptor):
        path = re.sub(r"['H]", "h", path)
        origin = f"[{fp.lower()}{path}]" if fp else ""
        out.append(origin + plain(key))
    return out


def address(spk_hex, net):
    """btclib's address of a script on a network: what a file states."""
    return ScriptPubKey(bytes.fromhex(spk_hex), net).address


def spk_of(text):
    try:
        return ScriptPubKey.from_address(text).script.hex()
    except Exception as e:  # noqa: BLE001 - not an address
        raise ValueError(f"btclib: not an address: {text}") from e


# ----- the wallets ----------------------------------------------------------------------

class Wallet:
    """A wallet of one shape over fresh accounts: its template, its keys,
    and Core's scripts."""

    def __init__(self, area, shape, net, acct=None, seed=None):
        rng = area.rng
        self.shape, self.net = shape, net
        coin = 0 if net == "mainnet" else 1
        acct = rng.choice([0, 0, 0, 1, 7]) if acct is None else acct
        self.acct = acct
        if shape in SINGLE:
            self.kind = "single"
            purpose, self.template, _, _ = SINGLE[shape]
            self.accounts = [Account(area, net, f"{purpose}/{coin}h/{acct}h", seed)]
        elif shape in MULTI:
            self.kind = "multi"
            path, wrap, _, _ = MULTI[shape]
            n = rng.choice([1, 2, 2, 3, 3, 3, 4, 5, rng.randrange(6, 16)])
            self.m = rng.randrange(1, n + 1)
            self.sorted = True
            self.accounts = [Account(area, net, path.format(coin=coin, acct=acct))
                             for _ in range(n)]
            self.template = wrap.format(
                f"sortedmulti({self.m}," + ",".join(f"@{i}" for i in range(n)) + ")")
        else:
            self.kind = "mini"
            path, self.template, n = MINI[shape]
            self.accounts = [Account(area, net, path.format(coin=coin, acct=acct))
                             for _ in range(n)]
        self._scripts = None

    def desc(self, tail, keys=None):
        keys = keys or [a.text() for a in self.accounts]
        return fill(self.template, [k + tail for k in keys])

    def scripts(self):
        if self._scripts is None:
            core = [a.core() for a in self.accounts]
            self._scripts = core_scripts(self.desc("/0/*", core), self.desc("/1/*", core))
        return self._scripts

    def expected(self, keys=None):
        receive, change = self.scripts()
        return {"keys": keys or [a.text() for a in self.accounts],
                "receive": receive, "change": change}

    def first(self):
        return address(self.scripts()[0][0], self.net)


def checked(d):
    return descsum_create(d)


def flip_checksum(d):
    body, chk = d.split("#")
    i = len(chk) - 1
    return body + "#" + chk[:i] + ("q" if chk[i] != "q" else "p")


# ----- what other wallets mean by their files (the area's reading) -------------------------

def read_bsms(text):
    """BIP 129's descriptor record as Faraday is to read it, judged by Core."""
    lines = [line.rstrip("\r") for line in text.rstrip("\n\r ").split("\n")]
    if lines[0].strip() != "BSMS 1.0":
        raise ValueError("BIP 129: not version 1.0")
    if len(lines) != 4:
        raise ValueError("BIP 129: a descriptor record is four lines")
    desc, paths, stated = (x.strip() for x in lines[1:])
    if "/**" not in desc:
        raise Ours("a descriptor record that is not a template (`/**`)")
    if paths not in ("/0/*,/1/*", "No path restrictions"):
        raise Ours("a paths line other than /0/*,/1/* or No path restrictions (F64)")
    keys = keys_stated(desc)
    if not keys:
        raise Ours("a record of plain public keys")
    receive, change = core_scripts(to_core(desc.replace("/**", "/0/*")),
                                   to_core(desc.replace("/**", "/1/*")))
    if re.fullmatch(r"(BC|TB|BCRT)1[0-9A-Z]+", stated):
        stated = stated.lower()  # BIP 173: a reader takes the upper case
    if spk_of(stated) != receive[0]:
        raise ValueError("BIP 129: the fourth line is not Core's first address")
    return {"keys": keys, "receive": receive, "change": change}


def read_coldcard(text):
    """Coldcard's generic export: the first account of bip84, bip86, bip49,
    bip44 it has, rooted at the top-level `xfp`, its `first` Core's."""
    v = json.loads(text)
    for field in COLDCARD_ORDER:
        a = v.get(field)
        if not isinstance(a, dict) or "xpub" not in a:
            continue
        shape = next(s for s, x in SINGLE.items() if x[2] == field)
        path = re.sub(r"['H]", "h", a["deriv"]).removeprefix("m/")
        key = f"[{v['xfp'].lower()}/{path}]{plain(a['xpub'])}"
        tmpl = SINGLE[shape][1]
        core = f"[{v['xfp'].lower()}/{path}]" + for_core(a["xpub"])
        receive, change = core_scripts(fill(tmpl, [core + "/0/*"]), fill(tmpl, [core + "/1/*"]))
        if "first" in a and spk_of(a["first"]) != receive[0]:
            raise ValueError("the export's first address is not Core's")
        return {"keys": [key], "receive": receive, "change": change}
    raise ValueError("no single-key account")


def read_specter_diy(text):
    """Specter DIY's `addwallet {name}&{descriptor}` as Specter DIY reads it
    (`Wallet.parse` and `Wallet.from_descriptor` in its
    `src/apps/wallets/wallet.py`): the descriptor after the last `&`; keys
    none of which carries a derivation are its two chains (embit's
    `AllowedDerivation.default()`, `/{0,1}/*`). Specter DIY drops a
    checksum unread; Faraday checks it, which no outside code here does."""
    if not text.startswith("addwallet "):
        raise ValueError("Specter DIY: not an addwallet command")
    _, desc = text[len("addwallet "):].rsplit("&", 1)
    if "#" in desc:
        if not descsum_check(desc, require=True):
            raise Ours("a checksum Specter DIY drops unread")
        desc = desc.split("#")[0]
    if not any(m.end() < len(desc) and desc[m.end()] == "/" for m in KEY.finditer(desc)):
        recv = KEY.sub(lambda m: m.group(0) + "/0/*", desc)
        change = KEY.sub(lambda m: m.group(0) + "/1/*", desc)
    else:
        recv, change = desc.replace("/<0;1>/*", "/0/*"), desc.replace("/<0;1>/*", "/1/*")
    receive, change = core_scripts(to_core(recv), to_core(change))
    return {"keys": keys_stated(desc), "receive": receive, "change": change}


def read_listdescriptors(text):
    """Core's `listdescriptors`: the receive and change pair of one wallet,
    a multisig or miniscript one before BIP-84's, 86's, 49's, 44's."""
    descs = []
    for d in json.loads(text)["descriptors"]:
        if not descsum_check(d["desc"]):
            raise ValueError("Core: a descriptor's checksum is wrong")
        descs.append(d["desc"].split("#")[0])

    def rank(d):
        for i, p in enumerate(["wpkh(", "tr(", "sh(wpkh(", "pkh("], 1):
            if d.startswith(p) and (p != "tr(" or "," not in d):
                return i
        return 0
    pairs = sorted((d for d in descs if "/0/*" in d and d.replace("/0/*", "/1/*") in descs),
                   key=rank)
    if not pairs:
        raise ValueError("no receive and change pair")
    d = pairs[0]
    receive, change = core_scripts(to_core(d), to_core(d.replace("/0/*", "/1/*")))
    return {"keys": keys_stated(d), "receive": receive, "change": change}


# ----- the files other wallets wrote (the area's reading of each format) -------------------

POLICY_WRAP = {"P2WSH": "wsh({})", "P2SH-P2WSH": "sh(wsh({}))", "P2WSH-P2SH": "sh(wsh({}))",
               "P2SH": "sh({})"}
# A key's SLIP-132 prefix, the script it names (SLIP-132's table).
VERSION_SHAPE = {"x": "pkh", "t": "pkh", "y": "sh-wpkh", "u": "sh-wpkh", "z": "wpkh", "v": "wpkh"}
VERSION_WRAP = {"x": "sh({})", "t": "sh({})", "Y": "sh(wsh({}))", "U": "sh(wsh({}))",
                "Z": "wsh({})", "V": "wsh({})"}
PURPOSE_SHAPE = {"44": "pkh", "49": "sh-wpkh", "84": "wpkh", "86": "tr"}


def origin_of(fp, path):
    path = re.sub(r"['H]", "h", path).removeprefix("m")
    return f"[{fp.lower()}{path}]"


def from_template(template, keys, tail="/<0;1>/*"):
    """The wallet a template over keys (each `[fp/path]key`, any SLIP-132
    version) is, by Core."""
    desc = fill(template, [k + tail for k in keys])
    receive, change = core_scripts(to_core(desc.replace("/<0;1>/*", "/0/*")),
                                   to_core(desc.replace("/<0;1>/*", "/1/*")))
    return {"keys": keys_stated(desc), "receive": receive, "change": change}


def read_descriptor_text(text):
    """Sparrow's, Jade's, Keystone's and Casa's descriptors: one with
    `<0;1>`, a receive and a change line (either way round, labelled or
    not), or one chain alone, which stands for both. A checksum, where
    there is one, is Core's code's to call."""
    descs = []
    for line in text.strip().splitlines():
        line = re.sub(r"^[A-Za-z ]+:\s*", "", line.strip())
        if not line:
            continue
        if "#" in line:
            if not descsum_check(line, require=True):
                raise ValueError("Core: a descriptor's checksum is wrong")
            line = line.split("#")[0]
        descs.append(line)
    if not descs:
        raise ValueError("no descriptor")
    if len(descs) == 2:
        a, b = descs
        if a.replace("/0/*", "/1/*") != b:
            raise ValueError("two descriptors that are not a receive and a change line")
        descs = [a]
    if len(descs) != 1:
        raise ValueError("more than two descriptors")
    d = descs[0]
    if "/<0;1>/*" in d:
        r, c = d.replace("/<0;1>/*", "/0/*"), d.replace("/<0;1>/*", "/1/*")
    elif "/0/*" in d:
        r, c = d, d.replace("/0/*", "/1/*")
    elif "/1/*" in d:
        r, c = d.replace("/1/*", "/0/*"), d
    elif not any(m.end() < len(d) and d[m.end()] == "/" for m in KEY.finditer(d)):
        # Keys without a derivation are the two chains (as Specter's are).
        r, c = (KEY.sub(lambda m: m.group(0) + tail, d) for tail in ("/0/*", "/1/*"))
    else:
        raise ValueError("no chain in the descriptor")
    receive, change = core_scripts(to_core(r), to_core(c))
    return {"keys": keys_stated(d), "receive": receive, "change": change}


def read_key_line(text):
    """`[fp/path]key` alone: a single-key wallet of the script the key's
    SLIP-132 version names."""
    m = KEY.fullmatch(text.strip())
    if not m or not m.group(1):
        raise ValueError("not a key with its origin")
    key = m.group(3)
    shape = VERSION_SHAPE.get(key[0])
    if shape is None or (key[0] in "xt" and not m.group(2)):
        raise ValueError("a key whose version names no script")
    if key[0] in "xt":
        shape = PURPOSE_SHAPE[m.group(2).split("/")[1].rstrip("'hH")]
    return from_template(SINGLE[shape][1], [origin_of(m.group(1), m.group(2)) + key])


def read_multisig_file(text):
    """Coldcard's multisig setup file, as Coldcard, Sparrow, Electrum and
    Nunchuk write it: a policy, a format, a derivation before the keys or
    before each, then `fingerprint: key`."""
    policy = fmt = None
    deriv = None
    keys = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        name, _, value = line.partition(":")
        value = value.strip()
        if name == "Policy":
            m = re.fullmatch(r"(\d+) of (\d+)", value)
            if not m:
                raise ValueError("a policy that is not M of N")
            policy = (int(m.group(1)), int(m.group(2)))
        elif name == "Format":
            fmt = value.upper()
        elif name == "Derivation":
            deriv = value
        elif re.fullmatch(r"[0-9A-Fa-f]{8}", name):
            if deriv is None:
                raise ValueError("a key with no derivation")
            keys.append(origin_of(name, deriv.removeprefix("m")) + value)
    if policy is None or fmt not in POLICY_WRAP:
        raise ValueError("no policy or no format")
    m, n = policy
    if n != len(keys) or not 1 <= m <= n:
        raise ValueError("the policy does not match the keys listed")
    template = POLICY_WRAP[fmt].format(
        f"sortedmulti({m}," + ",".join(f"@{i}" for i in range(n)) + ")")
    return from_template(template, keys)


def read_caravan(text):
    v = json.loads(text)
    keys = v["extendedPublicKeys"]
    q = v["quorum"]
    if q["totalSigners"] != len(keys) or not 1 <= q["requiredSigners"] <= len(keys):
        raise ValueError("the quorum does not match the keys listed")
    out = []
    for k in keys:
        if not k.get("bip32Path") or not k.get("xfp"):
            raise ValueError("a key with no path or fingerprint")
        out.append(origin_of(k["xfp"], k["bip32Path"]) + k["xpub"])
    template = POLICY_WRAP[v["addressType"]].format(
        f"sortedmulti({q['requiredSigners']}," + ",".join(f"@{i}" for i in range(len(out))) + ")")
    return from_template(template, out)


def read_cobo(v):
    """Cobo's and Keystone's single-key file: the script comes from the
    path's purpose."""
    if "AccountKeyPath" in v:
        path = v["AccountKeyPath"].removeprefix("m/")
    elif "ColdCardFirmwareVersion" in v:  # Coldcard's file for Wasabi: BIP-84 account 0
        path = "84'/" + ("0'" if v["ExtPubKey"][0] in "xyz" else "1'") + "/0'"
    else:
        raise ValueError("a key file with no account path")
    shape = PURPOSE_SHAPE[path.split("/")[0].rstrip("'hH")]
    return from_template(SINGLE[shape][1], [origin_of(v["MasterFingerprint"], "/" + path)
                                            + v["ExtPubKey"]])


def electrum_fp(store):
    if "root_fingerprint" in store:
        return store["root_fingerprint"]
    if "ckcc_xfp" in store:
        return int(store["ckcc_xfp"]).to_bytes(4, "little").hex()
    raise ValueError("a key with no fingerprint")


def read_electrum(text):
    v = json.loads(text)
    if v.get("use_encryption") or "keystore" not in v and "x1/" not in v:
        raise ValueError("an encrypted or empty Electrum file")
    if v["wallet_type"] == "standard":
        k = v["keystore"]
        if k["type"] not in ("bip32", "hardware"):
            raise ValueError("a keystore that is not an extended public key")
        shape = VERSION_SHAPE[k["xpub"][0]]
        if k["xpub"][0] in "xt":
            shape = PURPOSE_SHAPE[k["derivation"].removeprefix("m/").split("/")[0].rstrip("'")]
        return from_template(SINGLE[shape][1],
                             [origin_of(electrum_fp(k), k["derivation"]) + k["xpub"]])
    m, n = (int(x) for x in v["wallet_type"].split("of"))
    keys = [v[f"x{i}/"] for i in range(1, n + 1)]
    out = [origin_of(electrum_fp(k), k["derivation"]) + k["xpub"] for k in keys]
    template = VERSION_WRAP[keys[0]["xpub"][0]].format(
        f"sortedmulti({m}," + ",".join(f"@{i}" for i in range(n)) + ")")
    return from_template(template, out)


def read_real(text):
    """A file another wallet wrote, by what it is: the area's reading of
    each format (a file in none of them is not a wallet)."""
    t = text.strip()
    if t.startswith("BSMS"):
        return read_bsms(text)
    if t.startswith("addwallet "):
        return read_specter_diy(t)
    if t.startswith("{"):
        v = json.loads(t)
        if "descriptors" in v:
            return read_listdescriptors(text)
        if "descriptor" in v:
            return read_descriptor_text(v["descriptor"])
        if "quorum" in v:
            return read_caravan(text)
        if "ExtPubKey" in v:
            return read_cobo(v)
        if "xfp" in v and "chain" in v:
            return read_coldcard(text)
        if "wallet_type" in v or "x1/" in v or "keystore" in v:
            return read_electrum(text)
        raise ValueError("a JSON file with no wallet in it")
    if re.search(r"^(Policy|Format|Derivation):", t, re.M):
        return read_multisig_file(text)
    if KEY.fullmatch(t):
        return read_key_line(t)
    if t.startswith("ur:") or t.startswith("sp(") or "spscan" in t:
        raise ValueError("not a descriptor")
    return read_descriptor_text(text)


# ----- BIP 129's key records -------------------------------------------------------------

def header_address(sec, net, header):
    """The address a BIP-137 header names for a key: its form decides it."""
    if header < 31:
        sec = bytes_from_point(point_from_octets_var(sec), compressed=False)
        return b58.p2pkh(PubKeyData(sec, net))
    pub = PubKeyData(sec, net)
    if header < 35:
        return b58.p2pkh(pub)
    if header < 39:
        return b58.p2wpkh_p2sh(pub)
    return b32.p2wpkh(pub)


def btclib_verifies(text):
    """btclib's verdict on a key record's signature, over its four lines,
    for the record's key in the form its header names."""
    lines = text.rstrip("\n").split("\n")
    signed = "\n".join(lines[:4])
    key = KEY.search(lines[2]).group(3)
    data = bip32.BIP32KeyData.b58decode(key)
    net = "mainnet" if plain(key).startswith("xpub") else "testnet"
    try:
        raw = base64.b64decode(lines[4], validate=True)
        if len(raw) != 65 or not 27 <= raw[0] <= 42:
            return False
        return bms.verify(signed.encode(), header_address(data.key, net, raw[0]), lines[4])
    except Exception:  # noqa: BLE001 - every failure to verify is one
        return False


def record_rules(text):
    """The rules of a key record Faraday holds that no outside code has."""
    lines = text.rstrip("\n").split("\n")
    if lines[0] != "BSMS 1.0" or len(lines) != 5:
        raise ValueError("BIP 129: a key record is five lines under BSMS 1.0")
    token = lines[1]
    if not (len(token) in (16, 32) and re.fullmatch(r"[0-9a-fA-F]+", token)) and token != "00":
        raise Ours("a token that is not 00 or a 64- or 128-bit nonce in hex")
    if len(lines[3]) > 80:
        raise Ours("a description over BIP 129's eighty characters")
    if not KEY.fullmatch(lines[2].strip()) or not KEY.fullmatch(lines[2].strip()).group(1):
        raise Ours("a key that is not [fingerprint/path]xpub")


def signed_record(account, token, description, mark="h"):
    four = f"BSMS 1.0\n{token}\n{account.origin(mark)}{account.pub}\n{description}"
    return four + "\n" + bms.sign(four.encode(), account.secret()).b64encode()


def pre_blocks():
    text = BIP129.read_text()
    return [b for b in re.findall(r"<pre>(.*?)</pre>", text, re.S) if b.startswith("BSMS 1.0")]


DESCRIPTION_CHARS = "abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.:é"


def lower_fingerprints(answer):
    """A fingerprint is four bytes, however a file spells them: Faraday
    states one in the case the file used."""
    if answer[0] != "ok":
        return answer
    value = dict(answer[1])
    value["keys"] = [re.sub(r"^\[([0-9A-Fa-f]{8})", lambda m: "[" + m.group(1).lower(), k)
                     for k in value["keys"]]
    return ("ok", value)


class Import(Area):
    name = "import"
    CORE = True
    LOOPS = (("wallet", 300), ("signer", 150))
    faraday = (
        "`faraday_intake::wallet::read_wallet` (the wallet a file or text is: descriptors, "
        "`faraday_bip::coldcard`, `faraday_bip::multisig_config`, Caravan's, Electrum's, "
        "Cobo's, Keystone's and Wasabi's files, Specter DIY's `addwallet`, "
        "`faraday_bip::bsms::DescriptorRecord`), and "
        "`faraday_bip::bsms::SignerRecord` read and verified and `faraday_bip::bsms::signer_record` "
        "over the BIP-48 account node's key"
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest, offline (`deriveaddresses`), and its "
        "`descriptors.py` (BIP-380's checksum) and `address.py`, for each wallet's receive and "
        "change scripts and every checksum; btclib-wallet 2026.10.29's BIP-32 (btclib's own "
        "curve) for every key, btclib 2026.10.24's SLIP-132 versions and addresses, and its "
        "`bms` for every key record's signature, made, verified and written byte for byte; "
        "CPython's `json` for the value of every JSON file read and for a key named twice and "
        "a nesting past 32 refused (those two rules are the reader's own, written beside it); "
        "the files are written in the area from each format's documentation, and the meaning "
        "of their fields is the area's"
    )
    reference_checked = (
        "Core's release tarball against its pinned SHA-256 before every run; btclib's wheels "
        "against the hashes btclib-wallet's lock file names (`references.sha256`), with "
        "libsecp256k1 off; BIP 129's own records and the real files of Sparrow, BlueWallet "
        "(Nunchuk's, Coldcard's and Unchained's among them), Coldcard, Caravan and Liana, judged "
        "in the prelude"
    )
    vectors = (
        "BIP 129's key and descriptor records (`tools/vectors/bip129/`), and 93 real files from "
        "the wallets' own test suites, 62 of them a wallet and 31 refused by both sides "
        "(`faraday/faraday-core/tests/vectors/interop/`: Sparrow's and BlueWallet's; "
        "`tools/vectors/wallet-files/`: Coldcard's firmware tests, Caravan's and Liana's): "
        "BSMS records, Coldcard setup files and exports, Caravan, Electrum, Cobo, Keystone, "
        "Jade, Specter Desktop and Specter DIY files, descriptors and a Core `listdescriptors`"
    )
    corpus = (
        "single-key wallets (P2PKH, P2SH-P2WPKH, P2WPKH, P2TR), `sortedmulti` in P2WSH, "
        "P2SH-P2WSH and P2SH of one to fifteen keys, a miniscript in P2WSH, a taproot tree "
        "and a taproot `sortedmulti_a`, on mainnet and testnet keys, each as a descriptor "
        "(multipath with and without checksum, receive and change lines, labelled, receive "
        "alone, Core's `listdescriptors`, Specter Desktop's file, Specter DIY's `addwallet` "
        "with and without its keys' derivations) or as the files that "
        "state its kind (Coldcard's export and multisig file, Caravan's, Electrum's, Cobo's, "
        "Wasabi's, a key alone, a BSMS descriptor record), in BIP-32's or SLIP-132's "
        "spelling; damaged by a wrong checksum, a wrong first address, a SLIP-132 version "
        "of another script or another paths line; key records signed by btclib in each "
        "header form, with or without damage, and Faraday's own record of random seeds"
    )
    differences = [
        (
            "Still ours, expected from Faraday's rule: a SLIP-132 version that names another "
            "script than the file's (F155), a BSMS paths line other than /0/*,/1/* or No path "
            "restrictions, which Faraday reads as the two chains (F64), a BSMS descriptor that "
            "is not a template, a key record whose description is over eighty characters or "
            "whose token is not BIP 129's; and which account of a Coldcard export, which pair "
            "of a `listdescriptors`, and what each field of every format means, the area's "
            "reading of each format's documentation; a checksum in Specter DIY's `addwallet`, "
            "which Specter DIY drops unread and Faraday checks, is refused when Core's code "
            "calls it wrong; and the "
            "reading of each format's documentation."
        ),
        (
            "Real files both sides refuse, each by a named rule: a Caravan file or an Electrum "
            "multisig file whose keys lack a path or a fingerprint, a Caravan quorum that is not "
            "the keys listed, a multisig setup file with no policy, a key missing or no format "
            "line (Coldcard's own P2SH setup files omit Format, which Faraday refuses), a BSMS "
            "record with a paths line other than /0/*,/1/*, a derivation other than /** or "
            "<0;1>, a missing line, a first address that is not Core's or not an address, a "
            "Coldcard cosigner key file, an Unchained key file and a Coldcard stored setting "
            "(a key, not a wallet), and a UR text (area `ur`). Left out and named: a silent "
            "payments record (area `sp`) and a PSBT or transaction file (areas `psbt`, `tx`)."
        ),
        (
            "Formats with no published file in a test suite: Nunchuk's own files (only "
            "BlueWallet's copies of them), Specter DIY's beyond the one Sparrow keeps, "
            "Keystone's beyond Sparrow's, and Core's `listdescriptors` beyond the one in the "
            "interop vectors, whose source is not recorded; Electrum's published test wallets "
            "carry a seed and are not taken. Those formats stay the area's own reading "
            "(Faraday's second version), as above."
        ),
        (
            "A key record's signature in any of BIP-137's header forms is accepted when it "
            "recovers the record's key: btclib is asked with the address that form names "
            "(Faraday reads no address from the record)."
        ),
    ]
    not_compared = (
        "the wallet's name a file carries; Electrum's encrypted files and BSMS's encrypted "
        "records, which Faraday refuses by name; a silent payments record; the kind of a "
        "refusal"
    )

    # ----- one comparison ----------------------------------------------------------------

    def read(self, text, ref):
        raw = self.compare("import.read", {"text": text, "n": N}, ref, normalize=lower_fingerprints)
        if text.lstrip("\ufeff \t\r\n")[:1] in ("{", "["):
            self.json(text)
        return raw

    def json(self, text):
        """Faraday's JSON reader and CPython's `json` on the same text:
        the same value, or both refuse (X5)."""
        return self.compare("import.json", {"text": text}, lambda: {"value": json_reference(text)})

    def verify(self, text, expect=None):
        def ref():
            record_rules(text)
            return {"valid": btclib_verifies(text)}
        raw = self.compare("import.verify", {"text": text}, ref)
        if expect is not None and raw != ("ok", {"valid": expect}):
            raise HarnessError(f"a key record the BIP or the area made answered {raw}: {text}")
        return raw

    # ----- the prelude -------------------------------------------------------------------

    def prelude(self):
        """BIP 129's records and other wallets' files; then every shape in
        every format once."""
        blocks = pre_blocks()
        keys = [b for b in blocks if len(b.rstrip("\n").split("\n")) == 5]
        descs = [b for b in blocks if len(b.rstrip("\n").split("\n")) == 4]
        assert len(keys) == 9 and len(descs) == 4, "BIP 129's records are not where they were"
        good = 0
        for text in keys:
            raw = self.verify(text)
            if raw == ("ok", {"valid": True}):
                good += 1
                self.verify(text.replace(" key\n", " key.\n"), expect=False)
        assert good == 7, f"{good} of BIP 129's key records verify"
        for text in descs:
            self.read(text, lambda t=text: read_bsms(t))
        self.real_files()
        self.json_cases()
        # Faraday's own record, of both networks and BIP-48 scripts.
        for net in ("mainnet", "testnet"):
            for script in (1, 2):
                self.need(self.write_case(net, script, desc="Signer 1 key"), "a record written")
        for kind, formats in FORMATS.items():
            shapes = {"single": SINGLE, "multi": MULTI, "mini": MINI}[kind]
            for shape in shapes:
                for fmt in formats:
                    if self.fits(shape, fmt):
                        self.wallet_case(shape, fmt, damage=False)

    def real_files(self):
        """Every file under the interop vectors, read by Faraday and by
        the area's reading of its format, judged by Core: the wallet, or
        a refusal on both sides. Left out, named: a silent payments
        record (area `sp`) and a PSBT or transaction (areas `psbt`,
        `tx`)."""
        read = 0
        found = [p for d in (INTEROP, WALLET_FILES) for p in d.rglob("*") if p.is_file()]
        for f in sorted(p for p in found if p.name not in (
                "outcomes.txt", "README.md", "LICENSE", "LICENCE", "COPYING-CC")):
            if f.suffix in (".psbt", ".txn") or "-sp-" in f.name:
                continue
            t = f.read_text()
            self.read(t, lambda t=t: read_real(t))
            read += 1
        assert read >= 90, f"{read} of the other wallets' files read"

    # ----- JSON --------------------------------------------------------------------------

    def json_cases(self):
        """The reader's own rules, beside CPython's: a key named twice
        (written two ways too), the depth of 32, the grammar's refusals, the
        value of a float written each way, and random nestings of both kinds
        around the limit."""
        rng = self.rng
        for text in [
                '{"a":1,"a":2}', '{"a":1,"\\u0061":2}', '{"a":{"b":1},"b":{"b":2}}',
                '{"x":[{"a":1,"a":1}]}', '\ufeff {"a":[1e3,-0.5,12,-7,1.0]} \n',
                '"\\ud83d\\ude00"', '"\\ud800"', '"\\ud800\\u0041"', '[NaN]', '[Infinity]',
                '[1,]', '{"a":1,}', "{'a':1}", "01", "1.", ".5", "-", "1e", "+1", "tru", "nul",
                '"a', '"\\x"', '"\\u12"', '"tab\there"', '{"a" 1}', "{1:2}", "[1 2]", "1 2",
                "{} x", "", "{", "[[]", "[]]", "1e400", "-1e400", "[1e308]", "[1e309]",
                "5e111", "5" + "0" * 111,
        ]:
            self.json(text)
        for _ in range(60):
            f = rng.uniform(1, 10) * 10.0 ** rng.randrange(-300, 300)
            self.json(rng.choice([repr(f), format(f, ".17e"), format(f, "f")]))
        for _ in range(60):
            n = rng.randrange(JSON_MAX_DEPTH - 4, JSON_MAX_DEPTH + 5)
            inner = rng.choice(["1", '"s"', "null", "{}", "[]"])
            parts = [rng.choice(["[", '{"k":']) for _ in range(n)]
            close = ["]" if p == "[" else "}" for p in parts]
            self.json("".join(parts) + inner + "".join(reversed(close)))
        for _ in range(30):
            keys = [rng.choice(["a", "b", "c", "\\u0061", "\\u0062"]) for _ in range(rng.randrange(1, 6))]
            body = ",".join(f'"{k}":{rng.randrange(10)}' for k in keys)
            around = rng.choice([("", ""), ('{"z":[', "]}"), ('{"z":[1,', "]}")])
            self.json(around[0] + "{" + body + "}" + around[1])

    # ----- wallets -----------------------------------------------------------------------

    @staticmethod
    def fits(shape, fmt):
        if fmt == "electrum" and shape == "tr":
            return False
        if fmt == "keyline":
            return shape in ("wpkh", "sh-wpkh")
        if fmt == "wasabi":
            return shape == "wpkh"
        return True

    def case_wallet(self):
        rng = self.rng
        kind = rng.choices(["single", "multi", "mini"], [4, 5, 2])[0]
        shape = rng.choice(list({"single": SINGLE, "multi": MULTI, "mini": MINI}[kind]))
        fmt = rng.choice([f for f in FORMATS[kind] if self.fits(shape, f)])
        self.wallet_case(shape, fmt, damage=rng.random() < 0.15)

    def wallet_case(self, shape, fmt, damage):
        rng = self.rng
        net = rng.choice(["mainnet", "testnet"])
        w = Wallet(self, shape, net, acct=0 if fmt == "wasabi" else None)
        text, ref = getattr(self, "file_" + fmt.replace("-", "_"))(w, damage)
        self.read(text, ref)

    def descriptor_text(self, w, fmt, damage):
        multi = w.desc("/<0;1>/*")
        recv, change = checked(w.desc("/0/*")), checked(w.desc("/1/*"))
        whole = checked(multi)
        damage = damage and fmt != "bare"
        if damage:
            # A checksum one character off, which Core's code calls wrong.
            recv, whole = flip_checksum(recv), flip_checksum(whole)
            assert not descsum_check(recv, require=True)
            assert not descsum_check(whole, require=True)
        if fmt == "specter-diy":
            return self.specter_diy_text(w, damage)
        if fmt == "multipath":
            text = whole
        elif fmt == "bare":
            text = multi
        elif fmt == "pair":
            text = recv + "\n" + change
        elif fmt == "labelled":
            text = "Receive: " + recv + "\nChange: " + change
        elif fmt == "receive":
            text = recv
        elif fmt == "core":
            entries = [{"desc": d, "timestamp": 1791392643, "active": True, "internal": i,
                        "range": [0, 999], "next": 0, "next_index": 0}
                       for i, d in ((False, recv), (True, change))]
            if self.rng.random() < 0.5:
                entries.reverse()
            text = json.dumps({"wallet_name": "w", "descriptors": entries}, indent=2)
        else:  # Specter Desktop's wallet file
            text = json.dumps({"label": "Wallet", "blockheight": 0, "descriptor": recv,
                               "devices": [{"type": "other", "label": f"K{i}"}
                                           for i in range(len(w.accounts))]})
        if damage:
            return text, self.refused("Core's descsum_check: the checksum is wrong")
        return text, w.expected

    def specter_diy_text(self, w, damage):
        """Specter DIY's `addwallet`, as Sparrow writes it (a single-key
        or multisig wallet's keys without a derivation, no checksum) or
        with the two chains and a checksum; a name that may hold `&`."""
        rng = self.rng
        bare = w.kind != "mini" and rng.random() < 0.7
        d = w.desc("") if bare else w.desc("/<0;1>/*")
        if damage or rng.random() < 0.3:
            d = checked(d)
            if damage:
                d = flip_checksum(d)
        name = "".join(rng.choice(NAME_CHARS) for _ in range(rng.randrange(1, 30))).strip()
        text = f"addwallet {name or 'W'}&{d}"
        return text, lambda t=text: read_specter_diy(t)

    @staticmethod
    def refused(why, kind=ValueError):
        def ref():
            raise kind(why)
        return ref

    def file_multipath(self, w, damage):
        return self.descriptor_text(w, "multipath", damage)

    def file_bare(self, w, damage):
        return self.descriptor_text(w, "bare", damage)

    def file_pair(self, w, damage):
        return self.descriptor_text(w, "pair", damage)

    def file_labelled(self, w, damage):
        return self.descriptor_text(w, "labelled", damage)

    def file_receive(self, w, damage):
        return self.descriptor_text(w, "receive", damage)

    def file_core(self, w, damage):
        return self.descriptor_text(w, "core", damage)

    def file_specter(self, w, damage):
        return self.descriptor_text(w, "specter", damage)

    def file_specter_diy(self, w, damage):
        return self.descriptor_text(w, "specter-diy", damage)

    def file_coldcard(self, w, damage):
        """Coldcard's generic export: some of the four accounts of one
        master, each with its first address; the one read is the first of
        bip84, bip86, bip49, bip44 there."""
        rng = self.rng
        seed = w.accounts[0].seed
        fields = [SINGLE[w.shape][2]] + rng.sample(
            [x[2] for s, x in SINGLE.items() if s != w.shape], rng.randrange(0, 4))
        chosen = min(fields, key=COLDCARD_ORDER.index)
        file = {"chain": "BTC" if w.net == "mainnet" else "XTN",
                "xfp": w.accounts[0].fp.upper(), "account": w.acct}
        wallets = {}
        for field in fields:
            shape = next(s for s, x in SINGLE.items() if x[2] == field)
            other = w if shape == w.shape else Wallet(self, shape, w.net, w.acct, seed)
            wallets[field] = other
            a = other.accounts[0]
            file[field] = {"name": field, "deriv": "m/" + a.path.replace("h", "'"),
                           "xpub": a.pub, "first": other.first()}
        if damage:
            # Another address than the account's first: Core's receive 1.
            file[chosen]["first"] = address(wallets[chosen].scripts()[0][1], w.net)
            return json.dumps(file), self.refused("the stated first address is not Core's")
        if rng.random() < 0.2:
            del file[chosen]["first"]
        return json.dumps(file, indent=rng.choice([None, 2])), wallets[chosen].expected

    def file_cobo(self, w, damage):
        a = w.accounts[0]
        version = SINGLE[w.shape][3] if self.rng.random() < 0.5 else "bip32_pub"
        if version is None:
            version = "bip32_pub"
        file = {"ExtPubKey": spelled(a.pub, w.net, version),
                "MasterFingerprint": a.fp.upper(),
                "AccountKeyPath": a.path.replace("h", "'")}
        return json.dumps(file), w.expected

    def file_wasabi(self, w, damage):
        a = w.accounts[0]
        file = {"MasterFingerprint": a.fp.upper(),
                "ExtPubKey": spelled(a.pub, w.net, self.rng.choice(
                    ["bip32_pub", "slip132_p2wpkh_pub"])),
                "ColdCardFirmwareVersion": "5.2.2"}
        return json.dumps(file), w.expected

    def file_keyline(self, w, damage):
        a = w.accounts[0]
        key = spelled(a.pub, w.net, SINGLE[w.shape][3])
        if self.rng.random() < 0.3:
            keys = [plain(key)]
            return key, lambda: w.expected(keys)
        return a.origin(self.rng.choice(["h", "'"])) + key, w.expected

    def file_electrum(self, w, damage):
        rng = self.rng
        if w.kind == "single":
            a = w.accounts[0]
            file = {"keystore": {"type": "bip32", "xpub": spelled(a.pub, w.net, SINGLE[w.shape][3]),
                                 "derivation": "m/" + a.path.replace("h", "'"),
                                 "root_fingerprint": a.fp},
                    "wallet_type": "standard", "use_encryption": False, "seed_version": 17}
            return json.dumps(file), w.expected
        version = MULTI[w.shape][3]
        file = {"wallet_type": f"{w.m}of{len(w.accounts)}", "use_encryption": False,
                "seed_version": 17}
        for i, a in enumerate(w.accounts, 1):
            store = {"type": rng.choice(["bip32", "hardware"]),
                     "xpub": spelled(a.pub, w.net, version),
                     "derivation": "m/" + a.path.replace("h", "'")}
            if rng.random() < 0.5:
                store["root_fingerprint"] = a.fp
            else:
                store["ckcc_xfp"] = int.from_bytes(bytes.fromhex(a.fp), "little")
            file[f"x{i}/"] = store
        return json.dumps(file), w.expected

    def file_config(self, w, damage, each=False):
        """Coldcard's multisig file, which Sparrow, Nunchuk and Specter
        write: one Derivation line, or one before each key."""
        rng = self.rng
        path, _, fmt, version = MULTI[w.shape]
        lines = ["# Exported from a coordinator", "Name: Wallet",
                 f"Policy: {w.m} of {len(w.accounts)}", f"Format: {fmt}"]
        if each:
            coin = 0 if w.net == "mainnet" else 1
            for a in w.accounts:
                a.__init__(self, w.net, path.format(coin=coin, acct=rng.randrange(0, 9)), a.seed)
            w._scripts = None
        else:
            lines.append("Derivation: m/" + w.accounts[0].path.replace("h", "'"))
        lines.append("")
        spell = version if rng.random() < 0.5 else "bip32_pub"
        wrong = None
        if damage and version != "bip32_pub":
            # A key in the version of another script (F155): ours.
            wrong = rng.choice([v for v in ("slip132_p2wpkh_pub", "slip132_p2wsh_pub",
                                            "slip132_p2wsh_p2sh_pub") if v != version])
        for i, a in enumerate(w.accounts):
            if each:
                lines.append("Derivation: m/" + a.path.replace("h", rng.choice(["h", "'"])))
            v = wrong if wrong and i == len(w.accounts) - 1 else spell
            lines.append(f"{a.fp.upper()}: {spelled(a.pub, w.net, v)}")
        text = "\n".join(lines) + "\n"
        if wrong:
            return text, self.refused("a SLIP-132 version of another script (F155)", Ours)
        return text, w.expected

    def file_config_each(self, w, damage):
        return self.file_config(w, damage, each=True)

    def file_caravan(self, w, damage):
        file = {"name": "Wallet", "addressType": MULTI[w.shape][2],
                "network": w.net, "client": {"type": "public"},
                "quorum": {"requiredSigners": w.m, "totalSigners": len(w.accounts)},
                "extendedPublicKeys": [
                    {"name": f"K{i}", "bip32Path": "m/" + a.path.replace("h", "'"),
                     "xpub": a.pub, "xfp": a.fp} for i, a in enumerate(w.accounts)],
                "startingAddressIndex": 0}
        return json.dumps(file, indent=2), w.expected

    def file_bsms(self, w, damage):
        """BIP 129's descriptor record: the template over `/**`, its paths,
        Core's first address."""
        rng = self.rng
        mark = rng.choice(["h", "'"])
        keys = [a.origin(mark) + a.pub for a in w.accounts]
        if rng.random() < 0.3:
            # BIP 129's own example uses multi(): the keys in their order.
            w.template = w.template.replace("sortedmulti(", "multi(")
            w._scripts = None
        desc = fill(w.template, [k + "/**" for k in keys])
        paths = "/0/*,/1/*" if rng.random() < 0.85 else "No path restrictions"
        first = w.first()
        if damage:
            if rng.random() < 0.5:
                first = address(w.scripts()[0][1], w.net)
                text = f"BSMS 1.0\n{desc}\n{paths}\n{first}\n"
                return text, lambda t=text: read_bsms(t)
            paths = rng.choice(["/0/*", "/0/*,/2/*", "/1/*,/0/*"])
        text = f"BSMS 1.0\n{desc}\n{paths}\n{first}" + rng.choice(["", "\n"])
        return text, lambda t=text: read_bsms(t)

    # ----- key records -------------------------------------------------------------------

    def case_signer(self):
        rng = self.rng
        if rng.random() < 0.35:
            return self.write_case()
        net = rng.choice(["mainnet", "testnet"])
        coin = 0 if net == "mainnet" else 1
        a = Account(self, net, f"48h/{coin}h/{rng.randrange(0, 4)}h/{rng.choice([1, 2])}h")
        token = rng.choice(["00", "00", self.randbytes(8).hex(), self.randbytes(16).hex().upper()])
        desc = "".join(rng.choice(DESCRIPTION_CHARS) for _ in range(rng.randrange(0, 81)))
        text = signed_record(a, token, desc, rng.choice(["h", "'"]))
        lines = text.split("\n")
        how = rng.choices(["whole", "form", "description", "token", "other", "byte", "short",
                           "long", "badtoken"], [5, 3, 1, 1, 1, 1, 0.5, 0.5, 0.5])[0]
        raw = bytearray(base64.b64decode(lines[4]))
        if how == "form":
            raw[0] = rng.choice([27, 35, 39]) + (raw[0] - 31)
        elif how == "description":
            lines[3] = lines[3][:-1] if lines[3] else "x"
        elif how == "token":
            lines[1] = "00" if lines[1] != "00" else self.randbytes(8).hex()
        elif how == "other":
            # The four lines signed by another account's key.
            b = Account(self, net, a.path)
            four = "\n".join(lines[:4]).encode()
            raw = bytearray(base64.b64decode(bms.sign(four, b.secret()).b64encode()))
        elif how == "byte":
            raw[rng.randrange(1, 65)] ^= 1 << rng.randrange(8)
        elif how == "short":
            raw = raw[:64]
        elif how == "long":
            lines[3] = desc + "z" * (81 - len(desc))
            lines[4] = signed_record(a, token, lines[3]).split("\n")[4]
        elif how == "badtoken":
            lines[1] = rng.choice(["0", "01", "zz", self.randbytes(4).hex()])
        if how not in ("long",):
            lines[4] = base64.b64encode(bytes(raw)).decode()
        self.verify("\n".join(lines), expect=True if how in ("whole", "form") else None)

    def write_case(self, net=None, script=None, desc=None):
        rng = self.rng
        net = net or rng.choice(["mainnet", "testnet"])
        seed = self.randbytes(rng.choice([16, 32, 64]))
        script, account = script or rng.choice([1, 2]), rng.randrange(0, 5)
        token = rng.choice(["00", "00", self.randbytes(8).hex()])
        if desc is None:
            desc = "".join(rng.choice(DESCRIPTION_CHARS)
                           for _ in range(rng.choice([rng.randrange(0, 81), 81])))
        coin = 0 if net == "mainnet" else 1

        def ref():
            if len(desc) > 80:
                raise Ours("a description over BIP 129's eighty characters")
            a = Account(self, net, f"48h/{coin}h/{account}h/{script}h", seed)
            return {"text": signed_record(a, token, desc)}
        return self.compare("import.write", {"seed": seed.hex(), "network": net, "script": script,
                                      "account": account, "token": token,
                                      "description": desc}, ref)

