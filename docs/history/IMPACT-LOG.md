# What a change brings with it: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/IMPACT.md` on 2026-10-28, when its last batch
was built. Each keeps its original number; the source keeps a one-line stub
under the same heading. Sections from 1.3 on, below section 11, were moved out on
2026-11-14 (F258), when the review, held and trailer rules, the generated list
of checks and the report were removed.

## 10. Order of work

Each batch is its own commit, for Sonnet, with `just` passing at its
end. While working, the batch runs `python3 -I -B tools/impact.py` on
the tree; nothing here compiles.

1. **The script's core.** tools/impact.py: reading the registry and the
   extra files, the matcher of §1.5 with a self-test (`impact.py
   --self-test`, as `tools/lint-guide.py` has: each pattern form, `on`,
   `lines`, `rises`, captures), `lint` (§2.5), `plan` and `check` with
   the trailer and `--message`, `--paths`. A registry with the checks
   of `just lint` and the rules of kind `held` only. Check: `impact.py
   lint`; `impact.py check` over the last twenty commits of `main` runs
   and prints.
2. **The rules.** The rest of §9.1 and the consumers of §9.2; the app
   consumers' lint against `cargo metadata` (run with `--offline`).
   Check: `impact.py check` over the last fifty commits of `main` prints
   each unmet pair; each is looked at, and a rule that is wrong for a
   good commit is narrowed (its `lines`, its `on`) until the commits
   that kept the rule pass. The list of what remained unmet goes in the
   batch's report: those are the stragglers this was built to find.
3. **CHECKS.md and `just lint`.** `impact.py docs`; docs/CHECKS.md;
   `just lint` runs `check --quiet` and `docs --check`; the rows in
   `docs/README.md`; `tools/README.md`'s table and lints list;
   `CLAUDE.md`'s line (§12); `docs/DEVELOPING.md` §2 says what the
   check needs (Python 3.11 or later, git).
4. **`report` and `--consumer`.** `report`; `check --consumer website
   --site --site-since` and its output; a self-test over a fixture
   range.
5. **Held by.** `lint-docs` rule 6 (§8); the line in this decision, and
   in the earlier decisions if the owner agrees (Q1).
6. **The website's step** (in the website's repository). Its publish
   script's record; `just faraday` and the publish script calling
   `--consumer website`; its `CLAUDE.md`'s line.

Estimate: batches 1 and 2 are the work, about 900 lines of Python and
350 of TOML; 3 to 6 are small.

## 11. Questions for the owner

Answered (owner, 2026-10-28):

1. **Q1.** The active decisions before this one take a `**Held by:**`
   line too, in one batch (batch 5).
2. **Q2.** A screen change carries a `Material: <page>` trailer:
   `design.material3` is a `trailer` rule (§1.3, §3).
3. **Q3.** The consumers a script can tell are done refuse a publish
   (roles, wordmark, start pages, download's hardware ids, the standards
   page, the guide); the pictures (tour, Home) are listed and never
   refuse.
4. **Q4.** `deps.page` triggers on a new dependency name only, not on a
   version bump.
5. Rules on files git does not track are checked with `--paths`, from
   the files changed since the last run by their modification times
   (§2.2).

### 1.3 Kinds

As it was, with the `held`, `trailer` and `review` kinds (removed by F258):

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

As it was, with its checks of `held_by` and the `[[check]]` tables (removed by F258):

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
- every command of the `lint` recipe in `justfile`, and of the recipes
  it depends on (`lint-clippy`, `lint-deps`, `lint-repo`), is a
  `[[check]]` whose `stage` is `just lint`, and every such check is a command of the recipe;
- every rule of kind `held` names at least one check whose `status` is
  `built`.

## 3. The trailer

As it was, with the `Material:` and `Reviewed:` trailers (removed by F258):

A commit waives a rule it does not need to meet with a trailer:

```
Impact-skip: docs.readme-index: the document is a page of docs/guide/, which `docs/GUIDE-IN-APP.md` describes
```

One trailer a rule, the reason in words a reviewer can check. A waiver
in any commit of the range covers the range; `check` prints each waiver
it used with its commit. Before a commit exists, `--message <file>`
(or `FARADAY_IMPACT_MESSAGE=<file>`, for a check run inside `just`)
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
changes (F216, F14, F18, F66, F68 to F74, F79, F90, F91, F92, F108,
F111, F112, F116, F117, F127, F136, F145, F148, F234). Kinds:
H held, P pair, F forbid, T trailer, R review. "planned" names a check F148 or
§9.3 builds.

| Id | Triggers on | Kind | Change with it, or what holds it | Source |
|---|---|---|---|---|
| `repo.ci-check` | `.github/workflows/**`, `.github/actions/**` | R | runs `just` and nothing else, read-only, actions pinned by commit | F183 |
| `repo.build-machine-unnamed` | any change | R | the build machine is "the build machine" in docs, comments and messages | `CLAUDE.md` |
| `repo.personal-information` | any commit | R | its diff and message checked for names, addresses, hostnames, paths, keys | `CLAUDE.md` |
| `repo.upstream-name` | any text file but `LICENSE` | R | the upstream project named only in `LICENSE` (planned check reads the name from `LICENSE`) | F73 |
| `repo.contributing` | `CLAUDE.md` | R | `CONTRIBUTING.md`'s summary of the rules | `CONTRIBUTING.md` |
| `repo.developing` | a recipe line of `justfile` or `faraday/faraday.just` (`lines`) | P | `docs/DEVELOPING.md` or `tools/README.md` | `CLAUDE.md` |
| `repo.tests-kept` | a removed `#[test]` or test file (`lines`, `on = delete`) | R | changed to the rule it now encodes, never deleted to pass | `CLAUDE.md` |
| `repo.tests-user-facing` | `**/tests/**`, test modules | R | no test states a design rule, a token's value or geometry against a document | `CLAUDE.md` |
| `tree.top-level` | a top-level entry added | P | `README.md` ("Repository layout") and `tools/lint-tree.sh` | `CLAUDE.md`; H by `lint-tree` for the list |
| `tree.clean` | any path | H | `lint-tree` (leftovers, 2 MiB) | `CLAUDE.md` |
| `tree.image-metadata` | `**/*.png` added or changed (the only image form tracked) | R | no metadata (planned check: every tracked image) | `tools/README.md` |
| `docs.headers` | `docs/**/*.md` | H | `lint-docs` rules 1, 3, 4 | `CLAUDE.md` |
| `docs.paths` | `**/*.md` | H | `lint-docs` rule 2 | `CLAUDE.md` |
| `docs.readme-index` | `docs/*.md` added or deleted, or its Kind line changed | P | `docs/README.md` | `docs/README.md` |
| `docs.spec-done` | a Spec's `**Kind:** Spec` line removed from a document that stays (`lines`, `on = modify`) | P | its plan in `docs/history/{NAME}-LOG.md`, Kind Reference | `CLAUDE.md`; H in part by `lint-docs` rule 4 |
| `docs.no-silent-delete` | a file of `docs/decisions/` or a `docs/*.md` deleted | F | refused unless the same range adds it under `docs/history/` (`docs/history/decisions/F{n}.md`, `docs/history/{NAME}.md`: a move) | F149 |
| `decisions.indexed` | `docs/decisions/F{n}.md`, `docs/history/decisions/**` | H | `lint-docs` rule 5 | `CLAUDE.md`, `docs/DECISIONS.md` |
| `decisions.held-by` | `docs/decisions/F{n}.md` added | H | `lint-docs` rule 6 (§8) | F149 |
| `structure.ratchet-raised` | `tools/lint-layout.counts`, `tools/lint-tokens.counts` (`rises`) | P | a decision `docs/decisions/F{n}.md` naming the raised key | `CLAUDE.md`, F74 |
| `structure.code-map` | `faraday/faraday-core/src/**` | H | `lint-layout` | `CLAUDE.md`, F74 |
| `structure.size` | `**/*.rs` | H | `lint-layout` markers, clippy's `cognitive_complexity` | `CLAUDE.md`, F74, F253 |
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
| `standards.out-of-app` | `core/*/src/**` | R | unpublished or unreviewed work compiled out of the app | F234, `docs/STANDARDS.md` §0 |
| `standards.scope` | a new kind of key or signature | R | a decision | F116 |
| `crypto.crosscheck-stamp` | the stamp's files, before a release | H | `lint-crosscheck` | F79, F197 |
| `crypto.tested` | `core/*/src/**` | R | tested against published vectors or another implementation (planned: F148's inventory) | `CLAUDE.md`, F148 |
| `crypto.action-covered` | `core/*/src/**`, `faraday/faraday-keys/src/**` | H, planned | F148's inventory | F148 |
| `crypto.generated` | `docs/STANDARDS.md` §1 to §7 and §10, `docs/AUDIT.md` §3 | H, planned | F148's `gen.py check` | F148 |
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
| `strings.medium` | `faraday/faraday-strings/src/en.rs` | R | the medium's words from `Medium`; "card or stick" on the Pi | F216 |
| `design.material3` | `**/screen.rs`, `faraday/faraday-core/src/screens/**`, `core/faraday-draw/src/widgets/**`, `faraday/faraday-ui/**`, `faraday/faraday-starter/src/**` | T | a `Material: <page>` trailer naming the Material page checked (§3); a difference has a decision | `CLAUDE.md`, F91, F92, F108 |
| `design.left-out` | the same | R | none of F111's components | F111 |
| `design.new-component` | `core/faraday-draw/src/widgets/*.rs` added | P | `docs/DESIGN.md` (§4) | `CLAUDE.md` |
| `design.new-screen` | a `Screen` or `Sheet` variant added in `faraday/faraday-core/src/lib.rs` (`lines`) | P | `docs/DESIGN.md` (§5) and `CLAUDE.md`'s count | `CLAUDE.md` |
| `design.demo-matches` | `faraday/shells/window/**`, `faraday/faraday-host/**` | R | differs from the device only by a decision | F117 |
| `design.maker-pages` | the Starter's maker pages | R | several families at once, no model, no maker's screenshot or name | F14, F18 |
| `tokens.named` | `core/faraday-draw/src/**`, the screens | H | `lint-tokens` | `CLAUDE.md` |
| `themes.generated` | `faraday/faraday-ui/src/pal/**`, `tools/themes/**` | H | `lint-themes` | F113 |
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

### 9.3 Checks proposed beside it

Rules marked "planned check" above, each a script or a test of its own,
outside this spec's batches and each on the owner's word: every tracked
image metadata-free (`tree.image-metadata`); holds listed with their
decision (`standards.no-way-past`); the upstream name read from
`LICENSE` and refused elsewhere (`repo.upstream-name`); every `just`
recipe a document names exists (`repo.developing`); 48 dp touch targets
over the snapshot tours (`design.material3`). The crypto ones are F148's.

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
