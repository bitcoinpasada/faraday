# What is cryptography, and where it lives

**Kind:** Reference
**Status:** Built (the key boundary, F70)

`core/` is Faraday's code. One row per crate
that touches a key, a seed, a signature, a cipher or a passphrase,
saying what it computes, whether that is written in this tree or taken
from a crate, and what it is checked against. For someone deciding what
to read to audit this device, or how far a change reaches.

Code that defines bytes, or answers a question with no screen in it, is a
`core/` crate (`docs/WALLETS.md` §1 rule 5). Key use is one crate,
`faraday/faraday-keys`: it holds every key in memory and is the only
code that calls the primitive crates' secret-taking functions. The app,
`faraday-core`, holds handles, public results and the input a person is
typing; §2 says what that means and how `just` holds it. This file says
which crates compute cryptography, which hold keys, which only carry or
display bytes other crates computed, and which never see a key at all.

Three tiers, used through the rest of this file:

- **Computes** — makes, derives, signs, encrypts or decrypts with a key,
  a seed or a passphrase, or turns user entropy into one. The primitive
  crates compute and keep nothing; `faraday-keys` keeps the keys and
  calls them.
- **Hashes** — calls `faraday-crypto`'s SHA-256 for an integrity check (a
  file's checksum, a QR transfer's envelope) and never holds a private
  key or a passphrase.
- **Carries** — holds or renders bytes that a *Computes* crate already
  produced (an address, a fingerprint, a QR code, a descriptor), and
  computes nothing itself.

## 1. Crates that compute cryptography

### `core/`

| Crate | What it computes | Written here, or from a crate | Checked against |
|---|---|---|---|
| `faraday-crypto` | The hash/KDF choke point every other crate calls through (`sha256`, `hmac_sha256`/`512`, `pbkdf2_hmac_sha256`/`512`, `blake2b`, `scrypt`); `Secret`, `Pinned` (mlock'd pages) and `Sealed`/`SessionKey` wrappers that hold or encrypt a secret in memory. No signing, no curve operations, no random number generator: entropy is handed in by the shell. | SHA-256, HMAC, PBKDF2: RustCrypto crates; `Sealed` encrypts with ChaCha20-Poly1305 (`chacha20poly1305`). **BLAKE2b and scrypt are written here** (one caller each, LND's `aezeed`) rather than taken as crates. | `core/faraday-crypto/tests/primitives.rs`; BLAKE2b against RFC 7693's own vectors, scrypt against RFC 7914's, inline in `blake2b.rs`/`scrypt.rs`. |
| `faraday-selftest` | The start-up self-test, run before any key is accepted: BIP-39 words and seed (PBKDF2-HMAC-SHA512), BIP-32 derivation, a BIP-84 address, ECDSA (RFC 6979) and BIP-340 signing, scrypt and Argon2id, each through the function the product itself calls, with fixed vector keys only. A failure names the check and blocks the app. | Nothing computed here: it calls `faraday-bip`, `faraday-psbt`, `faraday-crypto` and `faraday-backup`. | Its expected values are copied from the vector files the integration tests use (`tools/vectors/bip39/`, `tools/vectors/psbt/rfc6979.txt`, `tools/vectors/kdf/`) and from libsecp256k1's copy of the BIP-340 vectors. |
| `faraday-bip` | BIP-32 keys (wraps `bitcoin::bip32`, so private material never gains `Debug`/`Clone`/a text form), BIP-39 mnemonics, BIP-44/49/84/86 accounts, SLIP-132, BIP-380 descriptor checksums, BIP-85 child mnemonics, SLIP-39 Shamir backup, BIP-327 MuSig2, BIP 445 FROST (with the trusted-dealer split), BIP 93 codex32 (its worksheets, fresh sets and error correction), aezeed. | Keys/addresses: `bitcoin`'s own `bip32` (external, vetted). **BIP-39, SLIP-39, MuSig2, FROST and codex32 are written here** — no vetted `no_std` crate exists for them on this tree's pin, or (BIP-39) the explorer screens need intermediate values a library hides. | `tools/vectors/{bip32,bip39,bip85,bip93,bip129,bip327,bip328,bip352,bip387,bip390,bip392,bip445}/`; BIP-39 and seed derivation additionally cross-checked against the `bip39` crate as a dev-dependency (`faraday-bip::crosscheck`). |
| `faraday-psbt` | PSBT parse/serialise, inspection (amounts, fee, verified change, warnings) from non-secret data only, signing (key derived from the PSBT's own origin data, never a guessed path; deterministic nonces; verifies its own signature before writing it; signs twice to check determinism), BIP-137/BIP-322 message signing, finalisation, and verification of signatures already on a transaction. | Signing itself: `faraday-bip`'s keys, over `bitcoin`'s secp256k1. The PSBT format, inspection and verification logic: written here. | `tools/vectors/psbt/`, `tools/vectors/message/`; wallet fixtures built and checked in `core/faraday-psbt/examples/wallet.rs` / `tests/all/wallet_fixtures.rs`. |
| `faraday-entropy` | Turns user-supplied dice, coin flips, hex, playing cards or camera noise into `RawEntropy` of the right size for a BIP-39 key, with the mistakes real hands make checked for and warned on; mixes two or more sources by hashing: SHA-256 over their 32-byte SHA-256 commitments, concatenated in the order taken, truncated to the strength (`mix.rs`). Seed XOR (`xor.rs`) is a separate paper form. | Written here (bit/face accumulation and packing; no external entropy crate fits a fixed-size, erase-on-drop buffer). | Its own unit tests (no published vectors exist for "pack these dice as entropy" — it is deterministic bit-packing, not a named standard); exercised end to end through `faraday-core/tests/all/keygen.rs`. |
| `faraday-backup` | The KDBX 4 writer, and the Argon2id helper and `Cost` type the vault, the kept-key blob and the self-test use. Argon2id at a caller-chosen, file-carried `Cost`; it draws no randomness of its own. | Argon2id, the KDBX cipher stack: RustCrypto crates (`argon2`, `chacha20`, `sha2`, `hmac`). The KDBX file itself: written here. | `tools/vectors/backup/` (the KDBX file), read back by `tools/backup/read-kdbx.py`. |

`faraday-bip` also uses private keys: BIP-85 derives from one, MuSig2 and
FROST sign with one, a silent payments wallet holds its scan key, and
the vanity grinder derives keys while it searches.

### `faraday/` (Faraday's own)

| Crate | What it computes | Written here, or from a crate | Checked against |
|---|---|---|---|
| `faraday-keys` | **Holds and uses keys.** The session's keys (words, passphrase, master key), the open vaults' slot keys and decrypted records, and every operation on them: PSBT, message and BSMS signing, BIP-85, Lightning node keys, silent payments, FROST dealing and shares, vanity grinding, key texts and fingerprints, paper forms (Seed XOR, codex32, SLIP-39), the GPG and Secure Boot keys and what is signed with them, TOTP codes, unlocking and sealing a vault and writing KDBX. Seed words become a `Mnemonic` only in its `words` module. A secret leaves only through one of eleven named reasons (§2). | No primitive is written here; every operation calls a crate in this section or `core/`. Which key is derived, when, and for what is written here. | `faraday/faraday-keys/tests/`: BIP-32/39, BIP-85, BIP-137, BIP-352 and RFC 6238 vectors, a vault and a deal end to end; `faraday/faraday-core/tests/` for each flow end to end (among others `bip85.rs`, `lightning.rs`, `gpg.rs`); the primitives' own vectors. |
| `faraday-vault` | The Faraday vault: one file, four equal slots, each its own passphrase, a slot nobody opens filled with random-looking bytes so the file does not say how many are in use. Argon2id stretches the passphrase; **HKDF-SHA256 (hand-assembled from `hmac`+`sha2` — one extract and the first expand block, not a dedicated `hkdf` crate) takes the stretched key to each slot's key**; XChaCha20-Poly1305 is the slot cipher; a ChaCha20 keystream fills a new vault's salt, nonces and unused slots from the caller's seed. | Argon2id, XChaCha20-Poly1305, ChaCha20, SHA-256/HMAC: RustCrypto crates. TOTP codes (RFC 6238, `totp.rs`) are HMAC over SHA-1, SHA-256 or SHA-512 from `hmac`, `sha1` and `sha2`. The four-equal-slot container, the HKDF construction over those primitives and the TOTP code derivation: written here. | `faraday/faraday-vault/tests/vectors.rs` against `tools/vectors/vault/`; `tools/vault/open.py` opens the same vectors independently, with either `argon2-cffi`+`pynacl` or `cryptography` on OpenSSL. |
| `faraday-pgp` | Version-4 OpenPGP packets for one key shape (Ed25519 certification + Ed25519 signing subkey): self-signatures, the subkey's back-signature, detached signatures over SHA-256/512, a revocation certificate and its `.rev` file, a `paperkey`-format export, and the reader of public armour that decides what Faraday calls a public file (every packet read to its exact end). | Ed25519: `ed25519-dalek`. SHA-1 (fingerprints only, not signatures), SHA-256/512: `sha1`/`sha2`. The packet format: written here. | `faraday/faraday-pgp/tests/gnupg.rs`: every packet judged by `gpg` 2.4 (imports, verifies including signatures whose r or s starts with a zero byte, rejects other data, takes the revocation, sees a key expire and be renewed, refuses a `.rev` file until its colon is removed) and by `paperkey` 1.6 (identical lines, seeds with leading zero bytes); `tests/read.rs` and the `pgp_armor` fuzz target for the reader. |
| `faraday-sb` | An owner's PK, KEK and db as RSA-2048 keys with self-signed X.509 certificates, EFI signature lists, signed `.auth` updates, Authenticode signatures over `BOOTX64.EFI`, and checking one against a db certificate. | RSA-2048 (**pinned to `0.10.0-rc.18`, a pre-1.0 release candidate — noted in `docs/deps/rsa.md`**): the `rsa` crate. The RNG `rsa` draws from is a **ChaCha20 keystream written here** (`rng.rs`) over the caller's own seed, implementing `rand_core`'s trait. From `rsa`: key generation, PKCS #1 v1.5 signing and verifying with SHA-256, and PKCS #8 and SubjectPublicKeyInfo encoding of the keys; `sha2` for the digests. Written here, with no DER, X.509 or CMS crate: the DER writer and the little reader (`der.rs`), the self-signed X.509 certificate and its reader (`x509.rs`), the CMS SignedData and signer info, the EFI signature lists and `.auth` files (`efi.rs`), and the Authenticode image hash, signature and check (`pe.rs`). | `faraday/faraday-sb/tests/judged.rs`: every certificate and `.auth`'s PKCS #7 judged by `openssl`; signed images checked by `sbverify`. |

## 2. The key boundary

`docs/KEYS.md` is the spec and its "as built" sections the record. What an
auditor can state from it:

- **Every key in memory is in `faraday-keys`.** The session's keys and
  the open vaults' records sit in private fields of `Keys` and
  `OpenVaults`. faraday-core addresses a key by its place and a record by
  its vault and index, and reads public values: a key's `KeyInfo`
  (fingerprint, label, word count, share), a record's public fields
  (`vault::SECRET_FIELDS` lists the secret ones).
- **A secret leaves through eleven named reasons** (`session::Reveal`):
  the words, the passphrase, a SeedQR, copy by hand, a paper form, a
  BIP-85 child, a Lightning node key, a GPG paperkey, a vault entry (into
  a form, onto the screen or into a record), an unprotected file to the
  Outbox, and the backup flow writing to a stick. The last three name the
  part they carry (`Part`: words, passphrase, SeedQR picture, password,
  TOTP secret, note, entry notes, revocation certificate, scan key, node
  key). A secret that leaves is lent (`Revealed`) or held (`Held`): not
  `Clone`, not `Debug`, wiped on drop.
- **Types that hold a secret are opaque outside the crate:** the silent
  payments `Receiver`, a Lightning `Node`, a FROST `Dealt`, a spend's
  carry `Section`, a key made from fresh entropy (`MadeKey`), and
  faraday-psbt's `MusigSession`. Each gives public values, and its secret
  leaves only through a method that takes a reason.
- **What faraday-core may hold** is the input a person is giving, until
  it is handed in (KEYS §2): typed words and passphrases, a scanned
  SeedQR, a vault passphrase while it is typed, dice and other entropy,
  an Inbox file that is a secret. It holds them in `Zeroizing` buffers
  and hands them to `Keys`; it can derive nothing from them.
- **`just` holds it.** `tools/lint-secrets.sh` rule 4 refuses, under
  `faraday/` outside `faraday-keys`, a key or seed in hand (`MasterKey`,
  `SecretKey`, `Xpriv`, `to_seed` and the rest), seed words made into a
  `Mnemonic`, the primitive crates' secret-taking functions, and a held
  secret taken out of its guard outside one named site (a GPG paperkey
  drawn while held). Exempt: the primitive crates, tests, examples, fuzz
  targets, the snapshot shell and `faraday/tools/` (never shipped), and
  faraday-core's testkit. `tools/lint-keys-tests.sh` fails when a shipped
  binary turns on the testkit or `Keys::master_for_tests`. Rules 1 to 3
  of `lint-secrets` refuse printing and `Debug` of secret types
  everywhere in faraday-core and `core/`.

To audit key use, read `faraday/faraday-keys` and the primitive crates of
§1, then the `Reveal` list and every call that names a reason
(`grep -rn "Reveal::" faraday/faraday-core/src`).

## 3. In-house algorithms, named

The highest-leverage single fact for an auditor: everything above marked
"written here" rather than "from a crate". In one list:

- BIP-39 mnemonics (`faraday-bip::bip39`)
- SLIP-39 Shamir backup (`faraday-bip::slip39`)
- BIP-327 MuSig2 (`faraday-bip::musig`)
- BIP 445 FROST, including the trusted-dealer split (`faraday-bip::frost`)
- BIP 93 codex32 (`faraday-bip::codex32`)
- The PSBT format, inspection, signing orchestration and verification
  logic (`faraday-psbt`, over `faraday-bip`'s keys and `bitcoin`'s secp256k1)
- Dice/coin/card/hex/camera entropy accumulation and packing
  (`faraday-entropy`)
- BLAKE2b and scrypt, one caller each (`faraday-crypto`)
- The KDBX 4 file format (`faraday-backup`) and the vault's HKDF-SHA256
  construction (`faraday-vault`)
- The four-equal-slot vault container (`faraday-vault`)
- OpenPGP v4 packet construction (`faraday-pgp`)
- X.509/CMS structure, Authenticode signing, and the RSA RNG adapter
  (`faraday-sb`)

Every one of these is checked against either a published vector set, an
independent library or tool, or (BLAKE2b, scrypt) the RFC's own test
vectors — see the "Checked against" column above. None is cryptography
invented for this project: each implements a named, published algorithm
or file format.

## 4. Crates that touch cryptographic bytes but compute none

| Crate | What it does with the bytes |
|---|---|
| `faraday-core` | The app: flows, state, screens and wording. Holds handles to keys and records, public results (signatures, xpubs, addresses, fingerprints, signed files), and the input a person is typing until it is handed to `faraday-keys` (§2). Hashes for integrity and for ids, as the *Hashes* tier does. |
| `faraday-codec` | QR/UR encoding and decoding, classification, PNG. Calls `faraday-crypto::sha256` only for SeedQR's own integrity check; never holds a private key. |
| `faraday-qr` | BBQr, numbered text parts, the Faraday file envelope, the UR registry's key/wallet types. Calls `sha256` to check an envelope's declared hash against its data; computes no key material. |
| `faraday-draw` | Layout engine and renderer. Takes `faraday-bip`/`faraday-codec` types (a fingerprint, an address, a QR code) to draw; never computes one. |
| `faraday-shell-api` | The event/command contract between an app and its shell. No dependency on any crypto crate. |
| `faraday-payload` | The Starter's payload: reads the `MANIFEST` of the release's `.xz` files, checks a payload file's SHA-256 against it before a byte is written, decompresses it as a stream (`lzma-rust2`'s xz decoder, which checks xz's own CRCs), and succeeds only if what was written has the manifest's size and SHA-256. The *Hashes* tier: no key, no signature. |
| `faraday-intake` | Reads every scan, paste and file first: whether it is a seed (SeedQR, words, checked through `faraday-bip`) or another secret, and what a transfer becomes. Holds what it reads in `Zeroizing` buffers until it is handed on; computes no key. |

## 5. Crates with no cryptographic involvement

`faraday-fat` (FAT16/32), `faraday-disk` (the unprivileged disk process),
`faraday-grant` (the `CAP_CHOWN` helper), `faraday-files` (the Place
trait, the pipe protocol), `faraday-storage` (Inbox/Outbox, PDFs),
`faraday-scanner` (camera frames), `faraday-strings` (the words on
screen), `faraday-boot` (the boot copier: copies a boot partition raw,
identified by the running kernel's release string, which is not a
signature), `faraday-ui` (the desktop app's and the Starter's drawing),
`faraday-learn` and `faraday-guides` (Learn pages and the start-from-stick
guides), `faraday-starter` (the Starter's state and screens; it writes
images through `faraday-payload` and holds a `faraday-core` in online
mode for Transfer and Learn), every shell (`faraday-desktop`, `faraday-stick`,
`faraday-snapshot`, `faraday-window`, `faraday-mediafoundation`, the
Starter's, `faraday-v4l2`, `faraday-avfoundation`), and the dev
tools (`faraday-testkit`, `faraday-argon2-bench`, `faraday-fontbake`,
`costs-lint`). The snapshot shell and the testkit tool build the test
keys and wallets they render and write, and ship in nothing. The Starter
handles a BitLocker recovery password through Windows' own API and
computes nothing with it.
These matter to the trust model (`docs/history/PLAN.md` §4 — privilege separation,
untrusted input, the stick rule) but never see a key, a seed or a
passphrase; that is a property of the process boundaries, not of
anything in this file.

## 6. The dependency chain

```
external, vetted crates (RustCrypto: sha2, hmac, pbkdf2, argon2,
chacha20(poly1305); ed25519-dalek; rsa; bitcoin's secp256k1; rqrr)
        │
        ▼
   faraday-crypto            (the hash/KDF choke point; BLAKE2b, scrypt written here)
        │
   ┌────┴─────┐
   ▼          ▼
faraday-bip   faraday-entropy     (BIP-32/39/44.., in-house BIP-39/SLIP-39/MuSig2/FROST/codex32)
   │
   ├──────────┬───────────┬──────────┐
   ▼          ▼           ▼          ▼
faraday-psbt  faraday-backup  faraday-codec
   │          │           │
   │      faraday-vault   │
   │      faraday-pgp     │     (Faraday's own: HKDF, OpenPGP, X.509/Authenticode)
   │      faraday-sb      │
   │          │           │
   └──────────┴─────┬─────┘
                     ▼
              faraday-keys               (every key held and used; the reveals)
                     │
                     ▼
              faraday-core               (flows and wording; handles and public results)
                     │
                     ▼
            faraday-draw, every shell          (rendering, input, storage; zero cryptography)
```

A change below the first horizontal line can change what a key, a
signature or a ciphertext *is*. A change in `faraday-keys` can change
which key signs, and when and why a secret leaves. A change in
`faraday-core`, `faraday-draw` or a shell can change what the person sees
and when, and which handle and reason it passes to `faraday-keys`, never
the bytes underneath it.

## 7. The external dependency layer

Every crate named "from a crate" above is pinned in `Cargo.lock` and
carries its own note under `docs/deps/` (why it was chosen, what it
replaces or what it is pinned against): `argon2.md`, `chacha20.md`,
`chacha20poly1305.md`, `ed25519-dalek.md`, `hmac.md`, `pbkdf2.md`,
`rand_core.md`, `rsa.md`, `sha1.md`, `sha2.md`, `subtle.md`, `getrandom.md`,
`zeroize.md`, `rqrr.md`, `miniscript.md`, plus non-cryptographic
build/platform deps (`cc.md`, `libc.md`, `libc-faraday-grant.md`,
`windows-sys.md`, `rustbus.md`, `miniz_oxide.md`, `lzma-rust2.md`,
`clipboard.md`, `ur.md`). `bitcoin`, the most important dependency, has
no page yet. `deny.toml` (advisories, licences, duplicate and source
checks) runs in `just lint`. `supply-chain/` holds `cargo vet`'s files,
but `cargo vet` does not pass today (23 crates are covered by neither an
audit nor an exemption, and 223 are exempt) and runs in no recipe; F71
makes it true.

## 8. Reading a change

A diff under `core/faraday-crypto`, `faraday-bip`, `faraday-psbt`, `faraday-backup`,
`faraday-entropy`, or `faraday/faraday-vault`, `faraday-pgp`, `faraday-sb`
or `faraday-keys`, can change cryptographic output and
needs §1 and §3's scrutiny:
re-run the vectors named above, and for anything marked "written here",
read the diff against the standard it implements, not just against its
own tests.

A diff in `faraday-keys` that adds a `Reveal` variant or `Part`, a public
function returning a secret, or a public field on a type that holds one,
widens the boundary: read it against §2.

A diff under `faraday-codec`/`faraday-qr` that stays inside existing
functions needs only an integrity-check read (§4); a new function there
that starts touching a key would move it into §1 and this file would be
wrong until updated.

A diff under `faraday-core`, `faraday-draw`, a shell or anything in §5
changes a screen, a flow or a process boundary, never a cryptographic
result, unless it changes which handle, input or reason it passes to
`faraday-keys`: worth noticing, since that is the one way a "flows only"
change can matter here. A diff that changes `tools/lint-secrets.sh` rule
4's patterns or exemptions changes what `just` holds, and is read as a
change to the boundary.
