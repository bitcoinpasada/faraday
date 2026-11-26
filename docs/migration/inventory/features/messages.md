# Signing and checking messages

Sign a message with a single-key wallet's receive address (BIP-322 or
BIP-137), and Check a signed message brought in as a file, pasted or as the
Demo sample. It is for the person proving they control an address, or
checking someone else's proof. Sign a message (`Screen::Message`) is reached
from the Tools tile SignMessage and from Wallets; Check a signed message
(`Screen::CheckMessage`) from the Tools tile CheckMessage and the Check
page's Messages group.

## Screens and sheets

Sign a message (`faraday/faraday-core/src/screens/message.rs`):
- Four steps: address, text, format, signature (`MSTEPS`,
  `faraday/faraday-core/src/screens/message.rs:~16-21`). Back goes to
  `Screen::WalletsStart`.
- Address step: wallet picks; address buttons, `-` and `+` index buttons
  only while no address is picked
  (`faraday/faraday-core/src/screens/message.rs:~316-335`). Address buttons
  of wallets waiting on a backup are disabled (`~265-267`); a picked
  address whose key waits is shown as the waiting line (`~66-72`,
  `~285-287`).
- Text step: typed or scanned message. The whole message is shown, each
  invisible character drawn as a marked glyph. The Type and Scan row's
  height is counted where Continue is pinned.
- Format step: BIP-322 or BIP-137; a format the script does not allow is
  disabled (`Format::for_script`,
  `faraday/faraday-core/src/screens/message.rs:~439-453`).
- Signature step: the signature, Show as QR, a PNG code, the file for the
  stick, Check it (`CheckMessageMade`), Another, Done. File and QR/PNG
  buttons only when the pane does not stand left (`~508-546`).

Check a signed message (`faraday/faraday-core/src/screens/check_message.rs`):
- Steps PICK (0) and RESULT (1); the flow card only on a wide window
  (`faraday/faraday-core/src/screens/check_message.rs:~50`).
- Landing: only `FileKind::Message` files offered (`~135`, `~154`).
- Result: the verdict (BIP-322 or BIP-137 by `faraday_psbt::message::Format`,
  or the error text, `~241-257`), the file name, the signing address, the
  matched wallet, the message text. The wallet line only if the signing
  address matches a wallet, else `check_not_an_address` (`~297-308`).

## Actions

Sign a message: `MWallet`, `MIndex(+-1)`, `TypeHere(Here::Message)`,
`MType`, `MScan`, `MNext`, `MFormat(bool)`, `MSign`, `MQr`,
`CodePng(Code::Message)`, `MOut`, `CheckMessageMade`, `SignMessage`
(Another), `MDone`, `MStep(k)`.
- Choosing a wallet moves to the address step; choosing a format moves on
  (`faraday/faraday-core/src/screens/message.rs:~50-57`).
- Any change before the signature drops the signature (`~167-170`).

Check a signed message:
- Opening from Tools (`open_tile`,
  `faraday/faraday-core/src/screens/check_message.rs:~18-33`): one message
  in Files is checked at once (`Action::CheckMessage(k)`); none, with
  Practice on, opens the sample (`check_use_sample()`); else the landing.
- `CheckMessageAnother`; `OpenWallet(i)` only when a wallet matched
  (`~342`).

## Effects

- `MOut` puts the signed message in the Outbox as
  `message-<last 6 characters of the address>.txt`
  (`faraday/faraday-core/src/screens/message.rs:511-514`). It keeps its
  own name when an Outbox item with the same name and hash exists, else a
  free name (`~192-200`).
- Show as QR and a PNG code (`CodePng(Code::Message)`).
- Camera for Scan (`MScan`).
- Costly work: `keys.sign_message`, `keys.xpub_at` through `Compute`.
- The message check result is cached in the drawing cache
  (`Kept.message_checked`, `faraday/faraday-core/src/drawing/kept.rs`),
  worked again when the text, network or wallet ids change
  (`faraday/faraday-core/src/drawing/kept.rs:~449-455`).

## State and lock

- `MessageState` (`faraday/faraday-core/src/state.rs:1049-1090`): the open
  card, `done: [bool; 4]`, scroll, `wallet` (index into the session),
  `of_key` (opened from a key's details), `index`, `at` (an address of a
  loaded key found by the address check, signed for instead of a receive
  address), `shown` (the receive address worked out when wallet or number
  changes; drawing only reads it), `text: Text` (the message), `typing`,
  `bip322`, `signed: Option<faraday_psbt::message::Signed>`, `signer`
  (fingerprint and `KeyId`, public), `out_name`, `out_refused`, `out_sha`.
- `MessageArea` (`faraday/faraday-core/src/state.rs:1098-1110`): `state`,
  `checking` (the Inbox file being checked), `made` (a signed message just
  made, checked without being filed: name and text), `check_back`, `shown`,
  the Check flow card's scroll.
- Lock (`AtLock for MessageArea`,
  `faraday/faraday-core/src/state.rs:2783-2791`): `holds` is NOTHING (the
  comment: "A message to sign and its signature are public; a lock drops
  them"); `lock` is `clear()`. The area is in `lock_areas`
  (`faraday/faraday-core/src/at_lock.rs:188`). A lock then restarts the
  process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- A message that is itself a secret (`out_refused`) is held in the plain
  `text: Text`, which wipes on drop (`core/faraday-text/src/text.rs:363-371`),
  and counts as nothing held at a lock.
- No key is kept by the feature: signing uses the session's loaded key
  (`faraday-keys`), wiped at `Session::wipe`
  (`faraday/faraday-core/src/wallet.rs:1539-1549`).
- `Kept.message_checked` is not cleared at lock
  (`faraday/faraday-core/src/drawing/kept.rs:1267-1277`); it holds a public
  verdict.
- What `MessageArea::clear` resets field by field was not read; leaving the
  screen was not read.

## Reach

- Opens: the scanner, the Outbox (via Files), `Screen::CheckMessage` from
  the signature step, a wallet (`OpenWallet`).
- Opened from: `tools` (SignMessage, CheckMessage tiles), `check-page`
  (Message row), Wallets (`faraday/faraday-core/src/screens/wallets/actions.rs`).
- Reads: the session's wallets and keys; Files (`FileKind::Message`).

## Rules and refusals

Signing (`Session::sign_message`,
`faraday/faraday-core/src/wallet.rs:4295-4345`):
- an unknown wallet: `wallet_no_such_wallet`;
- not a single-key wallet: `wallet_messages_are`;
- the first key without fingerprint or origin, or not loaded: refused;
- signs only on the receive chain (child 0, then the index);
- the signed address must equal the wallet's receive address at the index,
  else `wallet_not_your_key`.
- `sign_message_at` (`faraday/faraday-core/src/wallet.rs:~4347-4385`) for a
  found address: `message_key_not_loaded`, `message_key_cannot_derive`,
  `message_key_not_this_address`.
- `message_wallets` (`~4277-4291`): single-key wallets whose first key is
  loaded.
- Wallet picks (`faraday/faraday-core/src/screens/message.rs:134-152`):
  hidden unless `message_lists(i)`; usable with a key waiting for backup is
  refused `message_waits_backup`; usable is taken with its shape as a note;
  single-key not held: `message_key_not_held`; any other:
  `message_single_key_only`.
- A key waiting for its backup signs nothing; Sign a message is not
  offered for such a wallet. The Tools tile: no single-key wallet gives
  `catalog_needs_a_single`; every offered wallet waiting for its key's
  backup gives `message_waits_backup`
  (`faraday/faraday-core/src/catalog.rs:617-658`).
- Sign is withheld while waiting on a backup, or when no wallet is chosen
  though wallets exist (`faraday/faraday-core/src/screens/message.rs:~475-486`).
- A message is signed in a format the chosen wallet can sign in; a nested
  or legacy wallet does not open on BIP-322 greyed.
- A typed message that is itself a seed, key or share: no file is made
  (`out_refused`, code only), no "waits" toast, the scanner's refusal
  wording; the QR stays (`~509`, `~181-186`). No take-away without
  `out_name`, `file_name` or `out_sha` (`~187-191`).
- A transaction that is a BIP-322 request (only output a 0-value
  OP_RETURN) is blocked on Sign a transaction and sent here.

Checking (`check_message`, `faraday/faraday-core/src/wallet.rs:~4388-4411`):
- `None` if the text does not parse;
- the address must parse and be for the loaded network;
- the address is looked for among each wallet's receive and change
  addresses at indexes 0..=100;
- the verdict is `faraday_psbt::message::verify`'s.
- Not a signed message: refused with the network's name
  (`faraday/faraday-core/src/screens/check_message.rs:~232-240`). A non-UTF-8
  file is checked as `from_utf8_lossy` (`~232`).
- The Tools tile: no Message file and nothing to paste gives
  `catalog_copy_a_signed` (`faraday/faraday-core/src/catalog.rs:617-658`).
- Reading a signed-message file (`core/faraday-psbt/src/message.rs`): a
  signature is one base64 token, else the file is a text file (a seed under
  an address title is not a signed message)
  (`core/faraday-psbt/src/message.rs:~444`). A BIP-322 signature with a
  variant prefix (smp, ful, pof) on line 2 is split as verify splits it
  (`core/faraday-psbt/src/message.rs:~217-228`); the full variant is read
  and stated as unsupported (`~126`).
- A BIP-322 witness signature that is not strict BIP-66 DER is Malformed,
  as Bitcoin Core reports it (`strict_der`,
  `core/faraday-psbt/src/message.rs:~598-600`, `~651-655`). A 65-byte taproot signature ending 0x00 is
  malformed.
- The index steppers' labels "-" and "+" (screens/message.rs:322, 331) and the signed message's file name `message-<last 6 characters of the address>.txt` (message.rs:511-514) are literals; the strings lint allows both (no letter; a file name with an extension).
A change address can be signed when Sign a message is opened from an address check's find: a loaded key's account address on the change chain, or a single-key wallet's change address (`MessageAt` with `change` true, signed at its full path); Sign a message's own wallet and index choice signs only the receive chain, and no signing is offered for a multi-key wallet's address (faraday/faraday-core/src/address_find/look.rs:175-257, faraday/faraday-core/src/wallet.rs:4295-4385, faraday/faraday-core/src/state.rs:2039-2058).

## Domain calls

- `faraday_psbt::message::{parse_signed, verify, Format}`
  (`core/faraday-psbt/src/message.rs`).
- `faraday_keys` `Keys::sign_message`, `Keys::xpub_at`
  (`faraday/faraday-keys/src/session.rs`), through
  `faraday_crypto::work::Compute`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| aee93f731 | A seed file with an address title over seed words was read as a signed message | A signature is one base64 token; else a text file |
| e329d6ff1 | A one-word-per-line seed under an address title read as a signed message | Same |
| 315094b62 | A BIP-322 signature with a variant prefix on line 2 was refused by the file parser though verify reads it | Prefix split as verify splits it; full variant read, stated unsupported |
| 0074b6d25 | A BIP-322 full-variant signature got a "cannot sign for the address" format error | Its own refusal, stated as unsupported |
| 2b1a9b266 | A typed message that is itself a seed, key or share was offered a file | No file, no "waits" toast, the scanner's refusal wording; QR stays |
| 4f5e3bdd8 | Sign a message was offered for a wallet waiting for its key's backup | A key waiting for backup signs nothing; not offered |
| 38a890f28 | Sign a message did not show the whole message; characters that draw nothing were invisible | Whole message shown; each invisible character drawn as a marked glyph |
| 966560ab2 | At 480x640 the Type and Scan row ran into the About line where Continue is pinned | The row's height is counted in the pinned layout |
| 4598a2dca | Sign a message opened on BIP-322 greyed for a nested or legacy wallet (one of three faults in the commit) | A message is signed in a format the chosen wallet can sign in |
| 7d0e9cdd7 | BIP-322 verification accepted a witness signature that is not strict BIP-66 DER | Refused as Malformed, as Core does |

## Doubts

- No fix row was given to this feature directly. Every row above was given
  to another feature and placed here as it concerns messages: aee93f731,
  e329d6ff1, 315094b62, 0074b6d25, 2b1a9b266, 4f5e3bdd8, 38a890f28,
  966560ab2 and 4598a2dca from `spend-signing`; 7d0e9cdd7 from
  `check-page`.
- The State and lock lines for `MessageState` and `MessageArea` were read
  from the code by the inventory, not by its earlier accounts.
- `expect` at `faraday/faraday-core/src/screens/message.rs:~249` relies on
  the dispatcher checking first.
- 38a890f28 also changes fonts and `faraday/faraday-core/src/scan_key.rs`;
  4598a2dca's other faults are Vanity and Units (see `tools`).
