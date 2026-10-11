# What is cryptography, and where it lives

**Status:** written 2026-10-06, against upstream at c418768. One row per
crate that touches a key, a seed, a signature, a cipher or a passphrase,
saying what it computes, whether that is written in this tree or taken
from a crate, and what it is checked against. For someone deciding what
to read to audit this device, or how far a change reaches.

The boundary is already a rule, not a proposal: upstream's §16.138
(`docs/WALLETS.md` §1 rule 5) says code that defines bytes, or answers a
question with no screen in it, is a `core/` crate, and the app crates
(`opensigner-core`, `faraday-core`) hold only flows and wording. This file
makes that boundary explicit for cryptography specifically: which crates
compute it, which only carry or display bytes that other crates computed,
and which never see a key at all.

Three tiers, used through the rest of this file:

- **Computes** — makes, derives, signs, encrypts or decrypts with a key,
  a seed or a passphrase, or turns user entropy into one.
- **Hashes** — calls `osk-crypto`'s SHA-256 for an integrity check (a
  file's checksum, a QR transfer's envelope) and never holds a private
  key or a passphrase.
- **Carries** — holds or renders bytes that a *Computes* crate already
  produced (an address, a fingerprint, a QR code, a descriptor), and
  computes nothing itself.

## 1. Crates that compute cryptography

### `core/` (upstream, shared with OpenSigner)

| Crate | What it computes | Written here, or from a crate | Checked against |
|---|---|---|---|
| `osk-crypto` | The hash/KDF choke point every other crate calls through (`sha256`, `hmac_sha256`/`512`, `pbkdf2_hmac_sha256`/`512`, `blake2b`, `scrypt`); `Secret`, `Pinned` (mlock'd pages) and `Sealed`/`SessionKey` wrappers that hold or encrypt a secret in memory. No signing, no curve operations, no random number generator: entropy is handed in by the shell. | SHA-256, HMAC, PBKDF2: RustCrypto crates. **BLAKE2b and scrypt are written here** (one caller each, LND's `aezeed`) rather than taken as crates. | `core/osk-crypto/tests/primitives.rs`; BLAKE2b against RFC 7693's own vectors, scrypt against RFC 7914's, inline in `blake2b.rs`/`scrypt.rs`. |
| `osk-bip` | BIP-32 keys (wraps `bitcoin::bip32`, so private material never gains `Debug`/`Clone`/a text form), BIP-39 mnemonics, BIP-44/49/84/86 accounts, SLIP-132, BIP-380 descriptor checksums, BIP-85 child mnemonics, SLIP-39 Shamir backup, BIP-327 MuSig2, BIP 445 FROST (with the trusted-dealer split), BIP 93 codex32, aezeed. | Keys/addresses: `bitcoin`'s own `bip32` (external, vetted). **BIP-39, SLIP-39, MuSig2, FROST and codex32 are written here** — no vetted `no_std` crate exists for them on this tree's pin, or (BIP-39) the explorer screens need intermediate values a library hides. | `tools/vectors/{bip32,bip39,bip85,bip93,bip129,bip327,bip328,bip352,bip387,bip390,bip392,bip445}/`; BIP-39 and seed derivation additionally cross-checked against the `bip39` crate as a dev-dependency (`osk-bip::crosscheck`). |
| `osk-psbt` | **The only crate in `core/` that uses a private key.** PSBT parse/serialise, inspection (amounts, fee, verified change, warnings) from non-secret data only, signing (key derived from the PSBT's own origin data, never a guessed path; deterministic nonces; verifies its own signature before writing it; signs twice to check determinism), BIP-137/BIP-322 message signing, finalisation, and verification of signatures already on a transaction. | Signing itself: `osk-bip`'s keys, over `bitcoin`'s secp256k1. The PSBT format, inspection and verification logic: written here. | `tools/vectors/psbt/`, `tools/vectors/message/`; wallet fixtures built and checked in `core/osk-psbt/examples/wallet.rs` / `tests/wallet_fixtures.rs`; `docs/WALLETS.md` §8 additionally reproduces agreement against the Coldcard Q simulator and SeedSigner's libraries for the flows that should match. |
| `osk-entropy` | Turns user-supplied dice, coin flips, hex, playing cards or camera noise into `RawEntropy` of the right size for a BIP-39 key, with the mistakes real hands make checked for and warned on; mixes two or more sources by XOR over their SHA-256 commitments. | Written here (bit/face accumulation and packing; no external entropy crate fits a fixed-size, erase-on-drop buffer). | Its own unit tests (no published vectors exist for "pack these dice as entropy" — it is deterministic bit-packing, not a named standard); exercised end to end through `faraday-core/tests/keygen.rs`. |
| `osk-backup` | Two encrypted file formats: `.oskb` (a key's words, a master seed, a note or a recovery sheet) and KDBX 4. Both stretch the passphrase with Argon2id at a caller-chosen, file-carried `Cost`; neither draws its own randomness. | Argon2id, ChaCha20-Poly1305, the KDBX cipher stack: RustCrypto crates (`argon2`, `chacha20poly1305`, `chacha20`, `sha2`, `hmac`). The two file formats themselves: written here. | `tools/vectors/backup/`; `.oskb` additionally read by `tools/backup/decrypt.py` against two independent published libraries. |
| `osk-keep` | The format and cryptography of the key kept on a Tier B device's own storage. Not used by Faraday (`faraday-vault` holds keys instead; see `docs/OPENSIGNER-PARITY.md` §1). | Built on `osk-backup` and `osk-bip`. | Upstream's own tests; out of Faraday's build. |

### `faraday/` (Faraday's own)

| Crate | What it computes | Written here, or from a crate | Checked against |
|---|---|---|---|
| `faraday-vault` | The Faraday vault: one file, four equal slots, each its own passphrase, a slot nobody opens filled with random-looking bytes so the file does not say how many are in use. Argon2id stretches the passphrase; **HKDF-SHA256 (hand-assembled from `hmac`+`sha2` — one extract and the first expand block, not a dedicated `hkdf` crate) takes the stretched key to each slot's key**; XChaCha20-Poly1305 is the slot cipher; a ChaCha20 keystream fills a new vault's salt, nonces and unused slots from the caller's seed. | Argon2id, XChaCha20-Poly1305, ChaCha20, SHA-256/HMAC: RustCrypto crates. The four-equal-slot container, and the HKDF construction over those primitives: written here. | `faraday/faraday-vault/tests/vectors.rs` against `tools/vectors/vault/`; `tools/vault/open.py` opens the same vectors independently, with either `argon2-cffi`+`pynacl` or `cryptography` on OpenSSL. |
| `faraday-pgp` | Version-4 OpenPGP packets for one key shape (Ed25519 certification + Ed25519 signing subkey): self-signatures, the subkey's back-signature, detached signatures over SHA-256/512, a revocation certificate, a `paperkey`-format export. | Ed25519: `ed25519-dalek`. SHA-1 (fingerprints only, not signatures), SHA-256/512: `sha1`/`sha2`. The packet format: written here. | `faraday/faraday-pgp/tests/gnupg.rs`: every packet judged by `gpg` 2.4 (imports, verifies, rejects other data, takes the revocation) and by `paperkey` 1.6 (identical lines). |
| `faraday-sb` | An owner's PK, KEK and db as RSA-2048 keys with self-signed X.509 certificates, EFI signature lists, signed `.auth` updates, Authenticode signatures over `BOOTX64.EFI`, and checking one against a db certificate. | RSA-2048 (**pinned to `0.10.0-rc.18`, a pre-1.0 release candidate — noted in `docs/deps/rsa.md`**): the `rsa` crate. The RNG `rsa` draws from is a **ChaCha20 keystream written here** (`rng.rs`) over the caller's own seed, implementing `rand_core`'s trait. X.509/CMS structure, Authenticode: written here over the RustCrypto DER/X.509/CMS crates. | `faraday/faraday-sb/tests/judged.rs`: every certificate and `.auth`'s PKCS #7 judged by `openssl`; signed images checked by `sbverify`. |

## 2. In-house algorithms, named

The highest-leverage single fact for an auditor: everything above marked
"written here" rather than "from a crate". In one list:

- BIP-39 mnemonics (`osk-bip::bip39`)
- SLIP-39 Shamir backup (`osk-bip::slip39`)
- BIP-327 MuSig2 (`osk-bip::musig`)
- BIP 445 FROST, including the trusted-dealer split (`osk-bip::frost`)
- BIP 93 codex32 (`osk-bip::codex32`)
- The PSBT format, inspection, signing orchestration and verification
  logic (`osk-psbt`, over `osk-bip`'s keys and `bitcoin`'s secp256k1)
- Dice/coin/card/hex/camera entropy accumulation and packing
  (`osk-entropy`)
- BLAKE2b and scrypt, one caller each (`osk-crypto`)
- The `.oskb` and KDBX 4 file formats, and their HKDF-SHA256 construction
  (`osk-backup`, `faraday-vault`)
- The four-equal-slot vault container (`faraday-vault`)
- OpenPGP v4 packet construction (`faraday-pgp`)
- X.509/CMS structure, Authenticode signing, and the RSA RNG adapter
  (`faraday-sb`)

Every one of these is checked against either a published vector set, an
independent library or tool, or (BLAKE2b, scrypt) the RFC's own test
vectors — see the "Checked against" column above. None is cryptography
invented for this project: each implements a named, published algorithm
or file format.

## 3. Crates that touch cryptographic bytes but compute none

| Crate | What it does with the bytes |
|---|---|
| `osk-codec` | QR/UR encoding and decoding, classification, PNG. Calls `osk-crypto::sha256` only for SeedQR's own integrity check; never holds a private key. |
| `faraday-qr` | BBQr, numbered text parts, the Faraday file envelope, the UR registry's key/wallet types. Calls `sha256` to check an envelope's declared hash against its data; computes no key material. |
| `osk-ui` | Layout engine and renderer. Takes `osk-bip`/`osk-codec` types (a fingerprint, an address, a QR code) to draw; never computes one. |
| `osk-shell-api` | The event/command contract between an app and its shell. No dependency on any crypto crate. |
| `opensigner-core`, `faraday-core` | Flows and wording, the state machines behind every screen. Depend on the crates above to call into, and hold `Secret`/`Zeroizing` values in their own state, but perform no cryptographic operation themselves (§16.138's whole point). |

## 4. Crates with no cryptographic involvement

`faraday-fat` (FAT16/32), `faraday-disk` (the unprivileged disk process),
`faraday-grant` (the `CAP_CHOWN` helper), `faraday-files` (the Place
trait, the pipe protocol), `faraday-storage` (Inbox/Outbox, PDFs),
`faraday-scanner`/`opensigner-scanner` (camera frame capture), every
shell (`faraday-desktop`, `faraday-stick`, `faraday-snapshot`,
`opensigner-desktop`/`pi`/`v4l2`/`avfoundation`), and the dev tools
(`faraday-testkit`, `faraday-argon2-bench`, `osk-fontbake`, `osk-qrgen`).
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
   osk-crypto            (the hash/KDF choke point; BLAKE2b, scrypt written here)
        │
   ┌────┴─────┐
   ▼          ▼
osk-bip   osk-entropy     (BIP-32/39/44.., in-house BIP-39/SLIP-39/MuSig2/FROST/codex32)
   │
   ├──────────┬───────────┬──────────┐
   ▼          ▼           ▼          ▼
osk-psbt  osk-backup  osk-codec   osk-keep
   │          │           │
   │      faraday-vault   │
   │      faraday-pgp     │     (Faraday's own: HKDF, OpenPGP, X.509/Authenticode)
   │      faraday-sb      │
   │          │           │
   └──────────┴─────┬─────┘
                     ▼
     opensigner-core / faraday-core      (flows and wording; zero cryptography)
                     │
                     ▼
            osk-ui, every shell          (rendering, input, storage; zero cryptography)
```

A change below the first horizontal line can change what a key, a
signature or a ciphertext *is*. A change at or below `opensigner-core` /
`faraday-core` can change what the person sees and when, never the bytes
underneath it — the two layers are worth reading differently.

## 6. The external dependency layer

Every crate named "from a crate" above is pinned in `Cargo.lock` and
carries its own note under `docs/deps/` (why it was chosen, what it
replaces or what it is pinned against): `argon2.md`, `chacha20.md`,
`chacha20poly1305.md`, `ed25519-dalek.md`, `hmac.md`, `pbkdf2.md`,
`rand_core.md`, `rsa.md`, `sha1.md`, `sha2.md`, `subtle.md`, `getrandom.md`,
`rqrr.md`, plus non-cryptographic build/platform deps (`cc.md`,
`libc.md`, `jni-sys.md`, `miniz_oxide.md`, `clipboard.md`, `ur.md`,
`miniscript.md`). `deny.toml` (advisories, licences, duplicate and source
checks) and `supply-chain/` (`cargo vet`'s `audits.toml`, `config.toml`,
`imports.lock`) are the mechanism that keeps those pins themselves
reviewed, not just the code calling them.

## 7. Reading a change

A diff under `core/osk-crypto`, `osk-bip`, `osk-psbt`, `osk-backup`,
`osk-keep`, `osk-entropy`, or `faraday/faraday-vault`, `faraday-pgp`,
`faraday-sb` can change cryptographic output and needs §1/§2's scrutiny:
re-run the vectors named above, and for anything marked "written here",
read the diff against the standard it implements, not just against its
own tests.

A diff under `osk-codec`/`faraday-qr` that stays inside existing
functions needs only an integrity-check read (§3); a new function there
that starts touching a key would move it into §1 and this file would be
wrong until updated.

A diff under `opensigner-core`, `faraday-core`, `osk-ui`, a shell, or
anything in §4 changes a screen, a flow or a process boundary, never a
cryptographic result, unless it changes what bytes get passed into a §1
crate — which is itself worth noticing, since that is the one way a
"flows only" change can matter here.
