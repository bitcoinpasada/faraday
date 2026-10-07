# ur — in-house since 2026-09-11

**Purpose.** BC-UR (BCR-2020-005) encoding and decoding of animated QR
payloads: `ur:crypto-psbt` in and out of the Sign flow and the scanner
(`osk_codec::ur`, `docs/PLANNING.md` §7, §8.4 #1/#12, §16.20, §16.61).

**This is not a dependency page any more.** The `ur` 0.5.2 crate is gone
from the workspace. `core/osk-codec/src/ur/` is the whole transport,
`no_std` + `alloc`, `#![forbid(unsafe_code)]`, with no dependency but
`osk-crypto`'s SHA-256:

| File | What it holds |
|---|---|
| `bytewords.rs` | BCR-2020-012's 256-word table, the minimal style, and CRC-32/ISO-HDLC bitwise |
| `xoshiro.rs` | xoshiro256\*\*, `next_double`, `next_int`, `shuffled`, and Vose's alias sampler for the degree |
| `fountain.rs` | `fragment_length`, `partition`, `choose_fragments`, the XOR encoder, the reducing decoder, and the part's CBOR |
| `mod.rs` | the `ur:<type>[/<seq>-<count>]/<bytewords>` text, the CBOR byte-string wrapper, and the public API |

## What the crate cost

Nine crates for 1,687 lines of its own non-test code: `ur`,
`bitcoin_hashes` 1.x (a second major beside the 0.14 line `bitcoin` 0.32
uses) with `bitcoin-internals` and `bitcoin-consensus-encoding` behind
it, `crc`, `crc-catalog`, `minicbor`, `rand_xoshiro` and `rand_core`.
`minicbor`'s derive macro also kept `syn`, `quote`, `proc-macro2` and
`unicode-ident` in every device graph after `rqrr` stopped needing them.
Measured with
`cargo tree -p <root> -e normal --prefix none | sed 's/ (.*//' | sort -u | wc -l`:

| Graph | Before | After |
|---|--:|--:|
| `opensigner-core` | 60 | 51 |
| `opensigner-ffi` | 63 | 54 |
| `opensigner-pi` | 64 | 55 |
| `opensigner-desktop` | 137 | 128 |

`bitcoin_hashes` was there to SHA-256 eight bytes of fountain seed;
`osk-crypto` already had SHA-256. `crc` and `crc-catalog` were there for
one bitwise CRC-32. `minicbor` was there for a five-element array of four
integers and a byte string.

## What had to be reproduced exactly

A coordinator's stream and the device's stream have to be the same bytes,
or Sparrow, Nunchuk and BlueWallet cannot read what the device shows and
the device cannot finish what they show. Four details decide it, and none
is stated plainly in the specification:

- **The part's CBOR** is an *untagged* five-element array —
  `[seqNum, seqLen, messageLen, checksum, data]` — each integer in its
  shortest unsigned form and the data a definite-length byte string.
- **The fragment chooser.** Parts `1..=n` are the fragments themselves.
  After that the seed is `seqNum` and `checksum`, each 32-bit
  big-endian, hashed with SHA-256; the digest becomes four xoshiro256\*\*
  state words, read big-endian.
- **`next_int(low, high)` is `(next_double() * (high - low + 1)) as u64 + low`** —
  a float multiply, not a modulus — and `next_double` takes the top 53
  bits. `shuffled` removes by index from a shrinking vector. Either one
  "corrected" changes which fragments a part mixes.
- **The degree chooser** is Vose's alias sampler over the weights `1/i`
  for `i` in `1..=n`, drawing two doubles per sample in that order.

Two deliberate differences, neither of them interoperable behaviour:

- A part whose CBOR is followed by trailing bytes is refused. The crate
  ignored the trailing bytes. Nothing emits them.
- Only the minimal bytewords style is written. The standard and URI
  styles are in BCR-2020-012 but no UR uses them.

## The vectors

Every one of these is a test in `core/osk-codec/src/ur/`:

| Source | What it covers |
|---|---|
| BCR-2020-012 examples | the 100-byte payload in minimal style, `[0,1,2,128,255]`, the empty payload, and the CRC-32 check values for `"Wolf"`, `"Hello, world!"` and `"123456789"` |
| BCR-2020-005 examples | the `ur:seed` single part, the 50-byte `ur:bytes` single part, the `ur:crypto-request` example, and the twenty-part `ur:bytes/1-9/…` stream over a 256-byte message |
| `ur` 0.5.2 `xoshiro.rs`, `sampler.rs` | the three seeded `next_u64` / `next_int` runs, the ten shuffles, the alias sampler's run, and the degrees for `Wolf-1`…`Wolf-100` |
| `ur` 0.5.2 `fountain.rs` | `fragment_length` for six sizes, the eleven fragments of the 1,024-byte message, `choose_fragments` for sequences 1 to 30, the twenty emitted parts and their CBOR, the loss-tolerant round trip over 32,767 bytes, and every rejection case |
| `ur` 0.5.2 `ur.rs` | the single-part and multi-part vectors above, upper-case decoding, and the malformed strings |
| Recorded from `ur` 0.5.2 here | three streams over a 120-byte payload at fragment lengths 20, 60 and 400, and its single-part form — a payload that is in no specification, so the match is asserted against the crate's actual output and not against a document |
| `opensigner-core/tests/scan.rs` | the Sparrow-style stream through the scanner |

**The cross-check.** Before the crate was removed it was kept as a
dev-dependency and run beside the new code over 200 random payloads of 1
to 2,000 bytes at random fragment lengths of 1 to 400. For each: the
single-part encoding string, each of up to 400 emitted part strings, and
that each decoder finishes the other's stream to the same message when
half the parts are dropped. All 200 matched, string for string. The
dev-dependency and that test are gone; the recorded streams in the table
above are what is left of it, and they are enough to catch a change in
any of the four details named earlier.

**Reopen when** a later BCR adds a part encoding this does not implement
(`crypto-account`, `crypto-hdkey` and BIP-388 policies are §16.20's
later work, and each is a payload inside the same transport, not a change
to it).
