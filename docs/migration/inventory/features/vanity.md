# Vanity address search

Vanity address search looks for an address of a loaded key that begins with
chosen characters. It turns one of two dials: the account number of the
key's wallet, or a suffix added to the key's BIP-39 passphrase. It is for a
person who wants a recognisable receive address or silent payment code. The
search runs on the main thread in small budgets per tick. It is reached from
Tools (`Screen::Vanity`) and from Explore on a key (`OpenOn(fp)`).

## Screens and sheets

- `Screen::Vanity` (`faraday/faraday-core/src/vanity/mod.rs`, `faraday/faraday-core/src/vanity/screen.rs`): five step cards: Key, Dial, Kind of address, Characters, Search.
- Grinder steps (`faraday/faraday-core/src/grind.rs`): Script, Prefix, Running, Found, Secret (the found passphrase suffix, masked like a passphrase).
- Search card: rate while running; on a find, the address, its code (`ShowCode(Code::VanityAddress)`), the account or the suffix behind an eye, Use, Another, Done.
- A key waiting for backup: the found address is not shown and the code is not offered (`faraday/faraday-core/src/vanity/screen.rs:~271-281`, `~85-93`).
- The suffix shows only when `shown`, else as bullets (`faraday/faraday-core/src/vanity/screen.rs:~299-303`).

## Actions

`VanityAction`:
- `Open` opens on the only loaded key (`faraday/faraday-core/src/vanity/mod.rs:~387-392`); `OpenOn(fp)` hides Explore's eyes first (`~410-414`; `explore_hide` at `414`).
- `Key(fp)`, `Dial(i)` (Account or Passphrase), `Script(i)` (legacy, nested, native segwit, taproot, silent payment code).
- Typed characters, Backspace, Enter (Enter is `Start`) on the Characters card; `TypeHere(Here::Prefix)`.
- `Start`, `Stop`, `Show` (the passphrase suffix of a find; Passphrase dial only), `Use`, `Another`, `Done`, `Step(k)`, `Next(k)`.
- `KeyGen(None)` (add a key), `ShowCode(Code::VanityAddress)`, `Nav(v.back)`.
- `Use`:
  - Account dial: the key's own wallet at the found account.
  - Passphrase dial: a new key with the suffix added to the passphrase, then its wallet.
  - Silent payment code: opens the Silent payments tool on that key (`silent_open_for`, `faraday/faraday-core/src/vanity/mod.rs:518`), no wallet.
  - The wallet is named "<key label> · <first 10 characters of the address>" (`~542`).

## Effects

- Per tick: `keys.vanity_grind(c, key, dial, script, prefix, cursor, budget)` (`faraday/faraday-core/src/vanity/mod.rs:~315`), then `out.redraw()` / `Command::Draw` (`~338-339`). `now`/`now_ms` only for the rate (`~278`, `~300`, `~329`). Tick `TICK_MS = 100`; budget from `FIRST_BUDGET = 8` up to `MAX_BUDGET = 8192` (`faraday/faraday-core/src/grind.rs`). After a find the next search starts at `counter + 1`.
- `Use`: `session.keys.add_with_suffix(..)` (`~484`), `mark_made_here(fp)` (`~494`), `key_text(.., Spec::Single(script, account))` (`~522-526`), `session.add_wallet` (`~543`), `refresh_spend` (`~551`), `wallets.set_wallet` (`~553`), toasts (`~498`, `~532`, `~555`, `~557`).
- No randomness, no worker thread, no files.

## State and lock

- `VanityArea.state: Option<VanityState>` (`faraday/faraday-core/src/vanity/mod.rs:~74-93`, `~121-123`): `open`, `done[5]`, `scroll`, `key: Option<[u8;4]>`, `grind: Grind` (the typed prefix and the `find`: `address`, `suffix`, a BIP-39 passphrase fragment and so a secret, and `account`), `running`, `shown`, `back`, `error`.
- Lifetime: from `Open`/`OpenOn` until `Done`, `Another` (which reopens), a successful `Use` (`~552`, `~517`), leaving the screen, or a lock.
- Leaving the screen clears it: `after_input` calls `vanity.clear()` unless the vanity screen, or Add a key or New key opened from it, or Files for its key's stick visit is up (`faraday/faraday-core/src/pointer.rs:814`, `860-863`; `faraday/faraday-core/src/vanity/mod.rs:147-149`). The suffix is a `Counter` wiped on drop (`core/faraday-bip/src/vanity.rs:137`). `hide_shown` only sets `shown = false` while the screen stays (`faraday/faraday-core/src/vanity/mod.rs:127-129`).
- Lock: `AtLock for VanityArea` (`faraday/faraday-core/src/vanity/mod.rs:650-659`): `holds = Holds::typed(state.is_some())`, `lock = clear()`, which drops the `Grind` and its find. `Text` wipes on drop (`core/faraday-text/src/text.rs:363-367`). The lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- A key made by `Use` lives on in `session.keys`, marked made here (it must be backed up before it signs).

## Reach

- Opened from Tools and from Explore (`OpenOn`).
- Opens Add a key / New key (`KeyGen(None)`), the Silent payments tool (`silent_open_for`), the QR code sheet, Wallets (after Use).
- Reads `session.keys` (`position_of`, `position`, `has_words`, `label`, `fingerprint`), `makes_secrets()`.

## Rules and refusals

- `Step(s)` opens a card only if every earlier card is done (`faraday/faraday-core/src/vanity/mod.rs:231-233`).
- `Next(PREFIX)` is refused unless the prefix has a free position for the kind: `vanity_type_at_least` (`236-239`).
- `Dial(Passphrase)` is refused for a key without words, or in Practice (`makes_secrets` false): `vanity_this_key_was` (`253-258`; words test `~437`, `~565`).
- `Start` on the Passphrase dial without words: refused with no note (`~273-275`). Without a free prefix position: `vanity_type_at_least` (`~276-285`).
- `Prefix::push` refuses a non-ASCII character, a full prefix, and any character `vanity::can_begin` does not allow (`faraday/faraday-core/src/grind.rs:109-123`); the screen shows `vanity_cannot_stand` (`faraday/faraday-core/src/vanity/mod.rs:~361-363`).
- Typing is taken only on the Characters card and not while running (`~361`, `~368`).
- `Stop` returns to Characters if nothing was found (`~287-292`).
- Search exhausted: `vanity_every_counter` (`~334-337`).
- `OpenOn` only for a loaded key (`~410-411`).
- `Use` on the Passphrase dial is refused with no note for a key without words (`~481-483`); an `add_with_suffix` error is a toast (`~497-500`).
- `still_fits` (`~635-643`): any earlier change drops a find; a change of kind drops the characters.
- A key waiting for backup: the found address and code are hidden; `Use` is still offered (`faraday/faraday-core/src/vanity/screen.rs:~271-281`, `~85-93`).

## Domain calls

- `core/faraday-bip`: `vanity::{Kind, fixed_prefix, can_begin}`, `keys::{Fingerprint, ScriptType, Network}`.
- `faraday/faraday-keys`: `Keys::{position_of, position, vanity_grind, add_with_suffix, key_text, has_words, label, fingerprint}`, `Dial`, `Spec::Single`.

## Fixtures

None.

## Fixes

None.

## Doubts

- `Use` on the Account dial is offered for a key waiting for backup, though the address and code are hidden; there is no held-key check on that path.
- `Use` on the Passphrase dial adds the key before the later silent-payments branch.
