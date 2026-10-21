# The APK signing key

**Kind:** Spec
**Status:** Built; batch 2 as far as the key allows

Spec v0.4 · 2026-10-14 · decided by the owner
(`docs/DECISIONS.md` F25; §12 records the answers), revised after an
adversarial review the same day. Batch 1 (§11) is built, with the changes
the build made (§13); its wait for two copies is now the shared backup
flow's, `docs/BACKUP-FLOW.md` (F31), built in batch B1a.
Batch 2 is built as far as the key allows (§11); `docs/LARGE-FILES.md`
(F26) is not built.

**Switched off (F59, 2026-10-20).** APK signing is not in the app: the
switch `Faraday::apk_signing` is off by default. Everything here is
built and tested with the switch on, and comes back when it is turned on.

**Faraday Companion was dropped (F34, 2026-10-15).** This spec was
written for the Companion's APK key. **Sign an APK** stays, for the APK
of any app: the name and the package start empty, and the release
carries no APK. Where this text names the Companion, its APK, its
container, docs/COMPANION.md or faraday/companion-cert.pem, that is
history: §9 and batch 2 of §11 are not built and have no tool left to
build them with, and F34 says how the Companion comes back.

The problem it solves: Android installs an update only when it is
signed by the key that signed the installed copy, so an app's APK key
is the one key whose loss cannot be repaired by making another. Faraday
already makes, keeps and uses the
release's OpenPGP key in a vault, and its Secure Boot keys the same way;
the APK key is a third kind beside them. Android accepts RSA, ECDSA and
DSA for APK signatures, not Ed25519, so the GPG key cannot sign an APK.

The key is the device's, like GPG and Secure Boot: made on the stick or
the Pi, kept in a vault, used on the stick or the Pi. Everything in it is built into Faraday (`faraday-core`, a new
crate, the vault format, the device's screens), and the same feature
makes and keeps a key for any app, each key bound to its own app's
package.

## 0. Rules for the implementing agent

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens (the explanations are §10's Learn page);
  tests for user-facing behaviour only; `just` once at the end of a
  batch; no `rm`; no commits; no agents.
- Batches in §11's order; one review and one completion report each.
- The private key never leaves the vault: no file, code, paper form or
  keystore that Faraday makes carries it (§3); `open.py --field`, run by
  the person without Faraday, is the one way out. A change that would
  need another stops and asks the owner.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.

## 1. What is decided

- One RSA-3072 key per app, with a self-signed certificate naming the
  project, made from the system's entropy (§2). Each key is bound to
  one package, typed when it is made, and signs only APKs of that
  package; another app gets another key, as Android's own advice has
  it (§3, §4.4).
- APK Signature Scheme v2 and v3, as `apksigner` writes them for an APK
  whose `minSdkVersion` is 26; no v1, no v4, no rotation (§2.2).
- Kept in a vault as record type 12; the vault's copies on sticks are
  its backup: the key is made by the backup flow into the vaults its
  plan names (two places or more), and signs nothing until every place
  is written (§3, `docs/BACKUP-FLOW.md`).
- **Sign an APK** reads an unsigned, aligned APK copied in from a stick,
  shows what it is, takes the SHA-256 of an independent second build of
  the same source, signs only when the two match, checks each signature
  against the certificate, and writes the signed APK For the stick
  (§4, §6). The
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

**Measured** (batch 1, a release build of `faraday-apk` on the build
machine's PC): twelve keys from twelve seeds took 49 to 534 ms, 180 ms
in the middle; signing the 22 KB fixture took 8 ms. The same build with
the big-integer crates' debug assertions on took 35 s a key, which is why
`Cargo.toml` turns them off for `crypto-bigint`, `crypto-primes` and
`rsa` in dev builds. A Pi 3 was not measured; a Cortex-A53 core is
roughly 10 to 30 times slower than a PC core on this work. The Make
screen draws "Making the key" before the work starts, whatever the time.

### 2.2 Signature schemes

The `minSdkVersion` Faraday signs for is 26 (Android 8.0), the
Companion's while it existed. Android verifies
v2 from API 24 and v3 from API 28, and v1 (JAR signing) is needed only
below 24. For `minSdkVersion` 26, `apksigner sign` with its defaults
writes v2 and v3, plus a v4 `.idsig` file beside the APK that only
`adb`'s incremental install reads. Faraday writes:

- **v2** (block ID `0x7109871a`): one signer; signed data with the
  `0x0103` digest, the certificate, and the additional attribute
  `0xbeeff00d` with value 3, which tells a v3-capable verifier that a
  v3 block was there, so stripping it is detected.
- **v3** (block ID `0xf05368c0`): one signer, no lineage; the signer's
  SDK range (its minimum and maximum, inside the signed data and
  beside it). **Measured**: `apksigner` 36.0.0 writes the range
  24 to 2147483647 (`0x7fffffff`) for fixtures whose `minSdkVersion` is
  21, 26 and 33 alike, so the minimum does not follow the APK's: it is
  always 24, Android 7.0, the oldest an APK Faraday signs supports
  (§4.4). The byte-identical test (§8) holds it for 26 and 33.
- The padding pair (ID `0x42726577`) that makes the block a multiple of
  4096 bytes, as `apksigner` pads it, and before the block, zero bytes
  after the last entry that put the block's start on a multiple of 4096
  (§6). The v2 signed data ends with an empty fourth element, as
  `apksigner` writes it.
- Nothing else: no v1, no v4 file, no v3.1, no source stamp, no
  verity digests.

Where this list and `apksigner` differ in a byte, `apksigner` is right:
the reference is the `apksigner` of the build-tools version the pinned
build-tools of `tools/vectors/apk/make.sh`, and §8's test compares whole
files.

An APK whose `minSdkVersion` is below 24 is refused (§4.4): it would
need v1, which Faraday does not write.

### 2.3 Making it

32 bytes from the system's entropy (`fresh`, as Secure Boot's keys
take them) key a ChaCha20 stream (`faraday-sb`'s `rng::Stream`), from
which the `rsa` crate draws the primes; a second stream from the same
bytes draws the serial and blinds the certificate's self-signature.
Those bytes are not kept.

**Every later signing is blinded with bytes of its own**, as Secure
Boot's signing is (`faraday-core/src/secureboot.rs`): each **Sign an
APK** takes `fresh(b"apk signing")`, keys a new `rng::Stream` with it,
and passes that to `faraday-apk`, which draws no randomness of its own
(§7). With no answer from the system yet, signing is refused as
Secure Boot's is. A fixed or reused stream is never passed: the signature
would be the same, but the blinding would be known and so worthless.

The key is not derived from a seed a person could write down. The prime
search belongs to the `rsa` and `crypto-primes` crates and may change
between their versions, so a later build could make another key from
the same seed. The vault keeps the key itself (§3).

### 2.4 The certificate

Made once with the key, kept beside it, never made again.

- X.509 v3, self-signed, `sha256WithRSAEncryption`.
- Subject and issuer: one common name, typed when the key is made,
  empty at first. Nothing else: no organisation, no
  place, no email. `apksigner verify --print-certs` prints it to anyone
  who has the APK. The name is trimmed; empty is refused ("Name the
  certificate"), and it is at most 64 characters (X.520's bound for a
  common name), the field taking no more.
- Serial: 16 bytes from the stream, top bit cleared, as Secure Boot's.
- Valid from 00:00:00 UTC of the day it was made, not the second: the
  certificate is published in every APK for good, and the date is all
  it needs (the clock must be known: GPG and Secure Boot ask for the
  time first, `docs/FLOWS.md` decision 58). Valid until 9999-12-31
  23:59:59 UTC, RFC 5280 §4.1.2.5's value for a certificate with no
  expiry. Android does not check an APK certificate's dates.
- One extension, the subject key identifier. Not a CA: no basic
  constraints, no key usage, unlike Secure Boot's.

**The fingerprint** is the SHA-256 of the certificate's DER: what
`apksigner verify --print-certs` prints as "certificate SHA-256 digest",
what F-Droid's `AllowedAPKSigningKeys` takes (lower-case hex, no
separators). Faraday shows it in groups of four, lower case, as it shows an
image's SHA-256.

## 3. Where it lives

- **Record type 12, APK signing key** (`docs/VAULT.md` §7): field 1 the
  private key, PKCS#8 DER; field 2 the certificate, DER; field 3 the
  package the key signs for, as text. All three required. The name is
  the certificate's common name, read from field 2, as Secure Boot's
  are. A vault may hold several, for different packages or the same.
- **The package binds the key.** Sign an APK signs only an APK whose
  manifest `package` (§5) is field 3, compared as bytes. A key that
  signs one app's APKs and another's lets each app share the other's
  signature-level permissions and stand in for it wherever a check
  trusts the certificate; one key per app keeps them apart.
- **The second copy is the vault's.** A sealed vault written to two
  sticks kept apart holds the key twice. Nothing else copies it.
- **The way out without Faraday** is `tools/vault/open.py`
  (`docs/VAULT.md` §10), for when Faraday itself cannot be used. By
  default it prints every field of the slot, in hex, so it does not
  give `apksigner` a key by itself. Batch 1 adds `--field 12.1 --out
  FILE`: it writes that one field's bytes, the PKCS#8 DER that
  `apksigner --key` takes, to a new file created with mode 0600, and
  prints nothing of the slot; `--field 12.2` writes the certificate for
  `--cert`. The steps, on the Learn page (§10): a computer that is
  offline and stays offline until the file is gone; the passphrase
  typed at the prompt, never as `--passphrase`; the file on that
  computer only, signed with, then shredded. It is the one place the
  private key becomes a file, and only by the person's own hand.
- **The private key never leaves the vault.** Faraday makes no file,
  no code, no paper form, no PKCS#12 or JKS keystore of it. What
  reaches For the stick is the certificate (§4.3) and signed APKs
  (§4.4). The one way out is `open.py`'s, above, without Faraday.
- **Signing waits for two written copies** (F31,
  `docs/BACKUP-FLOW.md`). The key is made by the shared backup flow,
  after its plan is set and its sticks read: where it will be kept (two
  places or more, the count never below two; by default one new vault
  at Place 1 and Place 2), each place's stick brought in with nothing
  secret open, the vaults unlocked or made, then the key made straight
  into them, one lock, and each stick written by place. It signs nothing
  until every place is written, each copy read back and matched; a
  second copy is refused on a stick holding the first and needs the hold
  "This is a second stick"; the FAT volume serial is not used, since
  sticks written from one image can share it. Until then the key's
  **Sign an APK**, and an APK's **Sign with APK key** in Files, read
  **Back up first** and open the flow on its step; the key's page has
  the row **Backup** ("Not written", "Written to {n} of {m} places",
  "Written to every place") with **Continue the backup**. The flow never
  opens the stick visit. On the Pi the sticks are SD cards. A key in a
  vault unlocked from a stick signs at once. GPG and Secure Boot keys
  wait by the same rule: none of the three can be made again from seed
  words. Built (batch B1a): the wait is the flow's, so nothing counts but its
  own writes; a write made on the stick visit does not. This follows the
  rule `docs/history/NEW-WALLET.md` §14.3 sets for a Bitcoin key made here, for
  the same reason, and asks for two copies because this key, once lost,
  cannot be replaced: a key whose only copy is in memory is gone at
  power-off, and one whose only copy is one stick is gone with it.
- **Deleting** is Vault contents' hold, as for every item. The Learn
  page (§10) says what losing the key means; the hold carries no extra
  line.
- **Older builds.** A slot holding a type 12 record does not open in a
  build that does not know the type (`docs/VAULT.md` §7: an unknown
  type refuses the slot), which is every release up to 0.1.2, nor in a
  copy of `open.py` from before batch 1.
- **Which vault is the person's choice.** Make asks for the vault with
  no default steering it (§4.2). Putting the key beside other keys has
  two consequences, which the Learn page states and no working screen
  does: everything else in that slot no longer opens in the builds and
  the `open.py` above; and one passphrase then guards all of it, so a
  release's OpenPGP key and its APK key in one slot fall together.

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
  `BOOTX64.EFI` carries **Sign in Secure Boot** (decision 1). It picks
  the open vault's key whose package is the APK's; with several such,
  "Which key?"; with none, the row reads "Another app: {package}"
  (`apk_other_package`) and is inert.

### 4.2 Make an APK key

Step 4 of the backup flow (`docs/BACKUP-FLOW.md` §4), after its
places, sticks and vaults (§3): step cards (wide) or pages (small):
**Name on the certificate** (one field, empty at first, the hint
"Public: printed by anyone who checks the APK"), then
**Package** (one field, empty at first). The
vaults are the plan's, none chosen in advance (§3).
The time first where it is not known. The package is refused, "Not a
package name" (`apk_package_invalid`), unless it is Android's form: at
least two parts joined by dots, each a letter followed by letters,
digits or underscores, ASCII only, at most 255 bytes. **Make the key**
→ Result, "APK key made": Name, **Package**, **Certificate SHA-256** (a
reference row opening Compare), Key "RSA 3072", and the
certificate For the stick as it is made (as GPG's public key is), its
row carrying **Show as QR** and **PNG**, and a row per vault it was
made into. The flow goes on to its lock and the writes by place.

### 4.3 The key's page

Vault contents' item page, as GPG's and Secure Boot's are (Faraday has no
Compare screen, `docs/DESIGN.md` §4.5): **Name**, **Package**,
**Certificate SHA-256** (the whole value, in groups of four, in the mono
face), **Key** "RSA 3072", **Made** (the date), then
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
the Pi after boot (`docs/history/PLAN.md` §3). A code cannot carry it: Faraday's file
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
| Not an APK | not a ZIP or a manifest Faraday reads with one reading only (§5), or no `AndroidManifest.xml` |
| Already signed | an APK Signing Block before the central directory, or a v1 signature (`META-INF/*.SF`, `.RSA`, `.EC`, `.DSA`) |
| Not aligned | a stored entry's data not at a multiple of 4 bytes, or a stored `.so` not at a multiple of 16 KiB (`zipalign -c -P 16 4`) |
| Supports Android before 7.0 | `minSdkVersion` below 24 (§2.2) |
| Another app: {package} (`apk_other_package`) | the manifest's `package` is not the key's (§3) |
| Over 4 GiB on FAT32 | the signed APK would pass FAT32's largest file |
| Name too long (`apk_name_too_long`) | the output name (below) would pass 230 bytes of UTF-8 |

None copied in: "Copy the APK in on a stick visit first." (the Pi: "on
an SD card visit").

**The review** is a Hold screen, "Sign this APK?":

- **File**: the name as copied in; **Size**.
- **SHA-256**: of the file as copied in, in groups of four. For a
  signing-only file it is `docs/LARGE-FILES.md` §5's streamed SHA-256,
  finished when the pass ended; the file itself is not held.
- **Second build**: a field where the person enters the SHA-256 that an
  independent second build of the same source printed (§9): hex, case
  and spaces ignored, at least its first 32 hex digits or all 64.
  Faraday compares what was entered with the start of the file's
  SHA-256. Empty, the row reads "Enter the second build's SHA-256";
  shorter than 32 digits, "At least 32 hex digits"; different, "Does
  not match this file" in `WARN`; equal, "Matches" with the digits
  compared.
- **Package**, **Version** ("0.1.3 (4)": `versionName` and
  `versionCode`), **Android** ("8.0 and later", from `minSdkVersion`).
- **Key**: its name and the fingerprint's first 16 hex digits.
- **Will write**: the output name.

The hold is inert until **Second build** reads "Matches"; then the
hold signs. The file copied in came from one build machine, which is
online: compromised, it builds a malicious APK and prints that APK's
hash, so the hash it prints proves nothing. A second build on another
computer, from the same tag, that gives the same SHA-256 does, and a
signature on a malicious APK can never be withdrawn. Entering the hash
is chosen over a box ticked "matches": a tick records only that it was
pressed, while an entered hash is a comparison Faraday makes itself.
32 hex digits are 128 bits, beyond any search for a second file whose
hash starts the same; 16 would not be. The value entered is not kept
after the hold. Reading the package and version from the binary
manifest is worth its parser (§5): it is how a person sees which app and
which version the key is about to vouch for, which the file name does
not prove, and `minSdkVersion` is needed anyway for §2.2's refusal.

**The output** goes For the stick: the name copied in with
`-unsigned` removed before `.apk`, so
`example-X.Y.Z-android-unsigned.apk` comes out as
`example-X.Y.Z-android.apk`; a name
without `-unsigned` gains `-signed` before a final `.apk`, or
`-signed.apk` where it has no `.apk` (both matched ignoring ASCII case,
the rest of the name kept as it is). The output name obeys
`docs/LARGE-FILES.md` §9's limit of 230 bytes of UTF-8, as every name
on a stick does: removing `-unsigned` only shortens a name, but adding
`-signed` (7 bytes) or `-signed.apk` (11) can pass it, and then the APK
is listed "Name too long" in `WARN`, inert, as in the table above, and
is never shortened or renamed by Faraday; the person renames it on
another computer. Then "Insert a stick to write it", as every result
For the stick. The unsigned file stays in Files.

**Memory.** On a whole import, **Sign with APK key** adds the signed
APK beside the unsigned file, and `docs/LARGE-FILES.md` §4.1 counts it:
where the signed output would leave too little to open the vault, the
review reads "Not enough memory to open {vault}" and its hold is inert,
as that rule says. A signing-only file keeps only the block and the
tail, which the rule counts as it was read.

## 5. What Faraday reads of an APK

Untrusted input (`docs/history/PLAN.md` §4.4), bounded and fuzzed
(`faraday/fuzz/fuzz_targets/apk.rs`). Faraday reads:

- **The ZIP's end.** The end of central directory record, searched
  backwards over at most its 22 bytes and a 65,535-byte comment; exactly
  one; no ZIP64, no multiple disks; the central directory directly
  before it and wholly inside the file.
- **The central directory**: each entry's name, flags, method, CRC-32,
  sizes and local header offset, and each entry's local header; the 16
  bytes before the central directory, for an APK Signing Block's magic.
- **`AndroidManifest.xml`**, stored or deflated (`miniz_oxide`, already
  in the graph, `docs/deps/miniz_oxide.md`), inflated up to 1 MiB.
- **The binary manifest**: its string pool, its resource map, the
  `manifest` element's `package`, `versionCode`, `versionCodeMajor` and
  `versionName`, and `uses-sdk`'s `minSdkVersion`. Nothing else: not
  resources, not other entries' contents, not `classes.dex`, not the
  native libraries.

**One reading only.** What Faraday shows must be what Android installs,
so a ZIP or a manifest that two readers could take two ways is refused.
These are Faraday's own rules, stricter than Android's; `aapt2` and
`zipalign` output passes them, and §8's byte-identical test against
`apksigner` is the judge of what is signed.

- **The entries lie end to end**: the first local header at offset 0;
  each next local header exactly where the previous entry's data ends
  (its local header, name, extra field and compressed size); the last
  ending exactly at the central directory, or at an APK Signing Block
  before it ("Already signed"). Nothing before, between or after: no
  prefix (a DEX, ZIP or other file in front), no gap, no bytes the
  central directory does not account for.
- **The central directory lists the entries in the order of their
  offsets**, each local header once.
- **Each local header agrees with its central entry**: name,
  general-purpose flags, method, CRC-32, compressed and uncompressed
  sizes, byte for byte.
- **Plain entries only**: method 0 (stored, its two sizes equal) or 8
  (deflated); no encryption (flag bit 0), no data descriptor (bit 3),
  no ZIP64 extra field.
- **Each name once**, compared as bytes; `AndroidManifest.xml` exactly
  once, at the root.
- **The manifest inflates to exactly its uncompressed size**, using
  exactly its compressed bytes, with the CRC-32 its entries state (the
  CRC-32 written in the crate, as `faraday-qr`'s `ur.rs` writes one).
- **Manifest attributes are known by resource ID**, as Android knows
  them: the ID the resource map gives an attribute's name, never the
  name's text. `versionCode` is `0x0101021b`, `versionCodeMajor`
  `0x01010576`, `versionName` `0x0101021c`, `minSdkVersion`
  `0x0101020c`; an attribute named like one of them with another ID or
  none is ignored, as Android ignores it. `package` has no ID: it is
  the attribute named `package` with no namespace. The root element is
  `manifest`; `uses-sdk` is its direct child, at most once. The same
  attribute twice on an element is refused.
- **Values Faraday reads itself.** `package`: a literal string, not
  empty. `versionCode` and `versionCodeMajor`: integers,
  absent read as 0, as Android reads them; `versionCodeMajor` other than
  0 shows as "0.1.3 (1:4)". `versionName`: a literal string, or absent,
  when **Version** reads "No name (4)".
  `minSdkVersion`: an integer, or absent, which Android reads as 1 and
  so is "Supports Android before 7.0". A reference to a resource, or a
  `minSdkVersion` that is a codename (a preview platform's), is refused:
  Faraday would have to guess what Android reads.

Anything that does not read, or breaks a rule above, is "Not an APK";
nothing panics or allocates past the file's size and the manifest's
bound.

The reader takes the file in parts, not as one buffer: the end, the
central directory (over 16 MiB is "Not an APK"), the manifest entry,
and each local header as the entries pass. So it reads a whole file in
memory and a signing-only file streamed from a stick
(`docs/LARGE-FILES.md` §5, §9) the same way.

**A signing-only file is read in one streamed pass.** The tail is read
first (the end record and the central directory, to know the file), and
then the whole file once, front to back. Everything kept for it (the
chunk digests, the SHA-256, the central directory and end record as
bytes, the manifest's fields, the refusal if one applies) comes from
that one pass, never from the first read of the tail. When the pass
reaches the central directory and the end record, they must stream byte
for byte as the first read found them; if any byte differs, nothing is
kept and the file is refused as `docs/LARGE-FILES.md` §7 refuses a part
that differs ("{source name} changed since it was read"). The fuzz
target feeds, besides malformed files, a central directory and an end
record that stream differently from their first read.

## 6. What Faraday writes

From the unsigned bytes `U`, whose central directory starts at offset
`C`:

1. The content digest over three sections: the entries and the zero
   bytes after them (`U[0..C]` and `P` zeros, where `P` is the distance
   from `C` up to the next multiple of 4096, 0 when `C` is one), the
   central directory, and the end record with its central directory
   offset read as `C + P`, the offset at which the signing block starts.
   Each section is cut into 1 MiB chunks (the last shorter); each
   chunk's digest is SHA-256 of `0xa5`, its length as a `u32`
   little-endian, and its bytes; the top digest is SHA-256 of `0x5a`,
   the chunk count as a `u32`, and every chunk digest in order. One
   digest serves v2 and v3.
2. The v2 and v3 signer blocks (§2.2), each signed data signed with
   the key (blinded with the signing's own bytes, §2.3), each carrying
   the certificate and the public key (SubjectPublicKeyInfo DER).
   **Each signature is checked before anything is written**, with
   `faraday-sb`'s `x509::verify` against the public key read from the
   certificate in the record, not from the private key. One that does
   not check is refused, "The signature did not check. Nothing
   written.", and nothing goes For the stick. A signature made with a
   fault in the private-key operation (a flipped bit, an undervolted
   Pi) gives the private key to anyone who has it and the certificate,
   and signed APKs are published. The `rsa` crate checks its own result
   today (`rsa_decrypt_and_check`); this check does not depend on that,
   or on the pinned release candidate keeping it.
3. The APK Signing Block: its size, the ID-value pairs, the padding
   pair, its size again, and `APK Sig Block 42`.
4. The output: `U[0..C]`, `P` zero bytes, the block, the central
   directory unchanged, and the end record with its central directory
   offset raised by `P` and the block's length. Nothing else changes: no
   entry is re-aligned, re-compressed or renamed.

For a signing-only APK, steps 1 to 3 come from the chunk digests and
bytes kept when it was read, and step 4 is the copy of the next visit
(`docs/LARGE-FILES.md` §7).

So removing the zero bytes and the block and restoring the offset gives
back `U` byte for byte, given `C` (the zeros cannot be told from the
last entry's own zeros without it, and the person holding `U` has `C`):
what F-Droid's `apksigcopier` relies on to copy the signature onto
F-Droid's own build of the same tag. The
`apksigner` of the build-tools pinned in `tools/vectors/apk/make.sh`
(36.0.0) writes the same `P` and the same block; the spec of v0.3 left
`P` out, and the byte-identical test of §8 found it.

## 7. Crates

- **`faraday/faraday-apk`** (new), `no_std` + `alloc`: the reader of §5,
  the signer of §6, the key and certificate of §2. It takes a file in
  parts and keeps the chunk digests, never needing the whole file.
  Depends on `faraday-sb` (`der`, `rng`, `x509`, already public),
  `sha2`, `zeroize` and `miniz_oxide`; no new crate in the graph. It writes nothing to a
  vault and draws no randomness of its own: its signing takes the
  caller's `rng::Stream` (§2.3).
- **`faraday-sb`**: `x509::generate` takes the key's size and a profile
  (Secure Boot's CA certificate, or the APK's end certificate with no
  expiry). Secure Boot's output is unchanged, which its judged tests
  show. Moving `der`, `rng` and `x509` into a crate of their own waits
  for a third user.
- **`faraday-core/src/apk.rs`**, beside `gpg.rs` and `secureboot.rs`:
  the record, the screens' state, Make, Sign, the wait for a written
  copy.
- **`faraday-vault`**: `kind::APK_KEY = 12`. `tools/vault/open.py` reads
  type 12 and gains `--field TYPE.FIELD --out FILE` (§3): the one field
  of the record of that type in the slot (with several, `--index N`
  picks the N-th in the order they are printed, and without it the
  command refuses, naming how many), written to a new file created with mode 0600 (an existing
  file is refused), nothing printed of the slot. The vectors in
  `tools/vectors/vault/` gain a type 12 record, so each slot still holds
  one record of every type (`docs/VAULT.md` §10), and the vault tests
  check that `--field 12.1` writes field 1's bytes and nothing else.
- **`docs/deps/rsa.md`** names 3072 beside 2048; **`miniz_oxide.md`**
  names the new edge.

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
- `old-sdk-unsigned.apk` (`minSdkVersion` 21),
  `sdk33-unsigned.apk` (the small one with `minSdkVersion` 33, for the
  v3 signer's SDK range, §2.2) and `unaligned-unsigned.apk` (the small
  one with its `.so` moved off its boundary).
- One fixture per rule of §5's "One reading only", each the small one
  with bytes changed by the script: a prefix before the first entry; a
  gap between two entries; the central directory out of offset order;
  a local header whose method, sizes or flags differ from its central
  entry; a data descriptor; a second entry named `AndroidManifest.xml`;
  a manifest whose `versionCode` name string carries another resource
  ID beside the real attribute; a `versionName` that is a resource
  reference; a codename `minSdkVersion`.
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
  APK makes a file byte for byte equal to Faraday's, for the small,
  the `sdk33` and the large fixture. Without `--alignment-preserved`, `apksigner` may
  rewrite the entries' alignment padding, which §6 never does.
- Removing the block from Faraday's output and restoring the offset
  gives back the unsigned file.

**In `faraday-core/tests/all/apk.rs`**, through `Action`s as `gpg.rs` does:

- Make an APK key puts a type 12 record in the open vault, and the
  certificate For the stick; the vault sealed and opened again shows the
  same name and fingerprint.
- A key made in the flow offers **Back up first** in place of Sign
  an APK until every place of its plan is written, and signs after; its
  plan cannot go below two places; a key unlocked
  from a stick signs at once. The flow's own tests are
  `docs/BACKUP-FLOW.md` §11.
- Until F31's batch: a failed write does not count; a second stick
  holding the first write's bytes is refused, and one holding an older
  copy of the vault is replaced and lets the key sign.
- Each refusal of §4.4 on its fixture, each of §5's fixtures "Not an
  APK" (and a PSBT, a PNG, random bytes, `apksigner`'s own signed output
  for "Already signed"); the review's package, version and Android line
  for the small fixture; the output's name.
- The review's hold is inert with **Second build** empty, with fewer
  than 32 hex digits, and with digits of another file; it signs with the
  file's first 32 digits and with all 64, spaced or in capitals.
- A key made for `app.faradayoffline.fixture` signs the small
  fixture; a key made for `app.faradayoffline.other` lists the same
  fixture as "Another app: app.faradayoffline.fixture", inert, from the
  key's page and from Files; the key's page and the vault sealed and
  opened again show the package. A package typed as `other`,
  `1app.x` or with a space is refused, "Not a package name".
- An APK named with 224 bytes before `.apk` and no `-unsigned` is
  listed "Name too long"; one of 230 bytes ending `-unsigned.apk` signs
  as the shorter name.
- A certificate name left empty is refused; the certificate's validity
  starts at midnight UTC of the day it was made.
- Nothing For the stick ever holds the private key: after every
  action, no Outbox file contains the PKCS#8 bytes, nor the big-endian
  bytes of its private exponent or of either prime.

A signing-only APK, and the 200 MiB fixture, are tested in
`docs/LARGE-FILES.md` §11.

## 9. The release

**Historical (F34): the release no longer carries an APK, and
`stage-release.sh` has no `--companion`.** This is what the Companion's
release was, with the key in it:

1. The pinned container builds
   `faraday-companion-X.Y.Z-android-unsigned.apk`, checks it with
   `zipalign -c -P 16 4`, and prints its size and SHA-256.
2. **The second build**: the same container, from the same tag, on
   another computer that shares no account, key or network share with
   the build machine (COMPANION §10's "second build that matches"). It
   prints its SHA-256, which the person reads there, never through the
   build machine. The two must be equal; if they are not, nothing is
   signed.
3. On a Faraday stick (or Pi), in one session: a stick visit brings the
   unsigned APK in, from the build machine's output; **Sign an APK**,
   with the second build's SHA-256 entered in the review (§4.4); the
   signed APK goes out.
4. `faraday/tools/stage-release.sh` gains `--companion FILE`: it runs
   `apksigner verify --print-certs`, refuses an APK whose certificate is
   not faraday/companion-cert.pem's, and adds the APK and its line to
   `SHA256SUMS`.
5. A second visit in the same session signs `SHA256SUMS` with the
   release key, as before.

faraday/companion-cert.pem is the certificate, committed once the key
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
to the sticks of two or more places is its backup; that the certificate and its
fingerprint are public, and how anyone checks an APK against it
(`apksigner verify --print-certs`, F-Droid's page); why a key made here
waits for every place of its plan to be written before it signs; why
signing asks for the SHA-256 of a second build made on another
computer, and that a signature on a bad APK cannot be withdrawn. On the
choice of vault, both consequences plainly: a slot holding an APK key
does not open in Faraday up to 0.1.2, nor in an `open.py` from before
the APK key, so nothing else in that slot opens there either; and one
passphrase then guards everything in the slot, the APK key and, where
it is there, a release's OpenPGP key. The way out without Faraday, step
by step (§3): an offline computer, `open.py --field 12.1 --out` with
the passphrase typed at its prompt, `apksigner sign --key` with that
file and `--cert` with field 2's, then the file shredded before the
computer goes online again. Working screens carry none of this
(`CLAUDE.md`).

## 11. Batches

- **1, the key and the signer.** `faraday-apk` (§5, §6, §2), the
  `faraday-sb` change, record type 12 with `open.py` (reading type 12,
  and `--field … --out`, §7) and the vectors, the stick's FAT volume
  serial on a vault's receipt (`docs/VAULT.md` §6, for §3's two sticks),
  `faraday-core/src/apk.rs`, the screens of §4, the Learn page, §8's
  tests and the fuzz target, the docs of §7. Needs no release build:
  the fixtures stand in. Renders: the tile, Make, the key's page and the
  review at `1280x800` and `480x640`.
- **2, with the Companion's batch C (dropped, F34: removed from the
  tree, the staging script and the README).** The container's step 1, the
  second build's step 2 written as a release step, the staging script's
  step 4, the README paragraph, the fastlane metadata and
  `AllowedAPKSigningKeys`.
  **Built (2026-10-14) as far as the key allows:** the container's
  step 1, the staging script's step 4 (`--companion`), the fastlane
  metadata. Left until the certificate exists: the README paragraph and
  `AllowedAPKSigningKeys`, both TODOs named in
  faraday/shells/android/fastlane/README.md. Step 2, the second build,
  is `just companion-release-apk` run from the tag on another computer.
- **The owner**, between them: the key made on the stick of a published
  release that has batch 1, in the vault the owner chooses (§3, §12),
  that vault written to two sticks, and the certificate committed as
  faraday/companion-cert.pem (§12; dropped with batch 2).

Batch 1 signs APKs up to today's 18 MiB read; any size comes with
`docs/LARGE-FILES.md`'s batches L1 and L2, after batch 1. Batch 2 does
not wait for them while the release APK stays under 18 MiB.

## 12. Decided by the owner (2026-10-14)

1. RSA-3072 (§2.1).
2. Faraday Desktop offers the APK key, as it offers GPG and Secure
   Boot; the release's key is made on a stick regardless.
3. Signing waits for a written vault (§3); after the review, for
   receipts from two sticks with different FAT serials. Replaced by F31
   (2026-10-14): the shared backup flow's two writes, the second refused
   on the stick holding the first copy and confirmed by a hold.
4. The real key is made on the stick of a published release that has
   batch 1, so the build that made it is one anyone can reproduce.
5. Which vault holds it is the person's choice, the owner's for the
   real key. Make steers to none; the Learn page states what sharing a
   slot means (§3, §10). The owner's words: the premise of a secret is
   the ability to keep it, and the person keeps this flexibility.
6. The certificate's name was "Faraday Companion" (superseded, F34:
   the field starts empty).
7. Signing needs the SHA-256 of an independent second build of the
   same source entered in the review and matching (§4.4, §9). Entering
   it, rather than ticking it as matched, is this spec's
   recommendation.
8. Each key is bound to one package (record field 3), typed at Make
   (empty at first; it was `app.faradayoffline.companion` before F34)
   and shown on the key's page; Sign an APK refuses another package's APK. Another app gets
   another key.

## 13. What batch 1's build changed or chose

Items 4 to 6 below describe batch 1's placeholder for the backup; batch B1a
replaced it with the shared flow (`docs/BACKUP-FLOW.md`; `docs/history/BACKUP-FLOW-LOG.md` has its build), which keeps the
certificate and the signing as items 1 to 3 and 7 to 9 say.

Where this document was wrong or silent, the build followed the closest
existing pattern (`gpg.rs`, `secureboot.rs`) or `apksigner`:

1. **Zero bytes before the block and the end record's offset** (§6):
   `apksigner` pads the entries to a multiple of 4096 before the block,
   and its digest counts the end record with the offset of the block's
   start. Found by the byte-identical test.
2. **The v3 SDK range is 24 to 2147483647** whatever the APK's
   `minSdkVersion` (§2.2), measured.
3. **Files and the signed file.** (A manifest attribute named like
   one of the four it reads with another resource ID, the ID with another
   name, a `package` with an ID, a namespaced element at the first two
   levels, more than 256 attributes, or an unsigned or absurd
   `minSdkVersion` is "Not an APK".) The output is `x.apk` only for
   `x-unsigned.apk`; any other name, `x-unsigned` included, gains
   `-signed`, so `x-unsigned` becomes `x-unsigned-signed.apk`. A file is an APK when its central
   directory names `AndroidManifest.xml`; a PSBT, a picture or random
   bytes is not listed at all (§4.4's "Not an APK" is for a ZIP that
   names a manifest and breaks a rule of §5). A signed APK is "Already
   signed", with up to 4095 zero bytes between the last entry and the
   block allowed, as `apksigner` writes them. A `zipalign`-padded extra
   field may end in zero bytes that do not frame (`zipalign` pads so);
   the reader allows them and refuses a ZIP64 field. A stored directory
   entry (a name ending in `/`) is not checked for alignment.
4. **The wait is a log, not receipts** (§3): the last receipt is
   replaced by the next write, so a key made this session keeps the
   sealings that hold it and its first write (name, length, hash), kept
   across a lock. `StickInfo`, the disk protocol and the receipt carry
   the FAT volume serial (`faraday-fat`, `faraday-files`,
   `faraday-disk`), for display; nothing gates on it.
5. **Back up first locks the vault** (§3), since a stick is handed only
   to a clean process, and the key's page comes back at the next unlock.
   The two writes are placeholders for the shared backup flow.
6. **Make is the vault's form** with two fields and the hint, then the
   vault buttons where several are open; the result is a panel on the key's
   page (§4.2), not step cards. The certificate's SHA-256 is a mono row,
   not a reference row to Compare (no such screen).
7. **`open.py`** reads types 11 (a backup plan, missing from its table)
   and 12; `--index N` counts from 1.
8. **Not built** (`docs/LARGE-FILES.md`): files over today's 18 MiB read,
   signing-only files, the single streamed pass of §5's last paragraph,
   and the review's "Not enough memory to open {vault}". `faraday-apk`
   reads through a `Source` in parts, so they need no change to it.
9. **No info button on the Tools tile**: the page is named by Learn's `?`
   on the vault contents while an APK key is shown; the Tools page has
   no per-tile info.
