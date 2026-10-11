# EntropyLab's vanity counter

`docs/PLANNING.md` §16.117. The counter this device appends to a
passphrase is EntropyLab's, so that a find here and a find there are the
same find. The alphabet, the odometer order and the two vectors below
are read from the source named here, never from a summary.

Nothing is built, run or vendored from that repository into this one.
It was cloned to read and to run once, for the two vectors in §3, and
only this page is kept.

| Source | URL | Commit | Fetched |
|---|---|---|---|
| EntropyLab | <https://github.com/OogaBoogaX/entropylab> | `e113d1ec0a43d45403287b724a55c83fa5699917` | 2026-09-19 |

| File read | SHA-256 |
|---|---|
| `vanity-wasm/src/lib.rs` | `aa555794827540563dfa95d950f5c70768bf751aa9a49533d2d29043154c62ed` |
| `src/js/vanity.js` | `bac24c7aa7aa5a523648a2077f54a4aceb139d6143f6201c32aace8f051d4914` |

## 1. The alphabet and the order

`vanity-wasm/src/lib.rs`, quoted as the file holds it:

```rust
/// The passphrase alphabet, in the user-facing order a-zA-Z0-9.
const ALPHABET: &[u8; 62] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
```

The counter is a fixed-width odometer over that alphabet, laid out most
significant first and stepped least significant last:

```rust
    // Odometer digits (indexes into ALPHABET), most significant first.
    let mut digit = [0u8; MAX_PASS_LEN];
    if mode == MODE_PASSPHRASE {
        let mut c = start;
        for i in (0..pass_len).rev() {
            digit[i] = (c % 62) as u8;
            c /= 62;
        }
    }
```

```rust
        // Increment the odometer (least significant character last).
        if mode == MODE_PASSPHRASE {
            let mut i = pass_len;
            while i > 0 {
                i -= 1;
                digit[i] += 1;
                if digit[i] < 62 {
                    break;
                }
                digit[i] = 0;
            }
        }
```

The candidate passphrase is the key's own passphrase followed by that
string, and the address is derived the standard way:

```rust
                let mut seed = bip39_seed(mnemonic, &[salt, &pass[..pass_len]]);
```

The width is EntropyLab's own question: its screen asks for a passphrase
length, and one run grinds one width. `osk_bip::vanity` asks no such
question, so it grinds width 1 first (62 counters), then width 2 (3 844),
and so on. Inside a width the two orders are the same counter.

## 2. What the prefix may be

`src/js/vanity.js`, quoted:

```js
const VANITY_BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const VANITY_BECH32_ALPHABET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
```

and the expected work, which is the arithmetic behind this device's
"Expected" row:

```js
// Expected candidates per matching address: each free base58 character is one
// of 58 possibilities; each free bech32 character is one of 32 (or, for the
// Silent Payment parity character, one of 8).
export function estimateVanityWork(prefix, script = "p2wpkh") {
  const meta = vanityScript(script);
  const free = Math.max(0, String(prefix ?? "").length - meta.prefix.length);
  if (!free) return 1n;
  const per = BigInt(meta.bech32 ? 32 : 58);
  return meta.firstFree ? BigInt(meta.firstFree.length) * per ** BigInt(free - 1) : per ** BigInt(free);
}
```

`meta.firstFree` is EntropyLab's name for a first free place that holds
fewer than the whole character set. EntropyLab has one such case, a
silent payment code's parity character; this device has one of its own,
a legacy address on a test network, which begins with `m` or `n`.

## 3. The two vectors

Both were produced by running EntropyLab's `vanity-wasm` once, here, on
2026-09-19: the committed WebAssembly from `src/js/vanity-wasm-b64.js`
at the commit above, instantiated under Node v24.21.0 and called through
the same arguments `src/js/vanity-worker.js` passes. The key is the
twelve words of BIP-39's zero entropy, with no starting passphrase, and
the script type is native SegWit on mainnet.

```
mnemonic  abandon abandon abandon abandon abandon abandon abandon abandon
          abandon abandon abandon about
prefix    bc1qq
```

| Dial | EntropyLab call | First match | Address |
|---|---|---|---|
| Passphrase | `mode 0`, `path 84'/0'/0'/0/0`, `passLen 1`, `start 0` | counter `8`, odometer `i` | `bc1qqv2kx2d9tv4d3epztk59dc5kymkl8pk8scu6xz` |
| Account index | `mode 1`, master node, `path 84'/0'/0'/0/0`, `counterSlot 2`, `start 0` | counter `31` | `bc1qqpat9khft6dnm9qp0nnrvpyvmyg2ytshn7gglv` |

The grinder returns the address as a payload — HASH160 for the
hash-based scripts — and the two above are
`03156329a55b2ad8e4225da856e29626edf386c7` and
`007ab2dae95e9b3d94017ce636048cd910a22e17`, encoded with EntropyLab's
own `addressFromScript`. `core/osk-bip/tests/vanity.rs` asserts both
counters and both addresses.
