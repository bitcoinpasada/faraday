# The application layer: reducers and effects, step flows, worker answers

**Kind:** Spec
**Open:** Not built.

This section gives the application kernel's full shapes (section 01, §4.1): `Feature`, the
messages and `Public`, `Effects` with its orders, asks, changes and jobs, `Ctx` and `View`,
`AtLock` and the handles. It says how a composite routes its children's messages and outs, how
the session (keys, wallets, what waits) is a feature that others read through `View` and change
only by message, how a step flow is a typed state machine driven by one runner in the kernel, how
a worker's answer comes back as a message carrying its own context, how Practice refuses, how a
refusal is typed and reaches the wording, and how a file holding a secret is written. It answers
the questions sections 01, 02 and 08 left for it. It is written for whoever writes the kernel
crate, a feature, a composite or a root's model, and for the writers of sections 04 (present),
05 (the runtime) and 09 (the manifest). Rules continue section 01's application numbering: A9
onward.

## 1. The kernel crate

The kernel is one crate (`layer = "app"`, `role = "kernel"`). It depends on the port, the domain
crates whose types it names (`faraday-text`, `faraday-crypto`, `faraday-bip`, `faraday-psbt`,
`faraday-keys`, `faraday-strings`) and its derive crate. It holds no feature's types (A1).

| Module | Holds |
|---|---|
| `feature` | `Feature`, `Notice`, `route` (§2) |
| `message` | `Message`, `Public`, `Withheld`, the derives (§3) |
| `effects` | `Effects`, `Fx`, `AskFor`, `ChangeFor`, `Job`, `Route`, `Note` (§4) |
| `cx` | `Ctx`, `View`, `Facts`, `Mode`, `Clock`, `Running` (§5) |
| `shared` | `Change`, `Done`, `ChangeId`, `Owner`, and the read traits and change types of the session, the vaults and the settings (§7) |
| `flow` | `StepFlow`, `Flow`, `FlowMsg`, `Steps`, `CancelOffer`, `Ending` (§9) |
| `lock` | `AtLock`, `Holds`, `Loss` (§2.3) |
| `handle` | `KeyId`, `WalletId`, `FileId`, `Ticket`, `Waiting` (§3.3, §8) |
| `words` | `Worded`, `Refused`, `Line`, the wording of `port::Failure` (§10) |

The derive crate (`role = "kernel"`, a proc-macro crate) derives `Message`, `Public` and `AtLock`.

## 2. Features

### 2.1 The trait

```rust
pub trait Feature {
    /// Never Clone (L1). Its fields are private to the feature's crate (A7).
    type State: AtLock;
    type Msg: Message;
    /// What the feature tells its parent (A6).
    type Out: Message;
    /// Why a message is refused before `update` sees it (§2.2, §10).
    type Refusal: Refused;

    /// The state at the model's start, from what the last model carried
    /// for this feature (section 05 gives `Carried`).
    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (Self::State, Effects<Self::Msg, Self::Out>);

    /// One message: the state changes, effects come back.
    fn update(state: &mut Self::State, msg: Self::Msg, cx: &Ctx<'_>) -> Effects<Self::Msg, Self::Out>;

    /// What every feature is told (§2.4). No default: each feature says
    /// what it does with each notice.
    fn notice(state: &mut Self::State, notice: &Notice<'_>, cx: &Ctx<'_>) -> Effects<Self::Msg, Self::Out>;

    /// Whether `msg` is refused now. Called by `route` before `update`, and
    /// by `present` to draw the control that would send it (§2.2).
    fn refuses(state: &Self::State, msg: &Self::Msg, view: &View<'_>) -> Option<Self::Refusal>;
}
```

A feature is named by a type: an empty enum whose `State` is the feature's state, or, for a
step flow, the flow's own state type, which gets `Feature` from the kernel (§9).

### 2.2 One gate for update and drawing

```rust
/// How a composite or root passes a message to a child.
pub fn route<F: Feature>(state: &mut F::State, msg: F::Msg, cx: &Ctx<'_>) -> Effects<F::Msg, F::Out>;
```

`route` calls `F::refuses`; a refused message gives one note (the refusal's words) and nothing
else, as `main`'s Practice toast does; otherwise it calls `F::update`.

- **Rule A9.** A message reaches a feature's `update` only through `route`, so `update` is never
  given a message `refuses` refuses and does not check again. Held by `lint-layers`' module rule:
  outside the kernel, only the feature's own crate names its `update` (its tests included).
- **Rule A10.** `present` asks `refuses` for each control it offers and draws a refused one dimmed
  (or, where the refusal says `Drawn::AsUsual`, as usual: an eye or a Show button that answers
  with the note). A control drawn enabled is never refused, and one refused is never drawn
  enabled, since both read one function. Held by the signature (`refuses` takes `&View`, which
  `present` has) and by an update test of each refusal (section 07).
- `refuses` decides from state, the message and `View` only. A message that is an answer (an
  ask's, a change's, a job's) is never refused: `refuses` matches presses alone.

### 2.3 What a lock wipes

```rust
pub trait AtLock {
    /// What this state holds now; read by the runtime for the clean
    /// marker (`Order::Clean`), handing a stick out, the idle lock and the
    /// Lock sheet.
    fn holds(&self) -> Holds;
    /// Take every secret on show off the screen, keeping what was typed:
    /// at Lock pressed (even while a seal waits), at the idle warning, and
    /// at every step change of a flow (§9).
    fn hide(&mut self);
    /// Wipe what a lock wipes. Called before the model is dropped.
    fn at_lock(&mut self);
}

pub struct Holds {
    pub kept: bool,     // a key store or an open vault holds material
    pub typed: bool,    // a secret on its way: typed, rolled, scanned, shown
    pub session: bool,  // something a lock loses that is not secret
    /// A lock would lose this; the Lock sheet names it and asks first.
    pub asks_first: Option<Loss>,
}
```

**Decision: `AtLock` has `holds`, `hide` and `at_lock`.** Reason: `main`'s clean state, idle lock,
stick hold and Lock sheet all read `holds`, and its one list of secrets on show (called at Lock
and at the idle warning) is `hide`; a lock that drops the model still needs both, before the
seal. Turned down: `at_lock` alone, as section 01 sketches, which leaves the runtime no way to
know a typed secret is held.

- **Rule A11.** `AtLock` is derived on a struct. Every field carries one mark: `#[lock(kept)]`,
  `#[lock(typed)]`, `#[lock(session)]` (each wiped by `at_lock` and counted by `holds` through the
  field type's `Holding` impl), `#[lock(shown)]` (also reset by `hide`) or `#[lock(public)]`
  (kept until the model drops; the field's type must be `Public`). A field with no mark, or a
  `public` mark on a type that is not `Public`, fails to compile. A state that is an enum (a step
  flow's step, §9) writes its impl by hand and its update test covers `holds`. Held by the derive
  and the compiler. This carries `main`'s listing rule (a new field does not compile until its
  lock is said) into every state.
- **Rule A12.** After `at_lock`, `holds()` is all false. The runtime asserts it in debug builds;
  an update test of each feature holding a secret covers it.

### 2.4 Notices

```rust
pub enum Notice<'a> {
    /// None of this feature's pages is shown any more: a press elsewhere,
    /// Back, Home, a sheet's Cancel. Drop what only a screen needed.
    Left,
    /// Take secrets off the screen now (`AtLock::hide` is called after).
    Hide,
    /// A key, wallet, file or vault came, went or changed (§7.2).
    Shared(&'a shared::Changed),
    /// A fact in `View::facts` changed: sticks, display, cameras, time.
    Facts(FactsChanged),
}
```

- **Rule A13.** A screen left drops what it held, however it was left: the root's navigation
  sends `Notice::Left` to the feature whose page was left, through its composites, and the
  feature wipes every typed secret and every shown value the screen needed. A flow begun keeps
  its answers (it is resumed later) but empties its passphrase fields and hides. Held by
  `notice` having no default and by an update test of each feature with a typed secret.

## 3. Messages and handles

### 3.1 Message and Public

```rust
/// Every message type. Derived.
pub trait Message: Send + 'static {
    /// The variant's path, such as "create.Forward"; always logged.
    fn kind(&self) -> &'static str;
    /// The message whole, when its variant is Public; `None` when withheld.
    fn public(&self) -> Option<&dyn core::fmt::Debug>;
}

/// Holds no secret: may be logged, replayed, cloned and kept in a reply's
/// context. Sealed (§3.2).
pub trait Public: Clone + core::fmt::Debug + Send + 'static + __private::Sealed {}
```

`#[derive(Message)]` on an enum makes every variant public unless it is marked `#[withheld]`; a
variant not so marked whose fields are not all `Public` fails to compile. A withheld variant logs
its kind alone. `#[derive(Public)]` on a type requires every field to be `Public`.

- **Rule A14.** Only the derive implements `Public` and `__private::Sealed`. `lint-secrets` refuses a
  hand-written `impl Public` or `impl Sealed` outside the kernel, which writes the impls for
  `bool`, the integers, `char`, `Text`, `Fingerprint`, `Network`, `Txid`, `Sha256`, the handles,
  the port's kinds and ids (`AskId`, `StickId`, `EventKind`), `Option`, `Vec`, arrays and tuples
  of `Public`. No wiping secret type (`SecretText`, `Secret`, `Bytes`, `EntropyBytes`), no
  `faraday-keys` store and no part holding a typed passphrase is `Public`. Held by the compiler and
  `lint-secrets`. This makes M2 exact.

### 3.2 Why sealed by a lint

A Rust trait implemented by a derive in another crate cannot be sealed by visibility alone. The
derive names `faraday_kernel::__private::Sealed`, and `lint-secrets` refuses that path anywhere
but in derive output (it reads the source, which holds only `#[derive(Public)]`). Turned down:
an `unsafe trait Public`, which would put `unsafe` on every derive and grow U2's list.

### 3.3 Handles

```rust
#[derive(Public, Copy, PartialEq, Eq, Hash)] pub struct KeyId(u32);
#[derive(Public, Copy, PartialEq, Eq, Hash)] pub struct WalletId(u32);
#[derive(Public, Copy, PartialEq, Eq, Hash)] pub struct FileId(u32);
// VaultId is faraday-keys'; StickId is the port's.
```

**Decision: a handle is an id minted by the session and never reused within a model, not an
index.** A read through a handle whose thing is gone answers `None`, never another thing.
Reason: on `main` a wallet's leaving moved or ended every flow that held it by place
(`wallet_left`, inventory `features/wallets.md`); with ids, a feature holding a gone id learns of
it from `Notice::Shared` and ends or moves itself. Turned down: indices into the session's lists.
Held by the session's id counters (private) and an update test of the session.

## 4. Effects

### 4.1 One ordered list

```rust
#[must_use]
pub struct Effects<M, O>(Vec<Fx<M, O>>);

pub enum Fx<M, O> {
    /// To an adapter, no answer (section 02 §4.9).
    Order(port::Order),
    /// To an adapter, answered once (section 02 §2). A write is whole
    /// bytes in one ask, written whole or not at all (A34, §12).
    Ask(AskFor<M>),
    /// To a shared owner: the session, the vaults, the settings (§7).
    Change(ChangeFor<M>),
    /// Only an owner: the answer to a change it was given (§7.3).
    Done(ChangeId, shared::Done),
    /// To the worker pool (§8).
    Job(Job<M>),
    /// Ask the runtime to stop this feature's running job of this kind (§8.3).
    Stop(JobKind),
    /// A message to this same feature, handled before the next event.
    Then(M),
    /// To the parent (A6).
    Out(O),
    /// A short line the frame shows (a snackbar), typed (§10).
    Note(Note),
}

impl<M: Message, O: Message> Effects<M, O> {
    pub fn none() -> Self;
    pub fn order(self, o: port::Order) -> Self;
    pub fn ask<R: port::Request, C: Public>(self, r: R, context: C, reply: fn(C, R::Answer) -> M) -> Self;
    pub fn change<R: shared::Change, C: Public>(self, r: R, context: C, reply: fn(C, R::Done) -> M) -> Self;
    pub fn job(self, j: Job<M>) -> Self;
    pub fn then(self, m: M) -> Self;
    pub fn out(self, o: O) -> Self;
    pub fn note<W: Worded + Public>(self, n: W) -> Self;
    /// A composite takes a child's effects into its own (§6).
    pub fn lift<N: Message, P: Message>(self, msg: fn(M) -> N, out: fn(O) -> Route<N, P>) -> Effects<N, P>;
}
```

**Decision: effects are one ordered list, not a vector per kind.** Reason: P18 carries the
effects of one update in the order it returned them, and a write followed by the order that
depends on it (the camera off after a scan's ask) keeps its order only in one list. Turned down:
section 01's sketch of four vectors, whose order across kinds is lost.

- **Rule A15.** An event and every message it causes (`Then`, a routed `Out`, the answer of a
  change served at once) are handled before the runtime takes the next event, in the order they
  were returned; a frame is drawn only after. Held by the runtime (section 05) and an update test
  of the runtime.

### 4.2 Asks: `AskFor<M>` exactly

This answers section 02's question.

```rust
pub struct AskFor<M> {
    request: port::Ask,
    /// Built only by `new` and `map`; holds a Public context and fn pointers.
    reply: Box<dyn FnOnce(port::Answer) -> (M, Fault) + Send>,
}

impl<M: Message> AskFor<M> {
    pub fn new<R: port::Request, C: Public>(request: R, context: C, reply: fn(C, R::Answer) -> M) -> Self;
    pub fn map<N: Message>(self, f: fn(M) -> N) -> AskFor<N>;
    /// The runtime's only: the request to carry and the reply to keep.
    pub fn into_parts(self) -> (port::Ask, Reply<M>);
}
pub enum Fault { None, WrongKind }  // P9: the runtime notes WrongKind
```

`new` builds the box from `context` and `reply`: it calls `R::take` on the answer and, on the
wrong kind, `R::fault()`. `map` wraps it with one more `fn`. Two `fn` pointers cannot be one
`fn`, so the chain is erased into the box; what it captures is the `Public` context and `fn`
pointers alone, which is what P8 asks.

- **Rule A16.** No feature, composite or root builds an `AskFor` or a `ChangeFor` but through
  `new` and `map`, and only the runtime calls `into_parts`. Held by the private fields and by the
  application directories' `clippy.toml` (`disallowed-methods` on `into_parts`). P8 then holds:
  nothing a request waits on keeps a secret.

### 4.3 Jobs: `Job<M>`

```rust
pub struct Job<M> {
    kind: JobKind,
    quiet: bool,   // runs without a busy line (main's VaultMeasure, KefTime)
    run: Box<dyn FnOnce(&Compute) -> M + Send>,
}

impl<M: Message> Job<M> {
    /// The input is moved in, a secret included; `work` is a function.
    pub fn new<I: Send + 'static>(kind: JobKind, input: I, work: fn(I, &Compute) -> M) -> Self;
    pub fn quiet(self) -> Self;
    pub fn map<N: Message>(self, f: fn(M) -> N) -> Job<N>;
}

/// "<feature>.<job>", a constant of the feature's crate.
#[derive(Public, Copy, PartialEq, Eq)] pub struct JobKind(pub &'static str);
```

A job's input may hold a secret (a vault's passphrase, a PSBT, a MuSig2 round's secret nonces);
it is moved in and either dropped on the worker or moved back in the answer (K-3, §8.2 of
section 01). `faraday-keys`' job types (`SignJob`, `StretchJob`, `SealJob`) are such inputs: the
key material stays in its types, masters behind the `Arc` the key store already shares.

### 4.4 Notes

```rust
pub struct Note(Box<dyn Worded>);
impl Note { pub fn new<W: Worded + Public>(w: W) -> Self; }
```

A note is a typed value the frame's snackbar words when it is drawn (§10). It is how a refused
press, a written file or a removed key tells the person, and the only text-like effect: an update
forms no text (A30).

## 5. Ctx and View

This answers section 01's question, field by field.

```rust
/// What `present` reads. Shared borrows only; no Compute.
pub struct View<'a> {
    pub session: &'a dyn session::Read,     // kernel trait (§7.2), the session owner's state
    pub vaults: &'a dyn vaults::Read,       // kernel trait (§7.2), the vault composite's state
    pub settings: &'a Settings,             // kernel type (§7.2), the settings owner's state
    pub facts: &'a Facts,                   // kernel type, kept by the runtime from port facts
    pub mode: Mode,                         // kernel type, fixed for the model's life (§11)
    pub clock: Clock,                       // kernel type: the adapter's time and the person's offset
    pub running: &'a [Running],             // kernel type: jobs on the pool now, for busy lines
    pub strings: &'static faraday_strings::Strings,
}

/// What `update` reads: the view, and the token for costly work.
pub struct Ctx<'a> {
    pub view: View<'a>,
    pub compute: &'a Compute,               // faraday-crypto's; minted by the runtime (R2)
}

pub struct Facts {
    pub display: port::Display,
    pub medium: port::Medium,
    pub abilities: port::Abilities,
    pub machine: port::Machine,
    pub root_read_only: Option<bool>,
    pub sticks: Sticks,                     // attached, listed (port::StickInfo), boot medium, problems
    pub cameras: Vec<port::CameraInfo>,
    pub downloads: Option<Downloads>,       // port::HostFact::Downloads, latest
    pub entropy_source: Option<port::EntropySource>,
    pub selftest: SelfTest,                 // Passed | Failed(check) | NotRun
}

pub struct Clock { pub now_ms: u64, pub unix_secs: u64, pub offset_secs: i64, pub set: bool }
pub struct Running { pub kind: JobKind, pub began_ms: u64, pub done: u32, pub of: u32 }
```

`Facts` holds the latest value of each fact that is a state of the world; the runtime replaces a
field as its port fact arrives and sends `Notice::Facts`. Facts that are a stream (input devices,
camera frames and codes, the Starter's system facts) are not in `Facts`: the root maps each to a
message for the feature that handles it (section 06).

- **Rule A17.** `Abilities` reach a feature as `view.facts.abilities`. A feature offers a control
  only where its effect is carried (a Paste only with `clipboard`), through `refuses` (A10), and
  handles the `Unsupported` answer as any failure (P22). This answers section 02's question.
- **Rule A18.** `View` has no `Compute` and no `&mut`, and `Ctx` adds only `Compute`. A read the
  shared traits give is cheap; a costly derivation (an address list, a merged share pool) is
  worked out in `update` or a job and kept in state, or memoised by `present` (section 04). Held by
  the compiler and `costs-lint` (V2).
- **Rule A19.** A Starter model without a Demo child has no keys: its root lends an empty session
  and empty vaults (`session::Read` and `vaults::Read` for `()`), so every feature has the same
  `View`. Held by the types.

**Decision: `Ctx` is `View` plus `Compute`.** Reason: `refuses` and `present` read the same
facts `update` does, and one struct makes that a type. Turned down: a separate `Ctx` of its own
fields, which lets the gate and the drawing read different things.

**Decision: randomness is drawn by a change to the session, not through `Ctx`.** A draw consumes
the session's pool and may have to wait for the seed (F102 on `main`): `session::Draw { what, n }`
is answered with the bytes or `NoRandomness` after the wait. Reason: a shared borrow cannot wait
or consume, and an update test gives the bytes as the answer. Turned down: a randomness handle in
`Ctx` with interior mutability, which would be the one write through `&Ctx` and could not wait.

## 6. Composites and routing

A composite is a feature whose state holds its children's states (A6). Its `Msg` has one variant
per child carrying the child's `Msg`; its `update` routes each to the child with `route` and lifts
the child's effects.

```rust
pub enum Route<M, O> {
    /// A message to this composite: usually another child's, wrapped.
    To(M),
    /// Up to this composite's parent.
    Up(O),
    /// Nothing; the out is answered by doing nothing here.
    Drop,
}
```

`lift(msg, out)` maps every `Ask`, `Change` and `Job` with `msg` (the child's variant
constructor, a `fn`), keeps every `Order`, `Note` and `Stop`, turns each `Then(m)` into
`Then(msg(m))`, and each `Out(o)` into `Then(n)` for `Route::To(n)` or `Out(p)` for
`Route::Up(p)`, in place. So a child's out reaches its sibling as a queued message to the
composite, handled before the next event (A15).

- **Rule A20.** A composite never calls a sibling's `update` inside a child's dispatch: it routes
  by `Route::To`. Each `update` is then one step, its effects one list, and an update test of one
  feature sees every out it makes. Held by A9 (only `route` reaches `update`) and the module rule.
- **Rule A21.** A composite forwards every `Notice` to each child in the order it holds them,
  and its `AtLock` is the derive over its children's states (each `#[lock(...)]` field a child).
  Held by A11.

### 6.1 Worked example: Make a new wallet, then Back up

Inventory `features/create-wallet.md` and `features/wallet-backup.md`: Continue on Check marks the
wallet checked and starts Back up of it (unless money is being moved); Move the money opens Spend
from a backup. Both live under the wallets composite.

```rust
// The wallets composite (role = "composite").
pub enum Msg { Create(create::Msg), Backup(backup::Msg), SpendBackup(spend_backup::Msg) }
pub enum Out { Open(Page), Leave(Ending), Scan(ScanFor) }

fn update(s: &mut State, msg: Msg, cx: &Ctx<'_>) -> Effects<Msg, Out> {
    match msg {
        Msg::Create(m) => route::<create::MakeWallet>(&mut s.create, m, cx).lift(Msg::Create, from_create),
        Msg::Backup(m) => route::<backup::BackUp>(&mut s.backup, m, cx).lift(Msg::Backup, from_backup),
        Msg::SpendBackup(m) => route::<spend_backup::SpendBackup>(&mut s.spend_backup, m, cx)
            .lift(Msg::SpendBackup, from_spend_backup),
    }
}

fn from_create(o: create::Out) -> Route<Msg, Out> {
    match o {
        create::Out::Checked { wallet } => Route::To(Msg::Backup(FlowMsg::Own(
            backup::Own::Begin { wallet, from: backup::From::Create }))),
        create::Out::MoveMoney { from } => Route::To(Msg::SpendBackup(FlowMsg::Own(
            spend_backup::Own::Begin { from }))),
        create::Out::Scan(purpose) => Route::Up(Out::Scan(purpose)),
        create::Out::Ended(e) => Route::Up(Out::Leave(e)),
    }
}
```

`backup::Own::Begin` answers with `Out::Open(Page::Backup)` among its effects, which the
composite sends up and the root's navigation shows. Create never names Back up's types (A5); the
composite is the one place the hand-over is written, and its test is one routing test.

## 7. The shared owners: the session, the vaults, the settings

### 7.1 Where they sit

**Decision: three features are shared owners, held by the root beside its app composite: the
session, the vault composite and the settings.** Every other feature reads them through `View` and
changes them only by a change (§7.3). Reason: the runtime can then lend `&` of the three to `Ctx`
while it updates the app composite with `&mut`, two disjoint fields of the model; and when it
serves a change it updates the owner with `&mut` while no `Ctx` borrows it. Turned down: the
session as a child of the device composite, whose state the composite may not read (A7) and could
not lend while it updates a sibling.

The session holds: the key store (`faraday_keys::Keys`), the wallets, what waits (PSBTs waiting
to be signed, keys held until their backup is written), the Inbox and Outbox (files held), the
session's randomness (seed, pool, the person's own) and the wallet-confirm queue. **Decision: the
Inbox and Outbox are the session's.** Reason: most features put a file in the Outbox and the
session pane draws the files held; one owner keeps one list (K8). Turned down: a files feature
others message, which would be a fourth owner for one list.

The vault composite holds the open vaults (`faraday_keys::vault::OpenVaults`) and the vault
features' states (section 01, §4.3). The settings owner holds the person's choices.

### 7.2 Reading

```rust
pub mod session {
    pub trait Read {
        fn keys(&self) -> &faraday_keys::Keys;             // reveals are faraday-keys' (K-1)
        fn key(&self, id: KeyId) -> Option<KeyView<'_>>;  // fingerprint, name, held, sample, made online
        fn wallets(&self) -> &[WalletView];               // public data only
        fn wallet(&self, id: WalletId) -> Option<&WalletView>;
        fn waiting(&self) -> &[Waiting];                  // PSBTs to sign, keys held for backup
        fn inbox(&self) -> &[FileView];
        fn outbox(&self) -> &[FileView];
        fn seeded(&self) -> bool;
    }
    pub struct WalletView { /* id, name, policy (faraday-bip), origin, source, checked, network */ }
    pub enum Changed { KeyCame(KeyId), KeyGone(KeyId), WalletCame(WalletId), WalletGone(WalletId),
                       FileCame(FileId), FileGone(FileId), HeldReleased(KeyId), Network(Network) }
}
pub mod vaults { pub trait Read { fn open(&self) -> &faraday_keys::vault::OpenVaults; /* marks, current */ } }
pub struct Settings { /* idle lock and off minutes, QR speed, time offset, appearance, scale, rotation */ }
```

`WalletView`, `KeyView` and `FileView` are kernel types of public data (A1); `main`'s `Wallet`
holds no secret (inventory `features/wallets.md`) and becomes `WalletView`, its caches kept by
the session. A `FileView` of a file holding a secret gives its name, kind and whether it holds
one, never its bytes; the bytes are taken by a change.

### 7.3 Changing: `ChangeFor<M>`

This answers section 01's question on how a feature asks the key store for an operation that
changes it.

**Decision: a change to the key store, a wallet, what waits, a file held, a vault or a setting is
a change sent to its owner, answered once as the asker's own message. There is no `&mut` in
`Ctx`.** Reason: A8 stands (`&Ctx` gives no `&mut`), the owner's `update` is the one place its
rules are tested, and a change has the same shape as an ask, so a feature handles both alike.
Turned down: a narrow `&mut` handle in `Ctx` for one call, which would let any update change the
session during another's dispatch and make the session's rules testable only through every caller.

```rust
pub mod shared {
    /// Every change's own type implements this; the mirror of port::Request.
    pub trait Change: Into<AnyChange> {
        type Done;
        fn take(done: Done) -> Result<Self::Done, Done>;
        fn fault() -> Self::Done;
    }
    pub enum AnyChange { Session(session::Change), Vaults(vaults::Change), Settings(settings::Change) }
    pub enum Done { Session(session::Done), Vaults(vaults::Done), Settings(settings::Done) }
    /// Minted by the runtime, as AskId is.
    #[derive(Public, Copy, PartialEq, Eq)] pub struct ChangeId { pub generation: u32, pub n: u32 }

    /// A shared owner: its Msg can carry a change it is given.
    pub trait Owner: Feature {
        fn given(id: ChangeId, change: AnyChange) -> Self::Msg;
    }
}
pub struct ChangeFor<M> { change: shared::AnyChange, reply: Box<dyn FnOnce(shared::Done) -> (M, Fault) + Send> }
```

The runtime takes each `Fx::Change` out of the effects, keeps its reply under a new `ChangeId`,
and routes `Owner::given(id, change)` to the owner through `route`. The owner answers with
`Fx::Done(id, done)`, in the same dispatch or after a job of its own; the runtime calls the kept
reply and routes the message to the asker. A change may move a secret in (typed words, a
passphrase, a MuSig2 round): the change is moved, never kept by the runtime beyond its handing
over; the reply's context is `Public` (A16).

- **Rule A22.** Every change given to an owner is answered exactly once, also when refused. Held
  by the owner's `update` returning `Done` on every path (an update test per refusal) and by the
  runtime, which at a lock in debug builds asserts no change waits unanswered.
- **Rule A23.** A change's answer is dropped if its generation is not the current model's, as P9
  drops an ask's. Held by the runtime.
- **Rule A35.** `faraday-keys`' job builders and store views (`job_view`, `compare_view`, the
  signing, stretch and seal jobs' constructors) are called only in the shared owners' crates, so
  no feature builds a job that passes the session's refusals (a key held for its backup). Held by
  the application directories' `clippy.toml` (`disallowed-methods`), allowed in the owners'.
- **Rule A24.** Only an owner's crate emits `Fx::Done`; a feature that is not an owner never sees
  a `ChangeId`. Held by `lint-layers` (only crates with `owner = true` name `Fx::Done`).

The session's changes, from the inventory's uses (`features/wallets.md`, `add-key.md`,
`keygen-flow.md`, `spend-signing.md`, `files-inbox.md`, `randomness.md`, `lock.md`):

| Change | Moves in | Done |
|---|---|---|
| `AddKey` | the key as `faraday-keys`' input type (words, seed, xprv, share set) and its passphrase | `Result<KeyId, faraday_keys::Refusal>` |
| `RemoveKey`, `RemoveAll` | | `Result<(), Refusal>` |
| `AddWallet` | policy, name, source | `Result<WalletId, AddRefusal>`; `AddRefusal::Unconfirmed(Confirm)` for a wallet of several keys (F254) |
| `ConfirmWallet`, `RemoveWallet`, `RenameWallet`, `MarkChecked`, `Follow(Network)` | | `Result<_, Refusal>` |
| `Sign` | wallet, PSBT, the spend's MuSig2 round | after the session's sign job: the PSBT and the round, moved back |
| `Draw` | what for, how many bytes | `Result<EntropyBytes, NoRandomness>` |
| `Seed`, `AddOwn` | the adapter's entropy; the person's own accumulator | `()` |
| `PutOutbox`, `TakeFile`, `RemoveFile`, `RenameFile`, `MarkWritten` | the file's bytes (`Bytes`) | `Result<FileId, Refusal>` |
| `Hold`, `Release` | key and copy written | `()` |

### 7.4 Two cases worked through

**Adding a wallet from Make a new wallet.** Continue on Keys sends
`change(session::AddWallet { policy, name: None, source }, Making { ticket }, Own::Added)` and
sets the step running. The session's update checks the policy against its keys and network
(`Follow` where the session is empty) and answers. `Own::Added(making, Ok(id))` moves the flow to
Check; `Err(Unconfirmed(c))` keeps it on Keys with the confirm sheet open, whose Confirm sends
`ConfirmWallet(c)` (F254: a wallet of several keys comes in only through the sheet that shows its
keys); any other `Err` stays on Keys with the refusal shown.

**Signing with a MuSig2 round.** Sign here in Spend sends
`change(session::Sign { wallet, psbt, round: state.musig.take() }, Signing { spend, ticket }, Own::Signed)`.
The session refuses at once (`Done`) when a signing key is held for its backup or the wallet is
gone; otherwise it builds `faraday_keys::job::SignJob` from its masters (`Arc`, never a copy),
draws the round's seed from its own randomness, and returns a job whose answer becomes
`Fx::Done(id, Signed { psbt, round })`. The round's secret nonces move from Spend's state into the
job and back into Spend's state in the answer; nothing of them is kept by the session. Spend's
`Own::Signed` compares the ticket (§8.1) and keeps the round until every nonce has signed.

## 8. Worker answers

### 8.1 An answer carries its context

A job's answer, an ask's answer and a change's answer each come back as a message of the feature
that asked, carrying what the feature needs to place it: which spend, which slot, which step, and
a ticket.

```rust
#[derive(Public, Copy, PartialEq, Eq)] pub struct Ticket(u32);

/// One awaited answer of a feature; a part of its state.
pub struct Waiting { next: u32, live: Option<Ticket> }
impl Waiting {
    pub fn begin(&mut self) -> Ticket;           // a new ticket; any earlier one is stale
    pub fn take(&mut self, t: Ticket) -> bool;   // true once, for the live ticket
    pub fn forget(&mut self);                    // Back, Cancel, a stick that moves the screen
    pub fn live(&self) -> bool;                  // the step is running
}
```

- **Rule A25.** A feature that waits on an answer holds a `Waiting` per kind of answer, puts the
  ticket in the context, and acts on an answer only when `take` is true; a stale answer is dropped
  and its payload wipes as it drops. Back, Cancel and a fact that moves the screen `forget`.
  Reason: the runtime's generation (P9, A23) drops answers of an older model, and only the
  feature knows that its own flow was cancelled and begun again. Held by an update test of each
  feature that starts a job: a stale answer changes nothing.
- **Rule A26.** While a feature waits (`live`), its `refuses` takes only the presses that keep the
  screen live (Back and Cancel to leave the wait, scrolling, the eye), and refuses the rest; Back
  and Cancel are refused while a seal or the made-online check runs. This is `main`'s
  `taken_while_busy`, per feature. Held by A10 and the feature's update tests.

### 8.2 Busy lines and progress

A job's progress is reported by the work through `Compute::progress(done, of)`, which the runtime
reads into `View::running`. A feature's `present` draws its own busy line from its waiting state
and the `Running` of its `JobKind`; a `quiet` job shows none.

### 8.3 Stopping, and a lock that does not end the process

- **Rule A27.** `Fx::Stop(kind)` asks the runtime to stop the feature's job of that kind: a job
  not started is dropped (its input wipes), and a running one sees `Compute::stop_asked()` true,
  which costly work checks between its counted pieces and then returns its `Stopped` answer. A
  ticket made stale is enough for correctness; `Stop` frees the worker and ends the input sooner.
- **Rule A28.** No job's input outlives the model that made it. At a lock the runtime drops every
  job not started, asks every running one to stop, and builds the next model (in Faraday Demo,
  the Starter, or after a failed keep with keys waiting for their backup) only once every running
  job has returned and its answer has been dropped. On the device the process ends anyway. Held by
  the runtime (section 05) and a test of the runtime with a job that blocks until stopped.

## 9. Step flows

### 9.1 The trait

This answers section 08's question: the trait is **`StepFlow`**, and the impact rule
`design.flow-row` matches `impl StepFlow for`.

```rust
pub trait StepFlow: AtLock + Sized + 'static {
    /// The flow's steps: one enum, Copy, Public.
    type Step: Copy + Eq + Public;
    /// The flow's own messages (presses on a step's card, answers).
    type Own: Message;
    type Out: Message + From<Ending>;
    type Refusal: Refused;
    /// The value an answered step shows in Your choices. Public, so it can
    /// hold no secret.
    type Answer: Public;

    // What the runner and present read.
    fn steps(&self) -> Steps<Self::Step>;            // in order; whether the count is known
    fn at(&self) -> Self::Step;
    fn answer(&self, s: Self::Step) -> Option<Self::Answer>;
    fn back_stop(&self) -> Option<Self::Step>;      // the first step Back may not reach
    fn running(&self) -> bool;
    fn begun(&self) -> bool;                        // listed among flows in progress
    fn cancel(&self) -> CancelOffer;                // Now | Ask(Loss) | NotOffered
    fn width(&self, s: Self::Step) -> Width;        // Column | Wide | Full (LAYOUT §14)

    // What the runner calls.
    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (Self, FlowFx<Self>);
    fn forward(&mut self, cx: &Ctx<'_>) -> Result<FlowFx<Self>, Self::Refusal>;
    fn back(&mut self) -> Back;                     // Stepped | Leave
    fn open(&mut self, s: Self::Step) -> bool;      // a row of Your choices; false: not reachable
    fn own(&mut self, m: Self::Own, cx: &Ctx<'_>) -> FlowFx<Self>;
    fn notice(&mut self, n: &Notice<'_>, cx: &Ctx<'_>) -> FlowFx<Self>;
    fn refuses(&self, m: &Self::Own, view: &View<'_>) -> Option<Self::Refusal>;
    /// The flow is cancelled. No effects: a Cancel can order nothing.
    fn cancelled(&mut self) -> Ending;
}

pub enum FlowMsg<S, M> { Forward, Back, ForwardAgain, Open(S), Cancel, CancelSure, CancelNot, Own(M) }
pub type FlowFx<F> = Effects<FlowMsg<<F as StepFlow>::Step, <F as StepFlow>::Own>, <F as StepFlow>::Out>;

/// The state of a step flow: the kernel's part and the flow's own.
pub struct Flow<F: StepFlow> { inner: F, asking: Option<Loss>, refused: Option<F::Refusal>, again: Option<F::Step> }

impl<F: StepFlow> Feature for F {
    type State = Flow<F>;
    type Msg = FlowMsg<F::Step, F::Own>;
    type Out = F::Out;
    type Refusal = FlowRefusal<F::Refusal>;
    // update is the runner (§9.2); start, notice and refuses call F's.
}
```

**Decision: a step flow implements `StepFlow` and gets `Feature` from the kernel's one blanket
impl, whose `update` is the step-flow runner.** Reason: Back, Forward, Cancel, the hide on a step
change and the refusal at the foot are then written once and tested once, in the kernel, and no
flow can skip them. Turned down: a helper each flow calls from its own `update`, which a flow
could forget.

### 9.2 What the runner holds

- **Rule A29.** The runner:
  - refuses Back, Open and Cancel while `running()`, and Back or Open to `back_stop()` or before;
  - on Back from the first step, ends the flow with `Ending::Left`; the root returns to the page
    the flow was begun from (F77 as amended);
  - keeps the step Back left in `again`, which `ForwardAgain` (the mouse's forward button) opens;
  - on Cancel: `Now` ends at once with `cancelled()`; `Ask(loss)` keeps `asking` until
    `CancelSure` or `CancelNot`; `NotOffered` refuses. `cancelled()` has no effects, so a Cancel
    can never remove a key, a vault or a file held: those go only by a change the person asked for;
  - keeps a refusal from `forward` in `refused` until the next message, for the foot to show;
  - compares `at()` before and after every call into the flow and calls `hide()` when the step
    changed, by any way (Back, Forward, Open, an answer, a choice that moves on by itself, F103);
  - calls `hide()` at `Notice::Left` and `Notice::Hide`.
  Held by the blanket impl (a flow cannot write another `update`) and the kernel's update tests of
  the runner over a test flow.
- A step that exists only after something is made holds what it made in its own variant (below),
  so leaving the step drops it and a step before it cannot reach it.
- The flow container's layout facts are what `StepFlow` reads out: the steps and whether their
  count is known ("Step N of M" or "Step N"), the current step, each answer, which steps are
  wide or full width, where Back stops, Cancel's offer and the asking sheet. `present` words them
  and fills the step-flow template's model (section 04); the template alone places the card, the
  pane and the foot (K8). No geometry is in a flow's state.

### 9.3 Worked example: Make a new wallet

Inventory `features/create-wallet.md`. The cards a flow lists are Kind, Quorum (multisig) or
Backup form (single key), Keys and Check; `main`'s other card numbers are never reached and have
no step here.

```rust
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Step { Kind, Quorum, BackupForm, Keys, Check }

pub struct MakeWallet {
    draft: Draft,       // kind, m and n, path, account, backup form, slots: public answers
    at: At,
    pass: PassField,    // the passphrase field part: the one holder of a typed passphrase (A3)
    adding: Waiting,    // the session's answer to AddWallet
    confirm: Option<session::Confirm>,        // F254's sheet, open
}

enum At {
    Kind,
    Quorum,
    BackupForm,
    Keys { making: Option<KeyOnCard>, xpub: Option<TextField> },
    /// The wallet is made. Kind and Quorum are not steps any more: the
    /// draft's shape cannot be edited, since no message reaches it here.
    Check { built: WalletId },
}

pub enum Refusal {
    NoKind, BackupFormUnanswered, SlotEmpty(u8), SlotLater(u8), VaultStickPending,
    Cosigner(CosignerRefusal),     // origin missing, other path, other script, not one key, the same key twice
    Session(session::AddRefusal),
    Practice,
}
```

- `steps()` lists Kind, then Quorum for a multisig kind or Backup form for a single key, then Keys
  and Check; the count is known once the kind is chosen.
- `forward()` on Kind refuses `NoKind` while no row is ticked; on Backup form, `BackupFormUnanswered`;
  on Keys it checks every slot (an empty or Later slot refuses with its slot's own line), refuses
  while a vault's stick is pending, then sends `AddWallet` (§7.4) and the step is `running()`
  until the answer. On Check it sends `MarkChecked(built)` and outs `Checked { wallet }`, or
  `MoveMoney { from }` where money is being moved.
- `back_stop()` is `Some(Keys)` once `at` is `Check`: Back stops at Check, as on `main`. Take back
  (a path change) is a press on Check that sends `RemoveWallet(built)` and, on its answer, returns
  to Keys with the slots of keys removed since the make cleared.
- `cancel()` is `Ask(loss)` while a passphrase is typed or a key made on the card is kept nowhere,
  `Now` otherwise. A key made on the card was added to the session by `AddKey` and stays there.
- `AtLock`: `holds().typed` while the passphrase field holds text or a key is being made on the
  card; `asks_first` while the wallet's keys are kept nowhere (main's `lock_held_on`); `hide`
  closes the eye and the words shown on the card; `at_lock` drops the whole state.
- A key made on the card is the New key flow's making, so the state machine of making a key (its
  entries, the words made, the quiz) is a part both features embed (A2), not a feature Make a new
  wallet depends on (A5).

## 10. Refusals and the wording

```rust
/// A typed value the person is told about. Implemented in the feature's
/// `present` module, by an exhaustive match. Object-safe: a `Note` boxes one;
/// the `Public` bound is on `Note::new` and on `Refused`.
pub trait Worded: Send + 'static {
    fn words(&self, s: &'static Strings) -> Line;
}
pub enum Line { Fixed(&'static str), Filled(Text) }   // Filled: faraday_strings::fill of a template

/// A refusal: worded, and how a control it refuses is drawn.
pub trait Refused: Worded + Public {
    fn drawn(&self) -> Drawn { Drawn::Dimmed }      // Dimmed | AsUsual
}
```

- **Rule A30.** Every refusal is a variant of a typed enum: the feature's `Refusal`, whose
  variants wrap the domain's (`faraday_keys::Refusal`, `faraday-intake`'s answer, `faraday-psbt`'s
  errors) and the port's `Failure`. An update forms no user-facing text: it returns typed
  refusals and notes, and the `state`, `msg` and `update` modules never name `faraday_strings`.
  Text that goes into a file is made by the domain function that makes the file (section 01,
  §2.2). Held by `lint-layers`' module rule and `lint-strings` (V4).
- **Rule A31.** `Worded` is implemented by an exhaustive `match` with no wildcard arm, so a new
  refusal fails to compile until it is worded. Held by clippy's `wildcard_enum_match_arm`, denied
  in `present` modules by `lint-layers`.
- The kernel words `port::Failure` once (`Worded for Failure`), so a failed read or write reads
  alike everywhere. A domain refusal the domain already words (section 01, §2.2) passes its text
  through as `Line::Filled`.
- A refusal shown at the foot of a step (§9.2), beside a slot, or as a note are the same value;
  where it shows is `present`'s.

## 11. Practice

`Mode` is fixed for a model's life:

```rust
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Mode { pub network: Network, pub practice: bool, pub online: bool }
```

The root picks the mode a model starts in (Faraday Demo and the Starter's Demo: Testnet,
Practice, online); a person's change of mode is kept in the carry and takes effect at the lock it
asks for, as on `main` (inventory `features/settings-time.md`, `features/practice-demo.md`).

- **Rule A32.** Practice makes no secret a person could take away. A feature whose press would
  make one returns `Refusal::Practice` from `refuses` when `view.mode.practice`; `route` then
  gives one note and nothing else, and `present` draws the control dimmed, or as usual for an eye
  or a Show button (`Drawn::AsUsual`). The presses `main` refuses carry over, each in its feature:
  Seed XOR parts; SLIP-39 in New key; the backup flow's Make for GPG and Secure Boot keys; the
  backup flow's Restore; every BIP-85 output; Vanity's passphrase; Lightning's show, vault and
  out; Silent payments' scan-key and descriptor outputs; Convert's source; a vault opened from
  those screens (`features/practice-demo.md`). Held by an update test per press.
- **Rule A33.** The key boundary fails closed behind the gate: in Practice the session's key store
  takes only the published samples, and on Mainnet it refuses a sample by every way a key comes
  in (`faraday_keys::Refusal`). A feature that forgot its gate still makes no secret. Held by
  `faraday-keys` and its tests, as on `main`.
- The passphrase field part offers the `faraday` chip only when `view.mode.practice`, and not on
  the KeePass export prompts, whose minimum it is under.
- Transfer is offered only when `view.mode.online`.

## 12. Writing a file that may hold a secret

A feature forms every write as one ask carrying the file's whole bytes: `StorageAsk::Write`,
`HostAsk::Save`, `Ask::Print` and `Ask::Keep`. The port has no append and no partial write, so a
write in pieces cannot be formed. Every file's bytes may hold a secret (P14).

- **Rule A34.** A file that may hold a secret is written whole or not at all: the adapter writes
  it under a temporary name in the same folder, reads it back (P11), then renames it to its name;
  a write that fails removes what it wrote at once, zeroing it first where the medium keeps what
  was removed (FAT), before it answers. The feature counts a file written (a take-away, a copy of
  a backup, `MarkWritten`) only on `Wrote` with the name and SHA-256 it asked for. Held by the
  port's types (one ask, whole bytes), the storage and host adapters, and an adapter test of a
  write that fails part-way (section 07). Section 02 is asked for the adapter's half below.

## 13. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| A9, A10 | one gate, `refuses`, for update and drawing | `route`; `lint-layers`' module rule; update tests |
| A11, A12 | every state field says what a lock does; nothing held after `at_lock` | the `AtLock` derive; a runtime assertion; update tests |
| A13 | a screen left drops what it held | `notice` with no default; update tests |
| A14 | only the derive makes a type `Public` | the derive; the compiler; `lint-secrets` |
| A15 | an event settles before the next | the runtime; a runtime test |
| A16 | replies hold a `Public` context and `fn`s | private fields; `clippy.toml` on `into_parts` |
| A17 to A19 | `Ctx` and `View`, abilities, no `Compute` in `View` | the types; `costs-lint` |
| A20, A21 | composites route, never call a sibling | A9; the derive |
| A22 to A24 | a change is answered once, by its owner | update tests of the owners; the runtime; `lint-layers` (`owner`) |
| A25, A26 | stale answers dropped by ticket; presses while waiting | update tests |
| A27, A28 | stopping jobs; no job input outlives its model | the runtime; a runtime test |
| A29 | Back, Cancel, hide and refusals of every step flow | the blanket impl; the kernel's runner tests |
| A30, A31 | refusals typed; worded exhaustively in `present` | `lint-layers`; `lint-strings`; clippy |
| A32, A33 | Practice refuses at the gate and at the key boundary | update tests; `faraday-keys` |
| A34 | a secret file written whole or not at all | the port's types; the adapters; an adapter test |
| A35 | only the owners build `faraday-keys`' jobs and views | `clippy.toml` |

## Changes asked of earlier sections

- Section 01, §4.1: the sketch of `AtLock` becomes `holds`, `hide` and `at_lock` (§2.3 here);
  `Effects` becomes one ordered list of `Fx` (§4.1), with `Change`, `Done`, `Stop`, `Then` and
  `Note` beside orders, asks, jobs and outs; `Ctx` is `View` plus `Compute` (§5); `Feature` gains
  `start`, `notice`, `refuses` and `Refusal` (§2.1). Reason: the runtime needs `holds` and `hide`
  before a seal, P18 needs one order across kinds, and the gate must be one function.
- Section 01, §4.3, A8: name the three shared owners (the session, the vault composite, the
  settings) held by the root beside its app composite, read through `View`'s kernel traits and
  changed only by a `ChangeFor` (§7). Reason: A7 and the borrow of `Ctx` while a sibling is
  updated rule out the session as a child of the device composite.
- Section 01, §5, the module table: `state`, `msg` and `update` never name `faraday_strings`
  (A30). Reason: an update forms no text; wording is `present`'s.
- Section 01, §1 metadata: add `owner = true` for the crates of the three shared owners (A24).
- Section 02, §4.1 and P11: the storage and host adapters write every file under a temporary name,
  read it back, then rename; a write that fails removes (zeroes, on FAT) what it wrote before it
  answers, and its `Failure` says when a temporary file could not be removed, so the app can name
  it (A34). Reason: a file that may hold a secret is written whole or not at all.
- Section 02, §2.2: `AskFor`'s shape is §4.2 here (the erased reply built only by `new` and
  `map`, and `into_parts` for the runtime).

## Questions for later sections

- Section 04: the step-flow template's model filled from `StepFlow` (§9.2), the memo of a
  `present` (A18), and how a note's words reach the snackbar organism.
- Section 05: the runtime's serving of changes (§7.3) beside asks; the order of a lock (`hide`,
  `Stop` and the wait of A28, the seals, `at_lock`, `Ask::Keep`, `Exit`); `Carried` and how each
  feature's `start` reads it and what it keeps; the log of `Public` messages and their replay.
- Section 06: each root's model (the three owners and the app composite), its `Page` enum and
  navigation, and how each stream fact becomes a feature's message.
- Section 09: the feature crates, which features share one, the owners' crates, and the part that
  holds the making of a key (§9.3).
