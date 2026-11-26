# Developer card and diagnostics

Two kinds of checking. The self-test is a product feature: at start, before
anything can be pressed, `faraday-core` runs a fixed set of published vectors
through the functions the product itself calls; a failure names the check and
blocks the app, and Settings shows the result with Run again. The rest is for
developers on the dev image (the dev card on the Pi, a dev stick) and on
Faraday Demo: timings (`--timings`), the boot report (`--boot-report`) and
verbose lines (`--verbose`), written as files to the boot stick. Apart from a
short dev notice over the top of the frame, none of the dev part appears on
screen.

## Screens and sheets

- A failed self-test screen (`selftest_failed`, `faraday/faraday-core/src/screens/mod.rs:~1016`): the app is replaced by a screen naming the failed check.
- Settings' self-test row (`faraday/faraday-core/src/screens/settings.rs:~476-500`): "passed" with the number of checks (`settings_selftest_passed`, Ok tone), "failed" naming the check (`settings_selftest_failed`, Error tone), or not run (`settings_not_run`); a Run again button (`Action::SelfTestRun`). While a run is under way, Run again shows Running and is disabled; a snackbar says the result.
- The dev notice of the stick shell (`faraday/shells/stick/src/notice.rs`), shown `NOTICE_FOR` 3 s after a dev file is written (see `stick-shell`).
- No other screen. Faraday Demo prints slow-frame and fade-count lines to stderr.

## Actions

- `Action::SelfTestRun` (`faraday/faraday-core/src/action.rs:~1478-1485`): runs `faraday_selftest::run` again; a failure closes any sheet; the result is said.
- Shell flags (dev): `--timings`, `--boot-report`, `--verbose`, `--frames`, `--files` (`faraday/shells/stick/src/main.rs`).

## Effects

- Self-test: `faraday_selftest::run(&c)` runs once at start, when the first display arrives and before any input (`faraday/faraday-core/src/drawing.rs:~990-995`). The checks (`CHECKS`, 9 of them, `core/faraday-selftest/src/lib.rs:~53`): BIP-39 words and seed (PBKDF2-HMAC-SHA512), BIP-32 derivation, a BIP-84 address, ECDSA and BIP-340 signing, scrypt and Argon2id (`core/faraday-selftest/src/lib.rs:~1-17`). `Outcome = Result<usize, &'static str>` (`core/faraday-selftest/src/lib.rs:~102`).
- Files (`faraday/shells/stick/src/files.rs`): `Files { dir, on_device }`; `write_named` and `write_whole` (temporary file, write, `sync_all`, rename) and `append_timings` (create and append, `sync_all`) to `faraday-timings.txt` and `faraday-boot-report.txt`. On a stick with a disk process, timings go through `write_copies` instead (`faraday/shells/stick/src/main.rs:~1412-1438`); `Files` is used only without a disk.
- Timings (`faraday/shells/stick/src/timings.rs`, `Timings`): tallies per bucket, work rows (24 at most), screen rows (24), fade steps (40), ghost-row lines (8 x 6 bands); blocks reset every 5 s; the whole file handed out at most every 30 s; kept under `MAX_KEPT` 1 MiB (oldest runs dropped, then oldest blocks); overflow goes to "other work" and "other screens". A `timings-carry` file in the run directory carries the run number and text across a lock. Timings say what fades did.
- `Shell::report` (`faraday/shells/stick/src/main.rs:~1392-1463`) writes the timings.
- `faraday/shells/stick/src/costly.rs`: `Watch` times an event; `Timed` forwards every `Sticks` call and times file writes; taken only with `--timings` (used at `faraday/shells/stick/src/main.rs:1263, 1590, 1603`).
- Boot report (`faraday/shells/stick/src/bootreport.rs`): `Report { name, inventory, system: Sampler, schedule, said }`; writes `faraday-boot-report.txt` first (retried every `RETRY` 1 s until it lands), then every `WRITE_EVERY` 30 s, and always at the end (`faraday/shells/stick/src/bootreport.rs:32-47, 90-132`). It uses `files::BOOT_REPORT` and `Sampler::start("/")` (`faraday/shells/stick/src/bootreport.rs:60, 62`). `kernel_log` runs `/bin/dmesg` (`DMESG`) with no timeout and keeps the newest `MAX_LOG` 1 MiB cut at a line start. Inventory: panel, input names, camera nodes (`/dev/video*`), `input: none` and `camera: none` when empty; `faraday_crypto::pin_failure_to_report` (`faraday/shells/stick/src/main.rs:3065`).
- `Shell::write_boot_report` (`faraday/shells/stick/src/main.rs:~1465-1538`).
- `faraday/shells/stick/src/devnodes.rs::block`: lists `/dev/dri` (`absent`, `empty`, per-entry lstat, `unreadable`), at most 256 entries.
- `faraday/shells/stick/src/radios.rs::block` (read-only): `/sys/class/bluetooth`, `/sys/class/net`, device-tree radio nodes by `compatible` (walk capped at 4096 nodes; no tree gives one "not available" line), consoles, `/etc/inittab` getty lines (a non-comment line with "getty" and the port as a whole word), serial ports listed only if `sys/class/tty/<n>/device` exists.
- `faraday/shells/stick/src/system.rs` `Sampler`, thread "board-sampler": once a second reads uptime, meminfo, cpu0 `scaling_cur_freq`, up to 8 thermal zones (skipping unreadable), the hwmon undervoltage alarm; the first frame time once; peak and current memory from `/proc/self/status`. It runs on its own thread so the drawing loop is not held; it stops when dropped.
- `faraday/shells/stick/src/clock.rs` `Boost`: for each cpufreq policy whose governor is `schedutil` and whose min and max are readable, writes `scaling_min_freq` to max while input or frames arrive and back to min after `HOLD` 1 s of stillness (`faraday/shells/stick/src/clock.rs:50-70, 105-117`).
- `faraday/shells/stick/src/work.rs`: the `wake` closure `try_send`s `Wake::Work` (dropped if full; the loop takes the answer on its next pass); `running(app)` is `app.busy().is_some()` (`faraday/shells/stick/src/work.rs:22`).
- Faraday Demo and the window shell: slow-frame lines (not in release builds, over 50 ms, once a second, `faraday/shells/demo/src/main.rs:~207-229`); a fade-count line per drawn frame (`~362-386`); `--timings` frame figures (`faraday/shells/window/src/pace.rs:~141-154, 192-278`).
- The boot report's camera lines are every `/dev/video*` entry, not only capture nodes; the scan sheet's camera list is only nodes whose sysfs `index` is 0 (`faraday/shells/stick/src/bootreport.rs:206-221`, `faraday/shells/camera/src/lib.rs:97-126`).
- Faraday Demo prints a "fades" line on stderr with each timings line, only with `--timings` (any build, release too); neither the Demo nor the core checks for a release build (`faraday/shells/demo/src/main.rs:362-384`, `faraday/shells/window/src/view.rs:118`, `faraday/shells/window/src/pace.rs:116`).

## State and lock

- `selftest: Option<faraday_selftest::Outcome>` on `Faraday` (`faraday/faraday-core/src/lib.rs:~1054`); `Outcome` is `at_lock::Public` (`faraday/faraday-core/src/at_lock.rs:~94`). The process ends at a lock, so the next process runs the self-test again at start.
- Timings carry across a lock only through the `timings-carry` file (an unreadable or unparsable file gives run 1).
- `Timings`, `Report`, `Sampler`, `Watch`, `Work` hold no secret. `faraday/shells/stick/src/costly.rs` holds only references; bytes, copies and vault keys pass through by reference and are not kept. `vault_label` names only act, strength, memory, passes and end reason (no name or passphrase). The boot report holds no key material by design.
- `faraday/shells/stick/src/clock.rs` has no `Drop`: a raised clock is not restored on exit or crash in this file.

## Reach

- The self-test gates everything: `may_load_keys` needs it passed (`devices-inputs`, `files-inbox`); Faraday Demo's host Load is pressable only when it passed, a box is ticked and no job runs (`faraday/faraday-host/src/screen.rs:1480-1484`).
- Settings shows its result and Run again.
- The dev files are written through `storage` (`write_copies` on the boot stick) or `Files` without a disk; the stick shell (`stick-shell`) runs the reports from its loop.
- `faraday/shells/stick/src/costly.rs` calls `faraday_core::work::{snapshot, take_vault_work, since, Counts::of}`.

## Rules and refusals

Self-test:
- A build whose vectors do not reproduce does nothing but exit: with a failed self-test every press is dropped except PowerOff, which runs `exit_forgetting` (`faraday/faraday-core/src/action.rs:1405-1410`).
- `selftest_failed` is `matches!(self.selftest, Some(Err(_)))` (`faraday/faraday-core/src/tools/mod.rs:1019-1021`).
- The self-test runs before any key is accepted; its expected values are copied from the vector files the integration tests use, and from libsecp256k1's copy of the BIP-340 vectors (`core/faraday-selftest/src/lib.rs:~8-14`).

Dev files and reports:
- On the device (no `--files`) every `Files` write is refused with "the exchange partition is not mounted" (the exchange directory is the initramfs); `mounted()` is checked first in both writers; errors carry the path and never file contents (`faraday/shells/stick/src/files.rs:40-87`, `80-86`).
- No clock is read for timings unless `--timings` (`send_timed`, `faraday/shells/stick/src/main.rs:~1378`).
- `Shell::report`: nothing new while a write is out; a stick write error without a disk process turns timings off and says so once; the boot stick must be listed to write timings. The timings file is written to the boot stick.
- `costly::label` names the row by vault notes if present, else the first non-zero of `ORDER` (20 kinds, `faraday/shells/stick/src/costly.rs:32-53`).
- `Shell::write_boot_report`: not due unless scheduled; not while a write is out; with a disk process and no boot stick listed it retries in 1 s; a failure is said once. A missing kernel log is written as "no kernel log: why".
- `Sampler`: "not known: no kernel log" and "not in the kernel log" when absent; throttling flags are never read (reported as not available).
- `Boost`: a policy whose write fails is dropped; only schedutil policies are touched.
- The "dev card" is any image built with `DEV=1` (Pi card or PC stick): only that build passes `--verbose --timings --boot-report` to the app, and the image script fails a non-dev image whose arguments carry one of them (`faraday/image/build.sh:276-290`). The parser accepts the flags in every build (`faraday/shells/stick/src/main.rs:624-627`).

## Domain calls

- `faraday_selftest::run`, `run_checks`, `CHECKS` (`core/faraday-selftest/src/lib.rs:~53, 106, 112`), with `Compute`.
- `faraday_core::work::{snapshot, take_vault_work, since, Counts::of}`; `faraday_crypto::pin_failure_to_report`.

## Fixtures

None of the listed fixtures. The self-test's expected values are copied from vector files named in its module doc (`tools/vectors/bip39/vectors.json`, `tools/vectors/bip39/test_JP_BIP39.json`, `tools/vectors/psbt/rfc6979.txt`, `tools/vectors/kdf/README.md`); the code does not read them.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 798d0c73c | Settings' self-test Run again gave no sign it had run: no running state and no result when it ended. | Run again shows Running and is disabled while the run is under way; a snackbar says the result. |
| 86ec4713c | Dev stick timings never reached the stick: `--timings` wrote into a folder the Pi card did not mount, failed on its first block and turned itself off. | The timings file is written to the boot stick; timings say what fades did. |
| acf516aa5 | The dev stick stalled once a second, because a thermal zone read taking hundreds of ms ran on the drawing loop. | System figures are read on a thread of their own; the drawing loop takes the latest. |

## Doubts

- 86ec4713c and acf516aa5 came with the stick shell's rows and are carried here.
- `devnodes` (256 entries), `radios` (4096 nodes) and the dev notice (two lines) truncate silently.
- `faraday/shells/stick/src/timings.rs:~99` and `~801` have misplaced doc comments.
