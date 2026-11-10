#!/bin/sh
# A vector: `kdbxweb-yubikey-password.kdbx`, made by
# make-yubikey.mjs with kdbxweb 2.1.1 and hash-wasm 4.13.0 (npm), which are
# installed into a scratch directory first.
#
#   sh make-yubikey.sh OUT_DIR
set -eu
here=$(cd "$(dirname "$0")" && pwd)
out=$(mkdir -p "${1:?usage: make-yubikey.sh OUT_DIR}" && cd "$1" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$here/make-yubikey.mjs" "$work/"
cd "$work"
cat > package.json <<'JSON'
{ "private": true, "dependencies": { "kdbxweb": "2.1.1", "hash-wasm": "4.13.0" } }
JSON
npm install --no-audit --no-fund --silent
node make-yubikey.mjs "$out/kdbxweb-yubikey-password.kdbx"
