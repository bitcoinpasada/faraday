# Tools

Every script and helper program in `tools/` and `faraday/tools/`. Stage is
one of build, test, lint, release, generator, data or dev aid. Recipes are
in `justfile` and `faraday/faraday.just`.

## Which folder

- `tools/` is for the repository as a whole: lints, generators, vectors,
  fonts, wordlists and reference data.
- `faraday/tools/` is for the device and the release: image tests, the
  release staging and the testkit.

## Table

| Path | Stage | What uses it | What it does |
|---|---|---|---|
| `tools/clippy.py` | lint | `just lint`, `tools/check-macos.sh` | clippy with warnings as errors; also fails on a warning about `clippy.toml` |
| `tools/vet-refresh.py` | lint | `just vet-refresh` | fetches the imported audit sets and prunes (online), keeps the exemptions the fuzz workspaces still need, then vets all three workspaces offline (F71) |
| `tools/lint-secrets.sh` | lint | `just lint` | secret-hygiene lint (below) |
| `tools/lint-strings.sh` | lint | `just lint` | user-facing-strings lint (below) |
| `tools/lint-tokens.sh` | lint | `just lint` | design-token lint (below) |
| `tools/lint-docs.sh` | lint | `just lint` | documentation lint (below) |
| `tools/lint-layout.sh` | lint | `just lint` | layout lint: the code map, file sizes, long functions (below) |
| `tools/lint-keys-tests.sh` | lint | `just lint` | no shipped binary carries the test helpers that reach a key (below) |
| `tools/lint-unchecked.sh` | lint | `just lint` | no shipped binary compiles `Compute::unchecked` (below) |
| `tools/fuzz-release.sh` | release | `just fuzz-release` | the fuzz run before a release: every target of `fuzz/` and `faraday/fuzz/`, 10 minutes each, in parallel, corpus kept between runs; writes `out/fuzz-release.txt` for the release notes |
| `tools/fuzz-corpus-export.sh` | release | `just fuzz-corpus-export` | each target's kept corpus and seeds minimized with `cargo fuzz cmin` into `out/fuzz-corpus/<target>/`, for the repository that publishes the corpora |
| `tools/fuzz-common.sh` | release | the two scripts above | the pinned nightly, each target's libFuzzer options, the corpus store |
| `tools/check-macos.sh` | lint | `just lint` | clippy over the Starter's macOS code for both Mac targets, without a Mac |
| `tools/lint-crosscheck.sh` | lint | `just lint` | the crypto has not changed since the last passing cross-check (below) |
| `tools/lint-themes.sh` | lint | `just lint` | the colour tables are what the vendored material-color-utilities makes (below) |
| `tools/costs-lint/` | lint | `just lint` (`cargo run -p costs-lint`) | who takes, mints and keeps the compute token (below) |
| `tools/learn/sync.py` | lint, generator | `just lint` (`check`), `just learn-export`, `just learn-import` | keeps `docs/learn/*.md` and the Learn pages' `en.rs` identical |
| `tools/fontbake/` | generator | `just fonts` | turns `tools/fonts/` into glyph outlines for `faraday-draw` |
| `tools/fonts/` | data | `tools/fontbake` | vendored font files and their licences |
| `tools/wordlists/` | generator | `just wordlists` (needs network) | `gen.py` and `gen-slip39.py` write the BIP-39 and SLIP-39 wordlist modules in `faraday-bip` |
| `tools/diceware/gen.py` | generator | `just diceware` | writes the EFF diceware modules in `faraday-bip` from `tools/vectors/eff/` |
| `tools/themes/gen.mjs` | generator | `just themes`, `tools/lint-themes.sh` | writes the colour tables, `faraday/faraday-ui/src/pal/schemes.rs`, from the themes' seeds with the vendored material-color-utilities (`docs/THEMES.md` §10); Node 22 or later, offline |
| `tools/themes/resolve.mjs` | generator | `tools/themes/gen.mjs` | Node resolve hook: adds `.js` to the reference's relative imports that lack it |
| `tools/themes/reference.sha256` | data | `tools/lint-themes.sh` | SHA-256 of every vendored material-color-utilities file |
| `tools/vectors/` | data | tests in `core/` and `faraday/` (see Vectors) | published and cross-implementation test vectors, one folder per spec |
| `tools/vectors/psbt/extract.py` | generator | by hand | extracts the BIP-174 vectors from the BIP text into `bip174.txt` |
| `tools/reference/` | data | comments and tests that cite it (`aezeed`, `vanity`, `dice`, `kdbx4`), `tools/crosscheck/` (`bbqr`, `bip327`, `bip352`, `bip380`, `bip39`, `bip85`, `codex32`, `core`, `qrcodegen`, `slip39`, `ur`; the statements in `aezeed`, `dice`, `entropy`, `specter` and `vanity`), `tools/themes/gen.mjs` (`material-color-utilities`) | vendored or quoted reference code and notes, each with a README |
| `tools/crosscheck/` | test | `just crosscheck` (`run.sh`) | the crypto cross-check (F79): Faraday's Rust against the vendored Python references over the published vectors and a seeded random corpus; writes `stamp` ([README](crosscheck/README.md)) |
| `tools/scripts/{musig,tapmulti}-regtest.sh` | generator | by hand; `core/faraday-psbt/examples/` and `just psbt-fixtures` write the rest | Bitcoin Core regtest round trips behind the MuSig2 and taproot-multisig PSBT fixtures |
| `tools/backup/read-kdbx.py` | test, dev aid | `faraday-backup` `tests/vectors.rs` | opens a KDBX 4 file Faraday wrote and prints its entries |
| `tools/vault/open.py` | test, dev aid | `faraday-vault` `tests/open_py.rs` | opens a Faraday vault with published libraries |
| `tools/vault/primitives.py` | test, dev aid | `tools/vault/open.py` | Argon2id, XChaCha20-Poly1305 and the BIP-39 wordlists for the vault reader |
| `tools/winvm/` | dev aid | `docs/history/BITLOCKER-ACCEPTANCE.md`, `docs/STARTER.md` | Windows 11 test machine in QEMU for the Starter's BitLocker checks |
| `faraday/tools/testkit/` | test | `just faraday-testkit`, `just faraday-snap`, `just faraday-snapshots` | writes the test stick's files after checking every spend signs |
| `faraday/tools/alloc-count/` | test | `faraday-core`'s frame budgets (`docs/SPEED.md`) | allocator that counts allocations per thread; tests only |
| `faraday/tools/argon2-bench/` | dev aid | `docs/VAULT.md` §3.1 | times the vault's Argon2id presets on a machine |
| `faraday/tools/stage-release.sh` | release | `just faraday-release-stage`, `faraday-release-stage-starter` | stages a release's files and `SHA256SUMS` under `out/release/VERSION/` |
| `faraday/tools/stick-qemu.py` | test | `disk-test.py`, `qemu-check.py`, `upgrade-check.py`; `faraday/image/README.md` | boots the stick image in QEMU with OVMF the way a laptop does |
| `faraday/tools/qemu-check.py` | test | by hand (`faraday/image/README.md`) | boots the stick image headless and photographs it |
| `faraday/tools/disk-test.py` | test | by hand | boots the dev stick image and checks the stick rule |
| `faraday/tools/upgrade-check.py` | test | `just faraday-upgrade-check` | upgrades an older stick from a new one in QEMU and boots it |
| `faraday/tools/sb-ovmf-check.py` | test | `just faraday-sb-ovmf` | Secure Boot end to end in OVMF with `faraday-sb`'s keys |
| `faraday/tools/core-check.py` | test | by hand (`docs/INTEROP.md`) | Faraday against Bitcoin Core on regtest |

## Before a release

The checks that run before a release is staged. Each is its own recipe;
none is part of `just`, which stays fast.

1. `just` passes: the tests and the lints.
2. `just crosscheck --fresh`: Faraday's own Rust against the Python
   references with a new seed and ten times the random cases, over the
   areas listed in `tools/crosscheck/README.md` (what they leave out is
   under `docs/CROSSCHECK.md` §10.5 items 4 and 6: PSBT signing and
   finalizing end to end, taproot script trees, miniscript, AEZ). The
   `vault` and `kdbx` areas need the distribution's argon2-cffi, PyNaCl
   and pykeepass. A disagreement it finds is kept in
   its `found` folder with its seed and inputs
   (`docs/decisions/F79.md`).
3. The fuzz run, and the corpora published after it (`fuzz/README.md`,
   "Before a release"). On the build machine:
   1. `just fuzz-release`, starting from the last published corpus
      (`FARADAY_FUZZ_CORPUS=<checkout of the corpus repository>`): every
      target for 10 minutes, in parallel, each keeping its corpus between
      runs. Any crash fails it; fix the parser, add the input to the
      target's corpus and run again.
   2. `just fuzz-corpus-export`: each target's corpus, minimized, in
      `out/fuzz-corpus/`.
   3. Commit that folder's contents to the corpus repository,
      https://github.com/bitcoinpasada/faraday-fuzz-corpus, and tag the
      commit with the release's tag.
   4. The release notes quote `out/fuzz-release.txt` (each target's
      seconds, executions, corpus size and "no crashes", with the commit,
      the nightly, the cargo-fuzz version and the corpus it started from)
      and the corpus repository's commit from 3.
4. The release is staged (`faraday/tools/stage-release.sh`, `docs/STARTER.md`
   §1.3) and signed.

A new parser of untrusted input gets its fuzz target in the change that
adds it; `fuzz/README.md` and `faraday/fuzz/README.md` list what each
feeds.

## The lints `just lint` runs

- `python3 tools/clippy.py --locked --workspace --all-targets`: clippy with `-D warnings`.
- `cargo deny check`: advisories, licences, duplicates and sources, against `deny.toml`.
- `cargo vet --locked`: every dependency trusted by a named publisher, covered by an imported audit, or an exemption, against the committed `supply-chain/` and `imports.lock`, offline (F71). `just vet-refresh` fetches the imports and prunes, online, when chosen, keeping what the fuzz workspaces need; trust entries end within a year and are renewed with it.
- `cargo fmt --all --check`: formatting.
- `lint-secrets.sh`: in `core/*/src` and `faraday-core/src`, no printing or logging macros and no `Debug` derive on secret type names; no heap text (`String`, `to_string`, `format!`) in the files that hold secrets (a new one is added to `SECRET_FILES`); and rule 4, the key boundary: under `faraday/` outside `faraday-keys`, no key or seed in hand, no seed words made into a `Mnemonic`, no primitive's secret-taking function, and no held secret taken out of its guard outside one named site (`docs/KEYS.md`).
- `lint-strings.sh`: no wording literal in screen code; every user-facing string is in `faraday-strings/src/en.rs`, which is also checked for untidy strings and for strings no code uses (a key no Rust file names as `.key` fails). Learn pages and self-test names are outside it.
- `lint-tokens.sh`: no layout number written in `core/faraday-draw/src/widgets/`; each is a name in `tokens.rs`. In `faraday-core` and `faraday-ui` the numbers written in place are counted against `lint-tokens.counts`: a count that rises fails, and so does one that falls until the file is lowered to match.
- `lint-layout.sh`: every module faraday-core's `lib.rs` declares is named in its code map, every file at the top of its `src/` is declared, and every `Screen` has a row naming the file it is drawn in; no `.rs` file over 2,500 lines and no function over 100 lines (clippy's `too_many_lines`) beyond those counted in `lint-layout.counts`, with the `too_many_arguments` allowances; a count that rises fails, and so does one that falls until the file is lowered (F74).
- `lint-keys-tests.sh`: no shipped binary turns on faraday-keys' `master-for-tests` or faraday-core's `testkit` feature.
- `lint-docs.sh`: every document under `docs/` has a Kind and a Status in its first 12 lines; a Spec has an `**Open:**` line saying what is not built and a Reference or Record has none; a Reference has no heading of plan or build log (rules for the implementing agent, batches, order of work, estimates, questions for the owner) unless the section is a one-line stub naming where it moved in `docs/history/`, nor a line of build log ("Built 2026-10-19", "Specified, not built", words for the implementing agent); `docs/DECISIONS.md` has one row per decision with none missing, each linking its file in `docs/decisions/` or, once retired, `docs/history/decisions/`, with the file's Status the row's, and every decision file has a row; every backticked repository path in a tracked `.md` file exists; nothing outside `CLAUDE.md` cites the private folder.
- `cargo run -p costs-lint`: a function taking `&Compute` has "Costs " first in its doc; `Compute::new` is minted only where listed; no `Compute` is stored except in `View`; drawing reaches the token only through `View`'s accessors (`docs/COMPUTE.md` §8).
- `lint-unchecked.sh`: the `unchecked` feature of `faraday-crypto` is off in the dependency graph of every shipped binary (`docs/COMPUTE.md` §3).
- `lint-crosscheck.sh`: the hash of the crypto sources, the cross-check, its references and vectors, and the crypto crates' `Cargo.lock` entries matches `tools/crosscheck/stamp`, which a passing `just crosscheck` writes; otherwise "crypto changed since the last cross-check: run `just crosscheck`" (F79).
- `lint-themes.sh`: the vendored material-color-utilities matches `tools/themes/reference.sha256`, with no file beside them but its README, and `tools/themes/gen.mjs` run on it writes `faraday/faraday-ui/src/pal/schemes.rs` byte for byte; otherwise "run `just themes`". Needs Node 22 or later (`docs/THEMES.md` §11.3).
- `check-macos.sh`: clippy for `aarch64-apple-darwin` and `x86_64-apple-darwin` over the Starter's shell.
- `python3 tools/learn/sync.py check`: fails while `docs/learn/` and `en.rs` differ.

## Vectors

Two places, split by origin and not by how many crates read them:

- `tools/vectors/<spec>/` holds vectors published by a spec or another
  implementation (BIP texts, RFC numbers, Bitcoin Core's PSBTs, Trezor's
  BIP-39 data) and fixtures a second implementation also reads (the
  vault and KDBX files that the Python readers open). Tests refer
  to them by path, from `core/` and `faraday/` alike (`tools/vectors/psbt/`
  is read by `faraday-bip`, `faraday-psbt` and `faraday-selftest`;
  `tools/vectors/bip39/` by `faraday-bip` and `faraday-selftest`). Most
  folders have a README naming the source.
- A crate's own `tests/vectors/` holds fixtures only that crate reads, which
  no spec or other tool publishes: `faraday-qr` (QR and BBQr kits, files
  written by Faraday OS), `faraday-payload` (xz streams), `faraday-core`
  (`interop/`) and the Starter shell.
