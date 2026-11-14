# LND's own code, in Go

The cross-check's `lightning` area asks LND's code what it makes of an
aezeed and of an entropy, and compares the answers with Faraday's
(`tools/crosscheck/area_lightning.py`). This folder is a Go module
holding that code, fetched unchanged by the Go toolchain, and the one small program that answers
for it.

| Part | What | Source | Version | Licence |
|---|---|---|---|---|
| LND | `aezeed` package: wordlist, version byte, CRC-32C, scrypt, AEZ call | <https://github.com/lightningnetwork/lnd> | commit `88959aec3d7a9d11a509714d9bee97c29ee02269` (Go `v0.0.0-20260919003839-88959aec3d7a`) | MIT |
| Yawning Angel's AEZ | the AEZ v5 cipher LND's aezeed calls | <https://github.com/Yawning/aez> | `v0.0.0-20211027044916-e49e68abd344` | CC0 1.0 |
| btcd | `btcutil/v2/hdkeychain`, `chaincfg/v2`, `btcec/v2` and what they import: LND's BIP-32 | <https://github.com/btcsuite/btcd> | `btcutil/v2 v2.0.0`, `chaincfg/v2 v2.0.0`, `btcec/v2 v2.5.0` | ISC |
| the rest | `kkdai/bstream`, `golang.org/x/crypto` (scrypt, BLAKE2b, PBKDF2), btcwallet's `walletdb`, decred's secp256k1 and the other packages the build imports | `go.mod` and `go.sum` name each with its version and hash | | each module's `LICENSE` beside its files |

Every licence allows copying with its notice, which the files keep.
The versions are the ones LND's own `go.mod` at that commit asks for, as
Go's module resolution picks them; `go.sum` holds their hashes.

## What is here, and what is ours

- There is no `vendor/`: the Go toolchain fetches the modules into its module
  cache, and `go.sum` holds the hash of each, which the toolchain checks. They
  are upstream's, unchanged. LND's `aezeed/cipherseed.go`,
  `cipherseed_integration.go`, `errors.go` and `wordlist.go` were compared with
  a checkout of the commit above and are the same files.
- `go.mod`, `go.sum`: written for this folder (`go mod tidy`), requiring LND at the commit and nothing else.
- `main.go`: Faraday's, the only source file here that is not
  upstream's. It reads one JSON request a line and answers one a line.
  `aezeed` calls LND's `Mnemonic.ToCipherSeed`; `node` runs
  `hdkeychain.NewMaster` and `Derive` over `m/1017'/coin'/6'/0/0`.
  LND derives the node key through btcwallet, which is not used, so
  the path's numbers (1017, the coin type, key family 6, 0, 0) are
  written in `main.go` from `keychain/derivation.go`; the BIP-32 steps
  are btcd's, which btcwallet uses. The scrypt cost is a package
  variable LND does not export, as its own tests lower it (`init` in
  `aezeed/cipherseed_test.go`): `main.go` names it with `go:linkname`,
  so LND's file stays as it is.

## Building

`tools/crosscheck/run.sh` builds it with the installed toolchain only:

```
GOFLAGS=-mod=readonly GOTOOLCHAIN=local \
  go build -C tools/reference/lnd -trimpath -o out/crosscheck/lnd-ref .
```

`go.mod`'s `go` line is the least Go that builds it (1.27); the run
stops when `go` is missing or older, and the stamp records the version
that ran. The first build on a machine fetches the modules from the network
into the module cache, each checked against `go.sum`; a later build reads the
cache.

Every file here but this README is in
`tools/crosscheck/references.sha256`; the run stops when one differs.

## Updating

In a scratch copy of this folder with network: change the version in
`go.mod`, `GOFLAGS=-mod=mod go mod tidy`, then compare
LND's `aezeed` files with a checkout of the new commit and copy the
folder back, with a new list in `references.sha256` and the table above
changed, in a reviewed commit.

## What was run to check it

At the pinned commit, LND's code answered the cross-check's published
vectors of `tools/vectors/aezeed/` (five cipher seeds, two node keys):
the same words give the same version, birthday and entropy, and the same
node secrets and ids, as the vectors say.
