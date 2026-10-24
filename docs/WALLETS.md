# The Wallets tab

**Kind:** Reference
**Status:** Built

The Wallets tab does every piece of cryptography and
every format with `core/`'s library crates (`faraday-psbt`, `faraday-bip`), and
extends its flows to every wallet kind they can sign for.

## 1. Architecture

`faraday-core` names `core/`'s items directly: a wallet, a key
slot, a key that can sign, a reviewed transaction, a signature set are
Wallets-tab types that wrap `faraday-psbt`, `faraday-bip`, `faraday-codec` and
`faraday-crypto`. It also holds a capability table: for each wallet kind,
whether `core/` can build it, load it, sign for it, and in how many
rounds; the shared steps of §3, each a declared group of fields, a
summary line and a result, rendered by `faraday-ui` as step cards on
`wide` and pages on `small` (`docs/history/PLAN.md` §9.1); and flows as ordered lists
of steps per wallet kind (§4–§6), the reassembly model (§2), and the
Wallets tab's own non-cryptographic logic: the split-backup plan, the
two-round amount memory, the descriptor QR plan, the plain-sentence
review. Its wording is in the one strings file.

**Rules for the Wallets tab and `core/`:**

1. **Faraday maintains `core/` itself** (F65; no change loosens a
   check). The Wallets tab runs no app code from anywhere else.
2. **A variant of a `core/` part is a change to it**, not a copy: a
   thing is fixed at its source (`CLAUDE.md`).
3. **`faraday-core` names `core/` types directly**, so if something the
   Wallets tab uses is renamed or changed, `faraday-core` stops
   compiling, and its errors are the complete list of what to adapt.
4. **A new scheme is a row, not a rewrite.** When `core/` gains a
   wallet kind or a signing protocol (PSBT version 2, anti-exfil, the MuSig2
   fallback tree), the capability table gains a row and an adapter, and
   the flows gain one assembled from existing steps, plus a new step
   only where the protocol has a round no other scheme has.
5. **Shared logic lives in `core/`.** Code that defines
   bytes, or answers a question with no screen in it, is a `core/` crate.
   That put the KDBX writer and the
   Argon2id `Cost` in **`faraday-backup`**, the signed-message text form and a
   transaction with its previous outputs in `faraday-psbt`, and address parsing
   and a wallet's address search in `faraday-bip`. The Wallets tab therefore
   depends on no app code. Moved into `core/` when the Wallets tab needs
   them: the Tools calculators, the self-test's
   vector set, and `inspect.rs`'s reading of key origins through `faraday-draw`.
   Codex32 and SLIP-39 share handling and the MuSig2 session state are
   checked against the same rule before the Wallets tab uses them.
6. **One check after every merge.** `just` runs `core/`'s tests, then
   the Wallets tab's flow tests, then the conformance corpus (§8).

**What the Wallets tab lacks at first, and when it comes back.** The key
explorer, vanity addresses, silent payments, the Lightning
node key, SLIP-39 and codex32 backup screens and Learn are not part of the
Wallets tab on day one; they
return as Wallets-tab tools in the order of §9.

## 2. Reassembling a wallet

Most of the time a person here holds part of a wallet: one key of a 2-of-3
in a vault, the wallet's description on a backup sheet, the other keys on a
SeedSigner and a Coldcard. The Wallets tab's central object is therefore a
**wallet in progress**, which is valid at every stage of being put together.

**A wallet card** shows, at every point:
- the wallet's shape in a sentence ("2 of 3, native SegWit multisig");
- one row per key slot, in the descriptor's order, each marked **can sign
  here** (with the vault it came from), **watch only**, **signed this
  transaction**, or **missing**;
- the threshold as pips: how many signatures this transaction has, and how
  many it needs;
- the first receive address, to compare with the online wallet;
- what is still needed, as one line ("Add 1 more key or collect 1 more
  signature").
- its first and primary button, from the vault to a spend
  (`docs/history/NEW-WALLET.md` §11.2, 2026-10-10): **Spend from this wallet**,
  which opens the Spend tab on it at **Load the wallet in Sparrow**;
  **Sign the PSBT from the stick** with a PSBT for this wallet in Files,
  which opens its review; **Carry on the spend · Signatures 1 of 2**
  while this wallet's spend is under way, which resumes it where it
  was;
- its other buttons: **Rename** and **New wallet from this one** beside
  its name (any kind Create makes but a threshold wallet;
  `docs/history/NEW-WALLET.md` §16.2, 2026-10-19; in the foot when a long name
  leaves no room, and in the button list on a small panel), then along
  the foot **Show wallet QR**, **Back up**, **Sign a message** where
  the wallet can, and **Remove from session** at the end. New wallet
  from this one opens Create filled from the wallet at the next account
  its keys and kind do not use (§5).

**Every way in leads to the same card:**

| Starting from | What happens |
|---|---|
| **A vault holding a wallet and one of its keys** | At unlock the wallet loads into the Wallets tab with its key attached. The card opens at "1 of 2 signatures possible here." Nothing to do. The boot sheet's Unlock lands on Wallets with "Savings loaded from vault.ofv", the first wallet loaded that can sign here selected (`docs/history/NEW-WALLET.md` §11.1). |
| **A key with no wallet** (from a vault, or added on its own) | Listed under **Keys without a wallet**. With no wallet loaded the card offers **Make a wallet from this key** (Restore's Seeds card with the key in its slot: single key, native SegWit, account 0), **Add another key** (the same on a 2-of-2 multisig) and **Load or restore a wallet**; with wallets loaded the key's row carries **Make a wallet from this key**. |
| **A wallet description arriving** (scan, file, Files; any form in §3.2) | The Wallets tab derives each vault key's account key at the descriptor's paths and compares, attaches every match, and marks the rest watch-only. "This wallet uses 73c5da0a from Main" says which matched. **Save the wallet to a vault** is offered. |
| **Pieces of a description** (split sheets, key lines, a partial config, SeedSigner exports one at a time) | The card exists from the first piece, with the quorum unknown or stated, and lists fingerprints in hand and still missing. It completes when the last piece arrives. Overlapping pieces deduplicate. |
| **Seed words alone** (one list or several, perhaps a line like "2 of 3", no description) | Restore (`docs/history/NEW-WALLET.md` §12): **Kind** (one key: native SegWit ticked; several: Multisig · native SegWit; More kinds for the rest), **Quorum** for a multisig (M of N on two sliders, 2 of 3 to start; tap, drag or the arrow keys), **Description** › **I do not have it** (**I have the seed** for a single key), then **Keys**: a slot per key. The first empty slot takes its seed through **Type the words** (Add a key, which returns here, with its BIP-39 passphrase), **Scan a SeedQR**, a seed from an open vault or a key already loaded, each of the last three asking on a sheet whether it has a passphrase (typed twice; `docs/FLOWS.md` decision 52); on a multisig a slot can take **Its xpub** instead (Scan, a key file in Files, or typed: `xpub`, `zpub`, `Zpub`, `tpub`, with or without origin). **Make the wallet** builds the descriptor Create a wallet writes (`create.rs`) over the seeds' keys at the kind's standard path, loads it with the seeds in their slots, and opens Check: the descriptor and the first addresses. Done offers **Back up this wallet**, **Spend from this wallet** and **Wallets**. The Spend tab's words route keeps the older piece (`seeds.rs`), with the kind and path chosen after the seeds. |
| **A description first, then its seeds** | Restore's Kind offers **I have the wallet description: Scan it** and **From Files**. A description read there, or on Description, fills Kind, Quorum and the cosigners' keys and opens Seeds, a slot per key of the description. Each seed entered (with the slot's passphrase, if any) is matched to its keys: a match reads "Key 2 of the wallet"; any other seed, or the right words with the wrong passphrase, reads "Not a key of this wallet: check the words or the passphrase" and is not loaded. A key whose seed is elsewhere is marked **Not here**. |
| **A transaction arriving first** | The Wallets tab reads its key origins and finds the wallet it belongs to among loaded wallets, then among vault keys. "This transaction spends from Savings; you can sign with 73c5da0a." With no wallet known, it can still sign from the PSBT's own key origins, but change cannot be verified, and the review says so and offers **Load the wallet**. |

**Adding a key to a slot.** A wallet short of keys has one **Scan a key**
(`docs/history/SCAN-TO-SIGN.md` §2), which takes any of its missing keys and
works out which; its words can be typed instead, and a key an open vault
holds loads from it. A key from no slot of this wallet is refused with
its fingerprint. A
backup carrying a BIP-39 passphrase gets the passphrase field beside the
words, because no SeedQR carries one.

**Keeping what was assembled.** Keys and wallets added in a session are in
memory only. The card shows **Save to vault** for each addition (a secret
key with a hold; a wallet or watch-only key without), and locking lists
what was not saved before it seals.

## 3. Shared steps

These are the elements the Wallets tab's flows repeat. Every flow in §4–§6
is a list of them.

| Step | What it does |
|---|---|
| **Kind** | Choose the wallet kind, from the capability table: single-sig (four script types), multisig, Taproot multisig, MuSig2, FROST, recovery (timelocked paths). Imported-only kinds (miniscript, Taproot tree, watch-only) appear only on the load path. |
| **Quorum** | M and N, with a plain sentence of what that quorum means. Fixed at N of N for MuSig2. |
| **Slots** | One positional row per signer. Removing signer 2 leaves slot 2 empty rather than renumbering. Sources: a key in a vault, a new key (dice or entropy), typed words, a scanned SeedQR, an account key (watch only), or **someone else will add this later**. Each filled slot has **Show key QR**, as a `ur:crypto-account` byte-identical to SeedSigner's, or as the plain key expression. |
| **Load** | A wallet in any form (§3.2). Builds the wallet card, routing itself to the right kind. |
| **Signers** | The wallet card's slots, filled for signing (§2). |
| **Check** | First addresses, receive and change; the wallet as a QR for the online wallet (one code or animated, by the plan in §3.2). An address is checked against every loaded wallet's first 100 receive and change addresses by **Check an address** (Tools, Transactions). |
| **Transaction** | A PSBT by scan, a file in Files or typing. The review in sentences with the full table one choice away; every warning from one function, never softened for guided mode (§3.3). |
| **Txid** | The transaction id before anyone signs, for native SegWit and Taproot spends; refused for legacy and P2SH-wrapped inputs, whose id changes on signing. |
| **Sign** | Signs with every key held, producing one partly signed PSBT per key, the form separate devices would give. |
| **Collect** | **Signatures for this transaction**: cosigners' PSBTs by scan, file or Files. Each signature is verified against its key and this transaction's sighash; a PSBT for another transaction is refused, never merged. The collection belongs to the transaction, so reviewing again keeps it. |
| **Finish** | Two results: the **signed PSBT**, not finalised, for the wallet that wrote it (`ur:crypto-psbt`); and the **finished transaction** with its txid in a box of its own and **Decode this transaction**. Finalising may fail one signature short; the signed PSBT is shown anyway, labelled with what is still missing. |
| **Backup** | The four steps of §5. |
| **Save** | Wallet and keys to a vault: which vault, each key's "load at unlock" flag. |

### 3.2 Wallet forms read

Through `faraday-codec` and `faraday-bip`: descriptors, BIP-388 policies, the
Coldcard/Sparrow/Nunchuk/Specter multisig config (whole or partial), BSMS
records, key lines in every form a keyholder may have (`73C5DA0A: xpub…`,
`[73c5da0a/48h/0h/0h/2h]xpub…/<0;1>/*`, a bare xpub, flagged because it
cannot be matched to a PSBT), Coldcard's JSON account export, SLIP-132
keys, `ur:crypto-account`, `ur:crypto-hdkey` and `ur:crypto-output`
(`docs/QR.md`). Sparrow's wallet `.json` is read here. A
descriptor is kept exactly as it arrived, never re-emitted from its keys,
so `multi()` stays `multi()`.

### 3.3 The review's checks

From `faraday-psbt`'s inspection, plus the Wallets tab's additions, which are
non-cryptographic and live in `faraday-core`:

- **Change** is labelled only when it re-derives from the wallet's keys;
  an output that claims to be change and does not is **change, unverified**,
  and needs explicit confirmation.
- **Amounts** from a full previous transaction must hash to the outpoint
  and agree with any `witnessUtxo`; an amount that is only stated is
  **amount unchecked**.
- **A SegWit v0 input without its previous transaction, in a transaction
  with more than one input,** is refused, with no override
  (`faraday-psbt`'s `AmountUnverified`, a blocked warning). BIP-143 signs the
  amount an input states, so two signing rounds over two different
  stated amounts give two valid signatures that pay the difference to
  the miner (the 2020 two-round attack). The fix is the previous
  transaction, in the PSBT or brought in through Files. A single-input
  transaction is signed from its `witnessUtxo`: a false amount there
  gives a signature no node accepts. Taproot inputs commit to every
  amount and need no previous transaction.
- **Signed-amount memory**: the amounts each input stated are remembered
  against the unsigned transaction's hash. The same transaction stated
  differently is refused with no override. The memory is kept in
  `/run/faraday`, outside the app process, so it survives a lock; with
  a vault open it is also sealed into that vault (on by default, a setting),
  so it survives power-off. Decided 2026-10-04.
- **Add the previous transactions**: raw transactions typed, scanned or
  read from Files, each kept only if it hashes to an outpoint being
  spent. It opens itself when the transaction needs it.
- **Anything other than `SIGHASH_ALL`** stops, and those inputs are refused.
- Fee, dust, network, unknown paths and the deterministic nonce check
  are shown.

## 4. Spending

The Wallets tab's order, kept for every kind: **load the wallet → the
transaction → check → the signers → sign → collect → finish.** Check is
open at entry only the first time a wallet (by descriptor checksum) is
checked this power-on; once Continue has been pressed on it (here, on
Create's own Check, or "Yes, my money shows" on the Spend tab), it stays closed
and done across a lock and the next process, open again only in a fresh
one with no kept state (`docs/history/SIMPLIFY.md` §2.5). Everything before the
signers is watch-only, so any reason to stop is found before a key is
used. In Faraday the keys are usually already loaded from a vault, so
the signers step shows them rather than asking for them, and asks only
for what is missing. The transaction's id, where it is known before
signing, is a row of the Transaction card's table, not a step of its
own.

| Kind | Flow | What differs |
|---|---|---|
| Single-sig | Load (or the vault's wallet) → Transaction → Check → Sign → Finish | No Signers or Collect: the vault key is the wallet. |
| Multisig (`wsh`, `sh(wsh)`) | Load → Transaction → Check → Signers → Sign → Collect → Finish | The Wallets tab's flow as it stands. The txid row is refused for `sh(wsh)`. |
| Taproot multisig, `tr(H, sortedmulti_a)` | as multisig | Script-path signatures only (the internal key is unspendable); the review says which leaf. |
| Miniscript, Taproot tree (imported) | Load → **Path** → Transaction → Check → Signers → Sign → Collect → Finish | Signs every signature its keys can give; Finish completes only when the script is satisfied, and says which condition is unmet (a cosigner, or a timelock not yet reached). |
| Recovery (Liana shapes) | Load → **Path** → Transaction → Check → Signers → Sign → Collect → Finish | **Path**: the primary path, or a recovery path, with its timelock in blocks and whether the transaction's inputs have aged enough (from the PSBT's sequence values). |
| MuSig2, `tr(musig(…))` | Load → Transaction → Check → **Round 1: nonces** → Collect nonces → **Round 2: sign** → Collect partial signatures → Finish | Two rounds; every signer must take part. See §4.2. |
| FROST, m of n | Load (group record) → Transaction → Check → **Round 1** → Collect commitments → **Round 2** → Collect shares → Finish | Two rounds among the m chosen signers. See §4.2. Compiled out of the app (F109). |
| Message (any wallet with addresses) | Choose wallet and address → Message → Sign (BIP-137 or BIP-322) → Result as text and QR | Also **Check a signed message**. |

### 4.2 Rounds that hold a secret between them

MuSig2 and FROST make a secret nonce in round 1 and need it in round 2.
Locking restarts the app, so a nonce held in memory dies with any stick
visit. The app offers the first way only: rounds across a lock, and
FROST, are compiled out of the app until their standards mature
(F109), and what follows of them describes the code kept for then.
The two ways:

- **Both rounds by QR.** Round 1's nonce leaves as a code, the others'
  arrive by camera, and round 2 follows without a lock. This is the default.
- **Rounds across a lock.** The pending round is sealed into the vault
  that holds the key, bound to the unsigned transaction's hash, and marked
  used the moment round 2 signs. A nonce is never written anywhere else.
  The binding means an old copy of the vault restored later cannot sign a
  *different* transaction with a nonce already spent. It can still sign
  the same transaction again, and if the other signers' nonces changed in
  between that is a second signature under one secret nonce, which leaks
  the key (2026-10-10 audit): a restored round is for one answer from the
  cosigners, never a replay. The flow says this before it seals.

Every pass that may open a round draws its nonces from a seed of its own
(`Faraday::sign_seed`, the session's seed with a count of the passes), a session whose nonces have all
signed is dropped, and a pass is refused until the shell has answered
the first request for randomness (2026-10-10, after the audit found the
process's one seed reused: a nonce shared again was the nonce shared
before).

The vault's encryption holds the bound secret nonce.

## 5. Creating and backing up

| Kind | Flow |
|---|---|
| Single-sig | Kind (script type) → Keys (from a vault, new, typed, scanned) → Check → Back up |
| Multisig, Taproot multisig | Kind → Quorum → Keys → Check → Back up |
| MuSig2 | Kind → Keys (N of N; every key's account key) → Check → Back up |
| FROST | Kind → Quorum → Keys, whose Continue **deals** (on this device) → Check → Back up |
| Recovery | Kind → Primary keys → **Recovery paths** (up to three, each with keys and a timelock) → Check → Back up |

Create's cards are Kind · Quorum · Keys · Check · Back up (2026-10-10,
`docs/history/SIMPLIFY.md` §2.3). Create opens with Kind open, whatever opened
it — Wallets, Home, or a Tools tile with its kind already ticked
(2026-10-10, `docs/history/NEW-WALLET.md` §2.1, reversing `docs/history/SIMPLIFY.md` §2.1 for
this flow only); Kind's Continue closes it as done and opens the next
card. Keys' Continue makes the wallet and opens Check; there is no card
of its own for the descriptor. On the Keys card, a key chosen for one
slot — loaded here, or a cosigner's file — is not offered for any other
slot, until the slot is cleared (`docs/history/NEW-WALLET.md` §2.2); its foot
button reads **New key for Key N** (**Share N** on a threshold wallet)
while a slot is still empty, N the first one, and opens New key for it,
else **Continue** (`docs/history/NEW-WALLET.md` §2.3). Check shows the
descriptor as a summary row ("2 of 3 · native SegWit multisig ·
9a6a2580, cf0e9805, 048ab54e · #qf45pmyh"), which opens the wallet's code
with the descriptor's whole text under it, then the first addresses
under the section label **The wallet's first addresses**
(`docs/history/NEW-WALLET.md` §2.4), so a multisig's three rows do not read as
one per key. **Back up** is the plan's three presets; each opens Back
up a wallet on the wallet just made with that preset applied, its chip
"Then: Wallets", its way back the wallet's card. The seeds and the
wallet into a vault, the paper and the public files are the checklist's
items below, not cards of Create.

New key opened from a Create slot ends on its **Key** card
(2026-10-10, `docs/history/NEW-WALLET.md` §3): the randomness check is on the
entry card once the last entry is in, and Key shows the words, an
optional **Passphrase** typed twice, and **Lock in**, which adds the
key and fills the slot. On a single-key wallet Lock in also makes the
wallet, so Key shows its descriptor and first address and Continue
returns to Create on **Back up**, Keys and Check done; on a wallet of
several keys Key shows the key text and Continue returns to Keys. New
key in Create neither asks for the words to be written down nor
quizzes them: Back up shows them and checks the copy, and asks where a
passphrase goes. A slot filled by a key loaded here from BIP-39 words
without a passphrase has **Add a passphrase**, which locks in the key
those words and a passphrase make in its place.

Keys from a vault and a wallet from another (2026-10-19,
`docs/history/NEW-WALLET.md` §16, F45). Under each slot's session keys the Keys
card lists every open vault's keys not loaded, under the vault's name
(only 24-word shares for a threshold kind); a press loads that key and
fills the slot. A locked vault whose file is read is one **Unlock
{vault}** button, returning to the Keys card. A wallet card's **New
wallet from this one** opens Create filled from that wallet: Kind and
Quorum closed and changeable, Kind with an **Account** stepper starting
at the lowest account no loaded wallet of the same keys and kind uses,
every key here (or loaded from an open vault) at that account
(`NewKind::path_at`), a cosigner's xpub kept only at the wallet's own
account and script type and otherwise "Needs Key n's xpub at account
n", the name proposed "{wallet} · account n".

**Backup** is a plan, then a checklist of only what the plan needs
(2026-10-09, the owner's proposal A; it replaced five fixed cards that
each showed every option). Any loaded wallet can be backed up again,
not only the one just made: from the wallet's card (**Back up**, that
card's wallet), Start's **Back up a wallet** and the Tools tile (the
wallet last picked on Wallets), Create's **Back up** card (with the
preset chosen there) and Restore's **Back up this wallet**. With more than one wallet loaded, the flow's head reads
"Back up a wallet" and the wallet's name is a chip; a press on it lists
the loaded wallets, one row each with its shape, and a press on one
starts the backup again on that wallet. The plan's logic is `plan.rs`,
tested on its own; nothing is marked done that was not.

**1. Presets, then the plan by place** (2026-10-15, F35,
`docs/BACKUP-FLOW.md` §15). *Narrowed by F53 (2026-10-20,
`docs/BACKUP-FLOW.md` §17.21): a backup starts from The standard way
only, on the board; Paper only, Paper and vault, and Paper, vault and
watch-only software are offered nowhere, and vaults and the software
are added on the board.* The flow opened on three presets, one tap
each: **Paper only**, **Paper and vault**, **Paper, vault and watch-only
software**. Each filled every card below; every answer stays changeable.
The cards are step cards on the wide layout and a page each on the
small panel:

- **Places**: **Number of places**, a stepper, one or more with no upper
  bound; and, with a vault open, each place's name (below). One more
  adds an empty place; one fewer removes the last place and what it
  held.
- **Place 1**, **Place 2** …, one card per place, titled with its name
  where it has one: a multi-choice list (DESIGN §4.2) of what the place
  keeps, **Seed words by hand** · **SeedQR by hand** · **Vault 1**,
  **Vault 2** … (a row per vault of the plan, named as the vault set up
  for it is) · **Seed as a file, unprotected** (on the place's stick of
  files; the secret sheet's warning and "I understand", below) ·
  **Passphrase** (where a seed here has one). A wallet with no seed here
  has the vaults' rows alone. A press on a row opens its sheet, "{row} ·
  {place}": the seeds loaded here, by fingerprint, any number ticked,
  and **Done**; for a vault, the seeds the vault holds and, under each
  ticked seed with a passphrase, "{fingerprint} with its passphrase",
  and "Also at {place}" where another place keeps its stick (a vault is
  one vault wherever its stick is kept); for the passphrase, a seed
  whose words or SeedQR the place keeps is dimmed. Done with no seed
  ticked leaves the row unticked. A ticked row shows what it holds under
  its label; a press on its box unticks it (a vault's stick at this
  place; the vault goes with its last stick). Under the list, what
  finding the place alone gives, as the map words it.
- **The wallet description** (a multisig): **Whole** or **In shares**,
  with **What is a share?** beside In shares opening the Learn page
  *Shares of a wallet description* (`docs/learn/faraday/shares.md`); in
  shares, **Keys left off each share** on a slider from 0 to m − 1, the
  shares with their keys' fingerprints and the split audit's lines,
  drawn as the checklist's Shares card draws them.
- **Where the description goes**: a section per place, its forms **On
  paper** · **As a file** (on the place's stick of files) · **In
  {vault}** for each vault whose stick the place keeps; in shares,
  **Shares here**, Share 1 … Share n, which the forms ticked there hold.
  Then **Not a place**: **Into watch-only software**.
- **Software** (asked when the description goes to software or as files
  at a place): Sparrow (the wallet file) · Coldcard, Keystone, Passport
  (the multisig config) · Nunchuk (the BSMS record) · Bitcoin Core (its
  import) · Not sure (the descriptor), each the descriptor where the
  wallet has no such file; form: QR picture · text · both. This decides
  which public files are offered at all.
- **The plan**: the map, every spot, with the check lines under it,
  then **Make the checklist**, the plan's one way on. A vault's stick at
  no place, or a key made here kept nowhere, holds it there, in its
  place.

From the first answer on a card after the presets, the map is the plan
and every answer is an edit of it (`plan::edited`). A plan saved before
the cards (no `spot` lines) is read as it was and shown on the cards as
its map.

Defaults from the wallet: one key, paper words, two places, the sheet
in each, the descriptor as a QR picture; a multisig, a place per seed,
each with its seed and the whole wallet sheet, m − 1 keys left off each
share should shares be ticked; seeds held elsewhere are listed "On
its own device" and get no paper here; watch only, no seed question.
"Paper and vault" puts one seed in each vault, vault *v* holding the
*v*th seed here, and the wallet in every vault; the third preset adds
watch-only software (these two are plan builders since F53, not
offered). Each vault's stick goes, in turn, to the first
place that keeps no seed's words and no other vault's stick; failing
that, to the last place that keeps only that vault's own seeds' words
(a second copy of a key already there, never a second key). So no place
holds two different keys, even behind vault passphrases, where the
places allow it: a single key's vault stick at Place 2; a 2-of-3 with
two seeds here, Vault 1's stick at Place 3 and Vault 2's at Place 2. A
vault made later by a tick gets its stick the same way; one whose only
stick was at a place removed is given another.

**2. The map**, the side panel and The plan card (on the small panel
The plan card and the envelopes' item), redrawn as answers change: one box per place,
each vault made ("Vault 1 · 9a6a2580", by number and the seeds it
holds), a stick of unprotected files (from a plan saved before the
cards; files kept at a place are on that place's stick of files),
watch-only software and the seeds
on their own devices, each listing what it holds tagged secret,
sealed or public. Under it the check, found as `backup::audit` finds a
split's, by trying each place lost and each place found:

- **Any one place lost: the rest rebuild the wallet**: Yes, **Yes, with
  the vault's passphrase**, or No. Rebuilding takes a quorum of seeds,
  each with its passphrase, and for more than one key the description,
  whole or from shares that together hold every key. Watch-only
  software counts as a copy of the description; a seed on its own
  device counts as kept.
- **One place found: can spend**: Yes, **Only with the vault's
  passphrase** (the place keeps a vault's stick and that vault holds
  what is missing), or No. Seeds alone count: the description is public
  (`docs/BACKUP-FLOW.md` §17.22).
- **One place found: sees the balance**: the same three values; the
  whole description, or a one-key wallet's seed, shows it.

A place here is a paper place (its stick of files included: a seed's
file there counts as that seed there), a stick of files, or a vault's
own stick when no place keeps one; each vault is read, with its passphrase, from a
place that keeps its stick. Labels and values only; what each line
means is in Learn's Backups page.

**3. The checklist**, the plan made (**Make the checklist**): only the
items the plan needs, each done by what it does, never by a tap. An
item's foot button is its own next action until it is done, and only
then **Continue**; an item not done cannot be passed, and the items
after it open once it is done, each saying "After: {item}" meanwhile
(2026-10-10, `docs/history/NEW-WALLET.md` §14.1). Opened
from an action on the wallet's chart (2026-10-10, `docs/history/NEW-WALLET.md`
§9) it opens at that item with the plan the chart shows, and makes
nothing For the stick: each item's own **Make it** does. A change made on the chart (a
move, another copy, a thing destroyed, a place removed; `docs/history/NEW-WALLET.md`
§9.7) is an edit of the map, as the plan's cards make: the checklist is
the map's, and **Change the plan** opens the cards on it, nothing
replaced (2026-10-15, F35).

1. **Print N blank templates** (one per paper copy of a seed): printed
   from Faraday Starter (`docs/STARTER.md` §3.8), on the online computer,
   before the copies by hand (2026-10-14, F31): the body reads "Print {n}
   from Faraday Starter: Blank templates, {words} words, {copy}" and the
   foot is **Done**, a press, as the envelopes' is. No template is made or
   written to a stick here. The template holds no secret
   (numbered word lines where a seed is copied as words, path, network,
   the SeedQR grid's fixed squares where one is copied as a SeedQR;
   F29) and a second page of empty lines, "Place ____ holds ____",
   one per place, filled in by hand; nothing of the plan is printed.
2. **Copy seed {fp} by hand**, one per seed here: the words with their
   numbers, and the SeedQR grid only when the plan copies the seed as a
   SeedQR, behind hold-to-reveal, never a printer, no line for a BIP-39
   passphrase. Its button, **Check my copy**, checks each form the
   seed's copies have (F29, `docs/history/NEW-WALLET.md` §14.7; built, except
   Keep the words only and the grid drawn in part): a words copy by typing the whole seed back as words (or
   their four-digit numbers, or four-letter prefixes) with the seed
   not on screen, compared once the last word is in and the first word
   that differs named; a SeedQR copy by **Scan my copy** (the camera
   reads the drawn SeedQR, compares its words with the seed in place,
   never loaded, names the first word that differs), by typed words
   where there is no camera. Done when every form's check has matched.
   **Keep the words only**, on a SeedQR copy's item and at its foot
   after three scans without a match, turns that seed's SeedQR copy
   into words by hand; the grid drawn so far stays on the map, marked
   drawn in part, until destroyed. Seed XOR and codex32 are here too.
3. **Vault 1: {fp}**, one item per vault made, its body the rows one
   press will save ("Seed {fp}", "Seed {fp} with its passphrase",
   "Wallet description"), each marked once the open vault holds it. Its
   one button: with no vault open **Make a vault**, **Unlock {name}** or
   **Unlock a vault**, which come back to this item; then **Save into
   {vault}**, which saves in one press each of its seeds (the key record
   Vaults' Save writes, loaded at unlock), with its passphrase where the
   plan keeps it there, and the wallet where the plan puts it there
   (owner, 2026-10-10: no seed by seed, nothing left out); then
   **Continue**. An open vault that holds a seed only another vault is
   to hold is not offered: "{vault} holds another vault's seed", **Lock
   it and make a new vault** (that vault alone is sealed For the stick
   and closed). Done when an open vault, or one locked since and
   remembered, holds every seed of the item (with its passphrase where
   planned), the wallet where planned, and no other vault's seed.
4. **The seeds as files**: **Make it** opens the secret sheet for the
   first seed whose file is not made:
   "Save into {vault}" there writes the same key record; or, once "I
   understand: anyone who copies the stick or sees this file can spend
   these coins" is ticked, the words (`{wallet}-{fp}-words.txt`) or the
   SeedQR as a labelled PNG (`{wallet}-{fp}-seedqr.png` or
   `-compactseedqr.png`) go For the stick under Unprotected secrets,
   unticked on a stick visit; never the passphrase. Done when each
   seed's file is there.
5. **The wallet sheet** (descriptor, keys, first addresses, a PDF) or,
   split, **The shares**: each sheet omits at most M−1 keys, so any
   quorum rebuilds and no one share watches, with the measured minimum
   rebuild group; each share as its PDF, its text and
   `{name}-share-k-of-n.png`. Made For the stick with the checklist,
   with **Remove**; done while it is For the stick or on the receipt.
6. **The public files** for the software and form chosen, drawn from
   the wallet (descriptor, wallet file, multisig config, BSMS record,
   Bitcoin Core's import), with each key whose seed is here
   (`xpub-{fp}.txt` and its BIP 129 record) for the cosigners. The
   files the plan chose are made For the stick the moment the checklist
   is made (2026-10-10, `docs/history/SIMPLIFY.md` §4.2); each row reads "For the
   stick" with **Remove**, or "Written to {label}" once a visit wrote
   it, and **Make the file** where it is neither. Done when each chosen
   file is For the stick or on the receipt.
7. **Show the descriptor to the software**: the descriptor as one code
   or animated (`QrWallet`); done once shown.
8. **One envelope per place**, listing what goes in it ("Stick with
   Vault 2" names which vault's stick): done by **Done**, the one thing
   the device cannot see.
9. **Write to sticks**, last, where the plan has a stick to write (a
   vault's stick, the stick of files, or the public files and sheets to
   take to an online computer): the lock's Kept and Wiped from memory, a
   row for each stick by place with its files, ticked as written. Its
   foot **Lock and write** locks once; each stick is then written by
   place inside the backup flow (`docs/BACKUP-FLOW.md` §10), the stick
   visit page never shown. Done when every stick is written. The items
   that end For the stick read "Written to {stick}" once the flow wrote
   them.

A stick visit with more than one vault file For the stick ticks one by
default, the first the last write did not write; the rest wait for the
next stick, and the receipt names the vault file each stick got.

Under the list, **Change the plan** goes back to the questions; it is
the one way out of an item not done. Items the new plan still has stay
done; what a vault holds that the new plan drops stays on the map as
what it is ("vault.ofv holds seed 9A6A2580"). With every item done,
"Backup done", then the plan's three checks as a strip, each value
large in its colour, the wallet's chart of the plan (wallet first),
**Open the wallet**, **Change the plan** (2026-10-10,
`docs/history/NEW-WALLET.md` §14.6; the done card offers no **Write to a stick**
and nothing waits For the stick after it, F31). On the side panel each line of the map carries its
item's state, a dot in OK once done. The small panel's copy page keeps
the per-seed lines of `Faraday::backup_kept` (in which vault, copy
checked, a file for the stick unprotected, not here).

**The backup before the spend** (owner, 2026-10-10;
`docs/DECISIONS.md` F6, `docs/history/NEW-WALLET.md` §14). A key made here
(New key's words, or a passphrase added in Create) signs nothing until
a checklist that keeps its seed and passphrase is complete: the
wallet's card reads **Finish the backup first** (its checklist, or its
plan when none is made), and Who signs lists it "Waiting for its
backup" with **Finish the backup**. Its addresses carry "Back up before
you receive". The plan's first card says "A key made here signs only
once this backup is done."; a plan that keeps no copy of it says "Seed
{fp} is kept nowhere" in place of Make the checklist. Places' Continue,
while a vault made has no stick ticked, reads "Vault 1's stick needs a
place"; a plan whose check reads "Any one place lost: No" says why
under it on the map panel. Keys restored from words, a SeedQR or a
vault with nothing pending sign at once.

**Place names: in the vault only** (owner). A place may be named ("Home
safe") only with a vault open; the plan is then saved into it as record
type 11, "Backup plan" (`docs/VAULT.md` §7: the wallet, the answers,
each place's name, what each holds), when the checklist is made, over
the one it kept for that wallet, and the next backup of the wallet with
the vault open starts from it rather than the presets. A name is shown
on screen and never put in any file, sheet, PDF or PNG.
Without a vault the places are "Place 1", "Place 2". There is no
printed map: where things are is sensitive.

The vault is one more copy, not a replacement for paper. A file is the
least safe of the three.

### The Backups screen

**Backups** (`docs/history/SIMPLIFY.md` §5, `docs/FLOWS.md` "Backups") lists
every wallet the app knows, once each by descriptor checksum: loaded,
in an open vault, or remembered of a locked one. Under each, the map as
the plan draws it, a row per place with what it holds, its tag and what
this device saw of it: a paper place **Checked** only once a copy of
each seed it keeps matched here (the first check of a seed is the first
place's copy, the second the second); each vault ("Vault", or with
several "Vault 1", "Vault 2") by the file that holds its seeds and its
currency; public files on the stick the receipt names, For the stick,
or not made; the software shown or not; the seeds away "Not here". A
wallet with no plan reads **No backup plan** with **Back up**.

The record of what a plan holds is made when its checklist is made;
each matching check adds one copy. It holds the map's places, tags and
file names and the counts by fingerprint, never a place's name, and is
kept across a lock (kept state, `backups`) and gone at power-off. A
backup of the wallet opened again starts with its seeds checked. The
wallet card's line under its shape says the same in one line ("Backup:
paper ×2 checked · vault.ofv · for the stick", or "Not backed up") and
opens Backups on that wallet.

## 6. Guided mode

The same steps and the same engine serve someone inheriting bitcoin, so
the same paper gives the same addresses and byte-identical signatures,
and every warning is word for word the same, from the same function.
That path is the **Spend** tab (`docs/FAMILY.md`): its first question is
*What do you have?*, and its order, wording and folds are written there.

A switch beside each flow's title chooses **Steps only** or **Guided** (called Full until 2026-10-06); a wallet in
progress carries over between them. The choice is one setting for every
flow, kept with the others; Steps only is on the left, and is where a
first start is, on a device that has never had a setting saved (F82):
a working screen carries no explanation unless the person turns Guided
on.

## 7. Tools

Tools (`Screen::Catalog`, `faraday-core/src/catalog.rs`) is every flow
on one page, by what it works on, each tile with its line and the
standards it follows; **Find a tool** narrows the page to the tiles whose
name, line, group or standards hold what is typed. A tile opens its
flow, or names what it needs first ("Load a key first", "Make a vault
first"). The groups and their tiles:

| Group | Tiles |
|---|---|
| Keys and seeds | New key · Add a key · Scan a SeedQR · SLIP-39 shares · Restore from SLIP-39 · codex32 · Seed XOR · Explore a key · Child seeds and passwords (BIP-85) · Vanity address · Silent payments · Lightning node key |
| Wallets | Single-key wallet · Multisig · Taproot multisig · MuSig2 · FROST threshold · Miniscript and inheritance · Restore an existing wallet · Back up a wallet |
| Transactions | Sign a transaction · Nonce check · Decode a transaction · Check an address |
| Messages | Sign a message · Check a signed message |
| Vaults and other keys | Vaults · TOTP codes · Diceware passphrase · GPG key · Secure Boot keys · KeePass export |
| Calculators | Hashes · Encodings · Descriptor checksum · Convert an xpub · Units · Miniscript compiler |
| Word lists | BIP-39 words · EFF large list · EFF short list 1 · EFF short list 2 |
| QR | Scan a QR code |

## 8. Tests

- **Published vectors**: BIP-174's valid and invalid PSBTs
  (`tools/vectors/psbt/`), each invalid one refused, and the vectors of
  every BIP the wallets use (`tools/vectors/`), in `core/`'s tests.
- **Other wallets' files**: the exports of Coldcard, Sparrow, BlueWallet,
  Nunchuk, Caravan and others, taken from Sparrow's and BlueWallet's
  test suites (`faraday/faraday-core/tests/vectors/interop/`), read by
  `faraday/faraday-core/tests/all/interop.rs` to their stated first
  addresses or refused for the reason `docs/INTEROP.md` gives.
- **Flow tests** drive each flow through its steps for every kind in the
  capability table, from every entry point of §2.
- **`core/`'s own tests** run first, unchanged.

## 9. Order of work

Moved to `docs/history/WALLETS-LOG.md`.

## 10. Decided 2026-10-04

Moved to `docs/history/WALLETS-LOG.md`.
