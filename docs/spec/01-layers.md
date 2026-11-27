# The layers and their dependency rules

**Kind:** Spec
**Open:** Not built.

This section fixes the layers of `next` named in `docs/plan/ARCHITECTURE-PLAN.md` §2: the
domain, the port, the application, presentation in its two stages, the UI kit by atomic level,
the runtime, the adapters and the composition roots. For each layer it says what it holds, what
it may depend on, what it may never depend on, and which rule holds that: the compiler, a crate
boundary, an attribute, a clippy configuration or a named lint. It says where key material may
live and how it crosses from one layer to the next, which of `main`'s crates fall in each layer,
and which of their edges are cut. It is written for whoever writes the skeleton manifest
(section 09) and for anyone who later adds a crate, a module or a dependency. Crates are named by
role here; their exact names are the manifest's. `main`'s crates are named as they are on `main`.

## 1. The layers in one view

From the bottom up. A layer may depend on the layers below it in the column "May depend on" and
on nothing else.

| Layer | What it holds | May depend on | Never depends on |
|---|---|---|---|
| Domain | Cryptography, Bitcoin formats, codecs, the key boundary, vault and file formats, the input parsers, the key-form registry, the wording | Other domain crates (no cycle), outside crates that pass `cargo vet` | Every other layer |
| Port | The one contract between app and shells: events in, effects out, the frame out | Domain | Kit, application, runtime, adapters, roots |
| UI kit, tokens to templates | Faraday's look, by atomic level; no app types (`Ui<A>` is generic over the action) | Lower kit levels, port value types, domain | Application, runtime, adapters, roots |
| Application kernel | The shared shapes every feature uses: `Ctx`, `Effects`, `Message`, `Public`, `AtLock`, the step-flow machine, the handles | Port, domain | Kit, features, runtime, adapters, roots |
| Presentation kernel (`role = "present"`) | `Present`, `Page`, `ShowFlow`, the step-flow fill and the helpers every `present` shares (section 04 §2) | Kernel, port, domain, the passphrase part (for `pass_model`, through its read view, A3), the kit's templates and organisms | Features, other parts, runtime, adapters, roots |
| Application parts | State machines a feature embeds and owns (a text field, the passphrase field, the scanner sheet's state) | Kernel, port, domain | Features, kit, runtime, adapters, roots |
| Application features | One feature each: `State`, `Msg`, `update`; and its presentation, `present` and its pages (§5) | Kernel, presentation kernel, parts, port, domain; its `model`, `present` and `page` also the kit's templates and organisms | Another feature (but a composite's children, §4.3), the kit below organisms, runtime, adapters, roots |
| Product crates (`role = "product"`) | One per product: the runtime's `Root` for that product, its app composite's frame and the assertion of L1 (§7.2) | Runtime, composites, features, parts, the kernels, the kit's templates and organisms, port, domain | Adapters, roots |
| Runtime | The model's owner: queue, effect dispatch, worker pool, memo, note queue, motion store, lock and restart, the frame's hit targets | Kernel, port, kit atoms (the `Ui` and its frame), domain | Any feature or composite, the presentation kernel, product crates, adapters, roots |
| Adapters | The shells' I/O: the stick's framebuffer, input and disk client, the desktop window, sticks as folders, the camera, the scanner, the disk and grant processes, the Starter's operating-system side | Other adapters (no cycle; not a separate process, S2), port, domain, kit tokens | Kit above tokens, application, runtime, roots |
| Composition roots | One binary per product (the device app, Faraday Demo, the Starter) and the tour root | Every layer above | Another root, a tool (but as a dev-dependency, §7.3, and the tour root, `ships = false`, through a normal edge, T1) |
| Tools and test support | Bakers, lints, cross-check servers, cost measures, test allocators, fixture writers | Any library crate | A root; nothing that ships depends on a tool (§7.3) |

Presentation's two stages are modules of each feature crate (§5), and its second stage is the
kit's pages level (§6). The traits that join a feature to the kit, and the step-flow fill, are in
one application crate of their own, the presentation kernel (section 04 §2), since the kernel may
not name the kit. Only a root and a tool depend on a product crate.

**Decision: a layer is declared in each crate's manifest, and one lint checks every edge.** Each
crate carries

```toml
[package.metadata.faraday]
layer = "domain"     # domain | port | kit | app | runtime | adapter | root | tool
level = "organisms"  # kit only: tokens | atoms | molecules | organisms | templates
role  = "feature"    # app only: kernel | present | part | feature | composite | product
owner = true         # app only: the crate of a shared owner (section 03 §7, A24)
online = true        # a crate only an online product holds: Transfer, the strip, the samples, the window and desktop adapters (section 06, S8)
offline = true       # root only: the device root, whose resolved graph holds no `online` crate (section 06, S8)
ships = false        # root only: the tour root, which never ships (section 06, S5 and S16)
checks = "app"       # tool only, for a tool crate that holds tests: the layer they check (section 07 §4)
```

`lint-layers` reads `cargo metadata --format-version 1 --no-deps --locked`, refuses a workspace
crate without a `layer`, and checks every normal, build and dev edge between workspace crates
against the table above and the rules of §2 to §8. It reports every refused edge in one run.
Reason: the layer is a fact about the crate and changes with it in the same commit. Turned
down: a table of crate names in the lint's own file, which drifts from the workspace as crates
are added.

## 2. The domain

### 2.1 What it holds and what it may not do

- **Rule D1.** A domain crate holds no I/O, no UI and no app types. Its library code (`src/`
  outside `#[cfg(test)]` modules) names none of `std::fs`, `std::net`, `std::process`, `std::env`,
  `std::thread::spawn`, `std::time::SystemTime::now` and `std::time::Instant::now`; its tests,
  in-source test modules and examples may, to read fixtures and to time. Time and randomness reach
  the domain as arguments. Held by `lint-layers`: no edge above the domain, and its io rule, which
  reads those names over each domain crate's library code. Not by the domain's `clippy.toml`:
  clippy's `disallowed_methods` is one lint, and allowing it for a test would allow the mint and
  keys sets too (09 §1.4), in files D3 forbids editing.
- Two domain crates on `main` do I/O, and that part moves out:
  - `faraday-files` keeps its rules over the `Place` trait (listing, reading, writing with
    read-back, vault write-back, names); its folder `Place`, the one part over `std::fs`, moves to
    the desktop adapter. Its FAT `Place` is over `faraday-fat` and stays. So the Starter's
    features still name `faraday-files` (its `fat_text`).
  - `faraday-selftest` keeps the vectors; its check that the device's root is mounted read-only
    (a read of the kernel's mount list) moves to the stick adapter, which gives the answer to the
    self-test as an argument.
- **Rule D2.** A crate that is `no_std` on `main` stays `no_std`, whatever its layer: in the
  domain `faraday-text`, `faraday-crypto`, `faraday-bip`, `faraday-psbt`, `faraday-entropy`,
  `faraday-selftest`, `faraday-kef`, `faraday-pgp`, `faraday-fat`, `faraday-ots`, `faraday-sb`,
  `faraday-vault`, and `faraday-codec` without its `decode` feature; the port; and the kit's
  crates made from `faraday-draw`. Held by the attribute itself and by `lint-layers`, which reads
  `no_std = true` from the crate's metadata and checks the attribute is present.
- **Rule D3.** Domain crates are copied from `main` with their own tests and are changed only to
  cut the edges in §2.3 and move out the I/O above. A copied test takes its header (section 07
  §3) when it is next changed, and a header added then is not a change this rule forbids. One
  further change is made: `faraday-crypto`'s `Compute` gains the `stop` and `progress` hooks
  beside `count` (`Compute::new` takes `Hooks`; section 05 §5.2), function pointers as `count` is,
  so the crate stays `no_std` and the token `!Send`. The functions section 10 ports into a domain
  crate from `faraday-core` are also allowed: file classification into `faraday-intake` and the
  decode view into `faraday-psbt::transaction` (10 §9). Each is new code with headed tests, not an
  edit of copied code. Held by review against `main` at
  `d89dde5de`; section 10 names how.
- **Rule D4.** A domain crate's dev-dependencies are domain crates and test-support tools only
  (§7.3). Its tests are published vectors and outside implementations, as on `main`.

### 2.2 The wording is a domain crate

`faraday-strings` and `faraday-guide` are domain crates. They are data, hold no app types and do
no I/O. `faraday-strings` depends only on `faraday-text`, `faraday-crypto` and `faraday-bip` (it
names their errors' wording), and `faraday-guide` only on `faraday-strings`. Held by
`lint-layers` (the two crates' metadata name `wording = true`; a wording crate may depend only on
`faraday-text`, `faraday-crypto`, `faraday-bip` and the other wording crate).

**Decision: domain crates may name the wording.** On `main`, `faraday-plan`, `faraday-forms`,
`faraday-paper`, `faraday-intake`, `faraday-files`, `faraday-core-wallet` and `faraday-keys`
use `faraday-strings`: a printed sheet, a file's text and a refusal's text are
worded where they are made. Reason: the rule that every user-facing string lives in the strings
crate holds either way, and keeping these crates as they are keeps them as reviewed. Turned down:
every domain crate answering in enums only, with all wording done in presentation, which would
rewrite tested domain code for no change in what depends on what.

### 2.3 Edges cut from main's domain

Each cut moves the type that crosses to the crate where it belongs, so that the edge has no
reason left.

| Edge on `main` | What crosses | Where it goes |
|---|---|---|
| `faraday-keys` to `faraday-intake` | `secret_buf::SecretText`, `ROOM` | `faraday-text`, beside the wiping `Text` it wraps; every crate that used it from intake takes it from there |
| `faraday-keys` to `faraday-intake` | `wallet::Refusal` | `faraday-keys`, as the key boundary's refusal (its variants are about a key or wallet joining the session: duplicate, wrong key, stick attached, not allowed in Practice). `faraday-intake` keeps its own answer for what it parses, and the application maps either one to wording |
| `faraday-keys` to `faraday-forms` | `Makes`, returned by `identity::Material::makes` | `faraday-keys` owns the kinds of material it holds. `faraday-forms` depends on `faraday-keys` for that enum and maps each form to it |
| `faraday-vault` to `faraday-intake` | `SecretText`; `is_otpauth` | `SecretText` to `faraday-text`; the `otpauth://` scheme test to `faraday-codec`, which both crates already use |
| `faraday-core-wallet` to `faraday-intake` | `SecretText`; `wallet::network_name` | `SecretText` to `faraday-text`; `network_name` is wording, to `faraday-strings` |
| `faraday-paper` to `faraday-draw` | the print layout's named dimensions in `tokens` | the kit's tokens crate (§6.2), which a domain crate may name (§6.3) |
| `faraday-samples` to `faraday-core` | the samples' types | Practice's samples are data over domain types only; the feature that loads them is in the application |

**Decision: the registry of key forms points at the key boundary, not the other way.** Reason:
the boundary is the audited surface (`docs/KEYS.md` on `main`), and a change to a form's name,
typing or wording must not reach it. Turned down: `faraday-keys` depending on the registry as on
`main`, which puts a crate that depends on the wording beneath the key boundary.

After the cuts: `faraday-keys` and `faraday-vault` depend on no input parser and not on the
registry. Held by `lint-layers`: their metadata names `boundary = true`, and a boundary crate may
not depend on `faraday-intake`, `faraday-forms` or any crate that depends on either.

## 3. The port

- **Rule P1.** One crate, grown from `main`'s `faraday-shell-api`, is the whole contract between
  the app and every shell: events in (touch, keys, display, sticks and files, camera frames,
  clipboard, time, randomness), typed effects out (storage, camera, clipboard, print, vibrate,
  exit), the frame out. A worker job is not a port type: it is the kernel's `Job<M>`, which the
  runtime takes from `Effects::jobs` itself, since it owns the pool; the port's effects go through
  the root to the adapters. Section 02 gives its types.
- **Rule P2.** The port depends on domain crates only, for the value types its events and effects
  carry: the wiping text and secret types, a file's bytes, a display's facts. Held by
  `lint-layers`.
- **Rule P3.** No application crate depends on an adapter or a root, and no adapter depends on
  the application or the runtime. The root joins them: it hands the runtime's effects to the
  adapters and the adapters' events to the runtime. Held by `lint-layers`.
- **Rule P4.** The kit may name the port's value types (`Display`, `Orientation`, `Point` and
  `Medium`: a display's density, size and kind, which `faraday-draw` reads on `main`; and
  `Anchor`, the name the app gives a place it draws, which the kernel's `ViewOrder` and the kit's
  `Ui` both name, section 04 §8.1) and never its events or effects. Held by `lint-layers` with the port's metadata naming its value module
  (`values = "display"`); a kit crate's use of the port is checked to name only that module.

## 4. The application

### 4.1 The kernel

The kernel crate holds the shapes every feature uses and nothing feature-specific. Sketch (section
03 gives the full shapes):

```rust
/// Every feature implements this; the root composes them.
pub trait Feature {
    type State: AtLock;          // never Clone (§8.4)
    type Msg: Message;
    /// What the feature tells its parent; the parent routes it (§4.3).
    type Out: Message;
    /// Why a message is refused before `update` sees it (section 03 §2.2).
    type Refusal: Refused;
    fn start(carried: &mut Carried, cx: &Ctx<'_>) -> (Self::State, Effects<Self::Msg, Self::Out>);
    fn update(state: &mut Self::State, msg: Self::Msg, cx: &Ctx<'_>) -> Effects<Self::Msg, Self::Out>;
    fn notice(state: &mut Self::State, notice: &Notice<'_>, cx: &Ctx<'_>) -> Effects<Self::Msg, Self::Out>;
    fn refuses(state: &Self::State, msg: &Self::Msg, view: &View<'_>) -> Option<Self::Refusal>;
    /// At a lock, after the seal: what the next build of the scope needs (section 05 §7.4).
    /// The default carries nothing.
    fn carry(state: &mut Self::State, into: &mut Carried) {}
}

/// One ordered list: P18 keeps the order across kinds (section 03 §4.1).
pub struct Effects<M, O>(Vec<Fx<M, O>>);
pub enum Fx<M, O> {
    Order(port::Order),    // to the adapters, through the root; not answered
    Ask(AskFor<M>),        // answered: the runtime keeps each reply (section 02 §2)
    Change(ChangeFor<M>),  // to a shared owner (section 03 §7)
    Done(ChangeId, shared::Done), // only an owner: the answer to a change
    Job(Job<M>),           // to the runtime's worker pool
    Stop(JobKind),         // stop this feature's running job of this kind
    Then(M),               // to this same feature, before the next event
    Out(O),                // to the parent
    Note(Note),            // a short typed line the frame shows
    View(ViewOrder),       // to the runtime's motion store; not answered (section 04 §8.2)
}

/// A worker job: it owns its inputs (a secret moved in, never copied)
/// and its answer is a message carrying its own context.
pub struct Job<M>(Box<dyn FnOnce(&Compute) -> M + Send>);

/// Read-only: what `present` reads (section 03 §5). Built by the root.
pub struct View<'a> { /* the shared owners' reads, the port facts, the mode, the clock, ... */ }
/// What `update` reads: the view, and the token for costly work.
pub struct Ctx<'a> { pub view: View<'a>, pub compute: &'a Compute }

pub trait Message: Send {}
/// Sealed: implemented only by the kernel's derive, which requires every
/// field to be Public. No secret type implements it (§8.3).
pub trait Public: Message {}

/// What a lock wipes, declared by every state (as `AtLock` on main).
pub trait AtLock {
    fn holds(&self) -> Holds;   // what it holds now: the clean marker, the Lock sheet, the idle lock
    fn hide(&mut self);         // every secret on show off the screen, what was typed kept
    fn at_lock(&mut self);      // wipe; called before the model is dropped
}
```

- **Rule A1.** The kernel holds no feature's types, and a type two features both need (a key's
  place, a `VaultId`, a wallet's handle) is a handle in the kernel or a domain type, never a type
  of one of the features. Held by `lint-layers` (the kernel may not depend on a feature) and by
  the compiler (a feature cannot name a sibling's types, §4.3).

### 4.2 Parts

A part is a state machine a feature embeds: the passphrase field (`pass_field` on `main`), a text
field and its caret, the word grid, the scanner sheet's state, the ways in to a key. It has a
`State`, a `Msg` and an `update` like a feature, and it answers only its owner, through its return
value. It never routes, never names a feature and never returns an `Out`. A part may expose a
read view for its owner's `present`: the passphrase field's gives its dots, its eye and its chips,
and its text only while the eye is open. Only the part's crate names the part's state.

- **Rule A2.** A part depends on the kernel, the port, the domain and other parts, with no cycle
  (the ways in embed the text field and the word grid; the making of a key embeds the word grid
  and the passphrase field). Held by `lint-layers` (`role = "part"`, and its cycle check).
- **Rule A3.** The passphrase field part is the only holder and filter of a typed passphrase
  (`docs/KEYS.md` §2 on `main`; inventory `features/secrets.md`). Held by the rule `main` keeps
  in `lint-components` (a passphrase field drawn or held outside `pass_field`), carried into
  `lint-layers`' module rule: only the part's crate names the passphrase field's state type; a
  feature's `present` reads the field through its read view alone.

### 4.3 Features, composites and routing

- **Rule A4.** A feature crate holds one feature of the inventory (or a few that share one state,
  named in the manifest). It depends on the kernel, the presentation kernel, parts, the port and
  the domain, and its `model`, `present` and `page` on the kit's templates and organisms (§6.4).
- **Rule A5.** A feature crate never depends on another feature crate. Held by the crate boundary
  and `lint-layers`: an edge from `role = "feature"` to `role = "feature"` is refused.
- **Rule A6.** A composite is a crate whose state holds its children's states. It depends on its
  children, receives each child's `Out` and turns it into a message for another child, or into
  its own `Out`. It is the only way one feature reaches another. Held by `lint-layers`: only
  `role = "composite"` and `role = "product"` crates depend on a feature, and only roots and
  tools depend on a product.
- **Rule A7.** A feature's `State` keeps its fields private to its crate. A composite holds a
  child's state, passes it to the child's `update` and `present`, and reads nothing in it. Held by
  the compiler, and by `lint-layers`, which refuses a `pub` field on a type that implements
  `AtLock` outside the kernel.
- **Rule A8.** What features share to read (the keys and wallets the session holds, the sticks
  attached, the mode, the clock) reaches them only through `Ctx` and `View`, which are sets of
  shared borrows. Three shared owners hold what is shared: the session, the vault composite and
  the settings, each a top state held by the runtime beside the app composite (never a child of
  it; section 05 §1) and read through `View`'s kernel traits. A change to one is a `ChangeFor` effect the runtime serves to
  that owner, answered once (section 03 §7). Held by the compiler (`&Ctx` gives no `&mut`) and by
  `lint-layers` (`owner = true`).

**Decision: features are crates.** Reason: the compiler then holds "a feature reaches another only
through its parent", which `main` held by no rule (inventory `features/session-pane.md`,
`features/scanner-qr.md` and `features/app-shell.md` list reaches across areas into each other's
state and functions). Turned down: features as modules of one application crate under a lint,
which is `main`'s layout and its cross-area reach.

**Decision: areas exchange only messages** (overturning `main`'s choice not to; the plan's
"Overturned"). The scanner's dispatch to the feature that asked (inventory `features/scanner-qr.md`,
"Dispatch order"), the session pane's openings (`features/session-pane.md`), the lock's call into
every area (`features/lock.md`) and Practice's refusals (`features/practice-demo.md`) each become
an `Out` routed by a composite or the root. Reason: one way in to each feature's state makes its
`update` the one place its rules are tested (section 07). Turned down: features calling one
another's functions over shared state, as on `main`, which makes a rule of one area breakable
from any other.

**Decision: the vault area splits into use cases** (overturning `main`'s one vault module). The
vault features are separate crates under one vault composite: the list and making a vault,
unlocking, inside a vault, sealing at a lock, one-time codes and GPG keys (inventory
`features/vault-list-create.md`, `vault-unlock.md`, `vault-contents.md`, `vault-seal.md`,
`totp-codes.md`, `gpg-keys.md`). Reason: each use case is then tested by its own updates. What
`main` gained from one module is kept by the domain: every decrypted vault byte stays in
`faraday-keys`, behind its reveals, so an audit of what a vault holds reads `faraday-keys` and
`faraday-vault`, not the features.

**Decision: the ratchets on `Faraday`'s field count and on the files that may hold an
`impl Faraday` are dropped with that struct.** Reason: they counted what one struct gathered, and
there is no such struct; a feature's reach is now held by A5 to A8. Turned down: a ratchet on
the number of fields of the root's model, which would count composition, not reach.

## 5. Presentation, in two stages

Each feature crate has these modules. The module split is the presentation layer.

| Module | Holds | May name | Never names |
|---|---|---|---|
| `state`, `msg`, `update` | the feature's state, messages, update | kernel, parts, port, domain | `present`, `model`, `page`, any kit crate, `Show`, `Memo`, `faraday_strings` (any path, `EN` included), `Worded::words` (section 03, A30) |
| `model` | the feature's `ScreenModel` and the kit models it fills | kit model types (§6.5), domain value types, wording | `state`, `update`, the kit's drawing |
| `present` | `present(&State, &View, &Show) -> Model` | `state`, `model`, kernel, presentation kernel, domain, wording | `update`, `page`, `Ui`, the kit's drawing, `Compute` |
| `page` | `draw(&Model, &mut Ui<'_, Press>)` | `model`, `msg`, the presentation kernel, the kit's templates and organisms | `state`, `update`, `present`, `View`, `Show`, `faraday_strings`, the kit below organisms |

Section 04 §2.2 gives these additions with the rule that keeps the wording crate from being
re-exported under another path.

Sketch (section 04 §2 gives the full shapes), both traits in the presentation kernel:

```rust
pub trait Present: Feature {
    type Model<'s>;
    fn present<'s>(state: &'s Self::State, view: &View<'s>, show: &Show<'s>) -> Self::Model<'s>;
}
pub trait Page: Present {
    /// What a press on this page sends.
    type Press: Pressed;
    fn draw(model: &Self::Model<'_>, ui: &mut Ui<'_, Self::Press>);
}
```

- **Rule V1.** `present` does all the working out of what a screen shows; `draw` reads the model
  and paints. `draw` is given no state, no `Ctx`, no `View` and no `Show`, so it can read nothing
  else. Held
  by the compiler (the signature) and by `lint-layers`' module rule (the table above), which
  checks each module's `use` paths and path expressions.
- **Rule V2.** `present` is given no `Compute` (`View` has none) and so cannot do costly work.
  Costly work is done in `update` with `Ctx`'s token or in a job, and its result is state.
  Held by the compiler and `costs-lint` (who takes and mints the token, as on `main`). This
  settles the inventory's doubt "Drawing writes state or does work" (README, and
  `features/drawing-compute.md`, `features/pane-container.md`, `features/session-pane.md`,
  `features/add-key.md`, `features/codex-shares.md`, `features/silent-payments.md`,
  `features/address-check-explore.md`, `features/spend-backup.md`): a value worked out per frame
  on `main` becomes state written by `update` (or a `Derived` kept by its inputs), or a cheap
  derivation kept in the memo. A measure an update decides on comes back as a message; one read
  only by the next frame is the motion store's (section 04 §7.2, §8.1).
- **Rule V3.** Hit targets are the page's output, held by the runtime for the frame (§7.1). No
  model and no state holds a rectangle a page drew. Held by the compiler: the `Ui` records its
  hit targets in the `FrameOut` it returns to the runtime (section 04 §7.1), and a page cannot
  reach state.
- **Rule V4.** Every user-facing string a model carries comes from the wording crate. Held by
  `lint-strings` (as on `main`).

**Decision: the stages are modules, not crates.** Reason: a feature's stages share its `Msg` and
model types, and the signature already keeps `draw` from state; three crates per feature would
add a crate boundary that holds nothing the signature does not. Turned down: a crate per stage
per feature.

## 6. The UI kit by atomic level

### 6.1 The levels

| Level | Holds | On `main` |
|---|---|---|
| tokens | named dimensions, colours, durations, type roles; no code but constants and their helpers | `faraday-draw`'s `tokens` |
| atoms | the canvas, surface, text, shapes, fonts, icons, the `Ui<A>` frame and hit registration; buttons, chips, switches, fields | `faraday-draw`, `faraday-ui`'s controls and frame |
| molecules | rows, list items, chip sets, the passphrase field's drawing, the appearance rows | `faraday-ui` |
| organisms | the session pane, sheets and dialogs, menus, the snackbar, ways-in, the navigation rail and bar, the scanner's viewfinder, the on-screen keyboard's panel | `faraday-ui`, `faraday-core`'s `screens/session_pane/`, `ways_in` |
| templates | the pane container, the step-flow layout, the menu page | `faraday-core`'s `screens/hub_layout/`, the flow container, the menu page container |
| pages | one per screen: each feature's `page` module (§5) | `faraday-core`'s screens |

### 6.2 The rules

- **Rule K1.** Each level from tokens to templates is one crate (or more, named in the manifest,
  each with one level). A level depends only on lower levels, and on a crate of its own level
  only where the manifest names the edge, with no cycle (the atoms on `faraday-draw` and
  `faraday-atoms-derive`, so that `faraday-draw` stays `no_std`). Held by `lint-layers` (`level`).
- **Rule K2.** The tokens crate depends on nothing. A new dimension is a name in it, as on `main`
  (`lint-tokens`).
- **Rule K3.** No kit crate names an app type. A component is generic over the action it returns
  (`Ui<A>`), and an organism or template takes its own model type, defined in the kit. Held by
  `lint-layers` (no kit edge to the application) and by the compiler.
- **Rule K4.** A kit crate may name the wording crate (a dialog's Cancel, a key's label) and the
  guide (its blocks), and domain value types it draws (a QR code's modules from `faraday-codec`).
- **Rule K5.** The kit owns no state across frames but what a component's motion needs, kept in
  the motion store: a kit atoms type the runtime owns and lends to each frame's `Ui`, keyed by
  the component's `Anchor` (section 04 §8.1).

### 6.3 Who may name the tokens

**Decision: the tokens crate is the one kit crate the domain and the adapters may depend on.**
`faraday-paper` lays out its printed sheets with named dimensions, and the camera adapters size a
preview with one (`faraday-scanner`'s `PREVIEW_MAX`). Reason: a dimension is named once, in the
tokens. Turned down: copying the values into each crate, which keeps two places for one value.
Held by `lint-layers`: an edge from domain or adapter to the kit is refused unless it is to
`level = "tokens"`.

### 6.4 What a page uses

- **Rule K6.** A page uses only templates and organisms, and the `Ui` the templates hand it;
  `present` names the kit's model types and never its drawing. Held by Cargo (a feature crate's
  kit dependencies are the templates and organisms crates only), by `lint-layers` (a feature
  crate's edge to a kit crate below organisms is refused), and by its module rule (only `page`,
  `model` and `present` name the kit, and only `page` its drawing).
- **Rule K7.** A template or organism re-exports from a lower level only what a page needs to
  call it: the `Ui` type, the action type's bounds and its own model's field types. Held by
  `lint-layers`, which lists each kit crate's `pub use` of a lower level and refuses one not
  named in the crate's metadata (`reexports = [...]`).
- **Rule K8.** Step flows draw on the step-flow template, menu pages on the menu-page template,
  and a page's panes are placed only by the pane container (inventory `features/flow-container.md`,
  `features/menu-pages.md`, `features/pane-container.md`). What the session holds is drawn only
  by the session pane organism (`features/session-pane.md`). Held by K6 (there is nothing else to
  draw them with) and by the templates' and organisms' own signatures.

### 6.5 Kit models

An organism or template that shows structured content takes a model the kit defines: the session
pane takes a `SessionPaneModel<'s, A>` of rows, each a label, a value, an icon and the action its
press returns. A feature's `present` fills it; its `page` passes it.

Every text a kit model holds is a `Shown<'s>`, a kit atoms type whose variants are private
(section 04 §11). `Shown::plain` takes only a `&'s Text` or a `&'static str`; `Shown::secret`
takes a `SecretText`'s text, a reveal's text or the passphrase part's read view. So a revealed
secret a model shows is a borrow of a reveal guard (§8.2), marked secret, and a borrowed secret
cannot be made plain.

## 7. The runtime, adapters, roots and tools

### 7.1 The runtime

- **Rule R1.** The runtime is generic over the product (`Runtime<R: Root>`) and names no
  feature. `Root` is the runtime's own trait, which each product crate implements (section 05
  §1). The runtime owns the model, the queue, the dispatch of effects, the worker pool, the memo,
  the motion store, the lock and restart, and the last frame (its `FrameOut`). It never depends
  on the presentation kernel: it calls `Root::frame`, whose arguments are the kernel's `View` and
  `Show` and the kit atoms' `Ui`. Held by `lint-layers`.
- **Rule R2.** The runtime is the only shipped code that mints `Compute`, in event handling and
  on a worker. Test code and tools (`layer = "tool"`) may mint one with `Compute::unchecked`
  under `#[expect(clippy::disallowed_methods, reason = "...")]`, as on `main`, so that a pure
  update test can build a `Ctx` and a tool can call a costly operation. Held by `clippy.toml`'s
  `disallowed-methods` on `Compute::new` and `Compute::unchecked`, and by `costs-lint`, which
  refuses a mint in a shipped crate's non-test code outside the runtime (`docs/COMPUTE.md` on
  `main`).
- **Rule R3.** The runtime may spawn threads (the worker pool) and keeps no other I/O: its
  directory's `clippy.toml` refuses `std::fs`, `std::net`, `std::process` and `std::env`.
- **Rule R4.** The last frame, its call list and its hit targets are the runtime's ordinary
  read-only output, which the root paints through an adapter. A tour reads the same output.
  No app or kit crate carries a hook for tours. This replaces `main`'s hooks in non-test code
  (`press`, `where_offered`, `drawn_texts`, `drawn_targets`, `drawn_ink`; inventory
  `features/starter-app.md`, `features/tour-runner.md`). The tour reads the drawn text from
  `FrameOut::ink`, recorded only once `Runtime::record_ink` turns it on; `record_ink` is in
  `clippy.toml`'s `disallowed-methods` outside the tour root, the leak crate and test support.

### 7.2 Adapters and roots

- **Rule S1.** An adapter turns the port's effects into I/O and I/O into the port's events. It
  depends on the port, the domain, the kit's tokens and other adapters (the window on the camera
  and the scanner, each capture back end on the shared camera crate, as on `main`). Held by
  `lint-layers`; Cargo refuses a cycle.
- **Rule S2.** The disk process and the grant helper (`faraday-disk`, `faraday-grant` on `main`)
  are adapter binaries of their own, with the privileges `docs/SECURITY-MODEL.md` gives them on
  `main`. They depend on the domain only, and no crate depends on them: the root's adapters
  speak to them over a pipe. Held by `lint-layers` (`layer = "adapter"`, `process = true`: no
  edge to the port, and no edge into them).
- **Rule S3.** A composition root is a binary. Its product crate builds the model (the `Root`
  and the composites it holds); the binary holds only the command line, the adapters it runs and
  the loop that starts the runtime (section 06 §1). It never depends on another root. Held by
  `lint-layers` (no edge from `root` to `root`).
- **Rule S4.** The device app, Faraday Demo and the Starter are each their own root over their
  own product crate: the device product, the Demo product and the Starter product. The tour root,
  which renders tours for review, builds the three product crates' models and has none of its
  own; its metadata names `ships = false`, and it never ships.

**Decision: the Starter holds Faraday Demo by composing the same library crates, not by
depending on the Demo's root.** The device app's features are composed by a library composite
(the device composite). The online composite holds it beside the Demo's own features (the strip
at the window's foot, from `main`'s `faraday-host`, the samples, and Transfer); with its frame it
is the online app, which the Demo product holds. The Starter has one model: Faraday Demo is its
lock scope, the session and vaults owners and the same online app, held by the runtime as top
states that exist only while the scope is built. The Starter's own composite (`Root::Outside`)
and its settings owner stand beside the scope, outside it, on the same desktop adapters (sticks
as folders, the window). A lock inside the Demo drops and rebuilds the scope alone in the
Starter's process, as `restart_demo` does on `main` (section 05 §7.9; inventory
`features/starter-app.md`, `features/practice-demo.md`). Reason: the Starter carries Faraday Demo,
both products then share library crates only, and the routing between the strip, Transfer and the
device's features is written once. Turned down: the Demo as a child state of the Starter's
composite, which puts the Starter's own state inside what a lock drops; and the Starter starting
the Demo as a second process, which needs a second binary shipped beside the Starter and a
channel between them.

Today's edges this removes: `shells/starter` to `faraday-demo` (normal) and `faraday-starter` to
`faraday-demo` (dev); `faraday-starter`, `shells/starter`, `faraday-host`, `shells/window`,
`faraday-storage` and `shells/demo` to `faraday-core`. The Starter's use of `faraday-core`'s
printed blank sheets and codex32 sizes (`wallet_backup::sheet_blank`, `codex32_backup::printed`,
inventory `features/starter-app.md`, "Uses from `faraday_core`") moves to `faraday-paper`,
where printed sheets are made.

### 7.3 Tools and test support

- **Rule T1.** A tool may depend on any library crate of any layer. No root that ships and no
  crate it depends on (through normal or build edges) depends on a tool; the tour root
  (`ships = false`) runs on the fixture adapters and the test kit, both tools. Held by
  `lint-layers` over the resolved graph of each root that ships.
- **Rule T2.** A crate's dev-dependencies are crates of its own layer or below, test-support
  crates (the test allocators, the fixtures), and nothing above. An update test is pure (section
  07) and needs no adapter; a journey test lives in a root's tests. Held by `lint-layers`.
- **Rule T3.** A build-dependency follows the table of §1 as a normal one does.

## 8. Key material across the layers

### 8.1 Where it may live

| Layer | May hold key material | How |
|---|---|---|
| Domain | Yes: `faraday-keys` holds it (`docs/KEYS.md` §2 on `main`); the wiping types of `faraday-crypto` and `faraday-text` carry transient secrets | As on `main`; `faraday-keys` is the only crate whose functions operate on it |
| Port | Only in transit, in wiping types: a pasted text, a file's bytes read from a stick, a camera frame, a file's bytes to write | The port's event and effect types use the wiping types for these fields |
| Application | In state, only in `faraday-keys` types (the key store, an open vault) and the wiping types (typed words before Done, a typed passphrase in its part) | A message moves one; it never copies one |
| Presentation | Only a borrow of a reveal guard, for one frame | §8.2 |
| Kit | Only borrowed text for one frame | `Shown<'s>` from the model, a secret marked by `Shown::secret` (§6.5) |
| Runtime | Only what the model it owns holds, what moves through its queue and jobs, and the carry it holds between the drop and the next start or Keep | It drops the model at a lock (§8.4); the carry's secret bytes are the Inbox's and Outbox's alone (section 05 §7.3) |
| Adapters | Only in transit, as the port's types | Written or read, never kept |

- **Rule K-1.** Key material lives in `faraday-keys` and leaves it only through its named reveals
  (`Reveal`, F70 on `main`). Held by `faraday-keys`' API and `lint-leak-reveals`, as on `main`.
- **Rule K-2.** No layer above the domain defines a type that holds a secret. A feature holds one
  only in a `faraday-keys` type or a wiping type. Held by `lint-secrets` (its names and types, as
  on `main`), applied to every application, kit and runtime crate.
- **Rule K-3.** Every secret type is not `Clone`, so a message, a state or a job cannot copy one.
  Held by the compiler.

### 8.2 How it crosses

- **Into the key boundary:** by move. A feature's update calls a `faraday-keys` operation through
  the session (§4.3, A8: a message routed to the feature that owns the key store), giving it the
  wiping value it held; the call consumes it.
- **Into a job:** by move into the job's closure; the answer, if it carries a secret, moves it
  back in its message. Held by `Job<M>: Send` owning its inputs and K-3.
- **To the screen:** `present` may take a reveal guard from the key store through `View` and put
  it, or text borrowed from it, in the model. The model borrows from state for `'s`, so the guard
  lives no longer than the frame: the runtime drops the model after `draw`. Held by the compiler
  (the lifetime) and by R5 below.
- **To a stick or a print:** only an effect built from a reveal whose reason names that writing
  (on `main`: the backup flow's words or SeedQR, an unprotected file to the Outbox), carrying a
  wiping type. Held by `faraday-keys`' reveal list.
- **Into a log or a replay:** never. Only `Public` messages are logged or replayed, and no secret
  type is `Public`. Held by the compiler (the sealed trait and its derive).
- **Rule R5.** The memo keeps only `Public` values: `Memo::get`'s key, input and value are each
  bound `Public` (`K: Public`, `I: Public`, `V: Public`; section 04 §6), so no borrow and no
  secret is kept, and a reveal is never memo input. Held by the compiler (the memo's bounds).

### 8.3 Messages

- **Rule M1.** A message carries a handle (a key's place, a `VaultId`) or moves a secret type; it
  never carries a copy. Held by K-3.
- **Rule M2.** Every message type implements `Message`; one that the runtime may log or replay
  implements `Public` through the kernel's derive, which fails to compile when a field is not
  `Public`. Held by the compiler.

### 8.4 Lock

- **Rule L1.** The model is never `Clone`. Held by a compile-time assertion in each product
  crate over its model's types (its top states).
- **Rule L2.** Every feature's state implements `AtLock`, saying what a lock wipes. At a lock the
  runtime seals the vaults, then calls `Feature::carry`, then `at_lock`, and then drops the model
  and restarts (section 05 §7.2). Held by the kernel's `Feature` bound. Section 05 gives what carries over a restart (on `main`, the Inbox and Outbox kept by the
  storage adapter; inventory `features/lock.md`, `features/storage.md`).

## 9. Main's crates by layer

Main's 53 workspace crates at `d89dde5de` (the vendored `rqrr` is outside them), by the layer
each takes in `next`.

| Crate on `main` | Layer in `next` | What changes |
|---|---|---|
| `faraday-text`, `faraday-crypto`, `faraday-bip`, `faraday-psbt`, `faraday-codec`, `faraday-entropy` | domain | Copied. `faraday-text` gains `SecretText` and `ROOM`; `faraday-codec` gains the `otpauth://` test; `faraday-crypto`'s `Compute` gains the `stop` and `progress` hooks (`Compute::new(Hooks)`, D3) |
| `faraday-selftest`, `faraday-handwriting`, `faraday-boot`, `faraday-payload` | domain | Copied; the self-test's read of the mount list moves to the stick adapter (§2.1) |
| `faraday-keys`, `faraday-vault` | domain, `boundary = true` | Copied; edges to `faraday-intake` and `faraday-forms` cut (§2.3); `faraday-keys` gains `Refusal` and `Makes` |
| `faraday-pgp`, `faraday-sb`, `faraday-kef`, `faraday-ots`, `faraday-plan`, `faraday-fat`, `faraday-sqlite`, `faraday-core-wallet`, `faraday-qr` | domain | Copied; `faraday-core-wallet`'s edge to `faraday-intake` cut |
| `faraday-files` | domain | Copied; its folder `Place` moves to the desktop adapter (§2.1) |
| `faraday-intake` | domain | Copied; loses `SecretText`, `ROOM`, `Refusal`, `is_otpauth` to the crates above |
| `faraday-forms` | domain | Copied; depends on `faraday-keys` for `Makes` |
| `faraday-paper` | domain | Copied; its tokens from the kit's tokens crate; gains the Starter's printed blanks |
| `faraday-strings`, `faraday-guide` | domain, `wording = true` | Copied; `faraday-strings` gains `network_name` |
| `faraday-samples` | domain | Its edge to `faraday-core` cut; data over domain types |
| `faraday-shell-api` | port | Grown into the port (section 02) |
| `faraday-draw` | kit: tokens (its `tokens`) and atoms (the rest) | Split in two crates by level |
| `faraday-ui` | kit: atoms, molecules, organisms | Copied, then arranged by level |
| `faraday-core` | application (kernel, presentation kernel, parts, features, composites, product crates) and the kit's organisms and templates | Ported, not copied; its tests are not ported (the plan's §1) |
| `faraday-starter` | application: the Starter's features, its composite and the Starter product | Ported like `faraday-core`; its edges to `faraday-core`, `faraday-host` and `faraday-demo` cut |
| `faraday-host` | application: the Demo's strip feature, in the online composite; its window-holding part to the runtime and the desktop adapter | Ported; it reaches the device composite only through messages the online composite routes (it calls into `Faraday` on `main`, inventory `features/desktop-shells.md`) |
| `faraday-storage` | adapter | Re-pointed at the port |
| `faraday-scanner`, `faraday-camera` (`shells/camera`), `faraday-v4l2`, `faraday-mediafoundation`, `faraday-window` | adapter | Re-pointed at the port; `faraday-window`'s edge to `faraday-core` cut |
| `faraday-disk`, `faraday-grant` | adapter, `process = true` | Copied |
| `shells/stick` (`faraday-stick`) | root: the device app, and its stick adapter | Re-pointed at the runtime and the device product |
| `shells/demo` (`faraday-demo`) | root: Faraday Demo, and the desktop adapter for sticks as folders | Re-pointed at the runtime and the Demo product; the adapter part is a library the Starter's root shares |
| `shells/starter` (`faraday-starter-shell`) | root: the Starter, and its operating-system adapter | Re-pointed at the runtime and the Starter product; its edge to `faraday-demo` cut (§7.2) |
| `shells/snapshot` (`faraday-snapshot`) | root: the tour root, never shipped (`ships = false`) | Re-pointed at the three product crates and the runtime's frame output (R4) |
| `fontbake`, `costs-lint`, `op-cost`, `crosscheck`, `faraday-testkit`, `faraday-alloc-count`, `faraday-leak-scan` | tool | Copied or re-pointed; `op-cost`, `crosscheck` and `testkit` lose their edges to `faraday-core` and name the domain or the features they measure |

## 10. unsafe code

- **Rule U1.** The workspace sets `unsafe_code = "forbid"`. Every crate inherits the workspace
  lints (`[lints] workspace = true`) except the crates on a closed list, which set
  `unsafe_code = "deny"` and allow it on the one item that needs it. Held by `lint-layers`, which
  refuses a crate that does not inherit the workspace lints unless it is on the list in its own
  file.
- **Rule U2.** The list, carried from `main`: `faraday-crypto` (`pinned.rs`: page-aligned memory
  and `mlock`), the storage adapter (on Windows, the free memory for a vault's Argon2id), the
  grant helper, the V4L2 and Media Foundation camera adapters, the desktop window adapter
  (clipboard and the system library loading), the Starter's operating-system adapter, the stick adapter (`main`'s stick shell: the ioctls of its input,
  touch and file code), and the two
  test allocators. No crate of the application, the kit, the port or the runtime is on it.
- **Rule U3.** A new entry is a security trade-off and needs a recorded decision.

## 11. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| §1 | every crate declares its layer; every edge allowed | `lint-layers` over `cargo metadata` |
| D1 | no I/O in the domain's library code | `lint-layers` (no edge above; the io rule over `src/` outside `#[cfg(test)]`) |
| D2 | `no_std` kept | the attribute; `lint-layers` |
| D4, T2, T3 | dev and build edges | `lint-layers` |
| §2.2 | the wording's own dependencies | `lint-layers` (`wording`) |
| §2.3 | the boundary crates' cuts | `lint-layers` (`boundary`) |
| P1 to P4 | the port is the one contract | `lint-layers` |
| A1, A2, A5, A6 | kernel, parts, no feature-to-feature edge, composites | the crate boundary; `lint-layers` (`role`) |
| A3 | one holder of a typed passphrase | `lint-layers`' module rule (from `main`'s `lint-components`) |
| A7 | a state's fields are private | the compiler; `lint-layers` |
| A8, V1, V2 | `Ctx` is read-only; `draw` sees only its model; `present` has no `Compute` | the compiler (signatures); `lint-layers`' module rule; `costs-lint` |
| V3 | hit targets are the frame's | the compiler (the `Ui`'s `FrameOut`) |
| V4 | strings in the wording crate | `lint-strings` |
| K1 to K7 | the kit's levels and what a page uses | `lint-layers` (`level`, `reexports`); `lint-tokens` |
| R1 to R4 | the runtime is generic; only it mints `Compute`; tours read its frame | `lint-layers`; `clippy.toml`; `costs-lint` |
| S1 to S4 | adapters and roots | `lint-layers` |
| T1 | nothing shipped depends on a tool | `lint-layers` over each root's resolved graph |
| K-1 to K-3, M1, M2, R5 | key material and messages | `faraday-keys`' API; `lint-leak-reveals`; `lint-secrets`; the compiler (`!Clone`, `Public`, lifetimes, the memo's bound) |
| L1, L2 | the model is not `Clone`; every state says what a lock wipes | a compile-time assertion in each product crate; the kernel's bound |
| U1 to U3 | `unsafe` only on the closed list | the workspace lints; `lint-layers` |

## Questions for later sections

- Section 02: how the worker-job effect is typed in the port, given that the runtime takes it
  (P1) and §4.1's sketch keeps jobs apart from the port's effects.
- Section 02: which of the port's event fields carry wiping types (§8.1); the camera frame's
  pixels in particular.
- Section 03: how a feature asks the key store for an operation that changes it (signing with a
  MuSig nonce, adding a key): a message to the session's feature through the root, or a narrow
  `&mut` handle in `Ctx` for that one call.
- Section 03: what `Ctx` and `View` hold, field by field.
- Section 04: where a component's motion state lives between frames (K5), and how the frame's
  call list is cleared of text after it is painted.
- Section 05: what carries over a restart (the Inbox and Outbox, the settings) and where it is
  kept while the model is dropped; and the Demo's and the Starter's lock, which rebuild the model
  in their process (inventory README, "Faraday Demo's lock was not traced").
- Section 09: the exact crate names, the directory per layer (each with its `clippy.toml`), and
  the feature crates: which inventory features share one crate (A4), and which composites hold
  them.
