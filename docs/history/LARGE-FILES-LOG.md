# Large files on a stick visit: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/LARGE-FILES.md` on 2026-10-22. Each keeps
its original number; the source keeps a one-line stub under the same
heading.

## Header and introduction (from docs/LARGE-FILES.md)

Spec v0.2 · 2026-10-14 · drafted from the owner's decision
(`docs/DECISIONS.md` F26), with a security review's fifteen findings
applied the same day. Not built.

This applies to every file read on a stick or SD card visit, not only
to APKs. `docs/APK-SIGNING.md` is the first user.

## §0 Rules for the implementing agent (from docs/LARGE-FILES.md)

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of a batch; no `rm`; no commits; no agents.
- Batches in §12's order; one review and one completion report each.
- The privilege split of `docs/history/PLAN.md` §4.3 stays as it is: the disk process
  reads and writes FAT and nothing else; the app decides what a file is
  and hashes what it signs. The disk process still treats the app as
  hostile: no request lets the app rename or remove a file other than
  the one write it has open (§8).
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at. No
  restating the spec.
