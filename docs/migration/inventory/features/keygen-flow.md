# Making a new key

New key is the flow that makes a key on the device, or takes one typed or
scanned, and loads it into the session. It is for anyone who needs a seed:
a person making a first wallet, or adding a key to a multi-key wallet. It is
reached as its own screen (`Screen::KeyGen`), as the Keys card of Make a new
wallet (`Screen::Create`, "on card"), and from Tools, where it ends on a
result card. Its cards are Start, Length, Source, Enter, Key, Check, Words
and Quiz; SLIP-39 shares and codex32 shares are made inside the same flow
(codex32 making is described in `codex-shares`).

## Screens and sheets

- `Screen::KeyGen` (`faraday/faraday-core/src/keygen/screen.rs:~106`).
- Hosted as the Keys card of Make a new wallet (`faraday/faraday-core/src/keygen/on_card.rs`).
- Result card for a key made from Tools (`made_body`, `faraday/faraday-core/src/keygen/screen.rs:~187`).
- Sheets: `Sheet::Pull` (pull the stick, `faraday/faraday-core/src/keygen/mod.rs:~1368`, `~2894`); `Sheet::CancelKey` (`faraday/faraday-core/src/keygen/mod.rs:~1553`); `Sheet::KeyLengths` (`faraday/faraday-core/src/keygen/mod.rs:~1807`, `~1848`); the words-so-far sheet or page overlay (`faraday/faraday-core/src/keygen/words_so_far.rs`).
- Entry points: `keygen_open` (`faraday/faraday-core/src/keygen/mod.rs:~1341`), `keygen_open_with` (`~1358`), `keygen_open_on` (`~1581`), `keygen_open_dice` (`~1596`), `keygen_open_card` (`~2890`).
- Start card (`faraday/faraday-core/src/keygen/start.rs`): `Start::{Make(Form), Type(Form), Scan}`, nine starts (`STARTS`); six shown first (`UP_FRONT`), the rest under Other forms (`faraday/faraday-core/src/keygen/screen.rs:~328-378`).
- Step indices `kstep::{START, LENGTH, SOURCE, ENTER, TYPED, KEY, CHECK, WORDS, QUIZ, CODEX32...}`; `KEYGEN_STEPS` (`faraday/faraday-core/src/keygen/steps.rs:~24-27`) feeds Back, Forward and the Your choices pane. The made card is `MADE` = 99; codex32 cards are 100 and up.
- Key card (`faraday/faraday-core/src/keygen/screen.rs:~715`): the words grid, Show/Hide, the passphrase pair (only when `asks_passphrase()`), Lock in. After Lock in: a passphrase Set/Not set pill, the fingerprint, and the descriptor and first address for a single-key slot, or the key text for a multi-key slot (`~862-879`). Read-only once locked (`~599-604`, `~618`).
- Check card (`faraday/faraday-core/src/keygen/screen.rs:~2128`, `check_rows` `~2156`): per-source counts (dice faces, chi-square, longest run; coins; card suits; hex count; camera frames; the mix's SHA-256 commitment; device rows) and a verdict, "Fair: no cautions" or each caution (`~648-666`).
- Words card (`faraday/faraday-core/src/keygen/screen.rs:~2423`): BIP-39 words hidden until shown (`~702-706`); fingerprint, source, key text. For SLIP-39 it shows one share at a time with Show, Previous share, Next share, and "I wrote them all" (`~2321-2418`).
- Quiz card (`faraday/faraday-core/src/quiz.rs`, drawn at `faraday/faraday-core/src/keygen/screen.rs:~2488`): four choices per word.
- Result card from Tools: fingerprint, name, whether typed, rows; the line "Nothing of the key is kept"; buttons Make a wallet, Another, Done.

## Actions

- From outside: `Action::KeyGen(Option<u8>)`, `KeyOtherWays(u8)`, `KeyOnCard(u8)`, `KeyGenSlip39`.
- `KStart(i)`: choose a start; pressing the chosen start again moves on. `KNext`, `KLock`, `KOtherLengths`, `KWords(n)`, `KAdd`, `KRoller`, `KStep`, `KShow`, `KQuiz(i)`, `KQuizRetry`, `KSkip`, `KMadeWallet`, `KMadeAnother`, `KMadeDone`, `ScanSeed`, `Entry(None)`, `CreateWallet`. Codex32 cards are opened through `Action::Codex32(C32::Card(n))`.
- Pressing New key again while entries exist raises `Sheet::CancelKey`; Discard reopens the flow as pressed (`faraday/faraday-core/src/keygen/mod.rs:~1548-1564`).
- `KeyGenArea::act` (`faraday/faraday-core/src/keygen/area.rs:~457-724`) handles `KOthers`, `KPassField`, `KStep`, `KSlipWords`, `KSlipM`, `KSlipN`, `KShare`, `KWords`, `KWay`, `KGroup`, `KMix`, `KNext`, the entry presses `KRoll`, `KFlip`, `KDie`, `KByDie`, `KTyping`, `KRank`, `KSuit`, `KHex`, the word view presses `KWordOpen`, `KWordClose`, `KWordsPage`, and `KFrame`, `KUndo`, `KClear`, `KAgain`, `KSkip`. Any other press does nothing.
- SLIP-39: `KSlipWords(n)` picks a word count from `counts()` and clears entries; `KSlipM`/`KSlipN` set the threshold and total; `KShare(d)` moves between shares.
- Typed key: `keygen_typed_next` (`faraday/faraday-core/src/keygen/mod.rs:~1748`), then `keygen_lock_typed` (`~2143`). Scanned SeedQR: `keygen_scanned` (`~1715`).
- Lock in: `keygen_lock` (`faraday/faraday-core/src/keygen/mod.rs:2016`).
- Quiz: pick one of four; a wrong answer offers Ask again or Show words; Skip needs two presses (`skip_ask`, then `skipped`, `faraday/faraday-core/src/keygen/area.rs:~709-716`).
- Keyboard (`keygen_key`, `faraday/faraday-core/src/keygen/mod.rs:~2737-2838`): passphrase typing, codex32 keys, typing into the box, digits 1-6 for dice, `h`/`t` or `1`/`0` for coins, hex digits, card letters, Space takes a camera frame, Backspace undoes, Enter is Continue or Add, `1`-`4` answer the quiz.
- On a Create card: `card_dice`, `reveals_as_made`, `card_forget_words` (`faraday/faraday-core/src/keygen/on_card.rs:~10-33`); `keygen_open_card` (`faraday/faraday-core/src/keygen/mod.rs:~2890`), `keygen_card_tidy` (`~2915`), `keygen_open_on_card` (`~2934`).
- Result card: `keygen_done` (`faraday/faraday-core/src/keygen/mod.rs:~2238`), `keygen_made_act` (`~2300`): Make a wallet (`CreateWallet`), Another, Done.

## Effects

- Session keys: `session.keys.add_words`, `add_words_with`, `add_seed` (`faraday/faraday-core/src/keygen/mod.rs:~1975-2081`); `add_codex32` for codex32 (`~2672`).
- `mark_made_here(fp)` (`faraday/faraday-core/src/keygen/mod.rs:~1991`, `~2093`, `~2685`): the key must be backed up before it signs.
- `session.key_marks.note` (`~2195`, `~2688`).
- Make a new wallet: `create.fill_slot` (`~1995`, `~2127`); `create_make()` for a single-key Create (`~2129`); `refresh_spend()`.
- Labels "New key n" from the `keygen.new_keys` counter (`~1961`, `~2066`, `~2652`).
- Randomness: SLIP-39 split bytes `fresh_bytes(b"slip39 split", n)` (`~1860`); `Command::RequestEntropy` for the device source; `Command::CameraOff` for camera noise (see `randomness`).
- No file, stick or clipboard writes.

## State and lock

- `KeyGenArea` (`faraday/faraday-core/src/keygen/area.rs:~9-24`): `state: Option<KeyGen>`, `new_keys: u32`, `camera_on`, `entropy_source`, `dice: codex32::DiceChoice` (kept for the session), `reopen`.
- `KeyGen` (`faraday/faraday-core/src/keygen/mod.rs:~227-417`) holds secrets: `secret` (the SLIP-39 master secret, `Zeroizing`), `shares`, `mnemonic`, `entered: SecretText` (typed key text), `passphrase: PassField`, the entropy accumulators (`dice`, `coins`, `cards`, `hex`, `camera`, `mixed`, `device`), `codex32: Option<Codex32Make>`, `frame` (the camera frame's luma).
- `Text` and `SecretText` wipe when dropped or cleared (`core/faraday-text/src/text.rs:363-367`, `faraday/faraday-intake/src/secret_buf.rs:21-25`).
- Lock: `AtLock for KeyGenArea` forgets the whole flow (`faraday/faraday-core/src/keygen/area.rs:728-741`). `holds` counts typed input only when the flow is not on a card, or when the person entered their own randomness on it (`!on_card || entered_own()`). A lock then restarts the process (`faraday/faraday-core/src/vault/seal.rs:933-946`).
- Leaving the screen: `faraday/faraday-core/src/pointer.rs:832-838` drops the flow unless the screen is New key, Randomness or the Make a new wallet card; `keygen_card_tidy` ends the on-card flow. `keygen_drop` (`faraday/faraday-core/src/keygen/mod.rs:2320-2324`) takes the state and puts back `entry_kept`.
- `hide_left_step` and `hide_secrets` keep the flow but hide the words and the codex32 QR (`faraday/faraday-core/src/keygen/area.rs:~147-172`). Opening a step sets `shown = false` (decision F104).
- After Lock in the key is in `session.keys`; the flow keeps its words until it ends. The Tools result card keeps only fingerprint, name, typed flag and rows (`faraday/faraday-core/src/keygen/mod.rs:~2243-2268`).
- Passphrase fields are emptied when their screen goes (`faraday/faraday-core/src/pass_field/mod.rs:623-639`) and when a stick arrives (`faraday/faraday-core/src/pass_field/mod.rs:642-644`).
- `Mnemonic` (`core/faraday-bip/src/bip39.rs:759`) and `slip39::Share` (`core/faraday-bip/src/slip39.rs:574`) wipe on drop and have no `Debug` or `Clone`. `Codex32Make` (`faraday/faraday-core/src/codex32/mod.rs:466`) has no `Drop` or `Debug`; its secret fields each wipe themselves: `Zeroizing` rolls, `BookletChar`, `Made`, `SecretText`, `Held<Vec<Text>>`, `Codex32` and the `WorkSet` (`faraday/faraday-keys/src/codex32.rs:49-68`).

## Reach

- Opens: Add a key (`Entry(None)`, and Krux through `keygen_to_krux`, `faraday/faraday-core/src/keygen/mod.rs:~1693`), Add randomness (`Screen::Randomness`), the scanner (`ScanSeed`), Make a new wallet (`CreateWallet`), codex32 making (`codex-shares`).
- Opened from: Make a new wallet's Keys card, Tools, Add a key's Make a new key, the Wallets start page.
- Reads or uses: `forms::{Form, word_typer}`, `quiz`, `codex32::*`, `codex32_correct::Mark`, `secret_text`, `pass_field`, `fresh::roller`, `samples` (`samples::put::offer`, `faraday/faraday-core/src/keygen/screen.rs:1371`), `create::{NewKind, BackupForm, Source, SharesSet}`, `cstep::KEYS`, `wallet::{KeyTexts, parse_words, Refusal, fp_text}`, `flow`, `EntryState`, `screens::{keygen_page_w, dice_pad, randomness::*}`, `compact::eye_hit`, `at_lock`, `out::Out`, `word_entry::EntryList`, `wordlist`, `menu`, `session.keys`, `session.key_marks`, `session.wallets`, `vaults`, `backup`, `entry`, `camera`, `stick`, `visit`, `typing.pass_eye`.
- New key hosts the one-die roller, `Roller::default()` (`faraday/faraday-core/src/keygen/mod.rs:524`).
- The shell's entropy answer reaches the flow through `entropy_arrived` (`faraday/faraday-core/src/keygen/mod.rs:2339-2346`).

## Rules and refusals
- A New key result card names the key "Key {n}" with the number the key was made under (the counter after its increment), the same as the key's own label (faraday/faraday-core/src/keygen/mod.rs:1963, 1993, 2255).
- Lock in on the Key card: refused with no message while the passphrase pair differs (the pair's own line says so once the second field is begun) and while the eye has not shown a typed passphrase once (faraday/faraday-core/src/keygen/mod.rs:2027-2030, 2056). Scanned words that do not parse leave New key and go to the seed read, which sends them to Add a key (keygen/mod.rs:1715-1731; scan_key.rs:2601-2619).

Start and steps
- Make is offered only for Words and Codex32, and SLIP-39 through its own start; Xor, Krux, Xprv and HexSeed are never made here (`faraday/faraday-core/src/keygen/start.rs:~57-64`, `faraday/faraday-core/src/keygen/mod.rs:~633`, `~1676-1681`).
- `Start::form()` exists only for typed Words, ShareWords, Codex32 and PartWords (`faraday/faraday-core/src/keygen/start.rs:~78-86`).
- Start's Next is refused until a choice is made (`faraday/faraday-core/src/keygen/mod.rs:~1892-1896`).
- `KStep` is refused unless every earlier step is done and the step is in `steps()`; opening a step hides the secret (`faraday/faraday-core/src/keygen/area.rs:484-500`).
- Entries and the quiz show a value only when answered or open (`faraday/faraday-core/src/keygen/steps.rs:~124-129`); KEY counts as answered when done or locked (`~84`).
- After Lock in, Back may open the step after Key (or after Strings made) and never one before (`faraday/faraday-core/src/keygen/steps.rs:~37-48`).
- A roller click blocks Back (`faraday/faraday-core/src/keygen/steps.rs:~169-170`).
- Changing Length keeps the Source answer and clears later ones (`faraday/faraday-core/src/keygen/steps.rs:~243-248`).
- A typed-in set has no identifier, entropy or entry card (`faraday/faraday-core/src/keygen/steps.rs:~50-51`); a key made from Tools has all steps answered (`~154-161`).
- After Lock in, `inputs_closed` takes only step, show, quiz and skip presses, and Next on done cards (`faraday/faraday-core/src/keygen/mod.rs:1878-1891`).

Gates
- No key loads while the stick is in the way: the Pull sheet if the self-test passed, else a toast (`faraday/faraday-core/src/keygen/mod.rs:~1359-1376`, `~2891-2902`).
- No key is accepted before the self-test passes (`keygen_add`, `faraday/faraday-core/src/keygen/mod.rs:1934-1940`; `keygen_lock`, `2016-2042`; `keygen_codex32_lock`, `~2649`).
- In Practice (`demo_refuses`) SLIP-39 sources and the deal do nothing (`faraday/faraday-core/src/keygen/mod.rs:~1838`, `2016-2042`).

Lock in (`faraday/faraday-core/src/keygen/mod.rs:2016-2042`)
- Refused before the self-test passes; by `demo_refuses(KLock)`; on the Keys card while the stick is in the way; while `pass_eye.waits()` and a passphrase is asked.
- A SLIP-39 key, an already locked key, or an unfinished ENTER step returns without locking. SLIP-39 keys are added through the quiz's Add (`keygen_add`).
- Words for a Create slot need the passphrase equal to its repeat; a mismatch is refused silently (`~2056-2058`, `~2163-2165`).
- The Lock in button is enabled only when the passphrases agree and the eye is not waiting (`faraday/faraday-core/src/keygen/screen.rs:~810-812`).
- The passphrase pair is asked only when `asks_passphrase()`: never for codex32, never for a key made on its own (`faraday/faraday-core/src/keygen/mod.rs:~576-582`).
- Practice: `next_handout` swaps the made key for a sample of the same length unless the words are a sample's own (`faraday/faraday-core/src/keygen/mod.rs:~1967-1971`, `~2050-2055`).

Typed and scanned
- Typed words must parse, else the error is shown (`faraday/faraday-core/src/keygen/mod.rs:~1756-1763`); other forms need `parts.ready(form)` (`~1765`).
- `keygen_lock_typed` needs `may_load_keys()` and the typed step done (`faraday/faraday-core/src/keygen/mod.rs:~2145-2161`).
- A scanned SeedQR is taken only from Start Scan, before Lock in, and only English words; otherwise it returns false with no note (`faraday/faraday-core/src/keygen/mod.rs:~1719-1730`).

SLIP-39 shares
- `set_slip39` (`faraday/faraday-core/src/keygen/mod.rs:~660`), `deal` (`~717`), using `faraday_keys::words::slip39_split` (`~739`) and a 15-bit identifier (`~726`).
- Threshold 2..=n; total 2..=`MAX_COUNT`; the threshold is capped at the total (`faraday/faraday-core/src/keygen/area.rs:509-516`).
- `KNext` at ENTER with no randomness sets the `no_randomness` note and returns (`faraday/faraday-core/src/keygen/area.rs:~564-567`); a failed deal sets a note and ENTER stays undone (`~570-573`).
- Deal errors: `keygen_no_secret_yet` (`faraday/faraday-core/src/keygen/mod.rs:~719`); `keygen_the_shares_could` with `slip39_error` (`~748-753`).
- Strength is 128 bits when words <= `MIN_WORDS`, else 256 (`faraday/faraday-core/src/keygen/mod.rs:~764-773`).

Check card
- Dice show "high" chi-square only above `DICE_CHI_SQUARE_LIMIT` with at least `DICE_SKEW_MIN` rolls, and only for procedures other than direct selection (`faraday/faraday-core/src/keygen/screen.rs:~2188-2192`).
- A caution is not a refusal (`faraday/faraday-core/src/keygen/screen.rs:~1512-1513`).

Quiz
- Answers are ignored while `quiz_next` is set (`faraday/faraday-core/src/keygen/area.rs:~654`); a SLIP-39 pass moves to the next share or completes (`~654-681`).
- A BIP-39 key is already added at Lock in, so the last button only ends the flow: "Use this key" with a slot, else "Done" (`faraday/faraday-core/src/keygen/screen.rs:~2531-2537`).
- A skipped key gets `mark_made_here` (`faraday/faraday-core/src/keygen/mod.rs:~1959-1992`).

On a Create card
- `card_forget_words` resets the passphrase, `pass_open`, focus and `shown` (`faraday/faraday-core/src/keygen/on_card.rs:~10-33`).
- The first slot only, for a single key backed up as words, or a multisig with no key yet. Typing is on when the window is not compact (`faraday/faraday-core/src/keygen/mod.rs:~2949`).
- On Tools' New key card, typing is on when the layout is not one-column (`k.typing = !self.compact`, `faraday/faraday-core/src/keygen/mod.rs:~2945-2949`): the expanded rail (1200 dp or wider and not low), or in the Starter 1280 dp or wider (`faraday/faraday-core/src/drawing.rs:389-395`); the comment's 600 dp is stale. Under Reduce motion the rerolled face's drop is 0.0, so the face and its caption stay put until the next roll (`faraday/faraday-core/src/keygen/mod.rs:~1263-1270`, `faraday/faraday-core/src/keygen/dice_strip.rs:~258-285`). With coins by die, keys 1 to 6 are die flips (`faraday/faraday-core/src/keygen/mod.rs:~2822`).
- `Keys::next_handout` is `None` when no demo is set (stick and any non-Practice run), so a key made is added as made; with the demo it names the next sample of the same word count not loaded, which `keygen_add` and `keygen_lock` add in place of the key just made (`faraday/faraday-keys/src/session.rs:753-768`, `faraday/faraday-core/src/keygen/mod.rs:1966-1971`, `:2048-2054`); the callers are those two, `demo_refuses` for Lock in (`faraday/faraday-core/src/demo.rs:46-51`) and the demo lines (`faraday/faraday-core/src/samples/mod.rs:990`).

## Domain calls

- `core/faraday-entropy`: `DiceProcedure`, `DiceRolls`, `CoinFlips`, `CardDraws`, `RawHex`, `CameraNoise`, `Mixed`, `DeviceRandom`, `Strength`, `Warnings`, `RawEntropy`, `card_index`, `RANKS`, `SUITS`, `WORD_COUNTS`, `DICE_WORD_ROLLS`, `DICE_WORD_FACES`, `DICE_CHI_SQUARE_LIMIT`, `DICE_SKEW_MIN`, `MIN_MIX`, `MIX_SOURCES`, `SOURCE_ROWS`.
- `core/faraday-bip`: `bip39::{Language, Mnemonic}`, `slip39::{GroupSpec, random_len, Share, MIN_WORDS, MAX_COUNT, MAX_STRENGTH_BYTES, word}`, `codex32::Codex32::parse`, `keys::Fingerprint`.
- `faraday/faraday-keys`: `words::{slip39_split, from_entropy, from_indices}`, `codex32_seed`, `identity::{fingerprint_of_seed, fingerprint_of_words}`, `Keys::{next_handout, add_words, add_seed, add_words_with, add_codex32, account_text, position, is_public}`.
- `core/faraday-crypto`: `work::Compute`, `hex::encode_string`.
- `faraday/faraday-strings`: `bip::slip39_error`.
- `core/faraday-shell-api`: `Command::{CameraOff, RequestEntropy}`, `Key`, `EntropyBytes`.
- `faraday/faraday-ui`: `keyboard::OskPress`.

## Fixtures

None. Practice's codex32 sample (BIP 93 test vector 2) comes from the shell's samples data, not a fixture file. The guide page is `docs/guide/keygen.md` (`DICE_GUIDE`, `faraday/faraday-core/src/keygen/mod.rs:~19-21`).

## Fixes

| hash | what was broken | rule restored |
|---|---|---|
| 311d293ea | In the SLIP-39 quiz a right tap on a share's last word was dropped when the system's random bytes arrived late; the quiz could start in a fixed order when the generator's answer was late. | A right tap passes the share and the next share's quiz opens when the bytes arrive; the quiz order comes from the session generator. |
| c59eceeb5 | On the Pi's 480 x 640 panel the dice pad was pushed off the foot; none of its keys showed. | The dice pad is drawn at the top of the Rolls step under the progress line; the choice sits under the pad. |
| 24a4901f7 | Step 4 of New key: the "Never type a seed" line ran into what followed it. | Text does not run into the following line. |

## Doubts

- Row 298b5c5f9 left out: outside the inventory's scope.
- 24a4901f7 is a mixed commit; most of it added behaviour (sources on the card, Skip, the word-list search).
- No visible zero or overflow guard on the dice strip divisor `need` (`faraday/faraday-core/src/keygen/dice_strip.rs:~181`, `~187`), `k.words - 1` (`~138`), and `[11 - sums..]` (`faraday/faraday-core/src/keygen/words_so_far.rs:~334`, `~361`).
- The SLIP-39 quiz fix (311d293ea) also touched a cross-check tool; that part is not about this feature.
