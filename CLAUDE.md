# Working in this repository

These are the current rules for anyone, human or agent, changing this
repository. They are versioned here so that when one changes, this file
changes and nothing else has to be kept in step.

- The design system is `docs/DESIGN.md`. OpenSigner's decisions and open
  questions are `docs/PLANNING.md` §15 and §16. PLANNING.md is about
  10,000 lines; read the section you need.
- Faraday's decisions are `docs/DECISIONS.md`, numbered F1 onward.
  `docs/PLANNING.md` is upstream's copy and is not edited here.
- A screen is one of DESIGN §5's sixteen screens, built from the §4
  components. The Starter's screens are `docs/STARTER.md` §3, built from
  the same components, drawn with Faraday's own look (`faraday-ui`).
- Every dimension is a name in `core/osk-ui/src/tokens.rs`;
  `tools/lint-tokens.sh` fails the build otherwise.
- Every user-facing string is in one file,
  `faraday/faraday-strings/src/en.rs`. Two exceptions: the Learn pages
  (`core/osk-learn/src/en.rs`, edited as `docs/learn/`) and the
  self-test's check names (`core/osk-selftest`).
- A working screen carries labels, values and actions, never an
  explanation. Explanations live in Learn and About, which are not
  working screens.
- Text anywhere in the product is plain statement of fact, never
  mannered prose.
- `just` is the check: tests, clippy with warnings as errors, fmt and the
  lints in the `lint` recipe. Keep every test passing or change it to the
  rule it now encodes; do not delete a test to make the build pass. Run
  it once, at the end; while working, run the one test file you touched
  (`cargo nextest run -p faraday-core --test keygen`). To look at a
  screen, render one tour at one size (`just faraday-snap 480x640 spend`,
  into `out/snapshots/faraday/480x640/spend/`); `just faraday-snapshots`,
  the default tour at every size, is for a change to the layout engine
  itself, not for a change to a screen.
- Tests cover user-facing behaviour: what a person using the device or a
  wallet talking to it would notice, and correctness against published
  vectors and other implementations. A test never states a design rule, a
  token's value, a library's or the language's own behaviour, or a
  comparison of a screen's geometry against the design document.
- `core/` and `opensigner/` are Faraday's to change (a soft fork,
  `docs/UPSTREAM.md`, F65): fix a thing at its source rather than wrap
  it. Each change there is its own commit, listed in `docs/UPSTREAM.md`
  §3 the same day; a cryptographic change is tested against published
  vectors or another implementation. Owning the code never loosens it:
  what OpenSigner refuses stays refused (a multisig key without an
  origin, a guessed path, an ambiguous input); refuse what cannot be
  verified rather than assume; accepting more than a standard requires
  is the owner's decision, recorded in DECISIONS.
- Faraday's own version of an upstream type keeps what upstream's works
  out once and keeps (a key's fingerprint, its share); a spec that
  re-implements an upstream type lists those values.
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
