# The Spend tab: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/SPEND-FROM-BACKUP.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## Header (from docs/SPEND-FROM-BACKUP.md)

V0.3 · 2026-10-11 (page 3 and the guessed Check,
`docs/history/SCAN-TO-SIGN.md` §4 and §5) · built: `faraday-core/src/family/mod.rs`
(state and logic), `family/screen.rs`, `family/text.rs`,
`tests/all/family.rs`, and the snapshot tool's `spend` tour
(`cargo run -p faraday-snapshot -- 1366x768 out/testkit OUT spend`).
The code names it `family`; the tab is labelled **Spend**.

## §5 Building it (from docs/SPEND-FROM-BACKUP.md)

| Part | Where |
|---|---|
| `Screen::Family`, the sidebar row, `Action::Family(FamilyAction)` | `faraday-core/src/lib.rs`, `faraday-core/src/screens/mod.rs`, `faraday-core/src/screens/sidebar.rs` |
| `FamilyState { route, open }`; which pages apply; each page's done line from app state | new `faraday-core/src/family/mod.rs` |
| Drawing: the column, the map, the answer tiles, the More folds | new `faraday-core/src/family/screen.rs`, on `flow::column` |
| The text of §4 | new `faraday-core/src/family/text.rs`, one file, like `guide.rs` |
| Unlock's controls without its title and back link, called by Unlock and by page 3 | `vault/screen.rs`: `unlock()` split into `unlock_body()` |
| Pages 8, 10, 11 call Wallets' `step_body` for the spend's own steps; while one is open the Family card follows `spend.open` | `faraday-core/src/screens/spend.rs` |
| Word entry, Restore and the stick visit return to Family when opened from it (as `after_visit` does for Create) | `lib.rs` |
| Route and page kept across a lock; after Unlock, Family opens if it was open | `memory.rs` `kept()` / `restore_kept()` |
| The words route's single-key wallet from the loaded key | `wallet.rs`, the descriptor Create builds for a single key |

No file in `core/` is edited; no new dependency.

**Tests** (`tests/all/family.rs`, user-facing behaviour, on the test kit):
- Vault route end to end: the test stick's `vault.ofv` and passphrase
  `test vault` → the wallet opens → a spend from Files → signed → finished,
  with the same signature bytes as Wallets › Sign.
- Paper route: split sheets in any order → wallet → seeds → signed; a seed
  from another wallet refused.
- Words route: the test words give the test kit's first address.
- A lock on page 7 (stick route) comes back on page 7 after Unlock, with
  the PSBT copied in loaded.
- On the desktop app, page 2 shows the stop.
- The review's warnings are the same text in Family as in Wallets.

The snapshot tour gains the Family pages at one size.
