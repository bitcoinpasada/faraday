"""btclib-wallet's descriptors and miniscript (`tools/reference/btclib/`:
`btclib_wallet.descriptors`), set up for the areas that judge a wallet's
descriptor: `addr` (an account's descriptor) and `miniscript` (the
wallets of every shape, and the recovery wallets).

btclib is set up as `bip32_ref.py` sets it up, on its own curve. What
btclib answers here is its own: the descriptor read and refused
(`read`), the script it pays at an index of either chain (`script_at`),
the kind of descriptor it is (`shape`), the text it writes back
(`text_of`), the checksum, and the spend of the wallet's coins: whether
a set of signatures, at a given nSequence and nLockTime, satisfies it
(`spends`). A miniscript on its own is `miniscript()`: its type, whether
it is sane, its script.

Nothing here is a model of a standard: the signatures are made-up bytes
(btclib assembles a satisfaction and does not verify it), the lock times
are numbers given to its `SpendContext`, and every answer is btclib's.
btclib's `script/solver.py` and `script/spendability.py` are ports of
Core (ESTABLISHED §9.2 rule 3) and are not used.
"""

import bip32_ref  # noqa: F401 - unpacks btclib, on its own curve
from btclib_wallet import descriptors as bd
from btclib_wallet.descriptors import miniscript as bm

P2WSH, TAPSCRIPT = bm.P2WSH, bm.TAPSCRIPT
Refused = Exception  # btclib's BTClibValueError and its relatives; any refusal


def network_name(network):
    """btclib's name: its mainnet or its testnet, which signet and regtest share."""
    return "mainnet" if network == "mainnet" else "testnet"


def read(text, network):
    """The descriptor btclib reads from `text`, checksum checked when
    there is one; raises when btclib refuses it."""
    return bd.parse(text, network_name(network))


def chains(text, network):
    """The descriptors of the two chains of a multipath descriptor
    (BIP-389), each read."""
    name = network_name(network)
    return [bd.parse(one, name) for one in bd.multipath_descriptors(text, name)]


def texts(text, network):
    """The text btclib writes for each chain of a multipath descriptor."""
    return [text_of(d) for d in chains(text, network)]


def split(text):
    """A descriptor written with `<0;1>` as its two chains' texts
    (BIP-389's expansion, for the one pair Faraday writes)."""
    return [text.replace("<0;1>", "0"), text.replace("<0;1>", "1")]


def script_at(text, network, change, index):
    """The script the wallet pays at `index` of one chain, hex."""
    one = chains(text, network)
    return one[int(change)].script_pub_key(index).script.hex()


def shape(d):
    """The functions of a descriptor from the outside in, with the script
    in a `wsh()` or a `tr()` named for what it is: `wsh(miniscript)`,
    `sh(wsh(sortedmulti))`, `tr(tree)`, `tr` for a key alone."""
    if isinstance(d, bd.PkhDescriptor):
        return "pkh"
    if isinstance(d, bd.WpkhDescriptor):
        return "wpkh"
    if isinstance(d, bd.ShDescriptor):
        return f"sh({shape(d.inner)})"
    if isinstance(d, bd.WshDescriptor):
        return f"wsh({shape(d.inner)})"
    if isinstance(d, bd.MultiDescriptor):
        return "sortedmulti" if d.sort else "multi"
    if isinstance(d, bd.MiniscriptDescriptor):
        return "miniscript"
    if isinstance(d, bd.TrDescriptor):
        return "tr" if d.tree is None else "tr(tree)"
    return type(d).__name__


def text_of(d):
    """The descriptor as btclib writes it, a hardened step written `h`
    (btclib keeps the spelling it read, and the checksum is over it)."""
    return str(d).replace("'", "h")


def checksum(text):
    """btclib's own BIP-380 checksum of a descriptor without one."""
    return bd.checksum(text)


def checksum_of(text):
    """The descriptor with btclib's checksum, as the areas' other
    checksum makers write it."""
    return f"{text}#{bd.checksum(text)}"


def verify(text):
    """Whether a descriptor ends in a checksum that is its own, by
    btclib's `strip_checksum` (which also takes one with none)."""
    try:
        bd.strip_checksum(text)
    except Exception:  # noqa: BLE001 - any refusal of btclib
        return False
    return "#" in text


def miniscript(text, tap):
    """btclib's reading of a miniscript with raw keys: (its type
    properties the area compares, its script), or raises."""
    node = bm.parse(text, TAPSCRIPT if tap else P2WSH)
    return {
        "valid": node.is_valid_top_level,
        "non_malleable": node.is_non_malleable,
        "mixes": node.mixes_timelocks,
        "sane": node.is_sane(),
        "signature": node.is_signature_required,
        "script": node.script().hex(),
        "text": str(node),
    }


# ----- spending ------------------------------------------------------------

SIGHASH_ALL = b"\x01"
# A DER signature (r and s of 32 bytes with the top bit clear) and a
# 64-byte Schnorr one; btclib assembles a witness from them and does not
# verify.
ECDSA = bytes.fromhex("3044022001" + "02" * 31 + "022001" + "03" * 31) + SIGHASH_ALL
SCHNORR = bytes(range(64))


def keys_of(d, index, network, texts):
    """For each of Faraday's key texts (`[origin]xpub`), btclib's key
    (SEC) at `index`: the key expression of the descriptor that names
    that extended key."""
    name = network_name(network)
    found = {}
    for expression in d.key_expressions:
        written = str(expression)
        for i, text in enumerate(texts):
            if text.rsplit("]", 1)[-1] in written:
                found[i] = expression.sec(index, name)
    return found


def spends(d, index, network, keys, signing, sequence, locktime, tap):
    """Whether btclib's satisfaction of the descriptor `d` (one chain) at
    `index` succeeds when the keys numbered in `signing` have signed and
    the transaction carries `sequence` and `locktime` (version 2)."""
    signature = SCHNORR if tap else ECDSA
    signatures = {keys[i]: signature for i in signing}
    context = bd.SpendContext(locktime=locktime, sequence=sequence, version=2)
    try:
        d.satisfy(signatures, index=index, spend=context)
    except Exception:  # noqa: BLE001 - any refusal is "does not spend"
        return False
    return True
