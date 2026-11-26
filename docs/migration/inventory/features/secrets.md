# Secrets, text checks and the passphrase field

Where a secret may go when it leaves a screen, how the app refuses text or
files that hold a secret, and the passphrase field with its eye. The secrets
sheet is reached from the backup checklist, vault saves, plan changes and the
Outbox; the text check guards Faraday Demo's Transfer, My seed's Compare and
wallet names; file classification runs on every Inbox file, every stick file
and every Outbox write. The passphrase field is the one type that holds a
typed passphrase on sixteen screens and sheets.

## Screens and sheets

- Secrets sheet `Sheet::SecretOut` (and `VaultSave` on cancel of a vault-save ask): a secret waits for the person to choose where it goes: into the open vault as a note or key record (`SecretVault`, `vault_push`), or to the Outbox unprotected after the hold "save without encryption" (`SecretHold`, `SecretUnprotected`; `SecretForm(i)` picks a file form) (`faraday/faraday-core/src/secrets.rs:~311-420`).
- `offer_seed` from the backup checklist offers the words as `.txt` and, for English only, SeedQR and Compact SeedQR PNGs through `Reveal::OutboxFile` (`faraday/faraday-core/src/secrets.rs:~311-420`). SeedQR is offered only where the plan copies the seed as SeedQR (fix 1b0bc0bd7).
- The plain-save sheet's keeper line shows only in builds with Faraday Demo's Transfer (fix d9cfc24f7).
- Passphrase field (`faraday/faraday-core/src/pass_field/mod.rs`, drawn by `faraday/faraday-core/src/pass_field/screen.rs`, pairs in `faraday/faraday-core/src/pass_field/pair.rs`): one field, or a pair (the passphrase and the one typed again). Dots unless the eye is open.
- The eye (`faraday/faraday-core/src/pass_field/eye.rs`): what is typed shows in the clear only while it is open; closed by default, by a lock and by the idle warning.
- A "keys are read as US" note shows where a passphrase is typed with hardware keys (Add a key, a Create slot's key, the seed check) (fix c4ef92575).
- The caution "Never type a seed that holds real bitcoin here." stands under the passphrase field; Unlock does not show it while nothing can be typed (fix 25d6eda26).
- Text check refusal lines: `EN.transfer_text_too_long` (`faraday/faraday-core/src/transfer/screen.rs:201`), `EN.seed_check_compare_too_long`, `EN.seed_check_compare_private` (`faraday/faraday-core/src/seed_check/compare.rs:432-447`).

## Actions

- Secrets sheet: into the vault (`SecretVault`), hold to save without encryption (`SecretHold`), pick a file form (`SecretForm(i)`), Cancel.
- Hold dispatch list for the sheet's holds: `SecretHold`, `RegisterHold`, `Sweep`, `ReviewHold`, `DevicesHold`, `NameTakenOverwrite`, `OutboxDeleteHold` (`faraday/faraday-core/src/secrets.rs:~587-598`).
- Passphrase field keys (`PassField::key`, `faraday/faraday-core/src/pass_field/mod.rs:~150-188`): a character goes in if the kind takes it and it fits; Backspace takes one out; Tab and Shift-Tab move between the two fields of a pair; Enter and Escape are handed to the screen.
- `Action::PassEye`: opens or closes the eye (`PassEye::toggle`).
- "Use faraday" (Practice's sample passphrase) on an empty field, offered only while the other field is empty too (fix d102b3c33).
- A press on a passphrase field brings the on-screen keyboard up on a one-column layout (fixes e3e249aaf, 77649ed0e, 23e467c86).

## Effects

- An unprotected secret goes to the Outbox as a plain file; a same-name file there is replaced (`faraday/faraday-core/src/secrets.rs:~719-724`). The file's bytes are copied into a plain `Vec` for the Outbox item (`~725`).
- A secret saved into the vault becomes a note or key record of the open vault.
- Cancel drops the secret, undoes the plan (`board_undo`), returns to `VaultSave`, or skips a transfer (`faraday/faraday-core/src/secrets.rs:~747-754`).
- The text check and file classification write nothing.

## State and lock

- `SecretsArea { out, plain }` (`faraday/faraday-core/src/secrets.rs`). `SecretOut` bytes, `also` and `forms` are `Zeroizing`. `Plain.then` may carry `plan::Answers` (not secret).
- Lock: `holds()` is typed when `out` is some and not `to_visit`; `lock()` calls `clear()`, dropping both (`faraday/faraday-core/src/secrets.rs:786-796`).
- `TextCheck` is `Copy` and holds no secret bytes. Process-lifetime caches (`OnceLock`): `lists()` (all BIP-39 languages and the SLIP-39 list, `faraday/faraday-core/src/text_check.rs:~320-341`), `prefixes()` (`~345-354`). Temporary `Text` copies are made while it runs; `Text` zeroizes on drop and on clear (`core/faraday-text/src/text.rs:253-371`).
- File classification: local buffers are `Zeroizing<Vec<u16>>` (`faraday/faraday-core/src/wallet.rs:~2695`, `~2792`).
- `PassField` (`faraday/faraday-core/src/pass_field/mod.rs:114-124`): `kind`, `text: SecretText`, `again: Option<SecretText>`, `on_again`. It has no Clone, Debug or PartialEq. `SecretText` wraps a wiping `Text` (`faraday/faraday-intake/src/secret_buf.rs:25`).
- The field is emptied:
  - when its screen or sheet is no longer up, however it was left (`pass_field::left`, `faraday/faraday-core/src/pass_field/mod.rs:~623-640`); a field of a flow in progress (F41) is emptied only by finishing, letting go or a lock;
  - when a stick arrives: every field (`stick_arrived`, `~642-646`);
  - by its owner's `lock()` at a lock.
- Leaving Vault contents drops the one-line prompt (`left_contents`, called from `pass_field::left`).
- The eye `PassEye { shown, hardware, opened }` lives in `typing.pass_eye` (`Public`). `settle` closes it whenever the set of passphrase fields drawn changes (`faraday/faraday-core/src/pass_field/mod.rs:~684-697`).
- The fields built on `PassField` (`Where`, `faraday/faraday-core/src/pass_field/mod.rs:~304-363`): Potential (Inbox possible wallet), Aezeed (Lightning), Entry (Add a key), SeedRead, KefRead, Slot (Make a new wallet's key slot), NewKey, SeedCheck, KefTool, BackupKef, CoreWallet, CopyCheck, GlanceCheck, Unlock, VaultCreate, VaultPrompt.
- Other secret text types confirmed as `SecretText`: `WordCheck.typed` (`faraday/faraday-core/src/copy_check.rs:19-24`), `EntryState.typed` (`faraday/faraday-core/src/state.rs:28-49`, `299`).
- An unprotected secret goes to the Outbox as `Item::new(.., bytes.to_vec())` (`faraday/faraday-core/src/secrets.rs:722-726`). `Item::new` moves the `Vec` into `Zeroizing` with no copy (`faraday/faraday-core/src/item.rs:93-94`), and `Item`'s `Debug` prints no bytes (`faraday/faraday-core/src/item.rs:59-68`). The Outbox `Item` is `at_lock::Public` (`faraday/faraday-core/src/item.rs:56-57`) and is kept across a lock by design (`faraday/faraday-core/src/storage_api.rs:686-690`).

## Reach

- Secrets sheet is opened from: the backup checklist (`offer_seed`), vault Save to a stick, plan changes on the backup board, Lock-and-write, the key file's `.keyx` (`OutReason::KeyFile`), the vault's Save to a stick (`OutReason::VaultStick`).
- Text check callers: Transfer's Send text (`faraday/faraday-core/src/transfer/mod.rs:16` re-exports `MAX_TEXT`, `TextCheck`), My seed Compare (`faraday/faraday-core/src/seed_check/compare.rs:432-447`), wallet names (`faraday/faraday-core/src/wallet.rs:96-120`; a secret-looking name is replaced by the default, not refused).
- File classification reaches `restore::is_share`, `inbox::part_of`, `create::read_key(s)`, `one_line::line_form`, `core_wallet::is_sqlite`, `secure_boot::is_image`, `forms::*`. It is used by Files, the Inbox, the stick visit and the Outbox.
- `scan_read::secret_kind` decides a whole-text secret.
- The passphrase field is used by Unlock, Create a vault and the vault prompt (`vault-unlock`, `vault-list-create`, `vault-contents`), Add a key, New key, Lightning, the Krux code forms, the Core wallet tool, Check my copy, At a glance, the seed check.

## Rules and refusals

- Exposure per file kind (`faraday/faraday-core/src/secrets.rs:~27-54`): Vault, Kdbx, KdbxOther, KruxCode are sealed; Entries, Words, SeedPart, KeyLine, SecretWallet, KeyWithSecret, CoreWallet are secret; the others are public.
- Secrets sheet:
  - A key with no words: no offer, no message (`faraday/faraday-core/src/secrets.rs:~327-329`).
  - A vault that refuses the record puts the secret back (`~468-473`).
  - A key whose fingerprint changed, or whose record cannot be made: the sheet closes with no message (`~444-455`).
  - A plan change that leaves a secret as a plain file on a new stick place asks the hold once per file not yet warned (F129, `~604-662`). Lock-and-write asks for the first unwarned file.
  - A vault save to a stick asks the hold for each ticked non-public key file before writing (`~557-581`).
  - Secret files come into a plain copy only on the person's own stick (fix d5e312cf7).
- Text check (`text_check::check(c, text)`, `faraday/faraday-core/src/text_check.rs:45-70`; result `TextCheck::{Fine, Secret(Kind), Inside(Kind), Partial, SeedHex, TooLong}`, `~22-36`):
  - Empty or whitespace only: `Fine` (`~47-49`).
  - Over `MAX_TEXT` = 16 KiB after trim: `TooLong`, tested before any secret scan, so an over-long secret is `TooLong` (`faraday/faraday-core/src/text_check.rs:50-52`). Callers treat `TooLong` as a refusal.
  - The invisible characters U+200B, U+200C, U+200D, U+2060, U+FEFF, U+00AD are dropped before the checks (`~75-91`).
  - Whole text a secret by `scan_read::secret_kind`: `Secret(kind)`; but `Kind::Xprv` followed by a second whitespace word gives `Inside(Xprv)` (`faraday/faraday-core/src/text_check.rs:55-60`).
  - A text the file classifier calls secret (`wallet::classify(c, "text.txt", ..)`, `Exposure::Secret`), unless it is bare hex that `one_line::line_form` reads as `HexSeed`: `Secret(file_secret(file))` (`~62-68`). `file_secret` (`~94-103`) maps Words to SeedWords, SeedPart to Codex32Share, Entries to Entries, SecretWallet and KeyWithSecret to Wallet, every other secret file kind to `Kind::Wif` (`~101`).
  - `inside` (`~107-113`), in order: armour (`-----BEGIN` plus `PRIVATE KEY`, case-insensitive: `Inside(Wif)`); SeedQR digits (a run of 48 or 96 digits, or 12 or more consecutive 4-digit groups: `Inside(SeedQr)`); tokens; words.
  - Tokens (`~181-243`): each whitespace token, split at `,;=()[]{}<>"'|` (`JOINS`, `~140`), at `:` and `/` (label parts of 26 bytes or more), and the text before `://`. Parts under 26 bytes are skipped unless they are a URL or UR (`~223`).
  - `ur:` parts go to a UR decoder (`ur_part`, `~289-316`): types `eckey`/`sskr` (also with the `crypto-` prefix) give `Partial`; a registry `Error::Private` gives `Inside(Xprv)`, `Error::Seed` gives `Inside(UrSeed)`; `Message::Bytes` is checked again with `check`; `Message::Psbt` is never refused. A UR of a type that is not public and never came whole is `Partial` (`~240-242`). Only `psbt` and `crypto-psbt` are public (`faraday/faraday-core/src/text_check.rs:144-147`).
  - `seed_hex`: 32, 40, 48, 56 or 128 hex digits (optional `0x`) is `SeedHex`; 64 hex digits (a hash) is not refused (`faraday/faraday-core/src/text_check.rs:247-253`).
  - Shape (`~258-282`): WIF (51 characters starting 5 or 9, or 52 starting K, L or c, base58); codex32 (`ms1`, 48-127 characters; `Codex32Secret` if the 9th byte is `s`, else `Codex32Share`). A token containing `prv` that `wallet::classify` calls secret: `Inside(Xprv)` (`~232-236`).
  - Words (`~377-436`): split at non-letters, lower-cased NFKD. A SLIP-39 run of `MIN_WORDS-1` or more (or a dense window) gives `Slip39Share`; 12 or more words in a row matching English BIP-39 by their first four letters; 11 or more in a row on one BIP-39 list; a 12-word window with 11 on one list. `run_kind` (`~474-487`) classes the first or last 12, 15, 18, 21 or 24 words, else `BadChecksum`.
  - My seed Compare: `TooLong` gives `seed_check_compare_too_long`; `Secret`, `Inside`, `Partial` and `SeedHex` give `seed_check_compare_private`; the field is emptied and wiped when refused (`faraday/faraday-core/src/seed_check/compare.rs:~450-455`). Compare refuses secret text in every build; the classifier lives outside Transfer (fix 895ff886b).
- File classification (`classify`, `faraday/faraday-core/src/wallet.rs:~2881`; `classify_read`, `~2955-3100`; `FileKind`, `~2432-2497`):
  - Order: vault, PDF, SQLite (Core wallet), older KDBX, other KDBX, EFI image (name plus `MZ`), PSBT, then text forms: sheet (`faraday-sheet 1`), GPG public (no secret), signed message, share, secret wallet, wallet, key (key, key with secret), words, entries (authenticator export or `otpauth://`), seed part, raw hex transaction, key line, text. A `.kef` with `Raw` form is a Krux code; anything else is `Other`.
  - Secret detection: `text_may_be_secret` (`~2554`), `secret_body_kind` (`~2576-2595`), `line_is_secret` (`~2639`), `lines_hold_secret` (`~2654-2685`), `words_across`/`words_across_in` (`~2769-2845`: 12/15/18/21/24-word runs, English words of 3-8 letters, stops after 48 words in a run), `slip39_across` (`~2693-2732`), `seed_mnemonic` (`~2857-2878`: a non-English list only if it holds at least half the words), `fragments` (at most 64 per line).
  - `is_text`: non-empty, at most 64 KiB (`TEXT_MAX`), no control characters other than newline, CR and tab. Text under 8 bytes is no key unless it starts with a private version.
  - Every imported word list classes a seed as secret in any surrounding text, accented, accent-less or partly accented (fixes 5bd9bcc00, d3a1fe6e4, bc4b2a364); seed text in a GPG public key's header or user ID is secret regardless of accents or case (fix 7ed76a00b); a file that is only armoured OpenPGP public-key or signature blocks is public (see `gpg-keys`). A key copied in groups of four is still a secret; reading cost is linear in length (fix efdfc236a).
- The open vault's secret accessors (`words_of`, `totp`, `gpg_paperkey`) give their secret only for the reasons that lend it; any other reason returns nothing (fix f8fea5987, `faraday/faraday-keys/src/vault/reveals.rs`).
- Passphrase field (`faraday/faraday-core/src/pass_field/mod.rs`):
  - The stick rule comes first: with a stick in the way no character, Backspace, Delete, Enter, Tab or Shift-Tab goes in; `Refused::Stick` (`~150-165`). Shift-Tab is refused as Tab is (fixes f60a98701, b93415816).
  - A control character is ignored. A character the kind does not take: `Refused::Ascii`. A character past the kind's most: `Refused::TooLong` (`~166-178`).
  - Kinds (`~20-61`): Bip39 (printable ASCII, F155, at most `MAX_PASSPHRASE_BYTES`); KefMake (printable ASCII, at most `kef::PASSPHRASE_MAX`); KefRead (any non-control character, no most); Lightning (printable ASCII, no most); Vault and Kdbx (any non-control character, no most); CoreWallet (any non-control character, 1024 bytes).
  - Refusal text (`refused_text`, `~296-302`): `state::ascii_refused()`, `EN.passphrase_too_long`, `stick::keys_load(medium)`.
  - The eye: a passphrase typed with a hardware keyboard waits for the eye to be opened once before the step goes on (`PassEye::waits`, `faraday/faraday-core/src/pass_field/eye.rs`; `after_key`, `faraday/faraday-core/src/pass_field/mod.rs:~671-680`). Add a key and Create wait until the eye is opened (fix c4ef92575). The eye does not close a sheet or clear the field (fix 665a69d16).
  - The two typed passphrases of a pair are compared in constant time (lengths may show) (fix bbbe17f44).
- Secret arithmetic is libsecp256k1's or runs with no branch and no table read by a secret's value: MuSig2's and BIP-352's secret scalars through `SecretScalar` (`core/faraday-bip/src/musig.rs:222-288`, `core/faraday-bip/src/silent.rs:996-1021`), the nonce `reduce` (`core/faraday-bip/src/musig.rs:118-160`), SLIP-39's GF(256) (`core/faraday-bip/src/slip39.rs:345-365`), AEZ's round through `aes::hazmat::cipher_round` (`core/faraday-bip/src/aez.rs:49`); the `_public` helpers are for public values only (fix a67d98731).
  - A Krux read's passphrase has no maximum; a field is emptied when either of its texts holds anything (fix 3e14471e0).
  - Lightning's aezeed words and passphrases take no typing with a stick attached and are emptied when one arrives; the screen says keys load only with no stick (fixes ba111ecdb, ffe9ed711).
- Leaving Add a key by any way out (arrow, Escape, the mouse's back button) wipes the words typed there; one leave hook covers every way out (fixes 8e172237b, c70cdd9c7, 57798af03).

## Domain calls

- `faraday_codec::ur` (`Decoder`, `is_ur`), `faraday_codec::classify::looks_like_entries`, `faraday_codec::vault_file::is_vault_file`, `faraday_codec::kef::recognize` (`core/faraday-codec/src/`).
- `faraday_qr::registry::read` (`faraday/faraday-qr/src/registry.rs`).
- `faraday_bip::bip39` (`Language`, `fold`, NFKD lower-casing) and `faraday_bip::slip39` (`WORDLIST_LEN`, `word`, `MIN_WORDS`) (`core/faraday-bip/src/bip39.rs`, `core/faraday-bip/src/slip39.rs`).
- `faraday_text::Text` (`core/faraday-text/src/text.rs`); `SecretText` (`faraday/faraday-intake/src/secret_buf.rs`).
- `faraday_crypto::work::Compute` (`core/faraday-crypto/src/work.rs`).
- `faraday_intake::{secret_text, starts_with_private_version, wallet::{holds_secret, unread_texts}, is_otpauth_migration, is_kdbx, is_older_kdbx}` (`faraday/faraday-intake/src/lib.rs`, `faraday/faraday-intake/src/wallet.rs`).
- `faraday_pgp::read_public` (`faraday/faraday-pgp/src/lib.rs`).
- `faraday_keys::words::{from_indices, parse}` (`faraday/faraday-keys/src/words.rs`); `faraday_keys::Reveal::OutboxFile` (`faraday/faraday-keys/src/session.rs`).
- `faraday_bip::bip39::MAX_PASSPHRASE_BYTES`, `faraday_keys::kef::PASSPHRASE_MAX`.

## Fixtures

None. The word lists come from `faraday-bip`.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 3e14471e0 | A Krux read's passphrase was capped at the Krux make's max; a field was emptied on leaving only by the old non-empty test | No maximum for a Krux read; any field emptied when either text holds anything |
| f60a98701 | With a stick attached Shift-Tab in a passphrase field moved between passphrase and repeat and could clear the "why" line | Shift-Tab refused as Tab is |
| b93415816 | Same, in Add a key's passphrase | Shift-Tab goes to the field, refused, line stays |
| 665a69d16 | At a glance's passphrase check, the eye closed the Chart sheet and wiped what was typed | The eye does not close a sheet or clear the field |
| 9e43e0584 | A passphrase question for a loaded key was titled "SeedQR read"; a search over a step did not fill the flow card | The title names what is asked; search fills the card |
| ba111ecdb | The Lightning aezeed's words took typing with a stick attached and were not emptied when one arrived | No typing with a stick; emptied on arrival |
| ffe9ed711 | The aezeed's and Core Lightning's passphrase took typing with a stick; the screen did not say keys load only with no stick | No typing with a stick; the screen says so |
| c4ef92575 | A passphrase typed with hardware keys showed no "keys are read as US" note on Add a key, a Create slot's key or the seed check; Add key and Create were enabled while the eye choice was pending | The note is shown on each of these screens; the action waits until the eye is opened |
| e3e249aaf | On a one-column layout a press on the Sweep sheet's field and on key-reading sheets' passphrase fields did not bring the keyboard up | A press on a text field brings the keyboard up |
| 77649ed0e | A press on the passphrase field of the seed-by-passphrase sheet did not bring the keyboard up | A field press brings the keyboard up |
| 23e467c86 | On a one-column layout a press on My seed's passphrase fields (and account path), and in Check a seed against it, did not bring the keyboard up | A field press brings the keyboard up |
| 25d6eda26 | The caution "Never type a seed that holds real bitcoin here." was drawn across the bottom edge; Unlock showed it while nothing could be typed; Demo's sample-passphrase hint showed at once | The caution stands under the passphrase field; Unlock does not show it while nothing can be typed |
| d102b3c33 | Use faraday, offered by an empty passphrase field even when the other field held typed text, could write over what the person typed | Use faraday is offered on an empty field only while the other field is empty too |
| bbbe17f44 | The two typed passphrases were compared with a non-constant-time comparison; the hidden passphrase's bullets were not a wiping text | A comparison that decides access is constant time (lengths may show); secret text is held in wiping buffers |
| e5eeecc2a | A non-ASCII letter typed into a passphrase field was accepted | A passphrase is printable ASCII; other letters are refused and the field says so |
| 1b0bc0bd7 | A words-only copy still offered a SeedQR picture beside the words in Save as a file | SeedQR is offered only where the plan copies the seed as SeedQR |
| d5e312cf7 | A stick file that reads as a secret (seed, TOTP, wallet, SeedQR picture) was copied in plain on the wrong stick | Secret files come into a plain copy only on the person's own stick |
| 630c39de3 | The published-seed caution's keep list counted as cryptographic, and the loaded-key check did not answer the caution | Published test seeds raise the caution and only that |
| 57798af03 | Words typed into Add a key stayed paused in the pointer's drawn controls; the buffers kept old entries after the screen was left | A secret is gone from app state once the screen that needed it is left |
| 8e172237b | Leaving Add a key by the arrow, Escape or the mouse's back button did not wipe the words typed there | All ways out of a secret screen clear it the same way |
| c70cdd9c7 | Add a key had a private leave copy in trail.rs beside the state-wide one | One leave hook covers every way out |
| f8fea5987 | The vault's three secret accessors (`words_of`, `totp`, `gpg_paperkey`) handed back their secret for any reason passed in | Each accessor gives its secret only for the reasons that lend it; any other reason returns nothing |
| d9cfc24f7 | A build without Faraday Demo warned on the plain-save sheet that the keeper text was never read, although the text was not shown in that build | The keeper line is read only by the demo's Transfer; a build without the demo does not carry the warning |
| 8aa52fad5 | Item's Debug printed bytes; plain String copies of words and the typed passphrase were kept (glance sheet, vault words, the eye toggle); a secret reached the Outbox with no reason | No secret is printed by Debug; no plain String copy of words or a passphrase is kept; every secret leaves through a reason |
| efdfc236a | A large file of short lines or short words was read at a cost that stalled the device; a key copied in groups of four was not named as a secret | Reading costs linear in length; grouped keys are detected as secrets |
| 895ff886b | Compare with the wallet app's refusal sat in the demo-only Transfer module, so a stick without Transfer could not refuse a private key, seed or over-long value (and did not compile) | Compare refuses secret text in every build; the classifier lives outside Transfer |
| 5bd9bcc00 | A seed in Spanish, Japanese or another imported list, beside other text in a share sheet, signed message or file, was not kept as a secret | Every imported word list classes a seed as secret in any surrounding text |
| 636cac6a2 | Scanning text for a seed cost one lookup per list (ten lists); opening a stick or long file was slow | One word lookup shared across all lists |
| d3a1fe6e4 | A seed written without accents was not classed secret in a share sheet comment or below a signed message's signature | Accent-less seed text is secret where accented text is |
| bc4b2a364 | A French or Spanish seed with only some accents was not kept as a secret file | Partly accented seed words are secret |
| 7ed76a00b | A GPG public key whose header or user ID held an accent-less or capitalised seed was not kept as a secret file | Seed text in a GPG header or user ID is secret regardless of accents or case |

## Doubts

- Rows 5bd9bcc00, 636cac6a2, d3a1fe6e4, bc4b2a364 and 7ed76a00b were given under `lock` and moved here.
- Rows left out as outside the inventory's scope (copies of secrets in freed memory or on the stack, wipe on unwind): 853e92165, 651fc00a5 and 6b7e8096b, whose compact entry field and SLIP-39 deal parts are locals, not app state.
- bbbe17f44 is described from its commit message only; its file list was not inspected. 8aa52fad5's file list was not checked. c70cdd9c7 may only have removed duplication, not a live leak.
- e5eeecc2a (history) says a passphrase is printable ASCII; the code at the frozen commit keeps that only for the Bip39, KefMake and Lightning kinds; the Vault, Kdbx, KefRead and CoreWallet kinds take any non-control character (`faraday/faraday-core/src/pass_field/mod.rs:~45-51`).
- The passphrase field rules come from a direct read of `faraday/faraday-core/src/pass_field/` for this file; only `faraday/faraday-core/src/pass_field/mod.rs:114-124` is inventory-verified.
- `faraday/faraday-core/src/text_check.rs:~101` maps every other secret file kind to `Kind::Wif` (a catch-all); whether that is intended is not stated.
- `check` -> `tokens` -> `ur_part` -> `check` has no depth cap in code; input is bounded by `MAX_TEXT`; the termination argument is unverified.
- `outbox_secret` takes an `OutReason` it discards (`let _ = why;`, `faraday/faraday-core/src/secrets.rs:503`): a dead parameter.
- Fields not yet moved onto `PassField` keep their own state and are handled where they are (module doc, `faraday/faraday-core/src/pass_field/mod.rs:~7-10`); which ones those are is not listed by the inventory.
