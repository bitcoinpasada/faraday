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
summary line and a result, rendered by `faraday-ui`, one step's card at a time
by the flow container (`docs/FLOW-CONTAINER.md`, `docs/LAYOUT.md`); and flows as ordered lists
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
   That put the
   Argon2id `Cost` in **`faraday-crypto`**, the signed-message text form and a
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
return as Wallets-tab tools in the order they were listed.

## 2. Reassembling a wallet

Most of the time a person here holds part of a wallet: one key of a 2-of-3
in a vault, the wallet's description on a backup sheet, the other keys on a
SeedSigner and a Coldcard. The Wallets tab's central object is therefore a
**wallet in progress**, which is valid at every stage of being put together.

Loading a key makes no wallet; the address check (`docs/QR.md` §2) still searches the key's standard single-key accounts (BIP-84, BIP-86, BIP-49 and BIP-44, account 0) that no loaded wallet is, so an address of a key with no wallet made is still found. Such a find, like a single-key account found in Explore, is to end with **Make a wallet**, designed and not yet built: `docs/FLOWS.md` (Explore, Make a wallet at the path) holds it, and `docs/DESIGN.md` §5 (Check an address, The result) the answer card.

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
  its name (any kind Create makes;
  `docs/history/NEW-WALLET.md` §16.2, 2026-10-19; in the foot when a long name
  leaves no room, and in the button list on a small panel), then along
  the foot **Show wallet QR**, **Back up**, **Sign a message** where
  the wallet can, **Check an address** and **Explore its keys** (for a key
  of the wallet that is loaded), and **Remove from session** at the end. New wallet
  from this one opens Create filled from the wallet at the next account
  its keys and kind do not use (§5).

A PSBT waiting leads the page as a banner naming its file ("{file} in
Files"). It carries a **Sign** of its own only when the PSBT is another
wallet's, its one way in; when the selected wallet's card carries the action,
the banner is the file's name alone.

**Every way in leads to the same card:**

| Starting from | What happens |
|---|---|
| **A vault holding a wallet and one of its keys** | At unlock the wallet loads into the Wallets tab with its key attached. The card opens at "1 of 2 signatures possible here." Nothing to do. The boot sheet's Unlock lands on Wallets with "Savings loaded from vault.kdbx", the first wallet loaded that can sign here selected (`docs/history/NEW-WALLET.md` §11.1). |
| **A key with no wallet** (from a vault, or added on its own) | Listed under **Keys without a wallet**. With no wallet loaded the card offers **Make a wallet from this key** (Restore's Seeds card with the key in its slot: single key, native SegWit, account 0), **Start a multisig wallet with this key** (the same on a 2-of-2 multisig) and **Load or restore a wallet**; with wallets loaded the key's row carries **Make a wallet from this key**. |
| **A wallet description arriving** (scan, file, Files; any form in §3.2) | The Wallets tab derives each vault key's account key at the descriptor's paths and compares, attaches every match, and marks the rest watch-only. "This wallet uses 73c5da0a from Main" says which matched. **Save the wallet to a vault** is offered. |
| **Pieces of a description** (split sheets, key lines, a partial config, SeedSigner exports one at a time) | The card exists from the first piece, with the quorum unknown or stated, and lists fingerprints in hand and still missing. It completes when the last piece arrives. Overlapping pieces deduplicate; pieces that give one fingerprint two xpubs or two derivations, or that carry two names, are refused, and a vault's piece is taken only where the wallet it names holds every key of the pieces in hand and of its own. |
| **Seed words alone** (one list or several, perhaps a line like "2 of 3", no description) | Restore (`docs/history/NEW-WALLET.md` §12): **Kind** (one key: native SegWit ticked; several: Multisig · native SegWit; More kinds for the rest), **Quorum** for a multisig (M of N on two sliders, 2 of 3 to start; tap, drag or the arrow keys), **Description** › **I do not have it** (**I have the seed** for a single key), then **Keys**: a slot per key. The first empty slot takes its seed through **Type the words** (Add a key, which returns here, with its BIP-39 passphrase), **Scan a SeedQR**, a seed from an open vault or a key already loaded, each of the last three asking on a sheet whether it has a passphrase (typed twice; `docs/FLOWS.md` decision 52); on a multisig a slot can take **Its xpub** instead (Scan, a key file in Files, or typed: `xpub`, `zpub`, `Zpub`, `tpub`, with or without origin). With no description the seeds do not say the account or a path other than the standard one, so the Keys card also asks for them (the **Account** stepper and **Custom path**; a path that gives no wallet says why in place of Make). **Make the wallet** builds the descriptor Create a wallet writes (`create.rs`) over the seeds' keys at the kind's standard path and account, loads it with the seeds in their slots, and opens Check: the descriptor, the first addresses and, for a wallet made from the seeds alone, a line "Made from the seed alone: {kind}, account {n}, {path}" with **It matches my record** and **I have nothing to compare**. A backup plan that keeps only the words does not read as rebuilding such a wallet unless it is Native SegWit at account 0 of the wallet's network, and the blank seed sheet of a wallet prints its path beside its shape. Done offers **Back up this wallet**, **Spend from this wallet** and **Wallets**. The Spend tab's words route keeps the older piece (`seeds.rs`), with the kind and path chosen after the seeds. |
| **A description first, then its seeds** | Restore's Kind offers **I have the wallet description: Scan it** and **From Files**. A description read there, or on Description, fills Kind, Quorum and the cosigners' keys and opens Seeds, a slot per key of the description. Each seed entered (with the slot's passphrase, if any) is matched to its keys: a match reads "Key 2 of the wallet"; any other seed, or the right words with the wrong passphrase, reads "Not a key of this wallet: check the words or the passphrase" and is not loaded. A key whose seed is elsewhere is marked **Not here**. A description whose keys include one used twice (the same public key and chain code under another origin or depth) still loads; Kind carries a warning line for each, "Key 2 is the same as key 1". |
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
| **Kind** | Choose the wallet kind, from the capability table: single-sig (four script types), multisig, Taproot multisig, MuSig2, recovery (timelocked paths). Imported-only kinds (miniscript, Taproot tree, watch-only) appear only on the load path. |
| **Quorum** | M and N, with a plain sentence of what that quorum means. Fixed at N of N for MuSig2. A multisig that Create writes as a descriptor (`wsh`, `sh(wsh)`, `sh`, Taproot) has a path choice above the Account row, a segmented button: **BIP-48** (**m/45'** for `sh`), the default and ticked on entry, or **BIP-87**, `m/87'/coin'/account'`, which leaves the script to the descriptor (BIP 87, Complete; `docs/STANDARDS.md` §2). While the wallet is made, an info button after BIP-48 (after the value, or after the segmented button; not for `m/45'`) opens a tooltip saying BIP-48 is the standard multisig path most wallets use and BIP-87 is newer, the same for every script type, and supported by fewer (F287). The key's origin in the wallet's descriptor, and in every file written from it (the multisig config, BIP 129's record, the descriptor's code), follows the choice. A BIP-87 wallet's own keys give the `ur:crypto-account` code and the signed BIP 129 key record as a BIP-48 wallet's do, their origin naming `m/87'/coin'/account'`; both formats carry any origin path. A cosigner's key fills a slot only at the path chosen: at BIP-87 a key at m/87' and no other, at BIP-48 or `m/45'` the kind's own. A wallet made from one at BIP-87 opens with BIP-87 chosen, and its cosigners' keys come back only while the choice and the account are the wallet's. |
| **Slots** | One positional row per signer. Removing signer 2 leaves slot 2 empty rather than renumbering. Sources: a key in a vault, a new key (dice or entropy), typed words, a scanned SeedQR, an account key (watch only), or **someone else will add this later**. Each filled slot has **Show key QR**, as a `ur:crypto-account` byte-identical to SeedSigner's, or as the plain key expression. |
| **Load** | A wallet in any form (§3.2). Builds the wallet card, routing itself to the right kind. |
| **Signers** | The wallet card's slots, filled for signing (§2). |
| **Check** | First addresses, receive and change; the wallet as a QR for the online wallet (one code or animated, by the plan in §3.2). An address is checked against every loaded wallet's first 100 receive and change addresses by **Check an address** (Tools, Transactions), and a miss can **Look further**, to 1,000 or 10,000 of each chain in every loaded wallet, with **Stop**. |
| **Transaction** | A PSBT by scan, a file in Files or typing. The review in sentences with the full table one choice away; every warning from one function, never softened (§3.3). |
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
`[73c5da0a/48h/0h/0h/2h]xpub…/<0;1>/*`, a bare xpub, refused because it
has no origin to match a key or a PSBT), Coldcard's JSON account export, SLIP-132
keys, `ur:crypto-account`, `ur:crypto-hdkey` and `ur:crypto-output`
(`docs/QR.md`). Sparrow's wallet `.json` is read here. A
descriptor is kept exactly as it arrived, never re-emitted from its keys,
so `multi()` stays `multi()`.

### 3.3 The review's checks

From `faraday-psbt`'s inspection, plus the Wallets tab's additions, which are
non-cryptographic and live in `faraday-core`:

- **Change** is labelled only when it re-derives from the wallet's keys.
  An output that claims to be change and cannot be verified (its origin
  names a key not loaded, or a path under no loaded account) is drawn under
  **Sends** as a payment, with its address and amount, never as change;
  its caution stays among the warnings (F136). One that names a loaded key
  and is not that key's address is refused (`ChangeSpoof`). Change at an
  address index of 10,000 or more is still change but carries a Caution,
  as a wallet with a gap limit does not find coins that far (F242).
- **Change is of the wallet spent** (F254): an address of one of the
  person's wallets (each loaded key's single-key accounts, the registered
  wallets that hold a loaded key) is change only where every input of
  theirs spends that same wallet. Otherwise it is a **transfer**, counted
  as sent, drawn under **Sends** as "To" and the wallet's name with its
  whole address, with a Caution; a transfer is never blocked. The Review
  names the wallet on a change row, which reads "Change" only for an
  address on chain 1 (an own address on chain 0 reads "Receive", still not
  sent), and on the spend panel's change fact. A registered wallet is named
  by its name; a loaded key's account by the loaded single-key wallet at
  it, else the key's label and the account's kind.
- **Review stops** where nothing would sign (F136): a blocked warning, or a
  danger with no way past it, stops the Review step with the refusal and
  what to do instead, and signing refuses it too. No key here taking
  part is no stop: the spend goes on to collect the signatures made
  elsewhere. Only two dangers have a
  way past, and it is a hold: **Fee holds** below.
- **Fee holds.** A fee of 20 % or more of what is sent, or over 0.01 BTC
  (`HighFee`), and a rate of 1000 sat/vB or more (`AbsurdFeeRate`), show
  the fee, its rate and its share of what is sent, the cases such a fee
  is for (a child paying for its parent, a large batch, a consolidation),
  and **Hold to pay this fee**. Signing is passed those dangers by name
  only after the hold. The fee is worked out from checked amounts only;
  a fee that cannot be worked out reads **Not known**, never zero. A fee
  of 5 to 20 % is a caution.
- **A spend path that waits longer than the transaction's sequence and
  locktime allow** (`TimelockNotMet`) is refused when no way to spend
  the wallet that the person's keys take part in is open under that
  sequence and locktime: the transaction can never be valid. The
  coordinator sets the input's sequence or the transaction's locktime
  to the path's wait and makes it again. While another way of theirs is
  open, the path read from the PSBT (which may lack a cosigner's
  origin) is not refused.
- **A key origin that names a loaded key at a path under its account, and
  derives another key than the input's**, is refused at Review, as signing
  refuses it. A path under no loaded account is a caution.
- **Amounts** from a full previous transaction must hash to the outpoint
  and agree with any `witnessUtxo`; an amount that is only stated is
  **amount unchecked**.
- **A SegWit v0 input without its previous transaction, in a transaction
  with more than one input,** is refused, with no override
  (`faraday-psbt`'s `AmountUnverified`, a blocked warning). BIP-143 signs the
  amount an input states, so two signing rounds over two different
  stated amounts give two valid signatures that pay the difference to
  the miner (the 2020 two-round attack). The fix is the previous
  transaction, in the PSBT or brought in through Files. A finalized input
  is checked the same way, since its final witness is the coordinator's.
  A single-input
  transaction is signed from its `witnessUtxo`: a false amount there
  gives a signature no node accepts. Taproot inputs commit to every
  amount and need no previous transaction. The rule holds for an input
  another party brings (a payjoin, a coinjoin) too, though it is never
  signed here: an input that names none of the loaded keys is never
  signed, and nothing tells it from one of the person's own whose key
  origin a coordinator left off, so its amount is checked as theirs
  would be.
- **Signed-amount memory**: the amounts each input stated are remembered
  against the unsigned transaction's hash. The same transaction stated
  differently is refused with no override. The memory is kept in
  `/run/faraday`, outside the app process, so it survives a lock; with
  a vault open it is also sealed into that vault (on by default, a setting),
  so it survives power-off. Decided 2026-10-04. What a vault remembers
  is read at its unlock and counts while the vault is open, for whichever
  wallet signs the transaction; it is not sealed into another vault or
  kept across a lock (the vault keeps it).
- **Add the previous transactions**: raw transactions typed, scanned or
  read from Files, each kept only if it hashes to an outpoint being
  spent. It opens itself when the transaction needs it.
- **Sighash.** `ALL|ANYONECANPAY` signs, with a caution. `NONE`, `SINGLE` and
  a non-standard value are refused, with no way past (F136).
- **A script of several keys** (a `multi`, a miniscript, a taproot leaf
  of two or more keys) is signed only for a registered wallet whose
  script it is at the chain and index the input states (F135); otherwise
  the transaction is blocked, with no override (`WalletNotRegistered`).
  A PSBT that carries its cosigners' extended keys (BIP 174's global
  xpubs) and spends one `multi` shape offers **Register this wallet**:
  its shape, a name, and the confirmation sheet's rows (each cosigner's
  fingerprint, path and whole key, which is yours or checked, F286, and
  the checksum of the reading chosen), and a hold, the wallet's one confirmation (F254),
  that adds the wallet once its descriptor gives every input's script and
  every change output's that claims it and one of its keys is a loaded
  key's own ("None of these keys is yours" otherwise); where the spend
  cannot tell `sortedmulti` from `multi`, the sheet offers both, sorted
  first. Otherwise the line says to load the wallet's descriptor. A
  wallet that holds none of the loaded keys, however it came in,
  verifies no change and signs nothing of several keys.
- **A miniscript or a taproot tree of one key** (a single-key leaf, or
  the key path of an output that states a tree) is signed only through
  a registered wallet whose script it is, too (F141). Without one the
  input is left unsigned and the rest of the transaction is signed: the
  warnings say "Input 1 is left unsigned: no registered wallet covers
  this taproot tree. Register its wallet to sign it." Where no input is
  left to sign, nothing here takes part (`NoParticipatingKey`).
- Fee, dust, network, unknown paths and the deterministic nonce check
  are shown. With Nonce history on and a vault open (`docs/FLOWS.md`,
  Nonce history), an opened vault's entries name each checked input on
  chain by its `r`, transactions of wallets not held here included.

### 3.4 The key path of a `tr()` wallet

Every `tr()` wallet with a script tree states who can spend it by the
key path, its internal key alone (F132): "Key path: none
(unspendable)" for the NUMS point or an extended key over it; "Key
path: <the key> alone" for one of the wallet's keys; and for any other
key the danger line naming that key in groups of four. Only the first
reads as "k of n". The third is added only after a hold on the wallet
confirmation sheet (F254), whose rows and key path line name the key,
after which the flow that asked goes on; it keeps
the danger line on its card, counts its signatures against its leaf's
own quorum, reads "Another key can spend" in the error tone where a
wallet reads "Can spend", and a spend from it repeats the line as a
caution at the head of the review's warnings. None is refused for its
key path.

### 3.5 A wallet of several keys is added through one sheet

A wallet of more than one key (a multisig wallet, a MuSig2 `tr()`) is
added only after the person holds the wallet confirmation sheet (F254),
which shows each key whole, however the wallet comes in. Not held back:
a wallet whose every key is a loaded key's own at its path (the public
key and chain code a loaded key gives there, never the fingerprint
alone), since nothing in it came from outside; a wallet the person's own
vault holds (loaded at unlock, loaded later from the open vault or
restored from it); and the public samples Load samples adds. Several
held at once are asked one after another, "{i} of {n}", and Cancel
drops only the wallet on the sheet.

Files' **Add to vault** first refuses, in the words a load refuses them
in and keeping nothing, a key under a loaded key's fingerprint that is
not that key's at its path, a MuSig2 wallet with two keys of one seed
and a silent payments record that does not match the loaded keys, at the
press and again at the sheet's hold. The first and the last are seen
only while the key is loaded. A key waiting for its backup under another
fingerprint, which a load refuses, is kept, and does not hide a refused
key listed after it.

It asks the same sheet, under the same condition, before the vault keeps
a file's wallet. Its hold loads nothing: it puts the wallet, with the
sheet's ticks, in the vault that was the current one at the press, from
the wallet file of that name wherever Files now has it, while it reads
as the keys, in their order, and the name the sheet showed. Where that
vault is closed or another is shown, or Files has no such file, nothing
is kept and a toast says so. Cancel keeps nothing. A vault that would
not take the wallet says why before the sheet. The hold counts for a
later load, which then asks nothing, only where the vault kept the
wallet and a load would not refuse it before its sheet then.

**Each key carries a check** (F286). The sheet's keys, and everywhere
else a wallet of several keys shows them, are one block
(`faraday/faraday-core/src/wallet/confirm.rs`): per key its number,
fingerprint, path and whole extended key, and a state. A green check
"Yours, can sign" is a key Faraday derived from a loaded seed ("Yours,
can sign · your single-key account" for a key at a single-key account
path); a green check "Checked by you" is one the person ticked; a grey
circle "Not checked" is one nothing vouches for, which a press ticks and
another unticks; a red cross is a key whose fingerprint is a loaded key's
and which is not that key. Under the keys one line, grey "2 keys not
checked · compare each with its own device" or green "Every key checked";
the descriptor checksum stays the last row, in the quieter tone: it is 40
bits, a typing check, and a wallet with the same eight characters can be
made, so it is not proof. Ticking is optional and the hold adds the wallet
whatever is ticked; the confirmation reads "{name} added" or "{name} added
· 2 keys not checked".

After it is added, "Not checked" stays with the wallet for the session:
its node on the chart carries a second line, "2 keys not checked", in the
caution tone; its own sheet shows the block, tickable, under the
Descriptor row; receive addresses (first addresses, an address's code)
carry the caution line "2 keys of this wallet are not checked" and are not
held back; a spend's review shows the block while any key is grey and one
green line "Every key checked" once none is. A seed loaded later checks
every loaded wallet again: a matching key turns green by itself and earlier
ticks stay; a wallet with a key that claims the seed's fingerprint and is
not its key is taken out of the session with a notice
(`Session::drop_forged`). A vault keeps the ticks in a field of the
wallet's own record, so an unchecked wallet saved into a vault does not
come back looking checked; a wallet loaded from a vault still skips the
sheet. A single-key wallet shows no block, a wallet whose keys are all the
person's is green at once, and a lock ends the ticks no vault holds. The
states are worked out when a wallet is added and when a key is loaded or
removed, never while drawing.

Not decided: further ways the device
turns a grey key green (an open vault that holds the key, the key scanned
from the cosigner's own screen or the paper backup's code).

**A silent payments wallet's labels** are a count in its record, which is
no part of its id. An open vault's own record of the wallet that states
fewer labels than the wallet has handed out is raised to that count, in
every open vault that holds it; the vault then has a change to seal. That
is done on the press that shows such a label, and when a vault is opened,
whether or not anything is loaded from it, or the session's wallet comes
in or takes a higher count while one is open (a vault's load, a file). A
vault that cannot be written is told of and left as it is, and the load
goes on. A count is never lowered, a vault whose count is the same or
higher is not written, and neither is one that is not open or holds no
record of the wallet. A record put into an open vault with fewer labels
than the loaded wallet's is raised too. A write a vault refuses is told
of once, with the vault's name and why, and again when it is tried with
another count; two vaults refusing at once are named in one message. A
vault being sealed is not tried and nothing is said of it. Its write,
and one refused because memory is short or because the seal cannot write
it, is tried again after each later press, answer of the worker or change
to the sticks, until the vault takes it. A write refused as more than a
vault can hold is tried again only once that vault's records have
changed. A read-only vault's Faraday items are not read
([VAULT.md](VAULT.md)), so it has no wallet record to raise, and its
count is not seen either. Loading a
record of a wallet already loaded gives the loaded one its count when
that is higher, and nothing else of it.

A label shown on a key's page counts for the loaded wallet the key gives
at account 0, whether or not the page is that wallet's; another account
of the key, or a record that only names its fingerprint, is another
wallet.

The session keeps the most labels handed out for each wallet, whether or
not it was ever added as a wallet, until Lock or power off: leaving the
page, removing the wallet or the keys and opening a vault keep it. A
record of that wallet that comes in with fewer labels, into the
session's wallets or in a vault that is opened, is raised to it; no
wallet and no vault record is made by it. On entering a key's page, on
choosing its key, on coming back from an unlock started on the page and
on **+** from no label (a guard on the press that hands out a label), the
page's count is the highest of the one kept, the
loaded wallet's and every open vault's own record of the wallet; its
record, and the wallet added from it, state that count. The label on show
starts at none when another key is chosen. From none, **+** shows the
first label not yet handed out, one past that count (the last label once
all are handed out); **-** steps back down through the ones handed out. A
payment to a label is looked
for only up to the loaded wallet's count (and the Silent payments page's
own).

## 4. Spending

The Wallets tab's order, kept for every kind: **load the wallet → the
transaction → check → the signers → sign → collect → finish.** Check is
open at entry only the first time a wallet (by its id, a SHA-256 of its
whole descriptor, and for a silent payments wallet of its address and the
key origin its record states, never by its 8-character checksum, which
another wallet can share; two silent records over one address with another
fingerprint or account are two wallets with one checksum) is checked this power-on; once Continue has been pressed on it (here, on
Create's own Check, or "Yes, my money shows" on the Spend tab), it stays closed
and done across a lock and the next process, open again only in a fresh
one with no kept state (`docs/history/SIMPLIFY.md` §2.5). The kept list holds
the ids as 64 hexadecimal characters, and a list of 8-character checksums
from before names no wallet. The same id picks the wallet Spend from backup
chooses and the wallet a spend's Check was passed for, so another wallet of
the same checksum asks again. The same id tells the wallet whose money
moves to a new wallet and the new wallet, and which wallet's share record
(type 13) the backup checklist's Save to a vault replaces, so a wallet of
the same checksum keeps its own there. The backup flow's copy to a vault
goes by that id throughout: the share records it picks and takes out, the
description it writes into an added vault (only from a holder's
record or a Files description whose own id is the wallet's, and always the
text Faraday makes of the policy it read, never the file's own bytes, so a
crafted file of the same id never carries another account into the vault; where silent records of one id differ in their label count, the one with the most labels among Files' and the loaded wallet's), the description and plan
record it takes out or writes answers into, the holder's plan record it
copies, and the other wallets' records it protects (a twin's is one).
Only this wallet's description in an added vault, stored as the text Faraday makes of it, counts as already there (a file's raw bytes do not stop a clean copy being put beside it).
The flow keeps its wallet's id beside the checksum from the time it takes
the wallet while it is loaded and across the lock's restart, so a twin
loaded later never stands in and a twin's Back up starts a flow of its
own, never taking over or overwriting the first one. A flow with no id
(the gate's, made at a restart from a gate line that holds none: it
holds back keys; the flow a wallet being made begins with has no checksum
either and is found by its number; a gate line carries the flow's id, and one without joins a flow only where exactly one flow has its checksum, else it is a flow of its own and its key waits) goes by the one wallet of
its checksum that is here: a loaded wallet, else the open vaults' records
(a wallet, a plan, a share's wallet field), else Files, asked only where
the vaults give none. Where it finds two, the copy puts in and takes out
no share record and says to load only this wallet; where it finds none
and a share is to be put in, it says no copy of the descriptor is here to put in. With no id found, a
share the vault held before the flow is told by its number among the
tokens that carry an id or its own checksum, and is never taken out. Restore returns the shares ticked in Files as they are where they merge whole
on their own: a vault's contents never refuse them. Otherwise it tries each
wallet the open vaults' share records name, whose description reads and has
sheets (one wallet in another key order is one), on its own with the ticked
shares: a text is a share of a wallet where its quorum, Format and each key's
fingerprint, derivation and xpub are the wallet's, whatever wallet its record
names (the records that name the wallet come first); a record whose wallet
does not read counts for none. A wallet is complete where what the texts merge
to reads as it. The first complete one by record order is rebuilt; where none
completes the screen shows the count of the one holding most keys; where two
complete and share a checksum it names both and rebuilds neither, and says what
from this screen ends it: ticking a share file only one of them holds, or
opening only the vault of one; where no such step exists it says only that
neither is rebuilt. Everything before the
signers is watch-only, so any reason to stop is found before a key is
used. In Faraday the keys are usually already loaded from a vault, so
the signers step shows them rather than asking for them, and asks only
for what is missing. The transaction's id, where it is known before
signing, is a row of the Transaction card's table, not a step of its
own. Nothing of a spend goes to the Outbox until Review (and Check, where the wallet is known) is gone through on this device for that exact PSBT, also for a PSBT that arrives signed by enough keys elsewhere or becomes complete through Collect; a Collect answer that changes the PSBT needs Review again. Sign stops and Review is asked again after a Collect, or a wallet or key loaded, that changes what the inspection finds (a warning, or a key that takes part, Review did not show).

| Kind | Flow | What differs |
|---|---|---|
| Single-sig | Load (or the vault's wallet) → Transaction → Check → Sign → Finish | No Signers or Collect: the vault key is the wallet. |
| Multisig (`wsh`, `sh(wsh)`) | Load → Transaction → Check → Signers → Sign → Collect → Finish | The Wallets tab's flow as it stands. The txid row is refused for `sh(wsh)`. |
| Taproot multisig, `tr(H, sortedmulti_a)` | as multisig | Script-path signatures only (the internal key is unspendable); the review says which leaf. A `multi_a` tree over any other internal key is a Taproot tree (§3.4). |
| Miniscript, Taproot tree (imported) | Load → **Path** → Transaction → Check → Signers → Sign → Collect → Finish | Signs every signature its keys can give; Finish completes only when the script is satisfied, and says which condition is unmet (a cosigner, or a timelock not yet reached). |
| Recovery (Liana shapes) | Load → **Path** → Transaction → Check → Signers → Sign → Collect → Finish | **Path**: the primary path, or a recovery path, with its timelock in blocks and whether the transaction's inputs have aged enough (from the PSBT's sequence values). |
| MuSig2, `tr(musig(…))` | Load → Transaction → Check → **Round 1: nonces** → Collect nonces → **Round 2: sign** → Collect partial signatures → Finish | Two rounds; every signer must take part. See §4.2. |
| Message (any wallet with addresses) | Choose wallet and address → Message → Sign (BIP-137 or BIP-322) → Result as text and QR | Also **Check a signed message**. |

### 4.2 Rounds that hold a secret between them

MuSig2 makes a secret nonce in round 1 and needs it in round 2.
Locking restarts the app, so a nonce held in memory dies with any stick
visit. The app runs both rounds within one session: round 1's nonce
leaves as a code, the others' arrive by camera, and round 2 follows
before any lock, so no MuSig2 secret nonce outlives the session it was
drawn in (F234).

Every pass that may open a round draws its nonces from a seed of its own
(`Faraday::sign_seed`, the session's seed with a count of the passes), a session whose nonces have all
signed is dropped, and a pass is refused until the shell has answered
the first request for randomness (2026-10-10, after the audit found the
process's one seed reused: a nonce shared again was the nonce shared
before).

## 5. Creating and backing up

| Kind | Flow |
|---|---|
| Single-sig | Kind (script type) → Keys (from a vault, new, typed, scanned) → Check → Back up |
| Multisig, Taproot multisig | Kind → Quorum → Keys → Check → Back up |
| MuSig2 | Kind → Keys (N of N; every key's account key) → Check → Back up |
| Recovery | Kind → Primary keys → **Recovery paths** (up to three, each with keys and a timelock) → Check → Back up |

Create's cards are Kind · Quorum · Keys · Check · Back up (2026-10-10,
`docs/history/SIMPLIFY.md` §2.3). Create opens with Kind open and no row ticked, whatever opened
it — Wallets or Home — and Next waits for a row; a Tools tile opens it with its kind already ticked
(2026-10-10, `docs/history/NEW-WALLET.md` §2.1, reversing `docs/history/SIMPLIFY.md` §2.1 for
this flow only); Kind's Continue closes it as done and opens the next
card. Keys' Continue makes the wallet and opens Check; there is no card
of its own for the descriptor. A wallet with a cosigner's key no loaded
key gives is made once its confirmation sheet is held (F254): Continue
goes on from the hold as it would have with no sheet, and Cancel leaves
Keys open with no wallet. Restore's Make from typed seeds and its
description work the same, and a vault's wallet restored from the open
vault takes no sheet. The Keys card is always shown (the vault counters it once passed
over are gone), and its empty first slot of a single key backed up as
words, or Key 1 of a multisig while no slot holds a key made here,
holds the making of its key in place of New key (F282, `docs/FLOWS.md`,
A key made on the Keys card); **Other ways** opens the key's cards inside
Make a new wallet (F288), whose Cancel returns to Keys. On the Keys card, a key chosen for one
slot — loaded here, or a cosigner's file — is not offered for any other
slot, until the slot is cleared (`docs/history/NEW-WALLET.md` §2.2); its foot
button reads **New key for Key N**
while a slot is still empty, N the first one, and opens the key's cards for it
inside Make a new wallet, else **Continue** (`docs/history/NEW-WALLET.md` §2.3). Check shows the
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
card lists every open vault's keys not loaded, under the vault's name;
a press loads that key and
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
The cards are shown one at a time by the flow container, each answer a
row of Your choices:

- **Places**: **Number of places**, a stepper, one or more with no upper
  bound; and, with a vault open, each place's name (below). One more
  adds an empty place; one fewer removes the last place and what it
  held.
- **Place 1**, **Place 2** …, one card per place, titled with its name
  where it has one: a multi-choice list (DESIGN §4.2) of what the place
  keeps, **Seed words by hand** · **SeedQR by hand** · **Vault 1**,
  **Vault 2** … (a row per vault of the plan, named as the vault set up
  for it is) · **Seed as a file, unprotected** (on the place's stick of
  files; **Save without encryption?**'s hold, F129, below) ·
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
  *Shares of a wallet description*; in
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
its own device" and get no paper here; a key loaded here with neither
words nor a codex32 set (SLIP-39 shares, a master seed) is on no device
of the plan's: it is listed nowhere and no paper or file place offers it
(it has no words to write), and while no vault of the plan holds it
it counts for no check and one line on the board and the map says "Seed
{fingerprint} was loaded from shares or a master seed: this plan does
not count it"; a vault takes it as it takes any wordless key, and then
it counts for that vault's checks and the line goes; watch only, no
seed question. "Paper and vault" puts one seed in each vault, vault *v*
holding the *v*th seed here (a key kept as shares included), and the
wallet in every vault; the third preset adds
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
from an action on the wallet's chart (2026-10-10) it opens at that item with the plan the chart shows, and makes
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
   "Save into {vault}" there writes the same key record; or, past
   **Save without encryption?**'s hold (F129), the words
   (`{wallet}-{fp}-words.txt`) or the
   SeedQR as a labelled PNG (`{wallet}-{fp}-seedqr.png` or
   `-compactseedqr.png`) go For the stick under Unprotected secrets,
   unticked on a stick visit; never the passphrase. Done when each
   seed's file is there.
5. **The wallet sheet** (descriptor, keys, first addresses, a PDF; the descriptor's QR code is left off a silent payments wallet's sheet while its key waits for its backup, the address printed as text) or,
   split, **The shares**: each sheet omits at most M−1 keys, so any
   quorum rebuilds and no one share watches, with the measured minimum
   rebuild group; each share as its PDF, its text and
   `{name}-share-k-of-n.png`. Made For the stick with the checklist,
   with **Remove**; done while it is For the stick or on the receipt.
   Shares, like the Coldcard multisig config, are written in a form that
   rebuilds the keys sorted (`sortedmulti`) and a Taproot multisig under
   the NUMS point (`tr(NUMS, sortedmulti_a)`), so they are offered only
   for those wallets: a `sortedmulti` in `wsh`, `sh` or `sh(wsh)`, and a
   `tr(NUMS, sortedmulti_a)` with its keys at `/<0;1>/*`. Any other
   wallet of several keys (`multi`, `multi_a`, another internal key,
   two keys of one seed, a miniscript, MuSig2) has the whole-wallet
   sheet with its full descriptor; for a multisig among them the board
   says why partial descriptions are not offered. Before shares or a
   config are written, Faraday rebuilds the wallet from them as Restore
   does (every quorum of sheets where there
   are at most ten, else the first and the last m) and compares the
   first receive and change addresses with the wallet's; on a
   difference nothing is written and a toast says so.
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
what it is ("vault.kdbx holds seed 9A6A2580"). With every item done,
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
(New key's words, or a passphrase added in Create or found in Vanity) signs nothing until
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
every wallet the app knows, once each by its id: loaded,
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
backup of the wallet opened again starts with its seeds checked.
A press on a place's row opens that place's sheet (§12) over Backups: a
place of paper with Seed XOR parts or SLIP-39 shares has a sheet too, and
its parts are counted there. An old Seed XOR part still kept (Make new
parts, F285) has a sheet with **I destroyed it** and no check. A Seed XOR part's sheet has **Check my copy**:
the words typed back are compared by their master fingerprint with the
part's recorded one, with no part kept here, so a part counts as
**Checked** after Back up is gone. A SLIP-39 share has no fingerprint of
its own kept, so its sheet has no check. A node's **Mark checked** dates
what a node holds except Seed XOR parts, SLIP-39 shares and codex32
strings: a copy is dated only by a check that matched. The wallet card's
line under its shape says the same in one line ("Backup:
paper ×2 checked · vault.kdbx · for the stick", or "Not backed up") and
opens Backups on that wallet.

## 6. One way every flow draws

The same steps and the same engine serve someone inheriting bitcoin, so
the same paper gives the same addresses and byte-identical signatures,
and every warning is word for word the same, from the same function.
That path is the **Spend** tab (`docs/SPEND-FROM-BACKUP.md`): its first question is
*What do you have?*, and its order, wording and folds are written there.

Every flow draws as Steps only drew it: a working screen carries labels,
values and actions and no explanation. The explanations are in the guide
(`docs/guide/`), opened by the help button at the step the person is on
(`docs/GUIDE-IN-APP.md`). A settings file with an old `guided=` line
still loads and the line is ignored.

## 7. Tools

Tools (`Screen::Catalog`, `faraday-core/src/catalog.rs`) is every flow
on one page, by what it works on, each tile with its line and the
standards it follows; **Find a tool** narrows the page to the tiles whose
name, line, group or standards hold what is typed. A tile opens its
flow, or names what it needs first ("Load a key first", "Make a vault
first"). Every feature has a tile here: the Check place's rows are a
second way into flows that also have one (`tools/lint-check-tiles.py`). The
groups and their tiles:

| Group | Tiles |
|---|---|
| Keys and seeds | Add a key · Scan a SeedQR · SLIP-39 shares · Restore from SLIP-39 · codex32 · Seed XOR · Explore a key · My seed makes my wallet · Sweep a private key · Convert a seed · Krux code · Child seeds and passwords (BIP-85) · Vanity address · Silent payments · Lightning node key |
| Wallets | Single-key wallet · Multisig · Taproot multisig · MuSig2 · Miniscript and inheritance · Restore an existing wallet · Back up a wallet · Read a Bitcoin Core wallet · Check a multisig setup |
| Transactions | Sign a transaction · Nonce check · Check a signer · Decode a transaction · Decode a Lightning invoice · Check an address |
| Messages | Sign a message · Check a signed message · Check a timestamp |
| Vaults and other keys | Vaults · TOTP codes · Diceware passphrase · GPG key · Secure Boot keys · KeePass export |
| Calculators | Hashes · Encodings · Descriptor checksum · Convert an xpub · Units · Miniscript compiler |
| Word lists | BIP-39 words · EFF large list · EFF short list 1 · EFF short list 2 |
| QR | Scan a QR code |

A check that needs a wallet and finds none loaded leads into Restore an
existing wallet rather than ending at a line: Tools' **Check an address**
opens it with **Tools** as its way back, and Decode's **Load a wallet**
(drawn while none is loaded) opens it with **Decode a transaction** as its
way back. The wallet is read through Restore's own description card, with
no new way in, and the moment it is loaded Restore ends and the check is
shown again: the scanner of Check an address, the decoded transaction with
its outputs named again against the wallet. Back leaves Restore for the
check with nothing loaded. The seeds are not asked for here.

The Descriptor checksum tool reads what is typed and shows its checksum,
the checksum given with it and whether it holds, the text with its
checksum, and whether this device loads it as a wallet. When the
descriptor uses one key twice it adds a **Keys** row in the warning tone,
one line per repeat: "Key 2 is the same as key 1", or "Key 1 is used
twice" for one key written twice at one derivation. Two keys are the same
when their extended keys hold the same public key and chain code,
whatever origin or depth they are written with. One key at disjoint
derivations (BIP-388's `@0/<0;1>/*` and `@0/<2;3>/*`) is one key used
twice on purpose, and is not reported. A descriptor that repeats no key
adds no row. Create a wallet refuses two slots holding the same key by
the same rule.

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

## 11. Rules the flows file held

The rules below were kept in `docs/FLOWS.md` and are held here; each is the
rule of its flow.

- **Every way in reaches a wallet card** (`docs/WALLETS.md` §2) that says what
  the wallet is, which keys can sign here, how many signatures the
  transaction has and needs, and the one thing still to do.
- **A `tr()` wallet states who can spend by the key path** (F132,
  `docs/WALLETS.md` §3.4) on its card, on Make a new wallet's Check and,
  for a key not of the wallet, on a sheet that takes a hold before the
  wallet is added and as a caution on every spend from it.
- **A transaction never stops at "can't sign"**: a missing key opens **Scan
  a key**, which takes any of the wallet's missing keys (below); a
  missing signature opens **Collect**; a missing wallet opens **Load the
  wallet**.
- **Signing ends on two results**, the signed PSBT for the wallet that wrote
  the transaction and the finished transaction with its txid, each shown as
  a code and put For the stick the moment the transaction is finished and
  its Review gone through, also when it arrived signed elsewhere
  (decision 24): Finish's primary is **Insert a stick to write it**
  (opening Files), or **Open Files** with a stick in, and each result's
  card carries **Remove** while it waits.
- **Anything added in a session offers Save to vault**, and locking lists
  what was not saved.
- **Creating a wallet ends in Backup and Save**, never on the descriptor
  alone.
- **Wallets with something loaded is the list and the card alone**
  (decision 12): every job (Sign a transaction, Back up, Show wallet QR,
  Sign a message) is reached from the wallet card, opened from a
  wallet's row, and from Tools. Wallets' ways in are Make a new wallet,
  Restore an existing wallet, **Spend from a backup, step by step**
  (offered with wallets loaded as well; the Spend tab lights **Wallets**
  in the sidebar) and last **Add a key**, "Keys first, wallets later"
  (decision 50).
- **Make a new wallet** (decisions 17, 20 and 37; F53, F55, F98, F240; `docs/CODEX32.md` §6.9, §14 D5).
  A single key's cards are Kind · Backup · Use vaults · Keys · Check; a
  multisig's Kind · Quorum · Use vaults · Keys · Check, **Back up
  wallet** at Check's foot for both. The flow opens on Kind, open, no row
  ticked (owner, 2026-11-09). Kind asks only who signs: **Single key**
  ("One key signs") and **Multisig** ("3 keys, any 2 sign", the
  quorum's counts), both native SegWit; the address types (Single key
  Taproot, nested, legacy; Multisig nested, legacy, Taproot) and MuSig2
  are behind **More kinds**. A kind pressed closes Kind and opens the
  next card, the one pressed again too. Next waits until a row is
  chosen; a Tools tile that opens Create with a kind chosen ticks it.
  Use vaults' **Yes** opens How many, **No** opens Keys; closed before
  it is answered it reads "Not answered".
- **A single key's Backup card** (F240; `docs/CODEX32.md` §6.9, §14 D5).
  A single key of any address type has a **Backup** card after Kind with
  **Words (BIP-39)** ("12 to 24 words on paper") and **Shares
  (codex32)** ("The key in shares on paper, a few of them rebuild it"),
  nothing ticked; a row pressed closes it and opens the next card (F103),
  Next waits until one is chosen, and closed it reads the form's name or
  "Not answered". An info button follows "(codex32)" in the Shares
  row; it and the word carry a tooltip (DESIGN §4.12, Tooltip; F112):
  "codex32 is the only method that makes a seed and its shares fully by
  hand, the checksum included." Keys stays closed until Backup is
  answered. A wallet made from another (New wallet from this one) has no
  Backup card. There is no shares Quorum: the codex32 scheme is asked on
  Make codex32 shares' own Scheme card. On chain the wallet is what the
  address type makes, one descriptor, one xpub. Check has a **Backup**
  row, "codex32 w0za · 2 of 3", for a key made here as codex32 shares,
  whatever its address type. Back up opens on codex32's standard way,
  one string per place on paper (`docs/CODEX32.md` §9.5; that way is
  planned). Restore does not offer the choice. Every kind whose path has an
  account has an **Account** row, at the foot of Quorum under the
  quorum's line, or at the head of Keys for a kind with no Quorum card:
  "Account" and its number, starting at 0, with **Change**, which opens
  the − n + stepper in its place. The card that carries it says the
  account in its row of Your choices ("2 of 3 · account 1") when it is not 0.
  Keys' **Make the wallet** makes it; Check shows the descriptor as a
  summary row that opens its code with the whole text under it, a
  **Path** row under it with **Change** (which takes the made wallet
  back and opens the Account row's card on its stepper), and the
  section label **The wallet's first addresses**
  above the address rows, so that a multisig's three rows do not read as
  one address per key; its press reads **It matches**, which leaves
  Check open with **Back up now** at its foot. There is no Back up
  card (owner, 2026-10-26): **Back up now** opens the backup on the
  new wallet (its chip "Then: Wallets", its way back the wallet's card)
  on The standard way. A single key made here is checked as it is made:
  Lock in opens Check with **Back up now** at its foot.
- **The Keys card** (decisions 35, 36 and 39, F282). The card is always
  shown; Keys is done with the key once it is locked in (owner,
  2026-10-24). A single key backed up as words, and Key 1 of a multisig
  while no slot holds a key made here, makes its key on the card, and
  any other empty multisig slot does through **New words**
  (`docs/FLOWS.md`). A key loaded here that
  fills one slot, or a cosigner's file whose key fills one slot, is not
  offered for any other slot; clearing the slot offers it again. A code
  scanned while a multisig's slot waits for a cosigner (one marked for
  later, else the first empty) fills it with an account key (an xpub
  file, a BIP 129 key record, a SLIP-132 key, a single-key JSON) or a
  descriptor that names one key, with its fingerprint and path, at the
  kind's derivation (any account) and, for a descriptor, under its
  script type, the path's coin type its xpub's network and that network
  the session's. A key with no origin or at another path is never taken
  for a slot, from a scan or from Files, since its path would be a
  guess; a descriptor of several keys is not split. What cannot be the
  slot's key (no origin, another kind or network, no key for the kind, a
  checksum that does not hold) stays in Files, the toast naming why;
  a key or wallet file in the Inbox that cannot be a slot's key is still
  listed on the card, disabled, with its reason under its name (a key
  with no origin: it needs its fingerprint and derivation path, which
  most wallets export). While a
  slot is empty there is no button at the foot: the first empty slot's
  own button leads (**Make the words**, **Make the wallet**, **Lock in** or **New key**); once no slot is empty the foot reads **Make the
  wallet**. A slot
  filled by a key loaded here from BIP-39 words with no passphrase has
  **Add a passphrase** (beside Show xpub QR on a multisig, on the slot's
  own row on a single key): **Passphrase**, **Passphrase again** and
  **Lock in** under the slot; Lock in adds the key the words and that
  passphrase make, labelled as the first with " · passphrase", puts it
  in the slot with a **Locked in** tag, and leaves the first key loaded.
  A key with a passphrase already, a cosigner's xpub, a share, and a key
  from SLIP-39, codex32 or a bare seed have no such button.
- **Sign a transaction reviews the transaction before Check** (decision
  19): the Transaction card comes before Check, carries the txid as one
  of its rows, and Check stays closed once a wallet has been checked this
  power-on.
- **Sign a transaction's review shows a fee bump** (F233): on the
  Transaction card, after the fee and before the warnings, the section
  "Fee bump" appears when Files holds another transaction spending the
  same inputs. It gives the result line, its lines, its facts and
  **Compare**, which opens Check a signer on the pair, and does not stop
  signing (`docs/COMPARE.md` §4.2).
- **The wallet card's first button** (decisions 43 and 45), primary, is
  **Spend from this wallet**; **Sign the PSBT from the stick** with a
  PSBT for this wallet in Files, which opens its review at once; **Carry
  on the spend · Signatures 1 of 2** while this wallet's spend is under
  way, as Home's lead says too; **Finish the backup first** while a key
  made here waits for its backup. Spend from this wallet opens the Spend
  tab on the wallet, past What do you have? (`docs/history/SCAN-TO-SIGN.md` §4):
  **Load the wallet in Sparrow** (the wallet QR, receive address 0, **It
  is loaded**; done already for a wallet checked this power-on) with **I
  have the PSBT from Sparrow ready: Scan it** over it, then **Write the
  payment in Sparrow** (**Scan the PSBT**, **On a stick**, this wallet's
  PSBTs in Files), then the spend's steps. Who signs, with fewer keys
  here than sign, offers **Scan a key**, its Unlock buttons and **Collect
  a signature**, which opens The other signatures. On a small panel the
  card's spend button sits under the signatures box, and the shortcut is
  its line and **Scan it**.
- **Scan a key; Can spend** (decision 57, `docs/history/SCAN-TO-SIGN.md` §2–§3).
  Where a wallet is short of keys, one **Scan a key** opens the camera at
  once, titled "Scan a key for {wallet}", a line per missing key ("Key 2
  · 5d388376", and " · {place}" where the backup's plan keeps that
  seed), with **Type the words instead** and Cancel. A seed whose key
  with no passphrase is a missing key loads at once, with no passphrase
  question: the camera stays on with "Key 2 added · 1 more" while more
  are needed, else closes with the toast "Key 2 added · Can spend". A key
  already here says "Key 1 · already here"; a seed of no key of the
  wallet closes the camera and opens the passphrase sheet (SeedQR read)
  with "{fp} is not one of {wallet}'s keys", which takes only a
  passphrase giving a missing key and refuses another with the same
  line; anything else shows on the read card, as at Scan a SeedQR ("A wallet
  description", **Load the wallet**). A codex32 string or a SLIP-39
  share opens its form in "Add a key for {wallet}" with the part in,
  which takes the key once whole only as one of the wallet's keys. **Type the
  words instead** opens "Add a key for {wallet}", which refuses words of
  no missing key with "These words give {fp}, not a key of {wallet}". A
  key loads under its holder's name where the plan keeps one, else none;
  a name that is the fingerprint is never drawn beside it. Scan a key is
  on the wallet card's signatures bar in place of "Add 1 more key or
  collect 1 more signature" (full width under the line on a small
  panel), on the Wallets column as **Scan a key for {wallet}** in place
  of **Add a key** while the wallet shown is short, and on Who signs (the
  Spend tab's and Wallets' Sign step). A wallet with enough keys here to
  sign shows a green **Can spend** on the card's bar (in place of
  "Nothing missing"), on Who signs beside "{here} of {needed} signatures
  here", and at the right of its Wallets row, whose sub-line is then its
  shape alone; the small panel's list and Home's say "Can spend · 2 of
  2". Short, Who signs says "1 of 2 signatures here · 1 more needed" and
  a key not here **Not here**; its Scan a key, Unlock and Collect a
  signature show only while short, or, for a miniscript or tree wallet,
  until the spend is signed.
- **The backup comes before spending** (decision 45, F6). Back up's
  checklist: an item's foot button is its next action (a vault item's
  **Make a vault** or the way to one, then one **Save into {vault}**
  that saves every seed, passphrase and the description the plan puts
  there, then **Continue**; a copy's **Check my copy**, which types the
  words back for a words copy and scans a SeedQR copy; files' **Make
  it**), and the items after one not done do not open. A plan waits
  while a vault's stick has no place ("Vault 1's stick needs a place");
  a plan that keeps no copy of a key made here is not made into a
  checklist ("Seed {fp} is kept nowhere"); **Change the plan** stays
  under every item, and done work the new plan drops stays on the map. A
  key made here signs once its checklist is complete: the wallet card's
  first button is **Finish the backup first**, Who signs lists it
  **Waiting for its backup** with **Finish the backup**, and the card's
  address, Create's Check, the Key card's address and the chart's wallet
  node say "Back up before you receive". The done card, the wallet card
  and the open vault lead with the plan's three checks as a strip above
  the chart.
- **Shares and vaults in a plan** (decisions 40 and 41). A multisig's
  shares take **Keys left off each share**, a slider from 0 to m − 1,
  then the shares' fingerprints and the split audit, with **What is a
  share?** opening Learn on *Shares of a wallet description* (closing
  Learn leaves the plan where it was); the checklist's Shares card has
  the same slider. Each vault of a plan holds the seeds ticked into it,
  vault *v* the *v*th seed by default, a seed in more than one where
  ticked; each vault's stick has a place, by default one that adds no
  second key. The map names each vault ("Vault 1 · 9a6a2580"). The
  checklist has one item per vault, **Vault 1: 9a6a2580**, which makes a
  vault and saves its seeds and the wallet; an open vault holding
  another vault's seed is not offered there (**Lock it and make a new
  vault**). A stick visit with several vault files waiting ticks one.
- **Create's Keys card offers the open vaults' keys** (owner,
  2026-10-19; F45; `docs/history/NEW-WALLET.md` §16.1). Under each slot's
  session keys, each open vault's keys not loaded, under the vault's
  name (a key in another slot is not offered); a press loads that one key as Load into session does
  and fills the slot, the vault staying open. A locked vault whose file
  is read is one **Unlock {vault}** button, coming back to the Keys card
  with the vault open and nothing loaded.
- **New wallet from this one** (owner, 2026-10-19; F45;
  `docs/history/NEW-WALLET.md` §16.2). A wallet's card offers it beside
  Rename (on a small panel, in its list of buttons) for every kind
  Create makes. It
  opens Create filled from the wallet: Kind and Quorum closed with their
  values, the **Account** row (on Quorum, or at the head of Keys, as for
  any new wallet) starting at the lowest account no loaded wallet of the
  same keys and kind uses; each
  slot its key, loaded from an open vault where it is kept there; a
  cosigner's key held only as an xpub stays at the wallet's own account
  and script type, and otherwise its slot is Someone else reading
  "Needs Key n's xpub at account n". Use vaults is asked as for any new
  wallet; the name proposed is "{wallet} · account n" ("{kind} · account
  n" once the kind changed). The wallet it came from is not changed.
- **The backup starts with a plan, then a checklist of only what it
  needs** (owner, 2026-10-09, proposal A; `docs/WALLETS.md` §5). The
  plan opens on The standard way, on the board, with **Add watch-only
  software** and **Customize backup** (F53, `docs/BACKUP-FLOW.md`
  §17.21; the three presets Paper only, Paper and vault, and Paper,
  vault and watch-only software are withdrawn). Before F53 the presets
  filled the plan's questions, each a multi-choice
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
- **Sign a message's Message step takes the text from a scan.** **Scan**
  sits beside **Type**: a plain text read, or Paste on a shell with a
  clipboard, becomes the message as read, in place of what was typed.
  Every other kind of code, a seed, a private key or any other secret
  among them, is named on the scanner ("{kind} · not used here") and
  refused, and no text is kept. Home's Scan of a text offers **Sign this
  message** while a single-key wallet is loaded, and it opens this flow
  with the text in (`docs/SCANNING.md` §4.3).
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
  payments address and record (with "Not checked: load key {fp} to check
  this address" while a wallet's key is not loaded; no code where a loaded
  key with its fingerprint gives another scan key or spend key, and a
  record that names another fingerprint over the points a loaded key gives
  at its account or at account 0 is refused, or taken out of the session
  when that key loads), `Signed by {address}` for a message, the
  wallet's shape and keys for its files. A file past one code says so
  instead of writing parts; a PSBT, a transaction, a file shown from
  Files and a secret offer no picture.
- **Receive addresses as codes** (F28; `docs/QR.md` §7). A receive
  address row opens its code when pressed, a chevron at its end, as
  Create's descriptor row opens the wallet's: the wallet card's first
  receive address (and the small panel's), the first addresses of
  Create's Check, Restore's Check and the chart sheet's wallet, the
  Spend tab's **Receive address 0** and **Check the money**'s receive
  rows, and Vanity address's found address. The code is the address as printed,
  shown whole under it; no PNG. A change address, the signing steps'
  **Addresses · Compare** and a wallet whose backup is not done ("Back
  up before you receive") open none.
- **A key in no wallet has a way on**: the card's buttons above with no
  wallet loaded; with wallets loaded, the key's row in the left column
  carries **Make a wallet from this key**. The left column runs top
  down, wallets, keys in no wallet, then its buttons, and scrolls when
  taller than the screen.
- **The wallet's card shows it at a glance** (owner, 2026-10-10;
  `docs/history/NEW-WALLET.md` §6). Its Keys list becomes **At a glance**, a
  structure chart (DESIGN §4.16): the wallet (its name and policy id; the shape is in the card's
  heading, once);
  its keys, numbered as before, each with its fingerprint, label and
  where it is ("Can sign here", "Can sign · passphrase", "Cosigner ·
  xpub only", "On its own device"); its backup, a node per spot of the
  plan's map, each listing what it holds ("Key 1 words", "Vault 2
  stick", "Key 2 seed + passphrase", "Description"), its tag, what
  Backups saw of it, and what it gives whoever finds it alone
  ("Cannot see balance", "Sees the balance", "Can spend", "Can spend with the
  vault's passphrase"; a vault "Sealed", with where its stick is). A
  line runs from the wallet to each key and from each key held here to
  every node holding its seed, a colour and dash pattern per key; a
  cosigner's key has none, and its seed is in no node. Above it the
  plan's three checks as a strip (decision 45). The plan is the backup's once its checklist
  is made, else the one recorded this power-on (kept across a lock),
  else an open vault's. No plan: "No backup plan" (the card's **Back up** is the one that starts it); a
  plan in a locked vault not remembered: "Plan in {vault}, locked" with
  **Unlock**. Every node opens its sheet (below), which lists each line
  of what a backup node holds, each a touch target tall; while the wallet is short of keys a
  key not here reads **Not here**, and one an open vault holds keeps
  **Load from vault** as a line of its own; the one way to scan any
  missing key is the signatures bar's **Scan a key**. The card's
  body scrolls between its head and its buttons. On a small panel an
  **At a glance** row opens the chart as its own page, one column, no
  lines, each backup node naming the keys it holds ("Key 1 words · Key
  2 in the vault"); Back returns to the card. A wallet with a relative
  wait (`older`) has a **Spend paths** group under the signatures: a row
  per path with its keys and when it can spend ("Any time", "52 560
  blocks after each coin arrives · about 365 days") and **Move each coin
  within**, 90 % of the shortest wait in whole days (`docs/BACKUP-FLOW.md`
  §18.4); on a small panel a **Spend paths** row opens it as its own
  page, and the chart's wallet sheet shows the same rows. The card and
  that sheet offer **I moved every coin**, which dates a vault note
  ("Coins moved today"; "Open a vault to keep this" with none open, and
  **Set the time** in its place with the clock not checked), and a
  **Coins last moved** row reads "300 days ago · the wait ends at about
  365 days" (without "about" for a wait in 512-second units), in the
  warning tone once the age reaches the margin.
- **The chart's sheets** (decision 44). A press on a node of the chart,
  on the card or in an open vault, or a line in a backup node's sheet,
  opens a sheet (DESIGN §4.16): its title, what it is and where
  (Shape and Checksum; Where, Its seed is at; Stick, Found alone, Seen
  here; Kept at), then its actions as menu rows, each leading into the
  flow that does it, and Close (Back on a small panel, where the sheet is
  the page). Opening a sheet changes nothing and puts nothing For the
  stick; nor does opening the backup's checklist from the chart, whose
  items each offer what they make. The wallet: **Open in Wallets** (in a
  vault), **Show wallet QR**, **Public files** (the checklist's item),
  **Print the wallet sheet again** (or the shares), **Rename the wallet**
  (its card's name field), **Change the plan** (the wallet's card flow),
  **Check addresses** (the
  descriptor row and the first addresses, as Create's Check). A key held
  here, or kept in the open vault: **Show xpub QR**, **Xpub file**,
  **Explore this key**, **Where it is** (its lines bold, the rest dimmed,
  until the next screen or sheet); **Make another backup of this seed**:
  **By hand** (the checklist's copy, with Seed XOR and codex32), **Into
  Vault n**, **As a file**, each where the plan has that item; **Check a
  copy of this seed**: **Type my copy** for a words copy, which opens the
  typed check at the checklist's copy item, and **Scan my copy** for a
  SeedQR copy with a camera, which leaves the chart under the camera. A
  key not here: **Load this key** with its way in, where the quorum needs
  it; **Show its xpub**, from the wallet's descriptor. A place: its
  lines, each opening its own sheet; **Rename**; **Change in the plan**;
  **What goes in its envelope**; **The envelope list** (a PDF For the stick, places by
  number, never by the names the vault keeps). A vault: the vault way or
  **Open the vault**, **Save its seeds into it**, **Change in the plan**,
  **Write it out again** while it has changed since it was written. Files: **Write the seed files again**, **Write
  the public files again**. Software: **Show the descriptor QR again**,
  **Public files for this software**. Lines: words, **Type my copy**;
  SeedQR, **Scan my copy** (**Type my copy** with no camera); **Make
  another copy**; the wallet sheet, **Print it again**, **As a QR
  picture**, **As text**; a share, **Print the shares again**, **Keys
  left off each share**; a vault's stick, the vault way and **Copy the
  vault to another stick**; a seed file, **Write it again**; the
  description, **Show wallet QR**. Every key, place, line and mark sheet,
  and a SeedQR drawn in part, has one **Change in the plan** row, which
  opens the backup board (`docs/BACKUP-FLOW.md` §17) with that place or
  thing selected; the sheet itself moves, copies, adds and removes
  nothing. A flow on Back up comes back to the card, or to the vault
  opened from.
- **The board changes the plan** (decision 49). The backup board
  (`docs/BACKUP-FLOW.md` §17) is the one editor of the plan's map: a
  move, another copy, a thing added at a place, **Into Vault n**, **As a
  file**, a place removed and a line taken off the plan are made there,
  and the chart's sheets offer only **Change in the plan**. A place that
  holds something asks on its × and lists what it holds; it goes on
  **Take off**. A SeedQR drawn in part has its toggle on the paper chip:
  it reads on, and pressing it destroys the grid, the plan's map as it
  was. After a save, a plan already made opens only the item for what
  was added, and the flow that makes it follows. From the first change
  the map is the plan: the backup's record and the vault's plan record
  keep it, the chart and the checklist read it, and it loads back the
  same on another power-on. **Change the plan** (the wallet's sheet, the
  checklist's foot) opens the plan's cards on that map, nothing
  replaced. In the vault alone, never in the kept state or on disk
  outside the vault: the date each
  thing of a place was checked (a place's **Mark checked**; a passphrase
  checked), **Lost** and **Exposed** (a line, or **Mark lost**, **Mark
  exposed** on a place: struck through on the chart until taken off the
  plan), a key not held here's **Name its holder** and **Mark checked**
  (the date its holder confirmed it), a place's names and a vault's
  **Rename**; with no vault open these rows read "Open a vault to keep
  this" and lead to one. A check of a copy matched in the checklist with no
  vault open is to be held in RAM only until the next unlock, then
  written into the plan's notes (`docs/BACKUP-FLOW.md` §18.3, planned). A mark says what follows, in order: "Someone can
  now spend" (or "could … with a vault's passphrase") with **Move the
  money** (Make a new wallet, opened on Kind with no kind ticked, the
  person choosing one, owner 2026-10-24; its last card then offers
  **Move the money from {wallet}**, the spend from the old wallet with "Every coin
  to {new wallet}" and its first address on Write the payment); "The
  wallet can no longer be rebuilt from what is left" with **Back up again
  now**; "Someone can see the balance"; "Nothing lost that is not
  elsewhere". **Check it** on a
  passphrase line (or the key's **Its passphrase**): typed into a masked
  field, the key it makes with the seed here compared by fingerprint, the
  typed text wiped at once; "Matches: it makes key {fp}" or "Does not
  match", and a match dates the line in the vault.
- **Backups age** (`docs/BACKUP-FLOW.md` §18, F87). Every check of a
  copy (Check my copy typed or scanned, a codex32 string's check, a
  passphrase's Check it, a place's Mark checked, a vault's stick read and
  unlocked) dates that copy in the plan's notes in the vault, asking
  **Which copy was this?** where the seed has that form at more than one
  place; a date is written only from a checked clock (`docs/TOTP.md`
  §1.3). The chart's place nodes, their sheets, the card's strip ("To
  check: Bank box", or "Every place checked in the last 12 months") and
  Backups show each copy's age ("Last checked 14 months ago") and mark it
  **To check** after 12 months, or sooner by a recovery wallet's margin
  (90 % of its shortest relative wait); with the clock not checked they
  show dates only, and the strip offers **Set the time**. A stick read
  from an older copy of its vault reads **Older copy** on the chart and
  Backups. A wallet with a relative timelock shows its **Spend paths** on
  the card, the chart's wallet sheet and the wallet sheet PDF, each wait
  in blocks and days with **Move each coin within**, **I moved every
  coin** and **Coins last moved** (the Wallets tab above), and the PDF a
  table to fill by hand per deposit.
- **Restore** (decision 48): **Kind** (Single key · native SegWit
  ticked, Multisig · native SegWit, More kinds), with **I have the wallet
  description: Scan it** and **From Files** above it; **Quorum** for a
  multisig (M of N on two sliders, 2 of 3 to start); **Description**
  (Scan it, From a file in Files, the shares, the stick or a vault; for a
  single key **I have the seed** first, for a multisig **I do not have
  it**); **Keys**, a slot per key, whose ways in (Type the words, Scan a
  SeedQR, a vault's seed or a key already loaded, **Its xpub** with no
  description or **Not here** with one) stand over the slot they fill,
  the passphrase asked after the seed (SeedQR read; typed words take it
  on Add a key); **Check** (the descriptor row and the first addresses);
  and **Done**. A description read on Kind or Description fills Kind and
  Quorum and opens Keys. With the description known each seed is matched
  to its keys as it is entered: a match reads "Key 2 of the wallet"; any
  other reads "Not a key of this wallet: check the words or the
  passphrase" and is not loaded. **Make the wallet** once every slot is
  filled. Done offers, all
  secondary, **Spend from this wallet**, **Back up this wallet**,
  **Another wallet from these seeds** (New wallet from this one) and
  **Wallets**. A restored key is not held for its backup. Vaults set up
  before the seeds wait in the wallet's flow: Back up resumes it at the
  plan with them, and until a plan is made the wallet's card and Backups
  read **Continue the backup**.

## 12. The chart's sheets

The wallet's chart of the plan (the card's glance, an open vault's page,
the checklist's done card) is also where the backup is looked after. A
press on a node, or on a line inside a node, opens a sheet; on the small
panel the sheet is the page and Back returns to the chart. The sheets and
their rows are listed under "The chart's sheets" in §11; the rules below
hold for every one.

- **A sheet changes nothing.** It says what the thing is, where it is and
  when it was last checked here, then offers each action as a row leading
  into the flow that does it. It moves, copies, adds and removes nothing:
  every key, place, line and mark sheet has one **Change in the plan**
  row, which opens the backup board with that thing selected. The board
  (`docs/BACKUP-FLOW.md` §17) is the one editor of the plan's map.
- **A secret copy is made only by the flows that exist**: by hand with the
  copy check, into a vault, or as a file behind its warning. A file goes
  to For the stick, as everywhere.
- **A secret action needs the seed loaded here.** A key not loaded
  offers **Load this key** with its way in, where the quorum needs it;
  a key not held here (a cosigner's) offers nothing secret: **Show its
  xpub**, **Name its holder**, **Mark checked**.
- **What is kept, and only in a vault.** The plan's vault record (type 11,
  `docs/VAULT.md`) keeps per thing the date it was last checked here, a
  key not held here's holder name, a place's or vault's name, and a mark
  (lost, exposed). Never a secret: a passphrase is checked by the
  fingerprint it makes, typed into a masked field and wiped at once, and
  only "checked on" is kept. With no vault open those rows read "Open a
  vault to keep this".
- **A Seed XOR part's check is by fingerprint.** A part's sheet offers
  **Check my copy** (Back up's check of that part, then back to the
  chart); the typed words are compared with the part's recorded master
  fingerprint and wiped, so the check works once the on-screen parts are
  gone. A SLIP-39 share's sheet has no check and
  no **Mark checked**.
- **A copy's check follows its form.** A words line opens the typed check
  (**Type my copy**); a SeedQR line opens the camera (**Scan my copy**,
  **Type my copy** with no camera). A key node offers one row per form
  its copies have (§5, the copy item).
- **Lost and exposed** run the check with that place gone, or found by
  someone, and say what follows, in this order: someone can now spend (or
  could with a vault's passphrase) with **Move the money**, which opens
  Make a new wallet and then the spend of every coin to it, in `ERR`; the
  wallet can no longer be rebuilt from what is left, with **Back up again
  now**, in `ERR`; someone can see the balance; nothing lost that is not
  elsewhere. A marked thing stays on the chart, struck through, until it
  is taken off the plan on the board.
- **The map is the plan once edited.** The plan's answers spread things
  over the places by rule; an edit on the board is not a rule, so from the
  first the map itself is the plan: the backup record's and the vault's
  plan record's map lines are read back as the spots and the check runs on
  them. **Change the plan** opens the answers' cards on that map, and
  nothing is replaced.

## 13. The backup before the spend

A key made here (its words came from New key, or a passphrase was added
to it in Create) cannot sign a spend until a checklist that keeps its seed,
and its passphrase, is complete. Receiving is warned about, not blocked. A
key restored from words, a SeedQR or a vault with no backup pending is
already backed up and signs at once. A message waits as a spend does,
since the signed message carries the address: Sign a message does not
offer a wallet or a picked address whose key is held (its row, button or
tile says "Its key waits for its backup", or is left out), every press
that would sign, or make the file, the code, its sheet or **Check it**,
is refused, and a key held after signing drops the signature and what
**Check it** kept. A signed message's file already For the stick stays
there.

- **What waits.** The wallet card's first button reads **Finish the backup
  first** and opens the wallet's checklist (its plan when none is made); in
  a spend, Who signs lists the key "Waiting for its backup" with **Finish
  the backup**. No receive address or code of the wallet is shown in
  Create's and Restore's Check, the card, the Key card, the chart's wallet
  node, the Spend tab's family pages or the message address, and in a
  silent payments wallet's card, its item in an open vault, the Silent
  payments page with its Your choices row, the Krux code tool (which
  refuses a held address, and a record that does not match its key), and
  the Vanity tool's found address (the address, the codes of it and its DNS record, and the
  wallet's descriptor, which is its address, as a code and as a picture
  among its public files; the record's own code carries the line first, as
  a descriptor's does): "Back up before you receive" stands in its place,
  in `WARN`, until the hold is released. Only the receive path reads it: a
  silent payments wallet has no key in its plan, so its own backup places
  no seed and releases nothing, and its plan, Who signs and the plan
  record's `held` line do not name its key; an ordinary wallet's backup of
  the key, finished, its vault included, releases the hold. While the
  Silent payments page's address is held, its address card has **Back up**
  (**Continue the backup** where one is left part way), which opens the
  backup of the first ordinary wallet that places the key's seed, as a
  vault item's Back it up does; Back returns to the page. With no such
  wallet loaded the card has no button (saving the key
  in a vault only marks it pending, which holds it again at unlock). While
  the hold stands the plan of a silent payments wallet reads its
  description as the text file only, wherever the answers keep a QR
  picture: the checklist, Write to sticks, the public slot, the names, the
  files made, the board, the map, Backups and the plan record saved in a
  vault all say and do the text file (the board's toggles edit the stored
  answer; its read-only mark says the text file). The answers as chosen are
  not changed. The lock of a backup that writes the text file in the
  picture's place marks each stick it writes so, and the software's place
  where the public files are made as the text file: the mark is the item's
  in the plan (the vault's `+txt` on its `spot` line, `docs/VAULT.md` §7),
  so it goes with the item when a place or a stick before it is removed or
  a file is added, and is dropped with the item when it is dropped,
  cleared or moved to another stick. The next lock clears it before it
  marks what that lock writes, so a stick written again as a picture reads
  as one. Marked, the stick reads as the text file in the map, the
  checklist's map lines, the envelopes, the board's mark, Backups and the
  plan record's holds lines after the hold is released too, though the
  chosen form alone decides what the next backup makes: the checklist's
  stick rows and its public file buttons say what will be made, the
  picture. The mark is kept with the plan's answers, in the backup's own
  sticks, in Backups' record (synced after every lock of a silent
  payments wallet) and in the plan record of each vault that lock saves
  the plan in (the one read first, the flow's and the ones written); a
  vault closed, or open and not of the flow, keeps its older plan whole.
  The plan text carries `+txt` only for a silent payments wallet: for any
  other wallet it is refused, and an ordinary wallet's record is
  unchanged. An older Faraday does not know `+txt` and refuses the whole
  plan text: its Backups shows no plan for the wallet, nothing warns, and
  its next save of the plan writes the default one over it (the vault,
  keys, wallet and notes still read). A backup locked while held and given
  up with a stick unwritten says the text file for that stick until the
  next lock. A backup past its lock keeps, in its own map and board, the
  form its files were made in; one not yet locked and kept over a restart
  follows the release once Back up is open on its wallet.
- **Which keys, across a lock.** Which keys are made here is known by
  fingerprint only (`faraday-core/src/held.rs`): in memory while the key is
  here, as a flag in the key's vault record beside its seed, and as a
  `held` line in the plan's record. A key loaded from a vault that says so
  is held again. A hold stays until every item of the checklist that keeps
  the seed is done; the checklist's last item writes the sticks inside the
  flow, so nothing waits For the stick after it.
- **The plan says so.** The plan's first card says "A key made here signs
  only once this backup is done." when the wallet has a key made here.
- **A checklist item's foot is its next action** until it is done, then
  **Continue**: a vault item, **Make a vault** (or the way to one), then one
  **Save into {vault}** that saves each seed, passphrase and the wallet
  description the plan puts there; a copy, **Check my copy**; files,
  **Make it**. An item not done cannot be passed; the items after it open
  once it is, each saying "After: {item}".
- **Every vault's stick has a place.** The plan gives a vault's stick the
  place holding the fewest keys when nothing else places it; Places'
  Continue, while a vault made has no stick ticked, is replaced by "Vault
  {n}'s stick needs a place", and a check reading "Any one place lost: No"
  says why under it on the map panel.
- **Changing the plan part way.** **Change the plan** stays under every
  item, the one way out of a forced one. Items the new plan still has stay
  done; work done that it drops stays on the map as what it is until the
  copy is destroyed; a plan that keeps no copy of a key made here is not
  made into a checklist ("Seed {fp} is kept nowhere"); the hold follows the
  plan now chosen.
- **The three checks lead.** The plan's three lines ("Any one place lost",
  "One place found: can spend", "One place found: sees the balance") are
  drawn first wherever the backup is shown whole: a strip of three, each
  value large in its colour, on the done card above the wallet's chart, on
  the wallet card and in the open vault above the chart, and stacked as
  rows at the top of the chart's page on the small panel (`glance::check_lines`
  draws all of them).
- **A copy by hand is checked by its form.** A words copy by typing the
  whole seed back, a SeedQR copy by scanning the drawn grid (typed words
  with no camera); with both forms both are needed. The typed check hides
  the seed, takes words, four-letter prefixes or four-digit numbers (as
  Show the seed prints them), masks each word once taken, and compares
  only when the last word is in; a mismatch names the first word that
  differs ("Word 5 differs from the seed"), keeps the words before it and
  resumes there. The scan compares word by word and names the first that
  differs. A passphrase on paper is its own item after its seed's copy,
  checked by typing it back whole, hidden, and compared exactly; neither
  check is kept beyond this backup, and the typed text is wiped on a match,
  a mismatch, Clear, Show, leaving the item and a lock. After three scans
  of one seed's SeedQR without a match, **Keep the words only** joins
  **Check my copy** at the foot: it turns that seed's SeedQR copies into
  words by hand, and a grid already drawn stays on the map marked "drawn in
  part", counted as a secret at its place for "One place found" and not as
  a copy that rebuilds, until **I destroyed this copy** takes it off.
- **Vaults come before the keys.** Make a new wallet and Restore ask after
  Kind whether vaults are part of the backup and set each up (stick checked,
  vault unlocked or made); the plan comes after the keys with those vaults
  as its vaults, and a plan that wants one not set up takes one guided
  round inside the flow (`docs/BACKUP-FLOW.md` §4.8). When every secret of
  the wallet sits at one place the check lines say so in `WARN`; they
  inform and never block.

## 14. Keys from a vault, a wallet from another, the path on Check

- **An open vault's keys on the Keys card.** Under each slot's session
  keys, each open vault's keys not loaded, under the vault's name; a
  threshold kind lists only the vault's shares, and a key already in
  another slot is not offered. A press loads that key with the passphrase
  its record stores and fills the slot; the vault stays open and nothing in
  it changes. A locked vault whose file is read is one **Unlock {vault}**
  button, returning to the Keys card with the vault open and nothing loaded.
- **New wallet from this one.** The wallet card's button opens Make a new
  wallet filled from that wallet, each card closed with its value and
  **Change**: Kind, Quorum, the **Account** (starting at the lowest no
  loaded wallet of the same keys and kind uses, so Continue throughout makes
  a separate wallet) and each Key slot. A key here, or in an open vault,
  gives its account key at the chosen account (`NewKind::path_at`); a
  cosigner's key held only as an xpub cannot, so at the wallet's own account
  and script type it stays and otherwise its slot is Someone else, reading
  "Needs Key n's xpub at account n". The name proposed is "{wallet} ·
  account n". The wallet it came from is not changed; there is no Backup
  card for a wallet made from another.
- **The account on any new wallet.** Every kind whose path has an account
  has an **Account** row, 0 to start, with **Change** opening a stepper:
  on Quorum, or at the head of Keys for a kind with no Quorum card. `m/45'`
  has none, and FROST's shares are dealt, not derived, so neither has the
  row or a Path. Its row in Your choices says "account n" when n is not 0. Only the
  account changes; the rest is the kind's standard path (BIP 84, 86, 49,
  44, 48).
- **Check's Path row** sits under the descriptor row: **Path** and the path
  in the mono face (`m/84'/0'/0'`). When the keys' paths differ, each key's
  fingerprint and path, separated by " · "; a key with no origin reads "no
  path given". It is worked out once, when the wallet is made
  (`create::paths_text`).
- **Change beside the path takes the wallet back**: the card with the
  Account row opens on its stepper, Check and Keys are no longer done, the
  keys stay in their slots, and Keys' Continue makes the wallet again at the
  account then chosen. A wallet Create added is removed from the session;
  one found already loaded stays. The setup flow of the vaults is put back
  as it was, and a move of money waiting for the wallet forgets it. Change
  is offered only while nothing has used the wallet: its backup has not
  begun, the setup flow is where it was when the wallet was made, and the
  wallet is the last one added; otherwise the row shows the path alone.
