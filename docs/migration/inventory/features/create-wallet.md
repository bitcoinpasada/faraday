# Making a new wallet

Make a new wallet (`Screen::Create`) is a step flow for making a wallet on
the device. The person picks its kind (single key, multisig, taproot,
MuSig2 and others), its backup form and its quorum. They then fill each key
slot with one of:

- a key made here;
- a loaded key;
- a key from a vault;
- a cosigner's xpub (scanned, typed or from a file);
- "someone else later".

The wallet is made and checked, and the flow goes on to Back up.

It is opened from:

- the Wallets start page (`Screen::WalletsStart`) and Home;
- a wallet's "New wallet from this one" (`NewWalletFrom`);
- the chart's "move money to a new wallet" (`Action::CreateWallet`);
- the session pane's From key.

Create shares the Sticks-and-vaults board with Restore.

## Screens and sheets

Logic: `faraday/faraday-core/src/create.rs`, `faraday/faraday-core/src/cstep.rs`, `faraday/faraday-core/src/create_pick.rs`, `faraday/faraday-core/src/create_from.rs`. Drawing: `faraday/faraday-core/src/screens/create.rs`, `faraday/faraday-core/src/screens/create_making.rs`.

`Screen::Create` cards (`faraday/faraday-core/src/cstep.rs`):
- KIND 0, QUORUM 1, KEYS 2, CHECK 3;
- WHERE 5, STICKS 6, VAULTS 7, HOW_MANY 8;
- BACKUP_FORM 9, COUNT 10;
- there is no 4.

The cards show:
- Kind. There is a More kinds row.
- Backup form.
- Quorum: M and N, path rows (BIP-48 or BIP-87 where the kind has both), and the account row where the kind has an account.
- Keys: one row per slot. A slot's state is one of:
  - `Source::Here`;
  - `Source::Cosigner` (a fragment of the cosigner's xpub is shown);
  - `Source::Empty`;
  - `Source::Later`.
- The use-existing-vault switch, and refused key or wallet files with their reason.
- Making a key on the card (`faraday/faraday-core/src/screens/create_making.rs`):
  - length 12 or 24;
  - Other ways;
  - show the words;
  - again;
  - passphrase and lock;
  - dice Undo, Clear, Next.
- Check:
  - the wallet QR, the descriptor and the paths;
  - the codex32 shares row;
  - Move the money;
  - Path Change (take back);
  - "It matches".

The foot (`faraday/faraday-core/src/screens/create.rs:385-395`):
- no button while a slot is empty and not locked;
- Continue once locked;
- otherwise "Make the wallet".

Other screens and sheets:
- `Screen::Backup` follows the make.
- `Sheet::ChildNotice`: the BIP-85 child notice, a `compact::dialog` with OK = `Cancel` only (`faraday/faraday-core/src/screens/create.rs:~1010-1038`, F166).
- `Sheet::BoardMenu`.
- The Sticks-and-vaults board (`faraday/faraday-core/src/create.rs:~2546-2628`).

## Actions

The presses are handled by `create_act` (`faraday/faraday-core/src/create.rs:~1715`) and `edit` (`~1510`).

Opening
- `CreateWallet` and `CreateOver` open on Kind with nothing ticked, NativeSegwit 2-of-3 and one empty slot.
- An unfinished wallet resumes (`~1718-1750`).

Steps and shape
- `CStep`, `CKind`, `CMoreKinds`, `CBackupForm`, `CM`, `CN`.
- `CAccount`, `CMultiPath`, `CMultiPathChange`, `CPathAbout`, `CBackupAbout` (tooltips).
- `CExistingVault`.

Slots
- `CSlotHere`, `CSlotPick`, `CSlotLater`, `CSlotClear`, `CSlotFile`, `CSlotVault`.
- `KeyOnCard`, `KeyGen(Some(slot))`.
- `ScanFor(Ask::Xpub)`.
- `Vault(OpenFrom(i, Screen::Create))`.

Typed xpub and passphrase
- `CXpubOpen`, `CXpubField`, `CXpubTake`.
- `CPassOpen`, `CPassField`, `CPassLock`.

Key out
- `CKeyOut`, `CKeyQr`, `CKeyBsms`, `CodePng(Code::Key)`: the account xpub as a file, a QR or a BIP 129 record.

Making a key on the card
- `KWords`, `KeyOtherWays`, `KShow`, `KAgain`, `KPassField`, `KLock`, `KUndo`, `KClear`, `KNext`.

Continue
- `CNext` on KEYS makes the wallet.
- `CNext` on CHECK marks it checked and starts Back up, unless money is being moved.
- `CBackup`.

Check
- `QrWallet`, `CPathChange` (take back), `SpendBackup(SpendFrom(old))` (move the money), `PlanAddPass`.

## Effects

- The wallet is added to the session through `session.add_wallet`. The name is "New wallet N", cut to `WALLET_NAME_MAX`.
- Keys are added with `faraday_keys` (`Spec`, `multisig_account`, `add_with_passphrase`). This includes a key made on the card.
- Out of the device:
  - the account xpub file;
  - a QR;
  - a PNG (`CodePng`);
  - a BIP 129 record, put in the Outbox.
- The ur:crypto-account code (`faraday_qr::registry::account_cbor`, `faraday_codec::ur::encode_single`).
- Vault keys loaded through `create_from`, including the vault's load job.
- Back up is started after Check.
- `session.follow(net)` can move the session's network (`faraday/faraday-core/src/create.rs:~2122`).

## State and lock

- `CreateState`, in `CreateArea.state` (`faraday/faraday-core/src/create.rs:~1007, ~1191`), holds:
  - choices;
  - slots;
  - `built`;
  - `pass: PassField` (a secret);
  - `before_made`;
  - `shares_set`.
- Lock (`faraday/faraday-core/src/create.rs:2841-2852`):
  - `holds().typed` is set when a passphrase is held or a pre-made backup flow exists;
  - `lock()` drops the whole state.
- `PassField` holds a `SecretText`, which wipes on drop.
- The process restarts at every lock (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Take back clears `built`, the Keys and Check done marks, and the slots of keys removed since the make (`faraday/faraday-core/src/create.rs:~1439-1462`).
- `CreateVaults` is built per frame in Drawing, with fingerprints and labels only.
- Keys made on the card are the New key flow's (see keygen-flow). The idle warning puts seed words, SeedQR and passphrase away once (fix 4f273508a).
- A key removed from the session is taken out of the flow's counts (fix 083d29bc5).

## Reach

- Opens:
  - New key on the card (keygen; it stays in Make a new wallet);
  - Add a key;
  - the scanner;
  - Files;
  - vaults (unlock, load a key, make a vault);
  - Back up;
  - Spend from a backup (move the money);
  - QR view.
- Opened from: the Wallets start page, Home, the wallet card, the chart, and Explore's and Check an address's Make a wallet.
- Code reach: `wallet`, `pass_field`, `secret_text`, `seeds::cosigner_key`, `codex32`, `forms`, `backup_flow`, `flow`, `plan`, `pick`, `trail`, `board`, `in_progress`, the wallet-flow helpers, `faraday_intake::{wallet, single_key}`.
- `create_pick` is used by create (`faraday/faraday-core/src/create.rs:2783-2785`).

## Rules and refusals

Kinds and shape
- Kind codes (`faraday/faraday-core/src/create.rs:~120-179`): nine kinds; codes 0 to 7 and 9 are valid; 8 is refused; 10 is read as Single key plus Codex32.
- Multisig has 2..=15 keys (`max_keys` 15, `faraday/faraday-core/src/create.rs:293-296`).
- CN clamps (`faraday/faraday-core/src/create.rs:1576-1585`). A kind change clamps (`~1533`).
- m is clamped to 1..=n. For MuSig2 (`all_sign`) it is forced to n (`faraday/faraday-core/src/create.rs:1534, 1569`). MuSig2 and all-sign kinds choose only the key count (`faraday/faraday-core/src/screens/create.rs:~236-238`).
- Single-key kinds have one slot (`faraday/faraday-core/src/create.rs:~1537`).
- A kind or path change empties cosigner slots that are at another path (`faraday/faraday-core/src/create.rs:~1545-1548, ~1400-1402`).
- Account:
  - clamped to 0..=2^31-1 (`faraday/faraday-core/src/create.rs:~1374`);
  - `MultiLegacy` has no account;
  - `m/45'` has no coin type.
- Choices that do not check `built`:
  - Only `CKind` (`faraday/faraday-core/src/create.rs:1521`) and a few slot edits check `built.is_none()`.
  - `CStep` (`1510-1517`), `CM` (`1567-1575`) and `CN` (guarded by `kind.multi()` only, `1576-1585`) do not check whether the wallet is already made.
- The BIP-87 path choice shows only for kinds that have it (`faraday/faraday-core/src/screens/create.rs:~273`).
- The account row shows only where the kind has an account (`~276, ~948-974`). At the head of Keys it shows only when the kind has no Quorum card (`~314-318`).

Continue
- On KIND it waits while no row is ticked.
- On BACKUP_FORM it waits while unanswered.
- Keys stays closed until a single key's backup is answered (`faraday/faraday-core/src/create.rs:~1766-1774`).
- On KEYS it is refused while a vault's stick is pending (`~1754`).

Passphrase lock (`faraday/faraday-core/src/create.rs:2238-2260`)
- It needs:
  - the self-test passed;
  - an offered slot (this device's own BIP-39 words key, with no passphrase yet);
  - the eye not waiting;
  - the two fields equal.
- A mismatch returns without a note.
- Empty means no passphrase.
- The same key with the same passphrase is treated as the same key.

Typed xpub
- At most 400 characters (`faraday/faraday-core/src/create.rs:2349`).
- Control characters are refused.
- It is taken only on `Screen::Create` with no sheet (`~2342-2352`).

Key files and cosigner keys (`faraday/faraday-core/src/create.rs:~728-780, ~878-922`)
- The origin is required.
- The key must be at the kind's path and coin type.
- A SLIP-132 version naming another script is refused.
- A descriptor must hold one key.
- A scanned descriptor fills the waiting slot only when it names exactly one key that can be it. A key fills a slot only with its origin at the kind's derivation (fix 26c383b64).
- Duplicate cosigner keys are refused (`create_keys_the_same`). Two slots with the same key and chain code are refused, whatever their origins (fix 39896fa44).
- A key held in one slot is not offered in another (`faraday/faraday-core/src/screens/create.rs:93-101, 546-549`; `create_pick` says "In slot n").
- A file's key is not offered where the same cosigner text is in another slot (`faraday/faraday-core/src/screens/create.rs:~588-590`).

Making the wallet
- A Later slot, or an empty slot, refuses the make with the slot's own line (`faraday/faraday-core/src/create.rs:~2064-2096`).
- A cosigner of another network is refused through `session.follow(net)` (`~2122`).
- `Unconfirmed` from `session.add_wallet` is held for its sheet (F254). Other refusals go into `error` (`~2526-2535`).
- No reason is shown while the wallet waits for the hold (fix e4347e8d4).

Key out
- BIP 129 only for wsh or sh(wsh) multisig.
- The account UR only for Multi.
- Show xpub (QR, PNG, file) only for multisig with a key held here (`faraday/faraday-core/src/screens/create.rs:~654-698`).

Slot offers (`faraday/faraday-core/src/create_pick.rs`, `faraday/faraday-core/src/screens/create.rs`)
- Scan its xpub, and New words, only for an empty multisig slot not being made (`faraday/faraday-core/src/screens/create.rs:~561-571`).
- A single-key kind offers New key only, when not in the pane and not making (`~577-587`).
- Type or paste xpub only for multisig Empty or Later slots (`~605-611`).
- "Someone else later" only for multisig Empty slots (`~612-618`).
- Add a passphrase:
  - single-key: on the slot row;
  - multisig: on the Show xpub QR row (`~685-693`).
- Use xpub only when the text is not blank (`~733`).
- Lock passphrase only when both fields match and the eye is not waiting (`~799-801`).
- The use-existing-vault switch shows when no slot is filled, or when it is on. A stick is never read while a key is open (`~319-328`).
- With a vault to unlock (wallet flow pending, locked), the step shows only the bring-in text and Show (`~329-345`).
- Refused key and wallet files are listed with their reason, and not offered, only when not locked (`~374-378`).

Making a key on the card (`faraday/faraday-core/src/screens/create_making.rs`)
- The length is chosen first (`~41, ~147-150`).
- Lock is enabled with no passphrase, or with two matching fields and the eye not waiting (`~107-109`).
- Make the words only when `k.ready()` (`~145`).
- A key made in Make a new wallet stays on the Keys card, even after a stick is pulled (fixes 86ac6fd76, c3587d436). It offers only the backup form the Backup card chose (fix 1e4bf7b55).

Check
- The descriptor and paths show only once built (`faraday/faraday-core/src/screens/create.rs:~819-841`).
- The codex32 shares row shows only when the made key is a codex32 set (F240, `~828-830`).
- Move the money only for a wallet made to move another's (`~858-870`).
- Path Change only while `create_can_take_back` (`~877-883`).
- BIP-48 info only while being made, not legacy (`~900-903`, F287).
- The child notice only when a child (BIP-85) key was recorded (`~1016-1022`, F166).
- No receive address is shown before the backup (fix f50c4cf01).

From vaults and from another wallet (`faraday/faraday-core/src/create_from.rs`)
- Vault keys not already in the session are offered as slot fills (`~81-83`).
- Keys load only when `may_load_keys()`, and only from KEY records (`~134-143`).
- A key just loaded from a vault fills the slot that asked (fix 30e145c10).
- A fingerprint is exactly eight hex digits; one with a plus is refused (fix 32b90e7c5).
- New wallet from this one:
  - takes the next free account for the same key set, with the search capped at 2^31 (`~175-196`);
  - is refused when the policy has no `NewKind` (`~209`);
  - gives back keys only when their origin is at the path chosen (fix 64075b4e2).
- `create_can_take_back` refuses (`~357-379`):
  - with no account;
  - once the backup has begun;
  - when the added wallet is not the last;
  - when the owner's flow has moved on.
- Take back removes the added wallet (`~386-408`).
- A key file from Add a key's stick fills the slot that asked; a slot waits while Files stands (fix cd2e42596).

## Domain calls

- `faraday_bip`: keys, policy, slip132, bsms, coldcard, json, descriptor, bip32.
- `faraday_keys`: `Keys`, `Spec`, `multisig_account`, `add_with_passphrase`.
- `faraday_qr::registry::account_cbor`.
- `faraday_codec::ur::encode_single`.
- `faraday_vault::codex32::SetFacts`.
- `faraday_intake::{wallet, single_key}`.
- `faraday_crypto::work::Compute`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| f50c4cf01 | In the vaults-first setup, a plan vault with none set up gave a dead Continue; Lock on a new vault did not lock its target; the lock that makes the process clean was not asked first; Create and Restore returned past questions that should stay fixed once a setup vault is open; a receive address showed before the backup. | The vaults-first setup survives a lock; Make a vault is offered where Continue was dead; no receive address before the backup. |
| 26c383b64 | A scanned descriptor went into any open slot while Create waited for a cosigner's key; a key could fill a slot without its origin at the kind's derivation. | A descriptor fills the waiting slot only when it names exactly one key that can be it; a key fills a slot only with its origin at the kind's derivation. |
| 86ac6fd76 | With a stick in, New key on the Keys card opened the separate New key flow. | New key stays on the Keys card, after the stick is pulled. |
| c3587d436 | Making a key through Other ways, New key on a slot or New key for Key N left Make a new wallet. | The key flow stays in Make a new wallet. |
| 1e4bf7b55 | A key made in Make a new wallet offered SLIP-39 shares under Other forms beyond the Backup card's choice. | Only the way the Backup card chose is offered. |
| 3324c78d7 | Machine rows in Create's "Where will you open it?" were 40 units tall, too small to press on touch. | Touch targets are 48 units tall. |
| 39896fa44 | A descriptor using one key twice (same key and chain code under another origin or depth) was not refused by Create. | Create refuses two slots with the same key whatever their origins; disjoint derivations of one key stay allowed. |
| 64075b4e2 | Changing a multisig wallet's kind kept a cosigner's key at the old path; New wallet from this one gave back a stored key whose origin was not the chosen path. | A cosigner's slot whose key is not at the new kind's path is emptied; keys given back only at the path chosen. |
| 6007cfeb7 | A several-key wallet with no kept description was offered Make the checklist and the plan gave no reason. | Make the checklist is withheld with a stated reason. |
| e4347e8d4 | Create showed a reason beside the confirmation sheet while its wallet waited for the hold; a Restore from typed seeds was not finished as Make would be. | No reason shown while waiting for the hold; a Restore from typed seeds is finished as Make is. |
| 4f273508a | While Make a new wallet held the idle lock, seed words, SeedQR and passphrase stayed on show at the warning. | They are put away once when the warning comes; Show words shows them again. |
| 30e145c10 | A key loaded from a vault did not fill Create's slot when the press ran again after the load, so a key whose passphrase was kept alone in the vault left the slot empty. | The key just loaded fills the slot; Enter on those fields runs like the buttons. |
| 32b90e7c5 | A fingerprint with a leading plus was accepted by some paths. | A fingerprint is exactly eight hex digits everywhere. |
| cd2e42596 | A key file from Make a new wallet's Add a key stick did not fill the slot that asked. | The waiting slot is kept while Files stands and filled by the pull; with two files the first fills; with no slot it ends on Wallets. |
| 083d29bc5 | After Remove every key, Make a new wallet (and Restore, Spend from a backup) still counted removed keys. | A removed key is taken out of every flow that counts keys. |

## Doubts

- Rows left out as not about this feature:
  - 4ea07767f and 72fba6cf0 (Make a vault);
  - 9f65d0e6a (the policy compiler);
  - the Convert half of 6007cfeb7.
- Rows moved here from restore-wallet: cd2e42596 and 083d29bc5.
- The two sources disagree on the card numbers. One says `cstep` has cards 5 to 10 with no 4. The other says the drawing has "five unused numbers 4-8 that draw blank" (`faraday/faraday-core/src/screens/create.rs:~20-32`). The cards past CHECK appear to be drawn by the board, not by this screen. Not settled.
- Kind code 8 refused and 10 read as Single key plus Codex32 (`faraday/faraday-core/src/create.rs:~120-179`) is unconfirmed.
- Whether the UI hides `CStep`, `CM` and `CN` once a wallet is made is unclear.
- A possible panic: `faraday/faraday-core/src/screens/create.rs:451` slices the cosigner text as `&t[1..9.min(t.len())]`. An empty string would give `1..0`, and non-ASCII at byte 1 or 9 would split a character. Whether `Source::Cosigner` text can be either was not traced.
- Possibly dead: `create_keys_opened` is empty (`faraday/faraday-core/src/create.rs:~1957`), `into_vaults` is always false (`~1128`), `still_fits` is always true (`~2794`). `_room` is unused (`faraday/faraday-core/src/screens/create.rs:~57`).
- The `expect` at `faraday/faraday-core/src/screens/create.rs:~156` relies on the dispatcher checking first.
