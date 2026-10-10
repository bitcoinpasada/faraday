# The Wallets tab

**Status:** built 2026-10-05: every flow in §4 and §5 is in
`faraday/faraday-core`, for spending, creating, backing up, restoring,
messages and the camera — every single-key, multisig, Taproot multisig,
miniscript, tree, MuSig2 and FROST wallet with recovery paths
(`HANDOFF.md`, work queue).

The Wallets tab sits beside the OpenSigner tab, which stays until it is
robust enough to deprecate it. It does every piece of cryptography and
every format with OpenSigner's library crates (`osk-psbt`, `osk-bip`), and
extends its flows to every wallet kind OpenSigner can sign for.

## 1. Architecture, and staying downstream

`faraday-core` names OpenSigner's `osk-*` items directly: a wallet, a key
slot, a key that can sign, a reviewed transaction, a signature set are
Wallets-tab types that wrap `osk-psbt`, `osk-bip`, `osk-codec` and
`osk-crypto`. It also holds a capability table: for each wallet kind,
whether OpenSigner can build it, load it, sign for it, and in how many
rounds; the shared steps of §3, each a declared group of fields, a
summary line and a result, rendered by `faraday-ui` as step cards on
`wide` and pages on `small` (`PLAN.md` §9.1); and flows as ordered lists
of steps per wallet kind (§4–§6), the reassembly model (§2), and the
Wallets tab's own non-cryptographic logic: the split-backup plan, the
two-round amount memory, the descriptor QR plan, the plain-sentence
review. Its wording is in one strings file, as OpenSigner's is.

**Rules that keep Faraday downstream of OpenSigner:**

1. **No upstream file is edited.** The Wallets tab does not run
   `opensigner-core`, and the OpenSigner tab connects to the vault through
   its existing interface (below), so no vault bridge inside OpenSigner is
   needed. The workspace manifest is the one shared file, and its change
   is one line per new crate.
2. **Where Faraday needs a variant of an upstream part** (the stick's
   image boards, its shell, `osk-ui`), the variant is a copy under
   `faraday/` recording the upstream commit it was copied from.
   `just upstream-diff` lists every upstream change to those originals since
   then, so a fix upstream is seen and carried over deliberately.
3. **`faraday-core` names upstream types directly**, so if OpenSigner
   renames or changes something the Wallets tab uses, `faraday-core`
   stops compiling, and its errors are the complete list of what to
   adapt.
4. **A new OpenSigner scheme is a row, not a rewrite.** When upstream adds a
   wallet kind or a signing protocol (PSBT version 2, anti-exfil, the MuSig2
   fallback tree), the capability table gains a row and an adapter, and
   the flows gain one assembled from existing steps, plus a new step
   only where the protocol has a round no other scheme has.
5. **Shared logic lives upstream, in `core/`.** Upstream's rule since
   §16.138 (2026-10-04, made for forks like this one): code that defines
   bytes, or answers a question with no screen in it, is a `core/` crate, and
   `opensigner-core` holds only flows and wording. That move put the `.oskb`
   format, the KDBX writer and the Argon2id `Cost` in **`osk-backup`**, the
   kept-key blob in `osk-keep`, the signed-message text form and a
   transaction with its previous outputs in `osk-psbt`, and address parsing
   and a wallet's address search in `osk-bip`. The Wallets tab therefore
   depends on no app code. Still in the app, and asked for upstream when
   the Wallets tab needs them: the Tools calculators, the self-test's
   vector set, and `inspect.rs`'s reading of key origins through `osk-ui`.
   Codex32 and SLIP-39 share handling and the MuSig2 session state are
   checked against the same rule before the Wallets tab uses them.
6. **One check after every merge.** `just` runs upstream's own tests, then
   the Wallets tab's flow tests, then the conformance corpus (§8).

**The OpenSigner tab stays** beside the Wallets tab until the Wallets tab is
robust enough to deprecate it (decided 2026-10-04). It keeps zero upstream edits:
a vault key is opened in it by handing OpenSigner the key as a scanned
payload (`Event::Scanned`), which runs OpenSigner's own load flow, and a key
leaves it as an `.oskb` encrypted backup, which imports into a vault.

**What the Wallets tab lacks at first, and when it comes back.** OpenSigner's other
screens (the key explorer, vanity addresses, silent payments, the Lightning
node key, SLIP-39 and codex32 backup screens, Learn) are not part of the
Wallets tab on day one; the OpenSigner tab keeps them meanwhile, and they
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

**Every way in leads to the same card:**

| Starting from | What happens |
|---|---|
| **A vault holding a wallet and one of its keys** | At unlock the wallet loads into the Wallets tab with its key attached. The card opens at "1 of 2 signatures possible here." Nothing to do. |
| **A key with no wallet** (from a vault, or added on its own) | Listed under **Keys without a wallet**. With no wallet loaded the card offers **Make a wallet from this key** (Restore's seeds card with the key in, at 1 of 1, native SegWit, account 0), **Add another key** (the same at 2 keys) and **Load or restore a wallet**; with wallets loaded the key's row carries **Make a wallet from this key**. |
| **A wallet description arriving** (scan, file, Files; any form in §3.2) | The Wallets tab derives each vault key's account key at the descriptor's paths and compares, attaches every match, and marks the rest watch-only. "This wallet uses 73c5da0a from Main" says which matched. **Save the wallet to a vault** is offered. |
| **Pieces of a description** (split sheets, key lines, a partial config, SeedSigner exports one at a time) | The card exists from the first piece, with the quorum unknown or stated, and lists fingerprints in hand and still missing. It completes when the last piece arrives. Overlapping pieces deduplicate. |
| **Seed words alone** (one list or several, perhaps a line like "2 of 3", no description) | Restore › The wallet › **Type the seeds** opens The seeds in seeds-first mode (the Spend tab's words route has the same piece, `seeds.rs`). Each seed goes in through Add a key, which returns to the card: the seeds by fingerprint, **Add another key**, Scan a SeedQR, a seed from an open vault, a seed already loaded, and **Make the wallet**. That leads to the shape: M of N on two sliders (keys from the seeds in hand to 15, signatures from 1 to that; tap, drag or the arrow keys), an account key for each cosigner whose seed is not here (Scan, a key file in Files, or typed: `xpub`, `zpub`, `Zpub`, `tpub`, with or without origin), the kind (several keys: native SegWit multisig by default, nested, legacy at m/45', Taproot `sortedmulti_a`; one key: native SegWit by default, Taproot, nested, legacy), the path (the kind's standard path with an account number, or a custom path) and the first address. **Make the wallet** builds the descriptor Create a wallet writes (`create.rs`) over the seeds' keys at that path, loads it with the seeds in their slots, and opens Check. |
| **A transaction arriving first** | The Wallets tab reads its key origins and finds the wallet it belongs to among loaded wallets, then among vault keys. "This transaction spends from Savings; you can sign with 73c5da0a." With no wallet known, it can still sign from the PSBT's own key origins, but change cannot be verified, and the review says so and offers **Load the wallet**. |

**Adding a key to a slot.** Each slot row has **Add its key**: from another
open vault, typed words, a scanned SeedQR, or a scanned account key (watch
only). The key is matched against the slot before it is attached. A key for
a different slot goes to that slot and says so ("that is key 2, not key
3"). A key from no slot of this wallet is refused with its fingerprint. A
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
| **Check** | First addresses, receive and change; the wallet as a QR for the online wallet (one code or animated, by the plan in §3.2); **Is this address mine?**. |
| **Transaction** | A PSBT by scan, a file in Files or typing. The review in sentences with the full table one choice away; every warning from one function, never softened for guided mode (§3.3). |
| **Txid** | The transaction id before anyone signs, for native SegWit and Taproot spends; refused for legacy and P2SH-wrapped inputs, whose id changes on signing. |
| **Sign** | Signs with every key held, producing one partly signed PSBT per key, the form separate devices would give. |
| **Collect** | **Signatures for this transaction**: cosigners' PSBTs by scan, file or Files. Each signature is verified against its key and this transaction's sighash; a PSBT for another transaction is refused, never merged. The collection belongs to the transaction, so reviewing again keeps it. |
| **Finish** | Two results: the **signed PSBT**, not finalised, for the wallet that wrote it (`ur:crypto-psbt`); and the **finished transaction** with its txid in a box of its own and **Decode this transaction**. Finalising may fail one signature short; the signed PSBT is shown anyway, labelled with what is still missing. |
| **Agree** | Paste another device's signed PSBT; compare the signatures byte for byte: **identical** or **differs**, with both shown. |
| **Backup** | The four steps of §5. |
| **Save** | Wallet and keys to a vault: which vault, each key's "load at unlock" flag. |

### 3.2 Wallet forms read

Through `osk-codec` and `osk-bip`: descriptors, BIP-388 policies, the
Coldcard/Sparrow/Nunchuk/Specter multisig config (whole or partial), BSMS
records, key lines in every form a keyholder may have (`73C5DA0A: xpub…`,
`[73c5da0a/48h/0h/0h/2h]xpub…/<0;1>/*`, a bare xpub, flagged because it
cannot be matched to a PSBT), Coldcard's JSON account export, SLIP-132
keys, `ur:crypto-account`, `ur:crypto-hdkey` and `ur:crypto-output`
(`docs/QR.md`). Sparrow's wallet `.json` is read here and not by
OpenSigner; it is proposed upstream as an `osk-bip` reader. A
descriptor is kept exactly as it arrived, never re-emitted from its keys,
so `multi()` stays `multi()`.

### 3.3 The review's checks

From `osk-psbt`'s inspection, plus the Wallets tab's additions, which are
non-cryptographic and live in `faraday-core`:

- **Change** is labelled only when it re-derives from the wallet's keys;
  an output that claims to be change and does not is **change, unverified**,
  and needs explicit confirmation.
- **Amounts** from a full previous transaction must hash to the outpoint
  and agree with any `witnessUtxo`; an amount that is only stated is
  **amount unchecked**.
- **A SegWit v0 input without its previous transaction, in a transaction
  with more than one input,** is refused, with no override
  (`osk-psbt`'s `AmountUnverified`, a blocked warning). BIP-143 signs the
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
- Fee, dust, network, unknown paths and the deterministic nonce check, as
  OpenSigner already shows them.

## 4. Spending

The Wallets tab's order, kept for every kind: **load the wallet → the
transaction → check → the signers → sign → collect → finish.** Check is
open at entry only the first time a wallet (by descriptor checksum) is
checked this power-on; once Continue has been pressed on it (here, on
Create's own Check, or "It matches" on the Spend tab), it stays closed
and done across a lock and the next process, open again only in a fresh
one with no kept state (`docs/SIMPLIFY.md` §2.5). Everything before the
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
| FROST, m of n | Load (group record) → Transaction → Check → **Round 1** → Collect commitments → **Round 2** → Collect shares → Finish | Two rounds among the m chosen signers. See §4.2. |
| Message (any wallet with addresses) | Choose wallet and address → Message → Sign (BIP-137 or BIP-322) → Result as text and QR | Also **Check a signed message**. |

### 4.2 Rounds that hold a secret between them

MuSig2 and FROST make a secret nonce in round 1 and need it in round 2.
Locking restarts the app, so a nonce held in memory dies with any stick
visit. Two ways through, both offered:

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
(`Faraday::sign_seed`, the session's seed with a count of the passes, as
OpenSigner's `musig_seed` draws it), a session whose nonces have all
signed is dropped, and a pass is refused until the shell has answered
the first request for randomness (2026-10-10, after the audit found the
process's one seed reused: a nonce shared again was the nonce shared
before).

This replaces OpenSigner's FROST carry file, which today holds the bound
secret nonce unencrypted, with the vault's encryption.

## 5. Creating and backing up

| Kind | Flow |
|---|---|
| Single-sig | Kind (script type) → Keys (from a vault, new, typed, scanned) → Check → Back up |
| Multisig, Taproot multisig | Kind → Quorum → Keys → Check → Back up |
| MuSig2 | Kind → Keys (N of N; every key's account key) → Check → Back up |
| FROST | Kind → Quorum → Keys, whose Continue **deals** (on this device, as OpenSigner does) → Check → Back up |
| Recovery | Kind → Primary keys → **Recovery paths** (up to three, each with keys and a timelock) → Check → Back up |

Create's cards are Kind · Quorum · Keys · Check · Back up (2026-10-10,
`docs/SIMPLIFY.md` §2.3). Keys' Continue makes the wallet and opens
Check; there is no card of its own for the descriptor. Check shows the
descriptor as a summary row ("2 of 3 · native SegWit multisig ·
9a6a2580, cf0e9805, 048ab54e · #qf45pmyh"), which opens the wallet's code
with the descriptor's whole text under it, then the first addresses.
**Back up** is the plan's three presets; each opens Back up a wallet on
the wallet just made with that preset applied, its chip "Then: Wallets",
its way back the wallet's card. The seeds and the wallet into a vault,
the paper and the public files are the checklist's items below, not
cards of Create.

**Backup** is a plan, then a checklist of only what the plan needs
(2026-10-09, the owner's proposal A; it replaced five fixed cards that
each showed every option). Any loaded wallet can be backed up again,
not only the one just made: from the wallet's card (**Back up**, that
card's wallet), Start's **Back up a wallet** and the Tools tile (the
wallet last picked on Wallets), Create's **Back up** card (with the
preset chosen there) and Restore's **Back it up again**. With more than one wallet loaded, the flow's head reads
"Back up a wallet" and the wallet's name is a chip; a press on it lists
the loaded wallets, one row each with its shape, and a press on one
starts the backup again on that wallet. The plan's logic is `plan.rs`,
tested on its own; nothing is marked done that was not.

**1. Presets, then the plan.** The flow opens on three presets, one tap
each: **Paper only**, **Paper and vault**, **Paper, vault and watch-only
software**. Each fills the plan's questions; every answer stays
changeable. The questions are cards, each a multi-choice list (DESIGN
§4.2), Continue under it; on the small panel one question per page:

- **The seeds go**: on paper, words by hand · on paper, SeedQR by hand ·
  in the vault · as a file, unprotected (the secret sheet's warning and
  "I understand", below). Not asked for a wallet with no seed here.
- **Places**: how many places keep paper (1 to the wallet's seeds, at
  least 3); for a multisig, each place its own share (the default) or
  the whole wallet sheet; which places keep a stick with the vault;
  and, with a vault open, each place's name (below). Seed *i* goes to
  place *i* mod the places, and a place past the last seed keeps
  another copy; shares are spread the same way.
- **The wallet description goes**: a sheet (or share) in each place ·
  in the vault · into watch-only software · as files on a stick.
- **Software** (asked when the description goes to software or files):
  Sparrow (the wallet file) · Coldcard, Keystone, Passport (the
  multisig config) · Nunchuk (the BSMS record) · Bitcoin Core (its
  import) · Not sure (the descriptor), each the descriptor where the
  wallet has no such file; form: QR picture · text · both. This decides
  which public files are offered at all.
- **Passphrases** (asked when a seed was loaded with a BIP-39
  passphrase): where each passphrase is kept, a row per place and "In
  the vault, with its seed"; a place that keeps its words is dimmed and
  cannot be ticked.

Defaults from the wallet: one key, paper words, two places, the sheet
in each, the descriptor as a QR picture; a multisig, a place per seed,
each with its seed and its share; seeds held elsewhere are listed "On
its own device" and get no paper here; watch only, no seed question.
"Paper and vault" adds the seeds and the wallet into the vault and its
stick in place 1; the third preset adds watch-only software.

**2. The map**, the side panel (on the small panel the plan's last page
and the envelopes' item), redrawn as answers change: one box per place,
the vault, a stick of unprotected files, watch-only software and the
seeds on their own devices, each listing what it holds tagged secret,
sealed or public. Under it the check, found as `backup::audit` finds a
split's, by trying each place lost and each place found:

- **Any one place lost: the rest rebuild the wallet**: Yes, **Yes, with
  the vault's passphrase**, or No. Rebuilding takes a quorum of seeds,
  each with its passphrase, and for more than one key the description,
  whole or from shares that together hold every key. Watch-only
  software counts as a copy of the description; a seed on its own
  device counts as kept.
- **One place found: can spend**: Yes, **Only with the vault's
  passphrase** (the place keeps the vault's stick and the vault holds
  what is missing), or No.
- **One place found: sees the balance**: the same three values; the
  whole description, or a one-key wallet's seed, shows it.

A place here is a paper place, a stick of files, or the vault's own
stick when no place keeps one. Labels and values only; what each line
means is in Learn's Backups page.

**3. The checklist**, the plan made (**Make the checklist**): only the
items the plan needs, each done by what it does, never by a tap:

1. **Print N blank templates** (one per paper copy of a seed): made For
   the stick with the checklist, again when the word count changes; done
   while it is For the stick or on the last visit's receipt. The template holds no secret
   (numbered word lines, path, network, the SeedQR grid's fixed
   squares) and a second page of empty lines, "Place ____ holds ____",
   one per place, filled in by hand; nothing of the plan is printed.
2. **Copy seed {fp} by hand**, one per seed here: words and the SeedQR
   grid behind hold-to-reveal, never a printer, no line for a BIP-39
   passphrase. Done when **Scan my copy** (the camera reads the drawn
   SeedQR, compares its words with the seed in place, never loaded,
   names the first word that differs) or, with no camera, **Type my
   copy's numbers** matched that seed. Seed XOR and codex32 are here
   too.
3. **Save the seeds into {vault}**: done when the open vault holds
   every seed here. Each seed: **Save into {vault}** (the key record
   Vaults' Save writes, loaded at unlock) and **with its passphrase**
   when it has one; with no vault open **Make a vault**, **Unlock
   {name}** or **Unlock a vault**, which come back to this item.
4. **The seeds as files**: **Save as a file…** opens the secret sheet:
   "Save into {vault}" there writes the same key record; or, once "I
   understand: anyone who copies the stick or sees this file can spend
   these coins" is ticked, the words (`{wallet}-{fp}-words.txt`) or the
   SeedQR as a labelled PNG (`{wallet}-{fp}-seedqr.png` or
   `-compactseedqr.png`) go For the stick under Unprotected secrets,
   unticked on a stick visit; never the passphrase. Done when each
   seed's file is there.
5. **Save the wallet into {vault}**: done when an open vault holds it.
6. **The wallet sheet** (descriptor, keys, first addresses, a PDF) or,
   split, **The shares**: each sheet omits at most M−1 keys, so any
   quorum rebuilds and no one share watches, with the measured minimum
   rebuild group; each share as its PDF, its text and
   `{name}-share-k-of-n.png`. Made For the stick with the checklist,
   with **Remove**; done while it is For the stick or on the receipt.
7. **The public files** for the software and form chosen, drawn from
   the wallet (descriptor, wallet file, multisig config, BSMS record,
   Bitcoin Core's import), with each key whose seed is here
   (`xpub-{fp}.txt` and its BIP 129 record) for the cosigners. The
   files the plan chose are made For the stick the moment the checklist
   is made (2026-10-10, `docs/SIMPLIFY.md` §4.2); each row reads "For the
   stick" with **Remove**, or "Written to {label}" once a visit wrote
   it, and **Make the file** where it is neither. Done when each chosen
   file is For the stick or on the receipt.
8. **Show the descriptor to the software**: the descriptor as one code
   or animated (`QrWallet`); done once shown.
9. **One envelope per place**, listing what goes in it: done by
   **Done**, the one thing the device cannot see.

Under the list, **Change the plan** goes back to the questions. With
every item done, "Backup done", the count of files for the stick,
**Write to a stick**, **Open Files**. On the side panel each line of the map carries its
item's state, a dot in OK once done. The small panel's copy page keeps
the per-seed lines of `Faraday::backup_kept` (in which vault, copy
checked, a file for the stick unprotected, not here).

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

**Backups** (`docs/SIMPLIFY.md` §5, `docs/FLOWS.md` "Backups") lists
every wallet the app knows, once each by descriptor checksum: loaded,
in an open vault, or remembered of a locked one. Under each, the map as
the plan draws it, a row per place with what it holds, its tag and what
this device saw of it: a paper place **Checked** only once a copy of
each seed it keeps matched here (the first check of a seed is the first
place's copy, the second the second); the vault by its file and
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
the same paper gives the same addresses and byte-identical signatures.
What changes is wording, order and what is folded away:
- a first page of large choices ("What are you holding?") whose answer is
  the navigation;
- the public half first and the seeds last: load the wallet, check the
  money, add the transaction, then the seeds, then sign;
- the review in sentences, with the full table one choice away;
- warnings word for word as in full mode, from the same function.

A switch beside each flow's title chooses **Steps only** or **Guided** (called Full until 2026-10-06); a wallet in
progress carries over between them. The choice is one setting for every
flow, kept with the others; Steps only is on the left, and Guided is
where a first start is, on a device that has never had a setting saved
(`docs/SIMPLIFY.md` §6.1, 2026-10-10; reverses the 2026-10-07 default
for a first start only).

This is the **Spend** tab (`docs/FAMILY.md`, built
2026-10-06): one column from what is in the envelope to a sent
transaction, through the vault's unlock and load, on the same session and
spend as the Wallets tab.

## 7. Tools

**Read a transaction** (no wallet needed), **Is this address
mine?**, the address table, exports (descriptor, BIP-388 policy, BSMS, Core
`importdescriptors`, Sparrow `.json`), the nonce check, **Do these agree?**,
BIP-85 (hidden until turned on: children of one seed are not
independent signers, and none are offered as one), and **Just show me**
(a 3-of-5 test wallet from five repeated-word seeds, marked as a
test). From OpenSigner, in §9's order: the key explorer, silent payments
(receive and check), codex32, SLIP-39 and Seed XOR backups, the Lightning
node key, Learn.

## 8. Tests

- **The conformance corpus**: `qr-testing/` and
  `qr-testing-testnet/` restore to the stated first addresses by every
  route; the ten `bad-*` PSBTs are each refused for the reason
  `bad-psbts.txt` states; the 3-of-5 split rebuilds byte-identically from
  any three sheets; the five repeated-word seeds give the same descriptor
  and first address.
- **Flow tests** drive each flow through its steps for every kind in the
  capability table, from every entry point of §2.
- **Upstream's own tests** run first, unchanged.
- Agreement checks against the Coldcard Q simulator and SeedSigner's
  libraries are reproduced for the flows that produce the same outputs.

## 9. Order of work

1. The capability table; the wallet card and the
   reassembly model (§2); Load and Signers.
2. Spending for single-sig and multisig: Check, Transaction with every check
   of §3.3, Txid, Sign, Collect, Finish, Agree. Run against the conformance
   corpus.
3. Creating and backing up single-sig and multisig, with the split plan.
4. Guided mode.
5. Taproot multisig, miniscript and Taproot tree, recovery.
6. MuSig2 and FROST, with §4.2.
7. Messages; the Wallets tab's tools; then OpenSigner's tools.

## 10. Decided 2026-10-04

1. **The OpenSigner tab stays** beside the Wallets tab until the Wallets tab is robust
   enough to deprecate it (§1).
2. **Shared logic upstream**: done by OpenSigner's §16.138 for the file
   formats, the kept-key blob, messages, addresses and transactions (§1,
   rule 5).
3. **Signed-amount memory** is sealed into the open vault by default, and
   kept in `/run` across locks (§3.3).
