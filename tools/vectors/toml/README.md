# TOML v1.0.0 vectors

Test cases for TOML version 1.0.0 from toml-test, the TOML project's
language-agnostic test suite, for the reader of the vault's contents
(`faraday/faraday-vault/src/contents/tree.rs`, `docs/VAULT.md` §7). Run by
the tests in `faraday/faraday-vault/src/contents/tree/toml_test.rs`.

Source: <https://github.com/toml-lang/toml-test>, the `main` branch as of
2026-11-02 (commit 96fd088448ec0f4d9f061c730ed7854399c46faf). Offered
under the MIT licence (`LICENSE`); the files are unchanged.

## Taken

From `tests/files-toml-1.0.0`, the list of the suite's cases that are
TOML 1.0.0 (a case valid or invalid only in 1.1 is not in it), the `.toml`
files in the folders `table`, `inline-table`, `array`, `key` and `string`:
265 under `testdata/invalid/` and 115 under `testdata/valid/`, about 22 KB.
These are the cases for the parts of TOML the vault's names use (tables
and arrays of tables, inline tables, arrays, bare, quoted and dotted keys,
basic, literal and multi-line strings), where the rules across keys are
Faraday's own code.

| Cases | Expect | What the test checks |
|---|---|---|
| `invalid/` | refused | `tree::parse` returns a refusal (a file that is not UTF-8 is refused before it) |
| `valid/` | accepted | `tree::parse` returns a tree; none of the cases falls outside what the reader accepts |

The test also lists the valid cases the reader refuses by Faraday's own
rule, each with the rule (`OUTSIDE`); there are none.

## Not taken

The `.json` files (the values each valid case decodes to: the reader's
tree is not compared with them, since the vault's records are checked in
`tests/contents.rs`), and the folders for integers, floats, booleans,
dates and times, comments, control characters and encodings: the values
are decoded by `toml_parser` (`docs/deps/toml_parser.md`), which has its
own tests, and the vault reads few of them. The `spec-*` folders and the
cases that are only 1.1 are not taken.

## Update

    git clone https://github.com/toml-lang/toml-test

in an empty directory outside the repository; copy the files named in
`tests/files-toml-1.0.0` from the five folders, and change the commit
above.
