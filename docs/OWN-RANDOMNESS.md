# Your own randomness, and the kernel's at boot

**Kind:** Spec
**Status:** Specified, partly built (F86, F157): §2.3's mixing into the seed and the pool, and the lock (R1, `faraday/faraday-core/src/fresh.rs`); New key's This device mixing and its label (§2.3, §2.4; `faraday/faraday-core/src/keygen/mod.rs`, `device_own`); the Add randomness page with camera, dice, coins and cards, Settings' Randomness row, the status line and New key's link (R2; `faraday/faraday-core/src/fresh/page.rs`, `faraday/faraday-core/src/screens/randomness.rs`); pointer and keys on the desktop app (R3; `core/faraday-entropy/src/pointer_keys.rs`); option A (§3.3, R4); the kernel check of §3 done from the repository
**Open:** §8's Add randomness at every press that draws the system's randomness (R7, F157, planned); §3.5's late seed and the Pi's generator (R5, R6, F102); the boot measurement on one real PC, in passing (§3.4 item 2; QEMU's worst case is measured).

Two things. (a) A person may add randomness of their own, camera noise,
dice rolls, coin flips or cards on every shell and pointer moves and
keys on the desktop, which is hashed together with the system's bytes into the
session's seed and pool. It is mixed, never substituted: what comes out
is at least as strong as the stronger of the two. (b) What the stick's
and the Pi's kernels do for randomness at boot, read from the image's
configuration, and a plan for the boot measurement (QEMU, then one real PC in passing).

## 1. What is built

- **The channel.** The core has no generator. It sends
  `Command::RequestEntropy`; the shell answers with one `Event::Entropy`
  of 32 bytes, preceded by `Event::EntropySource` naming where they came
  from (`core/faraday-shell-api/src/lib.rs`). The stick and the Pi read
  `/dev/random` (`faraday/shells/stick/src/main.rs`, `entropy`); the
  desktop app and the Starter read the `getrandom` crate
  (`faraday/shells/window/src/lib.rs`: getrandom(2), ProcessPrng,
  getentropy(2), crypto.getRandomValues). A shell that cannot answer
  sends nothing.
- **The session** (`faraday/faraday-core/src/fresh.rs`). The first
  answer is the seed (`Seed`); the next ones wait in a pool: the first
  display asks for 26 (`POOL`, as many as the most one press takes),
  and up to 52 are kept. `fresh(what)` takes one pooled answer and
  returns SHA-256("faraday fresh" ‖ what ‖ bytes): vault creation, GPG
  and Secure Boot keys, codex32's identifier. `fresh_bytes(what, n)`
  takes an answer for every 32 of `n` bytes, each made as `fresh` makes
  it, the latest answer first and the last cut to what is left, or
  takes none while fewer wait: SLIP-39's split (its identifier and
  `split`'s bytes, `slip39::random_len`), Seed XOR's random parts,
  codex32's random shares (a set of nine from a 64-byte seed, the most,
  takes 26 answers), a KeePass file's seed, IV, salt, inner key and
  UUIDs (9 answers). No answer is stretched into more bytes than it
  holds (owner, 2026-10-28). `Seed::draw` is SHA-256
  (what ‖ seed ‖ count): a vault's seal nonces, a new keyboard's code.
  `sign_seed` draws a MuSig2 round's nonces from the seed with a
  count. Nothing is made before the first answer: `seeded()` is false,
  `fresh` and `sign_seed` return `None`, and after two seconds Home says
  the system gave no randomness (changed by F102, §3.5: nothing is said
  at start; a press that needs the seed waits for it).
- **New key** (`faraday/faraday-core/src/keygen/mod.rs`, `core/faraday-entropy`)
  already offers dice, coins, cards, hex, camera noise, this device
  (`DeviceRandom`, one answer of its own, not the pool) and a mix of two
  to five of them (`Mixed`: SHA-256 over the sources' commitments). A
  key from dice alone takes nothing from the system, by design: it can be
  reproduced elsewhere.

What a person adds reaches the seed and the pool (R1) from Add
randomness (R2): Settings › Randomness and New key's This device open
it, and §8 adds every other press that draws the system's (F157, not
built); camera, dice, coins and cards are entered on New key's own pads.
A camera frame is added as it is taken, its commitment that of a
one-frame `CameraNoise` (SHA-256 over the frame's SHA-256), as R1's
tests take it. New key's This device bytes taken before an addition,
not yet made into words, are asked for again, so its label says what
they hold. Pointer and keys (R3) is offered where the shell says it
has a pointer and keys (`Faraday::set_pointer_keys_offered`: the
desktop app; the Starter's held app shows only Transfer and Learn, so
no Add randomness), its events recorded and
committed by `faraday-entropy`'s `PointerKeys`.

## 2. Adding your own randomness

### 2.1 Sources

| Source | Shells | What is taken | Its commitment |
|---|---|---|---|
| Dice | every shell | rolls on the dice pad, as New key takes them (`DiceRolls`) | SHA-256 of the rolls as ASCII digits, as `faraday-entropy`'s dice commitment |
| Camera noise | every shell with a camera | frames, as New key takes them (`CameraNoise`, the same refusal of a frame with under 32 distinct values) | each frame's SHA-256, as it is taken |
| Coins | every shell | flips, as New key takes them (`CoinFlips`) | `CoinFlips::commitment`, as New key's |
| Cards | every shell | draws from a shuffled deck, as New key takes them (`CardDraws`, refusing a card already drawn from its deck) | `CardDraws::commitment`: SHA-256 of the draws' indices, as New key's |
| Pointer and keys | the desktop app | while it is the source chosen (§5 item 4): each move of the pointer, with no button down, over its area, its x and y in the frame's pixels; each key typed for it, a character, Backspace or Delete, by its code (a character's Unicode scalar value, Backspace 0x110000, Delete 0x110001); each with the core's time in ms and a running count from 0 | a 17-byte big-endian record per event, `P` ‖ count (u32) ‖ ms (u64) ‖ x (u16) ‖ y (u16) or `K` ‖ count ‖ ms ‖ code (u32); SHA-256("faraday pointer keys" ‖ the records), committed every 64 events, when the pointer leaves the area or the window, when a key is not the source's, when the source or the page is left, and when New key's Make the words is pressed, which then makes no words: the key waits for the generator's bytes again, so they hold the batch |

Nothing is credited: no source claims a number of bits, and the status
line counts what was taken, not how strong it is. A dice batch of 99
rolls or more is named as enough for 256 bits on its own, as New key
names it.

Faraday never collects randomness passively (owner, 2026-10-26).
Pointer and keys counts only as a source the person picks, on Add
randomness or on New key's Generator card, and works on deliberately:
moving the pointer over its area, typing for it. Nothing is gathered in
the background: not input made while the page is open on another source
(keys typed while Dice is the source are dice values, never timings),
not the pointer elsewhere on the page or on any other screen, and never
a passphrase's or any field's keys. The keys that move about the page
(Tab, the arrows, Enter, Escape, and Space on a control the keyboard is
on) are the page's, not the source's.

### 2.2 Where it is offered

- **Settings › Randomness**: a row with the status line (§2.4); it opens
  **Add randomness**, a page with Camera, Dice, Coins, Cards and, on the
  desktop app, Pointer and keys. Camera is
  listed first where the shell has a camera; nothing is chosen and
  nothing started, no camera and no pad, until a source is pressed
  (owner, 2026-10-26). Each source adds when its batch is done (dice,
  coins and cards: the **Add** button under the pad; camera: each frame;
  pointer and keys: as §2.1 says). Pointer and keys' pad is an area to
  move the pointer over and type, with a count of what it took ("412
  pointer moves, 30 keys"). The page stays until Back.
- **New key, This device**: its row's label carries the status (§2.4),
  and the source's step, the Generator card, offers Camera (where the
  shell has one), Dice, Coins, Cards and, where the page offers it,
  Pointer and keys on the card itself, with no page
  to open (owner, 2026-10-25). None is chosen until the person presses
  one: no pad is live, no key is taken and no camera runs before that
  (owner, 2026-10-26: Faraday never collects randomness passively; keys
  count only for a source chosen that takes them). Once chosen, its pad
  and its batch's **Add** are on the card. While nothing is added,
  **Skip** moves on without it; once anything is added the card says
  **Randomness added to the pool**, then **Make the words** as before.
- **Every other press that draws the system's randomness** (F157,
  §8): a vault's keys, a GPG key, Secure Boot's keys and signatures,
  SLIP-39's, Seed XOR's and codex32's random parts, a KeePass file, a
  MuSig2 round's nonces, and the rest of §8.1's list. The step or pane
  that holds the press carries the Randomness row (§8.2): the status
  line and **Add randomness**, which opens this page and comes back to
  the press.
- Not on Home: Home is a working screen. Its missing-seed line goes
  (F102, §3.5).
- Not at boot: the stick's and the Pi's shells do not offer it before
  the passphrase; it is reached from Settings, New key and the presses
  of §8.1.

### 2.3 Mixing

A new `Own` in `fresh.rs` (on the `Fresh` area struct,
`docs/STRUCTURE-PASS.md` §7.4, not a new `Faraday` field): `acc`, 32
bytes in a `Zeroizing`, and the counts for the status line.

- **Adding** a commitment `c`:
  `acc` ← SHA-256("faraday own" ‖ `acc` ‖ `c`), with `acc` all zero
  before the first.
- **The seed.** When `acc` changes and the seed exists:
  seed ← SHA-256("faraday reseed" ‖ seed ‖ `acc`). When the system's
  first answer arrives and `acc` exists: seed ← SHA-256("faraday
  reseed" ‖ bytes ‖ `acc`). Everything drawn from the seed (seal nonces,
  a keyboard's code, `sign_seed`) follows; the counts are unchanged, so
  no draw repeats.
- **The pool.** `fresh(what)` with `acc` present returns SHA-256
  ("faraday fresh" ‖ what ‖ bytes ‖ `acc`); without, exactly what it
  returns today.
- **New key's This device**: `DeviceRandom` takes SHA-256("faraday
  device own" ‖ bytes ‖ `acc`) when `acc` exists (§5 item 2).
- **Never substituted.** `acc` alone makes nothing: no seed, no pool
  entry, no key. Before the system's first answer `acc` waits.
- **Strength.** Every output is SHA-256 of an input holding both the
  system's bytes and `acc`. Someone who lacks either one cannot
  predict it; so it is at least as strong as the stronger of the two,
  on the usual assumption about SHA-256. It does not help against a
  system that is itself compromised (one that reads `acc` from memory),
  which `docs/SECURITY-MODEL.md` already leaves out.
- **Lock.** `acc` is in the session: a lock wipes it with the seed, and
  the next session starts with none. It is never kept, never written to
  a stick, never shown.
- **Cost.** Hashing a commitment is cheap. A camera frame's SHA-256
  costs as New key's does; it takes the compute token as that one does
  (`docs/COMPUTE.md`).

### 2.4 The status line

Settings' Randomness row, and New key's This device label:

- "System: {source}" with the shell's `EntropySource`, or "System:
  operating system" when it names none;
- then "· your own: {counts}", for example "· your own: 6 camera
  frames, 99 dice rolls, 442 pointer moves and keys", or "· your own:
  none"; coin flips and cards are counted as dice rolls are, and
  pointer moves and keys together, as they are added.

New key's label reads "operating system · {source} · with your own"
once `acc` exists. The strings go in `faraday/faraday-strings/src/en.rs`.

## 3. The kernel's randomness at boot

### 3.1 The stick (x86-64)

Read from `faraday/image/boards/x86_64-uefi/buildroot.fragment`, its
`linux.fragment`, `kernel.forbidden` and `cmdline.txt`, and the common
parts. Linux 6.18.56, x86_64 defconfig without its `=m` lines, modules
off.

- **RDRAND and RDSEED are trusted.** `linux.fragment:267-275` says the
  CPU's RDRAND and the firmware's seed are mixed in but not credited,
  and sets `# CONFIG_RANDOM_TRUST_CPU is not set` and `#
  CONFIG_RANDOM_TRUST_BOOTLOADER is not set`. Linux 6.2 removed both
  options. Since then `drivers/char/random.c` trusts both by default
  (`trust_cpu` and `trust_bootloader` start true) and only the boot
  parameters `random.trust_cpu=off` and `random.trust_bootloader=off`
  turn them off. Neither is on the stick's command line
  (`faraday/image/common/cmdline.txt`,
  `faraday/image/boards/x86_64-uefi/cmdline.txt`), and
  `CONFIG_CMDLINE_OVERRIDE` means nothing else can add them. On 6.18 the
  two lines in the fragment name symbols that no longer exist. Kconfig
  drops them without a word. `kernel.forbidden:8-9` forbids the same
  two symbols, and `check-kernel-config.sh` passes because they are
  absent. On a CPU with RDSEED or RDRAND, `random_init` credits their
  output and the CRNG is ready before userspace starts. The fragment's
  comment, and the forbidden list's reasons, are no longer true.
- **The firmware's seed is trusted too.** The EFI stub asks the
  firmware's EFI_RNG_PROTOCOL for a seed and passes it to the kernel,
  which adds it as bootloader randomness. With `trust_bootloader` on,
  that is credited. It applies only when the firmware offers the
  protocol.
- **Jitter.** When a read of `/dev/random` would block, the kernel's
  own jitter loop (`try_to_generate_entropy`, built in since 5.4, timed
  by RDTSC on x86) generates entropy from timer jitter. That needs no
  option. `CONFIG_CRYPTO_JITTERENTROPY` is something else: it seeds the
  crypto API's DRBG, not `/dev/random`.
- **No network and no disk needed.** None of the three needs a network
  or a disk. Interrupts from the keyboard, the touchpad and USB are
  mixed in and credited as they come.
- **Hardware generators.** Whether the generated configuration has
  `HW_RANDOM` and a TPM's or a chipset's generator built in is not in
  the repository: the defconfig is in the kernel tarball. §3.4's first
  step reads it.

### 3.2 The Pi 3 and Zero 2 W

Read from `faraday/image/boards/pi3/buildroot.fragment`, its
`linux.fragment`, `cmdline.txt` and the Zero 2 W's, which match.

- **The kernel is 6.1, not 6.6.** It is the Raspberry Pi 6.1 tree at the
  commit Buildroot 2024.02 pins (`17f135b7…`), `bcm2709` defconfig, 32-bit.
- **bcm2835-rng is built in and credited.** `linux.fragment:81-84` sets
  `CONFIG_HW_RANDOM=y` and `CONFIG_HW_RANDOM_BCM2835=y`. The hwrng core
  feeds a built-in generator into the pool from the moment it probes and
  credits it at `rng_core.default_quality`, full (1024 per 1024 bits) in
  this kernel for a driver that states no quality of its own, as
  bcm2835-rng does not. The command line sets no `rng_core.` parameter.
  So the CRNG is ready shortly after the driver probes, early in boot.
- **No CPU generator.** 32-bit ARM has no RDRAND equivalent; 6.1's
  `CONFIG_RANDOM_TRUST_CPU` has nothing to trust there. Whether
  `bcm2709_defconfig` sets `CONFIG_RANDOM_TRUST_BOOTLOADER`, and whether
  the Pi firmware writes `/chosen/rng-seed`, is not in the repository;
  §3.4 reads both.
- **Jitter** is there as a fallback, timed by the ARM generic timer.
- **A comment to correct.** `linux.fragment:81-82` says the session key
  is drawn from `/dev/urandom`; the shell reads `/dev/random`.

### 3.3 The choice for the stick

Decided (owner, 2026-10-23): A. The measurement of §3.4 still runs, to
confirm that A brings no long wait at boot; a first answer later than
two seconds after the kernel starts on any machine tried is raised with
the owner.

The comment in the fragment states a rule (do not credit the CPU or the
firmware), and the kernel does not follow it. The two ways that were
weighed to make the image and its comments agree:

- **A. Make the rule true.** Add `random.trust_cpu=off
  random.trust_bootloader=off` to
  `faraday/image/boards/x86_64-uefi/cmdline.txt`; replace the two
  `kernel.forbidden` lines with a check in `check-kernel-config.sh` that
  `CONFIG_CMDLINE` carries both words; leave the fragment's comment and
  drop its two dead lines. The CRNG then waits for jitter and interrupts.
  The stick shell's first `/dev/random` read blocks until then, on the
  event loop, before the first frame. §3.4 measures how long that is.
- **B. Accept the trust.** RDRAND and the firmware's seed are credited.
  The fragment's comment and the two forbidden lines are replaced with
  one that says so. With §2, a person who does not trust the CPU can add
  their own randomness.

A is built (R4, 2026-10-23): `boards/x86_64-uefi/cmdline.txt` carries
both parameters; `kernel.forbidden` no longer lists the two dead
symbols; a new `boards/x86_64-uefi/kernel.cmdline` lists the two words
and `check-kernel-config.sh` fails a config whose `CONFIG_CMDLINE`
lacks either; the fragment's comment says what the kernel does; the
stick shell's comment is corrected, and in the dev image (`--verbose`)
the shell says the CLOCK_BOOTTIME at which it opened `/dev/random` and
at which the read returned. The Pi fragments' comment now names
`/dev/random`. Two seconds is the bound because the core's
`SEED_GRACE_MS` is two seconds after the first display. The stick
shell's comment at
`faraday/shells/stick/src/main.rs:602-613` changes: it says the board
seeds the pool from bcm2835-rng, which is the Pi's and not the stick's.

### 3.4 The measurement

How long `/dev/random` takes to be ready after power-on with option A,
and when the app's first answer arrives. Since Linux 5.18 the kernel
generates jitter entropy itself when a reader waits, which on an x86
PC takes about a second, and the Pi credits its hardware generator
either way, so a long wait is not expected; the measurement confirms
it (owner, 2026-10-23: no session on several PCs).

1. **QEMU, the worst case** (the build machine, automated): the stick
   image with A's two parameters, booted through
   `faraday/tools/stick-qemu.py` on QEMU's `qemu64` CPU, which has no
   RDRAND. Recorded: the time of the kernel's "random: crng init done"
   line and of the app's first answer, the worst of ten boots.
2. **One real PC, in passing** (the owner): the next time an option-A
   dev stick starts on a PC, the dev image's log shows the same two
   times; the owner reads them off. The dev variant of the stick shell
   writes the CLOCK_BOOTTIME at which it opened `/dev/random` and at
   which the read returned; the kernel's line is in `dmesg` in every
   variant.
3. **The Pi** needs no measurement for A. Its generator is credited
   from `bcm2835-rng` (§3.2) whatever the two parameters say.

The results go in this section; the dev instrumentation stays in the
dev variant only.

Results (2026-10-23, item 1): the dev stick image at 0e781882, ten
boots on QEMU's `qemu64` CPU (no RDRAND) under TCG, since the build
machine offers no KVM; option A; times from power-on (CLOCK_BOOTTIME).

| | Best | Worst |
|---|---|---|
| The kernel's "random: crng init done" | 2.22 s | 3.04 s |
| The app's first read of `/dev/random` returned | 2.17 s | 3.00 s |
| Time the app's read waited | 0 ms | 260 ms |

Under TCG every instruction is emulated, so a real PC is faster; the
pool is ready within about a quarter of a second of the app asking for
it, and before anything a person does needs randomness. Item 2, one
real PC, is still to read.

### 3.5 A late seed, and the Pi's generator (F102)

- **Nothing at start.** Home's missing-seed line and `SEED_GRACE_MS`'s
  two seconds go. Nothing a person does in the first seconds needs
  randomness.
- **A press that needs it waits.** Making a key, a vault, a share set,
  a signing round's nonces: while the seed has not arrived, the button
  reads "Waiting for randomness" and the action goes on by itself when
  the seed arrives.
- **No randomness at all.** A seed still missing 60 seconds after the
  first display is a system that gives none: a press that needs it then
  says so and does nothing.
- **The Pi mixes bcm2835-rng and does not credit it.** `rng_core.`'s
  quality is set to 0 on the command line. Where the Pi's 6.1 kernel
  then stops feeding the generator into the pool (its hwrng thread
  starts only for a quality above 0), the Pi's shell reads
  `/dev/hwrng` and writes the bytes to `/dev/random`, which mixes them
  without crediting them, before its own read of `/dev/random`. The
  pool is then ready on the kernel's jitter alone, timed by the ARM
  generic timer.
- **The Pi's wait** is read off a dev Pi's log in passing (the owner),
  as the PC's is in §3.4.

## 4. Tests

In `faraday/faraday-core/tests/all/` (a new `own_randomness.rs`):

1. Mixing changes the derived bytes: with the same system answers, a
   vault created after dice are added has another seal key and another
   nonce than one created without (`fresh` and `Seed::draw` both).
2. Mixing never removes the system's: with the same dice added, two
   sessions given different system answers derive different bytes.
3. No key material before the system's first answer: dice and camera
   added before any `Event::Entropy` leave the session unseeded; a vault
   does not seal, New key's This device makes no words, a signing round
   is not opened, a keyboard gets no code; after the answer all work.
4. With nothing added, `fresh` and the seed's draws are the bytes they
   are today, for fixed system answers.
5. The status line: Settings' Randomness row reads the system's source
   and "none"; after 99 dice rolls are added, it counts them; after a
   lock, "none" again.
6. Pointer and key events count only while Pointer and keys is the
   chosen source and the events are made on it: the same events on
   Home, on another source of the page, off its area, or typed into a
   field leave the count unchanged; a shell that does not say it has a
   pointer and keys is not offered the source. A batch of pointer moves
   and keys on the source changes the derived bytes as its records say,
   and the system's bytes still change the result; 64 events are added
   as a batch and the rest as the page goes.
7. More than 32 bytes are an answer for every 32: `fresh_bytes` gives
   the known answers of three pooled answers, latest first, and takes
   none while only two wait; a SLIP-39 split's identifier and shares
   are those of `split` given exactly those bytes, and a KeePass file's
   master seed and IV are the known answers of its answers.

The construction (§2.3) is SHA-256 over concatenations; per CLAUDE.md a
cryptographic change is checked against another implementation, so its
known answers are added to the Python cross-check (F79) and test 4's
and a mixed case's bytes are compared with it, as are pointer and
keys' records and commitment (§2.1).

For §3: `check-kernel-config.sh` fails a board whose `CONFIG_CMDLINE`
lacks the two parameters (A).

## 5. Questions for the owner

Answered by the owner on 2026-10-23, item 4 on 2026-10-26:

1. **The stick.** Decided: A, do not credit RDRAND or the firmware's
   seed (§3.3). The boot measurement (§3.4) still runs, to confirm no
   long wait.
2. **New key's This device.** Decided: yes, it mixes in what the
   person added, and its label says so (§2.3, §2.4).
3. **Where Add randomness is offered.** Decided (owner, 2026-10-28,
   F157; it replaces "Settings and New key only" of 2026-10-23):
   wherever the system's randomness is used, the person can add their
   own. Add randomness is offered at every press that draws it (§8),
   and in Settings.
4. **Pointer and keys.** Decided (owner, 2026-10-26): Faraday never
   collects randomness passively. Pointer and keys count only as a
   source the person picks on the page (Settings' Add randomness, New
   key's Generator card) and works on deliberately (moving the pointer
   over the source's area, pressing keys for it). Nothing is gathered in
   the background, including input made while the page is open on
   another source, and never a passphrase's or any field's key timings
   (§2.1).
5. **At boot.** Decided: not at boot for now; only from Settings, New
   key and the presses of §8 (§2.2).
6. **Coins and cards.** Decided: yes, added as sources (§2.1).

Also decided (owner, 2026-10-23): camera noise is the default source,
listed first, and the others stay available (§2.2). Since 2026-10-26
(owner) the page opens with nothing chosen, the camera off until Camera
is pressed.

## 6. Batches

| Batch | What | Size |
|---|---|---|
| R1 | `Own` in `fresh.rs`: adding, the seed's and the pool's mixing, the lock; tests 1–4; the cross-check | small |
| R2 | The Add randomness page: camera (the default), dice, coins and cards on every shell, through `faraday-entropy`'s accumulators; Settings' row and the status line; New key's label and link; tests 5 | medium |
| R3 | Pointer and keys on the desktop app, as §5 item 4 decides: `faraday-entropy`'s `PointerKeys`, the source on the page and the card, its known answers in the cross-check; test 6. Built | small |
| R4 | The kernel: A (§3.3), the comments at `faraday/image/boards/x86_64-uefi/linux.fragment:267-275`, `kernel.forbidden:8-9`, the Pi fragments' lines 81-82, the stick shell's comment; the dev instrumentation; then the measurement on hardware | small, then hardware |

| R5 | §3.5: the late seed (Home's line gone, a press waiting with "Waiting for randomness", the minute's bound) in `fresh.rs` and the screens that need the seed; tests that a press made before the seed goes on when it arrives and that a minute without one says so | small |
| R6 | §3.5: the Pi's generator mixed, not credited (the command line, and the shell's write where the kernel needs it), checked against the Pi kernel's source | small |
| R7 | §8: the Randomness row at every press of §8.1, its place in the Pi's ⋮, the way back to the press; §8.5's tests | medium |

R4 does not wait for the measurement; the measurement confirms it, and
the comments say what the kernel does.

Edit list:

- `faraday/faraday-core/src/fresh.rs`: `Own`, the mixing, the status.
- `faraday/faraday-core/src/keygen/mod.rs`: This device's mix, its
  label.
- A new screen for Add randomness in `faraday/faraday-core/src/screens/`,
  its actions, and Settings' row in
  `faraday/faraday-core/src/screens/settings.rs`.
- `faraday/faraday-strings/src/en.rs`: the page's and the line's
  strings.
- `faraday/faraday-core/tests/all/`: `own_randomness.rs`.
- The Python cross-check (F79): the mix's known answers.
- `faraday/image/boards/x86_64-uefi/linux.fragment`,
  `faraday/image/boards/x86_64-uefi/kernel.forbidden`,
  `faraday/image/boards/x86_64-uefi/cmdline.txt`,
  `faraday/image/boards/x86_64-uefi/kernel.cmdline` (new),
  `faraday/image/check-kernel-config.sh`,
  `faraday/image/boards/pi3/linux.fragment`,
  `faraday/shells/stick/src/main.rs`.

## 7. Docs that follow

- `docs/SECURITY-MODEL.md`, a paragraph at the end of "Where keys live
  in the program": "Randomness. The core has no generator: the shell
  answers with 32 bytes from `/dev/random` on the stick and the Pi and
  the operating system's call on the desktop. A person may add dice
  rolls, coin flips, cards, camera noise and, on the desktop, pointer
  moves and keys made on the source for them (Settings › Randomness); these are hashed with the system's bytes
  into the session's seed and every fresh draw, never used alone, so
  what comes out is at least as strong as the stronger of the two. On
  the stick the kernel does not credit the CPU's RDRAND or
  the firmware's seed (`docs/OWN-RANDOMNESS.md` §3); on the Pi it
  credits the SoC's generator, bcm2835-rng."
- `core/faraday-shell-api/src/lib.rs`, the entropy channel's module
  doc: replace "The core asks once at start and again whenever it
  rotates its session key (on lock, on wipe). A shell that cannot answer
  leaves the request unanswered; the core then keeps working without
  sealing its secrets in memory and reports the session as weak." with
  "The core asks at start for its seed and a small pool, and again for
  each key it makes. A shell that cannot answer leaves the request
  unanswered; the core then makes no key, seals no vault and opens no
  signing round, and says the system gave no randomness. What a person
  adds (dice, camera, pointer and keys) is mixed with the shell's bytes
  in the core; the shell's bytes are never replaced."
- `docs/learn/03-randomness.md`, at the end of "This device":
  "Faraday also uses the computer's generator for keys it makes itself,
  such as a vault's, and for the one-time numbers a signature needs. You
  can add camera noise, dice rolls, coin flips or cards to it in
  Settings › Randomness.
  What you add is mixed in, not used instead, so the result is as good
  as the better of the two."
- `docs/learn/03-randomness.md`, a section after "Mixing":
  "## Adding your own to the device's. Settings › Randomness takes
  camera noise, dice rolls, coin flips and cards, and on a computer the
  pointer's movement over its box and your typing for it, while it is the
  source you chose. They are hashed together with the
  device's own randomness for everything the device makes by itself:
  vault keys, encryption keys and the one-time numbers of a signature.
  The device's randomness stays in the hash, so adding yours can only
  help. The row shows what you added; nothing you add is kept after a
  lock."
- `docs/learn/03-randomness.md`, "This device", after "it is what
  this device uses to blind its own calculations": "If you added your
  own randomness in Settings, this row uses it too, and says so."
- `docs/README.md`, the Spec list: "`docs/OWN-RANDOMNESS.md`: Your own
  randomness, and the kernel's at boot. Specified, not built (F86)."
  and in By question: "Where does Faraday's randomness come from, and
  can I add my own? | `docs/OWN-RANDOMNESS.md`, then
  `docs/SECURITY-MODEL.md`".

## 8. Add randomness at every press that draws the system's (F157)

The owner's decision (2026-10-28): wherever a cryptographic function
would just use the system's randomness, the person can add their own,
and this holds for seed generation too. §2.3 already mixes what was added
into every draw. `Fresh::take` and `Fresh::take_bytes` hash `acc` into
every pooled answer they take, `own_add` reseeds, so `Seed::draw` and
`sign_seed` follow, and New key's This device takes `device_own`. So
what was added before a press is in that press's bytes. What is missing
is the offer. This section puts it at every such press, as one pattern.

Nothing changes in what is mixed or how (§2.3), and nothing is collected
passively (§2.1, owner, 2026-10-26): the row only opens the page, and
the page starts no source until one is pressed.

Keys made from Dice, Coins or Cards are the person's own randomness:
they take none of the system's and have no Randomness row (owner,
2026-10-28). Hex and the camera are the same: what is entered is the
person's, and a camera is a source that adds to another, never a key's
only source. New key's This device, and a mix that includes it, draws
the system's randomness and is on the list; the Generator card already
offers the sources (§2.2).

### 8.1 The presses

Built from the code at fa72a199: every caller of `Faraday::fresh`,
`Faraday::fresh_bytes`, `Faraday::sign_seed` and `Fresh::draw`, and every
`Command::RequestEntropy` the core sends, in `faraday/faraday-core/src/`.

| # | Screen and press | What it draws | What it makes | Code |
|---|---|---|---|---|
| 1 | New key, This device (alone or in a mix): **Make the words** | an answer of its own, `device_own` with `acc` | the key's entropy | `keygen/area.rs` (`device`, `entropy`) |
| 2 | New key, SLIP-39 shares: **Next** on the Enter step, which deals the shares | `fresh_bytes("slip39 split")` | the split's random shares | `keygen/mod.rs` (`keygen_act`) |
| 3 | New key, codex32: **Suggest** beside the identifier | `fresh("codex32 identifier")` | the set's identifier | `keygen/mod.rs` (`keygen_codex32`) |
| 4 | Faraday Demo in Practice only: the dice roller on New key's dice pad, on the codex32 worksheet's step-through (**Roll**, **Play**), and on Create a vault's Generate with dice | `fresh("faraday dice roller")`, `fresh("faraday codex32 dice roller")` | simulated dice values, marked as this computer's | `keygen/mod.rs` (`keygen_roller_tick`, `keygen_codex32_walk`, `keygen_tick`), `vault/dice.rs` (`tick`) |
| 5 | Vaults, Create a vault: **Create** | `fresh("vault create")` | the vault's keys | `vault/mod.rs` (`vault_create`) |
| 6 | Vaults, a vault, Export for KeePass: **Seal and export** | `fresh_bytes("kdbx")` | the KeePass file's seed, IV, salt, inner key and UUIDs (F154) | `vault/mod.rs` (`vault_kdbx`) |
| 7 | A backup flow of a GPG key, its Make step: **Make the key** | `fresh("gpg primary")`, `fresh("gpg subkey")` | the primary key and the subkey | `backup_flow/steps.rs`, `vault/mod.rs` (`gpg_build`) |
| 8 | A backup flow of Secure Boot keys, its Make step: **Make the keys** | `fresh("secure boot")`, `fresh("secure boot enrolment")` | PK, KEK and db, their GUIDs, the enrolment files' blinding | `secure_boot/mod.rs` (`sb_build`) |
| 9 | Vaults, a Secure Boot record: **Enrolment files** | `fresh("secure boot enrolment")` | the signing's blinding | `secure_boot/mod.rs` (`sb_enrol`) |
| 10 | Vaults, a Secure Boot record, an image picked: **Hold to sign** | `fresh("secure boot signing")` | the signing's blinding | `secure_boot/mod.rs` (`sb_sign`) |
| 11 | A wallet's paper backup: **Seed XOR, 2 parts** or **3 parts** | `fresh_bytes("seed xor")` | the random parts | `wallet_backup.rs` (`paper_xor`) |
| 12 | A codex32 set's **Rotate the set**: **Make the new set** | `fresh_bytes("codex32 rotation")` | the new set's random shares | `backups/mod.rs` (`c32_make_new`) |
| 13 | Spend, the Sign step: **Sign here**, when the PSBT opens a MuSig2 round | `sign_seed` | the round's secret nonces | `lib.rs` (`sign_here`) |
| 14 | Spend, and a split wallet's Files: **Collect** a co-signer's PSBT, when a MuSig2 round is aggregated | `sign_seed` | a nonce shared again, never the one before | `lib.rs` (`collect`), `family/mod.rs` |
| 15 | **Lock**, a vault's **Lock this vault**, the idle lock, and every seal of a changed vault | `Seed::draw` (`vault_draw`) | the seal's nonce | `vault/mod.rs` (`vault_draw`, `vault_sealed`) |
| 16 | No press: a keyboard or touchscreen plugged in (F153) | `Seed::draw("faraday input code")` | its code and its target's place | `inputs.rs` (`input_code`) |
| 17 | No press: the seed arriving, or an addition | `Seed::draw("faraday key blinding")` | the blinding of the session's keys | `fresh.rs` (`keys_reblind`) |

Rows 16 and 17 are covered by the session's mixing, with no row of
their own: they have no press to put the row at. Row 15's row is on the
Lock sheet, shown only when the lock will seal a changed vault (owner,
2026-10-28); the idle lock has no sheet and is covered by the mixing.
Row 4, the demo's dice roller, gets no row (owner, 2026-10-28): it
stands in for dice in Faraday Demo's Practice only.

Draws nothing from the system, so nothing to offer: signing a PSBT
(ECDSA nonces are RFC 6979's, low-R; Schnorr's aux_rand is zero, F156),
GPG signing, a vault's open and its changes, Transfer, and every
restore.

**The Starter** draws no key material. Its held core is seeded like
every shell's (the shell answers `RequestEntropy` from `getrandom`), but
the core offers only Transfer and Learn, and neither draws. The Starter's
own `/dev/urandom` read (`faraday/shells/starter/src/pdf.rs`,
`random_name`) names a temporary folder nobody can predict. That is not
a cryptographic function's input, and the Starter has no Add randomness.
If a later Starter page draws for a key, the rule applies to it as to
the app.

### 8.2 The pattern: the Randomness row

One component, **Randomness row**, on the step, pane or prompt that
holds the press, directly above that press's button, so it is read
before the press:

- **What it is.** Material 3's list item, two lines
  (m3.material.io, Lists): the headline **Randomness** (`own_randomness`),
  the supporting text the status line of §2.4 (`own_status`, "System:
  getrandom(2) · your own: 99 dice rolls"), and as its trailing element
  Material's text button **Add randomness** (`own_add_randomness`;
  Buttons, Text button: the low-emphasis action of a list item), in a
  48 dp target. Colours are faraday-ui's roles: the headline
  on-surface, the supporting text on-surface-variant, the button primary.
  No icon, no explanation: a label, a value and an action.
- **What a press does.** It opens Add randomness (`page::Act::Open`), the
  page of §2.2, with no source chosen, no camera and no pad running. It
  is reached from the press as a new screen on the trail (F77): the
  page's top app bar reads "Back to {the step's page}" (F146), and Back
  lands on the step as it was left, fields, choices and scroll included.
  A step flow keeps its open step. A vault's prompt and pane are the
  vault area's state and are still there. On the Generator card (row 1)
  the sources stay on the card itself (owner, 2026-10-25) and no row is
  added.
- **Never blocks.** The press beside it is enabled exactly as it is
  today, whatever was added. Nothing asks, nags or waits for an addition.
  §3.5's "Waiting for randomness" waits for the system only.
- **What is used.** The press draws when it is pressed (§8.1). Everything
  added up to then is in its bytes, and the status line on the step says
  so as it changes.
- **Only where it draws.** Row 13's row shows only while the spend opens
  a MuSig2 round, and row 14's only while one is being aggregated. Row
  3's row sits on the codex32 identifier's step. Row 15's sits on the Lock
  sheet only when the lock will seal a changed vault. Row 4 has none.
- **Drawing reads.** The status line is a lookup and a format through the
  drawing memo (`docs/SPEED.md` §3.3), as Settings' row draws it. The
  row adds no costly work.

### 8.3 By window size (`docs/LAYOUT.md`)

- **Medium and wider.** The row as §8.2 draws it, at the column's width,
  one list item (72 dp) above the step's buttons.
- **Compact** (under 600 dp: a narrow window, a 7-inch panel in
  portrait). The same list item. Where the headline and the button do
  not fit on one line beside the supporting text, the supporting text
  wraps to a second line; the button keeps its 48 dp target and is never
  cut.
- **Compact and short** (the Pi's 268 × 358 panel). The step keeps the
  supporting text alone, one line in Label M above the press, shortened
  at a count's comma with an ellipsis where it would be cut, never in a
  number. **Add randomness** moves into the bar's overflow menu (the ⋮
  at the flow bar's or the bar's end, `compact::tools`, a sheet on the
  small panel as every menu there). It is that menu's first row, before
  How Faraday does it and Learn, and the ⋮ is drawn on every step that
  offers it, even where the bar's tools would otherwise fit (DESIGN
  §4's Bar row). Its 48 dp target and the chevron's do not overlap.

### 8.4 Code changes (batch R7)

- `faraday/faraday-core/src/screens/randomness.rs`: `row`, the Randomness
  row of §8.2, drawn by every screen below; the compact short form of
  §8.3.
- `faraday/faraday-core/src/compact/mod.rs`: the overflow menu's **Add
  randomness** row and the ⋮ kept on a step that offers it.
- The screens that draw it, at the presses of §8.1:
  `faraday/faraday-core/src/keygen/screen.rs` (rows 2, 3),
  `faraday/faraday-core/src/vault/screen.rs` (rows 5, 6, 9, 10),
  the Lock sheet's screen (row 15),
  `faraday/faraday-core/src/backup_flow/screen.rs` (rows 7, 8),
  `faraday/faraday-core/src/screens/backup.rs` (row 11),
  `faraday/faraday-core/src/codex32_backup/screen.rs` (row 12),
  `faraday/faraday-core/src/screens/spend.rs` (rows 13, 14).
- `faraday/faraday-core/src/fresh.rs`: `own_act(Open)` from any of them
  puts the step on the trail as Settings' row does. `own_on_show` is
  unchanged: the page alone takes input.
- `faraday/faraday-core/src/trail.rs`: Back from Add randomness restores
  the step as left (it does already for a screen; a test holds it for
  each press).
- `faraday/faraday-strings/src/en.rs`: no new string is expected
  (`own_randomness`, `own_add_randomness`, `own_status` serve). If the
  Pi's line needs a shorter form, it is one key there.
- `docs/DESIGN.md` §4: the Randomness row's component row. §5: the
  screens of §8.1 name it. The Bar row names its ⋮ entry.
- `docs/learn/03-randomness.md`, "Adding your own to the device's": the
  page is also opened from each press that makes a key or a nonce.
- `docs/SECURITY-MODEL.md`, the randomness paragraph (§7): "Settings ›
  Randomness" reads "Settings › Randomness, or Add randomness at the
  press".
- The snapshot tours: `faraday/shells/snapshot/src/tours/randomness.rs`
  gains a step at one press per screen file above, at 240 x 320 and
  480 x 640, so the touch-target test (F152) covers the row and the ⋮.
- `tools/lint-tokens.counts`: the row uses tokens only, so no count rises.

### 8.5 Tests

In `faraday/faraday-core/tests/all/own_randomness.rs`, user-facing:

1. **Reachable from each press.** For each of rows 1 to 14, on its step
   the drawn targets (`Faraday::drawn_targets`) include Add randomness's
   open, and on the Pi's panel its ⋮ row does. Pressing it shows Add
   randomness with no source chosen and no `CameraOn` sent. Back shows
   the step as it was left, and the press then works.
2. **It changes the press's bytes.** With fixed system answers, the same
   press made with and without 99 dice rolls added first gives other
   output: a vault's seal key, the GPG key's fingerprint, the Secure
   Boot keys' GUID, the SLIP-39 shares, the Seed XOR parts, the rotated
   codex32 shares, the KeePass file, a MuSig2 public nonce, This device's
   words. Each also gives other output for other system answers with the
   same dice (§2.3: never substituted).
3. **Known answers.** One press per draw kind (`fresh`: the GPG key;
   `fresh_bytes`: the Seed XOR parts; `sign_seed`: the MuSig2 public
   nonce; This device's words) is compared with the Python cross-check's
   bytes for a fixed system answer and a fixed dice batch (F79).
4. **Never blocks.** With nothing added, each press works as before and
   gives the bytes it gives today (§4 test 4, extended to the presses).
5. **Only where it draws.** A spend with no MuSig2 input has no
   Randomness row on its Sign step. A Dice, Coins or Cards key has none.

### 8.6 Questions for the owner

1. **Lock and seals (row 15).** Settled (owner, 2026-10-28): a row on the
   Lock sheet, shown only when the lock will seal a changed vault.
2. **The demo's dice roller (row 4).** Settled (owner, 2026-10-28): no
   row; the roller stands in for dice for practice.
