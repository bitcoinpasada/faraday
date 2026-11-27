# Speed: what a frame and an event may cost

**Kind:** Spec
**Open:** The rules of §1 to §3 hold in what is built (the runtime draws only after an event and reads, the memo is `Show`'s); the budget tests, the instruction counts and the timed late-flow session of §4 come with the screens, and the memo's store with the workers (spec 05 §6).

The device is a Raspberry Pi: a frame is drawn in a bounded time, and work that is not drawing runs
where it costs nothing the person sees. These are the rules that keep it so; the runtime keeps them
(spec 05 §2.5, §5).

## 1. Drawing reads

- Drawing builds `View` and `Show` from borrows, calls `Root::frame`, and works nothing out: a value
  a screen needs that costs is kept in the memo, keyed by the inputs it came from (04 §6), or in a
  `Derived` on a feature's state, worked out in `update` with the event's `Compute`.
- A frame is drawn after an event that dispatched anything, at each tick while the last frame said
  `Pace::Moving`, and when the note on show changes. An idle process draws nothing.
- A fact the layout gives back (a laid fact) is sent only when it differs from the last one sent for
  its anchor, so a fact sent every frame cannot make every frame dispatch and draw again.

## 2. Costly work

- Costly work takes `c: &Compute` (`docs/COMPUTE.md`), so it can run only where a token is: in an
  event's handling, on the root thread, or on a pool lane.
- Work that may take longer than a frame is a job (`Fx::Job`): its answer is a message and its
  input is moved in. The heavy lane runs the jobs that ask Argon2id for memory; the light lane runs
  the rest; each lane runs its jobs one at a time (spec 05 §5.1).
- The memo is emptied at every lock and every end, so no costly value outlives the scope.

## 3. Parse once

A value read from bytes (a PSBT, a descriptor, a file's contents) is parsed once, where it comes
in, and kept parsed; a screen never parses what it shows. A published file's result is kept with
the file by its SHA-256 (04, the Inbox's kept results).

## 4. How it is held

- A named costly operation is held by an instruction count, raised only in a commit that says why
  (decision F238); a drawing budget test counts allocations and work per frame with
  `faraday-alloc-count`.
- Work added while drawing is timed once in a late-flow session, before it is pushed, and the time
  is in the commit message.
