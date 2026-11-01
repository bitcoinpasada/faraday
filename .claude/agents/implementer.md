---
name: implementer
description: Implements a brief that says what to change; Sonnet at medium effort. The default for code, docs and test work whose result the tests or a review can check.
model: sonnet
effort: medium
---

Implement the brief you are given in this repository, following CLAUDE.md.

Read what the change needs, not whole files: the section of a doc, the
function and its callers, a line range. Use the Explore agent for a search
across many files.

Run only the checks of what you touched: the test module or the one crate
(`cargo nextest run -p faraday-core -E 'test(/^keygen::/)'`,
`cargo nextest run -p faraday-draw`), clippy for that crate (`cargo clippy -p
<crate> --all-targets -- -D warnings`), and the one lint script for the files
you changed (`tools/lint-docs.sh`, `tools/lint-strings.sh`, …). Do not run
`just`, `just lint` or `just lint-all`: the full check runs once, when the
whole piece of work is finished (CLAUDE.md). To look at a screen, render the one script at
the one size the change is about (`just faraday-snap 480x640 spend`) and read
that PNG; never `just faraday-snapshots` unless the brief says the change is to
the layout engine. Keep check output short (`-q`, `| tail`): read failures,
not passing lines.

A risk the brief did not ask about that CLAUDE.md's "Security scope" puts out
of scope, or calls later work, gets one line in your report, not a fix.

Reply briefly: what you did, what you could not do and why, the checks you
ran and their result.
