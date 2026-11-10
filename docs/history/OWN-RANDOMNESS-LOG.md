# Your own randomness, and the kernel's at boot: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/OWN-RANDOMNESS.md` on 2026-11-05. Each keeps
its original number; the source keeps its heading.

## 6 Batches (from docs/OWN-RANDOMNESS.md)

The labels are the ones decisions and old commits use.

| Batch | What | Size |
|---|---|---|
| R1 | `Own` in `fresh.rs`: adding, the seed's and the pool's mixing, the lock; tests 1–4; the cross-check | Built |
| R2 | The Add randomness page: camera (the default), dice, coins and cards on every shell, through `faraday-entropy`'s accumulators; Settings' row and the status line; New key's label and link; tests 5 | Built |
| R3 | Pointer and keys on the desktop app, as §5 item 4 decides: `faraday-entropy`'s `PointerKeys`, the source on the page and the card, its known answers in the cross-check; test 6. Built | small |
| R4 | The kernel: A (§3.3), the comments at `faraday/image/boards/x86_64-uefi/linux.fragment:267-275`, `kernel.forbidden:8-9`, the Pi fragments' lines 81-82, the stick shell's comment; the dev instrumentation; then the measurement on hardware | Built, then hardware |
| R5 | §3.5: the late seed (Home's line gone, a press waiting with "Waiting for randomness", the minute's bound) in `fresh.rs` and the screens that need the seed; tests that a press made before the seed goes on when it arrives and that a minute without one says so | small |
| R6 | §3.5: the Pi's generator mixed, not credited (the command line, and the shell's write where the kernel needs it), checked against the Pi kernel's source | small |
| R7 | §8: the Randomness row at every press of §8.1, its place in the Pi's ⋮, the way back to the press; §8.5's tests | medium |

## Sections moved on 2026-11-10 (from docs/OWN-RANDOMNESS.md)

Moved when the Spec became a Reference, its last open work, the boot measurement on one real PC, being read. Each keeps its original number.

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

## 6. Order of building and files changed

The mixing and the lock come first, then the Add randomness page, then the desktop's pointer and keys, which need the page. The kernel change does not wait for the others. The late seed and the Pi's generator (§3.5) follow, and the Randomness row at every press (§8) comes last, as it needs the page.

The kernel change does not wait for the measurement; the measurement confirms it, and
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
  mixes in the SoC's generator, bcm2835-rng, without crediting it."
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

### 8.4 Code changes

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
  `faraday/faraday-core/src/screens/spend.rs` (row 13).
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

### 8.6 Questions for the owner

1. **Lock and seals (row 15).** Settled (owner, 2026-10-28): a row on the
   Lock sheet, shown only when the lock will seal a changed vault, and on
   the open vault's screen while it holds changes not yet sealed, for the
   idle lock.
2. **The demo's dice roller (row 4).** Settled (owner, 2026-10-28): no
   row; the roller stands in for dice for practice.
