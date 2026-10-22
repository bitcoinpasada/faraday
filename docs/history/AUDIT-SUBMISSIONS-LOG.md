# Audits: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/AUDIT-SUBMISSIONS.md` on 2026-10-22. Each
keeps its original number; the source keeps a one-line stub under the
same heading.

## Header (from docs/AUDIT-SUBMISSIONS.md)

Planned with the owner on 2026-10-13 (DECISIONS F19).
`SECURITY.md` and the issue form (`.github/ISSUE_TEMPLATE/audit.yml`)
are built; §8 lists what is not. The process lives here. `SECURITY.md`
and the issue form follow it. The website's Audits page shows it, and
the website repository's HANDOFF has that page.

## §8 To build (from docs/AUDIT-SUBMISSIONS.md)

- [x] The owner turns on private vulnerability reporting (§4); on
      2026-10-13.
- [x] Whether private reports take a file. They do not: the form's
      fields take no attachments (a draft advisory, 2026-10-14). A
      pasted copy, made LF with one newline at the end (§3), came back
      with the same hash as the file.
- [x] `SECURITY.md` at the repository's root: the private route (§4),
      what goes there (critical and high findings), the timeline (§5),
      and a link to this file and the Audits page.
- [x] `.github/ISSUE_TEMPLATE/audit.yml`, an issue form (a template, not
      a GitHub Action) with these fields: the model and its version, or
      "expert"; the commit audited; a name or pseudonym for credit
      (optional); whether a private report was sent, and the hash of
      `private.md`; a text area for the attached `prompt.md` and
      `audit.md`; a box ticked for "these files were checked for
      personal information".
- [x] The Audits page and the bar's **Audits** link (the website
      repository's HANDOFF); published 2026-10-13.
