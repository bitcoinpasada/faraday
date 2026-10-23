# Your own randomness, and the kernel's at boot

**Kind:** Spec
**Status:** Specified, not built (F86); the kernel check of §3 done from the repository
**Open:** all of §2 (adding your own randomness, R1 to R3); the boot measurement that confirms option A brings no long wait (§3.4: QEMU on the build machine, not yet run; then one real PC in passing). Option A itself (§3.3, R4) is built.

Two things. (a) A person may add randomness of their own, camera noise,
dice rolls, coin flips or cards on every shell and pointer movement and
key timings on the desktop, which is hashed together with the system's bytes into the
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
  answer is the seed (`Seed`); the next ones wait in a pool of up to six.
  `fresh(what)` takes one pooled answer and returns SHA-256("faraday
  fresh" ‖ what ‖ bytes): vault creation, GPG and Secure Boot keys,
  Seed XOR, codex32, KeePass, SLIP-39's split. `Seed::draw` is SHA-256
  (what ‖ seed ‖ count): a vault's seal nonces, a new keyboard's code.
  `sign_seed` draws a MuSig2 or FROST round's nonces from the seed with a
  count. Nothing is made before the first answer: `seeded()` is false,
  `fresh` and `sign_seed` return `None`, and after two seconds Home says
  the system gave no randomness.
- **New key** (`faraday/faraday-core/src/keygen/mod.rs`, `core/faraday-entropy`)
  already offers dice, coins, cards, hex, camera noise, this device
  (`DeviceRandom`, one answer of its own, not the pool) and a mix of two
  to five of them (`Mixed`: SHA-256 over the sources' commitments). A
  key from dice alone takes nothing from the system, by design: it can be
  reproduced elsewhere.

What is not built: nothing a person adds reaches the seed or the pool,
so every key Faraday makes itself (a vault's, GPG's, Secure Boot's, a
share split's) and every signing round's nonce rests on the system's
generator alone.

## 2. Adding your own randomness

### 2.1 Sources

| Source | Shells | What is taken | Its commitment |
|---|---|---|---|
| Dice | every shell | rolls on the dice pad, as New key takes them (`DiceRolls`) | SHA-256 of the rolls as ASCII digits, as `faraday-entropy`'s dice commitment |
| Camera noise | every shell with a camera | frames, as New key takes them (`CameraNoise`, the same refusal of a frame with under 32 distinct values) | each frame's SHA-256, as it is taken |
| Coins | every shell | flips, as New key takes them (`CoinFlips`) | `CoinFlips::commitment`, as New key's |
| Cards | every shell | draws from a shuffled deck, as New key takes them (`CardDraws`, refusing a card already drawn from its deck) | `CardDraws::commitment`: SHA-256 of the draws' indices, as New key's |
| Pointer and keys | the desktop app and the Starter's held app | while the page is open (§5 item 4): each pointer move's x and y, each key's code, each with the core's time in ms and a running count | SHA-256 over the events, committed every 64 events and when the page is left |

Nothing is credited: no source claims a number of bits, and the status
line counts what was taken, not how strong it is. A dice batch of 99
rolls or more is named as enough for 256 bits on its own, as New key
names it.

Pointer and key events are taken only while the page is open, never in
the background: a passphrase's key timings are never hashed into
anything. This is what is specified until the owner decides §5 item 4;
the owner leans towards collecting from all on-screen input, mixed in
automatically, and that answer changes this paragraph, the table's row,
§2.2, test 6 and batch R3.

### 2.2 Where it is offered

- **Settings › Randomness**: a row with the status line (§2.4); it opens
  **Add randomness**, a page with Camera, Dice, Coins, Cards and, on the
  desktop, Pointer and keys. It opens on Camera, the default source
  (owner, 2026-10-23), where the shell has a camera, and on Dice where it
  has none; the others are a choice away. Each source adds when its
  batch is done (dice, coins and cards: the **Add** button under the
  pad; camera: each frame; pointer and keys: as above). The page stays
  until Back.
- **New key, This device**: its row's label carries the status (§2.4),
  and the source's step has a link to Add randomness.
- Not on Home: Home is a working screen and its one randomness line,
  the missing seed, stays as it is.
- Not at boot: the stick's and the Pi's shells do not offer it before
  the passphrase; it is reached from Settings and New key only.

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
  frames, 99 dice rolls, 412 pointer moves", or "· your own: none"; coin
  flips and cards are counted as dice rolls are.

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

Results: none yet (2026-10-23). The dev image could not be built from a
worktree: the remote run script gives a worktree's checkout on the build
machine a new empty git repository, and `faraday/image/build-id.sh`
(`git diff HEAD`) fails there with "bad revision 'HEAD'" before any
compile. QEMU, `stick-qemu.py` and OVMF were not reached.
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
6. Pointer and key events count only while Add randomness is open: the
   same events on Home leave the count unchanged.

The construction (§2.3) is SHA-256 over concatenations; per CLAUDE.md a
cryptographic change is checked against another implementation, so its
known answers are added to the Python cross-check (F79) and test 4's
and a mixed case's bytes are compared with it.

For §3: `check-kernel-config.sh` fails a board whose `CONFIG_CMDLINE`
lacks the two parameters (A).

## 5. Questions for the owner

Answered by the owner on 2026-10-23, except item 4:

1. **The stick.** Decided: A, do not credit RDRAND or the firmware's
   seed (§3.3). The boot measurement (§3.4) still runs, to confirm no
   long wait.
2. **New key's This device.** Decided: yes, it mixes in what the
   person added, and its label says so (§2.3, §2.4).
3. **Where Add randomness is offered.** Decided: Settings and New key
   only (§2.2).
4. **Pointer and keys.** Decided: only while the page is open, as
   §2.1 specifies. Collecting timings passively from all on-screen input
   is set aside (owner, 2026-10-23), not planned.
5. **At boot.** Decided: not at boot for now; only from Settings and New
   key (§2.2).
6. **Coins and cards.** Decided: yes, added as sources (§2.1).

Also decided (owner, 2026-10-23): camera noise is the default source;
the page opens on it, and the others stay available (§2.2).

## 6. Batches

| Batch | What | Size |
|---|---|---|
| R1 | `Own` in `fresh.rs`: adding, the seed's and the pool's mixing, the lock; tests 1–4; the cross-check | small |
| R2 | The Add randomness page: camera (the default), dice, coins and cards on every shell, through `faraday-entropy`'s accumulators; Settings' row and the status line; New key's label and link; tests 5 | medium |
| R3 | Pointer and keys on the desktop, once the owner decides §5 item 4; test 6 | small |
| R4 | The kernel: A (§3.3), the comments at `faraday/image/boards/x86_64-uefi/linux.fragment:267-275`, `kernel.forbidden:8-9`, the Pi fragments' lines 81-82, the stick shell's comment; the dev instrumentation; then the measurement on hardware | small, then hardware |

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
  `faraday/image/boards/pi02w/linux.fragment`,
  `faraday/shells/stick/src/main.rs`.

## 7. Docs that follow

- `docs/SECURITY-MODEL.md`, a paragraph at the end of "Where keys live
  in the program": "Randomness. The core has no generator: the shell
  answers with 32 bytes from `/dev/random` on the stick and the Pi and
  the operating system's call on the desktop. A person may add dice
  rolls, coin flips, cards, camera noise and, on the desktop, pointer
  and key timings (Settings › Randomness); these are hashed with the system's bytes
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
  pointer's movement and your typing while the page is open. They are hashed together with the
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
