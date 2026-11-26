# Addresses: check, find and explore

This feature has two tools for the person at the offline device, reached from Tools, Check and a wallet card.

- **Check an address** (`Screen::CheckAddress`). It takes an address that was typed, pasted, scanned or read from a file. It looks for the address in the loaded wallets and in the standard accounts of the loaded keys, and says whether it belongs to them. It can make the matching wallet, open it, or sign a message with the address. The search is the `address_find` piece.
- **Explore** (`Screen::Explore`). It walks a loaded key's derivation: a path, its script, address, public key and xpub. It lists addresses ten at a time and finds an address among a key's addresses. Behind eyes, it can show the key's seed, its xprv and WIF keys. It can make a single-key wallet from an account.

## Screens and sheets

Check an address (`faraday/faraday-core/src/address_check/{mod,file,pane,screen}.rs`, `faraday/faraday-core/src/address_find/{mod,accounts,card,look,screen}.rs`)
- `Screen::CheckAddress`. The page has:
  - the field;
  - Scan, Paste and Open Files;
  - Add a key (`Action::Entry(None)`) and Load a wallet (`Action::RestoreFor(Check::Address)`);
  - the result card;
  - Another and Done.
- With no wallet and no key loaded, the page shows `load_first`, with the add-key and load-wallet ways.
- `Sheet::AddressFiles`: the files menu.
- `Sheet::Scan` in the pane. On the wide layout the scanner is drawn in the pane (`faraday/faraday-core/src/address_check/pane.rs`).
- `Sheet::MadeWallet`.
- The result card (`faraday/faraday-core/src/address_find/screen.rs`, `faraday/faraday-core/src/address_find/card.rs`):
  - Found (OK role) or Missed (warning role);
  - Make a wallet, Open the wallet and Sign a message;
  - Look further (to 1,000 or 10,000) and Stop.
- A file with several addresses gives a list of rows, each checked.

Explore (`faraday/faraday-core/src/explore/{mod,addresses,screen,pages,shown,make,made,pane}.rs`)
- `Screen::Explore` has three pages: `Path`, `Addresses`, `Find` (`faraday/faraday-core/src/explore/mod.rs:~44`, `faraday/faraday-core/src/explore/addresses.rs:~45`).
- Path page:
  - the loaded key;
  - the path, typed or stepped;
  - presets: BIP-84, 86, 49, 44 and BIP-48 (`PRESETS`, `faraday/faraday-core/src/explore/mod.rs:~34-40`);
  - the script type;
  - the address, public key and xpub;
  - eyes for the seed, the xprv and the WIF.
- Addresses page: receive or change, 10 per page from a typed first index.
- Find page: a depth from `DEPTHS`, a typed or pasted address, and the matches with the matched part marked.
- The Keys pane on the wide layout (`faraday/faraday-core/src/explore/pane.rs:~23-36`).
- `Sheet::MadeWallet` (`faraday/faraday-core/src/explore/made.rs:~18-47`).
- `Sheet::ChildNotice` after making from a BIP-85 child (`faraday/faraday-core/src/explore/mod.rs:~556-560`, `~653-658`).

## Actions

Check an address (`AddressPress`, `faraday/faraday-core/src/address_check/mod.rs:~33-51`)
- `Scan` gives `Action::ScanFor(Ask::Address)`.
- `Paste` sends `Command::RequestClipboard`. It is offered only if `app.clipboard`.
- `Files` runs `file::open_menu`. It is offered only if the Inbox offers files.
- `File(u16)`, `Row(u8)`, `Back`, `Another`.
- `Way(u8)` is one of `Way::Look(LookWay)`, `MakeWallet`, `OpenWallet` or `SignAddress`.
- Typing (`key`, `faraday/faraday-core/src/address_check/mod.rs:~568-599`):
  - printable characters other than whitespace are appended, up to 400;
  - Backspace removes a character and forgets the result;
  - Enter runs `check_text` (empty does nothing);
  - Escape returns from a shown row to the list.
- With exactly one text file waiting, the page reads it at once (`open_tile`, `faraday/faraday-core/src/address_check/mod.rs:~286-293`).

Explore
- `XKey(fp)`, `XPreset(i)`, `XIndex(±1)`, `XScript(i)`.
- `ShowCode(ExploreAddress | ExplorePublicKey | ExploreXpub)`.
- `X(XAction::...)`:
  - `Eye(Eye::Secret | Xprv | Wif | Row)`;
  - `Page`, `Chain`, `Step`, `Depth`;
  - `Search`, `Paste`;
  - `Make`, `MadeDone`, `MadeOpen`.
- `Entry(None)` (Add a key), `Convert(OpenOn(fp))`, `UseSample`.
- Arrow keys step the index (`faraday/faraday-core/src/explore/mod.rs:~321-322`).
- Escape returns to the path (`faraday/faraday-core/src/explore/addresses.rs:~530`).

## Effects

Check an address
- Reads the Inbox; writes nothing to it.
- The clipboard request.
- The camera: `ScanFor`, then `CameraOff`.
- `app.make_wallet_at(&c, ..)`, then `Sheet::MadeWallet` or a toast.
- Derivations run through `Compute`.
- The search is paced by time (`app.now_ms`).

Explore
- Derivations with `Compute`: `xpub_at`, `key_text`, `xprv_at`, `wif_at`, `seed_shown` (`faraday/faraday-core/src/explore/mod.rs:~249`, `~612-616`, `~629`; `faraday/faraday-core/src/explore/shown.rs:~150-176`). Addresses come from `address_find::address`.
- Reveals:
  - `SeedShown`, `ExtendedPrivateKey`, `AddressKey`;
  - `Words` and `Passphrase` per frame (`faraday/faraday-core/src/explore/screen.rs:~474-479`).
- Make (`faraday/faraday-core/src/explore/mod.rs:~641-652`):
  - `session.add_wallet(c, &name, &descriptor, EN.create_make_made_here, Origin::Statement)`;
  - `glance.moved_to`;
  - `wallets.set_wallet(i)`;
  - `trail.set_returning()` (`~579`);
  - `Sheet::MadeWallet` (`~537`).
- The clipboard request (`faraday/faraday-core/src/explore/addresses.rs:~651-656`). The paste is checked with `scan_key::pasted_secret` (`~696`). The search is paced by `now_ms` (`~448-517`).

## State and lock

Check an address
- `AddressCheckArea` (`faraday/faraday-core/src/address_check/mod.rs:~189-196`) holds:
  - `state: Option<AddressCheckState>`;
  - `held_for`.
- `AddressCheckState` holds:
  - `typed: Text` (at most 400);
  - `back`, `refusal`, `look`, `answer`, `ways`, `load_first`;
  - `list: Option<Listed>`;
  - `files_at`.
- `Look` holds:
  - the target script and address;
  - the loaded wallet ids;
  - `accounts`, `looked`, `under_way`, `found`, `found_in`, `made`, `listed`.
- All of this is public data: addresses, scripts and xpub-based policies.
- `AddressCheckArea` is `at_lock::Public` (`faraday/faraday-core/src/address_check/mod.rs:196-199`). The lock walk wipes nothing in it. Every lock still ends in a restart (`faraday/faraday-core/src/vault/seal.rs:933-946`), so the state does not outlive a lock.
- A typed or pasted secret is refused, and the field and result are cleared (see Rules).

Explore
- `ExploreState` (`faraday/faraday-core/src/explore/mod.rs:~44-85`) holds:
  - the key (a fingerprint), the path text, the script;
  - back, page, change, from;
  - `query: String`, depth, `search`;
  - note, sample, made, make_error;
  - `kept: RefCell<Kept>`: public addresses per account, capped at `RUN_KEPT` = 10,000;
  - `reading`: xpub, public key and address, which are public;
  - `shown: Shown`.
- `ExploreArea` also holds `child` and `held_for`.
- `Shown` (`faraday/faraday-core/src/explore/shown.rs:46-59`) holds:
  - `secret: bool`;
  - `seed: Option<Held<Vec<Text>>>`;
  - `xprv: Option<(path, Held<XprivAscii>, Held<XprivAscii>)>`;
  - `wifs: Vec<(path, Held<WifAscii>)>`.
- Every value in `Shown` wipes itself (`faraday/faraday-core/src/explore/shown.rs:44-45`). The words and passphrase are not kept; they are borrowed each frame.
- Lock (`AtLock for ExploreArea`, `faraday/faraday-core/src/explore/mod.rs:664-672`):
  - `holds` is NOTHING. Per its comment, it reads a key of the session's, and the session counts that key.
  - `lock` = `clear()`, which drops the state and `shown`.
- The idle warning (`explore_hide`) drops only `shown` (`faraday/faraday-core/src/explore/mod.rs:386-390`).
- A different key or network resets `shown` (`faraday/faraday-core/src/explore/shown.rs:137-142`).
- `prune` runs after every press and key (`faraday/faraday-core/src/explore/shown.rs:~189-228`):
  - the Path page keeps address keys only for the path explored, and the xprv only for its `xprv_path`;
  - the Addresses page drops the secret, the seed and the xprv, and keeps only the row keys;
  - Find drops all of them.
- Leaving Explore clears the state (`faraday/faraday-core/src/explore/mod.rs:~396-416`), except:
  - when going to Add a key;
  - when going to Convert or Lightning and returning here;
  - while a made wallet's card is open.
- The secrets this feature shows are the seed words, the xprv and the WIF keys, as `Held` values in `Shown`. They go on prune, on a key change, at the idle warning, on leaving and at lock.

## Reach

Check an address
- Uses `scan_key::pasted_secret`, `wallet::classify`, `scan_read::public_kind`, `create::NewKind` (`faraday/faraday-core/src/address_find/accounts.rs:12, 17`), `wallet::{Kind, MessageAt, fp_text, message_wallets}`, `screens::hub_layout` and `message_open_at` / `message_open_path`.
- Opens Add a key, Restore (Load a wallet), Sign a message, the made wallet's card and the wallet.
- Restore returns here (`address_check::reopen`).

Explore
- Uses `wallet::{Session, fp_text, Origin, place_after_removal}`, `session.keys`, `session.wallets`, `convert::state`, `lightning`, `address_find::{address, DEPTHS, Pace}`, `create::NewKind`, `scan_key::pasted_secret`, `samples::put::{Use, offer}`, `edit::Here`, `at_lock`, `pick`, `screens::*` and `compact::*`.
- Is used by: `Faraday::explore_*`; `made_place`, which Check an address uses (`faraday/faraday-core/src/explore/mod.rs:~546-552`); `card_open`; `wallet_left` (`faraday/faraday-core/src/explore/make.rs:~121-132`).
- Explore opens Convert with `explore_hide`.
- My seed makes my wallet uses `explore::{implied_script, parse_path}` and `address_find::{DEPTHS, script}`.

## Rules and refusals

Check an address: file (`faraday/faraday-core/src/address_check/file.rs:159-190`)
- A file of kind `Wallet` gives `EN.address_check_descriptor`.
- A file that is empty, or over `FILE_MAX` (64 KiB), gives `address_check_not_text`.
- A file that is not UTF-8, or is blank, is also not text.
- A secret, per `scan_key::pasted_secret`: `forget()`, the typed text cleared, the refusal shown.
- No address gives `address_check_none_in_file`.
- One address is checked as if typed.
- Several addresses are listed. Each row is checked against the first 100 receive and change addresses of each wallet and standard account (`DEPTH` = 100).
- The sheet is closed (`app.sheet = None`) before reading, even on a refusal.
- The menu offers Wallet and Other files (`offered`), but a Wallet file is then refused. `waiting` offers Text only (`faraday/faraday-core/src/address_check/file.rs:~105-137`).

Check an address: typed or pasted (`faraday/faraday-core/src/address_check/mod.rs:~350-402`)
- Empty gives `pasted_nothing`.
- A secret (`pasted_secret`) is refused, and the field and result are cleared.
- Pasted text over 400 characters is cut in the field. `address_read` and the secret check still run on the full text.
- Text that is not an address gives `address_check_not_bitcoin`, `address_check_not_an_address` or `other.not_here()`, by kind.
- BIP 21 errors: `address_note_uri_required` and `address_note_uri_malformed`.
- Address-like text that fails its checksum gives `address_note_that_address_fails`.
- An address of the wrong network is refused, naming the network it is for and the current one.

Search (`address_find`)
- `DEPTHS = [100, 1_000, 10_000]` (`faraday/faraday-core/src/address_find/mod.rs:32`).
- Look further (`faraday/faraday-core/src/address_find/look.rs:~306-329`):
  - offers only depths past `looked`;
  - only if no look is under way, the address is not found, and the look has not ended.
- Stop keeps the indices already looked at.
- `Pace` aims at 50 ms per tick. The first budget is 4 indices; the maximum is 4096.
- A look ends when the loaded wallets change before a find (`faraday/faraday-core/src/address_find/look.rs:~152-172`). A find keeps its wallet's name (fix 093de8e21).
- Account search:
  - only standard account 0 of native SegWit, Taproot, nested SegWit and Legacy, on the network in use;
  - accounts a loaded wallet already covers are skipped;
  - a key with no origin, or a multi-key wallet, is of no account.
- Matching a loaded wallet to an account compares the parsed master fingerprint, account path, extended key and script, not the descriptor text (fix 420b6f5f1).
- Result card offers:
  - Make a wallet only for an account find, and not for a listed row;
  - Open the wallet not for a listed row;
  - Sign a message only for wallets in `message_wallets()` (a single-key wallet with an origin, change chain included), or for a loaded account key.
- Sign a message is disabled while the key waits for its backup (`sign_waits`, `EN.message_waits_backup`).
- `same_wallet` guards the wallet case.
- Wide pane (`faraday/faraday-core/src/address_check/pane.rs:~102-115`):
  - the scanner shows in the pane only when wide, on page CheckAddress, with sheet Scan, one camera, a scan asking for an address and no read card;
  - leaving the page or opening another sheet ends the scan (`camera.close()`, `Command::CameraOff`), except for a read card kept under a waiting sheet (`read_card_waits`).
- Back from a wallet opened on the Wallet made dialog returns to the page as it was (fix 8b82c5c4e).
- Every address in a file's list, and a typed address, is drawn whole. It wraps only between groups of four (fixes 2798ed131, 4967b2d3e).

Explore: path (`faraday/faraday-core/src/explore/mod.rs`)
- Path text:
  - trimmed, with the `m/` prefix stripped, and `'` becomes `h`;
  - typing takes only digits, `/`, `h`, `'` and `m`, at most 120 characters (`~213-219`, `~313-326`).
- `parse_path` errors (`~437-502`):
  - an empty level;
  - unfinished (a trailing slash);
  - not a number;
  - an index ≥ 2^31;
  - more than 10 levels.
- The index step is clamped to 0..2^31-1 (`~281-299`).
- There is no reading without a key in the session, or when the xpub or path does not derive (`~236-260`).
- A script is implied only by a hardened 44, 49, 84 or 86 first level (`~504-514`).
- `account_of` (`faraday/faraday-core/src/explore/addresses.rs:289-308`):
  - refuses a first level of hardened 45 or 48, and unhardened 45 (multisig);
  - accepts `…/0|1/i` as the account, or an all-hardened path.

Explore: make a wallet
- `make_wallet_at` (`faraday/faraday-core/src/explore/mod.rs:~591-660`):
  - a key not loaded gives `explore_load_a_key`;
  - an empty label uses the fingerprint;
  - the descriptor must parse;
  - it fails closed if the first address the wallet derives differs from the key's own at `account/0/0` (`made_wallet_differs`; no wallet is kept, `~622-633`);
  - the name is cut to `WALLET_NAME_MAX`.
- `make_offer` (`faraday/faraday-core/src/explore/make.rs:~74-85`):
  - only for a standard account: three hardened levels, purpose = the script's, coin type = the network's;
  - only for a loaded key.
- A loaded wallet matches only a single-key template with that script, fingerprint and key path (`faraday/faraday-core/src/explore/make.rs:~39-68`).

Explore: eyes
- With no key, or a key not in the session, an eye does nothing.
- Xprv needs both the root and the account key to derive (`faraday/faraday-core/src/explore/shown.rs:~161-163`).
- Row on a path with no account does nothing (`~168-170`).
- An open eye closes (`~174-175`).
- `Eye::Row` cannot be reached from the Path page.

Explore: addresses and find
- The first index is clamped to 0..`LAST_FROM` (2^31-10). Previous is enabled when from != 0 (`faraday/faraday-core/src/explore/pages.rs:~197-201`).
- Search (`faraday/faraday-core/src/explore/addresses.rs:~423-450`):
  - a search start with no account, or an empty query, does nothing;
  - pressing while it runs stops it.
- Matches (`~497-512`):
  - up to 50 are kept, and the count goes on;
  - a whole match ends the search.
- A depth change drops the search.
- Typing takes ASCII letters and digits, up to `QUERY_MAX` = 90 (`~554-572`). Typed text is not checked for secrets.
- Paste (`~577-594`):
  - reduces BIP-21 to the address;
  - refuses empty (`pasted_nothing`);
  - refuses a secret, with the `pasted_secret` reason;
  - refuses text that is not alphanumeric, or over 90 (`explore_not_an_address`);
  - clears the search even when refused.
- The paste button shows only with a clipboard.
- Search is `PrimaryOff` with an empty query (`faraday/faraday-core/src/explore/pages.rs:~271-274`).
- `explore_pasted` is taken only on Explore, with no sheet, on the Find page (`faraday/faraday-core/src/explore/addresses.rs:~688-699`).
- Make a wallet is offered after a finished search with matches (`faraday/faraday-core/src/explore/pages.rs:~282-284`).
- Every address on these pages is drawn whole, never cut (fix 254930c64).

Explore: screen (`faraday/faraday-core/src/explore/screen.rs`)
- With no key, only the key entry and Add a key show (`~35-80`).
- The Convert button shows only for a session key (`~81-91`).
- The passphrase row shows only when the key has one (`~477-480`).
- The pane shows only on the wide Path page with a session key (`faraday/faraday-core/src/explore/pane.rs:~34-35`).
- The Wallet made dialog shows only if the made index still exists (`faraday/faraday-core/src/explore/made.rs:~18-24`).
- Find an address's field takes typed ASCII alphanumerics only, up to 90 characters, with no secret check (explore/addresses.rs:554-571); a paste is checked for a secret by the caller and refused with its reason, and is refused if over 90 characters or not alphanumeric (addresses.rs:577-594). The query is dropped with Explore's state when the screen is left (explore/mod.rs:413-415) and at lock (explore/mod.rs:669-671).
- `account_of` (explore/addresses.rs:289-309): no account for a first level of 45 (hardened or not) or hardened 48; otherwise the path minus its last two levels where it ends Normal 0|1 then Normal i, whatever the purpose (so 44'/0'/0'/0/5 and m/0/1 alike); or the whole path where it is non-empty and every level is hardened; else none.
- The Files menu offers Text, Wallet and Other kinds, newest first, so that a wallet description is refused by name ("A descriptor, not an address") and a non-text file is said not to be text (address_check/file.rs:105-112, 164-175); the page's own waiting list holds Text files only (file.rs:127-137).
A look ends when the loaded wallets are no longer the ones it began in: a different count, or a different `WalletId` (the policy's descriptor hash) at any place. The indices already looked at are kept as looked, the look runs no further, and a find already made stays (faraday/faraday-core/src/address_find/look.rs:154-172).

## Domain calls

Check an address and the search
- `faraday_codec::{bip21::parse, address_text::find}`.
- `faraday_bip::{bitcoin::{Address, ScriptBuf}, policy::{WalletPolicy::parse_any, find_address}, address::is_address_prefix, keys}`.
- `faraday_keys`: `session.keys.key_text`, `position`.
- `faraday_text::Text`.
- `faraday_shell_api::{Command, Key, FileKind}`.

Explore
- `faraday-bip`: `bip32`, `Address::p2*`, `secp256k1`, `ScriptType`, `WalletPolicy::{parse_any, from_xpub}`.
- `faraday-keys`: `Keys`, `Held`, `Reveal`, `WifAscii`, `XprivAscii`.
- `faraday-codec`: `bip21::parse`.
- `faraday-intake`: `wallet::{cut_to, WALLET_NAME_MAX}`.
- `faraday-strings`: `bip::policy_error`.

## Fixtures

None. Explore's sample comes from `crate::samples::put`.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 2798ed131 | Each address in a file's list was cut short. | Never cut; wraps only between groups of four. |
| 4967b2d3e | A typed address in Check an address and Explore's Find was not shown whole. | The field grows to the lines needed; the caret follows. |
| 8b82c5c4e | Back from a wallet opened on the Wallet made dialog did not return to the page Make a wallet was pressed on (Explore's path page or Check an address's page). | Back returns to the page as it was. |
| 093de8e21 | An address look kept searching a loaded wallet from its start when the loaded wallets changed; a refresh read a found wallet's name by place, so it could show the wrong name. | A look ends when the loaded wallets change; a find keeps its wallet's name. |
| 420b6f5f1 | The check matched a loaded wallet to a key's account by descriptor text, so the same account written with h or another checksum was missed; a wallet naming a key's origin over another extended key hid the key's own account. | Matching compares the parsed master fingerprint, account path, extended key and script. |
| 254930c64 | The Addresses and Find pages cut long addresses; a found address's matching part was not shown. | Every address there is drawn whole, wrapped where it does not fit; the matched part is marked. |

## Doubts

- Rows left out as not about this feature:
  - def14f1d9 and 09a325d4b (Decode and Check a signer);
  - e1063a469 (Your choices, Sign a message and Spend);
  - 893a358af (Spend review rows).
- `explore_reading` and `explore_list` can run a first derivation during a frame. That is work done while drawing.
