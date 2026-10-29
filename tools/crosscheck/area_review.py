"""What Faraday shows before signing a PSBT, against Bitcoin Core v31.1
itself (bitcoind on regtest, offline; `core_node.py`).

Every PSBT is made with Core: `createpsbt` for the unsigned transaction,
`createrawtransaction` and `decoderawtransaction` for the previous
transactions, `descriptorprocesspsbt` over the wallet's public descriptors
for the key origins and scripts of the inputs and the outputs, every
address derived from `tprv` descriptors with `deriveaddresses` (no
`btc.py`). The area adds the UTXO fields of the inputs, splices a spoofed
output, and derives each registered wallet's account keys (Core derives
the same addresses from them, or the area stops).

Faraday's review of each PSBT is compared with:

- `decodepsbt` and `analyzepsbt`: every input's amount (witness UTXO, or
  the previous transaction's output), the money in and out, the fee (Core
  states none when an input has no UTXO data, Faraday states zero and
  warns), which inputs lack their UTXO data.
- `deriveaddresses`: which outputs are change. An output is change when
  it claims the origins of a key Faraday holds and the script Core derives
  from the wallet's keys at the path the output states is the output's
  own; it is a spoof when it claims them and Core derives another script.
  A script of several keys, a miniscript or a taproot tree is change only
  where its wallet is registered with Faraday.

Wallets: single-key accounts of each address type (P2PKH, P2SH-P2WPKH,
P2WPKH, P2TR) and registered wallets of a `sortedmulti` in P2WSH, a
miniscript in P2WSH, a taproot tree of miniscript leaves and a taproot
`sortedmulti_a`. Outputs: payments, change on either chain at random
indices, change of another wallet (a foreign claim), and spoofed change
(the wallet's origin claimed over the wallet's own script at another
index, over another wallet's script, or over a stranger's). Inputs:
UTXO data as a witness UTXO or a previous transaction, or none.
"""

import base64
import hashlib
import hmac
import re
import struct
from decimal import Decimal

import core_node
from common import Area, use_reference

use_reference("core")
from test_framework.address import byte_to_base58  # noqa: E402
from test_framework.authproxy import JSONRPCException  # noqa: E402
from test_framework.descriptors import descsum_create  # noqa: E402
from test_framework.key import ECKey  # noqa: E402
from test_framework.script import hash160  # noqa: E402

NUMS = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0"
MAGIC = b"psbt\xff"
INDICES = 6  # change and inputs use indices 0 to 5 of either chain

# shape: (account path, template, keys, a script of several keys)
SHAPES = {
    "pkh": ("44h/1h/0h", "pkh({0})", 1, False),
    "sh-wpkh": ("49h/1h/0h", "sh(wpkh({0}))", 1, False),
    "wpkh": ("84h/1h/0h", "wpkh({0})", 1, False),
    "tr": ("86h/1h/0h", "tr({0})", 1, False),
    "sortedmulti": ("48h/1h/0h/2h", "wsh(sortedmulti(2,{0},{1},{2}))", 3, True),
    "miniscript": ("48h/1h/0h/2h", "wsh(or_d(pk({0}),and_v(v:pkh({1}),older(52560))))", 2, True),
    "tree": ("86h/1h/0h", "tr({0},{{and_v(v:pk({1}),older(4320)),pk({2})}})", 3, True),
    "tapmulti": ("48h/1h/0h/2h", "tr(" + NUMS + ",sortedmulti_a(2,{0},{1},{2}))", 3, True),
}
SHAPE_NAMES = list(SHAPES)
# Throwaway keys for payments: one of each address type.
PAYEE_TYPES = ["pkh({})", "sh(wpkh({}))", "wpkh({})", "tr({})"]
KINDS = ["change", "recipient", "foreign", "spoof"]
SPOOF_OVER = ["own", "foreign", "stranger"]


# ----- PSBT maps, only to add a field or splice a map ---------------------------------

def ser_compact(n):
    if n < 0xfd:
        return bytes([n])
    return b"\xfd" + n.to_bytes(2, "little") if n <= 0xFFFF else b"\xfe" + n.to_bytes(4, "little")


def read_compact(b, at):
    n = b[at]
    if n < 0xfd:
        return n, at + 1
    size = {0xfd: 2, 0xfe: 4}[n]
    return int.from_bytes(b[at + 1:at + 1 + size], "little"), at + 1 + size


def read_map(b, at):
    pairs = []
    while True:
        n, at = read_compact(b, at)
        if n == 0:
            return pairs, at
        key, at = b[at:at + n], at + n
        m, at = read_compact(b, at)
        pairs.append((key, b[at:at + m]))
        at += m


def write_map(pairs):
    return b"".join(ser_compact(len(k)) + k + ser_compact(len(v)) + v for k, v in pairs) + b"\0"


def read_psbt(raw, n_in, n_out):
    assert raw[:5] == MAGIC
    glob, at = read_map(raw, 5)
    ins, outs = [], []
    for _ in range(n_in):
        m, at = read_map(raw, at)
        ins.append(m)
    for _ in range(n_out):
        m, at = read_map(raw, at)
        outs.append(m)
    assert at == len(raw)
    return glob, ins, outs


def write_psbt(glob, ins, outs):
    return MAGIC + write_map(glob) + b"".join(write_map(m) for m in ins + outs)


# ----- Bitcoin Core ----------------------------------------------------------------------

def batch(calls):
    """Each (method, params) asked of Core in one HTTP request; the results in
    order. A Core error here is the harness's (it builds only valid
    requests)."""
    node = core_node.rpc()
    reqs = [getattr(node, method).get_request(*params) for method, params in calls]
    by_id = {r["id"]: r for r in node.batch(reqs)}
    out = []
    for q in reqs:
        r = by_id[q["id"]]
        if r.get("error"):
            raise JSONRPCException(r["error"])
        out.append(r["result"])
    return out


def sats(value):
    return int(Decimal(str(value)) * 10**8)


def btc(n):
    return format(Decimal(n) / Decimal(10**8), "f")


def tprv_of(seed):
    i = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    raw = bytes.fromhex("04358394") + bytes(9) + i[32:] + b"\0" + i[:32]
    return byte_to_base58(raw[1:], raw[0])


def checked(text):
    return descsum_create(text)


N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141


def pubkey(secret):
    k = ECKey()
    k.set(secret.to_bytes(32, "big"), True)
    return k.get_pubkey().get_bytes()


def account_node(seed, path):
    """The extended public key at a hardened `path` below the seed's master,
    as a `tpub`, with the master's fingerprint. Only the wallet's setup: the
    account key a registered wallet names, which Core cannot be asked for.
    It is not what is compared: Core derives every address from the `tprv`
    itself, and `Wallet.check` requires the `tpub` it gives to derive the
    same addresses."""
    i = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    k, c = int.from_bytes(i[:32], "big"), i[32:]
    fp = hash160(pubkey(k))[:4]
    parent, index = fp, 0
    for step in path.split("/"):
        index = int(step[:-1]) + 0x80000000
        parent = hash160(pubkey(k))[:4]
        i = hmac.new(c, b"\0" + k.to_bytes(32, "big") + index.to_bytes(4, "big"),
                     hashlib.sha512).digest()
        k, c = (int.from_bytes(i[:32], "big") + k) % N, i[32:]
    tail = bytes([len(path.split("/"))]) + parent + index.to_bytes(4, "big") + c
    pub = bytes.fromhex("043587cf") + tail + pubkey(k)
    prv = bytes.fromhex("04358394") + tail + b"\0" + k.to_bytes(32, "big")
    return fp.hex(), byte_to_base58(pub[1:], pub[0]), byte_to_base58(prv[1:], prv[0])


class Wallet:
    """A wallet of one shape over fresh masters: the private descriptors
    Core derives from, and the public ones (and the registered policy) a
    review reads."""

    def __init__(self, area, shape):
        self.shape = shape
        self.acct, self.tmpl, n, self.multi = SHAPES[shape]
        self.seeds = [area.randbytes(area.rng.choice([16, 32, 64])) for _ in range(n)]
        self.tprvs = [tprv_of(s) for s in self.seeds]
        nodes = [account_node(s, self.acct) for s in self.seeds]
        self.fps = [fp for fp, _, _ in nodes]
        self.account_tprvs = [tprv for _, _, tprv in nodes]
        # Core refuses a miniscript over several keys derived from a master
        # by hardened steps (it takes them for one key); it is given the
        # account's own `tprv` under its origin instead.
        self.from_account = shape == "miniscript"
        origins = [f"[{fp}/{self.acct}]{tpub}" for fp, tpub, _ in nodes]
        self.pub = [checked(self.tmpl.format(*[f"{o}/{c}/*" for o in origins])) for c in (0, 1)]
        self.policy = None
        if self.multi:
            self.policy = "\n".join(
                [self.tmpl.format(*[f"@{i}/**" for i in range(n)])] + origins)

    def key(self, j, tail):
        """Key `j` of the wallet, as a `tprv` expression with `tail` below its
        account."""
        if self.from_account:
            return f"[{self.fps[j]}/{self.acct}]{self.account_tprvs[j]}/{tail}"
        return f"{self.tprvs[j]}/{self.acct}/{tail}"

    def private(self, chain):
        return checked(self.tmpl.format(*[self.key(j, f"{chain}/*") for j in range(len(self.fps))]))

    def at(self, claims):
        """The wallet's descriptor with each key at the path its claim states."""
        depth = len(self.acct.split("/"))
        keys = [self.key(j, "/".join(claims[fp][2:].split("/")[depth:]))
                for j, fp in enumerate(self.fps)]
        return checked(self.tmpl.format(*keys))


class Review(Area):
    name = "psbt-review"
    CORE = True
    LOOPS = (("corpus", 200),)
    faraday = (
        "`faraday_psbt::inspect` (amounts in and out, the fee, verified change and the "
        "spoofed-change and missing-UTXO warnings), over `faraday_psbt::Psbt` and the keys "
        "and registered wallets the Sign screen holds"
    )
    reference = (
        "Bitcoin Core v31.1's `bitcoind` on regtest, offline: `decodepsbt` and `analyzepsbt` "
        "for amounts and fee, `deriveaddresses` from `tprv` descriptors for which outputs are "
        "change, `createpsbt`, `createrawtransaction` and `descriptorprocesspsbt` to make "
        "the PSBTs; the area adds each input's UTXO fields, splices a spoofed output, derives "
        "the account keys a registered wallet names (checked by Core deriving the same "
        "addresses from them), and applies the rule that an output is change when it claims "
        "a held key's origins and Core's script at the path it states is the output's own"
    )
    reference_checked = (
        "Core's release tarball is checked against its pinned SHA-256 "
        "(`tools/reference/core/release.pin`) before every run, and the node's version is "
        "asked; every PSBT is one Core made and read, and every change decision is one of "
        "Core's derived scripts"
    )
    vectors = (
        "none: Core's own decoding and derivation are the reference, over each wallet shape "
        "with change, spoofed change, a foreign claim and missing UTXO data in the prelude"
    )
    corpus = (
        "PSBTs of one to three inputs and one to four outputs over single-key accounts "
        "(P2PKH, P2SH-P2WPKH, P2WPKH, P2TR) and registered wallets (`sortedmulti` in P2WSH, "
        "a miniscript in P2WSH, a taproot tree, a taproot `sortedmulti_a`) or the same "
        "wallets unregistered: witness UTXOs, previous transactions and no UTXO data, fees "
        "from nothing to a third of the inputs, payments, change on either chain, another "
        "wallet's change, and spoofed change over the wallet's own script at another index, "
        "another wallet's and a stranger's; one key held, two, or none of the wallet's"
    )
    differences = [
        (
            "Change of a script of several keys, a miniscript or a taproot tree: Faraday "
            "verifies it only for a registered wallet that holds a loaded key, and counts it "
            "as paid to others otherwise, which Core has no notion of. The area expects the "
            "same refusal, and does not compare the spoof warning for an unregistered wallet "
            "(Faraday's reading of such a claim depends on the account path it states)."
        ),
    ]
    not_compared = (
        "Faraday's other warnings (high fee, dust, address reuse, sighash, network, "
        "locktime) and thresholds, which are Faraday's own rules; the estimated virtual size "
        "and fee rate (Core's estimate needs signing data the PSBT does not carry)"
    )

    def prelude(self):
        """Every shape once with each output kind, once with missing UTXO
        data, and once unregistered; and the cases must have had what they
        were drawn to have (the area cannot pass by asking nothing)."""
        self.seen = {"ours": 0, "others": 0, "spoof": 0, "missing": 0, "fee": 0, "unregistered": 0}
        for shape in SHAPE_NAMES:
            self.run_case(shape, kinds=KINDS + ["change"], loaded="first", registered=True,
                          modes=["nonwitness" if shape == "pkh" else "witness", "nonwitness"])
            self.run_case(shape, kinds=["change", "recipient"], loaded="first",
                          registered=True, modes=["none", "nonwitness"])
            if SHAPES[shape][3]:
                self.run_case(shape, kinds=["change", "spoof"], loaded="first",
                              registered=False,
                              modes=["witness"])
                self.run_case(shape, kinds=["change"], loaded="none", registered=True,
                              modes=["witness"])
        n = len(SHAPE_NAMES)
        for what, least in [("ours", 2 * n), ("others", 2 * n), ("spoof", n), ("missing", n)]:
            assert self.seen[what] >= least, f"the prelude drew {self.seen} (wanted {what} >= {least})"

    def case_corpus(self):
        self.run_case(self.rng.choice(SHAPE_NAMES))

    # ----- one PSBT -------------------------------------------------------------------

    def run_case(self, shape, kinds=None, loaded=None, registered=None, modes=None):
        rng = self.rng
        w1, w2 = Wallet(self, shape), Wallet(self, shape)
        stranger = tprv_of(self.randbytes(32))

        n_in = rng.randrange(1, 4) if modes is None else len(modes)
        if modes is None:
            legacy = shape == "pkh"
            pick = ["nonwitness", "none"] if legacy else ["witness", "nonwitness", "none"]
            weights = [5, 3] if legacy else [5, 3, 1.5]
            modes = rng.choices(pick, weights, k=n_in)
        if kinds is None:
            kinds = rng.choices(KINDS, [4, 4, 1, 2], k=rng.randrange(1, 5))
        if loaded is None:
            loaded = rng.choices(["first", "two", "none"], [6, 2, 1])[0]
        if registered is None:
            registered = rng.random() < 0.75
        held_at = {"first": [0], "two": [0, 1] if len(w1.seeds) > 1 else [0], "none": []}[loaded]
        seeds = [w1.seeds[i] for i in held_at] or [self.randbytes(32)]
        held_fps = {w1.fps[i] for i in held_at}
        wallets = [w1.policy] if registered and w1.policy else []

        # Addresses: both wallets' on both chains, and a payee of each type.
        found = batch(
            [("deriveaddresses", [w.private(c), [0, INDICES - 1]]) for w in (w1, w2) for c in (0, 1)]
            + [("deriveaddresses", [w.pub[c], [0, INDICES - 1]]) for w in (w1, w2) for c in (0, 1)]
            + [("deriveaddresses", [checked(t.format(f"{stranger}/9h/*")), [0, 3]])
               for t in PAYEE_TYPES])
        own, foreign, payees = found[0:2], found[2:4], found[8:]
        # The public descriptors the review reads derive what Core derives
        # from the private ones.
        assert found[0:4] == found[4:8], "an account key the area derived is not Core's"

        def payee():
            return rng.choice(payees)[rng.randrange(4)]

        # The outputs: the address each claims, the address it pays.
        # (Core refuses one address twice in a transaction: draw again.)
        while True:
            claim, real = [], []
            for kind in kinds:
                c, i = rng.randrange(2), rng.randrange(INDICES)
                if kind == "change":
                    claim.append(own[c][i])
                    real.append(own[c][i])
                elif kind == "recipient":
                    claim.append(a := payee())
                    real.append(a)
                elif kind == "foreign":
                    claim.append(foreign[c][i])
                    real.append(foreign[c][i])
                else:
                    over = rng.choice(SPOOF_OVER)
                    other = {"own": own[c][(i + 1) % INDICES] if rng.random() < 0.5
                             else own[1 - c][i],
                             "foreign": foreign[c][i], "stranger": payee()}[over]
                    claim.append(own[c][i])
                    real.append(other)
            if len(set(claim)) == len(claim) and len(set(real)) == len(real):
                break

        # Inputs: scripts of the wallet, and the value each states.
        in_at = [(rng.randrange(2), rng.randrange(INDICES)) for _ in range(n_in)]
        in_value = [rng.randrange(20_000, 60_000_000) for _ in range(n_in)]
        total = sum(in_value)
        fee = rng.choice([0, rng.randrange(0, 5_000), rng.randrange(0, total // 3 + 1)])
        left = total - fee
        if len(kinds) > 1:
            cuts = sorted(rng.sample(range(1, left), len(kinds) - 1))
        else:
            cuts = []
        amounts = [b - a for a, b in zip([0] + cuts, cuts + [left])]
        spks = batch([("validateaddress", [own[c][i]]) for c, i in in_at])
        spks = [bytes.fromhex(s["scriptPubKey"]) for s in spks]
        txids = [self.randbytes(32)[::-1].hex() for _ in range(n_in)]
        vouts = [rng.randrange(0, 4) for _ in range(n_in)]
        prevs = {}
        for i, mode in enumerate(modes):
            if mode == "nonwitness":
                vouts[i] = rng.randrange(0, 3)
                prevs[i] = (vouts[i],)
        if prevs:
            made = batch([
                ("createrawtransaction", [
                    [{"txid": self.randbytes(32).hex(), "vout": 0}],
                    [{a: btc(rng.randrange(1000, 10**6))}
                     for a in rng.sample([x for p in payees for x in p], vouts[i])]
                    + [{own[in_at[i][0]][in_at[i][1]]: btc(in_value[i])}]])
                for i in prevs])
            decoded = batch([("decoderawtransaction", [h]) for h in made])
            for (i, _), h, d in zip(prevs.items(), made, decoded):
                txids[i] = d["txid"]
                prevs[i] = bytes.fromhex(h)
        locktime = rng.choice([0, rng.randrange(1, 900_000)])

        def psbt_of(addresses):
            ins = [{"txid": t, "vout": v} for t, v in zip(txids, vouts)]
            outs = [{a: btc(x)} for a, x in zip(addresses, amounts)]
            return batch([("createpsbt", [ins, outs, locktime])])[0]


        draft = base64.b64decode(psbt_of(claim))
        glob, ins, outs = read_psbt(draft, n_in, len(kinds))
        for i, mode in enumerate(modes):
            if mode == "witness":
                ins[i].append((b"\x01", struct.pack("<q", in_value[i]) + ser_compact(len(spks[i]))
                               + spks[i]))
            elif mode == "nonwitness":
                ins[i].append((b"\x00", prevs[i]))
        filled = batch([("descriptorprocesspsbt", [
            base64.b64encode(write_psbt(glob, ins, outs)).decode(),
            [{"desc": d, "range": [0, INDICES - 1]} for d in w1.pub + w2.pub],
            "DEFAULT", True, False])])[0]["psbt"]
        _, ins, outs = read_psbt(base64.b64decode(filled), n_in, len(kinds))
        glob, _, _ = read_psbt(base64.b64decode(psbt_of(real)), n_in, len(kinds))
        raw = write_psbt(glob, ins, outs)

        inputs = {"psbt": raw.hex(), "seeds": [s.hex() for s in seeds], "xprvs": [],
                  "network": "regtest", "wallets": wallets}
        skip = bool(w1.multi and not registered)

        def ref():
            return self.core_review(raw, w1, held_fps, registered or not w1.multi, skip,
                                    base64.b64encode(raw).decode())

        def norm(answer):
            if skip and answer[0] == "ok":
                return ("ok", {k: v for k, v in answer[1].items() if k != "change_spoof"})
            return answer
        self.compare("review.inspect", inputs, ref, normalize=norm)

    # ----- Bitcoin Core's reading --------------------------------------------------------

    def core_review(self, raw, w1, held_fps, registered, skip, b64):
        decoded, analyzed = batch([("decodepsbt", [b64]), ("analyzepsbt", [b64])])
        tx = decoded["tx"]
        amounts = []
        for inp, vin in zip(decoded["inputs"], tx["vin"]):
            if "witness_utxo" in inp:
                amounts.append(sats(inp["witness_utxo"]["amount"]))
            elif "non_witness_utxo" in inp:
                amounts.append(sats(inp["non_witness_utxo"]["vout"][vin["vout"]]["value"]))
            else:
                amounts.append(None)
        missing = any(not i.get("has_utxo") for i in analyzed["inputs"])
        assert missing == any(a is None for a in amounts)
        fee = sats(analyzed["fee"]) if "fee" in analyzed else 0
        # Which outputs claim a held key, and the script Core derives at the
        # path each claims.
        todo = []
        for n, out in enumerate(decoded["outputs"]):
            claims = {d["master_fingerprint"]: d["path"] for d in out.get("bip32_derivs", [])}
            claims.update({d["master_fingerprint"]: d["path"]
                           for d in out.get("taproot_bip32_derivs", [])})
            if held_fps & set(claims) and all(fp in claims for fp in w1.fps):
                todo.append((n, w1.at(claims)))
        derived = batch([("deriveaddresses", [d]) for _, d in todo])
        verdict = {}
        for (n, _), got in zip(todo, derived):
            same = got[0] == tx["vout"][n]["scriptPubKey"]["address"]
            verdict[n] = "ours" if same else "spoof"
        outputs, spoof = [], False
        for n, vout in enumerate(tx["vout"]):
            v = verdict.get(n)
            ours = v == "ours" and registered
            spoof = spoof or (v == "spoof" and not skip)
            outputs.append({"amount": sats(vout["value"]), "ours": ours})
        out_total = sum(o["amount"] for o in outputs)
        seen = getattr(self, "seen", None)
        if seen is not None:
            seen["ours"] += sum(o["ours"] for o in outputs)
            seen["others"] += sum(not o["ours"] for o in outputs)
            seen["spoof"] += spoof
            seen["missing"] += missing
        return {
            "fee": fee,
            "total_in": sum(a for a in amounts if a is not None),
            "total_out": out_total,
            "to_others": sum(o["amount"] for o in outputs if not o["ours"]),
            "change": sum(o["amount"] for o in outputs if o["ours"]),
            "inputs": amounts,
            "outputs": outputs,
            "missing_utxo": missing,
            "change_spoof": spoof,
        }


