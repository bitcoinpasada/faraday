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

**Adds.** `faraday-core/src/backup_flow/mod.rs` (§7, the steps, the
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

**Where it is.** `faraday-core/src/backup_flow/mod.rs` (the state of §7, the
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

**Where it is.** `faraday-core/src/backup_flow/wallet.rs` (what only a
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

Sections below moved out of `docs/BACKUP-FLOW.md` on 2026-10-22; the source keeps a
one-line stub under each heading.

## Header (from docs/BACKUP-FLOW.md)

**Kind:** Spec
**Status:** Built; the Nostr key's part is not

Spec v0.3 · 2026-10-14 · decided by the owner (`docs/DECISIONS.md` F31),
reworked the same day to the order "decide where the key will live, then
make it". Built for the APK key, GPG keys, the Secure Boot keys and the
Bitcoin wallet (§10, §11).

## §0 Rules for the implementing agent (from docs/BACKUP-FLOW.md)

- `CLAUDE.md` applies in full: every build and test on the build machine; every dimension a token; labels, not
  prose, on working screens; tests for user-facing behaviour only;
  `just` once at the end of the batch; no `rm`; no commits; no agents.
- Every string in `faraday/faraday-strings/src/en.rs` with the prefix
  `backup_flow_` (§12), the one strings file (F68).
- A stick is handed only to a clean process (`docs/history/PLAN.md` §5.3 and §5.4).
  Nothing in this flow opens a secret in a process that holds a stick,
  or hands a stick to a process that holds a secret. A change that would
  need either stops and asks the owner.
- The flow is driven by the plan and narrows nothing the plan can say
  (§2). A change that would make the flow refuse a plan the plan screens
  accept stops and asks the owner.
- Completion report: items done, each with its tests and whether they
  pass; what was left out and why; docs changed; renders to look at.

## §4.8 Changing the plan: the extra round for an existing vault, dropped (from docs/BACKUP-FLOW.md)

- **A stick holding an existing vault the key should go into**:
  *Dropped (owner, 2026-10-20): an existing vault is
  brought in only before the keys (§10.2, §17.21 item 5). The round
  below is not built.* One
  guided extra round, inside the flow, on its own card, **Add {vault}**:
  1. **Lock**: the key's vaults are sealed and kept For the stick, as
     the Outbox is kept; everything secret wiped;
  2. **Insert** that stick, its checks, "Which vault?", the vault copied
     in, the place confirmed (§4.2); pull it;
  3. **Unlock** that vault and one vault holding the key; the key is
     copied across (a wallet's seed by **Save into {vault}**, with its
     passphrase where the plan puts it there; another kind's record as
     it is);
  4. **Lock**, and the flow goes on at step 6.
- **A key kept only on paper** does not survive that lock. Before the
  round the flow offers both, in this order: **Finish its paper copy
  first** (the Copy item and F29's check, then the round, the key
  typed back in from the copy at its unlock), and **Keep it in a vault
  for the round** (a vault made in memory with its own passphrase,
  sealed at the lock, unlocked in the round and dropped after it, never
  written). The first is chosen.

## §10.1 What the code does today (from docs/BACKUP-FLOW.md)

- **The checklist** (`plan::checklist`, `plan::Item`): Templates, a Copy
  per paper seed, a Vault per vault made, SeedFiles, Sheets,
  PublicFiles, ShowDescriptor, Envelopes, each only where the plan
  needs it.
- **Four end in For the stick**: Templates and Sheets (**Make it**,
  `Action::BOut`), SeedFiles (`Action::BFile` through the secret
  sheet), PublicFiles (`Action::BFilesMake`); `backup_item_done` counts
  them done once the file is For the stick or on the last receipt.
- **The vault items write nothing**: `item_action` gives **Make a
  vault** or the way to one (`vault_way`), **Save into {vault}**
  (`Action::BVaultSave`), **Lock it and make a new vault**
  (`Action::BNewVault`, sealing that one vault with `vault_lock_one`),
  then **Continue**; done by `backup_vault_fits`.
- **The writes are left to the stick visit**: the done card's **Write
  to {a stick}** and the chart's stick node (`chart_edit.rs`,
  `glance/sheet.rs`) send `Action::WriteAsk`: with anything open the
  Write out sheet, whose **Lock** sends the next stick to
  `Screen::Visit`; clean, `Screen::Visit` at once.
- **The plan comes after the keys** (decision 20): New key makes the
  seed in memory; a vault item saves it later; F6's hold lifts when
  every item is done, before anything is written.

## §10.2 and §13: the lines naming the extra round (from docs/BACKUP-FLOW.md)

§10.2 item 6 ended:

     added copy is (§4.8). A plan that wants a vault that was not set up
     takes the extra round of §4.8.

§13's Changing the plan test read:

one stick to step 6; adding a stick holding an existing vault runs
  **Add {vault}**: lock, the stick read and its vault picked, both
  vaults unlocked, the key copied across, lock, step 6, with the screen
  never `Screen::Visit`; a key kept only on paper is offered **Finish
  its paper copy first** before that round. Removing

## §15.1 The cards, items 2, 3 and 5 (from docs/BACKUP-FLOW.md)

2. *(Replaced by §17 in Back up, batch F2a, and in the wallet's guided
   flow at any width, batch F2b.)* **Places**: "Number of places", the stepper (one or more, no upper
   bound, §10.3), and with a vault open each place's name, as today. The
   share question and the vault-stick sections leave this card.
3. *(Replaced by §17 in Back up, batch F2a, and in the wallet's guided
   flow at any width, batch F2b.)* **Place 1**,
   **Place 2** … one card per place, titled with the
   place's name where it has one. A multi-choice list (DESIGN §4.2) of
   what the place keeps, a row each:
   - **Seed words by hand**
   - **SeedQR by hand**
   - **Vault 1**, **Vault 2** …, a row per vault of the plan
     (`Shape::vault_rows`), named as `backup_vault_title` names it today
   - **Seed as a file, unprotected**
   - **Passphrase**, drawn only when a seed here has one.
   Ticking a row opens a sheet (a popup; full screen on `small`) titled
   "{row} · {place}", whose body is a multi-choice of the seeds loaded
   here, by fingerprint, and **Done**:
   - for words, SeedQR and file: which seeds this place keeps in that
     form; for words and SeedQR, a seed whose passphrase this place keeps
     is dimmed and cannot be ticked, so nothing already planned is moved;
   - for a vault: which seeds the vault holds, and under them, for each
     ticked seed that has a passphrase, "{fingerprint} with its
     passphrase". A vault is one vault wherever its stick is kept: the
     sheet edits the vault itself, and where another place also keeps
     its stick it reads "Also at {place}" above Done;
   - for the passphrase: which seeds' passphrases this place keeps; a
     seed whose words or SeedQR this place keeps is dimmed and cannot be
     ticked (the rule `set_places` keeps today).
   **Done** with no seed ticked leaves the row unticked, except a vault's
   row: a vault may hold only the description or shares (the person's
   choice, §15.6), so its stick stays at the place and the row reads that
   it holds nothing yet; a vault still holding nothing holds The plan
   card, with a line naming it, in place of Make the checklist. A ticked row
   shows what it holds under its label ("9a6a2580, 8c7e2500"); a press on
   the label opens its sheet again; a press on its box unticks it and
   clears what it held at this place (for a vault, its stick at this
   place; the vault keeps its seeds while another place keeps its stick).
   Under the list, the place's own line of fact: what someone who finds
   this place alone gets (`plan::alone` for `At::Place(p)`), as the map
   panel words it. Then **Continue**.
5. *(Replaced by §17 in Back up, batch F2a, and in the wallet's guided
   flow at any width, batch F2b.)* **Where the
   description goes**: a section per place, its name as the
   label, each a multi-choice of forms: **On paper**, **As a file**, and
   **In {vault}** for each vault whose stick that place keeps. With
   shares, a second list under each place's forms, **Shares here**: Share
   1 … Share n; the forms ticked hold every share ticked there. Then a
   last section, **Not a place**: **Into watch-only software**, and for
   a plan saved before this change, what its stick of files
   (`At::Files`) holds, a row each, which can be unticked.

## §17.11 Batches (from docs/BACKUP-FLOW.md)

- **F1, the model**: `At::Stick`, the new `What`s and forms, reading old
  plans, the write step's sticks and file names, the passphrase record
  Restore reading every form, and the read back after the write (§17.3,
  §17.4, §17.12 items 4–7, 10).
  Tests on the map, the checklist and the round trips.
- **F2, the board**: the cards, the list, drag and tap to place, forms
  popup, warnings, both plan cards, place names, the narrow column, the
  lock (§17.1, §17.2, §17.5, §17.6, §17.12 items 1–3, 8), DESIGN §4.17.
- **F3, the vault's show and save** (§17.7). Independent of F1 and F2.
- **F2d, + stick and a vault at no place** (§17.16), from the owner's
  test of F2c. Before F3.

F1 built 2026-10-16. F2d and F3 built 2026-10-19. F3's choices where
the spec says nothing: the SeedQR row is shown for English words only;
the save popup's warning is a line, and Save is the consent; a wallet
record's popup lists every software file (the record says no software),
the descriptor alone ticked.

**F2a built** (2026-10-16): the board in Back up on `wide` (§17.1–§17.3,
§17.5, §17.6, §17.12 items 2, 7, 8), DESIGN §4.17. Left for F2b: the
board in the wallet's guided flow (§17.12 item 1), the narrow column with
**Add** (item 3), and §15's strings no longer drawn. Choices made where
the spec says nothing:

- Back up opened on a wallet with no plan starts on Start from over an
  empty board (one place, nothing kept, `Answers::empty`); a preset fills
  it as before.
- The board is one card, **Places**, between Start from and the
  software; its Continue opens the next card.
- A press on a placed chip both selects it (a press on another zone moves
  it) and, where its zone has more than one form for it, opens its forms.
  A chip with one form there has no popup. A press on any other control
  lets the chip go.
- The software place's stick is the stick of public files (`At::Files`):
  it takes the whole description only, in the forms the Software card
  answers. Its × takes the software and that stick off together.
- A vault's last stick taken off a place leaves the vault on the plan,
  holding what it holds, and the plan held on "Vault n's stick needs a
  place"; a vault leaves the plan only by the × on its list chip.
- **+ vault** adds a vault holding nothing and kept at no place; the list
  shows the plan's vaults and the vaults set up before the keys, not one
  per seed.
- The description counts as in shares (`split`) while any share is kept
  anywhere; with Whole and shares both on paper, the checklist's Sheets
  item prints the shares (it makes one or the other).
- After the lock only the last place has a ×; the **+** card stays (a
  place may be added, §15.8).
- The warning mark goes on a passphrase and its seed's words kept at one
  place, and on a seed whose every copy is at one place when losing that
  place loses the wallet. The checks that stand are lines under the
  board: a seed or passphrase kept nowhere, `lost_why`, every secret at
  one place, a place that alone can spend (with two places or more), a
  vault with no stick, a vault holding nothing.
- The holds of NEW-WALLET §14.2 and §14.4 (a vault with no stick, a vault
  holding nothing) still hold The plan beside a seed kept nowhere; the
  owner kept them (2026-10-16), so §17.5's "only a seed kept nowhere"
  reads "a seed kept nowhere, or a vault with no stick or holding
  nothing".
- Timed per SPEED §7 (SPEED §6): the board's frames, a drag's move and
  the forms popup 5–7 ms at 1366×768 and 14–15 ms at 2560×1600 scale
  2; Back up's checklist as before.

**F2b built** (2026-10-16): the board in the wallet's guided flow's plan
card, on the flow's own plan, before and after the lock (§17.12 items 1
and 8); the narrow column with **Add** (item 3); §15's cards, their
`Action`s and handlers and their strings gone; §17.14's boxes, colours,
list column and checks. Choices made where the spec says nothing:

- Whose plan a board press edits follows the screen: the flow's on the
  flow's screen, Back up's (or, past the lock, its flow's) on Back up.
  The flow keeps its own board state (the chip held) with its screen
  state; nothing of it is kept across a lock.
- In the guided flow the board is the plan card's first card, then the
  software where it is asked, then The plan; the flow has no Start from,
  as before. After the lock, the board's paper and files act as in Back
  up after the lock (§17.12 item 8); every change still goes through the
  flow's own refusals (§15.8).
- Place names on the flow's board are typed only where §15.7 typed them
  (before the lock, a vault open, Back up open on the wallet); they are
  Back up's names, and a place removed there takes its name with it.
- **Add** is a button at the top of the board card's body in the narrow
  column; the sheet is the list as on `wide` (with the keys left off and
  **+ vault**), a head reading Add and a way out. A press on the
  software's chip there places the software place at once, as on
  `wide`. A placed chip's × and its forms work as on `wide`.
- The narrow column takes no drag at all: a press on a chip is a tap,
  and the page scrolls under the finger.
- A stick box's × takes the stick off with all it carries (its vault's
  stick and its files); after the lock only a stick that carries a vault
  and no file shows it.
- A chip is the card's surface edged in the border on every level, so it
  stands on paper, a stick box, a vault box and the list alike; Paper and
  the software's zones are the place box's surface edged in the border;
  a stick box is the inner fill with a heavier border edge; a vault box
  is the accent at low opacity with the accent's edge (DESIGN §4.17).
  These are the palette's own colours, so every theme draws them.
- The list column on the guided flow's screen is the side panel's width
  (`side_panel_w`), as on Back up; the flow's column narrows by it while
  the board is shown on `wide`.
- The Pi draws what the PC stick draws at its size: the narrow column.
  Nothing of §15's cards is kept for it (§17 left the Pi out; the Pi's
  screens are a later question for the owner).
- §15's model functions in `plan.rs` (`row_seeds`, the forms
  of Where the description goes) stay: the presets, old plans and the
  tests read plans through them.
- Timed per SPEED §7: at 1366×768 the board's frames with the list
  column, a drag's move and the forms popup 5–6.5 ms, at 2560×1600
  scale 2 14–16 ms; the narrow column at 800×600 (`compact`) 2 ms a
  frame, a scroll step and its frame 2 ms, the Add sheet 1.7 ms. The
  guided flow's board is not reached by the harness (it needs sticks
  served and a restart past the lock); it is the same view and drawing.

**F2c built** (2026-10-16): the board in rows and its colours (§17.15),
and one page colour across the app (`docs/DECISIONS.md` F39); DESIGN
§4.17 and §5. Choices made where the spec says nothing:

- A place's sticks and Paper share the row's width equally, **+ stick**
  `BOARD_ADD_STICK` (72 dp) between them. "A chip's width" is
  `BOARD_BOX_MIN`, 160 dp: a fingerprint's chip with its marks. Below it
  Paper keeps that width and the sticks wrap, as many to a line as fit;
  the last line of sticks takes what the row has over them.
- The software place is drawn as a place: its band (no line of fact),
  its one stick of public files as a stick box "Stick 1" with no × of
  its own and no **+ stick** (it keeps one stick), and Scanned in in
  Paper's colours.
- The glyphs are the closed icon set's: house (place), drive (stick, as
  a stick is drawn elsewhere), vault, and file for Paper (the set has no
  pencil).
- The line of fact is a tag on `SURFACE`, outlined and written in the
  chart's tone for it. In Bitcoin Orange Light that tone for a sealed
  place is the theme's own accent, 2.3:1 on its surface, as the chart
  already draws it; the board's own colours all pass.
- **+ place** is a dashed row in `MUTED`, label and edge, `BOARD_HEAD`
  high, as the mockup's grey.
- On a vault's band everything is in `VAULT_ON`: glyph, name, link and
  warning marks, ×, and the ring of the vault held. The lock mark on a
  sealed chip is `VAULT_EDGE`.
- A chip's edge is the mockup's (#AEB9C3 in the light theme, 1.6:1
  against the stick): a chip is not one of the table's boxes, and its
  surface, edge and shadow set it apart. Its faint shadow is a fill
  `BOARD_CHIP_DROP` below it, not a blur, which would cost a frame a
  blur per chip.
- On Back up the board takes its step's whole width, under the step's
  number as well as its title; in the guided flow it stays inside the
  plan card's own card, as F2b drew it.
- The theme tables key a colour by its dark value, so `VAULT_EDGE` and
  `VAULT_ON` are one step from `ACCENT` and `ON_ACCENT` in the dark
  theme (#83D8EE, #0D121A).
- F39 across the app: the page (wide, narrow and the held frame), a
  sheet and the gap a stretched scroll shows are `SURFACE`; a step is a
  section with a `LINE` rule above it and one under the last, on `wide`
  and in a small panel's list of steps; a card or tile on the page
  (`Ui::card`, Home's and Tools' tiles, the narrow menu, menu rows, the
  codes' rows, the vault pickers, the pill, the guided flow's plan card
  heads) keeps its fill and is edged in `BORDER` instead of `LINE`,
  since the fill no longer sets it apart. Wells drawn in `BG` (fields,
  the chart's nodes) are unchanged. The Starter, a program of its own
  with its own drawing, is not changed.
- Timed per SPEED §7: at 1366×768 the board's frames, a chip selected,
  a drag's move and the forms popup 5.3–6.6 ms (median), at 2560×1600
  scale 2 12.7–14 ms; the narrow column at 800×600 2.3 ms a frame and
  a scroll step, the Add sheet 1.9 ms. The checklist's frames 3.5–8.6 ms
  at 1366×768 (Vault 2's card 13 ms) and 8–14 ms at 2560×1600 (Vault
  2's 21 ms), Create's Keys 3–4.6 ms and 11 ms.

## §17.19 and §17.20: the build blocks' labels (from docs/BACKUP-FLOW.md)

Both blocks read, under the same text now headed "As built":

**Built (2026-10-19), with what the building decided:**

## §17.21 One way to start: item 10 and the build block (from docs/BACKUP-FLOW.md)

10. **Docs:** §17.6 (presets), §10.2 (the line under the question),
    `docs/DECISIONS.md` F53, and the preset lists in `docs/WALLETS.md`,
    `docs/FLOWS.md` and `docs/DESIGN.md`.

**Built (2026-10-20), with what the building decided:**

- `plan::Preset` keeps its four variants as plan builders only
  (`Answers::preset`): nothing in the UI offers them, and they are what
  a saved plan's `preset` line named and what the tests' plans by
  presses aim at. Since item 12 Paper only builds the same plan as The
  standard way. `Preset::name` and `Preset::index`, which only Start
  from and the plan's text used, are gone, and so is `Answers::preset`'s
  field: a `preset k` line read sets `custom` for any `k` but 0 and is
  not kept.
- The tests' shared helper is `testkit::backup_as(app, preset)`: from
  the board on screen (Back up's board card opened where another card
  is), it presses **Add watch-only software** (on the read-only board;
  the software chip's **Add to…** once customised), **Customize
  backup**, a seed chip's SeedQR toggle on paper, **+ new vault** for
  each vault not set up before the keys, and the ⋮ menu's **Add to…**
  for each vault's stick and for what each vault holds; then it asserts
  that the plan reached is the one the preset builds. It presses
  nothing where the board shows that plan already. The snapshot tool's
  tours use it too (`Tour::backup_as`). Tests that set a plan over two
  places for a single key (`preset_two_places`, plans saved before
  §17.19) still set the answers directly: no press reaches them now.
- **Add watch-only software** is `BoardPress::Software`, taken while the
  board is read-only; it toggles the software place as the software
  chip does (`Answers::toggle_software`), refused after the lock as that
  chip is.
- Create's Back up card reads "Opens on the standard way" while closed,
  in place of the presets' list. Back up this wallet is the card's
  primary; **Move the money**, where a wallet is made to move money to,
  stays as it was.
- The vault box's band is chosen by the drawing: "{} · sealed · made in
  the checklist" where it fits the band's room, "{} · new" where not.
  Made or not is `backup_vault_target(v)` on Back up's board, and on a
  wallet flow's own board a vault set up before the keys
  (`v < shape.vaults`). `board_view` is worked out once a frame, so a
  vault opened, made or locked shows at the next frame.
- Change the plan sets Back up's open card to the board with the column
  at its top (the board is the first card); a wallet flow's opens its
  board card with the column following it. Tests that opened the board
  by its card after Change the plan now find it open, and a test that
  looks at The plan's card after the board's lets the cards' motion run
  its course first.

## §9, §14 item 7 and §15.8: the lines naming the extra round (from docs/BACKUP-FLOW.md)

§9's Changing the plan bullet read:

- **Changing the plan part way**: §4.8. A lock during the extra round
  resumes on its card; a power-off ends it, the key's vaults sealed For
  the stick lost with the rest of `/run/faraday`.

§14 item 7 read:

7. **A key kept only on paper before an extra round** (§4.8): both
   offered, **Finish its paper copy first** chosen.

§15.8's first paragraph began:

(owner, 2026-10-15.) After **Lock and write** no wallet and no key is
loaded, so the plan changes there only in what the vault round (§4.8)
handles: **what goes into a vault and where a vault's sticks are kept.**

## §15.1 The cards, items 1 and 4 (from docs/BACKUP-FLOW.md)

1. **Start from**: the three presets, unchanged. Each fills every card
   below; every answer stays changeable.
4. **The wallet description**, for a wallet whose description splits
   (`Shape::splits`); not drawn otherwise. Two rows, one ticked:
   **Whole** and **In shares**, with **What is a share?** beside In
   shares (as today beside Its own share). With In shares, the slider
   **Keys left off each share** (0 to `min(m−1, n−1)`) and the shares
   table with the split's audit lines (`shares_card`, unchanged).
