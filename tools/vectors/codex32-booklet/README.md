# The codex32 booklet's tables

The tables of the codex32 booklet ("Shamir Secret Sharing Codex"), black
and white edition, revision of 2023-03-07, transcribed from its PDF, for
`core/faraday-bip/tests/all/codex32_sheets.rs` and the crypto
cross-check's codex32 area (`tools/crosscheck/area_codex32.py`).

Source: `https://secretcodex32.com/docs/2023-03-07--bw.pdf`, listed on
`https://secretcodex32.com/docs/index.html`; its PostScript source is
`https://secretcodex32.com/docs/2023-03-07--bw.ps`, and the booklet's
repository is `https://github.com/roconnor-blockstream/SSS32`. Licence:
MIT, "Copyright (c) 2022 Blockstream", stated inside the PostScript and
printed on the booklet's second page; the notice is `LICENSE` here,
copied from the PostScript's `/MIT` array. Downloaded 2026-10-23.

| File | SHA-256 |
|---|---|
| `2023-03-07--bw.pdf` (not kept here) | `0370ea863d2eae692408aeefa9b13c14283e520f45a00f7373ad933ccf418f2e` |
| `2023-03-07--bw.ps` (not kept here) | `c12cc875c16667bed1f022c94f493263f5501af91081bcf797209dfbb2d77219` |
| `tables.txt` | `87093bf36e8c85a87012673490645f9fe9e4a225ebf5d398d1e615ac18e82640` |
| `extract.py` (written here) | `cd619dabd42552531fbb3300c2fab803c257885285ed1e6c82670489496a631c` |
| `tables.json` | `79fcd9e81b236b0ca0f793eeb4e8852ff05ed7ee3ffa080cce5c9fed21032b9c` |
| `LICENSE` | `cbfbfc11c35745d9e634085cccf1637534cb48f2d8627a77a5ff431121615840` |

`tables.txt` is `pdftotext -layout` (poppler 26.08) of the PDF's pages
4, 16, 22, 23, 48 and 49, keeping only the lines of the tables (their
headings, column heads and rows) under a `=== page N ===` line for each
page; the prose between them is left out. The pages are:

- page 4, the Principal Tables: Addition, Translation, Recovery and
  Fusion;
- page 16 (booklet page 9), III.1.B: the derivation tables for k = 2
  (initial shares A, C) and k = 3 (A, C, D);
- pages 22 and 23 (booklet pages 15 and 16): the codex32 checksum
  table, the 13 symbols each of the 1,024 leading pairs selects;
- pages 48 and 49 (Module 2): the derivation tables for k = 4 to 8 from
  A, C, D, …, and for k = 2 to 8 from S, A, C, …, the case where the
  secret is an initial share.

`extract.py` reads `tables.txt` into `tables.json`, checking every row's
length against its column heads:

    cd tools/vectors/codex32-booklet && python3 -I extract.py > tables.json

pdftotext writes the Symbol font's Delta and mu as U+2206 and U+00B5;
`extract.py` writes them as the Greek letters U+0394 and U+03BC, the
names the booklet's list of symbols gives them. Every other glyph is as
pdftotext wrote it; the booklet's diamond is U+2666.

The booklet's second alphabet, the glyph of each symbol value from 0 to
31, is its PostScript's `code2` array (line 1011 of the `.ps` above):
`/multiply /aleph /alpha /beta /Gamma /Delta /epsilon /eta /Theta /Lambda
/mu /Xi /Pi /rho /Sigma /Phi /Psi /Omega /at /numbersign /percent /cent
/yen /Euro /currency /circleplus /dagger /daggerdbl /section /paragraph
/diamond /heart`. `faraday_bip::codex32::SYMBOL_NAMES` is that list, and
the tests check `SYMBOL_GLYPHS` against the Fusion table's column heads.

Every cell of every table here was checked against BIP 93's arithmetic
(python-codex32's `bech32_lagrange`, `bech32_mul` and checksum) when it
was transcribed; the cross-check does so on every run.
