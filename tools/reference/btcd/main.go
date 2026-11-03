// Faraday's answer program for a released btcd (tools/reference/btcd/,
// README.md): the cross-check's `bip32` and `taproot` areas ask it, one JSON
// array of requests a line, one JSON array of answers a line, in order. It
// calls btcd's `hdkeychain`, `txscript` (control blocks) and `btcec` and does
// no BIP-32 or BIP-341 work of its own beyond reading the answers out.
//
//	{"op": "master", "seed": hex, "test": bool}
//	    -> {"xprv", "xpub", "fingerprint"} or {"error"}
//	{"op": "derive", "seed": hex, "test": bool, "path": [index, ...]}
//	    -> {"xprv", "xpub", "fingerprint", "master_fingerprint"} or {"error"}
//	{"op": "derive_pub", "xpub": text, "path": [index, ...]}
//	    -> {"xpub"} or {"error"}
//	{"op": "decode", "key": text}
//	    -> {"key": text read back, "private": bool, "depth": n,
//	    "fingerprint"} or {"error"}
//	{"op": "control", "output_key": hex, "control": hex, "script": hex}
//	    -> {"valid": bool} and, when not, {"error"}
package main

import (
	"bufio"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"

	"github.com/btcsuite/btcd/address/v2"
	"github.com/btcsuite/btcd/btcutil/v2/hdkeychain"
	"github.com/btcsuite/btcd/chaincfg/v2"
	"github.com/btcsuite/btcd/txscript/v2"
)

type request struct {
	Op        string   `json:"op"`
	Seed      string   `json:"seed"`
	Test      bool     `json:"test"`
	Path      []uint32 `json:"path"`
	Xpub      string   `json:"xpub"`
	Key       string   `json:"key"`
	OutputKey string   `json:"output_key"`
	Control   string   `json:"control"`
	Script    string   `json:"script"`
}

type answer = map[string]any

func fail(err error) answer { return answer{"error": err.Error()} }

func params(test bool) *chaincfg.Params {
	if test {
		return &chaincfg.TestNet3Params
	}
	return &chaincfg.MainNetParams
}

func fingerprint(k *hdkeychain.ExtendedKey) (string, error) {
	pub, err := k.ECPubKey()
	if err != nil {
		return "", err
	}
	return hex.EncodeToString(address.Hash160(pub.SerializeCompressed())[:4]), nil
}

func master(r request) (*hdkeychain.ExtendedKey, error) {
	seed, err := hex.DecodeString(r.Seed)
	if err != nil {
		return nil, err
	}
	return hdkeychain.NewMaster(seed, params(r.Test))
}

func keys(k *hdkeychain.ExtendedKey) (answer, error) {
	pub, err := k.Neuter()
	if err != nil {
		return nil, err
	}
	fp, err := fingerprint(k)
	if err != nil {
		return nil, err
	}
	return answer{"xprv": k.String(), "xpub": pub.String(), "fingerprint": fp}, nil
}

func derive(k *hdkeychain.ExtendedKey, path []uint32) (*hdkeychain.ExtendedKey, error) {
	for _, i := range path {
		var err error
		if k, err = k.Derive(i); err != nil {
			return nil, err
		}
	}
	return k, nil
}

func handle(r request) answer {
	switch r.Op {
	case "master":
		m, err := master(r)
		if err != nil {
			return fail(err)
		}
		out, err := keys(m)
		if err != nil {
			return fail(err)
		}
		return out
	case "derive":
		m, err := master(r)
		if err != nil {
			return fail(err)
		}
		k, err := derive(m, r.Path)
		if err != nil {
			return fail(err)
		}
		out, err := keys(k)
		if err != nil {
			return fail(err)
		}
		if out["master_fingerprint"], err = fingerprint(m); err != nil {
			return fail(err)
		}
		return out
	case "derive_pub":
		k, err := hdkeychain.NewKeyFromString(r.Xpub)
		if err != nil {
			return fail(err)
		}
		if k.IsPrivate() {
			return fail(fmt.Errorf("a private key"))
		}
		if k, err = derive(k, r.Path); err != nil {
			return fail(err)
		}
		return answer{"xpub": k.String()}
	case "decode":
		k, err := hdkeychain.NewKeyFromString(r.Key)
		if err != nil {
			return fail(err)
		}
		fp, err := fingerprint(k)
		if err != nil {
			return fail(err)
		}
		return answer{"key": k.String(), "private": k.IsPrivate(), "depth": k.Depth(),
			"fingerprint": fp}
	case "control":
		output, err1 := hex.DecodeString(r.OutputKey)
		control, err2 := hex.DecodeString(r.Control)
		script, err3 := hex.DecodeString(r.Script)
		if err1 != nil || err2 != nil || err3 != nil {
			return fail(fmt.Errorf("bad hex"))
		}
		block, err := txscript.ParseControlBlock(control)
		if err != nil {
			return answer{"valid": false, "error": err.Error()}
		}
		if err := txscript.VerifyTaprootLeafCommitment(block, output, script); err != nil {
			return answer{"valid": false, "error": err.Error()}
		}
		return answer{"valid": true}
	}
	return fail(fmt.Errorf("unknown op %q", r.Op))
}

func main() {
	in := bufio.NewReaderSize(os.Stdin, 1<<20)
	out := bufio.NewWriter(os.Stdout)
	for {
		line, err := in.ReadBytes('\n')
		if len(line) > 0 {
			var reqs []request
			var answers []answer
			if jerr := json.Unmarshal(line, &reqs); jerr != nil {
				answers = []answer{fail(jerr)}
			} else {
				for _, r := range reqs {
					answers = append(answers, handle(r))
				}
			}
			b, _ := json.Marshal(answers)
			out.Write(b)
			out.WriteByte('\n')
			out.Flush()
		}
		if err != nil {
			return
		}
	}
}
