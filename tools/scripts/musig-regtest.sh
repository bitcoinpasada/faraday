#!/usr/bin/env bash
# The Bitcoin Core round trip behind the MuSig2 fixtures
# (docs/PLANNING.md §16.100, tools/vectors/psbt/README.md).
#
# Core 31.1 is the only counterpart that speaks BIP-373's fields today.
# This script builds a two-key regtest `musig()` wallet with Core holding
# the second key, funds it, and produces the two PSBTs the fixtures are:
# the funded one, before any nonce, and Core's round-1 one, with Core's
# nonce on it.
#
#   tools/scripts/musig-regtest.sh setup <datadir>
#   cargo run -p osk-psbt --example musig -- tools/vectors/psbt
#   tools/scripts/musig-regtest.sh finish <datadir> <signed.psbt>
#   tools/scripts/musig-regtest.sh answer <datadir> <nonce.psbt>
#   tools/scripts/musig-regtest.sh accept <datadir> <signed.psbt>
#
# `setup` leaves the daemon running and writes the policy and the two
# PSBTs into the datadir; copy them into tools/vectors/psbt/ to make them
# the fixtures. `finish` hands the device's PSBT back to Core, which
# writes its own partial signature, aggregates, finalizes, and judges the
# transaction with `testmempoolaccept`. `stop` stops the daemon.
#
# The other order is the device first: `answer` hands Core the PSBT the
# device's round 1 wrote and prints what Core makes of it, which is the
# fixture round 2 reads; `accept` finalizes and judges the PSBT the
# device's round 2 wrote, which is already aggregated.
#
# Core's key is generated afresh on every `setup`, so a rerun produces a
# different wallet and different fixtures. Its `tprv` stays in the
# datadir and never in this tree.
set -euo pipefail

B=$HOME/.local/opt/bitcoin-31.1/bin
MODE=${1:?setup|finish|stop}
D=${2:?datadir}
C() { "$B/bitcoin-cli" -regtest -datadir="$D" "$@"; }

# The "abandon abandon … about" seed's regtest master key, which is the
# key the device loads; @0 of the wallet.
TPRV=tprv8ZgxMBicQKsPe5YMU9gHen4Ez3ApihUfykaqUorj9t6FDqy3nP6eoXiAo2ssvpAjoLroQxHqr3R5nE3a5dU3DHTjTgJDd7zrbniJr6nrCzd

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
    # @0: the device's key, watch-only here so that Core never holds it.
    C createwallet keyA false false "" false true >/dev/null
    CK=$(C getdescriptorinfo "tr($TPRV/86h/1h/0h/<0;1>/*)" | jq -r .checksum)
    C -rpcwallet=keyA importdescriptors \
        "[{\"desc\":\"tr($TPRV/86h/1h/0h/<0;1>/*)#$CK\",\"timestamp\":\"now\",\"active\":true}]" \
        | jq -c '.[0].success'
    PUBA=$(C -rpcwallet=keyA listdescriptors \
        | jq -r '.descriptors[].desc' | grep '^tr(\[73c5da0a' | head -1 \
        | sed -E 's/^tr\((.*)\/0\/\*\)#.*/\1/')

    # @1: Core's own key, whose private half stays in its wallet.
    C createwallet keyB false false "" false true >/dev/null
    C -rpcwallet=keyB getnewaddress "" bech32m >/dev/null
    PRIVB=$(C -rpcwallet=keyB listdescriptors true \
        | jq -r '.descriptors[] | select(.desc|startswith("tr(")) | select(.desc|contains("/86h/1h/0h/0/")) | .desc' \
        | head -1 | sed -E 's/^tr\((.*)\/0\/\*\)#.*/\1/')
    PUBB=$(C -rpcwallet=keyB listdescriptors \
        | jq -r '.descriptors[] | select(.internal|not) | select(.desc|startswith("tr([")) | select(.desc|contains("/86h/1h/0h]")) | .desc' \
        | head -1 | sed -E 's/^tr\((.*)\/0\/\*\)#.*/\1/')

    MDESC="tr(musig($PUBA,$PRIVB)/<0;1>/*)"
    MCK=$(C getdescriptorinfo "$MDESC" | jq -r .checksum)
    C createwallet musig false false "" false true >/dev/null
    C -rpcwallet=musig importdescriptors \
        "[{\"desc\":\"$MDESC#$MCK\",\"timestamp\":\"now\",\"active\":true}]" \
        | jq -c '.[0].success'

    ADDR=$(C -rpcwallet=musig getnewaddress "" bech32m)
    C -rpcwallet=keyB generatetoaddress 101 "$(C -rpcwallet=keyB getnewaddress)" >/dev/null
    C -rpcwallet=musig sendtoaddress "$ADDR" 1.0 >/dev/null 2>&1 \
        || C -rpcwallet=keyB sendtoaddress "$ADDR" 1.0 >/dev/null
    C -rpcwallet=keyB generatetoaddress 1 "$(C -rpcwallet=keyB getnewaddress)" >/dev/null

    DEST=$(C -rpcwallet=keyB getnewaddress "" bech32m)
    PSBT=$(C -rpcwallet=musig walletcreatefundedpsbt '[]' "[{\"$DEST\":0.5}]" | jq -r .psbt)
    R1=$(C -rpcwallet=musig walletprocesspsbt "$PSBT" | jq -r .psbt)

    # The wallet policy in the two-line form the device registers, with
    # the `h` Core writes in the origin rewritten as the `'` the policy
    # files use. Only the origin: the `h` of an extended key is base58.
    policy_key() {
        local origin=${1%%]*}
        origin=${origin#[}
        printf '[%s]%s\n' "${origin//h/\'}" "${1#*]}"
    }
    {
        echo "tr(musig(@0,@1)/**)"
        policy_key "$PUBA"
        policy_key "$PUBB"
    } > "$D/wallet-musig-regtest.policy"
    printf '%s' "$PSBT" > "$D/wallet-musig-device-first.psbt"
    printf '%s' "$R1" > "$D/wallet-musig-core-first.psbt"

    echo "policy:"
    cat "$D/wallet-musig-regtest.policy"
    echo "address funded: $ADDR"
    echo "wrote $D/wallet-musig-device-first.psbt and $D/wallet-musig-core-first.psbt"
    ;;

finish)
    SIGNED=${3:?signed psbt file}
    start
    P=$(cat "$SIGNED")
    OUT=$(C -rpcwallet=musig walletprocesspsbt "$P")
    echo "walletprocesspsbt: $(echo "$OUT" | jq -c '{complete}')"
    FINAL=$(echo "$OUT" | jq -r .psbt)
    HEX=$(C finalizepsbt "$FINAL" | jq -r .hex)
    echo "finalizepsbt: $(C finalizepsbt "$FINAL" | jq -c '{complete}')"
    C testmempoolaccept "[\"$HEX\"]" | jq -c '.[0]'
    ;;

answer)
    NONCE=${3:?the round-1 psbt}
    start
    P=$(cat "$NONCE")
    OUT=$(C -rpcwallet=musig walletprocesspsbt "$P")
    echo "walletprocesspsbt: $(echo "$OUT" | jq -c '{complete}')"
    echo "$OUT" | jq -r .psbt
    ;;

accept)
    SIGNED=${3:?the round-2 psbt}
    start
    FINAL=$(cat "$SIGNED")
    echo "finalizepsbt: $(C finalizepsbt "$FINAL" | jq -c '{complete}')"
    HEX=$(C finalizepsbt "$FINAL" | jq -r .hex)
    C testmempoolaccept "[\"$HEX\"]" | jq -c '.[0]'
    ;;

*)
    echo "usage: $0 setup|finish|answer|accept|stop <datadir> [psbt]" >&2
    exit 2
    ;;
esac
