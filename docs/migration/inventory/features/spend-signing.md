# Signing a transaction

Signing a PSBT on the offline device: opening a PSBT as a spend, the
step-by-step Sign a transaction page (wallet, check, review, signers, sign,
collect, finish, paths, nonces), collecting other devices' signatures,
finishing the transaction, MuSig2 rounds and registering a multisig wallet
from the PSBT itself. It is for the person holding one or more of the
wallet's keys. It is reached from the Tools tile Sign (`Go::Sign`), from a
PSBT file in Files or the Inbox, from a scan, from a wallet's Spend
(`Action::StartSpend`), and from the Spend tab (see `spend-backup`), which
runs the same spend through its own step cards.

## Screens and sheets

- `Screen::Spend`, the Sign a transaction page
  (`faraday/faraday-core/src/screens/spend.rs`). Its steps are `STEPS`
  (`faraday/faraday-core/src/screens/spend.rs:212`): 0 Wallet, 1 Check,
  2 Transaction (the review), 3 Transaction id, 4 Signers, 5 Sign,
  6 Signatures (collect), 7 Finish, 8 Path, 9 Nonces. Step 3 has no body of
  its own (`step_body`, `faraday/faraday-core/src/screens/spend.rs:763-805`,
  arm `_ => y`).
- Review (Transaction step): what is sent, change, fee, inputs, outputs,
  locktime, RBF, txid; warnings; a table (`ToggleTable`); other
  transactions spending the same inputs with a fee-bump comparison
  (`SignerPress::FromReview` opens Check a signer, see `decode`); the
  stop or hold block for a holdable danger (`ReviewHold`, F136, a danger
  the person holds past).
  - Recipients are outputs that are not verified change
    (`faraday/faraday-core/src/screens/spend.rs:~669`). Change rows read
    "verified" and name the wallet, also another wallet of the person's
    (F254) (`faraday/faraday-core/src/screens/spend.rs:~980-1011`).
  - A fee not known reads "not known", never zero
    (`faraday/faraday-core/src/screens/spend.rs:~1019-1026`). "Amount
    unchecked" shows when any input lacks an amount
    (`faraday/faraday-core/src/screens/spend.rs:~1029`, `~1042-1043`).
  - The locktime row shows only for a block height
    (`faraday/faraday-core/src/screens/spend.rs:~1048-1050`).
  - The txid row is shown as known before signing only where signing does
    not change it (`unchanged_by_signing`,
    `faraday/faraday-core/src/screens/spend.rs:2744-2770`): legacy P2PKH and
    P2SH-wrapped multisig or miniscript change the txid when signed;
    segwit, taproot and unknown count as unchanged.
  - The key-path danger line is repeated first (F132)
    (`faraday/faraday-core/src/screens/spend.rs:~1297-1307`).
  - A wallet-not-registered warning shows a Register button when the PSBT
    carries a descriptor, else text to load the wallet
    (`faraday/faraday-core/src/screens/spend.rs:~1323-1348`).
  - Fee-bump tone: only the fee changed is ok, more changed is an error,
    cannot tell is a warning
    (`faraday/faraday-core/src/screens/spend.rs:~1236-1240`).
  - `review_stop_block`: a stop that is not a hold shows its text only; a
    hold-first stop shows fee, rate, share and the hold button. Which one is
    decided by string equality with `EN.step_review_hold_first`.
  - The first recipient's address is shown whole, in groups of four, for
    scripts with an address form (p2pkh, p2sh, p2wpkh, p2wsh, witness
    programs other than v0), else shortened
    (`faraday/faraday-core/src/screens/spend.rs:~2421-2433`). Change is
    named only when verified
    (`faraday/faraday-core/src/screens/spend.rs:~2446-2455`).
- Signers (keys) step: a label equal to the fingerprint is not drawn twice;
  a seed held by an open vault loads from it; a vault key with a
  passphrase needs it (F62); "not here" and "signs here" lines; when short
  of signers it offers Scan a key, Type words, unlock vaults and Collect
  (`faraday/faraday-core/src/screens/spend.rs:~1520-1665`).
- Sign step: an "already in file" line when a partial signature is present,
  and the key is not signed again
  (`faraday/faraday-core/src/screens/spend.rs:~1709-1714`).
- Signatures (collect) step: per key, signed here / signed / not needed /
  does not verify / in file unconfirmed / waiting
  (`faraday/faraday-core/src/screens/spend.rs:~1806-1820`). Cosigner copies
  are listed only from Inbox items with the same txid that are not the
  source (`faraday/faraday-core/src/screens/spend.rs:~1826-1834`). Show as
  QR is withheld while a nonce hold stands
  (`faraday/faraday-core/src/screens/spend.rs:~1879-1881`, `~2211-2212`).
- Finish step: the cards of the signed PSBT and the finished transaction,
  each with Make again / Remove / Show as QR (`QrPart`, `QrSigned`, `QrTx`),
  `SignedToOutbox`, `TxToOutbox`, `DecodeFinished` (opens `decode`),
  `SpendHex`. It needs a line or a finish reason; a nonce hold has the error
  tone; Remove shows while the file waits for the stick, else Make again
  (`faraday/faraday-core/src/screens/spend.rs:~1992-1996`). The two cards
  sit side by side only where their titles and buttons fit.
- Nonces step: "go first and share" when neither the others' nonces nor
  this device's are in (`faraday/faraday-core/src/screens/spend.rs:~2221-2233`).
- Nonce card (also on the Nonce check page, see `decode`):
  `nonce_card` (`faraday/faraday-core/src/screens/spend.rs:~46`; drawn on
  the Send-it card at `faraday/faraday-core/src/screens/spend.rs:33`).
  Headline green when sound, red when wrong, else amber
  (`faraday/faraday-core/src/screens/spend.rs:~56-63`); at most 12 rows
  then a count (`SHOWN`, `faraday/faraday-core/src/screens/spend.rs:~94`,
  `~136-144`); history lines (`~166-173`); `row_verdict` per verdict
  (`~183-210`).
- Foot bar (`spend_bar`, `faraday/faraday-core/src/screens/spend.rs:~603`)
  on the small panel; a signatures box (`~2341`) on a wide window; a "this
  transaction" panel (`review_under`, `~2406`).
- End card (`faraday/faraday-core/src/screens/spend.rs:~347`): Sign another
  (`Nav(Files)`), Lock, Power off. Single-key routes when the key is not
  loaded (`single_key_routes`, `~415`).
- Main button count: `needed` when complete, else `min(signed, needed)`
  (`faraday/faraday-core/src/screens/spend.rs:~487-491`). A partial in the
  file the device cannot confirm is not counted (`~497`). With two or more
  keys whose names do not fit, the button counts the keys. Label "review
  first" while a review is owed (`~524-525`); `waiting_for_randomness`
  while it waits for randomness (`~561-566`).
- `Sheet::Register` (register a wallet from the PSBT,
  `faraday/faraday-core/src/register.rs`): for a multisig spend whose PSBT
  carries all cosigners' keys. A name field, a reading chooser (sorted or in
  order) only when there is more than one reading, Hold to register, Cancel.
  It draws nothing without a spend, an offer and the sheet. It shows the
  refusal text per reason (descriptor, input i, output i, not yours, not
  your key i).
- `Sheet::WriteOut` (asked before writing when the stick is not clean);
  `Screen::Files` for the files.

## Actions

- `Faraday::wallet_act` (`faraday/faraday-core/src/wallet.rs:~5275-5457`):
  - `StartSpend(i)` runs `signing::read::start_spend`
    (`faraday/faraday-core/src/wallet.rs:~5321`).
  - `Step(n)`; `StepNext(n)` also calls `spend_files_out`.
  - `ReviewScroll`; `ReviewHold` (passes Review after the inputs are
    reviewed, holds the holdable dangers); `ToggleTable`.
  - `SignHere` runs `signing::sign_here`.
  - `Collect(i)` runs `signing::collect::start_collect`
    (`faraday/faraday-core/src/wallet.rs:5373`).
  - `SignedToOutbox` and `TxToOutbox` run `put_outbox_beside`
    (`faraday/faraday-core/src/wallet.rs:~5379`, `~5385`).
  - `WriteAsk`: opens `Sheet::WriteOut` if the stick is not clean, a toast
    when there is no stick, else `Screen::Files`.
  - Also `PickWallet`, `PickKey`, `RemoveWallet` (no-op out of range),
    `RemoveKeys` (a sheet when the keys are kept nowhere, else removes) and
    `RemoveKeysHold`, `Primary`, `OpenWallet`, `Rename`, `LoadWallet`,
    `LoadKey`, `PublicOut/PublicMore/PublicRemove`, `WalletKeyOut`,
    `WalletKeyBsms`. The `_ => unreachable!()` arm
    (`faraday/faraday-core/src/wallet.rs:~5452-5454`) relies on `act_inner`
    routing.
- Register sheet (`register_act`,
  `faraday/faraday-core/src/wallet.rs:~4759-4837`): `RegisterOpen`,
  `RegisterField` (typing the name; Enter ends typing), `RegisterReading`,
  `RegisterCancel`, `RegisterHold`.
- Every route to signing (the Sign card, the side panel, the main button,
  Go first on the Nonces card) requires the steps before it done; if one is
  not, the first undone step opens and nothing is signed.
- Rename a wallet: `rename_type`; Rename again keeps the typed text.

## Effects

- Worker jobs (`faraday/faraday-core/src/signing/`), applied on the main
  thread at `faraday/faraday-core/src/compute.rs:287-288`:
  - `Job::SpendRead` (`faraday/faraday-core/src/signing/read.rs:~135`):
    `wallet::read_psbt_kept`, follows the network on a session copy,
    `is_silent_payment_psbt`.
  - `Job::Collect` (`faraday/faraday-core/src/signing/collect.rs:~165`):
    merges the file into the spend, refreshes, plans the part file.
  - `Job::Sign` (`faraday/faraday-core/src/signing.rs:~279-282`).
  - `faraday/faraday-core/src/signing/fresh.rs` `worked_out`: inspection,
    nonce check, MuSig2 aggregation; it holds no key.
- Outbox files: the signed PSBT `<stem>-signed.psbt`, the finished
  transaction `<stem>-final.txn`, and a part-signed file for other signers.
  They are made only when finished, reviewed now and with the nonce hold
  off (`faraday/faraday-core/src/wallet/state.rs:~903-958`). The part file
  is written only if the PSBT changed or a nonce-only pass opened a round.
  Writing is `put_outbox_beside`.
- A nonce hold takes Sign's and Collect's outputs back out of the Outbox
  (`apply_nonce_hold`, `faraday/faraday-core/src/wallet/state.rs:~1054-1081`).
- `CameraOff` (`faraday/faraday-core/src/wallet.rs:~4980`); toasts
  (`EN.register_added`, `keycheck_registered_one/many` and refusals).
- Nonce history written (see `decode`).
- No clock, randomness drawn by this feature beyond the MuSig2 session
  seed, clipboard or printing.

## State and lock

- `Spend` (`faraday/faraday-core/src/wallet.rs:~3103-3138`, Clone): `psbt`,
  `source`, `txid`, `signed_here`, `nonces_written`, `wrote: Vec<Written>`,
  `collected: Vec<(Fingerprint, Text)>`, `finished: Option<Transaction>`,
  `nonce_hold`, `v2`, `wallet_id`. It holds no secret (MuSig2 public nonces
  only) and is not a wiping type.
- `SpendState` (`faraday/faraday-core/src/wallet/state.rs:~12-112`) inside
  `WalletsArea` (`~450-1157`); `SpendSteps` (`~1160-1262`) with
  `done: [bool; 10]`. `SpendState.nonce_check: RefCell<Option<(Psbt,
  Rc<NonceCheck>)>>` caches the nonce check, read at
  `faraday/faraday-core/src/drawing/kept.rs:1140`.
- Seeds: `Session::sign` and `sign_with`
  (`faraday/faraday-core/src/wallet.rs:~1987-2015`) take the seed as
  `&Zeroizing<[u8;32]>` and pass it to `sign_job`; `sign_job`
  (`faraday/faraday-core/src/review.rs:~549-583`;
  `faraday/faraday-core/src/wallet/state.rs:~751-761`) borrows an
  `Option<&Zeroizing<[u8;32]>>`. The seed is not stored by the spend. The
  MuSig2 session is moved into the job and back
  (`faraday/faraday-core/src/review.rs:~614-624`). `MusigSession` zeroizes
  its seed and held secret nonces on drop
  (`core/faraday-psbt/src/musig.rs:656-664`).
- `SignWork.run` takes the session's wallets out and drops the session so
  no key rides in the answer (`faraday/faraday-core/src/signing.rs:~111-112`).
  `CollectWork` and `ReadWork` carry `Zeroizing` bytes and a job copy of the
  session (the keys' masters shared); they are dropped on the worker before
  the answer. `Fresh` holds public data and verdicts.
- Lock: `impl AtLock for WalletsArea`: `holds` is
  `Holds::session(spend().is_some())`; `lock` is `clear_spend()` and
  `row_open.lock()` (`faraday/faraday-core/src/wallet/state.rs:1264-1274`).
  The session's keys and wallets go at `Session::wipe`
  (`faraday/faraday-core/src/wallet.rs:1539-1549`). A lock ends in
  `lock_areas`, a restart and `Command::Exit`
  (`faraday/faraday-core/src/vault/seal.rs:933-946`), so nothing of the spend
  outlives a lock in the process.
- A lock drops the session seed (fix fb5f284c0).
- Back or a lock drops a signing answer still on the worker.
- The spend is also cleared when its wallet leaves (`wallet_left`,
  `refresh_spend`, `faraday/faraday-core/src/wallet.rs:~5042-5046`).
- What leaving the Spend screen does to the spend was not read.
- `session.key_xpubs` caches public xpubs of held keys (register sheet).

## Reach

- Opens: `decode` (Decode the finished transaction, Check a signer from
  Review, the Nonce check card), `Screen::Files`, the scanner (Scan a key),
  vault unlock, Type words (`add-key`), the Register sheet.
- Opened from: `tools` (Sign tile; Nonce check tile with a finished spend
  goes to the Finish step), Files and Inbox PSBTs, `spend-backup`
  (`faraday/faraday-core/src/spend_backup/mod.rs:~2126`, `~2232`), wallet
  pages.
- Reads: the session's keys and wallets, open vaults (seeds held there,
  passphrase keys), the nonce history (`memory::nonces`), Inbox and Outbox.
- `refresh_others` (`faraday/faraday-core/src/wallet.rs:~5070-5117`) lists
  other transactions spending the same inputs (`decode::check_others`), only
  when the spend was inspected again or the Inbox or Outbox changed.
- `faraday/faraday-core/src/slow.rs` lists the presses that start a worker
  job (`starts_job`, including Collect) and those run in the press
  (`runs_in_press`); `guard` is a debug assertion only.

## Rules and refusals

Review and stops:
- A Blocked warning stops; a Danger stops unless holdable or
  `NoParticipatingKey`. `review_stop`: all stopping warnings
  timelock-not-met gives `step_review_timelock`; all wallet-not-registered
  gives `step_review_register` (F135); any other stop gives
  `step_review_refused`; a holdable danger not yet held gives
  `step_review_hold_first` (F136); else none. No key here taking part is not
  a stop (`faraday/faraday-core/src/review.rs:17-54`).
- `hold` works only when the inspection is current and the only stop is
  "hold first"; then every holdable danger is held
  (`faraday/faraday-core/src/review.rs:~527`). `stopped` sets the error and
  follow (`~515`).
- The primary button is off (`PrimaryOff`) while `review_stop` holds a
  transaction nothing signs or a held danger
  (`faraday/faraday-core/src/screens/spend.rs:~504-507`, `~569-572`).
- Review is owed again after a PSBT change (`review_lapse`,
  `faraday/faraday-core/src/wallet/state.rs:~153-165`), after an unseen
  warning or key (`~191-220`), and when signed elsewhere (`open_finish`,
  `~255-270`). MuSig2 nonce-replacement warnings are excluded (`~176-178`).
  Warnings are compared as counted multisets with their detail, keys as
  sets; a later inspection whose holdable dangers differ reopens Review
  (step undone, hold cleared). A Collect adding only signatures or nonces
  signs on.
- Finish is refused until Review is done: it opens Review with an error
  (`faraday/faraday-core/src/wallet/state.rs:~255-270`, `~687-694`).
  Nothing reaches the Outbox until Review (and Check where the wallet is
  known) is gone through for that exact PSBT; Finish says "Finish step N
  first".
- A press on a step needing an earlier undone one is refused and that step
  opens, never past Sign (`stop_at_undone_step`,
  `faraday/faraday-core/src/wallet/state.rs:~712-739`).
- Sign is refused while a warning stops it or a key owes its backup
  (`signs_here`, `faraday/faraday-core/src/wallet/state.rs:~296-310`;
  `review::sign_is_stopped`). Hold-class warnings (F136) are held on Review
  (`~667-669`).
- The Check step is done only once the person has checked the money on the
  Check page.
- A transaction whose only output is a 0-value OP_RETURN (a BIP-322
  signed-message request) is blocked on Review, to be signed with Sign a
  message (`WarningKind::MessageRequest`, Level Blocked,
  `core/faraday-psbt/src/inspect/checks.rs:~58-62`).

Signing on the device (`faraday/faraday-core/src/signing.rs:192-250`), in
order:
- an undone step (`stop_at_undone_step`);
- a key made here whose backup is not done is refused, listing
  fingerprints (`sign_here_backup_first`); every press on a signature path
  refuses a key waiting for its backup;
- a pass opening a MuSig2 round with no randomness (`no_randomness`); a
  nonce is never made without a seed (`Error::NoNonceSeed`,
  `core/faraday-psbt/src/psbt.rs:~319`, raised at
  `core/faraday-psbt/src/sign.rs:~1085`);
- the same transaction stating other amounts than when signed here, with no
  override (`sign_here_this_transaction`);
- no job: silent; no open spend: silent.
- Only session keys in `participating_keys` sign; none gives
  `wallet_no_signing_key` (`faraday/faraday-core/src/review.rs:566-576`).
- Sign refreshes the spend's checks before reading them.
- The answer is applied only if the spend still holds the same PSBT, else
  dropped silently (`faraday/faraday-core/src/signing.rs:~287-297`).
- A spend held back by nonce history writes no part file; an unwritable
  part PSBT shows `spend_v2_not_written`. A complete spend takes the part
  file back only if this spend recorded it or the Outbox holds the same
  unsigned transaction.
- A miniscript or taproot-tree input with no registered wallet for its
  script is left unsigned and the rest is signed, saying why. An input
  spending a multi-key script naming one of the person's keys is signed
  only through a registered wallet holding that key. Key origin paths past
  `MAX_PATH_STEPS` = 16 steps (`core/faraday-bip/src/keys.rs:~310`) are
  refused, never cut (`core/faraday-psbt/src/psbt.rs:~117-135`).
- `take_signed` (`faraday/faraday-core/src/wallet.rs:~3371-3394`): a
  MuSig2 nonce-only pass is not noted as a signature. `note_written`
  (`~3399-3424`) notes only new partials.
- `aggregate_musig` (`faraday/faraday-core/src/wallet.rs:~3432-3464`)
  writes an aggregate only when `faraday_psbt::musig::aggregate` returns one
  verified under the output key, and skips inputs already final or lacking
  any participant's partial.
- A nonce this device made is written back only under the participant key
  and aggregate key it was made for; a signed copy with this device's nonce
  under another aggregate key is refused. The write-back replaces an older
  nonce pair of the participant key with the one just drawn.
- MuSig2 rand' is HKDF-Expand of the session seed with a draw counter
  (label `Faraday/musig-rand`, `RAND_LABEL`,
  `core/faraday-psbt/src/musig.rs:~503`); at the counter's last value the
  session refuses to draw and never wraps (`~580-584`).
- A version 2 PSBT signer clears the modifiable flags for the sighash types
  BIP-370 names (`update_modifiable`, `core/faraday-psbt/src/v2.rs:~1091`,
  `~1122`).

Opening a PSBT (`faraday/faraday-core/src/signing/read.rs`,
`faraday/faraday-core/src/wallet/v2.rs`):
- Start is refused without an open spend or Inbox file. The answer is
  dropped if the spend no longer holds the PSBT, or the Inbox file or the
  screen or sheet changed (`faraday/faraday-core/src/signing/collect.rs:~183-185`,
  `faraday/faraday-core/src/signing/read.rs:~159-165`).
- Not a PSBT: `start_spend_not_a_psbt`. A silent-payments PSBT:
  `start_spend_silent_payments` (this device does not sign silent
  payments). A `follow_psbt` refusal is shown as error text. A refused PSBT
  opened from Files shows on the details sheet, else a toast.
- The nonce check is not run at a spend's opening; a spend held by a nonce
  check stays on hold; a changed wallet owes a Check again.
- BIP-370 PSBTs are read as version 0 and written back as version 2, never
  as v0. Refused: a v0 PSBT with BIP-375 fields (global 0x07/0x08, input
  0x1d/0x1e, output 0x09/0x0a), a v2 PSBT with silent fields or an output
  with no script (`read_psbt_kept`,
  `faraday/faraday-core/src/wallet/v2.rs:24-50`). A version 2 PSBT that
  cannot take signatures back errors
  (`faraday/faraday-core/src/wallet/v2.rs:~107-114`).
- PSBT parsing in `core/faraday-psbt` refuses what BIP-370, BIP-371 and
  BIP-341 refuse: a taproot sighash byte 0 in a 65-byte signature, a v2
  output without script, invalid or wrongly sized locktimes and flags,
  bytes after the last map, key data on a field defined without it
  (including the v2 unsigned-transaction type), a compact size not in
  shortest form, repeated keys (linear-time check). A declared input count
  reserves nothing up front. Untouched fields are written back byte for
  byte. An empty `PSBT_OUT_SCRIPT` is the empty script.

Wallet match and limits:
- `MAX_PSBT_PLACES` 8192, `MAX_INPUT_PLACES` 20
  (`faraday/faraday-core/src/wallet.rs:~2258`, `~2264`): an input with
  more than twenty distinct key places has none read.
- `wallet_for` and `spends_from` look at the first input's script only
  (`faraday/faraday-core/src/wallet.rs:~2135-2145`). `txid_known` only if
  every spent output is P2WPKH, P2WSH or P2TR (`~2349-2359`).
  `same_wallet` sorts `sortedmulti` keys (`~2392-2422`).
- `Kind` (`faraday/faraday-core/src/wallet.rs:~2033-2053`): Single, Multi,
  TapMulti, Miniscript, Tree, MuSig, Silent, Other. `Kind::steps`
  (`~2108-2127`): Path for Miniscript and Tree, Nonces for MuSig2, Signers
  and Collect for kinds with cosigners.

Collect (merge another device's file, `Spend::collect`,
`faraday/faraday-core/src/wallet.rs:~3661-3773`; first refusals at
`faraday/faraday-core/src/wallet.rs:3661-3671`):
- not a PSBT: `EN.wallet_not_a_psbt`;
- another transaction id: `wallet_psbt_other_transaction`;
- another sighash field (SIGHASH_ALL and DEFAULT count equal):
  `wallet_sighash_field_other`;
- a witness not matching its output in the file alone:
  `file_witness_refusal` (`~4162-4177`, used at `~3688`);
- own input witness shape mismatch: `psbt_warn_witness_mismatch`;
- the file states something false (`file_lies`, `lie_refusal`):
  `wallet_outputs_differ_unproven` or `wallet_file_states_false`
  (`~3869-3917`);
- cannot combine: `wallet_cannot_combine`;
- `keep_own_over_file` conflicts `Conflict::{Finals, Rows, Scripts,
  False}`: `wallet_finals_differ`, `wallet_signature_unchecked`,
  `wallet_scripts_differ`, `psbt_final_prev_disagree` (`~3965-4150`);
- a final that cannot stand (`FinalRefusal::Signature`):
  `wallet_a_signature`;
- a signature reading Invalid or a `SighashMismatch` warning:
  `wallet_a_signature`, `wallet_sighash_other` (`~3773-3787`). A signature
  is accepted only under its input's sighash (ALL or DEFAULT where none is
  asked).
- A previous transaction is kept only when its hash matches the input's
  txid; a bare `witness_utxo` is kept but proves nothing (`~3712-3737`).
- A Collect for a cosigner's file applies only while the spend and file
  stand (see Opening).

Finish (`Spend::finish`, `faraday/faraday-core/src/wallet.rs:3818-3838`):
- refused while `nonce_hold` (`EN.spend_nonce_held`);
- refused on `refused()`: the first failing input (`refusal_of`,
  `~4189-4228`), in order witness shape, witness mismatch, utxo
  disagreement, final prev unread, false prev output, invalid final
  signature;
- not enough signatures: `wallet_not_enough_signatures`;
- a finalize error via `review::sign_error_text`; `finish_reason`
  (`~3840-3853`) returns `wallet_script_unsatisfied`.
- A nonce hold drops the finished transaction; lifting it re-runs
  `finish()`, which may refuse
  (`faraday/faraday-core/src/wallet/state.rs:~1054-1081`).
- The finalized witness is the smallest Bitcoin Core would write, with
  Core's tie-break order; the taproot leaf chosen is the one whose
  serialized witness is smallest. A refused input is named by its own
  index. A funding transaction without the output at the spent index is
  refused at finalize, not a crash.

Who signed (`Spend::signing`,
`faraday/faraday-core/src/wallet.rs:~3542-3637`; `count_rows`
`~3469-3521`; `musig_states` `~3314-3365`; `Tally::of` `~3184-3212`):
- a signature row counts only if valid, on an open input, not a MuSig2
  wallet's input, not already counted, with a stated place;
- a master is proved by `derives` or a held `xpub_at`, or counted with no
  proof only when no wallet names it (`~3513`); a row names a master only
  for a key of the script or leaf spent, a key-path row only via the
  internal key tweaked to the output key (BIP-341);
- MuSig2 participant states: `Here` (own partial verifies), `Word` (not
  own, not counted), `Fails`; a held secret nonce marks a participant still
  to sign; a master with an open or failing participant is still to sign,
  a failing one also in `failed`.
- A script-path signature on a leaf the device cannot run reads as not
  checked, names no fingerprint and is not counted by Collect.

Paths (`paths`, `faraday/faraday-core/src/wallet.rs:~4424-4491`):
- a BIP-68 relative lock is in force when bit 31 is clear, time-based when
  bit 22 is set; a Blocks lock is ok only if relative, not time-based and
  value >= n; Intervals likewise; absolute locks are shown as text only and
  ignored by `sequence_ok`; blocks to days is n/144.
- A lock is stated only when some input's sequence is not 0xFFFFFFFF;
  version 3 is always replaceable.
- `musig_nonces` (`~4493-4522`) reads the first input's public nonces
  (errors give none).
- `faraday/faraday-core/src/wallet/paths.rs`: "Key N" / "Keys N and M, k
  of k"; "about" only for block waits; `moved_row` shows the date alone and
  never the warning when the clock is unchecked.

Register (`faraday/faraday-core/src/register.rs`,
`faraday/faraday-core/src/wallet.rs:~4759-4837`):
- opens only if `inspection.register` is some; Hold needs the ask and the
  offer; uses a loaded key at its path (`yours`, `~4786-4792`).
- Name: control characters not typed, length capped at `WALLET_NAME_MAX`
  bytes, blank uses the file's name (`register::name_of_file`)
  (`faraday/faraday-core/src/register.rs:60-95`).
- A refusal shows on the sheet (`register_refused`, `~4833`) except
  `Refusal::Unconfirmed`, which holds for its own sheet (`~4818`).
- A register hold counts as confirmation (`wallet_confirmed_by_register`,
  `faraday/faraday-core/src/wallet.rs:~1472`) only while
  `refused_before_sheet` still accepts it.
- Only held keys derive (`wallet_xpubs`).

Keys and names:
- Rename accepts while `chars().count() < 40`
  (`faraday/faraday-core/src/wallet/state.rs:537-548`).
- Key rows list each fingerprint once; a key name reading like a mark
  ("Signed", "Signs here", lookalike letters `1 l ! |` as `i`, `5 $` as
  `s`) or equal to the fingerprint is left out
  (`faraday/faraday-core/src/wallet/state.rs:~355-360`, `~1300-1330`).
- `faraday/faraday-core/src/wallet/own_name.rs` (`Session::own_wallet_name`): the registered wallet
  name, else key label or fingerprint plus the account kind (kind only for
  purposes 44, 49, 84, 86).

Amounts and sighash (in `core/faraday-psbt`):
- Amounts at most MAX_MONEY and their sums checked as Bitcoin Core does;
  over range is blocked with `AmountOutOfRange`, no way past
  (`core/faraday-psbt/src/inspect.rs:~156`;
  `core/faraday-psbt/src/inspect/checks.rs:~80`, `~89`).
- A finalized input's amount is verified like any other input's. The
  spent output is stated only when the previous tx id matches and agrees
  with `witness_utxo`; an input whose spent output is false is refused
  everywhere, final or not.
- A finalized non-taproot input is checked against the output it spends
  (`witness_mismatch`).
- SIGHASH_SINGLE with no output at its index is blocked
  (`Detail::SingleNoOutput`, `core/faraday-psbt/src/inspect/sighash.rs:~46`;
  wording `EN.psbt_warn_single_no_output`,
  `faraday/faraday-core/src/review.rs:~266`); with one it needs force. A NONE sighash is blocked. A signature stricter than asked (ALL
  where ALL|ANYONECANPAY or SINGLE was asked) is a Caution.
- A timelock is met only as consensus checks it (BIP 68, 112).
- Change at address index 10,000 or more (`FAR_CHANGE_INDEX`,
  `core/faraday-psbt/src/inspect.rs:~59`) is shown with a Caution (F242);
  signing stays possible. Change is verified only when every change output
  is, else "n of m unverified".
- A non-compressed MuSig2 aggregate key is refused (BIP-373); refused
  MuSig2 fields count as not read.
- A script-path witness element is data only when a hash of it is pushed
  by the leaf; a hashlock preimage is not shown as a signature.
- A fee is stated only when known (128-bit sums, money range); else
  "Cannot tell" or "amount not stated".

## Domain calls

- `faraday_psbt::{finalize, inspect, register, message}`
  (`core/faraday-psbt/src/`).
- `faraday_psbt::verify::{states_false, witness_mismatch, utxos_disagree,
  final_unread, final_invalid, final_signature_wrong, script_row_verdicts,
  tap_entry_commits, input_finishes}` (`core/faraday-psbt/src/verify.rs`).
- `faraday_psbt::musig::{pub_nonces, partial_sigs, aggregate,
  verified_partials, picked_partials}` (`core/faraday-psbt/src/musig.rs`);
  signing in `core/faraday-psbt/src/sign.rs`; v2 in
  `core/faraday-psbt/src/v2.rs`; `Psbt`, `Inspection`, `MusigSessionView`.
- `faraday_bip::spend::Lock`, `faraday_bip::WalletId`
  (`core/faraday-bip/src/`).
- `faraday_keys` session keys (`faraday/faraday-keys/src/session.rs`).
- `zeroize::Zeroizing`.

## Fixtures

- `faraday/faraday-samples/data/savings.psbt`
- `faraday/faraday-samples/data/spending.psbt`
- `faraday/faraday-samples/data/taproot.psbt`
- `faraday/faraday-samples/data/nested.psbt`
- `faraday/faraday-samples/data/legacy.psbt`
- `faraday/faraday-samples/data/vault-nested.psbt`
- `faraday/faraday-samples/data/vault-legacy.psbt`
- `faraday/faraday-samples/data/taproot-multisig.psbt`
- `faraday/faraday-samples/data/inheritance.psbt`
- `faraday/faraday-samples/data/tree.psbt`
- `faraday/faraday-samples/data/musig.psbt`
- `faraday/faraday-core/tests/vectors/interop/bluewallet/quicklook-preview-sample.psbt`
  (read as a PSBT file by `classify`,
  `faraday/faraday-core/tests/all/interop.rs:~56-64`)

(The samples are the Demo spends, `faraday/faraday-samples/src/data.rs:559-659`.)

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 072d8578f | A spend with many inputs each stating many key places made the spend check cost grow with its size | Per-input wallet check computed once per open input set |
| fb5f284c0 | A lock did not drop the session's seed; the next signing round reused it | Lock drops the session seed |
| 711e31f9c | Presses on a signature path did not refuse a key waiting for backup | Every press refuses it |
| fc3d58544 | A MuSig2 pass could sign under nonces a later share replaced, leaving stale partial signatures | Nonces are shared before signing; the PSBT holds what the pass reports |
| 4bf1d2ece | A v2 PSBT declaring a huge input count reserved memory before its maps were read; a wallet JSON with many members was checked for repeated keys in quadratic time | A declared count reserves nothing up front; repeated JSON keys found in n log n |
| 40f0b80e6 | A transaction whose only output is a 0-value OP_RETURN (a BIP-322 request) could be signed as a payment | Blocked on Review as a signed-message request, to be signed with Sign a message |
| e4f3b7d6b | Review warned of a time lock no input's sequence made hold; a version 3 spend read as Final | A lock is stated only when some sequence is not 0xFFFFFFFF; version 3 is always replaceable |
| b7e59396f | A re-inspection after loading a wallet or key with a new holdable danger did not reopen Review | Review reopens when holdable dangers differ from those shown when it was passed |
| df92da3a6 | A Collect adding warnings or keys after Review was signed without a new Review; a Collect adding only signatures or nonces also had to sign on | Changed warnings or keys reopen Review; only signatures or nonces sign on |
| b6e21a0f9 | A second warning of a kind on another output did not reopen Review (warnings compared as sets) | Review's baseline is what it showed; warnings compared as counted multisets with detail |
| 9e13ce9df | A finalized 65-byte taproot signature ending in sighash 0x00 read as valid | Malformed; one shared check for PSBT fields and finalized witnesses |
| 6f3d8c9bf | A 64-byte script-path witness element was always read as a signature, raising false dangers | Data only when a hash of it is pushed by the leaf; a non-verifying data-like element is Unchecked |
| 9ee972cc8 | A hashlock preimage read as an unchecked signature in Review | A hashlock preimage is not shown as a signature to check |
| 251204bc2 | Change at index 10,000 or more was shown as ordinary verified change | Shown as change with a Caution; signing still possible (F242) |
| caf3e7142 | The change row did not name the wallet and read Change on any chain; a transfer did not name the wallet it paid | Change named only on chain 1, with its wallet; a transfer names its wallet |
| 2cbe8c4ba | MuSig2 and FROST secret nonces, FROST shares and silent-payment keys were multiplied with the variable-time public-scalar routine | Secret scalars multiplied in constant time (`secret_base_mul`) |
| 8e2605814 | A returned signature under a sighash other than its input asked passed as valid | Accepted only under its input's sighash; NONE is Blocked |
| 3723b1218 | A signature stricter than asked showed a Danger card | Stricter-than-asked is Caution |
| 683e73f9e | The signer stopped on a taproot pkh leaf and read a miniscript wallet that was not sane | No stop on a taproot pkh leaf; a non-sane miniscript wallet is refused |
| 85b47d69f | Sign with stayed live while a check on Transaction stopped signing | Signing stays off while the review stop holds (F136) |
| 8dffff79b | The main button cut "Sign with" fingerprints short with two or more keys | The button counts keys when names do not fit |
| 823ef2fbc | At larger text, Finish's two file cards overlapped | Cards side by side only where titles and buttons fit |
| 1d5219c69 | Finalizing a wsh miniscript multi() with extra signatures wrote the first k, not the smallest | The finalized witness is the smallest Core would write, with Core's tie-break |
| 580d68970 | A taproot script-path input could be finalized with a leaf one byte heavier serialized | The leaf with the smallest serialized witness is chosen |
| 41be3a2a6 | This device's MuSig2 nonce could be written back under an aggregate key it was not made for | Written back only under the participant and aggregate key it was made for |
| 7501864ae | A MuSig2 nonce's random input was not wiped on drop; the session seed was copied | Nonce input wiped; seed passed by reference |
| 4f5492079 | A v2 PSBT signed with SIGHASH_ALL came back still modifiable | The signer clears the modifiable flags BIP-370 names |
| af64f09a9 | A MuSig2 participants field with a non-compressed aggregate key was not blocked | Refused (BIP-373) |
| 4dda26858 | A refused MuSig2 field was counted as read | Refused MuSig2 fields count as not read |
| c70527885 | The PSBT map parser accepted a compact size not in shortest form | An error in v0 and v2 |
| abba04b57 | Review rows listed a length written in more bytes than needed as valid | Non-minimal length not listed as valid |
| 4ed95455f | A v2 PSBT with key data on a field defined without it was accepted | Refused as Core and HWI refuse it |
| c520e654d | A v2 unsigned-transaction type with key data was accepted | Refused |
| 02835fe88 | An input with thousands of key origins made each spend check derive thousands of scripts | An input with more than twenty distinct key places has none read |
| 661712998 | A v0 PSBT paying the empty script lost its form through v2 and back; Spend refused the write | An empty `PSBT_OUT_SCRIPT` is the empty script; v0 round-trips |
| 0bf2afb5b | An input whose `witness_utxo` or previous transaction stated a false output passed nonce check, collect and finish | Refused everywhere, final or not |
| 033f05918 | A taproot ANYONECANPAY input with a false spent output panicked the nonce check; an honest cosigner's file was named wrong | Reads as an unreadable output; the merge refuses with the spend's reason |
| 745d3902b | A finalized non-taproot input was never compared with its output | Checked against the output it spends (`witness_mismatch`) |
| d5c68605f | The signed count counted a master that had not signed | A row names a master only for a key of the script or leaf spent; key path only via the tweaked internal key |
| 477302144 | A valid script-path signature on a non-miniscript leaf read Invalid and Collect refused a file Core accepts | Such rows read as not checked; no fingerprint or count |
| 08bffb5cc | A PSBT of about 3200 inputs took seconds to inspect and Collect | Inspecting, opening and Collect far faster |
| 8b62a00c8 | The Sign card signed before the steps before it were done | Every route to signing requires the earlier steps; else the first opens |
| 1611a5498 | Fee bump and Compare could show "Only the fee" for an out-of-range or unstated fee | A fee only when known (128-bit sums); else "Cannot tell" |
| bd4debdc0 | A fee bump with a total past 21,000,000 BTC was shown as a fee | Money-range check as a node applies it |
| 9804f88c7 | A PSBT signed enough elsewhere, or completed by Collect, reached the Outbox without this device's Review | Nothing reaches the Outbox until Review (and Check) for that exact PSBT |
| 48d98ee1b | A MuSig2 round with no seed made a nonce from all-zero bytes; Sign read stale checks | Refused with `NoNonceSeed`; Sign refreshes checks first |
| efb5a99a9 | MuSig2 rand' drew from the session seed without a counter | HKDF-Expand with a draw counter; refuses at its last value |
| f97886762 | A v2 MuSig2 PSBT with an older nonce of this device's was not written back with the new nonce | The write-back replaces the participant's nonce pair |
| e8b153851 | A PSBT with BIP-375 silent payment fields was signed without its output scripts checked | Refused with a toast |
| cfd3086c9 | A PSBT with tens of thousands of pairs in one map stalled the device | Linear-time duplicate check, still refused |
| c0da9d6be | A miniscript or taproot-tree input was signed through no registered wallet | Left unsigned, the rest signed, saying why |
| 6dd1e7b1f | A multi-key script input naming one of the person's keys was signed without a registered wallet; origin paths unbounded | Signed only through a registered wallet holding the key; origins past 16 steps refused |
| 11da09a2a | A PSBT input with a funding transaction lacking the spent output panicked the extractor | Refused at finalize; spend paths ask a bounds-checked `spent_output` |
| 2c46b9078 | On the Pi panel, the review's Sign button and names ran off the edge | Nothing cut off at the panel's sides |
| 03edba890 | PSBT parsers accepted inputs BIP-370, BIP-371 and BIP-341 refuse and did not write untouched fields back byte for byte | Refuse what the BIPs refuse |
| 9e6e10f13 | Review shortened a recipient's address; Continue could sit below the card | The whole address in groups of four; Continue reachable by scrolling |
| 956aa05dc | The Check step was marked done without the money checked | Check done only once the money is checked on the Check page |
| 31f7f5841 | A finalized input's amount was trusted unchecked | Verified like any other input's |
| 5a47047e4 | A refused input was always named input 0 at finalize | Named by its own index |
| 266b6d395 | An input's spent output could come from a previous transaction with another id | Stated only when the id matches and agrees with `witness_utxo` |
| 597729254 | A wsh miniscript witness counted every ECDSA signature as 73 bytes | Smaller satisfaction chosen by real sizes |
| 2e42ef107 | The panel said change was verified when only some was | Verified only when every change output is |
| 2755db545 | SIGHASH_SINGLE with no output at its index was not refused | Blocked; with one it needs force |
| 20a5bd564 | A timelock was judged met without consensus rules | Met only as consensus checks it (BIP 68, 112) |

## Doubts

- Rows moved to other features of this cluster: aee93f731, e329d6ff1,
  315094b62, 0074b6d25, 2b1a9b266, 4f5e3bdd8, 38a890f28, 966560ab2 to
  `messages`; c287628be and c3ab6eb33 to `decode`; 4598a2dca to `messages`
  and `tools`.
- Step names: one account listed step 1 as "addresses"; the code's label is
  `EN.app_check` (`faraday/faraday-core/src/screens/spend.rs:212`). Another
  account gave the steps as Review, Check, Signers, Nonces, Sign, Collect,
  Finish; both match the ten `STEPS` in a different order of naming.
- `sign_job` is placed in `faraday/faraday-core/src/review.rs:~549-583` by
  one account and in `faraday/faraday-core/src/wallet/state.rs:~751-761`
  by another; both may be true (a `Session` method and a spend-state
  wrapper). Not checked.
- `MAX_PATH_STEPS` is named in a doc of the register sheet but not visible
  in the code read.
- `expect("spend")` on `app.wallets.spend()` at
  `faraday/faraday-core/src/screens/spend.rs:768` relies on the dispatcher
  checking first; reachability unchecked.
- `K::None` locktime prints a literal "0", not from the strings file.
- `faraday/faraday-core/src/slow.rs` entries say "not yet timed on a Pi 3B+".
- What leaving `Screen::Spend` (not a lock) does to `SpendState` was not
  read.
- 2cbe8c4ba names old crate paths (`core/osk-bip`, `core/osk-psbt`,
  `opensigner/`) that no longer exist; the rule carries to
  `core/faraday-psbt` and `core/faraday-bip`.
- 11da09a2a and 03edba890 are mixed commits (EFI image reading; BIP-39
  NFKD and vault records); only the PSBT parts belong here. 2c46b9078 also
  touches Transfer.
- caf3e7142's file list was not inspected beyond its body.
- 711e31f9c ("signature path") touches `faraday/faraday-core/src/qr_view.rs`
  and `faraday/faraday-core/src/state.rs`; whether it is about spends or
  messages is not clear.
- Many line numbers above come from one account each and were not re-read.
- The PSBT QR vectors (`faraday/faraday-qr/tests/vectors/wallets/psbts.txt`,
  `multisig-5-psbt.png`, `multisig-6-psbt-bbqr-part-*-of-10.png`) are read
  only by the scanner's own tests (`faraday/faraday-qr/tests/interop.rs`);
  they belong to scanning, not listed here.
- Rows 2cbe8c4ba (constant-time multiplication of secret scalars) and
  7501864ae (a MuSig2 nonce input not wiped on drop) are kept: the project's
  security scope names secret-key arithmetic outside a constant-time
  library, and a Faraday secret type that does not wipe, as ordinary bugs.
  bce5d73fa (old secret nonces left in freed memory) is left out under the
  scope rule.
- The rule anchors marked `~` that point into `core/faraday-psbt` and
  `core/faraday-bip` were found by searching the code at `d89dde5de` for
  the constant or error the fix row names; `faraday/faraday-core/src/screens/spend.rs:212`
  was read from the code.
