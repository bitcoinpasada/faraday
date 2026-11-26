# The screen frame and navigation

What every screen shares: the frame that picks the layout (compact, held, or wide with the sidebar), sizes the page area, draws the sheet over the page and decides what a press outside a sheet does, the notices (busy line, "remove the stick", toast, boot and keyboard notices), the pinned bottom bar and floating Scan button on compact layouts, and the sidebar with its tabs. Every screen of every product is drawn through it.

## Screens and sheets

- Every screen is drawn through `dispatch_page` (`faraday/faraday-core/src/screens/mod.rs:~719-770`). Three layouts: compact (one column, with a rail or a bar), held (a holder's app, no sidebar), wide with the sidebar.
- Sheets are drawn over the page by the `match sheet` at `faraday/faraday-core/src/screens/mod.rs:~503-585`.
- Self-test failure screen: the whole screen is the failed check's name and an Exit button (`Action::PowerOff`); no sidebar, no sheet (`faraday/faraday-core/src/screens/mod.rs:~333-337, ~1016-1042`). The comment reads: "a build whose published vectors do not reproduce has nowhere to go but out".
- Page area sizing (`faraday/faraday-core/src/screens/mod.rs:~117-249`): the reading column, the work width or the old area, per screen.
  - Quick paper backup uses the reading width (`:~229-235`).
  - KeyGen goes wide only while a codex32 is made (`keygen_wide`, `:~1004-1006`).
  - Check an address uses the reading width unless the scanner is in a pane (`:~216-219`).
  - Screens with `panes::has_list` use the work width (`:~182`).
- Notices (`faraday/faraday-core/src/screens/mod.rs:~927-958`): the busy line; the "remove the stick" snackbar; the toast (also drawn under a NewInput sheet, `:~590-594`); boot and keyboard notices, drawn only when they have text (`:~623-629`).
- Pinned bottom bar on compact layouts: the page's forward action, plus Cancel at left when an aside exists (`:~884-903`). The frame is drawn again when the pin bar appears or goes (`app.pin_h.set`, then `draw_compact` again, `:~876-882`).
- Floating Scan button -> `Action::Scan` (`:~905-908`); room for it at the foot is reserved off Home only (`:~916-921`).
- Held frame: the way back is always `Nav(Screen::Home)` (`:~679-681`); a held Home draws nothing, and the holder's Home is the one seen (`:~823-824`).
- Sidebar (`faraday/faraday-core/src/screens/sidebar.rs`):
  - Tabs Home, Check, Wallets, Files, Vaults, Tools, Settings, with the Scan button under them.
  - The app name carries DEMO (online) or OFFLINE (on the device) (`:~15-21`).
  - The power button reads Quit online and Power off on the device (`:~121-127`).
  - A tag shows while the auto-lock is held (`lock_held`, `:~114-116`; tag `:~135-140`).
  - A Transfer tab above Settings only with the `demo` feature and `app.online` (`faraday/faraday-core/src/screens/sidebar.rs:63-72`).
  - Its shadow is dropped in a flow (`:~33-35`); High contrast draws an outline (`:~37-39`).

## Actions

- Sidebar tabs: `Nav(Screen::...)`; Scan: `Action::Scan`; power: `PowerAsk`.
- A press outside a sheet (`faraday/faraday-core/src/screens/mod.rs:470-498`):
  - Lock -> `NotNow`; Import -> `boot_import::LATER`; UnlockLoad -> `LoadNotNow`; OlderCopy -> `OlderBack`; SaveAlso -> `SaveAlsoCancel`; VaultPassphrase and VaultMemory -> `FormCancel`; CopyTo -> `CopyCancel`; SaveKeeps -> `KeepCancel`; Trim -> `TrimCancel`; KeyFile -> `KeyFileCancel`.
  - NewInput -> `InputIgnore(id)`, only for a device that is not the input target.
  - BoardMenu -> `MenuClose`; WalletConfirm -> `WalletCancel`; Account -> `LoadAccount(None)`; Register -> `RegisterCancel`.
  - Every other sheet -> `Cancel`.
  - No outside close for WordList, IdleWarn, Zeroing (stopped by its own Stop) and Published (only its two actions) (`faraday/faraday-core/src/screens/mod.rs:489-494`).
- Pinned bar: the page's forward action; Cancel.

## Effects

- None of its own. These files only draw and return `Action` values.
- Draw-time writes to view state (layout memos): `app.pin_h.set` (`faraday/faraday-core/src/screens/mod.rs:~876`); `app.camera.pane_forget()` when the frame is not frosted (`:~344-346`).
- Testkit only: the hook `crate::leaks::page_drawn` (`:~447-448`).

## State and lock

- Owns no state. It reads `View` and `Faraday`.
- What a lock wipes is decided by each owning area (`faraday/faraday-core/src/at_lock.rs:145-153`), and every lock ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).

## Reach

- Draws every screen and sheet: `sheets-parts`, `menu-pages`, `pane-container`, `session-pane`, `guide`, `small-panel`.
- Take-away names for the Message and KEF tool flows (`messages`, `krux-codes`).
- Demo only: the `Screen::Transfer` page area and its `drop_edge` highlight (`faraday/faraday-core/src/screens/mod.rs:~169-170, ~616-617, ~637-661`).
- `vaults_pair::draw` is called from `faraday/faraday-core/src/screens/mod.rs:749` (`vault-list-create`).
- `bar_head`, `bar_head_back`, `take_away_names`, `board_work_end`, `backup_work_end` are `pub(crate)` for other modules.

## Rules and refusals

- A failed self-test replaces the whole screen, Exit only (`faraday/faraday-core/src/screens/mod.rs:~333-337`).
- Under a sheet nothing below is reachable (`clear_hits`, `in_sheet`, `:~440-443`).
- A frosted frame skips redrawing the page (`:~338-343`).
- The busy line is not drawn under a sheet (`faraday/faraday-core/src/screens/mod.rs:931-934`).
- The "remove the stick" notice shows only when `app.stick.not_now()` is true, the stick is not empty and no sheet is up (`faraday/faraday-core/src/screens/mod.rs:935-937`).
- Take-away files (`faraday/faraday-core/src/screens/mod.rs:1047-1076`): only the Message flow (SIGN step) and the KEF tool flow (CODE step) offer them. A name is listed only when the Outbox holds the same name and SHA-256 and its `exposure()` is not `Secret`.
- The keyboard button stands over a sheet only when the sheet wants a keyboard (`:~598-602`).
- The outside-press map and the sheets with no outside close (Actions).
- Sidebar: Transfer only with `demo` and online (`faraday/faraday-core/src/screens/sidebar.rs:63-72`); Quit online, Power off on the device.
- Since the history fixes:
  - Sidebar rows are at least 48 dp tall (Material 3 navigation drawer).
  - On the Pi panel the stick pills are part of the page the sheet dims and take no press while a sheet is open.
  - The lock tag under the sidebar reads "Auto-lock off until backed up".
  - A tab press resumes a flow in progress.
  - A flow's noted heading goes in every frame after the flow is left.

## Domain calls

- None. `faraday/faraday-ui`, `core/faraday-draw` and `faraday/faraday-strings` for drawing.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 74fcfd4d5 | A toast left a dark rim at its edge as its fill thinned. | The toast fades as one surface. |
| 8b1be957c | Settings' theme chips did not take a press near their edge or between rows. | A chip takes a press on a 48 dp target. |
| 89c5c5f27 | Home's and Tools' search did not find a Settings card by a drawn word, nor show the whole card. | Search finds Settings cards by their drawn words. |
| 20002202c | On the wide layout Home could not be scrolled to its last key; a flow in progress showed no chip on Home. | Home scrolls; the chip shows. |
| 5d5967d92 | A page with nothing to scroll was pulled by a two-finger scroll. | It stays still under a pan. |
| cb040a64f | On the small panel, part of a flow's bar behind its tools kept the surface colour once scrolled. | The bar's fill is continuous. |
| c4cf85d5d | Settings' two columns came out unequal at some widths and text sizes. | The columns are about equal, order kept. |
| f69da7f3e | The compact bar lost the ? into its overflow on pages with a guide block. | The ? goes into the overflow first. |
| 83859ec2a | A narrower reading page was centred only above 1360 dp; Scan beside the rail was not at the edge. | Centred on any window. |
| 2a04b0600 | Check's tiles did not take Tools' grid width; tiles were not one height. | Same width; the tallest height. |
| 63f1d89c4 | Home's and Wallets' tiles did not take Tools' page width. | At most `TRANSFER_W`, centred. |
| 5dff9eb8e | A flow in progress's sidebar row was not lit on the destination's rounded shape. | The same shape. |
| c3be7fa06 | On the narrow rail at 1024 wide the network pill's arrow pushed Home's three pills out of view. | The arrow only with the sidebar. |
| beb9798d4 | The lock tag under the sidebar was cut off when its sentence was too long. | The text fits; shortened to "Auto-lock off until backed up". |
| ab1608356 | At 480x640 a stacked dialog button's label was drawn wider than its button. | It breaks at word breaks. |
| 535b708b9 | The boot import's Import button changed height when ticked. | It keeps the height its longest words need. |
| 2c88d94a2 | A flow's noted heading stayed after the flow was left, in some frames. | It goes in every frame. |
| c1d34f534 | On the Pi, Check's Seed row name ran under its Seed tag. | The name wraps clear of the tags. |
| c2f8fc77f | On Home, search results and details ran under the guide pane. | They keep the room the guide needs. |
| c4c1eb2dc | A resumed flow's Back, Cancel and Done led to where it was resumed, not where it was begun. | They return to the page first begun from (F77 amended). |
| bf7d68120 | A raised card drawn twice reused the first pass's frost; the page behind a sheet showed the card at its old foot. | A frame drawn again is drawn again. |
| eb0be84dd | A split sheet's row in the session pane drew no button (an entry worded with a count). | The button says Restore and does what the entry does. |
| 043eb09c0 | The search and Home's pills moved when the guide opened. | They are placed from the bar's whole room. |
| 3639d319b | A page changing under a still pointer drew a second frame for a control not offered. | No second frame. |
| d52e47f94 | Leaving a flow begun from a Home search result landed on Home with the words lost. | Back and Cancel return to the results with the words kept. |
| 9d1081270 | Letting the guide pane down moved its top to the window's top. | It keeps the page's top; its foot goes to the window's foot. |
| 0eccbc55f | From Vaults with a Spend in progress, the Wallets tab opened Wallets instead of resuming the Spend. | A tab press resumes a flow in progress. |
| fb680e1fb | Home on a window with the navigation rail drew a Check tile although the rail lists Check. | No Check tile on the rail layout; the phone-size and Pi menu keep it. |
| f3616aba7 | Sidebar rows were below 48 dp. | Sidebar rows reach 48 dp (Material 3 navigation drawer). |
| 07b61adb7 | On the 480x640 compact layout Cancel took the bar's room and crowded the page's name. | Cancel is the first row of the bar's menu on compact layouts; the name has room. |
| 728a62c09 | On the Pi panel the stick pills stayed on top of an open sheet, text scrolling under them, and took presses. | The pills are part of the page the sheet dims and take no press while a sheet is open. |
| a1bd591b0 | On the Pi a scrolled page redrew the bar and its fade for most of a step instead of moving rows. | The bar's tint comes in over the page's first few units only, so scrolling moves rows. |

## Doubts

- Which `Sheet` variant opens which drawing function is in the `match sheet` at `faraday/faraday-core/src/screens/mod.rs:~503-585`; the mapping was not recorded sheet by sheet.
- Draw-time writes (`pin_h`, `camera.pane_forget`) contradict "drawing reads and does not work things out" in spirit; whether each is real state is undecided.
- Many rows above are polish more than breakage; c4c1eb2dc is a decision change.
- Rows left out as not about the frame: 80609811b (backup end card title), 6b09697b7 (chart place lines), 06ab70678 (backup board stick head), 9448d5702 (backup board descriptor chip), 6238eae9a (wallet card backup line).
- Files not read for this feature: `faraday/faraday-core/src/screens/visit.rs`, `faraday/faraday-core/src/screens/visit_same.rs`, `faraday/faraday-core/src/screens/wallets.rs`, `faraday/faraday-core/src/screens/wallets/actions.rs`, `faraday/faraday-core/src/screens/wallets_pair.rs`, `faraday/faraday-core/src/screens/wallets_start.rs`, `faraday/faraday-core/src/screens/vaults_pair.rs` (in the same folder).
