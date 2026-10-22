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
| `tools/lint-secrets.sh` | lint | `just lint` | secret-hygiene lint (below) |
| `tools/lint-strings.sh` | lint | `just lint` | user-facing-strings lint (below) |
| `tools/lint-tokens.sh` | lint | `just lint` | design-token lint (below) |
| `tools/lint-docs.sh` | lint | `just lint` | documentation lint (below) |
| `tools/lint-layout.sh` | lint | `just lint` | layout lint: the code map, file sizes, long functions (below) |
| `tools/lint-keys-tests.sh` | lint | `just lint` | no shipped binary carries the test helpers that reach a key (below) |
| `tools/lint-unchecked.sh` | lint | `just lint` | no shipped binary compiles `Compute::unchecked` (below) |
| `tools/check-macos.sh` | lint | `just lint` | clippy over the Starter's macOS code for both Mac targets, without a Mac |
| `tools/costs-lint/` | lint | `just lint` (`cargo run -p costs-lint`) | who takes, mints and keeps the compute token (below) |
| `tools/learn/sync.py` | lint, generator | `just lint` (`check`), `just learn-export`, `just learn-import` | keeps `docs/learn/*.md` and the Learn pages' `en.rs` identical |
| `tools/fontbake/` | generator | `just fonts` | turns `tools/fonts/` into glyph outlines for `faraday-draw` |
| `tools/fonts/` | data | `tools/fontbake` | vendored font files and their licences |
| `tools/wordlists/` | generator | `just wordlists` (needs network) | `gen.py` and `gen-slip39.py` write the BIP-39 and SLIP-39 wordlist modules in `faraday-bip` |
| `tools/diceware/gen.py` | generator | `just diceware` | writes the EFF diceware modules in `faraday-bip` from `tools/vectors/eff/` |
| `tools/vectors/` | data | tests in `core/` and `faraday/` (see Vectors) | published and cross-implementation test vectors, one folder per spec |
| `tools/vectors/psbt/extract.py` | generator | by hand | extracts the BIP-174 vectors from the BIP text into `bip174.txt` |
| `tools/vectors/apk/make.sh`, `variants.py` | generator | `faraday-apk` tests (`docs/APK-SIGNING.md` §8) | make the unsigned APK fixtures with Android's own tools |
| `tools/reference/` | data | comments and tests that cite it (`aezeed`, `vanity`, `dice`, `kdbx4`), `tools/scripts/threshold-reference.py` (`bip445`) | vendored or quoted reference code and notes, each with a README |
| `tools/scripts/threshold-reference.py` | test | `just threshold-reference` | replays the threshold fixture session against BIP 445's reference |
| `tools/scripts/{musig,tapmulti,threshold}-regtest.sh` | generator | by hand; `core/faraday-psbt/examples/` and `just psbt-fixtures` write the rest | Bitcoin Core regtest round trips behind the MuSig2, taproot-multisig and threshold PSBT fixtures |
| `tools/backup/decrypt.py` | test, dev aid | `faraday-backup` `tests/vectors.rs` | reads a `.oskb` backup with published libraries, no Faraday code |
| `tools/backup/read-kdbx.py` | test, dev aid | `faraday-backup` `tests/vectors.rs` | opens a KDBX 4 file Faraday wrote and prints its entries |
| `tools/vault/open.py` | test, dev aid | `faraday-vault` `tests/open_py.rs` | opens a Faraday vault with published libraries |
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

## The lints `just lint` runs

- `python3 tools/clippy.py --workspace --all-targets`: clippy with `-D warnings`.
- `cargo deny check`: advisories, licences, duplicates and sources, against `deny.toml`.
- `cargo fmt --all --check`: formatting.
- `lint-secrets.sh`: in `core/*/src` and `faraday-core/src`, no printing or logging macros and no `Debug` derive on secret type names; no heap text (`String`, `to_string`, `format!`) in the files that hold secrets (a new one is added to `SECRET_FILES`); and rule 4, the key boundary: under `faraday/` outside `faraday-keys`, no key or seed in hand, no seed words made into a `Mnemonic`, no primitive's secret-taking function, and no held secret taken out of its guard outside one named site (`docs/KEYS.md`).
- `lint-strings.sh`: no wording literal in screen code; every user-facing string is in `faraday-strings/src/en.rs`, which is also checked for untidy strings. Learn pages and self-test names are outside it.
- `lint-tokens.sh`: no layout number written in `core/faraday-draw/src/widgets/`; each is a name in `tokens.rs`. In `faraday-core` and `faraday-ui` the numbers written in place are counted against `lint-tokens.counts`: a count that rises fails, and so does one that falls until the file is lowered to match.
- `lint-layout.sh`: every module faraday-core's `lib.rs` declares is named in its code map, every file at the top of its `src/` is declared, and every `Screen` has a row naming the file it is drawn in; no `.rs` file over 2,500 lines and no function over 100 lines (clippy's `too_many_lines`) beyond those counted in `lint-layout.counts`, with the `too_many_arguments` allowances; a count that rises fails, and so does one that falls until the file is lowered (F74).
- `lint-keys-tests.sh`: no shipped binary turns on faraday-keys' `master-for-tests` or faraday-core's `testkit` feature.
- `lint-docs.sh`: every document under `docs/` and the root `README.md` has a Kind and a Status in its first 12 lines; every backticked repository path in a tracked `.md` file exists; nothing outside `CLAUDE.md` cites the private folder.
- `cargo run -p costs-lint`: a function taking `&Compute` has "Costs " first in its doc; `Compute::new` is minted only where listed; no `Compute` is stored except in `View`; drawing reaches the token only through `View`'s accessors (`docs/COMPUTE.md` §8).
- `lint-unchecked.sh`: the `unchecked` feature of `faraday-crypto` is off in the dependency graph of every shipped binary (`docs/COMPUTE.md` §3).
- `check-macos.sh`: clippy for `aarch64-apple-darwin` and `x86_64-apple-darwin` over the Starter's shell.
- `python3 tools/learn/sync.py check`: fails while `docs/learn/` and `en.rs` differ.

## Vectors

Two places, split by origin and not by how many crates read them:

- `tools/vectors/<spec>/` holds vectors published by a spec or another
  implementation (BIP texts, RFC numbers, Bitcoin Core's PSBTs, Trezor's
  BIP-39 data) and fixtures a second implementation also reads (the
  `.oskb`, vault and KDBX files that the Python readers open). Tests refer
  to them by path, from `core/` and `faraday/` alike (`tools/vectors/psbt/`
  is read by `faraday-bip`, `faraday-psbt` and `faraday-selftest`;
  `tools/vectors/bip39/` by `faraday-bip` and `faraday-selftest`). Most
  folders have a README naming the source.
- A crate's own `tests/vectors/` holds fixtures only that crate reads, which
  no spec or other tool publishes: `faraday-qr` (QR and BBQr kits, files
  written by Faraday OS), `faraday-payload` (xz streams), `faraday-core`
  (`interop/`) and the Starter shell.
