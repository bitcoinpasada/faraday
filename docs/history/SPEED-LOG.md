# Speed in long flows: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/SPEED.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.

## The header (from docs/SPEED.md)

**Kind:** Spec
**Status:** In progress (batches C to E)

2026-10-16, batches A and B built and checked by the owner on
the slow laptop's stick; C next, after BACKUP-FLOW §17's F batches; D
measured at the stick's high-density sizes before it is decided (§6); E
in the website's own repository.
§7 is a standing rule (owner). §8: the speed audit's batch S1
(2026-10-21); §11: batch E of 2026-10-21, the stick's panel and the
Starter. Tracker issue 119 and its sub-issues 120–130.

## §1 What is wrong (read from the code at 2e807ae; not yet measured) (from docs/SPEED.md)

Faraday draws in immediate mode: `Faraday::render` rebuilds the whole
screen from the app's state on every frame, and a frame is drawn on
every step of a scroll or glide, every change of what is under the
pointer, every caret blink, every step of a fade or a toast, and twice
on the frame a step card opens. Whatever a screen works out while it
draws is worked out that often. The backup screens work out a great deal:

1. **A key's fingerprint is computed on every call.**
   `MasterKey::fingerprint` (`core/faraday-bip/src/keys.rs`) derives the
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
   `backup_screen` (`screens/backup.rs`) calls `item_title`, `backup_reachable`
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
   `core/faraday-draw/src/fonts/mod.rs` drops every rasterised glyph when the
   cache passes `CACHE_BUDGET` (256 KB). One dense screen's glyphs at
   twice the size may not fit, and then every glyph is rasterised again
   on every frame. Not confirmed.
9. **The web demo reads its whole state on every pointer move.**
   The website's `web/index.html` sends each `pointermove` through
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

## §4 Measuring: its last paragraph (from docs/SPEED.md)

Measured before batch A and after each batch; the results go in §6 of
this file. The owner's check on the slow laptop is the desktop app with
`--timings` (after batch C) through the same flow, and the stick.

## §5 Batches (from docs/SPEED.md)

Each batch is one commit, built by Sonnet from this file, audited from
the tree, `just` green.

- **A, the kept values** (§3.1, §3.2, §3.7; issues 120, 121, 126).
  Mechanical: new fields, constructors, accessors, call sites. Tests:
  - `tests/all/network.rs`: the "abandon … about" key reads `73c5da0a` on
    the Wallets page before and after the session moves to testnet and
    back.
  - `tests/all/silent.rs`: after a label is handed out, the wallet's card
    shows the checksum of the new policy (the one a descriptor read from
    its exported file gives).
  - `tests/all/vault_keys.rs`: a key whose words are in a vault reads as
    kept there, and a key of different words does not; if the app takes
    a seed's words in a second language, the same seed typed in that
    language reads as kept too.
- **B, the checklist and the hold** (§3.3, §3.4, §3.5; issues 122,
  123, 124). Tests:
  - `tests/all/backup_first.rs` passes unchanged.
  - `tests/all/backup_checklist.rs`: through a 2-of-3 checklist, after each
    item is done, the cards drawn tick exactly the items done and every
    card after the waiting one reads "After …" with its title.
  - `tests/all/backup_first.rs`: the press that completes the last item
    shows the wallet's receive address on the next frame, with no frame
    drawn between the press and the check (the decision does not wait
    for a memo).
  - `tests/all/pictures.rs`: for a descriptor that fits one code and one
    that needs parts, the public files item lists exactly the picture
    names of the files written.
- **C, the desktop shell** (§3.6, §3.10, §3.11; issues 125, 129, 130).
  Tests: a stick toggled in the app shows on the next frame, and a
  file the app writes is listed on the stick's page after the write
  (the desktop shell's tests, if it has a harness for them; otherwise
  the owner's check, and the report says so).
- **D, the glyph cache** (§3.8; issue 127): measured by the architect;
  built only if it clears every frame.
- **E, the web demo** (§3.9; issue 128): in the website's own repository.
- **F, the vault 2 card, the sticks card, the late flow again**
  (§3.12; issue 119). Tests: `tests/all/backup_checklist.rs` passes
  unchanged; a vault opened mid-checklist, holding the wallet, ticks
  the vault 2 card on the next frame; a stick's files written show on
  the sticks card on the next frame.

Order: measure, A, measure, B, measure, C; D and E after.

## §6 Results (from docs/SPEED.md)

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

Timed per §7 after BACKUP-FLOW §17's F2a (the board in Back up), same
session and conditions, medians in ms: the board open 5.1 (1366×768) and
14.1 (2560×1600 scale 2); a chip selected 5.3 and 14.3; a chip dragged,
each touch move and its frame, 6.7 and 15.2; a chip's forms open 5.6 and
14.7. The checklist's rows as after B. After F2b (the list in the right-hand
column, the boxes): the board's rows within 1 ms of F2a's; the narrow
column at 800×600, a frame 2.0, a scroll step and frame 2.0, the Add
sheet 1.7. After the access layers' L2 (their lines under
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

The layer view of the board (since removed) had not reached main on
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

## §8 The speed audit and batch S1 (2026-10-21): its opening (from docs/SPEED.md)

The speed audit of 2026-10-20 (a session's report, kept outside the
repository) measured every path a person waits on with a harness like
§4's, extended with a counting allocator, pixel counters, three more
counted kinds and a log of every press, event and frame. Batch S1 fixed
its findings 1, 2, 3, 5, 6, 7 and 8 at their cause and put each under a
count in `just`. Findings 4 (PNGs read on the UI thread), 9 (presses
that make many pictures), 10 to 12 (the frosted backdrop, pixel
overdraw, the word list at high density) are later batches.

## §8.1 What S1 fixed (from docs/SPEED.md)

Each finding, its cause, and the fix. "Before" is `main` after the
parse-once work and the guard (§3.3), which had already removed the
vault records' parses per press (finding 6) and the import sheet's
descriptor parses per frame (finding 7); "after" is S1. Build machine,
release profile, the audit's harness (outside the repository), medians
in ms at 1366×768 unless said. The before run shared the machine with
another build (the heavy session took 1.3 s to build against 0.6 s
after), so single numbers read ±30 %; the counts are exact.

1. **A page at rest redrew at 60 Hz at a scale that is not whole.** A
   frame rounded the scroll offset to whole pixels, which could land a
   hair past the page's end; the flow pages (and the Spend tab) clamped
   it back to the end and asked for another frame, which rounded it past
   again, for ever. Now the frame never rounds past the end the last
   frame drew (`Faraday::whole_offset`), and a page counts an offset as
   past its end only beyond float noise (`flow::past_end`). Before: 120
   of 120 idle ticks drew on the Backup board at 1.25, 1.5 and 2.5
   (1.2 to 6.0 s of drawing in 2 s), the checklist at 1.75, Spend at
   1.25 and 2.5. After: none at any of 1, 1.25, 1.5, 1.75, 2, 2.5 (the
   harness) and 3 (the test).
2. **Wallets and Home parsed every PSBT in Files, and matched each
   against every wallet, every frame** (`lead_psbt`, `spend_button` →
   `psbt_for` → `wallet_for`). Each Files item now keeps what its PSBT
   says, read once as it comes in (`Item::psbt`, a `wallet::PsbtSeen`:
   txid, signature count, the first input's script and places), from
   the same parse that classifies it (`wallet::classify_read`); which
   wallet it spends from is found from that against each wallet's
   scripts, which a wallet derives once per place and keeps
   (`Wallet::script_at`, dropped with its policy), so it follows the
   wallets as they load and change. Before: Home 21 PSBT parses a frame;
   a wallet's card with the vault's wallets loaded 23 to 48 ms (42 parses
   and a match against 13 wallets each frame); with 405 files 80 ms and
   with 1,620 files 313 ms (600 ms on the Pi's layout). After: no PSBT
   parse or match in any frame; Home 1.7 ms, every card 2.8 to 3.4 ms,
   the card with 1,620 files 2.4 ms (0.9 ms on the Pi's layout).
3. **With a spend open, Files and the spend's cosigner rows parsed every
   PSBT in Files each frame** to find copies of the same transaction
   (`screens/spend.rs` and `screens/files.rs`, two places each). They compare the kept txid. Before:
   Files 18.8 ms a frame with 82 PSBT parses; after 6.4 ms with none.
4. **Opening a spend inspected the PSBT four times, and Sign here
   twice** (`start_spend` and `refresh_spend`, each with `inspect` and
   `Spend::signers`' own; `refresh_spend` runs after some forty kinds of
   press). The spend keeps its inspection with what it is of — the PSBT,
   the session's network, keys (by id) and wallets (by checksum), the
   carry (`SpendState::inspected`, `wallet::InspectInputs`) — and
   inspects again only when one changed; `signers` reads it
   (`Spend::signers_by`), and Sign here chooses its keys from it when it
   is current (`Session::sign_with`'s `known`). Before: StartSpend 6 to
   9 ms at one input, 84 to 114 ms at 100 inputs; SignHere 6 to 9 ms and
   82 to 112 ms. After: StartSpend 1.6 to 2.6 ms and 22 to 31 ms;
   SignHere 3.9 to 5.7 ms and 21 to 30 ms (one inspection inside the
   signing, one of the signed PSBT).
5. **Vault presses re-parsing every wallet record** (finding 6): gone
   before S1 (`VaultRecords` keeps each record's description read).
   The press budget test (§8.3) now holds every vault press to it.
6. **Load ticked parsed every wallet record again** (finding 8,
   48 parses, then 12 after the parse-once work): it loads the
   descriptions `VaultRecords` read at unlock (`Session::add_policy`).
   Before: the press 11.6 ms, then Wallets 49 to 53 ms a frame. After:
   3.2 ms, then 5 ms a frame; no description parsed.
7. **The boot import sheet** (finding 7): no counted work a frame since
   the parse-once work; S1 adds its scaling test. Its first frame 32 to
   57 ms before, 19 to 20 ms after (from the Inbox's PSBTs no longer
   parsed on the Home under it); then 4.6 ms a frame against 10 to 15.
8. **The power sheet parsed each loaded seed's words every frame**, to
   say which vault keeps it (found by S1's sheet budget). It compares
   the hash of each seed's entropy the key and the vault's records keep
   (`Key::words_id`, `VaultRecords::words_id`).

Files still grows with the Inbox without any counted work: 3.8 ms at
81 files, 11 ms at 405 and 40 ms at 1,620 (78 before): it lays out
every row, seen or not. A later batch.

## §9 Batch S2: nothing a person waits on blocks a frame (2026-10-21) (from docs/SPEED.md)

The speed audit's findings 4 and 9 to 12, and what S1 left in Files.
Build machine, release profile, the audit's harness and three probes
(all outside the repository); "before" is `main` at 0712461e, "after"
is S2, both run twice in turn on the same machine, medians in ms. The
"Pi" column is the Pi's layout on the build machine's CPU; a Pi 3 is
about ten times slower.

1. **Pictures on a stick were read for their codes on the shell's own
   thread, all at once.** A boot stick or a visit with pictures froze
   the window until every one was decoded: 15 code pictures and two
   photographs (1920×1080 and 4000×3000) took one pass of 512 ms with
   no frame drawn, the photograph alone 349 ms (est. 3 to 5 s on a Pi).
   Now a picture asked for is read off the shell's thread, one at a
   time in the order asked, and its answer comes as a storage event on
   a later pass (`faraday_storage::Sticks::read_qr_later`,
   `pictures_read`). On a desktop, `Dirs` keeps a thread that reads
   the file and its codes (`Pictures`); on the device the disk process
   keeps decoding there (`docs/QR.md` §4) but on a thread of its own,
   answering the requests asked meanwhile, and the shell has one
   picture asked of it at a time, so the process holds one picture's
   bytes (`DiskProcess`, `faraday-disk`'s `main`). An answer for a
   shell that has gone (a lock) is dropped. The boot stick's sheet
   says "Being read" for each picture until it comes; the visit's row
   now says so too (`VisitState::reading`). After: the longest pass
   0.7 to 0.9 ms, a frame drawn every pass (23 passes), all read in
   478 ms. A picture is decoded once and searched for light-on-dark
   codes only when it holds no dark-on-light one
   (`faraday_codec::decode::decode_picture`, one count where it was two):
   the 4000×3000 photograph 349 to 312 ms, 1920×1080 58 to 52 ms. The
   second search stays: dropping it would stop a dark-mode screenshot
   from reading.
2. **Presses that stretch a key or make pictures did it before any
   frame.** A press whose work stretches a key (PBKDF2, scrypt,
   Argon2) or makes more than one picture or PDF is now drawn first and
   done on the tick after (`slow`, `Faraday::press_in`, `Later`): a
   person's tap or Enter does none of that work, the frame after it
   shows the press and, for files or a node key, a toast ("Making the
   files", "Working out the node key") that goes when the work is
   done. The list came from the whole suite and the snapshot tours (about
   forty kinds of press: the checklist, a seed's files at the lock, every
   load of a key, Create's Next, New key's Lock, a vault's key saved,
   the chart's checks, Import, Lock, the self-test). A slider that
   changes the shares (`BOmit`) sets its value at once and makes the
   files again once it is let go and a frame drawn; working out which
   files it replaces no longer makes them (`Faraday::public_names`).
   Enter on Lightning's aezeed goes the same way. A scripted press
   (`Faraday::press`, the snapshot tool, the tests) does the work at
   once, as a script waits for no frame. Before → after, the press
   itself: the checklist of a 2-of-15 (15 pictures) 186 to 195 ms → 0.0
   (the tick after 162 to 185 ms); the aezeed's Enter 126 to 131 →
   0.0 (the tick 125); Add a key's Add 1.1 → 0.0 (the tick 1.1). **The
   rule, in every test** (`press_in`): in a debug build a press not on
   the list that stretches a key or makes more than one picture panics,
   naming itself; `tests/all/slow_press.rs` taps the checklist, Add and
   Enter on an aezeed and checks that the tap does none of it, the
   frame says so, and the tick does it. Enter on Add a key's, New
   key's and a chart's fields does what their buttons do at once: a
   key, not a press. **Typing waits for a pause** (COMPUTE D1): the
   passphrase fields on the potential wallet's and the seed read's
   sheets (`Sheet::Potential`, `Sheet::SeedRead`), and Load a key's
   probe of the words typed, worked out the key on each character typed (one
   PBKDF2, about 1 ms here). They now work it out once typing has
   paused for `motion::TYPED_PAUSE_MS` (400 ms); tests wait with
   `testkit::pause`. Enter and the sheets' buttons work at once. The
   key budget test (§8.3) holds every field to it.
3. **Files laid out and drew every row, seen or not; Vault contents
   walked ~24,400 glyphs a frame.** A line of text wholly above or
   below the clip is now measured from its advances, not drawn
   (`Canvas::text_with_face`), and Files, the visit's lists and a
   small panel's file cards pass over a row out of sight with its
   height worked out without drawing (`Ui::shows`,
   `parts::wrap_buttons_height`, `file_card`, which checks the two
   heights agree in a debug build). Glyphs a frame: Vault contents
   24,395 → 2,008 (1366), 23,344 → 331 (Pi); Files 5,750 → 1,301,
   9,072 → 203 (Pi); with 1,620 files 89,882 → 1,252 and 149,843 →
   244 (Pi), the same as with 81. Frames: Vault contents 4.6 → 3.7,
   4.2 → 2.9 (Pi); Files with 405 files 11.7 → 9.9, 18.6 → 9.9 (Pi);
   with 1,620 files 42.7 → 33.4, 73.2 → 37.5 (Pi). Files still
   worked out what the Inbox gives (`found_in`) once a frame, about
   20 µs a file; kept in §10. The scaling test (§8.3) now also holds every screen to the
   same lines of text drawn with one and five copies of the files
   (`Canvas::lines_drawn`).
4. **`ui.fit` measured the string again for each character it took
   off.** It adds the characters' advances once and stops at the first
   that does not fit with "…" (`Ui::fit`); the same output, which a
   test checks against the old loop for eight strings, nine widths,
   three faces and three densities. A 200-character descriptor into
   120 units 441 µs → 4.9 µs; a 40-character file name 16.9 → 1.4 µs.
5. **Pixels.** At 2560×1600 and 3840×2160 a scale of 2 and 2.5:
   - The frosted page under a sheet, made when it opens, reads a
     quarter of the pixels and works out the enlarged picture once for
     each two by two pixels where a small pixel is sixteen or more
     across (`Canvas::frost`). Below that, at 1× and on the Pi, it is
     the same pixels as before. At 2× the frosted area differs from
     before by 1 to 3 levels of 255 in 2 to 5 % of the pixels (39 of
     the 232 renders of the 2560×1600@320 tour, all sheets; the rest
     identical): not visible side by side. Frost alone 9.7 → 7.0 ms
     (2560), 20.7 → 14.4 (3840).
   - A sheet's later frames put back only the box drawn over since the
     page was last put back (`Canvas::restore`, which tracks the box
     every drawing call writes); the same pixels as a whole copy, which
     a test checks.
   - A screen change's fade passes over runs of 16 pixels where the two
     frames agree (`Canvas::blend_between`); the same pixels.
   - A shadow skips its covered middle instead of stepping through it
     (`Canvas::shadow`): 3.4 → 2.5 ms (2560), 5.4 → 3.6 (3840) for a
     sheet-sized one.
   - The backup board's overdraw is its look (each box's edge under its
     fill, a chip's shadow, fill and edge): left as it is; it gains
     from the text and shadow changes only (2560 13.2 → 12.0 ms).

   Frames (the Learn sheet over Settings; Settings to Tools and back):

   | | 1366 | 2560 | Pi | 3840 |
   |---|---|---|---|---|
   | sheet's first frame | 11.0 → 9.6 | 39.9–44.3 → 27.9 | 4.6 → 3.9 | 101 → 75 |
   | sheet's later frames | 4.8 → 3.6 | 20.8 → 14.8 | 2.6 → 2.0 | 37.8 → 29.1 |
   | screen change, first frame | 3.5 → 1.8 | 13.7 → 7.7 | 1.7 → 1.1 | 28.2 → 15.0 |
   | fade frames | 2.6 → 0.9 | 10.3 → 3.4 | 0.8 → 0.3 | 20.6 → 7.2 |
   | Load sheet over Files | 2.2 → 1.8 | 9.2 → 6.7 | 2.1 → 1.9 | 16.8 → 12.6 |

The renders of the 1280×800, 3840×2160 and `--sd-card` 480×640@286
tours are the same pixels before and after; the 2560×1600@320 tour
differs only in frosted pages, as above.

## §10 Batch S3: its opening (from docs/SPEED.md)

§9 item 3 left Files working out what the Inbox gives (`found_in`, for
its panel of wallets and keys to load) once a frame, about 20 µs a file.

## §10 Batch S3: the measurements (from docs/SPEED.md)

Files frame, release profile, the build machine, 1366×768, median of
41 frames, ms:

| Inbox files | before | after |
|---|---|---|
| 81 | 3.48 | 1.59 |
| 405 | 9.75 | 1.49 |
| 1,620 | 33.59 | 1.55 |

The frame no longer grows with the Inbox. `found_in` alone is 1.9, 8.2
and 31.7 ms at those sizes, now paid once after a change, not each frame.

## §11.1 E1: what was found (from docs/SPEED.md)

**Found.** The stick shell (`faraday/shells/stick`) draws when the core
asks (`Command::Draw`), at most once a loop pass; at rest it draws
nothing. But each draw converted the whole frame to the panel's format
(`fb::convert_into`, a pass over every pixel), and only then did the
panel compare the converted rows with what it shows and write the rows
that differ (`panel.rs`). The core paints its whole canvas on every
draw, so a frame drawn again with nothing changed (a press where nothing
is, a toast's frame that lands on the same pixels) converted a whole
frame and wrote none of it.

## §11.1 E1: the measurements (from docs/SPEED.md)

Build machine, release profile, a frame with nothing changed:

| Panel | convert (before) | compare (after) |
|---|---|---|
| 480×640, RGB565 | 0.27 ms | 0.02 ms |
| 1366×768, BGR888 | 0.98 ms | 0.08 ms |
| 1920×1080, BGRA8888 | 1.56 ms | 0.26 ms |

Before, the panel's own comparison of the converted rows came on top.

## §11.2 E2: what was found (from docs/SPEED.md)

**Found**, reading every path a frame and an event take
(`faraday/faraday-starter`, `faraday/shells/starter`):

1. **A sheet's frosted page was made again every frame.** The Starter
   made a new `faraday_ui::Ui` each frame with no kept backdrop, so each
   frame with a sheet open (Restart, Uninstall, Continue anyway) drew the
   whole page, reduced and blurred it (`Work::Blur`) and copied it
   (`Work::FullCanvas`) before drawing the sheet: on every hover change
   over the sheet's buttons, every press, every frame drawn again. A
   sheet frame took 5.4 ms at 1100×760 and 13.7 ms at 2200×1520; with
   the page kept, 0.9 ms and 3.0 ms (release, build machine).
2. **The Downloads folder and the cameras were read on the window's
   thread every 0.7 s**: a directory listing with each file's metadata,
   and on Windows a Media Foundation start-up and device enumeration,
   whether or not Transfer was open. A large Downloads folder or a slow
   enumeration was a stall of the window at that rate.
3. **Frames asked for that change nothing**: a wheel turn at a page's
   end, and a pointer moved with the button held that keeps the press as
   it was, each drew the page again.
4. The Starter's canvas noted no pixel work, so no count could see 1.
