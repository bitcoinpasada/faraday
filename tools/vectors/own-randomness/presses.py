"""The known answers of whole presses (`docs/OWN-RANDOMNESS.md` §8.5 test
3), written from §2.3 and the published BIP-39 and Seed XOR definitions
over Python's standard library alone, never from Faraday's output.

Run from the top of the repository: `python3 -I -B
tools/vectors/own-randomness/presses.py` prints the `presses` key of
`vectors.json`, each case's `device_words` in it.

The system's answers a press takes are the ones it is given here
(`faraday/faraday-core/tests/all/own_randomness.rs`): a GPG key's two
seeds are fresh answers for "gpg primary" (the answer sent last) and
"gpg subkey" (the one before), Seed XOR's random part is a fresh answer
for "seed xor", and This device's words are the first 16 bytes of the
SHA-256 of its answer (the accumulator mixed in, when the dice were
added), as 12 BIP-39 words.

A MuSig2 round's public nonce (Sign here on `musig-unsigned.psbt` with
test key 1, the device seeded with 32 bytes of 0x05) is BIP-327's
NonceGen over the reference's `nonce_gen_internal`, with the key, the
aggregate and the message worked out by `bitcoin.py` from the BIPs
(`musig-unsigned.psbt` is the test stick's file; `bitcoin.py --check`
shows each part agrees with the published vectors). The `rand'` is the
rule of Faraday's own specification: the session's seed (the system's
answer, hashed with the person's accumulator by "faraday reseed" when
they added some), then the signing pass's seed, SHA-256("faraday sign
seed" || seed || the pass's number as 8 bytes, little-endian), the
first pass; then BIP-340's tagged hash "Faraday/musig-rand" of that seed,
the input's index (4 bytes, little-endian) and the participant's
compressed key. The 66 bytes are the two nonce points, compressed.
"""
import importlib.util
import json
from hashlib import sha256
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FILE = ROOT / "tools/vectors/own-randomness/vectors.json"
WORDS = (ROOT / "tools/reference/bip39/mnemonic/wordlist/english.txt").read_text().split()
assert len(WORDS) == 2048


_spec = importlib.util.spec_from_file_location(
    "own_randomness_bitcoin", ROOT / "tools/vectors/own-randomness/bitcoin.py"
)
btc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(btc)


def H(*p):
    return sha256(b"".join(p)).digest()


def fresh(what, answer, acc):
    return H(b"faraday fresh", what, answer, acc or b"")


def indices(entropy):
    bits = "".join(f"{b:08b}" for b in entropy)
    bits += f"{H(entropy)[0]:08b}"[: len(entropy) // 4]
    return [int(bits[i:i + 11], 2) for i in range(0, len(bits), 11)]


def entropy_of(indices_):
    bits = "".join(f"{i:011b}" for i in indices_)
    n = len(bits) * 32 // 33
    ent = bytes(int(bits[i:i + 8], 2) for i in range(0, n, 8))
    assert indices(ent) == list(indices_), "a checksum that does not hold"
    return ent


def phrase(entropy):
    return " ".join(WORDS[i] for i in indices(entropy))


def musig_pubnonce(entropy, acc):
    """This device's public nonce on input 0 of `musig-unsigned.psbt`:
    test key 1, a device seeded with `entropy`, the person's accumulator
    `acc` (or None), the first Sign here."""
    psbt = btc.Psbt((ROOT / "tools/vectors/own-randomness/musig-unsigned.psbt").read_bytes())
    _, keys, _, steps = btc.musig_input_of(psbt)
    root = btc.master(btc.bip39_seed(" ".join(["bacon"] * 24)))
    account = btc.derive(root, "m/86h/1h/0h")
    assert account.pub in keys
    aggpk, _ = btc.musig_address(keys, steps)
    utxos = [i["utxo"] for i in psbt.inputs]
    hash_type = psbt.inputs[0].get("sighash", btc.DEFAULT)
    assert hash_type == btc.DEFAULT, "MuSig2 signs SIGHASH_DEFAULT"
    msg = btc.sighash(psbt.tx, utxos, 0, hash_type)
    seed = entropy if acc is None else H(b"faraday reseed", entropy, acc)
    seed = H(b"faraday sign seed", seed, (1).to_bytes(8, "little"))
    rand = btc.tagged("Faraday/musig-rand", seed + (0).to_bytes(4, "little") + account.pub)
    _, pub = btc.ref.nonce_gen_internal(rand, account.secret(), account.pub, aggpk, msg, None)
    assert len(pub) == 66
    return pub.hex()


def main():
    # The reference's checks first: a changed reference, spend or vector
    # file fails here instead of printing a new answer.
    btc.check_all()
    v = json.loads(FILE.read_text())
    a, b = bytes([0x61]) * 32, bytes([0x62]) * 32
    # The key the Seed XOR presses split: test key 1, "bacon" 24 times.
    seed = entropy_of([WORDS.index("bacon")] * 24)
    r = bytes([0x75]) * 32
    entropy = bytes([5]) * 32
    out = {}
    for name, case in v["cases"].items():
        acc = bytes.fromhex(case["acc"][-1]) if case["acc"] else None
        dev = bytes.fromhex(case["device_own"]) if "device_own" in case \
            else bytes.fromhex(v["seed_answer"])
        if name not in ("nothing added", "dice"):
            continue
        part = fresh(b"seed xor", r, acc)
        out[name] = {
            "device_words": " ".join(map(str, indices(H(dev)[:16]))),
            "answers": {"gpg": [a.hex(), b.hex()], "seed_xor": r.hex(),
                        "musig_entropy": entropy.hex()},
            "gpg_primary_seed": fresh(b"gpg primary", b, acc).hex(),
            "gpg_subkey_seed": fresh(b"gpg subkey", a, acc).hex(),
            "musig_pubnonce": musig_pubnonce(entropy, acc),
            "seed_xor_2": [phrase(part), phrase(bytes(x ^ y for x, y in zip(part, seed)))],
        }
    print(json.dumps(out, indent=1, ensure_ascii=False))


main()
