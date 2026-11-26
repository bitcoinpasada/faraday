# Step flows

The flow container: the card a step flow is drawn on, Back and Forward through its steps and between screens (the trail), and the record of flows begun, with Resume and Cancel. Every multi-step task (Create, Restore, Spend, Backup, Message, Create a vault, the tools) uses it. It is reached from wherever a flow starts, and Resume is reached from Home, the session pane and the sidebar.

## Screens and sheets

- No screen of its own. `faraday/faraday-core/src/flow.rs` is the card contract: trait `Contained` (rows, steps, pick rows, wide content), `Card`, `Scroll`, `Room`, `Pick`, `TakeAway`. Drawing only.
- `TakeAway` lists files and codes taken away. A file counts as written when its name and SHA-256 match.
- `Sheet::CancelFlow` asks before a Cancel that would lose something.
- Flows with steps (`step_flow`, `faraday/faraday-core/src/trail.rs:~845-914`): Restore, Spend, SpendBackup, Message, CreateVault, Bip85, Convert, KefTool, OtsCheck, the contained Tools, Invoice, Decode, CheckMessage and NonceCheck, SeedCheck, MultisigCheck, CoreWallet, Silent, Vanity, KeyGen, Create, Backup (the plan stage uses `BACKUP_PLAN_STEPS`), BackupFlow, SignerCheck.
- No steps: WalletsStart, Wallets, Files, Entry, Settings, SetTime, Randomness, Vaults, Backups, Explore, Lightning, Unlock, VaultContents, Codes, CheckAddress, Catalog, Check, Home (`faraday/faraday-core/src/trail.rs:~888-910`).

## Actions

- `Action::Back` (`faraday/faraday-core/src/trail.rs:~605-619`).
- Escape and the mouse's back button, through `leave_back` (`:~792-813`), in this order: a sheet's Cancel (the Lock sheet stays, `:~801`); the guide pane closes (`:~803`); the page's back link (`as_back`); else `Action::Back`.
- Mouse forward reopens the last screen left, through `Action::Nav(s)`, if `can_show` (`:~827-839`).
- Step Back and Forward inside a flow: `step_back` (`:~949`), `step_forward` (`:~971`), `act_moving_on` (`:~994`).
- Back-link presses that stay on the page (its own step back, not a leave) (`stays`, `:~429-441`): `Glance(false)`, `RowClose`, `SpendPaths(false)`, `Vault(ItemBack)`, `SettingsSection(None)`, `KWordsPage(false)`, `KWordClose`, `Vault(Search(Leave))`.
- Resume (`Resume(going)`) and Cancel of a flow begun.

## Effects

- App state only. `land` (`faraday/faraday-core/src/trail.rs:~696-738`) sets the sheet and screen, calls `osk_leave`, `wallets.set_renaming(None)` and `glance.set_focus(None)`, and restores scroll offsets.
- `trail_note` (`:~554-560`) calls `in_progress.keep_way` and `search_after`.
- A running scan is ended with `Action::Cancel` (`:~612-613`).

## State and lock

- `Trail` (`faraday/faraday-core/src/trail.rs:~44-82`): a trail of `Spot`s (at most 32, `MAX`, `:~31`), a forward stack (at most 8), step-forward (at most 16), `landed`, `back_link`, `back_hit`, `step_landed`, `apart`, `drawn_apart`.
- `Spot` (`:~393-423`): screen, guide, lock_ask, wallet index (`GONE` = `usize::MAX`), glance, scroll offsets, vault item and category fields. No secret.
- `Trail` holds nothing a lock wipes (`faraday/faraday-core/src/trail.rs:84-85`).
- `InProgress` (`faraday/faraday-core/src/in_progress.rs`, `Public`): begun, ways, shown, search. No secrets. Going kinds: Create, Restore, Backup, Spend, Message, Vault, Flow(i).
- Every lock ends the app: a fresh process, or, when the save fails with keys waiting for their backup, a fresh app in the same process (`faraday/faraday-core/src/vault/seal.rs:933-946`, `faraday/faraday-core/src/storage_api.rs:1167-1202`). So `Trail` and `InProgress` do not outlive a lock. Of the flows, only backup flows cross a lock, through the kept state (`faraday/faraday-core/src/memory/mod.rs:503-515`), without `layouts` or `live`. Make a new wallet with its keys kept nowhere is ended by `lock_flows` (`faraday/faraday-core/src/vault/seal.rs:654-660`). Create, Restore, Spend, Back up and Message are cleared by their areas' locks.

## Reach

- Opened from every flow's start; Resume from Home and the session pane (`menu-pages`, `session-pane`) and from a sidebar tab (`screen-frame`).
- Reads `wallet::place_after_removal` (`faraday/faraday-core/src/trail.rs:453`), `nav::is_tool` (`faraday/faraday-core/src/trail.rs:746`), `address_check::pane::scanning` (`faraday/faraday-core/src/trail.rs:595, 612`).
- `lock_held_on` is read by `lock_ends_create` (`faraday/faraday-core/src/idle.rs:232-236`) and by the PowerAsk path (`faraday/faraday-core/src/state.rs:2540, 2587`) (`lock`).
- `crate::screens::RSTEPS` used at `faraday/faraday-core/src/in_progress.rs:~924` (`restore-wallet`).
- The flows listed above (`create-wallet`, `restore-wallet`, `spend-signing`, `spend-backup`, `messages`, `vault-list-create`, `child-keys`, `convert`, `krux-codes`, `decode`, `seed-checks`, `core-wallet`, `silent-payments`, `vanity`, `keygen-flow`).

## Rules and refusals

- Trail:
  - Home empties the trail (`faraday/faraday-core/src/trail.rs:141-143`).
  - A screen reached again cuts the trail back to it, unless `returning` (`:~145-150`).
  - Input that went back does not push the screen it left (`:~138-139`).
  - Back is a link press only when no sheet is open or the address check's scanner is running (`:~593-597`).
  - A stay is not Back. On a held app, Home goes to its holder (`:~593-598`).
  - A landing screen is shown only if it can be: Wallets by `wallet < session.wallets.len()` (`:~538`), VaultContents by `vault_current < vaults.open.len()` (`:~539`), others by `can_show`; else the spot is dropped (`:~647-652`).
  - `end_vault_detour` drops Unlock, Vaults and CreateVault (`:~659-665`).
  - Step Back is refused while a step is `running` (`:~953-955`); on the first step it leaves the flow (`:~956-958`).
  - Forward only while the landing screen is the same and no sheet is open (`:~263-267`).
  - Moving on only if the screen did not change and no sheet opened (`:~997-999`).
  - `nav_open` opens Transfer only with `demo` (`:~480-481`); `back_offered` exists only with `testkit` (`:~570-574`).
- Flows begun (`faraday/faraday-core/src/in_progress.rs:~265-352`):
  - Create is begun with a built wallet or a done card.
  - Vault is begun only past PRESET with no back target.
  - A backup flow is begun past Where, or once made and not held by Create, Restore or Backup.
  - Backup is begun with a non-empty plan or a checklist.
- Cancel (`flow_cancel`, `faraday/faraday-core/src/in_progress.rs:~571-614`):
  - Not offered for a backup flow whose key is made (`:~551-566`).
  - Key Cancel is not offered while locked or once a hosted key is built; it asks first when rolls, characters, words or a passphrase were entered (`:~628-639`).
  - Flow Cancel asks (`Sheet::CancelFlow`) when `cancel_loses` names a line for its kind (`:~683-746`).
  - Keys, vaults and the Outbox are never lost by a flow Cancel.
  - Spend Cancel offers Start over. A flow's end routes per kind (`:~753-835`).
- `lock_held_on` (`faraday/faraday-core/src/in_progress.rs:1130-1140`): Make a new wallet holds the auto-lock only while begun, not kept in a made wallet flow and not finished.
- Home search return: opening a result remembers the words; Back restores them (`:~385-407`).
- Since the history fixes: a resumed flow's Back, Cancel and Done return to the page it was first begun from (F77 amended); every wide page's back arrow and Escape go back by the trail, and Spend falls back to Wallets.
- `still_fits` in `Contained` defaults to false and is marked `#[expect(dead_code)]`.

## Domain calls

None. The flows' own files make the domain calls.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 3900b5959 | A flow's heading chip ("Then: Wallets", or the wallet picked) could sit under the flow's buttons (Steps, Guided, Cancel). | The chip is never under buttons: it moves to its own line under the heading and the column moves down. |
| 687d8a29c | On the small panel a step flow's title ran under the trailing actions (eye, Testnet, ?, Cancel); "Add a key" on Make codex32 shares was covered by the eye. | The title ends before the top bar's trailing actions and is cut with an ellipsis (Material 3 top app bar). |
| e9f1af13c | The step count on a small window was wrong (it read Step n of m before the Keys step was listed; Spend steps read early). | The step count reads Step 1 and Step 2 until the Keys step is in the list, then Step n of m. |
| 2a279ad6c | A GPG or Secure Boot key made in a flow could be saved from other screens as a plain entry; leaving a key flow at Make left its form open on Vault contents. | A key made new is saved only by the flow's Make; leaving the flow closes its form. |
| be45dd563 | A Make form was closed by a detour (such as Set the time opened from Make a key) and not rebuilt when the flow came back. | The Make form stays through a detour that returns; it is closed only when the flow is left for good. |
| e6636ed97 | A long step title in Spend's step list was cut instead of wrapping; on the small panel Restore's More kinds was faded while resting at the bottom. | Titles wrap to a second line, every row the height of the longest. |
| 530905a6c | On wide windows a flow's column ran under the Testnet pill and the ? in the top-right row, for every flow through the shared column. | The column ends before the top-right row (`flow::column_foot`). |
| 2e43c665b | On every flow's page head at 1280x800 and wider, "Guided" was covered by the mode pill and ?. | Page-head controls end left of the top strip's mode pill and ? (LAYOUT §15). On New key, Enter or Space presses the focused control before the open card takes the key. |
| 114e71ae4 | On the Pi panel the bar's Cancel started past the panel's left edge in some flows; the Practice pill read "Practice · Testnet" and overflowed; Make codex32 shares' tabs and Vault contents' kinds moved. | Nothing is cut off in any flow's bar, tab or step, in any mode, on the stick and in Demo's Pi preview; where tools would reach Back's touch target the ? gives way; the eye keeps 40 dp in a 48 dp target. |
| 3f3b64116 | On the wide layout some pages drew their back arrow from a different way back than the compact layout; Spend with an empty trail had no fallback; Set the time opened from the codes went back wrongly from its foot button and Escape. | Every wide page's back arrow and Escape go back by the trail, as the compact layout does; Spend falls back to Wallets. |

## Doubts

- A Vault flow never counts as finished (`faraday/faraday-core/src/in_progress.rs:~1164`); not checked further.
- `c4c1eb2dc` (a resumed flow returns to where it was first begun) is a decision change (F77 amended) as much as a repair; its row sits under the screen frame's fixes.
- The trail rules were spot-checked as a group; only the line numbers written without `~` were checked one by one.
