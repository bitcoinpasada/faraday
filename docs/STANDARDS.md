# Standards: what Faraday follows, how fully, and where it decides

**Kind:** Reference
**Status:** Built

For each wallet kind, key and backup method, signing and transfer
format and tool: the standard it follows and that standard's status, how
completely Faraday implements it, how that is checked, and where Faraday
made its own call. It is the source for the Learn pages' "How Faraday
does it" blocks and their manifest, and is kept in step with them: a
change to a block, to the manifest or to a row here changes the others.
Decisions are in `docs/DECISIONS.md`; what each cryptographic crate is
checked against is in `docs/AUDIT.md` and `docs/MULTIVENDOR.md`.

Taken from the working tree on 2026-10-23.

## 0. What goes into Faraday

The rule for what the app offers (owner, 2026-10-23; written in the
decision that took FROST out of the app):

- **A published standard, followed and checked.** A wallet kind, key
  method, format or signing procedure is in the app when it follows a
  published standard (a BIP at Complete or Deployed, a SLIP, an RFC, or a
  convention other wallets already use), is built in full or names the
  part it leaves out, and is checked against that standard's vectors or
  another implementation (F79's cross-check, other wallets).
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
- **Otherwise it waits in the repository.** Work on a feature whose
  standard is unpublished, or whose construction is Faraday's own and not
  yet reviewed, stays in the repository, built and tested but compiled
  out of the app, until its standard is published or it is reviewed.
- **Scope.** Faraday is an offline Bitcoin signer and its keys' keeper; a
  feature outside that, however well specified, is not taken on.

### 0.1 Left out, and why

| Left out | Why | Where it stands |
|---|---|---|
| FROST threshold wallets | BIP 445 is not published, and signing across places is Faraday's own construction, not yet reviewed | Kept in the repository (`core/faraday-bip`, `core/faraday-psbt`), compiled out of the app; back when the BIP is published and signing across places is reviewed |
| MuSig2 signing across places | Secret nonces drawn at one place and carried or kept in the vault between visits; BIP 327 advises against storing a secret nonce | Kept in the repository, compiled out of the app |
| Sending to silent payments | BIP 374 and BIP 375 are drafts; receiving (BIP 352) is in the app | Built in `core/`, not reached from the app |
| SSH signatures and an SSH CA, minisign and signify, an X.509 certificate authority | Outside a Bitcoin signer's scope | Not planned |
| The Companion phone app, and APK signing with it | Dropped (F34); APK signing removed (F99) | Removed |
| Reading `.oskb` backups | A format Faraday no longer writes; the vault's records are its own (F97) | Removed |

## 1. How to read it

**BIP statuses.** The BIPs repository uses BIP 3's statuses: Draft,
Complete, Deployed, Closed. The old "Final" corresponds roughly to
Deployed and "Proposed" to Complete. A status was read from the BIP text
in the BIPs repository's master branch on 2026-10-23, or from the text
vendored in `tools/vectors` (marked "vendored" where the vendored copy
was the source). "Status not checked" marks a standard whose status was
not looked up: SLIP-0039, SLIP-0132 and the other non-BIP documents.
BIP 445 is not published; `tools/vectors/bip445/README.md` pins the
draft its vectors come from. Links follow the pattern
`https://github.com/bitcoin/bips/blob/master/bip-NNNN.mediawiki`.

| BIP | Status | BIP | Status |
|---|---|---|---|
| 32, 44, 48, 49, 84, 86 | Deployed | 341, 342 | Deployed |
| 39 | Deployed | 350, 173 | Deployed |
| 45 | Complete | 370 | Deployed |
| 67 | Complete | 373 | Complete |
| 85 | Deployed (vendored) | 374, 375 | Draft |
| 93 | Draft (vendored) | 379 | Draft |
| 129 | Complete (vendored) | 380 to 386 | Deployed |
| 137 | Deployed | 387 | Deployed (vendored) |
| 141, 143, 144 | Deployed | 388 | Complete |
| 174 | Deployed | 389 | Draft |
| 322 | Complete | 390 | Draft (vendored) |
| 327 | Deployed | 392 | Draft (vendored) |
| 328 | Complete | 321, 353 | Complete (vendored) |
| 340 | Deployed | 352 | Complete (vendored) |
| 445 | Not published | | |

**Columns.** *Completeness*: how much of the standard is built.
*Verified by*: published vectors, a second implementation (the F79
cross-check, `docs/MULTIVENDOR.md`), Bitcoin Core or other wallets'
files. *Faraday's own call*: its kind, and the decision that records it
or "no decision". Kinds:

- **Conservative**: stricter than the standard; fails closed.
- **Neutral**: a format or interface choice with no security effect.
- **Novel**: Faraday's own cryptographic construction or protocol.

## 2. Wallet kinds

| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| Single key: `pkh`, `sh(wpkh)`, `wpkh`, `tr` (`faraday/faraday-core/src/create.rs`) | BIP 32, 44, 49, 84, 86 (Deployed); BIP 380 to 386 (Deployed); BIP 341 (Deployed) | Full for derivation and addresses (rust-bitcoin) | BIP 32 vectors (`tools/vectors/bip32`); Bitcoin Core end to end (`faraday/tools/core-check.py`, INTEROP §1, against Core 31.1); not in F79 (MULTIVENDOR §10.5 items 3 and 4 open) | Account asked after the quorum (F98); `h` written for hardened (F66). Neutral |
| Multisig `wsh` and `sh(wsh)` `sortedmulti` | BIP 48 (Deployed), BIP 67 (Complete), BIP 383 (Deployed) | Full | Core end to end with two sessions combined (INTEROP §1); Sparrow, Coldcard and Electrum files | A key with no origin is refused; a SLIP-132 version is checked against the script. Conservative; F90 covers part, see §6 |
| Legacy multisig `sh(sortedmulti)` at `m/45'` | BIP 45 (Complete) defines `m/45'/cosigner/change/index`; Faraday uses `m/45'` as an account with `/<0;1>/*`, the de-facto Coldcard and Unchained export | A convention, not BIP 45's scheme | Core end to end; Unchained and Coldcard files | Path convention. Neutral; no decision |
| Taproot multisig `tr(H, sortedmulti_a)` (`core/faraday-bip/src/tapmulti.rs`) | BIP 386 (Deployed), BIP 387 (Deployed, vendored). The path `m/48'/c'/0'/3'` has no standard: BIP 48 defines script types 1' and 2' | `sortedmulti_a` is parsed here because miniscript lacks it; a hardened wildcard is parsed but not derived | BIP 387 vectors (`core/faraday-bip/tests/all/bip387.rs`); Core 31.1 PSBT round trip (`tools/vectors/psbt`, `wallet-tapmulti-*`) | The `3'` path is Faraday's choice. Only BIP 341's bare `H` counts as unspendable. Neutral and Conservative; no decision |
| Miniscript and timelocked recovery, Liana shapes (`core/faraday-bip/src/recovery.rs`, `core/faraday-bip/src/policy.rs`) | BIP 379 (Draft), BIP 382 and 386 (Deployed), BIP 388 (Complete), BIP 389 (Draft) | Reads any `wsh`, `sh(wsh)` or `tr` miniscript; Create does not make one; the compiler is in Tools | Core end to end for the `wsh` timelock and Taproot tree wallets (INTEROP §1); Liana fixtures (`tools/vectors/psbt/README.md`) | Script shape left to rust-miniscript's compiler. Neutral; no decision |
| MuSig2 `tr(musig(...)/<0;1>/*)` | BIP 327 (Deployed), BIP 328 (Complete), BIP 373 (Complete), BIP 390 (Draft, vendored) | Key path only; script-path spends refused; no WIF participant or tap tree (`tools/vectors/bip390/README.md`) | All eight BIP 327 vector files; BIP 328 and 390 vectors; F79 area `musig` against BIP 327's `reference.py`; Core 31.1 round trip in both orders (`tools/vectors/psbt`, `wallet-musig-*`) | `rand'` taken from the session seed chain (`core/faraday-psbt/src/musig.rs`); the secret nonce is sealed into the vault across a lock (`docs/WALLETS.md` §4.2). Neutral, and Novel for the persisted nonce; no decision |
| FROST threshold (`core/faraday-bip/src/frost.rs`, `core/faraday-psbt/src/threshold.rs`) | BIP 445 (not published); key generation has no standard (a trusted dealer is written here) | Signing per BIP 445; dealer only, no DKG; at most 15 participants against BIP 445's 128; SIGHASH_DEFAULT only; no merkle root | All six BIP 445 vector files; F79 area `frost` against `frost_ref`; regtest fixtures and `tools/scripts/threshold-reference.py` | Dealer design; shares held as 24 BIP-39 words of a raw scalar; the `osk-threshold 1` record; BIP 328's synthetic xpub over a FROST key; the `osk` PSBT proprietary fields; signing across places (§7). Novel; no decision |
| Silent payments, receive (`core/faraday-bip/src/silent.rs`, `core/faraday-bip/src/silent_wallet.rs`) | BIP 352 (Complete, vendored); BIP 392 (Draft, vendored); BIP 321 and 353 (Complete, vendored) | Receive, labels, scan-key export; reading `sp()` open (INTEROP §5 I9); sender arithmetic is test-only, no K_max | All BIP 352 vectors, both sides; F79 area `sp` against `reference.py` | The `osk-silent 1` record. Neutral; no decision |
| Silent payments, send and PSBT (`core/faraday-psbt/src/silent.rs`, `core/faraday-bip/src/dleq.rs`) | BIP 374 and 375 (Draft) | Built in core, not reachable from the app: the device does not send | BIP 374 and 375 vectors (`tools/vectors/bip374`, `tools/vectors/bip375`); not in F79 | A stated key counts only where the input's script commits to it (F89). Conservative |
| Watch-only wallets | Descriptors | Full | Core and file interop | None |

## 3. Keys and backups

| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| BIP-39 (`core/faraday-bip/src/bip39.rs`, written here) | BIP 39 (Deployed) | Ten word lists, every length. A passphrase is printable ASCII, at most 256 bytes | Official and Japanese vectors (`tools/vectors/bip39`); the `bip39` crate as a dev-dependency (`docs/AUDIT.md` §1); F79 only through XOR and BIP 85 | ASCII-only passphrase: Conservative, not in F90. Word entry by readings and kana: Neutral |
| Dice (`core/faraday-entropy/src/dice.rs`) | De facto: Coldcard `rolls.py`, SeedSigner, Keystone, BitBox; sources pinned in `tools/reference/dice/README.md` | Three procedures; 99 rolls for 256 bits, as the others accept | Transcripts from those tools (`faraday/faraday-core/tests/all/keygen.rs`); not in F79 | Cautions (runs, chi-square) are warnings only. Neutral; no decision |
| Coins, hex | Convention: most significant bit first | Full | Own tests | Neutral; no decision |
| Playing cards (`core/faraday-entropy/src/cards.rs`) | None: SHA-256 of one-byte draw indices | | Own tests only | Own procedure. Novel, low risk; no decision |
| Camera noise (`core/faraday-entropy/src/camera.rs`) | None: SHA-256 of the frames' SHA-256 digests; 3 frames per 128 bits is a heuristic | | Own tests only | Own procedure. Novel, low risk; no decision |
| Mix of sources (`core/faraday-entropy/src/mix.rs`) | None: SHA-256 over the concatenated commitments | | Own tests | Novel, low risk; no decision |
| Own randomness in the session (`faraday/faraday-core/src/fresh.rs`) | None (`docs/OWN-RANDOMNESS.md` §2.3) | R1 built; R2 and R3 open | The spec's tests; its required F79 known answers are open (§4) | Never replaces the system's bytes (Conservative); the construction is Novel (F86) |
| Booklet dice for codex32 (`core/faraday-entropy/src/booklet.rs`) | codex32 booklet SSS32.ps 2023-03-07 (`tools/reference/booklet-dice/README.md`) | Six-faced dice only | Booklet tables; F79 codex32 area | Neutral; no decision |
| Desktop dice roller | None | Desktop only, labelled "simulated dice" | None | An exception to "Faraday's randomness never makes a codex32 seed"; testnet-only shell. Neutral (F96) |
| SLIP-39 (`core/faraday-bip/src/slip39.rs`, written here) | SLIP-0039 (status not checked) | Secrets of 16 to 32 bytes; exponent at most 5 on read; writes exponent 1 with the extendable flag set | All 45 vectors; F79 area `slip39` against python-shamir-mnemonic, word for word | Refusals in F90; the 256-bit cap is not in F90 (Conservative). The 1-of-1 identifier is taken from the fingerprint (`docs/SEED-CONVERSION.md` §4.4): Neutral, a privacy cost |
| codex32 (`core/faraday-bip/src/codex32/`) | BIP 93 (Draft, vendored) | Codec, split, derive, recover, worksheets and the correction decoder built; the flows in `docs/CODEX32.md` marked open are not | Every BIP 93 vector including the 55 invalid; booklet and volvelle tables; F79 areas `codex32` and `codex32-correct` | Recovery from exactly `k` shares (F90, Conservative); error correction: BIP 93 specifies no procedure; follows python-codex32's unmerged decoder; cross-checked over 100,057 cases (F94); opt-in, off by default, never applied automatically (Conservative); identifier taken from the fingerprint, 20 bits (Neutral, a privacy cost); a 127-character codex32 of a BIP-39 seed with the passphrase folded in (F93, Neutral) |
| Codex32QR/v1-48 (`tools/vectors/codex32qr/README.md`) | A profile of one SeedSigner fork, pinned in that README; Faraday's own, fails closed (F95) | 48 characters only, by rule | Profile vectors; F79 codex32 area | Refuses to invent longer profiles. Conservative (F95) |
| Seed XOR (`core/faraday-entropy/src/xor.rs`) | Coldcard's seed-xor document (de facto) | 2 to 4 parts | Coldcard's two examples at commit f28e1b26 and two constructed here (`tools/vectors/xor/README.md`); F79 area `xor` | Neutral; no decision |
| SeedQR and CompactSeedQR (`core/faraday-codec/src/seedqr.rs`) | SeedSigner's SeedQR document (de facto) | English only | Spec vectors (INTEROP §2) | A copy checked by scan or typed words (F29). Neutral |
| BIP 85 (`core/faraday-bip/src/bip85.rs`) | BIP 85 (Deployed, vendored) | Applications 2', 32', 39', 128169', 707764', 707785'; no DICE, RSA or Nostr 128002' (F27) | BIP 85 vectors; F79 area `bip85` | Neutral; no decision |
| Electrum seeds | Electrum's own scheme | Not built (INTEROP §5 I7) | None | None |
| aezeed (`core/faraday-bip/src/aezeed.rs`, `core/faraday-bip/src/aez.rs`, written here) | LND's implementation (de facto); AEZ v5 | Decode, for node keys and conversion | AEZ reference vectors (`tools/reference/aezeed/aez-testdata`); LND's cipher seed and node id (`faraday/faraday-core/tests/all/lightning.rs`); not in F79 | Neutral; no decision |
| Paper backups, the backup plan and its sticks | None | Partly built (`docs/BACKUP-FLOW.md` Open) | Own tests | Places, presets, warnings, "seeds alone spend" (F31, F35, F37, F87): Neutral. Partial descriptions (F52) and the Coldcard config carrying `Format: P2TR`: Neutral, an interoperability risk |
| The vault (`faraday/faraday-vault`, `docs/VAULT.md`) | None: Faraday's own container over Argon2id 1.3, HKDF-SHA256 (RFC 5869, assembled by hand) and XChaCha20-Poly1305 | Built | Own vectors (`tools/vectors/vault`) and a Python reader over independent primitives (`tools/vault/open.py`); F79 vault area open | Four equal slots and deniability (§9); deterministic seal nonces (§5); reader cost limits, a Conservative choice; unnormalised UTF-8 passphrase. Novel; no decision beyond the document |
| KDBX 4 export (`core/faraday-backup/src/kdbx.rs`) | KeePass's KDBX spec (de facto; pages pinned in `tools/reference/kdbx4`) | Write only: 4.0, ChaCha20, Argon2id | Read back by `pykeepass` when installed (`tools/backup/read-kdbx.py`) | Nostr entries never exported (F27). Neutral |
| Diceware passphrases | EFF word lists (`tools/vectors/eff`) | Three lists | The lists' digests | Neutral; no decision |

## 4. Signing and transfer

| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| PSBT v0 (`core/faraday-psbt`) | BIP 174 (Deployed) | Signs every wallet kind above; the finalizer is written here | BIP 174 and role vectors (`tools/vectors/psbt`); Core end to end; not in F79 (§10.5 item 4) | A key comes from the PSBT's origin only; signed-amount memory; non-ALL sighash refused; a v0 input with only a witness UTXO in a multi-input transaction refused. Conservative; part in F90, see §6 |
| PSBT v2 | BIP 370 (Deployed) | Silent-payment path only; the general reader is open (INTEROP §5 I10); `core/faraday-psbt/src/psbt.rs` refuses any version but 0 | None | The Sign tile in `faraday/faraday-core/src/catalog.rs` carries a BIP 370 tag that the code does not back (§8) |
| Sighash | BIP 143, 341, 342 (Deployed) | All computed (rust-bitcoin); only ALL and DEFAULT are signed; others are review cards (`core/faraday-psbt/src/inspect/sighash.rs`) | Core end to end | Conservative; no decision |
| ECDSA nonces | RFC 6979; Core's low-R grind | Both offered, low-R by default (`core/faraday-psbt/src/sign.rs`) | RFC 6979 vectors (`tools/vectors/psbt/rfc6979.txt`); self-test | Sign twice and compare. Neutral |
| Schnorr | BIP 340 (Deployed) | Zero `aux_rand` by default, fresh as an option | libsecp256k1 vectors in the self-test | BIP 340 allows zero aux but recommends fresh randomness against fault attacks; the default favours reproducibility. Neutral, a judgement; no decision |
| Address and change checks | BIP 173 and 350 (Deployed); Faraday's review rules | The first 100 receive and change addresses for a scanned address (`docs/QR.md` §1) | Core end to end | A gap of 100 is a choice. Neutral; no decision |
| Check a signer | None | Not built (`docs/SIGNER-CHECK.md`) | None | Fails closed by design (F84). Conservative |
| Anti-exfil | Jade's protocol, libsecp256k1-zkp s2c | Not built (`docs/ANTI-EXFIL.md`) | None | None |
| Message signing (`core/faraday-psbt/src/message.rs`) | BIP 137 (Deployed), BIP 322 (Complete) | BIP 322 "simple" only: no "full" form, no proof of funds, no nested SegWit; header ranges as Electrum and Trezor use | `bip322-basic.json` from the BIPs repository; BIP 137 vectors from Trezor's and Electrum's suites (`tools/vectors/message/README.md`); not in F79 | Neutral; no decision |
| UR (`core/faraday-codec/src/ur/`) | Blockchain Commons research documents BCR-2020-005, 006, 012 and others (de facto) | Legacy `crypto-*` types and fountain codes; `crypto-output` is read, never written | BCR-2020-012 vectors; SeedSigner and Keystone codes | `crypto-account` byte-identical to SeedSigner's. Neutral |
| BBQr (`faraday/faraday-qr/src/bbqr.rs`) | bbqr.org (Coinkite, de facto) | Reads base32, hex and zlib; writes base32 only | Own tests | Neutral; no decision |
| Specter `pNofM` (`faraday/faraday-qr/src/specter.rs`) | None published; Specter and SeedSigner behaviour | Read and write | Own tests | Neutral; no decision |
| File envelope `faraday-file-v1` | Faraday OS's own format | Full | Own tests | SHA-256 shows integrity, not origin. Neutral; no decision |
| Descriptors and checksums | BIP 380 to 386 (Deployed), 387 (Deployed, vendored), 388 (Complete), 389 (Draft), 390 (Draft, vendored) | As in §2 | Core checksums; F79 does not cover them (§10.5 item 4) | One text, with `h` (F66). Neutral |
| BSMS (`core/faraday-bip/src/bsms.rs`) | BIP 129 (Complete, vendored) | NO_ENCRYPTION records only (INTEROP §5 I3); signer records' signatures checked | The BIP text's examples (`tools/vectors/bip129`); Nunchuk and Sparrow records | Upper-case bech32 and "No path restrictions" accepted; fixed-path records refused. Neutral and Conservative; no decision |
| Other wallets' files | Vendor formats | As listed in INTEROP §2 | Files from those wallets' test suites; not against the running apps (INTEROP §1) | Neutral; no decision |

## 5. Tools

| What | Standard (status) | Completeness | Verified by | Faraday's own call |
|---|---|---|---|---|
| Explore a key | BIP 32, 44, 84, 86 (Deployed) | Full | BIP 32 vectors | Neutral; no decision |
| Vanity address (`core/faraday-bip/src/vanity.rs`) | EntropyLab's counter (de facto; `tools/reference/vanity/README.md`) | Passphrase dial or account dial | Two EntropyLab vectors | A passphrase found by grinding becomes key material to back up; the account dial gives a non-zero account. Neutral; no decision |
| Decode a transaction | BIP 141 and 144 (Deployed) | Full (rust-bitcoin) | None | Neutral; no decision |
| Lightning node key (`faraday/faraday-keys/src/lightning.rs`) | ldk-node and rust-lightning; LND aezeed | ldk-node from BIP-39, LND from aezeed | One ldk vector computed with `lightning` 0.2.6 (`tools/reference/aezeed/README.md` §6); LND's own | Neutral; no decision |
| TOTP (`faraday/faraday-vault/src/totp.rs`) | RFC 6238 and 4226; the `otpauth://` Key URI format (de facto) | No HOTP, no migration QR | RFC 6238 vectors (`docs/AUDIT.md` §1; faraday-keys tests) | Manual clock, fixed-offset zone (F7). Neutral |
| GPG (`faraday/faraday-pgp`) | RFC 4880 and 9580 v4 packets; GnuPG behaviour (F32) | One key shape: Ed25519 certify plus a sign subkey | Judged by real `gpg` 2.4 and `paperkey` 1.6 (`faraday/faraday-pgp/tests/gnupg.rs`) | Signs only after the backup flow writes at least two places (F31, F32). Conservative |
| GPG paperkey restore | paperkey format | Not built (F32; GPG paperkey spec) | None | None |
| Secure Boot keys (`faraday/faraday-sb`) | UEFI spec, Authenticode, X.509 and CMS | PK, KEK and db RSA-2048, `.auth` files, signing | `openssl` and `sbverify` (`faraday/faraday-sb/tests/judged.rs`) | An RNG adapter written here: a ChaCha20 keystream over a 32-byte draw (`faraday/faraday-sb/src/rng.rs`), feeding the pre-1.0 `rsa` 0.10.0-rc.18. Novel, low risk; no decision |
| Convert a seed | Each target's own standard | Not built (`docs/SEED-CONVERSION.md`) | Planned: vectors, another implementation, read-back by fingerprint | Only same-fingerprint conversions. Conservative (F88) |
| Seed XOR tool | Coldcard | As in §3 | F79 area `xor` | Neutral; no decision |
| Miniscript compiler (`core/faraday-bip/src/compile.rs`) | BIP 379 (Draft); rust-miniscript's compiler | SegWit or Taproot | rust-miniscript | Neutral; no decision |
| Hashes, encodings, descriptor checksum, xpub spellings, units | SHA-256, HASH160; Base58Check; BIP 173 and 350; BIP 380; SLIP-0132 (status not checked) | Full | Own tests | Neutral; no decision |
| Nostr key | BIP 85 128002' (Deployed), NIP-19, NIP-49 | Not built (`docs/NOSTR.md`) | None | None |

## 6. Where Faraday is stricter than the standard

Recorded in F90 or F89:

- SLIP-39 iteration exponent above 5 (`core/faraday-bip/src/slip39.rs`).
- SLIP-39 group threshold 1 over several groups.
- codex32: recovery from exactly `k` shares only; a share derived only at
  an unused index.
- BIP 375: a stated key counts only where the input's script commits to
  it.

Not recorded in a decision, each refusing what a standard allows:

- BIP-39 passphrase: printable ASCII only, at most 256 bytes
  (`core/faraday-bip/src/bip39.rs`). BIP 39 allows any NFKD-normalised
  UTF-8, so a wallet made elsewhere with a non-ASCII passphrase cannot be
  restored here.
- SLIP-39 secrets above 256 bits.
- Any PSBT sighash other than SIGHASH_ALL is refused for signing
  (`docs/WALLETS.md` §3.3); FROST signs DEFAULT only.
- A SegWit v0 input with only `witness_utxo` in a transaction of several
  inputs is refused, which BIP 174 allows (`core/faraday-psbt/README.md`,
  "What is refused").
- The PSBT's own stated data is not trusted: the signed-amount memory
  refuses one transaction stated with other amounts; keys are derived
  only from the PSBT's origins, and a multisig key with no origin is
  refused; a PSBT whose fingerprint is the account's rather than the
  master's is refused (INTEROP, end of §5).
- codex32 correction reads no non-bech32 character as an erasure and
  refuses more than 8 erasures not in one run (`tools/crosscheck/README.md`,
  "Known differences"); BIP 93 makes the former optional.
- Taproot's unspendable internal key: only BIP 341's bare `H` is
  recognised (`core/faraday-bip/src/tapmulti.rs`). A wallet using `H + rG`
  reads as having a key path.
- The vault reader: Argon2id lanes exactly 1, memory 8 MiB to 4 GiB, at
  most 10 passes (`docs/VAULT.md` §3).
- Silent payments: spends of SegWit above v1 and sighashes other than ALL
  are refused (`core/faraday-psbt/src/silent.rs`).
- SLIP-132: a key whose version disagrees with the script is refused
  (INTEROP I1).
- Other wallets' files: Caravan `startingAddressIndex` other than 0, an
  Electrum multisig keystore with no fingerprint, and BSMS
  path-restricted records are refused.
- Schnorr signatures use zero `aux_rand` by default; a person can change
  it. This is a judgement, not a refusal.
- The stick's kernel credits neither RDRAND nor the bootloader
  (`faraday/image/boards/x86_64-uefi/cmdline.txt`, F86).

## 7. Novel constructions for outside review

Ranked by how much a mistake would cost.

1. **Signing across places, and secret nonces kept between them** (MuSig2 and FROST; Faraday's own carry construction, `core/faraday-psbt/README.md` "The rounds"; vault record
   type 9; `docs/WALLETS.md` §4.2, `docs/VAULT.md` §7). BIP 327 warns
   against storing a secret nonce. Faraday binds the record to the
   transaction hash and states that a restored older vault copy can sign
   twice under one nonce.
2. **The FROST trusted dealer and the wallet around it.** `t` shares
   chosen freely fix the polynomial and the rest are interpolated
   (`core/faraday-bip/src/frost.rs`). A share is a raw 32-byte scalar
   written as 24 BIP-39 words (`tools/vectors/psbt/README.md`); loaded
   into any BIP-39 wallet, those words open an unrelated wallet. The
   group's record format (`core/faraday-psbt/src/threshold.rs`), the
   `osk` PSBT records and BIP 328's synthetic xpub over a FROST group key
   with BIP 32 tweaks are Faraday's; no BIP combines them.
3. **The vault** (`docs/VAULT.md`): the four-equal-slot container and its
   deniability promise (§9); an HKDF assembled by hand
   (`docs/AUDIT.md` §1); seal nonces derived as SHA-256(label, session
   seed, count) rather than drawn (§5); unused slots filled from a
   ChaCha20 keystream (§8). It is checked against Faraday's own vectors
   and its own Python reader; F79's vault area is open.
4. **The session's random generator** (`faraday/faraday-core/src/fresh.rs`;
   `docs/OWN-RANDOMNESS.md` §2.3, F86): "fresh", the seed's draws and the
   mixing of a person's own randomness, all SHA-256 over concatenations;
   signing seeds and the MuSig2 and FROST `rand'` chain
   (`core/faraday-psbt/src/musig.rs`). Every nonce and key Faraday draws
   passes through it, and its required cross-check is open.
5. **Entropy conversions with no published procedure:** cards (SHA-256 of
   one-byte indices), camera (SHA-256 of the frames' digests, 3 frames
   per 128 bits) and the mix (SHA-256 over the sources' commitments)
   (`core/faraday-entropy/src/lib.rs`, `core/faraday-entropy/src/mix.rs`).
   All hash-based; the risk is low.
6. **Secure Boot RSA keys** from an RNG adapter written here
   (`faraday/faraday-sb/src/rng.rs`), feeding a pre-1.0 release candidate
   of `rsa` (`docs/AUDIT.md` §1).
7. **Format extensions with interoperability risk, not cryptography:**
   partial descriptions (F52); a Coldcard-config file carrying
   `Format: P2TR`, which Coldcard cannot read
   (`faraday/faraday-core/src/wallet_backup.rs`); the codex32 and SLIP-39
   1-of-1 identifiers taken from the master fingerprint, which puts 20 or
   15 bits of it on every share, the linkage F27 avoids elsewhere.

## 8. Documentation that disagrees with the code

- `docs/AUDIT.md` §1 says faraday-entropy mixes "by XOR over their
  SHA-256 commitments". The code hashes the concatenation
  (`core/faraday-entropy/src/mix.rs`).
- `docs/AUDIT.md` §3 says "None is cryptography invented for this
  project". The vault container, the multi-place FROST nonce handling
  and the session's generator are Faraday's own constructions.
- `core/faraday-psbt/README.md`, "What is refused", still lists Taproot
  script-path inputs; `core/faraday-psbt/src/sign.rs` signs `P2trScript`
  and the tapmulti fixtures show it working.
- The carry file: `core/faraday-psbt/README.md` describes a file, and
  `docs/FLOWS.md` says the carry goes into the vault as record type 9.
  Which path the app takes should be stated once.
- The Taproot multisig path is `3'` in `faraday/faraday-core/src/create.rs`;
  the fixture is at BIP 48's `2'` (`tools/vectors/psbt/README.md`).
- The Sign tile's BIP 370 tag (`faraday/faraday-core/src/catalog.rs`)
  claims PSBT v2 support that INTEROP I10 says is not built.
- Domain-separation tags in `core/faraday-psbt/src/musig.rs` and
  `core/faraday-psbt/src/sign.rs` still carry the name of the project
  Faraday was forked from, against F73.
