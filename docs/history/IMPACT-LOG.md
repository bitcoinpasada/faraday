# What a change brings with it: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/IMPACT.md` on 2026-10-28, when its last batch
was built. Each keeps its original number; the source keeps a one-line stub
under the same heading.

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
