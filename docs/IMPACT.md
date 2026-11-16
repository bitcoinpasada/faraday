# Impact: what a change brings with it

**Kind:** Reference
**Status:** Built (F149, narrowed by F258)

A change to Faraday often needs a second change somewhere else: a new
board needs its line in `faraday/builds.toml`, a new document needs its row
in `docs/README.md`, a theme's seed needs the website's colour roles. The
registry, `tools/impact.toml`, lists these pairs, the one refusal (a
decision or a document is moved to `docs/history/`, never deleted) and
who outside the repository reads which paths. One script reads it at three
moments: when a task is planned, when a change lands, and when the website
moves to a new Faraday commit.

The rules that bind a change are written where they come from
(`CLAUDE.md`, `docs/decisions/`, the documents). What no script can see,
whether a screen follows Material 3 or a cryptographic change was tested,
is a checklist in `.claude/agents/reviewer.md` and in the area documents
`CLAUDE.md` names (F258); no trailer attests it.

## 1. The registry

### 1.1 Form and place

tools/impact.toml, TOML read with Python's `tomllib`. Two kinds of table:
`[[consumer]]`, a part of the suite that reads paths of this repository (an
app, the website); and `[[rule]]`, one rule. Ordered by area, the areas in
§1.5's order. Which lint holds what is in `tools/README.md`.

### 1.2 Schema

```toml
[[consumer]]
id = "website.roles"                # suite part; "app.<name>" or "website.<name>"
name = "the website's colour roles"
reads = ["faraday/faraday-ui/src/pal/schemes.rs"]
act = "write the site's roles again from the pinned commit"
site_check = "python3 -I -B tools/roles.py check"   # website only; run in its tree
site_with = ["web/site.css"]        # website only; its paths that show it done
blocking = true                     # website only; false: listed, never refused

[[rule]]
id = "docs.readme-index"            # area.name, lower case, unique, never reused
title = "a document added or removed has its row in docs/README.md"
source = ["docs/README.md"]         # where it is written (public paths;
                                    # a section as "docs/X.md §n")
at = "change"                       # change (§1.4)
when = ["docs/*.md"]
on = ["add", "delete"]              # add | modify | delete; default all three
lines = '^\*\*Kind:\*\*'            # optional: an added or removed line of a
                                    # triggering file matches this regex
rises = false                       # optional: triggers only when a line's last
                                    # number is higher than before, or the line
                                    # is new (a ratchet count raised)
names_raised = false                # optional, with rises and a pair: a partner
                                    # in `with` counts only when its changed lines
                                    # name each raised key
kind = "pair"                       # pair | forbid (§1.3)
with = ["docs/README.md"]           # pair: at least one changed path matches
with_each = []                      # pair: every pattern here is matched
consumers = ["website.guide"]       # optional
misses = ""                         # optional: what the rule does not catch;
                                    # `plan --long` prints it
status = "active"                   # active | planned (a planned rule is
                                    # listed by `check`, never enforced)
```

Fields not given take their defaults: `at = "change"`, `on` all three,
`kind = "pair"`, `status = "active"`, the lists and strings empty.

### 1.3 Kinds

| Kind | Meaning | What check does |
|---|---|---|
| `pair` | when the rule triggers, a path in `with` (any one) and every path in `with_each` change in the same range | fails while unmet and not waived (§3) |
| `forbid` | a change matching `when` is refused, unless a path in `with` (captures filled) changes in the same range, which makes it a move | fails unless waived |

### 1.4 Moments

`change` rules are about what a range of commits, or the working tree,
contains: `check` enforces them. `release` rules are about the steps
before a release (`tools/README.md`, "Before a release"); `plan --at
release` lists them and nothing enforces them here. An extra rule file
(§6) may use further moments of its own; the script lists them by `--at`
and never enforces them.

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
rule's source.

```
impact: 3 paths, 5 rules
change with it:
  docs.readme-index     docs/README.md
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
impact: met docs.readme-index: docs/README.md changed (e5f6a7b8)
impact: 3 rules triggered: 1 unmet, 1 waived, 1 met
```

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
`FARADAY_IMPACT_RANGE=<range>` gives the same range as the argument.
On a push to `main` the check is given the pushed range, so a pair left
unmet there fails as it does on a pull request (F258). With no range it
compares the tip with its merge base on `main`.

### 2.5 `impact.py lint`

The registry checked against itself and the tree, run by `check` before
anything else (exit 2; it prints `impact: lint: <problem>` for each, or
the count of consumers and rules when it is clean). It reads the extra
files of §6 too:

- every table has only the fields of §1.2 with the types given there,
  an `id`, and no table other than `consumer`, `rule` and `amend`;
- ids lower case and unique across consumers and rules, and across the
  extra files with the public one; a consumer's id starts `app.` or
  `website.`;
- a rule's `kind` is `pair` or `forbid`, its `status` is `active` or
  `planned`, each `on` is `add`, `modify` or `delete`, `lines` is a
  regular expression, a `pair` names something in `with` or
  `with_each`, and `names_raised` goes with `rises`, a `pair` and `with`;
- the public rules' areas (the id's first part) are those of §1.5 and
  in its order;
- every `consumers` entry names a consumer;
- every workspace crate `app.faraday` and `app.starter` are built from
  is read by a pattern of that consumer's `reads` (from `cargo metadata
  --offline`; with no cargo it prints a note and skips this);
- every `source` path exists, with its section heading where one is
  given;
- every pattern without `*`, `{` or `!` names a path that exists, and
  every pattern of a `when` matches at least one tracked path, unless
  the rule's `on` is `add` alone.

## 3. The trailer

A commit waives a pair it does not need to meet with a trailer:

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

The `Material:` and `Reviewed:` trailers are gone (F258).

## 4. At landing

`just lint` runs `python3 -I -B tools/impact.py check --quiet`. A fresh
clone on `main` has an empty range and passes. A tree without `main`, or
not a git checkout, says so and passes: the range is whoever lands it to
check. GitHub Actions gives the check the pushed range on a push to `main`
and the pull request's range on a pull request.

What impact adds to `just lint` is the pairs no lint holds, the refusal of
`forbid`, and the consumers a change affects.

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
- Its `faraday` recipe runs the pinned commit's script (`--repo` names the
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
order, by `plan`, `check` and `lint`. An extra file
has the same tables, and one more:

```toml
[[amend]]
id = "repo.developing"   # a rule of the public registry
with = ["a-path-of-the-extra-file"]
consumers = []           # optional
misses = "what it does not catch"   # optional
```

`[[amend]]` adds to a public rule's `with` and `consumers` (a value
already there is not repeated) and appends its `misses` to the rule's;
it never removes, and never changes a rule's kind or its other fields.
An amend naming no rule is a lint error. An extra
file's `source`, `when`, `with` and `with_each` are not checked against
the tree (`lint` skips an extra rule's paths), and its patterns may name paths git does not track; `check --paths`
reaches them. The repository names no extra file and holds none:
`lint-docs` already refuses a document citing the private side, and the
script prints an extra rule's id and its file's name only (`(a rule of
<file>)`, `(a consumer of <file>)`), never the file's other text, in
what `check` writes to a log.

## 9. The first rules

### 9.1 The rules

The pair rules and the one refusal, with the paths each fires on and what
must change with it, are in `tools/impact.toml`; read it by `impact.py plan
<paths>`. The pairs: a change to SECURITY-MODEL's "What this does not
defend against" is made in `CLAUDE.md`'s Security scope too; a kernel patch
is in `docs/KERNEL-PATCHES.md`; a recipe line of the justfile
has its line in `docs/DEVELOPING.md` or `tools/README.md`; a document added or
removed has its row in `docs/README.md`; a new reveal in `faraday-keys` is
in `docs/KEYS.md` and `docs/AUDIT.md`; a registry dependency has its page in
`docs/deps/`; a build in `faraday/builds.toml` is in `docs/IMAGES.md` or
`docs/DEVELOPING.md`; a new widget or screen is in `docs/DESIGN.md`; a raised
ratchet count has a decision naming it. The refusal: a decision or a
`docs/*.md` is moved to `docs/history/`, never deleted.

### 9.2 The consumers

| Id | Reads | When it changes | Blocking |
|---|---|---|---|
| `app.faraday` | `faraday/faraday-core/**`, `faraday/faraday-ui/**`, `faraday/faraday-strings/**`, `faraday/faraday-guide/**`, `faraday/faraday-keys/**`, `faraday/shells/stick/**`, `faraday/shells/window/**`, `faraday/image/**`, `core/**` | the device app, the stick and the Pi images are built again | n/a |
| `app.starter` | `faraday/faraday-starter/**`, `faraday/shells/starter/**`, `faraday/faraday-ui/**`, `faraday/faraday-strings/**`, `core/faraday-draw/**`, `faraday/builds.toml` | the Starter is built again | n/a |
| `website.start` | `docs/guide/start/*.md` | a page added needs its description in the site's guide builder | yes, on add |
| `website.download` | `faraday/builds.toml` | a new hardware id needs its lines and drawing in the site's download builder | yes, on a hardware id added |
| `website.download-pictures` | Home's screen file | the pictures of Home at each panel size made again | no |
| `website.roles` | `faraday/faraday-ui/src/pal/schemes.rs` | the site's colour roles written again | yes (its `site_check` exists) |
| `website.wordmark` | `faraday/faraday-ui/assets/wordmark.txt` | the site's wordmark outlines copied again | yes |
| `website.standards` | `docs/STANDARDS.md` | the standards page edited to match (rendered from the manifest once F148 is built) | yes |
| `website.audits` | `docs/AUDIT-SUBMISSIONS.md`, `docs/crypto/actions.toml`, `SECURITY.md` | the Audits page, written by hand from these, says what they now say | yes |
| `website.tour` | the screens the tour shows | the tour's pictures made again | no |
| `website.demo` | `faraday/faraday-host/**`, `faraday/faraday-samples/**`, `Cargo.lock` | the demo built and its tests run | no; its own build refuses a break |

The app consumers are for plan's "read by" line; which apps a commit
touches is `cargo tree -i <crate>`'s answer, and the patterns above are
its summary: tools/impact.toml names each crate's directory. `impact.py
lint` compares each app consumer with `cargo metadata --offline`, from
the packages `faraday-demo` and `faraday-stick` (the device app) and
`faraday-starter-shell` (the Starter), and fails on a workspace crate
the app is built from that no pattern of its consumer reads. Where cargo
cannot run, it prints a note and skips the comparison.

## 13. What this does not catch

- A pair met by a token change (the paired file touched, its text not
  brought up to date) passes; the waiver's reason is the only record of
  why a pair was skipped.
- A path pattern too wide triggers on changes the rule is not about; a
  waiver given often is the sign of one more.
- What is a checklist (F258) is read by whoever changes or reviews the
  area; nothing records that it was.
