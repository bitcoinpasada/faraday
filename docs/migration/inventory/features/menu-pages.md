# Home, menu pages and search

Home and the other menu pages (Tools, Check, Wallets start, Files, Vaults, Settings), drawn on the menu page container, with the bar search and its pills. Home shows one lead card (the job to do next), first-run buttons while nothing is loaded, and wallet cards. Tools (`Screen::Catalog`) lists the tools in groups and has Find. These are the landing pages of the sidebar's tabs and of the small panel's menu.

## Screens and sheets

- Home (`Screen::Home`, `home::draw`, `faraday/faraday-core/src/screens/home.rs:~839`):
  - A search bar with three tonal buttons (New, Restore, Spend backup), on one row or stacked.
  - A lead card: one job to do next.
  - First-run buttons while nothing is loaded (`:~572-590`).
  - Wallet cards ranked by readiness (`:~478-568`).
- Jobs (`home_jobs`, `faraday/faraday-core/src/screens/home.rs:~110-310`): spend under way, backup not finished, waiting PSBTs in Files, import put off, receipt from the last stick visit, locked vault files, vault changed since written, stick waiting unvisited. Shown as Home's lead in the one-column Home, as the session pane's Waiting group (`pane_jobs`) and as a menu page's Waiting band (`band_jobs`).
- Menu page container (`faraday/faraday-core/src/screens/menu_page.rs`, `docs/MENU-PAGE.md`): the page name in the top bar; sections of tiles or blocks; cards in one or two columns; rows with icon, name, line, tags and chevron.
- Tools (`faraday/faraday-core/src/screens/catalog.rs`, `Screen::Catalog`): tiles by group (`crate::catalog::{GROUPS, TILES}`). Find filters tiles, Settings cards and guide words.
- Bar search (`faraday/faraday-core/src/screens/bar_search.rs`): the field and its pills, on the seven menu pages only.

## Actions

- Home lead card: the job's own action: `CreateWallet`, `RestoreWallet`, `Nav(Spend | SpendBackup | Files)`, `CarryOn`, `StartSpend(i)`, `Flow(FlowAction::Show(i))`, `Vault(Open(i))`, `boot_import::OPEN` (`faraday/faraday-core/src/screens/home.rs:~124-308`).
- Pills: New, Restore, Spend (`Action::CreateWallet`, `RestoreWallet`, `Nav(Screen::SpendBackup)`), or one Start button (`Action::NavMenu(MenuPress::Start)`) (`faraday/faraday-core/src/screens/bar_search.rs:150-154`).
- A menu row press opens the face's action (`faraday/faraday-core/src/screens/menu_page.rs:~419-426`).
- Tools Find: typing filters; a word hit opens the guide (`Action::GuideWord(i)`).
- Typing on a menu page goes to its search bar (`home_key`, `faraday/faraday-core/src/tools/mod.rs:~939-958`). Escape with nothing typed passes on to leave the page (`:~946-948`).

## Effects

- None outward. Draw-time memo writes: `app.card_split_keep` (`faraday/faraday-core/src/screens/menu_page.rs:~583-626`), `app.drawing.needing` (`faraday/faraday-core/src/screens/home.rs:~511-521`).
- `follow_home_find` scrolls Home to the results' foot after typing (`faraday/faraday-core/src/tools/mod.rs:~963-979`).

## State and lock

- The pages own no state.
- The search words live in the Tools area: `home_find`, and `home_find_kept` (the words kept while a pressed result is open). A lock clears `home_find_kept` and keeps `home_find` (`faraday/faraday-core/src/tools/mod.rs:1084-1112`). Every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- No secrets.

## Reach

- Opens Create, Restore, Spend from backup, Spend, Files, a vault, the boot import, a flow's Resume (`create-wallet`, `restore-wallet`, `spend-backup`, `spend-signing`, `vault-unlock`, `flow-container`).
- Uses the pane container for typed search results and row details (`pane-container`), the session pane (`session-pane`), the guide's glossary (`guide`), the Settings cards (`settings-time`), the tools (`tools`), the Check page (`check-page`).
- Bar search reaches `row_open::opened` (`faraday/faraday-core/src/screens/bar_search.rs:182`), `catalog::home_search_bar`, `compact::screen::{measured, files_stick_chip}`, `nav::{Nav, MenuPress}`.
- Search uses `faraday_guide::glossary` (`faraday/faraday-core/src/tools/search.rs:32`).

## Rules and refusals

- Home lead:
  - A lead that would open or restart a flow already shown above is skipped; Create is the fallback lead (`faraday/faraday-core/src/screens/home.rs:~20-44, ~392-403`).
  - The spend lead uses `app.carry_on()` only when the wallet is loaded; else "Continue signing" goes to `Screen::Spend` (`:~113-158`).
  - The backup job only when `backup_pending()` yields a flow (`:~161-179`).
  - Waiting PSBTs: all of them in `all` mode, else the first; fewest signatures first (`:~182-197, ~314-323`).
  - Receipt, locked-vault and stick jobs only when nothing is loaded (`:~237-308`). The stick job also needs `!holds_secret()` (`faraday/faraday-core/src/screens/home.rs:288-293`).
  - A locked vault job with a stick attached is a disabled card reading "pull the stick" (`:~358-373`).
- `nothing_loaded`: no key held, no session key, no session wallet, no open vault. Files, an attached stick and a locked vault file do not count (`faraday/faraday-core/src/screens/home.rs:379-384`).
- Wallet cards:
  - The line reads "Not backed up" when `receive_held` (`:~548-549`).
  - A single key not of the wallet spends alone, so no "k of n" (`:~551-561`, F132).
  - Ranked by readiness, then index (`:~497-506`).
- Lead card: enabled uses the secondary container, disabled the warning container (`:~719-746`); its button is `Primary` or `PrimaryOff` (`:~775-779`).
- Top row: the three buttons stack under the field when it would drop below `HOME_SEARCH_MIN` (`:~676-684`); the compact rail draws the field plus Start only (`:~651-659`). The search bar is pinned only where `above + SEARCH_BAR_H + HOME_RESULTS_ROWS * row <= window` (`:~828-831`).
- Home passes `panes: None` to `menu_page::draw`, so Home is one column; the pane container is used only for typed results and row details (`:~839-853`, `faraday/faraday-core/src/screens/menu_page.rs:~218-244`).
- Menu page container:
  - Panes only when not compact and nothing typed (`faraday/faraday-core/src/screens/menu_page.rs:~234-239`).
  - Typed results stand in the tile's place only with the search in the top bar and not compact (`:~218-224`).
- Section card columns: `fit.clamp(1, MENU_CARD_COLUMNS).min(cards)`; a page with `fixed_columns` set uses exactly one column whatever the number (the Pi's panel Home passes `Some(2)` and draws one column; faraday/faraday-core/src/screens/menu_page.rs:498-506, faraday/faraday-core/src/compact/mod.rs:868).
  - The card split picks the division of closest heights, earlier column first (`:~569-644`).
  - Tags drop where they meet the text (`:~465-489`).
  - A row that does not open uses disabled ink and takes no press (`:~420-431`). A warning face's line is `Role::Warning` (`:~459-463`).
  - Cards off screen are not drawn (`:~390-392`).
- Bar search:
  - Search and pills only on the seven menu pages: Home, Catalog, Check, WalletsStart, Files, Vaults, Settings (`MENU_PAGES`, `faraday/faraday-core/src/screens/bar_search.rs:21-29`).
  - The pills only if the field keeps `HOME_SEARCH_MIN`, else Start (`faraday/faraday-core/src/screens/bar_search.rs:150-154`).
  - Results alone only in a flow, or on Files or Vaults wide (`:~47-53`). The flow's own pill is dimmed (`:~77-84`). In a flow, search and pills are dropped where not even the field plus Start fit (`:~218-223`).
- Search (`faraday/faraday-core/src/tools/search.rs`): hits are `Tile`, then Settings `Card`, then guide glossary `Word` that hold every typed word; nothing when empty (`:~21-39`).
  - Home search only on menu pages, not under a sheet, not on the Pi's compact Home (`home_search_here`, `faraday/faraday-core/src/tools/mod.rs:~981-988`).
  - Words kept and restored around a pressed result (`faraday/faraday-core/src/tools/mod.rs:~419-432`).
  - A Settings card matches only when every word of the find is among its drawn words (`faraday/faraday-core/src/screens/settings/rows.rs:~97-104`).
- Tools page:
  - A tile's need replaces its line and is drawn as a warning (`faraday/faraday-core/src/screens/catalog.rs:~67-68, ~231-232`). The need lines come from `tile_need` (`faraday/faraday-core/src/catalog.rs:617-658`).
  - A tile that does not open is drawn disabled and takes no press (`faraday/faraday-core/src/screens/catalog.rs:~390-438`).
  - Word hits draw only `Hit::Word` (`:~92-93`); results only `Hit::Tile` (`:~223-225`); no match shows `catalog_no_tool_matches` (`:~209-218`); tags that do not fit are cut (`:~461-463`).
- Since the history fixes: on the small panel Home's name scrolls with the page, and the page scrolls far enough to show Lock and Power off in full above the Scan button.
- The Settings search matches a card's words from `drawn` (screens/settings/rows.rs:14-85), a hand-kept list. Not in it, though drawn: the keys-loaded line, the ignored-inputs line, the randomness status line, the two "remembered" count lines, the self-test result lines and "Run again", and the institute note (screens/settings.rs:218-260, 290, 410-449, 476-495, 613-615).
- Wallets start (`faraday/faraday-core/src/screens/wallets_start.rs`): five ways in (Make a new wallet, Restore an existing wallet, Spend from a backup, Add a key, Make a new key) (`:16-48`); with no wallet, no open vault, no unfinished create and no spend the ways are the page, with the first locked vault file's Unlock line (`:141-209`); on the small panel, keys in no wallet are listed after the wallets, each with a From this key button (`:96-115`, `:157-166`); wallets are ordered ready first and each says does not match, another key can spend, can spend, n of m here, or watch only (`:223-370`). The wide start pair shows four ways (not Make a new key) (`faraday/faraday-core/src/screens/wallets_pair.rs:99-113`).

## Domain calls

- `faraday/faraday-guide`: `glossary`, `guide_groups` (`faraday/faraday-core/src/screens/catalog.rs:~89`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| aa82a99f0 | On the small panel, Home's Faraday name stayed fixed while the tiles scrolled over it, and a letter showed in the gap between tiles. | The name scrolls with the page; no letter shows between tiles. |
| 216bab656 | On the small panel Home's page ended short, so Lock and Power off could not be shown in full above the Scan button. | The page scrolls far enough to show Lock and Power off in full. |

## Doubts

- Stale doc comments: `faraday/faraday-core/src/screens/home.rs:~104-109`; `faraday/faraday-core/src/screens/catalog.rs:~126-127, ~261-263`.
- Row left out as not about these pages: 857d5cf80 (the desktop host's own menu kept keys and pointer away from the app).
