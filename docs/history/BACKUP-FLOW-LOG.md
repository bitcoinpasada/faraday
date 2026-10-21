# BACKUP-FLOW build log: what each batch built, as it was written at the time

**Kind:** History
**Status:** Kept as written; section numbers are the spec's when it was written. The current spec is `docs/BACKUP-FLOW.md`.

## 14. Batch B1

After APK batch 1 (`docs/APK-SIGNING.md` §11) and the GPG fix batch
are committed. Renders at `1280x800` and `480x640`: Where, a stick
being brought in with the vault choice, the hold, Vaults, Lock, a stick
written, Done, the checklist's **Write to sticks**, and Make a new
wallet's new cards. It touches `faraday-core`, so the device app's
builds follow it.

**Adds.** `faraday-core/src/backup_flow.rs` (§7, the steps, the
checks); the cards (§4, §5) in one screen function used by the kinds'
Make screens and by Make a new wallet and Back up; the hooks in
`stick_arrived` and on a pull; the gate (§8) for the APK key, GPG and
Secure Boot, which no earlier batch builds; Home's lead; the Power
sheet's line; for the wallet, §10.2: Create's new cards, New key into
vaults, `Item::Sticks` in `plan::checklist`, its foot in `item_action`
and done test in `backup_item_done`, the "Written to" lines, the visit's
**In the backup** tag; §12's strings; §13's tests.

**Replaces, in APK batch 1** (`faraday-core/src/apk.rs`, `memory.rs`,
`vaults.rs`, `visit_taken.rs`, `lib.rs`, `stick_wipe.rs`):

- the placeholder `V::ApkWrite`, whose **Back up first** and **Write
  the vault to two sticks** lock the vault and open the stick visit with
  the vault ticked;
- Make an APK key's form with its vault buttons: Make is step 4, the
  vault decided at steps 1 to 3;
- `ApkState::{waits, visit_for, back, sealed}`, `Wait`, `First`,
  `waits_encode`, `waits_decode`, the kept keys `apk-waits`,
  `apk-visit`, `apk-back`; `apk_waiting`, `apk_second_refusal`,
  `apk_inbox_ticks`, `apk_written`, `apk_sealed`, `apk_first_gone`,
  `apk_return`, and their hooks;
- the strings of §12's last paragraph; `tests/all/apk.rs`'s two-stick
  tests, which move here.
- `vault_is_this_sessions` and `apk_vault_not_replaced` stay: they guard
  the ordinary visit's copy of a vault from Files.

**Replaces, in the GPG fix batch:** Make a key's vault step and its
place in Vault contents (now step 4 of the flow); **Sign a file** and
**Renew** gain the gate. **Secure Boot:** Make keys becomes step 4; the
gate on Enrolment files and Sign with db. **Nostr** (N1, not built):
whichever of N1 and B1 is second makes Save into a vault the flow.
**Decision 10** no longer applies to the GPG, Secure Boot and APK tiles.

**Replaces, in the wallet:** the done card's **Write to {a stick}**;
Back up's vault part, which moves before Keys; F6's hold moves at the
lock; the passphrase's default place in `Answers::defaults` and
`Answers::preset` (§10.3); `glance::check_lines` gains §10.3's lines;
`Shape::max_places` and `set_places`' upper clamp go; the account keys
join `backup_public_names_for` and `backup_files_make`; the Templates
item's **Make it** (`Action::BOut(0)`, `blank-template-{words}-words.pdf`)
becomes the Starter line and **Done**.

**Batch T, Faraday Starter's blank templates** (`docs/STARTER.md`
§3.8, §11), its own small batch, built before B1 so the Templates line
points at a screen that exists: it touches only the Starter
(`faraday-starter`, `shells/starter`) and reuses `faraday-core`'s
`backup::sheet_blank` and `pdf::sheet` as they are. Its build is the
Starter's.

## 16. Built (batch B1a)

What B1a built, and where the spec was silent or the build followed the
closest existing pattern (`gpg.rs`, the stick visit, the lock, the kept
state, NEW-WALLET's checklist). The wallet's part is batch B1b.

**Where it is.** `faraday-core/src/backup_flow.rs` (the state of §7, the
plan, the kept state, the gate of §8), `backup_flow_steps.rs` (the steps,
`FlowAction`, the checks) and `backup_flow_screen.rs` (the cards).
`Screen::BackupFlow` is the flow's one screen and `Action::Flow` its
presses. Step 3's Create a vault is `vault_screens::create`, drawn in the
flow's place with `Screen::BackupFlow` kept, so no other screen shows.

1. **The plan source.** A flow is built from a `Plan`: places, each with
   the vaults whose stick it keeps (none: no stick there), and the vaults
   (`VaultPlan`: planned existing or new, then its file, salt and length
   once known). `BackupFlow::new(kind)` fills it with the key's own plan,
   one new vault at two places; `BackupFlow::with_plan(kind, plan)` takes
   any other, which is how B1b gives a wallet's. Only places that keep a
   stick take part in steps 2, 5 and 6. Sticks named after step 2 (the
   stick of files, the stick of public files) are `Slot`s with
   `Target::Files` or `Target::Public` and a list of Outbox names; step 6
   checks them as step 2 does, takes the tap, and writes them by the same
   routine. Nothing makes one yet.
2. **One stick per vault, or one for both.** A place may keep several
   vaults. By default each is a stick of its own (`Target::PlaceVault`),
   asked for in turn; **One {stick} for both** on the place's card puts
   them on one stick (`Plan.together`, `Target::Place`), and **Also {vault}**, which adds
   {vault} here** adds a vault to a place. Nothing forces either. A key's
   own plan gives each place one vault; a place with "nothing" is not
   offered, since it would not count toward the two a key needs.
3. **The Tools tile.** With no vault open, or a flow waiting before its
   key is made, the tile opens the flow (**Continue** on the tile while
   one waits). With a vault open it opens that vault's category, where
   the keys and their actions are, and **Make a key** there starts the
   flow; a person with a vault open reaches an existing key without
   going through Where. Decision 10 no longer applies to these three
   tiles.
4. **Usable sticks (§4.2).** An `FARADAYBOOT` label is refused. The disk
   process lists every USB and SD device it is handed (a partition with
   no FAT volume, a disk with no partition, an empty reader slot not) and
   marks each with a `Problem` on the wire (`faraday-files::proto::Stick`):
   `NotFat`, or `ReadOnly` when the kernel's `ro` flag of the partition
   or its disk is set. The stick shell sends `StorageEvent::StickProblem`
   for each, and the flow shows it at step 2 with what to do. The privilege
   split is as `docs/history/PLAN.md` has it: only the disk process opens a node, and
   the app only sees the answer. Room is asked with `StickSpace`; a new
   vault's length is counted at the default slot's file (a larger slot
   chosen at step 3 is checked by the write).
5. **Already read (§4.2).** **Use {label}, read at {time}** is offered for
   the stick the machine started from, at step 2 and at step 6; the reading
   of it ends at the bind, so nothing waits for it to be pulled, and at
   step 6 it is checked like any stick. Keys are still made with no stick
   in, the boot stick too. Sticks read earlier in the flow are not offered
   again.
6. **Which vault.** A vault the plan already knows (its salt is known from
   an earlier place) is never asked again: its copy on the next stick is
   written over, or the vault goes beside. Choosing a file at a place
   whose vault was planned new makes it that existing vault; the choice
   takes effect when the place is confirmed. Two files of one vault, and
   a copy that differs from the one first read (**Use {label}'s copy**,
   either), are as §4.2 says. A different vault already under that name
   in Files keeps its name; the copy comes in under a free one.
7. **Refusals at step 6.** A stick that holds a file the flow wrote for
   another stick is refused before it is compared with what step 2 read
   ("This stick has the first copy" for a copy of the same vault,
   "This stick has Place 1's files" otherwise); only vault files are read
   for that, since a stick of files holds no vault in B1a. "Not the stick
   read for Vault 1 (Place 2)" names the vault and place alone.
8. **The hold at step 6** is asked for every later copy of a vault
   (**a second stick**, **another stick** from the third), whether or not
   the stick looks like the first; the first stick put back is refused
   outright, since no hold would make that right.
9. **Make (§4.4).** The key is made once and saved into every vault of the
   plan in one action; if any vault cannot take it, none does. A GPG key
   is made at once (its toast says where the revocation is kept); the
   others on the tick after "Making the key" is drawn. The result shows
   the kind's own rows, its public file with **Show as QR** and **PNG**,
   and a row per vault. Make is refused until every vault of the plan is
   open, the clock is asked for first, and Escape does not leave the
   form.
10. **Step 3** unlocks the existing vaults first (the first one still
    locked takes the typing), then makes each new one. A vault made in
    the flow is unlocked at once with the passphrase just typed twice,
    where decision 10 types it once more.
11. **The lock and where a lock resumes (§9).** A key that is made when
    a lock comes resumes on step 6, since the lock was the step; before
    it, on Vaults. The state is kept under `backup-flow` with the flows'
    plans, sealings and written copies, and no secret. What was on screen
    when it was kept opens on return.
12. **The extra round (§4.8)** shares the steps: the flow is `adding`, step
    2 reads only a stick that holds an existing vault, step 3 asks for
    the added vault and one that holds the key, step 4 is **Copy the
    key**, then the lock and step 6. A new vault on an empty stick needs
    no stick read and no lock while the key's vault is open. The paper
    case is the wallet's (B1b).
13. **Removing a place** before its write drops a new vault that held only
    this flow's key and takes the key out of an existing one; after its
    write the control reads **I destroyed this copy** and the place stays
    until it is pressed. The minimums hold.
14. **The gate (§8).** A key made in a flow waits until every one of its
    sticks is written; the key's page has the row **Backup**, its waiting
    actions read **Back up first** and open the flow on its step, and
    Files' row for an APK a waiting key fits does the same. Sign a file,
    Renew, Enrolment files and Sign with db, and Sign an APK, wait; Check a
    signature does not. A key no flow made here signs at once.
15. **A written copy is deleted or the stick wiped** opens that stick's
    card again. Each written stick keeps its label, volume serial and the
    SHA-256 of its files in the state that survives a lock, and a wipe or
    delete is matched on those, never on "all". Two sticks that read
    exactly alike (one label, one serial, the same files) cannot be told
    apart: both open again.
16. **The stick visit** lists the sealed vaults a flow will write as
    **In the backup**, not tickable, and Back is not offered while a write
    is under way. An idle power-off waits while the flow is on screen.
17. **The receipt** is the last stick's, as the visit's is.
18. **The key's public files** (GPG `.asc`, APK certificate `.pem`, the
    Secure Boot enrolment files) are made with the key and written in the
    flow at step 6 to the stick of the place the plan names (**Public
    files here**, Place 1 by default; any place may), listed on that
    stick's card and in Done. They may share a stick with a vault. A
    Secure Boot key's enrolment files wait for the backup like its signing
    does (§3), so they are written in the last write of step 6, to
    whichever stick completes the backup, never the first when there are
    two places. After Done nothing the flow made waits For the stick: the
    sealed copies and the public files move to Files, and a copy deleted
    later is written again from there.
19. **A stick that returns mid-write.** A stick pulled while it was being
    written counts nothing. When it comes back, differences that are only
    this flow's own sealed bytes or its part file are accepted, a write
    that landed whose answer was lost is recognised and counted, and a
    file the stick already holds is written beside under a free name.
    A stick named at step 6 that holds another copy of a flow vault is
    not asked which copy to unlock (the key is made); that copy is
    declined, and it is written over only after the hold. The flow's writes
    are not an ordinary visit's: its receipt is marked, so a later visit
    never writes the flow's vault over another copy. A whole wipe counts
    as soon as the partition is emptied, whether or not it is then
    stopped; one that fails or is cut off before its first answer reopens
    them too. A partition the grant has not handed out yet waits and shows
    no problem; a stick whose room cannot be asked for says it cannot be
    read, with Use another stick; each listing clears and re-tells what is
    wrong with a stick. The visit lists a waiting Secure Boot key's
    enrolment files as In the backup, not tickable.
    Limit: a later copy's stick that reads exactly like the first stick
    is refused as holding the first copy.
20. **A copy that differs** from one already read, or from the one in
    Files, is asked about at every bind, the first place included. The
    copy not unlocked is marked declined on its slot, and writing that
    stick needs the hold (a second stick's, whatever it looks like).
21. **The kept state** has a version line (`version\t2`); it is validated
    after it is read (indices in range, slots as the plan makes them, from
    Lock on the key made, from Write on a sealed copy per vault). A flow
    that does not read is dropped with the toast **A kept backup did not
    read and was dropped**. A lock from the Set the time screen reached
    from Make, or before **Copy the key** in the added-vault round,
    resumes the flow (on Vaults when the key is not in every vault). A
    flow whose key is made is never dropped: what is wrong with its steps
    (a plan changed at step 6 and locked with no Continue) is repaired by
    sending it back to Vaults as an extra round. The keys that wait are
    also kept apart (`backup-gate`), so a kept state that cannot be read
    at all leaves each of them waiting, with a flow at Where to finish it
    (such a stub knows no vault, so it stays waiting until power-off; it
    fails closed). The gate is the first thing a save writes, whole (a
    temporary file, then a rename), the flows are restored before it, and
    a lock that cannot keep a waiting key's state does not go ahead.
22. **Who writes.** Write and Confirm act only on a clean process at step
    6; a reading ends when the step changes. A stick is written only when
    every vault of its slot has its sealing and its Outbox copy, and no
    file of the slot is a secret. The plan cannot be changed while a write
    is under way. A sealing is recorded only if the key's record is in the
    vault. Removing a place takes a new vault out of the plan and the
    session only if it holds nothing but this flow's key, and never the
    last vault that holds the key. The signing functions (`gpg_code` for a
    signature, `gpg_sign_ticked`, `gpg_renew`, `sb_enrol`, `sb_sign`) refuse
    a waiting key themselves, not only the screens that offer them.
23. **Not built in B1a:** the progress bar for a write (a vault is small),
    "Changed since written" for a vault changed after a write (the flow's
    writes stay counted), a Nostr key's part, and everything of §10 and
    §11 (B1b).

**B1b needs from this code.** `BackupFlow::with_plan(Kind::Wallet, plan)`
with a `Plan` from `plan::map`: its stick places with `vault v`'s indices,
`VaultPlan::existing` from the plan's choice; a `Kind::Wallet` with
`category`, `record_kind` and `subject_of` for a wallet (the descriptor's
checksum), and a result in `make_body` of its own; `Step::Make` replaced by
New key and the checklist, entered through `flow_enter`/`flow_settle`
(which advance Sticks, Vaults and Write); the Files and Public `Slot`s with
their Outbox names for the stick of files and the stick of public files;
and `Kind::min_places` returning one. The `Step`'s Done transition already
moves the sealed vaults into Files. The gate for a wallet is
`BackupFlow::waiting` read by `backup_waits`.

## 16. Built (batch B1b)

The wallet's part: §10 and §11, on the code B1a built. Where the spec was
silent, or two readings were open, the build followed the closest B1a
pattern; each choice is here.

**Where it is.** `faraday-core/src/backup_flow_wallet.rs` (what only a
wallet needs: its flow, the hold, the files, the round),
`backup_flow_wallet_screen.rs` (its Where and Make cards and Make a new
wallet's three), the shared `backup_flow.rs`, `backup_flow_steps.rs` and
`backup_flow_screen.rs` with a `Kind::Wallet` arm where a wallet differs,
`plan.rs` (the checklist's `Item::Sticks`, `all_at_one_place`, the
passphrase's default place, no cap), `glance.rs` (the two lines under the
checks) and `lib.rs`/`screens.rs` for the checklist and the cards.

1. **The plan is `plan_of(shape, answers)`**, the map's: a place for each
   place of the map keeping the vaults whose stick it holds; vaults
   numbered as the answers number them (a number no map uses is an empty
   `VaultPlan` that no place names, so `used_vaults()` never reaches it);
   the stick of files where the map has an `At::Files` spot with seed
   files, or holding the wallet; the stick of public files where nothing
   else carries them. The public files and sheets go (`PublicOn`): to the
   stick of a place that keeps a vault copy and also holds the wallet (a
   chart edit), else to the stick of files when it holds the wallet,
   else to a stick of their own. A plan whose checklist has no
   Sheets, public files, seed files or vault stick has no slot, so no
   stick step and no lock. The default Paper preset has a sheet, so it
   has the stick of public files and the lock; a plan with no sheet
   (the sheet row unticked), no vault and no files has none.
2. **The wallet's state in the flow** is `WalletFlow`: its shape (the
   quorum and the seeds by name), its answers (`Answers::to_text`, no
   secret), its descriptor checksum and name, the fingerprints it holds
   back, and the seeds each vault was sealed with. All of it is in the
   kept state (`version 3`) and the held fingerprints in `backup-gate`
   too, so a flow that does not read still holds its keys.
3. **The vaults are set up on the flow's own screen** for a wallet, and
   Make a new wallet hosts the setup (reworked 2026-10-15, §10.2): its
   card list gains **Use vaults** (the question, Yes and No, after Kind
   and Quorum), **Number of vaults** (a count of one or more) and, once the
   setup has begun, **Bring in the sticks** and **Vaults** as closed
   cards with **Change**. The question and the count are never done
   until the person has answered and pressed their own Continue; with No,
   Continue goes to Keys and no flow exists. The setup opens on **Bring
   in the sticks**, a stick for each vault named "the stick for Vault 1"
   (a page each on a small panel); when the vaults are open it returns to
   Keys. Restore has the same two cards before Seeds and returns to
   Seeds; for a wallet read from its description, the wallet is tied to
   the setup when the setup ends.
4. **Before the keys exist** the flow's plan is the setup's: Vault *v* at
   Place *v*, each new, whatever Back up's plan will be; its shape is a
   placeholder, a seed to be made here for each key ("Key 1"). When the
   wallet is made the shape is the real one, with the count of vaults set
   up (`Shape::vaults`, kept in the flow's state), and the answers are the
   wallet's defaults fitted to it (`Answers::refit`; a vault set up stays
   whether or not a seed is ticked into it, and the plan offers at least
   that many vaults, however many seeds there are). The sticks read in
   the setup are kept by vault (`BackupFlow::parked`, not kept across a
   lock) and go to the first place Back up's plan gives each vault's
   stick.
5. **No key goes into a vault until the checklist's vault item saves it.**
   The hook that saved each key into its vaults
   (`wallet_flow_save_keys`), the mark that asked for a passphrase before
   the key existed (`CreateState::pass_marked`), Back up's adoption of the
   flow's answers and the preset's keeping of the vault part are gone.
   Back up opens at its presets; the vault item's **Save into** puts in
   the vault set up for its number each seed, its passphrase where the
   plan puts it, and the wallet description where it goes there.
6. **The checklist's last item**, `Item::Sticks`, shows the lock's Kept and
   Wiped from memory and a row for each stick by place (the stick of files
   and of public files by name), each with its files, ticked as written.
   Its foot **Lock and write** is the lock: one press (the Lock card's
   sections are in the item, so the flow's Lock card is passed). It acts
   only with every other item done and every vault of the plan open and
   holding what the plan puts in it. The vault part of the plan is saved
   (`plan_save`) first, each vault is resealed (`changes` raised, so a
   copy for every stick is sealed), the vault's open index is the plan
   vault the flow knows, else the one that fits; the flow takes the
   files the checklist made. An item that ends For the stick is done when
   its file is made, as before, or written by the flow; its closed line
   says **Written to {label}** once it was. The item is done when every
   stick is written: the Back up screen, opened again from the vault after
   Done, shows it.
7. **F6's hold.** The held keys the plan keeps in a vault move to the
   flow's state at the press (`WalletFlow::held`), and the marks come off
   the vault records and the plan's `held` lines before the seal, so the
   sealed copies say the backup is done; a key the plan keeps only on
   paper is released when the other items are done, as its copy was
   checked; every other held key is released by the one rule: every stick
   of the plan is written, and the seed's own planned sticks are among them,
   non-empty. `held_keys` is the stored hold plus the flow's; a record
   saved while a flow holds a key says nothing of it.
8. **Seed files.** A seed's file let out unprotected on the secret sheet
   is written to the stick of files and to no other, as the one secret the
   flow writes; every other file of a stick still must not be one.
9. **The account keys** (`xpub-{fp}.txt`, and `xpub-{fp}-bsms.txt` on a
   `wsh` or `sh(wsh)` multisig at Create's own account) are among the
   public files: named by `backup_public_names_for` (without signing),
   made by `backup_files_make` with the other public files when the
   checklist is made, written to the stick that carries them. A key not
   held here has none; a single key has none.
10. **Templates.** The item's body says "Print {n} from Faraday Starter:
    Blank templates, {words} words, {Words|SeedQR|Both}" with the 24 or
    12 chosen, its foot is **Done** (`BackupState::printed`), and no
    template is made: `blank-template-{words}-words.pdf` and
    `public_made`'s arm for it are gone.
11. **§10.3.** `plan::all_at_one_place` (the three cases of §10.3) and
    `Check::{one_place, passphrase_apart}`; `glance::check_lines` draws the
    two lines under the three boxes wherever they are drawn, with the
    place's name from the chart's nodes or the vault. **Add a passphrase**
    after the wallet exists opens Create anew on Keys with the single
    key in its slot and the fields open: a new key and wallet. The
    passphrase's place is the first that keeps no copy of its seed in
    any form; where none, a place is added that keeps no paper
    (`Answers::bare`, a new `bare` line in the plan's text), since the
    spread would otherwise give every new place another copy of the
    words. `Answers::preset` places the passphrase after the vault's stick.
    A single key with Paper and vault and a passphrase: words at Places 1
    and 2, the vault's stick at Place 2, the passphrase at Place 3.
12. **No cap.** `Shape::max_places` is gone and `set_places` keeps its
    lower bound of one; so does `Answers::from_text`.
13. **Restore** takes the same question, count and setup before its
    seeds, its seeds into memory, then Back up's plan and the checklist. A restored key is no key made here, so nothing is
    held back and it signs at once; the flow is offered by the plan and
    the key's page shows the written count as any flow does.
14. **Change the plan** for a wallet edits the answers (`FlowAction::Answer`
    takes Back up's question and row), at every step; a stick already
    written is not taken off the plan from here. After the lock a vault
    that now holds a seed it was not sealed with is as one added: its
    written sticks are cleared, the round unlocks it and the vaults that
    hold the seed, copies the records across, locks and writes again. The
    round reads the key records of the open vaults; a seed kept only on
    paper is typed back (**Add the key from its copy**) and then saved.
15. **The done card** loses **Write to {a stick}** and the line of files
    waiting; **Open the wallet** is its first button. Done's **Unlock
    {vault}** opens the wallet's checklist, done. A plan with no vault has
    **Home** in its place.

16. **Review fixes.** (a) A flow that is Done is never looked up for a new
    backup: a new lock starts a fresh flow for the wallet, so a second
    backup in the same power-on writes again. (b) Each wallet's flow is tied
    to its Make a new wallet or Restore by an id (`owner`, not kept across a
    lock). (The press hook that saved keys into the flow's vaults is gone;
    see the reworked note below.) (c) F6
    holds every seed the plan keeps on any stick, in a vault or as a seed
    file, and moves it to the flow at the lock. (d) `backup-flow` is written
    like `backup-gate`, by a temporary file and a rename; a flow with no
    wallet part is not read, and the gate's held fingerprints are merged
    into a flow that is. (e) `refit` returns which rows it kept and the
    plan's vaults follow their rows, so a vault keeps its salt when a slot
    turns out to be a cosigner's key. (f) After the lock a seed is matched by
    its words (its base fingerprint), the passphrase is put in the vault only
    where the plan puts it there, and the round copies only this wallet's
    `WALLET` and `PLAN` records. (g) A vault preset chosen after the keys
    exist is the adding-after-the-keys case of §4.8: a new vault is made at
    once with the key in it and its stick is checked at step 6; a vault that
    exists takes the extra round. (h) Make a new wallet's Bring in
    and Vaults cards are done once the wallet is made (the question and the
    count are done only by their own Continue), and
    Check's Continue opens Back up. (i) A seed has one record when a passphrase is
    added later. (No record is saved before the lock now, so none is
    removed.) (j) A plan changed after the
    lock to add the wallet description or a passphrase to a vault that
    exists is a round. (k) `refit` recommends and never forces: a passphrase
    is placed apart only when a preset is chosen or a passphrase is added;
    attaching, adopting and resuming move no placed passphrase and add no
    place. (l) A seed file is matched as `{stem}-{fp}-...` for this wallet's
    seeds exactly, in the slot's files and again at write time.
17. **Public files beside a plain seed file.** When any seed file is
    planned, the public files get their own stick by default (`PublicOn::Own`)
    so a seed file never shares a stick with the files that are handed
    around. Merging them stays a deliberate choice. This is a default the
    owner may change.
18. **Back** on a card of a wallet's flow before its keys exist returns to
    Make a new wallet or Restore. Taking a written stick off the plan says
    so in a toast (**A stick already written stays on the plan**). The seed
    files and shares items of the checklist read **Written to {stick}** once
    written.

19. **Second review.** A key held back is released by one rule: every
    stick of the plan is written, and the seed's own planned sticks are among
    them; the second is a check against a plan that shrank. After the lock
    an answer that leaves a held seed or its passphrase on no stick, or a
    vault with no stick, is refused with a toast. A plan change after the
    lock marks the vaults that must hold more as stale at once, without
    Continue. A
    seed file goes only to a stick of files that does not carry the public
    files, and only for a seed the plan keeps as a file now. Every `refit`
    remaps the flow's vaults, the lock keeps the flow's own vault while it
    fits, every Make a new wallet and Restore has its own id (never 0, and a
    flow read back after a lock gets one nothing matches), a Done flow that
    waits hands its hold to the flow that replaces it, and Add a passphrase
    acts on the wallet shown.

20. **Review of the reworked order (2026-10-15).** (a) A plan vault with no
    vault set up for it, while another is open, offers **Make a vault**
    (a new vault on an empty stick, written at the write step as an added
    copy is) and, when a vault on a stick read this power-on is locked,
    the way to unlock it. A vault on a stick not read yet is the extra
    round of §4.8, which is not built before the keys (see Open).
    (b) **Lock it and make a new vault** locks the vault the plan vault
    would be saved into, never the selected one; the vault set up for a
    plan vault is not locked from there, and the press says so.
    (c) **Lock and bring the sticks** asks as any lock does, and the Make a
    new wallet or Restore that asked for the setup is kept across it
    (kind, quorum, answer and count, nothing secret) and returns past
    its questions. The question and the count are fixed once a vault of
    the setup is open, and Keys (Seeds on Restore) stay closed until the
    setup is done. (d) The question and the count are done only by their
    own Continue, also once the wallet is made. (e) The count is one to
    1024, what a saved plan reads back.

**Reworked (owner, 2026-10-15).** Items 3 to 5 and 13 describe the order
as reworked. The first build asked where the keys would be kept (places,
vaults, which seed goes into which vault, the stick places) before any
key existed, in a card of seed forms and then four checkboxes; that
card, its actions, the plan editing before the keys and the hook that
saved each key into its vaults are gone. The tests that carried that
order set up the vaults first and decide the rest in Back up.

**Open (for the owner).**

- **Keep it in a vault for the round** is not built, because a lock wipes
  Create and the checklist. What is built is the round after the lock, with
  the key typed back from its checked copy, which is the default choice.
- **A key kept only on paper in a multisig whose description reaches only a
  stick** is released before the writes (§16 item 7 of B1a). Is that the
  wanted order, or does it wait until the stick that holds the description is
  written?
- **L4 and L5 of the security review** fail closed: nothing secret is lost
  or sent. They are left as they are.

**Not built in B1b, and why.**

- **The two choices for a key kept only on paper before an extra round**
  (§4.8, §15 item 7): **Finish its paper copy first** and **Keep it in a
  vault for the round**. A wallet's keys live in Create and the checklist,
  whose state a lock wipes, and the flow's round that starts before the
  final lock has nowhere to return to once the lock has wiped them: the
  decision needed is whether such a round returns to the checklist (and
  which of its items it keeps) or to the writes. What is built is the round
  after the lock, where the key is typed back from the copy the checklist
  checked. This needs the owner.
- **The Nostr key's part** (N1), as before.
- **A progress bar for a write** and **"Changed since written"**, as in B1a.
