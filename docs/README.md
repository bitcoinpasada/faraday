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
  line naming where it went, so a cited section number still resolves. Documents deleted from `docs/history/` are kept at the git tag `history/docs-2026-11`.

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
| How does a person use Faraday, step by step, with the screens? | the guide's pages, `docs/guide/<place>.md` (`docs/GUIDE-IN-APP.md`) |
| What does a word Faraday uses in its own way (key, seed, plan, place, seal, vault, quorum) mean? | the guide's glossary, `docs/guide/glossary.md` (`docs/GUIDE-IN-APP.md` §3.1) |
| What does a screen look like, and what is it made of? | `docs/DESIGN.md`; the Starter's screens, `docs/STARTER.md` §3 |
| What happens, step by step, in a flow? | `docs/FLOWS.md`; backups, `docs/BACKUP-FLOW.md`; spending, `docs/SPEND-FROM-BACKUP.md` |
| How does a step flow show one step at a time, fade between steps and list Your choices? | `docs/FLOW-CONTAINER.md`; the decision, `docs/decisions/F296.md` |
| How are codex32 shares made, checked by hand and repaired? | `docs/CODEX32.md` |
| How is a lost GPG key restored from its paperkey? | `docs/GPG-PAPERKEY.md` |
| How is a vault stored? | `docs/VAULT.md`, the KDBX vault (F190); the passphrase in NFC and the seal count are in it (§3, §11) |
| What is the proposed small encrypted format (a 25 × 25, 29 × 29 or 33 × 33 QR code, or 36, 48 or 56 words), offered for outside review? | the ACE specification, in its own repository: `https://github.com/bitcoinpasada/acenv`, `ACE.md` (F264) |
| What is in the KeePass (KDBX 4) file Faraday writes? | `docs/VAULT.md` §8: an export is a vault of one entry |
| How does Faraday read and write Krux's encrypted codes, and what are their trade-offs? | `docs/KEF.md` |
| How does data move by QR code, and how is a code scanned? | `docs/QR.md` |
| What does each scanner do with each kind of code, and with one read in the wrong scanner? | `docs/SCANNING.md` |
| Which other wallets' files and codes does Faraday read and write? | `docs/INTEROP.md`, `docs/WALLETS.md` |
| How does the Starter write a stick and start a computer from it? | `docs/STARTER.md` |
| Did a signing device do only what it was asked? | `docs/SIGNER-CHECK.md` |
| What does Check a timestamp take and show, and what does a proof prove? | `docs/DESIGN.md` §5 (`OtsCheck`), `docs/decisions/F230.md`, `docs/guide/ots-check.md` |
| What does My seed makes my wallet take, show and keep? | `docs/FLOWS.md` (My seed makes my wallet), `docs/DESIGN.md` §5 (`SeedCheck`) |
| What does Read a Bitcoin Core wallet take and show, and what does it check? | `docs/DESIGN.md` §5 (`CoreWallet`), `docs/decisions/F229.md`, `docs/guide/core-wallet.md` |
| What does Decode a Lightning invoice show, and how does it open? | `docs/DESIGN.md` §5 (`Invoice`), `docs/SCANNING.md`, `docs/guide/invoice.md` |
| How do two PSBTs differ, and does a replacement only raise the fee? | `docs/COMPARE.md` (the rules); `docs/SIGNER-CHECK.md` §11, where they are shown |
| What does the Check place list, and where is it in the navigation? | `docs/DESIGN.md` §5 (`Check`), `docs/decisions/F255.md` |
| What does Check an address show for a found address and for a miss? | `docs/DESIGN.md` §5 (Check an address, The result), §4.5 (Whole string); the search, `docs/QR.md` §2 |
| What does Check a signer show, and how does it open? | `docs/SIGNER-CHECK.md` (§3, the flow), `docs/guide/signer-check.md` |
| How does a person check that a seed makes their wallet (My seed makes my wallet)? | `docs/FLOWS.md` (My seed makes my wallet), `docs/DESIGN.md` §5 |
| How does a person make a wallet at a path they found in Explore, or from an address the address check found in a key's account? | `docs/FLOWS.md` (Explore, Make a wallet at the path), `docs/DESIGN.md` §5 (Make a wallet) |
| How does a person check a multisig setup (its quorum, checksum and cosigners, and which cosigner a seed is)? | `docs/FLOWS.md` (Check a multisig setup), `docs/DESIGN.md` §5 (`MultisigCheck`) |
| How does text get larger for people who need it? | `docs/TEXT-SIZE.md` |
| Where does the caret go in a text field, and how is it moved? | `docs/TEXT-CURSOR.md` |
| How is a screen laid out at each window size, from the Pi to a monitor? | `docs/LAYOUT.md` |
| What does Home show at each size, and in which order do the destinations stand? | `docs/HOME.md`; the layouts' rules, `docs/LAYOUT.md` §3, §4 |
| Which way up is a 7-inch or 10-inch panel, and how are its screen and touch turned? | `docs/ORIENTATION.md` |
| How does Faraday Demo show its screens at another screen's size? | `docs/DESKTOP-HOST.md` §10 |
| What stands outside the app on Faraday Demo: the sticks, the size, Practice's aids? | `docs/DESKTOP-HOST.md` |
| How are the themes' colours made, and which colour role does each component take? | `docs/THEMES.md`; `docs/DESIGN.md` §3 |
| Can a person add their own randomness, and when is the stick's kernel ready? | `docs/OWN-RANDOMNESS.md` |
| How does a seed change form and stay the same wallet? | `docs/SEED-CONVERSION.md` |
| Which boards, platforms and screens does Faraday run on, with which layout, and which were considered? | `docs/HARDWARE.md` |
| Which image does each piece of hardware get, and how does a person choose and check it? | `docs/IMAGES.md` |
| What are TOTP codes and the clock? | `docs/TOTP.md` |
| How do I build Faraday, check it, reproduce a release, or try it on my own hardware? | `docs/DEVELOPING.md` |
| What are Practice and the developer builds? | `docs/DEMO.md` |
| Why is a flow fast or slow, and how is costly work kept out of drawing? | `docs/SPEED.md`, `docs/COMPUTE.md` |
| How does Faraday keep a secret from being left behind? | `docs/KEYS.md` (where secrets live and wipe), `docs/SECRET-LEAKS.md` (the tests that find a copy), `tools/README.md` (which lint holds it) |
| Which thread does costly work run on, and what does a screen do while it runs? | `docs/WORKERS.md` |
| How does a frame reach the panel at its refresh rate: a scroll step, the cores, the panel's conversion and write? | `docs/FRAMES.md` |
| What could still make drawing cheaper, how much, and what was looked at and left? | `docs/DRAW-COST.md` |
| How do the modes, Practice and its samples work? | `docs/MODES.md` |
| How do scrolling and motion work? | `docs/MOTION.md` |
| Why is a dependency there? | `docs/deps/<crate>.md`; how they are checked, `docs/deps/README.md`; the policy is DECISIONS F71 |
| Which tool or lint does what? | `tools/README.md` |
| What must change with what? | `docs/IMPACT.md`; which lint holds what, `tools/README.md` |
| What is run before a release, and which parsers are fuzzed? | `tools/README.md`, "Before a release"; `fuzz/README.md`, `faraday/fuzz/README.md` |
| For each cryptographic action, which standard, code and checks, and how does a screen show it; what fails `just lint` when one is added or changed? | `docs/CRYPTO-LEARN.md` |
| Where are keys held, and how do they leave? | `docs/KEYS.md`, then `docs/AUDIT.md` §2 |
| What is planned and not built? | each Spec's `**Open:**` line (the Specs are listed below) |
| How do I submit an audit, or an outside review of one of Faraday's own specifications? | `docs/AUDIT-SUBMISSIONS.md`; the own specifications, §10 |
| How do I report a security problem? | `SECURITY.md` |
| What are the rules for changing this repository, and how do I send a change? | `CLAUDE.md`, `CONTRIBUTING.md` |
| What did an earlier plan say? | `docs/history/decisions/` for the retired decisions; deleted documents, build logs included, are at the git tag `history/docs-2026-11` |

## Every document

Generated from each document's header (`python3 -I -B tools/docs-index.py write`); change the header, not this list.

<!-- docs-index: begin -->

**Reference**

- `docs/AT-LOCK.md`: What holds a secret: one rule for clean, the idle lock and Lock. Built: the `AtLock` trait and `Public` marker (`faraday-core/src/at_lock.rs`), the exhaustive listing of `Faraday` and of `ToolsArea`, the clean state, the idle lock's test and Lock's clears derived from it (§4 to §6), and the leak tour that walks every area by name (§7).
- `docs/AUDIT.md`: What is cryptography, and where it lives. Built (the key boundary, F70).
- `docs/BACKUP-FLOW.md`: The backup flow. Built: GPG keys, the Secure Boot keys and the Bitcoin wallet.
- `docs/CODEX32.md`: Codex32: making, checking and repairing shares. Built.
- `docs/COMPARE.md`: Comparing two transactions, and the fee bump. Built (F233, F270).
- `docs/COMPUTE.md`: Compute: costly work takes a token. Built (F69).
- `docs/CROSSCHECK.md`: A second implementation of each cryptographic step. Option A decided (F79) and built, §10.5 items 1 to 6; the references of Faraday's own replaced by outside code where it exists (§10.7).
- `docs/CRYPTO-LEARN.md`: How Faraday does it: one manifest for every cryptographic action. Built (F148, F190): the manifest, its inventory lint, the generators and check.py held by `just lint`; Learn's blocks and §18's constructions in the app; the website's /cryptography page with the constructions (§11.1, §18.7).
- `docs/DEMO.md`: Practice, and Faraday Demo for developers. Built (F30, F81, F126; Practice's aids in Faraday Demo's host strip, §14, F118).
- `docs/DESKTOP-HOST.md`: Faraday Demo as a simulator: the host. Built (F118, `faraday/faraday-host/`): the host holds the app, the strip at the window's foot with the sticks, Samples, Size and every practice aid of the screen on show, the preview frame centred above it, the pointer carried in; the host's colours the app's theme turned inside out, the screen's aids in a section of their own colour, the strip's row centred and scrolling sideways where it is wider than the window, its controls 48 dp tall whatever the window and the preview (§2, §3); Samples' sheet is the host's (§6); at the window's own size the shell copies the app's pixels into the window once (§9); the menus fade in and out (§5); the menus, Samples' sheet and the strip's row scroll under a finger and coast after it, a tap pressing once (§5); a stick the app asks for has its pill marked, pulsing (§3); the app's frame carries a focus border (§4); the die's values are in the entry at the click (§3.2); the app draws no stick pill, Load samples, Preview size or "Demo only" card; it runs inside Faraday Starter's window, under the Starter's top app bar (§1, F126).
- `docs/DEVELOPING.md`: Developing Faraday. Built.
- `docs/ESTABLISHED.md`: Established implementations first. Built (F178, F180, F181, F190, F191, F192, F193, F213): the timing fixes, the switches, the reasons in the manifest and their check, the generated lists and the cross-check's outside sources.
- `docs/FLOWS.md`: Flows. Built; the code is the record where it differs.
- `docs/GPG-PAPERKEY.md`: Restore a GPG key from its paperkey. Built (F169, F215): reading a paperkey and a certificate, the restore and its tests against paperkey and GnuPG in `faraday-pgp`; the restore set in `faraday-keys`; the flow in faraday-core, `faraday/faraday-core/src/backup_flow/gpg_restore.rs` and its screen; the public key and the paperkey scanned as codes, and the paperkey shown as a code.
- `docs/GUIDE-IN-APP.md`: The guide: one guide, in the app, opened where the person is. Built (F247).
- `docs/HARDWARE.md`: Hardware: boards, platforms and screens. Built: every platform and screen Faraday is built for, and the boards considered and not supported.
- `docs/HOME.md`: Home at each layout. Built: the wide layout's Home, the phone-size window's and the rail's windows' (one column with the search bar and the places), and the Pi's menu.
- `docs/IMPACT.md`: Impact: what a change brings with it. Built (F149, narrowed by F258).
- `docs/KEF.md`: KEF: Krux's encrypted code, read in full and written in two versions. Built (F219, amended by F261).
- `docs/KERNEL-PATCHES.md`: The kernel patches the images carry. Built.
- `docs/KEYS.md`: The key boundary: faraday-keys. Built (steps 0 to 4); the "as built" sections of §4 are the boundary as it stands.
- `docs/LAYOUT.md`: Layout at every window size. Built: the five size classes and density (F101), the navigation bar, rail and expanded rail, and the top app bar over every page on every class but compact short, Home and the step flows included (§3); Home's lead tile and tile grid (§4); list and detail (§5, §17); the supporting pane, Guide in it on extra-large windows (§6); the page area to the window's edge (§15); reading width and wide content (§14); Make codex32 shares on a wide window (§16); the tours at every class's size and §11's tests.
- `docs/M3-CHECKS.md`: Material 3 checks: touch targets and state layers. Built (F152).
- `docs/MODES.md`: Modes, Practice and samples. Built (F81).
- `docs/ORIENTATION.md`: Panel orientation. Built (F100): native orientation and the panel's density read at start, the known list, the stick shell turning its frame, touch and arrow, Settings › Display › Rotation kept on the stick, the panels' snapshot sizes and the Size menu's portrait rows.
- `docs/OWN-RANDOMNESS.md`: Your own randomness, and the kernel's at boot. Built (F86, F157): §2.3's mixing into the seed and the pool, and the lock (`faraday/faraday-core/src/fresh.rs`); New key's This device mixing and its label (§2.3, §2.4; `faraday/faraday-core/src/keygen/mod.rs`, `device_own`); the Add randomness page with camera, dice, coins and cards, Settings' Randomness row, the status line and New key's link (`faraday/faraday-core/src/fresh/page.rs`, `faraday/faraday-core/src/screens/randomness.rs`); pointer and keys on the desktop app (`core/faraday-entropy/src/pointer_keys.rs`); option A (§3.3); §8's Randomness row at every press that draws the system's randomness, first in the Pi's ⋮ (F157; `faraday/faraday-core/src/screens/randomness.rs`, `row`); the kernel check of §3 and its boot measurement (§3.4) on QEMU's worst case, a PC and the Pi.
- `docs/QR.md`: QR transfer and scanning. Built.
- `docs/README.md`: Faraday's documents. Built.
- `docs/SCANNING.md`: Scanning: what each scanner takes, and the way over. Built; the owner's choices were taken on 2026-10-29 (F173).
- `docs/SECRET-LEAKS.md`: Secret leaks: tests that fail on a secret left in memory or on screen. Built (F250): the allocator, the flow tests, the tours' check, the clippy refusal of `clone`, `to_string`, `to_owned` and `to_vec` (each secret-holding crate's own `clippy.toml`) and `lint-secrets`' rule on every secret type.
- `docs/SECURITY-MODEL.md`: Security model. Built.
- `docs/SEED-CONVERSION.md`: Convert a seed. Built.
- `docs/SIGNER-CHECK.md`: Check a signer. Built, the seed step (§12) and the one flow for two PSBTs or finished transactions (§11) included; the check is `faraday_psbt::signer` and the flow is the decode area's `check.rs` and `check_screen.rs`; in Practice both scanners offer a sample pair (`docs/MODES.md` §5.3).
- `docs/SPEED.md`: Speed in long flows. Built; the glyph cache measured (§3.8).
- `docs/SPEND-FROM-BACKUP.md`: The Spend tab. Built.
- `docs/STANDARDS.md`: Standards: what Faraday follows, how fully, and where it decides. Built.
- `docs/STICK-IDENTITY.md`: A stick that does not say it is Faraday's. Built (F174, F182): the image, the device, the Starter, the Name field, Learn and the guide (§3 to §8), with a stick Windows leaves whole (§4, §5.1).
- `docs/STICK-SETUP.md`: Sticks and vaults, laid out together. Built (F175; `faraday/faraday-core/src/backup_flow/`, `faraday/faraday-disk/src/lib.rs`).
- `docs/STRUCTURE-PASS.md`: The structure pass. Built: phases 0 to 7 and the lints that hold them.
- `docs/TEXT-CURSOR.md`: The caret in every text field. Built (`faraday/faraday-core/src/edit.rs`, `Ui::typed`, `Ui::typed_lines`, `Ui::draw_typed`).
- `docs/TEXT-SIZE.md`: Text size. Built (F85): the setting in Faraday and the Starter, its storage, the one-column decision, the components and screens that make room for larger text, the Larger renders and their `cut.txt`.
- `docs/THEMES.md`: Themes on Material 3's colour system. Built (owner, 2026-10-24; F113, `docs/decisions/F113.md`): the roles, the generator and the tables; every screen and component of Faraday and the Starter drawn on the roles by name; Settings › Appearance's Theme, Mode, Contrast and Reduce motion, the settings file's lines and the migration, the Starter's Appearance page; the theme tour.
- `docs/TOTP.md`: TOTP codes and the time. Built.
- `docs/VAULT.md`: The Faraday vault. Built (F190; F208; F227); the app reads and seals KDBX 4 file vaults, each seal under the vault's own passphrase, growing by itself from 32 KiB to 128 MiB and at the memory step at or above its own.
- `docs/WALLETS.md`: The Wallets tab. Built.

**Spec**

- `docs/AUDIT-SUBMISSIONS.md`: Audits: the request, the submission and the record. Built (F19), except the Audits page's part of §10 (F160).
- `docs/DESIGN.md`: Design system. Built but for the items under Open.
- `docs/DRAW-COST.md`: Cheaper drawing: what is left. Specified; none of it is built and none of it is timed.
- `docs/FLOW-CONTAINER.md`: One flow container. Specified (F296); built.
- `docs/FRAMES.md`: Frames at the panel's rate. Specified; the first two steps of §3 are built (`Ui::region`, the whole-pixel offset, the pages and panes laid out from offset 0 and the content layer; the moved frame with `moved.rs`, `Out::redraw` and `Out::moved`, `Parts::moved` and the two tests of §3.2 and §3.4, opted in by the small panel's Settings, Wallets, a wallet's page, Files, Vault contents, Backups, the backup board and checklist and the step columns the test walks, and by the wide Wallets, Vault contents, Settings, Backups and those step-column pages), and so are the Pi's frame written without a conversion (§5, the kernel patch and the shell's red-first depth; its figure on a card is not yet taken), the framebuffer write's first three points (§6) and the face kept per size, `Ui::wrap_lines` linear in its words and Home's backup count and a wallet's backup line worked out without every wallet's map, the signing page's text lines kept by file, the Backups screen's list and the wallet chart kept across moved frames and the shell's calls to the disk process on a thread of their own, of §7; the stick shell takes the moved frame (§3.7: `Presenter::present_moved`, in ordinary memory, with the panel written bands rather than a span), and the frame drawn in bands across the cores (§4: `faraday-core`'s `bands.rs`, in every whole frame; a moved frame paints its strip as it is called); and the stick's centred window on a larger screen (§6, `faraday.window=`); the rest is not.
- `docs/IMAGES.md`: Images for each piece of hardware. Specified (F185, F186), the PC's one build, the Pi's one build, the one Pi image for an HDMI screen and the 2.8-inch (its screen directories, overlays, boot script and kernel options) and the release's files and image list built.
- `docs/INTEROP.md`: Interoperability with other wallets. Tested; §2 lists what works; §5 is built except Electrum seed words.
- `docs/MOTION.md`: Scrolling, motion and appearance. Built; not yet checked by the owner on hardware.
- `docs/STARTER.md`: The Starter. Partly built: the Starter on Linux and Windows with its Material 3 navigation (§3.1d), the Pi 3 card.
- `docs/WORKERS.md`: Costly work off the drawing thread. Partly built: the worker, the shells' wake, the busy screen and the jobs listed in §9 run; the rest are open.

**Record**

- `docs/DECISIONS.md`: Decisions. Kept as written; indexed here.

<!-- docs-index: end -->

**History**, in `docs/history/`, describes how things were built and
  the tree as it was. A section moved there leaves its heading and one
  line naming where it went, so a cited section number still resolves. Documents deleted from `docs/history/` are kept at the git tag `history/docs-2026-11`.

**Elsewhere:** `docs/decisions/` holds one file per decision, `docs/deps/` a page per dependency, `docs/screenshots/` the README's pictures.
