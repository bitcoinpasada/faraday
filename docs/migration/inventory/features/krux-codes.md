# Krux encrypted codes

Krux encrypted codes (KEF) hold a seed, a note or any text sealed under a
passphrase, as Krux writes them. Faraday reads them and makes them. Reading
is for a person who brings a Krux code in, by camera or from a file: the
Krux read sheet opens it and offers what it holds (a seed, a note, a code
inside it). Making is in two places: the Krux code tool in Tools
(`Screen::KefTool`), which seals typed text, a wallet's description or a
loaded key, and a seed's Krux code item on the backup board, which uses the
same form (`kef_form`). The key derivation (PBKDF2) runs as worker jobs.

## Screens and sheets

Read
- `Sheet::KefRead` (`faraday/faraday-core/src/seeds/kef.rs`), opened by `Action::ScanKef` (`kef_code_scanned`) or `Action::KefFile(i)` (`kef_file`). Stages: Ask, Working, Offer, Note, Bytes.
- The read rows also appear on Add a key's Other forms and on the Krux tool's READ card (`seeds::kef::read_rows`).

Make: the form (`faraday/faraday-core/src/kef_form/mod.rs`, `faraday/faraday-core/src/kef_form/screen.rs`)
- Stages Entry, Making, Made, Check, Comparing (`faraday/faraday-core/src/kef_form/mod.rs:~38`).
- Entry: the passphrase pair or dice entry for a passphrase; Make.
- Code card (Made): the code's picture, its size, Check, Another, By hand, block pins and rows.
- Check card: type the passphrase again and Compare.
- In-progress card (Making or Comparing) with Cancel.
- Options sheet `Sheet::KefOptions` (`compact::dialog`): size, version, low correction, rounds, id (random 8, random 16, label, fingerprint).

Make: the tool (`faraday/faraday-core/src/kef_tool/mod.rs`, `faraday/faraday-core/src/kef_tool/screen.rs`)
- `Screen::KefTool`; cards READ (`kstep::READ`), Contents, Make, Code. Contents pick (`flow::Pick`): type, wallet or key. `Sheet::KefOptions`.
- The take-away shows the made picture and file only, never the content or passphrase (`faraday/faraday-core/src/kef_tool/screen.rs:~261-300`).

## Actions

Read (`KefAct` in `faraday/faraday-core/src/seeds/kef.rs`)
- `Field` (type the passphrase), `Open`, `Hex` (passphrase as hex), `Show` (eye), `NotEncrypted`, `UseAsPassphrase` (carries a note to the next seed's passphrase and fires `ScanSeed`), `ReadAsSeed` (a 16 or 32 byte note), `Cancel` (while Working, returns to Ask).

Form (`KefAct` in `faraday/faraday-core/src/kef_form/mod.rs`)
- `Field`, `FieldAgain`; `Dice`, `DiceField` (needs `may_load_keys()`, `~536`); `Options`, `Back`; `Size(side)` from precomputed `sizes` (`~543-548`); `Version(v)` (only `WRITTEN_VERSIONS`, `~561`); `Low` (only if `low_offered()`, `~563`, `~570`); `Rounds` (digits, at most 7, `~660`); `Id(Random8|Random16|Label|Fingerprint)`; `Label` (at most `ID_MAX` bytes, no control characters, `~673-675`); `Make` (`~966`).
- Made: `Check`, `Another`, `ByHand` (only if `by_hand_offer` and the side is at most `KEF_BY_HAND_MAX_SIDE`), `Pin(n)`, `Rows(bool)`.
- Check: `CheckField`, `Compare` (`~845`), `CheckBack`; `cancel`.
- `NotOnPaper`, `ScanCopy` and `Done` are handled by the caller.

Tool (`KefToolAction`)
- `Open`; `OpenOn(fp)` (only a loaded key on offer; calls `explore_hide`, `faraday/faraday-core/src/kef_tool/mod.rs:~296-305`); `Step(k)` (only after earlier steps are done, `~382-395`; READ is always reachable); `Pick(Type|Wallet|Key)` (one key on offer starts the form at once, `~352-355`); `Field`, `Eye`; `Wallet(i)`; `Key(i)`; `Next`; `SaveFiles`; `Form(a)`, `Dice(p)`; `Form(Done)` goes to `state.back`; `Backup(a)`, `BackupDice(p)` forwarded to the backup board's Krux item (`wallet_backup::kef`).
- Keys: Enter while typing is Next; Escape closes Options, else goes Back.

## Effects

- Worker jobs (`faraday/faraday-core/src/worker/kef.rs`): `SealWork` and `OpenWork` run `faraday_keys::kef` `SealJob`/`OpenJob` (PBKDF2) with `c: &Compute`; `TimeWork` is a 50,000-round timed `pbkdf2_hmac_sha256_block` with a stop flag that gives the ms rate for the read card (`None` if stopped or zero time). A seal or an open stops a running timed run first.
- Form: `app.kef_make_seed` / `kef_make_text` (`faraday/faraday-core/src/kef_form/mod.rs:~977`, `~980`), `kef_take_made` (`~704`), `kef_open` for Compare (`~855`), `kef_take_opened`, `kef_make_cancel`.
- Read: `kef_time_start`, `kef_timed` (timed run on the first read), `kef_open`, `kef_take_opened`, `kef_job_drop`; the camera reopens with `Ask::Any`; `note_secrets()`; a toast on a bad file.
- Randomness: the dice roller draws `app.fresh(b"faraday dice roller")` (`faraday/faraday-core/src/kef_form/mod.rs:~927`); `KefStart::NoFreshBytes` asks for more entropy and repeats the press.
- Files: `app.kef_save_files` writes to the Outbox the picture (PNG) and the sealed code file (`faraday/faraday-core/src/kef_form/mod.rs:~709`, `~744`); `toast_out`.
- Tool: `sha256_of` the taken files (`faraday/faraday-core/src/kef_tool/mod.rs:~628`), `app.visit.out_extend` (`~643`); `receive_held` and `silent_marks` for silent-payments wallets (`~208-211`); toasts for refusals.
- Camera: the form only asks `app.cameras()` (`faraday/faraday-core/src/kef_form/mod.rs:~492`, `~886`).
- `made` draws a `QrView::kef` base43 QR with a version, count and id label; bytes `Zeroizing`; SHA-256 through `sha256_whole` (`faraday/faraday-core/src/worker/kef_make.rs`).

## State and lock

Read
- `KefRead` (`faraday/faraday-core/src/seeds/kef.rs:~102-125`): `pass: PassField`, `forms: Vec<Zeroizing<Vec<u8>>>` (passphrase forms to try), `offered: Option<Zeroizing<Vec<u8>>>`, `shown: Option<Text>` (the note text or hex), `eye`, `sure`. `holds_secret()` = pass, forms, shown or offered.
- Cleared when the sheet closes, and at lock: the seeds area's lock clears the seed sheet, the Krux read and the carried passphrase (`faraday/faraday-core/src/seeds/mod.rs:455-464`, `1086-1094`).
- The ways the sheet ends that drop `KefRead` with `shown`: Cancel outside Working (`faraday/faraday-core/src/seeds/kef.rs:791-803`), the general Cancel (`faraday/faraday-core/src/action.rs:1374`), Use as passphrase (`faraday/faraday-core/src/seeds/kef.rs:740-748`), read on as a public code or seed words (`faraday/faraday-core/src/seeds/kef.rs:655-700`), a stick arriving (`faraday/faraday-core/src/seeds/kef.rs:753-765`), its wallet leaving (`faraday/faraday-core/src/seeds/kef.rs:769-785`), the worker's answer finding another sheet up (`faraday/faraday-core/src/seeds/kef.rs:486-491`), the lock.
- A stick arriving closes the sheet and drops the carried passphrase. `hide_secrets` only closes the eye. `kef_cancel` in Working keeps `KefRead` but clears `forms` (`faraday/faraday-core/src/seeds/kef.rs:~791-799`).
- The carried note lives only on the seed scanner, its card and the Krux sheet it leads to; it is forgotten on Cancel, at lock, and when the next code holds something public or is a codex32 string (fix rows 1450b6c00, e867840c5).

Form
- `KefForm` (`faraday/faraday-core/src/kef_form/mod.rs:~69-108`): `source` (`Seed{key, bytes, fingerprint}`, a key id only, or `Text(Text)`, the text being sealed), `pass`, `check` (`PassField`), `dice: DiceEntry`, `options`, `label` and `rounds` (`TextBox`), `made: Option<Made>` (the sealed bytes, not secret), `saved`, `checked`, `offered`, `sizes`, `error`, flags.
- `holds_secret()` = a typed passphrase, a check passphrase, or dice rolls entered (`~228`).
- `close` cancels the work and clears `pass`, `check` and the dice rolls; keeps `made`, `checked`, `saved` (`~771`). `again` clears made, saved and the passphrases (`~750`).
- A stick attached (`stick_attached(true)`) wipes the dice rolls and drops focus (`~160`).
- The passphrase copy given to the worker is `Zeroizing` (`~846`, `~967`).

Tool
- `KefToolState` (`faraday/faraday-core/src/kef_tool/mod.rs:~94-113`): `open`, `done[3]`, `back`, `pick`, `typed: TextBox` (the text to seal; secret for a text source), `typing`, `shown` (eye), `wallet`, `key` (indices), `form: Option<KefForm>`, `out` (SHA-256 of taken files). Held in `app.tools.kef_tool: KefToolArea`.
- Lock: `AtLock for KefToolArea` (`faraday/faraday-core/src/kef_tool/mod.rs:772-779`): `holds` = typed text non-empty or the form `holds_secret`; `lock` = `clear()`, dropping the whole state.
- Leaving: `tidy` (`faraday/faraday-core/src/kef_tool/mod.rs:601-615`) calls `wallet_backup::kef::tidy`, then `clear()` unless the screen is KefTool or Randomness; when kept and Options was closed from outside, it closes the form's Options. The tool survives a visit to Add randomness.

Worker
- `KefArea { progress, sealed: Zeroizing<Vec<u8>>, opened, timing }` (`faraday/faraday-core/src/worker/kef.rs`). `started` clears the answers and sets progress, but not `timing` (`156-160`). `stop` cancels progress but keeps the answers. `clear`, the lock, stops the job and the timing and clears `sealed` and `opened` (`175-180`). `take_sealed` and `take_opened` hand answers to a screen.

All areas: a lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).

## Reach

- Read sheet opened from the scanner (`ScanKef`), Files (`KefFile`), Add a key's Krux form, and the tool's READ card. It hands a seed to the seed sheet (`ReadAsSeed`, as a compact SeedQR) and a note to the next seed's passphrase (`UseAsPassphrase`, `ScanSeed`).
- The tool is wired from Tools (`faraday/faraday-core/src/tools/mod.rs:340`, `736`, `748`, `757`). New key sends a Krux start to Add a key (`keygen_to_krux`).
- The form is used by the tool and by a seed's Krux code item on the backup board (`wallet_backup::kef`, also `wallet_backup::structural_fixed`).
- Uses `dice_entry` (passphrase from dice, see `randomness`), `pass_field` (`Kind::KefMake`, `Kind::KefRead`, `pass_pair`, `tool_box`), `worker::{kef, kef_make}`, `vault::{TextBox, screen::{text_box, secret_box}}`, `stick`, `screens::{dice_pad, randomness::row, busy_bar, copy_grid, next_button_when}`, `compact::dialog`, `demo::PASSPHRASE`, `pick::{Decl, Take}`, `wallet`, `trail`, `flow`, `at_lock`, `sha256_of`.
- Practice samples include a Krux code file, offered by the scanner's Sample tile in turn with other samples.
- Documented in `docs/KEF.md` (§6, §9, §15.1-15.3).

## Rules and refusals

Read (`faraday/faraday-core/src/seeds/kef.rs`)
- An iteration count above 550,000 needs a second Open (`COUNT_ASKED`, `26`).
- Nesting depth at most 8 (`DEPTH_MAX`, `29`); see also `237`, `269`.
- Nothing runs before Open.
- A hex passphrase needs an even number of hex digits; spaces are ignored.
- The typed form is tried first, then the other forms; if all fail, "failed" and back to Ask.
- An answer for a sheet no longer shown is dropped.
- The passphrase field takes characters only in the Ask stage and not with a stick in the way.
- Contents are a note only if they are UTF-8 with no control characters other than tab, newline and CR. 16 or 32 bytes that are not text go to the seed branch as a compact SeedQR; 16 or 32 bytes of text are a note and offer Read as a seed unless control characters are present (fix rows 1450b6c00, 765c9e129). Other bytes are shown as hex.
- A code inside the code is offered unless at the depth limit.
- Reading a Krux file is gated by `may_load_keys`; the scan path is not.
- File kinds shown: Text, Other, KruxCode.
- Scan again and Open wait for the stick to be out; a stick plugged in closes the sheet and drops the held secret; the seed sheet does not open with a stick in ("Remove the stick first.") (fix rows d4d584a61, 12f84b7ea, 1450b6c00).

Form (`faraday/faraday-core/src/kef_form/mod.rs`)
- Make is refused unless the stage is Entry and the passphrases agree (`966-969`).
- Any secret press is refused while the stick is in the way (`stick::refuses_secret`, `~204-214`).
- Passphrase keys follow `pass_field` `Kind::KefMake`: printable ASCII, at most `PASSPHRASE_MAX`.
- A passphrase from dice words must pass `writable_passphrase`, else `kef_refuse_passphrase` (`~958-962`).
- In Practice (`!makes_secrets()`) the public `demo::PASSPHRASE` replaces rolled words (`953-956`).
- Non-numeric rounds parse to 0 (`~909`), with the `kef_refuse_count` line in Options.
- The fingerprint id is offered only when available. A key's id is the fingerprint of the words alone, written as Krux writes it: 8 lower-case hex characters as text (fix rows 3ccbeaee5, 44e9cc139).
- Make from a key no longer held: `MakeStart::NotOffered`. `NoFreshBytes`: `kef_form_need_random` (`~986`). A worker error returns to Entry (`~716`).
- Compare is refused silently if the code does not parse (`faraday_keys::kef::unwrap`, `~847`).
- Compare outcome: a match goes to Made and sets `checked`, unless the code is on Paper with a camera and not yet scanned (`~886`); opened but another seed: `check_made_other_seed`; not opened: `check_made_failed` (`~903`).
- Put on Paper with a camera attached, a code not yet scanned reads Not checked until its drawn copy is scanned; with no camera it stays Checked; state reads Not checked, never Not made (fix rows 8e4ad5d5a, 2c1b52674, 7f0c748b6).
- Scan of the copy is offered only for sides 25-37 (`~352-360`).
- Size shows "no size" when the contents are too large (`faraday/faraday-core/src/kef_form/screen.rs:~546-552`).

Options (`faraday/faraday-core/src/worker/kef_make.rs`)
- `problem()` (`74-87`): count in `COUNT_MIN..=COUNT_SETTING_MAX`, else `kef_refuse_count`; a random id of `DEFAULT_ID_LEN` or `LONG_ID_LEN`, a label non-empty and at most `ID_MAX`, else `kef_refuse_id`; version in `WRITTEN_VERSIONS`, else `kef_refuse_version`.
- Low correction only for version 5.
- `side` finds the smallest QR version 1..=25 that holds the code (None for versions not written).
- `refusal_line`: Empty, Passphrase, Refused(Count), Refused(IdLength), Refused(_)/Iv/Cancelled (all as `kef_refuse_version`), Duplicate, NulEnd.
- `IdChoice::{Random(n), Label(Text), Fingerprint([u8;4]), WordsFingerprint}`.

Tool (`faraday/faraday-core/src/kef_tool/`)
- Only `kef_offered()` keys (12 or 24 English words) can be picked; others are shown with `kef_tool_not_this_key` (`faraday/faraday-core/src/kef_tool/screen.rs:~171-179`).
- A wallet's text is its checksummed public descriptor; the fingerprint is offered as id only for a single-key wallet (`faraday/faraday-core/src/kef_tool/mod.rs:~213-215`).
- A silent-payments wallet with a key waiting: `code_view_no_address`; an unknown wallet: `code_view_that_wallet` (`~196`, `~207-211`).
- Next is refused for empty text or when a secret is in the way (`362-372`).
- Typed text is limited to `COMPRESS_MAX`; control characters are refused (`~549-550`).
- The take-away row only when hashes are present (`~628-643`).
- Options opens only on the KefTool screen (`~412`).

## Domain calls

- `faraday/faraday-keys`: `kef::{unwrap, seed_matches, writable_passphrase, reading_passphrases, WRITTEN_VERSIONS, DEFAULT_ID_LEN, LONG_ID_LEN, ID_MAX, PASSPHRASE_MAX, OpenError, Envelope, SealJob, OpenJob}`, `KeyId`, `Keys`, `words::from_compact_seedqr`.
- `core/faraday-codec`: `kef::{base43_encode, Mode, recognize, id_text}`.
- `faraday/faraday-intake`: `seed_in_code`.
- `core/faraday-crypto`: `hex`, `pbkdf2_hmac_sha256_block` (timed run), `work::Compute`.

## Fixtures

- `faraday/faraday-samples/data/krux-code.kef`: Practice's sample Krux code (included at `faraday/faraday-samples/src/data.rs:~694`).

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 2d753ce8b | The code size shown did not match the code made for some versions and id lengths; Forward after leaving the tool opened an empty page; the idle warning did not close the form's passphrase eyes. | The size shown matches the code made for every version and id length; Forward opens nothing empty; the idle warning closes the eyes. |
| 7f0c748b6 | On Paper, the last blocks of a 33 x 33 or 37 x 37 grid could not be marked; the miss count per camera close was wrong; Check my code said "Not made" for an opened code without a scanned copy; "Checked" stuck when a camera was plugged in later. | Any block can be marked; a scan miss counts once per camera close; Scan my copy vs Check my code follows the camera; the state reads Not checked, never Not made. |
| 8e4ad5d5a | A seed's code checked by passphrase while kept only on a stick stayed Checked once put on Paper with a camera attached, although its drawn copy had not been scanned. | Put on Paper with a camera attached, a code not yet scanned goes back to Not checked until its copy is scanned; with no camera it stays Checked. |
| 2c1b52674 | The same case after the backup was opened again: the code read Checked where it should read Not checked. | Same rule; the kept backup record notes the code's state. |
| 3ccbeaee5 | A code's id for a key with a BIP-39 passphrase was the fingerprint including the passphrase, unlike Krux. | The id is the fingerprint of the words alone. |
| 44e9cc139 | A code's id from a key's words was not written as Krux writes a fingerprint, so a match was not recognised. | The id is the fingerprint as Krux writes it (8 lower-case hex characters as text). |
| 9eabb721a | The Krux backup's Scan my copy did not wait for the stick to be out, unlike every other scanner. | Scanners wait for the stick. |
| 81674fbc6 | The read card stalled before it was drawn, and its time was wrong. | The read card draws without waiting on the work, and its time is true. |
| 1450b6c00 | A passphrase carried from a note to the next seed was kept after Cancel and a lock; 16/32 bytes of text opened as a seed; a 32-byte inner code did not offer Open; the seed sheet opened with a stick plugged in. | The carried passphrase is forgotten on Cancel and lock; text of those lengths is a note; a stick plugged in stops the seed sheet with "Remove the stick first." |
| 765c9e129 | A note of exactly 16 or 32 bytes did not offer Read as a seed; 16/32 bytes with a control character showed as a garbled note. | Those byte counts offer Read as a seed unless control characters are present; the sheet shows the time after Open. |
| d4d584a61 | The sheet stayed up when a stick was plugged in and its work ran on holding a secret; Open and Scan again did not wait for the stick; a carried note outlived its code; Cancel dropped a signing or seal job. | The sheet closes when a stick arrives and the held secret is dropped; a process that handled a secret counts as used. |
| 12f84b7ea | With a stick in, Scan again on a read card worked; a note carried as a passphrase was dropped while the seed scanner was up. | Scan again waits for the stick to be out; a carried note stays for the Krux code it leads to. |
| e867840c5 | A carried note could fill the passphrase of the next, unrelated seed read; a stick plugged in and pulled over the sheet left it held; a seed's Add this key dropped it. | The carried note lives only on the seed scanner, its card and the Krux sheet it leads to; it is forgotten when the next code holds something public or is a codex32 string. |

## Doubts

- Row 93ba09294 left out: outside the inventory's scope.
- `kef_timing` and `kef_rate` survive a lock; a timed run that never answers would leave `kef_timing` true (whether the worker always answers was not read).
- `Cancelled` and every other `Refused(_)` map to the version refusal text (`faraday/faraday-core/src/worker/kef_make.rs:~215`); whether Cancel reaches it is unclear.
- `stop` does not drop an answer that arrives later; `sealed_done` stores it; whether the caller discards it is unclear.
- `WordsFingerprint` is converted outside `faraday/faraday-core/src/worker/kef_make.rs`; `MakeStart` has no constructor there.
- The comment at `faraday/faraday-core/src/worker/kef.rs:~4-5` says no screen reaches these jobs yet; the backup board and Tools do.
- Reading a Krux file is gated by `may_load_keys` while scanning is not; whether the file path is reachable otherwise is unclear.
- `Header.high`/`sure` in `faraday/faraday-core/src/seeds/kef.rs` and `KefAct::Show` in the Ask stage look unused.
- `reading_passphrases` (which passphrase forms are tried) is in faraday-keys and was not read.
- Line numbers in the form and tool marked `~` were found off by up to 40 lines where checked.
- The inventory gave no line numbers for the read-sheet rules from the hex passphrase through the file kinds in `faraday/faraday-core/src/seeds/kef.rs`; only the file is named.
