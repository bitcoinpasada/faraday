# Standards: what Faraday follows, how fully, and where it decides

**Kind:** Reference
**Status:** Built

For each wallet kind, key and backup method, signing and transfer
format and tool: the standard it follows and that standard's status, how
completely Faraday implements it, how that is checked, and where Faraday
made its own call. §1 to §7 are written by `docs/crypto/gen.py` from the
manifest of cryptographic actions, `docs/crypto/actions.toml`, between
marker lines, and are never edited by hand: a change is made in the
manifest, which also gives the Learn pages' "How Faraday does it" blocks
(`docs/CRYPTO-LEARN.md`). §0, §8 and §9 are written here. Decisions are
in `docs/DECISIONS.md`; what each cryptographic crate is checked against
is in `docs/AUDIT.md` and `docs/CROSSCHECK.md`.

## 0. What goes into Faraday

The rule for what the app offers (owner, 2026-10-23; written in the
decision F109):

- **A published standard, followed and checked.** A wallet kind, key
  method, format or signing procedure is in the app when it follows a
  published standard (a BIP at Complete or Deployed, a SLIP, an RFC, or a
  convention other wallets already use), is built in full or names the
  part it leaves out, and is checked against that standard's vectors or
  an implementation Faraday did not write (F79's cross-check, other
  wallets; F160).
- **A draft that is stable enough.** A draft whose text has held still,
  that has independent implementations and that Faraday's cross-check
  agrees with, can be in the app; the table says it is a draft (codex32,
  BIP 93).
- **Faraday's own, conservative.** Where no standard exists, Faraday's
  own format or rule is in the app when it fails closed (a refusal, a
  record layout) and a decision records it. A cryptographic construction
  of Faraday's own is in the app only once it has been reviewed, or
  when it can only add to a standard's strength (mixing a person's own
  randomness into the system's); §7 lists them for outside review.
- **No way past a check without a legitimate reason** (F136). A check that
  keeps a person's keys or money safe has no way past it unless a decision
  names a real case that needs one. Where one does, the way past is a hold,
  and the hold says what is not checked and the case it is for. A
  transaction that can never be valid, or a claim about the person's own
  key that is false, is refused. A note that keeps nothing safe (dust,
  address reuse) is a note, not a check, and needs no hold.
- **Checked against an implementation Faraday did not write** (F160).
  Anything that makes or reads a key, a secret, a seed or backup
  encoding, an address, a signature or a transaction is checked against
  an implementation Faraday did not write: an F79 cross-check area
  against a pinned outside reference (`tools/crosscheck/README.md`), or
  the other tool itself end to end (Bitcoin Core, `gpg`, `openssl`); and
  against the standard's published vectors where they exist. A model
  written by Faraday's own authors, from the spec or from the code, is
  not independent: it is said to be Faraday's own wherever the check is
  described, and each action's `independence` in the manifest says which
  its reference is. Today several areas compare against such models
  (`docs/CROSSCHECK.md` §10.7 lists them and what replaces them). Where
  no outside implementation can exist (Faraday's own formats and
  procedures), the construction is labelled "own construction, not yet
  independently reviewed" until an outside party reviews it, by a model
  of its own written from the spec or by reproducing the published
  vectors (§7).
  Exceptions: flow and screen logic, and plumbing that carries bytes it
  does not interpret (the backup plan, the file envelope's framing), rest
  on Faraday's own tests; a small standard its published vectors cover in
  full (TOTP's RFC 6238, the Diceware lists' digests) may rest on them.
  New cryptographic code lands with its check; a gap found later is
  closed before 1.0, not a block on every release.
- **Otherwise it waits in the repository.** Work on a feature whose
  standard is unpublished, or whose construction is Faraday's own and not
  yet reviewed, stays in the repository, built and tested but compiled
  out of the app, until its standard is published or it is reviewed.
- **Scope** (F116). Faraday is an offline signer and keeper of keys: a
  person's Bitcoin wallets and their backups first, and the other keys a
  person keeps offline beside them: an OpenPGP key (signing with GnuPG),
  TOTP secrets, Secure Boot keys. A new kind of key or signature comes in
  on the owner's word, recorded in DECISIONS; the list below names what
  is left out.

`just lint` holds this section: `tools/lint-standards.py` fails on
cryptography the cross-check's stamp does not cover and
`tools/lint-standards.toml` does not list with its reason, on app code
that reaches work kept out of the app (§0.1), and on a hold that names no
decision; `docs/crypto/verified.py` fails on a row of §2 to §5 that names
no check, and on a cross-check area or vectors folder no row names.

### 0.1 Left out, and why

| Left out | Why | Where it stands |
|---|---|---|
| FROST threshold wallets | BIP 445 and a key-generation standard (ChillDKG) are drafts, and the dealer and the signing across places were Faraday's own constructions, not reviewed | Removed from the repository (F127); back only as a new implementation of the published standards |
| MuSig2 signing across places | Secret nonces drawn at one place and carried or kept in the vault between visits; BIP 327 advises against storing a secret nonce | Compiled out of the app (F109): no MuSig2 secret nonce outlives the session it was drawn in; MuSig2 with the session open between rounds, and signing last, stay |
| Anti-exfil (`docs/ANTI-EXFIL.md`) | FractalEncrypt's four-stage exchange over QR codes is an alpha, AI-written and AI-reviewed with no human audit, under an unregistered UR type; no BIP defines anti-exfil or PSBT fields for it | Not built; planned in dev builds only, behind the `experimental` feature, which no release or test release carries (F162); in the app once a published standard carries it |
| Sending to silent payments | BIP 374 and BIP 375 are drafts; receiving (BIP 352) is in the app | Built in `core/`, not reached from the app |
| SSH signatures and an SSH CA, minisign and signify, an X.509 certificate authority | Outside a Bitcoin signer's scope | Not planned |
| The Companion phone app, and APK signing with it | Dropped (F34); APK signing removed (F99) | Removed |
| Reading `.oskb` backups | A format Faraday no longer writes; the vault's records are its own (F97) | Removed |
| An unspendable `tr()` internal key stated other than as `H` or an extended key over `H`: a bare `H + rG` with `r` beside the descriptor (Coldcard's `r=<hex>`), the `unspend(<hex>)` key expression, and a chain code computed from the wallet's keys | BIP 341 recommends `H + rG` and proving it by revealing `r`, but defines no way to write `r`; BIP 380 to 390 and 388 define none either. `r=` and `unspend()` are one vendor's syntax and a proposal (Coldcard has since deprecated both for an extended key over `H`); the computed chain code is a draft BIP | Read as a key someone holds, with the danger line (F132). Comes in when a BIP defines the syntax. An extended key whose public key is `H`, with any chain code, is read as unspendable now: its key at each index is `H + rG` with `r` worked out from the chain code (BIP 32) |

## 1. How to read it

**Statuses.** The BIPs repository uses BIP 3's statuses: Draft,
Complete, Deployed, Closed. The old "Final" corresponds roughly to
Deployed and "Proposed" to Complete. An RFC has the RFC editor's status.
A convention is a format other wallets already use, with no standards
body. Each standard's status was read from the source its row names, on
the date beside it: the BIPs repository's master branch, the RFC editor,
or the text vendored in `tools/vectors` (marked "vendored" in §2 to §5).
"Status not checked" marks a standard whose status was not looked up.
Links follow the pattern
`https://github.com/bitcoin/bips/blob/master/bip-NNNN.mediawiki`.

<!-- crypto-manifest: begin §1 -->
| Standard | Name | Status | Read from | Checked |
|---|---|---|---|---|
| BIP 32 | Hierarchical Deterministic Wallets | Deployed | bips master | 2026-10-23 |
| BIP 39 | Mnemonic code for generating deterministic keys | Deployed | bips master | 2026-10-23 |
| BIP 44 | Multi-Account Hierarchy for Deterministic Wallets | Deployed | bips master | 2026-10-23 |
| BIP 45 | Structure for Deterministic P2SH Multisignature Wallets | Complete | bips master | 2026-10-23 |
| BIP 48 | Multi-Script Hierarchy for Multi-Sig Wallets | Deployed | bips master | 2026-10-23 |
| BIP 49 | Derivation scheme for P2WPKH-nested-in-P2SH based accounts | Deployed | bips master | 2026-10-23 |
| BIP 67 | Deterministic Pay-to-script-hash multi-signature addresses through public key sorting | Complete | bips master | 2026-10-23 |
| BIP 84 | Derivation scheme for P2WPKH based accounts | Deployed | bips master | 2026-10-23 |
| BIP 85 | Deterministic Entropy From BIP32 Keychains | Deployed | vendored: tools/vectors/bip85 | 2026-10-23 |
| BIP 86 | Key Derivation for Single Key P2TR Outputs | Deployed | bips master | 2026-10-23 |
| BIP 93 | codex32: Checksummed SSS-able BIP32 Seeds | Draft | vendored: tools/vectors/bip93 | 2026-10-23 |
| BIP 129 | Bitcoin Secure Multisig Setup (BSMS) | Complete | vendored: tools/vectors/bip129 | 2026-10-23 |
| BIP 137 | Signatures of Messages using Private Keys | Deployed | bips master | 2026-10-23 |
| BIP 141 | Segregated Witness (Consensus layer) | Deployed | bips master | 2026-10-23 |
| BIP 143 | Transaction Signature Verification for Version 0 Witness Program | Deployed | bips master | 2026-10-23 |
| BIP 144 | Segregated Witness (Peer Services) | Deployed | bips master | 2026-10-23 |
| BIP 173 | Base32 address format for native v0-16 witness outputs | Deployed | bips master | 2026-10-23 |
| BIP 174 | Partially Signed Bitcoin Transaction Format | Deployed | bips master | 2026-10-23 |
| BIP 321 | URI Scheme for Bitcoin Payments | Complete | vendored: tools/vectors/bip392 | 2026-10-23 |
| BIP 322 | Generic Signed Message Format | Complete | bips master | 2026-10-23 |
| BIP 327 | MuSig2 for BIP340-compatible Multi-Signatures | Deployed | bips master | 2026-10-23 |
| BIP 328 | Derivation Scheme for MuSig2 Aggregate Keys | Complete | bips master | 2026-10-23 |
| BIP 340 | Schnorr Signatures for secp256k1 | Deployed | bips master | 2026-10-23 |
| BIP 341 | Taproot: SegWit version 1 spending rules | Deployed | bips master | 2026-10-23 |
| BIP 342 | Validation of Taproot Scripts | Deployed | bips master | 2026-10-23 |
| BIP 350 | Bech32m Format for v1+ Witness Addresses | Deployed | bips master | 2026-10-23 |
| BIP 352 | Silent Payments | Complete | vendored: tools/vectors/bip352 | 2026-10-23 |
| BIP 353 | DNS Payment Instructions | Complete | vendored: tools/vectors/bip392 | 2026-10-23 |
| BIP 370 | PSBT Version 2 | Deployed | bips master | 2026-10-23 |
| BIP 373 | MuSig2 PSBT Fields | Complete | bips master | 2026-10-23 |
| BIP 374 | Discrete Log Equality Proofs | Draft | bips master | 2026-10-23 |
| BIP 375 | Sending Silent Payments with PSBTs | Draft | bips master | 2026-10-23 |
| BIP 379 | Miniscript | Draft | bips master | 2026-10-23 |
| BIP 380 | Output Script Descriptors General Operation | Deployed | bips master | 2026-10-23 |
| BIP 381 | Non-Segwit Output Script Descriptors | Deployed | bips master | 2026-10-23 |
| BIP 382 | Output Script Descriptors: wsh | Deployed | bips master | 2026-10-23 |
| BIP 383 | Output Script Descriptors: multi | Deployed | bips master | 2026-10-23 |
| BIP 386 | Output Script Descriptors: tr | Deployed | bips master | 2026-10-23 |
| BIP 387 | Tapscript Multisig Output Script Descriptors | Deployed | vendored: tools/vectors/bip387 | 2026-10-23 |
| BIP 388 | Wallet Policies for Descriptor Wallets | Complete | bips master | 2026-10-23 |
| BIP 389 | Multipath Descriptor Key Expressions | Draft | bips master | 2026-10-23 |
| BIP 390 | musig() Descriptor Key Expression | Draft | vendored: tools/vectors/bip390 | 2026-10-23 |
| BIP 392 | Silent Payment Scanning Descriptors | Draft | vendored: tools/vectors/bip392 | 2026-10-23 |
| RFC 1924 | A Compact Representation of IPv6 Addresses (base 85 alphabet) | Status not checked | not looked up | 2026-10-28 |
| RFC 2104 | HMAC: Keyed-Hashing for Message Authentication | Status not checked | not looked up | 2026-10-28 |
| RFC 4226 | HOTP: An HMAC-Based One-Time Password Algorithm | Informational | rfc-editor.org | 2026-10-28 |
| RFC 4231 | Identifiers and Test Vectors for HMAC-SHA-224, HMAC-SHA-256, HMAC-SHA-384, and HMAC-SHA-512 | Status not checked | not looked up | 2026-10-28 |
| RFC 4648 | The Base16, Base32, and Base64 Data Encodings | Status not checked | not looked up | 2026-10-28 |
| RFC 4880 | OpenPGP Message Format, obsoleted by RFC 9580 | Proposed Standard | rfc-editor.org | 2026-10-28 |
| RFC 5280 | Internet X.509 Public Key Infrastructure Certificate and Certificate Revocation List (CRL) Profile | Proposed Standard | rfc-editor.org | 2026-10-28 |
| RFC 5652 | Cryptographic Message Syntax (CMS) | Internet Standard | rfc-editor.org | 2026-10-28 |
| RFC 5869 | HMAC-based Extract-and-Expand Key Derivation Function (HKDF) | Informational | rfc-editor.org | 2026-10-28 |
| RFC 6238 | TOTP: Time-Based One-Time Password Algorithm | Informational | rfc-editor.org | 2026-10-28 |
| RFC 6979 | Deterministic Usage of the Digital Signature Algorithm (DSA) and Elliptic Curve Digital Signature Algorithm (ECDSA) | Informational | rfc-editor.org | 2026-10-28 |
| RFC 7914 | The scrypt Password-Based Key Derivation Function | Status not checked | not looked up | 2026-10-28 |
| RFC 8017 | PKCS #1: RSA Cryptography Specifications Version 2.2 | Informational | rfc-editor.org | 2026-10-28 |
| RFC 8018 | PKCS #5: Password-Based Cryptography Specification Version 2.1 | Status not checked | not looked up | 2026-10-28 |
| RFC 8032 | Edwards-Curve Digital Signature Algorithm (EdDSA) | Informational | rfc-editor.org | 2026-10-28 |
| RFC 8259 | The JavaScript Object Notation (JSON) Data Interchange Format | Status not checked | not looked up | 2026-10-28 |
| RFC 8439 | ChaCha20 and Poly1305 for IETF Protocols | Status not checked | not looked up | 2026-10-28 |
| RFC 9106 | Argon2 Memory-Hard Function for Password Hashing and Proof-of-Work Applications | Informational | rfc-editor.org | 2026-10-28 |
| RFC 9580 | OpenPGP | Proposed Standard | rfc-editor.org | 2026-10-28 |
| Authenticode | Windows Authenticode Portable Executable Signature Format | Status not checked | Microsoft documentation | 2026-10-28 |
| Base58Check | Base58Check encoding, as Bitcoin Core writes it | Convention | tools/crosscheck/area_addr.py | 2026-10-28 |
| BBQr | BBQr, a file split across QR codes | Convention | bbqr.org | 2026-10-28 |
| BCR-2020-005 | Uniform Resources (UR), the animated-QR transport | Convention | vendored: tools/vectors/ur | 2026-10-28 |
| BCR-2020-006 | The crypto-psbt UR type | Convention | vendored: tools/vectors/ur | 2026-10-28 |
| BCR-2020-007 | UR Type Definition for Hierarchical Deterministic (HD) Keys | Convention | faraday/faraday-qr/src/registry.rs | 2026-10-28 |
| BCR-2020-010 | UR Type Definition for Bitcoin Output Descriptors | Convention | faraday/faraday-qr/src/registry.rs | 2026-10-28 |
| BCR-2020-012 | Bytewords | Convention | vendored: tools/vectors/ur | 2026-10-28 |
| BCR-2020-015 | UR Type Definition for BIP44 Accounts | Convention | faraday/faraday-qr/src/registry.rs | 2026-10-28 |
| Booklet dice de-biasing | codex32 booklet SSS32.ps of 2023-03-07, Dice De-biasing Worksheet | Convention | tools/reference/booklet-dice/README.md | 2026-10-28 |
| Codex32QR/v1-48 | Codex32QR/v1-48, one 48-character codex32 string in a QR code | Convention | vendored: tools/vectors/codex32qr | 2026-10-28 |
| Dice hashing of Coldcard and SeedSigner | Coldcard rolls.py, SeedSigner, EntropyLab, Keystone and BitBox dice procedures | Convention | tools/reference/dice/README.md | 2026-10-28 |
| draft-irtf-cfrg-xchacha-03 | XChaCha: eXtended-nonce ChaCha and AEAD_XChaCha20_Poly1305 | Draft | datatracker.ietf.org (expired draft) | 2026-10-28 |
| EFF word lists | EFF long and short Diceware word lists | Convention | vendored: tools/vectors/eff | 2026-10-28 |
| EntropyLab vanity counter | EntropyLab vanity-wasm counter | Convention | tools/reference/vanity/README.md | 2026-10-28 |
| FIPS 180-4 | Secure Hash Standard | Status not checked | not looked up | 2026-10-28 |
| ISO/IEC 18004 | QR Code bar code symbology specification | Status not checked | not read; compared with Project Nayuki's qrcodegen in the cross-check | 2026-10-28 |
| KeePass KDBX 4 | The KeePass database format, version 4 | Convention | keepass.info/help/kb/kdbx_4.html, pinned in tools/reference/kdbx4 | 2026-10-28 |
| ldk-node node key | ldk-node and rust-lightning node key from a BIP-39 seed | Convention | tools/reference/aezeed/README.md | 2026-10-28 |
| LND aezeed | LND cipher seed (aezeed) with AEZ v5 | Convention | tools/reference/aezeed/README.md | 2026-10-28 |
| otpauth Key URI Format | Key URI Format of Google Authenticator | Convention | github.com/google/google-authenticator wiki | 2026-10-28 |
| paperkey | paperkey's text and raw forms of an OpenPGP secret key | Convention | paperkey 1.6, tools/vectors/paperkey | 2026-10-28 |
| Seed XOR | Seed XOR, Coldcard's split of a seed into parts | Convention | vendored: tools/vectors/xor | 2026-10-28 |
| SeedQR | SeedQR and CompactSeedQR | Convention | vendored: tools/vectors/seedqr | 2026-10-28 |
| SLIP-0039 | Shamir's Secret-Sharing for Mnemonic Codes | Status not checked | vendored: tools/vectors/slip39 | 2026-10-28 |
| SLIP-0132 | Registered HD version bytes for BIP-0032 | Status not checked | not looked up | 2026-10-28 |
| Specter pNofM | Numbered text parts, pNofM | Convention | tools/reference/specter/README.md | 2026-10-28 |
| UAX #15 | Unicode Standard Annex #15: Unicode Normalization Forms | Complete | unicode.org | 2026-10-29 |
| UEFI Specification | Unified Extensible Firmware Interface Specification: Secure Boot, signature lists and authenticated variables | Status not checked | uefi.org | 2026-10-28 |
<!-- crypto-manifest: end §1 -->

**Columns.** *Completeness*: how much of the standard is built.
*Verified by*: published vectors, a second implementation (the F79
cross-check, `docs/CROSSCHECK.md`), Bitcoin Core or other wallets'
files. *Faraday's own call*: its kind, and the decision that records it
or "no decision". Kinds:

- **Conservative**: stricter than the standard; fails closed.
- **Neutral**: a format or interface choice with no security effect.
- **Novel**: Faraday's own cryptographic construction or protocol.

## 2. Wallet kinds

<!-- crypto-manifest: begin §2 -->
| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| Single key: `pkh`, `sh(wpkh)`, `wpkh`, `tr` (`faraday/faraday-core/src/create.rs`) | BIP 32, 44, 49, 84, 86, 173, 341, 350 (Deployed) | Full for derivation and addresses (rust-bitcoin) | vectors `tools/vectors/bip32`, `tools/vectors/addresses`; F79 area `bip32` against btclib-wallet's `bip32` over btclib and `ecc` (partly Faraday's own model, not an independent check); F79 area `addr` against Bitcoin Core's functional-test framework; Bitcoin Core end to end over every test wallet (`faraday/tools/core-check.py`, INTEROP §1, Core 31.1); 20 tests (faraday-bip, faraday-keys, faraday-core); self-test `bip32_vector`, `bip84_address`; fuzz `xkey`, `policy` | An extended private key is decoded by Faraday's own strict reader because bitcoin accepts seven of the sixteen serialisations vector 5 rejects. Conservative; no decision. Hardened steps are written h. Neutral; F66. The account is asked after the quorum, not on Kind. Neutral; F98. Restore takes a seed for a wallet's key slot only when its whole extended public key at the slot's path matches the wallet's, not the four bytes of its fingerprint. Conservative; F155. |
| Multisig `wsh` and `sh(wsh)` `sortedmulti` | BIP 48 (Deployed); BIP 67 (Complete) | Full: the account key is derived here; the script is read through a wallet policy | vectors `tools/vectors/bip32`; F79 area `bip32` against btclib-wallet's `bip32` over btclib and `ecc`; Bitcoin Core end to end, two sessions' signatures combined by Core (INTEROP §1); Sparrow's, Coldcard's and Electrum's files (`faraday/faraday-core/tests/all/interop.rs`); 2 tests (faraday-bip) | A multisig key with no origin is refused where a policy reads it. Conservative; F155. |
| Legacy multisig `sh(sortedmulti)` at `m/45'` | BIP 45 (Complete) defines `m/45'/cosigner/change/index`; Faraday uses `m/45'` as an account with `/<0;1>/*`, the Coldcard and Unchained export (a convention) | A convention, not BIP 45's scheme; the account key is derived as Single key's are | vectors `tools/vectors/bip32`, `tools/vectors/addresses`, `tools/vectors/bip387`, `tools/vectors/miniscript`; F79 area `bip32` against btclib-wallet's `bip32` over btclib and `ecc`; F79 area `miniscript` against Bitcoin Core v31.1's `bitcoind` on regtest; F79 area `addr` against Bitcoin Core's functional-test framework; Core's checksums (INTEROP §1); Bitcoin Core end to end over every test wallet (`faraday/tools/core-check.py`, INTEROP §1, Core 31.1); 23 tests (faraday-bip, faraday-core); self-test `bip32_vector`, `bip84_address`; fuzz `policy` | `m/45'` is an account path with no account number to choose. Neutral; no decision. |
| Taproot multisig `tr(H, sortedmulti_a)` (`core/faraday-bip/src/tapmulti.rs`) | BIP 387 (Deployed, vendored); BIP 341, 342, 386 (Deployed) | `sortedmulti_a` is parsed here because miniscript lacks it, as a multipath descriptor or one chain a descriptor (a receive and a change line, a receive line alone); a hardened wildcard is parsed but not derived | vectors `tools/vectors/bip387`; F79 area `taproot` against Core's `taproot_construct`, `key.py`'s tweak and `segwit_addr.py` (partly Faraday's own model, not an independent check); Core 31.1 PSBT round trip (`tools/vectors/psbt`, `wallet-tapmulti-*`); 3 tests (faraday-bip); fuzz `policy` | The path m/48'/coin'/0'/3' has no standard; it is Faraday's choice. Neutral; no decision. Only BIP 341's H, or an extended key over H, counts as an unspendable internal key. Conservative; F132. |
| Miniscript and timelocked recovery, Liana shapes (`core/faraday-bip/src/recovery.rs`, `core/faraday-bip/src/policy.rs`) | BIP 379 (Draft); BIP 382, 386 (Deployed) | Reads any sane `wsh`, `sh(wsh)` or `tr` miniscript, as Core does; Create does not make one; the compiler is in Tools; the builder of Liana's shapes is in core and not reached from the app; a taproot leaf with a `pkh` is signed but left for the coordinator to finalize | vectors `tools/vectors/miniscript`; F79 area `miniscript` against Bitcoin Core v31.1's `bitcoind` on regtest; Core end to end for the `wsh` timelock and Taproot tree wallets (INTEROP §1); Liana's fixtures (`tools/vectors/psbt/README.md`); 5 tests (faraday-bip); fuzz `policy` | Script shape is left to rust-miniscript's compiler. Neutral; no decision. |
| MuSig2 `tr(musig(...)/<0;1>/*)` | BIP 327, 341 (Deployed); BIP 328, 373 (Complete); BIP 390 (Draft, vendored) | Key path only; script-path spends refused; no WIF participant or tap tree (`tools/vectors/bip390/README.md`) | vectors `tools/vectors/bip327`, `tools/vectors/bip328`, `tools/vectors/bip390`, `tools/vectors/psbt`; F79 area `musig` against BIP-327's `reference.py`; Core 31.1 round trip in both orders (`tools/vectors/psbt`, `wallet-musig-*`); 19 tests (faraday-bip, faraday-psbt); fuzz `policy` | Script-path spends of a musig() wallet are refused; only the key path is built. Conservative; F155. Each nonce's rand' is a tagged hash of the session's seed, the input's index and the participant's key: a fresh value per call from the session's seed chain. Neutral; no decision. The secret nonce is held in memory for its one session and never serialised. Conservative; F109. |
| Silent payments, receive (`core/faraday-bip/src/silent.rs`, `core/faraday-bip/src/silent_wallet.rs`) | BIP 321, 352, 353 (Complete, vendored); BIP 392 (Draft, vendored) | Receive, labels, scan-key export; reading `sp()` open (INTEROP §5 I9); scanning stops at K_max; sender arithmetic is test-only | vectors `tools/vectors/bip352`, `tools/vectors/bip392`; F79 area `sp` against BIP-352's `reference.py`; 7 tests (faraday-bip, faraday-keys, faraday-core) | The faraday-silent 1 record holds the kind, the network, the key's origin, the label count and the address. Neutral; no decision. |
| Silent payments, send and PSBT (`core/faraday-psbt/src/silent.rs`, `core/faraday-bip/src/dleq.rs`) | BIP 374, 375 (Draft); BIP 352 (Complete, vendored) | Built in core, not reachable from the app: the device does not send | vectors `tools/vectors/bip374`, `tools/vectors/bip375`, `tools/vectors/bip352`; F79 area `sps` against BIP-374's `reference.py`; 6 tests (faraday-bip, faraday-psbt) | A key a PSBT states for an input counts only where the input's script commits to it, which is stricter than BIP 375's reference validator. Conservative; F89. Spends of SegWit above v1 and sighashes other than ALL are refused. Conservative; no decision. |
| Watch-only wallets | BIP 44, 49, 84, 86, 173, 341, 350, 380 to 383, 386 (Deployed); BIP 388 (Complete); BIP 379, 389 (Draft) | Full: a wallet read from its descriptor or an account's extended public key, with no key loaded | vectors `tools/vectors/addresses`, `tools/vectors/bip387`, `tools/vectors/miniscript`, `tools/vectors/bip32`; F79 area `miniscript` against Bitcoin Core v31.1's `bitcoind` on regtest; F79 area `addr` against Bitcoin Core's functional-test framework; Core's checksums (INTEROP §1); Bitcoin Core end to end over every test wallet (`faraday/tools/core-check.py`, INTEROP §1, Core 31.1); 18 tests (faraday-bip, faraday-core); self-test `bip84_address`; fuzz `policy` | Only BIP 341's H, or an extended key over H, counts as an unspendable internal key. Conservative; F132. One text is written, with h for hardened. Neutral; F66. |
<!-- crypto-manifest: end §2 -->

## 3. Keys and backups

<!-- crypto-manifest: begin §3 -->
| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| BIP-39 (`core/faraday-bip/src/bip39.rs`, written here) | BIP 39 (Deployed) | Ten word lists, every length. A passphrase is printable ASCII, at most 256 bytes | vectors `tools/vectors/bip39`; F79 area `bip39` against python-mnemonic; the `bip39` crate as a dev-dependency (`core/faraday-bip/tests/all/crosscheck.rs`); 13 tests (faraday-bip, faraday-keys); self-test `bip39_vector`, `bip39_japanese_vector`; fuzz `bip39_words` | Word entry by readings and kana: typed readings find the listed words. Neutral; no decision. A passphrase is printable ASCII, at most 256 bytes. BIP 39 allows any NFKD text, so a wallet made elsewhere with another passphrase cannot be restored here; a letter outside the range typed into a passphrase field is refused. Conservative; F155. |
| Keys held in memory and the words that show them (`core/faraday-crypto`, `faraday/faraday-keys`) | BIP 39 (Deployed) | Blinded, pinned and sealed in the session; shown only through a named reason | 13 tests (faraday-core, faraday-crypto, faraday-keys); not cross-checked: nothing outside the program reads these values; the sealing is checked by the program's own tests; the reasons are Faraday's own list; no other implementation has one; a comparison inside the program; there is no second implementation to compare it with | The in-memory seal's nonce is a counter with a prefix derived from the key, because the crate has no random number generator; no shipped code reaches it, so no secret is sealed in memory today. Novel; no decision. A key's signing context is blinded from the session's randomness. Neutral; no decision. The list of reasons is closed: a new reason is a new variant, seen in review. Conservative; F70. The id is all 32 bytes of a hash, so two seeds never pass for one. Conservative; no decision. |
| Dice (`core/faraday-entropy/src/dice.rs`) | Dice hashing of Coldcard and SeedSigner (Convention). Coldcard's `rolls.py`, SeedSigner, Keystone and BitBox; sources pinned in `tools/reference/dice/README.md`. | Three procedures; 99 rolls for 256 bits, as the others accept | vectors `tools/vectors/entropy`; F79 area `entropy` against the procedures written out in `area_entropy.py` over `hashlib` (partly Faraday's own model, not an independent check); 6 tests (faraday-entropy, faraday-core) | Cautions about runs and skew are warnings only. Neutral; no decision. 99 rolls give 256 bits, as the other tools accept. Neutral; no decision. |
| Coins, hex | None: a convention, most significant bit first | Full | vectors `tools/vectors/entropy`; F79 area `entropy` against the procedures written out in `area_entropy.py` over `hashlib` (Faraday's own model, not an independent check); 3 tests (faraday-entropy, faraday-core) | Coins and hex are packed or taken as typed, with no hashing. Neutral; no decision. |
| Playing cards (`core/faraday-entropy/src/cards.rs`) | None: SHA-256 of one-byte draw indices | Draws tracked against one deck, or two for 24 words | vectors `tools/vectors/cards`, `tools/vectors/entropy`; F79 area `entropy` against the procedures written out in `area_entropy.py` over `hashlib` (Faraday's own model, not an independent check); 6 tests (faraday-entropy, faraday-core) | The procedure is Faraday's own: a card is the index suit × 13 + rank, and the entropy is SHA-256 of those one-byte indices; no other wallet's card mode is claimed to match, and the bits counted assume a well-shuffled deck. Novel; F158. |
| Camera noise (`core/faraday-entropy/src/camera.rs`) | None: SHA-256 of the frames' SHA-256 digests; 3 frames per 128 bits is a heuristic | Frames hashed in order; 3 frames per 128 bits; a mix's member or added to the session's randomness, never a key's only source | vectors `tools/vectors/camera`, `tools/vectors/entropy`; F79 area `entropy` against the procedures written out in `area_entropy.py` over `hashlib` (Faraday's own model, not an independent check); 4 tests (faraday-entropy, faraday-core); fuzz `v4l2_luma` | The procedure is Faraday's own, and three frames per 128 bits is a heuristic; the frames only add to a mix or to the system's randomness, never make a key alone. Novel; F158. |
| Mix of sources (`core/faraday-entropy/src/mix.rs`) | None: SHA-256 over the concatenated commitments | Two to five sources, dice or this device among them | vectors `tools/vectors/entropy`, `tools/vectors/mix`; F79 area `entropy` against the procedures written out in `area_entropy.py` over `hashlib` (Faraday's own model, not an independent check); 3 tests (faraday-entropy, faraday-core) | The construction is Faraday's own: SHA-256 over the concatenated commitments. With dice or this device among the sources, each at its full count, it only adds to that member's strength. Novel; F158. |
| This device's generator as a key's source (`core/faraday-entropy`) | None: SHA-256 of the shell's 32 bytes | One 32-byte answer of the shell, hashed | vectors `tools/vectors/entropy`; F79 area `entropy` against the procedures written out in `area_entropy.py` over `hashlib` (Faraday's own model, not an independent check); 2 tests (faraday-entropy, faraday-core) | The source trusts the device's generator. Neutral; no decision. |
| Own randomness in the session (`faraday/faraday-core/src/fresh.rs`) | None: `docs/OWN-RANDOMNESS.md` §2.3 | R1 built; R2 and R3 open | vectors `tools/vectors/own-randomness`; F79 area `own-randomness` against `docs/OWN-RANDOMNESS.md` §2.1 and §2.3 written out in `area_own.py` over `hashlib` (Faraday's own model, not an independent check); 13 tests (faraday-entropy, faraday-core); not cross-checked: the bytes are the operating system's; there is nothing to compare them with | Pointer and key timings are added to the session's randomness, never made into a key on their own. Novel; F86. The stick and the Pi read /dev/random; the Pi's generator is mixed, not credited. Conservative; F102. The stick's kernel credits neither the CPU's RDRAND nor the firmware's seed; both are still mixed in. Conservative; F86. Added randomness never replaces the system's bytes. Conservative; F86. The mixing construction is Faraday's own. Novel; F86. MuSig2's rand' is taken from the session seed chain; rounds across a lock are not offered. Neutral; F109. |
| Booklet dice for codex32 (`core/faraday-entropy/src/booklet.rs`) | Booklet dice de-biasing (Convention). The codex32 booklet SSS32.ps of 2023-03-07 (`tools/reference/booklet-dice/README.md`) | Six-faced dice in the booklet; dice of 2 to 20 faces are read by the same comparison | F79 area `roller` against `docs/CODEX32.md` §5.6's "The values" and §5.5's worksheet written out in `area_roller.py` (Faraday's own model, not an independent check); 4 tests (faraday-entropy) | The comparison is the booklet's; reading dice of 2 to 20 faces is an addition. Neutral; no decision. |
| Faraday Demo dice roller (`faraday/faraday-core/src/fresh/roller`) | None | Faraday Demo only, labelled "simulated dice" | vectors `tools/vectors/roller`; F79 area `roller` against `docs/CODEX32.md` §5.6's "The values" and §5.5's worksheet written out in `area_roller.py` (Faraday's own model, not an independent check); 6 tests (faraday-core) | The roller is an exception to Faraday's randomness never making a codex32 seed; it is testnet-only. Neutral; F96. |
| SLIP-39 (`core/faraday-bip/src/slip39.rs`, written here) | SLIP-0039 (Status not checked, vendored) | Secrets of 16 to 32 bytes; exponent at most 5 on read; writes exponent 1 with the extendable flag set | vectors `tools/vectors/slip39`; F79 area `slip39` against python-shamir-mnemonic; 6 tests (faraday-bip); fuzz `seed_parts` | Shares with an iteration exponent above 5 are refused; a split writes exponent 1 with the extendable flag set. Conservative; F90. A split with a group threshold of 1 over several groups is refused. Conservative; F90. Secrets above 256 bits are refused. Conservative; F155. The 1-of-1 identifier is taken from the master fingerprint, which puts 15 bits of it on every share. Neutral; no decision. The split's randomness is taken from the session's randomness, an answer for every 32 bytes; SLIP-0039 does not say where a split's randomness comes from. Neutral; F154. |
| codex32 (`core/faraday-bip/src/codex32/`) | BIP 93 (Draft, vendored) | Codec, split, derive, recover, worksheets and the correction decoder built; the flows in `docs/CODEX32.md` marked open are not | vectors `tools/vectors/bip93`, `tools/vectors/codex32-booklet`, `tools/vectors/codex32-volvelle`, `tools/vectors/codex32-correction`; F79 area `codex32` against python-codex32 (partly Faraday's own model, not an independent check); F79 area `codex32-correct` against python-codex32's decoder, `correction.py` at a later commit; 19 tests (faraday-bip, faraday-keys); fuzz `seed_parts` | Recovery is from exactly k shares only. Conservative; F90. The set's identifier is the first 20 bits of the master fingerprint, which puts 20 bits of it on every share. Neutral; no decision. A further share is derived only from exactly k strings at an index not in use. Conservative; F90. Correction is opt-in, off whenever the form opens, proposes only where the correction is unique, and is never applied: the person types the changed boxes again. Conservative; F94. A character outside the bech32 alphabet is not read as an erasure, and more than 8 erasures not in one run are refused; BIP 93 makes the first optional, and python-codex32 accepts both. Conservative; F155. |
| Codex32QR/v1-48 (`tools/vectors/codex32qr/README.md`) | Codex32QR/v1-48 (Convention, vendored); BIP 93 (Draft, vendored). The profile is from FractalEncrypt's SeedSigner fork, pinned in that README; new and not peer-reviewed. The full-string QR is the form the codex32 authors' discussion 66 ("CodexQR") describes, where the format is being settled; no compact CodexQR until it is. Faraday follows the profile for the QR, in alphanumeric mode as its pictures are, and not the fork's device workflow (owner, 2026-10-24). Faraday's own, fails closed (F95) | 48 characters only, by rule | vectors `tools/vectors/codex32qr`; F79 area `seedqr` against SeedSigner's own encoders and `DecodeQR`; 2 tests (faraday-codec, faraday-bip) | No code is drawn for a 74 or 127 character string, and none is read but as plain text, until the profile's author publishes a profile for them. Conservative; F95. |
| Seed XOR (`core/faraday-entropy/src/xor.rs`) | Seed XOR (Convention, vendored); BIP 39 (Deployed) | 2 to 4 parts | vectors `tools/vectors/xor`; F79 area `xor` against Coldcard's own `xor` and `xor_split` (partly Faraday's own model, not an independent check); 2 tests (faraday-entropy) | None |
| SeedQR and CompactSeedQR (`core/faraday-codec/src/seedqr.rs`) | SeedQR (Convention, vendored); BIP 39 (Deployed) | 12 and 24 words; English only, as the specification assumes | vectors `tools/vectors/seedqr`; F79 area `seedqr` against SeedSigner's own encoders and `DecodeQR` (partly Faraday's own model, not an independent check); 8 tests (faraday-codec, faraday-core); fuzz `seedqr` | The picture is a copy only when it is checked by a scan or typed words. Neutral; F29. A copy by hand is checked by its form: typed words, or a scan of a SeedQR. Neutral; F29. |
| BIP 85 (`core/faraday-bip/src/bip85.rs`) | BIP 85 (Deployed, vendored); RFC 1924, 4648 (Status not checked) | Applications 2', 32', 39', 128169', 707764', 707785'; no DICE, RSA or Nostr 128002' (F27) | vectors `tools/vectors/bip85`; F79 area `bip85` against btclib-wallet's `bip85`; 8 tests (faraday-bip, faraday-keys) | Nostr keys by application 128002' are not built. Neutral; F27. |
| Electrum seeds | Electrum's own scheme | Not built (INTEROP §5 I7) | None | None |
| aezeed (`core/faraday-bip/src/aezeed.rs`, `core/faraday-bip/src/aez.rs`, written here) | LND aezeed (Convention); RFC 7914 (Status not checked). AEZ v5. | Decode, for node keys and conversion | vectors `tools/vectors/aezeed`; F79 area `lightning` against LND's own code, vendored in Go; AEZ's reference vectors (`tools/reference/aezeed/aez-testdata`); LND's cipher seed and node id (`faraday/faraday-core/tests/all/lightning.rs`); 6 tests (faraday-bip); fuzz `aezeed` | aezeed and AEZ are written here; the cross-check asks LND's own Go aezeed package, over Yawning Angel's AEZ, the same words and passphrases and requires the same answers or refusals. Neutral; no decision. |
| Paper backups, the backup plan and its sticks | None | Partly built (`docs/BACKUP-FLOW.md` Open) | Own tests | Places, presets, warnings, "seeds alone spend" (F31, F35, F37, F87): Neutral. Partial descriptions (F52) and the Coldcard config carrying `Format: P2TR`: Neutral, an interoperability risk. |
| Seed files for a backup stick and for the Outbox (`faraday/faraday-core`) | BIP 39 (Deployed); SeedQR (Convention, vendored); BIP 93 (Draft, vendored) | Words, passphrase, SeedQR and codex32 files for a stick; words and SeedQR for the Outbox | 11 tests (faraday-core); not cross-checked: the files are text lines and pictures made by seedqr-make and the codex32 actions, which have their own checks; no second implementation reads the files this writes; the files are one line of words and a picture made by seedqr-make; no second implementation reads what this writes | A secret goes to a stick only for the backup-stick reason, and only the part that reason names. Conservative; no decision. Nothing is written to the Outbox until the hold on the secret sheet is held to its end. Conservative; no decision. |
| The vault (`faraday/faraday-vault`, `docs/VAULT.md`) | RFC 5869, 9106 (Informational); draft-irtf-cfrg-xchacha-03 (Draft); RFC 8439 (Status not checked); UAX #15 (Complete); BIP 32, 39 (Deployed). Faraday's own container over Argon2id 1.3, HKDF-SHA256 assembled by hand, and XChaCha20-Poly1305. | Built: create, open, seal, records | vectors `tools/vectors/vault`, `tools/vectors/vault-primitives`, `tools/vectors/kdf`, `tools/vectors/bip39`, `tools/vectors/bip32`; F79 area `vault` against `docs/VAULT.md` written out in `area_vault.py` (partly Faraday's own model, not an independent check); F79 area `bip39` against python-mnemonic; a Python reader Faraday's authors wrote from `docs/VAULT.md`, over outside primitives (`tools/vault/open.py`): not an independent check of the container; 31 tests (faraday-vault, faraday-keys, faraday-core); fuzz `vault_header`, `vault_records`; not cross-checked: it computes no value of a standard; it hands back stored text and a parse of stored words | Faraday's own container with four equal slots, so a file does not say how many slots are in use (docs/VAULT.md §9); unused slots are keystream, one salt serves every slot, and an older copy of a slot still opens. Kept until the outside review. Novel; F158. The reader limits (lanes exactly 1, passes at most 10, memory 8 MiB to 4 GiB) refuse headers that would cost a guess or an allocation the vault does not state. Conservative; F155. Every passphrase is put in Unicode NFC before Argon2id, and nothing else is done to it: compatibility characters, case and spaces are kept as typed (docs/VAULT.md §3). Neutral; F159. Every slot's plaintext begins with 32 zero bytes, the padding fix for key commitment, so a slot that verifies under two keys shows them under at most one (docs/VAULT.md §4, §7). Conservative; F159. Four tag checks on every open, so timing does not show which slot a passphrase opens. Conservative; no decision. A slot whose tag verifies and whose commitment block is not zero refuses the whole file, as two slots opening under one key do: this format never writes one (docs/VAULT.md §4). Conservative; F159. The seal nonce is a fresh system answer when the pool holds one, and the session's seed with a count of seals only when none waits, so a lock never waits on the system (docs/VAULT.md §5). No two seals of one session share a nonce. Neutral; F158. Each slot counts its seals (record 16): the count and the time by the sealing computer's clock, inside the slot and read by whoever opens it, so an older copy can be told from a newer one (docs/VAULT.md §5, §7). Neutral; F159. The list of reasons is closed: a new reason is a new variant, seen in review. Conservative; F70. Faraday's own record layouts (docs/VAULT.md §7); a reader Faraday's authors wrote from that text, not from the code, reads them, which is not an independent check. Novel; F97. Which passphrase goes with which words is decided by the vault's record, not by a guess; a mismatched fingerprint is not loaded. Conservative; no decision. |
| KDBX 4 export (`core/faraday-backup/src/kdbx.rs`) | KeePass KDBX 4 (Convention); RFC 9106 (Informational); RFC 8439 (Status not checked). KeePass's pages are pinned in `tools/reference/kdbx4`. | Write only: 4.0, ChaCha20, Argon2id | vectors `tools/vectors/backup`, `tools/vectors/kdf`; F79 area `kdbx` against pykeepass, the distribution's package; every file read back by `tools/backup/read-kdbx.py`; 6 tests (faraday-backup, faraday-core) | Nostr entries are never exported (F27). Neutral; F27. The file's seed, IV, salt, inner key and UUIDs are taken from the session's randomness, an answer for every 32 bytes. Neutral; F154. |
| Diceware passphrases | EFF word lists (Convention, vendored) | Three lists | vectors `tools/vectors/eff`; 3 tests (faraday-bip); not cross-checked: the lists are checked by their digests against the committed EFF files | None |
<!-- crypto-manifest: end §3 -->

## 4. Signing and transfer

<!-- crypto-manifest: begin §4 -->
| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| PSBT v0 (`core/faraday-psbt`) | BIP 174 (Deployed) | Signs every wallet kind above; the finalizer is written here | vectors `tools/vectors/psbt`, `tools/vectors/psbt-bips`; F79 area `psbt` against Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (partly Faraday's own model, not an independent check); F79 area `sign` against Bitcoin Core v31.1 on regtest (partly Faraday's own model, not an independent check); Core end to end (INTEROP §1); 12 tests (faraday-psbt); fuzz `psbt_parse` | A key origin of more than 16 steps anywhere in a PSBT is refused when it is parsed. Conservative; F155. Of several satisfactions of a wsh miniscript, the one with the smaller real signature size is chosen; where two weigh the same, the one Bitcoin Core keeps: an or's first branch, a multi()'s keys named first, a thresh()'s branches named last, a taproot spend's leaf whose script sorts first by its bytes. Neutral; F161. |
| PSBT v2 (`core/faraday-psbt/src/v2.rs`) | BIP 370 (Deployed) | Read, checked and converted to version 0 in core; the silent payments code uses it; the app reads version 0 only (INTEROP §5 I10) | vectors `tools/vectors/psbt-bips`; F79 area `psbt` against Bitcoin Core v31.1's `bitcoind` on regtest for version 0 (partly Faraday's own model, not an independent check); 4 tests (faraday-psbt) | The reader refuses what BIP 370 marks invalid and keeps every pair as read. Conservative; no decision. |
| Sighash | BIP 143, 341, 342 (Deployed) | All computed (rust-bitcoin); only ALL and DEFAULT are signed; others are review cards (`core/faraday-psbt/src/inspect/sighash.rs`) | vectors `tools/vectors/sighash`; F79 area `tx` against Core's `messages.py` and `script.py`; Core end to end (INTEROP §1); 4 tests (faraday-psbt); fuzz `psbt_parse` | NONE, SINGLE and non-standard sighashes are refused for signing; ALL with ANYONECANPAY is signed after a caution. Conservative; F136. |
| ECDSA nonces | BIP 143, 174, 341 (Deployed); RFC 6979 (Informational). Core's low-R grind. | Low-R RFC 6979 in the app; the library also offers the first RFC 6979 nonce, which the app does not use (`core/faraday-psbt/src/sign.rs`) | vectors `tools/vectors/psbt`; F79 area `sign` against Bitcoin Core v31.1 on regtest (partly Faraday's own model, not an independent check); 13 tests (faraday-psbt); self-test `ecdsa_vector`; fuzz `psbt_parse`; not cross-checked: area sign compares the signatures Faraday makes; the checks of signatures already on a PSBT have no second implementation | A key comes from the PSBT's origin only, under the master key's fingerprint; a key whose derived public key differs from the one named, or that the script does not hold, is refused, and an origin under an account's fingerprint matches no key. Conservative; F155. A script of several keys is signed only for a registered wallet that holds a loaded key. Conservative; F135. Only a high fee and an absurd fee rate can be held past; every other danger refuses. Conservative; F136. Each signature is made twice and compared, and verified before it is written. Neutral; no decision. Low-R grinding is the default nonce. Neutral; no decision. The nonce check is run on signatures a device writes, comparing them with the RFC 6979 or BIP 340 nonce rule. Neutral; no decision. |
| Schnorr | BIP 340 to 342 (Deployed) | Zero `aux_rand` in the app; the library also offers fresh `aux_rand`, which the app does not use | vectors `tools/vectors/psbt`; F79 area `sign` against Bitcoin Core v31.1 on regtest (partly Faraday's own model, not an independent check); 6 tests (faraday-psbt); self-test `schnorr_vector` | aux_rand is zero, so a signature can be made again byte for byte; BIP 340 recommends fresh randomness against fault attacks. The library offers fresh aux_rand, and the app does not use it. Neutral; F156. A miniscript or a taproot tree of one key is signed only through its registered wallet. Conservative; F141. |
| Address and change checks | BIP 173, 174, 350 (Deployed). Faraday's review rules. | Change verified by deriving it again; the first 100 receive and change addresses searched for a scanned address (`docs/QR.md` §1) | vectors `tools/vectors/psbt`; F79 area `psbt-review` against Bitcoin Core v31.1's `bitcoind` on regtest, offline (partly Faraday's own model, not an independent check); Core end to end (INTEROP §1); 9 tests (faraday-psbt, faraday-core); fuzz `psbt_parse` | Change under an unloaded key, or at a path under no loaded account, counts as a payment to others. Conservative; F136. A script of several keys is signed only for a registered wallet that holds a loaded key. Conservative; F135. A miniscript or a taproot tree of one key is signed only through its registered wallet. Conservative; F141. A SegWit v0 input with only a witness UTXO in a transaction of several inputs is refused, which BIP 174 allows. Conservative; F155. The signed-amount memory refuses one transaction stated again with other amounts. Conservative; F155. |
| Check a signer | None | Not built (`docs/SIGNER-CHECK.md`) | None | Fails closed by design (F84). Conservative. |
| Anti-exfil | libsecp256k1-zkp `ecdsa_s2c`; FractalEncrypt's four-stage exchange over QR (alpha, unpublished) | Not built; planned in dev builds only (`docs/ANTI-EXFIL.md`, F162) | None | None |
| Message signing (`core/faraday-psbt/src/message.rs`) | BIP 137, 340 (Deployed); BIP 322 (Complete); RFC 6979 (Informational) | BIP 322 "simple" only: no "full" form, no proof of funds, no nested SegWit; BIP 137 header ranges as Electrum and Trezor use | vectors `tools/vectors/message`; F79 area `msg` against Bitcoin Core v31.1 and btclib for BIP-137, btclib-wallet and btcd for BIP-322; 7 tests (faraday-psbt); fuzz `message_verify` | The app signs with the low-R nonce and zero aux_rand for a taproot address. Neutral; F156. The address form is read from the address, not from the BIP 137 header's range, and a P2PKH address is tried in both key encodings; more lenient than BIP 137. Neutral; no decision. |
| QR codes (`core/faraday-codec/src/qr.rs`) | ISO/IEC 18004 (Status not checked) | Versions 1 to 40, the four error-correction levels, numeric, alphanumeric and byte modes; reading is rqrr's | vectors `tools/vectors/seedqr`; F79 area `seedqr` against SeedSigner's own encoders and `DecodeQR`; 6 tests (faraday-codec); fuzz `decode_luma`, `classify`, `png_qr` | The error-correction level is never boosted, so a SeedQR comes out at the level its specification names. Neutral; no decision. |
| UR (`core/faraday-codec/src/ur/`) | BCR-2020-005, BCR-2020-006, BCR-2020-012 (Convention, vendored); BCR-2020-007, BCR-2020-010, BCR-2020-015 (Convention) | Legacy `crypto-*` types and fountain codes; `crypto-output` is read, never written | vectors `tools/vectors/ur`; F79 area `ur` against Foundation Devices' foundation-ur-py; SeedSigner's and Keystone's codes (INTEROP §2); 8 tests (faraday-codec); fuzz `ur_receive`, `ur_registry` | Every part is written as ur:<type>/<seq>-<count>/..., a message of one fragment included. Neutral; no decision. There is no minimum fragment length; a fragment is ceil(length / ceil(length / max)) bytes. Neutral; no decision. A part of another message, and an empty message to animate, are refused. Conservative; no decision. |
| BBQr and Specter `pNofM` (`faraday/faraday-qr/src/bbqr.rs`, `faraday/faraday-qr/src/specter.rs`) | BBQr, Specter pNofM (Convention) | BBQr is read in base32, hex and zlib and written in base32 only; Specter's numbered parts are read and written | F79 area `bbqr` against Coinkite's BBQr library (partly Faraday's own model, not an independent check); the numbered parts against SeedSigner's own decoder (`tools/reference/seedsigner/README.md`, `tools/reference/specter/README.md`); 4 tests (faraday-qr); fuzz `qr_assembler` | Every limit fails closed: a frame, a count or a size past it is refused. Conservative; F155. |
| File envelope `faraday-file-v1` (`faraday/faraday-qr`) | None: Faraday OS's own format | Full | 2 tests (faraday-qr); fuzz `qr_assembler`; not cross-checked: Faraday OS's own format; the tests read what Faraday OS writes, and the cross-check has no second implementation | The SHA-256 shows that the file arrived whole, not who sent it. Neutral; no decision. An envelope is refused unless it is exactly what pack would write for its file. Conservative; no decision. |
| File hashes | FIPS 180-4 (Status not checked) | The SHA-256 of a whole file is shown and kept | 1 test (faraday-crypto); not cross-checked: SHA-256 is sha2's; the known answers are the check | The digest is shown as the file's integrity, not as proof of who made it. Neutral; no decision. |
| Seeds and keys found in files and scans | BIP 32, 39 (Deployed); BIP 93 (Draft, vendored); SLIP-0039 (Status not checked, vendored) | Words, SeedQR, codex32, SLIP-39, extended private keys and WIF recognised | 7 tests (faraday-core); not cross-checked: the key a seed makes is checked by the bip39, bip32, codex32 and slip39 areas; the recognising of files and pasted text has no second implementation | A secret pasted or scanned where a secret may not go is refused, whatever its form. Conservative; no decision. |
| Descriptors and checksums | BIP 380 to 383, 386 (Deployed); BIP 388 (Complete); BIP 379, 389 (Draft) | Reads and writes the descriptors of §2; one text, with `h` | vectors `tools/vectors/addresses`, `tools/vectors/bip387`, `tools/vectors/miniscript`; F79 area `miniscript` against Bitcoin Core v31.1's `bitcoind` on regtest; Core's checksums (INTEROP §1); 9 tests (faraday-bip); fuzz `policy` | Only BIP 341's H, or an extended key over H, counts as an unspendable internal key. Conservative; F132. One text is written, with h for hardened. Neutral; F66. |
| BSMS (`core/faraday-bip/src/bsms.rs`) | BIP 129 (Complete, vendored); BIP 137 (Deployed) | NO_ENCRYPTION records only (INTEROP §5 I3); signer records' signatures checked | vectors `tools/vectors/bip129`; F79 area `import` against Bitcoin Core v31.1's `bitcoind` on regtest, offline (partly Faraday's own model, not an independent check); 7 tests (faraday-bip) | Upper-case bech32 and No path restrictions are accepted. Neutral; no decision. A descriptor record whose paths line is other than /0/*,/1/* is refused. Conservative; no decision. |
| Other wallets' files | BIP 388 (Complete); RFC 8259 (Status not checked). Vendor formats. | As listed in INTEROP §2 | F79 area `import` against Bitcoin Core v31.1's `bitcoind` on regtest, offline (partly Faraday's own model, not an independent check); 10 tests (faraday-bip, faraday-core); fuzz `json`, `read_wallet` | A SLIP-132 version is checked against the script and the path around the key. Conservative; F155. A Caravan file whose startingAddressIndex is not 0, and an Electrum multisig keystore with no fingerprint, are refused. Conservative; F155. |
<!-- crypto-manifest: end §4 -->

## 5. Tools

<!-- crypto-manifest: begin §5 -->
| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| Explore a key | BIP 93 (Draft, vendored); BIP 32 (Deployed) | Full | vectors `tools/vectors/bip93`, `tools/vectors/bip32`; F79 area `codex32` against python-codex32; F79 area `bip32` against btclib-wallet's `bip32` over btclib and `ecc`; 9 tests (faraday-keys, faraday-core) | None |
| Vanity address (`core/faraday-bip/src/vanity.rs`) | EntropyLab vanity counter (Convention) | Passphrase dial or account dial | F79 area `vanity` against EntropyLab's counter as `tools/reference/vanity/README.md` quotes it (partly Faraday's own model, not an independent check); 6 tests (faraday-bip, faraday-core) | A passphrase found by grinding becomes key material to back up; the account dial gives a non-zero account. Neutral; no decision. |
| Decode a transaction | BIP 141, 144 (Deployed) | Full (rust-bitcoin) | F79 area `tx` against Core's `messages.py` and `script.py`; 3 tests (faraday-psbt); fuzz `transaction` | A previous transaction is attached to an input only when its id is the spent one. Conservative; no decision. |
| Lightning node key (`faraday/faraday-keys/src/lightning.rs`) | LND aezeed, ldk-node node key (Convention); BIP 32, 39 (Deployed) | ldk-node from BIP-39, LND from aezeed | vectors `tools/vectors/aezeed`; F79 area `lightning` against LND's own code, vendored in Go (partly Faraday's own model, not an independent check); five ldk-node vectors computed with `lightning` 0.2.6 (`tools/reference/aezeed/README.md` §6); LND's own node id; 4 tests (faraday-core, faraday-bip); fuzz `aezeed` | None |
| TOTP (`faraday/faraday-vault/src/totp.rs`) | RFC 4226, 6238 (Informational); otpauth Key URI Format (Convention) | No HOTP, no migration QR | 5 tests (faraday-vault, faraday-core); fuzz `vault_entries`; not cross-checked: RFC 6238 and RFC 4226 publish vectors that cover the algorithm in full (STANDARDS section 0) | A manual clock with a fixed-offset time zone, since a device with no network has no other time (F7). Neutral; F7. |
| GPG (`faraday/faraday-pgp`) | RFC 4880, 9580 (Proposed Standard); RFC 8032 (Informational); paperkey (Convention). Version 4 packets; GnuPG behaviour (F32) | One key shape: Ed25519 certify plus a sign subkey | vectors `tools/vectors/paperkey`, `tools/vectors/release-sums`; Real `gpg` 2.4 end to end: it imports the certificate, checks its signatures and revocation and takes its renewal, with its clock set (`faraday/faraday-pgp/tests/gnupg.rs`, `faraday/faraday-keys/tests/vault.rs`); GnuPG's authors, outside Faraday; required, nothing skips it (docs/DEVELOPING.md); Real `paperkey` 1.6 reads the key's secret to the same lines (`faraday/faraday-pgp/tests/gnupg.rs`); paperkey's author, outside Faraday; Real `gpg` 2.4 verifies Faraday's signatures, including those whose r or s starts with a zero byte (`faraday/faraday-pgp/tests/gnupg.rs`); GnuPG's authors, outside Faraday; required, nothing skips it; Real `gpg` 2.4 takes the revocation and lists the key as revoked, and refuses a `.rev` file until its colon is removed (`faraday/faraday-pgp/tests/gnupg.rs`); GnuPG's authors, outside Faraday; required, nothing skips it; Real `paperkey` 1.6 prints the same lines and reads Faraday's back (`faraday/faraday-pgp/tests/gnupg.rs`, `paperkey.rs`); paperkey's author, outside Faraday; required, nothing skips it; Signatures made by real `gpg` 2.4 over SHA-512 and SHA-256 verify (`faraday/faraday-pgp/tests/verify.rs`, `tools/vectors/release-sums`); GnuPG's authors, outside Faraday; fixtures committed, so this check needs no tool at run time; 16 tests (faraday-pgp, faraday-keys); fuzz `pgp_armor`, `pgp_paperkey`, `pgp_verify`; not cross-checked: judged by real gnupg and paperkey in faraday/faraday-pgp/tests/gnupg.rs, not by an F79 area; those tests fail where the tools are missing; judged by real gnupg in faraday/faraday-pgp/tests/gnupg.rs, not by an F79 area; the test fails where gnupg is missing; judged by real gnupg in faraday/faraday-pgp/tests/gnupg.rs, not by an F79 area; the tests fail where gnupg is missing; judged by real paperkey 1.6 in faraday/faraday-pgp/tests/gnupg.rs, not by an F79 area; the tests fail where paperkey is missing; judged by signatures made with real gnupg (tools/vectors/release-sums), not by an F79 area | Signing waits until the backup flow has written every place the key's plan gives it, two or more; GnuPG has no such gate. Conservative; F32. The primary key's flags are certify only and the signature hash is SHA-512, as GnuPG 2.4 chooses for Ed25519. Neutral; F32. Signing takes a hold (F63) and shows the files' names, sizes and SHA-256 first. Conservative; F32. The revocation is kept in the key's vault record and nothing is written to the Outbox at key generation. Neutral; F32. The paperkey is computed only while it is held to show, and wiped after. Conservative; F32. Anything not honoured is refused rather than read, so the release check of F134 cannot be satisfied by a signature of another shape. Conservative; F134. |
| GPG paperkey restore | paperkey (Convention); RFC 4880, 9580 (Proposed Standard); RFC 8032 (Informational) | Built in faraday-pgp and faraday-keys; no screen offers it yet (`docs/GPG-PAPERKEY.md`) | vectors `tools/vectors/paperkey`; Real `paperkey` 1.6 restores Faraday's lines to the key and reads its own output as Faraday's lines (`faraday/faraday-pgp/tests/paperkey.rs`); paperkey's author, outside Faraday; required, nothing skips it; 5 tests (faraday-pgp, faraday-keys); fuzz `pgp_restore`, `pgp_paperkey`; not cross-checked: judged by real paperkey 1.6 in faraday/faraday-pgp/tests/paperkey.rs, not by an F79 area; the tests fail where paperkey is missing | A restore that cannot reproduce the certificate's own signatures is refused rather than accepted on the public key alone. Conservative; F155. |
| Secure Boot keys (`faraday/faraday-sb`) | UEFI Specification, Authenticode (Status not checked); RFC 5280 (Proposed Standard); RFC 5652 (Internet Standard); RFC 8017 (Informational) | PK, KEK and db RSA-2048, `.auth` files, signing a release's boot program, checking a signed file | vectors `tools/vectors/release-sums`; Real `openssl` reads the certificates and verifies the `.auth` PKCS #7 signatures and every private key made (`faraday/faraday-sb/tests/judged.rs`); OpenSSL's authors, outside Faraday; required, nothing skips it (docs/DEVELOPING.md); Real `sbverify` (sbsigntools, by its authors, outside Faraday) verifies a signed image against the db certificate and refuses an altered one, and `openssl` (OpenSSL's authors) checks the PKCS #7 signature (`faraday/faraday-sb/tests/judged.rs`); required, nothing skips it; Images signed by `sbsign` (sbsigntools) and `osslsigncode` (its author) are read and their signer named, and `sbverify` agrees with Faraday on a signed and an altered image (`faraday/faraday-sb/tests/judged.rs`); outside Faraday; the signed fixtures are committed, `sbverify` is required; 12 tests (faraday-sb); fuzz `release_sums`, `pe_image`; not cross-checked: judged by openssl in faraday/faraday-sb/tests/judged.rs, which fails where openssl is missing; not by an F79 area; judged by sbverify and openssl in faraday/faraday-sb/tests/judged.rs, which fails where one is missing; not by an F79 area; judged by images signed by sbsign and osslsigncode and by sbverify (faraday/faraday-sb/tests/judged.rs), not by an F79 area; sbverify fails the test where it is missing | An RNG adapter written here, ChaCha20 (RFC 8439) as a stream keyed by a fresh 32-byte draw with the purpose in the nonce's first byte, feeds a pre-1.0 release of the rsa crate; its keystream is checked against RFC 8439 and every key it makes by openssl pkey -check. Novel; F158. A boot program is signed only when it is a release's own. Conservative; F134. Checks the signature alone and not the chain a computer's db would walk. Neutral; no decision. |
| Convert a seed | Each target's own standard | Not built (`docs/SEED-CONVERSION.md`) | Planned: vectors, another implementation, read-back by fingerprint | Only same-fingerprint conversions. Conservative (F88). |
| Seed XOR tool | Seed XOR (Convention, vendored); BIP 39 (Deployed) | As in §3 | vectors `tools/vectors/xor`; F79 area `xor` against Coldcard's own `xor` and `xor_split` (partly Faraday's own model, not an independent check); 2 tests (faraday-entropy) | None |
| Miniscript compiler (`core/faraday-bip/src/compile.rs`) | BIP 379 (Draft). rust-miniscript's compiler. | SegWit or Taproot | 3 tests (faraday-bip); not cross-checked: the compiler is rust-miniscript's; the area miniscript checks the scripts Faraday reads, not this compiler's choices | The script shape is left to rust-miniscript's compiler. Neutral; no decision. |
| Hashes, encodings, descriptor checksum, xpub spellings, units | SLIP-0132, FIPS 180-4 (Status not checked); BIP 32, 173, 350, 380 (Deployed); Base58Check (Convention) | Full | vectors `tools/vectors/bip32`, `tools/vectors/addresses`; F79 area `addr` against Bitcoin Core's functional-test framework; 10 tests (faraday-bip, faraday-codec); fuzz `xkey`, `encodings` | A SLIP-132 version is checked against the script and the path around the key. Conservative; F155. |
| Hash, MAC and key-stretching functions (`core/faraday-crypto`) | FIPS 180-4, RFC 2104, 4231, 7914, 8018 (Status not checked) | SHA-256, HMAC-SHA256 and -SHA512, PBKDF2 through vetted crates; scrypt written here | vectors `tools/vectors/kdf`; F79 area `bip39` against python-mnemonic; F79 area `lightning` against LND's own code, vendored in Go; 8 tests (faraday-crypto); self-test `bip39_vector`, `scrypt_vector` | scrypt is written here rather than taken from a crate. Neutral; no decision. |
| Nostr key | BIP 85 128002' (Deployed), NIP-19, NIP-49 | Not built (`docs/NOSTR.md`) | None | None |
| Start-up self-test (`core/faraday-selftest`) | BIP 32, 39, 84, 340 (Deployed); RFC 6979, 9106 (Informational); RFC 7914 (Status not checked) | Eight fixed-vector checks before any key is accepted | vectors `tools/vectors/bip39`, `tools/vectors/psbt`, `tools/vectors/kdf`; 4 tests (faraday-selftest, faraday-core); self-test `bip39_vector`, `bip39_japanese_vector`, `bip32_vector`, `bip84_address`, `ecdsa_vector`, `schnorr_vector`, `scrypt_vector`, `argon2id_vector`; not cross-checked: it checks the app's own functions against fixed published answers; the functions it calls are cross-checked in the bip39, bip32, sign and vault areas | No key is accepted until the checks pass. Conservative; no decision. |
| The Starter writes an image (`faraday/faraday-payload`) | FIPS 180-4 (Status not checked) | Payload and developer image, each checked by SHA-256 before and after decompression | 5 tests (faraday-payload); not cross-checked: the sums are compared with `sha256sum`'s in the tests; no F79 area runs the decoder | Both the compressed and the decompressed sums are checked, and a mismatch after writing is reported as a failure. Conservative; no decision. |
| The Starter writes a boot file the person signed | Authenticode (Status not checked); RFC 8017 (Informational) | Signature moved onto the release's own bytes, then checked | Real `sbverify` (sbsigntools, outside Faraday) verifies the signature moved onto the release's bytes and refuses it on a changed program (`faraday/faraday-sb/tests/judged.rs`, the transplant test); required, nothing skips it; 5 tests (starter, faraday-sb); fuzz `pe_transplant`; not cross-checked: judged by images signed by sbsign and osslsigncode and by sbverify (faraday/faraday-sb/tests/judged.rs), not by an F79 area; sbverify fails the test where it is missing | Anything but the release's own bytes with the person's signature is refused by name. Conservative; F133. |
| The Starter prints the BitLocker recovery key (`faraday/faraday-starter`) | None | Read, held until the notes sheet is made, and wiped | 4 tests (faraday-starter); not cross-checked: the key is read and printed as text; nothing is computed from it, and there is no second implementation | The sheet carries the key only when it is ticked, and holds it only until the sheet is made. Conservative; F49. |
<!-- crypto-manifest: end §5 -->

## 6. Where Faraday is stricter than the standard

Faraday's own calls of kind Conservative, from the manifest: each refuses or
requires more than its standard does.

<!-- crypto-manifest: begin §6 -->
Recorded in a decision:

- A passphrase is printable ASCII, at most 256 bytes. BIP 39 allows any NFKD text, so a wallet made elsewhere with another passphrase cannot be restored here; a letter outside the range typed into a passphrase field is refused. (F155)
- Restore takes a seed for a wallet's key slot only when its whole extended public key at the slot's path matches the wallet's, not the four bytes of its fingerprint. (F155)
- A multisig key with no origin is refused where a policy reads it. (F155)
- The list of reasons is closed: a new reason is a new variant, seen in review. (F70)
- Only BIP 341's H, or an extended key over H, counts as an unspendable internal key. (F132)
- A SLIP-132 version is checked against the script and the path around the key. (F155)
- A Caravan file whose startingAddressIndex is not 0, and an Electrum multisig keystore with no fingerprint, are refused. (F155)
- Script-path spends of a musig() wallet are refused; only the key path is built. (F155)
- The stick and the Pi read /dev/random; the Pi's generator is mixed, not credited. (F102)
- The stick's kernel credits neither the CPU's RDRAND nor the firmware's seed; both are still mixed in. (F86)
- Added randomness never replaces the system's bytes. (F86)
- A key origin of more than 16 steps anywhere in a PSBT is refused when it is parsed. (F155)
- Change under an unloaded key, or at a path under no loaded account, counts as a payment to others. (F136)
- A script of several keys is signed only for a registered wallet that holds a loaded key. (F135)
- A miniscript or a taproot tree of one key is signed only through its registered wallet. (F141)
- A SegWit v0 input with only a witness UTXO in a transaction of several inputs is refused, which BIP 174 allows. (F155)
- The signed-amount memory refuses one transaction stated again with other amounts. (F155)
- NONE, SINGLE and non-standard sighashes are refused for signing; ALL with ANYONECANPAY is signed after a caution. (F136)
- A key comes from the PSBT's origin only, under the master key's fingerprint; a key whose derived public key differs from the one named, or that the script does not hold, is refused, and an origin under an account's fingerprint matches no key. (F155)
- Only a high fee and an absurd fee rate can be held past; every other danger refuses. (F136)
- The secret nonce is held in memory for its one session and never serialised. (F109)
- Every limit fails closed: a frame, a count or a size past it is refused. (F155)
- Recovery is from exactly k shares only. (F90)
- A further share is derived only from exactly k strings at an index not in use. (F90)
- Correction is opt-in, off whenever the form opens, proposes only where the correction is unique, and is never applied: the person types the changed boxes again. (F94)
- A character outside the bech32 alphabet is not read as an erasure, and more than 8 erasures not in one run are refused; BIP 93 makes the first optional, and python-codex32 accepts both. (F155)
- No code is drawn for a 74 or 127 character string, and none is read but as plain text, until the profile's author publishes a profile for them. (F95)
- Shares with an iteration exponent above 5 are refused; a split writes exponent 1 with the extendable flag set. (F90)
- A split with a group threshold of 1 over several groups is refused. (F90)
- Secrets above 256 bits are refused. (F155)
- A key a PSBT states for an input counts only where the input's script commits to it, which is stricter than BIP 375's reference validator. (F89)
- The reader limits (lanes exactly 1, passes at most 10, memory 8 MiB to 4 GiB) refuse headers that would cost a guess or an allocation the vault does not state. (F155)
- Every slot's plaintext begins with 32 zero bytes, the padding fix for key commitment, so a slot that verifies under two keys shows them under at most one (docs/VAULT.md §4, §7). (F159)
- A slot whose tag verifies and whose commitment block is not zero refuses the whole file, as two slots opening under one key do: this format never writes one (docs/VAULT.md §4). (F159)
- Signing waits until the backup flow has written every place the key's plan gives it, two or more; GnuPG has no such gate. (F32)
- Signing takes a hold (F63) and shows the files' names, sizes and SHA-256 first. (F32)
- The paperkey is computed only while it is held to show, and wiped after. (F32)
- A restore that cannot reproduce the certificate's own signatures is refused rather than accepted on the public key alone. (F155)
- Anything not honoured is refused rather than read, so the release check of F134 cannot be satisfied by a signature of another shape. (F134)
- A boot program is signed only when it is a release's own. (F134)
- Anything but the release's own bytes with the person's signature is refused by name. (F133)
- The sheet carries the key only when it is ticked, and holds it only until the sheet is made. (F49)

Not recorded in a decision:

- An extended private key is decoded by Faraday's own strict reader because bitcoin accepts seven of the sixteen serialisations vector 5 rejects.
- A descriptor record whose paths line is other than /0/*,/1/* is refused.
- The id is all 32 bytes of a hash, so two seeds never pass for one.
- The reader refuses what BIP 370 marks invalid and keeps every pair as read.
- A previous transaction is attached to an input only when its id is the spent one.
- A part of another message, and an empty message to animate, are refused.
- An envelope is refused unless it is exactly what pack would write for its file.
- A secret goes to a stick only for the backup-stick reason, and only the part that reason names.
- Nothing is written to the Outbox until the hold on the secret sheet is held to its end.
- A secret pasted or scanned where a secret may not go is refused, whatever its form.
- Spends of SegWit above v1 and sighashes other than ALL are refused.
- Four tag checks on every open, so timing does not show which slot a passphrase opens.
- Which passphrase goes with which words is decided by the vault's record, not by a guess; a mismatched fingerprint is not loaded.
- No key is accepted until the checks pass.
- Both the compressed and the decompressed sums are checked, and a mismatch after writing is reported as a failure.
<!-- crypto-manifest: end §6 -->

## 7. Novel constructions for outside review

Ranked by how much a mistake would cost. The call for an outside review
of each, what a reviewer gets and how a review is recorded, is
`docs/AUDIT-SUBMISSIONS.md` §10.

<!-- crypto-manifest: begin §7 -->
1. **Makes a vault file of four equal slots from one to four passphrases, with Argon2id, HKDF-SHA256 and XChaCha20-Poly1305, and a ChaCha20 keystream for the salt, the nonces and the unused slots.** Faraday's own container with four equal slots, so a file does not say how many slots are in use (docs/VAULT.md §9); unused slots are keystream, one salt serves every slot, and an older copy of a slot still opens. Kept until the outside review. (F158) **Adds, changes, copies and makes public the records of an open vault, holding every secret field in a value that wipes on drop.** Faraday's own record layouts (docs/VAULT.md §7); a reader Faraday's authors wrote from that text, not from the code, reads them, which is not an independent check. (F97) Own construction, not yet independently reviewed (F160).
2. **Records pointer moves and key presses with time and a running count, and commits them in batches of SHA-256 for the session's randomness.** Pointer and key timings are added to the session's randomness, never made into a key on their own. (F86) **Keeps the session's seed and pool from the shell's answers, mixes a person's own randomness in, never in place of the system's, and draws the seed of a MuSig2 round.** The mixing construction is Faraday's own. (F86) Own construction, not yet independently reviewed (F160).
3. **Holds a key in memory blinded and on pinned pages; the sealing under a session key built in faraday-crypto is reached by no shipped code.** The in-memory seal's nonce is a counter with a prefix derived from the key, because the crate has no random number generator; no shipped code reaches it, so no secret is sealed in memory today. (no decision) Own construction, not yet independently reviewed (F160).
4. **Turns playing-card draws into entropy as SHA-256 of the draws' indices, one byte each.** The procedure is Faraday's own: a card is the index suit × 13 + rank, and the entropy is SHA-256 of those one-byte indices; no other wallet's card mode is claimed to match, and the bits counted assume a well-shuffled deck. (F158) **Commits to camera frames as SHA-256 over the frames' SHA-256 digests, for a mix or the session's randomness, never a key alone.** The procedure is Faraday's own, and three frames per 128 bits is a heuristic; the frames only add to a mix or to the system's randomness, never make a key alone. (F158) **Combines two to five sources, dice or this device among them, by hashing their commitments in the order taken.** The construction is Faraday's own: SHA-256 over the concatenated commitments. With dice or this device among the sources, each at its full count, it only adds to that member's strength. (F158) Own construction, not yet independently reviewed (F160).
5. **Makes an owner's Secure Boot keys, PK, KEK and db as RSA-2048 with self-signed certificates, and the enrolment files: signature lists, PKCS #7 signed updates and certificates.** An RNG adapter written here, ChaCha20 (RFC 8439) as a stream keyed by a fresh 32-byte draw with the purpose in the nonce's first byte, feeds a pre-1.0 release of the rsa crate; its keystream is checked against RFC 8439 and every key it makes by openssl pkey -check. (F158) Own construction, not yet independently reviewed (F160).
<!-- crypto-manifest: end §7 -->

## 8. Documentation that disagrees with the code

Each disagreement here is held by `tools/lint-standards.py`, which fails
when the text that makes it changes (`tools/lint-standards.toml`,
`[[disagreement]]`); a fix removes the bullet and the entry together.

None now.

## 9. Display

| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| Colour schemes (`faraday/faraday-ui/src/pal/schemes.rs`, `docs/THEMES.md`) | Material 3's colour system: HCT, tonal palettes, dynamic colour, the 2021 colour spec (Google, published) | Ten seeds × two modes × three contrast levels, sixty schemes; the 2021 role set, with OK and Warning as custom colours; no custom seed | material-color-utilities 0.4.0, vendored (`tools/reference/material-color-utilities`), regenerating the tables byte for byte (`tools/lint-themes.sh`); Material's contrast constraints on every scheme (`faraday/faraday-ui/src/theme.rs`) | The seeds and styles, the status colours left unharmonised near a primary, focus and pressed layers at 0.10 by Material's guidance (F113). Neutral |
