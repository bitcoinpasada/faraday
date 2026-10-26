# Convert a seed

**Kind:** Spec
**Status:** Specified from the owner's ask (2026-10-23), not built; the decision, F88, is the owner's (2026-10-23)
**Open:** all of it, batches S1 to S7 (§10), after the codex32 work (`docs/CODEX32.md` batches C1 and C4)

A tool, **Tools › Convert a seed**, that writes a seed held here in
another form. It replaces putting conversions in vault items or on the
backup board (`docs/CODEX32.md` §9.6 and §10.5 send them here). A
conversion is offered only where its result restores the same wallet in
the standard it is written in, checked against that standard's
published vectors and another implementation. Each result says which
standard it is and which software restores it.

## 1. The rules

1. **Same wallet, or not offered.** A conversion is offered only where
   the result, read by its standard, gives the same BIP-32 master key as
   the source: the same master fingerprint. Faraday reads every result
   back through its own decoder for that standard and compares the
   fingerprint before showing it; a mismatch shows nothing and says
   "Not shown: it did not read back as the same wallet". That is the
   device's own check; the vectors and the second implementation (§9)
   are the build's.
2. **Each result names its standard and its software.** A **Standard**
   row ("SLIP-39, 1 of 1, 20 words") and a **Restores in** row (§4's
   last column), and the fingerprint row: "73c5da0a · same as the
   source".
3. **A passphrase is folded in or kept apart, and the result says
   which.** Where the source's passphrase is folded into the result (a
   BIP-39 seed with its passphrase, written as a 64-byte seed or
   codex32), the result reads "Holds the passphrase's wallet: restore
   it with no passphrase". Where the target standard has a passphrase of
   its own (SLIP-39), the result is made with it empty, always (Q4).
4. **Not offered is said, not hidden.** The targets that would give a
   different wallet are listed dimmed with one line each (§5).
5. **The source is a key here.** The tool converts a key loaded in the
   session or kept in an open vault, never text typed into it, except an
   LND aezeed, which the tool reads as the Lightning screen does, since
   Faraday never loads one as a key (`core/faraday-bip/src/aezeed.rs`).
   A key not loaded is brought in by Add a key, which returns to the
   tool. Typed words therefore follow Add a key's rules for transient
   input, and the tool holds no secret of its own (`docs/KEYS.md` §2).
6. **The seed bytes must exist.** A key loaded from a master seed
   (codex32, SLIP-39) keeps its master key and not
   its seed today (`Keys::add_seed`, `faraday/faraday-keys/src/session.rs`);
   `docs/CODEX32.md` batch C4 keeps the seed bytes. Until then, such a
   key offers only the conversions from its master key (xprv).
7. **A set action is the backup's.** Add a share, Replace a lost share
   and Rotate the set change what a plan's places keep, so they live
   with the backup (`docs/CODEX32.md` §9.7; SLIP-39's in §8 here). The
   tool's "a set → a new set" is offered only for a key with no backup
   plan; with one, the tool's row opens the chart's set actions.

## 2. What is built today

- **Readers and writers in `core/`**: BIP-39 (`core/faraday-bip/src/bip39.rs`,
  every list), SeedQR and CompactSeedQR (`core/faraday-codec/src/seedqr.rs`),
  Seed XOR split and combine (`core/faraday-entropy/src/xor.rs`), codex32
  (`core/faraday-bip/src/codex32/mod.rs`: parse, `from_seed`, `split`,
  `derive_share`, `recover`, `identifier_for`, 16 to 64 bytes), SLIP-39
  (`core/faraday-bip/src/slip39.rs`: `split`, `recover`, `encrypt`,
  `decrypt`; 16 to 32 bytes; extendable flag and iteration exponent as
  parameters), aezeed read only (`core/faraday-bip/src/aezeed.rs`).
- **Where they surface**: Add a key reads BIP-39, SeedQR, CompactSeedQR,
  codex32, SLIP-39 and Seed XOR (`faraday/faraday-core/src/forms.rs`);
  New key deals SLIP-39 shares (`faraday/faraday-core/src/keygen/mod.rs`,
  extendable set, iteration exponent 1, a random identifier); a
  backup's copies offer Words, SeedQR, Compact SeedQR and Seed XOR, and
  never codex32 strings for a key of words (F121)
  (`faraday/faraday-keys/src/operations.rs`); the BIP-39 words tool
  lists each word's number and bits; New key takes hex entropy; the
  Lightning screen reads an aezeed.
- **Not built**: a SLIP-39 split of the encrypted master secret without
  its passphrase, a SLIP-39 member derived at a new index (§8), master
  seed or xprv shown as text, the tool itself.
- **Leaving the store**: a paper form leaves through
  `Reveal::PaperForm` (`faraday/faraday-keys/src/session.rs`), which
  names Seed XOR, codex32 and SLIP-39. A master seed as hex and an xprv
  are not in the list today; they will leave through it too, each being
  written by hand, and no `Reveal` variant is added (Q3).

## 3. Words

- **Entropy**: the bits BIP-39 words carry (16 to 32 bytes). Not a seed:
  BIP-39 makes the seed from the words' text by PBKDF2.
- **Master seed**: the bytes BIP-32's `Xpriv::new_master` takes, 16 to
  64. BIP-39's output is 64 bytes; codex32 carries 16, 20, 24, 28, 32 or
  64; SLIP-39 carries 16 to 32 (even); an aezeed carries 16.
- **EMS**: SLIP-39's encrypted master secret, the master seed under the
  SLIP-39 passphrase; what the shares split.
- **Set**: the shares of one split (codex32: one identifier, threshold
  and length; SLIP-39: one identifier and its groups).

## 4. The conversions

The software column names what restores the result as written. Rows
marked † are not certain from what this spec could check (§12), and are
confirmed against each program's own documentation or by a restore
before the row ships.

### 4.1 Same words, another writing (from BIP-39 words)

| From → to | Standard of the result | Restores in |
|---|---|---|
| Words → SeedQR | SeedQR (SeedSigner's standard) | SeedSigner, Krux, Jade †, Passport †, Faraday |
| Words → CompactSeedQR | CompactSeedQR (SeedSigner's) | SeedSigner, Krux, Jade †, Faraday |
| Words → hex entropy | BIP-39 entropy, hex | Ian Coleman's BIP-39 page (its entropy field), Faraday's New key (hex); no wallet takes entropy as a seed |
| Words → word numbers | BIP-39, each word's index (1 to 2048, and 0 to 2047 shown beside) | any BIP-39 wallet, once turned back into words by the list |
| Words → 4-letter starts | BIP-39 (the English list is unique in its first four letters) | any BIP-39 wallet that completes a word from its start (Coldcard, Trezor †), and any, written out in full |
| SeedQR, CompactSeedQR, numbers or starts → words | BIP-39 | every BIP-39 wallet |

Faraday makes most of these already (§2); the tool gathers them. The
words' own language is kept: the English list for SeedQR, CompactSeedQR
and the starts (SeedQR is defined on English; another list is not
offered, §5).

### 4.2 Words split and rebuilt

| From → to | Standard | Restores in |
|---|---|---|
| Words → Seed XOR parts (2, 3 or 4) | Seed XOR (Coldcard's) | Coldcard, Faraday; each part is itself valid BIP-39 words |
| Seed XOR parts → words | BIP-39 | every BIP-39 wallet |

### 4.3 Words to a master seed, one way

| From → to | Standard | Restores in |
|---|---|---|
| Words (+ passphrase) → master seed, hex (64 bytes) | BIP-32 seed | any BIP-32 library; Faraday; no common wallet takes a raw seed † |
| Words (+ passphrase) → xprv (or tprv) | BIP-32 master extended private key | Sparrow (master private key), Bitcoin Core (in a descriptor with the wallet's path), Faraday; Electrum takes an xprv as the account, not the master, so its addresses differ † |

Every row in §4.3 to §4.5 that writes a master seed or an xprv says
which paths the source wallet uses (from the loaded wallets that hold
the key: "Wallet 'Savings' · m/84'/0'/0'"), since the master key alone
does not say them.

### 4.4 Seed to seed, between standards

| From → to | Standard | Restores in |
|---|---|---|
| codex32 (16 or 32 bytes) → SLIP-39 1 of 1 | SLIP-39, one group of one share, 20 or 33 words | Trezor (Model T, Safe 3, Safe 5), Electrum †, Sparrow †, Faraday |
| codex32 (16 or 32 bytes) → SLIP-39 k of n, or groups | SLIP-39 | the same |
| SLIP-39 set (any passphrase) → codex32 of the seed that passphrase gives | BIP-93 | Faraday; the BIP-93 reference code † |
| codex32 or SLIP-39 → master seed hex (16 to 64 bytes) | BIP-32 seed | as §4.3 |
| codex32, SLIP-39 or a master seed → xprv | BIP-32 | as §4.3 |
| codex32 set → a new codex32 set (other k, n or identifier) | BIP-93 | as above; rule 7 |
| SLIP-39 set → a new SLIP-39 set | SLIP-39, made from the EMS so the passphrase still opens the same wallet (§8) | as above; rule 7 |

**SLIP-39 1 of 1 is always the same words** for one seed, so a person
can make it twice and compare: identifier = the first 15 bits of the
master fingerprint; group threshold 1 of 1, member 1 of 1 (SLIP-39
draws no randomness for a threshold of 1); iteration exponent and
extendable flag set and iteration exponent 1, as New key and Trezor
make them (Q2); SLIP-39 passphrase empty (Q4). The codex32
secret (k = 0, index `s`) is likewise fixed by its identifier,
`identifier_for` the fingerprint (`docs/CODEX32.md` §6.4.5).

A codex32 seed of 20, 24 or 28 bytes is a valid SLIP-39 master secret
too (SLIP-39 asks for at least 128 bits, in whole 16-bit steps), but
no hardware wallet reads a SLIP-39 backup of that length; those rows are
not offered (Q5).

### 4.5 Another wallet's seed to a master seed, one way

| From → to | Standard | Restores in |
|---|---|---|
| LND aezeed (+ its passphrase) → master seed hex (16 bytes), codex32 (48 characters), SLIP-39 (20 words), xprv | BIP-32 seed; BIP-93; SLIP-39; BIP-32 | as above, for the on-chain wallet only |
| Electrum's seed words → master seed hex, xprv (64 bytes) | as above | once Faraday reads Electrum seeds (planned, not scheduled); not offered until then |

An aezeed's result says: the paths LND uses (BIP-84 `m/84'/0'/0'`,
BIP-49 `m/49'/0'/0'` with LND's own change addresses, BIP-86
`m/86'/0'/0'`, and its key families under `m/1017'`); the birthday it
carries, as a date, for the rescan; and "On-chain coins only: channels
come back through LND's channel backup". LND uses the aezeed's sixteen
bytes of entropy as the BIP-32 seed itself, which is what makes the
rows possible.

## 5. Not offered, and what the tool says

Each is a dimmed row under **To**, its line in place of a press.

| Target | Line | Why |
|---|---|---|
| A seed, an xprv, codex32 or SLIP-39 → BIP-39 words | "BIP-39 words cannot be made from a seed" | BIP-39 makes the seed from the words by PBKDF2, which is one-way |
| An xprv → a seed | "A seed cannot be made from an xprv" | BIP-32's master key is an HMAC of the seed |
| BIP-39 entropy written as SLIP-39 or codex32 | "Another wallet: these bytes as SLIP-39 or codex32 open a different, empty wallet" | Both standards carry a master seed, not BIP-39 entropy (`core/faraday-bip/src/slip39.rs`'s header) |
| BIP-39 words in another language's list | "Another wallet: the seed is made from the words' text" | PBKDF2 runs over the words themselves |
| A 64-byte seed → SLIP-39 | "SLIP-39 backups of 64 bytes are read by no wallet" | Faraday's SLIP-39 holds 16 to 32 bytes, as Trezor's does |
| Shamir39 | "Not offered" | §11 (Q1) |

## 6. The screen

A new step flow, `Screen::Convert`, drawn as DESIGN §5's step flow, from
DESIGN §4's components; a row in DESIGN §5's table (edit list). Ways in:
a Tools tile, **Convert a seed**, in the Keys and seeds group (its line
"The same wallet in another standard"); a held key's chart sheet and
Explore's key row, **Write it in another form**, which opens the tool
on that key.

1. **From** (Option rows, §4.2): the loaded keys and the open vaults'
   keys, each by fingerprint, label and what it was loaded from ("BIP-39
   words, 24", "codex32, 48 characters", "SLIP-39, 2 of 3"), then
   **Add a key** (returns here) and **An LND aezeed** (the Lightning
   screen's entry: the 24 words and its passphrase field).
2. **To** (Option rows): the targets §4 offers for that source, by
   group, each with its line naming the standard; then the dimmed rows
   of §5 with their lines.
3. **Settings** (where the target has any): k and n on Sliders (§4.2)
   for a split; groups on a Stepper with a threshold Slider per group
   (SLIP-39); the length (codex32 48 or 74 where the seed allows);
   for a new set, the identifier (codex32, typed as `docs/CODEX32.md`
   §6.2 types it). A row that would draw randomness carries
   "Fresh randomness: Faraday's" as a Fact.
4. **Result**: a Held secret (§4.10) per string or share, numbered, and
   for a SeedQR or CompactSeedQR the code held; Facts (§4.11):
   **Standard**, **Restores in**, **Fingerprint** "73c5da0a · same as
   the source", **Paths** where §4.3 says, **Birthday** for an aezeed;
   the passphrase line of rule 3. Buttons: **Check my copy** (typed or
   scanned back, as the backup's), **Save as a file…** (the Secret out
   sheet, §4.10), **Done**. Nothing is written or kept unless pressed.
5. Leaving drops every result (rule 5).

On the small panel, a page per step, as every step flow. The work of a
conversion (PBKDF2, scrypt, a split) runs on the press that asks for
it, takes the compute token, and its result is kept on the tool's state
until the flow is left; drawing reads it (`docs/COMPUTE.md`).

## 7. Practice

`docs/DEMO.md` §3: an action is disabled where its result is a secret
a person could take for a new one; a public seed's own spelling is not
new.

| Conversion | In Practice | Why |
|---|---|---|
| §4.1, a public seed | Open | Its own words, another spelling |
| §4.3, a public seed (seed hex, xprv, codex32 k = 0) | Open | Derived from a public seed with no randomness |
| §4.4 one-to-one (codex32 ↔ SLIP-39 1 of 1, hex, xprv) of a public seed (the BIP-93 and SLIP-39 samples, `docs/MODES.md` §5.1) | Open | No randomness: the same words every time |
| Any split, k of n with k of 2 or more (Seed XOR, codex32, SLIP-39), and a new set | Disabled, as DEMO §6 disables splits | Fresh randomness: the parts would look like a real backup |
| §4.5, LND's published vector only (DEMO §6, F81) | Open | The vector is public |
| A key that is not a public seed | Not reachable | Practice loads only public seeds |

## 8. SLIP-39 set actions

The same three as codex32's (`docs/CODEX32.md` §9.7), with the backup,
on the set's node of the chart; the screens are §9.7's once built. What
SLIP-39 needs that codex32 does not:

- **Add a share** to a group: the group's threshold of members
  interpolated at an unused member index (0 to 15). A share carries its
  member index and the group's threshold, not the member count, so
  every existing share stays valid. No passphrase is needed: the
  arithmetic is on the group's share of the EMS. A new **group** is not
  offered: the group count is in every share.
- **Replace a lost share**: the same, at the lost share's index.
- **Rotate the set**: a new split of the EMS, recovered from a quorum of
  the old shares, so the set's passphrase opens the same wallet without
  being typed. With the extendable flag set, the new set takes a new
  identifier (the identifier is not in the encryption's salt); with it
  clear, the identifier stays (it is in the salt), and old and new
  shares cannot be told apart by it; a mixed set then fails the
  digest check rather than opening another wallet. Steps 1 to 5 of
  §9.7 (marks, counting, destroying before making) apply as written.
- **Core functions** (`core/faraday-bip/src/slip39.rs`): `recover_ems`
  (shares to EMS and its parameters), `split_ems` (an EMS to a new set
  under given parameters), `derive_member` (a group's quorum to a share
  at an index). `python-shamir-mnemonic` has the first two, which the
  tests compare against.

## 9. Tests

Each conversion against its standard's published vectors and against
another implementation (`docs/CROSSCHECK.md` §2's references, run
through §10's cross-check once built; until then their output is
checked in as fixtures under `tools/vectors/`, each with the script and
the reference's commit that made it, as `tools/reference/aezeed/README.md`
records its sources).

| Conversion | Published vectors | Another implementation |
|---|---|---|
| Words ↔ SeedQR, CompactSeedQR | SeedSigner's SeedQR specification vectors | SeedSigner's own encoder (Python) |
| Words ↔ entropy, numbers, starts | BIP-39's vectors (`tools/vectors/bip39`) | python-mnemonic |
| Words ↔ Seed XOR | Coldcard's published examples (`tools/vectors/xor`) | Coldcard's `xor_seed` code (Python) |
| Words → seed, xprv | BIP-39's vectors (seed and xprv columns) | python-mnemonic, embit |
| codex32 ↔ SLIP-39 | BIP-93's and SLIP-39's vectors (`tools/vectors/slip39`), each read and written the other way | python-codex32 and python-shamir-mnemonic over random seeds: the same fingerprint both ways |
| SLIP-39 with passphrase → codex32 | SLIP-39's vectors (passphrase `TREZOR`) | python-shamir-mnemonic |
| codex32, SLIP-39, seed → xprv | BIP-32's vectors; BIP-93's and SLIP-39's xprv columns | embit, Bitcoin Core's test framework |
| A set → a new set; SLIP-39 add, replace, rotate | SLIP-39's vectors for EMS | python-shamir-mnemonic's `split_ems` and `recover_ems`; the codex32 reference's interpolation |
| aezeed → seed, codex32, SLIP-39 | LND's `cipherseed_test.go` vectors (`tools/reference/aezeed/README.md`) | none: LND's published vectors only (Q6) |

And, user-facing: each §5 target is dimmed with its line; a result
whose read-back fingerprint differed is not shown (a corrupted decoder
in a test build); SLIP-39 1 of 1 of one seed is the same words twice;
the Practice rows of §7; leaving the flow drops the results (no secret in
the kept state, `tools/lint-secrets.sh`).

## 10. Plan

### 10.1 Batches

After `docs/CODEX32.md` C1 (the codex32 folder) and C4 (seed bytes for
keys loaded from a seed); S6 after C8 (codex32 in the backup).

| Batch | What | Model | Estimate |
|---|---|---|---|
| S1 | `faraday-bip`: SLIP-39 `recover_ems`, `split_ems`, `derive_member`; fixtures from the references for every row of §9 | Opus | 1.5 days |
| S2 | `faraday-keys`: the conversions as operations returning held results, the read-back fingerprint check, master seed hex and xprv through `Reveal::PaperForm` (Q3) | Sonnet | 1 day |
| S3 | The screen (§6) with §4.1 and §4.3; Tools tile and the two ways in | Opus | 2 days |
| S4 | §4.2 and §4.4, settings for splits and groups | Sonnet | 1.5 days |
| S5 | §4.5 (aezeed) | Sonnet | 0.5 day |
| S6 | §8 with the backup's set actions | Opus | 1.5 days |
| S7 | Practice (§7), Learn (`docs/learn/19-other-backups.md` gains a section on converting), DESIGN §5's row, this file's plan to a history log of its own and its Kind to Reference | Sonnet | 0.5 day |

About 8.5 days.

### 10.2 Edit list

| File | Change |
|---|---|
| `core/faraday-bip/src/slip39.rs` | §8's three functions |
| `faraday/faraday-keys/src/operations.rs`, `faraday/faraday-keys/src/session.rs` | the conversions; `Reveal::PaperForm` names master seed hex and xprv (Q3) |
| `faraday/faraday-core/src/lib.rs` | `Screen::Convert`, its module in the code map |
| a new area folder in faraday-core, convert (its mod.rs and screen.rs) | the tool's state and screen |
| `faraday/faraday-core/src/screens/catalog.rs` | the tile |
| `faraday/faraday-strings/src/en.rs` | every string |
| `docs/DESIGN.md` §5, `docs/FLOWS.md`, `docs/DEMO.md` §6, `docs/CODEX32.md` §10.5 | the screen, the flow, the Practice rows, the pointer here |
| `docs/README.md` | this file in the Spec list and the By question table |

## 11. Shamir39

Ian Coleman's Shamir39 splits BIP-39 words into shares that are
themselves words of the BIP-39 list. It is not a SLIP or a BIP; its
author marks it as not a standard, and few programs read it (his
BIP-39 page's Shamir39 tab †). Decided (owner, 2026-10-23): not
offered, and listed in §5 as "Not offered" (Q1).

## 12. Where this spec is not sure

- Which wallets restore SeedQR and CompactSeedQR beyond SeedSigner and
  Krux; whether Electrum and Sparrow restore SLIP-39 today; whether any
  wallet in common use restores codex32 (Bitcoin Core's codex32 import
  was proposed and, as far as known here, not merged); Electrum's
  handling of a master xprv. Rows marked † in §4.
- Whether SLIP-39 sets an upper bound on the master secret's length.
  The SLIP asks for at least 128 bits in 16-bit steps; Trezor reads 128
  and 256 only, and Faraday holds at most 256. §5 refuses 64 bytes
  either way.
- The extendable flag and iteration exponent "by default": the owner's
  ask said the flag clear; Faraday's New key and current Trezor
  firmware set it, with exponent 1. Decided (owner, 2026-10-23): set,
  exponent 1 (Q2).

## 13. Questions for the owner

Decided (owner, 2026-10-23), each as recommended:

1. **Shamir39.** Decided: not offered (§5, §11).
2. **SLIP-39 1 of 1 parameters.** Decided: the extendable flag set and
   iteration exponent 1, as New key and Trezor make them (§4.4).
3. **The reveal.** Decided: a master seed as hex and an xprv leave
   through `Reveal::PaperForm`, each being written by hand; no `Reveal`
   variant is added (§2, batch S2).
4. **A SLIP-39 passphrase on the result.** Decided: never (§1 rule 3).
5. **codex32 of 20, 24 or 28 bytes → SLIP-39.** Decided: not offered
   (§4.4).
6. **aezeed's second implementation.** Decided: checked against LND's
   published vectors only; no Go toolchain in the cross-check (§9).
7. **A new set in the tool.** Decided: only for a key with no backup
   plan; with one, the tool's row opens the chart's set actions (§1
   rule 7).
