# The session pane

The pane that shows what the session holds: keys, wallets, files held, flows in progress and what waits (the Waiting band). Only this pane draws those lists; no page keeps a list of its own. It stands on Home (as This session's card), beside the menu pages Tools, Check, Wallets and Vaults, and inside step flows as the first-step chooser (pick rows). A row opened in it shows a second pane of details and offers.

## Screens and sheets

- Home card (`pane_card`, `stacked_card`, `loaded_sections`, `faraday/faraday-core/src/screens/session_pane/mod.rs:~431, ~38, ~496`): keys, wallets, files, flows, and the Waiting band. Compact box (`compact_box`, `:~534`).
- Menu-page mode (`faraday/faraday-core/src/screens/session_pane/page_mode.rs`, `Mode::{Tools, Check, Wallets, Vaults}`): waiting PSBTs with Sign, files held, wallets, keys, flows in progress.
- Waiting band (`faraday/faraday-core/src/screens/session_pane/band.rs`), rows (`faraday/faraday-core/src/screens/session_pane/rows.rs`), the sidebar's Scan inside the pane (`faraday/faraday-core/src/screens/session_pane/scan.rs`), vault marks (`faraday/faraday-core/src/screens/session_pane/vault_marks.rs`), all in `faraday/faraday-core/src/screens/session_pane/`.
- Pick rows (`faraday/faraday-core/src/screens/session_pane/pick.rs`): `Decl`, `PickRow`, `Take`. A flow's first-step chooser: radio, checkbox, chevron ("opens") and way-in rows in groups, drawn by `draw_groups` (`:~358`).
- Row details (`faraday/faraday-core/src/screens/row_details.rs`): a second pane for an open key, vault, held file or wallet (`row_open::Opened`): offers first, then facts.
  - Key offers (`key_offers`, `:~59`): Explore, Derive, Convert, Krux code, vanity, silent payments, Lightning node key, sign a message, make a wallet, start multisig, restore with it, add to vault, check a copy of the words, show or save the account xpub.
- Scan frame (`faraday/faraday-core/src/screens/scan_frame.rs`): the camera picture with its outline.

## Actions

- `Resume(going)`, `Nav(Files | Backups)`, `OpenWallet(i)`, `Entry(Option<[u8; 4]>)` (Add a key), `ExploreKey(fp)`, `KeyWallet(fp, 1)`, `Explore`, `RowOpen(Row::{File, Key, Wallet})`, `RowClose`, `StartSpend(k)` (Sign on PSBT rows), `ScanPaste`, `Cancel`, `Vault(SaveKey | SaveWallet)`.
- Pick rows: the flow's own `Take` actions.

## Effects

- None outward. The camera is reached only through actions. Draw-time writes: `app.camera.pane_stands(app.screen)` (`faraday/faraday-core/src/screens/session_pane/mod.rs:~55, ~507`, `faraday/faraday-core/src/screens/session_pane/page_mode.rs:~261`).
- Paste only through `ScanPaste`, offered when `app.scan_pastes()` (`faraday/faraday-core/src/screens/session_pane/scan.rs:~73`).

## State and lock

- Owns no state. It reads the session (`Session` keys and wallets), the Inbox and Outbox, the vaults, `in_progress` and `row_open`.
- The session's keys and wallets are wiped at a lock by `Session::wipe` (`faraday/faraday-core/src/wallet.rs:1539-1549`). The Wallets area's lock clears the spend and `row_open` (`faraday/faraday-core/src/wallet/state.rs:1264-1274`). Every lock then ends in a fresh process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Per-frame values only: `SessionRow`, `Decl`, `PickRow`, `Take`, `Group`, `band::Layout`.

## Reach

- Read by Home and the menu pages (`menu-pages`), placed by the pane container (`pane-container`).
- Opens Add a key (`add-key`), Explore (`address-check-explore`), a wallet (`wallets`), Spend (`spend-signing`), Files, Backups, a vault save (`vault-contents`), the flows' Resume (`flow-container`), and through row details the key tools (`child-keys`, `convert`, `krux-codes`, `vanity`, `silent-payments`, `messages`, `create-wallet`, `restore-wallet`, `seed-checks`).
- Reaches `bip85::key_name` (`faraday/faraday-core/src/screens/session_pane/rows.rs:199, 312`), `home::{pane_jobs, home_wallets, CardLead}`, `row_open::*`, `in_progress::Going`, `vault::VaultAction`, `sheets::{read_card*}`, `scan_frame::draw`.
- Row details reach `wallets::{actions, wallet_page}` (`faraday/faraday-core/src/screens/row_details.rs:~276`).

## Rules and refusals

- Groups:
  - Groups with no rows are left out; Waiting is the foot band (`faraday/faraday-core/src/screens/session_pane/rows.rs:~47-58`).
  - Add a key: only for a wallet that is not silent, cannot spend, and has a slot not filled. With exactly one empty slot of known fingerprint the press is `Entry(Some(fp))`, else `Entry(None)`. Enabled only with `may_load_keys()` (`faraday/faraday-core/src/screens/session_pane/rows.rs:166-181`).
  - A key's name is shown only when it differs from its fingerprint (`faraday/faraday-core/src/screens/session_pane/rows.rs:~201, ~314`).
  - Compact rows: keys only when held and only loose keys (`faraday/faraday-core/src/screens/session_pane/rows.rs:~300-309`); the Files row only when the Outbox is not empty (`:~96, ~277`); the Backups row only when some wallet is not backed up (`:~144-152, ~291-299`); a wallet already on the lead card is skipped (`:~288`).
  - `account_key`: none for a sole silent-payments wallet or for differing account xpubs; none when the slot index does not fit a `u8` (`faraday/faraday-core/src/screens/session_pane/rows.rs:~350-368`).
- Scan in the pane: only on wide layouts, with the Scan sheet open, at most one camera, drawn on this screen (`faraday/faraday-core/src/screens/session_pane/scan.rs:22-28`). Paste only when `scan_pastes()` (`:~73`). The frame is at least `SCAN_PANE_MIN_H` (`:~75`). The read card is offered with `stick_in = !may_load_keys()` (`:~55-63`).
- Vault marks (`faraday/faraday-core/src/screens/session_pane/vault_marks.rs:14-24, 64-68`):
  - Only while exactly one vault is open with a current id.
  - A read-only vault gets no Save.
  - Key Save needs the vault not to hold its words and the key to be savable (words or seed bytes).
  - Wallet Save needs the vault not to hold it and to be writable.
  - A key held by the vault is marked "in a vault" (`:~58-59`).
- Pick rows (`faraday/faraday-core/src/screens/session_pane/pick.rs`):
  - A refused item stays listed, dimmed; its reason replaces its line; it takes no press (`:~94-98, ~337-354, ~486-488`).
  - Items with `take == None` are hidden (`:~202, ~243`); empty groups are left out (`:~313`).
  - The ways-in group is headed "Other ways in" when something loaded is taken, else "Ways in" (`:~375-381`).
  - Files first when opened from Check (`:~308-310`).
  - A row with both `opens` and `several` draws as opens (`:~343-349`).
- Page mode (`faraday/faraday-core/src/screens/session_pane/page_mode.rs`):
  - PSBT rows offer only `StartSpend` (`:~39`).
  - Files exclude PSBTs, carry facts, and only those `keep` accepts (`:~83-85`).
  - The Wallets pane excludes Backups (`:~182, ~337`).
  - The Vaults pane's waiting rows lose their open action (`:~329-330`) and have no row opening (`:~124-126`).
  - A flow in progress shows in the band only if not already a band item (`:~179`).
  - The Spend flow shows with the Sign rows only when a PSBT Sign exists (`:~209-213, ~331-334`).
- Band: at most `SESSION_BAND_SHARE` of the pane, at least its label and one row (`faraday/faraday-core/src/screens/session_pane/band.rs:~60-65`).
- Home card (`faraday/faraday-core/src/screens/session_pane/mod.rs`): scrolls only when rest groups exist, the content overflows and at least `HOME_BOX_MIN_ROWS` rows fit (`:~405-407`). Nothing loaded shows only the "nothing loaded" line (`:~451-460`). The card width is capped at `MENU_PAGE_W` (`:~384, ~439-440`). The compact box lists up to `HOME_BOX_KEYS` keys (`:~553-557`).
- Row details (`faraday/faraday-core/src/screens/row_details.rs`):
  - A wallet opens as its own page, not these offers (`:~247, ~296-298`).
  - Krux code only when `kef_offered` (`:~76-79`).
  - Silent payments hidden while the key is held for backup (`:~82-85`).
  - Lightning node key only with words (`:~87-89`).
  - Sign: none for zero wallets, the wallet for one, a key picker for several (`:~90-95`).
  - Check a copy only with words (`:~127-130`).
  - The xpub rows only when `account_key` returns one (`:~131-136`).
  - An open vault omits where and size (`:~192-195`).
- Scan frame (`faraday/faraday-core/src/screens/scan_frame.rs`): no outline when a dimension is zero (`:~50-52`); green once read, warning colour otherwise (`:~70-74`); the found outline only while `sc.frames - at <= SEEN_FRAMES` (10, `faraday/faraday-core/src/qr_view.rs:~266`) (`:~30-31`).

## Domain calls

- `faraday/faraday-keys`: `KeyInfo`, `holds_seed_bytes`, `is_sample`, `made_online` (read through the session).
- `core/faraday-bip`: `keys::Fingerprint`.

## Fixtures

None.

## Fixes

None. (eb0be84dd, a split sheet's row in this pane with no button, sits under the screen frame's fixes.)

## Doubts

- `KeyWallet(fp, 1)` uses a literal 1 (`faraday/faraday-core/src/screens/session_pane/rows.rs:~180, ~330`); its meaning is not stated.
- `faraday/faraday-core/src/screens/session_pane/vault_marks.rs` offers Save when `words` is false even if the vault holds the key with its passphrase; intent not known.
- `faraday/faraday-core/src/screens/session_pane/scan.rs:~26-27` checks the same fact twice (`camera.pane_seen` and `scan().pane`).
- `camera.pane_stands` writes during drawing, against "drawing reads"; whether it is real state is undecided.
- The ranking comment at `faraday/faraday-core/src/screens/session_pane/page_mode.rs:~88-92` is stale.
- The tests in `faraday/faraday-core/src/screens/session_pane/pick.rs` (`:~506-726`) were not read.
