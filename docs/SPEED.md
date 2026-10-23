# Speed in long flows

**Kind:** Spec
**Status:** Built; the glyph cache measured (§3.8). The web demo's part is in the website's own repository.
**Open:** the glyph cache's least-recently-used eviction (§3.8), not scheduled: measured, a frame drawn again never misses, and a dense panel clears once in about ten screen visits.

§2 and §7 are standing rules; §3.3, §8.3 and §9 to §11 say what holds now.

The owner's report: on a slow laptop Faraday became very slow in the
later stages of setting up a multisig wallet, the backups especially,
and the desktop app and the web demo slow down in the same flows. The
ask: Faraday always runs quick and smooth.

## 1. What is wrong (read from the code at 2e807ae; not yet measured)

Moved to `docs/history/SPEED-LOG.md`.

## 2. Rules for every part

1. **What is kept is public.** A value kept to save work is a
   fingerprint, a checksum, an account key, a file name, a count or a
   yes/no about public data. Nothing secret is kept, and no new copy of
   a secret is made, wiped or not.
2. **What is kept goes with what it describes.** A value lives in the
   struct it belongs to (a `Key`, a `Wallet`) or in the drawing memo
   (rule 3), so a lock, which drops the session, drops it. No static, no
   global, nothing that outlives the session.
3. **Decisions are never read from the drawing memo.** The memo of
   §3.3 exists only while `render` runs. Releasing a hold
   (`backup_settle`), marking an item done, gating a press
   (`backup_reachable` in `act`), and anything else outside `render`
   computes from the state as it is.
4. **A kept value cannot go stale.** It is computed where the thing it
   describes is made, and every way of changing that thing computes it
   again, through a setter; the field is private so nothing else can
   change it.
5. **The same answer as before.** Every part changes how often work is
   done, never what a person sees or what is written. F31 (no receive
   address before the backup is done) holds at every step; its tests in
   `tests/all/backup_first.rs` pass unchanged.

## 3. Design

### 3.1 The fingerprint is kept on the key (issue 120)

Built first in Faraday (a `fp` field on `wallet::Key`), then moved into
core (`docs/COMPUTE.md` §5.1). `MasterKey`
(`core/faraday-bip/src/keys.rs`) works out its fingerprint once, when the
key is made, from the blinded context; `for_network` copies it,
`fingerprint()` returns it, and `derive()` takes the master fingerprint
from it. Every constructor of a `MasterKey` takes the token and says it
costs the multiplication. Both apps save one elliptic-curve
multiplication per derivation; no output changes, and the BIP-32
vectors pass unchanged.

Faraday dropped its own field. `wallet::Key`
(`faraday/faraday-core/src/wallet.rs`) keeps `master` private:
`Key::master(&self) -> &MasterKey` reads it, `Key::fingerprint(&self)`
is a lookup through it, and `Key::set_network(&mut self, network)` is
the only way to change it (the fingerprint is the same on every
network). A `Key` is made through `Key::new`.

This also means fewer uses of the private key: each fingerprint call
had been a multiplication with the master private key (constant time
and blinded), thousands of times a second on the checklist.

### 3.2 The descriptor checksum is kept on the wallet (issue 121)

Built first in Faraday (a `checksum` field on `wallet::Wallet`), then
moved into core (`docs/COMPUTE.md` §5.2).
`WalletPolicy` (`core/faraday-bip/src/policy.rs`) sets its checksum in every
constructor, and `checksum(&self) -> &str` is a lookup. A policy has one
text: hardened steps are kept and written `h` whichever way they were
read (`docs/COMPUTE.md` §5.3, F66), so a wallet read with `'`, `h` or
both has one checksum.

A wallet is made through `Wallet::new(name, policy, source, origin)`.
`wallet::Wallet` keeps `policy` private, with `Wallet::policy(&self) ->
&WalletPolicy`; `Wallet::checksum(&self)` is a lookup through it. The
one place that changes a loaded wallet's policy, `silent.rs` (a silent
payment wallet's labels handed out), calls `Wallet::set_policy(policy)`,
which drops the addresses and scripts kept from the policy before.
`name`, `source` and `origin` stay public.

### 3.3 The drawing memo (issue 122)

`Faraday` gains a field `drawing: Drawing`, a struct in a new module
`faraday-core/src/drawing.rs`:

- `on: Cell<bool>`, true only while `render` runs;
- `items: OnceCell<Vec<u8>>`, what `backup_items` returns;
- `done: RefCell<Vec<(plan::Item, bool)>>`, what `backup_item_done`
  returned for each item asked;
- `shapes: RefCell<Vec<(usize, plan::Shape)>>`, `plan_shape` per wallet;
- `seed_lists: RefCell<Vec<(usize, Vec<(String, Option<usize>)>)>>`,
  `backup_seed_list` per wallet;
- `held: OnceCell<Vec<Fingerprint>>`, `held_keys`;
- `public_names: RefCell<Vec<(usize, Vec<String>)>>`,
  `backup_public_names_for` per wallet, for the backup's own answers.

`render` sets `on`, draws, then clears every cell and sets `on` back to
false, also when a draw pass returns early. Each of the six functions
above, while `on` is true, returns the memo's value when it has one and
otherwise computes, stores and returns it; while `on` is false it
computes as today and stores nothing. `Drawing::clear` empties every
cell; it is also called at the top of `render`, so a memo left by a
panic in a test cannot carry over.

Drawing changes no state these values depend on: the draw path writes
only scroll offsets (`backup_screen`'s `b.scroll`, the vault create
column's `c.scroll`), `dirty` and the command queue. Both draw passes of
a frame that opens a card share the memo.

On top of the memo, `backup_screen` asks for the waiting item once and
finds, for each card, whether it is reachable from the done marks of the
items before it in one pass over `items`, instead of calling
`backup_reachable` per card. `backup_reachable` stays for `act`.

**Parsed once, kept parsed (owner, 2026-10-20).** The memo keeps what a
frame puts together; what it is put together from is kept on the state,
parsed or derived once, with what it describes (§2 rules 2 and 4):

- each open vault's records keep the wallet descriptions they hold read
  (policy, checksum, shape) and each key record's word count
  (`VaultRecords`): read at the first ask after a change, and before
  every frame; any change to the records goes through `DerefMut`, which
  drops what was read;
- each Inbox and Outbox file keeps the wallet it describes, read when it
  came in, and a vault's hash (`Item::wallet`, `Item::sha`):
  an Item's bytes never change;
- each plan's map, by place and with its check (`plan::Mapped`), is kept
  by the shape and answers it maps (`Kept::plan_mapped`), and the
  plan's helpers read the map they are given (`alone_on`, `reach_on`,
  `checklist_on` and the rest) instead of mapping again;
- a wallet keeps its addresses, a key its account key texts, Explore
  its last reading, the power sheet's guide address its code, each by
  what it is worked out from: a change to any of those is another key,
  so none is stale;
- a key, and each open vault's key records, keep the SHA-256 of their
  words' entropy (`wallet::WordsId`, all 32 bytes, the same from any
  list): what "this vault holds this seed" compares, so two seeds never
  pass for one (F65). A hash of secret entropy, it lives only as long
  as the key or the records, is wiped when dropped and is never
  written.

A kept value is worked out by whoever asks first, a frame included;
after that every frame and every press only looks it up.

**The token** (`docs/COMPUTE.md`, F69). Every costly function, in core
and in Faraday's crates, takes `c: &Compute` (`faraday_crypto::work`) and
counts its work through it; its doc's first line says "Costs …". The
token is not `Clone`, `Copy`, `Default`, `Send` or `Sync`, and is
minted only at entry points: event code holding `&mut Faraday`
(`Faraday::compute`), the drawing view, worker threads and processes
that draw nothing, tools and tests (COMPUTE §3). Drawing gets a
read-only `View` (`faraday-core/src/drawing.rs`), which derefs to
`&Faraday` and cannot mint, so a screen that calls costly work does not
compile. `View` reaches costly work only through its named accessors in
`drawing/`: the memo's fills (`memo.rs`), values kept on the state
worked out the first time (`kept.rs`), and the owner's exceptions
(`allowed.rs`). A decision (a press gated, a hold released) uses a
`*_now(&self, c: &Compute)` form on `Faraday`, never the memo, which
exists only on `View`. What the types cannot hold, the costs lint
(`tools/costs-lint`) and `faraday/clippy.toml` hold, both in `just
lint` (§8.3). The debug guard that panicked on costly work while a
frame was drawn was removed once the token held everything it checked
(COMPUTE §10); the counters (`work::count`, `Counts`, `snapshot`,
`since`) and the budget tests in `work/budget/` stay, and count what
each frame, press and key does.

**The owner's list.** `Allowed` (`drawing/allowed.rs`) names each
exception and why, one method on `View` per entry; a new exception is a
new method, which the owner decides. One entry now, `SecretShown`: a
secret the person opened to see or copy (a seed's SeedQR grid on the
copy page, a vault key record's words, a BIP-85 child), drawn from its
source each frame because nothing secret is kept, not even for a frame
(§2 rule 1).

### 3.4 Public file names without making the files (issue 123)

A new function `picture_names(name: &str, part: &str, text: &str) ->
Vec<String>` in `lib.rs`, beside `picture_codes`, gives the names of the
pictures `picture_codes(text)` would make, without making them: one name
when `text.len() <= faraday_codec::qr::byte_capacity(25, Ecc::Medium)` (the
bound `encode_bounded` uses for a byte payload), else one per part of
`faraday_qr::bbqr::encode('U', text.as_bytes(), QR_PARTS[1])`, numbered
as today (`{name}{part}{j}-of-{k}.png`), and none when that encoder
refuses the text, as `picture_codes` then gives no codes. `public_pictures` names its pictures
through `picture_names`, so the names written and the names checked come
from one function. `backup_public_names_for` calls `picture_names` with
the descriptor text in place of `public_pictures(wi, 8)`.

`account_key_names` needs only each key's fingerprint and whether the
wallet's key in that slot is the one Create makes for its kind
(`standard`). It keeps calling `wallet_key`; with §3.1 the search is
cheap, and the derivation behind `standard` runs once per frame through
the memo of §3.3.

### 3.5 The hold is checked when something changed (issue 124)

- `App::event` calls `backup_settle` after `Touch` with phase `Down` or
  `Up`, `Key`, `Back`, `Forward`, `Scanned`, `CameraUnavailable` and
  `Display`, and not after `Touch` moves, `Scroll`, `Wheel`,
  `ScrollEnd`, `Hover`, `HoverEnd`, `CameraFrame` or `Tick`. `press`
  and `storage` keep their calls. Calling it less often can only keep a
  hold longer, never end one early; no event that can complete an item
  is left out.
- `held_keys` computes `flow_held` once and passes it to a private
  `held_keys_stored_with(flow: &[Fingerprint])`; `held_keys_stored`
  calls that with its own `flow_held`. The sets returned are the same as
  today.
- `receive_held` computes `held_keys` once, finds the wallet by
  `Wallet::checksum`, and calls `wallet_held_in` with what it computed.

### 3.6 The desktop app re-reads sticks on its timer and after writes (issue 125)

`Shell::send` calls `rescan` only when this pass toggled a stick or
served a storage command. `serve_all` (`faraday-storage/src/lib.rs`)
already loops over `app.poll_storage()`; `Faraday` gains
`pub fn storage_pending(&self) -> bool` (`!storage_out.is_empty()`),
which `send` reads before calling `serve`. The 700 ms `RESCAN` timer
and the forced rescans at start and after a restart stay. A stick
plugged in or pulled outside the app (its folder made or removed by
hand) is seen within 700 ms, as today when nothing else happens.

### 3.7 Words compared without parsing (issue 126)

`wallet::Key` gains `fn words_are(&self, indices: &[u16]) -> bool`: it
walks the key's words (stored as `lang.word(i)` joined by single spaces,
as `add_words` writes them) beside `indices`, comparing each word with
`self.language.word(index)`, and returns false at the first difference or
a different count. No `Mnemonic`, no new string, no copy of the words.
`same_words` in `held.rs` becomes
`records::words_of(r).is_some_and(|m| key.words_are(m.indices()))`.

The comparison stops at the first different word, as `==` on the
indices does today; it runs inside the process on secrets the process
already holds, so its timing tells nothing new.

`Language::index_of` is not changed (with this
change nothing hot calls it).

### 3.8 The glyph cache: measure first (issue 127)

Measured with a harness outside the repository (§4): at scale 2 on the
desktop window's default size, the number of times `GlyphCache::clear`
runs per frame on the checklist, the Wallets page and the create flow.
The harness counts with a temporary patch; nothing is committed for the
measurement.

If a screen clears the cache on every frame: the cache evicts the least
recently used glyphs until the new one fits, in place of clearing
everything, and `CACHE_BUDGET` becomes 1 MB. That is a change to
`core/faraday-draw`. If no screen clears it every frame,
nothing changes and the issue is closed with the numbers.

Measured 2026-10-16 at 2560×1600 scale 2: no screen (the checklist at
every stage, the Wallets page, Create's Keys card) cleared the cache, in
a frame, a scroll or the whole session; the most it held was 171,800 of
its 262,144 bytes.

Measured (2026-10-23), the heavy-session walk of §7 over 23 screens in
the release profile: a frame drawn again misses no glyph on any screen
at any size. The cache cleared once in the whole walk at 3200 × 1800
(scale 2, Wallets after Home, at 261,294 bytes; refilled to 97 KB in
that frame), never at 1366 × 768, 2560 × 1600 (scale 2) or 1920 × 1080
at 400 dpi (scale 2.5). Redraws at scale 2 cost the pixel count, not the
cache: Home 1.7, 4.7 and 6.4 ms at scale 1, 2560 × 1600 and 3200 × 1800;
the backup board 4.0, 11.1 and 15.1 ms. Evicting the least-recently-used
glyphs until a new one fits (`GlyphCache::glyph`,
`core/faraday-draw/src/fonts/mod.rs`) instead of `clear()` would remove
that one slow frame; not scheduled.

### 3.9 The web demo (issue 128, built in the website's own repository)

In the website's `web/index.html`, `pointermove` (both
`faraday_move` and `faraday_hover`) and `pointerleave` go through a
`call` that does not force `watch`: the state is read at most every
150 ms, as `watch` already does when not forced, and at once after any
other call. `draw` still runs whenever `faraday_dirty` says. The
website's own handoff file takes the item; the website is rebuilt and
published by its own flow.

### 3.10 `--timings` shows events and storage (issue 129)

The desktop's `FrameTimings` line gains `events {mean} ms (worst
{worst})` and `storage {mean} ms (worst {worst})`: the time in
`app.event` plus its commands, and the time in `serve` and `rescan`,
summed per second as render and blit are. It prints durations only:
never which event, which key or which file.

### 3.11 Test builds say when a frame is slow (issue 130)

In a build that is not a release (`faraday_core::is_release()` false:
test releases, dev builds and a plain `cargo build`), the desktop shell
prints one line on stderr when drawing a frame, or handling one event
with its commands and storage, takes more than 50 ms:
`faraday: slow frame 182 ms · Backup` or `faraday: slow event 64 ms ·
Backup / Qr`, with the screen and the sheet as `Screen` and `Sheet` name
themselves (`Faraday::screen_name`, a new `pub fn` returning
`format!("{:?}", screen)` and the sheet's when one is up). At most one
line a second; nothing else, never what the screen shows, what was
pressed or a file's name. A release prints nothing.

It is seen when the app is started from a terminal. The stick prints
no such line; its dev image records what its loop costs in
`faraday-timings.txt`, which the shell keeps in RAM and writes onto the
boot stick through the disk process (`faraday/image/README.md`).

### 3.12 The vault 2 card and the sticks card; the late flow again (2026-10-19)

The two leftovers §6 names after B, and a measurement of the late flow
with what was added since (the board's files box, the access layers,
Create's Start and Wallet cards, the flows-in-progress rows).

- **The vault 2 card.** Whether an open vault holds the wallet
  (`vault_has_wallet`, through `backup_vault_fits` and
  `backup_open_fits`) parses every wallet record's descriptor on every
  frame. It is worked out where the vault is opened or its records
  change and kept with the open vault (a fingerprint set of the wallets
  it holds, public), or through the drawing memo; a decision computes
  it from the state as it is (§2 rule 3).
- **The sticks card.** `sticks_item` (`kept_wiped`, `key_text`) and
  `wallet_stick_rows` work out each stick's files on every frame. They
  are worked out once per frame through the drawing memo, or kept where
  the stick's files are written, whichever keeps §2's rules.
- **The late flow, measured** with §4's harness after both: the
  checklist at every stage, a scroll, a pointer move, the board in
  every layer view, Create's cards, at 1366×768 and 2560×1600 scale 2;
  the results in §6. Any frame over 16 ms is named with what it spends
  its time on, for a later batch; nothing else is changed in this one.

## 4. Measuring

The numbers come from a harness outside the repository, as MOTION.md
§2 did: it drives `Faraday` through `App::event` and `App::frame` on the
build machine, release profile, at 1366×768 scale 1 and 2560×1600 scale
2, and times:

- one frame of each checklist stage of a 2-of-3 wallet made here with
  three new keys and a vault open, from the first item to the last;
- one scroll frame on the same checklist;
- the event handling of a pointer move over it.

The plan of measuring before and after each batch: moved to `docs/history/SPEED-LOG.md`.

## 5. Batches

Moved to `docs/history/SPEED-LOG.md`.

## 6. Results

Moved to `docs/history/SPEED-LOG.md`.

## 7. Standing rule: a late-flow frame is timed (owner, 2026-10-15)

After any batch that adds or changes what a screen works out while it
draws, or what runs after every event, the architect times it with the
harness of §4, before the batch is committed:

- one frame, and one scroll frame, of each screen the batch touched,
  in the heaviest session the flow reaches: a 2-of-3 wallet whose three
  keys were made here, its backup flow under way, a vault open;
- the event handling of a pointer move over the same screen.

The numbers go in the batch's report beside the ones from before it. A
frame that costs more than before, by more than the run-to-run spread,
is explained in the report or fixed in the batch. The harness stays
outside the repository; this rule is how it is used.

The counts are enforced in `just` (owner, 2026-10-20): each costly
function counts itself by kind (`faraday-core/src/work.rs`), and the
budget test there (`work/budget.rs`) draws every main screen and the heavy
sheets of the session above twice, and moves the pointer over each; a
frame that works out more than its screen's named budget, or a pointer
move that works out anything, fails the build. Since what a frame reads
is kept on the state (§3.3), every budget is none: a frame drawn again
works out nothing, and a line in the table names any screen that cannot
keep what it draws, with why. The token of §3.3 holds the same rule at
compile time: drawing cannot call costly work but through `View`'s
accessors. The timing above stays for milliseconds. Since batch S1 (§8)
the same budget tests also press what each main screen offers, compare
frames as the Inbox and the wallets grow, let every flow page rest at
seven scales, and fix the work of the key moments; since COMPUTE D1
(§8.3) they also hold each kept accessor, a frame's allocations and
lines of text, each typed character, the Pi's panel and a long
session's kept lists.

## 8. The speed audit and batch S1 (2026-10-21)

Every path a person waits on is held by counts in `just`, no wall
time: frames, every press, screens as the Inbox and the wallets grow,
pages at rest, and the key moments (§8.3), in the kinds of §8.2.

The speed audit and what S1 fixed: moved to `docs/history/SPEED-LOG.md`.

### 8.1 What S1 fixed

Moved to `docs/history/SPEED-LOG.md`.

### 8.2 New counted kinds

Counted at their source, the core's through `faraday_crypto::work` (F65). S1 counted them behind a `count` feature and a
process-wide counter the app installed; since COMPUTE C3 the count
goes through the token the caller holds (`c.count(kind)`), the feature
and the counter are gone, and Faraday's tokens carry
`work::core_count`, which maps each core kind to Faraday's own. S1's
kinds:

| Kind | Where |
|---|---|
| `PsbtParse` | `faraday_psbt::Psbt::parse_bytes`, past the magic check (base64 too) |
| `PsbtInspect` | `faraday_psbt::inspect`, also the one inside every signing |
| `Sign` | `faraday_psbt::sign`, `faraday_psbt::message::sign` |
| `WalletMatch` | `Wallet::script_at`, a script derived to match a PSBT |
| `PngDecode` | `faraday_files::qr_in_png` |
| `QrDecode` | `faraday_codec::decode::decode_luma` |
| `Kdf` | `faraday_crypto`'s PBKDF2 and scrypt, `faraday_backup::argon2id`, `faraday-vault`'s Argon2id |
| `Blur` | `faraday_draw::Canvas::frost` |
| `FullCanvas` | `Canvas::snapshot`, `snapshot_into`, `restore`, `blend_between` |

The kinds Faraday had counted itself before COMPUTE C1 to C3 are now
counted at their source in core, each by a function that takes the
token (COMPUTE §2.3):

| Kind | Where |
|---|---|
| `WordsParse` | `faraday_bip::bip39::Mnemonic::parse` |
| `MasterKey` | `MasterKey::from_seed`, `from_seed_bytes`, `from_xpriv`, `decode`, `from_chain_code_and_key`, each with the fingerprint it keeps (§3.1) |
| `KeyDerive` | `MasterKey::xpub`, `derive`, `account_xpub`, `multisig_account_xpub`, `with_multisig_account_secret`; `DerivedKey::to_xpub` |
| `DescriptorParse` | `WalletPolicy::parse`, `from_parts`, `from_descriptor`, `parse_any`, `from_record`, and the import readers through them |
| `Address` | `WalletPolicy::script_at`, `script_at_with`, `address_at`, `find_address` |
| `QrEncode` | `faraday_codec::qr::encode`, `encode_bounded`; `seedqr::encode_seedqr`, `encode_compact` |
| `FileHash` | `faraday_crypto::sha256_whole`, which Faraday's whole-file hashes use |

Faraday's own kinds (`PlanMap`, `Picture`, `WalletMatch`, `PngDecode`,
and `Kdf` in `faraday-vault`) are counted with `work::count` by
functions that take the token as the gate.

Every kind but the two pixel kinds, `Blur` and `FullCanvas`, needs the
token, so drawing reaches it only through `View`'s accessors (§3.3).
The pixel kinds are drawing's own work and take no token: a canvas
counts them through the counter its owner sets
(`Canvas::set_counter`), and the budgets count them per frame. A
sheet's frame puts its kept backdrop back, one `FullCanvas`; a frame
drawn again does nothing else.

### 8.3 The tests in `just`

All counts, no wall time, in `faraday-core/src/work/budget/`:

- **Frames** (`budget.rs`): the heavy session's screens of §7, now with
  the power, Learn and scan sheets; every budget none but a sheet's one
  `FullCanvas`.
- **Every press** (`presses.rs`): on each main screen and the open
  spend, every action the frame offers is pressed in turn (about 170
  presses), each counted with the frame, a tick and the frame after;
  left out by name are the presses that end the session or drop what
  the next needs (lock, power, wipe, network, demo, removals, a shorter
  idle lock, holds). Budgets, all else none:

  | Press | Kind | Budget | Why |
  |---|---|---|---|
  | any | `FullCanvas` | 3 (5 to a sheet) | the screen fades from a copy of the frame before, blended into the next two; a sheet also keeps its frosted page and puts it back |
  | any | `Blur` | 1 | a sheet frosts the page under it once |
  | `PickWallet` | `Address` | 1 | the card's first receive address, derived once, kept |
  | `PickWallet` | `WalletMatch` | 11 | the card's button asks which wallet each PSBT spends from; each wallet derives its script at a place once, kept |
  | `Step` | `Address` | 2 | a spend's step shows the wallet's own addresses its outputs pay, each derived once, kept |
  | `Vault(Category)` | `WordsParse` | 1 | the seeds category shows a key record's words (`Allowed::SecretShown`) |
  | `Backup` | `PlanMap` | 2 | the wallet's plan mapped and checked once for its shape and answers, kept (eight kept) |
  | `Resume(Flow)` | `FileHash` | 2 | the GPG flow resumed at its end finishes: its vault sealed and hashed, then hashed as it comes into the Outbox |

- **Scaling** (`scaling.rs`): Home, Wallets, a wallet's card, Files,
  the Spend tab and Spend, with no spend and with Savings' open, do the
  same work a frame (none) with one and five copies of the test stick's
  files and one and all ten test wallets; the boot import sheet does
  the same with one and five copies (its kept backdrop only).
- **Idle** (`idle.rs`): every flow page of the walk and the main
  screens that scroll, at scales 1, 1.25, 1.5, 1.75, 2, 2.5 and 3, at
  their top and at their end: 120 ticks ask for no frame.
- **Key moments** (`moments.rs`), each exactly its table:

  | Moment | Work |
  |---|---|
  | start to Home with the test stick's 82 files (the self-test apart) | `PsbtParse` 21, one per PSBT; `DescriptorParse` 41, the classifier trying each other text file as a description; `FileHash` 2, the vault and the EFI image |
  | Unlock to the Load sheet | `Kdf` 5 (Argon2id 1; PBKDF2 for the vault's seed and the three seed files Files shows under the sheet); `MasterKey` 4; `WordsParse` 5; `FileHash` 2; `DescriptorParse` 13 (12 records once, the kept wallet text once); `Blur` 1; `FullCanvas` 5 |
  | Load ticked | `WordsParse` 2, `Kdf` 1, `MasterKey` 1, `FileHash` 1 (the seed loaded); `Address` 1 and `WalletMatch` 12 (Wallets' cards, kept); `FullCanvas` 5; no description parsed |
  | a PSBT opened to its review | `PsbtParse` 1, `PsbtInspect` 1, `WalletMatch` 1, `FullCanvas` 5 |
  | Sign here (Savings, both seeds) | `Sign` 1; `PsbtInspect` 2 (the signing's own, the signed PSBT's); `PsbtParse` 1 and `DescriptorParse` 1 (the two results classified as they come into the Outbox); `FullCanvas` 4 |

- **Kept values follow** (`work/kept.rs`): a PSBT waiting in Files is
  offered on its wallet's card once that wallet loads; a spend opened
  before its seeds were loaded signs with both once they are, and its
  review shows both signatures.

COMPUTE D1 (2026-10-21) added:

- **Each kept accessor** (`accessors.rs`): every value kept on the
  state that drawing reads (`drawing/kept.rs`), read on two frames in a
  row with nothing changed, does no counted work the second time, each
  accessor on its own.
- **Allocations and lines a frame** (`budget.rs`): each screen of the
  walk allocates and draws no more lines of text than its measured
  count, in two tables, the desktop's (`FRAME_COSTS`) and the Pi's
  compact screens (`FRAME_COSTS_PI`), with a margin of 10 % or at least
  8 (`MARGIN_PCT`, `MARGIN_MIN`). A change that draws more on purpose
  measures again and writes the new counts there, with why. The test
  binary counts allocations with `faraday/tools/alloc-count`.
- **Each typed character** (`keys.rs`): a character typed into any
  field does no costly work beyond its named budget, the key and the
  frames after it together; a key stretch is never in a budget (§9
  item 2).
- **The Pi's panel**: the frame budgets, every press and the pages at
  rest also run at the Pi's 480×640, whose compact screens are other
  code.
- **A long session** (`caches.rs`): a session that loads and drops
  every test wallet, brings files in and out, changes a plan, locks and
  unlocks the vault and walks far through addresses, account keys,
  typed descriptions and QR codes keeps every kept list within its
  bound, a named constant beside the list (`PUBLIC_QRS_KEPT`,
  `TEXTS_READ_KEPT`, `WALLET_ADDRESSES_KEPT` and the rest).

The lints in `just lint` hold what the types do not (COMPUTE §7 to §9):
`tools/costs-lint` (a function taking the token says "Costs …" first;
counted work takes a token; a mint only at its listed sites; no kept
token but `View`'s; no mint while drawing), `faraday/clippy.toml` run
through `tools/clippy.py`, and the `compile_fail` doctests on
`Compute` and `View`.

## 9. Batch S2: nothing a person waits on blocks a frame (2026-10-21)

Nothing a person waits on blocks a frame. What holds:

1. **Pictures are read off the shell's thread**, one at a time in the
   order asked; the answer comes as a storage event on a later pass
   (`faraday_storage::Sticks::read_qr_later`, `pictures_read`). On a
   desktop `Dirs` keeps a thread that reads the file and its codes
   (`Pictures`); on the device the disk process decodes on a thread of
   its own, one picture asked of it at a time (`DiskProcess`,
   `faraday-disk`'s `main`). An answer for a shell that has gone (a
   lock) is dropped. The boot stick's sheet and the visit's row say
   "Being read" until it comes (`VisitState::reading`). A picture is
   decoded once and searched for light-on-dark codes only when it holds
   no dark-on-light one (`faraday_codec::decode::decode_picture`).
2. **A press that stretches a key or makes pictures is drawn first.** A
   press whose work stretches a key (PBKDF2, scrypt, Argon2) or makes
   more than one picture or PDF is drawn, then done on the tick after
   (`slow`, `Faraday::press_in`, `Later`), with a toast for files or a
   node key that goes when the work is done. A slider that changes the
   shares (`BOmit`) sets its value at once and makes the files again
   once it is let go and a frame drawn. A scripted press
   (`Faraday::press`, the snapshot tool, the tests) does the work at
   once. In a debug build a press not on the list that stretches a key
   or makes more than one picture panics, naming itself
   (`tests/all/slow_press.rs`). Enter on Add a key's, New key's and a
   chart's fields does what their buttons do at once. **Typing waits for
   a pause:** a field that works out a key from what is typed (the
   passphrase on `Sheet::Potential` and `Sheet::SeedRead`, Load a key's
   probe of the words) works it out once typing has paused for
   `motion::TYPED_PAUSE_MS` (400 ms); tests wait with `testkit::pause`.
   The key budget test (§8.3) holds every field to it.
3. **Rows out of sight are not drawn.** A line of text wholly outside
   the clip is measured from its advances, not drawn
   (`Canvas::text_with_face`); Files, the visit's lists and a small
   panel's file cards pass over a row out of sight with its height
   worked out without drawing (`Ui::shows`, `parts::wrap_buttons_height`,
   `file_card`, which checks the two heights agree in a debug build).
   The scaling test (§8.3) holds every screen to the same lines of text
   drawn with one and five copies of the files (`Canvas::lines_drawn`).
4. **`Ui::fit` adds the characters' advances once** and stops at the
   first that does not fit with "…".
5. **Pixels.** The frosted page under a sheet reads a quarter of the
   pixels where a small pixel is sixteen or more across
   (`Canvas::frost`); a sheet's later frames put back only the box drawn
   over (`Canvas::restore`); a fade passes over runs of 16 pixels where
   the two frames agree (`Canvas::blend_between`); a shadow skips its
   covered middle (`Canvas::shadow`). The backup board's overdraw is its
   look and stays.

The batch's account, with its measurements: moved to
`docs/history/SPEED-LOG.md`.

## 10. Batch S3: what the Inbox gives is kept with the Inbox (2026-10-21)

What S2 left: moved to `docs/history/SPEED-LOG.md`.

`Faraday::inbox` is now `InboxFiles`, which reads as the files
(`Vec<Item>`) and keeps that result (`Found`) beside them. It is worked
out again only when an input changed, and a frame compares the session's
keys and wallets, not the files. Inputs of `found_in`, each how the kept
copy is told:

1. The files (added, removed, renamed, replaced, marked): every change
   goes through `DerefMut`, which drops the kept copy. An `Item`'s bytes,
   kind, wallet and sha never change after it is made.
2. The session's keys: fingerprint, label and share of each, compared on
   each ask.
3. The session's wallets: name and policy of each, compared on each ask.
4. Nothing else: the seeds' fingerprints and the texts read are
   functions of the files, kept by what they come from.

The invariant is written on `InboxFiles`. Decisions are not read from
it: a press that loads, removes or opens a file calls `found_in` or
`inbox_found` as before, and `inbox_found` answers from the state as it
is. Tests: `work/kept.rs`, `what_the_inbox_gives_follows_the_files_and_the_session`
changes each input in turn (file added, renamed, replaced, removed;
wallet loaded, renamed, policy changed, gone; seed file added; key
loaded, relabelled, gone) and checks the kept result is the fresh one and
that asking again works nothing out; the scaling test (§8.3) also counts
the times a frame worked the Inbox's result out (`InboxFiles::rebuilds`):
none, with 1x and 5x the files.

The measurements: moved to `docs/history/SPEED-LOG.md`.

## 11. Batch E: the stick's panel and the Starter (2026-10-21)

The counts of §8 covered faraday-core. Batch E (owner; not §5's E,
the web demo) takes them to the two places they did not reach: the stick shell's path from the core's
frame to the framebuffer, and the Starter.

### 11.1 E1: the stick shell writes only what changed

What was found: moved to `docs/history/SPEED-LOG.md`.

**Fixed.** `present.rs`'s `Presenter` keeps the core's last frame beside
its converted copy and compares first (`fb::changed_rows`, a `memcmp` a
row): only the rows from the first to the last that differ are
converted, and only those are walked to the panel. A frame that is the
same converts and writes nothing; a stale panel (the first frame, after
a blanking or a failed write) still takes the whole frame. The cost is
a copy of the core's frame (1.2 MB at 480×640, 8.3 MB at 1920×1080).

The measurements: moved to `docs/history/SPEED-LOG.md`.

**Tested.** `Panel` writes to a `Sink` (the framebuffer `File`, or a
`Vec<u8>` in a test) and counts the rows it writes; the `Presenter`
counts the rows it converts. `present::tests::the_panel_is_written_only_where_the_frame_changed`
drives a real `Faraday` on the Pi's 480×640: the first frame converts
and writes 640 rows; 120 ticks at rest, none; the same frame drawn again
twice, none; a changed frame and the frames after it, at most 640 rows
converted and 640 written each, and the panel then holds the frame; a
blanked panel takes the next frame whole without converting it again.

### 11.2 E2: the Starter

What was found: moved to `docs/history/SPEED-LOG.md`.

Nothing else on a frame costs more than a lookup: drawing takes
`&Starter`, the phone's QR codes are made when the computer is reported
(`Starter::phone_codes_make`), the Mac table is parsed once
(`macs::macs`), the PDFs are made on a press with the held app's token,
and the costs lint's rule 4 keeps `.compute()` out of the drawing
functions. The disks, the concerns, BitLocker, the installed copy and
every write already run on threads of their own. The developer's
`--image` is hashed once at start, before the window opens
(`payload::Image::hash`); it stays there, since a file that cannot be
read ends the program with its reason before a window is shown.

**Fixed.**

1. The frosted page is kept on the Starter (`Starter::frost`) and given
   to each frame's `Ui`; a frame under a sheet that has it puts it back
   and draws the sheet alone (`draw::frame`). It is dropped by every
   change that can alter the page under the sheet (a display, a report
   from the shell, an action, a scroll) and by a frame with no sheet; a
   hover or a press cannot, since under a sheet only the sheet's own
   buttons, none of which is on a page, can be pressed. The invariant is
   written on the field. `drawn_texts` drops it, so a test still reads
   the page's words under a sheet.
2. The shell reads the Downloads folder and the cameras on a thread of
   its own (`watch_listing`), which sends the listing when it changed;
   the window's thread takes the newest and tells the held app as
   before.
3. A wheel turn that does not move the page, and a pointer move that
   keeps the press, ask for no frame.
4. The Starter's canvas counts its pixel work with faraday-core's
   counter (`work::core_count`, now public).

**Tested.** `faraday-starter/tests/all/budget.rs`, on Home, the restart
sheet, Start and its restart sheet, Which stick, About, Blank templates,
Install, the uninstall sheet, the Continue anyway sheet, Transfer and
Learn: a frame drawn again does no counted work but one `FullCanvas` a
frame (a sheet's kept page put back) and shows the same pixels; the
pointer moved over the whole window, the same; 120 ticks at rest ask for
no frame and do no work. With the page not kept the first two fail on
`Blur`. The kept page follows its source: a receiver plugged in under
the open restart sheet shows on Home under it, the same pixels as the
sheet opened after (`the_page_under_a_sheet_shows_what_changed_under_it`). The 390 renders of `cargo run -p faraday-starter --example snap`
at 1280×800, 480×640@286, 800×600@240 and 1100×760@160 are byte for byte
the same before and after (sha256 of each).

**Next.** The Starter's costly work already takes the held app's token
(`Starter::held().compute()`, minted at its events), and its drawing is
read-only through `&Starter`, so no `View` of its own is needed yet. If
the Starter grows a value worked out from its state (an offer list, a
checklist) that becomes costly, it is kept on the state as §3.3 says,
and the budget test names it. The desktop app's shell reads the
Downloads folder and the cameras on a thread of its own (`watch_listing`
in `faraday/shells/desktop/src/main.rs`), as the Starter does; the window's
thread takes the newest listing, a lookup. Done.
