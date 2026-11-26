# Decoding and checking transactions

Four tools that look at a transaction brought in from the online side
without signing it: Decode a transaction (`Screen::Decode`), the Nonce check
of a signed PSBT or finished transaction (`Screen::NonceCheck`), Check a
signer, which compares what was sent to a signer with what came back
(`Screen::SignerCheck`), and the comparison of other transactions spending
the same inputs (the fee-bump judgement shown in Sign's review). They are for
the person checking a hardware wallet or a cosigner's work. They are reached
from the Tools tiles and the Check page's Signing group, from a file in Files
or the Inbox, from a scan, and from the Sign a transaction page (Decode the
finished transaction, Check a signer from Review, the nonce card). All code
is in `faraday/faraday-core/src/decode/` with pages in
`faraday/faraday-core/src/screens/`.

## Screens and sheets

Decode a transaction:
- `Screen::Decode`: a page (`faraday/faraday-core/src/decode/screen.rs:~45-84`)
  and, on a non-compact panel, the same content as the flow's one card
  (`flow::DECODE_STEPS`, steps PICK = 0 and RESULT = 1,
  `faraday/faraday-core/src/decode/flow.rs:~22-24`;
  `faraday/faraday-core/src/decode/screen.rs:~370-440`).
- Landing: lists held transaction files or says none waits; offers Scan,
  Type it, Open Files (`faraday/faraday-core/src/decode/screen.rs:~544-582`).
  The typed entry reuses `invoice::screen::field`
  (`faraday/faraday-core/src/decode/screen.rs:~586-622`).
- Result: txid, wtxid, version, locktime, size (bytes, vbytes, weight), fee
  and fee rate, total in, inputs (outpoint, sequence, witness, scriptSig),
  outputs (amount, address, kind, whose), hex with show/hide, QR of the hex.
  The table is at most 960 wide (`faraday/faraday-core/src/decode/screen.rs:~17`).
  Whose-text tone is Ok for change or receive, Warning for a payment away
  (`~486-498`). "Load a wallet" shows only when none is loaded
  (`~309-315`).

Nonce check:
- `Screen::NonceCheck`, page `faraday/faraday-core/src/screens/nonce_check.rs`;
  flow card `decode::nonce_flow::NONCE_STEPS` / `contained_check`
  (`faraday/faraday-core/src/decode/nonce_flow.rs:72-98`; used by
  `faraday/faraday-core/src/trail.rs:868-869` and
  `faraday/faraday-core/src/motion.rs:1356`), only on a wide, non-compact
  page with a check or waiting.
- Pane (`faraday/faraday-core/src/decode/nonce_pane.rs`): groups "file" and
  "inputs": an All inputs row, up to 12 input rows (`SHOWN`,
  `faraday/faraday-core/src/decode/nonce_pane.rs:~24`), one summary row.
  With nothing to check it shows the ways in: Files to check, Scan, Open
  Files (`~93-105`).
- The nonce card itself is `faraday/faraday-core/src/screens/spend.rs`
  `nonce_card` (see `spend-signing`).
- Page body: nothing is drawn without an outside check
  (`faraday/faraday-core/src/screens/nonce_check.rs:~29-31`); only the CHECK
  step draws (`~82-84`). "Why not finished" ladder
  (`faraday/faraday-core/src/screens/nonce_check.rs:128-152`):
  only witness wrong and only fields refused gives `refused_fields`; only
  witness wrong gives `refused_witness`; disagree first gives
  `refused_disagree`; used before and none reused gives `refused_used`;
  `wrong()` gives `refused`; the refused flag gives `refused_unread`; else
  `more_wanted` (neutral).
- Buttons (`~155-207`): Save to stick (`CheckPress::Out`) or Remove
  (`OutboxRemove(i)`) when already in the Outbox; Decode
  (`CheckPress::Decode`) and raw hex (`CheckPress::Hex`) only when finished;
  Show as QR (`CheckPress::Qr`); Scan another (`Action::Scan`); Done
  (`Nav(o.back)`). Save, Remove and Show as QR are hidden when refused.

Check a signer:
- `Screen::SignerCheck`; cards Sent (0), Returned (1), Result (`RESULT`).
  Sheets `Sheet::Scan` (`faraday/faraday-core/src/decode/mod.rs:~1417`) and
  `Sheet::SignerSeed` (`~1463`). A seed dialog drawn by `compact::dialog`
  with ways in `Opening::CheckSigner` (Type words, Scan SeedQR, codex32,
  Open a vault, Other forms, Cancel)
  (`faraday/faraday-core/src/decode/check_screen.rs:~273-312`).
- The Use-the-stick button only with a stick attached and no key open
  (`faraday/faraday-core/src/decode/check_screen.rs:~112-124`); a sample
  PSBT only where `sample_psbt_offered()` (Demo, `~79-81`).
- Result: verdict card, findings with level colour (Blocked and Danger as
  Error, Caution and Info as Warning,
  `faraday/faraday-core/src/decode/check_screen.rs:~390-393`), stated lines,
  differences, the fee-bump judgement, nonces recomputed, keys not loaded to
  Add (`Load(n)`), Show as QR (only when the Returned side is a PSBT with a
  version, `faraday/faraday-core/src/decode/check.rs:~492-493`,
  `faraday/faraday-core/src/decode/mod.rs:~1433`), Swap, Choose again, Done.
- A closed card's line wraps (it asks `Column::wrap_closed`).

Compare (other transactions):
- `faraday/faraday-core/src/decode/check_others.rs` lists files spending an
  input of the open PSBT with their fee-bump result in words, shown in
  Sign's review.
- Words: `faraday/faraday-core/src/decode/diff_words.rs`,
  `faraday/faraday-core/src/decode/check_words.rs`.

## Actions

Decode:
- `Action::DecodeTyped(TypedPress::{Open, Next, Clear})`,
  `TypeHere(Here::DecodeText)`, paste (`decode_pasted`,
  `faraday/faraday-core/src/decode/mod.rs:~1305`, `~1335`),
  `DecodeInbox(k)`, `DecodeAnother` (`~1169`), `DecodeHex` / `toggle_hex`
  (`~1205`), `QrDecoded` (`~1178`), `ScanFor(Ask::Decode)`,
  `Nav(Screen::Files)`, `RestoreFor(restore::Check::Decode)` (only when no
  wallet is loaded).
- Way back `decode_back_to`: `NonceCheck(CheckPress::Back)` or `Nav(back)`
  (`faraday/faraday-core/src/decode/screen.rs:~38-43`).
- Entry points: the Tools tile (`open_tile`,
  `faraday/faraday-core/src/decode/mod.rs:~1975-1991`: exactly one Inbox
  transaction opens it directly, else the landing), a file (`decode_file`,
  `~1104`), Inbox (`DecodeInbox`), Spend (`DecodeFinished`, `~1147`), the
  nonce check (`CheckPress::Decode`, `~1078`).

Nonce check:
- Opened by `nonce_check_open` (`faraday/faraday-core/src/decode/mod.rs:~882`)
  from Inbox file k; from the tile (`nonce_check_tile`, `~1355`: the first
  Inbox file that is checked-not-signed, else the landing on a wide window,
  Scan on a compact panel, `~1358`); from a scan (`scan_landed`, `~842`).
- Presses `CheckPress::{Inbox, Back, Input(Option), Hex, Qr, Out, Decode}`
  (`faraday/faraday-core/src/decode/mod.rs:~1030-1090`).
- From the Tools catalog, Nonce check with a finished spend goes to the
  spend's Finish step instead.

Check a signer:
- `SignerPress` (`faraday/faraday-core/src/decode/check.rs:~48-91`): `Open,
  Card, Pick(slot, k), Scan(slot), Swap, Again, Load(n), Seed(n), SeedWords,
  SeedScan, SeedForm, SeedVault, File, FromReview, Qr, Back`.
- Entry points `signer_check_open`
  (`faraday/faraday-core/src/decode/mod.rs:~1517`), `signer_check_file`
  (`~1549`), `signer_check_from_review` (`~1596`), `signer_check_scanned`
  (`~1935`).

## Effects

- Clipboard request `Command::RequestClipboard { kind: FileKind::Text }`
  (`faraday/faraday-core/src/decode/mod.rs:~1339-1344`).
- QR via `open_qr(QrView::text/psbt)` (`~1062`, `~1202`, `~1436`).
- Camera on for Scan (`Command::CameraOn`).
- Costly address derivation per output per loaded wallet up to index 100
  (`find_address`, `faraday/faraday-core/src/decode/mod.rs:~180`).
- Nonce history read and written (`nonce_snapshot`, `memory.nonces.note`,
  `faraday/faraday-core/src/decode/mod.rs:~916-917`, `~1003`, `~1024`).
- Save to a stick from the nonce check adds the file to the Outbox beside a
  same-named file (`put_outbox_beside`, `~1071`; `toast_out`, `~1075`).
  The file is `<stem>-final.txn` for a PSBT, else the file as it came
  (`~937-943`).
- `nonce_spend_sync` (`faraday/faraday-core/src/wallet.rs:~5124-5199`)
  removes Outbox files of spends taken back and calls `save_boxes()`
  (`~5183-5187`).
- Drawing writes scroll state (`faraday/faraday-core/src/decode/screen.rs:~77`,
  `~82`, `~439`; `faraday/faraday-core/src/decode/check_screen.rs:~60`).

## State and lock

- `DecodeArea` (`faraday/faraday-core/src/decode/mod.rs:~403`,
  `Faraday.decode`):
  - `state: Option<DecodeState>`: the decoded transaction (public);
  - `landing`, `nonce_landing`, `scroll`;
  - `outside: Option<Box<OutsideCheck>>`: the nonce check (`check`,
    `finished: Option<Transaction>`, `hex`, `out_name`,
    `out: (Text, Zeroizing<Vec<u8>>)`, `worked`, `made_with`, `psbt`,
    `prevouts`, `rows`, `input`, `scroll`); the `out` bytes are
    `Zeroizing`; `finished`, `psbt` and hex are not wiping types;
  - `check: Option<Box<check::CheckState>>`: Check a signer;
  - `typed: Option<Typed>`: `Typed.text` is a `Text` ("may be a mistyped
    secret"), which wipes on drop and on zeroize
    (`core/faraday-text/src/text.rs:363-371`).
- `CheckState` (`faraday/faraday-core/src/decode/check.rs:~131-165`):
  `sent/returned: Option<Side>` (`bytes: Zeroizing<Vec<u8>>`, `psbt`, name,
  origin, differs, txid, version, outpoints), `report`, `shown`,
  `made_with`, `ids`, `open`, `refused: [Option<Text>; 2]`, `scroll`,
  `back`, `seed: Option<Fingerprint>`. Its module doc says public data.
  `hide_shown` is a no-op (`faraday/faraday-core/src/decode/check.rs:723-726`).
- `NonceCheck` (`faraday/faraday-core/src/wallet/nonce.rs`): `reused`,
  `used_before`, `mismatched`, `fields_refused`, `unread`, `disagree`,
  `not_evaluated`; made by `Session::nonce_check`
  (`faraday/faraday-core/src/wallet.rs:1822`), cached in
  `SpendState.nonce_check`. Nonce history is in `memory::nonces`
  (`MemoryArea` is `Public`, `faraday/faraday-core/src/memory/mod.rs:~156`).
  No secret.
- Lock (`AtLock for DecodeArea`,
  `faraday/faraday-core/src/decode/mod.rs:1994-2003`): `holds` is NOTHING;
  `lock` is `clear_check()` (all of Check a signer), `drop_typed()`,
  `drop_nonce_input()`, `nonce_landing = None`. Not dropped by this hook:
  `outside` (the nonce check with its PSBT, finished transaction, hex and
  Save-to-stick bytes) and `state`. A lock then restarts the process
  (`faraday/faraday-core/src/vault/seal.rs:933-946`), so neither outlives it
  in memory.
- Leaving: `open_nonce_landing` drops `outside`
  (`faraday/faraday-core/src/decode/mod.rs:~669`); `show` clears `typed`
  (`~590`); `DecodeAnother` and `TypedPress::Open` clear `state`. The
  decode flow's `hide_shown` is a no-op: the hex is public
  (`faraday/faraday-core/src/decode/flow.rs:~77-79`).
- A nonce hold on a spend stays until the spend is closed (F227); closing a
  vault, turning the history off or opening another vault does not release
  it. A key found to have reused a nonce stays in the check for the spend's
  life (`faraday/faraday-core/src/wallet.rs:~5133-5136`).

## Reach

- Out: `wallet::{NonceRow, NonceCheck, Session, InspectInputs, FileKind,
  read_psbt, read_psbt_kept, classify}`, `memory::nonces`, `pick`, `flow`,
  `trail`, `review::{name_scripts, sighash_text, unchecked_text}`,
  `vault::{SlotOf, VaultAction}`, `qr_view::ScanState`, `ScanPurpose`,
  `at_lock`, `edit::Here::DecodeText`, `screens::{row_verdict, decode_back,
  parts}`, `invoice::screen::field`, `restore::Check::Decode`.
- In: `faraday/faraday-core/src/screens/nonce_check.rs`,
  `faraday/faraday-core/src/trail.rs:868`,
  `faraday/faraday-core/src/motion.rs:1356`,
  `faraday/faraday-core/src/screens/spend.rs:5` (uses `Said`).
- Opens: Files, the scanner, Add a key ways in (Check a signer seed dialog),
  vault unlock, Restore (Decode with no wallet).
- Opened from: `tools`, `check-page` (Transaction, Signed, Signatures rows),
  `spend-signing`, Files and Inbox.

## Rules and refusals

Decode:
- Outputs totalling over max money are refused
  (`decode_outputs_over_max_money`,
  `faraday/faraday-core/src/decode/mod.rs:203-204`); stated inputs over max
  money likewise (`faraday/faraday-core/src/decode/mod.rs:207-208`). Decode
  refuses with a toast; it does not panic on an overflowing sum.
- The fee is shown only when every input amount is stated and in >= out
  (`faraday/faraday-core/src/decode/mod.rs:~209`, `~259`).
- Amounts come only from a PSBT whose unsigned transaction is exactly this
  one (`same_unsigned`, `~118-128`), which is named; disagreeing PSBTs give
  `AmountsFrom::Disagree` and no amounts (`~796-800`).
- Non-address outputs show as script hex (`~243`). `raw_as_psbt` returns
  None for non-transaction bytes (`~724-748`).
- A raw transaction must have at least one input (a segwit marker and flag
  with no inputs is refused, as Core reads it)
  (`core/faraday-psbt/src/transaction.rs:~38-45`).
- Type it: at most `TYPED_MAX` = 1 MiB, typing past it ignored
  (`~440`, `~1281`); an empty paste `pasted_nothing`; a paste over the cap
  `decode_typed_too_large` with the text kept (`~1313-1316`); reading
  refuses anything `wallet::classify` does not call a PSBT or Transaction,
  with `compare_not_a_transaction` (`~1251-1255`); typing only with no
  result, no sheet and the field open (`~1268`).
- Hex and QR are not offered when the nonce check refuses
  (`check.refused(hex.is_some())`) or when Decode was opened from a spend
  under a nonce hold (`faraday/faraday-core/src/decode/screen.rs:~289-293`);
  the QR is refused silently (`faraday/faraday-core/src/decode/mod.rs:~1197-1200`,
  `~829-834`).
- Only Transaction files are offered
  (`faraday/faraday-core/src/decode/screen.rs:~344`, `~402`, `~549`).

Nonce check:
- Opens only for a PSBT that reads or a raw transaction that reads and
  wraps (`faraday/faraday-core/src/decode/mod.rs:~893-913`).
- A wrong signature, or a nonce used before (history check), refuses the
  finished transaction (`~918-924`, `~949-953`); an unread or falsely stated
  output gives no finished transaction (`~927-931`).
- The check is re-made when keys or wallets change; a found refusal stays
  while the page holds the file (`~988-995`).
- Qr, Out and QrDecoded are refused when `check.refused(...)` (`~1054`,
  `~1068`, `~1188-1195`); Decode needs a finished transaction
  (`~1083-1085`); the input choice is only among `rows`
  (`faraday/faraday-core/src/decode/mod.rs:1036`).
- Warnings order: mismatched, reused, used before; a warning wins over a
  verdict (`faraday/faraday-core/src/decode/nonce_pane.rs:~29-48`). Row
  rank: wrong, not checked, any (`faraday/faraday-core/src/decode/nonce_flow.rs:~34-42`).
- An unread, unrun or unchecked input reads "not known", never "none
  wrong", also for rows past the twelfth and in "N more".
- A PSBT of the open spend stays with the spend; otherwise toast
  `scanned_is_in_files` and go to Files unless on Spend, Restore, Home or
  SpendBackup (`faraday/faraday-core/src/decode/mod.rs:~843-858`).
- `checked_not_signed` (`~865-876`): a Transaction always; a PSBT only with
  a signature and no key here left to sign.
- `held_back()` = used before or reused non-empty: nothing finished, saved
  or shown as a code, no way past
  (`faraday/faraday-core/src/wallet/nonce.rs:111-113`).
- `sound()` needs at least one row, no unread, reuse, used-before,
  mismatch or not-evaluated, and every row valid and checked
  (`faraday/faraday-core/src/wallet/nonce.rs:161-170`). An unchecked
  signature shows amber "signatures not checked"; a wrong one red.
- Headline order: witness wrong, disagree, wrong, unread, script not run,
  no signatures, unchecked, valid
  (`faraday/faraday-core/src/wallet/nonce.rs:~243-281`).
  `Unmatched::{NoOrigin, Finalized, NotRun}`.
- A nonce entry is kept in history only when its key is proved (derived at
  the stated place by a registered wallet key or a loaded master)
  (`faraday/faraday-core/src/wallet.rs:~1857-1874`), so a file cannot fill
  history with its own keys. `used_before` (F227) looks at the nonce
  history of open vaults (`against`,
  `faraday/faraday-core/src/wallet/nonce.rs:~72-108`).
- `nonce_spend_sync`: a spend that cannot be finished, was not held and has
  no partial or final returns without a check
  (`faraday/faraday-core/src/wallet.rs:~5143-5151`); held-back spends go
  through `apply_nonce_hold` (`~5184`).
- `nonce_partials` (`faraday/faraday-core/src/wallet.rs:~1921-1948`) lists
  each verified MuSig2 partial by input and the first 8 hex of the
  participant key.

Check a signer:
- Each side must read as a PSBT (`read_psbt_kept`) or a raw transaction
  (`raw_as_psbt`), else `compare_not_a_transaction` on that card
  (`faraday/faraday-core/src/decode/mod.rs:~1709-1714`).
- Card n opens only when n < RESULT or the result is ready
  (`~1395-1398`; `faraday/faraday-core/src/decode/check.rs:~238`); a pick
  index must exist in `check::files` (`~1404`); Result only with both sides
  (`~1763-1765`); a signer report only when both are PSBTs (`~1814`); the
  seed row loads only if `may_load_keys()` (`~1459`); a scan that is not a
  PSBT or transaction is refused on the scanner (`~1945-1950`); a stale
  result is rechecked when keys or wallets change (`~1908-1928`).
- `put` drops the report and result, and clears the Returned side only when
  its origin is the same File origin (scanned never cleared)
  (`faraday/faraday-core/src/decode/check.rs:197-210`); `still_fits` is
  `later < RESULT` (`faraday/faraday-core/src/decode/check.rs:734-736`).
- `rows` hides a file that is the other side's copy unless it differs
  (`faraday/faraday-core/src/decode/check.rs:~318-326`); `default_pair`:
  exactly two PSBTs sharing an unsigned transaction with different
  signature counts (`~343-369`). Raw bytes: the `psbt\xff` magic is raw; a
  text file that is not base64 falls back to raw (`~377-398`).
- Verdict tones: Clean OK, Findings WARN (ERR at worst Danger),
  NotSameTransaction ERR (`~410-439`); fee bump OnlyFee OK, MoreChanged
  ERR, CannotTell WARN (`~444-450`); `check_facts` only for one transaction
  (`~611-613`); `has_findings` false for two different transactions
  (`~635`); finished transactions give no verdict (`~478-479`); "seeds" are
  signing keys not loaded, "load" are fee-bump keys not loaded
  (`~598-609`).
- Sent choice: the other file of the same transaction with fewer
  signatures, else the one other unsigned-transaction file spending an
  input of it (`faraday/faraday-core/src/decode/mod.rs:~1559-1584`); from
  review only for a PSBT or Transaction file that is the one the result was
  made from (`~1602-1628`).
- A fee is shown only from an amount the PSBT's previous transaction agrees
  with.

Compare (`faraday/faraday-core/src/decode/check_others.rs`):
- counts only PSBT or finished-transaction files (`~53-54`) sharing an
  outpoint (`~79-89`) whose unsigned id differs from the open one
  (`~58-63`), one per transaction id with the PSBT preferred (`~64-72`);
  unreadable or resultless files are skipped (`~124-141`); finished
  transactions need `amounts_in` (`~175-179`).
- Words: an output with no address is named by script length
  (`faraday/faraday-core/src/decode/diff_words.rs:~51-62`); an unknown fee
  reads "Not known" (`~305-307`, `~375-378`); unknown or proprietary PSBT
  pairs show type, subtype and sizes, never content
  (`faraday/faraday-core/src/decode/check_words.rs:~128-185`); a signature
  not checked shows the review's reason (`~109-115`); outputs are named by
  whole address else script length (`~41-46`).

## Domain calls

- `faraday_psbt::signer::{check, nonce_view, with_nonces, Report, Verdict,
  Finding, Kind}` (`core/faraday-psbt/src/signer.rs`).
- `faraday_psbt::bump::{judge, fill_amounts, Bump, Line, Outcome}`
  (`core/faraday-psbt/src/bump.rs`); `faraday_psbt::diff::{list, Side,
  Differences}` (`core/faraday-psbt/src/diff.rs`).
- `faraday_psbt::verify::{spent_outputs, Field, Where, verify_signatures,
  repeated_nonces, stated_place, witness_mismatch, witness_shape_mismatch,
  not_evaluated, unread_counts}` (`core/faraday-psbt/src/verify.rs`);
  `faraday_psbt::musig::verified_rounds`.
- `faraday_psbt::transaction::{read_raw, wrap_raw, money_total}`
  (`core/faraday-psbt/src/transaction.rs`); `Psbt::{parse_bytes,
  finalize}`; `inspect`.
- `faraday_bip`: `consensus::encode::serialize_hex`, `Address::from_script`,
  `Amount`, `Transaction::{compute_txid, vsize, weight}`, `Script::is_*`,
  `base64`.
- `faraday_crypto::{work::Compute, hex::encode_string}`;
  `faraday_shell_api::{Key, Command::{RequestClipboard, CameraOn}}`;
  `faraday_text`; `faraday_ui::{short, grouped}`; `zeroize`.
- `keys.recompute_signatures` (`faraday/faraday-keys`).

## Fixtures

- `faraday/faraday-samples/data/signer-sent.psbt`
- `faraday/faraday-samples/data/signer-returned.psbt`
- `faraday/faraday-samples/data/signer-returned-proprietary.psbt`
- `faraday/faraday-samples/data/signer-elsewhere-sent.psbt`
- `faraday/faraday-samples/data/signer-elsewhere-returned.psbt`
- `faraday/faraday-samples/data/bump-original.psbt`
- `faraday/faraday-samples/data/bump-replacement.psbt`
- `faraday/faraday-core/tests/vectors/interop/bluewallet/quicklook-preview-sample.txn`
  (read as a Transaction file by `classify`,
  `faraday/faraday-core/tests/all/interop.rs:~56-64`)

(The samples are the Demo's Check a signer and Compare pairs,
`faraday/faraday-samples/src/data.rs:687-693`.)

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 091ca12fe | A signature that could not be checked still let the Nonce check headline read valid | Sound only when every signature was checked; unchecked amber, wrong red |
| 50cce374a | Decode panicked on outputs or inputs adding past u64; a PSBT over 21,000,000 BTC was not refused | Amounts and sums checked as Core does; Decode toasts, the signer blocks with `AmountOutOfRange` |
| 4def5599c | A raw transaction with a segwit marker and no inputs was accepted and written back | A transaction read from bytes must have at least one input |
| a7742cd3b | Decode took input amounts from a PSBT of another transaction; Collect accepted a cosigner's signature under another sighash | Amounts only from a PSBT of exactly this transaction; a sighash mismatch is refused |
| 432aaae12 | A spend held for nonce reuse was released by closing the vault, turning history off or opening another vault | A nonce hold stays until the spend is closed (F227) |
| c287628be | The row for an input past the twelfth and "N more" read an unread or unchecked input as fine | Unread, unrun or unchecked reads "not known" |
| c3ab6eb33 | Compare and Check showed fees from an amount the previous transaction contradicts | Fee shown only from an agreeing amount |
| a7fef6f08 | Check a signer's closed card cut its line | A closed card's line wraps where the flow asks |

## Doubts

- c287628be and c3ab6eb33 were given to `spend-signing`; a7fef6f08 to
  `check-page`. They are placed here as they concern these screens.
- `outside` and `state` survive `DecodeArea::lock`; PSBTs can carry key
  origins and xpubs. The process restart at lock clears memory; whether any
  other lock path clears them earlier was not found.
- The doc at `faraday/faraday-core/src/wallet.rs:~1851-1852` says only a
  valid signature is looked at for nonce history, but the filter at
  `~1857` checks only fingerprint and place; whether
  `NonceEntry::of_signature` checks validity is unclear.
- `decode_act` has `unreachable!()` for unmatched actions
  (`faraday/faraday-core/src/decode/mod.rs:~1212`). `"replace-by-fee"` and
  `"0x…"` are hard-coded English (`~160-162`).
- Stale comments: `faraday/faraday-core/src/decode/mod.rs:~748-754`,
  `faraday/faraday-core/src/decode/screen.rs:~19-22`,
  `faraday/faraday-core/src/decode/check.rs:~371-372`,
  `faraday/faraday-core/src/decode/check_words.rs:~1-4` (says exhaustive;
  `pair()` ends in `_ => None`).
- `faraday/faraday-core/src/screens/nonce_check.rs:~103` `under` closure is
  a no-op; the comment at `~27-28` says a small panel opens the scanner but
  the code only returns.
- The dispatcher `act_inner` for decode was not read.
- Check a signer's `put` behaviour for a scanned Returned side (never
  cleared) is as stated; whether that is intended was not asked.
