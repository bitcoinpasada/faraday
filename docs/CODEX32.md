# Codex32: making, checking and repairing shares

**Kind:** Reference
**Status:** Built. Codex32 in Faraday: the worksheets, symbols and fresh sets in `faraday-bip`, the error-correction decoder, the dice method, the work set, Make codex32 shares and its wallet kind, dice of up to twenty faces in colours the person picks with the booklet's dice sheet drawn as the booklet draws it, hand-made shares, Add a share, Check a string, scanning and the correction in the form, backups, the Starter's templates, both dice rollers on Faraday Desktop, the codex32 one stepping through the work in a Demo only pane that holds its place, and Demo's sample; decisions F93 to F96 and F107. Strings of 74 and 127 characters have no Codex32QR picture, as the profile does not define one (§10.4)

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
| FractalEncrypt's printable cards | the same branch at `9b8cbe0aaf527ebe9e85d0e90c0b79e2018126c8`, docs/codex32_qr/printable_templates/Seedsigner_Codex32_Printable_SeedCards.pdf (SHA-256 `a17867d8a2368cdfcb0c2569715e8f2c6c1769dce374946748b91926644aecc2`; three Letter pages: a share card, a share card beside a QR worksheet, four smaller cards), and Codex32_Screen_Reference.md's zoomed QR transcription; compared with Faraday's sheets in §11, not copied (no test needs the file) | MIT, as above (`tools/vectors/codex32qr/LICENSE.md`) | 2026-10-24 |

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

1. **Masking of the live sheets**: on Faraday Desktop, the stick and
   the Pi alike (F117; first answered as none on Faraday Desktop):
   unmasked while the flow is open, masked when it is left, the eye at
   any time (§6.4.6).
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
   codex32 Learn page; not printed on the sheets (§11), except one
   small credit line on the printed dice worksheet (item 37, 2026-10-24).
7. **Templates**: the published 256-bit checksum sheet is printed; no
   translation sheet the booklet has not published is drawn (§4, §11).
8. **Error correction** at every length (§8.4).
9. **Dice** (owner, 2026-10-24, replacing "six-faced only" of
   2026-10-22): dice of up to twenty faces, as the booklet's sheet has
   pads 1 to 6 and, for dice of seven faces or more, pads 7 to 20. The
   person says which dice they use (d6 by default; d4, d8, d10, d12,
   d20), picks each die's colour, and sees each die's two rolls side by
   side; the roller on Faraday Desktop rolls the chosen die; the sheet
   on screen and the printed one are drawn as the booklet draws them
   (§5.1, §5.5, §6.4.2, §11).
10. **Seed bytes**: `faraday-keys` keeps a master seed's bytes for keys
    loaded from one (§9.1).
11. **127 characters** are not offered in Make codex32 shares (§4, §6.1).
12. **A fingerprint box** on the printed share sheet (§11).
13. **"Identifier already in use"** also checks the open vaults (§6.4.5).
14. **The review after entry**: tabs, one per worksheet in the booklet's
    order, each scrolling within itself and checked once shown; on the
    small panel a Choice with Previous and Next (§6.5).
15. **The sheets' look**: the booklet's layout and colour roles, so they
    feel familiar; more beautiful where possible; every colour, line and
    motion helping the reader follow, none distracting (§7.5).
16. **A key that already has a codex32 set** (made here from dice, or
    loaded from strings): its backup places and checks that set and never
    splits the seed again; the key record keeps the set's plain facts
    (k, identifier, length, indices), no strings (owner, 2026-10-23); a
    new set only by Rotate the set (§9.4, §9.7).
17. **Codex32 on New key's Start card at its top level**, beside BIP-39
    words, with BIP-39 words ticked; the default to be looked at again
    when more wallets read codex32 (§6).
18. **Shares on the backup board**: each string a chip, placed on paper
    (string or a drawn Codex32QR), as stick files (`.txt`, Codex32QR
    `.png`; allowed, with the files warning) or in a vault (a new share
    record, no seed needed); places count distinct shares; strings from
    several vaults add up to the key; the standard way, as BIP-39's,
    paper only: one string per place, with Customize backup for vaults,
    files and Codex32QR (§9.5).
19. **Codex32 is made for single-key wallets only**: Make codex32
    shares is not offered in a multisig, Taproot multisig, MuSig2, FROST
    or recovery wallet's slot; a codex32 key already loaded is accepted
    in any slot (§6.9).
20. **A wallet kind, "Single key, backup in shares"**, on Make a new
    wallet's Kind card (its place and default since item 24): single-sig
    on chain, its Quorum the share scheme, its one slot made by Make
    codex32 shares; codex32 only, not SLIP-39 (§6.9).
21. **SLIP-39 words and other conversions** leave this spec (owner,
    2026-10-23): seed conversions (codex32 to SLIP-39 and the others) are
    a dedicated conversion tool of their own, planned, not scheduled
    (§10.5). Nothing about them is in a vault or on the backup board.
22. **Typed characters**: the line "Characters from dice or a
    worksheet, not chosen" above the entry (§5.2).
23. **A codex32 key in a vault** is saved and shown as any key is: the
    vault gives nothing it holds special treatment (owner, 2026-10-23).
24. **Codex32 first on Make a new wallet** (owner, 2026-10-24): the flow
    opens on Kind; "Single key, backup in shares (codex32)" is its top
    row and the default, ticked as the card opens, so the ticked row
    pressed again opens its Quorum; each row's second line stays as
    §6.9 states it. The word "codex32" in that name carries a tooltip,
    "codex32 is the only method that makes a seed and its shares fully
    by hand, the checksum included.": the one explanation on a working
    screen, a deliberate exception recorded as a decision (F112). The
    tooltip is marked by Material's info icon button right after
    "(codex32)" (§6.9).
25. **Move the money opens Kind with nothing ticked** (owner,
    2026-10-24): the person moves money to a wallet of their choosing,
    so Faraday does not guess. The rows are in the same order; Next waits
    until a row is chosen, and a row chosen moves on as any does. Make a
    new wallet opened on its own keeps the codex32 row ticked (§6.9).
    Its key record keeps the codex32 set for the backup plan (§9.4), and
    the board names a backup-in-shares seed placed whole in a vault
    (§9.5, §9.6).
26. **The set's actions** with the backup, on the set's chart node:
    Add a share, Replace a lost share (k strings re-derive it exactly),
    Rotate the set (the person marks lost and exposed strings; Faraday
    says which held strings to destroy so fewer than k old ones can
    exist, or that the seed must be treated as known; then a new set
    from the seed with a new identifier). Splits always take fresh
    randomness. The same actions suit SLIP-39 sets, specified
    elsewhere (§9.7).
27. **Desktop dice rollers** (Faraday Desktop only; the stick and the Pi
    keep real dice typed on the pad): five dice for codex32, one die for
    BIP-39 words and Generate with dice, their values this computer's
    randomness and labelled so (§5.5, §5.6). The entry for real dice
    stays beside the roller on the desktop, so the person chooses: their
    own rolls, Faraday's, or both; codex32's entry for real rolls is laid
    out as the de-biasing worksheet, five dice by two rolls (§5.5).

Added 2026-10-24 (owner):

28. **The checksum worksheet on screen is the booklet's**, formatting and
    layout exactly, against the booklet's generation sheet and its
    verification examples: the same staircase, the boxes in groups of
    four with their small numbers, `MS1` at the head, the `+` and `=`
    row marks, the given and checksum boxes bolded, the pink checksum
    region, the small row letters (a, b, c, …) and the final
    `SECRETSHARE32` row. §7.5's readability rules stay where they do
    not change the layout (§7.1, §7.5).
29. **Where the worksheet does not fit it scrolls both ways**, and a
    click and hold (mouse) or a touch and hold drags it, on every shell
    with a pointer or touch; the keyboard scrolls it too (§7.5).
30. **Every box of the string is numbered**, small and quiet over each
    box, the groups of four kept (§6.4.1).
31. **"Demo only"** (owner, 2026-10-24): the computer's dice roller and
    Demo's **Use a sample** are not the person's entry. They stand in
    one outlined box headed "Demo only", at the right of the entry where
    the card is wide enough and under it where not, so the person's own
    dice (the sheet and "Your dice") are the page. Both are offered in
    Demo only (owner, 2026-10-24 evening; `docs/DEMO.md` §14): the roller
    on Faraday Desktop in Demo, the sample in Demo; in every other mode,
    Faraday Desktop's too, the box and all it holds are gone, and the
    stick and the Pi have neither. The dice's size is one choice, on
    the Entropy card at every size (owner, 2026-10-25), which the pad
    and the box ("Rolls d20") follow; changed in the middle of a
    character, that character's rolls are cleared and the note says so
    (§5.1, §5.5).
32. **A Codex32QR drawn by hand from the backup flow.** Faraday follows
    the Codex32QR/v1-48 profile (§10). Where the plan draws a string's
    Codex32QR on paper, the checklist names the sheet it is drawn on
    (the Starter's Blank templates, codex32, 48 characters, Codex32QR),
    shows the code as a grid to copy square by square with its blocks
    named, and checks the drawn copy by scanning it back (§9.5,
    BACKUP-FLOW §17.24).
33. **The copy grid is marked block by block**, five squares from the
    top left as the sheets' heavier lines are (narrower at the edge where
    the side is not a multiple of five), with a toggle to Rows; the
    pointer over a square lights its block; the keys move block by block.
    For SeedQR and Codex32QR alike (BACKUP-FLOW §17.24).
34. **The Starter's Codex32QR sheets** are at least as usable as
    FractalEncrypt's printable cards: the grid on a page of its own at
    true size, its blocks named as the device names them, the squares
    every code shares printed black, the string's header boxes and the
    fingerprint beside it; and a page of four grids (§11).
35. **The sheets' line** (owner, 2026-10-24), on every blank template
    the Starter or Faraday prints (codex32, Codex32QR, SeedQR, words):
    "Written by hand. Never photographed or scanned by any other device
    once filled in." Faraday's own camera checks a drawn copy (§9.5);
    no other device sees it.
36. **A string of a set that exists is typed from its paper before its
    grid shows** (owner, 2026-10-24), as built: the device keeps no
    string of a set it did not make, and `faraday-keys` gains no reveal
    for one (§9.5).
37. **The printed dice worksheet's credit** (owner, 2026-10-24, amending
    item 6 for that sheet alone): each of its two printed pages carries
    one small line at its foot, "Dice worksheet after the codex32
    booklet, © 2022 Blockstream, MIT." No other sheet and no screen
    shows it, and the worksheets always start on a page of their own
    after the share card and the Codex32QR pages, so nobody reads a
    printed backup as someone else's (§11).
38. **The demo dice step through the work** (owner, 2026-10-24),
    replacing the roller's one-click choices: in the Demo only card,
    Previous and Next (and ← → while the card's entry is open), a step
    count ("Step 14 of 19 · character 3"), and Play with how far it runs
    (One pass, One character, One share, To the end) and how fast. A
    character's values are rolled first, as the roller rolled them, then
    shown a step at a time: each die's first roll and its pad, each
    second roll with its comparison and bit, the five bits read down the
    tree into their box, and the checksum worksheet's cells that box
    fills, a sum or a table row at a time, each in plain arithmetic in
    the card with the cells it touches marked. Previous shows the state
    before and rolls nothing; Play reaches what stepping reaches. The
    worksheet's view follows the work as smoothly as the fades: a timed,
    eased move, at once under Reduce motion, for every way the front
    moves (§5.5, §6.4.7, §7.4, §7.5). Built.
39. **The Demo only card holds its place** (owner, 2026-10-24): a
    supporting pane (`docs/LAYOUT.md` §5, §6) that does not scroll with
    the page; the page scrolls beside it, and a card taller than the
    window scrolls within itself. Its top is the page content's, below
    the top strip of the network pill and the eye, never level with
    them, against the right edge with Material's 24 dp margin and the
    24 dp spacer from the page. On windows too narrow for the side pane,
    the step's line, Previous, Next and Play sit in a strip pinned at the
    window's foot that opens to the whole card and never scrolls away
    (§5.5). Built.

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

- **The five-dice roller** (C11), `faraday/faraday-core/src/fresh/roller/five.rs`
  and `five_screen.rs`: `FiveRoller` rolls the booklet's dice sheet for a
  click, five dice and four reaches (One pass, One character, One share,
  To the end), as faces 1 to N of the dice chosen (six unless told, up to
  twenty, each face equally likely, §5.5) in the pad's order, an equal pair's die
  rolled again, so that the faces entered as the pad's keys give
  exactly what the same faces typed give. It marks each face it made by
  character and place (`is_rolled`, `char_rolled`, `sim`), tumbles for
  `ROLLER_TUMBLE_MS` and pauses after each character, and under Reduce
  motion lands all at once. The dice are drawn as vectors, as the one die
  is; the website's WebP pictures are not used, as no image decoder
  is to be added. Tested in `five_tests.rs` against the codex32 flow's own
  entry; snapshot tour `dice5`. It stands beside the entry's dice pad,
  "Your dice", on Faraday Desktop (`codex32/screen.rs`, `pad`; side by
  side where the card is wide, under the pad where not); its faces enter
  as the pad's keys, the dice of the character in progress it made are
  named under the dice sheet, a character it made is marked in the
  review and in its cell sheet, and Strings made's entropy row reads
  "simulated dice" or "simulated dice and your dice"
  (`tests/all/codex32_roller.rs`). The one die (C12) stands the same way
  beside New key's dice pad, its rolls marked, and the Enter card's
  summary reads "Entropy · simulated dice" where it made them.
- **The codec**, `core/faraday-bip/src/codex32/mod.rs`: `Codex32::parse`
  (every rule of BIP 93; one case, mixed case refused), `from_seed`,
  `split(threshold, shares, fresh)` (an existing secret and k − 1
  random shares, then the rest interpolated), `derive_share`, `recover`,
  `to_seed` (padding bits ignored, as BIP 93 says), `identifier_for`
  (the first 20 bits of the master fingerprint), `symbols_for` and
  `seed_symbols`. It detects errors; correcting them is the decoder's,
  below. Tested in
  `core/faraday-bip/tests/all/codex32.rs` against every BIP 93 vector
  (`tools/vectors/bip93/vectors.json`).
- **The worksheets and fresh sets** (C1), in the same folder, tested in
  `core/faraday-bip/tests/all/codex32_sheets.rs` and cross-checked
  against python-codex32 (F79, `tools/crosscheck/area_codex32.py`):
  - `worksheet.rs`: `ChecksumWorksheet`, the booklet's checksum
    worksheet of one string (§7.1), every cell kept, filled one box at a
    time (`push`, `pop`) or whole (`of`), the checksum made from the
    bottom row (`finish`), the final row against `SECRETSHARE32`
    (`holds`, `first_difference`), at every length BIP 93 allows (the
    long checksum's at 127; a first step of one box where the
    characters after the first row are odd in number, as at 54, 61, 67
    and 127, for which the booklet prints no sheet);
    `TranslationWorksheet` (§7.2, §7.3), its translation symbols, each
    input's pair symbols (the recovery symbols towards `s`), the
    translated rows and the sum, filled one input's box at a time;
    `checksum_table`, `pair_symbol`, `translation_symbols`, and the
    booklet's second alphabet (`SYMBOL_GLYPHS`, `SYMBOL_NAMES`).
  - `fresh.rs`: `Scheme` (a threshold of 0 or 2 to 9, up to 31
    strings, an identifier, 48, 74 or 127 characters) and `fresh_set`,
    a set made from the k initial shares' data: each checksummed, the
    secret and the derived shares interpolated.
  - Fixtures: volvelle-website's worksheet cells for every published
    string at 48 and 74 characters (`tools/vectors/codex32-volvelle/`),
    the booklet's checksum, derivation, recovery, addition, translation
    and fusion tables (`tools/vectors/codex32-booklet/`), and the
    Codex32QR profile's vectors and pictures (`tools/vectors/codex32qr/`).
  - The costly functions take the compute token, say "Costs …" and
    count themselves as `Work::Codex32` (core's kind, mapped by
    faraday-core's `of_core`; C4).
- **The decoder** (C2, §8.4, F94), `core/faraday-bip/src/codex32/correct.rs`:
  `correct(c, text, held)` reads a string in which `?` marks an
  unreadable box and returns the one correction the checksum justifies
  (`Proposal`: the string, parsed again by `Codex32::parse`, and its
  edits, box by box, `was` and `now`) or a `Refusal`. Within BIP 93's
  bound, `2 × substitutions + unreadable ≤ 8`, at every length, by
  syndromes over GF(1024), Berlekamp–Massey and Forney; a run of 9 to 13
  unreadable boxes in a row (9 to 15 at 127 characters, the bound of the
  corpus below) by solving the checksum's equations, marked
  `assumes_the_rest`. `ms1` is never changed; given `held`, a string of
  the set already in, boxes 4 to 8 must read as its and are never
  changed. Anything else is refused: more unreadable boxes not in one
  run, more changes than the bound, a character neither bech32 nor `?`.
  Tested in `core/faraday-bip/tests/all/codex32_correct.rs` against
  python-codex32's frozen corpus of 57 cases
  (`tools/vectors/codex32-correction/`) and every mix and run within the
  bound at every length BIP 93 publishes a string of; cross-checked
  against python-codex32's decoder (F79,
  `tools/crosscheck/area_codex32_correct.py`) over that corpus and random
  damage up to and past the bound.
- **§8.4's screens** (C7), `faraday/faraday-core/src/codex32_correct/`:
  `Correction`, which a codex32 form holds, fresh with the tick off each
  time the form opens. Its `offer` makes the one decode, with a string of
  the set already in as `held`; a refusal is its own line (no correction
  within the bound, too many unreadable boxes, boxes 4 to 8 fixed by the
  strings in, not a codex32 string) with **Check the boxes**; a proposal
  is shown as "Box 17 · q → p", four to a page, a run of more than eight
  unreadable boxes after its caution, and **I mistyped**, **The paper is
  damaged** and, where every changed box was `?`, **Unreadable on the
  paper** are pressable only once every page was seen. The re-entry
  clears the changed boxes, locks the rest, takes bech32 characters only
  and takes the string only on an exact match; what it hands the form is
  read by `Codex32::parse` like any string typed. A string taken carries
  its mark (`Mark::Corrected`, "Share entry corrected"; `Mark::Repaired`,
  "Share repairs unchecked", the string "Repaired, not checked"), and the
  key the strings make carries the strongest (`key_mark`, `KeyMarks` by
  fingerprint, never removed). `screen.rs` draws the tick's row, the
  panel in place of the form's field and the line beside a fingerprint;
  drawing works nothing out, and a press, a key or a frame does no costly
  work (`faraday/faraday-core/tests/all/codex32_correct.rs`). The codex32
  form does not hold it yet: the tick, `?` on the keyboard and the
  fingerprint's line appear once the form (C5, C6) carries it.
- **Make codex32 shares** (C5, §6, §7), the area
  `faraday/faraday-core/src/codex32/` (`mod.rs`, the flow's state and
  presses; `screen.rs`, its cards), opened from New key's Start card
  (Make a new key › Codex32, `faraday/faraday-core/src/keygen/mod.rs`),
  offered on its own and in a single-key Create's slot, not in a slot of
  a wallet of several keys. Built: the Scheme card (48 or 74 characters;
  2 of 3, 3 of 5, Other with its steppers, None; the line for k of 4 or
  more); the Identifier typed on the codex32 keyboard or suggested from
  this device's randomness, refused where the strings typed in Add a key
  carry it; Entropy (dice by the booklet's de-biasing sheet, or
  characters typed); the entry, its title "Share A · character 7 of 26",
  with the dice sheet (tracks, pairs, an equal pair struck and rolled
  again, the bits, the tree's path), the share's string in numbered
  boxes, and the checksum worksheet's rows round the front column or a
  translation worksheet from the strip, all filling per press from the
  work set, which now also keeps one live translation worksheet per
  derived share and towards `s` (`WorkSet::live_translation`); the
  layout by the window's class (`docs/LAYOUT.md` §2): expanded and up
  side by side, medium stacked, compact one panel at a time under the
  pad; every cell opening its working (§7.4) on a sheet; Worksheets as
  tabs (Dice and Checksum per initial share, each derived share, the
  secret), ticked once shown, Previous and Next on a compact window;
  Strings, each typed back on the codex32 keyboard ("Box 17 differs") or
  skipped, with String · Codex32QR at 48 characters (alphanumeric mode,
  version 3, level L, upper case); Strings made with Check on another device
  (§6.8) and Lock in (`Keys::add_codex32`, marked made here, a Create
  slot filled). The sheets' colour roles are `CODEX32_CHECKSUM_FILL` and
  `CODEX32_LOOKUP_FILL` in every theme. Tested in
  `faraday/faraday-core/tests/all/codex32_make.rs` against BIP 93's
  vectors 1 and 2, a press's work in
  `faraday/faraday-core/src/work/budget/codex32.rs`, the live
  translation worksheets in `faraday/faraday-keys/tests/codex32.rs`; the
  snapshot tour `codex32`. The masking of §6.4.6 is built
  (`Codex32Make::masked`): the eye, Material's icon button (on the small
  panel's bar; on the wide page in the right margin under the ?, clear of
  the cards' rows), masks every character, roll and working
  as dots and shows them again; on Faraday Desktop, the stick and the Pi the sheets are
  shown while the flow is open and masked again when the entry is left
  backwards (an earlier card, Start's Change, the entry card closed);
  Faraday Desktop masks and unmasks as the device (F117); New key keeps nothing when
  its screen is left, so there is nothing to show again. The translation
  symbols are drawn as the booklet's second alphabet's glyphs
  (`SYMBOL_GLYPHS`, §7.2). In Demo (§12) the entry runs as anywhere,
  with **Use a sample** in the "Demo only" box filling the characters left from the
  sample's shares; at the entry's Continue the set entered is dropped and
  the shell's codex32 sample, BIP 93's vector 2, stands in its place
  (`WorkSet::given` with `Making::All`: A, C and D, the sample's 2 of 3 at
  48 characters), every later card carrying "Demo puts sample NAME in
  place of these strings", no Dice tab, "Entropy · sample NAME", and Lock
  in loading the sample's key; a Demo with no sample shows none
  (`codex32_rest.rs`). Hand-made shares (the Entropy card's third way,
  §5.3) go on to Add a key's codex32 form, Make the rest pressed (§8.1).
- **Single key, backup in shares** (§6.9, D5), a kind of Make a new
  wallet (`faraday/faraday-core/src/create.rs`, `NewKind::Shares`; drawn
  in `faraday/faraday-core/src/screens/create.rs`): on Kind between
  Single key and Multisig, its line following the scheme; its Quorum the
  scheme's two steppers (2 to 9, k to 31), the Account row at its foot;
  its one slot offering Make codex32 shares only, opened with the scheme
  fixed (`Codex32Make::with_scheme`); the set's facts kept with the
  creation at Lock in (`SharesSet`: fingerprint, identifier, k, n,
  length) and shown on Check as "Backup · codex32 w0za · 2 of 3"; a key
  not made as the set refused. On chain one `wpkh` descriptor. Not in
  Restore. Tested in `faraday/faraday-core/tests/all/create_shares.rs`.
  Back up opens on the wallet as for Single key; codex32's standard way
  (§9.5) is C8's, and reads the set from `Faraday::create_shares_set`.
- **Add a key › Type codex32 strings** (C6; `faraday/faraday-core/src/forms.rs`,
  `state.rs`, `screens/entry.rs` and the compact form): one string at a
  time, typed on the codex32 keyboard (`?` on it with the tick, a
  keyboard row a touch target tall) or scanned, each checked at once; a
  string whose checksum fails is not taken and is named by its index
  ("Share C · checksum fails", "Secret · checksum fails", §8.1 step 1);
  a string with another identifier is refused
  (`forms_other_seed_string`); the same string again is "Already in";
  another at an index already in shows both, their differing boxes
  marked, with **Keep the one held** · **Replace it** (§8.5). **Check a
  string** (§8.3) on the same form: "Checksum holds" or "Checksum fails",
  what the string is, and whether it is of the set typed in; nothing is
  loaded or kept, and the string goes on leaving. The form holds the
  correction of §8.4 (C7): the tick above the field, the panel in its
  place, "Repaired, not checked" under a repaired string, and the key's
  mark kept by fingerprint on the session (`Session::key_marks`). Tested
  in `faraday/faraday-core/tests/all/codex32_forms.rs`. Once a string of
  a split set is in, the form offers **Make the rest** (§8.1 step 2) and
  **Add a share** (§8.2); pressed before the last of the k strings, the
  k strings then open Make codex32 shares on them in place of loading the
  key (`Codex32Make::given`, `forms::C32Press`). The form is reached from
  Add a key, from New key's Start card (Type codex32 strings), from Tools'
  codex32 tile, and from Make codex32 shares' Entropy card, whose **Type
  shares made by hand** (§6.3, §5.3) goes on to the form with Make the
  rest pressed and the strings in all the Scheme card's. New key's Start
  card then reads "Add a key you have · Type codex32 strings". Its first card is the
  strings it is made from and, for Make the rest, how many strings in all
  (one more than k, at least), for Add a share the index, the next unused
  one offered and any other typed on the codex32 keyboard, one typed in
  refused with "Give it an index no other string of the set has" under it;
  then Worksheets (the checksum worksheet of each string typed, in the
  verification form and marked "as typed", then each string made, and the
  secret for Make the rest, on its translation worksheet), Strings (only the strings made, each named
  by its index) and Strings made ("Entropy · your shares"), Lock in
  loading the key with the mark the strings carry, not marked as made
  here, the strings typed and made kept with it (`Keys::keep_codex32`). The arithmetic is `faraday-keys`' `WorkSet::given` (`Making::Rest`,
  `Making::Add`): exactly k strings of one set, the secret counting as
  one, every worksheet filled whole, the strings made by BIP 93's
  derivation and the secret by its recovery; never k − 1 strings and the
  seed. **Check it** keeps the string's checksum worksheet in the
  verification form (`faraday_keys::StringCheck`, whether its checksum
  holds or not): its final row is drawn under the answer beside
  `SECRETSHARE32`, "Final row differs at box 36" where it does, and
  **Show the worksheet** draws every row. A key loaded from the form
  keeps its set's plain facts (§9.4: k, the identifier, the length, each
  index typed; `forms::set_facts`, `Keys::set_codex32`) and its strings
  (`Keys::keep_codex32`); so does a key loaded from string files in Files
  (`inbox::Secret::Codex32`). How many strings of the set exist besides
  is the plan's question (§9.4), not asked on the form: the checklist's
  codex32 item for a set that exists has the stepper "How many strings
  of name are there?" (those known to 31), and the strings past those
  known are kept on the plan's form and in the key's facts as "And 1
  more, their index not known" (`codex32_backup::Press::Exist`). The share strings
  open vaults keep of the set typed in count towards k (§9.5,
  `OpenVaults::codex32_count`, `OpenVaults::codex32_with_typed`): the
  list of strings in says "1 more of this set in open vaults", Check it
  says the same of the string checked, and once those typed and the
  vaults' make k the key loads, the strings combined inside
  `faraday-keys`. A key
  carrying a correction's mark has its line beside its fingerprint on New
  key's Key card and on Wallets (each loose key, and under the wallet's
  keys held; `screens/wallets.rs`, `mark_line`). Tested in
  `faraday/faraday-core/tests/all/codex32_rest.rs` and, against BIP 93's
  vectors 2 and 3, `faraday/faraday-keys/tests/codex32.rs` (its `given`
  module and `vault_codex32` test); snapshot tour `codex32` from
  `codex32-check-final-row` on.
- **Scanning** (§10.1): in the codex32 form, Scan reads a string and adds
  it as typed (`ScanPurpose::KeyPart`); Add a key's **Scan a SeedQR**
  takes a codex32 string and opens the codex32 form with it in
  (`Faraday::seed_scan_codex32`, `faraday/faraday-core/src/scan_key.rs`);
  Home's Scan and the intake scanner send a string to that form
  (`intake_key_part`, `faraday/faraday-core/src/family/mod.rs`); a file
  of one string is a seed part in Files (`part_of`,
  `faraday/faraday-intake/src/lib.rs`; `core/faraday-codec/src/classify.rs`).
  Every picture of the Codex32QR profile's vectors is read to its stated
  fingerprint (§10.3). `pasted_secret`
  (`faraday/faraday-core/src/scan_key.rs`) refuses a pasted string where
  secrets are not taken.
- **Case** (§8.5): every path parses the text as given, so a mixed-case
  string is refused as BIP 93 says; the codex32 keyboard types one case.
  `Codex32::parse` refuses a threshold of 1 (BIP 93: 0 or 2 to 9).
- **A backup's Other paper forms** (`paper_section`,
  `faraday/faraday-core/src/screens/backup.rs`; `paper_codex32`,
  `faraday/faraday-core/src/wallet_backup.rs`; `Keys::codex32_shares`,
  `faraday/faraday-keys/src/operations.rs`; C8, §9.1): one chip,
  **codex32**, opening a choice of k (None · 2 to 9, 2 first) and n (k to
  31, 3 first) and **Make the strings**; for a key with words, the 64-byte
  BIP-39 seed as 127-character shares, from the system's randomness spread
  by `faraday-keys`, behind `Reveal::PaperForm`. The strings are drawn as
  a numbered list under the Result's rows (Split, Length, Identifier,
  Fingerprint, "Random shares · This device", and at 127 characters
  "Holds · the seed, not the words" and "Not for checking by hand"), then
  one string at a time, "String 1 of 3", each typed back on the codex32
  keyboard ("Box 17 differs", "String 1 matches") or skipped behind the
  quiz's caution, Continue to the next, and past the last "Every string
  written down" (`PaperTyping`, `faraday/faraday-core/src/codex32_backup/mod.rs`;
  `paper_one`, its `screen.rs`; tested in
  `faraday/faraday-core/tests/all/paper.rs`).
- **Backups** (C8, §9; `docs/BACKUP-FLOW.md` §17.23): the plan's codex32
  form (`faraday/faraday-core/src/plan/codex32.rs`: a seed's form, its
  strings as `What::Codex32Share`, `Codex32QrPaper`, `Codex32ShareFile` and
  `Codex32QrFile`, every check counting k distinct strings as the seed,
  The standard way for a key with a set, the Templates clause); the
  board's seed group and string chips
  (`faraday/faraday-core/src/board/codex32.rs`); the checklist's item
  **Codex32 strings of seed i**, Make them or Check my copy string by
  string, and the set's actions Add a share, Replace a lost share and
  Rotate the set (`faraday/faraday-core/src/codex32_backup/`), their
  arithmetic in `faraday-keys` (`Keys::codex32_rebuilds`, `codex32_derive`,
  `codex32_new_set`); the key's set kept as its plain facts
  (`KeyInfo::codex32`, written by `Keys::add_codex32` at Lock in and by
  Make them, kept in its vault record and read back,
  `faraday/faraday-vault/src/codex32.rs`, `docs/VAULT.md` §7 type 2 field 5
  and type 15). A seed key goes into a vault as any key does (§9.6).
  Replace a lost share needs k strings: the key's seed does not stand in
  for the secret, whose padding bits it lacks, so a share made from it
  would be of another set. Tested in
  `faraday/faraday-core/tests/all/codex32_backup.rs`,
  `faraday/faraday-keys/tests/codex32.rs` (its `set` module) and
  `faraday/faraday-vault/tests/codex32.rs`, with the vector
  `tools/vectors/vault/vault-codex32.ofv`.
- **Strings kept beyond paper** (C8-rest, §9.5): the strings of a key's
  set this session has seen (made here, typed back in the checklist's
  item, or read from a vault's share records) are kept with the key in
  `faraday-keys` (`Keys::keep_codex32`, wiped with the key, dropped when
  the set's facts change), and leave only as a file on a stick
  (`Keys::codex32_string`, `Reveal::BackupStick(Part::Codex32)`) or a
  vault's share record (`Keys::codex32_share_record`,
  `Reveal::VaultEntry(Part::Codex32)`). **Lock and write** writes a string
  the plan keeps on a stick as `codex32-{fingerprint}-{index}.txt`, the
  string and a newline, and at 48 characters its Codex32QR as
  `codex32qr-{fingerprint}-{index}.png` (upper case, alphanumeric mode, version 3,
  level L, labelled "Codex32 share C of name"), read back after the write
  against the string (`ReadBack::Codex32`); the vault item's save, and the
  flow's round, write a type 15 record per string the plan keeps in a
  vault, once. A string the session does not hold stops both, named:
  "Share C of name is not here: type it in its Codex32 strings item
  first" (`faraday/faraday-core/src/backup_flow/wallet.rs`,
  `faraday/faraday-core/src/held.rs`). At every unlock the share records
  of all open vaults are gathered (`OpenVaults::gather_codex32`,
  `faraday/faraday-keys/src/vault.rs`): a key held keeps its strings; a key
  not held loads once `k` strings of one set are in and the seed they
  make has the fingerprint the records name, its set's facts the strings
  found, "Loaded fab6868a from codex32 shares". A vault lists a share
  record under Bitcoin keys as "Share A of fab6868a", "codex32 name ·
  threshold 2". A place on the board says what its paper and sticks keep
  of each set, "1 of 2 shares of fab6868a · alone: nothing", "2 of 2
  shares of fab6868a: the seed", or "…: the seed, with a vault's
  passphrase" counting a vault whose stick it keeps; the chart's place
  node says the same where it would read nothing
  (`faraday/faraday-core/src/board/codex32.rs`, `place_lines`). Tested in
  `faraday/faraday-core/tests/all/codex32_kept.rs`. A set whose threshold is 0 is not kept as a file (its string is
  the seed: a seed file or a vault's key record keeps it).
- **A key loaded from a master seed** (codex32, SLIP-39) keeps its seed
  bytes with its master key (`Keys::add_seed`,
  `faraday/faraday-keys/src/session.rs`; C4), so `Keys::codex32_shares`
  writes any k of n again for it, as it does for a key with words.
- **The work set** (C4), `faraday/faraday-keys/src/codex32.rs`: the
  secret `WorkSet`, held in faraday-keys, lent only as a paper form
  (§13.1); `Keys::add_codex32` loads the finished set's key.
- **Demo** refuses a split with k of 2 or more (`faraday/faraday-core/src/demo.rs`,
  `docs/DEMO.md` §4 and §6) and has the BIP 93 samples
  (`docs/MODES.md` §7); Make codex32 shares puts vector 2 in place of
  the strings entered, and Make the rest and Add a share take only a
  public set, as the form does (§12).
- **The dice flow** for BIP-39 words (`faraday/faraday-core/src/keygen/mod.rs`,
  `core/faraday-entropy/src/dice.rs`) shows every entry and, for
  Dice that name words, every word as it is made.
- **Blank templates** (`docs/STARTER.md` §3.8,
  `faraday/faraday-starter/src/template.rs`, `faraday/faraday-core/src/pdf.rs`)
  print BIP-39 sheets and, from batch C9, the codex32 card and the
  booklet's worksheets and tables (§11;
  `faraday/faraday-starter/src/sheets.rs`).

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

**Dice of any number of faces up to twenty** (owner, 2026-10-24). The
booklet's tracks have pads 1 to 6 (diamonds) and, under "Die Pads
(d7+)", pads 7 to 20 (hexagons), so any die of up to twenty faces fits
the sheet; the booklet names no size and does not say which to prefer.
The comparison is the same whatever the faces: the second value lower
gives 0, higher gives 1, equal is rolled twice again, so the method's
freedom from bias does not depend on N. The Entropy card's **Dice** row
offers **d4 · d6** (ticked) **· d8 · d10 · d12 · d20**, the dice sold in
sets; the pad then takes 1 to N (a key for each face on dice other
than d6, whose six-key pad is unchanged), and a face past N is refused.
On a keyboard a face of two digits is typed as two digits; a 1 or a 2
that could begin one waits, shown as "Face 1…", for its units digit,
Space or Enter, and Backspace drops it. A ten-faced die's 0 is entered
as 10: the pad of ten-faced dice reads "0 is 10", and on a keyboard a 0
alone is 10 (the method needs only one fixed order of the faces, which
this is). One size for all five dice: mixing sizes would keep the method
correct, as each die compares with itself, but is not offered (the pad,
the roller and the sheet take one N). The dice can be changed at any
time, on the Entropy card, over "Your dice" on the entry, or in the
"Demo only" box (§5.5), one value for all three (a small panel has it on
the Entropy card only, to keep its room for the sheet and the pad); in
the middle of a character its rolls so far are cleared, as they were of
the other dice, and the note reads "Dice changed: this character's
rolls are cleared". A character keeps the faces it was rolled with, so
Backspace into it brings its rolls back on its own dice. Larger dice make a character in
fewer rolls on average: an equal pair comes up 1 time in N.

**Each die's colour** (owner, 2026-10-24). The booklet asks for five
distinct dice and five markers; the owner's d20 set came as five sets of
five, each a colour. The Entropy card's **Colours** row has a chip per
die, "Die 1 · Red", that steps through Red, Orange, Yellow, Green, Blue,
Purple, White and Black; the five start distinct (Red, Blue, Green,
Yellow, Purple). The die and its marker are drawn in its colour on the
sheet, in each die's two rolls and on the roller, always with the die's
number or name beside, so colour is never the only cue. The dice chosen
are kept for the session (New key's area), so the next Make codex32
shares starts on them; they are not in the settings file.

On the device the worksheet itself is on screen and fills as the
rolls are entered (§6.4.2): each die's first value, its second, the
comparison and the bit; then the five bits, the path through the tree,
and the character dropping into its numbered box. The entries are taken
in the booklet's order, die 1 to die 5 for the first roll and then for
the second, into an entry laid out as the worksheet's five dice and two
rolls (§5.5, "Your dice"), on every shell. How the sheet is masked is
§6.4.6.

Built: `faraday_entropy::booklet` (`BookletChar::with_faces`, faces 2
to 20, and `uniform_face`), the Entropy card's rows and the pad
(`faraday/faraday-core/src/codex32/dice_screen.rs`), tested in
`faraday/faraday-core/tests/all/codex32_dice.rs` (vector 2 rolled on
twenty-faced dice, every size reading one character, two-digit typing,
faces past the die refused, the dice changing between characters,
colours and the session) and by the cross-check's `roller` area.

`tools/reference/dice/README.md` is the model for the record of this
method: a new page under `tools/reference/` (C3) quotes the worksheet's
instructions and the tree's leaf order from SSS32.ps with the file's
SHA-256, so a later change to the booklet is seen as a change there.

### 5.2 Characters typed

The person types the characters on the codex32 keyboard (DESIGN §4.3),
for a person who made them on paper by any method. Each character goes
into its box as it is typed, masked as the dice characters are. One line of fact
stands above the entry: "Characters from dice or a worksheet, not
chosen" (owner, 2026-10-23): characters a person makes up are not
random, and a share built from them can give the seed away to someone
holding fewer than k strings. Nothing
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

On Faraday Desktop only, the dice step shows the five dice, numbered 1
to 5, the de-biasing worksheet's five tracks, as each die's two rolls
side by side (owner, 2026-10-24): under **First roll** the die's marker,
a disc in the die's colour with its first value; under **Second roll**
the die, a rounded square in its colour with its pips (dice of six faces
or fewer) or its number; after them what the pair gives, "lower · 0" or
"higher · 1"; under the five, each pair struck as equal, "Die 3 · 20
then 20 · equal · roll both again", struck through. The roller rolls the
dice chosen (§5.1): a byte of this computer's randomness below the
largest multiple of N under 256 gives the face `byte mod N + 1`, any
other byte is skipped (rejection sampling, no modulo bias; for six
faces, every byte of 252 or more, as before). The website's die
pictures are not used, so no image decoder comes into the app.

**A step at a time** (owner, 2026-10-24, replacing what one click
rolled). The computer's dice step through the work: **Previous**,
**Next** and **Play** (Pause while it runs), and ← → while the entry is
open on Faraday Desktop, the worksheet under the pointer included (Up
and Down still pan it). Next, where every step of the character so far
is shown, rolls the next character's values first, all of them, as a
click of One character rolled them (the same answers give the same
faces, an equal pair's die rolled twice again), and then shows them one
step at a time; Previous shows the state before and never rolls again,
and Next after it shows the same faces. Over the steps the Working area
beside the pad (§5.7) reads "Step 14 of 19 · character 3" and the step's
working in plain arithmetic (the card does not repeat it; owner,
2026-10-25):

| Step | It reads | The sheet marks |
|---|---|---|
| A die's first roll | "Die 2, first roll: 3. Marker on pad 3." | the pad on the die's track |
| Its second roll | "Die 2, second roll: 5. 5 is higher than 3, so bit 1." (or lower, bit 0); an equal pair "Die 4: 2 then 2. Equal, so roll both again." | the pad of the second value |
| The character | "Bits 0 1 1 0 1 = 8 + 4 + 1 = 13. Leaf 13 of the tree is D. D goes in box 10." | the tree's path and leaf (the character's whole sheet), and the box on the checksum worksheet |
| A sum of the checksum worksheet | "Box 21, row c: 3 + W = N", the cell above (or the string's character, or the residue of `ms` on the first row) and the table cell added, as the addition wheel gives it | the two cells added and the sum, the sum more heavily |
| A table row | "Boxes 17–18 read M Z. Table row MZ is …, written from box 19." | the leading cells and the row |
| A checksum box, at the last data character | "Box 36, from below: 3 + E = 7" | the cell below, the table cell between, the box |

A character's last face is held, whole, out of its box until the
character's own step puts it there, so the box is filled by the step
that says so. The steps are §7.4's working, read the same way: a
character's steps are worked out once, when its values are rolled (the
dice's places from its faces, the cells its box fills from the
worksheet's shape), and hold places, not values; drawing reads the
values from the faces and the kept worksheet. **Play runs** One pass,
One character (the default), One share or To the end, at **Speed**
Slow, Normal (the default), Fast or Fastest (a step a frame), and
reaches what stepping reaches. Any other change of the entry (a face of
the person's own dice, Backspace, a change of dice, Use a sample)
forgets the steps; the faces the walk entered stay marked as this
computer's. The person's own entry has the same steps in its Working
area (§5.7). Built in `codex32/walk.rs` and `codex32/walk_screen.rs`,
tested in `codex32/walk_tests.rs` (each step's text for vector 2's
first character, an equal pair, Previous after Next, Play against
stepping) and `tests/all/codex32_roller.rs` (the steps on Faraday
Desktop and none on the stick, the arrow keys, Play to the end).

**The buttons stay put (owner, 2026-10-24; `docs/LAYOUT.md` §16.5).**
In the card Previous, Next and Play stand in one row over Play's options,
in the narrow column one under another, and in the strip at its right
end under the step's line, so that whatever a step says, the buttons
keep their places and the press that follows lands where the last one
did. Play and Pause share the
wider's width; nothing is measured per frame but the fixed labels. Tested
in `tests/all/codex32_roller.rs` (Next's rectangle the same at every step
of a character and across characters, in the pane and in the strip).

**Both ways, side by side (owner, 2026-10-22).** The roller never
replaces the entry for real dice: on Faraday Desktop the dice step
shows the stick's entry, **Your dice**, and beside it the roller,
**Or let Faraday roll**, and the person uses either, or both, at any
point. A value from the entry is a real die's and is not labelled.

**"Demo only" (owner, 2026-10-24; moved out of the app 2026-10-25,
F118).** The roller must not look like part of the real app: on
Faraday Desktop its controls stand in the host's strip at the window's
foot, outside the app, under "Demo" (`docs/DESKTOP-HOST.md` §3.1):
Demo's **Use a sample** while a character is left, then Previous, Next
and Play (Pause while it runs), Next filled as the step's main press,
**Play options** opening a menu of how far Play runs and how fast, and
"Rolls d20", the dice chosen on the Entropy card. The entry's work (the
string's boxes, the tabs, the worksheet, the tracks and the tree) takes
the page area's whole width; the dice sheet still stands in its column
at the right where the window holds it (`docs/LAYOUT.md` §16.1). The
step's count and working are the Working area's, beside the pad. The
rolls themselves are drawn once, in the entry's input panel (owner,
2026-10-24; `docs/LAYOUT.md` §16.3). The marks are unchanged: "Rolled by
this computer", "simulated dice". The person's own entry is the dice
sheet in the page and the input panel: each die's two rolls and
**Your dice**. The strip reads the walk from `codex32/walk_screen.rs`
(`host_aids`, `host_play_options`); tested in
`tests/all/desktop_host.rs` (the walk in the strip, Next stepping it
from one place at every step, nothing demo-only in the app, Play's
options) and `tests/all/codex32_dice.rs` (none on the stick; a d20
share played and padded 1 to 20).

**Your dice, both rolls side by side** (every shell, on windows of the
expanded class and up; owner, 2026-10-24). The input panel
(`docs/LAYOUT.md` §16.3; on the dice sheet, §6.4.2, where the panel
stands in the page) shows each die's two rolls as the roller does: **First roll** (the
marker) and **Second roll** (the die) side by side, in the die's colour,
"lower · 0" or "higher · 1" once both are in, and the pairs struck as
equal under them, so a person sees each die's pair and which were
discarded and why. The values come from the pad, from the keyboard, or
from the roller, in the booklet's order (die 1 to die 5 for the first
roll, then for the second), one layout whichever way they came. Narrower
windows show the same as rows of words, "Die 3 · 4 then 2 · lower · 0"
(§6.4.7). At the window's foot the panel lays the five dice across, a
column each, with the latest two struck pairs under them; beside the
sheets, one under another as above. Backspace takes back the last value
of the character being entered. Built in `codex32/input_panel.rs`,
tested in `tests/all/codex32_input_panel.rs`.

**Honesty.** A rolled value is this computer's randomness (from
`Faraday::fresh`, as any of Faraday's randomness), not a die. The row
above the dice reads "Rolled by this computer"; each value it made is
marked so on the sheet and in its cell sheet (§7.4); the Result's
entropy row reads "Entropy · simulated dice" (or "simulated dice and
your dice" where both were used). This is the exception to §5's rule,
recorded as D4 (§14). The stick and the Pi have no roller: real dice
are typed on the pad there.

The conversion is shown live as the values land, and the checksum
worksheet's view follows the work as smoothly as the fades (§7.5). Where the screen is
too crowded for the whole codex32 sheet (§6.4.7), the dice row and the
de-biasing sheet stay in view and the checksum worksheet opens on a
press, every cell with its working (§7.4).

### 5.6 The roller elsewhere on Faraday Desktop

Since 2026-10-24 (owner) this roller is Demo's alone and stands in the
page's "Demo only" card (`docs/DEMO.md` §14), not beside the pad; what
follows describes the roller itself.

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

**The values** (both rollers). Each value comes from one byte of a
32-byte answer of `Faraday::fresh`, the answer's bytes taken from its
last to its first: a byte below 252 gives the face `byte mod 6 + 1`,
and a byte of 252 or more is skipped, so that each face has 42 of the
252 bytes kept. A new answer is asked for when the bytes of the last
one are used up; the bytes left of an answer are dropped when the
entries start over. The cross-check's `roller` area checks both
rollers against this paragraph (`tools/crosscheck/area_roller.py`).

### 5.7 The Working area: the person's own entry, a step at a time

Every shell, every mode (owner, 2026-10-24). What the Demo walk (§5.5)
shows a step at a time, the person's own entry shows too: one
**Working** area in the input panel (`docs/LAYOUT.md` §16.3), beside the
pad, says what the last value entered did and what to do next on paper.
It is the walk's one step model (the same steps, the same text, the same
marks on the sheets), with the person's faces as its source:

- each value as entered: "Die 2, first roll: 3. Marker on pad 3.",
  "Die 2, second roll: 5. 5 is higher than 3, so bit 1.", an equal pair
  "Die 4: 2 then 2. Equal, so roll both again.", with its pad marked on
  the die's track;
- the value that completes a character: "Bits 0 1 1 0 1 = 8 + 4 + 1 = 13.
  Leaf 13 of the tree is D. D goes in box 10.", the character's whole
  sheet with its path through the tree and its box marked (a character
  typed reads "D goes in box 10.");
- then, a press of **Next** at a time, the worksheet's cells that
  character fills (the sums, the table rows, at the last data character
  the checksum boxes from below), each with its inputs and its result
  outlined on the sheet and the view gliding there; **Previous** goes
  back, into the character before when the next is begun.

Over the steps it reads "Step 12 of 18 · character 7" (for the character
in progress, "Roll 6 · character 7"); at the last step, what is entered
next follows the working: "Next: die 2, first roll." Previous and Next
are Material's outlined icon buttons (the chevrons, with the tooltips
"Previous step" and "Next step"). They move only what is shown: nothing
is entered, taken back or rolled, and outside Demo nothing else offers a
step (no Play, no roll). Entering the next value is always allowed and
moves the tracker to it; Backspace moves it back with the entry. While
the Demo walk shows a step, the same two buttons step the walk.

The area's height is fixed for its width (as many lines as the longest
step needs there), so the pad beside it never moves as a step's text
changes. Beside the sheets it stands under the rolls, over the pad; at
the window's foot, after the pad or over it, right of the rolls, or
right of the codex32 keyboard for characters typed; in the
page, over the pad. On the small panel it is the context row's fourth
tile, **Working** (Dice · Checksum · String · Working, §6.4.7), over the
pad. Masked sheets (§6.4.6) mask its working.

The steps are worked out once, when a value is entered or taken back:
the last character in its box (from its record) and the one in progress
(from its faces), each with its steps, as the walk works a rolled
character out; drawing reads them and the kept worksheet. At the step
that puts a character in its box the view follows the entry as before,
to the box being filled next. Built in `codex32/tracker.rs` and
`codex32/tracker_screen.rs` on `codex32/walk.rs`'s model, tested in
`codex32/tracker_tests.rs` (each value's step, an equal pair, a character
completed with its box marked, Next through the worksheet's steps in
order with their cells marked and Previous back, a new value moving the
tracker, a typed character) and `tests/all/codex32_tracker.rs` (the step
beside the pad at 1280x800, 1920x1080 and 480x640, the small panel's
tile over the pad, outside Demo no walk and no Play, and Next entering
nothing).

**Past the last checksum: the shares made and S (owner, 2026-10-24).**
The derived shares' and S's translation worksheets fill live as before
(§6.4.4); the steps stay on the rolling and the checksums, share by
share, to the last initial share's last checksum box. There, Next opens
the first share made's tab (Share D at 2 of 3, Share E at 3 of 5), its
boxes already filled, and goes through it a step at a time, as the
booklet works it with one wheel setting per row: each initial share's
translation symbol, "Share A's translation symbol for E: μ, value 10,
from the derivation table."; then each initial share's row, box by box,
"Share A, box 23: Q × Δ = 7. Translation wheel set to Δ."; then the sum,
box by box, "Box 23, Share E: 7 + M + Q = 4"; then that share's checksum
worked as the booklet's verification (§7.1), drawn over its translation
worksheet on its tab: each sum row's cells and the table row they
select, read as on the entry's worksheet, the final row's cells "Final
row, box 36: 3 + E = S. It should read S.", and its end, "Share E's
final row reads SECRETSHARE32: its checksum holds."; then the next share
made; then S, its recovery symbols and their fusion first (§7.3), "Share
A's recovery symbols @ × Λ fused: Φ, value 15.", then its rows and its
sum (S has no checksum check: it is not written). Each step outlines its
cells (the one worked out more heavily, the symbol on the symbols' line)
and the page and the worksheet's view glide there; the Working area
reads "Step 158 of 1025 · Share E" ("158/1025 · E" on the small panel,
where the Checksum tile shows the tour's sheet). Before the tour the
last checksum box's step ends "Next: Share E."; at its last step, "Next:
the worksheets.", and Next there goes on to the Worksheets card as
Continue does. Previous goes back through the same steps, and from the
first to the last checksum box on the Checksum tab. The Demo walk's
Next and Play reach the same steps (Play stops at the tour's end; its
reach is a row, a part, a share or the end); a character taken back
unmakes the set and forgets the tour.

**Show the other shares and S as they fill**, Material's switch in a
list row under the Working area once every character is in (on by
default, kept on the flow for the session): off, Next at the last
checksum box opens nothing and a tour shown closes back to it; the tabs
still fill live. It stands only in the panel's state after the last
character, where Continue has replaced the pad, so the panel is never
taller than while rolling; at the window's foot Continue stands on the
switch's row.

The steps are worked out once, when the set is made (and dropped when
it is unmade): places, not values; each share made's checksum check is
its verification worksheet (`faraday_keys::StringCheck`), worked out
then and kept. Drawing reads them. Built in `codex32/tour.rs` and
`codex32/tour_screen.rs`, tested in `codex32/tour_tests.rs` (a share
made's box filling on its tab as soon as its inputs exist; after the
last checksum box Share D's tab with its boxes filled, the steps in the
booklet's order with their cells marked, the check, then S, then on to
the worksheets; Previous back out of it; the switch off; the Demo walk
reaching the same steps, and Play stopping at the end),
`tests/all/codex32_tour.rs` (at 1280x800, the switch offered, Next
opening Share D's tab with its first step, and a late check row brought
into view) and `work/budget/codex32.rs` (the set's last character works
the check once; a step into and through the tour works out nothing).

## 6. Make codex32 shares: the main flow

**Where**: New key's Start card (`faraday/faraday-core/src/keygen/mod.rs`,
`Start`), at its top level, not under **Other forms** (owner, §1.1 item
17). The card's choices become:

- **Make a new key**: **BIP-39 words** (ticked) · **Codex32**, side by
  side; Codex32 opens this flow.
- **Add a key you have**: **Type the words** · **Scan a SeedQR** ·
  **Type codex32 strings** (§8.1) · **Scan a Codex32QR** (§10.1).
- **Other forms** (closed): Make SLIP-39 shares · Type SLIP-39 shares ·
  Type Seed XOR parts.

BIP-39 words stays the default while few wallets other than Faraday
restore a codex32 seed; the default is to be looked at again when more
do. A threshold wallet's slot (FROST) still offers Make a new key's
BIP-39 words and Type the words only. Also reached from the codex32 tile
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

Drawn as the booklet's page draws it (SSS32.ps of 2023-03-07, page 18,
`drawDiceRow`; owner, 2026-10-24), across the card's width: each die's
two rolls side by side on the left (§5.5) and the tree of 31 nodes over
32 leaves on the right, at the booklet's proportions (a level 4.57
times the space between two leaves), the leaves
`Q P Z R Y 9 X 8 G F 2 T V D W 0 S 3 J N 5 4 K H C E 6 M U A 7 L`;
under them **Die tracks** with its rule, **Free space**, **Die pads**
and **Die pads (d7+)** where the booklet sets its heads, and five
tracks, each the die's name and colour, the free space (a circle),
pads 1 to 6 as diamonds and pads 7 to 20 as hexagons, every other one
raised, in the booklet's geometry scaled to the card (`CODEX32_TRACK_*`
tokens, each the booklet's measure over its diamond's half-diagonal).
Pads past the dice's faces are outlined only. As each entry is
pressed:

1. **First roll** (entries 1 to 5): die n's marker, a ring in its
   colour, moves from the free space to the pad of its value on track n.
2. **Second roll** (entries 6 to 10): die n is drawn on the pad of its
   second value, a square in its colour with the value, and the pair
   completes at once: "lower · 0", or "higher · 1".
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

On Faraday Desktop the Demo only card's step-through (§5.5) fills the
same sheet a step at a time: each step outlines in the accent the pad
it puts a marker or a die on, and the character's own step shows the
whole character, its five bits, the path and the leaf, until the next
step moves on to the checksum worksheet's cells.

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
  checksum worksheet (verification, §7.1) is one press away, and past
  the last checksum box the tour of §5.7 works it on the share's tab.

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

- **Faraday Desktop, the stick and the Pi** (F117: Faraday Desktop
  matches the device; it first had no masking): the sheets are **unmasked while the flow is
  open** and masked again whenever it is left (Back past the first entry
  card, Cancel, a flow put aside, the device locking or idling, Faraday Desktop closing the flow); the eye
  in the app bar masks them at any moment.

#### 6.4.7 How it fits each size class

| Class | Over the pad | One press away |
|---|---|---|
| `wide` (the pane right of the sidebar) | the share's string with every box numbered; the dice sheet across the card's width as the booklet's page (each die's two rolls and the tree, then the five tracks, §6.4.2); under it the checksum worksheet in its view (§7.5), following the entry's front cell, or the cells the Demo only card's step marks, in a timed move (§7.5) (owner, 2026-10-24: the dice sheet with its d7+ pads does not fit the side panel it had) | a share's translation rows, from a strip of the worksheets under the title; the cell sheet (§7.4) |
| `mobile` | stacked: the share's string in rows of numbered boxes; the dice sheet as five track rows ("Die 3 · 4 then 2 · lower · 0") with the tree drawn in a strip of 32 leaves and the path over it; the whole checksum worksheet in its view, panning both ways and following the front | translation rows; the cell sheet |
| `small` (the Pi panel and 240 × 320) | the pad and one context panel at a time: the dice sheet while a character is rolled, as track rows and the path as a line "0 1 1 0 · ·"; after the fifth bit, the character's box for a moment, then back to the tracks | a tile row over the pad, **Dice · Checksum · String**, swaps the context panel; the checksum panel is the whole worksheet in its view, panning both ways and following the front; the cell sheet is the whole panel as a page |

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
14). A tab row above the worksheet names every worksheet in the order
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

### 6.9 A wallet kind: Single key, backup in shares (owner, 2026-10-22)

Built (F107).

Codex32 gives a single-key wallet much of what a person reaches for
multisig to get, on the backup side: no one paper or place loses the
wallet, and fewer than k papers reveal nothing. It does not do what
multisig does at spending: to sign, the whole key is on one device. So
it is offered where the person chooses what kind of wallet to make, as
an answer of its own, each row stating plainly what it is.

**Make a new wallet's Kind card** (FLOWS, Make a new wallet; DESIGN
§4.2, Option row). The flow opens on it, open, the first row ticked
(owner, 2026-10-24, §1.1 item 24):

- **Single key, backup in shares (codex32)**, the default: one key, its
  backup in 3 shares, any 2 rebuild it (the line follows the scheme
  chosen).
- **Single key**: one key, its words on paper.
- **Multisig**: 3 keys, any 2 sign.
- **More kinds**, as today.

The ticked row pressed again opens its Quorum, as any row does (F103).
Opened by a chart's **Move the money**, the card has no row ticked:
the person chooses, and Next waits until they have (owner, 2026-10-24,
§1.1 item 25).

Right after "(codex32)" in the first row's name stands Material's info
icon button (the outlined "info" symbol, in the row's secondary ink, a
48 dp target that takes the press only on the button itself). It and
the word carry a rich tooltip (DESIGN §4.12, Tooltip): "codex32 is the
only method that makes a seed and its shares fully by hand, the
checksum included." It shows when the pointer rests on the button or
the word, when the keyboard is on the button, on a long press of
either, and at once when the button is pressed. It is the one
explanation on a working screen, by the owner's decision (F112).
Where Make codex32 shares is not offered, the row is not drawn and Single
key is the default.

The second line of each row is a fact of the row, not an explanation;
how the two compare is Learn's (§15.3). The new kind is single-sig on
chain, native SegWit, as Single key is; its descriptor is a single-key
descriptor, and a coordinator sees one xpub.

**Its cards** follow the wallet flow's (Kind · Quorum · Keys · Check ·
Back up), each doing codex32's part:

- **Quorum** asks the scheme: **How many to rebuild it?** (2 ticked, to
  9) and **How many shares?** (k to 31, 3 ticked), the steppers of
  §6.1's Other; "None, the secret alone" is not offered here (that is
  Single key with a codex32 seed, §6). The Account row follows, at the
  foot of Quorum as F98 places it.
- **Keys** has one slot; its button opens Make codex32 shares (§6) with
  the scheme already chosen, so §6.1 is skipped, starting at the
  identifier (§6.2). Lock in fills the slot and returns to Keys.
- **Check** as for Single key, with "Backup · codex32 w0za · 2 of 3" among
  its rows.
- **Back up** opens on the standard way for a codex32 key (§9.5): one
  string per place, on paper, Customize backup for the rest.

It is codex32 only (owner, §1.1 item 20); SLIP-39 stays under New key's
Other forms.

**Where codex32 is offered, and where not** (owner, §1.1 item 19):

- Make codex32 shares is offered where a single-key wallet is made: this
  kind's slot; Single key's slot and Add a key on its own, through New
  key's Start card (§6), BIP-39 words ticked there.
- A slot of a multisig, Taproot multisig, MuSig2, FROST or recovery
  wallet offers BIP-39 words only under Make a new key: two thresholds
  stacked add work and ways to go wrong for little more safety.
- A codex32 key already loaded (typed, scanned, from a vault) is refused
  nowhere: it can fill any slot a key can, as a cosigner, and its backup
  places the set it has (§9.4).

## 7. The worksheets as Faraday shows them

During entry the worksheets are the Pad screen's context (§6.4); for
review, each is a Secret screen whose panel is the worksheet's grid.
Both are DESIGN §5 screens; no new screen is added. Masking is §6.4.6.
Each box is drawn at a token's size (§13.4). A string's boxes stand in
groups of four, each with its number above it, small and in the muted
colour (owner, 2026-10-24); the checksum worksheet is drawn as the
booklet draws it (§7.1).

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

**As the booklet lays it out** (owner, 2026-10-24; its generation sheet
and its verification examples; `codex32/worksheet_screen.rs`). Columns
are the string's boxes, in groups of four with a gap between groups;
rows go in pairs with the same gap between pairs. Square cells share
their rules, as on paper. From the top:

1. **The first row**: boxes 1 to 16 (18 at 127 characters), `M S 1`
   and the first 13 characters, each bolded, its box number small in
   its top right corner.
2. **`+`** and the residue of `ms` (`3 3XW8 7RR3 YLJG` at 48 and 74
   characters), from box 4.
3. **Each step**: an **`=`** row, the sum, two boxes on from the last
   (the first from box 4), its leading cells (one or two, the cells
   that select the next row) lettered `a`, `b`, `c`, … `z`, `aa`, `ab`,
   … small in their bottom left corners, and at its end the next
   characters of the string brought down, bolded and numbered; then a
   **`+`** row, the checksum table's row the leading cells select. The
   `=` rows' cells in the checksum's columns (boxes 36 to 48 at 48
   characters) are pink, bolded ones included; `+` rows are never
   pink.
4. **`=`** and the final row, under the last `+` row, which reads
   `SECRETSHARE32`.

Characters are the booklet's capitals in the mono face. Until a
final-row cell is worked out it shows what it must read, in the muted
colour, as the verification sheet prints `SECRETSHARE32` in grey; worked
out, it is in the text colour, or, where it differs, in the error
colour with a bold error-coloured rule round it. Check a string's first
look (§8.3) is the final row alone, `=` and its 13 cells. The cell
being filled has the accent outline (§7.5); every cell with a value
opens its working (§7.4).

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
opening one computes nothing. The Demo only card's step-through (§5.5)
says the same working a cell at a time, a sum as "Box 21, row c: 3 + W =
N" (a row named by the letter of its first leading cell) and a table
row by its leading pair, read from the same kept worksheet, with the
cells it names outlined in the accent.

### 7.5 How the sheets look (owner, 2026-10-22)

The owner: the sheets match the look of the worksheets people already
know, so they feel familiar; more beautiful where that is possible; and
every colour, shade and line helps the reader follow along rather than
distracting.

**Familiar: the booklet's sheets, cell for cell.** Each worksheet keeps
the layout and the colour roles of the booklet's (SSS32.ps, the colour
edition and its black-and-white one), so a person who has filled one on
paper recognises it at a glance:

| Booklet | Role | Faraday |
|---|---|---|
| Plain squares on thin black rules, in the booklet's staircase, rows offset as the ladder works down | the grid | the same staircase and offsets, square cells sharing their rules, groups of four columns and pairs of rows set apart by a gap; rules in the theme's inner-edge colour |
| Bold-outlined squares | the given data (the person's characters, the header) | a heavier outline in the text colour |
| Pink fill (`/pink`, RGB 1, 0.9, 0.9) on the `=` rows' cells in the checksum's columns, its bolded squares among them | the checksum, worked from the bottom up | a pink fill named for the role (`CODEX32_CHECKSUM_FILL`): Faraday's checksum custom colour (THEMES §5), source #C57D81 at the booklet's hue, never harmonised, its container the fill (#FFDADA on Light, the muted rose #6F353A on Dark, at Standard contrast) and its on-container (`CODEX32_ON_CHECKSUM`) everything written on it, the characters, box numbers and letters; a string's checksum boxes the same |
| Light grey fill (0.95, 0.85), in the colour edition | rows read from a table | on the translation worksheet, a raised-surface fill (`CODEX32_LOOKUP_FILL`); none on the checksum worksheet, whose `+` and `=` rows are plain as on the owner's sheet (owner, 2026-10-24) |
| The final row, `SECRETSHARE32`, after `=` | the check | the same row after `=`; its letters muted until worked out (§7.1) |
| Small box numbers in the bolded squares' corners; small letters in the `=` rows' leading squares | finding a box, naming a cell | the same, in the muted colour at `CODEX32_CELL_NOTE` (on a checksum square, its on-colour); a character stands left of its box's number |
| The dice worksheet's five tracks (free space, diamond pads 1 to 6, hexagon pads 7 to 20), the 32-leaf tree | the dice | drawn as the booklet draws them, in its geometry: tracks left to right, the tree's leaves in its order; each die's marker and the die in the colour the person gave it (§5.1), pads past the dice's faces outlined only |
| The second alphabet (`code2`) for translation symbols | the multipliers | the same glyphs |

Names, numbers and the order of the boxes are the booklet's (§3), and so
are the box numbers in the bolded squares. A string's own boxes (§6.4.1)
carry a number each, over the box.

**More beautiful, without changing what is where.** Even spacing and
alignment on the token grid; characters set in the mono face at a size
where each box holds its character with room around it; no ornament
that carries no meaning. The checksum worksheet keeps the booklet's
square cells on shared rules (owner, 2026-10-24); a string's own boxes
keep the rounded corners the rest of Faraday uses.

**In a view that pans both ways** (owner, 2026-10-24). The checksum
worksheet is drawn in a view as wide as its column and at most
`codex32_sheet_view` tall; where the sheet is larger it pans, across
and down, through the scroll machinery every region uses (MOTION §3.3;
a region of its own, `Slot::Worksheet`, whose content also pans
sideways). A click and hold of the mouse, or a touch and hold, drags
it either way (past the touch slop, so a press on a cell still opens
its working); the wheel and a trackpad scroll it down, Shift with the
wheel across; the arrow keys pan it once the pointer is over it (where
the Demo only card's step-through is open, ← and → step it instead,
§5.5). At its foot or top a drag up or down, the wheel or a trackpad go
on to the page. During the entry the view follows the work (owner,
2026-10-24): when the front moves (a die's face, a character typed, a
step of the Demo only card or Play), the place that keeps the front
cell, or the cells the step marks, in view with the least move is
worked out once, and the view goes there in a timed move on Material's
standard curve over `motion::SHEET_FOLLOW_MS` (300 ms), its frames paced
as the fades are, each placed by the time it is drawn at; a new place
while a move is under way takes over from where the view is, at the
speed it has, so the move bends rather than jumps; under Reduce motion
the view is placed at once. A drag, the wheel or a key stops the move
where it is (`codex32/pan.rs`; MOTION §3.5). The review's tabs open at
the sheet's top left. Only the cells in the view are drawn. A trackpad's sideways
gesture does not pan it: the shells send the core no sideways scroll.

**Readability: every visual element points at the work.**
- **The cell being filled** has an accent outline, so the eye knows
  where the next value goes. Only one cell at a time is outlined;
  while the Demo only card shows a step (§5.5), the cells that step
  names are outlined instead, the one it works out more heavily, and
  the front is not.
- **A value that just landed** fades in over the motion time
  (MOTION; at once under Reduce motion), so a change is noticed without
  anything moving across the sheet.
- **Cells already done stay at full contrast.** Nothing is dimmed to
  show progress: dimmed text is harder to check against paper, which is
  what the sheet is for.
- **Lines that connect** (the tree's path, the cells a lookup used) are
  drawn only while that step is the current one, or in a cell's working
  sheet (§7.4), never all at once.
- **No shadows on the sheet**, which is one flat surface as paper is;
  the cell sheet of §7.4 keeps the ordinary sheet shadow as every sheet
  does.
- **Colour is never the only cue**: the checksum squares are also
  bolded, the given data also outlined, a failing final-row cell also
  marked with a symbol, as the booklet relies on outlines in its
  black-and-white edition. The sheets read in the high-contrast themes
  and in grey.
- **Contrast**: characters on every fill at least 4.5:1 in every theme
  (7:1 in the high-contrast themes); rules at least 3:1 against the page.
- **Rolled by this computer** (§5.5) is a small mark in the cell's
  corner, not a colour, so a mixed sheet stays one sheet.

Tokens: the fills, outlines, the radius and the spacing are named in
`core/faraday-draw/src/tokens.rs` and `faraday-ui`'s palettes (§13.4),
one value per theme. A test checks the contrast of the characters on
each fill in every theme, which is what a reader notices; the snapshot
tours render each worksheet in a light, a dark and a high-contrast theme
for the owner's look before the batch is committed.

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
- a key loaded from a master seed (codex32, SLIP-39):
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

### 9.4 A key that already has a codex32 set (owner, 2026-10-22)

A key made by Make codex32 shares has its strings on paper before any
backup is planned: they came from the person's dice. A key loaded from
k codex32 strings has a set on paper too. For such a key the backup
places and checks **the set that exists**; it never splits the seed
again, since a second split would make a second set that rebuilds the
same seed, and the plan would not know about the first.

- **What is kept.** The key record (VAULT §7, type 2) gains a field,
  the key's codex32 set, holding its plain facts only (owner,
  2026-10-23): k, the identifier, the length, and the indices known to
  exist, each marked placed, lost, exposed or destroyed as the plan
  notes them. No string is kept: the record holds the seed, and for
  k = 0 the string is the seed written out, so it can be shown again; a
  split set's strings cannot be made again from the seed alone (they
  came partly from the person's dice), and are re-derived from k of
  them (§9.7) or replaced by a new set (§9.7). A new field of the
  vault's record; the vault vectors gain a record carrying it, and a
  vault without it reads as today.
- **Made here.** Make codex32 shares writes the set with the key at
  Lock in (§6.7): k, identifier, length and every index made.
- **Loaded from strings.** Typed or scanned shares give the set's k,
  identifier, length and the indices typed; the plan asks how many
  strings exist ("How many strings of w0za are there?", k to 31) and
  which indices, where the person knows them, else "C, D and 1 more".
- **The backup plan (§9.2).** For such a key the codex32 form starts on
  and fixed to the set: n chips, one per string, labelled by index
  ("9a6a2580 · codex32 w0za · C"); k and n are not asked. Words and
  SeedQR are not offered (the key has no words); a vault holds the
  seed as for any wordless key. The checklist item **Codex32 strings of
  seed i** has no **Make them**: its foot is **Check my copy**, string
  by string, each typed back (or scanned as Codex32QR) and checked: its
  checksum, and its header against the set's facts (identifier,
  threshold, an index of the set), which catches any copying error of up
  to 8 characters ("C · checksum holds, a string of w0za" / "Box 17
  differs" where the string was just made and is still in the session);
  once k strings of the set are typed, also that they rebuild this seed.
  Or skipped behind the quiz's caution. Done when every string placed is
  checked or skipped.
- **More copies later.** **Add a share** (§8.2) extends the same set at
  an unused index, with the same identifier, and its chip is placed like
  the others.
- **A new set on purpose** is **Rotate the set** (§9.7), never a
  second split beside the first.
- **A key made of words** has no set: §9.1 and §9.2 split its 64-byte
  seed as written (127 characters, made with this device's randomness,
  "Not for checking by hand"); the plan's codex32 toggle for it carries
  the line "127 characters; Words is shorter for this key".

This follows F31 (decide where a key is kept, then make it): the set
exists before the plan, so the plan places it rather than making one.

### 9.5 Shares on the backup board (owner, 2026-10-22)

How a codex32 key's strings are placed on the board (BACKUP-FLOW §17),
whether the set exists already (§9.4) or the plan makes one (§9.2).

**The list.** Under **Seeds**, a codex32 seed is a group: its head
"9a6a2580 · codex32 w0za · 2 of 3", then one chip per string by index
(**A**, **C**, **D**), each reading how many places keep it ("2
places"), and **+ share**, which derives the next string of the same set
(§8.2) and adds its chip. A string chip never runs out: dragging places
a copy, as every list chip does.

**Each zone, each form.** A string placed in a zone is one chip there,
its index large ("w0za · **C**"), its forms as the chip's toggles
(BACKUP-FLOW §17.3):

| Zone | Forms of string j of seed i | `What` |
|---|---|---|
| Paper | the string written · its Codex32QR drawn by hand | `Codex32Share(i, j)` · `Codex32QrPaper(i, j)` |
| Stick (files) | the string as `.txt` · its Codex32QR as `.png` (48 characters; §10.4 for longer) | `Codex32ShareFile(i, j)` · `Codex32QrFile(i, j)` |
| Vault | a codex32 share record | `Codex32Share(i, j)` in the vault |

All are tagged Secret. A vault takes a string without holding the seed:
the vault gains a record type, **15 Codex32 share** (1 the string; 2 the
key's fingerprint; at most one per index per vault), beside type 13's
wallet share, with a vector; a vault without it reads as today. A vault
that holds the seed and also a string of it is allowed and warned
(below).

**Plain files are allowed** (owner, §1.1 item 18). A string with k of 2
or more reveals nothing alone, so a `.txt` or `.png` of one on a stick
is less exposed than the seed-words file the board already allows; it
carries the same warning mark as any file that is not encrypted, since
k such files together are the seed.

**What the board counts.** A place's line of fact counts distinct
indices of each set across its zones: "1 of 2 shares of 9a6a2580 ·
alone: nothing", "2 of 2: the seed". A string in a vault counts at a
place with that vault's passphrase, as a vault's seed does
(`plan::alone`). The checks (warnings, §17.5) gain:

- "Share C of 9a6a2580 is kept nowhere";
- "Home keeps 2 shares of w0za: the seed";
- "Vault 1 holds 9a6a2580 and a share of it";
- "Vault 1 holds 9a6a2580 whole: one place with its passphrase is
  enough", where a backup-in-shares key's seed (not a string) is placed
  in a vault: allowed, and named, since it sidesteps the threshold;
- "Fewer than 2 places keep shares of w0za" where losing one place
  leaves fewer than k strings anywhere;
- the files warning, for a string in a stick's files.

**Getting the seed back.** Strings from several vaults, unlocked one
after another in a session, are gathered as the codex32 form gathers
typed ones (§8.1, §8.5: same set, distinct indices); once k are in, the
key loads, with its fingerprint shown. A vault's string and a typed or
scanned paper string add up the same way. Spend from a backup, step by
step, reads this.

**The standard way** (owner, 2026-10-22: as BIP-39's, paper only, with
Customize backup). A codex32 key's backup opens on BACKUP-FLOW §17.21's
standard way, read-only until **Customize backup**: one string per
place, on paper, with the wallet description as the standard way places
it; three places for 2 of 3, so losing any one place leaves two strings
elsewhere. No vault, no stick file and no Codex32QR are in it; each is
added in Customize, as a vault and a SeedQR are for BIP-39 words, and
everything in it changes there (places, which string goes where, forms,
more strings by + share, another k and n where the plan makes the set,
§9.2; a set made already keeps its k, §9.4).

**A Codex32QR drawn by hand** (owner, §1.1 items 32, 33 and 36). For a
string kept at a place as `Codex32QrPaper`, the
checklist's item names its sheet ("Sheet · Faraday Starter, Blank
templates, codex32, 48 characters, Codex32QR", §11) and the Templates
item counts it ("1 from Blank templates, codex32, 48 characters,
Codex32QR"; "Both" where the string is also written there). The string
is shown as its Codex32QR on the copy grid of BACKUP-FLOW §17.24: the
whole code ruled in blocks of five named A to F and 1 to 6, as the sheet
names them, the block marked drawn large, the squares the sheet prints
grey, Blocks or Rows. A string made here opens on it; a string of a set
that exists is typed from its paper first, and its grid is shown once it
holds (the device keeps no string of a set it did not make). **Scan my
copy** reads the drawn copy back as that string: it holds, and checks
the string, or the box that differs is named; nothing scanned is loaded.
### 9.6 A codex32 key in a vault (owner, 2026-10-23)

A codex32 key gets into a vault as any key does: the plan's vault item
(**Save into Vault 1**, BACKUP-FLOW §17.3) where the plan places the
seed there, or **Save to vault** for anything loaded in a session. Its
key record holds the seed (VAULT §7, type 2, the seed payload) and the
key's codex32 set (§9.4), which the backup plan reads to place and check
the set's strings; a string alone goes in as a share record (§9.5) and
is gathered at unlock towards the key.

The vault gives nothing it holds special treatment (owner): its item for
a codex32 key is the item every key has, with no codex32 rows, no
strings, no SLIP-39 words and no check of its own. The strings are shown
and checked where the backup is (§6.6, the checklist's Check my copy,
§9.4); conversions to other forms are the conversion tool's (§10.5).

### 9.7 The set's actions: add, replace, rotate (owner, 2026-10-23)

A codex32 set's upkeep belongs with the backup, since it changes what
the places keep, so the plan, its board and its checks stay true. The
actions are on the set's node in the wallet's chart (`docs/history/NEW-WALLET.md`
§6, the structure chart; DESIGN §4.16's node sheet), beside where its
places are shown, and on the plan's codex32 chips. Changing a form
between standards (codex32 to SLIP-39 and the like) is not here; that is
the conversion tool (§10.5). The same three actions apply to a SLIP-39
set, with SLIP-39's arithmetic; that is the conversion tool's spec or a
later one, not this.

- **Add a share**: one more string of the same set, at an unused index:
  k strings of the set are typed or scanned (S counts as one, and the
  key's seed is S where the key is loaded), the new one is derived on
  its translation worksheet (§7.2) and placed on the board as a chip.
  The set's facts gain the index.
- **Replace a lost share**: the same, at the lost string's own index, so
  the new paper is the very string that was lost. It needs k strings of
  the set besides S, or k − 1 and the loaded key's seed. Where fewer
  survive, the set cannot be repaired; **Rotate the set** is offered.
- **Rotate the set**: a new set of the same seed, with new randomness,
  that makes the old one worthless.
  1. The person marks each old string lost, exposed or held (the plan's
     `lost` and `exposed` notes, BACKUP-FLOW §17's chart notes). An
     exposed string counts as existing somewhere, whatever happens to
     its paper: a copy may have been taken.
  2. Faraday counts the old strings that may exist out of the person's
     reach (lost and exposed). Where that count is k or more, the seed
     itself is to be treated as known: "3 strings of w0za may be in
     other hands: rotating is not enough; move the coins to a new key",
     and rotation is not offered as the remedy. Where it is under k, the
     old set dies once every held string is destroyed, since fewer than
     k can then exist anywhere: "Destroy A and D. Then at most C is
     left, and one string of a 2 of 3 is worthless."
  3. The person confirms each held string destroyed (a press per
     string, as the plan's other checklist items are confirmed); only
     then is the new set made, so two live sets never exist at once
     by the flow's doing.
  4. The new set: its k and n chosen again (the old ones ticked), its
     identifier typed again (a new one, so old and new strings cannot be
     mixed by mistake; the old identifier is refused), its initial
     shares from the person's dice (§5.1) or, since the seed already
     exists, from Faraday's randomness as §5.4 splits an existing seed.
     S is the seed, unchanged: the wallet, its addresses and its coins
     do not change.
  5. The checklist opens on the new strings to place and check (§9.4);
     the old chips show "Destroyed" and leave the board once the new set
     is placed. The set's facts are replaced by the new set's.

A rotation is shown as a worksheet like any split (§6.4): the new set
is S and k − 1 random strings, interpolated, and every cell opens.

## 10. Codex32QR

**Status of the profile.** Codex32QR/v1-48 comes from FractalEncrypt's
SeedSigner fork, branch `codex/codex32-error-correction`, commit
`9b8cbe0aaf527ebe9e85d0e90c0b79e2018126c8` (MIT), pinned in
`tools/vectors/codex32qr/README.md`. It is new and not peer-reviewed.
The full-string QR is the form the codex32 authors' discussion
describes (BlockstreamResearch/codex32 discussion 66, "CodexQR"), where
the format is being settled; Faraday draws no compact CodexQR until it
is. Faraday follows the profile for the QR itself, not the fork's
device workflow, such as its cap of 5 shares in a collection (owner,
2026-10-24). Faraday checks it against the profile's vectors and
pictures, and its own encoding against Project Nayuki's `qrcodegen.py`
(§10.3).

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
code: the string in upper case, alphanumeric mode, version 3, EC level
L, the standard quiet zone, black on white, at DESIGN §4.9's QR side,
the mode the profile's pictures use (owner, 2026-10-24; F95). Reading
takes a Codex32QR in either mode: a code drawn in byte mode, as Faraday
drew them before, still scans. It is a
secret as a SeedQR is, masked with the eye. The Starter's codex32
templates have a blank 29 × 29 grid for copying one by hand (§11). On
the backup's checklist a string drawn by hand is shown as the copy grid
of BACKUP-FLOW §17.24 instead, in blocks named as that grid is, and the
drawn copy is scanned back to check it (§9.5).

### 10.3 Tests

- Every string of the profile's §10 vectors 1 to 4 and every PNG in its
  `test_vectors/` and `img/` folders is read by Faraday's scanner path
  (`third_party/rqrr` decoding the picture) to the expected string; each
  set recovers to the stated S and the stated fingerprint (vectors 1, 3,
  4 also to the stated xpub at the stated path on testnet). The vectors
  and pictures are copied, with the commit and SHA-256 of each, into
  `tools/vectors/codex32qr/` (C1), as `tools/vectors/bip93/` is.
  This spec checked the strings of vectors 1 to 4 with BIP 93's
  reference arithmetic: every checksum holds and every S is the
  interpolation of its shares. Vector 2's xpub in the profile is 81
  characters long, cut short; the test uses its fingerprint, and the
  profile's author is told.
- A Codex32QR code Faraday draws decodes, with `third_party/rqrr`, to
  the string in upper case, at version 3 and level L; a byte-mode
  picture of a share still scans and loads its key.
- The code (`faraday_codec::qr::codex32qr`, which the Secret screen and
  the stick's `codex32qr-*.png` both use) is checked module for module
  against Project Nayuki's `qrcodegen.py` in the cross-check's `seedqr`
  area (alphanumeric mode), for every string of the profile and random 48-character
  strings, and so is the code read back from the picture Faraday writes.
  The profile's own pictures are read there from their pixels, and each
  is Faraday's code of its string module for module. Faraday picks its
  mask as Nayuki's encoder does; for 6 of the 22 pictures the profile's
  encoder chose another, and there Faraday's code with its mask swapped
  for the picture's (by `qrcodegen.py`'s masking) is the picture: the
  same data and error correction, another mask, which a reader ignores.

### 10.4 Longer strings

The profile defines 48 characters only, and says so (its §1). Faraday
shows no QR for a 74- or 127-character string and reads one only as
plain codex32 text. No profile is invented here: none for longer strings
is published, and this section takes one when its author publishes it.
Further input from the author (the duplicate rule, the error-correction
screens, a compact form) is taken the same way: as a change to this
section, with its vectors.

### 10.5 Conversions (owner, 2026-10-23)

Writing a seed in another standard's form (a codex32 seed as SLIP-39
words, and the like) is not part of this spec: it is a dedicated
conversion tool, planned, not scheduled, with its own spec. What was
worked out here for it: a codex32 seed of 16 or 32 bytes is a SLIP-39
master secret too, so it can be written as SLIP-39 words and read back
exactly; a 1 of 1 with every free choice fixed (identifier from the
fingerprint, extendable flag clear, the default iteration exponent, an
empty SLIP-39 passphrase) gives the same words every time; BIP-39 words
cannot be made for a codex32 seed (PBKDF2 is one-way). Reading SLIP-39
and codex32 back needs nothing new here: Add a key reads both.

## 11. Blank templates (the Starter)

STARTER §3.8's Menu gains a first row, **Form**: **BIP-39 words**
(ticked) · **codex32**. With codex32, the rows are:

- **Length**: 48 (ticked) · 74 · 127 characters;
- **Copy**: **String** (ticked) · **Codex32QR** · **Both** · **Four
  Codex32QR grids**, the last three at 48 only (Both: the string's card,
  then the grid's page);
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
of four, rows of eight boxes; a **Fingerprint** box, eight
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
Learn page, and is not printed on the sheets (owner, §1.1 item 6),
except that each printed page of the dice worksheet carries one small
line at its foot, "Dice worksheet after the codex32 booklet, © 2022
Blockstream, MIT." (owner, 2026-10-24, §1.1 item 37). The worksheets
always begin on a page of their own after the share card and the
Codex32QR pages, so no page that holds a backup holds the credit or a
worksheet.

Built (batch C9): the Form row, the card (`ms1` in boxes 1 to 3, the
labelled boxes, the
fingerprint, the alphabet with 0, 7, S and Z slashed), the dice
worksheet as the booklet draws it (owner, 2026-10-24; two landscape
pages, `faraday/faraday-starter/src/sheets/dice.rs`: the tree, "Die
tracks", "Die pads", "Die pads (d7+)" and one track, then four tracks
more, each the free space, diamond pads 1 to 6 and hexagon pads 7 to 20,
in the booklet's points; its instructions are Learn's, not printed;
the MIT licence allows the copy, its notice kept as above), the checksum worksheet (landscape, 48 on one page, 74 on two),
the translation worksheets at 48, the booklet's principal, derivation and
checksum tables (computed, and tested equal to `tables.json`). The
booklet's second alphabet is drawn in PDF's Symbol font and Helvetica
(the face baked into the screens is not in a PDF), and the booklet's
Greek, aleph, circled plus, diamond and heart come from the Symbol font.

**The Codex32QR sheets** (owner, §1.1 items 34 and 35). The
grid is on a page of its own, `codex32qr_page` in
`faraday/faraday-starter/src/sheets.rs`, beside the other codex32
sheets: 29 × 29 squares of 16 points (about 5.6 mm), printed at 100% so
a filled grid is a code a camera reads; the squares every version 3 code
shares (the finders, the timing lines, the alignment square and the dark
module, `codex32_backup::printed`) printed black, the same squares the
device's copy grid draws grey, so the person draws only the rest; a thin
rule round every square and a heavier one every five from the top left,
the blocks named columns 1 to 6 above and rows A to F to the left, as
the device names them; above the grid boxes 1 to 9 (`ms1` printed, then
Threshold, Identifier, Share index) and the Fingerprint's eight boxes,
so the drawn code says which string it is. **Four Codex32QR grids**
(`codex32qr_grids_page`) puts four such grids of 7-point squares (about
2.5 mm) on one page, each with its header boxes.

Compared with FractalEncrypt's printable cards (read 2026-10-24, the
sources table at the head of this document): theirs print the three finders and the alignment square black on
a 29 × 29 grid named in blocks of five (1 to 6, A to F), with the header
boxes and a fingerprint field on the QR card, and put four smaller cards
on a page. Faraday's sheet has all of that, and also prints the timing
lines and the dark module, names the blocks as the device's grid does
(so the screen and the paper agree block for block), draws the grid
larger, on A4, and leaves the separators white. No file of theirs is
copied; the layout is Faraday's own, drawn from the profile and the QR
standard, and their MIT notice stays beside the profile's files in
`tools/vectors/codex32qr/LICENSE.md`.

Saved as `faraday-blank-template-codex32-{48|74|127}-chars-{string|codex32qr|string-codex32qr|codex32qr-4}.pdf`,
with `-worksheets` before `.pdf` where they are asked for. Nothing else
changes in §3.8: no key, seed or wallet is asked for or shown.

## 12. Demo

| Flow | In Demo | Why |
|---|---|---|
| Make codex32 shares (§6) | Open, and the process runs whole: the person rolls (on the pad, or the roller in the "Demo only" box on Faraday Desktop) or types, and watches the live sheets, the conversion and the checksums as anywhere. Once the entry screen is finished, the strings are replaced by the sample, BIP 93's vector 2 (`NAME`, k = 2: shares `a`, `c` and the derived `d`), and every later card shows the sample's, with one line: "Demo puts sample NAME in place of these strings". **Use a sample**, in the "Demo only" box beside the entry (§5.5), fills vector 2's own characters for a person who would rather not roll | How the process works is the point; a seed rolled in Demo is a real seed that Demo would not hold, so it never leaves the entry screen. As Generate with dice puts `faraday` in place of the rolled words (DEMO §5.2) |
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

- `faraday-bip`: the folder `core/faraday-bip/src/codex32/` (C1):
  `mod.rs` (the codec), `worksheet.rs` (checksum rows, translation and
  recovery symbols, the code2 value order), `fresh.rs` (a set from k
  initial shares: S and the derived shares, headers written once the
  identifier is set), and `correct.rs` (the decoder of §8.4). Each
  file is among `tools/lint-secrets.sh`'s secret files.
- `faraday-entropy`: the booklet's dice method, a new module
  core/faraday-entropy/src/booklet.rs (new), one file a source like the
  others. In the secret files.
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
  (new), mod.rs (the flow's state, §6 and §8), screen.rs (its cards)
  and dice_screen.rs (the dice chooser, the pad of N faces, the
  booklet's dice sheet; each die's two rolls are in
  `fresh/roller/five_screen.rs`, which the roller draws too),
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
names in `core/faraday-draw/src/tokens.rs` (`codex32_box` and the like
for a string's boxes and the translation worksheet; `codex32_cell`,
`codex32_cell_char`, `codex32_sheet_view`, `CODEX32_SHEET_GAP`,
`CODEX32_SHEET_BOLD`, `CODEX32_CELL_NOTE` and `CODEX32_MARK_GAP` for
the checksum worksheet; `CODEX32_STRING_NUMBER` for the numbers over a
string's boxes; and the dice sheet's: `CODEX32_TRACK_*`, `CODEX32_FREE_*`,
`CODEX32_HEX_*`, the booklet's track geometry in units of its diamond's
half-diagonal; `CODEX32_TREE_LEVEL`, its level over its leaf pitch;
`ROLLER_PAIR_*`, each die's two rolls);
the PDF's in `pdf.rs`'s own constants as the SeedQR grid's are.

### 13.5 Tests of the cryptography

Each against published vectors or another implementation:

| What | Against |
|---|---|
| Strings made from initial shares, S, derived shares | BIP 93 vectors 2 and 3 (`tools/vectors/bip93/vectors.json`); the Codex32QR vectors (§10.3); BIP 93's Python reference (`ms32_interpolate`) through F79's offline cross-check, over random sets of every k and n at 48, 74 and 127 |
| Checksum worksheet rows | `volvelle-wasm`'s `checksum_worksheet.rs` (CC0), run once offline over every BIP 93 string and the Codex32QR strings at 48 and 74 characters, its cells kept as a fixture under `tools/vectors/` with the commit and SHA-256; every row also equals BIP 93's reference polymod over the same prefix (F79); the final row of every valid vector is the constant |
| Translation and recovery symbols, code2 | the booklet's derivation tables for k = 2 and k = 3 and its Recovery table (SSS32.ps, 2023-03-07), transcribed once from the PDF into a fixture |
| Dice to characters | the booklet's tree (§5.1), all 32 paths, from the `code` array, on six- and twenty-faced dice; a die with equal values is never read; F79's cross-check area `roller` over random dice of 2 to 20 faces |
| The roller's faces | every byte as a face of every die of 2 to 20 faces (each face exactly 256 div N times), known answers, and the chi-square of 60,000 faces of each die offered; the cross-check's `roller` area drives the roller itself over random answers |
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

Moved to `docs/history/CODEX32-LOG.md`.

## 15. Plan

Moved to `docs/history/CODEX32-LOG.md`.

### 15.1 Rules for the implementing agent

Moved to `docs/history/CODEX32-LOG.md`.

### 15.2 Batches

Moved to `docs/history/CODEX32-LOG.md`.

### 15.3 Edit list

Moved to `docs/history/CODEX32-LOG.md`.

### 15.4 Questions for the owner

Moved to `docs/history/CODEX32-LOG.md`.
