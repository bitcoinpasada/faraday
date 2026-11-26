# Checking a seed or a copy

This feature has three checks:

- **My seed makes my wallet** (`Screen::SeedCheck`, opened from Tools by `SeedAction::Open`). It works out a seed's key at an account and shows the fingerprint, xpub and first addresses. It can compare them with what a wallet app shows. The person can keep the key.
- **Check a seed against it** (`Screen::MultisigCheck`). This is the same flow with only the Seed and Passphrase cards; it is used from Check a multisig setup.
- **Check my copy** (`copy_check`). The person types back, or scans, the words or SeedQR of a seed they have copied, from the backup checklist.

All three are for the person at the offline device.

## Screens and sheets

My seed makes my wallet (`faraday/faraday-core/src/seed_check/{mod,area,compare,screen,act}.rs`)
- `Screen::SeedCheck` has four cards: Seed, Passphrase, Account, Result (`faraday/faraday-core/src/seed_check/screen.rs:~36`). The default back is `Screen::Catalog`.
- `Screen::MultisigCheck` has the Seed and Passphrase cards only. Its result is matched through `multisig_check::seed_given`.
- Seed card, ways in:
  - typed words (BIP-39 checksum);
  - a SeedQR scan;
  - a Krux code;
  - a file;
  - a loaded key (this skips Passphrase);
  - codex32, SLIP-39 or Seed XOR sets (`FORMS`);
  - Practice's sample.
- Account card:
  - five accounts: BIP-84, 86, 49 and 44, and 48 multisig (account 0 only);
  - Other path, with a script choice.
- Result card:
  - fingerprint, path, script, origin;
  - xpub, plain or SLIP-132;
  - receive and change address 0 (none for multisig);
  - a QR of the xpub or of an address.
- Compare: a typed, scanned or Files value is held against the Result. The answer is Matches, Differs or Refused. The scanner is `Sheet::Scan`.
- Keep this key.

Check my copy (`faraday/faraday-core/src/copy_check.rs`)
- Words typed back, through `forms::word_key`, with an optional passphrase.
- The SeedQR copy grid with miss counts. After `MISSES` = 3 misses, "Keep the words only" moves to the foot (`faraday/faraday-core/src/copy_check.rs:~271`; the backup screen's warning is at `faraday/faraday-core/src/screens/backup.rs:1403-1407`).
- `seedqr_side`: 25, 29, 25 or 21 (`faraday/faraday-core/src/copy_check.rs:~307-313`).

## Actions

My seed makes my wallet (`SeedAction`, `faraday/faraday-core/src/seed_check/mod.rs:~295-364`)
- Open and cards: `Open`, `File(k)`, `Step(k)` (drops the later cards), `Show(k)`.
- Seed: `Type`, `Words`, `Form(i)`, `FormDone`, `Loaded(i)`, `Spelling`, `Sample`.
- Passphrase: `NoPass`, `AddPass`, `PassField`, `UsePass`.
- Account: `Pick(Account)`, `Other`, `PathField`, `Script`, `UsePath`.
- Result: `Qr(0|1)`, `Another`, `AnotherSeed`.
- Compare: `Compare`, `CompareField`, `CompareScan`, `CompareFile`, `CompareGo`, `CompareSample`.
- Ending: `Keep` (loads the key as Add a key does, then leaves), `Done`, `Cancel`.
- Typing:
  - the path takes digits, h, ' and /, at most 60 characters;
  - the passphrase;
  - the compare field takes at most 8192 characters, control characters refused.

Check my copy
- Passphrase mode takes printable ASCII only (`faraday/faraday-core/src/copy_check.rs:~185-192`).
- Enter compares. Escape closes.
- `copy_key`: Up and Down always move; Left and Right move by block, unless in rows mode (`faraday/faraday-core/src/copy_check.rs:~317-320`).

## Effects

- The check key is held in the `faraday-keys` check slot: `check_read`, `check_reread`, `check_read_seed`, `check_set_codex32`, `check_forget`, `check_keep`.
- Compare runs as the worker job `Job::SeedCompare` (`faraday/faraday-core/src/seed_check/compare.rs:~506-509`).
- The camera: `ScanPurpose::SeedCompare`, then `Command::CameraOn`.
- Toasts.
- `refresh_spend()` runs after Keep.
- Costly work goes through `app.compute()`.
- Check my copy records a matching copy on the backup checklist.

## State and lock

My seed makes my wallet
- `SeedArea` on `app.tools.seed_check` holds `state: Option<SeedState>`.
- Secret fields:
  - `words: SecretText`;
  - `pass: PassField`;
  - shares, held in `app.entry`.
- `Made` and `Facts` hold public values only (xpub, fingerprint). `Made` is Clone, not Zeroize.
- `Compare.text` may hold a pasted key or a wallet file until it is answered. It is cleared on a refusal (`refuse_secret`, `faraday/faraday-core/src/seed_check/compare.rs:~455-472`).
- The check key itself is in `faraday-keys`. `check_forget` clears it:
  - on open;
  - in `drop_all`;
  - in `reset_cards` from Passphrase back;
  - on `Refused::Key`.
- Typed words are held only by `faraday-keys` after reading (fix 9d1dd6af2).
- Lock (`faraday/faraday-core/src/seed_check/area.rs:252-262`):
  - `holds` is typed exactly when the state holds a secret: shares, non-empty words, or a passphrase field;
  - `lock()` = `clear()`, which sets the whole state to None.
- Leaving without a press: `tidy` drops everything when the screen is not the flow (`faraday/faraday-core/src/seed_check/act.rs:~1010-1034`).
- Leaving by Done, Cancel or Keep drops the state and calls `check_forget` and `entry.reset()`. The key kept by Keep stays in the session.
- `SecretText` and `PassField` wipe on drop.

Check my copy
- `WordCheck` (`faraday/faraday-core/src/copy_check.rs:~21-46`; its `typed` is a `SecretText`, `faraday/faraday-core/src/copy_check.rs:19-24`) holds:
  - `typed`;
  - an optional `PassField`;
  - word entry, error, language and count.
- `clear_typed` wipes it (`faraday/faraday-core/src/copy_check.rs:~97-102`).
- The checklist state is the backup's, closed at lock (see the backup features).

## Reach

- Reads: `explore::{implied_script, parse_path}`, `address_find::{DEPTHS, script}`, `forms`, `entry`, `inbox::{words_of, part_form}`, `scan_key::secret_code`, `text_check`, `stick::refuses_secret`, `pass_field`, `secret_text`.
- Practice and samples: `demo::PASSPHRASE`, `samples::put`.
- Flow and drawing: `multisig_check`, `ways_in`, `pick`, `trail`, `flow`, `qr_view`, `worker`, `at_lock`.
- The seed passphrase sheet hands SeedQR words and its passphrase to `seed_check::scanned` (`faraday/faraday-core/src/seeds/read.rs:494`).
- Word entry calls `seed_check::here` and `act` (`faraday/faraday-core/src/word_entry.rs:1045-1046`).
- Check my copy is reached from the backup checklist (`faraday/faraday-core/src/backups/mod.rs`, `faraday/faraday-core/src/wallet_backup.rs`).

## Rules and refusals
- An extended key's network is Main or Test only (its version bytes); signet and testnet keys are not told apart (core/faraday-bip/src/slip132.rs:162-169).
- The seed passphrase sheet's Use without / Use with a passphrase presses are not gated by the stick (only the passphrase field and the Krux choice are); a stick arriving clears the sheet's passphrase and focus (faraday/faraday-core/src/seeds/read.rs:354-372, 469-491; seed_check/act.rs:642).

My seed makes my wallet
- Refused while `stick::refuses_secret` holds (`faraday/faraday-core/src/seed_check/act.rs:48, 216, 225, 237`):
  - Open from a Files file;
  - AddPass and PassField;
  - UsePass with a non-empty passphrase.
- `scanned` (SeedQR words plus the sheet's passphrase, `faraday/faraday-core/src/seed_check/act.rs:642-678`) has no `refuses_secret` check of its own.
- Keep (`faraday/faraday-core/src/seed_check/act.rs:807-811`):
  - refused unless `may_load_keys()`;
  - refused when the key is already loaded;
  - refused without a Result;
  - styled Disabled with `read_need_no_stick`.
- A card is reachable only if the one before it is filled (`reach`, `faraday/faraday-core/src/seed_check/act.rs:~326-332`). Passphrase opens only with typed words or shares held.
- A seed that is not BIP-39 gives `seed_check_not_seed`, as a card error or a toast.
- A Krux code is refused as a part; it is read only by the scanner or from Files.
- Sets:
  - codex32: fewer strings than k are completed from open vaults' strings;
  - SLIP-39: parts wait for the passphrase;
  - Seed XOR: needs combinable parts.
- `Refused::Path` is shown on the card.
- `Refused::Key` restarts the flow. Its causes: a missing key, a path longer than any wallet's, or a `from_xpub` failure.
- Compare refuses, through `text_check` (`faraday/faraday-core/src/seed_check/compare.rs:432-447`; `TooLong` comes before any secret scan, `faraday/faraday-core/src/text_check.rs:50-52`):
  - secrets and private keys;
  - over-long text (`MAX_TEXT`);
  - partial secrets;
  - seed hex.
- Compare, other rules:
  - A key is refused when it has no origin, when it is a bare SLIP-132 key, or when it is of the wrong network.
  - An origin at another path, or without the passphrase, gives Differs, with the path named.
  - An address is searched over the first `DEPTHS[0]` indices of both chains.
  - A multisig cosigner account is refused for an address.
  - A wallet file with a read error is refused; one with a key of the wrong network is refused.
  - The answer is dropped if the Result's xpub or path, or the field text, changed (`done`).
  - The scanner takes only the `SeedCompare` purpose. Non-UTF-8 is refused.
- Coin type 0 is mainnet and 1 the test networks. Testnet has its own legacy start text. A `network_kind` that is not Main is treated as test.

Check my copy
- `copy_indices` (`faraday/faraday-core/src/copy_check.rs:338-357`):
  - Input made only of ASCII digits must be exactly 48 or 96 digits, else None.
  - Each 4-digit group must be below 2048.
  - Other input is read as a Compact SeedQR (16 or 32 bytes) through `from_compact_seedqr`.
  - It is read without the checksum, so one wrong word still reads.
  - A 16- or 32-byte compact code made only of digit bytes returns None at the length test.
- The check field takes at most four digits per seed word. Checks stop at the seed's paper copies (fix 450d7f150).
- A copy is checked in its own form: words typed back, or SeedQR scanned or typed. A seed copied both ways needs both (fix e38cf0e17).
- A scan of another whole seed's SeedQR says "This is another seed's SeedQR". The comparison shows neither seed's words (fix e09382590).

## Domain calls

- `faraday-bip`: `slip132`, `WalletPolicy`, `PolicyKey`, `bip39`, `Language`.
- `faraday-keys`: `check_*`, `words::from_compact_seedqr`.
- `faraday-intake::wallet::read_wallet`.
- `faraday-crypto::work::Compute`.
- `faraday-shell-api`.
- `faraday-strings::bip`.
- `zeroize`.

## Fixtures

None. Practice uses `demo::PASSPHRASE` and the sample seed from `faraday/faraday-samples`, which are not fixture files.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 450d7f150 | The backup checklist's copy check accepted digits past the seed's length. After a matching copy, a retyped copy cost about 331 ms per frame (against 4.7 ms). | The check field takes at most four digits per seed word. Checks stop at the seed's paper copies. |
| e38cf0e17 | On a words-only plan with a camera, Check my copy opened the camera for a SeedQR scan. | A copy is checked in its own form (words typed back, SeedQR scanned or typed). Both forms are needed for a seed copied both ways. |
| e09382590 | A copy check's scan of another seed's SeedQR (a whole BIP-39 seed of its own) said "Your copy scans but word 1 differs". | The message says "This is another seed's SeedQR". The comparison does not reveal either seed's words. |
| 9d1dd6af2 | My seed makes my wallet kept the typed words after reading. The passphrase could not be taken again from the key check. Lock did not clear them. | Typed words are held only by faraday-keys. The passphrase can be taken again. Lock clears them. |
| 48689014d | My seed makes my wallet kept waiting shares and parts at the ways into its Seed card. It did not read a codex32 set finished with an open vault's strings as Add a key does. | Waiting shares are dropped at every way in. A codex32 set reads as Add a key reads it. Typed strings are held until Keep or leaving. |
| 7c45ea8d2 | Shares were lost through Back then Forward. Keep this key on a codex32 set lost the made-online mark, the share facts and both strings. | Shares and kept strings survive Back and Forward and the strings handoff. |

## Doubts

- Three rows came to this feature from Restore's history group, where they had been relabelled to My seed makes my wallet: 9d1dd6af2, 48689014d and 7c45ea8d2.
- `faraday/faraday-core/src/copy_check.rs:~240-243`: `wrong(k, ..)` with k = 0 would underflow. Its callers were not checked.
- `still_fits` always returns false (`faraday/faraday-core/src/seed_check/act.rs:~1125`). It looks dead.
- A comment problem was noted at `faraday/faraday-core/src/copy_check.rs:~189` (cosmetic).
