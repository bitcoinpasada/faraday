# The small panel

The compact layout used on small panels (the Pi's panel, phone-size windows, and the rail layout of narrow windows): Home as a menu page of tiles, a top bar with Back, the page name and an overflow (⋮) menu on every other page, the docked on-screen keyboard, the floating Scan button, and compact versions of the wallet card, the silent-payments card, the Add a key page and the Other forms sheet. It is chosen by the window's size and the device's panel.

## Screens and sheets

- Top bar on every page but Home (`bar`, `page`, `page_head`, `faraday/faraday-core/src/compact/mod.rs:~132, ~585, ~615`), unless `draws_own_bar` (`:~34-81`). `screen_name` gives the bar titles (`:~84-127`).
- Home (compact):
  - Tiles (`tiles`, `faraday/faraday-core/src/compact/mod.rs:~1043`): Check, Wallets start, Files, Vaults, Backups (only when `backup_counts` is known and > 0, `:~1093`), Tools, Guide (on the panel's menu only), Settings, `Screen::Transfer` (Demo online only). The panel's menu swaps Wallets and Check (`:~1148-1149`).
  - `first_actions` (`:~952`), `home_head` (`:~1014`), `prompts` (`:~1162`), `lock_row` (`:~983`, `LockAsk`, `PowerAsk`, the "lock held" tag), the Scan FAB (`:~879`, the list clipped above `FAB_ROOM`), `files_count` and `home_files_line` (`:~1935, ~1957`), an About walk-through (`CompactArea.about_open`, `toggle_about`, `:~1263`).
- Overflow menu (`overflow_sheet`, `faraday/faraday-core/src/compact/mod.rs:~421`; `tools`, `:~233`): Cancel, Add randomness, Your choices, Guide, keyboard, eye, Lock, the Power or leave label. Sheets `Sheet::NavMenu(Option<Overflow>)`, `Sheet::PowerMenu`, `Sheet::StartMenu`.
- Bar extras: the codex32 eye (`bar_eye`, `:~342`; `eye_state`, `:~484`), Cancel and Your choices on the bar (`bar_cancel` `:~370`, `bar_choices` `:~380`), the mode pill, the receiver mark ("Wireless keyboard" warning icon while `devices.receiver_marked()`, `:~195`).
- Docked keyboard (`Osk`, `CompactArea`, `faraday/faraday-core/src/compact/mod.rs:~1195-1433`; `Faraday` glue `:~1443-1845`). Kinds (`osk_kind`, `:~1615`): Address, Path, Hex (BackupFlow GPG paperkey), Codex32 and Codex32Unreadable, Passphrase.
- Compact screens (`faraday/faraday-core/src/compact/screen.rs`):
  - Wallet card (`wallet_card`, `:~125-441`): name, shape and source tag, key path, backup line, signatures held of needed, first receive address, buttons. With no wallet, "no wallet loaded" or the loose-keys card (`:~130-143`). A pending-spend or sign-file lead card above the name (`:~163-176`), not shown when the spend is already this wallet's or it has a PSBT waiting.
  - Silent-payments card (`silent_card`, `:~448-543`): the key fingerprint and state tag, Load from vault or Add its key, address lines, the count of labels handed out, Address labels, Show wallet.
  - Add a key page (`:~683-769`): the Words form (Scan a SeedQR, other-form chips, word keyboard `entry_words` `:~805-897`, language choice, Paste words, Use a sample, passphrase `entry_pass` `:~616-656` and `pass_field` `:~563-611`, pinned Add key `:~748-752`, Clear, Make a new key `:~760`); the Krux form (`seeds::kef::read_rows`, `:~708-709`); other forms (`entry_form`, `:~965-1045`, foot `:~1105-1159`).
  - Other forms sheet (`other_forms_sheet`, `:~773-798`): a row per form plus Close.
  - Demo only: "Demo only" offers (Use faraday chip, Paste words, Use a sample) (`:~580-584, ~869-871, ~1054-1056`).

## Actions

- Home tiles (`Action::Nav(Screen::...)`, `Action::Guide`); first-run actions (`CreateWallet`, `RestoreWallet`, `Nav(Screen::SpendBackup)`); Lock (`LockAsk`); Power (`PowerAsk`); Scan.
- Bar: Back, dots (`MenuPress::Open`), help (`Action::Guide`), keyboard (`Osk(Show | Hide)`), eye (`Codex32(C32::Eye)`), Cancel (`CancelFlow`), Your choices (`Choices(ChoicesPress::Open)`), the receiver mark, Own (`Own(fresh::page::Act::Open)`).
- Overflow rows: `MenuPress::{Tool(BarTool), Go(Screen), Way(Way::New | Restore | Spend), Power, Start}`, handled in `nav_menu` (`faraday/faraday-core/src/compact/mod.rs:~1890`). `BarTool::Restart` maps to `PowerAsk` (`:~1926`) and has no overflow row.
- Keyboard: `OskPress` (`Show`, `Hide`, `Done`, `Char`...); `Done` sends Enter and puts the keyboard away. A press on a field raises it (`osk_after`, `:~1741`).
- Back targets in `page_head`: wallet glance and spend paths back to the card; file details back to Files (`RowClose`); Explore sub-pages back to the path; Entry back to `Screen::WalletsStart`; check, decode and nonce back to where opened; `VaultAction::ItemBack`; search `Leave`; `SettingsSection(None)`; Randomness `Back`; else `Nav(Home)`.
- Wallet card: Rename, `Glance(true)` (`faraday/faraday-core/src/compact/screen.rs:~339-349`), `SpendPaths(true)` (`:~352-366`), `ScanKey(wallet)`, `Nav(Screen::Spend)`, `StartSpend(i)`, `ShowCode(Code::Receive(wallet, 0))` (`:~376-382`), `QrWallet`, `Backup` (Continue when unfinished, `:~392-399`), `NewWalletFrom` (`:~402-407`), `SignMessageWith` (`:~409-415`), `AddressCheck(Another)` (`:~418-422`), `ExploreKey(fp)` (`:~423-433`), `RemoveWallet` (Danger, `:~434-438`).
- Silent card: `VaultAction::LoadKeyOf` (`SlotOf::Bare`), `Entry(Some(fp))`, `SWallet(w)`.
- Add a key: `EntryPassphrase`, `EntryPassphraseAgain`, `PassEye`, `EntryAdd`, `EntryClear`, `KeyGen(None)`, `UseSample`, `PasteWords`, `EntryLanguages`, `EntryLanguage(i)`, `EntryOtherForms(AddAKey)`, `EntryPart`, `ScanPart`, `EntryRecover`, `TypeHere(Entry)`.

## Effects

- App state only: `self.sheet`, `sheet_offset`, the page offset (`faraday/faraday-core/src/compact/mod.rs:~1801-1802, ~1892-1916`).
- Clears focus in other areas: `vaults.set_focus(None)`, `seed_read.blur()`, `set_time.clear_focus()`, `create.clear_pass_focus()`, `guide.focus_find(false)` (`:~1730-1735`).
- Injects keys: `self.key(Key::Enter)` (`:~1836`), `self.key(k)` (`:~1842`); calls `act_inner` (`:~1905-1917`).
- No storage, clipboard, printing, camera, time or network of its own. Camera (ScanKey, ScanPart, entry scan), clipboard (PasteWords) and randomness (KeyGen) happen in the action handlers.

## State and lock

- `CompactArea` (`faraday/faraday-core/src/compact/mod.rs:~1241-1250`): `osk: Box<Osk>`, `about_open: Option<(Screen, u8)>`. It is `at_lock::Public` (`faraday/faraday-core/src/compact/mod.rs:1253`): a lock does not reset it.
- `Osk` (`:~1195-1235`): `keys` (shift and symbol layer), `manual`, `hidden`, `pressed`, `was_shown`, `reveal`, `lift`, `field_at`, `caret`, `was_field: Option<Text>` (built from debug strings of screen, sheet, vault focus and flags; an identifier for "same field", not typed text, `:~1673-1686`), `kept`, `drawn`, shape and box. No typed characters are kept; they go straight to `self.key`.
- `leave()` clears `manual` and resets `keys`; `done()` clears `manual` and sets `hidden` (`faraday/faraday-core/src/compact/mod.rs:1394-1404`).
- The compact screens own no state. They read the entry state's passphrase and typed words, which belong to Add a key (`EntryArea::lock` = `reset`, `faraday/faraday-core/src/state.rs:2766-2791`).
- Every lock ends the app: a fresh process, or, when the save fails with keys waiting, a fresh app in the same process (`faraday/faraday-core/src/vault/seal.rs:933-946`, `faraday/faraday-core/src/storage_api.rs:1167-1202`). So `CompactArea`, with `Osk.was_field`, does not outlive it.

## Reach

- Draws Home (`menu-pages`), the wallet card (`wallets`), the silent card (`silent-payments`), Add a key (`add-key`), the codex32 eye (`codex-shares`), the mode pill, the guide button (`guide`), Lock and Power (`lock`), the typing caret (`typing`).
- Reaches `crate::nav::{BarTool, MenuPress, Overflow, Way, Nav, scan_floats}`, `crate::modes::screen`, `crate::menu`, `crate::screens::{settings_open, decode_back, nothing_loaded, first_actions, menu_page, catalog, ...}`, `crate::vault::{Focus, SearchAct, VaultAction, screen::{contents_detail, unlock_back}}`, `crate::explore`, `crate::invoice::screen::back_of`, `crate::address_check::screen::back_of`, `crate::seed_check::{here, field}`, `crate::kef_tool::field`, `crate::keygen::kstep`, `crate::forms`, `crate::fresh::page::Act::Open`, `crate::row_open`, `crate::devices`, `crate::guide`.
- The compact screens reach the `crate::screens` helpers, `crate::glance`, `crate::wallet`, `crate::vault::screen::eye_at_marked`, `crate::pass_field::pair::pass_pair`, `crate::ways_in::screen`, `crate::seeds::kef::read_rows`, `crate::samples::put::offer`, `crate::create::NewKind::of`, `crate::address_check::AddressPress`, `crate::demo::PASSPHRASE`.
- Reached from `crate::drawing` (`View`) and the `Faraday` glue (`keyboard`, `nav_menu`, `osk_*`).

## Rules and refusals

- Bar and menu (`faraday/faraday-core/src/compact/mod.rs`):
  - Cancel moves into the dots menu only on `Nav::Menu` or `Nav::Bar` when `cancel_shown()` (`:~355`).
  - Lock is offered in the menu only while `app.holds_secret()` and only on `Nav::Menu` (`power_in_menu`, `faraday/faraday-core/src/compact/mod.rs:355-366, 455-462`).
  - The eye only on `Screen::KeyGen` with codex32 sheets (`:~485-489`).
  - Tool order in the overflow: help, keyboard, eye, choices; Cancel stays on the bar; pill room `NAME_BESIDE_PILL = 80.0` (`:~223, ~233-279`).
- Home:
  - The Guide tile only on `Nav::Menu` (`:~1118`). Home has no Guide tile elsewhere (F307 amended): the bar's ? is the way in; the panel's Home has no bar, so its menu keeps the row (comment at `:~1116-1117`).
  - The Check tile removed on `Nav::Rail` (`:~1150-1151`); the Backups tile removed on the rail in Home's grid (`:~807-809`).
  - First-run actions in three columns only on the rail when headings fit in two lines (`:~918, ~944-946`).
  - The Create-wallet lead only on the panel's menu; other leads dropped on the rail (`:~1176-1177`).
- Keyboard:
  - Offered (`keyboard_action`, `:~737-744`) only when `osk_offered() || shown || osk_auto()`.
  - Auto-raise (`osk_auto`, `:~1443-1535`): none while a sheet is shown, except Chart, SeedRead, KefRead, Sweep, PassphraseSeeds and the Potential, Rename, Register, KefOptions and Guide sheets (each with its own conditions).
    - Entry: the Krux code form is never typed (`:~1488`); the words form types only on the passphrase or with no keys (`:~1489-1491`); a codex32 pressed-only correction or clash takes no typing (`:~1482`).
    - Vaults: the list search takes typing; an item page only in detail (`:~1494-1499`).
    - KeyGen: only unlocked with a field focused, in the ENTER rolls box, or TYPED when not Words-with-keys (`:~1518-1528`).
    - SetTime only with a focus (`:~1503`). Explore only on Find (`:~1505-1508`).
    - Demo Transfer: `osk_auto` false, `osk_offered` true (`:~1531-1532, ~1589`).
    - Tools catalog search blocks the offer (`:~1549`). Home search takes typing on compact only (`:~1542`).
  - Shown only when `auto && !hidden` or `manual && offered` on compact (`:~1288`); on wide layouts only `manual && wanted` (`:~1290`). The wide layout drops the keyboard once nothing takes typing (`:~1317-1319`).
  - Done does not bring it back (`:~1401-1404`). A press on a non-field does not raise it (`:~1741-1753`).
  - A compact page opened by itself leaves the field unfocused (`osk_track`, `:~1337-1344, ~1703-1708`), except under a NavMenu sheet (`:~1695-1697`).
  - The search view's focus is not revealed above the keyboard (`:~1775`). The page scrolls only when the move is at least half a pixel (`:~1798-1800`). The lift is clamped to `[0, height]` (`:~1371`). `osk_height` is zero unless shown (`:~1663-1668`).
  - `osk_follow` (`:~1765`) scrolls the page to keep the field above the keyboard; the wide lift is `osk_lift_now` (`:~1812`).
- Wallet card (`faraday/faraday-core/src/compact/screen.rs`):
  - Silent wallets take the silent card instead of the signature box (`:~239-242`).
  - Scan a key is primary and enabled only when `app.may_load_keys()` (`:~294`).
  - The missing-keys line only when held signatures are below needed (`:~312, ~369-372`).
  - Another key that spends alone is tagged in the error tone (`:~281-285`).
  - The receive code is not offered when the address is held (`:~377-380`).
  - Sign a message only when `message_offered()`; Explore only when a held slot has a fingerprint (`:~423-433`).
- Silent card: the key state is Error when it differs, OK when here, else Neutral (`:~468-474`). A key not held: Load from vault when `vault_key_for` is some, else Add its key (`:~485-500`). Show wallet is not offered when `silent_wallet_held` (`:~528-535`).
- Add a key page (`faraday/faraday-core/src/compact/screen.rs`):
  - Other-form chips only if `es.other_forms`, the form is not Words, and not Codex32 with replaced input (`:~698-706`). The Krux form uses its own rows (`:~708`).
  - Add key is disabled if `es.differs()` or `pass_eye.waits()`, primary if typed (`:~741-747`).
  - Make a new key is omitted when `es.back == Restore` (`:~754-764`).
  - With no real seed allowed (`keys` none), `never_real_seed` replaces the word boxes (`:~847-850`). A form error replaces the keyed status (`:~852-855`).
  - Paste words only when `entry_paste_offered()` (`:~869-871`). Use a sample only when `entry_sample_shown()` (`:~866, ~1054`).
  - One-line forms: one button; Scan a QR code only for Xprv; a hex seed has no scanner button (`:~995-1021`). Continue or Add key is primary only when typed is not empty (`:~1012`). Codex32 found and the parts list are not for one-line forms (`:~1022-1038`); the codex32 check button only for Codex32 (`:~1041-1043`).
  - The passphrase row is omitted for Codex32 and when a flow is given (`:~1115`). Add this one is disabled when typed is empty or replaced (`:~1131-1135`). Make the key or Continue is primary only when `ready && !differs && !pass_eye.waits()` (`:~1146-1151`).
  - An empty passphrase shows a placeholder. The Demo passphrase is shown as a code field, not dots, when the chip is set and the secret equals it (`:~576-584`).
- Since the history fixes: the page scrolls to the focused field when the keyboard rises or focus moves; Cancel is the first row of the bar's menu on compact layouts.

## Domain calls

- None directly. `core/faraday-bip` types (`silent_wallet::SilentWallet`, `keys::Fingerprint`, `bip39::Language`), `faraday/faraday-forms` (`Typing::OneLine`) and `core/faraday-shell-api` (`Key`) are read; the `wallet` and `forms` wrappers make the domain calls.
- `faraday/faraday-ui` (`nav`, `keyboard`, `sheet`, `Status`), `core/faraday-draw` (`tokens`, `widgets::{Icon, keyboard::KeyboardKind}`), `core/faraday-text`.

## Fixtures

None. (`crate::demo::PASSPHRASE` and `crate::ABOUT_PAGE` are constants in code.)

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| cb8cda00f | On a small panel the field being typed in could be hidden behind the on-screen keyboard. | The page scrolls to the focused field when the keyboard rises or focus moves. |

## Doubts

- `flow_bar_tools` may overlap the pill when tools fill the width (`faraday/faraday-core/src/compact/mod.rs:~181`).
- The call site of `toggle_about` was not found in the code read.
- Which `Screen` variant hosts the compact Add a key page is not visible in `faraday/faraday-core/src/compact/screen.rs`; whether `entry_sample_shown()` includes "field empty" is not known.
- cb8cda00f is mixed with a new keyboard behaviour in the same commit.
- Related fixes sit under the screen frame's table (f69da7f3e, cb040a64f, 07b61adb7, fb680e1fb, 83859ec2a) and the menu pages' table (aa82a99f0, 216bab656).
