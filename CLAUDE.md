# Working in this repository

These are the current rules for anyone, human or agent, changing this
repository. They are versioned here so that when one changes, this file
changes and nothing else has to be kept in step. The rules of one area are
in that area's document, named under "By area": read it before changing
the area. `CONTRIBUTING.md` (on `main`) summarises this file and changes
with it. `docs/spec/08-conventions.md` gives the reasons for the rules on
documents, decisions and commits; where it and this file differ, this file
is the rule. `just lint` holds most rules, and `python3 -I -B
tools/impact.py plan <paths>` lists the ones a change touches.

Crates are named by role here. Their exact names and paths, the recipe
names and the top-level layout are the skeleton manifest's (section 09 in
`docs/spec/`).

## Everywhere

- `docs/` holds what is true now or still to be built; how it was built
  is history (`docs/history/`). Every document opens with its kind
  (`docs/spec/08-conventions.md` §4.1): a Reference describes what is
  built; a Spec holds a design and whether it is built, with an
  `**Open:**` line saying in plain words what is not, and carries no work
  list: no table of work, no labels for pieces of work, no estimates or
  sizes, no choice of who does a piece (F214); a Record is a decision; a
  History document is how something was built, or a decision that no
  longer binds; `docs/plan/` and `docs/migration/` are Migration, about
  moving `main` to `next`, their kind set by the folder. What must come
  first and which files change stay as plain text where they matter to
  anyone building Faraday. The change that builds the last of a Spec's
  open work makes it a Reference and deletes what was only about building
  it; sections are never renumbered and a deleted section's number is not
  reused. A Reference carries no `**Status:**` line. How something was
  built goes to `docs/history/`; a document or decision is moved there,
  never deleted. Held by `lint-docs` and the impact pair
  `docs.no-silent-delete`.
- `docs/README.md` (on `main`) says which document answers which
  question: open it first, then read only the section you need. Its
  "Every document" list is made by `python3 -I -B tools/docs-index.py
  write` from each document's header, never by hand; a document added,
  deleted or changed in kind has its line there. Held by `docs-index.py
  check` in `lint-docs` and the impact pair `docs.readme-index`.
- The specification's sections stay at their paths in `docs/spec/`, and
  each becomes a Reference in place when what it specifies is built.
- A document of `main` comes to `next` with the code it describes, its
  paths rewritten to `next`'s tree. It keeps its kind if all it describes
  is in crates copied unchanged; otherwise it is a Spec whose `**Open:**`
  line names the features not ported. A path in a Spec, a Record or a
  Migration document resolves in `next`'s tree or in `main`'s at
  `d89dde5de`; a path in a Reference resolves in `next`'s; a document
  not on `next` is cited as "on `main`". Held by `lint-docs`
  (`docs/spec/08-conventions.md` §4.4, §4.6). How `d89dde5de` stays
  reachable is section 10 in `docs/spec/`.
- Faraday's decisions are numbered F1 onward, one file each
  (`docs/decisions/F<n>.md`), indexed in `docs/DECISIONS.md` (on `main`):
  read the index, then only the decision you need. `next` keeps `main`'s
  record as written and goes on above F335; a number is used once across
  both branches, never reused, and a number missing from `main`'s series
  stays unused. A new decision is a new file in the form of
  `docs/spec/08-conventions.md` §5.5, with a `**Held by:**` line naming a
  lint, `compiler`, `tests (<layer>)`, `review` or `none`, and
  `tools/decisions-index.py write` makes its row. A decision of the
  specification is also a record when it replaces or amends one of
  `main`'s, is a security trade-off, is a difference from Material 3, or
  accepts more than a standard requires. One replaced, superseded or
  withdrawn moves to `docs/history/decisions/` and keeps its row; one
  amended in part stays, with a line at its top naming the decision that
  amends it and the points it changes, and its row says "Amended by". A
  decision also moves once it no longer binds a later change and a
  Reference, Spec or lint holds all of it (F268). Held by `lint-docs`
  rule 5 and review.
- What a change brings with it is `tools/impact.toml` (F149, narrowed by
  F258): `impact.py plan <paths>` says it for a task; `just lint` fails on
  a pair left unmet unless a commit says why (`Impact-skip: <rule>:
  <reason>`). `impact.py check` compares a range with its merge base on
  the branch it lands on. A new pair is added there if a change must
  bring another with it; a rule enters with the change that brings the
  paths it names (`impact.py lint` fails on a rule that matches nothing).
  What no script can see is a checklist in the area's document and in
  `.claude/agents/reviewer.md`, not a trailer; no trailer attests a
  review.
- `just` is the check: tests, clippy with warnings as errors, fmt and the
  lints in the `lint` recipe. Every run goes to the end and reports every
  failure at once: `cargo build --keep-going`, `cargo nextest run
  --no-fail-fast`, clippy over every crate it names, and every lint
  (`lint-all`). Held by the justfile's recipes. Keep every test passing or
  change it to the rule it now encodes. The full check is slow, so it
  runs once for a finished body of work, never after each change or each
  commit: GitHub Actions runs it (below), and a commit need not pass it on
  its own. While working, run only the test module you touched (`cargo
  nextest run -p <feature crate> -E 'test(/^update::/)'`) and the one lint
  script for the files you changed; when the full check fails, fix what
  it lists and run again only the commands that failed. To look at a
  screen, render one tour at one size with the tour root's recipe
  (section 09 in `docs/spec/`). No crate a root depends on holds a
  `todo!()` in a release (`lint-release`).
- Tests (`docs/plan/ARCHITECTURE-PLAN.md` §3). Every test opens with its
  **Invariant** (what must stay true), **Regression** (the plausible
  change that would break it) and **Owner** (the layer that holds the
  rule); the layer that owns a rule tests it, and no other layer does: a
  domain rule by published vectors in the domain crate, a state
  transition by a pure update test, a screen by a rendered tour a person
  reviews, a journey by a few end-to-end smoke tests per app through the
  port. A test is added only for a plausible regression (a security
  rule, a refusal, a rule that crosses features, or a bug that happened),
  never for a token's value, a library's or the language's own behaviour,
  or a screen's geometry; `main`'s old tests are not a source. A test is
  never deleted or weakened to make the check pass; it is removed only
  when another test names its invariant or the rule it holds is gone, and
  the commit says which. Held by the test-header lint and review; its
  name, the header's form and the layers an Owner may name are section 07
  in `docs/spec/`.
- Every user-facing string is in the wording crate's `src/en/`, except
  the guide (`docs/guide/`) and the self-test's check names. Held by
  `lint-strings` over every application, kit and adapter crate. Text in
  the product is plain statement of fact, never mannered prose; a working
  screen carries labels, values and actions, never an explanation (that
  is the guide and About). Held by review.
- Commit messages. The subject is one line stating what is now true, in
  plain words, with no type prefix (`fix:`, `feat:`) and no tracker or
  work number: what changed for a person using Faraday, or else what is
  now true of the repository. The body says why, then the test that
  holds it, by its invariant. A decision is cited after the words that
  say what it is about, its number in brackets. A screen change has a
  line `Checked against Material 3: <the Material pages>`. The only
  trailer the repository's tools read is `Impact-skip: <rule>: <reason>`.
  Held by review.
- `docs/DEVELOPING.md` (on `main`) is how anyone builds, checks,
  reproduces and tries Faraday. A change to any of that (a recipe, a
  prerequisite, a dev-image feature, an output path) changes it in the
  same commit, and every command it shows runs as written (the impact
  pair `repo.developing`; `lint-docs`, every `just <recipe>` shown names
  a recipe). GitHub Actions runs the check (F183): `just` on every push
  and pull request, and, started by hand, the fuzz run before a release
  (F210) and crate tests (split over runners, F284) or one lint recipe
  (F243), nothing else; read-only, no secrets, actions pinned by commit,
  no metered services. Held by review of `.github/`.
- The tree stays clean (`tools/lint-tree.sh`): top-level entries only as
  README.md's "Repository layout" lists them, nothing an editor, a merge
  or a session leaves behind, no file over 2 MiB, build output in `out/`
  and `target/`, scratch files outside the repository.
- The repository is public. No document, comment, commit message or file
  names a person, a private machine (by name, owner or address), a
  tracker item, a piece of work or how the work is shared out; paths in
  documents are relative to the repository. The remote machine that
  builds the images is "the build machine" in everything written here,
  never its hostname, its owner's name or its address. Every new commit
  is checked for personal information before it is pushed: names, email
  addresses, hostnames, IP addresses, local paths, keys and tokens, in
  the diff and in the commit message. A git hook that makes this check
  counts as it; without one, read them by hand. Held by `lint-docs`, the
  hooks and review.
- Do not use `rm`; move files to the trash (`trash` or `gio trash`). Use
  absolute paths in shell commands.

## Keys, cryptography and what the app takes in

- What the app takes in is `docs/STANDARDS.md` (on `main`) §0: a
  published standard, followed and checked against an implementation
  Faraday did not write, or Faraday's own rule that fails closed; work on
  an unpublished standard or an unreviewed construction stays in the
  repository, compiled out of the app (`lint-standards`). Nothing that
  comes through the port is trusted: its bytes, frames, pasted text and
  files are parsed and checked by the domain, and an adapter's own limits
  are a defence beside the domain's, not in place of them.
- The domain is Faraday's own code (`core/` and the other domain crates):
  fix a thing at its source rather than wrap it. A cryptographic change
  is tested against published vectors and an implementation Faraday did
  not write (a second one by Faraday's own authors is not one); an action
  of Faraday's own specification carries F177's label until an outside
  review (`docs/crypto/check.py`, the cross-check's `facts.py`, review).
  What the code refuses stays refused (a multisig key without an origin,
  a guessed path, an ambiguous input); accepting more than a standard
  requires is the owner's decision, recorded as a decision. A type
  re-implemented from a domain one keeps the values the original works
  out once.
- The full cross-check runs before a release, never on a schedule, and
  the tip is not kept cross-checked (F197): a change to an area's code
  runs that area and commits no stamp or fingerprint; a release passes
  `just lint-release`.
- A check that keeps keys or money safe has no way past it unless a
  decision names a legitimate case (a hold naming it, STANDARDS §0, F136;
  `lint-standards`). Key material lives in `faraday-keys` and leaves only
  through its named reveals (F70, `docs/KEYS.md` on `main`; its API and
  `lint-leak-reveals`); a new reveal is in `docs/KEYS.md` and
  `docs/AUDIT.md` (both on `main`) and names its leak test (the impact
  pair `keys.new-reveal`). A dependency passes `cargo vet --locked` (F71) and
  has its `docs/deps/` page (`lint-deps`, the impact pair `deps.page`);
  builds and checks pass `--locked`.
- Secrets in the one-way design (`docs/spec/01-layers.md` §8,
  `docs/spec/02-port.md` §6). No layer above the domain defines a type
  that holds a secret: a feature holds one only in a `faraday-keys` type
  or a wiping type (`lint-secrets`, on every application, kit and runtime
  crate). No secret type and no model is `Clone` (the compiler; a
  compile-time assertion in the runtime over the root's model). A
  message carries a handle (a key's place, a `VaultId`) or moves a secret
  type, never a copy, and a job owns what it is given. Only `Public`
  messages are logged or replayed; `Public` is sealed, its derive refuses
  a field that is not `Public`, and no port event, answer or effect is
  `Public` (the compiler). Every port payload that may hold a secret is a
  wiping type that is not `Clone` (the compiler; `lint-secrets`), and
  `Debug` on a port type prints names, kinds and lengths only. A revealed
  secret reaches a screen only as a borrow of a reveal guard for one
  frame, and the memo holds only `Public` models (the compiler). Every
  feature's state implements `AtLock`, saying what a lock wipes; a lock
  drops the model and restarts (the kernel's `Feature` bound).
- **Security scope** (`docs/SECURITY-MODEL.md` on `main`, "What this does not defend
  against"). The rules above are sized by it. Faraday is an offline
  computer, used in a secure place and powered off after use. Everything
  from the online side is untrusted (the host or coordinator, a PSBT, a
  file, a QR code) and the device verifies it; its cryptography is right
  against published vectors. A bug there is fixed now. Timing and memory
  side channels on the offline device (code whose time depends on a
  secret; a copy of a secret left in freed memory, on the stack or in a
  library's own buffer) are outside this scope: they are not looked for,
  not reported, not recorded as open work and not fixed. Four things stay
  in scope as ordinary bugs: a secret still reachable in the app's own
  state after the screen that needed it is done, or after a lock; one of
  Faraday's own secret types that does not wipe what it holds;
  secret-key arithmetic done outside a constant-time library; and a
  comparison that decides access. A malicious
  Faraday binary, firmware and the management engines, cold-boot attacks
  and a weak passphrase are out of scope, and so is a USB device that shows
  only as a keyboard or mouse and is plugged in at boot (trusted; a notice
  lists it). Faraday Demo and the Starter run on an online computer and hold
  test data. Any other risk outside this scope gets
  one line in a report; it is not fixed without the owner asking.

## Layers and structure

The layers, what each may depend on and what holds each rule are
`docs/spec/01-layers.md`; the port is `docs/spec/02-port.md`.

- Every crate declares its layer (`[package.metadata.faraday]`: `layer`,
  and `level` in the kit, `role` in the application), and every normal,
  build and dev edge between workspace crates is one section 01's table
  allows: domain, port, UI kit, application (kernel, parts, features,
  composites), runtime, adapters, composition roots, tools. Held by
  `lint-layers`, which reports every refused edge in one run.
- The domain holds no I/O, no UI and no app types; time and randomness
  reach it as arguments (`lint-layers`; the domain's `clippy.toml`). A
  crate that is `no_std` on `main` stays `no_std` (the attribute;
  `lint-layers`). Domain crates are copied from `main` with their own
  tests and changed only to cut the edges section 01 §2.3 names (review
  against `main` at `d89dde5de`). `faraday-keys` and `faraday-vault`
  depend on no input parser and not on the key-form registry
  (`lint-layers`, `boundary`).
- The port is the one contract between the app and every shell: events
  in, effects out, the frame out. It is one `no_std` crate of types over
  domain value types, with no I/O and no wording (`lint-layers`; the
  attribute; `clippy.toml`; `lint-strings`). No application crate
  depends on an adapter or a root, and no adapter on the application or
  the runtime; the root joins them (`lint-layers`). An effect that fails
  comes back as an answer carrying its typed failure, which the app
  words; no adapter panics on an ask and no app crate on an answer (the
  types; a crate-root `deny` of `unwrap`, `expect` and `panic`, checked
  by `lint-layers`).
- A feature is a crate with its own `State`, `Msg` and `update`. It never
  depends on another feature; only a composite or a root does, and routes
  one feature's `Out` to another as a message. A composite reads nothing
  in a child's state, whose fields are private. What features share to
  read reaches them only through `Ctx`, which gives no `&mut`; a type two
  features need is a handle in the kernel or a domain type. A part (the
  passphrase field, a text field, the ways in to a key) answers only its
  owner. Held by the crate boundary, the compiler and `lint-layers`
  (`role`).
- A feature's presentation is two stages, its modules: `present` works
  out everything a screen shows and is given no `Compute`; `draw` is
  given only its model and paints. Hit targets are the frame's, held by
  the runtime, never by a model or a state; a measure drawing finds comes
  back as a message. Held by the signatures and `lint-layers`' module
  rule.
- The kit is arranged by atomic level (tokens, atoms, molecules,
  organisms, templates; pages are each feature's `page` module), a crate
  per level, each depending only on lower levels and naming no app type.
  A page uses only templates and organisms; a template or organism
  re-exports only what a page needs to call it. The tokens crate depends
  on nothing and is the one kit crate the domain and the adapters may
  name. Held by `lint-layers` (`level`, `reexports`) and the compiler.
- The runtime is generic over the root's composite and names no feature;
  it owns the model, the queue, effect dispatch, the worker pool, the
  memo, lock and restart, and the last frame, which a tour reads as it is
  (no app or kit crate carries a hook for tours). It is the only shipped
  code that mints `Compute` (`clippy.toml`; `costs-lint`). Held by
  `lint-layers`.
- An adapter turns the port's effects into I/O and I/O into events; the
  disk process and the grant helper are adapter binaries no crate depends
  on. A composition root is a binary per product (the device app, Faraday
  Demo, the Starter) and the tour root, which never ships; no root
  depends on another. No root and nothing a root ships depends on a tool,
  and a crate's dev-dependencies are of its own layer or below or test
  support. Held by `lint-layers` over each root's resolved graph.
- Code is split by what it does, never by how long it is; a file and a
  function have no line limit (F248, F253). A function over clippy's
  `cognitive_complexity` threshold carries
  `#[expect(clippy::cognitive_complexity, reason = "...")]`, and the
  threshold only falls (F253); the reason is held by clippy's
  `allow_attributes_without_reason`, denied in the workspace lints.
- The workspace forbids `unsafe` code. Only the crates on the closed list
  of section 01 rule U2 allow it, on the one item that needs it; a new
  entry is a security trade-off and needs a recorded decision. Held by
  the workspace lints and `lint-layers`.

## By area

- **Screens and design:** every app follows Material 3 (F92, F91), drawn
  with Faraday's own look (the kit): `docs/DESIGN.md` (on `main`; the
  design system, its §5 screens, §4 components by atomic level),
  `docs/LAYOUT.md` (on `main`), and for the Starter `docs/STARTER.md` (on
  `main`) §3. Colours are the palette's roles (`lint-themes`); a new
  dimension is a name in the tokens crate (`lint-tokens`: counts only
  fall). A difference from Material needs a decision; a screen review
  names the Material page it was checked against. A new component is in
  `docs/DESIGN.md` §4 under its level, a new feature's page in §5 (the
  impact pairs `design.new-component`, `design.new-screen`). Step flows
  draw on the step-flow template (`docs/FLOW-CONTAINER.md` on `main`);
  menu pages draw on the menu-page template (`docs/MENU-PAGE.md` on
  `main`). Held by `lint-layers` (a page has nothing else to draw with).
- **The pane container:** a page or flow declares its panes; only the
  pane-container template places them and times a change of page
  (`docs/HUB-LAYOUT.md` on `main`). Held by `lint-layers`.
- **The session pane:** what the session holds (keys, wallets, what
  waits) is drawn only by the session pane organism
  (`docs/SESSION-PANE.md` on `main`); no page keeps a list of its own.
  Held by `lint-layers`.
- **Key forms and passphrases:** a key form is a row of the registry
  (`faraday-forms`, `forms::Form`) and of `docs/KEY-FORMS.md` (on `main`;
  the impact pair `keys.forms-registry`); a way in to a key is drawn by
  the ways-in organism, its state the ways-in part; a typed passphrase is
  held and filtered only by the passphrase field part (`docs/KEYS.md` on
  `main`, §2; `lint-layers`' module rule), and a vault whose passphrase
  was just set ends locked and is unlocked once with it (F301; a test of
  the vault feature's update).
- **Drawing and costly work:** `draw` reads its model and does not work
  things out; costly work is done in `update` with `Ctx`'s token or in a
  job, and its result is state (`docs/SPEED.md` on `main`, §3.3 and §7;
  `docs/COMPUTE.md` on `main`; the signatures and `costs-lint`). A change
  that adds work to a `present`, a memo or a page is timed once in a
  late-flow state before it lands, and the time and how it was measured
  are in the commit message. A named costly operation is held by an
  instruction count, raised only in a commit that says why, and a stated
  cost says how it was measured (F238, `lint-op-cost`).
- **Layers and the port:** `docs/spec/01-layers.md`,
  `docs/spec/02-port.md`; held by `lint-layers`, as "Layers and
  structure" above says.
- **Tests:** section 07 in `docs/spec/`, which also settles whether the
  touch-target and check-tile lints carry and where the leak tests are.
