# Faraday's documents

**Kind:** Reference
**Status:** Built

Where to look for what. Open this first, then only the section you
need. Each document's header says what kind it is:
- **Reference** describes what is built and true now.
- **Spec** is a feature's design, not built or partly built; its
  `**Open:**` line says in plain words what is not built. It carries no
  work list (F214).
- **Record** is kept as written: the decisions.
- **History**, in `docs/history/`, describes how things were built and
  the tree as it was. A section moved there leaves its heading and one
  line naming where it went, so a cited section number still resolves.

## By question

| Question | Look in |
|---|---|
| What runs on the device, as whom, and what can each part reach? | `docs/SECURITY-MODEL.md` |
| Which kernel patches does each image carry, why, and at what cost? | `docs/KERNEL-PATCHES.md` |
| Which standard does each wallet kind, key method, format and tool follow, how fully, and where did Faraday decide for itself? | `docs/STANDARDS.md` |
| Which code computes cryptography, and what checks it? | `docs/AUDIT.md`, then each crypto crate's README (`core/faraday-bip/README.md` and the others) |
| Why does Faraday write a piece of cryptography itself rather than take a library, and which libraries does it not establish? | `docs/ESTABLISHED.md` |
| Why is something the way it is? | `docs/DECISIONS.md`, the index; then the one decision, `docs/decisions/F<n>.md` |
| What does the in-app guide hold, how does it open at the step the person is on, and what replaces Learn and the website's walkthroughs? | `docs/GUIDE-IN-APP.md`; the decision, `docs/decisions/F247.md` |
| How does a person use Faraday, step by step, with the screens? | the guide's pages, `docs/guide/<place>.md` (`docs/GUIDE-IN-APP.md`); the earlier walkthroughs, screen pages and pictures, `docs/history/GUIDE.md` and `docs/history/guide/` |
| What does a screen look like, and what is it made of? | `docs/DESIGN.md`; the Starter's screens, `docs/STARTER.md` §3 |
| What happens, step by step, in a flow? | `docs/FLOWS.md`; backups, `docs/BACKUP-FLOW.md`; spending, `docs/SPEND-FROM-BACKUP.md` |
| How are codex32 shares made, checked by hand and repaired? | `docs/CODEX32.md` |
| How is a lost GPG key restored from its paperkey? | `docs/GPG-PAPERKEY.md` |
| How is a vault stored? | `docs/VAULT.md`, the KDBX vault and the OpenPGP QR vault (F190); the first format, `docs/history/VAULT-LOG.md`; the passphrase in NFC and the seal count are in it (§3, §11) |
| What is the proposed small encrypted QR format (25 × 25, 29 × 29 or 33 × 33), offered for outside review? | the AQRV specification, in its own repository: `https://github.com/bitcoinpasada/aqrv`, `AQRV.md` (F249) |
| What is in the KeePass (KDBX 4) file Faraday writes? | `docs/VAULT.md` §8: an export is a vault of one entry; the earlier writer, `docs/history/KDBX-LOG.md` |
| How does Faraday read and write Krux's encrypted codes, and what are their trade-offs? | `docs/KEF.md` |
| How does data move by QR code, and how is a code scanned? | `docs/QR.md` |
| What does each scanner do with each kind of code, and with one read in the wrong scanner? | `docs/SCANNING.md` |
| Which other wallets' files and codes does Faraday read and write? | `docs/INTEROP.md`, `docs/WALLETS.md` |
| How does the Starter write a stick and start a computer from it? | `docs/STARTER.md` |
| Did a signing device do only what it was asked? | `docs/SIGNER-CHECK.md` |
| What does Check a timestamp take and show, and what does a proof prove? | `docs/DESIGN.md` §5 (`OtsCheck`), `docs/decisions/F230.md`, `docs/history/learn/22-timestamps.md` |
| What does Read a Bitcoin Core wallet take and show, and what does it check? | `docs/DESIGN.md` §5 (`CoreWallet`), `docs/decisions/F229.md`, `docs/history/learn/23-core-wallet.md` |
| What does Decode a Lightning invoice show, and how does it open? | `docs/DESIGN.md` §5 (`Invoice`), `docs/SCANNING.md`, `docs/history/learn/21-lightning-invoices.md` |
| How do two PSBTs differ, and does a replacement only raise the fee? | `docs/COMPARE.md` |
| What does Check a signer show, and how does it open? | `docs/FLOWS.md` (Check a signer), `docs/guide/signer-check.md`, `docs/history/learn/06-verifying.md` |
| How does text get larger for people who need it? | `docs/TEXT-SIZE.md` |
| Where does the caret go in a text field, and how is it moved? | `docs/TEXT-CURSOR.md` |
| How is a screen laid out at each window size, from the Pi to a monitor? | `docs/LAYOUT.md` |
| Which way up is a 7-inch or 10-inch panel, and how are its screen and touch turned? | `docs/ORIENTATION.md` |
| How does Faraday Demo show its screens at another screen's size? | `docs/PREVIEW-SIZE.md` |
| What stands outside the app on Faraday Demo: the sticks, the size, Practice's aids? | `docs/DESKTOP-HOST.md` |
| How are the themes' colours made, and which colour role does each component take? | `docs/THEMES.md`; `docs/DESIGN.md` §3 |
| Can a person add their own randomness, and when is the stick's kernel ready? | `docs/OWN-RANDOMNESS.md` |
| How does a seed change form and stay the same wallet? | `docs/SEED-CONVERSION.md` |
| Which boards, platforms and screens does Faraday run on, with which layout, and which were considered? | `docs/HARDWARE.md` |
| Which image does each piece of hardware get, and how does a person choose and check it? | `docs/IMAGES.md` |
| What about Macs? | `docs/MAC.md` |
| What are TOTP codes and the clock? | `docs/TOTP.md` |
| How do I build Faraday, check it, reproduce a release, or try it on my own hardware? | `docs/DEVELOPING.md` |
| What are Practice and the developer builds? | `docs/DEMO.md` |
| Why is a flow fast or slow, and how is costly work kept out of drawing? | `docs/SPEED.md`, `docs/COMPUTE.md` |
| How does Faraday keep a secret from being left behind? | `docs/KEYS.md` (where secrets live and wipe), `docs/SECRET-LEAKS.md` (the tests that find a copy), `docs/CHECKS.md` (what enforces it) |
| Which thread does costly work run on, and what does a screen do while it runs? | `docs/WORKERS.md` |
| How does a frame reach the panel at its refresh rate: a scroll step, the cores, the panel's conversion and write? | `docs/FRAMES.md` |
| How do the modes, Practice and its samples work? | `docs/MODES.md` |
| How do scrolling and motion work? | `docs/MOTION.md` |
| Why is a dependency there? | `docs/deps/<crate>.md`; how they are checked, `docs/deps/README.md`; the policy is DECISIONS F71 |
| Which tool or lint does what? | `tools/README.md` |
| What must change with what, and which check holds each rule? | `docs/CHECKS.md`; why and how, `docs/IMPACT.md` |
| What is run before a release, and which parsers are fuzzed? | `tools/README.md`, "Before a release"; `fuzz/README.md`, `faraday/fuzz/README.md` |
| For each cryptographic action, which standard, code and checks, and how does a screen show it; what fails `just lint` when one is added or changed? | `docs/CRYPTO-LEARN.md` |
| Where are keys held, and how do they leave? | `docs/KEYS.md`, then `docs/AUDIT.md` §2 |
| What is planned and not built? | each Spec's `**Open:**` line (the Specs are listed below) |
| How would a signer be kept from hiding its key in a nonce (anti-exfil)? | `docs/ANTI-EXFIL.md` |
| How do I submit an audit, or an outside review of one of Faraday's own specifications? | `docs/AUDIT-SUBMISSIONS.md`; the own specifications, §10 |
| How do I report a security problem? | `SECURITY.md` |
| What are the rules for changing this repository, and how do I send a change? | `CLAUDE.md`, `CONTRIBUTING.md` |
| What did an earlier plan or batch say, or how was something built? | `docs/history/`: `<NAME>-LOG.md` beside each document's name, and the retired decisions in `docs/history/decisions/` |

## Every document

**Reference**

- `docs/AUDIT-SUBMISSIONS.md`: Audits: the request, the submission and the record, and the call for outside review of Faraday's own specifications (§10). Built (F19); the Audits page does not yet show §10 (F160, F177).
- `docs/AUDIT.md`: What is cryptography, and where it lives. Built (the key boundary, F70).
- `docs/COMPARE.md`: Compare two PSBTs, and the fee bump: the differences between any two PSBTs or finished transactions, listed without judging, and whether a replacement only raises the fee, in Compare and on Sign a transaction's review. Built (F233).
- `docs/CODEX32.md`: Codex32: making, checking and repairing shares. Built (F93 to F96, F240); no Codex32QR picture for 74 and 127 characters.
- `docs/CHECKS.md`: Checks: what holds each rule, generated from `tools/impact.toml`. Built (F149).
- `docs/COMPUTE.md`: Compute: costly work takes a token. Built (F69).
- `docs/DEMO.md`: Practice, and Faraday Demo for developers. Built (F30, F61).
- `docs/DESIGN.md`: Design system. Built.
- `docs/DEVELOPING.md`: Developing Faraday: build, check, reproduce, try on hardware. Built.
- `docs/ESTABLISHED.md`: Established implementations first: what Faraday writes itself and why, the timing fixes, what wiping reaches under the security model, the switches to outside code, the order of work, and what the cross-check compares each action with after the switches. Built (F178, F180, F181, F190, F191, F192, F193, F213); how it was built is in `docs/history/ESTABLISHED-LOG.md`; §6 is the work kept for later.
- `docs/GPG-PAPERKEY.md`: Restore a GPG key from its paperkey, and move a key to a vault on another machine by camera. Built (F169, F215).
- `docs/GUIDE-IN-APP.md`: The guide, one guide in the app, opened at the section for the place and step the person is on, in place of Learn and of the website's walkthroughs, the website's guide page saying the guide is in the app. Built (F247): the pages by place and step, the keys and their lookup, the sheet and pane, the Guide button and the Starter's guide destination.
- `docs/HARDWARE.md`: Hardware: boards, platforms and screens. Built; the ESP32-P4 board considered, not supported.
- `docs/KEF.md`: KEF, Krux's encrypted code, read and written in full. Built (F219).
- `docs/KERNEL-PATCHES.md`: The kernel patches the images carry. Built.
- `docs/KEYS.md`: The key boundary: faraday-keys. Built (steps 0 to 4).
- `docs/LAYOUT.md`: Layout at every window size. Built (F92): Material 3's size classes, navigation and top app bar, panes; the screens' sizes written in place are held by the `lint-tokens` ratchet.
- `docs/ORIENTATION.md`: Panel orientation. Built (F100): a 7-inch or 10-inch panel starts in landscape, Rotation in Settings › Display turns it and its touch.
- `docs/PREVIEW-SIZE.md`: Preview size: Faraday Demo at another screen's size. Built (F110, F118).
- `docs/OWN-RANDOMNESS.md`: Your own randomness, and the kernel's at boot. Built (F86, F157): Add randomness at every press that draws the system's randomness, the late seed and the Pi's generator, and the boot measurement on QEMU, a PC and the Pi.
- `docs/QR.md`: QR transfer and scanning. Built (§7, F28: codes of public values and the TOTP secret).
- `docs/README.md`: Faraday's documents. Built.
- `docs/SCANNING.md`: What each scanner takes, and the way over for a code read in the wrong one. Built.
- `docs/SECURITY-MODEL.md`: Security model. Built.
- `docs/SEED-CONVERSION.md`: Convert a seed. Built (F88, F204): the conversions, the tool's flow with every target and way in, the SLIP-39 set actions, an aezeed as a source, Check my copy typed or scanned back, a codex32 set never taking an identifier another set on this device carries, Practice, Learn's section and the guide's picture.
- `docs/SIGNER-CHECK.md`: Check a signer: the PSBT sent against the one returned. Built, Practice's sample pair included.
- `docs/SPEND-FROM-BACKUP.md`: The Spend tab. Built.
- `docs/STANDARDS.md`: Standards: what Faraday follows, how fully, and where it decides. Built.
- `docs/STICK-SETUP.md`: Sticks and vaults, laid out together. Built (F175): the Sticks and vaults card, the sticks brought in and checked before the keys, Check it later, and a stick's sheet.
- `docs/TEXT-CURSOR.md`: The caret in every text field. Built: the arrow keys, Home, End and a press place it in every field, the grids of words and codex32's boxes among them.
- `docs/TEXT-SIZE.md`: Text size. Built (F85): Standard, Large and Larger, in Faraday and the Starter.
- `docs/THEMES.md`: Themes on Material 3's colour system. Built (F113); the website, the guide's pictures and the README's screenshots are not yet redrawn in the new colours.
- `docs/TOTP.md`: TOTP codes and the time. Built.
- `docs/WALLETS.md`: The Wallets tab. Built.

**Spec**

- `docs/ANTI-EXFIL.md`: Anti-exfil, Faraday checking a signer and signing with it. Planned, not built; experimental, for dev builds only (F162, F176).
- `docs/BACKUP-FLOW.md`: The backup flow. Partly built: the Nostr key's part is open.
- `docs/DESKTOP-HOST.md`: Faraday Demo as a simulator: the host. Built (F118); the website's part planned.
- `docs/FLOWS.md`: Flows. Built but for the items under Open.
- `docs/IMAGES.md`: Images for each piece of hardware. Specified (F145); the PC's and the Pi's builds, the one Pi image and the release's files are built; trials and measurements are open.
- `docs/IMPACT.md`: What a change brings with it: one registry of rules, checked at landing and before the website publishes. Built (F149).
- `docs/INTEROP.md`: Interoperability with other wallets. Tested.
- `docs/LARGE-FILES.md`: Large files on a stick visit. Not built (F26, amended by F30).
- `docs/MAC.md`: Faraday on Macs. Partly built; paused until a Mac is available to test on.
- `docs/M3-CHECKS.md`: Material 3 checks: touch targets tested over the snapshot tours, and state layers by construction. Built (F152).
- `docs/MODES.md`: Modes, Practice and samples. Built here (F81), except the website.
- `docs/MOTION.md`: Scrolling, motion and appearance. Built; not yet checked by the owner on hardware.
- `docs/CRYPTO-LEARN.md`: How Faraday does it: one manifest for every cryptographic action. Specified (F148); the manifest, its inventory lint, fingerprints, re-stamp recipe and generator are built and held by `just lint`; the order of flow blocks, guide pictures, the website's rendering and a few tables are open.
- `docs/STICK-IDENTITY.md`: A stick that does not say it is Faraday's. Specified (F174, F182); built, a stick Windows leaves whole included; macOS keys checked against a Mac (paused until one is available) are not.
- `docs/VAULT.md`: The Faraday vault: a KDBX 4 database under one passphrase and Argon2id, its items as KeePass entries, in four sizes, and the smallest, a QR code, an RFC 9580 OpenPGP message. Specified (F190, amending F189; F226); the file vault and the QR vault tool are built; key files and a YubiKey's secret are built in part (its Open line).
- `docs/WORKERS.md`: Costly work off the drawing thread: work over a tenth of a second on a Pi 3B+ on one worker in the core for every Faraday (F184), shorter work drawn first; the list of each, the screen while a job runs, the token and key material on the worker. The worker, the shells' wake, the busy screen, the vault stretches, PSBT signing and read, the backup checklist and more are built; the rest of the jobs (§9) are open.
- `docs/FRAMES.md`: Frames at the panel's rate: a scroll step that moves the rows already drawn, the frame painted in bands across the cores, the Pi's panel without a conversion, the framebuffer write. Partly built: the moved frame on the compact Settings page, the stick shell taking it, the Pi's frame without a conversion and most of the framebuffer write; its Status and Open lines say the rest.
- `docs/CROSSCHECK.md`: A second implementation of each cryptographic step, and which compare Faraday with others' code and which with Faraday's second version (§10, §10.7, F160, F177, F179). Option A decided (F79) and built (§10.5); options B and C deferred.
- `docs/NOSTR.md`: The Nostr key. Not built, not scheduled (F27, amended by F34).
- `docs/SECRET-LEAKS.md`: Tests that fail on a secret left in memory or on screen: a test-only allocator that scans freed and live blocks for the test secrets' forms, the tours' check of the frame after Lock, a hide or leaving a page, the secret types' rules and the named entries that let a known copy through (F250). Built; the files that name a secret type and still build heap text are later work.
- `docs/SPEED.md`: Speed in long flows. Built but for the glyph cache (§3.8).
- `docs/STARTER.md`: The Starter. Partly built.
- `docs/STRUCTURE-PASS.md`: The structure pass. Built.

**Record**

- `docs/DECISIONS.md`: Decisions. Kept as written.

**History** (`docs/history/`): every finished Spec's log and every replaced decision, read by listing the folder.

**Elsewhere:** `docs/decisions/` holds one file per decision, `docs/history/learn/` the old Learn pages, `docs/deps/` a page per dependency, `docs/screenshots/` the README's pictures.
