# A released btcd, in Go

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
| btcd's release | the tag whose `go.mod` names the module versions below | <https://github.com/btcsuite/btcd> | tag `v0.27.0`, commit `dd169fe0c32a56087d28b92fff3b8cf4fb43477a`, the latest release tag when this was pinned | ISC |
| `btcutil/v2` | `hdkeychain`: BIP-32 master keys, derivation, extended keys as text | the same repository, module `btcutil/v2` | `v2.0.2` | ISC |
| `txscript/v2` | `ParseControlBlock` and `VerifyTaprootLeafCommitment` (BIP-341 control blocks) | module `txscript/v2` | `v2.0.1` | ISC |
| `btcec/v2`, `address/v2`, `chaincfg/v2`, `chainhash/v2`, `wire/v2` | the curve and the encodings those two read | the same repository | `btcec/v2 v2.5.1`, `address/v2 v2.0.1`, `chaincfg/v2 v2.0.0`, `chainhash/v2 v2.0.0`, `wire/v2 v2.0.2` | ISC |
| the rest | decred's secp256k1 and blake256, `btclog`, `golang.org/x/crypto`, `golang.org/x/sys` | `go.mod` and `go.sum` name each with its version and hash | | each module's `LICENSE` beside its files |

The versions are those the `go.mod` of btcd's `v0.27.0` names (its root
module, the daemon, is not fetched: no code of it is used). btcd's curve is
decred's pure-Go secp256k1, which shares no code with libsecp256k1 or
rust-bitcoin. Every licence allows copying with its notice, which the files
keep.

## What is here, and what is ours

- There is no `vendor/`: the Go toolchain fetches the modules into its module
  cache, and `go.sum` holds the hash of each, which the toolchain checks. They
  are upstream's, unchanged.
- `go.mod`, `go.sum`: written for this folder (`go mod tidy`), with the versions above required.
- `main.go`: Faraday's, the only source file here that is not upstream's.
  It reads one JSON array of requests a line and answers with one array a
  line (its header lists the requests). `master` and `derive` call
  `hdkeychain.NewMaster` and `Derive`; `derive_pub` and `decode` call
  `NewKeyFromString`; `control` calls `txscript.ParseControlBlock` and
  `VerifyTaprootLeafCommitment`. It does no BIP-32 or BIP-341 work of its
  own beyond a key's fingerprint (the first four bytes of `Hash160` of its
  public key).

## Building

`tools/crosscheck/run.sh` builds it with the installed toolchain only:

```
GOFLAGS=-mod=readonly GOTOOLCHAIN=local \
  go build -C tools/reference/btcd -trimpath -o out/crosscheck/btcd-ref .
```

`go.mod`'s `go` line is the release's (1.26.10); the run stops when `go` is
missing or older. The first build on a machine fetches the modules from the
network into the module cache, each checked against `go.sum`; a later build
reads the cache.

Every file here but this README is in
`tools/crosscheck/references.sha256`; the run stops when one differs.

## Updating

In a scratch copy of this folder with network: set the `require` lines to the
versions the new release's `go.mod` names, `GOFLAGS=-mod=mod go mod tidy`,
check that `go build` still runs, and copy the folder back, with a
new list in `references.sha256` and the table above changed, in a reviewed
commit.
