# A released btcd, vendored in Go

The cross-check's `bip32` area asks btcd's `hdkeychain` what it makes of every
master key, derivation and extended key Faraday works out or reads, and its
`taproot` area asks btcd's `txscript` what it makes of every control block
and leaf commitment (`tools/crosscheck/area_bip32.py`,
`tools/crosscheck/area_taproot.py`, through `tools/crosscheck/btcd_ref.py`).
This folder is a Go module holding that code, unchanged, and the one small
program that answers for it. BIP-322's code, which is not in a release, is
in `../btcd-bip322/` and is not touched.

| Part | What | Source | Version | Licence |
|---|---|---|---|---|
| btcd's release | the tag whose `go.mod` names the module versions below | <https://github.com/btcsuite/btcd> | tag `v0.27.0`, commit `dd169fe0c32a56087d28b92fff3b8cf4fb43477a`, the latest release tag when this was vendored | ISC |
| `btcutil/v2` | `hdkeychain`: BIP-32 master keys, derivation, extended keys as text | the same repository, module `btcutil/v2` | `v2.0.2` | ISC |
| `txscript/v2` | `ParseControlBlock` and `VerifyTaprootLeafCommitment` (BIP-341 control blocks) | module `txscript/v2` | `v2.0.1` | ISC |
| `btcec/v2`, `address/v2`, `chaincfg/v2`, `chainhash/v2`, `wire/v2` | the curve and the encodings those two read | the same repository | `btcec/v2 v2.5.1`, `address/v2 v2.0.1`, `chaincfg/v2 v2.0.0`, `chainhash/v2 v2.0.0`, `wire/v2 v2.0.2` | ISC |
| the rest | decred's secp256k1 and blake256, `btclog`, `golang.org/x/crypto`, `golang.org/x/sys` | `vendor/modules.txt` names each with its version | | each module's `LICENSE` beside its files |

The versions are those the `go.mod` of btcd's `v0.27.0` names (its root
module, the daemon, is not vendored: no code of it is used). btcd's curve is
decred's pure-Go secp256k1, which shares no code with libsecp256k1 or
rust-bitcoin. Every licence allows copying with its notice, which the files
keep.

## What is vendored, and what is ours

- `vendor/`: what `go mod vendor` copied, byte for byte, with one
  difference, made so that the repository's checks accept it, and no code
  changed: one comment line of
  `btcd/btcutil/v2/hdkeychain/extendedkey.go` (line 415) says "issue 172"
  where upstream's has a number sign before the 172 (the repository's commit
  check refuses that form of citation); upstream's file has SHA-256
  `c8bbf8c3dfe1aa7b448935bdc2cbedbed0d6d6a5c934d3458481d3cfec3164f3`.
- `go.mod`, `go.sum`: written for this folder (`go mod tidy`, then
  `go mod vendor`), with the versions above required.
- `main.go`: Faraday's, the only source file here that is not upstream's.
  It reads one JSON array of requests a line and answers with one array a
  line (its header lists the requests). `master` and `derive` call
  `hdkeychain.NewMaster` and `Derive`; `derive_pub` and `decode` call
  `NewKeyFromString`; `control` calls `txscript.ParseControlBlock` and
  `VerifyTaprootLeafCommitment`. It does no BIP-32 or BIP-341 work of its
  own beyond a key's fingerprint (the first four bytes of `Hash160` of its
  public key).

## Building

`tools/crosscheck/run.sh` builds it, offline and with the installed
toolchain only:

```
GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off \
  go build -C tools/reference/btcd -trimpath -o out/crosscheck/btcd-ref .
```

`go.mod`'s `go` line is the release's (1.26.10); the run stops when `go` is
missing or older. Nothing is downloaded: a build needs only `vendor/`.

Every file here but this README, `main.go` and `vendor/` included, is in
`tools/crosscheck/references.sha256`; the run stops when one differs.

## Updating

In a scratch copy of this folder with network: set the `require` lines to the
versions the new release's `go.mod` names, `GOFLAGS=-mod=mod go mod tidy`,
`go mod vendor`, make the one comment change above again, check that
`go build` still runs with `GOPROXY=off`, and copy the folder back, with a
new list in `references.sha256` and the table above changed, in a reviewed
commit.
