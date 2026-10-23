# Working in this repository

These are the current rules for anyone, human or agent, changing this
repository. They are versioned here so that when one changes, this file
changes and nothing else has to be kept in step.

- The design system is `docs/DESIGN.md`.
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
- A new dimension is a name in `core/faraday-draw/src/tokens.rs`. The
  screens still hold numbers written in place from before the rule was
  checked there; `tools/lint-tokens.counts` holds how many per crate,
  `tools/lint-tokens.sh` fails if a count rises, and a change that names
  numbers lowers the count.
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
    logic in `mod.rs`, its screen in `screen.rs`. Screens that belong to
    no area are `screens/<name>.rs`. `lib.rs`'s code map names every
    module and every screen's file (`lint-layout`).
  - No new monolith: files over 2,500 lines and functions over 100
    lines are counted and only fall (`lint-layout`,
    `tools/lint-layout.counts`); a new long function is split, not
    given an `expect`.
  - A dependency passes `cargo vet --locked` (F71): trusted by a
    publisher F71 names, covered by an imported audit, or an exemption
    that its `docs/deps/` page calls "not reviewed"; every direct
    dependency has that page. Builds and checks pass `--locked`.
  - Key material lives in `faraday-keys` and leaves only through its
    eleven named reveals (F70, `docs/KEYS.md`); `lint-secrets` rule 4
    holds it, and `lint-keys-tests` keeps the test helpers that reach a
    key out of shipped binaries.
  - Costly work takes the compute token (`costs-lint`), every string is
    in one file (`lint-strings`), numbers written in place in screens
    only fall (`lint-tokens`), docs carry a kind and status header,
    name only paths that exist and keep plans out of a Reference
    (`lint-docs`).
  - Areas reach each other only through what their functions are given;
    flows that span areas are explicit glue on `Faraday` (phase 7).
- No hosted CI, no GitHub Actions, no metered external services.
- `local/user-notes.txt` is the owner's own notes; agents ignore it.
- The remote machine that builds the images is "the build machine" in
  everything written here: docs, comments, commit messages. Never its
  hostname, its owner's name or its address.
- Before pushing, check every new commit for personal information:
  names, email addresses, hostnames, IP addresses, local paths, keys
  and tokens, in the diff and in the commit message.
- Do not use `rm`; use `trash`. Use absolute paths in shell commands.
- Agents do not commit and do not spawn agents; the orchestrator reviews
  and commits.
- The orchestrator hands each task to an agent on the model it needs:
  Opus for work that takes decisions or judgement (design, a new flow,
  a spec, a review), Sonnet for mechanical work whose result can be
  checked (a specified change, a rename, making tests pass), Haiku for
  menial read-only work (finding files, listing call sites, reading
  logs).
