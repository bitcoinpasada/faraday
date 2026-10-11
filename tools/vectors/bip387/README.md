# BIP 387 tapscript multisig vectors

BIP 387 ("Tapscript Multisig Output Script Descriptors"), verbatim, from
`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0387.mediawiki`.
The BIP carries its vectors in its own text: six valid descriptors with
the output scripts they pay to at index 0, 1 and 2, and seven invalid
ones with the reason each is invalid.

| File | Downloaded | SHA-256 | Read by |
|---|---|---|---|
| `bip-0387.mediawiki` | 2026-09-18 | `c0761b2b078dbfc1a973ae3f5187ce58c47688fe3ec58ca75f1fbb80f6b03207` | `core/osk-bip/tests/bip387.rs` |

`core/osk-bip/tests/bip387.rs` reads the two lists out of the text and
checks every script. The keys are raw public keys, WIF private keys and
extended private keys, which no wallet of this tree loads, so what the
vectors check is the reader underneath a wallet: `miniscript` for
`multi_a`, and `osk_bip::tapmulti` for `sortedmulti_a`, which
`miniscript` has no parser for.

One valid vector is read but not derived: its third key ends in `/*'`, a
hardened wildcard, whose addresses only the private key can produce. A
device holding its cosigners' extended public keys could not derive that
wallet's addresses at all, and `miniscript` panics rather than refusing,
so the test stops at the parse for that one.
