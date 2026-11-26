# Build, CI and tooling

What builds, checks, reproduces and releases Faraday: the image builds (Buildroot images for the PC stick and the Pi), the Windows program builds, the release staging, the CI workflows on GitHub Actions, the fuzz workspaces, the cross-check against outside implementations, the snapshot tours, the lint scripts and the dependency lockfiles. No person using the device reaches it. `docs/DEVELOPING.md` is the repository's own account of how to build and check.

## Screens and sheets

None.

## Actions

- Recipes and scripts run by a builder: `just` (tests, clippy, fmt, the lints), the image builds, `faraday/tools/stage-release.sh` for a release, the cross-check (`tools/crosscheck/run.sh`), the fuzz runs, `tools/fuzz-corpus-export.sh`.
- GitHub Actions: `.github/workflows/check.yml` on every push to main and pull request; `.github/workflows/targeted.yml` started by hand (crate tests split over runners, one lint recipe).

## Effects

- Build output in `out/` and `target/`.
- Images: boot and data FAT volumes per board (`faraday/image/overlay/boards/{pi02w,pi3,x86_64-uefi}/genimage.cfg`).
- A release: every image listed with a release name in `faraday/builds.toml`, plus its boot programs.
- Cross-check records naming the tree they ran on (`tools/crosscheck/record.py`, `tools/crosscheck/stamp.py`).

## State and lock

None. No app state.

## Reach

- Builds every product. Runs the test kit's tests and tours (`test-kit`).
- The cross-check drives Bitcoin Core nodes (`tools/crosscheck/core_node.py`), KeePassXC's command-line tool (`tools/crosscheck/area_vault.py`) and the import, review and Core wallet checks (`tools/crosscheck/area_import.py`, `tools/crosscheck/area_review.py`, `tools/crosscheck/area_core_wallet.py`).
- The snapshot tours live in `faraday/shells/snapshot/src/tours/`.

## Rules and refusals

Rules the fixes below restored, with the file that holds each today. No line numbers were checked.

- Reproducible images:
  - FAT volume serials come from the commit date and the partition role, so partitions differ and two builds of one commit match (`faraday/image/fat-serial.sh`).
  - Each vfat block is made with `mkfs.vfat --invariant` (the board `genimage.cfg` files).
  - Published binaries carry no home directory or user name in their strings; paths are remapped to `/build`, `/cargo`, `/cargo-git`, `/faraday` (`faraday/image/path-remap.sh`).
- The kernel-config check accepts the dev variant's command-line word (`faraday/image/check-kernel-config.sh`). The kernel forbidden list holds only real option names (`faraday/image/common/kernel.forbidden`).
- The image build container carries the tools its scripts call (`faraday/image/Dockerfile`, `faraday/image/stick-finish.py`).
- Release staging (`faraday/tools/stage-release.sh`):
  - Publishes every built image listed in `faraday/builds.toml` with a release name (checked by `tools/lint-builds.py`).
  - Reads the desktop app's build ID from its `--version` line ("Faraday Desktop BUILD").
  - Accepts a cross-check record that names a tree a release commit can have; refuses other differences, and refuses a record whose result line says an OVMF run failed.
- Windows programs: the delay-loaded imports are exactly dwmapi and uxtheme; propsys is loaded by name from System32 (`faraday/shells/window/src/system32.rs`, `faraday/build/windows/`); every delay-import slot is checked writable.
- Parsers of device input (faraday-bip, faraday-codec, faraday-fat, faraday-psbt) build with overflow checks in release, so an overflow stops the app rather than reading at a wrong bound.
- Locked builds: the lockfiles name the published, unyanked versions; the fuzz workspaces' lockfiles match the main workspace's dependencies, so they build with `--locked` (`Cargo.lock`, `faraday/fuzz/Cargo.lock`, `fuzz/Cargo.lock`, `supply-chain/imports.lock`).
- Fuzzing: the seed-parts target copies a SLIP-39 share through `Share::duplicate` (`fuzz/fuzz_targets/seed_parts.rs`). The corpus export writes crashes only under `fuzz/artifacts/<target>/` (`tools/fuzz-corpus-export.sh`).
- CI (`.github/workflows/check.yml`, `.github/workflows/targeted.yml`, `.github/actions/setup/action.yml`, `.github/actions/setup/apt-install.sh`):
  - Each package step has a time limit and three tries; the package install falls back to a second public mirror.
  - Test jobs unpack the build archive into `target/` where the build compiled; `target/` is a folder on the larger disk, not a link.
  - The warm job lists doc tests with `cargo test --doc -- --list`; runs started by hand run the warm job's commands (`tools/warm-compile.sh`).
  - A split targeted run passes when a runner's share holds no test.
  - A test that passes on retry does not fail the job.
- Cross-check (`tools/crosscheck/`):
  - `tools/crosscheck/run.sh` ends what it started on a signal.
  - A node that loses its RPC port picks a new one and starts again (`tools/crosscheck/core_node.py`).
  - The stamp ignores a crate's own version line and hashes a manifest outside the tree whole (`tools/crosscheck/stamp.py`); it writes the manifest again before the record names the tree.
  - The vault check asks only about bytes it reads, names the tool that failed, and retries one save where KeePassXC 2.7.10 refuses the first (`tools/crosscheck/area_vault.py` and its shared helpers).
  - A multisig file with no Format line is P2SH; a Format naming no script is refused (`tools/crosscheck/area_import.py`).
  - A second wallet's outputs are that wallet's own, not change (`tools/crosscheck/area_review.py`).
  - Known differences with Bitcoin Core count only where Core's own reading shows the condition (`tools/crosscheck/area_core_wallet.py`).
- Lint scripts: a failing awk fails the lint; mawk and gawk agree (`tools/lint-secrets.sh`, `tools/lint-strings.sh`, `tools/lint-tokens.sh`).
- Snapshot tours: tour wallets are one wallet of one network and tours run to their end; the default tour goes through Review and Check after Collect changes the PSBT.
- `.gitignore` ignores Python bytecode (`__pycache__`); the tree stays clean (`tools/lint-tree.sh`).

## Domain calls

None.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 635fbe3d5 | Windows programs loaded uxtheme, dwmapi and propsys from their own folder before main (a search-path risk). | The delay-loaded imports are exactly dwmapi and uxtheme; propsys is loaded by name from System32. |
| 9548a34ed | The delay-import writable check read only some slots. | It reads every slot. |
| ac7e934af | The cross-check's multisig parser rejected a file with no Format line. | No Format line is P2SH; a Format naming no script is refused. |
| ec29382d2 | The review cross-check's reference judged a second wallet's outputs as change. | They are read as that wallet's own. |
| 529b63125 | The vault cross-check asked keepassxc-cli about a byte it never reads. | Only bytes read are asked; a failure names the tool that failed. |
| a93a9199a | The stamp counted a crate's own version line, so a version bump broke combining records. | The stamp ignores it. |
| 88b8d8dad | `stamp.py` mishandled a manifest outside the tree. | It is hashed whole. |
| 68d763df3 | The CI flaky-test summary failed a job for a test that passed on retry. | A pass on retry does not fail. |
| 473ec98c9 | The dev image did not build: a command-line word the dev part takes off by name was asked of the dev variant. | The dev image builds; the kernel-config check accepts the dev variant's word. |
| 118b57011 | A stalled package mirror hung a CI test job in `apt-get update` for over 14 minutes with no error. | Each package step has a time limit and three tries. |
| 4f6522c98 | The warm job's doc-test compile used a flag cargo refuses (`--doc` with `--no-run`), so the job failed. | `cargo test --doc -- --list`. |
| cebebe9e5 | Runs started by hand did not run the same commands as the push-to-main warm job, so a refused command failed main's check and not the hand run. | Runs by hand run the warm job's commands. |
| 701f8e5fc | Cross-check runs failed when several workers started Bitcoin Core nodes at once, because another node took the chosen RPC port in between. | A node that loses its port picks a new one and starts again. |
| 5f526578b | A release commit's record was refused by staging, because its fingerprints were stamped after the run, so the tree never matched. | The record names a tree a release commit can have; other differences are still refused. |
| dcba172b9 | A signal to `run.sh` left the areas and the OVMF run going; a failed OVMF run still left a record that every answer agreed. | `run.sh` ends what it started on a signal; a failed OVMF run is written in the record's result line and staging refuses it. |
| 6f75e6123 | crypto-bigint and ctutils were pinned to versions upstream yanked, so the locked build would not reproduce. | The locked build uses the published, unyanked versions. |
| ef4ee7148 | The fuzz workspaces' lockfiles did not name the zeroize edges the main workspace's crates now had, so they could not build with `--locked`. | The lockfiles match; builds pass `--locked`. |
| 1717ea831 | The seed-parts fuzz target no longer compiled, since the SLIP-39 share no longer derives Clone; the fuzz lockfile lacked a dependency. | The fuzz target copies a share through `Share::duplicate`; the lockfile matches the dependencies. |
| 97d7b67e3 | Two builds of one commit did not give the same boot and data FAT volume serials (both partitions stamped alike). | The serial comes from the commit date and the partition role, so partitions differ and builds match. |
| 124694654 | Two builds of one commit still differed in the FAT volume-label time (dosfstools ignores `SOURCE_DATE_EPOCH`). | `mkfs.vfat --invariant` on each vfat block. |
| cdc30232c | The four parsers of device input (faraday-bip, faraday-codec, faraday-fat, faraday-psbt) built without overflow checks in release, so a wrapped length read on at a wrong bound instead of stopping the app. | An overflow in a parser of device input stops the app; cost measured under 0.1%. |
| 330e138c9 | Published binaries embedded absolute source paths in panic and assert messages, and two builders in different directories produced different bytes. | A published binary carries no home directory or user name in its strings; builds reproduce across directories (path remapping). |
| d3a558bfd | A release staged only two images (the PC stick's Strict and Compatible files) rather than every image with release files in `faraday/builds.toml`. | A release publishes every built image listed in `faraday/builds.toml` with a release name, plus its boot programs. |
| ec1b9686c | `.gitignore`'s `__pycache__` line had lost its separator and matched nothing, so Python bytecode could be committed. | Build output is ignored; the tree stays clean. |
| feb5ab176 | A stray line in the kernel option list was read as an option name, so the stick's and the Pi's kernel checks refused it and the image builds failed. | The kernel forbidden list holds only real option names; KCMP stays in the Pi Strict and PC lists. |
| ae13b146a | The fuzz corpus export left a crash file at the top of the checkout when `cargo fuzz cmin` met a crash. | The corpus export writes crashes under `fuzz/artifacts/<target>/` only. |
| 271c2c5bc | Four CI tests failed on GitHub ("No such file or directory") because paths fixed by `env!` did not match the unpacked archive. | Test jobs unpack the archive into `target/` where the build compiled. |
| 02a5f46e7 | Every CI test job's unpack into `target/` failed ("trying to unpack outside of destination path") because `target/` was a symlink. | `target/` is a folder on the larger disk, not a link. |
| 4fcb2cc3d | The image build container lacked fdisk, so sfdisk could not check the stick table; `stick-finish.py` named a missing tool. | The build container carries the tools its scripts call. |
| d325a4754 | After a cross-check stamp, the Learn blocks quoted a stale seed, so `gen.py check` failed until `gen.py write` ran; the record named a tree without the change. | The stamp writes the manifest again before the record names the tree. |
| 1a55f6dfb | `keepassxc-cli` save failed with KeePassXC 2.7.10, so the vault cross-check failed on a good build. | The cross-check makes one save retry where 2.7.10 refuses the first. |
| d3f176867 | lint-strings' regex made mawk 1.3.4 panic, and lint-secrets, lint-strings and lint-tokens passed when their awk failed or wrote to stderr. | A failing awk fails the lint; mawk and gawk agree. |
| 0db876f21 | The GitHub check's Ubuntu package install failed when Ubuntu's own archive did not answer. | The install falls back to a second public mirror. |
| ff8b0bf2c | The cross-check reported differences with Bitcoin Core on inputs where Core's own reading shows no difference. | Known differences are counted only where Core's reading shows the condition. |
| cf89e36b2 | The glance and chart tours made a third cosigner key on the wrong network, so the 2-of-3 wallet was mixed-network and the tours stopped early. | Tour wallets are one wallet of one network; tours run to their end. |
| 751f1a6d9 | The default tour did not go through Review and Check after Collect changed the PSBT, so Files and Rename showed no files. | Finish follows Review and Check for a changed PSBT. |
| 96d6663a1 | A split targeted run failed with "no tests to run" when a runner's share held no test. | Empty shares pass. |
| 696d26d0b | The release strings check refused the desktop app, because every build carries "(local build)" in its strings table. | The release check reads the desktop app's build ID from its `--version` line. |

## Doubts

- No code report covered the build, CI or tooling files; this file rests on the history's fix rows and on the existence of the files named. No line numbers were checked.
- The exact files of cdc30232c (the four parsers' `Cargo.toml`s) and of ef4ee7148 (the fuzz lockfiles) were not confirmed.
- Two `stage-release.sh` paths appear in the history; only `faraday/tools/stage-release.sh` exists today.
- 1717ea831 named `faraday/fuzz/fuzz_targets/seed_parts.rs`; the target is at `fuzz/fuzz_targets/seed_parts.rs` today.
- Row left out as not about tooling: 3b230db80 (the SQLite parser accepting a key/value table whose CREATE was not Bitcoin Core's; it belongs to the Core wallet import, `core-wallet`).
- Several rows are tooling breakage, not app behaviour; they are kept because each was a real failure of a build or check.
