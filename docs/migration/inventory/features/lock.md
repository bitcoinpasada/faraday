# The lock, idle lock and held secrets

What a Lock, an idle lock, a power-off or restart and leaving a screen clear,
and the keys held until their backup is done. It is for the person at the
offline device, who locks by hand (Lock on a vault card, the Lock sheet,
Settings, the compact dots menu) or is locked by the idle timer. Every area
of the app takes part: each declares what it holds and clears it at a lock.
A lock seals the open vaults (see `vault-seal`) and then ends the app. A key
made on the device, or held by a flow, stays "held" until its backup is
written.

## Screens and sheets

- Lock sheet `Sheet::LockAsk`: shown before a lock when Make a new wallet would end with keys kept nowhere, a lock would lose vault changes, a mode change is under way, a file waits in From/For the stick during a mode change, or an open vault has unsealed changes, unsaved keys or wallets (`faraday/faraday-core/src/idle.rs:212`). It names each vault a lock without saving would lose or leave short (see `vault-seal`). "Back to the vault" is offered only while a vault is open; otherwise Cancel, which wipes nothing (fix f537c3ffa).
- Idle warning sheet (`Sheet::IdleWarn`): shown `min(IDLE_WARN_MS, lock/2)` before the idle lock (`faraday/faraday-core/src/idle.rs:148-158`); it has a power-off line (`faraday/faraday-core/src/screens/sheets.rs:1764-1765`). A lock's own sheet is the only one the idle warning does not cover (fix 8e682d57c). The sheets `LockAsk`, `Qr` and `Import` re-open after the warning (`faraday/faraday-core/src/idle.rs:~195-203`).
- Locked sheet (shown in the next app after a lock): a countdown and up to five names (`faraday/faraday-core/src/screens/sheets.rs:1811`, `1840-1846`). It says "Locked" only after a lock and "Not locked" where nothing was locked; a plain secret still waiting in the Outbox is named in the error tone, not called wiped (fix 3cc36fdc4). It says when an idle-lock seal failed and changes were lost, and when a raised memory step was not applied (fixes 2b550d917, f65ef4ddc).
- Receive views of a held key read "Back up before you receive" (the string is `glance::RECEIVE_WARNING`, drawn as a warning; `faraday/faraday-core/src/held.rs:1278-1290`). Plans of a held key say "signs after its backup"; the silent payments page and receive view are held the same way (`faraday/faraday-core/src/held.rs:~1244-1271`).
- Toast `held_signs_from_now` when held seeds are released (`faraday/faraday-core/src/held.rs:~595`).

## Actions

- `Action::LockAsk` (vault list card, vault contents top bar, Settings, compact dots menu): runs `lock_ask` (`faraday/faraday-core/src/idle.rs:212`), which shows the Lock sheet when one of its conditions holds, else calls `lock()`.
- On the Lock sheet: lock with sealing, lock without sealing (loses what cannot be sealed), Back to the vault (only with a vault open), Cancel.
- Power off / `HoldPowerOff`: runs `exit_forgetting` (`faraday/faraday-core/src/state.rs:2548-2550`). `PowerAsk` goes straight to `exit_forgetting` when nothing waits: Outbox empty, no Secure Boot reminder, no vault changes, no restart offer, no `lock_held` (`faraday/faraday-core/src/state.rs:2536-2543`).
- Any input resets the idle timer and the carry, and clears the masked flag (`faraday/faraday-core/src/idle.rs:~148-185`).
- Leaving a screen (tap, key, Escape, the mouse's back button, the arrow): the post-press clean-up drops what the screen held, however the press came (fix 92d3e097b); every passphrase field whose screen is gone is emptied (`pass_field::left`, see `secrets`).

## Effects

- `lock()` (`faraday/faraday-core/src/idle.rs:242`): when sealing is under way the lock is remembered and done when the seal ends; when the lock would lose vault changes it shows the Lock sheet and waits; else `lock_now` (`faraday/faraday-core/src/idle.rs:286`).
- `lock_now`: `job_locked()`, `lock_secrets_off_screen()` (secrets off the screen, then `scan_drop_at_lock`, `backup.drop_paper`, `forget_glyphs`: `faraday/faraday-core/src/vault/seal.rs:192-200`), the idle under-sheet dropped, then `vault_seal_all` on the worker.
- `lock_finish` after the last seal (`faraday/faraday-core/src/vault/seal.rs:933-946`): `lock_areas()` (every area's own `lock()` in listing order, `faraday/faraday-core/src/at_lock.rs:145-153`), a debug assertion that `holds()` is empty, `wallets.set_list_changed()`, `more::drop_gone` (a made key file of a vault let go unsealed), `save_boxes_for_lock`, `restart = true`, `Command::Exit`.
- On the device the process then exits with the restart code and the kernel zeroes its pages (`docs/AT-LOCK.md:27`). When the save at lock fails ("what the app asked to be kept could not be kept", `StorageEvent::NotKept`, `faraday/faraday-core/src/storage_api.rs:~266-270`) and a flow has keys waiting for their backup, `not_kept` (`faraday/faraday-core/src/idle.rs:~273-278`, called from `faraday/faraday-core/src/stick/events.rs:~217`) clears `restart`; the stick shell then drops the old app and builds a fresh one in the same process over what the old one carries (Inbox, Outbox, kept state, mode, input carry: `carry_unkept_lock`, `faraday/faraday-core/src/storage_api.rs:~1167-1190`; `start_again`, `faraday/shells/stick/src/main.rs:~1745-1795`). The fresh app is marked not clean (fix 0b5a4d79c).
- The panel is blanked as the process exits (fix e7410f184, `faraday/shells/stick/src/main.rs`).
- `exit_forgetting` (power off; `faraday/faraday-core/src/action.rs:1317-1326`): `import.clear_all()`, Inbox keeps only items not `dropped_at_lock()`, `session.wipe()`, `wallets.set_list_changed()`, `own_wipe()`, `lock_secrets_off_screen()`, `StorageCommand::ForgetBoxes`, `Command::Exit`. Also run from `act_inner` on PowerOff when the self-test failed (`faraday/faraday-core/src/action.rs:1405-1409`).
- The canvas glyph cache is emptied at a lock (`faraday/faraday-core/src/at_lock.rs:101-108`; fix 77545b296).

## State and lock

- Classification (`faraday/faraday-core/src/at_lock.rs`): `Holds { kept, typed, session }` (`~14-26`), trait `AtLock` (`holds`, `lock`), marker `Public` (implemented for bool, f32, u64, Text, Option, Vec, Cell, DisplayInfo, Outcome). The `faraday_areas!` listing (`~124-193`) generates `Faraday::holds`, `Faraday::lock_areas` and the test kit's `Area`. A new `Faraday` field must be placed in one of the two lists or the crate does not compile (`faraday/faraday-core/src/at_lock.rs:4-6`).
- Areas that answer `AtLock` (own `lock()`): canvas, drawing, kept, vaults, fresh, session, decode, inbox, entry, wallets, visit, import, glance, qr, spend_backup, vanity, backup, message, create, restore, keygen, wordlist, bip85, explore, lightning, tools, secrets, camera, seed_read, worker, flows, and transfer with the demo feature (`faraday/faraday-core/src/at_lock.rs:184-192`).
- `Public` fields (nothing to wipe; they live until the app ends): f, w, h, compact, pointer, out, clock_drawn, restart, restart_computer, last_display, settings, set_time, idle, tainted, inputs, now_ms, screen, sheet, sheet_offset, panel, stick, pin_h, list_offset, scroll, guide, typing, trail, disclosed, fade, selftest, online, seed_line, held, shell_sticks, clipboard, medium, secure_boot, maker, maker_family, maker_model, devices, next_mode, samples, sample_area, in_progress, outbox, memory, backups_area, silent, later (`faraday/faraday-core/src/at_lock.rs:177-183`).
- Per-area lock effects (verified): Drawing memo cleared (`faraday/faraday-core/src/drawing.rs:1021-1027`); Fresh wipes own randomness and the session seed, keeps the pool (`faraday/faraday-core/src/fresh.rs:409-422`, `1041-1053`); Inbox drops `dropped_at_lock` items and the potential wallet (`faraday/faraday-core/src/inbox.rs:2119-2135`); Create clears its whole state (`faraday/faraday-core/src/create.rs:2841-2852`); Glance clears only `work.pass` (`faraday/faraday-core/src/glance/mod.rs:2487-2494`; `work.text` is a name); Vanity drops its state with the grinder and its find (`faraday/faraday-core/src/vanity/mod.rs:652-660`); Wallets clears the spend (`clear_spend()` and `row_open.lock()`, `faraday/faraday-core/src/wallet/state.rs:1264-1274`); the session wipes keys, wallets, asked queue, confirmed, xpubs, silent_handed and keeps waiting, psbt_wallets, key_marks, accounts, modes (`faraday/faraday-core/src/wallet.rs:1539-1549`, `5568-5583`); Backup calls `close()`, dropping the whole `BackupState` and shown seed (`faraday/faraday-core/src/wallet_backup.rs:5283-5297`, `957-961`); the backup flow area wipes only the GPG restore lines (`faraday/faraday-core/src/backup_flow/area.rs:790-799`); the worker clears only the Krux code, running jobs are not cancelled (`faraday/faraday-core/src/worker.rs:414-426`, `555-562`); `Trail` is `Public` (`faraday/faraday-core/src/trail.rs:84-85`).
- Not wiped by a lock (`Public`): Idle, InProgress, Fade, TypingArea, InputsArea (counts carried), GuideArea, DeviceCheck. Fade frame copies are not wiped at lock (`faraday/faraday-core/src/fade.rs:164`, `193-205`, `395`); they go when the app ends.
- Flows across a lock: `lock_flows` (`faraday/faraday-core/src/vault/seal.rs:652-659`) runs before the wipes, with vaults still open: it ends a Make-a-new-wallet flow whose keys are kept nowhere (`lock_ends_create`, `create_drop`) and calls `flow_before_lock`; other flows keep where they are. Flows are kept in the kept state under `backup-flow` / `backup-gate` (`faraday/faraday-core/src/memory/mod.rs:506-515`, `620`); a flow's `live` part is not encoded and is rebuilt as `Live::default()` after the restart (`faraday/faraday-core/src/backup_flow/mod.rs:1973`).
- `Idle` (`Public`): last_input, carry, locking, warn_min, masked flag, under-sheet. `lock_at(min)` locks at `min*60_000` ms; 0 disables (`faraday/faraday-core/src/idle.rs:148-158`). A process that followed an idle lock carries the idle ms. Masked words are put away once per input.
- Clean state: a stick may be handed out for writing only when nothing is held or typed and the camera is off (`faraday/faraday-core/src/idle.rs:~160-185`). `holds().typed` marks the process not clean.
- Held keys (`faraday/faraday-core/src/held.rs`): no state of its own; it reads `flows.made_here/copied/sealed_for` and `session.waiting*`, and holds fingerprints and digests only. A passphrase or codex32 string revealed with `Reveal::VaultEntry` is moved into vault records.
- What a lock leaves in the next app: the kept state, the Inbox and Outbox items not dropped at lock, the vault summaries (see `vault-seal`). Secrets held by the app (keys, typed text, shown values) do not survive: they are cleared by each area's `lock()` and then the app ends.

## Reach

- Opens: the Lock sheet, the idle warning, the Locked sheet.
- Calls into `vault-seal` (`vault_seal_all`, `lock_finish`, `lock_secrets_off_screen`, `lock_after_seal`, `lock_after_seal_ended`, `lock_waits_for_seal`, `lock_would_lose`, `seal_waits`).
- Every area's `AtLock::lock`; the passphrase fields and their eye (see `secrets`).
- `lock_held` is read by idle (`faraday/faraday-core/src/idle.rs:232-236`) and by power-off (`faraday/faraday-core/src/state.rs:2540`, `2587`).
- Held keys: read by Glance (receive), Explore and the QR view (held keys open no address, public key or xpub code), the silent payments page, the backup flow (`backup_settle`, `backup_vault_save`, `backup_first`, `backup_reachable`), Files (a held key's signed file).
- A results-alone page does not draw the flow underneath, so a secret it shows is gone at once (`faraday/faraday-core/src/screens/hub_layout/page.rs:~73-74`).

## Rules and refusals

- `lock_ask` asks first in the five cases listed under Screens (`faraday/faraday-core/src/idle.rs:212`).
- `lock()` stops with the Lock sheet when a vault would lose changes, and waits for a seal when seal answers are not ready (`faraday/faraday-core/src/idle.rs:242`).
- A Lock pressed while a seal of one vault runs is remembered in the seal's own state and done when the seal ends: the vault is sealed first, then the device locks; on seal failure the Lock still goes ahead after the error toast (fix 320ab235d).
- Every secret on show is taken off the screen the moment Lock is pressed, even while the lock waits for a seal (`lock_secrets_off_screen`; fixes 502c1fadb, db9a171b1, 6305debeb). One list (`secrets_off_screen`, `faraday/faraday-core/src/vault/seal.rs:151-199`) says what a secret on show is; the lock and the idle warning both call it (fix bfa09c241). The idle warning keeps the Seed XOR parts; a lock drops them (`backup.drop_paper`).
- `not_kept` keeps the process when keys waiting for backup would be lost (`faraday/faraday-core/src/idle.rs:~273-278`).
- A full lock asserts (debug builds) that nothing is held after the clears (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- A failed self-test drops every press except PowerOff (`faraday/faraday-core/src/action.rs:1405-1410`).
- Every area holding a secret holds the stick (no stick is handed out for writing) and triggers the idle lock, also when no key or vault is loaded (fixes 61647f490, 5cc494c97). Typed secrets include: words on the word keyboard, a typed key in any Restore box, a custom path in Restore or Spend, a TOTP setup code carried to Unlock, a secret read on a scanner under a sheet, a partly scanned transfer, codex32 strings, SLIP-39 shares and Seed XOR parts, dice rolled for a new vault's passphrase (fixes 3c9493d13, f03dea9f6, 234004219, 9e2d99e5b, b538d6f1a, a12df9536, 95c4b4b33, 22eff8941). An open Restore holds no secret by itself (fix 6b89cd342).
- Held keys:
  - A key is held when made here, loaded from a vault record marked held, or held by a flow until its last stick is written (`faraday/faraday-core/src/held.rs:~1228-1290`).
  - `backup_settle` (`faraday/faraday-core/src/held.rs:~595`) runs at the Checklist stage when every item except Sticks is done; it releases held seeds except those a vault keeps with writes pending or on another stick.
  - `copy_clears` refuses to clear while another passphrase of the same words is being made here; `held_unlock_clear` clears a vault mark only when it was made this session, all copies are written and none is made here or held by a flow (`faraday/faraday-core/src/held.rs:~736-783`).
  - A backup hold releases only keys with the same words in the same word list (fix 259a1eb3f). A hold lasts until a copy holding it is written, across lock, unlock and restart (fix 1f9ca48db).
  - `backup_vault_save` (`faraday/faraday-core/src/held.rs:~952`): writes seeds, the passphrase record, codex32 share records, the wallet record and share sheets, and the plan, in one press. It is refused when `plan_refuses` (a plan kept for another key order). It asks first for codex32 sets and BIP-85 origins through `vault::keep::gate`. Share numbers clamp to 255 with no message. Wallet shares are written only where `sheets_rebuild` says they rebuild the wallet, else toast `backup_sheets_rebuild_another`.
  - `backup_first` opens Backup directly only at the Plan stage of the same wallet; `backup_reachable` opens an item only if it is on the checklist and all earlier items are done; Load at unlock toggles (F62) only for the current vault target (`faraday/faraday-core/src/held.rs:~900-946`).
  - A held key's signed message file stays in Files: no write takes it, Show as QR is refused, the row says why (fix b3c39b886). A PNG of a signed code is held as the `.txt` is (fix e258c6ee9). A hold is released only by the signing key, all 32 bytes of its id (fix 9c881c4b2); the signer id and fingerprint are taken at the signing step and kept beside the signature (fixes 4bddaaca1, 00fdb9dd2). A held signed file stays held after Remove every key until its key is seen loaded and released with its backup done (fix 21755fca8). A removed wallet drops its signed message (fix 0cd3b1eb5).
  - Held keys open no address, public key or xpub code; Power off holds while a key or wallet has no other copy (fix 6b6f01d22).
- Leaving a step in any contained flow hides what it showed (the container calls each flow's `hide_shown`); a step with a secret is drawn from state with no image kept, and hidden when the fade ends (fixes f4aed390e, 9ed43cc4c, 502fd1bee, 9b1480bf6).
- Closing At a glance's passphrase check by a press elsewhere or by a row leaves no typed passphrase, result or refusal in state (fix d2fd9f471). Words typed into Check my copy are a typed secret (fix 307710fb1).
- A held keyboard or mouse used while the lock notice shows takes the screen; the lock notice comes back once the device is let in or kept out (fix 353f52397).

## Domain calls

- `faraday_keys::session` (`faraday/faraday-keys/src/session.rs`): `Reveal`, `Held`, the session's `wipe`.
- `faraday_keys::vault` (`faraday/faraday-keys/src/vault/mod.rs`): `RecordView`, `key_record_id`.
- `faraday_vault::records` (`faraday/faraday-vault/src/records.rs`): the records written by `backup_vault_save`.
- `faraday_shell_api` `Command::Exit`, `StorageCommand::ForgetBoxes`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 6b89cd342 | A restore with vaults was refused its vault sticks because an open Restore counted as holding a secret | An open Restore holds no secret by itself; the vault sticks are written |
| f03dea9f6 | A key typed into a Restore cosigner box was not a typed secret: the app could read clean and a stick be handed out while it was held | A typed key in any Restore box is a typed secret |
| 22eff8941 | Dice rolled for a new vault's passphrase did not keep the stick from being handed out, and the rolls outlived the form | Dice rolls are a held secret while the form is open and go when it closes |
| 9e2d99e5b | A TOTP setup code carried to Unlock did not hold the stick | A carried setup code is a held secret |
| b538d6f1a | A secret read on a scanner waiting under a sheet did not hold the stick | A scanned secret under a sheet is held |
| a12df9536 | A transfer scanned in part and kept after the camera closed did not hold the stick | A partly scanned transfer is a held secret |
| 95c4b4b33 | A stick was handed out for writing while a codex32 string, SLIP-39 share or Seed XOR part was held, scanned ones included | Those parts are held; no stick is handed out |
| 3c9493d13 | Seed words typed on the word keyboard were not a typed secret: no stick hold, no idle lock, no fade | Keyboard-typed words count as a typed secret |
| 5cc494c97 | The idle lock did not come while a tool held a typed secret and no key or vault was loaded | A tool-held secret triggers the idle lock |
| 61647f490 | A stick was not held back and the idle lock did not come while some tool or form held a secret; Lock missed some areas | Every area holding a secret holds the stick and clears at Lock |
| fe2344bf5 | Lock did not clear the Krux code, Bitcoin Core wallet, Lightning, Vanity, Sweep, calculator typed text or the camera's kept transfer | Lock clears every tool's typed text and kept transfer |
| 38e91c760 | Lock did not wipe a new vault's passphrases and dice rolls | Lock clears them |
| e7410f184 | After a lock the last screen stayed in the framebuffer until the next one drew | The panel is blanked as the process exits |
| 0b5a4d79c | A Lock whose save failed with a backup waiting kept the old app in the process | A fresh app is built in the same process, marked not clean |
| bec169c60 | A failed save at Lock lost which Outbox files were secret; a transfer part read from a stick picture was not held | The secret marks survive a failed save; a picture-read part is held until Lock |
| d2fd9f471 | Closing At a glance's passphrase check by a press elsewhere or by a row left the typed passphrase, result and refusal in app state | A secret is not reachable in app state after its screen is done |
| 307710fb1 | Words typed into Check my copy were not a typed secret, so the screen could be a fade's source | A secret typed on a screen is not kept in state or a fade |
| 234004219 | A path typed into Restore's or Spend's custom path box was not a typed secret | Typed secret text is not kept after its screen |
| 9ed43cc4c | The fade of a step that shows or takes a secret kept an image of the secret's screen | A secret step is drawn from state with no image kept; hidden when the fade ends |
| 502fd1bee | A step of Make a new vault left from the Your choices sheet did not hide a shown passphrase | The hide follows the step change when no live fade carries it |
| 9b1480bf6 | A step left with Show on and nothing typed did not hide the eye | The hide runs on every step change no live fade carries |
| f4aed390e | Leaving a step in a flow other than the vault flow left a passphrase eye, typed secret or shown value visible; reopening showed it unhidden | A step left hides what it showed in every contained flow (the container calls each flow's `hide_shown`) |
| 21755fca8 | After Lock and Remove every key, a signed file of a held key was no longer held | A held signed file stays held until its key is seen loaded and released with its backup done |
| 9c881c4b2 | A different key with the same fingerprint released a signed file's hold | A hold is released only by the signing key (all 32 bytes of its id) |
| 4bddaaca1 | A PNG made after Remove every key had no signer, so its hold did not outlast a lock; a same-fingerprint key could be recorded instead | The signer id is taken at the signing step, kept beside the signature, dropped with it |
| 00fdb9dd2 | A picture made after its wallet was removed lost its signer; a signature dropped while its code was shown left the code open | Fingerprint kept beside the id at signing; a dropped signature closes its code |
| 0cd3b1eb5 | When the wallet a message was signed with was removed, the signature, message and code stayed shown | A removed wallet drops its signed message |
| e258c6ee9 | A PNG of a signed code made before its key was held was an ordinary file the stick took | The PNG is held back as the .txt is, recorded with the signing key |
| b3c39b886 | A held key's signed message file could be written to a stick or shown as a code | Such a file stays in Files; no write takes it, Show as QR refused, row says why |
| f537c3ffa | The Lock sheet offered "Back to the vault" with no vault open, a way out that went nowhere | Back to the vault only while a vault is open to return to; otherwise Cancel, which wipes nothing |
| 502c1fadb | A code on show (may be an entry's TOTP secret) stayed on screen for the whole length of a seal that a pending Lock was waiting on | A secret on show is taken off the screen as soon as a Lock is pressed, even while the lock itself still waits for the seal |
| db9a171b1 | A Lock pressed during a vault seal did not take every secret on show off the screen at once (codex32, keygen, vault, wallet backup) | Every secret on show is hidden at the moment a Lock is pressed, including during a seal |
| 6305debeb | During a seal a Lock left on screen what shows a secret behind an eye or a code (BIP-85, Convert, keygen, seeds KEF) | A Lock hides every eye-gated secret and code at once, seal or not |
| 77545b296 | The canvas glyph cache kept characters of a shown passphrase or seed word after Lock | Lock empties the glyph cache |
| 838ea6938 | At a lock the signing page's file text (the file's text and the lines broken from it) stayed in app state | A lock wipes and drops the kept texts; the secret is not reachable after the lock |
| 353f52397 | A held keyboard or mouse used while the lock notice showed after an idle lock could not be answered; the lock notice was lost | The held device's sheet takes the screen, and the lock notice comes back once the device is let in or kept out |
| bfa09c241 | Secrets on show were hidden by separate code paths, so some stayed visible after a Lock or the idle warning (Seed XOR parts, Lightning private key, Vanity passphrase, inbox passphrase, Create a vault passphrases, scanned text shown unmasked) | One list (`secrets_off_screen`, in vault/seal.rs) says what a secret on show is; the lock and the idle warning both call it; typed input stays where it is held |
| 3cc36fdc4 | The Locked sheet said every key was wiped even where a plain secret still waited in the Outbox; it said Locked after a power-off held with nothing locked | A waiting plain secret is named in the error tone and not wiped; Locked only after a lock; Not locked where nothing was locked |
| 320ab235d | A Lock pressed while a seal of one vault ran (Back up's New vault) was lost or misordered; a failed seal could leave the Lock undone | The Lock is remembered in the seal's own state and done when the seal ends; on seal failure the Lock still goes ahead after the error toast |
| 92d3e097b | Only scripted presses ran the post-press clean-up; after a tap, a key or the mouse's back button a screen left still held New key, BIP-85, Explore, Lightning, Silent, the calculators, Vanity, the word list, pending sheets; a spent threshold round stayed in its vault | A screen left drops what it held, however the press came |
| 82bd3b685 | A removal on the published-words sheet did not set the wallet list changed, so the waiting words could be stale | A removal sets the wallet list changed; the waiting words' change test hashes the sealed files |
| 259a1eb3f | The backup hold was released for keys whose entropy matched a key in another BIP-39 word list, which is a different wallet | A backup hold releases only on keys with the same words in the same word list |
| 1f9ca48db | A key whose words had another passphrase waiting for its backup was released too early or kept across lock, unlock or restart | The hold lasts until a copy holding it is written, across lock, unlock and restart |
| b0d3dd319 | The idle warning did not keep a checklist's Seed XOR parts hidden; a codex32 string with text typed back was not closed; a lock did not put away the chart's passphrase sheet | The idle warning and lock hide Seed XOR parts; a shut codex32 string is not drawn once text is typed back; a lock puts the passphrase sheet away |
| 8e682d57c | The idle warning did not come over every sheet; a code on the QR sheet stayed on screen; a full capped field kept the old text; the sheet under a closed sheet did not come back | A lock's own sheet is the only one exempt from the idle warning; a secret code is not left on screen; a capped field keeps what is typed up to its cap; the sheet beneath returns |
| 6b6f01d22 | Explore of a key made here and waiting for its backup opened an address code, and a new wallet's key codes gave no warning; Power off did not hold while a key or wallet was kept nowhere else | Held keys open no address, public key or xpub code; the warning shows on the descriptor and key codes; Power off holds while a key or wallet has no other copy |
| 2b550d917 | An idle lock that could not seal gave no notice on the Locked sheet that changes were lost (also a wrong count in a sealed copy) | A failed idle-lock seal says changes were lost |
| f65ef4ddc | An idle lock that could not allocate a vault's newly chosen memory step sealed silently at the old step | The next app's Locked sheet says the step was not raised; the changes and any passphrase are kept |

## Doubts

- Rows moved here: 6b6f01d22 (given under one-time codes; it is about held keys and power-off), 2b550d917 and f65ef4ddc (given under vaults; they are about the Locked sheet).
- Rows moved to `secrets`: 5bd9bcc00, 636cac6a2, d3a1fe6e4, bc4b2a364, 7ed76a00b (classing seed text as secret).
- Rows left out as not about this feature: a28ea96c6 (a held list item's contrast, "held" as pressed), f8df718b3 (Tab and Enter on hold-to-confirm controls), 70ec0c230, fb16bd0cb, cba91f0e6 (fade drawing: position, motion and veil colour; they belong to the fade feature).
- The Demo shell's lock path was not traced here beyond "Demo lock clears dirs, seen, toggles, served, keeps boxes, preview, attached" (`faraday/shells/demo/src/lib.rs:317-335`). Whether the Demo ends the app or rebuilds it at a lock is not stated by the inventory.
- `not_kept` and the stick shell's `start_again` were read directly for this file; their line numbers are not inventory-verified.
- Whether each area's `lock()` impl clears all it holds was read only for the areas named under State; the others (entry, restore, keygen, message, decode, explore, lightning, tools, camera, seed_read, bip85, wordlist) were only seen as listed.
- `Fade` frames, the `Public` areas and every value no area's `lock()` clears go only when the app ends; in the not-kept path the old app is dropped (not the process), so heap is not returned to the kernel at that point.
- Stale doc comments were noted in `faraday/faraday-core/src/held.rs:~258-260`, `~689-694` and the doc lists of `faraday/faraday-core/src/idle.rs` (not re-read).
- `faraday/faraday-core/src/in_progress.rs:~1164`: a Vault flow never counts as finished (open, unverified).
