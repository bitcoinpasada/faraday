#!/usr/bin/env bash
# Builds the container (faraday/build/device/Containerfile) and runs build.sh
# in it for one board. `just faraday-stick-bin` and `just faraday-pi-bin`
# start it.
#
#   run.sh DOCKER BOARD RELEASE DEV VERSION SOURCE_DATE_EPOCH CACHE
#
# The binaries land in out/faraday/BOARD.
set -euo pipefail

docker="$1" board="$2" release="$3" dev="$4" version="$5" epoch="$6" cache="$7"
root="$(pwd)"
out="out/faraday/$board"
mkdir -p "$out" "$cache/device-$board" "$cache/cargo"
# Not `export FARADAY_BUILD=$(...)`: with `set -e`, export's own exit
# status hides a failure of the command inside it.
FARADAY_BUILD="$(FARADAY_RELEASE="$release" FARADAY_DEV="$dev" FARADAY_VERSION="$version" faraday/image/build-id.sh)"
$docker build -t faraday-build-device -f faraday/build/device/Containerfile faraday/build/device
$docker run --rm \
    -e SOURCE_DATE_EPOCH="$epoch" \
    -e FARADAY_BUILD="$FARADAY_BUILD" \
    -v "$root":/src:ro \
    -v "$root/$out":/out \
    -v "$root/$cache/device-$board":/build \
    -v "$root/$cache/cargo":/cargo \
    faraday-build-device /src/faraday/build/device/build.sh "$board"
sha256sum "$out/faraday" "$out/faraday-disk" "$out/faraday-grant"
