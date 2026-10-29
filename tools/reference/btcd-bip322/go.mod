module btcd-bip322-ref

go 1.26.10

require (
	github.com/btcsuite/btcd/address/v2 v2.0.0
	github.com/btcsuite/btcd/bip322 v0.0.0-00010101000000-000000000000
	github.com/btcsuite/btcd/btcec/v2 v2.5.0
	github.com/btcsuite/btcd/chaincfg/v2 v2.0.0
	github.com/btcsuite/btcd/txscript/v2 v2.0.0
)

require (
	github.com/btcsuite/btcd/btcutil/v2 v2.0.0 // indirect
	github.com/btcsuite/btcd/chainhash/v2 v2.0.0 // indirect
	github.com/btcsuite/btcd/psbt/v2 v2.0.0 // indirect
	github.com/btcsuite/btcd/wire/v2 v2.0.0 // indirect
	github.com/btcsuite/btclog v1.0.0 // indirect
	github.com/decred/dcrd/crypto/blake256 v1.1.0 // indirect
	github.com/decred/dcrd/dcrec/secp256k1/v4 v4.4.1 // indirect
	github.com/kcalvinalvin/anet v0.0.0-20251112173137-d8ddc1f6dbee // indirect
	golang.org/x/crypto v0.57.0 // indirect
	golang.org/x/sys v0.48.0 // indirect
)

// btcd's BIP-322 is pull request 2521, not merged: its bip322 module and
// the psbt, txscript and wire modules it patches are the pull request's
// own folders at its head commit 503719c950953906a8fad9286b3b51aef5ff152b,
// as the pull request's bip322/go.mod replaces them. `go mod vendor` copied
// them into vendor/; the folders named here are not kept (README.md).
replace (
	github.com/btcsuite/btcd/bip322 => ./btcd/bip322
	github.com/btcsuite/btcd/psbt/v2 => ./btcd/psbt
	github.com/btcsuite/btcd/txscript/v2 => ./btcd/txscript
	github.com/btcsuite/btcd/wire/v2 => ./btcd/wire
)
