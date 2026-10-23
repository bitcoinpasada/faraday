# Your own randomness, and the kernel's at boot

**Kind:** Spec
**Status:** Specified, not built (F86); the kernel check of §3 done from the repository
**Open:** all of §2 (adding your own randomness, R1 to R4); the kernel's crediting on the stick (§3.3, the owner's choice) and the boot measurement (§3.4, needs hardware).

Two things. (a) A person may add randomness of their own, dice rolls or
camera noise on every shell and pointer movement and key timings on the
desktop, which is hashed together with the system's bytes into the
session's seed and pool. It is mixed, never substituted: what comes out
is at least as strong as the stronger of the two. (b) What the stick's
and the Pi's kernels do for randomness at boot, read from the image's
configuration, and a plan for the one measurement that needs hardware.

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
  fresh" ‖ what ‖ bytes): vault creation, GPG, APK and Secure Boot keys,
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
| Pointer and keys | the desktop app and the Starter's held app | while the page is open: each pointer move's x and y, each key's code, each with the core's time in ms and a running count | SHA-256 over the events, committed every 64 events and when the page is left |

Nothing is credited: no source claims a number of bits, and the status
line counts what was taken, not how strong it is. A dice batch of 99
rolls or more is named as enough for 256 bits on its own, as New key
names it.

Pointer and key events are taken only while the page is open, never in
the background: a passphrase's key timings are never hashed into
anything.

### 2.2 Where it is offered

- **Settings › Randomness**: a row with the status line (§2.4); it opens
  **Add randomness**, a page with Dice, Camera and, on the desktop,
  Pointer and keys. Each source adds when its batch is done (dice: the
  **Add** button under the pad; camera: each frame; pointer and keys:
  as above). The page stays until Back.
- **New key, This device**: its row's label carries the status (§2.4),
  and the source's step has a link to Add randomness.
- Not on Home: Home is a working screen and its one randomness line,
  the missing seed, stays as it is.

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
  device own" ‖ bytes ‖ `acc`) when `acc` exists (§5, question 2).
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
- then "· your own: {counts}", for example "· your own: 99 dice rolls,
  6 camera frames, 412 pointer moves", or "· your own: none".

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

The comment in the fragment states a rule (do not credit the CPU or the
firmware), and the kernel does not follow it. Two ways to make the
image and its comments agree:

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

The default proposed here is A, if §3.4 shows the first answer within
two seconds of the kernel starting on every machine tried (the core's
`SEED_GRACE_MS` is two seconds after the first display). If not, B.
Either way the stick shell's comment at
`faraday/shells/stick/src/main.rs:602-613` changes: it says the board
seeds the pool from bcm2835-rng, which is the Pi's and not the stick's.

### 3.4 The measurement

One hardware measurement: how long `/dev/random` takes to be ready
after power-on, and when the app's first answer arrives.

1. **Read the generated configs** (read-only, the build machine): in
   each board's Buildroot output, `grep -E
   'HW_RANDOM|TCG_TPM|RANDOM_TRUST|CMDLINE' .config`, and for the Pi,
   whether `RANDOM_TRUST_BOOTLOADER` is set.
2. **Instrument the dev image only.** The stick shell, in the dev
   variant, writes to its log the CLOCK_BOOTTIME at which it opened
   `/dev/random` and at which the read returned. The kernel's own
   "random: crng init done" line carries its time in `dmesg`, at
   notice level, kept in the buffer in every variant.
3. **Machines.** The x86 stick on at least three PCs of different ages
   (one without RDRAND if one is at hand, otherwise QEMU's `qemu64` CPU,
   which has none, through `faraday/tools/stick-qemu.py`); the Pi 3 with
   its panel.
4. **Runs.** Ten cold boots each (power off, not reboot): the image as
   it is, and on x86 with A's two parameters (a dev cmdline line). On
   the Pi, also read `/sys/module/rng_core/parameters/default_quality`,
   `/sys/class/misc/hw_random/rng_current` and whether
   `/proc/device-tree/chosen/rng-seed` exists.
5. **Recorded.** Per machine and setting: the time of crng init, the
   time of the app's first answer, the worst of ten. The results go in
   this section; the dev instrumentation stays in the dev variant only.

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

For §3: if A is chosen, `check-kernel-config.sh` fails a board whose
`CONFIG_CMDLINE` lacks the two parameters.

## 5. Questions for the owner

1. The stick: A (do not credit RDRAND or the firmware's seed, wait for
   jitter) or B (accept the kernel's default trust)? Proposed: A if the
   measurement allows.
2. Should New key's This device take your own randomness too (proposed:
   yes, and its label says so), or stay the system's bytes alone as
   today?
3. Where Add randomness is offered: Settings and New key (proposed), or
   also Home?
4. Pointer and keys only while the page is open (proposed), or all
   session long outside text fields?
5. Should the stick's and the Pi's shells offer Add randomness at
   boot, before the passphrase, or only from Settings as proposed?
6. Coins and cards are in New key but not proposed here, to keep the
   page short. Add them?

## 6. Batches

| Batch | What | Size |
|---|---|---|
| R1 | `Own` in `fresh.rs`: adding, the seed's and the pool's mixing, the lock; tests 1–4; the cross-check | small |
| R2 | The Add randomness page: dice and camera on every shell, through `faraday-entropy`'s accumulators; Settings' row and the status line; New key's label and link; tests 5 | medium |
| R3 | Pointer and keys on the desktop; test 6 | small |
| R4 | The kernel: the owner's A or B (§3.3), the comments at `faraday/image/boards/x86_64-uefi/linux.fragment:267-275`, `kernel.forbidden:8-9`, the Pi fragments' lines 81-82, the stick shell's comment; the dev instrumentation; then the measurement on hardware | small, then hardware |

R4's comment fixes do not wait for the measurement: whichever way the
owner chooses, the comments say what the kernel does.

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
  `faraday/image/boards/x86_64-uefi/cmdline.txt` (if A),
  `faraday/image/check-kernel-config.sh` (if A),
  `faraday/image/boards/pi3/linux.fragment`,
  `faraday/image/boards/pi02w/linux.fragment`,
  `faraday/shells/stick/src/main.rs`.

## 7. Docs that follow

- `docs/SECURITY-MODEL.md`, a paragraph at the end of "Where keys live
  in the program": "Randomness. The core has no generator: the shell
  answers with 32 bytes from `/dev/random` on the stick and the Pi and
  the operating system's call on the desktop. A person may add dice
  rolls, camera noise and, on the desktop, pointer and key timings
  (Settings › Randomness); these are hashed with the system's bytes
  into the session's seed and every fresh draw, never used alone, so
  what comes out is at least as strong as the stronger of the two. On
  the stick the kernel {credits | does not credit} the CPU's RDRAND and
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
- `docs/learn/04-randomness.md`, at the end of "Computer randomness":
  "Faraday also uses the computer's generator for keys it makes itself,
  such as a vault's, and for the one-time numbers a signature needs. You
  can add dice rolls or camera noise to it in Settings › Randomness.
  What you add is mixed in, not used instead, so the result is as good
  as the better of the two."
- `docs/learn/05-where-randomness.md`, a section after "Mixing":
  "## Adding your own to the device's. Settings › Randomness takes dice
  rolls and camera noise, and on a computer the pointer's movement and
  your typing while the page is open. They are hashed together with the
  device's own randomness for everything the device makes by itself:
  vault keys, encryption keys and the one-time numbers of a signature.
  The device's randomness stays in the hash, so adding yours can only
  help. The row shows what you added; nothing you add is kept after a
  lock."
- `docs/learn/05-where-randomness.md`, "This device", after "it is what
  this device uses to blind its own calculations": "If you added your
  own randomness in Settings, this row uses it too, and says so." (if
  question 2 is yes)
- `docs/README.md`, the Spec list: "`docs/OWN-RANDOMNESS.md`: Your own
  randomness, and the kernel's at boot. Specified, not built (F86)."
  and in By question: "Where does Faraday's randomness come from, and
  can I add my own? | `docs/OWN-RANDOMNESS.md`, then
  `docs/SECURITY-MODEL.md`".
