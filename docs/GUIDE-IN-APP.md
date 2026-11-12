# The guide: one guide, in the app, opened where the person is

**Kind:** Spec
**Status:** Specified (F247); not built
**Open:** all of it: the guide's pages and their keys, the guide sheet and pane in place of Learn's, the Guide button and destination, the Starter's guide pages, Learn's pages and the website's walkthroughs taken out, the website's guide page saying the guide is in the app

Learn is replaced by the guide. Wherever the person is (a flow and its
step, a screen and its part), the guide button opens the guide at the
section for that place, and the section says what that place offers
and what each choice means. There is one guide, and it is in the app: the website's guide page
says so and keeps the guides to starting a computer from the stick.
The owner's choices are F247.

The name of this document: GUIDE.md was the website's index
of walkthroughs and is now `docs/history/GUIDE.md` (§7). When the guide is built, this document, then a Reference, keeps its
name, so that what cites it still resolves.

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
  action, the place's section for it says so in one line. The demo's
  page that Learn put first on such screens goes.

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

The rule line for CLAUDE.md, in place of today's strings line:

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
- The app reads the files when it is built, as Faraday's own Learn
  pages are read today (`include_str!` in `faraday/faraday-guide/src/lib.rs`),
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

The ? of today, renamed. **Guide** is its name: its tooltip and its ⋮
row read "Guide" (`tip_learn` and `bar_menu_learn` become the guide's
keys). It is Material's standard icon button with the `help` symbol, in
a 48 dp target (44 dp on the Pi's panel, F188), with no selected state.
It stands on every page that has a place in the guide (§1.2: every
page, once every place has its page; a place still to write opens the
contents), and on every step of a flow.

| Where | The button |
|---|---|
| The Pi's panel (compact, short, 268 × 358 dp) | On the bar right of the page's name, as the ? is; where the tools and the mode pill would reach Back's target it goes into the ⋮ after **Cryptography** (LAYOUT §8, `docs/CRYPTO-LEARN.md` §8.3). Home's menu lists **Guide** where it lists Learn. |
| Compact (480 × 640, a narrow window, a 7-inch panel in portrait) | On the top app bar, by the same rule. |
| The stick at 1024 × 600 and other expanded windows; medium | On the top app bar; **Guide** in the rail under the divider, where Learn is. |
| Large (Faraday Demo's 1280 × 800) | On the top app bar; **Guide** on the expanded rail, where Learn is. |
| Extra-large (1600 dp and over) | On the bar and the expanded rail; it opens the supporting pane (§4.2). |

The rail's and the menu's **Guide** opens the guide exactly as the bar's
button does, at the place's section: the destination Learn becomes this
entry and is no destination of its own (it keeps its place in the rail
and Home's menu). Its icon is `help`, in place of `Icon::Learn`.

The steps' rows of the stick's 1024 × 600 panel are being laid out
again elsewhere, and the Guided walk-through's "Learn: …" link they
moved goes with this change (§5.3): that frees its line in an open step
on every size, and the touch-target counts it changes are counted again
where this is built. Nothing here moves the step rows.

### 4.2 The guide's screen

The Learn sheet and pane as they are drawn today (DESIGN §4.12,
Explainer; LAYOUT §6), with the guide's contents in place of Learn's
pages:

- **Below extra-large, a sheet** over the page, as the Learn sheet is
  (the window's width less its margins on a compact window and the Pi's
  panel; the reading column, about 70 characters, from medium up). Its
  head reads **Guide**; at its foot **Done**.
- **On extra-large, the supporting pane** at the right, 400 dp, beside
  the page, laid out in the room left of it; its header row reads
  **Guide**, with the pin and the close button as today.
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
  page follow the step and the screen as Learn's do now: a new step
  scrolls to its section; a new place opens its page.
- **Moving between sections and places.** Scrolling moves between a
  place's sections. **Contents**, a text button at the foot of the page
  (beside **Done** in the sheet), opens the contents (§4.3).
- **Back.** On the place's page, Back (the chevron, Escape, the mouse's
  back button) closes the guide, as it closes the Learn sheet today
  (F77: the screen under it keeps its state, and Back over the screen
  reopens the sheet where Learn's did). On a page opened from the
  contents, Back returns to the contents, then to the place's page.
- **The pin** stays the pane's (LAYOUT §6), as today.
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
unchanged.

## 5. What happens to Learn

### 5.1 The pages

- **`docs/learn/*.md`** (the twenty-three concept pages, about 26,000
  words with Faraday's own): taken out. What a choice needs from them
  goes, rewritten to §2's bound, into the place's section that offers
  the choice; the rest is not kept. Nothing of Learn is kept as a page
  of concepts.
- **`docs/learn/faraday/*.md`** (Faraday's own pages): the same, except
  where a page is a place's: `time` becomes the pages of Set the time
  and Codes, `backup-plan-check` and `extra-copies` sections of Back up,
  `vaults` and `keys-kept` sections of Vaults and Settings, `gpg` and
  `secure-boot-keys` sections of a vault's GPG and Secure Boot
  categories, `wiping-a-stick` a section of Files' stick, `demo` the page
  `mode`, `shares` a section of the backup flow's sheets,
  `checking-written` goes (the Starter's Cryptography sheet holds its
  blocks, `docs/CRYPTO-LEARN.md` §8.4) and `boot/replace-faraday` the
  Starter's Keep data section.
- **The guides to starting a computer from the stick**
  (`docs/guide/start/`, read by `faraday-start` and published at
  `start/`, `docs/STARTER.md` §7) are directions for the computer's
  firmware, outside Faraday: they stay as they are, as the Starter's and
  the website's, and move under `docs/guide/` (a folder `start`) with the guide. The
  ids stay, so the website's addresses do.

### 5.2 The code

- **`faraday/faraday-guide`** (renamed from faraday-learn) is the guide's crate:
  the pages read from `docs/guide/`, the Cryptography blocks and the
  maker table. Learn's generated pages, its sync script and the
  `learn-export` and `learn-import` recipes are gone.
- **`src/how.rs`** (generated by `docs/crypto/gen.py` from
  `docs/crypto/actions.toml`) is the Cryptography tab's blocks, by
  screen; it **stays**. It has no page per action and no page lookup:
  a block is reached from the screens that offer it (`screens`), never
  from a page. The manifest has no `learn` key; the website's
  `/cryptography` reader lists it as allowed only.
- **`src/block.rs`** is the type `how.rs` is generated into; it
  **stays**.
- **`src/construction.rs`** and `how.rs`'s `CONSTRUCTIONS` (the
  accounts of a format several actions share, F159) **stay**: today they
  are drawn after a Learn page's sections; they are drawn on the
  Cryptography tab after the blocks of the actions they belong to, and on
  `/cryptography` as now. A construction names its actions, no longer a
  page.
- **`MAKERS`, `guide_id`, `guide_address`, `GUIDES`** (the start
  guides' table and addresses) stay. `faraday-guides` is renamed to
  `faraday-start`, so that "guide" names one thing.
- **`faraday/faraday-core/src/guide.rs`** (renamed from learn.rs) is the guide's state:
  it works out the key of §1.2 (`guide_key`); the sheet, the pane, the
  pin and the How tab stay; a page has no ticked steps and the Spend tab
  no **Spending, step by step** row (the guide acts on nothing, §4.2).
- **`faraday/faraday-core/src/walkthrough.rs`** (Guided mode's walk-through,
  F82) is renamed from guide.rs, so "guide" names one thing; its text stays
  (§5.3).

### 5.3 The strings and the screens that draw them

- **`guide_learn_link`** ("Learn: {}") and the `*_topic` functions in
  `faraday/faraday-core/src/walkthrough.rs`, `screens/create.rs` and
  `backup_flow/screen.rs`: taken out. The Guided walk-through stays on
  the open card without a link under it; the guide's button opens that
  card's section.
- **`tip_learn`, `bar_menu_learn`, `sidebar_learn`, `compact_learn`,
  `learn_pane_title`** read "Guide"; `learn_done`, `learn_close`,
  `learn_pin`, `learn_unpin` and `learn_tab_how` keep their text under
  the guide's names; `learn_spending_step` and `learn_tab_keys_kept` go;
  a key for **Contents** and one for **Here** are added.
- **The Starter's Learn strings** (`starter_row_learn`,
  `starter_guide_placeholder`, the `starter_learn_*` titles): the start
  guides' titles stay, renamed `starter_start_*`; the destination reads
  **Guide** (§6).

### 5.4 About

About (Settings' last section in Faraday, the ⋮'s About in the Starter)
stays what it is: the version, the facts of the build, the self-test.
It gains the notices a Learn page carried: the codex32 booklet's MIT
notice (F93). What Faraday is, for a person who asks, is its first line,
as now.

### 5.5 `docs/CRYPTO-LEARN.md`

Its §8.2 and §8.3 say "Learn" for the sheet and pane, and the How tab
is first among Learn's page tabs: they read "the guide", and its tabs
are Guide and Cryptography (§4.2). §3.1's "Learn's own page for the
action carries the same blocks" goes. §18's constructions are drawn as
§5.2 says. §8.7 (the guide's pictures of the bars) goes with the pictures (§7). The
document keeps its name: it is the manifest's, not Learn's.

### 5.6 The lints

- **learn-sync** (`python3 tools/learn/sync.py check` in the lint
  recipe): taken out with Learn's pages (moved to `docs/history/learn/`).
- **lint-guide** (`tools/lint-guide.py`) is kept and checks the new
  pages: bold names are strings (as now), links resolve (as now), every `Screen` and Starter page is named by one page or listed
  as still to write (as now, over the new pages), and §1.3's keys and
  §2's length bound.
- `tools/lint-docs.sh` exempted Learn's pages from the header rule; the
  exemption goes with it, and the guide's pages carry the header.

## 6. The Starter

The Starter has no ? today; its Learn destination lists the start
guides and Faraday's held Learn.

- Its destination **Learn** becomes **Guide** (`help`): the contents of
  the Starter's pages, then the start guides under their heading, then
  Faraday Demo's (the held app's guide).
- Its pages inside a task (What to write, Which stick, Confirm, Keep
  data, Start this computer from the stick) carry the Guide button on
  their top app bar, left of the ⋮, opening a sheet in the Starter's
  look (`faraday_ui::sheet::scroll_sheet`), as its Cryptography sheet
  is (`docs/CRYPTO-LEARN.md` §8.4), at the page's section; on a compact
  window it goes into the ⋮ after Cryptography.
- Faraday Demo inside the Starter has the guide as Faraday has it.

## 7. One guide, in the app; the website points to it

The guide is the app's alone (F247). The website does not show it or
keep a second account of the screens:

- **The website's `guide/` page** says the guide is in the app, opened
  with the Guide button at the screen and step the person is on, and
  links the guides to starting a computer from the stick (`start/`),
  which stay the website's. The website (its own repository) stops
  reading the walkthroughs and the screen pages.
- **The walkthroughs** (`docs/history/guide/walk/*.md`, twenty-three, about
  45,000 words) are in `docs/history/`, not published: what a task's
  steps are is the flow's own cards.
- **The screen pages** (`docs/history/guide/screens/*.md`, what each control
  does) are in `docs/history/`: each is replaced by its place's page.
- **`docs/history/GUIDE.md`** (the walkthroughs' index) is in `docs/history/`.
- **The pictures.** The 418 files in `docs/history/guide/pictures/` are the
  walkthroughs' and the screen pages': they went with them, and the
  recipe and script that drew them are out; no new set is drawn.
  The app shows no pictures: the person is looking at the screen. The
  stale pictures are not drawn again: they leave with the walkthroughs.

## 8. What comes first and which files change

The keys and their function come first (§1.2), with the test that walks
every place: every later page is written against them. Then the guide's
sheet and pane drawing the new pages in place of Learn's, the button's
and the rail's names, then the pages place by place, each written from
the screen and the Learn text it replaces, in the contents' order. Learn's
pages, its sync and the walkthroughs are taken out in the change that
writes the last place a Learn page served. The website's guide page becomes the pointer of §7 with the release
that carries the guide.

The files that change: `docs/guide/` (the pages and the index; `walk/`,
`screens/` and `pictures/` to history), Learn's pages (moved to `docs/history/learn/`;
its folder of start guides moved), `faraday/faraday-guide` (renamed; `pages/` out,
`how.rs` and `construction.rs` without `learn`), `faraday/faraday-start`
(renamed), `faraday/faraday-core/src/guide.rs`,
`faraday/faraday-core/src/walkthrough.rs`, `faraday/faraday-core/src/screens/sheets.rs`,
`faraday/faraday-core/src/screens/guide_pane.rs`, the screens that draw
"Learn:" links, `faraday/faraday-strings/src/en.rs`,
`faraday/faraday-starter` (its Learn destination and pages),
`docs/crypto/actions.toml` and `docs/crypto/gen.py` (no `learn` key),
`tools/lint-guide.py`, Learn's sync script (out), the pictures' script (out),
`justfile` and `faraday/faraday.just` (the recipes), `tools/lint-docs.sh`,
`docs/DESIGN.md` §2 and §4.12, `docs/LAYOUT.md` §3 and §6,
`docs/CRYPTO-LEARN.md` §3.1, §8 and §18, `docs/STARTER.md` §3.1d, §3.6
and §7.2, `docs/README.md`, `docs/DEVELOPING.md` where it names the
guide's recipes, and CLAUDE.md's strings line (§2).
