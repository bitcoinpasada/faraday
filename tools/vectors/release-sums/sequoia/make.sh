#!/usr/bin/env bash
# Makes the Sequoia fixtures in this directory with sq (README.md in the
# folder above names each file). Run from the repository root:
#
#   tools/vectors/release-sums/sequoia/make.sh
#
# sq runs with no certificate store or key store, in a throwaway home,
# with its clock set to 2026-05-28T20:26:40Z (1,780,000,000), so the
# key's and the signature's dates are fixed; the key itself is random, so
# a new run makes a new key and new files. The secret key is not kept.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
sums="$here/../SHA256SUMS"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

s() {
    sq --home "$work/home" --cert-store=none --key-store=none --time 2026-05-28T20:26:40Z "$@"
}
mkdir -m 700 "$work/home"

# A key of the shape of the release key: an Ed25519 primary that
# certifies and an Ed25519 subkey that signs, expiring in five years.
s key generate --own-key --userid 'Sequoia <sequoia@example.com>' \
    --cipher-suite cv25519 --profile rfc4880 --without-password \
    --cannot-authenticate --cannot-encrypt --expiration 5y \
    --output "$work/secret.pgp" --rev-cert "$work/rev.asc"
s key delete --overwrite --cert-file "$work/secret.pgp" --output "$here/sequoia-key.asc"

# The test release's sums, signed as the release is: a detached,
# armoured, binary-mode signature.
s sign --signer-file "$work/secret.pgp" --signature-file "$here/SHA256SUMS.asc" "$sums"

# sq's own verdict, for the README.
s verify --signer-file "$here/sequoia-key.asc" --signature-file "$here/SHA256SUMS.asc" "$sums"
cd "$here"
sha256sum sequoia-key.asc SHA256SUMS.asc
