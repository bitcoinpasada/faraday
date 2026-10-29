# LND's own code, vendored in Go

The cross-check's `lightning` area asks LND's code what it makes of an
aezeed and of an entropy, and compares the answers with Faraday's
(`tools/crosscheck/area_lightning.py`). This folder is a Go module
holding that code, unchanged, and the one small program that answers
for it.

| Part | What | Source | Version | Licence |
|---|---|---|---|---|
| LND | `aezeed` package: wordlist, version byte, CRC-32C, scrypt, AEZ call | <https://github.com/lightningnetwork/lnd> | commit `88959aec3d7a9d11a509714d9bee97c29ee02269` (Go `v0.0.0-20260919003839-88959aec3d7a`) | MIT |
| Yawning Angel's AEZ | the AEZ v5 cipher LND's aezeed calls | <https://github.com/Yawning/aez> | `v0.0.0-20211027044916-e49e68abd344` | CC0 1.0 |
| btcd | `btcutil/v2/hdkeychain`, `chaincfg/v2`, `btcec/v2` and what they import: LND's BIP-32 | <https://github.com/btcsuite/btcd> | `btcutil/v2 v2.0.0`, `chaincfg/v2 v2.0.0`, `btcec/v2 v2.5.0` | ISC |
| the rest | `kkdai/bstream`, `golang.org/x/crypto` (scrypt, BLAKE2b, PBKDF2), btcwallet's `walletdb`, decred's secp256k1 and the other packages `go mod vendor` found imported | `vendor/modules.txt` names each with its version | | each module's `LICENSE` beside its files |

Every licence allows copying with its notice, which the files keep.
The versions are the ones LND's own `go.mod` at that commit asks for, as
Go's module resolution picks them; `go.sum` holds their hashes.

## What is vendored, and what is ours

- `vendor/`: what `go mod vendor` copied, byte for byte, with two
  differences, both made so that the repository's checks accept it.
  First, the packages `go mod vendor` adds only for the dependencies'
  own tests (testify, yaml.v3, go-spew, difflib, btcwallet's walletdb
  and waddrmgr, btcd's txscript and others: 28 folders) are emptied,
  since `go list -deps` shows the program builds without them; their
  modules stay in `modules.txt` and keep their licences. Second, one
  comment line of `btcd/btcutil/v2/hdkeychain/extendedkey.go` (line 390)
  says "issue 172" where upstream's has a number sign before the 172 (the repository's
  commit check refuses that form of citation); upstream's file has
  SHA-256 `4f0d0c4e5a52af4ec382d44eab9d2c0078238419d0c68ca176986fcc74ad7108`,
  and no code differs. LND's `aezeed/cipherseed.go`,
  `cipherseed_integration.go`, `errors.go` and `wordlist.go` were
  compared with a checkout of the commit above and are the same files.
- `go.mod`, `go.sum`: written for this folder (`go mod tidy`, then
  `go mod vendor`), requiring LND at the commit and nothing else.
- `main.go`: Faraday's, the only source file here that is not
  upstream's. It reads one JSON request a line and answers one a line.
  `aezeed` calls LND's `Mnemonic.ToCipherSeed`; `node` runs
  `hdkeychain.NewMaster` and `Derive` over `m/1017'/coin'/6'/0/0`.
  LND derives the node key through btcwallet, which is not vendored, so
  the path's numbers (1017, the coin type, key family 6, 0, 0) are
  written in `main.go` from `keychain/derivation.go`; the BIP-32 steps
  are btcd's, which btcwallet uses. The scrypt cost is a package
  variable LND does not export, as its own tests lower it (`init` in
  `aezeed/cipherseed_test.go`): `main.go` names it with `go:linkname`,
  so the vendored file stays as it is.

## Building

`tools/crosscheck/run.sh` builds it, offline and with the installed
toolchain only:

```
GOFLAGS=-mod=vendor GOTOOLCHAIN=local GOPROXY=off \
  go build -C tools/reference/lnd -trimpath -o out/crosscheck/lnd-ref .
```

`go.mod`'s `go` line is the least Go that builds it (1.27); the run
stops when `go` is missing or older, and the stamp records the version
that ran. Nothing is downloaded: a build needs only `vendor/`.

Every file here, `main.go` and `vendor/` included, is in
`tools/crosscheck/references.sha256`; the run stops when one differs.

## Updating

In a scratch copy of this folder with network: change the version in
`go.mod`, `GOFLAGS=-mod=mod go mod tidy`, `go mod vendor`, then compare
LND's `aezeed` files with a checkout of the new commit and copy the
folder back, with a new list in `references.sha256` and the table above
changed, in a reviewed commit.

## What was run to check it

At the pinned commit, LND's code answered the cross-check's published
vectors of `tools/vectors/aezeed/` (five cipher seeds, two node keys):
the same words give the same version, birthday and entropy, and the same
node secrets and ids, as the vectors say.
