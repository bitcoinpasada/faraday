#!/bin/sh
# Two KDBX 4 databases that keepassxc-cli (2.7.10) locks with a key file, for
# `faraday/faraday-vault/tests/keyed.rs`, and the key file it makes for them:
#   keepassxc-keyfile-password.kdbx  passphrase `kdbx vector passphrase` and the key file
#   keepassxc-keyfile-only.kdbx      the key file alone (no passphrase)
#   keepassxc.keyx                   KeePassXC's key file (a KeyFile 2.0 document)
# Both start as pykeepass-aes-argon2d-gzip.kdbx (KDBX 4.0, AES-256, Argon2d at
# 8 MiB, 2 passes, 2 lanes, gzip; entry `Example`, user `alice`, password
# `hunter2`), which keepassxc-cli keeps as KDBX 4 when it edits (its own
# `db-create` writes KDBX 3.1), and gain an entry `Other` (user `bob`,
# password `secret5`). Not reproducible byte for byte (KeePassXC draws its own
# randomness): the files in this folder are the vectors.
#
#   sh make-keepassxc.sh OUT_DIR
set -eu
here=$(dirname "$0")
out=${1:?usage: make-keepassxc.sh OUT_DIR}
mkdir -p "$out"
pw='kdbx vector passphrase'
key="$out/keepassxc.keyx"
[ ! -e "$key" ] || { echo "$key exists" >&2; exit 1; }

with=$out/keepassxc-keyfile-password.kdbx
alone=$out/keepassxc-keyfile-only.kdbx
cp "$here/pykeepass-aes-argon2d-gzip.kdbx" "$with"
cp "$with" "$alone"
printf '%s\n' "$pw" | keepassxc-cli db-edit -q --set-key-file "$key" "$with"
printf '%s\n' "$pw" | keepassxc-cli db-edit -q --set-key-file "$key" --unset-password "$alone"
printf '%s\n%s\n' "$pw" secret5 | keepassxc-cli add -q -k "$key" -p -u bob "$with" Other
printf '%s\n' secret5 | keepassxc-cli add -q -k "$key" --no-password -p -u bob "$alone" Other
printf '%s\n' "$pw" | keepassxc-cli ls -q -k "$key" "$with"
keepassxc-cli ls -q -k "$key" --no-password "$alone"
