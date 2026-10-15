# The APK signing key

## What it is

An Android app is signed with a key. A phone installs an update only
when the key that signed it is the key that signed the copy already
installed. Faraday makes this key, keeps it in a vault and signs the
app with it.

## Why a lost key cannot be replaced

An update signed by another key is refused by every phone that has the
app. A new key means a new app: each phone installs it afresh and the
old app's data stays with the old app. For this reason the key has no
copy outside a vault, and a key made here signs nothing until the vault
holds it on two sticks.

## Where the key is

The private key is one record in a vault, with its certificate and the
package it signs for. A key signs APKs of its own package only, so
another app gets another key. The vault written to two sticks kept in
two places is the key's backup. Faraday writes no file, code, paper
form or keystore of the private key.

## The certificate is public

The certificate holds the name typed when the key was made and the
public key. Its SHA-256 is the fingerprint. Anyone can check that an
APK is signed by this key: `apksigner verify --print-certs` prints the
fingerprint as the certificate SHA-256 digest, and F-Droid's
`AllowedAPKSigningKeys` takes the same value. The certificate file and
its QR code are made for sharing.

## Why a new key waits for two sticks

A key made in this session exists only in the memory of the device
until a vault holding it is written. One stick is lost with the key it
holds. Signing starts once the vault has been written to a stick and
then to a second stick, each write read back and matched. A stick that
already holds the vault is refused as the second. On the Pi the two are
SD cards.

## Why signing asks for a second build

The APK copied in came from a build computer that is online. A
compromised computer builds a bad APK and prints that APK's SHA-256,
so the number it prints proves nothing. A second build of the same
source on another computer gives the same SHA-256 when the build is
honest. Signing asks for that number, at least its first 32 hex digits,
and compares it with the file's. A signature on a bad APK cannot be
withdrawn.

## Choosing the vault

The vault for the key is chosen with nothing chosen in advance. Two
things follow from putting it beside other keys. A slot holding an APK
signing key does not open in Faraday up to 0.1.2, nor in a copy of
`open.py` from before the APK key, so nothing else in that slot opens
there either. And one passphrase then guards everything in the slot:
the APK key and, where it is in the same slot, a release's OpenPGP key.

## Signing without Faraday

For the day Faraday cannot be used. Use a computer that is offline and
stays offline until the file is gone.

The computer needs Python 3 with the libraries `open.py` reads Argon2id
and XChaCha20 with (`argon2-cffi` and `pynacl`, or `cryptography` 44 or
newer), Java, and `apksigner` of the Android build-tools, all installed
before it goes offline.

1. Check the APK. Its `sha256sum` equals the second build's, and
`aapt2 dump badging APK` names the package the key is for.

2. Run `python3 tools/vault/open.py VAULT --field 12.1 --out /dev/shm/signing-key.pk8`.
Type the passphrase at the prompt: `--passphrase` is refused, since an
argument stays in the shell's history. `/dev/shm` is memory, not a disk;
`shred` does nothing useful on flash. The file has mode 0600.

3. Run `python3 tools/vault/open.py VAULT --field 12.2 --out /dev/shm/cert.der`
for the certificate.

4. Run `apksigner sign --key /dev/shm/signing-key.pk8 --cert /dev/shm/cert.der APK`.

5. Remove both files and power the computer off before it goes online
again.

This is the one way the private key becomes a file, and only by the
hand of the person who holds the passphrase.
