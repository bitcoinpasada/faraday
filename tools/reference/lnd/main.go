// Command lnd answers the cross-check's `lightning` area with LND's own
// code: the aezeed package (read, with LND's wordlist, checksum, version,
// scrypt and AEZ) and the node key over btcd's hdkeychain at LND's path
// (keychain/derivation.go: m/1017'/coin'/6'/0/0). One process answers every
// request of a run, a JSON object a line in, a JSON object a line out.
//
// Written for Faraday's cross-check; the vendored packages are LND's and
// its dependencies', unchanged (tools/reference/lnd/README.md).
package main

import (
	"bufio"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	_ "unsafe" // for go:linkname

	"github.com/btcsuite/btcd/btcutil/v2/hdkeychain"
	"github.com/btcsuite/btcd/chaincfg/v2"
	"github.com/lightningnetwork/lnd/aezeed"
)

// LND's scrypt cost is a package variable it does not export; its own tests
// lower it the same way (aezeed/cipherseed_test.go, init). The vendored file
// is not changed: this names the variable from here.
//
//go:linkname scryptN github.com/lightningnetwork/lnd/aezeed.scryptN
var scryptN int

type request struct {
	ID         int      `json:"id"`
	Op         string   `json:"op"`
	Words      []string `json:"words"`
	Passphrase string   `json:"passphrase"`
	Cost       int      `json:"cost"`
	Entropy    string   `json:"entropy"`
	Network    string   `json:"network"`
}

func fail(id int, msg string) map[string]any {
	return map[string]any{"id": id, "err": msg}
}

func readSeed(r request) map[string]any {
	if len(r.Words) != aezeed.NumMnemonicWords {
		return fail(r.ID, "wrong word count")
	}
	pass, err := hex.DecodeString(r.Passphrase)
	if err != nil {
		return fail(r.ID, "bad passphrase hex")
	}
	var m aezeed.Mnemonic
	copy(m[:], r.Words)
	scryptN = r.Cost
	seed, err := m.ToCipherSeed(pass)
	if err != nil {
		return fail(r.ID, err.Error())
	}
	return map[string]any{"id": r.ID, "ok": map[string]any{
		"internal_version": seed.InternalVersion,
		"birthday":         seed.Birthday,
		"entropy":          hex.EncodeToString(seed.Entropy[:]),
	}}
}

func nodeKey(r request) map[string]any {
	entropy, err := hex.DecodeString(r.Entropy)
	if err != nil {
		return fail(r.ID, "bad entropy hex")
	}
	coin := uint32(1)
	params := &chaincfg.TestNet3Params
	if r.Network == "mainnet" {
		coin, params = 0, &chaincfg.MainNetParams
	}
	key, err := hdkeychain.NewMaster(entropy, params)
	if err != nil {
		return fail(r.ID, err.Error())
	}
	for _, n := range []uint32{
		hdkeychain.HardenedKeyStart + 1017, hdkeychain.HardenedKeyStart + coin,
		hdkeychain.HardenedKeyStart + 6, 0, 0,
	} {
		if key, err = key.Derive(n); err != nil {
			return fail(r.ID, err.Error())
		}
	}
	priv, err := key.ECPrivKey()
	if err != nil {
		return fail(r.ID, err.Error())
	}
	return map[string]any{"id": r.ID, "ok": map[string]any{
		"secret":  hex.EncodeToString(priv.Serialize()),
		"node_id": hex.EncodeToString(priv.PubKey().SerializeCompressed()),
	}}
}

func main() {
	in := bufio.NewScanner(os.Stdin)
	in.Buffer(make([]byte, 1<<20), 1<<24)
	out := bufio.NewWriter(os.Stdout)
	for in.Scan() {
		var r request
		var ans map[string]any
		if err := json.Unmarshal(in.Bytes(), &r); err != nil {
			ans = fail(0, "bad request: "+err.Error())
		} else {
			switch r.Op {
			case "aezeed":
				ans = readSeed(r)
			case "node":
				ans = nodeKey(r)
			default:
				ans = fail(r.ID, "unknown op "+r.Op)
			}
		}
		line, _ := json.Marshal(ans)
		fmt.Fprintf(out, "%s\n", line)
		out.Flush()
	}
}
