# Costly work off the drawing thread

**Kind:** Spec
**Status:** The worker, the shells' wake, the busy screen, the vault stretches, the aezeed read, the KeePass export, signing a PSBT, a PSBT's read and inspection, and a wallet's public files, the backup's checklist and its Make the files as jobs are built; files written as storage requests and the list of §2.1 confirmed on a Pi are specified, not built; the owner's answers are in §10, the line between a job and work drawn first is F184
**Open:** the rest of the jobs (§9): files written as storage requests (§7.3), held until a Pi shows it is needed; the list of §2.1 confirmed on a Pi 3B+

Touch and drawing run on each shell's own thread, the main thread. Work
that takes longer than a tenth of a second on a Raspberry Pi 3B+ runs on
a thread of its own, one worker in the core; shorter work stays where
it is, drawn first and done on the tick after (owner, 2026-10-31,
2026-11-02; F184, which amends `docs/SPEED.md` §9 point 2). This holds
for every Faraday: the stick and the Pi (`faraday/shells/stick`, one
shell for both) and Faraday Demo (`faraday/shells/desktop` over
`faraday/shells/window`). The worker is in the core, so every shell that
holds a `Faraday` gets it with no code of its own but a wake.

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
the stretch ends. Moving the long work off the main thread fixes both:
the inspection and every stretch that holds the screen for seconds are
jobs (§2.1), and what stays is drawn first and shorter than a tenth of a
second on a Pi.

**The Starter.** Its own work already runs on threads (the disks,
the concerns, BitLocker, every write; SPEED §11.2), and only its PDFs are
made on a press, which are short. The Demo it holds under its top app bar
(F126) is a `Faraday` and shares this path: it gets the worker through
the core, and the Starter's window gives it the same wake as Faraday
Demo's (§7.2).

## 2. Rules

1. **The main thread draws, takes input and does short work.** A shell's
   main thread hands the core its events, draws, presents, and times the
   panic chord. Work on §2.1's list of jobs runs on the worker; any other
   costly press is drawn first and done on the tick after, as SPEED §9
   point 2 says, and is short by the line of §2.1.
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

### 2.1 The line, and the list

**The line** (owner, 2026-11-02; F184). A press is a job when its work
takes longer than a tenth of a second, 100 ms, on a Raspberry Pi 3B+ at
its full clock (1400 MHz); shorter work is drawn first and done on the
tick after. A vault's key stretch is a job whatever it measures. Work
that grows with what it is given is judged at a large ordinary case: a
2-of-3 spend of 50 inputs, a multisig of fifteen keys.

**Until a Pi measures it.** No Pi has timed these presses yet, so a
release build on a desktop PC stands in, at a tenth of the line (10 ms):
SPEED's history found that "a Pi 3 is about ten times
slower" (`docs/history/SPEED-LOG.md` §9). The list below was
made that way.

**How an item moves.** The dev card's `work` rows (`costly::label`),
read on a Pi 3B+ at 1400 MHz, confirm an item or move it: a press drawn
first whose row shows more than 100 ms becomes a job, and a job whose
row shows well under it goes back to drawn first, except a vault's
stretch. A move is one commit that changes `slow.rs`'s list and this
table together and says in its message what was measured on what.

| Work | Presses | Side |
|---|---|---|
| Argon2id for a vault | A vault opened, an older one opened, made; each changed vault at a lock, one at a time (`Kind::VaultSeal`); the boot import's unlock; the Spend tab's unlock | Job |
| Argon2id for a KeePass file | The KDBX export written as a vault of one entry (`V::ExportKdbx`, then `V::PromptGo`) | Job |
| Argon2id for each key | Practice's samples loaded (`SamplesLoad`) | Job |
| scrypt | An aezeed read (Lightning) | Job |
| PBKDF2 for a Krux code | A Krux encrypted code sealed or opened (`Kind::KefSeal`, `Kind::KefOpen`; `Faraday::kef_seal`, `Faraday::kef_open`); its count runs from 10,000 to 100,000,000 rounds, so it is a job whatever the count, and it stops on Cancel (§4) | Job |
| The Krux code read card's timed run | The first Krux code read in a session sends a run of 50,000 PBKDF2 rounds (`Kind::KefTime`) that tells the card how long its count takes here; it has no busy screen, makes no job stale, is stopped by a seal or an open as it starts, and the card shows the count alone until its answer. The time is the run's own, on the worker, and is not kept if the run was stopped or took no time. Not measured on a Pi; about 100,000 compressions, so over the line on a Pi 3B+ if it ran drawn first | Job |
| Signing | A PSBT signed, on the Spend tab or from a review | Job |
| A PSBT's inspection | A PSBT parsed and inspected, from a file or the last part of a scanned code (`Work::PsbtParse`, `PsbtInspect`); the key references it is given are worked out on the main thread, and a scanned card's one-line fact (`read_psbt_fact`) still parses there, both left for the measurement on a Pi | Job |
| A set of files | A wallet's public files (`PublicOut` 2, 4 and 14: the multisig config checked, the split shares, Specter's numbered pictures), the backup's checklist (`BChecklist`) and its Make the files (`BFilesMake`): jobs; the backup stick's files made at Lock and write and the flow's writes (`Flow`), which write to a stick beside secret words | Job (the files made inside Lock and write stay with the write, §7.3) |
| PBKDF2 alone | A BIP-39 seed with or without a passphrase: Add a key, Load a key and its probe of the words, `SeedRead`, the read card's Add this key on a secret (`ReadGo`), `Potential`, New key, a chart's passphrase try; SLIP-39 shares split and recovered, Convert a seed's SLIP-39 set among them (a split and two read-backs: about the stand-in's line for 2 of 3 and for two groups, ten times it for the largest shape the card reaches, 13 of 16 groups of 2 of 16 each, which is no ordinary case) | Drawn first |
| A key's fingerprint for each key record of each target | A copy or move of an item (`V::CopyTo`, `V::MoveTo`, and `V::CopyKeep`, the tick that judges every row again), a seed saved into a vault with its Keep sheet (`V::SaveKey`, `V::KeepSave`, `V::SaveKeyKept`). A copy tells a key the target already holds by its fingerprint and master id, worked out once for each key record and kept by the record's id (a hash of its words and passphrase, so a record changed is worked out again). The words of a key record are read once and kept for the record, dropped for the record changed alone, and a document measure reads no other entry again. Measured (2026-11-05) in a release build on a desktop PC, the stand-in of this section, with two other vaults of 15 seeds each (each carrying a stored passphrase, a codex32 set and a BIP-85 origin, and without): with every fingerprint kept, `V::CopyTo`, `V::CopyKeep` and `V::MoveTo` under 1 ms, `V::SaveKey` and `V::KeepSave` 2 ms with the frame after under 1 ms, the same at 40 seeds. Every key record of an open vault is worked out as the vault unlocks and after every vault press, so a copy or move finds them kept and pays for none of them; the work at unlock (about 1.7 ms for each key record on the stand-in) is on the main thread, outside these presses | Drawn first |
| One picture or one PDF | A SeedQR picture, a seed's words or a paper form, one backup sheet | Drawn first |
| The self-test | Run self-test (`SelfTestRun`) | Drawn first |
| Not yet timed | codex32's new set, Vanity's Use, the Inbox's load, the Seeds tab's Make and Sample | Drawn first until timed with the stand-in |

Files written to a stick are not a press's work but a wait on the disk
process; whether they cross the line is the dev card's `file written`
row's answer (§7.3).

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
- **Latest wins** for a job asked again while one of its kind waits: a
  new one replaces the one not yet started and makes the running one's
  answer stale, as the scanner takes the newest frame. The probes that
  follow typing (`Sheet::Potential`, `Sheet::SeedRead`, Load a key's
  probe of the words) stretch PBKDF2 alone and stay drawn first, after
  SPEED §9 point 2's typed pause (§2.1); one that moves to the worker
  takes this rule.
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

Nothing that is drawn first. `Later`, `later_drawn`, `later_tick` and
`later_now` stay, for the presses under the line (§2.1): their work is
short, and the frame drawn first already shows the press. `slow`'s list
is split in two, the presses that start a job and those drawn first, as
§2.1's table says, and the debug guard in `press_in` stays as it
is: a press on neither list that stretches a key or makes more than one
picture on the main thread panics, naming itself. What the worker
replaces is the tick after for the jobs alone: a job's press is drawn at
once and its work no longer runs on the thread that draws.

## 4. The screen while a job runs

This screen is for the jobs of §2.1 alone. A press drawn first keeps
what it has today: the frame that shows it, with its toast for files or
a node key, and the work on the tick after.

`Faraday.busy: Option<Busy>`, public and read by drawing: the job's
`Kind`, its line of text, the time it began, and `done`/`of` where the
work has a count (Argon2's passes, a PSBT's inputs signed, a set of
pictures).

**What is drawn.** The screen that started the job stays, with its
working line and Material's linear progress indicator along its foot
(`working_bar`, DESIGN §4.14): **indeterminate** and moving, as Material
draws it, where the work has no count; **determinate**, its track filled
by the share done, where it has one; still under reduce motion, as
today. Its other actions are drawn in Material's disabled state: a button off, and a row, link, chip or card (whatever else is pressable) with its content at the disabled opacity over its own container (`Ui::fade_disabled`), taking no press. The
page checked against is Material 3's Progress indicators
(m3.material.io/components/progress-indicators), "Linear" and
"Indeterminate"; the disabled buttons against Material 3's Buttons,
"States", and the rows against Material 3's Lists, the disabled item. A sheet that waits today with a still bar (a vault's unlock,
creation and sealing, an unlock from the boot import and the Spend
tab) keeps its text and gets the moving bar. The shell is asked for
frames while `busy` is set, at the motion pace (`motion.rs`), and for
none once it clears.

**A job that stops.** A Krux code's seal or open (`docs/KEF.md` §7.2) is
the one job that really stops on Back, Cancel, a lock or a screen moved
away from, since its count can be 100,000,000 rounds and a stale job would
hold the one worker for minutes. It shares a flag and a count of rounds
done with the main thread (`faraday_keys::kef::Progress`): the work reads
the flag between steps of 5,000 rounds, an answer of `Cancelled` comes at
once and is dropped as a stale one is, and the worker takes the next job
without waiting. `Busy.of` is the count and `Busy.done` the rounds done,
brought up to date on each pass, so the bar is determinate.

**What is taken, what is refused, what cancels.**

| While a job runs | What happens |
|---|---|
| The same press again, or any other action of the screen | Refused: the buttons are disabled, and a press on one does nothing |
| Back, Cancel on the screen or sheet that started the job | Accepted: the screen goes back as it would, the answer is made stale and dropped when it comes; what the job was given by move (a typed passphrase) is gone with it, so the person types it again. For most jobs the work runs on to its end with its answer dropped; **a Krux code's job really stops** (below) |
| A tab, the rail or the menu | Refused, drawn disabled (§10, question 2) |
| Lock | Accepted at once: the session is dropped as a lock drops it, the worker ends with the app, and the answer, when it comes, has nowhere to go and is wiped on the worker |
| The idle warning and the idle lock | The idle clock does not run while a job runs and starts again from its end (§10, question 3); a job is the person's own press, and today the frame it held let no idle tick through either |
| The panic chord, Power off | Taken by the shell on its main thread at once, the job or not; the machine goes off with the worker mid-run (a key in its memory goes with the power, inside the Security scope) |
| Theme, text size, rotation, Faraday Demo's preview size or window size | Accepted: they change what is drawn, not what the job reads |
| Typing into a field | Accepted as today; no typed probe is a job (§2.1), and one that became one would take latest wins (§3.2) |
| A stick put in or pulled | Taken as today (`storage` events); a job that reads or writes a stick is one of §7.3's, answered by the storage, not by the worker. When the event moves the screen or the sheet away from where the job was started (to Visit, Import closed, the Lock sheet over Import), the job's answer is made stale and dropped where it arrives, as Back does, for every job; an event that moves neither leaves the job live |
| A scanned code | Taken by the scanner as today; a code that would start a job while one runs is held until the job ends, as a stick plugged in under the camera is held back |

Strings: the working lines exist (`working_files` and the vault's);
each new job's line is added to `faraday-strings/src/en.rs` with the job,
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

The decision is F184 (owner, 2026-11-02): it binds later changes (a new
costly press is put on one side of the line of §2.1), it amends a built
rule (SPEED §9 point 2: frame first, work on the tick after, now for
short work), and it lets key material cross a thread inside faraday-keys
(§6). Its rule is `speed.worker-line` in `tools/impact.toml`: a change to
`slow.rs`'s list says which side of the line the press is on and how
that was measured.

## 6. Key material in a job

F70 stands: key material lives in faraday-keys and leaves only through
its named reveals (`docs/KEYS.md` §2). A job's key material never leaves
faraday-keys' types; the thread it runs on changes, the crate holding it
does not. Only the jobs of §2.1 carry key material to the worker; a
press drawn first keeps its secrets on the main thread, as today.

**What moves to the worker.**

- **A stretch's secret input, by move.** A vault passphrase at Open,
  Make or the boot import's unlock, the passphrase a KDBX export is
  sealed under, an aezeed passphrase: transient input (KEYS §2,
  "Transient input"), handed in by move into a `faraday_keys::StretchJob`
  (`SecretText` inside), so no copy is made. The result comes back
  inside faraday-keys' types (`Stretched`: a slot key, a node key, a
  sealed file) and joins the store through `Keys`' existing `add_*` or
  the vault's open, on the main thread. Both wipe on drop. A BIP-39 or
  SLIP-39 passphrase is not on the list and does not move.
- **Signing, by shared reference to what the store holds** (§10,
  question 1). The store keeps each master key's secret behind an
  `Arc` that never changes once made; a `SignJob` holds a clone of the
  `Arc`, not a copy of the key, and the PSBT by value. A key forgotten
  or a lock while it signs drops the store's `Arc`; the job's keeps the
  key until it ends, and the key is wiped as the last one drops. A
  reblind (`Keys::reblind`) skips a key a job holds and marks it owing;
  `Keys::reblind_owed` randomizes it as the main thread finishes the
  job's answer, applied or dropped, once the store's `Arc` is alone again.
- **A picture or PDF of secret words** (a backup stick's words, a
  SeedQR picture; `Reveal::BackupStick`, `Reveal::PaperForm`): the
  reveal is a guard that borrows the store and cannot cross a thread.
  Each is one picture or one PDF, under the line (§2.1), so these
  presses stay on the main thread, frame first, as today; where a flow's
  press both seals a vault and makes words pictures, the stretch is the
  job and the pictures are made when its answer is applied. Should a Pi
  move one of them over the line, its maker runs inside faraday-keys as
  a `MakeJob` that takes the revealed text into a `Zeroizing` it owns, a
  copy that lives as long as the job, as the owner answered (§10,
  question 4).

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

Held for a Pi's answer: a write is a wait on the disk process, and only
the dev card's `file written` row on a Pi 3B+ says whether it crosses
the line of §2.1. Until it does, files are written as today. If it does,
a write is a storage request answered by a storage event, as a picture
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
- **A short press is drawn first.** A press on the drawn-first side of
  §2.1 shows its frame before its work, and its work is done on the tick
  after, with no busy screen (`tests/all/slow_press.rs`, as today).
- **A scan goes on.** The Scan sheet with an animated PSBT: frames keep
  being drawn while its last part's PSBT is read and checked, and the
  review opens after `settle`.
- **The panic chord during work** (`faraday/shells/stick`, its tests of
  `PanicTimer` and the loop): with a job held running, the chord held
  for `PANIC_HOLD` asks for power off at once.
- **A write that is slow**, if files written as storage requests are built (`faraday-storage`'s tests, a
  disk process that answers late): frames go on, and the file's toast
  comes with the answer; one that never answers ends as "not kept".
- The budget tests (`work/budget/`), `slow_press.rs` and the snapshot
  tours pass unchanged, since scripted presses run inline and the counts
  come home.

## 9. Order of building

The worker, with the split of `slow`'s list into presses that start a job
and presses drawn first (§2.1), comes first; everything else uses it. The
shells' wake and the busy screen (§3, §4) follow. The vault stretches come
before the other jobs that hold key material, because they build
faraday-keys' `job.rs`, which the aezeed read, the KeePass export, signing
and a PSBT's read use; a set of public files needs only the worker and the
busy screen. Files written as storage requests (§7.3) needs only the
worker, and is built only if a Pi's `file written` row crosses the line.
Confirming the list of §2.1 on a Pi 3B+ needs a Pi dev card. A job that
touches keys, cryptography, signing or what the device takes in gets the
reviewer before it lands.

What each piece is and changes:

- **The worker** (built): `Worker`, `Job`, `Done`, generations, latest
  wins, inline mode, `work::absorb`, `busy`, the refusals of §4, the idle
  clock held, `testkit::settle`; the mint's row in COMPUTE §3 and `MINTS`
  (checked by the costs lint); `slow`'s list split as §2.1's table says,
  the presses not yet timed timed with the stand-in and put on their side.
  The first job is Practice's samples loaded (`SamplesLoad`, public data).
  `faraday-core/src/worker.rs`, `lib.rs` (`press_in`, the drain), `slow.rs`,
  `idle.rs`, `work.rs`, `testkit.rs`, `tools/costs-lint/src/main.rs`,
  `docs/COMPUTE.md`.
- **The shells' wake and frames while busy** (built): `Wake::Work`, the
  winit proxy for both windows, the snapshot tool inline; the panic chord
  test. `faraday/shells/stick/src/main.rs`, `window/src/lib.rs`,
  `desktop/src/main.rs`, `starter/src/main.rs`, `snapshot/src/main.rs`.
- **The busy screen** (built): the moving bar, determinate where counted,
  disabled actions, Back and Cancel stale-making; strings; DESIGN §4.14; a
  tour of a busy screen (`just faraday-snap 480x640 vault`), reviewed
  against Material's Progress indicators. `faraday-core/src/screens/parts.rs`,
  the sheets that use `working_bar`, `faraday-strings/src/en.rs`,
  `faraday/shells/snapshot/src/tours`.
- **Vault stretches** (built): Open (an older vault's, the boot import's
  and Spend's unlock included) and Make as `StretchJob` and `Stretched` in
  faraday-keys, the passphrase by move. Seal again runs no stretch; the
  samples' keys stay drawn first (PBKDF2). faraday-keys' `job.rs`,
  `faraday-vault`'s Argon2 call site, `faraday-core/src/vault/`,
  `boot_import/`, `docs/KEYS.md`.
- **The other stretches** (built): an aezeed read, the KDBX export written
  and verified. A BIP-39 seed, SLIP-39 and a chart's passphrase try stay
  drawn first (§2.1). `faraday-core/src/lightning/`, the KDBX writer's
  call site (`faraday/faraday-keys/src/vault.rs`).
- **Signing and checking** (built): `SignJob` with the store's keys behind
  `Arc` (§10 question 1; `Keys::sign_job`, `faraday-core/src/signing.rs`);
  a PSBT's parse and inspection from a file or the last scanned part as a
  job (`faraday-core/src/signing/read.rs`): the Inbox file's bytes and the
  public key references, policies and network go by move, and the answer
  brings back the PSBT, its version 2 form and the inspection. The key
  references are still worked out on the main thread, as the signing's
  are. The answer is dropped where the person is no longer on the screen
  and sheet they pressed on, and the inspection is made again on the main
  thread only where the session's inputs changed meanwhile. The Spend
  tab's signatures counted is read from the inspection the job made, with
  no second inspection. The scanned card's one-line fact (`scan_key.rs`,
  `read_psbt_fact`) still parses on the main thread when the last part
  arrives, left for the measurement on a Pi. `faraday/faraday-keys/src/session.rs`,
  faraday-keys' `job.rs`, `faraday-core/src/scan_key.rs`, the Spend and
  sign areas.
- **Sets of files of public values** (built in part): a wallet's files,
  the checklist's, the backup stick's and the flow's public files. A single
  picture or PDF, and those of secret words, stay drawn first (§2.1, §6).
  Built: `public_files.rs` makes a wallet's files from owned public inputs
  (name, policy, network, the sheet's fingerprints, the slider's setting)
  in one `FilesWork` job. A press on the multisig config, the split shares
  or Specter DIY's pictures (`PublicOut` 2, 4, 14) and the checklist
  (`BChecklist`, whose account key files are worked out on the main thread
  from the keys held and go to the job made) start it; the files and the
  checklist open with the answer, dropped by Back, a lock, a moved screen
  or sheet, or a backup no longer on the wallet; the shares made again
  once a slider is let go start the same job. The checklist's Make the
  files (`BFilesMake`) starts the same job and brings the backup's screen
  up to date with the answer, dropped the same way. A descriptor picture
  (8), the backup sheet (3) and the text kinds stay on the thread that
  asked; the files made inside Lock and write (`backup_flow/`) and the
  vault's save are made with the write of a stick, beside secret words,
  and stay there (§6, §7.3). `faraday-core/src/public_files.rs`,
  `wallet_backup.rs`, `picture.rs`, `pdf.rs`, `backup_flow/`.
- **Files written as storage requests** (§7.3, held, not built):
  `Sticks::write_later`, `StorageEvent::Written`, the stick's disk process
  answer, Faraday Demo's `Dirs` thread, the boot report's write.
  `faraday-storage/src/lib.rs`, `faraday/shells/stick/src/main.rs`,
  `files.rs`, `faraday-core/src/lib.rs` (the storage events),
  `faraday/shells/desktop/src/lib.rs`. A backup's files are written as
  before; it has no parser and no key.
- **The list confirmed on a Pi** (not built): the list of §2.1 on a
  Pi 3B+ at 1400 MHz from the dev card's `work` rows, each item kept or
  moved as §2.1 says; one late-flow session timed as SPEED §7 says. `Later`
  stays. `slow.rs`, `docs/SPEED.md`.


## 10. For the owner

Decided by the owner, 2026-11-02, as below.

1. **Signing keys shared, not copied** (§6). The store
   holds each master key's secret behind an `Arc`, and a signing job
   shares it, so no copy is made and a key forgotten mid-sign is wiped
   when the job ends. The other way, a copy for the job, is a second
   secret in memory for the job's length.
2. **Leaving a job to run on another screen.** No for now.
   While a job runs, only Back, Cancel and Lock are taken (§4); letting
   a person open another tab while a vault opens means a job applied to
   a state that may have changed under it.
3. **The idle clock during a job.** Held while a job runs
   and started again from its end, since the job is the person's press
   and the person is waiting on it; a Strong vault on a Pi would
   otherwise warn mid-unlock.
4. **Pictures and PDFs of secret words on the worker** (§6). Yes, as a faraday-keys `MakeJob` holding a wiped copy of the revealed
   text for the job's length; until then those presses stay frame first
   on the main thread.
5. **Stopping a stretch early.** After Back or a lock the worker runs
   the stretch to its end and drops it, which holds the next job up to
   its length (seconds on a Pi). Argon2 and PBKDF2 could look at a stop
   flag between passes or blocks, which changes no output but touches
   the crypto crates' loops. Not now; record it as later
   work and decide once the vault stretches are timed.
6. **The decision** (§5): one new decision, F184.

**The middle way** (owner, 2026-11-02, F184). Not every costly press is
a job: draw first and work on the tick after stays the rule for short
work, and work longer than a tenth of a second on a Pi 3B+ runs on the
worker (§2.1). The answers above stand for the jobs; answer 4 applies
to none of today's presses, since a picture or PDF of secret words is
drawn first, and holds for one a Pi moves over the line.
