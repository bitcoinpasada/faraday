# Contributing to Faraday

Reviews, reports and changes are welcome. Faraday holds keys, so every
change is held to the same rules, whoever makes it.

## Reporting

- **A security problem**: privately, as `SECURITY.md` says. Never in a
  public issue.
- **A bug, a screen that is unclear, a flow that leads nowhere**: an
  [issue](https://github.com/bitcoinpasada/faraday/issues), with what you
  did, what you saw and what you expected. For a machine that does not
  boot or work, attach the dev image's boot report (`docs/DEVELOPING.md`
  §6.2, §6.4).
- **An audit**: `docs/AUDIT-SUBMISSIONS.md`.
- **A reproduced build**: an issue with the commit, the sums you got and
  whether they match the release's `SHA256SUMS` (`docs/DEVELOPING.md` §5).

## Sending a change

1. Read `CLAUDE.md`: the rules every change follows, for people and
   agents alike. `docs/README.md` says which document answers which
   question; read only the section you need.
2. Build and check as `docs/DEVELOPING.md` says. While working, run the
   tests of the module you touched; before sending, run `just`, the full
   check, once. A change that fails it is not ready.
3. Keep one change to one thing, with its tests and the documents it
   changes. A user-facing behaviour gets a test of what a person or a
   wallet would notice; a cryptographic change is checked against
   published vectors or another implementation, and the cross-check
   (`just crosscheck`) passes and its stamp is committed with it.
4. Write the commit message as what changed for a person using Faraday
   and why, then the test that holds it.

What `just` holds, among the rest: clippy with warnings as errors and
formatting; every dependency vetted (`cargo vet`, a page in
`docs/deps/`); every user-facing string in
`faraday/faraday-strings/src/en.rs`; the design system's numbers and
colours named, not written in place; no file or function over its size
limit; every document's header and every path it cites; nothing at the
top level that `tools/lint-tree.sh` does not list. `tools/README.md`
lists every lint.

## Design

Faraday and Faraday Starter follow Material 3, drawn with Faraday's
own look (`docs/DESIGN.md`, `docs/LAYOUT.md`, `docs/THEMES.md`). A new
or changed screen uses the components in `docs/DESIGN.md` §4, the type
scale, 4 dp spacing, 48 dp touch targets, and the colour roles; where
Faraday departs from Material, a decision in `docs/DECISIONS.md` says
so.
