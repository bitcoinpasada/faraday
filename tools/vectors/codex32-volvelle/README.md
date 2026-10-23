# volvelle-website's checksum worksheet cells

Every cell of the codex32 checksum worksheet, as the interactive
worksheet of volvelle-website fills it, for every valid string BIP 93
and the Codex32QR profile publish at 48 and 74 characters: the lengths
the booklet prints checksum worksheets for, and the only ones volvelle's
worksheet lays out. Read by `core/faraday-bip/tests/all/codex32_sheets.rs`
and the crypto cross-check's codex32 area
(`tools/crosscheck/area_codex32.py`), which compare Faraday's worksheet
with it cell for cell.

Source: `https://github.com/apoelstra/volvelle-website`, commit
`9096b331733d2b6bfb22ae9b545c84261e2e0c72`, folder `volvelle-wasm/src/`:
`checksum_worksheet.rs`, `fe.rs` and `error.rs`, copied unchanged into
`volvelle/` with the repository's `volvelle-wasm/LICENSE` (CC0 1.0).
Fetched with `git clone` on 2026-10-23.

| File | SHA-256 |
|---|---|
| `volvelle/checksum_worksheet.rs` | `b8ff30feb342599a9a89b51141a1450146efcb996f36bb0ac4a6616b96af4f77` |
| `volvelle/fe.rs` | `0919bfa4c4b5ac34f4a591ac68bb95e11ec92fc4f36def167b8e920b8cb1dd31` |
| `volvelle/error.rs` | `578611662d53952ef6281f6f32afef1f17266c3b7329a3ab2439535f03a10e43` |
| `volvelle/LICENSE` | `7179683e8000e6bdc9bbc60d85edf0a4ac8e76f951857f54fcb775d5886f1309` |
| `strings.py` (written here) | `9dba7948cc7b95a856e1d6d10c87503f436b26950cdcb61a980cfa6a29d3c80c` |
| `strings.txt` | `5f875096d358d9e05e37c3adb0e25e97eeec339c8ea6cb0c30b63628cd329edd` |
| `harness/src/main.rs` (written here) | `fbda109ee57239a7cc02032b1a4369c38924400a2212f410eee76d32c938eb4c` |
| `cells.json` | `a506d194102a35f70cd489a148699475dddf5f2970337c4c83b2f45c02a264e1` |

How `cells.json` was made, once, offline:

    cd tools/vectors/codex32-volvelle && python3 -I strings.py > strings.txt
    cargo run --offline --manifest-path tools/vectors/codex32-volvelle/harness/Cargo.toml -- \
        tools/vectors/codex32-volvelle/strings.txt tools/vectors/codex32-volvelle/cells.json

`strings.py` lists the 42 strings (26 of 48 characters, 16 of 74) from
`tools/vectors/bip93/vectors.json` and `tools/vectors/codex32qr/vectors.json`.
`harness/` is a crate of its own, outside the workspace, with nothing
from crates.io: it builds volvelle's three files natively, with two
stand-ins for wasm-bindgen (`harness/wasm-bindgen/`, an attribute that
returns its item unchanged and an empty error type, numbered 0.2.100
because rustc refuses a crate of that name numbered below 0.2.88). For
each string it types the header and data boxes into a blank worksheet
as a person generating a checksum would, through the worksheet's own
`handle_input_change`, lets the sheet work the checksum boxes from its
fixed bottom row, reads every cell back with `cells_into_str`, and stops
unless the sheet's checksum is the string's own (it was, for all 42).

`cells.json` holds one entry a string: `string`, and `rows`, volvelle's
rows in order, each a string of its cells in upper case: the first row
(13 boxes from box 4), the residue of `MS`, then for each of the 16
steps (29 at 74 characters) a sum row (13 sums and the step's two data
boxes) and a residue row, and the final row, `SECRETSHARE32`.
