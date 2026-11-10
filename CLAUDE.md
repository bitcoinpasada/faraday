# Working in this repository

These are the current rules for anyone, human or agent, changing this
repository. They are versioned here so that when one changes, this file
changes and nothing else has to be kept in step. The rules of one area are
in that area's document, named under "By area": read it before changing
the area. `just lint` holds most rules, and `python3 -I -B tools/impact.py
plan <paths>` lists the ones a change touches.

## Everywhere

- `docs/` holds what is true now or still to be built; how it was built
  is history (`docs/history/`). `docs/README.md` says which document
  answers which question: open it first, then read only the section you
  need. A Reference describes what is built; a Spec holds a feature's
  design and whether it is built, with an `**Open:**` line saying in plain
  words what is not, and carries no work list: no table of work, no labels
  for pieces of work, no estimates (F214). What must come first and which
  files change stay as plain text where they matter to anyone building
  Faraday. The change that builds the last of a Spec's open work moves what
  is only about building it to `docs/history/<NAME>-LOG.md` and makes it a
  Reference; a moved section keeps its heading with one line, ``Moved to
  `docs/history/<NAME>-LOG.md`.``, and sections are never renumbered.
  `lint-docs` holds this.
- Faraday's decisions are numbered F1 onward, one file each
  (`docs/decisions/F<n>.md`), indexed in `docs/DECISIONS.md`: read the
  index, then only the decision you need. A new decision is a new file
  and a row; one replaced, superseded or withdrawn moves to
  `docs/history/decisions/` and keeps its row.
- What a change brings with it is `tools/impact.toml` (F149): `impact.py
  plan <paths>` says it for a task; `just lint` fails on a pair left unmet
  unless a commit says why (`Impact-skip: <rule>: <reason>`); a review
  rule a change triggers is attested with a `Reviewed: <rule>` trailer
  (F151). A new rule line here, or a decision that binds later changes,
  is a rule in the registry in the same commit. `docs/CHECKS.md` is
  generated from it.
- `just` is the check: tests, clippy with warnings as errors, fmt and the
  lints in the `lint` recipe, every lint run to the end (`lint-all`). Keep
  every test passing or change it to the rule it now encodes; do not
  delete a test to make the build pass. The full check is slow, so it runs
  once for a finished body of work, never after each change or each
  commit: GitHub Actions runs it on every push and pull request, and a
  commit on main need not pass it on its own. While working, run only the test module you touched (`cargo
  nextest run -p faraday-core -E 'test(/^keygen::/)'`) and the one lint
  script for the files you changed; when the full check fails, fix what it
  lists and run again only the commands that failed. To look at a screen,
  render one tour at one size (`just faraday-snap 480x640 spend`, into
  `out/snapshots/faraday/480x640/spend/`); `just faraday-snapshots` is for
  a change to the layout engine itself.
- Tests cover user-facing behaviour: what a person using the device or a
  wallet talking to it would notice, and correctness against published
  vectors and other implementations. A test never states a design rule, a
  token's value, a library's or the language's own behaviour, or a
  comparison of a screen's geometry against the design document.
- Every user-facing string is in `faraday/faraday-strings/src/en.rs`,
  except the Learn pages (`faraday/faraday-learn/src/pages/en.rs`, edited
  as `docs/learn/`) and the self-test's check names
  (`core/faraday-selftest`). Text in the product is plain statement of
  fact, never mannered prose; a working screen carries labels, values and
  actions, never an explanation (that is Learn and About).
- `docs/DEVELOPING.md` is how anyone builds, checks, reproduces and tries
  Faraday. A change to any of that (a recipe, a prerequisite, a dev-image
  feature, an output path) changes it in the same commit, and every
  command it shows runs as written. GitHub Actions runs the check (F183):
  `just` on every push to main and pull request, and, started by hand, the
  fuzz run before a release (F210), nothing else; read-only, no secrets,
  actions pinned by commit, no metered services.
- The tree stays clean (`tools/lint-tree.sh`): top-level entries only as
  README.md's "Repository layout" lists them, nothing an editor, a merge or
  a session leaves behind, no file over 2 MiB, build output in `out/` and
  `target/`, scratch files outside the repository.
- The remote machine that builds the images is "the build machine" in
  everything written here, never its hostname, its owner's name or its
  address. Every new commit is checked for personal information before
  it is pushed: names, email addresses, hostnames, IP addresses, local
  paths, keys and tokens, in the diff and in the commit message. A git
  hook that makes this check counts as it; without one, read them by
  hand.
- Do not use `rm`; move files to the trash (`trash` or `gio trash`). Use
  absolute paths in shell commands.

## Keys, cryptography and what the app takes in

- What the app takes in is `docs/STANDARDS.md` §0: a published standard,
  followed and checked against an implementation Faraday did not write,
  or Faraday's own rule that fails closed; work on an unpublished
  standard or an unreviewed construction stays in the repository,
  compiled out of the app.
- `core/` is Faraday's own code: fix a thing at its source rather than
  wrap it. A cryptographic change is tested against published vectors and
  an implementation Faraday did not write (a second one by Faraday's own
  authors is not one); an action of Faraday's own specification carries
  F177's label until an outside review. What the code refuses stays
  refused (a multisig key without an origin, a guessed path, an ambiguous
  input); accepting more than a standard requires is the owner's
  decision, recorded in DECISIONS. A type re-implemented from a `core/`
  one keeps the values the original works out once.
- The full cross-check runs before a release, never on a schedule, and the
  tip is not kept cross-checked (F197): a change to an area's code runs that
  area and commits no stamp or fingerprint; a release passes `just
  lint-release`.
- A check that keeps keys or money safe has no way past it unless a
  decision names a legitimate case (a hold naming it, STANDARDS §0, F136).
  Key material lives in `faraday-keys` and leaves only through its named
  reveals (F70, `docs/KEYS.md`). A dependency passes `cargo vet --locked`
  (F71) and has its `docs/deps/` page; builds and checks pass `--locked`.
- **Security scope** (`docs/SECURITY-MODEL.md`, "What this does not defend
  against"). The rules above are sized by it. Faraday is an offline
  computer, used in a secure place and powered off after use. Everything
  from the online side is untrusted (the host or coordinator, a PSBT, a
  file, a QR code) and the device verifies it; its cryptography is right
  against published vectors. A bug there is fixed now. Timing and memory
  side channels on the offline device (constant-time arithmetic, wiping
  copies in freed memory) are worth having but later work: recorded as
  open work and done when the app is mature, not fixed now. A malicious
  Faraday binary, firmware and the management engines, cold-boot attacks
  and a weak passphrase are out of scope, and so is a USB device that shows
  only as a keyboard or mouse and is plugged in at boot (trusted; a notice
  lists it). Faraday Demo and the Starter run on an online computer and hold
  test data. A risk outside this scope, or later work found in passing, gets
  one line in a report; it is not fixed without the owner asking.

## By area

- **Screens and design:** every app follows Material 3 (F92, F91), drawn
  with Faraday's own look (`faraday-ui`): `docs/DESIGN.md` (the design
  system; its §5 screens, §4 components), `docs/LAYOUT.md`, and for the
  Starter `docs/STARTER.md` §3. Colours are faraday-ui's palette roles; a
  new dimension is a name in `core/faraday-draw/src/tokens.rs`
  (`lint-tokens`: counts only fall). A difference from Material needs a
  decision; a screen review names the Material page it was checked
  against.
- **Drawing and costly work:** drawing reads and does not work things
  out; costly work takes `c: &Compute` and says "Costs …" in its doc's
  first line; work added while drawing is timed once in a late-flow
  session for the batch, before it is pushed: `docs/SPEED.md` §3.3 and §7,
  `docs/COMPUTE.md` (held by `costs-lint` and the budget tests); a named
  costly operation is held by an instruction count, raised only in a
  commit that says why, and a stated cost says how it was measured (F238).
- **Code structure** (F74, F196): an area's code, state and screen in its
  folder, no `.rs` file over 5,000 lines (those over it only fall),
  private area state reached through accessors, `impl Faraday` only as
  glue: `docs/STRUCTURE-PASS.md` §7 (held by `lint-layout`).
