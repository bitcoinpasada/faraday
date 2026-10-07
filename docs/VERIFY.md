# Verify a release

No release exists yet and no release key exists, so these steps describe
what checking a release will look like rather than something you can do
today. Nothing published anywhere is an OpenSigner release.

Every OpenSigner release is published on GitHub with two extra files:

- `manifest.txt` — the SHA-256 sum of each file in the release, plus a
  header line naming the version and the commit it was built from.
- `manifest.txt.asc` — a detached signature over `manifest.txt`, made
  with the OpenSigner release key.

Checking them tells you two things: the file you downloaded is the file
that was published (the sum matches), and it was published by whoever
holds the release key (the signature verifies). A signing device is worth
checking. Do it every time.

You need GnuPG (`gpg`) and `sha256sum`. On macOS, `brew install gnupg`
and use `shasum -a 256` where these steps say `sha256sum`.

## The release key

```
FINGERPRINT PLACEHOLDER — the owner fills this in when the release key
exists. Until this line is replaced by a fingerprint, there is nothing to
check an imported key against, and no release has been signed.
```

## Steps

Download the artifact you want, `manifest.txt`, `manifest.txt.asc` and
`pubkey.asc` into the same directory, then:

1. **Import the key.**

   ```
   gpg --import pubkey.asc
   ```

2. **Check the fingerprint.** Print the fingerprint of what you just
   imported and compare it, character by character, with the one above.

   ```
   gpg --fingerprint
   ```

   If they differ, stop. The key you have is not the release key, and
   nothing it signs means anything.

3. **Verify the manifest.**

   ```
   gpg --verify manifest.txt.asc manifest.txt
   ```

   Good:

   ```
   gpg: Good signature from "OpenSigner releases <...>"
   ```

   A `WARNING: This key is not certified with a trusted signature`
   underneath is normal. It says you have not told GnuPG you trust the
   key, which is exactly what step 2 did by hand. What matters is the
   words `Good signature`.

   Bad:

   ```
   gpg: BAD signature from "..."
   ```

   or `Can't check signature: No public key`. Either way, stop: the
   manifest has been altered, or it was not signed by this key.

4. **Check the file.**

   ```
   sha256sum -c manifest.txt --ignore-missing
   ```

   Good — one line per file you actually downloaded:

   ```
   opensigner-0.1.0-linux-x86_64: OK
   ```

   Bad:

   ```
   opensigner-0.1.0-linux-x86_64: FAILED
   sha256sum: WARNING: 1 computed checksum did NOT match
   ```

   A failure means the file on your disk is not the released file.
   Delete it and download again; if it fails a second time, do not use
   it.

`--ignore-missing` is there because the manifest lists every artifact in
the release and you have downloaded one or two of them. Without it,
`sha256sum` reports the files you did not download as failures.

## What to do with each artifact

Verify first, then:

**`opensigner-VERSION-pi3-waveshare-28dpi.img` — the Raspberry Pi card.**
Write the image to a microSD card with `dd` or a tool like Raspberry Pi
Imager (choose "use custom image"), put the card in the board and power
it on. `dd` writes to the whole card, not a partition, and the device
name is the card's, not your disk's — check it twice:

```
sudo dd if=opensigner-VERSION-pi3-waveshare-28dpi.img of=/dev/sdX bs=4M conv=fsync status=progress
```

The image is built for a Pi 3 with the Waveshare 2.8" panel.
`opensigner/shells/pi/image/README.md` covers other boards and panels,
and what the first boot looks like.

**`opensigner-VERSION-android-unsigned.apk` — the Android app.** The
release APK is built unsigned, because it is what a second builder
reproduces bit for bit and because there is no release signing key yet
(`docs/PLANNING.md` §15 item 33). Android installs a signed APK and
nothing else, so until that key exists the published APK is a file to
check and to compare, not one to install; build a development APK with
`just android-apk` to try the app, and use test seeds only.

Once releases are signed, the published APK is the signed one: the same
file with a signature added. Adding a signature changes the `META-INF`
entries and the signing block between the entries and the directory, and
nothing else, so a rebuild is compared against it with the signature set
aside rather than byte for byte. Two ways:

```
# Who signed it, and with which certificate.
"$ANDROID_HOME/build-tools/36.0.0/apksigner" verify --print-certs published.apk

# The F-Droid method: apksigcopier lifts the signature off the published
# APK and puts it on your rebuild, then compares the two whole files.
apksigcopier compare published.apk --unsigned your-rebuild.apk
```

`apksigcopier compare` printing nothing means the two APKs are the same
archive with the same signature, which is what "reproduced" means for an
APK.

**`opensigner-VERSION-linux-x86_64` — the Linux desktop binary.** Make
it executable and run it:

```
chmod +x opensigner-VERSION-linux-x86_64
./opensigner-VERSION-linux-x86_64
```

It needs the X11 or Wayland client libraries, including
`libxkbcommon-x11`. `--camera /dev/video0` names a camera; `--no-camera`
refuses one, so that reading codes from files can be tried.

**`opensigner-VERSION-macos.zip` — the macOS application bundle.**
Unzip it and move `OpenSigner.app` where you keep applications. It is
ad-hoc signed and not notarised, so a double-click is refused: right-click
the app, choose **Open**, then **Open** again in the dialog. That is
needed once per copy. macOS grants the camera permission to the bundle,
which is why the Mac artifact is an app and not a bare binary.

## Checking a signature's nonce

A signature's nonce is a one-time secret. A signer that chooses it freely
can hide key material in it, and nothing in the signature itself would
show that. OpenSigner derives every nonce from the key and the message
alone — RFC 6979 for ECDSA, and, as it comes, BIP-340 with no auxiliary
randomness for Schnorr — so the same key over the same transaction always
produces the same signature bytes, and a second implementation holding
the same words produces them too.

Two ways of picking the RFC 6979 nonce are both standard, and Settings ›
Nonce chooses between them. They give different bytes, so the comparison
only works when both signers use the same one.

| Setting | What it does | Matches |
|---|---|---|
| `Low R` (default) | Plain RFC 6979 first, then retries with a little-endian counter as the RFC's additional data while the signature's `r` is 33 DER bytes, so the signature comes out in its shortest form. | Bitcoin Core, Sparrow and Electrum for transactions; Sparrow and Electrum for messages. |
| `First` | The first RFC 6979 nonce, no additional data. | RFC 6979's own vectors, BIP-174's role vectors, Trezor, and Bitcoin Core's message signing (`SignCompact`). |
| `Schnorr: Deterministic` (default) | BIP-340 with all-zero auxiliary randomness, for taproot inputs and taproot BIP-322 messages. | Any implementation that signs with `sign_schnorr_no_aux_rand`, which is what BIP-340's own test vector 0 through 3 use. |
| `Schnorr: Fresh randomness` | 32 bytes drawn from the session key for every signature, mixed into the BIP-340 nonce. It blinds the nonce against a fault or side-channel attack, and no two signatures of the same input match. | Nothing: a taproot signature made this way cannot be compared byte for byte. |

Bitcoin Core grinds for a low `r` when it signs a transaction and does
not when it signs a message, so a message signature from Core is
compared against `First` even though its transactions are compared
against `Low R`.

That is what makes the comparison possible. On an offline machine, with
test seeds:

1. Load the same words into a second implementation — Sparrow or Bitcoin
   Core will do — with the same script type, path and network.
2. Set Settings › Nonce to the scheme that implementation uses, and, for
   a taproot input, Settings › Schnorr to `Deterministic`.
3. Sign the same PSBT there.
4. On OpenSigner, sign the same PSBT and open the Signatures screen.
5. Compare the signature bytes for each input, character by character.

Identical bytes mean the nonce carried nothing of its own. Bytes that
differ mean the two signers did not derive the same nonce, which is worth
running down before that key holds anything.

The comparison is a check you run, not one the wallet runs for you.
Schemes exist in which the wallet contributes to the nonce and verifies
afterwards that the device did not pick it alone; OpenSigner does not
implement one yet.

## Reproduce it yourself

Verification tells you a file matches what was published. It does not
tell you the published file matches the source. That check is building
the artifact yourself, from the tagged source, and getting the same
bytes. A release is blocked until two people on two machines have done
it (`docs/PLANNING.md` §11.1).

Every release artifact is built in a container whose base image is pinned
by digest and whose compilers are pinned by version, with the commit date
as `SOURCE_DATE_EPOCH`, so the build depends on the commit and not on the
machine. You need `git`, `just` and Docker — rootless Docker is what the
recipes are written for — and no toolchain of your own.

```
git clone https://github.com/<owner>/opensignerkit
cd opensignerkit
git checkout v0.1.0            # the tag the release names

just reproduce                 # the Linux binary and the APK, twice each
just pi=1 reproduce            # and the Pi card image, which takes hours
```

`just reproduce` builds each artifact twice from clean, prints the
SHA-256 of both runs, and fails if any pair differs. Compare the printed
sums with the lines in `manifest.txt`:

- the Linux binary and the two `libopensigner.so` libraries are
  bit-identical;
- the APK is bit-identical while releases are unsigned, and identical
  with the signature set aside once they are signed (above);
- the Pi image is bit-identical.

A single artifact, without the double build:

```
just linux-bin                 # out/linux/opensigner-desktop
just android-release-apk       # out/android/release/
just pi-image                  # out/pi/opensigner-pi-<board>-<panel>.img
```

If a sum differs from the manifest, say so publicly: either the published
file was not built from that source, or the build has a dependence on the
machine that neither builder has found yet. Both are worth knowing.
