# Tools

Every script and helper program in `tools/` and `faraday/tools/`. Stage is
one of build, test, lint, release, generator, data or dev aid. Recipes are
in `justfile` and `faraday/faraday.just`.
`just lint` runs its parts in order, each a recipe of its own: `lint-clippy`,
`lint-deps`, `lint-repo`, `lint-touch-targets` and `lint-op-cost`.
`lint-macos` is not one of them: macOS is paused and it is run by hand.

## Which folder

- `tools/` is for the repository as a whole: lints, generators, vectors,
  fonts, wordlists and reference data.
- `faraday/tools/` is for the device and the release: image tests, the
  release staging and the testkit.

## Table

| Path | Stage | What uses it | What it does |
|---|---|---|---|
| `tools/clippy.py` | lint | `just lint`, `tools/check-macos.sh` | clippy with warnings as errors; also fails on a warning about `clippy.toml` |
| `tools/vet-refresh.py` | lint | `just vet-refresh` | fetches the imported audit sets and prunes (online), keeps the exemptions the separate workspaces (the two fuzz ones, keepass-rs's and rPGP's) still need, then vets all five workspaces offline (F71) |
| `tools/lint-all.py` | runner | `just lint-all`, `just` | runs every command of `lint` to the end and lists the ones that failed |
| `tools/lint-secrets.sh` | lint | `just lint` | secret-hygiene lint (below) |
| `tools/lint-strings.sh` | lint | `just lint` | user-facing-strings lint (below) |
| `tools/lint-tokens.sh` | lint | `just lint` | design-token lint (below); `--write` rewrites `lint-tokens.counts` |
| `tools/touch-targets.counts` | ratchet | `just lint-touch-targets` (faraday-snapshot's and faraday-starter's ignored `targets::` tests, `docs/M3-CHECKS.md` §2) | press targets under 48 dp per size, screen and action kind; a count that rises fails, and so does one that falls until the file is lowered; `FARADAY_TARGETS_WRITE=1 NEXTEST_TEST_THREADS=1 just lint-touch-targets` rewrites it |
| `tools/lint-docs.sh` | lint | `just lint` | documentation lint (below) |
| `tools/lint-layout.sh` | lint | `just lint` | layout lint: the code map, file sizes, long functions (below) |
| `tools/lint-builds.py` | lint | `just lint` | `faraday/builds.toml`, the list of builds, against the recipes and the image directories (below) |
| `tools/impact.py` | lint | `just lint` (`lint`, `check --quiet`, `docs --check`); `plan <paths>` for a task's brief; `docs` writes `docs/CHECKS.md`; `report` the full map; `check <range> --consumer <id>` one part of the suite | the registry `tools/impact.toml`: what a change brings with it, which check holds each rule, a pair left unmet in the range from main (below, `docs/IMPACT.md`) |
| `tools/lint-tree.sh` | lint | `just lint` | tree lint: the top level's entries, no leftovers, no file over 2 MiB, no image metadata (below) |
| `tools/image-metadata.py` | dev aid | `lint-tree.sh`; by hand | lists (`--check`) or removes (`--strip`) the metadata of PNG, JPEG, WebP and SVG images |
| `tools/lint-keys-tests.sh` | lint | `just lint` | no shipped binary carries the test helpers that reach a key (below) |
| `cargo check --locked --bins` | lint | `just lint` | every target of `fuzz/` and `faraday/fuzz/` still compiles (below) |
| `tools/lint-fuzz-parsers.py` | lint | `just lint` | a fuzz target or an excuse for every parser of untrusted input in `core/`, listed in `tools/fuzz-parsers.toml` (below) |
| `tools/lint-unchecked.sh` | lint | `just lint` | no shipped binary compiles `Compute::unchecked` (below) |
| `tools/fuzz-release.sh` | release | `just fuzz-release` | the fuzz run before a release: every target of `fuzz/` and `faraday/fuzz/`, 10 minutes each, in parallel, corpus kept between runs; writes `out/fuzz-release.txt` for the release notes |
| `tools/fuzz-corpus-export.sh` | release | `just fuzz-corpus-export` | each target's kept corpus and seeds minimized with `cargo fuzz cmin` into `out/fuzz-corpus/<target>/`, for the repository that publishes the corpora; the store keeps the minimized set |
| `tools/fuzz-common.sh` | release | the two scripts above | the pinned nightly, each target's libFuzzer options and dictionary, the corpus store, the targets of the partitions `FARADAY_FUZZ_PARTITION` names |
| `tools/fuzz-join.py` | release | `just fuzz-join OUT PART...`; `python3 -I -B tools/fuzz-join.py --later OUT EARLIER LATER` | a fuzz run split into partitions joined: each part's `fuzz-release.txt` and `fuzz-corpus/` into one record, each target once, and one corpus; refuses parts of different runs, a partition twice or one missing. With `--later`, two runs of one release (F241): the record names both and marks each target's row with its run, the corpus is the later run's for the targets it ran |
| `tools/fuzz-plan.py` | release | `tools/fuzz-release.sh` | the coverage map made at the end of a full fuzz run (the files each target's corpus reaches) and the targets a later run runs again after the changes since the earlier run's commit (F241) |
| `tools/check-macos.sh` | by hand | `just lint-macos` (macOS is paused; no automatic check runs it) | clippy over the Starter's macOS code for both Mac targets, without a Mac |
| `tools/lint-crosscheck.sh` | lint | `just lint-release` | before a release, the crypto has not changed since the last passing cross-check (below) |
| `tools/lint-standards.py` | lint | `just lint` | `docs/STANDARDS.md` §0 held against `tools/lint-standards.toml`: cross-check coverage, work kept out of the app, every hold with its decision, §8's disagreements (below) |
| `tools/lint-standards.toml` | data | `tools/lint-standards.py` | the cryptography the stamp does not cover with its reasons, the work kept out of the app, every hold with its decision and case (F136), one entry per STANDARDS §8 disagreement |
| `docs/crypto/verified.py` | lint | `just lint` (`check`) | every STANDARDS row of the manifest names a check, and every cross-check area and vectors folder is a row's (below) |
| `tools/lint-upstream.py` | lint | `just lint` | the upstream project is named only in LICENSE (F73, below) |
| `tools/op-cost/` | measure | `tools/op-cost.py` | runs one named costly operation once on a fixed public test vector (`op-cost <name>`, `--list`); never runs on a device |
| `tools/op-cost.py` | lint | `just lint` (`just lint-op-cost`; `--write` rewrites the counts) | the instruction count of each operation under valgrind's callgrind, held to `tools/op-cost.counts` (F238, below) |
| `tools/op-cost.counts` | ratchet | `tools/op-cost.py` | each named costly operation's instruction count; more than 25% over fails, raised only in a commit that says why |
| `tools/crosscheck/facts.py` | lint | `just lint` (`python3 -I -B tools/crosscheck/facts.py check`; `write` rewrites the README) | every cross-check area states its facts, the paths it names resolve, and `tools/crosscheck/README.md`'s table and known differences are what they say |
| `tools/lint-themes.sh` | lint | `just lint` | the colour tables are what the vendored material-color-utilities makes (below) |
| `tools/costs-lint/` | lint | `just lint` (`cargo run -p costs-lint`) | who takes, mints and keeps the compute token (below) |
| `tools/costs-lint/` (`crypto-inventory`) | generator | `cargo run -q -p costs-lint --bin crypto-inventory`; `just lint` through `docs/crypto/check.py check` | the cryptographic functions of the tree with their body hashes, callees, tests and call sites in faraday-core, as JSON lines (`docs/CRYPTO-LEARN.md` §4, below) |
| `docs/crypto/check.py` | lint, generator | `just crypto-review ID`, `just crypto-review-stale`, `just crypto-changes TAG`, `just lint` (`check`), `just lint-release` (`check --release`) | the manifest `docs/crypto/actions.toml` against the inventory and the tree: coverage, schema, what its entries name, fingerprints; `review` is the only writer of a fingerprint (`docs/CRYPTO-LEARN.md` §5); it also checks `docs/AUDIT.md` §1's crate rows, that every `Reveal` variant is named and the count in words, `docs/COMPUTE.md` §2.3's roots, the Tools tiles' tags and the `[[not_crypto]]` count against `tools/lint-crypto.counts` (a ratchet that only falls; raising it is the owner's decision); `inventory` prints the inventory for `--inventory FILE`; `--self-test` runs a fixture |
| `docs/crypto/gen.py` | generator, lint | `python3 -I -B docs/crypto/gen.py write`, `just lint` (`check`; `just crypto-review` runs `write`) | the generated places of `docs/STANDARDS.md` §1 to §7 and §10 and `docs/AUDIT.md` §3, between `crypto-manifest` marker lines, from `docs/crypto/actions.toml`; `check` regenerates in memory and fails on any difference; `--self-test` runs a fixture (`docs/CRYPTO-LEARN.md` §3.1) |
| `tools/fontbake/` | generator | `just fonts` | turns `tools/fonts/` into glyph outlines for `faraday-draw` |
| `tools/fonts/` | data | `tools/fontbake` | vendored font files and their licences |
| `tools/wordlists/` | generator | `just wordlists` (needs network) | `gen.py` and `gen-slip39.py` write the BIP-39 and SLIP-39 wordlist modules in `faraday-bip` |
| `tools/diceware/gen.py` | generator | `just diceware` | writes the EFF diceware modules in `faraday-bip` from `tools/vectors/eff/` |
| `tools/themes/gen.mjs` | generator | `just themes`, `tools/lint-themes.sh` | writes the colour tables, `faraday/faraday-ui/src/pal/schemes.rs`, from the themes' seeds with the vendored material-color-utilities (`docs/THEMES.md` §10); Node 22 or later, offline |
| `tools/themes/resolve.mjs` | generator | `tools/themes/gen.mjs` | Node resolve hook: adds `.js` to the reference's relative imports that lack it |
| `tools/themes/reference.sha256` | data | `tools/lint-themes.sh` | SHA-256 of every vendored material-color-utilities file |
| `tools/vectors/` | data | tests in `core/` and `faraday/` (see Vectors) | published and cross-implementation test vectors, one folder per spec |
| `tools/vectors/aqrv/check.py` | checker | by hand | a pinned copy of the spec repository's second reader and writer of the Argon QR Vault, written from the AQRV specification (`https://github.com/bitcoinpasada/aqrv`, `AQRV.md` §2 to §10) with other libraries, run over every case of `aqrv.json` and the two QR pictures; prints a count and every disagreement, exits 1 on any ([README](vectors/aqrv/README.md)) |
| `tools/vectors/core-wallet/gen.py` | generator | by hand | makes Bitcoin Core descriptor wallets on regtest with the pinned `bitcoind` (plain, encrypted, watch-only, blank, with labels, with an imported descriptor, with an imported uncompressed key, multisig, miniscript, and one migrated by `migratewallet` from a legacy wallet the older release of `tools/reference/core/legacy.pin` makes) and writes each one's records and what Core lists for it into `tools/vectors/core-wallet/` |
| `tools/vectors/psbt/extract.py` | generator | by hand | extracts the BIP-174 vectors from the BIP text into `bip174.txt` |
| `tools/reference/` | data | comments and tests that cite it (`aezeed`, `vanity`, `dice`, `kdbx4`), `tools/crosscheck/` (`bbqr`, `bip327`, `bip352`, `bip374`, `bip375`, `bip380`, `bip39`, `bip85`, `codex32`, `core`, `qrcodegen`, `slip39`, `ur`; the statements in `aezeed`, `dice`, `entropy`, `specter` and `vanity`), `tools/themes/gen.mjs` (`material-color-utilities`) | vendored or quoted reference code and notes, each with a README |
| `tools/crosscheck/` | test | `just crosscheck` (`run.sh`) | the crypto cross-check (F79): Faraday's Rust against the vendored Python references over the published vectors and a seeded random corpus; writes `stamp` ([README](crosscheck/README.md)) |
| `tools/scripts/{musig,tapmulti}-regtest.sh` | generator | by hand; `core/faraday-psbt/examples/` and `just psbt-fixtures` write the rest | Bitcoin Core regtest round trips behind the MuSig2 and taproot-multisig PSBT fixtures |
| `tools/scripts/signer-core.py` | generator | `python3 -I -B tools/scripts/signer-core.py <directory>`, by hand; its two files are inputs of `core/faraday-psbt/examples/signer.rs` | Bitcoin Core 31.1 on regtest signing `wallet-2of3.psbt` and `demo-regtest.psbt`: the signer check's two Core fixtures |
| `tools/vault/open.py` | test, dev aid | `faraday-vault` `tests/open_py.rs` | opens a Faraday vault with published libraries |
| `tools/winvm/` | dev aid | `docs/STARTER.md` | Windows 11 test machine in QEMU, used to record the Starter's BitLocker answers (`tests/windows/vm-*.wmi`) and to check them by hand |
| `faraday/tools/testkit/` | test | `just faraday-testkit`, `just faraday-snap`, `just faraday-snapshots` | writes the test stick's files after checking every spend signs |
| `faraday/tools/alloc-count/` | test | `faraday-core`'s frame budgets (`docs/SPEED.md`) | allocator that counts allocations per thread; tests only |
| `faraday/tools/leak-scan/` | test | `faraday-core`'s leak tests (`faraday/faraday-core/tests/leaks`, `docs/SECRET-LEAKS.md`) | allocator that searches the blocks an armed test thread frees, and the live ones on demand, for a test secret's forms; tests only |
| `tools/leak-entries.toml` | test | the leak tests | the copies of a test secret a flow is let to leave in a freed block: its flow, form, place, reason and fix; an entry no run hits fails |
| `faraday/tools/argon2-bench/` | dev aid | `docs/VAULT.md` §3.1 | times the vault's Argon2id presets on a machine |
| `faraday/tools/stage-release.sh` | release | `just faraday-release-stage`, `faraday-release-stage-starter` | stages a release's files, every image `faraday/builds.toml` releases, `IMAGES` and `SHA256SUMS` under `out/release/VERSION/` |
| `faraday/tools/stick-qemu.py` | test | `disk-test.py`, `qemu-check.py`; `docs/IMAGES.md` | boots the stick image in QEMU with OVMF the way a laptop does |
| `faraday/tools/qemu-check.py` | test | by hand (`docs/IMAGES.md`) | boots the stick image headless and photographs it |
| `faraday/tools/disk-test.py` | test | by hand | boots the dev stick image and checks the stick rule |
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
   `vault` area needs the distribution's argon2-cffi, PyCA cryptography and
   pykeepass. A disagreement it finds is kept in
   `out/crosscheck/found/` with its seed and inputs
   (`docs/decisions/F79.md`). A run that failed in areas whose fix is in
   their own files is made whole by `just crosscheck --combine RECORD` at
   its seed, its record and report copied aside first: only those areas run
   again, and it refuses when anything the areas share changed (F239,
   `tools/crosscheck/README.md`). A full run also writes
   `out/crosscheck-record.txt` (the tree it checked without the fingerprints, seed, scale, date, each area's
   cases and answers, each reference's SHA-256, Core's tarball, the
   result); staging puts it beside `SHA256SUMS` as
   `crosscheck-<version>.txt`, listed there so it is signed, and the
   release notes quote its summary as they do the fuzz record. Then
   `just crypto-review-stale` (or `just crypto-review ID` for one action) for
   what `check.py check --release` names, and `just lint-release` passes: the release is what is
   cross-checked, the tip between releases is not (F197).
3. The fuzz run, and the corpora published after it (`fuzz/README.md`,
   "Before a release"). On any machine, or on GitHub's runners with
   `.github/workflows/fuzz.yml` started by hand, which does 1 and 2 in
   nineteen partitions and joins them (F210):
   1. `just fuzz-release`, starting from the last published corpus
      (`FARADAY_FUZZ_CORPUS=<checkout of the corpus repository>`): every
      target for 10 minutes, in parallel, each keeping its corpus between
      runs. Any crash fails it; fix the parser, add the input to the
      target's corpus and run again.
   2. `just fuzz-corpus-export`: each target's corpus, minimized, in
      `out/fuzz-corpus/`, and the same minimized set kept as the store.
   A long run can be split into partitions (`FARADAY_FUZZ_PARTITION=M/N`,
   the same SECONDS and starting corpus for each part), each running 1 and
   2, and `just fuzz-join OUT PART...` makes the one `fuzz-release.txt` and
   `fuzz-corpus/` of the parts' `out/` folders.
   A full run ends with `out/fuzz-map.txt`, the files each target's
   corpus reaches at that commit (a run of parts: `tools/fuzz-release.sh
   map out/fuzz-corpus` after the join, which the workflow does). After a
   change, a later run needs only the targets the change reaches
   (`FARADAY_FUZZ_EARLIER=<folder of the earlier run's fuzz-release.txt,
   fuzz-map.txt and fuzz-corpus/> just fuzz-release`, or the `targets`
   input of the workflow with the list `tools/fuzz-plan.py plan EARLIER`
   prints), and `python3 -I -B tools/fuzz-join.py --later OUT
   EARLIER LATER` joins the two into the record the notes quote (F241).
   3. Commit that folder's contents to the corpus repository,
      https://github.com/bitcoinpasada/faraday-fuzz-corpus, and tag the
      commit with the release's tag.
   4. The release notes quote `out/fuzz-release.txt` (each target's
      seconds, executions, corpus size and "no crashes", with the commit,
      the nightly, the cargo-fuzz version and the corpus it started from)
      and the corpus repository's commit from 3.
   The notes also quote `crosscheck-<version>.txt`: the seed, the scale,
   each area's answers and "every answer agreed".
4. The release is staged (`faraday/tools/stage-release.sh`, `docs/STARTER.md`
   §1.3): the plain files and `IMAGES`, then the Starters, built against
   that `IMAGES` (`FARADAY_STARTER_IMAGES`); and signed.

A new parser of untrusted input gets its fuzz target in the change that
adds it; `fuzz/README.md` and `faraday/fuzz/README.md` list what each
feeds.

## The lints `just lint` runs

- `python3 tools/clippy.py --locked --workspace --all-targets`: clippy with `-D warnings`.
- `cargo deny check`: advisories, licences, duplicates and sources, against `deny.toml`.
- `cargo vet --locked`: every dependency trusted by a named publisher, covered by an imported audit, or an exemption, against the committed `supply-chain/` and `imports.lock`, offline (F71). `just vet-refresh` fetches the imports and prunes, online, when chosen, keeping what the separate workspaces (the two fuzz ones, keepass-rs's and rPGP's) need; trust entries end within a year and are renewed with it.
- `cargo fmt --all --check`: formatting.
- `lint-secrets.sh`: in `core/*/src` and `faraday-core/src`, no printing or logging macros; no `Clone`, `PartialEq` or `Debug` derive and no `Display` impl on a type that holds a secret (found by its fields, its `Drop` or `Zeroize`, or its name; a type that keeps one is named in `ALLOWED_TYPES` with its reason); no heap text (`String`, `to_string`, `format!`) in the files that hold secrets (`SECRET_FILES`); a non-test source file that uses a secret type is in `SECRET_FILES`; a NAMELESS file that now uses one is reported; the wiping text is not counted as a secret type yet, so a file that holds text only as wiping `Text` is held only when listed; the method calls that build a `String` from other text (`.join(`, `.concat()`, case changes, `.replace(`, `.repeat(`, each on one line) are refused in the same files, but for a line marked `// not text:` and the path joins of three named files; and rule 4, the key boundary: under `faraday/` outside `faraday-keys`, no key or seed in hand, no seed words made into a `Mnemonic`, no primitive's secret-taking function, and no held secret taken out of its guard outside one named site (`docs/KEYS.md`).
- `lint-strings.sh`: no wording literal in screen code; every user-facing string is in `faraday-strings/src/en.rs`, which is also checked for untidy strings and for strings no code uses (a key no Rust file names as `.key` fails). Learn pages and self-test names are outside it.
- `lint-tokens.sh`: no layout number written in `core/faraday-draw/src/widgets/`; each is a name in `tokens.rs`. The numbers written in place are counted per file in `lint-tokens.counts`, in every file of `faraday-ui` and `faraday-starter` and in `faraday-core`'s screen files (`screen.rs` and `screens/`): a file over its count fails, a file under it fails until the count is lowered, and a counted file with no line allows none; `tools/lint-tokens.sh --write` rewrites the file from the tree. The colours written in place (`Color::rgb(`, where a screen takes one of Material 3's colour roles from faraday-ui's palette) are one count for `faraday-core`, held the same way, and none are allowed in `faraday-starter`: any one fails, with its file and line.
- `lint-builds.py`: `faraday/builds.toml` lists every build the repository makes and the ones planned; each id once, only the fields its header names, a built or untried entry's recipe a `just` recipe with its outputs, an image's board and panel directories that exist, a planned entry's document that exists, and every directory under `faraday/image/boards` and `faraday/image/panels` named by a built or untried entry; and its `[[hardware]]` entries, the download page's tiles: each id once, only the fields the header names, each built or planned image offered under one of them as strict or open, no two images of one hardware alike, every hardware with an image, and `tested` only where an image is built.
- `lint-tree.sh`: the top level holds only the entries the script lists, each with what it is (a new one is added there and to README.md's "Repository layout"); no editor, merge or operating-system leftover is tracked anywhere (`*.orig`, `*.rej`, `*.bak`, `*.swp`, `*~`, `.DS_Store`, `*.log`, `*.tmp`, `*.before-*`, `*.old`); no file is over 2 MiB; no tracked png, jpg, jpeg, webp or svg carries metadata (`image-metadata.py --check`: a PNG's text, EXIF, ICC and time chunks, a JPEG's EXIF, XMP, ICC and comment segments, a WebP's EXIF, XMP and ICC chunks, an SVG's `<metadata>`, comments and editor namespaces; `--strip` removes it).
- `cargo check --locked --bins --manifest-path fuzz/Cargo.toml`, and the same for `faraday/fuzz/`: every fuzz target compiles against the code it feeds. The two crates are workspaces of their own, so nothing else builds them. It runs on the workspace's stable toolchain, needs no nightly and no cargo-fuzz, and does not run a target; `just fuzz` and `just fuzz-release` build with the nightly.
- `lint-fuzz-parsers.py`: every `pub fn` in the non-test code of `core/` named `parse*`, `decode*`, `from_bytes`, `from_str` or `from_slice` that takes a `&[u8]`, `&str`, `Vec<u8>` or `impl AsRef<[u8]>` is an entry of `tools/fuzz-parsers.toml` (by file and name), with either the fuzz target in `fuzz/` or `faraday/fuzz/` that calls it or an excuse ("no target yet" is a gap, any other text a reason); a target must exist and mention the function's name, and an entry whose function is gone fails. Entries for `faraday/` are checked the same way but a new parser there is not required. It does not see a parser under another name or type.
- `lint-layout.sh`: every module faraday-core's `lib.rs` declares is named in its code map, every file at the top of its `src/` is declared, and every `Screen` has a row naming the file it is drawn in; a file has no size limit (F248), and no function over 100 lines without clippy's `too_many_lines` expectation; Faraday's field count and the `impl Faraday` list only fall (F74).
- `lint-keys-tests.sh`: no shipped binary turns on faraday-keys' `master-for-tests` or faraday-core's `testkit` feature.
- `lint-docs.sh`: every document under `docs/` has a Kind and a Status in its first 12 lines; a Spec has an `**Open:**` line saying what is not built and a Reference or Record has none; a Reference has no heading of plan or build log (rules for the implementing agent, batches, order of work, estimates, questions for the owner) unless the section is a one-line stub naming where it moved in `docs/history/`, nor a line of build log ("Built 2026-10-19", "Specified, not built", words for the implementing agent); `docs/DECISIONS.md` has one row per decision, each linking its file in `docs/decisions/` or, once retired, `docs/history/decisions/`, with the file's Status the row's, and every decision file has a row; every active decision has a `**Held by:**` line naming rules of `tools/impact.toml` or one of `review`, `tests`, `none`, and every rule whose source is a decision is named in that decision's line; every backticked repository path in a tracked `.md` file exists; every `just <recipe>` that starts a backticked span or a line of a fenced block (not under `docs/history/`) names a recipe of `justfile`, `faraday/faraday.just` or an import, after any `name=value` assignments and options; nothing outside `CLAUDE.md` cites the private folder.
- `cargo run -p costs-lint`: a function taking `&Compute` has "Costs " first in its doc; `Compute::new` is minted only where listed; no `Compute` is stored except in `View`; drawing reaches the token only through `View`'s accessors (`docs/COMPUTE.md` §8).
- `cargo run -q -p costs-lint --bin crypto-inventory` (second binary of the same package, sharing its source walker; `--self-test` runs it over a fixture crate; `--all` prints every function of a cryptographic crate): one JSON line per cryptographic function with the rules that flagged it (`docs/CRYPTO-LEARN.md` §4.2), its body hash (§5.1), its same-crate callees and cross-crate calls; one per `#[test]` function; one per call in faraday-core of a cryptographic function of faraday-keys or `core/` (§9.2). A summary by rule and crate goes to stderr. `just lint` runs it through `docs/crypto/check.py check`.
- `lint-unchecked.sh`: the `unchecked` feature of `faraday-crypto` is off in the dependency graph of every shipped binary (`docs/COMPUTE.md` §3).
- `lint-crosscheck.sh`: the hash of the crypto sources, the cross-check, its references and vectors, and the crypto crates' `Cargo.lock` entries matches `tools/crosscheck/stamp`, which a passing `just crosscheck` writes with each area's corpus case count; otherwise "crypto changed since the last cross-check: run `just crosscheck`" (F79). Before a release only (F197).
- `lint-standards.py` (`docs/STANDARDS.md` §0), its self-test first, then four checks over `cargo run -p costs-lint --bin crypto-inventory -- --all` (about 15 s) and `tools/lint-standards.toml`. Coverage: a crate whose normal dependencies include one of the inventory's `PRIMITIVE_CRATES`, or that has a function the inventory finds cryptographic, is in `tools/crosscheck/stamp.py`'s `CRATES` or an `[[uncovered]]` crate with its reason; in faraday-core the same per file against `FILES`; the crates no binary ships are left out. Out of the app: no shipped function of a `faraday/` crate reaches an `[[out_of_app]]` root through calls in its crate and calls into others, and no line of a `faraday/` crate's `src/` matches an entry's `unnamed`. Holds (F136): every action the functions `[[hold_source]]` names call a hold (`is_hold` in faraday-core and the Starter, `is_holdable` in faraday-psbt) is a `[[hold]]` naming a decision that has a file, and its case. §8: each `[[disagreement]]`'s text is still in its files, one per bullet of STANDARDS §8. An entry the tree no longer needs fails too.
- `docs/crypto/verified.py check`, its self-test first: through `docs/crypto/check.py`'s reader and `gen.py`'s `row_actions`, each row of the manifest with actions names a cross-check area, a vectors folder in `stamp.py`'s `VECTORS` or a test file that exists, and every `area_*.py` and `tools/vectors/` folder is named by a row's actions or is a `[[vector_no_action]]`.
- `lint-upstream.py`, its self-test first: no tracked file but LICENSE holds the upstream project's name, read from LICENSE's copyright line (a LICENSE without that line names none), or the name without its last word, in any case and with `-` or `_` between its words; any occurrence fails; it prints paths and counts, never the name (F73).
- `op-cost.py`, its self-test first: builds `tools/op-cost` (`cargo build --locked --release -p op-cost`) and runs each operation it lists under `valgrind --tool=callgrind`, reading the "Collected" instruction count; fails an operation more than 25% over its count in `tools/op-cost.counts`, an operation with no count and a count with no operation; a fall says to lower it with `--write` and is not a failure. Needs valgrind (F238).
- `tools/crosscheck/facts.py check`: every `area_*.py` class has `faraday`, `reference`, `reference_checked`, `vectors`, `corpus`, `differences` and `not_compared`, none empty ("none" says there is nothing), each `faraday_*` path or file in `faraday` exists, and `tools/crosscheck/README.md` between its `facts:` marker lines is what the areas say; otherwise it names the area and the attribute, or "run `python3 -I -B tools/crosscheck/facts.py write`" (`docs/CRYPTO-LEARN.md` §10.1).
- `lint-themes.sh`: the vendored material-color-utilities matches `tools/themes/reference.sha256`, with no file beside them but its README, and `tools/themes/gen.mjs` run on it writes `faraday/faraday-ui/src/pal/schemes.rs` byte for byte; otherwise "run `just themes`". Needs Node 22 or later (`docs/THEMES.md` §11.3).
- `check-macos.sh` (by hand, `just lint-macos`; macOS is paused): clippy for `aarch64-apple-darwin` and `x86_64-apple-darwin` over the Starter's shell.
- `python3 -I -B tools/impact.py lint`, `check --quiet` and `docs --check` (F149, `docs/IMPACT.md`): the registry `tools/impact.toml` has unique ids, `held_by` and `consumers` that name what exists, source documents and paths that exist, a `just lint` recipe equal to its checks, and each app's reads matching `cargo metadata`; no rule of kind pair, forbid or trailer is unmet by the commits from `main` to the working tree (waived only by an `Impact-skip: <rule>: <reason>` trailer); and `docs/CHECKS.md` is what `python3 -I -B tools/impact.py docs` writes. `plan <paths>` says what a change to some paths brings with it; `report [--out <file>]` writes the whole map, extra rule files included; `check <range> --consumer website [--site <tree> --site-since <commit>]` says which of the website's parts a range leaves to do, and exits 1 while a blocking one is. Python 3.11 or later and git; no build.

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
