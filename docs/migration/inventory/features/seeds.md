# Seeds to a wallet

The seeds piece turns typed or loaded seeds into a wallet. It covers three
parts that share the `SeedsArea` state. The seeds piece itself serves
Restore's seeds-first route, the Spend tab's words route, Make a new wallet's
slot pick and BIP-85. The seed passphrase sheet (`Sheet::SeedRead`) opens
after a seed is read anywhere: Home Scan, Add a key, the Create and Restore
slots, the Spend tab, a wallet's key scanner, or Check the money. It asks
whether the seed loads with or without a passphrase. The Sweep sheet
(`Sheet::Sweep`) sweeps the coins of one private key (WIF). It is opened
from the Tools tile, from Home Scan on a WIF, or from a read card. All of it
is used by the person at the offline device.

## Screens and sheets

Seeds piece (`faraday/faraday-core/src/seeds/mod.rs`)
- No screen of its own. It is drawn inside `Screen::Restore`, `Screen::SpendBackup`, `Screen::Create` (the slot pick) and `Screen::Bip85`.
- It shows:
  - the wallet's shape: kind, N keys, M of N (no M slider for MuSig2), account and path;
  - one box per key, holding a seed or a cosigner's account key;
  - the loaded keys, with ticks;
  - Sample seeds in Practice;
  - the error line of the descriptor being built.

Seed passphrase sheet (`faraday/faraday-core/src/seeds/read.rs`, `faraday/faraday-core/src/seeds/screen.rs`)
- `Sheet::SeedRead` offers these choices:
  - no passphrase (`Plain`);
  - a passphrase typed twice (`AddPass`, then `Field(0|1)`, then `WithPass`);
  - "Not this one" (`Skip`);
  - Seed XOR form (`Xor`);
  - Krux (`Kef`);
  - Cancel.
- The Spend tab opens it in one of four modes: `ForSpendBackup::{Intake, Guess, Replace, Stray}`.
- In Practice, the "faraday" chip fills both fields with `demo::PASSPHRASE`.

Sweep (`faraday/faraday-core/src/seeds/sweep.rs`, `faraday/faraday-core/src/seeds/read.rs`)
- `Sheet::Sweep` steps:
  1. A field for the WIF, or the scanner (`Sheet::Scan` with `ScanPurpose::Sweep`).
  2. The WIF's addresses, with QR codes.
  3. A review of a PSBT: the inputs that spend the key, the outputs, and the warnings.
  4. A hold to sign.
  5. The signed PSBT or the final transaction hex, shown as QR.

## Actions

Seeds piece (`SeedsAction`)
- Shape and keys: `Shape`, `Keys`, `Kind`, `Account`, `Standard`, `Custom`.
- Boxes: `Focus`, `Open`, `Clear`.
- Loaded keys: `Tick`, `UseTicked`, `UseLoaded`.
- Other sources: `UseFile` (an Inbox file as an account key), `FromVault`, `Sample`.
- `Make`.
- Sliders and arrow keys also change the shape.
- Typing goes into the cosigner boxes (`seeds_key`), only while shaping.
- Make (`seeds_make`, `faraday/faraday-core/src/seeds/mod.rs:~939-1003`):
  - It names the wallet "Typed seed(s)" plus a number, unique.
  - The origin is `Origin::TypedSeeds`.
  - On Restore with a described wallet, it goes to the Check card.
  - On the Spend tab, a `Refusal::Unconfirmed` holds (the hold runs Make again). Other refusals show on the Spend seed sheet.

Seed passphrase sheet
- `Plain` loads the seed with no passphrase.
- `WithPass` loads it with the passphrase typed twice.
- `Skip` passes over this seed.
- `Xor` opens the Seed XOR form.
- `Kef` opens the Krux code.
- Cancel closes the sheet.
- A seed that is not a BIP-39 seed goes to Add a key as typed words.
- A Restore refusal keeps the sheet open, with the error.

Sweep (`SweepAction`)
- `Open`; `Field` (typing); `Use` (reads the typed WIF); `Scan`; `Back`; `Hold` (signs); `ShowQr`; `Done` (erases the key and the sweep).

## Effects

- The camera goes off after a read.
- Key derivation runs through `key_of` / `with_key_now`: PBKDF2 and EC multiplication, on the draw-free path, through `Compute`.
- Keys are added to the session (`session.keys`).
- The sheet sets the typing timer (`inbox.set_typed_at`).
- Make adds a wallet to the session.
- A vault seed can start the vault's load job (`vault::load_job::start_replay`).
- Sweep writes to the Outbox:
  - `sweep-signed.psbt`;
  - `sweep-final.txn`, only when every input is signed.

## State and lock
- The Krux read's timed run: `kef_timing` is true only while one is out; its answer, also a stopped run's, always arrives and clears it, and a rate is kept only for a run that finished with a nonzero time. The flag and the rate (a time in milliseconds) stay across a lock (faraday/faraday-core/src/seeds/mod.rs:493-509; seeds/kef.rs:261-267, 283-286; worker/kef.rs:78-98).

- `SeedsArea` (`faraday/faraday-core/src/seeds/mod.rs:~404-416`) holds:
  - `state` (the `SeedRead` sheet);
  - `sweep`;
  - `kef`, `kef_rate`, `kef_timing`;
  - `carried_pass: Option<SecretText>`.
- `SeedsState` holds no seed material: fingerprints (`keys`), ticks, shape, focus, and `cosigners: Vec<Text>` (account-key text). The seeds themselves live in `session.keys`.
- `SeedRead` (`faraday/faraday-core/src/seeds/read.rs:~131-172`) holds:
  - `source`: `Words(SecretText)` or `Loaded(fp)`;
  - `pass: PassField`;
  - flags and `kef`.
- The words wait in memory, never drawn, until a choice loads them or the sheet closes.
- `seed_read_stick_arrived` (`faraday/faraday-core/src/seeds/read.rs:~354`) clears the passphrase when a stick is in the way.
- `Sweep` holds `typed: SecretText`, the review and the signed result. The review is the PSBT as read with address, amount, fee and warning texts (`faraday/faraday-core/src/seeds/sweep.rs:88-111`); the signed result is the signed PSBT and finished hex also put in the Outbox (`faraday/faraday-core/src/seeds/sweep.rs:114-121`, `faraday/faraday-core/src/seeds/read.rs:977-992`): public, since the signer adds only partial signatures (`core/faraday-psbt/src/sweep.rs:137-138`). The key itself is `Keys::sweep` in `session.keys`.
- `sweep_tidy` (`faraday/faraday-core/src/seeds/read.rs:732-740`) runs after every press (`faraday/faraday-core/src/action.rs:1294`). It erases the key and the sweep unless the sheet or the sweep scanner is up. Done also erases them.
- Lock: `AtLock for SeedsArea` (`faraday/faraday-core/src/seeds/mod.rs:1086-1094`).
  - `holds` is typed exactly when `holds_secret()`.
  - `lock()` calls `clear()`, which drops the seed sheet state, the Krux read and the carried passphrase.
  - It then calls `clear_sweep()`, which sets `sweep = None` (`faraday/faraday-core/src/seeds/mod.rs:455-464`).
  - `kef_rate` and `kef_timing` are kept.
- The typed seeds and the sweep key in `session.keys` are wiped by `Session::wipe` (`faraday/faraday-core/src/wallet.rs:1539`). `Keys::remove_all` also sets the sweep key to None (`faraday/faraday-keys/src/session.rs:1272-1278`).
- The lock ends in a restart (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- `Text`, `SecretText` and `PassField` wipe what they hold on drop.

## Reach

- Opened from:
  - Restore, Spend from a backup and Make a new wallet (the seeds piece);
  - `scan_key` (`seed_read_open` and `seed_read_for_wallet`, `faraday/faraday-core/src/scan_key.rs:558, 2616`), Add a key and the wallet key scanner (the passphrase sheet);
  - the Tools tile and Home Scan (Sweep).
- Reads or opens: `create`, `wallet`, `spend_backup`, `restore`, `vault`, `bip85`, `seed_check` (`here`, `scanned`), `stick`, `compact::dialog`, `qr_view`, `demo`, `at_lock`.
- The passphrase sheet hands typed words to My seed makes my wallet (`crate::seed_check::scanned`, `faraday/faraday-core/src/seeds/read.rs:494`). See seed-checks.
- Krux codes: the `Kef` choice and `SeedsArea.kef` (see krux-codes).

## Rules and refusals

Seeds piece
- At most 15 keys (`MAX_KEYS`, `faraday/faraday-core/src/seeds/mod.rs:36`).
- `set_n` clamps to `kind.max_keys().min(MAX_KEYS)`, and never below the seeds already held (`faraday/faraday-core/src/seeds/mod.rs:270-275`).
- M is clamped to 1..=N. The default is a majority. MuSig2 has no M slider.
- When the kind is fixed first, each key takes an empty slot, and `take` returns false when there is none (`faraday/faraday-core/src/seeds/mod.rs:168-176`).
- The same seed given twice changes nothing.
- A single key of a multi-key kind is made as NativeSegwit, without notice (`faraday/faraday-core/src/seeds/mod.rs:~713-717`).
- The account is 0..=0x7fff_ffff. Changing the account clears a custom path. A custom path must be a normal derivation.
- Cosigner box typing (`faraday/faraday-core/src/seeds/mod.rs:1005-1050`):
  - only while shaping;
  - control characters refused;
  - at most 400 characters per box.
- Descriptor errors, in this order (`seeds_descriptor`, `faraday/faraday-core/src/seeds/mod.rs:~651-721`):
  1. no seed yet;
  2. seed not loaded;
  3. an empty cosigner box ("key has no account");
  4. text that is not an account key;
  5. a key of the other network (named);
  6. a key count not equal to N.
- Cosigner text:
  - whitespace is stripped;
  - the tails `/<0;1>/*`, `/0/*` and `/*` are tolerated;
  - SLIP-132 forms are accepted only where the script agrees with the path purpose.
- An Inbox file is taken as an account key only for key kinds.
- From a vault:
  - only KEY records;
  - vault keys already loaded are left out;
  - a seed that asks for a passphrase goes to the passphrase sheet.

Seed passphrase sheet
- A press on a passphrase field with a stick in the way is refused with the `keys_load` toast.
- `WithPass` needs both fields equal (`pass_pair_differ`) and not empty (`seed_read_type_passphrase`).
- Passphrase characters are BIP-39 printable ASCII.
- `Xor` is offered only on plain sheets with 12, 18 or 24 words, and only from `Source::Words`. `Kef` needs `may_load_keys()`.
- No passphrase is asked for a loaded key without words, or one that already has a passphrase.
- The wallet key scanner (`seed_read_wallet`) adds only a missing key of that wallet:
  - a key already here gives `seed_read_key_already`;
  - a key not in the wallet gives `not_a_wallet_key`.
- A Duplicate is accepted as the existing key.
- A set of seeds loaded from the unlock's Load sheet is added only to a session on the same network. `adopt_loaded` refuses the whole set otherwise (`faraday/faraday-keys/src/session.rs:~843`).

Sweep
- Refused in Practice: `SweepAction::Open` toasts `practice_refused` when `!makes_secrets()` (`faraday/faraday-core/src/seeds/read.rs:754-758`).
- Refusals of the WIF:
  - `wif_not_a_key`;
  - `sweep_mainnet_key`;
  - `sweep_test_key`;
  - a stick attached (`keys_load`).
- The typed field takes ASCII letters and digits only.
- The typed WIF is taken out (`mem::take`) before it is read, so it is lost on a refusal.
- A WIF scanned anywhere but Home or the sweep scanner is refused (`sweep_elsewhere`).
- On the sweep scanner, only a PSBT is taken (`sweep_not_psbt`).
- The review stops when:
  - no input spends the key (`sweep_no_inputs`); or
  - a warning is blocked or not holdable. The text shown is the first shown warning (`faraday/faraday-core/src/seeds/sweep.rs:253-265`), which may not be the one that blocks.
- When the review stops, Hold is hidden and Sign is refused.
- A v2 PSBT is never written as v0 (`spend_v2_not_written`).
- The warnings shown are only Danger and Blocked (and NoParticipatingKey).
- Inputs that do not spend the key are left unsigned.
- An uncompressed WIF is warned about.

## Domain calls

- `faraday_bip::{slip132::key_line, policy::PolicyKey, keys, bip39}`; `faraday_bip` Address and `policy` `find_address` (Sweep).
- `faraday_vault::records::kind::KEY`.
- `faraday_keys::{Keys, kef::Envelope}`; `Keys::sweep_*`; `faraday_keys::identity::is_wif`; `faraday_keys::sweep::Refusal`; `faraday_keys::session` `adopt_loaded` (`faraday/faraday-keys/src/session.rs`).
- `faraday_codec::kef::id_text`.
- `faraday_psbt`: Inspection, finalize, verify, v2.
- `faraday_intake::wallet::network_name`.
- `faraday_ui::{parts::qr_code, hold}`.
- `faraday_crypto::work::Compute`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 146bf6c86 | Seeds loaded from the unlock's Load sheet could add keys worked out on one network (for example testnet) to a session on another network. | A key set worked out on one network is added only to a session on that network. `adopt_loaded` refuses the whole set otherwise. |
| d512ef221 | Comparisons that decide access (passphrase match, SeedQR read check, SLIP-39 digest check, same-words check) were not constant time. | Comparisons that decide access run in constant time; outputs unchanged. |

## Doubts

- Fix rows left out as not about this feature:
  - 33cb48378 and bd9e630be (Seed XOR and SLIP-39 counting in the backup plan, `faraday/faraday-plan`);
  - 32c60b9a2 (a vault's stick write of seed words).
- One row given to this feature (1f6eb9aad) is outside the inventory's scope and is not carried.
- Several line numbers are unconfirmed: the Make path, the `SeedsArea` and `SeedRead` field lists, the silent NativeSegwit fallback and the descriptor error order.
