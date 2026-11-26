# Randomness and dice

The randomness that keys, shares, passphrases and signing rounds are made
from. The session draws its randomness from a seed the shell gives
(`fresh`). A person may add their own (Add randomness, `Screen::Randomness`)
from the camera, dice, coins, cards or pointer and keys. New key takes its
entropy from a chosen source or a mix: dice, coins, cards, hex, camera noise,
or this device's generator. Create a vault can make a passphrase from dice
(`dice_entry`). Faraday Demo offers a one-die roller and a five-dice roller
that stand in for real dice. Add randomness is reached from a Randomness row
on New key, Create a vault, Vault contents, the backup flow, Back up, Spend
and the Lock sheet, and from Settings.

## Screens and sheets

- Add randomness page (`faraday/faraday-core/src/fresh/page.rs`; drawn by `faraday/faraday-core/src/screens/randomness.rs`, `draw` `~213`): status, source choice (Camera, Dice, Coins, Cards, PointerKeys, `OwnSource`), pad, batch (Undo, Clear, Add), Another, Done.
- The Generator card in New key, the same page in a card.
- Randomness row (`row`/`row_in`, `faraday/faraday-core/src/screens/randomness.rs:~54-110`) on the screens that may use it.
- New key's entry cards (`faraday/faraday-core/src/keygen/entries.rs`): ways in `WAYS` (`faraday/faraday-core/src/keygen/mod.rs:~99-110`, groups `ByHand`, `Computed`, `Device`): DiceWords (direct selection, 6 rolls per word), DiceFlips (die as coin: 1-3 tails, 4-6 heads), Coins, DiceHashed (the Coldcard/SeedSigner procedure), DiceSixAsZero (Keystone), Hex, Cards (rank then suit), Mix, Device.
- The dice strip "Word being made" (`faraday/faraday-core/src/keygen/dice_strip.rs`): slots, bits, the 11-bit number, the checksum bits only on the last word (`~108`, `~328`), "Rolled again" only for a direct-selection reroll (`~258`, `~271`), an eye mask (`Action::KShow`, `~460-482`).
- The dice pad (`faraday/faraday-core/src/keygen/dice_panel.rs`, `screens::dice_pad`), greyed when full.
- Words so far (`faraday/faraday-core/src/keygen/words_so_far.rs`): a pill per word, masked "• • •" when the strip is masked (`~95`); the overlay shows only at ENTER, not hosted, when `dice_strip::stands` (`~250-252`); a words page on compact windows only (`~253`); the word sheet shows rolls, bits, number, word and a checksum line (hidden while masked, `~400`).
- The one-die roller beside the pad (Practice in Faraday Demo only, `faraday/faraday-core/src/fresh/roller/mod.rs`, `faraday/faraday-core/src/fresh/roller/screen.rs`).
- The five-dice roller for the codex32 booklet's dice de-biasing worksheet (Faraday Demo only, `faraday/faraday-core/src/fresh/roller/five.rs`, `faraday/faraday-core/src/fresh/roller/five_screen.rs`).
- Create a vault's Generate with dice (`faraday/faraday-core/src/dice_entry/mod.rs`, `faraday/faraday-core/src/dice_entry/screen.rs`): the rolls typed and, in Faraday Demo, the roller beside them.

## Actions

- Add randomness: choose a source; enter a batch on the pad (Undo, Clear); Add; Another; Done. Before a source is chosen every press but `Source` is refused (`faraday/faraday-core/src/fresh/page.rs:305-310`).
- New key entries: `KRoll`, `KFlip`, `KDie`, `KByDie`, `KTyping`, `KRank`, `KSuit`, `KHex`, `KFrame` (take a camera frame), `KUndo`, `KClear`, `KMix`, `KWay`. Keys: digits 1-6 for dice, `h`/`t` or `1`/`0` for coins, hex digits, card letters, Space for a camera frame, Backspace to undo.
- `KRoller`: the one-die roller (`keygen_roller`, `faraday/faraday-core/src/keygen/mod.rs:~2483`; `keygen_roller_tick`, `~2502`).
- Create a vault dice: type rolls or press the pad; change the list; Use these words.

## Effects

- `Command::RequestEntropy` to the shell (`faraday/faraday-core/src/fresh.rs:~308`, `~327`, `~435`, `~521`, `~1004`, `~1014`); `lock_waits_for_seal` pushes it straight onto `out.commands`.
- The shell answers in Faraday Demo with 32 bytes from `getrandom::fill`, sent as `Event::EntropySource` then `Event::Entropy` (`faraday/shells/window/src/lib.rs:~39-51`); nothing is sent when the OS has none. The answer reaches the app through `entropy_arrived` (`faraday/faraday-core/src/keygen/mod.rs:2339-2346`): the bytes go to a New key or become the session seed.
- Camera on and off for noise. On New key the camera is on only at ENTER with Camera on screen (`faraday/faraday-core/src/keygen/area.rs:~432-436`).
- Key blinding: `keys_reblind` draws `"faraday key blinding"` after each batch.
- Rollers draw `app.fresh(b"faraday dice roller")` (`faraday/faraday-core/src/keygen/mod.rs:~2516`).

## State and lock

- Session randomness (`faraday/faraday-core/src/fresh.rs`): `Seed(Zeroizing<[u8;32]>)`, `pool: Vec<Zeroizing<[u8;32]>>`, the person's own accumulator `Own.acc: Option<Zeroizing<..>>`.
- Constants (`faraday/faraday-core/src/fresh.rs:40-53`): `POOL = 26` answers; `TAKE_MAX = 26*32` bytes; the pool keeps at most 52 and drops any surplus silently (`~296`); `NO_SEED_MS = 60_000` (decision F102).
- Lock (`faraday/faraday-core/src/fresh.rs:409-422`, `1041-1053`): wipes `own` and `page`, drops the seed, the waiting press and the sign draw count. The pool is kept on purpose. Only the person's own accumulator counts as a hold. The lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Add randomness page: `wipe` keeps `chosen` and `source` (`faraday/faraday-core/src/fresh/page.rs:293-302`).
- New key: entries live in `KeyGen` (`dice`, `coins`, `cards`, `hex`, `camera`, `mixed`, `device`) and go with the flow (see `keygen-flow`). The camera frame's luma is in `KeyGen.frame`, zeroized when replaced, when the camera goes off, and on drop (`faraday/faraday-core/src/keygen/area.rs:348-355`, `445-451`; `faraday/faraday-core/src/keygen/mod.rs:419-425`).
- Device bytes: `entropy()` takes bytes only when asked and zeroizes the buffer (`faraday/faraday-core/src/keygen/area.rs:~417-428`).
- `clear_entries` (`faraday/faraday-core/src/keygen/entries.rs:~10-45`) and `clear_active` (`~48-66`) clear the entry vectors; `secret` is not cleared there.
- `keygen.dice` (`codex32::DiceChoice`) is kept for the session.
- Create a vault dice: `DiceEntry` holds the rolls in a `vault::TextBox`, which holds a `SecretText` (`faraday/faraday-core/src/vault/area.rs:1016-1021`). Create a vault's lock closes the dice entry (`faraday/faraday-core/src/vault/area.rs:4477-4486`).
- The rollers remember which positions they made, so the result can say "simulated dice" (`faraday/faraday-core/src/fresh/roller/mod.rs`, `faraday/faraday-core/src/dice_entry/mod.rs`).
- Every faraday-entropy accumulator wipes on drop and has no `Debug`: `DiceRolls`, `CoinFlips`, `CardDraws`, `CameraNoise`, `PointerKeys`, `DeviceRandom`, `RawHex`, `Mixed`, `RawEntropy`, `SeedXor` and `BookletChar` (for example `core/faraday-entropy/src/dice.rs:99-112`). `OwnPage`'s `Drop` wipes the camera frame (`faraday/faraday-core/src/fresh/page.rs:95-101`). The rollers' answers are `Zeroizing` (`faraday/faraday-core/src/fresh/roller/mod.rs:121-122`); their last face is a plain `u8`, in Faraday Demo only.

## Reach

- Read by New key, SLIP-39 dealing, codex32 Suggest, Seed XOR, signing (MuSig2 rounds), the vault area (create, Secure Boot enrol and sign, KeePass prompt), the backup flow's Make, Back up's codex32 make, Convert's split, the Krux code form.
- Add randomness is opened by the Randomness row from New key, Create a vault, Vault contents, the backup flow, Back up, Spend and the Lock sheet (`faraday/faraday-core/src/screens/randomness.rs:118-156`); `Screen::Randomness` is listed at `faraday/faraday-core/src/compact/mod.rs:99`, and Settings has a Randomness row (`faraday/faraday-core/src/compact/mod.rs:64`).
- New key and the Krux code tool survive a visit to Add randomness (`faraday/faraday-core/src/pointer.rs:832-838`; `faraday/faraday-core/src/kef_tool/mod.rs:601-615`).
- Faraday Demo's "Add randomness" offers pointer and keys (`faraday/shells/demo/src/lib.rs:~435-436`); the dice roller is offered only in the Demo (`faraday/shells/demo/src/lib.rs:~433-434`).
- See `docs/OWN-RANDOMNESS.md` §2.2-2.3 for the person's own randomness and `docs/CODEX32.md` §5.6 for the rollers.

## Rules and refusals

Session randomness
- Presses that draw system randomness wait for the seed and answers (`draws_system_randomness`, `faraday/faraday-core/src/fresh.rs:~931-970`): KNext at the SLIP-39 Enter step, codex32 Suggest, BXor, SignHere with a MuSig2 round, the vault's CGo, SbEnrol, SbHoldSign and PromptGo for KeePass at or above the minimum, the flow's Make, and BC32 MakeNew.
- After a minute without a seed they give up with `no_randomness` (`faraday/faraday-core/src/fresh.rs:~502-511`, `~975-988`).
- A MuSig2 round never opens on unseeded bytes (`faraday/faraday-core/src/fresh.rs:~433-437`).
- An empty batch is not counted (`faraday/faraday-core/src/fresh.rs:~207-211`).
- `take_bytes` above `TAKE_MAX` is refused.
- PointerKeys is refused unless the shell offers pointer and keys (`faraday/faraday-core/src/fresh.rs:~697`); the roller only in Faraday Demo's Practice (`~583-585`).

Add randomness and its row
- The row shows only where it can be used (`faraday/faraday-core/src/screens/randomness.rs:118-156`): New key not locked and either at the SLIP-39 entry step or on the codex32 Identifier card with no work; Create a vault not running Create and either not compact or at PHRASES; the backup flow at Make, not a wallet kind, not made, not adding, not running FlowMake; Back up rotating a codex32 scheme, or revealing with no check and no paper; Spend with nothing finished, the open step 5, 6 or 9, and `spend_opens_round`.
- The Lock sheet row needs `!lock_would_lose_seen()` and an open vault with changes (`faraday/faraday-core/src/screens/randomness.rs:~161-163`).
- The vault press is refused while a prompt is up unless it is an unstarted KeePass prompt; sign images only with an `Ok` signature; refused while a form, a save or a signing is in progress; shown when there are changes or a Secure Boot record is selected with no waits (`~170-191`).
- Camera is a source only where cameras exist or `wants_camera`; PointerKeys only where `pointer_keys_offered()` (`~288-292`).
- On the Generator card nothing is chosen until a source is pressed; pad and batch show only when `chosen()` (`~296-307`).
- Add is taken only when the batch is not empty (`~476-480`). A dice batch of at least `DiceRolls::needed(Bits256)` shows the "256 bits enough" line (`~461-465`). The Done state shows only when `p.done()` (`~233-244`).

New key sources
- `Way::offered` is false for DiceWords under SLIP-39 (`faraday/faraday-core/src/keygen/mod.rs:~156-158`).
- Hex takes lower case only (`faraday/faraday-core/src/keygen/mod.rs:~1066-1070`).
- Cards: a card already drawn is refused with `keygen_card_already_drawn` (`~1056-1058`).
- Mix needs at least `MIN_MIX` sources and at least one Dice or Device source (`~1231-1237`); the camera is never a key's only source (`~1137-1139`).
- Camera noise: no frame gives `keygen_no_camera_picture` (`~1079`); a repeated frame `keygen_picture_already_taken` (`~1085`); too little variation `keygen_too_little_variation` (`~1087`).
- Device: `Command::RequestEntropy` only at ENTER, with Device active, not yet enough, and not already asked (`faraday/faraday-core/src/keygen/area.rs:~401-411`).
- Own randomness ("Add randomness" on New key: `with_own`, `own_offered`, `own_changed`): offered only before Lock in, at ENTER, with the Device source (`faraday/faraday-core/src/keygen/area.rs:~182-185`); a change wipes the device bytes only before words exist (`~375-383`).

New key entries (`faraday/faraday-core/src/keygen/entries.rs`)
- A roll is ignored unless Dice is active and not full (`~185-196`).
- Direct selection: a 5 or 6 among the first five rolls of a word is a "rerolled" face and is not recorded (`~197-203`).
- Other procedures refuse 5 and 6 with `keygen_a_die_has_six` (`~217-218`).
- A flip is taken only for Coins or Dice-by-die and not when full (`~175-181`, `~256-259`); a die-as-flip face outside 1..=6 is refused (`~267-270`).
- Typing beyond what is needed is refused with `keygen_no_more_are` (`~296-299`).
- The entered record is capped at 1024 (`~168`).
- Roller digits only while `roller_typing` (`~135-142`); `roller_room` is zero unless Dice is active and not locked (`~153-155`).
- Words so far: the last word is the checksum word and live words never hold it (`faraday/faraday-core/src/keygen/words_so_far.rs:~36`, `~41-45`, `~180-185`); the gliding word is hidden from the grid (`~39-40`).

Rollers
- The one-die roller is offered only in Practice in Faraday Demo (`roller_offered`, `faraday/faraday-core/src/keygen/mod.rs:~2485`); never on the stick, the Pi or the website (`faraday/faraday-core/src/fresh/roller/mod.rs`).
- The five-dice roller takes faces 1 to N of the chosen dice, six unless told, up to 20 (`faraday/faraday-core/src/fresh/roller/five.rs`).

Create a vault dice (`faraday/faraday-core/src/dice_entry/mod.rs`)
- A passphrase from dice counts as strong at 77 bits, six words of the EFF long list (`STRONG_BITS`).
- `use_words` returns `Used` even when no word was made; changing the list clears the rolls (`faraday/faraday-core/src/dice_entry/mod.rs:204-213`, `241-252`).

## Domain calls

- `core/faraday-entropy`: `DiceRolls`, `CoinFlips`, `CardDraws`, `RawHex`, `CameraNoise`, `Mixed`, `DeviceRandom`, `DiceProcedure`, `Strength`, `MIN_MIX`, `DICE_WORD_ROLLS`, `DICE_WORD_FACES`, `booklet` (for the five-dice roller).
- `core/faraday-bip`: `diceware::List` (Create a vault dice).
- `core/faraday-shell-api`: `Command::RequestEntropy`, `Command::CameraOff`, `EntropyBytes`.

## Fixtures

None.

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 56479fe6a | A new key's dice on Make a new wallet's Keys card started on Buttons even on a wide window, where the keyboard stays usable. | Dice start on Typed on a window of 600 dp or more and on Buttons on a small panel. |
| 1cbd24b0c | On a one-column layout a press on Typed, or on the box of rolls, in a new key's entries did not bring the keyboard up. | A field press brings the keyboard up. |
| 83fc831c1 | On the small panel the dice-pad keys, "Generate with dice" and the coin, die, typed and buttons choices were smaller than a touch target; a die's pad had the wrong column count. | The pad is four columns of face keys plus Backspace, wrapping to more rows; keys reach the touch target size; the Pi panel's dice pad is drawn in its card again. |
| 60f23e515 | On Create a vault, after Generate with dice with no word rolled yet, the first word moved the rows below it, so Use these words and Cancel slid under the dice panel. | "No word yet" holds the first row's room, so the first word moves nothing below it. |
| a8fd9ad8e | On a one-column layout Add randomness's dice started on Typed and a press on the Typed box did not bring the keyboard up, so typed rolls were lost. | Add randomness's dice start on Buttons as New key's do; a press on Typed brings the keyboard up. |

## Doubts

- `Skip`, `Another` and `Area` of Add randomness fall to `_ => {}` in `act()`; `Act::Roll(f)` and the card `Rank`/`Suit` are not range-checked in the page. Unconfirmed whether this is reachable.
- The Spend open steps 5, 6 and 9 are bare numbers with no names.
- A dice sample file for Practice, `tools/vectors/entropy/dice.json`, is named in the samples' docs but is not in the kept fixture list.
- Whether `faraday/faraday-core/src/stick/settings.rs` `layout_scale` dropping a newly added `Command::RequestEntropy` (`faraday/faraday-core/src/stick/settings.rs:~483-491`) is intended is unclear.
