# Child keys (BIP-85)

Child keys derives BIP-85 children from a loaded key: new seed words, hex,
Base64 or Base85 passwords, a WIF key or an xprv, at an index. It is for a
person who wants a separate secret that can always be made again from the
parent key. The value can be saved into the open vault, loaded as a key
(words only), or written out unprotected through the secret sheet. It is
reached from Tools (`Action::Bip85`) as `Screen::Bip85`, a step flow of four
cards.

## Screens and sheets

- `Screen::Bip85` (`draw`, `faraday/faraday-core/src/bip85/screen.rs:~24`). Four step cards `TITLES` (`~17-22`): Key, Application, Index, Value.
- Key card: every loaded key as a pick; Next shown once a key is picked (`faraday/faraday-core/src/bip85/screen.rs:~112-118`).
- Application card: six applications (`Bip85App`, `faraday/faraday-core/src/bip85/mod.rs:~25-38`): Words, Hex, Base64, Base85, Wif, Xprv; a length where the application has one.
- Index card: the index field; an invalid index shows `bip85_index_limits` in red (`faraday/faraday-core/src/bip85/screen.rs:~230-233`).
- Value card: the value behind an eye (`vault::screen::eye_at`, `faraday/faraday-core/src/vault/screen.rs:210`), Save to vault, Load as a key, Write out, Another; a vault way when no vault is open (`faraday/faraday-core/src/bip85/screen.rs:~302-313`).
- Sheet `Sheet::ChildRecord`, "Record it as a child seed?" (`faraday/faraday-core/src/bip85/mod.rs:~558-561`; drawn by `record_dialog`, `faraday/faraday-core/src/bip85/screen.rs:~346`).
- The secret sheet for writing out, with `bip85_secret_warning`.

## Actions

- Open: `Action::Bip85` (`faraday/faraday-core/src/bip85/mod.rs:~749`); opens on the loaded key when there is exactly one (`~535-537`). `Action::PAnother` restarts with the same key (`~750-754`).
- Key card: `PKey(fp)` picks a key (`~445-449`); `PNext` continues and needs a key (`~469-481`).
- Application card: `PApp(i)` resets the length to the application's default (`~450-456`); `PLength(+-1)` (`457-463`).
- Index card: `PIndex(+-1)`; typed digits and Backspace (`509-520`); `PNext`.
- Value card: `PShow`, the eye (`~482`, `~759-766`); `PVault`, save into the open vault (`~755`, `~580-634`); `PLoad`, load child words as a key (`~756`, `~638-647`); `PRecord(bool)` answers the child-seed question (`~757`, `~565-575`); `POut`, write out unprotected (`~758`, `~684-743`); `PAnother`; `Nav(back)` (`faraday/faraday-core/src/bip85/screen.rs:~334`).
- Any card: `PStep(k)` opens a reached card, only when every earlier card is done (`faraday/faraday-core/src/bip85/mod.rs:~433-438`).
- Keys: Enter presses `PNext` (`~508`); on a focused control other than the index field, Enter presses that control (`~774-784`).
- Other actions offered: `Action::Entry` (Add a key, the "Add a key" way in, `~790-796`), `Action::CreateWallet`, `Action::ExploreKey(fp)`, `Action::TypeHere(Here::Index)`.

## Effects

- Derivation: `Keys::bip85_child(c, place, app, i, Reveal::Bip85Child)` is the only place the value is derived (`faraday/faraday-core/src/bip85/mod.rs:~360`); `value` takes `Compute` (`~344-361`); `self.compute()` (`~597`, `~653`, `~685`, `~763`).
- Vault write: `vault_push(record, ...)` into `self.vaults.open` (`~581`, `~598`, `~633`).
- Session keys: `session.keys.add_words(&c, &value, "", None)` (`~660`), `note_child` (`~663-665`), `set_child_of` (`~668`).
- Write out: `offer_secret(SecretOut::new(...))` (`~695`, `~737`; `offer_secret` at `faraday/faraday-core/src/secrets.rs:304`); file name from `outbox_free_name` (`faraday/faraday-core/src/bip85/mod.rs:94`, `709`; `faraday/faraday-core/src/stick/visit_taken.rs:114`). File bytes are `Zeroizing`.
- Toasts: `bip85_no_vault_is_open` (`faraday/faraday-core/src/bip85/mod.rs:~582`, `~599`); the load error's text (`~675`).
- No printing, camera, clipboard, time or randomness.

## State and lock

- `Bip85State` (`faraday/faraday-core/src/bip85/mod.rs:~161-186`): open card, done flags, scroll, key fingerprint, application, length, index as `Text`, `shown`, `back: Screen`, `asking: Option<ChildTo>`, `loaded: Option<[u8;4]>`. No secret: the value is derived when needed.
- `Bip85Area` (`~225-231`): `state: Option<Bip85State>`, `held: Option<faraday_keys::Held<Text>>`, the shown child value (a wiping type).
- `held` is dropped on hide (`hide_secrets`, `~240-246`), on leaving a card while not shown (`tidy`, `~277-282`), on `open` (`~320`), and on `clear` (`249-252`).
- Lock (`faraday/faraday-core/src/bip85/mod.rs:900-907`): `lock()` = `clear()` drops both the state and `held`. `holds()` is `Holds::typed(held.is_some())`: only a shown child counts as held for the lock prompt; an open flow with the eye shut holds nothing but is still cleared. The lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Leaving the screen does not clear `state`; only hiding or leaving the card drops `held`.
- A child loaded as a key lives on in `session.keys` like any key.

## Reach

- Opened from Tools (`Faraday::bip85`, `bip85_open`, `faraday/faraday-core/src/bip85/mod.rs:~530-554`).
- Opens Add a key, Make a new wallet, Explore (a key).
- Reads `session.keys` (`~536`, `~553`, `~660-668`), `vaults.open` and `vaults.current` (`~581`, `~598`, `~302`), `wallet::fp_text`, `wallet::parse_words` (`~613`), `makes_secrets()` (`~550`, `~585`, `~644`), `may_load_keys()` (`~641`, `~654`), `app.vault_way(Screen::Bip85)` (`faraday/faraday-core/src/bip85/screen.rs:~310`), `app.named_back` (`~33`).
- Uses `pick::{Decl, Take, PickRow}`, `flow`, `trail::{StepFlow, Steps}` (`BIP85_STEPS`, `faraday/faraday-core/src/bip85/mod.rs:~817`), `visible::choice_row` (`~867`), `at_lock`, `compact::{M, dialog, screen::finish}`, `panes::screen`.
- Gives to other screens: `key_name`, `child_mark`, `record_child_mark` (how a key recorded as a child seed is named, for example "BIP-85 Words . <fp> . index 0", `~113-143`; used by Home's loaded box), `out_title` and `kdbx_out` (title and file name rules for a BIP-85 password, `~72-96`), `pick_of` (`~800`).

## Rules and refusals

- Index: parses as u32 and at most `bip85::MAX_INDEX` (`faraday/faraday-core/src/bip85/mod.rs:~202-207`); typed ASCII digits only, at most 10 characters, a leading "0" replaced by the first digit (`509-516`); `PIndex` clamps to 0..=MAX_INDEX (`~466`); Next on Index is refused when invalid (`~471-473`).
- Key card: Next is refused without a key. Every loaded key is offered.
- Practice: no value is derived when `makes_secrets()` is false (`548-554`); `bip85_load` does nothing (`644`); `bip85_to_vault` skips the child-seed question (`585`).
- Steps: a card opens only when all earlier cards are done (`~436-438`); a press on the open step does nothing; the value is hidden when its card is left (`~441-443`); an application with no length (Wif, Xprv) goes straight to the next step (`~845-849`).
- Lengths (`lengths()`, `189-197`): Words 12 to 24 in steps of 3 (12, 15, 18, 21, 24), default 24; Hex, Base64 and Base85 ranges from `bip85::HEX_BYTES`, `BASE64_LENGTHS`, `BASE85_LENGTHS`, defaults 32, 21 and 12. `PLength` clamps to the range (`457-463`).
- The shown value is hidden on any change: key, application, length, index, typing (`~413-416`, `~447-467`, `~516`, `~520`).
- Save to vault: refused with the toast `bip85_no_vault_is_open` when no vault is open (`580-583`, `~597-600`). Child words ask "Record it as a child seed?" first when `child_of()` is Some and `makes_secrets()` (`~585-587`); `KEY_CHILD` is recorded only on yes (`~617-625`). A words parse failure or a missing value returns with no note (`~602-604`, `~613-615`).
- Load: gated by `may_load_keys()` (with a stick in, it waits under the Pull sheet) and `makes_secrets()`; the question is asked only when `child_of()` is Some (`641-647`).
- `child_of()` is None unless the application is Words, a key is chosen, the length fits a u8 and the index is valid (`~366-377`).
- The record question acts only when the open sheet is `ChildRecord` (`~566-568`); the dialog draws only with a `ChildTo` and `child_of` (`faraday/faraday-core/src/bip85/screen.rs:~349-354`).
- Write out (all through the secret sheet with `bip85_secret_warning`, `faraday/faraday-core/src/bip85/mod.rs:~695-700`, `~737-742`):
  - Words: `<fp>-words.txt` holding only the words, named by the child's own fingerprint, with nothing that says its parent (`~689-702`, decision F166).
  - Password: `<password name>.txt`, unique in the Outbox, the value alone (`~703-713`, decision F172).
  - Other applications: `bip85-<fp>-<app>-<index>.txt` holding the path and the value (`~715-729`).
- Password title: BIP-85 password entries are named "Password" out of the vault (`~72-82`); a KeePass export uses the generic `kdbx_export_file` name beside any Outbox file (`~89-96`).
- Enter in the flow is always consumed (`~508`, `~524`), even when Next is refused.
- Published test words (BIP-85 vectors among them) raise the published-sample caution on Mainnet too (fix row 3f6f8a738).

## Domain calls

- `core/faraday-bip`: `bip85` constants (`MAX_INDEX`, `HEX_BYTES`, `BASE64_LENGTHS`, `BASE85_LENGTHS`), `bip39::Language`, `keys::Fingerprint`.
- `faraday/faraday-keys`: `Keys::{bip85_child, position_of, add_words, note_child, set_child_of, key_label}`, `Held`, `Reveal::Bip85Child`, `KeyInfo`, `identity::fingerprint_of_words`, `vault::RecordView`.
- `faraday/faraday-vault`: `Record`, `records::{words_payload, field, kind}`, `child::{ChildOf, child_of, payload}` (fields `KEY_CHILD`, `KEY`, `PASSWORD`, `NOTES`, `TITLE`, `NOTE`).
- `core/faraday-crypto`: `work::Compute`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 3f6f8a738 | Published test vectors (codex32, BIP-32, BIP-85) on Mainnet did not bring the caution. | Published test words raise the caution on Mainnet too. |

## Doubts

- `bip85_to_vault_as` returns with no note on a parse failure or a missing value.
- `marked_online` (`faraday/faraday-core/src/bip85/mod.rs:~626`) was not read.
