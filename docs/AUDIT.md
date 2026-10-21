# What is cryptography, and where it lives

**Status:** written 2026-10-06; corrected 2026-10-21 where it had become
false. `core/` is Faraday's code: a fork of OpenSignerKit at
`d26b84e92f4e960071fd4a8279112bf6d7f0d34c` (F65; F72 makes it a hard
fork), with Faraday's changes listed in `docs/history/UPSTREAM.md`. This file is
rewritten when the key boundary (F70) is built. One row per
crate that touches a key, a seed, a signature, a cipher or a passphrase,
saying what it computes, whether that is written in this tree or taken
from a crate, and what it is checked against. For someone deciding what
to read to audit this device, or how far a change reaches.

Upstream's §16.138 (`docs/WALLETS.md` §1 rule 5) says code that defines
bytes, or answers a question with no screen in it, is a `core/` crate.
For cryptography Faraday's app crate does not hold to that today:
`faraday-core` derives keys from seeds and calls the signing functions
itself (its row in §1). F70 moves that into one crate, `faraday-keys`;
until it is built, read `faraday-core` as a crate that computes. This
file says which crates compute cryptography, which only carry or display
bytes other crates computed, and which never see a key at all.

Three tiers, used through the rest of this file:

- **Computes** — makes, derives, signs, encrypts or decrypts with a key,
  a seed or a passphrase, or turns user entropy into one.
- **Hashes** — calls `faraday-crypto`'s SHA-256 for an integrity check (a
  file's checksum, a QR transfer's envelope) and never holds a private
  key or a passphrase.
- **Carries** — holds or renders bytes that a *Computes* crate already
  produced (an address, a fingerprint, a QR code, a descriptor), and
  computes nothing itself.

## 1. Crates that compute cryptography

### `core/` (OpenSigner's code as forked; changes listed in `docs/history/UPSTREAM.md`)

| Crate | What it computes | Written here, or from a crate | Checked against |
|---|---|---|---|
| `faraday-crypto` | The hash/KDF choke point every other crate calls through (`sha256`, `hmac_sha256`/`512`, `pbkdf2_hmac_sha256`/`512`, `blake2b`, `scrypt`); `Secret`, `Pinned` (mlock'd pages) and `Sealed`/`SessionKey` wrappers that hold or encrypt a secret in memory. No signing, no curve operations, no random number generator: entropy is handed in by the shell. | SHA-256, HMAC, PBKDF2: RustCrypto crates. **BLAKE2b and scrypt are written here** (one caller each, LND's `aezeed`) rather than taken as crates. | `core/faraday-crypto/tests/primitives.rs`; BLAKE2b against RFC 7693's own vectors, scrypt against RFC 7914's, inline in `blake2b.rs`/`scrypt.rs`. |
| `faraday-bip` | BIP-32 keys (wraps `bitcoin::bip32`, so private material never gains `Debug`/`Clone`/a text form), BIP-39 mnemonics, BIP-44/49/84/86 accounts, SLIP-132, BIP-380 descriptor checksums, BIP-85 child mnemonics, SLIP-39 Shamir backup, BIP-327 MuSig2, BIP 445 FROST (with the trusted-dealer split), BIP 93 codex32, aezeed. | Keys/addresses: `bitcoin`'s own `bip32` (external, vetted). **BIP-39, SLIP-39, MuSig2, FROST and codex32 are written here** — no vetted `no_std` crate exists for them on this tree's pin, or (BIP-39) the explorer screens need intermediate values a library hides. | `tools/vectors/{bip32,bip39,bip85,bip93,bip129,bip327,bip328,bip352,bip387,bip390,bip392,bip445}/`; BIP-39 and seed derivation additionally cross-checked against the `bip39` crate as a dev-dependency (`faraday-bip::crosscheck`). |
| `faraday-psbt` | PSBT parse/serialise, inspection (amounts, fee, verified change, warnings) from non-secret data only, signing (key derived from the PSBT's own origin data, never a guessed path; deterministic nonces; verifies its own signature before writing it; signs twice to check determinism), BIP-137/BIP-322 message signing, finalisation, and verification of signatures already on a transaction. | Signing itself: `faraday-bip`'s keys, over `bitcoin`'s secp256k1. The PSBT format, inspection and verification logic: written here. | `tools/vectors/psbt/`, `tools/vectors/message/`; wallet fixtures built and checked in `core/faraday-psbt/examples/wallet.rs` / `tests/wallet_fixtures.rs`; `docs/WALLETS.md` §8 additionally reproduces agreement against the Coldcard Q simulator and SeedSigner's libraries for the flows that should match. |
| `faraday-entropy` | Turns user-supplied dice, coin flips, hex, playing cards or camera noise into `RawEntropy` of the right size for a BIP-39 key, with the mistakes real hands make checked for and warned on; mixes two or more sources by XOR over their SHA-256 commitments. | Written here (bit/face accumulation and packing; no external entropy crate fits a fixed-size, erase-on-drop buffer). | Its own unit tests (no published vectors exist for "pack these dice as entropy" — it is deterministic bit-packing, not a named standard); exercised end to end through `faraday-core/tests/keygen.rs`. |
| `faraday-backup` | Two encrypted file formats: `.oskb` (a key's words, a master seed, a note or a recovery sheet) and KDBX 4. Both stretch the passphrase with Argon2id at a caller-chosen, file-carried `Cost`; neither draws its own randomness. | Argon2id, ChaCha20-Poly1305, the KDBX cipher stack: RustCrypto crates (`argon2`, `chacha20poly1305`, `chacha20`, `sha2`, `hmac`). The two file formats themselves: written here. | `tools/vectors/backup/`; `.oskb` additionally read by `tools/backup/decrypt.py` against two independent published libraries. |

`faraday-bip` also uses private keys: BIP-85 derives from one, MuSig2 and
FROST sign with one, a silent payments wallet holds its scan key, and
the vanity grinder derives keys while it searches.

### `faraday/` (Faraday's own)

| Crate | What it computes | Written here, or from a crate | Checked against |
|---|---|---|---|
| `faraday-core` | **Uses keys.** It works out a key's BIP-39 seed and BIP-32 master key, derives the private keys it signs with, and calls the signing functions: message signing (`wallet.rs`), Lightning (`lightning.rs`), the BSMS signer record (`create.rs`), BIP-85 (`bip85.rs`), seed formats (`seeds.rs`), the GPG, APK and Secure Boot keys (`gpg.rs`, `apk.rs`, `secureboot.rs`), and a vault's keys and their identity (`vaults.rs`, `vault_screens.rs`). It holds words, passphrases and seeds in its own state. | No primitive is written here; every operation calls a crate in this section or `core/`. Which key is derived, when, and for what is written here. | `faraday/faraday-core/tests/` (among others `bip85.rs`, `lightning.rs`, `gpg.rs` and `apk.rs`, each flow end to end); the primitives' own vectors. F70 moves all of this into `faraday-keys`. |
| `faraday-apk` | An APK signing key: RSA-3072 with a self-signed certificate; APK Signature Scheme v2 and v3 signatures over an APK, and reading a signed APK's signing block. | RSA: the `rsa` crate. The certificate and the APK signing block: written here. | `faraday/faraday-apk/tests/judged.rs`: `apksigner verify` accepts each signed fixture and prints the fingerprint Faraday shows, and `apksigner sign` with the same key makes the same file byte for byte; `tests/reader.rs` for the reader. |
| `faraday-vault` | The Faraday vault: one file, four equal slots, each its own passphrase, a slot nobody opens filled with random-looking bytes so the file does not say how many are in use. Argon2id stretches the passphrase; **HKDF-SHA256 (hand-assembled from `hmac`+`sha2` — one extract and the first expand block, not a dedicated `hkdf` crate) takes the stretched key to each slot's key**; XChaCha20-Poly1305 is the slot cipher; a ChaCha20 keystream fills a new vault's salt, nonces and unused slots from the caller's seed. | Argon2id, XChaCha20-Poly1305, ChaCha20, SHA-256/HMAC: RustCrypto crates. The four-equal-slot container, and the HKDF construction over those primitives: written here. | `faraday/faraday-vault/tests/vectors.rs` against `tools/vectors/vault/`; `tools/vault/open.py` opens the same vectors independently, with either `argon2-cffi`+`pynacl` or `cryptography` on OpenSSL. |
| `faraday-pgp` | Version-4 OpenPGP packets for one key shape (Ed25519 certification + Ed25519 signing subkey): self-signatures, the subkey's back-signature, detached signatures over SHA-256/512, a revocation certificate and its `.rev` file, a `paperkey`-format export, and the reader of public armour that decides what Faraday calls a public file (every packet read to its exact end). | Ed25519: `ed25519-dalek`. SHA-1 (fingerprints only, not signatures), SHA-256/512: `sha1`/`sha2`. The packet format: written here. | `faraday/faraday-pgp/tests/gnupg.rs`: every packet judged by `gpg` 2.4 (imports, verifies including signatures whose r or s starts with a zero byte, rejects other data, takes the revocation, sees a key expire and be renewed, refuses a `.rev` file until its colon is removed) and by `paperkey` 1.6 (identical lines, seeds with leading zero bytes); `tests/read.rs` and the `pgp_armor` fuzz target for the reader. |
| `faraday-sb` | An owner's PK, KEK and db as RSA-2048 keys with self-signed X.509 certificates, EFI signature lists, signed `.auth` updates, Authenticode signatures over `BOOTX64.EFI`, and checking one against a db certificate. | RSA-2048 (**pinned to `0.10.0-rc.18`, a pre-1.0 release candidate — noted in `docs/deps/rsa.md`**): the `rsa` crate. The RNG `rsa` draws from is a **ChaCha20 keystream written here** (`rng.rs`) over the caller's own seed, implementing `rand_core`'s trait. X.509/CMS structure, Authenticode: written here over the RustCrypto DER/X.509/CMS crates. | `faraday/faraday-sb/tests/judged.rs`: every certificate and `.auth`'s PKCS #7 judged by `openssl`; signed images checked by `sbverify`. |

## 2. In-house algorithms, named

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
- The `.oskb` and KDBX 4 file formats, and their HKDF-SHA256 construction
  (`faraday-backup`, `faraday-vault`)
- The four-equal-slot vault container (`faraday-vault`)
- OpenPGP v4 packet construction (`faraday-pgp`)
- X.509/CMS structure, Authenticode signing, and the RSA RNG adapter
  (`faraday-sb`)
- The APK signing block and its v2 and v3 signatures (`faraday-apk`)

Every one of these is checked against either a published vector set, an
independent library or tool, or (BLAKE2b, scrypt) the RFC's own test
vectors — see the "Checked against" column above. None is cryptography
invented for this project: each implements a named, published algorithm
or file format.

## 3. Crates that touch cryptographic bytes but compute none

| Crate | What it does with the bytes |
|---|---|
| `faraday-codec` | QR/UR encoding and decoding, classification, PNG. Calls `faraday-crypto::sha256` only for SeedQR's own integrity check; never holds a private key. |
| `faraday-qr` | BBQr, numbered text parts, the Faraday file envelope, the UR registry's key/wallet types. Calls `sha256` to check an envelope's declared hash against its data; computes no key material. |
| `faraday-draw` | Layout engine and renderer. Takes `faraday-bip`/`faraday-codec` types (a fingerprint, an address, a QR code) to draw; never computes one. |
| `faraday-shell-api` | The event/command contract between an app and its shell. No dependency on any crypto crate. |
| `faraday-intake` | Reads every scan, paste and file first: whether it is a seed (SeedQR, words, checked through `faraday-bip`) or another secret, and what a transfer becomes. Holds what it reads in `Zeroizing` buffers until it is handed on; computes no key. |

## 4. Crates with no cryptographic involvement

`faraday-fat` (FAT16/32), `faraday-disk` (the unprivileged disk process),
`faraday-grant` (the `CAP_CHOWN` helper), `faraday-files` (the Place
trait, the pipe protocol), `faraday-storage` (Inbox/Outbox, PDFs),
`faraday-scanner` (camera frames), `faraday-strings` (the words on
screen), every shell (`faraday-desktop`, `faraday-stick`,
`faraday-snapshot`, `faraday-window`, `faraday-mediafoundation`, the
Starter's, `faraday-v4l2`, `faraday-avfoundation`), and the dev
tools (`faraday-testkit`, `faraday-argon2-bench`, `faraday-fontbake`,
`costs-lint`). The Starter handles a BitLocker recovery password through
Windows' own API and computes nothing with it.
These matter to the trust model (`PLAN.md` §4 — privilege separation,
untrusted input, the stick rule) but never see a key, a seed or a
passphrase; that is a property of the process boundaries in §4, not of
anything in this file.

## 5. The dependency chain

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
   │      faraday-pgp     │     (Faraday's own: HKDF, OpenPGP, X.509/Authenticode, APK)
   │      faraday-sb      │
   │      faraday-apk     │
   │          │           │
   └──────────┴─────┬─────┘
                     ▼
              faraday-core               (flows and wording, and key use until F70)
                     │
                     ▼
            faraday-draw, every shell          (rendering, input, storage; zero cryptography)
```

A change below the first horizontal line can change what a key, a
signature or a ciphertext *is*. A change in `faraday-draw` or a shell can
change what the person sees and when, never the bytes underneath it.
`faraday-core` is in between until F70: most of it is flows and wording,
but a change to its key use can change which key signs.

## 6. The external dependency layer

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

## 7. Reading a change

A diff under `core/faraday-crypto`, `faraday-bip`, `faraday-psbt`, `faraday-backup`,
`faraday-entropy`, or `faraday/faraday-vault`, `faraday-pgp`, `faraday-sb`,
`faraday-apk`, or to `faraday-core`'s key use (§1), can change
cryptographic output and needs §1/§2's scrutiny:
re-run the vectors named above, and for anything marked "written here",
read the diff against the standard it implements, not just against its
own tests.

A diff under `faraday-codec`/`faraday-qr` that stays inside existing
functions needs only an integrity-check read (§3); a new function there
that starts touching a key would move it into §1 and this file would be
wrong until updated.

A diff under `faraday-draw`, a shell, anything in §4, or `faraday-core`
outside its key use changes a screen, a flow or a process boundary, never a
cryptographic result, unless it changes what bytes get passed into a §1
crate — which is itself worth noticing, since that is the one way a
"flows only" change can matter here.
