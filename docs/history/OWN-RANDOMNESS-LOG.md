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
