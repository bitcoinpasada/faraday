# Compare two PSBTs, and the fee bump: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/COMPARE.md` on 2026-11-10. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## 5.1 and 5.2 (from docs/COMPARE.md §5)

### 5.1 In core/faraday-psbt (added; what it accepts is unchanged)

Two modules, both pure, both reading inspections made by the caller, so
neither costs more than walking the two transactions:

- **core/faraday-psbt/src/diff.rs (new), `faraday_psbt::diff`.**
  `diff::list(first: Side<'_>, second: Side<'_>) -> Differences`, where
  `Side { psbt: &Psbt, inspection: &Inspection, psbt_version: Option<u8> }`
  (`None` for a finished transaction). `Differences` holds the two
  txids, the facts of §2.7 as numbers, the inputs and outputs paired by
  §2.4 (each pair with both indexes), and the differences as typed
  values, one variant per id of §2.5. It returns no text.
- **core/faraday-psbt/src/bump.rs (new), `faraday_psbt::bump`.**
  `bump::judge(original: Side<'_>, replacement: Side<'_>, loaded:
  &[Fingerprint]) -> Option<Bump>`: `None` where §3.1 says it does not
  apply; otherwise `Bump { result, lines, fees, added_inputs, signalled
  }`, `result` one of `OnlyFee`, `MoreChanged`, `CannotTell`, and the
  lines typed, one variant per id of §3.4, in the rules' order, the
  fingerprints U2 and U3 name carried for **Load {fp}**. It uses
  `diff`'s pairing, so the two never pair outputs differently.
  `bump::fill_amounts(original: &mut Psbt, from: &Psbt)` sets the
  `witness_utxo` of each original input that states no amount from the
  input of `from` spending the same outpoint (I4); the caller runs it
  before inspecting the original.
- `lib.rs` declares both modules. No existing function changes; the
  inspection already gives every value the rules read.

### 5.2 In faraday-core

- **faraday/faraday-core/src/compare/ (new), the area's folder**
  (`docs/STRUCTURE-PASS.md` §7.1): `mod.rs`, the flow's state
  (`CompareState`, the two sides as Check a signer's `Side` keeps them,
  the result, its words, what it was made with), its presses
  (`ComparePress`, as `SignerPress`) and the logic that needs no app;
  `screen.rs`, the cards; `words.rs`, every difference's, line's and
  fact's text from the typed values, in exhaustive matches, the fee
  bump's included, which the review uses too; and `others.rs`, §4.1's
  rule over Files and the making of the review's results.
- **Its state lives in the decode area,** as Check a signer's does: the
  decode area's struct gains a private `compare` field with its
  accessors, and the presses that need the app (`compare_act`, the
  sync on a key loaded or dropped) are in
  `faraday/faraday-core/src/decode/mod.rs`'s `impl Faraday`. No field
  is added to `Faraday` and no file gains an `impl Faraday`
  (`tools/lint-layout.counts`). `raw_as_psbt` and `amounts_in` there
  become `pub(crate)` for the compare area.
- **lib.rs's glue:** `mod compare;`, `Screen::Compare` and its row in
  the code map's table (`compare/screen.rs`), the `Action::Compare`
  arm sent to `decode_act` beside Check a signer's. lib.rs is over
  5,000 lines and its count only falls (F196), so the same change moves
  `Item` and its `impl` (with the new outpoints field) out of lib.rs
  into faraday/faraday-core/src/item.rs, re-exported as `crate::Item`,
  and lowers lib.rs's line in `tools/lint-layout.counts`.
- **The review:** `SpendState` (`faraday/faraday-core/src/wallet.rs`)
  gains the kept results and what they were made from; `refresh_spend`
  calls `compare::others` once (§4.3); `screens/spend.rs` draws them on
  the Transaction step.
- **Files' ⋮:** `faraday/faraday-core/src/file_menu.rs` gains
  **Compare** on a PSBT's and a finished transaction's row.
- **Tools:** `catalog.rs` gains the tile (§2.1).
- **Practice:** both cards' scanners offer a sample pair, the Core
  original and its replacement (§6.1), as Check a signer's offer
  theirs (`docs/MODES.md` §5.3, `faraday/faraday-core/src/samples/`).
- **Words:** every string in `faraday/faraday-strings/src/en.rs`.


## 6.2, last item (from docs/COMPARE.md §6.2)

- Learn: docs/learn/06-verifying.md gains a section on comparing two
  PSBTs and a fee bump, the page the ? opens; the guide gains
  docs/guide/screens/compare.md and the review's line on
  `docs/guide/screens/sign.md`.


## 8. What comes first, and which files change


The two modules in `core/faraday-psbt` come first: everything else reads
their types. They change core/faraday-psbt/src/diff.rs and bump.rs (new),
`core/faraday-psbt/src/lib.rs`, and the tests diff.rs and bump.rs under
`core/faraday-psbt/tests/all/` with `core/faraday-psbt/tests/all/main.rs`.
The Core fixtures are separate and can be made beside them: the script
tools/scripts/bump-core.py (new), the bump-core-* files and
`tools/vectors/psbt/README.md`; the written fixtures and the Core pair's
test follow both.

Then the shared part of faraday-core, before the two screens: `Item`
moved to faraday/faraday-core/src/item.rs with its outpoints
(`faraday/faraday-core/src/lib.rs`, `faraday/faraday-core/src/wallet.rs`'s
`classify_read`, `tools/lint-layout.counts`), the compare folder's
`mod.rs` shell, `words.rs` and `others.rs`, `raw_as_psbt` and
`amounts_in` made `pub(crate)` in `faraday/faraday-core/src/decode/mod.rs`,
and the fee bump's strings in `faraday/faraday-strings/src/en.rs`.

After it, two pieces in parallel. The Compare flow: compare/mod.rs's
flow and compare/screen.rs, `faraday/faraday-core/src/decode/mod.rs`
(the field, accessors and glue), `faraday/faraday-core/src/lib.rs`
(the screen, the dispatch arm, the code map), `faraday/faraday-core/src/action.rs`,
`faraday/faraday-core/src/catalog.rs`, `faraday/faraday-core/src/file_menu.rs`,
the samples, Compare's strings in `en.rs`, and its faraday-core tests. The
review: `faraday/faraday-core/src/wallet.rs` (`SpendState`,
`refresh_spend`), `faraday/faraday-core/src/screens/spend.rs`, the
review's strings in `en.rs`, its tests and the budget test. Both add
to `en.rs`, each in its own group, and the review's **Compare** button
needs the flow's press, so the review lands second or adds that one line
after the flow lands. Neither touches the other's files otherwise.

Last, the documents: the Learn section (`docs/learn/06-verifying.md`
and through `tools/learn/sync.py` the Learn pages' `en.rs`), the guide's
pages, `docs/FLOWS.md`'s Files ⋮ list and Spend's review, `docs/WALLETS.md`
§7's Transactions row, the STANDARDS row (`docs/crypto/actions.toml`),
and this document made a Reference, what is only about building it moved
to docs/history/COMPARE-LOG.md.
