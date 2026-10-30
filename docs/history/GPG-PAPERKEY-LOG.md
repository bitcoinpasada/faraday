# Restore a GPG key from its paperkey: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/GPG-PAPERKEY.md` on 2026-10-29, when its last open work (P4 to P6) was built and it became a Reference. Each keeps its original number; the source keeps a one-line stub under the same heading. The decision drafted in §10 is F169.

## Header: the spec's status as it was (from docs/GPG-PAPERKEY.md)

**Kind:** Spec
**Status:** Specified from the owner's ask (2026-10-15); P1 to P3 built (reading a paperkey and a certificate, the restore and its tests against paperkey and GnuPG in `faraday-pgp`; the restore set in `faraday-keys`; the flow in faraday-core, `faraday/faraday-core/src/backup_flow/gpg_restore.rs` and its screen); the decision is drafted in §10 as F-next, for the owner
**Open:** batches P4 to P6 (§11.2): scanning the public key and a paperkey code, and the docs

## 10. Decision draft: F-next

The number is assigned when it is committed.

**F-next A GPG key is restored from its paperkey and its public
certificate** (draft, 2026-10-23)

**Why.** A GPG key made in Faraday cannot be made again from seed
words. Its paperkey is shown so that it can be written down (F32), but
only GnuPG could turn it back into a key: a person who lost the key's
vault had no way to bring it back into Faraday.

**Decision** (the owner's ask, 2026-10-15; the spec is
`docs/GPG-PAPERKEY.md`).

- A GPG key is restored from its paperkey's lines and its public
  certificate, through the backup flow (F31), as a wallet is restored:
  the plan, the sticks, the vaults, then the lines typed at step 4. The
  certificate comes in as a public file (F22) or a scanned code.
- The lines are paperkey's base16 form, each checked by its CRC-24 as
  it is ended, and the whole data by its own; the line format and the
  refusals' words are paperkey 1.6's.
- The result is the type 7 record Make wrote: the certificate and the
  kept revocation are made again from the seeds and the times read from
  the certificate.
- Faraday refuses what paperkey would take and Faraday cannot verify:
  a key of another algorithm or shape; a secret protected by a
  passphrase; a secret that does not make the certificate's public key;
  a certificate whose signatures by the key are not the ones Faraday
  makes for it; and a CRC that does not match (paperkey's
  `--ignore-crc-error` has no counterpart). Signatures by other keys
  are not kept.
- The restored key's bytes are held by `faraday-keys` from each line
  checked, and the key enters the vaults as a made key does. No reveal
  is added to F70's list.
- A restored key signs at once (the owner's answer to Q2 may change
  this).

## 11. Plan

### 11.1 Rules for the implementing agent

- CLAUDE.md holds. Agents do not commit. While working, run the one
  test module touched (`cargo nextest run -p faraday-pgp --test
  gnupg`, and the new paperkey tests); `just` once at the end.
- Nothing cryptographic lands without its rows of §8 passing against
  the real `paperkey` and `gpg`.
- No upstream file is copied into the repository unless Q10 says so;
  fixtures are made by the tools, and their README records how.
- No path in a doc that does not exist; new files are named in plain
  text until they exist.
- Never name a person other than the public author of a standard or a
  tool.

### 11.2 Batches

| Batch | What | Model | Estimate |
|---|---|---|---|
| P1 | `faraday-pgp`: the paperkey readers (text, raw), the certificate reader with GnuPG's algorithm names, the rebuild and compare, `restore`; tests 1 to 15 against paperkey 1.6 and GnuPG 2.4, the fixtures and their README; the two fuzz targets; lint-secrets rule 4(c) | Opus | 2 days |
| P2 | `faraday-keys`: the restore set (`Restore::new`, `line`, `finish`), wiped on drop, the lock's wipe; tests 16 to 18 | Sonnet | 1 day |
| P3 | `faraday-core`: the ways in (Files' action, Vault contents, the Tools line), the sheet, the head card with line 1, step 4's lines and the hex keyboard, the refusals, the Result, signing per Q2, Practice dimmed; strings; tests 19 to 22; the snapshot tour | Opus | 3 days |
| P4 | Scanning the public key: an armoured public key code filed in From the stick as a public file, named `{name}-{fp8}.asc`; its routing row in `docs/QR.md` §2 | Sonnet | 0.5 day |
| P5 | Only if Q5 says yes: a paperkey code (base16 text or raw) scanned at step 4 fills the lines, each still checked | Sonnet | 0.5 day |
| P6 | Docs: the Learn page (§9), `docs/FLOWS.md` GPG, `docs/QR.md`, `docs/DEMO.md` §4, `docs/VAULT.md` §7's note, `docs/KEYS.md` §3, the decision recorded with its number, then this file's plan to docs/history/GPG-PAPERKEY-LOG.md (new) and its Kind to Reference | Sonnet | 0.5 day |

About 7.5 days. P1 before P2, P2 before P3; P4 and P5 after P3; P6
last.

### 11.3 Edit list

| File | Change |
|---|---|
| `faraday/faraday-pgp/src/lib.rs` and two new modules beside it | §7.1 |
| `faraday/faraday-pgp/tests/gnupg.rs`, a new tests/paperkey.rs | §8, 1 to 15 |
| `faraday/faraday-keys/src/gpg.rs`, `faraday/faraday-keys/tests/vault.rs` | the restore set; §8, 16 to 18 |
| `faraday/faraday-core/src/backup_flow/mod.rs`, `faraday/faraday-core/src/backup_flow/steps.rs`, `faraday/faraday-core/src/backup_flow/screen.rs` | the restore's state, step 4, the head card |
| `faraday/faraday-core/src/lib.rs` | the sheet's `Sheet` variant |
| `faraday/faraday-core/src/vault/screen.rs`, `faraday/faraday-core/src/screens/catalog.rs`, `faraday/faraday-core/src/inbox.rs` | the ways in |
| `faraday/faraday-core/tests/all/gpg.rs` | §8, 19 to 22 |
| `faraday/faraday-strings/src/en.rs` | §7.3 |
| `tools/lint-secrets.sh` | rule 4(c) |
| `fuzz/fuzz_targets/` | two targets |
| `docs/learn/faraday/gpg.md`, `docs/FLOWS.md`, `docs/QR.md`, `docs/DEMO.md`, `docs/DESIGN.md` §5's sheets, `docs/README.md` | P6 |

### 11.4 Questions for the owner

Answered by the owner on 2026-10-23: every proposal below is taken (Q1
refused; Q2 at once; Q3 two or more places; Q4 set aside and counted;
Q5 P5 builds scanning; Q6 not offered; Q7 armour only; Q8 not offered;
Q9 dimmed in Practice; Q10 not copied; Q11 refused).

- **Q1. Keys made elsewhere.** A key of Faraday's shape made by GnuPG
  (an Ed25519 key that certifies, one Ed25519 signing subkey, no
  passphrase) has signatures Faraday does not make byte for byte (other
  preferences, other subpackets). Proposed: refused. The alternative is
  to verify each of its self-signatures with Ed25519 instead of
  rebuilding them, keep its certificate as it came, and let Renew write
  Faraday's own at the next renewal; it accepts more than Faraday makes,
  so it is the owner's decision.
- **Q2. Signing after a restore.** Proposed: at once, as a restored
  wallet (`docs/BACKUP-FLOW.md` §11): the paperkey is a copy that
  exists. Or wait for every place of the plan to be written, as Make's
  keys do (F31).
- **Q3. The places.** Proposed: two or more, as Make. Or one, since the
  paperkey is kept at a place of its own.
- **Q4. Signatures by other keys.** Proposed: set aside and counted, the
  restore going on. Or refuse a certificate that carries any.
- **Q5. A paperkey as a code.** paperkey's raw form is meant for codes,
  and the ask says "typed (or scanned)". Faraday never shows a paperkey
  as a code (`docs/QR.md` §7, row 23). Proposed: P5 builds scanning a
  paperkey code at step 4 (the scanner opens with no stick in, as for a
  SeedQR, decision 9). Or typing only.
- **Q6. A paperkey file from a stick.** A stick is read only with
  nothing secret open, and a secret file brought in before the flow
  does not last through step 2's lock; step 4 comes after the vaults
  are open. Proposed: not offered. A later design could read the file
  from one of the plan's sticks at step 2 and hold it in the restore
  set across the lock.
- **Q7. Binary certificates.** `gpg --export` without `--armor` writes
  packets with no armour, and F22 reads only armour. Proposed: armour
  only. Taking binary packets as public files amends F22.
- **Q8. A suggested correction.** A CRC-24 points at a line; a single
  mistyped digit could be found by trying each of the line's 44 digits
  at each other value (660 tries) and offering the one that matches, as
  codex32's opt-in correction does (F94). Proposed: not offered; the
  person reads the line again.
- **Q9. Practice.** Proposed: Restore is dimmed in Practice, as Make is. Or a
  sample key with its certificate and paperkey in `faraday/faraday-samples`,
  for the website's walk-through.
- **Q10. paperkey's check keys.** They are GPL-2.0; Faraday is MIT.
  Proposed: not copied; the tests make their own keys with GnuPG, and
  test 5 uses one of the same shape. Or copy `papertest-eddsa.pub` and
  `.sec` into tools/vectors/paperkey/ with their licence notice.
- **Q11. A certificate with no user ID.** keys.openpgp.org serves a key
  without its user ID until the address is verified. Proposed: refused;
  the person exports the key from another copy. Or the user ID typed and
  a new self-signature made by the restored key, which is a change to
  the certificate the person must then publish.
