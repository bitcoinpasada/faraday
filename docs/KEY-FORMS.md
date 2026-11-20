# Key forms: one registry of the forms a key comes in

**Kind:** Spec
**Status:** Specified (F299, F300); the registry crate `faraday-forms` is built (the identifier and the table of §3.2, and `scan_read::Kind::form`, which every scanner's seed branch reads through `Kind::route`), and the other lists of §2 are not yet moved onto it: seven lists still repeat what it knows by hand.
**Open:** a master xprv read by a scanner; every list moved onto the registry; the ways-in component on each screen that takes a key; copies and their checks worked out from it; the two forms of an extended private key and a master seed in hex in the registry, their screens, the Inbox's finding of a file holding one and Convert's reading of them (the key boundary reads both)

A key comes into Faraday in one of a few forms: BIP-39 words, SLIP-39
shares, codex32 strings, Seed XOR parts, a Krux code, and, by F300, an
extended private key (xprv) and a master seed in hex. Each form has the
same few facts wherever it appears: its name, how it is typed, which codes
scanned are it, whether a passphrase goes with it, what it makes (words, a
seed, a master key), what it is written as on paper and in files, and how
a copy of it is checked. This document is where those facts are described
once, the registry, and how every screen that lists forms reads them
there. The decision is F299; reading an xprv and a hex seed back is F300.

## 1. The rule

1. **One description.** A form's facts are stated in one table, the
   registry, and nowhere else. A screen, a flow or a plan that lists forms
   lists them by reading the table: as it is, filtered, or mapped into a
   type of its own.
2. **Exhaustive mappings.** Every map from a form to something else, and
   from something else to a form, is a `match` that names every form, with
   no `_` arm. A form added to the table fails to compile in every place
   that has not said what it does with it. The drift §2 lists came from
   two such arms: `Start::form` ends in `_ => None`, and `Start::typing`
   sends `Form::Krux` to the words by hand.
3. **Append only.** The table's order is its codes. `Action::EntryForm`,
   New key's `STARTS`, the guide's keys of Add a key's forms
   (`faraday/faraday-core/src/guide.rs`) and the plan's saved spellings
   carry a form by its place or its code; a new form goes at the end, and
   none is removed or moved. What a screen shows first is the screen's own
   list (§4), never the table's order.
4. **Three things kept apart.** A *form* is what the key is written in. A
   *way in* is how it reaches the screen: Scan, the word grid at 12 or 24,
   15, 18 or 21 words, another word list, a vault or a loaded key that
   holds it, From Files. An *opener* is the screen that asks for a key.
   The table describes forms; the component (§4) draws ways in; each
   opener says which forms it takes and which ways show first.

## 2. The lists today

Surveyed on the code as it stands; each is a list of forms, or of forms
crossed with how they are kept, written by hand.

| List | Where | What it lists | What it becomes |
|---|---|---|---|
| Add a key's forms | `forms::Form`, `faraday/faraday-core/src/forms.rs` | Words, SLIP-39, codex32, Seed XOR, Krux | the registry's identifier, moved down a crate (§3.1); the typing state (`Parts`, `Codex32Form`) stays here |
| New key's Start | `keygen::Start` and `STARTS`, `faraday/faraday-core/src/keygen/start.rs` | make words, make codex32, make SLIP-39, type each form, scan | `Make(Form)`, `Type(Form)` and `Scan`, the forms read from the table (§3.4) |
| Check's backup menu | `check::Backup` and `check::FORMS`, `faraday/faraday-core/src/check/mod.rs` | SLIP-39, Seed XOR, SeedQR, codex32, Krux | a filter of the table's copies that Check reads back (§5) |
| Kinds a seed scanner loads | `Kind::seed`, `faraday/faraday-core/src/scan_read.rs` | words, SeedQR, CompactSeedQR, codex32 secret | built: `Kind::form`, one map from every kind to its form, whole or a part (§3.3), and `Kind::route`, an exhaustive map from form and piece to what a scanner does |
| The Inbox's seeds | `inbox::SeedSource` and `inbox::Secret`, `faraday/faraday-core/src/inbox.rs` | words (with a passphrase file), SLIP-39, codex32, xprv, hex seed, loaded | stay their own types, since they carry Inbox places and the read secret; each maps from a form, exhaustively |
| Samples | `samples::SampleForm`, `faraday/faraday-core/src/samples/mod.rs` | words, SLIP-39, codex32, Seed XOR, master seed, aezeed | a form of the table, or the aezeed, which is no key form; its passphrase fact read from the table |
| Create's Backup card | `create::BackupForm`, `faraday/faraday-core/src/create.rs` | words, codex32 | a filter of the table: the forms Create makes a new key in |
| The backup plan's copies | `What`, `CopyForm`, `CopyForms`, `faraday/faraday-plan/src/lib.rs` | words, SeedQR, passphrase; a SeedQR file; codex32 strings and Codex32QRs on paper and in files; a Krux code drawn and as a file; Seed XOR parts; SLIP-39 shares | copies worked out from the table's outputs (§5), the saved spellings kept |
| Convert's targets | `convert::Target`, `faraday/faraday-core/src/convert/mod.rs` | master seed hex, xprv, words, SeedQR, CompactSeedQR, entropy hex, word numbers, starts, Seed XOR, SLIP-39, codex32 | stays its own type (most targets are writings of the words, not forms); which targets a key offers is read from what its form makes, and each target that is a form names it |
| What makes a master key | `faraday_keys::Material`, `faraday/faraday-keys/src/identity.rs` | words, SLIP-39 shares, a seed | stays its own type (it holds the secret); one variant for each thing the table says a form makes, and a new one for an xprv |

Drift found on this survey: Krux was added to each list by hand;
`Start::typing` maps `Form::Krux` to the words ("not a choice here");
`Kind::seed` left out a Krux code and the SLIP-39 and codex32 shares, which
each scanner handled in a branch of its own (now read from `Kind::route`);
`samples::SampleForm`
has a master seed that Add a key does not.

## 3. The registry

### 3.1 Where it lives

The table is read by `faraday-core`, and by two crates below it that do
not depend on it: `faraday-plan` (copies, §5) and `faraday-keys` (what a
form makes, for `Material`). It therefore lives in a small crate of its
own under `faraday/`, `faraday-forms`, that depends only on
`faraday-strings` (the names) and `faraday-text`. It holds the identifier
and plain facts only: no secret, no parser, no reference to a type of
`faraday-core`. `faraday-core`'s `forms` module keeps what types the
parts (`Parts`, `Codex32Form`, the word lists) and re-exports the
identifier.

What names a type of `faraday-core` stays in `faraday-core`, beside that
type, as an exhaustive map: the scan kinds (`scan_read::Kind::form`), the
camera's purpose for a check (`ScanPurpose`), the actions, and each
opener's list. Keeping the table free of them is what lets the plan and
the key boundary read it.

### 3.2 What a form says

`Form` is an enum with `ALL` in the table's order, and one method per
fact. Each method is a `match` over every form.

- **`name`**: the form's name on screen, from `faraday-strings`.
- **`code`**: its stable code (§1 point 3), carried by actions, the guide's
  keys and the plan's spellings.
- **`typing`**: how it is typed, which picks the entry Add a key draws:
  the word grid of a BIP-39 list (words); shares typed one at a time, each
  a list of words (SLIP-39); strings on the codex32 keyboard, one secret or
  shares (codex32); parts, each BIP-39 words, every one needed (Seed XOR);
  one line of text (xprv, hex seed); not typed, read from a code or a file
  only (Krux).
- **`collects`**: one piece, or a set put together (SLIP-39 shares, codex32
  shares, Seed XOR parts).
- **`passphrase`**: what goes with it once read: a BIP-39 passphrase
  (words, Seed XOR, a Krux code, whose sealed contents are words), SLIP-39's
  own (SLIP-39), or none (codex32, xprv, hex seed). A Krux code also has
  the passphrase that opens it, which is its own sheet, not this one.
- **`makes`**: what it gives the key boundary: words, SLIP-39 shares, a
  master seed (codex32, hex seed), or a master key with no seed (xprv).
  `faraday_keys::Material` has one variant for each, matched exhaustively.
- **`outputs`**: what a key of this form is written as for a backup, each
  output with its medium (paper by hand, a file on a stick) and its check
  (§5). Words: the words and a SeedQR on paper, a `.txt` and a SeedQR
  picture in files. codex32: each string and its Codex32QR on paper, each
  as `.txt` and `.png`. Seed XOR: each part's words on paper. SLIP-39: each
  share's words on paper. Krux: the code drawn by hand, and as a picture
  and a `.kef` file. xprv and hex seed: none (§6.4).
- **`made_here`**: whether Faraday makes a new key in it from randomness
  (words, codex32, SLIP-39), which New key's Start and Create's Backup card
  read.

### 3.3 Scanned kinds

`scan_read::Kind::form` maps every kind to the form it is, whole or a
part, or to none: words, a SeedQR and a CompactSeedQR are whole words; a
codex32 secret is a whole codex32 key and a codex32 share a part; a SLIP-39
share is a part; a Krux code is a whole Krux code; an xprv is a whole xprv
(F300). `Kind::route` reads it with the piece: a whole words or codex32 key
a seed scanner loads at once, a part it hands to its form's collection in
Add a key, a Krux code it opens on its passphrase sheet; `Kind::seed` is
the first. Every scanner's seed branch reads `Kind::route` (built, with no
change to what any scanner takes). A hex seed has no kind (§6.2).

Not built, and each scanner's row in `docs/SCANNING.md` §4.3 changes with
it: a master xprv shows the seed sheet with its fingerprint and no
passphrase; anything else that is no form keeps its row of §4.2.

### 3.4 Every list as a read of the table

- **New key's Start.** `Start` becomes `Make(Form)`, `Type(Form)` and
  `Scan`. Its choices are `Make` of each form `made_here`, `Scan`, and
  `Type` of each form, so a Krux code is `Type(Krux)` and opens Add a key's
  Krux page, and an xprv and a hex seed are typed choices like the others.
  Which choices show first and which under Other forms is New key's own
  list (§4.2).
- **Create's Backup card.** The forms `made_here` that Create offers
  (words, and codex32 where making codex32 shares is offered), each carried
  by its code.
- **Check's backup menu.** The copies (§5) Check reads back against a
  loaded key, each by form and output.
- **Samples.** A sample is of a form, or an aezeed. A master seed sample
  becomes the hex seed form, which Add a key now takes.
- **The Inbox.** `SeedSource` and `Secret` keep the Inbox places and the
  read secret; each variant names its form, and a form found in files that
  has no variant fails to compile there.
- **Convert.** `Target` keeps every writing Convert offers. The targets a
  loaded key offers follow from what its form makes: words give every
  target; a master seed the seed targets (master seed hex, xprv, codex32,
  SLIP-39); a master key the xprv only. `Target::form` names the form of
  each target that is one (words, Seed XOR, SLIP-39, codex32, hex seed,
  xprv), so its result is read back by that form's reader.
- **`Material`.** Its variants follow `makes`, and an xprv is passed as
  its text and decoded inside `faraday-keys`.

## 4. Ways in: one component on every screen that takes a key

One component draws the ways a key reaches a screen. The table gives it
the forms; the opener gives it which forms it takes and which ways show
first; everything the opener takes that is not shown first is under
Other forms. The owner's rule (2026-11-20):

### 4.1 What the component draws

- **Up front**: **Scan** (it switches form itself, by §3.3), **Type the
  words** with 12 and 24, **codex32** (typed and scanned), and one row for
  the one vault or loaded key that holds this key, by its fingerprint,
  where the screen knows which key it wants and one holds it.
- **Other forms**: one text button with a chevron, under the standard
  buttons and above the field. It opens its rows in place; on the small
  panel it opens a modal bottom sheet. Its rows: SLIP-39, Seed XOR, Krux
  code, xprv, hex seed, 15, 18 or 21 words, the other word lists, the other
  vaults and loaded keys, From Files.
- A screen opened for one form starts on that form. The form used last is
  not promoted.

The component's ways are an enum of its own: Scan, Words with a length or
a list, a Form, a holder (a vault or a loaded key) and Files. A Form way
is any form of the table but words, whose lengths and lists are ways of
their own.

### 4.2 Each opener

Each opener is a constant: the forms it takes and its up-front ways. Its
Other forms are the forms it takes less those shown first, worked out, so
no screen lists them by hand.

| Opener | Takes | Up front |
|---|---|---|
| Add a key | every form | Scan, the word grid with 12 and 24, codex32 |
| Check's backup menu | the copies Check reads back | SeedQR, codex32 |
| My seed makes my wallet | every form | Type, Scan, codex32, the matching source |
| Check a signer | every form | Type, Scan, codex32, the matching source |
| Spend's sign card | every form | Type, Scan, codex32, the matching source |
| Restore's slot | every form | Type, Scan, codex32, the matching source, Not here |
| Convert's From | every form, and an aezeed | the loaded keys, Add a key |
| New key's Start | every form `made_here` to make, every form to type | as New key lists it today |

A test for each opener checks what a person sees: that it offers every
form it takes, up front or under Other forms, and nothing it does not
take.

## 5. Copies and their checks

A copy is a form's output (§3.2) for one key or one piece of its set, on
one medium at one place. The plan's `What` keeps a copy as the form, the
piece, the output and the medium, rather than one hand-built variant for
each pair (today's `KefCode` and `KefCodePaper`, `Codex32QrFile` and the
rest). The spellings the plan writes into the vault's notes
(`faraday/faraday-plan/src/text.rs`: `w0`, `q0`, `file0`, `xp0.1`,
`kefp0` and the others) are kept, each the spelling of its form and
output, so every note already written reads back as the same copy; a test
reads each of today's spellings back.

A check is the output's: typed back (words, a codex32 string, a part, a
share, the xprv or hex text) or a drawn code scanned back (a SeedQR, a
CompactSeedQR, a Codex32QR, a Krux code). The four camera purposes that
check a drawn copy today (`CheckCopy`, `CheckConvert`, `CheckCodex32`,
`CheckKef` in `faraday/faraday-core/src/qr_view.rs`) become one, naming
the copy it compares; it compares and never loads, as each does today.
`CopyForm` (words, SeedQR, passphrase) is replaced by the output, and the
passphrase line, which is no form of the key, stays a copy of its own.

## 6. An xprv and a hex seed read back (F300)

### 6.1 An extended private key

- **Read** by the strict reader Faraday already has,
  `faraday_bip::xkey::decode_xpriv` (BIP-32's vector 5 rules: checksum, 78
  bytes, the version asked for, a zero parent and child number at depth
  0, a zero pad byte and a scalar in range), inside `faraday-keys`.
- **Shown**: its depth, parent fingerprint and child number, and the
  fingerprint it makes.
- **A master key only.** At depth 0 it loads as a key with no words and
  no seed. At any other depth it is refused, its three facts shown, with
  the line that it is not a master key: Faraday does not guess where in a
  wallet it sits. An origin written before it is taken only where it
  names the key's own fingerprint and no path.
- **Version.** `xprv` on Bitcoin, `tprv` on a test network; the other is
  refused by name, and so are SLIP-132's private spellings (`yprv`,
  `zprv` and their kin).
- **No passphrase.**

### 6.2 A master seed in hex

- **Read** as BIP-32's seed, 16 to 64 bytes: an even count of 32 to 128
  hex digits, either case, spaces ignored. Anything else is refused with
  its length.
- **Typed only, or scanned in its own form.** 64 hex digits are also a
  transaction id or a file's hash, and 32 or 64 may be BIP-39 entropy, so a
  hex seed has no scan kind and Scan never switches to it. It is read only
  where the person chose the hex seed form, and the form's name says it is
  the seed, not the words' entropy.
- **Loads** as a seed loads today (`Keys::add_seed`), so it has the seed's
  forms in Convert. No passphrase.

### 6.3 Where they are taken

Add a key, Convert's From, and every opener of §4.2 that takes every
form. The seed sheet shows an xprv as it shows a codex32 secret: its
fingerprint and no passphrase step.

The Inbox finds them in files on a stick as it finds words, SLIP-39 and
codex32 files: a text file whose whole text is one xprv (or tprv), or one
master seed in hex of a BIP-32 length, is listed as that form. A listed
file loads only when the person opens it, so a hex file that is a hash is
listed and never loaded by itself.

### 6.4 What a key from an xprv can do

A key loaded from an xprv holds a master key and nothing it was made
from: it signs, derives accounts and is checked like any key, and Convert
offers it its xprv only. `docs/SEED-CONVERSION.md` §1 item 6 ("no key
offers only its xprv") no longer holds for it. In the backup plan it is a
key kept outside the plan, as a key of SLIP-39 shares or a master seed is
(`Seed::shares`), and the plan makes no copies of it (owner, 2026-11-20). A key from a hex seed is a seed key like a codex32 one.

## 7. Tests

What a person would notice, against published vectors where cryptography
is touched:

- each opener offers every form it takes, and nothing more (§4.2);
- a code scanned on Add a key moves to its form, for every form that has
  a scan kind;
- each plan spelling written today reads back as the same copy (§5);
- an xprv: BIP-32's vectors 1 to 3 load and give their published
  fingerprints; every key of vector 5 is refused; a non-master key is
  refused with its depth, parent fingerprint and child number; the wrong
  network's version and a SLIP-132 private spelling are refused;
- a hex seed: BIP-32's vector seeds load and give the published master
  keys; 15 and 65 bytes and an odd digit count are refused; the same key
  checked against an implementation Faraday did not write.
