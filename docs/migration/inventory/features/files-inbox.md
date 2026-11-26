# Files, Inbox and the file menu

The Files page is where a person sees what is in Faraday and what is on a
stick, and moves files between them. Its left card, In Faraday, lists the
Inbox (files read from a stick or scanned), the Outbox (files waiting to be
written), the settings row and the open vault; its right card, On the stick,
lists the plugged stick's files. Each row has a file menu (the dots menu:
Rename, Details and per-kind actions). The Inbox also offers what its files
make possible: wallets found in them, loose seeds that can become a wallet,
loose xpubs that can become a watch-only wallet, and incomplete share sets.
The page is `Screen::Files`, reached from Home, the navigation, a stick being
plugged in, and the boot import (`boot-import`), whose list takes the stick
card's place while it is up. The Vaults pair (`Screen::Vaults`) shares the
page's drawing helpers and is described here where it is drawn.

## Screens and sheets

- `Screen::Files`, entry `visit::draw` (`faraday/faraday-core/src/screens/visit.rs:~53`), called from `faraday/faraday-core/src/screens/files.rs:435`.
- Wide layout: two cards, In Faraday on the left (`in_faraday`, `faraday/faraday-core/src/screens/visit.rs:~619`) and On the stick on the right (`on_the_stick`, `faraday/faraday-core/src/screens/visit.rs:~1166`). Compact: one card at a time under primary tabs (`faraday/faraday-core/src/screens/visit.rs:129-157`).
- Inbox panel (`faraday/faraday-core/src/screens/files.rs:~21-314`, compact `~863-1097`):
  - found wallets, each ticked (`InboxChoose`), with Load (`InboxLoad`);
  - loose seeds, "Make it a wallet" (`PotentialOpen(fp)`);
  - loose xpubs, "Make it a watch-only wallet" (`XpubOpen`);
  - incomplete backups (share sets short of their threshold);
  - a folded summary (`InboxSummary`), one row unless opened, at most 3 rows (`SUMMARY_ROWS = 3`, `faraday/faraday-core/src/screens/files.rs:317-318`; the compact path and the vault panel define their own 3).
- File card actions (`inbox_actions`, `faraday/faraday-core/src/screens/files.rs:~1345-1397`): the offered action from `items::offer`, Import into the open vault (`Vault(ImportEntries(k))`), Contents (`Vault(Open(at))`), Make sheet PDF (`PdfInbox(k)`).
- Vault panel (`faraday/faraday-core/src/screens/files.rs:~566-847`, compact `~1101-1334`): the open vault's seeds and wallets, tickable; Select all (`Vault(ChooseAll)`), Load (`LoadChosen`), Load with its keys (`LoadWithKeys`), Each seed (`EachShown`), Go to Wallets (`Nav(WalletsStart)`).
- Stick status and chip (`faraday/faraday-core/src/screens/files.rs:~452-513`): the boot medium line (two sticks of this build; boot one not found).
- Visit receipt rows (`faraday/faraday-core/src/screens/files.rs:~516-559`): heading "Written to <stick>", then each file verified or failed. A row the same on both sides carries the same-file mark (`faraday/faraday-core/src/screens/visit_same.rs`).
- No stick (`on_no_stick`, `faraday/faraday-core/src/screens/visit.rs:~1454`): an empty list shows the stick status line. A stick in but not usable shows "Lock to use it" and a Lock button (`Action::Lock`, `faraday/faraday-core/src/screens/visit.rs:1466-1483`).
- Row details (`row_details::draw`) replace the stick card or the page.
- End card (`spend::end_card`) after a signed write that is checked (`faraday/faraday-core/src/screens/visit.rs:~1530-1532`).
- Sheets:
  - `Sheet::FileMenu`: the file menu of a row. Rows are `file_menu::Row::{In, Out, Stick, Vault}`.
  - `Sheet::Rename`: a new name for a file, or the stick's own label.
  - `Sheet::Details`: a file's details.
  - `Sheet::Potential`: make a wallet from a loose seed, or a watch-only wallet from an xpub; kind choice and passphrase typed twice.
  - `Sheet::OutboxDelete`: asks before removing an Outbox file no stick holds.
  - `Sheet::Register` receives key forwarding from the file menu.
  - `Screen::Restore` is a back target of the Inbox.
- Vaults pair (`faraday/faraday-core/src/screens/vaults_pair.rs:12`, drawn from `faraday/faraday-core/src/screens/mod.rs:749` only when `app.screen == Screen::Vaults`): left, the session pane in Vaults mode; right, a card of vault cards plus "Make a new vault".

## Actions

- File menu presses (`FilePress`, `faraday/faraday-core/src/file_menu.rs`): `Menu(Row)`, `Close`, `Rename`, `Details`, `Decode`, `SignMessage`, `CheckAddress`, `StickDelete`, `NameStick`, `Field`.
- Inbox presses (`faraday/faraday-core/src/inbox.rs`):
  - `InboxLoad`: loads the chosen wallets and the seeds that are their keys.
  - `InboxChoose`: ticks or unticks a wallet (a skip set keyed by descriptor).
  - `InboxRemove`; `OutboxRemove` and `OutboxDeleteHold` (asks first when a file no stick holds would be lost).
  - `PotentialOpen` (refused with a toast when keys may not load), `XpubOpen`.
  - `PotentialKind(i)`: index into `KINDS` (NativeSegwit, Taproot, NestedSegwit, Legacy); out of range is ignored.
  - `PotentialType` and `TypeAgain`: typing only with no stick in the way. `PotentialMake`.
  - Keys on the Potential sheet: Escape clears; Enter moves to the repeat field or makes.
- In Faraday card:
  - tick `VisitOut(k)` (only with a usable stick and a file not held, `faraday/faraday-core/src/screens/visit.rs:~812`);
  - settings row `VisitSettings` (only when the settings differ from the boot stick's, `~736-741`);
  - vault row Contents `Vault(VaultAction::Show(v))`, with a menu only when wallets with keys can load (`~520-539`);
  - Inbox rows `VisitInbox(k)` and the first `inbox_actions` entry (`~844-883`, `main_action` `~1519`);
  - row open `RowOpen(Row::File(k))`.
- On the stick card:
  - tick or press `VisitIn(k)` (known kind, not too large);
  - Rename stick `File(FilePress::NameStick)`;
  - chips `VisitStick(k)`, `VisitCopy`, `VisitCopyAndLoad`, `VisitCopyAndSign` (only with exactly one PSBT, `~1563-1581`);
  - `DeleteFiles`, `DeleteUnprotected`, `Lock`;
  - select all `VisitOutAll`, unselect all `VisitInAll`;
  - scrollbar `VisitBar(column)`, tabs `VisitTab(Column::Outbox | Stick)`, write `VisitWrite`.
- Stick chip on a stick holding a locked secret: `Action::Lock`.
- Vaults pair: "Make a new vault" `Vault(VaultAction::Create)` (only when `vaults.back_to` is none, `faraday/faraday-core/src/screens/vaults_pair.rs:~42-48`); sample vault `samples::put::offer(ui, Action::UseSample)` only when `put_on()` is some; card body `RowOpen(Row::Vault(i))`.

## Effects

- Storage commands from the file menu: `StorageCommand::Hash`, `Rename`, `NameStick` (`faraday/faraday-core/src/file_menu.rs:~544-547`, `~698-730`).
- SHA-256 of Inbox and Outbox files worked out in process under `Compute`.
- `save_boxes()` after Inbox and Outbox pushes, removes and renames: the boxes are kept by the shell across a restart (`storage`).
- Decode and Sign this message take `item.bytes.clone()`, a local wiping copy dropped when the press is done (`faraday/faraday-core/src/file_menu.rs:480-498`). Decode keeps only the decoded transaction (`faraday/faraday-core/src/decode/mod.rs:1104-1140`); Sign this message keeps the text as its message (`faraday/faraday-core/src/state.rs:2063-2074`), public by the message area's lock rule (`faraday/faraday-core/src/state.rs:2783-2791`), until Done, Cancel the flow or a lock.
- Making a potential wallet runs key derivation (PBKDF2) under `Compute`, then `session.add_wallet`; toasts.
- Drawing writes only scrollbar cells (`app.visit.bar(..).set`, `faraday/faraday-core/src/screens/visit.rs:~89-90`, `~1803-1805`), scroll regions and clip stacks. No storage, worker, camera, clipboard, time or randomness from drawing.

## State and lock

- `FileMenus` on `visit` (the Files visit area): the open menu, the rename in progress, the details shown, the `renamed` set.
- `InboxArea { files, skip, potential, typed_at, summary_open }` (`faraday/faraday-core/src/inbox.rs`).
  - `files: Vec<Item>`. `Item` (`faraday/faraday-core/src/item.rs`) holds `bytes: Zeroizing<Vec<u8>>` (wiped on drop), the kind, a `secret` flag, picture, sha (vault and EFI only), facts, wallet policy, PSBT facts.
  - `Potential.pass: PassField` holds a typed passphrase (secret).
  - The `Secret` enum holds a `Mnemonic` or SLIP-39 shares without a wiping wrapper in this file; whether they wipe depends on their types.
- Lock (`faraday/faraday-core/src/inbox.rs:2119-2135`):
  - drops every item whose `dropped_at_lock()` is true: kinds that may be secret, or items flagged secret (`faraday/faraday-core/src/item.rs:145-148`);
  - clears the potential wallet and its passphrase;
  - `holds()` reports `typed` when the passphrase is non-empty, `session` when a potential exists or such an item exists;
  - `skip`, `typed_at`, `summary_open` and the kept `Found` survive in process. The process ends at a lock (`faraday/faraday-core/src/vault/seal.rs:933-946`), so they go with it.
  - Non-secret Inbox and Outbox files survive into the next process through the boxes.
- `impl Public for Item` (`faraday/faraday-core/src/item.rs:178`) is only the element bound; the Inbox's own `lock` does the dropping.
- A secret file's hash is kept as zeros (`hash_to_keep`).
- `copy_ack` kinds: `Ack::Any` (secret, Entries, SecretWallet, KeyWithSecret, KruxCode), `Ack::Seed` (Words, SeedPart, KeyLine), `Ack::Unknown` (Text, Other), none (public or sealed).
- The page drawing keeps nothing. It reads `app.visit` (out, from_inbox, inn, stick, tab_stick, log, receipt, reading, load_after, bars), `app.vaults.open`, `app.stick`, `app.inbox`, `app.outbox`, `app.scroll`. It reads file bytes only for share text (`share_note`, `faraday/faraday-core/src/screens/visit.rs:~1680`).
- `VisitArea` lock clears `pictures_scan`, `taken`, `name_taken` and `after_visit` only (`faraday/faraday-core/src/stick/visit_taken.rs:2526-2541`).
- Every arm of `Secret` (`faraday/faraday-core/src/inbox.rs:633-645`) wipes on drop. `Mnemonic` has `Drop` (`core/faraday-bip/src/bip39.rs:759`) and `slip39::Share` has `Drop` (`core/faraday-bip/src/slip39.rs:574`). `Codex32` and `Seed` are `Zeroizing`; `Xprv` is a `SecretText`. `Potential.pass` is a `PassField` of `SecretText` (`faraday/faraday-core/src/pass_field/mod.rs:117-124`), with no `Clone` or `Debug`.
- A `write_waiting` of this process goes at every lock, the not-kept rebuild included. Only a "Lock and write" crosses, as `lock_to_write` encoded under `write-waiting` (`faraday/faraday-core/src/memory/mod.rs:460-465`). The new app reads it back as its `write_waiting`. `WriteWait` holds names and hashes, no bytes (`faraday/faraday-core/src/stick/visit_taken.rs:213-227`).

## Reach

- Opens: Restore (`Screen::Restore` back target, Restore from a share set), Decode, Sign a message, Check an address, Spend (`StartSpend`, Collect), Back up, the open vault's Contents and Load, Wallets (`Nav(WalletsStart)`), Lock.
- Draws with: `screens/hub_layout`, `session_pane`, `wallets_pair` (span, way_rows), `files` (boot_medium_line, inbox_actions, inbox_panel, receipt_rows, stick_status, vault_at), `parts`, `row_details`, `spend::end_card`, `boot_import`, `file_menu`, `row_open`, `vault` (panes_screen::card), `samples::put`, `stick::settings::FILE`, `app.visit_same(...)`, `app.visit_inbox_rows()`.
- Opened from: `faraday/faraday-core/src/screens/mod.rs:749` (Vaults pair), `faraday/faraday-core/src/screens/files.rs:435, 547`. The boot import reuses `stick_heading`, `visit_list_bar` and `delete_files` (`faraday/faraday-core/src/boot_import/screen.rs:114, 190, 201`).
- Fed by: the boot import (`boot-import`), the stick visit (`stick-shell`, `storage`), the scanner (`scanner-qr`) for scanned files.
- `crate::restore::merge` is called for the share note (pure parse).

## Rules and refusals

File menu (`faraday/faraday-core/src/file_menu.rs`):
- Rename refuses (`faraday/faraday-core/src/file_menu.rs:603-663`):
  - an empty stem, a leading dot, control characters;
  - any of `/ \ : * ? " < > |` (`NOT_IN_NAMES`, `faraday/faraday-core/src/file_menu.rs:182`);
  - more than 255 characters (`NAME_MAX`, `faraday/faraday-core/src/file_menu.rs:185`);
  - a name taken in the Outbox, case-insensitive.
- A vault-length file renamed onto another listed file's name is refused silently, so the disk process does not read them (`faraday/faraday-core/src/file_menu.rs:~636-651`). The rename onto another name is judged by the two listed sizes.
- Rename is blocked with a toast while the boot import reads that file (`faraday/faraday-core/src/file_menu.rs:~684-691`). While the import list stands, a rename onto a vault's name is refused.
- The stick's own label refuses: too long, a bad character, a boot-partition label; an empty label removes it (`faraday/faraday-core/src/file_menu.rs:~1092-1110`).
- No menu on the settings file (`faraday/faraday-core/src/file_menu.rs:~219-221`).
- Out row: Make PDF only for sheets and only online; QR only if it fits; Decode and Check a signer only for a PSBT or a transaction.
- Stick row: Details hidden while the import list is up.
- A menu action is refused when the listed file is not the one named (`faraday/faraday-core/src/file_menu.rs:~402-409`); a stick file's menu acts on its own place.
- Stick Delete is refused while the import reads that file.
- Windows device names (CON, COM1 and similar) are refused and get a suffix (history fix 2efdf3d57; `faraday/faraday-files/src/lib.rs`).

Inbox (`faraday/faraday-core/src/inbox.rs`):
- `PAIRS_TRIED = 64` word and passphrase pairs are tried (`faraday/faraday-core/src/inbox.rs:136`).
- A `passphrase-<8 hex>.txt` file is used only if the fingerprint matches its name.
- SLIP-39 and codex32 shares are grouped by identifier; a group is a seed only at its threshold (codex32: or any secret string). The threshold comes from the first share only.
- An xpub is offered as watch-only only with its origin, a purpose not 45, 48 or 87 (44, 49, 84, 86 map to kinds; `kind_of_path`, `faraday/faraday-core/src/inbox.rs:580-593`), not already in a wallet, and not a seed here (`faraday/faraday-core/src/inbox.rs:~1226-1241`).
- The passphrase is typed twice; a differing pair does nothing; nothing happens while the eye waits. With a stick in the way, make is refused and `inbox_remove_the_first` is shown on the sheet (`faraday/faraday-core/src/inbox.rs:1680-1691`).
- Seeds load only with no stick in (`may_load_keys`). `load_found_in` loads keys only when `may_load_keys()`; wallets still load (`faraday/faraday-core/src/inbox.rs:1302-1317`).
- A duplicate key on make counts as success. A passphrase on a key without words is refused.
- A same-name file with other bytes is kept under a fresh name and the visit says so. A read-again of the same bytes is not duplicated. One case-insensitive name finder gives free names (`stem-2.ext`); scanned names (`scanned-N`) are unused in the Inbox.
- Wallet add refusals: NotYourKey and NotBackedUp give a toast; Unconfirmed counts as loaded; others are silent.
- InboxLoad loads only the ticked wallets.
- A signed message whose body holds a secret (such as seed words) is not a public signed message; it is marked secret and dropped at lock.
- A QR vault's `.pgp` is sealed under its passphrase; Open in QR vault only for a file that is a QR vault.
- An encrypted backup needing more Argon2 memory than is free is refused before the passphrase is tried, naming both amounts.
- A pending vault key's `.keyx` stays listed in the Outbox until its vault can take it back.

Per-kind offers (`items::offer`, `faraday/faraday-core/src/items.rs:~68-115`):
- Unlock refused with a stick in.
- Load wallet and Add to vault only while not loaded; Add to vault only while no vault is open on Files.
- Check an address only for a loaded wallet file; Back up only for a loaded wallet.
- Sign this message only if a message wallet is offered and the body is UTF-8.
- Sign hidden when the PSBT is checked-not-signed or no loaded wallet can sign.
- Collect when the PSBT txid matches the spend and the name differs.
- Restore only when the held shares make the seed; GPG restore only if `restorable`.
- The Files card button is the first entry of the same list.

Files page:
- Import into a vault only while a vault is open: `app.vault_way(Screen::Files).is_none()` (`faraday/faraday-core/src/screens/files.rs:1366`); `vault_way` is `None` exactly while a vault is open (`faraday/faraday-core/src/vault/mod.rs:321-327`).
- Contents only for an open vault; Make sheet PDF only while online (`FileKind::Sheet if app.online`) (`faraday/faraday-core/src/screens/files.rs:1375-1388`).
- Nothing is drawn without content (`faraday/faraday-core/src/screens/files.rs:~28-34`).
- A wallet row is a checkbox only with several wallets (`~65-71, 156, 160`). A ready-to-sign tag when seeds here reach the number needed (`~109, 122, 894`).
- The Load label counts only chosen wallets; nothing chosen reads "nothing chosen" with no primary button (`~154-175, 952-967`).
- Make a wallet is disabled without key-load permission, with the "keys load once" line (`~178-189, 227-231`).
- A folded vault shows only the together list (`~595, 613, 754-756`); a loaded row is marked Done, not ticked (`~761-768`); Go to Wallets only after a load and not on Spend backup (`~739, 835, 1118, 1205, 1320`); no open vault, no vault card (`~571-573`).
- Stick chip hidden when no stick is in; a stick attached but not listed shows as unreadable with no press (`~463-467`); a stick holding a locked secret gives Lock (`~470-474`).
- `usable = app.stick_pane()` (`faraday/faraday-core/src/screens/visit.rs:~103`). `stick_pane` is `screen == Files && !stick.is_empty() && !holds_secret()` (`faraday/faraday-core/src/stick/visit_taken.rs:1852-1854`). So a stick "in but not usable" means a secret is held, and the Lock button is shown.
- Without a usable stick: no ticks, no Write, no Import, no Select all, no stick file actions (`faraday/faraday-core/src/screens/visit.rs:~645-668, 738, 812, 859-866, 1283`).
- An unprotected secret file on the stick is named under the cards in the error tone with Delete (`faraday/faraday-core/src/screens/visit.rs:~105-123`).
- A stick file bigger than the disk process reads, or of unknown kind, is listed greyed, with no tick and no open (`~1268-1288, 1327-1333`). The settings file row has no menu (`~1273-1276`).
- An Outbox file already in the backup, or a signed message waiting for its backup, is greyed with no press (`~783-812`). `VisitOut` is refused for these and `VisitIn` only if `unread_why` is none (`faraday/faraday-core/src/stick/visit_taken.rs:1931-1951`).
- `put_item` refuses Secret items in every build (`faraday/faraday-core/src/stick/visit_taken.rs:1014-1016`).
- An Inbox vault's Open waits while a stick is in (`faraday/faraday-core/src/screens/visit.rs:~1517-1524`); Load wallet is dropped when the summary already loads it (`~843, 870`).
- Import buttons are disabled with nothing ticked (`~1405, 1427-1431`); the write button is primary only when something is ticked (`~931`).
- Select all counts only writable items: settings, Outbox files except unprotected secrets, not in the backup, not waiting for backup; Inbox files with no copy ack (`~1613-1628`).
- Inbox tag: sealed is ok, public grey, `Ack::Unknown` a warning "may", other acks an error "secret" (`~1697-1706`).
- Outbox state text: Secret reads "unprotected" (error tone); open vault with changes "unsaved"; open vault with no changes "sealed"; else "ready"; an open vault adds "written after Lock" (`~590-612`).
- The pull line counts keys copied in for Import and load; "remove to unlock" only while no secret is held (`~1503-1507`).
- The share note reads "n of m keys" or the merge error (`~1675-1692`).
- Same-file mark: a faint green box and a check at the row end, so not colour alone (`faraday/faraday-core/src/screens/visit_same.rs:1-3`); used at `faraday/faraday-core/src/screens/visit.rs:415, 1281` and `faraday/faraday-core/src/screens/files.rs:547`. Compact drops the tag on such rows (`faraday/faraday-core/src/screens/visit.rs:~388-390`).
- Vaults pair: never stacks and has no pair in a narrow room (`PANES` stacks false, narrow `OneColumn`, `faraday/faraday-core/src/screens/vaults_pair.rs:24-29`); returns false in compact (`~16-18`) and where no pair fits (`~13-15`); vault cards are pinned at the tile foot when they do not fit (`~68-78`).
- A stick's directory listing is read once; entries after the end are not shown; names are bounded (history fix d7492bfc2).
- Delete files (`faraday/faraday-core/src/stick/wipe.rs`): the sheet chooses Files, Free space or Whole partition; Whole takes one or two passes (`:43-63`, `:263-267`); a vault, a `.kdbx`, a `.keyx` or a key file used this power-on among the chosen files, and Free and Whole always, make the button a hold (`:189-232`); nothing the boot import still reads is deleted or zeroed (`:271-288`); a Whole wipe or a failure of one before its first answer drops the backup flow's written-copy records for that stick (`:393-404`, `:455-467`); the ask and a zeroing end when their stick is gone (`:479-496`).

## Domain calls

- `faraday_fat::label_bytes` (`faraday/faraday-fat/src/lib.rs`), `faraday_boot::layout::is_boot_label`, `faraday_codec::vault_file::is_vault_len`, `faraday_crypto` hex (file menu).
- `faraday_bip`; `faraday_keys` (fingerprint, `slip39_seed`, `codex32_seed`, `xprv::parse_hex_seed`); `faraday_intake::part_of`; `faraday_vault::codex32` (Inbox).
- Drawing: `faraday-text`, `faraday-draw`, `faraday-ui` only.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 0bab55cbf | The boot-import list let In Faraday's controls, Delete files, zeroing and renames act while up; any stick took the list down. | Controls drawn as with no stick and refused; the list stays until its own answer. |
| c8bb85314 | Import was allowed with a listing the stick did not hold; a tick followed place, not file; a deleted or overwritten file stayed in the list. | Import off without a held listing; a tick names its file; a changed file leaves the list. |
| 2b14e801e | A rename onto another name was judged by a listing that stops at a limit; a stick file's menu could act on the wrong file; a file Import still reads could be deleted, renamed or zeroed. | Rename refused by the two listed sizes; the menu acts on its own place; a file Import reads is not touched. |
| 5d9019486 | Boot import's stick buttons and file menus were drawn off the stick card; a rename onto a vault's name was not refused while the list stands. | Refused while the list stands; drawing matches the stick card. |
| 0ff5ce3f8 | Files' card button offered Sign on a PSBT no loaded wallet can sign or already signed; a text file showed a button. | The button is the first entry of the shared "What you can do" list. |
| f35bac78e | Files' card of recognised files said "From the stick" though not on it; a wallet the card loads had a second Load. | Card named for what it is; one Load. |
| ee9b0bceb | A Files rewrite dropped features: row button on a narrow row, open vault not reading open, Outbox state and unsaved changes missing, log not wrapping. | The page carries back each dropped behaviour. |
| f43883d71 | Small panel: the In Faraday list redrew the whole frame each scroll step. | List starts below the corners; scroll redraws only the list. |
| e05d3b4c9 | The Stick tab on the small panel did not scroll as one page. | Scrolls as one page; Import buttons card-wide. |
| 5db482b7c | At 1024x600 the notice button and Scan button had no gap; press area not whole. | A gap keeps both whole. |
| 77f4fb6f0 | Files' wallet rows and stick rows ran under the scrollbar and each other, below the touch floor. | Rows keep the touch floor, clear of the scrollbar. |
| d2fb2e5f1 | Stick card chip, Delete files and Unselect all were below the touch floor on a small panel. | They meet the touch floor. |
| 01d598097 | The line under the In Faraday list said "1 more" for a row not hidden and counted rows not below the foot. | Counts rows whose bottom is below the foot, as drawn. |
| f8df51208 | At 480 wide with a stick, Select all and the summary sat ahead of the file rows. | File rows first; Select all beside Write. |
| 4629df472 | In a file's details a no-icon row moved into the padding where its label did not fit. | The row takes the icon column its action has. |
| 33d5be32b | A locked vault file's row did not say Unlock. | The Unlock label is right. (The same commit groups addresses in fours elsewhere.) |
| 2efdf3d57 | A file name Windows cannot open (CON, COM1 and similar) was accepted. | Windows device names are refused and get a suffix. |
| dc2bc0513 | On the small panel a vault's header in Files ended in "..." rather than wrapping. | The header wraps and its card grows. |
| 1a5cc4e15 | A file in Files with the same name but other bytes overwrote or was lost; a vault row named the wrong vault. | A differing same-named file is kept under a numbered name and said; a row names its own vault or none. |
| c01bac0e2 | Inbox Load loaded wallets the person had unticked; a passphrase whose key was already loaded made the wrong potential wallet. | Load loads only ticked wallets; the potential wallet is that of the key the passphrase made. |
| 919b82756 | A save of Inbox and Outbox removed the old files first and stopped at the first error, so a lock could lose part of the folder. | Every file is written through a temporary file and a rename; only files no longer listed are removed; one bad name does not stop the rest; the first error is returned at the end. |
| 3da328e7c | A scanned file could get a `scanned-N` name already used in the Inbox, also after a lock restored it; names differing only in case were not the same file; four name finders disagreed. | One case-insensitive name finder; scanned names are unused in the Inbox. |
| b7cff95ac | A signed message whose body held seed words was read as a public signed message. | Secret-bearing bodies are not public signed messages; dropped at lock. |
| a824ef856 | A signed message whose body held a secret was not marked secret. | Marked secret; the body is checked only for a signed message with a valid address. |
| 1e93c1177 | A QR vault's `.pgp` in the Outbox was listed as public; Files offered Open in QR vault for any file. | A QR vault is sealed under its passphrase; Open in QR vault only for a QR vault file. |
| 8025b46cc | A made key's `.keyx` stayed listed in the Outbox only sometimes; it could be removed while its vault was sealed or read-only. | A pending key stays listed until its vault can take it back. |
| f47648598 | An encrypted backup needing more Argon2 memory than free reached its passphrase prompt; the Inbox's sealed-backup prompt did not compare memory first. | Refused before the passphrase is tried, naming both amounts. |
| d5fd12b64 | A delete sheet's files came from the wrong stick; a file deleted from one stick unticked the same name on another. | The files are those of the stick the sheet was opened for. |
| dad3b6c94 | After a stick file's read answer was taken by a backup flow or another handler, the file stayed marked as being read, so Copy again did not ask for it. | A read's answer ends its being asked for, whichever handler takes it. |
| d7492bfc2 | A crafted stick's directory listing could loop, show entries past the directory's end on a write, and let names and labels reach the screen unchecked. | The listing is read once; entries after the end are not shown; names are bounded. |

## Doubts

- Rows left out as not about this feature: a7c9aed6c and 578f5021a (the boot import's Import button words; carried in `boot-import`); f797db66b and 1c57be5b2 (a loaded key's details and Restore from them); 396b4f0c5 (the top bar name and action labels at 480 wide). The Argon2 cgroup part of 919b82756 belongs to vault unlock.
- d5fd12b64 and dad3b6c94 came with the stick shell's rows and are carried here.
- An earlier report said Import is offered "only when no vault is open"; the code says the opposite (a vault must be open). The correction is carried.
- Line numbers in `faraday/faraday-core/src/screens/visit.rs` and most of `faraday/faraday-core/src/screens/files.rs` come from one report each and were not re-read; only the ones marked plainly were checked.
- `inbox_actions` (`faraday/faraday-core/src/screens/files.rs:~1340-1361`) always returns `note = None`: apparently a dead return.
- Doc comments: `faraday/faraday-core/src/screens/files.rs:1` says "Inbox and Outbox" though the file has no Outbox code; `~560-565` stacked docs.
- `faraday/faraday-core/src/screens/visit_same.rs` (drawing) and `faraday/faraday-core/src/stick/visit_same.rs` (logic, `SameFiles`) are two distinct modules with the same name.
