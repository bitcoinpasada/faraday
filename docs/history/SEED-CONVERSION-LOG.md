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
