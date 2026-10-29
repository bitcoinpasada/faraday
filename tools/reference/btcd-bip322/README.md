# btcd's BIP-322, vendored in Go

The cross-check's `msg` area asks btcd's `bip322` package what it makes of
every BIP-322 signature Faraday makes or checks, and of the virtual
transactions' ids, and compares the answers with Faraday's
(`tools/crosscheck/area_msg.py`). This folder is a Go module holding that
code, unchanged, and the one small program that answers for it.

| Part | What | Source | Version | Licence |
|---|---|---|---|---|
| btcd's BIP-322 | the `bip322` module: simple, full and proof-of-funds signatures made and verified, the three results | pull request 2521 of <https://github.com/btcsuite/btcd>, branch `bip-322` of <https://github.com/guggero/btcd> | head commit `503719c950953906a8fad9286b3b51aef5ff152b` (2026-10-23), not merged | ISC |
| btcd's `psbt`, `txscript`, `wire` | the three modules as the pull request patches them, which its `bip322/go.mod` replaces with its own folders | the same commit | `v2.0.0`, replaced | ISC |
| btcd's `address`, `btcec`, `btcutil`, `chaincfg`, `chainhash` | released modules the pull request's `go.mod` asks for | <https://github.com/btcsuite/btcd> | `address/v2 v2.0.0`, `btcec/v2 v2.5.0`, `btcutil/v2 v2.0.0`, `chaincfg/v2 v2.0.0`, `chainhash/v2 v2.0.0` | ISC |
| the rest | decred's secp256k1 and blake256, `btclog`, `kcalvinalvin/anet`, `golang.org/x/crypto` (RIPEMD-160, SHA-3), `golang.org/x/sys` | `vendor/modules.txt` names each with its version | | each module's `LICENSE` beside its files |

The pull request is by Oliver Gugger; BIP-322 names it as its complete
reference implementation and the source of its test vectors. btcd's curve
is decred's pure-Go secp256k1, which shares no code with libsecp256k1 or
rust-bitcoin. Every licence allows copying with its notice, which the files
keep; `LICENSE-btcd` is the ISC licence at the root of the pull request's
tree, which covers its four replaced folders (they carry none of their own).

## What is vendored, and what is ours

- `vendor/`: what `go mod vendor` copied, byte for byte, with three
  differences, all made so that the repository's checks accept it, and
  no code changed. One comment line of
  `btcd/btcutil/v2/hdkeychain/extendedkey.go` (line 390) says "issue 172"
  where upstream's has a number sign before the 172 (the repository's
  commit check refuses that form of citation); upstream's file has SHA-256
  `4f0d0c4e5a52af4ec382d44eab9d2c0078238419d0c68ca176986fcc74ad7108`. And
  `kcalvinalvin/anet`'s `README.md` is left out: its sample output carries
  a private network address, which the push check refuses; upstream's has
  SHA-256 `2cabb5b64adcabdbf05f420c6de9d90808b6c7bd2481fe8ae7ef8a20ccf90abe`,
  and no build reads it. `btcd/btcutil/v2/certgen.go` is left out for the
  same reason: it writes the loopback address into a TLS certificate;
  upstream's has SHA-256
  `cceb10a31078ab03518d2185529268d41a555385267831d494bec2b03d5d66f8`, and
  nothing the program builds calls it. The `bip322`, `psbt`, `txscript` and `wire` folders were
  compared with the pull request's tree at the commit above and are its
  files, less their tests, test data and `go.mod`.
- `go.mod`, `go.sum`: written for this folder (`go mod tidy`, then
  `go mod vendor`). They replace the four modules with folders named
  `btcd/…`, as the pull request's own `bip322/go.mod` replaces three of
  them; those folders were the pull request's, copied in for `go mod
  vendor` and not kept: a build reads `vendor/` only.
- `main.go`: Faraday's, the only source file here that is not upstream's.
  It reads one JSON array of requests a line and answers with one array a
  line. `verify` calls `bip322.VerifyMessage` and reports valid, invalid or
  inconclusive (`bip322.ErrInconclusive`); `sign` calls `SignP2PKH`,
  `SignNestedP2WPKH`, `SignP2WPKH` or `SignP2TR`; `ids` reads the two
  transaction ids off `BuildToSignPacketFull` with version, lock time and
  sequence 0, which is what the simple variant builds. The network an
  address is decoded for is read from its prefix.

## Building

`tools/crosscheck/run.sh` builds it, offline and with the installed
toolchain only:

```
GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off \
  go build -C tools/reference/btcd-bip322 -trimpath -o out/crosscheck/btcd-bip322-ref .
```

`go.mod`'s `go` line is the pull request's (1.26.10); the run stops when
`go` is missing or older. Nothing is downloaded: a build needs only
`vendor/`.

Every file here but this README, `main.go` and `vendor/` included, is in
`tools/crosscheck/references.sha256`; the run stops when one differs.

## Updating

In a scratch copy of this folder with network: put the pull request's
`bip322`, `psbt`, `txscript` and `wire` folders at the new commit under
`btcd/`, `GOFLAGS=-mod=mod go mod tidy`, `go mod vendor`, take `btcd/`
away, check that `go build` still runs with `GOPROXY=off`, and copy the
folder back, with a new list in `references.sha256` and the table above
changed, in a reviewed commit. Once the pull request is merged, require
the released module instead and drop the replacements.

## What was run to check it

At the pinned commit, btcd's code answered BIP-322's basic and generated
vectors and its own negative vectors (`tools/vectors/message/`) with the
results they name, and the BIP's empty-message and "Hello World" txids.
