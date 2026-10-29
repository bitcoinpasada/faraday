// Faraday's answer program for btcd's BIP-322 (tools/reference/btcd-bip322/,
// README.md): the cross-check's `msg` area asks it, one JSON array of
// requests a line, one JSON array of answers a line, in order. It calls
// btcd's own `bip322` package and does no BIP-322 work of its own.
//
//	{"op": "verify", "address": a, "message": m, "signature": s}
//	    -> {"state": "valid" | "invalid" | "inconclusive", "error": e}
//	{"op": "sign", "kind": "p2pkh" | "p2sh-p2wpkh" | "p2wpkh" | "p2tr",
//	 "secret": hex, "message": m}
//	    -> {"signature": s} or {"error": e}
//	{"op": "ids", "address": a, "message": m}
//	    -> {"to_spend": txid, "to_sign": txid} or {"error": e}
package main

import (
	"bufio"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"

	"github.com/btcsuite/btcd/address/v2"
	"github.com/btcsuite/btcd/bip322"
	"github.com/btcsuite/btcd/btcec/v2"
	"github.com/btcsuite/btcd/chaincfg/v2"
	"github.com/btcsuite/btcd/txscript/v2"
)

type request struct {
	Op        string `json:"op"`
	Address   string `json:"address"`
	Message   string `json:"message"`
	Signature string `json:"signature"`
	Kind      string `json:"kind"`
	Secret    string `json:"secret"`
}

// params names the network an address is written for, by its prefix:
// btcd's decoder checks the address against the network it is given.
func params(addr string) *chaincfg.Params {
	lower := strings.ToLower(addr)
	switch {
	case strings.HasPrefix(lower, "bcrt1"):
		return &chaincfg.RegressionNetParams
	case strings.HasPrefix(lower, "bc1"), strings.HasPrefix(addr, "1"),
		strings.HasPrefix(addr, "3"):
		return &chaincfg.MainNetParams
	default:
		return &chaincfg.TestNet3Params
	}
}

func decode(addr string) (address.Address, error) {
	return address.DecodeAddress(addr, params(addr))
}

func verify(r request) map[string]any {
	addr, err := decode(r.Address)
	if err != nil {
		return map[string]any{"state": "invalid", "error": "address: " + err.Error()}
	}
	valid, _, err := bip322.VerifyMessage(r.Message, addr, r.Signature)
	switch {
	case err != nil && errors.Is(err, bip322.ErrInconclusive):
		return map[string]any{"state": "inconclusive", "error": err.Error()}
	case err != nil:
		return map[string]any{"state": "invalid", "error": err.Error()}
	case !valid:
		return map[string]any{"state": "invalid", "error": "not valid"}
	}
	return map[string]any{"state": "valid"}
}

func sign(r request) map[string]any {
	raw, err := hex.DecodeString(r.Secret)
	if err != nil || len(raw) != 32 {
		return map[string]any{"error": "secret"}
	}
	key, _ := btcec.PrivKeyFromBytes(raw)
	var sig string
	switch r.Kind {
	case "p2pkh":
		sig, err = bip322.SignP2PKH(r.Message, key)
	case "p2sh-p2wpkh":
		sig, err = bip322.SignNestedP2WPKH(r.Message, key)
	case "p2wpkh":
		sig, err = bip322.SignP2WPKH(r.Message, key)
	case "p2tr":
		sig, err = bip322.SignP2TR(r.Message, key)
	default:
		err = fmt.Errorf("no such kind %q", r.Kind)
	}
	if err != nil {
		return map[string]any{"error": err.Error()}
	}
	return map[string]any{"signature": sig}
}

// ids are the txids of the BIP's two virtual transactions, read off the
// to_sign packet btcd builds with version, lock time and sequence 0 (what
// its simple variant builds, which it builds only for native SegWit): its
// transaction, and the outpoint it spends, which is to_spend's.
func ids(r request) map[string]any {
	addr, err := decode(r.Address)
	if err != nil {
		return map[string]any{"error": "address: " + err.Error()}
	}
	script, err := txscript.PayToAddrScript(addr)
	if err != nil {
		return map[string]any{"error": err.Error()}
	}
	packet, err := bip322.BuildToSignPacketFull([]byte(r.Message), script, 0, 0, 0)
	if err != nil {
		return map[string]any{"error": err.Error()}
	}
	tx := packet.UnsignedTx
	return map[string]any{
		"to_spend": tx.TxIn[0].PreviousOutPoint.Hash.String(),
		"to_sign":  tx.TxHash().String(),
	}
}

func answer(r request) map[string]any {
	switch r.Op {
	case "verify":
		return verify(r)
	case "sign":
		return sign(r)
	case "ids":
		return ids(r)
	}
	return map[string]any{"error": "no such op " + r.Op}
}

func main() {
	in := bufio.NewReaderSize(os.Stdin, 1<<20)
	out := bufio.NewWriter(os.Stdout)
	enc := json.NewEncoder(out)
	enc.SetEscapeHTML(false)
	for {
		line, err := in.ReadBytes('\n')
		if len(line) > 0 {
			var reqs []request
			if jerr := json.Unmarshal(line, &reqs); jerr != nil {
				fmt.Fprintln(os.Stderr, "btcd-bip322-ref:", jerr)
				os.Exit(2)
			}
			answers := make([]map[string]any, len(reqs))
			for i, r := range reqs {
				answers[i] = answer(r)
			}
			if err := enc.Encode(answers); err != nil {
				os.Exit(2)
			}
			if err := out.Flush(); err != nil {
				os.Exit(2)
			}
		}
		if err != nil {
			return
		}
	}
}
