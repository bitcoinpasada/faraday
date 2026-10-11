#!/usr/bin/env bash
# The Bitcoin Core round trip behind the taproot-multisig fixtures
# (docs/PLANNING.md §16.106, tools/vectors/psbt/README.md).
#
# A taproot multisig is BIP 387's `tr(H,sortedmulti_a(k,…))`: every key
# is in one leaf and the internal key is the NUMS point, so nothing but
# the script path can spend. Core holds no key of it. It imports the
# wallet watch-only, funds it, builds the spend, and later judges the
# transaction this tree signed.
#
#   tools/scripts/tapmulti-regtest.sh setup <datadir>
#   cp <datadir>/wallet-tapmulti-first.psbt tools/vectors/psbt/
#   cargo run -p osk-psbt --example tapmulti -- tools/vectors/psbt
#   tools/scripts/tapmulti-regtest.sh accept <datadir> \
#       tools/vectors/psbt/wallet-tapmulti-signed.psbt
#   tools/scripts/tapmulti-regtest.sh stop <datadir>
#
# `setup` leaves the daemon running and writes the funded PSBT into the
# datadir. `accept` finalizes the PSBT this tree wrote and judges it with
# `testmempoolaccept`. Core signs nothing here.
set -euo pipefail

B=${BITCOIN_BIN:-$HOME/.local/opt/bitcoin-31.1/bin}
MODE=${1:?setup|accept|stop}
D=${2:?datadir}
C() { "$B/bitcoin-cli" -regtest -datadir="$D" "$@"; }

POLICY=${TAPMULTI_POLICY:-$(dirname "$0")/../vectors/psbt/wallet-tapmulti.policy}

# The committed policy as one descriptor: the template with every `@i/**`
# replaced by that key and the `<0;1>/*` a two-chain descriptor ends in.
descriptor() {
    local desc i=0 key
    desc=$(head -1 "$POLICY")
    while read -r key; do
        [ -n "$key" ] || continue
        desc=${desc//@$i\/\*\*/$key\/<0;1>\/*}
        i=$((i + 1))
    done < <(tail -n +2 "$POLICY")
    printf '%s' "$desc"
}

start() {
    if ! C getblockchaininfo >/dev/null 2>&1; then
        mkdir -p "$D"
        "$B/bitcoind" -regtest -datadir="$D" -daemon -listen=0 -dnsseed=0 \
            -fallbackfee=0.0001 >/dev/null
        sleep 3
    fi
}

case "$MODE" in
stop)
    C stop
    ;;

setup)
    start
    # The miner, which holds coins and nothing of the taproot multisig.
    C createwallet miner >/dev/null 2>&1 || true
    C -rpcwallet=miner generatetoaddress 101 "$(C -rpcwallet=miner getnewaddress)" >/dev/null

    DESC=$(descriptor)
    CK=$(C getdescriptorinfo "$DESC" | jq -r .checksum)
    echo "descriptor: $DESC#$CK"

    C createwallet tapmulti true false "" false true >/dev/null 2>&1 || true
    C -rpcwallet=tapmulti importdescriptors \
        "[{\"desc\":\"$DESC#$CK\",\"timestamp\":\"now\",\"active\":true}]" \
        | jq -c '.[0]'

    ADDR=$(C -rpcwallet=tapmulti getnewaddress "" bech32m)
    C -rpcwallet=miner sendtoaddress "$ADDR" 1.0 >/dev/null
    C -rpcwallet=miner generatetoaddress 1 "$(C -rpcwallet=miner getnewaddress)" >/dev/null

    DEST=$(C -rpcwallet=miner getnewaddress "" bech32m)
    PSBT=$(C -rpcwallet=tapmulti walletcreatefundedpsbt '[]' "[{\"$DEST\":0.5}]" | jq -r .psbt)
    printf '%s' "$PSBT" > "$D/wallet-tapmulti-first.psbt"

    echo "address funded: $ADDR"
    echo "what Core wrote for the input:"
    C decodepsbt "$PSBT" | jq -c '.inputs[0] | {taproot_scripts, taproot_bip32_derivs, taproot_internal_key, taproot_merkle_root}'
    echo "the fee and the change:"
    C decodepsbt "$PSBT" | jq -c '{fee, outputs: [.tx.vout[].value]}'
    echo "wrote $D/wallet-tapmulti-first.psbt"
    ;;

accept)
    SIGNED=${3:?the finished psbt}
    start
    FINAL=$(cat "$SIGNED")
    echo "finalizepsbt: $(C finalizepsbt "$FINAL" | jq -c '{complete}')"
    HEX=$(C finalizepsbt "$FINAL" | jq -r .hex)
    C decoderawtransaction "$HEX" | jq -c '{txid,vsize,weight}'
    C testmempoolaccept "[\"$HEX\"]" | jq -c '.[0]'
    ;;

*)
    echo "usage: $0 setup|accept|stop <datadir> [psbt]" >&2
    exit 2
    ;;
esac
