# Audits: the request, the submission and the record

**Kind:** Reference
**Status:** Built (F19); the Audits page does not yet show §10 (F160)

The process lives here; `SECURITY.md` and the issue form follow it, and
the website's Audits page shows it.

## 1. What it is for

Faraday invites audits from experts and AIs and publishes each one at
`https://faradayoffline.app/audits/`. An entry has the submitter's
summary, Faraday's short response, and links to the full prompt and
output, to Faraday's reply on the issue, which gives the verdict on each
finding, and to anything else written about it. Critical and high findings are sent privately and published
once fixed. A hash posted on the day they are sent shows that the file
published later is the one that was sent.

Faraday's own specifications, which no outside implementation checks,
are listed for review in `docs/STANDARDS.md` §7, ranked by how much a
mistake would cost, and called for in `docs/AUDIT-SUBMISSIONS.md` §10.

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
> 2. State the scope: the areas covered, the parts of the code read,
>    and the parts not read. `docs/AUDIT.md` lists where the
>    cryptography is.
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
> - **Critical:** a key, seed or passphrase can be extracted; the device
>   signs, or shows as approved, something the user did not approve;
>   Faraday's output is wrong in a way that loses funds (an address it
>   cannot spend from, a backup or shares that do not restore); or
>   crafted input runs code on Faraday.
> - **High:** the same, but only under specific conditions (physical
>   access, a non-default setting, a rare input); a vault slot opens
>   without its passphrase or reveals how many slots are used; the air
>   gap fails; secrets outlive the session; a tampered image or upgrade
>   is accepted; Faraday Starter writes to the wrong disk; the design
>   makes a mistake that loses funds or exposes a key likely, through a
>   missing warning or confirmation or a misleading label.
> - **Medium:** a protection is weaker than intended but holds (a lower
>   key-stretching cost); a crash from crafted input with no code run;
>   public wallet data leaked.
> - **Low:** a defect with no direct security effect, including a wrong
>   result that Faraday detects and refuses.
> - **Note:** an observation, not a defect.

The severities grade by what happens to the person using Faraday, not
by how a defect works: a wrong result that loses funds is critical, as
theft is.

The areas to cover are a second text, copied on its own, so that people
writing their own prompt can take them, change them or leave them out.
The page offers them as a starting point for the audit's scope:

> **Areas to cover.** A starting point for the audit's scope. Add them
> to the prompt as they are, change them, or write your own.
>
> Areas to cover:
>
> - **Key material:** whether a key, seed or passphrase can leave the
>   device; the randomness and entropy behind key generation; nonces in
>   ECDSA, Schnorr and MuSig2 signing.
> - **Signing:** whether what the screen shows (amounts, fee,
>   destinations, change) is what gets signed.
> - **Correct results:** derivations, addresses, mnemonics, SLIP-39 and
>   codex32 shares, backup files (KDBX), MuSig2,
>   against published vectors and other implementations.
> - **The vault:** each slot's passphrase opens only that slot, the
>   key-stretching cost is the one stated, the file does not reveal how
>   many slots are used, and a damaged file is detected.
> - **The air gap and what remains:** no network interface comes up,
>   secrets are written only where the user chooses, and none outlive
>   the session.
> - **Untrusted input:** PSBTs, QR codes, images and file systems from
>   the online computer.
> - **The image and its tools:** release verification, the Secure Boot
>   keys, the upgrade, and Faraday Starter's disk writing and checks.
> - **Access:** the keyboard code and the passphrase prompts.
> - **Mistakes a person can make:** where Faraday or
>   Faraday Starter make an easy mistake possible that loses funds,
>   exposes a key or locks a person out of their computer: a misleading
>   label, steps in a confusing order, a risky action with no
>   confirmation, a risky setup with no warning (a TV, a capture card or
>   KVM, a wireless keyboard), a backup that can be skipped or left
>   unchecked, and whether the warnings that exist come at the moment
>   they are needed.

The areas are named generally, never after a known open finding.

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
   on the same repository, with the issue's number. The report's form
   takes no files: paste the whole of `private.md` into its Details
   field, between a line `~~~~~` above it and the same line below it.
6. Optional: timestamp the hash with OpenTimestamps, which anchors it in
   a Bitcoin block, so its date does not depend on GitHub keeping the
   issue.

| Piece | Made by | Where it goes | Who can see it |
|---|---|---|---|
| `prompt.md` | The person | The public issue, attached | Everyone |
| `audit.md` | The AI or the expert | The public issue, attached | Everyone |
| `private.md` | The AI or the expert | The private report, pasted in a code block | Faraday and the submitter, until published |
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

GitHub stores pasted text with CRLF line ends and without its last
newline. Faraday takes the text between the two `~~~~~` lines as stored
(the report's edit view, or the API), makes every line end LF, ends it
with one newline, and checks the hash on that. If it does not match,
Faraday checks it again with CRLF line ends, as a file made on Windows
has them, before saying the hash does not match. Five tildes, not
backticks, so that code blocks inside `private.md` do not end the block.

## 4. The private route

GitHub's private vulnerability reporting on `bitcoinpasada/faraday`. A
report is seen only by the repository's owner and the reporter. Issues,
discussions and pull requests on a public repository are all public, so
this is the only private route through GitHub, and the process needs it.
It is on since 2026-10-13 (owner). It is turned off under Settings →
Code security → Private vulnerability reporting, or with
`gh api -X DELETE repos/bitcoinpasada/faraday/private-vulnerability-reporting`.

## 5. Faraday's response

- **Within 15 days of the issue:** a reply on it that quotes back the
  hash, if there is one, and gives a verdict on each finding: confirmed
  and fixed (with the fixing commit), confirmed and open, could not
  reproduce, or not a bug.
- **A private report with no issue:** a reply on it within 15 days of
  its arrival (owner, 2026-10-13).
- **Then:** the audit's entry is published.
- **Private findings:** fixed, then `private.md` is published and the
  entry updated. `private.md` is published no later than 90 days after
  the private report arrives, fixed or not (owner, 2026-10-13).

## 6. The record

One entry per audit on the Audits page, newest first:

- the model and its version, or the expert's credit name; the date; the
  commit audited, and how many releases behind the current one it is
- the submitter's summary, unedited; when none came with the audit,
  one written by Faraday and marked as Faraday's
- the findings counted by severity, with the private count shown as
  private until published
- Faraday's short response: a line or two overall
- links to `prompt.md`, `audit.md` and, once published, `private.md`,
  with the hash and the issue; to Faraday's reply on the issue, with the
  verdict on each finding; and to anything else Faraday wrote about it

The site keeps its own copies of the submitted files, so its publish
check scans them for personal information like any other file. The page
also says what an entry means: it records what was submitted and
Faraday's short response; the verdict on each finding is in Faraday's
reply on the issue. It is not an endorsement, and it does
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

Everything here is built, but for the Audits page's part of §10, which
the website's repository adds. Where each part lives:

- the private route (§4): GitHub's private vulnerability reporting, on;
  a private report takes no file, so `private.md` is pasted (§3);
- `SECURITY.md` at the repository's root: the private route, what goes
  there, the timeline (§5), and links to this file and the Audits page;
- `.github/ISSUE_TEMPLATE/audit.yml`: the audit issue form (§3);
- `.github/ISSUE_TEMPLATE/bug.yml`: the bug report form (§9);
- the Audits page and the bar's **Audits** link: the website's
  repository.

The checklist it was built from: moved to
`docs/history/AUDIT-SUBMISSIONS-LOG.md`.

## 9. Bug reports (owner, 2026-10-19)

Ordinary bug reports go to the repository's issues, through the
**Bug report** form (`.github/ISSUE_TEMPLATE/bug.yml`), so nobody takes
"experts" to mean only experts may report. Every report the form opens
carries the label **user report**. The Audits page says so in one line
beside the Audit form's link. A security problem still goes in a private
report (§4).

## 10. Outside review of Faraday's own specifications (F160, F177)

Some of Faraday's cryptography follows no published standard: the
specification is Faraday's own, and no implementation by anyone else
exists to check it against. The cross-check (`docs/CROSSCHECK.md` §10)
compares Faraday's Rust there with a second implementation written by
Faraday's own authors, in Python, from the specification. That catches
a slip between the two implementations. It does not catch a misreading
both share, or a flaw in the specification itself: those parts are not
checked against an outside source. Each such action carries the label
"own specification, not yet reviewed outside Faraday" (its `label` in
`docs/crypto/actions.toml`), shown in the app's and the website's
Cryptography blocks and in `docs/STANDARDS.md` §7, until an outside
party reviews it (F160 item 3, F177).

This section is the call for that review: what a reviewer gets, what is
asked, and how a result is recorded. No reviewer is chosen. Anyone who
is not one of Faraday's authors, or an agent working for them, may
answer it.

### 10.1 The specifications, in order

The vault first: it holds a person's keys. Each row names the
specification to write from, the answers to reproduce, and Faraday's
code for it, which the author of an outside implementation does not
read until that implementation is written. The vectors of own
randomness, the cards, the camera, a mix and the rollers are a JSON file
each, beside a README that says what each vector covers and how it was
made, and gives the procedure in words and a few lines of Python,
standard library only, that reproduce its answers; Faraday's tests read
the same files.

| Specification | Actions (`id`) | Spec | Vectors and known answers | Faraday's code (not read first) | Cross-check area |
|---|---|---|---|---|---|
| The vault: its file, slots, key schedule, sealing and records | `vault-create`, `vault-unlock`, `vault-seal`, `vault-edit` | `docs/VAULT.md` §1 to §9 and §11; F97, F158, F159; `docs/SECURITY-MODEL.md`, "Vault cryptography"; RFC 9106, RFC 5869, draft-irtf-cfrg-xchacha-03, RFC 8439, UAX #15 | `tools/vectors/vault/` (its README lists each file and passphrase); `tools/vectors/vault-primitives/`; `tools/vectors/kdf/` | `faraday/faraday-vault/`, `faraday/faraday-keys/src/vault.rs`, `tools/vault/`, `tools/crosscheck/area_vault.py` | `vault` |
| Own randomness: the accumulator, the seed, the pool, a seed draw, the lock, and the pointer-and-keys commitment | `fresh`, `pointer-keys` | `docs/OWN-RANDOMNESS.md` §2.1, §2.3; F86, F154 | `tools/vectors/own-randomness/`, which `faraday/faraday-core/tests/all/own_randomness.rs` reads | `faraday/faraday-core/src/fresh.rs`, `core/faraday-entropy/src/pointer_keys.rs`, `tools/crosscheck/area_own.py` | `own` |
| Entropy from playing cards, camera frames and a mix of sources | `cards`, `camera`, `mix` | `tools/reference/entropy/README.md` (the procedures as stated); `docs/STANDARDS.md` §3; F158 | `tools/vectors/cards/`, `tools/vectors/camera/` and `tools/vectors/mix/`, which the tests in `core/faraday-entropy/src/cards.rs`, `camera.rs` and `mix.rs` read; `tools/vectors/entropy/` for a mix's dice | `core/faraday-entropy/src/`, `tools/crosscheck/area_entropy.py` | `entropy` |
| Faraday Demo's dice rollers | `roller` | `docs/CODEX32.md` §5.5, §5.6; `tools/reference/booklet-dice/README.md`; F96 | `tools/vectors/roller/` for the five dice, which `faraday/faraday-core/src/fresh/roller/five_tests.rs` reads; the one die has none beyond the cross-check's cases | `faraday/faraday-core/src/fresh/roller/`, `tools/crosscheck/area_roller.py` | `roller` |
| Secure Boot keys: the generator and the enrolment files | `sb-keys` | F134, F158; `docs/learn/faraday/secure-boot-keys.md`; the UEFI Specification, RFC 5280, RFC 5652, RFC 8017 | `faraday/faraday-sb/tests/golden.rs` (the keys made from a fixed seed), `judged.rs` (OpenSSL's checks), `faraday/faraday-sb/tests/vectors/` | `faraday/faraday-sb/` | none: OpenSSL judges the keys and signatures |
| A key held in memory: blinding, pinned pages and the in-memory seal, which no shipped code reaches | `hold` | `core/faraday-crypto/README.md`; `docs/SECURITY-MODEL.md`; `docs/KEYS.md` | the tests in `core/faraday-crypto/src/sealed.rs` | `core/faraday-crypto/`, `faraday/faraday-keys/src/session.rs` | none |

### 10.2 What is asked

One or more of these, for one specification or several:

1. **An outside implementation.** A second implementation of the
   specification, written from it alone, without reading the code the
   table lists. It reproduces the vectors and known answers, and it
   offers the operations of §10.3, so that it can take the place of
   Faraday's second implementation in the cross-check. Each place where
   the specification leaves the answer open, and each disagreement
   between the outside implementation and the vectors, is a finding.
2. **A reproduction of the vectors.** The reviewer's own code, in any
   language, opening or computing each vector, with the answer it gives
   for each.
3. **A review of the specification.** Whether it does what it says it
   does: for the vault, the promises of `docs/VAULT.md` §9; for own
   randomness, that what a person adds cannot lower the system's
   randomness; for cards, the camera and a mix, that the bits counted
   are there; for the Secure Boot generator, that the keys are drawn as
   F134 and F158 state. Findings take §2's severities.

For the Secure Boot keys and a key held in memory, the review (3) is
what is asked: an outside implementation would have to reproduce the
`rsa` crate's own search for primes to match the known answers, and the
in-memory seal is reached by no shipped code.

### 10.3 What an outside implementation offers, to be used in the cross-check

An outside implementation the cross-check can call is:

- Python 3, over the standard library and the packages the cross-check
  already takes from the distribution (`argon2-cffi`, PyNaCl, PyCA
  `cryptography`; `docs/DEVELOPING.md`), fetching nothing at run time,
  with no file over 2 MiB;
- under a licence that allows copying it unchanged into this
  repository (MIT, BSD, Apache-2.0, CC0 or public domain).

An implementation in another language counts as a reproduction (§10.2,
item 2). Running it in the cross-check needs its toolchain allowed by a
decision, as Go was for LND's aezeed (F160 item 6).

The operations each area compares:

- **The vault:** open a file with a passphrase, giving the slot that
  opens, its seal count and its records, or the refusal of §4 that
  applies; make a vault from its passphrases, cost, slot size and records
  with every random byte given (the salt, the order of the slots, the
  nonces and the bytes of the unused slots; §8 leaves how they are drawn
  to the implementation), so the file is compared byte for byte; seal an
  open slot under a given nonce and time; write and read a slot's
  records (§7), with each refusal.
- **Own randomness:** §2.3's operations over given system answers: a
  batch added, the seed, a fresh answer for a purpose, a seed draw, This
  device's answer, a lock; and §2.1's commitments.
- **Cards, camera and a mix:** each source's commitment and its entropy
  at each strength, with the refusals the procedures state.
- **The rollers:** the values made from given answers; the one die's
  clicks and the five dice's worksheet, as §5.5 and §5.6 state them.

### 10.4 Faraday's side, to compare with

Once the outside implementation is written:

- `python3 tools/vault/open.py tools/vectors/vault/vault-three.fvlt`
  prints the records of the slot a passphrase opens (`docs/VAULT.md`
  §10), over `argon2-cffi` and PyNaCl, or `cryptography`.
- `just crosscheck --area vault` runs one area: Faraday's Rust against
  the reference the area names, over a corpus drawn from a seed, and
  prints the seed. `just crosscheck --area vault --seed S --case
  corpus:N` runs case N of it again (`tools/crosscheck/README.md`). The
  prerequisites are in `docs/DEVELOPING.md`.
- `cargo test -p faraday-core --test all own_randomness` runs the known
  answers of own randomness, and `cargo test -p faraday-core --lib
  five_tests` those of the five dice, over the files in
  `tools/vectors/own-randomness/` and `tools/vectors/roller/`;
  `cargo test -p faraday-entropy`, those of cards, the camera and a mix,
  over `tools/vectors/cards/`, `tools/vectors/camera/` and
  `tools/vectors/mix/`; `cargo test -p
  faraday-sb`, the Secure Boot keys. Each of those five folders' README
  checks its file with Python alone.

### 10.5 Submitting

As §3, with the audit form. The form's model field is "expert", or the
model and its version where an AI wrote the implementation or the
review under the reviewer's direction. `audit.md` says:

- which specification, and which of §10.2's three was done;
- for an outside implementation, that it was written from the
  specification without reading the code §10.1 lists, where it is (a
  public repository at a commit, or an archive attached to the issue),
  its licence, and how to run it;
- the answer for each vector and known answer, and where it differs
  from the file;
- the findings, by §2's severities. Critical and high findings go in
  `private.md` (§3, §4).

### 10.6 The record

- **The Audits page** has an entry as §6 gives it, which also names the
  specification, the kind of review, and for an outside implementation
  its repository, commit and each file's SHA-256.
- **Faraday's reply** on the issue gives a verdict on each finding (§5).
  A disagreement between the outside implementation and Faraday is
  settled by a fix to Faraday, or by a change to the specification where
  it left the answer open or was misread.
- **The decision.** Whether a submission counts as the outside review
  of a specification is the owner's call (F160 item 3). A decision
  records it: the reviewer as credited, the specification, the commit
  reviewed and the kind of review. With it the action's `label` goes.
  `docs/crypto/check.py` allows an action's `novel_rank`, its place in
  `docs/STANDARDS.md` §7, only with the label, so the decision also says
  whether the specification stays in §7's list, and the check follows
  it.
- **An outside implementation** is copied unchanged into
  `tools/reference/<name>/`, with a README giving its source, commit,
  licence and each file's SHA-256, and listed in
  `tools/crosscheck/references.sha256`. The area compares Faraday with
  it in place of Faraday's own second implementation, which is retired
  (F160 item 8); a case the outside implementation does not cover stays
  on Faraday's, and the area's facts say so. Where the comparison is
  wholly against the outside implementation, the action's `independence`
  becomes `{ class = "outside", author = "<the reviewer's credit name>" }`,
  and the Cryptography blocks and the cross-check's README, which are
  generated from it, name the reviewer as its author.
- **A reproduction or a review alone** takes the label away with the
  decision. The area still compares with Faraday's own second
  implementation, and its `independence` still says so.
