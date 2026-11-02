# Costly work off the drawing thread

**Kind:** Spec
**Status:** Specified, not built; waits on the owner's answers in §10
**Open:** all of it: the core's worker (§3), the busy screen (§4), the token on the worker (§5), key material in a job (§6), the shells' wake and the stick's writes (§7), the jobs themselves (§9)

Touch and drawing run on each shell's own thread, the main thread; every
other computation runs on a thread of its own (owner, 2026-10-31,
2026-11-02). This holds for every Faraday: the stick and the Pi
(`faraday/shells/stick`, one shell for both) and Faraday Demo
(`faraday/shells/desktop` over `faraday/shells/window`). The design is
one worker in the core, so every shell that holds a `Faraday` gets it
with no code of its own but a wake.

## 1. What runs where today

| Work | Stick and Pi | Faraday Demo |
|---|---|---|
| Input | Readers on threads of their own (`evdev.rs`, `touch.rs`, `pointer.rs`); the main loop takes their `Wake`s | winit's event loop, on the main thread; nothing costly |
| Camera capture | A thread (`camera.rs`), which also reduces the frame | A thread (`faraday_window::camera`, `faraday_v4l2::spawn` on Linux, Media Foundation and AVFoundation elsewhere) |
| QR codes in camera frames | **Off the main thread already**: `faraday_scanner::Scanner`, a worker that takes the newest frame offered and drops the one not taken (`ReadMode::Reduced`) | The same worker (`faraday/shells/window/src/lens.rs`, `ReadMode::Full`) |
| What a scanned code leads to | On the main thread: `Event::Scanned` into `Faraday::scanned` (`scan_key.rs`): a UR or BBQr part joined, a PSBT parsed and inspected, a key or wallet read | The same |
| Pictures on a stick, their codes | The disk process's own thread (`read_qr_later`, `docs/SPEED.md` §9 point 1) | `Dirs`' pictures thread (`faraday_storage::Pictures`) |
| Key stretching (PBKDF2, scrypt, Argon2) | On the main thread, the tick after the frame that shows the press (`slow`, `Later`, `Faraday::press_in`; SPEED §9 point 2): the screen stands still for its length, seconds on a Pi | The same, shorter |
| Signing, a transaction checked, pictures and PDFs made, QR codes made | On the main thread, inside the press (the slow ones the tick after, as above) | The same |
| Files written to a stick | On the main thread: a request to the disk process and a wait for its answer, with a timeout (`DiskProcess`, `costly::FILE_WRITTEN`) | On the main thread: `serve_with` into `Dirs`, plain writes to the stick folders |
| Stick folders listed, Downloads, cameras | Stick listing on the main loop (`Shell::mounted_sticks`) | A thread (`watch_listing`) for Downloads and the cameras; the stick folders on the main thread (`Demo::sticks`), four bytes a file at most |

Two corrections to the problem as first stated. Camera frames are not
decoded on the main thread in either shell: since the scanner crate,
the core is handed the reduced frame to draw and the scanner the full
one, and a decode slower than the frames only means frames the scanner
never reads. What does hold the Scan sheet is what a code leads to: the
last part of an animated PSBT ends in a parse and an inspection on the
main thread (`Work::PsbtParse`, `PsbtInspect`), and the sheet stands
still for them. And the panic chord is timed by the stick's main loop
(`keyboard::PanicTimer`, polled in `main_loop`): while a key is
stretched there, the chord held for its whole time does nothing until
the stretch ends. Moving the work off the main thread fixes both.

**The Starter.** Its own work already runs on threads (the disks,
the concerns, BitLocker, every write; SPEED §11.2), and only its PDFs are
made on a press, which are short. The Demo it holds under its top app bar
(F126) is a `Faraday` and shares this path: it gets the worker through
the core, and the Starter's window gives it the same wake as Faraday
Demo's (§7.2).

## 2. Rules

1. **The main thread draws and takes input.** A shell's main thread
   hands the core its events, draws, presents, and times the panic
   chord. Nothing on it works out a key, a signature, a picture, a code
   or a file, nor waits on a disk.
2. **Drawing reads.** The busy screen (§4) reads what the core keeps
   of the job (`Busy`: what it is, when it began, a count where it has
   one); it works nothing out (`docs/SPEED.md` §3.3, §7;
   `docs/COMPUTE.md` §4).
3. **A job changes nothing until it is applied.** A job is given what it
   needs by move when it starts, runs with nothing of `Faraday`, and its
   result is applied by the core on the main thread, on a later pass, as
   `read_qr_later`'s answer is.
4. **The same answer as before** (SPEED §2 rule 5). A job gives what the
   press gave when it ran inline: the same vault, the same signed bytes,
   the same files.
5. **Nothing waits on a result that will not come.** A lock, Back or a
   new job of the same kind makes the result stale; a stale result is
   dropped where it arrives, and wiped as it drops.

## 3. The worker

### 3.1 One worker, in the core

`faraday-core/src/worker.rs` (new) holds `Worker`: one thread, made when
the first job is asked for and ended when the `Faraday` that owns it
drops (a lock starts a fresh app in every shell, so a lock ends it). It
runs one job at a time, in the order asked.

One worker, not one per kind: two key stretches at once would ask for
two Argon2 memories, and a vault at Strong strength is most of a Pi's
memory; and the person waits on one thing at a time, so a second thread
would buy nothing they see. The scanner and the pictures thread stay
threads of their own, since they already are and serve another shell
concern (frames and files, not presses).

The core owns it, not each shell, so that every shell, the snapshot
tool and the tests get one behaviour; a shell's part is the wake of
§7.

### 3.2 A job

```rust
pub(crate) enum Job { Stretch(StretchJob), Sign(SignJob), Read(ReadJob), Make(MakeJob), … }
pub(crate) enum Done { Stretch(Stretched), Signed(SignOutcome), … }

struct Asked { job: Job, gen: u64, kind: Kind }
struct Answer { done: Done, gen: u64, counts: work::Counts }
```

- A job is the costly part of a press, cut out of today's `act` arm:
  the arm's front half reads the state and builds the job (cheap), the
  worker runs it, and the arm's back half applies `Done` (cheap). Each
  `Job` variant is a plain struct that is `Send` and owns its inputs;
  key material goes in only inside faraday-keys' own types (§6).
- `Faraday::start(job, busy)` gives the job the next generation number,
  sets `self.busy` (§4), and sends it to the worker. The worker mints its
  own token (§5), runs `job.run(&c)`, and sends `Answer` back with the
  counts it did, then calls the shell's wake (§7).
- The core drains answers at the top of every `event`, `tick` and
  `storage` call. An answer whose generation is the current one is
  applied by its area (`Done::Stretch` to the vault's or the seed's
  area, and so on) and clears `busy`; any other is dropped.
- **Latest wins** for the jobs that follow typing (the passphrase probes
  of `Sheet::Potential` and `Sheet::SeedRead`, Load a key's probe of the
  words, SPEED §9 point 2's typed pause): a new one replaces the one not
  yet started and makes the running one's answer stale, as the scanner
  takes the newest frame.
- **Inline.** A scripted press (`Faraday::press`, the snapshot tool,
  most tests) runs the job on the calling thread with the caller's
  token and applies it at once, as it does the work at once today, so
  the tours and the tests' expectations do not move. `testkit::settle`
  waits for the worker and applies its answer, for the tests of the busy
  screen (§8).
- A job that panics: the release builds abort on a panic (as a panic on
  the main thread does today); a test build carries the panic back as
  the answer and fails the test that asked.

### 3.3 What the worker replaces

`Later`, `later_drawn`, `later_tick` and `later_now` go once every press
on `slow`'s list is a job (§9, W9): a press is drawn at once because the
work is no longer on the thread that draws. `slow` becomes the list of
presses that start a job, and the debug guard in `press_in` stays as it
is: a press not on it that stretches a key or makes more than one
picture on the main thread panics, naming itself. SPEED §9 point 2 is
rewritten to say this in the batch that removes `Later`.

## 4. The screen while a job runs

`Faraday.busy: Option<Busy>`, public and read by drawing: the job's
`Kind`, its line of text, the time it began, and `done`/`of` where the
work has a count (Argon2's passes, a PSBT's inputs signed, a set of
pictures).

**What is drawn.** The screen that started the job stays, with its
working line and Material's linear progress indicator along its foot
(`working_bar`, DESIGN §4.14): **indeterminate** and moving, as Material
draws it, where the work has no count; **determinate**, its track filled
by the share done, where it has one; still under reduce motion, as
today. Its other actions are drawn in Material's disabled state. The
page checked against is Material 3's Progress indicators
(m3.material.io/components/progress-indicators), "Linear" and
"Indeterminate"; the disabled buttons against Material 3's Buttons,
"States". A sheet that waits today with a still bar (a vault's unlock,
creation and sealing, an unlock from the boot import and the Spend
tab) keeps its text and gets the moving bar. The shell is asked for
frames while `busy` is set, at the motion pace (`motion.rs`), and for
none once it clears.

**What is taken, what is refused, what cancels.**

| While a job runs | What happens |
|---|---|
| The same press again, or any other action of the screen | Refused: the buttons are disabled, and a press on one does nothing |
| Back, Cancel on the screen or sheet that started the job | Accepted: the screen goes back as it would, the answer is made stale and dropped when it comes; what the job was given by move (a typed passphrase) is gone with it, so the person types it again |
| A tab, the rail or the menu | Refused, drawn disabled (§10, question 2) |
| Lock | Accepted at once: the session is dropped as a lock drops it, the worker ends with the app, and the answer, when it comes, has nowhere to go and is wiped on the worker |
| The idle warning and the idle lock | The idle clock does not run while a job runs and starts again from its end (§10, question 3); a job is the person's own press, and today the frame it held let no idle tick through either |
| The panic chord, Power off | Taken by the shell on its main thread at once, the job or not; the machine goes off with the worker mid-run (a key in its memory goes with the power, inside the Security scope) |
| Theme, text size, rotation, Faraday Demo's preview size or window size | Accepted: they change what is drawn, not what the job reads |
| Typing into a field whose probe is the job | Accepted: each pause starts a new probe, latest wins (§3.2) |
| A stick put in or pulled | Taken as today (`storage` events); a job that reads or writes a stick is one of §7.3's, answered by the storage, not by the worker |
| A scanned code | Taken by the scanner as today; a code that would start a job while one runs is held until the job ends, as a stick plugged in under the camera is held back |

Strings: the working lines exist (`working_files` and the vault's);
each new job's line is added to `faraday-strings/src/en.rs` in its batch,
a plain statement of what is being done ("Opening the vault").

## 5. The token on the worker

The worker mints its own `Compute`, as the pictures thread does
(`faraday_storage::Pictures::ask`; COMPUTE §3): the token is still not
`Clone`, `Copy`, `Send` or `Sync`, and nothing moves one between
threads, so the doctests of COMPUTE §9 stand unchanged.

- The mint is one new row in COMPUTE §3 and in the costs lint's `MINTS`:
  `faraday-core/src/worker.rs`, the worker's loop, `new(work::core_count)`,
  under `#[expect(clippy::disallowed_methods, reason = "a mint: the
  worker, which draws nothing")]`.
- **Counting.** The counts are per thread (`faraday-core/src/work.rs`),
  so the worker takes `work::snapshot()` before a job and `work::since`
  after it, and sends the difference with the answer; the core adds it
  to the main thread's counts (`work::absorb`, new) as it applies the
  answer. A press's work is then counted once, on the thread that asked
  for it, whether it ran inline or on the worker; the budget tests,
  `slow_press.rs` and the dev card's rows (`costly::label`) read the same
  counts as before. The vault notes (`work::note_vault`) travel the same
  way.
- `job.run(&c)` is a costly function: it takes `c: &Compute`, says
  "Costs …" on its doc's first line, and the costs lint holds it as any
  other.

Whether this needs a decision: yes, one, unnumbered here. It binds later
changes (a new costly press is a job, not work in `act`), it changes a
built rule (SPEED §9 point 2: frame first, work on the tick after), and
it lets key material cross a thread inside faraday-keys (§6). Its rule
goes into `tools/impact.toml` in the batch that builds the worker (W1):
a change to `slow.rs` or to an `act` arm on its list changes `Job` with
it.

## 6. Key material in a job

F70 stands: key material lives in faraday-keys and leaves only through
its named reveals (`docs/KEYS.md` §2). A job's key material never leaves
faraday-keys' types; the thread it runs on changes, the crate holding it
does not.

**What moves to the worker.**

- **A stretch's secret input, by move.** A vault passphrase at Open, a
  passphrase typed for a BIP-39 seed, a SLIP-39 or aezeed passphrase:
  transient input (KEYS §2, "Transient input"), handed in by move into
  a `faraday_keys::StretchJob` (`SecretText` inside), so no copy is made.
  The result comes back inside faraday-keys' types (`Stretched`: a slot
  key, a seed) and joins the store through `Keys`' existing `add_*` or
  the vault's open, on the main thread. Both wipe on drop.
- **Signing, by shared reference to what the store holds** (§10,
  question 1). The store keeps each master key's secret behind an
  `Arc` that never changes once made; a `SignJob` holds a clone of the
  `Arc`, not a copy of the key, and the PSBT by value. A key forgotten
  or a lock while it signs drops the store's `Arc`; the job's keeps the
  key until it ends, and the key is wiped as the last one drops. A
  reblind (`Keys::reblind`) makes the key's new context for the store
  and leaves the job's to it.
- **A picture or PDF of secret words** (a backup stick's words, a
  SeedQR picture; `Reveal::BackupStick`, `Reveal::PaperForm`): the
  reveal is a guard that borrows the store and cannot cross a thread.
  The maker runs inside faraday-keys as a `MakeJob` that takes the
  revealed text into a `Zeroizing` it owns, a copy that lives as long as
  the job, and gives back the file's bytes as today's maker does. This
  is a new copy of a secret (SPEED §2 rule 1 is about kept values; this
  one is not kept), so it is the owner's (§10, question 4); until then
  these presses stay on the main thread, frame first, as today.

**What never moves.** The store (`Keys`), the open vault's records, a
`Revealed` guard, the codex32 work set, anything drawing reads. The
worker sees only what one job was given.

**How it is wiped.** Every secret a job holds is a faraday-keys type or
a `Zeroizing`, wiped as it drops: at the job's end on the worker, at a
stale answer's arrival on the main thread, or on the worker when its
app has gone. A power off wipes nothing more than today. Wiping copies
that freed memory still holds is the Security scope's later work, not
this spec's.

## 7. The shells

### 7.1 The stick and the Pi

- The main loop waits on `Receiver<Wake>`; the core is given a waker
  (`Faraday::set_wake(Box<dyn Fn() + Send>)`) that sends `Wake::Work` on
  a clone of the loop's sender. The loop's pass then hands the core its
  tick, which drains the answer.
- While `busy` is set the loop asks for frames at the motion pace, and
  the panel writes only what changed (SPEED §11.1), so a moving bar
  costs its rows.
- The panic chord is untouched: it is timed on the main loop, which no
  longer waits on work.

### 7.2 Faraday Demo, and the Starter's held Demo

- winit's loop takes a user event: the waker is the loop's
  `EventLoopProxy::send_event`, in `faraday/shells/window` so both windows
  share it (`faraday/shells/desktop/src/main.rs`, the Starter's window in
  `faraday/shells/starter/src/main.rs`).
- `Demo::sticks` stays on the main thread: four bytes a file of a
  vault's length, a lookup.

### 7.3 Files written to a stick

A write is a storage request answered by a storage event, as a picture
read is (`read_qr_later`):

- `faraday_storage::Sticks::write_later` (new): the stick shell sends
  the request to the disk process and goes on; the answer thread it
  already has (`DiskProcess`) hands back `StorageEvent::Written` or
  `NotKept` on a later pass, with the timeout as today. Faraday Demo's
  `Dirs` writes on its pictures thread, renamed for both.
- The core shows "Writing to the stick" with the indeterminate bar
  while a write it waits on is out; a stick pulled meanwhile ends it as
  `NotKept` does today.
- The boot report's write (`costly::DEV_FILE_WRITTEN`, a dev card's) goes
  the same way.

## 8. Tests

Each states what a person would notice, in the area's tests
(`faraday-core/tests/all/`) unless named.

- **The screen moves while a vault opens.** A vault opened through the
  worker: two frames drawn while it runs differ in the bar only, and the
  vault is open after `settle`, with the entries the inline open gives.
- **One press, one job.** Open pressed three times while it runs opens
  the vault once (the counts show one `Kdf`).
- **Back leaves it closed.** Back while a vault opens: the screen is the
  one before, after `settle` the vault is still closed, and its
  passphrase field is empty.
- **A lock wins.** Lock while a vault opens: the locked sheet shows, and
  after the answer comes (`settle`) no vault is open and no key is held.
- **The idle lock waits.** With the idle time short, a job that runs past
  it does not lock; the idle warning comes its full time after the job
  ends.
- **The same answer.** A PSBT signed through the worker gives the bytes
  the inline press gives; a vault made through the worker opens with the
  same passphrase in a fresh app; files made through the worker are the
  same bytes.
- **Typing goes on.** A passphrase typed during a probe: the fingerprint
  shown at the end is the one of the whole passphrase, and the counts
  show no probe applied but the last.
- **A scan goes on.** The Scan sheet with an animated PSBT: frames keep
  being drawn while its last part's PSBT is read and checked, and the
  review opens after `settle`.
- **The panic chord during work** (`faraday/shells/stick`, its tests of
  `PanicTimer` and the loop): with a job held running, the chord held
  for `PANIC_HOLD` asks for power off at once.
- **A write that is slow** (`faraday-storage`'s tests, a disk process
  that answers late): frames go on, and the file's toast comes with the
  answer; one that never answers ends as "not kept".
- The budget tests (`work/budget/`), `slow_press.rs` and the snapshot
  tours pass unchanged, since scripted presses run inline and the counts
  come home.

## 9. Batches

Each about fifty agent turns, in order. **Review** marks a batch that
touches keys, cryptography, signing or what the device takes in, and
gets the reviewer before it lands.

| Batch | What | Files | Review |
|---|---|---|---|
| W1 | The worker: `Worker`, `Job`, `Done`, generations, latest wins, inline mode, `work::absorb`, `busy`, the refusals of §4, the idle clock held, `testkit::settle`; the mint's row in COMPUTE §3 and `MINTS`; the decision's rule in `impact.toml`. The first job: Practice's samples loaded (`SamplesLoad`, public data), with its tests | `faraday-core/src/worker.rs` (new), `lib.rs` (`press_in`, the drain), `slow.rs`, `idle.rs`, `work.rs`, `testkit.rs`, `tools/costs-lint/src/main.rs`, `tools/impact.toml`, `docs/COMPUTE.md` | No (no key, no crypto); the mint is checked by the costs lint |
| W2 | The shells' wake and frames while busy: `Wake::Work`, the winit proxy for both windows, the snapshot tool inline; the panic chord test | `faraday/shells/stick/src/main.rs`, `faraday/shells/window/src/lib.rs`, `faraday/shells/desktop/src/main.rs`, `faraday/shells/starter/src/main.rs`, `faraday/shells/snapshot/src/main.rs` | No |
| W3 | The busy screen: the moving bar, determinate where counted, disabled actions, Back and Cancel stale-making; strings; DESIGN §4.14; one tour of a busy screen (`just faraday-snap 480x640 vault`) | `faraday-core/src/screens/parts.rs`, the sheets that use `working_bar`, `faraday-strings/src/en.rs`, `docs/DESIGN.md`, `faraday/shells/snapshot/src/tours` | No; a screen review against Material's Progress indicators |
| W4 | Vault stretches as jobs: Open, Make, seal again, the boot import's unlock, Spend's unlock; `StretchJob` and `Stretched` in faraday-keys, the passphrase by move | faraday-keys (a new `job.rs` in its `src/`), `faraday-vault`'s Argon2 call site, `faraday-core/src/vault/`, `boot_import/`, `docs/KEYS.md` | **Yes**: keys, crypto |
| W5 | The other stretches: a BIP-39 passphrase seed (Load a key, `SeedRead`, `Potential`, latest wins), SLIP-39 recovery, aezeed, KDBX write and verify, a chart's passphrase try | faraday-keys' `job.rs` (W4), `faraday-core/src/seeds/`, `glance/sheet.rs`, the KDBX writer's call site | **Yes**: keys, crypto |
| W6 | Signing and checking: `SignJob` with the store's keys behind `Arc` (on the owner's answer to §10 question 1), a PSBT's parse and inspection from a file or the last scanned part as a job, the Spend tab's signatures counted | `faraday/faraday-keys/src/session.rs`, faraday-keys' `job.rs`, `faraday-core/src/scan_key.rs`, the Spend and sign areas | **Yes**: signing, keys, QR intake |
| W7 | Pictures and PDFs of public values (a wallet's files, `PublicOut`, the checklist's), and on the owner's answer to §10 question 4 those of secret words, as `MakeJob` | `faraday-core/src/picture.rs`, `pdf.rs`, `wallet_backup.rs`, `backup_flow/`; faraday-keys' `job.rs` for the secret ones | **Yes** for the secret ones; no for the public ones |
| W8 | Files written as storage requests: `Sticks::write_later`, `StorageEvent::Written`, the stick's disk process answer, Faraday Demo's `Dirs` thread, the boot report's write | `faraday-storage/src/lib.rs`, `faraday/shells/stick/src/main.rs`, `files.rs`, `faraday-core/src/lib.rs` (the storage events), `faraday/shells/desktop/src/lib.rs` | No (no parser, no key); a backup's files are written as before |
| W9 | `Later` removed; every press on `slow` is a job; SPEED §9 point 2 rewritten; this spec's plan moves to docs/history/WORKERS-LOG.md and it becomes a Reference; one late-flow session timed as SPEED §7 says, the numbers in the report | `faraday-core/src/lib.rs`, `slow.rs`, `docs/SPEED.md`, `docs/WORKERS.md`, `docs/README.md` | No |

W4 and W5 can run in either order after W3; W6 and W7 after W4, which
builds faraday-keys' `job.rs`; W8 needs only W1.

## 10. For the owner

1. **Signing keys shared, not copied** (§6). Recommended: the store
   holds each master key's secret behind an `Arc`, and a signing job
   shares it, so no copy is made and a key forgotten mid-sign is wiped
   when the job ends. The other way, a copy for the job, is a second
   secret in memory for the job's length.
2. **Leaving a job to run on another screen.** Recommended: no for now.
   While a job runs, only Back, Cancel and Lock are taken (§4); letting
   a person open another tab while a vault opens means a job applied to
   a state that may have changed under it.
3. **The idle clock during a job.** Recommended: held while a job runs
   and started again from its end, since the job is the person's press
   and the person is waiting on it; a Strong vault on a Pi would
   otherwise warn mid-unlock.
4. **Pictures and PDFs of secret words on the worker** (§6). Recommended:
   yes, as a faraday-keys `MakeJob` holding a wiped copy of the revealed
   text for the job's length; until then those presses stay frame first
   on the main thread.
5. **Stopping a stretch early.** After Back or a lock the worker runs
   the stretch to its end and drops it, which holds the next job up to
   its length (seconds on a Pi). Argon2 and PBKDF2 could look at a stop
   flag between passes or blocks, which changes no output but touches
   the crypto crates' loops. Recommended: not now; record it as later
   work and decide once W4 is timed.
6. **The decision** (§5): one new decision, "costly work runs on one
   worker in the core", taking its number at the push.
