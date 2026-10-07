# Signed-message vectors

What `core/osk-psbt/tests/message.rs` checks `osk_psbt::message` against.
Every file here is copied verbatim from the source named below; nothing
in it was produced by this repository.

| File | Source |
|---|---|
| `bip322-basic.json` | `bip-0322/basic-test-vectors.json` in the [BIP repository](https://github.com/bitcoin/bips) |
| `bip137.txt` | Trezor's and Electrum's own test suites (see below) |

## `bip322-basic.json`

The BIP's own vectors: the message hash and the `to_spend` and `to_sign`
transaction ids for three messages, "simple" signatures for a p2wpkh, a
p2wsh 3-of-3 and a p2tr address, and signatures that must not verify.

Each `bip322_signatures` entry carries the `smp` variant prefix the BIP
added in 2025. Wallets in the field write the base64 witness alone, so
the crate reads both and writes the plain form.

The p2wsh 3-of-3 case is a script address, which the "simple" variant of
this crate does not check; the test asserts it is refused rather than
silently called invalid.

## `bip137.txt`

One record per case, `key: value` lines, records separated by a blank
line — the format `core/osk-psbt/tests/common` reads.

- `source: trezor` — from
  `tests/device_tests/bitcoin/test_signmessage.py` in
  [trezor-firmware](https://github.com/trezor/trezor-firmware). The key
  is the Trezor test mnemonic ("all" twelve times) with no passphrase,
  at the path in the record. `no-script-type: yes` marks the cases where
  Trezor writes a p2pkh-range header for a segwit address, which is what
  Electrum and Sparrow also accept.
- `source: electrum` — from `tests/test_bitcoin.py` in
  [Electrum](https://github.com/spesmilo/electrum). The key is a WIF, and
  Electrum always writes a header in the p2pkh range, whatever the
  address form.

Signatures are recorded in the encoding their source publishes them in:
`signature-hex` for Trezor, `signature-base64` for Electrum.

Trezor's signatures are reproduced byte for byte: both it and this crate
sign with plain RFC 6979. Electrum's are checked, not reproduced — it
grinds its nonce for a low R, so its signature over the same message by
the same key is a different valid one. The same holds for the BIP-322
taproot vector: BIP-340 lets the signer choose the auxiliary randomness,
and this crate uses none.
