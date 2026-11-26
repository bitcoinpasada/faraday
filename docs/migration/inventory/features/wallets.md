# Wallets in the session

The session holds the keys and wallets loaded on the device, in memory only. This feature covers:

- the session itself (`Session`, `faraday/faraday-core/src/wallet.rs`): how a wallet is added, deduplicated, refused or held back for its confirmation sheet;
- the Wallets tab (`Screen::Wallets`, `Screen::WalletsStart`, and the wide pair tile), with its wallet cards, loose keys and offers;
- the wallet chart (`Sheet::Chart`, `faraday/faraday-core/src/chart_edit.rs`): notes, checks and moves for a wallet's backup;
- wallet names and renaming;
- wallet identity (`WalletId`);
- the per-key checks on the confirmation and register sheets;
- the account choice for a two-account export.

It is used by the person at the offline device. Every other flow that loads or makes a wallet ends here.

## Screens and sheets

Wallets tab (`faraday/faraday-core/src/screens/wallets.rs`, `faraday/faraday-core/src/screens/wallets/actions.rs`, `faraday/faraday-core/src/screens/wallets_start.rs`, `faraday/faraday-core/src/screens/wallets_pair.rs`)
- `Screen::Wallets` is the list of wallets plus the card of the chosen wallet. The card shows:
  - name, source, kind;
  - the first receive address;
  - who can spend;
  - missing keys;
  - the lead banner of a spend;
  - "What you can do" (the offers).
- Loose keys are the loaded keys in no wallet (`loose_card`).
- The silent payments card has the address, Show, Remove and a key Load or Add.
- `Screen::WalletsStart` offers the ways in:
  - Make a new wallet;
  - Restore an existing wallet;
  - Spend from a backup;
  - Add a key;
  - Make a new key.
- The start page also shows wallets two to a row, at most two rows, then "more".
- The wide pair tile (`wallets_pair`) is the list and the card side by side.

Sheets (drawn at `faraday/faraday-core/src/screens/mod.rs:481-495, 539-559`)
- `Sheet::WalletConfirm` (`faraday/faraday-core/src/wallet/confirm.rs`), opened at `faraday/faraday-core/src/wallet.rs:4609-4613`. It shows:
  - per-key rows (number, fingerprint, path, whole xpub, state);
  - a count line;
  - the checksum;
  - the key-path warning;
  - a receive-address caution.
- `Sheet::Register`: the same key rows, for a registered wallet.
- `Sheet::Account` (`faraday/faraday-core/src/wallet/account.rs`), opened at `faraday/faraday-core/src/silent/mod.rs:978-982`. For a Coldcard export holding both a single-sig and a silent payments account, the person chooses one or cancels.
- `Sheet::Published`: words published or made online, with Remove and Keep.
- `Sheet::RemoveKeys`, `Sheet::MadeWallet`, `Sheet::Pull`.
- `Sheet::Chart` (`faraday/faraday-core/src/chart_edit.rs`): the wallet's backup chart. It holds the holder and vault names, the lost and exposed marks, checks, the passphrase check, "move money to a new wallet", envelope PDFs and destroying a part.

## Actions

Wallets tab
- Rows:
  - `PickWallet`, `OpenWallet`;
  - a loose key: `PickKey(fp)`, `KeyWallet(fp,1)` (From key), `KeyWallet(fp,2)` (multisig);
  - `RestoreWallet`;
  - `Entry(None)` or `ScanKey(wallet)`.
- Renaming: `Rename`, a typed field (`faraday/faraday-core/src/screens/wallets.rs:~606-639`).
- `NewWalletFrom(w)`.
- Lead banner: `Nav(Screen::Spend)` / `StartSpend(i)`.
- `ShowCode(Code::Receive(w,0))`; `OpenSetTime`; `Chart(ChartAction::Moved(Press::Loaded(wi)))`; `BackupsOf(i)`; `Vault(OpenFrom|Open|LoadKeyOf)`.
- Offers (`faraday/faraday-core/src/screens/wallets/actions.rs`):
  - spend (`spend_button`);
  - Back up / Continue backup (`Backup(w)`);
  - Check an address (`AddressCheck(Another)`);
  - Sign a message (`SignMessageWith`);
  - Show wallet (`QrWallet`), Show receive code;
  - New wallet from this one;
  - Explore its keys (`ExploreKey(fp)`);
  - Remove from session (`RemoveWallet(w)`). It is always last, has the danger tone, no chevron, and acts at once.
- Silent card: Address `SWallet(w)`; Show `QrWallet`, hidden when held; Remove `RemoveWallet`; key Load `Vault(LoadKeyOf(fp, SlotOf::Bare))` or Add `Entry(Some(fp))`.

Confirmation and register sheets
- `confirm_act` (`faraday/faraday-core/src/wallet.rs:~4622-4650`): only `WalletHold` and `WalletCancel` act.
- Hold runs `confirm_held` (`faraday/faraday-core/src/wallet.rs:~4653-4709`):
  - "Add to vault" runs the vault branch (`vault_add_held`);
  - otherwise `session.add_policy`, and an error becomes a toast.
- Cancel runs `session.wallet_drop()`.
- `Action::KeyTick(KeyBlockOf, u8)` toggles a key's tick. Sites are `KeyBlockOf::{Asked, Register, Wallet(u16), Check}` (`key_tick`, `faraday/faraday-core/src/wallet.rs:~4723-4752`).

Published sheet (`published_act`, `faraday/faraday-core/src/wallet.rs:~4572-4604`)
- `PublishedRemove` drops the key with its spend and typed wallets (`remove_published`, `drop_typed_with`).
- `PublishedKeep` keeps it for testing.

Account sheet
- `Action::LoadAccount(Option<Account>)`.

Chart (`ChartAction`, `faraday/faraday-core/src/chart_edit.rs`)
- Names: set the key holder or vault name (`Target::Holder`, `VaultName`).
- Marks: mark lost or exposed.
- Checks:
  - check one thing, or everything, of a backup node;
  - check a holder's backup today;
  - mark coins moved today.
- Passphrase check: `Target::PassCheck`.
- Move money to a new wallet: goes to `Action::CreateWallet` (`faraday/faraday-core/src/chart_edit.rs:~603-617`).
- Envelope PDFs: save them per place (`faraday/faraday-core/src/chart_edit.rs:~695-729`).
- Destroy a part: a Seed XOR or SLIP-39 part on the chart (`faraday/faraday-core/src/chart_edit.rs:~294-346`).
- Keys:
  - characters, Backspace and Delete;
  - Enter saves (`ChartAction::Save` / `PassTry`);
  - Escape closes, or cancels a running job (`faraday/faraday-core/src/chart_edit.rs:~629-676`).

## Effects

- Adding a wallet: `add_wallet` (`faraday/faraday-core/src/wallet.rs:~925`), `add_wallet_kept` (`~939`), `add_policy` (`~1144`), `add_kept` (`~1163`), `add_checked` (`~1210`).
- `load_wallet` (`faraday/faraday-core/src/wallet.rs:~5223-5246`):
  - takes the name from the file, `carried_name` first;
  - `Unconfirmed` holds for its sheet;
  - other errors become a toast.
- Wallet key files:
  - `wallet_key_out` (`faraday/faraday-core/src/wallet.rs:~5602-5615`) writes `# Account xpub {fp}, {kind}`;
  - `wallet_key_bsms` (`~5617-5630`) writes `xpub-<fp>-bsms.txt`.
- The tab itself only draws: no storage, camera, clipboard or time. It scrolls through `ui.region_open` with `app.wallets.card_offset()`.
- Chart:
  - edits vault records in memory (`vaults.edit`, `plan_store`);
  - rewrites backup records (`backups_area.set_records`);
  - writes PDFs to the Outbox (`put_outbox`);
  - reads the clock.
- `wallet_left` (`faraday/faraday-core/src/wallet.rs:~4903-5014`): when a wallet leaves, every flow that keeps it by place is moved or ended. These are restore, backup, create, vault, silent, the KEF tool, the camera (CameraOff), devices, the made-wallet sheet and import.
- `Check` Continue marks `mark_checked(id)` (`faraday/faraday-core/src/wallet.rs:~5352`). Per its comment (`~5348-5350`), this survives a lock, through `memory`.

## State and lock

- `Session` (`faraday/faraday-core/src/wallet.rs:~682-738`) holds:
  - `keys: faraday_keys::Keys`, all key material, which leaves only through faraday-keys reveals;
  - `wallets: Vec<Wallet>`;
  - `modes`, `key_marks`;
  - `wallet_asked: VecDeque<WalletAsk>`, with `wallet_asked_past` and `wallet_asked_last`;
  - `waiting: Vec<Fingerprint>`, with `waiting_inputs`;
  - `psbt_wallets`, with `psbt_wallets_inputs`;
  - `wallet_covered`;
  - `wallet_confirmed: Vec<Text>`, the descriptors confirmed this session;
  - `accounts`;
  - `key_xpubs: RefCell<Vec<KeyXpub>>`;
  - `silent_handed`.
- `Wallet` (`faraday/faraday-core/src/wallet.rs:~182-219`) holds:
  - name, private policy, source, origin;
  - caches: `addresses` (cap 256), `scripts` (cap 256), `same`, `checked`, `waited`, `key_checks`, `waits`.
  - All of it is public data: xpubs, descriptors, addresses. No secret is on `Wallet`.
- `job_copy` (`faraday/faraday-core/src/wallet.rs:~350-364`) is a copy for a worker, with `checked` and `waited` emptied.
- `WalletsArea` (`faraday/faraday-core/src/wallet/state.rs`) holds the chosen wallet, the renaming field, the card offset and the spend.
- The chart's `Work` (`faraday/faraday-core/src/chart_edit.rs:~16-32`) holds:
  - `text` (a name);
  - `pass: PassField` (a typed passphrase);
  - result, refused, moving, moving_to.
  - It is kept in glance.
- Lock, Session (`faraday/faraday-core/src/wallet.rs:5568-5583`):
  - `holds()` = `keys.holds_secret()` as kept; session = wallets or `silent_handed` not empty.
  - `lock()` calls `wipe()` (`faraday/faraday-core/src/wallet.rs:1539-1549`). It clears `keys` (`keys.clear()`), `wallets`, `wallet_asked` and its counters, `wallet_covered`, `wallet_confirmed`, `key_xpubs` and `silent_handed`.
  - Not cleared: `waiting`, `waiting_inputs`, `psbt_wallets`, `psbt_wallets_inputs`, `key_marks`, `accounts`, `modes` (fingerprints, indices and display marks).
- Lock, Wallets area: `WalletsArea::lock` calls `clear_spend()` and `row_open.lock()` (`faraday/faraday-core/src/wallet/state.rs:1264-1274`).
- Lock, chart: the glance lock clears `work.pass` only (`faraday/faraday-core/src/glance/mod.rs:2487-2494`). `work.text`, `result`, `refused` and `moving_to` stay.
- The chart sheet closes on any other press (`faraday/faraday-core/src/action.rs:1430-1442`). A sheet's own field presses, the keyboard's included, and tick presses stay in the sheet (fixes f36ccb21d, 278d951d8).
- Every lock ends in a restart and Exit (`faraday/faraday-core/src/vault/seal.rs:933-946`), so whatever is not wiped is gone with the process.
- The secrets this feature holds are the session's keys, wiped at lock, and the chart's typed passphrase, wiped at lock.
- `Text` and `PassField` wipe on drop.

## Reach

- Opens:
  - Spend;
  - Back up and Continue backup;
  - Check an address;
  - Sign a message;
  - Explore;
  - New wallet from this one (Create);
  - Restore;
  - Add a key and Make a new key;
  - vaults;
  - Set the time;
  - Backups;
  - the codex32 correction;
  - QR view;
  - silent payments.
- Opened from: Home, the session pane, and every flow that makes or loads a wallet.
- Reads, in code: `crate::{account, confirm, keycheck, state (SpendSteps, WalletsArea, SPEND_STEPS), nonce, silent, register, codex32_correct::KeyMarks, modes, samples, memory::nonces, work, scan_read, text_check, create (NewKind, path_spec), spend_backup, seeds, cstep, core_wallet::tool, multisig_check}`.
- Reads, in the upkeep code: `signing::{read, collect}`, `decode::check_others`, `review::sign_error_text`, `wallet_backup::kef::dropped`, `seeds::kef`, `visit`, `glance`, `memory`.
- The tab draws with `drawing::View`, `panes::screen`, `compact`, `menu`, `row_open`, `guide`, `vault::{VaultAction, SlotOf}`, `silent::{RecordCheck, differs_line}` and `address_check::AddressPress`.
- `set_demo` (`faraday/faraday-core/src/wallet.rs:1533`) puts in the public sample set.

## Rules and refusals

Adding a wallet (Session)
- A secret-looking name (`safe_name`, `faraday/faraday-core/src/wallet.rs:96-120`):
  - The rule applies to a name, or its one-line form, of 26 bytes or more whose `text_check::check` is `Secret` (except `CompactSeedQr`), `Inside` or `TooLong`.
  - Such a name is replaced by the default name, without notice. It is not refused.
  - An empty name after `wallet_name` also becomes the default.
  - The name is checked whole for secrets before it is cut (fix e6a94955a).
- A wallet name is one line of text wherever it is read (fix 62851fc36).
- `Refusal::Network { offered, loaded }` (`follow`, `faraday/faraday-core/src/wallet.rs:896-911`):
  - A wallet or PSBT of another network kind moves the session only if the session is empty (no keys, no wallets), not demo (`keys.demo()`), and the mode is offered (`modes.contains(Mode::of(n))`).
  - Otherwise it is refused, naming both networks.
  - Same kind: no move.
- A PSBT's network (`faraday/faraday-core/src/wallet.rs:~803-839`):
  - It comes from its global xpubs, or from the BIP-44, 48, 49, 84 or 86 coin type (0 mainnet, else test).
  - `psbt_network` returns None if neither is there (`~840`).
- `Refusal::NotYourKey(fp)`: a key under a loaded fingerprint is not what that master gives at its origin (`forged_key`, `faraday/faraday-core/src/wallet.rs:~991-1038`).
- `Refusal::NotBackedUp(fp)`: a key under an unloaded fingerprint equals the xpub of a key waiting for backup. This applies only with `see_waiting` (`~1008-1019`). Only the origin path is compared (fix be1213326).
- `drop_forged` (`faraday/faraday-core/src/wallet.rs:~1060-1123`) removes such wallets when a key loads or starts to wait.
- A bare xpub without its origin is refused as a wallet key everywhere (fix 89aa14678).
- Silent payments: `Refusal::SilentOtherKey` and `SilentDiffers`, through `silent_refusal` (`faraday/faraday-core/src/wallet.rs:~1269-1274`, `~2372-2384`) and `check_record` `RecordCheck::Differs`.
- `musig_one_seed` (`faraday/faraday-core/src/wallet.rs:845-855`) gives `Refusal::MuSigOneSeed` (`~1275-1277`) wherever a wallet is added. The case: a MuSig2 wallet with two keys of one master fingerprint, loaded or not. The message: "Two keys of this wallet come from one seed. This device cannot sign for both."
- `Refusal::Unconfirmed` (`faraday/faraday-core/src/wallet.rs:~1243-1246`):
  - The case: a multi-key wallet with a key that is not a loaded key's own, or whose key path another key spends alone (`KeyPath::Other`).
  - The wallet is held until its sheet is confirmed (`held_for_sheet`, `~1310-1316`).
  - `key_path_line` shows a danger line (`~1499-1523`).
  - A foreign tr() key path is a warning, not a refusal (fix eecf2bfae).
- `add_to_vault_refusal` (`faraday/faraday-core/src/wallet.rs:~1292-1300`) is the same as `refused_before_sheet` with `see_waiting` false.
- Dedupe:
  - The same descriptor returns the existing place; a higher silent label count takes over (`faraday/faraday-core/src/wallet.rs:~1226-1241`).
  - The same descriptor with its vault already waiting is not asked again (`~1338-1345`).
  - A sortedmulti wallet is one wallet whatever its key order (fix e45fa284d).
- Replay after a hold (`replays`, `faraday/faraday-core/src/wallet.rs:~657-673`):
  - Only these presses run again: `LoadWallet`, `TTake`, `RUse`, `RRebuild`, `RestoreShares`, `RestoreShareAt`, `Seeds(Make)`, `CNext(KEYS)`, `SpendBackup(UseWallet|TakeFile|Rebuild|GuessWay)`, `CoreWallet(Take)`, `MultisigCheck(Add)`.
  - Other presses only get the wallet added.
- The press that asked is replayed only if `inbox_print()` is unchanged (`faraday/faraday-core/src/wallet.rs:~4670-4672`).
- `wallet_confirmed` is recorded only if no vault took the wallet (`faraday/faraday-core/src/wallet.rs:~1447-1449`).
- Caps:
  - scripts and addresses are capped at 256 (`faraday/faraday-core/src/wallet.rs:~432-434`, `~509-511`);
  - `derives` refuses a path that is no place of the wallet (`~453-455`);
  - `take_kept` takes only for the same wallet id (`~371-373`).
- Waits (`Waits::of`, `faraday/faraday-core/src/wallet.rs:252-268`):
  - Only `Lock::Blocks` (×600 s, days = n/144, margin 90%) and `Lock::Intervals` (×512 s) count.
  - Other locks are skipped without notice.
- `spend_ways` (`faraday/faraday-core/src/wallet.rs:~1664-1713`):
  - MuSig2 needs every seed.
  - A path through a key with no seed, or past 64 seeds (`i < 64`, `faraday/faraday-core/src/wallet.rs:1696`), cannot be met by seeds.
  - A preimage-lock path is not a seed way.
  - `stated_quorum` is None for MuSig2, for miniscript, and for a tree with several ways, unless a key-path-other leaf quorum exists (`~1718-1724`).
  - The seeds a spend needs are the stated quorum, else the fewest seeds of any spend (fixes 61fcca48d, c1c88d6e4).
- Wallet identity is the SHA-256 of the whole descriptor (`WalletId`), not the 8-character checksum. Every lookup, kept place and share record goes by id (fixes aba412739, 6ccc7839b, 5ccbb1659, 054836b32, 8096cf7af, 6440042e0).
- When a wallet is removed, the wallet list is marked changed until the next frame. A tick press for a place is then dropped (fixes 68a0eff42, c239c5105). An open spend follows its own wallet and closes when that wallet is gone (fix a141eebb6).

Wallet files read (through `faraday_intake::wallet`)
- A file with several descriptors loads in the documented order: multisig, miniscript or MuSig2, then BIP-84, 86, 49, 44. Two different wallets of the best kind refuse the file (fix 7e6b2eaff).
- A silent payments wallet beside another wallet refuses the file (fix fe411fdfe).
- Refused:
  - a repeated key;
  - mixed networks;
  - a bad header or key;
  - a BSMS address of another network kind (fix 3e7ce0ba9).
- A SLIP-132 version naming another script than the path's purpose is refused and named, never converted (fixes 1590a5825, b0010427e).
- A BSMS first address is compared as an address. `/**` with "No path restrictions" reads as `/0/*,/1/*` (fix d89080ffc).
- A secret key text, or a passphrase, is never read as a descriptor (fixes 35aba23cf, 7a17abcf7).
- `tr(…, sortedmulti_a)` written as one chain is accepted (fix fc42643ae).

Account sheet (`faraday/faraday-core/src/wallet/account.rs`)
- A Coldcard export holding both a single-sig and a silent account, unchosen, is refused with `Refusal::AccountUnchosen` and waits for the dialog (`faraday/faraday-core/src/wallet/account.rs:~90-120`).
- A silent account that fails to read leaves the single-sig wallet (`~93-97`).
- The load choice is kept for the session. Add to vault always asks (`~128-138`, `~185-190`).

Key checks (`faraday/faraday-core/src/wallet/confirm.rs`, `faraday/faraday-core/src/wallet/keycheck.rs`)
- A key is Yours only if its public key and chain code equal the loaded key's at that path (`faraday/faraday-core/src/wallet/confirm.rs:217-231`).
- It is Unchecked if the fingerprint is not loaded.
- It is Wrong (`EN.mscheck_wrong_key`) otherwise.
- The single-account flag applies to 3-step paths with hardened 44, 49, 84 or 86 (`faraday/faraday-core/src/wallet/confirm.rs:~197-206`).
- A tick toggles only Unchecked and Checked (`faraday/faraday-core/src/wallet/confirm.rs:~112-120`; `faraday/faraday-core/src/wallet/keycheck.rs:~165-175`). A sheet's ticks carry to the wallet by key, not by place.
- The count line (`faraday/faraday-core/src/wallet/confirm.rs:~243-258`):
  - none under two keys;
  - green "all" only with no unchecked key and no wrong one.
- The receive-address caution is not shown when no key is unchecked (`~337-346`).
- A wallet of one key has no key checks (`~70-76`).
- `Session.waiting` and `waiting_inputs` show as "waiting" (`~190-216`).
- Check a multisig setup:
  - "Open the wallet" and "loaded" show only while the wallet at the place is still the matched one (fix 92cf271c2);
  - ticks merge into the added wallet by key (fix f1129cc72).

Wallet key files
- `wallet_key` refuses (`faraday/faraday-core/src/wallet.rs:~5484-5524`):
  - silent payments;
  - a missing slot;
  - a fingerprint not held;
  - an unknown wallet.
- `wallet_key_record` works only for `wsh` or `sh(wsh)` multisig at Create's own account (`~5550-5565`).
- `wallet_keys_here` gives none for a one-key wallet (`~5531-5533`).
- A key's ur:crypto-account code and BIP 129 record come from its own origin path (fix 7299afb3b).

Published sheet
- `PublishedRemove` and `PublishedKeep` both do nothing for a sample seed (`is_sample`).
- Keep does nothing for a key made online; only the hold keeps it.

Wallets tab (`faraday/faraday-core/src/screens/wallets.rs` and neighbours)
- Keys:
  - A key already in a wallet is not "loose" (`~181-185`).
  - The missing-keys line names only a locked vault. It is hidden when an open vault holds a missing key (`~88-120`).
- Disabled when `!may_load_keys()`: Unlock (`~146-171`), Add key and Scan key (`~387-390`, `~761-762`, `~993`).
- Scan a key for the shown wallet only when it is not silent and cannot spend; otherwise Add a key (`~372-383`).
- Spend status:
  - Watch only is shown only when the wallet cannot spend and no key is held toward the quorum (`~260-264`).
  - "Another key can spend" overrides "can spend", in the error tone (`~270-271`, `~745-749`; the same rule is in `faraday/faraday-core/src/screens/wallets_start.rs:~255-258, ~322-323`).
  - "Does not match" shows for a silent record that differs from the loaded keys (`~268-269`, `~963-964`).
- New wallet from this one is offered only for kinds that `NewKind::of(policy)` accepts. A silent wallet offers none (`~605`; `faraday/faraday-core/src/screens/wallets/actions.rs:~54-74`).
- The receive row is not pressable, and is Warning-toned, when the wallet is held back (`~455-464`).
- Spend path buttons:
  - "Set the time" when the clock is not set;
  - "I moved" only when the vault keeps the coins, else Disabled (`~849-866`);
  - missing keys and Scan only when the quorum is not met (`~790-794`).
- Silent card:
  - the address is withheld when a loaded key contradicts the record;
  - "not checked" caution when no loaded key has the fingerprint (`~900-915`);
  - Load only when a vault key matches, else Add (`~972-987`);
  - no Show code for a held silent address (`faraday/faraday-core/src/screens/wallets/actions.rs:~61-66`).
- An address is shown whole on one line; the smaller mono size is used where the larger would split it (fix 1c11e9963).
- Start page (`faraday/faraday-core/src/screens/wallets_start.rs`):
  - The empty page (ways only) shows only with no wallet, no open vault, no unfinished create and no spend (`~156-159`).
  - Wallets are two to a row, at most two rows, then "more" (`~162`, `~229-231`, `~363-369`).
  - On compact, loose-key rows show only when wallets are loaded (`~167`).
- Make a new key is on the start page only. The pair tile offers four ways (`faraday/faraday-core/src/screens/wallets_pair.rs:~99-113`); the pills variant offers only Add a key.
- The pair (`faraday/faraday-core/src/screens/wallets_pair.rs`):
  - refused where the room holds less, and never in a narrow room (`~22-33`);
  - ways pinned when the list does not fit (`~206-216`).
- Where the page cannot hold the list and a detail at least as wide, the card stands alone (fix 55a12d5a1).
- Rename: a name under 40 characters (`faraday/faraday-core/src/wallet/state.rs:537-548`).

Chart (`faraday/faraday-core/src/chart_edit.rs`)
- A date is kept only when the clock is checked; otherwise it is empty (`~122-134`).
- Notes are saved to the vault plan record only if a record reads for the wallet's keys. Otherwise the toast is `chart_edit_keys_order` (fix c8ec28109).
- A storage failure gives `not_set_text` (`~252-262`).
- A chart copy that is not the current vault gives `chart_edit_not_current`, and nothing is written (`~388-401`).
- A Seed XOR or SLIP-39 part or share is never marked Checked (F276, `~412-423`).
- Only Xor and Slip39 kept parts can be destroyed (`~749-754`).
- The passphrase check waits while seeds load. It marks Checked, with today's date, only on a match (`~541-555`).
- A change waits for its load's answer and is then made. Escape during a load takes the Cancel path, and a cancelled load adds no seeds (fixes f1a755273, 2eeaa102a).

## Domain calls

- `faraday_intake::wallet::{read_wallet, one_line, Refusal, wallet_name, network_of_key, policy_network, carried_name, network_of_kind, WALLET_NAME_MAX}`.
- `faraday_keys::{Keys, words::parse, identity::master_id, vault::VaultId, job::Derived}`.
- `faraday_bip::policy::WalletPolicy`: `script_at`, `keys_at`, `derive_named`, `address_at`, `leaf_of`, `spend_paths`, `quorum`, `tapscript_quorum`, `silent`, `key_path`, `template`.
- `faraday_bip::spend::Lock`, `faraday_bip::keys::{Fingerprint, Network, ScriptType}`, `faraday_bip::silent_wallet::SilentWallet`.
- `faraday_psbt::{Psbt, verify::*}`.
- `faraday_crypto::work::{Compute, Work::Address}`.
- `faraday_strings::bip::policy_error`.
- Chart: `faraday_vault` records (PLAN, PLAN_NOTE), `faraday_paper::pdf::sheet`, `faraday_bip` WalletId.

## Fixtures

The other wallets' files read into the session, used by `faraday/faraday-core/tests/all/interop.rs`, `faraday/faraday-core/tests/all/interop_outcomes.rs` and `faraday/faraday-core/tests/all/inbox.rs`. All are under `faraday/faraday-core/tests/vectors/interop/`:

- `README.md`, `faraday/faraday-core/tests/vectors/interop/outcomes.txt`, `faraday/faraday-core/tests/vectors/interop/core/listdescriptors.json`.
- `bluewallet/`:
  - `faraday/faraday-core/tests/vectors/interop/bluewallet/caravan-multisig.json`, `faraday/faraday-core/tests/vectors/interop/bluewallet/casa-descriptor.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/cobo-crypto-account.txt`;
  - `faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardQ.json`, `faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardmk4-descriptor.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardmk4-generic.json`, `faraday/faraday-core/tests/vectors/interop/bluewallet/coldcardmk4-wasabi.json`;
  - `faraday/faraday-core/tests/vectors/interop/bluewallet/fromsparrow-coldcard.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/fromsparrow-electrum.json`, `faraday/faraday-core/tests/vectors/interop/bluewallet/fromsparrow-specter.json`;
  - `faraday/faraday-core/tests/vectors/interop/bluewallet/nunchuck-bsms.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/nunchuk-export.json`;
  - `faraday/faraday-core/tests/vectors/interop/bluewallet/skeleton-cobo.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/skeleton-coldcard.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/skeleton-electrum-hex-only.txt`, `faraday/faraday-core/tests/vectors/interop/bluewallet/skeleton-walletdescriptor.txt`;
  - `faraday/faraday-core/tests/vectors/interop/bluewallet/sparrow-crypto-output.txt`, `unchained.json`.
- `sparrow/`:
  - `bsms/multisig-1.bsms` to `bsms/multisig-17.bsms`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/caravan-multisig-export-1.json`, `-2.json`, `-3.json`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-export-1.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-export-2.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-export-missingkey.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-export-multideriv.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-export-nopolicy.txt`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-keystore-1.json`, `faraday/faraday-core/tests/vectors/interop/sparrow/cc-multisig-keystore-2.json`, `faraday/faraday-core/tests/vectors/interop/sparrow/cc-singlesig-keystore-1.json`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/cobo-singlesig-keystore-1.json`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/descriptor-labelled.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/descriptor-multipath.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/descriptor-receive-change1.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/descriptor-receive-change2.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/descriptor-receive.txt`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/electrum-multisig-wallet.json`, `faraday/faraday-core/tests/vectors/interop/sparrow/electrum-singlesig-wallet.json`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/jade-keystore.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/keystone-singlesig-keystore-1.txt`;
  - `faraday/faraday-core/tests/vectors/interop/sparrow/specter-diy-export.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/specter-diy-keystore.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/specter-multisig-wallet.json`, `faraday/faraday-core/tests/vectors/interop/sparrow/specter-wallet.json`.

The silent payments keystores (`faraday/faraday-core/tests/vectors/interop/sparrow/cc-singlesig-sp-keystore-1.json`, `faraday/faraday-core/tests/vectors/interop/sparrow/keystone-singlesig-sp-keystore-1.txt`, `faraday/faraday-core/tests/vectors/interop/sparrow/specter-diy-sp-keystore.txt`) are listed under silent-payments. The two `bluewallet/quicklook-preview-sample.*` files are a PSBT and a transaction, not wallet files.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| aba412739 | Two wallets sharing an 8-character checksum were treated as one: Check, Write and address counts crossed; Spend from backup and the address look picked the wrong wallet. | Identity is the SHA-256 of the whole descriptor (`WalletId`). |
| 6ccc7839b | When a wallet left the list, kept places could still point at it; a flow ended on another wallet's record. | Every kept place follows a wallet leaving. |
| 5ccbb1659 | Share records and the backup guard matched by checksum; a twin took another's share records. | Match by id; twins get their own refusal. |
| 054836b32 | A wallet's description, plan record and flow were found by checksum. | Lookups by id; the checksum only for a flow that kept none. |
| 8096cf7af | The money move, the held receive address and a vault's share record compared by checksum. | Compare by id. |
| 6440042e0 | A wallet sharing a checksum with one in a locked vault showed backed up; plan, seals line, checklist and stick note attached to the wrong wallet. | Wallets identified by id. |
| be1213326 | A wallet naming this device's own key (xpub, not yet backed up) under another fingerprint was accepted, and its addresses showed as not held. | Refused when added, removed when a waiting key starts to wait; only the origin path compared. |
| f1a755273 | Escape on a chart's passphrase check or name field, while the wallet's seeds were loading from a vault, only closed the sheet; the seeds then joined the session when the load finished. | Escape during a load takes the Cancel path; a cancelled load adds no seeds. |
| 2eeaa102a | A confirmed chart change, a typed passphrase check, and Create or New wallet from this one, pressed while seeds were loading from a vault, were dropped after the load answered; a key left out gave no reason. | A change waits for its load's answer and is then made; a press runs again after the load; a key left out says why. |
| c8ec28109 | A chart press on a plan kept only for the other key order did nothing visible. | A refused press says the plan cannot be changed with the keys in this order (toast). |
| 81f0c4a35 | A held guess's cancelled sheet did not leave its tried way as the wallet is; the register sheet repeated the shape. | A cancelled held guess keeps its tried way; the register sheet states the shape once. |
| 92cf271c2 | A Check a multisig setup Result said "Open the wallet" / "loaded" for a wallet taken out since, pointing at whatever sat at that place. | Shown only while the wallet at the place is still the matched one; otherwise Add this wallet is offered. |
| f1129cc72 | Check a multisig setup: Add this wallet dropped the Result's ticks when a loaded wallet had its keys in another order; a tick after a removal could act on the wrong wallet. | Ticks merge into the added wallet by key; a Result reads a loaded wallet only while the wallet at its place has the Result's policy. |
| e6a94955a | A wallet name was cut to 64 bytes before being checked for secret text; invoice text was cut. | The name is checked whole for secrets before the cut; invoice text not cut. |
| 8121c90cd | A wallet file whose name line, carried name or file name was a seed or extended private key was added under that secret as its name. | Secret text is never a wallet name; the wallet is added under the default name Wallet. |
| f36ccb21d | Putting the keyboard away, or pressing the chart holder's field, closed the Chart sheet. | A sheet's own field presses stay in the sheet. |
| 278d951d8 | A tick press on a chart sheet closed it on a one-column layout, and the key was not shown checked. | The sheet stays open after a tick; the key shows checked and the count line updates. |
| 1c11e9963 | A wallet card's first receive address wrapped to two lines at the larger size. | An address is shown whole on one line, at the smaller mono size where needed. |
| ffdcf6e1c | The Wallets list pane heading repeated "Wallets" under the top app bar. | One name for the page. |
| 61fcca48d | A MuSig2 wallet, or one whose script needs every key, was read as needing one seed; a script with no stated quorum read as m of n keys, so two keys of one seed looked like two of three. | Keys that must all sign need every seed; quorum counts are read as seeds. |
| 35aba23cf | An xprv in a Caravan file's xpub field was handed to the descriptor parser. | A secret key text is never read as a descriptor. |
| 7a17abcf7 | A Krux code's note, used as the next seed's passphrase, was handed to the descriptor parser. | A passphrase is not read as a descriptor. |
| c1c88d6e4 | A wallet whose script states no quorum counted one seed at two keys as two of what was needed (wallet cards, glance, spend backup). | The seeds a spend needs = the stated quorum, else the fewest seeds of any spend; MuSig2 needs every seed. |
| 68a0eff42 | A tick made on a sheet landed on the wrong key of a wallet loaded in another key order; a tick could apply after a wallet was taken out. | A sheet's ticks carry by key; a press for a place is dropped when the list lost a wallet since the last frame. |
| c239c5105 | After a wallet was removed, a tick drawn on its place before the removal could toggle a key of another wallet. | Every removal marks the list changed until the next frame; a stale tick press is dropped. |
| 6d64494fe | A wallet file's large number could read one step from the nearest float (`5e111` as 5.000000000000001e111). | A number reads as one value, the same as the kept text. |
| 7299afb3b | A multisig wallet at BIP-87's path gave no ur:crypto-account code and no signed BIP 129 key record. | Each key's code and record come from its own origin path; BIP 129 and crypto-account carry any origin path. |
| 7fcae04a9 | A MuSig2 wallet whose two keys came from one seed was accepted, and could never sign. | Refused when added. |
| 89aa14678 | A wallet key with a loaded key's fingerprint but another seed's xpub was accepted; a bare xpub without origin was accepted as a wallet key. | Refused whenever the key loads; a bare xpub is refused everywhere. |
| 3e7ce0ba9 | A wallet description that repeated a key, mixed networks, or wrote a key or header badly was accepted. | Refused; a BSMS address of another network kind than its keys is refused. |
| 7e6b2eaff | A file with several descriptors loaded whichever came first; two wallets of the best kind were not refused. | Loads the documented order; two different wallets of the best kind refuse the file. |
| fe411fdfe | A file with a silent payments wallet beside a BIP-84 wallet loaded the silent payments one. | A silent payments wallet beside another wallet refuses the file. |
| af3aa3474 | The silent payments card encoded its address on every frame. | The address is worked out once (a cost fix; drawing unchanged). |
| a141eebb6 | Removing a wallet, or taking back one made in Create, left an open spend on another wallet. | An open spend follows its own wallet and closes when that wallet is gone. |
| 55a12d5a1 | At 1280 x 800 a wallet card beside a chart's side sheet was squeezed to about 200 dp. | Where the page cannot hold the list and a detail as wide, the card stands alone. |
| eecf2bfae | A tr() key path whose key was not the wallet's was refused. | Worked out once; a foreign key path is a warning, not a refusal. |
| fc42643ae | A taproot sortedmulti_a wallet written as one chain was refused as an unsupported template. | `tr(…, sortedmulti_a)` from a wallet file is accepted as Core reads it. |
| 62851fc36 | A wallet name with a newline or control character broke the sheet; a sheet naming a field twice was accepted. | A wallet name is one line of text wherever read; a duplicate field is refused. |
| e45fa284d | The same sortedmulti wallet with its keys in another order counted as a new wallet; a saved plan did not open for the other order. | A sortedmulti wallet is one wallet whatever its key order; a plan record opens for either order. |
| 1590a5825 | A SLIP-132 version was converted to xpub without checking it against the script; upper-case bech32 BSMS addresses and "No path restrictions" BSMS records were refused. | A key whose version names another script, or its path's purpose, is refused and named, never converted. |
| d89080ffc | A BSMS first address was compared as a string; a `/**` template with "No path restrictions" was not read as Faraday reads it. | Compared as an address (canonical form, network checked); `/**` reads as `/0/*,/1/*`. |
| b0010427e | Multisig configs accepted a key whose version disagreed with its script; Coldcard's fields were converted unchecked. | Multisig configs refuse a disagreeing key; SLIP-132 decoded and checked in one place. |

## Doubts

- Rows given to this feature but left out as not about it:
  - c2a2331c8, 5b50f041a, dffb58d10, e4de9f628, b0cc7dc2d, f9eebc20c (the made-online fingerprint check, in backups and vault);
  - 9d0783725 (Lock and write in the backup flow);
  - 47bfe9479 and e8b9376ac (PSBT change and multisig classification, in signing);
  - 2dfb7fe57 (the spend route's signers card).
- Rows moved here from public-files, because they are about reading wallet files: 1590a5825, d89080ffc, b0010427e. The last two name `core/osk-bip/...` paths, an earlier name of `core/faraday-bip`. 35aba23cf and 7a17abcf7 are carried only for their descriptor rule.
- `e45fa284d` was not read beyond its commit message.
- Line numbers in `faraday/faraday-core/src/screens/wallets.rs`, `faraday/faraday-core/src/screens/wallets/actions.rs`, `faraday/faraday-core/src/screens/wallets_start.rs` and `faraday/faraday-core/src/screens/wallets_pair.rs` are unconfirmed. Those files were read once and not checked.
- `Sheet::Pull` in `confirm_act` (`faraday/faraday-core/src/wallet.rs:~4640`) clears the sheet with no further step. The reason is unclear.
- Duplicates in `faraday/faraday-core/src/wallet.rs`:
  - `fp_text` and `fp_label` (`~604`, `~610`) are the same;
  - `address` (`~1776`) and `address_shown` (`~1782`) are the same;
  - `master_proof` (`~475`) had no caller found.
- `file_witness_refusal` (`faraday/faraday-core/src/wallet.rs:~4162-4177`) is live; it is used at `~3688`.
- `nonce_check`'s doc says only valid signatures are looked at, but the filter does not check validity.
- The chart's sheet state (`result`, `refused`, `moving_to`, `work.text`) survives the glance lock hook. The process restarts at lock anyway.
- `Make a new key` is on the start page but not on the pair tile. Whether that is intended is unclear.
- `KeyWallet(fp, 1)` is a literal of unclear meaning (`faraday/faraday-core/src/screens/session_pane/rows.rs:~180, ~330`).
- The callers of `wallet_page`, `wallet_rows`, `loose_card`, `missing_keys_line` and `first_receive_row` outside the tab were not traced. `faraday/faraday-core/src/screens/row_details.rs:~276` calls `wallets::{actions, wallet_page}`.
