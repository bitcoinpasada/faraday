#!/usr/bin/env bash
# Makes the paperkey fixtures in this directory with the real tools,
# paperkey 1.6 and GnuPG 2.4 (README.md names each file). Run from the
# repository root:
#
#   tools/vectors/paperkey/make.sh
#
# Every gpg call runs in a throwaway home with its clock set to
# 1,780,000,000 (--faked-system-time), so the keys' dates are fixed; the
# keys themselves are random, so a new run makes new keys and new files.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'gpgconf --homedir "$work/gnupg" --kill all 2>/dev/null || true; rm -rf "$work"' EXIT
mkdir -m 700 "$work/gnupg"

g() {
    gpg --homedir "$work/gnupg" --batch --no-tty --faked-system-time '1780000000!' \
        --pinentry-mode loopback "$@" 2>/dev/null
}
fpr() {
    g --with-colons --list-keys "$1" | awk -F: '/^fpr/ { print $10; exit }'
}

# A Faraday key's secret key, given to paperkey in both forms.
cargo run -q --locked -p faraday-pgp --example paperkey_inputs -- "$work"
paperkey --secret-key "$work/faraday-key.gpg" > "$here/faraday-key.txt"
paperkey --secret-key "$work/faraday-key.gpg" --output-type raw > "$here/faraday-key.raw"

# GnuPG's defaults: an Ed25519 primary and a cv25519 encryption subkey.
g --passphrase '' --quick-generate-key 'Default <default@example.com>'
g --armor --export "$(fpr default@example.com)" > "$here/gpg-default.asc"

# An RSA key.
g --passphrase '' --quick-generate-key 'Rsa <rsa@example.com>' rsa3072
g --armor --export "$(fpr rsa@example.com)" > "$here/gpg-rsa3072.asc"

# Faraday's shape made by GnuPG, with a passphrase and with none.
for which in protected unprotected; do
    pass=''
    [ "$which" = protected ] && pass='test passphrase'
    uid="${which^} <$which@example.com>"
    g --passphrase "$pass" --quick-generate-key "$uid" ed25519 cert never
    fp=$(fpr "$which@example.com")
    g --passphrase "$pass" --quick-add-key "$fp" ed25519 sign never
    g --armor --export "$fp" > "$here/gpg-$which.asc"
    g --passphrase "$pass" --export-secret-keys "$fp" > "$work/$which.gpg"
    paperkey --secret-key "$work/$which.gpg" > "$here/gpg-$which.txt"
done

# paperkey's own check key's shape (papertest-eddsa): an Ed25519 primary
# that certifies and signs, with no subkey.
g --passphrase '' --quick-generate-key 'Single <single@example.com>' ed25519 cert,sign never
fp=$(fpr single@example.com)
g --armor --export "$fp" > "$here/gpg-no-subkey.asc"
g --passphrase '' --export-secret-keys "$fp" > "$work/single.gpg"
paperkey --secret-key "$work/single.gpg" > "$here/gpg-no-subkey.txt"

cd "$here"
sha256sum faraday-key.txt faraday-key.raw gpg-*.asc gpg-*.txt
