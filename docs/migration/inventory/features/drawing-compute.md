# Drawing and costly work

The rules and plumbing behind every frame: the per-frame drawing memo and the `View` the screens read, the costly-work token (`Compute`) and its job queue, presses taken while a job runs, the background worker thread, and the costly-work counters that the budget tests and the dev card read. It has no screen; every feature that draws or does costly work goes through it.

## Screens and sheets

- No screen. The busy line and busy bars a running job shows (`Busy { kind, line, began_ms, done, of }`) are drawn by the screen frame and shared parts.
- The first display runs `faraday_selftest::run` once and makes the canvas (`faraday/faraday-core/src/drawing.rs:~988-1017`); a failure gives the self-test failure screen (`screen-frame`).

## Actions

- None of its own. It decides which presses are taken while a job runs (Rules).
- A press left waiting runs only after a frame is drawn (`later_tick`, `faraday/faraday-core/src/compute.rs:~50-56`), and not while a slider is held (`:~60-66`).

## Effects

- Jobs on one OS thread named "worker", spawned on first send, with `mpsc` channels and a wake callback after each answer (`faraday/faraday-core/src/worker.rs`).
- Job kinds: SamplesLoad, VaultStretch, KeysLoad, KdbxWrite, VaultMeasure (quiet), VaultSeal, AezeedRead, Signing, PsbtRead, Collecting, PublicFiles, KefSeal, KefOpen, KefTime (quiet), OtsHash, CoreWalletRead, SeedCompare, MadeOnline.
- `Compute::start` sends a `Job` to the worker or runs it inline (`faraday/faraday-core/src/compute.rs:~146-166`); `take_answer` and `drain_jobs` apply `Done::*` (`:~169-311`).
- KEF (Krux code) sealing: randomness from `fresh_bytes(b"kef code", ..)` (`faraday/faraday-core/src/compute.rs:~558-562`); the Outbox pair `.kef` file plus picture (`:~787-794`); vault store made-online marks.
- `before_frame` warms vaults, text files, the plan, TOTP codes, codex32 refit and the guide's place (`faraday/faraday-core/src/drawing.rs:~800-816`). `render_frame` (`:~818-974`).

## State and lock

- `Drawing` (`faraday/faraday-core/src/drawing.rs`): the per-frame memo, cleared after each render (`:~628, ~639`) and by a lock (`:~1021-1027`).
- `View`: the screens' view of the app, with a private compute token; writes go through `After`.
- Layout window facts: `nav_hidden` (rail and bar hidden while a BackupFlow stick is written or a key made, `:~427-434`), `nav_bar_hidden` (`:~442-467`), `one_column`, `touch_side`, `note_up` (never when held).
- On `Faraday` for compute: `later: Option<Later>`, worker busy, origin and generation, the Krux state.
- Secrets in flight: the KEF passphrase, plaintext and sealed bytes are `Zeroizing<Vec<u8>>`, and an opened code is `faraday_keys::kef::Opened` (`faraday/faraday-core/src/compute.rs:~541, ~641, ~651`). Job inputs carry seeds, passphrases and PSBTs to the worker thread; each job type keeps its own.
- Seed bytes shown while drawing are borrows: `reveal_record` lends a borrow behind `Allowed::SecretShown`; no copy is kept (`faraday/faraday-core/src/drawing/allowed.rs:94-106`). `record_words_held` is a borrow, only while the eye is open (`faraday/faraday-core/src/drawing/allowed.rs:68`).
- Lock:
  - Lock stops the Krux state and asks the made-online check again (`faraday/faraday-core/src/compute.rs:~453-461`).
  - `Worker::locked()` and `stale()` only set `live = None`. Running and queued jobs are not cancelled; their answers are dropped where they arrive (`faraday/faraday-core/src/worker.rs:414-426, 555-562`).
  - `AtLock for Worker`: `holds` = an opened Krux code is held; `lock()` calls `kef.clear()`. `busy` and `quiet` are not cleared.
  - Every lock then ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Counters (`faraday/faraday-core/src/work.rs`): thread-local `COUNTS` (`[Cell<u64>; 22]`), `count`, `core_count`, `snapshot` (adds plan and paper counters), `since`, `absorb`. Vault timing notes `VaultWork`, `VaultAct`, `VaultEnded` (thread-local, at most 8, extras dropped); `time_vaults(on)` is an atomic, so the clock is read only when asked. No secret.

## Reach

- Used by every drawing feature and every costly one: vaults (`vault-unlock`, `vault-seal`, `vault-list-create`), Krux codes (`krux-codes`), signing (`spend-signing`), public files (`public-files`), timestamps, Core wallet (`core-wallet`), seed compare (`seed-checks`), Lightning reads, samples.
- Worker job types: `samples`, `vault::*Work`, `lightning::ReadWork`, `signing::*Work`, `public_files::FilesWork`, `ots_check::HashWork`, `core_wallet::tool::ReadWork`, `seed_check::CompareWork`, `faraday_keys::job::MadeOnlineJob`.
- Collect and read answers are applied at `faraday/faraday-core/src/compute.rs:287-288`.
- The drawing cache `faraday/faraday-core/src/drawing/kept.rs:1140`.
- Read by the dev card's timings and by the budget tests (`test-kit`).

## Rules and refusals

- `Compute::new(work::core_count)` is the only mint of the work token (`faraday/faraday-core/src/compute.rs:~33-35`). Costly work takes `c: &Compute`.
- `taken_while_busy` (`faraday/faraday-core/src/compute.rs:334-366`): while a job runs, only presses that keep the screen live are taken (`stays_live`). Back and Cancel from a screen other than the job's origin are taken and leave the job. Back and Cancel are refused while a seal or the made-online check runs. Otherwise Back and Cancel make the job stale, stop the KEF run and clear busy.
- `key_refused_busy`: every key but Escape is refused while busy (`faraday/faraday-core/src/compute.rs:~410-412`).
- `kef_seal` refuses a version with no IV length, an id longer than `ID_MAX`, and no fresh bytes (`NoFreshBytes`) (`faraday/faraday-core/src/compute.rs:~539-562`).
- `kef_make_seed` offers only a 15, 18 or 21-word seed, else `NotOffered` (`:~687-712`).
- An options problem, or a passphrase that is not UTF-8 or not writable, gives `kef_refuse_passphrase` (`:~735-742`).
- Worker: `send` makes a job the live one, so an earlier job's answer is ignored (`is_live`); `send_quiet` adds a quiet generation that is not made stale.
- `latest_wins` returns false for every kind (`faraday/faraday-core/src/worker.rs:60-90`), so the "newer job replaces one not started" branch never runs.
- A job panic is `Err(())` only in a test build; a release aborts.
- `VaultEnded::of_error` (`faraday/faraday-core/src/work.rs:~273-290`): memory errors are `NoMemory`, format errors `Refused`, everything else (a wrong passphrase included) `Ran`. `VaultWork::strength` maps to a `vault::PRESETS` name.
- Since the history fixes: while a job runs, refused non-button presses are drawn disabled, and a storage event that moves the screen makes the job's answer stale.
- Shared drawing rule from the repository's instructions: drawing reads and does not work things out.

## Domain calls

- `faraday/faraday-keys`: `kef::{seal, open, iv_len, ID_MAX, writable_passphrase, count_with_change, seed_plaintext}`, `job::MadeOnlineJob`.
- `core/faraday-selftest`: `run` at first display.
- `core/faraday-crypto`: `Compute`.

## Fixtures

None. (The budget tests name `tools/vectors/psbt/{signer-core-sent,signer-core-signed,bump-core-original}.psbt`, which are not in the kept fixture list.)

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| f6ff11b56 | Toast text was not centred, and a toast on Home drew over the Scan button rather than above it. | Every toast is centred and stands above the frame's floating action button, as Material places it. |
| b6300ccc0 | Under a sheet, a frame drawn in several passes kept the ink of each pass, so each line was recorded twice (seen only by the recorded-ink audit and tests). | Canvas restore resets the recorded ink as clear does. |
| aa1f03d51 | The Check place's shield icon was drawn smaller than the house, file and lock icons beside it. | Icons are drawn to one height. |
| a3ee8df58 | A passphrase field that takes no typing while a vault job runs was drawn as a filled box with an eye box, which made the field look cut in two. | A disabled passphrase field is one disabled outlined text field (Material 3). |
| 820d09cb0 | While a job ran, refused list rows, links, chips and cards were drawn enabled, so they looked pressable; a stick put in or taken out that moved the screen did not make the running job's answer stale. | While a job runs, refused non-button presses are drawn disabled; a storage event that moves the screen makes the answer stale. |

## Doubts

- Counts are thread-local, so a worker's counts may not reach the main thread's snapshot.
- The `inline` flag on the worker is stored but no run path in `faraday/faraday-core/src/worker.rs` reads it.
- `kef_take_made`'s doc (`faraday/faraday-core/src/compute.rs:~757`) says "Costs a QR encoding", which one report thought wrong.
- `is_strip_room`'s doc mentions the self-test but the code does not (`faraday/faraday-core/src/drawing.rs:~360-362`). The `drawn_*` accessors are `pub` and mostly not gated to the test kit.
- Several screens write state while drawing (camera pane, scroll extents, guide scroll, `drawn_check`, `public_qr`, `wordlist_scroll`, `note_dice_body_h`, `note_sheet_room`); which of these are memos and which are real state is undecided.
- Some drawing does work (Spend from backup drawing, `restore::merge`, `read_key`, address derivation per draw); whether results are cached elsewhere is unclear.
- `faraday/faraday-core/src/work.rs:~389-390` gates `work/budget` as test code; that gate was not re-read.
- `memory_mib * 1024` is a plain `u32` multiply over fixed presets.
