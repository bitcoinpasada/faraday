# Guide pages and flow text

The read-only text components of the component layer (`faraday-ui`): the
guide's block lists (what each action does, which standards, how it is
implemented and checked, cross-checks, limits), construction documents with
sections and tables, the flow paragraph of bold and plain runs, and the
demo items a page hands to Faraday Demo's host. They are reached from the
guide pane, About and the Starter's Cryptography sheet (`PageLook::SHEET`),
and from any flow page that writes a paragraph.

## Screens and sheets

All `faraday/faraday-ui/src/`:
- `how::blocks`: a list of guide blocks, OutlineVariant dividers between, constructions after their actions (`faraday/faraday-ui/src/how.rs:~28-51`).
- `block`: title (TITLE_S) and fields action, standards, implementation, checked, cross-check, code, secret, limits, own calls (`faraday/faraday-ui/src/how.rs:~115-165`); `field` (headline BODY_L, entries BODY_M, `faraday/faraday-ui/src/how.rs:~168-177`); `item` (`LIST_PAD_V` padding, headline BODY_L OnSurface, value under it, `faraday/faraday-ui/src/how.rs:~296-306`); `entries` (`faraday/faraday-ui/src/how.rs:~309-322`); `line` Plain (BODY_M OnSurfaceVariant), Mono (BODY_M mono), Warn (`Icon::Warning` in the Warning role) (`faraday/faraday-ui/src/how.rs:~328-352`).
- `cross_check`: independence lines (F160), then a warning line when not checked, or the area (`faraday/faraday-ui/src/how.rs:~183-206`); `area`: area name, six part names with entries, last-run line (`faraday/faraday-ui/src/how.rs:~209-249`).
- `construction` / `construction_parts`: title, sections with Primary headings, paragraphs and tables (`faraday/faraday-ui/src/how.rs:~386-435`); `table`: heads TITLE_S OnSurfaceVariant, rows BODY_M, OutlineVariant divider under the heads and each row (`faraday/faraday-ui/src/how.rs:~454-485`); `shared` column widths (`faraday/faraday-ui/src/how.rs:~490-504`).
- `PageLook` and `PageLook::SHEET`: title, heading and body roles and room after each; `SHEET` is the Starter's Cryptography sheet (`faraday/faraday-ui/src/how.rs:~357-377`).
- `Flow`: a laid-out paragraph of bold and plain runs, bold in the medium face (`faraday/faraday-ui/src/flow.rs:~28-61`); `lay_out` (`faraday/faraday-ui/src/flow.rs:~69-139`); `word_flow` (`faraday/faraday-ui/src/flow.rs:~144-151`).
- `DemoItem` Button(label, action) or Line(text) (`faraday/faraday-ui/src/demo.rs:~12-18`).

## Actions

- Guide blocks are read, not pressed (`faraday/faraday-ui/src/how.rs:~6-7`, `faraday/faraday-ui/src/how.rs:~381`).
- A demo Button item gives its action through Faraday Demo's host.

## Effects

- Demo items leave the page for Faraday Demo's host strip: `take_demo` empties `Ui.demo` (`faraday/faraday-ui/src/demo.rs:23-48`).

## State and lock

- Two process-wide caches, the only kept state in faraday-ui: `KEPT` in `cooked` (formatted code entries and last-run line by block id and release, `faraday/faraday-ui/src/how.rs:265`) and `KEPT` in `construction_heights` (part heights by construction id, `faraday/faraday-ui/src/how.rs:440`), both `Mutex`. Guide text only, no secrets.
- `Flow` is kept by the caller (lines, line height, role).
- `Ui.demo` is per frame: `demo_offer` and `demo_line` push to it; `take_demo` takes all.

## Reach

- Guide pane, About, the Starter's Cryptography sheet; flow pages (`word_flow`).
- Demo items are used in `faraday/faraday-core/src/demo.rs`, `faraday/faraday-core/src/screens/entry.rs`, `faraday/faraday-core/src/screens/backup.rs`, `faraday/faraday-core/src/screens/sheets.rs` and others, and read by Faraday Demo's host.
- Uses `ui-text` wraps, `ui-regions` clip (a part wholly outside the clip is measured only).

## Rules and refusals

- Guide items are as tall as their text, never cut (`faraday/faraday-ui/src/how.rs:~4-5`).
- A block's height is kept per width and line heights; a part wholly outside the clip returns its kept height and is not drawn (`kept_block`/`kept`, `faraday/faraday-ui/src/how.rs:~53-111`; key at `faraday/faraday-ui/src/how.rs:58`). A text-size change changes the line heights and so the key.
- `cooked` is keyed by (block id, release) (`faraday/faraday-ui/src/how.rs:268-271`).
- Table columns: natural widths where all fit, else narrow columns keep theirs and the rest share (`faraday/faraday-ui/src/how.rs:~490-504`).
- Flow draws only the lines in view (`faraday/faraday-ui/src/flow.rs:~46-48`); `lay_out` word-wraps spans at width `w`, one run per face per line.
- Demo items: each action is offered once; each line once by its text (`faraday/faraday-ui/src/demo.rs:~23-43`).

## Domain calls

- `faraday_draw::tokens::{TITLE_S, TITLE_M, BODY_L, BODY_M, LIST_PAD_V, PAD, GAP, GAP_SMALL, ICON_GLYPH_SMALL, TypeRole::medium, TypeRole::mono}` (`core/faraday-draw/src/tokens.rs`).
- `faraday_draw::text::wrap`, `width` (`core/faraday-draw/src/text.rs`).
- `faraday_draw::widgets::icon::Icon::Warning` (`core/faraday-draw/src/widgets/icon.rs`).

## Fixtures

None.

## Fixes

None given to this feature.

## Doubts

- Callers were not traced past the component crates (faraday-ui and faraday-draw); the Reach section rests on the component code and a few searches, not on a read of every screen.
- The guide caches are not keyed by language; strings are English only, so whether a language change would need a new key is unclear.
- The guide's content (which blocks, from where) is outside this cluster.
