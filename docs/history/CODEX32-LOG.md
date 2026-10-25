# Codex32: making, checking and repairing shares: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/CODEX32.md` on 2026-10-24, when its last batch
was built. Each keeps its original number; the source keeps a one-line stub
under the same heading.

## 14. Decisions to record

The build's first batch records these (the next free numbers at the
time), each its own file and a row in `docs/DECISIONS.md`. D1 to D4 are
recorded (C1): F93, F94, F95 and F96; D5 is recorded with §6.9's build.

- **D1. Codex32 made by hand, any split** (recorded, F93). §1 items 1 to 8 and 12: the
  main flow, lengths, identifier, hand-made shares, add a share, check a
  string, backups and templates, with the owner's answers (§1.1).
- **D2. Codex32 error correction, opt-in** (recorded, F94). §8.4. It replaces the
  module's statement that it "detects; it does not correct".
- **D3. Codex32QR/v1-48** (recorded, F95). §10: read, shown, tested; nothing longer
  until its author publishes it; the duplicate-index rule of §8.5.
- **D4. Faraday Desktop's dice rollers** (recorded, F96). §5.5, §5.6: on Faraday Desktop
  only, simulated dice from this computer's randomness, labelled
  "Rolled by this computer" and "simulated dice"; the exception to §5's
  rule that Faraday's own randomness never makes a fresh seed in these
  flows.
- **D5. Single key, backup in shares** (recorded with §6.9's build). §6.9: the wallet kind on
  Make a new wallet's Kind card, codex32 only; codex32 made for
  single-key wallets only, a loaded codex32 key accepted anywhere.

## 15. Plan

### 15.1 Rules for the implementing agent

- CLAUDE.md holds. Agents do not commit. Run one test module while
  working (`cargo nextest run -p faraday-bip -E 'test(/^codex32::/)'`),
  `just` once at the end.
- Nothing cryptographic lands without its row of §13.5 passing.
- Fixtures copied from other projects keep their licence notice and are
  recorded with commit and SHA-256 in their folder's README, as
  `tools/vectors/bip93/README.md` is.
- No path in a doc that does not exist; new files are named in plain
  text until they exist.
- Never cite the private board; never name a person who is not the
  public author of a standard or a tool.

### 15.2 Batches

| Batch | What | Model | Estimate |
|---|---|---|---|
| C1 | `faraday-bip` codex32 folder: worksheet rows, symbols, fresh sets, any k and n; fixtures (volvelle cells, booklet tables, Codex32QR vectors and pictures); F79 cross-check hooks; D1 to D4 recorded | Opus | 1.5 days |
| C2 | The decoder of §8.4, the correction corpus, its cross-check | Opus | 2 days |
| C3 | The booklet dice method in `faraday-entropy`, its reference page | Sonnet | 0.5 day |
| C4 | `faraday-keys`: the work set, seed bytes for `add_seed` keys, `codex32_shares` for any k, n and wordless keys; lint-secrets | Sonnet | 1 day |
| C5 | The main flow (§6, §7): cards, the live entry screen (dice sheet, checksum and translation worksheets filling per press, at three size classes), the cell sheet, review cards, strings, Strings made, check on another device, Codex32QR shown; snapshot tours at each class | Opus | 5 days |
| C6 | §8.1 to §8.3 and §8.5: hand-made shares, add a share, check a string, case and duplicates; scanning (§10.1) and its tests | Sonnet | 1.5 days |
| C7 | §8.4's screens: the tick, proposals, re-entry, the lines on the fingerprint | Opus | 1.5 days |
| C8 | §9: copies by hand, the plan's codex32 form, its checklist item, the Templates text | Opus | 2 days |
| C9 | §11: the Starter's codex32 templates and worksheets | Sonnet | 1.5 days |
| C11 | §5.5: the codex32 roller on Faraday Desktop: the die pictures into the assets, five dice, the click choices, the tumble, the pause and Reduce motion, the honesty labels, and tests that a roll fills the sheet as the pad would and that its values are marked as this computer's | Sonnet | 1.5 days |
| C12 | §5.6: the one-die roller for New key's dice and Generate with dice, Rolls per click, the labels; FLOWS and DESIGN edits | Sonnet | 1 day |
| C10 | §12 Demo, Learn, the edit list (§15.3), then this file's plan to docs/history/CODEX32-LOG.md (new) and its Kind to Reference | Sonnet | 1 day |

About 20 days. C1 before everything; C2 before C7; C3 and C4 before C5;
C5 before C6, C8, C9 and C11; C12 stands alone; C10 last.

### 15.3 Edit list

| Document | Section | Change |
|---|---|---|
| `docs/FLOWS.md` | New key and Add a key | Start's top level: BIP-39 words · Codex32 under Make a new key, the codex32 and Codex32QR ways under Add a key you have, SLIP-39 and Seed XOR left under Other forms; the form's Add a share, Check a string and Suggest corrections; Scan a SeedQR taking a codex32 string |
| `docs/FLOWS.md` | Backups, Backup flow, Wallets tab | the one codex32 choice; the plan's codex32 form and item |
| `docs/DESIGN.md` | §4.3 Entry | the codex32 keyboard's numbered boxes, `?` with the tick |
| `docs/DESIGN.md` | §4.10, §5 Pad, Secret, Result, Choice | the worksheet as a Secret panel; the booklet dice pad; String · Codex32QR; Strings made as built; the Scheme choices |
| `docs/FLOWS.md` | New key and Add a key; Vaults (Create a vault) | §5.6: the roller on Faraday Desktop for dice words and Generate with dice |
| `docs/DESIGN.md` | §5 Pad; §4.10 | the dice row and the one die, Rolls per click, "Rolled by this computer"; the masking exception of §6.4.6 |
| `docs/VAULT.md` | §7, type 2 | the key's codex32 set field, plain facts only (§9.4); a vector with it |
| `docs/VAULT.md` | §7, new type 15 | the codex32 share record (§9.5); a vector with it |
| `docs/BACKUP-FLOW.md` | §17.1, §17.3, §17.5, §17.21 | the codex32 seed group and its string chips, the forms per zone, the checks, the preset (§9.5) |
| `docs/BACKUP-FLOW.md` | chart, §17 | the set's node actions: Add a share, Replace a lost share, Rotate the set (§9.7) |
| `docs/BACKUP-FLOW.md` | §17.3, the checklist | a key with a codex32 set: the form fixed to the set, Check my copy per string, no Make them (§9.4) |
| `docs/FLOWS.md` | Make a new wallet | the Kind card's Single key, backup in shares; its Quorum as the scheme; its slot opening Make codex32 shares; no codex32 in other kinds' slots (§6.9) |
| `docs/DESIGN.md` | §5 Choice, "What kind of wallet?" | the new kind's row and its second line; the rows' second lines (§6.9) |
| `docs/DESIGN.md` | §4.10 | the worksheet's look, §7.5: the booklet's colour roles as named fills and outlines per theme |
| `docs/DESIGN.md` | §4 (new component) | the tab row: one tab per item, a check mark once shown, wrapping on `wide` and `mobile`, a Choice with Previous and Next on `small` (§6.5) |
| `docs/DEMO.md` | §5.2 | the roller in the demo: Generate with dice and New key unchanged in outcome |
| `docs/STARTER.md` | §3.8 | the Form row and §11 |
| `docs/BACKUP-FLOW.md` | §10.2, §12, §17.3, §17.6 | the Templates text; strings; the codex32 toggle and `What::Codex32Share`; presets unchanged |
| `docs/KEYS.md` | §2, §3 | the work set as a new secret; seed bytes kept for `add_seed` keys; `PaperForm` covering both |
| `docs/QR.md` | §1 Payloads read, §2 | Codex32QR read and shown |
| `docs/INTEROP.md` | §2 | a row: Codex32QR/v1-48, read and written, tested |
| `docs/DEMO.md` | §4, §5.3, §6 | §12's rows |
| `docs/MODES.md` | §7 | the samples for §6 and §8 |
| `docs/AUDIT.md`, `core/faraday-bip/README.md` | the faraday-bip row | worksheets and correction, their vectors |
| `docs/CROSSCHECK.md` | §4 | codex32 sets and correction, python-codex32 |
| `docs/learn/19-other-backups.md` | Codex32 | making shares here and by hand, the booklet, lengths, checking on another device, corrections; a link to the codex32 user flow guide, https://github.com/FractalEncrypt/FractalEncrypt_seedsigner/blob/codex/codex32-error-correction/docs/codex32_qr/Codex32_User_Flow_Guide.md, as the guide to read beside it (owner, 2026-10-23) |
| `docs/learn/15-glossary.md` | Codex32 | identifier, share index, translation symbol |
| `docs/README.md` | Spec list | the status as built |

### 15.4 Questions for the owner

None open: the owner answered every question on 2026-10-22 (§1.1).
