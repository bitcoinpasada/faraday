# The guide

The guide: the ? (Guide) button on every screen, which opens the guide at the section for the screen and step on show, as a sheet or, on an extra-large window, as a side pane at the right. It has Guide and How tabs, contents, glossary words and a Find field. Reached from the ? in the top bar or the small panel's overflow menu, from Tools' Find word hits, and from Home on the small panel's menu.

## Screens and sheets

- `Sheet::Guide`; `Sheet::GpgRestore` opens the Sheet section.
- Guide sheet drawing: `guide_sheet`, `guide_body`, contents, words, tabs (`faraday/faraday-core/src/screens/sheets.rs:~2008-2376`).
- Side pane (`faraday/faraday-core/src/screens/guide_pane.rs`): on an extra-large window the guide stands as a right pane with a close button (`GuideClose`), placed only through `hub_layout::guide_place` (`faraday/faraday-core/src/screens/guide_pane.rs:29`).

## Actions

- `Action::Guide` (the ?), `GuideTabGuide`, `GuideTabHow`, `GuideContents`, `GuideHere`, `GuidePage(i)`, `GuideWord(i)`, `GuideClose`, `TypeHere(Here::GuideFind)`.
- Find: typing, Backspace; Escape clears.
- Cancel steps back inside the guide, then closes it (`faraday/faraday-core/src/guide.rs:~760-772`).
- Escape closes the pane only when no sheet is over the page (`faraday/faraday-core/src/guide.rs:~777-785`).

## Effects

- None outward. Draw-time write: `app.guide_scroll(max, scroll, section)` (`faraday/faraday-core/src/screens/sheets.rs:~2078`).

## State and lock

- `GuideArea` (`faraday/faraday-core/src/guide.rs`) is `Public` and holds no secret: the section, tab, scroll and the Find text.
- Every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`), so the guide's place does not outlive it.

## Reach

- Opened from every screen; from Tools' word hits (`menu-pages`); from the small panel (`small-panel`).
- Placed by the pane container (`pane-container`).
- `address_check::pane::scanning` is read at `faraday/faraday-core/src/guide.rs:667`.

## Rules and refusals

- `guide_key` maps every screen (and step) to a section (`faraday/faraday-core/src/guide.rs:~46-119`).
- Find takes at most 40 characters, no control characters; Backspace deletes; Escape clears (`faraday/faraday-core/src/guide.rs:963-972`).
- The supporting pane only on an extra-large, non-compact, non-held window, and only when its place is free (`faraday/faraday-core/src/guide.rs:~684-748`).
- The How tab is offered only when the screen has blocks.
- Home counts as a first start with nothing held, no wallet, no Inbox file and no flow (`faraday/faraday-core/src/guide.rs:~645-650`).
- Nothing under the side pane takes a press (`faraday/faraday-core/src/screens/guide_pane.rs:~31-33`).
- The sheet's rules (tabs, held guide, Done) are listed in `sheets-parts`.
- Since the history fixes: a press on the contents' Find on a one-column layout focuses it and brings the keyboard up; tab names wrap at word breaks.

## Domain calls

- `faraday/faraday-guide`: the guide's pages, `glossary`, `guide_groups`, `GuidePage`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| bd3def143 | On a one-column layout a press on the guide contents' Find did not focus it or bring the keyboard up. | A field press brings the keyboard up. |
| a41f374d1 | On the small panel Learn's tab name was cut; a tab name wider than the tab did not wrap; "Sign a transaction" in the top bar was cut under Cancel. | A tab name is read whole (wraps at word breaks); a top-bar name is whole words or none. |

## Doubts

- `how_screen_name` returns "" for a variant name over 24 bytes; `how_step` filters only Spend.
- `GuideAt::Opening` looks unused.
- Row left out as not about the guide: 61f745906 (the Learn text on a word list's edit distance, and a dealt FROST share in the backup flow).
