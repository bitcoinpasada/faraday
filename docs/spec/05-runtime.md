# The runtime: the queue, the memo, the workers, the lock and the restart

**Kind:** Spec
**Open:** Not built.

This section gives the runtime (section 01, R1 to R5): the crate that owns the model, takes every
event, dispatches every effect, runs the worker pool, keeps the memo and the last frame, and
carries out a lock, an end and a rebuild of the lock scope. It fixes the order in which an event
and everything it causes are handled (A15), how effects reach the adapters, the pool and the
shared owners, how workers are stopped and their stale answers dropped, where the token for costly
work is minted, what crosses a lock and where it waits while no scope exists, and what Faraday
Demo and the Starter do at a lock. It answers the questions sections 01 to 04 and 06 left for it.
It is written for whoever writes the runtime crate, a product crate's `Root` (section 06), an
adapter's reset, or a feature that carries state across a lock. Rules continue section 01's
runtime numbering: R6 onward.

## 1. The crate and its shape

The runtime is one crate (`layer = "runtime"`). It depends on the kernel, the port, the kit's atoms
(the `Ui`, `FrameOut`, the motion store and the frame) and the domain crates it calls
(`faraday-crypto` for `Compute` and the wiping buffer, `faraday-selftest`). It does not depend on
the presentation kernel (04 §2): the root's one frame function calls `present` and `draw`. It
names no feature (R1), spawns only its pool's threads and does no I/O (R3).

Each product crate (`layer = "app"`, `role = "product"`) implements `Root`. A product crate may
depend on the runtime; only roots and tools depend on a product crate (01 §1).

```rust
/// What a product is made of, as the runtime sees it. Its value carries the product's Options.
pub trait Root: Sized + 'static {
    type Settings: shared::Owner<Out = Top, State: Borrow<Settings>>;
    type Session: shared::Owner<Out = Top, State: session::Read>;
    type Vaults: shared::Owner<Out = Top, State: vaults::Read>;
    /// The frame and the product's composite (06 §4).
    type App: Feature<Out = Top>;
    /// The Starter's own composite, outside the lock scope (§7.9); `Nothing` elsewhere.
    type Outside: Feature<Out = Top>;
    /// One variant per top state's press (kit atoms, 04 §2.1).
    type Press: Pressed;
    type Options: Public;

    /// The mode the scope is built in: the carried one, or the root's own (§7.5).
    fn mode(&self, carried: &mut Carried, facts: &Facts) -> Mode;
    /// A press of the last frame, or a laid fact's message, addressed to its top state.
    fn press(press: Self::Press) -> Addr<Self>;
    /// Input the last frame did not take (Back, Forward, a key with no focus).
    fn input(&self, tops: &Tops<'_, Self>, input: &port::Input) -> Option<Addr<Self>>;
    /// A stream fact (devices, camera frames and codes, a host's drop, a write's progress).
    fn fact(&self, tops: &Tops<'_, Self>, fact: Stream) -> Option<Addr<Self>>;
    /// The window's close button, as a message (desktop roots).
    fn close_asked(&self) -> Option<Addr<Self>>;
    /// Present then draw (04 §1, V5): the app's frame and the page its navigation shows,
    /// presented from the top state that holds it (an owner's page from `tops`).
    fn frame(&self, tops: &Tops<'_, Self>, view: &View<'_>, show: &Show<'_>, ui: &mut Ui<'_, Self::Press>);
}

/// The top states, lent read-only. `None`: the Starter's lock scope is not built.
pub struct Tops<'a, R: Root> { /* settings, session, vaults, app, outside */ }
impl<'a, R: Root> Tops<'a, R> {
    pub fn settings(&self) -> &'a OwnerState<R::Settings>;
    pub fn session(&self) -> Option<&'a OwnerState<R::Session>>;
    pub fn vaults(&self) -> Option<&'a OwnerState<R::Vaults>>;
    pub fn app(&self) -> Option<&'a <R::App as Feature>::State>;
    pub fn outside(&self) -> &'a <R::Outside as Feature>::State;
}

/// A message addressed to one top state. The variant names the part it is for.
pub enum Addr<R: Root> {
    Settings(<R::Settings as Feature>::Msg),
    Session(<R::Session as Feature>::Msg),
    Vaults(<R::Vaults as Feature>::Msg),
    App(<R::App as Feature>::Msg),
    Outside(<R::Outside as Feature>::Msg),
}

/// A port fact that goes to one feature, not into `Facts` (§2.2).
pub enum Stream { Devices(DevicesFact), Camera(CameraFact), Host(HostFact), System(SystemFact) }

pub struct Runtime<R: Root> { /* the top states, queue, replies, pool, facts, frame, memo, log */ }

impl<R: Root> Runtime<R> {
    pub fn new(root: R, waker: Arc<dyn port::Waker>, workers: Workers) -> Self;
    /// One port event and everything it causes (§2). Returns the effects for
    /// the root to hand to its adapters, in the order they were made (P18).
    pub fn event(&mut self, event: port::Event) -> Vec<port::Effect>;
    /// The waker woke the loop: worker answers and progress (§5).
    pub fn woken(&mut self) -> Vec<port::Effect>;
    /// The frame to paint, when it changed since the last call (§2.5).
    pub fn frame(&mut self) -> Option<port::Frame<'_>>;
    /// The lock scope's phase (§7).
    pub fn phase(&self) -> Phase;
}

/// Closed: the Starter's scope is not built. Ended: the process is over.
pub enum Phase { Starting, Running, Locking, Ending, Closed, Ended }
pub enum Workers { Threads, Inline }   // Inline: the tour root and runtime tests (§5.1)
```

`Nothing` is the runtime's feature with no state, no message and no page, the `Outside` of every
root but the Starter. Its state is the unit type `Empty`, which holds nothing and wipes nothing,
and its refusal is `NoRefusal`, an enum with no variant, since it has no message to refuse. The assertion that no top state is `Clone` is written in each product crate
over its own types (01 L1), since Rust checks a negative bound only on a concrete type.

**The lock scope** is the part of the model a lock drops and builds again. In the device app and
in Faraday Demo it is the whole model: the three owners and the app. In the Starter it is the
session, the vaults and the app (Faraday Demo's), and the settings owner and `Outside` stand
outside it (§7.9). Only the scope's states are wiped, dropped and built again.

### 1.1 What the top states tell the runtime

**Decision: the `Out` of every top state (the three owners, the app and `Outside`) is the
kernel's `Top`, and the runtime handles it.** A feature asks for a lock, an end or a broadcast by
an `Out` its parents route up, as section 03 routes any out; no new kind of effect is needed.

```rust
/// In the kernel's `feature` module.
pub enum Top {
    /// Every secret on show off the screen now: the idle warning (§2.3).
    Hide,
    /// What an owner holds came, went or changed; broadcast as `Notice::Shared`.
    Shared(shared::Changed),
    /// Begin a lock of the scope (§7.2).
    Lock(Lock),
    /// End the process, or close the Starter's scope (§7.7, §7.9).
    End(End),
    /// Run the self-test again (Settings' Run again).
    SelfTest,
    /// The Starter only: build the lock scope, after `PrepareDemoSticks` answers `Ok` (§7.9).
    Open,
}
pub struct Lock { pub seal: bool, pub idle: bool }   // seal: false is "lock without saving"
pub enum End { PowerOff, RestartComputer, Quit }
```

Reason: a lock must pass through the runtime's order (§7.2) and a broadcast must reach every top
state, and both are already an `Out` in section 03's routing. Turned down: new `Fx` variants for
lock and broadcast, which would give every feature a way to skip its parents.

- **Rule R6.** Only the runtime emits `Order::Exit`, `Order::Clean`, `Ask::Keep` and
  `Ask::Forget`. No application crate names them (`port::Exit` included), and only an owner's
  crate names `Top::Shared`. Held by `lint-layers`' module rule. A Lock pressed anywhere reaches
  the runtime as `Top::Lock`, so no press can end a scope without the hide, the seal and
  `at_lock` of §7.2, no feature can mark the process clean, and no feature can hand the carry to
  an adapter or erase it.
- A `Top::Lock` or `Top::End` while one is under way is dropped and noted. A `Top::Open` in a
  root whose `Outside` is `Nothing`, or while the scope is built, is dropped and noted.

## 2. The event queue and the order of dispatch

### 2.1 One event at a time

The root hands the runtime one port event at a time (P18). Finished jobs are events of the
runtime's own: each is handled as one event, before the port event the root hands in next, in the
order the jobs ended (§5.3).

Handling one event:

1. The event becomes zero or more addressed messages (§2.2), pushed on the event's queue.
2. The runtime pops the front message, dispatches it to its top state through `route` (A9),
   and walks the effects it returns in order (§3). What they cause is pushed at the back.
3. Step 2 repeats until the queue is empty.
4. The orders and asks collected in steps 2 and 3 are returned to the root, in the order made.
5. The clean marker is worked out (§4.3); if it changed, `Order::Clean` is appended.
6. A frame is drawn if anything was dispatched (§2.5).

- **Rule R7.** An event and every message it causes (a `Then`, a routed `Out`, a change and its
  `Done`, a notice) are handled before the next event, first in first out, so each message is
  handled in the order it was returned; a frame is drawn only after. This is A15, made exact.
  Held by the runtime and a test of the runtime with a cascade across two features and an owner.
- **Rule R8.** One event's cascade is bounded (10 000 dispatches). Past the bound the runtime
  drops the queued `Then`s and notices, notes a fault for diagnostics and draws. Every queued
  message that answers something (a change's `given`, the asker's message of a `Done` or of an
  answer) is carried into the next event, so no change or ask waits for an answer that cannot come
  (A22, R12); at a lock, a `Close` whose `given` was carried over is given in that event. A lock that reaches the bound goes on, and a vault whose
  Outbox put was dropped is named failed in the vault summaries (§7.4). Reason: a feature that
  sends itself `Then` forever must not stop the screen. Held by a test of the runtime.

### 2.2 What each event becomes

| Event | Becomes |
|---|---|
| `Start` | the model (§7.5); once per process, in `Starting`; a later one is dropped and noted (P17) |
| `Display`, `Sticks(SticksFact::*)`, `Host(HostFact::Downloads)`, `Camera(CameraFact::Listed)`, `System(SystemFact)` but `WriteProgress` | the field of `Facts` replaced (`Facts::system` for the Starter's system facts), then `Notice::Facts` to every top state |
| `Time` | `View::clock` set; `Notice::Facts` with `time` once each wall second; a frame each tick while the last frame said `Pace::Moving` (§2.5) |
| `Input::Touch` | the press of the last frame's hit target under it (R9), through `Root::press` |
| `Input::Key` | the focused control along the last frame's `reach` order (04 §7.1), then `Root::input` |
| `Input::Scroll`, `Wheel`, `ScrollEnd` | the runtime's motion: the motion store moved against the last frame's `regions` (04 §8.1); no message |
| `Input::Hover` | the pointer state the next frame reads; no message |
| every other `Input` (`Back`, `Forward`, `KeyUp`, `Shift`, `Focus`) | `Root::input` |
| `Devices(DevicesFact::*)`, `Camera::{Frame, Code, Seen, Ended}`, `Host::{Hovered, Dropped}`, `System(SystemFact::WriteProgress)` | `Root::fact`, after the stream check of §5.5 |
| `Answer(id, answer)` | the kept reply called (§3.1) and its message to the asker |
| `CloseAsked` | `Root::close_asked` |
| a job's end | the job's message to the top state that started it (§5.3) |

Every `Input` is dropped while the scope is `Locking` or `Ending`, for the whole window, the
Starter's own composite included.

- **Rule R9.** A press is hit-tested against the last frame handed to the root, the one the person
  saw, and then passes `refuses` against the current state (A9, A10). A control that became
  refused between the frame and the press is refused. The runtime draws a frame after a press is
  handled before it matches the next press, so a press always meets what the person saw. Held by
  the runtime and a runtime test.

### 2.3 Notices

The runtime sends a notice to each top state in a fixed order (settings, session, vaults, app,
outside), each as one queue item; a state not built gets none. A composite forwards it to its
children (A21).

- `Notice::Facts` follows a change of `Facts` (§2.2).
- `Notice::Shared(changed)` follows each `Top::Shared` from an owner. The owner that changed
  broadcasts it, by its `Out`.
- `Notice::Hide` follows `Top::Hide` and the start of a lock. After each top state's `notice`, the
  runtime calls its `AtLock::hide` (section 03 §2.4).
- `Notice::Left` is the app's navigation's (A13), with one exception: when the runtime closes the
  Starter's lock scope (§7.9), it sends `Notice::Left` to the Starter's own composite
  (`Root::Outside`), whose navigation then leaves the Demo's page.

### 2.4 The Compute token in event handling

- **Rule R10.** `Compute` is minted in shipped code at exactly these places: once in
  `Runtime::new`, on the root's thread, for every `Ctx` of event handling; and once on each pool
  thread as it starts (§5.1). This makes R2 exact. Held by `costs-lint`, which lists the two mint
  sites by path and refuses any other in a shipped crate, and by `clippy.toml`'s
  `disallowed-methods` on `Compute::new`, allowed at those two sites under `#[expect(..., reason)]`.

`Compute` stays `!Send`, so the root thread's token never reaches a job.

### 2.5 Frames

After an event that dispatched anything, at each tick while the last frame said `Pace::Moving`,
and when the note on show changes, the runtime draws one frame, in this order (04 §1):

1. It builds `View` (§4.1) and `Show` (04 §3) with the memo and the note up.
2. It builds a `Ui` over the canvas with the last frame's pointer state, the motion store and the
   theme from `view.settings`, and calls `R::frame`, which presents and draws (V5). The screen
   model is a local of `R::frame` and is dropped when it returns.
3. `Ui::finish` gives the `FrameOut` (04 §7.1): hits, the `reach` order, anchors, scrolled
   regions, laid facts, the `secret` flag, the pace, the page's `Anchor` and the sheets' anchors
   the screen template set, and the ink record when it is on (§9).
4. The canvas replays its call list into the frame buffer (`Secret<Vec<u8>>`, P13), recording the
   changed rectangles, and wipes the list's text (V20).
5. Each `Fx::View` order waiting (§3) whose anchor this frame drew moves the motion store: a
   `Reveal` glides the region to the anchor's place (at once under Reduce motion), a `Top` sets its
   offset to the top. The next frame draws it moved. An order whose anchor no frame has drawn
   waits, and a lock drops it.
6. The laid facts are compared with the last one sent for each `Anchor` (V17). Each that differs,
   or is the first, becomes `Root::press(press)` and is handled as one event after the frame; a
   frame is drawn again if that dispatched anything, at most once more. Facts that change again in
   that second frame are sent only after the next event; in debug builds that is an assertion
   ("layout settles in one frame"). Reason: a fact sent every frame would make every frame
   dispatch and draw again (01 V2). A laid fact is an answer of the layout and is never refused
   (03 §2.2).
7. The frame is kept for a fade or a sheet's frost only when its `secret` flag is clear, but as
   a step fade's source, emptied when that fade out ends or is cut short (04 V19, F337).

The frame keeps its hit targets, focus, pointer state and the kit's motion state (K5) until the
next frame. No model or state holds any of them (V3).

- A hold that completes is the runtime's: when the pointer has been down on a hold control for its
  time, the runtime sends the control's press (04 V18).
- Notes: the runtime keeps the queue of 04 §10. A note comes up when the one before it has been
  up for `TOAST_SHOW_MS` by the runtime's clock, and the change draws a frame.
- The motion store keeps a page's own region offset per page `Anchor` for the last 32 pages
  (04 K11). Held by a runtime test: after Back, the page lands where the person was.

## 3. Effect dispatch

The runtime takes a returned list with `Effects::into_list` (its only caller), which gives the
`Fx` in the order they were made, and walks each in order:

| `Fx` | What the runtime does |
|---|---|
| `Order(o)` | appended to the effects for the root (P18); `CameraOff` also ends the running stream (§5.5) |
| `Ask(a)` | `into_parts` (A16); a new `AskId`; the reply kept under it; `Effect::Ask` appended |
| `Change(c)` | a new `ChangeId`; the reply kept; `Owner::given(id, change)` pushed to that owner |
| `Done(id, done)` | the kept reply under `id` called; the asker's message pushed |
| `Job(j)` | `into_parts` (its only caller), to the pool (§5) |
| `Stop(kind)` | to the pool: that part's jobs of that kind stopped (§5.4) |
| `Then(m)` | pushed, to the same top state |
| `Out(top)` | handled as §1.1 says |
| `Note(n)` | kept in the note queue (04 §10) |
| `View(order)` | kept until a frame draws its anchor (§2.5 step 5) |

### 3.1 Asks and their answers

`AskId { generation, n }`, `ChangeId` and `JobId` carry a generation. The generation counts the
builds of the lock scope in this process, from 1; whatever the part outside the scope asks for,
changes or starts carries generation 0. The reply table maps each id to its reply and its top
state.

- **Rule R11.** Every ask is answered once (section 02, §2.1); on `Event::Answer` the runtime
  removes the reply and calls it. An answer whose id is not in the table (its scope was dropped,
  or it was answered already) is dropped, and its payload wipes as it drops (P9). An answer of the
  wrong kind reaches the feature as the ask's fault and is noted. Held by a test of the runtime:
  an answer of a dropped generation changes nothing, and its `Bytes` are wiped (a test
  allocator).
- **Rule R26.** The part of the model outside the lock scope (the Starter's settings owner and its
  own composite) has generation 0: what it asks for, changes or starts is never stale at a lock,
  never dropped by `Exit::Reset` and never stopped by the scope's end. The runtime lends it the
  empty session and vaults readers (A19) whether or not the scope is built, so no feature of the
  Starter's own reads Faraday Demo's session or vaults. Held by the runtime's dispatch and a
  runtime test over a test root with one scope and one outside part (06 S12).

### 3.2 Changes, served beside asks

A change is served inside the event that made it: the owner's `given` message is pushed on the
same queue, and its `Done`, when the owner answers in the same dispatch, is handled in the same
event. An owner that answers after a job of its own keeps the `ChangeId` in its state and returns
`Done` when the job's answer comes; the runtime keeps the reply until then.

A feature that wants a shared owner to act (a press that begins an owner's page with something
given, such as the file to unlock, 06 §4.3) sends that owner a change through `Fx::Change`. The
runtime serves it as any other change; there is no `Out` for it, and no top state routes another's
message.

- **Rule R12.** A change answer whose id is not in the table is dropped and noted, as R11 drops an
  ask's (A23). Held by the test of R11.
- **Rule R13.** When the runtime dispatches to an owner, the `View` it lends has the empty reader
  in that owner's own slot (`session::Read` and `vaults::Read` for `()`, A19), and for the
  settings owner a copy of the settings as they were before the message. An owner reads itself
  from its own state. Reason: the owner is borrowed `&mut` while the others are lent `&`; Rust
  cannot lend it both ways. Turned down: a `View` per owner type, which makes every feature's
  signature depend on who calls it. Held by the borrow checker and the runtime's dispatch.

### 3.3 Effects of a scope that is locking or ending

Once a lock or an end of the scope has begun (§7), the scope's orders and asks are still carried
until the scope is dropped. What is delivered to a top state meanwhile is R16's rule (§5.4): what
serves `Close`, and every answer of generation 0. A new job of the scope that is not a seal is
dropped at once, unstarted.

## 4. View and Ctx

### 4.1 How the runtime builds them

For each dispatch and each frame the runtime builds `View` from shared borrows of these places:
the three owners' states (but the one being dispatched to, R13; the empty session and vaults
readers for the part outside the scope, R26), the `Facts` it keeps, the list of running jobs
(`View::running`), and the scope's `Mode`, `Clock` and `View::started` (§7.5). `Ctx` is that
`View` and `&self.compute` (R10).

**Decision: `View` holds no strings. The runtime holds the wording as `&'static Strings` and
gives it only to `present`, in `Show` (04 §3), which it builds for each frame and passes to
`R::frame`.** `Ctx` is `View` plus `Compute`. Reason: an update given the wording could form text,
and A30 would hold only by review. Turned down: strings in `View`, as section 03 first had them.

### 4.2 Facts are the process's

`Facts` belong to the process, not the scope: the runtime keeps them across an in-process rebuild
(§7.6) and lends the same `Facts` to the part outside the scope and to every build of the scope.
They hold no secret. The runtime also keeps every field of the process's `Start` but `carried`
(display, medium, abilities, machine, scheme, the read-only root), updated by later facts, to
build the scope's start in process (§7.5).

The self-test runs once per process, on the root's thread with the runtime's token, as the runtime
handles `Start` and before the first state is built, so no press precedes it; its outcome is
`Facts::selftest`, and `Top::SelfTest` runs it again.

### 4.3 The clean marker

- **Rule R14.** This process is clean while it has never held or typed a secret and the camera is
  off: after each event the runtime reads `holds()` of every top state, sets a latch (`tainted`)
  once `kept` or `typed` is true, and works out `clean = !tainted && no stream runs && !(kept ||
  typed)`. It sends `Order::Clean` when the value changes, and gives it to features as
  `Facts::clean`. The latch is the process's: it stays set across an in-process rebuild, so a
  rebuilt scope is never clean. Held by a test of the runtime (typed text, then `at_lock`, stays
  not clean) and the stick adapter's hold on sticks (section 02, §8).

## 5. Workers

### 5.1 The pool

**Decision: the pool has two lanes, each one thread, each running its jobs one at a time in the
order given.** The heavy lane runs the jobs that ask Argon2id for memory (a vault's stretch, its
seal, the unlock measure); the light lane runs every other job. Reason: two Argon2id memories are
never asked for at once (on the Pi a vault at Strong strength is most of its memory, the reason
`main` has one worker), and a PSBT read or a signing does not wait behind a quiet measure or a
seal. Turned down: one thread, as on `main`; and a thread per core, which could run two stretches
at once.

```rust
impl<M: Message> Job<M> {
    /// The heavy lane. Only the shared owners' crates call it (A35's list).
    pub fn heavy(self) -> Self;
    /// A seal: it runs to its end at a lock (§7.2). Only the vault owner's crate calls it.
    pub fn seal(self) -> Self;
    /// The runtime's only: the job's parts, for the pool.
    pub fn into_parts(self) -> JobParts<M>;
}
/// A job opened by the runtime: its kind, its lane and flags, and the work to run.
pub struct JobParts<M> {
    pub kind: JobKind, pub quiet: bool, pub heavy: bool, pub seal: bool,
    pub run: Box<dyn FnOnce(&Compute) -> M + Send>,
}
```

- Lane threads start with the runtime and live for the process; each mints its own `Compute`
  (R10).
- The runtime holds each lane's queue on its own thread and hands a lane one job when the lane is
  free, so a job not started is always the runtime's to drop.
- The order of jobs is kept within a lane only. A feature that needs two jobs in order starts the
  second on the first's answer.
- `Workers::Inline` (the tour root, runtime tests) runs each job on the root's thread after the
  cascade that started it, with the root's token, its answer handled as the next event, so a tour
  renders the same every time.
- A job that panics ends the process (release builds abort); no answer stands in for it.

### 5.2 Compute on a worker

`Compute` gains two hooks beside `count`, function pointers as `count` is, so `faraday-crypto`
stays `no_std` and the token stays `!Send`:

```rust
// core/faraday-crypto/src/work.rs
pub struct Hooks { pub count: fn(Work), pub stop: fn() -> bool, pub progress: fn(u32, u32) }
impl Compute {
    pub fn new(hooks: Hooks) -> Compute;
    pub fn stop_asked(&self) -> bool;          // the job was asked to stop
    pub fn progress(&self, done: u32, of: u32);
}
```

A lane thread sets its current job's signal (a stop flag and two progress counters, atomics shared
with the runtime) in a thread-local before it runs the job; the hooks read and write it. On the
root's thread `stop` is false and `progress` does nothing. Costly work checks `stop_asked` only
where it already counts pieces (a PSBT's inputs, a stick's zeroing, a seal's records); an
Argon2id pass is one piece and runs to its end.

### 5.3 Answers and progress

A job's answer comes back on a channel with its `JobId { generation, n }`, and the lane calls the
waker. `woken` takes every answer that came and handles each as one event (§2.1). Progress is read
from the signals into `View::running` at each frame; the lane wakes the root at most ten times a
second while a job's progress changes.

- **Rule R15.** A job's answer whose generation is neither 0 nor the built scope's is dropped, and
  its payload wipes as it drops. Within a scope, a stale answer is the feature's to drop by its
  ticket (A25). Held by the test of R11 and the features' update tests.

### 5.4 Stopping

`Fx::Stop(kind)` (A27): a queued job of that kind of that part is dropped, its input wiped; a
running one has its stop flag set, and its answer, when it returns, is handled as any answer.

- **Rule R16.** No job's input outlives the scope that made it (A28). When the scope ends, by a
  lock that ends the process or one that does not, by an end or by the Starter's Close, the
  runtime drops every queued job of it (at a lock, but a seal), sets the stop flag of every running
  one (at a lock, but a seal's), and waits until each has returned. While the scope is `Locking` or `Ending`, one rule
  says what is delivered: what serves `Close` (the vault owner's asks, changes and seal jobs,
  their answers and `Done`s, and `Close`'s own `Done`, which the runtime reads), and every answer
  of generation 0. Every other answer and job answer of the scope is dropped unread and wipes as it
  drops, so its inputs (a signing job's keys behind the key store's `Arc`, a typed passphrase, a
  MuSig2 round) are gone before the next scope is built, and nothing handled after the hide puts a
  secret back on the screen. This replaces `main`'s rebuild, in which the old app's worker thread
  ran its job to the end beside the new app (`faraday/faraday-core/src/worker.rs`, inventory
  `features/lock.md`). Held by a test of the runtime with a job that blocks until stopped: no
  `start` of the next scope runs before the job has returned, and the job's input is wiped (a test
  allocator).

### 5.5 Stream facts

The runtime knows the running camera stream: the `AskId` of the `camera::On` answered
`Started`. It ends at an `Order::CameraOff` that passes through dispatch, at
`CameraFact::Ended`, at the start of a lock or an end, and when the scope that asked for it drops.

- **Rule R17.** A `CameraFact` whose `stream` is not the running stream is dropped before
  `Root::fact` sees it, and its bytes wipe as they drop (P21). At the start of a lock the runtime
  itself appends `Order::CameraOff` if a stream runs. Held by a test of the runtime: a frame after
  `CameraOff` reaches no feature.

## 6. The memo

The memo is 04 §6's: `Memo` is in the kernel's `cx`, reached through `Show`, and the runtime owns
the one instance. 04 §6 gives its API (`Memo::get(slot, key, input, fill)`, `I: Public`); this
section gives the store.

- Entries are keyed by (slot, the value's type, key), and kept while the last frame used them: an
  entry no frame used is dropped at that frame's end.
- The write into the store on a miss cannot be seen by the caller, which is why `&Memo` may hold
  a `RefCell`.
- The debug refill check (V13) runs inside `get`: under `cfg(debug_assertions)`, a hit fills the
  value again from `input` and asserts it equals the kept one, so `get` asks `V: PartialEq` there.
- Nothing the memo reaches holds `Compute` (V2, V14): `fill` is given `input` alone.
- The store is emptied at every lock and every end.
- Three calls are the runtime's only: `Memo::new()` (the one store, empty; there is no `Default`),
  `Memo::end_frame(&self)` (at a frame's end: the entries that frame did not use are dropped) and
  `Memo::clear(&self)` (at every lock and every end: every entry dropped).
- Rule R18 is not used: the memo's entries are `Public` and owned by R5.

## 7. Lock, end and restart

### 7.1 What a lock is for

A lock drops the lock scope and builds it again: on the device the process ends and the kernel
zeroes its pages; in Faraday Demo, in the Starter's Faraday Demo, and after a failed keep, the
next scope is built in the same process. Before either, every secret on show leaves the screen,
the vaults are sealed, each feature puts what the next scope needs into the carry, and every state
of the scope wipes what a lock wipes.

### 7.2 The order of a lock

On `Top::Lock(lock)` from a state of the scope, the phase becomes `Locking`, and:

1. **Hide.** `Notice::Hide` and `AtLock::hide` to every top state (§2.3); `Order::CameraOff` if a
   stream runs (R17); every `Input` is dropped from now on. The frame after this event is painted
   whole, and the motion state of every component is dropped, so no pixel and no fade image of the
   screen before the hide stays in the runtime's frame.
2. **Stop.** Every queued job of the scope but a seal is dropped; every running one but a seal is
   asked to stop (R16). A seal of one vault already running (a passphrase change, a vault made in
   a flow) runs to its end, so its vault is sealed first and the lock then goes on (inventory
   `features/lock.md`).
3. **Seal.** The runtime gives the vault owner the change `vaults::Close { seal: lock.seal }`,
   which only the runtime gives (03 §7.3), under a reply entry of its own: the runtime reads its
   `Done` and never routes it. The owner seals every changed open vault (one at a time, on the
   heavy lane, into the Outbox through the session's change), closes every open vault, and
   answers once with what it sealed and what failed (inventory `features/vault-seal.md`). A failed
   seal is reported to the next scope through the vault owner's carry; the lock goes on.
4. **Wait.** Until `Close`'s `Done` is read and every job of the scope has returned. Only what
   R16 lets through is delivered meanwhile.
5. **Carry.** `Feature::carry` on the scope's top states and, always, on the settings owner, in the
   fixed order (§7.4).
6. **Wipe.** `AtLock::at_lock` on the scope's top states; in debug builds the runtime asserts
   every `holds()` is all false (A12).
7. **Drop.** Every waiting reply of the scope is dropped (their contexts are `Public`); the memo
   is emptied; the log, the note queue and the `Fx::View` orders waiting are dropped. The frame's
   output is dropped whole: the call list, the hits, the laid facts and the last laid fact kept
   per `Anchor`, the pointer and focus state, the motion state and the kept images; the frame
   buffer is zeroed. The scope's states are dropped.
8. **Keep or not.** Where the lock ends the process (`Abilities::lock_ends_process`, §7.6),
   `Ask::Keep(carry)` is asked and its answer awaited; the runtime holds the carry, and
   `only_copy` aside, until then.
9. **Exit.** `Order::Exit(Exit::Lock)` (the process ends) or, in process,
   `Order::Exit(Exit::Reset(generation))` and the next build (§7.6).

**Decision: the stop comes before the seal, and the seals are the jobs that survive it.** Reason:
a seal is the one job whose result the person would otherwise lose, and every other job's result
belongs to a screen the lock ends. Turned down: waiting for every running job to finish, as
`main` does, which keeps a signing job's copy of the keys live for its whole run.

- **Rule R19.** The steps above run in this order at every lock, by every way a lock is asked
  (Lock, the Lock sheet, the idle lock, a mode change). Held by a test of the runtime over a test
  root whose features record the calls they get.
- **Rule R20.** The frame buffer is cleared before the scope is dropped (P13): the first frame
  after the hide is painted whole, and at step 7 the buffer is zeroed and the call list (which
  holds the text drawn) dropped. The adapters' copies are theirs (§7.6). Held by the buffer's type
  (`Secret<Vec<u8>>`) and a test of the runtime that reads the buffer after a lock.

### 7.3 `Carried`

```rust
/// The kernel's (`lock` module): what one build of the scope leaves the next, by name.
pub struct Carried { inbox: Files, outbox: Files, entries: Vec<(CarryName, Bytes)>, only_copy: bool }

/// "<feature>.<entry>", a constant of the feature's crate. Compared by `name` alone.
#[derive(Public, Copy)]
pub struct CarryName { name: &'static str, first: bool, any_mode: bool }
impl PartialEq for CarryName { /* self.name == other.name */ }

/// A carried value: Public, so no secret crosses a lock in an entry.
pub trait CarryValue: Public + Sized {
    fn encode(&self) -> Bytes;
    fn decode(bytes: &[u8]) -> Option<Self>;
}

impl Carried {
    pub fn put<V: CarryValue>(&mut self, name: CarryName, value: &V);
    pub fn take<V: CarryValue>(&mut self, name: CarryName) -> Option<V>;   // once; then None
    /// The session's crate only: a held file, its bytes moved, never copied.
    pub fn put_file(&mut self, to: FileBox, name: Text, bytes: Bytes);
    pub fn take_files(&mut self, from: FileBox) -> Files;
    /// Something carried has no other copy (§7.6). The session's crate only.
    pub fn mark_only_copy(&mut self);
}
```

- **Rule R21.** An entry is the encoding of a `Public` value; only the Inbox and Outbox carry
  bytes that may be secret, and only the session's crate puts them. Held by the compiler
  (`V: CarryValue`, so `Public`) and `clippy.toml`'s `disallowed-methods` on `put_file` and
  `mark_only_copy`, allowed in the session's crate.
- **Rule R22.** A start that cannot read an entry (`decode` gives `None`) starts as if there were
  none, and every entry no start took is dropped, its bytes wiped, after the last start. A renamed
  or removed feature therefore loses its entry and never misreads one. Held by an update test of
  each feature that carries.
- A lock that changes the mode carries only entries marked `any_mode`.

`Carried` maps onto the port's `Carry` (02 §4.2) at a Keep and at `Start`:

- The Inbox and Outbox are `Carry::inbox` and `Carry::outbox`.
- Each entry is a file of `Carry::kept`, named by `CarryName::name`. Entries marked `first` come
  first, so that what matters most is written first (`main`'s `backup-gate` and `backup-flow`).
- `only_copy` is not in `Carry`: the runtime holds it aside for the whole Keep, and reads it again
  when `Kept(Err)` hands the carry back (§7.6).
- At `Start`, each file of `kept` becomes an entry by its name; `take` finds it by name alone, so a
  name rebuilt from a file name matches.

### 7.4 What each feature carries

`carry` is called on the settings owner, then the session, the vaults and the app composite, which
calls each child's in the order it holds them. Its default carries nothing. A step flow carries
through `StepFlow::carry`, which the kernel's blanket impl forwards (03 §9.1).

| Entry | Put by | Holds | Taken at start by |
|---|---|---|---|
| Inbox files | the session | the files of kinds that cannot hold a secret; the others are dropped at the lock (inventory `features/files-inbox.md`) | the session |
| Outbox files and their secret marks | the session | every file waiting to go to a stick, vaults sealed at this lock among them; a plain secret the person chose to take away stays, and the next scope names it (inventory `features/lock.md`) | the session |
| held keys | the session | fingerprints and digests of keys held until their backup (`features/lock.md`, "Held keys") | the session |
| vault summaries | the vault owner | seal counts, what was sealed or failed, a memory step not raised (`features/vault-seal.md`) | the vault owner; the lock feature's Locked sheet reads them through `vaults::Read` |
| settings, `any_mode` | the settings owner | the person's choices, the clock's offset and whether it was checked (`features/settings-time.md`) | the settings owner |
| mode, `any_mode` | the settings owner | the mode asked for, which takes effect at this lock | `Root::mode` (§7.5) |
| the backup flow, `first` | the backup flow | its plan, its stage, which keys it holds, by fingerprint; no key material (`features/backup-flow.md`) | the backup flow |
| write waiting | the stick visit | a "Lock and write": names and hashes, no bytes (`features/files-inbox.md`) | the stick visit |
| input counts, `any_mode` | the devices feature | kept out, beside presses, wrong letters (`features/devices-inputs.md`) | the devices feature |
| what was locked | the lock feature | idle or asked, the names for the Locked sheet | the lock feature |

A feature not in this table carries nothing: what it holds goes with the scope. A feature that
carries has its row here.

The session sets `only_copy` in its `carry` whenever a key is held for its backup or the Outbox
holds a sealed vault, whether sealed at this lock or carried from an earlier one. Held by an
update test of the session's `carry`, with both cases.

In the Starter the settings owner stands outside the scope and is not built again: its mode entry
is read by `Root::mode`, and its other entries are taken by no start and dropped (R22).

### 7.5 Building the scope

On `Start` or in process (§7.6), the runtime:

1. takes the carry: `Start::carried` from the adapter, or the carry it holds;
2. calls `Root::mode`, which takes the mode entry or gives the root's own (the device: Mainnet
   when none or unreadable; Faraday Demo and the Starter's Faraday Demo: Testnet, Practice,
   online, section 03 §11);
3. starts the scope's states in the fixed order (settings, session, vaults, app). Each `start` is
   given the carry and a `View` lending the owners started before it and the empty readers (and
   the default settings) for the rest;
4. drops every entry not taken (R22) and handles the starts' effects as one event (R7).

At `Start`, the part outside the scope is built first, with generation 0: in the Starter the
settings owner and `Outside`, after which the phase is `Closed` until `Top::Open` (§7.9). Each
build of the scope takes the next generation.

`View::started` tells features how this build began: `Power` (no carry), `Lock`, or `Rebuilt`
(§7.6), which the lock feature's Locked sheet words.

### 7.6 Where the carry waits, and the rebuild in process

**Decision: the runtime asks `Ask::Keep` only when the lock ends the process; a scope built in
the same process gets the carry from the runtime's hand, moved.** On the device the carry waits in
the stick adapter's RAM directory while no process runs, and the next process's `Start` brings it
(section 02, §4.2). In Faraday Demo and in the Starter's Faraday Demo the carry never leaves the
runtime: it is moved from step 5 of §7.2 into the next build's start. Reason: no adapter holds a
copy of the Inbox when none is needed. Turned down: `Keep` at every lock, which hands the Inbox's
bytes to an adapter only to have them handed back. `Abilities::lock_ends_process` (true on the
device) says which.

**Decision: when the keep fails, the process ends unless the carry holds an only copy; then the
scope is rebuilt in the same process.** A failed Keep has removed every file it wrote before it
answers (02 §4.2). `Answer::Kept(Err((failure, carry)))` hands the carry back. If `only_copy` is
set (a key waiting for its backup, or a sealed vault, sealed at this lock or carried from an
earlier one, whose only copy is in the Outbox), the runtime builds the next scope in process with that carry, marked `Rebuilt` and not
clean (R14); otherwise it drops the carry and sends `Exit::Lock`. Reason: the restart at every
lock, with the kernel zeroing the process's pages, is what the device relies on, so it is given up
only where it would lose a key or a sealed vault; a rebuilt scope gives no stick out until a lock
that restarts (R14). Turned down: a rebuild at every failed keep, which keeps a process that cannot
write a stick when nothing needs it; and ending the process always, which loses the only copy of a
new key.

An answer to `Keep` that is not `Answer::Kept` (02 §4.2, P9) is a failed keep whose carry is empty:
what was carried is lost. `only_copy`, held aside for the whole Keep, still decides: if it is set,
the scope is rebuilt as `Rebuilt`, not clean, and the runtime puts up the note `KeepLost`, a typed
value worded from the strings crate (04 §10); otherwise the runtime sends `Exit::Lock`. Nothing is
retried. A runtime test over a test root holds it.

`Start` is the first event of every process, exactly once. A scope built in process (Faraday
Demo's lock, the Starter's Faraday Demo opened or locked, a failed keep) gets its start from the
runtime's kept facts (§4.2) and the carry; no adapter sends a second `Start`.
`Order::Exit(Exit::Reset(generation))` tells the adapters that the scope of that generation was
dropped in this process. It is sent at every in-process lock, at a failed keep's rebuild and at
the Starter's Close (§7.9). On it each adapter drops what belonged to that generation, drops that
generation's asks it has not begun, unanswered, and sends its state facts again:

| Adapter | Resets on `Exit::Reset(generation)` |
|---|---|
| display (stick framebuffer, window) | the panel blanked; every copy of the picture wiped (P13); the arrow, the cursor and the notice cleared; `Display` sent again |
| input (device) | every held device back to waiting, every believed flag false (the counts come in the carry); `BootUsb` and `Trust` sent again |
| camera | a stream of that generation stopped and its buffers wiped (P28); the camera list sent again |
| storage (device) | every `StorageAsk` of that generation not begun dropped, unanswered; the RAM directory's carry files removed; a new session with the disk process; sticks scanned again and `Attached`, `Listed`, `BootMedium` sent again |
| storage and host (desktop) | every ask of that generation not begun dropped, unanswered (a folder stick's write, `HostAsk::Save`, `Ask::Print`); the sticks folder's pictures, toggles and served lists reset; the folder sticks attached stay; sticks listed again; `Downloads` sent again |
| clean marker | removed, and not written again in this process |

**Decision: a write the dropped scope asked for is not carried out, unless the adapter had begun
it.** At `Exit::Reset` every adapter drops the asks of that generation it has not begun. One
begun ends whole or not at all (P11), its bytes are wiped when it ends, and its answer is dropped
by its generation (R11). Reason: an ask belongs to the scope that made it, and a file counts as
written only on the `Wrote` its asker sees (A34), which no later scope can see; a write cut
half-way would break P11. Turned down: serving the old queue after the rebuild, which writes to a
stick for a scope that no longer exists and can never count the file. A "Lock and write" is not
lost by this: it crosses as the write-waiting entry, and the next scope asks for the write itself.
This settles the inventory's doubt on the stick shell's disk queue (`features/lock.md`, Doubts).
On the device's ordinary lock the process ends; the stick adapter does the same with the disk
process before it exits.

- **Rule R23.** A lock that does not end the process, and the Starter's Close, stop the scope's
  running jobs and drop their inputs (R16), drop the scope, and send `Exit::Reset` before any
  effect of the next scope is carried (P18's order). Nothing the dropped scope held stays reachable
  from the next one or from an adapter: an ask of its generation not begun is dropped, and a write
  begun has its bytes wiped when it ends and counts for no scope. Held by R16's test, runtime tests
  over a test root (a keep that fails while a key waits for its backup; `Exit::Reset` before the
  next scope's first effect), and the stick adapter's test of its reset.

### 7.7 Ending

On `Top::End(end)` the phase becomes `Ending`. A `Top::End(Quit)` from inside the scope in a root
that has an `Outside` closes the scope instead (§7.9). Otherwise:

1. hide, as §7.2 step 1;
2. every job stopped, a seal included, and waited for (R16): an end seals nothing and keeps
   nothing, and the Power sheet has asked first where something would be lost (inventory
   `features/lock.md`);
3. `at_lock` on every top state; the states, the memo, the log and the frame's output dropped as
   §7.2 step 7;
4. where `lock_ends_process` (the device), `Ask::Forget`, which removes every carried file in the
   adapter's keeping, awaited; elsewhere no carry left the runtime, and none is asked;
5. `Order::Exit(Exit::End)` for `PowerOff` and `Quit`, `Order::Exit(Exit::RestartComputer)` for
   `RestartComputer`.

`RestartComputer` follows this order too, with no seal.

### 7.8 Faraday Demo

Faraday Demo's lock does not end the process: §7.2 runs, step 8 is skipped (`lock_ends_process`
is false), the carry passes from hand to hand, and the scope is rebuilt after
`Exit::Reset(generation)`. The mode is the carried one where the person chose a mode, else the
root's (Testnet, Practice, online), so the samples come again. The desktop adapter keeps what is
its own across the rebuild: the folder sticks attached, the preview size, the window.

### 7.9 The Starter's Faraday Demo

**Decision: the Starter has one model, and Faraday Demo is its lock scope, beside the Starter's
own composite.** The scope is the session and vaults owners and the online app (the frame and the
online composite), held by the runtime as top states that exist only while the scope is built.
The Starter's own composite is `Root::Outside`, and the settings owner stands outside the scope
with it (06 §4.1). The runtime knows the part a message is for from its `Addr` variant.

- `Top::Open` from the Starter's composite, after `SystemAsk::PrepareDemoSticks` is answered
  `Ok`, builds the scope with a start the runtime makes from its facts (§7.5), at the next
  generation.
- A lock inside the Demo runs §7.2 over the scope alone and builds it again, as in Faraday Demo
  (§7.8). The settings owner's `carry` is called too, so the mode a person picks inside the Demo
  is carried.
- The Demo's Quit, and the Starter's Back over it, reach the runtime as `Top::End(Quit)` from
  inside the scope: a Close. It follows the end's order (§7.7) over the scope, with no seal and no
  `Ask::Forget` (the carry never left the runtime), then sends `Exit::Reset(generation)`, the
  phase becomes `Closed`, and the runtime sends `Notice::Left` to `Outside` (§2.3), whose
  navigation drops the Demo's page.
- `CloseAsked` and a `Top::End(Quit)` from the Starter's composite end the process as §7.7 does,
  over the scope first if it is built.
- The part outside the scope is never wiped, dropped or rebuilt by these, and its asks never go
  stale (R26).

Reason: once the runtime holds the owners, the Demo's session and vaults can only be its top
states, and the Demo needs real owners, a generation per build so its stale answers drop, and the
lock of §7.2, all of which are the runtime's. With the Demo as the scope, the part outside it is
lent the empty session and vaults by construction (R26); the Demo's frame records live in the
scope and are built again with it; and the Starter names only its page of the Demo, never a type
of Faraday Demo's product. One model keeps one settings owner, one id space, one pool and one
frame, and no edge between roots. Turned down: the Demo as a child state of the Starter's
composite, which would need the lock's whole order rebuilt inside a composite and would lend the
Starter's own features the Demo's owners; and a second model or a second runtime in the Starter,
which would mint ids the adapters cannot tell apart and compose two frames.

## 8. The log

**Decision: the runtime keeps a log only when the root turns it on, holds it in memory, and writes
it nowhere.** A line is the time, the generation, `Event::kind()` or `Effect::kind()` for port
traffic (P15), and for each dispatch `Message::kind()` with `Message::public()` where the variant
is public; a withheld variant logs its kind alone. The log is a ring of 512 lines, dropped at every
lock and end. The roots turn it on for the dev card's verbose run, the tour root and tests
(inventory `features/dev-diagnostics.md`); a root may print it there.

- **Rule R24.** Only kinds and `Public` messages are logged. Held by the compiler: the log takes
  `&dyn Message` and reads only `kind` and `public` (section 03, A14).

**Decision: there is no replay of a session.** A session's answers carry files, frames and typed
text that are not `Public`, so a replay of its public part does not reproduce it, and a replay of
the rest would keep a copy of secrets. Tours drive the tour root through the port from their own
scripts (section 01, R4). Turned down: recording sessions to replay them in tests.

## 9. What a check reads from the runtime

The tour root's screens check and the leak flows (section 07, §5.3 and §6.2) read the runtime's
ordinary output (R4); the runtime has no hook for them.

```rust
impl<R: Root> Runtime<R> {
    /// What the last frame shows and keeps, and what the last event did.
    pub fn seen(&self) -> Seen<'_>;
    /// Turn the ink record on (04 V21). The tour root's, the leak crate's and test support's
    /// only (clippy `disallowed-methods`).
    pub fn record_ink(&mut self, on: bool);
}
pub struct Seen<'a> {
    pub ink: Option<&'a Ink>,        // the last frame's `FrameOut::ink`; None unless the record is on
    pub kept: &'a [KeptImage<'a>],   // every image the runtime keeps: a sheet's frost, a fade's frame
    pub page: Anchor,                // `FrameOut::page`, set by the screen template
    pub sheets: &'a [Anchor],        // `FrameOut::sheets`, the lowest first
    pub marks: &'a [Mark],           // for the last event
}
pub enum Mark { LockBegun, Hidden, Hides, ModelDropped, ModelBuilt }
pub struct KeptImage<'a> { pub id: ComponentId, pub taken_at: u64, pub secret: bool, pub pixels: &'a [u8] }
```

- The ink record keeps every string the frame drew with its box, secret text too, marked secret
  (04 §11), so the screens check reads both. It is a wiping list, replaced at the next frame and
  dropped at a lock.
- The kept images are the runtime's alone (04 K16), in wiping buffers; `taken_at` is the number
  of the frame they were copied from, so the check knows which frame's text they showed. By V19
  only a step fade's source has `secret` set, and only while its fade out runs.
- `Mark::Hides` is recorded when the event's press was a hiding press (`Pressed::hides`, from the
  `#[hides]` mark on the press type's variant, 04 V24). A lock and a hide are marks too.
  `Notice::Left` is sent inside the app by its navigation and the runtime does not see it (but the
  one it sends at the Starter's Close, §2.3); leaving a page or a sheet is a change of `page` or `sheets` between two frames, which the check reads
  instead.
- A leak flow runs with `Workers::Threads`. After a lock that ends the process, `phase()` is
  `Ended` once the scope is dropped and every job has returned (R16), and the scan runs then, over
  the root's thread and the lane threads, which live on but hold nothing of the dropped scope.
  After a lock in Faraday Demo or the Starter, the scan runs again once `ModelBuilt` is marked;
  after the Starter's Close, once `ModelDropped` is.
- **Rule R25.** `seen` is read-only and present in every build. Held by its signature (`&self`)
  and R4.

## 10. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| R6 | only the runtime exits, marks clean, keeps and forgets; a lock reaches it as `Top::Lock` | `lint-layers`' module rule |
| R7, R8 | an event settles first in first out before the next; the cascade is bounded, and a dropped change is answered with its fault | a runtime test |
| R9 | a press is hit-tested against the frame seen, then gated by `refuses` | a runtime test |
| R10 | `Compute` minted at two sites | `costs-lint`; `clippy.toml` |
| R11, R12, R15 | stale and wrong answers of asks, changes and jobs dropped and wiped | a runtime test with a test allocator |
| R13 | an owner's own slot in its `View` is empty | the borrow checker; the runtime |
| R14 | the clean latch is the process's | a runtime test; the stick adapter |
| R16 | no job's input outlives its scope; what a locking or ending scope is delivered | a runtime test with a job that blocks until stopped |
| R17 | facts of a stopped stream dropped | a runtime test |
| R18 | not used (R5 holds the memo's entries) | |
| R19, R20 | the lock's order; the frame cleared before the scope drops | a runtime test over a recording root; the buffer type |
| R21, R22 | carried entries are `Public` but the held files; an unread entry is dropped | the compiler; `clippy.toml`; update tests |
| R23 | an in-process rebuild or a Close leaves nothing of the old scope reachable | R16's test; runtime tests; a stick adapter test |
| R24 | only kinds and `Public` messages are logged | the compiler |
| R25 | a check reads the frame, its kept images and the marks, never state | the signature; R4 |
| R26 | the part outside the scope has generation 0 and is lent empty readers | the runtime's dispatch; a runtime test with one scope and one outside part |

## Questions for later sections

- Section 07: the runtime tests named here (a test root that records calls, a job that blocks
  until stopped, a test allocator that sees wiped bytes, a test root with one scope and one outside
  part) and where they live.
- Section 09: the runtime crate's name and the two mint sites `costs-lint` lists.
