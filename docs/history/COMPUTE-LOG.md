# Compute: costly work takes a token: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/COMPUTE.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## The header (from docs/COMPUTE.md)

**Kind:** Reference
**Status:** Built

2026-10-21 · built (U4 to D3, 2026-10-21); owner approved the
approach 2026-10-20 (F69)

## The opening: before the token (from docs/COMPUTE.md)

Before the token, two values move into core: the fingerprint and the
checksum (§5). Each core change is its own commit. Nothing here
loosens a check.

## §5.4 What changes for imported wallets (from docs/COMPUTE.md)

Wallets it made already write `h`. Wallets imported with `'` now export
`h` and show a different checksum. Faraday's stored records were already `h`. Tests whose
expectations change from `'` to `h`: `faraday-bip` (`policy.rs` units,
`coldcard.rs`, `multisig_config.rs`, `tests/all/{descriptor,bsms,musig,
recovery,silent}.rs`), `faraday-psbt` (`tests/all/multisig.rs`, `musig.rs`,
`tests/all/common/builder.rs`); reading tests keep their `'` inputs.

## §6 Batches (from docs/COMPUTE.md)

Each is one implementer; `just` green at the end of each; each core
change its own commit.

| Batch | Content | After | Alongside | Model |
|---|---|---|---|---|
| U4a | §5.1 | — | U4b | Sonnet |
| U4b | §5.2, then §5.3–5.4 (two commits), DECISIONS F66 | — | U4a | Opus |
| A0 | `View`, `After`, `before_frame`; the 11 `&mut Faraday` draw roots take `&View`; `osk_track` leaves drawing; the guard stays | U4 | — | Opus |
| A1 | Memo, kept and allowed accessors on `View`, beside the old forms; decisions switch to `*_now` | A0 | — | Opus |
| B1 | `screens/` | A1 | B2–B6, C0 | Sonnet |
| B2 | `vault/screen.rs`, `vault/load_screen.rs`, `glance/sheet.rs`, `glance.rs` | A1 | ″ | Sonnet |
| B3 | `backup_flow/screen.rs`, `backup_flow/wallet_screen.rs`, `board/screen.rs`, `plan/cards.rs`, `backups/screen.rs` | A1 | ″ | Sonnet |
| B4 | `family/screen.rs`, `family/text.rs`, `keygen/screen.rs`, `seeds/screen.rs`, `codes/screen.rs`, `bip85/screen.rs` | A1 | ″ | Sonnet |
| B5 | `compact.rs`, `compact/screen.rs`, `boot_import/screen.rs`, `in_progress.rs`, `stick/pills.rs`, `stick/wipe.rs`, `upgrade/screen.rs`, `transfer/screen.rs`, `faraday/faraday-core/src/tools/screen.rs` | A1 | ″ | Sonnet |
| B6 | the rest (`explore_screen`, `silent_screen`, `vanity_screen`, `lightning_screen`, `wordlist_screen`, `file_menu`, `edit`, …) | A1 | ″ | Sonnet |
| C0 | `Compute` and `unchecked`; `Faraday::compute`; `View`'s token; the mint sites; `faraday/clippy.toml` with the two mint entries; the global counter stays | A1 | B1–B6 | Opus |
| C1 | Gate `WordsParse`, `Kdf`, `MasterKey`, `KeyDerive`; their counts move to `c.count` | B, C0 | — | Sonnet |
| C2 | Gate `DescriptorParse`, `Address`, `WalletMatch`, `PlanMap`; `faraday-intake`'s miniscript call moves into `faraday-bip` | C1 | D1 drafting | Sonnet |
| C3 | Gate PSBT, QR, `PngDecode`, `Picture`, `FileHash`; the Starter's QR kept on its state; remove `set_counter`, the `OnceLock`, the `count` feature; `Canvas::set_counter`; drop the old memo forms | C2 | D1 drafting | Sonnet |
| D1 | Costs lint (§8), compile_fail doctests (§9), full `faraday/clippy.toml` (§7), in `just lint` | C3 | — | Opus |
| D2 | Remove the guard (§10) | D1 | D3 | Sonnet |
| D3 | Docs (§11) | D2 | — | Sonnet |

C1–C3 run in sequence (they share ~700 call sites). Each C batch updates
`fuzz/` and `faraday/fuzz/` for what it gates. When a count moves to its
source the batch re-derives `moments.rs` and `presses.rs` and says why
each number moved; no frame budget may rise from none. The guard stayed
active through every C batch.

Built (2026-10-21):

- **D1.** The costs lint, the compile_fail doctests, the full
  `faraday/clippy.toml` with `tools/clippy.py`, and new budgets (each
  kept accessor, allocations and lines a frame, each typed character,
  the Pi's panel, a long session's kept lists; `docs/SPEED.md` §8.3).
  Found: `View::item_hash` hashed a file on every read when it had no
  hash from intake; three sheets stretched a key on every character
  typed (now after a 400 ms pause, `docs/SPEED.md` §9 item 2); four
  kept lists had no bound.
- **D2.** The debug guard removed (§10); the token, the costs lint, the
  doctests and the budget tests hold the rule.
- **D3.** These docs: `docs/SPEED.md` §3.1 to §3.3, §8, §9; CLAUDE.md;
  DECISIONS F66 and F69. Found: §3, §7, §9 and
  §12 corrected to what D1 built.

## §10 When the debug guard is removed (from docs/COMPUTE.md)

In D2, the commit after D1, green on top of C3: every counted kind but
the two pixel kinds requires `&Compute`, the only token drawing can
reach is `View`'s (used only by the guard's own three exceptions), and the
lint and doctests hold the rest. Removed from `faraday-core/src/work.rs`:
`Frame`, `DRAWING`, `FILLING`, `filling`, `keeping`, `allowed`, `callers`,
the panic in `count`, and `PIXELS` if nothing reads it; `render`'s
`work::Frame::begin()`. The `Allowed` enum moves to `drawing/allowed.rs`
as the owner's list. Kept: `count`, `Counts`, `snapshot`, `since`,
`KINDS`, `of_core`, `core_count`, `work/budget/`, `work/kept.rs`, and
everything parse-once keeps.

## §11 Docs (from docs/COMPUTE.md)

`docs/SPEED.md` §3.1–§3.2 (core keeps fingerprint and checksum), §3.3
("The guard" becomes "The token"), §8.2 (new sources); CLAUDE.md's
"Drawing reads" bullet (the token replaces the guard sentence);
`docs/DECISIONS.md` F66 (`h`, and
the token).

## §12 Risks: item 1 (from docs/COMPUTE.md)

1. Merge conflicts: ~700 call sites, mostly tests; C1–C3 in sequence, B in
   parallel by disjoint files.

## §12 Risks: items 6 to 8 (from docs/COMPUTE.md)

6. `View` grows to ~37 thin accessors in `drawing/`, each saying what it
   reads.
7. Count tables shift as counts move to their source; batches explain
   each change.
8. Rustdoc on this toolchain does not check pinned error codes (found
   in D1). The compiling twin beside each `compile_fail` doctest is the
   real guard: it catches a doctest failing for the wrong reason (§9).

## §13 Build time (from docs/COMPUTE.md)

Runtime nil (one pointer; one indirect call inside microsecond work).
Costs lint ~10 s clean, ~1 s after. Doctests: ~15 s more for `cargo test
--doc`. Removing the guard makes tests marginally faster.

## §14 Estimate (from docs/COMPUTE.md)

U4a 0.5, U4b 1, A0 1, A1 1, B1–B6 3 (about 1 elapsed), C0 0.5, C1 1, C2 1,
C3 1.5, D1 1, D2+D3 0.5 implementer days: about 12 of work, about 8
elapsed with the parallel batches.
