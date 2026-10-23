# Codex32: making, checking and repairing shares

**Kind:** Spec
**Status:** Specified (owner, 2026-10-22, with the owner's answers of the same day in §1.1, none left open), not built; its four decisions are proposed in §14, not yet recorded
**Open:** all of it, batches C1 to C10 (§15.2); a Codex32QR profile for 74- and 127-character strings, which waits on that profile's author (§10.4)

The owner's ask: full codex32 support for making shares, as the codex32
booklet does it, with every step shown so that a person can follow it
on paper. This document says what Faraday does with codex32 (BIP 93)
once it is built: how a person's own entropy becomes shares, how each
worksheet the booklet has is shown, how a string is checked and, when
the person asks for it, repaired, how codex32 sits in backups and blank
templates, and how it reads and shows the Codex32QR profile.

Sources read for it, each as data and none vendored:

| Source | Where | Licence | Read |
|---|---|---|---|
| BIP 93 | `tools/vectors/bip93/bip-0093.mediawiki` (2026-09-18), and the current text on GitHub, identical in every section cited here | BSD-3-Clause | 2026-10-22 |
| The codex32 booklet, black and white, revision of 2023-03-07 | https://secretcodex32.com/docs/2023-03-07--bw.ps (PostScript source; the booklet's source repository is https://github.com/roconnor-blockstream/SSS32) | MIT, "Copyright (c) 2022 Blockstream", stated inside the PostScript; the repository has no LICENSE file and GitHub detects none | 2026-10-22 |
| The booklet's single worksheets | https://secretcodex32.com/docs/index.html: checksum worksheets for 128 and 256 bits, translation worksheets for 128 bits at k = 2 and k = 3 | as the booklet | 2026-10-22 |
| BlockstreamResearch/codex32 | https://github.com/BlockstreamResearch/codex32 (an older `SSS32.ps`, revision alpha-4.6, MIT, "Copyright (c) 2020 Blockstream"; pull request 70, a Haskell implementation with error correction, open, head `610cbad30258c80cd862b3773a20f8099d25e36e`) | MIT | 2026-10-22 |
| The interactive checksum worksheet | https://github.com/apoelstra/volvelle-website, `volvelle-wasm/src/checksum_worksheet.rs` at `9096b331733d2b6bfb22ae9b545c84261e2e0c72` | CC0 | 2026-10-22 |
| python-codex32 | https://github.com/BenWestgate/python-codex32 at `f19d4627c5725a9f6cfa995a1f89fe62e1becb4c`: `src/codex32/correction.py` and the frozen correction corpus `tests/data/p70_correction_vectors.json` (57 cases: 29 of 48 characters, 28 of 127), drawn from pull request 70 above | MIT | 2026-10-22 |
| The Codex32QR profile and SeedSigner's codex32 error correction | https://github.com/FractalEncrypt/FractalEncrypt_seedsigner, branch `codex/codex32-error-correction`, folder docs/codex32_qr: Codex32QR_Spec.md, Codex32_Error_Correction.md, Codex32_User_Flow_Guide.md, test_vectors/, printable_templates/ | MIT (the fork of SeedSigner) | 2026-10-22 |

## 1. The owner's decisions (2026-10-22)

1. **Full codex32 support for making shares**, as the booklet does.
2. **Any split, everywhere**: a threshold k of 2 to 9, or none (k = 0,
   the secret alone), and up to 31 shares, in Add a key, in a backup's
   copies by hand (the two fixed chips become one codex32 choice that
   asks k and n) and in the backup plan's form choices.
3. **The main flow**: the person puts in their entropy, chooses the
   scheme, and follows every step to the end, as the dice flow for
   BIP-39 words shows all the entropy. It lives in Add a key beside Make
   SLIP-39 shares (§6).
4. **Lengths**: 48 characters (128 bits) is the default; 74 (256 bits)
   is offered and computed on the device with every row shown; 127 (512
   bits, BIP 93's long checksum) only where it cannot be avoided, a
   codex32 backup of a key made of words (§4).
5. **What the person can check**: the entropy's characters stand
   unchanged in the data of S (k = 0) or of each initial share; every
   checksum row is shown; after any generation, a step to check on
   another implementation (§6.8).
6. **Shares made by hand, typed in**: each checksum checked, the first
   failure named, the rest derived step by step; the secret is never
   typed whole (§8.1).
7. **Add a share later** from any k strings of a set (§8.2).
8. **The identifier**: from the fingerprint by default, or typed; one
   already in use is refused. Narrowed by §1.1 item 2: typed in Make
   codex32 shares, the fingerprint where a seed exists (§6.2, §6.4.5).
9. **Check a string by hand** without loading it (§8.3).
10. **Error correction as a tick**, off by default, after SeedSigner's
    design (§8.4). A new decision (§14).
11. **Codex32QR/v1-48**: read, tested against its published vectors and
    pictures, and shown wherever one 48-character string is shown; no
    profile is made up for longer strings (§10).
12. **Templates**: the Starter's Blank templates gain codex32 sheets and,
    licence allowing, the booklet's 48-character worksheets (§11).
13. **Randomness and the key boundary**: Faraday's own randomness goes
    through faraday-keys (F70); the person's entropy uses the dice pad
    and the mask policy (§5, §13.1).
14. **Demo**: what each flow does in Demo (§12).

### 1.1 The owner's answers (2026-10-22)

1. **Masking of the live sheets**: none on Faraday Desktop, which is
   testnet only for real seeds. On the stick and the Pi, unmasked while
   the flow is open, masked when it is left, the eye at any time
   (§6.4.6).
2. **The identifier in Make codex32 shares** is typed by the person,
   before any entropy, as the booklet does and as the Codex32QR
   profile's author describes it (a self-chosen four-character label;
   his examples `L0VE`, `WSFP`, `F0UR`), so the checksum worksheet fills
   live. A suggestion may be offered; the default is typed. The
   identifier from the fingerprint stays where a seed already exists:
   the copies by hand of a key held and the plan (§9).
3. **A share index already in**: as the Codex32QR profile, the same
   string again is ignored, a different one asks before replacing
   (§8.5).
4. **Demo**: the sample only, but the process runs: the person rolls and
   watches the whole live process (the sheets, the conversion, the
   checksum); once that screen is finished, the result is replaced by
   the sample (BIP 93 vector 2), with a line saying so, as Create a
   vault's Generate with dice puts `faraday` in place of the rolled
   words (DEMO §5.2) (§12).
5. **k and n**: the full range. The Scheme card offers "2 of 3"
   (ticked) · "3 of 5" · "Other", which opens a threshold stepper 2 to 9
   and a strings stepper k to 31; "None, the secret alone" (k = 0) stays.
   Where k is 4 or more, one line of fact states that the booklet
   publishes no translation sheet for it, so the derived shares are the
   device's arithmetic; the worksheets are still shown live (§6.1).
6. **The booklet's MIT notice**: kept in the repository and on the
   codex32 Learn page; not printed on the sheets (§11).
7. **Templates**: the published 256-bit checksum sheet is printed; no
   translation sheet the booklet has not published is drawn (§4, §11).
8. **Error correction** at every length (§8.4).
9. **Dice**: six-faced only (§5.1).
10. **Seed bytes**: `faraday-keys` keeps a master seed's bytes for keys
    loaded from one (§9.1).
11. **127 characters** are not offered in Make codex32 shares (§4, §6.1).
12. **A fingerprint box** on the printed share sheet (§11).
13. **"Identifier already in use"** also checks the open vaults (§6.4.5).
15. **The review after entry**: tabs, one per worksheet in the booklet's
    order, each scrolling within itself and checked once shown; on the
    small panel a Choice with Previous and Next (§6.5).
14. **Desktop dice rollers** (Faraday Desktop only; the stick and the Pi
    keep real dice typed on the pad): five dice for codex32, one die for
    BIP-39 words and Generate with dice, their values this computer's
    randomness and labelled so (§5.5, §5.6). The entry for real dice
    stays beside the roller on the desktop, so the person chooses: their
    own rolls, Faraday's, or both; codex32's entry for real rolls is laid
    out as the de-biasing worksheet, five dice by two rolls (§5.5).

What this spec found that the decision assumed otherwise: the booklet's
site lists "Checksum Worksheets: Standard 128-bit, Standard 256-bit" and
"Translation Worksheets: 128-bit k = 2, k = 3; TODO higher k; 256 bits".
So a 74-character string can be checksummed by hand on a published
sheet; translation (deriving a share, recovering S) by hand has a
published sheet only for 48 characters at k = 2 and k = 3 (§4).

Added the same day (owner, 2026-10-22): "Are we showing the sheet on
screen and filling in the information from the dice rolls in real time,
as you would on paper by hand? All of the math has to be totally
transparent on screen." The sheets are on screen during entry and fill
as each roll and each character arrives; nothing appears without its
working, and every computed cell opens to what made it (§6.4, §7.4).

## 2. What is built today

- **The codec**, `core/faraday-bip/src/codex32.rs`: `Codex32::parse`
  (every rule of BIP 93; one case, mixed case refused), `from_seed`,
  `split(threshold, shares, fresh)` (an existing secret and k − 1
  random shares, then the rest interpolated), `derive_share`, `recover`,
  `to_seed` (padding bits ignored, as BIP 93 says), `identifier_for`
  (the first 20 bits of the master fingerprint), `symbols_for` and
  `seed_symbols`. It detects errors and corrects none. Tested in
  `core/faraday-bip/tests/all/codex32.rs` against every BIP 93 vector
  (`tools/vectors/bip93/vectors.json`). There is no function that makes
  a fresh secret from k initial shares (`recover` over them does it),
  and none that exposes a checksum's intermediate rows.
- **Add a key › Type a codex32 secret** (`faraday/faraday-core/src/forms.rs`):
  one string at a time; a second string with another identifier is
  refused (`forms_that_string_is`), and one whose share index is already
  in is refused (`forms_that_share_is`), identical or not.
- **Scanning**: in the codex32 form, Scan reads a string and adds it as
  typed (`ScanPurpose::KeyPart`, `faraday/faraday-core/src/lib.rs`); Home's
  Scan and the intake scanner send a string to that form
  (`intake_key_part`, `faraday/faraday-core/src/family/mod.rs`); a file
  of one string is a seed part in Files (`part_of`,
  `faraday/faraday-intake/src/lib.rs`; `core/faraday-codec/src/classify.rs`).
  Upper case is read in all three, so a Codex32QR code is read today,
  though no test scans one. Add a key's **Scan a SeedQR** does not take
  one: it says the code is not a SeedQR. `pasted_secret`
  (`faraday/faraday-core/src/scan_key.rs`) refuses a pasted string where
  secrets are not taken.
- **Lower case before the check**: the form, `part_of` and
  `intake_key_part` lowercase the text before `Codex32::parse`, so a
  mixed-case string from a scan or a file is taken although BIP 93 says
  it must be refused. §8.5 fixes this.
- **A backup's Other paper forms** (`paper_section`,
  `faraday/faraday-core/src/screens/backup.rs`; `paper_codex32`,
  `faraday/faraday-core/src/paper.rs`; `Keys::codex32_shares`,
  `faraday/faraday-keys/src/operations.rs`): two chips, "codex32, 2 of 3"
  and "codex32, 3 of 5", for a key with words only; the 64-byte BIP-39
  seed as 127-character shares, from the system's randomness spread by
  `faraday-keys`, behind `Reveal::PaperForm`. The strings are drawn as a
  numbered list in the section. DESIGN §5 describes a Secret panel per
  string and a "Strings made" Result; neither is built (no string for
  them is in `faraday/faraday-strings/src/en.rs`).
- **The backup plan** (`faraday/faraday-core/src/plan/mod.rs`) has no
  codex32 form: a seed's paper forms are `What::Words` and
  `What::SeedQr`, and the checklist has no codex32 item, although
  `docs/DEMO.md` §4 and §5.3 name one.
- **A key loaded from a master seed** (codex32, SLIP-39, an `.oskb`
  kind 2) keeps its master key and not the seed bytes
  (`Keys::add_seed`, `faraday/faraday-keys/src/session.rs`), so no paper
  form of it can be written again.
- **Demo** refuses a split with k of 2 or more (`faraday/faraday-core/src/demo.rs`,
  `docs/DEMO.md` §4 and §6) and has the BIP 93 samples
  (`docs/MODES.md` §7).
- **The dice flow** for BIP-39 words (`faraday/faraday-core/src/keygen/mod.rs`,
  `core/faraday-entropy/src/lib.rs`) shows every entry and, for
  Dice that name words, every word as it is made.
- **Blank templates** (`docs/STARTER.md` §3.8,
  `faraday/faraday-starter/src/template.rs`, `faraday/faraday-core/src/pdf.rs`)
  print BIP-39 sheets only.

## 3. Words

- **String**: one codex32 string, `ms1` included. **Box n** is its n-th
  character counted from 1: `m` is box 1, `s` box 2, `1` box 3, the
  threshold box 4, the identifier boxes 5 to 8, the share index box 9,
  the data from box 10, the checksum the last 13 boxes (15 at 127
  characters). This is the numbering of the booklet's checksum worksheet
  (each box's upper-right number, from the `ladder` code in SSS32.ps:
  `n = i + 1` for the 0-based position `i` in the whole string) and of
  the Codex32QR profile author's cards and device ("boxes 4 and 9" for
  the threshold and the index). The booklet's second label, a letter in
  each box's lower-left corner, is not used.
- **Groups of four** start at box 1: `ms12` `name` `a320` …, as the
  cards print them and as `faraday_ui::grouped` groups any string.
- **S**: the secret, the string whose share index is `s`.
- **Initial shares**: the k strings whose data is the entropy, indices
  `a`, `c`, `d`, … in BIP 93's order. **Derived shares**: the others,
  made from the initial shares.
- **Data**: a string's characters between the header and the checksum;
  BIP 93's payload.
- **Checksum worksheet**, **translation worksheet**, **translation
  symbol**, **recovery symbol**, **fusion**: as the booklet uses them
  (§7).
- **Set**: the strings that share a threshold, an identifier and a
  length.

## 4. Lengths

| Characters | Seed | Data | Checksum | By hand, with the booklet's sheets | In Faraday |
|---|---|---|---|---|---|
| 48 | 16 bytes, 128 bits | 26 | 13 | the dice sheet; the Standard 128-bit checksum sheet; translation sheets for k = 2 and k = 3 only (the site lists higher k as to do; the booklet prints derivation tables to k = 8 but no translation sheet for them) | **the default** for every flow that makes a seed |
| 74 | 32 bytes, 256 bits | 52 | 13 | the dice sheet; the Standard 256-bit checksum sheet; no translation sheet (the site lists 256 bits as to do) | offered beside 48; computed here with every row shown |
| 127 | 64 bytes, 512 bits | 103 | 15 (the long checksum) | none: no sheet exists, and none is designed here | only for a codex32 backup of a key made of BIP-39 words, whose seed is 64 bytes; computed here; its Result says "Not for checking by hand" |

Where no published sheet exists (translation at 74 characters, or at
k of 4 or more), Faraday's on-screen worksheets are the only worked form;
the arithmetic is the same per character, so a person can still check
any cell by hand with the wheels or tables (§7.4), but there is no
printed sheet to follow, and Faraday prints none (owner, §1.1 item 7).

The 54-, 61- and 67-character lengths (160, 192 and 224 bits) are read,
as today, and never made.

A worksheet for 127 characters is out of scope: the long checksum's
generator needs its own lookup table, which nobody has published or
tested on paper. Faraday shows a 127-character string's checksum rows
(§7.1) all the same, since they cost nothing to show, but makes no claim
that a person can work them.

## 5. Entropy

The flow takes the person's entropy in one of three ways, chosen on its
Entropy card (§6.3). Faraday's own randomness is never used to make a
fresh seed in this flow, with one exception the owner made: Faraday
Desktop's dice roller (§5.5), whose values are this computer's
randomness, labelled as such wherever they appear.

### 5.1 Dice, by the booklet's de-biasing worksheet

The booklet's **Dice De-biasing Worksheet** (section "Dice De-biasing
Worksheet", SSS32.ps of 2023-03-07, pages 18 and 19 of its source)
turns ten rolls of five dice into one character, without bias from an
uneven die. Faraday reads the same rolls into the same character, so a
person with the paper sheet and a person with Faraday get one result:

1. Five distinct dice, die 1 to die 5, each with its own track.
2. Roll all five; each die's first value is noted (the marker on its
   track).
3. Roll the same five again; each die's second value is noted.
4. A die whose two values are equal is rolled twice again, both rolls;
   the booklet: "You must redo both rolls!"
5. Each die gives one bit: **0 when its second value is lower than its
   first** (the die lies left of its marker), **1 when it is higher**.
6. Die 1's bit is the most significant: the character is the bech32
   character whose value is `16·b1 + 8·b2 + 4·b3 + 2·b4 + b5`, the
   booklet's tree, whose leaves read left to right
   `Q P Z R Y 9 X 8 G F 2 T V D W 0 S 3 J N 5 4 K H C E 6 M U A 7 L`
   (the `code` array of SSS32.ps), the bech32 alphabet in value order.

On the device the worksheet itself is on screen and fills as the
rolls are entered (§6.4.2): each die's first value, its second, the
comparison and the bit; then the five bits, the path through the tree,
and the character dropping into its numbered box. The entries are taken
in the booklet's order, die 1 to die 5 for the first roll and then for
the second, into an entry laid out as the worksheet's five dice and two
rolls (§5.5, "Your dice"), on every shell. How the sheet is masked is
§6.4.6.

The booklet's sheet also has pads for dice of 7 to 20 faces. Faraday
takes six-faced dice only (owner, §1.1 item 9).

`tools/reference/dice/README.md` is the model for the record of this
method: a new page under `tools/reference/` (C3) quotes the worksheet's
instructions and the tree's leaf order from SSS32.ps with the file's
SHA-256, so a later change to the booklet is seen as a change there.

### 5.2 Characters typed

The person types the characters on the codex32 keyboard (DESIGN §4.3),
for a person who made them on paper by any method. Each character goes
into its box as it is typed, masked as the dice characters are. Nothing
checks them for randomness; Faraday shows none of its randomness
statistics here, since a character typed carries no roll to count.

### 5.3 Shares made by hand

The person types k whole initial shares made on paper, checksums
included (§8.1).

### 5.4 Faraday's randomness

Only where a seed that exists is split: the copies by hand of a key
held (§9.1) and the plan's codex32 form (§9.2), where BIP 93's
"For an existing secret" needs k − 1 random shares. The bytes come from
`Faraday::fresh` and are spread inside `faraday-keys` as
`Keys::codex32_shares` does today; nothing leaves `faraday-keys` but the
strings, through `Reveal::PaperForm`.

### 5.5 Faraday Desktop's dice roller (codex32)

On Faraday Desktop only, the dice step shows five small dice in a row,
numbered 1 to 5, the de-biasing worksheet's five tracks. They are drawn
from the website's die pictures (`die-1` to `die-6` and the `die-pose-`
frames, WebP), copied into Faraday's assets; nothing else of the
website comes with them. Beside them, a choice of what one click rolls:

- **One pass**: the five dice once (the first or the second values);
- **One character** (the default): both passes; a die whose two values
  are equal is rolled twice again, and the struck-through pair stays on
  the sheet as §6.4.2 draws it;
- **One share**: every character of the share being entered;
- **To the end**: every character of every initial share.

Each click tumbles the dice, an animation modelled on the website's
toss motion, and the values land on the sheet in turn, each character
with a short pause so its conversion can be followed (§6.4.2); under
Reduce motion (MOTION) there is no tumble and no pause.

**Both ways, side by side (owner, 2026-10-22).** The roller never
replaces the entry for real dice: on Faraday Desktop the dice step
shows the stick's entry, **Your dice**, and beside it the roller,
**Or let Faraday roll**, and the person uses either, or both, at any
point. A value from the entry is a real die's and is not labelled.

**Your dice, laid out as the worksheet** (every shell, the stick and the
Pi included). The entry for real rolls has the shape of the booklet's
Dice De-biasing Worksheet rather than a single line: five fields across,
**Die 1** to **Die 5**, in two rows, **First roll** and **Second roll**,
the character's ten values in the booklet's order (die 1 to die 5 for the
first roll, then for the second). The field to fill is outlined; a
value comes from the six-key dice pad, or on the desktop from the
keyboard's 1 to 6. As a die's second value lands, its column reads
"lower · 0" or "higher · 1"; an equal pair clears that die's two fields
with "Die 3 · roll both again" and the struck-through pair moves to the
sheet (§6.4.2). Once the five bits are in, the character drops into its
box and the two rows clear for the next character. The roller's values
land in the same fields, marked "rolled by this computer", so a person
sees one layout whichever way the values came. Backspace takes back the
last value of the character being entered.

**Honesty.** A rolled value is this computer's randomness (from
`Faraday::fresh`, as any of Faraday's randomness), not a die. The row
above the dice reads "Rolled by this computer"; each value it made is
marked so on the sheet and in its cell sheet (§7.4); the Result's
entropy row reads "Entropy · simulated dice" (or "simulated dice and
your dice" where both were used). This is the exception to §5's rule,
recorded as D4 (§14). The stick and the Pi have no roller: real dice
are typed on the pad there.

The conversion is shown live as the values land. Where the screen is
too crowded for the whole codex32 sheet (§6.4.7), the dice row and the
de-biasing sheet stay in view and the checksum worksheet opens on a
press, every cell with its working (§7.4).

### 5.6 The roller elsewhere on Faraday Desktop

Outside codex32, and on Faraday Desktop only: New key's dice for BIP-39
words (every dice way of `faraday/faraday-core/src/keygen/mod.rs`) and
Create a vault's **Generate with dice** (the diceware passphrase) gain
one die drawn the same way, with **Rolls per click**: 1 (the default)
· 5 · 10 · To the end, or a number typed. It stands beside the pad and
its entries, which stay for real dice exactly as the stick shows them
(DESIGN §4's dice pad: the entries in groups of five, the progress line);
the person uses either, or both. Each rolled value lands on the pad's
entries as a press would, with the same pause and the same Reduce motion
rule, and the same honesty: "Rolled by this computer" over the die, and
"simulated dice" on the result's entropy line ("simulated dice and your
dice" where both were used). The flows themselves are
unchanged; FLOWS and DESIGN take this (§15.3).

## 6. Make codex32 shares: the main flow

**Where**: New key's Start card, under **Other forms**, a new choice
**Make codex32 shares** after **Make SLIP-39 shares**
(`faraday/faraday-core/src/keygen/mod.rs`, `Start`), and the codex32 tile
of Tools. The flow is its own area (§13.1). Its cards, each a card of
New key's column (`docs/history/NEW-WALLET.md` §15), are below. Back
returns a card at a time; Cancel is F51's.

### 6.1 Scheme

A Choice per question, in this order:

- **Length**: 48 characters (ticked) · 74 characters. 127 is not
  offered (owner, §1.1 item 11).
- **Split**: **2 of 3** (ticked) · **3 of 5** · **Other** · **None, the
  secret alone** (k = 0). **Other** opens two steppers under it: the
  threshold, 2 to 9, and the strings, k to 31. The strings can be
  raised later (§8.2).
- Where k is 4 or more, one line under the choice: "The booklet
  publishes no translation sheet for k = 5; derived shares are this
  device's arithmetic." The worksheets are shown live all the same.

### 6.2 Identifier, first form

- **Identifier**: four characters typed on the codex32 keyboard,
  before any entropy, the booklet's order (III.1, step 2) and the
  Codex32QR profile's (a self-chosen label such as `L0VE`); the field
  starts empty and ✓ is live once four characters are in and none is
  refused (§6.4.5). **Suggest** fills four characters from this
  device's randomness, which the person may keep or change; the
  identifier is not secret. Typed first, it lets the checksum worksheet
  fill live (§6.4.3), since every checksum cell right of box 4 depends
  on boxes 5 to 8. The identifier from the fingerprint is used only
  where a seed already exists (§9, §6.4.5).

### 6.3 Entropy

A Choice: **Roll dice** (ticked; §5.1) · **Type the characters**
(§5.2) · **Type shares made by hand** (§5.3, §8.1).

### 6.4 Enter: the sheets on screen, filling live

#### 6.4.1 The screen

The Pad screen (DESIGN §5, Enter intent; or the Entry screen with the
codex32 keyboard for typed characters), titled by where the entry
stands: "Share A · character 7 of 26" (k of 2 or more) or "Secret ·
character 7 of 26" (k = 0). Its context, above the pad, is the
worksheets themselves, not a summary of them: the dice de-biasing sheet
(§6.4.2) for the character being rolled, and the checksum worksheet of
the share being entered (§6.4.3), with the translation worksheets
(§6.4.4) one press away. Nothing appears without its working: a
character is drawn only after the dice that made it, a computed cell
only once the cells it is computed from are drawn, and every computed
cell opens to its inputs (§7.4). 26 characters per share at 48, 52 at
74; k shares (one for k = 0), in the booklet's order: all of share A,
then all of C, and so on.

#### 6.4.2 The dice de-biasing sheet, live

Drawn as the booklet's sheet: five die tracks, each a free space and
pads 1 to 6, and the tree of 31 nodes over 32 leaves, the leaves
`Q P Z R Y 9 X 8 G F 2 T V D W 0 S 3 J N 5 4 K H C E 6 M U A 7 L`.
As each entry is pressed:

1. **First roll** (entries 1 to 5): die n's marker moves to the pad of
   its value on track n, the value written beside the track.
2. **Second roll** (entries 6 to 10): die n is drawn on the pad of its
   second value, and the track's row completes at once: "4 then 2 ·
   lower · 0", or "2 then 5 · higher · 1".
3. **Equal**: "3 then 3 · equal · roll both again"; the pair stays
   drawn, struck through, the marker goes back to the free space, and
   the next two entries are that die's again; the redone pair is drawn
   under the struck one. The other dice keep their values.
4. **The bits**: as each die's bit is known it is written at the
   track's end; the five read "0 1 1 0 1".
5. **The path**: the tree's branch for each known bit is drawn in the
   accent from the root, die 1's first; once die 5's bit is in the path
   reaches a leaf, "01101 = 13 → d", and the leaf's character moves
   into its numbered box in the share's string (box 10 for the first
   data character) and into the checksum worksheet's data cell for that
   box.

Bits arrive in die order only where the dice are entered in order; a
die whose second value is in before an earlier die's is shows its bit,
and the path is drawn only as far as the bits from die 1 run unbroken.
With typed characters (§5.2) there is no dice sheet: the character goes
to its box with "typed" as its working.

#### 6.4.3 The checksum worksheet, live

Every cell of the booklet's checksum worksheet (§7.1) stands in the
column of one box of the string, and every cell in column c is computed
only from the string's characters in columns c and to its left. This
holds by construction: a sum cell in column c adds the cell above it in
column c to the lookup cell in column c; a lookup row is the checksum
table's row for the two leading symbols of the sum row above it, and is
written two columns to the right of those symbols, so it depends only
on columns to its left. So the worksheet fills from left to right as
characters arrive, a diagonal front that follows the entry.

For a 48-character share (columns are box numbers; data is boxes 10 to
35; checksum boxes 36 to 48; at 74 characters read 61 for 35 and 62 to
74 for the checksum):

| When | What fills |
|---|---|
| The scheme and a typed identifier are in | the header's cells, boxes 4 to 9 (threshold, identifier, share index), and every computed cell in those columns: the HRP residue row (fixed, the residue of `MS`, the same on every sheet), the first sum row's cells for boxes 4 to 9, and the lookup rows selected by leading pairs within boxes 4 to 9 (each lookup's cells are known at once, though the sum cells beside them wait for their columns' characters) |
| Each data character, box c from 10 to 35 | its data cell; the first sum row's cell for box c while c ≤ 16; for c from 17, each sum row's cell in column c and, where c is the second column of a sum row's leading pair, the lookup row that pair selects (13 cells, columns c + 1 to c + 13: its cells are known, though the sum cells beside them wait for their own columns' characters) |
| Box 35, the last data character | lookup row 16, the last one; then, all at once, the 13 checksum characters (boxes 36 to 48) and every sum cell in columns 36 to 48, worked from the bottom row up as the booklet's instructions say: the final row is fixed at `SECRETSHARE32`, and each cell above it is the cell below it with the lookup cell between them added (addition is its own inverse in GF(32)) |

Why the checksum cannot fill earlier: the checksum characters are the
unknowns, and each one sits in a column whose cells include lookup row
16, which is selected by boxes 34 and 35. Nothing about box 36 is fixed
until box 35 is. Before that, the checksum boxes are drawn pink and
empty, as on the paper sheet.

At 127 characters (a backup of a word key's seed, §9.1, which has no
entry step) the same holds with the long checksum: 15 checksum
characters, and no published table.

The same worksheet in the verification direction (§8.1, §8.3) fills
the same way, every box being given: the final row is computed, column
by column, as the last characters arrive, and compared with
`SECRETSHARE32` one cell at a time; the first cell that differs is
marked the moment it is drawn.

#### 6.4.4 Translation worksheets, live

A derived share's (or S's) character at box c is the sum over the k
initial shares of each share's character at box c times that share's
translation symbol (§7.2). The symbols depend only on the share indices,
known from the scheme, so they are drawn first. Then:

- each initial share's translated row fills column by column while that
  share is entered (its character at box c times its symbol), so share
  A's translated rows in every derived share's worksheet fill during
  A's entry;
- the header columns (boxes 4 to 9) of every derived share and of S
  fill once the identifier is known: threshold and identifier carried
  through, the share index box reading the new index by construction;
- the sum row (the new string) fills in column c when the last initial
  share's character at box c arrives: during the last initial share's
  entry, column by column;
- the checksum columns (36 to 48) of the derived shares and of S fill
  when the last initial share's checksum does (§6.4.3), since they are
  sums of the initial shares' checksum characters; the derived strings'
  checksums hold by construction, and each derived string's own
  checksum worksheet (verification, §7.1) is one press away.

The initial shares are entered one after another, as on paper, so no
sum cell fills before the last share's entry begins. Entering the k
shares column by column (A's box 10, C's box 10, A's box 11, …) would
fill every sum cell as soon as its column is complete; the booklet does
not do it, and this spec does not offer it.

The identifier is typed before any entropy (§6.2), so nothing waits
for it.

#### 6.4.5 The identifier: refusals, and the fingerprint where a seed exists

Where a seed already exists (§9.1, §9.2), the identifier is the one
from the fingerprint, as `Keys::codex32_shares` takes it today. The same
arithmetic would allow it for a fresh set, since S's data is the interpolation of the initial shares'
data at `s`, position by position, which depends on the data and the
share indices alone and not on the identifier, so the seed and its
master fingerprint are known before any header is written. The
identifier is `identifier_for` of that fingerprint, and where it is
used its working is shown: "Fingerprint 73c5da0a · first 20 bits 01110
01111 00010 11101 · 14 15 2 29 · w 0 z a". Make codex32 shares does not
use it (owner, §1.1 item 2): it would hold the checksum worksheet's
computed cells back until the last character.

An identifier is refused, typed or worked out, where another set on
this device already carries it: the codex32 strings in Add a key's
form, the set of a key loaded from codex32 in this session, the
codex32 forms of the plan in hand, and the codex32 records of every
open vault (owner, §1.1 item 13). The line under the field names which
("Used by the set of key 9a6a2580", "Used in vault Home"). From the
fingerprint, a collision needs two seeds whose fingerprints share 20
bits; it is refused all the same, and a typed one asked for.

#### 6.4.6 Masking

DESIGN §4.10's mask policy masks dice entries, checksum bits and all
that the entropy becomes, behind the eye. A worksheet that fills live
under a mask shows nothing of its working, which is the opposite of the
owner's requirement. The owner's answer (§1.1 item 1), an exception
to the mask policy for this flow:

- **Faraday Desktop**: no masking of the live sheets; it is testnet only
  for real seeds.
- **The stick and the Pi**: the sheets are **unmasked while the flow is
  open** and masked again whenever it is left (Back past the first entry
  card, Cancel, a flow put aside, the device locking or idling); the eye
  in the app bar masks them at any moment.

#### 6.4.7 How it fits each size class

| Class | Over the pad | One press away |
|---|---|---|
| `wide` (the pane right of the sidebar) | two panels side by side: the dice sheet (tracks, tree, path) and the checksum worksheet, scrolled so the entry's front column is in view, the share's string with numbered boxes above both | a share's translation rows, from a strip of the worksheets under the title; the cell sheet (§7.4) |
| `mobile` | stacked: the share's string in two rows of numbered boxes; the dice sheet as five track rows ("Die 3 · 4 then 2 · lower · 0") with the tree drawn in a strip of 32 leaves and the path over it; the checksum worksheet as a band of the rows around the front, scrolling sideways with it | the full checksum worksheet, scrollable both ways; translation rows; the cell sheet |
| `small` (the Pi panel and 240 × 320) | the pad and one context panel at a time: the dice sheet while a character is rolled, as track rows and the path as a line "0 1 1 0 · ·"; after the fifth bit, the character's box for a moment, then back to the tracks | a tile row over the pad, **Dice · Checksum · String**, swaps the context panel; the checksum panel shows the rows that include the front column, two columns either side; the cell sheet is the whole panel as a page |

The worksheet's boxes are drawn at token sizes (§13.4). On `small` and
`mobile` a box is under the touch floor, so a press on the checksum
panel selects the nearest column and opens the cell sheet on that
column's cells, each listed with its working (§7.4).

### 6.5 The worksheets

After the last character, a review of every worksheet, already filled
during entry, one tab per worksheet, in the booklet's order (III.1.A,
III.1.B); nothing is computed here that was not drawn during entry:

0. **Dice · A**, then of C, D, … (k = 0: of the secret), where the
   entropy was rolled: the de-biasing rows of every character of that
   string, struck-through pairs included, each character's five bits and
   tree path, in box order.
1. **Checksum of share A**, then of C, D, … (k = 0: of the secret): the
   checksum worksheet (§7.1) with the data from the entropy in its bold
   boxes, marked "from your rolls" or "as typed", and the 13 checksum
   characters it makes.
2. **Share D** (k = 2), each derived share up to n in index order: the
   translation worksheet (§7.2), its translation symbols, each initial
   share translated, the sum. Its checksum worksheet, worked as the
   booklet's verification (§7.1), is one press away: **Check it**.
3. **The secret** (k of 2 or more): the translation worksheet towards
   `s`, with the recovery symbols and their fusion (§7.3). S is shown
   so that a person can follow it; it is not among the strings to
   write.

**Tabs, each scrolling within itself** (owner, 2026-10-22, §1.1 item
15). A tab row above the worksheet names every worksheet in the order
above ("Dice · A", "Dice · C", "Checksum · A", "Checksum · C", "Share D",
"Secret"); a press on a tab shows that worksheet, and each tab gains a
check mark once it has been shown, so what is left to read is plain. The
worksheet under the tabs scrolls vertically within itself, its box
numbers kept in view as it scrolls, every cell still opening its working
(§7.4). Not one long page (a 2 of 3 at 48 characters is the dice rows of
52 characters and four worksheets; a 74-character checksum worksheet
alone is some 60 rows), and not cards stepped one at a time (the person
comparing with paper jumps to one sheet). On `wide` and `mobile` the
tab row wraps to a second line where it does not fit; on `small`, where
six tabs do not fit, the tab row is a Choice titled by the worksheet
shown ("Checksum · A ▾") with **Previous** and **Next**. The tab row is
new to DESIGN §4 (§15.3). The live entry screen (§6.4) is where the
sheets are worked; the tabs are where each is read back whole.

### 6.6 The strings

One Secret screen per string (DESIGN §4.10, §5 Secret), titled
"Share 2 of 5" or "Codex32" for k = 0: the string as a comparison
string in groups of four at the largest mono size that fits; for a
48-character string a toggle **String · Codex32QR** shows its Codex32QR
code instead (§10.2). **Continue** asks the string typed back, as the
quiz does for words, and **Skip** goes on without, behind the quiz's own
caution. A typed-back string that differs names the first box that
differs ("Box 17 differs").

### 6.7 Strings made

The Result DESIGN §5 describes, now built: "Split · 3 of 5" or
"Split · none", "Length · 48", "Identifier · w0za", "Fingerprint ·
73c5da0a", "Entropy · your dice" (or "your characters", "your shares"),
and, where Faraday's randomness was used (§9), "Random shares · <source>"
with the Trust row for This device. **Check on another device** and
**Lock in**, which loads the key from S's seed as Add a key loads a
codex32 secret (no passphrase step) and fills a Create slot where one
asked for it.

### 6.8 Check on another device

A Record, opened from Strings made and from the key's backup checks:

- "Fingerprint · 73c5da0a": what any implementation must show for these
  strings.
- "Identifier · w0za", "Split · 3 of 5".
- **Sets to try**: a list of k-subsets in which every string of the set
  is at least once and the initial shares are not all in every subset:
  for 2 of 5, "A, C" · "D, E" · "F, A"; for 3 of 5, "A, C, D" ·
  "E, F, A". The count is ceil(n / k). Each must give the same
  fingerprint on the other implementation; a subset that gives another
  names a string to look at again.
- For k = 0, "The secret · ms10…" and the fingerprint only.

The record names no product. Learn (§15.3) names the implementations
that read codex32 as of the build, with the date.

## 7. The worksheets as Faraday shows them

During entry the worksheets are the Pad screen's context (§6.4); for
review, each is a Secret screen whose panel is the worksheet's grid.
Both are DESIGN §5 screens; no new screen is added. Masking is §6.4.6.
Each box is drawn at a token's size (§13.4) with its number above it,
in groups of four.

### 7.1 Checksum worksheet

The rows are the booklet's checksum worksheet, cell for cell, as
`volvelle-wasm/src/checksum_worksheet.rs` builds it from the booklet's
`ladder` (the first row of data, the HRP residue row, then pairs of a
sum row and a residue row, then the final row): the residue after the
first 13 data characters, then 2 characters a step, 16 steps at 48
characters and 29 at 74. Each step shows the two leading symbols of the
sum row and the checksum-table row they select, as the booklet's table
on its pages 15 and 16 gives it, so a person with the table checks the
lookup and a person with the addition wheel checks the sum.

- **Making a checksum** (generation): the final row is fixed at
  `SECRETSHARE32`, and the checksum boxes (the booklet's pink boxes) are
  worked from the bottom up, as its instructions say.
- **Checking a string** (verification, §8.3): every box of the string is
  given, and the final row is computed. A string is valid when the final
  row reads `SECRETSHARE32` (13 characters) or, at 127 characters,
  `SECRETSHARE32EX` (15): these are BIP 93's constants
  `MS32_CONST` and `MS32_LONG_CONST` written in the bech32 alphabet.
- A final row that differs is shown as it is, with the first box of the
  final row that differs marked; the booklet says this row "does not
  encode any secret share data" but encodes where the errors are, which
  is what §8.4 reads when the tick is on.

### 7.2 Translation worksheet

For a share at index x made from the k strings at indices x1 … xk:

- **Translation symbols**: one per input string, the Lagrange weight
  `∏ (x − xj) / (xi − xj)` over j ≠ i in GF(32), the value
  `faraday-bip`'s `lagrange` computes today. Each is drawn as the
  booklet draws it, in its second alphabet (`code2` in SSS32.ps, value
  0 to 31): `× ℵ α β Γ Δ ε η Θ Λ μ Ξ Π ρ Σ Φ Ψ Ω @ # % ¢ ¥ € ¤ ⊕ † ‡ § ¶
  ◊ ♥`, the value v drawn as the v-th; the booklet's Translation,
  Recovery and Fusion tables index it by value directly.
- **Rows**: each input string translated, character by character, by its
  symbol (a GF(32) product), under it; then the sum row, the new share,
  whose share index box reads x by construction (the booklet: "If not,
  you have likely misread the instructions").
- For a derived share the symbols are the booklet's derivation table
  column for x (III.1.B); for S they are recovery symbols fused (§7.3).

### 7.3 Recovery symbols and fusion

Towards `s` from any k strings, each string's symbol is the fusion (the
product) of its recovery symbols, one per other string, read off the
recovery wheel or table (III.2). The worksheet shows each string's
recovery symbols and the fused symbol, so the wheel's reading can be
checked. For k = 2 there is one recovery symbol and nothing to fuse.

### 7.4 Every cell opens to its working

A press on any cell of any worksheet opens DESIGN §4.16's **Sheet** (a
`CHART_SHEET_WIDTH` sheet on `wide`, the whole panel as a page on
`small`; on `mobile` the same sheet from the bottom), a component and not
a screen. Its head names the cell ("Box 23 · sum row 7"); its rows are
the cell's inputs and the rule, each input naming its own cell so the
person can find it on paper; the cells it names are outlined on the
worksheet behind the sheet while it is open. One action, **Done**. By
cell:

| Cell | The sheet shows |
|---|---|
| A data cell from dice | the die rows that made it ("Die 1 · 4 then 2 · 0", … ), the five bits, the path, the leaf |
| A data cell typed | "Typed" |
| A header cell | "Threshold", "Identifier", "Share index", as chosen, or the identifier's working (§6.4.5) |
| The HRP residue row | "The residue of MS: the same on every codex32 sheet" and its 13 characters |
| A sum cell | the two cells added and the result, "q + 9 = x", with "Addition wheel, or the addition table" |
| A lookup cell | the leading pair it was selected by, "Sum row 6 starts p z", and "Checksum table, row p z, character 5 of 13 (booklet pages 15 and 16)", with the whole table row drawn and this character marked |
| A checksum cell worked from below (generation) | the cell below, the lookup cell between them, the sum: "Below 3 + lookup e = this 7" |
| A final-row cell | in generation, "Fixed: S E C R E T S H A R E 3 2"; in verification, as a sum cell, with "should read R" and whether it does |
| A translation symbol | its value and glyph, how it was made ("derivation table, k = 2, column d, row A", or the recovery symbols and their fusion, §7.3) |
| A translated cell | "Share A, box 23: q × Δ = 7", with "Translation wheel set to Δ" |
| A translation sum cell | the k translated cells added, "7 + m = 4" |
| A character of a string shown elsewhere (§6.6) | which worksheet and box it came from, opening that worksheet |

The values in these sheets are read from the kept worksheet (§13.2);
opening one computes nothing.

## 8. The other flows

### 8.1 Shares made by hand, typed in

From §6.3's third choice, or Add a key › Type a codex32 secret with
"Make the rest" pressed after the first string:

1. The k initial shares are typed one at a time, as the form takes them
   today. Each is checked at once; one whose checksum fails is not
   taken: the line names it ("Share C · checksum fails") and **Show its
   worksheet** opens §7.1 in verification form with its final row, so
   the person can compare row by row with their paper. With the tick on
   (§8.4), a proposal is offered.
2. Once k are in, the set is checked (one threshold, one identifier, one
   length, distinct indices) and n is asked (§6.1).
3. §6.5's steps 2 and 3 follow, then §6.6 to §6.8. S is computed, never
   typed.

### 8.2 Add a share

Add a key › codex32 › **Add a share**: any k strings of one set are
typed (S may be one of them); the next unused index is offered
(**Index · f**), and any other unused one can be typed. The share is
derived on its translation worksheet (§7.2) and shown as in §6.6. The
strings are not loaded as a key unless the person presses **Lock in**
on the Result. An index already among the strings typed is refused;
Faraday cannot know the indices of strings it was not shown, and says
"Give it an index no other string of the set has" under the field.

### 8.3 Check a string

Tools › codex32 › **Check a string**, and **Check it** wherever a
string is shown in these flows: one string is typed or scanned, and the
answer is "Checksum holds" in the positive colour or "Checksum fails" in
the negative one, with the final row of its checksum worksheet
(`SECRETSHARE32`, or what it reads instead) and **Show the worksheet**
(§7.1, verification form). Nothing is loaded, kept or filed; the string
is dropped on leaving. Checking needs no key and no vault, and it is
offered where loading a key is not (a stick attached), since it loads
none.

### 8.4 Error correction (a tick, off by default)

**Where**: the codex32 form (Type a codex32 secret, §8.1, §8.2) and
Check a string carry a toggle row **Suggest corrections**, off whenever
the form opens. Off, a string whose checksum fails is refused as today.
On:

- The keyboard gains `?`, which marks a box the person cannot read (an
  erasure). With the tick off `?` is not on the keyboard.
- A failing string is decoded as BIP 93's "Error Correction" section
  allows: up to 4 substituted characters, or up to 8 erasures, or any
  mix with 2 × substitutions + erasures ≤ 8; and a run of 9 to 13
  erasures in a row on a 48- or 74-character string, with its own
  caution before the proposal ("This assumes every other box is right").
  Every length is corrected (owner, §1.1 item 8); the 127-character
  string's run bound is taken from the reference implementations'
  agreement, never assumed. Insertions, deletions and
  swapped groups are not searched for.
- `ms1` is never changed. Where earlier strings of a set are in, the
  threshold and identifier boxes are fixed to theirs.
- **A proposal is shown only where it is unique** within that bound, and
  never applied: "Possible correction" with the numbered pairs,
  "Box 17 · q → p", four to a page, all pages seen before going on.
- **I mistyped** (the paper is right): the changed boxes are cleared,
  every other box is locked, and the person types the changed boxes
  again **from the paper**, not from the proposal; only an exact match
  with the proposal is taken. **The paper is damaged**: the pairs are
  shown as repairs to write on the paper beside the original, then the
  same re-entry; the string is marked "Repaired, not checked".
  **Unreadable on the paper** (only for `?` boxes): the proposal is
  taken as it is, after a caution, and marked the same way.
- After any correction, the key's fingerprint is shown where it loads,
  with the line "Share entry corrected" (I mistyped) or "Share repairs
  unchecked" (damaged or unreadable), which stays beside any fingerprint
  comparison; a typed or seen match with a fingerprint recorded before
  never removes it.
- No proposal: "No correction found within 4 changes or 8 unreadable
  boxes" and **Check the boxes**, which keeps the entry for editing.

The decoder is written in `faraday-bip` (§13.1) and is a proposal
generator only: everything it returns is parsed again by
`Codex32::parse`, which stays the only door in. This loosens nothing BIP
93 requires (it says implementations SHOULD offer corrections and SHOULD
NOT proceed without confirmation), but it is new behaviour that the code
said it would not have, so it is a decision (§14, D2).

### 8.5 Loading: case, duplicates

- **Case**: the text is parsed as given, and only a string that parses
  is lowercased. A mixed-case string from a scan, a paste or a file is
  refused as BIP 93 says (§2). The codex32 keyboard types one case, so
  typing is unchanged.
- **A share index already in**: today refused, identical or not. As the
  Codex32QR profile (its §5.2) and the owner (§1.1 item 3): the same
  string again is ignored ("Already in"); a different string at the same
  index asks before replacing the one held, showing both with their
  differing boxes, **Keep the one held** · **Replace it**.

## 9. Backups

### 9.1 A backup's copies by hand

The Other paper forms' two codex32 chips become one, **codex32**, which
opens a Choice of k (None · 2 · … · 9, 2 ticked) and n (k to 31, 3
ticked), then makes the strings (§5.4) and shows them as §6.6 to §6.8 do,
"Random shares · This device" on the Result. The length is the seed's:

- a key made of BIP-39 words: its 64-byte seed, 127 characters, with
  "Holds · the seed, not the words" and "Not for checking by hand" on
  the Result;
- a key loaded from a master seed (codex32, SLIP-39, an `.oskb` kind 2):
  its own seed, 48 characters for 16 bytes. For this `faraday-keys` keeps
  the seed bytes of a key added by `add_seed` beside its master key (owner,
  §1.1 item 10; KEYS §2 already lists "seed" as key material).

k = 0 needs no randomness and is open in Demo (§12).

### 9.2 The backup plan's codex32 form

A seed's chip on the board gains a third form toggle, **codex32**,
beside Words and SeedQR (BACKUP-FLOW §17.3). Turning it on asks k and n
as §9.1 does; the seed's chip then stands for n string chips,
"9a6a2580 · codex32 2 of 3", each placed at a place by drag or the ⋮
menu as a share of the description is (`What::Share`). The plan's model
gains `What::Codex32Share(i, j)` for string j of seed i, Secret, and the
answers keep k, n and the length per seed. The checks read them as they
read shares: a place with k strings of one seed holds the seed.

The checklist gains **Codex32 strings of seed i**: its body the strings'
places, its foot **Make them** (§5.4, §6.6 to §6.8), each string typed
back or skipped, then Done. It is done when every string is shown and
typed back or skipped; the strings are never written to a file or a
stick (paper only, as the copies by hand are today). A plan made in
Demo dims the codex32 toggle for k of 2 or more, as DEMO §5.3 says.

### 9.3 The Templates item

Where a plan copies a seed as codex32, the checklist's **Templates** item
names that sheet too: "Print 2 from Faraday Starter: Blank templates,
24 words, words; 3 from Blank templates, codex32, 127 characters"
(BACKUP-FLOW §10.2's text, one clause per form).

## 10. Codex32QR

### 10.1 Reading

The Codex32QR/v1-48 profile is a QR whose text is one 48-character
codex32 string in upper case, version 3, error correction L. Faraday
reads its text as any codex32 string (§2), so:

- every scanner that takes a codex32 string takes a Codex32QR code: the
  codex32 form's Scan, Home's Scan and the intake scanner, and Files for
  a picture of one;
- Add a key's **Scan a SeedQR** takes one too, and opens the codex32
  form with the string in (as Home's Scan does), in place of "not a
  SeedQR".

A share (index other than `s`) scanned on its own opens the form and
waits for the rest of the set, as the profile's §5.1 asks; nothing is
loaded until enough strings are in.

### 10.2 Showing

Where Faraday shows one 48-character string (§6.6, §8.2, §9.1), the
Secret screen's toggle **String · Codex32QR** draws it as a Codex32QR
code: the string in upper case, byte mode, version 3, EC level L, the
standard quiet zone, black on white, at DESIGN §4.9's QR side. It is a
secret as a SeedQR is, masked with the eye. The Starter's codex32
template has a blank 29 × 29 grid for copying one by hand (§11).

### 10.3 Tests

- Every string of the profile's §10 vectors 1 to 4 and every PNG in its
  `test_vectors/` and `img/` folders is read by Faraday's scanner path
  (`third_party/rqrr` decoding the picture) to the expected string; each
  set recovers to the stated S and the stated fingerprint (vectors 1, 3,
  4 also to the stated xpub at the stated path on testnet). The vectors
  and pictures are copied, with the commit and SHA-256 of each, into a
  folder under `tools/vectors/` (C1), as `tools/vectors/bip93/` is.
  This spec checked the strings of vectors 1 to 4 with BIP 93's
  reference arithmetic: every checksum holds and every S is the
  interpolation of its shares. Vector 2's xpub in the profile is 81
  characters long, cut short; the test uses its fingerprint, and the
  profile's author is told.
- A Codex32QR code Faraday draws decodes, with `third_party/rqrr`, to
  the string in upper case, at version 3 and level L.

### 10.4 Longer strings

The profile defines 48 characters only, and says so (its §1). Faraday
shows no QR for a 74- or 127-character string and reads one only as
plain codex32 text (which it already does). No profile is invented
here: one for longer strings waits for the profile's author, and this
section takes it when it is published. Further input from the author
(the duplicate rule, the error-correction screens, a compact form) is
taken the same way: as a change to this section, with its vectors.

## 11. Blank templates (the Starter)

STARTER §3.8's Menu gains a first row, **Form**: **BIP-39 words**
(ticked) · **codex32**. With codex32, the rows are:

- **Length**: 48 (ticked) · 74 · 127 characters;
- **Copy**: **String** (ticked) · **Codex32QR** · **Both**, the last two
  at 48 only;
- **Worksheets**: **None** (ticked) · **Dice, checksum and
  translation**: at 48, the dice sheet, the checksum sheet and the
  translation sheets for k = 2 and k = 3, as the booklet's site
  publishes them; at 74, the dice sheet and the 256-bit checksum sheet
  (the site publishes no 256-bit translation sheet); none at 127.
  Translation sheets for k of 4 or more, and at 74 characters, are not
  published, and Faraday draws none (owner, §1.1 item 7);
- **Places**, **Print**, **Save the PDF** as for words.

A codex32 share sheet holds, per string: `ms1` printed in boxes 1 to 3;
labelled boxes **Threshold** (box 4), **Identifier** (boxes 5 to 8),
**Share index** (box 9); every box numbered as §3 numbers it, in groups
of four, rows of eight boxes; the 29 × 29 grid with the QR's fixed
patterns grey where Codex32QR is asked; a **Fingerprint** box, eight
hex digits, filled by hand (owner, §1.1 item 12); and at the foot the 32 characters of the alphabet in the sheet's
mono face, with 0, 7, Z and S slashed as the booklet asks handwriting to
be. The sheet prints at true size. The PDF's standard Courier has no
slashed zero, so the slashes are drawn as strokes over those four
glyphs.

**The booklet's worksheets.** The booklet's PostScript carries the MIT
licence, "Copyright (c) 2022 Blockstream" (its repository has no LICENSE
file), so its worksheets may be copied with that notice. Faraday draws
its own, in `faraday/faraday-core/src/pdf.rs`, with the booklet's
layout and numbering, from BIP 93's arithmetic (the checksum table rows
are computed, not copied); the tests check the drawn checksum table
against the residues of §7.1. The MIT notice is kept in the repository
(beside the fixtures and the reference page, §5.1) and on the codex32
Learn page, and is not printed on the sheets (owner, §1.1 item 6).

Saved as `faraday-blank-template-codex32-{48|74|127}-chars-{string|codex32qr|string-codex32qr}.pdf`,
with `-worksheets` before `.pdf` where they are asked for. Nothing else
changes in §3.8: no key, seed or wallet is asked for or shown.

## 12. Demo

| Flow | In Demo | Why |
|---|---|---|
| Make codex32 shares (§6) | Open, and the process runs whole: the person rolls (on the pad, or the roller on Faraday Desktop) or types, and watches the live sheets, the conversion and the checksums as anywhere. Once the entry screen is finished, the strings are replaced by the sample, BIP 93's vector 2 (`NAME`, k = 2: shares `a`, `c` and the derived `d`), and every later card shows the sample's, with one line: "Demo puts sample NAME in place of these strings". **Use a sample** fills vector 2's own characters for a person who would rather not roll | How the process works is the point; a seed rolled in Demo is a real seed that Demo would not hold, so it never leaves the entry screen. As Generate with dice puts `faraday` in place of the rolled words (DEMO §5.2) |
| Shares made by hand (§8.1), Add a share (§8.2) | Open for the BIP 93 vectors' strings (the demo's public seeds, MODES §7); a string of any other set is refused as a key that is not public is | The same |
| Check a string (§8.3) | Open for any string | It loads nothing and keeps nothing |
| Error correction (§8.4) | Open, on the sample strings with boxes changed | A proposal for a public string |
| Copies by hand, plan form (§9) | k = 0 open; k of 2 or more disabled, as today | k of 2 or more is Faraday's randomness (DEMO §6) |
| Codex32QR shown | Open for a public seed's string | Another spelling of a public seed |
| Blank templates | Not in the Starter's Demo; unchanged | Blank sheets hold nothing |

The first row is the owner's answer (§1.1 item 4). The scheme the
person chose is kept for the process; the sample's own scheme (2 of 3,
48 characters) replaces it with the strings.

## 13. Code

### 13.1 Where things live

- `faraday-bip`: `core/faraday-bip/src/codex32.rs` becomes a folder,
  core/faraday-bip/src/codex32/ (new): mod.rs (today's file), worksheet.rs
  (checksum rows, translation and recovery symbols, the code2 value
  order), fresh.rs (a set from k initial shares: S and the derived
  shares, headers written once the identifier is set) and correct.rs
  (the decoder of §8.4). Each new file joins `tools/lint-secrets.sh`'s
  secret files, as `codex32.rs` is.
- `faraday-entropy`: the booklet's dice method, a new module
  core/faraday-entropy/src/booklet.rs (new), not `lib.rs`, which is over
  2,500 lines (`tools/lint-layout.counts`). In the secret files.
- `faraday-keys`: a codex32 work set held as a new secret, as a BIP-85
  child or a FROST share is (KEYS §3: it "leaves only by `add_*` or
  `reveal`"): the characters handed in one by one as each is complete
  (the rolls of the character in progress stay on the pad, transient
  input under lint-secrets' rules, as keygen's do), the rows and strings
  computed there, lent to drawing through `Reveal::PaperForm`, and the
  seed handed to `add_seed` at Lock in. No new `Reveal` variant: the
  worksheets and strings are a paper form. `Keys::codex32_shares` takes
  any k and n and keys with a seed and no words (§9.1). New module
  faraday/faraday-keys/src/codex32.rs (new).
- `faraday-core`: a new area folder, faraday/faraday-core/src/codex32/
  (new), mod.rs (the flow's state, §6 and §8) and screen.rs (its cards),
  named in `lib.rs`'s code map (F74). Add a key's form keeps its place in
  `forms.rs`; the tick and `?` are there.
- `faraday-starter`: `template.rs` gains the Form row and the codex32
  choices; `faraday-core`'s `pdf.rs` draws the sheets.

### 13.2 Costs and drawing

- Every new function that does more than a lookup says so first: "Costs
  a checksum worksheet of one string", "Costs an interpolation of k
  strings", "Costs a decode of one string (syndromes, a search over at
  most 8 positions)". Each takes `c: &Compute`.
- What drawing shows is kept on the state: the work set's rows and
  strings are computed when a character, a share or the identifier
  changes, and are kept in step with them at every change; the invariant
  is written on the work set and tested by changing a character and
  reading the rows. Drawing reaches them through a named `View`
  accessor (COMPUTE's `Allowed`), never by computing.
- The worksheets are filled incrementally: a pad press computes the
  cells of at most the columns its character completes (one column of
  each worksheet the character is in, the dice sheet's track, and at
  box 35 the checksum columns), never the whole sheet again. The budget
  tests (`faraday/faraday-core/src/work/budget/`) gain the pad press that
  completes a character and the press that completes box 35, each with
  that bound, and the cell sheet's opening, which computes nothing.
- The incremental worksheet equals the worksheet computed whole from the
  finished string, cell for cell; a test fills one character at a time
  and compares at every step.

### 13.3 Strings

Every new string is in `faraday/faraday-strings/src/en.rs`: the cards'
titles ("Scheme", "Entropy", "Share A · character 7 of 26"), the Choice
rows, the worksheets' labels ("Checksum of share A", "Translation
symbols", "Recovery symbols"), "Strings made" and its rows, "Check on
another device" and its rows, the error-correction screens' words (§8.4),
the Starter's rows. Working screens carry no explanation: why the
characters must be random, what a translation symbol is, and how to use
the booklet are in Learn (§15.3).

### 13.4 Tokens

The worksheet's box side, gap, group gap and number size per class are
names in `core/faraday-draw/src/tokens.rs` (WORKSHEET_BOX and the like,
and the dice sheet's track pad, tree node and leaf);
the PDF's in `pdf.rs`'s own constants as the SeedQR grid's are.

### 13.5 Tests of the cryptography

Each against published vectors or another implementation:

| What | Against |
|---|---|
| Strings made from initial shares, S, derived shares | BIP 93 vectors 2 and 3 (`tools/vectors/bip93/vectors.json`); the Codex32QR vectors (§10.3); BIP 93's Python reference (`ms32_interpolate`) through F79's offline cross-check, over random sets of every k and n at 48, 74 and 127 |
| Checksum worksheet rows | `volvelle-wasm`'s `checksum_worksheet.rs` (CC0), run once offline over every BIP 93 string and the Codex32QR strings at 48 and 74 characters, its cells kept as a fixture under `tools/vectors/` with the commit and SHA-256; every row also equals BIP 93's reference polymod over the same prefix (F79); the final row of every valid vector is the constant |
| Translation and recovery symbols, code2 | the booklet's derivation tables for k = 2 and k = 3 and its Recovery table (SSS32.ps, 2023-03-07), transcribed once from the PDF into a fixture |
| Dice to characters | the booklet's tree (§5.1), all 32 paths, from the `code` array; a die with equal values is never read |
| Error correction | python-codex32's frozen corpus (57 cases; the sources table at the top), every case's expected string; python-codex32's `correction.py` through F79's cross-check over random errors within the bound, both lengths; the Haskell implementation of pull request 70, from whose `Tests.hs` the corpus is drawn; and, for every string, that no proposal is made beyond the bound where a second candidate exists |
| Codex32QR | §10.3 |

User-facing tests (CLAUDE.md): a flow rolled from the sample makes
vector 2's strings and loads its fingerprint; a string typed back with a
box changed names that box; a share at an index in use is refused or
replaced as decided; a failing string with the tick off is refused and
with it on shows the unique proposal and loads only after the boxes are
typed again from the paper; Check a string loads nothing; Demo refuses
non-sample entropy; each template choice makes a PDF saved under its
name.

## 14. Decisions to record

The build's first batch records these (the next free numbers at the
time, F83 onward as of this writing), each its own file and a row in
`docs/DECISIONS.md`:

- **D1. Codex32 made by hand, any split.** §1 items 1 to 8 and 12: the
  main flow, lengths, identifier, hand-made shares, add a share, check a
  string, backups and templates, with the owner's answers (§1.1).
- **D2. Codex32 error correction, opt-in.** §8.4. It replaces the
  module's statement that it "detects; it does not correct".
- **D3. Codex32QR/v1-48.** §10: read, shown, tested; nothing longer
  until its author publishes it; the duplicate-index rule of §8.5.
- **D4. Faraday Desktop's dice rollers.** §5.5, §5.6: on Faraday Desktop
  only, simulated dice from this computer's randomness, labelled
  "Rolled by this computer" and "simulated dice"; the exception to §5's
  rule that Faraday's own randomness never makes a fresh seed in these
  flows.

## 15. Plan

### 15.1 Rules for the implementing agent

- CLAUDE.md holds. Agents do not commit. Run one test module while
  working (`cargo nextest run -p faraday-bip -E 'test(/^codex32::/)'`),
  `just` once at the end.
- Nothing cryptographic lands without its row of §13.5 passing.
- Fixtures copied from other projects keep their licence notice and are
  recorded with commit and SHA-256 in their folder's README, as
  `tools/vectors/bip93/README.md` is.
- No path in a doc that does not exist; new files are named in plain
  text until they exist.
- Never cite the private board; never name a person who is not the
  public author of a standard or a tool.

### 15.2 Batches

| Batch | What | Model | Estimate |
|---|---|---|---|
| C1 | `faraday-bip` codex32 folder: worksheet rows, symbols, fresh sets, any k and n; fixtures (volvelle cells, booklet tables, Codex32QR vectors and pictures); F79 cross-check hooks; D1 to D4 recorded | Opus | 1.5 days |
| C2 | The decoder of §8.4, the correction corpus, its cross-check | Opus | 2 days |
| C3 | The booklet dice method in `faraday-entropy`, its reference page | Sonnet | 0.5 day |
| C4 | `faraday-keys`: the work set, seed bytes for `add_seed` keys, `codex32_shares` for any k, n and wordless keys; lint-secrets | Sonnet | 1 day |
| C5 | The main flow (§6, §7): cards, the live entry screen (dice sheet, checksum and translation worksheets filling per press, at three size classes), the cell sheet, review cards, strings, Strings made, check on another device, Codex32QR shown; snapshot tours at each class | Opus | 5 days |
| C6 | §8.1 to §8.3 and §8.5: hand-made shares, add a share, check a string, case and duplicates; scanning (§10.1) and its tests | Sonnet | 1.5 days |
| C7 | §8.4's screens: the tick, proposals, re-entry, the lines on the fingerprint | Opus | 1.5 days |
| C8 | §9: copies by hand, the plan's codex32 form, its checklist item, the Templates text | Opus | 2 days |
| C9 | §11: the Starter's codex32 templates and worksheets | Sonnet | 1.5 days |
| C11 | §5.5: the codex32 roller on Faraday Desktop: the die pictures into the assets, five dice, the click choices, the tumble, the pause and Reduce motion, the honesty labels, and tests that a roll fills the sheet as the pad would and that its values are marked as this computer's | Sonnet | 1.5 days |
| C12 | §5.6: the one-die roller for New key's dice and Generate with dice, Rolls per click, the labels; FLOWS and DESIGN edits | Sonnet | 1 day |
| C10 | §12 Demo, Learn, the edit list (§15.3), then this file's plan to docs/history/CODEX32-LOG.md (new) and its Kind to Reference | Sonnet | 1 day |

About 20 days. C1 before everything; C2 before C7; C3 and C4 before C5;
C5 before C6, C8, C9 and C11; C12 stands alone; C10 last.

### 15.3 Edit list

| Document | Section | Change |
|---|---|---|
| `docs/FLOWS.md` | New key and Add a key | Make codex32 shares under Other forms; the form's Add a share, Check a string and Suggest corrections; Scan a SeedQR taking a codex32 string |
| `docs/FLOWS.md` | Backups, Backup flow, Wallets tab | the one codex32 choice; the plan's codex32 form and item |
| `docs/DESIGN.md` | §4.3 Entry | the codex32 keyboard's numbered boxes, `?` with the tick |
| `docs/DESIGN.md` | §4.10, §5 Pad, Secret, Result, Choice | the worksheet as a Secret panel; the booklet dice pad; String · Codex32QR; Strings made as built; the Scheme choices |
| `docs/FLOWS.md` | New key and Add a key; Vaults (Create a vault) | §5.6: the roller on Faraday Desktop for dice words and Generate with dice |
| `docs/DESIGN.md` | §5 Pad; §4.10 | the dice row and the one die, Rolls per click, "Rolled by this computer"; the masking exception of §6.4.6 |
| `docs/DESIGN.md` | §4 (new component) | the tab row: one tab per item, a check mark once shown, wrapping on `wide` and `mobile`, a Choice with Previous and Next on `small` (§6.5) |
| `docs/DEMO.md` | §5.2 | the roller in the demo: Generate with dice and New key unchanged in outcome |
| `docs/STARTER.md` | §3.8 | the Form row and §11 |
| `docs/BACKUP-FLOW.md` | §10.2, §12, §17.3, §17.6 | the Templates text; strings; the codex32 toggle and `What::Codex32Share`; presets unchanged |
| `docs/KEYS.md` | §2, §3 | the work set as a new secret; seed bytes kept for `add_seed` keys; `PaperForm` covering both |
| `docs/QR.md` | §1 Payloads read, §2 | Codex32QR read and shown |
| `docs/INTEROP.md` | §2 | a row: Codex32QR/v1-48, read and written, tested |
| `docs/DEMO.md` | §4, §5.3, §6 | §12's rows |
| `docs/MODES.md` | §7 | the samples for §6 and §8 |
| `docs/AUDIT.md`, `core/faraday-bip/README.md` | the faraday-bip row | worksheets and correction, their vectors |
| `docs/MULTIVENDOR.md` | §4 | codex32 sets and correction, python-codex32 |
| `docs/learn/23-other-backups.md` | Codex32 | making shares here and by hand, the booklet, lengths, checking on another device, corrections |
| `docs/learn/19-glossary.md` | Codex32 | identifier, share index, translation symbol |
| `docs/README.md` | Spec list | the status as built |

### 15.4 Questions for the owner

None open: the owner answered every question on 2026-10-22 (§1.1).
