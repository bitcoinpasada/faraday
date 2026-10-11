# OpenSigner — UX: Jobs, Navigation, Frames, Components, Flows

**Status:** Draft v0.4 · companion to PLANNING.md · consolidates the former JOBS.md and JOURNEYS.md

> **How to read this document.** Both PLANNING.md and this file were drafted by Claude in conversation with the project owner, and Claude will also do the implementation. Nothing here is a hard-and-fast rule. These documents record our *current best understanding*; when implementation, testing, or use reveals something better, the right move is to change the document and the code together, not to follow the document off a cliff. Where the text says "always" or "never" it means "this is a deliberate default with a reason behind it — change it knowingly, not accidentally." The security policies in PLANNING.md §5 are the closest thing to firm commitments; even those may be refined as understanding improves.

---

## 1. Design stance

OpenSigner is SeedSigner-*inspired*, not a clone. Ideas taken deliberately: SeedSigner's stateless model, dice and QR ergonomics, fingerprints on the home screen; Sparrow's structured transaction review; Trezor Suite's step-by-step confirmation and guided onboarding; Ledger's "you are the signer, the coordinator is untrusted" framing; Ian Coleman's live derivation workspace; Coldcard's address explorer, Seed XOR and blunt warnings. The layout itself is derived from the jobs inventory (§3) and four facts about our users:

1. Multiple keys are normal; the fingerprint is a key's identity everywhere.
2. Verification (address, wallet config, signature, nonce, backup) is a first-class job, a peer of signing.
3. Learning must be self-contained: a novice with only this app should get from zero to safe self-custody.
4. Screens range from 2.4" to desktop; we optimise for the 2.8"–6.5" touch range and adapt outward.

---

## 2. Screen classes and input

Touch is required. Every screen class is a first-class target; nothing is "the fallback."

**Small panels are portrait.** The 2.8" reference panel is 480×640 native, and every screen is designed for a tall, narrow column. The layout engine still solves a landscape panel, but nothing is designed for one.

| Class | Reference size | Panel | Layout |
|---|---|---|---|
| **small** | 480×640 @ 286 dpi; 240×320 @ 143 dpi is the minimum | the 2.8" panel on the Pi | one column, a 2-column hub grid that fits without scrolling |
| **mobile** | 1080×2340 @ 420 dpi | phones | one column; a 2-column hub grid packed at the bottom of the screen |
| **wide** | 960×640 @ 160 dpi | tablet, desktop, a laptop booted from the stick | the sidebar on every screen, plus a content pane |

A class does more than swap variants. The **type ramp** is scaled with
it — 1.0 on `small`, 1.1 on `mobile`, 1.3 on `wide` — because a phone is
read at the same distance as a panel twice its size and a desktop window
at twice a phone's. Lengths are not scaled: a touch target is 48 dp
everywhere. **Chips** have a floor of 44 dp on `small` and `mobile`, where a finger
aims, and 32 dp on `wide`, where a pointer does; a **candidate cell**
follows the chip except on `small`, where it is 36 dp so that two rows
of candidates fit above the keys (§16.118). A screen may also branch on how much room it has in dp,
not only on the class it belongs to.

**The desktop window has one size.** A desktop build is not a resizable app: it opens at 960×640 at 160 dpi, the `wide` class, which fits any laptop. `--size` and `--dpi` are review flags for the other screens the app is built for, not a way to run it at an arbitrary size.

**A laptop is a device class, not a port.** The OpenSigner stick boots a
laptop or a PC into the same binary the Pi runs (`opensigner/shells/pi`),
so a machine somebody already owns becomes an airgapped signer for as
long as it is powered from the stick and nothing else. The display is
whatever the firmware left running — 1366×768, 1920×1080, 2560×1600 —
and every one of them is the `wide` class at 160 dpi, which is the
density the design system's dp are 1:1 at. Nothing in the core changed
for it: the screens are spacious, a content column in the middle of a
large black field, which is what a screen designed for a 2.8" panel
looks like on a laptop (PLANNING.md §16.94, §16.95).

**A keyboard and a mouse are accelerators too.** The Pi shell reads a USB
or built-in keyboard, a mouse and a touchpad over evdev, so the same
binary drives a laptop booted from the OpenSigner stick. A mouse or a
touchpad is a touch at a cursor the shell draws, and the wheel scrolls;
a keyboard types where an on-screen keyboard would, in a US layout.
Neither is required: every flow is fully usable with touch alone, and no
screen is designed for a pointer (PLANNING.md §16.94).

Not supported (at least initially): screens under ~2.4" / 240×240 (SeedSigner classic, Krux M5StickV) and d-pad-only devices. Supporting them would force T9 keyboards, paged everything, and an unusable Learn section; those devices are already well served by SeedSigner/Krux. A reduced "lite" profile can be revisited if demand appears. Optional physical buttons on DIY builds are accelerators, not the sole input.

Layout engine, size classes, density-independent units, fonts and rendering: see PLANNING.md §4.3–4.6.

---

## 3. Jobs inventory (what people come here to do)

**Rule for this section:** no screens, no menus. Only jobs, who has them, how often, what triggers them, what goes in and out, and what "done well" means. The UI in §4–§7 is designed *from* this.

Frequency scale: **daily** · **weekly** · **monthly** · **yearly** · **once** · **rare**
Persona tags: **N** newcomer · **H** holder (has keys, transacts occasionally) · **P** power user / multisig · **B** DIY builder · **T** tinkerer/learner · **A** auditor/helper (checking someone else's setup)

---

## A. Getting started

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| A1 | Understand what this thing is and how much to trust it on *this* device | N H | once | first launch | — → clear tier statement | user can say in one sentence what the OS can/can't see |
| A2 | Confirm the software is what it claims | P A | once/update | first launch, after update | — → hash, version, *and* how to check it | shows the core hash; explains (offline, in-app) how to verify a downloaded binary against the developers' signing keys, and how to build from source and compare; includes a QR/URL to the canonical instructions page. Hard, but done properly it's the only proof the rest is real |
| A3 | Confirm the device works (self-test) | all | every start | start | — → pass/fail | fails loudly; never silently degrades |
| A4 | Be guided through creating the first key safely | N | once | no keys loaded | — → a backed-up key | user has words on paper and has proved it via quiz before doing anything else |

## B. Key creation

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| B1 | Create a key from dice I rolled | H P | rare | new wallet | rolls → mnemonic | count/bias sanity shown; user can see the bits → words if curious |
| B2 | Create from coin flips | T | rare | | flips → mnemonic | |
| B3 | Create from a shuffled deck | T | rare | | cards → mnemonic | deck tracker prevents double-entry |
| B4 | Create from camera noise | H | rare | | image → mnemonic | user understands it's device-trusting-ish |
| B5 | Mix several sources | P | rare | distrust any single source | 2+ sources → mnemonic | each source's commitment visible before mixing |
| B6 | Create from the device RNG (quick, lower trust) | N | rare | testing, low value | — → mnemonic | caution is explicit, not nagging |
| B7 | Enter raw entropy I generated elsewhere (hex/binary) | P T | rare | | bits → mnemonic | |
| B8 | Pick word count and word language | all | with B1–B7 | | choice → | English default; other languages equally first-class |
| B9 | Derive a child key (BIP-85) | P | rare | need a new seed from an existing one | index → child mnemonic | child has its own fingerprint, can be loaded/backed up like any key |
| B10 | Split a key into XOR parts / combine parts | P | rare | distributed backup | key → N keys via Backup › "Split with Seed XOR", or N → 1 via Load a key › "Seed XOR parts" | each part is a real key with fingerprint; the math is shown on request |
| B11 | I have 11 or 23 words (chosen or rolled) — find valid final words | T P | rare | manual seed construction | words → candidates | explains that the last word carries checksum bits |
| B12 | Create a strong passphrase from dice (EFF diceware) | H P | rare | before C4 | rolls → words | entropy bits shown; user told how to store it separately from the seed |

## C. Loading keys (every session on stateless tiers)

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| C1 | Load by scanning my SeedQR | H P | weekly–monthly | want to sign/verify | QR → loaded key | 5 seconds; fingerprint confirmed |
| C2 | Load by typing words | H | monthly | no SeedQR handy | words → loaded key | candidates from first letter; typo located if checksum fails |
| C3 | Load by word numbers / hex | P T | rare | steel plate with numbers | numbers → key | |
| C4 | Apply a passphrase, and *know* I typed the right one | H P | with C1–C3 | | passphrase → distinct key | with/without fingerprints both visible; recognise, not re-type |
| C5 | Load several keys at once (multisig, several wallets) | P | weekly | signing a multisig | N keys → N loaded | each clearly labelled by fingerprint; switching is instant |
| C6 | Load from the secure element (Tier B) | H | daily–weekly | phone use | auth → loaded key(s) | biometric/PIN; nothing else to do |
| C7 | Tell which of my loaded keys is which | all | constantly | | — → fingerprints | fingerprint everywhere a key is referenced; optional label |
| C8 | Forget a key, or everything, right now | all | every session end | done / paranoia | — → wiped | one deliberate gesture; obvious it happened |

## D. Backup and its verification

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| D1 | Write my words down (paper/steel) | all | once per key | after B | key → words on screen | numbers shown by default, words on hold; paged per size |
| D2 | Copy my words as numbers for a numbered steel plate | H | once per key | | key → numbers | |
| D3 | Make a SeedQR on paper by hand (transcription grid) | H P | once per key | want QR loading without a printer | key → 21/25/29 grid | thick guide borders, row/col labels, zoom by quadrant on small screens; printable blank on desktop |
| D4 | Show my SeedQR on screen to scan into another device | H P | rare | migration | key → QR | screen-blank-until-held; camera warning on phones |
| D5 | Prove my backup is correct — all words, random order — any time | all | after D1, and yearly | paranoia, inheritance planning | backup + memory → pass/fail | every word tested; can be re-run for any loaded key forever |
| D6 | Check that a *physical* backup I hold matches a loaded key (without revealing) | H A | yearly | audit | — → match/mismatch | via quiz or fingerprint comparison; never shows the words |
| D7 | Save a key to the phone's secure element | H | once per key | Tier B convenience | key → SE-wrapped | user understands device-bound; words still the backup |
| D8 | Remove a key from the secure element | H | rare | | | separate from forgetting in RAM |

## E. Setting up a wallet with a coordinator

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| E1 | Give my xpub/descriptor to Sparrow/Nunchuk/BlueWallet/… | H P | once per wallet | new wallet | key + script type + account → QR/text | right format for that coordinator, path and fingerprint shown |
| E2 | Verify the multisig the coordinator built is really made of the keys I think | P A | once per wallet, and after any change | Add › "Scan a wallet", or Scan; a BIP-388 policy, a multipath descriptor or a coordinator's text config | config → review | quorum, each cosigner fingerprint, own key highlighted, checksum; swapped-xpub detection |
| E3 | Remember a wallet config so change can be verified later | P | once per wallet | after E2 | config → in use for the session | a row on Home with its checksum, and a page of its own; forgotten by one tap, and by a wipe. Across restarts on Tier B, PLANNING §16.72 |
| E4 | Compare the first receive address with the coordinator | all | once per wallet | after E1/E2 | — → address | full address, chunked, on one screen |

## F. Transacting

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| F1 | Sign a PSBT after understanding it | H P | weekly–monthly | coordinator built a tx | PSBT + keys → signed PSBT | every output full address; change verified or loudly not; fee absolute + rate; which keys signed |
| F2 | Sign a multisig PSBT with one, some, or all of my loaded keys | P | weekly | | | see who already signed; pick which of mine sign |
| F3 | Be told when the coordinator is lying or careless | all | with F1 | | | unverified change, high fee, dust, odd sighash, wrong network, address reuse — all surfaced, ranked |
| F4 | Get the signed result back to the coordinator | all | with F1 | | → QR/UR/SD/file | animated QR that scans first time; finalised hex when complete |
| F5 | Prove to myself the signature couldn't have leaked my key (nonce) | P | with F1, occasionally | paranoia | → nonce status / signature bytes | on-device "nonce OK"; export bytes to compare with another implementation |
| F6 | Use anti-exfil with a coordinator that supports it | P | with F1 | Jade/Green-style host | host randomness ↔ commitment | user knows verification is host-side in this mode |
| F7 | Sign a message (BIP-322/137) | H P | rare | prove ownership | msg + key → sig | |
| F8 | Take part in a MuSig2 signing session (v2) | P | rare | | two rounds over QR | never loses nonce state mid-session; never reuses it; obvious "session open" |
| F9 | Sign on testnet/signet while learning | N T | rare | | | network always visible when not mainnet |

## G. Verifying things

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| G1 | Is this address mine? (which key, which index) | all | monthly | about to receive | address → yes/no + key + index | searches receive+change for all loaded keys / remembered configs |
| G2 | Show me my addresses so I can compare with the coordinator | H P | monthly | | key → list | full chunked addresses, QR each, index visible |
| G3 | Does this xpub belong to one of my keys? | P A | rare | reviewing a config | xpub → yes/no | |
| G4 | Verify a message someone sent me | H | rare | | addr+msg+sig → valid? | |
| G5 | Decode a raw transaction or PSBT without signing anything | P T A | rare | curiosity, support | hex/QR → structure | no key required |
| G6 | Check a descriptor's checksum / validity | P | rare | | descriptor → ok/fix | |
| G7 | Confirm my software hash after an update | P A | per update | | → hash | (same as A2) |

## H. Exploring (the deep end — Ian Coleman-style, for users who already know the concepts)

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| H1 | See how words ↔ bits ↔ hex ↔ index work | T N | rare | curiosity | word → all forms | live: change one thing, everything updates |
| H2 | See the whole chain: entropy → checksum → words → seed → xprv → xpub → address | T | rare | | | every step visible; secrets hold-to-reveal |
| H4 | Explore any derivation path and see keys/addresses at each level | P T | rare | | path → tree | |
| H5 | Convert xpub/ypub/zpub and understand it's only an encoding | H | rare | coordinator asks for zpub | | explainer attached |
| H6 | Build or read a descriptor / miniscript policy | P | rare | | | |
| H7 | Encode/decode base58/bech32/hex; compute hashes | P T | rare | | | scratchpad |
| H9 | Read a short explainer when a term appears | N | often at first | tap a term | | one tap away, one screen long; links into the Learn section (L) for more |

## L. Learning (the kiddie pool — a total novice, no outside resources needed)

The app must be self-contained: someone who has only this app and no internet should be able to go from zero to safely holding and spending bitcoin. Text-heavy is fine. Each lesson is short, sequential, plain-language, ends with "try it" that jumps to the real job (on testnet/signet or a throwaway key), and is re-readable any time.

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| L1 | Understand what a seed / the words actually are, and why 12 or 24 | N | once | | | no jargon before it's defined; one analogy that sticks |
| L2 | Understand signer vs wallet vs coordinator vs node, and what this app is and isn't | N | once | | | user can explain why the app has no balance screen |
| L3 | Understand addresses, transactions, inputs/outputs, change, fees | N | once | | | prepares them to read the sign screen without fear |
| L4 | Understand why dice / why not trust a computer's randomness | N | once | before B | | motivates B1 without paranoia |
| L5 | Understand backups: paper vs steel, what destroys them, where to keep them, the "never" list (photos, cloud, typing into websites, telling anyone) | N | once | before D | | user makes a backup plan before making a key |
| L6 | Understand the backup quiz and why it's repeated yearly | N | once | | | |
| L7 | Understand passphrases: what they add, what they risk (forgetting = loss), how to store them | N H | once | before C4 | | user decides deliberately, not by default |
| L8 | Understand fingerprints and how to use them to tell keys apart | N | once | | | |
| L9 | Understand single-sig vs multisig, and when multisig is *not* the right answer | N H | once | | | honest about complexity |
| L10 | Understand why to verify addresses and change, and what a lying coordinator can do | N | once | before F | | user verifies because they understand, not because told to |
| L11 | Understand xpubs and privacy (what sharing one reveals) | N | once | before E1 | | |
| L12 | Understand the air gap and QR exchange, and why this app has no networking | N | once | | | |
| L13 | Understand what the secure element does and doesn't do (Tier B) | N | once | before D7 | | device-bound explained plainly |
| L14 | Recognise common scams and mistakes (fake support, "validate your wallet", clipboard swaps, address poisoning, malicious coordinators) | N H | once, revisit | | | concrete examples, no lecture |
| L15 | Understand inheritance basics: how an heir recovers, what to leave them, what not to | H | once | | | ties to K1 |
| L16 | Practice the whole flow end to end on testnet/signet with a throwaway key | N | once | after L1–L12 | | guided: create, back up, quiz, export xpub, sign a practice PSBT, verify an address — with the real screens |
| L17 | Look up any term (glossary) | N | often | | term → definition | same content as H9 explainers |
| L18 | "I want to … — which job do I need?" | N | often at first | | goal → job | maps plain-language goals to the jobs in this inventory |

## I. Session and device

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| I1 | Lock now / auto-lock | all | daily | walking away | | secrets sealed; unlock is quick |
| I2 | Auto-wipe if I forget the device on | all | background | | | timers visible; wipe is certain |
| I3 | End the session and be sure nothing remains | all | every session | done | | one gesture; confirmation it happened |
| I4 | Change timeouts, masking paranoia, PIN-pad scrambling, UI language, size/orientation | all | rare | | | settings are few and each has a one-line "why" |
| I5 | Open a decoy/empty session under duress (❓) | P | rare | | | indistinguishable from empty |

## J. DIY builder and maintainer

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| J1 | Flash the image and have it work on *my* panel/camera | B | once | build | image + config → running device | one config file: resolution, orientation, touch/buttons mapping, camera |
| J2 | Reproduce the build myself and compare hashes | B P | per release | | source → same hash | documented, scripted |
| J3 | Map physical buttons if I have no touchscreen | B | once | | | every job reachable |
| J4 | Run the same app on a laptop to learn/test before building hardware | B N | once | | | desktop build is the same product |

## K. Helping someone else (auditor/heir)

| # | Job | Who | Freq | Trigger | In → Out | Done well means |
|---|---|---|---|---|---|---|
| K1 | Check a relative's backup works without learning their words | A | yearly | inheritance planning | their backup → pass/fail | quiz mode where the helper holds the device and the owner answers; nothing revealed |
| K2 | Verify someone's multisig config on my device | A | rare | | config → review | no key needed |
| K3 | Walk a newcomer through their first key on their device | A N | once | | | guided mode (A4) is good enough that the helper is optional |

---
### 3.13 What the inventory implies

- **Highest-frequency jobs** on stateless tiers are C1/C4/C7 (load + passphrase + know which key), F1–F4 (sign), G1 (address check), I3 (wipe). These deserve the shortest paths.
- **Scan** is the entry to C1, F1, G1, E2, G5 — five jobs in four different areas. Whether that argues for one global scan or a scan inside each area is a UI question to settle with wireframes; the inventory only says scan is *never* more than one step from the relevant job.
- **Verification jobs (D5, D6, E2, E4, G1–G7, K1, K2)** outnumber signing jobs and several need *no key loaded*. That's the case for verification being a peer of signing rather than a sub-tool.
- **Backup verification (D5/D6/K1)** is a recurring job, not a creation-time step, and has an "owner answers, helper holds" variant.
- **Exploration (H)** is mostly rare-frequency but wide; it wants a single live workspace rather than many entry points.
- **Learning (L)** is a different thing from Exploring: sequential, plain-language, novice-first, self-contained. It is read once per topic but must be reachable from every explainer (H9) and should gate nothing — a novice can skip it, a helper (K3) can point to it.
- **Fingerprint** is the identity in C7, D6, E2, F2, G1, G3 — it must be a consistent component everywhere.
- Roughly a third of jobs need no key loaded at all (A, E2, G3–G7, H1/H3/H5–H8, J, K2). The empty state is not an edge case.

---

## 4. Navigation: hub-and-spoke

- **Home** is the list of what is loaded. A **status line**, then one row per wallet — a loaded key's own single-sig wallet, a policy in use, a wallet over public keys alone — each with the key glyph where this device can sign for it and the eye where it cannot, the kind and the network as the row's label and what the wallet is called — its name, else the fingerprint or the shape — as the bold line under it; with nothing loaded, the three start rows "Load a key", "Create a key", "Scan a wallet". Under the list a band of three small tiles, **Tools · Learn · Settings**, and at the bottom the two actions, **Add** and **Scan**, Scan primary. The list scrolls when it must; the band and the actions never move. There is no Keys, Wallets, Sign or Verify area: a key is reached through the wallets it signs for, and signing and checking are rows on a wallet's page. On `wide` the sidebar is Home · Tools · Learn · Settings, and the pane is the same list with the same actions; a tapped row's page replaces the list in the pane.
- **A wallet's page** has the same shape for every kind, and its title is what the wallet is called, which is the bold line of its row on Home. A named wallet carries a Name value row first (PLANNING §16.84; a coordinator config's `Name:` is the name on load); an unnamed one carries no Name row and instead a "Set a name" row after Export, which moves to the top as the value row once a name is typed. Then the actions, then Key or Keys, then Forget. A loaded key's wallet: Sign a transaction · Sign a message · Addresses · Check an address · Export · [Set a name] · Key · Forget. A policy this device holds a key of: Sign a transaction · Addresses · Check an address · Export · [Set a name] · Keys · Forget. A wallet it holds no key of: Addresses · Check an address · Export · [Set a name] · Keys · Forget. Sign a transaction opens the scanner with a PSBT expected; Check an address opens the scanner with an address expected, and the answer is searched over every wallet.
- **Add** is a menu of the three ways in and the one wallet made here: Load a key, Create a key, Scan a wallet, Build a multisig. Scan reads a SeedQR or a descriptor without going through Add.
- **The `wide` sidebar is on every screen.** During a wizard, the scanner, the lock screen, the four hold-to-confirm screens and the terminal screens it is drawn dimmed and takes no taps, so the content never shifts sideways between two screens of one flow, and navigation never shares a screen with a secret or a destructive step.
- **Status line**, 56 dp, on Home: the tier badge at the left, which names the tier and the kind of device it stands for (`C · desktop`) and opens the screen saying what tiers A–D mean; just after a wipe it also carries "1 key removed." for a few seconds, so an empty hub is never the only confirmation; the network badge beside it when the network is not mainnet; then, at the right, the lock — a lock icon, joined by a countdown inside the last minute before the auto-lock, which locks now when tapped. Nothing else: Settings is a tile, and the version is in Settings › About.
- **Scan** is the default screen for every input (PLANNING §16.88). Home carries it as an action and routes by payload type (table below); every other flow that takes a value opens the same scanner with an expectation set, and under the viewfinder the same band of tiles — "Read a file", "Paste", "Type" — takes the value another way. Paste goes through the router a scan uses; Type opens the keyboard the flow has. There is no floating or persistent scan control.
- **Back** always goes one step toward Home; Home is at most two taps from anywhere. Secret screens are re-gated on re-entry. Where nothing is behind — Home, the lock screen, the stored key's pad — one Back shows "Back again to exit and clear memory" on the caption line for two seconds, and a second Back inside that window clears memory, keeps a key kept on the device, and closes the app (the Android idiom; on desktop Escape is Back). "Wipe and exit" in Settings is the only exit that also forgets the device's copy.
- **Key context** is shown only where it matters: the Sign review steps carry the "Key" row naming the keys the transaction is about, and both Explore screens carry it with Explore's own "Which key?" chooser behind it. Nowhere else: a wallet's page is its own context. There is no global key bar, and the Sign result screen has no key row: its three ways out need the height.
- **Status**: the assurance tier, the network when it is not mainnet, the lock and its countdown — in the status line on Home (§5), and as a thin countdown strip over any flow that touches secrets.
- **Warnings** are ranked info / caution / danger / blocked; danger requires a distinct confirm gesture and never shares the tap that proceeds, and a block offers no acknowledgement — the review ends at the warning and the hold stays shut.

### Scan routing
| Payload | Goes to |
|---|---|
| SeedQR / CompactSeedQR | Load a key, at the confirm screen (entry, word language and the separate checksum step all skipped) |
| PSBT (base64 / BC-UR) | the Sign review |
| Address | checked against every wallet → the address Result |
| Descriptor / wallet config / xpub | the wallet review, ending in "Add this wallet" or the chevron; no key needed |
| BIP-39 words as plain text | caution, then Load a key; the words name their language, so the language step is asked only when more than one wordlist holds all of them |
| Signed message | the checked-message Result |
| Encrypted backup (`OSKB`) | the backup passphrase, then Load a key at its confirm screen |
| Descriptor, under Tools › Descriptor checksum | the checksum answer, or the Type entry with the text and the reason |
| Extended public key, under Tools › Convert key | every spelling of it, or the Type entry with the text and the reason |
| Any text, under Tools › Hashes or Encodings | the tool's answer, or the Type entry with the text and the reason |
| A policy, under Tools › Miniscript | the compiled descriptor, or the Type entry with the compiler's own words |
| Words in the clear, a seed code, an xprv or a WIF, pasted or under any tool | refused; a secret is never pasted (DESIGN §4.10) |
| Unknown | raw bytes, chunked hex, "treat as…" chooser: read as a transaction, read as text, hash it, show encodings |

---

## 5. Chrome: one app bar, one bottom action

Superseded by `docs/DESIGN.md` on 2026-09-09. The chrome (§4.1 there),
the choice idiom (§4.2), entry (§4.3), the secret panel and the eye
(§4.10), the hold (§4.13) and the one placement law (§2.6) are all in
that document, with the sixteen reusable screens in its §5. What this
section used to say that is still true is there in its current form; what
it said that changed (segmented rows, the panel captions, the countdown
beside the eye, six words per page on every class) is listed in DESIGN
§6.

## 6. Components (atomic design)

Superseded by `docs/DESIGN.md` §4 (one component per content kind) and
§3 (tokens, theme and type). The code follows the same split:
`core/osk-ui/src/tokens.rs`, `components/` and `screens/`.

## 7. Flows by area (jobs → frames → steps)

Each flow names the jobs it serves (§3), its frame, and its steps. Steps are indicative; wireframes decide the details.

### 7.1 First run and every start — A1–A4
- **Self-test** (document, unskippable on failure): vectors, RNG sanity. Failure names the failing vector and stops.
- **Tier notice** (document, first run; later under Settings › About): plain statement of the assurance tier and what is not stored.
- **Verify this software** (document): version, core hash, and offline instructions for checking a binary against the developers' signing keys or building from source; QR/URL to the canonical page.
- **Start here** (PLANNING §16.85): while nothing is loaded and `first_run_done` is unset — every start on Tier D — Home is a Document of four short sections (the words, the backup plan, what the device is, what happens next) with Continue opening Add and the chevron setting the flag. A key created then has no Skip on its quiz and lands on a "Key created" Result whose second action opens the key's Addresses; a key loaded sets the flag by itself. Settings › Start here reopens the page without touching the flag.
- **Try it**: a Learn page whose subject has a flow ends in one row that opens it — Create a key, Roll dice for a key, Verify a backup, Make an encrypted backup, Open a passphrase, Check an address, Build a multisig, Miniscript tool, Export, Sign a message — going to Add when the flow needs a key and none is loaded.

### 7.2 Keys — B1–B12, C1–C8, D1–D8

A key is reached through the wallet it signs for: its row on Home, then
the **Key** row of that page. Add on Home starts the two ways in.

- **Create**: source (dice / coins / hex / cards / camera / device / mix; PLANNING §16.83) → word count → language → entropy entry with live count and sanity checks → optional "show the math" → **words** (a secret panel, bullets until it is held or the eye opens it, with "Show numbers" beside them) → quiz (all words, random order; skippable with caution) → passphrase offer → fingerprint confirm → Home.
- **Load**: source (scan / type / encrypted backup / numbers / hex / from SE) → word count (skipped for SeedQR) → **Word language** (two rows: English, checked, and "Another language ›", which opens the full list where a language that is not baked yet is dimmed and says why; the tap that picks one moves on) → entry (BIP-39 keyboard: candidates from the first letter, impossible keys disabled; the screen shows the word being typed and nothing else, and a backspace on an empty field steps back to the word before it) → checksum result with most-likely-wrong-word hint → passphrase (optional; typed once with with/without fingerprints shown to recognise, or typed twice) → fingerprint confirm → Home.
- **Key detail** is a **menu**, not a page: an app bar titled `Key 73c5da0a`, the network and backup badges, then one 72 dp row per screen — **Backup · Open passphrase · Open BIP-85 child seed · Keep on this device**. Each row opens a screen of its own. Every row is about the secret; what the key derives in public — addresses and a descriptor — is its wallet, the page this menu was opened from (§7.2a). Forget is that page's last row, not this menu's.
  - **Backup**: rows for Show words · SeedQR · CompactSeedQR · Encrypted backup · Draw a SeedQR · Numbers for steel · Split with Seed XOR · Verify backup (PLANNING §16.82 for the last three but one). Draw a SeedQR is the code as a hand-copyable grid with guide lines every five modules and row and column labels, paged by quadrant on `small`; Numbers for steel is the Words screen as `0001 · ABAN · abandon` rows, with "Print template" on `wide` writing a blank numbered table; Split with Seed XOR is 2, 3 or 4 parts, where the random parts come from a source the person chooses on a "Random parts from?" Choice with Create a key's own rows and is entered on Create a key's own screens once per random part (PLANNING §16.92), the parts then shown in turn as Words screens, then a Result that names the source the random parts came from and each part's fingerprint; Load a key › "Seed XOR parts" combines them again, because what combining gives back is a key that already existed. Encrypted backup (PLANNING §16.78) asks a passphrase twice, eight characters or more, and lands on a Result — the key, what is inside (this key's words, or the parent's for a passphrase key), the size — with "Save to file" and "Show as QR", one static code. It is read back by Load a key › Encrypted backup, or by Scan, which asks the passphrase and lands the Load wizard at its confirm step. Each of the three is one secret panel: the words page by what the panel holds, as bullets until the panel is held or the app bar's eye opens it, with "Show numbers" putting each word's place in the BIP-39 list beside it under the same mask; each seed code blank until it is held or opened. Later: SLIP-39.
  - **Open passphrase**: the passphrase over the words this key already holds, typed on its own Entry screen, with the fingerprint of the key it opens above the field as it is typed. The words are not typed again. ✓ lands on a Result that states which key was added, with "Open key" to its menu. The key that comes out carries the same words, has its own fingerprint, and lives in memory for the session: it is never written to a device that keeps keys. Shown only for a key that holds its words.
  - **Open BIP-85 child seed**: a word count (12, 18 or 24) and an index, the child's fingerprint above the index field as it is typed, and the child mnemonic of `m/83696968'/39'/0'/{words}'/{index}'` is loaded as a key of its own, on the same Result as the passphrase key. Like the passphrase key, it lives in memory for the session and is never written to the device.
  - **Forget** (from the wallet's page): the fingerprint, what leaves with it — the wallet and its addresses — and, where a policy in use has this key, that the policy stays but can no longer sign; hold to confirm.
- **Backup quiz**: all words, random order, each chosen from candidates; available any time for any loaded key. A wrong answer never names the right word, in either mode: it says the answer did not match and offers "Show the words", which walks the words and returns to the same question with fresh candidates. The question number does not move until the answer is right. The "helper holds, owner answers" variant (K1) changes nothing else; the screen carries a "Helper mode" line the whole time it runs, so an auditor can see which mode is on.
- **Wipe all** (danger): Settings › Wipe all keys is a row; the screen behind it says what leaves memory and what does not, and holds the button. The hold lands on Home, where the status line says "1 key removed." for a few seconds.

### 7.2a Wallets — E1–E4

A **wallet** is public data that turns keys into addresses: one key at a
script type (single-sig) or a policy over several (multisig, MuSig2).
Home is the one place they are listed, and a wallet's page is the one
place anything is done with it.

- **The list** is Home (§4): one row per loaded key — its own single-sig wallet, `single-sig · mainnet` over `73c5da0a` — then one row per policy in use, `multisig · mainnet` over `2 of 3 · SegWit`. The bold line under the label is what the wallet is called, which is its name once it has one. Every row carries §4.4's glyph: the key when this device holds a private key of that wallet, the eye when it holds only public keys.
- **A loaded key's wallet page**, titled `73c5da0a`: **Sign a transaction · Sign a message · Addresses · Check an address · Export · Set a name · Key · Forget**, with the Name value row first and no "Set a name" row once it is named.
  - **Sign a transaction**: the scanner with a PSBT expected, and "Read a file" on it; the review that follows is §7.3's.
  - **Sign a message**: the scanner with any text expected; §7.3.
  - **Addresses**: the script type as a value row, a `Receive | Change` pair, then one row per address, each opening the address whole with its code. One at a time is what people compare against a coordinator.
  - **Check an address**: Scan a QR or Type an address; the answer is searched over every wallet, and says which wallet and which index (§7.4).
  - **Export**: the format as a value row — descriptor, account key, SLIP-132 key — over the script type, then the string as a reference row and the code one row further on.
  - **Key**: the key's own menu (§7.2).
  - **Forget**: the key's forget hold (§7.2). The wallet follows its key.
- **A policy's page**, titled with what the wallet is (`2 of 3 · SegWit`): **Sign a transaction · Addresses · Check an address · Export · Keys · Forget**, and without Sign a transaction when this device holds none of its keys. Addresses is the same list, derived from the policy, with the script type as a fact rather than a choice. Export offers the descriptor with its checksum and the BIP-388 policy, as text and as a code. Keys opens the review the wallet was accepted from — quorum, checksum, script type, every cosigner with the key or the eye — and a cosigner this device holds opens that key's menu. Forgetting a policy is a tap, not a hold: a wallet is public data, and what goes is a row.
- **Scan a wallet**: a row of Add, present whether or not a key is loaded. It opens the scanner, which reads a code or a file; a BIP-388 policy, a multipath descriptor, a `tr(musig(…)/**)` MuSig2 wallet in either of those two forms, the `Name:`/`Policy:`/`Derivation:`/`Format:` text file coordinators export, and Coldcard's JSON account export all land on the same review, as does any of them read by Scan on Home. A MuSig2 wallet has no quorum to state, so its review's first row and its page's title are `MuSig2 · 2 keys`. "Add this wallet" on the review lands on the wallet's own page, with Home behind it; the chevron leaves the review as a review.
- **Build a multisig**: a row of Add, the builder (PLANNING §16.76). **Which script type?** — Legacy, Nested (`sh(wsh(sortedmulti))`, BIP-48 `1'`), SegWit (`wsh(sortedmulti)`, keys at BIP-48 `2'`), Taproot, MuSig2 (`tr(musig())`, keys at BIP-86, every key signs), in that order; Legacy and Taproot dimmed. **Keys · N** — one row per key gathered, the key glyph for this device's own, the eye for a scanned cosigner, the account path as the value, and "Add a key" as the last row, so the empty screen is that one row and Continue stands alone at the bottom; a key's row opens it with Remove; "Add a key" opens **Which key?**, a menu of the loaded keys not yet in with "Scan a key" last, where a tap is the answer; that scanner reads a key with its origin, a single-key descriptor, a Coldcard export's multisig account, or a bare xpub, and refuses a wrong-network, duplicate or already-a-wallet text with a reason. **How many must sign?** — 1 to N, skipped for MuSig2. **New wallet** — the same review a loaded wallet gets, without the checksum row and the count of keys, since nothing was given to this device to check, and ending in "Add this wallet"; a wallet equal to one in use opens that one. Back with keys gathered asks first.
- **A miniscript or Taproot tree wallet** (PLANNING §16.80): `wsh(<miniscript>)`, `sh(wsh(<miniscript>))` or `tr(<key>, <tree>)` loads like any other. Its review's first row is the kind, then one **Spend path** row per way it can be spent — "Key A", "Keys B and C after 52 560 blocks · about 1 year" — with the keys lettered in the order of the key rows below. Its row on Home is "Miniscript · SegWit" or "Taproot tree" over the checksum. A key with one chain only is refused. Tools › **Miniscript** compiles a typed policy into a descriptor, lists its paths, and offers "Load as wallet".
- **A wallet over a public key alone**: a single-key descriptor (`wpkh([73c5da0a/84'/0'/0']xpub…/<0;1>/*)`) is a wallet like any other and lands on the same review. A bare extended public key says nothing about what it pays to, so a `Which script type?` Choice asks first: the check starts on the type a SLIP-132 prefix names (`zpub` → SegWit, `ypub` → Nested), and on SegWit for an `xpub` or `tpub`, which are BIP-32's own prefixes and name none. The chevron leaves the key as the document it was. Such a wallet is a row on Home, labelled by the master fingerprint its key's origin names, or by the script type where the key arrived without an origin; its review's key row then says `origin unknown` where a fingerprint would stand, and its Export offers the descriptor but no BIP-388 policy, since BIP-388 writes an origin on every key. Its page has no Sign rows. Nothing on the row or the page says "read-only": the eye says it.
- **Wallets stay across restarts on Tier B**: on a device that keeps keys, the wallets in use are kept with them, as public data under the same PIN, and are listed again when the PIN opens the keys. Forgetting a wallet forgets it on the device too, and forgetting the last key takes the wallets with it. On a stateless tier the wallets are the session's, as the keys are.

### 7.3 Sign — F1–F9

There is no Sign area. A transaction arrives by Scan on Home, or by
"Sign a transaction" on a wallet's page, which opens the same scanner
with a PSBT expected; both land on the review below.

- **Sign PSBT**: the network badge is on every review screen whenever the network is not mainnet. **Summary** (amount out, recipients, fee absolute + rate, change verified / **not verified** in words as well as colour, which loaded keys participate; none → danger and stop) → **Outputs** (each full address chunked, amount, change marked only when derived from our key/config) → **Inputs & warnings** (count, ours, sighash, RBF, locktime; ranked warnings: unverified change, high fee, dust, odd sighash, mixed scripts, network mismatch, address reuse) → **Confirm** (recap line, choose which of my keys sign, hold to sign) → **Result**: every signature verified, per-input nonce status ("nonce OK" or "anti-exfil — host verifies"), output as QR/UR/SD/file or finalised hex; optional signature-bytes export for cross-implementation comparison.
- **Miniscript and Taproot tree inputs** (PLANNING §16.81) are signed like any other: every signature this device's keys can give — a partial signature per key in a `wsh` script, a leaf signature per Taproot leaf naming one of its keys, and the key path when the internal key is its — and the finalizer completes the input only when the path is satisfied; a timelock the transaction does not meet is a caution, "Spend path · not met", and the input row names the path the transaction spends.
- **Sign message** ("Sign a message" on a loaded key's wallet page): message review → key → hold → signature.
- **MuSig2 session** (v2): round 1 nonces → persistent "session open — do not power off" banner → round 2 partial signature.

### 7.4 Verify — D5, D6, E2–E4, G1–G7, K1, K2

There is no Verify area. Each question is answered where its object is:

- **Address**: "Check an address" on any wallet's page — Scan a QR or Type an address — or an address read by Scan on Home. The result is "Yours — `ABCD 1234`, receive #7", naming the wallet, or "not in first 100 → search further"; the search runs over every wallet.
- **Verify a message**: Tools › Verify a message, or a signed message read by Scan: address + message + signature → valid/invalid. No key needed.
- **Wallet**: a descriptor or config read by Scan, or Add › Scan a wallet, lands on the review — quorum, cosigner fingerprints with the key or the eye, script type, descriptor checksum. When a wallet in use has the same shape and exactly one key differs, the review carries a danger card naming the wallet in use and the changed key — "same fingerprint, different key" when only the xpub changed, which is the swap attack — the changed row is in the caution tone, and "Add this wallet" becomes a hold. A single key claiming one of this device's fingerprints with a key that master does not derive gets the same card. The chevron leaves it a review; "Add this wallet" keeps it.
- **Software** → Settings › About: the version, the core hash and how to check it.
- **Decode** (later, a tool): raw tx or PSBT → structure; no key needed.
- **Backup** → Key › Backup › Verify backup, in helper mode.

### 7.5 Tools — H1–H9

Tools are standalone calculators, none of which needs a key: the **key
explorer**, the **word list**, the **dice passphrase**, **Verify a
message** (§7.4), **Hashes** (SHA-256, SHA-256d, HASH160 of text or
hex, the mode row forcing one reading), **Encodings** (which alphabet a
string is in, its bytes and checksum, and from hex the other
spellings), **Descriptor checksum**, **Convert key** (a public extended
key in its BIP-32 and four SLIP-132 spellings, with its depth,
fingerprint and child number; private keys refused), **Units** (a digit
pad with a "From" mode row; the other three units live beside it on
`mobile` and `wide`, and on ✓ as a Record everywhere) and **Decode a
transaction** (the Sign review over a PSBT or a raw transaction with
no Confirm step and Done at the end; this device's wallets are
recognised when any are loaded, and with none the change row says "no
key loaded"). What each is for is the Learn page "Tools".

**Key explorer**: two screens and the path editor, over the active key or over words typed with no key loaded. Both screens carry the "Using" chip (§4); what the path, the checksum and the encodings are is a Learn page.

- **Keys** (the screen Explore opens on): the applied path as a **breadcrumb** whose crumbs are tappable and whose current level is boxed (`m / 84h / 0h / 0h / 0 / 0`), the BIP purpose presets under it, a row into the path editor, a "Passphrase: none / set" row for typed words, then the fingerprint, the extended public key and the extended private key of the level that is boxed, then every encoding of the account level together — the account key and its SLIP-132 form, with "The prefix names the script type. The key is the same." — and the rows out: Addresses — the address screen of whatever is being explored, a loaded key or the typed words, since typed words are a key — and Words and bits. On `wide` the two halves are two columns; on `small` it is a document with a scrollbar.
- **Words and bits**: the words table, 1-based like every other screen, with the app bar's eye and a pager when it takes two pages; the checksum arithmetic with the verdict first and large, then the entropy, the first byte of its hash and the bits it gives, then the sum; and the seed with the master private key under the formula that makes them.
- **Path editor**: the field, its error line (always there, so nothing moves when a path stops parsing), the purpose presets, a "receive /0/0" shortcut that is never drawn on the accent, and the path keyboard whose ✓ is dead while the text does not parse.

Leaving with typed words asks first: "Keep exploring" or "Discard words".

**Word list**: the BIP-39 list of any of the ten languages, by word, by
number, by binary or by hex. No key is involved and nothing is secret, so
no screen here carries the eye.

- **Which language?** the same Choice the Load wizard uses, English checked.
- **Search**: an Entry whose field is read by the notation the "Search by"
  mode row names. **Word** is the language's own keyboard and candidate
  strip, as in Load, and a candidate tapped opens the word; **Number** is
  the digit pad, 1 to 2048, the way the app numbers words everywhere;
  **Binary** is a two-key pad of 0 and 1, eleven bits; **Hex** is the hex
  keyboard, `000` to `7ff`. A value the list has no word for says why on
  the caption line and leaves ✓ dead. The mode row's Choice carries a
  fifth row, **Browse the list**, which opens the list at word 1.
- **One word**: a Record titled with the word — Number (from 1), Index
  (from 0, which is what BIP-39 and every other tool counts by), Binary
  (eleven bits), Hex (three digits), Language — with a pager labelled with
  the word in the bottom slot, so the list can be walked from any word.

**Dice passphrase**: a passphrase rolled from an EFF diceware list (§8.1
#11, job B12). Nothing is stored, and leaving zeroizes the rolls.

- **Which list?** Long list (7776 words, five dice), Short list (1296
  words, four dice), Short list 2 (1296 words, three edits apart).
- **How many words?** 4 · 5 · 6 · 7 · 8 · 10, each row stating what it is
  worth: "6 words · 77 bits".
- **Rolls**: the Create wizard's dice pad, asking for words × dice rolls,
  with the same count, the same masked run and the same sanity checks. As
  each group of rolls completes, its word joins a masked panel above the
  pad, so the phrase is seen to build.
- **Passphrase**: a Secret screen — the words on the panel, masked until
  the eye, "Entropy · 77 bits" under it, and Done. Back with any roll
  entered asks first: "Discard" or "Keep".

### 7.6 Learn — L1–L18
**Reading** frame: sequential chapters L1–L15, plain language, each ending in "try it" which deep-links to the real flow on signet with a throwaway key; L16 is the guided end-to-end practice run; glossary (L17, same content as the one-tap explainers used throughout); "I want to… which job?" (L18). Content lives in structured per-language files, not in code, so it can be translated and its glyphs baked (PLANNING §4.3).

### 7.7 Session and settings — I1–I5
Auto-lock and auto-wipe timers (visible), unlock via PIN or SE, explicit "Wipe and exit" (hold, then a terminal "Session ended" screen with nothing to press), settings kept few with a one-line "why" each; Tier / About / hash / self-test rerun / factory wipe under Settings.

**The lock screen** says how many keys are sealed ("1 key sealed") and not which ones: a locked device tells whoever picks it up nothing about the keys on it. It uses the key glyph, not a biometric one, states the five-wrong-PINs rule before the first attempt, and fills a desktop window with the navigation rail beside it dimmed.

**PIN dots have one placement:** a field directly above the pad, on the lock screen and on both session-PIN steps. The set-PIN step carries one line explaining the shuffled pad while the shuffle is on, the repeat step shuffles differently from the first, and a repeat that did not match restarts the entry and says so **under the field**, in the danger tone, rather than in the title.

---

## 8. Wireframe plan

No HTML wireframes: the real renderer is the wireframe (PLANNING.md §13.1). The snapshot shell writes each screen at the four reference sizes (240×320, 480×640, 1080×2340, 960×640), and the desktop shell shows the same screens at its own fixed size or, with `--size`/`--dpi`, at any of the others. Order: Home + scanner → Load → Sign → Check an address → Create (dice) → Wallet page + key menu + backup → Wallet export/config → Explore → Learn → Settings. Components in §6 are revised as the wireframes are drawn; the jobs in §3 are the acceptance criteria.
