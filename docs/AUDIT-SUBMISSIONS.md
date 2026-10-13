# Audits: the request, the submission and the record

**Status:** planned with the owner on 2026-10-13 (DECISIONS F19); not
built. The process lives here. `SECURITY.md` and the issue form follow
it. The website's Audits page shows it, and the website repository's
HANDOFF has that page.

## 1. What it is for

Faraday invites audits from experts and AIs and publishes each one at
`https://faradayoffline.app/audits/`. An entry has the submitter's
summary, Faraday's verdict on each finding, and links to the full prompt
and output. Critical and high findings are sent privately and published
once fixed. A hash posted on the day they are sent shows that the file
published later is the one that was sent.

## 2. The request

The text below is published on the Audits page as written. People follow
it for an expert audit. For an AI audit, it is not the prompt: people
write their own prompt and add the list at its end, where the AI reads
it last, closest to writing its output. The copied text starts with a
line saying what it is.

> **What an audit includes.** By an expert or an AI. This list is not a
> prompt by itself: write your own prompt for the audit, then add this
> list at its end.
>
> Faraday's request for this audit. Follow it in the output:
>
> 1. Record what was audited: the repository
>    (`https://github.com/bitcoinpasada/faraday`), the commit
>    (`git rev-parse HEAD`), the date, any tools run, and for an AI, the
>    model and its version.
> 2. State the scope: the parts of the code read, and the parts not
>    read. `docs/AUDIT.md` lists where the cryptography is.
> 3. For each finding, give: a severity, the file and line, what goes
>    wrong, how to trigger it, and whether it was confirmed by running
>    code or only found by reading.
> 4. Write the output as Markdown in two files: `audit.md` for
>    everything except critical and high findings, and `private.md` for
>    critical and high findings only. Leave out `private.md` if there are
>    none.
> 5. End `audit.md` with a summary of at most 150 words: what was
>    examined, what was found, and an assessment of whether the code can
>    be trusted with keys. If `private.md` exists, the summary gives its
>    number of findings without describing them.
> 6. Redact personal information from both files: names, email
>    addresses, usernames, and anything about the computer the audit ran
>    on, including home directory paths, hostnames, IP addresses, the
>    operating system and its version, hardware, and environment
>    variables. Replace each with a placeholder such as `<home>`,
>    `<user>` or `<host>`. Leave unchanged the names and addresses in
>    Faraday's own code.
> 7. If `private.md` exists, hash it last, after step 6: run
>    `sha256sum private.md > private.md.sha256`. If you cannot run
>    commands, do not write a hash; say that it still needs to be made.
>
> Severities:
>
> - **Critical:** a key or seed can be extracted, or the device signs
>   something the user did not approve.
> - **High:** the same, but only under specific conditions.
> - **Medium:** a protection is weaker than intended.
> - **Low:** a defect with no direct security effect.
> - **Note:** an observation, not a defect.

An expert audit has no `prompt.md`; `audit.md` describes its method.
The files carry no names (step 6). A submitter who wants credit gives a
name or pseudonym in the issue form.

## 3. Submitting

All submissions go through GitHub, none by email. A pseudonymous
GitHub account is enough. The steps, as the page gives them:

1. For an AI audit, save the exact prompt, with the list at its end, as
   `prompt.md`.
2. Check every file for personal information. A hashed file cannot be
   corrected afterwards without a new hash.
3. If there is a `private.md`, run `sha256sum -c private.md.sha256`. It
   prints `private.md: OK`. If the AI could not make the file, or
   `private.md` changed afterwards, make it:
   `sha256sum private.md > private.md.sha256`.
4. Open an issue on the Faraday repository with the audit form. Attach
   `prompt.md` and `audit.md` as files, not pasted text, and give the
   hash from `private.md.sha256` if there is one.
5. If there is a `private.md`, send it as a private vulnerability report
   on the same repository, attached, with the issue's number.
6. Optional: timestamp the hash with OpenTimestamps, which anchors it in
   a Bitcoin block, so its date does not depend on GitHub keeping the
   issue.

| Piece | Made by | Where it goes | Who can see it |
|---|---|---|---|
| `prompt.md` | The person | The public issue, attached | Everyone |
| `audit.md` | The AI or the expert | The public issue, attached | Everyone |
| `private.md` | The AI or the expert | The private report, attached | Faraday and the submitter, until published |
| Hash of `private.md` | The AI (step 7), checked by the person | The public issue, a form field | Everyone, from the day it is sent |

The hash is in the issue only, never in `audit.md`. The issue is what
dates it, and one copy cannot disagree with another. When the prompt
itself points at a vulnerability (a follow-up prompt naming it, say),
the prompt goes in the private report too, and the issue carries its
hash (`sha256sum prompt.md`) in its place. When `private.md` has to
change after it was sent, the submitter sends the new file and posts its
new hash on the issue; both hashes stay there.

The person checks the hash because an AI that cannot run commands may
still write one that looks right. A wrong hash posted publicly would
make an honest `private.md` look altered later.

## 4. The private route

GitHub's private vulnerability reporting on `bitcoinpasada/faraday`. A
report is seen only by the repository's owner and the reporter. Issues,
discussions and pull requests on a public repository are all public, so
this is the only private route through GitHub, and the process needs it.
It is off (checked 2026-10-13). The owner turns it on:
Settings → Code security → Private vulnerability reporting, or
`gh api -X PUT repos/bitcoinpasada/faraday/private-vulnerability-reporting`.

## 5. Faraday's response

- **Within 15 days of the issue:** a reply on it that quotes back the
  hash, if there is one, and gives a verdict on each finding: confirmed
  and fixed (with the fixing commit), confirmed and open, could not
  reproduce, or not a bug.
- **Then:** the audit's entry is published.
- **Private findings:** fixed, then `private.md` is published and the
  entry updated. `private.md` is published no later than N days after
  the private report arrives, fixed or not (§9).

## 6. The record

One entry per audit on the Audits page, newest first:

- the model and its version, or the expert's credit name; the date; the
  commit audited, and how many releases behind the current one it is
- the submitter's summary, unedited
- the findings counted by severity, with the private count shown as
  private until published
- Faraday's response: the verdict on each finding, and a line or two
  overall
- links to `prompt.md`, `audit.md` and, once published, `private.md`,
  with the hash and the issue

The site keeps its own copies of the submitted files, so its publish
check scans them for personal information like any other file. The page
also says what an entry means: it records what was submitted and
Faraday's verdict on each finding. It is not an endorsement, and it does
not prove which model wrote the output.

## 7. What the hash proves

It proves that `private.md` existed on the day its hash was posted, and
that the file published later is that file. Faraday cannot play a
finding down or fix it quietly and say it was never reported, and a
submitter cannot later claim to have reported something the file does
not hold. It does not prove which model wrote the output, that the
findings are right, or that the submitter did not choose one run out of
several.

## 8. To build

- [ ] The owner turns on private vulnerability reporting (§4).
- [ ] A test report with a file attached, to see whether private reports
      take attachments. If they take pasted text only, the pasted copy
      may not match the hash byte for byte; then `private.md` goes in a
      code block and its hash is checked when it arrives.
- [ ] `SECURITY.md` at the repository's root: the private route (§4),
      what goes there (critical and high findings), the timeline (§5),
      and a link to this file and the Audits page.
- [ ] `.github/ISSUE_TEMPLATE/audit.yml`, an issue form (a template, not
      a GitHub Action) with these fields: the model and its version, or
      "expert"; the commit audited; a name or pseudonym for credit
      (optional); whether a private report was sent, and the hash of
      `private.md`; a text area for the attached `prompt.md` and
      `audit.md`; a box ticked for "these files were checked for
      personal information".
- [ ] The Audits page and the bar's **Audits** link (the website
      repository's HANDOFF).

## 9. Open

1. N, the latest day `private.md` is published (§5); 90 days proposed.
2. Whether the page says that ordinary bug reports go to the
   repository's issues, so nobody takes "experts" to mean only experts
   may report.
