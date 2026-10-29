# Working in this repository

These are the current rules for anyone, human or agent, changing this
repository. They are versioned here so that when one changes, this file
changes and nothing else has to be kept in step.

- The design system is `docs/DESIGN.md`.
- Every Faraday app follows Material 3 (F92, F91): the device app,
  Faraday Demo and the Starter, and the website, whose own repository's
  rules carry the same list. A new or changed screen, component or
  token follows Material 3's guidance (its components, type scale, 4 dp
  spacing, 48 dp touch targets, state layers, colour roles, shape,
  elevation, motion, and the layout rules of `docs/LAYOUT.md`), checked
  against Material's own pages and drawn with Faraday's own look
  (`faraday-ui`). A screen's colours are the colour roles from
  faraday-ui's palette, never a colour written in place. Where Faraday
  differs, a decision says so; a difference without one is a defect to
  fix, and a review of a screen names the Material page it was checked
  against.
- What the app takes in is `docs/STANDARDS.md` §0: a published standard,
  followed and checked against an implementation Faraday did not write,
  or Faraday's own rule that fails closed; work on
  an unpublished standard or an unreviewed construction of Faraday's own
  stays in the repository, compiled out of the app.
- Faraday's decisions are numbered F1 onward, one file each
  (`docs/decisions/F<n>.md`), indexed in `docs/DECISIONS.md`: read the
  index, then only the decision you need. A new decision is a new file
  and a row; one replaced, superseded or withdrawn moves to
  `docs/history/decisions/` and keeps its row.
- `docs/` holds what is true now or still to be built; how it was built
  is history (`docs/history/`). `docs/README.md` says which document
  answers which question: open it first, then read only the section you
  need. A Reference describes what is built; a Spec is planned work and
  carries an `**Open:**` line naming what is not built. The change that
  builds the last of a Spec's open work also moves its plan (rules for
  the implementing agent, batches, estimates, questions for the owner,
  edit lists, the tree as it was before) to `docs/history/<NAME>-LOG.md`
  and makes it a Reference. A moved section keeps its heading, with one
  line under it, ``Moved to `docs/history/<NAME>-LOG.md`.``, so a cited
  section number still resolves; sections are never renumbered.
  `lint-docs` holds the headers, the Open line, no plan or build log in
  a Reference, and the decisions' index against their files.
- A screen is one of DESIGN §5's thirty-three screens (a `Screen`
  variant in `faraday-core`, drawn in the file `lib.rs`'s code map
  names) or one of its sheets (`Sheet`), built from the §4 components.
  The Starter's screens are `docs/STARTER.md` §3, built from the same
  components, drawn with Faraday's own look (`faraday-ui`).
- What a change brings with it is `tools/impact.toml` (F149): a rule
  for each "a change to these paths needs this too", what holds it, and
  what reads the paths outside the repository. `python3 -I -B
  tools/impact.py plan <paths>` says it for a task; `just lint` fails on
  a pair left unmet, unless a commit says why (`Impact-skip: <rule>:
  <reason>`); a review rule a change triggers is attested with a
  `Reviewed: <rule>[, <rule>]` trailer (F151). A new rule line here, or a decision that binds later
  changes, is a rule in the registry in the same commit.
  `docs/CHECKS.md` is generated from it.
- A new dimension is a name in `core/faraday-draw/src/tokens.rs`. The
  screens (Faraday's and the Starter's) still hold numbers, and a few
  colours, written in place from before the rule was checked there;
  `tools/lint-tokens.counts` holds how many per crate,
  `tools/lint-tokens.sh` fails if a count rises, and a change that names
  them lowers the count.
- Every user-facing string is in one file,
  `faraday/faraday-strings/src/en.rs`. Two exceptions: the Learn pages
  (`faraday/faraday-learn/src/pages/en.rs`, edited as `docs/learn/`) and the
  self-test's check names (`core/faraday-selftest`).
- A working screen carries labels, values and actions, never an
  explanation. Explanations live in Learn and About, which are not
  working screens.
- Text anywhere in the product is plain statement of fact, never
  mannered prose.
- `just` is the check: tests, clippy with warnings as errors, fmt and the
  lints in the `lint` recipe. Keep every test passing or change it to the
  rule it now encodes; do not delete a test to make the build pass. Run
  it once, at the end; while working, run the one test module you touched
  (`cargo nextest run -p faraday-core -E 'test(/^keygen::/)'`). To look at a
  screen, render one tour at one size (`just faraday-snap 480x640 spend`,
  into `out/snapshots/faraday/480x640/spend/`); `just faraday-snapshots`,
  the default tour at every size, is for a change to the layout engine
  itself, not for a change to a screen.
- Tests cover user-facing behaviour: what a person using the device or a
  wallet talking to it would notice, and correctness against published
  vectors and other implementations. A test never states a design rule, a
  token's value, a library's or the language's own behaviour, or a
  comparison of a screen's geometry against the design document.
- `core/` is Faraday's own code: fix a thing at
  its source rather than wrap it. A cryptographic change, anywhere in the
  repository, is tested against published vectors or another
  implementation. Owning the code never loosens it: what the code
  refuses stays refused (a multisig key without an origin, a guessed
  path, an ambiguous input); refuse what cannot be verified rather than
  assume; accepting more than a standard requires is the owner's
  decision, recorded in DECISIONS.
- A check that keeps keys or money safe has no way past it unless a
  decision names a legitimate case; that way past is a hold naming the
  case (STANDARDS §0, F136).
- A type re-implemented from a `core/` one keeps what the original works
  out once and keeps (a key's fingerprint, its share); a spec that
  re-implements a `core/` type lists those values.
- A function that costs more than a lookup (an elliptic-curve
  multiplication, a descriptor written out, a QR code made, a file read)
  says so in the first line of its doc comment.
- Drawing reads; it does not work things out. What is parsed or derived
  once is kept on the state with what it describes (a vault's wallet
  records read, an Inbox file's wallet, a plan's map, an address), kept
  in step with its source at every change, and that invariant is written
  where the kept value lives and tested by changing the source. What a
  screen puts together from it is computed once per frame through the
  drawing memo (`docs/SPEED.md` §3.3), and a decision (a press gated, a
  hold released) computes from the state as it is (a `*_now` form),
  never from the memo.
  A batch that adds work done while drawing or after every event is
  timed in a late-flow session before it is committed
  (`docs/SPEED.md` §7). A costly function takes `c: &Compute` and
  counts itself through it, and its doc's first line says "Costs …";
  drawing gets a read-only `View`, which cannot mint a token, so it
  reaches costly work only through `View`'s named accessors
  (`docs/COMPUTE.md`). `tools/costs-lint` fails `just` when this does
  not hold, and the budget tests in `faraday-core/src/work/budget/`
  when a frame drawn again, a pointer move, a press or a key does
  costly work its budget does not name.
- **Structure** (F74; `docs/STRUCTURE-PASS.md` §7). Each rule is held
  by `just lint`; a ratchet count in a counts file only falls, and
  raising one is the owner's decision, recorded in DECISIONS.
  - An area's code is in its folder in faraday-core: its state and
    logic in `mod.rs` (or its types in `area.rs` beside it, where
    `mod.rs` is at its line count), its screen in `screen.rs`. Screens
    that belong to no area are `screens/<name>.rs`. `lib.rs`'s code map
    names every module and every screen's file (`lint-layout`).
  - No new monolith: files over 2,500 lines and functions over 100
    lines are counted and only fall (`lint-layout`,
    `tools/lint-layout.counts`); a new long function is split, not
    given an `expect`.
  - A dependency passes `cargo vet --locked` (F71): trusted by a
    publisher F71 names, covered by an imported audit, or an exemption
    that its `docs/deps/` page calls "not reviewed"; every direct
    dependency has that page. Builds and checks pass `--locked`.
  - Key material lives in `faraday-keys` and leaves only through its
    fourteen named reveals (F70, `docs/KEYS.md`); `lint-secrets` rule 4
    holds it, and `lint-keys-tests` keeps the test helpers that reach a
    key out of shipped binaries.
  - Costly work takes the compute token (`costs-lint`), every string is
    in one file (`lint-strings`), numbers written in place in screens
    only fall (`lint-tokens`), docs carry a kind and status header,
    name only paths that exist and keep plans out of a Reference
    (`lint-docs`).
  - `Faraday`'s top-level field count and the list of files with
    `impl Faraday` only fall (`lint-layout`, `tools/lint-layout.counts`);
    a new `impl Faraday` goes into the area's struct or its glue file.
  - An area's state is a struct with private fields, held in one field
    of `Faraday` (`docs/STRUCTURE-PASS.md` §7.3). Other areas read it
    through accessors returning `&T` and change it through its named
    methods; a new `*_mut` accessor or `DerefMut` on an area is avoided
    (the few that exist are listed there, to be narrowed). Flows that span
    areas are explicit glue: short `impl Faraday` functions in the area's
    own files.
- No hosted CI, no GitHub Actions, no metered external services.
- `docs/DEVELOPING.md` is how anyone builds, checks, reproduces and
  tries Faraday on their own hardware. A change to any of that (a
  recipe, a prerequisite, a dev-image feature, an output path) changes
  it in the same commit, and every command it shows runs as written.
- The tree stays clean (`tools/lint-tree.sh`): the top level holds only
  the entries the lint lists, each named in README.md's "Repository
  layout"; nothing an editor, a merge or a session leaves behind is
  committed (a backup, a log, a copy kept from before a change: history
  is git's); no file over 2 MiB; build output stays in `out/` and
  `target/`, scratch files outside the repository.
- The remote machine that builds the images is "the build machine" in
  everything written here: docs, comments, commit messages. Never its
  hostname, its owner's name or its address.
- Before pushing, check every new commit for personal information:
  names, email addresses, hostnames, IP addresses, local paths, keys
  and tokens, in the diff and in the commit message.
- Do not use `rm`; use `trash`. Use absolute paths in shell commands.
