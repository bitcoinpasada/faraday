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
  components.
- Every dimension is a name in `core/osk-ui/src/tokens.rs`;
  `tools/lint-tokens.sh` fails the build otherwise.
- Every user-facing string is in
  `opensigner/opensigner-core/src/strings/en.rs`, with two exceptions: the
  Learn pages (`core/osk-learn/src/en.rs`, edited as `docs/learn/`) and
  the self-test's check names (`core/osk-selftest`).
- A working screen carries labels, values and actions, never an
  explanation. Explanations live in Learn and About, which are not
  working screens.
- Text anywhere in the product is plain statement of fact, never
  mannered prose.
- `just` is the check: tests, clippy with warnings as errors, fmt and the
  lints in the `lint` recipe. Keep every test passing or change it to the
  rule it now encodes; do not delete a test to make the build pass. Run
  it once, at the end; while working, run the one test file you touched
  (`cargo nextest run -p opensigner-core --test keep`). To look at a
  screen, render one script at one size (`just snap load-key 480x640`);
  `just snapshots`, all scripts at all sizes, is for a change to the
  layout engine itself, not for a change to a screen.
- Tests cover user-facing behaviour: what a person using the device or a
  wallet talking to it would notice, and correctness against published
  vectors and other implementations. A test never states a design rule, a
  token's value, a library's or the language's own behaviour, or a
  comparison of a screen's geometry against the design document.
- No hosted CI, no GitHub Actions, no metered external services.
- `local/user-notes.txt` is the owner's own notes; agents ignore it.
  `local/REMAINING.md` is the short list of remaining work, kept up to
  date by whoever finishes, adds or changes an item.
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
