# Check a signer: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/SIGNER-CHECK.md` on 2026-11-04. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## 10. Plan

### 10.1 Rules for the implementing agent

CLAUDE.md holds. In particular: a finding's wording is in `en.rs` only;
the check fails closed, so a case this spec does not name is a finding
at Danger, never clean, and is raised with the owner; the check touches
no key material outside `recompute_signatures`; the structure ratchets
only fall.

### 10.2 Batches

| Batch | What | Model |
|---|---|---|
| S1 | built: the fields `compare` did not read (§2): an output's taproot key origins and tap tree, an input's hash preimages; and from the two files' bytes the PSBT's version, the pairs only version 2 has and the modifiable flags (§6.5), with the returned file's length on version 2 (§6.7). Its tests in `core/faraday-psbt/tests/all/signer.rs` | Opus |
| S2 | `faraday_psbt::signer`: §1, §5, §6, the byte count of §6.7; the fixtures of §9.1 and the README rows; §9.2's core tests | Opus |
| S3 | §7: `UnknownFields` in the inspection, its string, its test | Sonnet |
| S4 | built: the flow: the decode area's folder, `Screen::SignerCheck`, the tile, Files' ⋮ entry (§3), the cards, the strings, the nonce lines, the Learn section, faraday-core's and the budget tests; the docs of §10.3 | Opus for the flow, Sonnet for the docs |
| S5 | built: Practice: a sample pair, one clean and one with a proprietary pair (§10.4 item 4); then this file's plan moved to docs/history/SIGNER-CHECK-LOG.md (new) and its Kind to Reference | Sonnet |

### 10.3 Edit list

| File | Change |
|---|---|
| `core/faraday-psbt/src/verify.rs`, `core/faraday-psbt/src/v2.rs`, `core/faraday-psbt/src/signer.rs` | S1: built |
| core/faraday-psbt/src/signer.rs (new), `core/faraday-psbt/src/lib.rs` | S2 |
| `core/faraday-psbt/src/inspect.rs` | S3: `WarningKind::UnknownFields` |
| `core/faraday-psbt/README.md` | S2: the module's line |
| `tools/vectors/psbt/README.md` | S2: the fixtures |
| `faraday/faraday-core/src/decode/mod.rs`, `faraday/faraday-core/src/decode/screen.rs` | S4: built, in the decode/ folder (§8.1) |
| `faraday/faraday-core/src/lib.rs` | S4: `Screen::SignerCheck`, the code map |
| `faraday/faraday-core/src/catalog.rs` | S4: the tile, `Go::SignerCheck` |
| `faraday/faraday-core/src/file_menu.rs`, `faraday/faraday-core/src/screens/files.rs` | S4: **Check a signer** on a PSBT row's ⋮ (§3) |
| `faraday/faraday-strings/src/en.rs` | S3, S4 |
| `tools/lint-layout.counts` | S4: the decode line renamed |
| `docs/learn/06-verifying.md` | S4: a section on checking a signer, which the ? opens |
| `docs/FLOWS.md` | S4: a section "Check a signer" (the text is in the report that brought this spec); A ⋮ on every file, item 1, gains **Check a signer** after **Show as QR** |
| `docs/DESIGN.md` | S4: §5's table, a row for `SignerCheck`, and the count of `Screen`'s variants |
| `docs/README.md` | S4: this file under Spec now, under Reference when built; a By-question row |
| `docs/ANTI-EXFIL.md` | S4: the sentence on the checks that need no new protocol names this file |

### 10.4 Questions for the owner

Decided (owner, 2026-10-23), each as recommended:

1. **Levels.** Decided: a field removed (M3), a signature removed (S7)
   and a signature not checked (S2) stay Caution; added or changed is
   Danger. All fail the verdict.
2. **Nonces not recomputed.** Decided: a clean verdict does not require
   every added signature's nonce recomputed here; the "Nonces
   recomputed" fact states the count (§1 item 5, §4).
3. **MuSig2 nonces.** Decided: a nonce added is Info (§6.4, §6.8).
4. **Practice.** Decided: yes, a sample pair in Practice, one clean and one
   with a proprietary pair; batch S5 is in the plan (§10.2).
5. **A finished transaction as Returned.** Decided: not now. A
   Returned file that is a transaction, not a PSBT, is refused on the
   card (§3).
6. **Files' ⋮.** Decided: yes, **Check a signer** on a PSBT row's ⋮,
   opening the flow with that file as Returned (§3, batch S4, §10.3).
7. **The decode area as a folder.** Decided: the flow is in the decode
   area, made a folder (§8.1); no field is added to `Faraday`.
