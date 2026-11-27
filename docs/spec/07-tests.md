# The test architecture and the test-header lint

**Kind:** Spec
**Open:** Not built.

This section makes `docs/plan/ARCHITECTURE-PLAN.md` §3 exact. It names the kinds of test `next`
has, the layer that owns each kind of rule, and where each kind of test lives and what it may
drive. It gives the header every test opens with (Invariant, Regression, Owner) and
`lint-test-headers`, the lint that holds it. It turns `main`'s rule on what a test never states
into checkable rules, and says where the cross-checks, the fuzz targets, the leak tests, the
tours' own checks and the cost checks sit. It answers the questions sections 02, 03 and 08 left
for it. It is written for whoever writes a test, the test-support crates, the leak crate, the tour
root's checks or the lint, and for the writers of sections 05 (the runtime), 06 (the roots) and
09 (the manifest). Rules are numbered TS1 onward. Sources: the plan's §3; `main`'s `CLAUDE.md`
("Everywhere", "Security scope"), `docs/SECRET-LEAKS.md`, `docs/M3-CHECKS.md` §2 and §3,
`docs/SPEED.md` §7, `docs/CROSSCHECK.md` §10, decisions F250 and F281, at `d89dde5de`, and F336; the
inventory's `features/test-kit.md`, `tour-runner.md`, `tours.md`, `tooling.md`, `secrets.md`,
`lock.md`, `check-page.md`, `tools.md`, `drawing-compute.md` and `fixtures.md`.

## 1. The principle

- **Rule TS1. One invariant, one test.** A test holds one invariant: one thing that must stay
  true. It may assert several facts (the state and the effects of one message) when they are
  the same invariant seen twice. Two invariants are two tests. Held by the header (§3), which
  names one invariant, and by review.
- **Rule TS2. The layer that owns a rule tests it, and no other layer does.** A rule held by a
  feature's `update` is tested by an update test of that feature, never again through a journey
  or a tour. Held by `lint-test-headers` (§4): a test's Owner is the layer of the crate it is in.
- **Rule TS3. A test is added only for a plausible regression:** a security rule, a refusal, a
  rule that crosses features, or a bug that happened. Never for a code path another test or the
  compiler already holds. Held by the header's Regression line (§3), by the duplicate check of
  §4 and by review.
- **Rule TS4. Old tests are not a source.** `main`'s tests of `faraday-core` (and of the other
  ported crates, `faraday-starter` and `faraday-host`) are not ported and their assertions are
  not read for guidance (the plan's §1). Their fixtures and test data carry
  (`docs/migration/inventory/fixtures.md`). Invariants come from the documented rules (the
  security model, keys and lock, the standards, the decisions, every refusal the code makes), from
  the specification of each feature as it is ported, and from `main`'s bug fixes and the outside
  audits' findings. Held by review.
- **Rule TS5. A test is never deleted or weakened to make the check pass** (section 08, §3.6). A
  test is removed only when the rule it holds is gone or another test names its invariant, and
  the commit says which. Held by review and the duplicate check (§4).

**Decision: a test states its invariant in words, in a header the lint reads.** Reason: the plan
asks that every test say what it holds and why it could break, and a header read by a lint is
the one form a duplicate can be found in. Turned down: invariants listed in a document beside the
tests, which drifts from the tests it names.

## 2. The kinds of test, by owner

### 2.1 The table

Each kind belongs to one layer of section 01, and the Owner of its header is that layer's name.

| Owner | Kind | Where it lives | What it drives | What it never uses |
|---|---|---|---|---|
| `domain` | vector test | the domain crate's `tests/` or unit tests | the crate's functions, on published vectors, frozen outputs of an outside implementation, and the refusals of its parsers | any other layer |
| `kit` | kit test | the kit crate's unit tests | one component's `Ui<A>` call and the hit targets and actions it records (§2.3) | an app type; a size, colour or position compared with a design value |
| `app` | update test | the module it tests: `update`'s tests in a feature, part, composite, owner or product crate (`role = "product"`, section 01 §1); the runner's in the kernel | `route`, `start`, `notice` and `refuses`, with a test `Ctx` (§2.2); `present` for a rule it holds (§2.2) | drawing, the runtime, an adapter, a shell |
| `runtime` | runtime test | the runtime crate's tests | `Runtime<R>` over a test root of test features, events in, effects and frames out | an adapter; a product's features |
| `adapter` | adapter test | the adapter crate's tests | `Adapters::carry` and `poll` over a stand-in for its I/O (§2.4) | the application, the runtime |
| `root` | journey; the tours' run | the root crate's tests | the root's model over scripted adapters through the port (§2.5, §5) | a feature's internals; a geometry assertion |

The port holds types and no behaviour (section 02, P6), so nothing is owned by it: what its
types promise is held by the compiler. A tool holds no product rule; a tool crate that holds
tests (the leak crate, §6) declares the one layer it checks (§4, TS13).

**Decision: adapter tests are a kind of their own, owned by the adapter layer.** This answers
section 02's question. The rules 02 gives the adapters (a write whole or not at all, P11; the
order of a family, P19; the timeouts, P23; the frame copies wiped, P13; the capture buffers,
P28) are held in the adapters' code, and only a test that drives an adapter can reach them.
Reason: an update test cannot see an adapter, and a journey that waits for a 120 s timeout or
fails a write half-way is not a smoke test. Turned down: folding them into journeys, which would
make the journeys the many slow tests the plan keeps them from being.

**Decision: what sections 02 and 03 call "an update test of the runtime" is a runtime test.**
P9 (a stale or wrong answer), A15 (an event settles before the next), A23 (a stale change's
answer dropped) and A28 (no job input outlives its model) are runtime tests. No earlier section needs a change for this.

### 2.2 Update tests

An update test gives a state and a message and reads the state and the effects that come back.

- **Rule TS6.** An update test sends its message through `route`, never through `update`
  directly, so the gate (A9, A10) is part of what it tests. It builds its `Ctx` from the
  application's test support (below), answers an ask or a change by calling the reply the
  effect carries, and runs a job's work inline with the test `Compute`. Held by review and by
  A9's module rule (only the feature's own crate names its `update`).
- **Rule TS7.** An update test asserts on state through what the feature exposes (`holds`,
  `refuses`, `present`'s model, the effects), not on private fields it reaches by a test-only
  accessor. Reason: the test then fails only when behaviour a person or a parent would notice
  changes. Held by A7 (fields are private) and review.
- **Rule TS8.** A test of `present` asserts only a rule `present` holds: a refused control is
  marked refused in the model (A10), a hidden secret is not in the model after `Notice::Hide`,
  a value worked out in `update` is the one shown. It never compares a model's text with a
  string of the wording crate, nor a model's layout with the design. Held by review.

The application's test support is a crate of `layer = "tool"` (section 01, §7.3), a
dev-dependency of the application crates only. It holds:

```rust
/// A View for a test: an empty or loaded session, vaults and settings,
/// and Facts for a named product (device, Demo, Starter).
pub struct TestView { /* owned session::Read, vaults::Read, Settings, Facts, Mode, Clock */ }
impl TestView {
    pub fn device() -> Self;
    pub fn demo() -> Self;                         // Practice, Testnet, online
    pub fn with_key(self, words: TestSeed) -> Self; // through the session owner's AddKey
    pub fn with_wallet(self, kit: TestWallet) -> Self;
    pub fn ctx(&self) -> Ctx<'_>;                  // with the test Compute
}
/// Minted with Compute::unchecked under expect (R2); counts costly work by kind.
pub fn test_compute() -> &'static Compute;
/// What an update returned, by kind, in order.
pub struct Seen<M, O> { /* orders, asks, changes, jobs, thens, outs, notes */ }
pub fn seen<M: Message, O: Message>(fx: Effects<M, O>) -> Seen<M, O>;
/// Answers an ask or a change as an adapter or an owner would.
pub fn answer<M: Message>(ask: AskFor<M>, a: port::Answer) -> M;
pub fn done<M: Message>(change: ChangeFor<M>, d: shared::Done) -> M;
```

The test support is outside the application directories, so it may call `into_parts` (A16),
which the application's `clippy.toml` refuses in the features. Its fixtures are the kept test
data (`fixtures.md`): the ten test wallets, the test seeds, the test sticks' files, the release
sums. `lint-keys-tests`, re-pointed (section 08, §8.1), holds that no test-support crate or test
feature is in the normal graph of anything a root ships.

### 2.3 Kit tests

A kit test is allowed only for a rule a person would notice and a tour cannot show: a press
inside a component's drawn target returns its action; a control with no press records no target; a
component's keyboard focus moves in drawing order; the passphrase field's drawing shows no
character of its text with the eye off. It reads the hit targets and actions `Ui<A>` records,
never a rectangle's size or place against a value.

**Decision: the kit is tested by the tours, and by kit tests only for input.** Reason: how a
component looks is reviewed on its rendered tour (the plan's §3); how it answers a press is
behaviour no picture shows. Turned down: snapshot comparisons of components, which assert
geometry and fail on every intended change of the look.

### 2.4 Adapter tests

An adapter test builds one adapter over a stand-in for what it talks to, carries effects into
it and reads its events.

- **Rule TS9.** Every adapter rule of section 02 has its adapter test, against a stand-in that
  fails as the real thing can:
  - P11: a `Place` (`faraday-files`' trait) that fails after a given number of bytes, and one
    whose removal fails; the test sees no file under the asked name, the temporary file zeroed
    and removed, or `Failure::LeftBehind` naming it.
  - P19: asks of one family carried in order and answered in order.
  - P20, P23: a disk process stand-in that never answers; every waiting ask is answered `Silent`
    at its timeout, and a late answer is dropped by its sequence number. The test runs on the
    adapter's clock, which the stand-in advances; it never sleeps.
  - P13: the stick adapter's converted and last-shown bands, painted with a planted pattern, are
    blank after `Order::Exit`. It runs under the leak allocator (§6), in a test binary of its own.
  - P28: the capture adapter's buffers are zeroed before a stream's end gives them back. The
    capture adapters take their buffers from a source behind a trait, so that a test supplies the
    memory and reads it after the stream ends; the V4L2 source maps the kernel's buffers.
  - P29: every `camera::On` is preceded by `CameraFact::Listed` and answered, also when no camera
    starts.
  Held by these tests.
- An adapter copied from `main` (the disk process and the grant helper, S2; the capture crates)
  keeps its own tests under §4.1.

**Decision: adapter tests run on the adapter's own clock.** Reason: a timeout test that sleeps
makes the suite as slow as its longest timeout. Turned down: shortening the timeouts in tests,
which tests a different number from the one shipped.

### 2.5 Journeys

A journey runs one root's model, its runtime and its features as the product composes them, over
scripted adapters (fixture sticks, fixed randomness, a scripted clock and input), through the
port's `Adapters`. It is a smoke test: it asserts that the run reaches its end and that the one
effect that ends it is carried (a signed PSBT written to the fixture stick, the carry kept at a
lock).

- **Rule TS10.** A journey asserts no rule a lower layer owns. Each product has a few: the device
  app (start, a test key typed, a PSBT signed from the fixture stick, a lock that keeps its
  carry and restarts); Faraday Demo (start in Practice on Testnet with the samples, a lock that
  rebuilds the model in process); the Starter (Faraday Demo opened as the lock scope of its
  model, and a lock that rebuilds only the scope, leaving the Starter's own composite and its
  settings owner as they were). The rules that are a root's own are held here: `Start`
  first and once (P17), a stick written only when asked (P27). Held by the header (Owner
  `root`) and review.

Section 06 is asked that each root build its model over any `Adapters` value, so that a journey
gives scripted adapters to the same construction the product runs.

## 3. The header

### 3.1 Its form

Every test function opens with a doc comment whose first three lines are the header, in this
order:

```rust
/// Invariant: A vault whose passphrase was just set ends locked and is unlocked once with it.
/// Regression: Create hands the open vault to Inside a vault, skipping the unlock.
/// Owner: app
#[test]
fn a_new_vault_ends_locked() {
    // ...
}
```

- **Invariant:** one sentence, in the present tense, saying what must stay true, in words a
  person using Faraday or reading its rules would use. It names the rule, not the code: "A key
  held for its backup is refused for signing", not "`sign` returns `Err`".
- **Regression:** one sentence naming the plausible change that would break it. Where the
  invariant comes from a fix in `main`'s history, it may end with the fix's commit on `main`
  in brackets; it names no tracker item.
- **Owner:** one of `domain`, `kit`, `app`, `runtime`, `adapter`, `root`.
- Further lines of the doc comment are free.

The header applies to every `#[test]` function, to each case of a property test, and to a
test-support function that a test macro expands into tests (it carries the header of the tests
it makes, with the case named in the Invariant). A table-driven test over one vector file holds
one invariant ("Words, seed and master key agree with BIP-39's published vectors").

**Decision: the header is a doc comment, not an attribute.** Reason: it reads as prose where the
test is, needs no proc-macro crate in every test's dependencies, and `rustdoc` keeps it. Turned
down: a `#[invariant("...")]` attribute, which adds a macro to each test crate for a check a
script can do.

### 3.2 What a test never states

`main`'s rule ("A test never states a design rule, a token's value, a library's or the
language's own behaviour, or a comparison of a screen's geometry against the design document")
carries as four rules:

- **Rule TS11.** No test states a token's value or a design rule. A test does not name the
  kit's tokens crate. Held by `lint-test-headers` (§4, check 7). A design rule is held by the
  tokens, the kit's types and the reviewed tours.
- **Rule TS12.** No test states a library's or the language's own behaviour (that `zeroize`
  zeroes, that a `Vec` grows, that `serde` reads a field). A test of Faraday's use of a library
  states what Faraday does with it. Held by review: the Regression line must name a change to
  Faraday's code.
- **Rule TS13.** No test compares a screen's geometry with the design document: no assertion on a
  rectangle, a size, a colour or a position of a frame or a model. The tours are reviewed; the
  two counted checks over them (§5.2) are lints, not tests. Held by review, and by TS8 and §2.3.
- **Rule TS14.** No test states a string's wording. A test of a refusal asserts the refusal's
  variant (A30), never its words, so rewording a line breaks no test. Held by A30 (an update
  forms no text) and review.

**Decision: these are rules of the test, not of the review alone.** Reason: TS11 can be checked
by a script, and the others are what a reviewer reads a test against (section 08, §8.4). Turned
down: a lint that guesses geometry from numbers in assertions, which would refuse a count.

## 4. lint-test-headers

This answers section 08's question on the lint's name and form.

**Decision: the lint is `lint-test-headers`, a script of `tools/`, run by `just lint`.** It reads
every Rust file of the workspace's crates (`src/` and `tests/`) and the layer metadata from `cargo
metadata --format-version 1 --no-deps --locked`, and reports every failure in one run (`lint-all`).

It fails on:

1. A test function (`#[test]`, or a property-test case) whose doc
   comment does not start with the three lines of §3.1 in order.
2. An Owner that is not one of the six names.
3. An Owner that is not the layer of the crate the test is in (`layer` in its metadata), or, in a
   crate of `layer = "tool"`, not the layer its metadata names in `checks = "<layer>"`. A tool
   crate with tests and no `checks` fails.
4. An Invariant named twice in the workspace: two Invariant sentences equal after their case is
   folded, their whitespace collapsed and a final full stop dropped.
5. An empty Invariant or Regression, or a Regression equal to its Invariant by the same compare.
6. A test file on the list of copied tests (§4.1) that has a header in every test and is still on
   the list, or a listed file that does not exist.
7. A test function that names the tokens crate (TS11).
8. A test marked `#[ignore]` without `reason = "..."` naming the recipe that runs it (as the
   touch-target walks are on `main`). An ignored test is counted only by that recipe.

What it cannot see: two invariants that say the same thing in different words, a test that holds
two invariants under one sentence, a Regression that is not plausible. These are the reviewer's
(section 08, §8.4: "a test that names an invariant its layer does not own").

**Decision: the duplicate check compares normalised sentences, not meaning.** Reason: a script
can hold exact repeats, which is what grows when tests are copied from one another; a paraphrase
needs a reader. Turned down: a similarity score, which fails on near phrasings of different rules
("refused in Practice" for two different presses).

### 4.1 The copied tests

The domain crates, the kit's crates, the disk process, the grant helper and the capture adapters
are copied from `main` with their own tests (section 01, D3), and those tests have no header.

**Decision: the copied test files are listed in `test-headers.copied` in `tools/`, a list that
only falls.** A listed file is not checked by checks 1 to 5. A change that edits a listed file for
another reason gives each of its tests the header and takes the file off the list in the same
change. Held by `lint-test-headers` (check 6) and by a new impact pair, `tests.copied-header`:
a change to a listed file changes the list. Reason: D3 changes a copied crate only to cut its
edges, and a header written in the copy is a change no review of the copy can check against
`main`. Turned down: writing every header in the copying change, which mixes thousands of new
lines into the change whose review is "the same as `main`".

The list's paths are of `next`'s tree, one per line; a file is never added to it after the copy.

## 5. The tours

### 5.1 What a tour is

A tour is a script of `Input` and fact events run by the tour root (section 01, S4) over its
fixture adapters (section 02, §10), with a picture saved at each named step. A person reviews
the pictures; a screen change names the Material page it was checked against (section 08, §2).
A tour has no assertion on what a screen shows.

- **Rule TS15.** A tour reads only the runtime's frame output (R4): the frame's pixels, its
  drawn text with its boxes (`FrameOut::ink`, through the runtime's `Seen::ink`), its hit targets,
  and the kept images, which are the runtime's alone (`Seen::kept`: a sheet's frost, a fade's
  frame). No app or kit crate carries a hook for tours. Held by R4 and `lint-layers`.
- **Rule TS16.** Every tour runs to its end. A test of the tour root (Owner `root`) runs each
  tour at one small size with no pictures and fails on a tour that stops early; this is the
  check of the scripts themselves, and the screens check of §5.3 runs in the same pass. Held by
  that test, in `just`.

A tour's exceptions and counted presses name a press by its message's kind (`Message::kind()`,
section 03 §3.1, such as `"create.Forward"`), never by a screen position.

### 5.2 The touch-target ratchet carries

This answers section 08's question.

**Decision: `lint-touch-targets` carries, as a lint over the tours, not a test.** The tour root
walks every tour at the four sizes of `docs/M3-CHECKS.md` §2.3 on `main` with no pictures, reads
each frame's hit targets from the runtime's output, and counts the press targets under the
Material 3 floor (48 dp, and 44 dp on the compact and short layout, scaled by the display's
density) by the rules of M3-CHECKS §2.2: a target cut by its own clip is not counted, a side a
neighbour trimmed is. The exceptions of M3-CHECKS §3 are named by message kind. The counts per
size, screen and message kind are held to `tools/touch-targets.counts`, which only falls; a raise
is a commit that says why, and the impact rule `structure.ratchet-raised` keeps these counts.
Reason: Material's minimum target is an accessibility rule a reviewer cannot measure on a
picture, and the hit targets are already the runtime's ordinary output, so the check needs no
hook. It is a lint because it compares with Material's floor, not a layer's rule, and TS13 keeps
geometry out of tests. Turned down: dropping it for review of the pictures, which cannot see a
target 4 dp short; and a test, which TS13 forbids. `next`'s counts file starts empty: every
target is at the floor unless a commit says why it cannot be.

### 5.3 The screens check carries

The screens check of `docs/SECRET-LEAKS.md` §5 on `main` carries into the tour root, unchanged in
what it checks. At the frame after a lock, a hide, leaving a page and leaving a sheet, the frame's
text is read against every text form of the planted test secrets, and every kept image whose
frame showed one and that is still kept after the event is a hit. A hit fails the run (TS16).

- **Rule TS17.** The events the check knows are the runtime's marks (a lock, a hide, and
  `Mark::Hides` for a hiding press) and a change of `FrameOut::page` or `FrameOut::sheets`
  between two frames (leaving a page, a sheet closed). A hiding press (an eye, Show pressed
  again) is one whose variant of the page's press type carries `#[hides]`, read through
  `#[derive(Pressed)]`. `Notice::Left` is sent inside the app by its navigation and is not
  visible to the check; the change of page stands for it. Reason: `main` found a hide by words in
  the action's name; a mark on the variant is the page's own statement. Held by the check and the
  derive.

**Decision: the screens check is a check of the tour root, not a test of each screen.** Reason:
it covers every screen a tour reaches, a screen added later included, without its author listing
it (`docs/SECRET-LEAKS.md` §5), and the frosted copies and fades it reads are kit and runtime
state no update test sees. Turned down: one test per secret screen, which a new screen forgets.

### 5.4 lint-check-tiles becomes a type

This answers section 08's question. On `main`, `lint-check-tiles` reads `faraday-core`'s source
for every Check row that opens a tool and fails when the tool has no Tools tile (inventory
`check-page.md`, `tools.md`). Not every Check row opens a tool: Dice opens New key at its dice
source and Backup opens a sheet.

**Decision: the rule is held by the compiler.** The tools a menu page can open are one enum,
the kernel's `Tool` (its `handle` module, section 06 §3.3; `Go` on `main`), which Check's and
Tools' features both name, and the Tools catalog's tile list is an exhaustive `match` over it
with `clippy::wildcard_enum_match_arm` denied. Check's row that opens a tool carries a `Tool`; a
`Tool` with no tile does not compile. Reason: with the rule a type, there is nothing left for a
script to find. Turned down: carrying the regex lint re-pointed at two feature crates, which
reads source text the compiler already types.

## 6. Leak tests

### 6.1 What is in scope

`main`'s leak tests (F250, `docs/SECRET-LEAKS.md`) look for a test secret in freed memory, in
memory live after a lock, and on screen, and let a known freed copy through by a named entry in
`tools/leak-entries.toml`. F281 (the decision that side channels on the offline device are
outside the security model) puts a copy left in freed memory outside the security model (its
point 1) and keeps as ordinary bugs a secret still reachable in the app's state after its screen
is done or after a lock, and a Faraday secret type that does not wipe (its point 2).

**Decision: `next`'s leak tests look at live memory and at the secret types' own wipe, never at
freed memory in general (F336).**
- No flow scans freed blocks, and no freed copy is listed: `tools/leak-entries.toml` is not
  carried, and no leak test has an entry or any other way through.
- After a screen that held a typed, scanned or shown secret is left, no live block holds that
  secret's forms (F281 point 2).
- After a lock no live block of the process holds a form of a planted secret.
- Each of Faraday's own secret types wipes what it holds when it drops: its domain crate's test of
  that type finds no form in the blocks freed by that one drop. This reads freed memory, but only
  the type's own blocks, to test the type's wipe, not to look for a leftover copy.
- A planted secret on screen after a lock, a hide or leaving (§5.3) fails.
Reason: the owner's call (2026-11-27), "Drop the freed scan, as recommended". A freed copy is
outside the security model (F281 point 1), and on the device every lock restarts the app and the
kernel zeroes freed memory, so a freed copy does not outlive the lock; a list of freed copies with
a fix each is the record of such copies that F281 point 1 says is not kept. Turned down: keeping
`main`'s freed scan and its entries, which looks for what the security scope puts outside it and
fails on every new freed copy, often inside a library, until an entry is written.

**Decision: the leak allocator stays, smaller.** Listing a thread's live blocks (address and size
only, as on `main`, SECRET-LEAKS §2) needs a global allocator, and so does reading the blocks one
drop frees. So `faraday-leak-scan` keeps its allocator; it reads a freed block only inside
`Leak::drop_checked`, which drops one value and reports a form in a block that drop freed, and
otherwise scans live blocks on request (`Leak::live`). Held by `drop_checked`'s shape: it takes
the one value it drops, so it checks that value's blocks and no other. Reason:
with the freed check reachable only around one drop, no flow can scan freed memory by mistake,
and the live scan, the in-scope rule, keeps `main`'s mechanism. Turned down: a live scan without an
allocator of its own, which has no list of a thread's blocks to read: the system allocator
gives none, and tests running side by side in one process would read each other's blocks.

### 6.2 Where they sit

The leak allocator is a global allocator, and a test binary has one. So every leak test is in a
test binary of its own, never in a binary of update tests.

| What | Owner | Where |
|---|---|---|
| A Faraday secret type wipes on drop | `domain` | the type's crate: `tests/leak.rs`, one binary, the allocator as a dev-dependency |
| The stick adapter's frame copies wipe at `Exit` (P13) | `adapter` | the stick adapter's `tests/leak.rs` |
| A flow's secret is not live after its screen is left, nor after a lock | `app` | the leak crate (below) |
| A reveal of `faraday_keys::Reveal` has a flow that ends clean | `app` | the leak crate's `tests/leaks/reveals.rs` |
| Nothing planted shows after a lock, a hide or leaving | `root` | the tour root's screens check (§5.3) |

**Decision: the flows run in one leak crate, `layer = "tool"` with `checks = "app"`.** It depends
on the runtime, the device composite, the shared owners and the tour root's fixture adapters (a
library, section 06), which a tool may (section 01, T1). A flow plants a test secret through the
app's input, runs the screens that handle it, and ends with a lock the runtime carries out
(`hide`, `at_lock`, the model dropped); the scan then runs over the thread's live blocks and over
the worker's. A flow for a Demo or Starter lock, which rebuilds the model in process, scans after
the rebuild as well. Reason: only the runtime drops a model as a lock does, and a scan of live
blocks after that drop is the in-scope rule exactly. Turned down: leak flows inside each feature's
update tests, which would put the allocator in every feature's test binary and end without the
runtime's drop.

- **Rule TS18.** Every feature whose state holds a typed, scanned or shown secret has a leak flow,
  and the leak crate's `flows` match over the device composite's features has no wildcard arm, so
  a new feature does not compile there until it has one or a line saying it holds no secret.
  Held by the compiler. This carries `main`'s `tests/leaks/areas.rs`.
- **Rule TS19.** The path the impact rule `keys.new-reveal` names is the leak crate's
  `tests/leaks/reveals.rs`; section 09 gives the crate's directory. The file matches
  `faraday_keys::Reveal` (and its `Part`) with no wildcard and names each arm's flows.
  `lint-leak-reveals`, re-pointed, fails on an arm with no flow or a flow that is no running test,
  as on `main`. Held by the compiler and that lint.
- The test secrets are published vectors, long and unusual, as on `main` (SECRET-LEAKS §3); a
  PIN is looked for only in its own tests.
- A leak flow's header Invariant names the flow ("A seed typed in Add a key is not live after a
  lock"); the flows of one feature differ by the way in.

The update tests of A12 (nothing held after `at_lock`) and A13 (a screen left drops what it held)
are kept beside the flows. They hold the state's marks; a flow holds that no copy outside the
marked fields stays live (a model a memo kept, a part's buffer). Two invariants, two tests.

## 7. Cross-checks, fuzz targets and costs

### 7.1 The cross-checks

The cross-check (`docs/CROSSCHECK.md` §10 on `main`) carries with the domain. It is owned by the
domain: a test-only command (a tool crate) answers requests with the domain's results, and a
script compares each with an outside implementation. It is not a Rust test and carries no header.
It runs before a release and on no schedule, a change to an area runs that area, and no stamp is
committed between releases (F197); `lint-release` checks the release's record.

- **Rule TS20.** A cross-check area compares a domain function's result. A rule of the
  application (which file a feature writes, which outputs it shows as change) is an update test,
  and the cross-check reaches it only through the domain function the feature calls. Held by the
  cross-check server's dependencies: domain crates only, and `faraday-draw` for the picture
  requests alone (section 10 §9.1).

On `main`, three areas (import, review, Core wallet) already call domain crates only. The
cross-check's users of `faraday-core` are the server's `hash`, `tx` and `seedqr` requests and
`core-check.py`, through `faraday-core`'s `interop` example (inventory `test-kit.md`,
`tooling.md`); section 10 §9 maps each to a domain function.

### 7.2 Fuzz targets

The fuzz targets carry with the domain parsers they feed (`lint-fuzz-parsers`). They are not
Rust tests and carry no header. A crash a fuzz run finds becomes a vector test in the parser's
crate, with its header, before it is fixed.

### 7.3 Costs

`main` enforces in `just` that a frame works out nothing, a pointer move works out nothing and a
shown secret is worked out once per press (`docs/SPEED.md` §7, the budget tests; inventory
`drawing-compute.md`, `test-kit.md`).

**Decision: each budget is held where its rule now sits.**
- A frame works out nothing: held by the compiler (V2: `present` has no `Compute`, `draw` has
  only its model). No test.
- A message works out no more than its rule allows (a pointer move or a scroll nothing; a press
  that shows a secret, the derivation once; Check's second card, the only costly press of a
  compare): an update test reads the test `Compute`'s count by kind after `route`. Owner `app`.
- A named costly operation's instruction count: `lint-op-cost` (F238), as on `main`.
- A frame's allocations and drawn text lines: measured when a change is timed (section 08,
  §3.3) with the counting allocator, not held by a test. Reason: on `next` a frame can call no
  costly work, and what is left of a frame's cost is time on the Pi, which a count of allocations
  stands for poorly and which the timing rule measures directly. Turned down: carrying the frame
  allocation budgets as a ratchet over the tours, a second counted check of screens for a cost
  the timing already reports.

## 8. The tests the earlier sections name

This answers section 03's question, and places the tests sections 01, 02 and 08 name.

| Rule | Test | Owner | One invariant per |
|---|---|---|---|
| A9, A10 | a refused press gives one note and no other effect | `app` | refusal of each feature (with A32's, below) |
| A12 | after `at_lock`, `holds()` is all false | `app` | feature that holds a secret |
| A13 | `Notice::Left` drops every typed secret and shown value | `app` | feature with a typed secret |
| A22 | a change given to an owner is answered once, also when refused | `app` | refusal of each owner's change (`faraday_keys::Refusal` and the owner's own variants, one test each) |
| A25 | a stale answer changes nothing and its payload is dropped | `app` | feature that starts a job or waits on an ask |
| A26 | while waiting, only Back, Cancel and the eye are taken | `app` | feature that waits |
| A29 | Back, Cancel, the hide on a step change, the foot's refusal | `app` | runner rule, in the kernel, over a test flow |
| A32 | in Practice, the press is refused | `app` | press of A32's list; the same test is that press's A10 test |
| A33 | the key store refuses a sample on Mainnet and takes only samples in Practice | `domain` | way a key comes in, in `faraday-keys` |
| A15 | an event and the messages it causes settle before the next event | `runtime` | one |
| A23, P9 | a stale or wrong answer is dropped or reaches the feature as its fault | `runtime` | case |
| A28 | a lock waits for a running job's answer and drops it | `runtime` | one, with a job that blocks until stopped |
| V19 | a step fade's kept source from a card that shows a secret is empty when the fade out ends, at a lock, at a hide and when the page is left; a fade towards such a card keeps no copy of it | `runtime` | way the copy ends, and the fade towards |
| §3.3 of 03 | a handle whose thing is gone reads `None`, and an id is never reused in a model | `app` | the session owner |
| P11, A34 | a write that fails part-way leaves no file and zeroes what it wrote | `adapter` | way it fails (TS9) |
| P12 | a file over the limit is refused before the ask | `app` | feature that reads |
| P13, P19, P23, P28, P29 | §2.4 | `adapter` | rule |
| P17, P27 | `Start` once and first; a stick written only when asked | `root` | journey |
| F301 | a vault whose passphrase was just set ends locked and is unlocked once with it | `app` | the vault composite's create feature |

A refusal is one invariant; a press refused for two reasons is two tests. A32's test of a press is
also that press's A10 test: one test, not two.

## 9. Running the suite

- **Rule TS21.** Every check runs to its end and reports every failure in one run (`cargo build
  --keep-going`, `cargo nextest run --no-fail-fast`, clippy over every crate it names, every
  lint), as section 08 §3.5 says.
- **Rule TS22.** The whole suite has a time budget, stated in `docs/DEVELOPING.md` for the
  check's run on GitHub Actions. A change that would go over it removes a test whose invariant
  another names, or makes a slow test fast (a stand-in clock, a smaller vector set that keeps
  every published case), before it lands. Held by review and the check's own time.
- One test module is run while working: `cargo nextest run -p <crate> -E 'test(/^update::/)'`
  for a feature (section 08, §3.5); `--filter` on a leak flow's name for the leak crate.

## 10. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| TS1 | one invariant per test | the header; review |
| TS2 | the owning layer tests a rule, no other | `lint-test-headers` (check 3) |
| TS3, TS4 | only plausible regressions; old tests not a source | the header; review |
| TS5 | no test deleted to pass | review; the duplicate check |
| TS6 to TS8 | update tests go through `route`, read what a feature exposes, test `present` only for its rules | A9; A7; review |
| TS9 | each adapter rule has its adapter test | those tests |
| TS10 | journeys are smoke tests | the header; review |
| TS11 | no token in a test | `lint-test-headers` (check 7) |
| TS12 to TS14 | no library behaviour, no geometry, no wording | review; A30 |
| TS15, TS16 | tours read the frame only and run to their end | R4; the tour root's test |
| §5.2 | press targets at Material's floor | `lint-touch-targets`; `structure.ratchet-raised` |
| TS17 | the screens check's events | the tour root; the derive |
| §5.4 | every Check tool row has a Tools tile | the compiler |
| TS18, TS19 | a leak flow per feature with a secret and per reveal | the compiler; `lint-leak-reveals` |
| TS20 | cross-checks compare domain functions | the server's dependencies |
| TS21, TS22 | runs to the end; a time budget | the justfile; `lint-all`; review |
| §3, §4 | the header and the copied list | `lint-test-headers`; `tests.copied-header` |

## Changes asked of earlier sections

- Section 01, §1, the metadata: add `checks = "<layer>"` for a crate of `layer = "tool"` that
  holds tests (the leak crate), read by `lint-test-headers` (§4, check 3). Reason: a tool holds no
  product rule, so its tests need the layer they check.
- Section 08, §6.2: add the impact pair `tests.copied-header` (a change to a file listed in
  `test-headers.copied` in `tools/` changes the list), and keep `structure.ratchet-raised`'s
  touch-target counts (§5.2). Reason: §4.1 and §5.2 here.
- Section 08, §8.1 and §8.3: `lint-touch-targets` and `lint-leak-reveals` are carried, re-pointed
  (§5.2, TS19); `lint-check-tiles` is dropped for the compiler (§5.4); the new lint is
  `lint-test-headers`. Reason: §8 there lists the lints by fate.
- Section 01, §2.1 (D3): the copied crates' tests take their headers when they are next changed,
  and a header added then is not a change D3 forbids. Reason: §4.1 here.

## Questions for later sections

- Section 06: that each root builds its model over any `Adapters` value, for journeys (§2.5);
  that the tour root's fixture adapters are a library the leak crate can use (§6.2); and where the
  enum of the tools a menu page opens lives, below both Check and Tools (§5.4).
- Section 05: how the runtime exposes the kept images (a sheet's frosted copy, a fade's frame)
  and the frame's text to the tour root's screens check (§5.3), and whether `Notice::Left` and
  `Notice::Hide` are visible in its output for the check to know its events.
- Section 04: the `#[hides]` mark on a message variant (§5.3), and the kit's motion state that
  the screens check reads as a kept image.
- Section 09: each adapter as a crate of its own with `layer = "adapter"` (section 01 §9 puts the
  stick adapter in `shells/stick` and the desktop adapter in `shells/demo`, both roots), so that
  its tests carry Owner `adapter` under check 3 of §4; the leak crate's and the test-support
  crate's names and directories; the path of
  the copied-tests list (§4.1) and the first list it holds.
- Section 10: how the cross-check areas that drive `faraday-core` on `main` (import, review, the
  Core wallet, through the `interop` example and `core-check.py`) map to domain functions
  (TS20), and which of them, if any, need the application.
