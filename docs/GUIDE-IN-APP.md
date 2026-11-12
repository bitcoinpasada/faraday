# The guide: one guide, in the app, opened where the person is

**Kind:** Reference
**Status:** Built (F247)

The guide replaces Learn. Wherever the person is (a flow and its
step, a screen and its part), the guide button opens the guide at the
section for that place, and the section says what that place offers
and what each choice means. There is one guide, and it is in the app: the website's guide page
says so and keeps the guides to starting a computer from the stick.
The owner's choices are F247.

The name of this document: GUIDE.md was the website's index
of walkthroughs and is now `docs/history/GUIDE.md` (§7). This document
keeps its name, so that what cites it still resolves.

## 1. Structure: by place, not by concept

The guide is one page per **place** and one section per **part** of
the place.

- A **place** is a screen (`Screen` in `faraday/faraday-core/src/lib.rs`)
  or, where one screen holds several flows, a flow: Make a new wallet,
  New key, Add a key, Restore, Back up, the backup flow, a spend, Spend
  from a backup, each tool, Vaults and a vault's contents, Files,
  Settings, Home. The Starter's places are its pages inside a task
  (`docs/STARTER.md` §3.2 to §3.4) and its destinations.
- A **part** is what the person is on inside the place: a flow's step
  (a card: Kind, Backup, Quorum, Use vaults, Sticks and vaults, Bring
  in the sticks, Vaults, Keys, Check), a tab (Settings' sections, a
  wallet's tabs), a vault's category, or the place as a whole.
- A page begins with a **lead section**, the place as a whole: what
  the place is for in one or two sentences. Its parts' sections follow
  in the order the place offers them (a flow's steps in its order).

Nothing in the guide is ordered by concept. A concept is told only in
the section of the choice that needs it, and only as far as the choice
needs it (§2).

### 1.1 Keys

Every section carries a **key**, written once in its source and never
changed after it is published (the website's addresses and the app's
lookups use it):

- `<place>` for a lead section, `<place>.<part>` for a part:
  `create`, `create.kind`, `create.backup`, `create.sticks`,
  `keygen.length`, `spend.transaction`, `settings.appearance`,
  `starter-stick.disks`. Lowercase ASCII, words joined by `-`.
- The place's name is the screen's in lowercase where the screen is the
  place (`create` for `Screen::Create`, `keygen` for `Screen::KeyGen`),
  and the flow's where a screen holds several (the GPG restore that
  `Screen::BackupFlow` shows is the place `gpg-restore`).
- The part's name is the step's constant's in lowercase (`cstep::STICKS`
  is `sticks`, `kstep::LENGTH` is `length`) or the tab's or category's
  name as the app names it.

### 1.2 From where the person is to the section

The app works out the key from what it shows, in one function in
`faraday-core` that matches on `Screen` and fails to build when a
screen is added, `guide_key` in `faraday/faraday-core/src/guide.rs`; a flow's step is named by a match on its step constants
beside them (the `cstep`, `kstep`, `rstep`, `bstep`, `mstep` and `step`
modules). It is a lookup, done when the guide is opened or the step
changes, never while drawing.

- **The open step.** In a flow, the section is the open step's: card 1
  of Make a new wallet opens `create.kind`, card 5 opens that card's
  section. A step that is closed and reopened by Change opens its own
  section, not the flow's first.
- **Fallback.** A part with no section opens its place's lead section;
  a place with no page opens the guide's contents (§4.3). The guide
  never opens empty.
- **Kinds of the same step.** Where a step offers different choices by
  what was chosen before (Keys for a single key and for a multisig),
  the section says both, each under its own bold name; there is no key
  per kind.
- **Practice.** The guide is the same in Practice. The page `mode`
  says what Practice does and does not make; where Practice dims an
  action, the place's section for it says so in one line. 

### 1.3 What holds the keys true

- A test walks every screen and every flow step (as
  `faraday/faraday-core/tests/all/pi_panel.rs` walks them) and fails
  where the key the app works out has no section and is not the
  fallback.
- The guide lint (`tools/lint-guide.py`, §8) fails a key written twice,
  a key no place reaches (the code's keys are read from the one
  function), and a `Screen` or Starter page that no page names and the
  guide's list of places still to write does not list. That list only
  shrinks.

## 2. What a section holds

- **The choices on that place**, each under its name in bold as the app
  shows it (the lint holds every bold name to an `en.rs` string): what
  choosing it does, what it makes, what it needs and what it rules out.
- **Why a step asks for something**, in one sentence, where the person
  must act on it: the Bring in the sticks step says that the vaults are
  written to the sticks in this step, so each stick must be attached
  here, and that a stick brought in later is checked before the keys.
- **A concept only as far as a choice needs it**: Words (BIP-39) and
  Shares (codex32) say what each backup is on paper and how many pieces
  rebuild the key, not the history of either standard.
- **Nothing else**: no theory beyond the choice, no reassurance, no
  advice that is not a fact of the choice, no account of how Faraday
  was built. Where a fact belongs to no place, it is About's (§5.4) or
  it is left out.

**Length.** A lead section is at most two sentences. A part's section
is at most 150 words, and one choice at most 40; a place whose section
would need more splits it into parts. The lint holds the bound.

**The text rule.** Plain statement of fact, in an encyclopedia's voice:
no metaphor, no flourish, no second person beyond the imperative of a
direction. The guide is the one place a working screen's explanation
lives; a working screen still carries labels, values and actions only
(DESIGN §2, F82's Guided walk-through and F112's tooltip remain the two
short exceptions on a screen).

The rule line in CLAUDE.md:

> Every user-facing string is in `faraday/faraday-strings/src/en.rs`,
> except the guide (`docs/guide/`, read into the app as written) and
> the self-test's check names (`core/faraday-selftest`). Text in the
> product is plain statement of fact, never mannered prose; a working
> screen carries labels, values and actions, never an explanation: what
> a place's choices mean is its guide section, what Faraday is and its
> notices are About.

## 3. The source

The guide's text is Markdown, one file a place, edited where it is read:
`docs/guide/<place>.md`, and `docs/guide/starter-<place>.md` for the
Starter's. The guide's contents, in reading order, is the guide's own
index page in `docs/guide/`. A page:

```
# Make a new wallet

**Kind:** Reference
**Status:** Checked against the app
**Screen:** `Create`
**Key:** create

One card per step, from the kind to the first addresses. The wallet is
made at **Make the wallet**.

## Bring in the sticks

**Key:** create.sticks

**Bring in the sticks** ...
```

- The header lines are the lint's;
  the app reads the title, each section's heading, key and paragraphs,
  and the bold spans, which it draws in the medium face.
- The app reads the files when it is built, (`include_str!` in `faraday/faraday-guide/src/lib.rs`),
  parsed once at first use. Nothing is generated into Rust: there is no
  second copy to keep in step.
- The pages are English, as every string is.

## 4. The button and the guide's screen

Checked against Material 3's pages on m3.material.io:

- Top app bar, Guidelines (trailing actions, the overflow menu):
  https://m3.material.io/components/top-app-bar/guidelines
- Icon buttons, standard, with a plain tooltip naming the action:
  https://m3.material.io/components/icon-buttons/guidelines
- Tooltips, plain: https://m3.material.io/components/tooltips/guidelines
- Bottom sheets: https://m3.material.io/components/bottom-sheets/guidelines
- Side sheets: https://m3.material.io/components/side-sheets/guidelines
- Supporting pane, canonical layout:
  https://m3.material.io/foundations/layout/canonical-layouts/supporting-pane
- Tabs, primary: https://m3.material.io/components/tabs/guidelines
- Lists: https://m3.material.io/components/lists/guidelines
- Navigation rail: https://m3.material.io/components/navigation-rail/guidelines
- Material Symbols, `help`: https://m3.material.io/styles/icons/overview

### 4.1 The button

**Guide** is the button's name: its tooltip and its ⋮
row read "Guide" (`tip_learn` and `bar_menu_learn`). It is Material's standard icon button with the `help` symbol
(`Icon::Help`, drawn as Font Awesome's circle-question, the icon face's form
of Material's `help`), in a 48 dp target (44 dp on the Pi's panel, F188), with no selected state.
It stands on every page that has a place in the guide (§1.2: every
page, once every place has its page; a place still to write opens the
contents), and on every step of a flow.

| Where | The button |
|---|---|
| The Pi's panel (compact, short, 268 × 358 dp) | On the bar right of the page's name, as the old ? was; where the tools and the mode pill would reach Back's target it goes into the ⋮ after **Cryptography** (LAYOUT §8, `docs/CRYPTO-LEARN.md` §8.3). Home's menu lists **Guide** where it lists Learn. |
| Compact (480 × 640, a narrow window, a 7-inch panel in portrait) | On the top app bar, by the same rule. |
| The stick at 1024 × 600 and other expanded windows; medium | On the top app bar; **Guide** in the rail under the divider, in Learn's place. |
| Large (Faraday Demo's 1280 × 800) | On the top app bar; **Guide** on the expanded rail, in Learn's place. |
| Extra-large (1600 dp and over) | On the bar and the expanded rail; it opens the supporting pane (§4.2). |

The rail's and the menu's **Guide** opens the guide exactly as the bar's
button does, at the place's section: it is no destination of its
own (it has its place in the rail and Home's menu). Its icon is `Icon::Help`.

The Guided walk-through has no link to the guide under an open step: the
button opens that step's section. Nothing here moves the step rows.

### 4.2 The guide's screen

The guide's sheet and pane (DESIGN §4.12, Explainer; LAYOUT §6):

- **Below extra-large, a sheet** over the page, as a sheet is
  (the window's width less its margins on a compact window and the Pi's
  panel; the reading column, about 70 characters, from medium up). Its
  head reads **Guide**; at its foot **Done**.
- **On extra-large, the supporting pane** at the right, 400 dp, beside
  the page, laid out in the room left of it; its header row reads
  **Guide**, with the pin and the close button.
- **What it shows.** The place's page, its title in Title L and its
  sections' headings in Title S, their text in Body M, opened scrolled
  so the place's section is at its top (the page's other sections above
  and below it, reached by scrolling). The section opened is not
  marked: it is the one at the top.
- **Tabs only for Cryptography.** Where the place offers a cryptographic
  action, primary tabs stand under the head: **Guide** and
  **Cryptography** (`docs/CRYPTO-LEARN.md` §8.2, its How tab, otherwise
  unchanged). The guide's button opens the Guide tab, the bar's
  **Cryptography** the Cryptography tab. There are no tabs per page.
- **Following the step.** While it is open, a sheet's page and a pane's
  page follow the step and the screen: a new step
  scrolls to its section; a new place opens its page.
- **Moving between sections and places.** Scrolling moves between a
  place's sections. **Contents**, a text button at the foot of the page
  (beside **Done** in the sheet), opens the contents (§4.3).
- **Back.** On the place's page, Back (the chevron, Escape, the mouse's
  back button) closes the guide (F77: the screen under it keeps its state, and Back
  over the screen reopens the sheet). On a page opened from the
  contents, Back returns to the contents, then to the place's page.
- **The pin** is the pane's (LAYOUT §6).
- **Nothing on it is acted on** but scrolling, Contents, the pages it
  lists, the tabs, the pin and closing it. It never starts or changes
  anything in the app.

### 4.3 The contents

A list of every page, in the reading order of the guide's index (§3),
under the index's group headings (Get Faraday, First start, Keys,
Backups, Wallets, Spending, Vaults, Tools, Settings, the Starter):
Material's one-line list items, the page's title, a press opening that
page at its lead section. The place the guide was opened from is
listed first under **Here**. The contents is the only way from one
place's page to another's; the guide does not link between sections.

### 4.4 Links into the guide from a working screen

A working screen opens the guide only through the button and the rail
entry, with two exceptions that name a section: Back up's and the
wallet flow's **Extra copies** (Guided only, DESIGN §4.17) opens
`backup.extra-copies`, and the Starter's Keep data opens its section.
The Starter's links to the guides for starting a computer (§6) are
the start guides' own.

## 5. Learn's place

### 5.1 The pages

Moved to `docs/history/GUIDE-IN-APP-LOG.md`.

### 5.2 The code

- **`faraday/faraday-guide`** is the guide's crate: the pages read from
  `docs/guide/`, the Cryptography blocks and the maker table.
- **`src/how.rs`** (generated by `docs/crypto/gen.py` from
  `docs/crypto/actions.toml`) is the Cryptography tab's blocks, by
  screen. It has no page per action and no page lookup: a block is
  reached from the screens that offer it (`screens`), never from a page.
  The manifest has no `learn` key; the website's `/cryptography` reader
  lists it as allowed only.
- **`src/block.rs`** is the type `how.rs` is generated into.
- **`src/construction.rs`** and `how.rs`'s `CONSTRUCTIONS` (the
  accounts of a format several actions share, F159) are drawn on the
  Cryptography tab after the blocks of the actions they belong to, and
  on `/cryptography`. A construction names its actions, not a page.
- **`MAKERS`, `guide_id`, `guide_address`, `GUIDES`** are the start
  guides' table and addresses, in **`faraday/faraday-start`**.
- **`faraday/faraday-core/src/guide.rs`** is the guide's state: it
  works out the key of §1.2 (`guide_key`); it holds the sheet, the pane,
  the pin and the How tab. A page has no ticked steps and the Spend tab
  no step-by-step row (the guide acts on nothing, §4.2).
- **`faraday/faraday-core/src/walkthrough.rs`** is Guided mode's
  walk-through (F82).

### 5.3 The strings and the screens that draw them

Moved to `docs/history/GUIDE-IN-APP-LOG.md`.

### 5.4 About

About (Settings' last section in Faraday, the ⋮'s About in the Starter)
stays what it is: the version, the facts of the build, the self-test.
It holds the notices a Learn page carried: the codex32 booklet's MIT
notice (F93). What Faraday is, for a person who asks, is its first line,
as now.

### 5.5 `docs/CRYPTO-LEARN.md`

Moved to `docs/history/GUIDE-IN-APP-LOG.md`.

### 5.6 The lints

Moved to `docs/history/GUIDE-IN-APP-LOG.md`.

## 6. The Starter

- Its destination **Guide** (`help`) lists the contents of the
  Starter's pages, then the start guides under their heading, then
  Faraday Demo's (the held app's guide).
- Its pages inside a task (What to write, Which stick, Confirm, Keep
  data, Start this computer from the stick) carry the Guide button on
  their top app bar, left of the ⋮, opening a sheet in the Starter's
  look (`faraday_ui::sheet::scroll_sheet`), as its Cryptography sheet
  is (`docs/CRYPTO-LEARN.md` §8.4), at the page's section; on a compact
  window it is in the ⋮ after Cryptography.
- Faraday Demo inside the Starter has the guide as Faraday has it.

## 7. One guide, in the app; the website points to it

The guide is the app's alone (F247). The website does not show it or
keep a second account of the screens: its `guide/` page says the guide
is in the app, opened with the Guide button at the screen and step the
person is on, and links the guides to starting a computer from the stick
(`start/`), which stay the website's. The walkthroughs, the screen
pages, GUIDE.md and the pictures are history (`docs/history/`), not
published.

## 8. What comes first and which files change

Moved to `docs/history/GUIDE-IN-APP-LOG.md`.
