# The Check page

The Check page (`Screen::Check`) is a tile grid of the checks a person can
hold up against a hardware wallet or a wallet app (F255, the Check page as a
tile grid): a seed against a wallet, an address, a transaction before
signing, what a signer signed, a transaction's signatures, a dice seed, a
multisig setup, a backup in another form, a signed message, child seeds and
an xpub's other spellings. Each tile opens the Tools tool it names. It is
reached from the sidebar's Check and from the Tools catalog's Check tile.
Code: `faraday/faraday-core/src/check/mod.rs` (rows, groups, presses),
`faraday/faraday-core/src/check/screen.rs` (drawing).

## Screens and sheets

- `Screen::Check` (`draw`, `faraday/faraday-core/src/check/screen.rs:29-61`):
  the tiles on the menu page container (`menu_page::draw`), scrolling where
  the panel is short; the top app bar carries the name
  (`EN.compact_check`) and the way back. Every tile opens (`opens: true`);
  tiles that work from the seed carry the `EN.check_seed` tag.
- Panes (`faraday/faraday-core/src/check/screen.rs:20-25`): the session pane
  in Check mode (`Left::Session(Some(PaneMode::Check))`), one tile that
  fills, stacking, one column when narrow.
- Groups and rows (`GROUPS`, `faraday/faraday-core/src/check/mod.rs:105-192`):
  - Wallet: My seed makes my wallet (`Row::SeedCheck`, seed), Check an
    address (`Row::Address`).
  - Signing: Decode a transaction (`Row::Transaction`), Check a signer
    (`Row::Signed`), Nonce check (`Row::Signatures`).
  - Setup: Check a dice seed (`Row::Dice`, seed), Check a multisig setup
    (`Row::Multisig`), Check a backup (`Row::Backup`, seed).
  - Messages: Check a signed message (`Row::Message`), Child seeds
    (`Row::Child`, seed), Convert an xpub (`Row::Xpub`).
- Check a backup menu: `Sheet::CheckBackups`, `Sheet::CheckBackupsOther`
  (the menu with Other forms open), drawn by `backups_dialog`
  (`faraday/faraday-core/src/check/screen.rs:64-131`, dispatched at
  `faraday/faraday-core/src/screens/mod.rs:568`): a dialog headed `EN.catalog_check_a_backup`
  with a row per form, an Other forms button with a chevron, and
  Cancel. Forms up front and under Other forms come from
  `ways_in::CHECK_BACKUP`. On a one-column panel Other forms is
  `Sheet::OtherForms(Opening::CheckBackup)` instead.
- Menu labels (`Backup::label`, `faraday/faraday-core/src/check/mod.rs:51-63`):
  `EN.check_menu_slip39`, `check_menu_xor`, `check_menu_codex32`,
  `check_menu_krux`, `check_menu_seedqr`.

## Actions

- `Press::Row(Row)`, `Press::Form(Backup)`, `Press::Other`
  (`faraday/faraday-core/src/check/mod.rs:79-88`); Cancel on the dialog
  (`Action::Cancel`, `faraday/faraday-core/src/check/screen.rs:71`).
- `press` (`faraday/faraday-core/src/check/mod.rs:195-245`):
  - `Row::Dice` opens New key at its dice source (`keygen_open_dice`,
    `faraday/faraday-core/src/check/mod.rs:197`).
  - `Row::Backup` opens `Sheet::CheckBackups` (`198`).
  - `Press::Other` on a one-column panel opens
    `Sheet::OtherForms(Opening::CheckBackup)` (`200-202`); otherwise it
    toggles between `CheckBackups` and `CheckBackupsOther` (`203-207`).
  - Other rows go to a Tools tile (`209-222`): SeedCheck to `Go::SeedCheck`,
    Multisig to `Go::MultisigCheck`, Address to `Go::CheckAddress`,
    Transaction to `Go::Decode`, Signed to `Go::SignerCheck`, Signatures to
    `Go::NonceCheck`, Message to `Go::CheckMessage`, Child to `Go::Bip85`,
    Xpub to `Go::Calculator(3)`.
  - `Press::Form` closes the sheet (`225`), then: Krux opens Add a key
    (`Action::Entry(None)`) and chooses its Krux code form when the screen is
    Entry (`227-234`); SLIP-39, Seed XOR and codex32 go to
    `Go::AddKey(form)` (`236-238`); SeedQR to `Go::SeedQr` (`241`).

## Effects

- `app.act(Action::Catalog(i))` for the tile
  (`faraday/faraday-core/src/check/mod.rs:255`); a toast with the tile's
  need (`257`); `app.sheet` set; `app.keygen_open_dice()`;
  `app.act(Action::Entry(None))` then `entry.choose_form(Form::Krux)`.
- Nothing written, no camera, no jobs of its own.

## State and lock

- No state of its own: `Row`, `Backup` and `Press` are copy enums and
  `GROUPS` a constant. The open sheet is the app's `sheet`. Nothing is held
  at a lock.

## Reach

- Opens: `decode` (Decode a transaction, Check a signer, Nonce check),
  `messages` (Check a signed message), `tools` (Convert an xpub calculator),
  My seed makes my wallet, Check an address, Check a multisig setup, Child
  seeds (`child-keys`), New key's dice source (`keygen-flow`), Add a key on a
  form (`add-key`), SeedQR scan.
- Reads: `catalog::{Go, TILES}`, `tile_opens`, `tile_need`, `toast`,
  `ways_in::{Way, CHECK_BACKUP}`, `forms::Form`, `entry.choose_form`,
  `screens::hub_layout::{Panes, Left, Tile, Narrow}`, `PaneMode::Check`,
  `screens::menu_page::{draw, Page, Section}`,
  `screens::catalog::{Face, Mark}`.
- Opened from: the sidebar, the Tools catalog (`Go::Check`); presses come in
  through the action dispatch (`check::press`).

## Rules and refusals

- A row whose tool is not built is not in `GROUPS`
  (`faraday/faraday-core/src/check/mod.rs:103-104`).
- The backup menu offers only forms from `ways_in::CHECK_BACKUP`
  (`backup_of`, `faraday/faraday-core/src/check/mod.rs:67-77`): SeedQR and
  table forms; Scan, Words, Aezeed, Holder and Add a key are not choices.
- Words, Xprv and HexSeed are typed on Add a key's own tile and are never in
  the menu; chosen from Check they do nothing
  (`faraday/faraday-core/src/check/mod.rs:59`, `240`).
- `tile` (`faraday/faraday-core/src/check/mod.rs:250-259`): an unknown Go
  target does nothing; a tile that opens is pressed as the catalog presses
  it; a tile that cannot open shows its need as a toast (the catalog's
  `tile_need` rules, `faraday/faraday-core/src/catalog.rs:617-658`, for
  example Check a signed message with nothing to check gives
  `catalog_copy_a_signed`).
- A card's title or line that does not fit wraps and the card grows.

## Domain calls

None. The page calls only app-level code (`catalog`, `ways_in`, `forms`,
`menu_page`).

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 50b5534de | On the Check page a card's title or line that did not fit ended in an ellipsis | A card's title or line wraps and the card grows |

## Doubts

- Rows given to this feature and placed elsewhere: 7d0e9cdd7 (BIP-322
  strict DER) in `messages`; a7fef6f08 (Check a signer's closed card) in
  `decode`; 264b056e4 (descriptor checksum repeated keys) in `tools`.
  b28b434d8 and db93e7cbb are about Check a timestamp (`ots_check`), which
  is not in this cluster; left out.
- `Row::Xpub => Go::Calculator(3)` hard-codes a `TILES` index
  (`faraday/faraday-core/src/check/mod.rs:220`).
- `Row::Dice | Row::Backup => return`
  (`faraday/faraday-core/src/check/mod.rs:221`) is unreachable: both are
  handled earlier in the match.
- Earlier accounts gave `faraday/faraday-core/src/check/screen.rs` lines
  267-390 for the draw, panes and dialog; the file has 131 lines. The lines
  above were read from the code.
