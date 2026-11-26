# Silent payments

Silent payments (BIP-352) lets a person receive on the offline device
without an address being reused. The flow starts from a loaded key:

- it shows the key's silent payment address, with optional labels;
- it offers a DNS payment record (BIP-353) and a record file;
- it can give the scan key or the `sp(...)` descriptor (BIP-392) through the secret sheet;
- it can add a silent payments wallet to the session;
- it can check whether a transaction paid the key.

It is reached from:

- Tools (`Action::Silent`);
- a silent wallet's card on the Wallets tab (`Action::SWallet(i)`, which returns to `Screen::Wallets`);
- a Coldcard export that holds both a single-sig and a silent account (the account sheet).

## Screens and sheets

Files: `faraday/faraday-core/src/silent/mod.rs`, `faraday/faraday-core/src/silent/screen.rs`, `faraday/faraday-core/src/silent/check.rs`.

- `Screen::Silent` has four cards (`sstep`, `faraday/faraday-core/src/silent/mod.rs:~29-41`; drawn at `faraday/faraday-core/src/silent/screen.rs:~25-58`):
  - Key: choose a loaded key.
  - Address: the address with its label, QR, DNS name and domain fields, the DNS record and the record file.
  - Scan key: give it to the vault or through the secret sheet, and the descriptor.
  - Check: a payment.
- A silent session wallet opens the same flow as its page.
- `Sheet::Account` (`account_dialog`, `faraday/faraday-core/src/silent/screen.rs:~518-559`; opened at `faraday/faraday-core/src/silent/mod.rs:978-982`) is for a Coldcard export with both accounts. The person chooses one, or Cancels.
- The wallet card's silent part is drawn by the Wallets tab (see wallets).

## Actions

Steps and keys
- `Silent` opens the flow.
- `SStep(k)` moves to a card.
- `SKey(fp)` picks a key, and resets the label on a key page.
- `SLabel(±1)` moves the label within 0..=`MAX_LABELS`. "+" from no label goes to the first label not yet handed out (`faraday/faraday-core/src/silent/mod.rs:826-834`).
- `SAnother`.

Address card
- `SQr(uri)`.
- `SRecord` writes `silent-<fp>.txt` to the Outbox and shows a toast.
- `ShowCode(Code::SilentDns | SilentRecord)` and `CodePng(Code::SilentRecord)`.
- Typing in the DNS name and domain fields.

Scan key card
- `SScanVault` stores the scan key as a NOTE record in the open vault.
- `SScanOut` gives the scan key, `spscan1q...`, through the secret sheet.
- `SDescriptorOut(i)` gives the BIP-392 `sp(...)` descriptor through the secret sheet.

Wallet and check
- `SAddWallet`.
- `SCheck(k)`.

Other
- `KeyGen(None)`, `Backup(w)`, `Entry(fp)`.
- `LoadAccount(Some(Silent | SingleSig) | None)` on the account sheet.

## Effects

- Vault writes:
  - Label counts are raised into the open vaults' wallet records (`faraday/faraday-core/src/silent/mod.rs:~1576-1588`, `vaults.edit_or`, `changed`).
  - The scan key goes in as a NOTE record through `vault_push` (`~1301-1304`).
- The Outbox file goes through `put_outbox` (`~1279`).
- The secret sheet: `offer_secret(SecretOut)`, with `Zeroizing` (`~1320-1325`, `~1387-1392`).
- Session:
  - `set_policy`, `add_wallet`, `add_policy`;
  - `silent_handed.keep` and `silent_handed.raise` (the labels handed out).
- Toasts.
- Derivations run through `Compute`: `silent_receiver`, per key and account.
- The receiver is cached in `SilentState.shown`, a RefCell written from drawing paths (`~609-644`).
- Check a payment reads an Inbox file as a transaction. It takes previous transactions from the Inbox until every prevout is known (`~1620-1641`).

## State and lock

- `SilentArea` (`faraday/faraday-core/src/silent/mod.rs:~355-369`) holds:
  - `state: Option<SilentState>`;
  - `followed`;
  - `told`;
  - `full` (hashes).
- `SilentState` holds:
  - open, done[4], scroll;
  - the key's fingerprint;
  - label, labels;
  - wallet;
  - check (Text, Check);
  - back;
  - the DNS fields;
  - `shown`.
- The state holds a fingerprint, label counts, DNS text, a transaction file's bytes and prevouts. None of these is secret.
- `impl at_lock::Public for SilentArea` (`faraday/faraday-core/src/silent/mod.rs:372`), and `silent` is in the public areas list (`faraday/faraday-core/src/at_lock.rs:~175-191`). The lock walk wipes nothing in it. It is cleared only by `clear()` (`faraday/faraday-core/src/silent/mod.rs:~408`) and `wallet_left` (`~378-392`).
- Every lock still ends in a restart (`faraday/faraday-core/src/vault/seal.rs:933-946`), so the flow does not outlive a lock.
- `HandedOut`, the labels handed out per silent wallet, lives on the Session. It ends at `Session::wipe` (`faraday/faraday-core/src/wallet.rs:1539-1549`, which sets `silent_handed = Default`), that is, at lock.
- Secret material:
  - The scan key and the descriptor are `Zeroizing` on their way to the secret sheet.
  - The vault NOTE holding the scan key is built as `Text` (`faraday/faraday-core/src/silent/mod.rs:~1300`), which wipes on drop.

## Reach

- Opened from Tools, the Wallets tab's silent card (`silent::RecordCheck`, `differs_line`, `check_record`), the account sheet of a loaded file, and `Faraday::silent_*` entry points.
- Opens:
  - Make a new key (`KeyGen`);
  - Back up (`Backup(w)`);
  - Add a key (`Entry`);
  - vaults (`vault::VaultAction::AddFile` for Add to vault from the account sheet);
  - the secret sheet (`secrets::SecretOut`).
- Code reach: `wallet` (`Session`, `Account`, `same_wallet`), `trail`, `flow`, `edit::Here`, `visible`.
- The session's add-wallet refusals `SilentOtherKey` and `SilentDiffers` (see wallets) use the record check.
- The public files of a silent wallet follow `Inputs.blank` and `Inputs.held` (see public-files).

## Rules and refusals

Record against keys (`faraday/faraday-core/src/silent/mod.rs:~283-352`)
- The result is Matches, Differs or Unchecked. Matching wins.
- A differing record gives:
  - no address, no labels, no scan key;
  - no check, no code;
  - `silent_marks` Err;
  - `public_wallet` None.
- Unchecked gives only the address without a label, and a "not checked" warning.
- A record under another fingerprint, over a loaded key's points, is Differs, with the refusal named (fix f7b186d26).
- An account ≥ 2^31 is refused when the record is read (fix f7b186d26).
- A silent wallet's id is over its address and origin, with its own domain tag (fix 312c6f59c).
- The record is checked against the key at its account (fix b255b6291).

Key waiting for its backup
- The address and DNS codes are refused (`silent_code`, `faraday/faraday-core/src/silent/mod.rs:1243-1248`).
- "Back up before you receive" stands in for the address (fix 9f01d737c).
- The record code carries the back-up warning.
- `SLabel` is ignored while the key is held (`~1339`).
- The backup sheet carries no QR code of the address; the address stays as text (fix 9dded92c6).
- A record whose address is held or mismatched goes out by no tool and no vault item (fix 97d73414f).

Scan key and descriptor
- They need a loaded key that gives the record's points.
- `SScanVault` needs an open vault; otherwise the toast `silent_no_vault_open`.
- `SAddWallet` needs a record and a named key. A refusal from `add_wallet` is shown as a toast.

DNS fields (`faraday/faraday-core/src/silent/mod.rs:426-432`, within `~415-440`)
- At most 253 characters.
- Control characters are refused.
- Keys are taken only while the address card is open and a field is focused.
- The record is shown only when both fields are valid.

Label count catch-up (`faraday/faraday-core/src/silent/mod.rs:~1460-1615`)
- A count is never lowered.
- Only the wallet with the same policy id is raised.
- A vault being sealed waits.
- A read-only vault is refused and not retried.
- A full refusal is not retried until the vault's version changes.
- Each refusal is shown as a toast once.
- `silent_handed.keep` refuses a count of 0.

Check a payment (`faraday/faraday-core/src/silent/mod.rs:~876-912`, `faraday/faraday-core/src/silent/check.rs`)
- Results:
  - an unsigned input gives Unsigned;
  - no answer while a previous transaction is missing;
  - KeyNotLoaded;
  - NoInputs;
  - NotPaid.
- Labels are searched over 0..=`labels_out` only.
- A file that is not a transaction gives the toast `silent_not_a_transaction`.
- Text with non-ASCII characters is classified, not a crash (fix 3035d5ec5).

Account sheet
- The press is replayed only when the Inbox is unchanged.
- Add to vault works only on a file still in Files (`vault_key_file_gone`).
- `Refusal::Unconfirmed` is held for its sheet.
- A file with a silent payments wallet beside another wallet is refused as holding more than one (see wallets).

Drawing
- The Address row draws the address whole, in groups of four (fix 6e7609df4).

## Domain calls

- `faraday_keys::silent::Receiver`: `address`, `payments_in`, `scan_key`, `descriptor`.
- `faraday_keys`: `Keys::silent_receiver`, `Reveal`, `Part`, `NotSet`.
- `faraday_bip`: `silent::{dns_record, uri, needs_signature_data}`, `silent_wallet`, `policy`.
- `faraday_psbt::transaction::{Spending, is_signed}`.
- `faraday_vault::{Record, records}`.
- `faraday_crypto::work::Compute`.
- `zeroize`.

## Fixtures

Used by `faraday/faraday-core/tests/all/silent.rs`, `faraday/faraday-core/tests/all/home_scan.rs` and `faraday/faraday-core/tests/all/inbox.rs`:

- `faraday/faraday-core/tests/vectors/interop/sparrow/cc-singlesig-sp-keystore-1.json`
- `faraday/faraday-core/tests/vectors/interop/sparrow/keystone-singlesig-sp-keystore-1.txt`
- `faraday/faraday-core/tests/vectors/interop/sparrow/specter-diy-sp-keystore.txt`

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| f7b186d26 | A silent record under another fingerprint over a loaded key's points was added unchecked, with its address shown; an account ≥ 2^31 aborted the app in derivation. | Such a record is Differs (refusal named); an account ≥ 2^31 is refused on read. |
| 312c6f59c | Two silent records over one address with a different fingerprint or account counted the same for the id; lookup took the first. | The id is over the address and origin, with its own domain tag. |
| 97d73414f | The Krux tool and a vault's wallet item could emit a silent record whose address was held or mismatched. | Such a record goes out by no tool or vault item. |
| 9f01d737c | A silent address was shown before its key's backup, unlike a receive address. | The address waits for the backup; "Back up before you receive" stands in; no code or DNS record until then. |
| 9dded92c6 | A held silent wallet's backup sheet carried a QR of its address. | No code while the key waits for backup; the address stays as text. |
| 6e7609df4 | The Address row drew a cut address. | Drawn whole in groups of four. |
| 3035d5ec5 | The silent payment check stopped (the program halted in the text classifier) when the text had non-ASCII characters in the first few positions, for example a Chinese seed's words. | Text is sliced only at character bounds; non-ASCII input is classified, not a crash. |
| b255b6291 | A silent payments record was not checked against its key or its account. | The record is checked against the key at its account; a mismatch is shown. |

## Doubts

- 3035d5ec5 names `core/osk-bip/src/silent.rs` and `osk_codec`, earlier names of `core/faraday-bip` and `core/faraday-codec`.
- Line numbers in `faraday/faraday-core/src/silent/mod.rs`, other than those written plainly, are unconfirmed.
- The receiver cache in `SilentState.shown` is written from drawing paths. That is work done while drawing.
- How BIP-375 silent payment PSBT outputs are refused when signing belongs to signing (`faraday/faraday-core/src/wallet/v2.rs:24-50`). It is not described here.
