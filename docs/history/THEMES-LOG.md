# Themes build log: the plan, the edit list and the owner's answers, as written at the time

**Kind:** History
**Status:** Kept as written; section numbers are the spec's when it was written. The current document is `docs/THEMES.md`.

Sections moved out of `docs/THEMES.md` on 2026-10-24, when C4, the last batch, landed. Each keeps its original number; the source keeps a one-line stub under the same heading. C4 built the theme tour (each theme in Light and Dark at Standard contrast, Cobalt at Medium and High, on Home, Wallets, the backup board, a codex32 worksheet and Settings › Appearance) and the docs of §16.

## 15. Plan

The batch that builds the last of it moves this plan to a THEMES-LOG
in `docs/history/` and makes this document a Reference.

| Batch | What | Model | Estimate |
|---|---|---|---|
| C1 | The reference vendored with its README and digests; the resolve shim; the generator and `just themes`; schemes.rs; `pal::Role`, the paint, `Theme` as (theme, mode, contrast); today's 39 names as paints for their main role; state and disabled tokens (§8); the tests of §11.1 to §11.3; tools/README.md rows; Node on the build machine confirmed. Timed in a late-flow session. | Opus | one long session |
| C2 | Every screen and component moved from the 39 names onto roles by name, by §6 and §7, file owner by file owner: `faraday-ui` first (buttons, chips, segments, fields, cards, rail, bar, sheets, dialogs, toast, focus ring, disabled), then `faraday-core`'s areas, then the Starter; the old names removed; the high contrast rules of §9 keyed to Contrast High. About 3,600 uses in about 100 files. | Sonnet | three to four sessions, split by crate |
| C3 | Settings › Appearance (§12); the settings file's three lines and the migration (§13) with its tests; the strings in `faraday/faraday-strings/src/en.rs` (ten names, Mode, Contrast and their values; the retired names and the group labels removed); the Starter (§14); folded in by the owner, the Reduce motion switch and the selected state of §12. Built 2026-10-24. | Opus | one session |
| C4 | The theme tour (`faraday/shells/snapshot/src/tours/themes.rs`): each theme Light and Dark at Standard, and Cobalt at Medium and High, not all sixty; renders checked by the owner; the docs of §16. | Sonnet | one session |

C2 landed after C1 (`faraday-ui`, then `faraday-core`, then the Starter);
C3 follows. DESIGN §3 is rewritten from this spec.

## 16. Edit list

Docs that change, and in which batch:

- `docs/DESIGN.md` §3 (C2): the themes, the colour roles, the states and
  the high contrast paragraphs rewritten from this spec; its pointer line
  to this spec removed.
- `docs/decisions/F91.md` and `docs/decisions/F108.md` (C1, at landing):
  marked amended by the new decision; the decision
  numbered F113 (`docs/decisions/F113.md`) and indexed in
  `docs/DECISIONS.md`.
- `docs/STANDARDS.md` (C1): a row for the colour method (Material 3's
  colour system; ten seeds × two modes × three contrasts, no custom seed;
  verified by material-color-utilities 0.4.0 regenerating the tables and
  by Material's contrast rule; Neutral), in a new §9, Display, since no
  table there covers it and sections are not renumbered.
- `docs/MOTION.md` §3.5, §3.6 and §3.7 (C4): the theme fade, Light
  mode and the named themes as built.
- `docs/GUIDE.md` (C4): a new §2.4, Settings › Appearance; the guide
  has no appearance section today.
- `docs/TEXT-SIZE.md` (C4): "under the themes" becomes under Contrast.
- `tools/README.md` (C1): the generator, `just themes`, lint-themes.sh
  and the reference's row.
- `docs/README.md` (C4): this spec moves to Reference once nothing is
  open but the follow-on and the picker.
- Follow-on, after C4: the website, the user guide's pictures
  (`docs/guide/`) and the README's (`docs/screenshots/`) redrawn.

## 17. The owner's answers (2026-10-24)

The questions this spec put to the owner, and what was decided:

1. **Cobalt's style**: fidelity, seed #0047AB. The exact cobalt stays
   in every Cobalt scheme as primary-container (§2).
2. **Bitcoin Orange's filled button**: Material's scheme as it comes. A
   filled button is primary, #8C4F00 in Light and #FFB874 in Dark, and
   the orange itself, #F7931A, is primary-container. No recorded
   difference.
3. **The sidebar's shade**: surface-container-low, a slight shade apart
   from the page (§6).
4. **Status hues near a primary**: Warning is not harmonised in
   Lamplight and Bitcoin Orange, and OK is not harmonised in Forest.
   Everywhere else both are harmonised (§5). All sixty schemes were
   regenerated with these exceptions and still meet §11.1 and §11.2. In
   Lamplight and Forest the source hue is already next to the primary's,
   so icons and words still tell the two apart (§5).
5. **The Starter**: it gets the same three controls, Theme, Mode and
   Contrast (§14).
6. **State layers**: focus 0.10 and pressed 0.10, per Material 3's
   current guidance. Material Web's tokens (`v0_192`) give 0.12, and
   Faraday follows the guidance (§8).
7. **Status colours**: OK and Warning as §5 gives them ("the colors are fine").
