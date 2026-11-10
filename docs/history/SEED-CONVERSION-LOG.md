# Convert a seed: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/SEED-CONVERSION.md` on 2026-11-05. Each keeps
its original number; the source keeps its heading.

## 10.1 Batches (from docs/SEED-CONVERSION.md)

The labels are the ones decisions and old commits use. C1, C4 and C8 are
the batches of `docs/history/CODEX32-LOG.md`.

After `docs/CODEX32.md` C1 (the codex32 folder) and C4 (seed bytes for
keys loaded from a seed); S6 after C8 (codex32 in the backup).

| Batch | What | Model | Estimate |
|---|---|---|---|
| S1 | `faraday-bip`: SLIP-39 `recover_ems`, `split_ems`, `derive_member`; fixtures from the references for every row of §9 | Opus | 1.5 days |
| S2 | `faraday-keys`: the conversions as operations returning held results, the read-back fingerprint check, master seed hex and xprv through `Reveal::PaperForm` (Q3) | Sonnet | 1 day |
| S3 | The screen (§6) with §4.1 and §4.3; Tools tile and the two ways in | Opus | 2 days |
| S4 | §4.2 and §4.4, settings for splits and groups | Sonnet | 1.5 days |
| S5 | §4.5 (aezeed) | Sonnet | 0.5 day |
| S6 | §8 with the backup's set actions: built | Opus | 1.5 days |
| S7 | Practice (§7), Learn (`docs/learn/18-other-backups.md` gains a section on converting), DESIGN §5's row and the guide's picture: built. This file's plan to a history log of its own and its Kind to Reference wait for the copy scanned back | Sonnet | 0.5 day |

S1's fixtures cover the rows an area already runs (§9 names it); the two codex32 ↔ SLIP-39 rows (codex32 ↔ SLIP-39, SLIP-39 with passphrase → codex32) run in the `slip39` area, built with S4's core, and aezeed's come with S5.

About 8.5 days.

## 10.1 Order of building (from docs/SEED-CONVERSION.md)

Moved on 2026-11-10, when the last of the tool was built.

The conversions come after the codex32 folder and the seed bytes kept for keys loaded from a seed (`docs/CODEX32.md`); the SLIP-39 set actions of §8 come after codex32 in the backup. In order: `faraday-bip` gets SLIP-39 `recover_ems`, `split_ems` and `derive_member`, with fixtures from the references for every row of §9; `faraday-keys` gets the conversions as operations returning held results, the read-back fingerprint check, and master seed hex and xprv through `Reveal::PaperForm` (Q3); then the screen (§6) with §4.1 and §4.3, the Tools tile and the two ways in; §4.2 and §4.4 with the settings for splits and groups; §4.5 (aezeed); §8 with the backup's set actions; and last Practice (§7), Learn (`docs/learn/18-other-backups.md` gains a section on converting), DESIGN §5's row and the guide's picture.

The fixtures of the SLIP-39 functions cover the rows an area already runs (§9 names it); the two codex32 ↔ SLIP-39 rows (codex32 ↔ SLIP-39, SLIP-39 with passphrase → codex32) run in the `slip39` area, built with the fixed 1 of 1 conversion, and aezeed's come with aezeed.

## 10.2 Edit list (from docs/SEED-CONVERSION.md)

| File | Change |
|---|---|
| `core/faraday-bip/src/slip39.rs` | §8's three functions |
| `faraday/faraday-keys/src/operations.rs`, `faraday/faraday-keys/src/session.rs` | the conversions; `Reveal::PaperForm` names master seed hex and xprv (Q3) |
| `faraday/faraday-core/src/lib.rs` | `Screen::Convert`, its module in the code map |
| a new area folder in faraday-core, convert (its mod.rs and screen.rs) | the tool's state and screen |
| `faraday/faraday-core/src/screens/catalog.rs` | the tile |
| `faraday/faraday-strings/src/en.rs` | every string |
| `docs/DESIGN.md` §5, `docs/FLOWS.md`, `docs/DEMO.md` §6, `docs/CODEX32.md` §10.5 | the screen, the flow, the Practice rows, the pointer here |
| `docs/README.md` | this file in the Spec list and the By question table |
