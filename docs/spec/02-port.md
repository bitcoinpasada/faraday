# The port: events in, effects out, storage and camera behind it

**Kind:** Spec
**Open:** Not built.

This section gives the one contract between the app and every shell (section 01, P1 to P4): the
port crate grown from `main`'s `faraday-shell-api`. It fixes every event the shells send, every
effect the app asks for, the frame out, and their Rust types; how an answer finds its way back to
the request that asked for it; which values wipe and which may be logged; the order things happen
in and what a failure looks like; and how storage and the camera sit behind the port, from
`main`'s `faraday-storage`, the stick shell, the desktop shells and the camera crates. It ends with
what each product's adapters carry and a table placing every effect `main` makes. It is written for
whoever writes the port crate, the runtime's dispatch (section 05), an adapter or a root
(section 06), and for anyone who adds an effect. Rules continue section 01's numbering for the
port: P5 onward.

## 1. The crate and the loop

- **Rule P5.** The port is one crate (`layer = "port"`, `no_std` with `alloc`, rule D2). It
  depends on `faraday-text` (names, `SecretText`), `faraday-crypto` (`Secret`) and `zeroize`, and
  nothing else. Held by `lint-layers` and the attribute.
- **Rule P6.** The port holds types and two small traits, no behaviour: no I/O, no thread, no
  clock, no wording. Every enum the app reads is plain data the app words. Held by D1's
  `clippy.toml`, applied to the port's directory, and by `lint-strings` (no user-facing literal).

Its modules:

| Module | Holds |
|---|---|
| `event` | `Event`, `Start`, `Abilities`, `Machine`, `Scheme`, `Time`, `Input`, `Key` and every unasked fact (§3) |
| `ask` | `Ask`, `Answer`, `AskId`, the `Request` trait, every ask's own type, `Failure`, `OsError` (§2, §4, §7.2) |
| `order` | `Order`: effects with no answer (§4.9) |
| `frame` | `Frame`, `Rect`, `Moved`, `Region` (§5) |
| `values` | the display's and the machine's facts the kit may name (P4): `Display`, `Preview`, `Orientation`, `Point`, `Medium`, `Anchor`, and `Pace` (§5), which the kit's frame output gives and the frame carries |
| `bytes` | `Bytes`, `EntropyBytes` and the rule of what wipes (§6) |
| `storage`, `host`, `camera`, `devices`, `system` | each family's value types (§4): `storage` also holds `StickId`, `StickInfo` and `Sha256`; `camera` holds `CameraId`; `devices` holds `UsbDevice`, `InputId` and `Edid` |

`Anchor` is the name the app gives a place it draws; the kernel's view orders and the kit's motion
store key by it (section 04, K10):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Anchor { pub scope: &'static str, pub n: u32 }
```

The loop, as the root runs it (section 06 gives each root):

```rust
/// The root's side: every adapter of a product behind one value.
pub trait Adapters {
    /// Carry out one effect. Never waits on a stick, a child process or a
    /// camera: slow work runs on the adapter's own thread and its answer
    /// comes back through `poll`.
    fn carry(&mut self, effect: Effect);
    /// The next event, or `None` when none waits.
    fn poll(&mut self) -> Option<Event>;
}

/// Given by the root to the runtime and the adapters: wakes the root's
/// loop when a worker or an adapter thread has something.
pub trait Waker: Send + Sync {
    fn wake(&self);
}

pub enum Effect {
    Ask(AskId, Ask),
    Order(Order),
}
```

The root takes events from `poll`, hands each to the runtime, hands every effect the runtime
returns to `carry`, and paints the runtime's frame (§5) through its display adapter. The paint
and the end status are the display adapter's own methods, not `Adapters`': the device's
`Stick::present(&Frame)` and `Stick::ended() -> Option<u8>` (section 06, §2.1), a window's
`Window::paint(&Frame)` (section 09, §6). The runtime is the only caller of `carry` with an
`Ask`: it mints the `AskId` (§2.2).

## 2. Asks, answers and the worker job

### 2.1 Three kinds of traffic

- An **ask** is an effect that is answered: exactly one `Event::Answer` with the same `AskId`.
  Reading a file, writing one, starting the camera, asking for randomness, saving the carry.
- An **order** is an effect with no answer, because nothing the app shows depends on how it went,
  or because a fact event follows it (§4.9): the camera off, the arrow aimed at a held device,
  the clean marker, a turn of the screen, exit.
- A **fact** is an event nobody asked for: input, time, sticks coming and going, the display
  changing, camera frames of a running stream, the Starter's disk watcher.

**Decision: every effect whose outcome the app shows is an ask, answered exactly once, also by an
adapter that cannot carry it.** An adapter without a clipboard answers `Failure::Unsupported`; an
adapter without randomness answers `Failure::Unavailable`. The camera's and the system's own
failures have their `Unsupported` too, and an answer with no failure in it is answered with
nothing known, as P9 gives it. Reason: one rule (one ask, one answer)
lets the app's `update` handle every outcome as a message and lets a pure update test cover it.
Turned down: `main`'s exceptions, where a shell with no randomness leaves the request
unanswered and the stick shell ignores `StoreSettings` and `PreviewSize`, which leave the app
guessing whether anything happened.

### 2.2 How an answer finds its request

```rust
/// Minted by the runtime only. `generation` counts the builds of the
/// lock scope in this process, from 1 (a lock in Faraday Demo or the
/// Starter's Faraday Demo builds the scope again in process); the part
/// outside the scope (the Starter's own composite and its settings owner)
/// asks with generation 0. `n` counts asks within one generation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AskId { pub generation: u32, pub n: u32 }

/// Every ask's own type implements this; the answer type is the ask's.
pub trait Request: Into<Ask> {
    type Answer;
    /// Takes this ask's answer out of the port's enum; any other variant
    /// is the adapter's fault.
    fn take(answer: Answer) -> Result<Self::Answer, Answer>;
    /// This ask's answer when the adapter gave the wrong one (P9).
    fn fault() -> Self::Answer;
}
```

For example, `storage::Read { stick, name }` has `type Answer = Result<Bytes, Failure>`, and
`camera::On { camera }` has `type Answer = Result<camera::Started, camera::Failure>`.

The context of a request (which item, which flow step, which stick) is kept on the app's side, in
the runtime, and never travels to an adapter. The kernel's shape for it (section 03 §4.2 gives it
in full; this is the port's requirement):

```rust
/// In the kernel. A reply is a function, not a closure, and its context
/// is Public: what waits in the runtime for an answer can hold no secret.
pub struct AskFor<M> { /* request: port::Ask, reply: erased, built from a Public context and fn pointers */ }

impl<M: Message> AskFor<M> {
    pub fn new<R: Request, C: Public>(request: R, context: C, reply: fn(C, R::Answer) -> M) -> Self;
    pub fn map<N: Message>(self, f: fn(M) -> N) -> AskFor<N>;   // a composite lifts a child's ask
    pub fn into_parts(self) -> (Ask, Reply<M>);                // the runtime's only (A16)
}
```

- **Rule P7.** An answer carries its request's context back as the feature's own message: the
  runtime keeps `(context, reply)` under the `AskId`, and on `Event::Answer` calls
  `reply(context, R::take(answer))` and queues the message to the feature that asked, through the
  composites that hold it. Held by the compiler (the kernel's signature).
- **Rule P8.** A reply's context is `Public` and the reply is a function pointer, so nothing a
  request waits on keeps a secret. Held by the compiler (`C: Public`, `fn`, not `FnOnce`).
  Reason: a closure could capture a typed passphrase and keep it in the runtime's table past its
  screen. Turned down: a boxed closure as `Job<M>` is, which a job needs (it moves its secret in)
  and an answer does not (its secret comes in the answer).
- **Rule P9.** An answer whose generation is neither 0 nor the lock scope's current build is
  dropped by the runtime, and its payload wipes as it drops (§6). Generation 0 is never stale: no
  lock and no `Exit::Reset` (§4.9) drops it. An answer of the wrong kind for its ask (`R::take`
  fails) reaches the feature as `R::fault()`, which is that ask's adapter failure:
  `Failure::Adapter`, `camera::Failure::Adapter` or `system::Reason::Adapter` (for `Keep`, with an
  empty carry: the carry went to the adapter). An answer with no failure in it gives nothing
  known: `None` for `Ask::Memory`, `ReadBitLocker` and `JudgeBoot`, empty lists for
  `ListBootFiles` and `FindImages`, `()` for `CheckAgain`. The runtime notes the fault for
  diagnostics. Held by an update test of the runtime (section 07).
- **Rule P10.** An adapter echoes the `AskId` and never makes one. Its answers also repeat the
  request's own identifying fields where they have them (the stick, the file's name), so that an
  answer read alone says what it answers.

### 2.3 The worker job is not a port type

This answers section 01's question on how the worker-job effect is typed.

**Decision: the port has no job variant.** A job is the kernel's `Job<M>` (section 01, §4.1): a
boxed function of `&Compute` that owns its inputs and returns the feature's message. The runtime
owns the pool, takes every job from `Effects::jobs` and never hands one to a root or an adapter.
Reason: a job's answer is an app message, which the port, below the application, cannot name, and
no adapter ever runs app code. Turned down: an `Effect<J>` generic over a job type, which puts a
type parameter on every adapter for a variant none of them may carry. "Worker job" in P1's list
of effects is therefore the kernel's, and §8 of this section asks section 01 to say so.

The `Waker` (§1) is how a finished job wakes the root's loop; it replaces `main`'s
`Faraday::set_wake`.

## 3. Events in

```rust
pub enum Event {
    /// First, exactly once per process. Nothing else comes before it. A
    /// scope built again in process gets its start from the runtime, not
    /// from an adapter (section 05, §7.6).
    Start(Start),
    /// The display changed: a resize, a turn, a preview size, a new screen.
    Display(Display),
    /// Monotonic and wall time, at each tick (§3.3).
    Time(Time),
    Input(Input),
    Sticks(SticksFact),
    Devices(DevicesFact),
    Camera(CameraFact),
    Host(HostFact),
    System(SystemFact),
    /// The person closed the window (desktop roots only).
    CloseAsked,
    Answer(AskId, Answer),
}
```

`Event` and `Answer` are neither `Clone` nor `Public` (§6). `Event::kind(&self) -> EventKind`
gives a `Copy` enum of the variant names, which is what the runtime logs.

### 3.1 Start

```rust
pub struct Start {
    pub display: Display,
    pub medium: Medium,            // Stick | SdCard | Desktop
    pub abilities: Abilities,
    pub machine: Machine,
    pub time: Time,
    /// What the previous process kept for this one (§4.2); moved in, once.
    pub carried: Option<Carry>,
    /// The Starter's own settings text, stored unread (§4.8).
    pub settings: Option<Text>,
    /// The computer's light or dark scheme, where the adapter reads one.
    pub scheme: Option<Scheme>,
    /// The stick adapter's read of the kernel's mount list: the device's
    /// root is mounted read-only. The self-test takes it as an argument
    /// (section 01, §2.1). `None` off the device. On the device the stick
    /// adapter always answers (a mount list it cannot read is `Some(false)`),
    /// and the runtime passes the self-test `Some(false)` for a `None` there
    /// (`Medium::Stick` or `SdCard`): the check fails closed.
    pub root_read_only: Option<bool>,
}

/// What this product's adapters carry. The app offers a control only
/// where its effect is carried (a Paste button only with a clipboard).
pub struct Abilities {
    pub clipboard: bool,
    pub print: bool,
    pub host_files: bool,        // Downloads, a dropped file, saving a received file
    pub opens: bool,             // opening a folder, a file or the one fixed address
    pub camera: bool,
    pub names_sticks: bool,      // false for folders standing for sticks
    pub holds_back_input: bool,  // the device: new input devices wait (F153)
    pub restart_computer: bool,
    pub moves_window: bool,
    pub system: bool,            // the Starter's operating-system asks (§4.10)
    pub lock_ends_process: bool, // the device: a lock ends the process (§4.2, §4.9)
}

pub struct Machine {
    pub maker: Option<Text>,
    pub product: Option<Text>,
    pub family: Option<Text>,
    pub chassis: Option<u8>,
    pub hypervisor: bool,                  // the CPU's hypervisor flag
    pub screen: Option<Edid>,              // the first 128 bytes, as read
    pub secure_boot: SecureBootAtStart,    // from the firmware's boot parameters
    pub memory_total_mib: Option<u32>,
    pub cores: u16,
}

/// `Start::scheme` is `None` where the computer's setting says neither
/// or cannot be read; the product's own default then holds (the Starter's is Light).
pub enum Scheme { Light, Dark }
```

`Medium`, `BootMedium`, `SecureBootAtStart` and `StickInfo` move from `faraday-core` into the port
as plain data; their wording (`StickMake::about`, `size_text`) stays in the application's
presentation.

What the root picks (which modes this product offers, the Demo's dice roller and its pointer and
keys randomness: `main`'s `set_modes`, `set_roller_offered`, `set_pointer_keys_offered`) is not a
port fact: it is how the root builds its composite (section 06).

### 3.2 Display

```rust
pub struct Display {
    pub width: u16,
    pub height: u16,
    pub dpi: u16,
    pub orientation: Orientation,
    /// The screen may be turned; its native orientation.
    pub rotation_offered: Option<Orientation>,
    /// The picture goes to an external screen.
    pub external: bool,
    /// Faraday Demo's preview of a device's size, and the size chosen.
    pub preview: Option<Preview>,
}

/// Sent by an adapter that can preview. `chosen` is the size shown, in
/// dp, as `Order::PreviewSize` asks it; `None` is the window's own size.
pub struct Preview { pub chosen: Option<(u16, u16)> }
```

`Preview` and `Order::PreviewSize` count in dp, as on `main`; every other size in the port is in
pixels.

**Decision: the display's facts are what some feature reads, and no more.** `main`'s
`SecureHardware`, `BootState`, `buttons`, `camera_fixed`, `inset_top` and `inset_bottom` are
dropped: no shell sets them to anything but their default, or no feature reads them. Reason: a
field no adapter fills is a rule nobody can test. Turned down: keeping them for a phone shell,
which `next` does not have. Section 01's P4 names two of them; §8 asks for that to change.

### 3.3 Time

```rust
pub struct Time { pub now_ms: u64, pub unix_secs: u64 }
```

`now_ms` is monotonic from the adapter's start; `unix_secs` is the computer's clock, which the app
corrects by the offset the person set (it never writes the clock: `features/settings-time.md`). A
tick comes every 50 ms at rest and every frame while the last frame said `Pace::Moving` (§5). This
folds `main`'s `Event::Tick` and `StorageEvent::Clock` into one fact. The app has no clock of its
own: rule D1 and R3 keep every read of time in the adapters.

### 3.4 Input

```rust
pub enum Input {
    Touch { at: Point, phase: TouchPhase },   // a finger, or the left button
    Hover(Option<Point>),                     // None: the pointer left
    Scroll { at: Point, dy: i16 },            // a trackpad, in whole pixels
    ScrollEnd { at: Point },
    Wheel { at: Point, dy: i16 },             // 48 px a notch
    Key(Key),
    KeyUp(Key),
    Shift { held: bool },
    Back,
    Forward,
    Focus(bool),
}
```

`Key` is `main`'s, unchanged (characters, editing keys, Tab and BackTab, SelectAll, Paste).
`main`'s `Event::Button` (hardware buttons) is dropped: no shell sends it. A held-back device's
input is not `Input`: it is a `DevicesFact` (§3.6), so a device the person has not let in can
never press anything (`features/devices-inputs.md`).

### 3.5 Sticks

```rust
pub enum SticksFact {
    /// The kernel shows storage attached, listed or not (it blocks typing a
    /// passphrase and loading a key: features/stick-visit.md).
    Attached(bool),
    /// Every stick present now, on start and on every change.
    Listed(Vec<StickInfo>),
    /// A stick the backup flow cannot use: not FAT32 as Faraday reads it,
    /// or read-only.
    Problem { stick: StickId, read_only: bool },
    BootMedium(BootMedium),
}
```

### 3.6 Devices

```rust
pub enum DevicesFact {
    /// The device check's view, while the app watches (§4.9).
    Usb(Vec<UsbDevice>),
    Screen(Option<Edid>),
    /// The USB devices present when the start window closed.
    BootUsb(Vec<Text>),
    NewInput { id: InputId, name: Text, keyboard: bool, pointer: bool, touch: bool },
    Typed { id: InputId, ch: char },
    Tapped { id: InputId, at: Point },
    Used { id: InputId },
    Gone { id: InputId },
    Trust { pointer: bool, keyboard: bool },
}

/// The adapter's number for a held-back input device, as on `main`.
pub struct InputId(pub u32);
```

`UsbDevice` (port, vendor, product, strings, interfaces) moves from `faraday-core`'s `devices`
into the port's `devices` module. These replace `main`'s calls from the stick shell into the app
(`usb_devices`, `screen_changed`) and the input events carried on `StorageEvent`.

### 3.7 Camera

```rust
pub enum CameraFact {
    /// The cameras this adapter can open; sent before every answer to
    /// `camera::On`, and when the list changes.
    Listed(Vec<CameraInfo>),     // id and the name it gives
    /// A running stream's frame, for the preview and New key's noise.
    Frame { stream: AskId, frame: CameraFrame },
    /// A code the adapter's scanner read in this stream.
    Code { stream: AskId, bytes: Bytes },
    /// What one scan pass saw: corners, module size, read or not, whole.
    Seen { stream: AskId, seen: QrSeen },
    /// The stream ended without being asked to: the camera went or fell
    /// silent at every size.
    Ended { stream: AskId, why: camera::Failure },
}

pub struct CameraFrame {
    pub width: u16,
    pub height: u16,
    pub luma: Bytes,
    pub chroma: Option<Bytes>,   // NV12's plane; a grey preview without it
}
```

A stream is named by the `AskId` of the `camera::On` that started it, so a frame of a stream the
app has turned off is recognised and dropped (§7).

### 3.8 Host and System

`HostFact` is the desktop computer's: `Downloads { dir: Text, files: Vec<(Text, u64)> }` (sent on
change; the adapter rescans every 700 ms), `Hovered(bool)` and `Dropped { path: Text, size: u64 }`
(`features/transfer.md`). `SystemFact` is the Starter's operating system: `Computer`, `Disks`,
`NoWriter`, `Installed`, `DevImage`, `ImagesWanted`, `Concerns` (every 3 s), `OtherEncryption`,
`BootEntry(bool)`, and `WriteProgress` of a stick being written (§4.10) (`features/starter-start.md`, `features/starter-make-stick.md`), with
`main`'s `StarterEvent` value types moved into the port's `system` module, `String` replaced by
`Text`.

## 4. Effects out

```rust
pub enum Ask {
    Storage(StorageAsk),
    Keep(Carry),
    Forget,
    Host(HostAsk),
    Camera(camera::On),
    Clipboard,
    Print(PrintJob),
    Entropy,
    Memory,
    Settings(Text),
    System(SystemAsk),
}

pub enum Answer {
    Storage(StorageAnswer),
    Kept(Result<(), (Failure, Carry)>),
    Forgotten(Result<(), Failure>),
    Host(HostAnswer),
    Camera(Result<camera::Started, camera::Failure>),
    Clipboard(Result<SecretText, Failure>),
    Printed(Result<Text, Failure>),           // where the PDF went
    Entropy(Result<Entropy, Failure>),
    Memory(Option<u32>),                       // MiB free now
    Settings(Result<(), Failure>),
    System(SystemAnswer),
}
```

Each variant's payload is a struct of its own implementing `Request` (§2.2); the enums above are
the wire. Every `Failure` is typed (§7.2).

### 4.1 Storage: the sticks

```rust
pub enum StorageAsk {
    Read { stick: StickId, name: Text },
    /// The codes in a picture on a stick; the adapter decodes it.
    ReadCodes { stick: StickId, name: Text },
    Write {
        stick: StickId, name: Text, bytes: Bytes,
        copies: Vec<Sha256>,   // every copy of this vault the session has seen
        replace: bool,         // the person chose to replace
        renamed: bool,
        on_taken: OnTaken,     // Ask | Beside | Over(Sha256)
    },
    Wipe { stick: StickId, name: Text },
    Space { stick: StickId },
    ZeroFree { stick: StickId, from: u32 },
    MakeEmpty { stick: StickId },
    Rename { stick: StickId, from: Text, to: Text, vaults: Vec<Vec<Sha256>> },
    Name { stick: StickId, label: Text },
    Hash { stick: StickId, name: Text },
}

pub enum StorageAnswer {
    Read(Result<Bytes, Failure>),
    Codes(Result<Codes, Failure>),             // the picture's bytes and each code's payload
    Written(Result<Written, Failure>),         // Wrote { wrote_as } | NameTaken { on_stick, sha }
    Wiped(Result<Wiped, Failure>),             // Zeroed | NameOnly
    Space(Result<Room, Failure>),              // free, size; 0 and 0 for a folder
    Zeroing(Result<Zeroing, Failure>),         // next, total, done
    Emptied(Result<(), Failure>),              // MakeEmpty's
    Renamed(Result<(), Unrenamed>),            // Taken | SameVault | Failed(Failure)
    Named(Result<(), NameRefused>),            // Invalid | Failed(Failure)
    Hashed(Result<Sha256, Failure>),
}

pub struct Sha256(pub [u8; 32]);
pub struct Room { pub free: u64, pub size: u64 }   // in bytes
```

The space answer's type is `Room`, since `Space` is already the ask's own type (§2.2).
`MakeEmpty` is answered `Emptied`, where `main` answered it as a zeroing step at cluster 2 of 0;
the app then asks `ZeroFree` itself. `NameRefused::Invalid` is a label FAT cannot hold, which the
app has already checked before it asks; a folder stick, which has no label, answers
`Failed(Unsupported)`. `Sha256` is the port's own type of the 32 bytes, in `storage`.

These are `main`'s `StorageCommand` and their answers from `StorageEvent`, with each answer tied
to its ask by `AskId` (`features/storage.md`, `features/stick-visit.md`,
`features/delete-and-zero.md`, `features/files-inbox.md`, `features/boot-import.md`). `StickId` is a
`Text` newtype: the adapter's name for a stick, as `StickInfo::id` gives it. `OnTaken`,
`Unrenamed` and `NameRefused` are the port's own: `faraday-files`, which defines the disk side's,
is not `no_std`, and the storage adapter maps one to the other.

- **Rule P11.** A write is written whole or not at all (section 03, A34): the storage and host
  adapters write every file under a temporary name in the same folder, read it back and compare
  it, and only then rename it to its name and answer `Wrote`. A write that fails removes what it
  wrote before it answers, zeroing it first on FAT, where a removed file's bytes stay on the
  stick; when the temporary file cannot be removed, the answer is `Failure::LeftBehind` with its
  name, so the app can name it. A vault's write-back replaces only a file whose SHA-256 is one of
  `copies` or the `Over` hash; a write never goes over a file unless `replace` or `Over` says so.
  Held by the storage and host adapters over `faraday-files`' rules, and by an adapter test of a
  write that fails part-way (section 07).
- **Rule P12.** The app checks a file's size against the domain's limits before it asks
  (`faraday-codec`'s `MAX_READ`, a vault's `MAX_FILE_LEN`), and the adapter checks again as it
  reads (`accept_part`'s rules on `main`); the answer is `Failure::TooLarge`, never a cut file.
  Held by the storage adapter and an update test of the feature that reads.

### 4.2 Storage: the carry across a lock

```rust
/// What one process leaves the next across a lock: named files in the
/// app's own format, stored unread, each written whole (a temporary file, then a rename)
/// in the order given.
pub struct Carry {
    pub inbox: Files,
    pub outbox: Files,
    pub kept: Files,      // settings, the signed-amount memory, the mode, the input counts, ...
}
pub type Files = Vec<(Text, Bytes)>;
```

`Ask::Keep(Carry)` hands it to the adapter, and is asked only where the lock ends the process
(`Abilities::lock_ends_process`, the device). Where the lock does not end it (Faraday Demo, the
Starter's Faraday Demo), the carry never leaves the runtime: it moves from the dropped scope into
the next build (section 05, §7.6). `Ask::Forget` removes every carried file (zeros first) when the
app ends for good. `Keep` and `Forget` are a family of their own: the adapter serves them from its
own keeping (the RAM directory), never through the disk process, so they are never queued behind
a stick's work nor answered `Busy`; a lock during a vault's write-back keeps its carry. The next
process gets it once, moved, in `Start::carried`, and the adapter keeps no copy after handing it
over.

**Decision: the carry is handed over once, at lock, by move.** Reason: no second copy of the
Inbox's secrets sits in an adapter while the app runs. Turned down: `main`'s `SaveBoxes` after
every change to the Inbox or Outbox, which keeps every Inbox secret twice for the whole session.
A failed keep first removes every file it wrote, zeroing each, and then hands the carry back in
its answer (`Err((failure, carry))`), so the app still holds it and decides what to do (section
05, §7.6). The one case in which the carry does not come back is a wrong answer to `Keep` (P9): the
carry went to the adapter, and the runtime reads the answer as a failed keep with an empty carry.
That is an adapter defect, never a state the device is meant to reach: each adapter answers `Keep`
only with `Kept`, held by the adapter's own test.

**Decision: the mode and the input counts are entries of `kept`.** `main`'s stick shell read them
from the app at lock (`mode_after_restart`, `input_carry`) and wrote them to its own files. Reason:
the adapter then knows nothing of the app's modes or counts; it stores bytes. Turned down: two
files of the stick adapter's own beside the carry, which make the adapter parse the app's mode. Which entries are
written first (on `main`, `backup-gate` and `backup-flow`) is the app's order in `kept`.

### 4.3 Host: the desktop computer

```rust
pub enum HostAsk {
    /// A file in Downloads or one dropped on the window, for Transfer.
    Read { path: Text },
    /// A received file into Downloads: never over a file; `name (2).ext`
    /// and on, up to 999.
    Save { name: Text, bytes: Bytes },
    Open(Openable),     // Downloads | Institute | File(path)
}

pub enum HostAnswer {
    Read(Result<Bytes, Failure>),
    Saved(Result<Text, Failure>),       // the path
    Opened(Result<(), Failure>),
}
```

`Openable::Institute` is the one fixed address `main` opens (the Settings About note), and the
port names no other: the app can open no address it composes. `Openable::File` opens a PDF the
Starter saved.

### 4.4 Camera

`camera::On { camera: Option<CameraId> }` starts a stream on the chosen camera, or the adapter's
first, and is answered `Started { stream }` once frames flow or the camera is on, or a
`camera::Failure` (`Refused`, `InUse`, `NotFound`, `Unavailable`, `Unsupported`, and `Adapter`
for P9). `CameraId` is a `Text` newtype, the adapter's name for a camera as `CameraInfo::id`
gives it, like `StickId`. Choosing another
camera on the scan sheet is `Order::CameraOff` then a new `camera::On` with its id, so the choice
gets a real answer. This replaces `main`'s `take_camera`, read by the shell from the app, whose
failure was only printed.

### 4.5 Clipboard

`Ask::Clipboard` reads the system clipboard once, for a Paste the person pressed, answered with
`SecretText` or `Failure::Empty` (nothing, not text, over 1 MiB) or `Unsupported`.

**Decision: the port has no clipboard write.** Faraday has no Copy action, and a secret is never
put on the clipboard (`docs/DESIGN.md` §4.9 on `main`; inventory `features/scanner-qr.md`). Reason:
an effect that does not exist cannot be misused. Turned down: keeping `main`'s unused
`WriteClipboard`.

### 4.6 Print

```rust
pub struct PrintJob { pub name: Text, pub pdf: Bytes, pub holds_secret: bool }
```

Answered with where the PDF went. Where that is belongs to the adapter: Faraday Demo saves it in
its print folder; the Starter writes it to a fresh temporary folder and opens it in the system's
viewer, never printing it itself (`features/starter-app.md`). `holds_secret` is set for the
Starter's notes with a recovery key, whose folder rules the Starter's adapter keeps
(`features/starter-start.md`). A device adapter answers `Unsupported`.

### 4.7 Randomness and memory

`Ask::Entropy` is answered with `Entropy { bytes: EntropyBytes, source: EntropySource }`: 32 bytes
from the adapter's best source (the kernel's random device on the device, the operating system's on a desktop)
and which one (`DevRandom`, `Os`), which New key words. The app asks at start and whenever it
rotates its session key (`features/randomness.md`). `Ask::Memory` is answered with the MiB free
now, which a vault's unlock reads before it asks Argon2id for memory
(`features/vault-unlock.md`). It replaces `main`'s `set_memory_reader`, a function the app called.

### 4.8 Settings

`Ask::Settings(Text)` keeps the app's settings text where the adapter's platform keeps application
data, unread; `Start::settings` hands it back. Only the Starter uses it (its text size, kept only
while a Starter is installed). The device's settings travel on the boot stick and in the carry, as
files.

### 4.9 Orders

```rust
pub enum Order {
    CameraOff,
    /// A held pointer's targets are up for this device; the arrow shows
    /// (None: no sheet aims).
    AimInput(Option<InputId>),
    /// The person let a held device in, or kept it out.
    DecideInput { id: InputId, believed: bool },
    /// Run the device check and send `DevicesFact::Usb` and `Screen`.
    WatchDevices(bool),
    /// Whether this process has held or typed no secret; the stick
    /// adapter publishes it for the grant helper.
    Clean(bool),
    Turn(Orientation),                 // a `Display` follows
    PreviewSize(Option<(u16, u16)>),   // a `Display` follows
    /// Faraday Demo's stick pills: plug a folder stick in or pull it; a
    /// `SticksFact::Listed` follows.
    ToggleStick(StickId),
    Vibrate { ms: u16 },
    Exit(Exit),
}

pub enum Exit {
    /// The process ends for good: the device powers off; a desktop root
    /// closes.
    End,
    /// A lock that ends the process: the device's process ends with the
    /// restart code.
    Lock,
    /// The device restarts the computer.
    RestartComputer,
    /// The lock scope of this generation was dropped in this process: at a
    /// lock that does not end the process, at a failed keep's rebuild and at
    /// the Starter's Close. Each adapter drops what belonged to that
    /// generation and sends its state facts again (section 05, §7.6).
    Reset(u32),
}
```

`Reset` carries the generation of the scope dropped (§2.2); it never touches generation 0.

`AimInput`, `DecideInput` and `Clean` replace the stick shell's reads of the app's state
(`input_aims`, `input_is_used`, `poll_input_decision`, `clean`); `Exit` replaces `Command::Exit`
read beside `restart_requested`. `Vibrate` is in the plan's list of effects; no feature in the
inventory sends it and every adapter of the three products ignores it.

### 4.10 System: the Starter's operating system

`SystemAsk` holds `main`'s `StarterCommand`s that do operating-system work, typed as they are on
`main` and answered by the matching `SystemAnswer`: `WriteStick(WriteOrder)` (its progress comes as
`SystemFact::WriteProgress { ask, stage, done, total }`), `Install`, `Uninstall`, `RemoveBootEntry`,
`RestartIntoFirmware`, `RestartFromStick { device }`, `ReadBitLocker`, `BitLocker(step)`,
`ReadRecoveryKey` (answered with recovery keys as `SecretText`), `ReadVersion`, `ListBootFiles`,
`JudgeBoot`, `CheckBootFile`, `FindImages { dropped }`, `CheckAgain`. The others find a home
elsewhere: `SaveTemplate` is `HostAsk::Save`, `PrintTemplate` and `PrintNotes` are `Ask::Print`,
`OpenTemplate` is `HostAsk::Open(File)`, `KeepTextSize` is `Ask::Settings`, and the Starter's
own `Close` is `Order::Exit(End)`. `OpenDemo` and `DemoClosed` are not effects: the Starter has
one model, and Faraday Demo is its lock scope, beside the Starter's own composite (section 01,
§7.2; section 05, §7.9). What `OpenDemo` did on the computer is one ask,
`SystemAsk::PrepareDemoSticks`: the desktop adapter makes the Demo's sticks folder and its
`BOOTSTICK` and `STICK-2`, answered `Ok` or `Reason::File { path, detail }` naming the folder,
which the Starter shows as `main` shows `DesktopStarted(Err)` (`features/starter-app.md`). On `Ok` the Starter's
composite sends `Top::Open` and the runtime builds the scope at the next generation, with a start
it makes from its own facts; no adapter sends a second `Start`. Closing Faraday Demo inside the
Starter (its Quit, or the Starter's Back over it) is a Close: the runtime drops the scope and
sends `Order::Exit(Exit::Reset(generation))`; the Starter goes on.

Each ask's answer, and the `StarterEvent` it replaces:

| `SystemAsk` | `SystemAnswer` | On `main` |
|---|---|---|
| `WriteStick(WriteOrder)` | `WriteStick(Result<(), Reason>)` | `Written` |
| `Install` | `Install(Result<Text, Reason>)` | `InstallDone` |
| `Uninstall` | `Uninstall(Result<Uninstalled, Reason>)`, `Uninstalled { boot_entry_kept: Option<Reason> }` | `UninstallDone`, and `BootEntryKept` before it |
| `RemoveBootEntry` | `RemoveBootEntry(Result<(), Reason>)` | `BootEntryRemoved` |
| `RestartIntoFirmware`, `RestartFromStick { device }` | `RestartIntoFirmware(Result<(), Reason>)`, `RestartFromStick(..)` | `RestartFailed`, `StickRestartFailed`; a restart that happens ends the process, and nothing waits for its `Ok` |
| `ReadBitLocker` | `ReadBitLocker(Option<BitLocker>)` | `BitLocker` |
| `BitLocker(step)` | `BitLocker { step, result: Result<(), Reason> }` | `BitLockerDone` |
| `ReadRecoveryKey` | `RecoveryKey(Result<Vec<RecoveryKey>, Reason>)` | `RecoveryKey` |
| `ReadVersion { disk, serial, size, source }` | `VersionRead { disk, serial, size, version: Version }`, `Version { result, signed }` | `VersionRead` |
| `ListBootFiles` | `BootFiles { folder, files }` | `BootFiles` |
| `JudgeBoot { disk, serial, size, sha256, want, source }` | `BootJudged { ..., signed: Option<bool> }` | `BootJudged` |
| `CheckBootFile { path, source }` | `BootFileChecked { path, result: Result<(BootFile, BootSigner), Reason> }` | `BootFileChecked` |
| `FindImages { dropped }` | `ImagesFound(Vec<ImageFinding>)` | `ImagesFound` |
| `CheckAgain` | `CheckedAgain` | none |
| `PrepareDemoSticks` | `PrepareDemoSticks(Result<(), Reason>)` | `DesktopStarted` |

The rest of `StarterEvent` are `SystemFact`s (§3.8), the answers of `HostAsk::Save` and `Open`
and of `Ask::Print` (`TemplateSaved`, `TemplateOpened`, `NotesOpened`), or `Start::settings`
(`TextKept`). `CheckAgain` is an ask though its result comes as `SystemFact::Concerns`: its answer
comes after the facts it sent, so the Starter knows the check is done.

The family's failure is `system::Reason`, `main`'s, with `Unsupported` for an adapter that has no
system (§2.1) and `Adapter` added for P9. Its value types are `main`'s, with three changes:

- `Os` has no macOS: it is not being developed.
- `RecoveryKey { drive, id, key: SecretText }` has public fields and no checks: the port holds no
  behaviour (P6), and the app checks its shape (P25). Its hand-written `Debug` names the drive
  only (P16).
- `ReleaseFault::Fat` holds `FatFault`, the port's copy of `faraday_fat::Error` variant for
  variant (with `FatBoot`, `FatCorrupt`, `FatName` for its inner enums), which the Starter's
  adapter maps. Reason: the app words it (P24), and the port does not depend on `faraday-fat`
  (P5). Turned down: the adapter's wording of it as `Text`.

## 5. The frame out

```rust
pub struct Frame<'a> {
    pub width: u16,
    pub height: u16,
    /// Premultiplied RGBA8888, row-major, width x height x 4 bytes; the
    /// adapter converts to its panel's format.
    pub rgba: &'a [u8],
    /// Rectangles that may have changed since the last frame. Empty is
    /// unknown, not unchanged: the adapter takes the frame whole.
    pub changed: &'a [Rect],
    /// A band moved by dy since the last frame (a scroll).
    pub moved: Option<Moved>,
    /// Places a press belongs to the window, not the app.
    pub regions: &'a [Region],    // Region::MovesWindow(Rect)
    pub pace: Pace,               // Rest | Moving
}
```

```rust
/// The rows the adapter last took inside `band` now stand `dy` rows lower
/// (higher for a negative `dy`), but for the rows the move brought into
/// view, which the frame painted. `None` for a frame drawn whole.
pub struct Moved { pub band: Rect, pub dy: i16 }
```

An adapter that ignores `moved` is right, as on `main`. The frame is borrowed from the runtime's last frame (R4) and
lives until the root's next call into the runtime. `main`'s `Parts::inner` (one app's frame held inside another's) is dropped: the
Demo's strip is a feature of the same model, so there is one frame.

**Decision: a press in a `MovesWindow` region is the window adapter's.** It starts a drag after
4 px, and two presses within 400 ms maximise or restore, as on `main`; the app is not told. Reason:
the operating system needs the drag started while the button is down, which an answer through the
app's queue cannot promise. Turned down: `main`'s call from the window into the app
(`is_strip_room`) on each press, and an order from the app, which would come after the press.

- **Rule P13.** The runtime's frame buffer wipes on drop and is cleared before a lock drops the
  lock scope. An adapter that keeps a copy of the picture (the stick's converted and last-shown
  bands, a window's buffer) keeps it in a wiping buffer, blanks the panel and wipes its copies on
  `Exit`, before the process ends or the scope is built again. Held by the runtime's and the
  adapters' buffer types (`Secret<Vec<u8>>`) and a test of the stick adapter (section 07).

## 6. What wipes, what is Public

This answers section 01's question on which event fields carry wiping types.

- **Rule P14.** Every payload of an event or effect that may hold a secret is a wiping type
  that is not `Clone`: `Bytes` (`faraday_crypto::Secret<Vec<u8>>`) for bytes, `SecretText` for text that
  may be a secret, `EntropyBytes` for randomness. Held by the compiler (the field types) and by
  `lint-secrets`, which refuses `Vec<u8>`, `String` or `Zeroizing` in a port type.

| Field | Type | Why it wipes |
|---|---|---|
| a file read, written, carried, saved, read from the computer, a picture's bytes | `Bytes` | any file may hold a seed or a key; a file received by Transfer may be one |
| a code's payload (`Codes`, `CameraFact::Code`) | `Bytes` | it may be a SeedQR |
| a camera frame's luma and chroma | `Bytes` | a frame can show a SeedQR or written words, and New key takes frames as noise |
| a PDF to print | `Bytes` | a backup sheet or the Starter's notes may carry a key |
| pasted text | `SecretText` | a paste may be seed words |
| a recovery key | `SecretText` | it unlocks a disk |
| randomness | `EntropyBytes` | it seeds the session key |

Names, paths, labels and stick ids are `Text`, which wipes on drop and is `Clone`; none is a
secret. Hashes, sizes, coordinates and enums are plain values.

**Decision: camera frames wipe at the port, as everywhere else.** On `main`, the camera crates and
the app's scan state wipe their frames, and the contract between them did not (`luma: Vec<u8>`).
Reason: a frame is a picture of whatever the person holds up, and one plain hop leaves the copy
the others avoid. Turned down: borrowing frames from a shared buffer, which would make the
adapter's buffer live as long as the app's view of it.

**Decision: `EntropyBytes` loses `Clone`.** Reason: K-3; nothing copies it on `main`. Turned
down: keeping `Clone` for tests, which can build two from the same array.

- **Rule P15.** No port event, answer or effect is `Public`, and none is logged or replayed
  whole. The runtime logs `Event::kind()` and `Effect::kind()`, `Copy` enums of the variant names,
  which the kernel marks `Public`. A typed character (`Key::Char`, `DevicesFact::Typed`) or a
  touch on the on-screen keyboard is part of a passphrase. Held by the compiler: `Public` is the
  kernel's sealed trait (M2) and is implemented for the kinds only.
- **Rule P16.** `Debug` on a port type prints names, kinds and lengths, never bytes or text that
  may be a secret. Held by the compiler: `Secret` and `SecretText` have no `Debug`, so a derived
  one fails to compile; the hand-written ones are the port's.

## 7. Order and failure

### 7.1 Order

- **Rule P17.** `Start` is the first event of every process, exactly once; an adapter queues
  anything that comes before it, and no adapter sends a second. A scope built again in process
  starts from the runtime's kept facts and the carry (section 05, §7.6). Held by the runtime,
  which refuses to build the model without it, and by a journey test of each root.
- **Rule P18.** The effects of one `update` are carried in the order it returned them; a
  composite keeps its children's order. Events from one adapter arrive in the order it made them,
  and the runtime handles one event at a time.
- **Rule P19.** Asks of one family (`StorageAsk`, host, system) are carried out in the order sent and
  answered in that order. Across families there is no order. Held by each adapter's single queue
  per family, and an adapter test.
- **Rule P20.** One `StorageAsk` runs at a time. A `StorageAsk` made while a vault is moving in
  pieces is answered `Failure::Busy` at once, nothing sent (`features/storage.md`).
- **Rule P21.** After `Order::CameraOff` or `Exit`, no fact of the stopped stream reaches a
  feature: the runtime drops `CameraFact`s whose `stream` is not the running one. A lock drops
  the camera's stream before it waits on a seal (`features/scanner-qr.md`, `features/lock.md`).

### 7.2 Failure

```rust
pub enum Failure {
    Unsupported,      // this product does not carry it
    Unavailable,      // carried, but nothing here now (no camera, no randomness)
    Busy,             // P20
    Silent,           // the disk process or a helper did not answer in time
    Gone,             // the stick, the file or the device went
    TooLarge,
    NoRoom,
    ReadOnly,
    NotFat,
    Malformed,        // an answer from the disk process the adapter could not read
    Empty,            // the clipboard held nothing usable
    Declined,         // the person declined an elevation or a prompt
    Adapter,          // P9: the wrong answer for the ask
    LeftBehind(Text), // P11: a failed write's temporary file, by name, that could not be removed
    Os(OsError),      // the operating system's code, for diagnostics
}

/// As `std::io::Error::raw_os_error` gives it: an errno, or a Windows
/// error code.
pub struct OsError(pub i32);
```

- **Rule P22.** An effect that fails comes back as an answer carrying its `Failure`; no adapter
  panics on an ask, and no app crate panics on an answer. Every answer type is total over its
  failures. Held by the types, by `#![cfg_attr(not(test), deny(clippy::unwrap_used,
  clippy::expect_used, clippy::panic))]` at the root of every port, application and runtime
  crate (checked by `lint-layers`), and by an update test for each failure a feature shows. An
  adapter copied from `main` keeps its own asserts; P23 and an adapter test hold that it answers.
- **Rule P23.** An adapter answers every ask, at most at its own timeout: storage 15 s, a write
  and a piece read 120 s, a picture 60 s, as on `main`. A late answer from below (the disk
  process) is dropped by its sequence number. When an adapter's worker thread or helper ends,
  every waiting ask is answered `Silent` at once and later asks fail at once. Held by the storage
  adapter and an adapter test.
- **Rule P24.** Failures are typed and the app words them. Reason: one place for wording (V4), and
  an update test can name the failure. Turned down: `main`'s `reason: Text` worded by the storage
  client and copied from the disk process (`NOT_OPEN`), which puts wording in an adapter and makes
  the app match strings.
- **Rule P25.** Nothing that comes through the port is trusted. An answer's bytes, a frame, a
  pasted text and a dropped file are parsed and checked by the domain (STANDARDS §0 on `main`).
  The adapters' own limits (a picture at most 4096 x 4096, a FAT top folder at most 65536
  entries, a clipboard at most 1 MiB) are defences beside the domain's, not in place of them.

## 8. Storage behind the port

| On `main` | In `next` |
|---|---|
| `faraday-core`'s `storage_api` (`StorageCommand`, `StorageEvent`, `StickInfo`, `Files`, `OnTaken`, `Carried`) | The port's `storage` module and §4.1, §4.2. The app's side (what to read, refusing a file too large, constant-time comparison of commands in tests) goes to the features that ask |
| `faraday-storage`: the `Sticks` trait, `Dirs` (folder sticks), `DiskProcess` (the FIFO client), `Boxes`, the host's Downloads, print folder and opening, the memory read | The storage adapter, re-pointed at the port: it takes `Ask::Storage`, `Keep`, `Forget`, `Host`, `Print`, `Memory` and answers them. It depends on the port, `faraday-files`, `faraday-fat` (through the files crate), `faraday-crypto` (SHA-256, in place of `faraday-core`'s `file_menu::sha256`), `faraday-qr` (`MAX_FILE`). Its edge to `faraday-core` is cut |
| `faraday-disk` (the disk process) and `faraday-grant` | Unchanged adapter processes (S2). The storage adapter speaks to the disk process over `faraday_files::proto`, as on `main`; the response `Failed` carries the failure's kind (`proto::Kind`: `Gone`, `TooLarge`, `NoRoom`, `ReadOnly`, `NotFat`) beside its words, and the adapter maps it one to one to the port's `Failure` of that name (P24) |
| The stick shell's disk-work thread, stick scan every 700 ms, `attached`, hiding sticks not yet handed out | The stick adapter. It sends `SticksFact::Attached`, `Listed`, `Problem`, `BootMedium`; it hides a stick the grant has not handed over while `Order::Clean(true)` stands, and tells a process that is not clean at once |
| The stick shell's boxes in its RAM directory, `mode` and `input-counts` files | The storage adapter's carry in the RAM directory, read and removed at start into `Start::carried`; the RAM directory's carry files removed at `Exit::Reset` |
| Faraday Demo's sticks folder, `BOOTSTICK`, `STICK-2`, the pills, the Downloads and camera listing thread | The desktop adapter (a library the Demo and Starter roots share, section 01 §9): folder sticks over the folder `Place` moved out of `faraday-files` (section 01, §2.1), `Order::ToggleStick`, `HostFact::Downloads`, `CameraFact::Listed`. It holds no carry: a lock does not end the process, so the carry stays in the runtime (§4.2) |
| The Starter's own boxes (memory) and `settings.txt` | The Starter's adapters: no carry (Faraday Demo is the Starter's lock scope, and its carry stays in the runtime, §4.2); `Ask::Settings` for its text size |

- **Rule P26.** The storage adapter knows no app type and no file's meaning beyond what
  `faraday-files` reads (a vault by its contents, FAT names, system files ignored by name). Held by
  `lint-layers` (no edge to the application, P3).
- **Rule P27.** A stick's bytes reach the app only as the answer to an ask, and a stick is written
  only by an ask the app made (a person's action, or the seal at a lock). The stick adapter's own writes are its dev reports (the
  timings and the boot report), written only when the root is started with its dev flags, as on
  `main` (`features/dev-diagnostics.md`). Held by the stick adapter's arguments and a journey test.

## 9. The camera behind the port

| On `main` | In `next` |
|---|---|
| `faraday-camera` (frames, NV12 helpers, scaling, the list), `faraday-v4l2`, `faraday-mediafoundation` | Capture adapters, as they are; `faraday-camera`'s `Frame` holds a `Vec<u8>` (it has no edge to the port), and the adapter that sends a frame wraps it in `Bytes` |
| `faraday-scanner` (`Scanner`, `ReadMode`, `reduce`) | The scanning adapter: it reads codes from frames on its own thread and sends `CameraFact::Code` and `Seen` |
| The stick shell's capture thread (the size ladder 1920 x 1080, 1280 x 720, 640 x 480, 15 fps, a queue of 2, the newest frame per pass) | The stick adapter, with `ReadMode::Reduced` |
| The window shell's lens (V4L2 on Linux, Media Foundation on Windows, `--no-camera`) | The window adapter, with `ReadMode::Full` |
| The app's `ScanState` (the decoder, the parts, the read card) | The scanner part of the application (section 01, §4.2) |

**Decision: codes are read in the adapter, as on `main`.** Reason: a decode costs more than the gap
between frames on the Pi, and the runtime's loop would then answer presses at the decode rate.
Turned down: decoding in a worker job, which would move every frame through the app twice.
The adapter reads; the app classifies what it read (`features/scanner-qr.md`, "Classification").

- **Rule P28.** A frame the app is sent is reduced to what the preview needs (the adapter reads
  codes in the whole frame first); the adapter keeps no frame after it has been sent and read, and
  dropping a stream wipes the capture buffers (on V4L2, the mapped buffers are zeroed before they
  are unmapped). Held by the capture adapters' types and an adapter test.
- **Rule P29.** Every `camera::On` is preceded by `CameraFact::Listed` and answered, whether or not
  the camera starts; a camera that ends on its own sends `Ended` and is off, so the next
  `camera::On` tries again.

## 10. What each product's adapters carry

| Effect or fact | Device (stick and Pi) | Faraday Demo | Starter | Tour root |
|---|---|---|---|---|
| `Start`, `Display`, `Time`, `Input` | framebuffer, evdev (each device grabbed), touch, the arrow | window, mouse, touch, keys | window, as the Demo | scripted |
| `Turn`, rotation offered | yes | no | no | scripted |
| `PreviewSize`, preview | no | yes (not in panel mode) | yes | scripted |
| `Region::MovesWindow` | no | yes | yes | ignored |
| `Ask::Storage`, `SticksFact` | disk process over FAT | folder sticks | folder sticks, for its Demo | fixture sticks |
| `ToggleStick` | no | yes | yes | scripted |
| `Keep`, `Forget`, `Start::carried` | RAM directory | never asked (answered if asked); `carried` is `None` | as the Demo | as the Demo |
| `Ask::Host`, `HostFact` | `Unsupported` | yes | yes | fixture |
| `Ask::Print` | `Unsupported` | print folder | temporary folder and viewer | recorded |
| `Ask::Camera`, `CameraFact` | V4L2, reduced read | V4L2 or Media Foundation, full read | as the Demo | fixture frames |
| `Ask::Clipboard` | `Unsupported` | yes | yes, for its Demo | fixture |
| `Ask::Entropy` | the kernel's random device | the operating system's | the operating system's | fixed test bytes |
| `Ask::Memory` | the kernel's memory figures | the kernel's and the cgroup's, or Windows' | as the Demo | fixed |
| `DevicesFact`, `AimInput`, `DecideInput`, `WatchDevices` | yes | `Trust` both true at start; the rest never | as the Demo | scripted |
| `Clean` | the clean marker | ignored | ignored | ignored |
| `Ask::Settings` | `Unsupported` | `Unsupported` | `settings.txt` | memory |
| `Ask::System`, `SystemFact` | `Unsupported` | `Unsupported` | yes | fixture |
| `Exit::Lock` | process ends, restart code | never sent | never sent | never sent |
| `Exit::Reset` | after a failed keep's rebuild: the adapters reset (section 05, §7.6) | after every lock: the adapters reset | after a lock inside Faraday Demo and at its Close | after every lock |
| `Exit::End`, `RestartComputer` | power off, restart | close; never offered | close the Starter; never offered | ends the tour |
| `Vibrate` | ignored | ignored | ignored | ignored |

An `Unsupported` row is still answered (§2.1), and the `Abilities` in `Start` say which rows a
product carries, so the app offers no control whose effect would be refused.

## 11. Every effect of main and its home

`main`'s effects are `Command` (`faraday-shell-api`), `StorageCommand` (`faraday-core`),
`StarterCommand` (`faraday-starter`), and the calls the shells make into the app outside the
contract.

| On `main` | In the port |
|---|---|
| `Command::Draw` | none: the root paints the runtime's frame when it changed (§5) |
| `Command::Exit`, read with `restart_requested` and the exit codes 0, 75, 76 | `Order::Exit(End \| Lock \| RestartComputer)`; an in-process lock is `Exit::Reset` |
| `Command::CameraOn`, `CameraOff` | `camera::On` (an ask), `Order::CameraOff` |
| `Command::RequestClipboard` | `Ask::Clipboard` |
| `Command::RequestEntropy` | `Ask::Entropy` |
| `Command::Orientation`, `PreviewSize` | `Order::Turn`, `Order::PreviewSize` |
| `Command::Vibrate` | `Order::Vibrate` |
| `StorageCommand::Read`, `ReadQr`, `Write`, `Wipe`, `StickSpace`, `ZeroFree`, `MakeEmpty`, `Rename`, `NameStick`, `Hash` | `StorageAsk` (§4.1); `StickSpace` is answered `StorageAnswer::Space(Room)`, and `MakeEmpty`, which `main` answered as a zeroing step, `StorageAnswer::Emptied` |
| `StarterEvent::*` | `SystemAnswer`, `SystemFact`, the answers of `HostAsk` and `Ask::Print`, and `Start::settings` (§4.10) |
| `StorageCommand::SaveBoxes`, `ForgetBoxes`, `StorageEvent::Restored`, `NotKept` | `Ask::Keep`, `Ask::Forget`, `Start::carried`, `Answer::Kept(Err)` |
| `StorageCommand::Print` | `Ask::Print` |
| `StorageCommand::ReadHost`, `SaveDownload`, `OpenDownloads`, `OpenInstitute` | `HostAsk::Read`, `Save`, `Open(Downloads)`, `Open(Institute)` |
| `StarterCommand::*` (`OpenDemo` as `SystemAsk::PrepareDemoSticks`) | `SystemAsk`, `HostAsk`, `Ask::Print`, `Ask::Settings`, `Order::Exit` (§4.10) |

Added, each for a call `main`'s shells made into the app or a value it read:

| Added | Replaces on `main` |
|---|---|
| `Ask::Memory` | `set_memory_reader`, `StorageEvent::Memory`, `DisplayInfo::memory_mib` |
| `Order::AimInput`, `Order::DecideInput` | `input_aims`, `input_is_used`, `poll_input_decision` |
| `Order::WatchDevices` | the stick shell reading `app.devices.read` |
| `Order::Clean` | `clean()` read by the stick shell |
| `Order::ToggleStick` | `take_stick_toggles` |
| `camera::On { camera }` with a camera id | `take_camera` |
| `Region::MovesWindow` in the frame | `is_strip_room`, `moves_window` |
| `Pace` in the frame | the window and stick shells reading `busy()` and the fades |
| `Start` (facts, abilities, carried, the read-only root) | `fresh_app`'s setters, `Event::Display` first, `RotationOffered`, `ExternalScreen`, `PreviewOffered`, `EntropySource`, `start_unrestarted` |
| `DevicesFact::Usb`, `Screen` | `usb_devices`, `screen_changed` |
| `Event::CloseAsked` | the window closing the loop with no word to the app |
| the `Waker` trait | `set_wake` |

Not carried, each with its reason:

| On `main` | Why not |
|---|---|
| The file channel (`RequestFile`, `ReadFile`, `WriteFile`, `Event::File`, `FileList` and their answers) | No feature uses it; files go through storage and the host |
| `StoreSettings`, `Event::Settings` | Never sent by the app; the Starter's settings are `Ask::Settings` |
| The kept-secret channel (`SecureMac`, `StoreSecret`, `LoadSecret`, `ForgetSecret`, `SecretKept`) | Never sent: no shell has a secure element |
| `WriteClipboard` | §4.5 |
| `Event::Button`, `Event::Lock` | No shell sends them |
| `Parts::inner` | §5 |
| `StarterCommand::DemoClosed` | §4.10: closing Faraday Demo is the runtime's Close of the lock scope, which the adapters see as `Exit::Reset` |
| `Faraday::press` from the shells | Test and dev scripting; tours send `Input` through the tour root (R4) |

## 12. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| P5, P6 | one `no_std` crate of types, no I/O, no wording | `lint-layers`; the attribute; `clippy.toml`; `lint-strings` |
| P7, P8 | an answer returns as the asker's message; its context is `Public` and its reply a `fn` | the compiler (the kernel's signature) |
| P9, P10 | stale and wrong answers, each ask's fault; ids echoed | an update test of the runtime |
| P11, P12 | writes whole or not at all, read back; sizes checked twice | the storage and host adapters; adapter and update tests |
| P13 | the frame and its copies wipe | buffer types; a stick adapter test |
| P14, P16 | payloads wipe and are not `Clone`; `Debug` prints no secret | the compiler; `lint-secrets` |
| P15 | only kinds are `Public` and logged | the compiler (sealed `Public`) |
| P17 to P21 | order | the runtime; each adapter's queues; journey and adapter tests |
| P22 to P25 | failure is an answer; timeouts; typed failures; nothing trusted | the types; the crate-root `deny`, checked by `lint-layers`; update and adapter tests |
| P26, P27 | storage knows no app type; sticks written only when asked | `lint-layers`; a journey test |
| P28, P29 | camera frames and streams | the capture adapters' types; an adapter test |

## Changes asked of earlier sections

- Section 01, §3, P1: "worker job" in the list of effects is the kernel's `Job<M>`, which the
  runtime takes from `Effects::jobs`; the port has no job type (§2.3 here). Reason: as written, P1
  reads as if the port carried it.
- Section 01, §3, P4: the display's value types the kit may name are `Display`, `Orientation`,
  `Point` and `Medium`; drop "its boot and secure-hardware state", which the port no longer has
  (§3.2). Reason: no shell sets them.
- Section 01, §4.1: the sketch of `Effects` gains the asks, `asks: Vec<AskFor<M>>`, beside
  `port: Vec<port::Order>`, since an ask needs its reply kept on the app's side (§2.2). Reason: a
  `port::Effect` alone cannot bring its answer back to the feature.

## Questions for later sections

- Section 03: the kernel's `AskFor<M>` exactly, how a composite maps a child's asks to its own
  message type, and how `Abilities` reach a feature (in `Ctx`).
- Section 05: what happens when `Ask::Keep` fails at a lock while keys wait for their backup: the
  model stays (as `main`'s not-kept path) or the lock scope is built again in process, and in
  that case which adapter state is reset (the held inputs back to waiting, the camera off, the
  sticks scanned again).
- Section 05: the runtime's dropping of stale answers and stream facts (P9, P21), and the frame
  buffer's clearing at lock (P13).
- Section 06: which adapters each root builds, and how the Starter's root builds Faraday Demo's
  lock scope after `SystemAsk::PrepareDemoSticks`.
- Section 07: the adapter tests this section names (storage write-back, timeouts, the camera's
  buffers, the stick's frame copies): whether they are a test kind of their own beside update
  tests and journeys.
