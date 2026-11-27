# Faraday next: architecture plan

**Kind:** Migration

This branch starts Faraday again on a new architecture. The current app
(`main`) is frozen while it is built; `next` replaces it at cut-over. This
file records the direction agreed so far. The specification written from it
will hold the full layer rules, the skeleton manifest and the test rules.

## 1. What the current code is, and what carries over

Measured on `main` at `d89dde5de` (about 560k lines of Rust in 50 crates).

| Layer | On `main` | Carries over |
|---|---|---|
| Cryptography, Bitcoin formats, codecs (`core/faraday-crypto`, `-bip`, `-psbt`, `-codec`, `-entropy`, `-text`) | Pure, mostly `no_std`, checked against published vectors and outside implementations | Copied unchanged, with their own tests |
| Keys, vault and file formats (`faraday-keys`, `-vault`, `-pgp`, `-sb`, `-kef`, `-ots`, `-plan`, `-paper`, `-fat`, `-files`, `-sqlite`, `-core-wallet`, `-payload`, `-qr`, `-scanner`) | Mostly pure; key material leaves `faraday-keys` only through named reveals | Copied; the keys and vault crates lose their edges to the input parsers (`faraday-intake`) and the key-form registry (`faraday-forms`) |
| Drawing base, tokens, fonts (`core/faraday-draw`), strings (`faraday-strings`), guide (`faraday-guide`) | No app types | Copied |
| Components (`faraday-ui`, `Ui<A>` generic over the action type) | No app types | Copied, then arranged by atomic level |
| Shell contract (`faraday-shell-api`: events in, commands out, a frame back) | Already one-way at the edge | Copied and grown into the one port (below) |
| The app (`faraday-core`, 247k lines; one `Faraday` struct of about 70 area fields; 154 `impl Faraday` blocks in 67 files) | Use cases mixed with presentation; drawing records hit targets and measures into state; continuations kept in state | **Ported**: its logic is carried into the new layers, not copied |
| Storage adapter (`faraday-storage`) | Depends on the app's own types | Re-pointed at the port |
| Shells (stick and Pi, Demo, Starter, snapshot) | Starter depends on Demo, which depends on all of `faraday-core` | Re-pointed at the new app; Starter no longer depends on Demo |
| Tests of `faraday-core` (196k lines) | Not tied to named regressions; much overlap | **Not ported.** Their fixtures and data (test wallets, PSBTs, test sticks, vectors) are kept; their code and assertions are not read for guidance |

## 2. Target architecture

1. **Domain.** The crates above. No I/O, no UI, no app types.
2. **Port.** `faraday-shell-api` becomes the single contract. Events come in: touch, keys, display, sticks and files, camera frames, clipboard, time, randomness. Typed effects go out: storage, camera, clipboard, print, worker job, vibrate, exit. The app depends on no shell crate, and no adapter depends on the app.
3. **Application: reducers with effects.** Each feature has its own `State`, `Msg` and `update(&mut State, Msg, &Ctx) -> Effects`. A root composes the features, and features reach one another only through messages their parent routes. A step flow is a typed state machine (one enum per flow). A worker's answer comes back as a message that carries its own context.
4. **Presentation, in two stages.**
   - `present(&State) -> ScreenModel` does all the working out, in update or a memo.
   - `draw(&ScreenModel, &mut Ui<Msg>)` only reads and paints. Hit targets are the view's output, held by the runtime and never by the model. Measures, such as content height, come back as messages.
   - Drawing stays immediate-mode: a retained widget tree was tried and removed on 2026-10-21, and the Pi's frame budget is 16.7 ms.
5. **UI kit by atomic level.** Levels are crate or module boundaries, and a lint holds them:
   - tokens
   - atoms: canvas primitives and controls
   - molecules: rows, chips, the passphrase field
   - organisms: the session pane, sheets, ways-in
   - templates: the pane container, the step-flow layout, the menu page
   - pages: one per screen
   
   A page uses only templates and organisms.
6. **Runtime.** Owns the model, the message queue, effect dispatch, the worker pool, the memo, and lock and restart.
7. **Composition roots.** The device app (stick and Pi), Faraday Demo and the Starter each choose the features they hold.

**Secrets in a one-way design:**
- The model is never `Clone`.
- A message carries a handle (a key's place, a `VaultId`) or moves a secret type; it never carries a copy.
- Only messages marked public are logged or replayed.
- Every feature's state declares what a lock wipes, as `AtLock` does today.
- A lock drops the model and restarts.

**Kept from `main`:**
- Key material only in `faraday-keys`, behind named reveals.
- Costly work takes the compute token, which drawing cannot mint.
- Named costly operations are held by an instruction count.
- Material 3.
- Cryptography is tested against published vectors and an implementation Faraday did not write.
- What the app takes in follows a published standard or a rule that fails closed.
- `cargo vet --locked`; builds with `--locked`.

**Overturned, each needing a new decision:**
- Areas exchanging only messages was not taken on `main`. It is taken here.
- The vault code was kept in one module for auditability. It splits into use cases here.
- The ratchets on `Faraday`'s field count and on the files allowed an `impl Faraday` are dropped with that struct.

## 3. Test architecture

- **One invariant, one test.** Every test opens with:
  - **Invariant:** what must stay true.
  - **Regression:** the plausible change that would break it.
  - **Owner:** the layer that holds the rule.
  
  A lint checks the header and refuses an invariant named twice.
- **The layer that owns a rule tests it, and no other layer does:**
  - **Domain rule:** published vectors in the domain crate.
  - **State transition:** a pure update test (state and message in; state and effects out). No drawing, no shell.
  - **Screen:** a rendered tour that a person reviews. No assertions on geometry.
  - **Journey:** a few per app, end to end through the shell contract, as smoke tests only.
- **A test is added only for a plausible regression:** a security rule, a refusal, a rule that crosses features, or a bug that happened. Not for a token's value, the language's or a library's own behaviour, or a code path that another test already covers.
- **Where invariants come from:**
  - the documented rules (security model, keys and lock, standards, decisions, and every refusal the code makes);
  - the bug fixes in `main`'s history and the outside audits' findings;
  - each feature's specification as it is ported.
  
  Old tests are not a source.
- **A run goes to the end.** Every check runs the whole of what it was asked
  for and reports every failure at once, not one per run:
  - `cargo build --keep-going`;
  - `cargo nextest run --no-fail-fast`;
  - clippy over every crate it names;
  - every lint, run to the end.
- **The suite is kept small enough to run whole.** The test rules above are
  what make that affordable: the full suite has a time budget, and a test
  that duplicates another is removed, not run twice.

## 4. How the work is done

1. Inventory of `main`'s app code (facts only: features, user actions, effects, state and what a lock wipes, reach between areas, rules and refusals, domain calls, fixtures), and of its bug fixes.
2. The specification, with the skeleton manifest: every crate and module per layer, its public types and traits, and its function signatures.
3. The skeleton: that manifest compiled, every body `todo!()`. The count of `todo!()` left is the measure of progress, and a release needs it at zero.
4. A mapping of every old module to the named items of the skeleton (copy, port, rewrite or drop).
5. Porting, a feature at a time, filling the stubs.
6. Parity checks on the hardware, then cut-over: `next` becomes `main`.
