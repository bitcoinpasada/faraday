# The Spend tab

**Status:** v0.2 · 2026-10-06 · built: `faraday-core/src/family.rs`
(state and logic), `family_screen.rs`, `family_text.rs`,
`tests/family.rs`, and the snapshot tool's `spend` tour
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
(`PLAN.md` §5, `docs/FLOWS.md`).

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
5. **The question is the page.** *What are you holding?* has big answers
   and no Next; the answer turns the page.
6. **The landing page is the map.** Short working text on each page; the
   longer explanation is behind **More about this** on the same page.

## 2. Where it sits

- `Screen::Family`. Not in the sidebar or the small Home grid
  (`docs/SIMPLIFY.md` §6.2, 2026-10-10): reached from the Wallets empty
  state's fourth way in, **Spend from a backup, step by step** ("A
  stick, words, or paper"); the Learn sheet's first row, **Spending,
  step by step**; and Home's lead when a spend started in it is under
  way. The tab's screen and code are unchanged.
- Each page is a step card (`flow.rs`): one open, the others closed to a
  line saying what was done. A page that does not apply to the route is not
  in the column at all, so numbering closes over it.
- Whether a page is done is worked out from the app's state (a vault open,
  a wallet loaded, a spend loaded, signed, finished, written), not stored.
  So it is right after a lock, after work done on another tab, and after a
  stick visit.
- What the tab stores is only the route chosen and the page open. That is
  not secret; it is kept across a lock with the settings (`memory.rs`
  `kept()`), so a lock for a stick visit or for idleness returns the
  person to the same page after Unlock.
- The Guided/Full switch does not apply here: the Spend tab always shows its text.

## 3. The pages

| # | Page | Routes | Controls on the page |
|---|---|---|---|
| 1 | How to spend bitcoin | all | the map; **Start** |
| 2 | Start from the stick | all | none; on the desktop app, a line saying it is the desktop copy |
| 3 | What are you holding? | all | four answers |
| 4 | Open the wallet | all, differs | Vault: the vault list, Remove the stick, passphrase, Unlock, what opened · Words: Type the words, Add another seed; with two seeds or more, M of N on two sliders, the cosigners' xpubs, the kind and the path · Paper: Restore (scan, type, sheets) |
| 5 | Check the money is really there | all | the wallet as a QR; addresses, receive and change, more on request |
| 6 | Write the payment | all | none |
| 7 | Bring the transaction here | all | Scan its QR · Copy it from a stick (the lock cycle) · PSBTs already in Files |
| 8 | Read the transaction | all | Wallets' Transaction and Txid steps |
| 9 | Add the seeds that can sign | paper route, multisig | word entry per seed, Scan a SeedQR, the count |
| 10 | Sign it | all | Wallets' Signers, Sign and Collect steps, as the wallet needs |
| 11 | Send it | all | Wallets' Finish step: the finished transaction as a QR, or to the Outbox |
| 12 | Put everything away | all | Lock; the stick visit for the Outbox; Power off |

Routes: **Vault** (a Faraday stick and its passphrase), **Words** (words
only: one key, or several lists of words and perhaps a line like 2 of 3),
**Paper** (words and a wallet description).

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

### 3 · What are you holding?

**Lead:** Lay out everything in the envelope first. Several items is
normal and does not mean anything is missing.

Answers:

- **A Faraday stick and its passphrase.** The stick this computer started
  from, or another one with a vault on it, and a passphrase written down
  apart from it. → Vault route.
- **Words, and nothing else.** Twelve, eighteen or twenty-four ordinary
  English words, numbered, in a fixed order, and nothing else that looks
  like a long code, a block of letter-and-number lines, or a QR code. A
  single list of words is usually the whole wallet. Several lists, perhaps
  with a line like 2 of 3, are one wallet of several keys and go here too.
  → Words route.
- **Words and something else.** As well as the words there is one long
  line starting `wsh(` or `wpkh(`, or lines pairing short codes with
  `xpub`s, or QR codes meant to be scanned, perhaps spread across several
  sheets. The wallet needs more than one key, and its description goes in
  before the words. → Paper route.
- **I am not sure.** Opens the list below.

**The list, with Faraday's items added:**
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

#### Vault route

The Unlock controls, in four parts, each closed to a line once done.

1. **The vault.** Lead: The vault came in from the stick this computer
   started from. It is listed here, locked. If it is on another stick, plug
   that stick in and copy it in. *(Controls: the locked vaults with where
   each came from; Copy one in from a stick, which opens the stick visit
   and returns here.)*
2. **Remove the stick.** Lead: Faraday opens a vault only with no stick
   attached. Take the stick out now; keep it with the papers. *(Clears
   itself when the last stick goes.)*
3. **The passphrase.** Lead: Type the vault's passphrase exactly as it is
   written, capital letters and spaces included. Unlocking takes about
   {time} on this computer; the wait is on purpose and makes guessing slow.
   More: A wrong passphrase opens nothing and says so; try again as often
   as you like. A vault can have more than one passphrase, and each opens
   its own contents. If what opens has no wallet in it, check the
   passphrase against the paper again.
4. **What opened.** Lead: the wallet's shape in a sentence, which of its
   keys can sign here, and its first address. "Compare this address with
   the one Sparrow shows on the next page." If the vault holds keys but no
   wallet: "The vault holds a key but not the wallet it belongs to. Load
   the wallet's description from the paper," which opens the Paper route's
   Restore controls here. If it holds several wallets, a choice of which
   one to spend from.

#### Words route

**Lead:** Type the words in the order they are written,
separated by spaces. Spelling and order matter; nothing else does. Faraday
checks them against the list of 2048 words and the backup's own checksum,
so a mistyped or missing word is caught here instead of opening a wallet
that is real, empty and not theirs. If a separate passphrase came with the
words, put it in the second box exactly as written. If there was none,
leave it empty. If you are unsure, try empty first.

*(Controls: Type the words, which opens word entry and returns here; Scan
a SeedQR. The wallet opened is native SegWit, account 0, and the page
stays open with **Add another seed** and **Continue**.)* More: If Sparrow
shows nothing on page 5, the wallet may be an older kind; **Try another
kind** lists Taproot, nested SegWit and legacy for the same words.

**Several seeds** (2026-10-08, the owner's request): for someone whose
paper holds several lists of words and a wallet policy written by hand,
with no descriptor. **Add another seed** opens word entry again (or Scan a
SeedQR, a seed from an open vault, one already loaded). With two seeds or
more the single-key wallet goes and the page lists the seeds by
fingerprint, then the wallet's shape: **M of N** on two sliders, the
number of keys (the seeds in hand up to 15) and the signatures needed (1
up to that), each moved by tap, drag or the arrow keys; a box for each key
whose seed is not here, filled with its account key (`xpub`, `zpub`,
`Zpub`, `tpub`, with or without its key origin) by Scan, a key file in
Files, or typing; the kind (native SegWit multisig by default, nested,
legacy, Taproot); the path (the kind's standard path with an account
number, or a custom path); the first address; **Make the wallet**. The
wallet is the descriptor Create a wallet writes over the seeds' keys at
that path, and the tab goes on to page 5 with it. The same piece is
Restore's seeds-first route (`docs/WALLETS.md` §2), `seeds.rs`.

More: the number of keys and how many sign are often written as 2 of 3;
a cosigner's key goes in as its xpub; if Sparrow shows nothing, try
another kind or account.

#### Paper route

**Lead:** Scan or type the long code
exactly as it is written, including everything after the `#`. It cannot
move money: it opens the wallet read-only, which is enough to see every
address and to check the balance. The words come later.

More: If it came in pieces, do one sheet at a time. Scan or type what one
sheet gives you; Faraday says something like "3 of 5 parts collected" and
waits. Order does not matter, a sheet entered twice does no harm, and the
wallet opens by itself when the last part arrives. If the sheets do not
say how many signatures are needed, Faraday asks; the number is printed at
the top of the sheet, as 2-of-3 or similar.

*(Controls: Scan the QR; Type it; a description in Files; the parts
collected and still missing.)*

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
change addresses; Show more.)*

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
- Faraday may lock while you are away; it locks after 5 minutes without
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
| Unlock's controls without its title and back link, called by Unlock and by page 4 | `vault_screens.rs`: `unlock()` split into `unlock_body()` |
| Pages 8, 10, 11 call Wallets' `step_body` for the spend's own steps; while one is open the Family card follows `spend.open` | `screens.rs` |
| Word entry, Restore and the stick visit return to Family when opened from it (as `after_visit` does for Create) | `lib.rs` |
| Route and page kept across a lock; after Unlock, Family opens if it was open | `memory.rs` `kept()` / `restore_kept()` |
| The words route's single-key wallet from the loaded key | `wallet.rs`, the descriptor Create builds for a single key |

Nothing upstream is edited; no new dependency.

**Tests** (`tests/family.rs`, user-facing behaviour, on the test kit):
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

- **Loading after the unlock.** The vault's own list appears on page 4,
  as on Files: every seed and wallet ticked, **Select all**, and a Load
  button that counts what it loads. Nothing loads before Load.
- **Partial vaults.** Page 4 lists every wallet the vault loaded with how
  many of its keys are here. The signing card (9, titled *Who signs*
  unless on the paper route) lists every key of the wallet: here, or
  **Type its words** for that slot, **Scan a SeedQR**, or **Unlock** for
  each other locked vault in Files. A key from another wallet is refused.
  A key held by someone else need not come at all: their signed copy is
  collected on *The other signatures*.
- **A vault holding keys but no wallet** shows the paper route's
  controls on page 4.
- **The wallet chooses itself** when there is only one, or when a PSBT
  in Files spends from one of those loaded.
- **The words route** opens native SegWit and lists all four single-key
  kinds with each one's first address; a transaction that spends from
  another kind opens that kind on its own.
- **The paper route** opens a wallet file or the split sheets in Files,
  and does so by itself when one arrives by camera on the tab.
- **The next page** after one is done is the first not done *after* it;
  a page skipped earlier stays where it is.
- **Send it** adds **Show the finished transaction as QR** to Wallets'
  Finish step.
