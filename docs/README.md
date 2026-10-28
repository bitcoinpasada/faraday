# Faraday's documents

**Kind:** Reference
**Status:** Built

Where to look for what. Open this first, then only the section you
need. Each document's header says what kind it is:
- **Reference** describes what is built and true now.
- **Spec** is planned work, or work partly built; its `**Open:**` line
  says what is not built.
- **Record** is kept as written: the decisions.
- **History**, in `docs/history/`, describes how things were built and
  the tree as it was. A section moved there leaves its heading and one
  line naming where it went, so a cited section number still resolves.

## By question

| Question | Look in |
|---|---|
| What runs on the device, as whom, and what can each part reach? | `docs/SECURITY-MODEL.md` |
| Which standard does each wallet kind, key method, format and tool follow, how fully, and where did Faraday decide for itself? | `docs/STANDARDS.md` |
| Which code computes cryptography, and what checks it? | `docs/AUDIT.md`, then each crypto crate's README (`core/faraday-bip/README.md` and the others) |
| Why is something the way it is? | `docs/DECISIONS.md`, the index; then the one decision, `docs/decisions/F<n>.md` |
| How does a person use Faraday, step by step, with the screens? | `docs/GUIDE.md`, the index; then the one walkthrough, `docs/guide/walk/<name>.md`; what each control on a screen does, `docs/guide/screens/README.md` |
| What does a screen look like, and what is it made of? | `docs/DESIGN.md`; the Starter's screens, `docs/STARTER.md` §3 |
| What happens, step by step, in a flow? | `docs/FLOWS.md`; backups, `docs/BACKUP-FLOW.md`; spending, `docs/SPEND-FROM-BACKUP.md` |
| How are codex32 shares made, checked by hand and repaired? | `docs/CODEX32.md` |
| How will a lost GPG key be restored from its paperkey? | `docs/GPG-PAPERKEY.md` |
| How is a vault stored? | `docs/VAULT.md` |
| What is in the KeePass (KDBX 4) file Faraday writes? | `docs/KDBX.md` |
| How does data move by QR code, and how is a code scanned? | `docs/QR.md` |
| Which other wallets' files and codes does Faraday read and write? | `docs/INTEROP.md`, `docs/WALLETS.md` |
| How does the Starter write a stick and start a computer from it? | `docs/STARTER.md` |
| Did a signing device do only what it was asked? | `docs/SIGNER-CHECK.md` |
| How does text get larger for people who need it? | `docs/TEXT-SIZE.md` |
| How is a screen laid out at each window size, from the Pi to a monitor? | `docs/LAYOUT.md` |
| Which way up is a 7-inch or 10-inch panel, and how are its screen and touch turned? | `docs/ORIENTATION.md` |
| How does Faraday Demo show its screens at another screen's size? | `docs/PREVIEW-SIZE.md` |
| What stands outside the app on Faraday Demo: the sticks, the size, Practice's aids? | `docs/DESKTOP-HOST.md` |
| How are the themes' colours made, and which colour role does each component take? | `docs/THEMES.md`; `docs/DESIGN.md` §3 |
| Can a person add their own randomness, and when is the stick's kernel ready? | `docs/OWN-RANDOMNESS.md` |
| How does a seed change form and stay the same wallet? | `docs/SEED-CONVERSION.md` |
| What about Macs? | `docs/MAC.md` |
| What are TOTP codes and the clock? | `docs/TOTP.md` |
| What are Practice and the developer builds? | `docs/DEMO.md` |
| Why is a flow fast or slow, and how is costly work kept out of drawing? | `docs/SPEED.md`, `docs/COMPUTE.md` |
| How do the modes, Practice and its samples work? | `docs/MODES.md` |
| How do scrolling and motion work? | `docs/MOTION.md` |
| Why is a dependency there? | `docs/deps/<crate>.md`; the policy is DECISIONS F71 |
| Which tool or lint does what? | `tools/README.md` |
| What is run before a release, and which parsers are fuzzed? | `tools/README.md`, "Before a release"; `fuzz/README.md`, `faraday/fuzz/README.md` |
| Where are keys held, and how do they leave? | `docs/KEYS.md`, then `docs/AUDIT.md` §2 |
| What is planned and not built? | each Spec's `**Open:**` line (the Specs are listed below); the board holds the rest |
| How would a signer be kept from hiding its key in a nonce (anti-exfil)? | `docs/ANTI-EXFIL.md` |
| How do I submit an audit? | `docs/AUDIT-SUBMISSIONS.md` |
| How do I report a security problem? | `SECURITY.md` |
| What are the rules for changing this repository? | `CLAUDE.md` |
| What did an earlier plan or batch say, or how was something built? | `docs/history/`: `<NAME>-LOG.md` beside each document's name, and the retired decisions in `docs/history/decisions/` |

## Every document

**Reference**

- `docs/AUDIT-SUBMISSIONS.md`: Audits: the request, the submission and the record. Built (F19).
- `docs/AUDIT.md`: What is cryptography, and where it lives. Built (the key boundary, F70).
- `docs/CODEX32.md`: Codex32: making, checking and repairing shares. Built (F93 to F96, F107); no Codex32QR picture for 74 and 127 characters.
- `docs/COMPUTE.md`: Compute: costly work takes a token. Built (F69).
- `docs/DEMO.md`: Practice, and Faraday Demo for developers. Built (F30, F61).
- `docs/DESIGN.md`: Design system. Built.
- `docs/KDBX.md`: KDBX 4: the encrypted file Faraday writes for others. Built.
- `docs/KEYS.md`: The key boundary: faraday-keys. Built (steps 0 to 4).
- `docs/PREVIEW-SIZE.md`: Preview size: Faraday Demo at another screen's size. Built (F110, F118).
- `docs/README.md`: Faraday's documents. Built.
- `docs/SECURITY-MODEL.md`: Security model. Built.
- `docs/SPEND-FROM-BACKUP.md`: The Spend tab. Built.
- `docs/STANDARDS.md`: Standards: what Faraday follows, how fully, and where it decides. Built.
- `docs/TEXT-SIZE.md`: Text size. Built (F85): Standard, Large and Larger, in Faraday and the Starter.
- `docs/THEMES.md`: Themes on Material 3's colour system. Built (F113); the website, the guide's pictures and the README's screenshots are not yet redrawn in the new colours.
- `docs/TOTP.md`: TOTP codes and the time. Built.
- `docs/VAULT.md`: The Faraday vault. Built.
- `docs/WALLETS.md`: The Wallets tab. Built.

**Spec**

- `docs/ANTI-EXFIL.md`: Anti-exfil. Planned, not scheduled.
- `docs/BACKUP-FLOW.md`: The backup flow. Partly built.
- `docs/DESKTOP-HOST.md`: Faraday Demo as a simulator: the host. Built (F118); the website's part planned.
- `docs/FLOWS.md`: Flows. Built but for the items under Open.
- `docs/GPG-PAPERKEY.md`: Restore a GPG key from its paperkey. Not built; its decision is a draft.
- `docs/GUIDE.md`: Faraday user guide, the index. Sections 1 to 7 are stubs pointing to `docs/guide/walk/`, one file per walkthrough; all twenty-three walkthroughs hold the written text.
- `docs/guide/screens/README.md`: the guide's screen reference, one page per screen or group of small screens; every screen and sheet of Faraday has its page.
- `docs/INTEROP.md`: Interoperability with other wallets. Tested.
- `docs/LARGE-FILES.md`: Large files on a stick visit. Not built (F26, amended by F30).
- `docs/LAYOUT.md`: Layout at every window size. Specified (F92), not built.
- `docs/MAC.md`: Faraday on Macs. Partly built.
- `docs/MODES.md`: Modes, Practice and samples. Specified (F81), not built.
- `docs/MOTION.md`: Scrolling, motion and appearance. Built; not yet checked by the owner on hardware.
- `docs/CROSSCHECK.md`: Two implementations of every cryptographic step. Option A decided (F79) and built (§10.5); options B and C deferred.
- `docs/NOSTR.md`: The Nostr key. Not built, not scheduled (F27, amended by F34).
- `docs/ORIENTATION.md`: Panel orientation. Specified (F100), not built.
- `docs/OWN-RANDOMNESS.md`: Your own randomness, and the kernel's at boot. Specified (F86), not built.
- `docs/QR.md`: QR transfer and scanning. Built, except §7.
- `docs/SEED-CONVERSION.md`: Convert a seed. Specified (F88), not built.
- `docs/SIGNER-CHECK.md`: Check a signer. Planned, not built.
- `docs/SPEED.md`: Speed in long flows. Built but for the glyph cache (§3.8).
- `docs/STARTER.md`: The Starter. Partly built.
- `docs/STRUCTURE-PASS.md`: The structure pass. Built.

**Record**

- `docs/DECISIONS.md`: Decisions. Kept as written.

**History** (`docs/history/`): `APK-SIGNING.md`, `APK-SIGNING-LOG.md`, `AUDIT-LOG.md`, `AUDIT-SUBMISSIONS-LOG.md`, `BACKUP-FLOW-LOG.md`, `BITLOCKER-ACCEPTANCE.md`, `COMPUTE-LOG.md`, `CROSSCHECK-LOG.md`, `DEMO-LOG.md`, `DESIGN-LOG.md`, `FLOWS-LOG.md`, `HANDOFF.md`, `INTEROP-LOG.md`, `KEYS-LOG.md`, `LARGE-FILES-LOG.md`, `MAC-LOG.md`, `MOTION-LOG.md`, `NEW-WALLET.md`, `NOSTR-LOG.md`, `PLAN.md`, `QR-LOG.md`, `SCAN-TO-SIGN.md`, `SIMPLIFY.md`, `SPEED-LOG.md`, `SPEND-FROM-BACKUP-LOG.md`, `STARTER-LOG.md`, `STRUCTURE-PASS-LOG.md`, `TEXT-SIZE-LOG.md`, `THEMES-LOG.md`, `TOTP-LOG.md`, `VAULT-LOG.md`, `WALLETS-LOG.md`; `decisions/` holds the decisions replaced, superseded or withdrawn.

**Elsewhere:** `docs/decisions/` holds one file per decision, `docs/learn/` the Learn pages, `docs/deps/` a page per dependency, `docs/screenshots/` the README's pictures, `docs/guide/` the user guide's.
