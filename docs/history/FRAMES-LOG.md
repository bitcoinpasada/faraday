# Frames at the panel's rate: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/FRAMES.md` on 2026-11-05. Each keeps its
original number; the source keeps a one-line stub under the same heading.
The labels (FR0 to FR6b) are the ones decisions and old commits use.

## 8. Batches

Each about fifty agent turns. FR0, FR1 to FR3 and FR5 leave
`faraday/shells/stick/src/main.rs` alone; FR4b touches it in one place
(taking `Parts` for `frame`), after WORKERS' W2 has landed.

| Batch | What | Files | Review | Status |
| --- | --- | --- | --- | --- |
| FR0 | What the floor is (§7): the Vault contents and Files frames timed part by part at a one-row clip, with a probe kept outside the repository; what costs named here, with the step that removes it | `docs/FRAMES.md` | No | Built |
| FR1 | Content placed in whole pixels (`Ui::oy_px`), `Ui::region` in place of each clip and `report_scroll*` pair, the content layer taken and the stretch moving it; snapshots rendered again where an edge moved, each named in the report; the clipped-shapes test extended to bands through a shape, a glyph and a shadow | `faraday/faraday-ui/src/lib.rs`, `sheet.rs`, `menu.rs`, the 28 core files that report a region, `faraday-starter/src/draw.rs`, `core/faraday-draw/src/canvas.rs`, `docs/MOTION.md` §3.3 | No; a screen review of the tours that changed against Material's Scrolling | Built |
| FR2 | The moved frame: `moved.rs`, `out.redraw()` and `out.moved()`, `Moves::Yes` on the compact Settings page, `draw_whole_next`, the two tests of §3.2 and §3.4; `Parts::moved` and its composition; timed as SPEED §7 says, a scroll step of each screen that opted in (§3.5) | `faraday-core/src/moved.rs` (new), `lib.rs`, `motion.rs`, the screens that opt in, `faraday-ui/src/lib.rs`, `core/faraday-shell-api/src/lib.rs`, `docs/MOTION.md`, `docs/SPEED.md` §7 | No | Built for the compact Settings page only; `Parts::moved` not yet taken by a shell |
| FR2b | `Moves::Yes` on the other long screens (Files, Wallets, a wallet's page, Vault contents, the backup checklist and board, the step column, Faraday Demo's desktop Settings once the cause of its card's corner difference is found), each timed as SPEED §7 says | the screens that opt in, `docs/SPEED.md` §7 | No | Open; waits for FR2's figure on a Pi (§3.5) |
| FR3 | The Pi's panel in its own order (§5): `Depth::Rgba8888` from the driver's `fbswap`, the Presenter with no converted copy for it, the panel's `config.txt` and the README's colour line; the present tests at 32 bits in both orders; checked on the dev card | `faraday/shells/stick/src/fb.rs`, `present.rs`, `panel.rs`, `faraday/image/panels/any/screens/waveshare-28dpi/display.line`, `faraday/image/README.md`, `docs/HARDWARE.md` if it names the format | No | Open |
| FR4a | The write (§6 points 1 to 3): `changed_rows` as bands, `Panel::write_known` with one comparison, no band copy outside the arrow; `present::tests` for a caret and a clock as two bands, the panel after each change with the arrow across a band and moved away, and turned frames | `faraday/shells/stick/src/present.rs`, `panel.rs`, `fb.rs`, `docs/SPEED.md` §11.1 | No | Built |
| FR4b | The shell after a moved frame (§3.7) and §6 point 4: the Presenter moving its copies by `Parts::moved`, one buffer for `shown` and `last`; the stick takes `Parts`; `present::tests` for a moved frame (only the strip and the overlays converted, the panel showing the frame); measured on the dev card with `--timings` | `faraday/shells/stick/src/present.rs`, `panel.rs`, `main.rs` (one call), `docs/SPEED.md` §11.1 | No | Open |
| FR5 | The bands (§4): the recording mode, the barriers, the pool, the test of §4.6; WORKERS §2 rule 1 worded for the bands; the heat on the dev card (§4.5); timed as SPEED §7 says, a frame of each main screen drawn whole | `core/faraday-draw/src/canvas.rs`, `canvas/list.rs` (new), `faraday-core/src/bands.rs` (new), `lib.rs` (`render_frame`), `docs/WORKERS.md`, `docs/SPEED.md` | No | Open |
| FR6a | Two costs in the drawing path (§7): the face kept per size on the `Ui`, `Ui::wrap_lines` linear in its words | `faraday/faraday-ui/src/lib.rs`, `roles.rs`, `docs/FRAMES.md` | No | Built |
| FR6b | The rest of §7 (what is not yet found or fixed); this spec's plan moves to docs/history/FRAMES-LOG.md and it becomes a Reference | as FR0 says, `docs/FRAMES.md`, `docs/README.md` | No | Open |

FR1 comes before FR2; FR3 and FR0 can run at any time and in parallel
with FR1; FR4a needs nothing; FR4b needs FR2 and FR3; FR5 needs FR1 (the regions it bands
are drawn through `Ui::region`) and is best measured after FR2. None of
them touches keys, cryptography or what the device takes in.
