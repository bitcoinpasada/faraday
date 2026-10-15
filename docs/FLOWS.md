# Flows

**Status:** draft v0.1 · 2026-10-04 · every flow, where it starts, and
where it leaves the person. Not built.

The rule this file checks: **no flow ends on a dead end.** Each one
finishes on a screen that shows what happened and offers the next thing a
person would do. Where a flow cannot go on (no vault open, no file in
Files, a stick still attached), it says what is missing and offers the way
to get it.

`PLAN.md` holds the decisions; `docs/VAULT.md` the vault format;
`docs/QR.md` the scanner.

## Decisions this pass made

1. **One Files screen holds what came from the stick and what goes to
   it.** Files read from a stick had nowhere to be seen until a screen
   asked for one. The sidebar's status row reads "Files · 2 in · 1 out"
   and opens it. Every file copied in carries the action that uses it: a
   PSBT **Sign in Wallets**, `BOOTX64.EFI` **Sign in Secure Boot**, an
   unsigned APK **Sign with APK key** (`docs/APK-SIGNING.md`), an
   entries file **Import into a vault**, a vault file **Unlock**, a
   received file **Show as QR**. Both halves are emptied at power-off.
   The halves are named by direction (owner, 2026-10-10;
   `docs/SIMPLIFY.md` §4.1): **From the stick** and **For the stick**
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
   (2026-10-10, `docs/SIMPLIFY.md` §2.3): its last card, **Back up**,
   offers the plan's presets and opens the backup on the wallet just
   made, whose checklist puts the seeds and the wallet into a vault,
   lists the public files (descriptor, wallet file, multisig config,
   BSMS record, Bitcoin Core import, each key's account key and its BSMS
   record; each with **Show as QR** and **PNG** where a wallet reads it
   from a code, and **Remove** while it waits For the stick) and has the seeds copied by hand,
   each copy checked by its form (F29; specified, not built): a words
   copy by typing the whole seed back as words with the seed not on
   screen, a SeedQR copy by scanning the drawn grid (by typed words
   where there is no camera), which **Keep the words only** turns into
   words by hand.

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

11. **Home is the Start section, one lead and at most two secondaries**
    (owner, 2026-10-10; `docs/SIMPLIFY.md` §1 and §1.2). The three status
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
    (owner, 2026-10-10; `docs/SIMPLIFY.md` §1.3). "What do you have?" and
    "What do you want to do?" are gone from the Wallets start page: every
    job they offered (Sign a transaction, Back up, Show wallet QR, Sign a
    message) is reached from the wallet card, opened from a wallet's row,
    and from Tools.
13. **Learn joins the sidebar, after Tools** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §1.4). The small panel's Home grid carries it in
    Add a key's old place.
14. **The network chooser moves to Settings** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §1.5). Home's network pill, and the sidebar's own
    badge, are off-mainnet badges that open nothing; choosing a network
    is a Settings row.
15. **The session strip** (owner, 2026-10-10; `docs/SIMPLIFY.md` §1.6).
    The sidebar (and the small Home's status card) carries one line,
    **Bring in · Open · Work · Write out**, the current stage in the
    accent, computed from what is attached, loaded and waiting; tapping
    it opens Files. **Write out** is files For the stick, or a vault
    changed since it was written (decision 26), with no stick attached.
16. **A step card with a default opens closed** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §2.1). Make a new wallet, New key and Sign a
    transaction open on the first card without one; a closed default
    card shows **Change** in place of the chevron.
17. **Make a new wallet shows two kinds** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §2.2). Kind offers Single key and Multisig, both
    native SegWit; the other eight kinds are behind **More kinds**.
18. **Add a key opens on words** (owner, 2026-10-10; `docs/SIMPLIFY.md`
    §2.4). SLIP-39 shares, codex32 and Seed XOR are behind **Other
    forms**, unless a Tools tile for one of them opened the screen.
19. **Sign a transaction reviews the transaction before Check** (owner,
    2026-10-10; `docs/SIMPLIFY.md` §2.5). The Transaction card comes
    before Check, carries the txid as one of its rows, and Check stays
    closed once a wallet has been checked this power-on.
20. **Make a new wallet ends in the backup plan** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §2.3). The cards are Kind · Quorum · Keys · Check
    · Back up. Keys' Continue makes the wallet; Check shows the
    descriptor as a summary row that opens its code with the whole text
    under it; Back up is the plan's three presets, each opening the
    backup on the new wallet with that preset, its chip "Then: Wallets",
    its way back the wallet's card. The vault, public-files and paper
    cards are gone: they are the checklist's items.
21. **Create a vault offers two sizes** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §3.1 and §3.2). Its cards are Size and Name and
    passphrases; Size is **PCs only** (512 MiB) or **PCs and a Raspberry
    Pi** (64 MiB, the default), both 3 passes, so the flow opens on Name
    and passphrases. **Customise** opens the three cards it stands for
    (where it opens, unlock cost, space per passphrase) in its place. The
    Name field's caption says the name is what it holds, for anyone who
    sees the stick.
22. **A new vault is opened once more, then shows its next steps**
    (owner, 2026-10-10; `docs/SIMPLIFY.md` §3.3). This replaces decision
    2: **Create vault** always goes to Unlock with the new vault picked,
    under "Type the passphrase once more to open it"; unlocked, it opens
    on Vault contents, which says "Nothing in it yet" with every add
    action as a button (decision 58) and **Write it to a stick** (Files).
    Vault contents lists only the kinds a slot holds something of, then
    a row per add action.
23. **A locked vault says what it held** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §3.4, `docs/VAULT.md` §11). What a vault held when
    last seen open is kept across a lock until power-off: its Vaults row
    goes by its name and Home's lead is "Unlock {name}", both with the
    line "Savings 2 of 3 · key 9a6a2580 · 12 entries · seen 14:02"; with
    nothing remembered, "Unlock to see what it holds".
24. **For the stick fills itself** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §4.2). A public file a flow makes goes For the
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
25. **A write leaves a receipt** (owner, 2026-10-10; `docs/SIMPLIFY.md`
    §4.3). When a stick visit's write is answered, what it sent leaves
    For the stick and becomes the receipt, shown under For the stick as
    "Written to {label} at {time}" with a row per file, "verified" or why
    not. It is kept across a lock until power-off and replaced by the
    next write; nothing on it is written again. A vault written stays in
    Files as the stick now holds it. With nothing loaded, Home leads with
    "Written to {label}" and the file names, opening Files; a backup
    checklist item counts a file on the receipt as done.
26. **A vault's currency** (owner, 2026-10-10; `docs/SIMPLIFY.md` §3.5).
    Each vault file reads one of three states, computed from the
    receipt: "Never written" (sealed For the stick, on no receipt), "On
    {label} · current" (the receipt wrote the bytes it has now),
    "Changed since written" (open with unsaved changes, or sealed bytes
    that differ). A vault copied in from a stick and not changed reads "On
    {label} · current" for the stick it came from, remembered across a
    lock, or "Unchanged" when that stick is not known. It leads the vault's Vaults row; a changed vault makes
    Home lead with "Write {name} to a stick" and the session strip read
    Write out.
27. **The boot sheet is a front door** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §4.4). With the stick in, the sheet reads "Read
    from {label}", one line of counts ("1 vault · 2 PSBTs · 4 other
    files") and "Pull the stick to continue", with no lists. Once it is
    out, a stick with one vault puts that vault's passphrase field on the
    sheet with **Unlock** and **Not now**; several put **Unlock** on each
    row, which opens the field under it. An unlock that succeeds imports
    everything, the vault's wallets and keys with the stick's, and closes
    the sheet; a wrong passphrase stays and says so. Everything is ticked
    by default but a secret's file, whose key loads without it. **Choose
    what to import** shows the ticked lists, and **Import** brings in what
    is ticked without unlocking. With no vault everything comes in as the
    stick is pulled; with only the settings file there is no sheet. **Not
    now** leaves Home leading with "Unlock {name}", which opens the sheet
    again.
28. **The lock, write-out and power-off sheets lead with what is kept**
    (owner, 2026-10-10; `docs/SIMPLIFY.md` §4.5). Each lists **Kept**
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
    `docs/SIMPLIFY.md` §4.6). A stick visit ticks every file it can read
    to come in, but a seed's by its name (words, SeedQR, BIP-85), one over
    18 MiB, and one already From the stick by name and size (F26,
    specified, not built: over 64 MiB it asks first, whole or only for
    signing; past the memory a whole import may take, `docs/LARGE-FILES.md`
    §4, it is unticked; one whose name §9 there refuses is inert). **Select
    all** on the visit's right column is gone; **Unselect all** takes its
    place while anything is ticked.
30. **The visit says where it goes next** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §4.7). The stick visit's title carries the chip
    "Then: Unlock" when a vault was sealed for it (For the stick, or on
    the last write's receipt), "Then: {flow}" when the stick came in
    during one, else "Then: Home"; pulling the stick goes there.
31. **Backups: where each wallet's backup is** (owner, 2026-10-10;
    `docs/SIMPLIFY.md` §5). A Backups screen lists every wallet known
    and its backup map, each place with what this device saw of it; the
    wallet card carries the same as one line, and Home's second tile is
    Backups once a wallet is known (below, "Backups"). A paper place is
    checked only once a check of that copy matched here: one check, one
    copy.
32. **Light is the default theme** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §1.1). A device with no settings saved starts
    in `Theme::Light`, not Tokyo Night.
33. **No bar for a test network** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §1.2). The 4-point `WARN` fill along the top on
    a test network is gone, in the main area and in the sidebar; the
    pill alone marks it.
34. **Make a new wallet and New key open on their first step** (owner,
    2026-10-10; `docs/NEW-WALLET.md` §2.1, reversing decision 16 for
    these two flows only; Sign, Back up and Create a vault keep it).
    Create opens with Kind open, whatever opened it — Wallets, Home, or
    a Tools tile with its kind already ticked; Kind's Continue closes it
    as done and opens the next card. New key opens with Length open;
    Length gains a Continue that takes the count shown and opens
    Randomness (pressing a count does so already); Randomness's
    Continue opens the entry card as now. A card with a default and not
    open still shows its value and **Change**.
35. **A key in one slot is offered in no other** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §2.2). On the Keys card, a key loaded here
    that fills one slot, or a cosigner's file whose key fills one slot,
    is not offered for any other slot; clearing the slot offers it
    again.
36. **The Keys card's foot button opens the next empty slot** (owner,
    2026-10-10; `docs/NEW-WALLET.md` §2.3). While a slot is empty, the
    button at the foot of Keys reads **New key for Key N** (**Share N**
    on a threshold wallet), N the first empty slot, and opens New key
    for it; once no slot is empty it reads **Continue**, as now.
37. **Check names whose addresses they are** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §2.4). On Create's Check, a section label **The
    wallet's first addresses** sits above the address rows, so that a
    multisig's three rows do not read as one address per key.
38. **New key: the check on the entry card, Lock in, then back up**
    (owner, 2026-10-10; `docs/NEW-WALLET.md` §3.1–§3.5, decisions 4–6).
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
    `docs/NEW-WALLET.md` §3.6). On the Keys card, a slot filled by a
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
    (owner, 2026-10-10; `docs/NEW-WALLET.md` §4, decision 8). On the
    plan's Places question a multisig's **Each place keeps** lists **The
    whole wallet sheet**, ticked by default, then **Its own share**,
    with **What is a share?** beside it opening Learn on *Shares of a
    wallet description*; closing Learn leaves the plan on Places. With
    Its own share ticked, **Keys left off each share** is a slider from
    0 to m − 1, then the shares' fingerprints and the split audit. The
    checklist's Shares card has the same slider in place of its number
    buttons. The page is listed among a multisig backup's Learn pages.
41. **Back up puts one seed in each vault** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §5, decision 9). The seeds' row reads **Into
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
    2026-10-10; `docs/NEW-WALLET.md` §7.2, decision 11). Vault contents
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
    `docs/NEW-WALLET.md` §11, decision 13). The boot sheet's Unlock that
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
    wallet, past What do you have? (`docs/SCAN-TO-SIGN.md` §4): **Load the
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
44. **The chart's sheets** (owner, 2026-10-10; `docs/NEW-WALLET.md`
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
    plan has that item; **Check a copy of this seed**: **Scan my copy**,
    which leaves the chart under the camera, and **Type it back**. A key
    not here: **Load this key** with its way in, where the quorum needs
    it. A place: its lines, each opening its own sheet; **Rename** (the
    plan's place names, with a vault open; else the vault way, "Open a
    vault to keep this"); **What goes in its envelope**. A vault: the
    vault way or **Open the vault**, **Save its seeds into it**, **Copy
    to another stick** (the stick visit). Files: **Write the seed files
    again**, **Write the public files again**. Software: **Show the
    descriptor QR again**, **Public files for this software**. Lines:
    words or SeedQR, **Scan my copy**, **Type it back**, **Make another
    copy**; the wallet sheet, **Print it again**, **As a QR picture**,
    **As text**; a share, **Print the shares again**, **Keys left off
    each share** (the checklist's shares); a vault's stick, the vault way
    and **Copy the vault to another stick**; a seed file, **Write it
    again**; the description, **Show wallet QR**. A flow on Back up comes
    back to the card, or to the vault opened from. What only edits the
    plan, marks and dates, lost and exposed, and a holder's name came
    with the second batch (decision 49).
45. **The backup comes before spending** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §14, `docs/DECISIONS.md` F6). Back up's
    checklist: an item's foot button is its next action (a vault item's
    **Make a vault** or the way to one, then one **Save into {vault}**
    that saves every seed, passphrase and the description the plan puts
    there, then **Continue**; a copy's **Check my copy**; files' **Make
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
    `docs/NEW-WALLET.md` §11.1a). Reversing decision 43's own rule for
    it: an Unlock from the Vaults list loads the wallets the vault
    holds, with their keys where the vault holds them, and lands on
    Wallets with the same toast and wallet chosen as the boot sheet's
    Unlock (decision 43). A vault that holds no wallet lands where it
    does today; an unlock begun from inside a flow still returns to it.
47. **Mouse back and forward, selecting text, the default count**
    (owner, 2026-10-10; `docs/NEW-WALLET.md` §13). On the desktop the
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
    2026-10-10; `docs/NEW-WALLET.md` §12). Restore opens on **Kind**
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
    held for its backup (decision 45).
49. **The chart changes the plan** (owner, 2026-10-10;
    `docs/NEW-WALLET.md` §9.5–§9.7, §9's second batch). A change to
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
    while something is under way or waiting (`docs/SIMPLIFY.md` §1.2
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
    first", or why not). Hidden files and the boot partition are never
    touched.
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
    `docs/SCAN-TO-SIGN.md` §2–§3). Where a wallet is short of keys, one
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

## Home

**Start** is decision 50's tiles. **Add a key** (type, scan or bring
in a seed) is on Wallets and in Tools → the Add a key screen: **Scan a
SeedQR** first, full width over the words (primary until a word is
typed, when **Add key** is), a form (BIP-39 words, with the count
**12 · 15 · 18 · 21 · 24** above as many boxes, 12 first; SLIP-39
shares, codex32, Seed XOR parts), **Make a new key**, and "From a stick ·
plug it in" (with a stick attached, **Copy files in** → Stick visit; a key
file copied in loads when the stick is pulled). **Scan** floats in the
bottom-right corner of Home on every panel → the scanner (with a stick
in, the sheet asks for it to be pulled first). Home's Scan takes a seed
too: a SeedQR, a CompactSeedQR or a seed's words go to Add a key and are
added as **Scan a SeedQR** adds them, never into Files; a
private key or xprv is named, since Add a key does not take one. The
Wallets tab
carries **Add a key** last among its ways in (decision 50).
A key added with no wallet loaded lands on Wallets, whose card says
"Key 73c5da0a is loaded. No wallet uses it." with **Make a wallet from
this key** (Restore's Seeds card with the key in its slot: single key,
native SegWit, account 0 → **Make the wallet**), **Add another key** (the
same card on a 2-of-2 multisig) and **Restore an existing wallet**.
Several such keys: one line each, the buttons on the one picked. Back
from the seeds card returns to Wallets with the key still loaded.

## Session

**Boot.** Firmware loads the system → the boot medium's data partition
copied into memory, but a file over 18 MiB (F26, specified, not built:
over 64 MiB, counted and left for a stick visit): vault files into From the stick, every other file held
apart for the import, a PNG as what its QR codes hold → the import sheet
over Home: "Read from {label}", what was read in one line ("1 vault ·
2 PSBTs · 4 other files"), "Pull the stick to continue" (Pi: "Pull the SD
card to continue") → the moment no removable partition remains (decision
27):
- one vault from the medium → its passphrase field on the sheet →
  **Unlock** → everything imported (the vault's wallets and keys, the
  stick's wallets, keys and files but a secret's file) → Wallets, on the
  first wallet loaded that can sign here, with "Savings loaded from
  vault.ofv" (decision 43; Home, leading by its ranking, when no wallet
  loaded); a wrong passphrase → stays, says so;
- several vaults → **Unlock** on each row opens the field under it; one
  unlocked imports everything; the others stay locked in Files;
- **Choose what to import** → the wallets found, once each, ticked, each
  with whether it can sign here; the keys no wallet uses, ticked; every
  file, ticked but a secret's → **Import** → what is ticked, without
  unlocking → Home;
- no vault → everything imported at once → Home, leading with **Sign
  {psbt}** when one came;
- **Not now**, or a tap beside the sheet → Home, the files waiting; Home
  leads with **Unlock {name}**, which opens the sheet again;
- nothing on the medium but the settings file → no sheet; Home.

**Unlock.** Choose a vault (its memory need shown first) → passphrase →
- opens → Home, or the next step of what is in Files (decision 3);
- wrong passphrase → stays, says so; no lockout (an offline copy can be
  attacked without the device anyway);
- needs more memory than is free → said before the passphrase field;
  choose another vault;
- a stick is attached → "Remove the stick to unlock", which clears itself
  when the stick goes.

**Lock** (sidebar, Home's vault card, a vault's page, or 5 minutes idle) →
vaults sealed For the stick → the Wallets session wiped → app restarts → Unlock.
After 10 more idle minutes with nothing For the stick, power off; with
anything there, or a backup flow not finished (decision 59), stay on
Unlock and list what is waiting. While the backup flow is on screen
waiting for a stick, there is no idle power-off. The idle power-off is
not withheld when Secure Boot is off; it shows no sheet, so its
reminder to turn Secure Boot back on is the guides' last step
(`docs/STARTER.md` §7.5, F33).

**Power off** → decision 5, the sheet listing what is kept and what is
wiped (decision 28) → off. On a PC stick that started with Secure Boot
off, or with its state not read, the sheet leads, under its title and
before Kept, with "Turn Secure Boot back on in the BIOS before starting
Windows." in the warning tone, and the QR code of this computer's guide
under it (`docs/DECISIONS.md` F33, `docs/STARTER.md` §7.5). It asks for
no press of its own: Power off is the press. Not on the Pi, not on
Faraday Desktop; a Mac as `docs/MAC.md` §3.2. Specified, not built
(batch F33c; today the line is muted, reads "Secure Boot is off. Turn
it back on before starting Windows." and is not shown when the state
was not read).

## Sticks and cards

On the Pi every medium is an SD card and every screen says so: "SD card"
for "stick", **SD card visit** for **Stick visit**, **Lock and use SD
card**, "Remove the SD card" (`docs/DECISIONS.md` F4). The flows
below use the PC's words.

**A stick or card inserted while unlocked** → **Lock to use this stick**
(what is kept, then what is wiped from memory and where else it is,
then what happens; decision 28) →
- **Lock and use stick** → lock → **Stick visit**;
- **Not now** → nothing is handed out; a banner says "Remove the stick to
  keep working" until it goes.

**A stick or card inserted while locked, or with no vault open** → **Stick
visit** directly.

**Stick visit** (one screen, both directions):
- **In Faraday** (left): chosen files written, read back, compared:
  those For the stick, then those from it under **From the stick** (below).
  What is written and matched leaves For the stick for the receipt
  (decision 25). When a file to write has a name the stick already
  holds (ignoring case), the sheet **Already on the stick** comes first,
  a row each, with **Replace** (written over the old file: the new bytes
  under a hidden name, read back, then the old file removed and the new
  one renamed to its name), **Keep both** (written beside it as
  `name-2.ext`; the row says "{name} · Keep both: {name-2}") and
  **Cancel** (nothing written, the visit stays). A vault of the same
  length under its own name is listed "{name} · the same vault, updated
  here" and is always replaced; a vault under another vault's name
  "{name} · another vault · kept, this one as {name-2}", whatever is
  chosen. Those two alone bring no sheet, nor does the settings file. A sealed vault replaces the file with its identity, or is written
  under its own name (`name.ofv`, `name-2.ofv` when the stick has one;
  `docs/VAULT.md` §6) where there is none. A failed comparison
  leaves the file For the stick and says which. The first row is the
  settings, `faraday-settings.txt`, which is not a file For the stick: ticked
  on the boot stick when the settings differ from what the boot stick
  holds, unticked on any other stick, and written over the settings
  file already there.
- **From the stick**: every file copied in, unticked, written from Files,
  where it stays for the next stick: a copy from one stick to another,
  one after the other or both in at once. A file the stick shown holds
  already (by name and size) is listed greyed, tagged **On this stick**
  (**On this SD card**), with a disabled checkbox that cannot be ticked:
  a file just copied in shows here at once, and no "Copied" line says
  it. A public or sealed file (a GPG public key or signature
  included, F22; a revocation `.rev` file is not one, F32) is ticked like a file For the stick, and
  Select all ticks it. A secret kind (words, seed
  parts, entries, a carry file), text and a file Faraday does not read
  open the secret sheet instead, never ticked by Select all: text, words,
  seed parts and entries may go into the open vault as a note; past the
  line ("Faraday cannot tell whether this is a secret" for text and
  other files) **Tick it to write** ticks the row. Nothing goes For the
  stick. A picture is what its codes hold.
- **On the stick** (right): every file it reads ticked but a seed's, one
  over 18 MiB and one already From the stick (decision 29); **Unselect
  all** clears them. A file whose name and size match an Inbox file is
  tagged **In Faraday**. Chosen files, any of them; a file Faraday
  reads as nothing it knows comes in as a File, for **Sign a file** or
  QR **Send**, and is dropped at a lock. A vault file copied in appears
  on Vaults, locked. A
  PNG is ticked like any other file: the disk
  process decodes it, and each code it finds is routed as a scan would be
  (`docs/QR.md` §2), with a multi-part transfer showing its progress
  across images; the picture itself comes in too, as it came, to be
  written to another stick. A JPEG's codes are not read; it comes in as
  a File. A file over 18 MiB is listed and cannot be ticked.
  (F26, `docs/LARGE-FILES.md`; specified, not built.) A file over 64 MiB
  is listed unticked with its size; ticking it opens the sheet of
  §3 there: **Import the whole file** (with Size, Free after importing
  and what the vault needs; refused where it would leave too little to
  open one) or **Import only what signing needs** (a signing-only entry
  in Files: Sign a file, Sign with APK key, Delete, nothing else). A
  long read shows its progress and **Stop**. A pending signed APK (§7
  there) is a row For the stick, ticked when this stick holds its
  source, else "Needs {source name} on this stick"; it is written with
  progress and **Stop**, and "{source name} changed since it was read.
  Nothing written." where a part differs.
- **Delete a vault file** (decision 4).
- The title's chip says where pulling the stick goes (decision 30) →
  **Remove the stick** → once gone → **Unlock**, with the vault sealed for
  the visit, or the flow it came in during, or Home, which leads with the
  receipt.

**A stick pulled during a write** → the write is reported failed, the file
stays For the stick, and the visit screen says to put the stick back.

## Vaults

**Vaults overview**: each vault in memory with its state (open, locked)
and its currency (never written, on a stick and current, or changed
since written; decision 26) and source; an open one with its size and
cost, a locked one with what it held when last seen open (decision 23);
**Create a vault** at the top.
- open vault → **Open** → Vault contents;
- locked vault → **Unlock** → passphrase sheet → Vault contents.
- opened from a flow that needs a vault (decision 10): **Unlock** →
  the flow, on the step it was on; the way back returns to it.

**Create a vault** (step cards: Size, then Name and passphrases;
**Customise** puts where it opens, unlock cost and space per passphrase
in Size's place; decision 21) → **Create vault** → Unlock with the new
vault picked, "Type the passphrase once more to open it", sealed copy
For the stick → **Unlock** → Vault contents, "Nothing in it yet", a
button per add action and **Write it to a stick** (Files) (decisions 22
and 58). Opened from a
flow that needs a vault (decision 10), it shows "Then: {flow}" by its
title, and the unlock goes back to the flow.

**Vault contents**: the kinds it holds, then a row per add action
(decision 58) → items → details. Unlocked, it opens on Wallets where it holds one (decision 42).
- **Add an entry** → form (title, username, password with **Generate with
  dice**, URL, notes, TOTP secret typed or **Scan**) → saved → the entry
  selected in the list; vault marked unsaved.
- Entries list → **TOTP codes** → the codes screen (below).
- An entry with a TOTP secret → **Hold to show as QR** under **TOTP
  secret** → the QR sheet with the stored `otpauth://` URI, marked
  Secret, no PNG, nothing For the stick; **Done** closes it (F28).
- **Import entries** (from a file in Files) → preview: entries found, lines
  skipped with the reason → choose the vault → import → Entries list.
- **Edit** → the same form → back to the item.
- **Hold to delete from vault** → gone; the next item selected.
- A Bitcoin key or wallet → **Open in Wallets**, **Open in OpenSigner** (a key), or **Don't load at unlock**.
- A GPG key → **Export public key** with **Show as QR** and **PNG** on its
  row; **Export the kept revocation** and **Make a revocation** (the
  secret sheet, no code or picture); **Sign a file**, whose list shows
  each file's SHA-256 and a small text file's text, with **Show as QR** and
  **PNG** on each file's row and a hold to sign; **Renew** with a term
  chosen; **Hold to mark the key revoked** (F32).
- Secure Boot keys → **Open Secure Boot**.
- A recovery sheet → its descriptor, with **Show as QR** and **PNG** on
  a row under it (F28).
- An APK signing key → its page: name, certificate SHA-256, **Certificate**
  with **Show as QR** and **PNG** on its row, **Sign an APK**.

**Change passphrases, cost or space** → **Make a new vault from this one**
→ Create vault, filled in from this vault, with this slot's contents
carried over; the form says that slots whose passphrases are not entered
are not carried (`docs/VAULT.md` §8) → new vault created → the old one
offered for **Remove from this session**, and decision 4 for the stick.

**TOTP codes** (decision 57, `docs/TOTP.md` §5): from a Tools tile or
the Entries list's own button. No vault open → Make or unlock one, same
as a vault category; the clock not checked this power-on → **Set the
time**, which returns to the codes. **Now** (local time with seconds and
the zone) and **Set the time** at the top, then one row per entry with a
TOTP secret in every open vault: its code, grouped, with a progress bar
for the time left in its period; an HOTP secret shows an **HOTP** badge
and no code, one that does not read shows **Unreadable**. A row opens
the entry in its vault, where the **Code** row above **TOTP secret**
shows the same code live, or **Set the time** while the clock is not
checked. **Scan a setup code** and **Add an entry** at the foot.

## Backups

**Backups** is reached from Home's second tile (with no stick attached,
once any wallet is known; on the small panel, a tile after Wallets) and
from the wallet card's backup line. It lists every wallet known, once
each by descriptor checksum: the loaded wallets, then those in an open
vault, then those a locked vault was seen to hold (`docs/VAULT.md` §11).
Under each, its backup map a row per place, as the plan draws it (each
place, the vault, a stick of files, the watch-only software, the seeds
on their own devices), each with what it holds, its tag (secret, sealed,
public) and its state:

- a paper place: **Checked** once a copy of each seed it keeps matched
  here, typed back or scanned; the first check of a seed counts for the
  first place that keeps it, the second for the second, and so on; else
  **Not checked**. A place that keeps only a sheet reads its file.
- the vault: the vault file holding the wallet and where it stands,
  "vault.ofv · for the stick", "vault.ofv · on STICK, 14:02",
  "vault.ofv · changed since written"; "Not in a vault" when none holds
  it; "A vault is locked" when a locked vault not seen open this
  power-on might.
- public files: "On {label}, {time}" once the last visit wrote and read
  them back, **For the stick** while they wait, "Not made"; seed files
  For the stick read "For the stick, unprotected".
- the software: its files' state, else **Shown** once the descriptor
  was shown in the backup.
- the seeds on their own devices: "Not here".

A wallet with no plan reads **No backup plan** with **Back up** (on a
loaded wallet). A locked vault's wallet with a plan in that vault and no
record here reads "Plan · In {vault} · locked". What a plan holds is
recorded when its checklist is made, and each check as it matches; the
records are kept across a lock and gone at power-off. A place's name
shows only from the plan an open vault keeps, else "Place 1". On the
small panel one wallet a page, with **Previous** and **Next**.

The wallet card's line, under its shape: "Backup: paper ×2 checked ·
vault.ofv · for the stick" (paper "1 of 2 checked" while one is not), or
"Not backed up"; a press opens Backups on that wallet.

## Backup flow

(Decision 59; `docs/BACKUP-FLOW.md`, F31; built for the APK key, GPG keys
and the Secure Boot keys; the wallet's and Nostr's parts are not.)
Tools' **APK signing key**, **GPG key** and **Secure Boot keys**, Vault
contents' Make actions, a Nostr key's **Save into a vault**, Make a new
wallet and Restore → step cards (pages on a small panel, "Step 2 of
7"; the steps a plan does not need are not drawn):

1. **Where it will be kept**: a row per place (**Add a place**), what
   each keeps (a stick with Vault 1, Vault 2, a new vault; for a
   wallet, the plan's vault part), a row per vault (new or existing),
   the check lines; at least two places for APK, GPG and Secure Boot;
   for a wallet the one-place warning and, on a single key with no
   passphrase, the passphrase line with **Add a passphrase**;
2. **Bring in the sticks** (with anything secret open: **Lock and bring
   in the sticks** first) → "Insert the stick for Vault 1 (Place 1)", or
   **Use {label}, read at {time}** → usable, else why and what to do →
   its vaults: "Which vault is Vault 1?", or **A new vault beside them**
   → **This is the stick for …** (a tap), or the hold where two sticks
   read alike; a stick bound to another place refused → pull;
3. **Vaults**: **Unlock {vault}** for each existing one, Create a vault
   for each new one, **I do not have its passphrase** → a new vault
   beside it or another stick;
4. **Make the key**: the kind's own form → made into every vault the
   plan keeps it in; a wallet's New key into its vaults, a key kept
   only on paper shown for copying; then, for a wallet, the rest of the
   plan and the checklist;
5. **Lock**: Kept and Wiped from memory → **Lock and write**;
6. **Write the sticks**: "Insert the stick for Vault 1 (Place 1)" with
   what it will hold → a stick unlike the one read: "Not the stick read
   for …"; a vault's second copy: the first stick refused, then **Hold:
   this is a second stick** (**another stick** for a third or later
   copy); a stick named after step 2 (the stick of files, the stick of
   public files, which carries the public files, each key's account key
   and BIP 129 record, and the sheets as PDFs) checked now → writing → "Written
   and checked on {label}", its files and items ticked → pull;
7. **Done**: a row per stick → **Unlock {vault}** → the key's page, or
   the checklist's done card, with nothing For the stick.

**Change the plan** under every card: a new vault on an empty stick, or
another copy of a planned vault, adds a stick to step 6; a stick
holding an existing vault takes **Add {vault}** (lock, the stick read,
both vaults unlocked, the key copied across, lock), with a key kept only
on paper offered **Finish its paper copy first**; a place removed before
its write drops its new vault, after it stays on the map.

Back leaves on any card but while writing; the key's page, Home's lead
and a waiting action offer **Continue the backup**. A lock resumes on
the step; a power-off ends the flow, the Power sheet listing "Backup of
{key}: written to {n} of {m} places". A stick visit while a flow is left
lists its files **In the backup**, not tickable.

## Wallets tab

Every flow, entry point and step is in `docs/WALLETS.md`. What it
guarantees for this file's rule:

- **Every way in reaches a wallet card** (`docs/WALLETS.md` §2) that says what
  the wallet is, which keys can sign here, how many signatures the
  transaction has and needs, and the one thing still to do.
- **A transaction never stops at "can't sign"**: a missing key opens **Scan
  a key**, which takes any of the wallet's missing keys (decision 57); a
  missing signature opens **Collect**; a missing wallet opens **Load the
  wallet**.
- **Signing ends on two results**, the signed PSBT for the wallet that wrote
  the transaction and the finished transaction with its txid, each shown as
  a code and put For the stick the moment the transaction is finished
  (decision 24): Finish's primary is **Insert a stick to write it**
  (opening Files), or **Open Files** with a stick in, and each result's
  card carries **Remove** while it waits.
- **Anything added in a session offers Save to vault**, and locking lists
  what was not saved.
- **Creating a wallet ends in Backup and Save**, never on the descriptor
  alone.
- **The backup starts with a plan, then a checklist of only what it
  needs** (owner, 2026-10-09, proposal A; `docs/WALLETS.md` §5). Three
  presets (**Paper only**, **Paper and vault**, **Paper, vault and
  watch-only software**) fill the plan's questions, each a multi-choice
  list (DESIGN §4.2): where the seeds go, the places (how many, share or
  whole sheet, which keep the vault's stick, their names with a vault
  open), where the wallet description goes, the software and form, each
  passphrase's places. Beside them the map (one box per place, the
  vault, a stick of files, the software, the seeds on their own
  devices, each tagged secret, sealed or public) and the check: "Any one
  place lost: the rest rebuild the wallet", "One place found: can
  spend", "One place found: sees the balance", qualified "with the
  vault's passphrase" where the vault is what decides it. **Make the
  checklist** replaces the questions with only the items the plan needs,
  each done by what it does (a file For the stick or written, a seed or the wallet
  in the vault, a copy matched, the descriptor shown); the envelopes
  alone by **Done**. A place's name lives only in the vault (record
  type 11, `docs/VAULT.md` §7), which the next backup of the wallet
  starts from; no file, sheet, PDF or PNG holds one, and
  the blank template carries empty "Place ____ holds ____" lines
  instead. On a small panel: one question per page, the map as the
  plan's last page and in the envelopes item.
- **A seed's backup: by hand, into a vault, a file only past the secret
  sheet** (owner, 2026-10-09; `docs/WALLETS.md` §5 step 3). Paper stays
  first: a copy item per seed; the vault item has **Save into {vault}**
  (and "with its passphrase" when loaded with one) under each seed; the
  files item **Save as a file…**, which opens the secret sheet for the
  seed. The same on a small panel.
- **The backup says where each seed is kept** (owner, 2026-10-09;
  `docs/WALLETS.md` §5). The map on the side panel marks each line
  with its item's state; the copy page on a small panel lists the wallet (in which open vault, or
  not in one) and every key of the wallet by fingerprint: in which open
  vault, whether its copy by hand was checked, whether its file is in
  For the stick unprotected, or not here (backed up on its own device). With
  vault files and none open, it says the vaults were not checked.
- **Any loaded wallet backs up again, with every file on offer**
  (owner, 2026-10-09; `docs/WALLETS.md` §5). Entries: the wallet's card
  (**Back up**, also on the small panel), Start's **Back up a wallet**,
  the Tools tile, Create's **Back up** card and Restore's **Back up
  this wallet**. With more than one wallet loaded the head is "Back up a
  wallet" with the wallet's name as a chip, which lists the loaded
  wallets; a press on one starts the backup on it. A watch-only wallet
  (no seeds here) is asked nothing about seeds. The checklist's public
  files item is drawn from the
  wallet (descriptor, wallet file, multisig config, BSMS record, Bitcoin
  Core import, each key held here and its BSMS key record), the item
  showing the rows the plan's software picks. Each vault made has its
  own item (or the way to make or unlock one, back to the item), with
  its seeds and the wallet where planned (decision 41).
- **The descriptor and its shares as pictures** (2026-10-09). The
  backup's public files item (**PNG** on the descriptor's row) write `{name}-descriptor.png`: the
  checksummed descriptor as one static code, with the wallet's name, its
  shape (and the network when not mainnet), the keys' fingerprints, the
  descriptor's checksum and "Public: watch only, spends nothing" under
  it. Past one code (version 25 at ECL M) it goes as the BBQr parts the
  QR view makes, `{name}-descriptor-1-of-3.png` and on, each labelled
  with its part. The shares item makes each split share's
  `{name}-share-k-of-n.png` beside its PDF and text: the share's text as
  one code, labelled with the share's number and quorum, the keys it
  holds and leaves off, and "Not a wallet on its own". A scan of a share
  picture comes back as the share, and Restore takes any quorum of them.
- **Every public file a flow makes as a code and a picture**
  (2026-10-09; `docs/QR.md` §4 item 4). Beside a file's row, For the
  stick with **Remove** (decision 24), sit **Show as QR** and **PNG** where a wallet or a person
  reads it from a code: the backup's public files item (descriptor,
  multisig config, BSMS descriptor record, each key held here and its
  BSMS key record), Create's Keys card (**Show xpub QR**, **Xpub PNG for
  the stick**, **Xpub file for the stick** under each key held here),
  Sign a message
  (the message file For the stick as it is signed, **Show as QR**, **PNG
  for the stick**) and
  Silent payments (**Record for the stick**, **Record as QR**, **Record
  as PNG**). The QR sheet carries **PNG for the stick** (**PNG** beside
  the format on a small panel) whenever it shows one code of public
  content; a row's **PNG** is that button without the sheet, and says
  "{name} waits for the stick". Labels under the code: `Key {fp} · {kind}
  · {path}` for a key, `Silent payment address · {fp}` for the silent
  payments address and record, `Signed by {address}` for a message, the
  wallet's shape and keys for its files. A file past one code says so
  instead of writing parts; a PSBT, a transaction, a file shown from
  Files and a secret offer no picture.
- **Receive addresses as codes** (F28; `docs/QR.md` §7). A receive
  address row opens its code when pressed, a chevron at its end, as
  Create's descriptor row opens the wallet's: the wallet card's first
  receive address (and the small panel's), the first addresses of
  Create's Check, Restore's Check and the chart sheet's wallet, and
  Vanity address's found address. The code is the address as printed,
  shown whole under it; no PNG. A change address, the signing steps'
  **Addresses · Compare** and a wallet whose backup is not done ("Back
  up before you receive") open none.
- **A key in no wallet has a way on**: the card's buttons above with no
  wallet loaded; with wallets loaded, the key's row in the left column
  carries **Make a wallet from this key**. The left column runs top
  down, wallets, keys in no wallet, then its buttons, and scrolls when
  taller than the screen.
- **The wallet's card shows it at a glance** (owner, 2026-10-10;
  `docs/NEW-WALLET.md` §6). Its Keys list becomes **At a glance**, a
  structure chart (DESIGN §4.16): the wallet (name, shape, checksum);
  its keys, numbered as before, each with its fingerprint, label and
  where it is ("Can sign here", "Can sign · passphrase", "Cosigner ·
  xpub only", "On its own device"); its backup, a node per spot of the
  plan's map, each listing what it holds ("Key 1 words", "Vault 2
  stick", "Key 2 seed + passphrase", "Description"), its tag, what
  Backups saw of it, and what it gives whoever finds it alone
  ("Nothing", "Sees the balance", "Can spend", "Can spend with the
  vault's passphrase"; a vault "Sealed", with where its stick is). A
  line runs from the wallet to each key and from each key held here to
  every node holding its seed, a colour and dash pattern per key; a
  cosigner's key has none, and its seed is in no node. Above it the
  plan's three checks as a strip (decision 45). The plan is the backup's once its checklist
  is made, else the one recorded this power-on (kept across a lock),
  else an open vault's. No plan: "No backup plan" with **Back up**; a
  plan in a locked vault not remembered: "Plan in {vault}, locked" with
  **Unlock**. Every node, and every line of what a backup node holds,
  opens its sheet (decision 44); while the wallet is short of keys a
  key not here reads **Not here**, and one an open vault holds keeps
  **Load from vault** as a line of its own; the one way to scan any
  missing key is the signatures bar's **Scan a key** (decision 57). The
  card's
  body scrolls between its head and its buttons. On a small panel an
  **At a glance** row opens the chart as its own page, one column, no
  lines, each backup node naming the keys it holds ("Key 1 words · Key
  2 in the vault"); Back returns to the card.
- **Restore** (decision 48): Kind (with **I have the wallet
  description: Scan it**, **From Files**), Quorum for a multisig,
  Description or **I do not have it** (**I have the seed** for a single
  key), Seeds (a slot per key: its seed, its passphrase asked after it
  (decision 52), a cosigner's xpub, or Not here), Check,
  Done (**Back up this wallet**, **Spend from this wallet**, **Wallets**).

## Spend tab

Spend left the sidebar and the small Home grid (`docs/SIMPLIFY.md`
§6.2, 2026-10-10); it is reached from the Wallets empty state's
**Spend from a backup, step by step**, the Learn sheet's first row
**Spending, step by step**, Home's lead when a spend is under way, and
a wallet card's **Spend from this wallet** (decision 43), which opens it
on that wallet at **Load the wallet in Sparrow**, then **Write the
payment in Sparrow** (the PSBT by camera or stick), with **I have the
PSBT from Sparrow ready: Scan it** over the first page. The tab's own
screen and steps below are unchanged.

Family mode for someone spending for the first time
(`docs/FAMILY.md`): the map → the stick → **What do you have?**
(`docs/SCAN-TO-SIGN.md` §4, 2026-10-11: a vault in Files with its
Unlock and list, **Scan the QR codes you have**, **Type the words you
have**, **Not sure what you have?**, what came in, and the next
question: with one seed **Anything else in the envelope?** and **That's
everything**; with several and no description **Scan the description**
or **I can't find one**, which shows the seeds' shape and **Make the
wallet**; a description opens the wallet) → check the money in Sparrow
(**Yes, my money shows**; for a wallet worked out from seeds alone,
**Check the money is there, on the online computer** with **No money
shows**, which tries a passphrase, Taproot, nested SegWit and legacy in
turn, §5; **Receive address 0** and the receive rows open their code,
F28) → write the payment in Sparrow → bring the PSBT (camera, or a
stick: the lock cycle, after which the tab opens on the same page) → the
spend's own steps → **Put everything away** (lock, what waits for the
stick written to it, power off). Every page that cannot be reached yet sends the
person to the one that comes first. The intake scanner, **Scan what you
have**, reads one code after another until **Done**: a seed asks how it
loads and the camera comes back, a description opens the wallet, split
sheets rebuild it, an xpub fills a cosigner's box, a SLIP-39 share or a
codex32 string goes to Add a key's form for it, and a PSBT waits for
Bring the transaction here.

## OpenSigner tab (kept for now)

OpenSigner's own screens and flows, unchanged. From a vault, **Open in
OpenSigner** on a Bitcoin key runs OpenSigner's load flow with the key
already supplied, ending on its key page. Its encrypted-backup export
offers **Import into a vault**; its PSBTs and public exports go For the
stick ("Insert a stick to write it").

## GPG

Every GPG action needs an open vault (the secret key lives there). With
none open, Tools' **GPG key** tile reads "Make a vault first" (no vault
file) or "Unlock a vault first" and leads into Create a vault or Unlock,
which come back to the vault's GPG keys (decision 10). Making a key is
the exception (F31): **Make a key** opens the backup flow (decision 59),
whose steps decide and open the vaults first.

**Make a key** (the backup flow's steps 1 to 3 (F31): two places or
more, their sticks, their vaults; then name and email, expiry with 3 years chosen, as `gpg
--quick-generate-key` makes a key with none given) → created in every vault the plan keeps it in → after the flow's lock
and writes by place, the key's page: fingerprint, signing subkey, user IDs, validity, **Revocation
certificate** ("Kept with the key · made {date}"), **Paperkey** (a hold to
reveal; made only while held). The public key goes For the stick as the
key is made (decision 24). The revocation certificate does not: it is kept
in the key's record, as GnuPG keeps `openpgp-revocs.d/<FPR>.rev` beside the
keyring (F32). A key made earlier, with none kept, makes one when its page
is first opened with the clock known.

**Signing waits for the backup** (F31). A GPG key made in this session
cannot be made again from seed words, and its revocation is kept in the
same vault, so until the backup flow has written every place its plan
gives it (two or more), **Export public key** (the file again, **Show as QR**, **PNG**:
`{name}-{fp8}.png` with the user ID and fingerprint under the code).

**Export the kept revocation** and **Make a revocation** → the secret
sheet, a file `{name}-{FPR}.rev` in GnuPG's layout (its explanation, the
key, then the armour with a colon before its dashes, so that `gpg
--import` refuses it until the colon is taken out). **Make a revocation**
first asks the reason as `gpg --generate-revocation` does, 0 no reason, 1
compromised, 2 superseded, 3 no longer used, and an optional description.
**Hold to mark the key revoked** marks the key revoked here: it signs and
renews nothing after.

**Sign a file** → the page names the key as gpg-agent's pinentry does
(user ID, signing subkey ID and main key ID, creation date) and lists the
files copied in, each with its name, size, SHA-256 and, when small, its
text → tick → **Hold to sign** → a detached signature `{file}.asc` For the
stick. A file replaced by a stick visit after it was ticked is unticked.
A signature is not written over a file of that name. An expired or
revoked key, or a clock before the key was made, signs nothing and says
why in GnuPG's words. Each file's row also shows the signature as a code
(**Show as QR**) or writes it as `{file}.asc.png` (**PNG**). No file
copied in: "Copy the file in on a stick visit first." Changing the key,
the category or the vault closes the page and clears its ticks.

**Renew** → the current term is shown, a new term is chosen (1, 2, 3, 5
years or never) → the updated public key For the stick, and the key's
record changes with it. Refused when the clock is more than 4 seconds
before the key's last signature.

## Secure Boot

Needs an open vault for the keys. With none open, Tools' **Secure Boot
keys** tile leads into Create a vault or Unlock and back, as GPG's does;
**KeePass export** the same, to the vault's entries.

**Make keys** → the backup flow (decision 59): two places or more,
their sticks, their vaults → PK, KEK, db generated into every vault the
plan keeps them in → lock, the sticks written by place → Secure Boot
screen, with the **Backup** row until every place is written. **Signing
waits for the backup** (F31): keys made in the flow cannot be made again
from seed words, so until every place is written, **Enrolment
files** (its updates are signed by PK and KEK) and **Sign with db**,
with Files' **Sign in Secure Boot**, read **Back up first** and open the
flow. **Check a signature** does not wait. Keys in a vault unlocked from
a stick sign at once.

**Enrolment files** → policy (Windows-compatible or own keys only) → the
files For the stick → Learn page: enrolling in the firmware's setup screen.

**Sign `BOOTX64.EFI`** → needs the file copied in (none: "Copy
BOOTX64.EFI in from a stick first", with what a stick visit is) → build
record match → fingerprint compared through another channel → **Sign with
db** → signed file For the stick → Learn page: copying it onto the boot
partition on another computer.

**Check a signature** → result against the vault's db certificate.

## APK signing key

(`docs/APK-SIGNING.md`, F25.) Needs an open vault for the key. With
none open, Tools' **APK signing key** tile leads into Create a vault or
Unlock and back, as GPG's does (decision 10).

**Make an APK key** (the backup flow, decision 59: two places or more,
by default Vault 1 at Place 1 and Place 2, their sticks brought in,
their vaults unlocked or made; then step cards: name on the
certificate, **Faraday Companion** filled in, empty refused; package,
`app.faradayoffline.companion` filled in, "Not a package name" unless
Android's form)
→ the time first where it is not known → made → result: name, package,
certificate SHA-256, "RSA 3072", the certificate For the stick as the
key is made, with **Show as QR** and **PNG** on its row, and a row per
vault it is in → lock → the sticks written by place → **Unlock
{vault}** → the key's page in Vault contents (name, package,
certificate SHA-256, key, made, and the **Backup** row until every
place is written).

**Sign an APK** → while the key was made in the flow and not every
place of its plan is written (F31): **Back up first** →
the flow on its step → Done → **Unlock {vault}** → the key's page.

Otherwise:
choose an APK copied in (none: "Copy the APK in on a stick visit
first"; on the Pi "on an SD card visit"), each row with its size, and
on one that cannot be signed why, inert: "Not an APK", "Already
signed", "Not aligned", "Supports Android before 7.0", "Over 4 GiB on
FAT32", "Name too long", "Another app: {package}" (its package is not
the key's) → "Sign this APK?" (Hold: file, size, SHA-256, **Second
build**, package, version, Android, key, will write). **Second build**
is typed: the SHA-256 an independent second build printed, at least
its first 32 hex digits, case and spaces ignored; "Matches", or "Does
not match this file" in `WARN`; the hold is inert until it matches
(and, with `docs/LARGE-FILES.md`, while there is not enough memory to
open the vault, "Not enough memory to open {vault}") → hold → each signature checked
against the certificate ("The signature did not check. Nothing
written." otherwise) → the signed APK For the stick, named as the file
copied in without `-unsigned`, or with `-signed` → "Insert a stick to
write it". From Files, an APK's **Sign with APK key** picks the open
vault's key whose package is the APK's: "Which key?" when several are,
and "Another app: {package}", inert, when none is.

## Nostr key

(`docs/NOSTR.md`, F27; specified, not built.) BIP-85 application
128002', on the stick and the Pi only: Faraday Desktop has no Nostr
key on any network, and the demo has none. Needs a loaded key: with
none, Tools' **Nostr key** tile reads "Load a key", as Child seeds and
passwords does.

The tile opens the BIP-85 screen with Nostr chosen → Key (skipped with
one key loaded) → **Identity and account** (1 and 1, filled in; 0 and
anything above 2147483647 refused; **Find a pair**: the npubs of
identities 1 to 5 at account 1 and accounts 1 to 5 at the identity
typed, a tap picks the pair) → **Value**: the path, the npub with
**Show as QR** and **PNG**, the nsec masked until **Show** and never a
code or a file → **Save into {vault}** (an Entry; no vault open:
decision 10's way into making or unlocking one, and back; or the
backup flow of decision 59, any number of places, none included, with
nothing waiting for it) or **Export
encrypted…** (Password and Again, 12 printable characters or more →
"Encrypting" → `ncryptsec1…` as text, **Show as QR**, **PNG** and **For
the stick**, sealed). There is no **Out unprotected…**: the
ncryptsec is the only way the nsec leaves. Files and labels are named
by the npub's first 13 characters, never by the key's fingerprint or
the two numbers. The BIP-85 tile's Application card lists Nostr
seventh and leads to the same cards.

A saved Nostr entry's page in Vault contents adds **Show as QR** and
**PNG** for its npub, **Export encrypted…**, and, with its key loaded,
a **Path** row saying whether the notes' path gives the entry's key. An
Entry holding an nsec has no **Export for KeePass**. A file holding an
nsec is a secret in Files with no code and no way to the Outbox; its
**Save into {vault}** makes a Nostr entry. Faraday Companion refuses an
nsec and an ncryptsec.

## Explore

(F28, `docs/QR.md` §7; specified, not built.) From a key's **Explore
this key** or the Tools tile → the path and the script type → the
**Address**, **Public key** and **Extended key** at that path, each a
row that opens its code when pressed (the value as printed, whole under
the code, no PNG) → **Wallets**.

## Lightning node key

From the Tools tile → ldk-node from a loaded key, or LND with its
aezeed typed → **Node id**, with **Show as QR** and **PNG** (F28;
`lightning-{8 hex}.png` For the stick, "{name} waits for the stick"),
**Made**, the private key masked until **Show** and never a code →
**Save into {vault}** (decision 10's way when none is open), **Out
unprotected…** (the secret sheet).

## QR transfer tab

On the device this is Files and Scan, already: Files' **Show as QR**
sends a file in Files as codes (format, frame rate, part size,
progress, **Done**), and **Scan** receives into Files, routed by
`docs/QR.md` §2.

**Transfer**, in the desktop app only (the online Faraday; never on the
stick or the Pi), is the other end, the computer beside the device:

**Send** → a file from the list of `~/Downloads`, newest first, or one
dropped on the window (X11 only) → animated code, chosen by kind as Files
chooses it (a PSBT as itself; a wallet, key, message or share as text;
anything else in the file envelope) → format, frame rate and part size →
**Done**. Over 256 KiB: "Larger than 256 KiB: carry it on a stick".

**Receive** → the webcam reads anything, routes nothing → each whole
file saved into `~/Downloads` under its own name (envelope) or
`received-N.psbt` / `.txt`, never over a file ("name (2).ext") → "Saved
{path}", or "Saved {path}: this file holds a secret" for words, a
SeedQR, seed parts, entries or a carry file → the camera stays on for
the next → **Cancel**. Nothing goes into the desktop app's Files; a file
to use in the desktop Faraday is copied in from a stick folder.

## New devices and settings

**A keyboard or pointer arrives** → it does nothing until it types the
code shown, or clicks the target drawn → **Ignore this device** lists it
in Settings until unplugged. A keyboard that types three codes' worth of
letters without the code is ignored the same way, with a toast saying
to unplug it to try again. After three keyboards are ignored that way,
new keyboards are ignored without a code until Faraday starts again.

**No camera, or the camera is refused** → the scanner says which, and
offers Type and Read a file.

**Settings**: keyboard layout (US first), display scale, idle lock and
power-off times, reduce motion, light or dark, ignored devices, **Time**
(decision 56) and About (version, build hash). They are kept across a
lock in RAM, and across a power-off in `faraday-settings.txt` on the
boot stick, read at boot before the passphrase (`PLAN.md` §5.2). Never
for an idle time, the signed-amount memory's seal and the network are
not read from the file.

**Time** (decision 56): **Now**, the local date and time, or **Not
known**; **Time zone**, stepped by 15 minutes; **Set the time**, which
opens **Set the time** (a Date and a Time field, **Set**, and **Keep
this time** once the clock is already known) and returns to Settings.
