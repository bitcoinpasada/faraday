# Adding a key

Add a key (`Screen::Entry`) brings an existing key into the session: seed
words typed on the on-screen word grid or a hardware keyboard, or one of the
other key forms (xprv, hex seed, codex32, SLIP-39, Seed XOR, Krux encrypted
code), one share, string or part at a time. It is for a person who holds a
key elsewhere. The ways in to a key (`ways_in`) decide which forms each
opener shows first. The word list sheet lets a person look a word up. Add a
key is reached from the session pane's Add a key button, from New key (Add
a key form), from Restore, Spend from backup, Check a signer, My seed makes
my wallet, Convert and the backup check, each with its own opener.

## Screens and sheets

- `Screen::Entry` (`faraday/faraday-core/src/screens/entry.rs`; state `EntryArea`/`EntryState`, `faraday/faraday-core/src/state.rs:~26-86`, `~193-195`).
- Words form: word count chips 12/15/18/21/24 (`EntryWordCount`); language (`EntryLanguage(s)`); word fields or the list's own keyboard (Latin, kana, jamo, pinyin, zhuyin); a candidate strip; status (count, checksum, fingerprint, last refusal); the passphrase and its repeat; Add key, Clear, Make a new key, Scan a SeedQR, Paste words, Use a sample, From a stick (`VisitFrom(Screen::Entry)`).
- Other forms (`Sheet::OtherForms(o)` or a row of chips by opener): xprv, hex seed, Codex32, SLIP-39, Seed XOR, Krux code (the Krux read rows, `seeds::kef::read_rows`). For codex32: the Suggest corrections tick and panel (see `codex-shares`), the clash panel (Keep the one held / Replace it), an "Already in" vaults line, the worksheet.
- Hint text `ms1…`, xprv and hex seed hints are built as `SecretText::of` hints (`faraday/faraday-core/src/screens/entry.rs:886-893`).
- Word list sheet (`Sheet::WordList`, `faraday/faraday-core/src/wordlist/mod.rs`, `faraday/faraday-core/src/wordlist/screen.rs`): over the screen beneath; BIP-39 English (2048 words, 11 bits) and three EFF diceware lists; each word's number and code; searchable by word prefix, number prefix (from 1) or code prefix. The search field shows on desktop or when something is typed.
- Back is `Nav(app.entry_leave())`.
- Faraday Demo: each step that takes a seed carries a line, with no warning first (`faraday/shells/demo/src/lib.rs:~421-429`).

## Actions

Entry (`entry_act`, `faraday/faraday-core/src/word_entry.rs:~913-1037`):
- `Entry(wanted)` (only if `may_load_keys()`), `EntryWordCount(n)` (only values in `EntryState::COUNTS`), `EntryClear`, `EntryAdd` (also Enter), `EntryLanguages`, `EntryLanguage(i)` (index in `forms::LANGUAGES`), `EntryKey(c)` (a whole word is taken at once when `word_whole`), `EntryKeyBack`, `EntryShift`, `EntryCandidate(n)`, `EntryForm(i)` (index in `Form::ALL`), `EntryOtherForms(o)`, `EntryField`, `EntryPart`, `Codex32Correct`, `Codex32Form(C32Press::{Keep, Replace, Rest, AddShare, Check, Sheet})`, `EntryRecover`, `EntryPassphrase`, `EntryPassphraseAgain`, `EntryWallet(w)` (needs `may_load_keys` and `w < wallets.len()`; takes the camera and pushes `CameraOff`), `EntryWalletForm(w, code)`.
- Also `ScanSeed`, `ScanKey(w)`, `ScanPart`, `PasteWords`, `UseSample`, `KeyGen(None)`, `TypeHere(Here::Words|Keyed|Entry)`.
- Enter on the seed check or New key screens routes to `seed_check::act(Words)` or `keygen_act(KNext)` (`faraday/faraday-core/src/word_entry.rs:1044-1048`).
- Keys (`faraday/faraday-core/src/state.rs`): Space separates words and shares; Tab completes the word or moves to the repeat field; Backspace; Enter is `Keyed::Add` from the passphrase on word forms, `AddPart` with text in a part form, else `Recover`; Escape leaves (on a codex32 clash it keeps the string). While a codex32 string does not parse, the correction screen takes every key first.
- Word keyboard: Latin typing with Tab or Space completion on a unique candidate; digits only with `numbers`, a four-digit number below 2048; on-screen entry for Japanese, Korean and Chinese; caret editing; Backspace on an empty prefix steps back to the previous word (not while a word is open for edit).
- Word list (`WordListAction::{Open(list, mark), List(k), Mark(i), Close, Outside}`): ASCII letters and digits, lower-cased, up to `FIND_MAX` = 12; Backspace; Up/Down scroll 60; Escape closes; other keys are taken and ignored (`faraday/faraday-core/src/wordlist/mod.rs:~268-296`).
- Ways in (`faraday/faraday-core/src/ways_in/mod.rs`) emit `ScanSeed`, `SeedCheck(Type|Form(i)|Loaded(i))`, `SignerCheck(...)`, `ScanKey(wi)`, `EntryWallet(wi)`, `EntryWalletForm`, `RSlot*`, `Check(Press::Form)`, `EntryForm(code)`, `EntryWordCount(n)`.

## Effects

- Loads the key into `session.keys` (`typed_load`), then resets the entry, goes to `back` or to Spend/Wallets, `refresh_spend`, `seeds_took`, and a toast (`entry_add`, `faraday/faraday-core/src/word_entry.rs:~1054-1206`).
- Fingerprint of a duplicate through `faraday_keys::identity::fingerprint_of_mnemonic` (PBKDF2).
- `EntryWallet(w)` pushes `Command::CameraOff`.
- No files written.

## State and lock

- `EntryState` (`faraday/faraday-core/src/state.rs`): `typed` (`SecretText`), `passphrase` (`PassField`), `parts` (`Parts`), `keys` (`WordEntry`), `codex32`, `correction` (Suggest corrections). All secret while held. `probe` holds a public fingerprint until `forget_probe`. `reset()` replaces the whole state.
- `WordEntry` (`faraday/faraday-core/src/word_entry.rs:~219-236`): `count`, `lang`, `words: [u16;33]`, `committed`, `prefix`, `allowed_last` bitmask, `open`, `held`, `held_n`, `held_prefix`. `Zeroize` clears words, prefix, `allowed_last`, `held`, `held_prefix` and resets the counters; `Drop` zeroizes; `count` and `lang` are kept (`faraday/faraday-core/src/word_entry.rs:238-260`). A count not in `WORD_COUNTS` becomes 12 (`faraday/faraday-core/src/word_entry.rs:265-271`).
- `typed_index` and `typed_mnemonic` buffers in `faraday/faraday-core/src/forms.rs` are `Zeroizing`.
- Lock: `EntryArea::holds` counts typed text, parts, passphrase or word keys; `lock()` calls `reset()` (`faraday/faraday-core/src/state.rs:2766-2781`). The lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Leaving the screen resets the entry (`faraday/faraday-core/src/state.rs:~2234-2239`). Passphrase fields are emptied when their screen goes and when a stick arrives (`faraday/faraday-core/src/pass_field/mod.rs:623-644`).
- Word list: `WordListArea` holds `find: String` (a search, which can hold a seed word) and a mark; `holds` = find non-empty or a mark set; `lock` = `clear()` (`faraday/faraday-core/src/wordlist/mod.rs:325-335`).
- `Parts` and `Clash` (`faraday/faraday-core/src/forms.rs:18`, `160`) have no `Drop` of their own; each element wipes itself. `slip39::Share` has `Drop` and `ZeroizeOnDrop` (`core/faraday-bip/src/slip39.rs:574-580`). `Codex32` and `Codex32Ascii` derive `ZeroizeOnDrop` (`core/faraday-bip/src/codex32/mod.rs:370-381`). `SeedXor` has `Drop` (`core/faraday-entropy/src/xor.rs:40`). None of them has `Debug`. `Clash.differ` holds box positions only.

## Reach

- Opened by the ways in (`faraday/faraday-core/src/ways_in/mod.rs`, openers `ADD_A_KEY`, `SEED_MAKES_WALLET`, `CHECK_A_SIGNER`, `SPEND_SIGN`, `RESTORE_SLOT`, `CHECK_BACKUP`, `CONVERT_FROM`), New key (`Entry(None)`; Krux through `keygen_to_krux`), BIP-85 child keys (`Action::Entry(None)`), Convert, Restore, Spend from backup.
- Opens New key (Make a new key), the scanner (Scan a SeedQR), Files on a stick (From a stick), the Krux read sheet (`krux-codes`), codex32 making for a given set (`codex-shares`).
- Uses `edit::Here`, `backup.check_edit`, `words_use`, `copy_check_after` (in a check), `restore`, `seed_check`, `keygen`, `scan_key::Found`, `pass_field::eye`, `secret_text::SecretText`, `spend_backup_key_added`, `samples` (Use a sample).
- The word grid is shared with New key's Add a key form and Check my copy.

## Rules and refusals

Key forms (`faraday/faraday-core/src/forms.rs`)
- `Parts` readiness (`~115-130`): SLIP-39 needs the member threshold of the first share's group; a codex32 secret is ready at once, a share set needs `len + codex32_vaults >= threshold`; Seed XOR needs `parts() >= MIN_XOR_PARTS`; Words, Krux, Xprv and HexSeed are never "ready" as parts. `SLIP39_ROOM = 256`, `PART_ROOM = 32`.
- codex32 presses (`C32Press`): Keep, Replace, Check a string, Check it, worksheet, Make the rest, Add a share; `offers_more` only for threshold >= 2. A bad checksum names the share index, or says the secret fails (`~300-315`). `of_the_set`: same identifier, threshold and length (`~354-361`).
- Words: up to 24 words; typed mnemonic words must be on the list, then `from_indices` checks the mnemonic (`~492-500`).

Add a key (`faraday/faraday-core/src/state.rs`)
- The passphrase is typed twice unless the entry has a wallet or is Restore/SpendBackup. Differing passphrases leave the key waiting: `form_recover` returns with no text set.
- The passphrase is printable ASCII only; the refusal names no letter.
- Word counts 12/15/18/21/24 (default 12, grows to 24). The word keyboard moves on only when the word is whole and unique. Words need a valid checksum.
- Space is refused when the field is empty or after a space, for forms other than codex32.
- SLIP-39: a share must parse; a repeat is refused; no more shares than the room made at the start.
- Codex32: a string must parse; another identifier is refused; the same index and identical string reads "Already in"; the same index with a different string is a clash, Keep or Replace; k must be met, and open vaults' strings count toward it.
- Seed XOR: words must parse, within the parts room.
- Restore takes only a key that fills a slot, refused before loading.
- Keys load only where `may_load_keys()` and the form is not Words or Krux (those load elsewhere).
- A key made here and added without its quiz is released when the same quorum is typed back.

Word entry (`faraday/faraday-core/src/word_entry.rs`; `entry_add` at `~1054-1206`)
- `entry_add` is a silent no-op when `!may_load_keys()`, when the passphrases differ, or when the passphrase eye waits.
- A mnemonic parse error is shown as the entry error.
- Wallet key check: a key not of the wallet is `Refusal::NotOfWallet`; a blank label takes the wallet's label; a key already present is not added again (`Found::Here`).
- Restore refusal through `restore_refusal_typed`.
- `Duplicate` on SpendBackup, Restore, Wallets or None is taken as loaded; an error computing its fingerprint is swallowed and the form stays.
- `WrongKey` on SignerCheck shows the wrong-seed message; all other errors go to `entry.set_error`.
- The candidate strip holds up to 8 on spelled lists and `MAX_CANDIDATES_HANZI` on one-character lists; on the final word only checksum-valid words are offered, but all matching words if none fit.
- `type_char` refuses a key with no continuing candidate; the kana small key toggles only if the swapped form has a candidate; `put` does nothing at 33 words; `commit_candidate` refuses an index past the strip.

Add a key screen (`faraday/faraday-core/src/screens/entry.rs`)
- Counts 12/15/18/21/24 are complete only when every typed word is in the list (`185-188`); other counts show the count alone (`~356-358`).
- The word-list refusal comes from `faraday_keys::words::from_indices`; the `typed_mnemonic` error becomes the status (`~360-361`, `~389-390`). Checksum and fingerprint only for a whole key, through the `entry_probed` probe (`~344-412`).
- Add key is disabled while the passphrase fields differ or the eye waits (`~575`); Make the key primary only when `ready && !differs && !pass_eye.waits()` (`~805`); Add key styled primary only with a typed word (`~577`, `~852`).
- No Make a new key on a Restore slot (`~599`). No passphrase on codex32 or on a flow's form (`~753`).
- The stick way is offered only when empty (as a hint) or with files (`~656-681`).
- Add share is disabled while the field is empty or a clash is open (`~778`). The codex32 "in vaults" line only for a holding string with a vault count above 0 (`~1062`). More / Add a share only when `offers_more(&parts)` and not checking (`~1153`).

Ways in (`faraday/faraday-core/src/ways_in/mod.rs`, `faraday/faraday-core/src/ways_in/screen.rs`)
- Each opener names which key forms it takes first as chips; the rest are under Other forms (`Action::EntryOtherForms(o)`).
- Check a signer's Other forms is always a sheet (`sheet_only`).
- Loaded-key "holders" appear under Other forms only for My seed makes my wallet.
- `Way::SeedQr`, `Aezeed` and `AddAKey` have no row text (their press is `Action::Cancel`; their own screens draw them).
- `others_with` keeps only `Way::Words`, so the aezeed in `CONVERT_FROM` never shows in those rows.
- `CHECK_BACKUP` has no word form.

Word list
- An unknown list index is refused (`faraday/faraday-core/src/wordlist/mod.rs:~212-214`).

## Domain calls

- `core/faraday-bip`: `bip39`, `kana`, `slip39`, `codex32`, `wordlists`.
- `core/faraday-entropy`: `SeedXor`, `WORD_COUNTS`.
- `faraday/faraday-keys`: `StringCheck`, `words::from_indices`, `identity::fingerprint_of_mnemonic`.
- `faraday/faraday-intake`; `faraday/faraday-vault`: `codex32`.
- `core/faraday-draw`: `widgets::keyboard`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 867cd6738 | On a window 800 high with 24 words, the passphrase row ran under "From a stick" and the Add key row, pinned to the bottom, so Add key could not be reached. | A form's footer sits below its content when the content pushes it down; the page scrolls so the action is always reachable. |
| c2c4c296f | A very long key or descriptor was decoded by a quadratic base58 decoder, so one crafted key could stall the device. | Keys over 112 characters (WIF over 64) are refused before decoding; every key that read before still reads. |
| 1761268bd | A word grid showed a caret in every empty cell after the one being typed into. | The caret shows only in the cell being typed into. |
| 0d9ef7dd7 | In lists typed on their own keyboard (Add a key, New key's Add a key form in Japanese, Korean and Chinese, Check my copy) the caret did not stay where the person put it; past a word's edge the earlier word was not reopened. | The caret goes where the person puts it; moving past a word's edge reopens the earlier word's cell. |
| 1dff48548 | Seed words in a composed, decomposed or accent-less form, and words of non-Latin lists (Korean, Japanese), were not found on their BIP-39 list, so a valid seed was not recognised. | Words of any BIP-39 list match in any form (NFKD, lower case, accent fold); a seed of another list is named as such. |

## Doubts

- c2c4c296f names wallet files, BSMS, Coldcard exports, PSBT registration and BBQr. The 112-character cap is in `core/faraday-bip/src/xkey.rs:~43`; that Add a key's Xprv form goes through it was not traced.
- `EntryList::Slip39` is never built by `WordEntry::list()` (`faraday/faraday-core/src/word_entry.rs:~473-475`).
- `reading_keys` loops over 2048 indices per keystroke.
- The word list's draw calls `app.wordlist_scroll(max, scroll)`, which writes the clamped scroll to state during drawing (`faraday/faraday-core/src/wordlist/screen.rs:~133`); a memo or a breach of "drawing reads", unclear.
- `SEED_MAKES_WALLET`, `CHECK_A_SIGNER` and `SPEND_SIGN` have identical constants; the `CONVERT_FROM` doc says the aezeed is under Other forms but `others_with` drops it.
- Which `Screen` hosts the compact Add a key page, and whether `entry_sample_shown()` includes "field empty", were not read.
- The inventory gave no line numbers for the Add a key rules in `faraday/faraday-core/src/state.rs` or for the word keyboard and candidate rules in `faraday/faraday-core/src/word_entry.rs`; only the files are named.
