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
| **A wallet description arriving** (scan, file, Inbox; any form in §3.2) | The Wallets tab derives each vault key's account key at the descriptor's paths and compares, attaches every match, and marks the rest watch-only. "This wallet uses 73c5da0a from Main" says which matched. **Save the wallet to a vault** is offered. |
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
| **Transaction** | A PSBT by scan, Inbox file or typing. The review in sentences with the full table one choice away; every warning from one function, never softened for guided mode (§3.3). |
| **Txid** | The transaction id before anyone signs, for native SegWit and Taproot spends; refused for legacy and P2SH-wrapped inputs, whose id changes on signing. |
| **Sign** | Signs with every key held, producing one partly signed PSBT per key, the form separate devices would give. |
| **Collect** | **Signatures for this transaction**: cosigners' PSBTs by scan, file or Inbox. Each signature is verified against its key and this transaction's sighash; a PSBT for another transaction is refused, never merged. The collection belongs to the transaction, so reviewing again keeps it. |
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
  read from the Inbox, each kept only if it hashes to an outpoint being
  spent. It opens itself when the transaction needs it.
- **Anything other than `SIGHASH_ALL`** stops, and those inputs are refused.
- Fee, dust, network, unknown paths and the deterministic nonce check, as
  OpenSigner already shows them.

## 4. Spending

The Wallets tab's order, kept for every kind: **load the wallet → check →
the transaction → the signers → sign → collect → finish.** Everything
before the signers is watch-only, so any reason to stop is found before a
key is used. In Faraday the keys are usually already loaded from a vault,
so the signers step shows them rather than asking for them, and asks only
for what is missing.

| Kind | Flow | What differs |
|---|---|---|
| Single-sig | Load (or the vault's wallet) → Check → Transaction → Txid → Sign → Finish | No Signers or Collect: the vault key is the wallet. |
| Multisig (`wsh`, `sh(wsh)`) | Load → Check → Transaction → Txid → Signers → Sign → Collect → Finish | The Wallets tab's flow as it stands. Txid refused for `sh(wsh)`. |
| Taproot multisig, `tr(H, sortedmulti_a)` | as multisig | Script-path signatures only (the internal key is unspendable); the review says which leaf. |
| Miniscript, Taproot tree (imported) | Load → Check → Transaction → Signers → Sign → Collect → Finish | Signs every signature its keys can give; Finish completes only when the script is satisfied, and says which condition is unmet (a cosigner, or a timelock not yet reached). |
| Recovery (Liana shapes) | Load → Check → **Path** → Transaction → Signers → Sign → Collect → Finish | **Path**: the primary path, or a recovery path, with its timelock in blocks and whether the transaction's inputs have aged enough (from the PSBT's sequence values). |
| MuSig2, `tr(musig(…))` | Load → Check → Transaction → **Round 1: nonces** → Collect nonces → **Round 2: sign** → Collect partial signatures → Finish | Two rounds; every signer must take part. See §4.2. |
| FROST, m of n | Load (group record) → Check → Transaction → **Round 1** → Collect commitments → **Round 2** → Collect shares → Finish | Two rounds among the m chosen signers. See §4.2. |
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
  *different* transaction with a nonce already spent: it can only reproduce
  the identical partial signature. The flow says this before it seals.

This replaces OpenSigner's FROST carry file, which today holds the bound
secret nonce unencrypted, with the vault's encryption.

## 5. Creating and backing up

| Kind | Flow |
|---|---|
| Single-sig | Kind (script type) → Key (from a vault, new, typed, scanned) → Account → Check → Backup → Save |
| Multisig, Taproot multisig | Kind → Quorum → Slots → Build → Check → Backup → Save |
| MuSig2 | Kind → Slots (N of N; every key's account key) → Build → Check → Backup → Save |
| FROST | Kind → Quorum → **Deal** (on this device, as OpenSigner does) → each share to its own vault slot or exported to its holder → Check → Backup → Save |
| Recovery | Kind → Primary keys → **Recovery paths** (up to three, each with keys and a timelock) → Build → Check → Backup → Save |

**Backup** keeps four steps, adjusted for a machine with no printer:

1. **The blank sheets.** A template holding no secret (numbered word
   lines, path, network, the SeedQR grid's fixed squares) goes to the
   Outbox as a file to print anywhere.
2. **The seeds: by hand; into a vault; a file only past the secret
   sheet.** Words and the SeedQR grid to copy on paper, behind
   hold-to-reveal; never a printer. No line for a BIP-39 passphrase, and
   the screen says why. **Scan my copy** checks
   the copy afterwards: the camera reads the drawn SeedQR (Standard or
   Compact), compares its words with the seed on screen in place (never
   loaded as a key, never put in Files) and names the first word that
   differs. With no camera, **Type my copy's numbers** checks it from the
   four-digit number beside each word, typed back in order. Under each
   seed, after the copy by hand (2026-10-09): **Save into {vault}**
   writes the key record Vaults' Save writes (`kind::KEY`, loaded at
   unlock), and **Save into {vault} with its passphrase** when the key was
   loaded with one; the row reads "In {vault}" once the vault holds it,
   and with no vault open it says to unlock or make one on Vaults.
   **Save as a file…** opens the secret sheet for the seed: "Save into
   {vault}" there writes the same key record, never a note; or, once
   "I understand: anyone who copies the stick or sees this file can
   spend these coins" is ticked, the file goes to the Outbox under
   Unprotected secrets, unticked on a stick visit. The file is one of
   two forms picked on the sheet, the words (`{wallet}-{fp}-words.txt`,
   read back by Add a key and other signers) or the SeedQR in the form
   picked on the step as a labelled PNG
   (`{wallet}-{fp}-seedqr.png` or `-compactseedqr.png`, "Seed {fp}"
   over "{n} words · secret: spends", read back by Add a key's
   scanner); never both, and never the passphrase.
3. **The wallet in public keys.** The descriptor as one code or animated
   (one code while version ≤ 25 at ECL M or better), the same code as a
   labelled PNG in the Outbox (`{name}-descriptor.png`: the wallet's
   name, shape, key fingerprints and descriptor checksum under the code;
   past that bound, one labelled picture per BBQr part), and a
   backup sheet (descriptor, keys, first addresses) as a file in the
   Outbox. For a multisig, **Split between signers**: a split plan
   where each sheet omits at most M−1 keys, so any quorum can rebuild and no
   single holder can watch. The sheet states the measured minimum rebuild
   group, and that this is not secret sharing. Each share goes out as its
   PDF, its text and `{name}-share-k-of-n.png`, the share's text as one
   code labelled with the keys it holds and leaves off; a scan of it is
   the share, as the text file is.
4. **The envelope.** Which sheet goes with which seed.

The vault is a fifth copy, not a replacement for paper, and the Backup step
says so. A file is the least safe of the three.

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
flow, kept with the others; Steps only, on the left, is where a first
start is (owner, 2026-10-07).

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
