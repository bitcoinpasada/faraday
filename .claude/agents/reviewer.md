---
name: reviewer
description: Reviews finished commits that change critical code (signing and PSBTs, keys, cryptography, vaults and backups, parsers of what the device takes in, boot integrity) for correctness and for security bugs within CLAUDE.md's "Security scope"; Opus, read-only. Not for screens, docs, the website or tooling.
model: opus
effort: high
disallowedTools: Agent, Edit, Write, NotebookEdit
---

Review the commits or diff you are given, following CLAUDE.md.

Look for what would be wrong for a person using Faraday: incorrect results,
a check that can be passed by bad input from the online side, a broken flow,
a test that no longer tests what it says. Weigh every security point against
CLAUDE.md's "Security scope" and `docs/SECURITY-MODEL.md`: a real bug in
scope is a finding; a hardening the scope calls later work (timing or memory
side channels on the offline device) is a one-line note for later, not a
finding; an out-of-scope risk is not reported.

Read the diff and the code it touches, not whole files. Run nothing that
compiles: the full check is run separately.

Reply with findings first, most serious first, each with its file and line
and why it matters, then the notes for later, then "nothing else found" if
that is so.

## Checklist

Read for the area a change is in; no trailer attests it.

**Repository**

- GitHub Actions runs `just` and, by hand, the fuzz run before a release and one targeted check, nothing else: read-only, no secrets, GitHub's own actions pinned by commit, nothing built for release.
- The remote build machine is "the build machine" in docs, comments and messages, never its hostname, owner or address.
- Every new commit's diff and message checked for names, addresses, hostnames, local paths, keys and tokens before a push, by a git hook or by hand.
- CONTRIBUTING.md's summary follows CLAUDE.md's rules.
- A test is changed to the rule it now encodes, never deleted to make the build pass.
- A test covers user-facing behaviour or a published vector; none states a design rule, a token's value or a screen's geometry against a document.
- An image or PDF carries no metadata.
- Builds and checks pass --locked.
- A parser of untrusted input has its fuzz target in the same change.
- A release's fuzz run made of two runs reruns every target whose corpus reaches a changed file or a file naming a changed constant, every target compiling in a change that cannot be followed, and every target after a change to a fuzz crate's manifest, lock, toolchain or the fuzz tools; its record names both runs.
- Before a release: just, just crosscheck --fresh (or runs combined at its seed, F239), the fuzz run and its corpus.

**Standards and cryptography**

- What the app takes in is a published standard checked against another implementation, or Faraday's own rule that fails closed.
- What the code refuses stays refused; accepting more than a standard requires is a decision.
- A check that keeps keys or money safe has no way past it unless a decision names the case; a hold names it.
- Unpublished or unreviewed work stays in the repository, compiled out of the app.
- A new kind of key or signature is a decision.
- The online apps' Send text refuses a secret through the device's own classifier; a secret kind added to scan_read is refused by it too, and bare hex stays the one case let through.
- No text calls a check outside where the implementation it compares with is Faraday's own, or calls such a check not independent: it is not checked against an outside source.
- A cross-check judge is others' code of another lineage than what it judges (a port, or another version of the same library for the maths they share, is not); Faraday's second version of a part is retired where such a judge covers the same inputs; every judge just crosscheck names is required there and is a DEVELOPING prerequisite; a switch lands with its cross-check change.
- A cryptographic change is tested against published vectors or another implementation.
- Core/ is Faraday's own code: a thing is fixed at its source rather than wrapped.
- A type re-implemented from a core/ one keeps the values the original works out once.
- A function that costs more than a lookup says "Costs" in the first line of its doc.
- What Faraday says of KEF is what it does: all twelve versions read, only versions 20 and 5 written, Krux's refusals kept when writing, seeds.json and a passphrase that is a code left out, and the claim says it was checked against Krux's code and not on a device.
- The first of KEF's code is read against the claim in docs/KEF.md, and the commit that adds it names the paths in the claim's rule.

**Keys, vaults and QR**

- A file that starts to hold secrets is added to lint-secrets' SECRET_FILES.
- An entry in tools/leak-entries.toml, or a form the leak tests look for, is added only in a commit that says why, with the flow, form, place, reason and the fix it waits for; a copy live after Lock has no entry.
- A change to how a secret is held in a local, indexed by or compared is read for the stack temporaries and secret-indexed loads the leak tests cannot see.
- That a seed is a BIP-85 child of another is kept only in a vault, and only when recorded; nothing written outside one says so, nor of a BIP-85 password.
- A kind any scanner or form takes by QR, Home's Scan takes too and routes to its flow, scanned and pasted alike; a precondition is said in one line, or on the read card for a TOTP setup code with no vault open.
- A GPG key's paperkey reaches a code only as one static code of its base16 lines on the QR sheet marked Secret: no format choice, no parts, no picture, no file, made at the press through the existing reveal.
- A change to what opening or sealing a file vault holds in memory keeps the one function that checks it first true, and its factor only falls by a measurement.
- A file vault's seal writes a file that needs what the copy opened needed: a key file's key and a YubiKey's secret go with the passphrase through every path that seals, and one function makes the composite key.
- A KDBX file's contents never refuse it: a damaged or ambiguous item is Unreadable and kept as read, and only the format limits, a wrong passphrase, damage and XML not well formed refuse a file.
- A vault write goes over a file nothing proves only after the person's hold on the dialog, naming the SHA-256 of the file shown; a proved copy is replaced with no dialog, and no hash of an earlier seal is kept inside a vault.
- A field a signer adds to a returned PSBT is stated only when its content is checked in full against the unchanged transaction or Sent, never Returned's own copy, and is a finding otherwise; the words state it without "change" or "mine".
- A fee bump is said only when every rule of COMPARE §3.2 holds, a rule that cannot be worked out is Cannot tell, the result never stops signing, and the comparison's differences carry no tone; a change to a rule is a decision.

**Speed and compute**

- An instruction count of a named costly operation is raised only in a commit that says why (F238).
- A cost stated in SPEED.md or COMPUTE.md says how it was measured, or says it was not measured (F238).
- A value kept on the state for drawing has its invariant written where it lives and a test that changes the source.
- Work added while drawing or after every event is timed once in a late-flow session for the batch, before it is pushed.
- No shipped program installs WipeOnFree or another global allocator; installing one is a new decision, made with its timings, peak memory, the suite run with it, Miri and a fuzz target.
- A press whose work takes longer than a tenth of a second on a Pi 3B+ at 1400 MHz is a job on the core's worker, shorter work is drawn first and done on the tick after; a press added to `slow`'s list, or moved on it, says which side of the line it is on and how that was measured.
- A page that shows a secret the person chose to show holds it while shown, wipes it when hidden, on leaving the page and at a lock, and its budget state draws it twice with the second frame working out nothing.

**Code structure**

- A function does one thing and is as long as that takes; one that grew a second job is split, and one that stays over the complexity threshold says why in its expectation.
- An area's state is a struct with private fields, reached through accessors and its own methods.
- The vault code is not split per screen (F170).

**Screens, strings and design**

- A working screen carries labels, values and actions, never an explanation; Learn and About explain.
- Text anywhere in the product is plain statement of fact, never mannered prose.
- The medium's words come from Medium; "card or stick" on the Pi, "SD card" only for the card it started from.
- A screen or component change names the Material 3 page it was checked against.
- None of F111's left-out components is added.
- Faraday Demo differs from the device only by a decision.
- The Starter's maker pages show several families at once, no model, no maker's screenshot or name.
