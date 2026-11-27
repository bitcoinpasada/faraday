# Conventions for next: its CLAUDE.md, documents and decisions

**Kind:** Spec
**Open:** Not built.

This section fixes the rules that `next` holds itself to as a repository, outside its code: the
rules its `CLAUDE.md` carries, the documents it keeps and how they are checked, its decisions
record, its registry of what a change brings with it (`tools/impact.toml` on `main`), and three
small conventions (the user-facing strings, the test header's home, commit messages). It is
written for whoever writes `next`'s `CLAUDE.md`, `docs/README.md`, `lint-docs` and the first
change that brings documents and decisions over from `main`, and for anyone who later changes a
rule. `next`'s `CLAUDE.md` is written from this section; where the two differ later, `CLAUDE.md`
is the rule and this section is history. Sources: `main`'s `CLAUDE.md`, `CONTRIBUTING.md`,
`docs/README.md`, `docs/DECISIONS.md`, `docs/IMPACT.md` and `tools/impact.toml` at `d89dde5de`,
and section 01 for the layers.

## 1. The principle

- **Rule C1.** `next` keeps `main`'s rules for the repository, unchanged in meaning, except where
  this section names a change or a drop. A rule `main` held by a lint stays held by a lint on
  `next` (the same one, re-pointed, or one named here). Held by review of `next`'s `CLAUDE.md`
  against this section and against `main`'s at `d89dde5de`.
- **Rule C2.** `CLAUDE.md` is the one place a rule for changing the repository is stated in
  full. An area's rules are in that area's document, which `CLAUDE.md` names under "By area".
  `CONTRIBUTING.md` summarises `CLAUDE.md` and is changed with it. Held by review (the reviewer's
  checklist, §8.4).
- **Rule C3.** The repository is public. No document, comment, commit message or file names a
  person, a private machine (by name, owner or address), a tracker item, a piece of work or how
  the work is shared out, and every commit's diff and message is checked for personal
  information before it is pushed, as `main`'s "Everywhere" says. Paths are relative to the
  repository. Held by `lint-docs` (a `local/` path anywhere but `CLAUDE.md` fails, as on
  `main`), by the commit hooks that check for personal information, and by review.

**Decision: `next`'s `CLAUDE.md` keeps `main`'s shape.** Its parts are the same: a paragraph on
what the file is, "Everywhere", "Keys, cryptography and what the app takes in", and "By area",
with one part added, "Layers and structure", after "Keys". Reason: a reader who knows `main`'s
file finds each rule where it was, and the changes are visible as changes. Turned down: a new
file written from the architecture plan alone, which would lose rules that the plan does not
mention but `main` holds (the tree, the build machine's name, `--locked`).

## 2. Rules carried unchanged

These carry into `next`'s `CLAUDE.md` with the same words, but for the paths, which follow the
skeleton manifest (section 09). What holds each is what holds it on `main`.

| Rule (short) | On `main` | Held on `next` by |
|---|---|---|
| The Security scope, word for word: an offline device; everything from the online side untrusted and verified; side channels on the offline device out of scope; the four things kept in as ordinary bugs; what else is out of scope; one line in a report for other risks | `CLAUDE.md`, "Security scope"; `docs/SECURITY-MODEL.md`, "What this does not defend against" | the impact pair `repo.security-scope` (§6.2) |
| What the app takes in is `docs/STANDARDS.md` §0: a published standard checked against an outside implementation, or Faraday's own rule that fails closed; unpublished or unreviewed work stays in the repository, compiled out of the app | "Keys" | `lint-standards` |
| A cryptographic change is tested against published vectors and an implementation Faraday did not write; Faraday's own specification carries F177's label until an outside review | "Keys" | `docs/crypto/check.py`, the cross-check's `facts.py`; review |
| What the code refuses stays refused; accepting more than a standard requires is the owner's decision, recorded | "Keys" | review; a decision (§5) |
| A type re-implemented from a domain one keeps the values the original works out once | "Keys" | review |
| The full cross-check runs before a release, never on a schedule; a change to an area's code runs that area and commits no stamp (F197) | "Keys" | `lint-release` |
| A check that keeps keys or money safe has no way past it unless a decision names a legitimate case (F136) | "Keys" | `lint-standards` (every hold names its decision) |
| Key material lives in `faraday-keys` and leaves only through its named reveals (F70) | "Keys" | section 01 rule K-1 |
| A dependency passes `cargo vet --locked` (F71) and has its `docs/deps/` page; builds and checks pass `--locked` | "Keys" | `lint-deps`; the impact pair `deps.page` |
| Tests, clippy with warnings as errors, fmt and every lint are the check, `just`; every lint runs to the end (`lint-all`) | "Everywhere" | `just` |
| GitHub Actions runs `just` on every push and pull request, and by hand the fuzz run before a release, a split test run and one lint recipe; read-only, no secrets, actions pinned by commit, no metered services (F183, F210, F243, F284) | "Everywhere" | review of `.github/` |
| `docs/DEVELOPING.md` is how anyone builds, checks, reproduces and tries Faraday, and every command it shows runs as written | "Everywhere" | the impact pair `repo.developing`; `lint-docs` (recipes) |
| The tree stays clean: top-level entries only as `README.md`'s "Repository layout" lists them, no file over 2 MiB, build output in `out/` and `target/` | "Everywhere" | `lint-tree` |
| No `rm`: files go to the trash; shell commands use absolute paths | "Everywhere" | review |
| Every app follows Material 3 (F92, F91) with Faraday's own look; colours are palette roles; a difference from Material needs a decision; a screen review names the Material page it was checked against | "By area" | `lint-tokens`, `lint-themes`; review |
| A new dimension is a name in the tokens, and the count of numbers written in place only falls | "By area" | `lint-tokens` (section 01 rule K2) |
| A vault whose passphrase was just set ends locked and is unlocked once with it (F301) | "By area" | a test of the vault feature's update (section 07) |
| A named costly operation is held by an instruction count, raised only in a commit that says why; a stated cost says how it was measured (F238) | "By area" | `lint-op-cost` |
| Text in the product is plain statement of fact; a working screen carries labels, values and actions, never an explanation | "Everywhere" | review |

## 3. Rules that change with the architecture

Each is given as `next`'s rule, what it replaces on `main`, and what holds it.

### 3.1 Code structure becomes the layers

`main`: "an area's code, state and screen in its folder, code split by function and never by
length, private area state reached through accessors, `impl Faraday` only as glue" (F74, F196,
F248, F253; `docs/STRUCTURE-PASS.md` §7; `lint-layout`).

`next`, in its new part "Layers and structure":

- Every crate declares its layer, and every dependency edge is one section 01's table allows.
  Held by `lint-layers`.
- A feature is a crate with its own `State`, `Msg` and `update`; features reach one another only
  through messages a composite or the root routes, never by naming a sibling. Held by the crate
  boundary and `lint-layers` (section 01 rules A4 to A8).
- A feature's presentation is two stages: `present` works out everything a screen shows, and
  `draw` reads the model and paints. Held by the signatures and `lint-layers`' module rule
  (section 01 rules V1 to V3).
- The kit is arranged by atomic level, and a page uses only templates and organisms. Held by
  `lint-layers` (section 01 rules K1 to K8).
- Code is split by what it does, never by how long it is; a file and a function have no line
  limit (F248, F253).
- A function over clippy's `cognitive_complexity` threshold carries
  `#[expect(clippy::cognitive_complexity, reason = "...")]`, and the threshold only falls (F253).
  The reason is held by clippy's `allow_attributes_without_reason`, denied in the workspace
  lints, in place of `lint-layout`'s check.

The document this points to is section 01 (and, once built, the Reference it becomes, §4.3).

### 3.2 The standard parts are templates, organisms and parts

`main` holds the flow container, the menu page container, the pane container, the session pane,
the ways-in component and the passphrase field by `lint-components`' counted patterns, which
find hand-made copies in `faraday-core`. On `next` each is a kit template, a kit organism or an
application part, and a page has nothing else to draw with:

| Part | `main`'s rule | `next`'s rule | Held by |
|---|---|---|---|
| Step flows | draw on the flow container (`docs/FLOW-CONTAINER.md`) | draw on the step-flow template | section 01 K6, K8 |
| Menu pages | draw on the menu page container (`docs/MENU-PAGE.md`) | draw on the menu-page template | K6, K8 |
| Panes | only `screens/hub_layout/` places them and times a change of page | only the pane-container template places them and times a change of page | K6, K8 |
| What the session holds | drawn only by `screens/session_pane/` | drawn only by the session pane organism | K8 |
| A way in to a key | drawn by `ways_in` | drawn by the ways-in organism, its state the ways-in part | K8; A2 |
| A typed passphrase | held and filtered only by `pass_field` | held and filtered only by the passphrase field part | A3 in `lint-layers` |
| A key form | a row of `faraday-forms` and of `docs/KEY-FORMS.md` | unchanged | the impact pair `keys.forms-registry` |

The two counted rules of `main`'s `lint-components` that are not about these parts become types:
a wallet export kind is an enum of the port's file effect, never a number, and a menu page's
width and columns are the template's alone. The compiler and K6 hold them.

Inventory: `features/flow-container.md`, `features/menu-pages.md`, `features/pane-container.md`,
`features/session-pane.md`, `features/add-key.md`, `features/secrets.md`.

### 3.3 Drawing and costly work

`main`: "drawing reads and does not work things out; costly work takes `c: &Compute`; work added
while drawing is timed once in a late-flow session, before it is pushed" (`docs/SPEED.md` §3.3
and §7, `docs/COMPUTE.md`).

`next`:

- `draw` is given only its model; `present` is given no `Compute`. Costly work is done in
  `update` with `Ctx`'s token, or in a job. Held by the signatures and `costs-lint` (section 01
  rules V1, V2).
- Only the runtime mints `Compute` in shipped code (section 01 rule R2). Held by `clippy.toml`
  and `costs-lint`.
- A change that adds work to a `present`, a memo or a page is timed once in a late-flow state
  before it lands, and the time is in the commit message. Held by review; `docs/SPEED.md` gives
  the method once it is carried (§4.4).

Inventory: `features/drawing-compute.md`.

### 3.4 Secrets in the one-way design

New in "Keys": the model is never `Clone`; a message carries a handle or moves a secret type and
never a copy; only `Public` messages are logged or replayed; every feature's state says what a
lock wipes, and a lock drops the model and restarts. Held as section 01 §8 says (the compiler,
`lint-secrets` on every application, kit and runtime crate, the runtime's assertion). This
replaces `main`'s reference to `docs/AT-LOCK.md`'s exhaustive listing of `Faraday`'s fields,
which goes with that struct. Inventory: `features/secrets.md`, `features/lock.md`.

### 3.5 The check, its examples and its runs

`main`'s "`just` is the check" bullet carries, with three changes:

- **Runs go to the end.** `cargo build --keep-going`, `cargo nextest run --no-fail-fast`, clippy
  over every crate it names and every lint report every failure in one run (the plan's §3).
  Held by the recipes in the justfile and `lint-all`.
- **The examples are `next`'s.** The one-module test run names a feature crate and its update
  tests (`cargo nextest run -p <feature crate> -E 'test(/^update::/)'`); the one-tour render
  names the tour root's recipe. The exact names are the manifest's (section 09). Held by
  `lint-docs`' recipe rule (§4.6, rule 7).
- **A release has no stub.** No crate a root depends on holds a `todo!()` in a release. Held by
  `lint-release`, which reads each root's resolved graph as `lint-layers` does.

Inventory: `features/tooling.md`, `features/tour-runner.md`, `features/tours.md`.

### 3.6 Tests

`main`'s bullet "Tests cover user-facing behaviour..." is replaced by the plan's §3, in one
bullet of "Everywhere":

- Every test opens with its **Invariant**, **Regression** and **Owner**; the layer that owns a
  rule tests it, and no other layer does; a test is added only for a plausible regression (a
  security rule, a refusal, a rule that crosses features, or a bug that happened), never for a
  token's value, a library's or the language's own behaviour, or a screen's geometry; `main`'s
  old tests are not a source. Held by the test-header lint (section 07 gives its name, its form
  and what it refuses).
- A test is never deleted or weakened to make the check pass. A test is removed only when its
  invariant is named by another test or the rule it holds is gone, and the commit says which.
  Reason: `main`'s rule ("do not delete a test to make the build pass") and the plan's ("a test
  that duplicates another is removed") then both hold. Held by the test-header lint (an
  invariant named twice fails) and review.

**Decision: the test rules' home is section 07, and `CLAUDE.md` carries only the bullet above.**
Reason: the rules have a lint and a form that need more than a bullet, and `CLAUDE.md` names
the document for an area's detail. Turned down: the whole of section 07 in `CLAUDE.md`, which
would make it the area's document as well.

### 3.7 Rules reworded for `next`'s names

- **The domain is Faraday's own code** (`main`: "`core/` is Faraday's own code: fix a thing at
  its source rather than wrap it"). On `next` the rule names the domain layer, which holds
  `core/` and the other domain crates (section 01 §1).
- **The strings** (§7.1).
- **Unsafe code** is allowed only in the closed list of section 01 rule U2, and a new entry is a
  security trade-off recorded in a decision (U3).
- **The pane container's, the session pane's and the key forms' bullets** under "By area" point
  at the templates, organisms and parts of §3.2.

## 4. The documents next keeps

### 4.1 Kinds and headers

| Kind | What it holds | Header in its first 12 lines | Where |
|---|---|---|---|
| Reference | what is built and true now | `**Kind:** Reference` | `docs/`, `docs/spec/` |
| Spec | a design, not built or partly built | `**Kind:** Spec`, then `**Open:**` saying in plain words what is not built | `docs/`, `docs/spec/` |
| Record | the decisions, kept as written | `**Kind:** Record`, `**Status:**`, `**Area:**`, `**Held by:**` (§5.6) | `docs/DECISIONS.md`, `docs/decisions/` |
| History | how something was built; a decision that no longer binds | `**Kind:** History`; a moved decision also keeps its `**Status:**` (why it moved) and `**Area:**`, as on `main` | `docs/history/` |
| Migration | facts and plans about moving `main` to `next`: the plan, the inventory, the mapping | none needed: the kind is the folder's | `docs/plan/`, `docs/migration/` |

**Decision: a Reference carries no `**Status:**` line.** A Reference is built by its kind; a Spec
says what is open on its `**Open:**` line; only a Record has a status. Reason: on `main` a
Reference's Status line repeated "Built" and grew into a list of decisions and build steps,
which is the build log F214 moved out of Specs. Turned down: keeping `main`'s Status line on
every document, which the specification's sections already do without.

**Decision: the documents about moving `main` to `next` are a kind of their own, Migration, set
by their folder.** They state facts about `main` and how its code becomes `next`'s, and they cite
`main`'s tree. Reason: they are neither a design of `next` nor a record of how `next` was built,
and the 82 feature files and their tables should not each carry a header. Turned down: calling
them History now, which would let `lint-docs` skip checking their paths, and calling them Specs,
which would ask each for an `**Open:**` line about work that is not a feature. They move to
`docs/history/` with the change that makes `next` the main branch (section 10).

### 4.2 The map

`docs/README.md` is the map, as on `main`: what each kind means (the table above in prose), a
"By question" table of the question a reader has and the document that answers it, and the
"Every document" list, generated from each document's header by `tools/docs-index.py write` and
never edited by hand. A Reference's line is its title; a Spec's line is its title and its
`**Open:**` line. The Migration folders and the specification are each one line naming their
README.

- **Rule DOC1.** A document added, deleted or changed in kind has its line in `docs/README.md`.
  Held by `docs-index.py check` in `lint-docs` and the impact pair `docs.readme-index`.

### 4.3 Specs, References and the specification

- **Rule DOC2.** A Spec holds a design and whether it is built, and carries no work list: no
  table of work, no labels for pieces of work, no estimates or sizes, no choice of who does a
  piece (F214). What must come first and which files change stay as plain text where they matter
  to anyone building Faraday. Held by `lint-docs` (rules 3, 4 and 8).
- **Rule DOC3.** The change that builds the last of a Spec's open work makes it a Reference and
  deletes what was only about building it; sections are never renumbered, and a deleted
  section's number is not reused. Held by `lint-docs` (rule 3) and review.
- **Rule DOC4.** The specification's sections stay at their paths in `docs/spec/`. Each becomes a
  Reference in place when what it specifies is built. Sections 09 (the skeleton manifest) and 10
  (the migration method) are about building `next`; once the code they describe is the record,
  each moves to `docs/history/`, leaving its line in `docs/spec/README.md` naming where it went.

**Decision: the architecture stays in `docs/spec/` after it is built.** Reason: code comments,
decisions and documents cite section numbers by path, and a move breaks every citation. Turned
down: moving the sections to a `docs/architecture/` folder when they become References.

### 4.4 Which of main's documents come to next

A document of `main` is brought to `next` by the change that brings what it describes: with the
domain crates when they are copied, with the kit when it is copied, with a feature when it is
ported. Its paths and names are rewritten to `next`'s tree in that change. Until then, a section
of the specification cites it as "`docs/X.md` on `main`".

- **Rule DOC5.** A carried document keeps its kind if all it describes is in crates copied
  unchanged; otherwise it enters as a Spec whose `**Open:**` line says which features it
  describes are not ported, and becomes a Reference again when the last of them is. Held by
  `lint-docs` (rule 3) and review. A carried document that cites code not yet on `next` (on
  `main`, `docs/KEYS.md`, `docs/STANDARDS.md`, `docs/AUDIT.md` and `docs/CRYPTO-LEARN.md` each
  name a few `faraday-core` files) is such a Spec, and its citations resolve at `main`'s frozen
  commit until the code is ported (§4.6, rule 2).
- **Rule DOC5a.** The documents `CLAUDE.md` names (`docs/SECURITY-MODEL.md`, `docs/STANDARDS.md`,
  `docs/KEYS.md`, `docs/DEVELOPING.md`, `docs/README.md`, `docs/DECISIONS.md` and the area
  documents of "By area") come in the same change as `CLAUDE.md`, or `CLAUDE.md` names them "on
  `main`" until they do. Held by `lint-docs` (rule 2).

**Decision: documents come with their code, not all at once.** Reason: a document brought before
its code cites paths that do not exist and calls built what is not built on `next`. Turned down:
copying every document at the start, which makes every Reference untrue until its feature is
ported.

| Class | Documents | On `next` |
|---|---|---|
| Carried with the domain, as they are but for paths | `SECURITY-MODEL`, `STANDARDS`, `ESTABLISHED`, `AUDIT`, `AUDIT-SUBMISSIONS`, `KEYS`, `VAULT`, `KEF`, `CODEX32`, `GPG-PAPERKEY`, `COMPARE`, `SEED-CONVERSION`, `CRYPTO-LEARN` (and `docs/crypto/`), `CROSSCHECK`, `KERNEL-PATCHES`, `HARDWARE`, `IMAGES`, `STICK-IDENTITY`, `INTEROP`, `docs/deps/`, `docs/guide/` | Reference where they were; a section about an app screen is open until that feature is ported (DOC5) |
| Carried with the kit | `DESIGN`, `LAYOUT`, `THEMES`, `TEXT-SIZE`, `TEXT-CURSOR`, `MOTION`, `M3-CHECKS`, `ORIENTATION`, `DRAW-COST` | Spec until the kit and the pages they describe are built; `DESIGN` §4 lists components by atomic level |
| Carried with their features | `FLOWS`, `BACKUP-FLOW`, `SPEND-FROM-BACKUP`, `SIGNER-CHECK`, `WALLETS`, `HOME`, `QR`, `SCANNING`, `TOTP`, `OWN-RANDOMNESS`, `STICK-SETUP`, `KEY-FORMS`, `MODES`, `DEMO`, `DESKTOP-HOST`, `STARTER`, `GUIDE-IN-APP`, `WORKSPACE` | Spec until ported (DOC5); `WORKSPACE` keeps its unbuilt part open |
| Rewritten for the new architecture | `DEVELOPING`, `IMPACT`, `COMPUTE`, `WORKERS`, `AT-LOCK`, `SECRET-LEAKS`, `SPEED`, `FRAMES`, `FLOW-CONTAINER`, `MENU-PAGE`, `HUB-LAYOUT`, `SESSION-PANE`; and `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `tools/README.md` at the top | Each says how `next` does it: `COMPUTE` who mints the token (R2); `WORKERS` a job and its answer as a message; `AT-LOCK` each feature's `AtLock` and the restart; the four container documents as the templates and organisms of §3.2 |
| Left behind | `STRUCTURE-PASS` (`main`'s structure pass and its lints); `main`'s `docs/history/` but for its decisions | Read on `main` at `d89dde5de`. `STRUCTURE-PASS` §6, the dependency policy F71 names, moves into `docs/deps/README.md` with the domain's copy |

`docs/guide/` is a build input: `faraday-guide` reads its pages with `include_str!`, so it comes
with that crate, and the relative path follows wherever the manifest puts the crate (inventory
`features/guide.md`).

### 4.5 History

- **Rule DOC6.** How something was built goes to `docs/history/`, never into a Reference or Spec.
  A decision or a document is moved there, never deleted. Held by `lint-docs` (rule 4) and the
  impact refusal `docs.no-silent-delete`.
- `next`'s `docs/history/` starts with `main`'s `docs/history/decisions/` (§5.2) and nothing
  else. `main`'s other history documents stay at `main`'s frozen commit and at the tag
  `history/docs-2026-11` that `main` already names.

### 4.6 How documents are checked

`lint-docs` carries `main`'s rules, numbered as there, with these changes.

| Rule | On `main` | On `next` |
|---|---|---|
| 1 Header | `**Kind:**` and `**Status:**` on every document | §4.1's headers; Migration by folder; `docs/history/` carries `**Kind:** History` or a Record header |
| 2 Paths | every backticked repository path resolves in the tree; a `local/` path fails | in a Reference, as on `main`. In a Spec (the specification's sections and the carried Specs of DOC5), a Record and a Migration document, a path that does not resolve in `next`'s tree must resolve in `main`'s tree at `d89dde5de`, read with `git ls-tree -r --name-only d89dde5de`; when that commit is not in the clone the rule fails and says so |
| 3 Open | a Spec has `**Open:**`, a Reference none | unchanged |
| 4 No plan in a Reference | unchanged | unchanged |
| 5 Decisions | one row per decision file, none twice, a number may be missing; each row in the folder its status names; rows made by `decisions-index.py` | unchanged; and a decision numbered above F335 has a `**Held by:**` of §5.6's form |
| 7 Recipes | every `just <recipe>` shown names a recipe | unchanged |
| 8 No work list | no Batch column, no batch on an `**Open:**` line | unchanged |
| 9 Index | the "Every document" list is what `docs-index.py` makes | unchanged, with §4.2's lines |

**Decision: a citation of `main` is checked against `main`'s frozen tree, not skipped.** Reason:
the specification and the decisions name `main`'s files as evidence, and an unchecked path rots
unseen. Turned down: skipping every path marked "on `main`", which a typo passes. `d89dde5de` is
not an ancestor of `next`, so the check's checkout fetches it; section 10 says how it stays
reachable after the cut-over.

The other document lints carry as they are, re-pointed at `next`'s paths: `docs-index.py`,
`decisions-index.py`, `lint-guide` (the guide's bold names are the app's strings, its links
resolve) and `lint-tree`.

## 5. The decisions record

### 5.1 Numbering goes on

**Decision: `next`'s decisions continue `main`'s F-series, above its highest number, F335.**
Reason: the crates and documents `next` copies cite about 150 distinct decision numbers (F70,
F71, F136, F177 and the rest), and after the cut-over a restarted series would give one number
to two decisions; `lint-docs`' rule 5 holds as it is. Turned down: starting again at F1 or with
a new letter, which needs every copied citation rewritten or qualified.

- **Rule DEC1.** A decision number is used once across both branches and never reused. A new
  decision takes a number above every number either branch has used; a number missing from
  `main`'s series (F256, F262, F267, F271, F274, F289, F321, F330 at `d89dde5de`) stays unused.
  `main` is frozen and records no decision; if a change on `main` ever needs one, it is written
  in `next`'s record and the change on `main` cites it. Held by `lint-docs` rule 5 (no number
  twice) and by review.

### 5.2 What is copied

The change that brings the conventions copies, from `main` at `d89dde5de` and as written:
`docs/DECISIONS.md`, every file of `docs/decisions/` and every file of `docs/history/decisions/`.
A copied record's text is not edited, but for the amendment lines of §5.4. Its paths to `main`'s
code resolve at the frozen commit (§4.6, rule 2). When a feature is ported, the document that
describes it changes; the decisions that bind it do not.

- **Rule DEC2.** A decision stays under `docs/decisions/` only while it binds later changes and
  no Reference, Spec or lint holds all it says; otherwise it moves to `docs/history/decisions/`
  and keeps its row, as Withdrawn, Superseded by F<n> or Replaced by F<n> (F268). Held by review
  and `lint-docs` rule 5 (a row's folder matches its status).

### 5.3 Which of the specification's decisions are records

The specification states many decisions with their reasons. Most are design, held by the rules
and lints beside them, and stay in the specification.

- **Rule DEC3.** A decision of the specification is also written as a record, numbered on, when
  it: replaces or amends a record of `main`; is a security trade-off (more kernel reach, new
  `unsafe`, a new kept copy of secret data); is a difference from Material 3; or accepts more
  than a standard requires. Held by review; `lint-layers` for an `unsafe` entry (section 01
  rule U3).

**Decision: the specification is the record of its own design decisions.** Reason: a record per
design choice would repeat the specification and drift from it, and the record's purpose is what
a later change must not undo without the owner. Turned down: one record per "Decision:"
paragraph of every section.

### 5.4 The decisions the plan calls overturned

`docs/plan/ARCHITECTURE-PLAN.md` §2 lists three choices of `main` as overturned. Section 01
states the decisions (§4.3 there). Each is recorded as its own new decision, and `main`'s records
are changed point by point, by `main`'s own mechanism: a record every point of which falls moves
to history as Replaced by the new one; a record of which some points still bind stays, with an
amendment line at its top naming the decision that amends it and the points it changes, and
its row says "Amended by".

| New decision | What it says | `main`'s records |
|---|---|---|
| Features exchange only messages | A feature is a crate; it reaches another only through messages its parent composite or the root routes; a composite reads nothing in a child's state (section 01 A4 to A8) | Amends F74's last point (phase 7's area structs, glue on `Faraday`, "areas exchanging only messages is not taken"). `STRUCTURE-PASS` §7.3, which also said it, is left behind (§4.4) |
| The vault area splits into use cases | The vault features are separate crates under one vault composite; every decrypted vault byte stays in `faraday-keys` behind its reveals, so the audit of a vault reads `faraday-keys` and `faraday-vault` (section 01 §4.3) | Replaces F170 (every point is about `faraday-core/src/vault/` staying one module and its lint counts) |
| The structure is held by the layers | `Faraday`'s field count, the list of files allowed an `impl Faraday` and `lint-layout`'s code map go with that struct; the structure is held by `lint-layers`, the crate boundaries and the signatures; the cognitive-complexity marker's reason by clippy (§3.1) | Replaces F272 (the glue's place, every point). Amends F74 (its code map, `impl Faraday` and size points; its `lint-secrets`, `cargo vet`, "a ratchet only falls" and "`CLAUDE.md` names each rule and its lint" stand), F196 (points 2 and 4), F248 (point 3's naming of F74's structural rules) and F253 (point 2's naming of `lint-layout` as the reason check) |

Each of the three is written with a `**Why.**` naming the plan's §2 and section 01, and its
`**Decision**` stating the points above in the record's form. Their numbers follow DEC1.

### 5.5 A new decision's form

As on `main`: one file, `docs/decisions/F<n>.md`; its row made by
`python3 -I -B tools/decisions-index.py write` from its header, never by hand.

```markdown
# F<n> <title: what is decided, as a statement> (<date>)

**Kind:** Record
**Status:** Active
**Area:** <one of the record's areas>
**Held by:** <§5.6>

*Amends F<m> (<date>): <which points>; the rest of F<m> holds.*   (only for an amendment)

**Why.** <what made the decision needed, in a paragraph>

**Decision** (owner, <date>; <the document that holds its detail>).

1. **<point>.** <...>
```

### 5.6 What a decision says holds it

- **Rule DEC4.** In a decision numbered above F335, `**Held by:**` names one or more of: a lint by its name in
  `tools/README.md`; `compiler`; `tests (<layer>)`, with a layer of section 07; `review`; or
  `none`. Held by `lint-docs` rule 5. The copied records keep their lines as written; their rule
  ids (`structure.impl-faraday` and the like) are not checked, as on `main` since F258.

## 6. The impact registry

### 6.1 It carries over

**Decision: `tools/impact.toml` and `tools/impact.py` carry over, with `main`'s mechanism
unchanged:** the consumers and the rules, the kinds `pair` and `forbid`, `plan`, `check` and
`lint`, the waiver trailer `Impact-skip: <rule>: <reason>`, and the extra-file extension point
(the repository names and holds no extra file). Reason: what a change must bring with it (a
document's line, a dependency's page, the scope said twice) is still true on `next`, and a lint
on the range is what holds it (F149, F258). Turned down: dropping the registry for `lint-layers`
alone, which checks the code's shape and not a pair of files.

- **Rule IMP1.** `impact.py check` compares a range with its merge base on the branch it lands on:
  `next` until the cut-over, `main` after it. Held by the script's default and by the workflow,
  which passes the pushed or pull request range as on `main`.
- **Rule IMP2.** What no script can see is a checklist in the area's document and in
  `.claude/agents/reviewer.md`, not a trailer (F258); no trailer attests a review. Held by
  review.

### 6.2 Its rules on next

| Rule on `main` | On `next` |
|---|---|
| `repo.security-scope` | carried: `docs/SECURITY-MODEL.md` and `CLAUDE.md`'s Security scope change together |
| `repo.kernel-patches` | carried with the image tree |
| `repo.developing` | carried: a recipe line of the justfile changes `docs/DEVELOPING.md` or `tools/README.md` |
| `docs.readme-index` | carried, its `when` widened to `docs/spec/*.md` |
| `docs.no-silent-delete` | carried, its `when` widened to `docs/spec/*.md` |
| `keys.new-reveal` | re-pointed: a new reveal in `faraday-keys` is in `docs/KEYS.md` and `docs/AUDIT.md` and names its leak test, at the path section 07 gives |
| `keys.forms-registry` | carried |
| `structure.ratchet-raised` | narrowed: its `when` keeps the counts that remain (`lint-tokens.counts`, `lint-crypto.counts`, the touch-target counts if section 07 keeps them) and drops `lint-layout.counts` and `lint-components.counts` |
| `design.new-component` | re-pointed: a new component file in a kit crate is in `docs/DESIGN.md` §4, under its level |
| `design.new-screen` | re-pointed: a new feature's `page` module is in `docs/DESIGN.md` §5 |
| `design.flow-row` | re-pointed: a new step-flow machine (the trait section 03 names) has its row in `docs/FLOW-CONTAINER.md` |
| `design.menu-page` | re-pointed at the menu-page template's crate |
| `deps.page` | carried |
| `builds.docs` | carried with `builds.toml` |

A rule enters `next`'s registry with the change that brings the paths it names; until then it is
absent, not planned. Held by `impact.py lint`, which fails on a rule whose paths match nothing.

### 6.3 Consumers

- The app consumers (`app.faraday`, `app.starter`) are kept, with a third for Faraday Demo, which
  on `next` is a root of its own (section 01 S4). `impact.py lint` compares each with its root's
  resolved workspace crates from `cargo metadata`, as `main`'s does, and fails on a crate the
  root is built from that no pattern reads.
- The website's consumers enter `next`'s registry with the change that points the website at
  `next`, each re-pointed at `next`'s paths. Until then the website reads `main`.

## 7. Strings, the test header, commit messages

### 7.1 User-facing strings

- **Rule W1.** Every user-facing string is in the wording crate's `src/en/`, but the guide's pages
  (`docs/guide/`) and the self-test's check names (inventory `features/dev-diagnostics.md`). Held
  by `lint-strings`, over every application, kit and adapter crate, and by section 01 rule V4
  for what a model carries.
- **Rule W2.** Text in the product is plain statement of fact, never mannered prose; a working
  screen carries labels, values and actions, never an explanation (that is the guide and
  About). Held by review.

The wording crate's name and path are the manifest's (section 09); the rule names it by role.

### 7.2 The test header's home

`CLAUDE.md` carries §3.6's bullet and names section 07 for the detail: the header's exact form,
the layers a header may name as its Owner, and the lint that refuses a test without a header or
an invariant named twice. When section 07 becomes a Reference, the bullet names it at the same
path.

### 7.3 Commit messages

- **Rule G1.** The subject is one line stating what is now true, in plain words, with no type
  prefix (`fix:`, `feat:`) and no tracker or work number. For a change a person using Faraday
  would notice, it says what changed for them; for any other change (a layer, a stub, a lint, a
  document), what is now true of the repository ("The unlock feature refuses a wrong passphrase
  in its update"; "Specification §01: the layers and their dependency rules").
- **Rule G2.** The body says why, then the test that holds it, by its invariant. A decision is
  cited after the words that say what it is about, number in brackets ("a vault whose passphrase
  was just set ends locked (F301)"). A screen change has a line `Checked against Material 3:
  <the Material pages>`. A change timed under §3.3 gives the time and how it was measured.
- **Rule G3.** The only trailers the repository's tools read are `Impact-skip: <rule>:
  <reason>`. No trailer attests a review.
- **Rule G4.** A commit message holds no personal information (C3). Held by the commit hooks.

Rules G1 and G2 are held by review; `CONTRIBUTING.md` states them for anyone sending a change.

## 8. The lints next keeps, drops and adds

### 8.1 Carried, re-pointed at next's paths

`lint-secrets` (on every application, kit and runtime crate, section 01 K-2), `lint-strings`,
`lint-tokens`, `lint-docs` (§4.6), `lint-leak-reveals`, `lint-tree`, `lint-fuzz-parsers`,
`lint-builds`, `costs-lint` (section 01 R2, V2), `lint-standards`, `docs/crypto/verified.py`,
`docs/crypto/check.py`, `docs/crypto/gen.py`, `lint-upstream`, the cross-check's `facts.py` and
`judges.py harness`, `lint-themes`, `lint-guide`, `impact.py lint` and `check`, `lint-all`,
`lint-op-cost`, `lint-release`, fmt, clippy, cargo deny and cargo vet. `lint-keys-tests` is
re-pointed at the test-support crates' features (section 01 §7.3): no test feature is on in the
normal graph of anything a root ships.

### 8.2 Dropped or folded

| Lint on `main` | Why | What holds its rule on `next` |
|---|---|---|
| `lint-layout` | its code map is of `faraday-core`'s modules, its lists of `Faraday`'s fields and of files allowed an `impl Faraday` go with that struct | `lint-layers`; clippy's `allow_attributes_without_reason` for the marker's reason (§3.1) |
| `lint-components` and its own test | its rules find hand-made copies of the standard parts inside one crate | the templates, organisms and parts (§3.2), held by K6, K8 and A3 in `lint-layers` |

### 8.3 New

- `lint-layers`: section 01.
- The test-header lint: section 07.

### 8.4 The reviewer's checklist

`.claude/agents/reviewer.md` carries, its checklist rewritten for `next`: "Repository" as on
`main`; the layers (an edge `lint-layers` allows but a feature reaching state through `Ctx` it
should not; a message that copies a secret by a type that is `Clone` by mistake); and a test that
names an invariant its layer does not own. Its list of critical code (signing and PSBTs, keys,
cryptography, vaults and backups, parsers of what the device takes in, boot integrity) is
unchanged.

## 9. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| C1, C2 | `main`'s rules kept unless named here; `CLAUDE.md` states them | review |
| C3 | nothing about people, machines or the work's organisation | `lint-docs`; the commit hooks; review |
| §2 | the rules carried unchanged | as on `main` (its table) |
| §3.1 | the layers replace `impl Faraday` and area folders | `lint-layers`; the compiler; clippy |
| §3.2 | standard parts are templates, organisms and parts | `lint-layers` (K6, K8, A3) |
| §3.3 | drawing reads; only the runtime mints `Compute` | signatures; `costs-lint`; `clippy.toml`; review for timing |
| §3.4 | secrets in the one-way design | section 01 §8 |
| §3.5 | runs to the end; no `todo!()` in a release | the justfile; `lint-all`; `lint-release` |
| §3.6 | the test header; no test deleted to pass | the test-header lint; review |
| DOC1 to DOC6 | kinds, the map, Specs and References, carried documents, history | `lint-docs`; `docs-index.py`; the impact pairs; review |
| DEC1 to DEC4 | the series goes on; what is copied; which decisions are records; Held by | `lint-docs` rule 5; `decisions-index.py`; review |
| IMP1, IMP2 | the registry's base; no trailer attests | `impact.py`; review |
| W1, W2 | strings in the wording crate; plain text | `lint-strings`; V4; review |
| G1 to G4 | commit messages | review; the commit hooks |

## Changes asked of earlier sections

- `docs/spec/README.md`: add `**Kind:** Spec` and an `**Open:** Not built.` line under its
  title, so that it passes §4.1's header rule (it is the specification's index, a Spec in
  sections).
- `docs/plan/ARCHITECTURE-PLAN.md` (not a section, but on `next`): its `**Kind:** Draft` and
  `**Status:**` lines are not a kind of §4.1. Under §4.1 the folder sets its kind (Migration),
  so the two lines are dropped or the first reads `**Kind:** Migration`. Its direction is not
  touched.

## Questions for later sections

- Section 07: the test-header lint's name; whether the touch-target ratchet (`lint-touch-targets`
  on `main`, a count of press targets under 48 dp over the tours) and `lint-check-tiles` (every
  Check row's flow has a Tools tile) carry, and how, given that a screen is a reviewed tour; the
  path of the leak tests that the impact rule `keys.new-reveal` names.
- Section 09: the paths that replace `faraday/faraday-strings/src/en/`, the guide's
  `include_str!` path, the tour root's recipe names, and `next`'s top-level layout for
  `README.md`'s "Repository layout" and `lint-tree`.
- Section 10: how `main`'s frozen commit `d89dde5de` stays reachable after the cut-over (a tag),
  since `next`'s records and specification resolve paths there (§4.6) and `next` does not
  descend from it; which change moves `docs/plan/` and `docs/migration/` to `docs/history/`; and
  whether a ported module's commit names the module of `main` it came from.
- Section 10: how `docs/crypto/actions.toml` and `docs/crypto/check.py` treat an action whose
  code is not yet on `next`: the manifest names over two thousand paths on `main`, many in
  `faraday-core`, and `check.py` checks them against the tree.
- Section 03: the name of the step-flow trait that the impact rule `design.flow-row` matches.
