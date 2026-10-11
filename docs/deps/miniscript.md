# miniscript

**Purpose.** Miniscript and output descriptors in `osk-bip`: reading a
`wsh(<miniscript>)`, `sh(wsh(<miniscript>))` or `tr(<key>,<tree>)`
wallet, deriving its addresses, listing the ways it can be spent
(`osk_bip::spend`), and compiling a typed policy into a descriptor for
Tools › Miniscript.

**Why this crate.** This project writes formats in-house where the code
is short: BC-UR, the BIP-388 policy parser, the finalizer. Miniscript is
not short. Its type and correctness system (the base/verify/key/wrapped
types, the malleability and safety analysis), its script encoding, and
its satisfaction rules are each a specification in their own right, and
an in-house version would be wrong in the two places that cost money —
the address a person is shown, and the signature a spend needs. The
reference implementation is `rust-miniscript`, by the people who wrote
the specification; it pairs with the `bitcoin` crate already in the
graph, and Bitcoin Core, Liana, Sparrow and Nunchuk all read the same
descriptors it does.

It also brings the second implementation the tests need. Every address
of the two fixture wallets is checked against `miniscript` used directly
and against Bitcoin Core's `deriveaddresses`
(`tools/vectors/psbt/README.md`).

**Version and features.** `miniscript = { version = "12.0",
default-features = false, features = ["no-std", "compiler"] }`. The 12.x
line is the one that pairs with `bitcoin` 0.32; 13.x moves to
`bitcoin` 0.33 and is for whenever the rest of the tree does.

- `no-std` keeps the core `no_std` + `alloc`, which the Pi and Android
  shells build. Checked by building `osk-bip` for
  `wasm32-unknown-unknown`, which has no `std` at all.
- `compiler` is the concrete-policy compiler
  (`miniscript::policy::Concrete::compile`, `compile_tr`), which is what
  Tools › Miniscript calls. It is compiled into the device binary; it is
  not a build-time tool.
- Not enabled: `std`, `serde`, `rand`, `base64`, `trace`.

**Cost.** One crate. `miniscript` 12.3.7 depends on `bitcoin` and
`bech32`, and both were already in `osk-bip`'s graph — `bitcoin` as the
direct dependency, `bech32` through it. `cargo tree -p osk-bip` before
and after differs by exactly these three lines:

```
├── miniscript v12.3.7
│   ├── bech32 v0.11.1
│   └── bitcoin v0.32.102 (*)
```

Measured with
`cargo tree -p <root> -e normal --prefix none | sed 's/ (.*//' | sort -u | wc -l`:

| Graph | Before | After |
|---|--:|--:|
| `osk-bip` | 34 | 35 |
| `opensigner-core` | 51 | 52 |
| `opensigner-ffi` | 54 | 55 |
| `opensigner-pi` | 55 | 56 |
| `opensigner-desktop` | 128 | 129 |

**What it is not used for.** Signing. A miniscript or taproot script-path
input is read, its change is verified, and signing it returns
`Unsupported` with the reason the review shows. `miniscript`'s
satisfaction machinery is the obvious way to do that, and is a pass of
its own.

`osk_bip::policy` still parses the BIP-388 templates it always did —
`multi`, `sortedmulti`, the four single-key forms and BIP-390's
`tr(musig(…))` — in-house, and reaches `miniscript` only when none of
them matches. Those templates are short, their tests are the BIP's own
vectors, and routing them through a second parser would change what the
existing wallets read as for no gain.

**Licence.** CC0-1.0, as `bitcoin` is.

**Reopen when** the tree moves to `bitcoin` 0.33, which wants
`miniscript` 13; or if the `compiler` feature's cost in binary size
stops being worth Tools › Miniscript, in which case the compiler moves
out and the parser stays.
