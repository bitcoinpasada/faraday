# The backup board

The board draws a wallet's backup plan as places (cards with paper and stick
boxes), vaults, a watch-only software place and a list of chips (seeds,
passphrases, codex32 strings, Seed XOR parts, SLIP-39 shares, the description,
its partial shares, Krux codes). Chips are tapped or dragged and placed, moved,
copied or taken off; each drop is a `plan::Edit`, and the check lines under the
board update at once. The plan is read-only until "Customize backup". It is for
the person deciding what is kept where. It is drawn on Back up's plan stage
(`wallet-backup`) and on a wallet flow's own plan card (`backup-flow`).

## Screens and sheets

- Shown on `Screen::Backup` and `Screen::BackupFlow` only (drop and drag only there, `faraday/faraday-core/src/board/mod.rs:~2983-2987`). `board_shown` is `BStage::Plan` with `qstep::BOARD` (`~916-929`).
- Places, vaults, the software place (`software()`, `faraday/faraday-core/src/board/screen.rs:~1465`), the chip list, empty sticks (`Zone::Empty`, board only, never in the plan, `faraday/faraday-core/src/board/mod.rs:~86-89`, `~2188-2196`, `~2367-2370`), "+ place", "+ stick".
- Check lines under the board (`BoardView.warnings`, `faraday/faraday-core/src/board/mod.rs:~1166-1306`).
- Read-only buttons (`read_only_buttons`, `faraday/faraday-core/src/board/plans.rs:~15`, called from `faraday/faraday-core/src/board/screen.rs:2042`, `2179`): one per ready plan, Add or Take off watch-only software, Customize.
- Sheets: `Sheet::BoardList` (the Add list on a narrow panel, `faraday/faraday-core/src/board/mod.rs:~2162-2166`, drawn by `list_sheet` `faraday/faraday-core/src/board/screen.rs:~1807`); `Sheet::BoardMenu` (a chip's dots menu, `faraday/faraday-core/src/board/mod.rs:~2426`, `menu_sheet` `faraday/faraday-core/src/board/screen.rs:~2249`); `Sheet::BoardSees` (the balance or spend sheet: what a drop changed, `faraday/faraday-core/src/board/mod.rs:~2871`, `~2893`, `sees_sheet` `faraday/faraday-core/src/board/screen.rs:~1841`); `Sheet::BoardPartials` (`faraday/faraday-core/src/board/mod.rs:~2949`, `partials_sheet` `faraday/faraday-core/src/board/screen.rs:~1887`).
- Codex32 chip groups (`codex32_groups`, `faraday/faraday-core/src/board/screen.rs:~1773`; `faraday/faraday-core/src/board/codex32.rs`: `groups`, `warnings`, `forms_shown`, `place_lines`, `vault_lines`, `facts`): one chip per string, share warnings and counts.
- Seed XOR and SLIP-39 part groups (`groups`, `faraday/faraday-core/src/board/parts.rs:~23`): each seed's XOR parts, old XOR sets, SLIP-39 shares and old shares as chip groups (F276).
- Partial descriptions (`Thing::Share`): a button per place adds or removes a share of the description (`faraday/faraday-core/src/board/screen.rs:~1676-1692`; BACKUP-FLOW §17.19).
- A "vault keeps a copy of the plan" checkbox per vault (`BoardPress::VaultKeeps`, `faraday/faraday-core/src/board/mod.rs:~2299-2331`, `faraday/faraday-core/src/board/screen.rs:~1721-1750`).
- Place names typed on a card while a vault is open (`Action::BName(p8)`, `faraday/faraday-core/src/board/screen.rs:~1172-1174`; `board_backup_names`, `faraday/faraday-core/src/board/mod.rs:~999-1020`).
- The flow's stick figures use the board's stick box (`card_frame` `faraday/faraday-core/src/board/screen.rs:~2294`, `card_stick` `~2327`).
- Stick word or noun from `self.medium` (`faraday/faraday-core/src/board/mod.rs:~1226`, `~1293`, `~1421`, `~2603`).

## Actions

All through `BoardPress` (`faraday/faraday-core/src/board/mod.rs:~119-196`) and `board_act` (`~1968-2384`), plus drag and keys.

- `Chip(c)`: a tap selects (the software list chip places software on Scanned). `Zone(z)`: puts a held chip, or with nothing held on "+ stick" adds an empty stick (`~2185-2207`). `Off(z,t)`: the x (`~2208-2219`). `Form(z,t,k)`: toggles a form of a placed chip (`~2220-2238`). `LetGo`.
- `AddPlace`, `RemovePlace(p)` and `RemovePlaceSure(p)`, `AddVault`, `RemoveVault(v)` and `RemoveVaultSure(v)`, `VaultKeeps(v)`, `RemoveStick(p,s)`, `RemoveEmpty(p)`, `RemoveSoftware`, `Software`, `Plan(r)` (a ready plan, `~2091-2101`), `Customize` (`~2087-2090`), `OpenList` (Add), `Menu(c)`, `MenuVerb(v)` (Add, Move, Copy, Kef), `MenuTo(z)`, `MenuClose`, `SeesKeep`, `SeesUndo`, `Partials`, `PartialsKeep`, `PartialsWholeOff` (`~2126-2161`, `~2911-2951`).
- Drag: `board_down` (`~2970-2997`), `board_move` (`~3001-3025`, drag slop `motion::SLOP * f`, `~3002`), `board_up` (`~3029-3070`).
- Keys (`board_key`, `faraday/faraday-core/src/board/mod.rs:~2677-2762`): Tab and Shift+Tab move focus among the dots on screen; Enter or Space opens the focused dots; in a menu the arrows move, Enter or Space chooses, Escape steps back or closes.
- Seed chip menu "Krux code..." (`Verb::Kef`, `~2038-2041`, `~2454-2464`).
- Others: `Action::Mark(chip, MarkTip::Warning|Link)` tooltip (`faraday/faraday-core/src/board/screen.rs:~106-117`); `Action::GuideShares` (`~1703`); list scroll `flows.board_list_offset()` (`~2185`, `~2202`); list sheet close `Action::Cancel`.
- Chart jump `board_select` (`faraday/faraday-core/src/board/mod.rs:~1842-1865`); the quick paper backup hook `quick_put` (`~1815-1831`).

## Effects

- No direct storage. Plan writes go through `self.backup.edited(Edit)`, `added_set`, `place_removed`, `board_tidy` (`faraday/faraday-core/src/board/mod.rs:~1901`, `~1921`, `~2283`, `~2296`, `~2959`); `self.flows.board_edit`, `board_tidy` (`~943`, `~2961`); `board_change` goes to `flow_wallet_edit` or `backup_change_plain` (`~1798-1810`).
- Reads vault records: `board_before` calls `self.vault_tokens(c, o)` on the open vault (`~1707-1714`); `self.session.wallets.get`, `self.session.keys.info(k)` for the Krux offer (`~1009`, `~1690`, `~2475`, `~2488`, `~2499`).
- Costly work: `board_view_uncached` (`~1142-1638`) works out the plan map, `plan_mapped`, `check` and each place's line, in a drawing memo (`board_view_now`, `~1136-1138`); costly presses `board_sees_ask` (`~2824-2895`), `board_partials_ask` (`~2915-2951`), `board_partials_refresh` (`~2900-2909`).
- Toasts `backup_flow_files_after_lock` (`~2094`, `~2105`, `~2129`, `~2178`, `~2201`, `~2214`, `~2222`, `~2342`, `~2374`, `~2771`, `~2789`).
- Drawing writes only `fr.held_at` (per frame) and region and clip state (`faraday/faraday-core/src/board/screen.rs:~2199-2208`).
- No clipboard, camera, printing, time or randomness.

## State and lock

- `BoardState` (`faraday/faraday-core/src/board/mod.rs:~312-340`, `Default`): `held`, `down`, `drag`, `removing` (vault), `removing_place`, `empty: Vec<usize>` (board only, gone when the board is left), `sees: Option<Sees>`, `partials`, `menu`, `focus`. It lives in `backup.state().board` (Back up) or `flows.get(i).live.board` (a flow), through `board_state` (`~932-937`).
- `Sees` (`~345-356`): a clone of the plan `Answers` before the drop, title, line, spend flag: plan answers, fingerprints and set identifiers, no seed material.
- `BoardView` (`~817-870`): a per-frame memo of shape, check, cards, warnings, flags. `Frame` and `CardFrame` (`faraday/faraday-core/src/board/screen.rs:~28-43`, `~2290`) per draw; `Thing::Kept(usize,[u8;4])` and `SlipKept` hold public ids only.
- No secret is held in the board files.
- `board_tidy` drops the empty sticks when the board is left (`faraday/faraday-core/src/board/mod.rs:2954-2965`). `board_let_go` and `board_menu_open` clear what is held (`~949-955`, `~2411-2425`).
- Lock: the board wipes nothing itself. In Back up its state goes with `BackupState` at a lock (`wallet-backup`); in a flow it is part of `Live`, which is not kept across the lock's restart (`backup-flow`). The plan itself lives in the answers, kept with the flow or saved to vaults.

## Reach

- Drawn in `wallet-backup` (plan stage) and `backup-flow` (a wallet flow's plan card, the list pane beside the flow).
- Calls: `crate::plan::*` (Answers, At, Edit, Medium, Shape, Spot, What, Writing, form, `codex32::{max_places_of, C32, strings_at, index_char, SECRET}`); `Faraday` methods `backup_vault_target`, `backup_vault_set_up`, `backup_locked_flow`, `backup_seed_list_of`, `backup_change_plain`, `flow_wallet_edit`, `flow_shape_said`, `plan_shape_now`, `place_name`, `vault_tokens`, `kef_offered_of`, `compute`; `crate::backup_flow::{Step, BackupFlow::place_name, Target, stick_target}` (`faraday/faraday-core/src/board/mod.rs:~924`, `~1027`; `faraday/faraday-core/src/board/lines.rs:~237-262`); `crate::wallet_backup::{old_part_number, Tone}` (`old_part_number` at `faraday/faraday-core/src/wallet_backup.rs:2981`); `crate::restore::repeats_a_seed` (`faraday/faraday-core/src/board/lines.rs:~198`; `faraday/faraday-core/src/restore.rs:490`); `crate::menu`; `crate::qstep::{BOARD, MAX_PLACES}`; `crate::glance::{alone_color, check_lines}` (`faraday/faraday-core/src/board/screen.rs:~1180`, `~2058`); `crate::compact::{scroll_sheet, sheet_head, buttons}`; `crate::screens::board_work_end`; `crate::ui::{Region, Slot::BoardList}`.
- `faraday/faraday-core/src/board/lines.rs` reads `app.backup.state()`, `app.flows[i].wallet`, `app.session.wallets` (`~180-190`).
- Opens the Krux code form (`krux-codes`) from a seed chip's menu, the guide on shares, a chart node.
- Reached by `drawing::View::board_view` (`faraday/faraday-core/src/board/mod.rs:~1134-1137`), pointer and key paths (`board_marks`, `board_plan`), `quick_put`, `kef_dropped`, `board_place_name`, `board_map`.

## Rules and refusals
- A codex32 set's threshold is 0 (the secret alone) or 2 to 9; 1 is refused by the parser (core/faraday-bip/src/codex32/mod.rs:422, 521). The board's head line names k == 0 as "secret alone" (faraday/faraday-core/src/board/codex32.rs:31) and its warnings (a place that keeps enough strings, losing a place, a vault holding the whole seed) apply only where k is 2 or more (board/codex32.rs:111, 118, 152).

Gates (`faraday/faraday-core/src/board/mod.rs`)

- Read-only until Customize: any press not in {Customize, Plan, Software, LetGo, SeesKeep, SeesUndo, PartialsWholeOff, PartialsKeep, MenuClose} returns with nothing changed (`1981-1998`).
- A press outside the open menu closes it first (`~2009-2011`); a chip tap in the narrow Add sheet opens its menu (`~2015-2019`); `Menu(c)` only for software or a movable chip (`~2023-2025`); `MenuVerb` and `MenuTo` only the open menu's offered choices (`~2032-2034`, `~2060-2062`); `MenuVerb(Kef)` only for a list seed chip (`~2038-2041`).
- Krux row: only on Back up's board, before the lock, custom, the seed here (`~2443-2450`); dimmed with "not offered" when the seed cannot have one (`~2459-2464`); `kef_offered_of`: only a 12- or 24-word English seed (`info.kef_offered()`, `~2471-2489`); a loaded seed that refuses loses its Krux code from a plan read back, an unloaded one keeps it (`kef_dropped`, `~2492-2516`). Never offered on a flow's board.
- Locked plan (a flow past its lock, `board_locked`, `~986-994`): `Plan`, `Software`, `Partials`, `RemoveSoftware` and a software drop are refused with a toast (`~2093-2095`, `~2104-2106`, `~2129`, `~2374`, `~2770-2771`); a locked paper, file or public chip (not a vault) is refused on Chip, Zone (nothing held), Off and Form, and is not movable (`~2172-2179`, `~2198-2203`, `~2210-2215`, `~2221-2223`, `~1938-1945`).
- `Form` on a placed SeedQR drawn in parts destroys the parts; the map is unchanged (`~2225-2235`).
- `AddPlace` only while places are fewer than min(`qstep::MAX_PLACES`, `plan::codex32::max_places_of`), "at most one place per key" (`2242-2249`).
- `RemovePlace` and `RemovePlaceSure`: after the lock only the last place, silently (`~2257-2260`, `~2277-2279`); a place holding things asks first (`~2262-2271`). `RemoveVault` asks first if it holds anything (`~2306-2317`). `RemoveStick`: after the lock a stick with any non-vault file is refused with the toast `backup_flow_files_after_lock`; a stick carrying only a vault goes (`2334-2343`).
- `board_movable` (`1927-1956`): not when not custom; a list chip never; locked paper, file or public chips except vaults; a seed only if `here` or `shares`; String, Part, Kept, Slip, SlipKept and Kef only if the seed is here; other chips yes.
- `takes` (`548-574`): the List takes nothing; Software only when not locked; a vault chip goes only to Card, Files, NewStick or Empty; a vault zone takes everything except Kef, Part, Kept, Slip and SlipKept ("a code is no vault record"); after the lock nothing else; Scanned takes nothing; SoftStick only Whole; else what the zone has a form for. Seed, String, Part and the like need the seed `here`; Pass needs `here` and a passphrase; Share needs `splits && j < keys`.
- `board_put`: a drop on the List from a placed chip is an Off; a refused drop toasts when locked (`~2781-2791`).
- No "Copy to..." on a share chip (`~2543-2548`); a stick of a vault can go to a card only where the place does not keep it (`~2586-2599`); Take off is always offered, Move and Copy need an available box (`~2538-2553`).
- `board_down` is refused in the narrow column, with a sheet over the board, for a non-movable chip, on other screens, and for the list's software chip after the lock (`~2980-2991`); `board_up` drops only on the topmost zone that takes the chip, and a placed chip dropped on the list is taken off (`~3044-3068`); `quick_put` is refused unless `takes` on Paper(0) (`~1821-1823`).
- Names are typed only with a vault open, before the lock, on Back up's plan (`~999-1020`, `~1415`).
- Sees sheet (`board_sees_ask`, `~2824-2895`): skipped when the plan is unchanged or `secrets.plain()` is set (`2829`); the nearer-to-spend sheet only on a multi-key shape at a place with two or more signing seeds (`~2841-2844`); the balance sheet when a place newly sees the balance and does not spend (`~2879-2881`).
- Partials: "Take it off every place" only when the rest can rebuild and some place keeps the whole (`~2945`); refresh does nothing when locked or off (`~2902-2904`). A plan is reported as rebuilding only when checked whole (fix 701604d8c, in `backup-flow`).
- Marks follow a vault stick when it moves; a vault chip dropped on a card is a move; a vault stick taken off loses the marks of what it held unless its place keeps them (fixes 98d08da86, 4beab374e).

Check lines (each added once, `faraday/faraday-core/src/board/mod.rs:~1166-1306`)

- Seed kept nowhere; passphrase beside its seed's words; all copies of a seed at one place; `lost_why_on`; all keys at one place; one place alone spends; description not kept; a share alone at a place that sees the balance; a place with no seed or string; `codex32::warnings`; a vault with no stick; an empty vault.

Codex32 lines (`faraday/faraday-core/src/board/codex32.rs`)

- A key of words is not split into chips (`~43-47`); the QR form is offered only for 48-character strings (`~174-183`).
- Warnings (`~70-169`): a string kept nowhere; a place keeps enough strings to be the seed, and a place whose loss leaves too few (only k of 2 or more); a vault holding a seed and a string of it, or the whole seed of a threshold set; a codex32 string as a file in the clear (`~159-166`). `vault_both` counts only paper strings (`~145`); the secret index counts as whole (`~232`).

Board lines (`faraday/faraday-core/src/board/lines.rs`)

- `form_label` is none where the zone has one form (`~29`); a Krux code is labelled as such whatever the medium (`~63`); `sees_line` with no shares returns "whole" (`~93`); `description_line` reads "nothing" with no kept shares (`~119-121`); `software_name` is the first chosen, else Watch-only software (`~153-160`); `partials`: none when locked, offered for a splitting description; a multisig that does not split shows `board_partials_one_seed` when the policy repeats a seed, else `board_partials_not_offered` (`~172-202`).

Parts (`faraday/faraday-core/src/board/parts.rs`)

- Only seeds in `shape.here()` get groups (`~33`); an old part or share with no count is left out; an old-set group shows only if it has chips (`~56`, `~65`, `~108`, `~119`); a part's empty count reads "not checked" (`~12-18`).

Drawing (`faraday/faraday-core/src/board/screen.rs`)

- The place x is offered only if `v.custom && places.len() > 1 && (!locked || last)` (`~1171`); the stick x is not drawn read-only, or after the lock with files (`~1391`); the software x only `custom && !locked` (`~1493`); the vault-keeps checkbox is live only when custom (`~1725`, `~1740`); "+ place" only `custom && room` (`~1970`); the partials button only when `v.splits == Some(None)` (`~1676-1692`); the list shows entries only when custom, else the read-only buttons (`~2170-2181`); with no `BoardView` or state nothing is drawn.
- Chips are one touch target high (48 dp; 44 dp on the compact and short layouts); the vault chip's menu and x are 48 dp; form toggles wrap at the chip's width; chip marks use the label type (fixes 438acd7fb, 8ec2a9ff1, 36658def0, 95a1e38cd).
- The sees/spend sheet after a drop is not asked while a "Save without encryption?" sheet is waiting for its hold (`secrets.plain()` is Some: `faraday/faraday-core/src/board/mod.rs:2829`, `faraday/faraday-core/src/secrets.rs:266`); the hold of a plan-kind record asks it afterwards with the plan as it was (`faraday/faraday-core/src/secrets.rs:~534-546`).
- A drop of the software chip on any zone but the list turns the software place on when unlocked, never off; after the lock it toasts "files after lock" (`faraday/faraday-core/src/board/mod.rs:~556-558`, `~2769-2778`, `~3064-3066`). The software stick (`Zone::SoftStick`, `At::Files`) takes only the whole wallet from a drop (`faraday/faraday-core/src/board/mod.rs:571`); its `forms_in` arm for a seed (`faraday/faraday-core/src/board/mod.rs:420`) is only for a chip already on it (no drop makes one).

## Domain calls

- `faraday_crypto::work::Compute` (passed to `plan_mapped`, `check`, `add_partials`, `whole_at`, `hand_out`, `vault_tokens`, `Mapped::of`).
- faraday-plan through `crate::plan` (`stored_map`, `edited`, `holds_at`, `alone_on`, `reach_on`, `hand_out`, `description_kept_on`, `kept_nowhere_on`, `lost_why_on`, `all_at_one_place_on`; `Answers` methods `add_place`, `add_vault`, `remove_vault`, `add_partials`, `whole_at`, `toggle_software`; `Ready`) (`faraday/faraday-plan/src/`).
- `faraday_forms::Form`; `faraday_keys` `info.kef_offered()`; `faraday_text::Text`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 438acd7fb | Vault chips' menu and x buttons, and the Sticks and vaults card's rows, were under the 48 dp press target. | Press targets are 48 dp for the vault chip menu, its x and the Sticks and vaults card rows; the item chips' menu and x inside a second vault on one stick stay the board's 28 dp stacked chips (still under the floor, held by a count). |
| 8ec2a9ff1 | The backup board's item chips were one row too short, so their dots and x were hard to press. | Chips are one touch target high (48 dp; 44 dp on the compact and short layouts). |
| 36658def0 | On the backup board a chip's form toggles ran past the stick and over the "+" stick slot (Watch-only software, the third form). | Form toggles under a chip's name wrap at the chip's width, a row each, and the chip's height counts those rows. |
| 95a1e38cd | The board's chips set Whole, a vault and the software in the body type, unlike Material's chips and the fingerprints beside them. | Chip marks use the label type. |
| 98d08da86 | A vault stick moved on the board lost its checked and lost marks; a vault chip dropped on a card was not recorded as a move. | Marks follow the stick; the drop is recorded as a move. |
| 4beab374e | A vault stick taken off the board kept the marks of the things it held. | Marks go with the stick unless its place keeps them. |

## Doubts

- `faraday/faraday-core/src/board/screen.rs:~1969` comment says "one place per key" but the draw checks only `v.room`; gap tokens differ at `~1191` and `~1127` (`BOARD_CHIP_GAP` and `BOARD_PAD`); `card_frame` builds a placeholder `plan::Shape` (`~2290-2322`); every place uses `Icon::House` (`~1010-1016`).
- `sees_line` with an empty list reads "whole" (`faraday/faraday-core/src/board/lines.rs:~93`); whether that is reachable is not known.
- The caller of `card_frame` and `card_stick` is `faraday/faraday-core/src/backup_flow/figures.rs` per a doc comment, not verified.
- Fix 98b1be3af (Words and SeedQR form pills on the board under the touch floor) was given to `backups-list`; it is about this board.
- Test modules were not read; no fixture is named for this feature.
