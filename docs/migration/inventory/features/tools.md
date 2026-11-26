# The Tools page and calculators

The Tools page is the catalog of every tool, grouped into tiles with a "Find
a tool" field; a press opens the tool or says what it needs first. The
calculators screen (`Screen::Tools`) holds six small offline tools: Hashes,
Encodings, Descriptor checksum, Convert an xpub, Units and the Miniscript
policy compiler. The same module also hosts the dispatch for several other
tools and the Lightning invoice decoder and Lightning node key tool reached
from the catalog. It is for anyone on the device; nothing here signs or
stores secrets. Reached from the sidebar's Tools, Home search, and the Check
page (Xpub row opens Convert an xpub). Code:
`faraday/faraday-core/src/catalog.rs` (tiles and presses),
`faraday/faraday-core/src/screens/catalog.rs` (drawing the catalog),
`faraday/faraday-core/src/tools/{mod,screen,flow,search}.rs` (calculators),
`faraday/faraday-core/src/invoice/`, `faraday/faraday-core/src/lightning/`.

## Screens and sheets

Tools page (catalog):
- Tiles in 9 groups (`faraday/faraday-core/src/catalog.rs:~115-125`; tiles
  `~128-570`) with "Find a tool": every typed word must match; letters,
  digits and spaces are folded (`~584-603`).
- `Sheet::GpgTile`: a GPG key with no resumable flow asks make or restore
  (`~675-678`).
- Tiles open, among others, `Screen::Check`, `SpendBackup`, `Spend`,
  `Vaults`, `VaultContents`, `Codes`.

Calculators (`Screen::Tools`, `faraday/faraday-core/src/tools/screen.rs:~38-61`):
- `Tool::{Hashes, Encodings, Descriptor, ConvertKey, Units, Miniscript}`
  (`faraday/faraday-core/src/tools/mod.rs:~55-82`); one open at a time
  (buttons `faraday/faraday-core/src/tools/screen.rs:~316-327`).
  - Hashes: SHA-256, SHA-256d, HASH160 of typed text, read as Auto, Text
    or Hex (`ReadAs::ALL`, `faraday/faraday-core/src/tools/screen.rs:~338-349`,
    `~491-497`).
  - Encodings: Base58Check, Base58, Bech32, Bech32m, Hex: prefix, version,
    checksum holds, bytes as hex (`~498-521`). Hex is tried first, before
    plain Base58 (`core/faraday-codec/src/encodings.rs:~56-58`). A bech32
    string that is a segwit address (bc, tb, bcrt) shows its witness
    version in Version and its program in Bytes.
  - Descriptor: checksum, whether a given checksum holds, the
    with-checksum form, the wallet on this device, a repeated-key warning,
    Take to Wallets (`~522-559`, `~630-643`).
  - Convert an xpub: fingerprint, depth, child, every SLIP-132 spelling; an
    extended private key gets a note and no rows (`~560-577`).
  - Units: one amount in sat, BTC, mBTC, bits; up to 6 amounts as tabs
    (`MAX_AMOUNTS`, `faraday/faraday-core/src/tools/mod.rs:~88`;
    `faraday/faraday-core/src/tools/screen.rs:~217-298`).
  - Miniscript: a policy compiled to a descriptor, wsh or tr; spend-path
    count, wallet status; no Take (`~355-364`, `~592-606`).
- Flow: Input then Result, one card on a non-compact window; Units is one
  card without Next (`faraday/faraday-core/src/tools/flow.rs:~27-32`,
  `~51-54`, `~65-67`; `faraday/faraday-core/src/tools/screen.rs:~53-55`,
  `~113-211`). The compact panel draws the page, not a card.
- Ways in (Scan, From Files, Type) for Descriptor, ConvertKey and
  Miniscript only (`takes_codes`, `faraday/faraday-core/src/tools/mod.rs:26-30`).
  Scan opens `Sheet::Scan` with `ScanPurpose::ToolText` (`~786-797`).
- Demo only: a "Demo only" card with `Action::UseSample`
  (`faraday/faraday-core/src/tools/screen.rs:~393-396`; `type_sample`,
  `faraday/faraday-core/src/tools/mod.rs:~399-406`).

Lightning tools (from their module docs only):
- Decode a Lightning invoice (`faraday/faraday-core/src/invoice/`): a BOLT
  #11 invoice or a BOLT #12 offer (`lno1`), typed, pasted or scanned, taken
  apart: network, amount, description, payee, payment hash, creation and
  expiry, the other fields. Read only; nothing is paid, signed or stored;
  leaving the screen drops the text
  (`faraday/faraday-core/src/invoice/mod.rs:1-13`). The description and
  issuer are drawn on one line.
- Lightning node key (`faraday/faraday-core/src/lightning/`): a node's key
  from a loaded key read as ldk-node reads a BIP-39 seed, or from an LND
  aezeed typed here; the public key is public; the private key goes into the
  vault or out only through the secret sheet
  (`faraday/faraday-core/src/lightning/mod.rs:1-6`).

## Actions

Catalog (`catalog_go`, `faraday/faraday-core/src/catalog.rs:~683-803`):
- An out-of-range index or a tile that cannot open does nothing (`~684`).
- `Go` variants: Sign (StartSpend if a PSBT is loaded, else
  `ScanFor(Sign)`), NonceCheck (a finished spend goes to Spend's FINISH
  step, else the nonce tile), NewKey, LoadKey, NewCodex32, AddKey(form),
  SeedQr (Entry then ScanSeed), NewShares, MakeXor (Convert to Seed XOR),
  CreateWallet, Restore, Backup(wallet), Check, SpendBackup, Vaults,
  Explore, Bip85, Convert, KefCode, Vanity, Silent, Lightning, SeedCheck,
  Sweep, Decode, Invoice, CheckAddress, CheckTimestamp, ReadCoreWallet,
  MultisigCheck, CheckMessage, SignMessage, Scan, CreateVault, Codes,
  VaultCategory(c), WordList(k), Calculator(t).
- `Action::Catalog(i)` reaches `catalog_go`
  (`faraday/faraday-core/src/action.rs:1944`).

Calculators:
- `Action::Tools` runs `tools_open`
  (`faraday/faraday-core/src/tools/mod.rs:~770-772`, `~715-718`), which
  remembers `back`.
- `TTool(i)` (`~617-627`; clears the field and the Units amounts);
  `TMode(i)` (Hashes read-as; Units unit clamped 0..3 with trailing places
  dropped, `~634-641`; Miniscript wsh or tr, `~628-649`); `TClear` (`~650`);
  `TAddAmount` / `TAmount(i)` / `TRemoveAmount(i)` (`~651-653`); `TStep(k)`
  (Input 0, Result 1; Result only with text and not for Units,
  `~655-663`); `TScan` (`~778`, `~786-797`); `TFile(i)` (`~779`,
  `~801-826`; a disabled row uses a `TFile(usize::MAX)` sentinel,
  `faraday/faraday-core/src/tools/screen.rs:~445-449`); `TKey(i, n)`
  (`~780`, `~830-838`); `TTake` (`~774-776`, `~881-903`);
  `TypeHere(Here::Tools)`; `Nav(t.back)`.
- Keys: typing and Backspace edit the field; Escape clears and returns to
  `back` (`faraday/faraday-core/src/tools/mod.rs:~670-693`, `~906-935`).
  Home search typing, Backspace and Escape (`~697-710`, `~939-959`).
- Scanned text for a code tool: `tools_scanned` fills the field and closes
  the scanner (`~857-876`).
- Tools also dispatches the Convert, Krux code (`kef_tool`), Check a
  timestamp (`ots_check`), My seed makes my wallet (`seed_check`) and Read
  a Bitcoin Core wallet (`core_wallet`) tools and tidies `multisig_check`
  (`tool_act`, `tool_key`, `tool_tidy`,
  `faraday/faraday-core/src/tools/mod.rs:732-767`). Invoice and Check an
  address take typing through the Tools key path (`~911-916`).
- Self-test glue: `selftest`, `selftest_passed`, `selftest_failed`
  (`~1004-1021`, `~1031-1033`).

## Effects

- Camera: `camera.open(scan)`, `Command::CameraOn`
  (`faraday/faraday-core/src/tools/mod.rs:~792`, `~795-796`).
- Toasts: too long (`~844`, `~870`), the add-wallet refusal (`~901`).
- Files read from `self.inbox.get(i)` as UTF-8 lossy (`~805`, `~824`,
  `~834`).
- Take to Wallets: `session.add_wallet(..Origin::Named..)`,
  `wallets.set_wallet(i)`, `refresh_spend()` (`~881-903`).
- Costly work: `ToolsArea::keys` runs `keys.account_text(c, i,
  NewKind::Multi, 0)` per loaded key (`~530-541`); reading calls `compile`
  (`~594`), `checksum_facts` (`~589`), `key_facts` (`~591`), `hashes`
  (`~585`), `read_input` / `encoding_of` (`~584`, `~587`), cached in
  `ToolsState.read` (`~566-575`, `~598-606`).
- Scroll `list_offset` (`~953`, `~973`); `flow scroll.follow` (`~662`).

## State and lock

- `ToolsArea` (`faraday/faraday-core/src/tools/mod.rs:~314-353`):
  `tools: Option<ToolsState>`, `home_find: Text`, `home_find_kept: Text`,
  `home_focus`, `home_bar_y`, `pills_up`, `home_follow`,
  `flow_head: RefCell<Option<(Text, Option<(Text, Action)>)>>`, and the
  sub-areas `convert`, `kef_tool`, `ots`, `seed_check`, `mscheck`,
  `core_wallet`, `invoice`, `address_check`.
- `ToolsState` (`~97-127`): `tool`, `typed: SecretText` (the field; an
  extended private key typed is refused but was typed), `read_as`, `unit`,
  `script`, `amounts: Vec<UnitAmount>` (each `typed: SecretText`), `amount`,
  `back`, `open`, `scroll`, `keys_of`, `read: RefCell<Option<KeptReading>>`.
- `KeptReading` (`~236-244`): `typed: SecretText` (wiped on drop),
  `keys: Vec<(Text, Text)>` public account keys, `reading: Rc<Reading>`
  (hashes, decoded bytes, descriptor facts, key facts). `Reading` is not
  itself a wiping type; for Hashes and Encodings it holds bytes derived from
  whatever was typed.
- `SecretText` wraps `Text` and wipes
  (`faraday/faraday-intake/src/secret_buf.rs:17-25`;
  `core/faraday-text/src/text.rs:363-371`).
- Leaving: `tools` is dropped when the screen is not `Screen::Tools`
  (`tidy`, `faraday/faraday-core/src/tools/mod.rs:512-515`), on Escape
  (`~686-688`) and at a lock. `tidy` also clears `invoice` unless
  `Screen::Invoice` is up and releases `address_check` unless its card is
  open (`faraday/faraday-core/src/tools/mod.rs:512-526`). Transfer's text is
  cleared when leaving Transfer (`~763-766`, Demo feature).
- Lock (`AtLock for ToolsArea`,
  `faraday/faraday-core/src/tools/mod.rs:1084-1112`): `tools = None`;
  `home_find_kept.clear()`; `flow_head.take()`; locks `convert`,
  `kef_tool`, `seed_check`, `mscheck`, `core_wallet`. Kept as public:
  `home_find`, `home_focus`, `home_bar_y`, `pills_up`, `home_follow`,
  `ots`, `invoice`, `address_check`. Holds: the calculator's typed line
  counts as held when non-empty, plus the holds of convert, kef_tool,
  seed_check, mscheck and core_wallet (`~1075-1082`). A lock then restarts
  the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- The catalog has no state (static `TILES`).
- Lightning node key: `LightningArea` has its own lock
  (`faraday/faraday-core/src/lightning/mod.rs:894`); what it clears was not
  read.

## Reach

- Out: `crate::flow::{Contained, Pick, Row, contained, Page, pair_left}`,
  `trail::{StepFlow, Steps}`, `wallet::{KeyTexts, fp_text, Session, Origin,
  Refusal, FileKind}`, `create::{NewKind, read_keys}`, `qr_view::ScanState`,
  `ScanPurpose::ToolText`, `self.camera`, `self.inbox`, `item::Item`,
  `convert/kef_tool/ots_check/seed_check/core_wallet::tool`,
  `multisig_check::tidy`, `stick::keep_focus_off`, `invoice::key`,
  `address_check::{key, held_for, release, clear}`,
  `scan_read::{secret_kind, Kind}`, `screens::{in_flow, bar_on, button_rows,
  bar_head, catalog::cards_matching}`, `pick::{Decl, Take, PickRow}`,
  `samples::put::offer`, `catalog::{TILES, matches, words_in}`,
  `drawing::View`, `compact`
  (`faraday/faraday-core/src/tools/mod.rs:735-752`, `742`, `760`).
- Tools search reads `faraday_guide::glossary`
  (`faraday/faraday-core/src/tools/search.rs:32`).
- In: the catalog screen and Home read `search::hits`;
  `ToolsArea::{act, key, reading, state}`; `Faraday::tool_tidy` after
  presses.
- The catalog opens nearly every feature: `spend-signing`, `decode`,
  `messages`, `spend-backup`, `check-page`, vaults, keys, backups and the
  other tools.

## Rules and refusals

Catalog (`tile_need`, `faraday/faraday-core/src/catalog.rs:617-658`):
- Backup with no wallet loaded: `catalog_load_a_wallet`.
- Sweep outside key-making mode: `practice_refused`.
- SignMessage with no single-key wallet: `catalog_needs_a_single`; with
  every offered wallet waiting for its key's backup: `message_waits_backup`.
- CheckMessage with no Message file and nothing to paste:
  `catalog_copy_a_signed`.
- VaultCategory of a backup-flow kind with a resumable flow:
  `backup_flow_continue_make`; a backup-flow kind otherwise needs no vault.
- VaultCategory or Codes with no open vault: the make-a-vault or
  unlock-a-vault line, still pressable, leading into making or unlocking
  (`~663-665`).

Calculators:
- Six tools; an out-of-range index is ignored
  (`faraday/faraday-core/src/tools/mod.rs:~617-618`).
- Typing is refused on the Result card (`~920-929`) and when the screen is
  not Tools (`~917-919`).
- The field holds at most 1024 characters in total; a typed character past
  it is dropped (`faraday/faraday-core/src/tools/mod.rs:678-679`).
- Scan, File or fill text is at most `FIELD_MAX` = 8192 characters, toast
  `tools_too_long` (`faraday/faraday-core/src/tools/mod.rs:23-24`,
  `843-846`, `~869-872`).
- Files: only Wallet, Key and Text kinds (`file_fits`,
  `faraday/faraday-core/src/tools/mod.rs:26-37`); secret files are not
  offered.
- Scanned codes: only `ScanPurpose::ToolText`; secret or other kinds are
  named on the scanner and nothing goes in (`~852-868`); the refusal is in
  `scanned_plain_text`.
- A multi-key watch-only file in Convert gives each key a row; a single
  key fills (`~813-822`; `faraday/faraday-core/src/tools/screen.rs:~459-472`).
  The origin is stripped by `bare_xpub` (`~49-52`, `~816`, `~836`).
- Miniscript: "keys need origins" when there is no wallet
  (`faraday/faraday-core/src/tools/screen.rs:~598-602`); refusal notes by
  kind (`faraday/faraday-core/src/tools/mod.rs:~993-1000`). Under Taproot a
  policy of one key alone (`pk(A)`, `thresh(1,pk(A))`) is refused as "One
  key alone has no script under Taproot"
  (`core/faraday-bip/src/compile.rs:~55`); no `tr(A)` is produced; segwit
  `pk(A)` compiles.
- Descriptor: Take only if it loads as a wallet here
  (`faraday/faraday-core/src/tools/screen.rs:~539`, `~630`); an add-wallet
  refusal toasts `e.text()`, a held confirmation returns silently
  (`faraday/faraday-core/src/tools/mod.rs:~887-903`). Repeated keys give a
  warning row (`faraday/faraday-core/src/tools/screen.rs:~548-556`,
  `~618-622`); the repeated-key check covers a wildcard and a fixed index it
  reaches, a parent and child, and a plain key deriving one key; a two-part
  policy with `/<M;N>/*` is read as a descriptor.
- Units: sat clamped to `MAX_SATS` = 2.1e15, silently
  (`faraday/faraday-core/src/tools/mod.rs:~84-85`, `~294`); at most 6
  amounts (`~181-189`); the last cannot be removed (`~191-196`); digits
  only, one point only where the unit has places (BTC, mBTC, bits, to one
  satoshi), places capped (`~297-307`, `~676-677`, `~259-273`); a unit
  change drops extra places (`~634-641`).
- The reading cache is reused only when tool, text, read mode, script,
  network and keys match (`~566-575`).
- The sample is offered only in Demo
  (`faraday/faraday-core/src/tools/screen.rs:~393-396`;
  `faraday/faraday-core/src/tools/mod.rs:~763`, `~907`).
- Home search `home_find` is a public `Text`, capped at 40 characters,
  control characters refused (`faraday/faraday-core/src/tools/mod.rs:700`).

Lightning:
- On Mainnet without Demo, a Core Lightning `hsm_secret` made from a
  sample's words and a SLIP-39 set of a sample are refused with the
  public-sample line; Testnet takes both.

## Domain calls

- `faraday_bip::{compile::{PolicyScript, compile, Refusal, PolicyFacts},
  hashes::hashes, descriptor::checksum_facts, slip132::{key_facts,
  KeyReading}, keys::{Network, ScriptType}}` (`core/faraday-bip/src/`).
- `faraday_codec::encodings::{read, read_input, ReadAs, Reading, Encoding}`
  (`core/faraday-codec/src/encodings.rs`).
- `faraday_codec::bolt11::decode`, `faraday_codec::bolt12::decode`
  (invoice).
- `faraday_bip::aezeed` (Lightning node key).
- `faraday_crypto::{work::Compute, hex::encode_string}`; `faraday_text`;
  `faraday_keys::Keys::{all, account_text}`; `faraday_shell_api::{Key,
  Command}`; `faraday_selftest::{Outcome, Check, CHECKS, run_checks}`;
  `faraday_guide::glossary`; `zeroize::Zeroizing`
  (`faraday/faraday-core/src/tools/mod.rs:~857`).

## Fixtures

- `faraday/faraday-samples/data/hsm-secret`
- `faraday/faraday-samples/data/hsm-secret-encrypted`
- `faraday/faraday-samples/data/hsm-secret-words`

(The Lightning samples, `faraday/faraday-samples/src/data.rs:705-719`; the
Lightning tool lists `hsm_secret` files in Files as buttons to read,
`faraday/faraday-core/src/lightning/screen.rs:~381`. The Units factor table is in code,
`faraday/faraday-core/src/tools/mod.rs:~259-264`.)

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| c535fcc16 | An invoice's or offer's description and issuer could draw a line break as an extra row (and a stick grant fault in the same commit) | Description and issuer on one line |
| bcf651f80 | On Mainnet with no Demo, an `hsm_secret` from a sample's words and a SLIP-39 set of a sample were accepted | Mainnet with no Demo refuses published sample material in both forms |
| dc2577558 | `faraday-codec` did not build without its decode feature (the BOLT #11 parser's import) | Build-only fix; no behaviour change |
| 275a04edf | Under Taproot, a one-key policy showed rust-miniscript's raw "Empty Miniscript compilation" | The same refusal, worded "One key alone has no script under Taproot" |
| 4dce709d2 | For a segwit address, the Encodings Bytes row was the 5-bit groups with the witness version, shifted | Witness version in Version, program in Bytes |
| 69002a128 | A hex string was read as Base58; non-hex was read as bytes | Hex is tried first; hex that is not hex is no bytes |
| 264b056e4 | The descriptor checksum tool missed repeated keys; a two-part `/<M;N>/*` policy read as no descriptor | Repeated-key refusal covers these; the policy is read as a descriptor |
| 4598a2dca | The Units calculator took no point (one of three faults in the commit) | Units accepts a point in BTC, mBTC and bits, to one satoshi |

## Doubts

- 264b056e4 was given to `check-page` and 4598a2dca to `spend-signing`;
  both placed here for the calculator parts.
- Left out: ead91002f (base64 padding bits refused, in
  `core/faraday-bip/src/base64.rs`; the Encodings calculator uses
  `faraday_codec`, so the row may not be about this feature); 2763f2157 (a
  cross-check script, `tools/scripts/signer-core.py`, not the Tools page).
- The inventory has no code account of the Lightning node key tool, the
  invoice decoder or the other dispatched tools (Convert, Krux code, Check a
  timestamp, My seed makes my wallet, Read a Bitcoin Core wallet, Check a
  multisig setup); only their module docs and fix rows are here.
- `Reading` (bytes derived from what was typed) sits in an `Rc` not known
  to wipe; the `hide_shown` doc in `faraday/faraday-core/src/tools/flow.rs:~112-115`
  says the typed text is public while
  `faraday/faraday-core/src/tools/mod.rs:~101-102`, `~1075-1076` say an
  extended private key can be typed. Not resolved.
- `ots` is declared public at lock and not cleared in the code read;
  `invoice` is cleared by `tidy` when the screen is left.
- `type_descriptor` (`~379-388`) and `run_selftest_with` (`~1023-1035`) are
  testkit-gated items in product code.
- Whether Take to Wallets persists the wallet to storage is unclear.
- The catalog's 9 groups and their tile list were not re-read; tile line
  numbers are approximate.
- Plain line numbers in `faraday/faraday-core/src/invoice/mod.rs:1-13`,
  `faraday/faraday-core/src/lightning/mod.rs:1-6`, `894`,
  `faraday/faraday-core/src/action.rs:1944` and
  `faraday/faraday-samples/src/data.rs:705-719` were read from the code at
  `d89dde5de`; `~` anchors into `core/` were found by searching it.
