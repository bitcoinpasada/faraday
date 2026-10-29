# Impact: what a change brings with it

**Kind:** Reference
**Status:** Built (F149)

A change to Faraday often needs a second change somewhere else: a string
renamed needs the guide's bold name, a new board needs its line in
`faraday/builds.toml`, a new document needs its row in `docs/README.md`,
a theme's seed needs the website's colour roles. Some of these pairs are
held by a lint in `just lint`; many are held by nobody, written in
`CLAUDE.md`, in a decision or in a document's preamble, and found by a
reviewer or not at all. This spec puts every such rule in one file, the
registry, which says for each: what change triggers it, what holds it
today, what must change with it, who outside the repository reads the
paths it covers, and what nothing catches. One script reads it at three
moments: when a task is planned, when a change lands, and when the
website moves to a new Faraday commit.

The rules that bind a change are also written where they come from
(`CLAUDE.md`, `docs/decisions/`, `docs/STANDARDS.md` §0, the documents);
the registry does not replace them. It is the index of what holds each
one, and the check for the pairs a script can see.

## 1. The registry

### 1.1 Form and place

tools/impact.toml, TOML read with Python's `tomllib`. Three kinds of
table: `[[check]]`, a check that exists (a lint, a test suite, a hook in
the repository, a step of the release); `[[consumer]]`, a part of the
suite that reads paths of this repository (an app, the website); and
`[[rule]]`, one rule. Ordered by area, the areas in §1.5's order.

### 1.2 Schema

```toml
[[check]]
id = "lint-strings"                 # unique; named by rules' held_by
run = "tools/lint-strings.sh"        # the command, as the recipe runs it
stage = "just lint"                 # just lint | just test | just crosscheck
                                    # | release | website | repository hook
catches = "a wording literal in screen code; a string no code uses"
misses = "Learn pages and the self-test's check names, by rule"
status = "built"                    # built | planned (named, not yet run)

[[consumer]]
id = "website.roles"                # suite part; "app.<name>" or "website.<name>"
name = "the website's colour roles"
reads = ["faraday/faraday-ui/src/pal/schemes.rs"]
act = "write the site's roles again from the pinned commit"
site_check = "python3 -I -B tools/roles.py check"   # website only; run in its tree
site_with = ["web/site.css"]        # website only; its paths that show it done
blocking = true                     # website only; false: listed, never refused

[[rule]]
id = "strings.guide-names"          # area.name, lower case, unique, never reused
title = "a string renamed is renamed in the guide's bold names"
source = ["CLAUDE.md", "tools/README.md"]   # where it is written (public paths;
                                            # a section as "docs/X.md §n")
at = "change"                       # change | release (§1.4)
when = ["faraday/faraday-strings/src/en.rs"]
on = ["modify"]                     # add | modify | delete; default all three
lines = '^[+-]\s+[a-z_]+:'          # optional: an added or removed line of a
                                    # triggering file matches this regex
rises = false                       # optional: triggers only when a line's last
                                    # number is higher than before, or the line
                                    # is new (a ratchet count raised)
kind = "held"                       # held | pair | forbid | trailer | review (§1.3)
held_by = ["lint-guide"]
with = []                           # pair: at least one changed path matches
with_each = []                      # pair: every pattern here is matched
trailer = ""                        # trailer: its name, e.g. "Material"
misses = "prose that describes old steps with the current names"
consumers = ["website.guide"]
status = "active"                   # active | planned (its check not built)
```

Fields not given take their defaults: `at = "change"`, `on` all three,
`kind = "review"`, the lists empty, `status = "active"`.

### 1.3 Kinds

| Kind | Meaning | What check does |
|---|---|---|
| `held` | a check in `held_by` holds the rule whole | lists it in plan; nothing more |
| `pair` | when the rule triggers, a path in `with` (any one) and every path in `with_each` change in the same range | fails while unmet and not waived (§3) |
| `forbid` | a change matching `when` is refused, unless a path in `with` (captures filled) changes in the same range, which makes it a move | fails unless waived |
| `trailer` | when the rule triggers, a commit in the range carries the trailer named in `trailer` (`Material: <page>`), with a value | fails while no commit carries it and the rule is not waived (§3) |
| `review` | no script can see whether it was kept | fails while no commit in the range carries a `Reviewed:` trailer naming it and it is not waived (§3); a rule of an extra file (§6), or a standing rule whose only trigger is every file (`**`), is listed in the output and never fails (F151 point 6) |

A `held` or `review` rule may still name checks that hold part of it;
`misses` says what part nothing holds. A rule whose `status` is
`planned` is listed and never enforced.

### 1.4 Moments

`change` rules are about what a range of commits, or the working tree,
contains: `check` enforces them. `release` rules are about the steps
before a release (`tools/README.md`, "Before a release"); `plan --at
release` lists them and nothing enforces them here. An extra rule file
(§6) may use further moments of its own (a push, a publish, anything
posted); the script lists them by `--at` and never enforces them.

### 1.5 Paths and areas

A pattern is a path from the repository's root. `*` matches within one
segment, `**` any number of segments, `{name}` one segment, captured: a
capture in `when` may be used in `with` and `with_each`
(`docs/decisions/F{n}.md`). A pattern starting `!` excludes what it
matches from the list it is in. The script's own matcher does this; no
library.

The areas, in the registry's order: repo, tree, docs, decisions,
standards, crypto, keys, compute, speed, structure, strings, design,
tokens, themes, guide, learn, deps, builds, fuzz, release, impact.

## 2. The script

tools/impact.py, the standard library only, run as `python3 -I -B`. It
reads git through `git` itself (`diff --name-status -M`, `diff -U0`,
`log --format=%(trailers)`).

### 2.1 `impact.py plan <path>…`

What a change to these paths brings with it, for a task's brief. Short
by design: at most one line a rule, grouped, and nothing for a group
that is empty. `--at release` adds the release rules; `--long` adds each
rule's source and misses.

```
impact: 3 paths, 7 rules
change with it:
  docs.readme-index     docs/README.md
  guide.pictures        (needs Reviewed) the guide's pictures, just guide-pictures
held by: lint-strings, lint-guide, lint-tokens, lint-layout
trailer: design.material3 (Material: <the Material page checked>)
needs a Reviewed: <rule> line in the commit: strings.no-explanation
read by: app.faraday, app.starter, website.guide
```

A path may be a directory or a pattern. With no path it reads the
working tree's changes.

### 2.2 `impact.py check [<range>]`

The landing check. The range defaults to the merge base of `main` and
`HEAD`, to `HEAD`; uncommitted and untracked changes are added to it,
so an agent running `just` before its change is committed sees the same
answer. `--paths <path>…` checks a list given instead of git's, for a
tree that git does not track.

For each `change` rule it decides whether the rule triggers (a changed
path matches `when`, with `on` and `lines` where given, captures
filled), then, for `pair` and `forbid`, whether it is met or waived.
Output, one line each, unmet first:

```
impact: unmet docs.readme-index: docs/IMPACT.md added; change docs/README.md
impact: waived builds.website-hardware: "no new hardware id" (a1b2c3d4)
impact: met design.material3: "Material: Lists" (e5f6a7b8)
impact: unreviewed strings.plain: faraday/faraday-strings/src/en.rs; add "Reviewed: strings.plain" once checked
impact: met crypto.kept-values: "Reviewed: crypto.kept-values" (a1b2c3d4)
impact: 12 rules triggered: 1 unmet, 1 waived, 6 held, 2 met
```

A triggered `review` rule of the registry is met by a `Reviewed:` trailer
naming it (§3) and prints `unreviewed` while it is not; a `Reviewed:` for
a rule that is not triggered, is not a review rule or does not exist is
a note, not a failure.

Exit 0 when nothing is unmet, 1 when something is, 2 when the registry
does not read (§2.5). `--quiet` prints only unmet rules and the count.

`--consumer <id>` narrows the answer to one part of the suite: `website`
means every `website.` consumer, `website.guide` one. It prints the
consumers the range affects (a changed path they read, or a triggered
rule that names them), then the triggered rules that name them (§5
shows the output). It needs `--site <tree>` and `--site-since <commit>`
only for the website's side; without them a blocking consumer is
pending, since nothing here can tell it is done. Exit 1 while a
blocking consumer is pending, 0 otherwise: a consumer without
`blocking` is listed and never fails the check.

### 2.3 `impact.py docs [--check]`

Writes docs/CHECKS.md from the public registry alone (§7); `--check`
fails when the file differs from what it would write. Extra rule files
(§6) are never read by `docs`.

### 2.4 `impact.py report [--out <file>]`

The whole map, from the public registry and every extra rule file:
every check with what it catches and misses, every rule by area with
its trigger, kind, checks and misses, the review-only rules on their
own, the consumers and what each reads, and the rules of each moment.
Markdown, in the style of docs/CHECKS.md, to the file or standard
output; a rule or consumer of an extra file is marked `(of <file name>)`,
and the file's other text is never printed. It is for reading, never
committed: a private map is written by whoever sets the extra files.

### 2.5 `impact.py lint`

The registry checked against itself and the tree, run by `check` and
`docs` before anything else (exit 2):

- ids unique across checks, consumers and rules, and across the extra
  files with the public one;
- every `held_by` names a check, every `consumers` entry a consumer;
- every `source` path exists, with its section heading where one is
  given;
- every pattern without `*`, `{` or `!` names a path that exists, and
  every pattern of a `when` matches at least one tracked path, unless
  the rule's `on` is `add` alone;
- every command of the `lint` recipe in `justfile` is a `[[check]]`
  whose `stage` is `just lint`, and every such check is a command of the
  recipe;
- every rule of kind `held` names at least one check whose `status` is
  `built`.

## 3. The trailer

A commit waives a rule it does not need to meet with a trailer:

```
Impact-skip: docs.readme-index: the document is a page of docs/guide/, which GUIDE.md indexes
```

One trailer a rule, the reason in words a reviewer can check. A waiver
in any commit of the range covers the range; `check` prints each waiver
it used with its commit. Before a commit exists, `--message <file>`
reads the trailers from the message being written, so the change and its
waiver are checked together. A waiver is the answer for one change,
never a standing exception: a rule waived again and again is a rule to
change in the registry.

A `trailer` rule is met by its own trailer, which names what was
checked rather than why nothing was:

```
Material: Lists, Top app bar
```

The value is the Material 3 page or pages the change was checked
against (CLAUDE.md, F91, F92); `--message <file>` reads it the same way.

A `review` rule of the registry is met by a trailer naming the rule,
which says the author did the judgement the rule asks for (F151):

```
Reviewed: strings.plain, crypto.kept-values
```

One trailer may name several rules, comma-separated, or there may be
one trailer a rule; a trailer in any commit of the range, or in the
`--message` file, covers the range. `Impact-skip: <rule>: <reason>` waives
a review rule as it does a pair. A rule of a `FARADAY_IMPACT_EXTRA` file
(§6) is exempt: the repository does not know it, so it is listed and
never fails.

## 4. At landing

`just lint` runs `python3 -I -B tools/impact.py check --quiet` and
`python3 -I -B tools/impact.py docs --check`. A fresh clone on `main`
has an empty range and passes. A tree without `main`, or not a git
checkout, says so and passes: the range is whoever lands it to check.

A rule's check is in `just lint` already when its kind is `held`; impact
does not run it again. What it adds is the pairs no lint holds, the
refusals of `forbid`, the review rules a change touched (met by a `Reviewed:` trailer), and the
consumers it affects.

## 5. The website

The website builds from a pinned Faraday commit (its guide, its start
pages, its download page, its colour roles, its standards page, the
demo). Its consumers are `[[consumer]]` tables with `id` starting
`website.`, each naming the Faraday paths it reads, what to do when they
change, a `site_check` run in the website's tree where one exists, and
the website's paths (`site_with`) whose change shows the work done.

The website's side, in its own repository (a script `faraday-impact.sh` in its tools,
its publish script, the file `faraday.published` and its `CLAUDE.md`):

- Its publish script records, at every publish, the Faraday commit it
  published from and its own commit, in a file of its repository.
- Its `just faraday` runs the pinned commit's script (`--repo` names the
  Faraday checkout beside it, whose history is read), on the range from
  the last published commit to the pin: `impact.py check <published>..<pin> --consumer website --site
  <its tree> --site-since <its commit at that publish>`. The registry
  read is the pin's, not the checkout's `HEAD`'s.
- Its output, one line each, pending first, then the triggered rules
  that name the consumers, then the count:

  ```
  impact: consumer pending website.guide (blocking): docs/guide/x.png changed; the guide's pages and pictures are made again
  impact: consumer met website.roles (blocking): site_check passed
  impact: consumer waived website.standards (blocking): "no change to the list" (a1b2c3d4)
  impact: consumer listed website.tour: faraday/faraday-core/src/codex32/screen.rs changed; the tour's pictures are made again
  impact: review guide.pictures: the guide's pictures are drawn again when a screen changes
  impact: 4 consumers affected: 1 pending, 1 waived, 1 met, 1 listed
  ```

- For each `website.` consumer of a triggered rule, the consumer is met
  when its `site_check` passes, or a website commit since that publish
  changes a `site_with` path, or such a commit carries `Impact-skip:
  <consumer id>: <reason>`. It prints each one, met or not.
- The publish script runs the same and refuses while a blocking
  consumer is unmet. A consumer with `blocking = false` is listed and
  never refuses (owner, 2026-10-28: the consumers a script can tell are done refuse; the pictures are listed).

The consumers and what they read are rules of the public registry
(§9.2), so the Faraday side changes them in the same commit as the paths
they read.

## 6. The extension point

The environment variable `FARADAY_IMPACT_EXTRA` holds a list of further
rule files, separated by `:`, read after tools/impact.toml in that
order, by `plan`, `check` and `report`, never by `docs`. An extra file
has the same tables, and one more:

```toml
[[amend]]
id = "repo.personal-information"   # a rule of the public registry
held_by = ["a-check-of-the-extra-file"]
misses = "what that check does not see"
with = []
```

`[[amend]]` adds to a public rule's `held_by`, `misses`, `with` and
`consumers`; it never removes, and never changes a rule's kind. An extra
file's patterns may name paths git does not track; `check --paths`
reaches them. The repository names no extra file and holds none:
`lint-docs` already refuses a document citing the private side, and the
script prints an extra rule's id and its file's name only, never the
file's other text, in what `check` writes to a log.

## 7. docs/CHECKS.md

Generated by `impact.py docs`, never edited by hand: a Reference.

1. **Checks.** A table: id, stage, catches, misses, status.
2. **Rules by area.** For each area, a table: id, what triggers it,
   kind, what must change with it, held by, misses, source.
3. **Review only.** The rules no script holds, with their source: the
   list a reviewer reads.
4. **Read by.** Each consumer, what it reads, what to do when it
   changes.

Its header names the registry as its source. `docs/README.md` answers
"Which check holds which rule, and what must change with what?" with it,
and "Which tool or lint does what?" stays `tools/README.md`'s; the
lints' paragraphs in `tools/README.md` stay as written, a check's
catches and misses in the registry being one line each.

## 8. Where a rule comes from

A rule is written first where it belongs: `CLAUDE.md` for a rule of
work, a decision for an owner's call, a document's section for its
area. The registry indexes it and holds what can be held.

- **Decisions.** From the decision of this spec on, every file in
  `docs/decisions/` carries in its first 12 lines a line `**Held by:**`
  followed by the ids of the registry's rules it sets, or by `review`
  (it binds changes, and no rule fits a script), or by `tests` (its
  behaviour is the app's and the tests hold it; it binds no other
  change; only where a Rust file of the tree names the decision, so a
  test cites what it holds), or by `none` (a one-time act, or a rule another decision
  now carries). `lint-docs` holds it: the line is there in every
  decision numbered from this one, each id it names is a rule of
  tools/impact.toml, and every rule whose `source` names a decision is
  named in that decision's line. A decision that amends another's rule
  changes the rule's `source` to name both.
- **CLAUDE.md.** A new rule line there is a new rule in the registry in
  the same commit; `impact.rules-indexed` (§9) pairs them.
- **A document's rule** (a preamble such as STANDARDS' "kept in step",
  a section such as SPEED §7) gets its rule when it is written; the
  rule's `source` names the section.

The decisions before this one took the line in one batch (§10, batch 5;
§11, Q1).

## 9. The first rules

### 9.1 The rules

From an inventory of `CLAUDE.md`, `CONTRIBUTING.md`, `tools/README.md`,
`docs/STANDARDS.md` §0, `docs/DESIGN.md` §2, `docs/LAYOUT.md` §11,
`docs/STRUCTURE-PASS.md` §7, `docs/SPEED.md` §3.3 and §7,
`docs/COMPUTE.md` §8, `docs/KEYS.md` §2, `docs/CRYPTO-LEARN.md` §14,
`faraday/builds.toml`'s header and the decisions that bind later
changes (F4, F14, F18, F66, F68 to F74, F79, F90, F91, F92, F108,
F109, F111, F112, F116, F117, F127, F136, F145, F148). Kinds:
H held, P pair, F forbid, T trailer, R review. "planned" names a check F148 or
§9.3 builds.

| Id | Triggers on | Kind | Change with it, or what holds it | Source |
|---|---|---|---|---|
| `repo.no-hosted-ci` | `.github/workflows/**` added | F | refused | `CLAUDE.md` |
| `repo.build-machine-unnamed` | any change | R | the build machine is "the build machine" in docs, comments and messages | `CLAUDE.md` |
| `repo.personal-information` | any commit | R | its diff and message checked for names, addresses, hostnames, paths, keys | `CLAUDE.md` |
| `repo.upstream-name` | any text file but `LICENSE` | R | the upstream project named only in `LICENSE` (planned check reads the name from `LICENSE`) | F73 |
| `repo.contributing` | `CLAUDE.md` | R | `CONTRIBUTING.md`'s summary of the rules | `CONTRIBUTING.md` |
| `repo.developing` | a recipe line of `justfile` or `faraday/faraday.just` (`lines`) | P | `docs/DEVELOPING.md` or `tools/README.md` | `CLAUDE.md` |
| `repo.tests-kept` | a removed `#[test]` or test file (`lines`, `on = delete`) | R | changed to the rule it now encodes, never deleted to pass | `CLAUDE.md` |
| `repo.tests-user-facing` | `**/tests/**`, test modules | R | no test states a design rule, a token's value or geometry against a document | `CLAUDE.md` |
| `tree.top-level` | a top-level entry added | P | `README.md` ("Repository layout") and `tools/lint-tree.sh` | `CLAUDE.md`; H by `lint-tree` for the list |
| `tree.clean` | any path | H | `lint-tree` (leftovers, 2 MiB) | `CLAUDE.md` |
| `tree.image-metadata` | `**/*.png` added or changed (the only image form tracked) | R | no metadata; `just guide-pictures` checks the guide's only (planned check: every tracked image) | `tools/README.md` |
| `docs.headers` | `docs/**/*.md` | H | `lint-docs` rules 1, 3, 4 | `CLAUDE.md` |
| `docs.paths` | `**/*.md` | H | `lint-docs` rule 2 | `CLAUDE.md` |
| `docs.readme-index` | `docs/*.md` added or deleted, or its Kind line changed | P | `docs/README.md` | `docs/README.md` |
| `docs.spec-done` | a Spec's `**Kind:** Spec` line removed from a document that stays (`lines`, `on = modify`) | P | its plan in `docs/history/{NAME}-LOG.md`, Kind Reference | `CLAUDE.md`; H in part by `lint-docs` rule 4 |
| `docs.no-silent-delete` | a file of `docs/decisions/` or a `docs/*.md` deleted | F | refused unless the same range adds it under `docs/history/` (`docs/history/decisions/F{n}.md`, `docs/history/{NAME}.md`: a move) | F149 |
| `decisions.indexed` | `docs/decisions/F{n}.md`, `docs/history/decisions/**` | H | `lint-docs` rule 5 | `CLAUDE.md`, `docs/DECISIONS.md` |
| `decisions.held-by` | `docs/decisions/F{n}.md` added | H | `lint-docs` rule 6 (§8) | F149 |
| `structure.ratchet-raised` | `tools/lint-layout.counts`, `tools/lint-tokens.counts` (`rises`) | P | a decision added: `docs/decisions/F{n}.md` | `CLAUDE.md`, F74 |
| `structure.code-map` | `faraday/faraday-core/src/**` | H | `lint-layout` | `CLAUDE.md`, F74 |
| `structure.size` | `**/*.rs` | H | `lint-layout` counts | `CLAUDE.md`, F74 |
| `structure.impl-faraday` | `faraday/faraday-core/src/**` | H | `lint-layout` | `CLAUDE.md`, F74 |
| `structure.no-new-mut` | a `fn …_mut(` or `impl DerefMut` added in `faraday/faraday-core/src/**` (`lines`) | P | `docs/STRUCTURE-PASS.md` (§7.3's list) | `docs/STRUCTURE-PASS.md` §7.3 |
| `structure.areas-private` | `faraday/faraday-core/src/**` | R | an area's state reached through accessors and its own methods | `docs/STRUCTURE-PASS.md` §7.3 |
| `structure.new-lint` | a lint command added to `justfile` (`lines`: a `tools/lint-*`, `tools/check-*`, `cargo deny`, `cargo vet` or `costs-lint` line) | P | `tools/README.md` ("The lints `just lint` runs") and the registry's `[[check]]` | F74; H by `impact.py lint` |
| `keys.boundary` | `faraday/**` | H | `lint-secrets` rule 4, `lint-keys-tests` | `CLAUDE.md`, F70 |
| `keys.new-reveal` | `faraday/faraday-keys/src/session.rs` (`lines` in `Reveal`) | P | `docs/KEYS.md` and `docs/AUDIT.md` | `docs/KEYS.md` §2 |
| `keys.secret-files` | a file starting to hold secrets | R | added to `lint-secrets`' `SECRET_FILES` | `tools/README.md` |
| `compute.token` | `core/*/src/**`, `faraday/**/src/**` | H | `costs-lint`, the budget tests | `CLAUDE.md`, F69 |
| `compute.new-mint` | a `Compute::new` added (`lines`) | P | `docs/COMPUTE.md` (§3's list) | `docs/COMPUTE.md` §8 |
| `speed.kept-in-step` | a field kept on the state for drawing (`lines`, in `faraday/faraday-core/src/**`) | R | the invariant written where it lives, a test that changes the source | `CLAUDE.md`, `docs/SPEED.md` §3.3 |
| `speed.late-flow-timed` | `**/screen.rs`, `faraday/faraday-core/src/drawing.rs`, `faraday/faraday-core/src/screens/**` | R | the late-flow timing before commit | `docs/SPEED.md` §7 |
| `speed.budgets` | the same | H | the budget tests in `faraday/faraday-core/src/work/budget/` | `CLAUDE.md` |
| `standards.what-goes-in` | `core/*/src/**`, the readers of `faraday/faraday-core/src/**` | R | a published standard checked against another implementation, or a fail-closed rule with a decision | `docs/STANDARDS.md` §0 |
| `standards.refusals-stay` | `core/*/src/**` | R | what is refused stays refused; accepting more is a decision | `CLAUDE.md`, F90 |
| `standards.no-way-past` | `core/faraday-psbt/src/**`, a hold added (`lines`) | R | a hold names its decision (planned check: holds listed) | F136 |
| `standards.out-of-app` | `core/*/src/**` | R | unpublished or unreviewed work compiled out of the app | F109, `docs/STANDARDS.md` §0 |
| `standards.scope` | a new kind of key or signature | R | a decision | F116 |
| `standards.learn-in-step` | `docs/STANDARDS.md` | P | `docs/learn/**` (until F148 generates both) | `docs/STANDARDS.md` |
| `crypto.crosscheck-stamp` | the stamp's files | H | `lint-crosscheck` | F79 |
| `crypto.tested` | `core/*/src/**` | R | tested against published vectors or another implementation (planned: F148's inventory and re-stamp) | `CLAUDE.md`, F148 |
| `crypto.action-covered` | `core/*/src/**`, `faraday/faraday-keys/src/**` | H, planned | F148's inventory | F148 |
| `crypto.generated` | `docs/STANDARDS.md` §1 to §7, `docs/AUDIT.md` §3 | H, planned | F148's `gen.py check` | F148 |
| `crypto.at-source` | `core/**` | R | fixed at its source, not wrapped | `CLAUDE.md` |
| `crypto.kept-values` | a type re-implemented from `core/` | R | keeps the values the original keeps | `CLAUDE.md` |
| `crypto.costs-said` | a costly function added | R | "Costs …" first in its doc (H by `costs-lint` where it takes the token) | `CLAUDE.md` |
| `deps.vetted` | `**/Cargo.toml`, `Cargo.lock` | H | `cargo vet --locked`, `cargo deny check` | F71 |
| `deps.page` | a registry dependency line added to a `Cargo.toml` (`lines`; not a `path` dependency, not a package key) | P | a page of `docs/deps/` (the script cannot take the crate's name from the line) | F71 |
| `deps.locked` | a `cargo` line added in `justfile`, `faraday/faraday.just`, `**/*.sh` (`lines`) | R | it passes `--locked` | `CLAUDE.md`, F71 |
| `strings.one-file` | `faraday/**/src/**` | H | `lint-strings` | `CLAUDE.md`, F68 |
| `strings.guide-names` | `faraday/faraday-strings/src/en.rs` | H | `lint-guide` | `tools/README.md` |
| `strings.no-explanation` | `faraday/faraday-strings/src/en.rs` | R | a working screen carries labels, values, actions; F112's one exception | `CLAUDE.md`, F112 |
| `strings.plain` | `faraday/faraday-strings/src/en.rs`, `docs/learn/**`, `docs/guide/**` | R | plain statement of fact | `CLAUDE.md`, `docs/DESIGN.md` §2 |
| `strings.medium` | `faraday/faraday-strings/src/en.rs` | R | the medium's words from `Medium`; "SD card" | F4 |
| `design.material3` | `**/screen.rs`, `faraday/faraday-core/src/screens/**`, `core/faraday-draw/src/widgets/**`, `faraday/faraday-ui/**`, `faraday/faraday-starter/src/**` | T | a `Material: <page>` trailer naming the Material page checked (§3); a difference has a decision | `CLAUDE.md`, F91, F92, F108 |
| `design.left-out` | the same | R | none of F111's components | F111 |
| `design.new-component` | `core/faraday-draw/src/widgets/*.rs` added | P | `docs/DESIGN.md` (§4) | `CLAUDE.md` |
| `design.new-screen` | a `Screen` or `Sheet` variant added in `faraday/faraday-core/src/lib.rs` (`lines`) | P | `docs/DESIGN.md` (§5) and `CLAUDE.md`'s count | `CLAUDE.md` |
| `design.demo-matches` | `faraday/shells/window/**`, `faraday/faraday-core/src/host/**` | R | differs from the device only by a decision | F117 |
| `design.maker-pages` | the Starter's maker pages | R | several families at once, no model, no maker's screenshot or name | F14, F18 |
| `tokens.named` | `core/faraday-draw/src/**`, the screens | H | `lint-tokens` | `CLAUDE.md` |
| `themes.generated` | `faraday/faraday-ui/src/pal/**`, `tools/themes/**` | H | `lint-themes` | F113 |
| `guide.screens-named` | a `Screen` or `Sheet` added | H | `lint-guide` | `tools/README.md` |
| `guide.pictures` | `**/screen.rs`, `faraday/faraday-core/src/screens/**` | R | the guide's pictures drawn again (`just guide-pictures`) | `docs/GUIDE.md` |
| `learn.sync` | `docs/learn/**`, `faraday/faraday-learn/src/pages/en.rs` | H | `tools/learn/sync.py check` | `CLAUDE.md` |
| `builds.list` | `faraday/image/boards/**`, `faraday/image/panels/**`, recipes | H | `lint-builds` | `faraday/builds.toml` |
| `builds.docs` | `faraday/builds.toml` | P | `docs/IMAGES.md` or `docs/DEVELOPING.md` | `faraday/builds.toml` |
| `fuzz.parser-target` | a parser of untrusted input added | R | its fuzz target in the same change (planned: F148's fuzz check) | `tools/README.md` |
| `release.before` | at release | R | `just`, `just crosscheck --fresh`, the fuzz run and its corpus | `tools/README.md` |
| `impact.registry` | tools/impact.toml | H | `impact.py lint`, `impact.py docs --check` | F149 |
| `impact.rules-indexed` | `CLAUDE.md` | P | tools/impact.toml | F149 |

That is 71 rules: 23 held (four of them by a check still to be
built), 15 pair, 2 forbid, 1 trailer, 30 review. `tree.top-level` sees
a file added at the top, not a new directory. `design.new-screen` and
`keys.new-reveal` read an enum variant line of `lib.rs` and
`session.rs`, so another enum's variant added there triggers them too.
Every pair rule is checkable from paths and changed lines. The rules
whose trigger is not a path ("a file starting to hold secrets", "a
parser added", "a new kind of key") are review: plan lists them for any
change to the paths they name.

### 9.2 The consumers

| Id | Reads | When it changes | Blocking |
|---|---|---|---|
| `app.faraday` | `faraday/faraday-core/**`, `faraday/faraday-ui/**`, `faraday/faraday-strings/**`, `faraday/faraday-learn/**`, `faraday/faraday-keys/**`, `faraday/shells/stick/**`, `faraday/shells/window/**`, `faraday/image/**`, `core/**` | the device app, the stick and the Pi images are built again | n/a |
| `app.starter` | `faraday/faraday-starter/**`, `faraday/shells/starter/**`, `faraday/faraday-ui/**`, `faraday/faraday-strings/**`, `core/faraday-draw/**`, `faraday/builds.toml` | the Starter is built again | n/a |
| `website.guide` | `docs/GUIDE.md`, `docs/guide/**` | the guide's pages and pictures made again | yes |
| `website.start` | `docs/learn/faraday/boot/*.md` | a page added needs its description in the site's guide builder | yes, on add |
| `website.download` | `faraday/builds.toml` | a new hardware id needs its lines and drawing in the site's download builder | yes, on a hardware id added |
| `website.download-pictures` | Home's screen file | the pictures of Home at each panel size made again | no |
| `website.roles` | `faraday/faraday-ui/src/pal/schemes.rs` | the site's colour roles written again | yes (its `site_check` exists) |
| `website.wordmark` | `faraday/faraday-ui/assets/wordmark.txt` | the site's wordmark outlines copied again | yes |
| `website.standards` | `docs/STANDARDS.md` | the standards page edited to match (rendered from the manifest once F148 is built) | yes |
| `website.tour` | the screens the tour shows | the tour's pictures made again | no |
| `website.demo` | `faraday/faraday-core/src/host/**`, `faraday/faraday-samples/**`, `Cargo.lock` | the demo built and its tests run | no; its own build refuses a break |

The app consumers are for plan's "read by" line; which apps a commit
touches is `cargo tree -i <crate>`'s answer, and the patterns above are
its summary: tools/impact.toml names each crate's directory. `impact.py
lint` compares each app consumer with `cargo metadata --offline`, from
the packages `faraday-desktop` and `faraday-stick` (the device app) and
`faraday-starter-shell` (the Starter), and fails on a workspace crate
the app is built from that no pattern of its consumer reads. Where cargo
cannot run, it prints a note and skips the comparison.

### 9.3 Checks proposed beside it

Rules marked "planned check" above, each a script or a test of its own,
outside this spec's batches and each on the owner's word: every tracked
image metadata-free (`tree.image-metadata`); holds listed with their
decision (`standards.no-way-past`); the upstream name read from
`LICENSE` and refused elsewhere (`repo.upstream-name`); every `just`
recipe a document names exists (`repo.developing`); 48 dp touch targets
over the snapshot tours (`design.material3`). The crypto ones are F148's.

## 10. Order of work

Moved to `docs/history/IMPACT-LOG.md`.

## 11. Questions for the owner

Moved to `docs/history/IMPACT-LOG.md`.

## 12. Rule line for CLAUDE.md

Added by batch 3:

- What a change brings with it is tools/impact.toml (F149): a rule
  for each "a change to these paths needs this too", what holds it, and
  what reads the paths outside the repository. `impact.py plan
  <paths>` says it for a task; `just lint` fails on a pair left unmet,
  unless a commit says why (`Impact-skip: <rule>: <reason>`). A new rule
  line here, or a decision that binds later changes, is a rule in the
  registry in the same commit. docs/CHECKS.md is generated from it.

## 13. What this does not catch

- A rule a reviewer must judge stays review: the commit must say it
  was judged (`Reviewed:`, F151), but no script can tell whether the
  judgement was made; the trailer is the author's word, on the record.
- A pair met by a token change (the paired file touched, its text not
  brought up to date) passes; the waiver's reason is the only record of
  why a pair was skipped.
- A path pattern too wide triggers on changes the rule is not about;
  batch 2 narrows the first rules, and a waiver given often is the sign
  of one more.
- A rule nobody wrote down is in no registry; the decisions' `**Held
  by:**` line and `impact.rules-indexed` catch only rules written in a
  decision or in `CLAUDE.md`.
