# Faraday's documents

**Kind:** Reference
**Status:** Built

Where to look for what. Each document's header says what kind it is:
- **Reference** describes what is built and true now.
- **Spec** is planned work, or work partly built.
- **Record** is a log kept as written.
- **History**, in `docs/history/`, describes the tree as it was.

## By question

| Question | Look in |
|---|---|
| What runs on the device, as whom, and what can each part reach? | `docs/SECURITY-MODEL.md` |
| Which code computes cryptography, and what checks it? | `docs/AUDIT.md`, then each crypto crate's README (`core/faraday-bip/README.md` and the others) |
| Why is something the way it is? | `docs/DECISIONS.md`, indexed at the top |
| What does a screen look like, and what is it made of? | `docs/DESIGN.md`; the Starter's screens, `docs/STARTER.md` §3 |
| What happens, step by step, in a flow? | `docs/FLOWS.md`; backups, `docs/BACKUP-FLOW.md`; spending, `docs/FAMILY.md` and `docs/SCAN-TO-SIGN.md` |
| How is a vault stored? | `docs/VAULT.md` |
| What is an `.oskb` backup, byte by byte? | `docs/BACKUP-FORMAT.md` |
| How does data move by QR code, and how is a code scanned? | `docs/QR.md` |
| Which other wallets' files and codes does Faraday read and write? | `docs/INTEROP.md`, `docs/WALLETS.md` |
| How does the Starter write a stick and start a computer from it? | `docs/STARTER.md` |
| What about Macs? | `docs/MAC.md` |
| What are TOTP codes and the clock? | `docs/TOTP.md` |
| How are APKs signed? | `docs/APK-SIGNING.md` |
| What are the demo and the developer builds? | `docs/DEMO.md` |
| Why is a flow fast or slow, and how is costly work kept out of drawing? | `docs/SPEED.md`, `docs/COMPUTE.md` |
| How do scrolling and motion work? | `docs/MOTION.md` |
| Why is a dependency there? | `docs/deps/<crate>.md`; the policy is DECISIONS F71 |
| Which tool or lint does what? | `tools/README.md` |
| Where are keys held, and how do they leave? | `docs/KEYS.md`, then `docs/AUDIT.md` §2 |
| What is planned and not built? | `docs/NOSTR.md`, `docs/LARGE-FILES.md`, `docs/MULTIVENDOR.md`, `docs/STRUCTURE-PASS.md`; the board holds the rest |
| How do I submit an audit? | `docs/AUDIT-SUBMISSIONS.md` |
| How do I report a security problem? | `SECURITY.md` |
| What are the rules for changing this repository? | `CLAUDE.md` |
| What did an earlier plan or batch say? | `docs/history/` |

## Every document

**Reference**

- `docs/AUDIT.md`: What is cryptography, and where it lives. Built; rewritten 2026-10-21 around the key boundary (F70).
- `docs/BACKUP-FORMAT.md`: `.oskb`: the encrypted backup Faraday reads. Built.
- `docs/COMPUTE.md`: Compute: costly work takes a token. Built.
- `docs/DESIGN.md`: Design system. Built; §4 and §5 still describe some screens Faraday does not have.
- `docs/FAMILY.md`: The Spend tab. Built.
- `docs/MOTION.md`: Scrolling, motion and appearance. Built; waiting for the owner's check on hardware.
- `docs/SCAN-TO-SIGN.md`: Scan to sign, and the Spend tab's intake. Built.
- `docs/SECURITY-MODEL.md`: Security model. Built.
- `docs/TOTP.md`: TOTP codes and the time. Built.
- `docs/VAULT.md`: The Faraday vault. Built.
- `docs/WALLETS.md`: The Wallets tab. Built.

**Spec**

- `docs/APK-SIGNING.md`: The APK signing key. Built; batch 2 as far as the key allows.
- `docs/AUDIT-SUBMISSIONS.md`: Audits: the request, the submission and the record. Built; §8 lists what is not.
- `docs/BACKUP-FLOW.md`: The backup flow. Built; the Nostr key's part is not.
- `docs/DEMO.md`: The demo, and Faraday Desktop for developers. Built; DM4 is in the website's repository.
- `docs/FLOWS.md`: Flows. Draft; the code is the record where it differs.
- `docs/INTEROP.md`: Interoperability with other wallets. Tested; the batches that build §4 are in §5.
- `docs/KEYS.md`: The key boundary: faraday-keys. Built 2026-10-21 (steps 0 to 4).
- `docs/LARGE-FILES.md`: Large files on a stick visit. Not built.
- `docs/MAC.md`: Faraday on Macs. Partly built; a Mac is needed for the rest.
- `docs/MULTIVENDOR.md`: Two implementations of every cryptographic step. Proposal; not scheduled.
- `docs/NOSTR.md`: The Nostr key. Not scheduled.
- `docs/QR.md`: QR transfer and scanning. Partly built; §6 says what; §7 not scheduled.
- `docs/SPEED.md`: Speed in long flows. In progress (batches C to E).
- `docs/STARTER.md`: The Starter. Built; the Mac parts (`docs/MAC.md`) are not.
- `docs/STRUCTURE-PASS.md`: The structure pass. In progress: Waves A and B and phase 5 built; F74's layout lint and phases 6 and 7 to come.

**Record**

- `docs/DECISIONS.md`: Decisions. Kept as written; indexed at the top.

**History** (`docs/history/`): `BACKUP-FLOW-LOG.md`, `BITLOCKER-ACCEPTANCE.md`, `HANDOFF.md`, `MOTION-LOG.md`, `NEW-WALLET.md`, `PLAN.md`, `SIMPLIFY.md`, `STARTER-LOG.md`.

**Elsewhere:** `docs/learn/` holds the Learn pages, `docs/deps/` a page per dependency, `docs/screenshots/` the README's pictures.
