# The APK signing key

**Status:** spec v0.2 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F25; §12 records the answers). Not built. Files of
any size are `docs/LARGE-FILES.md` (F26).

The problem it solves: Android installs an update only when it is
signed by the key that signed the installed copy, so Faraday
Companion's APK key is the one key whose loss cannot be repaired by
making another (`docs/COMPANION.md` §2). COMPANION D7 left it to the
owner, outside Faraday. Faraday already makes, keeps and uses the
release's OpenPGP key in a vault, and its Secure Boot keys the same way;
the APK key is a third kind beside them. Android accepts RSA, ECDSA and
DSA for APK signatures, not Ed25519, so the GPG key cannot sign an APK.

The key is the device's, like GPG and Secure Boot: made on the stick or
the Pi, kept in a vault, used on the stick or the Pi. Nothing of it is
in the Companion. This is a separate document from `docs/COMPANION.md`
because everything in it is built into Faraday (`faraday-core`, a new
crate, the vault format, the device's screens), none of it into the
phone app, and the key can sign any APK, not only the Companion's.

## 0. Rules for the implementing agent

- `CLAUDE.md` and `CLAUDE.local.md` apply in full: every build and test
  through `local/remote-run.sh`; every dimension a token; labels, not
  prose, on working screens (the explanations are §10's Learn page);
  tests for user-facing behaviour only; `just` once at the end of a
  batch; no `rm`; no commits; no agents.
- Batches in §11's order; one review and one completion report each.
- The private key never leaves the vault: no file, code, paper form or
  keystore carries it (§3). A change that would need one stops and asks
  the owner.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What is decided

- One RSA-3072 key per app, with a self-signed certificate naming the
  project, made from the system's entropy (§2).
- APK Signature Scheme v2 and v3, as `apksigner` writes them for an APK
  whose `minSdkVersion` is 26; no v1, no v4, no rotation (§2.2).
- Kept in a vault as record type 12; the vault's copies on sticks are
  its backup (§3).
- **Sign an APK** reads an unsigned, aligned APK copied in from a stick,
  shows what it is, and writes the signed APK For the stick (§4). The
  signed file is the unsigned file with the APK Signing Block inserted
  and one offset changed, so F-Droid can copy the signature onto its own
  build (§6).
- The code is `faraday/faraday-apk`, `no_std` + `alloc`, on
  `faraday-sb`'s RSA, DER and X.509 (§7). It is judged by `apksigner`:
  the signed APK is byte for byte the one `apksigner sign` makes with
  the same key (§8).
- An APK of any size signs: one over 64 MiB is imported whole or only
  for signing, and a signing-only APK's signed file is written on the
  next visit (`docs/LARGE-FILES.md`).
- The owner's answers of 2026-10-14 are §12.

## 2. The key

### 2.1 Algorithm

**RSA-3072, public exponent 65537, signatures RSASSA-PKCS1-v1_5 with
SHA-256** (APK signature algorithm `0x0103`).

| | RSA-2048 | **RSA-3072** | RSA-4096 | ECDSA P-256 |
|---|---|---|---|---|
| Strength | 112-bit | 128-bit | about 140-bit | 128-bit |
| Code in Faraday | `faraday-sb`'s, as is | `faraday-sb`'s, with the size as a parameter | the same | a new dependency (`p256`, `ecdsa`) |
| Digest `apksigner` pairs with it | SHA-256 | SHA-256 | SHA-512 (`0x0104`) | SHA-256 |
| Same bytes as `apksigner sign` | yes | yes | yes | no: Java's ECDSA draws a random nonce |
| Making it on a Pi 3 | seconds | several times RSA-2048 | many times RSA-2048 | instant |

The key cannot change without v3 rotation, which Faraday does not
build, so it has to stay sound for as long as the app is published.
NIST's SP 800-57 accepts 112-bit strength for new signatures only
through 2030; RSA-2048 falls short of the app's life. RSA-3072 is
128-bit, the strength `apksigner` itself pairs with SHA-256 ("3072-bit
RSA is roughly 128-bit strong"), so the chunk digests stay SHA-256.
RSA-4096 adds little strength over 3072 and moves every digest to
SHA-512, a second digest path to write and test, and its key takes the
longest to make on a Pi 3. P-256 needs a new dependency and loses the
byte-identical check against `apksigner` (§8), which is the strongest
test this code can have. PKCS#1 v1.5 is deterministic: the same APK and
key always give the same signature, and so the same signed file.

The time to make one key is measured on a PC and on a Pi 3 in batch 1
and written here. Where it is over two seconds, the Make screen draws
"Making the key" (DESIGN §4.14) before the work starts.

### 2.2 Signature schemes

The Companion's `minSdkVersion` is 26 (Android 8.0). Android verifies
v2 from API 24 and v3 from API 28, and v1 (JAR signing) is needed only
below 24. For `minSdkVersion` 26, `apksigner sign` with its defaults
writes v2 and v3, plus a v4 `.idsig` file beside the APK that only
`adb`'s incremental install reads. Faraday writes:

- **v2** (block ID `0x7109871a`): one signer; signed data with the
  `0x0103` digest, the certificate, and the additional attribute
  `0xbeeff00d` with value 3, which tells a v3-capable verifier that a
  v3 block was there, so stripping it is detected.
- **v3** (block ID `0xf05368c0`): one signer, no lineage; the signer's
  SDK range as `apksigner` writes it for this `minSdkVersion`.
- The padding pair (ID `0x42726577`) that makes the block a multiple of
  4096 bytes, as `apksigner` pads it.
- Nothing else: no v1, no v4 file, no v3.1, no source stamp, no
  verity digests.

Where this list and `apksigner` differ in a byte, `apksigner` is right:
the reference is the `apksigner` of the build-tools version the release
container pins (`docs/COMPANION.md` §3), and §8's test compares whole
files.

An APK whose `minSdkVersion` is below 24 is refused (§4.4): it would
need v1, which Faraday does not write.

### 2.3 Making it

32 bytes from the system's entropy (`fresh`, as Secure Boot's keys
take them) key a ChaCha20 stream (`faraday-sb`'s `rng::Stream`), from
which the `rsa` crate draws the primes; a second stream from the same
bytes draws the serial and blinds the signing.

The key is not derived from a seed a person could write down. The prime
search belongs to the `rsa` and `crypto-primes` crates and may change
between their versions, so a later build could make another key from
the same seed. The vault keeps the key itself (§3).

### 2.4 The certificate

Made once with the key, kept beside it, never made again.

- X.509 v3, self-signed, `sha256WithRSAEncryption`.
- Subject and issuer: one common name, typed when the key is made,
  **Faraday Companion** filled in. Nothing else: no organisation, no
  place, no email. `apksigner verify --print-certs` prints it to anyone
  who has the APK.
- Serial: 16 bytes from the stream, top bit cleared, as Secure Boot's.
- Valid from the time it was made (the clock must be known: GPG and
  Secure Boot ask for the time first, `docs/FLOWS.md` decision 58), until
  9999-12-31 23:59:59 UTC, RFC 5280 §4.1.2.5's value for a certificate
  with no expiry. Android does not check an APK certificate's dates.
- One extension, the subject key identifier. Not a CA: no basic
  constraints, no key usage, unlike Secure Boot's.

**The fingerprint** is the SHA-256 of the certificate's DER: what
`apksigner verify --print-certs` prints as "certificate SHA-256 digest",
what F-Droid's `AllowedAPKSigningKeys` takes (lower-case hex, no
separators), and what the Companion's About shows (`docs/COMPANION.md`
§4.8). Faraday shows it in groups of four, lower case, as it shows an
image's SHA-256.

## 3. Where it lives

- **Record type 12, APK signing key** (`docs/VAULT.md` §7): field 1 the
  private key, PKCS#8 DER; field 2 the certificate, DER. Both required.
  The name is the certificate's common name, read from field 2, as
  Secure Boot's are. A vault may hold several.
- **The second copy is the vault's.** A sealed vault written to two
  sticks kept apart holds the key twice. Nothing else copies it. Given
  the passphrase, `tools/vault/open.py` reads it out of a vault without
  Faraday (`docs/VAULT.md` §10), as PKCS#8 that `apksigner --key` takes:
  that is the way out if Faraday itself cannot be used.
- **The private key never leaves the vault.** No file, no code, no
  paper form, no PKCS#12 or JKS keystore. What reaches For the stick is
  the certificate (§4.3) and signed APKs (§4.4).
- **Signing waits for a written copy.** A key made in this session
  signs nothing until the vault holding it has been written to a stick
  after the key was added (the stick visit's receipt, `docs/VAULT.md`
  §6). Until then the key's **Sign an APK** reads **Write the vault to
  a stick first** and opens the stick visit with the vault ticked; once
  written, it returns to the key's page. A key in a vault unlocked from
  a stick signs at once. This is the rule `docs/NEW-WALLET.md` §14.3
  sets for a Bitcoin key made here, for the same reason: a key whose
  only copy is in memory is gone at power-off.
- **Deleting** is Vault contents' hold, as for every item. The Learn
  page (§10) says what losing the key means; the hold carries no extra
  line.
- **Older builds.** A slot holding a type 12 record does not open in a
  build that does not know the type (`docs/VAULT.md` §7: an unknown
  type refuses the slot), which is every release up to 0.1.2.

## 4. Screens and flow

On the stick and the Pi. Faraday Desktop has the same screens, as it
has GPG's and Secure Boot's (§12). Drawn from DESIGN §5's
screens; every string in `faraday/faraday-strings/src/en.rs` with the
prefix `apk_`, the Tools tile's with `catalog_`. `docs/FLOWS.md`'s
"APK signing key" section is the flow.

### 4.1 Where it starts

- Tools, group "Vaults and …": a tile **APK signing key**, line "Make a
  key, sign Android apps", tags "Android" and "APK v2 · v3". It opens
  the open vault's new category **APK signing keys**
  (`vaults::CATEGORIES`, after Secure Boot), and needs an open vault as
  GPG's tile does (`docs/FLOWS.md` decision 10).
- Vault contents: **Make an APK key** among the add actions (decision
  58).
- Files: an APK copied in carries **Sign with APK key** on its row, as
  `BOOTX64.EFI` carries **Sign in Secure Boot** (decision 1).

### 4.2 Make an APK key

Step cards (wide) or pages (small): **Name on the certificate** (one
field, **Faraday Companion** filled in, the hint "Public: printed by
anyone who checks the APK"), then the vault. The time first where it is
not known. **Make the key** → Result, "APK key made": Name, **Certificate
SHA-256** (a reference row opening Compare), Key "RSA 3072", and the
certificate For the stick as it is made (as GPG's public key is), its
row carrying **Show as QR** and **PNG**. Actions: **Write the vault to a
stick** (primary, §3) and **Done** → the key's page.

### 4.3 The key's page

A Menu in Vault contents: **Name**, **Certificate SHA-256** (reference
row → Compare), **Key** "RSA 3072", **Made** (the date), then
**Certificate** (written For the stick again; **Show as QR**, **PNG**),
**Sign an APK**, and the hold to delete.

The certificate file is PEM, one `CERTIFICATE` block,
`{name}-{first 8 hex of the fingerprint}-certificate.pem` with the name
lower-cased and every other character a hyphen, as GPG's file stem;
its PNG `{same stem}.png` with the name and fingerprint under the code.
Files reads a file that is exactly one PEM `CERTIFICATE` block as
public, as F22 reads an armored public key.

### 4.4 Sign an APK

**Getting the APK in.** A stick visit, as for `BOOTX64.EFI`. On the Pi,
which has no USB, an SD card visit: the unsigned APK is put on an SD
card's exchange partition on another computer, and the card is put in
the Pi after boot (`PLAN.md` §3). A code cannot carry it: Faraday's file
envelope stops at 256 KiB and the APK is megabytes. Any size comes in:
up to 64 MiB as any file, and a larger one after the choice between the
whole file and only what signing needs (`docs/LARGE-FILES.md` §3). A
signing-only APK is listed, reviewed and signed as below; its signed
file is written on the next visit (`docs/LARGE-FILES.md` §6, §7).

**Which files.** A file copied in whose ZIP central directory names
`AndroidManifest.xml` is an APK, whatever its name. **Sign an APK**
lists every APK in Files, newest first: name, size, and, on one that
cannot be signed, why, in `WARN`, the row inert:

| Refused | When |
|---|---|
| Not an APK | not a ZIP Faraday reads (§5), or no `AndroidManifest.xml` |
| Already signed | an APK Signing Block before the central directory, or a v1 signature (`META-INF/*.SF`, `.RSA`, `.EC`, `.DSA`) |
| Not aligned | a stored entry's data not at a multiple of 4 bytes, or a stored `.so` not at a multiple of 16 KiB (`zipalign -c -P 16 4`) |
| Supports Android before 7.0 | `minSdkVersion` below 24 (§2.2) |
| Over 4 GiB on FAT32 | the signed APK would pass FAT32's largest file |

None copied in: "Copy the APK in on a stick visit first." (the Pi: "on
an SD card visit").

**The review** is a Hold screen, "Sign this APK?":

- **File**: the name as copied in; **Size**.
- **SHA-256**: of the file as copied in, in groups of four, to compare
  with what the release build printed.
- **Package**, **Version** ("0.1.3 (4)": `versionName` and
  `versionCode`), **Android** ("8.0 and later", from `minSdkVersion`).
- **Key**: its name and the fingerprint's first 16 hex digits.
- **Will write**: the output name.

The hold signs. Reading the package and version from the binary
manifest is worth its parser (§5): it is how a person sees which app and
which version the key is about to vouch for, which the file name does
not prove, and `minSdkVersion` is needed anyway for §2.2's refusal.

**The output** goes For the stick: the name copied in with
`-unsigned` removed before `.apk`, so the release build's
`faraday-companion-X.Y.Z-android-unsigned.apk` comes out as
`faraday-companion-X.Y.Z-android.apk` (`docs/COMPANION.md` §2); a name
without `-unsigned` gains `-signed`. Then "Insert a stick to write it",
as every result For the stick. The unsigned file stays in Files.

## 5. What Faraday reads of an APK

Untrusted input (`PLAN.md` §4.4), bounded and fuzzed
(`faraday/fuzz/fuzz_targets/apk.rs`). Faraday reads:

- **The ZIP's end.** The end of central directory record, searched
  backwards over at most its 22 bytes and a 65,535-byte comment; exactly
  one; no ZIP64, no multiple disks; the central directory directly
  before it and wholly inside the file.
- **The central directory**: each entry's name, method, sizes and local
  header offset; the local headers in the order of their offsets, each
  inside the entries' section, none overlapping, each with the name its
  central entry has; the 16 bytes before the central directory, for an
  APK Signing Block's magic.
- **`AndroidManifest.xml`**, stored or deflated (`miniz_oxide`, already
  in the graph, `docs/deps/miniz_oxide.md`), inflated up to 1 MiB.
- **The binary manifest**: its string pool, the `manifest` element's
  `package`, `versionCode` and `versionName`, and `uses-sdk`'s
  `minSdkVersion`. Nothing else: not resources, not other entries'
  contents, not `classes.dex`, not the native libraries.

Anything that does not read is "Not an APK"; nothing panics or
allocates past the file's size and the manifest's bound.

The reader takes the file in parts, not as one buffer: the end, the
central directory (over 16 MiB is "Not an APK"), the manifest entry,
and each local header as the entries pass. So it reads a whole file in
memory and a signing-only file streamed from a stick
(`docs/LARGE-FILES.md` §5, §9) the same way.

## 6. What Faraday writes

From the unsigned bytes `U`, whose central directory starts at offset
`C`:

1. The content digest over three sections: the entries (`U[0..C]`), the
   central directory, and the end record with its central directory
   offset read as `C`. Each section is cut into 1 MiB chunks (the last
   shorter); each chunk's digest is SHA-256 of `0xa5`, its length as a
   `u32` little-endian, and its bytes; the top digest is SHA-256 of
   `0x5a`, the chunk count as a `u32`, and every chunk digest in order.
   One digest serves v2 and v3.
2. The v2 and v3 signer blocks (§2.2), each signed data signed with
   the key, each carrying the certificate and the public key
   (SubjectPublicKeyInfo DER).
3. The APK Signing Block: its size, the ID-value pairs, the padding
   pair, its size again, and `APK Sig Block 42`.
4. The output: `U[0..C]`, the block, the central directory unchanged,
   and the end record with its central directory offset raised by the
   block's length. Nothing else changes: no entry is re-aligned,
   re-compressed or renamed.

For a signing-only APK, steps 1 to 3 come from the chunk digests and
bytes kept when it was read, and step 4 is the copy of the next visit
(`docs/LARGE-FILES.md` §7).

So removing the block and restoring the offset gives back `U` byte for
byte, which is what F-Droid's `apksigcopier` relies on to copy the
signature onto F-Droid's own build of the same tag
(`docs/COMPANION.md` §8).

## 7. Crates

- **`faraday/faraday-apk`** (new), `no_std` + `alloc`: the reader of §5,
  the signer of §6, the key and certificate of §2. It takes a file in
  parts and keeps the chunk digests, never needing the whole file.
  Depends on `faraday-sb` (`der`, `rng`, `x509`, already public),
  `sha2`, `zeroize` and `miniz_oxide`; no new crate in the graph. It writes nothing to a
  vault and draws no randomness of its own.
- **`faraday-sb`**: `x509::generate` takes the key's size and a profile
  (Secure Boot's CA certificate, or the APK's end certificate with no
  expiry). Secure Boot's output is unchanged, which its judged tests
  show. Moving `der`, `rng` and `x509` into a crate of their own waits
  for a third user.
- **`faraday-core/src/apk.rs`**, beside `gpg.rs` and `secureboot.rs`:
  the record, the screens' state, Make, Sign, the wait for a written
  copy.
- **`faraday-vault`**: `kind::APK_KEY = 12`. `tools/vault/open.py` reads
  type 12, and the vectors in `tools/vectors/vault/` gain a type 12
  record, so each slot still holds one record of every type
  (`docs/VAULT.md` §10).
- **`docs/deps/rsa.md`** names 3072 beside 2048; **`miniz_oxide.md`**
  names the new edge.
- The Companion's lint (`tools/lint-companion-deps.sh`) adds
  `faraday-apk` to what `faraday-companion` must not depend on.

## 8. Tests

User-facing behaviour and agreement with Android's own tools
(`CLAUDE.md`).

**Fixtures.** `tools/vectors/apk/make.sh` makes them from Android's own
tools at the build-tools version the release container pins (`aapt2`,
`zipalign`), and the small ones are committed with it:

- `small-unsigned.apk`: a manifest-only app (`hasCode="false"`,
  package `app.faradayoffline.fixture`, version `1.0` (1),
  `minSdkVersion` 26), a stored `lib/arm64-v8a/libfixture.so` of a few
  KiB, aligned with `zipalign -P 16 4`.
- `old-sdk-unsigned.apk` (`minSdkVersion` 21) and
  `unaligned-unsigned.apk` (the small one with its `.so` moved off its
  boundary).
- `large-unsigned.apk`, made by the script in the test's scratch
  directory and not committed: the small one with a stored 2.5 MiB
  asset of fixed bytes, so every section crosses a chunk boundary.

**The key** is made by Faraday's own code from a fixed seed, as
`faraday-sb/tests/judged.rs` makes its keys, and written to scratch as
PKCS#8 and DER for `apksigner --key` and `--cert`. No key is committed.

**Judged by `apksigner`** (`faraday-apk/tests/judged.rs`), found as
`FARADAY_APKSIGNER` or under `$ANDROID_HOME/build-tools/<pinned>/`, and
skipped with a note where missing, as the Secure Boot tests skip without
`sbverify`. The build machine has the Android SDK, so the check runs
there:

- `apksigner verify --verbose --print-certs` accepts each signed
  fixture: verified by v2 and v3, one signer, and the certificate
  SHA-256 it prints is the one Faraday shows.
- `apksigner sign --v1-signing-enabled false --v2-signing-enabled true
  --v3-signing-enabled true --v4-signing-enabled false
  --alignment-preserved true` with the same key on the same unsigned
  APK makes a file byte for byte equal to Faraday's, for the small and
  the large fixture. Without `--alignment-preserved`, `apksigner` may
  rewrite the entries' alignment padding, which §6 never does.
- Removing the block from Faraday's output and restoring the offset
  gives back the unsigned file.

**In `faraday-core/tests/apk.rs`**, through `Action`s as `gpg.rs` does:

- Make an APK key puts a type 12 record in the open vault, and the
  certificate For the stick; the vault sealed and opened again shows the
  same name and fingerprint.
- A key made this session offers **Write the vault to a stick first**,
  and signs once the vault has been written; a key unlocked from a
  stick signs at once.
- Each refusal of §4.4 on its fixture (and a PSBT, a PNG, random bytes,
  `apksigner`'s own signed output for "Already signed"); the review's
  package, version and Android line for the small fixture; the output's
  name.
- Nothing For the stick ever holds the private key: after every
  action, no Outbox file contains the PKCS#8 bytes.

A signing-only APK, and the 200 MiB fixture, are tested in
`docs/LARGE-FILES.md` §11.

## 9. The release

`docs/COMPANION.md` §10 batch C, with the key in it:

1. The pinned container builds
   `faraday-companion-X.Y.Z-android-unsigned.apk`, checks it with
   `zipalign -c -P 16 4`, and prints its size and SHA-256.
2. On a Faraday stick (or Pi), in one session: a stick visit brings the
   unsigned APK in; **Sign an APK** after comparing the SHA-256 with
   step 1's; the signed APK goes out.
3. `faraday/tools/stage-release.sh` gains `--companion FILE`: it runs
   `apksigner verify --print-certs`, refuses an APK whose certificate is
   not `faraday/companion-cert.pem`'s, and adds the APK and its line to
   `SHA256SUMS`.
4. A second visit in the same session signs `SHA256SUMS` with the
   release key, as before.

`faraday/companion-cert.pem` is the certificate, committed once the key
is made, as `faraday/release-key.asc` is. README's "Verifying a
release" gains a paragraph: the APK is signed with an APK key made in
Faraday whose private part has never left a Faraday vault; its
certificate is that file; its SHA-256 fingerprint in full; and
`apksigner verify --print-certs faraday-companion-X.Y.Z-android.apk`
prints the same as "Signer #1 certificate SHA-256 digest". F-Droid's
metadata names the same fingerprint in `AllowedAPKSigningKeys`.

## 10. Learn page

One page, `docs/learn/faraday/apk-key.md`, opened from the key's page
and from the Tools tile's info: what an APK signing key is; that
Android installs an update only when the key matches, so a lost key
means a new app that every phone installs afresh, losing what the old
one kept; that the key lives only in the vault, and the vault written
to two sticks kept apart is its backup; that the certificate and its
fingerprint are public, and how anyone checks an APK against it
(`apksigner verify --print-certs`, F-Droid's page); why a key made here
waits for the vault to be written before it signs. Working screens
carry none of this (`CLAUDE.md`).

## 11. Batches

- **1, the key and the signer.** `faraday-apk` (§5, §6, §2), the
  `faraday-sb` change, record type 12 with `open.py` and the vectors,
  `faraday-core/src/apk.rs`, the screens of §4, the Learn page, §8's
  tests and the fuzz target, the docs of §7. Needs no release build:
  the fixtures stand in. Renders: the tile, Make, the key's page and the
  review at `1280x800` and `480x640`.
- **2, with the Companion's batch C.** The container's step 1, the
  staging script's step 3, the README paragraph, the fastlane metadata
  and `AllowedAPKSigningKeys`.
- **The owner**, between them: the key made on the stick of a published
  release that has batch 1, in the vault that holds the release's
  OpenPGP key, that vault written to two sticks, and the certificate
  committed as `faraday/companion-cert.pem` (§12).

Batch 1 signs APKs up to today's 18 MiB read; any size comes with
`docs/LARGE-FILES.md`'s batches L1 and L2, after batch 1. Batch 2 does
not wait for them while the release APK stays under 18 MiB.

## 12. Decided by the owner (2026-10-14)

1. RSA-3072 (§2.1).
2. Faraday Desktop offers the APK key, as it offers GPG and Secure
   Boot; the release's key is made on a stick regardless.
3. Signing waits for a written vault (§3).
4. The real key is made on the stick of a published release that has
   batch 1, so the build that made it is one anyone can reproduce.
5. It is kept in the vault that holds the release's OpenPGP key, so the
   same copies back up both.
6. The certificate's name is "Faraday Companion".
