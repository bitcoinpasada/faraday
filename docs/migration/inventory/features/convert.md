# Convert a seed

Convert a seed writes a loaded key (or a key in an open vault, or an LND
aezeed) in another form: SeedQR, Compact SeedQR, word numbers, Seed XOR
parts, codex32 strings, SLIP-39 shares, the seed in hex, an xprv, and more.
It is for a person who wants another backup form of a key they hold, or to
change a SLIP-39 set (add, replace or rotate shares). It is reached from
Tools (back name Tools, `Screen::Catalog`) and from Explore on a key
(`ConvertAction::OpenOn(fp)`). It is a step flow on `Screen::Convert`.

## Screens and sheets

- `Screen::Convert`: cards From, To, Settings (only for targets with settings), Result (`ConvertSteps`, `faraday/faraday-core/src/convert/mod.rs:~1834-1911`; plugs into `trail::StepFlow` and `flow::Contained`). Back to `Screen::Catalog` (`faraday/faraday-core/src/convert/screen.rs:~33-37`).
- From card: loaded keys as a pick; open vault keys (`ConvertAction::VaultKey`); Add a key; an "Other forms" toggle; a SLIP-39 set source; an LND aezeed (`ways_in::CONVERT_FROM`, `faraday/faraday-core/src/convert/screen.rs:~166-179`).
- To card: 17 rows (`ROWS`, `faraday/faraday-core/src/convert/mod.rs:~352-455`); rows that do not apply read "not offered" with a reason line.
- Settings card: Seed XOR parts; codex32 k and n; SLIP-39 groups, groups needed, and per group the shares needed and made; Make.
- Result card: the value behind an eye; part buttons; facts (Standard, Restores in, Fingerprint, Word list, Paths, the aezeed's Birthday and Channels, Passphrase); Check my copy; Compare; Clear; Save; Another; Done. A refused make shows the refusal line.
- Leaves for `Screen::Lightning` (aezeed read, then back, `faraday/faraday-core/src/convert/mod.rs:~1665`, `~1818-1821`), Add a key (`Action::Entry(None)`), `Sheet::Scan` (Check my copy by camera, `~1265`), and the secret sheet (`app.offer_secret`, `~1631`).

## Actions

`ConvertAction`:
- `Open` (a key is chosen at once when exactly one is loaded, `faraday/faraday-core/src/convert/mod.rs:~1667-1672`); `OpenOn([u8;4])` from Explore, which calls `app.explore_hide()` (`1683`; `~1673-1686`).
- `Step(u8)`, `Key`, `VaultKey(v, r)`, `Aezeed`, `OtherForms`, `To(u8)`.
- Settings: `Parts`, `K`, `N`, `SlipGroups`, `SlipGroupK`, `SlipK`, `SlipN`, `Make`.
- Result: `Show` (eye), `Part(±1)`, `Check`, `Compare`, `CheckClear`, `Save`, `Another`, `Done`.
- SLIP-39 set source (`faraday/faraday-core/src/convert/set.rs:~337-406`): `SetSource`, `SetHold`, `SetAgain`, `SetAct(Add|Replace|Rotate)`, `SetGroup`, `SetMember`, `Lost(g, by)`, `Destroy(g, m)`, `DestroyOthers`, `Make`, `Save`.
- Keyboard (`faraday/faraday-core/src/convert/mod.rs:~1784-1805`): while typing a share or the check copy, the stick's secret refusal may take the key; then `set::key`; then `check_key`; Escape goes back when not typing. Check my copy takes printable ASCII plus Backspace, Delete, Enter and Escape; other characters are swallowed (`~800-824`).
- Share typing (`faraday/faraday-core/src/convert/set.rs:~228-262`): ASCII letters only (forced to lower case), one Space between words (none leading), Backspace, Tab completes through `forms::slip39_completion`, Enter holds the share.

## Effects

- Randomness: `app.fresh_bytes(b"convert split", need)`; with none, `app.toast(EN.no_randomness)` and nothing is made (`faraday/faraday-core/src/convert/mod.rs:1436-1439`). Rotate retries up to 8 times to avoid the old identifier (`faraday/faraday-core/src/convert/set.rs:442-454`).
- Compute: `app.compute()` (`faraday/faraday-core/src/convert/mod.rs:~1441`, `~1555`); key operations in faraday-keys with `Reveal::PaperForm` (`~1442`; `faraday/faraday-core/src/convert/set.rs:~423`).
- Codex32 identifier clashes: `codex32::taken_ids` over the entry state, backup answers and vaults (`faraday/faraday-core/src/convert/mod.rs:~1447-1452`).
- Camera: `Command::CameraOn` (`~1264-1268`); scan decode `faraday_codec::decode::decode_luma` (`~1511`).
- Save: files through the secret sheet `app.offer_secret(SecretOut)` (`~1631`; `faraday/faraday-core/src/convert/set.rs:~575`). Names `{fp}-{target}.txt` or `.png` (`faraday/faraday-core/src/convert/mod.rs:~1579-1613`); the SeedQR picture through `backup_flow::seedqr_picture` (`~1612`). A set's shares are one file each, named by identifier, group, member and `-share-i-of-n` (`faraday/faraday-core/src/convert/set.rs:~484-494`, `~510-540`); a rotated set is one file. Keep is Note for one file, None for codes and multi-file offers (`faraday/faraday-core/src/convert/mod.rs:~1625`; `faraday/faraday-core/src/convert/set.rs:~570-574`).
- Vault key load: `app.vault_key_load(v, r)` (`faraday/faraday-core/src/convert/mod.rs:~1638`); `app.lightning_for_convert()` (`~1665`).
- Keys store writes by `set::record` (`faraday/faraday-core/src/convert/set.rs:~682-711`): `replace_slip39`, `set_xor`, `set_slip39`.
- No clipboard, printing or direct storage.

## State and lock

- `ConvertArea` (`faraday/faraday-core/src/convert/mod.rs:~644-662`) at `app.tools.convert`: `state: Option<ConvertState>`; `held: Option<Value>` (`Text(Held<Text>)`, `Code(Held<Box<QrMatrix>>, usize)`, `Set(Held<Vec<Text>>)`), the made value; `shown_text: Option<Text>`; `check: Option<WordCheck>` (the typed copy); `set_typed: SecretText` (a typed share, `657`); `found`, `found_part`.
- `ConvertState` (`~459-506`): public data only (open card, done flags, scroll, key fingerprint, aezeed, other forms, `set: Option<SetState>`, target, shown, part, back, checkable, refused, `paths`, settings, parts, SLIP-39 shape, k, n). `SetState` (`faraday/faraday-core/src/convert/set.rs:~27-45`): act, group, member, lost, destroyed, others, error, refused; nothing secret.
- Secrets: `held`, `shown_text`, `check`, `set_typed`, the random draw (`Zeroizing`, `faraday/faraday-core/src/convert/mod.rs:1436-1439`), the scanned luma (`Zeroizing`, `~1507`), file bytes (`Zeroizing`). A typed aezeed and a typed SLIP-39 set are held in faraday-keys and wiped by `forget_held` (`1645-1648`).
- Lock (`AtLock for ConvertArea`, `faraday/faraday-core/src/convert/mod.rs:1900-1911`): `holds()` is `Holds::typed` of held, shown text, check or a typed share; `lock()` = `clear()`: the state is None, `set_typed` is cleared, and `drop_value` drops held, shown text, check, found and found part (`700-716`). The lock does not call `forget_held`. The lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- The aezeed and the SLIP-39 set Convert holds in faraday-keys are cleared in the same lock by the session area: `Session::lock` calls `wipe`, then `keys.clear()`, then `remove_all`, which sets `aezeed` and `slip39_set` to `None` (`faraday/faraday-core/src/wallet.rs:1539-1549`, `5568-5579`; `faraday/faraday-keys/src/session.rs:1274-1293`).
- Leaving: `tidy` (`faraday/faraday-core/src/convert/mod.rs:1817-1826`) clears the flow and calls `forget_held` unless the screen is Convert, Add a key opened from Convert, or Lightning for Convert.
- Idle warning: `hide` (`1807-1812`) drops `shown_text` and closes the eye; the value stays held.
- Stick attached: `stick_attached` (`1653-1659`) wipes the typed share and the check copy and closes share entry.
- Open resets everything and drops the previous value (`~858-860`); a new key or target press drops the value and check; `choose` clears the typed share and SLIP-39 shape when the key changes (`~995-1010`).

## Reach

- Opened from Tools, from Explore (`faraday/faraday-core/src/explore/screen.rs:~81-91`), from a wallet chart's key sheet (`Convert(OpenOn(fp))`).
- Opens Lightning (aezeed read; `lightning` calls `choose_aezeed` back), Add a key, the scanner, the secret sheet.
- Uses `convert::set`, `copy_check`, `secrets`, `secret_text`, `wallet::{fp_text, key_line}`, `codex32::{taken, taken_ids}`, `fresh::{TAKE_MAX, fresh_bytes}`, `backup_flow::seedqr_picture`, `vault::screen::secret::{secret_with, code_with}`, `compact::kv`, `pick`, `flow`, `trail::StepFlow`, `at_lock`, `ways_in::CONVERT_FROM`, `forms::{Form::Slip39, language_name, slip39_completion}`, `stick::{refuses_secret, refuses_secret_key, keys_load}`, `edit::Here`, `explore_hide`, `lightning_for_convert`, `vault_key_load`, `session.keys`, `session.wallets`.
- Drawing borrows `convert_held`, `convert_part` and `convert_shown_text` (`faraday/faraday-core/src/drawing/allowed.rs:~50-66`).
- Documented in `docs/SEED-CONVERSION.md` and `docs/VAULT.md` §7.
- `convert::set::record` is called by `Faraday::record_made` after a set's files go to the vault (`faraday/faraday-core/src/secrets.rs:475`) or to the Outbox past the secret sheet (`faraday/faraday-core/src/secrets.rs:718`), and by the backup flow's Seed XOR count step when the key has no XOR set yet (`faraday/faraday-core/src/wallet_backup.rs:2851`).

## Rules and refusals
- A SeedQR or CompactSeedQR result is shown only after the drawn code is decoded back and reads as the same seed; a code that does not decode, decodes to more than one, or reads as another seed is refused with the one text "did not read back as the same wallet" (faraday/faraday-core/src/convert/mod.rs:1456, 1499-1523, 1032).

Choosing
- With more than one key loaded, none is chosen at Open (`faraday/faraday-core/src/convert/mod.rs:~1669`). `OpenOn` and `Key` refuse a fingerprint not in `session.keys`; `OpenOn` then changes no screen (`~1674-1676`, `~1688`).
- A card opens only when every earlier card is done; pressing the open card does nothing (`step`, `972-993`).
- Targets must apply to the key (`ROWS` `applies`, `~352-455`): SeedQR and Compact SeedQR need the English list and 12 or 24 words (`seedqr_words`, `~322-329`); Starts and Seed XOR need English words; SLIP-39 needs 16 or 32 seed bytes; codex32 any length; a 64-byte SLIP-39 and an xprv-only key are not offered. Rows with no target cannot be pressed (`~1273-1277`). A make with no row that applies does nothing (`key_for`, `~1210-1221`; `make`, `~1400-1402`).
- Targets that draw randomness are dimmed when the session does not make secrets (Practice); the Settings card only when it does (`dimmed`, `has_settings`, `1198-1206`).

Settings bounds (`faraday/faraday-core/src/convert/mod.rs:926-955`, `1055-1081`)
- Seed XOR parts 2..=4.
- codex32 threshold steps 0, then 2..=9; the count k..=31 when k > 0, and n = 1 when k = 0.
- SLIP-39 groups 1..=`MAX_SLIP` (`faraday_bip::slip39::MAX_COUNT`, 16, `28`); the group threshold 1 for one group, else 2..=count; the member threshold 1 for one share, else 2..=shares; shares per group 1..=`MAX_SLIP`.
- A SLIP-39 press is taken only when the randomness can pay for the shape (`slip_press`, `slip_fits`, `TAKE_MAX`); the `+` button shows disabled otherwise (`faraday/faraday-core/src/convert/screen.rs:~650-654`). A shape a key cannot pay for does not leave Make on that key (fix row 02c8e6bda).

Making and the result
- No randomness: a toast, nothing changes (`faraday/faraday-core/src/convert/mod.rs:1436-1439`).
- A failed make: `Convert::NotSameWallet` shows `convert_not_same_wallet`; any other error `convert_not_given` (`~1032-1033`); Save and Check are hidden (`checkable` false, `~1044`).
- A code result must scan back: exactly one decoded code (`~1511-1513`), a Compact SeedQR decodes to the entropy (`~1516-1518`), `seedqr_reads_as == Some(true)` (`~1529`); otherwise `NotSameWallet` (`~1460-1462`).
- The passphrase is left out of the value for every target except SeedHex and Xprv, and the file name and the "gives" line say so (`keeps_passphrase_apart`, `save`, `~1565-1569`). A non-English list puts the list's name in the file for word-number targets (`file_for`).
- Paths fact: "no wallet" when no loaded wallet holds the key; paths update only on a press (`faraday/faraday-core/src/convert/screen.rs:~963-966`). Aezeed paths' coin is 0 on mainnet, else 1 (`faraday/faraday-core/src/convert/mod.rs:~1366-1374`); a tprv name off mainnet (`faraday/faraday-core/src/convert/screen.rs:~72-79`).
- Another keeps the key only if it is still loaded (`faraday/faraday-core/src/convert/mod.rs:~1767-1768`); Done goes to `back` (`~1773-1778`); Escape goes back when not typing (`~1798-1800`).
- Sample material: Mainnet refuses sample material with the public-sample line; a set with a sample's identifier is refused only when it also matches the sample's iteration exponent and is not extendable (fix row 5ac5643f1).

Check my copy and Compare
- Words and Seed XOR are checked only for English-list keys (`Target::checks`, `~284-288`).
- Word numbers accept 1 to 2048 alone or paired, whitespace-insensitive (`copy_is`, `~630-640`); the comparison is constant time (`same_text`, `~600-602`; `ct_eq` in `same_tokens`); a length mismatch returns early.
- Compare with nothing typed does nothing (`~755-757`); a match closes the check; a difference clears the typed text.
- Opening Check my copy, and Compare with a typed copy, are refused when the stick refuses secrets (`1749-1759`). Key typing is refused by the stick (`~1789`).
- Show only when a value is held (`~1718`). Part buttons only with several shares (`part_of`).
- Check on a SeedQR or Compact SeedQR result opens the camera instead (`checks_by_scan`); `scan_open` needs a checkable result and a loaded key (`~1252-1262`).

SLIP-39 set source (`faraday/faraday-core/src/convert/set.rs`)
- Refused with a toast while a stick is attached (`~316-318`); refused with no note in Practice (`!makes_secrets`) or when keys may not load (`~319-320`).
- Holding empty text does nothing (`~276-278`); a refused share keeps the typed text unless the refusal is `Stick` (`~296-299`).
- Add and Replace are offered only when some group has its quorum; Rotate only when the set can be recovered; else a "needs N" line (`~49-68`; `faraday/faraday-core/src/convert/screen.rs:~387-399`). An act not offered does nothing on press (`~353-356`).
- Group and member cycle only among groups with quorum and members not typed (`~358-377`). The lost count is clamped to 0..16 (`~383`).
- Rotate returns with no note when the set may be in other hands or not every share is destroyed (`433-435`); Make on screen is offered only when every destroyed share is set, refused with an "other hands" text otherwise (`faraday/faraday-core/src/convert/screen.rs:~488-501`). After 8 collisions with the old identifier the result stays `SameIdentifier`, shown as the "not the same set" text (`faraday/faraday-core/src/convert/set.rs:~127`, `442-454`).
- Recording (`~682-711`): a rotated set with the same identifier is not recorded; recorded only on a loaded key whose recorded set is the one rotated; a key keeps one set of a kind. `same_set` needs the identifier, exponent, group threshold, group count and member threshold to match (`~716-727`).
- `Value::Code` cannot be saved as a file; nothing is said (`~549`).

## Domain calls

- `core/faraday-bip`: `bip39::Language`, `keys::{Fingerprint, Network}`, `slip39::{MAX_COUNT, GroupSpec, Share::parse, identifier, iteration_exponent, group_threshold, group_count, member_threshold}`.
- `faraday/faraday-keys`: `aezeed_birthday`, `aezeed_forget`, `slip39_set_{hold, facts, forget, share, rotated}`, `slip39_{converted, one_of_one}`, `seed_xor_converted`, `codex32_converted`, `words_written`, `seedqr_written`, `master_seed_hex`, `master_xprv`, `*_random_len`, `Held`, `Reveal::PaperForm`, `operations::{Convert, WordsForm}`, `words::from_compact_seedqr`, `slip39_set::{SetFacts, SetRefused, RotateGroup, rotate_count}`, `identity::fingerprint_of_words`, `Keys::{replace_slip39, set_xor, set_slip39}`.
- `faraday/faraday-vault`: `xor::XorFacts`, `slip39::{Slip39Facts, Group}`.
- `core/faraday-codec`: `qr::QrMatrix`, `decode::decode_luma`, `seedqr::to_digits`.
- `core/faraday-crypto`: `work::Compute`. `faraday/faraday-strings`: `bip::slip39_error`. `subtle::ConstantTimeEq`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 02c8e6bda | A SLIP-39 shape set on a 16-byte key could leave Make enabled on a 32-byte key, where it never works. | A shape a key cannot pay for does not leave Make on that key. |
| f1b50e27d | Convert's steppers were under the touch floor on the stick; the fourth step's title was cut. | Steppers reach the touch floor (decision F201). |
| 5ac5643f1 | A Seed XOR sample's combined words saved as a Core Lightning hsm_secret were not refused on Mainnet; Convert's refusal of a sample's SLIP-39 set gave the wrong text; a set Convert made was refused for sharing a sample's identifier. | Mainnet refuses sample material with the public-sample line; a set with a sample's identifier is refused only when it also matches the sample's iteration exponent and is not extendable. |

## Doubts

- `ROWS` predicates, `Target::checks` and `file_for` line numbers were not re-checked exactly.
