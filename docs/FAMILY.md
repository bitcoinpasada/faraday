# The Spend tab

**Status:** v0.3 · 2026-10-11 (page 3 and the guessed Check,
`docs/SCAN-TO-SIGN.md` §4 and §5) · built: `faraday-core/src/family.rs`
(state and logic), `family_screen.rs`, `family_text.rs`,
`tests/all/family.rs`, and the snapshot tool's `spend` tour
(`cargo run -p faraday-snapshot -- 1366x768 out/testkit OUT spend`).
The code names it `family`; the tab is labelled **Spend**.

**The owner's answers (2026-10-06):** the tab is called **Spend**; page 6
keeps the short Sparrow lines; the desktop app is for testing and keeps
every function, so page 2 states that it is the desktop copy and stops
nothing; the words route offers every single-key kind. Added the same
day: **a vault may hold only part of a wallet**, its descriptor and one
key say, and the other keys come from their own places (§4, pages 4
and 9).

The Spend tab is for someone spending from a wallet for the first time,
usually one they inherited: a Faraday stick, a passphrase, maybe paper. It
is one column of pages that goes from "what is in the envelope" to "the
transaction is sent and the stick is back in the envelope", with every
control the job needs on the page that needs it. Creating wallets and
backing up are out of scope.

Faraday's own parts: the vault, the stick rule and the lock cycle
(`docs/history/PLAN.md` §5, `docs/FLOWS.md`).

## 1. Rules

1. **A wording layer, not a second implementation.** Every control on the
   tab is one that already exists (Unlock, Restore, word entry, the spend
   steps, the QR sheet, the stick visit). The same paper gives the same
   addresses and byte-identical signatures in Spend and in Wallets.
2. **One state.** The tab drives `app.spend`, `app.session` and
   `app.vaults` like every other screen. A spend started in Spend shows in
   Wallets › Sign, and the reverse.
3. **Warnings are not simplified.** The review's warnings come from the
   function Wallets uses, word for word.
4. **Public first, secrets last, where the route allows.** On the paper
   route the wallet is opened from its description, checked and its
   transaction read before any words are typed. On the vault route the
   vault holds the wallet and its key together, so both arrive at unlock;
   nothing is signed before the transaction has been read.
5. **Data, not a classification** (`docs/SCAN-TO-SIGN.md` §4,
   2026-10-11). *What do you have?* takes whatever is in the envelope,
   scanned or typed, and asks only the next question; the wallet is
   worked out from what came in. (It was *What are you holding?*, whose
   answer chose a route.)
6. **The landing page is the map.** Short working text on each page; the
   longer explanation is behind **More about this** on the same page.

## 2. Where it sits

- `Screen::Family`. Not in the sidebar or the small Home grid
  (`docs/history/SIMPLIFY.md` §6.2, 2026-10-10): reached from the Wallets empty
  state's fourth way in, **Spend from a backup, step by step** ("A
  stick, words, or paper"); the Learn sheet's first row, **Spending,
  step by step**; Home's lead when a spend started in it is under
  way; and a wallet card's **Spend from this wallet**, which opens it on
  that wallet past pages 1 to 3 (§7).
- Each page is a step card (`flow.rs`): one open, the others closed to a
  line saying what was done. A page that does not apply to the route is not
  in the column at all, so numbering closes over it.
- Whether a page is done is worked out from the app's state (a vault open,
  a wallet loaded, a spend loaded, signed, finished, written), not stored.
  So it is right after a lock, after work done on another tab, and after a
  stick visit.
- What the tab stores is the route what came in set, the page open and
  the names of the files that came in on page 3. That is not secret; it
  is kept across a lock with the settings (`memory.rs` `kept()`), so a
  lock for a stick visit or for idleness returns the person to the same
  page after Unlock, with those files still listed. Seeds come back by a
  vault or not at all. A page 4 kept from before maps to page 3.
- The Guided/Full switch does not apply here: the Spend tab always shows its text.

## 3. The pages

| # | Page | Routes | Controls on the page |
|---|---|---|---|
| 1 | How to spend bitcoin | all | the map; **Start** |
| 2 | Start from the stick | all | none; on the desktop app, a line saying it is the desktop copy |
| 3 | What do you have? | all | a vault in Files: its Unlock and list; **Scan the QR codes you have**, **Type the words you have**; **Not sure what you have?**; what came in; the next question; the wallet once it is worked out |
| 4 | Check the money is really there (guessed: Check the money is there, on the online computer) | all | the wallet as a QR; addresses, receive and change, more on request; **Yes, my money shows**; guessed: **No money shows** |
| 5 | Write the payment | all | none |
| 6 | Bring the transaction here | all | Scan its QR · Copy it from a stick (the lock cycle) · PSBTs already in Files |
| 7 | Read the transaction | all | Wallets' Transaction and Txid steps |
| 8 | Add the seeds that can sign | paper route, multisig | word entry per seed, Scan a SeedQR, the count |
| 9 | Sign it | all | Wallets' Signers, Sign and Collect steps, as the wallet needs |
| 10 | Send it | all | Wallets' Finish step: the finished transaction as a QR, or to the Outbox |
| 11 | Put everything away | all | Lock; the stick visit for the Outbox; Power off |

Pages 3 and 4 of the first build (*What are you holding?* and *Open the
wallet*) are one page since `docs/SCAN-TO-SIGN.md` §4; the sections
below keep their numbers. The route is not chosen: it is set from what
came in. **Vault** when a vault was unlocked here, **Paper** when a
wallet description came in (whole or rebuilt from split sheets),
**Words** when only seeds did.

## 4. The prose, page by page

"This page" is Faraday, the system stick is the
Faraday stick, and Faraday's vault and lock cycle are added. **Lead** is
always shown; **More** is behind *More about this*.

### 1 · How to spend bitcoin

The map, six stops, offline and online marked apart:

| Where | Stop | Line |
|---|---|---|
| Paper | What you have | The Faraday stick and its passphrase, or words written down, and often a sheet describing the wallet. |
| Offline | Open the wallet here | Start a computer from the Faraday stick and open the wallet. |
| Online | Check the balance | Put the wallet's description into Sparrow on an online computer. |
| Online | Write the payment | Make the transaction in Sparrow. It gives you a PSBT. |
| Offline | Sign it here | Bring the PSBT to Faraday, read what it does, and sign it. |
| Online | Send it | Take the signed transaction back to Sparrow and broadcast it. |

**Lead:** The words are the money. Never photograph them, and never type
them into a phone, a website or a password manager. The same goes for the
vault's passphrase.

**More:**
- Anyone who reads the words can spend the money from anywhere in the
  world, and it cannot be reversed. Where a wallet needs several lists,
  anyone with enough of them can. Treat every list as if it were the only
  one. No legitimate bitcoin company will ever ask for them: not a support
  line, not someone helping you, not an AI.
- Nothing you do here is irreversible until the last step, and that step
  does not happen in Faraday. Opening the vault, typing words, looking at
  addresses and reading a transaction move nothing and notify no one. You
  can stop and come back.
- There is no deadline; bitcoin does not expire. Rushing is how people get
  robbed, by scammers and by helpers acting in their own interest. Take
  your time, and find someone you trust.
- If any of this stops making sense, stop. Put the stick and the paper
  somewhere safe and get help from someone you would trust with the money
  itself. Waiting costs nothing.

### 2 · Start from the stick

**Lead, on the stick:** This computer is running Faraday from the stick.
It has no network, and it forgets everything when it is switched off.

**Lead, on the desktop app (`app.online`):** This is the desktop copy of
Faraday, which runs on an ordinary computer that has been online. It is
for testing. For real money, start a computer from the Faraday stick.
(The owner's answer 3: nothing is stopped.)

**More (how to start a computer from the stick):**
- Shut the computer down completely, not sleep or restart.
- Unplug any network cable.
- Plug the Faraday stick in, switch the computer on and press the boot
  menu key straight away, repeatedly. It is usually F12, F9, Esc or F2.
  The first screen often names the key for a second or two.
- Pick the USB stick from the list that appears. If no list appears, the
  key was wrong or came too late: shut down and try another one.
- If the stick is missing, or someone else has opened the envelope, do not
  use it. Do not use a stick you are unsure about: the words are the money,
  and taking longer costs nothing.

### 3 · What do you have? (`docs/SCAN-TO-SIGN.md` §4, 2026-10-11)

**Lead:** Scan or type everything in the envelope, one thing at a time,
in any order. Faraday works out the wallet from it.

The card, top to bottom:

1. **A vault, when one is found** in Files (the stick this computer
   started from brings its own in): headed **Unlock {vault}**, the
   locked vaults to pick from, Remove the stick, the passphrase,
   **Unlock**, then the vault's list and **Load**, as the vault route
   drew them. Then **Or bring in what is on paper**. With no vault in
   Files, nothing of the vault shows.
2. **Scan the QR codes you have** (the intake scanner) and **Type the
   words you have** (Add a key, returning here).
   Under them, while no wallet is chosen: **Open {file}** for each
   wallet description in Files, and **Copy files in from a stick** (the
   stick visit, with its notes when a stick is not in or a key is open,
   coming back to this card). Split sheets in Files are listed under What
   came in, and **Rebuild the wallet** shows once they add up. A
   description that arrives by camera while the card is open opens by
   itself.
3. **Not sure what you have?** shows and hides the list below.
4. **What came in**, a row each, newest last: `Seed · {fp}` (and its
   label), `Wallet description · {name} · {shape}`, `Part {k} · {have}
   of {n} keys in hand` for split sheets, `Key · {fp} · xpub` for a
   cosigner's public key, `Transaction · kept for later`.
5. A seed that is none of the described wallet's keys: `Seed {fp} is not
   one of {wallet}'s keys` and **Add its passphrase**.
6. **The next question**, worked out each frame from what came in:
   - one seed, nothing else: **Anything else in the envelope?** ("Another
     list of words, another QR code, a line starting wsh( or tr(, or a
     word kept apart from the list."), **Scan more**, **Type more
     words**, **That's everything**, which opens the single-key wallet
     as native SegWit, a guess (page 5);
   - two seeds or more and no description: **These seeds are one wallet
     with several keys.** (what a description looks like), **Scan the
     description**, **I can't find one**;
   - after I can't find one: the seeds' shape (M of N on two sliders, a
     box per missing key filled by scan, a key file or typing, the kind,
     the path, the first address) and **Make the wallet**, also a guess.
     While N exceeds the seeds here: "Without the description Faraday
     needs all {n} keys: their seeds or xpubs".
   - a description in (whole, rebuilt, or a vault's): none. The wallet
     opens and the column goes on to page 5; seeds already in fill its
     slots.
7. Once there is a wallet (the card's closed line says it once: its
   name and shape, or the shape alone when the name says it): a choice
   when several are loaded, then its name, shape, how many of its keys are here, receive address 0 and
   **Continue**. The card is done once a wallet is chosen.
8. The last refusal, and **Start over**.

**The intake scanner**, titled **Scan what you have**, with the
question's hint under the camera line, **Type the words instead** while
keys may load, and **Done**. It reads one code after another until Done:

| Read | Result |
|---|---|
| A SeedQR, CompactSeedQR or a code of BIP-39 words | The passphrase sheet; once it loads the seed, the camera comes back with `Seed {fp} read`. With a description in, a seed whose key with no passphrase is a missing key loads at once; `Key {n} · already here` for one here. |
| A seed that is none of a description's keys | The passphrase sheet with `{fp} is not one of {wallet}'s keys`; a passphrase that gives a missing key loads it, the plain choice is refused, and **Not this one** drops the words and returns to the camera. |
| A wallet description, whole or in UR/BBQr parts | Into Files; the wallet opens. `Wallet description read`. |
| A split sheet | Into Files; `Part {k} read · {have} of {n} keys in hand`, or the wallet rebuilds once they add up. |
| An xpub or similar | Into Files; it fills a box of the seeds' shape when one is open. `A public key: it shows the wallet but cannot sign`. |
| A SLIP-39 share or a codex32 string | Add a key's form for it, with the share in, returning here once the key is put together. |
| A PSBT | Into Files: `A transaction to sign · kept for later`. Bring the transaction here offers it; the spend does not start before. With one seed and nothing else, the single-key wallet of the kind it spends from opens: the transaction shows the kind, so it is not a guess. |
| Anything else | Named, as the Files scanner names it. |

**More:** a vault unlocks slowly on purpose; words are checked against
the 2048 words and the checksum; a description opens the wallet
read-only, and the words that sign come after the transaction is read;
split sheets go in one at a time, in any order.

**The list (Not sure what you have?), with Faraday's items added:**
- *A USB stick labelled Faraday, and a passphrase on a separate card or
  sheet:* the stick starts the computer and carries the vault, a locked
  file named like `vault.ofv`; the passphrase opens it. The vault holds
  the wallet and the keys that sign for it.
- *A square black-and-white pattern (a QR code):* the same information in
  a form a camera can read. Several numbered 1 of 5 and so on are one thing
  in parts: scan them one after another. Never scan a QR code that sits
  beside seed words into anything but Faraday.
- *Words numbered 1 to 12, 18 or 24:* the seed phrase. Often headed
  recovery phrase, seed words or backup, or stamped into steel.
- *One long line starting `wpkh(`, `wsh(`, `xpub`, `zpub` or `tpub`:* a
  descriptor or a public key. It shows the addresses and the balance but
  cannot spend. Do not post it publicly: it shows the wallet's whole
  history.
- *Several sheets each holding a few lines,* like `73C5DA0A: xpub…`, often
  under a heading such as "this sheet carries 2 of 3 wallet descriptor
  parts": one description split up on purpose. Nothing is missing: enough
  of the sheets, in any order, add up to the whole. A line such as
  `Policy: 2 of 3` says how many signatures the wallet needs.
- *A separate word or sentence kept apart from the list, called a
  passphrase or a 25th word:* if one exists, the words alone open a wallet
  that is real but empty, and the money is in the wallet the words plus
  the passphrase open. **This is not the vault's passphrase.** The vault's
  passphrase opens the vault file; this one changes which wallet the words
  open.
- *A short line starting `bc1`, `3` or `1`:* one address, not a backup.
- *A small device with a screen and buttons:* a hardware wallet. It is a
  way of using the words, not a replacement for them.

### 4 · Open the wallet

One card with page 3 since `docs/SCAN-TO-SIGN.md` §4: the vault's
Unlock and list, word entry, the seeds' shape and the description by
camera all happen on *What do you have?*.

### 5 · Check the money is really there

**Lead:** Load this same wallet in Sparrow on your
online computer to check the balance. The code below is the wallet's
description: enough for Sparrow to find every address the wallet uses and
add up what is on all of them. It cannot spend; that still needs the keys.

Then check that the first address Sparrow shows is character for character
the same as address 1 below. If it matches, two separate programs have
read the same backup and agreed, and the balance Sparrow shows is the real
one. If it does not match, stop: something was typed or scanned wrongly.

*(Controls: Show the wallet as a QR; the first three receive and three
change addresses; Show more; **Yes, my money shows**, which marks the
wallet checked this power-on.)*

**When Faraday guessed** (`docs/SCAN-TO-SIGN.md` §5): a wallet worked
out from seeds alone (one seed and That's everything, the Loaded view's
Spend as a single-key wallet, or seeds shaped without a description).
The title is **Check the money is there, on the online computer**; the
lead says words alone do not say which kind of wallet they made, to open
it in the wallet on the online computer and look at its balance, and
that Faraday makes the next kind if no money shows. Under it: the ways
tried, each `{kind} · no money`, struck; the line `Trying: {kind}, {no
passphrase | with a passphrase} · {k} of {total}`; the wallet QR with
"Open this in the wallet on the online computer" ("Open this one on the
online computer too" after the first); the addresses; **Yes, my money
shows**, the only way on, and **No money shows**, which makes the next
way in place of the wallet:

1. One key only, after native SegWit: **Is there a word or sentence
   kept apart from the list?** **Yes, type it** opens the passphrase
   sheet on the seed and tries native SegWit with it; **No** skips on.
2. Taproot (one key) or Taproot multisig (several keys; native SegWit
   multisig first).
3. Nested SegWit.
4. Legacy.

After the last: **None of these show money.** "The words may belong to a
wallet with several keys, or need a different passphrase." **Scan more**
and **Type a passphrase** drop the guessed wallet and go back to page 3,
with the camera or the seed's passphrase sheet. A wallet from a
description or a vault was not guessed: its page is as above.

**More:**
- In Sparrow: File → New Wallet, give it a name, then the way in that
  scans a QR code or imports a descriptor; hold this screen up to the
  webcam. Menu names change between versions. (Checked 2026-10-06
  against sparrowwallet.com/docs/airgapped-wallet-qr.html, which names
  File → New Wallet and Scan…; it does not name an Output Descriptor
  import, so the text no longer does.)
- If the code is animated, let it loop once.
- Only load it into wallet software you chose and trust.
- Faraday can work out every address the wallet owns, but not which were
  used. Money is often spread over several addresses, including change
  addresses from old payments, so the first few can be empty while the
  money is further down. Sparrow adds them all up.
- On a phone you can also look an address up on a block explorer such as
  mempool.space. That moves nothing; it does tell the website you looked.

### 6 · Write the payment

**Lead:** This part happens on the online computer. In Sparrow, open the
wallet's Send tab, enter the address to pay and the amount, and choose a
fee. Press **Create Transaction**, then **Finalize Transaction for
Signing**, then **Show QR**: an animated code, which is what Faraday reads.
Sparrow can also save it as a `.psbt` file. (Button names as Sparrow's QR
guide gives them; the signed copy goes back with **Scan QR** and
**Broadcast Transaction**.)

**More:**
- Paying into an exchange: the exchange gives you a deposit address for
  bitcoin in your account. Copy it exactly; compare the first and last few
  characters after pasting.
- Faraday may lock while you are away; it locks after 30 minutes without
  input. That is expected. Unlock again with the same passphrase and this
  tab opens where you left it.

### 7 · Bring the transaction here

**Lead:** Two ways. Scanning is the simpler: nothing has to lock.

- **Scan it.** Press Scan and hold Sparrow's QR up to this computer's
  camera. An animation is read as it loops; the count shows the parts.
- **On a stick.** Save the `.psbt` from Sparrow onto a stick (the Faraday
  stick will do). Faraday never reads a stick while a vault is open, so it
  locks first: it seals the vault, forgets the keys and restarts. Then copy
  the PSBT in, remove the stick, and unlock with the same passphrase. This
  tab comes back on this page with the PSBT ready.

*(Controls: Scan; Copy it from a stick, which explains the lock in one
line and then locks; any PSBTs already in Files, each with Use this one.
The page closes when a spend is loaded.)*

### 8 · Read the transaction

**Lead:** Read the summary before you sign, every
time: what leaves, what comes back, which output is change, and the fee.
Change is the rest coming back to the same wallet, not money leaving.
Signing is the one step that cannot be taken back. If what Faraday
describes is not what you meant to do (a different amount, an address you
do not recognise, or a warning of any kind), do not sign. Declining costs
nothing.

Then the transaction id: write down its first and last few characters.
Sparrow shows the same id for the transaction it wrote; after sending, the
id is how you follow it.

*(Controls: Wallets' Transaction and Txid steps, as they are.)*

### 9 · Add the seeds that can sign (Paper route, multisig)

**Lead:** One seed at a time: put its words in, add
it, and the count goes up. Order does not matter and the same seed twice
does no harm. Each list is checked against the wallet's own keys, so one
belonging to a different wallet is refused. A seed kept as a QR code beside
the words does not have to be typed: scan it. Neither the words nor the
code carry a BIP-39 passphrase; if that backup has one, type it in the
passphrase box before adding.

**More:** Never photograph this screen. Never scan a seed QR into an
online device.

### 10 · Sign it

**Lead:** Signing adds the signature of every key loaded here. For a
wallet that needs signatures from elsewhere, the page lists who still has
to sign and takes their signed copies by scan or from Files; each is
checked against its key and this transaction before it counts.

**More:** If the envelope held a
hardware wallet that came with the papers and looks untouched, you can
sign the same PSBT on it too and compare. Follow the device's own guide to
restore from the same words. Compare what both screens say before
approving anything: the amount, the fee and the address. They must match.
Faraday on its own is enough to do the whole job.

### 11 · Send it

**Lead:** The finished
transaction is what the network accepts. Show it as a QR and scan it into
Sparrow, which broadcasts it. Or put it in the Outbox and carry it on a
stick (the stick visit writes it when Faraday next locks).

The finished transaction is safe to hand to anyone. Nobody can change
where the money goes without breaking the signatures, and a broken
transaction is rejected, not redirected. If you would like help with this
last step, this is the part you can safely hand over.

**More:**
- Any of these sends it: Sparrow (open the transaction from the QR, the
  file or pasted text, then Broadcast); a block explorer's broadcast page
  (mempool.space, blockstream.info); someone you trust.
- Then check the id. Whatever sends it should report the id Faraday
  showed. If the id you see anywhere differs, what was sent is not what you
  signed here: stop and find out why.
- And then wait. The first confirmation usually takes about ten minutes
  but can take hours if the fee was low. Exchanges usually want one to six
  confirmations before the money shows in the account.

### 12 · Put everything away

**Lead:** Lock Faraday. The vault is sealed again, and the keys are
forgotten. If the Outbox holds anything you still need (the finished
transaction, or a vault that changed), plug a stick in and the visit
writes it. Then switch off, take the stick out, and put it back with the
papers.

*(The vault changes when Faraday records the amounts it signed (the
signed-amount memory, `docs/WALLETS.md` §3.3). The page says whether the
sealed vault in the Outbox differs from the one on the stick, and that
writing it back is safe and optional.)*

## 5. Building it

| Part | Where |
|---|---|
| `Screen::Family`, the sidebar row, `Action::Family(FamilyAction)` | `faraday-core/src/lib.rs`, `screens.rs` |
| `FamilyState { route, open }`; which pages apply; each page's done line from app state | new `faraday-core/src/family.rs` |
| Drawing: the column, the map, the answer tiles, the More folds | new `faraday-core/src/family_screen.rs`, on `flow::column` |
| The text of §4 | new `faraday-core/src/family_text.rs`, one file, like `guide.rs` |
| Unlock's controls without its title and back link, called by Unlock and by page 3 | `vault_screens.rs`: `unlock()` split into `unlock_body()` |
| Pages 8, 10, 11 call Wallets' `step_body` for the spend's own steps; while one is open the Family card follows `spend.open` | `screens.rs` |
| Word entry, Restore and the stick visit return to Family when opened from it (as `after_visit` does for Create) | `lib.rs` |
| Route and page kept across a lock; after Unlock, Family opens if it was open | `memory.rs` `kept()` / `restore_kept()` |
| The words route's single-key wallet from the loaded key | `wallet.rs`, the descriptor Create builds for a single key |

No file in `core/` is edited; no new dependency.

**Tests** (`tests/all/family.rs`, user-facing behaviour, on the test kit):
- Vault route end to end: the test stick's `vault.ofv` and passphrase
  `test vault` → the wallet opens → a spend from Files → signed → finished,
  with the same signature bytes as Wallets › Sign.
- Paper route: split sheets in any order → wallet → seeds → signed; a seed
  from another wallet refused.
- Words route: the test words give the test kit's first address.
- A lock on page 7 (stick route) comes back on page 7 after Unlock, with
  the PSBT copied in loaded.
- On the desktop app, page 2 shows the stop.
- The review's warnings are the same text in Family as in Wallets.

The snapshot tour gains the Family pages at one size.

## 6. As built, beyond the proposal

- **Loading after the unlock.** The vault's own list appears on page 3,
  as on Files: every seed and wallet ticked, **Select all**, and a Load
  button that counts what it loads. Nothing loads before Load.
- **Partial vaults.** Page 3 lists every wallet the vault loaded with how
  many of its keys are here. The signing card (9, titled *Who signs*
  unless on the paper route) lists every key of the wallet: **Signs
  here** or **Not here**, and while short of signers **Scan a key**
  (any missing key, its words typed instead from its sheet) and
  **Unlock** for each other locked vault in Files
  (`docs/SCAN-TO-SIGN.md` §3). A key from another wallet is refused.
  A key held by someone else need not come at all: their signed copy is
  collected on *The other signatures*.
- **A vault holding keys but no wallet**: its seeds come in on page 3
  as seeds, and the next question asks for the rest; a description in
  Files opens by itself.
- **The wallet chooses itself** when there is only one, or when a PSBT
  in Files spends from one of those loaded.
- **Seeds alone** open nothing until the next question is answered
  (`docs/SCAN-TO-SIGN.md` §4.3); the single-key wallet is native SegWit,
  and Check the money walks through the other kinds (§5). A transaction
  that spends from another kind opens that kind on its own.
- **A description** opens the wallet as it comes in, by camera on
  page 3; after a lock, a description that came in opens again from
  Files.
- **A seed already loaded** typed or scanned again on page 3 comes in as
  it is.
- **The next page** after one is done is the first not done *after* it;
  a page skipped earlier stays where it is.
- **Send it** adds **Show the finished transaction as QR** to Wallets'
  Finish step.

## 7. From a wallet already open (2026-10-10, `docs/history/NEW-WALLET.md` §11)

A wallet card's **Spend from this wallet** (and **Sign the PSBT from
the stick**, **Carry on the spend**; the Loaded view's **Spend from
it** too) opens the tab on that wallet, titled "Spend from {wallet}"
with **Wallets** as its way back and no note. Pages 1 to 3 are not in
the column; page 5 is **Load the wallet in Sparrow** and pages 6 and 7
are one page, **Write the payment in Sparrow**:

| Page | Lead | Controls |
|---|---|---|
| Load the wallet in Sparrow | In Sparrow: File, New Wallet, then scan the wallet QR. Its first address is receive address 0 here. | **I have the PSBT from Sparrow ready: Scan it** (the camera; once the PSBT reads, its review); **Show the wallet QR** (the descriptor, full text under it); receive address 0; **It is loaded**. Done already for a wallet checked this power-on. |
| Write the payment in Sparrow | In Sparrow's Send tab: the address, the amount and a fee. Then Create Transaction, Finalize Transaction for Signing and Show QR. | **Scan the PSBT**; **On a stick** (the stick round trip); this wallet's PSBTs in Files. Done once a spend is loaded. |

Then the spend's steps as they are. **Who signs**, with fewer keys here
than sign, offers **Scan a key**, its Unlock buttons and **Collect a
signature**, which opens *The other signatures*; with enough, **Can
spend** (`docs/SCAN-TO-SIGN.md` §3). A PSBT
for this wallet in Files starts the spend at once, on the transaction's
review. Another wallet's spend gives way to this one, its PSBT staying
in Files. **It is loaded** does not mark the wallet checked: it says
Sparrow has the wallet, not that the addresses were compared. On a
small panel the shortcut is its line and **Scan it**. What the tab
keeps across a lock includes that it was entered from the card.
