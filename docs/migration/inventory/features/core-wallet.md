# Reading a Bitcoin Core wallet

Read a Bitcoin Core wallet (`Screen::CoreWallet`, a Tools tile) brings a wallet in from a Bitcoin Core `wallet.dat` file (SQLite) in Files.

- The person picks the file.
- For an encrypted wallet, they type its passphrase.
- The device reads the file off the drawing path.
- It shows the active descriptors, failed records and kept lines.
- The person can take an active receive-and-change pair as a session wallet.

In Practice, the tool uses two sample wallet files.

## Screens and sheets

Files: `faraday/faraday-core/src/core_wallet/mod.rs`, `faraday/faraday-core/src/core_wallet/tool.rs`, `faraday/faraday-core/src/core_wallet/screen.rs`.

- `Screen::CoreWallet`. Its scroll is `Scrolled::CoreWallet`.
- It has two cards:
  - File: the chosen file and the passphrase field.
  - Result: the verdict, facts, descriptors, kept lines and failed lines.
- The landing page (`faraday/faraday-core/src/core_wallet/screen.rs:~63-109`) lists the wallet files in Files, or shows an empty part.

## Actions

- `CoreWalletAction` (`faraday/faraday-core/src/core_wallet/tool.rs:~31`):
  - `Open`, `OpenFile(i)`;
  - `Step(k)`;
  - `File(i)`;
  - `Focus` (on the passphrase);
  - `Read`;
  - `Take(i)`;
  - `Another`.
- Keys in the passphrase field (`faraday/faraday-core/src/core_wallet/tool.rs:~721-728`):
  - Enter reads;
  - Escape stops typing.
- Practice: "Use faraday" fills the sample passphrase (`fill_faraday`, `faraday/faraday-core/src/core_wallet/tool.rs:~620-625`).
- The tile (`open_tile`, `faraday/faraday-core/src/core_wallet/tool.rs:~750-761`):
  - exactly one wallet file opens it;
  - several open the landing page;
  - none opens Practice's sample.

## Effects

- Reading:
  - The read is the worker job `Job::CoreWalletRead(Box<ReadWork>)`, labelled `EN.working_core_wallet` (`faraday/faraday-core/src/core_wallet/tool.rs:~580-584`).
  - It runs `read_file(c, ...)` off the drawing path (`~535-545`).
  - The Practice sample reads at once, on the drawing path (`read_now`, `~590-595`, `~785-788`).
- Practice pushes the two sample wallet files into `app.inbox` if they are absent (`~775-783`). The bytes come from `app.put_on()` (`~767`).
- Take (`faraday/faraday-core/src/core_wallet/tool.rs:652-662`) runs, in order:
  1. `app.session.add_wallet(&c, &name, &text, &source, Origin::Named)`;
  2. `app.wallets.set_wallet(i)`;
  3. `app.screen = Screen::Wallets`;
  4. `app.refresh_spend()`.
- Toasts (`~638`, `~665`, `~718`).
- Idle closes the eye: `app.typing.pass_eye.close()` (`~564`, `~734-736`).

## State and lock

- `CoreWalletState` (`faraday/faraday-core/src/core_wallet/tool.rs:~102-121`) lives in `app.tools.core_wallet`. It holds:
  - open, landing, scroll, back;
  - `file: Option<Text>`;
  - `passphrase: PassField`, which is secret;
  - typing, refused;
  - `shown: Option<Shown>`: verdict, facts, descriptors, kept lines and failed lines, as text;
  - `take: Vec<TakePair>`;
  - `take_refused`.
- `ReadWork` (`~520-526`) is consumed by the worker. It holds:
  - `bytes: Zeroizing<Vec<u8>>`;
  - `beside: Vec<Text>`;
  - `mode`;
  - `passphrase: Option<SecretText>`.
- `ReadDone` holds `Result<Wallet, Refusal>`. `Wallet` (`faraday/faraday-core-wallet/src/lib.rs:369-400`) holds nothing secret: descriptors, labels, the `mkey` method and rounds (`faraday/faraday-core-wallet/src/lib.rs:343-352`), the passphrase's verdict and counts. Private keys give only pass or fail (`faraday/faraday-core-wallet/src/lib.rs:28-32`). Its texts are `Text`, which wipes on drop (`core/faraday-text/src/text.rs:363`).
- The passphrase is taken, and so cleared, as the read begins, whatever the result (`faraday/faraday-core/src/core_wallet/tool.rs:561`).
- Lock (`faraday/faraday-core/src/core_wallet/tool.rs:852-858`):
  - `holds` is typed when the passphrase has text or a file is chosen (`holds_secret`, `~149-151`).
  - `lock` = `clear()`, which drops the state.
- Leaving: `tidy` clears the tool when the screen is not CoreWallet (`faraday/faraday-core/src/core_wallet/tool.rs:740-744`).
- Open replaces the state.
- The secret held is the typed passphrase. It goes when the read starts, at lock, or on leaving.
- The parser keeps no extra copy of the encrypted key material (fix a01ab83e9).

## Reach

- Opened from the Tools tile and from Files.
- Take ends on the Wallets tab (see wallets).
- Code reach:
  - `faraday_core_wallet::*`, re-exported at `faraday/faraday-core/src/core_wallet/mod.rs:~5`;
  - `pass_field::{Gate, Kind, PassField, PassKey}`;
  - `register::name_of_file`;
  - `wallet::{Origin, Refusal, network_name}`;
  - `samples::PASSPHRASE`;
  - `worker::Job`;
  - `stick::*`;
  - `trail::StepFlow`;
  - `flow::Contained`;
  - `demo::PassField::CoreWallet` (`faraday/faraday-core/src/core_wallet/screen.rs:~203`);
  - `screens::parts::{ending, file_choices, nothing_to_work_on}`.

## Rules and refusals

The file and the passphrase
- The chosen file must still be in Files, else `core_wallet_none_in_files` (`faraday/faraday-core/src/core_wallet/tool.rs:556-559`).
- Focus on the passphrase, and Read, are refused while a stick blocks secrets (`~485-487`, `~577-579`).
- Typed keys are refused by the stick guard and by the `pass_field` gate, with the reason as a toast (`~706-720`).
- Keys are ignored while a sheet is open (`~699`).

Verdicts
- The verdicts are `WrongPassphrase` (`~349`, `~434`) and `NotWhole` (`~351`).
- Take works only when `is_whole()` (`~429`).
- The refusal text is the first failed line, or the not-whole line (`~429-439`, `~637-640`).

Descriptors
- Inactive descriptors are never taken. Only the active receive descriptor is taken, with its change of the same output type (`~325-344`, `~423-425`).
- Output type order: Legacy, P2SH-SegWit, Bech32, Bech32m (`~298-303`).
- `no_active` shows when the file is whole but has no active pair (`~447`).
- Failed records, and kept `combo()` descriptors, are each capped at 12, with a "more" line (`~51`, `~248-264`, `~413-422`).
`Refusal::Unconfirmed` on Take does nothing (faraday/faraday-core/src/core_wallet/tool.rs:~663-664). The hold then runs Take again only when no wallet sheet already waited and the Inbox's names and sizes are unchanged; else it adds the wallet itself and toasts (faraday/faraday-core/src/wallet.rs:657-673, 4539-4544, 4653-4705).

Cards
- The open step's own press does nothing (`~468-475`).
- The Result tone is OK only for Whole (`faraday/faraday-core/src/core_wallet/screen.rs:~234-236`).
- A read refusal reopens the File card (`faraday/faraday-core/src/core_wallet/tool.rs:~614`).

The parser (`faraday/faraday-core/src/core_wallet/mod.rs`, `faraday/faraday-sqlite`)
- It refuses what Bitcoin Core's Verify refuses (fix 283a2b8c2):
  - a database version other than 0;
  - unreachable pages;
  - an over-large `main` table;
  - a journal beside the file.
- Its checks and its memory are bounded by the file (fixes 283a2b8c2, a01ab83e9).
- A descriptor's creation time is read as unsigned 64-bit (fix d9bf89852).
- A range size or next index of 1,000,000 or more is refused, as Core's CheckDescriptorRangeBounds does. A range over one index on a descriptor with no `*` is refused as corrupted (fix df76fb29a).
- A sortedmulti wallet is one wallet whatever its key order (fix e45fa284d).
- A descriptor record with a private key in its text is not kept. `key_facts` reads only descriptors of public keys (`core/faraday-bip/src/descriptor.rs:216-225`), so such a record fails as `Fault::Descriptor`, keeping only its name and key hex (`faraday/faraday-core-wallet/src/lib.rs:1143-1147`, `656-667`). Normalising refuses an extended private key too (`faraday/faraday-core-wallet/src/normal.rs:215-218`). Descriptor texts are `Text`, wiped on drop.

## Domain calls

- `faraday-core-wallet`: `read_file`, `Wallet`, `Refusal`, `Fault`, `Passphrase`, `OutputType`, `is_wallet_file`.
- `faraday-sqlite`, the SQLite parser.
- `faraday-keys` `core_wallet` (`faraday/faraday-keys/src/core_wallet.rs`), for the encrypted keys.
- `faraday-bip` `keys::Network`.
- `faraday-crypto` `Compute`.
- `zeroize`.

## Fixtures

- `faraday/faraday-samples/data/core-wallet-plain.dat`
- `faraday/faraday-samples/data/core-wallet-encrypted.dat`
- `faraday/faraday-core/tests/all/core_wallet_pinned.txt`
- `tools/vectors/core-wallet/`, made by Bitcoin Core on regtest by `gen.py` (ten descriptor wallets: plain, encrypted with passphrase `faraday`, watch-only, blank, labels, imported, uncompressed, multisig, active multisig, migrated from a legacy wallet): `<name>.json` for each (application id, every `main` record as hex, listdescriptors, listdescriptors_public, getwalletinfo) and `<name>.dat` for plain, encrypted, multisig, activemultisig, watchonly and migrated. Read by `faraday/faraday-core/tests/all/core_wallet.rs`, `faraday/faraday-core/tests/all/core_wallet_pinned.rs`, `faraday/faraday-core/tests/all/core_wallet_tool.rs`, `faraday/faraday-core/tests/all/tool_opens.rs`, `faraday/faraday-core/tests/all/tool_stick.rs`, `faraday/faraday-core/tests/all/flow_hide_shown.rs`, `faraday/faraday-core/tests/all/stick_copy.rs`, `faraday/faraday-core/tests/leaks/readers.rs`, `faraday/faraday-keys/tests/core_wallet.rs` and the core-wallet tour; the two Practice sample files are copies of plain.dat and encrypted.dat (`faraday/faraday-samples/examples/regenerate.rs:625-626`).
- The parser's SQLite fixtures:
  - `faraday/faraday-sqlite/tests/fixtures/deleted.db`, `empty.db`, `freelist.db`, `other_table.db`, `overflow.db`;
  - `ps512.db`, `ps1024.db`, `ps2048.db`, `ps4096.db`, `ps8192.db`, `ps16384.db`, `ps32768.db`, `ps65536.db`;
  - `small.db`, `statement_column.db`, `statement_type.db`, `text_value.db`, `three_columns.db`, `twin_tables.db`, `wal.db`;
  - `faraday/faraday-sqlite/tests/make_fixtures.py`.
- The Practice strings `EN.sample_core_wallet_plain_file` and `EN.sample_core_wallet_encrypted_file` (`faraday/faraday-core/src/core_wallet/tool.rs:~776-777`) name the samples.
- `docs/decisions/F229.md` is the tool's decision.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 283a2b8c2 | The wallet.dat parser accepted a database version other than 0, unreachable pages, an over-large `main` table and a journal beside the file; its checks could cost more than the file. | Refuses what Bitcoin Core's Verify refuses; checks bounded by the file. |
| a01ab83e9 | The parser's memory bound did not hold for a record that is all header, and a fourth copy of the encrypted keys was kept. | Memory bounded by the file; no extra kept copy of key material. |
| d9bf89852 | A descriptor whose creation time has the top bit set was listed as a negative time. | The creation time is read as unsigned 64-bit, as Core writes it. |
| df76fb29a | A Core wallet file with a range or next index of 1,000,000 or more was accepted and listed. | Refused, as Core's CheckDescriptorRangeBounds; a range over one index on a descriptor with no `*` is refused as corrupted. |
| e45fa284d | The same sortedmulti wallet with its keys in another order (Caravan JSON against a Coldcard file) counted as a new wallet; a saved backup plan did not open for the other key order. | A sortedmulti wallet is one wallet whatever its key order; a plan record opens for either order. |

## Doubts

- e45fa284d was not read beyond its commit message. Its main surface is the session's dedupe (see wallets).
