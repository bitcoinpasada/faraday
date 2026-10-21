# Speed in long flows

**Status: 2026-10-16, batches A and B built and checked by the owner on
the slow laptop's stick; C next, after BACKUP-FLOW §19's F batches; D
measured at the stick's high-density sizes before it is decided (§6); E
in faraday-website.
§7 is a standing rule (owner). Tracker issue 119 and its sub-issues
120–130.**

The owner's report: on a slow laptop Faraday became very slow in the
later stages of setting up a multisig wallet, the backups especially,
and the desktop app and the web demo slow down in the same flows. The
ask: Faraday always runs quick and smooth.

## 1. What is wrong (read from the code at 2e807ae; not yet measured)

Faraday draws in immediate mode: `Faraday::render` rebuilds the whole
screen from the app's state on every frame, and a frame is drawn on
every step of a scroll or glide, every change of what is under the
pointer, every caret blink, every step of a fade or a toast, and twice
on the frame a step card opens. Whatever a screen works out while it
draws is worked out that often. The backup screens work out a great deal:

1. **A key's fingerprint is computed on every call.**
   `MasterKey::fingerprint` (`core/osk-bip/src/keys.rs`) derives the
   public key from the private key (an elliptic-curve multiplication)
   and hashes it, each time. About 100 call sites in faraday-core ask
   for it, many inside a search over every session key for each slot of
   each wallet (`backup_keys_of`, `backup_seed_list_of`, `wallet_key`,
   `Session::key_label`, `flow_held`, the sidebar's key rows).
2. **A wallet's descriptor checksum is computed on every call.**
   `WalletPolicy::checksum` writes out the whole descriptor and checksums
   it. `plan_shape`, `receive_held`, `wallet_flow_of_backup` and the
   sidebar call it, several of them once per loaded wallet.
3. **The checklist asks the same questions many times per frame.**
   `backup_screen` (`screens.rs`) calls `item_title`, `backup_reachable`
   and `backup_item_done` for each card, then `backup_waiting` and the
   all-done check; `backup_reachable(k)` re-asks `backup_item_done` for
   every item before `k`; `backup_panel` and `plan_cards` ask again for
   the map's lines. Each `backup_item_done` rebuilds the seed list and
   the plan's shape. The work grows with the number of items done, which
   is why the later stages are the slowest.
4. **The public files item's check makes a QR code.**
   `backup_public_names_for` calls `public_pictures(wi, 8)`, which
   encodes the whole descriptor as one or more QR codes, only to read
   back the first file's name; `account_key_names` calls `wallet_key`
   per slot, which derives each account key from the master.
5. **The hold on keys made here is re-checked after nearly every
   event.** `backup_settle` runs at the end of `App::event` for touch
   moves, scrolls, wheel notches, hovers and camera frames as well as
   presses; at the checklist stage it asks `backup_item_done` for every
   item. `held_keys` computes `flow_held` twice (once itself, once in
   `held_keys_stored`), and `receive_held` computes `held_keys` twice.
6. **The desktop app re-reads the stick folders after every event.**
   `Shell::send` (`faraday/shells/desktop/src/main.rs`) ends with
   `rescan`, which lists and reads every attached stick folder, on every
   pointer move, idle tick and animation tick, beside the 700 ms
   `RESCAN` timer. The backup writes more files there as it goes.
7. **Seed words are parsed again to compare them.** `same_words`
   (`held.rs`) parses a session key's words into a `Mnemonic` for every
   vault key record it compares with; `Language::index_of` searches the
   2048-word list in order for each word.
8. **Possibly: the glyph cache is cleared every frame at scale 2.**
   `core/osk-ui/src/fonts/mod.rs` drops every rasterised glyph when the
   cache passes `CACHE_BUDGET` (256 KB). One dense screen's glyphs at
   twice the size may not fit, and then every glyph is rasterised again
   on every frame. Not confirmed.
9. **The web demo reads its whole state on every pointer move.**
   faraday-website's `web/index.html` sends each `pointermove` through
   `call`, which forces `watch`, which calls `faraday_state`; `Web::state`
   (`web/src/lib.rs`) runs the checklist (`story.rs` `checklist_next`)
   and serialises it to JSON, whether or not anything changed.
10. **`--timings` on the desktop times drawing only**
    (`faraday/shells/window/src/pace.rs` `FrameTimings`), not the events
    or the serving of storage, so items 5 and 6 do not show in it.

Estimate for one frame late in the checklist of a 2-of-3 wallet whose
three keys were made here, with a vault open: some thousands of
elliptic-curve multiplications, about ten QR encodes and about a hundred
descriptor checksums. Nothing grows without bound; the same work is
repeated, and there is more of it as the session holds more keys,
wallets, flows and open vault records.

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
   `tests/backup_first.rs` pass unchanged.

## 3. Design

### 3.1 The fingerprint is kept on the key (issue 120)

`wallet::Key` (`faraday/faraday-core/src/wallet.rs`) gains a private
field `fp: Fingerprint`, set where the key is made: `Session::add_mnemonic`
(which `add_words` calls) and `Session::add_seed` already compute it before pushing the key, and
pass it in. `Key::fingerprint(&self) -> Fingerprint` returns it. Every
`k.master.fingerprint()` on a session key in faraday-core becomes
`k.fingerprint()`.

The fingerprint does not depend on the network, so `Session::set_network`,
which replaces each key's `master` with `master.for_network(network)`,
leaves `fp` as it is. That line is the only assignment to `master`; it
moves into a method `Key::set_network(&mut self, network)`, and `master`
loses its `pub` for writing: it becomes a private field with an accessor
`Key::master(&self) -> &MasterKey`. Construction of a `Key` happens only
in `wallet.rs`; if a test or another module builds one, it goes through
a constructor `Key::new(master, label, words, language, share,
passphrase)` that computes `fp`.

`core/osk-bip` is not changed: the cache is Faraday's, so there is no
new difference from upstream. The snapshot tool
(`faraday/shells/snapshot`) may keep calling `master().fingerprint()`; it
draws each screen once.

This also means fewer uses of the private key: each fingerprint call
today is a multiplication with the master private key (constant time
and blinded), thousands of times a second on the checklist.

### 3.2 The descriptor checksum is kept on the wallet (issue 121)

`wallet::Wallet` gains a private `checksum: String`, and `policy`
becomes private with `Wallet::policy(&self) -> &WalletPolicy`. A wallet
is made through `Wallet::new(name, policy, source, origin)`, which
computes the checksum; the seven struct literals in faraday-core
(`wallet.rs` `add_wallet`, `backups.rs`, `boot_import.rs`, `inbox.rs`,
`glance.rs`, `vaults.rs`, `vault_screens.rs`) use it. `Wallet::checksum(&self)
-> &str` returns it. The one place that changes a loaded wallet's policy,
`silent.rs` (a silent payment wallet's labels handed out), calls
`Wallet::set_policy(policy)`, which computes the checksum again.
`name`, `source` and `origin` stay public.

Every `w.policy.checksum()` on a `Wallet` in faraday-core becomes
`w.checksum()`. Calls on a bare `WalletPolicy` (one read from a file or
a record, not a loaded wallet) are left as they are.

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
  came in, and a vault's or an APK's hash (`Item::wallet`, `Item::sha`):
  an Item's bytes never change;
- each plan's map, by place and with its check (`plan::Mapped`), is kept
  by the shape and answers it maps (`Faraday::plan_mapped`), and the
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

A kept value is worked out by whoever asks first, a frame included
(`work::keeping`); after that every frame and every press only looks it
up.

**The guard.** In debug and test builds every costly function's count
(`work::count`) panics while a frame is drawn (`render` marks it,
`work::Frame`) unless a memo cell is filling (`work::filling`) or a kept
value is worked out the first time (`work::keeping`): "costly work while
drawing: {kind} in {function}; compute it on the state or through the
drawing memo". Release builds compile it out. Every test that draws a
screen checks every screen state it reaches, and so do the snapshot
tours. Its allowlist, `work::Allowed`, names each exception and why; the
owner decides each. One entry now, `SecretShown`: a secret the person
opened to see or copy (a seed's SeedQR grid on the copy page, a vault
key record's words, a BIP-85 child), drawn from its source each frame
because nothing secret is kept, not even for a frame (§2 rule 1).

### 3.4 Public file names without making the files (issue 123)

A new function `picture_names(name: &str, part: &str, text: &str) ->
Vec<String>` in `lib.rs`, beside `picture_codes`, gives the names of the
pictures `picture_codes(text)` would make, without making them: one name
when `text.len() <= osk_codec::qr::byte_capacity(25, Ecc::Medium)` (the
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

`Language::index_of` is not changed (it is upstream's, and with this
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
`core/osk-ui`, upstream's, and is listed in HANDOFF's "Where Faraday's
upstream files differ from upstream". If no screen clears it every frame,
nothing changes and the issue is closed with the numbers.

### 3.9 The web demo (issue 128, built in faraday-website)

In faraday-website's `web/index.html`, `pointermove` (both
`faraday_move` and `faraday_hover`) and `pointerleave` go through a
`call` that does not force `watch`: the state is read at most every
150 ms, as `watch` already does when not forced, and at once after any
other call. `draw` still runs whenever `faraday_dirty` says. The
website's `HANDOFF.md` takes the item; the website is rebuilt and
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

It is seen when the app is started from a terminal. The stick is left
out: its timings go to an exchange partition Faraday does not mount, so
a line there would reach no one.

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

Measured before batch A and after each batch; the results go in §6 of
this file. The owner's check on the slow laptop is the desktop app with
`--timings` (after batch C) through the same flow, and the stick.

## 5. Batches

Each batch is one commit, built by Sonnet from this file, audited from
the tree, `just` green.

- **A, the kept values** (§3.1, §3.2, §3.7; issues 120, 121, 126).
  Mechanical: new fields, constructors, accessors, call sites. Tests:
  - `tests/network.rs`: the "abandon … about" key reads `73c5da0a` on
    the Wallets page before and after the session moves to testnet and
    back.
  - `tests/silent.rs`: after a label is handed out, the wallet's card
    shows the checksum of the new policy (the one a descriptor read from
    its exported file gives).
  - `tests/vault_keys.rs`: a key whose words are in a vault reads as
    kept there, and a key of different words does not; if the app takes
    a seed's words in a second language, the same seed typed in that
    language reads as kept too.
- **B, the checklist and the hold** (§3.3, §3.4, §3.5; issues 122,
  123, 124). Tests:
  - `tests/backup_first.rs` passes unchanged.
  - `tests/backup_checklist.rs`: through a 2-of-3 checklist, after each
    item is done, the cards drawn tick exactly the items done and every
    card after the waiting one reads "After …" with its title.
  - `tests/backup_first.rs`: the press that completes the last item
    shows the wallet's receive address on the next frame, with no frame
    drawn between the press and the check (the decision does not wait
    for a memo).
  - `tests/pictures.rs`: for a descriptor that fits one code and one
    that needs parts, the public files item lists exactly the picture
    names of the files written.
- **C, the desktop shell** (§3.6, §3.10, §3.11; issues 125, 129, 130).
  Tests: a stick toggled in the app shows on the next frame, and a
  file the app writes is listed on the stick's page after the write
  (the desktop shell's tests, if it has a harness for them; otherwise
  the owner's check, and the report says so).
- **D, the glyph cache** (§3.8; issue 127): measured by the architect;
  built only if it clears every frame.
- **E, the web demo** (§3.9; issue 128): in faraday-website.
- **F, the vault 2 card, the sticks card, the late flow again**
  (§3.12; issue 119). Tests: `tests/backup_checklist.rs` passes
  unchanged; a vault opened mid-checklist, holding the wallet, ticks
  the vault 2 card on the next frame; a stick's files written show on
  the sticks card on the next frame.

Order: measure, A, measure, B, measure, C; D and E after.

## 6. Results

Measured 2026-10-16 with the harness of §4, release profile, in a
four-core cloud container shared with other builds, not the build
machine: the numbers compare with each other, not with MOTION.md's. The
session: a 2-of-3 wallet whose three keys were made here with coin flips,
the kit vault open, the backup preset paper, vault and software. Each
row is the median of 30 frames, in ms; the stage is the number of items
done.

| What | Before | After A | After B |
|---|---|---|---|
| 1366×768, checklist, 0 done: frame | 91 | 6 | 4.5 |
| 1366×768, checklist, 4 done (vault 1 open): frame | 155 | 11 | 8.6 |
| 1366×768, checklist, 5 done (vault 2 open): frame | 184 | 41 | 16 |
| 1366×768, checklist, 6 done (vault 3 open): frame | 195 | 10 | 5.1 |
| 1366×768, checklist, 10 done (sticks open): frame | 174 | 21 | 10 |
| 1366×768, checklist, 10 done: scroll step and frame | 202 | 21 | 8.9 |
| 1366×768, checklist, 10 done: a finger move, event only | 12.7 | 0.21 | <0.01 |
| 1366×768, Wallets page: frame | 27 | 8.7 | 9.3 |
| 2560×1600 scale 2, checklist, 0 done: frame | 96 | 12 | 11 |
| 2560×1600 scale 2, checklist, 5 done (vault 2 open): frame | 192 | 42 | 18 |
| 2560×1600 scale 2, checklist, 10 done (sticks open): frame | 176 | 28 | 15 |
| 2560×1600 scale 2, checklist, 10 done: scroll step and frame | 170 | 29 | 15 |

Before A, about 83% of a late checklist frame was `MasterKey::fingerprint`
(§1.1), most of it through `backup_seed_list_of` under
`backup_item_done`; descriptor checksums were about 10% and QR encodes
about 3%. The cost stepped up at the vault items, whose done marks
(`backup_vault_fits`, `backup_open_fits`, `backup_vault_open`) search
the session's keys for each seed.

What is left after B, from stacks sampled at 1366×768:

- The vault 2 card: `vault_has_wallet` reads every wallet record in the
  open vault (`read_wallet`, a descriptor parsed each) on every frame,
  through `backup_vault_fits` and `backup_open_fits` outside the memo.
  Not in §3.
- The sticks card: `sticks_item` (`kept_wiped`, `key_text`) and
  `wallet_stick_rows` work out each stick's files on every frame. Not in
  §3.

The glyph cache (§3.8): at 2560×1600 scale 2 no screen measured (the
checklist at every stage, the Wallets page, Create's Keys card) cleared
it, in a frame, a scroll or the whole session; the most it held was
171,800 of its 262,144 bytes. At a layout scale of about 2.5, which
the stick reaches full screen on a high-density panel, a dense screen's
glyphs would pass the budget (estimated from the area, not measured).
Batch D waits for that measurement at the stick's full-screen sizes on
high-density panels.

Timed per §7 after BACKUP-FLOW §19's F2a (the board in Back up), same
session and conditions, medians in ms: the board open 5.1 (1366×768) and
14.1 (2560×1600 scale 2); a chip selected 5.3 and 14.3; a chip dragged,
each touch move and its frame, 6.7 and 15.2; a chip's forms open 5.6 and
14.7. The checklist's rows as after B. After F2b (the list in the right-hand
column, the boxes): the board's rows within 1 ms of F2a's; the narrow
column at 800×600, a frame 2.0, a scroll step and frame 2.0, the Add
sheet 1.7. After BACKUP-FLOW §20's L2 (the access layers' lines under
the board), on the build machine, a 2-of-3 with three keys here and a
vault open, the board on Paper and vault at 1366×768, medians of 30 in
ms over three runs each: a frame 4.07 to 4.39 before and 3.97 to 4.36
after, a scroll step and frame 4.26 to 4.37 before and 4.24 to 4.62
after, within the runs' spread; a pointer move under 0.001 both; the
new Access layers card 1.84 a frame.

### After F (2026-10-19)

Measured on the build machine, release profile, with §4's harness
(outside the repository, removed after). The session: a 2-of-3 made in
Create with three vaults set up before the keys and three keys made by
New key from coin flips, all three vaults open; Back up on Paper, vault
and software, its checklist walked from the first item to Write to
sticks (blank templates, three copies by hand, three vaults, the wallet
sheet, the descriptor shown, the envelopes); then the lock and the
flow's writes. Before and after are one binary, the frame memo of §3.12
switched off and on per run, the runs interleaved: each row is the
median of five runs of the median of 30 frames, in ms; the stage is the
number of items done. A run of the code before F, alone, gave the same
before column within its spread.

| What | Before F | After F |
|---|---|---|
| 1366×768, Create's Keys card, three keys: frame | 1.75 | 1.70 |
| 1366×768, Create's Keys card: scroll step and frame | 1.73 | 1.70 |
| 1366×768, Back up, the plan with the board (All view): frame | 4.40 | 4.43 |
| 1366×768, the board: scroll step and frame | 4.55 | 4.61 |
| 1366×768, checklist, 0 done: frame | 3.62 | 3.79 |
| 1366×768, checklist, 0 done: scroll step and frame | 3.67 | 3.73 |
| 1366×768, checklist, 3 done (copies checked): frame | 3.81 | 3.78 |
| 1366×768, checklist, 4 done (vault 1 open): frame | 3.63 | 3.66 |
| 1366×768, checklist, 5 done (vault 2 open): frame | 3.63 | 3.59 |
| 1366×768, checklist, 5 done: scroll step and frame | 3.73 | 3.69 |
| 1366×768, checklist, 6 done (vault 3 open): frame | 3.85 | 3.67 |
| 1366×768, checklist, 9 done (envelopes open): frame | 4.14 | 3.93 |
| 1366×768, checklist, 10 done (sticks open): frame | 4.42 | 3.87 |
| 1366×768, checklist, 10 done: scroll step and frame | 4.48 | 3.90 |
| 1366×768, Home with the flow in progress: frame | 3.39 | 3.44 |
| 1366×768, the flow's writes after the lock: frame | 0.95 | 0.97 |
| 2560×1600 scale 2, Create's Keys card, three keys: frame | 6.03 | 6.03 |
| 2560×1600 scale 2, the board (All view): frame | 12.78 | 12.74 |
| 2560×1600 scale 2, the board: scroll step and frame | 13.08 | 12.84 |
| 2560×1600 scale 2, checklist, 0 done: frame | 10.38 | 10.08 |
| 2560×1600 scale 2, checklist, 5 done (vault 2 open): frame | 9.42 | 9.11 |
| 2560×1600 scale 2, checklist, 5 done: scroll step and frame | 9.46 | 9.70 |
| 2560×1600 scale 2, checklist, 10 done (sticks open): frame | 10.23 | 9.18 |
| 2560×1600 scale 2, checklist, 10 done: scroll step and frame | 10.36 | 9.16 |
| 2560×1600 scale 2, Home with the flow in progress: frame | 8.47 | 7.99 |
| 2560×1600 scale 2, the flow's writes after the lock: frame | 3.29 | 3.36 |

A pointer move over the board and over the checklist, event only, was
under 0.01 ms at every stage, before and after. The spread between runs
was about 0.2 ms at 1366×768 and 0.5 ms at 2560×1600.

What the two cards did per frame and do now, counted at the stages
above: a frame with the sticks card open called `wallet_stick_rows` nine
times (each about 1.0 ms outside a frame, its stick names already kept),
`kept_wiped` four times (0.15 ms each) and parsed thirteen wallet records
for `vault_has_wallet` and `kept_wiped`; now one, one and three, one per
open vault. With vault 2's card open, seven `vault_has_wallet` calls
parsed three records, now one. Every earlier stage drew
`wallet_stick_rows` twice, now once. On this machine a record's parse is
cheap, so the vault 2 card's frame moved within the spread; the sticks
card's frame dropped by about 0.5 ms at 1366×768 and 1 ms at 2560×1600.

The layer view of the board (BACKUP-FLOW §20) had not reached main on
the day, so the board is timed in its All view only.

Frames over 16 ms: none in this session at either size, the heaviest
being the board at 2560×1600 scale 2, about 13 ms. One aside, from a
harness slip that checked the first copy by hand again and again: each
further check of the same copy added about 0.7 ms to every later
checklist frame at 1366×768 (4 ms after the first, 24 ms after the
thirtieth). A person does not check one copy thirty times, but something
grows with each check; the build machine has no sampling profiler, so
its stacks are not taken. It is left for a later batch.

What grew (tracker #156, 2026-10-19): the digits typed back. The field
took every digit typed while it had the typing, also past the seed's own
48 or 96 once the copy matched, and drawing it joins all of them in
groups of four and shortens the line to its box one character at a time
(`ui.fit`), each try measured again: a cost that grows faster than the
digits. A check after Clear, and a scanned check, keep nothing more;
the record of the backup counted one more copy per check without end,
which drew nothing more. Now the field takes no digit past the seed's,
and the record counts no more checks than the plan has copies on paper
of that seed. Measured on the build machine, release profile, with a
harness like §4's (outside the repository, removed after), in a lighter
session than the one above: the 2-of-3 with two of its seeds typed in,
no vault, Back up on Paper, vault and software, the first copy's item
open with its seed shown and the typing on; one copy checked once or
thirty times, then a frame, a scroll step and frame, and a frame with
the next item open. Medians of five runs of the median of 30, in ms, at
1366×768:

| How the copy was checked again | Checks | Before | After |
|---|---|---|---|
| typed again, no Clear: frame | 1 | 4.68 | 4.60 |
| typed again, no Clear: frame | 30 | 330.98 | 4.25 |
| typed again, no Clear: scroll step and frame | 30 | 331.33 | 4.31 |
| typed again, no Clear: next item open, frame | 30 | 3.73 | 3.57 |
| Clear, then typed: frame | 1 | 4.71 | 4.36 |
| Clear, then typed: frame | 30 | 4.51 | 4.35 |
| scanned: frame | 1 | 3.98 | 4.25 |
| scanned: frame | 30 | 4.15 | 3.98 |

The spread between runs was about 0.3 ms. The growth here is steeper
than the aside's 0.7 ms a check, which came from the heavier session and
a harness not kept; the aside's exact numbers were not reproduced. The
field was the only state a further check grew that a frame reads.

The owner's check (2026-10-16): the stick built from batch B, on the
slow laptop, through the same flow: quick throughout.

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
keep what it draws, with why. The guard of §3.3 holds the same rule in
every test that draws. The timing above stays for milliseconds.
