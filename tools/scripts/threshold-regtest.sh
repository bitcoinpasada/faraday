#!/usr/bin/env bash
# The Bitcoin Core round trip behind the threshold-wallet fixtures
# (docs/PLANNING.md §16.103, tools/vectors/psbt/README.md).
#
# A threshold wallet is a single-signature taproot wallet to everything
# outside the device: `tr(XPUB/<0;1>/*)` over the group key's synthetic
# extended public key. Core therefore needs no FROST at all. It imports
# the record's own descriptor watch-only, funds it, builds the spend, and
# later judges the transaction the device finished.
#
#   tools/scripts/threshold-regtest.sh setup <datadir>
#   cp <datadir>/wallet-threshold-first.psbt tools/vectors/psbt/
#   cargo run -p osk-psbt --example threshold -- tools/vectors/psbt
#   tools/scripts/threshold-regtest.sh accept <datadir> \
#       tools/vectors/psbt/wallet-threshold-signed.psbt
#   tools/scripts/threshold-regtest.sh stop <datadir>
#
# `setup` leaves the daemon running and writes the funded PSBT into the
# datadir. `accept` finalizes the device's PSBT and judges it with
# `testmempoolaccept`. Core signs nothing here: it holds no key of this
# wallet, and both 31.1 and 29.4 build and judge the same way.
set -euo pipefail

B=${BITCOIN_BIN:-$HOME/.local/opt/bitcoin-31.1/bin}
MODE=${1:?setup|accept|stop}
D=${2:?datadir}
C() { "$B/bitcoin-cli" -regtest -datadir="$D" "$@"; }

# The committed regtest record's descriptor line, checksum and all.
DESC=${THRESHOLD_DESC:-$(tail -1 "$(dirname "$0")/../vectors/psbt/wallet-threshold-regtest.record")}

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
    # The miner, which holds coins and nothing of the threshold wallet.
    C createwallet miner >/dev/null 2>&1 || true
    C -rpcwallet=miner generatetoaddress 101 "$(C -rpcwallet=miner getnewaddress)" >/dev/null

    # The threshold wallet as Core sees it: one taproot key, watch-only.
    C createwallet threshold true false "" false true >/dev/null 2>&1 || true
    C -rpcwallet=threshold importdescriptors \
        "[{\"desc\":\"$DESC\",\"timestamp\":\"now\",\"active\":true}]" \
        | jq -c '.[0].success'

    ADDR=$(C -rpcwallet=threshold getnewaddress "" bech32m)
    C -rpcwallet=miner sendtoaddress "$ADDR" 1.0 >/dev/null
    C -rpcwallet=miner generatetoaddress 1 "$(C -rpcwallet=miner getnewaddress)" >/dev/null

    DEST=$(C -rpcwallet=miner getnewaddress "" bech32m)
    PSBT=$(C -rpcwallet=threshold walletcreatefundedpsbt '[]' "[{\"$DEST\":0.5}]" | jq -r .psbt)
    printf '%s' "$PSBT" > "$D/wallet-threshold-first.psbt"

    echo "address funded: $ADDR"
    echo "internal key origin Core wrote:"
    C decodepsbt "$PSBT" | jq -c '.inputs[0].taproot_bip32_derivs'
    echo "wrote $D/wallet-threshold-first.psbt"
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
