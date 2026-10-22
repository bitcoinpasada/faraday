# Flows: how it was built

**Kind:** History
**Status:** Kept as written

Sections moved out of `docs/FLOWS.md` on 2026-10-22. Each keeps its
original number; the source keeps a one-line stub under the same heading.
The decisions still in force are stated in the flow sections of
`docs/FLOWS.md`; those replaced (2 by 22, 18 by 60, 46 by F62, the
presets of 20 and 40 by F53) are here only.

## Header (from docs/FLOWS.md)

Draft v0.1 · 2026-10-04 · every flow, where it starts, and
where it leaves the person. Not built.

## Decisions this pass made (from docs/FLOWS.md)

1. **One Files screen holds what came from the stick and what goes to
   it.** Files read from a stick had nowhere to be seen until a screen
   asked for one. The sidebar's status row reads "Files · 2 in · 1 out"
   and opens it. Every file copied in carries the action that uses it: a
   PSBT **Sign in Wallets**, `BOOTX64.EFI` **Sign in Secure Boot**, an
   unsigned APK **Sign with APK key** (`docs/APK-SIGNING.md`), an
   entries file **Import into a vault**, a vault file **Unlock**, a
   received file **Show as QR**. Both halves are emptied at power-off.
   The halves are named by direction (owner, 2026-10-10;
   `docs/history/SIMPLIFY.md` §4.1): **From the stick** and **For the stick**
   ("From the SD card", "For the SD card" on the Pi), on Files, the stick
   visit, the sheets and every button; no screen says "Inbox" or
   "Outbox", which remain the code's names only.
2. **A new vault opens as soon as it is created**, on its first
   passphrase's slot, because a new vault is empty and the next thing is
   putting something in it. Create vault lands on the Vaults overview with
   the new vault open and **Add contents** beside it. Replaced by
   decision 22 (2026-10-10).
3. **After any lock cycle, what came from the stick decides what is offered first.**
   Locking restarts the app, so the screen a person was on is gone. When
   unlocking again finds a new PSBT in Files, Home leads with "Sign
   unsigned.psbt" instead of the generic tiles.
4. **A stick visit can delete a vault file.** Changing a passphrase means
   making a new vault; the old file, still openable with the old
   passphrase, stays on the stick until deleted. The visit offers **Delete
   vault-2.ofv** (a hold) and states that deleting on flash media removes
   the file, not every copy of its bytes.
5. **Power off warns when something would be lost.** If files wait
   for the stick or a vault has unsaved changes, Power off lists them and needs a
   hold.
6. **Public and secret leave by different doors** (2026-10-06). Every file
   on its way out is public (anyone may read it: descriptors, wallet files,
   account keys, PSBTs, certificates), sealed (a vault or an encrypted
   backup) or secret (anyone holding it can use what is in it). Public
   files go For the stick as they are. A secret goes into the open vault by
   default and leaves only inside the vault file; the one way For the
   stick unprotected is the secret sheet, which names the file, what it
   is and who can use it, and enables **Put it for the stick unprotected**
   only once the person ticks that anyone with the stick can read it. For
   the stick lists the three apart, Public, Sealed and Unprotected secrets, a
   stick visit tags each row, and an unprotected secret is not ticked for
   writing until the person ticks it on the visit. A secret let out this
   way is marked as one whatever its text reads as (a node key, a child
   seed's words), and the mark is kept across a lock. A seed's words file
   is a secret by its kind. The code refuses a
   secret on the plain path For the stick, so a new flow cannot skip the sheet.
   The FROST carry (the next share's secret nonce) goes into
   the vault as a signing-round record (`docs/VAULT.md` §7, type 9),
   its public PSBT goes For the stick, the next device finds the round by
   the transaction when it opens that PSBT with the vault unlocked, and
   the round leaves the vault once it has signed. A wallet's seed, from
   the backup's checklist (2026-10-09), goes in as a key record
   (`kind::KEY`, as Vaults saves it), never a note; as a file it is the
   words or the SeedQR picture, one picked on the sheet, behind its own
   acknowledgement ("anyone who copies the stick or sees this file can
   spend these coins"), and never holds the BIP-39 passphrase. A seed's
   SeedQR never goes through the QR sheet's **PNG for the stick**, which
   is for public content. Make a new wallet ends in the same doors
   (2026-10-10, `docs/history/SIMPLIFY.md` §2.3): its last card, **Back up**,
   offers the plan's presets and opens the backup on the wallet just
   made, whose checklist puts the seeds and the wallet into a vault,
   lists the public files (descriptor, wallet file, multisig config,
   BSMS record, Bitcoin Core import, each key's account key and its BSMS
   record; each with **Show as QR** and **PNG** where a wallet reads it
   from a code, and **Remove** while it waits For the stick) and has the seeds copied by hand,
   each copy checked by its form (F29; built, except Keep the words
   only): a words copy by typing the whole seed back as words with the
   seed not on screen, a SeedQR copy by scanning the drawn grid (by typed
   words where there is no camera); **Keep the words only**, which turns
   a SeedQR copy into words by hand, is specified, not built.

7. **Words for keys** (2026-10-06). A **key** or **seed** is what signs:
   words, a SeedQR, a share of one. An account's public key with its
   origin, `[fingerprint/path]xpub`, is an **xpub**, and a wallet's
   description is a **descriptor** (or its split sheets). No file name,
   label or sentence calls an xpub or a descriptor a key; the test
   stick's files are named `seed-…` and `xpub-…` accordingly.

8. **What loads a key asks for the stick to be pulled** (2026-10-08).
   Keys still load only with no stick attached, but nothing that loads one
   is shown disabled for it: Add a key, Make a new key, Load this key, a
   Tools tile that adds or makes a key, opening a seed from Files or a
   backup. Pressed with a stick in, each opens a sheet, "Pull the stick to
   add a key" (or to make a key, …), the stick's label, and **Cancel**.
   The sheet closes the moment the last stick goes, and what was pressed
   carries on; Cancel closes it with nothing done.

9. **Every scan waits for the stick to be out** (2026-10-09). A camera
   frame may picture a seed, so the scanner opens only with no stick
   attached: Scan, Scan a SeedQR, Scan a key, Scan your copy and a vault
   entry's Scan,
   pressed with a stick in, open the same sheet, "Pull the stick to scan",
   and the camera turns on once the stick is out. A stick plugged in while
   the camera is on is held back, nothing read from it, and arrives as any
   stick does once the camera is off. Signing by QR alone needs no stick
   and is unchanged.

10. **Whatever needs a vault leads into making or unlocking one, and
    back** (owner, 2026-10-09). Where a flow needs an open vault and none
    is, its button is **Make a vault** when no vault file is in Files,
    **Unlock {name}** when one is, and **Unlock a vault** (the Vaults
    list) when several are; never a disabled button or a line to go to
    Vaults. Create a vault, opened this way, carries a chip at its head,
    "Then: {flow}", and its way back returns to the flow. Making the vault
    goes straight to Unlock with it picked: the passphrase is typed once
    more, which shows the person has it. Unlocking needs the stick out:
    with one in, the Pull sheet asks first ("Pull the stick to unlock").
    Once unlocked, the flow is back on the step it was on, with the
    vault's button live. The places: Tools' **GPG key**, **Secure Boot
    keys**, **APK signing key** and **KeePass export** ("Make a vault
    first" / "Unlock a vault first" on the tile, pressable; back to that category of the vault
    unlocked, or to Tools when the way back is taken); BIP-85, the
    Lightning node key and the Silent payments scan key (**Save into
    {vault}**); the secret sheet (the FROST carry, a seed from the backup's
    files item), which closes on the way, keeps its secret, and opens
    again on return; the backup's seeds and wallet vault items; Files (an entries file, an
    `.oskb`, a file added to a vault); Vault contents with none open; and
    Restore's seeds route, which offers Unlock only, as a new vault holds
    no seed. Going back anywhere on the way, or
    leaving for another page, ends it: a secret held for it is dropped,
    wiped, as Cancel on the sheet does.

    The one exception (F30, `docs/DEMO.md` §7.1): while
    `Session::demo()` is true, an action listed in DEMO §4 is drawn
    dimmed, with no reason, and does nothing. It is the only action in
    Faraday dimmed for a condition the person cannot change on the
    device. A Tools tile that leads to one of these screens is not
    dimmed: "Make a vault first" and "Unlock a vault first" still lead
    into the vault and back, and the screen reached shows its Make
    action dimmed.

11. **Home is the Start section, one lead and at most two secondaries**
    (owner, 2026-10-10; `docs/history/SIMPLIFY.md` §1 and §1.2). The three status
    cards (Session, Files, Sticks) are gone from Home: the sidebar's
    status block already carries them. Start is one full-width lead tile,
    the first job that applies (a spend under way, a PSBT in Files, an
    import waiting, what the last visit wrote with nothing loaded, a
    locked vault, a vault changed since it was written, otherwise Make a
    new wallet), then at
    most two secondary tiles (Wallets, Vaults; Stick visit when they
    leave room). Add a key is not on Home: it is reached from Wallets'
    empty state and from Tools.
12. **Wallets with something loaded is the list and the card alone**
    (owner, 2026-10-10; `docs/history/SIMPLIFY.md` §1.3). "What do you have?" and
    "What do you want to do?" are gone from the Wallets start page: every
    job they offered (Sign a transaction, Back up, Show wallet QR, Sign a
    message) is reached from the wallet card, opened from a wallet's row,
    and from Tools.
13. **Learn joins the sidebar, after Tools** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §1.4). The small panel's Home grid carries it in
    Add a key's old place.
14. **The network chooser moves to Settings** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §1.5). Home's network pill, and the sidebar's own
    badge, are off-mainnet badges that open nothing; choosing a network
    is a Settings row.
15. **The session strip** (owner, 2026-10-10; `docs/history/SIMPLIFY.md` §1.6).
    The sidebar (and the small Home's status card) carries one line,
    **Bring in · Open · Work · Write out**, the current stage in the
    accent, computed from what is attached, loaded and waiting; tapping
    it opens Files. **Write out** is files For the stick, or a vault
    changed since it was written (decision 26), with no stick attached.
16. **A step card with a default opens closed** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §2.1). Make a new wallet, New key and Sign a
    transaction open on the first card without one; a closed default
    card shows **Change** in place of the chevron.
17. **Make a new wallet shows two kinds** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §2.2). Kind offers Single key and Multisig, both
    native SegWit; the other eight kinds are behind **More kinds**.
18. **Add a key opens on words** (owner, 2026-10-10; `docs/history/SIMPLIFY.md`
    §2.4; replaced 2026-10-19 by F44, `docs/history/NEW-WALLET.md` §15: Add a
    key is New key's flow, opening on Start, with Type the words one of
    its choices). SLIP-39 shares, codex32 and Seed XOR are behind **Other
    forms**, unless a Tools tile for one of them opened the screen.
19. **Sign a transaction reviews the transaction before Check** (owner,
    2026-10-10; `docs/history/SIMPLIFY.md` §2.5). The Transaction card comes
    before Check, carries the txid as one of its rows, and Check stays
    closed once a wallet has been checked this power-on.
20. **Make a new wallet ends in the backup plan** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §2.3). The cards are Kind · Quorum · Keys · Check
    · Back up. Keys' Continue makes the wallet; Check shows the
    descriptor as a summary row that opens its code with the whole text
    under it; Back up is the plan's three presets, each opening the
    backup on the new wallet with that preset, its chip "Then: Wallets",
    its way back the wallet's card. The vault, public-files and paper
    cards are gone: they are the checklist's items. Since F55
    (`docs/history/NEW-WALLET.md` §17, 2026-10-20) Kind carries an **Account**
    − n + stepper for every kind whose path has one, starting at 0, and
    Check shows a **Path** row under the descriptor, with **Change**
    that takes the made wallet back and opens Kind on its Account.
21. **Create a vault offers two sizes** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §3.1 and §3.2). Its cards are Size and Name and
    passphrases; Size is **PCs only** (512 MiB) or **PCs and a Raspberry
    Pi** (64 MiB, the default), both 3 passes, so the flow opens on Name
    and passphrases. **Customise** opens the three cards it stands for
    (where it opens, unlock cost, space per passphrase) in its place. The
    Name field's caption says the name is what it holds, for anyone who
    sees the stick.
22. **A new vault is opened once more, then shows its next steps**
    (owner, 2026-10-10; `docs/history/SIMPLIFY.md` §3.3). This replaces decision
    2: **Create vault** always goes to Unlock with the new vault picked,
    under "Type the passphrase once more to open it"; unlocked, it opens
    on Vault contents, which says "Nothing in it yet" with every add
    action as a button (decision 58) and **Write it to a stick** (Files).
    Vault contents lists only the kinds a slot holds something of, then
    a row per add action.
23. **A locked vault says what it held** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §3.4, `docs/VAULT.md` §11). What a vault held when
    last seen open is kept across a lock until power-off: its Vaults row
    goes by its name and Home's lead is "Unlock {name}", both with the
    line "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen 14:02"; with
    nothing remembered, "Unlock to see what it holds".
24. **For the stick fills itself** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §4.2). A public file a flow makes goes For the
    stick the moment it is made: the signed PSBT and the finished
    transaction at Finish, the PSBT with this pass's signatures while
    more are needed, the backup checklist's template, sheet or shares
    and public files when the checklist is made (made again when the
    word count or the shares change), a GPG public key or signature (a
    revocation certificate is a secret and goes through the secret sheet,
    F32), a signed message, a sealed vault at lock. No button puts a
    file out: where **To the Outbox** stood, the row reads "For the
    stick" with **Remove**, and after a Remove the button makes it again.
    The secret sheet stays the one way a secret goes For the stick.
25. **A write leaves a receipt** (owner, 2026-10-10; `docs/history/SIMPLIFY.md`
    §4.3). When a stick visit's write is answered, what it sent leaves
    For the stick and becomes the receipt, shown under For the stick as
    "Written to {label} at {time}" with a row per file, "verified" or why
    not. It is kept across a lock until power-off and replaced by the
    next write; nothing on it is written again. A vault written stays in
    Files as the stick now holds it. With nothing loaded, Home leads with
    "Written to {label}" and the file names, opening Files; a backup
    checklist item counts a file on the receipt as done.
26. **A vault's currency** (owner, 2026-10-10; `docs/history/SIMPLIFY.md` §3.5).
    Each vault file reads one of three states, computed from the
    receipt: "Never written" (sealed For the stick, on no receipt), "On
    {label} · current" (the receipt wrote the bytes it has now),
    "Changed since written" (open with unsaved changes, or sealed bytes
    that differ). A vault copied in from a stick and not changed reads "On
    {label} · current" for the stick it came from, remembered across a
    lock, or "Unchanged" when that stick is not known. It leads the vault's Vaults row; a changed vault makes
    Home lead with "Write {name} to a stick" and the session strip read
    Write out.
27. **The boot sheet is a picker** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §4.4; changed 2026-10-20 by F60). With the stick in
    or out, the sheet reads "Read from {label}", one line of counts ("1
    vault · 2 PSBTs · 4 other files") and every vault and file read, each
    with a checkbox, all ticked, a seed's file too, under **Unselect all**
    (**Select all** once none is); the settings file is not listed.
    **Import**, disabled while nothing is ticked, brings in what is ticked: the files From the
    stick with their wallets and keys, the vaults into Files, locked; then
    each vault in turn takes its passphrase on the sheet, once the stick is
    pulled, with **Unlock** and **Cancel** (left locked in Files). An
    unlock shows the vault's Load sheet (F62) before the next vault; it
    loads what is ticked there. **Later** brings in nothing,
    vaults included; Home leads with "Import from {label}", or after
    Import "Unlock {name}", which opens the sheet again. Pulling the stick
    before a choice changes nothing; with only the settings file there is
    no sheet.
28. **The lock, write-out and power-off sheets lead with what is kept**
    (owner, 2026-10-10; `docs/history/SIMPLIFY.md` §4.5). Each lists **Kept**
    first: one line per vault with its currency, and one for the files
    For the stick; then **Wiped from memory**: one line per seed (a key
    with no name of its own as "Seed {fingerprint}") and wallet, with
    where else it is ("in {vault}", "copy checked", "in
    Files" for a wallet whose file is there) or "nowhere else", that line
    alone in the danger tone and listed first; then **Then**. At power-off
    a vault never written or changed since, and the files For the stick,
    are under Wiped from memory, in the danger tone. The lock sheet's
    title stays.
29. **Everything comes in by default** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §4.6). A stick visit ticks every file it can read
    to come in, but a seed's by its name (words, SeedQR, BIP-85), one over
    18 MiB, and one already From the stick by name and size (F26,
    specified, not built: over 64 MiB it asks first, whole or only for
    signing; past the memory a whole import may take, `docs/LARGE-FILES.md`
    §4, it is unticked; one whose name §9 there refuses is inert). **Select
    all** on the visit's right column is gone; **Unselect all** takes its
    place while anything is ticked.
30. **The visit says where it goes next** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §4.7). The stick visit's title carries the chip
    "Then: Unlock" when a vault was sealed for it (For the stick, or on
    the last write's receipt), "Then: {flow}" when the stick came in
    during one, else "Then: Home"; pulling the stick goes there.
31. **Backups: where each wallet's backup is** (owner, 2026-10-10;
    `docs/history/SIMPLIFY.md` §5). A Backups screen lists every wallet known
    and its backup map, each place with what this device saw of it; the
    wallet card carries the same as one line, and Home's second tile is
    Backups once a wallet is known (below, "Backups"). A paper place is
    checked only once a check of that copy matched here: one check, one
    copy.
32. **Light is the default theme** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §1.1). A device with no settings saved starts
    in `Theme::Light`, not Tokyo Night.
33. **No bar for a test network** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §1.2). The 4-point `WARN` fill along the top on
    a test network is gone, in the main area and in the sidebar; the
    pill alone marks it.
34. **Make a new wallet and New key open on their first step** (owner,
    2026-10-10; `docs/history/NEW-WALLET.md` §2.1, reversing decision 16 for
    these two flows only; Sign, Back up and Create a vault keep it).
    Create opens with Kind open, whatever opened it — Wallets, Home, or
    a Tools tile with its kind already ticked; Kind's Continue closes it
    as done and opens the next card. New key opens with Length open
    (since decision 60, Start first, then Length);
    Length gains a Continue that takes the count shown and opens
    Randomness (pressing a count does so already); Randomness's
    Continue opens the entry card as now. A card with a default and not
    open still shows its value and **Change**.
35. **A key in one slot is offered in no other** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §2.2). On the Keys card, a key loaded here
    that fills one slot, or a cosigner's file whose key fills one slot,
    is not offered for any other slot; clearing the slot offers it
    again.
36. **The Keys card's foot button opens the next empty slot** (owner,
    2026-10-10; `docs/history/NEW-WALLET.md` §2.3). While a slot is empty, the
    button at the foot of Keys reads **New key for Key N** (**Share N**
    on a threshold wallet), N the first empty slot, and opens New key
    for it; once no slot is empty it reads **Continue**, as now.
37. **Check names whose addresses they are** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §2.4). On Create's Check, a section label **The
    wallet's first addresses** sits above the address rows, so that a
    multisig's three rows do not read as one address per key.
38. **New key: the check on the entry card, Lock in, then back up**
    (owner, 2026-10-10; `docs/history/NEW-WALLET.md` §3.1–§3.5, decisions 4–6).
    New key's BIP-39 cards are Length · Randomness · *entry* · **Key**
    for a Create slot, and Length · Randomness · *entry* · Key · Words ·
    Quiz on its own (Tools, Seeds); SLIP-39 shares keep Length ·
    Randomness · *entry* · Check · Words · Quiz. For BIP-39 the Check
    card is gone: once the last entry is in, the entry card shows the
    counts and the verdict ("Fair: no cautions", or each caution in
    `WARN` with **Roll again**, or the source's word, beside the foot
    button; a caution is not a refusal); this device's generator, the
    camera and a mix as a whole show none. The foot button appears with
    the last entry and reads **Continue** where the words showed as they
    came in, else **Make the words**; for a Create slot a line under the
    words so far says Back up shows them again. **Key** shows the words,
    for a Create slot **Passphrase** and **Passphrase again** (masked,
    empty is none; unequal ones lock nothing in), and **Lock in**, which
    adds the key under the self-test gate, fills the slot, and on a
    single-key Create makes the wallet and marks Keys and Check done.
    After it the card shows **Locked in**, the fingerprint and, for a
    single-key Create, the descriptor row and the first address; for a
    slot of a wallet of several keys, or on its own, the key text and no
    address. Length, Randomness and the entry card then open to be read
    only. Key's **Continue** returns to Create (on Back up for a single
    key, on Keys otherwise) or, on its own, goes on to Words and the
    Quiz, whose last button reads **Done**. A key added with a
    passphrase is a seed with a passphrase to Back up's plan, which asks
    where it goes.
39. **A passphrase for a key chosen in Create** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §3.6). On the Keys card, a slot filled by a
    key loaded here from BIP-39 words with no passphrase has **Add a
    passphrase** (beside Show xpub QR on a multisig, on the slot's own
    row on a single key). It opens **Passphrase**, **Passphrase again**
    and **Lock in** under the slot; Lock in adds the key the words and
    that passphrase make, labelled as the first with " · passphrase",
    puts it in the slot with a **Locked in** tag, and leaves the first
    key loaded. A key with a passphrase already, a cosigner's xpub, a
    share, and a key from SLIP-39, codex32 or a bare seed have no such
    button.
40. **Back up leads with the whole wallet sheet; shares come second**
    (owner, 2026-10-10; `docs/history/NEW-WALLET.md` §4, decision 8). On the
    plan's Places question a multisig's **Each place keeps** lists **The
    whole wallet sheet**, ticked by default, then **Its own share**,
    with **What is a share?** beside it opening Learn on *Shares of a
    wallet description*; closing Learn leaves the plan on Places. With
    Its own share ticked, **Keys left off each share** is a slider from
    0 to m − 1, then the shares' fingerprints and the split audit. The
    checklist's Shares card has the same slider in place of its number
    buttons. The page is listed among a multisig backup's Learn pages.
41. **Back up puts one seed in each vault** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §5, decision 9). The seeds' row reads **Into
    vaults**; under it a section per vault, **Vault 1**, **Vault 2** …,
    each a multi-choice of the seeds here, vault *v* holding the *v*th
    seed by default; a seed may be ticked into more than one. Places
    asks **Vault 1's stick** per vault made, each stick by default at a
    place that adds no second key. The map names each vault ("Vault 1 ·
    9a6a2580"). The checklist has one item per vault, **Vault 1:
    9a6a2580**, which makes a vault and saves its seeds and the wallet;
    an open vault holding another vault's seed is not offered there
    (**Lock it and make a new vault**). The separate wallet-into-the-
    vault item is gone. A stick visit with several vault files waiting
    ticks one. Plans saved with one vault still load as one vault.
42. **An open vault shows each wallet the other way up** (owner,
    2026-10-10; `docs/history/NEW-WALLET.md` §7.2, decision 11). Vault contents
    unlocked opens on Wallets where the vault holds one. A wallet's item
    shows the plan's three check lines, then the wallet's chart (DESIGN
    §4.16) read from the backup up: the places, vaults, files and
    software of the plan this vault keeps for it, each with what it
    holds and what it gives whoever finds it alone; the keys under them,
    a seed the vault keeps reading "In this vault"; the wallet at the
    foot. Then its descriptor and **Open in Wallets**. Each node opens
    its sheet (decision 44); an action there loads the wallet, and its
    seeds the vault keeps where keys may load, first. While a wallet's
    item is shown the kinds and items columns narrow, so the chart has
    room for five nodes on a line at the desktop window's size. A
    wallet with no plan in this vault: its wallet and key nodes, and "No
    backup plan in this vault". On a small panel the item's page stacks
    the same nodes in that order, with no lines.
43. **From the vault to a spend** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §11, decision 13). The boot sheet's Unlock that
    loads wallets lands on Wallets, on the first loaded wallet with a key
    here (else the first loaded), with the toast "Savings loaded from
    vault.ofv" or "12 wallets loaded from vault.ofv" (the stick's label
    when the stick's own files brought some). The Vaults list's Unlock
    loads too (decision 46); an unlock from inside a flow returns to it.
    The wallet card's first button, primary,
    is **Spend from this wallet**; **Sign the PSBT from the stick** with
    a PSBT for this wallet in Files, which opens its review at once;
    **Carry on the spend · Signatures 1 of 2** while this wallet's spend
    is under way, as Home's lead says too. It opens the Spend tab on the
    wallet, past What do you have? (`docs/history/SCAN-TO-SIGN.md` §4): **Load the
    wallet in Sparrow** (the wallet QR, receive address 0, **It is
    loaded**; done already for a wallet checked this power-on) with **I
    have the PSBT from Sparrow ready: Scan it** over it, then **Write the
    payment in Sparrow** (**Scan the PSBT**, **On a stick**, this
    wallet's PSBTs in Files), then the spend's steps unchanged. Who signs,
    with fewer keys here than sign, offers **Scan a key**, its Unlock
    buttons and **Collect a signature**, which opens The other
    signatures (decision 57). On a small panel the card's spend button sits
    under the signatures box, and the shortcut is its line and **Scan
    it**.
44. **The chart's sheets** (owner, 2026-10-10; `docs/history/NEW-WALLET.md`
    §9.1–§9.4, decision 12, first of two batches). A press on a node of
    the chart, on the card or in an open vault, or on a line of what a
    backup node holds, opens a sheet (DESIGN §4.16): its title, what it
    is and where (Shape and Checksum; Where, Its seed is at; Stick, Found
    alone, Seen here; Kept at), then its actions as menu rows, each
    leading into the flow that does it, and Close (Back on a small
    panel, where the sheet is the page). Opening a sheet changes nothing
    and puts nothing For the stick; nor does opening the backup's
    checklist from the chart, whose items each offer what they make.
    The wallet: **Open in Wallets** (in a vault), **Show wallet QR**,
    **Public files** (the checklist's item), **Print the wallet sheet
    again** (or the shares), **Rename the wallet** (its card's name
    field), **Change the plan** (Back up's questions, with the plan the
    chart shows). A key held here, or kept in the open vault: **Show
    xpub QR**, **Xpub file**, **Explore this key**, **Where it is** (its
    lines bold, the rest dimmed, until the next screen or sheet); **Make
    another backup of this seed**: **By hand** (the checklist's copy, with
    Seed XOR and codex32), **Into Vault n**, **As a file**, each where the
    plan has that item; **Check a copy of this seed**: a row for each check
    its copies ask for (`docs/history/NEW-WALLET.md` §14.7): **Type my copy**
    for a words copy, which opens the typed check at the checklist's
    copy item, and **Scan my copy** for a SeedQR copy with a camera,
    which leaves the chart under the camera. A key
    not here: **Load this key** with its way in, where the quorum needs
    it. A place: its lines, each opening its own sheet; **Rename** (the
    plan's place names, with a vault open; else the vault way, "Open a
    vault to keep this"); **What goes in its envelope**. A vault: the
    vault way or **Open the vault**, **Save its seeds into it**, **Copy
    to another stick** (the stick visit). Files: **Write the seed files
    again**, **Write the public files again**. Software: **Show the
    descriptor QR again**, **Public files for this software**. Lines:
    words, **Type my copy**; SeedQR, **Scan my copy** (**Type my copy**
    with no camera); **Make another copy**; the wallet sheet, **Print it again**, **As a QR picture**,
    **As text**; a share, **Print the shares again**, **Keys left off
    each share** (the checklist's shares); a vault's stick, the vault way
    and **Copy the vault to another stick**; a seed file, **Write it
    again**; the description, **Show wallet QR**. A flow on Back up comes
    back to the card, or to the vault opened from. What only edits the
    plan, marks and dates, lost and exposed, and a holder's name came
    with the second batch (decision 49).
45. **The backup comes before spending** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §14, `docs/DECISIONS.md` F6). Back up's
    checklist: an item's foot button is its next action (a vault item's
    **Make a vault** or the way to one, then one **Save into {vault}**
    that saves every seed, passphrase and the description the plan puts
    there, then **Continue**; a copy's **Check my copy**, which types the
    words back for a words copy and scans a SeedQR copy; files' **Make
    it**), and the items after one not done do not open. Places waits
    while a vault's stick has no place ("Vault 1's stick needs a
    place"); a plan that keeps no copy of a key made here is not made
    into a checklist ("Seed {fp} is kept nowhere"); **Change the plan**
    stays under every item, done work the new plan drops stays on the
    map. A key made here signs once its checklist is complete: the
    wallet card's first button is **Finish the backup first**, Who signs
    lists it **Waiting for its backup** with **Finish the backup**, and
    the card's address, Create's Check, the Key card's address and the
    chart's wallet node say "Back up before you receive". The done card,
    the wallet card and the open vault lead with the plan's three checks
    as a strip above the chart.
46. **The Vaults list's Unlock loads too** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §11.1a). Reversing decision 43's own rule for
    it: an Unlock from the Vaults list loads the wallets the vault
    holds, with their keys where the vault holds them, and lands on
    Wallets with the same toast and wallet chosen as the boot sheet's
    Unlock (decision 43). A vault that holds no wallet lands where it
    does today; an unlock begun from inside a flow still returns to it.
47. **Mouse back and forward, selecting text, the default count**
    (owner, 2026-10-10; `docs/history/NEW-WALLET.md` §13). On the desktop the
    mouse's back button does what the screen's back link does (Escape
    where there is none, a sheet's Cancel over a sheet), and its forward
    button opens again the screen back left, as it was left, until
    anything else is opened. In a line of typing — typed text, a masked
    passphrase, New key's box of rolls or flips — a press puts the caret
    where it lands, a drag selects what it goes over, Shift with a press
    or Left and Right extends the selection, and a second press selects
    all; Backspace and Delete take the selection, a character replaces
    it. In dots a selection is by position; nothing is copied out of a
    masked field. Rolls or flips deleted in the box leave the key's
    entropy with them. New key's Length shows its default count chosen
    from the start. An empty BIP-39 passphrase field (New key's Key card,
    Create's Add a passphrase, Add a key) shows a **No passphrase** pill
    at its start, with the caret after it while focused; a vault's
    passphrase field has none.
48. **Restore: what it is, then its description or its seeds** (owner,
    2026-10-10; `docs/history/NEW-WALLET.md` §12). Restore opens on **Kind**
    (Single key · native SegWit ticked, Multisig · native SegWit, More
    kinds), with the shortcut **I have the wallet description: Scan it**
    and **From Files** above it; then **Quorum** for a multisig (M of N
    on the seeds piece's two sliders, 2 of 3 to start), **Description**
    (Scan it, From a file in Files, the shares, the stick or a vault, or
    **I do not have it**), **Seeds** (**Keys** since decision 53),
    **Check** and **Done**. The
    transaction card goes: a spend is the Spend tab's. A description
    read on Kind or Description fills Kind and Quorum and opens Seeds.
    Seeds has a slot per key; the first empty one has **Passphrase** and
    **Passphrase again** (optional, the No passphrase pill), then **Type
    the words**, **Scan a SeedQR**, a seed an open vault holds, a key
    already loaded, and on a multisig **Its xpub** (no description) or
    **Not here** (with one). The passphrase goes with the seed entered
    from that slot. With the description known each seed is matched to
    its keys as it is entered: a match reads "Key 2 of the wallet"; any
    other reads "Not a key of this wallet: check the words or the
    passphrase" and is not loaded. **Make the wallet** once every slot
    is filled; Check shows the descriptor row and the first addresses;
    Done offers **Back up this wallet**, the wallet card's spend button
    (**Spend from this wallet**) and **Wallets**. A restored key is not
    held for its backup (decision 45). Since `docs/history/NEW-WALLET.md` §12.3
    (2026-10-19) Restore ends on Done whatever the vault question's
    answer, its choices all secondary: **Spend from this wallet**,
    **Back up this wallet**, **Another wallet from these seeds** (New
    wallet from this one, §16) and **Wallets**. Vaults set up before the
    seeds wait in the wallet's flow: Back up resumes it at the plan with
    them, and until a plan is made the wallet's card and Backups read
    **Continue the backup**.
49. **The chart changes the plan** (owner, 2026-10-10;
    `docs/history/NEW-WALLET.md` §9.5–§9.7, §9's second batch). A change to
    where something is kept is asked on a sheet first: its title says
    the change ("Move Vault 1 stick to Place 1"), the plan's three check
    lines read before → after, a line made worse in `WARN`, and
    **Confirm** makes it; Close leaves the plan as it was. The changes:
    **Move** (a line at a place, to another place or **A new place**);
    **Another copy** (words or SeedQR by hand, a passphrase, the wallet
    sheet, a vault's stick, at a place not holding it, a passphrase never
    with its own words); **Add here** (a place: what is held here and is
    not there yet); **Into Vault n** and the vault's **Save more into
    it** (a seed held here, with its passphrase where it was loaded with
    one, the description); **As a file**; **I destroyed this copy**, and
    **Remove from the plan** for a marked line or the stick of files
    ("Destroy what goes off the plan by hand"); **Remove the place**,
    which first asks where each thing in it goes (another place, or
    **Nowhere**) and is pressable once each has one. Once confirmed, the
    flow that makes what was added follows: the copy by hand with its
    check, the vault's item, the seed files, the sheet For the stick,
    the stick visit for a vault's stick ("Vault 1 · copy", its own line,
    counted by the check), a passphrase shown to copy (**Show it**) and
    then **Check my copy**. From the first change the map is the plan:
    the backup's record and the vault's plan record keep it, the chart
    and the checklist read it, and it loads back the same on another
    power-on. **Change the plan** (the wallet's sheet, the checklist's
    foot) then asks first, "Your changes on the chart are replaced", and
    the questions start from the answers. In the vault alone, never in
    the kept state: the date each thing of a place was checked (a
    place's **Mark checked**; a passphrase checked), **Lost** and
    **Exposed** (a line, or **Mark lost**, **Mark exposed** on a place:
    struck through on the chart until taken off the plan), a key not
    held here's **Name its holder** and **Mark checked** (the date its
    holder confirmed it), and a vault's **Rename**; with no vault open
    these rows read "Open a vault to keep this" and lead to one. A mark
    says what follows, in order: "Someone can now spend" (or "could …
    with a vault's passphrase") with **Move the money** ("New
    wallet": Make a new wallet, whose last card then offers **Move the
    money from {wallet}**,
    the spend from the old wallet with "Every coin to {new wallet}" and
    its first address on Write the payment); "The wallet can no longer
    be rebuilt from what is left" with **Back up again now**; "Someone
    can see the balance"; "Nothing lost that is not elsewhere" with
    **Make a replacement copy**. **Check it** on a passphrase line (or
    the key's **Its passphrase**): typed into a masked field, the key it
    makes with the seed here compared by fingerprint, the typed text
    wiped at once; "Matches: it makes key {fp}" or "Does not match", and
    a match dates the line in the vault. Also on the sheets: the
    wallet's **Check addresses** (the descriptor row and the first
    addresses, as Create's Check); a key not held here's **Show its
    xpub**, from the wallet's descriptor; a place's **The envelope
    list** (a PDF For the stick, places by number, never by the names the
    vault keeps); a vault stick's **Write it out again** while the vault
    has changed since it was written.
50. **Home's four ways in; Back goes where a flow was opened from**
    (owner, 2026-10-11). Home's Start is a lead tile, full width, only
    while something is under way or waiting (`docs/history/SIMPLIFY.md` §1.2
    rules 1 to 6), then half-width tiles, two to a row: **Make a new
    wallet** (Create), **Restore an existing wallet** (Restore), **Spend
    from a backup, step by step** (the Spend tab) and **Vaults**, which
    with no vault in Files or open reads **Create a vault** and opens
    it; then **Stick visit** while a stick is attached and nothing is
    held, and **Backups** once a wallet is known. Create, Restore, the
    Spend tab and Create a vault opened from Home have **Home** as their
    way back, and from Wallets, **Wallets**. The Spend tab lights
    **Wallets** in the sidebar, and the Wallets page offers **Spend from
    a backup, step by step** with wallets loaded as well, so a spend
    left for another tab is one press away. The Wallets page no longer
    says a stick with a vault brings its wallets and keys. Restore's
    Description card leads, for a single key, with **I have the seed**
    (on to Seeds), then **Scan descriptor** and the files; a multisig
    keeps **I do not have it**. Wallets' ways in are Make a new wallet,
    Restore an existing wallet, Spend from a backup, and last **Add a
    key**, "Keys first, wallets later".
51. **Create a vault: one memory figure, This computer chosen**
    (owner, 2026-10-11). The size rows, the cost card and the summary
    panel state the vault's unlock memory, the Argon2id memory (64 MiB
    for PCs and a Raspberry Pi); the panel's row is **Unlock memory**.
    What this computer has free is shown only where it falls short.
    Customise's Where card has **This computer** ticked by default and
    untickable, for a vault that asks less than this computer can give;
    its row reads the most unlock memory it gives a vault — the largest
    custom memory that fits in its free memory beside the app's 100
    MiB, at most 4 GiB — with its free memory in parentheses: "1 GiB
    (1.2 GiB free)". Another machine gives half its memory. The cost
    suggested is the largest preset the weakest chosen machine gives.
52. **A SeedQR asks for its passphrase after it is read** (owner,
    2026-10-11). Wherever a SeedQR (or a seed's words in a code) is
    scanned — Add a key, Home's Scan, a slot of Create or Restore, the
    Spend tab's words — the camera closes and the sheet **SeedQR read**
    shows the key it makes **Without a passphrase**, with **Use without a
    passphrase**, **Add a passphrase** and Cancel. Add a passphrase shows
    **Passphrase** and **Passphrase again**, the key they make once they
    match (**With this passphrase**), and **Use with this passphrase**;
    two that differ load nothing. With Restore's description known, a key
    that is not one of its keys is said on the sheet, which stays for the
    passphrase to be typed again. The words wait in memory, never shown,
    and Cancel or a press beside the sheet wipes them. Restore's Seeds
    slot loses its passphrase fields: its ways in (Type the words, Scan a
    SeedQR, a vault's seed or a key already loaded, Its xpub or Not
    here) stand over the slot they fill, and typed words take their
    passphrase on Add a key. A key already loaded, or a vault's seed,
    taken into a slot of Restore or the Spend tab with words and no
    passphrase opens the same sheet: a vault may keep a seed apart from
    its passphrase (decision 49).
53. **Keys and seeds** (owner, 2026-10-11). What is loaded and signs is a
    **key**, named by its fingerprint; a **seed** is the words that make
    it. The lists of what is loaded say Keys: the sidebar's **Keys**,
    Files' **Keys with no wallet**, the Spend tab's **Keys with no wallet
    loaded**, Restore's **Keys** card, counts such as "2 of 3 keys here"
    and **Load 2 keys**. The words stay seeds: Scan a SeedQR, I have the
    seed, "Key 1 seed", the seed sheet. A key loaded with a passphrase is
    marked **passphrase** at the end of its sidebar row.
54. **Delete files from a stick** (owner, 2026-10-11). One way in: the
    stick visit's stick card has **Delete files**, a quiet button over
    its Import buttons. Its sheet, "Delete files from {stick}", has the
    band **Danger zone: what is deleted here cannot be undone**, then
    three choices: **Chosen files** (the stick's files, ticked on a list,
    each zeroed, read back and removed with its name), **The free space**
    and **The whole partition** (decision 55); then that only the files
    partition is written and why, that Faraday only ever writes there so
    zeroing it reaches everything Faraday wrote, what zeros do not reach
    (no zeroing is certain: a stick's or SD card's controller can keep
    old copies of blocks, and only destroying the stick is; a seed or other
    secret ever saved to the stick unencrypted is surely removed only by
    destroying the stick), and one button naming the action ("Delete 2
    files with zeros", "Zero the free space", "Zero the whole
    partition") or Cancel. The disk process does the work; a file's
    answer is a line on the visit ("{file} deleted, zeros over it
    first", or why not). A computer's own files (dot-files and
    Windows' own, `docs/STARTER.md` §4.4) are never offered or deleted
    one by one, and the boot partition is never touched.
55. **The free space; the whole partition** (owner, 2026-10-11). On
    the Delete files sheet (decision 54): each says how much it writes
    and about how long on a typical stick, and the whole partition
    offers **One pass** or **Two passes**. The free space is zeroed in steps of about 8 MB, each asked for on
    the tick after the last one's answer, read back, the first step also
    clearing deleted directory entries and kept files' last clusters
    past their ends. The whole partition is first made an empty volume
    with its layout and label, then zeroed the same way, once per pass.
    A sheet shows the share done and the time left, with **Stop**; Stop,
    a failure or a pull ends it with a line on the visit. A whole-device
    wipe is not offered: it would need the whole device, outside the
    stick process's boundary.
    Files' **From the stick** and **For the stick** scroll each on its
    own, each with its own scrollbar, under the pointer.
56. **The clock: a reading before 2020 is not known, and a time set by
    hand is an offset, never the computer's own clock** (owner and
    architect, 2026-10-11; `docs/TOTP.md` §1–§3). The Pi has no
    real-time clock and starts at 1970; the stick reads a PC's hardware
    clock, which a PC running Windows keeps in local time. Settings
    gains a **Time** section: **Now**, the local date and time or **Not
    known**; **Time zone**, a fixed offset from UTC stepped by 15
    minutes, kept in `faraday-settings.txt`; and **Set the time**, which
    opens a new screen to type a date and a 24-hour time, or **Keep
    this time** when the clock is already known. Setting it stores an
    offset against the shell's own clock, kept across a lock, lost at
    power-off; Faraday never calls `settimeofday` or `hwclock`. Every
    date or time shown to a person is local; what goes into a format
    (a GPG signature's time, Secure Boot's) stays UTC.
57. **Scan a key; Can spend** (owner, 2026-10-11;
    `docs/history/SCAN-TO-SIGN.md` §2–§3). Where a wallet is short of keys, one
    **Scan a key** opens the camera at once, titled "Scan a key for
    {wallet}", a line per missing key ("Key 2 · 5d388376", and " ·
    {place}" where the backup's plan keeps that seed), with **Type the
    words instead** and Cancel. A seed whose key with no passphrase is a
    missing key loads at once, with no passphrase question: the camera
    stays on with "Key 2 added · 1 more" while more are needed, else
    closes with the toast "Key 2 added · Can spend". A key already here
    says "Key 1 · already here"; a seed of no key of the wallet closes
    the camera and opens the passphrase sheet (decision 52) with "{fp}
    is not one of {wallet}'s keys", which takes only a passphrase giving
    a missing key and refuses another with the same line; anything else
    is named as Scan a SeedQR names it. **Type the words instead** opens
    "Add a key for {wallet}", which refuses words of no missing key with
    "These words give {fp}, not a key of {wallet}". A key loads under
    its holder's name where the plan keeps one, else none; a name that
    is the fingerprint is never drawn beside it. It is on the wallet
    card's signatures bar in place of "Add 1 more key or collect 1 more
    signature" (full width under the line on a small panel), on the
    Wallets column as **Scan a key for {wallet}** in place of **Add a
    key** while the wallet shown is short, and on Who signs (the Spend
    tab's and Wallets' Sign step). The per-key **Add its key** and **Type
    its words** go. A wallet with enough keys here to sign shows a green
    **Can spend** on the card's bar (in place of "Nothing missing"), on
    Who signs beside "{here} of {needed} signatures here", and at the
    right of its Wallets row, whose sub-line is then its shape alone;
    the small panel's list and Home's say "Can spend · 2 of 2". Short,
    Who signs says "1 of 2 signatures here · 1 more needed" and a key
    not here **Not here**; its Scan a key, Unlock and Collect a signature
    show only while short, or, for a miniscript or tree wallet, until the
    spend is signed.
58. **Every way to add, the word count, the time, a taken name** (owner,
    2026-10-11; HANDOFF item 68). An open vault shows every add action
    (Save a key, Save a wallet, Add an entry, Add a note, Make a key,
    Make keys, Make an APK key) with no press first: an empty vault as
    buttons under "Nothing in it yet", then **Write it to a stick**; one with contents
    as a row each under its kinds, the Add icon in the accent colour.
    **Put a wallet in it** and the **Add…** row go. Add a key's words
    take a count first, **12 · 15 · 18 · 21 · 24** above the boxes, 12
    chosen; typing past the boxes still grows them to 24; Clear starts
    the count over. A key's xpub is its own: **Its xpub**, "waiting for
    its xpub file", **Scan its xpub**. GPG, Secure Boot and the APK key
    ask for the time first where TOTP codes does (`docs/TOTP.md` §1.3).
    A stick visit's write asks before going over a file of the same name (the
    stick visit, below).
59. **Decide where a key will be kept, then make it** (owner,
    2026-10-14; `docs/BACKUP-FLOW.md`, `docs/DECISIONS.md` F31; built
    for the APK key, GPG keys and Secure Boot keys, not yet the
    wallet or Nostr). One flow makes and backs up an APK key, a
    GPG key, Secure Boot keys, a Nostr key and a Bitcoin wallet's keys:
    where it will be kept (the plan) → each stick the plan names brought
    in with nothing secret open, checked, its vaults read, its place
    confirmed → existing vaults unlocked, new ones made → the key made
    into every vault the plan keeps it in → one lock → each stick
    written by place → done, unlock. The plan decides what each stick
    holds, and the flow narrows nothing it can say; only places with a
    stick are asked for. Two places or more for APK, GPG and Secure
    Boot keys, whose signing waits until every place is written; one or
    more for a wallet, with a warning when every secret sits at one
    place; any for Nostr; no upper bound anywhere. The blank template
    is printed from Faraday Starter. **Change the plan** at every step. The stick
    visit page is never shown, and after Back up's checklist nothing
    waits For the stick.
60. **Add a key is New key, opening on Start** (owner, 2026-10-19;
    `docs/history/NEW-WALLET.md` §15, `docs/DECISIONS.md` F44; replaces
    decision 18). New key's first card is **Start**: **Make a new key**
    (ticked), **Type the words**, **Scan a SeedQR**, and under **Other
    forms** (closed) **Make SLIP-39 shares**, **Type SLIP-39 shares**,
    **Type a codex32 secret**, **Type Seed XOR parts**; closed, it shows
    the choice and **Change** until the key is locked in. Make a new key
    goes on as decisions 34 and 38 say. A key typed is Start · **Words**
    · Key, one scanned Start · Key: Words is Add a key's form drawn by
    its own code (count, boxes or keyboard, lists; the other forms'
    fields with **Add this one** and **Scan a QR code**), whose Continue
    opens Key once the words are whole with a checksum that holds or the
    parts are enough; Key shows the words behind **Show words**,
    **Passphrase** and **Passphrase again** (none for codex32), and
    **Lock in**, which loads the key as Add a key does (not marked made
    here) and fills a Create slot; Continue then leaves the flow, with no
    Words or Quiz. Add a key on Wallets and its Tools tile open on Start;
    New key from a Create slot or its tile opens there too, Make a new
    key ticked, so a third Continue reaches the entries; a threshold
    wallet's slot offers Make a new key and Type the words only. The
    SLIP-39, codex32 and Seed XOR tiles open with their choice made and
    Start closed. The page is titled **Add a key**, and **New key** for a
    Create slot and from the Add a key screen's Make a new key, which
    opens with Start closed on it. The Add a key screen stays for a key
    asked for by name: a slot's fingerprint, a wallet's missing key,
    Restore's seed slots, and a seed scanned from Home or the Spend tab.
61. **A ⋮ on every file; Rename** (owner, 2026-10-20; `docs/DECISIONS.md`
    F54). Every file row, in Files and on the stick visit,
    keeps the one action it is for as a visible button and puts the rest
    behind a ⋮ at its right end, as the backup board's chips do
    (`docs/BACKUP-FLOW.md` §17.20: a popup under the ⋮, the sheet on the
    small panel, keyboard reachable, a press outside closes it).
    1. **Files, both columns.** The visible button is the row's main
       action, as today: Sign, Unlock or Contents, Load the wallet, Load
       this key, Import into the vault, Decode,
       Check the signature, Add to signatures, the shares' restore, the
       APK row's button, Make PDF for a sheet From the stick, Add to
       vault where it is the only action. The ⋮ holds, in this order and
       only where the row has them: **Add to vault** (where it is not the
       visible one), **Show as QR**, **Make PDF** (For the stick),
       **Rename** (For the stick only), **Details**, **Remove**. The
       visible **Remove** button goes. A row with no main action shows
       only the ⋮. A vault action (Add to vault, Import, Sign with key)
       is not shown, as a button or in the ⋮, while no vault is open;
       the vault's own row keeps its Unlock (changed 2026-10-20, F60).
    2. **Rename, For the stick.** A popup, "Rename {name}": a field
       holding the name before its extension, the extension shown fixed
       after it (`.ofv` for a vault; every file keeps its own), and
       **Rename** and **Cancel**. Refused, with a line in the popup and
       Rename disabled: an empty name, one starting with a dot, `/`,
       `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|` or a control character, a
       name over 255 characters with its extension, and a name another
       file For the stick has, ignoring case. The new name is the one the
       next visit writes it under; the visit's **Already on the stick**
       sheet then works as today. A vault For the stick whose same vault
       (by identity, `docs/VAULT.md` §6) is on the stick under another
       name is written over that file and the file takes the new name;
       where the new name is taken on the stick, as Already on the stick
       says. Renaming a vault's file never changes the vault's own name
       (the open vault's **Rename**), nor the reverse.
    3. **From the stick has no Rename**: its files are Faraday's copies of
       files on a stick, renamed on the stick itself (item 4).
    4. **On the stick (the visit page).** Every file row Faraday lists
       gets a ⋮ with **Rename**, **Delete from the stick** and
       **Details**; the computer's own files (decision 54) and the
       settings file get none. **Rename** is the same popup, renaming the
       file on the stick in place: the disk process renames its
       directory entry, lists the stick again, and the visit shows
       "{old} renamed {new}", or why not. Refused, nothing written, where
       the name is on the stick already, ignoring case: "{name} is
       already on this {stick}"; where that file is the same vault by
       identity, "{name} on this {stick} is this same vault" (one copy is
       deleted first). **Delete from the stick** opens the Delete files
       sheet (decision 54) with this file alone chosen; a vault file
       there needs the hold of decision 4. The **Delete files** button
       stays, for many files, the free space and the whole partition.
    5. **Details**, a sheet: the name, what it is (the row's kind line),
       its size in bytes, and its SHA-256 in groups of eight; nothing to
       press but Close.
    6. **Names elsewhere.** Backups and the backup flow name a copy by
       the file it was written as; a copy renamed on the visit is named
       anew from the next read of that stick, which the rename itself is.
    7. **Tests** (user-facing): a file For the stick renamed is written
       under its new name; a vault For the stick renamed, with the same
       vault on the stick as `vault.ofv`, leaves one file, under the new
       name; Rename refuses a name another file For the stick has and an
       empty name; on the visit a file renamed is listed under its new
       name and its bytes are unchanged; a rename to a name on the stick
       is refused and nothing is written, a vault's same vault named as
       such; Delete from the stick opens the sheet with that file alone
       chosen; Details shows the SHA-256 of the bytes; the Files row of a
       PSBT shows Sign and a ⋮ holding Details and Remove; no row shows a
       Remove button. Existing tests that press `InboxRemove` or the
       outbox's Remove reach it through the ⋮.

    **Built (2026-10-20), with what the building decided:**

    - Every ⋮ menu is drawn and keyed by one module (`menu.rs`): the
      backup board's chip menu and the file rows' menu share its popup,
      its sheet on the small panel, its rows, and its arrows, Enter,
      Escape and Tab handling. The file menu is `Sheet::FileMenu`; a
      choice in it closes it before it acts.
    - Where the ⋮ is: in Files on `wide`, at the right end of the row's
      button line From the stick, and at the card's top right For the
      stick, whose rows have no button line, with the row's state
      ("Ready") left of it and the card one line shorter (owner,
      2026-10-20); on the small panel, at the card's top right beside
      the name; on the visit, at the stick row's right end, where a
      press opens the menu and does not tick the row. A file over 18 MiB
      has one; the settings file has none.
    - For the stick keeps one of Make PDF (a sheet, online) and Show as
      QR, as its button did. Add to vault in the ⋮ is there only while
      a vault is open (F60).
    - The Rename field takes any character and says why at once; spaces
      at the ends are dropped; the name unchanged leaves Rename disabled
      with no line. On the stick the popup checks only the name's form:
      whether the name is taken, and by the same vault, is the disk
      process's answer (it reads both vaults for their identity), said
      on the visit. The pipe has `Request::Rename`, answered
      `Renamed`, `NameTaken { same_vault }` or `Failed`.
    - A vault written over its same vault on a stick keeps the name the
      stick has for it, unless the person renamed it For the stick: the
      app keeps the files renamed (across a lock, key `renamed`) and the
      write says so (`renamed` on `StorageCommand::Write` and the pipe's
      `Request::Write`); then the file takes the new name, or
      `name-2.ofv` where another file has it. The mark goes once a visit
      has written the file, so another stick holding the same vault
      under its own name keeps that name. `docs/VAULT.md` §6 step 3 says
      so.
    - Renaming a vault's file For the stick renames the file an open
      vault seals back into, so Lock does not leave a second copy under
      the old name; that changes what the next write of it is called,
      and renames the same vault only on the stick that write goes to.
    - A file chosen on the visit stays chosen under its new name; a
      backup flow's copy on that stick (by label and serial) takes the
      new name when the rename is answered.
    - Details of a file on the stick: the shell reads it through the
      disk process and hashes it (`StorageCommand::Hash`); its bytes are
      wiped there and only the hash reaches the app, "Reading…" until
      then, and "Not read: …" for a file over 18 MiB. The SHA-256 is two
      lines of four groups.
    - Wherever a vault is among the files chosen on the Delete files
      sheet, by Delete from the stick or by ticks, its button is the
      hold of decision 4, "Hold to delete 1 file with zeros"
      (`VaultAction::HoldWipe`); the plain button is not offered.
    - The ⋮s on Files and the visit are in the Tab order, in the order
      drawn; Enter or Space opens the one the keyboard is on, which is
      edged only once the keyboard has been used on it.

## Flows in progress (owner, 2026-10-16; F41) (from docs/FLOWS.md)

### header

Built 2026-10-19 (`faraday-core/src/in_progress.rs`; the sidebar's
`flow_row`, the small Home's tiles; `Action::Resume`).

### item 6

6. **Survey first**, before building: every screen whose state is kept
   when it is left, what counts as its first step done, and how each is
   finished or let go; the list in item 1 is corrected from it.

### item 8

8. **Tests** (`tests/all/sidebar.rs`, `tests/all/small_panel.rs`): Create a
   wallet taken to Places, Settings opened, then the sidebar's Create a
   wallet row returns to Places with the board as it was left; no row
   before a step is done; the row goes when the flow is finished or
   started over; a GPG key's guided flow has its row under Tools; the
   row is lit on the flow's screen and its tab is not; the small Home's
   first tile opens the flow in progress.

## Restart (owner, 2026-10-16; F42) (from docs/FLOWS.md)

### header

Built 2026-10-19.

### item 7

7. **Tests**, user-facing: the stick's power sheet offers Restart and
   Power off, the Pi's too, Faraday Desktop's Power off only; Restart
   with files waiting asks first, as Power off does; Restart ends the
   app with Restart's status and Power off with its own; on a Dell
   started with Secure Boot off the sheet shows Usual BIOS key F2, on a
   computer started with it on no row, on an unknown maker no row and
   the guide's code; the image's inittab restarts on Restart's status
   and powers off otherwise (a test of the line with a stub `reboot`
   and `poweroff`); item 4's table agrees with the maker pages.

## Cancel: leaving a flow for good (owner, 2026-10-19; F51) (from docs/FLOWS.md)

### item 5

5. **Tests** (`tests/all/sidebar.rs`): Create taken to Places, Cancel, the
   sheet, Cancel the flow: Wallets shown, no row, keys made still
   loaded; Keep going keeps it; a flow on its first card ends without
   the sheet; the GPG flow's Cancel removes its Tools row.

### built line

Built 2026-10-19 (`faraday-core/src/in_progress.rs`: `flow_cancel`,
`cancel_loses`, `flow_end`, `cancel_sheet`; `Action::CancelFlow`,
`Sheet::CancelFlow`).
